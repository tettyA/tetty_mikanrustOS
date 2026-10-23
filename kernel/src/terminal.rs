use alloc::{collections::VecDeque, format, rc::Rc};
use core::cell::RefCell;

use crate::{
    font::{write_ascii, write_string},
    graphics::{Rectangle, Vector2D, fill_rectrange, to_color},
    layer::{LAYER_MANAGER, LayerManager},
    message::{LayerOperation, Message},
    pci, printk, redraw_layers_id,
    task::{self, receive_message},
    window::{MARGIN_X, MARGIN_Y, TOP_LEFT_MARGIN, Window, draw_terminal},
};

pub struct Terminal {
    window: Rc<RefCell<Window>>,
    layer_id: u32,
    cursor: Vector2D<usize>,
    cursor_visible: bool,
    line_buf: [u8; Self::LINE_MAX],
    linebuf_index: usize,
    cmd_history: VecDeque<[u8; Self::LINE_MAX]>,
    cmd_history_index: i32,
}

impl Terminal {
    pub const ROWS: usize = 15;
    pub const COLUMNS: usize = 60;
    pub const LINE_MAX: usize = 128;

    pub fn new() -> Self {
        let (layer_id, window) = LAYER_MANAGER.with(|lm| {
            let lm = lm.as_mut().unwrap();
            let pixel_format = lm.pixel_format();
            let width = Self::COLUMNS * 8 + 8 + MARGIN_X;
            let height = Self::ROWS * 16 + 8 + MARGIN_Y;
            let window = Rc::new(RefCell::new(Window::new_toplevel(
                width,
                height,
                pixel_format,
                "tettyATerm",
            )));
            let inner_size = window.borrow().inner_size();

            draw_terminal(
                &window.borrow_mut().inner_writer(),
                Vector2D { x: 0, y: 0 },
                inner_size,
            );

            let layer_id = lm
                .new_layer()
                .set_window(window.clone())
                .set_draggable(true)
                .id;

            (layer_id, window)
        });

        let mut cmd_history = VecDeque::with_capacity(8);
        cmd_history.resize(8, [0; Self::LINE_MAX]);

        let mut term = Self {
            window,
            layer_id,
            cursor: Vector2D { x: 0, y: 0 },
            cursor_visible: false,
            line_buf: [0; Self::LINE_MAX],
            linebuf_index: 0,
            cmd_history,
            cmd_history_index: -1,
        };
        term.print(">");

        term
    }

    pub fn layer_id(&self) -> u32 {
        self.layer_id
    }

    fn calc_cursor_pos(&self) -> Vector2D<usize> {
        Vector2D {
            x: TOP_LEFT_MARGIN.x + 4 + 8 * self.cursor.x,
            y: TOP_LEFT_MARGIN.y + 4 + 16 * self.cursor.y,
        }
    }

    pub fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.cursor_visible = !self.cursor_visible;

        self.draw_cursor(self.cursor_visible);

