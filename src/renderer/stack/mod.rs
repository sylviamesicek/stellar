use wesl::include_wesl;
use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferUsages, FilterMode, MipmapFilterMode, RenderPass, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, TextureSampleType, TextureView,
    TextureViewDescriptor, TextureViewDimension,
};

use super::Graphics;
use crate::{
    components::{BloomCompositeMode, BloomSettings, Camera, Global},
    math::Transform,
};
use glam::{Mat4, Vec2, Vec4};

mod bloom;

use bloom::{BloomTexture, BloomUniformBuffer};

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CameraUniform {
    proj: Mat4,
    view: Mat4,

    inv_proj: Mat4,
    inv_view: Mat4,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct GlobalUniform {
    time: f32,
    pre_saturation: f32,
    post_saturation: f32,
    gamma: f32,
    exposure: f32,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct BloomUniform {
    threshold_precomputations: Vec4,
    // viewport: Vec4,
    scale: Vec2,
    aspect: f32,
    _unused: f32,
}

#[derive(Debug)]
pub struct RenderStack {
    pub physical_size: [u32; 2],

    // Camera data
    camera: hecs::Entity,
    camera_buffer: wgpu::Buffer,
    // Global Params Data
    global_buffer: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,

    // HDR color texture (primary render target)
    hdr_color: wgpu::Texture,
    hdr_color_view: TextureView,
    hdr_sampler: wgpu::Sampler,
    hdr_bind_group_layout: BindGroupLayout,
    hdr_bind_group: wgpu::BindGroup,

    // Sierpinksi Pipeline
    fractal: [wgpu::RenderPipeline; 2],

    // Bloom Pipelines
    bloom_downsample_first: wgpu::RenderPipeline,
    bloom_downsample_main: wgpu::RenderPipeline,
    bloom_upsample_main_additive: wgpu::RenderPipeline,
    bloom_upsample_last_additive: wgpu::RenderPipeline,
    bloom_upsample_main_conserving: wgpu::RenderPipeline,
    bloom_upsample_last_conserving: wgpu::RenderPipeline,

    bloom_texture: BloomTexture,
    bloom_buffer: BloomUniformBuffer,

    // Composite Pipeline
    composite: wgpu::RenderPipeline,

    staging_belt: wgpu::util::StagingBelt,
}

impl RenderStack {
    pub fn new(gfx: &Graphics, camera: hecs::Entity, physical_size: [u32; 2]) -> Self {
        let camera_buffer = gfx.device.create_buffer(&BufferDescriptor {
            label: Some("camera_buffer"),
            size: size_of::<CameraUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let global_buffer = gfx.device.create_buffer(&BufferDescriptor {
            label: Some("global_buffer"),
            size: size_of::<GlobalUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let frame_bind_group_layout =
            gfx.device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("frame_bind_group_layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let frame_bind_group = gfx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("frame_bind_group"),
            layout: &frame_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &camera_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &global_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let hdr_color = create_hdr_color(gfx, physical_size[0], physical_size[1]);
        let hdr_color_view = create_hdr_color_view(&hdr_color);
        let hdr_sampler = create_hdr_sampler(gfx);

        let hdr_bind_group_layout =
            gfx.device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("hdr_bind_group_layout"),
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let hdr_bind_group =
            create_hdr_bind_group(gfx, &hdr_bind_group_layout, &hdr_color_view, &hdr_sampler);

        // *******************************
        // Bloom Pipelines

        let bloom_settings = BloomSettings::NATURAL;

        let bloom_texture = BloomTexture::new(gfx, &bloom_settings, physical_size);
        let bloom_buffer = BloomUniformBuffer::new(gfx);

        let bloom_shader = gfx.create_shader_module("bloom", include_wesl!("bloom"));
        let bloom_layout = gfx.create_pipeline_layout(
            0,
            &[
                bloom_texture.bind_group_layout(),
                bloom_buffer.bind_group_layout(),
            ],
        );

        let bloom_downsample_first = gfx
            .post_processing_pipeline(&bloom_shader)
            .name("bloom_downsample_first")
            .color_format(gfx.bloom_format)
            .layout(&bloom_layout)
            .add_constant("first_downsample", 1.0)
            .entry_point("downsample")
            .build();

        let bloom_downsample_main = gfx
            .post_processing_pipeline(&bloom_shader)
            .name("bloom_downsample_main")
            .color_format(gfx.bloom_format)
            .layout(&bloom_layout)
            .entry_point("downsample")
            .build();

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

        let bloom_upsample_main_additive = gfx
            .post_processing_pipeline(&bloom_shader)
            .name("bloom_upsample_main")
            .color_format(gfx.bloom_format)
            .color_blend_state(wgpu::BlendState {
                color: color_blend_additive,
                alpha: alpha_blend,
            })
            .layout(&bloom_layout)
            .entry_point("upsample")
            .build();

        let bloom_upsample_last_additive = gfx
            .post_processing_pipeline(&bloom_shader)
            .name("bloom_upsample_last")
            .color_format(gfx.hdr_format)
            .color_blend_state(wgpu::BlendState {
                color: color_blend_additive,
                alpha: alpha_blend,
            })
            .layout(&bloom_layout)
            .entry_point("upsample")
            .build();

        let bloom_upsample_main_conserving = gfx
            .post_processing_pipeline(&bloom_shader)
            .name("bloom_upsample_main")
            .color_format(gfx.bloom_format)
            .color_blend_state(wgpu::BlendState {
                color: color_blend_conserving,
                alpha: alpha_blend,
            })
            .layout(&bloom_layout)
            .entry_point("upsample")
            .build();

        let bloom_upsample_last_conserving = gfx
            .post_processing_pipeline(&bloom_shader)
            .name("bloom_upsample_last")
            .color_format(gfx.hdr_format)
            .color_blend_state(wgpu::BlendState {
                color: color_blend_conserving,
                alpha: alpha_blend,
            })
            .layout(&bloom_layout)
            .entry_point("upsample")
            .build();

        // *******************************
        // Composite Pipeline

        let composite_shader = gfx.create_shader_module("composite", include_wesl!("composite"));
        let composite_layout =
            gfx.create_pipeline_layout(0, &[&hdr_bind_group_layout, &frame_bind_group_layout]);
        let composite = gfx
            .post_processing_pipeline(&composite_shader)
            .name("composite")
            .color_format(gfx.surface_format)
            .layout(&composite_layout)
            .build();

        // *****************************
        // Sierpinski Pipeline

        let fractal_shader = gfx.create_shader_module("fractal", include_wesl!("fractal"));
        let fractal_layout = gfx.create_pipeline_layout(0, &[&frame_bind_group_layout]);

        let mandlebulb = gfx
            .post_processing_pipeline(&fractal_shader)
            .name("sierpinski")
            .color_format(gfx.hdr_format)
            .layout(&fractal_layout)
            .add_constant("fractal_type", 0.0)
            .build();

        let sierpinski = gfx
            .post_processing_pipeline(&fractal_shader)
            .name("sierpinski")
            .color_format(gfx.hdr_format)
            .layout(&fractal_layout)
            .add_constant("fractal_type", 1.0)
            .build();

        // ******************************
        // Staging Belt

        let staging_belt = wgpu::util::StagingBelt::new(gfx.device.clone(), 1024);

        Self {
            physical_size,

            camera,
            camera_buffer,
            global_buffer,
            frame_bind_group,

            bloom_texture,
            bloom_buffer,
            bloom_downsample_first,
            bloom_downsample_main,
            bloom_upsample_main_additive,
            bloom_upsample_last_additive,
            bloom_upsample_main_conserving,
            bloom_upsample_last_conserving,

            hdr_color,
            hdr_color_view,
            hdr_sampler,
            hdr_bind_group_layout,
            hdr_bind_group,

            composite,

            fractal: [mandlebulb, sierpinski],

            staging_belt,
        }
    }

    pub fn resize(&mut self, gfx: &Graphics, physical_size: [u32; 2]) {
        if self.physical_size == physical_size {
            return;
        }

        self.physical_size = physical_size;

        log::info!(
            "Resizing render stack viewport to ({}, {})",
            physical_size[0],
            physical_size[1]
        );

        self.hdr_color = create_hdr_color(gfx, physical_size[0], physical_size[1]);
        self.hdr_color_view = create_hdr_color_view(&self.hdr_color);
        self.hdr_bind_group = create_hdr_bind_group(
            gfx,
            &self.hdr_bind_group_layout,
            &self.hdr_color_view,
            &self.hdr_sampler,
        );
    }

    pub fn prepare(
        &mut self,
        gfx: &Graphics,
        world: &mut hecs::World,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // Camera
        {
            let camera = world.get::<&Camera>(self.camera).unwrap();
            let transform = world.get::<&Transform>(self.camera).unwrap();

            let proj = camera.projection.get_clip_from_view();
            let inv_proj = proj.inverse();
            let view = transform.to_matrix().inverse();
            let inv_view = transform.to_matrix();

            let mut uniform = self.staging_belt.write_buffer(
                encoder,
                &self.camera_buffer,
                0,
                (size_of::<CameraUniform>() as u64).try_into().unwrap(),
            );
            uniform.copy_from_slice(bytemuck::cast_slice(&[CameraUniform {
                proj,
                view,
                inv_proj,
                inv_view,
            }]));
        }

        // Global
        {
            let global_default = Global::default();
            let global = world
                .query_mut::<&Global>()
                .into_iter()
                .next()
                .unwrap_or(&global_default);

            let time = global.time.as_secs_f32();

            let mut uniform = self.staging_belt.write_buffer(
                encoder,
                &self.global_buffer,
                0,
                (size_of::<GlobalUniform>() as u64).try_into().unwrap(),
            );
            uniform.copy_from_slice(bytemuck::cast_slice(&[GlobalUniform {
                time,
                pre_saturation: global.tonemap.pre_saturation,
                post_saturation: global.tonemap.post_saturation,
                gamma: global.tonemap.gamma,
                exposure: global.tonemap.exposure,
            }]));

            // Bloom
            let bloom = &global.bloom;

            self.bloom_texture.prepare(gfx, bloom, self.physical_size);
            self.bloom_buffer
                .prepare(bloom, encoder, &mut self.staging_belt);
        }

        self.staging_belt.finish();
    }

    pub fn render(
        &mut self,
        _gfx: &Graphics,
        world: &mut hecs::World,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let global_default = Global::default();
        let global = world
            .query_mut::<&Global>()
            .into_iter()
            .next()
            .unwrap_or(&global_default);

        let bloom = &global.bloom;

        let fractal_index = match global.pipeline {
            crate::components::Pipeline::Mandlebulb => 0,
            crate::components::Pipeline::Sierpinski => 1,
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.hdr_color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 0.24,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        render_pass.set_pipeline(&self.fractal[fractal_index]);
        render_pass.set_bind_group(0, &self.frame_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        drop(render_pass);

        // Bloom render passes
        if bloom.intensity != 0.0 {
            // First downsample pass
            {
                let mut downsampling_first_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("bloom_downsampling_first_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: self.bloom_texture.view(0),
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                downsampling_first_pass.set_pipeline(&self.bloom_downsample_first);
                downsampling_first_pass.set_bind_group(0, &self.hdr_bind_group, &[]);
                downsampling_first_pass.set_bind_group(1, self.bloom_buffer.bind_group(), &[]);
                downsampling_first_pass.draw(0..3, 0..1);
            }

            // Other downsample passes
            for mip in 1..self.bloom_texture.mip_level_count() {
                let mut downsampling_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("bloom_downsampling_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: self.bloom_texture.view(mip),
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                downsampling_pass.set_pipeline(&self.bloom_downsample_main);
                downsampling_pass.set_bind_group(0, self.bloom_texture.bind_group(mip - 1), &[]);
                downsampling_pass.set_bind_group(1, self.bloom_buffer.bind_group(), &[]);
                downsampling_pass.draw(0..3, 0..1);
            }

            // Upsample passes
            for mip in (1..self.bloom_texture.mip_level_count()).rev() {
                let mut upsampling_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bloom_upsampling_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: self.bloom_texture.view(mip - 1),
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
                        upsampling_pass.set_pipeline(&self.bloom_upsample_main_conserving)
                    }
                    BloomCompositeMode::Additive => {
                        upsampling_pass.set_pipeline(&self.bloom_upsample_main_additive)
                    }
                }
                upsampling_pass.set_bind_group(0, self.bloom_texture.bind_group(mip), &[]);
                upsampling_pass.set_bind_group(1, self.bloom_buffer.bind_group(), &[]);
                let blend = compute_blend_factor(
                    bloom,
                    mip as f32,
                    (self.bloom_texture.mip_level_count() - 1) as f32,
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
                let mut upsampling_last_pass =
                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("bloom_upsampling_final_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.hdr_color_view,
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
                        upsampling_last_pass.set_pipeline(&self.bloom_upsample_last_conserving)
                    }
                    BloomCompositeMode::Additive => {
                        upsampling_last_pass.set_pipeline(&self.bloom_upsample_last_additive)
                    }
                }
                upsampling_last_pass.set_bind_group(0, self.bloom_texture.bind_group(0), &[]);
                upsampling_last_pass.set_bind_group(1, self.bloom_buffer.bind_group(), &[]);
                let blend = compute_blend_factor(
                    bloom,
                    0.0,
                    (self.bloom_texture.mip_level_count() - 1) as f32,
                ) as f64;
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

    pub fn recall(&mut self, _gfx: &Graphics, _world: &mut hecs::World) {
        self.staging_belt.recall();
    }

    pub fn draw_composite(&self, render_pass: &mut RenderPass<'static>) {
        render_pass.set_pipeline(&self.composite);
        render_pass.set_bind_group(0, &self.hdr_bind_group, &[]);
        render_pass.set_bind_group(1, &self.frame_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

fn create_hdr_color(gfx: &Graphics, width: u32, height: u32) -> wgpu::Texture {
    let texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hdr_color_attachment"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: gfx.hdr_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture
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

fn create_hdr_color_view(texture: &wgpu::Texture) -> TextureView {
    texture.create_view(&TextureViewDescriptor {
        label: Some("hdr_color_attachment_view"),
        ..Default::default()
    })
}

fn create_hdr_sampler(gfx: &Graphics) -> wgpu::Sampler {
    gfx.device.create_sampler(&SamplerDescriptor {
        label: Some("hdr_sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Nearest,
        ..Default::default()
    })
}

fn create_hdr_bind_group(
    gfx: &Graphics,
    hdr_layout: &BindGroupLayout,
    hdr_color_view: &TextureView,
    hdr_sampler: &Sampler,
) -> wgpu::BindGroup {
    gfx.device.create_bind_group(&BindGroupDescriptor {
        label: Some("hdr_bind_group"),
        layout: &hdr_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&hdr_color_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(&hdr_sampler),
            },
        ],
    })
}
