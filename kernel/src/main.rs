#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library

mod font;
mod frame_buffer_config;
mod graphics;
use crate::font::write_ascii;
use crate::frame_buffer_config::FrameBufferConfig;
use crate::frame_buffer_config::PixelFormat::{
    kPixelBGRResv8BitPerColor, kPixelRGBResv8BitPerColor,
};
use crate::graphics::{Bgrpixelwriter, PixelColor, PixelWriter, Rgbpixelwriter};

use core::arch::asm;
use core::panic::PanicInfo;

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

    write_ascii(
        pixel_writer,
        40,
        50,
        't',
        &PixelColor {
            r: 25,
            g: 205,
            b: 150,
        },
    );
    write_ascii(
        pixel_writer,
        50,
        50,
        'e',
        &PixelColor {
            r: 250,
            g: 205,
            b: 50,
        },
    );
    write_ascii(
        pixel_writer,
        60,
        50,
        't',
        &PixelColor {
            r: 20,
            g: 205,
            b: 50,
        },
    );

    write_ascii(
        pixel_writer,
        70,
        50,
        't',
        &PixelColor {
            r: 145,
            g: 25,
            b: 50,
        },
    );
    write_ascii(
        pixel_writer,
        80,
        50,
        'y',
        &PixelColor {
            r: 145,
            g: 150,
            b: 0,
        },
    );
    write_ascii(
        pixel_writer,
        90,
        50,
        'A',
        &PixelColor { r: 152, g: 0, b: 5 },
    );
    write_ascii(
        pixel_writer,
        100,
        50,
        '@',
        &PixelColor { r: 2, g: 0, b: 255 },
    );
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
