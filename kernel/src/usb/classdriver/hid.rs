#![allow(non_upper_case_globals)]

use crate::usb::{endpoint::{EndpointConfig, EndpointID, EndpointType}, error::{ErrorCode, Result}, setupdata::{request, request_type, SetupData}};

pub trait HIDDataHandler { fn OnDataReceived(&mut self) -> Result; }

pub struct HIDBaseDriver {
    pub ep_interrupt_in_: EndpointID,
    pub ep_interrupt_out_: EndpointID,
    pub interface_index_: i32,
    pub in_packet_size_: usize,
    pub initialize_phase_: u8,
    pub buf_: [u8; 1024],
    pub previous_buf_: [u8; 1024],
}

impl HIDBaseDriver {
    pub const kBufferSize: usize = 1024;
    pub fn new(interface_index: i32, in_packet_size: usize) -> Self { Self { ep_interrupt_in_: EndpointID::default(), ep_interrupt_out_: EndpointID::default(), interface_index_: interface_index, in_packet_size_: in_packet_size, initialize_phase_: 0, buf_: [0; 1024], previous_buf_: [0; 1024] } }
    pub fn SetEndpoint(&mut self, config: &EndpointConfig) -> Result {
        if config.ep_type == EndpointType::kInterrupt && config.ep_id.IsIn() { self.ep_interrupt_in_ = config.ep_id; }
        else if config.ep_type == EndpointType::kInterrupt { self.ep_interrupt_out_ = config.ep_id; }
        Ok(())
    }
    pub fn set_protocol_setup(&mut self) -> SetupData {
        self.initialize_phase_ = 1;
        SetupData::new(((request_type::kClass << 5) | request_type::kInterface) as u8, request::kSetProtocol as u8, 0, self.interface_index_ as u16, 0)
    }
    pub fn OnControlCompleted(&mut self, _ep_id: EndpointID, _setup_data: SetupData, _buf: &[u8]) -> Result {
        if self.initialize_phase_ == 1 { self.initialize_phase_ = 2; return Ok(()); }
        Err(ErrorCode::kNotImplemented.into())
    }
    pub fn OnInterruptCompleted<H: HIDDataHandler>(&mut self, handler: &mut H, ep_id: EndpointID, buf: &[u8]) -> Result {
        if !ep_id.IsIn() { return Err(ErrorCode::kNotImplemented.into()); }
        let length = buf.len().min(self.buf_.len());
        self.buf_[..length].copy_from_slice(&buf[..length]);
        handler.OnDataReceived()?;
        self.previous_buf_[..length].copy_from_slice(&self.buf_[..length]);
        Ok(())
    }
}