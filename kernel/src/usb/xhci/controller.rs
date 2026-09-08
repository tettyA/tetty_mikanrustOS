#![allow(non_snake_case, non_upper_case_globals, dead_code, unsafe_op_in_unsafe_fn)]

use crate::pci;
use super::{registers::{CapabilityRegisters, OperationalRegisters, InterrupterRegisterSet, PortRegisterSet, DoorbellRegister}, ring::{EventRing, Ring}, port::Port, trb::{EnableSlotCommandTRB, AddressDeviceCommandTRB, ConfigureEndpointCommandTRB, CommandCompletionEventTRB, PortStatusChangeEventTRB, TransferEventTRB}};
use crate::usb::memory::{AllocArray, FreeMem};
use crate::usb::device::Device as UsbDevice;
use super::device::Device as XhciDevice;
use super::context::{DeviceContext, InputContext};
use core::ptr::{read_volatile, write_volatile};

struct SlotState { port_id_: u8, context_: *mut DeviceContext, input_context_: *mut InputContext, transfer_ring_: *mut Ring, usb_device_: Option<UsbDevice<XhciDevice>> }

pub struct Controller {
    mmio_base_: usize,
    cap_: *mut CapabilityRegisters,
    op_: *mut OperationalRegisters,
    max_ports_: u8,
    command_ring_: Ring,
    event_ring_: EventRing,
    doorbell_base_: *mut DoorbellRegister,
    dcbaa_: *mut u64,
    slots_: [Option<SlotState>; 256],
    addressing_port_: u8,
}

static mut CONTROLLER: Option<Controller> = None;

pub fn Initialize() {
    unsafe {
    crate::printk!("xHCI: initialize begin\n");
        CONTROLLER = None;
        for index in 0..pci::num_device {
            let device = pci::devices[index];
            if device.class_code.Match(0x0c, 0x03, 0x30) {
                crate::printk!("xHCI: PCI device {:02x}.{:02x}.{:02x}\n", device.bus, device.device, device.function);
                if let Some(mmio_base) = pci::ReadBar(&device, 0) {
                    if pci::ReadVendorId(device.bus, device.device, device.function) == 0x8086 { SwitchEhci2Xhci(&device); }
                    let _ = pci::ConfigureMSIFixedDestination(&device, 0, 0x40, 0);
                    if let Some(mut controller) = Controller::new(mmio_base as usize) {
                        crate::printk!("xHCI: registers and rings ready\n");
                        controller.Run();
                        CONTROLLER = Some(controller);
                        crate::printk!("xHCI: controller running\n");
                    }
                }
                break;
            }
        }
    }
}

unsafe fn SwitchEhci2Xhci(xhci: &pci::Device) {
    let mut ehci_present = false;
    for index in 0..pci::num_device {
        let device = pci::devices[index];
        if device.class_code.Match(0x0c, 0x03, 0x20) && pci::ReadVendorId(device.bus, device.device, device.function) == 0x8086 { ehci_present = true; break; }
    }
    if ehci_present {
        let superspeed_ports = pci::ReadConfReg(xhci, 0xdc);
        pci::WriteConfReg(xhci, 0xd8, superspeed_ports);
        let usb2_ports = pci::ReadConfReg(xhci, 0xd4);
        pci::WriteConfReg(xhci, 0xd0, usb2_ports);
    }
}

