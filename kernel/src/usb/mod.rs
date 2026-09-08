#![allow(non_camel_case_types, non_snake_case, dead_code)]

pub mod arraymap;
pub mod descriptor;
pub mod endpoint;
pub mod error;
pub mod classdriver;
pub mod device;
pub mod memory;
pub mod setupdata;
pub mod xhci;

static mut MOUSE_OBSERVER: Option<extern "C" fn(i8, i8)> = None;
static mut KEYBOARD_OBSERVER: Option<extern "C" fn(u8, u8, bool)> = None;

pub extern "C" fn mouse_observer_adapter(_buttons: u8, displacement_x: i8, displacement_y: i8) {
    crate::printk!("USB mouse observer dx={} dy={}\n", displacement_x, displacement_y);
    unsafe { if let Some(observer) = core::ptr::addr_of!(MOUSE_OBSERVER).read() { observer(displacement_x, displacement_y); } }
}

pub fn usb_set_mouse_obserever(observer: extern "C" fn(i8, i8)) {
    unsafe { core::ptr::addr_of_mut!(MOUSE_OBSERVER).write(Some(observer)); }
}

pub fn usb_set_keyboard_observer(observer: extern "C" fn(u8, u8, bool)) {
    unsafe { core::ptr::addr_of_mut!(KEYBOARD_OBSERVER).write(Some(observer)); }
}

pub extern "C" fn keyboard_observer_adapter(modifier: u8, keycode: u8, pressed: bool) {
    unsafe { if let Some(observer) = core::ptr::addr_of!(KEYBOARD_OBSERVER).read() { observer(modifier, keycode, pressed); } }
}

pub fn usb_enumarate() {
    crate::printk!("USB: PCI scan begin\n");
    unsafe { let _ = crate::pci::ScanAllBus(); }
    crate::printk!("USB: PCI scan done\n");
    xhci::controller::Initialize();
    crate::printk!("USB: controller init returned\n");
}

pub fn usb_process_events() { xhci::controller::ProcessEvents(); }
pub fn xhc_primaryeventring_hasfront() -> bool { xhci::controller::PrimaryEventRingHasFront() }

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::descriptor::{ConfigurationDescriptor, DeviceDescriptor, EndpointDescriptor};
    use super::endpoint::{EndpointConfig, EndpointID, EndpointType};
    use super::setupdata::SetupData;

    #[test]
    fn c_layout_matches_usb_definitions() {
        assert_eq!(size_of::<DeviceDescriptor>(), 18);
        assert_eq!(size_of::<ConfigurationDescriptor>(), 9);
        assert_eq!(size_of::<EndpointDescriptor>(), 7);
        assert_eq!(size_of::<SetupData>(), 8);
        assert_eq!(size_of::<EndpointID>(), size_of::<i32>());
        assert_eq!(size_of::<EndpointConfig>(), 16);
        assert_eq!(align_of::<SetupData>(), 1);
        assert_eq!(align_of::<DeviceDescriptor>(), 1);
    }

    #[test]
    fn endpoint_id_preserves_cpp_encoding() {
        let endpoint = EndpointID::new(3, true);
        assert_eq!(endpoint.Address(), 7);
        assert_eq!(endpoint.Number(), 3);
        assert!(endpoint.IsIn());
        assert_eq!(EndpointType::kInterrupt as u8, 3);
    }
}