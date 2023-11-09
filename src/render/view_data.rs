use std::{rc::Rc, cell::RefCell};

use super::render::Renderer;

pub struct ViewData {
    render: Rc<RefCell<Option<Renderer>>>,
    vertices: Vec<f32>,
    triangles: Vec<u32>,
}

impl ViewData {
    pub fn new(render: Rc<RefCell<Option<Renderer>>>, vertices: Vec<f32>, triangles: Vec<u32>) -> Self {
        let mut data = Self {
            render,
            vertices,
            triangles,
        };
        data.init();
        data
    }

    fn init(&mut self) {
        if let Some(render) = self.render.borrow().as_ref() {
            
        }
    }
}
