use core::ptr::addr_of_mut;

use x86_64::{
    registers::segmentation::{CS, DS, ES, FS, GS, SS, Segment},
    structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
};

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();

pub fn init_segment() {
    let gdt = unsafe { &mut *addr_of_mut!(GDT) };

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
}
