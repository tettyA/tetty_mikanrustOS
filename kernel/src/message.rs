#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Message {
    InterruptXhci,
    InterruptLapicTimer { timeout: u64, value: i32 },
}
