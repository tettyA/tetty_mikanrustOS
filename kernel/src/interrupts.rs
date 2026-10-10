use crate::global::Global;
use core::ptr::write_volatile;

use crate::queue::Queue;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

//const INTERRUPTVECTOR_KXHCI: u8 = 0x40; // そう定義されている。
pub enum InterruptVector {
    XHCI = 0x40,
    LAPICTimer = 0x41,
}
#[derive(Clone, Copy)]
pub enum InterruptMessage {
    InterruptXhci,
    InterruptLapicTimer,
}
// 割り込み記述子表
static IDT: Global<InterruptDescriptorTable> = Global::new(InterruptDescriptorTable::new());

extern "x86-interrupt" fn interrupt_handler_xhchi_mouse_cursor(_stack_frame: InterruptStackFrame) {
    // キューが満杯なら取りこぼす (xHCI イベントは次の割り込みでまとめて処理される)
    let _ = INTERRUPTMESSAGE_QUEUE.with(|q| q.push(InterruptMessage::InterruptXhci));
    notify_end_of_interrupt();
}

extern "x86-interrupt" fn interrupt_handler_lapic_timer(_stack_frame: InterruptStackFrame) {
    // キューが満杯なら取りこぼす (xHCI イベントは次の割り込みでまとめて処理される)
    let _ = INTERRUPTMESSAGE_QUEUE.with(|q| q.push(InterruptMessage::InterruptLapicTimer));
    notify_end_of_interrupt();
}
fn notify_end_of_interrupt() {
    unsafe {
        // ここに値を書き込むと割り込み処理完了を知らせることが出来る。
        write_volatile(0xFEE000B0 as *mut u32, 0);
    }
}

pub fn init_idt() {
    IDT.init_static(|idt| {
        idt[InterruptVector::XHCI as u8].set_handler_fn(interrupt_handler_xhchi_mouse_cursor);
        idt[InterruptVector::LAPICTimer as u8].set_handler_fn(interrupt_handler_lapic_timer);
        idt.load();
    });
}

pub static INTERRUPTMESSAGE_QUEUE: Global<Queue<InterruptMessage>> = Global::new(Queue::new());
