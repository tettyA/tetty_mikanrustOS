use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::{
    frame_buffer::FrameBuffer,
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{PixelColor, PixelDraw, Rectrange, Vector2D},
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
    pub fn window_move(&mut self, dst_pos: Vector2D<i32>, src: &Rectrange<i32>) {
        self.shadow_buffer.r#move(dst_pos, src);
    }
    pub fn write(&mut self, x: usize, y: usize, c: PixelColor) {
        if x < self.width && y < self.height {
            self.data[y][x] = c;
            self.shadow_buffer.writer().write(x, y, &c);
        }
    }

    pub fn fill_rect(&mut self, pos: Vector2D<usize>, size: Vector2D<usize>, c: PixelColor) {
        let end_x = (pos.x + size.x).min(self.width);
        let end_y = (pos.y + size.y).min(self.height);
        let shadow_writer = self.shadow_buffer.writer();
        for y in pos.y..end_y {
            for x in pos.x..end_x {
                self.data[y][x] = c;
                shadow_writer.write(x, y, &c);
            }
        }
    }

    /// Draw the drawing-region of this window by given PixelWriter
    /// # Arguments
    /// `writer`: Writer  
    ///
    /// `pos` : Left-up x-y position
    pub fn draw_to(&self, dst: &mut FrameBuffer, pos: Vector2D<i32>) {
        if let Some(tc) = self.transparent_color {
            let dst_writer = dst.writer();
            for y in 0..self.height {
                for x in 0..self.width {
                    if tc == self.data[y][x] {
                        continue;
                    }
                    let dst_x = pos.x + x as i32;
                    let dst_y = pos.y + y as i32;
                    let c = self.data[y][x];
                    if dst_x >= 0 && dst_y >= 0 {
                        dst_writer.write(dst_x as usize, dst_y as usize, &c);
                    }
                }
            }
        } else {
            dst.copy(pos, &self.shadow_buffer);
        }
    }

    pub fn draw_to_area(&self, dst: &mut FrameBuffer, pos: Vector2D<i32>, area: Rectrange<i32>) {
        let start_x = area.pos_lu.x.max(pos.x).max(0);
        let start_y = area.pos_lu.y.max(pos.y).max(0);
        let end_x = (area.pos_lu.x + area.size.x).min(pos.x + self.width as i32);
        let end_y = (area.pos_lu.y + area.size.y).min(pos.y + self.height as i32);
        if start_x >= end_x || start_y >= end_y {
            return;
        }

        let writer = dst.writer();
        for y in start_y..end_y {
            for x in start_x..end_x {
                let local_x = (x - pos.x) as usize;
                let local_y = (y - pos.y) as usize;
                if self.transparent_color == Some(self.data[local_y][local_x]) {
                    continue;
                }
                writer.write(x as usize, y as usize, &self.data[local_y][local_x]);
            }
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
