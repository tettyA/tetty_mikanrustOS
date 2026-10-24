use crate::{PixelColor, graphics::PixelDraw};
const FONT: &[u8; 4096] = include_bytes!("../hankaku.bin");

pub fn write_ascii<W: PixelDraw + ?Sized>(
    writer: &W,
    x: usize,
    y: usize,
    c: u8,
    color: &PixelColor,
) {
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

pub fn write_string<W: PixelDraw + ?Sized>(
    writer: &W,
    mut x: usize,
    y: usize,
    s: &str,
    color: &PixelColor,
) {
    for c in s.bytes() {
        write_ascii(writer, x, y, c as u8, color);
        x += 8;
    }
}
