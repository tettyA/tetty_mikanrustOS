#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Message {
    InterruptXhci,
    TimerTimeout {
        timeout: u64,
        value: i32,
    },
    KeyPush {
        keycode: u8,
        modifier: u8,
        ascii: char,
    },
}
