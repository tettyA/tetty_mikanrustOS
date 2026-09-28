#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code
)]

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    kControl = 0,
    kIsochronous = 1,
    kBulk = 2,
    kInterrupt = 3,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct EndpointID {
    addr_: i32,
}

impl EndpointID {
    pub const fn new(addr: i32) -> Self {
        Self { addr_: addr }
    }

    pub const fn from_parts(ep_num: i32, dir_in: bool) -> Self {
        Self::new((ep_num << 1) | dir_in as i32)
    }

    pub const fn Address(self) -> i32 {
        self.addr_
    }

    pub const fn Number(self) -> i32 {
        self.addr_ >> 1
    }

    pub const fn IsIn(self) -> bool {
        (self.addr_ & 1) != 0
    }
}

pub const kDefaultControlPipeID: EndpointID = EndpointID::from_parts(0, true);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EndpointConfig {
    pub ep_id: EndpointID,
    pub ep_type: EndpointType,
    pub max_packet_size: i32,
    pub interval: i32,
}
