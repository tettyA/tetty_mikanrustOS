#![allow(non_snake_case, dead_code, unsafe_op_in_unsafe_fn)]

use crate::usb::{device::TransferBackend, endpoint::{EndpointConfig, EndpointID, EndpointType}, error::Result, setupdata::SetupData};
use super::{context::{DeviceContext, DeviceContextIndex, InputContext}, registers::DoorbellRegister, ring::Ring, trb::{ConfigureEndpointCommandTRB, DataStageTRB, SetupStageTRB, StatusStageTRB, NormalTRB}};

#[derive(Clone, Copy)]
pub struct TransferRecord { pub issuer: *mut super::trb::TRB, pub setup: Option<SetupData>, pub buffer: *mut u8, pub length: usize }

pub struct Device {
    pub slot_id_: u8,
    pub dbreg_: *mut DoorbellRegister,
    pub transfer_rings_: [*mut Ring; 31],
    pub pending_setup_: Option<SetupData>,
    pub pending_buffer_: *mut u8,
    pub pending_length_: usize,
    pub input_context_: *mut InputContext,
    pub command_ring_: *mut Ring,
    pub command_dbreg_: *mut DoorbellRegister,
    pub device_context_: *mut DeviceContext,
    pub transfer_records_: [Option<TransferRecord>; 32],
}

impl Device {
    pub const fn new(slot_id: u8, dbreg: *mut DoorbellRegister, input_context: *mut InputContext, command_ring: *mut Ring, command_dbreg: *mut DoorbellRegister, device_context: *mut DeviceContext) -> Self { Self { slot_id_: slot_id, dbreg_: dbreg, transfer_rings_: [core::ptr::null_mut(); 31], pending_setup_: None, pending_buffer_: core::ptr::null_mut(), pending_length_: 0, input_context_: input_context, command_ring_: command_ring, command_dbreg_: command_dbreg, device_context_: device_context, transfer_records_: [const { None }; 32] } }
    pub fn complete_transfer(&mut self, issuer: *mut super::trb::TRB, residual: usize) -> Option<TransferRecord> { for record in &mut self.transfer_records_ { if record.is_some_and(|record| record.issuer == issuer) { let mut value = record.take().unwrap(); value.length = value.length.saturating_sub(residual); return Some(value); } } None }
    pub unsafe fn ReleaseTransferRings(&mut self) { for ring in &mut self.transfer_rings_ { if !ring.is_null() { core::ptr::drop_in_place(*ring); crate::usb::memory::FreeMem(*ring as *mut u8); *ring = core::ptr::null_mut(); } } }
    pub fn take_control_completion(&mut self, length: usize) -> Option<(SetupData, *mut u8, usize)> { self.pending_setup_.take().map(|setup| (setup, self.pending_buffer_, length.min(self.pending_length_))) }
    pub fn take_interrupt_buffer(&self, length: usize) -> (*mut u8, usize) { (self.pending_buffer_, length.min(self.pending_length_)) }
    pub unsafe fn SetTransferRing(&mut self, endpoint: EndpointID, ring: *mut Ring) -> Result { let index = endpoint.Address() - 1; if !(0..31).contains(&index) { return Err(crate::usb::error::ErrorCode::kInvalidEndpointNumber.into()); } self.transfer_rings_[index as usize] = ring; Ok(()) }
    unsafe fn ring_for(&mut self, endpoint: EndpointID) -> Result<&mut Ring> { let index = endpoint.Address() - 1; if !(0..31).contains(&index) { return Err(crate::usb::error::ErrorCode::kInvalidEndpointNumber.into()); } self.transfer_rings_[index as usize].as_mut().ok_or(crate::usb::error::ErrorCode::kTransferRingNotSet.into()) }
    unsafe fn ring_doorbell(&mut self, endpoint: EndpointID) { if !self.dbreg_.is_null() { (*self.dbreg_).Ring(endpoint.Address() as u8, 0); } }
}

impl Device {
    fn configure_endpoints_impl(&mut self, configs: &[EndpointConfig]) -> Result {
        unsafe {
            if self.input_context_.is_null() || self.command_ring_.is_null() { return Err(crate::usb::error::ErrorCode::kTransferRingNotSet.into()); }
            (*self.input_context_).input_control_context = core::default::Default::default();
            if !self.device_context_.is_null() { (*self.input_context_).slot_context = core::ptr::read_unaligned(core::ptr::addr_of!((*self.device_context_).slot_context)); }
            let slot_context = (*self.input_context_).EnableSlotContext();
            (*slot_context).set_context_entries(31);
            for config in configs {
                let dci = DeviceContextIndex::from_endpoint(config.ep_id);
                let ring = crate::usb::memory::AllocArray::<Ring>(1, 64, 4096);
                if ring.is_null() { return Err(crate::usb::error::ErrorCode::kNoEnoughMemory.into()); }
                ring.write(Ring::new()); if !(*ring).Initialize(32) { return Err(crate::usb::error::ErrorCode::kNoEnoughMemory.into()); }
                self.transfer_rings_[(dci.value - 1) as usize] = ring;
                let endpoint = (*self.input_context_).EnableEndpoint(dci);
                let endpoint_type = match config.ep_type { EndpointType::kControl => 4, EndpointType::kIsochronous => if config.ep_id.IsIn() { 5 } else { 1 }, EndpointType::kBulk => if config.ep_id.IsIn() { 6 } else { 2 }, EndpointType::kInterrupt => if config.ep_id.IsIn() { 7 } else { 3 } };
                let speed = if self.device_context_.is_null() { 3 } else { ((*self.device_context_).slot_context.dwords[0] >> 20) & 0xf };
                let interval = if speed == 1 || speed == 2 { if config.ep_type == EndpointType::kIsochronous { config.interval.saturating_add(2) as u32 } else { (usize::BITS - config.interval.max(1).leading_zeros() - 1 + 3) as u32 } } else { config.interval.saturating_sub(1) as u32 };
                (*endpoint).set_endpoint_type(endpoint_type); (*endpoint).set_max_packet_size(config.max_packet_size as u32); (*endpoint).set_interval(interval); (*endpoint).set_error_count(3); (*endpoint).set_average_trb_length(1); (*endpoint).SetTransferRingBuffer((*ring).Buffer()); (*endpoint).dwords[2] |= 1;
            }
            (*self.command_ring_).Push(ConfigureEndpointCommandTRB::new(self.input_context_, self.slot_id_).data);
            if !self.command_dbreg_.is_null() { (*self.command_dbreg_).Ring(0, 0); }
            Ok(())
        }
    }
}

