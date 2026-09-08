#![allow(non_snake_case, non_upper_case_globals, dead_code, unsafe_op_in_unsafe_fn)]

use core::arch::asm;

pub const kConfigAddress: u16 = 0x0cf8;
pub const kConfigData: u16 = 0x0cfc;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ClassCode { pub base: u8, pub sub: u8, pub interface: u8 }
impl ClassCode {
    pub const fn Match(&self, base: u8, sub: u8, interface: u8) -> bool { self.base == base && self.sub == sub && self.interface == interface }
    pub const fn MatchBase(&self, base: u8) -> bool { self.base == base }
    pub const fn MatchSub(&self, base: u8, sub: u8) -> bool { self.MatchBase(base) && self.sub == sub }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Device { pub bus: u8, pub device: u8, pub function: u8, pub header_type: u8, pub class_code: ClassCode }

pub static mut devices: [Device; 32] = [Device { bus: 0, device: 0, function: 0, header_type: 0, class_code: ClassCode { base: 0, sub: 0, interface: 0 } }; 32];
pub static mut num_device: usize = 0;
const DEVICE_CAPACITY: usize = 32;

const fn MakeAddress(bus: u8, device: u8, function: u8, reg_addr: u8) -> u32 { (1 << 31) | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | (reg_addr as u32 & 0xfc) }

pub unsafe fn WriteAddress(address: u32) { asm!("out dx, eax", in("dx") kConfigAddress, in("eax") address, options(nostack, preserves_flags)); }
pub unsafe fn WriteData(value: u32) { asm!("out dx, eax", in("dx") kConfigData, in("eax") value, options(nostack, preserves_flags)); }
pub unsafe fn ReadData() -> u32 { let value: u32; asm!("in eax, dx", in("dx") kConfigData, out("eax") value, options(nostack, preserves_flags)); value }

pub unsafe fn ReadVendorId(bus: u8, device: u8, function: u8) -> u16 { WriteAddress(MakeAddress(bus, device, function, 0)); ReadData() as u16 }
pub unsafe fn ReadDeviceId(bus: u8, device: u8, function: u8) -> u16 { WriteAddress(MakeAddress(bus, device, function, 0)); (ReadData() >> 16) as u16 }
pub unsafe fn ReadHeaderType(bus: u8, device: u8, function: u8) -> u8 { WriteAddress(MakeAddress(bus, device, function, 0x0c)); (ReadData() >> 16) as u8 }
pub unsafe fn ReadClassCode(bus: u8, device: u8, function: u8) -> ClassCode { WriteAddress(MakeAddress(bus, device, function, 8)); let value = ReadData(); ClassCode { base: (value >> 24) as u8, sub: (value >> 16) as u8, interface: (value >> 8) as u8 } }
pub const fn IsSingleFunctionDevice(header_type: u8) -> bool { header_type & 0x80 == 0 }

pub unsafe fn ReadConfReg(device: &Device, reg_addr: u8) -> u32 { WriteAddress(MakeAddress(device.bus, device.device, device.function, reg_addr)); ReadData() }
pub unsafe fn WriteConfReg(device: &Device, reg_addr: u8, value: u32) { WriteAddress(MakeAddress(device.bus, device.device, device.function, reg_addr)); WriteData(value); }
pub unsafe fn ReadBusNumbers(bus: u8, device: u8, function: u8) -> u32 { WriteAddress(MakeAddress(bus, device, function, 0x18)); ReadData() }

pub unsafe fn ScanAllBus() -> bool {
    num_device = 0;
    for device in 0..32u8 {
        if ReadVendorId(0, device, 0) == 0xffff { continue; }
        let header_type = ReadHeaderType(0, device, 0);
        let class_code = ReadClassCode(0, device, 0);
        if num_device < DEVICE_CAPACITY { devices[num_device] = Device { bus: 0, device, function: 0, header_type, class_code }; num_device += 1; }
        if !IsSingleFunctionDevice(header_type) {
            for function in 1..8u8 {
                if ReadVendorId(0, device, function) == 0xffff { continue; }
                if num_device < DEVICE_CAPACITY { devices[num_device] = Device { bus: 0, device, function, header_type: ReadHeaderType(0, device, function), class_code: ReadClassCode(0, device, function) }; num_device += 1; }
            }
        }
    }
    true
}

pub unsafe fn ReadBar(device: &Device, bar_index: u8) -> Option<u64> {
    if bar_index >= 6 { return None; }
    let address = 0x10 + 4 * bar_index;
    let lower = ReadConfReg(device, address);
    if lower & 4 == 0 { return Some((lower & !0xf) as u64); }
    if bar_index >= 5 { return None; }
    Some(((ReadConfReg(device, address + 4) as u64) << 32) | (lower & !0xf) as u64)
}

pub unsafe fn ConfigureMSIFixedDestination(device: &Device, apic_id: u8, vector: u8, num_vector_exponent: u8) -> bool {
    let mut address = (ReadConfReg(device, 0x34) & 0xff) as u8;
    while address != 0 {
        let header = ReadConfReg(device, address);
        if (header & 0xff) == 0x05 {
            let mut control = header;
            let capable = (control >> 17) & 7;
            control |= 1 << 16;
            control = (control & !(7 << 20)) | ((u32::from(num_vector_exponent).min(capable)) << 20);
            WriteConfReg(device, address, control);
            WriteConfReg(device, address + 4, 0xfee0_0000 | ((apic_id as u32) << 12));
            let data_offset = if control & (1 << 23) != 0 { 12 } else { 8 };
            WriteConfReg(device, address + data_offset, vector as u32);
            return true;
        }
        address = ((header >> 8) & 0xff) as u8;
    }
    false
}