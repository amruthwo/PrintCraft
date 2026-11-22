//! One renderer for screen, picture export and printing: each item is drawn
//! into its own pixel buffer (so the editor can cache it) and pages are
//! composed from those buffers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ab_glyph::{Font, FontRef, OutlineCurve};
use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, PixmapPaint, Point, RadialGradient,
    Shader, SpreadMode, Stroke, Transform,
};

use crate::fonts::FontBook;
use crate::model::*;

/// Decoded pictures, shared between threads.
#[derive(Clone, Default)]
pub struct PictureCache {
    inner: Arc<Mutex<HashMap<u64, Arc<Pixmap>>>>,
}

impl PictureCache {
    pub fn get(&self, id: u64, project: &Project) -> Option<Arc<Pixmap>> {
        if let Some(p) = self.inner.lock().unwrap().get(&id) {
            return Some(p.clone());
        }
        let asset = project.assets.get(&id)?;
        let pm = Arc::new(decode_picture(&asset.bytes)?);
        self.inner.lock().unwrap().insert(id, pm.clone());
        Some(pm)
    }
}

pub fn decode_picture(bytes: &[u8]) -> Option<Pixmap> {
    if bytes.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) {
        return crate::legacy::wmf::render(bytes, 1200);
    }
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    rgba_to_pixmap(img.width(), img.height(), img.as_raw())
}

pub fn rgba_to_pixmap(w: u32, h: u32, rgba: &[u8]) -> Option<Pixmap> {
    let mut data = rgba.to_vec();
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u16;
        px[0] = ((px[0] as u16 * a + 127) / 255) as u8;
        px[1] = ((px[1] as u16 * a + 127) / 255) as u8;
        px[2] = ((px[2] as u16 * a + 127) / 255) as u8;
    }
    Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h)?)
}

pub fn pixmap_to_rgba(pm: &Pixmap) -> Vec<u8> {
    let mut out = pm.data().to_vec();
    for px in out.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            px[0] = ((px[0] as u32 * 255 + a / 2) / a).min(255) as u8;
            px[1] = ((px[1] as u32 * 255 + a / 2) / a).min(255) as u8;
            px[2] = ((px[2] as u32 * 255 + a / 2) / a).min(255) as u8;
        }
    }
    out
}

fn color(c: Rgba, tint: f32) -> Color {
    let t = crate::effects::tint(c, tint);
    Color::from_rgba8(t.0, t.1, t.2, t.3)
}

/// Shadow placement for an item, in inches.
fn shadow_geometry(item: &Item, s: &Shadow) -> (f32, f32, f32) {
    let size = crate::effects::effect_size(item.frame.w, item.frame.h);
    if s.dx != 0.0 || s.dy != 0.0 {
        // Older designs: a fixed offset.
        return (s.dx, s.dy, if s.soft { 0.04 } else { 0.0 });
    }
    let (dx, dy) = crate::effects::shadow_offset(s.distance, s.angle, size);
    (dx, dy, crate::effects::shadow_blur(s.softness, size))
}

/// Frames around an item (see effects.rs for the line widths).
fn draw_frame(pm: &mut Pixmap, item: &Item, pad: f32, w: f32, h: f32, ppi: f32) {
    use crate::effects::FrameStyle;
    let style = FrameStyle::from_code(item.frame_style);
    if style == FrameStyle::None {
        return;
    }
    let c = color(if item.stroke.is_clear() { Rgba::BLACK } else { item.stroke }, item.tint);
    let (line, gap) = style.lines();
    let (line, gap) = (line * ppi, gap * ppi);
    let paint = solid(c);
    let st = Stroke { width: line.max(1.0), ..Default::default() };
    // Lines sit just inside the item's edge.
    let rect_at = |inset: f32| tiny_skia::Rect::from_xywh(pad + inset, pad + inset, w - 2.0 * inset, h - 2.0 * inset);
    match style {
        FrameStyle::RoundCorner => {
            let r = crate::effects::corner_radius() * ppi;
            let pts = outline_points(Outline::Rectangle, 1.0, 1.0);
            let _ = pts;
            let mut pb = PathBuilder::new();
            let (x0, y0, x1, y1) = (pad + line / 2.0, pad + line / 2.0, pad + w - line / 2.0, pad + h - line / 2.0);
            pb.move_to(x0 + r, y0);
            pb.line_to(x1 - r, y0);
            pb.quad_to(x1, y0, x1, y0 + r);
            pb.line_to(x1, y1 - r);
            pb.quad_to(x1, y1, x1 - r, y1);
            pb.line_to(x0 + r, y1);
            pb.quad_to(x0, y1, x0, y1 - r);
            pb.line_to(x0, y0 + r);
            pb.quad_to(x0, y0, x0 + r, y0);
            pb.close();
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &paint, &st, Transform::identity(), None);
            }
        }
        FrameStyle::DropShadow => {
            let off = crate::effects::FRAME_SHADOW_OFFSET * ppi;
            if let Some(r) = rect_at(line / 2.0) {
                pm.stroke_path(&PathBuilder::from_rect(r), &paint, &st, Transform::identity(), None);
            }
            // Solid bars along the right and bottom, offset down and right.
            for r in [tiny_skia::Rect::from_xywh(pad + w, pad + off, off, h), tiny_skia::Rect::from_xywh(pad + off, pad + h, w, off)].into_iter().flatten() {
                pm.fill_rect(r, &paint, Transform::identity(), None);
            }
        }
        _ => {
            if let Some(r) = rect_at(line / 2.0) {
                pm.stroke_path(&PathBuilder::from_rect(r), &paint, &st, Transform::identity(), None);
            }
            if gap > 0.0
                && let Some(r) = rect_at(line * 1.5 + gap)
            {
                pm.stroke_path(&PathBuilder::from_rect(r), &paint, &st, Transform::identity(), None);
            }
        }
    }
}

fn solid(c: Color) -> Paint<'static> {
    Paint { shader: Shader::SolidColor(c), anti_alias: true, ..Default::default() }
}

