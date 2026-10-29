use crate::{
    global::Global,
    graphics::{SCREEN_CONFIG, ScreenWriter},
};
use core::ptr::write_volatile;

use crate::message::Message;
use x86_64::{
    VirtAddr,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
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

fn with_screen_writer(f: impl FnOnce(&ScreenWriter)) {
    SCREEN_CONFIG.with(|sc| {
        if let Some(config) = sc {
            let writer = ScreenWriter::new(*config);
            f(&writer);
        }
    });
}
fn print_hex(writer: &crate::graphics::ScreenWriter, value: u64, width: usize, x: usize, y: usize) {
    let color = crate::graphics::PixelColor { r: 0, g: 0, b: 0 };
    for i in 0..width {
        let digit = ((value >> (4 * (width - i - 1))) & 0xf) as u8;
        let c = if digit >= 10 {
            b'a' + (digit - 10)
        } else {
            b'0' + digit
        };
        crate::font::write_ascii(writer, x + 8 * i, y, c, &color);
    }
}
fn print_frame(
    writer: &crate::graphics::ScreenWriter,
    frame: &InterruptStackFrame,
    exp_name: &str,
) {
    let color = crate::graphics::PixelColor { r: 0, g: 0, b: 0 };
    crate::font::write_string(writer, 500, 16 * 0, exp_name, &color);
    crate::font::write_string(writer, 500, 16 * 1, "CS:RIP", &color);
    print_hex(writer, frame.code_segment.0 as u64, 4, 500 + 8 * 7, 16 * 1);
    print_hex(
        writer,
        frame.instruction_pointer.as_u64(),
        16,
        500 + 8 * 12,
        16 * 1,
    );
    crate::font::write_string(writer, 500, 16 * 2, "RFLAGS", &color);
    print_hex(writer, frame.cpu_flags.bits(), 16, 500 + 8 * 7, 16 * 2);
    crate::font::write_string(writer, 500, 16 * 3, "SS:RSP", &color);
    print_hex(writer, frame.stack_segment.0 as u64, 4, 500 + 8 * 7, 16 * 3);
    print_hex(
        writer,
        frame.stack_pointer.as_u64(),
        16,
        500 + 8 * 12,
        16 * 3,
    );
}

macro_rules! fault_handler_no_error {
    ($fn_name:ident,$exp_name:expr) => {
        extern "x86-interrupt" fn $fn_name(frame: InterruptStackFrame) {
            with_screen_writer(|writer| {
                print_frame(writer, &frame, $exp_name);
            });
            loop {
                x86_64::instructions::hlt();
            }
        }
    };
}
macro_rules! fault_handler_with_error {
    ($fn_name:ident, $exp_name:expr) => {
        extern "x86-interrupt" fn $fn_name(frame: InterruptStackFrame, error_code: u64) {
            with_screen_writer(|writer| {
                print_frame(writer, &frame, $exp_name);
                let color = crate::graphics::PixelColor { r: 0, g: 0, b: 0 };
                crate::font::write_string(writer, 500, 16 * 4, "ERR", &color);
                print_hex(writer, error_code, 16, 500 + 8 * 4, 16 * 4);
            });
            loop {
                x86_64::instructions::hlt();
            }
        }
    };
}
fault_handler_no_error!(int_handler_de, "#DE");
fault_handler_no_error!(int_handler_db, "#DB");
fault_handler_no_error!(int_handler_bp, "#BP");
fault_handler_no_error!(int_handler_of, "#OF");
fault_handler_no_error!(int_handler_br, "#BR");
fault_handler_no_error!(int_handler_ud, "#UD");
fault_handler_no_error!(int_handler_nm, "#NM");
fault_handler_no_error!(int_handler_mf, "#MF");
fault_handler_no_error!(int_handler_xm, "#XM");
fault_handler_no_error!(int_handler_ve, "#VE");
fault_handler_with_error!(int_handler_ts, "#TS");
fault_handler_with_error!(int_handler_np, "#NP");
fault_handler_with_error!(int_handler_ss, "#SS");
fault_handler_with_error!(int_handler_gp, "#GP");
fault_handler_with_error!(int_handler_ac, "#AC");
extern "x86-interrupt" fn int_handler_df(frame: InterruptStackFrame, error_code: u64) -> ! {
    with_screen_writer(|writer| {
        print_frame(writer, &frame, "#DF");
        let color = crate::graphics::PixelColor { r: 0, g: 0, b: 0 };
        crate::font::write_string(writer, 500, 16 * 4, "ERR", &color);
        print_hex(writer, error_code, 16, 500 + 8 * 4, 16 * 4);
    });
    loop {
        x86_64::instructions::hlt();
    }
}
extern "x86-interrupt" fn int_handler_mc(frame: InterruptStackFrame) -> ! {
    with_screen_writer(|writer| {
        print_frame(writer, &frame, "#MC");
    });
    loop {
        x86_64::instructions::hlt();
    }
}
// Page Fault ハンドラ (PageFaultErrorCode 型)
extern "x86-interrupt" fn int_handler_pf(
    frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    with_screen_writer(|writer| {
        print_frame(writer, &frame, "#PF");
        let color = crate::graphics::PixelColor { r: 0, g: 0, b: 0 };
        crate::font::write_string(writer, 500, 16 * 4, "ERR", &color);
        print_hex(writer, error_code.bits(), 16, 500 + 8 * 4, 16 * 4);
    });
    loop {
        x86_64::instructions::hlt();
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
                crate::task::IntHandlerLAPICTimer as *const () as usize as u64,
            ));
        }

        idt.divide_error.set_handler_fn(int_handler_de);
        idt.debug.set_handler_fn(int_handler_db);
        idt.breakpoint.set_handler_fn(int_handler_bp);
        idt.overflow.set_handler_fn(int_handler_of);
        idt.bound_range_exceeded.set_handler_fn(int_handler_br);
        idt.invalid_opcode.set_handler_fn(int_handler_ud);
        idt.device_not_available.set_handler_fn(int_handler_nm);
        idt.double_fault.set_handler_fn(int_handler_df);
        idt.invalid_tss.set_handler_fn(int_handler_ts);
        idt.segment_not_present.set_handler_fn(int_handler_np);
        idt.stack_segment_fault.set_handler_fn(int_handler_ss);
        idt.general_protection_fault.set_handler_fn(int_handler_gp);
        idt.page_fault.set_handler_fn(int_handler_pf);
        idt.x87_floating_point.set_handler_fn(int_handler_mf);
        idt.alignment_check.set_handler_fn(int_handler_ac);
        idt.machine_check.set_handler_fn(int_handler_mc);
        idt.simd_floating_point.set_handler_fn(int_handler_xm);
        idt.virtualization.set_handler_fn(int_handler_ve);

        idt.load();
    });
}
