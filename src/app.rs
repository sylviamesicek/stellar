use std::f32;
use std::time::Duration;

use egui::Color32;
use egui::epaint::ViewportInPixels;
use glam::Vec3;
use hecs::World;

use crate::components::{
    BloomCompositeMode, Camera, Global, PanOrbitController, Pipeline, Star, update_pan_orbit_camera,
};
use crate::math::{Projection, Transform};
use crate::renderer::{DrawCameraCallback, UiCallback};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Simulation {
    Fractal,
    #[default]
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Fractal {
    Sierpinski,
    Mandlebulb,
}

pub struct App {
    camera: hecs::Entity,
    global: hecs::Entity,

    star: hecs::Entity,

    simulation: Simulation,
    fractal: Fractal,

    show_post_processing: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            camera: hecs::Entity::DANGLING,
            global: hecs::Entity::DANGLING,
            star: hecs::Entity::DANGLING,

            simulation: Simulation::Standard,
            fractal: Fractal::Mandlebulb,
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
                .add(PanOrbitController::default())
                .build(),
        );
        self.global = world.spawn(hecs::EntityBuilder::new().add(Global::default()).build());
        self.star = world.spawn(
            hecs::EntityBuilder::new()
                .add(Star::sun().with_temperature(2700.0))
                .build(),
        );
    }

    pub fn update(
        &mut self,
        world: &mut World,
        ctx: egui::Context,
        screen: [u32; 2],
        delta_time: Duration,
    ) {
        // Update timers
        for timer in world.query_mut::<&mut Global>() {
            timer.time += delta_time;
        }

        // Update camera positions
        for (transform, camera, controller) in
            world.query_mut::<(&mut Transform, &mut Camera, &mut PanOrbitController)>()
        {
            ctx.input(|input| {
                update_pan_orbit_camera(input, delta_time, transform, camera, controller);
            });
        }

        // egui::Window::new("Stellar")
        //     .resizable(true)
        //     .show(&ctx, |ui| {
        //         ui.label("Hello World");
        //         if ui.secondary_button("Click me!").clicked() {}
        //         ui.allocate_space(ui.available_size())
        //     });

        // Draw Top panel
        egui::TopBottomPanel::top("top").show(&ctx, |ui| {
            egui::containers::menu::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Simulation", |ui| {
                    ui.selectable_value(&mut self.simulation, Simulation::Fractal, "Fractal");
                    ui.selectable_value(&mut self.simulation, Simulation::Standard, "Standard");
                });
                ui.menu_button("Graphics", |ui| {
                    if ui.button("Post-Processing").clicked() {
                        self.show_post_processing = true;
                    }
                });
            });
        });

        // Draw info panel
        egui::SidePanel::left("left").show(&ctx, |ui| {
            // ui.allocate_space(ui.available_size())
            if self.simulation == Simulation::Fractal {
                ui.heading("Fractal");

                ui.horizontal(|ui| {
                    ui.label("Kind:");

                    egui::containers::ComboBox::from_id_salt("fractal_box")
                        .selected_text(format!("{:?}", self.fractal))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.fractal,
                                Fractal::Mandlebulb,
                                "Mandlebulb",
                            );
                            ui.selectable_value(
                                &mut self.fractal,
                                Fractal::Sierpinski,
                                "Sierpinski",
                            );
                        });

                    let mut global = world.get::<&mut Global>(self.global).unwrap();
                    match self.fractal {
                        Fractal::Sierpinski => global.pipeline = Pipeline::Sierpinski,
                        Fractal::Mandlebulb => global.pipeline = Pipeline::Mandlebulb,
                    }
                });

                ui.heading("Camera");

                let mut transform = world.get::<&mut Transform>(self.camera).unwrap();
                ui.add(egui::Slider::new(&mut transform.translation[0], -5.0..=5.0));
                ui.add(egui::Slider::new(&mut transform.translation[1], -5.0..=5.0));
                ui.add(egui::Slider::new(&mut transform.translation[2], 0.0..=10.0));

                let mut camera = world.get::<&mut Camera>(self.camera).unwrap();

                match &mut camera.projection {
                    Projection::Perspective(perspective_projection) => {
                        let mut fov_degree = perspective_projection.fov.to_degrees();
                        ui.add(egui::Slider::new(&mut fov_degree, 15.0..=150.0).text("FoV"));
                        perspective_projection.fov = fov_degree.to_radians();
                    }
                    Projection::Orthographic(_) => {}
                }

                // let controller = world.get::<&PanOrbitController>(self.camera).unwrap();

                // let mut zoom_delta = 0.0;
                // ui.input(|input| zoom_delta = input.zoom_delta());
                // ui.label(format!(
                //     "{}, {:?}, {}",
                //     controller.target_radius, controller.radius, zoom_delta
                // ));

                // drop(controller);
            } else if self.simulation == Simulation::Standard {
                let mut global = world.get::<&mut Global>(self.global).unwrap();
                global.pipeline = Pipeline::Standard;

                let mut star = world.get::<&mut Star>(self.star).unwrap();
                ui.heading("Star");
                ui.add(
                    egui::Slider::new(&mut star.temperature, 800.0..=29200.0).text("Temperature"),
                );
                ui.add(
                    egui::Slider::new(&mut star.granule_frequency, 0.0..=80.0)
                        .text("Granule Frequency"),
                );
                ui.add(
                    egui::Slider::new(&mut star.granule_persistence, 0.0..=1.0)
                        .text("Granule Persistence"),
                );
                ui.add(
                    egui::Slider::new(&mut star.sunspot_frequency, 0.0..=40.0)
                        .text("Sunspot Frequency"),
                );
                ui.add(
                    egui::Slider::new(&mut star.sunspot_threshold, 0.0..=1.0)
                        .text("Sunspot Threshold"),
                );
                let response = ui.add(
                    egui::Button::new("Color Shift")
                        .selected(star.color_shift)
                        .frame_when_inactive(star.color_shift)
                        .frame(true),
                );
                if response.clicked() {
                    star.color_shift = !star.color_shift;
                }
            }
        });

        // Draw central viewport
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

                        let response = ui.interact(
                            rect,
                            egui::Id::new("viewport_interaction"),
                            egui::Sense::all(),
                        );

                        if response.clicked() || response.dragged() {
                            response.request_focus();
                        }

                        // Update Camera
                        let mut camera = world.get::<&mut Camera>(self.camera).unwrap();
                        camera.update(viewport.width_px as u32, viewport.height_px as u32);
                        drop(camera);

                        let mut controller =
                            world.get::<&mut PanOrbitController>(self.camera).unwrap();
                        controller.enabled = response.has_focus();
                        drop(controller);

                        ui.painter().add(UiCallback::new_paint_callback(
                            rect,
                            DrawCameraCallback::new(self.camera),
                        ));
                    });
            });

        // Draw post-processing window
        if self.show_post_processing {
            egui::Window::new("Post-Processing")
                .open(&mut self.show_post_processing)
                .show(&ctx, |ui| {
                    let mut global = world.get::<&mut Global>(self.global).unwrap();

                    ui.label("Tonemapping");
                    ui.add(
                        egui::Slider::new(&mut global.tonemap.pre_saturation, 0.0..=10.0)
                            .text("Pre-Saturation"),
                    );
                    ui.add(
                        egui::Slider::new(&mut global.tonemap.post_saturation, 0.0..=10.0)
                            .text("Post-Saturation"),
                    );
                    ui.add(egui::Slider::new(&mut global.tonemap.gamma, 0.0..=10.0).text("Gamma"));
                    ui.add(
                        egui::Slider::new(&mut global.tonemap.exposure, -10.0..=10.0)
                            .text("Exposure"),
                    );

                    ui.label("Bloom");
                    ui.add(
                        egui::Slider::new(&mut global.bloom.intensity, 0.0..=5.0).text("Intensity"),
                    );
                    ui.add(
                        egui::Slider::new(&mut global.bloom.low_frequency_boost, 0.0..=5.0)
                            .text("Low Frequency Boost"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut global.bloom.low_frequency_boost_curvature,
                            0.0..=5.0,
                        )
                        .text("Low Frequency Boost Curvature"),
                    );
                    ui.add(
                        egui::Slider::new(&mut global.bloom.high_pass_frequency, 0.0..=5.0)
                            .text("High Pass Frequency"),
                    );
                    egui::ComboBox::from_label("Composite Mode")
                        .selected_text(format!("{:?}", global.bloom.composite_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut global.bloom.composite_mode,
                                BloomCompositeMode::Additive,
                                "Additive",
                            );
                            ui.selectable_value(
                                &mut global.bloom.composite_mode,
                                BloomCompositeMode::EnergyConserving,
                                "EnergyConserving",
                            );
                        });
                    ui.add(
                        egui::Slider::new(&mut global.bloom.prefilter.threshold, 0.0..=5.0)
                            .text("Threshold"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut global.bloom.prefilter.threshold_softness,
                            0.0..=5.0,
                        )
                        .text("Threshold Softness"),
                    );
                });
        }
    }

    pub fn cleanup(&mut self, _world: &mut World) {}
}
