use std::{collections::HashMap, rc::Rc, cell::RefCell};

use super::{view_data::ViewData, render::Renderer};

pub struct Viewer {
    render: Rc<RefCell<Option<Renderer>>>,
    data: HashMap<u32, ViewData>,
    next_data_id: u32
}

impl Viewer {
    pub fn new(render: Rc<RefCell<Option<Renderer>>>) -> Self {
        Self {
            render,
            data: HashMap::new(),
            next_data_id: 0
        }
    }

    pub fn append_mesh(&mut self, points: &[f64], triangles: &[u32]) {
        let points = Vec::from_iter(points.iter().map(|&x| x as f32));
        let triangles = Vec::from_iter(triangles.iter().map(|&i| i as u32));
        let data = ViewData::new(self.render.clone(), points, triangles);
        self.data.insert(self.next_data_id, data);
        self.next_data_id += 1;
    }
}