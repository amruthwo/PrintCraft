//! A small Windows Metafile (WMF) painter: enough of the public format for
//! clip art (pens, brushes, polygons, lines, rectangles, ellipses, saved
//! states, embedded bitmaps). Records it doesn't know are skipped.

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};

use super::{le16, le32};

#[derive(Clone, Copy)]
enum Obj {
    Pen { color: Color, width: f32, none: bool },
    Brush { color: Color, none: bool },
    Other,
}

#[derive(Clone)]
struct State {
    org: (f32, f32),
    ext: (f32, f32),
    pen: Obj,
    brush: Obj,
    winding: bool,
    pos: (f32, f32),
}

fn i16_at(d: &[u8], at: usize) -> f32 {
    le16(d, at).map(|v| v as i16 as f32).unwrap_or(0.0)
}

fn colorref(d: &[u8], at: usize) -> Color {
    let r = d.get(at).copied().unwrap_or(0);
    let g = d.get(at + 1).copied().unwrap_or(0);
    let b = d.get(at + 2).copied().unwrap_or(0);
    Color::from_rgba8(r, g, b, 255)
}

/// Paint a placeable metafile so its longer side is `max_px` pixels.
pub fn render(d: &[u8], max_px: u32) -> Option<Pixmap> {
    if le32(d, 0)? != 0x9AC6_CDD7 {
        return None;
    }
    let (l, t, r, b) = (i16_at(d, 6), i16_at(d, 8), i16_at(d, 10), i16_at(d, 12));
    let (bw, bh) = ((r - l).abs().max(1.0), (b - t).abs().max(1.0));
    let k = max_px as f32 / bw.max(bh);
    let (pw, ph) = ((bw * k).ceil() as u32, (bh * k).ceil() as u32);
    let mut pm = Pixmap::new(pw.max(1), ph.max(1))?;
    let header = 22;
    let hsize = le16(d, header + 2)? as usize * 2;
    let mut at = header + hsize;
    let n_objects = le16(d, header + 10).unwrap_or(16) as usize;
    let mut table: Vec<Option<Obj>> = vec![None; n_objects.max(1)];
    let mut st = State {
        org: (l.min(r), t.min(b)),
        ext: (bw, bh),
        pen: Obj::Pen { color: Color::BLACK, width: 1.0, none: false },
        brush: Obj::Brush { color: Color::WHITE, none: false },
        winding: false,
        pos: (0.0, 0.0),
    };
    let mut saved: Vec<State> = Vec::new();
    let mut guard = 0;
    while at + 6 <= d.len() && guard < 2_000_000 {
        guard += 1;
        let size = le32(d, at)? as usize * 2;
        let func = le16(d, at + 4)?;
        if size < 6 || at + size > d.len() {
            break;
        }
        let p = at + 6;
        let map = |st: &State, x: f32, y: f32| ((x - st.org.0) / st.ext.0 * pw as f32, (y - st.org.1) / st.ext.1 * ph as f32);
        let line_w = |st: &State, w: f32| (w / st.ext.0.abs() * pw as f32).max(1.0);
        match func {
            0x0000 => break,
            0x020B => st.org = (i16_at(d, p + 2), i16_at(d, p)),
            0x020C => {
                let (ey, ex) = (i16_at(d, p), i16_at(d, p + 2));
                if ex != 0.0 && ey != 0.0 {
                    st.ext = (ex, ey);
                }
            }
            0x0106 => st.winding = le16(d, p).unwrap_or(1) == 2,
            0x001E => saved.push(st.clone()),
            0x0127 => {
                if let Some(s) = saved.pop() {
                    st = s;
                }
            }
            0x02FA => {
                let style = le16(d, p).unwrap_or(0);
                let w = i16_at(d, p + 2);
                add(&mut table, Obj::Pen { color: colorref(d, p + 6), width: w, none: style & 0x0F == 5 });
            }
            0x02FC => {
                let style = le16(d, p).unwrap_or(0);
                add(&mut table, Obj::Brush { color: colorref(d, p + 2), none: style == 1 });
            }
            0x00F7 | 0x0142 | 0x02FB | 0x01F9 | 0x06FF => add(&mut table, Obj::Other),
            0x012D => {
                let i = le16(d, p).unwrap_or(0) as usize;
                if let Some(Some(o)) = table.get(i) {
                    match o {
                        Obj::Pen { .. } => st.pen = *o,
                        Obj::Brush { .. } => st.brush = *o,
                        Obj::Other => {}
                    }
                }
            }
            0x01F0 => {
                let i = le16(d, p).unwrap_or(0) as usize;
                if let Some(slot) = table.get_mut(i) {
                    *slot = None;
                }
            }
            0x0214 => st.pos = (i16_at(d, p + 2), i16_at(d, p)),
            0x0213 => {
                let to = (i16_at(d, p + 2), i16_at(d, p));
                let mut pb = PathBuilder::new();
                let a = map(&st, st.pos.0, st.pos.1);
                let b2 = map(&st, to.0, to.1);
                pb.move_to(a.0, a.1);
                pb.line_to(b2.0, b2.1);
                stroke(&mut pm, pb, &st, line_w(&st, pen_w(&st)));
                st.pos = to;
            }
            0x0324 | 0x0325 => {
                let n = le16(d, p).unwrap_or(0) as usize;
                let mut pb = PathBuilder::new();
                for i in 0..n {
                    let (x, y) = map(&st, i16_at(d, p + 2 + i * 4), i16_at(d, p + 4 + i * 4));
                    if i == 0 {
                        pb.move_to(x, y)
                    } else {
                        pb.line_to(x, y)
                    }
                }
                if func == 0x0324 {
                    pb.close();
                    fill_and_stroke(&mut pm, pb, &st, line_w(&st, pen_w(&st)));
                } else {
                    stroke(&mut pm, pb, &st, line_w(&st, pen_w(&st)));
                }
            }
            0x0538 => {
                let polys = le16(d, p).unwrap_or(0) as usize;
                let mut counts = Vec::new();
                for i in 0..polys {
                    counts.push(le16(d, p + 2 + i * 2).unwrap_or(0) as usize);
                }
                let mut q = p + 2 + polys * 2;
                let mut pb = PathBuilder::new();
                for c in counts {
                    for i in 0..c {
                        let (x, y) = map(&st, i16_at(d, q), i16_at(d, q + 2));
                        q += 4;
                        if i == 0 {
                            pb.move_to(x, y)
                        } else {
                            pb.line_to(x, y)
                        }
                    }
                    pb.close();
                }
                fill_and_stroke(&mut pm, pb, &st, line_w(&st, pen_w(&st)));
            }
            0x041B | 0x0418 | 0x061C => {
                let (bottom, right, top, left) = if func == 0x061C {
                    (i16_at(d, p + 4), i16_at(d, p + 6), i16_at(d, p + 8), i16_at(d, p + 10))
                } else {
                    (i16_at(d, p), i16_at(d, p + 2), i16_at(d, p + 4), i16_at(d, p + 6))
                };
                let a = map(&st, left, top);
                let b2 = map(&st, right, bottom);
                let (x0, y0, x1, y1) = (a.0.min(b2.0), a.1.min(b2.1), a.0.max(b2.0), a.1.max(b2.1));
                let mut pb = PathBuilder::new();
                if func == 0x0418 {
                    if let Some(r) = tiny_skia::Rect::from_ltrb(x0, y0, x1.max(x0 + 0.1), y1.max(y0 + 0.1)) {
                        pb.push_oval(r);
                    }
                } else if let Some(r) = tiny_skia::Rect::from_ltrb(x0, y0, x1.max(x0 + 0.1), y1.max(y0 + 0.1)) {
                    pb.push_rect(r);
                }
                fill_and_stroke(&mut pm, pb, &st, line_w(&st, pen_w(&st)));
            }
            0x0F43 | 0x0B41 => draw_dib(&mut pm, d, p, at + size, func, &st, &map),
            _ => {}
        }
        at += size;
    }
    Some(pm)
}

