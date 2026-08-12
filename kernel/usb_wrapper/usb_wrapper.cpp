#include "pci.hpp"
extern "C"
{
    void usb_enumarate()
    {
        pci::ScanAllBus();
    }
}