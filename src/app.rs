use hecs::World;

use crate::toolkit;

pub struct App {}

impl App {
    pub fn new() -> Self {
        Self {}
    }

    pub fn egui_context(&self) -> egui::Context {
        let ctx = egui::Context::default();

        toolkit::apply_style_and_install_loaders(&ctx);

        ctx
    }

    pub fn start(&mut self, world: &mut World) {}

    pub fn update(&mut self, world: &mut World, ctx: egui::Context) {
        egui::Window::new("Stellar")
            .resizable(true)
            .show(&ctx, |ui| {
                ui.label("Hello World");
                if ui.button("Click me!").clicked() {}
                ui.allocate_space(ui.available_size())
            });
    }

    pub fn cleanup(&mut self, world: &mut World) {}
}
