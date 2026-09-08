#![allow(non_snake_case, dead_code, unsafe_op_in_unsafe_fn)]

use alloc::alloc::{Layout, alloc_zeroed, dealloc};
use core::mem::{align_of, size_of};
use core::ptr::{self, null_mut};

#[repr(C)]
struct AllocationHeader {
    base: *mut u8,
    layout_size: usize,
    layout_align: usize,
}

const HEADER_SIZE: usize = size_of::<AllocationHeader>();

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    value.checked_add(alignment - 1).map(|value| value & !(alignment - 1))
}

pub unsafe fn AllocMem(size: usize, alignment: usize, boundary: usize) -> *mut u8 {
    if size == 0 || !alignment.is_power_of_two() || alignment < align_of::<AllocationHeader>() { return null_mut(); }
    let extra = match HEADER_SIZE.checked_add(alignment) { Some(value) => value, None => return null_mut() };
    let layout_size = match size.checked_add(extra) { Some(value) => value, None => return null_mut() };
    let layout = match Layout::from_size_align(layout_size, alignment) { Ok(value) => value, Err(_) => return null_mut() };
    let base = alloc_zeroed(layout);
    if base.is_null() { return null_mut(); }

    let mut result = match align_up(base as usize + HEADER_SIZE, alignment) { Some(value) => value, None => { dealloc(base, layout); return null_mut(); } };
    if boundary != 0 {
        if !boundary.is_power_of_two() || size > boundary { dealloc(base, layout); return null_mut(); }
        while result / boundary != (result + size - 1) / boundary {
            result = match align_up((result / boundary + 1) * boundary, alignment) { Some(value) => value, None => { dealloc(base, layout); return null_mut(); } };
            if result + size > base as usize + layout_size { dealloc(base, layout); return null_mut(); }
        }
    }
    let header = (result - HEADER_SIZE) as *mut AllocationHeader;
    ptr::write(header, AllocationHeader { base, layout_size, layout_align: alignment });
    result as *mut u8
}

pub unsafe fn FreeMem(pointer: *mut u8) {
    if pointer.is_null() { return; }
    let header = (pointer as usize - HEADER_SIZE) as *mut AllocationHeader;
    let metadata = ptr::read(header);
    if let Ok(layout) = Layout::from_size_align(metadata.layout_size, metadata.layout_align) {
        dealloc(metadata.base, layout);
    }
}

pub unsafe fn AllocArray<T>(count: usize, alignment: usize, boundary: usize) -> *mut T {
    let size = match size_of::<T>().checked_mul(count) { Some(value) => value, None => return null_mut() };
    AllocMem(size, alignment.max(align_of::<T>()), boundary) as *mut T
}