#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library
#![feature(abi_x86_interrupt)] // Allows extern "x86-interrupt" fn(). 
mod console;
mod font;
mod frame_buffer_config;
mod graphics;
mod interrupts;
mod mousecursor;
use crate::console::Console;
use crate::font::{write_ascii, write_string};
use crate::frame_buffer_config::FrameBufferConfig;
use crate::frame_buffer_config::PixelFormat::{
    kPixelBGRResv8BitPerColor, kPixelRGBResv8BitPerColor,
};
use crate::graphics::{
    Bgrpixelwriter, PixelColor, PixelWriter, Rgbpixelwriter, Vector2D, draw_rectrange,
    fill_rectrange,
};
use crate::interrupts::init_idt;
use crate::mousecursor::MOUSE_CURSOR;
use core::fmt::Write;

use core::arch::asm;
use core::panic::PanicInfo;
use core::ptr::addr_of_mut;

unsafe extern "C" {
    fn usb_process_events();
    fn usb_enumarate();
    fn usb_set_mouse_obserever(observer: extern "C" fn(i8, i8));
}

const DESKTOP_BGCOLOR: PixelColor = PixelColor {
    r: 120,
    g: 255,
    b: 125,
};
const DESKTOP_FGCOLOR: PixelColor = PixelColor { r: 1, g: 8, b: 17 };

// must wrap the attribute 'no_mangle' by unsafe(...)
#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn KernelMain(frame_buffer_config: &FrameBufferConfig) -> ! {
    let pixel_writer: &PixelWriter = {
        match frame_buffer_config.pixel_format {
            kPixelRGBResv8BitPerColor => {
                &PixelWriter::Rgb(Rgbpixelwriter::new(frame_buffer_config))
            }
            kPixelBGRResv8BitPerColor => {
                &PixelWriter::Bgr(Bgrpixelwriter::new(frame_buffer_config))
            }
        }
    };
    init_idt();

    let frame_width: usize = frame_buffer_config.horizontal_resolution as usize;
    let frame_height: usize = frame_buffer_config.vertical_resolution as usize;

    unsafe {
        let cursor = &mut *addr_of_mut!(MOUSE_CURSOR);
        cursor.init(
            pixel_writer,
            Vector2D {
                x: frame_width,
                y: frame_height,
            },
        );
    }

    fill_rectrange(
        pixel_writer,
        &Vector2D { x: 0, y: 0 },
        &Vector2D {
            x: frame_width,
            y: frame_height - 50,
        },
        &DESKTOP_BGCOLOR,
    );
    fill_rectrange(
        pixel_writer,
        &Vector2D {
            x: 0,
            y: frame_height - 50,
        },
        &Vector2D {
            x: frame_width,
            y: 50,
        },
        &DESKTOP_FGCOLOR,
    );

    fill_rectrange(
        pixel_writer,
        &Vector2D {
            x: 0,
            y: frame_height - 50,
        },
        &Vector2D {
            x: frame_width / 5,
            y: 50,
        },
        &PixelColor {
            r: 80,
            g: 80,
            b: 80,
        },
    );

    draw_rectrange(
        pixel_writer,
        &Vector2D {
            x: 10,
            y: frame_height - 40,
        },
        &Vector2D { x: 30, y: 30 },
        &PixelColor {
            r: 160,
            g: 160,
            b: 160,
        },
    );
    let mut console = Console::new(pixel_writer, DESKTOP_FGCOLOR, DESKTOP_BGCOLOR);
    console.write_str("Welcome to tettya OS!!\n");
    // entrypoint

    unsafe {
        usb_set_mouse_obserever(crate::mousecursor::moverelative);

        usb_enumarate();
    }
    console.write_str("Success ScanAllBus");

    loop {
        unsafe {
            usb_process_events();
        }
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
