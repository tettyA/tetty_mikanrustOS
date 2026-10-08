use crate::frame_buffer_config::FrameBufferConfig;
use core::ops::{Add, AddAssign};
use core::ptr;

pub trait PixelDraw {
    fn write(&self, x: usize, y: usize, c: &PixelColor);
    fn width(&self) -> usize;
    fn height(&self) -> usize;
}

pub enum PixelWriter {
    Rgb(Rgbpixelwriter),
    Bgr(Bgrpixelwriter),
}
/*pub trait PixelWriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor);
}*/

impl PixelDraw for PixelWriter {
    #[inline(always)]
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        match self {
            PixelWriter::Bgr(writer) => writer.write(x, y, c),
            PixelWriter::Rgb(writer) => writer.write(x, y, c),
        }
    }

    fn width(&self) -> usize {
        match self {
            PixelWriter::Bgr(writer) => writer.horizontal_resolution,
            PixelWriter::Rgb(writer) => writer.horizontal_resolution,
        }
    }

    fn height(&self) -> usize {
        match self {
            PixelWriter::Bgr(writer) => writer.vertical_resolution,
            PixelWriter::Rgb(writer) => writer.vertical_resolution,
        }
    }
}

pub struct Bgrpixelwriter {
    frame_buffer: *mut u8,
    pixels_per_scan_line: usize,
    horizontal_resolution: usize,
    vertical_resolution: usize,
}
impl Bgrpixelwriter {
    pub fn new(config: &FrameBufferConfig) -> Self {
        Self {
            frame_buffer: config.frame_buffer,
            pixels_per_scan_line: config.pixels_per_scan_line as usize,
            horizontal_resolution: config.horizontal_resolution as usize,
            vertical_resolution: config.vertical_resolution as usize,
        }
    }
}
impl Bgrpixelwriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        if x >= self.horizontal_resolution || y >= self.vertical_resolution {
            return;
        }
        let pixel_posititon = (self.pixels_per_scan_line as usize * y) + x;
        let offset = 4 * pixel_posititon as usize;
        unsafe {
            ptr::write_volatile(self.frame_buffer.add(offset), c.b);
            ptr::write_volatile(self.frame_buffer.add(offset + 1), c.g);
            ptr::write_volatile(self.frame_buffer.add(offset + 2), c.r);
        }
    }
}
pub struct Rgbpixelwriter {
    frame_buffer: *mut u8,
    pixels_per_scan_line: usize,
    horizontal_resolution: usize,
    vertical_resolution: usize,
}
impl Rgbpixelwriter {
    pub fn new(config: &FrameBufferConfig) -> Self {
        Self {
            frame_buffer: config.frame_buffer,
            pixels_per_scan_line: config.pixels_per_scan_line as usize,
            horizontal_resolution: config.horizontal_resolution as usize,
            vertical_resolution: config.vertical_resolution as usize,
        }
    }
}
impl Rgbpixelwriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        if x >= self.horizontal_resolution || y >= self.vertical_resolution {
            return;
        }
        let pixel_posititon = (self.pixels_per_scan_line as usize * y) + x;
        let offset = 4 * pixel_posititon as usize;
        unsafe {
            ptr::write_volatile(self.frame_buffer.add(offset), c.r);
            ptr::write_volatile(self.frame_buffer.add(offset + 1), c.g);
            ptr::write_volatile(self.frame_buffer.add(offset + 2), c.b);
        }
    }
}

pub fn fill_rectrange<T, W: PixelDraw>(
    writer: &W,
    pos: &Vector2D<T>,
    size: &Vector2D<T>,
    c: &PixelColor,
) where
    T: Copy + Into<usize>,
{
    for dy in 0..size.y.into() {
        for dx in 0..size.x.into() {
            writer.write(pos.x.into() + dx, pos.y.into() + dy, c);
        }
    }
}

pub fn draw_rectrange<T, W: PixelDraw>(
    writer: &W,
    pos: &Vector2D<T>,
    size: &Vector2D<T>,
    c: &PixelColor,
) where
    T: Copy + Into<usize>,
{
    for dy in 0..size.y.into() {
        writer.write(pos.x.into(), pos.y.into() + dy, c);
        writer.write(pos.x.into() + size.x.into() - 1, pos.y.into() + dy, c);
    }
    for dx in 0..size.x.into() {
        writer.write(pos.x.into() + dx, pos.y.into(), c);
        writer.write(pos.x.into() + dx, pos.y.into() + size.y.into() - 1, c);
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub struct Vector2D<T> {
    pub x: T,
    pub y: T,
}

impl<T> Add for Vector2D<T>
where
    T: Add<Output = T>,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Vector2D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T> AddAssign for Vector2D<T>
where
    T: Add<Output = T> + Copy,
{
    fn add_assign(&mut self, rhs: Self) {
        self.x = self.x + rhs.x;
        self.y = self.y + rhs.y;
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Rectangle<T> {
    pub pos_lu: Vector2D<T>,
    pub size: Vector2D<T>,
}
