use anyhow::Result;
use cgmath::{InnerSpace, Quaternion, Rad, Rotation3, Vector3};
use rand::{
    distributions::{Distribution, Uniform},
    SeedableRng,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use winit::dpi::PhysicalPosition;

use crate::render::view_data::{Material, Vertex};

use super::{render::Renderer, view_core::ViewCore, view_data::ViewData};
pub enum MousePressed {
    Left(Option<(PhysicalPosition<f64>, Quaternion<f32>)>),
    Right(Option<PhysicalPosition<f64>>),
    None,
}

pub struct Viewer {
    pub render: Rc<RefCell<Option<Renderer>>>,
    data: HashMap<u32, ViewData>,
    next_data_id: u32,
    view_core: ViewCore,

    current_pos: PhysicalPosition<f64>,
    pub pressed_state: MousePressed,
    data_dirty: bool,
}

impl Viewer {
    pub fn new(render: Rc<RefCell<Option<Renderer>>>) -> Self {
        Self {
            render,
            data: HashMap::new(),
            next_data_id: 0,
            view_core: ViewCore::default(),
            current_pos: PhysicalPosition { x: 0.0, y: 0.0 },
            pressed_state: MousePressed::None,
            data_dirty: false,
        }
    }

    pub fn append_mesh(
        &mut self,
        points: &[f64],
        triangles: &[usize],
        color: Option<Vector3<f32>>,
    ) -> u32 {
        let point = |idx| {
            let start = idx * 3;
            &points[start..(start + 3)]
        };
        let vertices = Vec::from_iter(
            triangles
                .chunks(3)
                .map(|f| {
                    let verts = [point(f[0]), point(f[1]), point(f[2])]
                        .map(|p| Vector3::<f32>::new(p[0] as f32, p[1] as f32, p[2] as f32));
                    let vab = verts[1] - verts[0];
                    let vac = verts[2] - verts[0];
                    let normal = vab.cross(vac).normalize();
                    [
                        Vertex {
                            point: verts[0].into(),
                            normal: normal.into(),
                            barycentric: [1.0, 0.0, 0.0],
                        },
                        Vertex {
                            point: verts[1].into(),
                            normal: normal.into(),
                            barycentric: [0.0, 1.0, 0.0],
                        },
                        Vertex {
                            point: verts[2].into(),
                            normal: normal.into(),
                            barycentric: [0.0, 0.0, 1.0],
                        },
                    ]
                })
                .flatten(),
        );

        let data_color = if let Some(color) = color {
            color
        } else {
            let mut rng = rand::rngs::SmallRng::from_entropy();
            let between = Uniform::from(0.0..1.0);
            Vector3::new(
                between.sample(&mut rng),
                between.sample(&mut rng),
                between.sample(&mut rng),
            )
        };
        let data = ViewData::new(vertices, Material::new(data_color));
        let id = self.next_data_id;
        self.next_data_id += 1;
        self.data.insert(id, data);
        id
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
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &render.depth_texture_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                render_pass.set_viewport(0.0, 0.0, render.w() as f32, render.h() as f32, 0.0, 1.0);
                self.view_core.render(
                    render,
                    &mut render_pass,
                    &mut self.data,
                    self.data_dirty,
                    true,
                );
                self.data_dirty = false;
            }
            render.queue.submit(std::iter::once(encoder.finish()));
            texture.present();
        } else {
            leptos::logging::log!("render is None");
        }
        Result::Ok(())
    }

    pub fn mouse_move(&mut self, pos: PhysicalPosition<f64>) {
        self.current_pos = pos;
        match &mut self.pressed_state {
            MousePressed::Left(left_pos) => {
                if left_pos.is_none() {
                    left_pos.replace((pos, self.view_core.trackball_angle));
                }
            }
            MousePressed::Right(right_pos) => {
                if right_pos.is_none() {
                    right_pos.replace(pos);
                }
            }
            MousePressed::None => {}
        }

        self.current_pos = pos;

        match &self.pressed_state {
            MousePressed::Left(left) => {
                let (press_pos, press_quat) = left.as_ref().unwrap();
                let render = self.render.borrow();
                let render = render.as_ref().unwrap();
                let quat = two_axis_valuator_fixed_up(
                    render.w(),
                    render.h(),
                    4.0,
                    press_quat,
                    press_pos,
                    &pos,
                );
                self.view_core.trackball_angle = quat;
            }
            _ => {}
        }
    }

    pub fn mouse_scroll(&mut self, delta_y: f64) {
        if delta_y != 0.0 {
            const MIN_ZOOM: f32 = 0.1;
            let mult = 1.0 + if delta_y > 0.0 { 1.0 } else { -1.0 } * 0.05;
            self.view_core.camera_zoom = MIN_ZOOM.max(self.view_core.camera_zoom * mult);
        }
    }

    pub fn remove_data(&mut self, id: u32) {
        self.data.remove(&id);
        self.data_dirty = true;
    }

    pub fn set_visible(&mut self, id: u32, visible: bool) {
        if let Some(data) = self.data.get_mut(&id) {
            data.set_visible(visible);
        }
    }

    pub fn set_edge_width(&mut self, id: u32, width: f32) {
        if let Some(data) = self.data.get_mut(&id) {
            data.material.edge_width = width;
            data.dirty.insert(crate::render::view_data::DirtyFlags::DIRTY_MATERIAL);
        }
    }

    pub fn set_edge_color(&mut self, id: u32, color: [f32; 4]) {
        if let Some(data) = self.data.get_mut(&id) {
            data.material.edge_color = color;
            data.dirty.insert(crate::render::view_data::DirtyFlags::DIRTY_MATERIAL);
        }
    }
}

fn two_axis_valuator_fixed_up(
    w: u32,
    h: u32,
    speed: f32,
    press_quat: &Quaternion<f32>,
    press_pos: &PhysicalPosition<f64>,
    pos: &PhysicalPosition<f64>,
) -> Quaternion<f32> {
    const AXIS_X: Vector3<f32> = Vector3::new(1.0, 0.0, 0.0);
    const AXIS_Y: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);
    let mut quat = Quaternion::<f32>::from_axis_angle(
        AXIS_Y,
        Rad::<f32>(((pos.x - press_pos.x) as f32) / (w as f32) * speed),
    ) * Quaternion::<f32>::from_axis_angle(
        AXIS_X,
        Rad::<f32>(((pos.y - press_pos.y) as f32) / (h as f32) * speed),
    ) * press_quat;

    let len = quat.magnitude();
    quat /= len;
    quat
}
