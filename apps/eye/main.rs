#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{
    app_event, exit, print, sys_close_window, sys_open_window, sys_read_event,
    sys_win_fill_rectangle, AppEvent, LAYER_NO_REDRAW,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const CANVAS_SIZE: i32 = 100;
const EYE_SIZE: i32 = 10;

/// ニュートン法による簡易平方根計算 (no_std 環境用)
fn sqrt(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut cur = x;
    for _ in 0..10 {
        cur = 0.5 * (cur + x / cur);
    }
    cur
}

/// 黒目の位置を計算して描画する
fn draw_eye(layer_id_flags: u64, mouse_x: i32, mouse_y: i32, color: u32) {
    // 白目の中心座標に対するマウスの相対位置
    let center_x = (mouse_x - CANVAS_SIZE / 2 - 4) as f64;
    let center_y = (mouse_y - CANVAS_SIZE / 2 - 24) as f64;

    let distance_raw = sqrt(center_x * center_x + center_y * center_y);
    let max_distance = (CANVAS_SIZE / 2 - EYE_SIZE / 2) as f64;

    // 三角関数 (atan2, cos, sin) の代わりにベクトルの比率を用いて計算
    let (eye_center_x, eye_center_y) = if distance_raw > 0.0 {
        let ratio = if distance_raw < max_distance {
            1.0
        } else {
            max_distance / distance_raw
        };
        (center_x * ratio, center_y * ratio)
    } else {
        (0.0, 0.0)
    };

    let eye_x = eye_center_x as i32 + CANVAS_SIZE / 2 + 4;
    let eye_y = eye_center_y as i32 + CANVAS_SIZE / 2 + 24;

    sys_win_fill_rectangle(
        layer_id_flags,
        eye_x - EYE_SIZE / 2,
        eye_y - EYE_SIZE / 2,
        EYE_SIZE,
        EYE_SIZE,
        color,
    );
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> ! {
    let res = sys_open_window(
        (CANVAS_SIZE + 8) as usize,
        (CANVAS_SIZE + 28) as usize,
        10,
        10,
        "eye",
    );
    if res.error != 0 {
        exit(res.error);
    }
    let layer_id = res.value;

    // 白目の背景矩形を描画
    sys_win_fill_rectangle(layer_id, 4, 24, CANVAS_SIZE, CANVAS_SIZE, 0xffffff);

    let mut events = [AppEvent::empty(); 1];
    loop {
        let res = sys_read_event(events.as_mut_ptr(), 1);
        if res.error != 0 {
            print("ReadEvent failed\n");
            break;
        }

        if events[0].event_type == app_event::QUIT {
            break;
        } else if events[0].event_type == app_event::MOUSE_MOVE {
            let arg = events[0].arg.mouse_move;
            // 白目で一度塗りつぶし (再描画フラグなしでバッファのみ更新)
            sys_win_fill_rectangle(
                layer_id | LAYER_NO_REDRAW,
                4,
                24,
                CANVAS_SIZE,
                CANVAS_SIZE,
                0xffffff,
            );
            // 黒目を描画 (この呼び出しで画面全体にフラッシュ)
            draw_eye(layer_id, arg.x, arg.y, 0x000000);
        }
    }

    sys_close_window(layer_id);
    exit(0);
}
