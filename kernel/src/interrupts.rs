use core::ptr::{addr_of_mut, write_volatile};

use crate::{queue::Queue, usb_process_events};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

const INTERRUPTVECTOR_KXHCI: u8 = 0x40; // そう定義されている。

// 割り込み記述子表
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

extern "x86-interrupt" fn interrupt_handler_xhchi_mouse_cursor(stack_frame: InterruptStackFrame) {
    unsafe {
        let _ = (&mut *addr_of_mut!(INTERRUPTMESSAGE_QUEUE)).push(InterruptMessage::InterruptXhci);
    }
    unsafe {
        // ここに値を書き込むと割り込み処理完了を知らせることが出来る。
        write_volatile(0xFEE000B0 as *mut u32, 0);
    }
}
pub fn init_idt() {
    unsafe {
        let idt = &mut *addr_of_mut!(IDT);
        idt[INTERRUPTVECTOR_KXHCI].set_handler_fn(interrupt_handler_xhchi_mouse_cursor);
        idt.load();
    }
}
#[derive(Clone, Copy)]
pub enum InterruptMessage {
    InterruptXhci,
}

pub static mut INTERRUPTMESSAGE_QUEUE: Queue<InterruptMessage> = Queue::new();
