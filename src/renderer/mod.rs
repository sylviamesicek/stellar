mod ui;

use ui::UiRenderer;

pub use ui::ScreenDescriptor;

pub struct Graphics {
    pub _instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
}

impl Graphics {
    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to request gpu adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("GPU Device"),
                memory_hints: wgpu::MemoryHints::Performance,
                required_features: wgpu::Features {
                    features_wgpu: wgpu::FeaturesWGPU::default(),
                    features_webgpu: wgpu::FeaturesWebGPU::default(),
                },
                required_limits: wgpu::Limits::defaults().using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to request gpu device");

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        Self {
            _instance: instance,
            surface,
            device,
            queue,
            surface_config,
            surface_format,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

pub struct Renderer {
    ui: UiRenderer,
}

impl Renderer {
    pub fn new(gfx: &Graphics) -> Self {
        let ui = UiRenderer::new(&gfx.device, gfx.surface_format);

        Self { ui }
    }
}

impl Renderer {
    pub fn prepare_ui(
        &mut self,
        gfx: &Graphics,
        screen: ScreenDescriptor,
        textures_delta: &egui::TexturesDelta,
        paint_jobs: &[egui::ClippedPrimitive],
        encoder: &mut wgpu::CommandEncoder,
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.ui
                .update_texture(&gfx.device, &gfx.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.ui.free_texture(id);
        }

        self.ui
            .update_buffers(&gfx.device, &gfx.queue, encoder, &paint_jobs, &screen);
    }

    pub fn render_frame(
        &mut self,
        gfx: &Graphics,
        surface_view: &wgpu::TextureView,
        paint_jobs: &[egui::ClippedPrimitive],
        screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // Begin render pass
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.19,
                        g: 0.24,
                        b: 0.42,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        self.ui
            .draw(&mut render_pass.forget_lifetime(), paint_jobs, screen);

        // drop(render_pass);
    }
}
