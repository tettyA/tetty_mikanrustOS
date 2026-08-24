use core::marker::PhantomData;

use alloc::vec::Vec;

use crate::{
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    graphics::{PixelColor, PixelDraw, Rectrange, Vector2D},
};

pub struct FrameBuffer {
    config: FrameBufferConfig,
    buffer: Vec<u8>,
}

impl FrameBuffer {
    pub fn new(config: FrameBufferConfig) -> Self {
        let bytes_per_pixel = 4;
        let size =
            (config.pixels_per_scan_line * config.vertical_resolution * bytes_per_pixel) as usize;
        let buffer = if config.frame_buffer.is_null() {
            // 実RAMがnullである場合は，動的にメモリ領域を確保する。
            alloc::vec![0;size]
        } else {
            // 実RAMである，何らかのポインタが設定されている場合は，そのポインタが指す領域を描画領域として扱う。
            Vec::<u8>::new()
        };

        Self { config, buffer }
    }

    pub fn get_slice_mut(&mut self) -> &mut [u8] {
        if self.buffer.is_empty() {
            let size =
                (self.config.pixels_per_scan_line * self.config.vertical_resolution * 4) as usize;
            unsafe { core::slice::from_raw_parts_mut(self.config.frame_buffer, size) }
        } else {
            &mut self.buffer
        }
    }

    pub fn get_slice(&self) -> &[u8] {
        if self.buffer.is_empty() {
            let size =
                (self.config.pixels_per_scan_line * self.config.vertical_resolution * 4) as usize;
            unsafe { core::slice::from_raw_parts(self.config.frame_buffer, size) }
        } else {
            &self.buffer
        }
    }
    pub fn copy(&mut self, pos: Vector2D<i32>, src: &FrameBuffer) {
        let bytes_per_pixel = 4;

        let dst_width = self.config.horizontal_resolution;
        let dst_height = self.config.vertical_resolution;
        let src_width = src.config.horizontal_resolution;
        let src_height = src.config.vertical_resolution;

        // 描画先のクリッピング座標の計算
        let dist_start_x = pos.x.max(0) as u32;
        let dist_start_y = pos.y.max(0) as u32;
        let dist_end_x = ((pos.x + src_width as i32).max(0) as u32).min(dst_width);
        let dist_end_y = ((pos.y + src_height as i32).max(0) as u32).min(dst_height);
        if dist_start_x >= dist_end_x || dist_start_y >= dist_end_y {
            return;
        }
        let copy_width = (dist_end_x - dist_start_x) as u32;
        let copy_height = (dist_end_y - dist_start_y) as u32;

        // 転送元のオフセットの計算 (pos.xがマイナスだと良い感じに消えて一部描画。プラスだと0で全て描画)
        let src_start_x = (dist_start_x as i32 - pos.x) as u32;
        let src_start_y = (dist_start_y as i32 - pos.y) as u32;

        let dst_pixels_per_scan_line = self.config.pixels_per_scan_line;
        let src_pixels_per_scan_line = src.config.pixels_per_scan_line;

        let dst_slice = self.get_slice_mut();
        let src_slice = src.get_slice();

        for y in 0..copy_height {
            let current_dst_y = dist_start_y + y;
            let current_src_y = src_start_y + y;

            let dst_offset =
                (current_dst_y * dst_pixels_per_scan_line + dist_start_x) * bytes_per_pixel;
            let src_offset =
                (current_src_y * src_pixels_per_scan_line + src_start_x) * bytes_per_pixel;

            let line_bytes = copy_width * bytes_per_pixel;

            dst_slice[dst_offset as usize..(dst_offset + line_bytes) as usize].copy_from_slice(
                &src_slice[src_offset as usize..(src_offset + line_bytes) as usize],
            );
        }
    }
    pub fn r#move(&mut self, dst_pos: Vector2D<i32>, src: &Rectrange<i32>) {
        let bytes_per_pixel = bytes_per_pixel(&self.config.pixel_format) as usize;
        let bytes_per_scan_line = bytes_per_scan_line(&self.config) as usize;
        let width = src.size.x as usize;
        let height = src.size.y as usize;
        let dst_x = dst_pos.x as usize;
        let dst_y = dst_pos.y as usize;
        let src_x = src.pos_lu.x as usize;
        let src_y = src.pos_lu.y as usize;
        let line_bytes = width * bytes_per_pixel;
        let horizontal_resolution = self.config.horizontal_resolution as usize;
        let buffer = self.get_slice_mut();

