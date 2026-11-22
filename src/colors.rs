//! One color chooser used everywhere: swatches, recent colors, an
//! eyedropper, simple Hue / Saturation / Lightness sliders, a color code
//! you can copy and paste, and color families (lighter, darker, matching
//! and complementary colors).

use eframe::egui::{self, Color32, Rect, Sense, Stroke, pos2, vec2};

use crate::model::Rgba;
use crate::settings::Settings;

pub const BASIC: [&str; 24] = [
    "#000000", "#4a4e69", "#9a8c98", "#ffffff", "#d1495b", "#ef476f", "#f78c6b", "#f2b631", "#ffd166", "#06d6a0", "#2a9d8f", "#118ab2",
    "#2f8fd8", "#073b4c", "#7b2cbf", "#c77dff", "#8d5524", "#c68642", "#e0ac69", "#a7c957", "#386641", "#fde2e4", "#e2ece9", "#cddafd",
];

pub enum Picked {
    /// A color, and whether the person settled on it (a click on a swatch
    /// or OK) rather than passing through it while sliding.
    Color(Rgba, bool),
    Eyedropper,
}

pub fn c32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0, c.1, c.2, c.3)
}

pub fn to_hsl(c: Rgba) -> (f32, f32, f32) {
    let (r, g, b) = (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

pub fn from_hsl(h: f32, s: f32, l: f32) -> Rgba {
    let h = h.rem_euclid(360.0) / 360.0;
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |t: f32| {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Rgba(f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0), 255)
}

pub fn hex(c: Rgba) -> String {
    format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2)
}

fn parse_hex(s: &str) -> Option<Rgba> {
    let t = s.trim().trim_start_matches('#');
    if let Some(rest) = s.trim().strip_prefix("rgb(") {
        let v: Vec<u8> = rest.trim_end_matches(')').split(',').filter_map(|p| p.trim().parse().ok()).collect();
        return (v.len() == 3).then(|| Rgba(v[0], v[1], v[2], 255));
    }
    let t = if t.len() == 3 { t.chars().flat_map(|c| [c, c]).collect::<String>() } else { t.to_string() };
    (t.len() == 6 && t.chars().all(|c| c.is_ascii_hexdigit())).then(|| Rgba::hex(&t))
}

/// A small square swatch; returns true when clicked.
pub fn swatch(ui: &mut egui::Ui, c: Rgba, size: f32, tip: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if c.is_clear() {
        ui.painter().rect_filled(r, 4.0, Color32::WHITE);
        ui.painter().line_segment([r.left_bottom(), r.right_top()], Stroke::new(1.5, Color32::RED));
    } else {
        ui.painter().rect_filled(r, 4.0, c32(c));
    }
    let edge = if resp.hovered() { ui.visuals().selection.bg_fill } else { Color32::from_black_alpha(60) };
    ui.painter().rect_stroke(r, 4.0, Stroke::new(if resp.hovered() { 2.0 } else { 1.0 }, edge), egui::StrokeKind::Inside);
    let resp = if tip.is_empty() { resp.on_hover_text(hex(c)) } else { resp.on_hover_text(format!("{tip} ({})", hex(c))) };
    resp.clicked()
}

/// A painted slider bar: `paint(t)` gives the color at position t (0..1).
fn bar(ui: &mut egui::Ui, value: &mut f32, paint: &dyn Fn(f32) -> Color32) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width().min(260.0), 18.0), Sense::click_and_drag());
    let n = 48;
    for i in 0..n {
        let t0 = i as f32 / n as f32;
        let seg = Rect::from_min_max(pos2(r.left() + r.width() * t0, r.top()), pos2(r.left() + r.width() * (i + 1) as f32 / n as f32 + 0.5, r.bottom()));
        ui.painter().rect_filled(seg, 0.0, paint(t0 + 0.5 / n as f32));
    }
    ui.painter().rect_stroke(r, 4.0, Stroke::new(1.0, Color32::from_black_alpha(80)), egui::StrokeKind::Inside);
    let x = r.left() + r.width() * value.clamp(0.0, 1.0);
    ui.painter().rect(Rect::from_center_size(pos2(x, r.center().y), vec2(6.0, 22.0)), 2.0, Color32::WHITE, Stroke::new(1.5, Color32::BLACK), egui::StrokeKind::Middle);
    if let Some(p) = resp.interact_pointer_pos()
        && (resp.dragged() || resp.clicked())
    {
        *value = ((p.x - r.left()) / r.width()).clamp(0.0, 1.0);
        return true;
    }
    false
}

