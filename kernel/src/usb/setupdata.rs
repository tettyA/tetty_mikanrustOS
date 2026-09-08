#![allow(non_upper_case_globals, dead_code)]

pub mod request_type {
    pub const kDevice: i32 = 0;
    pub const kInterface: i32 = 1;
    pub const kEndpoint: i32 = 2;
    pub const kOther: i32 = 3;
    pub const kStandard: i32 = 0;
    pub const kClass: i32 = 1;
    pub const kVendor: i32 = 2;
    pub const kOut: i32 = 0;
    pub const kIn: i32 = 1;
}

pub mod request {
    pub const kGetStatus: i32 = 0;
    pub const kClearFeature: i32 = 1;
    pub const kSetFeature: i32 = 3;
    pub const kSetAddress: i32 = 5;
    pub const kGetDescriptor: i32 = 6;
    pub const kSetDescriptor: i32 = 7;
    pub const kGetConfiguration: i32 = 8;
    pub const kSetConfiguration: i32 = 9;
    pub const kGetInterface: i32 = 10;
    pub const kSetInterface: i32 = 11;
    pub const kSynchFrame: i32 = 12;
    pub const kSetProtocol: i32 = 11;
}

pub mod descriptor_type {
    pub const kDevice: i32 = 1;
    pub const kConfiguration: i32 = 2;
    pub const kString: i32 = 3;
    pub const kInterface: i32 = 4;
    pub const kEndpoint: i32 = 5;
    pub const kInterfacePower: i32 = 8;
    pub const kOTG: i32 = 9;
    pub const kDebug: i32 = 10;
    pub const kInterfaceAssociation: i32 = 11;
    pub const kBOS: i32 = 15;
    pub const kDeviceCapability: i32 = 16;
    pub const kHID: i32 = 33;
    pub const kSuperspeedUSBEndpointCompanion: i32 = 48;
    pub const kSuperspeedPlusIsochronousEndpointCompanion: i32 = 49;
}

#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct SetupData {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}

impl SetupData {
    pub const fn new(request_type: u8, request: u8, value: u16, index: u16, length: u16) -> Self {
        Self { request_type, request, value, index, length }
    }

    pub const fn recipient(self) -> u8 { self.request_type & 0x1f }
    pub const fn type_(self) -> u8 { (self.request_type >> 5) & 0x03 }
    pub const fn direction(self) -> u8 { self.request_type >> 7 }
}