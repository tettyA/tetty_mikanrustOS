#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{
    app_event, exit, print, sys_close_window, sys_open_window, sys_read_event, sys_win_draw_line,
    sys_win_fill_rectangle, AppEvent,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const WIDTH: i32 = 200;
const HEIGHT: i32 = 130;

fn is_inside(x: i32, y: i32) -> bool {
    4 <= x && x < 4 + WIDTH && 24 <= y && y < 24 + HEIGHT
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> ! {
    let res = sys_open_window(
        (WIDTH + 8) as usize,
        (HEIGHT + 28) as usize,
        10,
        10,
        "paint",
    );
    if res.error != 0 {
        exit(res.error);
    }
    let layer_id = res.value;

    let mut events = [AppEvent::empty(); 1];
    let mut press = false;

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
            let prev_x = arg.x - arg.dx;
            let prev_y = arg.y - arg.dy;
            if press && is_inside(prev_x, prev_y) && is_inside(arg.x, arg.y) {
                sys_win_draw_line(layer_id, prev_x, prev_y, arg.x, arg.y, 0x000000);
            }
        } else if events[0].event_type == app_event::MOUSE_BUTTON {
            let arg = events[0].arg.mouse_button;
            if arg.button == 0 {
                press = arg.press != 0;
                sys_win_fill_rectangle(layer_id, arg.x, arg.y, 1, 1, 0x000000);
            }
        }
    }

    sys_close_window(layer_id);
    exit(0);
}
