use crate::font::{write_ascii, write_string};
use crate::graphics::{PixelColor, PixelWriter};
use core::fmt;
pub struct Console<'a> {
    writer: &'a dyn PixelWriter,
    fg_color: PixelColor,
    bg_color: PixelColor,

    buffer: [[u8; 80]; 25 + 1],
    cursor_row: usize,
    cursor_column: usize,
}

impl<'a> fmt::Write for Console<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put_string(&s);
        Ok(())
    }
}

impl<'a> Console<'a> {
    const ROW_NUM: usize = 25;
    const COLUMN_NUM: usize = 80;
    pub fn new(writer: &'a dyn PixelWriter, fg_color: PixelColor, bg_color: PixelColor) -> Self {
        Self {
            writer,
            fg_color,
            bg_color,
            buffer: [[0; Self::COLUMN_NUM]; Self::ROW_NUM + 1],
            cursor_row: 0,
            cursor_column: 0,
        }
    }
    pub fn put_string(&mut self, str: &str) {
        for c in str.bytes() {
            if c == '\n' as u8 {
                self.new_line();
            } else if self.cursor_column < Self::COLUMN_NUM - 1 {
                write_ascii(
                    self.writer,
                    8 * self.cursor_column,
                    16 * self.cursor_row,
                    c,
                    &self.fg_color,
                );
                self.buffer[self.cursor_row][self.cursor_column] = c;
                self.cursor_column += 1;
            }
        }
    }
    fn new_line(&mut self) {
        self.cursor_column = 0;
        if self.cursor_row < Self::ROW_NUM - 1 {
            self.cursor_row += 1;
        } else {
            for y in 0..16 * Self::ROW_NUM {
                for x in 0..8 * Self::COLUMN_NUM {
                    self.writer.write(x, y, &self.bg_color);
                }
            }
            for y in 0..(Self::ROW_NUM - 1) {
                self.buffer[y] = self.buffer[y + 1];
                write_string(
                    self.writer,
                    0,
                    16 * y,
                    &(core::str::from_utf8(&self.buffer[y]).unwrap()),
                    &self.fg_color,
                );
            }
            self.buffer[Self::ROW_NUM - 1] = [0; Self::COLUMN_NUM];
        }
    }
}
