//! `usb` : 本家 MikanOS の USB ドライバ (`namespace usb`) を Rust から使うためのインターフェース．
//!
//! ドライバ本体は C++ のまま `kernel/cpp/usb/` にあり (本家コードを無改造で使用)，
//! `kernel/cpp/bridge/rust_bridge.cpp` 経由で呼び出す．名前は本家と同じにしてある．
//!
//! # 本家 main.cpp との対応
//! | C++ (main.cpp)                                  | Rust                                              |
//! |-------------------------------------------------|---------------------------------------------------|
//! | `pci::ScanAllBus();`                            | `pci::ScanAllBus();`                              |
//! | `usb::HIDMouseDriver::default_observer = f;`    | `usb::HIDMouseDriver::set_default_observer(f);`   |
//! | `usb::HIDKeyboardDriver::default_observer = f;` | `usb::HIDKeyboardDriver::set_default_observer(f);`|
//! | `usb::xhci::Initialize();`                      | `usb::xhci::Initialize();`                        |
//! | `usb::xhci::ProcessEvents();`                   | `usb::xhci::ProcessEvents();`                     |
//!
//! # 典型的な使い方 (KernelMain)
//! ```ignore
//! pci::ScanAllBus();
//! usb::HIDMouseDriver::set_default_observer(mouse_observer);
//! usb::xhci::Initialize();      // xHC 発見 → 初期化 → MSI 設定 → 起動 → 接続済みポート設定
//! // 割り込みを有効化 (sti) した後, xHCI 割り込み (ベクタ 0x40) のメッセージを受けたら:
//! usb::xhci::ProcessEvents();   // 溜まっているイベントを全て処理
//! ```
#![allow(non_snake_case, dead_code)]

/// `Error` : 本家 `error.hpp` の `Error` クラス相当．コードのみ保持する．
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Error(i32);

impl Error {
    /// `Error::kSuccess` のコード値．
    pub const kSuccess: i32 = 0;

    /// C++ 側のエラーコードから生成する．
    pub fn from_code(code: i32) -> Self {
        Error(code)
    }

    /// `Error::Cause()` : エラーコード (`Error::Code` の値)．0 が成功．
    pub fn Cause(&self) -> i32 {
        self.0
    }

    /// 本家の `operator bool` 相当: エラーが起きていれば true (`if (auto err = ...)` に対応)．
    pub fn is_err(&self) -> bool {
        self.0 != Self::kSuccess
    }
}

unsafe extern "C" {
    fn usb_HIDMouseDriver_set_default_observer(f: extern "C" fn(u8, i8, i8));
    fn usb_HIDKeyboardDriver_set_default_observer(f: extern "C" fn(u8, u8, bool));
    fn usb_xhci_Initialize();
    fn usb_xhci_ProcessEvents();
    fn usb_xhci_ProcessEvent() -> i32;
    fn usb_xhci_HasFront() -> bool;
}

/// マウス移動オブザーバの型 (`usb::HIDMouseDriver::ObserverType` 相当)．
/// 引数: (ボタン状態ビット, X 方向変位, Y 方向変位)．
pub type MouseObserverType = extern "C" fn(buttons: u8, displacement_x: i8, displacement_y: i8);

/// キー入力オブザーバの型 (`usb::HIDKeyboardDriver::ObserverType` 相当)．
/// 引数: (修飾キービット, キーコード, 押下なら true / 離したなら false)．
pub type KeyboardObserverType = extern "C" fn(modifier: u8, keycode: u8, press: bool);

/// `usb::HIDMouseDriver` : USB マウスのクラスドライバ．
pub struct HIDMouseDriver;

impl HIDMouseDriver {
    /// `usb::HIDMouseDriver::default_observer = f;` 相当．
    /// 以降に接続・認識されるマウスが動くたびに `f` が呼ばれる．
    /// `usb::xhci::Initialize()` より前に呼ぶこと (初期化時に接続済みのマウスにも効かせるため)．
    pub fn set_default_observer(f: MouseObserverType) {
        unsafe { usb_HIDMouseDriver_set_default_observer(f) }
    }
}

/// `usb::HIDKeyboardDriver` : USB キーボードのクラスドライバ．
pub struct HIDKeyboardDriver;

impl HIDKeyboardDriver {
    /// `usb::HIDKeyboardDriver::default_observer = f;` 相当．
    /// キーが押された/離されたときに `f` が呼ばれる．`usb::xhci::Initialize()` より前に呼ぶこと．
    pub fn set_default_observer(f: KeyboardObserverType) {
        unsafe { usb_HIDKeyboardDriver_set_default_observer(f) }
    }
}

/// `usb::xhci` : xHCI (USB 3.0) ホストコントローラ関連．
pub mod xhci {
    use super::*;

    /// `usb::xhci::Initialize()` : xHC を探して初期化・起動し，接続済みポートを設定する．
    /// 事前に `pci::ScanAllBus()` を呼んでおくこと．xHC が見つからない/初期化失敗時は停止する．
    /// MSI 割り込み (ベクタ 0x40) もここで設定される．
    pub fn Initialize() {
        unsafe { usb_xhci_Initialize() }
    }

    /// `usb::xhci::ProcessEvents()` : プライマリイベントリングに溜まったイベントを全て処理する．
    /// xHCI 割り込みを受けたメインループから呼ぶ．
    pub fn ProcessEvents() {
        unsafe { usb_xhci_ProcessEvents() }
    }

    /// `usb::xhci::ProcessEvent(*controller)` : イベントを高々 1 つ処理する．
    pub fn ProcessEvent() -> Error {
        Error::from_code(unsafe { usb_xhci_ProcessEvent() })
    }

    /// `usb::xhci::controller->PrimaryEventRing()->HasFront()` :
    /// 未処理のイベントがあれば true．
    pub fn HasFront() -> bool {
        unsafe { usb_xhci_HasFront() }
    }
}
