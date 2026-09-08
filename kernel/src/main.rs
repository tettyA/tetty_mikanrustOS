#![no_main] // disable all Rust-level entry points
#![no_std] // don't lonk the Rust standart library
#![feature(abi_x86_interrupt)] // Allows extern "x86-interrupt" fn(). 
mod allocator;
mod console;
mod font;
mod frame_buffer;
mod frame_buffer_config;
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
use crate::console::{Console, ConsoleWriter};
use crate::font::write_string;
use crate::frame_buffer::FrameBuffer;
use crate::frame_buffer_config::FrameBufferConfig;
use alloc::format;
use alloc::rc::Rc;

use crate::console::printk;
use crate::graphics::{
    PixelColor, PixelDraw, PixelWriter, Rectrange, Vector2D, draw_rectrange, fill_rectrange,
};
use crate::interrupts::{INTERRUPTMESSAGE_QUEUE, init_idt};
use crate::layer::LayerManager;
use crate::memory_manager::{BYTES_PER_FRAME, FrameID, MEMORY_MANAGER, UEFI_PAGE_SIZE};
use crate::memory_map::{MemoryDescriptor, MemoryMap};
use crate::mousecursor::{MOUSE_CURSOR, kMouseCursorHeight, kMouseCursorWidth};
use crate::window::{Window, draw_window};
use core::arch::asm;
use core::cell::RefCell;
use core::panic::PanicInfo;
use core::ptr::addr_of_mut;

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
static mut LAYER_MANAGER: *mut LayerManager = core::ptr::null_mut();
static mut FRAME_BUFFER: *mut FrameBuffer = core::ptr::null_mut();

pub fn redraw_layers_all() {
    unsafe {
        if !LAYER_MANAGER.is_null() && !FRAME_BUFFER.is_null() {
            (*LAYER_MANAGER).draw_all();
        }
    }
}

pub fn redraw_layers_id(id: u32) {
    unsafe {
        if !LAYER_MANAGER.is_null() && !FRAME_BUFFER.is_null() {
            (*LAYER_MANAGER).draw_id(id);
        }
    }
}

pub fn redraw_mouse_area(old_pos: Vector2D<i32>, new_pos: Vector2D<i32>) {
    unsafe {
        if !LAYER_MANAGER.is_null() && !FRAME_BUFFER.is_null() {
            let left = old_pos.x.min(new_pos.x);
            let top = old_pos.y.min(new_pos.y);
            let right =
                (old_pos.x + kMouseCursorWidth as i32).max(new_pos.x + kMouseCursorWidth as i32);
            let bottom =
                (old_pos.y + kMouseCursorHeight as i32).max(new_pos.y + kMouseCursorHeight as i32);
            (*LAYER_MANAGER).draw_area(Rectrange {
                pos_lu: Vector2D { x: left, y: top },
                size: Vector2D {
                    x: right - left,
                    y: bottom - top,
                },
            });
        }
    }
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

    //compile_error!("メモリマップ実装してない(8.16)");
    unsafe {
        let memory_manager = &mut *addr_of_mut!(MEMORY_MANAGER);
        let mut available_end = 0 as usize;
        for i in 0..(memory_map.map_size / memory_map.descriptor_size) {
            let ptr = memory_map
                .buffer
                .add((i * memory_map.descriptor_size) as usize);
            let desc: &MemoryDescriptor = &*(ptr as *const MemoryDescriptor);
            if available_end < desc.physical_start as usize {
                memory_manager.mark_allocated(
                    FrameID {
                        id: available_end as usize / BYTES_PER_FRAME,
                    },
                    (desc.physical_start as usize - available_end as usize) / BYTES_PER_FRAME,
                );
            }
            let physical_end =
                desc.physical_start as usize + desc.number_of_pages as usize * UEFI_PAGE_SIZE;
            if desc.memory_type().is_available() {
                available_end = physical_end;
            } else {
                memory_manager.mark_allocated(
                    FrameID {
                        id: desc.physical_start as usize / BYTES_PER_FRAME,
                    },
                    desc.number_of_pages as usize * UEFI_PAGE_SIZE / BYTES_PER_FRAME,
                );
            }
        }
        memory_manager.set_memory_range(
            FrameID { id: 0 },
            FrameID {
                id: available_end / BYTES_PER_FRAME,
            },
        );
    }

    allocator::init_heap();
    timer::initialize_lapic_timer();
    init_idt();

    let frame_width: usize = frame_buffer_config.horizontal_resolution as usize;
    let frame_height: usize = frame_buffer_config.vertical_resolution as usize;
    let mut frame_buffer = FrameBuffer::new(*frame_buffer_config);

    let mut bg_window = Window::new(frame_width, frame_height, frame_buffer_config.pixel_format);

    let bg_writer = bg_window.writer();

    draw_desktop(&bg_writer, frame_width, frame_height);

    // entrypoint

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

    let mut layer_manager: LayerManager = LayerManager::new(core::ptr::addr_of_mut!(frame_buffer));
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
    let main_window_id = layer_manager.new_layer().set_window(main_window.clone()).id;

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
    layer_manager.up_down(main_window_id, 1);
    layer_manager.up_down(console.get_layer_id(), 2);

    unsafe {
        LAYER_MANAGER = &mut layer_manager;
        FRAME_BUFFER = &mut frame_buffer;
    }
    layer_manager.draw_all();
    console::init(console);

    printk!("Welcome to tettya OS!!\n");

    unsafe {
        let cursor = &mut *addr_of_mut!(MOUSE_CURSOR);
        cursor.init(
            &mut layer_manager,
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
        redraw_layers_all();
    }

    unsafe {
        crate::usb::usb_set_mouse_obserever(crate::mousecursor::moverelative);
        crate::usb::usb_enumarate();
        asm!("sti");
    }
    printk!("Success ScanAllBus\n");

    printk!("memory_map: {:p}\n", &memory_map);
    for i in 0..(memory_map.map_size / memory_map.descriptor_size) {
        let desc: &MemoryDescriptor = unsafe {
            let ptr = memory_map
                .buffer
                .add((i * memory_map.descriptor_size) as usize);
            &*(ptr as *const MemoryDescriptor)
        };
        if desc.memory_type().is_available() {
            printk!(
                "type = {:?}, phys = {:08x} - {:08x}, pages = {}, attr = {:x}\n",
                desc.memory_type(),
                desc.physical_start,
                desc.physical_start + desc.number_of_pages * 4096 - 1,
                desc.number_of_pages,
                desc.attribute
            );
        }
    }
    let mut window_cnt = 0;
    loop {
        unsafe {
            // compile_error!("osbook_day10c 途中 タイマ表示したい。");
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
            if crate::usb::xhc_primaryeventring_hasfront() {
                crate::usb::usb_process_events();
            }
            asm!("cli");
            let q = &mut *addr_of_mut!(INTERRUPTMESSAGE_QUEUE);

            if let Some(msg) = q.pop() {
                asm!("sti");
                match msg {
                    interrupts::InterruptMessage::InterruptXhci => {
                        while crate::usb::xhc_primaryeventring_hasfront() {
                            crate::usb::usb_process_events();
                        }
                    }
                }
            } else {
                // asm!("sti\n\thlt");
                asm!("sti");
                continue;
            }
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
