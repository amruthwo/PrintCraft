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
        number(ui, value, min..=max, 1.0, |d| d.speed(0.2)).on_hover_text("Type a number, drag left and right, or scroll");
        if ui.add_enabled(*value < max, egui::Button::new(RichText::new("+").strong()).min_size(vec2(28.0, 26.0))).on_hover_text("One more").clicked() {
            *value += 1;
        }
    });
    *value != before
}

/// A slider that also follows the mouse wheel or trackpad while the pointer
/// is over it, and the arrow keys once it has been clicked.
/// `style` adds the usual touches: `|s| s.text("Size").suffix(" pt")`.
pub fn slider<N: egui::emath::Numeric>(ui: &mut egui::Ui, value: &mut N, range: std::ops::RangeInclusive<N>, style: impl FnOnce(egui::Slider<'_>) -> egui::Slider<'_>) -> egui::Response {
    let (lo, hi) = (range.start().to_f64(), range.end().to_f64());
    let mut resp = ui.add(style(egui::Slider::new(value, range)));
    if resp.clicked() || resp.drag_started() {
        resp.request_focus();
    }
    let step = if N::INTEGRAL { ((hi - lo) / 50.0).round().max(1.0) } else { (hi - lo) / 50.0 };
    let steps = wheel_steps(ui, &resp);
    if steps != 0.0 {
        let v = (value.to_f64() + steps * step).clamp(lo, hi);
        if v != value.to_f64() {
            *value = N::from_f64(v);
            resp.mark_changed();
        }
    }
    resp
}

/// A number box (drag left and right, or type) that also follows the
/// mouse wheel or trackpad while the pointer is over it.
pub fn number<N: egui::emath::Numeric>(ui: &mut egui::Ui, value: &mut N, range: std::ops::RangeInclusive<N>, step: f64, style: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>) -> egui::Response {
    let (lo, hi) = (range.start().to_f64(), range.end().to_f64());
    let mut resp = ui.add(style(egui::DragValue::new(value).range(range)));
    let steps = wheel_steps(ui, &resp);
    if steps != 0.0 {
        let v = (value.to_f64() + steps * step).clamp(lo, hi);
        if v != value.to_f64() {
            *value = N::from_f64(v);
            resp.mark_changed();
        }
    }
    resp
}

/// Whole wheel "clicks" turned while the pointer is over `resp` (up or right
/// is more). Trackpads send many small movements, so they are added up.
/// The scroll is used up so the window around doesn't scroll as well.
pub fn wheel_steps(ui: &egui::Ui, resp: &egui::Response) -> f64 {
    if !resp.hovered() || !resp.enabled() {
        return 0.0;
    }
    // Wheel "clicks" this frame: a mouse wheel reports lines, a trackpad points.
    let amount: f32 = ui.input(|i| {
        i.events
            .iter()
            .map(|e| match e {
                egui::Event::MouseWheel { unit, delta, modifiers, .. } if !modifiers.command => {
                    let d = delta.y - delta.x;
                    match unit {
                        egui::MouseWheelUnit::Line => d,
                        egui::MouseWheelUnit::Page => d * 10.0,
                        egui::MouseWheelUnit::Point => d / 40.0,
                    }
                }
                _ => 0.0,
            })
            .sum()
    });
    // Keep the window around from scrolling while the wheel is over a control.
    ui.ctx().input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    if amount == 0.0 {
        return 0.0;
    }
    let id = resp.id.with("wheel");
    let mut acc: f32 = ui.data(|m| m.get_temp(id).unwrap_or(0.0));
    acc += amount;
    let steps = acc.trunc();
    acc -= steps;
    ui.data_mut(|m| m.insert_temp(id, acc));
    steps as f64
}
