use std::borrow::Cow;

use smallvec::SmallVec;
use wesl::include_wesl;

pub struct Graphics {
    pub _instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,

    fullscreen_shader: wgpu::ShaderModule,
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

        // Temporary
        if surface_format.is_srgb() {
            panic!("SRGB render target currently not supported (see tonemap.frag.wgsl)!")
        }

        log::info!("Surface format: {:?}", surface_format);

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

        let fullscreen_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fullscreen"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_wesl!("fullscreen"))),
        });

        Self {
            _instance: instance,
            surface,
            device,
            queue,
            surface_config,
            surface_format,
            fullscreen_shader,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn create_shader_module(&self, name: &str, source: &str) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
            })
    }

    pub fn create_pipeline_layout(
        &self,
        immediate_size: u32,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::PipelineLayout {
        self.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts,
                immediate_size,
            })
    }

    pub fn post_processing_pipeline<'a>(
        &'a self,
        shader: &'a wgpu::ShaderModule,
        format: wgpu::TextureFormat,
    ) -> PostProcessingBuilder<'a> {
        PostProcessingBuilder {
            gfx: self,
            shader: shader,
            color_format: format,
            name: None,
            layout: None,
            constants: SmallVec::new(),
        }
    }
}

pub struct PostProcessingBuilder<'a> {
    gfx: &'a Graphics,
    shader: &'a wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    name: Option<&'a str>,
    layout: Option<&'a wgpu::PipelineLayout>,
    constants: SmallVec<[(&'a str, f64); 4]>,
}

impl<'a> PostProcessingBuilder<'a> {
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    pub fn layout(mut self, layout: &'a wgpu::PipelineLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn add_constant(mut self, name: &'a str, value: f64) -> Self {
        self.constants.push((name, value));
        self
    }

    pub fn build(mut self) -> wgpu::RenderPipeline {
        self.gfx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: self.name,
                layout: self.layout,
                vertex: wgpu::VertexState {
                    module: &self.gfx.fullscreen_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &self.constants,
                        ..Default::default()
                    },
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.color_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
}
