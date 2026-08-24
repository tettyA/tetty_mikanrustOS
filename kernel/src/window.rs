use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::{
    frame_buffer::FrameBuffer,
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{PixelColor, PixelDraw, Vector2D},
};

/// the region of graphics drawing.
/// Manage not only Window with title or menu, and also the visual region such as MouseCursor's region and on.
pub struct Window {
    width: usize,
    height: usize,
    data: Vec<Vec<PixelColor>>,
    shadow_buffer: FrameBuffer,
    transparent_color: Option<PixelColor>,
}
impl Window {
    pub fn new(width: usize, height: usize, shadow_format: PixelFormat) -> Self {
        Self {
            width,
            height,
            data: alloc::vec![alloc::vec![PixelColor{r:0,g:0,b:0};width];height],
            shadow_buffer: FrameBuffer::new(FrameBufferConfig {
                frame_buffer: core::ptr::null_mut(),
                pixels_per_scan_line: width as u32,
                horizontal_resolution: width as u32,
                vertical_resolution: height as u32,
                pixel_format: shadow_format,
            }),
            transparent_color: None,
        }
    }

    pub fn write(&mut self, x: usize, y: usize, c: PixelColor) {
        if x < self.width && y < self.height {
            self.data[y][x] = c;
            self.shadow_buffer.writer().write(x, y, &c);
        }
    }

    /// Draw the drawing-region of this window by given PixelWriter
    /// # Arguments
    /// `writer`: Writer  
    ///
    /// `pos` : Left-up x-y position
    pub fn draw_to<W: PixelDraw>(&self, dst: &W, pos: Vector2D<i32>) {
        if let Some(tc) = self.transparent_color {
            for y in 0..self.height {
                for x in 0..self.width {
                    if tc == self.data[y][x] {
                        continue;
                    }
                    let dst_x = pos.x + x as i32;
                    let dst_y = pos.y + y as i32;
                    let c = self.data[y][x];
                    if dst_x >= 0 && dst_y >= 0 {
                        dst.write(dst_x as usize, dst_y as usize, &c);
                    }
                }
            }
        } else {
            self.shadow_buffer.draw_to(dst, pos);
        }
    }

    /// Set transparent color.
    pub fn set_transparent_color(&mut self, c: Option<PixelColor>) {
        self.transparent_color = c;
    }

    /// Get WindowWriter associated with this instance.
    pub fn writer(&mut self) -> WindowWriter {
        WindowWriter {
            window: self as *mut Window,
            _marker: PhantomData,
        }
    }

    /// return specified PixelColor.
    pub fn at(&self, x: usize, y: usize) -> PixelColor {
        self.data[y][x]
    }

    /// return width
    pub fn width(&self) -> usize {
        self.width
    }
    /// return heigth
    pub fn height(&self) -> usize {
        self.height
    }
}

/// WinodowWriter provides a PixelWriter associated with the Window.

pub struct WindowWriter<'a> {
    window: *mut Window,
    _marker: PhantomData<&'a mut Window>,
}

impl<'a> PixelDraw for WindowWriter<'a> {
    /// draw color to specifyed pixel.
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        unsafe {
            (*self.window).write(x, y, *c);
        }
    }
}
