use crate::font::{write_ascii, write_string};
use crate::graphics::{PixelColor, PixelDraw, PixelWriter, Vector2D};
use crate::window::Window;
use alloc::boxed::Box;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::fmt::{self, Arguments, Write};

static mut CONSOLE: *mut Console<'static> = core::ptr::null_mut();

/// Initialize the kernel-wide console. Calls before initialization are discarded.
pub fn init(console: Console<'static>) {
    unsafe {
        CONSOLE = Box::into_raw(Box::new(console));
    }
}

/// Write formatted text from normal kernel context.
pub fn printk(args: Arguments<'_>) {
    unsafe {
        let console = CONSOLE;
        if !console.is_null() {
            let _ = (&mut *console).write_fmt(args);
        }
    }
}

#[macro_export]
macro_rules! printk {
    () => {
        $crate::console::printk(core::format_args!("\n"));
    };
    ($($arg:tt)*) => {
        $crate::console::printk(core::format_args!($($arg)*));
    };
}

pub enum ConsoleWriter<'a> {
    Pixel(&'a PixelWriter),
    Window(Rc<RefCell<Window>>),
}

impl PixelDraw for ConsoleWriter<'_> {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        match self {
            Self::Pixel(writer) => writer.write(x, y, c),
            Self::Window(window) => window.borrow_mut().write(x, y, *c),
        }
    }

    fn width(&self) -> usize {
        match self {
            Self::Pixel(writer) => writer.width(),
            Self::Window(window) => window.borrow_mut().width(),
        }
    }

    fn height(&self) -> usize {
        match self {
            Self::Pixel(writer) => writer.height(),
            Self::Window(window) => window.borrow_mut().height(),
        }
    }
}

pub struct Console<'a> {
    writer: ConsoleWriter<'a>,
    fg_color: PixelColor,
    bg_color: PixelColor,

    buffer: [[u8; 80]; 25 + 1],
    cursor_row: usize,
    cursor_column: usize,
    redraw: Option<fn(u32)>,
    layer_id: u32,
}

impl<'a> fmt::Write for Console<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put_string(&s);
        Ok(())
    }
}

impl<'a> Console<'a> {
    pub const ROW_NUM: usize = 25;
    pub const COLUMN_NUM: usize = 80;
    pub fn new(writer: ConsoleWriter<'a>, fg_color: PixelColor, bg_color: PixelColor) -> Self {
        Self {
            writer,
            fg_color,
            bg_color,
            buffer: [[b' '; Self::COLUMN_NUM]; Self::ROW_NUM + 1],
            cursor_row: 0,
            cursor_column: 0,
            redraw: None,
            layer_id: 0,
        }
    }
    pub fn set_writer(&mut self, writer: ConsoleWriter<'a>) {
        self.writer = writer;
    }
    pub fn set_redraw(&mut self, redraw: fn(u32)) {
        self.redraw = Some(redraw);
    }
    pub fn set_layerid(&mut self, id: u32) {
        self.layer_id = id;
    }
    pub fn get_layer_id(&self) -> u32 {
        self.layer_id
    }
    pub fn refresh(&self) {
        for r in 0..Self::ROW_NUM {
            write_string(
                &self.writer,
                0,
                16 * r,
                core::str::from_utf8(&self.buffer[r]).unwrap(),
                &self.fg_color,
            );
        }
    }
    pub fn put_string(&mut self, str: &str) {
        for c in str.bytes() {
            if c == b'\r' {
                self.cursor_column = 0;
            } else if c == b'\n' {
                self.new_line();
            } else if c == b'\t' {
                let next_tab = (self.cursor_column / 4 + 1) * 4;
                while self.cursor_column < next_tab {
                    self.put_printable(b' ');
                }
            } else {
                self.put_printable(c);
            }
        }
        if let Some(redraw) = self.redraw {
            redraw(self.layer_id);
        }
    }

    fn put_printable(&mut self, c: u8) {
        if self.cursor_column >= Self::COLUMN_NUM {
            self.new_line();
        }
        if self.cursor_column < Self::COLUMN_NUM {
                write_ascii(
                    &self.writer,
                    8 * self.cursor_column,
                    16 * self.cursor_row,
                    c,
                    &self.fg_color,
                );
                self.buffer[self.cursor_row][self.cursor_column] = c;
                self.cursor_column += 1;
        }
    }
    fn new_line(&mut self) {
        self.cursor_column = 0;
        if self.cursor_row < Self::ROW_NUM - 1 {
            self.cursor_row += 1;
            return;
        }

        match &self.writer {
            ConsoleWriter::Pixel(_) => {
                for y in 0..16 * Self::ROW_NUM {
                    for x in 0..8 * Self::COLUMN_NUM {
                        self.writer.write(x, y, &self.bg_color);
                    }
                }
                for y in 0..(Self::ROW_NUM - 1) {
                    self.buffer[y] = self.buffer[y + 1];
                    write_string(
                        &self.writer,
                        0,
                        16 * y,
                        &(core::str::from_utf8(&self.buffer[y]).unwrap()),
                        &self.fg_color,
                    );
                }
                self.buffer[Self::ROW_NUM - 1] = [b' '; Self::COLUMN_NUM];
            }
            ConsoleWriter::Window(ref_cell) => {
                for row in 0..Self::ROW_NUM - 1 {
                    self.buffer[row] = self.buffer[row + 1];
                }
                self.buffer[Self::ROW_NUM - 1] = [b' '; Self::COLUMN_NUM];

                let mut window = ref_cell.borrow_mut();
                window.fill_rect(
                    Vector2D { x: 0, y: 0 },
                    Vector2D {
                        x: 8 * Self::COLUMN_NUM,
                        y: 16 * Self::ROW_NUM,
                    },
                    self.bg_color,
                );
                let window_writer = window.writer();
                for row in 0..Self::ROW_NUM {
                    write_string(
                        &window_writer,
                        0,
                        16 * row,
                        core::str::from_utf8(&self.buffer[row]).unwrap(),
                        &self.fg_color,
                    );
                }
            }
        };
    }
}
