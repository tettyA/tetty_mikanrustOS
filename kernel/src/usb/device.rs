#![allow(non_snake_case, dead_code)]

use crate::usb::{classdriver::{base::ClassDriver, keyboard::HIDKeyboardDriver, mouse::HIDMouseDriver}, endpoint::{EndpointConfig, EndpointID, EndpointType, kDefaultControlPipeID}, error::{ErrorCode, Result}, setupdata::{descriptor_type, request, request_type, SetupData}};

pub trait TransferBackend {
    fn ControlIn(&mut self, ep_id: EndpointID, setup_data: SetupData, buffer: &mut [u8]) -> Result;
    fn ControlOut(&mut self, ep_id: EndpointID, setup_data: SetupData, buffer: &[u8]) -> Result;
    fn InterruptIn(&mut self, ep_id: EndpointID, buffer: &mut [u8]) -> Result;
    fn ConfigureEndpoints(&mut self, configs: &[EndpointConfig]) -> Result { let _ = configs; Ok(()) }
}

pub struct Device<B: TransferBackend> {
    pub backend: B,
    pub class_drivers_: [Option<ClassDriverSlot>; 16],
    pub endpoint_driver_: [Option<usize>; 16],
    pub buf_: [u8; 256],
    pub ep_configs_: [EndpointConfig; 16],
    pub num_ep_configs_: usize,
    pub num_configurations_: u8,
    pub config_index_: u8,
    pub initialize_phase_: u8,
    pub is_initialized_: bool,
}

pub enum ClassDriverSlot { Keyboard(HIDKeyboardDriver), Mouse(HIDMouseDriver) }
impl ClassDriverSlot {
    fn SetEndpoint(&mut self, config: &EndpointConfig) -> Result { match self { Self::Keyboard(driver) => driver.SetEndpoint(config), Self::Mouse(driver) => driver.SetEndpoint(config) } }
    fn OnEndpointsConfigured(&mut self) -> Result { match self { Self::Keyboard(driver) => driver.OnEndpointsConfigured(), Self::Mouse(driver) => driver.OnEndpointsConfigured() } }
    fn OnControlCompleted(&mut self, ep_id: EndpointID, setup: SetupData, buffer: &[u8]) -> Result { match self { Self::Keyboard(driver) => driver.OnControlCompleted(ep_id, setup, buffer), Self::Mouse(driver) => driver.OnControlCompleted(ep_id, setup, buffer) } }
    fn protocol_setup(&mut self) -> crate::usb::setupdata::SetupData { match self { Self::Keyboard(driver) => driver.hid.set_protocol_setup(), Self::Mouse(driver) => driver.hid.set_protocol_setup() } }
    fn OnInterruptCompleted(&mut self, ep_id: EndpointID, buffer: &[u8]) -> Result { match self { Self::Keyboard(driver) => driver.OnInterruptCompleted(ep_id, buffer), Self::Mouse(driver) => driver.OnInterruptCompleted(ep_id, buffer) } }
    fn interrupt_endpoint(&self) -> EndpointID { match self { Self::Keyboard(driver) => driver.hid.ep_interrupt_in_, Self::Mouse(driver) => driver.hid.ep_interrupt_in_ } }
    fn interrupt_packet_size(&self) -> usize { match self { Self::Keyboard(driver) => driver.hid.in_packet_size_, Self::Mouse(driver) => driver.hid.in_packet_size_ } }
}

