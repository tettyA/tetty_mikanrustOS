#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

struct Stack {
    data: [i64; 100],
    ptr: isize,
}

impl Stack {
    fn new() -> Self {
        Self {
            data: [0; 100],
            ptr: -1,
        }
    }

    fn push(&mut self, val: i64) {
        self.ptr += 1;
        self.data[self.ptr as usize] = val;
    }

    fn pop(&mut self) -> i64 {
        let val = self.data[self.ptr as usize];
        self.ptr -= 1;
        val
    }
}

#[repr(C)]
pub struct SyscallResult {
    pub value: u64,
    pub error: i32,
}
pub fn syscall_put_string(fd: u64, buf: *const u8, len: usize) -> SyscallResult {
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
pub fn syscall_exit(exit_code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("eax") 0x8000_0002u32,
            in("rdi") exit_code as u64,
            options(noreturn),
        );
    }
}
fn print(s: &str) {
    syscall_put_string(1, s.as_ptr(), s.len());
}
fn print_i64(mut val: i64) {
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
/// アプリケーションのエントリポイント
/// C言語の呼び出し規約（extern "C"）で公開し、カーネルから呼ばれる
#[no_mangle]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const u8) -> i32 {
    let mut stack = Stack::new();
    let mut i = 1;

    // argv[0] はコマンド名
    while i < argc {
        let arg = *argv.add(i as usize);
        let b0 = *arg;
        let b1 = *arg.add(1);

        if b0 == b'+' && b1 == 0 {
            let b = stack.pop();
            let a = stack.pop();
            stack.push(a + b);
        } else if b0 == b'-' && b1 == 0 {
            let b = stack.pop();
            let a = stack.pop();
            stack.push(a - b);
        } else {
            let a = atol(arg);
            stack.push(a);
        }
        i += 1;
    }
    let mut result = 0;
    if stack.ptr >= 0 {
        result = stack.pop();
    }
    print_i64(result);
    print("\n");
    syscall_exit(result as i32);
    //  stack.pop() as i32
}

/// 文字列を整数に変換
unsafe fn atol(mut s: *const u8) -> i64 {
    let mut v = 0i64;
    while *s != 0 {
        let c = *s;
        if c >= b'0' && c <= b'9' {
            v = v * 10 + (c - b'0') as i64;
        }
        s = s.add(1);
    }
    v
}
