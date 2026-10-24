//! アプリケーション (ELF) をメモリへ読み込み、実行する。
//!
//! 本家 MikanOS の `terminal.cpp` にある `LoadELF` / `CopyLoadSegments` / `MakeArgVector` に相当する。
//! 本家は `CleanPageMaps` を実行後に手で呼ぶが、ここでは [`LoadedApp`] の `Drop` で必ず行う。
//! そのため、途中で読み込みに失敗した場合も、確保済みのページマップが解放される。
use alloc::vec::Vec;

use crate::{
    elf::{self, ET_EXEC, Elf64Ehdr, Elf64Phdr},
    error::{KernelError, Result},
    paging::{LinearAddress4Level, clean_page_maps, setup_page_maps},
    printk,
};

/// アプリをロードしてよい仮想アドレスの下限 (アドレス空間の上位半分)。
/// それより下はカーネルが恒等写像で使っている。
const APP_BASE: u64 = 0xffff_8000_0000_0000;
const PAGE_SIZE: u64 = 4096;

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
    }
}

impl LoadedApp {
    /// エントリポイントを `main(argc, argv)` として呼び、その戻り値を返す。
    pub fn call(&self, argv: &ArgVector) -> i32 {
        type AppMain = extern "sysv64" fn(i32, *const *const u8) -> i32;
        // SAFETY: load_elf が e_entry を含む LOAD セグメントを読み込み済み。
        let f = unsafe { core::mem::transmute::<usize, AppMain>(self.entry as usize) };
        f(argv.ptrs.len() as i32, argv.ptrs.as_ptr())
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
        core::ptr::write_bytes(dst.add(image.len()), 0, (segment.p_memsz - segment.p_filesz) as usize);
    }
    Ok(())
}

/// アプリに渡す `argv`。各引数は NUL 終端の文字列として保持する。
pub struct ArgVector {
    /// 文字列の実体。`ptrs` が指す先なので、`ArgVector` が生きている間は保持する。
    /// (外側の `Vec` を move しても、内側の各 `Vec` のヒープ領域は動かない)
    _strings: Vec<Vec<u8>>,
    ptrs: Vec<*const u8>,
}

impl ArgVector {
    /// `argv[0] = command`、`argv[1..]` = `first_arg` を空白で区切ったもの。
    pub fn new(command: &str, first_arg: Option<&str>) -> Self {
        let mut strings: Vec<Vec<u8>> = Vec::new();
        let mut push = |s: &str| {
            let mut v = Vec::with_capacity(s.len() + 1);
            v.extend_from_slice(s.as_bytes());
            v.push(0);
            strings.push(v);
        };
        push(command);
        for arg in first_arg.into_iter().flat_map(|a| a.split_ascii_whitespace()) {
            push(arg);
        }
        let ptrs = strings.iter().map(|s| s.as_ptr()).collect();
        Self {
            _strings: strings,
            ptrs,
        }
    }
}