/// Points of an outline shape inside a w×h box, clockwise.
pub fn outline_points(o: Outline, w: f32, h: f32) -> Vec<(f32, f32)> {
    let n = |pts: &[(f32, f32)]| pts.iter().map(|(x, y)| (x * w, y * h)).collect::<Vec<_>>();
    match o {
        Outline::Rectangle => n(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        Outline::Rounded => {
            let r = 0.15 * w.min(h);
            let mut v = Vec::new();
            let corners = [(w - r, r, -90.0), (w - r, h - r, 0.0), (r, h - r, 90.0), (r, r, 180.0)];
            for (cx, cy, start) in corners {
                for i in 0..=8 {
                    let a = (start + i as f32 * 11.25f32).to_radians();
                    v.push((cx + r * a.cos(), cy + r * a.sin()));
                }
            }
            v
        }
        Outline::Ellipse => (0..72)
            .map(|i| {
                let a = i as f32 / 72.0 * std::f32::consts::TAU;
                (w / 2.0 + w / 2.0 * a.cos(), h / 2.0 + h / 2.0 * a.sin())
            })
            .collect(),
        Outline::Triangle => n(&[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        Outline::Diamond => n(&[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)]),
        Outline::Star => star(w, h, 5, 0.42),
        Outline::Burst => star(w, h, 16, 0.72),
        Outline::Heart => (0..90)
            .map(|i| {
                let t = i as f32 / 90.0 * std::f32::consts::TAU;
                let x = 16.0 * t.sin().powi(3);
                let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                (w / 2.0 + x / 34.0 * w, h * 0.42 - y / 30.0 * h)
            })
            .collect(),
        Outline::Arrow => n(&[(0.0, 0.3), (0.6, 0.3), (0.6, 0.0), (1.0, 0.5), (0.6, 1.0), (0.6, 0.7), (0.0, 0.7)]),
        Outline::Pentagon => regular(w, h, 5),
        Outline::Hexagon => regular(w, h, 6),
        Outline::Octagon => regular(w, h, 8),
        Outline::Parallelogram => n(&[(0.22, 0.0), (1.0, 0.0), (0.78, 1.0), (0.0, 1.0)]),
        Outline::Trapezoid => n(&[(0.2, 0.0), (0.8, 0.0), (1.0, 1.0), (0.0, 1.0)]),
        Outline::Cross => n(&[(0.33, 0.0), (0.67, 0.0), (0.67, 0.33), (1.0, 0.33), (1.0, 0.67), (0.67, 0.67), (0.67, 1.0), (0.33, 1.0), (0.33, 0.67), (0.0, 0.67), (0.0, 0.33), (0.33, 0.33)]),
        Outline::Star6 => star(w, h, 6, 0.55),
        Outline::Star8 => star(w, h, 8, 0.6),
        Outline::Moon => {
            let mut v: Vec<(f32, f32)> = (0..=40).map(|i| {
                let a = std::f32::consts::FRAC_PI_2 + i as f32 / 40.0 * std::f32::consts::PI;
                (w * 0.55 + w * 0.45 * a.cos(), h / 2.0 + h / 2.0 * a.sin())
            }).collect();
            v.extend((0..=40).rev().map(|i| {
                let a = std::f32::consts::FRAC_PI_2 + i as f32 / 40.0 * std::f32::consts::PI;
                (w * 0.75 + w * 0.33 * a.cos(), h / 2.0 + h * 0.42 * a.sin())
            }));
            v
        }
        Outline::Cloud => {
            let bumps = [(0.25, 0.62, 0.22), (0.42, 0.4, 0.24), (0.65, 0.38, 0.26), (0.8, 0.6, 0.2), (0.55, 0.7, 0.22), (0.3, 0.75, 0.18)];
            (0..120).map(|i| {
                let a = i as f32 / 120.0 * std::f32::consts::TAU;
                let (dx, dy) = (a.cos(), a.sin());
                // Farthest bump edge in this direction from the middle.
                let r = bumps.iter().map(|(bx, by, br)| {
                    let (ox, oy) = (bx - 0.5, by - 0.55);
                    let b = ox * dx + oy * dy;
                    let c = ox * ox + oy * oy - br * br;
                    let disc = b * b - c;
                    if disc > 0.0 { b + disc.sqrt() } else { 0.0 }
                }).fold(0.0f32, f32::max);
                (w * (0.5 + r * dx), h * (0.55 + r * dy))
            }).collect()
        }
        Outline::Lightning => n(&[(0.55, 0.0), (0.15, 0.55), (0.45, 0.55), (0.3, 1.0), (0.85, 0.4), (0.55, 0.4), (0.75, 0.0)]),
        Outline::Ribbon => n(&[(0.0, 0.25), (0.15, 0.25), (0.15, 0.1), (0.85, 0.1), (0.85, 0.25), (1.0, 0.25), (0.92, 0.5), (1.0, 0.75), (0.85, 0.75), (0.85, 0.9), (0.15, 0.9), (0.15, 0.75), (0.0, 0.75), (0.08, 0.5)]),
        Outline::Shield => {
            let mut v = vec![(0.0, 0.0), (w, 0.0), (w, h * 0.45)];
            v.extend((1..20).map(|i| {
                let t = i as f32 / 20.0;
                (w - w * 0.5 * t, h * 0.45 + h * 0.55 * (t * std::f32::consts::FRAC_PI_2).sin())
            }));
            v.push((w / 2.0, h));
            v.extend((1..20).map(|i| {
                let t = 1.0 - i as f32 / 20.0;
                (w * 0.5 * t, h * 0.45 + h * 0.55 * (t * std::f32::consts::FRAC_PI_2).sin())
            }));
            v.push((0.0, h * 0.45));
            v
        }
        Outline::Tag => n(&[(0.25, 0.0), (1.0, 0.0), (1.0, 1.0), (0.25, 1.0), (0.0, 0.5)]),
        Outline::Chevron => n(&[(0.0, 0.0), (0.7, 0.0), (1.0, 0.5), (0.7, 1.0), (0.0, 1.0), (0.3, 0.5)]),
        Outline::Teardrop => (0..80).map(|i| {
            let t = i as f32 / 80.0 * std::f32::consts::TAU;
            let k = (t / 2.0).sin().powi(2);
            (w / 2.0 + w / 2.0 * t.sin() * k, h - h * (1.0 - t.cos()) / 2.0)
        }).collect(),
        Outline::Ring => {
            let mut v: Vec<(f32, f32)> = (0..=72).map(|i| {
                let a = i as f32 / 72.0 * std::f32::consts::TAU;
                (w / 2.0 + w / 2.0 * a.cos(), h / 2.0 + h / 2.0 * a.sin())
            }).collect();
            v.extend((0..=72).rev().map(|i| {
                let a = i as f32 / 72.0 * std::f32::consts::TAU;
                (w / 2.0 + w * 0.3 * a.cos(), h / 2.0 + h * 0.3 * a.sin())
            }));
            v
        }
        Outline::Thought => (0..96).map(|i| {
            let a = i as f32 / 96.0 * std::f32::consts::TAU;
            let r = 1.0 + 0.06 * (a * 9.0).cos().abs();
            (w / 2.0 + w * 0.46 * r * a.cos(), h * 0.42 + h * 0.38 * r * a.sin())
        }).collect(),
        Outline::Balloon => {
            let mut v: Vec<(f32, f32)> = (0..64)
                .map(|i| {
                    let a = i as f32 / 64.0 * std::f32::consts::TAU;
                    (w / 2.0 + w / 2.0 * a.cos(), h * 0.4 + h * 0.4 * a.sin())
                })
                .collect();
            // tail toward bottom-left
            let at = v.iter().position(|p| p.0 < w * 0.35 && p.1 > h * 0.6).unwrap_or(40);
            v.insert(at, (w * 0.12, h));
            v
        }
    }
}

fn regular(w: f32, h: f32, sides: usize) -> Vec<(f32, f32)> {
    (0..sides)
        .map(|i| {
            let a = i as f32 / sides as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            (w / 2.0 + w / 2.0 * a.cos(), h / 2.0 + h / 2.0 * a.sin())
        })
        .collect()
}

/// Inside span of a closed outline along a horizontal line: the widest
/// stretch that's inside, or None when the line misses the shape.
pub fn span_at(pts: &[(f32, f32)], y: f32) -> Option<(f32, f32)> {
    let mut xs = Vec::new();
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        if (a.1 <= y && b.1 > y) || (b.1 <= y && a.1 > y) {
            xs.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
        }
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs.chunks_exact(2).map(|c| (c[0], c[1])).max_by(|a, b| (a.1 - a.0).partial_cmp(&(b.1 - b.0)).unwrap_or(std::cmp::Ordering::Equal))
}

fn star(w: f32, h: f32, points: usize, inner: f32) -> Vec<(f32, f32)> {
    (0..points * 2)
        .map(|i| {
            let r = if i % 2 == 0 { 1.0 } else { inner };
            let a = (i as f32 / (points * 2) as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            (w / 2.0 + w / 2.0 * r * a.cos(), h / 2.0 + h / 2.0 * r * a.sin())
        })
        .collect()
}

fn poly_path(pts: &[(f32, f32)], dx: f32, dy: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let (x0, y0) = *pts.first()?;
    pb.move_to(x0 + dx, y0 + dy);
    for (x, y) in &pts[1..] {
        pb.line_to(x + dx, y + dy);
    }
    pb.close();
    pb.finish()
}

/// Rendered item plus how much room effects took around the frame.
pub struct Rendered {
    pub pixmap: Pixmap,
    pub pad: f32,
}

/// Draw one item (without its rotation/flip) at `ppi` pixels per inch.
pub fn render_item(item: &Item, project: &Project, fonts: &FontBook, pics: &PictureCache, ppi: f32) -> Option<Rendered> {
    let w = (item.frame.w * ppi).max(1.0);
    let h = (item.frame.h * ppi).max(1.0);
    // Room around the item for strokes, frames, shadows and glows.
    let size = crate::effects::effect_size(item.frame.w, item.frame.h);
    let mut room = item.stroke_pt / 72.0 * line_style_reach(item.line_style) + crate::effects::FrameStyle::from_code(item.frame_style).reach();
    if let Some(s) = &item.shadow {
        let (dx, dy, blur) = shadow_geometry(item, s);
        room = room.max(dx.abs().max(dy.abs()) + 2.0 * blur);
    }
    if item.glow.is_some() {
        room = room.max(crate::effects::glow_reach(item.glow_size, size));
    }
    let pad = (room * ppi).ceil() + 2.0;
    let pw = (w + 2.0 * pad).ceil() as u32;
    let ph = (h + 2.0 * pad).ceil() as u32;
    if pw > 12000 || ph > 12000 {
        return None;
    }
    let mut pm = Pixmap::new(pw, ph)?;
    let stroke_px = item.stroke_pt / 72.0 * ppi;
    match &item.content {
        Content::Shape { outline } => {
            let pts = outline_points(*outline, w, h);
            if let Some(path) = poly_path(&pts, pad, pad) {
                if !item.fill.is_clear() {
                    pm.fill_path(&path, &solid(color(item.fill, item.tint)), FillRule::Winding, Transform::identity(), None);
                }
                if !item.stroke.is_clear() {
                    let moved: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (x + pad, y + pad)).collect();
                    styled_stroke(&mut pm, &moved, true, item.line_style, stroke_px, color(item.stroke, item.tint));
                }
            }
        }
        Content::Line => {
            // Straight across the middle of its box; the turn handle sets the angle.
            let c = if item.stroke.is_clear() { Rgba::BLACK } else { item.stroke };
            styled_stroke(&mut pm, &[(pad, pad + h / 2.0), (pad + w, pad + h / 2.0)], false, item.line_style, stroke_px, color(c, item.tint));
        }
        Content::Ornament { style } => draw_ornament(&mut pm, *style, item, (pad, pad, w, h), ppi),
        Content::Picture { asset } => {
            if let Some(src) = pics.get(*asset, project) {
                let sx = w / src.width() as f32;
                let sy = h / src.height() as f32;
                let paint = PixmapPaint { quality: tiny_skia::FilterQuality::Bicubic, ..Default::default() };
                pm.draw_pixmap(0, 0, src.as_ref().as_ref(), &paint, Transform::from_row(sx, 0.0, 0.0, sy, pad, pad), None);
                if item.tint < 0.999 {
                    let fade = Color::from_rgba8(255, 255, 255, ((1.0 - item.tint) * 255.0) as u8);
                    let r = tiny_skia::Rect::from_xywh(pad, pad, w, h)?;
                    let mut p = solid(fade);
                    p.blend_mode = tiny_skia::BlendMode::SourceAtop;
                    pm.fill_rect(r, &p, Transform::identity(), None);
                }
            } else {
                missing_picture(&mut pm, pad, w, h);
            }
            frame_stroke(&mut pm, item, pad, w, h, stroke_px);
        }
        Content::Text { text, look, shape, fit, runs } => {
            let mut outline = None;
            if let Some(o) = shape {
                let pts = outline_points(*o, w, h);
                if let Some(path) = poly_path(&pts, pad, pad) {
                    let fill = if item.fill.is_clear() { Rgba::WHITE } else { item.fill };
                    pm.fill_path(&path, &solid(color(fill, item.tint)), FillRule::EvenOdd, Transform::identity(), None);
                    let sc = if item.stroke.is_clear() { Rgba::BLACK } else { item.stroke };
                    let moved: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (x + pad, y + pad)).collect();
                    styled_stroke(&mut pm, &moved, true, item.line_style, stroke_px, color(sc, item.tint));
                }
                outline = Some(pts);
            } else {
                if !item.fill.is_clear() {
                    let r = tiny_skia::Rect::from_xywh(pad, pad, w, h)?;
                    pm.fill_rect(r, &solid(color(item.fill, item.tint)), Transform::identity(), None);
                }
                frame_stroke(&mut pm, item, pad, w, h, stroke_px);
            }
            let bytes: Vec<_> = [(false, false), (true, false), (false, true), (true, true)].iter().map(|(b, i)| fonts.face_bytes(&look.family, *b, *i)).collect();
            let faces: Vec<FontRef> = bytes.iter().filter_map(|b| FontRef::try_from_slice(b).ok()).collect();
            if faces.len() == 4 {
                let inset = (0.06 * ppi).max(stroke_px) + 2.0;
                let chars: Vec<char> = text.chars().collect();
                let styles = char_styles(look, runs, chars.len());
                let (has_b, has_i) = (fonts.has_face(&look.family, true, false), fonts.has_face(&look.family, false, true));
                let fake = [(false, false), (true, false), (false, true), (true, true)].map(|(b, i)| (b && !has_b, i && !has_i));
                let block = TextBlock { faces: [&faces[0], &faces[1], &faces[2], &faces[3]], fake, chars, styles, look, w, h, inset, outline: outline.as_deref() };
                let size_px = if *fit { block.fit_size(ppi) } else { look.size / 72.0 * ppi };
                block.draw(&mut pm, size_px, (pad, pad), item.tint);
            }
        }
        Content::Group { items, base_w, base_h } => {
            let (sx, sy) = (item.frame.w / base_w.max(0.01), item.frame.h / base_h.max(0.01));
            for child in items {
                let mut c = child.clone();
                c.frame = Frame::new(c.frame.x * sx, c.frame.y * sy, c.frame.w * sx, c.frame.h * sy);
                if let Some(l) = c.look_mut() {
                    l.size *= sx.min(sy);
                }
                c.tint *= item.tint;
                compose(&mut pm, &c, project, fonts, pics, ppi, (pad, pad));
            }
        }
        Content::Headline { text, look, flair, warp, second, outline, stretch, bend, bulge } => {
            if !item.fill.is_clear() {
                let r = tiny_skia::Rect::from_xywh(pad, pad, w, h)?;
                pm.fill_rect(r, &solid(color(item.fill, item.tint)), Transform::identity(), None);
            }
            let bytes = fonts.face_bytes(&look.family, look.bold, look.italic);
            if let Ok(font) = FontRef::try_from_slice(&bytes) {
                draw_headline(&mut pm, &font, text, look, (*flair, *warp, *stretch, *bend, *bulge), *second, *outline, item.tint, (pad, pad, w, h));
            }
        }
        Content::Border { style } => draw_border(&mut pm, *style, item, (pad, pad, w, h), ppi),
    }
    draw_frame(&mut pm, item, pad, w, h, ppi);
    if let Some(g) = item.glow {
        // The glow reaches glow_reach() past the edge; a blur of half that
        // spreads it there.
        let reach = crate::effects::glow_reach(item.glow_size, size) * ppi;
        add_halo(&mut pm, g, 0.0, 0.0, (reach / 2.0).max(1.0), 3.0);
    }
    if let Some(s) = &item.shadow {
        let (dx, dy, blur) = shadow_geometry(item, s);
        add_halo(&mut pm, s.color, dx * ppi, dy * ppi, blur * ppi, 1.0);
    }
    Some(Rendered { pixmap: pm, pad })
}

fn frame_stroke(pm: &mut Pixmap, item: &Item, pad: f32, w: f32, h: f32, stroke_px: f32) {
    if item.stroke.is_clear() || stroke_px <= 0.0 {
        return;
    }
    let corners = [(pad, pad), (pad + w, pad), (pad + w, pad + h), (pad, pad + h)];
    styled_stroke(pm, &corners, true, item.line_style, stroke_px, color(item.stroke, item.tint));
}

fn missing_picture(pm: &mut Pixmap, pad: f32, w: f32, h: f32) {
    if let Some(r) = tiny_skia::Rect::from_xywh(pad, pad, w, h) {
        pm.fill_rect(r, &solid(Color::from_rgba8(225, 228, 235, 255)), Transform::identity(), None);
        let mut pb = PathBuilder::new();
        pb.move_to(pad, pad);
        pb.line_to(pad + w, pad + h);
        pb.move_to(pad + w, pad);
        pb.line_to(pad, pad + h);
        if let Some(p) = pb.finish() {
            pm.stroke_path(&p, &solid(Color::from_rgba8(160, 166, 178, 255)), &Stroke::default(), Transform::identity(), None);
        }
    }
}

/// Shadow or glow: a colored, optionally blurred copy of the item's alpha,
/// placed behind it.
fn add_halo(pm: &mut Pixmap, c: Rgba, dx: f32, dy: f32, blur: f32, strength: f32) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    // Soft halos are blurred on a smaller copy (same look, far less work),
    // then stretched back up.
    let f = ((blur / 3.0) as usize).clamp(1, 8);
    let (sw, sh) = (w.div_ceil(f), h.div_ceil(f));
    let data = pm.data();
    let mut small = vec![0f32; sw * sh];
    for y in 0..h {
        for x in 0..w {
            small[(y / f) * sw + x / f] += data[(y * w + x) * 4 + 3] as f32;
        }
    }
    let k = 1.0 / (255.0 * (f * f) as f32);
    for v in small.iter_mut() {
        *v *= k;
    }
    let mut a = vec![0f32; sw * sh];
    let (ox, oy) = ((dx / f as f32).round() as i64, (dy / f as f32).round() as i64);
    for y in 0..sh as i64 {
        for x in 0..sw as i64 {
            let (sx, sy) = (x - ox, y - oy);
            if sx >= 0 && sy >= 0 && (sx as usize) < sw && (sy as usize) < sh {
                a[y as usize * sw + x as usize] = small[sy as usize * sw + sx as usize];
            }
        }
    }
    let r = (blur / f as f32).round() as usize;
    if r > 0 {
        box_blur(&mut a, sw, sh, r);
        box_blur(&mut a, sw, sh, r);
    }
    // Sample the small grid smoothly at full size.
    let at = |x: usize, y: usize| -> f32 {
        if f == 1 {
            return a[y * sw + x];
        }
        let fx = ((x as f32 + 0.5) / f as f32 - 0.5).max(0.0);
        let fy = ((y as f32 + 0.5) / f as f32 - 0.5).max(0.0);
        let (x0, y0) = ((fx as usize).min(sw - 1), (fy as usize).min(sh - 1));
        let (x1, y1) = ((x0 + 1).min(sw - 1), (y0 + 1).min(sh - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let top = a[y0 * sw + x0] * (1.0 - tx) + a[y0 * sw + x1] * tx;
        let bottom = a[y1 * sw + x0] * (1.0 - tx) + a[y1 * sw + x1] * tx;
        top * (1.0 - ty) + bottom * ty
    };
    let mut out = Pixmap::new(w as u32, h as u32).unwrap();
    for (i, px) in out.data_mut().chunks_exact_mut(4).enumerate() {
        let al = (at(i % w, i / w) * strength).min(1.0) * c.3 as f32 / 255.0;
        px[0] = (c.0 as f32 * al) as u8;
        px[1] = (c.1 as f32 * al) as u8;
        px[2] = (c.2 as f32 * al) as u8;
        px[3] = (al * 255.0) as u8;
    }
    out.draw_pixmap(0, 0, pm.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    *pm = out;
}

fn box_blur(a: &mut [f32], w: usize, h: usize, r: usize) {
    let mut tmp = vec![0f32; a.len()];
    let k = (2 * r + 1) as f32;
    for y in 0..h {
        let row = &a[y * w..(y + 1) * w];
        let mut acc: f32 = (0..=r).map(|i| row[i.min(w - 1)]).sum::<f32>() + row[0] * r as f32;
        for x in 0..w {
            tmp[y * w + x] = acc / k;
            let add = row[(x + r + 1).min(w - 1)];
            let sub = row[x.saturating_sub(r)];
            acc += add - sub;
        }
    }
    for x in 0..w {
        let mut acc: f32 = (0..=r).map(|i| tmp[i.min(h - 1) * w + x]).sum::<f32>() + tmp[x] * r as f32;
        for y in 0..h {
            a[y * w + x] = acc / k;
            let add = tmp[(y + r + 1).min(h - 1) * w + x];
            let sub = tmp[y.saturating_sub(r) * w + x];
            acc += add - sub;
        }
    }
}

fn lerp_pt(a: ab_glyph::Point, b: ab_glyph::Point, t: f32) -> ab_glyph::Point {
    ab_glyph::point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn glyph_path(font: &FontRef, id: ab_glyph::GlyphId, scale: f32, x: f32, base: f32, warp: &dyn Fn(f32, f32) -> (f32, f32), pb: &mut PathBuilder) {
    let Some(outline) = font.outline(id) else { return };
    let tp = |p: ab_glyph::Point| warp(x + p.x * scale, base - p.y * scale);
    let mut last: Option<(f32, f32)> = None;
    let mut start: Option<(f32, f32)> = None;
    for c in &outline.curves {
        let (p0, rest): (ab_glyph::Point, Vec<ab_glyph::Point>) = match c {
            OutlineCurve::Line(a, b) => (*a, vec![*b]),
            OutlineCurve::Quad(a, b, d) => (*a, vec![*b, *d]),
            OutlineCurve::Cubic(a, b, d, e) => (*a, vec![*b, *d, *e]),
        };
        let s = tp(p0);
        let continues = last.map(|l| (l.0 - s.0).abs() < 0.01 && (l.1 - s.1).abs() < 0.01).unwrap_or(false);
        if !continues {
            if start.is_some() {
                pb.close();
            }
            pb.move_to(s.0, s.1);
            start = Some(s);
        }
        // Curves become short straight pieces first, then each point is
        // moved by the warp (docs/DESIGN.md "Headline warp"), so bent letters
        // keep smooth edges instead of having only their control points
        // moved.
        let pieces = |a: ab_glyph::Point, b: ab_glyph::Point| (((b.x - a.x).hypot(b.y - a.y) * scale / 3.0).ceil() as usize).clamp(1, 24);
        let flat: Vec<ab_glyph::Point> = match rest.len() {
            1 => {
                let n = pieces(p0, rest[0]);
                (1..=n).map(|i| lerp_pt(p0, rest[0], i as f32 / n as f32)).collect()
            }
            2 => {
                let n = pieces(p0, rest[1]).max(4);
                (1..=n)
                    .map(|i| {
                        let t = i as f32 / n as f32;
                        lerp_pt(lerp_pt(p0, rest[0], t), lerp_pt(rest[0], rest[1], t), t)
                    })
                    .collect()
            }
            _ => {
                let n = pieces(p0, rest[2]).max(6);
                (1..=n)
                    .map(|i| {
                        let t = i as f32 / n as f32;
                        let (ab, bc, cd) = (lerp_pt(p0, rest[0], t), lerp_pt(rest[0], rest[1], t), lerp_pt(rest[1], rest[2], t));
                        lerp_pt(lerp_pt(ab, bc, t), lerp_pt(bc, cd, t), t)
                    })
                    .collect()
            }
        };
        for p in &flat {
            let q = tp(*p);
            pb.line_to(q.0, q.1);
        }
        last = flat.last().map(|p| tp(*p));
    }
    if start.is_some() {
        pb.close();
    }
}

/// Words laid out inside a box or a shape. Each line wraps to the room the
/// shape has at that height, and alignment works within that room. Letters
/// can each have their own face, size, color and underline (spans).
struct TextBlock<'a> {
    /// Faces: regular, bold, italic, bold italic.
    faces: [&'a FontRef<'a>; 4],
    /// Faces the font lacks, drawn thickened (bold) or slanted (italic).
    fake: [(bool, bool); 4],
    chars: Vec<char>,
    styles: Vec<CharStyle>,
    look: &'a TextLook,
    w: f32,
    h: f32,
    inset: f32,
    outline: Option<&'a [(f32, f32)]>,
}

struct Glyph {
    face: usize,
    id: ab_glyph::GlyphId,
    x: f32,
    scale: f32,
    word: usize,
    style: usize,
}

struct Placed {
    glyphs: Vec<Glyph>,
    left: f32,
    room: f32,
    width: f32,
    words: usize,
    last: bool,
    top: f32,
    height: f32,
}

fn face_of(s: &CharStyle) -> usize {
    (s.bold as usize) | ((s.italic as usize) << 1)
}

impl TextBlock<'_> {
    /// Room on a line from `top` to `top + lh`: (left, width).
    fn room(&self, top: f32, lh: f32) -> Option<(f32, f32)> {
        match self.outline {
            None => Some((self.inset, self.w - 2.0 * self.inset)),
            Some(pts) => {
                let mut l = f32::MIN;
                let mut r = f32::MAX;
                for f in [0.0, 0.5, 1.0] {
                    let (a, b) = span_at(pts, top + lh * f)?;
                    l = l.max(a);
                    r = r.min(b);
                }
                let pad = self.inset + self.w * 0.04;
                (r - l - 2.0 * pad > lh * 0.8).then_some((l + pad, r - l - 2.0 * pad))
            }
        }
    }

    /// The part of the box (or shape) text sits in. For shapes this is the
    /// band where the shape is at least 55% of its widest, so text stays put
    /// as the shape is resized instead of chasing narrow tips.
    fn vertical_room(&self) -> (f32, f32) {
        match self.outline {
            None => (self.inset, self.h - self.inset),
            Some(pts) => {
                let top = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min);
                let bottom = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max);
                let steps = 60;
                let widths: Vec<(f32, f32)> = (0..=steps)
                    .map(|k| {
                        let y = top + (bottom - top) * k as f32 / steps as f32;
                        (y, span_at(pts, y).map(|(a, b)| b - a).unwrap_or(0.0))
                    })
                    .collect();
                let widest = widths.iter().map(|w| w.1).fold(0.0, f32::max);
                let band: Vec<f32> = widths.iter().filter(|w| w.1 >= widest * 0.55).map(|w| w.0).collect();
                let (a, b) = (band.first().copied().unwrap_or(top), band.last().copied().unwrap_or(bottom));
                (a + self.inset, b - self.inset)
            }
        }
    }

    /// Lay out from `start`; returns lines and whether everything fit.
    fn layout(&self, base_px: f32, start: f32) -> (Vec<Placed>, bool) {
        let base_size = self.look.size.max(0.1);
        let unit = |f: &FontRef| 1.0 / f.height_unscaled();
        let base_face = self.faces[face_of(&CharStyle { bold: self.look.bold, italic: self.look.italic, underline: false, color: self.look.color, size: base_size })];
        let lh_base = (base_face.height_unscaled() + base_face.line_gap_unscaled()) * base_px * unit(base_face);
        let (_, bottom) = self.vertical_room();
        let mut out = Vec::new();
        let mut top = start;
        let mut fits = true;
        // Split into paragraphs and words, remembering each letter's index.
        let mut paras: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new()]];
        for (i, c) in self.chars.iter().enumerate() {
            match c {
                '\n' => paras.push(vec![Vec::new()]),
                ' ' => paras.last_mut().unwrap().push(Vec::new()),
                _ => paras.last_mut().unwrap().last_mut().unwrap().push(i),
            }
        }
        for words in paras {
            // Measure every word.
            let measured: Vec<(Vec<Glyph>, f32, f32)> = words
                .iter()
                .map(|w| {
                    let mut gl = Vec::new();
                    let mut x = 0.0;
                    let mut prev: Option<(usize, ab_glyph::GlyphId)> = None;
                    let mut tallest: f32 = 0.0;
                    for &ci in w {
                        let st = &self.styles[ci];
                        let fi = face_of(st);
                        let f = self.faces[fi];
                        let px = base_px * st.size / base_size;
                        let scale = px * unit(f);
                        let id = f.glyph_id(self.chars[ci]);
                        if let Some((pf, pid)) = prev
                            && pf == fi
                        {
                            x += f.kern_unscaled(pid, id) * scale;
                        }
                        gl.push(Glyph { face: fi, id, x, scale, word: 0, style: ci });
                        x += f.h_advance_unscaled(id) * scale;
                        prev = Some((fi, id));
                        tallest = tallest.max(px);
                    }
                    (gl, x, tallest)
                })
                .collect();
            let space = |px: f32| self.faces[0].h_advance_unscaled(self.faces[0].glyph_id(' ')) * px * unit(self.faces[0]);
            let mut i = 0;
            loop {
                let mut tries = 0;
                let (left, room) = loop {
                    if let Some(r) = self.room(top, lh_base) {
                        break r;
                    }
                    top += lh_base * 0.5;
                    tries += 1;
                    if top > bottom || tries > 200 {
                        fits = false;
                        break (self.inset, self.w - 2.0 * self.inset);
                    }
                };
                let mut line = Placed { glyphs: Vec::new(), left, room, width: 0.0, words: 0, last: false, top, height: lh_base };
                let mut tallest = base_px;
                while i < measured.len() {
                    let (gl, ww, tall) = &measured[i];
                    let at = if line.words == 0 { 0.0 } else { line.width + space(base_px) };
                    if line.words > 0 && at + ww > room {
                        break;
                    }
                    if *ww > room {
                        fits = false;
                    }
                    let word = line.words;
                    line.glyphs.extend(gl.iter().map(|g| Glyph { face: g.face, id: g.id, x: at + g.x, scale: g.scale, word, style: g.style }));
                    line.width = at + ww;
                    line.words += 1;
                    if *tall > 0.0 {
                        tallest = tallest.max(*tall);
                    }
                    i += 1;
                }
                line.height = lh_base * tallest / base_px;
                line.last = i >= measured.len();
                top += line.height;
                if top > bottom + 0.5 {
                    fits = false;
                }
                out.push(line);
                if i >= measured.len() {
                    break;
                }
            }
        }
        (out, fits)
    }

    /// Lay out with the look's vertical alignment inside the room.
    fn arrange(&self, base_px: f32) -> (Vec<Placed>, bool) {
        let (top, bottom) = self.vertical_room();
        let (lines, fits) = self.layout(base_px, top);
        let used: f32 = lines.iter().map(|l| l.height).sum();
        let spare = bottom - top - used;
        if spare <= 0.0 || self.look.valign == VAlign::Top {
            return (lines, fits);
        }
        let shift = if self.look.valign == VAlign::Middle { spare / 2.0 } else { spare };
        self.layout(base_px, top + shift)
    }

    /// Largest base size (pixels) at which every word fits.
    fn fit_size(&self, ppi: f32) -> f32 {
        let (mut lo, mut hi) = (2.0, ppi * 3.0);
        for _ in 0..18 {
            let mid = (lo + hi) / 2.0;
            if self.arrange(mid).1 { lo = mid } else { hi = mid }
        }
        lo
    }

    fn draw(&self, pm: &mut Pixmap, base_px: f32, offset: (f32, f32), tint: f32) {
        if self.w <= 1.0 || base_px <= 0.5 {
            return;
        }
        let base_size = self.look.size.max(0.1);
        let (lines, _) = self.arrange(base_px);
        // One path per color.
        let mut paths: Vec<(Rgba, PathBuilder)> = Vec::new();
        let mut underlines: Vec<(Rgba, f32, f32, f32, f32)> = Vec::new();
        let ident = |x: f32, y: f32| (x, y);
        for line in &lines {
            let extra = (line.room - line.width).max(0.0);
            let (x0, per_gap) = match self.look.align {
                Align::Left => (line.left, 0.0),
                Align::Center => (line.left + extra / 2.0, 0.0),
                Align::Right => (line.left + extra, 0.0),
                Align::Justify if !line.last && line.words > 1 => (line.left, extra / (line.words - 1) as f32),
                Align::Justify => (line.left, 0.0),
            };
            // Baseline from the line's tallest letters.
            let ascent = line.glyphs.iter().map(|g| self.faces[g.face].ascent_unscaled() * g.scale).fold(0.0, f32::max).max(base_px * 0.7);
            let base = offset.1 + line.top + ascent;
            for g in &line.glyphs {
                let st = &self.styles[g.style];
                let x = offset.0 + x0 + g.x + g.word as f32 * per_gap;
                let k = match paths.iter().position(|(c, _)| *c == st.color) {
                    Some(k) => k,
                    None => {
                        paths.push((st.color, PathBuilder::new()));
                        paths.len() - 1
                    }
                };
                let (fake_bold, fake_italic) = self.fake[g.face];
                if fake_italic {
                    let slant = |px: f32, py: f32| (px + (base - py) * 0.2, py);
                    glyph_path(self.faces[g.face], g.id, g.scale, x, base, &slant, &mut paths[k].1);
                } else {
                    glyph_path(self.faces[g.face], g.id, g.scale, x, base, &ident, &mut paths[k].1);
                }
                if fake_bold {
                    // Thicken by drawing the outline once more, nudged right.
                    let px = base_px * st.size / base_size;
                    let nudge = (px * 0.035).max(0.6);
                    let shift = |qx: f32, qy: f32| (qx + nudge + if fake_italic { (base - qy) * 0.2 } else { 0.0 }, qy);
                    glyph_path(self.faces[g.face], g.id, g.scale, x, base, &shift, &mut paths[k].1);
                }
                if st.underline {
                    let adv = self.faces[g.face].h_advance_unscaled(g.id) * g.scale;
                    let px = base_px * st.size / base_size;
                    underlines.push((st.color, x, base + px * 0.1, adv + if g.word + 1 < line.words { per_gap } else { 0.0 }, (px * 0.06).max(1.0)));
                }
            }
        }
        for (c, pb) in paths {
            if let Some(path) = pb.finish() {
                pm.fill_path(&path, &solid(color(c, tint)), FillRule::Winding, Transform::identity(), None);
            }
        }
        for (c, x, y, w, t) in underlines {
            if let Some(r) = tiny_skia::Rect::from_xywh(x, y, w, t) {
                pm.fill_rect(r, &solid(color(c, tint)), Transform::identity(), None);
            }
        }
    }
}

