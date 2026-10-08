/**
 * @file bridge/rust_bridge.cpp
 *
 * Rust から C++ の USB ドライバ / PCI を呼ぶための extern "C" 薄ラッパー．
 * ここにはロジックを置かず，本家 MikanOS の関数をそのまま呼ぶだけにする．
 * Rust 側の対応物は kernel/src/usb/mod.rs と kernel/src/pci.rs (名前は本家と同じ)．
 */

#include <cstdint>
#include <functional>

#include "pci.hpp"
#include "usb/classdriver/keyboard.hpp"
#include "usb/classdriver/mouse.hpp"
#include "usb/xhci/xhci.hpp"

// Rust 側 pci::Device の repr(C) 定義と一致していることを確認する
static_assert(sizeof(pci::ClassCode) == 3, "pci::ClassCode layout");
static_assert(sizeof(pci::Device) == 7, "pci::Device layout");

namespace {
using MouseObserverFn = void (*)(uint8_t buttons, int8_t dx, int8_t dy);
using KeyboardObserverFn = void (*)(uint8_t modifier, uint8_t keycode, bool press);
MouseObserverFn mouse_observer = nullptr;
KeyboardObserverFn keyboard_observer = nullptr;
}  // namespace

extern "C" {

// ---- pci ----
int pci_ScanAllBus() { return pci::ScanAllBus().Cause(); }
int pci_num_device() { return pci::num_device; }
const pci::Device* pci_devices(int index) { return &pci::devices[index]; }
uint16_t pci_ReadVendorId(const pci::Device* dev) { return pci::ReadVendorId(*dev); }
uint32_t pci_ReadConfReg(const pci::Device* dev, uint8_t reg_addr) {
  return pci::ReadConfReg(*dev, reg_addr);
}
void pci_WriteConfReg(const pci::Device* dev, uint8_t reg_addr, uint32_t value) {
  pci::WriteConfReg(*dev, reg_addr, value);
}

// ---- usb::xhci ----
void usb_xhci_Initialize() { usb::xhci::Initialize(); }
void usb_xhci_ProcessEvents() { usb::xhci::ProcessEvents(); }
int usb_xhci_ProcessEvent() {
  return usb::xhci::ProcessEvent(*usb::xhci::controller).Cause();
}
bool usb_xhci_HasFront() {
  return usb::xhci::controller->PrimaryEventRing()->HasFront();
}

// ---- usb::HIDMouseDriver / usb::HIDKeyboardDriver ----
void usb_HIDMouseDriver_set_default_observer(MouseObserverFn f) {
  mouse_observer = f;
  usb::HIDMouseDriver::default_observer = [](uint8_t b, int8_t x, int8_t y) {
    if (mouse_observer) mouse_observer(b, x, y);
  };
}
void usb_HIDKeyboardDriver_set_default_observer(KeyboardObserverFn f) {
  keyboard_observer = f;
  usb::HIDKeyboardDriver::default_observer = [](uint8_t m, uint8_t k, bool p) {
    if (keyboard_observer) keyboard_observer(m, k, p);
  };
}

}  // extern "C"
