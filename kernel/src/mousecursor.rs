use core::ptr::addr_of_mut;

use crate::{
    graphics::{PixelColor, PixelDraw, Vector2D},
    layer::LayerManager,
};
pub const kMouseCursorWidth: usize = 15;
pub const kMouseCursorHeight: usize = 24;
pub const MOUSE_CURSOR_TRANSPARENT_COLOR: PixelColor = PixelColor {
    r: 255,
    g: 0,
    b: 255,
};
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
    layer_manager: Option<*mut LayerManager>,
    layer_id: u32,
    redraw: Option<fn()>,
    position: Vector2D<i32>,
    screen_size: Vector2D<usize>,
}

impl MouseCursor {
    pub const fn new() -> Self {
        Self {
            layer_manager: None,
            layer_id: 0,
            redraw: None,
            position: Vector2D { x: 200, y: 300 },
            screen_size: Vector2D { x: 100, y: 100 },
        }
    }
    pub unsafe fn init(
        &mut self,
        layer_manager: *mut LayerManager,
        layer_id: u32,
        screen_size: Vector2D<usize>,
        redraw: fn(),
    ) {
        self.layer_manager = Some(layer_manager);
        self.layer_id = layer_id;
        self.screen_size = screen_size;
        self.redraw = Some(redraw);
    }

    pub unsafe fn move_relative(&mut self, displacement: Vector2D<i32>) {
        if self.layer_manager.is_none() {
            return;
        }

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
        if let Some(layer_manager) = self.layer_manager {
            unsafe {
                (*layer_manager).move_layer(self.layer_id, self.position);
            }
        }
        if let Some(redraw) = self.redraw {
            redraw();
        }
    }

    pub fn draw<W: PixelDraw>(&self, writer: &W) {
        for (dy, s) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
            for (dx, c) in s.chars().enumerate() {
                if c == '@' {
                    writer.write(
                        dx,
                        dy,
                        &PixelColor { r: 0, g: 0, b: 0 },
                    );
                } else if c == '.' {
                    writer.write(
                        dx,
                        dy,
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
