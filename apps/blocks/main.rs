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

fn round(x: f64) -> i32 {
    if x >= 0.0 {
        (x + 0.5) as i32
    } else {
        (x - 0.5) as i32
    }
}

const NUM_BLOCKS_X: usize = 10;
const NUM_BLOCKS_Y: usize = 5;
const BLOCK_WIDTH: i32 = 20;
const BLOCK_HEIGHT: i32 = 10;
const BAR_WIDTH: i32 = 30;
const BAR_HEIGHT: i32 = 5;
const BALL_RADIUS: i32 = 5;
const GAP_WIDTH: i32 = 30;
const GAP_HEIGHT: i32 = 30;
const GAP_BAR: i32 = 80;
const BAR_FLOAT: i32 = 10;

const CANVAS_WIDTH: i32 = NUM_BLOCKS_X as i32 * BLOCK_WIDTH + 2 * GAP_WIDTH; // 260
const CANVAS_HEIGHT: i32 =
    GAP_HEIGHT + NUM_BLOCKS_Y as i32 * BLOCK_HEIGHT + GAP_BAR + BAR_HEIGHT + BAR_FLOAT; // 175
const BAR_Y: i32 = CANVAS_HEIGHT - BAR_FLOAT - BAR_HEIGHT; // 160

const FRAME_RATE: i32 = 60;
const BAR_SPEED: i32 = CANVAS_WIDTH / 2; // 130
const BALL_SPEED: i32 = BAR_SPEED;

fn draw_blocks(layer_id: u64, blocks: &[[bool; NUM_BLOCKS_X]; NUM_BLOCKS_Y]) {
    for by in 0..NUM_BLOCKS_Y {
        let y = 24 + GAP_HEIGHT + by as i32 * BLOCK_HEIGHT;
        let color: u32 = 0xff << ((by % 3) * 8);

        for bx in 0..NUM_BLOCKS_X {
            if blocks[by][bx] {
                let x = 4 + GAP_WIDTH + bx as i32 * BLOCK_WIDTH;
                let c: u32 = color | (0xff << (((bx + by) % 3) * 8));
                sys_win_fill_rectangle(layer_id, x, y, BLOCK_WIDTH, BLOCK_HEIGHT, c);
            }
        }
    }
}

fn draw_bar(layer_id: u64, bar_x: i32) {
    sys_win_fill_rectangle(
        layer_id,
        4 + bar_x,
        24 + BAR_Y,
        BAR_WIDTH,
        BAR_HEIGHT,
        0xffffff,
    );
}

