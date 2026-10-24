#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library
#![feature(abi_x86_interrupt)] // Allows extern "x86-interrupt" fn(). 
mod acpi;
mod allocator;
mod console;
mod elf;
mod error;
mod fat;
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
mod task;
mod terminal;
mod timer;
mod usb;
mod window;
extern crate alloc;
use crate::console::{Console, ConsoleWriter, printk};
use crate::font::{write_ascii, write_string};
use crate::frame_buffer::FrameBuffer;
use crate::frame_buffer_config::FrameBufferConfig;
use crate::graphics::{
    PixelColor, PixelDraw, Rectangle, Vector2D, draw_rectrange, fill_rectrange, to_color,
};
use crate::interrupts::init_idt;
use crate::keyboard::initialize_keyboard;
use crate::layer::{LAYER_MANAGER, LayerManager};
use crate::memory_manager::initialize_memory_manager;
use crate::memory_map::MemoryMap;
use crate::message::Message;
use crate::mousecursor::{MOUSE_CURSOR, kMouseCursorHeight, kMouseCursorWidth};
use crate::pci::{initialize_pci, initialize_usb};
use crate::task::{TASK_MANAGER, initialize_task, receive_message, send_message, sleep, wakeup};
use crate::timer::{TIMER_FREQ, TIMER_MANAGER};
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
    volume_image: *const u8,
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
        in("rcx") volume_image,
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

fn draw_text_cursor(writer: &impl PixelDraw, text_window_index: usize, visible: bool) {
    let pos = Vector2D {
        x: 4 + 8 * text_window_index,
        y: 5,
    };
    let color = if visible {
        &to_color(0)
    } else {
        &to_color(0xffffff)
    };
    fill_rectrange(writer, &pos, &Vector2D { x: 7, y: 15 }, color);
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

    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut text_window_ref = text_window.borrow_mut();
        let max_chars = (text_window_ref.width() - 8) / 8 - 1;
        let text_window_writer = text_window_ref.inner_writer();

        let pos = |index: usize| Vector2D {
            x: 4 + 8 * index,
            y: 6,
        };
        if c == '\x08' {
            if *text_window_index > 0 {
                draw_text_cursor(&text_window_writer, *text_window_index, false);
                *text_window_index -= 1;
                fill_rectrange(
                    &text_window_writer,
                    &pos(*text_window_index),
                    &Vector2D { x: 8, y: 16 },
                    &to_color(0xffffff),
                );
                draw_text_cursor(&text_window_writer, *text_window_index, true);
            }
        } else if c >= ' ' && *text_window_index < max_chars {
            draw_text_cursor(&text_window_writer, *text_window_index, false);
            let p = pos(*text_window_index);
            write_ascii(&text_window_writer, p.x, p.y, c as u8, &to_color(0));
            *text_window_index += 1;
            draw_text_cursor(&text_window_writer, *text_window_index, true);
        }
    });

    redraw_layers_id(text_window_layer_id);
}

