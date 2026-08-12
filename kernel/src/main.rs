#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library

mod console;
mod font;
mod frame_buffer_config;
mod graphics;
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
use core::fmt::Write;

use core::arch::asm;
use core::panic::PanicInfo;

unsafe extern "C" {
    fn usb_process_events();
    fn usb_enumarate();
    fn usb_set_mouse_obserever(observer: extern "C" fn(i8, i8));
}

extern "C" fn moverelative(displacement_x: i8, displacement_y: i8) {}

const DESKTOP_BGCOLOR: PixelColor = PixelColor {
    r: 120,
    g: 255,
    b: 125,
};
const DESKTOP_FGCOLOR: PixelColor = PixelColor { r: 1, g: 8, b: 17 };

const kMouseCursorWidth: usize = 15;
const kMouseCursorHeight: usize = 24;
const MOUSE_CURSOR_SHAPE: [&'static str; kMouseCursorHeight] = [
    "@              ",
    "@@             ",
    "@.@            ",
    "@..@           ",
    "@...@          ",
    "@....@         ",
    "@.....@        ",
    "@......@       ",
    "@.......@      ",
    "@........@     ",
    "@.........@    ",
    "@..........@   ",
    "@...........@  ",
    "@............@ ",
    "@......@@@@@@@@",
    "@......@       ",
    "@....@@.@      ",
    "@...@ @.@      ",
    "@..@   @.@     ",
    "@.@    @.@     ",
    "@@      @.@    ",
    "@       @.@    ",
    "         @.@   ",
    "         @@@   ",
];

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

    let frame_width: usize = frame_buffer_config.horizontal_resolution as usize;
    let frame_height: usize = frame_buffer_config.vertical_resolution as usize;

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
    for (dy, s) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, c) in s.chars().enumerate() {
            if c == '@' {
                pixel_writer.write(200 + dx, 100 + dy, &PixelColor { r: 0, g: 0, b: 0 });
            } else if c == '.' {
                pixel_writer.write(
                    200 + dx,
                    100 + dy,
                    &PixelColor {
                        r: 255,
                        g: 255,
                        b: 255,
                    },
                );
            }
        }
    }

    unsafe {
        usb_set_mouse_obserever(moverelative);

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