        Rectangle {
            pos_lu: Vector2D {
                x: self.calc_cursor_pos().x as i32,
                y: self.calc_cursor_pos().y as i32,
            },
            size: Vector2D { x: 7, y: 15 },
        }
    }

    fn draw_cursor(&mut self, visible: bool) {
        let color = if visible {
            to_color(0xffffff)
        } else {
            to_color(0x000000)
        };

        let pos = self.calc_cursor_pos();

        crate::graphics::fill_rectrange(
            &self.window.borrow_mut().writer(),
            &pos,
            &Vector2D { x: 7, y: 15 },
            &color,
        );
    }

    fn scroll1(&mut self) {
        let move_src = Rectangle {
            pos_lu: Vector2D {
                x: (TOP_LEFT_MARGIN.x + 4) as i32,
                y: (TOP_LEFT_MARGIN.y + 4 + 16) as i32,
            },
            size: Vector2D {
                x: (8 * Self::COLUMNS) as i32,
                y: (16 * (Self::ROWS - 1)) as i32,
            },
        };

        self.window.borrow_mut().window_move(
            Vector2D {
                x: (TOP_LEFT_MARGIN.x + 4) as i32,
                y: (TOP_LEFT_MARGIN.y + 4) as i32,
            },
            &move_src,
        );

        fill_rectrange(
            &self.window.borrow_mut().inner_writer(),
            &Vector2D {
                x: 4,
                y: 4 + 16 * self.cursor.y,
            },
            &Vector2D {
                x: 8 * Self::COLUMNS,
                y: 16,
            },
            &to_color(0),
        );
    }

    fn newline(&mut self) {
        self.cursor.x = 0;
        if self.cursor.y < Self::ROWS - 1 {
            self.cursor.y += 1;
        } else {
            self.scroll1();
        }
    }
    pub fn print_char(&mut self, c: u8) {
        if c == b'\n' {
            self.newline();
        } else {
            let pos = self.calc_cursor_pos();
            write_ascii(
                &self.window.borrow_mut().writer(),
                pos.x,
                pos.y,
                c,
                &to_color(0xffffff),
            );

            if self.cursor.x == Self::COLUMNS - 1 {
                self.newline();
            } else {
                self.cursor.x += 1;
            }
        }
    }

    pub fn print(&mut self, s: &str) {
        self.draw_cursor(false);

        for &c in s.as_bytes() {
            self.print_char(c);
        }

        self.draw_cursor(true);
    }

    fn execute_line(&mut self) {
        let line_buf = self.line_buf;
        let linebuf_index = self.linebuf_index;
        self.linebuf_index = 0;
        let line = core::str::from_utf8(&line_buf[..linebuf_index]).unwrap_or("");
        let (command, first_arg) = match line.split_once(' ') {
            Some((cmd, arg)) => (cmd, Some(arg)),
            None => (line, None),
        };
        if command == "echo" {
            if let Some(arg) = first_arg {
                self.print(arg);
            }
            self.print("\n");
        } else if command == "clear" {
            fill_rectrange(
                &self.window.borrow_mut().inner_writer(),
                &Vector2D { x: 4, y: 4 },
                &Vector2D {
                    x: 8 * Self::COLUMNS,
                    y: 16 * Self::ROWS,
                },
                &to_color(0),
            );
            self.cursor.y = 0;
        } else if command == "lspci" {
            for i in 0..pci::num_device() {
                let dev = pci::devices(i);
                let vendor_id = pci::ReadVendorId(&dev);

                let s = format!(
                    "{:02x}:{:02x}.{} vend={:04x} head={:02x} class={:02x}.{:02x}.{:02x}\n",
                    dev.bus,
                    dev.device,
                    dev.function,
                    vendor_id,
                    dev.header_type,
                    dev.class_code.base,
                    dev.class_code.sub,
                    dev.class_code.interface
                );
                self.print(&s);
            }
        } else if command == "ls" {
            let bpb = crate::fat::boot_volume_image();
            let root_dir_entries = unsafe {
                let ptr = crate::fat::get_sector_by_cluster::<crate::fat::DirectoryEntry>(
                    bpb.root_cluster as usize,
                );
                let count = (bpb.bytes_per_sector as usize
                    / core::mem::size_of::<crate::fat::DirectoryEntry>())
                    * bpb.sectors_per_cluster as usize;
                core::slice::from_raw_parts(ptr, count)
            };

            let mut base = [0u8; 9];
            let mut ext = [0u8; 4];

            for entry in root_dir_entries {
                crate::fat::read_name(entry, &mut base, &mut ext);
                if base[0] == 0x00 {
                    // このディレクトリエントリは空いており，これより後ろに有効なディレクトリエントリは存在しない
                    break;
                } else if base[0] == 0xe5 {
                    // このディレクトリは空いており，どんなファイルやディレクトリも表さない。
                    continue;
                } else if entry.attr == crate::fat::Attribute::LongName {
                    // ロングファイル名エントリは飛ばす
                    continue;
                }

                let base_len = base.iter().position(|&c| c == 0).unwrap_or(0);
                let ext_len = ext.iter().position(|&c| c == 0).unwrap_or(0);
                let base_str = core::str::from_utf8(&base[..base_len]).unwrap_or("");
                let ext_str = core::str::from_utf8(&ext[..ext_len]).unwrap_or("");

                if entry.attr == crate::fat::Attribute::VolumeId {
                    // ボリュームラベルは 8.3 形式ではなく， 11 文字の単一の文字列
                    let name = entry.name;
                    let len = name
                        .iter()
                        .rposition(|&c| c != b' ' && c != 0)
                        .map_or(0, |i| i + 1);
                    let name_str = core::str::from_utf8(&name[..len]).unwrap_or("");
                    let s = format!("{}\n", name_str);
                    self.print(&s);
                } else if !ext_str.is_empty() {
                    let s = format!("{}.{}\n", base_str, ext_str);
                    self.print(&s);
                } else {
                    let s = format!("{}\n", base_str);
                    self.print(&s);
                }
            }
        } else if command == "cat" {
            let file_name = first_arg.unwrap_or("");
            let file_entry = crate::fat::find_file(file_name, 0);

            if let Some(entry) = file_entry {
                let mut cluster = entry.first_cluster() as usize;
                let mut remain_bytes = entry.file_size as usize;

                self.draw_cursor(false);
                while cluster != 0 && cluster != crate::fat::ENF_OF_CLUTERCHAIN {
                    let mut bytes_per_cluster = crate::fat::bytes_per_cluster();
                    let p = crate::fat::get_sector_by_cluster::<u8>(cluster);

                    let read_bytes = remain_bytes.min(bytes_per_cluster);
                    let slice = unsafe { core::slice::from_raw_parts(p, read_bytes) };

                    for &byte in slice {
                        self.print_char(byte);
                    }

                    remain_bytes -= read_bytes;
                    cluster = crate::fat::next_cluster(cluster);
                }
                self.draw_cursor(true);
            }
        } else if !command.is_empty() {
            self.print("no such command: ");
            self.print(command);
            self.print("\n");
        }
    }

    /// direction = -1 -> 進む +1 -> 戻る
    fn history_updown(&mut self, direction: i32) -> Rectangle<i32> {
        if direction == -1 && self.cmd_history_index >= 0 {
            self.cmd_history_index -= 1;
        } else if direction == 1 && (self.cmd_history_index + 1) < self.cmd_history.len() as i32 {
            self.cmd_history_index += 1;
        }
        self.cursor.x = 1;
        let first_pos = self.calc_cursor_pos();

        let draw_area = Rectangle {
            pos_lu: Vector2D {
                x: first_pos.x as i32,
                y: first_pos.y as i32,
            },
            size: Vector2D {
                x: (8 * (Self::COLUMNS - 1)) as i32,
                y: 16,
            },
        };

        fill_rectrange(
            &self.window.borrow_mut().writer(),
            &first_pos,
            &Vector2D {
                x: 8 * (Self::COLUMNS - 1),
                y: 16,
            },
            &to_color(0),
        );

        let history: &[u8] = if self.cmd_history_index >= 0 {
            let entry = &self.cmd_history[self.cmd_history_index as usize];
            let len = entry.iter().position(|&c| c == 0).unwrap_or(entry.len());
            &entry[..len]
        } else {
            b""
        };

        self.line_buf[..history.len()].copy_from_slice(history);
        self.line_buf[history.len()..].fill(0);
        self.linebuf_index = history.len();

        let s = core::str::from_utf8(history).unwrap_or("");
        write_string(
            &self.window.borrow_mut().writer(),
            first_pos.x,
            first_pos.y,
            s,
            &to_color(0xffffff),
        );

        self.cursor.x = self.linebuf_index + 1;

        draw_area
    }
    pub fn input_key(&mut self, _modifier: u8, _keycode: u8, ascii: char) -> Rectangle<i32> {
        self.draw_cursor(false);

        let mut draw_area = Rectangle {
            pos_lu: Vector2D {
                x: self.calc_cursor_pos().x as i32,
                y: self.calc_cursor_pos().y as i32,
            },
            size: Vector2D { x: 8 * 2, y: 16 },
        };

        if ascii == '\n' {
            self.line_buf[self.linebuf_index..].fill(0); // 最後を0埋め
            if self.linebuf_index > 0 {
                self.cmd_history.pop_back();
                self.cmd_history.push_front(self.line_buf);
            }
            self.cmd_history_index = -1;

            self.newline();
            self.execute_line();
            self.print(">");
            draw_area.pos_lu = Vector2D {
                x: TOP_LEFT_MARGIN.x as i32,
                y: TOP_LEFT_MARGIN.y as i32,
            };
            let inner_size = self.window.borrow().inner_size();
            draw_area.size = Vector2D {
                x: inner_size.x as i32,
                y: inner_size.y as i32,
            };
        } else if ascii == '\x08' {
            // Backspace
            if self.cursor.x > 0 {
                self.cursor.x -= 1;
                fill_rectrange(
                    &self.window.borrow_mut().writer(),
                    &self.calc_cursor_pos(),
                    &Vector2D { x: 8, y: 16 },
                    &to_color(0),
                );
                draw_area.pos_lu = Vector2D {
                    x: self.calc_cursor_pos().x as i32,
                    y: self.calc_cursor_pos().y as i32,
                };
                if self.linebuf_index > 0 {
                    self.linebuf_index -= 1;
                }
            }
        } else if _keycode == 0x51 {
            draw_area = self.history_updown(-1);
        } else if _keycode == 0x52 {
            draw_area = self.history_updown(1);
        } else if ascii != '\0' {
            if self.cursor.x < Self::COLUMNS - 1 && self.linebuf_index < Self::LINE_MAX - 1 {
                self.line_buf[self.linebuf_index] = ascii as u8;
                self.linebuf_index += 1;
                let pos = self.calc_cursor_pos();
                write_ascii(
                    &self.window.borrow_mut().writer(),
                    pos.x,
                    pos.y,
                    ascii as u8,
                    &to_color(0xffffff),
                );
                self.cursor.x += 1;
            }
        }
        self.draw_cursor(true);
        draw_area
    }
}

