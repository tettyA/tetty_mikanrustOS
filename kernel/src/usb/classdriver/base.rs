use crate::usb::{endpoint::EndpointConfig, endpoint::EndpointID, setupdata::SetupData};
use crate::usb::error::Result;

pub trait ClassDriver {
    fn Initialize(&mut self) -> Result;
    fn SetEndpoint(&mut self, config: &EndpointConfig) -> Result;
    fn OnEndpointsConfigured(&mut self) -> Result;
    fn OnControlCompleted(&mut self, ep_id: EndpointID, setup_data: SetupData, buf: &[u8]) -> Result;
    fn OnInterruptCompleted(&mut self, ep_id: EndpointID, buf: &[u8]) -> Result;
}