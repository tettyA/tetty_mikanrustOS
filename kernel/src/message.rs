use crate::graphics::Rectangle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "Move / MoveRelative / Draw は process_layer_message が処理するが、現状送信側は DrawArea のみ"
)]
pub enum LayerOperation {
    Move { x: i32, y: i32 },
    MoveRelative { x: i32, y: i32 },
    Draw,
    DrawArea { area: Rectangle<i32> },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    Layer {
        op: LayerOperation,
        layer_id: u32,
        src_task: u64, // 走査官僚通知(layerFinish)を返信してもう為の送信元タスクID
    },
    LayerFinish,
}
