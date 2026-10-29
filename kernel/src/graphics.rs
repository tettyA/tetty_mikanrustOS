use core::ops::{Add, AddAssign, Sub, SubAssign};

use crate::{
    frame_buffer_config::{FrameBufferConfig, PixelFormat},
    global::Global,
};

pub trait PixelDraw {
    fn write(&self, x: usize, y: usize, c: &PixelColor);
    fn width(&self) -> usize;
    fn height(&self) -> usize;
}

pub fn fill_rectangle<T, W: PixelDraw>(
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

pub fn draw_rectangle<T, W: PixelDraw>(
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

impl<T> Sub for Vector2D<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Vector2D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T> SubAssign for Vector2D<T>
where
    T: Sub<Output = T> + Copy,
{
    fn sub_assign(&mut self, rhs: Self) {
        self.x = self.x - rhs.x;
        self.y = self.y - rhs.y;
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub const fn to_color(c: u32) -> PixelColor {
    PixelColor {
        r: ((c >> 16) & 0xff) as u8,
        g: ((c >> 8) & 0xff) as u8,
        b: (c & 0xff) as u8,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rectangle<T> {
    pub pos_lu: Vector2D<T>,
    pub size: Vector2D<T>,
}

impl<T> Rectangle<T>
where
    T: Copy + Ord + Add<Output = T> + Sub<Output = T>,
{
    /// 2つの矩形の交差（重なり）部分を計算する。重なりがない場合は None を返す。
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let lhs_end_x = self.pos_lu.x + self.size.x;
        let lhs_end_y = self.pos_lu.y + self.size.y;
        let rhs_end_x = other.pos_lu.x + other.size.x;
        let rhs_end_y = other.pos_lu.y + other.size.y;
        let start_x = self.pos_lu.x.max(other.pos_lu.x);
        let start_y = self.pos_lu.y.max(other.pos_lu.y);
        let end_x = lhs_end_x.min(rhs_end_x);
        let end_y = lhs_end_y.min(rhs_end_y);
        if start_x < end_x && start_y < end_y {
            Some(Rectangle {
                pos_lu: Vector2D {
                    x: start_x,
                    y: start_y,
                },
                size: Vector2D {
                    x: end_x - start_x,
                    y: end_y - start_y,
                },
            })
        } else {
            None
        }
    }
}

/// 例外発生時に画面へ直書きする為の設定とライター
pub static SCREEN_CONFIG: Global<Option<FrameBufferConfig>> = Global::new(None);

pub struct ScreenWriter {
    config: FrameBufferConfig,
}

impl ScreenWriter {
    pub fn new(config: FrameBufferConfig) -> Self {
        Self { config }
    }
}

impl PixelDraw for ScreenWriter {
    fn write(&self, x: usize, y: usize, c: &PixelColor) {
        if x >= self.config.horizontal_resolution as usize
            || y >= self.config.vertical_resolution as usize
        {
            return;
        }
        let offset = ((y * self.config.pixels_per_scan_line as usize + x) * 4) as isize;
        unsafe {
            let p = self.config.frame_buffer.offset(offset);
            match self.config.pixel_format {
                PixelFormat::kPixelRGBResv8BitPerColor => {
                    *p = c.r;
                    *p.add(1) = c.g;
                    *p.add(2) = c.b;
                }
                PixelFormat::kPixelBGRResv8BitPerColor => {
                    *p = c.b;
                    *p.add(1) = c.g;
                    *p.add(2) = c.r;
                }
            }
        }
    }
    fn width(&self) -> usize {
        self.config.horizontal_resolution as usize
    }
    fn height(&self) -> usize {
        self.config.vertical_resolution as usize
    }
}
