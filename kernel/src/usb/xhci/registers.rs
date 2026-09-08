#![allow(non_snake_case, non_camel_case_types, dead_code, unsafe_op_in_unsafe_fn)]

use core::ptr::{read_volatile, write_volatile};

#[repr(transparent)]
pub struct MemMapRegister<T> { value: T }
impl<T: Copy> MemMapRegister<T> {
    pub unsafe fn Read(&self) -> T { read_volatile(core::ptr::addr_of!(self.value)) }
    pub unsafe fn Write(&mut self, value: T) { write_volatile(core::ptr::addr_of_mut!(self.value), value); }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct HCSPARAMS1_Bitmap { pub data: [u32; 1] }
impl HCSPARAMS1_Bitmap { pub const fn max_device_slots(self) -> u8 { self.data[0] as u8 } pub const fn max_ports(self) -> u8 { (self.data[0] >> 24) as u8 } }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct HCCPARAMS1_Bitmap { pub data: [u32; 1] }
impl HCCPARAMS1_Bitmap { pub const fn context_size(self) -> bool { self.data[0] & (1 << 2) != 0 } pub const fn extended_capabilities_pointer(self) -> u16 { (self.data[0] >> 16) as u16 } }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct DBOFF_Bitmap { pub data: [u32; 1] }
impl DBOFF_Bitmap { pub const fn Offset(self) -> u32 { self.data[0] & !3 } }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct RTSOFF_Bitmap { pub data: [u32; 1] }
impl RTSOFF_Bitmap { pub const fn Offset(self) -> u32 { self.data[0] & !0x1f } }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct USBCMD_Bitmap { pub data: [u32; 1] }
impl USBCMD_Bitmap { pub const fn run_stop(self) -> bool { self.data[0] & 1 != 0 } pub fn set_run_stop(&mut self, value: bool) { self.data[0] = (self.data[0] & !1) | value as u32; } pub fn set_reset(&mut self, value: bool) { self.data[0] = (self.data[0] & !(1 << 1)) | ((value as u32) << 1); } }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct USBSTS_Bitmap { pub data: [u32; 1] }
impl USBSTS_Bitmap { pub const fn event_interrupt(self) -> bool { self.data[0] & (1 << 3) != 0 } }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct CRCR_Bitmap { pub data: [u64; 1] }
impl CRCR_Bitmap {
    pub const fn Pointer(self) -> u64 { self.data[0] & !0x3f }
    pub fn set_ring_cycle_state(&mut self, value: bool) { self.data[0] = (self.data[0] & !1) | value as u64; }
    pub fn SetPointer(&mut self, value: u64) { self.data[0] = (self.data[0] & 0x3f) | (value & !0x3f); }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct DCBAAP_Bitmap { pub data: [u64; 1] }
impl DCBAAP_Bitmap { pub const fn Pointer(self) -> u64 { self.data[0] & !0x3f } pub fn SetPointer(&mut self, value: u64) { self.data[0] = (self.data[0] & 0x3f) | (value & !0x3f); }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct CONFIG_Bitmap { pub data: [u32; 1] }
impl CONFIG_Bitmap { pub fn SetMaxDeviceSlotsEnabled(&mut self, value: u8) { self.data[0] = (self.data[0] & !0xff) | value as u32; } }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct IMAN_Bitmap { pub data: [u32; 1] }
impl IMAN_Bitmap { pub fn set_interrupt_pending(&mut self) { self.data[0] |= 1; } pub fn set_interrupt_enable(&mut self, value: bool) { self.data[0] = (self.data[0] & !(1 << 1)) | ((value as u32) << 1); } }
#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct ERSTSZ_Bitmap { pub data: [u32; 1] }
impl ERSTSZ_Bitmap { pub fn SetSize(&mut self, value: u16) { self.data[0] = (self.data[0] & !0xffff) | value as u32; } }
#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct ERSTBA_Bitmap { pub data: [u64; 1] }
impl ERSTBA_Bitmap { pub fn SetPointer(&mut self, value: u64) { self.data[0] = (self.data[0] & 0x3f) | (value & !0x3f); } }
#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct ERDP_Bitmap { pub data: [u64; 1] }
impl ERDP_Bitmap { pub fn SetPointer(&mut self, value: u64) { self.data[0] = (self.data[0] & 0xf) | (value & !0xf); } }
#[repr(C, packed)]
pub struct InterrupterRegisterSet { pub IMAN: MemMapRegister<IMAN_Bitmap>, pub IMOD: MemMapRegister<[u32; 1]>, pub ERSTSZ: MemMapRegister<ERSTSZ_Bitmap>, pub reserved: u32, pub ERSTBA: MemMapRegister<ERSTBA_Bitmap>, pub ERDP: MemMapRegister<ERDP_Bitmap> }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct PORTSC_Bitmap { pub data: [u32; 1] }
impl PORTSC_Bitmap { pub const fn connected(self) -> bool { self.data[0] & 1 != 0 } pub const fn enabled(self) -> bool { self.data[0] & 2 != 0 } pub const fn reset(self) -> bool { self.data[0] & (1 << 4) != 0 } pub const fn speed(self) -> u8 { ((self.data[0] >> 10) & 0xf) as u8 } pub fn request_reset(&mut self) { self.data[0] &= 0x0e00_c3e0; self.data[0] |= 0x0002_0010; } }
#[repr(C, packed)]
pub struct PortRegisterSet { pub PORTSC: MemMapRegister<PORTSC_Bitmap>, pub PORTPMSC: MemMapRegister<[u32; 1]>, pub PORTLI: MemMapRegister<[u32; 1]>, pub PORTHLPMC: MemMapRegister<[u32; 1]> }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct Doorbell_Bitmap { pub data: [u32; 1] }
#[repr(transparent)]
pub struct DoorbellRegister { value: MemMapRegister<Doorbell_Bitmap> }
impl DoorbellRegister { pub unsafe fn Ring(&mut self, target: u8, stream_id: u16) { self.value.Write(Doorbell_Bitmap { data: [target as u32 | ((stream_id as u32) << 16)] }); } }

#[repr(C, packed)]
pub struct CapabilityRegisters {
    pub CAPLENGTH: MemMapRegister<u8>, pub reserved1: MemMapRegister<u8>, pub HCIVERSION: MemMapRegister<u16>,
    pub HCSPARAMS1: MemMapRegister<HCSPARAMS1_Bitmap>, pub HCSPARAMS2: MemMapRegister<[u32; 1]>, pub HCSPARAMS3: MemMapRegister<[u32; 1]>,
    pub HCCPARAMS1: MemMapRegister<HCCPARAMS1_Bitmap>, pub DBOFF: MemMapRegister<DBOFF_Bitmap>, pub RTSOFF: MemMapRegister<RTSOFF_Bitmap>, pub HCCPARAMS2: MemMapRegister<[u32; 1]>,
}

#[repr(C, packed)]
pub struct OperationalRegisters {
    pub USBCMD: MemMapRegister<USBCMD_Bitmap>, pub USBSTS: MemMapRegister<USBSTS_Bitmap>, pub PAGESIZE: MemMapRegister<[u32; 1]>, pub reserved1: [u32; 2], pub DNCTRL: MemMapRegister<[u32; 1]>,
    pub CRCR: MemMapRegister<CRCR_Bitmap>, pub reserved2: [u32; 4], pub DCBAAP: MemMapRegister<DCBAAP_Bitmap>, pub CONFIG: MemMapRegister<CONFIG_Bitmap>,
}