/// Lines laid out at unit scale; the headline is then stretched to its box.
#[allow(clippy::too_many_arguments)]
fn draw_headline(pm: &mut Pixmap, font: &FontRef, text: &str, look: &TextLook, style: (Flair, Warp, bool, f32, f32), second: Rgba, edge: Rgba, tint: f32, area: (f32, f32, f32, f32)) {
    let (flair, warp, stretch, bend, bulge) = style;
    let (ax, ay, aw, ah) = area;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return;
    }
    // Room for stacked/shadow copies.
    let room = match flair {
        Flair::StackRight | Flair::StackLeft | Flair::StackBottom => 0.12,
        Flair::ShadowRight | Flair::ShadowLeft | Flair::SoftShadowRight | Flair::SoftShadowLeft | Flair::Neon | Flair::Blur => 0.08,
        _ => 0.02,
    };
    let bend_room = if warp == Warp::Straight { 0.0 } else { (bend.abs() - 1.0).max(0.0) * 0.06 } + (bulge.abs() * 0.05);
    let room = (room + bend_room).min(0.35);
    let (bx, by, bw, bh) = (ax + aw * room, ay + ah * room, aw * (1.0 - 2.0 * room), ah * (1.0 - 2.0 * room));
    let unit = 1.0 / font.height_unscaled();
    let widths: Vec<f32> = lines
        .iter()
        .map(|l| l.chars().map(|c| font.h_advance_unscaled(font.glyph_id(c)) * unit).sum::<f32>())
        .collect();
    let max_w = widths.iter().cloned().fold(0.01, f32::max);
    let n = lines.len() as f32;
    // Fit: scale so the widest line fills the width and all lines fill the height.
    let sx = bw / max_w;
    let sy = bh / n; // one em per line
    // Normal letters keep their proportions; stretched ones fill the box.
    let (scale_x, scale_y) = if stretch { (sx.min(sy * 3.0), sy * 0.92) } else { let k = sx.min(sy * 0.92); (k, k) };
    // Lines sit one em apart and the block is centered in the box.
    let line_h = scale_y / 0.92;
    let by = by + (bh - n * line_h) / 2.0;
    let mut pb = PathBuilder::new();
    for (i, line) in lines.iter().enumerate() {
        let lw = widths[i] * scale_x;
        let x0 = match look.align {
            Align::Left => bx,
            Align::Right => bx + bw - lw,
            _ => bx + (bw - lw) / 2.0,
        };
        let base = by + line_h * i as f32 + font.ascent_unscaled() * unit * scale_y;
        let warp_fn = |x: f32, y: f32| -> (f32, f32) {
            let t = ((x - bx) / bw).clamp(0.0, 1.0);
            let k = (t * std::f32::consts::PI).sin();
            let (mut x2, mut y2) = (x, y);
            match warp {
                Warp::Straight => {}
                Warp::ArchUp => y2 -= (k * bh * 0.18 - bh * 0.09) * bend,
                Warp::ArchDown => y2 += (k * bh * 0.18 - bh * 0.09) * bend,
                Warp::Wave => y2 += (t * std::f32::consts::TAU).sin() * bh * 0.08 * bend,
                Warp::Bulge => {
                    let mid = by + line_h * (i as f32 + 0.5);
                    y2 = mid + (y - mid) * (1.0 + (0.45 * k - 0.25) * bend);
                }
            }
            if bulge.abs() > 0.001 {
                let mid = by + line_h * (i as f32 + 0.5);
                y2 = mid + (y2 - mid) * (1.0 + (0.45 * k - 0.25) * bulge).max(0.1);
            }
            if flair == Flair::Perspective {
                let mid = by + line_h * (i as f32 + 0.5);
                y2 = mid + (y2 - mid) * (0.55 + 0.6 * t);
                x2 = bx + (x - bx) * 1.0;
            }
            (x2, y2)
        };
        let mut x = 0.0;
        let mut prev = None;
        for ch in line.chars() {
            let id = font.glyph_id(ch);
            if let Some(p) = prev {
                x += font.kern_unscaled(p, id) * unit * scale_x;
            }
            // Non-uniform scale: build with x scale, then squash y.
            let gx = x0 + x;
            if let Some(outline) = font.outline(id) {
                let mut sub = PathBuilder::new();
                let _ = &outline;
                glyph_path(font, id, 1.0, 0.0, 0.0, &|px, py| warp_fn(gx + px * unit * scale_x, base + py * unit * scale_y), &mut sub);
                if let Some(p) = sub.finish() {
                    pb.push_path(&p);
                }
            }
            x += font.h_advance_unscaled(id) * unit * scale_x;
            prev = Some(id);
        }
    }
    let Some(path) = pb.finish() else { return };
    let main = color(look.color, tint);
    let sec = color(second, tint);
    let edge_c = color(edge, tint);
    let fill_with = |pm: &mut Pixmap, paint: &Paint, dx: f32, dy: f32| {
        pm.fill_path(&path, paint, FillRule::Winding, Transform::from_translate(dx, dy), None);
    };
    let unit_px = ah.min(aw) * 0.02;
    match flair {
        Flair::ShadowRight | Flair::ShadowLeft => {
            let d = if flair == Flair::ShadowRight { 1.0 } else { -1.0 };
            fill_with(pm, &solid(edge_c), d * unit_px * 2.0, unit_px * 2.0);
        }
        Flair::SoftShadowRight | Flair::SoftShadowLeft => {
            let d = if flair == Flair::SoftShadowRight { 1.0 } else { -1.0 };
            for k in (1..=6).rev() {
                let mut c = edge_c;
                c.set_alpha(0.12);
                fill_with(pm, &solid(c), d * unit_px * k as f32 * 0.6, unit_px * k as f32 * 0.6);
            }
        }
        Flair::StackRight | Flair::StackLeft | Flair::StackBottom => {
            let (dx, dy) = match flair {
                Flair::StackRight => (1.0, 1.0),
                Flair::StackLeft => (-1.0, 1.0),
                _ => (0.0, 1.0),
            };
            for k in (1..=8).rev() {
                fill_with(pm, &solid(sec), dx * unit_px * k as f32 * 0.6, dy * unit_px * k as f32 * 0.6);
            }
        }
        Flair::Neon => {
            for (wdt, a) in [(unit_px * 3.0, 0.15), (unit_px * 2.0, 0.25), (unit_px, 0.5)] {
                let mut c = sec;
                c.set_alpha(a);
                pm.stroke_path(&path, &solid(c), &Stroke { width: wdt, line_join: tiny_skia::LineJoin::Round, ..Default::default() }, Transform::identity(), None);
            }
        }
        Flair::Blur => {
            for k in [-2.0, -1.0, 1.0, 2.0] {
                let mut c = main;
                c.set_alpha(0.18);
                fill_with(pm, &solid(c), k * unit_px * 0.7, 0.0);
                fill_with(pm, &solid(c), 0.0, k * unit_px * 0.7);
            }
        }
        _ => {}
    }
    let face: Paint = match flair {
        Flair::BlendDown | Flair::BlendRight | Flair::BlendLeft => {
            let (s, e) = match flair {
                Flair::BlendDown => (Point::from_xy(0.0, by), Point::from_xy(0.0, by + bh)),
                Flair::BlendRight => (Point::from_xy(bx, 0.0), Point::from_xy(bx + bw, 0.0)),
                _ => (Point::from_xy(bx + bw, 0.0), Point::from_xy(bx, 0.0)),
            };
            let sh = LinearGradient::new(s, e, vec![GradientStop::new(0.0, main), GradientStop::new(1.0, sec)], SpreadMode::Pad, Transform::identity());
            Paint { shader: sh.unwrap_or(Shader::SolidColor(main)), anti_alias: true, ..Default::default() }
        }
        Flair::RadialBlend => {
            let c = Point::from_xy(bx + bw / 2.0, by + bh / 2.0);
            let sh = RadialGradient::new(c, c, bw.max(bh) / 2.0, vec![GradientStop::new(0.0, main), GradientStop::new(1.0, sec)], SpreadMode::Pad, Transform::identity());
            Paint { shader: sh.unwrap_or(Shader::SolidColor(main)), anti_alias: true, ..Default::default() }
        }
        Flair::Ghost => {
            let mut c = main;
            c.set_alpha(0.25);
            solid(c)
        }
        Flair::Outline => solid(sec),
        Flair::Neon => solid(Color::from_rgba8(255, 255, 255, 255)),
        _ => solid(main),
    };
    fill_with(pm, &face, 0.0, 0.0);
    let outline_w = match flair {
        Flair::Outline | Flair::Ghost => unit_px * 0.6,
        _ if !edge.is_clear() && matches!(flair, Flair::BlendDown | Flair::BlendRight | Flair::BlendLeft | Flair::RadialBlend | Flair::Plain | Flair::StackBottom | Flair::StackLeft | Flair::StackRight) => unit_px * 0.25,
        _ => 0.0,
    };
    if outline_w > 0.0 {
        // One outline around all the letters together: stroked on its own
        // layer, then everything inside the letters cleared, so joined or
        // overlapping letters (script fonts) don't show lines where they meet.
        let c = if matches!(flair, Flair::Outline | Flair::Ghost) { main } else { edge_c };
        if let Some(mut layer) = Pixmap::new(pm.width(), pm.height()) {
            layer.stroke_path(&path, &solid(c), &Stroke { width: outline_w * 2.0, line_join: tiny_skia::LineJoin::Round, ..Default::default() }, Transform::identity(), None);
            let mut clear = solid(c);
            clear.blend_mode = tiny_skia::BlendMode::Clear;
            layer.fill_path(&path, &clear, FillRule::Winding, Transform::identity(), None);
            pm.draw_pixmap(0, 0, layer.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        }
    }
}

