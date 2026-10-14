use crate::global::Global;
use x86_64::{
    registers::segmentation::{CS, DS, ES, FS, GS, SS, Segment},
    structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
};

pub const KERNEL_CS: u64 = 1 << 3;
pub const KERNEL_SS: u64 = 2 << 3;

static GDT: Global<GlobalDescriptorTable> = Global::new(GlobalDescriptorTable::new());

pub fn init_segment() {
    GDT.init_static(|gdt| {
        // 0th data is added when GlobalDescriptorTable::new() runned and filled all-zero.

        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());

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
