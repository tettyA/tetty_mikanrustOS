use core::{cell::RefCell, ptr::null_mut};

use crate::{global::Global, layer, message::LayerOperation};

use alloc::{boxed::Box, rc::Rc, vec::Vec};

use crate::{
    frame_buffer::FrameBuffer,
    graphics::{Rectangle, Vector2D},
    window::Window,
};

pub struct Layer {
    pub id: u32,
    z: i32,
    pub pos: Vector2D<i32>,
    window: Option<Rc<RefCell<Window>>>,
    draggable: bool,
}

impl Layer {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            z: -1,
            pos: Vector2D { x: 0, y: 0 },
            window: None,
            draggable: false,
        }
    }

    pub fn set_draggable(&mut self, draggable: bool) -> &mut Self {
        self.draggable = draggable;
        self
    }
    pub fn is_draggable(&self) -> bool {
        self.draggable
    }
    pub fn set_window(&mut self, window: Rc<RefCell<Window>>) -> &mut Self {
        self.window = Some(window);
        self
    }

    pub fn move_to(&mut self, pos_to: Vector2D<i32>) -> &mut Self {
        self.pos = pos_to;
        self
    }
    pub fn move_relative(&mut self, pos_diff: Vector2D<i32>) -> &mut Self {
        self.pos += pos_diff;
        self
    }
    pub fn draw_to(&self, frame_buffer: &mut FrameBuffer) {
        if let Some(window) = &self.window {
            window.borrow().draw_to(frame_buffer, self.pos);
        }
    }
}

/// カーネル全体で共有するレイヤーマネージャ。`init` 前は `None`。
/// 使い方: `LAYER_MANAGER.with(|lm| if let Some(lm) = lm { lm.draw_all() })`
/// (通常は main.rs の `redraw_layers_*` を経由する)
pub static LAYER_MANAGER: Global<Option<LayerManager>> = Global::new(None);

/// 画面 (`screen`) とレイヤー群を所有し、重ね合わせ描画を行う。
pub struct LayerManager {
    latest_id: u32,
    layers: Vec<Box<Layer>>,
    /// 描画順に並べたレイヤー ID (末尾が最前面 = 最後に描画される)。
    layer_stack: Vec<u32>,
    screen: FrameBuffer,
    back_buffer: FrameBuffer,
    active_layer: u32,
    mouse_layer: u32,
}

impl LayerManager {
    /// `screen` の所有権を受け取る (同じ設定のバックバッファを内部で確保する)。
    pub fn new(mut screen: FrameBuffer) -> Self {
        let mut backbuffer_config = *screen.get_config_mut();
        backbuffer_config.frame_buffer = null_mut();
        Self {
            latest_id: 1,
            layers: Vec::new(),
            layer_stack: Vec::new(),
            screen,
            back_buffer: FrameBuffer::new(backbuffer_config),
            active_layer: 0,
            mouse_layer: 0,
        }
    }

    /// 生成されたレイヤーの可変参照を返す。
    pub fn new_layer(&mut self) -> &mut Layer {
        let id = self.latest_id;
        self.latest_id += 1;
        self.layers.push(Box::new(Layer::new(id)));
        self.layers.last_mut().unwrap()
    }

    pub fn move_layer(&mut self, id: u32, new_pos: Vector2D<i32>) {
        let mut old_area = None;
        let mut new_area = None;
        if let Some(layer) = self.find_layer(id) {
            if let Some(window) = &layer.window {
                let window = window.borrow();
                old_area = Some(Rectangle {
                    pos_lu: layer.pos,
                    size: Vector2D {
                        x: window.width() as i32,
                        y: window.height() as i32,
                    },
                });
                new_area = Some(Rectangle {
                    pos_lu: new_pos,
                    size: Vector2D {
                        x: window.width() as i32,
                        y: window.height() as i32,
                    },
                });
            }
            layer.move_to(new_pos);
        }
        // 再描画する領域を計算して描画する
        if let Some(area) = old_area {
            self.draw_area(area);
        }
        if let Some(area) = new_area {
            self.draw_area(area);
        }
    }

    pub fn get_height(&self, id: u32) -> Option<usize> {
        self.layer_stack.iter().position(|&x| x == id)
    }

    /// レイヤーの重なり順を設定する(new_height が小さいほど背面。負で非表示)
    pub fn up_down(&mut self, id: u32, new_height: i32) {
        if new_height < 0 {
            self.hide_layer(id);
            return;
        }
        let new_pos = (new_height as usize).min(self.layer_stack.len());

        if let Some(old_pos) = self.layer_stack.iter().position(|&x| x == id) {
            let layer_id = self.layer_stack.remove(old_pos);
            let insert_pos = if new_pos > old_pos {
                (new_pos - 1).min(self.layer_stack.len())
            } else {
                new_pos
            };
            self.layer_stack.insert(insert_pos, layer_id);
        } else {
            self.layer_stack.insert(new_pos, id);
        }
    }

    pub fn move_layer_rerative(&mut self, id: u32, pos_relative: Vector2D<i32>) {
        let mut old_area = None;
        let mut new_area = None;
        if let Some(layer) = self.find_layer(id) {
            if let Some(window) = &layer.window {
                let window = window.borrow();
                old_area = Some(Rectangle {
                    pos_lu: layer.pos,
                    size: Vector2D {
                        x: window.width() as i32,
                        y: window.height() as i32,
                    },
                });
                new_area = Some(Rectangle {
                    pos_lu: layer.pos + pos_relative,
                    size: Vector2D {
                        x: window.width() as i32,
                        y: window.height() as i32,
                    },
                });
            }
            layer.move_relative(pos_relative);
        }
        if let Some(area) = old_area {
            self.draw_area(area);
        }
        if let Some(area) = new_area {
            self.draw_area(area);
        }
    }

