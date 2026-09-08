#![allow(non_snake_case, dead_code, unsafe_op_in_unsafe_fn)]

use super::registers::PortRegisterSet;

pub struct Port { pub port_num_: u8, pub port_reg_set_: *mut PortRegisterSet }
impl Port {
    pub const fn new(port_num: u8, registers: *mut PortRegisterSet) -> Self { Self { port_num_: port_num, port_reg_set_: registers } }
    pub const fn Number(&self) -> u8 { self.port_num_ }
    pub unsafe fn IsConnected(&self) -> bool { (*self.port_reg_set_).PORTSC.Read().connected() }
    pub unsafe fn IsEnabled(&self) -> bool { (*self.port_reg_set_).PORTSC.Read().enabled() }
    pub unsafe fn Speed(&self) -> u8 { (*self.port_reg_set_).PORTSC.Read().speed() }
    pub unsafe fn IsConnectStatusChanged(&self) -> bool { (*self.port_reg_set_).PORTSC.Read().data[0] & (1 << 17) != 0 }
    pub unsafe fn ClearConnectStatusChanged(&self) { let mut portsc = (*self.port_reg_set_).PORTSC.Read(); portsc.data[0] &= 0x0e01_c3e0; portsc.data[0] |= 1 << 17; (*self.port_reg_set_).PORTSC.Write(portsc); }
    pub unsafe fn Reset(&mut self) -> bool {
        let mut portsc = (*self.port_reg_set_).PORTSC.Read(); portsc.request_reset(); (*self.port_reg_set_).PORTSC.Write(portsc);
        let mut wait = 1_000_000usize;
        while (*self.port_reg_set_).PORTSC.Read().reset() && wait != 0 { wait -= 1; }
        wait != 0
    }
}