use cgmath::{Matrix4, Point3, Quaternion, SquareMatrix, Vector3};

use super::{
    render::Renderer,
    view_data::{DirtyFlags, ViewData},
    BBox,
};

use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Buffer, CompareFunction, RenderPipeline, TextureFormat,
};

use crate::render::view_data::Vertex;

struct ViewBuffer {
    camera_bind_group_layout: BindGroupLayout,
    camera_bind_group: BindGroup,
    view_buffer: Buffer,
    proj_buffer: Buffer,
    normal_mat_buffer: Buffer,
}

pub(crate) struct ViewCore {
    light_position: Vector3<f32>,

    camera_base_zoom: f32,
    pub(crate) camera_zoom: f32,
    camera_base_translation: Vector3<f32>,

    camera_near: f32,
    camera_far: f32,
    camera_fov: cgmath::Rad<f32>,

    camera_eye: Point3<f32>,
    camera_center: Point3<f32>,
    camera_up: Vector3<f32>,

    pub trackball_angle: Quaternion<f32>,

    view_buffer: Option<ViewBuffer>,

    pub(crate) material_bind_group_layout: Option<BindGroupLayout>,
    pub(crate) pipeline_cull_back: Option<RenderPipeline>,
    pub(crate) pipeline_cull_front: Option<RenderPipeline>,
}

impl Default for ViewCore {
    fn default() -> Self {
        Self {
            light_position: Vector3::new(0.0, 0.3, 0.0),
            camera_base_zoom: 1.0,
            camera_zoom: 1.0,
            camera_base_translation: Vector3::new(0.0, 0.0, 0.0),

            camera_near: 1.0,
            camera_far: 100.0,
            camera_fov: cgmath::Deg(45.0).into(),

            camera_eye: Point3::new(0.0, 0.0, 5.0),
            camera_center: Point3::new(0.0, 0.0, 0.0),
            camera_up: Vector3::new(0.0, 1.0, 0.0),

            trackball_angle: Quaternion::<f32>::new(1.0, 0.0, 0.0, 0.0),

            view_buffer: None,

            material_bind_group_layout: None,
            pipeline_cull_back: None,
            pipeline_cull_front: None,
        }
    }
}

impl ViewCore {
    pub(crate) fn render<'b, 'a: 'b>(
        &'a mut self,
        render: &'a Renderer,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        data_map: &'a mut std::collections::HashMap<u32, ViewData>,
        update_box: bool,
        update_matrix: bool,
    ) {
        if self.view_buffer.is_none() {
            let camera_bind_group_layout =
                render
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("camera_bind_group_layout"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::VERTEX,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::VERTEX,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 2,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 3,
                                visibility: wgpu::ShaderStages::VERTEX,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                        ],
                    });

            let view_buffer = render
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("view_buffer"),
                    contents: bytemuck::cast_slice(&[0.0f32; 16]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

            let proj_buffer = render
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("proj_buffer"),
                    contents: bytemuck::cast_slice(&[0.0f32; 16]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

            // Because Downlevel flags BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED are required but not supported on web
            // we use vec4 to represent light position instead of vec3
            let light_position_buffer =
                render
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("light_position_buffer"),
                        contents: bytemuck::cast_slice(&[
                            self.light_position.x,
                            self.light_position.y,
                            self.light_position.z,
                            1.0,
                        ]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

            let normal_mat_buffer =
                render
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("normal matrix buffer"),
                        contents: bytemuck::cast_slice(&[0.0f32; 16]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

            let camera_bind_group = render.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: view_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: proj_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: light_position_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: normal_mat_buffer.as_entire_binding(),
                    },
                ],
                label: None,
            });
            self.view_buffer = Some(ViewBuffer {
                camera_bind_group_layout,
                camera_bind_group,
                view_buffer,
                proj_buffer,
                normal_mat_buffer,
            });
        }

        if self.material_bind_group_layout.is_none() {
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
                        bind_group_layouts: &[
                            &self
                                .view_buffer
                                .as_ref()
                                .unwrap()
                                .camera_bind_group_layout,
                            &material_bind_group_layout,
                        ],
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

            let render_pipeline_cull_back =
                render.device.create_render_pipeline(&pipeline_desc_base);

            let mut pipeline_desc_inv = pipeline_desc_base.clone();
            pipeline_desc_inv.primitive.cull_mode = Some(wgpu::Face::Front);
            let render_pipeline_cull_front =
                render.device.create_render_pipeline(&pipeline_desc_inv);

            self.material_bind_group_layout = Some(material_bind_group_layout);
            self.pipeline_cull_back = Some(render_pipeline_cull_back);
            self.pipeline_cull_front = Some(render_pipeline_cull_front);
        }
        let mut has_dirty_data = false;
        for data in data_map.values_mut() {
            if !data.visible {
                continue;
            }

            if data.pipeline.is_none() {
                data.init_resources(
                    render,
                    &self.material_bind_group_layout.as_ref().unwrap(),
                );
            }
            if data.dirty.contains(DirtyFlags::DIRTY_VERTEX) {
                has_dirty_data = true;
                data.update_vertex_buffer(render);
                data.dirty.remove(DirtyFlags::DIRTY_VERTEX);
            }

            if data.dirty.contains(DirtyFlags::DIRTY_MATERIAL) {
                data.update_material(render);
                data.dirty.remove(DirtyFlags::DIRTY_MATERIAL);
            }
        }

        if update_box || has_dirty_data {
            if !data_map.is_empty() {
                let mut bbox = BBox::default();
                for data in data_map.values() {
                    bbox.merge_box(&data.bbox);
                }
                let center = (bbox.min + bbox.max) / 2.0;
                self.camera_base_translation = -center;
                self.camera_base_zoom = 1.0 / bbox.max_len();
            }
        }

        if has_dirty_data || update_matrix {
            self.update_matrix(render);
        }

        render_pass.set_bind_group(
            0,
            &self.view_buffer.as_ref().unwrap().camera_bind_group,
            &[],
        );
        for data in data_map.values() {
            if data.visible {
                data.render(
                    render_pass,
                    self.pipeline_cull_back.as_ref().unwrap(),
                    self.pipeline_cull_front.as_ref().unwrap(),
                );
            }
        }
    }

    fn update_matrix(&self, render: &Renderer) {
        let view = Matrix4::look_at_rh(self.camera_eye, self.camera_center, self.camera_up)
            * Matrix4::from_scale(self.camera_base_zoom * self.camera_zoom)
            * Matrix4::from(self.trackball_angle)
            * Matrix4::from_translation(self.camera_base_translation);
        let mut normal_mat = view.invert().expect("failed to invert the view matrix");
        normal_mat.transpose_self();

        let w = render.w() as f32;
        let h = render.h() as f32;
        let proj = cgmath::perspective(self.camera_fov, w / h, self.camera_near, self.camera_far);

        let view_data: [[f32; 4]; 4] = view.into();
        let normal_mat_data: [[f32; 4]; 4] = normal_mat.into();
        let proj_data: [[f32; 4]; 4] = proj.into();
        render.queue.write_buffer(
            &self.view_buffer.as_ref().unwrap().view_buffer,
            0,
            bytemuck::cast_slice(&view_data),
        );
        render.queue.write_buffer(
            &self.view_buffer.as_ref().unwrap().proj_buffer,
            0,
            bytemuck::cast_slice(&proj_data),
        );
        render.queue.write_buffer(
            &self.view_buffer.as_ref().unwrap().normal_mat_buffer,
            0,
            bytemuck::cast_slice(&normal_mat_data),
        );
    }
}
