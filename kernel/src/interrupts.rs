use core::ptr::{addr_of_mut, write_volatile};

use crate::usb_process_events;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

unsafe extern "C" {
    fn xhc_primaryeventring_hasfront() -> bool;
}

const INTERRUPTVECTOR_KXHCI: u8 = 0x40; // そう定義されている。

// 割り込み記述子表
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

extern "x86-interrupt" fn interrupt_handler_xhchi_mouse_cursor(stack_frame: InterruptStackFrame) {
    unsafe {
        while xhc_primaryeventring_hasfront() {
            usb_process_events();
        }
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
