mod color_table;
mod design_tokens;

pub mod button;
pub mod icons;
pub mod menu;
mod ui_ext;

pub use ui_ext::UiExt;

use design_tokens::{DesignTokens, design_tokens_of};

/// Apply the Rerun design tokens to the given egui context and install image loaders.
pub fn apply_style_and_install_loaders(egui_ctx: &egui::Context) {
    egui_extras::install_image_loaders(egui_ctx);

    egui_ctx.options_mut(|o| {
        o.fallback_theme = egui::Theme::Dark; // If we don't know the system theme, use this as fallback
    });

    set_themes(egui_ctx);
}

pub trait HasDesignTokens {
    fn tokens(&self) -> &'static DesignTokens;
}

impl HasDesignTokens for egui::Context {
    fn tokens(&self) -> &'static DesignTokens {
        design_tokens_of(self.theme())
    }
}

impl HasDesignTokens for egui::Style {
    fn tokens(&self) -> &'static DesignTokens {
        design_tokens_of_visuals(&self.visuals)
    }
}

impl HasDesignTokens for egui::Visuals {
    fn tokens(&self) -> &'static DesignTokens {
        design_tokens_of_visuals(self)
    }
}

fn design_tokens_of_visuals(visuals: &egui::Visuals) -> &'static DesignTokens {
    if visuals.dark_mode {
        design_tokens_of(egui::Theme::Dark)
    } else {
        design_tokens_of(egui::Theme::Light)
    }
}

fn set_themes(egui_ctx: &egui::Context) {
    // It's the same fonts in dark/light mode:
    design_tokens_of(egui::Theme::Dark).set_fonts(egui_ctx);

    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let mut style = std::sync::Arc::unwrap_or_clone(egui_ctx.style_of(theme));
        design_tokens_of(theme).apply(&mut style);
        egui_ctx.set_style_of(theme, style);
    }
}
