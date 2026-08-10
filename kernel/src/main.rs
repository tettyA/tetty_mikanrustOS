#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library

mod frame_buffer_config;
use crate::frame_buffer_config::FrameBufferConfig;
use crate::frame_buffer_config::PixelFormat::{
    kPixelBGRResv8BitPerColor, kPixelRGBResv8BitPerColor,
};

use core::arch::asm;
use core::panic::PanicInfo;
use core::ptr;

pub trait PixelWriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor);
}

pub struct Bgrpixelwriter {
    frame_buffer: *mut u8,
    pixels_per_scan_line: usize,
}
impl Bgrpixelwriter {
    fn new(config: &FrameBufferConfig) -> Self {
        Self {
            frame_buffer: config.frame_buffer,
            pixels_per_scan_line: config.pixels_per_scan_line as usize,
        }
    }
}
impl PixelWriter for Bgrpixelwriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        let pixel_posititon = (self.pixels_per_scan_line as usize * y) + x;
        let offset = 4 * pixel_posititon as usize;
        unsafe {
            ptr::write_volatile(self.frame_buffer.add(offset), c.b);
            ptr::write_volatile(self.frame_buffer.add(offset + 1), c.g);
            ptr::write_volatile(self.frame_buffer.add(offset + 2), c.r);
        }
    }
}
pub struct Rgbpixelwriter {
    frame_buffer: *mut u8,
    pixels_per_scan_line: usize,
}
impl Rgbpixelwriter {
    fn new(config: &FrameBufferConfig) -> Self {
        Self {
            frame_buffer: config.frame_buffer,
            pixels_per_scan_line: config.pixels_per_scan_line as usize,
        }
    }
}
impl PixelWriter for Rgbpixelwriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        let pixel_posititon = (self.pixels_per_scan_line as usize * y) + x;
        let offset = 4 * pixel_posititon as usize;
        unsafe {
            ptr::write_volatile(self.frame_buffer.add(offset), c.r);
            ptr::write_volatile(self.frame_buffer.add(offset + 1), c.g);
            ptr::write_volatile(self.frame_buffer.add(offset + 2), c.b);
        }
    }
}
pub struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

// must wrap the attribute 'no_mangle' by unsafe(...)
#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn KernelMain(frame_buffer_config: &FrameBufferConfig) -> ! {
    let bgr = Bgrpixelwriter::new(frame_buffer_config);
    let rgb = Rgbpixelwriter::new(frame_buffer_config);
    let pixel_writer: &dyn PixelWriter = {
        match frame_buffer_config.pixel_format {
            kPixelBGRResv8BitPerColor => &bgr,
            kPixelRGBResv8BitPerColor => &rgb,
        }
    };
    // entrypoint
    for x in 0..frame_buffer_config.horizontal_resolution {
        for y in 0..frame_buffer_config.vertical_resolution {
            pixel_writer.write(
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
            pixel_writer.write(
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