fn pen_w(st: &State) -> f32 {
    match st.pen {
        Obj::Pen { width, .. } => width.max(1.0),
        _ => 1.0,
    }
}

fn add(table: &mut Vec<Option<Obj>>, o: Obj) {
    if let Some(slot) = table.iter_mut().find(|s| s.is_none()) {
        *slot = Some(o);
    } else {
        table.push(Some(o));
    }
}

fn fill_and_stroke(pm: &mut Pixmap, pb: PathBuilder, st: &State, w: f32) {
    let Some(path) = pb.finish() else { return };
    if let Obj::Brush { color, none: false } = st.brush {
        let paint = Paint { shader: tiny_skia::Shader::SolidColor(color), anti_alias: true, ..Default::default() };
        let rule = if st.winding { FillRule::Winding } else { FillRule::EvenOdd };
        pm.fill_path(&path, &paint, rule, Transform::identity(), None);
    }
    if let Obj::Pen { color, none: false, .. } = st.pen {
        let paint = Paint { shader: tiny_skia::Shader::SolidColor(color), anti_alias: true, ..Default::default() };
        pm.stroke_path(&path, &paint, &Stroke { width: w, ..Default::default() }, Transform::identity(), None);
    }
}

fn stroke(pm: &mut Pixmap, pb: PathBuilder, st: &State, w: f32) {
    let Some(path) = pb.finish() else { return };
    if let Obj::Pen { color, none: false, .. } = st.pen {
        let paint = Paint { shader: tiny_skia::Shader::SolidColor(color), anti_alias: true, ..Default::default() };
        pm.stroke_path(&path, &paint, &Stroke { width: w, ..Default::default() }, Transform::identity(), None);
    }
}

