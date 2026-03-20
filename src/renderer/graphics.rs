use std::{borrow::Cow, num::NonZero};

use smallvec::SmallVec;
use wesl::include_wesl;

pub struct Graphics {
    pub _instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,

    pub surface_format: wgpu::TextureFormat,
    pub hdr_format: wgpu::TextureFormat,
    pub bloom_format: wgpu::TextureFormat,

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

        let wgpu::Features {
            features_wgpu,
            features_webgpu,
        } = adapter.features();

        assert!(
            features_wgpu.contains(wgpu::FeaturesWGPU::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
        );
        assert!(features_webgpu.contains(wgpu::FeaturesWebGPU::RG11B10UFLOAT_RENDERABLE));

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("GPU Device"),
                memory_hints: wgpu::MemoryHints::Performance,
                required_features: wgpu::Features {
                    features_wgpu: wgpu::FeaturesWGPU::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    features_webgpu: wgpu::FeaturesWebGPU::RG11B10UFLOAT_RENDERABLE
                        | wgpu::FeaturesWebGPU::IMMEDIATES,
                },
                required_limits: wgpu::Limits {
                    max_immediate_size: 128,
                    max_color_attachment_bytes_per_sample: 48,
                    ..wgpu::Limits::defaults()
                }
                .using_resolution(adapter.limits()),
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

        let hdr_format_candidates = [
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Rgba8Unorm,
        ];

        let hdr_format = hdr_format_candidates
            .into_iter()
            .find(|format| {
                adapter
                    .get_texture_format_features(*format)
                    .allowed_usages
                    .contains(
                        wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                    )
            })
            .expect("Unable to find valid HDR texture format for post-processing");

        log::info!("HDR format: {:?}", hdr_format);

        let bloom_format_candidates = [
            wgpu::TextureFormat::Rg11b10Ufloat,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Rgba8Unorm,
        ];

        let bloom_format = bloom_format_candidates
            .into_iter()
            .find(|format| {
                adapter
                    .get_texture_format_features(*format)
                    .allowed_usages
                    .contains(
                        wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                    )
            })
            .expect("Unable to find valid bloom texture format for post-processing");

        log::info!("Bloom format: {:?}", bloom_format);

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
            hdr_format,
            bloom_format,
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

    pub fn start_bind_group_layout<'a>(&'a self) -> BindGroupLayoutBuilder<'a> {
        BindGroupLayoutBuilder {
            gfx: self,
            label: None,
            entries: SmallVec::new(),
        }
    }

    pub fn start_bind_group<'a>(
        &'a self,
        layout: &'a wgpu::BindGroupLayout,
    ) -> BindGroupBuilder<'a> {
        BindGroupBuilder {
            gfx: self,
            label: None,
            entries: SmallVec::new(),
            layout,
        }
    }

    pub fn start_post_processing_pipeline<'a>(
        &'a self,
        shader: &'a wgpu::ShaderModule,
    ) -> PostProcessingBuilder<'a> {
        PostProcessingBuilder {
            gfx: self,
            shader: shader,
            color_format: self.hdr_format,
            color_blend_state: None,
            name: None,
            layout: None,
            constants: SmallVec::new(),
            entry_point: None,
        }
    }

    pub fn fullscreen_vertex_state<'a>(&'a self) -> wgpu::VertexState<'a> {
        wgpu::VertexState {
            module: &self.fullscreen_shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        }
    }

    pub fn fullscreen_vertex_state_with_constants<'a>(
        &'a self,
        constants: &'a [(&'a str, f64)],
    ) -> wgpu::VertexState<'a> {
        wgpu::VertexState {
            module: &self.fullscreen_shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants,
                ..Default::default()
            },
            buffers: &[],
        }
    }

    pub fn fullscreen_primitive_state(&self) -> wgpu::PrimitiveState {
        wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        }
    }
}

pub struct BindGroupLayoutBuilder<'a> {
    gfx: &'a Graphics,
    label: Option<&'a str>,
    entries: SmallVec<[wgpu::BindGroupLayoutEntry; 4]>,
}

