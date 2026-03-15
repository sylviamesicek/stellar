use crate::renderer::Graphics;
use crate::renderer::stack::standard::StandardPipeline;
use crate::{
    components::{Camera, Global, Pipeline},
    math::Transform,
};
use wesl::include_wesl;
use wgpu::{BufferDescriptor, BufferUsages, RenderPass, ShaderStages};

mod bloom;
mod composite;
mod hdr;
mod standard;

use bloom::BloomPipeline;
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

#[derive(Debug, Clone)]
pub struct FrameData {
    camera_buffer: wgpu::Buffer,
    global_buffer: wgpu::Buffer,

    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl FrameData {
    pub fn new(gfx: &Graphics) -> Self {
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

        let bind_group_layout = gfx
            .start_bind_group_layout()
            .label("frame_bind_group_layout")
            .uniform_buffer_binding(0, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
            .uniform_buffer_binding(1, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
            .finish();

        let bind_group = gfx
            .start_bind_group(&bind_group_layout)
            .buffer_binding(0, &camera_buffer, 0, None)
            .buffer_binding(1, &global_buffer, 0, None)
            .finish();

        Self {
            camera_buffer,
            global_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn prepare(
        &mut self,
        _gfx: &Graphics,
        world: &mut hecs::World,
        camera: hecs::Entity,
        encoder: &mut wgpu::CommandEncoder,
        staging_belt: &mut wgpu::util::StagingBelt,
    ) {
        // Camera
        {
            let camera_handle = camera;
            let camera = world.get::<&Camera>(camera_handle).unwrap();
            let transform = world.get::<&Transform>(camera_handle).unwrap();

            let proj = camera.projection.get_clip_from_view();
            let inv_proj = proj.inverse();
            let view = transform.to_matrix().inverse();
            let inv_view = transform.to_matrix();

            let mut uniform = staging_belt.write_buffer(
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

            let mut uniform = staging_belt.write_buffer(
                encoder,
                &self.global_buffer,
                0,
                (size_of::<GlobalUniform>() as u64).try_into().unwrap(),
            );
            uniform.copy_from_slice(bytemuck::cast_slice(&[GlobalUniform { time }]));
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
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
    // HDR color texture (primary render target)
    hdr: HdrTextures,
    // Frame Buffers
    frame_data: FrameData,

    // Composite data
    composite_buffer: wgpu::Buffer,
    composite_bind_group: wgpu::BindGroup,

    // Standard Pipeline
    standard_pipeline: StandardPipeline,
    // Fractal Pipelines
    fractal: [wgpu::RenderPipeline; 2],

    // Bloom Manager
    bloom_pipeline: BloomPipeline,

    // Composite Pipeline
    composite: wgpu::RenderPipeline,

    staging_belt: wgpu::util::StagingBelt,
}

impl RenderStack {
    pub fn new(gfx: &Graphics, camera: hecs::Entity, physical_size: [u32; 2]) -> Self {
        // Hdr Textures
        let hdr = HdrTextures::new(gfx, physical_size);
        // Frame Data
        let frame_data = FrameData::new(gfx);

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

        // Standard Pipeline
        let standard_pipeline = StandardPipeline::new(gfx, &frame_data);
        // Bloom Pipeline
        let bloom_pipeline = BloomPipeline::new(gfx, physical_size);

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
        // Fractal Pipelines

        let fractal_shader = gfx.create_shader_module("fractal", include_wesl!("fractal"));
        let fractal_layout = gfx.create_pipeline_layout(0, &[frame_data.bind_group_layout()]);

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

            hdr,

            camera,

            frame_data,

            composite_buffer,
            composite_bind_group,

            standard_pipeline,
            bloom_pipeline,

            composite,

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
        self.frame_data
            .prepare(gfx, world, self.camera, encoder, &mut self.staging_belt);

        // Standard
        self.standard_pipeline
            .prepare(gfx, world, encoder, &mut self.staging_belt);

        // Bloom
        self.bloom_pipeline.prepare(
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

        match global.pipeline {
            Pipeline::Mandlebulb | Pipeline::Sierpinski => {
                let fractal_index = match global.pipeline {
                    Pipeline::Mandlebulb => 0,
                    Pipeline::Sierpinski => 1,
                    _ => 0,
                };

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

                render_pass.set_pipeline(&self.fractal[fractal_index]);
                render_pass.set_bind_group(0, self.frame_data.bind_group(), &[]);
                render_pass.draw(0..3, 0..1);
            }
            Pipeline::Standard => {
                self.standard_pipeline
                    .render(gfx, world, &self.hdr, &self.frame_data, encoder);
            }
        }

        self.bloom_pipeline.render(gfx, world, &self.hdr, encoder);
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