impl TransferBackend for Device {
    fn ConfigureEndpoints(&mut self, configs: &[EndpointConfig]) -> Result { self.configure_endpoints_impl(configs) }
    fn ControlIn(&mut self, ep_id: EndpointID, setup_data: SetupData, buffer: &mut [u8]) -> Result {
        unsafe {
            self.pending_setup_ = Some(setup_data); self.pending_buffer_ = buffer.as_mut_ptr(); self.pending_length_ = buffer.len();
            let ring = self.ring_for(ep_id)?;
            let mut setup = SetupStageTRB::new(); setup.set_setup_data(&setup_data); setup.set_transfer_type(SetupStageTRB::kInDataStage);
            ring.Push(setup.data);
            if !buffer.is_empty() { let mut data = DataStageTRB::new(); data.SetPointer(buffer.as_ptr()); data.set_transfer_length(buffer.len() as u32); data.set_direction(true); ring.Push(data.data); }
            let mut status = StatusStageTRB::new(); status.set_direction(false); status.set_interrupt_on_completion(); let issuer = ring.Push(status.data).unwrap() as *const super::trb::TRB as *mut super::trb::TRB; self.remember_transfer(issuer, Some(setup_data), buffer.as_mut_ptr(), buffer.len())?; self.ring_doorbell(ep_id); Ok(())
        }
    }
    fn ControlOut(&mut self, ep_id: EndpointID, setup_data: SetupData, buffer: &[u8]) -> Result {
        unsafe {
            self.pending_setup_ = Some(setup_data); self.pending_buffer_ = buffer.as_ptr() as *mut u8; self.pending_length_ = buffer.len();
            let ring = self.ring_for(ep_id)?;
            let mut setup = SetupStageTRB::new(); setup.set_setup_data(&setup_data); setup.set_transfer_type(if buffer.is_empty() { SetupStageTRB::kNoDataStage } else { SetupStageTRB::kOutDataStage }); ring.Push(setup.data);
            if !buffer.is_empty() { let mut data = DataStageTRB::new(); data.SetPointer(buffer.as_ptr()); data.set_transfer_length(buffer.len() as u32); data.set_direction(false); ring.Push(data.data); }
            let mut status = StatusStageTRB::new(); status.set_direction(true); status.set_interrupt_on_completion(); let issuer = ring.Push(status.data).unwrap() as *const super::trb::TRB as *mut super::trb::TRB; self.remember_transfer(issuer, Some(setup_data), buffer.as_ptr() as *mut u8, buffer.len())?; self.ring_doorbell(ep_id); Ok(())
        }
    }
    fn InterruptIn(&mut self, ep_id: EndpointID, buffer: &mut [u8]) -> Result {
        unsafe { self.pending_buffer_ = buffer.as_mut_ptr(); self.pending_length_ = buffer.len(); crate::printk!("USB: interrupt submit ep={} buf={:p} len={}\n", ep_id.Address(), buffer.as_mut_ptr(), buffer.len()); let ring = self.ring_for(ep_id)?; let mut normal = NormalTRB::new(); normal.SetPointer(buffer.as_ptr()); normal.set_transfer_length(buffer.len() as u32); normal.data[3] |= (1 << 2) | (1 << 5); let issuer = ring.Push(normal.data).unwrap() as *const super::trb::TRB as *mut super::trb::TRB; self.remember_transfer(issuer, None, buffer.as_mut_ptr(), buffer.len())?; self.ring_doorbell(ep_id); Ok(()) }
    }
}

impl Device {
    fn remember_transfer(&mut self, issuer: *mut super::trb::TRB, setup: Option<SetupData>, buffer: *mut u8, length: usize) -> Result { let slot = self.transfer_records_.iter_mut().find(|slot| slot.is_none()).ok_or(crate::usb::error::ErrorCode::kNoEnoughMemory)?; *slot = Some(TransferRecord { issuer, setup, buffer, length }); Ok(()) }
}