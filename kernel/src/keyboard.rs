use crate::global::Global;

use crate::{message::Message, queue::Queue, usb};

const KEYCODE_MAP: [char; 256] = {
    let mut map = ['\0'; 256];

    let defined: [char; 104] = [
        '\0', '\0', '\0', '\0', 'a', 'b', 'c', 'd', // '\0'
        'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', // 8
        'm', 'n', 'o', 'p', 'q', 'r', 's', 't', // 16
        'u', 'v', 'w', 'x', 'y', 'z', '1', '2', // 24
        '3', '4', '5', '6', '7', '8', '9', '0', // 32
        '\n', '\x1b', '\x08', '\t', ' ', '-', '=', '[', // 40
        ']', '\\', '#', ';', '\'', '`', ',', '.', // 48
        '/', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 56
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 64
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 72
        '\0', '\0', '\0', '\0', '/', '*', '-', '+', // 80
        '\n', '1', '2', '3', '4', '5', '6', '7', // 88
        '8', '9', '0', '.', '\\', '\0', '\0', '=', // 96
    ];
    let mut i = 0;
    while i < defined.len() {
        map[i] = defined[i];
        i += 1;
    }

    map
};

static mut MSG_QUEUE: Option<&'static Global<Queue<Message>>> = None;

extern "C" fn keyboard_observer(_modifier: u8, keycode: u8, press: bool) {
    if !press {
        return;
    }
    let msg = Message::KeyPush {
        keycode,
        ascii: KEYCODE_MAP[keycode as usize],
    };
    unsafe {
        if let Some(msg_queue) = MSG_QUEUE {
            msg_queue.with(|q| {
                let _ = q.push(msg);
            });
        }
    }
}

pub fn initialize_keyboard(msg_queue: &'static Global<Queue<Message>>) {
    unsafe {
        MSG_QUEUE = Some(&*msg_queue);
    }
    usb::HIDKeyboardDriver::set_default_observer(keyboard_observer);
}