/// The whole chooser. `allow_none` adds "No color".
pub fn chooser(ui: &mut egui::Ui, id: &str, current: Rgba, settings: &mut Settings, allow_none: bool) -> Option<Picked> {
    let mut out = None;
    ui.set_max_width(300.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Basic colors").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("💧 Eyedropper").on_hover_text("Pick a color from anything on the page").clicked() {
                out = Some(Picked::Eyedropper);
            }
        });
    });
    egui::Grid::new((id, "basic")).spacing([3.0, 3.0]).show(ui, |ui| {
        for (i, h) in BASIC.iter().enumerate() {
            if swatch(ui, Rgba::hex(h), 22.0, "") {
                out = Some(Picked::Color(Rgba::hex(h), true));
            }
            if i % 12 == 11 {
                ui.end_row();
            }
        }
    });
    if !settings.recent_colors.is_empty() {
        ui.label(egui::RichText::new("Recently used").strong());
        ui.horizontal(|ui| {
            for c in settings.recent_colors.clone() {
                if swatch(ui, c, 22.0, "Recent") {
                    out = Some(Picked::Color(c, true));
                }
            }
        });
    }
    ui.separator();
    // Hue / saturation / lightness, kept between frames so gray doesn't lose its hue.
    let key = egui::Id::new(("hsl", id));
    let (mut h, mut s, mut l) = ui.ctx().data(|d| d.get_temp::<(f32, f32, f32, Rgba)>(key)).filter(|v| v.3 == current).map(|v| (v.0, v.1, v.2)).unwrap_or_else(|| to_hsl(current));
    let mut ht = h / 360.0;
    ui.label("Color (hue)");
    let mut changed = bar(ui, &mut ht, &|t| c32(from_hsl(t * 360.0, 0.9, 0.5)));
    ui.label("Strength (saturation)");
    changed |= bar(ui, &mut s, &|t| c32(from_hsl(ht * 360.0, t, l.clamp(0.25, 0.75))));
    ui.label("Light or dark (lightness)");
    changed |= bar(ui, &mut l, &|t| c32(from_hsl(ht * 360.0, s, t)));
    h = ht * 360.0;
    if changed {
        let c = from_hsl(h, s, l);
        ui.ctx().data_mut(|d| d.insert_temp(key, (h, s, l, c)));
        out = Some(Picked::Color(c, false));
    }
    // Color families.
    ui.label(egui::RichText::new("Color families").strong()).on_hover_text("Colors that go well with the current one");
    let fam = |ui: &mut egui::Ui, label: &str, list: Vec<Rgba>, out: &mut Option<Picked>| {
        ui.horizontal(|ui| {
            ui.add_sized([96.0, 20.0], egui::Label::new(egui::RichText::new(label).size(12.0)));
            for c in list {
                if swatch(ui, c, 20.0, label) {
                    *out = Some(Picked::Color(c, true));
                }
            }
        });
    };
    let base_s = s.max(0.35);
    fam(ui, "Lighter / darker", [0.88, 0.75, 0.6, 0.45, 0.3, 0.18].iter().map(|v| from_hsl(h, s, *v)).collect(), &mut out);
    fam(ui, "Matching", [-40.0, -20.0, 0.0, 20.0, 40.0].iter().map(|d| from_hsl(h + d, base_s, l.clamp(0.3, 0.7))).collect(), &mut out);
    fam(ui, "Complementary", [0.75, 0.55, 0.4].iter().map(|v| from_hsl(h + 180.0, base_s, *v)).chain([0.55].iter().map(|v| from_hsl(h, base_s, *v))).collect(), &mut out);
    fam(ui, "Three-way", [0.0, 120.0, 240.0].iter().map(|d| from_hsl(h + d, base_s, l.clamp(0.35, 0.65))).collect(), &mut out);
    ui.separator();
    // Color code with copy and paste.
    let code_key = egui::Id::new(("code", id));
    let mut code = ui.ctx().data(|d| d.get_temp::<(String, Rgba)>(code_key)).filter(|v| v.1 == current).map(|v| v.0).unwrap_or_else(|| hex(current));
    ui.horizontal(|ui| {
        ui.label("Color code");
        let r = ui.add(egui::TextEdit::singleline(&mut code).desired_width(80.0)).on_hover_text("Like #FF8800 or rgb(255, 136, 0)");
        if r.changed()
            && let Some(c) = parse_hex(&code)
        {
            out = Some(Picked::Color(c, false));
        }
        if ui.small_button("Copy").clicked() {
            ui.ctx().copy_text(hex(current));
        }
        if ui.small_button("Paste").clicked()
            && let Some(t) = crate::platform::clipboard_text()
            && let Some(c) = parse_hex(&t)
        {
            code = hex(c);
            out = Some(Picked::Color(c, true));
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(code_key, (code, current)));
    ui.horizontal(|ui| {
        if allow_none && ui.button("No color").clicked() {
            out = Some(Picked::Color(Rgba::NONE, true));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let pending = match &out {
                Some(Picked::Color(c, _)) => *c,
                _ => current,
            };
            if ui.add(egui::Button::new(egui::RichText::new("OK").strong()).min_size(vec2(70.0, 26.0))).on_hover_text("Use this color (Enter)").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                out = Some(Picked::Color(pending, true));
            }
        });
    });
    // Only colors the person settles on go into Recently Used.
    if let Some(Picked::Color(c, true)) = &out {
        settings.remember_color(*c);
    }
    out
}

/// A labelled swatch button that opens the chooser.
pub fn button(ui: &mut egui::Ui, id: &str, label: &str, c: &mut Rgba, settings: &mut Settings, eyedrop: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if !label.is_empty() {
            ui.label(label);
        }
        let (r, resp) = ui.allocate_exact_size(vec2(44.0, 22.0), Sense::click());
        ui.painter().rect_filled(r, 5.0, if c.is_clear() { Color32::WHITE } else { c32(*c) });
        ui.painter().rect_stroke(r, 5.0, Stroke::new(1.0, Color32::from_black_alpha(90)), egui::StrokeKind::Inside);
        egui::Popup::from_toggle_button_response(&resp.on_hover_text("Choose a color")).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            match chooser(ui, id, *c, settings, true) {
                Some(Picked::Color(n, done)) => {
                    *c = n;
                    changed = true;
                    if done {
                        ui.close();
                    }
                }
                Some(Picked::Eyedropper) => *eyedrop = true,
                None => {}
            }
        });
    });
    changed
}