        if src_x == dst_x && width == horizontal_resolution {
            let src_offset = src_y * bytes_per_scan_line + src_x * bytes_per_pixel;
            let dst_offset = dst_y * bytes_per_scan_line + dst_x * bytes_per_pixel;
            buffer.copy_within(
                src_offset..src_offset + height * bytes_per_scan_line,
                dst_offset,
            );
            return;
        }

        if dst_pos.y < src.pos_lu.y {
            for row in 0..height {
                let src_offset = (src_y + row) * bytes_per_scan_line + src_x * bytes_per_pixel;
                let dst_offset = (dst_y + row) * bytes_per_scan_line + dst_x * bytes_per_pixel;
                buffer.copy_within(src_offset..src_offset + line_bytes, dst_offset);
            }
        } else {
            for row in (0..height).rev() {
                let src_offset = (src_y + row) * bytes_per_scan_line + src_x * bytes_per_pixel;
                let dst_offset = (dst_y + row) * bytes_per_scan_line + dst_x * bytes_per_pixel;
                buffer.copy_within(src_offset..src_offset + line_bytes, dst_offset);
            }
        }
    }

    pub fn draw_to<W: PixelDraw>(&self, writer: &W, pos: Vector2D<i32>) {
        let bytes_per_pixel = 4;
        let slice = self.get_slice();

        for y in 0..self.config.vertical_resolution as usize {
            let dst_y = pos.y + y as i32;
            if dst_y < 0 {
                continue;
            }
            for x in 0..self.config.horizontal_resolution as usize {
                let dst_x = pos.x + x as i32;
                if dst_x < 0 {
                    continue;
                }

                let offset = (y * self.config.pixels_per_scan_line as usize + x) * bytes_per_pixel;
                let c = match self.config.pixel_format {
                    PixelFormat::kPixelRGBResv8BitPerColor => PixelColor {
                        r: slice[offset],
                        g: slice[offset + 1],
                        b: slice[offset + 2],
                    },
                    PixelFormat::kPixelBGRResv8BitPerColor => PixelColor {
                        r: slice[offset + 2],
                        g: slice[offset + 1],
                        b: slice[offset],
                    },
                };
                writer.write(dst_x as usize, dst_y as usize, &c);
            }
        }
    }

    pub fn writer(&mut self) -> FrameBufferWriter<'_> {
        FrameBufferWriter {
            frame_buffer: self,
            _marker: PhantomData,
        }
    }
}

fn bytes_per_pixel(format: &PixelFormat) -> i32 {
    match format {
        PixelFormat::kPixelRGBResv8BitPerColor => 4,
        PixelFormat::kPixelBGRResv8BitPerColor => 4,
    }
}
fn frame_addr_at(pos: Vector2D<i32>, config: &FrameBufferConfig) -> *mut u8 {
    unsafe {
        config.frame_buffer.add(
            (bytes_per_pixel(&config.pixel_format)
                * (config.pixels_per_scan_line as i32 * pos.y + pos.x)) as usize,
        )
    }
}

fn bytes_per_scan_line(config: &FrameBufferConfig) -> i32 {
    bytes_per_pixel(&config.pixel_format) * config.pixels_per_scan_line as i32
}

fn frame_buffer_size(config: &FrameBufferConfig) -> Vector2D<i32> {
    Vector2D {
        x: config.horizontal_resolution as i32,
        y: config.vertical_resolution as i32,
    }
}

pub struct FrameBufferWriter<'a> {
    frame_buffer: *mut FrameBuffer,
    _marker: PhantomData<&'a mut FrameBuffer>,
}

impl<'a> PixelDraw for FrameBufferWriter<'a> {
    /// draw color to specifyed pixel.
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        unsafe {
            let fb = &mut *self.frame_buffer;
            if x >= fb.config.horizontal_resolution as usize
                || y >= fb.config.vertical_resolution as usize
            {
                return;
            }
            let fb_config_pixel_format = fb.config.pixel_format;
            let offset = ((y * fb.config.pixels_per_scan_line as usize + x) * 4) as usize;
            let slice = fb.get_slice_mut();

            match fb_config_pixel_format {
                PixelFormat::kPixelRGBResv8BitPerColor => {
                    slice[offset] = c.r;
                    slice[offset + 1] = c.g;
                    slice[offset + 2] = c.b;
                }
                PixelFormat::kPixelBGRResv8BitPerColor => {
                    slice[offset] = c.b;
                    slice[offset + 1] = c.g;
                    slice[offset + 2] = c.r;
                }
            }
        }
    }

    fn width(&self) -> usize {
        unsafe { (&mut *self.frame_buffer).config.horizontal_resolution as usize }
    }

    fn height(&self) -> usize {
        unsafe { (&mut *self.frame_buffer).config.vertical_resolution as usize }
    }
}
