use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::graphics::{PixelColor, PixelDraw, PixelWriter, Vector2D};

/// the region of graphics drawing.
/// Manage not only Window with title or menu, and also the visual region such as MouseCursor's region and on.
pub struct Window {
    width: usize,
    height: usize,
    data: Vec<Vec<PixelColor>>,
    transparent_color: Option<PixelColor>,
}

impl Window {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: alloc::vec![alloc::vec![PixelColor{r:0,g:0,b:0};width];height],
            transparent_color: None,
        }
    }

    pub fn write(&mut self, x: usize, y: usize, c: PixelColor) {
        if x < self.width && y < self.height {
            self.data[y][x] = c;
        }
    }

    /// Draw the drawing-region of this window by given PixelWriter
    /// # Arguments
    /// `writer`: Writer  
    ///
    /// `pos` : Left-up x-y position
    pub fn draw_to(&self, writer: &PixelWriter, pos: Vector2D<usize>) {
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(c) = self.transparent_color {
                    if c == self.data[y][x] {
                        continue;
                    }
                }

                writer.write(pos.x + x, pos.y + y, &self.data[y][x]);
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
// #@@range_end(window)

// / WinodowWriter provides a PixelWriter associated with the Window.

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