impl<'a> BindGroupLayoutBuilder<'a> {
    pub fn label(mut self, name: &'a str) -> Self {
        self.label = Some(name);
        self
    }

    pub fn uniform_binding(mut self, binding: u32, visibility: wgpu::ShaderStages) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    pub fn storage_buffer_binding(
        mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        read_only: bool,
    ) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self
    }

    pub fn sampler_binding(
        mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        ty: wgpu::SamplerBindingType,
    ) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Sampler(ty),
            count: None,
        });
        self
    }

    pub fn texture_binding(
        mut self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        sample_type: wgpu::TextureSampleType,
        view_dimension: wgpu::TextureViewDimension,
        multisampled: bool,
    ) -> Self {
        self.entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled,
            },
            count: None,
        });
        self
    }

    pub fn texture_filterable_binding(
        self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        view_dimension: wgpu::TextureViewDimension,
        multisampled: bool,
    ) -> Self {
        self.texture_binding(
            binding,
            visibility,
            wgpu::TextureSampleType::Float { filterable: true },
            view_dimension,
            multisampled,
        )
    }

    pub fn texture_depth_binding(
        self,
        binding: u32,
        visibility: wgpu::ShaderStages,
        view_dimension: wgpu::TextureViewDimension,
        multisampled: bool,
    ) -> Self {
        self.texture_binding(
            binding,
            visibility,
            wgpu::TextureSampleType::Depth,
            view_dimension,
            multisampled,
        )
    }

    pub fn finish(self) -> wgpu::BindGroupLayout {
        self.gfx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: self.label,
                entries: &self.entries,
            })
    }
}

pub struct BindGroupBuilder<'a> {
    gfx: &'a Graphics,
    label: Option<&'a str>,
    entries: SmallVec<[wgpu::BindGroupEntry<'a>; 4]>,
    layout: &'a wgpu::BindGroupLayout,
}

impl<'a> BindGroupBuilder<'a> {
    pub fn label(mut self, name: &'a str) -> Self {
        self.label = Some(name);
        self
    }

    pub fn buffer_binding(
        mut self,
        binding: u32,
        buffer: &'a wgpu::Buffer,
        offset: u64,
        size: impl Into<Option<u64>>,
    ) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset,
                size: size.into().map(|i| NonZero::new(i).unwrap()),
            }),
        });
        self
    }

    pub fn sampler_binding(mut self, binding: u32, sampler: &'a wgpu::Sampler) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
        self
    }

    pub fn texture_view_binding(mut self, binding: u32, view: &'a wgpu::TextureView) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        });
        self
    }

    pub fn finish(self) -> wgpu::BindGroup {
        self.gfx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: self.label,
                entries: &self.entries,
                layout: self.layout,
            })
    }
}

pub struct PostProcessingBuilder<'a> {
    gfx: &'a Graphics,
    shader: &'a wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    color_blend_state: Option<wgpu::BlendState>,
    name: Option<&'a str>,
    layout: Option<&'a wgpu::PipelineLayout>,
    constants: SmallVec<[(&'a str, f64); 4]>,
    entry_point: Option<&'a str>,
}

impl<'a> PostProcessingBuilder<'a> {
    pub fn label(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    pub fn layout(mut self, layout: &'a wgpu::PipelineLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn color_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.color_format = format;
        self
    }

    pub fn color_blend_state(mut self, state: wgpu::BlendState) -> Self {
        self.color_blend_state = Some(state);
        self
    }

    pub fn add_constant(mut self, name: &'a str, value: f64) -> Self {
        self.constants.push((name, value));
        self
    }

    pub fn entry_point(mut self, name: &'a str) -> Self {
        self.entry_point = Some(name);
        self
    }

    pub fn finish(self) -> wgpu::RenderPipeline {
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
                    entry_point: self.entry_point,
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &self.constants,
                        ..Default::default()
                    },
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.color_format,
                        blend: self.color_blend_state,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
}