fn KernelMainNewStack(
    frame_buffer_config: &FrameBufferConfig,
    memory_map: &MemoryMap,
    acpi_table: &acpi::Rsdp,
    volume_image: *const u8,
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
    let main_window = Rc::new(RefCell::new(Window::new_toplevel(
        160,
        52,
        frame_buffer_config.pixel_format,
        "Hello Window",
    )));

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

    let text_window = Rc::new(RefCell::new(Window::new_toplevel(
        160,
        52,
        frame_buffer_config.pixel_format,
        "Text Box TaskContext",
    )));
    let text_box_size = text_window.borrow().inner_size();
    draw_textbox(
        &text_window.borrow_mut().inner_writer(),
        Vector2D { x: 0, y: 0 },
        text_box_size,
    );
    let text_window_id = layer_manager
        .new_layer()
        .set_window(text_window.clone())
        .set_draggable(true)
        .move_to(Vector2D { x: 500, y: 100 })
        .id;
    layer_manager.up_down(text_window_id, 4);

    layer_manager.up_down(mouse_layer_id, 10);

    LAYER_MANAGER.with(|lm| *lm = Some(layer_manager));
    layer::set_mouse_layer(mouse_layer_id);
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

    // 色々初期化
    fat::initialize(volume_image);

    initialize_pci(); //バス走査のみ

    timer::initialize_lapic_timer();

    TIMER_MANAGER.with(|tm| tm.as_mut().unwrap().add_timer(&timer::Timer::new(200, 2)));

    TIMER_MANAGER.with(|tm| tm.as_mut().unwrap().add_timer(&timer::Timer::new(700, -1)));

    // まだxhci走っていないのでcli/stiは不要
    let mut text_window_index: usize = 0;
    let textbox_cursor_timer = 1;
    let timer_50sec = (TIMER_FREQ as f64 * 0.5) as u64;
    TIMER_MANAGER.with(|tm| {
        tm.as_mut()
            .unwrap()
            .add_timer(&timer::Timer::new(timer_50sec, textbox_cursor_timer));
    });
    let mut textbox_cursor_visible = false;

    // TaskB初期化

    initialize_task();

    x86_64::instructions::interrupts::enable();

    let task_terminal_id = TASK_MANAGER.with(|tm| {
        let tm = tm.as_mut().unwrap();
        let id = tm.new_task().init_context(terminal::task_terminal, 0).id();
        tm.wakeup(id, None).unwrap();
        id
    });

    let main_task_id = TASK_MANAGER.with(|tm| tm.as_ref().unwrap().current_task_id());
    // end TaskB初期化

    initialize_keyboard();
    initialize_usb();

    loop {
        unsafe { asm!("cli") };
        let tick = TIMER_MANAGER.with(|tm| tm.as_ref().map_or(0, |tm| tm.get_tick()));
        unsafe { asm!("sti") };
        x86_64::instructions::interrupts::without_interrupts(|| {
            fill_rectrange(
                &main_window.borrow_mut().inner_writer(),
                &Vector2D {
                    x: 20 as usize,
                    y: 4 as usize,
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
                &main_window.borrow_mut().inner_writer(),
                20,
                4,
                &format!("{:>10}", tick),
                &PixelColor { r: 0, g: 140, b: 0 },
            );
        });
        redraw_layers_id(main_window_id);

        x86_64::instructions::interrupts::disable();
        let Some(msg) = receive_message(main_task_id) else {
            let _ = sleep(main_task_id);
            x86_64::instructions::interrupts::enable();
            continue;
        };
        x86_64::instructions::interrupts::enable();
        match msg {
            Message::InterruptXhci => {
                usb::xhci::ProcessEvents();
            }
            Message::TimerTimeout {
                timeout,
                value: timer_id,
            } => {
                if timer_id == textbox_cursor_timer {
                    unsafe { asm!("cli") };
                    TIMER_MANAGER.with(|tm| {
                        tm.as_mut().unwrap().add_timer(&timer::Timer::new(
                            timeout + timer_50sec,
                            textbox_cursor_timer,
                        ));
                    });
                    unsafe { asm!("sti") };
                    textbox_cursor_visible = !textbox_cursor_visible;
                    x86_64::instructions::interrupts::without_interrupts(|| {
                        draw_text_cursor(
                            &text_window.borrow_mut().inner_writer(),
                            text_window_index,
                            textbox_cursor_visible,
                        );
                    });
                    redraw_layers_id(text_window_id);
                }
                let _ = send_message(task_terminal_id, msg);
            }
            Message::KeyPush {
                keycode,
                modifier: _,
                ascii,
            } => {
                let act = layer::get_active();
                if act == text_window_id {
                    input_text_window(ascii, &text_window, &mut text_window_index, text_window_id);
                } else if let Some(target_task_id) = layer::get_task_by_layer(act) {
                    let _ = send_message(target_task_id, msg);
                } else {
                    printk!(
                        "key push not handled: keycode: {:02x}, ascii {:02x}\n",
                        keycode,
                        ascii as u8
                    );
                }
            }
            Message::Layer {
                op,
                layer_id,
                src_task,
            } => {
                layer::process_layer_message(op, layer_id);
                let _ = send_message(src_task, Message::LayerFinish);
            }
            Message::LayerFinish => {}
        }
    }
}

// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    loop {
        unsafe {
            asm!("Hlt");
        }
    }
}
