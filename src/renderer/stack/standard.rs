use image::imageops::sample_bilinear;

use crate::{
    components::Star,
    renderer::{
        Graphics,
        stack::{FrameData, hdr::HdrTextures},
    },
};

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct StarUniform {
    origin: glam::Vec3,
    radius: f32,
    color: glam::Vec3,
    sunspot_threshold: f32,
    sunspot_frequency: f32,
    granule_frequency: f32,
    granule_persistence: f32,
    time_scale: f32,
}

#[derive(Debug)]
pub struct StandardPipeline {
    star_buffer: wgpu::Buffer,

    _star_bind_group_layout: wgpu::BindGroupLayout,
    star_bind_group: wgpu::BindGroup,

    star_pipeline: wgpu::RenderPipeline,

    star_black_body_lookup: image::RgbImage,
}

impl StandardPipeline {
    pub fn new(gfx: &Graphics, frame: &FrameData) -> Self {
        let star_buffer = gfx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("star_buffer"),
            size: size_of::<StarUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let star_bind_group_layout = gfx
            .start_bind_group_layout()
            .label("star_bind_group_layout")
            .uniform_buffer_binding(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT)
            .finish();

        let star_bind_group = gfx
            .start_bind_group(&star_bind_group_layout)
            .label("star_bind_group")
            .buffer_binding(0, &star_buffer, 0, None)
            .finish();

        let star_shader = gfx.create_shader_module("star", wesl::include_wesl!("star"));
        let star_layout =
            gfx.create_pipeline_layout(0, &[frame.bind_group_layout(), &star_bind_group_layout]);

        let star_pipeline = gfx
            .start_post_processing_pipeline(&star_shader)
            .label("star")
            .color_format(gfx.hdr_format)
            .layout(&star_layout)
            .finish();

        let star_black_body_bytes = include_bytes!("../data/star_black_body.png");
        let star_black_body_lookup = image::load_from_memory(star_black_body_bytes)
            .unwrap()
            .to_rgb8();

        Self {
            star_buffer,
            _star_bind_group_layout: star_bind_group_layout,
            star_bind_group,
            star_pipeline,
            star_black_body_lookup,
        }
    }

    pub fn prepare(
        &self,
        _gfx: &Graphics,
        world: &mut hecs::World,
        encoder: &mut wgpu::CommandEncoder,
        staging_belt: &mut wgpu::util::StagingBelt,
    ) {
        let star_default = Star::sun();
        let star = world
            .query_mut::<&Star>()
            .into_iter()
            .next()
            .unwrap_or(&star_default);

        let temp = star.temperature;
        let u = (temp - 800.0) / 29200.0;
        let color = sample_bilinear(&self.star_black_body_lookup, u, 0.0).unwrap();
        let mut color = glam::U8Vec3::from_array(color.0).as_vec3() / 255.0;

        if star.color_shift {
            color += get_temp_color_shift(temp);
        }

        let mut uniform = staging_belt.write_buffer(
            encoder,
            &self.star_buffer,
            0,
            (size_of::<StarUniform>() as u64).try_into().unwrap(),
        );
        uniform.copy_from_slice(bytemuck::cast_slice(&[StarUniform {
            origin: glam::Vec3::ZERO,
            radius: 1.0,
            color,
            granule_frequency: star.granule_frequency,
            granule_persistence: star.granule_persistence,
            sunspot_threshold: star.sunspot_threshold,
            sunspot_frequency: star.sunspot_frequency,
            time_scale: 200.0,
        }]));
    }

    pub fn render(
        &mut self,
        _gfx: &Graphics,
        _world: &mut hecs::World,
        hdr: &HdrTextures,
        frame: &FrameData,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: hdr.color_view(),
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
        render_pass.set_pipeline(&self.star_pipeline);
        render_pass.set_bind_group(0, frame.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.star_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

fn get_temp_color_shift(temp: f32) -> glam::Vec3 {
    glam::vec3(
        temp * (0.0534 / 255.0) - (43.0 / 255.0),
        temp * (0.0628 / 255.0) - (77.0 / 255.0),
        temp * (0.0735 / 255.0) - (115.0 / 255.0),
    )
}
