#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library
#![feature(abi_x86_interrupt)] // Allows extern "x86-interrupt" fn(). 
mod allocator;
mod console;
mod font;
mod frame_buffer;
mod frame_buffer_config;
mod global;
mod graphics;
mod interrupts;
mod layer;
mod memory_manager;
mod memory_map;
mod mousecursor;
mod paging;
mod pci;
mod queue;
mod segment;
mod timer;
mod usb;
mod window;
extern crate alloc;
use crate::font::write_string;
use crate::frame_buffer::FrameBuffer;
use crate::frame_buffer_config::FrameBufferConfig;
use crate::pci::initialize_pci;
use alloc::format;
use alloc::rc::Rc;

use crate::graphics::{PixelColor, PixelDraw, Rectangle, Vector2D, draw_rectrange, fill_rectrange};
use crate::interrupts::{INTERRUPTMESSAGE_QUEUE, init_idt};
use crate::layer::{LAYER_MANAGER, LayerManager};
use crate::memory_manager::{
    BYTES_PER_FRAME, FrameID, MEMORY_MANAGER, UEFI_PAGE_SIZE, initialize_memory_manager,
};
use crate::memory_map::{MemoryDescriptor, MemoryMap};
use crate::mousecursor::{MOUSE_CURSOR, kMouseCursorHeight, kMouseCursorWidth};
use crate::window::{Window, draw_window};
use core::arch::asm;
use core::cell::RefCell;
use core::panic::PanicInfo;

const DESKTOP_BGCOLOR: PixelColor = PixelColor {
    r: 120,
    g: 255,
    b: 125,
};
const DESKTOP_FGCOLOR: PixelColor = PixelColor { r: 1, g: 8, b: 17 };
#[repr(align(16))]
struct KernelStack([u8; 1024 * 1024]);

// exclusive stack for kernel
static KERNEL_STACK: KernelStack = KernelStack([0; 1024 * 1024]);

/// 全レイヤーを再描画する (LAYER_MANAGER 初期化前は何もしない)。
pub fn redraw_layers_all() {
    LAYER_MANAGER.with(|lm| {
        if let Some(lm) = lm {
            lm.draw_all();
        }
    });
}

/// レイヤー `id` の領域を再描画する (LAYER_MANAGER 初期化前は何もしない)。
pub fn redraw_layers_id(id: u32) {
    LAYER_MANAGER.with(|lm| {
        if let Some(lm) = lm {
            lm.draw_id(id);
        }
    });
}

/// マウスカーソルの移動前後の領域を再描画する。
pub fn redraw_mouse_area(old_pos: Vector2D<i32>, new_pos: Vector2D<i32>) {
    let left = old_pos.x.min(new_pos.x);
    let top = old_pos.y.min(new_pos.y);
    let right = (old_pos.x + kMouseCursorWidth as i32).max(new_pos.x + kMouseCursorWidth as i32);
    let bottom = (old_pos.y + kMouseCursorHeight as i32).max(new_pos.y + kMouseCursorHeight as i32);
    LAYER_MANAGER.with(|lm| {
        if let Some(lm) = lm {
            lm.draw_area(Rectangle {
                pos_lu: Vector2D { x: left, y: top },
                size: Vector2D {
                    x: right - left,
                    y: bottom - top,
                },
            });
        }
    });
}

// must wrap the attribute 'no_mangle' by unsafe(...)
#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn KernelMain(frame_buffer_config: &FrameBufferConfig, memory_map: &MemoryMap) {
    let kernel_main_stack_top_addr = KERNEL_STACK.0.as_ptr() as usize + (1024 * 1024);

    unsafe {
        asm!(
        "mov rsp, {stack}",
        "call {func}",
        stack=in(reg) kernel_main_stack_top_addr,
        func=sym KernelMainNewStack,
        in("rdi") frame_buffer_config,
        in("rsi") memory_map,
        options(noreturn)
            );
    }
}

fn draw_desktop<W: PixelDraw>(pixel_writer: &W, frame_width: usize, frame_height: usize) {
    fill_rectrange(
        pixel_writer,
        &Vector2D { x: 0, y: 0 },
        &Vector2D {
            x: frame_width,
            y: frame_height,
        },
        &PixelColor { r: 255, g: 0, b: 0 },
    );
    fill_rectrange(
        pixel_writer,
        &Vector2D { x: 0, y: 0 },
        &Vector2D {
            x: frame_width,
            y: frame_height - 50,
        },
        &DESKTOP_BGCOLOR,
    );
    fill_rectrange(
        pixel_writer,
        &Vector2D {
            x: 0,
            y: frame_height - 50,
        },
        &Vector2D {
            x: frame_width,
            y: 50,
        },
        &DESKTOP_FGCOLOR,
    );

    fill_rectrange(
        pixel_writer,
        &Vector2D {
            x: 0,
            y: frame_height - 50,
        },
        &Vector2D {
            x: frame_width / 5,
            y: 50,
        },
        &PixelColor {
            r: 80,
            g: 80,
            b: 80,
        },
    );

    draw_rectrange(
        pixel_writer,
        &Vector2D {
            x: 10,
            y: frame_height - 40,
        },
        &Vector2D { x: 30, y: 30 },
        &PixelColor {
            r: 160,
            g: 160,
            b: 160,
        },
    );
}

