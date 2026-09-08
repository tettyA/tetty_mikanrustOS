#![allow(dead_code)]

use core::ptr::{self, write_volatile};
const COUNT_MAX: u32 = 0xFFFF_FFFF;
const INITIAL_COUNT_VALUE: u32 = 10_000_000;
const LVT_TIMER: *mut u32 = 0xFEE0_0320 as *mut u32; // 割り込み発生方法の設定など
const INITIAL_COUNT: *mut u32 = 0xFEE0_0380 as *mut u32; // カウンタ初期値
const CURRENT_COUNT: *mut u32 = 0xFEE0_0390 as *mut u32; // カウンタ現在値
const DIVIDE_CONFIG: *mut u32 = 0xFEE0_03E0 as *mut u32; // カウンタ減少スピードの設定

pub fn initialize_lapic_timer() {
    unsafe {
        write_volatile(DIVIDE_CONFIG, 0b1011); // 分周比 1:1
        write_volatile(LVT_TIMER, 32); // unmasked, one-shot
        write_volatile(INITIAL_COUNT, INITIAL_COUNT_VALUE);
    }
}

pub fn start_lapic_timer() {
    unsafe {
        write_volatile(INITIAL_COUNT, INITIAL_COUNT_VALUE);
    }
}

pub fn lapic_timer_elapsed() -> u32 {
    INITIAL_COUNT_VALUE - unsafe { ptr::read_volatile(CURRENT_COUNT) }
}

pub fn stop_lapic_timer() {
    unsafe {
        write_volatile(INITIAL_COUNT, 0);
    }
}
