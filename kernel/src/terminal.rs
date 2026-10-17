use alloc::rc::Rc;
use core::cell::RefCell;

use crate::{
    graphics::{Rectangle, Vector2D, to_color},
    layer::{LAYER_MANAGER, LayerManager},
    message::{LayerOperation, Message},
    redraw_layers_id,
    task::{self, receive_message},
    window::{MARGIN_X, MARGIN_Y, TOP_LEFT_MARGIN, Window, draw_terminal},
};

pub struct Terminal {
    window: Rc<RefCell<Window>>,
    layer_id: u32,
    cursor: Vector2D<usize>,
    cursor_visible: bool,
}

impl Terminal {
    pub const ROWS: usize = 15;
    pub const COLUMNS: usize = 60;

    pub fn new() -> Self {
        let (layer_id, window) = LAYER_MANAGER.with(|lm| {
            let lm = lm.as_mut().unwrap();
            let pixel_format = lm.pixel_format();
            let width = Self::COLUMNS * 8 + 8 + MARGIN_X;
            let height = Self::ROWS * 16 + 8 + MARGIN_Y;
            let window = Rc::new(RefCell::new(Window::new_toplevel(
                width,
                height,
                pixel_format,
                "tettyATerm",
            )));
            let inner_size = window.borrow().inner_size();

            draw_terminal(
                &window.borrow_mut().inner_writer(),
                Vector2D { x: 0, y: 0 },
                inner_size,
            );

            let layer_id = lm
                .new_layer()
                .set_window(window.clone())
                .set_draggable(true)
                .id;

            (layer_id, window)
        });

        Self {
            window,
            layer_id,
            cursor: Vector2D { x: 0, y: 0 },
            cursor_visible: false,
        }
    }

    pub fn layer_id(&self) -> u32 {
        self.layer_id
    }

    pub fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.cursor_visible = !self.cursor_visible;

        self.draw_cursor(self.cursor_visible);

        Rectangle {
            pos_lu: Vector2D {
                x: TOP_LEFT_MARGIN.x as i32 + 4 + 8 * self.cursor.x as i32,
                y: TOP_LEFT_MARGIN.y as i32 + 5 + 16 * self.cursor.y as i32,
            },
            size: Vector2D { x: 7, y: 15 },
        }
    }

    fn draw_cursor(&mut self, visible: bool) {
        let color = if visible {
            to_color(0xffffff)
        } else {
            to_color(0x000000)
        };

        let pos = Vector2D {
            x: 4 + 8 * self.cursor.x,
            y: 5 + 16 * self.cursor.y,
        };

        crate::graphics::fill_rectrange(
            &self.window.borrow_mut().inner_writer(),
            &pos,
            &Vector2D { x: 7, y: 15 },
            &color,
        );
    }
}

pub extern "C" fn task_terminal(task_id: u64, _data: i64, _arg3: u64, _arg4: u64) {
    let mut terminal = Terminal::new();
    let layer_id = terminal.layer_id();

    LAYER_MANAGER.with(|lm| {
        let Some(lm) = lm.as_mut() else { return };
        lm.move_layer(layer_id, Vector2D { x: 100, y: 200 });
        lm.activate(layer_id);
    });

    loop {
        x86_64::instructions::interrupts::disable();
        let Some(msg) = receive_message(task_id) else {
            let _ = task::sleep(task_id);
            x86_64::instructions::interrupts::enable();
            continue;
        };

        x86_64::instructions::interrupts::enable();

        match msg {
            Message::TimerTimeout { .. } => {
                let area = terminal.blink_cursor();

                let _ = task::send_message(
                    1,
                    Message::Layer {
                        op: LayerOperation::DrawArea { area },
                        layer_id,
                        src_task: task_id,
                    },
                );
            }
            _ => {}
        }
    }
}
