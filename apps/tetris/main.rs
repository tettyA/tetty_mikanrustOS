#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{
    app_event, exit, sys_close_window, sys_create_timer, sys_open_window, sys_read_event,
    sys_win_fill_rectangle, sys_win_redraw, sys_win_write_string, AppEvent, LAYER_NO_REDRAW,
    TIMER_ONESHOT_ABS, TIMER_ONESHOT_REL,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const FIELD_WIDTH: usize = 10;
const FIELD_HEIGHT: usize = 20;
const CELL_SIZE: i32 = 12;

const FIELD_X: i32 = 10;
const FIELD_Y: i32 = 10;

const CANVAS_WIDTH: i32 = 240;
const CANVAS_HEIGHT: i32 = 260;

const FRAME_RATE: u64 = 60;
const NORMAL_DROP_FRAMES: u32 = 30; // 通常時: 0.5秒ごとに1マス落下
const FAST_DROP_FRAMES: u32 = 2; // 下キー押下時: 高速落下

// 7種類のテトリミノ (4x4 配列)
const PIECES: [[[u8; 4]; 4]; 7] = [
    // 0: I (シアン)
    [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // 1: O (黄)
    [
        [0, 2, 2, 0],
        [0, 2, 2, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // 2: T (紫)
    [
        [0, 3, 0, 0],
        [3, 3, 3, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // 3: S (緑)
    [
        [0, 4, 4, 0],
        [4, 4, 0, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // 4: Z (赤)
    [
        [5, 5, 0, 0],
        [0, 5, 5, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // 5: J (青)
    [
        [6, 0, 0, 0],
        [6, 6, 6, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
    // 6: L (オレンジ)
    [
        [0, 0, 7, 0],
        [7, 7, 7, 0],
        [0, 0, 0, 0],
        [0, 0, 0, 0],
    ],
];

const COLORS: [u32; 8] = [
    0x000000, // 0: 空白 (黒)
    0x00f0f0, // 1: I (シアン)
    0xf0f000, // 2: O (黄)
    0xa000f0, // 3: T (紫)
    0x00f000, // 4: S (緑)
    0xf00000, // 5: Z (赤)
    0x0000f0, // 6: J (青)
    0xf0a000, // 7: L (オレンジ)
];

struct Rng {
    state: u32,
}

impl Rng {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x98765432 } else { seed },
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

    fn gen_piece_kind(&mut self) -> usize {
        (self.next_u32() as usize) % 7
    }
}

/// 整数値を文字列にフォーマットするヘルパー
fn u32_to_str(mut val: u32, buf: &mut [u8; 16]) -> &str {
    if val == 0 {
        buf[0] = b'0';
        return core::str::from_utf8(&buf[..1]).unwrap_or("0");
    }
    let mut len = 0;
    let mut temp = [0u8; 16];
    while val > 0 {
        temp[len] = b'0' + (val % 10) as u8;
        val /= 10;
        len += 1;
    }
    for i in 0..len {
        buf[i] = temp[len - 1 - i];
    }
    core::str::from_utf8(&buf[..len]).unwrap_or("")
}

/// 時計回りに 90 度回転させたテトリミノを生成
fn rotate(piece: &[[u8; 4]; 4], kind: usize) -> [[u8; 4]; 4] {
    let mut res = [[0u8; 4]; 4];
    if kind == 1 {
        // O型は回転しない
        return *piece;
    }
    if kind == 0 {
        // I型 (4x4)
        for y in 0..4 {
            for x in 0..4 {
                res[x][3 - y] = piece[y][x];
            }
        }
    } else {
        // 3x3 (T, S, Z, J, L)
        for y in 0..3 {
            for x in 0..3 {
                res[x][2 - y] = piece[y][x];
            }
        }
    }
    res
}

/// 指定座標 (px, py) にピースを配置可能か判定
fn can_place(
    field: &[[u8; FIELD_WIDTH]; FIELD_HEIGHT],
    piece: &[[u8; 4]; 4],
    px: i32,
    py: i32,
) -> bool {
    for y in 0..4 {
        for x in 0..4 {
            if piece[y][x] != 0 {
                let fx = px + x as i32;
                let fy = py + y as i32;
                if fx < 0 || fx >= FIELD_WIDTH as i32 || fy < 0 || fy >= FIELD_HEIGHT as i32 {
                    return false;
                }
                if field[fy as usize][fx as usize] != 0 {
                    return false;
                }
            }
        }
    }
    true
}

/// ピースをフィールドに固定
fn lock_piece(
    field: &mut [[u8; FIELD_WIDTH]; FIELD_HEIGHT],
    piece: &[[u8; 4]; 4],
    px: i32,
    py: i32,
) {
    for y in 0..4 {
        for x in 0..4 {
            if piece[y][x] != 0 {
                let fx = px + x as i32;
                let fy = py + y as i32;
                if fx >= 0 && fx < FIELD_WIDTH as i32 && fy >= 0 && fy < FIELD_HEIGHT as i32 {
                    field[fy as usize][fx as usize] = piece[y][x];
                }
            }
        }
    }
}

/// 横一列揃ったラインを消去し、消去したライン数を返す
fn clear_lines(field: &mut [[u8; FIELD_WIDTH]; FIELD_HEIGHT]) -> u32 {
    let mut cleared = 0;
    let mut y = FIELD_HEIGHT as i32 - 1;
    while y >= 0 {
        let is_full = field[y as usize].iter().all(|&c| c != 0);
        if is_full {
            cleared += 1;
            for row in (1..=y as usize).rev() {
                field[row] = field[row - 1];
            }
            field[0] = [0; FIELD_WIDTH];
        } else {
            y -= 1;
        }
    }
    cleared
}

/// セルを1つ描画 (1ピクセル内側に隙間を空けて立体感を出す)
fn draw_cell(layer_id: u64, px: i32, py: i32, color_idx: u8) {
    let c = COLORS[color_idx as usize % 8];
    sys_win_fill_rectangle(
        layer_id,
        4 + px + 1,
        24 + py + 1,
        CELL_SIZE - 2,
        CELL_SIZE - 2,
        c,
    );
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> ! {
    let res = sys_open_window(
        (CANVAS_WIDTH + 8) as usize,
        (CANVAS_HEIGHT + 28) as usize,
        10,
        10,
        "tetris",
    );
    if res.error != 0 {
        exit(res.error);
    }
    let layer_id = res.value;

    let mut rng = Rng::new(0x12345678);
    let mut field = [[0u8; FIELD_WIDTH]; FIELD_HEIGHT];

    let mut current_kind = rng.gen_piece_kind();
    let mut current_piece = PIECES[current_kind];
    let mut next_kind = rng.gen_piece_kind();

    let mut piece_x: i32 = 3;
    let mut piece_y: i32 = 0;

    let mut score: u32 = 0;
    let mut lines_cleared: u32 = 0;
    let mut game_over = false;

    let mut fast_drop = false;
    let mut drop_counter: u32 = 0;

    let mut prev_timeout = 0u64;
    let mut buf = [0u8; 16];

    loop {
        // 背景クリア (黒)
        sys_win_fill_rectangle(
            layer_id | LAYER_NO_REDRAW,
            4,
            24,
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            0x000000,
        );

        // フィールド枠線描画 (グレー)
        sys_win_fill_rectangle(
            layer_id | LAYER_NO_REDRAW,
            4 + FIELD_X - 2,
            24 + FIELD_Y - 2,
            FIELD_WIDTH as i32 * CELL_SIZE + 4,
            FIELD_HEIGHT as i32 * CELL_SIZE + 4,
            0x444444,
        );
        sys_win_fill_rectangle(
            layer_id | LAYER_NO_REDRAW,
            4 + FIELD_X,
            24 + FIELD_Y,
            FIELD_WIDTH as i32 * CELL_SIZE,
            FIELD_HEIGHT as i32 * CELL_SIZE,
            0x111111,
        );

        // フィールド内の固定ブロックを描画
        for y in 0..FIELD_HEIGHT {
            for x in 0..FIELD_WIDTH {
                let c = field[y][x];
                if c != 0 {
                    draw_cell(
                        layer_id | LAYER_NO_REDRAW,
                        FIELD_X + x as i32 * CELL_SIZE,
                        FIELD_Y + y as i32 * CELL_SIZE,
                        c,
                    );
                }
            }
        }

        // 落下中のテトリミノを描画
        if !game_over {
            for y in 0..4 {
                for x in 0..4 {
                    let c = current_piece[y][x];
                    if c != 0 {
                        let draw_x = FIELD_X + (piece_x + x as i32) * CELL_SIZE;
                        let draw_y = FIELD_Y + (piece_y + y as i32) * CELL_SIZE;
                        draw_cell(layer_id | LAYER_NO_REDRAW, draw_x, draw_y, c);
                    }
                }
            }
        }

        // サイドバー: タイトル・スコア表示
        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 12, 0x00ffff, "TETRIS");

        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 36, 0xaaaaaa, "SCORE");
        let score_str = u32_to_str(score, &mut buf);
        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 52, 0xffffff, score_str);

        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 76, 0xaaaaaa, "LINES");
        let lines_str = u32_to_str(lines_cleared, &mut buf);
        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 92, 0xffffff, lines_str);

        // NEXT テトリミノ表示
        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 120, 0xaaaaaa, "NEXT");
        let next_piece = &PIECES[next_kind];
        for ny in 0..4 {
            for nx in 0..4 {
                let c = next_piece[ny][nx];
                if c != 0 {
                    draw_cell(
                        layer_id | LAYER_NO_REDRAW,
                        148 + nx as i32 * CELL_SIZE,
                        138 + ny as i32 * CELL_SIZE,
                        c,
                    );
                }
            }
        }

        // 操作ガイド
        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 204, 0x666666, "[<][>] Move");
        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 220, 0x666666, "[^]/Sp Rot");
        sys_win_write_string(layer_id | LAYER_NO_REDRAW, 148, 236, 0x666666, "[v]    Drop");

        // ゲームオーバー表示
        if game_over {
            sys_win_fill_rectangle(
                layer_id | LAYER_NO_REDRAW,
                4 + FIELD_X + 8,
                24 + FIELD_Y + 90,
                104,
                40,
                0x000000,
            );
            sys_win_write_string(
                layer_id | LAYER_NO_REDRAW,
                FIELD_X + 24,
                FIELD_Y + 96,
                0xff0000,
                "GAME OVER",
            );
            sys_win_write_string(
                layer_id | LAYER_NO_REDRAW,
                FIELD_X + 16,
                FIELD_Y + 112,
                0xffff00,
                "Press SPACE",
            );
        }

        sys_win_redraw(layer_id);

        // 60 FPS 定期タイマ待機
        if prev_timeout == 0 {
            let timeout = sys_create_timer(TIMER_ONESHOT_REL, 1, 1000 / FRAME_RATE);
            prev_timeout = timeout.value;
        } else {
            prev_timeout += 1000 / FRAME_RATE;
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
                let keycode = kp.keycode;
                let press = kp.press != 0;

                if press {
                    if game_over {
                        // スペースキーでリスタート
                        if keycode == 44 {
                            field = [[0u8; FIELD_WIDTH]; FIELD_HEIGHT];
                            score = 0;
                            lines_cleared = 0;
                            game_over = false;
                            current_kind = rng.gen_piece_kind();
                            current_piece = PIECES[current_kind];
                            next_kind = rng.gen_piece_kind();
                            piece_x = 3;
                            piece_y = 0;
                        }
                    } else {
                        // 左移動: LeftArrow (80) or 'a' (4) or 'h' (11)
                        if keycode == 80 || keycode == 4 || keycode == 11 {
                            if can_place(&field, &current_piece, piece_x - 1, piece_y) {
                                piece_x -= 1;
                            }
                        }
                        // 右移動: RightArrow (79) or 'd' (7) or 'l' (15)
                        else if keycode == 79 || keycode == 7 || keycode == 15 {
                            if can_place(&field, &current_piece, piece_x + 1, piece_y) {
                                piece_x += 1;
                            }
                        }
                        // 回転: UpArrow (82) or Space (44) or 'w' (26) or 'k' (14)
                        else if keycode == 82 || keycode == 44 || keycode == 26 || keycode == 14 {
                            let rotated = rotate(&current_piece, current_kind);
                            if can_place(&field, &rotated, piece_x, piece_y) {
                                current_piece = rotated;
                            } else if can_place(&field, &rotated, piece_x - 1, piece_y) {
                                // 壁蹴り (左に1マス)
                                piece_x -= 1;
                                current_piece = rotated;
                            } else if can_place(&field, &rotated, piece_x + 1, piece_y) {
                                // 壁蹴り (右に1マス)
                                piece_x += 1;
                                current_piece = rotated;
                            }
                        }
                        // 高速落下 ON: DownArrow (81) or 's' (22) or 'j' (13)
                        else if keycode == 81 || keycode == 22 || keycode == 13 {
                            fast_drop = true;
                        }
                    }
                } else {
                    // キー解放: 高速落下 OFF
                    if keycode == 81 || keycode == 22 || keycode == 13 {
                        fast_drop = false;
                    }
                }
            }
        }

        if quit {
            break;
        }

        // 自然落下処理
        if !game_over {
            drop_counter += 1;
            let drop_interval = if fast_drop {
                FAST_DROP_FRAMES
            } else {
                NORMAL_DROP_FRAMES
            };

            if drop_counter >= drop_interval {
                drop_counter = 0;
                if can_place(&field, &current_piece, piece_x, piece_y + 1) {
                    piece_y += 1;
                } else {
                    // 着地してブロック固定
                    lock_piece(&mut field, &current_piece, piece_x, piece_y);
                    let cleared = clear_lines(&mut field);
                    lines_cleared += cleared;
                    match cleared {
                        1 => score += 100,
                        2 => score += 300,
                        3 => score += 500,
                        4 => score += 800,
                        _ => {}
                    }

                    // 新しいピースの出現
                    current_kind = next_kind;
                    current_piece = PIECES[current_kind];
                    next_kind = rng.gen_piece_kind();
                    piece_x = 3;
                    piece_y = 0;

                    // 出現直後に置けなければゲームオーバー
                    if !can_place(&field, &current_piece, piece_x, piece_y) {
                        game_over = true;
                    }
                }
            }
        }
    }

    sys_close_window(layer_id);
    exit(0);
}

