use std::collections::HashMap;

use crate::components::Camera;
use crate::math::Transform;
use hecs::Entity;
use smallvec::SmallVec;
use ui::UiRenderer;

mod graphics;
mod stack;
mod ui;

pub use graphics::Graphics;
use stack::RenderStack;
pub use ui::{UiCallback, UiScreen};

pub struct Renderer {
    ui: UiRenderer,
    // Render stacks associated with each camera
    stacks: HashMap<hecs::Entity, RenderStack>,

    // Temporary state
    paint_jobs: Vec<egui::ClippedPrimitive>,
    screen: UiScreen,
}

impl Renderer {
    pub fn new(gfx: &Graphics) -> Self {
        let ui = UiRenderer::new(&gfx.device, gfx.surface_format);

        Self {
            ui,
            stacks: HashMap::new(),
            paint_jobs: vec![],
            screen: UiScreen {
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
        screen: UiScreen,
        textures_delta: &egui::TexturesDelta,
        paint_jobs: &[egui::ClippedPrimitive],
        encoder: &mut wgpu::CommandEncoder,
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.ui.update_texture(gfx, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.ui.free_texture(id);
        }

        self.ui.update_buffers(&gfx, encoder, &paint_jobs, &screen);

        self.paint_jobs.clear();
        self.paint_jobs.extend_from_slice(paint_jobs);
        self.screen = screen;
    }

    pub fn prepare(
        &mut self,
        gfx: &Graphics,
        world: &mut hecs::World,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // Remove any stacks that no longer exist
        let mut remove_list = SmallVec::<[Entity; 4]>::new();
        for &e in self.stacks.keys() {
            if !world.contains(e) || !world.entity(e).unwrap().has::<Transform>() {
                remove_list.push(e);
            }
        }

        for e in remove_list {
            self.stacks.remove(&e);
        }

        // Update any existing stacks
        for (e, camera, _) in world.query_mut::<(Entity, &Camera, &Transform)>() {
            let stack = self
                .stacks
                .entry(e)
                .or_insert_with(|| RenderStack::new(gfx, e, camera.physical_size()));

            if camera.physical_size() != stack.physical_size {
                stack.resize(gfx, camera.physical_size());
            }
        }

        // Render stacks
        for stack in self.stacks.values_mut() {
            stack.prepare(gfx, world, encoder);
        }
    }

    pub fn render(
        &mut self,
        gfx: &Graphics,
        surface_view: &wgpu::TextureView,
        world: &mut hecs::World,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // **************************************
        // Camera Render Passes

        // Render stacks
        for stack in self.stacks.values_mut() {
            stack.render(gfx, world, encoder);
        }

        // ******************************************
        // Composite Renderpass

        // Final render pass (ui and all composited viewports)
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

        // Make sure there is not some mistake
        assert!(
            !self
                .ui
                .callback_resources
                .contains::<RendererCallbackResources>()
        );
        // Insert stacks into typemap
        let resources = RendererCallbackResources {
            stacks: std::mem::take(&mut self.stacks),
        };
        self.ui.callback_resources.insert(resources);
        // Draw composite UI
        self.ui
            .draw(&mut render_pass, &self.paint_jobs, self.screen);
        // Retrieve stacks from typemap
        let resources = self
            .ui
            .callback_resources
            .remove::<RendererCallbackResources>()
            .unwrap();
        self.stacks = resources.stacks;
        // End render pass
        drop(render_pass);
    }

    pub fn recall(&mut self, gfx: &Graphics, world: &mut hecs::World) {
        for stack in self.stacks.values_mut() {
            stack.recall(gfx, world);
        }
    }
}

struct RendererCallbackResources {
    stacks: HashMap<hecs::Entity, RenderStack>,
}

pub struct DrawCameraCallback {
    camera: hecs::Entity,
}

impl DrawCameraCallback {
    pub fn new(camera: hecs::Entity) -> Self {
        Self { camera }
    }
}

impl ui::UiCallbackTrait for DrawCameraCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &ui::UiCallbackResources,
    ) {
        let resources: &RendererCallbackResources = resources.get().unwrap();
        let stack = resources.stacks.get(&self.camera).unwrap();
        stack.draw_composite(render_pass);
    }
}
