use core::f32;
use std::time::Duration;

use egui::{ecolor, epaint::ViewportInPixels};
use peroxide::fuga::{ODEIntegrator, ODEProblem, RKF45};

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

// /// ODE for motion under Newtonian Gravity for a point mass of the given mass (measured in G = 1 units).
// ///
// /// Indices for ODEProblem are
// /// 0..1: Position
// /// 2..3: Velocity
// /// 2: Arc Length
// struct NewtonianGravity(f64);

struct ADMSchwarschild(f64);

impl ODEProblem for ADMSchwarschild {
    fn rhs(&self, t: f64, y: &[f64], dy: &mut [f64]) -> peroxide::prelude::anyhow::Result<()> {
        let rs = 2.0 * self.0;

        let (r, _phi) = (y[0], y[1]);
        let (ur, uphi) = (y[2], y[3]);

        // Common schwarschild factor
        let f1 = 1.0 - rs / r;
        // Lapse
        let _alpha = f1.sqrt();
        // Time component of (contravariant) velocity.
        let u0 = (ur * ur + uphi * uphi / (r * r)).sqrt();

        let dr = f1 * ur / u0;
        let dphi = f1 * r.powi(-2) * uphi / u0;

        let dur = -u0 * rs / (2.0 * r * r)
            - ur.powi(2) / (2.0 * u0) * (rs / r.powi(2))
            - uphi.powi(2) / (2.0 * u0) * ((3.0 * rs / r.powi(4)) - 2.0 / r.powi(3));

        let duphi = 0.0;

        let dl = (f1 * (ur * ur + uphi * uphi / r.powi(2))).sqrt();

        dy[0] = dr;
        dy[1] = dphi;
        dy[2] = dur;
        dy[3] = duphi;
        dy[4] = dl;

        Ok(())
    }
}

// impl ODEProblem for NewtonianGravity {
//     fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) -> peroxide::prelude::anyhow::Result<()> {
//         let position = [y[0], y[1]];
//         let velocity = [y[2], y[3]];

//         let r = (position[0] * position[0] + position[1] * position[1]).sqrt();

//         // Derivative of position is velocity
//         dy[0] = velocity[0];
//         dy[1] = velocity[1];

//         // Velocity changes according to law of gravitation
//         dy[2] = -self.0 * position[0] / (r * r * r);
//         dy[3] = -self.0 * position[1] / (r * r * r);

//         // Arc length is integrated mag of velocity
//         dy[4] = (velocity[0] * velocity[0] + velocity[1] * velocity[1]).sqrt();

//         Ok(())
//     }
// }

#[derive(Debug, Clone, Copy)]
struct RayPosition {
    x: f64,
    y: f64,
    time: f64,
    /// Arc length along ray
    length: f64,
}

