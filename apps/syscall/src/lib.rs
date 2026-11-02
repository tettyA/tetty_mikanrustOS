#![no_std]
#![allow(dead_code)]

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallResult {
    pub value: u64,
    pub error: i32,
}

/// システムコール 0x80000000: ログ文字列出力
pub fn sys_log_string(level: u64, message: *const u8) -> SyscallResult {
    let mut value: u64;
    let mut error: i32;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0000u32,
            in("rdi") level,
            in("rsi") message,
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }
    SyscallResult { value, error }
}

/// システムコール 0x80000001: ターミナルへ文字列出力 (PutString)
pub fn sys_put_string(fd: u64, buf: *const u8, len: usize) -> SyscallResult {
    let mut value: u64;
    let mut error: i32;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0001u32,
            in("rdi") fd,
            in("rsi") buf,
            in("rdx") len as u64,
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }
    SyscallResult { value, error }
}

/// システムコール 0x80000002: アプリケーション終了 (Exit)
pub fn sys_exit(exit_code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0002u32,
            in("rdi") exit_code as u64,
            options(noreturn),
        );
    }
}

pub fn sys_open_window(w: usize, h: usize, x: i32, y: i32, title: &str) -> SyscallResult {
    let mut value: u64;
    let mut error: i32;

    let mut title_buf = [0u8; 128];
    let len = title.len().min(127);

    title_buf[..len].copy_from_slice(&title.as_bytes()[..len]);
    title_buf[len] = 0;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0003u32,
            in("rdi") w as u64,
            in("rsi") h as u64,
            in("rdx") x as u64,
            in("r10") y as u64,
            in("r8") title_buf.as_ptr(),
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }

    SyscallResult { value, error }
}
/// 文字列を標準出力 (fd=1) に出力するラッパー
pub fn print(s: &str) {
    sys_put_string(1, s.as_ptr(), s.len());
}

/// 整数 (i64) を文字列に変換して標準出力に出力するラッパー
pub fn print_i64(mut val: i64) {
    if val == 0 {
        print("0");
        return;
    }
    let mut buf = [0u8; 32];
    let mut i = buf.len();
    let neg = val < 0;
    if neg {
        val = -val;
    }
    while val > 0 {
        i -= 1;
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    if neg {
        i -= 1;
        buf[i] = b'-';
    }
    let s = unsafe { core::str::from_utf8_unchecked(&buf[i..]) };
    print(s);
}
