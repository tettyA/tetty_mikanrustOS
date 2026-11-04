#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{exit, sys_open_window, sys_win_draw_line};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const RADIUS: i32 = 90;

/// 角度 (0〜180度) に応じたグラデーション色 (0xRRGGBB) を計算
fn color(deg: i32) -> u32 {
    if deg <= 30 {
        ((255 * deg / 30) << 8) as u32 | 0xff0000
    } else if deg <= 60 {
        ((255 * (60 - deg) / 30) << 16) as u32 | 0x00ff00
    } else if deg <= 90 {
        (255 * (deg - 60) / 30) as u32 | 0x00ff00
    } else if deg <= 120 {
        ((255 * (120 - deg) / 30) << 8) as u32 | 0x0000ff
    } else if deg <= 150 {
        ((255 * (deg - 120) / 30) << 16) as u32 | 0x0000ff
    } else {
        (255 * (180 - deg) / 30) as u32 | 0xff0000
    }
}

/// 半径 90 の円弧上の座標 (x = 90 * cos, y = 90 * sin)
/// 0 度から 90 度まで 5 度刻みの事前計算テーブル
const COS_SIN_TABLE: [(i32, i32); 19] = [
    (90, 0),  //  0 deg
    (89, 7),  //  5 deg
    (88, 15), // 10 deg
    (86, 23), // 15 deg
    (84, 30), // 20 deg
    (81, 38), // 25 deg
    (77, 44), // 30 deg
    (73, 51), // 35 deg
    (68, 57), // 40 deg
    (63, 63), // 45 deg
    (57, 68), // 50 deg
    (51, 73), // 55 deg
    (45, 77), // 60 deg
    (38, 81), // 65 deg
    (30, 84), // 70 deg
    (23, 86), // 75 deg
    (15, 88), // 80 deg
    (7, 89),  // 85 deg
    (0, 90),  // 90 deg
];

#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> ! {
    let res = sys_open_window(
        (RADIUS * 2 + 10 + 8) as usize,
        (RADIUS + 28) as usize,
        10,
        10,
        "lines",
    );
    if res.error != 0 {
        exit(res.error);
    }
    let layer_id = res.value;

    let x0 = 4;
    let y0 = 24;
    let x1 = 4 + RADIUS + 10;
    let y1 = 24 + RADIUS;

    for (i, &(x, y)) in COS_SIN_TABLE.iter().enumerate() {
        let deg = (i * 5) as i32;
        // 左側の扇形
        sys_win_draw_line(layer_id, x0, y0, x0 + x, y0 + y, color(deg));
        // 右側の扇形 (90度回転)
        sys_win_draw_line(layer_id, x1, y1, x1 + x, y1 - y, color(deg + 90));
    }

    exit(0);
}