pub const BORDER_NAMES: [&str; 16] = [
    "Double Line", "Dots", "Pennants", "Rounded Double", "Hearts", "Stars", "Scallops", "Waves",
    "Checkerboard", "Dashes", "Rope", "Leaves", "Art Deco Corners", "Confetti", "Diamonds", "Thick and Thin",
];

/// Decorative page borders. Colors come from the item's line and fill
/// colors; thickness from its line weight (points).
fn draw_border(pm: &mut Pixmap, style: u8, item: &Item, area: (f32, f32, f32, f32), ppi: f32) {
    let (x, y, w, h) = area;
    let c = color(if item.stroke.is_clear() { Rgba::hex("#2f6fb0") } else { item.stroke }, item.tint);
    let c2 = color(if item.fill.is_clear() { Rgba::hex("#f2b631") } else { item.fill }, item.tint);
    let band = if item.stroke_pt > 3.0 { item.stroke_pt / 72.0 * ppi } else { w.min(h) * 0.05 };
    let m = band * 0.7;
    let rect = |inset: f32| tiny_skia::Rect::from_xywh(x + inset, y + inset, w - 2.0 * inset, h - 2.0 * inset).map(PathBuilder::from_rect);
    let strokes = |pm: &mut Pixmap, p: &tiny_skia::Path, col: Color, wd: f32, dash: Option<tiny_skia::StrokeDash>| {
        let st = Stroke { width: wd, dash, line_join: tiny_skia::LineJoin::Round, line_cap: tiny_skia::LineCap::Round, ..Default::default() };
        pm.stroke_path(p, &solid(col), &st, Transform::identity(), None);
    };
    // Centers of motifs spaced evenly around the band.
    let spots = |step: f32| -> Vec<(f32, f32, usize)> {
        let mut v = Vec::new();
        let (l, t, r, b) = (x + band / 2.0, y + band / 2.0, x + w - band / 2.0, y + h - band / 2.0);
        let nx = ((r - l) / step).round().max(1.0) as usize;
        let ny = ((b - t) / step).round().max(1.0) as usize;
        for i in 0..=nx {
            let px = l + (r - l) * i as f32 / nx as f32;
            v.push((px, t, i));
            v.push((px, b, i + 1));
        }
        for j in 1..ny {
            let py = t + (b - t) * j as f32 / ny as f32;
            v.push((l, py, j));
            v.push((r, py, j + 1));
        }
        v
    };
    let motif = |pm: &mut Pixmap, o: Outline, size: f32, step: f32, alternate: bool| {
        let pts = outline_points(o, size, size);
        let mut pb = PathBuilder::new();
        let mut pb2 = PathBuilder::new();
        for (cx, cy, k) in spots(step) {
            if let Some(p) = poly_path(&pts, cx - size / 2.0, cy - size / 2.0) {
                if alternate && k % 2 == 1 { pb2.push_path(&p) } else { pb.push_path(&p) }
            }
        }
        if let Some(p) = pb.finish() {
            pm.fill_path(&p, &solid(c2), FillRule::Winding, Transform::identity(), None);
            strokes(pm, &p, c, (band * 0.06).max(1.0), None);
        }
        if let Some(p) = pb2.finish() {
            pm.fill_path(&p, &solid(c), FillRule::Winding, Transform::identity(), None);
        }
    };
    match style % 16 {
        0 => {
            if let Some(p) = rect(m * 0.4) {
                strokes(pm, &p, c, m * 0.45, None);
            }
            if let Some(p) = rect(m * 1.3) {
                strokes(pm, &p, c, m * 0.15, None);
            }
        }
        1 => {
            let mut pb = PathBuilder::new();
            for (cx, cy, _) in spots(band * 1.2) {
                pb.push_circle(cx, cy, band * 0.32);
            }
            if let Some(p) = pb.finish() {
                pm.fill_path(&p, &solid(c), FillRule::Winding, Transform::identity(), None);
            }
        }
        2 => motif(pm, Outline::Triangle, band, band * 1.05, true),
        3 => {
            let pts = outline_points(Outline::Rounded, w - 2.0 * m, h - 2.0 * m);
            if let Some(p) = poly_path(&pts, x + m, y + m) {
                strokes(pm, &p, c2, m * 1.1, None);
                strokes(pm, &p, c, m * 0.22, None);
            }
        }
        4 => motif(pm, Outline::Heart, band * 0.9, band * 1.25, false),
        5 => motif(pm, Outline::Star, band, band * 1.2, false),
        6 => {
            let mut pb = PathBuilder::new();
            for (cx, cy, _) in spots(band * 0.9) {
                pb.push_circle(cx, cy, band * 0.5);
            }
            if let Some(p) = pb.finish() {
                pm.fill_path(&p, &solid(c2), FillRule::Winding, Transform::identity(), None);
            }
            if let Some(p) = rect(band * 0.5) {
                strokes(pm, &p, c, (band * 0.1).max(1.0), None);
            }
        }
        7 => {
            let mut pb = PathBuilder::new();
            let amp = band * 0.3;
            let edges = [((x + m, y + m), (x + w - m, y + m)), ((x + w - m, y + m), (x + w - m, y + h - m)), ((x + w - m, y + h - m), (x + m, y + h - m)), ((x + m, y + h - m), (x + m, y + m))];
            for ((ax, ay), (bx, by)) in edges {
                let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
                let n = (len / (band * 0.2)).max(4.0) as usize;
                let (nx, ny) = (-(by - ay) / len, (bx - ax) / len);
                for i in 0..=n {
                    let t = i as f32 / n as f32;
                    let wv = (t * len / band * std::f32::consts::PI).sin() * amp;
                    let (px, py) = (ax + (bx - ax) * t + nx * wv, ay + (by - ay) * t + ny * wv);
                    if i == 0 { pb.move_to(px, py) } else { pb.line_to(px, py) }
                }
            }
            if let Some(p) = pb.finish() {
                strokes(pm, &p, c, band * 0.18, None);
            }
        }
        8 => {
            let s2 = band / 2.0;
            let mut a = PathBuilder::new();
            let mut cols = 0;
            let mut cx = x;
            while cx < x + w - 0.5 {
                for (row, cy) in [(0, y), (1, y + s2), (0, y + h - band), (1, y + h - s2)] {
                    if (cols + row) % 2 == 0
                        && let Some(r) = tiny_skia::Rect::from_xywh(cx, cy, s2.min(x + w - cx), s2)
                    {
                        a.push_rect(r);
                    }
                }
                cx += s2;
                cols += 1;
            }
            let mut rows = 0;
            let mut cy = y + band;
            while cy < y + h - band - 0.5 {
                for (col, cx) in [(0, x), (1, x + s2), (0, x + w - band), (1, x + w - s2)] {
                    if (rows + col) % 2 == 0
                        && let Some(r) = tiny_skia::Rect::from_xywh(cx, cy, s2, s2.min(y + h - band - cy))
                    {
                        a.push_rect(r);
                    }
                }
                cy += s2;
                rows += 1;
            }
            if let Some(p) = a.finish() {
                pm.fill_path(&p, &solid(c), FillRule::Winding, Transform::identity(), None);
            }
        }
        9 => {
            if let Some(p) = rect(m * 0.6) {
                strokes(pm, &p, c, m * 0.35, tiny_skia::StrokeDash::new(vec![band * 0.8, band * 0.5], 0.0));
            }
        }
        10 => {
            if let Some(p) = rect(band / 2.0) {
                strokes(pm, &p, c2, band * 0.7, None);
                strokes(pm, &p, c, band * 0.7, tiny_skia::StrokeDash::new(vec![band * 0.25, band * 0.35], 0.0));
            }
        }
        11 => {
            if let Some(p) = rect(band / 2.0) {
                strokes(pm, &p, c, (band * 0.08).max(1.0), None);
            }
            motif(pm, Outline::Teardrop, band * 0.75, band * 0.9, false);
        }
        12 => {
            if let Some(p) = rect(m * 0.3) {
                strokes(pm, &p, c, m * 0.12, None);
            }
            let k = band * 2.5;
            let mut pb = PathBuilder::new();
            for (cx, cy, sx, sy) in [(x, y, 1.0, 1.0), (x + w, y, -1.0, 1.0), (x, y + h, 1.0, -1.0), (x + w, y + h, -1.0, -1.0)] {
                for f in [0.45, 0.7, 0.95] {
                    pb.move_to(cx + sx * m, cy + sy * (m + k * f));
                    pb.line_to(cx + sx * (m + k * f), cy + sy * (m + k * f));
                    pb.line_to(cx + sx * (m + k * f), cy + sy * m);
                }
            }
            if let Some(p) = pb.finish() {
                strokes(pm, &p, c2, m * 0.2, None);
            }
        }
        13 => {
            let palette = [c, c2, Color::from_rgba8(235, 106, 92, 255), Color::from_rgba8(76, 182, 140, 255)];
            for (i, (cx, cy, _)) in spots(band * 0.7).into_iter().enumerate() {
                let r = band * (0.12 + 0.08 * ((i * 7) % 3) as f32);
                let (jx, jy) = (((i * 37) % 11) as f32 / 11.0 - 0.5, ((i * 53) % 13) as f32 / 13.0 - 0.5);
                let mut pb = PathBuilder::new();
                if i % 3 == 0 {
                    let pts = outline_points(Outline::Triangle, r * 2.0, r * 2.0);
                    if let Some(p) = poly_path(&pts, cx + jx * band - r, cy + jy * band - r) {
                        pb.push_path(&p);
                    }
                } else {
                    pb.push_circle(cx + jx * band, cy + jy * band, r);
                }
                if let Some(p) = pb.finish() {
                    pm.fill_path(&p, &solid(palette[i % 4]), FillRule::Winding, Transform::identity(), None);
                }
            }
        }
        14 => motif(pm, Outline::Diamond, band * 0.9, band * 0.95, true),
        _ => {
            if let Some(p) = rect(m * 0.5) {
                strokes(pm, &p, c, m * 0.8, None);
            }
            if let Some(p) = rect(m * 1.5) {
                strokes(pm, &p, c2, m * 0.15, None);
            }
        }
    }
}

