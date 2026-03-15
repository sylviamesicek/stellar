use crate::{
    components::{BloomCompositeMode, BloomSettings, Global},
    renderer::{Graphics, stack::hdr::HdrTextures},
};

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
struct BloomTexture {
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

        let bind_group_layout = gfx
            .start_bind_group_layout()
            .label("bloom_bind_group_layout")
            .texture_filterable_binding(
                0,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureViewDimension::D2,
                false,
            )
            .sampler_binding(
                1,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::SamplerBindingType::Filtering,
            )
            .finish();

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
            self.bind_groups.push(
                gfx.start_bind_group(&self.bind_group_layout)
                    .label("bloom_attachment_bind_group")
                    .texture_view_binding(0, self.view(mip))
                    .sampler_binding(1, &self.sampler)
                    .finish(),
            );
        }
    }
}

#[derive(Debug, Clone)]
struct BloomUniformBuffer {
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

        let bind_group_layout = gfx
            .start_bind_group_layout()
            .label("bloom_bind_group_layout")
            .uniform_buffer_binding(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT)
            .finish();

        let bind_group = gfx
            .start_bind_group(&bind_group_layout)
            .label("bloom_bind_group")
            .buffer_binding(0, &buffer, 0, None)
            .finish();

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

#[derive(Debug, Clone)]
pub struct BloomManager {
    downsample_first: wgpu::RenderPipeline,
    downsample_main: wgpu::RenderPipeline,
    upsample_main_additive: wgpu::RenderPipeline,
    upsample_last_additive: wgpu::RenderPipeline,
    upsample_main_conserving: wgpu::RenderPipeline,
    upsample_last_conserving: wgpu::RenderPipeline,

    texture: BloomTexture,
    buffer: BloomUniformBuffer,
}

impl BloomManager {
    pub fn new(gfx: &Graphics, physical_size: [u32; 2]) -> Self {
        let settings = BloomSettings::NATURAL;

        let texture = BloomTexture::new(gfx, &settings, physical_size);
        let buffer = BloomUniformBuffer::new(gfx);

        let shader = gfx.create_shader_module("bloom", wesl::include_wesl!("bloom"));
        let layout = gfx.create_pipeline_layout(
            0,
            &[texture.bind_group_layout(), buffer.bind_group_layout()],
        );

        let downsample_first = gfx
            .start_post_processing_pipeline(&shader)
            .label("bloom_downsample_first")
            .color_format(gfx.bloom_format)
            .layout(&layout)
            .add_constant("first_downsample", 1.0)
            .entry_point("downsample")
            .finish();

        let downsample_main = gfx
            .start_post_processing_pipeline(&shader)
            .label("bloom_downsample_main")
            .color_format(gfx.bloom_format)
            .layout(&layout)
            .entry_point("downsample")
            .finish();

        let color_blend_additive = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Constant,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };

        let color_blend_conserving = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Constant,
            dst_factor: wgpu::BlendFactor::OneMinusConstant,
            operation: wgpu::BlendOperation::Add,
        };

