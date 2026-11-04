#![no_std]
#![allow(dead_code)]

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallResult {
    pub value: u64,
    pub error: i32,
}

pub const LAYER_NO_REDRAW: u64 = 0x0000_0001_0000_0000;
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
fn sys_exit(exit_code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0002u32,
            in("rdi") exit_code as u64,
            options(noreturn),
        );
    }
}

/// システムコール 0x80000004: ウィンドウに文字列を描画 (WinWriteString)
pub fn sys_win_write_string(
    layer_id_flags: u64,
    x: i32,
    y: i32,
    color: u32,
    s: &str,
) -> SyscallResult {
    let mut value: u64;
    let mut error: i32;
    // 文字列を NUL 終端にして渡すため一時バッファにコピー
    let mut buf = [0u8; 1024];
    let len = s.len().min(1023);
    buf[..len].copy_from_slice(&s.as_bytes()[..len]);
    buf[len] = 0;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0004u32,
            in("rdi") layer_id_flags,
            in("rsi") x as u64,
            in("rdx") y as u64,
            in("r10") color as u64,
            in("r8") buf.as_ptr(),
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }
    SyscallResult { value, error }
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

/// システムコール 0x80000005: ウィンドウに矩形を描画 (WinFillRectangle)
pub fn sys_win_fill_rectangle(
    layer_id_flags: u64,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
) -> SyscallResult {
    let mut value: u64;
    let mut error: i32;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0005u32,
            in("rdi") layer_id_flags,
            in("rsi") x as u64,
            in("rdx") y as u64,
            in("r10") w as u64,
            in("r8") h as u64,
            in("r9") color as u64,
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }
    SyscallResult { value, error }
}
/// システムコール 0x80000006: 現在のタイマ値 (tick) と周波数を取得 (GetCurrentTick)
/// 戻り値: SyscallResult { value: 現在のtick値, error: タイマ周波数(Hz) }
pub fn sys_get_current_tick() -> SyscallResult {
    let mut value: u64;
    let mut error: i32;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0006u32,
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }
    SyscallResult { value, error }
}
/// システムコール 0x80000007: ウィンドウを再描画 (WinRedraw)
pub fn sys_win_redraw(layer_id_flags: u64) -> SyscallResult {
    let mut value: u64;
    let mut error: i32;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0007u32,
            in("rdi") layer_id_flags,
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }
    SyscallResult { value, error }
}
/// 終了コードを指定してプロセスを終了する。
pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code);
}

/// システムコール 0x80000008: ウィンドウに線を描画 (WinDrawLine)
pub fn sys_win_draw_line(
    layer_id_flags: u64,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) -> SyscallResult {
    let mut value: u64;
    let mut error: i32;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0008u32,
            in("rdi") layer_id_flags,
            in("rsi") x0 as u64,
            in("rdx") y0 as u64,
            in("r10") x1 as u64,
            in("r8") y1 as u64,
            in("r9") color as u64,
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }
    SyscallResult { value, error }
}
/// システムコール 0x80000009: ウィンドウを閉じる (CloseWindow)
pub fn sys_close_window(layer_id_flags: u64) -> SyscallResult {
    let mut value: u64;
    let mut error: i32;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0009u32,
            in("rdi") layer_id_flags,
            out("rcx") _,
            out("r11") _,
            lateout("rax") value,
            lateout("rdx") error,
            clobber_abi("C"),
        );
    }
    SyscallResult { value, error }
}
