#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{
    exit, print, print_i64, sys_get_current_tick, sys_open_window, sys_win_fill_rectangle,
    sys_win_redraw, LAYER_NO_REDRAW,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const WIDTH: usize = 100;
const HEIGHT: usize = 100;

/// 簡易疑似乱数生成器 (Xorshift32)
/// C++ の std::default_random_engine の代替
struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// [min, max] の範囲の乱数を生成
    fn gen_range(&mut self, min: usize, max: usize) -> usize {
        min + (self.next_u32() as usize % (max - min + 1))
    }
}

/// 文字列ポインタから整数へ変換 (Cの atoi 相当)
unsafe fn atoi(mut s: *const u8) -> i32 {
    let mut v = 0i32;
    while *s != 0 {
        let c = *s;
        if c >= b'0' && c <= b'9' {
            v = v * 10 + (c - b'0') as i32;
        }
        s = s.add(1);
    }
    v
}

#[no_mangle]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const u8) -> ! {
    // 幅 100+8, 高さ 100+28 のウィンドウを作成
    let res = sys_open_window(WIDTH + 8, HEIGHT + 28, 10, 10, "stars");
    if res.error != 0 {
        exit(res.error);
    }
    let layer_id = res.value;

    // ウィンドウ背景を黒色 (0x000000) で塗りつぶす
    sys_win_fill_rectangle(
        layer_id | LAYER_NO_REDRAW,
        4,
        24,
        WIDTH as i32,
        HEIGHT as i32,
        0x000000,
    );

    let mut num_stars = 100;
    if argc >= 2 {
        let arg1 = *argv.add(1);
        let n = atoi(arg1);
        if n > 0 {
            num_stars = n;
        }
    }

    let tick_start_res = sys_get_current_tick();
    let tick_start = tick_start_res.value;
    let timer_freq = tick_start_res.error as u64;

    let mut rng = SimpleRng::new(12345);
    for _ in 0..num_stars {
        let x = rng.gen_range(0, WIDTH - 2) as i32;
        let y = rng.gen_range(0, HEIGHT - 2) as i32;
        // 2x2 の黄色い星 (0xfff100) を描画
        sys_win_fill_rectangle(layer_id | LAYER_NO_REDRAW, 4 + x, 24 + y, 2, 2, 0xfff100);
    }

    sys_win_redraw(layer_id);
    let tick_end = sys_get_current_tick().value;
    let elapsed_ms = (tick_end - tick_start) * 1000 / timer_freq;
    print_i64(num_stars as i64);
    print(" stars in ");
    print_i64(elapsed_ms as i64);
    print(" ms.\n");
    exit(0);
}
