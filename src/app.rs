use std::time::Duration;

use hecs::World;

use crate::toolkit;
use crate::toolkit::UiExt as _;

pub struct App {}

impl App {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ui_context(&self) -> egui::Context {
        let ctx = egui::Context::default();

        toolkit::apply_style_and_install_loaders(&ctx);

        ctx
    }

    pub fn start(&mut self, _world: &mut World) {}

    pub fn update(&mut self, _world: &mut World, ctx: egui::Context, _delta_time: Duration) {
        // egui::Window::new("Stellar")
        //     .resizable(true)
        //     .show(&ctx, |ui| {
        //         ui.label("Hello World");
        //         if ui.secondary_button("Click me!").clicked() {}
        //         ui.allocate_space(ui.available_size())
        //     });

        egui::SidePanel::left("left").show(&ctx, |ui| {
            ui.label("Hello World");
            if ui.secondary_button("Click Me!").clicked() {}
        });
    }

    pub fn cleanup(&mut self, _world: &mut World) {}
}