/// Draw one item onto `dst` with its position, turn, flip and opacity.
pub fn compose(dst: &mut Pixmap, item: &Item, project: &Project, fonts: &FontBook, pics: &PictureCache, ppi: f32, offset: (f32, f32)) {
    let Some(r) = render_item(item, project, fonts, pics, ppi) else { return };
    let (cx, cy) = item.frame.center();
    let (cx, cy) = (cx * ppi + offset.0, cy * ppi + offset.1);
    let ox = item.frame.x * ppi - r.pad + offset.0;
    let oy = item.frame.y * ppi - r.pad + offset.1;
    let fx = if item.flip_x { -1.0 } else { 1.0 };
    let fy = if item.flip_y { -1.0 } else { 1.0 };
    let t = Transform::from_translate(ox, oy).post_translate(-cx, -cy).post_scale(fx, fy).post_rotate(item.rotation).post_translate(cx, cy);
    let paint = PixmapPaint { quality: tiny_skia::FilterQuality::Bilinear, opacity: item.opacity.clamp(0.0, 1.0), ..Default::default() };
    dst.draw_pixmap(0, 0, r.pixmap.as_ref(), &paint, t, None);
}

/// A whole panel at `ppi`, white paper underneath.
pub fn render_panel(project: &Project, panel: usize, fonts: &FontBook, pics: &PictureCache, ppi: f32) -> Option<Pixmap> {
    let pw = (project.width * ppi).round() as u32;
    let ph = (project.height * ppi).round() as u32;
    let mut page = Pixmap::new(pw.max(1), ph.max(1))?;
    let p = project.panels.get(panel)?;
    page.fill(color(p.background, 1.0));
    draw_pattern(&mut page, p.pattern, p.pattern_color, ppi * p.pattern_scale.clamp(0.25, 4.0));
    for item in &p.items {
        compose(&mut page, item, project, fonts, pics, ppi, (0.0, 0.0));
    }
    Some(page)
}

