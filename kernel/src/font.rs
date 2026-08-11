use crate::{PixelColor, PixelWriter};
use core::fmt;
const FONT: &[u8; 4096] = include_bytes!("../hankaku.bin");

pub fn write_ascii(writer: &dyn PixelWriter, x: usize, y: usize, c: u8, color: &PixelColor) {
    let font_index = c as usize * 16;
    let font_data = &FONT[font_index..(font_index + 16)];

    for dy in 0..16 {
        for dx in 0..8 {
            if ((font_data[dy] << dx) & 0x80) != 0 {
                writer.write(x + dx, y + dy, color);
            }
        }
    }
}

pub fn write_string(writer: &dyn PixelWriter, mut x: usize, y: usize, s: &str, color: &PixelColor) {
    for c in s.bytes() {
        write_ascii(writer, x, y, c as u8, color);
        x += 8;
    }
}

pub struct StringWriter<'a> {
    writer: &'a dyn PixelWriter,
    x: usize,
    y: usize,
    color: &'a PixelColor,
}

impl<'a> StringWriter<'a> {
    pub fn new(writer: &'a dyn PixelWriter, x: usize, y: usize, color: &'a PixelColor) -> Self {
        Self {
            writer,
            x,
            y,
            color,
        }
    }
}

impl<'a> fmt::Write for StringWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_string(self.writer, self.x, self.y, s, self.color);
        self.x += s.len() * 8;
        Ok(())
    }
}

#[macro_export]
macro_rules! write_stringf {
    ($writer:expr,$x:expr,$y:expr,$color:expr,$($arg:tt)*) => {
        use core::fmt::Write;
        let mut drawer = $crate::font::StringWriter::new($writer, $x, $y, $color);
        write!(drawer, $($arg)*).unwrap();
    };
      ($writer:expr,$x:expr,$y:expr,R:$r:expr,G:$g:expr,B:$b:expr,$($arg:tt)*) => {
        let mut drawer = $crate::font::StringWriter::new($writer, $x, $y, &PixelColor{r:$r,g:$g,b:$b});
        write!(drawer, $($arg)*).unwrap();
    };
}
