#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall::{
    app_event, exit, print, print_i64, sys_create_timer, sys_read_event, AppEvent,
    TIMER_ONESHOT_REL,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// 文字列を u64 に変換 (Cの atoi/atol 相当)
unsafe fn atou64(mut s: *const u8) -> u64 {
    let mut v = 0u64;
    while *s != 0 {
        let c = *s;
        if (b'0'..=b'9').contains(&c) {
            v = v * 10 + (c - b'0') as u64;
        }
        s = s.add(1);
    }
    v
}

#[no_mangle]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const u8) -> ! {
    if argc <= 1 {
        print("Usage: timer <msec>\n");
        exit(1);
    }

    let duration_ms = atou64(*argv.add(1));
    let timeout = sys_create_timer(TIMER_ONESHOT_REL, 1, duration_ms);
    print("timer created. timeout = ");
    print_i64(timeout.value as i64);
    print("\n");

    let mut events = [AppEvent::empty(); 1];
    loop {
        sys_read_event(events.as_mut_ptr(), 1);
        if events[0].event_type == app_event::TIMER_TIMEOUT {
            print_i64(duration_ms as i64);
            print(" msecs elapsed!\n");
            break;
        }
    }

    exit(0);
}
