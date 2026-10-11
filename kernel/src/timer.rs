use alloc::vec::{self, Vec};

use crate::interrupts::InterruptVector;
use crate::message::Message;
use crate::queue::Queue;
use crate::{acpi, global::Global};
use core::ptr::{self, write_volatile};
const COUNT_MAX: u32 = 0xFFFF_FFFF;
const LVT_TIMER: *mut u32 = 0xFEE0_0320 as *mut u32; // 割り込み発生方法の設定など
const INITIAL_COUNT: *mut u32 = 0xFEE0_0380 as *mut u32; // カウンタ初期値
const CURRENT_COUNT: *mut u32 = 0xFEE0_0390 as *mut u32; // カウンタ現在値
const DIVIDE_CONFIG: *mut u32 = 0xFEE0_03E0 as *mut u32; // カウンタ減少スピードの設定

pub fn initialize_lapic_timer(msg_queue: &'static Global<Queue<Message>>) {
    TIMER_MANAGER.with(|tm| *tm = Some(TimerManager::new(msg_queue)));

    unsafe {
        write_volatile(DIVIDE_CONFIG, 0b1011); // 分周比 1:1
        write_volatile(
            LVT_TIMER,
            (0b001 << 16) | InterruptVector::LAPICTimer as u32,
        ); // masked, one-shot
    }
    start_lapic_timer();
    acpi::wait_milliseconds(100);
    let elapsed = lapic_timer_elapsed();
    stop_lapic_timer();

    LAPIC_TIMER_FREQ.with(|freq| *freq = Some(elapsed * 10));

    unsafe {
        write_volatile(DIVIDE_CONFIG, 0b1011); // 分周比 1:1
        write_volatile(
            LVT_TIMER,
            (0b010 << 16) | InterruptVector::LAPICTimer as u32,
        ); // not masked, periodic
    }
    let initial_count = LAPIC_TIMER_FREQ.with(|freq| freq.unwrap()) / TIMER_FREQ;
    unsafe {
        write_volatile(INITIAL_COUNT, initial_count);
    }
}

pub fn start_lapic_timer() {
    unsafe {
        write_volatile(INITIAL_COUNT, COUNT_MAX);
    }
}

pub fn lapic_timer_elapsed() -> u32 {
    COUNT_MAX - unsafe { ptr::read_volatile(CURRENT_COUNT) }
}

pub fn stop_lapic_timer() {
    unsafe {
        write_volatile(INITIAL_COUNT, 0);
    }
}

pub fn lapic_timer_on_interrupt() {
    TIMER_MANAGER.with(|tm| {
        if let Some(tm) = tm {
            tm.tick();
        }
    });
}

#[derive(Clone, Copy)]
pub(crate) struct Timer {
    timeout: u64, // タイムアウト時刻 (tick)
    value: i32,   // タイムアウト時に送信する値
}
impl Timer {
    pub fn new(timeout: u64, value: i32) -> Self {
        Self { timeout, value }
    }
    pub fn get_timeout(&self) -> u64 {
        self.timeout
    }
    pub fn get_value(&self) -> i32 {
        self.value
    }
}
impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.timeout == other.timeout
    }
}
impl Eq for Timer {}
impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Timer {
    // タイマーの優先度は timeout が小さい方が高い (早くタイムアウトする)
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.timeout.cmp(&other.timeout)
    }
}

pub(crate) struct TimerManager {
    tick: u64,
    timers: Vec<Timer>,
    msg_queue: &'static Global<Queue<Message>>,
}

impl TimerManager {
    pub fn new(msg_queue: &'static Global<Queue<Message>>) -> Self {
        Self {
            tick: 0,
            timers: alloc::vec![Timer::new(u64::MAX, 0)], // 番兵
            msg_queue,
        }
    }

    fn tick(&mut self) {
        self.tick += 1;

        while let Some(timer) = self.timers.first() {
            if timer.get_timeout() > self.tick {
                // まだタイムアウトしていない
                break;
            }

            // タイムアウトしたタイマーを削除してメッセージキューに送信する
            let message = Message::InterruptLapicTimer {
                timeout: timer.get_timeout(),
                value: timer.get_value(),
            };
            // TODO: Vecなので計算量改善 (DequeueとかBinaryHeapとか)
            let _ = self.msg_queue.with(|q| q.push(message));
            self.timers.remove(0);
        }
    }

    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    pub fn add_timer(&mut self, timer: &Timer) {
        self.timers.push(*timer);
        self.timers.sort();
    }
}
pub static TIMER_MANAGER: Global<Option<TimerManager>> = Global::new(None);
pub static LAPIC_TIMER_FREQ: Global<Option<u32>> = Global::new(None);
pub const TIMER_FREQ: u32 = 100; // 100Hz /s 1秒で100回tickが呼ばれる
