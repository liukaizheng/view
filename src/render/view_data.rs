use super::{render::Renderer, BBox};

use cgmath::Vector3;
use wgpu::{util::DeviceExt, Buffer, CompareFunction, RenderPass, RenderPipeline, TextureFormat};

#[inline]
fn box_from_points(points: &[f32]) -> BBox {
    let data = points.chunks(3).fold(
        (f32::MAX, f32::MAX, f32::MAX, f32::MIN, f32::MIN, f32::MIN),
        |(min_x, min_y, min_z, max_x, max_y, max_z), point| {
            let x = point[0];
            let y = point[1];
            let z = point[2];
            (
                min_x.min(x),
                min_y.min(y),
                min_z.min(z),
                max_x.max(x),
                max_y.max(y),
                max_z.max(z),
            )
        },
    );
    let min = Vector3::new(data.0, data.1, data.2);
    let max = Vector3::new(data.3, data.4, data.5);
    BBox { min, max }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct DirtyFlags: u32 {
        const DIRTY_NONE = 0b00000000;
        const DIRTY_VERTEX = 0b00000001;
        const DIRTY_EDGE = 0b00000010;
        const DIRTY_FACE = 0b00000100;
        const DIRTY_ALL = Self::DIRTY_VERTEX.bits() | Self::DIRTY_EDGE.bits() | Self::DIRTY_FACE.bits();
    }
}

pub(crate) struct Material {
    /// ambient color
    pub ka: Vector3<f32>,
    /// diffuse color
    pub kd: Vector3<f32>,
    /// specular color
    pub ks: Vector3<f32>,
}

impl Material {
    pub(crate) fn new(color: Vector3<f32>) -> Self {
        let kd = color;
        let ka = kd * 0.1;
        const GREY: Vector3<f32> = Vector3::new(0.3, 0.3, 0.3);
        let ks = GREY + 0.1 * (kd - GREY);
        Self {
            ka,
            kd,
            ks,
        }
    }
}

pub(crate) struct MeshPipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}
pub(crate) struct ViewData {
    vertices: Vec<f32>,
    triangles: Vec<u32>,
    material: Material,
    pub(crate) dirty: DirtyFlags,
    pub(crate) bbox: BBox,
    pub(crate) pipeline: Option<MeshPipeline>,
}

impl ViewData {
    pub(crate) fn new(vertices: Vec<f32>, triangles: Vec<u32>, material: Material) -> Self {
        Self {
            vertices,
            triangles,
            material,
            dirty: DirtyFlags::DIRTY_ALL,
            bbox: BBox::default(),
            pipeline: None,
        }
    }

    pub(crate) fn init_pipline(
        &mut self,
        render: &Renderer,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        let shader = render
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let render_pipeline_layout =
            render
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("render_pipeline_layout"),
                    bind_group_layouts: &[camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            render
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("render_pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: 3 * std::mem::size_of::<f32>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            }],
                        }],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(render.config.format.into())],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });

        let vertex_buffer = render
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX |
                // allow it to be the destination for [`Queue::write_buffer`] operation
                wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer = render
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: bytemuck::cast_slice(&self.triangles),
                usage: wgpu::BufferUsages::INDEX |
                // allow it to be the destination for [`Queue::write_buffer`] operation
                wgpu::BufferUsages::COPY_DST,
            });

        self.pipeline = Some(MeshPipeline {
            pipeline: render_pipeline,
            vertex_buffer,
            index_buffer,
        });
    }

    #[inline]
    fn update_box(&mut self) {
        self.bbox = box_from_points(&self.vertices);
    }

    #[inline]
    pub(crate) fn update_vertex_buffer(&mut self, render: &Renderer) {
        render.queue.write_buffer(
            &self.pipeline.as_ref().unwrap().vertex_buffer,
            0,
            bytemuck::cast_slice(&self.vertices),
        );
        self.update_box();
    }

    #[inline]
    pub(crate) fn update_face_buffer(&mut self, render: &Renderer) {
        render.queue.write_buffer(
            &self.pipeline.as_ref().unwrap().index_buffer,
            0,
            bytemuck::cast_slice(&self.triangles),
        );
    }

    pub(crate) fn render<'b, 'a: 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) {
        let pipeline_data = self.pipeline.as_ref().unwrap();
        render_pass.set_pipeline(&pipeline_data.pipeline);
        render_pass.set_vertex_buffer(0, pipeline_data.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            pipeline_data.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..self.triangles.len() as u32, 0, 0..1);
    }
}
