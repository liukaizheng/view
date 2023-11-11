use anyhow::Result;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::{render::Renderer, view_core::ViewCore, view_data::ViewData};

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
            let texture = render.surface.get_current_texture()?;
            let view = texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = render
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                render_pass.set_viewport(
                    0.0,
                    0.0,
                    render.width as f32,
                    render.height as f32,
                    0.0,
                    1.0,
                );
                self.view_core
                    .render(render, &mut render_pass, &mut self.data, true);
            }
            render.queue.submit(std::iter::once(encoder.finish()));
            texture.present();
        } else {
            leptos::logging::log!("render is None");
        }
        Result::Ok(())
    }
}
