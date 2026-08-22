use core::cell::RefCell;

use alloc::{boxed::Box, rc::Rc, vec::Vec};

use crate::{
    graphics::{PixelWriter, Vector2D},
    layer,
    window::Window,
};

pub struct Layer {
    id: u32,
    z: i32,
    pos: Vector2D<i32>,
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
    pub fn draw_to(&self, writer: &PixelWriter) {
        if let Some(window) = &self.window {
            window.borrow().draw_to(
                writer,
                Vector2D {
                    x: self.pos.x.max(0) as usize,
                    y: self.pos.y.max(0) as usize,
                },
            );
        }
    }
}

pub struct LayerManager {
    latest_id: u32,
    layers: Vec<Box<Layer>>,
    // 末尾が最全面(最も最後に描画される。) layers[layer_stack[i]] (i in 0..N) で描画
    layer_stack: Vec<*mut Layer>,
}

impl LayerManager {
    pub fn new() -> Self {
        Self {
            latest_id: 1,
            layers: Vec::new(),
            layer_stack: Vec::new(),
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

    pub fn up_down(&mut self, id: u32, new_z: i32) {
        if let Some(layer) = self.find_layer(id) {
            layer.z = new_z;
        } else {
            return;
        }
        self.layer_stack.clear();
        for layer in &mut self.layers {
            if layer.z >= 0 {
                self.layer_stack.push(layer.as_mut() as *mut Layer);
            }
        }
        self.layer_stack.sort_by_key(|&ptr| unsafe { (*ptr).z });
    }
    pub fn hide_layer(&mut self, id: u32) {
        self.up_down(id, -1);
    }
    pub fn show_layer(&mut self, id: u32, new_z: i32) {
        self.up_down(id, new_z);
    }

    pub fn draw(&self, writer: &PixelWriter) {
        for &l in &self.layer_stack {
            unsafe {
                let layer = &*l;
                if let Some(window) = &layer.window {
                    window.borrow().draw_to(
                        writer,
                        Vector2D {
                            x: layer.pos.x.max(0) as usize,
                            y: layer.pos.y.max(0) as usize,
                        },
                    );
                }
            }
        }
    }
    fn find_layer(&mut self, id: u32) -> Option<&mut Layer> {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            Some(layer)
        } else {
            None
        }
    }
}
