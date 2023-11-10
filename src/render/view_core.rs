use cgmath::{Matrix4, Point3, Vector3};

use super::{
    render::Renderer,
    view_data::{DirtyFlags, ViewData},
    BBox,
};

use wgpu::{util::DeviceExt, BindGroupLayout, Buffer};

struct ViewBuffer {
    camera_bind_group_layout: BindGroupLayout,
    camera_buffer: Buffer,
}

pub(crate) struct ViewCore {
    camera_base_zoom: f32,
    camera_base_translation: Vector3<f32>,

    camera_near: f32,
    camera_far: f32,
    camera_fov: cgmath::Rad<f32>,

    camera_eye: Point3<f32>,
    camera_center: Point3<f32>,
    camera_up: Vector3<f32>,
    view_buffer: Option<ViewBuffer>,
}

impl Default for ViewCore {
    fn default() -> Self {
        Self {
            camera_base_zoom: 1.0,
            camera_base_translation: Vector3::new(0.0, 0.0, 0.0),

            camera_near: 1.0,
            camera_far: 100.0,
            camera_fov: cgmath::Deg(45.0).into(),

            camera_eye: Point3::new(0.0, 0.0, 5.0),
            camera_center: Point3::new(0.0, 0.0, 0.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),

            view_buffer: None,
        }
    }
}

impl ViewCore {
    pub(crate) fn render<'a>(
        &mut self,
        render: &Renderer,
        data_map: &mut std::collections::HashMap<u32, ViewData>,
        update_matrix: bool,
    ) {
        if self.view_buffer.is_none() {
            let camera_bind_group_layout =
                render
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("camera_bind_group_layout"),
                        entries: &[wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }],
                    });
            let camera_buffer =
                render
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("camera_buffer"),
                        contents: bytemuck::cast_slice(&[0.0f32; 16]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });
            self.view_buffer = Some(ViewBuffer {
                camera_bind_group_layout,
                camera_buffer,
            });
        }
        let mut has_dirty_data = false;
        for data in data_map.values_mut() {
            if data.pipeline.is_none() {
                data.init_pipline(
                    render,
                    &self.view_buffer.as_ref().unwrap().camera_bind_group_layout,
                );
            }
            if data.dirty.contains(DirtyFlags::DIRTY_VERTEX) {
                has_dirty_data = true;
                data.update_box();
            }
        }

        if has_dirty_data {
            let mut bbox = BBox::default();
            for data in data_map.values() {
                bbox.merge_box(&data.bbox);
            }
            let center = (bbox.min + bbox.max) / 2.0;
            self.camera_base_translation = -center;
        }

        if has_dirty_data || update_matrix {
            self.update_matrix(render);
        }
    }

    fn update_matrix(&self, render: &Renderer) {
        let view = Matrix4::from_scale(self.camera_base_zoom)
            * Matrix4::from_translation(self.camera_base_translation);
        let look_at = Matrix4::look_at_rh(self.camera_eye, self.camera_center, self.camera_up);
        let w = render.width as f32;
        let h = render.height as f32;
        let proj = cgmath::perspective(self.camera_fov, w / h, self.camera_near, self.camera_far);
        let mat = proj * look_at * view;
        let data: [[f32; 4]; 4] = mat.into();
        render.queue.write_buffer(
            &self.view_buffer.as_ref().unwrap().camera_buffer, 0, bytemuck::cast_slice(&data));

    }
}