impl RayPosition {
    fn from_ray(ray: &[f64], time: f64) -> Self {
        let r = ray[0];
        let phi = ray[1];
        let length = ray[4];

        Self {
            x: r * phi.cos(),
            y: r * phi.sin(),
            time,
            length,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BlackHole2dCurvature {
    #[default]
    Flat,
    Schwarschild,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TemporalMode {
    #[default]
    Full,
    Manual,
    Interactive,
}

pub struct BlackHole2dState {
    eye_fov: f64,
    eye_position: [f64; 2],
    eye_angle: f64,

    ray_count: u64,
    // ray_distance: f64,
    ray_time: f64,
    ray_colors: bool,

    black_hole_mass: f64,
    black_hole_curvature: BlackHole2dCurvature,

    ray_positions: Vec<RayPosition>,
    ray_positions_offsets: Vec<usize>,

    time: f64,
    time_speed: f64,
    time_mode: TemporalMode,
}

impl BlackHole2dState {
    pub fn new() -> Self {
        Self {
            eye_fov: 60.0,
            eye_position: [-10.0, 0.0],
            eye_angle: 0.0,
            ray_count: 10,
            // ray_distance: 100.0,
            ray_time: 1.0,
            ray_colors: false,
            black_hole_mass: 1.0,
            black_hole_curvature: BlackHole2dCurvature::default(),

            ray_positions: Vec::new(),
            ray_positions_offsets: Vec::new(),

            time: 0.0,
            time_speed: 1.0,
            time_mode: TemporalMode::default(),
        }
    }

    pub fn start(&mut self, _world: &mut hecs::World) {}

    pub fn finish(&mut self, _world: &mut hecs::World) {}

    pub fn update(&mut self, _world: &mut hecs::World, _delta_time: Duration) {}

    pub fn ui(
        &mut self,
        _world: &mut hecs::World,
        ui: &mut egui::Ui,
        _screen: [u32; 2],
        delta_time: Duration,
    ) {
        // For detecting changes in settings
        let black_hole_mass = self.black_hole_mass;
        let eye_fov = self.eye_fov;
        let eye_angle = self.eye_angle;
        let ray_count = self.ray_count;
        let ray_time = self.ray_time;

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
                    });
            });

            ui.add(
                egui::Slider::new(&mut self.black_hole_mass, 0.0..=10.0).text("Black Hole Mass"),
            );

            let color_response = ui.add(
                egui::Button::new("Color Mode")
                    .selected(self.ray_colors)
                    .frame(true),
            );
            if color_response.clicked() {
                self.ray_colors = !self.ray_colors;
            }

            ui.heading("Eye Settings");
            ui.add(egui::Slider::new(&mut self.eye_fov, 30.0..=90.0).text("Eye FOV"));
            ui.add(
                egui::Slider::new(&mut self.eye_angle, 0.0..=360.0)
                    .text("Eye Angle")
                    .clamping(egui::SliderClamping::Never),
            );

            ui.heading("Ray Settings");
            ui.add(egui::Slider::new(&mut self.ray_count, 4..=100).text("Ray Count"));
            ui.add(egui::Slider::new(&mut self.ray_time, 0.0..=100.0).text("Max Ray Time"));
            // ui.add(egui::Slider::new(&mut self.ray_distance, 0.0..=100.0).text("Max Ray Distance"));

            ui.horizontal(|ui| {
                ui.label("Temporal Mode:");
                egui::containers::ComboBox::from_id_salt("black_hole_2d_temporal_mode")
                    .selected_text(format!("{:?}", self.time_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.time_mode, TemporalMode::Full, "Full");
                        ui.selectable_value(
                            &mut self.time_mode,
                            TemporalMode::Interactive,
                            "Interative",
                        );
                        ui.selectable_value(&mut self.time_mode, TemporalMode::Manual, "Manual");
                    });
            });
            match self.time_mode {
                TemporalMode::Full => {}
                TemporalMode::Manual => {
                    ui.add(egui::Slider::new(&mut self.time, 0.0..=self.ray_time).text("Time"));
                }
                TemporalMode::Interactive => {
                    ui.add(
                        egui::Slider::new(&mut self.time_speed, 0.0..=10.0).text("Temporal Speed"),
                    );
                    if ui.add(egui::Button::new("Reset").frame(true)).clicked() {
                        self.time = 0.0;
                    }
                }
            }
        });

        let eye_size = 1.0;
        let eye_center = self.eye_position;
        let black_hole_radius = 2.0 * self.black_hole_mass;
        let black_hole_center = glam::DVec2::ZERO;

        // Do we have to recast rays?
        let recast = black_hole_mass != self.black_hole_mass
            || eye_fov != self.eye_fov
            || eye_angle != self.eye_angle
            || ray_count != self.ray_count
            || ray_time != self.ray_time
            || self.ray_positions.len() == 0;

        if recast && self.black_hole_curvature == BlackHole2dCurvature::Schwarschild {
            let integrator = RKF45::new(1.0e-3, 0.9, 0.0, 1.0, 10000);

            let dtheta = self.eye_fov / (self.ray_count + 1) as f64;
            let ray_start = eye_size * 0.80;

            self.ray_positions.clear();
            self.ray_positions_offsets.clear();
            self.ray_positions_offsets.push(0);

            for i in 1..=self.ray_count {
                let angle = (dtheta * i as f64 - self.eye_fov / 2.0 + self.eye_angle).to_radians();

                // Start Position of ray
                let ray_start = glam::DVec2::from([
                    eye_center[0] + ray_start * angle.cos(),
                    eye_center[1] + ray_start * angle.sin(),
                ]);

                let ray_x = ray_start.x;
                let ray_y = ray_start.y;

                let ray_r = (ray_x.powi(2) + ray_y.powi(2)).sqrt();
                let ray_phi = f64::atan2(ray_y, ray_x);

                // Default ray velocity (length = 1)
                let ray_dxdt = angle.cos();
                let ray_dydt = angle.sin();
                // Rescale d(x, y)/dt to have length (1 - rs/r). This results in u0=1 on first step.
                let ray_dxdt = (1.0 - black_hole_radius / ray_r) * ray_dxdt;
                let ray_dydt = (1.0 - black_hole_radius / ray_r) * ray_dydt;
                // Convert to polar velocities
                let ray_drdt = ray_dxdt * ray_phi.cos() + ray_dydt * ray_phi.sin();
                let ray_dphidt =
                    1.0 / ray_r * (-ray_dxdt * ray_phi.sin() + ray_dydt * ray_phi.cos());
                // Assuming u0=1 we can easily convert to covariant form.
                let ray_ur = (1.0 - black_hole_radius / ray_r).powi(-1) * ray_drdt;
                let ray_uphi =
                    (1.0 - black_hole_radius / ray_r).powi(-1) * ray_r * ray_r * ray_dphidt;

                let ray_arc_length = 0.0;

                let mut ray = [ray_r, ray_phi, ray_ur, ray_uphi, ray_arc_length];
                let mut t = 0.0;
                let mut dt = 0.01;

                self.ray_positions.push(RayPosition::from_ray(&ray, t));

                while t <= self.ray_time && ray_r > black_hole_radius * 1.001 {
                    let dt_step = integrator
                        .step(&ADMSchwarschild(self.black_hole_mass), t, &mut ray, dt)
                        .unwrap();

                    t += dt;
                    dt = dt_step;

                    self.ray_positions.push(RayPosition::from_ray(&ray, t));
                }

                self.ray_positions_offsets.push(self.ray_positions.len());
            }

            log::info!("Schwarschild Ray Positions: {}", self.ray_positions.len());
        }

        match self.time_mode {
            TemporalMode::Full => self.time = self.ray_time,
            TemporalMode::Interactive => {
                if self.ray_time > 0.0 {
                    self.time += self.time_speed * delta_time.as_secs_f64();
                    self.time = self.time % self.ray_time;
                }
            }
            TemporalMode::Manual => {}
        };

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

                                let black_hole = egui_plot::Points::new(
                                    "black_hole",
                                    vec![black_hole_center.to_array()],
                                )
                                .radius(black_hole_radius as f32 * ui_points_per_unit)
                                .color(egui::Color32::WHITE);

                                plot_ui.points(black_hole);

                                // Plot Rays

                                if self.ray_time > 0.0 {
                                    let dtheta = self.eye_fov / (self.ray_count + 1) as f64;
                                    let ray_start = eye_size * 0.80;
                                    let ray_end = eye_size * 0.80 + self.time;

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

                                        let mut ray_points = egui_plot::PlotPoints::from(vec![
                                            ray_start.to_array(),
                                            ray_end.to_array(),
                                        ]);

                                        // If curvature isn't flat use ray integration.
                                        if self.black_hole_curvature != BlackHole2dCurvature::Flat {
                                            let positions = &self.ray_positions[self
                                                .ray_positions_offsets
                                                [(i - 1) as usize]
                                                ..self.ray_positions_offsets[i as usize]];

                                            let mut points = Vec::new();

                                            // First index which passes predicate
                                            #[allow(unused_assignments)]
                                            let mut last_index = 0;

                                            for (i, pos) in positions.iter().enumerate() {
                                                if pos.time <= self.time {
                                                    points.push([pos.x, pos.y]);
                                                    last_index = i;
                                                } else {
                                                    break;
                                                }
                                            }

                                            if last_index + 1 < positions.len() {
                                                let source = glam::DVec2::from_array([
                                                    positions[last_index].x,
                                                    positions[last_index].y,
                                                ]);
                                                let source_time = positions[last_index].time;
                                                let target = glam::DVec2::from_array([
                                                    positions[last_index + 1].x,
                                                    positions[last_index + 1].y,
                                                ]);
                                                let target_time = positions[last_index + 1].time;

                                                debug_assert!(
                                                    source_time <= self.time
                                                        && target_time >= self.time
                                                );

                                                let v = source.lerp(
                                                    target,
                                                    (self.time - source_time)
                                                        / (target_time - source_time),
                                                );

                                                if !v.is_nan() {
                                                    points.push(v.to_array());
                                                }
                                            }

                                            ray_points = egui_plot::PlotPoints::from(points);
                                        }

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
