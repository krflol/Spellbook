use super::{BG, GOLD, MUTED};
use eframe::egui::{self, RichText};
pub fn primary(label: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(label).color(BG).strong())
        .fill(GOLD)
        .corner_radius(6.0)
}
pub fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(MUTED));
}