impl Controller {
    unsafe fn new(mmio_base: usize) -> Option<Self> {
        let cap = mmio_base as *mut CapabilityRegisters;
        Self::RequestHCOwnership(mmio_base, (*cap).HCCPARAMS1.Read().extended_capabilities_pointer());
        let cap_length = (*cap).CAPLENGTH.Read() as usize;
        let op = (mmio_base + cap_length) as *mut OperationalRegisters;
        let max_ports = (*cap).HCSPARAMS1.Read().max_ports();
        let mut command_ring = Ring::new();
        if !command_ring.Initialize(256) { return None; }
        let doorbell_base = (mmio_base + (*cap).DBOFF.Read().Offset() as usize) as *mut DoorbellRegister;
        let dcbaa = AllocArray::<u64>(256, 64, 4096);
        if dcbaa.is_null() { return None; }
        for index in 0..256 { dcbaa.add(index).write(0); }
        let mut controller = Self { mmio_base_: mmio_base, cap_: cap, op_: op, max_ports_: max_ports, command_ring_: command_ring, event_ring_: EventRing::new(), doorbell_base_: doorbell_base, dcbaa_: dcbaa, slots_: [const { None }; 256], addressing_port_: 0 };
        let mut command = (*controller.op_).USBCMD.Read();
        command.set_run_stop(false);
        (*controller.op_).USBCMD.Write(command);
        let mut wait = 1_000_000usize;
        while !((*controller.op_).USBSTS.Read().data[0] & 1 != 0) && wait != 0 { wait -= 1; }
        if wait == 0 { return None; }
        command.set_reset(true);
        (*controller.op_).USBCMD.Write(command);
        wait = 1_000_000;
        while ((*controller.op_).USBCMD.Read().data[0] & (1 << 1)) != 0 && wait != 0 { wait -= 1; }
        if wait == 0 { return None; }
        wait = 1_000_000;
        while ((*controller.op_).USBSTS.Read().data[0] & (1 << 11)) != 0 && wait != 0 { wait -= 1; }
        if wait == 0 { return None; }
        let mut config = (*controller.op_).CONFIG.Read();
        config.SetMaxDeviceSlotsEnabled((*controller.cap_).HCSPARAMS1.Read().max_device_slots());
        (*controller.op_).CONFIG.Write(config);
        let mut dcbaap = (*controller.op_).DCBAAP.Read();
        dcbaap.SetPointer(controller.dcbaa_ as u64);
        (*controller.op_).DCBAAP.Write(dcbaap);
        let mut crcr = (*controller.op_).CRCR.Read();
        crcr.set_ring_cycle_state(true);
        crcr.SetPointer(controller.command_ring_.Buffer() as u64);
        (*controller.op_).CRCR.Write(crcr);
        let interrupter = (mmio_base + (*cap).RTSOFF.Read().Offset() as usize + 0x20) as *mut InterrupterRegisterSet;
        if !controller.event_ring_.Initialize(256, interrupter) { return None; }
        let mut iman = (*interrupter).IMAN.Read(); iman.set_interrupt_pending(); iman.set_interrupt_enable(true); (*interrupter).IMAN.Write(iman);
        let mut command = (*controller.op_).USBCMD.Read(); command.set_run_stop(true); command.data[0] |= 1 << 2; (*controller.op_).USBCMD.Write(command);
        Some(controller)
    }

    unsafe fn RequestHCOwnership(mmio_base: usize, extended_pointer: u16) {
        let mut offset = (extended_pointer as usize) * 4;
        let mut attempts = 0usize;
        while offset != 0 && attempts < 64 {
            let address = (mmio_base + offset) as *mut u32;
            let value = read_volatile(address);
            let capability_id = value & 0xff;
            let next = ((value >> 8) & 0xff) as usize;
            if capability_id == 1 {
                if value & (1 << 24) == 0 {
                    write_volatile(address, value | (1 << 24));
                    let mut wait = 1_000_000usize;
                    while wait != 0 {
                        let current = read_volatile(address);
                        if current & (1 << 16) == 0 && current & (1 << 24) != 0 { break; }
                        wait -= 1;
                    }
                }
                break;
            }
            offset = next * 4;
            attempts += 1;
        }
    }

