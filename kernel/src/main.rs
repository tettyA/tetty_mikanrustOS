#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library

use core::arch::asm;
use core::panic::PanicInfo;

// must wrap the attribute 'no_mangle' by unsafe(...)
#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn KernelMain(frame_buffer_base: u64, frame_buffer_size: u64) -> ! {
    // entrypoint
    let frame_buffer = frame_buffer_base as *mut u8;
    //uint8_t* frame_buffer = reinterpret_cast<uint8_t*>(frame_buffer_base);
    for i in 0..frame_buffer_size {
        unsafe {
            core::ptr::write_volatile(frame_buffer.add(i as usize), (i % 256) as u8);
        }
    }
    loop {
        unsafe { asm!("hlt") }
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("Hlt");
        }
    }
}