fn KernelMainNewStack(frame_buffer_config: &FrameBufferConfig, memory_map: &MemoryMap) -> ! {
    segment::init_segment();
    paging::setup_identify_page_table();

    initialize_memory_manager(&memory_map);
    allocator::init_heap();
    timer::initialize_lapic_timer();
    init_idt();

    let frame_width: usize = frame_buffer_config.horizontal_resolution as usize;
    let frame_height: usize = frame_buffer_config.vertical_resolution as usize;
    let frame_buffer = FrameBuffer::new(*frame_buffer_config);

    let mut bg_window = Window::new(frame_width, frame_height, frame_buffer_config.pixel_format);

    let bg_writer = bg_window.writer();

    draw_desktop(&bg_writer, frame_width, frame_height);

    let bg_window = Rc::new(RefCell::new(bg_window));
    let mouse_window = Rc::new(RefCell::new(Window::new(
        kMouseCursorWidth,
        kMouseCursorHeight,
        frame_buffer_config.pixel_format,
    )));
    let main_window = Rc::new(RefCell::new(Window::new(
        160,
        52,
        frame_buffer_config.pixel_format,
    )));
    draw_window(&main_window.borrow_mut().writer(), "Hello window");

    let mut layer_manager: LayerManager = LayerManager::new(frame_buffer);
    let bg_layer_id = layer_manager
        .new_layer()
        .set_window(bg_window.clone())
        .move_to(Vector2D { x: 0, y: 0 })
        .id;
    let mouse_layer_id = layer_manager
        .new_layer()
        .set_window(mouse_window.clone())
        .move_to(Vector2D { x: 200, y: 300 })
        .id;
    let main_window_id = layer_manager
        .new_layer()
        .set_window(main_window.clone())
        .set_draggable(true)
        .id;

    layer_manager.up_down(bg_layer_id, 0);
    layer_manager.up_down(mouse_layer_id, 3);
    layer_manager.up_down(main_window_id, 2);

    LAYER_MANAGER.with(|lm| *lm = Some(layer_manager));
    redraw_layers_all();

    printk!("Welcome to tettya OS!!\n");

    unsafe {
        MOUSE_CURSOR.with(|cursor| {
            cursor.init(
                mouse_layer_id,
                Vector2D {
                    x: frame_width,
                    y: frame_height,
                },
                redraw_layers_all,
                redraw_mouse_area,
            );
            mouse_window
                .borrow_mut()
                .set_transparent_color(Some(crate::mousecursor::MOUSE_CURSOR_TRANSPARENT_COLOR));
            {
                let mut mouse_window_ref = mouse_window.borrow_mut();
                let mouse_writer = mouse_window_ref.writer();
                fill_rectrange(
                    &mouse_writer,
                    &Vector2D { x: 0, y: 0 },
                    &Vector2D {
                        x: kMouseCursorWidth,
                        y: kMouseCursorHeight,
                    },
                    &crate::mousecursor::MOUSE_CURSOR_TRANSPARENT_COLOR,
                );
                cursor.draw(&mouse_writer);
            }
        });
        redraw_layers_all();
    }

    initialize_pci();

    let mut window_cnt = 0;
    loop {
        fill_rectrange(
            &main_window.borrow_mut().writer(),
            &Vector2D {
                x: 24 as usize,
                y: 28 as usize,
            },
            &Vector2D {
                x: 8 * 10 as usize,
                y: 16 as usize,
            },
            &PixelColor {
                r: 0xc6,
                g: 0xc6,
                b: 0xc6,
            },
        );
        write_string(
            &main_window.borrow_mut().writer(),
            24,
            28,
            &format!("{window_cnt}"),
            &PixelColor { r: 0, g: 0, b: 0 },
        );
        window_cnt += 1;
        redraw_layers_id(main_window_id);
        // キューの確認と hlt の間に割り込みが入ると取りこぼすため、
        // 割り込み禁止のまま確認し、無ければ `sti; hlt` (sti 直後の1命令は割り込まれない) で待つ。
        x86_64::instructions::interrupts::disable();
        let msg = INTERRUPTMESSAGE_QUEUE.with(|q| q.pop());
        match msg {
            Some(interrupts::InterruptMessage::InterruptXhci) => {
                x86_64::instructions::interrupts::enable();
                usb::xhci::ProcessEvents();
            }
            None => x86_64::instructions::interrupts::enable_and_hlt(),
        }
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("Hlt");
        }
    }
}
