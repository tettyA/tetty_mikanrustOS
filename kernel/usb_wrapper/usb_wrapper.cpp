

#include <new>
#include "pci.hpp"
#include "usb/memory.hpp"
#include "usb/device.hpp"
#include "usb/classdriver/mouse.hpp"
#include "usb/xhci/xhci.hpp"
#include "usb/xhci/trb.hpp"

void SwitchEhci2Xhci(const pci::Device &xhc_dev)
{
    bool intel_ehc_exist = false;
    for (int i = 0; i < pci::num_device; ++i)
    {
        if (pci::devices[i].class_code.Match(0x0cu, 0x03u, 0x20u) /* EHCI */ &&
            0x8086 == pci::ReadVendorId(pci::devices[i]))
        {
            intel_ehc_exist = true;
            break;
        }
    }
    if (!intel_ehc_exist)
    {
        return;
    }

    uint32_t superspeed_ports = pci::ReadConfReg(xhc_dev, 0xdc); // USB3PRM
    pci::WriteConfReg(xhc_dev, 0xd8, superspeed_ports);          // USB3_PSSEN
    uint32_t ehci2xhci_ports = pci::ReadConfReg(xhc_dev, 0xd4);  // XUSB2PRM
    pci::WriteConfReg(xhc_dev, 0xd0, ehci2xhci_ports);           // XUSB2PR

    // Log(kDebug, "SwitchEhci2Xhci: SS = %02, xHCI = %02x\n",
    //   superspeed_ports, ehci2xhci_ports);
}
// Rust側から渡される関数ポインタの型
typedef void (*MouseObserver)(int8_t displacement_x, int8_t displacement_y);

extern "C"
{
    static MouseObserver rust_mouse_observer = nullptr;
    static usb::xhci::Controller *xhc = nullptr;
    // 静的な空間に配置することによって関数抜けても消えない
    static uint8_t xhc_buffer[sizeof(usb::xhci::Controller)];

    void usb_set_mouse_obserever(MouseObserver observer)
    {
        rust_mouse_observer = observer;
        usb::HIDMouseDriver::default_observer = rust_mouse_observer;
    }
    void usb_enumarate()
    {
        pci::ScanAllBus();

        // intel製を有線してxHCを探す
        pci::Device *xhc_dev = nullptr;
        for (int i = 0; i < pci::num_device; ++i)
        {
            if (pci::devices[i].class_code.Match(0x0cu, 0x03u, 0x30u))
            {
                xhc_dev = &pci::devices[i];

                if (0x8086 == pci::ReadVendorId(*xhc_dev))
                {
                    break;
                }
            }
        }

        if (xhc_dev)
        {
            // Log(kInfo, "xHC has been found: %d.%d.\n",xhc_dev->bus,xhc_dev->device,xhc_dev->function);
        }

        // read BAR
        const WithError<uint64_t> xhc_bar = pci::ReadBar(*xhc_dev, 0);
        // Log(kDebug, "ReadBar: %s\n", xhc_bar.error.Name());
        const uint64_t xhc_mmio_base = xhc_bar.value & ~static_cast<uint64_t>(0xf);
        // Log(kDebug, "xHC mmio_base = %08lx\n", xhc_mmio_base);

        // init xHCI
        xhc = new (xhc_buffer) usb::xhci::Controller(xhc_mmio_base);
        // usb::xhci::Controller xhc{xhc_mmio_base};

        if (0x8086 == pci::ReadVendorId(*xhc_dev))
        {
            SwitchEhci2Xhci(*xhc_dev);
        }
        {
            auto err = xhc->Initialize();
            // Log(kDebug, "xhc.Initialize: %s\n", err.Name());
        }

        // Log(kInfo, "xHC starting\n");
        xhc->Run();

        // #@@range_begin(configure_port)
        // usb::HIDMouseDriver::default_observer = MouseObserver;

        for (int i = 1; i <= xhc->MaxPorts(); ++i)
        {
            auto port = xhc->PortAt(i);
            //  Log(kDebug, "Port %d: IsConnected=%d\n", i, port.IsConnected());

            if (port.IsConnected())
            {
                if (auto err = ConfigurePort(*xhc, port))
                {
                    //   Log(kError, "failed to configure port: %s at %s:%d\n",
                    //     err.Name(), err.File(), err.Line());
                    continue;
                }
            }
        }
        // #@@range_end(configure_port)
    }
    void usb_process_events()
    {
        usb::xhci::ProcessEvent(*xhc);
    }

    bool xhc_primaryeventring_hasfront()
    {
        return xhc->PrimaryEventRing()->HasFront();
    }
}