/// A line of text drawn in a font, as straight RGBA (for font previews).
pub fn text_image(bytes: &[u8], text: &str, px: f32, ink: [u8; 3]) -> Option<(usize, usize, Vec<u8>)> {
    let font = FontRef::try_from_slice(bytes).ok()?;
    let scale = px / font.height_unscaled();
    let width: f32 = text.chars().map(|c| font.h_advance_unscaled(font.glyph_id(c)) * scale).sum::<f32>() + 4.0;
    let h = (font.height_unscaled() * scale).ceil() + 4.0;
    let mut pm = Pixmap::new(width.ceil().max(1.0) as u32, h as u32)?;
    let mut pb = PathBuilder::new();
    let mut x = 2.0;
    let ident = |x: f32, y: f32| (x, y);
    for c in text.chars() {
        let id = font.glyph_id(c);
        glyph_path(&font, id, scale, x, 2.0 + font.ascent_unscaled() * scale, &ident, &mut pb);
        x += font.h_advance_unscaled(id) * scale;
    }
    let path = pb.finish()?;
    pm.fill_path(&path, &solid(Color::from_rgba8(ink[0], ink[1], ink[2], 255)), FillRule::Winding, Transform::identity(), None);
    Some((pm.width() as usize, pm.height() as usize, pixmap_to_rgba(&pm)))
}

pub const PATTERN_NAMES: [&str; 12] = [
    "Plain", "Grid", "Graph Paper", "Lined Paper", "Dots", "Diagonal Stripes", "Polka Dots", "Checkerboard", "Gingham", "Linen", "Kraft Paper", "Confetti",
];

