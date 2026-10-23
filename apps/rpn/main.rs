#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static mut STACK_PTR: isize = -1;
static mut STACK: [i64; 100] = [0; 100];
unsafe fn pop() -> i64 {
    let val = *STACK.as_ptr().offset(STACK_PTR);
    STACK_PTR -= 1;
    val
}
unsafe fn push(val: i64) {
    STACK_PTR += 1;
    *STACK.as_mut_ptr().offset(STACK_PTR) = val;
}

/// アプリケーションのエントリポイント
/// C言語の呼び出し規約（extern "C"）で公開し、カーネルから呼ばれる
#[no_mangle]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const u8) -> i32 {
    STACK_PTR = -1;
    let mut i = 1;

    // argv[0] はコマンド名
    while i < argc {
        let arg = *argv.add(i as usize);
        let b0 = *arg;
        let b1 = *arg.add(1);

        if b0 == b'+' && b1 == 0 {
            let b = pop();
            let a = pop();
            push(a + b);
        } else if b0 == b'-' && b1 == 0 {
            let b = pop();
            let a = pop();
            push(a - b);
        } else {
            let a = atol(arg);
            push(a);
        }
        i += 1;
    }

    if STACK_PTR < 0 {
        return 0;
    }

    pop() as i32
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
