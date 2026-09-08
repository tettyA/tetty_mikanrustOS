use crate::usb::{endpoint::{EndpointConfig, EndpointID}, error::Result};
use super::{base::ClassDriver, hid::{HIDBaseDriver, HIDDataHandler}};

pub type ObserverType = extern "C" fn(u8, i8, i8);
pub struct HIDMouseDriver { pub hid: HIDBaseDriver, observers_: [Option<ObserverType>; 4], num_observers_: usize }
impl HIDMouseDriver {
    pub fn new(interface_index: i32) -> Self { Self { hid: HIDBaseDriver::new(interface_index, 3), observers_: [None; 4], num_observers_: 0 } }
    pub fn SubscribeMouseMove(&mut self, observer: ObserverType) { if self.num_observers_ < 4 { self.observers_[self.num_observers_] = Some(observer); self.num_observers_ += 1; } }
    fn NotifyMouseMove(&self, buttons: u8, x: i8, y: i8) { for observer in self.observers_.iter().take(self.num_observers_).flatten() { observer(buttons, x, y); } }
    pub fn OnDataReceived(&mut self) -> Result { if self.hid.in_packet_size_ < 3 { return Err(crate::usb::error::ErrorCode::kBufferTooSmall.into()); } crate::printk!("HID mouse report raw={:02x} {:02x} {:02x} {:02x} buttons={:02x} dx={} dy={}\n", self.hid.buf_[0], self.hid.buf_[1], self.hid.buf_[2], self.hid.buf_[3], self.hid.buf_[0], self.hid.buf_[1] as i8, self.hid.buf_[2] as i8); self.NotifyMouseMove(self.hid.buf_[0], self.hid.buf_[1] as i8, self.hid.buf_[2] as i8); Ok(()) }
}
impl HIDDataHandler for HIDMouseDriver { fn OnDataReceived(&mut self) -> Result { self.OnDataReceived() } }
impl ClassDriver for HIDMouseDriver {
    fn Initialize(&mut self) -> Result { Ok(()) }
    fn SetEndpoint(&mut self, config: &EndpointConfig) -> Result { self.hid.SetEndpoint(config) }
    fn OnEndpointsConfigured(&mut self) -> Result { self.hid.set_protocol_setup(); Ok(()) }
    fn OnControlCompleted(&mut self, ep_id: EndpointID, setup_data: crate::usb::setupdata::SetupData, buf: &[u8]) -> Result { self.hid.OnControlCompleted(ep_id, setup_data, buf) }
    fn OnInterruptCompleted(&mut self, ep_id: EndpointID, buf: &[u8]) -> Result { let mut hid = core::mem::replace(&mut self.hid, HIDBaseDriver::new(0, 3)); let result = hid.OnInterruptCompleted(self, ep_id, buf); self.hid = hid; result }
}