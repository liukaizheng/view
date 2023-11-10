use std::{collections::HashMap, rc::Rc, cell::RefCell};
use anyhow::Result;

use super::{view_data::ViewData, render::Renderer, view_core::ViewCore};

pub struct Viewer {
    render: Rc<RefCell<Option<Renderer>>>,
    data: HashMap<u32, ViewData>,
    next_data_id: u32,
    view_core: ViewCore,
}

impl Viewer {
    pub fn new(render: Rc<RefCell<Option<Renderer>>>) -> Self {
        Self {
            render,
            data: HashMap::new(),
            next_data_id: 0,
            view_core: ViewCore::default(),
        }
    }

    pub fn append_mesh(&mut self, points: &[f64], triangles: &[usize]) {
        let points = Vec::from_iter(points.iter().map(|&x| x as f32));
        let triangles = Vec::from_iter(triangles.iter().map(|&i| i as u32));
        let data = ViewData::new(points, triangles);
        self.data.insert(self.next_data_id, data);
        self.next_data_id += 1;
        leptos::logging::log!("appended mesh");
    }

    pub fn render(&mut self) -> Result<()> {
        if let Some(render) = self.render.borrow().as_ref() {
            render.render()?;
            self.view_core.render(render, &mut self.data, true);
        } else {
            leptos::logging::log!("render is None");
        }
        Result::Ok(())
    }
}