use crate::{PixelColor, PixelWriter};
const FONT: &[u8; 4096] = include_bytes!("../hankaku.bin");

pub fn write_ascii(writer: &dyn PixelWriter, x: usize, y: usize, c: char, color: &PixelColor) {
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
