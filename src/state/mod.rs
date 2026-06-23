use core::f32;
use std::time::Duration;

use egui::{ecolor, epaint::ViewportInPixels};

use crate::{
    components::{Camera, Global, PanOrbitController, Pipeline, Star},
    math::{Projection, Transform},
    renderer::{DrawCameraCallback, UiCallback},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    BlackHole2d,
    Fractal,
    Space,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FractalType {
    Sierpinski,
    Mandlebulb,
}

pub struct FractalState {
    camera: hecs::Entity,
    r#type: FractalType,
}

impl FractalState {
    pub fn new() -> Self {
        Self {
            camera: hecs::Entity::DANGLING,
            r#type: FractalType::Mandlebulb,
        }
    }

    pub fn start(&mut self, world: &mut hecs::World) {
        self.camera = world.spawn(
            hecs::EntityBuilder::new()
                .add(
                    Transform::from_xyz(-2.3, 3.5, 0.0).looking_at(glam::Vec3::ZERO, glam::Vec3::Y),
                )
                .add(Camera::perspective(f32::consts::PI / 2.0, 0.1, 1000.0))
                .add(PanOrbitController::default())
                .build(),
        );
    }

    pub fn finish(&mut self, world: &mut hecs::World) {
        world.despawn(self.camera).unwrap();
    }

    pub fn update(&mut self, _world: &mut hecs::World, _delta_time: Duration) {}

    pub fn ui(&mut self, world: &mut hecs::World, ui: &mut egui::Ui, screen: [u32; 2]) {
        egui::Panel::left("fractal_left_panel").show_inside(ui, |ui| {
            ui.heading("Fractal");

            ui.horizontal(|ui| {
                ui.label("Kind:");

                egui::containers::ComboBox::from_id_salt("fractal_box")
                    .selected_text(format!("{:?}", self.r#type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.r#type,
                            FractalType::Mandlebulb,
                            "Mandlebulb",
                        );
                        ui.selectable_value(
                            &mut self.r#type,
                            FractalType::Sierpinski,
                            "Sierpinski",
                        );
                    });

                let mut global_default = Global::default();
                let global = world
                    .query_mut::<&mut Global>()
                    .into_iter()
                    .next()
                    .unwrap_or(&mut global_default);

                match self.r#type {
                    FractalType::Sierpinski => global.pipeline = Pipeline::Sierpinski,
                    FractalType::Mandlebulb => global.pipeline = Pipeline::Mandlebulb,
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
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                egui::Frame::canvas(ui.style())
                    .corner_radius(0)
                    .inner_margin(0)
                    .outer_margin(0)
                    .stroke(egui::Stroke::NONE)
                    .fill(egui::Color32::BLACK)
                    .show(ui, |ui| {
                        let (_, rect) = ui.allocate_space(ui.available_size());
                        let viewport =
                            ViewportInPixels::from_points(&rect, ui.pixels_per_point(), screen);

                        if viewport.width_px == 0 || viewport.height_px == 0 {
                            return;
                        }
                        let response = ui.interact(
                            rect,
                            egui::Id::new("fractal_viewport_interaction"),
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
    }

    // pub fn left_panel(&mut self, world: &mut hecs::World, ui: &mut egui::Ui)
}

struct StarPhysics {
    velocity: glam::Vec3,
    mass: f32,
}

struct StarAcceleration(glam::Vec3);

pub struct SpaceState {
    camera: hecs::Entity,
    star: hecs::Entity,
}

impl Default for SpaceState {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceState {
    pub fn new() -> Self {
        Self {
            camera: hecs::Entity::DANGLING,
            star: hecs::Entity::DANGLING,
        }
    }

    pub fn start(&mut self, world: &mut hecs::World) {
        self.camera = world.spawn(
            hecs::EntityBuilder::new()
                .add(
                    Transform::from_xyz(-2.3, 3.5, 10.5)
                        .looking_at(glam::Vec3::ZERO, glam::Vec3::Y),
                )
                .add(Camera::perspective(f32::consts::PI / 2.0, 0.1, 1000.0))
                .add(PanOrbitController::default())
                .build(),
        );

        self.star = world.spawn(
            hecs::EntityBuilder::new()
                .add(Transform::from_xyz(-3.0, 0.0, 0.0).with_uniform_scale(1.2))
                .add(Star::sun().with_temperature(5800.0))
                .add(StarPhysics {
                    velocity: glam::vec3(0.0, 0.0, -2.3 / 2.0),
                    mass: 40.0,
                })
                .add(StarAcceleration(glam::Vec3::ZERO))
                .build(),
        );

        // world.spawn(
        //     hecs::EntityBuilder::new()
        //         .add(Transform::from_xyz(3.0, 0.0, 0.0).with_uniform_scale(0.86))
        //         .add(Star::sun().with_temperature(3500.0))
        //         .add(StarPhysics {
        //             velocity: glam::vec3(0.0, 0.0, 2.3),
        //             mass: 20.0,
        //         })
        //         .add(StarAcceleration(glam::Vec3::ZERO))
        //         .build(),
        // );

        // world.spawn(
        //     hecs::EntityBuilder::new()
        //         .add(Transform::from_xyz(-2.0, 3.0, 10.0).with_uniform_scale(0.15))
        //         .add(Star::sun().with_temperature(3000.0))
        //         .add(StarPhysics {
        //             velocity: glam::vec3(0.0, 0.5, 0.0),
        //             mass: 0.1,
        //         })
        //         .add(StarAcceleration(glam::Vec3::ZERO))
        //         .build(),
        // );
    }

    pub fn finish(&mut self, world: &mut hecs::World) {
        world.despawn(self.camera).unwrap();
        world.despawn(self.star).unwrap();
    }

    pub fn update(&mut self, world: &mut hecs::World, _delta_time: Duration) {
        let mut global_default = Global::default();
        let global = world
            .query_mut::<&mut Global>()
            .into_iter()
            .next()
            .unwrap_or(&mut global_default);
        global.pipeline = Pipeline::Standard;

        // for (e, transform, _physics, acc) in world
        //     .query::<(
        //         hecs::Entity,
        //         &Transform,
        //         &StarPhysics,
        //         &mut StarAcceleration,
        //     )>()
        //     .into_iter()
        // {
        //     acc.0 = glam::Vec3::ZERO;

        //     for (eother, transform_other, physics_other) in world
        //         .query::<(hecs::Entity, &Transform, &StarPhysics)>()
        //         .into_iter()
        //     {
        //         if eother == e {
        //             continue;
        //         }

        //         let diff = transform_other.translation - transform.translation;
        //         acc.0 += physics_other.mass / diff.length().powi(3) * diff;
        //     }
        // }

        // for (transform, physics, acc) in world
        //     .query::<(&mut Transform, &mut StarPhysics, &mut StarAcceleration)>()
        //     .into_iter()
        // {
        //     let h = delta_time.as_secs_f32();

        //     transform.translation += h * physics.velocity;
        //     physics.velocity += h * acc.0;
        // }
    }

    pub fn ui(&mut self, world: &mut hecs::World, ui: &mut egui::Ui, screen: [u32; 2]) {
        egui::Panel::left("space_left_panel").show_inside(ui, |ui| {
            let mut star = world.get::<&mut Star>(self.star).unwrap();
            // let mut transform = world.get::<&mut Transform>(self.star).unwrap();

            ui.heading("Star");
            // ui.add(egui::Slider::new(&mut transform.translation[0], -5.0..=5.0).text("Star X"));
            // ui.add(egui::Slider::new(&mut transform.translation[1], -5.0..=5.0).text("Star Y"));
            // ui.add(egui::Slider::new(&mut transform.translation[2], -5.0..=5.0).text("Star Z"));

            // let mut scale = transform.scale[0];
            // ui.add(egui::Slider::new(&mut scale, 0.0..=10.0).text("Star Radius"));
            // transform.scale = glam::Vec3::new(scale, scale, scale);

            ui.add(egui::Slider::new(&mut star.temperature, 800.0..=29200.0).text("Temperature"));
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
                egui::Slider::new(&mut star.sunspot_threshold, 0.0..=1.0).text("Sunspot Threshold"),
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
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                egui::Frame::canvas(ui.style())
                    .corner_radius(0)
                    .inner_margin(0)
                    .outer_margin(0)
                    .stroke(egui::Stroke::NONE)
                    .fill(egui::Color32::BLACK)
                    .show(ui, |ui| {
                        let (_, rect) = ui.allocate_space(ui.available_size());
                        let viewport =
                            ViewportInPixels::from_points(&rect, ui.pixels_per_point(), screen);

                        if viewport.width_px == 0 || viewport.height_px == 0 {
                            return;
                        }
                        let response = ui.interact(
                            rect,
                            egui::Id::new("space_viewport_interaction"),
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
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlackHole2dCurvature {
    #[default]
    Flat,
    FauxNewtonian,
    Schwarschild,
}

pub struct BlackHole2dState {
    eye_fov: f64,
    eye_position: [f64; 2],
    eye_angle: f64,

    ray_count: u64,
    ray_distance: f64,
    ray_colors: bool,

    black_hole_mass: f64,
    black_hole_curvature: BlackHole2dCurvature,
}

impl BlackHole2dState {
    pub fn new() -> Self {
        Self {
            eye_fov: 60.0,
            eye_position: [-7.0, 0.0],
            eye_angle: 0.0,
            ray_count: 10,
            ray_distance: 0.0,
            ray_colors: false,
            black_hole_mass: 1.0,
            black_hole_curvature: BlackHole2dCurvature::Flat,
        }
    }

    pub fn start(&mut self, _world: &mut hecs::World) {}

    pub fn finish(&mut self, _world: &mut hecs::World) {}

    pub fn update(&mut self, _world: &mut hecs::World, _delta_time: Duration) {}

    pub fn ui(&mut self, _world: &mut hecs::World, ui: &mut egui::Ui, _screen: [u32; 2]) {
        egui::Panel::left("space_left_panel").show_inside(ui, |ui| {
            ui.heading("Black Hole 2d Demo");

            ui.horizontal(|ui| {
                ui.label("Curvature: ");
                egui::containers::ComboBox::from_id_salt("black_hole_2d_box")
                    .selected_text(format!("{:?}", self.black_hole_curvature))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.black_hole_curvature,
                            BlackHole2dCurvature::Flat,
                            "Flat",
                        );
                        ui.selectable_value(
                            &mut self.black_hole_curvature,
                            BlackHole2dCurvature::Schwarschild,
                            "Schwarschild",
                        );
                        ui.selectable_value(
                            &mut self.black_hole_curvature,
                            BlackHole2dCurvature::FauxNewtonian,
                            "Faux Newtonian",
                        );
                    });
            });

            ui.add(
                egui::Slider::new(&mut self.black_hole_mass, 0.0..=10.0).text("Black Hole Mass"),
            );
            ui.heading("Eye Settings");
            ui.add(egui::Slider::new(&mut self.eye_fov, 30.0..=90.0).text("Eye FOV"));
            ui.add(
                egui::Slider::new(&mut self.eye_angle, 0.0..=360.0)
                    .text("Eye Angle")
                    .clamping(egui::SliderClamping::Never),
            );

            ui.heading("Ray Settings");
            ui.add(egui::Slider::new(&mut self.ray_count, 4..=100).text("Ray Count"));
            ui.add(egui::Slider::new(&mut self.ray_distance, 0.0..=20.0).text("Ray Distance"));
            let color_response = ui.add(
                egui::Button::new("Color Mode")
                    .selected(self.ray_colors)
                    .frame(true),
            );
            if color_response.clicked() {
                self.ray_colors = !self.ray_colors;
            }

            // ui.add(egui::Slider::new(&mut self.eye_angle, -5.0..=5.0));
            // ui.add(egui::Slider::new(&mut transform.translation[2], 0.0..=10.0));
        });

        // Draw central viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                egui::Frame::canvas(ui.style())
                    .corner_radius(0)
                    .inner_margin(0)
                    .outer_margin(0)
                    .stroke(egui::Stroke::NONE)
                    .fill(egui::Color32::BLACK)
                    .show(ui, |ui| {
                        let available_size = ui.available_size();

                        // Construct the Plot and assign maximum available dimensions
                        egui_plot::Plot::new("black_hole_2d_canvas")
                            .width(available_size.x)
                            .height(available_size.y)
                            .allow_zoom(true)
                            .allow_scroll(false)
                            .allow_drag(true)
                            .show_axes(true)
                            .show_grid(true)
                            .default_x_bounds(-10.0, 10.0)
                            .auto_bounds(false)
                            .data_aspect(1.0)
                            .show_x(false)
                            .show_y(false)
                            // .view_aspect(1.0)
                            .show(ui, |plot_ui| {
                                let ui_points_per_unit = plot_ui.transform().dpos_dvalue_x() as f32;

                                let eye_size = 1.0;
                                let eye_center = self.eye_position;
                                let eye_lower = [
                                    eye_center[0]
                                        + eye_size
                                            * (self.eye_angle - self.eye_fov / 2.0)
                                                .to_radians()
                                                .cos(),
                                    eye_center[1]
                                        + eye_size
                                            * (self.eye_angle - self.eye_fov / 2.0)
                                                .to_radians()
                                                .sin(),
                                ];
                                let eye_upper = [
                                    eye_center[0]
                                        + eye_size
                                            * (self.eye_angle + self.eye_fov / 2.0)
                                                .to_radians()
                                                .cos(),
                                    eye_center[1]
                                        + eye_size
                                            * (self.eye_angle + self.eye_fov / 2.0)
                                                .to_radians()
                                                .sin(),
                                ];

                                let eye_frame = egui_plot::PlotPoints::new(vec![
                                    eye_lower, eye_center, eye_upper,
                                ]);

                                let eye_lid: egui_plot::PlotPoints = (0..10)
                                    .map(|i| {
                                        let dtheta = self.eye_fov / 9.0;
                                        let radius = eye_size * 0.80;

                                        let angle =
                                            dtheta * i as f64 - self.eye_fov / 2.0 + self.eye_angle;

                                        [
                                            eye_center[0] + radius * angle.to_radians().cos(),
                                            eye_center[1] + radius * angle.to_radians().sin(),
                                        ]
                                    })
                                    .collect();

                                plot_ui.line(
                                    egui_plot::Line::new("eye_frame", eye_frame)
                                        .color(egui::Color32::WHITE),
                                );
                                plot_ui.line(
                                    egui_plot::Line::new("eye_lid", eye_lid)
                                        .color(egui::Color32::WHITE),
                                );

                                // Plot Black Hole

                                let black_hole_radius = 1.0;
                                let black_hole_center = glam::DVec2::ZERO;

                                let black_hole = egui_plot::Points::new(
                                    "black_hole",
                                    vec![black_hole_center.to_array()],
                                )
                                .radius(black_hole_radius as f32 * ui_points_per_unit)
                                .color(egui::Color32::WHITE);

                                plot_ui.points(black_hole);

                                // Plot Rays

                                if self.ray_distance > 0.0 {
                                    let dtheta = self.eye_fov / (self.ray_count + 1) as f64;
                                    let ray_start = eye_size * 0.80;
                                    let ray_end = eye_size * 0.80 + self.ray_distance;

                                    for i in 1..=self.ray_count {
                                        let angle = (dtheta * i as f64 - self.eye_fov / 2.0
                                            + self.eye_angle)
                                            .to_radians();

                                        let ray_start = glam::DVec2::from([
                                            eye_center[0] + ray_start * angle.cos(),
                                            eye_center[1] + ray_start * angle.sin(),
                                        ]);
                                        let mut ray_end = glam::DVec2::from([
                                            eye_center[0] + ray_end * angle.cos(),
                                            eye_center[1] + ray_end * angle.sin(),
                                        ]);

                                        let ray_dir = ray_end - ray_start;
                                        let ray_to_black_hole = ray_start - black_hole_center;

                                        let a = ray_dir.length_squared();
                                        let b = 2.0 * ray_dir.dot(ray_to_black_hole);
                                        let c = ray_to_black_hole.length_squared()
                                            - black_hole_radius * black_hole_radius;

                                        let disc = b * b - 4.0 * a * c;

                                        if disc > 0.0 {
                                            let t1 = (-b - disc.sqrt()) / (2.0 * a);
                                            let t2 = (-b + disc.sqrt()) / (2.0 * a);
                                            let t1_in_seg = 0.0 <= t1 && t1 <= 1.0;
                                            let t2_in_seg = 0.0 <= t2 && t2 <= 1.0;

                                            // Dark magic to find time
                                            let v1 = t1.max(!t1_in_seg as u8 as f64);
                                            let v2 = t2.max(!t2_in_seg as u8 as f64);

                                            if t1_in_seg || t2_in_seg {
                                                ray_end = ray_start + ray_dir * v1.min(v2);
                                            }
                                        }

                                        let ray_points = egui_plot::PlotPoints::from(vec![
                                            ray_start.to_array(),
                                            ray_end.to_array(),
                                        ]);

                                        let color = if self.ray_colors {
                                            ecolor::Hsva::new(
                                                (i - 1) as f32 / self.ray_count as f32,
                                                1.0,
                                                1.0,
                                                1.0,
                                            )
                                            .into()
                                        } else {
                                            egui::Color32::RED
                                        };

                                        plot_ui.line(
                                            egui_plot::Line::new("ray", ray_points).color(color),
                                        );
                                    }
                                }
                            });
                    });
            });
    }
}
