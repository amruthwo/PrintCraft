//! Photo Editor operations: crop (box or shape), turn, color,
//! brightness/contrast, and one-click photo effects. Each returns a new PNG.

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

use crate::model::Outline;

/// Which parts of the photo to keep: one byte per pixel (255 = keep), at
/// the size of the cropped and turned working picture.
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combine {
    New,
    Add,
    Subtract,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Adjust {
    pub crop: [f32; 4], // left, top, right, bottom as fractions removed
    pub shape: Option<Outline>,
    pub turns: u8,
    pub brightness: i32,
    pub contrast: f32,
    pub saturation: f32,
    pub warmth: f32,
    pub sharpen: f32,
    pub effect: Effect,
    pub red_eye: bool,
    /// Recolor: turn around the color wheel (degrees) and push red/green/blue.
    pub hue: f32,
    pub rgb: [f32; 3],
    pub mask: Option<std::sync::Arc<Mask>>,
}

impl Default for Adjust {
    fn default() -> Self {
        Self {
            crop: [0.0; 4],
            shape: None,
            turns: 0,
            brightness: 0,
            contrast: 0.0,
            saturation: 0.0,
            warmth: 0.0,
            sharpen: 0.0,
            effect: Effect::None,
            red_eye: false,
            hue: 0.0,
            rgb: [0.0; 3],
            mask: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    BlackWhite,
    Sepia,
    Negative,
    Soften,
    Posterize,
    Emboss,
    Sketch,
}

impl Effect {
    pub const ALL: [Effect; 8] = [Effect::None, Effect::BlackWhite, Effect::Sepia, Effect::Negative, Effect::Soften, Effect::Posterize, Effect::Emboss, Effect::Sketch];
    pub fn label(self) -> &'static str {
        match self {
            Effect::None => "No effect",
            Effect::BlackWhite => "Black & White",
            Effect::Sepia => "Old-Time Sepia",
            Effect::Negative => "Negative",
            Effect::Soften => "Soften",
            Effect::Posterize => "Poster",
            Effect::Emboss => "Embossed",
            Effect::Sketch => "Pencil Sketch",
        }
    }
}

/// The picture after cropping and turning: what the selection tools work on.
pub fn geometry(bytes: &[u8], a: &Adjust, max_side: u32) -> Option<RgbaImage> {
    let mut img = image::load_from_memory(bytes).ok()?;
    if img.width().max(img.height()) > max_side {
        img = img.resize(max_side, max_side, image::imageops::FilterType::Triangle);
    }
    let (w, h) = img.dimensions();
    let (l, t) = ((a.crop[0] * w as f32) as u32, (a.crop[1] * h as f32) as u32);
    let (r, b) = ((a.crop[2] * w as f32) as u32, (a.crop[3] * h as f32) as u32);
    img = img.crop_imm(l, t, w.saturating_sub(l + r).max(1), h.saturating_sub(t + b).max(1));
    for _ in 0..a.turns % 4 {
        img = img.rotate90();
    }
    Some(img.to_rgba8())
}

fn close(a: [u8; 4], b: [u8; 4], tol: f32) -> bool {
    let d = ((a[0] as f32 - b[0] as f32).powi(2) + (a[1] as f32 - b[1] as f32).powi(2) + (a[2] as f32 - b[2] as f32).powi(2)).sqrt();
    d <= tol * 4.42 // tol in 0..100 maps to the full color distance
}

/// Magic wand: the connected area of similar color around (x, y).
pub fn wand(img: &RgbaImage, x: u32, y: u32, tol: f32) -> Vec<bool> {
    let (w, h) = img.dimensions();
    let mut sel = vec![false; (w * h) as usize];
    if x >= w || y >= h {
        return sel;
    }
    let seed = img.get_pixel(x, y).0;
    let mut stack = vec![(x, y)];
    while let Some((cx, cy)) = stack.pop() {
        let i = (cy * w + cx) as usize;
        if sel[i] || !close(img.get_pixel(cx, cy).0, seed, tol) {
            continue;
        }
        sel[i] = true;
        if cx > 0 {
            stack.push((cx - 1, cy));
        }
        if cx + 1 < w {
            stack.push((cx + 1, cy));
        }
        if cy > 0 {
            stack.push((cx, cy - 1));
        }
        if cy + 1 < h {
            stack.push((cx, cy + 1));
        }
    }
    sel
}

/// Every pixel close to a color, anywhere in the picture.
pub fn color_select(img: &RgbaImage, c: [u8; 4], tol: f32) -> Vec<bool> {
    img.pixels().map(|p| close(p.0, c, tol)).collect()
}

/// Inside of an outline drawn point by point (points in pixels).
pub fn lasso(w: u32, h: u32, pts: &[(f32, f32)]) -> Vec<bool> {
    let mut sel = vec![false; (w * h) as usize];
    if pts.len() < 3 {
        return sel;
    }
    for y in 0..h {
        let fy = y as f32 + 0.5;
        if let Some((a, b)) = crate::raster::span_at(pts, fy) {
            // Even-odd fill across all crossings for odd outlines.
            let mut xs = Vec::new();
            for i in 0..pts.len() {
                let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
                if (p.1 <= fy && q.1 > fy) || (q.1 <= fy && p.1 > fy) {
                    xs.push(p.0 + (fy - p.1) / (q.1 - p.1) * (q.0 - p.0));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let _ = (a, b);
            for pair in xs.chunks_exact(2) {
                let (x0, x1) = (pair[0].max(0.0) as u32, (pair[1].min(w as f32)) as u32);
                for x in x0..x1.min(w) {
                    sel[(y * w + x) as usize] = true;
                }
            }
        }
    }
    sel
}

/// Guess the background: colors connected to the picture's edges that
/// look like the edges. Returns what to keep (not background).
pub fn remove_background(img: &RgbaImage, tol: f32) -> Vec<bool> {
    let (w, h) = img.dimensions();
    let mut bg = vec![false; (w * h) as usize];
    let mut seeds = Vec::new();
    let step = (w.max(h) / 60).max(1);
    for x in (0..w).step_by(step as usize) {
        seeds.push((x, 0));
        seeds.push((x, h - 1));
    }
    for y in (0..h).step_by(step as usize) {
        seeds.push((0, y));
        seeds.push((w - 1, y));
    }
    for (sx, sy) in seeds {
        if bg[(sy * w + sx) as usize] {
            continue;
        }
        let area = wand(img, sx, sy, tol);
        for (i, v) in area.into_iter().enumerate() {
            bg[i] |= v;
        }
    }
    // Ignore tiny islands of "background" inside the subject.
    bg.iter().map(|b| !b).collect()
}

pub fn combine(mask: Option<&Mask>, w: u32, h: u32, sel: &[bool], how: Combine) -> Mask {
    let mut data = match (mask, how) {
        (Some(m), Combine::Add | Combine::Subtract) if m.w == w && m.h == h => m.data.clone(),
        (_, Combine::Subtract) => vec![255; (w * h) as usize],
        _ => vec![0; (w * h) as usize],
    };
    for (d, s) in data.iter_mut().zip(sel) {
        if *s {
            *d = if how == Combine::Subtract { 0 } else { 255 };
        }
    }
    Mask { w, h, data }
}

/// Soften the mask edge by `r` pixels.
pub fn feather(m: &Mask, r: u32) -> Mask {
    if r == 0 {
        return m.clone();
    }
    let img = image::GrayImage::from_raw(m.w, m.h, m.data.clone()).unwrap_or_else(|| image::GrayImage::new(m.w, m.h));
    let b = image::imageops::blur(&img, r as f32 / 2.0);
    Mask { w: m.w, h: m.h, data: b.into_raw() }
}

pub fn apply(bytes: &[u8], a: &Adjust, max_side: u32) -> Option<DynamicImage> {
    let mut img = image::load_from_memory(bytes).ok()?;
    if img.width().max(img.height()) > max_side {
        img = img.resize(max_side, max_side, image::imageops::FilterType::Triangle);
    }
    let (w, h) = img.dimensions();
    let (l, t) = ((a.crop[0] * w as f32) as u32, (a.crop[1] * h as f32) as u32);
    let (r, b) = ((a.crop[2] * w as f32) as u32, (a.crop[3] * h as f32) as u32);
    let cw = w.saturating_sub(l + r).max(1);
    let ch = h.saturating_sub(t + b).max(1);
    img = img.crop_imm(l, t, cw, ch);
    for _ in 0..a.turns % 4 {
        img = img.rotate90();
    }
    if a.brightness != 0 {
        img = img.brighten(a.brightness);
    }
    if a.contrast.abs() > 0.01 {
        img = img.adjust_contrast(a.contrast);
    }
    if a.sharpen > 0.01 {
        img = img.unsharpen(1.5, (a.sharpen * 10.0) as i32);
    }
    let mut rgba = img.to_rgba8();
    if a.saturation.abs() > 0.01 || a.warmth.abs() > 0.01 {
        for p in rgba.pixels_mut() {
            let [r, g, b, al] = p.0;
            let gray = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            let k = 1.0 + a.saturation;
            let mix = |c: u8, warm: f32| (gray + (c as f32 - gray) * k + warm).clamp(0.0, 255.0) as u8;
            *p = Rgba([mix(r, a.warmth * 25.0), mix(g, 0.0), mix(b, -a.warmth * 25.0), al]);
        }
    }
    if a.hue.abs() > 0.5 || a.rgb.iter().any(|v| v.abs() > 0.005) {
        for p in rgba.pixels_mut() {
            let [r, g, b, al] = p.0;
            let mut c = crate::model::Rgba(r, g, b, 255);
            if a.hue.abs() > 0.5 {
                let (h, s, l) = crate::colors::to_hsl(c);
                c = crate::colors::from_hsl(h + a.hue, s, l);
            }
            let push = |v: u8, k: f32| (v as f32 + k * 120.0).clamp(0.0, 255.0) as u8;
            *p = Rgba([push(c.0, a.rgb[0]), push(c.1, a.rgb[1]), push(c.2, a.rgb[2]), al]);
        }
    }
    if a.red_eye {
        for p in rgba.pixels_mut() {
            let [r, g, b, al] = p.0;
            if r as f32 > 1.6 * (g as f32 + b as f32) / 2.0 && r > 90 {
                let v = ((g as u16 + b as u16) / 2) as u8;
                *p = Rgba([v, g, b, al]);
            }
        }
    }
    rgba = effect(rgba, a.effect);
    if let Some(shape) = a.shape {
        mask(&mut rgba, shape);
    }
    if let Some(m) = &a.mask {
        let g = image::GrayImage::from_raw(m.w, m.h, m.data.clone())?;
        let g = image::imageops::resize(&g, rgba.width(), rgba.height(), image::imageops::FilterType::Triangle);
        for (p, k) in rgba.pixels_mut().zip(g.pixels()) {
            p.0[3] = ((p.0[3] as u16 * k.0[0] as u16) / 255) as u8;
        }
    }
    Some(DynamicImage::ImageRgba8(rgba))
}

fn effect(img: RgbaImage, e: Effect) -> RgbaImage {
    let mut img = img;
    match e {
        Effect::None => {}
        Effect::BlackWhite | Effect::Sepia | Effect::Negative | Effect::Posterize => {
            for p in img.pixels_mut() {
                let [r, g, b, a] = p.0;
                let y = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
                *p = match e {
                    Effect::BlackWhite => Rgba([y, y, y, a]),
                    Effect::Sepia => Rgba([(y as f32 * 1.07 + 20.0).min(255.0) as u8, (y as f32 * 0.9 + 10.0).min(255.0) as u8, (y as f32 * 0.7) as u8, a]),
                    Effect::Negative => Rgba([255 - r, 255 - g, 255 - b, a]),
                    _ => Rgba([r / 64 * 64 + 32, g / 64 * 64 + 32, b / 64 * 64 + 32, a]),
                };
            }
        }
        Effect::Soften => img = image::imageops::blur(&img, 2.0),
        Effect::Emboss | Effect::Sketch => {
            let gray = DynamicImage::ImageRgba8(img.clone()).to_luma8();
            let k: [f32; 9] = if e == Effect::Emboss { [-2.0, -1.0, 0.0, -1.0, 1.0, 1.0, 0.0, 1.0, 2.0] } else { [-1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0] };
            let f = image::imageops::filter3x3(&gray, &k);
            for (x, y, p) in img.enumerate_pixels_mut() {
                let v = f.get_pixel(x, y).0[0];
                let v = if e == Effect::Sketch { 255 - v.saturating_mul(2) } else { v.saturating_add(40) };
                *p = Rgba([v, v, v, p.0[3]]);
            }
        }
    }
    img
}

fn mask(img: &mut RgbaImage, shape: Outline) {
    let (w, h) = img.dimensions();
    let Some(mut m) = tiny_skia::Pixmap::new(w, h) else { return };
    let pts = crate::raster::outline_points(shape, w as f32, h as f32);
    let mut pb = tiny_skia::PathBuilder::new();
    if let Some((x, y)) = pts.first() {
        pb.move_to(*x, *y);
    }
    for (x, y) in &pts[1..] {
        pb.line_to(*x, *y);
    }
    pb.close();
    if let Some(path) = pb.finish() {
        let paint = tiny_skia::Paint { anti_alias: true, ..Default::default() };
        m.fill_path(&path, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
    }
    for (i, p) in img.pixels_mut().enumerate() {
        let a = m.data()[i * 4 + 3] as u16;
        p.0[3] = ((p.0[3] as u16 * a) / 255) as u8;
    }
}

pub fn to_png(img: &DynamicImage) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png);
    out
}
