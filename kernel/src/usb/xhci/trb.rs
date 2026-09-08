#![allow(non_snake_case, non_upper_case_globals, dead_code, unsafe_op_in_unsafe_fn)]

use core::mem::size_of;
use crate::usb::endpoint::EndpointID;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TRB {
    pub data: [u32; 4],
}

impl TRB {
    pub const fn new() -> Self { Self { data: [0; 4] } }
    pub const fn parameter(self) -> u64 { (self.data[1] as u64) << 32 | self.data[0] as u64 }
    pub fn set_parameter(&mut self, value: u64) {
        self.data[0] = value as u32;
        self.data[1] = (value >> 32) as u32;
    }
    pub const fn status(self) -> u32 { self.data[2] }
    pub fn set_status(&mut self, value: u32) { self.data[2] = value; }
    pub const fn cycle_bit(self) -> bool { self.data[3] & 1 != 0 }
    pub fn set_cycle_bit(&mut self, value: bool) { self.data[3] = (self.data[3] & !1) | value as u32; }
    pub const fn trb_type(self) -> u8 { ((self.data[3] >> 10) & 0x3f) as u8 }
    pub fn set_trb_type(&mut self, value: u8) { self.data[3] = (self.data[3] & !(0x3f << 10)) | ((value as u32 & 0x3f) << 10); }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NormalTRB { pub data: [u32; 4] }
impl NormalTRB {
    pub const Type: u32 = 1;
    pub fn new() -> Self { let mut value = Self { data: [0; 4] }; value.data[3] = Self::Type << 10; value }
    pub fn SetPointer(&mut self, pointer: *const u8) { self.data[0] = pointer as u64 as u32; self.data[1] = (pointer as u64 >> 32) as u32; }
    pub fn Pointer(&self) -> *mut TRB { ((self.data[1] as u64) << 32 | self.data[0] as u64) as *mut TRB }
    pub fn set_transfer_length(&mut self, value: u32) { self.data[2] = (self.data[2] & !0x1ffff) | (value & 0x1ffff); }
    pub fn set_chain_bit(&mut self, value: bool) { self.data[3] = (self.data[3] & !(1 << 4)) | ((value as u32) << 4); }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SetupStageTRB { pub data: [u32; 4] }
impl SetupStageTRB {
    pub const Type: u32 = 2;
    pub const kNoDataStage: u32 = 0;
    pub const kOutDataStage: u32 = 2;
    pub const kInDataStage: u32 = 3;
    pub fn new() -> Self { let mut value = Self { data: [0; 4] }; value.data[2] = 8; value.data[3] = (Self::Type << 10) | (1 << 6); value }
    pub fn set_setup_data(&mut self, setup: &crate::usb::setupdata::SetupData) { self.data[0] = setup.request_type as u32 | ((setup.request as u32) << 8) | ((setup.value as u32) << 16); self.data[1] = setup.index as u32 | ((setup.length as u32) << 16); }
    pub fn set_transfer_type(&mut self, value: u32) { self.data[3] = (self.data[3] & !(3 << 16)) | ((value & 3) << 16); }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DataStageTRB { pub data: [u32; 4] }
impl DataStageTRB {
    pub const Type: u32 = 3;
    pub fn new() -> Self { let mut value = Self { data: [0; 4] }; value.data[3] = Self::Type << 10; value }
    pub fn SetPointer(&mut self, pointer: *const u8) { self.data[0] = pointer as u64 as u32; self.data[1] = (pointer as u64 >> 32) as u32; }
    pub fn set_transfer_length(&mut self, value: u32) { self.data[2] = (self.data[2] & !0x1ffff) | (value & 0x1ffff); }
    pub fn set_direction(&mut self, value: bool) { self.data[3] = (self.data[3] & !(1 << 16)) | ((value as u32) << 16); }
    pub fn set_interrupt_on_completion(&mut self) { self.data[3] |= 1 << 5; }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct StatusStageTRB { pub data: [u32; 4] }
impl StatusStageTRB { pub const Type: u32 = 4; pub fn new() -> Self { let mut value = Self { data: [0; 4] }; value.data[3] = Self::Type << 10; value } pub fn set_direction(&mut self, value: bool) { self.data[3] = (self.data[3] & !(1 << 16)) | ((value as u32) << 16); } pub fn set_interrupt_on_completion(&mut self) { self.data[3] |= 1 << 5; } }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct EnableSlotCommandTRB { pub data: [u32; 4] }
impl EnableSlotCommandTRB { pub const Type: u32 = 9; pub fn new() -> Self { let mut value = Self { data: [0; 4] }; value.data[3] = Self::Type << 10; value } }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AddressDeviceCommandTRB { pub data: [u32; 4] }
impl AddressDeviceCommandTRB {
    pub const Type: u32 = 11;
    pub fn new(input_context: *const crate::usb::xhci::context::InputContext, slot_id: u8) -> Self { let value = input_context as u64; Self { data: [value as u32, (value >> 32) as u32, 0, (Self::Type << 10) | ((slot_id as u32) << 24)] } }
    pub fn Pointer(&self) -> *mut crate::usb::xhci::context::InputContext { ((self.data[1] as u64) << 32 | self.data[0] as u64) as *mut crate::usb::xhci::context::InputContext }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ConfigureEndpointCommandTRB { pub data: [u32; 4] }
impl ConfigureEndpointCommandTRB {
    pub const Type: u32 = 12;
    pub fn new(input_context: *const crate::usb::xhci::context::InputContext, slot_id: u8) -> Self { let value = input_context as u64; Self { data: [value as u32, (value >> 32) as u32, 0, (Self::Type << 10) | ((slot_id as u32) << 24)] } }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TransferEventTRB { pub data: [u32; 4] }
impl TransferEventTRB {
    pub const Type: u32 = 32;
    pub fn new() -> Self { let mut value = Self { data: [0; 4] }; value.data[3] = Self::Type << 10; value }
    pub fn Pointer(&self) -> *mut TRB { ((self.data[1] as u64) << 32 | self.data[0] as u64) as *mut TRB }
    pub fn SetPointer(&mut self, pointer: *const TRB) { let value = pointer as u64; self.data[0] = value as u32; self.data[1] = (value >> 32) as u32; }
    pub fn EndpointID(&self) -> EndpointID { EndpointID::new(((self.data[3] >> 16) & 0x1f) as i32) }
    pub const fn completion_code(&self) -> u8 { (self.data[2] >> 24) as u8 }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CommandCompletionEventTRB { pub data: [u32; 4] }
impl CommandCompletionEventTRB { pub const Type: u32 = 33; pub fn new() -> Self { let mut value = Self { data: [0; 4] }; value.data[3] = Self::Type << 10; value } pub fn Pointer(&self) -> *mut TRB { (((self.data[1] as u64) << 32 | self.data[0] as u64) & !0xf) as *mut TRB } pub const fn slot_id(&self) -> u8 { (self.data[3] >> 24) as u8 } }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PortStatusChangeEventTRB { pub data: [u32; 4] }
impl PortStatusChangeEventTRB { pub const Type: u32 = 34; pub fn new() -> Self { let mut value = Self { data: [0; 4] }; value.data[3] = Self::Type << 10; value } pub const fn port_id(&self) -> u8 { (self.data[0] >> 24) as u8 } }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LinkTRB { pub data: [u32; 4] }
impl LinkTRB {
    pub const Type: u32 = 6;
    pub fn new(pointer: *const TRB) -> Self { let mut value = Self { data: [0; 4] }; value.data[0] = pointer as u64 as u32; value.data[1] = (pointer as u64 >> 32) as u32; value.data[3] = Self::Type << 10; value }
    pub fn set_toggle_cycle(&mut self, value: bool) { self.data[3] = (self.data[3] & !(1 << 1)) | ((value as u32) << 1); }
}

pub fn trb_data<T>(trb: &T) -> [u32; 4] where T: AsTrbData { trb.as_trb_data() }
pub trait AsTrbData { fn as_trb_data(&self) -> [u32; 4]; }
impl<T: Copy> AsTrbData for T where T: Into<[u32; 4]> { fn as_trb_data(&self) -> [u32; 4] { (*self).into() } }
impl From<NormalTRB> for [u32; 4] { fn from(value: NormalTRB) -> Self { value.data } }
impl From<SetupStageTRB> for [u32; 4] { fn from(value: SetupStageTRB) -> Self { value.data } }
impl From<DataStageTRB> for [u32; 4] { fn from(value: DataStageTRB) -> Self { value.data } }
impl From<StatusStageTRB> for [u32; 4] { fn from(value: StatusStageTRB) -> Self { value.data } }
impl From<LinkTRB> for [u32; 4] { fn from(value: LinkTRB) -> Self { value.data } }
impl From<TransferEventTRB> for [u32; 4] { fn from(value: TransferEventTRB) -> Self { value.data } }
impl From<CommandCompletionEventTRB> for [u32; 4] { fn from(value: CommandCompletionEventTRB) -> Self { value.data } }
impl From<PortStatusChangeEventTRB> for [u32; 4] { fn from(value: PortStatusChangeEventTRB) -> Self { value.data } }
impl From<EnableSlotCommandTRB> for [u32; 4] { fn from(value: EnableSlotCommandTRB) -> Self { value.data } }
impl From<AddressDeviceCommandTRB> for [u32; 4] { fn from(value: AddressDeviceCommandTRB) -> Self { value.data } }
impl From<ConfigureEndpointCommandTRB> for [u32; 4] { fn from(value: ConfigureEndpointCommandTRB) -> Self { value.data } }

pub trait TRBType { const Type: u32; }
impl TRBType for NormalTRB { const Type: u32 = NormalTRB::Type; }
impl TRBType for AddressDeviceCommandTRB { const Type: u32 = AddressDeviceCommandTRB::Type; }
impl TRBType for ConfigureEndpointCommandTRB { const Type: u32 = ConfigureEndpointCommandTRB::Type; }
impl TRBType for SetupStageTRB { const Type: u32 = SetupStageTRB::Type; }
impl TRBType for DataStageTRB { const Type: u32 = DataStageTRB::Type; }
impl TRBType for StatusStageTRB { const Type: u32 = StatusStageTRB::Type; }
impl TRBType for LinkTRB { const Type: u32 = LinkTRB::Type; }
impl TRBType for TransferEventTRB { const Type: u32 = TransferEventTRB::Type; }
impl TRBType for CommandCompletionEventTRB { const Type: u32 = CommandCompletionEventTRB::Type; }
impl TRBType for PortStatusChangeEventTRB { const Type: u32 = PortStatusChangeEventTRB::Type; }

pub unsafe fn TRBDynamicCast<T: TRBType>(trb: *const TRB) -> Option<&'static T> {
    if trb.is_null() || (*trb).trb_type() as u32 != T::Type { return None; }
    Some(&*trb.cast::<T>())
}

const _: () = assert!(size_of::<TRB>() == 16);