use crate::renderer::Graphics;
use crate::{
    components::{Camera, Global, Pipeline},
    math::Transform,
};
use image::GenericImageView as _;
use wesl::include_wesl;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBinding, BufferBindingType, BufferDescriptor, BufferUsages,
    RenderPass, ShaderStages,
};

mod bloom;
mod composite;
mod hdr;

use bloom::BloomManager;
use hdr::HdrTextures;

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CameraUniform {
    proj: glam::Mat4,
    view: glam::Mat4,

    inv_proj: glam::Mat4,
    inv_view: glam::Mat4,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct GlobalUniform {
    time: f32,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct StarUniform {
    origin: glam::Vec3,
    radius: f32,
    temp: f32,
    _unused: glam::Vec3,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CompositeUniform {
    pre_saturation: f32,
    post_saturation: f32,
    gamma: f32,
    exposure: f32,
}

#[derive(Debug)]
pub struct RenderStack {
    pub physical_size: [u32; 2],

    // Camera corresponding to this render stack.
    camera: hecs::Entity,

    // Frame Buffers
    camera_buffer: wgpu::Buffer,
    global_buffer: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,

    // Composite data
    composite_buffer: wgpu::Buffer,
    composite_bind_group: wgpu::BindGroup,

    // Star Upload data (temp)
    star_buffer: wgpu::Buffer,
    star_black_body: wgpu::Texture,
    star_bind_group: wgpu::BindGroup,

    // HDR color texture (primary render target)
    hdr: HdrTextures,

    // Star Pipeline
    star: wgpu::RenderPipeline,
    // Fractal Pipelines
    fractal: [wgpu::RenderPipeline; 2],

    // Bloom Manager
    bloom_manager: BloomManager,

    // Composite Pipeline
    composite: wgpu::RenderPipeline,

    staging_belt: wgpu::util::StagingBelt,
}

impl RenderStack {
    pub fn new(gfx: &Graphics, camera: hecs::Entity, physical_size: [u32; 2]) -> Self {
        // Hdr Textures
        let hdr = HdrTextures::new(gfx, physical_size);

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

        let frame_bind_group_layout = gfx
            .start_bind_group_layout()
            .label("frame_bind_group_layout")
            .uniform_buffer_binding(0, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
            .uniform_buffer_binding(1, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
            .finish();

        let frame_bind_group = gfx
            .start_bind_group(&frame_bind_group_layout)
            .buffer_binding(0, &camera_buffer, 0, None)
            .buffer_binding(1, &global_buffer, 0, None)
            .finish();

        let composite_buffer = gfx.device.create_buffer(&BufferDescriptor {
            label: Some("composite_buffer"),
            size: size_of::<CompositeUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let composite_bind_group_layout = gfx
            .start_bind_group_layout()
            .label("composite_bind_group_layout")
            .uniform_buffer_binding(0, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
            .finish();

        let composite_bind_group = gfx
            .start_bind_group(&composite_bind_group_layout)
            .buffer_binding(0, &composite_buffer, 0, None)
            .finish();

        let star_buffer = gfx.device.create_buffer(&BufferDescriptor {
            label: Some("star_buffer"),
            size: size_of::<StarUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let star_black_body_bytes = include_bytes!("../data/star_black_body.png");
        let star_black_body_image = image::load_from_memory(star_black_body_bytes).unwrap();
        let star_black_body_rgba = star_black_body_image.to_rgba8();

        let dimensions = star_black_body_image.dimensions();

        assert!(dimensions.1 == 1);

        let star_black_body_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            // All textures are stored as 3D, we represent our 2D texture
            // by setting depth to 1.
            depth_or_array_layers: 1,
        };
        let star_black_body = gfx.device.create_texture(&wgpu::TextureDescriptor {
            size: star_black_body_size,
            mip_level_count: 1, // We'll talk about this a little later
            sample_count: 1,
            dimension: wgpu::TextureDimension::D1,
            // Most images are stored using sRGB, so we need to reflect that here.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("star_black_body_texture"),
            view_formats: &[],
        });
        gfx.queue.write_texture(
            // Tells wgpu where to copy the pixel data
            wgpu::TexelCopyTextureInfo {
                texture: &star_black_body,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            &star_black_body_rgba,
            // The layout of the texture
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            star_black_body_size,
        );
        let star_black_body_view = star_black_body.create_view(&wgpu::TextureViewDescriptor {
            ..Default::default()
        });

        let star_bind_group_layout = gfx
            .start_bind_group_layout()
            .label("star_bind_group_layout")
            .uniform_buffer_binding(0, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
            .texture_filterable_binding(
                1,
                ShaderStages::FRAGMENT,
                wgpu::TextureViewDimension::D1,
                false,
            )
            .sampler_binding(
                2,
                ShaderStages::FRAGMENT,
                wgpu::SamplerBindingType::Filtering,
            )
            .finish();

        let star_bind_group = gfx
            .start_bind_group(&star_bind_group_layout)
            .label("star_bind_group")
            .buffer_binding(0, &star_buffer, 0, None)
            .texture_view_binding(1, &star_black_body_view)
            .sampler_binding(2, &hdr.sampler)
            .finish();

        // Bloom Manager
        let bloom_manager = BloomManager::new(gfx, physical_size);

        // *******************************
        // Composite Pipeline

        let composite_shader = gfx.create_shader_module("composite", include_wesl!("composite"));
        let composite_layout = gfx
            .create_pipeline_layout(0, &[&hdr.bind_group_layout(), &composite_bind_group_layout]);
        let composite = gfx
            .start_post_processing_pipeline(&composite_shader)
            .label("composite")
            .color_format(gfx.surface_format)
            .layout(&composite_layout)
            .finish();

        // *****************************
        // Star Pipeline

        let star_shader = gfx.create_shader_module("star", include_wesl!("star"));
        let star_layout =
            gfx.create_pipeline_layout(0, &[&frame_bind_group_layout, &star_bind_group_layout]);

        let star = gfx
            .start_post_processing_pipeline(&star_shader)
            .label("sierpinski")
            .color_format(gfx.hdr_format)
            .layout(&star_layout)
            .finish();

        // *****************************
        // Fractal Pipelines

        let fractal_shader = gfx.create_shader_module("fractal", include_wesl!("fractal"));
        let fractal_layout = gfx.create_pipeline_layout(0, &[&frame_bind_group_layout]);

        let mandlebulb = gfx
            .start_post_processing_pipeline(&fractal_shader)
            .label("sierpinski")
            .color_format(gfx.hdr_format)
            .layout(&fractal_layout)
            .add_constant("fractal_type", 0.0)
            .finish();

        let sierpinski = gfx
            .start_post_processing_pipeline(&fractal_shader)
            .label("sierpinski")
            .color_format(gfx.hdr_format)
            .layout(&fractal_layout)
            .add_constant("fractal_type", 1.0)
            .finish();

        // ******************************
        // Staging Belt

        let staging_belt = wgpu::util::StagingBelt::new(gfx.device.clone(), 1024);

        Self {
            physical_size,

            camera,
            camera_buffer,
            global_buffer,
            frame_bind_group,

            composite_buffer,
            composite_bind_group,

            star_buffer,
            star_black_body,
            star_bind_group,

            bloom_manager,

            hdr,

            composite,

            star,
            fractal: [mandlebulb, sierpinski],

            staging_belt,
        }
    }

    pub fn resize(&mut self, gfx: &Graphics, physical_size: [u32; 2]) {
        self.hdr.resize(gfx, physical_size);
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
            uniform.copy_from_slice(bytemuck::cast_slice(&[GlobalUniform { time }]));
        }

        // Bloom
        self.bloom_manager.prepare(
            gfx,
            world,
            self.physical_size,
            encoder,
            &mut self.staging_belt,
        );

        // Composite
        {
            let global_default = Global::default();
            let global = world
                .query_mut::<&Global>()
                .into_iter()
                .next()
                .unwrap_or(&global_default);

            let mut uniform = self.staging_belt.write_buffer(
                encoder,
                &self.composite_buffer,
                0,
                (size_of::<CompositeUniform>() as u64).try_into().unwrap(),
            );
            uniform.copy_from_slice(bytemuck::cast_slice(&[CompositeUniform {
                pre_saturation: global.tonemap.pre_saturation,
                post_saturation: global.tonemap.post_saturation,
                gamma: global.tonemap.gamma,
                exposure: global.tonemap.exposure,
            }]));
        }

        // Star
        {
            let mut uniform = self.staging_belt.write_buffer(
                encoder,
                &self.star_buffer,
                0,
                (size_of::<StarUniform>() as u64).try_into().unwrap(),
            );
            uniform.copy_from_slice(bytemuck::cast_slice(&[StarUniform {
                origin: glam::Vec3::ZERO,
                radius: 1.0,
                temp: 2700.0,
                _unused: glam::Vec3::ZERO,
            }]));
        }

        self.staging_belt.finish();
    }

    pub fn render(
        &mut self,
        gfx: &Graphics,
        world: &mut hecs::World,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let global_default = Global::default();
        let global = world
            .query_mut::<&Global>()
            .into_iter()
            .next()
            .unwrap_or(&global_default);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.hdr.color_view(),
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
        match global.pipeline {
            Pipeline::Mandlebulb | Pipeline::Sierpinski => {
                let fractal_index = match global.pipeline {
                    Pipeline::Mandlebulb => 0,
                    Pipeline::Sierpinski => 1,
                    _ => 0,
                };

                render_pass.set_pipeline(&self.fractal[fractal_index]);
                render_pass.set_bind_group(0, &self.frame_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
            Pipeline::Standard => {
                render_pass.set_pipeline(&self.star);
                render_pass.set_bind_group(0, &self.frame_bind_group, &[]);
                render_pass.set_bind_group(1, &self.star_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
        }

        drop(render_pass);

        self.bloom_manager.render(gfx, world, &self.hdr, encoder);
    }

    pub fn recall(&mut self, _gfx: &Graphics, _world: &mut hecs::World) {
        self.staging_belt.recall();
    }

    pub fn draw_composite(&self, render_pass: &mut RenderPass<'static>) {
        render_pass.set_pipeline(&self.composite);
        render_pass.set_bind_group(0, self.hdr.color_bind_group(), &[]);
        render_pass.set_bind_group(1, &self.composite_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
