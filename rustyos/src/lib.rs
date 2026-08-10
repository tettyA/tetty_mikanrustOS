use core::arch::asm;

#[no_mangle]
pub extern "C" fn KernelMain() -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