    pub unsafe fn Run(&mut self) {
        let mut command = (*self.op_).USBCMD.Read();
        command.set_run_stop(true);
        (*self.op_).USBCMD.Write(command);
        for port_number in 1..=self.max_ports_ {
            if let Some(mut port) = self.PortAt(port_number) {
                if port.IsConnected() {
                    self.addressing_port_ = port_number;
                    let _ = port.Reset();
                    break;
                }
            }
        }
        let enable_slot = EnableSlotCommandTRB::new();
        self.command_ring_.Push(enable_slot.data);
        (*self.doorbell_base_).Ring(0, 0);
    }
    pub unsafe fn PortAt(&self, port_num: u8) -> Option<Port> { if port_num == 0 || port_num > self.max_ports_ { return None; } Some(Port::new(port_num, (self.op_ as usize + 0x400 + (port_num as usize - 1) * core::mem::size_of::<PortRegisterSet>()) as *mut PortRegisterSet)) }
    unsafe fn AddressDevice(&mut self, port_id: u8, slot_id: u8) {
        crate::printk!("xHCI: AddressDevice port={} slot={}\n", port_id, slot_id);
        if slot_id == 0 || slot_id as usize >= self.slots_.len() { return; }
        let input = AllocArray::<InputContext>(1, 64, 4096); let context = AllocArray::<DeviceContext>(1, 64, 4096); let ring = AllocArray::<Ring>(1, 64, 4096);
        if input.is_null() || context.is_null() || ring.is_null() { return; }
        input.write(InputContext::default()); context.write(DeviceContext::default()); ring.write(Ring::new()); if !(*ring).Initialize(32) { return; }
        let port = match self.PortAt(port_id) { Some(port) => port, None => return }; let speed = port.Speed();
        let slot = (*input).EnableSlotContext(); (*slot).set_root_hub_port_num(port_id as u32); (*slot).set_speed(speed as u32); (*slot).set_context_entries(1);
        let ep = (*input).EnableEndpoint(super::context::DeviceContextIndex::from_parts(0, false)); (*ep).set_endpoint_type(4); (*ep).set_max_packet_size(if speed == 4 { 512 } else if speed == 3 { 64 } else { 8 }); (*ep).SetTransferRingBuffer((*ring).Buffer()); (*ep).dwords[2] |= 1;
        self.dcbaa_.add(slot_id as usize).write(context as u64);
        let mut backend = XhciDevice::new(slot_id, self.doorbell_base_.add(slot_id as usize), input, &mut self.command_ring_, self.doorbell_base_, context); backend.transfer_rings_[0] = ring; self.slots_[slot_id as usize] = Some(SlotState { port_id_: port_id, context_: context, input_context_: input, transfer_ring_: ring, usb_device_: Some(UsbDevice::new(backend)) });
        self.command_ring_.Push(AddressDeviceCommandTRB::new(input, slot_id).data); (*self.doorbell_base_).Ring(0, 0);
    }
    unsafe fn DispatchPortStatusChange(&mut self, event: &PortStatusChangeEventTRB) { let port_id = event.port_id(); if let Some(mut port) = self.PortAt(port_id) { let connected = port.IsConnected(); crate::printk!("xHCI: port status port={} connected={} enabled={} speed={}\n", port_id, connected, port.IsEnabled(), port.Speed()); port.ClearConnectStatusChanged(); if !connected { for slot_id in 1..self.slots_.len() { if self.slots_[slot_id].as_ref().is_some_and(|slot| slot.port_id_ == port_id) { if let Some(mut slot) = self.slots_[slot_id].take() { if let Some(mut usb) = slot.usb_device_.take() { usb.backend.ReleaseTransferRings(); } FreeMem(slot.input_context_ as *mut u8); FreeMem(slot.context_ as *mut u8); self.dcbaa_.add(slot_id).write(0); } break; } } } else if !port.IsEnabled() { let _ = port.Reset(); self.addressing_port_ = port_id; let enable_slot = EnableSlotCommandTRB::new(); self.command_ring_.Push(enable_slot.data); (*self.doorbell_base_).Ring(0, 0); } } }
    unsafe fn DispatchCommandCompletion(&mut self, event: &CommandCompletionEventTRB) { let issuer = event.Pointer(); if issuer.is_null() { return; } let code = (event.data[2] >> 24) & 0xff; crate::printk!("xHCI: command complete type={} slot={} code={}\n", (*issuer).trb_type(), event.slot_id(), code); if code != 1 { return; } match (*issuer).trb_type() as u32 { EnableSlotCommandTRB::Type => { self.AddressDevice(self.addressing_port_, event.slot_id()); }, AddressDeviceCommandTRB::Type => { if let Some(slot) = self.slots_.get_mut(event.slot_id() as usize).and_then(Option::as_mut) { if let Some(usb) = slot.usb_device_.as_mut() { crate::printk!("USB: address complete, requesting descriptor\n"); let _ = usb.StartInitialize(); } } }, ConfigureEndpointCommandTRB::Type => { if let Some(slot) = self.slots_.get_mut(event.slot_id() as usize).and_then(Option::as_mut) { if let Some(usb) = slot.usb_device_.as_mut() { crate::printk!("USB: endpoint configuration complete\n"); let _ = usb.OnConfigureEndpointsCompleted(); } } }, _ => {} } }
    unsafe fn DispatchTransferCompletion(&mut self, event: &TransferEventTRB) { let slot_id = ((event.data[3] >> 24) & 0xff) as usize; let code = event.completion_code(); crate::printk!("xHCI: transfer complete slot={} ep={} code={} residual={}\n", slot_id, (event.data[3] >> 16) & 0x1f, code, event.data[2] & 0x00ff_ffff); if slot_id >= self.slots_.len() { return; } if let Some(slot) = self.slots_[slot_id].as_mut() { if let Some(usb) = slot.usb_device_.as_mut() { let backend = &mut usb.backend; let record = backend.complete_transfer(event.Pointer(), (event.data[2] & 0x00ff_ffff) as usize); if code != 1 && code != 13 { crate::printk!("USB: transfer failed code={}\n", code); return; } if let Some(record) = record { let bytes = core::slice::from_raw_parts(record.buffer, record.length); if record.setup.is_none() { crate::printk!("USB: interrupt received buf={:p} len={} raw={:02x} {:02x} {:02x} {:02x}\n", record.buffer, record.length, bytes.get(0).copied().unwrap_or(0), bytes.get(1).copied().unwrap_or(0), bytes.get(2).copied().unwrap_or(0), bytes.get(3).copied().unwrap_or(0)); } if let Some(setup) = record.setup { crate::printk!("USB: control completion req={} len={}\n", setup.request, record.length); let _ = usb.OnControlCompleted(setup, bytes); } else { let endpoint = ((event.data[3] >> 16) & 0x1f) as i32; let _ = usb.OnInterruptCompleted(crate::usb::endpoint::EndpointID::new(endpoint), bytes); } } else { crate::printk!("USB: no transfer record for {:p}\n", event.Pointer()); } } } }
}