impl<B: TransferBackend> Device<B> {
    pub fn new(backend: B) -> Self { Self { backend, class_drivers_: [const { None }; 16], endpoint_driver_: [const { None }; 16], buf_: [0; 256], ep_configs_: [EndpointConfig { ep_id: kDefaultControlPipeID, ep_type: EndpointType::kControl, max_packet_size: 0, interval: 0 }; 16], num_ep_configs_: 0, num_configurations_: 0, config_index_: 0, initialize_phase_: 0, is_initialized_: false } }
    pub fn StartInitialize(&mut self) -> Result {
        crate::printk!("USB: device initialization phase 1\n");
        self.initialize_phase_ = 1;
        let setup = SetupData::new(((request_type::kIn << 7) | (request_type::kStandard << 5) | request_type::kDevice) as u8, request::kGetDescriptor as u8, (descriptor_type::kDevice as u16) << 8, 0, 256);
        self.backend.ControlIn(kDefaultControlPipeID, setup, &mut self.buf_)?;
        Ok(())
    }
    pub fn OnControlCompleted(&mut self, setup: SetupData, buffer: &[u8]) -> Result {
        if self.is_initialized_ && setup.request == request::kSetProtocol as u8 {
            for index in 0..self.class_drivers_.len() {
                let endpoint = if let Some(driver) = self.class_drivers_[index].as_mut() {
                    driver.OnControlCompleted(kDefaultControlPipeID, setup, buffer)?;
                    Some(driver.interrupt_endpoint())
                } else { None };
                if let Some(endpoint) = endpoint { let size = self.class_drivers_[index].as_ref().map(ClassDriverSlot::interrupt_packet_size).unwrap_or(3).min(self.buf_.len()); self.backend.InterruptIn(endpoint, &mut self.buf_[..size])?; }
            }
            return Ok(());
        }
        match self.initialize_phase_ {
            1 if setup.request == request::kGetDescriptor as u8 && buffer.len() >= 18 && buffer[1] == descriptor_type::kDevice as u8 => self.InitializePhase1(buffer),
            2 if setup.request == request::kGetDescriptor as u8 && buffer.len() >= 9 && buffer[1] == descriptor_type::kConfiguration as u8 => self.InitializePhase2(buffer),
            3 if setup.request == request::kSetConfiguration as u8 => { self.backend.ConfigureEndpoints(&self.ep_configs_[..self.num_ep_configs_])?; Ok(()) },
            _ => Err(ErrorCode::kInvalidPhase.into()),
        }
    }
    fn InitializePhase1(&mut self, buffer: &[u8]) -> Result { crate::printk!("USB: device descriptor received configs={}\n", buffer[17]); self.num_configurations_ = buffer[17]; self.config_index_ = 0; self.initialize_phase_ = 2; let setup = SetupData::new(0x80, request::kGetDescriptor as u8, (descriptor_type::kConfiguration as u16) << 8, 0, 256); self.backend.ControlIn(kDefaultControlPipeID, setup, &mut self.buf_)?; Ok(()) }
    fn InitializePhase2(&mut self, buffer: &[u8]) -> Result {
        if buffer.len() < 9 { return Err(ErrorCode::kInvalidDescriptor.into()); }
        crate::printk!("USB: configuration descriptor total={}\n", u16::from_le_bytes([buffer[2], buffer[3]]));
        let total_length = u16::from_le_bytes([buffer[2], buffer[3]]) as usize;
        let limit = total_length.min(buffer.len()); let mut offset = 0usize; let mut current_driver: Option<usize> = None; let mut pending_hid_interface: Option<(u8, u8)> = None;
        while offset + 2 <= limit {
            let length = buffer[offset] as usize; if length < 2 || offset + length > limit { break; }
            crate::printk!("USB: descriptor offset={} len={} type={}\n", offset, length, buffer[offset + 1]);
            match buffer[offset + 1] {
                4 if length >= 9 => { let class = buffer[offset + 5]; let subclass = buffer[offset + 6]; let protocol = buffer[offset + 8]; crate::printk!("USB: interface class={} subclass={} protocol={} number={}\n", class, subclass, protocol, buffer[offset + 2]); pending_hid_interface = if class == 3 && subclass == 1 { Some((buffer[offset + 2], protocol)) } else { None }; current_driver = if class == 3 && subclass == 1 && protocol == 1 { Some(self.add_keyboard(buffer[offset + 2])) } else if class == 3 && subclass == 1 && protocol == 2 { Some(self.add_mouse(buffer[offset + 2])) } else { None }; },
                5 if length >= 7 => if let Some((interface_number, protocol)) = pending_hid_interface { let address = buffer[offset + 2]; let attributes = buffer[offset + 3] & 3; let max_packet_size = u16::from_le_bytes([buffer[offset + 4], buffer[offset + 5]]) as i32; crate::printk!("USB: endpoint raw addr={:02x} attr={} packet={} protocol={}\n", address, attributes, max_packet_size, protocol); if current_driver.is_none() && protocol == 0 && attributes == 3 && address & 0x80 != 0 { current_driver = Some(self.add_mouse(interface_number)); } if let Some(driver_index) = current_driver { let config = EndpointConfig { ep_id: EndpointID::from_parts((address & 0xf) as i32, address & 0x80 != 0), ep_type: match attributes { 0 => EndpointType::kControl, 1 => EndpointType::kIsochronous, 2 => EndpointType::kBulk, _ => EndpointType::kInterrupt }, max_packet_size, interval: buffer[offset + 6] as i32 }; crate::printk!("USB: endpoint addr={:02x} attr={} packet={} driver={}\n", address, attributes, max_packet_size, driver_index); if self.num_ep_configs_ < 16 { self.class_drivers_[driver_index].as_mut().ok_or(ErrorCode::kNoWaiter)?.SetEndpoint(&config)?; self.endpoint_driver_[config.ep_id.Number() as usize] = Some(driver_index); self.ep_configs_[self.num_ep_configs_] = config; self.num_ep_configs_ += 1; } } },
                _ => {},
            }
            offset += length;
        }
        crate::printk!("USB: HID endpoints={} drivers={}\n", self.num_ep_configs_, self.class_drivers_.iter().filter(|driver| driver.is_some()).count());
        if self.num_ep_configs_ == 0 { return Ok(()); }
        self.initialize_phase_ = 3; let setup = SetupData::new(0, request::kSetConfiguration as u8, 1, 0, 0); self.backend.ControlOut(kDefaultControlPipeID, setup, &[])?; Ok(())
    }
    fn add_keyboard(&mut self, interface_index: u8) -> usize { let index = self.class_drivers_.iter().position(Option::is_none).unwrap_or(0); let mut driver = HIDKeyboardDriver::new(interface_index as i32); driver.SubscribeKeyPush(crate::usb::keyboard_observer_adapter); self.class_drivers_[index] = Some(ClassDriverSlot::Keyboard(driver)); index }
    fn add_mouse(&mut self, interface_index: u8) -> usize { let index = self.class_drivers_.iter().position(Option::is_none).unwrap_or(0); let mut driver = HIDMouseDriver::new(interface_index as i32); driver.SubscribeMouseMove(crate::usb::mouse_observer_adapter); self.class_drivers_[index] = Some(ClassDriverSlot::Mouse(driver)); index }
    fn OnEndpointsConfigured(&mut self) -> Result { for index in 0..self.class_drivers_.len() { if let Some(driver) = self.class_drivers_[index].as_mut() { driver.OnEndpointsConfigured()?; let setup = driver.protocol_setup(); self.backend.ControlOut(kDefaultControlPipeID, setup, &[])?; } } Ok(()) }
    pub fn OnConfigureEndpointsCompleted(&mut self) -> Result { self.initialize_phase_ = 4; self.is_initialized_ = true; self.OnEndpointsConfigured() }
    pub fn OnInterruptCompleted(&mut self, ep_id: EndpointID, buffer: &[u8]) -> Result { let index = ep_id.Number() as usize; let driver_index = self.endpoint_driver_.get(index).and_then(|value| *value).ok_or(ErrorCode::kNoWaiter)?; let endpoint = { let driver = self.class_drivers_.get_mut(driver_index).and_then(Option::as_mut).ok_or(ErrorCode::kNoWaiter)?; driver.OnInterruptCompleted(ep_id, buffer)?; driver.interrupt_endpoint() }; let size = self.class_drivers_[driver_index].as_ref().ok_or(ErrorCode::kNoWaiter)?.interrupt_packet_size().min(self.buf_.len()); self.backend.InterruptIn(endpoint, &mut self.buf_[..size]) }
}

pub fn GetDescriptor<B: TransferBackend>(dev: &mut Device<B>, ep_id: EndpointID, desc_type: u8, desc_index: u8, buffer: &mut [u8]) -> Result { let setup = SetupData::new(0x80, request::kGetDescriptor as u8, ((desc_type as u16) << 8) | desc_index as u16, 0, buffer.len() as u16); dev.backend.ControlIn(ep_id, setup, buffer) }
pub fn SetConfiguration<B: TransferBackend>(dev: &mut Device<B>, ep_id: EndpointID, config_value: u8) -> Result { let setup = SetupData::new(0, request::kSetConfiguration as u8, config_value as u16, 0, 0); dev.backend.ControlOut(ep_id, setup, &[]) }