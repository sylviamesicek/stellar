use std::f32;
use std::time::Duration;

use egui::Color32;
use egui::epaint::ViewportInPixels;
use hecs::World;

use crate::components::{
    BloomCompositeMode, Camera, Global, PanOrbitController, Pipeline, Star, update_pan_orbit_camera,
};
use crate::math::{Projection, Transform};
use crate::renderer::{DrawCameraCallback, UiCallback};
use crate::state::{BlackHole2dState, FractalState, SpaceState, State};

pub struct App {
    global: hecs::Entity,

    state: State,
    prev_state: Option<State>,
    black_hole_2d: BlackHole2dState,
    fractal: FractalState,
    space: SpaceState,

    show_post_processing: bool,
}

pub struct StarPhysics {
    velocity: glam::Vec3,
    mass: f32,
}

pub struct StarAcceleration(glam::Vec3);

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            global: hecs::Entity::DANGLING,

            state: State::BlackHole2d,
            prev_state: None,
            black_hole_2d: BlackHole2dState::new(),
            fractal: FractalState::new(),
            space: SpaceState::new(),

            show_post_processing: false,
        }
    }

    pub fn ui_context(&self) -> egui::Context {
        let ctx = egui::Context::default();

        // toolkit::``apply_style_and_install_loaders``(&ctx);

        ctx
    }

    pub fn start(&mut self, world: &mut World) {
        let mut global = Global::default();
        global.bloom.composite_mode = BloomCompositeMode::Additive;
        global.bloom.intensity = 0.3;
        self.global = world.spawn(hecs::EntityBuilder::new().add(global).build());

        // Initialize initial state
        match self.state {
            State::BlackHole2d => self.black_hole_2d.start(world),
            State::Fractal => self.fractal.start(world),
            State::Space => self.space.start(world),
        }
    }

    pub fn update(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
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
            ui.input(|input| {
                update_pan_orbit_camera(input, delta_time, transform, camera, controller);
            });
        }

        // Update individual state objects
        match self.state {
            State::BlackHole2d => self.black_hole_2d.update(world, delta_time),
            State::Fractal => self.fractal.update(world, delta_time),
            State::Space => self.space.update(world, delta_time),
        }

        // Draw Top Panel
        egui::Panel::top("top").show_inside(ui, |ui| {
            egui::containers::menu::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Simulation", |ui| {
                    ui.selectable_value(&mut self.state, State::BlackHole2d, "BlackHole2d");
                    ui.selectable_value(&mut self.state, State::Fractal, "Fractal");
                    ui.selectable_value(&mut self.state, State::Space, "Space");
                });
                ui.menu_button("Graphics", |ui| {
                    if ui.button("Post-Processing").clicked() {
                        self.show_post_processing = true;
                    }
                });
            });
        });

        // Handle any state changes
        if let Some(prev) = self.prev_state
            && prev != self.state
        {
            match prev {
                State::Fractal => self.fractal.finish(world),
                State::BlackHole2d => self.black_hole_2d.finish(world),
                State::Space => self.space.finish(world),
            }

            match self.state {
                State::BlackHole2d => self.black_hole_2d.start(world),
                State::Fractal => self.fractal.start(world),
                State::Space => self.space.start(world),
            }
        }
        // We are definiately in the correct state now
        self.prev_state = Some(self.state);

        // Draw individual state ui
        match self.state {
            State::BlackHole2d => self.black_hole_2d.ui(world, ui, screen),
            State::Fractal => self.fractal.ui(world, ui, screen),
            State::Space => self.space.ui(world, ui, screen),
        }

        // Draw post-processing window
        if self.show_post_processing {
            egui::Window::new("Post-Processing")
                .open(&mut self.show_post_processing)
                .show(ui, |ui| {
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

    pub fn cleanup(&mut self, world: &mut World) {
        match self.state {
            State::Fractal => self.fractal.finish(world),
            State::BlackHole2d => self.black_hole_2d.finish(world),
            State::Space => self.space.finish(world),
        }
    }
}
