use crate::{
    global::Global,
    interrupts::IST_FOR_TIMER,
    memory_manager::{BYTES_PER_FRAME, MEMORY_MANAGER},
};
use x86_64::{
    VirtAddr,
    instructions::tables::load_tss,
    registers::segmentation::{CS, DS, ES, FS, GS, SS, Segment},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
};
/*
  15               3  2  1   0
  +----------------+-----+---+
  |     Index      | TI  |RPL|
  +----------------+-----+---+
*/
pub const KERNEL_CS: u64 = 1 << 3;
pub const KERNEL_SS: u64 = 2 << 3;
pub const USER_SS: u64 = (3 << 3) | 3;
pub const USER_CS: u64 = (4 << 3) | 3;
pub const TSS: u64 = 5 << 3;

static GDT: Global<GlobalDescriptorTable> = Global::new(GlobalDescriptorTable::new());
static TSS_INSTANCE: Global<TaskStateSegment> = Global::new(TaskStateSegment::new());

fn allocate_stack_area(num_4kframes: usize) -> VirtAddr {
    let frame = MEMORY_MANAGER
        .with(|mm| mm.allocate(num_4kframes))
        .expect("failed to allocate stack area");
    let stack_end = (frame.id * BYTES_PER_FRAME + num_4kframes * 4096) as u64;

    VirtAddr::new(stack_end)
}
pub fn init_segment() {
    GDT.init_static(|gdt| {
        // 0th data is added when GlobalDescriptorTable::new() runned and filled all-zero.

        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        // 0b00000000000000 + 00(PrivileeLevel::Ring0) = 0x0000 = nulldesc.
        let null_selector = SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0);
        gdt.load();
        unsafe {
            CS::set_reg(code_selector);
            DS::set_reg(null_selector);
            ES::set_reg(null_selector);
            FS::set_reg(null_selector);
            GS::set_reg(null_selector);
            SS::set_reg(data_selector);
        }
    });
}

const RSP0_FRAMES: usize = 8;
pub fn init_tss() {
    TSS_INSTANCE.init_static(|tss| {
        // privilege_stack_table[0] が Ring0用のスタックポインタ(RSP0)
        tss.privilege_stack_table[0] = allocate_stack_area(8);
        // タイマ割り込み用の独立したスタック(IST1)
        tss.interrupt_stack_table[IST_FOR_TIMER] = allocate_stack_area(8);
        GDT.init_static(|gdt| {
            // TSS 記述子は16 byte消費する。
            let tss_desc = Descriptor::tss_segment(tss);
            let tss_selector = gdt.append(tss_desc);
            gdt.load();
            unsafe { load_tss(tss_selector) }; // これで ltr命令を実行する
        })
    });
}
