#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{exit, sys_open_window, sys_win_write_string};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> ! {
    // 幅 200, 高さ 100, 座標 (10, 10) に "winhello" というタイトルのウィンドウを開く
    let res = sys_open_window(200, 100, 10, 10, "winhello");
    if res.error != 0 {
        exit(res.error);
    }
    let layer_id = res.value as u32;

    // 赤色 (0xc00000)
    sys_win_write_string(layer_id, 7, 24, 0xc00000, "hello world!");
    // 緑色 (0x00c000)
    sys_win_write_string(layer_id, 24, 40, 0x00c000, "hello world!");
    // 青色 (0x0000c0)
    sys_win_write_string(layer_id, 40, 56, 0x0000c0, "hello world!");

    exit(0);
}
