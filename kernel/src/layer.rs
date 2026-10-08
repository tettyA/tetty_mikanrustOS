use core::{cell::RefCell, ptr::null_mut};

use crate::global::Global;

use alloc::{boxed::Box, rc::Rc, vec::Vec};

use crate::{
    frame_buffer::FrameBuffer,
    graphics::{Rectrange, Vector2D},
    window::Window,
};

pub struct Layer {
    pub id: u32,
    z: i32,
    pub pos: Vector2D<i32>,
    window: Option<Rc<RefCell<Window>>>,
}

impl Layer {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            z: -1,
            pos: Vector2D { x: 0, y: 0 },
            window: None,
        }
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
            window.borrow().draw_to(
                frame_buffer,
                Vector2D {
                    x: self.pos.x.max(0),
                    y: self.pos.y.max(0),
                },
            );
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
        if let Some(layer) = self.find_layer(id) {
            layer.move_to(new_pos);
        }
    }

    pub fn move_layer_rerative(&mut self, id: u32, pos_relative: Vector2D<i32>) {
        if let Some(layer) = self.find_layer(id) {
            layer.move_relative(pos_relative);
        }
    }

    /// レイヤーの重なり順を設定する (`new_z` が小さいほど背面、負なら非表示)。
    pub fn up_down(&mut self, id: u32, new_z: i32) {
        if let Some(layer) = self.find_layer(id) {
            layer.z = new_z;
        } else {
            return;
        }
        let mut visible: Vec<(i32, u32)> = self
            .layers
            .iter()
            .filter(|l| l.z >= 0)
            .map(|l| (l.z, l.id))
            .collect();
        visible.sort_by_key(|&(z, _)| z);
        self.layer_stack = visible.into_iter().map(|(_, id)| id).collect();
    }
    pub fn hide_layer(&mut self, id: u32) {
        self.up_down(id, -1);
    }
    pub fn show_layer(&mut self, id: u32, new_z: i32) {
        self.up_down(id, new_z);
    }

    /// 全レイヤーを直接画面へ描画する。
    pub fn draw_all(&mut self) {
        for id in &self.layer_stack {
            if let Some(layer) = self.layers.iter().find(|l| l.id == *id) {
                if let Some(window) = &layer.window {
                    window.borrow().draw_to(
                        &mut self.screen,
                        Vector2D {
                            x: layer.pos.x.max(0),
                            y: layer.pos.y.max(0),
                        },
                    );
                }
            }
        }
    }

    /// 画面上の `area` だけを再描画する (バックバッファ経由)。
    pub fn draw_area(&mut self, area: Rectrange<i32>) {
        for id in &self.layer_stack {
            if let Some(layer) = self.layers.iter().find(|l| l.id == *id) {
                if let Some(window) = &layer.window {
                    window.borrow().draw_to_area(
                        &mut self.back_buffer,
                        Vector2D {
                            x: layer.pos.x.max(0),
                            y: layer.pos.y.max(0),
                        },
                        area,
                    );
                }
            }
        }
        self.screen.copy_area(&area, &self.back_buffer);
    }

    /// レイヤー `id` とその前面にあるレイヤーを、`id` の領域だけ再描画する。
    pub fn draw_id(&mut self, id: u32) {
        let mut draw = false;
        let mut window_area = Rectrange {
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
                    window.borrow().draw_to_area(
                        &mut self.back_buffer,
                        Vector2D {
                            x: layer.pos.x.max(0),
                            y: layer.pos.y.max(0),
                        },
                        window_area,
                    );
                }
            }
        }
        if draw {
            self.screen.copy_area(&window_area, &self.back_buffer);
        }
    }

    fn find_layer(&mut self, id: u32) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id).map(|l| l.as_mut())
    }
}
