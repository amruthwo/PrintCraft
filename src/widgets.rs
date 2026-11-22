//! Small shared controls with clear, easy targets.

use eframe::egui::{self, RichText, vec2};

/// A whole number with − and + buttons either side (arrow keys and typing
/// work in the middle box too). Returns true when it changed.
pub fn stepper(ui: &mut egui::Ui, value: &mut u32, min: u32, max: u32, label: &str) -> bool {
    let before = *value;
    ui.horizontal(|ui| {
        if !label.is_empty() {
            ui.label(label);
        }
        if ui.add_enabled(*value > min, egui::Button::new(RichText::new("−").strong()).min_size(vec2(28.0, 26.0))).on_hover_text("One less").clicked() {
            *value -= 1;
        }
        ui.add(egui::DragValue::new(value).range(min..=max).speed(0.2)).on_hover_text("Type a number, or drag left and right");
        if ui.add_enabled(*value < max, egui::Button::new(RichText::new("+").strong()).min_size(vec2(28.0, 26.0))).on_hover_text("One more").clicked() {
            *value += 1;
        }
    });
    *value != before
}
