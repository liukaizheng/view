use std::borrow::Cow;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use leptos::html::Canvas;
use leptos::*;
use web_sys::HtmlCanvasElement;
use wgpu::{Adapter, Device, Queue, RenderPipeline, Surface, SurfaceConfiguration};

struct Renderer {
    adapter: Adapter,
    surface: Surface,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    pipline: RenderPipeline,
}

impl Renderer {
    pub async fn new(canvas: HtmlCanvasElement) -> Self {
        let width = canvas.width();
        let height = canvas.height();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface_from_canvas(canvas)
            .expect("Failed to get surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an apporpriate adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty()
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to create device");
        // Load the shaders from disk
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        Self {
            adapter,
            surface,
            device,
            config,
            queue,
            pipline: render_pipeline,
        }
    }

    fn render(&self) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Falied to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.pipline);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

#[component]
pub fn Canvas() -> impl IntoView {
    let canvas: NodeRef<Canvas> = create_node_ref();
    let render: Rc<RefCell<Option<Renderer>>> = Default::default();
    let render_clone = render.clone();
    let on_mount = move |_| {
        if let Some(canvas) = canvas.get() {
            // canvas.set_width(1000);
            // canvas.set_height(1000);
            spawn_local(async move {
                let canvas = canvas.deref();
                *render_clone.borrow_mut() = Some(Renderer::new(canvas.clone()).await);
            })
        }
    };

    let on_click = move |_| {
        let render = render.borrow();
        if let Some(ref render) = *render {
            render.render();
        }
    };
    let view = view! {
        <div class = "w-full ml-10">
            <canvas node_ref = canvas/>
            <button on:click = on_click>render</button>
        </div>
    }
    .on_mount(on_mount);
    view
}