#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerOperation {
    Move { x: i32, y: i32 },
    MoveRelative { x: i32, y: i32 },
    Draw,
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