pub extern "C" fn task_terminal(task_id: u64, _data: i64, _arg3: u64, _arg4: u64) {
    let mut terminal = Terminal::new();
    let layer_id = terminal.layer_id();

    LAYER_MANAGER.with(|lm| {
        let Some(lm) = lm.as_mut() else { return };
        lm.move_layer(layer_id, Vector2D { x: 100, y: 200 });
        lm.activate(layer_id);
    });

    crate::layer::register_layer_task(layer_id, task_id);

    loop {
        x86_64::instructions::interrupts::disable();
        let Some(msg) = receive_message(task_id) else {
            let _ = task::sleep(task_id);
            x86_64::instructions::interrupts::enable();
            continue;
        };

        x86_64::instructions::interrupts::enable();

        match msg {
            Message::TimerTimeout { .. } => {
                let area = x86_64::instructions::interrupts::without_interrupts(|| {
                    terminal.blink_cursor()
                });

                let _ = task::send_message(
                    1,
                    Message::Layer {
                        op: LayerOperation::DrawArea { area },
                        layer_id,
                        src_task: task_id,
                    },
                );
            }
            Message::KeyPush {
                keycode,
                modifier,
                ascii,
            } => {
                let area = x86_64::instructions::interrupts::without_interrupts(|| {
                    terminal.input_key(modifier, keycode, ascii)
                });
                let _ = task::send_message(
                    1,
                    Message::Layer {
                        op: LayerOperation::DrawArea { area },
                        layer_id,
                        src_task: task_id,
                    },
                );
            }
            _ => {}
        }
    }
}
