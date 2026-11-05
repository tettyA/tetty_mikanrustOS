#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{
    app_event, exit, sys_close_window, sys_create_timer, sys_open_window, sys_read_event,
    sys_win_fill_rectangle, sys_win_redraw, AppEvent, LAYER_NO_REDRAW, TIMER_ONESHOT_ABS,
    TIMER_ONESHOT_REL,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const PI: f64 = 3.14159265358979323846;

/// テイラー展開による sin 計算 (no_std 用)
fn sin(mut x: f64) -> f64 {
    while x > PI {
        x -= 2.0 * PI;
    }
    while x < -PI {
        x += 2.0 * PI;
    }
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    for i in 1..=8 {
        term *= -x2 / ((2 * i) * (2 * i + 1)) as f64;
        sum += term;
    }
    sum
}

/// テイラー展開による cos 計算 (no_std 用)
fn cos(mut x: f64) -> f64 {
    while x > PI {
        x -= 2.0 * PI;
    }
    while x < -PI {
        x += 2.0 * PI;
    }
    let x2 = x * x;
    let mut term = 1.0;
    let mut sum = 1.0;
    for i in 1..=8 {
        term *= -x2 / ((2 * i - 1) * (2 * i)) as f64;
        sum += term;
    }
    sum
}

/// 傾きに応じた整数座標への丸め関数 (C++ の floor/ceil 相当)
fn roundish(val: f64, dx: i32) -> i32 {
    let i = val as i32;
    if dx >= 0 {
        // floor
        if val < i as f64 {
            i - 1
        } else {
            i
        }
    } else {
        // ceil
        if val > i as f64 {
            i + 1
        } else {
            i
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Vector3D<T> {
    x: T,
    y: T,
    z: T,
}

#[derive(Clone, Copy, Default)]
struct Vector2D<T> {
    x: T,
    y: T,
}

const SCALE: i32 = 50;
const MARGIN: i32 = 10;
const CANVAS_SIZE: usize = (3 * SCALE + MARGIN) as usize; // 160

// 立方体の 8 頂点
const CUBE: [Vector3D<i32>; 8] = [
    Vector3D { x: 1, y: 1, z: 1 },
    Vector3D { x: 1, y: 1, z: -1 },
    Vector3D { x: 1, y: -1, z: 1 },
    Vector3D { x: 1, y: -1, z: -1 },
    Vector3D { x: -1, y: 1, z: 1 },
    Vector3D { x: -1, y: 1, z: -1 },
    Vector3D { x: -1, y: -1, z: 1 },
    Vector3D {
        x: -1,
        y: -1,
        z: -1,
    },
];

// 立方体の 6 面 (頂点インデックスの反時計回り順)
const SURFACES: [[usize; 4]; 6] = [
    [0, 4, 6, 2],
    [1, 3, 7, 5],
    [0, 2, 3, 1],
    [0, 1, 5, 4],
    [4, 5, 7, 6],
    [6, 7, 3, 2],
];

// 面の色 (赤, 緑, 黄, 青, マゼンタ, シアン)
const COLORS: [u32; 6] = [0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff];

/// 1つの面をスキャンライン方式で塗りつぶし描画
fn draw_surface(layer_id: u64, sur: usize, scr: &[Vector2D<i32>; 8]) {
    let surface = SURFACES[sur];
    let mut ymin = CANVAS_SIZE as i32;
    let mut ymax = 0;
    let mut y2x_up = [0i32; CANVAS_SIZE];
    let mut y2x_down = [0i32; CANVAS_SIZE];

    for i in 0..surface.len() {
        let p0 = scr[surface[(i + 3) % 4]];
        let p1 = scr[surface[i]];
        ymin = ymin.min(p1.y);
        ymax = ymax.max(p1.y);
        if p0.y == p1.y {
            continue;
        }

        let (x0, y0, y1, dx, is_up) = if p0.y < p1.y {
            (p0.x, p0.y, p1.y, p1.x - p0.x, true)
        } else {
            (p1.x, p1.y, p0.y, p0.x - p1.x, false)
        };

        let m = dx as f64 / (y1 - y0) as f64;
        let y_start = y0.max(0);
        let y_end = y1.min(CANVAS_SIZE as i32 - 1);
        for y in y_start..=y_end {
            let x = roundish(m * (y - y0) as f64 + x0 as f64, dx);
            if is_up {
                y2x_up[y as usize] = x;
            } else {
                y2x_down[y as usize] = x;
            }
        }
    }

    let y_start = ymin.max(0);
    let y_end = ymax.min(CANVAS_SIZE as i32 - 1);
    for y in y_start..=y_end {
        let p0x = y2x_up[y as usize].min(y2x_down[y as usize]);
        let p1x = y2x_up[y as usize].max(y2x_down[y as usize]);
        sys_win_fill_rectangle(layer_id, 4 + p0x, 24 + y, p1x - p0x + 1, 1, COLORS[sur]);
    }
}

/// 3Dオブジェクトを透視投影変換し、陰面消去を行って描画
fn draw_obj(
    layer_id: u64,
    vert: &[Vector3D<f64>; 8],
    scr: &mut [Vector2D<i32>; 8],
    centerz4: &mut [f64; 6],
) {
    // 3D座標からスクリーン2D座標への透視投影変換
    for i in 0..8 {
        let t = (6 * SCALE) as f64 / (vert[i].z + (8 * SCALE) as f64);
        scr[i].x = (vert[i].x * t) as i32 + CANVAS_SIZE as i32 / 2;
        scr[i].y = (vert[i].y * t) as i32 + CANVAS_SIZE as i32 / 2;
    }

    loop {
        // 最も奥（Zが大きい）面を探す (ペインターズアルゴリズム)
        let mut best_sur = None;
        let mut max_z = f64::NEG_INFINITY;
        for (sur, &z) in centerz4.iter().enumerate() {
            if z > max_z {
                max_z = z;
                best_sur = Some(sur);
            }
        }

        let Some(sur) = best_sur else {
            break;
        };
        centerz4[sur] = f64::NEG_INFINITY;

        // 法線ベクトルの向きをチェック (背面カリング)
        let surface = SURFACES[sur];
        let v0 = vert[surface[0]];
        let v1 = vert[surface[1]];
        let v2 = vert[surface[2]];

        let e0x = v1.x - v0.x;
        let e0y = v1.y - v0.y;
        let e1x = v2.x - v1.x;
        let e1y = v2.y - v1.y;

        if e0x * e1y <= e0y * e1x {
            draw_surface(layer_id, sur, scr);
        }
    }
}

/// 一定時間待機する関数 (終了イベント検知時は true を返す)
fn sleep(prev_timeout: &mut u64, ms: u64) -> bool {
    if *prev_timeout == 0 {
        let timeout = sys_create_timer(TIMER_ONESHOT_REL, 1, ms);
        *prev_timeout = timeout.value;
    } else {
        *prev_timeout += ms;
        sys_create_timer(TIMER_ONESHOT_ABS, 1, *prev_timeout);
    }

    let mut events = [AppEvent::empty(); 1];
    loop {
        sys_read_event(events.as_mut_ptr(), 1);
        if events[0].event_type == app_event::TIMER_TIMEOUT {
            return false;
        } else if events[0].event_type == app_event::QUIT {
            return true;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> ! {
    let res = sys_open_window(CANVAS_SIZE + 8, CANVAS_SIZE + 28, 10, 10, "cube");
    if res.error != 0 {
        exit(res.error);
    }
    let layer_id = res.value;

    let mut vert = [Vector3D::<f64>::default(); 8];
    let mut scr = [Vector2D::<i32>::default(); 8];
    let mut centerz4 = [0.0f64; 6];

    let mut thx = 0i32;
    let mut thy = 0i32;
    let mut thz = 0i32;
    let to_rad = PI / 0x8000 as f64;
    let mut prev_timeout = 0u64;

    loop {
        // 立方体を X, Y, Z 軸回りに回転
        thx = (thx + 182) & 0xffff;
        thy = (thy + 273) & 0xffff;
        thz = (thz + 364) & 0xffff;

        let xp = cos(thx as f64 * to_rad);
        let xa = sin(thx as f64 * to_rad);
        let yp = cos(thy as f64 * to_rad);
        let ya = sin(thy as f64 * to_rad);
        let zp = cos(thz as f64 * to_rad);
        let za = sin(thz as f64 * to_rad);

        let scale = SCALE as f64;
        for i in 0..8 {
            let cv = CUBE[i];
            let zt = scale * cv.z as f64 * xp + scale * cv.y as f64 * xa;
            let yt = scale * cv.y as f64 * xp - scale * cv.z as f64 * xa;
            let xt = scale * cv.x as f64 * yp + zt * ya;
            vert[i].z = zt * yp - scale * cv.x as f64 * ya;
            vert[i].x = xt * zp - yt * za;
            vert[i].y = yt * zp + xt * za;
        }

        // 各面の中心 Z 座標 (を 4 倍した値) を算出
        for sur in 0..6 {
            centerz4[sur] = 0.0;
            for i in 0..4 {
                centerz4[sur] += vert[SURFACES[sur][i]].z;
            }
        }

        // 画面を黒でクリア (再描画なし)
        sys_win_fill_rectangle(
            layer_id | LAYER_NO_REDRAW,
            4,
            24,
            CANVAS_SIZE as i32,
            CANVAS_SIZE as i32,
            0x000000,
        );
        // 立方体を描画 (再描画なし)
        draw_obj(layer_id | LAYER_NO_REDRAW, &vert, &mut scr, &mut centerz4);
        // 完成したバッファを一括反映
        sys_win_redraw(layer_id);

        // 50ms 待機 (約 20 FPS)
        if sleep(&mut prev_timeout, 50) {
            break;
        }
    }

    sys_close_window(layer_id);
    exit(0);
}
