//! Little pictures of each project kind for the New Project window, drawn
//! from shapes in the app's palette.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

use crate::kinds::Kind;

const CORAL: Color32 = Color32::from_rgb(235, 106, 92);
const BLUE: Color32 = Color32::from_rgb(47, 143, 216);
const GOLD: Color32 = Color32::from_rgb(242, 182, 49);
const GREEN: Color32 = Color32::from_rgb(76, 182, 140);
const PURPLE: Color32 = Color32::from_rgb(150, 112, 210);
const PAPER: Color32 = Color32::from_rgb(255, 253, 248);
const INK: Color32 = Color32::from_rgb(70, 74, 90);

fn line(p: &Painter, a: Pos2, b: Pos2, c: Color32, w: f32) {
    p.line_segment([a, b], Stroke::new(w, c));
}

fn sheet(p: &Painter, r: Rect) {
    p.rect_filled(r.translate(vec2(2.0, 3.0)), 4.0, Color32::from_black_alpha(35));
    p.rect(r, 4.0, PAPER, Stroke::new(1.2, Color32::from_rgb(200, 196, 188)), eframe::egui::StrokeKind::Middle);
}

fn text_lines(p: &Painter, r: Rect, n: usize, c: Color32) {
    for i in 0..n {
        let y = r.top() + r.height() * (i as f32 + 0.5) / n as f32;
        let w = if i == n - 1 { 0.6 } else { 1.0 };
        line(p, pos2(r.left(), y), pos2(r.left() + r.width() * w, y), c, 2.0);
    }
}

fn heart(p: &Painter, c: Pos2, s: f32, col: Color32) {
    p.circle_filled(c + vec2(-s * 0.25, -s * 0.1), s * 0.28, col);
    p.circle_filled(c + vec2(s * 0.25, -s * 0.1), s * 0.28, col);
    p.add(Shape::convex_polygon(vec![c + vec2(-s * 0.52, -s * 0.02), c + vec2(s * 0.52, -s * 0.02), c + vec2(0.0, s * 0.55)], col, Stroke::NONE));
}

fn star(p: &Painter, c: Pos2, r: f32, col: Color32) {
    let pts: Vec<Pos2> = (0..10)
        .map(|i| {
            let rr = if i % 2 == 0 { r } else { r * 0.45 };
            let a = i as f32 / 10.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            pos2(c.x + rr * a.cos(), c.y + rr * a.sin())
        })
        .collect();
    // Draw as triangles from the center (the star isn't convex).
    for i in 0..10 {
        p.add(Shape::convex_polygon(vec![c, pts[i], pts[(i + 1) % 10]], col, Stroke::NONE));
    }
}