/// Page background patterns, drawn over the page color.
pub fn draw_pattern(pm: &mut Pixmap, pattern: u8, ink: Rgba, ppi: f32) {
    if pattern == 0 {
        return;
    }
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let c = color(ink, 1.0);
    let faint = { let mut f = c; f.set_alpha(0.35); f };
    let thin = Stroke { width: (ppi / 96.0).max(0.6), ..Default::default() };
    let lines = |pm: &mut Pixmap, step: f32, vertical: bool, horizontal: bool, paint: Color, st: &Stroke| {
        let mut pb = PathBuilder::new();
        if vertical {
            let mut x = step;
            while x < w {
                pb.move_to(x, 0.0);
                pb.line_to(x, h);
                x += step;
            }
        }
        if horizontal {
            let mut y = step;
            while y < h {
                pb.move_to(0.0, y);
                pb.line_to(w, y);
                y += step;
            }
        }
        if let Some(p) = pb.finish() {
            pm.stroke_path(&p, &solid(paint), st, Transform::identity(), None);
        }
    };
    let dots = |pm: &mut Pixmap, step: f32, r: f32, offset_rows: bool, paint: Color| {
        let mut pb = PathBuilder::new();
        let mut y = step / 2.0;
        let mut row = 0;
        while y < h + step {
            let mut x = if offset_rows && row % 2 == 1 { step } else { step / 2.0 };
            while x < w + step {
                pb.push_circle(x, y, r);
                x += step;
            }
            y += step;
            row += 1;
        }
        if let Some(p) = pb.finish() {
            pm.fill_path(&p, &solid(paint), FillRule::Winding, Transform::identity(), None);
        }
    };
    // Repeatable "random" numbers so prints match the screen. A full
    // integer mix, so neighbouring numbers don't line up into streaks.
    let rnd = |i: u32| {
        let mut x = i.wrapping_mul(0x9E37_79B9).wrapping_add(0x7F4A_7C15);
        x ^= x >> 16;
        x = x.wrapping_mul(0x85EB_CA6B);
        x ^= x >> 13;
        x = x.wrapping_mul(0xC2B2_AE35);
        x ^= x >> 16;
        (x >> 8) as f32 / 16_777_216.0
    };
    let fill_all = |pm: &mut Pixmap, paint: Color| {
        if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, w, h) {
            pm.fill_rect(r, &solid(paint), Transform::identity(), None);
        }
    };
    match pattern {
        1 => lines(pm, ppi * 0.5, true, true, faint, &thin),
        2 => {
            lines(pm, ppi * 0.125, true, true, { let mut f = c; f.set_alpha(0.18); f }, &thin);
            lines(pm, ppi * 0.5, true, true, faint, &Stroke { width: thin.width * 1.6, ..Default::default() });
        }
        3 => {
            lines(pm, ppi * 0.34, false, true, faint, &thin);
            let mut pb = PathBuilder::new();
            pb.move_to(ppi * 1.25, 0.0);
            pb.line_to(ppi * 1.25, h);
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &solid(Color::from_rgba8(220, 80, 90, 140)), &thin, Transform::identity(), None);
            }
        }
        4 => dots(pm, ppi * 0.25, (ppi * 0.012).max(0.8), false, faint),
        5 => {
            let mut pb = PathBuilder::new();
            let step = ppi * 0.3;
            let mut x = -h;
            while x < w {
                pb.move_to(x, h);
                pb.line_to(x + h, 0.0);
                x += step;
            }
            if let Some(p) = pb.finish() {
                pm.stroke_path(&p, &solid(faint), &Stroke { width: step * 0.35, ..Default::default() }, Transform::identity(), None);
            }
        }
        6 => dots(pm, ppi * 0.5, ppi * 0.09, true, faint),
        7 | 8 => {
            let s = if pattern == 7 { ppi * 0.5 } else { ppi * 0.25 };
            let a = if pattern == 7 { 0.25 } else { 0.22 };
            let mut pb = PathBuilder::new();
            let (mut y, mut row) = (0.0, 0);
            while y < h {
                let (mut x, mut col) = (0.0, 0);
                while x < w {
                    if pattern == 8 || (row + col) % 2 == 0 {
                        if let Some(r) = tiny_skia::Rect::from_xywh(x, y, s, s) {
                            if pattern == 7 || col % 2 == 0 || row % 2 == 0 {
                                pb.push_rect(r);
                            }
                        }
                    }
                    x += s;
                    col += 1;
                }
                y += s;
                row += 1;
            }
            if let Some(p) = pb.finish() {
                let mut f = c;
                f.set_alpha(a);
                pm.fill_path(&p, &solid(f), FillRule::Winding, Transform::identity(), None);
            }
        }
        9 => {
            // Linen: a fine woven look. Threads run both ways about 40 to
            // the inch; each thread is a little lighter or darker than the
            // next, with a few thicker slubs, in the pattern color.
            let step = (ppi / 40.0).max(1.5);
            let weave = Stroke { width: (step * 0.45).max(0.5), ..Default::default() };
            for (dir, seed) in [(0u8, 0u32), (1, 50_000)] {
                let (len, across) = if dir == 0 { (w, h) } else { (h, w) };
                // Group thread pieces by strength so each group is one path.
                // Threads sit a little unevenly and change strength along
                // their length, so the cloth never looks like graph paper.
                let mut groups: [PathBuilder; 4] = Default::default();
                let (mut pos, mut k) = (step * 0.5, 0u32);
                while pos < across {
                    let mut at = 0.0;
                    let mut j = 0u32;
                    while at < len {
                        let i = seed + k * 64 + j;
                        let piece = ppi * (0.15 + 0.5 * rnd(i * 2));
                        let g = (rnd(i * 2 + 1) * 4.0) as usize % 4;
                        let end = (at + piece).min(len);
                        if dir == 0 {
                            groups[g].move_to(at, pos);
                            groups[g].line_to(end, pos);
                        } else {
                            groups[g].move_to(pos, at);
                            groups[g].line_to(pos, end);
                        }
                        at = end;
                        j += 1;
                    }
                    pos += step * (0.7 + 0.6 * rnd(seed + 900_000 + k));
                    k += 1;
                }
                for (g, pb) in groups.into_iter().enumerate() {
                    if let Some(p) = pb.finish() {
                        let mut f = c;
                        f.set_alpha(0.07 + 0.045 * g as f32);
                        pm.stroke_path(&p, &solid(f), &weave, Transform::identity(), None);
                    }
                }
            }
            // Slubs: short thicker bits of thread.
            let n = ((w * h) / (ppi * ppi) * 25.0) as u32;
            let mut pb = PathBuilder::new();
            for i in 0..n.min(20_000) {
                let (x, y) = (rnd(i * 4 + 100_000) * w, rnd(i * 4 + 100_001) * h);
                let l = ppi * (0.04 + 0.08 * rnd(i * 4 + 100_002));
                pb.move_to(x, y);
                if i % 2 == 0 { pb.line_to(x + l, y) } else { pb.line_to(x, y + l) }
            }
            if let Some(p) = pb.finish() {
                let mut f = c;
                f.set_alpha(0.08);
                pm.stroke_path(&p, &solid(f), &Stroke { width: weave.width * 1.2, line_cap: tiny_skia::LineCap::Round, ..Default::default() }, Transform::identity(), None);
            }
        }
        10 => {
            // Kraft paper: a brown sheet with darker and lighter fibers
            // lying every which way. The page color tints the brown.
            fill_all(pm, Color::from_rgba8(176, 132, 86, 215));
            let n = ((w * h) / (ppi * ppi) * 220.0) as u32;
            let mut dark = PathBuilder::new();
            let mut light = PathBuilder::new();
            for i in 0..n.min(60_000) {
                let (x, y) = (rnd(i * 4 + 7) * w, rnd(i * 4 + 8) * h);
                let l = ppi * (0.02 + 0.07 * rnd(i * 4 + 9));
                let a = rnd(i * 4 + 10) * std::f32::consts::PI;
                let pb = if i % 3 == 0 { &mut light } else { &mut dark };
                pb.move_to(x, y);
                pb.line_to(x + l * a.cos(), y + l * a.sin());
            }
            let fiber = Stroke { width: (ppi / 150.0).max(0.5), ..Default::default() };
            if let Some(p) = dark.finish() {
                pm.stroke_path(&p, &solid(Color::from_rgba8(110, 76, 44, 70)), &fiber, Transform::identity(), None);
            }
            if let Some(p) = light.finish() {
                pm.stroke_path(&p, &solid(Color::from_rgba8(230, 200, 160, 70)), &fiber, Transform::identity(), None);
            }
            // A few darker specks, like recycled paper.
            dots_random(pm, w, h, ppi, 3.0, &rnd, Color::from_rgba8(90, 60, 35, 110));
        }
        _ => {
            // Confetti: bright dots and strips spread evenly (one piece per
            // cell of a jittered grid, so they never clump into lines).
            let palette = [
                Color::from_rgba8(235, 106, 92, 235),
                Color::from_rgba8(47, 143, 216, 235),
                Color::from_rgba8(242, 182, 49, 240),
                Color::from_rgba8(76, 182, 140, 235),
                Color::from_rgba8(160, 110, 220, 235),
                c,
            ];
            let cell = ppi * 0.42;
            let (cols, rows) = ((w / cell).ceil() as u32 + 1, (h / cell).ceil() as u32 + 1);
            let mut pieces: Vec<PathBuilder> = (0..palette.len()).map(|_| PathBuilder::new()).collect();
            for r in 0..rows {
                for q in 0..cols {
                    let i = r * 4096 + q;
                    let x = (q as f32 + rnd(i * 6)) * cell;
                    let y = (r as f32 + rnd(i * 6 + 1)) * cell;
                    let size = ppi * (0.035 + 0.035 * rnd(i * 6 + 2));
                    let pb = &mut pieces[(rnd(i * 6 + 3) * palette.len() as f32) as usize % palette.len()];
                    if rnd(i * 6 + 4) < 0.45 {
                        // A little strip, turned any way.
                        let a = rnd(i * 6 + 5) * std::f32::consts::PI;
                        let (dx, dy) = (a.cos() * size * 1.4, a.sin() * size * 1.4);
                        let (nx, ny) = (-a.sin() * size * 0.45, a.cos() * size * 0.45);
                        pb.move_to(x - dx - nx, y - dy - ny);
                        pb.line_to(x + dx - nx, y + dy - ny);
                        pb.line_to(x + dx + nx, y + dy + ny);
                        pb.line_to(x - dx + nx, y - dy + ny);
                        pb.close();
                    } else {
                        pb.push_circle(x, y, size * 0.8);
                    }
                }
            }
            for (k, pb) in pieces.into_iter().enumerate() {
                if let Some(p) = pb.finish() {
                    pm.fill_path(&p, &solid(palette[k]), FillRule::Winding, Transform::identity(), None);
                }
            }
        }
    }
}

/// Small round specks scattered over the page (`per_sq_in` of them per
/// square inch).
fn dots_random(pm: &mut Pixmap, w: f32, h: f32, ppi: f32, per_sq_in: f32, rnd: &dyn Fn(u32) -> f32, paint: Color) {
    let n = ((w * h) / (ppi * ppi) * per_sq_in) as u32;
    let mut pb = PathBuilder::new();
    for i in 0..n.min(5000) {
        let (x, y) = (rnd(i * 3 + 200_000) * w, rnd(i * 3 + 200_001) * h);
        pb.push_circle(x, y, (ppi * (0.006 + 0.01 * rnd(i * 3 + 200_002))).max(0.6));
    }
    if let Some(p) = pb.finish() {
        pm.fill_path(&p, &solid(paint), FillRule::Winding, Transform::identity(), None);
    }
}

/// A page's background (color and pattern) as a picture, for the screen.
pub fn background_image(panel: &Panel, w_in: f32, h_in: f32, ppi: f32) -> Option<Pixmap> {
    let mut pm = Pixmap::new((w_in * ppi).round().max(1.0) as u32, (h_in * ppi).round().max(1.0) as u32)?;
    pm.fill(color(panel.background, 1.0));
    draw_pattern(&mut pm, panel.pattern, panel.pattern_color, ppi * panel.pattern_scale.clamp(0.25, 4.0));
    Some(pm)
}

// ------------------------------------------------------------ line styles

/// How lines and outlines can be drawn (`Item::line_style`).
pub const LINE_STYLES: [&str; 9] = ["Solid", "Dashed", "Long dashes", "Dotted", "Dash and dot", "Double", "Wavy", "Zigzag", "Scribble"];

/// Extra room (in line widths) a style needs past the line itself.
pub fn line_style_reach(style: u8) -> f32 {
    match style {
        5 => 1.5,
        6..=8 => 3.0,
        _ => 1.0,
    }
}

fn polyline(pts: &[(f32, f32)], closed: bool) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let (x0, y0) = *pts.first()?;
    pb.move_to(x0, y0);
    for (x, y) in &pts[1..] {
        pb.line_to(*x, *y);
    }
    if closed {
        pb.close();
    }
    pb.finish()
}

/// Points every `step` along the outline (corners kept), with the distance
/// travelled and the direction there.
fn walk(pts: &[(f32, f32)], closed: bool, step: f32) -> Vec<((f32, f32), f32, (f32, f32))> {
    let mut seq: Vec<(f32, f32)> = pts.to_vec();
    if closed && let Some(f) = pts.first() {
        seq.push(*f);
    }
    let mut out = Vec::new();
    let mut dist = 0.0;
    for win in seq.windows(2) {
        let (a, b) = (win[0], win[1]);
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        if len < 1e-3 {
            continue;
        }
        let dir = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let n = (len / step).ceil().max(1.0) as usize;
        for k in 0..n {
            let t = k as f32 / n as f32;
            out.push(((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), dist + len * t, dir));
        }
        dist += len;
    }
    if !closed && let Some(l) = seq.last() {
        let dir = out.last().map(|o| o.2).unwrap_or((1.0, 0.0));
        out.push((*l, dist, dir));
    }
    out
}

