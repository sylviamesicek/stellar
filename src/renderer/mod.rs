mod egui_renderer;
mod graphics;

use egui::{ClippedPrimitive, epaint};
use egui_renderer::EguiRenderer;

pub use egui_renderer::EguiScreen;
pub use graphics::Graphics;

pub struct Renderer {
    ui: EguiRenderer,
    // Temporary state
    paint_jobs: Vec<egui::ClippedPrimitive>,
    screen: EguiScreen,
}

impl Renderer {
    pub fn new(gfx: &Graphics) -> Self {
        let ui = EguiRenderer::new(&gfx.device, gfx.surface_format);

        Self {
            ui,
            paint_jobs: vec![],
            screen: EguiScreen {
                size_in_pixels: [0, 0],
                pixels_per_point: 0.0,
            },
        }
    }
}

impl Renderer {
    pub fn prepare_ui(
        &mut self,
        gfx: &Graphics,
        screen: EguiScreen,
        textures_delta: &egui::TexturesDelta,
        paint_jobs: &[egui::ClippedPrimitive],
        encoder: &mut wgpu::CommandEncoder,
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.ui
                .update_texture(&gfx.device, &gfx.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.ui.free_texture(id);
        }

        self.ui
            .update_buffers(&gfx.device, &gfx.queue, encoder, &paint_jobs, &screen);

        self.paint_jobs.clear();
        self.paint_jobs.extend_from_slice(paint_jobs);
        self.screen = screen;
    }

    pub fn render_frame(
        &mut self,
        _gfx: &Graphics,
        surface_view: &wgpu::TextureView,
        _world: &hecs::World,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // Begin render pass
        let mut render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.19,
                            g: 0.24,
                            b: 0.42,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            })
            .forget_lifetime();
        self.ui
            .draw(&mut render_pass, &self.paint_jobs, self.screen);
        drop(render_pass);

        // drop(render_pass);
    }
}
