use crate::usb::{endpoint::{EndpointConfig, EndpointID}, error::Result};
use super::{base::ClassDriver, hid::{HIDBaseDriver, HIDDataHandler}};

pub type ObserverType = extern "C" fn(u8, u8, bool);

pub struct HIDKeyboardDriver { pub hid: HIDBaseDriver, observers_: [Option<ObserverType>; 4], num_observers_: usize }
impl HIDKeyboardDriver {
    pub fn new(interface_index: i32) -> Self { Self { hid: HIDBaseDriver::new(interface_index, 8), observers_: [None; 4], num_observers_: 0 } }
    pub fn SubscribeKeyPush(&mut self, observer: ObserverType) { if self.num_observers_ < 4 { self.observers_[self.num_observers_] = Some(observer); self.num_observers_ += 1; } }
    fn NotifyKeyPush(&self, modifier: u8, keycode: u8, press: bool) { for observer in self.observers_.iter().take(self.num_observers_).flatten() { observer(modifier, keycode, press); } }
    pub fn OnDataReceived(&mut self) -> Result { let mut previous = [false; 256]; let mut current = [false; 256]; for key in &self.hid.previous_buf_[2..8] { previous[*key as usize] = true; } for key in &self.hid.buf_[2..8] { current[*key as usize] = true; } for key in 1..256 { if previous[key] != current[key] { self.NotifyKeyPush(self.hid.buf_[0], key as u8, current[key]); } } Ok(()) }
}
impl HIDDataHandler for HIDKeyboardDriver { fn OnDataReceived(&mut self) -> Result { self.OnDataReceived() } }
impl ClassDriver for HIDKeyboardDriver {
    fn Initialize(&mut self) -> Result { Ok(()) }
    fn SetEndpoint(&mut self, config: &EndpointConfig) -> Result { self.hid.SetEndpoint(config) }
    fn OnEndpointsConfigured(&mut self) -> Result { self.hid.set_protocol_setup(); Ok(()) }
    fn OnControlCompleted(&mut self, ep_id: EndpointID, setup_data: crate::usb::setupdata::SetupData, buf: &[u8]) -> Result { self.hid.OnControlCompleted(ep_id, setup_data, buf) }
    fn OnInterruptCompleted(&mut self, ep_id: EndpointID, buf: &[u8]) -> Result { let mut hid = core::mem::replace(&mut self.hid, HIDBaseDriver::new(0, 8)); let result = hid.OnInterruptCompleted(self, ep_id, buf); self.hid = hid; result }
}