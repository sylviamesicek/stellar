use image::{GenericImageView, imageops::sample_bilinear};

use crate::{
    components::Star,
    math::Transform,
    renderer::{
        Graphics,
        stack::{FrameData, hdr::HdrTextures},
    },
};

pub const STAR_MATERIAL_ID: u32 = 0;
pub const SKYBOX_MATERIAL_ID: u32 = 255;

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct StarDesc {
    origin: glam::Vec3,
    radius: f32,
    color: glam::Vec3,
    sunspot_threshold: f32,
    sunspot_frequency: f32,
    granule_frequency: f32,
    granule_persistence: f32,
    time_scale: f32,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct SphereDesc {
    origin: glam::Vec3,
    radius: f32,
    material_id: u32,
    instance_id: u32,
    _padding: glam::UVec2,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct NaiveImmediates {
    num_stars: u32,
}

#[derive(Debug, Clone)]
struct RBuffer {
    instance: wgpu::Texture,
    instance_view: wgpu::TextureView,

    positions: wgpu::Texture,
    positions_view: wgpu::TextureView,

    directions: wgpu::Texture,
    directions_view: wgpu::TextureView,

    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl RBuffer {
    pub fn new(gfx: &Graphics, physical_size: [u32; 2]) -> Self {
        let instance = create_rbuffer_texture(
            gfx,
            "instance_buffer",
            physical_size,
            wgpu::TextureFormat::R32Uint,
        );
        let instance_view = instance.create_view(&wgpu::TextureViewDescriptor::default());

        let positions = create_rbuffer_texture(
            gfx,
            "positions",
            physical_size,
            wgpu::TextureFormat::Rgba32Float,
        );
        let positions_view = positions.create_view(&wgpu::TextureViewDescriptor::default());

        let directions = create_rbuffer_texture(
            gfx,
            "directions",
            physical_size,
            wgpu::TextureFormat::Rgba32Float,
        );
        let directions_view = directions.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_layout = gfx
            .start_bind_group_layout()
            .label("rbuffer_bind_group_layour")
            .texture_binding(
                0,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Uint,
                wgpu::TextureViewDimension::D2,
                false,
            )
            .texture_binding(
                1,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
                false,
            )
            .texture_binding(
                2,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
                false,
            )
            .finish();

        let bind_group = gfx
            .start_bind_group(&bind_group_layout)
            .texture_view_binding(0, &instance_view)
            .texture_view_binding(1, &positions_view)
            .texture_view_binding(2, &directions_view)
            .finish();

        Self {
            instance,
            instance_view,
            positions,
            positions_view,
            directions,
            directions_view,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn resize(&mut self, gfx: &Graphics, physical_size: [u32; 2]) {
        self.instance = create_rbuffer_texture(
            gfx,
            "instances",
            physical_size,
            wgpu::TextureFormat::R32Uint,
        );
        self.instance_view = self
            .instance
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.positions = create_rbuffer_texture(
            gfx,
            "positions",
            physical_size,
            wgpu::TextureFormat::Rgba32Float,
        );
        self.positions_view = self
            .positions
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.directions = create_rbuffer_texture(
            gfx,
            "directions",
            physical_size,
            wgpu::TextureFormat::Rgba32Float,
        );
        self.directions_view = self
            .directions
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.bind_group = gfx
            .start_bind_group(&self.bind_group_layout)
            .texture_view_binding(0, &self.instance_view)
            .texture_view_binding(1, &self.positions_view)
            .texture_view_binding(2, &self.directions_view)
            .finish();
    }
}

fn create_rbuffer_texture(
    gfx: &Graphics,
    name: &str,
    physical_size: [u32; 2],
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    let texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(name),
        size: wgpu::Extent3d {
            width: physical_size[0],
            height: physical_size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture
}

#[derive(Debug)]
pub struct StandardPipeline {
    rbuffer: RBuffer,

    spheres_host: Vec<SphereDesc>,
    spheres_buffer: wgpu::Buffer,
    spheres_bind_group_layout: wgpu::BindGroupLayout,
    spheres_bind_group: wgpu::BindGroup,
    // Naive Raymarching Pipeline
    naive_pipeline: wgpu::RenderPipeline,

    // Star material
    star_black_body_lookup: image::RgbImage,

    stars_host: Vec<StarDesc>,
    stars_buffer: wgpu::Buffer,
    stars_bind_group_layout: wgpu::BindGroupLayout,
    stars_bind_group: wgpu::BindGroup,

    star_pipeline: wgpu::RenderPipeline,

    // Skybox material
    _milkyway: wgpu::Texture,
    _milkyway_view: wgpu::TextureView,
    _milkyway_sampler: wgpu::Sampler,
    milkyway_bind_group: wgpu::BindGroup,

    skybox_pipeline: wgpu::RenderPipeline,
}

impl StandardPipeline {
    pub fn new(gfx: &Graphics, frame: &FrameData, physical_size: [u32; 2]) -> Self {
        let rbuffer: RBuffer = RBuffer::new(gfx, physical_size);

        let spheres_buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spheres_buffer"),
            size: 10 * size_of::<SphereDesc>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let spheres_bind_group_layout = gfx
            .start_bind_group_layout()
            .label("spheres_bind_group_layout")
            .storage_buffer_binding(0, wgpu::ShaderStages::FRAGMENT, true)
            .finish();

        let spheres_bind_group = gfx
            .start_bind_group(&spheres_bind_group_layout)
            .label("spheres_bind_group")
            .buffer_binding(0, &spheres_buffer, 0, None)
            .finish();

        let naive_shader =
            gfx.create_shader_module("naive", include_str!("../shaders/raymarching/naive.wgsl"));
        let naive_layout = gfx.create_pipeline_layout(
            size_of::<NaiveImmediates>() as u32,
            &[frame.bind_group_layout(), &spheres_bind_group_layout],
        );

        let naive_pipeline = gfx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("naive_pipeline"),
                layout: Some(&naive_layout),
                vertex: gfx.fullscreen_vertex_state(),
                primitive: gfx.fullscreen_primitive_state(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &naive_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::R32Uint,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba32Float,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                }),
                multiview_mask: None,
                cache: None,
            });

        let stars_buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stars_buffer"),
            size: 10 * size_of::<StarDesc>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let stars_bind_group_layout = gfx
            .start_bind_group_layout()
            .label("stars_bind_group_layout")
            .storage_buffer_binding(0, wgpu::ShaderStages::FRAGMENT, true)
            .finish();

        let stars_bind_group = gfx
            .start_bind_group(&spheres_bind_group_layout)
            .label("stars_bind_group")
            .buffer_binding(0, &stars_buffer, 0, None)
            .finish();

        let star_shader =
            gfx.create_shader_module("star", include_str!("../shaders/materials/star.wgsl"));
        let star_layout = gfx.create_pipeline_layout(
            0,
            &[
                frame.bind_group_layout(),
                &rbuffer.bind_group_layout,
                &stars_bind_group_layout,
            ],
        );

        let star_pipeline = gfx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("star_pipeline"),
                layout: Some(&star_layout),
                vertex: gfx.fullscreen_vertex_state_with_constants(&[(
                    "depth",
                    STAR_MATERIAL_ID as f64 / 256.0,
                )]),
                primitive: gfx.fullscreen_primitive_state(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Equal,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &star_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gfx.hdr_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        let star_black_body_bytes = include_bytes!("../data/star_black_body.png");
        let star_black_body_lookup = image::load_from_memory(star_black_body_bytes)
            .unwrap()
            .to_rgb8();

        // Skybox

        log::info!("Loading Milkway Texture...");

        let milkyway_image = image::ImageReader::open("data/milkyway.jpg")
            .expect("Failed to load milkyway panorama")
            .decode()
            .unwrap();

        let milkyway_rgba = milkyway_image.to_rgba8();
        let milkyway_dims = milkyway_image.dimensions();

        let milkyway = gfx.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: milkyway_dims.0,
                height: milkyway_dims.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1, // We'll talk about this a little later
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Most images are stored using sRGB, so we need to reflect that here.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
            // COPY_DST means that we want to copy data to this texture
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("milkyway_texture"),
            // This is the same as with the SurfaceConfig. It
            // specifies what texture formats can be used to
            // create TextureViews for this texture. The base
            // texture format (Rgba8UnormSrgb in this case) is
            // always supported. Note that using a different
            // texture format is not supported on the WebGL2
            // backend.
            view_formats: &[],
        });

        gfx.queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::TexelCopyTextureInfo {
                texture: &milkyway,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            &milkyway_rgba,
            // The layout of the texture
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * milkyway_dims.0),
                rows_per_image: Some(milkyway_dims.1),
            },
            wgpu::Extent3d {
                width: milkyway_dims.0,
                height: milkyway_dims.1,
                depth_or_array_layers: 1,
            },
        );

        let milkyway_view = milkyway.create_view(&Default::default());

        let milkyway_sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("milkyway_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let skybox_bind_group_layout = gfx
            .start_bind_group_layout()
            .label("skybox_bind_group_layout")
            .texture_binding(
                0,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2,
                false,
            )
            .sampler_binding(
                1,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::SamplerBindingType::Filtering,
            )
            .finish();

        let milkyway_bind_group = gfx
            .start_bind_group(&skybox_bind_group_layout)
            .label("skybox_bind_group")
            .texture_view_binding(0, &milkyway_view)
            .sampler_binding(1, &milkyway_sampler)
            .finish();

        let skybox_shader =
            gfx.create_shader_module("skybox", include_str!("../shaders/materials/skybox.wgsl"));
        let skybox_layout = gfx.create_pipeline_layout(
            0,
            &[
                frame.bind_group_layout(),
                &rbuffer.bind_group_layout,
                &skybox_bind_group_layout,
            ],
        );
        let skybox_pipeline = gfx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("skybox_pipeline"),
                layout: Some(&skybox_layout),
                vertex: gfx.fullscreen_vertex_state_with_constants(&[(
                    "depth",
                    SKYBOX_MATERIAL_ID as f64 / 256.0,
                )]),
                primitive: gfx.fullscreen_primitive_state(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Equal,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &skybox_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gfx.hdr_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        Self {
            rbuffer,
            spheres_host: Vec::new(),
            spheres_buffer,
            spheres_bind_group_layout,
            spheres_bind_group,
            naive_pipeline,

            star_black_body_lookup,
            stars_host: Vec::new(),
            stars_buffer,
            stars_bind_group_layout,
            stars_bind_group,
            star_pipeline,

            _milkyway: milkyway,
            _milkyway_view: milkyway_view,
            _milkyway_sampler: milkyway_sampler,
            milkyway_bind_group,
            skybox_pipeline,
        }
    }

    pub fn resize(&mut self, gfx: &Graphics, physical_size: [u32; 2]) {
        self.rbuffer.resize(gfx, physical_size);
    }

    pub fn prepare(
        &mut self,
        gfx: &Graphics,
        world: &mut hecs::World,
        encoder: &mut wgpu::CommandEncoder,
        staging_belt: &mut wgpu::util::StagingBelt,
    ) {
        let num_stars = world.query_mut::<(&Transform, &Star)>().into_iter().count();

        self.spheres_host.clear();
        self.spheres_host.reserve(num_stars);

        self.stars_host.clear();
        self.stars_host.reserve(num_stars);

        for (transform, star) in world.query_mut::<(&Transform, &Star)>().into_iter() {
            let instance_id = self.stars_host.len() as u32;

            let temp = star.temperature;
            let u = (temp - 800.0) / 29200.0;
            let color = sample_bilinear(&self.star_black_body_lookup, u, 0.0).unwrap();
            let mut color = glam::U8Vec3::from_array(color.0).as_vec3() / 255.0;

            if star.color_shift {
                color += get_temp_color_shift(temp);
            }

            self.spheres_host.push(SphereDesc {
                origin: transform.translation,
                radius: transform.scale.max_element(),
                material_id: STAR_MATERIAL_ID,
                instance_id: instance_id,
                _padding: glam::UVec2::ZERO,
            });
            self.stars_host.push(StarDesc {
                origin: transform.translation,
                radius: transform.scale.max_element(),
                color,
                granule_frequency: star.granule_frequency,
                granule_persistence: star.granule_persistence,
                sunspot_threshold: star.sunspot_threshold,
                sunspot_frequency: star.sunspot_frequency,
                time_scale: 200.0,
            });
        }

        if self.spheres_buffer.size() / (size_of::<SphereDesc>() as u64)
            < self.spheres_host.len() as u64
        {
            self.spheres_buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("spheres_buffer"),
                size: self.spheres_host.len() as u64 * size_of::<SphereDesc>() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.spheres_bind_group = gfx
                .start_bind_group(&self.spheres_bind_group_layout)
                .label("spheres_bind_group")
                .buffer_binding(0, &self.spheres_buffer, 0, None)
                .finish();
        }

        if self.stars_buffer.size() / (size_of::<StarDesc>() as u64) < self.stars_host.len() as u64
        {
            self.stars_buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("stars_buffer"),
                size: self.stars_host.len() as u64 * size_of::<StarDesc>() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.stars_bind_group = gfx
                .start_bind_group(&self.stars_bind_group_layout)
                .label("star_bind_group")
                .buffer_binding(0, &self.stars_buffer, 0, None)
                .finish();
        }

        // let temp = star.temperature;
        // let u = (temp - 800.0) / 29200.0;
        // let color = sample_bilinear(&self.star_black_body_lookup, u, 0.0).unwrap();
        // let mut color = glam::U8Vec3::from_array(color.0).as_vec3() / 255.0;

        // if star.color_shift {
        //     color += get_temp_color_shift(temp);
        // }

        let mut uniform = staging_belt.write_buffer(
            encoder,
            &self.spheres_buffer,
            0,
            ((size_of::<SphereDesc>() * self.spheres_host.len()) as u64)
                .try_into()
                .unwrap(),
        );
        uniform.copy_from_slice(bytemuck::cast_slice(&self.spheres_host));

        let mut uniform = staging_belt.write_buffer(
            encoder,
            &self.stars_buffer,
            0,
            ((size_of::<StarDesc>() * self.stars_host.len()) as u64)
                .try_into()
                .unwrap(),
        );
        uniform.copy_from_slice(bytemuck::cast_slice(&self.stars_host));
    }

    pub fn render(
        &mut self,
        _gfx: &Graphics,
        _world: &mut hecs::World,
        hdr: &HdrTextures,
        frame: &FrameData,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // Raymarching pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.rbuffer.instance_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.rbuffer.positions_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.rbuffer.directions_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: hdr.depth_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::DontCare(unsafe { wgpu::LoadOpDontCare::enabled() }),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        render_pass.set_pipeline(&self.naive_pipeline);
        render_pass.set_immediates(
            0,
            bytemuck::cast_slice(&[NaiveImmediates {
                num_stars: self.spheres_host.len() as u32,
            }]),
        );
        render_pass.set_bind_group(0, frame.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.spheres_bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        drop(render_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: hdr.color_view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: hdr.depth_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        render_pass.set_pipeline(&self.skybox_pipeline);
        render_pass.set_bind_group(0, frame.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.rbuffer.bind_group, &[]);
        render_pass.set_bind_group(2, &self.milkyway_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        drop(render_pass);

        // Star pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: hdr.color_view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: hdr.depth_view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        render_pass.set_pipeline(&self.star_pipeline);
        render_pass.set_bind_group(0, frame.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.rbuffer.bind_group, &[]);
        render_pass.set_bind_group(2, &self.stars_bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        drop(render_pass);
    }
}

fn get_temp_color_shift(temp: f32) -> glam::Vec3 {
    glam::vec3(
        temp * (0.0534 / 255.0) - (43.0 / 255.0),
        temp * (0.0628 / 255.0) - (77.0 / 255.0),
        temp * (0.0735 / 255.0) - (115.0 / 255.0),
    )
}
