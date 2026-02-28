use std::sync::Arc;
use std::time::{Duration, Instant};

use wgpu::InstanceDescriptor;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Theme, Window, WindowId};

use crate::renderer::{Graphics, Renderer, ScreenDescriptor};

#[derive(Default)]
pub enum App {
    #[default]
    Init,
    State {
        window: Arc<Window>,
        gfx: Graphics,
        renderer: Renderer,

        ui_state: egui_winit::State,

        last_size: (u32, u32),
        last_render_time: Instant,
    },
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let attributes = Window::default_attributes().with_title("Stellar");

        let Ok(new_window) = event_loop.create_window(attributes) else {
            return;
        };

        if let App::State { window, .. } = self {
            *window = Arc::new(new_window);
            return;
        }

        let window_handle = Arc::new(new_window);
        let window = window_handle.clone();

        let (width, height) = (
            window_handle.inner_size().width,
            window_handle.inner_size().height,
        );
        // Initialize graphics
        let gfx =
            pollster::block_on(
                async move { Graphics::new(window_handle.clone(), width, height).await },
            );
        let renderer = Renderer::new(&gfx);

        let ui_context = egui::Context::default();
        let viewport_id = ui_context.viewport_id();
        let ui_state = egui_winit::State::new(
            ui_context,
            viewport_id,
            &window,
            Some(window.scale_factor() as _),
            Some(Theme::Dark),
            None,
        );

        // Save state of app
        *self = Self::State {
            window,
            gfx,
            renderer,
            ui_state,
            last_size: (width, height),
            last_render_time: Instant::now(),
        };
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Self::State {
            window,
            gfx,
            renderer,
            last_size,
            last_render_time,
            ui_state,
        } = self
        else {
            return;
        };

        if ui_state.on_window_event(window, &event).consumed {
            return;
        }

        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let scale_factor = window.scale_factor() as f32;
                ui_state.egui_ctx().set_pixels_per_point(scale_factor);
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if width == 0 || height == 0 {
                    return;
                }

                log::info!("Resizing renderer surface to ({width}, {height})");
                gfx.resize(width, height);
                *last_size = (width, height);

                let scale_factor = window.scale_factor() as f32;
                ui_state.egui_ctx().set_pixels_per_point(scale_factor);
            }
            WindowEvent::CloseRequested => {
                log::info!("Close requested. Exiting...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Track delta time
                let now = Instant::now();
                let delta_time = now - *last_render_time;
                *last_render_time = now;

                // Handle Ui Events
                let ui_input = ui_state.take_egui_input(window);
                // Build Ui
                let ctx = ui_state.egui_ctx();

                ctx.begin_pass(ui_input);

                egui::Window::new("Test").show(ctx, |ui| ui.label("Hello World"));

                // End Building UI
                let ui_output = ctx.end_pass();
                ui_state.handle_platform_output(window, ui_output.platform_output);
                let pixels_per_point = ui_output.pixels_per_point;
                // Generate paint job
                let paint_jobs = ui_state
                    .egui_ctx()
                    .tessellate(ui_output.shapes, ui_output.pixels_per_point);

                // Perform rendering
                let (width, height) = *last_size;
                if width == 0 || height == 0 {
                    // Short circuit if surface is minimized
                    return;
                }

                let surface_texture = match gfx.surface.get_current_texture() {
                    Ok(texture) => texture,
                    Err(wgpu::SurfaceError::Outdated) => {
                        gfx.surface.configure(&gfx.device, &gfx.surface_config);
                        gfx.surface
                            .get_current_texture()
                            .expect("Failed to get surface texture after reconfiguration!")
                    }
                    Err(error) => panic!("Failed to get surface texture {:?}", error),
                };
                let surface_view =
                    surface_texture
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor {
                            label: wgpu::Label::default(),
                            aspect: wgpu::TextureAspect::default(),
                            format: Some(gfx.surface_format),
                            dimension: None,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: 0,
                            array_layer_count: None,
                            usage: None,
                        });

                // Build command encoder
                let mut encoder = gfx
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                // Perform rendering
                renderer.prepare_ui(
                    gfx,
                    ScreenDescriptor {
                        size_in_pixels: [width, height],
                        pixels_per_point,
                    },
                    &ui_output.textures_delta,
                    &paint_jobs,
                    &mut encoder,
                );
                renderer.render_frame(
                    gfx,
                    &surface_view,
                    &paint_jobs,
                    &ScreenDescriptor {
                        size_in_pixels: [width, height],
                        pixels_per_point,
                    },
                    &mut encoder,
                );
                // Submit command encoder
                gfx.queue.submit(std::iter::once(encoder.finish()));
                surface_texture.present();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let Self::State { window, .. } = self else {
            return;
        };
        window.request_redraw();
    }
}
