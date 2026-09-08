#![allow(unsafe_op_in_unsafe_fn, dead_code)]

use core::ptr::null_mut;

use super::trb::{LinkTRB, TRB};
use crate::usb::memory::{AllocArray, FreeMem};

#[repr(C, packed)]
pub struct EventRingSegmentTableEntry { pub data: [u32; 4] }

pub struct Ring {
    buf_: *mut TRB,
    buf_size_: usize,
    cycle_bit_: bool,
    write_index_: usize,
}

pub struct EventRing {
    buf_: *mut TRB,
    erst_: *mut EventRingSegmentTableEntry,
    interrupter_: *mut super::registers::InterrupterRegisterSet,
    buf_size_: usize,
    dequeue_index_: usize,
    cycle_bit_: bool,
}

impl EventRing {
    pub const fn new() -> Self { Self { buf_: null_mut(), erst_: null_mut(), interrupter_: null_mut(), buf_size_: 0, dequeue_index_: 0, cycle_bit_: true } }

    pub unsafe fn Initialize(&mut self, buf_size: usize, interrupter: *mut super::registers::InterrupterRegisterSet) -> bool {
        if buf_size == 0 { return false; }
        self.buf_ = AllocArray::<TRB>(buf_size, 64, 64 * 1024);
        self.erst_ = AllocArray::<EventRingSegmentTableEntry>(1, 64, 64 * 1024);
        if self.buf_.is_null() || self.erst_.is_null() { return false; }
        for index in 0..buf_size { self.buf_.add(index).write(TRB::new()); }
        (*self.erst_).data = [0; 4];
        (*self.erst_).data[0] = self.buf_ as u64 as u32;
        (*self.erst_).data[1] = (self.buf_ as u64 >> 32) as u32;
        (*self.erst_).data[2] = buf_size as u32;
        self.interrupter_ = interrupter;
        let mut size = (*interrupter).ERSTSZ.Read(); size.SetSize(1); (*interrupter).ERSTSZ.Write(size);
        let mut base = (*interrupter).ERSTBA.Read(); base.SetPointer(self.erst_ as u64); (*interrupter).ERSTBA.Write(base);
        let mut dequeue = (*interrupter).ERDP.Read(); dequeue.SetPointer(self.buf_ as u64); (*interrupter).ERDP.Write(dequeue);
        self.buf_size_ = buf_size;
        self.dequeue_index_ = 0;
        self.cycle_bit_ = true;
        true
    }

    pub fn ReadDequeuePointer(&self) -> Option<&TRB> { if self.buf_.is_null() { None } else { unsafe { Some(&*self.buf_.add(self.dequeue_index_)) } } }
    pub fn Front(&self) -> Option<&TRB> { self.ReadDequeuePointer() }
    pub fn HasFront(&self) -> bool { self.Front().is_some_and(|trb| trb.cycle_bit() == self.cycle_bit_) }
    pub fn Pop(&mut self) {
        if self.buf_.is_null() { return; }
        self.dequeue_index_ += 1;
        if self.dequeue_index_ == self.buf_size_ { self.dequeue_index_ = 0; self.cycle_bit_ = !self.cycle_bit_; }
        unsafe {
            if !self.interrupter_.is_null() {
                let mut dequeue = (*self.interrupter_).ERDP.Read();
                dequeue.SetPointer(self.buf_.add(self.dequeue_index_) as u64);
                (*self.interrupter_).ERDP.Write(dequeue);
            }
        }
    }
}

impl Default for EventRing { fn default() -> Self { Self::new() } }

impl Ring {
    pub const fn new() -> Self { Self { buf_: null_mut(), buf_size_: 0, cycle_bit_: true, write_index_: 0 } }

    pub unsafe fn Initialize(&mut self, buf_size: usize) -> bool {
        if buf_size < 2 { return false; }
        self.buf_ = AllocArray::<TRB>(buf_size, 64, 64 * 1024);
        if self.buf_.is_null() { return false; }
        for index in 0..buf_size { self.buf_.add(index).write(TRB::new()); }
        self.buf_size_ = buf_size;
        self.cycle_bit_ = true;
        self.write_index_ = 0;
        true
    }

    pub fn Buffer(&self) -> *mut TRB { self.buf_ }

    pub fn CopyToLast(&mut self, data: [u32; 4]) {
        unsafe { for index in 0..3 { (*self.buf_.add(self.write_index_)).data[index] = data[index]; } (*self.buf_.add(self.write_index_)).data[3] = (data[3] & !1) | self.cycle_bit_ as u32; }
    }

    pub fn Push(&mut self, data: [u32; 4]) -> Option<&TRB> {
        let index = self.write_index_;
        self.CopyToLast(data);
        self.write_index_ += 1;
        if self.write_index_ == self.buf_size_ - 1 {
            let pointer = self.buf_;
            let mut link = LinkTRB::new(pointer);
            link.set_toggle_cycle(true);
            self.CopyToLast(link.data);
            self.write_index_ = 0;
            self.cycle_bit_ = !self.cycle_bit_;
        }
        if self.buf_.is_null() { None } else { unsafe { Some(&*self.buf_.add(index)) } }
    }
}

impl Default for Ring { fn default() -> Self { Self::new() } }
impl Drop for Ring { fn drop(&mut self) { unsafe { if !self.buf_.is_null() { FreeMem(self.buf_ as *mut u8); } } } }
impl Drop for EventRing { fn drop(&mut self) { unsafe { if !self.buf_.is_null() { FreeMem(self.buf_ as *mut u8); } if !self.erst_.is_null() { FreeMem(self.erst_ as *mut u8); } } } }