        let alpha_blend = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };

        let upsample_main_additive = gfx
            .start_post_processing_pipeline(&shader)
            .label("bloom_upsample_main")
            .color_format(gfx.bloom_format)
            .color_blend_state(wgpu::BlendState {
                color: color_blend_additive,
                alpha: alpha_blend,
            })
            .layout(&layout)
            .entry_point("upsample")
            .finish();

        let upsample_last_additive = gfx
            .start_post_processing_pipeline(&shader)
            .label("bloom_upsample_last")
            .color_format(gfx.hdr_format)
            .color_blend_state(wgpu::BlendState {
                color: color_blend_additive,
                alpha: alpha_blend,
            })
            .layout(&layout)
            .entry_point("upsample")
            .finish();

        let upsample_main_conserving = gfx
            .start_post_processing_pipeline(&shader)
            .label("bloom_upsample_main")
            .color_format(gfx.bloom_format)
            .color_blend_state(wgpu::BlendState {
                color: color_blend_conserving,
                alpha: alpha_blend,
            })
            .layout(&layout)
            .entry_point("upsample")
            .finish();

        let upsample_last_conserving = gfx
            .start_post_processing_pipeline(&shader)
            .label("bloom_upsample_last")
            .color_format(gfx.hdr_format)
            .color_blend_state(wgpu::BlendState {
                color: color_blend_conserving,
                alpha: alpha_blend,
            })
            .layout(&layout)
            .entry_point("upsample")
            .finish();

        Self {
            downsample_first,
            downsample_main,
            upsample_main_additive,
            upsample_last_additive,
            upsample_main_conserving,
            upsample_last_conserving,
            texture,
            buffer,
        }
    }

    pub fn prepare(
        &mut self,
        gfx: &Graphics,
        world: &mut hecs::World,
        physical_size: [u32; 2],
        encoder: &mut wgpu::CommandEncoder,
        staging_belt: &mut wgpu::util::StagingBelt,
    ) {
        let global_default = Global::default();
        let global = world
            .query_mut::<&Global>()
            .into_iter()
            .next()
            .unwrap_or(&global_default);

        let bloom = &global.bloom;

        self.texture.prepare(gfx, bloom, physical_size);
        self.buffer.prepare(bloom, encoder, staging_belt);
    }

    pub fn render(
        &mut self,
        _gfx: &Graphics,
        world: &mut hecs::World,
        hdr: &HdrTextures,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let global_default = Global::default();
        let global = world
            .query_mut::<&Global>()
            .into_iter()
            .next()
            .unwrap_or(&global_default);
        let bloom = &global.bloom;

        if bloom.intensity == 0.0 {
            return;
        }

        // First downsample pass
        {
            let mut downsampling_first_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bloom_downsampling_first_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: self.texture.view(0),
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations::default(),
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            downsampling_first_pass.set_pipeline(&self.downsample_first);
            downsampling_first_pass.set_bind_group(0, hdr.color_bind_group(), &[]);
            downsampling_first_pass.set_bind_group(1, self.buffer.bind_group(), &[]);
            downsampling_first_pass.draw(0..3, 0..1);
        }

        // Other downsample passes
        for mip in 1..self.texture.mip_level_count() {
            let mut downsampling_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_downsampling_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.texture.view(mip),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            downsampling_pass.set_pipeline(&self.downsample_main);
            downsampling_pass.set_bind_group(0, self.texture.bind_group(mip - 1), &[]);
            downsampling_pass.set_bind_group(1, self.buffer.bind_group(), &[]);
            downsampling_pass.draw(0..3, 0..1);
        }

        // Upsample passes
        for mip in (1..self.texture.mip_level_count()).rev() {
            let mut upsampling_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_upsampling_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.texture.view(mip - 1),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            match bloom.composite_mode {
                BloomCompositeMode::EnergyConserving => {
                    upsampling_pass.set_pipeline(&self.upsample_main_conserving)
                }
                BloomCompositeMode::Additive => {
                    upsampling_pass.set_pipeline(&self.upsample_main_additive)
                }
            }
            upsampling_pass.set_bind_group(0, self.texture.bind_group(mip), &[]);
            upsampling_pass.set_bind_group(1, self.buffer.bind_group(), &[]);
            let blend = compute_blend_factor(
                bloom,
                mip as f32,
                (self.texture.mip_level_count() - 1) as f32,
            ) as f64;
            upsampling_pass.set_blend_constant(wgpu::Color {
                r: blend,
                g: blend,
                b: blend,
                a: 1.0,
            });
            upsampling_pass.draw(0..3, 0..1);
        }

        // Last upsample pass
        {
            let mut upsampling_last_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_upsampling_final_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: hdr.color_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            match bloom.composite_mode {
                BloomCompositeMode::EnergyConserving => {
                    upsampling_last_pass.set_pipeline(&self.upsample_last_conserving)
                }
                BloomCompositeMode::Additive => {
                    upsampling_last_pass.set_pipeline(&self.upsample_last_additive)
                }
            }
            upsampling_last_pass.set_bind_group(0, self.texture.bind_group(0), &[]);
            upsampling_last_pass.set_bind_group(1, self.buffer.bind_group(), &[]);
            let blend =
                compute_blend_factor(bloom, 0.0, (self.texture.mip_level_count() - 1) as f32)
                    as f64;
            upsampling_last_pass.set_blend_constant(wgpu::Color {
                r: blend,
                g: blend,
                b: blend,
                a: 1.0,
            });
            upsampling_last_pass.draw(0..3, 0..1);
        }
    }
}

/// Calculates blend intensities of blur pyramid levels
/// during the upsampling + compositing stage.
///
/// The function assumes all pyramid levels are upsampled and
/// blended into higher frequency ones using this function to
/// calculate blend levels every time. The final (highest frequency)
/// pyramid level in not blended into anything therefore this function
/// is not applied to it. As a result, the *mip* parameter of 0 indicates
/// the second-highest frequency pyramid level (in our case that is the
/// 0th mip of the bloom texture with the original image being the
/// actual highest frequency level).
///
/// Parameters:
/// * `mip` - the index of the lower frequency pyramid level (0 - `max_mip`, where 0 indicates highest frequency mip but not the highest frequency image).
/// * `max_mip` - the index of the lowest frequency pyramid level.
///
/// This function can be visually previewed for all values of *mip* (normalized) with tweakable
/// [`Bloom`] parameters on [Desmos graphing calculator](https://www.desmos.com/calculator/ncc8xbhzzl).
fn compute_blend_factor(bloom: &BloomSettings, mip: f32, max_mip: f32) -> f32 {
    let mut lf_boost = (1.0
        - (1.0 - (mip / max_mip)).powf(1.0 / (1.0 - bloom.low_frequency_boost_curvature)))
        * bloom.low_frequency_boost;
    let high_pass_lq = 1.0
        - (((mip / max_mip) - bloom.high_pass_frequency) / bloom.high_pass_frequency)
            .clamp(0.0, 1.0);
    lf_boost *= match bloom.composite_mode {
        BloomCompositeMode::EnergyConserving => 1.0 - bloom.intensity,
        BloomCompositeMode::Additive => 1.0,
    };

    (bloom.intensity + lf_boost) * high_pass_lq
}
