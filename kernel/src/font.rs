use crate::{PixelColor, PixelWriter};
const FONT: [u8; 16] = [
    0b00000000, //
    0b00011000, // ...##...
    0b00011000, // ...##...
    0b00011000, // ...##...
    0b00011000, // ...##...
    0b00100100, // ..#..#..
    0b00100100, // ..#..#..
    0b00100100, // ..#..#..
    0b00100100, // ..#..#..
    0b01111110, // .######.
    0b01000010, // .#....#.
    0b01000010, // .#....#.
    0b01000010, // .#....#.
    0b11100111, // ###..###
    0b00000000, // ........
    0b00000000, // ........
];

pub fn write_ascii(writer: &dyn PixelWriter, x: usize, y: usize, c: char, color: &PixelColor) {
    if c != 'A' {
        return;
    }

    for dy in 0..16 {
        for dx in 0..8 {
            if ((FONT[dy] << dx) & 0x80) != 0 {
                writer.write(x + dx, y + dy, color);
            }
        }
    }
}
