//! Toolbar and panel icons, drawn from simple shapes so they scale cleanly
//! in light and dark mode. Original drawings that echo the familiar meaning
//! (page, folder, disk, printer...), not copies of any product's artwork.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    New,
    Open,
    Save,
    Print,
    Preview,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    FillColor,
    LineColor,
    Help,
    Sun,
    Moon,
    // panel sections
    Sparkle,
    Picture,
    Letter,
    Wand,
    Seal,
    Camera,
    Pencil,
    Contacts,
    Flag,
    Cloud,
    ZoomIn,
    ZoomOut,
}

const BLUE: Color32 = Color32::from_rgb(47, 143, 216);
const CORAL: Color32 = Color32::from_rgb(235, 106, 92);
const GOLD: Color32 = Color32::from_rgb(242, 182, 49);
const GREEN: Color32 = Color32::from_rgb(76, 182, 140);
const PURPLE: Color32 = Color32::from_rgb(150, 112, 210);

fn p(r: Rect, x: f32, y: f32) -> Pos2 {
    pos2(r.left() + x * r.width(), r.top() + y * r.height())
}

fn poly(painter: &Painter, r: Rect, pts: &[(f32, f32)], fill: Color32, stroke: Stroke) {
    let pts: Vec<Pos2> = pts.iter().map(|(x, y)| p(r, *x, *y)).collect();
    painter.add(Shape::convex_polygon(pts, fill, stroke));
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, ink: Color32) {
    let r = rect.shrink(rect.width() * 0.08);
    let s = Stroke::new((r.width() / 14.0).max(1.2), ink);
    let thin = Stroke::new((r.width() / 20.0).max(1.0), ink);
    match icon {
        Icon::New => {
            poly(painter, r, &[(0.2, 0.05), (0.65, 0.05), (0.85, 0.25), (0.85, 0.95), (0.2, 0.95)], Color32::WHITE, s);
            painter.line_segment([p(r, 0.65, 0.05), p(r, 0.65, 0.25)], thin);
            painter.line_segment([p(r, 0.65, 0.25), p(r, 0.85, 0.25)], thin);
            painter.circle_filled(p(r, 0.78, 0.8), r.width() * 0.17, GOLD);
        }
        Icon::Open => {
            poly(painter, r, &[(0.05, 0.25), (0.35, 0.25), (0.45, 0.35), (0.9, 0.35), (0.9, 0.85), (0.05, 0.85)], GOLD, s);
            poly(painter, r, &[(0.15, 0.5), (0.98, 0.5), (0.88, 0.85), (0.05, 0.85)], Color32::from_rgb(250, 210, 110), s);
        }
        Icon::Save => {
            let b = Rect::from_min_max(p(r, 0.1, 0.1), p(r, 0.9, 0.9));
            painter.rect(b, 2.0, BLUE, s, egui::StrokeKind::Middle);
            painter.rect_filled(Rect::from_min_max(p(r, 0.28, 0.1), p(r, 0.72, 0.38)), 1.0, Color32::WHITE);
            painter.rect_filled(Rect::from_min_max(p(r, 0.25, 0.58), p(r, 0.75, 0.9)), 1.0, Color32::from_rgb(220, 236, 250));
        }
        Icon::Print => {
            painter.rect(Rect::from_min_max(p(r, 0.28, 0.08), p(r, 0.72, 0.4)), 1.0, Color32::WHITE, thin, egui::StrokeKind::Middle);
            painter.rect(Rect::from_min_max(p(r, 0.05, 0.35), p(r, 0.95, 0.75)), 3.0, Color32::from_rgb(120, 130, 150), s, egui::StrokeKind::Middle);
            painter.rect(Rect::from_min_max(p(r, 0.25, 0.6), p(r, 0.75, 0.95)), 1.0, Color32::WHITE, thin, egui::StrokeKind::Middle);
            painter.circle_filled(p(r, 0.82, 0.48), r.width() * 0.05, GREEN);
        }
        Icon::Preview => {
            poly(painter, r, &[(0.1, 0.05), (0.7, 0.05), (0.7, 0.95), (0.1, 0.95)], Color32::WHITE, s);
            painter.circle_stroke(p(r, 0.55, 0.55), r.width() * 0.22, Stroke::new(s.width * 1.2, BLUE));
            painter.line_segment([p(r, 0.7, 0.72), p(r, 0.95, 0.97)], Stroke::new(s.width * 1.6, BLUE));
        }
        Icon::Cut => {
            painter.circle_stroke(p(r, 0.28, 0.78), r.width() * 0.15, Stroke::new(s.width, CORAL));
            painter.circle_stroke(p(r, 0.72, 0.78), r.width() * 0.15, Stroke::new(s.width, CORAL));
            painter.line_segment([p(r, 0.36, 0.66), p(r, 0.75, 0.05)], s);
            painter.line_segment([p(r, 0.64, 0.66), p(r, 0.25, 0.05)], s);
        }
        Icon::Copy => {
            painter.rect(Rect::from_min_max(p(r, 0.08, 0.08), p(r, 0.62, 0.7)), 2.0, Color32::WHITE, thin, egui::StrokeKind::Middle);
            painter.rect(Rect::from_min_max(p(r, 0.38, 0.3), p(r, 0.92, 0.92)), 2.0, Color32::from_rgb(220, 236, 250), s, egui::StrokeKind::Middle);
        }
        Icon::Paste => {
            painter.rect(Rect::from_min_max(p(r, 0.12, 0.15), p(r, 0.78, 0.95)), 2.0, Color32::from_rgb(196, 150, 100), s, egui::StrokeKind::Middle);
            painter.rect_filled(Rect::from_min_max(p(r, 0.3, 0.05), p(r, 0.6, 0.22)), 2.0, ink);
            painter.rect(Rect::from_min_max(p(r, 0.42, 0.42), p(r, 0.95, 0.98)), 1.0, Color32::WHITE, thin, egui::StrokeKind::Middle);
        }
        Icon::Undo | Icon::Redo => {
            let flip = if icon == Icon::Redo { -1.0 } else { 1.0 };
            let c = p(r, 0.5, 0.6);
            let rad = r.width() * 0.32;
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.1 - i as f32 / 16.0 * 1.25);
                    pos2(c.x + flip * rad * a.cos(), c.y - rad * a.sin())
                })
                .collect();
            let head = pts[0];
            painter.add(Shape::line(pts, Stroke::new(s.width * 1.3, BLUE)));
            let d = r.width() * 0.16;
            painter.add(Shape::convex_polygon(
                vec![pos2(head.x - flip * d, head.y), pos2(head.x + flip * d, head.y - d * 0.2), pos2(head.x, head.y + d * 1.1)],
                BLUE,
                Stroke::NONE,
            ));
        }
        Icon::FillColor => {
            poly(painter, r, &[(0.2, 0.3), (0.5, 0.05), (0.8, 0.35), (0.5, 0.65)], Color32::WHITE, s);
            painter.circle_filled(p(r, 0.85, 0.6), r.width() * 0.1, CORAL);
        }
        Icon::LineColor => {
            painter.line_segment([p(r, 0.15, 0.85), p(r, 0.75, 0.15)], Stroke::new(s.width * 1.2, ink));
            poly(painter, r, &[(0.7, 0.1), (0.9, 0.1), (0.9, 0.3)], ink, Stroke::NONE);
        }
        Icon::Help => {
            painter.circle(p(r, 0.5, 0.5), r.width() * 0.45, BLUE, Stroke::NONE);
            painter.text(p(r, 0.5, 0.52), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(r.height() * 0.75), Color32::WHITE);
        }
        Icon::Sun => {
            painter.circle_filled(p(r, 0.5, 0.5), r.width() * 0.2, GOLD);
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU;
                let c = p(r, 0.5, 0.5);
                let (i0, o) = (r.width() * 0.3, r.width() * 0.45);
                painter.line_segment([pos2(c.x + i0 * a.cos(), c.y + i0 * a.sin()), pos2(c.x + o * a.cos(), c.y + o * a.sin())], Stroke::new(s.width, GOLD));
            }
        }
        Icon::Moon => {
            painter.circle_filled(p(r, 0.5, 0.5), r.width() * 0.4, GOLD);
            painter.circle_filled(p(r, 0.68, 0.36), r.width() * 0.34, painter_bg(painter));
        }
        Icon::Sparkle => {
            let pts = [(0.5, 0.0), (0.62, 0.38), (1.0, 0.5), (0.62, 0.62), (0.5, 1.0), (0.38, 0.62), (0.0, 0.5), (0.38, 0.38)];
            let c = p(r, 0.5, 0.5);
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                painter.add(Shape::convex_polygon(vec![c, p(r, a.0, a.1), p(r, b.0, b.1)], GOLD, Stroke::NONE));
            }
        }
        Icon::Picture => {
            let b = Rect::from_min_max(p(r, 0.05, 0.15), p(r, 0.95, 0.85));
            painter.rect(b, 2.0, Color32::from_rgb(200, 230, 250), s, egui::StrokeKind::Middle);
            poly(painter, r, &[(0.1, 0.8), (0.4, 0.45), (0.65, 0.8)], GREEN, Stroke::NONE);
            poly(painter, r, &[(0.45, 0.8), (0.68, 0.55), (0.9, 0.8)], Color32::from_rgb(60, 150, 110), Stroke::NONE);
            painter.circle_filled(p(r, 0.72, 0.35), r.width() * 0.08, GOLD);
        }
        Icon::Letter => {
            painter.text(p(r, 0.5, 0.5), egui::Align2::CENTER_CENTER, "A", egui::FontId::proportional(r.height() * 1.05), CORAL);
        }
        Icon::Wand => {
            painter.line_segment([p(r, 0.1, 0.9), p(r, 0.62, 0.38)], Stroke::new(s.width * 1.6, PURPLE));
            poly(painter, r, &[(0.75, 0.05), (0.8, 0.2), (0.95, 0.25), (0.8, 0.3), (0.75, 0.45), (0.7, 0.3), (0.55, 0.25), (0.7, 0.2)], GOLD, Stroke::NONE);
        }
        Icon::Seal => {
            let c = p(r, 0.5, 0.5);
            let pts: Vec<Pos2> = (0..24)
                .map(|i| {
                    let rr = if i % 2 == 0 { 0.48 } else { 0.38 } * r.width();
                    let a = i as f32 / 24.0 * std::f32::consts::TAU;
                    pos2(c.x + rr * a.cos(), c.y + rr * a.sin())
                })
                .collect();
            painter.add(Shape::convex_polygon(pts, GREEN, Stroke::NONE));
            painter.circle_stroke(c, r.width() * 0.24, Stroke::new(thin.width, Color32::WHITE));
        }
        Icon::Camera => {
            painter.rect(Rect::from_min_max(p(r, 0.05, 0.28), p(r, 0.95, 0.88)), 3.0, Color32::from_rgb(90, 100, 120), s, egui::StrokeKind::Middle);
            painter.rect_filled(Rect::from_min_max(p(r, 0.32, 0.15), p(r, 0.62, 0.3)), 1.0, ink);
            painter.circle(p(r, 0.5, 0.58), r.width() * 0.2, Color32::from_rgb(200, 230, 250), Stroke::new(thin.width, Color32::WHITE));
        }
        Icon::Pencil => {
            poly(painter, r, &[(0.68, 0.08), (0.92, 0.32), (0.35, 0.89), (0.11, 0.65)], GOLD, s);
            poly(painter, r, &[(0.11, 0.65), (0.35, 0.89), (0.05, 0.95)], Color32::from_rgb(240, 220, 190), s);
        }
        Icon::Contacts => {
            painter.rect(Rect::from_min_max(p(r, 0.05, 0.15), p(r, 0.95, 0.85)), 3.0, Color32::from_rgb(220, 236, 250), s, egui::StrokeKind::Middle);
            painter.circle_filled(p(r, 0.32, 0.42), r.width() * 0.12, CORAL);
            painter.rect_filled(Rect::from_min_max(p(r, 0.17, 0.58), p(r, 0.47, 0.75)), 4.0, CORAL);
            painter.line_segment([p(r, 0.56, 0.4), p(r, 0.85, 0.4)], thin);
            painter.line_segment([p(r, 0.56, 0.55), p(r, 0.85, 0.55)], thin);
            painter.line_segment([p(r, 0.56, 0.7), p(r, 0.78, 0.7)], thin);
        }
        Icon::Flag => {
            painter.line_segment([p(r, 0.15, 0.05), p(r, 0.15, 0.98)], s);
            for row in 0..3 {
                for col in 0..4 {
                    let dark = (row + col) % 2 == 0;
                    let cell = Rect::from_min_size(p(r, 0.18 + col as f32 * 0.19, 0.08 + row as f32 * 0.17), vec2(r.width() * 0.19, r.height() * 0.17));
                    painter.rect_filled(cell, 0.0, if dark { ink } else { Color32::WHITE });
                }
            }
        }
        Icon::Cloud => {
            painter.circle_filled(p(r, 0.35, 0.55), r.width() * 0.22, BLUE);
            painter.circle_filled(p(r, 0.6, 0.45), r.width() * 0.27, BLUE);
            painter.rect_filled(Rect::from_min_max(p(r, 0.15, 0.55), p(r, 0.85, 0.77)), 6.0, BLUE);
            painter.line_segment([p(r, 0.5, 0.4), p(r, 0.5, 0.7)], Stroke::new(thin.width * 1.4, Color32::WHITE));
            poly(painter, r, &[(0.38, 0.6), (0.62, 0.6), (0.5, 0.74)], Color32::WHITE, Stroke::NONE);
        }
        Icon::ZoomIn | Icon::ZoomOut => {
            painter.circle_stroke(p(r, 0.42, 0.42), r.width() * 0.3, s);
            painter.line_segment([p(r, 0.64, 0.64), p(r, 0.95, 0.95)], Stroke::new(s.width * 1.5, ink));
            painter.line_segment([p(r, 0.28, 0.42), p(r, 0.56, 0.42)], s);
            if icon == Icon::ZoomIn {
                painter.line_segment([p(r, 0.42, 0.28), p(r, 0.42, 0.56)], s);
            }
        }
    }
}

fn painter_bg(painter: &Painter) -> Color32 {
    painter.ctx().global_style().visuals.panel_fill
}

/// A flat icon button with a tooltip.
pub fn button(ui: &mut egui::Ui, icon: Icon, tip: &str, enabled: bool) -> egui::Response {
    let size = vec2(28.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let v = ui.visuals();
    if enabled && resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, v.widgets.hovered.weak_bg_fill);
    }
    let ink = if enabled { v.text_color() } else { v.weak_text_color() };
    let painter = ui.painter().clone();
    paint(&painter, rect.shrink(4.0), icon, ink);
    if !enabled {
        painter.rect_filled(rect, 6.0, v.panel_fill.gamma_multiply(0.55));
    }
    resp.on_hover_text(tip)
}
