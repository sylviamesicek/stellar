use std::f32;
use std::time::Duration;

use egui::Color32;
use egui::epaint::ViewportInPixels;
use glam::Vec3;
use hecs::World;

use crate::components::{Camera, Global};
use crate::math::Transform;
use crate::renderer::{DrawCameraCallback, UiCallback};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Simulation {
    Sierpinski,
    Mandlebulb,
}

pub struct App {
    camera: hecs::Entity,
    global: hecs::Entity,

    simulation: Simulation,

    show_post_processing: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            camera: hecs::Entity::DANGLING,
            global: hecs::Entity::DANGLING,

            simulation: Simulation::Mandlebulb,
            show_post_processing: false,
        }
    }

    pub fn ui_context(&self) -> egui::Context {
        let ctx = egui::Context::default();

        // toolkit::``apply_style_and_install_loaders``(&ctx);

        ctx
    }

    pub fn start(&mut self, world: &mut World) {
        self.camera = world.spawn(
            hecs::EntityBuilder::new()
                .add(Transform::from_xyz(0.0, 0.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y))
                .add(Camera::perspective(f32::consts::PI / 2.0, 0.1, 1000.0))
                .build(),
        );
        self.global = world.spawn(hecs::EntityBuilder::new().add(Global::default()).build());
    }

    pub fn update(
        &mut self,
        world: &mut World,
        ctx: egui::Context,
        screen: [u32; 2],
        delta_time: Duration,
    ) {
        // Advance timer forwards
        for timer in world.query_mut::<&mut Global>() {
            timer.time += delta_time;
        }

        // egui::Window::new("Stellar")
        //     .resizable(true)
        //     .show(&ctx, |ui| {
        //         ui.label("Hello World");
        //         if ui.secondary_button("Click me!").clicked() {}
        //         ui.allocate_space(ui.available_size())
        //     });

        egui::TopBottomPanel::top("top").show(&ctx, |ui| {
            egui::containers::menu::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("General", |ui| {});
                ui.menu_button("Graphics", |ui| {
                    if ui.button("Post-Processing").clicked() {
                        self.show_post_processing = true;
                    }
                });
            });
        });

        egui::SidePanel::left("left").show(&ctx, |ui| {
            // ui.label("Hello World");
            // if ui.secondary_button("Click Me!").clicked() {}

            // ui.allocate_space(ui.available_size())

            ui.heading("Simulation");

            egui::containers::ComboBox::from_id_salt("simulation_box")
                .selected_text(format!("{:?}", self.simulation))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.simulation, Simulation::Mandlebulb, "Mandlebulb");
                    ui.selectable_value(&mut self.simulation, Simulation::Sierpinski, "Sierpinski");
                });

            let mut transform = world.get::<&mut Transform>(self.camera).unwrap();

            ui.heading("Camera");

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

        if self.show_post_processing {
            egui::Window::new("Post-Processing")
                .open(&mut self.show_post_processing)
                .show(&ctx, |ui| {
                    let mut global = world.get::<&mut Global>(self.global).unwrap();

                    ui.label("Tonemapping");
                    ui.add(
                        egui::Slider::new(&mut global.pre_saturation, 0.0..=10.0)
                            .text("Pre-Saturation"),
                    );
                    ui.add(
                        egui::Slider::new(&mut global.post_saturation, 0.0..=10.0)
                            .text("Post-Saturation"),
                    );
                    ui.add(egui::Slider::new(&mut global.gamma, 0.0..=10.0).text("Gamma"));
                    ui.add(egui::Slider::new(&mut global.exposure, 0.0..=10.0).text("Exposure"));
                });
        }
    }

    pub fn cleanup(&mut self, _world: &mut World) {}
}
