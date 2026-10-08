#[repr(C)]
#[derive(PartialEq, Clone, Copy)]
#[allow(non_camel_case_types)] // 本家 MikanOS と名前を揃えているため
pub enum PixelFormat {
    kPixelRGBResv8BitPerColor,
    kPixelBGRResv8BitPerColor,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct FrameBufferConfig {
    pub frame_buffer: *mut u8,
    pub pixels_per_scan_line: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: PixelFormat,
}