/// Stroke an outline (`closed`) or a line in one of the LINE_STYLES.
/// `width` is in pixels; 0 draws nothing.
pub fn styled_stroke(pm: &mut Pixmap, pts: &[(f32, f32)], closed: bool, style: u8, width: f32, c: Color) {
    if width <= 0.0 || pts.len() < 2 {
        return;
    }
    let paint = solid(c);
    let cap = if closed { tiny_skia::LineCap::Butt } else { tiny_skia::LineCap::Round };
    let mut st = Stroke { width, line_join: tiny_skia::LineJoin::Round, line_cap: cap, ..Default::default() };
    let w = width.max(1.0);
    match style {
        1..=4 => {
            let dash = match style {
                1 => vec![w * 3.0, w * 2.0],
                2 => vec![w * 7.0, w * 3.0],
                3 => vec![0.01, w * 2.2],
                _ => vec![w * 5.0, w * 2.0, 0.01, w * 2.0],
            };
            if style >= 3 {
                st.line_cap = tiny_skia::LineCap::Round;
            }
            st.dash = tiny_skia::StrokeDash::new(dash, 0.0);
            if let Some(p) = polyline(pts, closed) {
                pm.stroke_path(&p, &paint, &st, Transform::identity(), None);
            }
        }
        5 => {
            // Two lines with a gap between: a wide stroke with its middle cleared.
            if let (Some(p), Some(mut layer)) = (polyline(pts, closed), Pixmap::new(pm.width(), pm.height())) {
                st.width = width * 1.6;
                layer.stroke_path(&p, &paint, &st, Transform::identity(), None);
                let mut clear = solid(c);
                clear.blend_mode = tiny_skia::BlendMode::Clear;
                st.width = width * 0.55;
                layer.stroke_path(&p, &clear, &st, Transform::identity(), None);
                pm.draw_pixmap(0, 0, layer.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            }
        }
        6..=8 => {
            let amp = (w * 1.4).max(2.0);
            let period = (w * 7.0).max(12.0);
            let along = walk(pts, closed, (period / 10.0).max(1.0));
            let wave = |d: f32| match style {
                6 => (d / period * std::f32::consts::TAU).sin(),
                7 => {
                    let t = (d / period).fract();
                    if t < 0.5 { 4.0 * t - 1.0 } else { 3.0 - 4.0 * t }
                }
                _ => 0.0,
            };
            let passes: &[u32] = if style == 8 { &[1, 2] } else { &[0] };
            for &seed in passes {
                let moved: Vec<(f32, f32)> = along
                    .iter()
                    .enumerate()
                    .map(|(i, ((x, y), d, (dx, dy)))| {
                        // Scribble: a steady hand that wobbles a little, twice over.
                        let off = if style == 8 {
                            let r = ((i as u32).wrapping_mul(2654435761).wrapping_add(seed * 40503) >> 16) as f32 / 65535.0;
                            (r - 0.5) * amp * 0.9 + (d / period * 2.1 + seed as f32).sin() * amp * 0.4
                        } else {
                            wave(*d) * amp
                        };
                        (x - dy * off, y + dx * off)
                    })
                    .collect();
                st.width = if style == 8 { width * 0.7 } else { width };
                st.line_cap = tiny_skia::LineCap::Round;
                if let Some(p) = polyline(&moved, closed) {
                    pm.stroke_path(&p, &paint, &st, Transform::identity(), None);
                }
            }
        }
        _ => {
            if let Some(p) = polyline(pts, closed) {
                pm.stroke_path(&p, &paint, &st, Transform::identity(), None);
            }
        }
    }
}

// --------------------------------------------------------------- ornaments

/// Decorative dividers and flourishes (`Content::Ornament`).
pub const ORNAMENT_NAMES: [&str; 12] = [
    "Swirl Flourish",
    "Diamond Divider",
    "Dotted Divider",
    "Hearts Divider",
    "Stars Divider",
    "Leafy Vine",
    "Scallop Underline",
    "Brush Underline",
    "Double Swoosh",
    "Laurel",
    "Arrow Flourish",
    "Ribbon Banner",
];

/// A leaf from `a` to `b`, plump by `k` of its length.
fn leaf(pb: &mut PathBuilder, a: (f32, f32), b: (f32, f32), k: f32) {
    let (mx, my) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let (nx, ny) = (-(b.1 - a.1) * k, (b.0 - a.0) * k);
    pb.move_to(a.0, a.1);
    pb.quad_to(mx + nx, my + ny, b.0, b.1);
    pb.quad_to(mx - nx, my - ny, a.0, a.1);
    pb.close();
}

fn shape_at(pb: &mut PathBuilder, o: Outline, cx: f32, cy: f32, s: f32) {
    let pts = outline_points(o, s, s);
    if let Some((x, y)) = pts.first() {
        pb.move_to(cx - s / 2.0 + x, cy - s / 2.0 + y);
    }
    for (x, y) in &pts[1..] {
        pb.line_to(cx - s / 2.0 + x, cy - s / 2.0 + y);
    }
    pb.close();
}

/// Draw an ornament in the box (x, y, w, h) in pixels.
pub fn draw_ornament(pm: &mut Pixmap, style: u8, item: &Item, area: (f32, f32, f32, f32), ppi: f32) {
    let (x, y, w, h) = area;
    let ink = color(if item.stroke.is_clear() { Rgba::hex("#333333") } else { item.stroke }, item.tint);
    let accent = color(if item.fill.is_clear() { if item.stroke.is_clear() { Rgba::hex("#333333") } else { item.stroke } } else { item.fill }, item.tint);
    let lw = (item.stroke_pt / 72.0 * ppi).max(1.0);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let line = Stroke { width: lw, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
    let stroke = |pm: &mut Pixmap, pb: PathBuilder, st: &Stroke| {
        if let Some(p) = pb.finish() {
            pm.stroke_path(&p, &solid(ink), st, Transform::identity(), None);
        }
    };
    let fill = |pm: &mut Pixmap, pb: PathBuilder, c: Color| {
        if let Some(p) = pb.finish() {
            pm.fill_path(&p, &solid(c), FillRule::Winding, Transform::identity(), None);
        }
    };
    // A plain line from the ends towards the middle, stopping `gap` short.
    let side_lines = |pm: &mut Pixmap, gap: f32| {
        let mut pb = PathBuilder::new();
        pb.move_to(x + lw, cy);
        pb.line_to(cx - gap, cy);
        pb.move_to(cx + gap, cy);
        pb.line_to(x + w - lw, cy);
        stroke(pm, pb, &line);
    };
    let s = h.min(w * 0.2);
    match style {
        0 => {
            // Two mirrored swirls meeting at a diamond.
            let mut pb = PathBuilder::new();
            for dir in [-1.0f32, 1.0] {
                let ex = cx + dir * w * 0.48;
                pb.move_to(cx + dir * s * 0.45, cy);
                pb.cubic_to(cx + dir * w * 0.18, cy - h * 0.45, cx + dir * w * 0.3, cy + h * 0.45, ex, cy);
                // the curl at the end
                pb.move_to(cx + dir * w * 0.3, cy + h * 0.05);
                pb.cubic_to(cx + dir * w * 0.36, cy - h * 0.4, cx + dir * w * 0.22, cy - h * 0.42, cx + dir * w * 0.25, cy - h * 0.12);
            }
            stroke(pm, pb, &line);
            let mut d = PathBuilder::new();
            shape_at(&mut d, Outline::Diamond, cx, cy, s * 0.7);
            fill(pm, d, accent);
        }
        1 => {
            side_lines(pm, s * 0.9);
            let mut d = PathBuilder::new();
            shape_at(&mut d, Outline::Diamond, cx, cy, s * 0.8);
            for k in [-1.0f32, 1.0] {
                d.push_circle(cx + k * s * 0.65, cy, lw * 1.4);
            }
            fill(pm, d, accent);
        }
        2 => {
            // Dots, biggest in the middle.
            let n = ((w / (h.max(4.0) * 0.9)) as i32).clamp(5, 41) | 1;
            let mut d = PathBuilder::new();
            for i in 0..n {
                let t = i as f32 / (n - 1) as f32;
                let r = (h * 0.32) * (1.0 - (t - 0.5).abs() * 1.6).max(0.18);
                d.push_circle(x + w * 0.02 + t * w * 0.96, cy, r);
            }
            fill(pm, d, accent);
        }
        3 | 4 => {
            let o = if style == 3 { Outline::Heart } else { Outline::Star };
            side_lines(pm, s * 1.7);
            let mut d = PathBuilder::new();
            for (k, size) in [(-1.0f32, 0.55), (0.0, 0.85), (1.0, 0.55)] {
                shape_at(&mut d, o, cx + k * s * 0.95, cy, s * size);
            }
            fill(pm, d, accent);
        }
        5 => {
            // A wavy stem with leaves on alternate sides.
            let mut stem = PathBuilder::new();
            let n = 6;
            stem.move_to(x + lw, cy);
            for i in 0..n {
                let (x0, x1) = (x + w * i as f32 / n as f32, x + w * (i + 1) as f32 / n as f32);
                let up = if i % 2 == 0 { -1.0 } else { 1.0 };
                stem.quad_to((x0 + x1) / 2.0, cy + up * h * 0.3, x1.min(x + w - lw), cy);
            }
            stroke(pm, stem, &line);
            let mut leaves = PathBuilder::new();
            for i in 0..n * 2 {
                let lx = x + w * (i as f32 + 0.5) / (n * 2) as f32;
                let up = if i % 2 == 0 { -1.0 } else { 1.0 };
                leaf(&mut leaves, (lx, cy), (lx + h * 0.35, cy + up * h * 0.42), 0.35);
            }
            fill(pm, leaves, accent);
        }
        6 => {
            let n = ((w / h.max(4.0)) as i32).clamp(4, 40);
            let r = w / n as f32 / 2.0;
            let mut pb = PathBuilder::new();
            pb.move_to(x, cy - r * 0.5);
            for i in 0..n {
                let x0 = x + i as f32 * 2.0 * r;
                pb.quad_to(x0 + r, cy + r * 1.1, x0 + 2.0 * r, cy - r * 0.5);
            }
            stroke(pm, pb, &line);
        }
        7 => {
            // A brush stroke: thick in the middle, tapering to points.
            let mut pb = PathBuilder::new();
            let t = h * 0.42;
            pb.move_to(x, cy + h * 0.1);
            pb.cubic_to(x + w * 0.3, cy - t, x + w * 0.7, cy - t * 0.9, x + w, cy - h * 0.05);
            pb.cubic_to(x + w * 0.7, cy + t * 0.3, x + w * 0.3, cy + t * 0.5, x, cy + h * 0.1);
            pb.close();
            fill(pm, pb, accent);
        }
        8 => {
            let mut pb = PathBuilder::new();
            for k in [0.0f32, 1.0] {
                let off = k * h * 0.28;
                pb.move_to(x + w * 0.03, cy + off);
                pb.cubic_to(x + w * 0.3, cy - h * 0.45 + off, x + w * 0.7, cy + h * 0.3 + off, x + w * 0.97, cy - h * 0.2 + off);
            }
            stroke(pm, pb, &line);
        }
        9 => {
            // Two branches curving up from the middle, leaves along them.
            let mut stems = PathBuilder::new();
            let mut leaves = PathBuilder::new();
            for dir in [-1.0f32, 1.0] {
                let p0 = (cx, y + h * 0.9);
                let p1 = (cx + dir * w * 0.48, y + h * 0.15);
                stems.move_to(p0.0, p0.1);
                stems.quad_to(cx + dir * w * 0.42, y + h * 0.95, p1.0, p1.1);
                for k in 1..8 {
                    let t = k as f32 / 8.0;
                    let bx = (1.0 - t) * (1.0 - t) * p0.0 + 2.0 * (1.0 - t) * t * (cx + dir * w * 0.42) + t * t * p1.0;
                    let by = (1.0 - t) * (1.0 - t) * p0.1 + 2.0 * (1.0 - t) * t * (y + h * 0.95) + t * t * p1.1;
                    let l = h * 0.28;
                    leaf(&mut leaves, (bx, by), (bx + dir * l * 0.6, by - l), 0.3);
                    leaf(&mut leaves, (bx, by), (bx + dir * l, by + l * 0.15), 0.3);
                }
            }
            stroke(pm, stems, &line);
            fill(pm, leaves, accent);
        }
        10 => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + w * 0.12, cy);
            pb.line_to(x + w * 0.96, cy);
            // the curl on the left
            pb.move_to(x + w * 0.12, cy);
            pb.cubic_to(x + w * 0.02, cy, x + w * 0.02, cy - h * 0.45, x + w * 0.08, cy - h * 0.4);
            pb.cubic_to(x + w * 0.13, cy - h * 0.35, x + w * 0.1, cy - h * 0.15, x + w * 0.07, cy - h * 0.2);
            stroke(pm, pb, &line);
            let mut head = PathBuilder::new();
            head.move_to(x + w, cy);
            head.line_to(x + w - h * 0.6, cy - h * 0.32);
            head.line_to(x + w - h * 0.45, cy);
            head.line_to(x + w - h * 0.6, cy + h * 0.32);
            head.close();
            fill(pm, head, accent);
        }
        _ => {
            // A ribbon banner with folded tails.
            let (top, bot) = (y + h * 0.12, y + h * 0.72);
            let mut tails = PathBuilder::new();
            for dir in [-1.0f32, 1.0] {
                let inner = cx + dir * w * 0.36;
                let outer = cx + dir * w * 0.5;
                tails.move_to(inner, top + h * 0.18);
                tails.line_to(outer, top + h * 0.18);
                tails.line_to(outer - dir * w * 0.05, (top + bot) / 2.0 + h * 0.18);
                tails.line_to(outer, bot + h * 0.18);
                tails.line_to(inner, bot + h * 0.18);
                tails.close();
            }
            let mut shade = accent;
            shade.apply_opacity(0.7);
            fill(pm, tails, shade);
            let mut band = PathBuilder::new();
            band.move_to(x + w * 0.12, top);
            band.line_to(x + w * 0.88, top);
            band.line_to(x + w * 0.88, bot);
            band.line_to(x + w * 0.12, bot);
            band.close();
            fill(pm, band, accent);
        }
    }
}
