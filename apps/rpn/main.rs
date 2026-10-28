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

/// アプリケーションのエントリポイント
/// C言語の呼び出し規約（extern "C"）で公開し、カーネルから呼ばれる
#[no_mangle]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const u8) -> i32 {
    let print_test_addr = 0x0000000000215600 as usize;
    type PrintFn = extern "C" fn(*const u8);
    let print_test: PrintFn = core::mem::transmute(print_test_addr);
    print_test(b"Hello from rpn yeah!\n\0".as_ptr());

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

    if stack.ptr < 0 {
        return 0;
    }

    stack.pop() as i32
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