/// Draw the picture for `kind` inside `area`.
pub fn paint(p: &Painter, area: Rect, kind: Kind) {
    let c = area.center();
    let box_ = |w: f32, h: f32| Rect::from_center_size(c, vec2(w, h));
    match kind {
        Kind::CardQuarter | Kind::CardHalf | Kind::NoteCard => {
            let (w, h) = match kind {
                Kind::NoteCard => (48.0, 36.0),
                Kind::CardHalf => (40.0, 54.0),
                _ => (34.0, 46.0),
            };
            // Folded card standing slightly open: back panel then front.
            let back = box_(w, h).translate(vec2(8.0, -2.0));
            sheet(p, back);
            let f = box_(w, h).translate(vec2(-6.0, 2.0));
            p.add(Shape::convex_polygon(vec![f.left_top(), f.right_top() + vec2(0.0, -4.0), f.right_bottom() + vec2(0.0, 4.0), f.left_bottom()], CORAL, Stroke::new(1.0, Color32::from_rgb(190, 80, 70))));
            if kind == Kind::NoteCard {
                for (k, col) in [GOLD, Color32::WHITE, GOLD, Color32::WHITE, GOLD].iter().enumerate() {
                    let a = k as f32 / 5.0 * std::f32::consts::TAU;
                    p.circle_filled(f.center() + vec2(7.0 * a.cos(), 7.0 * a.sin()), 4.0, *col);
                }
                p.circle_filled(f.center(), 3.5, GREEN);
            } else {
                heart(p, f.center() + vec2(0.0, -3.0), 18.0, Color32::WHITE);
                line(p, f.center() + vec2(-9.0, 13.0), f.center() + vec2(9.0, 13.0), Color32::WHITE, 2.0);
            }
        }
        Kind::Sign | Kind::Letterhead | Kind::Newsletter | Kind::FaxCover => {
            let r = box_(42.0, 54.0);
            sheet(p, r);
            let inner = r.shrink(6.0);
            match kind {
                Kind::Sign => {
                    p.rect_filled(Rect::from_min_size(inner.min, vec2(inner.width(), 12.0)), 2.0, BLUE);
                    star(p, inner.center() + vec2(0.0, 2.0), 10.0, GOLD);
                    text_lines(p, Rect::from_min_max(pos2(inner.left() + 4.0, inner.bottom() - 10.0), inner.right_bottom() - vec2(4.0, 0.0)), 2, INK);
                }
                Kind::Letterhead => {
                    p.circle_filled(inner.min + vec2(5.0, 5.0), 5.0, PURPLE);
                    line(p, inner.min + vec2(13.0, 3.0), inner.min + vec2(inner.width(), 3.0), PURPLE, 2.5);
                    line(p, inner.min + vec2(13.0, 8.0), inner.min + vec2(inner.width() * 0.8, 8.0), Color32::from_gray(170), 1.5);
                    text_lines(p, Rect::from_min_max(inner.min + vec2(0.0, 16.0), inner.max), 5, Color32::from_gray(160));
                }
                Kind::Newsletter => {
                    p.rect_filled(Rect::from_min_size(inner.min, vec2(inner.width(), 9.0)), 2.0, PURPLE);
                    let body = Rect::from_min_max(inner.min + vec2(0.0, 13.0), inner.max);
                    let half = body.width() / 2.0 - 2.0;
                    p.rect_filled(Rect::from_min_size(body.min, vec2(half, 12.0)), 2.0, GOLD);
                    text_lines(p, Rect::from_min_max(body.min + vec2(0.0, 15.0), pos2(body.left() + half, body.bottom())), 4, Color32::from_gray(150));
                    text_lines(p, Rect::from_min_max(pos2(body.left() + half + 4.0, body.top()), body.max), 6, Color32::from_gray(150));
                }
                _ => {
                    p.rect_filled(Rect::from_min_size(inner.min, vec2(inner.width(), 11.0)), 2.0, INK);
                    p.text(inner.min + vec2(inner.width() / 2.0, 5.5), eframe::egui::Align2::CENTER_CENTER, "FAX", eframe::egui::FontId::proportional(8.0), Color32::WHITE);
                    for i in 0..4 {
                        let y = inner.top() + 17.0 + i as f32 * 7.0;
                        line(p, pos2(inner.left(), y), pos2(inner.left() + 10.0, y), INK, 1.5);
                        line(p, pos2(inner.left() + 13.0, y), pos2(inner.right(), y), Color32::from_gray(185), 1.0);
                    }
                }
            }
        }
        Kind::Banner => {
            let r = box_(96.0, 26.0);
            let pts = vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
            p.add(Shape::convex_polygon(pts, GOLD, Stroke::NONE));
            for (x, dir) in [(r.left(), -1.0), (r.right(), 1.0)] {
                p.add(Shape::convex_polygon(
                    vec![pos2(x, r.top() + 4.0), pos2(x + dir * 12.0, r.top() + 4.0), pos2(x + dir * 7.0, r.center().y + 2.0), pos2(x + dir * 12.0, r.bottom() + 4.0), pos2(x, r.bottom() + 4.0)],
                    Color32::from_rgb(214, 150, 30),
                    Stroke::NONE,
                ));
            }
            p.add(Shape::convex_polygon(vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], GOLD, Stroke::NONE));
            p.text(r.center(), eframe::egui::Align2::CENTER_CENTER, "HOORAY!", eframe::egui::FontId::proportional(14.0), Color32::from_rgb(120, 60, 10));
        }
        Kind::Calendar => {
            let r = box_(60.0, 50.0);
            sheet(p, r);
            p.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), 13.0)), 4.0, CORAL);
            for i in 0..4 {
                let x = r.left() + 10.0 + i as f32 * 13.0;
                p.circle_stroke(pos2(x, r.top()), 3.0, Stroke::new(1.5, INK));
            }
            for row in 0..4 {
                for col in 0..7 {
                    let cell = Rect::from_min_size(pos2(r.left() + 4.0 + col as f32 * 7.5, r.top() + 17.0 + row as f32 * 7.5), vec2(6.0, 6.0));
                    let col_c = if row == 2 && col == 4 { BLUE } else { Color32::from_gray(222) };
                    p.rect_filled(cell, 1.0, col_c);
                }
            }
        }
        Kind::Certificate => {
            let r = box_(66.0, 48.0);
            sheet(p, r);
            p.rect_stroke(r.shrink(4.0), 2.0, Stroke::new(2.0, BLUE), eframe::egui::StrokeKind::Middle);
            line(p, pos2(r.left() + 14.0, r.top() + 14.0), pos2(r.right() - 14.0, r.top() + 14.0), INK, 2.5);
            text_lines(p, Rect::from_min_max(pos2(r.left() + 12.0, r.top() + 20.0), pos2(r.right() - 22.0, r.top() + 34.0)), 2, Color32::from_gray(165));
            let seal = pos2(r.right() - 14.0, r.bottom() - 13.0);
            p.add(Shape::convex_polygon(vec![seal + vec2(-5.0, 2.0), seal + vec2(-1.0, 2.0), seal + vec2(-4.0, 13.0), seal + vec2(-7.0, 10.0)], CORAL, Stroke::NONE));
            p.add(Shape::convex_polygon(vec![seal + vec2(1.0, 2.0), seal + vec2(5.0, 2.0), seal + vec2(7.0, 10.0), seal + vec2(4.0, 13.0)], CORAL, Stroke::NONE));
            star(p, seal, 8.5, GOLD);
        }
        Kind::Label => {
            let r = box_(44.0, 56.0);
            sheet(p, r);
            for row in 0..5 {
                for col in 0..2 {
                    let cell = Rect::from_min_size(pos2(r.left() + 4.0 + col as f32 * 19.0, r.top() + 4.0 + row as f32 * 10.0), vec2(17.0, 8.0));
                    p.rect_filled(cell, 3.0, if row == 0 && col == 0 { GREEN } else { Color32::from_rgb(214, 238, 228) });
                }
            }
        }
        Kind::Envelope => {
            let r = box_(70.0, 40.0);
            sheet(p, r);
            p.add(Shape::line(vec![r.left_top(), r.center() + vec2(0.0, 2.0), r.right_top()], Stroke::new(1.5, Color32::from_gray(170))));
            p.rect_filled(Rect::from_min_size(r.right_top() + vec2(-15.0, 4.0), vec2(11.0, 13.0)), 1.0, GREEN);
            text_lines(p, Rect::from_min_max(r.center() + vec2(-14.0, 6.0), r.center() + vec2(16.0, 16.0)), 3, INK);
        }
        Kind::BusinessCard => {
            let back = box_(58.0, 34.0).translate(vec2(6.0, 6.0));
            p.rect_filled(back, 4.0, GREEN);
            let r = box_(58.0, 34.0).translate(vec2(-4.0, -3.0));
            sheet(p, r);
            p.circle_filled(r.min + vec2(11.0, 11.0), 6.0, GREEN);
            line(p, r.min + vec2(21.0, 8.0), r.min + vec2(50.0, 8.0), INK, 2.5);
            text_lines(p, Rect::from_min_max(r.min + vec2(21.0, 13.0), r.min + vec2(46.0, 28.0)), 3, Color32::from_gray(160));
        }
        Kind::Postcard => {
            let r = box_(70.0, 46.0);
            sheet(p, r);
            let pic = Rect::from_min_max(r.min + vec2(4.0, 4.0), pos2(r.center().x - 2.0, r.bottom() - 4.0));
            p.rect_filled(pic, 2.0, Color32::from_rgb(200, 230, 250));
            p.circle_filled(pic.right_top() + vec2(-7.0, 8.0), 4.0, GOLD);
            p.add(Shape::convex_polygon(vec![pic.left_bottom(), pic.center() + vec2(-4.0, 2.0), pic.right_bottom()], GREEN, Stroke::NONE));
            p.rect_filled(Rect::from_min_size(r.right_top() + vec2(-14.0, 4.0), vec2(10.0, 12.0)), 1.0, PURPLE);
            text_lines(p, Rect::from_min_max(pos2(r.center().x + 4.0, r.top() + 20.0), r.max - vec2(5.0, 6.0)), 3, Color32::from_gray(160));
        }
        Kind::Brochure => {
            let r = box_(72.0, 46.0);
            let w3 = r.width() / 3.0;
            for i in 0..3 {
                let x0 = r.left() + i as f32 * w3;
                let skew = if i == 1 { 4.0 } else { 0.0 };
                let pts = vec![pos2(x0, r.top() + skew), pos2(x0 + w3, r.top() + 4.0 - skew), pos2(x0 + w3, r.bottom() - 4.0 + skew), pos2(x0, r.bottom() - skew)];
                p.add(Shape::convex_polygon(pts, if i == 1 { Color32::from_rgb(240, 236, 250) } else { PAPER }, Stroke::new(1.0, Color32::from_gray(190))));
                let inner = Rect::from_min_max(pos2(x0 + 4.0, r.top() + 10.0), pos2(x0 + w3 - 4.0, r.bottom() - 10.0));
                p.rect_filled(Rect::from_min_size(inner.min, vec2(inner.width(), 8.0)), 1.0, PURPLE);
                text_lines(p, Rect::from_min_max(inner.min + vec2(0.0, 11.0), inner.max), 3, Color32::from_gray(160));
            }
        }
        Kind::Scrapbook => {
            let r = box_(52.0, 52.0);
            p.rect_filled(r.translate(vec2(2.0, 3.0)), 4.0, Color32::from_black_alpha(35));
            p.rect_filled(r, 4.0, Color32::from_rgb(250, 232, 210));
            for (off, col) in [(vec2(-9.0, -8.0), BLUE), (vec2(9.0, 7.0), CORAL)] {
                let photo = Rect::from_center_size(r.center() + off, vec2(22.0, 18.0));
                p.rect_filled(photo, 1.0, Color32::WHITE);
                p.rect_filled(photo.shrink(2.5), 0.0, col);
                p.rect_filled(Rect::from_center_size(photo.center_top(), vec2(10.0, 4.0)), 0.0, Color32::from_rgba_unmultiplied(255, 240, 160, 200));
            }
            star(p, r.right_top() + vec2(-8.0, 8.0), 5.0, GOLD);
        }
        Kind::Sticker => {
            p.circle_filled(c + vec2(2.0, 3.0), 24.0, Color32::from_black_alpha(35));
            p.circle_filled(c, 24.0, GREEN);
            p.circle_stroke(c, 19.0, Stroke::new(1.5, Color32::WHITE));
            star(p, c, 11.0, GOLD);
            p.add(Shape::convex_polygon(vec![c + vec2(10.0, 22.0), c + vec2(22.0, 10.0), c + vec2(13.0, 13.0)], Color32::from_rgb(220, 240, 232), Stroke::NONE));
        }
        Kind::IronOn => {
            let s = 1.0;
            let shirt = vec![
                c + vec2(-12.0, -24.0) * s,
                c + vec2(-30.0, -14.0) * s,
                c + vec2(-22.0, -2.0) * s,
                c + vec2(-16.0, -6.0) * s,
                c + vec2(-16.0, 26.0) * s,
                c + vec2(16.0, 26.0) * s,
                c + vec2(16.0, -6.0) * s,
                c + vec2(22.0, -2.0) * s,
                c + vec2(30.0, -14.0) * s,
                c + vec2(12.0, -24.0) * s,
                c + vec2(0.0, -18.0) * s,
            ];
            // Fill as a fan from the chest (outline isn't convex).
            let mid = c + vec2(0.0, 4.0);
            for i in 0..shirt.len() {
                p.add(Shape::convex_polygon(vec![mid, shirt[i], shirt[(i + 1) % shirt.len()]], PURPLE, Stroke::NONE));
            }
            heart(p, c + vec2(0.0, 4.0), 16.0, GOLD);
        }
    }
}

