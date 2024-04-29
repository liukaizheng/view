use anyhow::Result;
use web_sys::HtmlCanvasElement;
use wgpu::{
    Device, DeviceDescriptor, Queue, Surface, SurfaceConfiguration, SurfaceTarget, TextureView,
};

pub struct Renderer {
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub device: Device,
    pub queue: Queue,
    pub depth_texture_view: TextureView,
}

impl Renderer {
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self> {
        let width = canvas.width();
        let height = canvas.height();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(SurfaceTarget::Canvas(canvas))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(anyhow::anyhow!("Failed to find an appropriate adapter"))?;
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty()
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .or_else(|_| Err(anyhow::anyhow!("Failed to create device")))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let texture_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let depth_texture_view = Self::create_depth_texture(&config, &device);
        Ok(Self {
            surface,
            device,
            config,
            queue,
            depth_texture_view,
        })
    }

    #[inline]
    pub fn w(&self) -> u32 {
        self.config.width
    }

    #[inline]
    pub fn h(&self) -> u32 {
        self.config.height
    }

    fn create_depth_texture(config: &SurfaceConfiguration, device: &Device) -> TextureView {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth_texture_view = Self::create_depth_texture(&self.config, &self.device);
    }
}
