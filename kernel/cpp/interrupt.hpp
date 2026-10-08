/**
 * @file interrupt.hpp
 *
 * usb/xhci/xhci.cpp が参照する割り込みベクタ番号の定義．
 * 本家 MikanOS の interrupt.hpp のうち USB ドライバが必要とする部分のみを抜粋したもの．
 * Rust 側 (src/interrupts.rs の INTERRUPTVECTOR_KXHCI) と値を一致させること．
 */

#pragma once

class InterruptVector {
 public:
  enum Number {
    kXHCI = 0x40,
  };
};
