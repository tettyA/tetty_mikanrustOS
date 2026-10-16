//! `pci` : 本家 MikanOS の `pci.hpp` / `pci.cpp` (namespace `pci`) の Rust 版インターフェース．
//!
//! 実体は `kernel/cpp/pci.cpp` (本家のコードそのまま) で，ここはその呼び出し口だけを提供する．
//! 関数名は本家と同じ (PascalCase) にしてある．
//!
//! # 使い方
//! ```ignore
//! pci::ScanAllBus();                       // PCI デバイスを全探索 (最初に 1 回)
//! for i in 0..pci::num_device() {
//!     let dev = pci::devices(i);
//!     if dev.class_code.Match3(0x0c, 0x03, 0x30) { /* xHCI */ }
//! }
//! ```
#![allow(non_snake_case, dead_code)]

use core::arch::asm;

use crate::usb;
use crate::usb::Error;
/// `pci::ClassCode` : PCI デバイスのクラスコード．
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ClassCode {
    pub base: u8,
    pub sub: u8,
    pub interface: u8,
}

impl ClassCode {
    /// ベースクラス, サブクラス, インターフェースがすべて等しければ true (`Match(b, s, i)`)．
    pub fn Match3(&self, b: u8, s: u8, i: u8) -> bool {
        self.base == b && self.sub == s && self.interface == i
    }
}

/// `pci::Device` : PCI デバイスを特定する情報 (C++ 側と同じレイアウト)．
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Device {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub header_type: u8,
    pub class_code: ClassCode,
}

unsafe extern "C" {
    fn pci_ScanAllBus() -> i32;
    fn pci_num_device() -> i32;
    fn pci_devices(index: i32) -> *const Device;
    fn pci_ReadVendorId(dev: *const Device) -> u16;
    fn pci_ReadConfReg(dev: *const Device, reg_addr: u8) -> u32;
    fn pci_WriteConfReg(dev: *const Device, reg_addr: u8, value: u32);
}

/// `pci::ScanAllBus()` : バス 0 から再帰的に PCI デバイスを探索し，一覧を内部に保持する．
/// 他の `pci::` / `usb::xhci::Initialize()` より先に 1 回呼ぶこと．
pub fn ScanAllBus() -> Error {
    Error::from_code(unsafe { pci_ScanAllBus() })
}

/// `pci::num_device` : `ScanAllBus()` で見つかったデバイス数．
pub fn num_device() -> usize {
    unsafe { pci_num_device() as usize }
}

/// `pci::devices[index]` : 見つかったデバイス (0 <= index < num_device())．
pub fn devices(index: usize) -> Device {
    assert!(index < num_device());
    unsafe { *pci_devices(index as i32) }
}

/// `pci::ReadVendorId(dev)` : ベンダ ID を読む (Intel は 0x8086)．
pub fn ReadVendorId(dev: &Device) -> u16 {
    unsafe { pci_ReadVendorId(dev) }
}

/// `pci::ReadConfReg(dev, reg_addr)` : コンフィグレーション空間の 32 ビットレジスタを読む．
pub fn ReadConfReg(dev: &Device, reg_addr: u8) -> u32 {
    unsafe { pci_ReadConfReg(dev, reg_addr) }
}

/// `pci::WriteConfReg(dev, reg_addr, value)` : コンフィグレーション空間の 32 ビットレジスタに書く．
pub fn WriteConfReg(dev: &Device, reg_addr: u8, value: u32) {
    unsafe { pci_WriteConfReg(dev, reg_addr, value) }
}

pub fn initialize_pci() {
    ScanAllBus();
}

/// タスク初期化の後に呼ぶ。キーボードの obserever はこれより前に設定しておくこと。
/// xHCI を初期化すると，割込みが来初める。宛先のメインタスクが先にある必要があるのでタスク初期化の後に呼ぶ。
pub fn initialize_usb() {
    usb::HIDMouseDriver::set_default_observer(crate::mousecursor::MouseObserver);
    usb::xhci::Initialize();
}