fn draw_ball(layer_id: u64, x: i32, y: i32) {
    sys_win_fill_rectangle(
        layer_id,
        4 + x - BALL_RADIUS,
        24 + y - BALL_RADIUS,
        2 * BALL_RADIUS,
        2 * BALL_RADIUS,
        0x007f00,
    );
    sys_win_fill_rectangle(
        layer_id,
        4 + x - BALL_RADIUS / 2,
        24 + y - BALL_RADIUS / 2,
        BALL_RADIUS,
        BALL_RADIUS,
        0x00ff00,
    );
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> ! {
    let res = sys_open_window(
        (CANVAS_WIDTH + 8) as usize,
        (CANVAS_HEIGHT + 28) as usize,
        10,
        10,
        "blocks",
    );
    if res.error != 0 {
        exit(res.error);
    }
    let layer_id = res.value;

    let mut blocks = [[true; NUM_BLOCKS_X]; NUM_BLOCKS_Y];

    let ball_init_x = CANVAS_WIDTH / 2 - BALL_RADIUS - 20;
    let ball_init_y = CANVAS_HEIGHT - BAR_FLOAT - BAR_HEIGHT - BALL_RADIUS - 20;

    let mut bar_x = CANVAS_WIDTH / 2 - BAR_WIDTH / 2;
    let mut ball_x = ball_init_x;
    let mut ball_y = ball_init_y;
    let mut move_dir = 0i32; // -1: left, 1: right
    let mut ball_dir = 0i32; // degree (0 は停止)
    let mut ball_dx = 0i32;
    let mut ball_dy = 0i32;

    let mut prev_timeout = 0u64;

    loop {
        // 背景クリア
        sys_win_fill_rectangle(
            layer_id | LAYER_NO_REDRAW,
            4,
            24,
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            0x000000,
        );

        draw_blocks(layer_id | LAYER_NO_REDRAW, &blocks);
        draw_bar(layer_id | LAYER_NO_REDRAW, bar_x);
        if ball_y >= 0 {
            draw_ball(layer_id | LAYER_NO_REDRAW, ball_x, ball_y);
        }
        sys_win_redraw(layer_id);

        // 60 FPS 定期タイマ
        if prev_timeout == 0 {
            let timeout = sys_create_timer(TIMER_ONESHOT_REL, 1, 1000 / FRAME_RATE as u64);
            prev_timeout = timeout.value;
        } else {
            prev_timeout += 1000 / FRAME_RATE as u64;
            sys_create_timer(TIMER_ONESHOT_ABS, 1, prev_timeout);
        }

        let mut events = [AppEvent::empty(); 1];
        let mut quit = false;
        loop {
            sys_read_event(events.as_mut_ptr(), 1);
            if events[0].event_type == app_event::TIMER_TIMEOUT {
                break;
            } else if events[0].event_type == app_event::QUIT {
                quit = true;
                break;
            } else if events[0].event_type == app_event::KEY_PUSH {
                let kp = events[0].arg.keypush;
                if kp.press == 0 {
                    // キー解放
                    move_dir = 0;
                } else {
                    // キー押下
                    let keycode = kp.keycode;
                    if keycode == 79 {
                        // RightArrow
                        move_dir = 1;
                    } else if keycode == 80 {
                        // LeftArrow
                        move_dir = -1;
                    } else if keycode == 44 {
                        // SpaceBar
                        if ball_dir == 0 && ball_y < 0 {
                            ball_x = ball_init_x;
                            ball_y = ball_init_y;
                        } else if ball_dir == 0 {
                            ball_dir = 45;
                        }
                    }
                    if bar_x == 0 && move_dir < 0 {
                        move_dir = 0;
                    } else if bar_x + BAR_WIDTH == CANVAS_WIDTH - 1 && move_dir > 0 {
                        move_dir = 0;
                    }
                }
            }
        }

        if quit {
            break;
        }

        bar_x += move_dir * BAR_SPEED / FRAME_RATE;
        bar_x = bar_x.clamp(0, CANVAS_WIDTH - BAR_WIDTH - 1);

        if ball_dir == 0 {
            continue;
        }

        let ball_x_ = ball_x + ball_dx;
        let ball_y_ = ball_y + ball_dy;

        // 壁との反射
        if (ball_dx < 0 && ball_x_ < BALL_RADIUS)
            || (ball_dx > 0 && CANVAS_WIDTH - BALL_RADIUS <= ball_x_)
        {
            ball_dir = 180 - ball_dir;
        }

        // 天井またはバーとの反射、落下
        if ball_dy < 0 && ball_y_ < BALL_RADIUS {
            ball_dir = -ball_dir;
        } else if bar_x <= ball_x_
            && ball_x_ < bar_x + BAR_WIDTH
            && ball_dy > 0
            && BAR_Y - BALL_RADIUS <= ball_y_
        {
            ball_dir = -ball_dir;
        } else if ball_dy > 0 && CANVAS_HEIGHT - BALL_RADIUS <= ball_y_ {
            ball_dir = 0;
            ball_y = -1;
            continue;
        }

        // ブロックとの衝突判定
        if ball_x_ >= GAP_WIDTH
            && ball_x_ < CANVAS_WIDTH - GAP_WIDTH
            && ball_y_ >= GAP_HEIGHT
            && ball_y_ < GAP_HEIGHT + NUM_BLOCKS_Y as i32 * BLOCK_HEIGHT
        {
            let index_x = ((ball_x_ - GAP_WIDTH) / BLOCK_WIDTH) as usize;
            let index_y = ((ball_y_ - GAP_HEIGHT) / BLOCK_HEIGHT) as usize;

            if index_x < NUM_BLOCKS_X && index_y < NUM_BLOCKS_Y && blocks[index_y][index_x] {
                blocks[index_y][index_x] = false;

                let block_left = GAP_WIDTH + index_x as i32 * BLOCK_WIDTH;
                let block_right = GAP_WIDTH + (index_x as i32 + 1) * BLOCK_WIDTH;
                let block_top = GAP_HEIGHT + index_y as i32 * BLOCK_HEIGHT;
                let block_bottom = GAP_HEIGHT + (index_y as i32 + 1) * BLOCK_HEIGHT;

                if (ball_x < block_left && block_left <= ball_x_)
                    || (block_right < ball_x && ball_x_ <= block_right)
                {
                    ball_dir = 180 - ball_dir;
                }
                if (ball_y < block_top && block_top <= ball_y_)
                    || (block_bottom < ball_y && ball_y_ <= block_bottom)
                {
                    ball_dir = -ball_dir;
                }
            }
        }

        let rad = PI * ball_dir as f64 / 180.0;
        ball_dx = round(BALL_SPEED as f64 * cos(rad) / FRAME_RATE as f64);
        ball_dy = round(BALL_SPEED as f64 * sin(rad) / FRAME_RATE as f64);
        ball_x += ball_dx;
        ball_y += ball_dy;
    }

    sys_close_window(layer_id);
    exit(0);
}
