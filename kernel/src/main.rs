#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library

mod console;
mod font;
mod frame_buffer_config;
mod graphics;
use crate::console::Console;
use crate::font::{StringWriter, write_ascii, write_string};
use crate::frame_buffer_config::FrameBufferConfig;
use crate::frame_buffer_config::PixelFormat::{
    kPixelBGRResv8BitPerColor, kPixelRGBResv8BitPerColor,
};
use crate::graphics::{Bgrpixelwriter, PixelColor, PixelWriter, Rgbpixelwriter};
use core::fmt::Write;

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
    {
        let mut i = 0;
        for c in '!'..'~' {
            write_ascii(
                pixel_writer,
                8 * i,
                10,
                c as u8,
                &PixelColor { r: 0, g: 0, b: 0 },
            );
            i += 1;
        }
    }
    write_string(
        pixel_writer,
        0,
        30,
        "Hello ##tettyA## Syste/\\/\\  World!",
        &PixelColor {
            r: 150,
            g: 150,
            b: 0,
        },
    );

    write_stringf!(
        pixel_writer,
        0,
        170,
        &PixelColor { r: 10, g: 0, b: 10 },
        "5 + 5 ={}",
        5 + 5
    );
    write_stringf!(pixel_writer,0,180,R:10,G:50,B:0,"RGB 10+10={}",10+10);

    let mut console = Console::new(
        pixel_writer,
        PixelColor { r: 0, g: 0, b: 0 },
        PixelColor {
            r: 120,
            g: 120,
            b: 120,
        },
    );
    for i in 0..28 {
        write!(console, "Line nunmber:{}\n", i + 1);
    }
    console.put_string("Last Line (^^)");
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
