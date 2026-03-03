use std::f32;
use std::time::Duration;

use egui::Color32;
use egui::epaint::ViewportInPixels;
use hecs::World;

use crate::camera::Camera;
use crate::math::{Transform, Vec3};
use crate::renderer::{DrawCameraCallback, UiCallback};

pub struct App {
    camera: hecs::Entity,
}

impl App {
    pub fn new() -> Self {
        Self {
            camera: hecs::Entity::DANGLING,
        }
    }

    pub fn ui_context(&self) -> egui::Context {
        let ctx = egui::Context::default();

        // toolkit::apply_style_and_install_loaders(&ctx);

        ctx
    }

    pub fn start(&mut self, world: &mut World) {
        self.camera = world.spawn(
            hecs::EntityBuilder::new()
                .add(Transform::from_xyz(-0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y))
                .add(Camera::perspective(f32::consts::PI / 2.0, 0.1, 1000.0))
                .build(),
        );
    }

    pub fn update(
        &mut self,
        world: &mut World,
        ctx: egui::Context,
        screen: [u32; 2],
        _delta_time: Duration,
    ) {
        // egui::Window::new("Stellar")
        //     .resizable(true)
        //     .show(&ctx, |ui| {
        //         ui.label("Hello World");
        //         if ui.secondary_button("Click me!").clicked() {}
        //         ui.allocate_space(ui.available_size())
        //     });

        egui::TopBottomPanel::top("top").show(&ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Simulation", |ui| {});
                ui.menu_button("Graphics", |ui| {});
            });
        });

        egui::SidePanel::left("left").show(&ctx, |ui| {
            // ui.label("Hello World");
            // if ui.secondary_button("Click Me!").clicked() {}

            // ui.allocate_space(ui.available_size())

            let mut transform = world.get::<&mut Transform>(self.camera).unwrap();

            ui.add(egui::Slider::new(&mut transform.translation[0], -5.0..=5.0));
            ui.add(egui::Slider::new(&mut transform.translation[1], -5.0..=5.0));
            ui.add(egui::Slider::new(&mut transform.translation[2], 0.0..=10.0));
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(&ctx, |ui| {
                egui::Frame::canvas(ui.style())
                    .corner_radius(0)
                    .inner_margin(0)
                    .outer_margin(0)
                    .stroke(egui::Stroke::NONE)
                    .fill(Color32::BLACK)
                    .show(ui, |ui| {
                        let (_, rect) = ui.allocate_space(ui.available_size());
                        let viewport =
                            ViewportInPixels::from_points(&rect, ui.pixels_per_point(), screen);

                        if viewport.width_px == 0 || viewport.height_px == 0 {
                            return;
                        }

                        // Update Camera
                        let mut camera = world.get::<&mut Camera>(self.camera).unwrap();
                        camera.update(viewport.width_px as u32, viewport.height_px as u32);
                        drop(camera);

                        ui.painter().add(UiCallback::new_paint_callback(
                            rect,
                            DrawCameraCallback::new(self.camera),
                        ));
                    })
            });
    }

    pub fn cleanup(&mut self, _world: &mut World) {}
}