/// StretchDIB / DIBStretchBlt with an embedded device-independent bitmap.
fn draw_dib(pm: &mut Pixmap, d: &[u8], p: usize, end: usize, func: u16, st: &State, map: &dyn Fn(&State, f32, f32) -> (f32, f32)) {
    // StretchDIB: rop(4) usage(2) srcH srcW srcY srcX destH destW destY destX, then DIB.
    // DIBStretchBlt: rop(4) srcH srcW srcY srcX destH destW destY destX, then DIB.
    let skip = if func == 0x0F43 { 6 } else { 4 };
    let q = p + skip;
    let (dh, dw, dy, dx) = (i16_at(d, q + 8), i16_at(d, q + 10), i16_at(d, q + 12), i16_at(d, q + 14));
    let dib = q + 16;
    let Some(bytes) = d.get(dib..end) else { return };
    // Wrap the DIB in a BMP file header so the image decoder can read it.
    let hdr_size = le32(bytes, 0).unwrap_or(40) as usize;
    let bits = le16(bytes, 14).unwrap_or(24) as usize;
    let colors = le32(bytes, 32).unwrap_or(0) as usize;
    let palette = if bits <= 8 { (if colors == 0 { 1usize << bits } else { colors }) * 4 } else { 0 };
    let mut bmp = Vec::with_capacity(bytes.len() + 14);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((bytes.len() + 14) as u32).to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&((14 + hdr_size + palette) as u32).to_le_bytes());
    bmp.extend_from_slice(bytes);
    let Ok(img) = image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp) else { return };
    let rgba = img.to_rgba8();
    let Some(src) = crate::raster::rgba_to_pixmap(rgba.width(), rgba.height(), rgba.as_raw()) else { return };
    let a = map(st, dx, dy);
    let b = map(st, dx + dw, dy + dh);
    let sx = (b.0 - a.0) / src.width() as f32;
    let sy = (b.1 - a.1) / src.height() as f32;
    pm.draw_pixmap(0, 0, src.as_ref(), &PixmapPaint::default(), Transform::from_row(sx, 0.0, 0.0, sy, a.0, a.1), None);
}
