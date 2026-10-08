use crate::global::Global;
use core::ptr::write_volatile;

use crate::queue::Queue;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

const INTERRUPTVECTOR_KXHCI: u8 = 0x40; // そう定義されている。

// 割り込み記述子表
static IDT: Global<InterruptDescriptorTable> = Global::new(InterruptDescriptorTable::new());

extern "x86-interrupt" fn interrupt_handler_xhchi_mouse_cursor(_stack_frame: InterruptStackFrame) {
    // キューが満杯なら取りこぼす (xHCI イベントは次の割り込みでまとめて処理される)
    let _ = INTERRUPTMESSAGE_QUEUE.with(|q| q.push(InterruptMessage::InterruptXhci));
    unsafe {
        // ここに値を書き込むと割り込み処理完了を知らせることが出来る。
        write_volatile(0xFEE000B0 as *mut u32, 0);
    }
}
pub fn init_idt() {
    IDT.init_static(|idt| {
        idt[INTERRUPTVECTOR_KXHCI].set_handler_fn(interrupt_handler_xhchi_mouse_cursor);
        idt.load();
    });
}
#[derive(Clone, Copy)]
pub enum InterruptMessage {
    InterruptXhci,
}

pub static INTERRUPTMESSAGE_QUEUE: Global<Queue<InterruptMessage>> = Global::new(Queue::new());
