use crate::global::Global;
use crate::printk;
use crate::{
    graphics::{PixelColor, PixelDraw, Vector2D},
    layer::LAYER_MANAGER,
};

pub const MOUSE_CURSOR_WIDTH: usize = 15;
pub const MOUSE_CURSOR_HEIGHT: usize = 24;
pub const MOUSE_CURSOR_TRANSPARENT_COLOR: PixelColor = PixelColor {
    r: 255,
    g: 0,
    b: 255,
};
const MOUSE_CURSOR_SHAPE: [&'static str; MOUSE_CURSOR_HEIGHT] = [
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
    drag_layer_id: u32,
    redraw: Option<fn()>,
    redraw_area: Option<fn(Vector2D<i32>, Vector2D<i32>)>,
    position: Vector2D<i32>,
    screen_size: Vector2D<usize>,
    previous_buttons: u8,
}

impl MouseCursor {
    pub const fn new() -> Self {
        Self {
            initialized: false,
            layer_id: 0,
            drag_layer_id: 0,
            redraw: None,
            redraw_area: None,
            position: Vector2D { x: 200, y: 300 },
            previous_buttons: 0,
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

    pub unsafe fn move_relative(&mut self, _buttons: u8, displacement: Vector2D<i32>) {
        if !self.initialized {
            return;
        }

        let old_position = self.position;
        self.position += displacement;

        self.position.x = self.position.x.clamp(0, self.screen_size.x as i32 - 1);
        self.position.y = self.position.y.clamp(0, self.screen_size.y as i32 - 1);

        let posdiff = [
            (self.position.x - old_position.x),
            (self.position.y - old_position.y),
        ];

        LAYER_MANAGER.with(|lm| {
            if let Some(lm) = lm {
                lm.move_layer(self.layer_id, self.position);
            }
        });

        let previous_left_pressed = (self.previous_buttons & 0x01) != 0;
        let left_pressed = (_buttons & 0x01) != 0;

        if !previous_left_pressed && left_pressed {
            // 左ボタンが押された瞬間
            let mut target_layer_id = 0;
            LAYER_MANAGER.with(|lm| {
                if let Some(lm) = lm {
                    if let Some(layer) = lm.find_layer_by_position(self.position, self.layer_id) {
                        if layer.is_draggable() {
                            target_layer_id = layer.id;
                        }
                    }
                }
            });
            if target_layer_id > 0 {
                self.drag_layer_id = target_layer_id;
                crate::layer::activate(target_layer_id);
                printk!(
                    "mouse_observer: layer_id = {}, pos = ({}, {})\n",
                    self.drag_layer_id,
                    self.position.x,
                    self.position.y
                );
            } else {
                crate::layer::activate(0);
            }
        } else if previous_left_pressed && left_pressed {
            // 左ボタンが押されている間
            if self.drag_layer_id > 0 {
                LAYER_MANAGER.with(|lm| {
                    if let Some(lm) = lm {
                        lm.move_layer_relative(
                            self.drag_layer_id,
                            Vector2D {
                                x: posdiff[0],
                                y: posdiff[1],
                            },
                        );
                    }
                });
            }
        } else if previous_left_pressed && !left_pressed {
            // 左ボタンが離された瞬間
            self.drag_layer_id = 0;
        }

        self.previous_buttons = _buttons;

        if let Some(redraw_area) = self.redraw_area {
            redraw_area(old_position, self.position);
        }
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

/// `usb::HIDMouseDriver` に登録するオブザーバ (本家 main.cpp の `MouseObserver` 相当)．
/// ボタン状態は現状使わず，変位だけをカーソル移動に反映する．
pub extern "C" fn mouse_observer(_buttons: u8, displacement_x: i8, displacement_y: i8) {
    MOUSE_CURSOR.with(|cursor| unsafe {
        cursor.move_relative(
            _buttons,
            Vector2D {
                x: displacement_x as i32,
                y: displacement_y as i32,
            },
        );
    });
}
