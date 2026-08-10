#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library

mod frame_buffer_config;
use crate::frame_buffer_config::PixelFormat::{
    kPixelBGRResv8BitPerColor, kPixelRGBResv8BitPerColor,
};
use crate::frame_buffer_config::{FrameBufferConfig, PixelFormat};

use core::arch::asm;
use core::ops::Add;
use core::panic::PanicInfo;
use core::ptr;

struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

fn WritePixel(config: &FrameBufferConfig, x: usize, y: usize, c: &PixelColor) -> i32 {
    let pixel_posititon = (config.pixels_per_scan_line as usize * y) + x;
    let offset = 4 * pixel_posititon as usize;
    if config.pixel_format == kPixelBGRResv8BitPerColor {
        unsafe {
            ptr::write_volatile(config.frame_buffer.add(offset), c.b);
            ptr::write_volatile(config.frame_buffer.add(offset + 1), c.g);
            ptr::write_volatile(config.frame_buffer.add(offset + 2), c.r);
        }
    } else if config.pixel_format == kPixelRGBResv8BitPerColor {
        unsafe {
            ptr::write_volatile(config.frame_buffer.add(offset), c.r);
            ptr::write_volatile(config.frame_buffer.add(offset + 1), c.g);
            ptr::write_volatile(config.frame_buffer.add(offset + 2), c.b);
        }
    } else {
        return -1;
    }
    return 0;
}

// must wrap the attribute 'no_mangle' by unsafe(...)
#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn KernelMain(frame_buffer_config: &FrameBufferConfig) -> ! {
    // entrypoint
    for x in 0..frame_buffer_config.horizontal_resolution {
        for y in 0..frame_buffer_config.vertical_resolution {
            WritePixel(
                frame_buffer_config,
                x as usize,
                y as usize,
                &PixelColor {
                    r: 255,
                    g: 255,
                    b: 255,
                },
            );
        }
    }

    for x in 0..100 {
        for y in 0..100 {
            WritePixel(
                frame_buffer_config,
                x as usize + 100,
                y as usize + 100,
                &PixelColor {
                    r: ((x as i32) % 256) as u8,
                    g: 125,
                    b: ((y as i32) % 256) as u8,
                },
            );
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
