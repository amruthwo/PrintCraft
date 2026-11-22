//! Light and dark looks. Colors are taken from the PrintCraft app icons:
//! warm cream with coral/blue/green for light, plum with gold for dark.

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Look {
    Light,
    Dark,
}

pub struct Palette {
    /// Title bar and the Modern side rail.
    pub chrome: Color32,
    pub workspace: Color32,
    pub panel: Color32,
    pub section: Color32,
    pub section_open: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub ruler: Color32,
    pub ruler_text: Color32,
    pub page_shadow: Color32,
    pub selection: Color32,
}

pub fn palette(look: Look) -> Palette {
    match look {
        Look::Light => Palette {
            chrome: Color32::from_rgb(244, 236, 224),
            workspace: Color32::from_rgb(233, 229, 222),
            panel: Color32::from_rgb(252, 249, 244),
            section: Color32::from_rgb(255, 255, 255),
            section_open: Color32::from_rgb(253, 232, 222),
            accent: Color32::from_rgb(58, 154, 217),
            accent_text: Color32::WHITE,
            text: Color32::from_rgb(38, 42, 52),
            muted: Color32::from_rgb(120, 126, 140),
            ruler: Color32::from_rgb(246, 244, 240),
            ruler_text: Color32::from_rgb(110, 114, 126),
            page_shadow: Color32::from_black_alpha(45),
            selection: Color32::from_rgb(47, 143, 216),
        },
        Look::Dark => Palette {
            chrome: Color32::from_rgb(52, 20, 64),
            workspace: Color32::from_rgb(28, 18, 40),
            panel: Color32::from_rgb(40, 26, 56),
            section: Color32::from_rgb(56, 36, 78),
            section_open: Color32::from_rgb(108, 40, 104),
            accent: Color32::from_rgb(232, 178, 64),
            accent_text: Color32::from_rgb(30, 22, 40),
            text: Color32::from_rgb(236, 230, 244),
            muted: Color32::from_rgb(170, 160, 190),
            ruler: Color32::from_rgb(46, 40, 62),
            ruler_text: Color32::from_rgb(180, 170, 200),
            page_shadow: Color32::from_black_alpha(120),
            selection: Color32::from_rgb(232, 178, 64),
        },
    }
}

pub fn apply(ctx: &egui::Context, look: Look) {
    let p = palette(look);
    let mut v = match look {
        Look::Light => Visuals::light(),
        Look::Dark => Visuals::dark(),
    };
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.extreme_bg_color = if look == Look::Light { Color32::WHITE } else { Color32::from_rgb(28, 24, 38) };
    v.selection.bg_fill = p.accent.gamma_multiply(0.85);
    v.selection.stroke = Stroke::new(1.0, p.accent_text);
    v.hyperlink_color = p.accent;
    v.window_corner_radius = CornerRadius::same(14);
    v.window_stroke = Stroke::new(1.0, p.muted.gamma_multiply(0.3));
    v.window_shadow = eframe::egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(if look == Look::Light { 40 } else { 120 }) };
    v.popup_shadow = eframe::egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if look == Look::Light { 35 } else { 110 }) };
    v.menu_corner_radius = CornerRadius::same(8);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open, &mut v.widgets.noninteractive] {
        w.corner_radius = CornerRadius::same(8);
    }
    v.widgets.inactive.weak_bg_fill = p.section;
    v.widgets.inactive.bg_fill = p.section;
    v.widgets.hovered.weak_bg_fill = p.section_open;
    v.widgets.hovered.bg_fill = p.section_open;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.muted.gamma_multiply(0.35));
    // Visible edges on every button and field, so white controls never
    // melt into the off-white panels; hover and press get the accent.
    let edge = if look == Look::Light { Color32::from_rgb(196, 188, 176) } else { Color32::from_rgb(98, 76, 124) };
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, edge);
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, p.accent);
    v.widgets.active.bg_stroke = Stroke::new(1.5, p.accent);
    v.widgets.active.weak_bg_fill = p.accent.gamma_multiply(0.35);
    v.widgets.open.bg_stroke = Stroke::new(1.5, p.accent);
    v.selection.bg_fill = p.accent;
    v.selection.stroke = Stroke::new(1.5, p.accent_text);
    v.text_cursor.stroke = Stroke::new(2.0, p.accent);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        s.spacing.slider_width = 180.0;
        s.spacing.combo_height = 320.0;
        s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(20.0));
        s.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    });
}
