#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library
#![feature(abi_x86_interrupt)] // Allows extern "x86-interrupt" fn(). 
mod acpi;
mod allocator;
mod console;
mod font;
mod frame_buffer;
mod frame_buffer_config;
mod global;
mod graphics;
mod interrupts;
mod keyboard;
mod layer;
mod memory_manager;
mod memory_map;
mod message;
mod mousecursor;
mod paging;
mod pci;
mod queue;
mod segment;
mod timer;
mod usb;
mod window;
extern crate alloc;
use crate::acpi::Rsdp;
use crate::console::{Console, ConsoleWriter};
use crate::font::{write_ascii, write_string};
use crate::frame_buffer::FrameBuffer;
use crate::frame_buffer_config::FrameBufferConfig;
use crate::graphics::{
    PixelColor, PixelDraw, Rectangle, Vector2D, draw_rectrange, fill_rectrange, to_color,
};
use crate::interrupts::{INTERRUPTMESSAGE_QUEUE, init_idt};
use crate::keyboard::initialize_keyboard;
use crate::layer::{LAYER_MANAGER, LayerManager};
use crate::memory_manager::initialize_memory_manager;
use crate::memory_map::MemoryMap;
use crate::message::Message;
use crate::mousecursor::{MOUSE_CURSOR, kMouseCursorHeight, kMouseCursorWidth};
use crate::pci::initialize_pci;
use crate::timer::TIMER_MANAGER;
use crate::window::{Window, draw_textbox, draw_window};
use alloc::format;
use alloc::rc::Rc;
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
pub extern "C" fn KernelMain(
    frame_buffer_config: &FrameBufferConfig,
    memory_map: &MemoryMap,
    acpi_table: &acpi::Rsdp,
) {
    let kernel_main_stack_top_addr = KERNEL_STACK.0.as_ptr() as usize + (1024 * 1024);

    unsafe {
        asm!(
        "mov rsp, {stack}",
        "call {func}",
        stack=in(reg) kernel_main_stack_top_addr,
        func=sym KernelMainNewStack,
        in("rdi") frame_buffer_config,
        in("rsi") memory_map,
        in("rdx") acpi_table,
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

fn input_text_window(
    c: char,
    text_window: &Rc<RefCell<Window>>,
    text_window_index: &mut usize,
    text_window_layer_id: u32,
) {
    if c == '\0' {
        return;
    }

    {
        let mut text_window_ref = text_window.borrow_mut();
        let max_chars = (text_window_ref.width() - 16) / 8;
        let text_window_writer = text_window_ref.writer();

        let pos = |index: usize| Vector2D {
            x: 8 + 8 * index,
            y: 24 + 6,
        };
        if c == '\x08' {
            if *text_window_index > 0 {
                *text_window_index -= 1;
                fill_rectrange(
                    &text_window_writer,
                    &pos(*text_window_index),
                    &Vector2D { x: 8, y: 16 },
                    &to_color(0xffffff),
                );
            }
        } else if c >= ' ' && *text_window_index < max_chars {
            let p = pos(*text_window_index);
            write_ascii(&text_window_writer, p.x, p.y, c as u8, &to_color(0));
            *text_window_index += 1;
        }
    }

    redraw_layers_id(text_window_layer_id);
}

fn KernelMainNewStack(
    frame_buffer_config: &FrameBufferConfig,
    memory_map: &MemoryMap,
    acpi_table: &acpi::Rsdp,
) -> ! {
    segment::init_segment();
    paging::setup_identify_page_table();

    initialize_memory_manager(&memory_map);
    allocator::init_heap();
    init_idt();
    acpi::initialize(acpi_table);

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

    let console_window = Rc::new(RefCell::new(Window::new(
        Console::COLUMN_NUM * 8,
        Console::ROW_NUM * 16,
        frame_buffer_config.pixel_format,
    )));

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

    let mut console = Console::new(
        ConsoleWriter::Window(console_window.clone()),
        DESKTOP_FGCOLOR,
        DESKTOP_BGCOLOR,
    );
    {
        let mut cw_ref = console_window.borrow_mut();
        let cw_writer = cw_ref.writer();
        fill_rectrange(
            &cw_writer,
            &Vector2D { x: 0, y: 0 },
            &Vector2D {
                x: Console::COLUMN_NUM * 8,
                y: Console::ROW_NUM * 16,
            },
            &DESKTOP_BGCOLOR,
        );
    }
    console.set_layerid(
        layer_manager
            .new_layer()
            .set_window(console_window)
            .move_to(Vector2D { x: 0, y: 0 })
            .id,
    );
    console.set_redraw(redraw_layers_id);

    layer_manager.up_down(bg_layer_id, 0);
    layer_manager.up_down(mouse_layer_id, 3);
    layer_manager.up_down(main_window_id, 2);
    layer_manager.up_down(console.get_layer_id(), 1);

    let text_window = Rc::new(RefCell::new(Window::new(
        160,
        52,
        frame_buffer_config.pixel_format,
    )));
    draw_window(&text_window.borrow_mut().writer(), "Text Box Test");
    draw_textbox(
        &text_window.borrow_mut().writer(),
        Vector2D { x: 4, y: 24 },
        Vector2D {
            x: 160 - 8,
            y: 52 - 24 - 4,
        },
    );
    let text_window_id = layer_manager
        .new_layer()
        .set_window(text_window.clone())
        .set_draggable(true)
        .move_to(Vector2D { x: 350, y: 200 })
        .id;
    layer_manager.up_down(text_window_id, 4);

    LAYER_MANAGER.with(|lm| *lm = Some(layer_manager));
    redraw_layers_all();
    console::init(console);

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

    initialize_keyboard(&INTERRUPTMESSAGE_QUEUE);

    initialize_pci();

    timer::initialize_lapic_timer(&INTERRUPTMESSAGE_QUEUE);

    TIMER_MANAGER.with(|tm| tm.as_mut().unwrap().add_timer(&timer::Timer::new(200, 2)));

    TIMER_MANAGER.with(|tm| tm.as_mut().unwrap().add_timer(&timer::Timer::new(700, -1)));

    let mut text_window_index: usize = 0;
    loop {
        unsafe { asm!("cli") };
        let tick = TIMER_MANAGER.with(|tm| tm.as_ref().map_or(0, |tm| tm.get_tick()));
        unsafe { asm!("sti") };

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
            &format!("{:>10}", tick),
            &PixelColor { r: 0, g: 140, b: 0 },
        );
        redraw_layers_id(main_window_id);

        x86_64::instructions::interrupts::disable();
        if INTERRUPTMESSAGE_QUEUE.with(|q| q.count() == 0) {
            x86_64::instructions::interrupts::enable_and_hlt();
            continue;
        }
        let msg = INTERRUPTMESSAGE_QUEUE.with(|q| q.pop());
        match msg {
            Some(Message::InterruptXhci) => {
                x86_64::instructions::interrupts::enable();
                usb::xhci::ProcessEvents();
            }
            Some(Message::TimerTimeout { .. }) => {
                x86_64::instructions::interrupts::enable();
            }
            Some(Message::KeyPush {
                keycode: _,
                modifier: _,
                ascii,
            }) => {
                x86_64::instructions::interrupts::enable();
                input_text_window(ascii, &text_window, &mut text_window_index, text_window_id);
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
