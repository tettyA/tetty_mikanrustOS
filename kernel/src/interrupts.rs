use crate::global::Global;
use core::ptr::write_volatile;

use crate::message::Message;
use x86_64::{
    VirtAddr,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
};
//const INTERRUPTVECTOR_KXHCI: u8 = 0x40; // そう定義されている。
pub enum InterruptVector {
    XHCI = 0x40,
    LAPICTimer = 0x41,
}

// 割り込み記述子表
static IDT: Global<InterruptDescriptorTable> = Global::new(InterruptDescriptorTable::new());

extern "x86-interrupt" fn interrupt_handler_xhchi_mouse_cursor(_stack_frame: InterruptStackFrame) {
    // キューが満杯なら取りこぼす (xHCI イベントは次の割り込みでまとめて処理される)
    let _ = crate::task::send_message(1, Message::InterruptXhci);
    notify_end_of_interrupt();
}
/// アセンブリの IntHandlerLAPICTimer から呼ばれる
#[unsafe(no_mangle)]
pub extern "C" fn lapic_timer_on_interrupt(ctx: *const crate::task::TaskContext) {
    let task_timer_timeout = crate::timer::lapic_timer_on_interrupt();
    notify_end_of_interrupt();
    if task_timer_timeout {
        crate::task::switch_task(ctx);
    }
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

        unsafe {
            idt[InterruptVector::LAPICTimer as u8].set_handler_addr(VirtAddr::new(
                crate::task::IntHandlerLAPICTimer as usize as u64,
            ));
        }
        idt.load();
    });
}
