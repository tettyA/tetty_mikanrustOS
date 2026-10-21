use alloc::rc::Rc;
use core::cell::RefCell;

use crate::{
    font::write_ascii,
    graphics::{Rectangle, Vector2D, fill_rectrange, to_color},
    layer::{LAYER_MANAGER, LayerManager},
    message::{LayerOperation, Message},
    printk, redraw_layers_id,
    task::{self, receive_message},
    window::{MARGIN_X, MARGIN_Y, TOP_LEFT_MARGIN, Window, draw_terminal},
};

pub struct Terminal {
    window: Rc<RefCell<Window>>,
    layer_id: u32,
    cursor: Vector2D<usize>,
    cursor_visible: bool,
    line_buf: [u8; Self::LINE_MAX],
    linebuf_index: usize,
}

impl Terminal {
    pub const ROWS: usize = 15;
    pub const COLUMNS: usize = 60;
    pub const LINE_MAX: usize = 128;

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
        let mut term = Self {
            window,
            layer_id,
            cursor: Vector2D { x: 0, y: 0 },
            cursor_visible: false,
            line_buf: [0; Self::LINE_MAX],
            linebuf_index: 0,
        };
        term.print(">");

        term
    }

    pub fn layer_id(&self) -> u32 {
        self.layer_id
    }

    fn calc_cursor_pos(&self) -> Vector2D<usize> {
        Vector2D {
            x: TOP_LEFT_MARGIN.x + 4 + 8 * self.cursor.x,
            y: TOP_LEFT_MARGIN.y + 4 + 16 * self.cursor.y,
        }
    }

    pub fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.cursor_visible = !self.cursor_visible;

        self.draw_cursor(self.cursor_visible);

        Rectangle {
            pos_lu: Vector2D {
                x: self.calc_cursor_pos().x as i32,
                y: self.calc_cursor_pos().y as i32,
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

        let pos = self.calc_cursor_pos();

        crate::graphics::fill_rectrange(
            &self.window.borrow_mut().writer(),
            &pos,
            &Vector2D { x: 7, y: 15 },
            &color,
        );
    }

    fn scroll1(&mut self) {
        let move_src = Rectangle {
            pos_lu: Vector2D {
                x: (TOP_LEFT_MARGIN.x + 4) as i32,
                y: (TOP_LEFT_MARGIN.y + 4 + 16) as i32,
            },
            size: Vector2D {
                x: (8 * Self::COLUMNS) as i32,
                y: (16 * (Self::ROWS - 1)) as i32,
            },
        };

        self.window.borrow_mut().window_move(
            Vector2D {
                x: (TOP_LEFT_MARGIN.x + 4) as i32,
                y: (TOP_LEFT_MARGIN.y + 4) as i32,
            },
            &move_src,
        );

        fill_rectrange(
            &self.window.borrow_mut().inner_writer(),
            &Vector2D {
                x: 4,
                y: 4 + 16 * self.cursor.y,
            },
            &Vector2D {
                x: 8 * Self::COLUMNS,
                y: 16,
            },
            &to_color(0),
        );
    }

    fn newline(&mut self) {
        self.cursor.x = 0;
        if self.cursor.y < Self::ROWS - 1 {
            self.cursor.y += 1;
        } else {
            self.scroll1();
        }
    }

    pub fn print(&mut self, s: &str) {
        self.draw_cursor(false);

        for c in s.chars() {
            if c == '\n' {
                self.newline();
            } else {
                let pos = self.calc_cursor_pos();
                write_ascii(
                    &self.window.borrow_mut().writer(),
                    pos.x,
                    pos.y,
                    c as u8,
                    &to_color(0xffffff),
                );

                if self.cursor.x == Self::COLUMNS - 1 {
                    self.newline();
                } else {
                    self.cursor.x += 1;
                }
            }
        }

        self.draw_cursor(true);
    }

    fn execute_line(&mut self) {
        let line_buf = self.line_buf;
        let linebuf_index = self.linebuf_index;
        self.linebuf_index = 0;
        let line = core::str::from_utf8(&line_buf[..linebuf_index]).unwrap_or("");
        let (command, first_arg) = match line.split_once(' ') {
            Some((cmd, arg)) => (cmd, Some(arg)),
            None => (line, None),
        };
        if command == "echo" {
            if let Some(arg) = first_arg {
                self.print(arg);
            }
            self.print("\n");
        } else if !command.is_empty() {
            self.print("no such command: ");
            self.print(command);
            self.print("\n");
        }
    }

    pub fn input_key(&mut self, _modifier: u8, _keycode: u8, ascii: char) -> Rectangle<i32> {
        self.draw_cursor(false);

        let mut draw_area = Rectangle {
            pos_lu: Vector2D {
                x: self.calc_cursor_pos().x as i32,
                y: self.calc_cursor_pos().y as i32,
            },
            size: Vector2D { x: 8 * 2, y: 16 },
        };

        if ascii == '\n' {
            self.newline();
            self.execute_line();
            self.print(">");
            draw_area.pos_lu = Vector2D {
                x: TOP_LEFT_MARGIN.x as i32,
                y: TOP_LEFT_MARGIN.y as i32,
            };
            let inner_size = self.window.borrow().inner_size();
            draw_area.size = Vector2D {
                x: inner_size.x as i32,
                y: inner_size.y as i32,
            };
        } else if ascii == '\x08' {
            // Backspace
            if self.cursor.x > 0 {
                self.cursor.x -= 1;
                fill_rectrange(
                    &self.window.borrow_mut().writer(),
                    &self.calc_cursor_pos(),
                    &Vector2D { x: 8, y: 16 },
                    &to_color(0),
                );
                draw_area.pos_lu = Vector2D {
                    x: self.calc_cursor_pos().x as i32,
                    y: self.calc_cursor_pos().y as i32,
                };
                if self.linebuf_index > 0 {
                    self.linebuf_index -= 1;
                }
            }
        } else if ascii != '\0' {
            if self.cursor.x < Self::COLUMNS - 1 && self.linebuf_index < Self::LINE_MAX - 1 {
                self.line_buf[self.linebuf_index] = ascii as u8;
                self.linebuf_index += 1;
                let pos = self.calc_cursor_pos();
                write_ascii(
                    &self.window.borrow_mut().writer(),
                    pos.x,
                    pos.y,
                    ascii as u8,
                    &to_color(0xffffff),
                );
                self.cursor.x += 1;
            }
        }
        self.draw_cursor(true);
        draw_area
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

    crate::layer::register_layer_task(layer_id, task_id);

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
                let area = x86_64::instructions::interrupts::without_interrupts(|| {
                    terminal.blink_cursor()
                });

                let _ = task::send_message(
                    1,
                    Message::Layer {
                        op: LayerOperation::DrawArea { area },
                        layer_id,
                        src_task: task_id,
                    },
                );
            }
            Message::KeyPush {
                keycode,
                modifier,
                ascii,
            } => {
                let area = x86_64::instructions::interrupts::without_interrupts(|| {
                    terminal.input_key(modifier, keycode, ascii)
                });
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