impl Drop for Controller { fn drop(&mut self) { unsafe { if !self.dcbaa_.is_null() { FreeMem(self.dcbaa_ as *mut u8); } } } }

pub fn ProcessEvents() {
    unsafe {
        let controller = core::ptr::addr_of_mut!(CONTROLLER);
        if let Some(controller) = (*controller).as_mut() {
            while controller.event_ring_.HasFront() { let event = *controller.event_ring_.Front().unwrap(); crate::printk!("xHCI: event type={} d0={:08x}\n", event.trb_type(), event.data[0]); match event.trb_type() as u32 { CommandCompletionEventTRB::Type => controller.DispatchCommandCompletion(&*(core::ptr::addr_of!(event).cast::<CommandCompletionEventTRB>())), PortStatusChangeEventTRB::Type => controller.DispatchPortStatusChange(&*(core::ptr::addr_of!(event).cast::<PortStatusChangeEventTRB>())), TransferEventTRB::Type => controller.DispatchTransferCompletion(&*(core::ptr::addr_of!(event).cast::<TransferEventTRB>())), _ => {} } controller.event_ring_.Pop(); }
        }
    }
}
pub fn PrimaryEventRingHasFront() -> bool { unsafe { (core::ptr::addr_of!(CONTROLLER) as *const Option<Controller>).as_ref().and_then(Option::as_ref).is_some_and(|controller| controller.event_ring_.HasFront()) } }