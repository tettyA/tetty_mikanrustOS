use crate::global::Global;

use crate::{
    graphics::{PixelColor, PixelDraw, Vector2D},
    layer::LAYER_MANAGER, timer,
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
    initialized: bool,
    layer_id: u32,
    redraw: Option<fn()>,
    redraw_area: Option<fn(Vector2D<i32>, Vector2D<i32>)>,
    position: Vector2D<i32>,
    screen_size: Vector2D<usize>,
}

impl MouseCursor {
    pub const fn new() -> Self {
        Self {
            initialized: false,
            layer_id: 0,
            redraw: None,
            redraw_area: None,
            position: Vector2D { x: 200, y: 300 },
            screen_size: Vector2D { x: 100, y: 100 },
        }
    }
    pub unsafe fn init(
        &mut self,
        layer_id: u32,
        screen_size: Vector2D<usize>,
        redraw: fn(),
        redraw_area: fn(Vector2D<i32>, Vector2D<i32>),
    ) {
        self.initialized = true;
        self.layer_id = layer_id;
        self.screen_size = screen_size;
        self.redraw = Some(redraw);
        self.redraw_area = Some(redraw_area);
    }

    pub unsafe fn move_relative(&mut self, displacement: Vector2D<i32>) {
        if !self.initialized {
            return;
        }

        let old_position = self.position;
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

        timer::start_lapic_timer();
        LAYER_MANAGER.with(|lm| {
            if let Some(lm) = lm {
                lm.move_layer(self.layer_id, self.position);
            }
        });
        if let Some(redraw_area) = self.redraw_area {
            redraw_area(old_position, self.position);
        }

        /*  let elapsed = timer::lapic_timer_elapsed();
        timer::start_lapic_timer();
        printk!("MouseObserver: elapsed = {}\n", elapsed);*/
    }

    pub fn draw<W: PixelDraw>(&self, writer: &W) {
        for (dy, s) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
            for (dx, c) in s.chars().enumerate() {
                if c == '@' {
                    writer.write(dx, dy, &PixelColor { r: 0, g: 0, b: 0 });
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
pub static MOUSE_CURSOR: Global<MouseCursor> = Global::new(MouseCursor::new());
pub extern "C" fn moverelative(displacement_x: i8, displacement_y: i8) {
    MOUSE_CURSOR.with(|cursor| unsafe {
        cursor.move_relative(Vector2D {
            x: displacement_x as i32,
            y: displacement_y as i32,
        });
    });
}

/// `usb::HIDMouseDriver` に登録するオブザーバ (本家 main.cpp の `MouseObserver` 相当)．
/// ボタン状態は現状使わず，変位だけをカーソル移動に反映する．
pub extern "C" fn MouseObserver(_buttons: u8, displacement_x: i8, displacement_y: i8) {
    moverelative(displacement_x, displacement_y);
}
