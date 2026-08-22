use crate::{PixelColor, graphics::PixelDraw};
use core::fmt;
const FONT: &[u8; 4096] = include_bytes!("../hankaku.bin");

pub fn write_ascii<W: PixelDraw + ?Sized>(writer: &W, x: usize, y: usize, c: u8, color: &PixelColor) {
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

pub fn write_string<W: PixelDraw + ?Sized>(writer: &W, mut x: usize, y: usize, s: &str, color: &PixelColor) {
    for c in s.bytes() {
        write_ascii(writer, x, y, c as u8, color);
        x += 8;
    }
}

pub struct StringWriter<'a, W: PixelDraw + ?Sized> {
    writer: &'a W,
    x: usize,
    y: usize,
    color: &'a PixelColor,
}

impl<'a, W: PixelDraw + ?Sized> StringWriter<'a, W> {
    pub fn new(writer: &'a W, x: usize, y: usize, color: &'a PixelColor) -> Self {
        Self {
            writer,
            x,
            y,
            color,
        }
    }
}

impl<'a, W: PixelDraw + ?Sized> fmt::Write for StringWriter<'a, W> {
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
