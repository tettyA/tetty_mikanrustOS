use alloc::vec::Vec;
use core::marker::PhantomData;

use crate::{
    font::write_string,
    frame_buffer::FrameBuffer,
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{PixelColor, PixelDraw, Rectangle, Vector2D, fill_rectrange, to_color},
};

const K_CLOSE_BUTTON_WIDTH: usize = 16;
const K_CLOSE_BUTTON_HEIGHT: usize = 14;
const WINDOW_CLOSE_BUTTON: [&'static str; K_CLOSE_BUTTON_HEIGHT] = [
    "...............@",
    ".:::::::::::::$@",
    ".:::::::::::::$@",
    ".:::@@::::@@::$@",
    ".::::@@::@@:::$@",
    ".:::::@@@@::::$@",
    ".::::::@@:::::$@",
    ".:::::@@@@::::$@",
    ".::::@@::@@:::$@",
    ".:::@@::::@@::$@",
    ".:::::::::::::$@",
    ".:::::::::::::$@",
    ".$$$$$$$$$$$$$$@",
    "@@@@@@@@@@@@@@@@",
];

pub const TOP_LEFT_MARGIN: Vector2D<usize> = Vector2D { x: 4, y: 24 };
pub const BOTTOM_RIGHT_MARGIN: Vector2D<usize> = Vector2D { x: 4, y: 4 };

/// タイトルバーを描画する
pub fn draw_window_title<T: PixelDraw>(writer: &T, title: &str, active: bool) {
    let win_w = writer.width();
    let bgcolor = if active { 0x000084 } else { 0x848484 };

    fill_rectrange(
        writer,
        &Vector2D { x: 3, y: 3 },
        &Vector2D {
            x: win_w - 6,
            y: 18,
        },
        &to_color(bgcolor),
    );
    write_string(writer, 24, 4, title, &to_color(0xffffff));
    for (y, s) in WINDOW_CLOSE_BUTTON.iter().enumerate() {
        for (x, c) in s.chars().enumerate() {
            let col = if c == '@' {
                to_color(0x000000)
            } else if c == '$' {
                to_color(0x848484)
            } else if c == ':' {
                to_color(0xc6c6c6)
            } else {
                to_color(0x000000)
            };
            writer.write(win_w - 5 - K_CLOSE_BUTTON_WIDTH + x, 5 + y, &col);
        }
    }
}

pub fn draw_window<T: PixelDraw>(writer: &T, title: &str) {
    let fill_rect = |posx: usize, posy: usize, sizew: usize, sizeh: usize, c: u32| {
        fill_rectrange(
            writer,
            &Vector2D { x: posx, y: posy },
            &Vector2D { x: sizew, y: sizeh },
            &to_color(c),
        );
    };

    let win_w = writer.width();
    let win_h = writer.height();

    fill_rect(0, 0, win_w, 1, 0xc6c6c6);
    fill_rect(1, 1, win_w - 2, 1, 0xffffff);
    fill_rect(0, 0, 1, win_h, 0xc6c6c6);
    fill_rect(1, 1, 1, win_h - 2, 0xffffff);
    fill_rect(win_w - 2, 1, 1, win_h - 2, 0x848484);
    fill_rect(win_w - 1, 0, 1, win_h, 0x000000);
    fill_rect(2, 2, win_w - 4, win_h - 4, 0xc6c6c6);
    fill_rect(3, 3, win_w - 4, 18, 0x000084);
    fill_rect(1, win_h - 2, win_w - 2, 1, 0x848484);
    fill_rect(0, win_h - 1, win_w, 1, 0x000000);

    draw_window_title(writer, title, false);
}

pub fn draw_textbox<T: PixelDraw>(writer: &T, pos: Vector2D<usize>, size: Vector2D<usize>) {
    let fill_rect = |pos: Vector2D<usize>, size: Vector2D<usize>, c: u32| {
        fill_rectrange(
            writer,
            &Vector2D { x: pos.x, y: pos.y },
            &Vector2D {
                x: size.x,
                y: size.y,
            },
            &to_color(c),
        );
    };

    // fill main box
    fill_rect(
        pos + Vector2D { x: 1, y: 1 },
        size - Vector2D { x: 2, y: 2 },
        0xffffff,
    );

    // draw border
    fill_rect(pos, Vector2D { x: size.x, y: 1 }, 0x848484);
    fill_rect(pos, Vector2D { x: 1, y: size.y }, 0x848484);
    fill_rect(
        pos + Vector2D { x: 0, y: size.y },
        Vector2D { x: size.x, y: 1 },
        0xc6c6c6,
    );
    fill_rect(
        pos + Vector2D { x: size.x, y: 0 },
        Vector2D { x: 1, y: size.y },
        0xc6c6c6,
    );
}

/// the region of graphics drawing.
/// Manage not only Window with title or menu, and also the visual region such as MouseCursor's region and on.
pub struct Window {
    width: usize,
    height: usize,
    data: Vec<Vec<PixelColor>>,
    shadow_buffer: FrameBuffer,
    transparent_color: Option<PixelColor>,
    title: Option<alloc::string::String>, // アクティブウィンドウの時，Some("??")
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
            title: None,
        }
    }

    pub fn new_toplevel(
        width: usize,
        height: usize,
        shadow_format: PixelFormat,
        title: &str,
    ) -> Self {
        let mut window = Self::new(width, height, shadow_format);
        window.title = Some(alloc::string::String::from(title));
        draw_window(&window.writer(), title);

        window
    }
    pub fn activate(&mut self) {
        if let Some(title) = &self.title.clone() {
            draw_window_title(&self.writer(), title, true);
        }
    }
    pub fn deactivate(&mut self) {
        if let Some(title) = &self.title.clone() {
            draw_window_title(&self.writer(), title, false);
        }
    }
    pub fn inner_size(&self) -> Vector2D<usize> {
        Vector2D {
            x: self
                .width
                .saturating_sub(TOP_LEFT_MARGIN.x + BOTTOM_RIGHT_MARGIN.x),
            y: self
                .height
                .saturating_sub(TOP_LEFT_MARGIN.y + BOTTOM_RIGHT_MARGIN.y),
        }
    }
    pub fn inner_writer(&mut self) -> InnerAreaWriter<'_> {
        InnerAreaWriter {
            window: self as *mut Window,
            _marker: PhantomData,
        }
    }
    pub fn window_move(&mut self, dst_pos: Vector2D<i32>, src: &Rectangle<i32>) {
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

    pub fn draw_to_area(&self, dst: &mut FrameBuffer, pos: Vector2D<i32>, area: Rectangle<i32>) {
        if self.transparent_color.is_none() {
            dst.copy_window_area(pos, &area, &self.shadow_buffer);
            return;
        }

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
    pub fn writer(&mut self) -> WindowWriter<'_> {
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

    fn width(&self) -> usize {
        unsafe { (*self.window).width }
    }

    fn height(&self) -> usize {
        unsafe { (*self.window).height }
    }
}

pub struct InnerAreaWriter<'a> {
    window: *mut Window,
    _marker: PhantomData<&'a mut Window>,
}

impl<'a> PixelDraw for InnerAreaWriter<'a> {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        unsafe {
            (*self.window).write(x + TOP_LEFT_MARGIN.x, y + TOP_LEFT_MARGIN.y, *c);
        }
    }

    fn width(&self) -> usize {
        unsafe { (*self.window).inner_size().x }
    }

    fn height(&self) -> usize {
        unsafe { (*self.window).inner_size().y }
    }
}
