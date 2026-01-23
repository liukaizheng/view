use super::{render::Renderer, BBox};

use cgmath::Vector3;
use wgpu::{util::DeviceExt, Buffer, CompareFunction, RenderPass, RenderPipeline, TextureFormat};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct DirtyFlags: u32 {
        const DIRTY_NONE = 0b00000000;
        const DIRTY_VERTEX = 0b00000001;
        const DIRTY_EDGE = 0b00000010;
        const DIRTY_FACE = 0b00000100;
        const DIRTY_MATERIAL = 0b00001000;
        const DIRTY_ALL = Self::DIRTY_VERTEX.bits() | Self::DIRTY_EDGE.bits() | Self::DIRTY_FACE.bits() | Self::DIRTY_MATERIAL.bits();
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) point: [f32; 3],
    pub(crate) normal: [f32; 3],
    pub(crate) barycentric: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Material {
    /// ambient color
    pub(crate) ka: [f32; 4],
    /// diffuse color
    pub(crate) kd: [f32; 4],
    /// specular color
    pub(crate) ks: [f32; 4],
    /// edge color
    pub(crate) edge_color: [f32; 4],
    /// edge width
    pub(crate) edge_width: f32,
    pub(crate) _pad: [f32; 3],
}

impl Material {
    pub(crate) fn new(color: Vector3<f32>) -> Self {
        let kd = color;
        let ka = kd * 0.1;
        const GREY: Vector3<f32> = Vector3::new(0.3, 0.3, 0.3);
        let ks = GREY + 0.1 * (kd - GREY);
        Self {
            ka: [ka.x, ka.y, ka.z, 1.0],
            kd: [kd.x, kd.y, kd.z, 1.0],
            ks: [ks.x, ks.y, ks.z, 1.0],
            edge_color: [0.0, 0.0, 0.0, 1.0],
            edge_width: 0.0,
            _pad: [0.0; 3],
        }
    }
}

pub(crate) struct MeshPipeline {
    material_bind_group: wgpu::BindGroup,
    pipeline_cull_back: RenderPipeline,
    pipeline_cull_front: RenderPipeline,

    material_buffer: Buffer,
    vertex_buffer: Buffer,
}
pub(crate) struct ViewData {
    vertices: Vec<Vertex>,
    pub(crate) material: Material,
    pub(crate) dirty: DirtyFlags,
    pub(crate) bbox: BBox,
    pub(crate) pipeline: Option<MeshPipeline>,
    pub(crate) visible: bool,
}

impl ViewData {
    pub(crate) fn new(vertices: Vec<Vertex>, material: Material) -> Self {
        Self {
            vertices,
            material,
            dirty: DirtyFlags::DIRTY_ALL,
            bbox: BBox::default(),
            pipeline: None,
            visible: true,
        }
    }

    pub(crate) fn init_pipline(
        &mut self,
        render: &Renderer,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        let material_bind_group_layout =
            render
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("camera_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let material_buffer = render
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::bytes_of(&self.material),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let material_bind_group = render.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &material_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: material_buffer.as_entire_binding(),
            }],
            label: None,
        });

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
                    bind_group_layouts: &[camera_bind_group_layout, &material_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline_desc_base = wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: render.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
        };

        let render_pipeline_cull_back = render.device.create_render_pipeline(&pipeline_desc_base);

        let mut pipeline_desc_inv = pipeline_desc_base.clone();
        pipeline_desc_inv.primitive.cull_mode = Some(wgpu::Face::Front);
        let render_pipeline_cull_front = render.device.create_render_pipeline(&pipeline_desc_inv);

        let vertex_buffer = render
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX |
                // allow it to be the destination for [`Queue::write_buffer`] operation
                wgpu::BufferUsages::COPY_DST,
            });

        self.pipeline = Some(MeshPipeline {
            material_bind_group,
            pipeline_cull_back: render_pipeline_cull_back,
            pipeline_cull_front: render_pipeline_cull_front,
            material_buffer,
            vertex_buffer,
        });
    }

    #[inline]
    pub(crate) fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
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
    pub(crate) fn update_material(&mut self, render: &Renderer) {
        render.queue.write_buffer(
            &self.pipeline.as_ref().unwrap().material_buffer,
            0,
            bytemuck::bytes_of(&self.material),
        )
    }

    pub(crate) fn render<'b, 'a: 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) {
        let pipeline_data = self.pipeline.as_ref().unwrap();
        render_pass.set_bind_group(1, &pipeline_data.material_bind_group, &[]);
        render_pass.set_vertex_buffer(0, pipeline_data.vertex_buffer.slice(..));
        
        // Check alpha for transparency
        if self.material.kd[3] < 0.999 {
            // Pass 1: Draw back faces (Cull Front)
            render_pass.set_pipeline(&pipeline_data.pipeline_cull_front);
            render_pass.draw(0..self.vertices.len() as u32, 0..1);

            // Pass 2: Draw front faces (Cull Back)
            render_pass.set_pipeline(&pipeline_data.pipeline_cull_back);
            render_pass.draw(0..self.vertices.len() as u32, 0..1);
        } else {
             // Opaque: Draw front faces (Cull Back)
             render_pass.set_pipeline(&pipeline_data.pipeline_cull_back);
             render_pass.draw(0..self.vertices.len() as u32, 0..1);
        }
    }
}

#[inline]
fn box_from_points(vertices: &[Vertex]) -> BBox {
    let data = vertices.iter().fold(
        (f32::MAX, f32::MAX, f32::MAX, f32::MIN, f32::MIN, f32::MIN),
        |(min_x, min_y, min_z, max_x, max_y, max_z), v| {
            let x = v.point[0];
            let y = v.point[1];
            let z = v.point[2];
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
