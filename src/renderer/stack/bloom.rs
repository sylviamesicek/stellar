use crate::{components::BloomSettings, renderer::Graphics};

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct BloomUniform {
    threshold_precomputations: glam::Vec4,
    // viewport: Vec4,
    scale: glam::Vec2,
    aspect: f32,
    _unused: f32,
}

#[derive(Debug, Clone)]
pub struct BloomTexture {
    texture: wgpu::Texture,
    views: Vec<wgpu::TextureView>, // One view for each mip level
    sampler: wgpu::Sampler,

    bind_group_layout: wgpu::BindGroupLayout,
    bind_groups: Vec<wgpu::BindGroup>,
}

impl BloomTexture {
    pub fn new(gfx: &Graphics, bloom: &BloomSettings, physical_size: [u32; 2]) -> Self {
        let mip_count = bloom.max_mip_dimension.ilog2().max(2) - 1;
        let mip_height_ratio = if physical_size[1] != 0 {
            bloom.max_mip_dimension as f32 / physical_size[1] as f32
        } else {
            0.
        };
        let bloom_size = (glam::UVec2::from_array(physical_size).as_vec2() * mip_height_ratio)
            .round()
            .as_uvec2()
            .max(glam::UVec2::ONE);

        let texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom_attachment"),
            size: wgpu::Extent3d {
                width: bloom_size[0],
                height: bloom_size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gfx.bloom_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let bind_group_layout =
            gfx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bloom_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let mut result = Self {
            texture,
            views: Vec::new(),
            sampler,
            bind_group_layout,
            bind_groups: Vec::new(),
        };
        result.create_views_and_bind_groups(gfx);
        result
    }

    /// Ensures the bloom texture is correctly sized for this resolution.
    /// `physical_size` is the size of the viewport in pixels.
    pub fn prepare(&mut self, gfx: &Graphics, bloom: &BloomSettings, physical_size: [u32; 2]) {
        let mip_count = bloom.max_mip_dimension.ilog2().max(2) - 1;
        let mip_height_ratio = if physical_size[1] != 0 {
            bloom.max_mip_dimension as f32 / physical_size[1] as f32
        } else {
            0.
        };
        let bloom_size = (glam::UVec2::from_array(physical_size).as_vec2() * mip_height_ratio)
            .round()
            .as_uvec2()
            .max(glam::UVec2::ONE);

        if bloom_size[0] == self.texture.size().width
            && bloom_size[1] == self.texture.size().height
            && mip_count == self.texture.mip_level_count()
        {
            return;
        }

        log::info!(
            "Resizing bloom render attachment to ({}, {})",
            bloom_size[0],
            bloom_size[1]
        );

        self.texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloom_attachment"),
            size: wgpu::Extent3d {
                width: bloom_size[0],
                height: bloom_size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gfx.bloom_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.create_views_and_bind_groups(gfx);
    }

    pub fn mip_level_count(&self) -> u32 {
        self.texture.mip_level_count()
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self, mip: u32) -> &wgpu::BindGroup {
        &self.bind_groups[mip as usize]
    }

    pub fn view(&self, mip: u32) -> &wgpu::TextureView {
        &self.views[mip as usize]
    }

    fn create_views_and_bind_groups(&mut self, gfx: &Graphics) {
        self.views.clear();
        for i in 0..self.texture.mip_level_count() {
            self.views
                .push(self.texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("bloom_attachment_view"),
                    base_mip_level: i,
                    mip_level_count: Some(1),
                    ..Default::default()
                }));
        }

        self.bind_groups.clear();
        for mip in 0..self.texture.mip_level_count() {
            self.bind_groups
                .push(gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bloom_attachment_bind_group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&self.views[mip as usize]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                }))
        }
    }
}

#[derive(Debug, Clone)]
pub struct BloomUniformBuffer {
    // Data Buffer
    buffer: wgpu::Buffer,

    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl BloomUniformBuffer {
    pub fn new(gfx: &Graphics) -> Self {
        let buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_buffer"),
            size: size_of::<BloomUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            gfx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bloom_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self {
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn prepare(
        &self,
        bloom: &BloomSettings,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut wgpu::util::StagingBelt,
    ) {
        let mut uniform = staging.write_buffer(
            encoder,
            &self.buffer,
            0,
            (size_of::<BloomUniform>() as u64).try_into().unwrap(),
        );
        let threshold = bloom.prefilter.threshold;
        let threshold_softness = bloom.prefilter.threshold_softness;
        let knee = threshold * threshold_softness.clamp(0.0, 1.0);
        uniform.copy_from_slice(bytemuck::cast_slice(&[BloomUniform {
            threshold_precomputations: glam::Vec4::new(
                threshold,
                threshold - knee,
                2.0 * knee,
                0.25 / (knee + 0.00001),
            ),
            // viewport: Vec4::new(0.0, 0.0, 1.0, 1.0),
            scale: bloom.scale,
            aspect: 1.0,
            _unused: 0.0,
        }]));
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