    pub fn hide_layer(&mut self, id: u32) {
        self.layer_stack.retain(|&x| x != id);
    }
    pub fn show_layer(&mut self, id: u32, new_z: i32) {
        self.up_down(id, new_z);
    }
    pub fn activate(&mut self, layer_id: u32) {
        if self.active_layer == layer_id {
            return;
        }

        if self.active_layer > 0 {
            if let Some(layer) = self.find_layer(self.active_layer) {
                if let Some(window) = &layer.window {
                    window.borrow_mut().deactivate();
                }
            }
            self.draw_id(self.active_layer);
        }

        self.active_layer = layer_id;

        if self.active_layer > 0 {
            if let Some(layer) = self.find_layer(self.active_layer) {
                if let Some(window) = &layer.window {
                    window.borrow_mut().activate();
                }
            }

            if let Some(mouse_h) = self.get_height(self.mouse_layer) {
                let new_h = if mouse_h > 0 { mouse_h - 1 } else { 0 };
                self.up_down(self.active_layer, new_h as i32);
            }
            self.draw_id(self.active_layer);
        }
    }

    /// 全レイヤーを直接画面へ描画する。
    pub fn draw_all(&mut self) {
        for id in &self.layer_stack {
            if let Some(layer) = self.layers.iter().find(|l| l.id == *id) {
                if let Some(window) = &layer.window {
                    window.borrow().draw_to(&mut self.screen, layer.pos);
                }
            }
        }
    }

    /// 画面上の `area` だけを再描画する (バックバッファ経由)。
    pub fn draw_area(&mut self, area: Rectangle<i32>) {
        for id in &self.layer_stack {
            if let Some(layer) = self.layers.iter().find(|l| l.id == *id) {
                if let Some(window) = &layer.window {
                    window
                        .borrow()
                        .draw_to_area(&mut self.back_buffer, layer.pos, area);
                }
            }
        }
        self.screen.copy_area(&area, &self.back_buffer);
    }

    /// レイヤー `id` とその前面にあるレイヤーを、`id` の領域だけ再描画する。
    pub fn draw_id(&mut self, id: u32) {
        let mut draw = false;
        let mut window_area = Rectangle {
            pos_lu: Vector2D { x: 0, y: 0 },
            size: Vector2D { x: 0, y: 0 },
        };
        for stack_id in &self.layer_stack {
            let Some(layer) = self.layers.iter().find(|l| l.id == *stack_id) else {
                continue;
            };
            if layer.id == id {
                if let Some(window) = &layer.window {
                    let window = window.borrow();
                    window_area.pos_lu = layer.pos;
                    window_area.size = Vector2D {
                        x: window.width() as i32,
                        y: window.height() as i32,
                    };
                    draw = true;
                }
            }
            if draw {
                if let Some(window) = &layer.window {
                    window
                        .borrow()
                        .draw_to_area(&mut self.back_buffer, layer.pos, window_area);
                }
            }
        }
        if draw {
            self.screen.copy_area(&window_area, &self.back_buffer);
        }
    }

    pub fn find_layer_by_position(
        &mut self,
        pos: Vector2D<i32>,
        exclude_id: u32,
    ) -> Option<&mut Layer> {
        for i in (0..self.layer_stack.len()).rev() {
            let id = self.layer_stack[i];
            if let Some(layer) = self.find_layer(id) {
                let hit = {
                    if let Some(window) = &layer.window {
                        let window = window.borrow();
                        let window_area = Rectangle {
                            pos_lu: layer.pos,
                            size: Vector2D {
                                x: window.width() as i32,
                                y: window.height() as i32,
                            },
                        };
                        window_area.pos_lu.x <= pos.x
                            && pos.x < window_area.pos_lu.x + window_area.size.x
                            && window_area.pos_lu.y <= pos.y
                            && pos.y < window_area.pos_lu.y + window_area.size.y
                            && (id != exclude_id)
                    } else {
                        false
                    }
                };
                if hit {
                    return Some(layer);
                }
            }
        }
        None
    }

    fn find_layer(&mut self, id: u32) -> Option<&mut Layer> {
        self.layers
            .iter_mut()
            .find(|l| l.id == id)
            .map(|l| l.as_mut())
    }
}

pub fn process_layer_message(op: LayerOperation, layer_id: u32) {
    LAYER_MANAGER.with(|lm| {
        let Some(lm) = lm.as_mut() else { return };
        match op {
            LayerOperation::Move { x, y } => {
                lm.move_layer(layer_id, Vector2D { x, y });
            }
            LayerOperation::MoveRelative { x, y } => {
                lm.move_layer_rerative(layer_id, Vector2D { x, y });
            }
            LayerOperation::Draw => {
                lm.draw_id(layer_id);
            }
        }
    });
}

pub fn activate(layer_id: u32) {
    LAYER_MANAGER.with(|lm| {
        if let Some(lm) = lm {
            lm.activate(layer_id);
        }
    });
}
pub fn set_mouse_layer(mouse_layer_id: u32) {
    LAYER_MANAGER.with(|lm| {
        if let Some(lm) = lm {
            lm.mouse_layer = mouse_layer_id;
        }
    });
}
pub fn get_active() -> u32 {
    LAYER_MANAGER.with(|lm| lm.as_ref().map_or(0, |lm| lm.active_layer))
}
