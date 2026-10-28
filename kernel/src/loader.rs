//! アプリケーション (ELF) をメモリへ読み込み、実行する。
//!
//! 本家 MikanOS の `terminal.cpp` にある `LoadELF` / `CopyLoadSegments` / `MakeArgVector` に相当する。
//! 本家は `CleanPageMaps` を実行後に手で呼ぶが、ここでは [`LoadedApp`] の `Drop` で必ず行う。
//! そのため、途中で読み込みに失敗した場合も、確保済みのページマップが解放される。
use core::arch::asm;

use alloc::vec::Vec;

use crate::{
    elf::{self, ET_EXEC, Elf64Ehdr, Elf64Phdr},
    error::{KernelError, Result},
    paging::{LinearAddress4Level, clean_page_maps, setup_page_maps},
    printk,
    segment::{USER_CS, USER_SS},
};

/// アプリをロードしてよい仮想アドレスの下限 (アドレス空間の上位半分)。
/// それより下はカーネルが恒等写像で使っている。
const APP_BASE: u64 = 0xffff_8000_0000_0000;
const PAGE_SIZE: u64 = 4096;
const APP_STACK_PAGE_ADDR: u64 = 0xffff_ffff_ffff_e000;
const APP_STACK_SIZE: u64 = 4096;
const APP_ARGS_PAGE_ADDR: u64 = 0xffff_ffff_ffff_f000;
/// 読み込み済みのアプリ。`Drop` でアプリ用のページング構造を解放する。
pub struct LoadedApp {
    entry: u64,
    /// 最初の LOAD セグメントの仮想アドレス (解放するページ構造の PML4 エントリを決める)
    first_addr: LinearAddress4Level,
}

impl Drop for LoadedApp {
    fn drop(&mut self) {
        // drop はエラーを返せないので、失敗は画面に出す。
        if clean_page_maps(self.first_addr).is_err() {
            printk!("failed to clean page maps\n");
        }
        let stack_addr = LinearAddress4Level::new(APP_STACK_PAGE_ADDR);
        if clean_page_maps(stack_addr).is_err() {
            printk!("failed to clean stack page maps\n");
        }
        let args_addr = LinearAddress4Level::new(APP_ARGS_PAGE_ADDR);
        if clean_page_maps(args_addr).is_err() {
            printk!("failed to clean args page maps\n");
        }
    }
}
/// アプリケーションを特権レベル 3 (Ring 3) で起動する。
///
/// x86-64 では特権レベルを下げる専用の jump 命令は存在しないため、
/// 64ビット Far Return (`retfq`) を使用して CPL を Ring 3 に遷移させる。
///
/// スタックには [SS, RSP, CS, RIP] の順に積み、retfq が [RIP, CS, RSP, SS] を復帰する。
pub fn call_app(argc: i32, argv: *const *const u8, cs: u64, ss: u64, rip: u64, rsp: u64) -> ! {
    unsafe {
        asm!(
            "push {ss}",
            "push {rsp}",
            "push {cs}",
            "push {rip}",
            "retfq",
            ss=in(reg) ss,
            rsp=in(reg) rsp,
            cs=in(reg) cs,
            rip=in(reg) rip,
            in("rdi") argc,
            in("rsi")argv,
            options(noreturn)
        );
    }
}
impl LoadedApp {
    /// エントリポイントを `main(argc, argv)` として呼び、その戻り値を返す。
    pub fn call(&self, argv: &ArgVector) -> ! {
        let stack_top = APP_STACK_PAGE_ADDR + APP_STACK_SIZE - 8;

        call_app(
            argv.argc, argv.argv, USER_CS, USER_SS, self.entry, stack_top,
        );
    }
}

