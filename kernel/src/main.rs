#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library

use core::arch::asm;
use core::panic::PanicInfo;

// must wrap the attribute 'no_mangle' by unsafe(...)
#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn KernelMain() -> ! {
    // entrypoint
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
