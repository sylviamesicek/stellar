use wesl::include_wesl;
use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, BufferBinding,
    BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, FilterMode,
    FragmentState, MipmapFilterMode, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, RenderPass, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, TextureSampleType, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};

use super::Graphics;
use crate::{
    components::{Camera, Global},
    math::Transform,
};
use glam::Mat4;

const HDR_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

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

#[derive(Debug)]
pub struct RenderStack {
    pub physical_size: [u32; 2],

    // Camera data
    camera: hecs::Entity,
    camera_buffer: wgpu::Buffer,

    // Global Params Data
    global_buffer: wgpu::Buffer,

    frame_bind_group_layout: wgpu::BindGroupLayout,
    frame_bind_group: wgpu::BindGroup,

    // HDR color texture (primary render target)
    hdr_color: wgpu::Texture,
    hdr_color_view: TextureView,
    hdr_sampler: wgpu::Sampler,

    // Sierpinksi Pipeline
    sierpinski: wgpu::RenderPipeline,

    // Composite Pipeline
    composite: wgpu::RenderPipeline,
    composite_hdr_bind_group_layout: BindGroupLayout,
    composite_hdr_bind_group: wgpu::BindGroup,

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

        let composite_hdr_bind_group_layout =
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

        let composite_hdr_bind_group = create_hdr_bind_group(
            gfx,
            &composite_hdr_bind_group_layout,
            &hdr_color_view,
            &hdr_sampler,
        );

        // *******************************
        // Composite Pipeline

        let shader = gfx.create_shader_module("composite", include_wesl!("composite"));

        let layout = gfx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&composite_hdr_bind_group_layout, &frame_bind_group_layout],
                immediate_size: 0,
            });

        let composite = create_post_processing_pipeline(
            gfx,
            "composite_pipeline",
            layout,
            shader,
            gfx.surface_format,
            Default::default(),
        );

        // *****************************
        // Sierpinski Pipeline

        let fractal_shader = gfx.create_shader_module("fractal", include_wesl!("fractal"));

        let layout = gfx
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&frame_bind_group_layout],
                immediate_size: 0,
            });

        let sierpinski = create_post_processing_pipeline(
            gfx,
            "sierpinski",
            layout,
            fractal_shader,
            HDR_COLOR_FORMAT,
            Default::default(),
        );

        // ******************************
        // Staging Belt

        let staging_belt = wgpu::util::StagingBelt::new(gfx.device.clone(), 1024);

        Self {
            physical_size,

            camera,
            camera_buffer,

            global_buffer,

            frame_bind_group_layout,
            frame_bind_group,

            hdr_color,
            hdr_color_view,
            hdr_sampler,

            composite,
            composite_hdr_bind_group_layout,
            composite_hdr_bind_group,

            sierpinski,

            staging_belt,
        }
    }

    pub fn resize(&mut self, gfx: &Graphics, physical_size: [u32; 2]) {
        self.physical_size = physical_size;

        log::info!(
            "Resizing render stack viewport to ({}, {})",
            physical_size[0],
            physical_size[1]
        );

        self.hdr_color = create_hdr_color(gfx, physical_size[0], physical_size[1]);
        self.hdr_color_view = create_hdr_color_view(&self.hdr_color);
        self.composite_hdr_bind_group = create_hdr_bind_group(
            gfx,
            &self.composite_hdr_bind_group_layout,
            &self.hdr_color_view,
            &self.hdr_sampler,
        );
    }

    pub fn prepare(
        &mut self,
        _gfx: &Graphics,
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
            let global = world
                .query_mut::<&Global>()
                .into_iter()
                .next()
                .unwrap_or(&Global::DEFAULT);

            let time = global.time.as_secs_f32();

            let mut uniform = self.staging_belt.write_buffer(
                encoder,
                &self.global_buffer,
                0,
                (size_of::<GlobalUniform>() as u64).try_into().unwrap(),
            );
            uniform.copy_from_slice(bytemuck::cast_slice(&[GlobalUniform {
                time,
                pre_saturation: global.pre_saturation,
                post_saturation: global.post_saturation,
                gamma: global.gamma,
                exposure: global.exposure,
            }]));
        }

        self.staging_belt.finish();
    }

    pub fn render(
        &mut self,
        _gfx: &Graphics,
        _world: &mut hecs::World,
        encoder: &mut wgpu::CommandEncoder,
    ) {
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
        render_pass.set_pipeline(&self.sierpinski);
        render_pass.set_bind_group(0, &self.frame_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        drop(render_pass);
    }

    pub fn recall(&mut self, _gfx: &Graphics, _world: &mut hecs::World) {
        self.staging_belt.recall();
    }

    pub fn draw_composite(&self, render_pass: &mut RenderPass<'static>) {
        render_pass.set_pipeline(&self.composite);
        render_pass.set_bind_group(0, &self.composite_hdr_bind_group, &[]);
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
        format: HDR_COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture
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

fn create_post_processing_pipeline(
    gfx: &Graphics,
    name: &str,
    layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    options: PipelineCompilationOptions,
) -> wgpu::RenderPipeline {
    gfx.device
        .create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(name),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: options,
                targets: &[Some(ColorTargetState {
                    format: format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
}