/// ELF ファイルの LOAD セグメントを、リンク時に指定された仮想アドレスへ配置する。
pub fn load_elf(file: &[u8]) -> Result<LoadedApp> {
    let ehdr = Elf64Ehdr::parse(file)?;
    if ehdr.e_type != ET_EXEC {
        return Err(KernelError::InvalidFormat);
    }

    let first_addr = elf::load_segments(file, &ehdr)
        .next()
        .ok_or(KernelError::InvalidFormat)?
        .p_vaddr;
    if first_addr < APP_BASE {
        return Err(KernelError::InvalidFormat);
    }

    // ガードを先に作る。以降 `?` で抜けても Drop が走り、確保済みのページマップを解放する。
    let app = LoadedApp {
        entry: ehdr.e_entry,
        first_addr: LinearAddress4Level::new(first_addr),
    };
    for segment in elf::load_segments(file, &ehdr) {
        copy_load_segment(file, &segment)?;
    }
    setup_page_maps(LinearAddress4Level::new(APP_STACK_PAGE_ADDR), 1)?;
    setup_page_maps(LinearAddress4Level::new(APP_ARGS_PAGE_ADDR), 1)?;
    //   setup_page_maps(LinearAddress4Level::new(APP_STACK_PAGE_ADDR), 1)?;
    Ok(app)
}

/// LOAD セグメント 1 つ分のページを用意し、内容をコピーして残りをゼロ埋めする。
fn copy_load_segment(file: &[u8], segment: &Elf64Phdr) -> Result<()> {
    if segment.p_vaddr < APP_BASE || segment.p_filesz > segment.p_memsz {
        return Err(KernelError::InvalidFormat);
    }
    let image = segment.file_image(file)?;
    let end = segment
        .p_vaddr
        .checked_add(segment.p_memsz)
        .ok_or(KernelError::InvalidFormat)?;

    // セグメントがページ境界から始まるとは限らない (--nmagic でリンクしている) ので、
    // 先頭を含むページから、末尾を含むページまでを数える。
    let num_4kpages = (end.div_ceil(PAGE_SIZE) - segment.p_vaddr / PAGE_SIZE) as usize;
    setup_page_maps(LinearAddress4Level::new(segment.p_vaddr), num_4kpages)?;

    let dst = segment.p_vaddr as *mut u8;
    // SAFETY: dst からの p_memsz バイトは setup_page_maps でマップ済み。
    // image は p_filesz バイトで、p_filesz <= p_memsz を確認済み。
    unsafe {
        core::ptr::copy_nonoverlapping(image.as_ptr(), dst, image.len());
        core::ptr::write_bytes(
            dst.add(image.len()),
            0,
            (segment.p_memsz - segment.p_filesz) as usize,
        );
    }
    Ok(())
}

/// アプリに渡す `argv`。各引数は NUL 終端の文字列として保持する。
pub struct ArgVector {
    argc: i32,
    argv: *const *const u8,
}

impl ArgVector {
    /// 本家 MakeArgVector と同様に、0xffff_ffff_ffff_f000 のページ内に
    /// argv 配列 (ポインタ列) と文字列本体をコピーして配置する。
    pub fn new(command: &str, first_arg: Option<&str>) -> Self {
        let base = APP_ARGS_PAGE_ADDR as *mut u8;
        let argv_len = 32; // 最大32引数
        let argv_ptr = base as *mut *const u8;
        let mut argbuf = unsafe { base.add(core::mem::size_of::<*const u8>() * argv_len) };
        let mut argc = 0;

        let mut push = |s: &str| {
            if argc < argv_len {
                unsafe {
                    *argv_ptr.add(argc) = argbuf;
                    core::ptr::copy_nonoverlapping(s.as_ptr(), argbuf, s.len());
                    *argbuf.add(s.len()) = 0; // NUL 終端
                    argbuf = argbuf.add(s.len() + 1);
                }
                argc += 1;
            }
        };

        push(command);
        for arg in first_arg
            .into_iter()
            .flat_map(|a| a.split_ascii_whitespace())
        {
            push(arg);
        }

        Self {
            argc: argc as i32,
            argv: argv_ptr,
        }
    }
}
