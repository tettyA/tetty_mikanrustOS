use core::ptr::addr_of_mut;

use crate::{
    DESKTOP_BGCOLOR,
    graphics::{PixelColor, PixelWriter, Vector2D},
};
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

pub struct MouseCursor {
    pixel_writer: Option<*const PixelWriter>,
    position: Vector2D<i32>,
    screen_size: Vector2D<usize>,
    erase_color: PixelColor,
}

impl MouseCursor {
    pub const fn new() -> Self {
        Self {
            pixel_writer: None,
            position: Vector2D { x: 200, y: 300 },
            screen_size: Vector2D { x: 100, y: 100 },
            erase_color: DESKTOP_BGCOLOR,
        }
    }
    pub unsafe fn init(&mut self, writer: *const PixelWriter, screen_size: Vector2D<usize>) {
        self.pixel_writer = Some(writer);
        self.screen_size = screen_size;
    }

    pub unsafe fn move_relative(&mut self, displacement: Vector2D<i32>) {
        if self.pixel_writer.is_none() {
            return;
        }

        self.erase();
        self.position += displacement;
        if self.position.x < 0 {
            self.position.x = 0;
        }
        if self.position.y < 0 {
            self.position.y = 0;
        }
        if self.position.x >= self.screen_size.x as i32 {
            self.position.x = self.screen_size.x as i32 - 1;
        }
        if self.position.y >= self.screen_size.y as i32 {
            self.position.y = self.screen_size.y as i32 - 1;
        }
        self.draw();
    }

    fn draw(&self) {
        if let Some(writer_ptr) = self.pixel_writer {
            let writer = unsafe { &*writer_ptr };
            for (dy, s) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
                for (dx, c) in s.chars().enumerate() {
                    if c == '@' {
                        writer.write(
                            self.position.x as usize + dx,
                            self.position.y as usize + dy,
                            &PixelColor { r: 0, g: 0, b: 0 },
                        );
                    } else if c == '.' {
                        writer.write(
                            self.position.x as usize + dx,
                            self.position.y as usize + dy,
                            &PixelColor {
                                r: 255,
                                g: 255,
                                b: 255,
                            },
                        );
                    }
                }
            }
        }
    }
    fn erase(&self) {
        if let Some(writer_ptr) = self.pixel_writer {
            let writer = unsafe { &*writer_ptr };
            for dy in 0..kMouseCursorHeight as i32 {
                for dx in 0..kMouseCursorWidth as i32 {
                    writer.write(
                        (self.position.x + dx) as usize,
                        (self.position.y + dy) as usize,
                        &self.erase_color,
                    );
                }
            }
        }
    }
}
pub static mut MOUSE_CURSOR: MouseCursor = MouseCursor::new();

pub extern "C" fn moverelative(displacement_x: i8, displacement_y: i8) {
    unsafe {
        let cursor = &mut *addr_of_mut!(MOUSE_CURSOR);

        cursor.move_relative(Vector2D {
            x: displacement_x as i32,
            y: displacement_y as i32,
        });
    }
}