/// Short names that fit on a tile.
pub fn short(kind: Kind) -> &'static str {
    match kind {
        Kind::CardQuarter => "Card (Quarter-Fold)",
        Kind::CardHalf => "Card (Half-Fold)",
        Kind::Sign => "Sign / Poster",
        Kind::IronOn => "Iron-On",
        Kind::Scrapbook => "Scrapbook",
        k => k.label(),
    }
}

/// Pictures for the welcome screen's three choices.
pub fn welcome(p: &Painter, area: Rect, title: &str, color: Color32) {
    let c = area.center();
    let page = Rect::from_center_size(c + vec2(-3.0, 1.0), vec2(28.0, 36.0));
    if title.starts_with("Open") {
        // A folder with a page peeking out.
        sheet(p, Rect::from_center_size(c + vec2(2.0, -6.0), vec2(26.0, 30.0)));
        text_lines(p, Rect::from_center_size(c + vec2(2.0, -10.0), vec2(16.0, 12.0)), 3, Color32::from_gray(170));
        let back = vec![c + vec2(-20.0, -4.0), c + vec2(-8.0, -4.0), c + vec2(-4.0, 0.0), c + vec2(20.0, 0.0), c + vec2(20.0, 18.0), c + vec2(-20.0, 18.0)];
        for i in 1..back.len() - 1 {
            p.add(Shape::convex_polygon(vec![back[0], back[i], back[i + 1]], color.gamma_multiply(0.8), Stroke::NONE));
        }
        p.add(Shape::convex_polygon(vec![c + vec2(-18.0, 4.0), c + vec2(23.0, 4.0), c + vec2(19.0, 18.0), c + vec2(-20.0, 18.0)], color, Stroke::NONE));
    } else if title.starts_with("Just") {
        // A page with a pencil drawing a line.
        sheet(p, page);
        p.add(Shape::line(vec![page.left_center() + vec2(5.0, 6.0), page.center() + vec2(-2.0, 2.0), page.center() + vec2(5.0, 9.0)], Stroke::new(2.0, color)));
        let tip = page.center() + vec2(6.0, 10.0);
        let dir = vec2(1.0, -1.0).normalized();
        let side = vec2(dir.y, -dir.x) * 3.5;
        let end = tip + dir * 22.0;
        p.add(Shape::convex_polygon(vec![tip + dir * 6.0 + side, end + side, end - side, tip + dir * 6.0 - side], GOLD, Stroke::new(1.0, Color32::from_rgb(170, 120, 20))));
        p.add(Shape::convex_polygon(vec![tip, tip + dir * 6.0 + side, tip + dir * 6.0 - side], Color32::from_rgb(240, 214, 170), Stroke::NONE));
        p.add(Shape::convex_polygon(vec![end + side, end + dir * 4.0 + side, end + dir * 4.0 - side, end - side], CORAL, Stroke::NONE));
    } else {
        // A fresh page with a sparkle.
        sheet(p, page);
        text_lines(p, Rect::from_min_max(page.left_bottom() + vec2(5.0, -13.0), page.right_bottom() + vec2(-5.0, -4.0)), 2, Color32::from_gray(185));
        star(p, page.right_top() + vec2(-2.0, 4.0), 10.0, GOLD);
        p.circle_filled(page.center() + vec2(-3.0, -4.0), 5.0, color);
    }
}
