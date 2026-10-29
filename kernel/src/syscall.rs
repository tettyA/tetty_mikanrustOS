use core::arch::global_asm;

const IA32_EFER: u32 = 0xc000_0080;
const IA32_STAR: u32 = 0xc000_0081;
const IA32_LSTAR: u32 = 0xc000_0082;
const IA32_FMASK: u32 = 0xc000_0084;
fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        core::arch::asm!("wrmsr" ,
    in("ecx") msr,
    in("eax") low,
    in("edx") high,
    options(nomem, nostack, preserves_flags))
    };
}

// システムコール関数の型定義 (最大6引数、戻り値 i64)
pub type SyscallFn = extern "C" fn(u64, u64, u64, u64, u64, u64) -> i64;

/// システムコール 0x80000000: 文字列をコンソールに出力する
pub extern "C" fn sys_log_string(
    _level: u64,
    message_ptr: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i64 {
    let s_ptr = message_ptr as *const u8;
    if s_ptr.is_null() {
        return -1;
    }
    let mut len = 0;
    unsafe {
        // 最大1024バイトまでNUL終端を探索
        while *s_ptr.add(len) != 0 {
            len += 1;
            if len > 1024 {
                return -1;
            }
        }
        if let Ok(s) = core::str::from_utf8(core::slice::from_raw_parts(s_ptr, len)) {
            crate::printk!("{}", s);
        }
    }
    0
}

/// システムコールの関数ポインタテーブル (本家 syscall_table 相当)
#[unsafe(no_mangle)]
pub static SYSCALL_TABLE: [SyscallFn; 1] = [sys_log_string];

global_asm!(
    r#"
.global SyscallEntry
SyscallEntry:  # void SyscallEntry(void);
    push rbp
    push rcx   # ユーザー空間の元の RIP (syscall 命令が RCX に自動退避)
    push r11   # ユーザー空間の元の RFLAGS (syscall 命令が R11 に自動退避)
    mov rcx, r10        # syscall ABI では第4引数は R10 に入るため、C ABI の RCX に移動
    and eax, 0x7fffffff # システムコール番号の最上位ビットをマスク (0x80000000 -> 0)
    mov rbp, rsp
    and rsp, -16        # 16バイト・スタックアライメント
    lea r11, [rip + SYSCALL_TABLE]
    call qword ptr [r11 + 8 * rax]
    # rbx, r12-r15 は callee-saved なので呼び出し先で保護される
    # rax は戻り値なので上書きしない
    mov rsp, rbp
    pop r11    # 元の RFLAGS を復帰
    pop rcx    # 元の RIP を復帰
    pop rbp    # 元の RBP を復帰
    sysretq    # ユーザーモードへ復帰 (RIP <- RCX, RFLAGS <- R11)
"#
);

unsafe extern "C" {
    pub fn SyscallEntry();
}
/// MSR を設定して syscall / sysret 命令を有効化する (本家 InitializeSyscall 相当)
pub fn init_syscall() {
    // 1. IA32_EFER: bit 0 (SCE) を 1 にして syscall 命令を有効化
    wrmsr(IA32_EFER, 0x0501);
    // 2. IA32_LSTAR: syscall 実行時のカーネルエントリポイントアドレスを設定
    wrmsr(IA32_LSTAR, SyscallEntry as *const () as usize as u64);
    // 3. IA32_STAR: syscall / sysret 実行時のセグメントセレクタを設定
    //    ・[47:32] = 8 (Kernel CS = 0x08, Kernel SS = 0x10)
    //    ・[63:48] = (16 | 3) (User SS = 0x1b, User CS = 0x23)
    wrmsr(IA32_STAR, (8u64 << 32) | (((16 | 3) as u64) << 48));
    // 4. IA32_FMASK: syscall 実行時の RFLAGS マスク (0 = マスクしない)
    wrmsr(IA32_FMASK, 0);
}
