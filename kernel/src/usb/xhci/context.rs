#![allow(non_snake_case, dead_code)]

use super::trb::TRB;
use crate::usb::endpoint::EndpointID;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SlotContext { pub dwords: [u32; 8] }

impl SlotContext {
    pub fn set_route_string(&mut self, value: u32) { self.dwords[0] = (self.dwords[0] & !0x000f_ffff) | (value & 0x000f_ffff); }
    pub fn set_speed(&mut self, value: u32) { self.dwords[0] = (self.dwords[0] & !(0xf << 20)) | ((value & 0xf) << 20); }
    pub fn set_context_entries(&mut self, value: u32) { self.dwords[0] = (self.dwords[0] & !(0x1f << 27)) | ((value & 0x1f) << 27); }
    pub fn set_root_hub_port_num(&mut self, value: u32) { self.dwords[1] = (self.dwords[1] & !(0xff << 16)) | ((value & 0xff) << 16); }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct EndpointContext { pub dwords: [u32; 8] }

impl EndpointContext {
    pub fn TransferRingBuffer(&self) -> *mut TRB {
        (((self.dwords[3] as u64) << 32 | self.dwords[2] as u64) & !0xf) as *mut TRB
    }
    pub fn SetTransferRingBuffer(&mut self, buffer: *const TRB) {
        let value = buffer as u64;
        self.dwords[2] = (self.dwords[2] & 0xf) | (value as u32 & !0xf);
        self.dwords[3] = (value >> 32) as u32;
    }
    pub fn set_endpoint_type(&mut self, value: u32) { self.dwords[1] = (self.dwords[1] & !(0x7 << 3)) | ((value & 0x7) << 3); }
    pub fn set_max_packet_size(&mut self, value: u32) { self.dwords[1] = (self.dwords[1] & !(0xffff << 16)) | ((value & 0xffff) << 16); }
    pub fn set_interval(&mut self, value: u32) { self.dwords[0] = (self.dwords[0] & !(0xff << 16)) | ((value & 0xff) << 16); }
    pub fn set_error_count(&mut self, value: u32) { self.dwords[1] = (self.dwords[1] & !(0x3 << 1)) | ((value & 0x3) << 1); }
    pub fn set_average_trb_length(&mut self, value: u32) { self.dwords[4] = (self.dwords[4] & !0xffff) | (value & 0xffff); }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DeviceContextIndex { pub value: i32 }
impl DeviceContextIndex {
    pub const fn new(dci: i32) -> Self { Self { value: dci } }
    pub const fn from_endpoint(ep_id: EndpointID) -> Self { Self::new(ep_id.Address()) }
    pub const fn from_parts(ep_num: i32, dir_in: bool) -> Self { Self::new(2 * ep_num + if ep_num == 0 { 1 } else { dir_in as i32 }) }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct DeviceContext { pub slot_context: SlotContext, pub ep_contexts: [EndpointContext; 31] }

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct InputControlContext {
    pub drop_context_flags: u32, pub add_context_flags: u32, pub reserved1: [u32; 5],
    pub configuration_value: u8, pub interface_number: u8, pub alternate_setting: u8, pub reserved2: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct InputContext {
    pub input_control_context: InputControlContext,
    pub slot_context: SlotContext,
    pub ep_contexts: [EndpointContext; 31],
}

impl InputContext {
    pub fn EnableSlotContext(&mut self) -> *mut SlotContext {
        self.input_control_context.add_context_flags |= 1;
        core::ptr::addr_of_mut!(self.slot_context)
    }
    pub fn EnableEndpoint(&mut self, dci: DeviceContextIndex) -> *mut EndpointContext {
        self.input_control_context.add_context_flags |= 1u32 << dci.value;
        core::ptr::addr_of_mut!(self.ep_contexts[(dci.value - 1) as usize])
    }
}