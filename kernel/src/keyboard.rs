use crate::{message::Message, usb};

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

const KEYCODE_MAP_SHIFTED: [char; 256] = {
    let mut map = ['\0'; 256];

    let defined: [char; 104] = [
        '\0', '\0', '\0', '\0', 'A', 'B', 'C', 'D', // '\0'
        'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', // 8
        'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', // 16
        'U', 'V', 'W', 'X', 'Y', 'Z', '!', '@', // 24
        '#', '$', '%', '^', '&', '*', '(', ')', // 32
        '\n', '\x1b', '\x08', '\t', ' ', '_', '+', '{', // 40
        '}', '|', '~', ':', '"', '~', '<', '>', // 48
        '?', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 56
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 64
        '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', // 72
        '\0', '\0', '\0', '\0', '/', '*', '-', '+', // 80
        '\n', '1', '2', '3', '4', '5', '6', '7', // 88
        '8', '9', '\0', '.', '\\', '\0', '\0', '=', // 96
    ];
    let mut i = 0;
    while i < defined.len() {
        map[i] = defined[i];
        i += 1;
    }

    map
};

/// USB HID の修飾キーのビット (`modifier` 引数)。
mod modifier {
    #![allow(
        dead_code,
        reason = "HID 仕様で決まるビット表。現在使うのは SHIFT / RSHIFT のみ"
    )]
    pub const CONTROL: u8 = 0b_0000_0001;
    pub const SHIFT: u8 = 0b_0000_0010;
    pub const ALT: u8 = 0b_0000_0100;
    pub const GUI: u8 = 0b_0000_1000;
    pub const RCONTROL: u8 = 0b_0001_0000;
    pub const RSHIFT: u8 = 0b_0010_0000;
    pub const RALT: u8 = 0b_0100_0000;
    pub const RGUI: u8 = 0b_1000_0000;
}

extern "C" fn keyboard_observer(modifier: u8, keycode: u8, press: bool) {
    if !press {
        return;
    }
    let msg = Message::KeyPush {
        keycode,
        modifier,
        ascii: if modifier & (modifier::SHIFT | modifier::RSHIFT) != 0 {
            KEYCODE_MAP_SHIFTED[keycode as usize]
        } else {
            KEYCODE_MAP[keycode as usize]
        },
    };
    let _ = crate::task::send_message(1, msg);
}

pub fn initialize_keyboard() {
    usb::HIDKeyboardDriver::set_default_observer(keyboard_observer);
}
