use crate::frame_buffer_config::FrameBufferConfig;
use core::ptr;
pub trait PixelWriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor);
}

pub struct Bgrpixelwriter {
    frame_buffer: *mut u8,
    pixels_per_scan_line: usize,
}
impl Bgrpixelwriter {
    pub fn new(config: &FrameBufferConfig) -> Self {
        Self {
            frame_buffer: config.frame_buffer,
            pixels_per_scan_line: config.pixels_per_scan_line as usize,
        }
    }
}
impl PixelWriter for Bgrpixelwriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
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
}
impl Rgbpixelwriter {
    pub fn new(config: &FrameBufferConfig) -> Self {
        Self {
            frame_buffer: config.frame_buffer,
            pixels_per_scan_line: config.pixels_per_scan_line as usize,
        }
    }
}
impl PixelWriter for Rgbpixelwriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        let pixel_posititon = (self.pixels_per_scan_line as usize * y) + x;
        let offset = 4 * pixel_posititon as usize;
        unsafe {
            ptr::write_volatile(self.frame_buffer.add(offset), c.r);
            ptr::write_volatile(self.frame_buffer.add(offset + 1), c.g);
            ptr::write_volatile(self.frame_buffer.add(offset + 2), c.b);
        }
    }
}
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
