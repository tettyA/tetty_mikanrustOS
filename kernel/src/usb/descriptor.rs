#![allow(
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    unsafe_op_in_unsafe_fn
)]

use core::slice;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub usb_release: u16,
    pub device_class: u8,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_release: u16,
    pub manufacturer: u8,
    pub product: u8,
    pub serial_number: u8,
    pub num_configurations: u8,
}

impl DeviceDescriptor {
    pub const kType: u8 = 1;
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ConfigurationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration_id: u8,
    pub attributes: u8,
    pub max_power: u8,
}
impl ConfigurationDescriptor {
    pub const kType: u8 = 2;
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct InterfaceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_sub_class: u8,
    pub interface_protocol: u8,
    pub interface_id: u8,
}
impl InterfaceDescriptor {
    pub const kType: u8 = 4;
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct EndpointDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}
impl EndpointDescriptor {
    pub const kType: u8 = 5;
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct HIDDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub hid_release: u16,
    pub country_code: u8,
    pub num_descriptors: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ClassDescriptor {
    pub descriptor_type: u8,
    pub descriptor_length: u16,
}

impl HIDDescriptor {
    pub const kType: u8 = 33;

    pub unsafe fn GetClassDescriptor(&self, index: usize) -> Option<&ClassDescriptor> {
        if index >= self.num_descriptors as usize {
            return None;
        }
        let address = (self as *const Self as *const u8).add(core::mem::size_of::<Self>());
        Some(&*address.cast::<ClassDescriptor>().add(index))
    }
}

pub trait DescriptorType {
    const kType: u8;
}
impl DescriptorType for DeviceDescriptor {
    const kType: u8 = DeviceDescriptor::kType;
}
impl DescriptorType for ConfigurationDescriptor {
    const kType: u8 = ConfigurationDescriptor::kType;
}
impl DescriptorType for InterfaceDescriptor {
    const kType: u8 = InterfaceDescriptor::kType;
}
impl DescriptorType for EndpointDescriptor {
    const kType: u8 = EndpointDescriptor::kType;
}
impl DescriptorType for HIDDescriptor {
    const kType: u8 = HIDDescriptor::kType;
}

pub unsafe fn DescriptorDynamicCast<T: DescriptorType>(
    desc_data: *mut u8,
) -> Option<&'static mut T> {
    if desc_data.is_null() || *desc_data.add(1) != T::kType {
        return None;
    }
    Some(&mut *desc_data.cast::<T>())
}

pub unsafe fn DescriptorDynamicCastConst<T: DescriptorType>(
    desc_data: *const u8,
) -> Option<&'static T> {
    if desc_data.is_null() || *desc_data.add(1) != T::kType {
        return None;
    }
    Some(&*desc_data.cast::<T>())
}

pub unsafe fn descriptor_bytes<'a>(descriptor: *const u8, length: usize) -> &'a [u8] {
    slice::from_raw_parts(descriptor, length)
}
