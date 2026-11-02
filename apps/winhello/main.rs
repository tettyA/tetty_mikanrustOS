#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{sys_exit, sys_open_window};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> ! {
    // 幅 200, 高さ 100, 座標 (10, 10) に "winhello" というタイトルのウィンドウを開く
    sys_open_window(200, 100, 10, 10, "winhello");
    sys_exit(0);
}
