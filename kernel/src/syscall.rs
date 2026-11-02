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
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallResult {
    pub value: u64,
    pub error: i32,
}

pub mod errno {
    pub const EPERM: i32 = 1;
    pub const E2BIG: i32 = 7;
    pub const EBADF: i32 = 9;
}

// システムコール関数の型定義 (最大6引数、戻り値 SyscallResult)
pub type SyscallFn = extern "C" fn(u64, u64, u64, u64, u64, u64) -> SyscallResult;

/// システムコール 0x80000000: 文字列をコンソールに出力する
pub extern "C" fn sys_log_string(
    level: u64,
    message_ptr: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    if level > 3 {
        return SyscallResult {
            value: 0,
            error: errno::EPERM,
        };
    }
    let s_ptr = message_ptr as *const u8;
    if s_ptr.is_null() {
        return SyscallResult {
            value: 0,
            error: errno::EPERM,
        };
    }
    let mut len = 0;
    unsafe {
        while *s_ptr.add(len) != 0 {
            len += 1;
            if len > 1024 {
                return SyscallResult {
                    value: 0,
                    error: errno::E2BIG,
                };
            }
        }
        if let Ok(s) = core::str::from_utf8(core::slice::from_raw_parts(s_ptr, len)) {
            crate::printk!("{}", s);
        }
    }
    SyscallResult {
        value: len as u64,
        error: 0,
    }
}
/// システムコール 0x80000001: ターミナルに文字列を出力する (PutString)
pub extern "C" fn sys_put_string(
    fd: u64,
    buf_ptr: u64,
    len: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    if len > 1024 {
        return SyscallResult {
            value: 0,
            error: errno::E2BIG,
        };
    }
    if fd == 1 {
        let task_id = crate::task::current_task_id();
        let slice = unsafe { core::slice::from_raw_parts(buf_ptr as *const u8, len as usize) };
        crate::terminal::TERMINALS.with(|map| {
            if let Some(&term_ptr) = map.get(&task_id) {
                unsafe {
                    (*term_ptr).print_bytes(slice);
                }
            }
        });
        return SyscallResult {
            value: len,
            error: 0,
        };
    }
    SyscallResult {
        value: 0,
        error: errno::EBADF,
    }
}
/// システムコール 0x80000002 : アプリケーションを終了する
pub extern "C" fn sys_exit(
    exit_code: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    let os_stack_ptr = crate::task::current_task_os_stack_ptr();
    SyscallResult {
        value: os_stack_ptr,     // RAX に OSスタックポインタが入る
        error: exit_code as i32, // RDX に終了コードが入る
    }
}
/// システムコールの関数ポインタテーブル (本家 syscall_table 相当)
#[unsafe(no_mangle)]
pub static SYSCALL_TABLE: [SyscallFn; 3] = [sys_log_string, sys_put_string, sys_exit];

global_asm!(
    r#"
.global CallApp
CallApp:  # int CallApp(int argc, char** argv, uint16_t ss, uint64_t rip, uint64_t rsp, uint64_t* os_stack_ptr);
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    mov [r9], rsp  # OS 用のスタックポインタを保存 (第6引数 r9)
    push rdx       # SS (ユーザー空間データセグメント)
    push r8        # RSP (ユーザー空間スタックポインタ)
    add rdx, 8     # CS は SS の次 (USER_CS = USER_SS + 8)
    push rdx       # CS (ユーザー空間コードセグメント)
    push rcx       # RIP (エントリポイント)
    retfq          # ユーザーモード (Ring 3) へジャンプ
.global SyscallEntry
SyscallEntry:  # void SyscallEntry(void);
    push rbp
    push rcx   # ユーザー空間の元の RIP (syscall 命令が RCX に自動退避)
    push r11   # ユーザー空間の元の RFLAGS (syscall 命令が R11 に自動退避)
    push rax   # ★【追加】システムコール番号をスタックに保存
    mov rcx, r10        # syscall ABI では第4引数は R10 に入るため、C ABI の RCX に移動
    and eax, 0x7fffffff # システムコール番号の最上位ビットをマスク (0x80000000 -> 0)
    mov rbp, rsp
    and rsp, -16        # 16バイト・スタックアライメント
    lea r11, [rip + SYSCALL_TABLE]
    call qword ptr [r11 + 8 * rax]
    # rax には value, rdx には error が格納されている
    mov rsp, rbp
    pop rsi             # 保存していたシステムコール番号を復帰
    cmp esi, 0x80000002 # Exit システムコール (0x80000002) か？
    je .Lexit           # Exit なら .Lexit へジャンプ
    pop r11    # 元の RFLAGS を復帰
    pop rcx    # 元の RIP を復帰
    pop rbp    # 元の RBP を復帰
    sysretq    # ユーザーモードへ復帰 (RIP <- RCX, RFLAGS <- R11)
.Lexit:
    # アプリ終了時のスタック復帰処理
    mov rsp, rax # rax は task.os_stack_ptr (CallApp 実行直前の OS スタックに復帰)
    mov eax, edx # edx は exit_code (第1戻り値として EAX に設定)
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    ret          # CallApp の呼び出し元 (Terminal::ExecuteFile) へリターン
"#
);

unsafe extern "C" {
    pub fn SyscallEntry();
    pub fn CallApp(
        argc: i32,
        argv: *const *const u8,
        ss: u16,
        rip: u64,
        rsp: u64,
        os_stack_ptr: *mut u64,
    ) -> i32;
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
