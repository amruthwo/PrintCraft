//! Older project files (.sig, .car, .ban, ...): an OLE2 compound file
//! whose main stream is a small record store. What we can read today: paper
//! size and orientation, the page preview picture, pictures stored inside,
//! the text, the fonts used and where page objects sit. See docs/FORMATS.md.

use std::io::Read;

use super::{le16, le32, lei32};
use crate::kinds::Kind;
use crate::model::*;

const BANNER_LEN: usize = 28;
const UNITS_PER_INCH: f32 = 1800.0;
/// Version marker of the file layout PrintCraft reads (0x4E4A).
const CURRENT_MARK: u16 = 20042;
/// Marker of an earlier file layout PrintCraft does not read (0x43A5).
const OLD_MARK: u16 = 17317;
/// Where an object's bounds sit inside its body (0xa2).
const BOUNDS_AT: usize = 162;
/// Where an object's center point sits inside its body.
const CENTER_AT: usize = 10;

#[derive(Debug, Default)]
pub struct Legacy {
    pub width: f32,
    pub height: f32,
    pub landscape: bool,
    pub preview: Option<(u32, u32, Vec<u8>)>,
    pub pictures: Vec<Vec<u8>>,
    pub texts: Vec<String>,
    pub fonts: Vec<String>,
    /// (object type, left, top, right, bottom) in inches.
    pub placements: Vec<(u16, f32, f32, f32, f32)>,
    pub notes: Vec<String>,
}

struct Store<'a> {
    d: &'a [u8],
    /// record id -> (kind, payload offset, payload length)
    index: Vec<(u32, u16, usize, usize)>,
    current: bool,
}

impl<'a> Store<'a> {
    fn open(d: &'a [u8]) -> Result<Self, String> {
        // The stream opens with a short text banner ending CR LF 0x1A.
        if d.len() < 0x3b || d[BANNER_LEN - 3] != b'\r' || d[BANNER_LEN - 2] != b'\n' || d[BANNER_LEN - 1] != 0x1a {
            return Err("This doesn't look like an older project file PrintCraft can read.".into());
        }
        let mark = u16::from_le_bytes([d[0x1d], d[0x1e]]);
        if mark == OLD_MARK {
            return Err("This project was saved by a much older version. Open and re-save it in the program that made it, then try again.".into());
        }
        let dir = le32(d, 0x37).ok_or("short header")? as usize;
        let rec_len = |payload: usize| -> Option<usize> {
            let h = payload.checked_sub(8)?;
            Some(u32::from_le_bytes([d[h + 1], d[h + 2], d[h + 3], 0]) as usize)
        };
        let dir_len = rec_len(dir).ok_or("bad directory")?;
        let mut index = Vec::new();
        let mut page_no = 0usize;
        for slot in 0..dir_len / 4 {
            let Some(off) = le32(d, dir + slot * 4) else { break };
            let off = off as usize;
            if off < 8 || off >= d.len() || d[off - 8] != 3 {
                continue;
            }
            for e in 0..64 {
                let at = off + e * 6;
                let (Some(kind), Some(payload)) = (le16(d, at), le32(d, at + 2)) else { break };
                if kind == 0 || payload == 0 {
                    continue;
                }
                let payload = payload as usize;
                let Some(len) = rec_len(payload) else { continue };
                if payload + len > d.len() || d[payload - 8] != 2 {
                    continue;
                }
                index.push(((page_no * 64 + e + 1) as u32, kind, payload, len));
            }
            page_no += 1;
        }
        Ok(Self { d, index, current: mark == CURRENT_MARK })
    }

    fn of_kind(&self, kind: u16) -> impl Iterator<Item = &'a [u8]> + '_ {
        self.index.iter().filter(move |e| e.1 == kind).map(|e| &self.d[e.2..e.2 + e.3])
    }
}

pub fn read(bytes: &[u8]) -> Result<Legacy, String> {
    let mut file = cfb::CompoundFile::open(std::io::Cursor::new(bytes)).map_err(|_| "This file isn't an older project PrintCraft can read (not a compound file).".to_string())?;
    let stream_name = file
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_path_buf())
        .find(|p| p.to_string_lossy().contains("Document"))
        .ok_or("The project data stream is missing.")?;
    let mut data = Vec::new();
    file.open_stream(&stream_name).map_err(|e| e.to_string())?.read_to_end(&mut data).map_err(|e| e.to_string())?;
    let store = Store::open(&data)?;
    let mut out = Legacy::default();
    if !store.current {
        out.notes.push("This file is from a different version than expected, so some parts may be missing.".into());
    }

    // Document info: paper size in 1/1800 inch.
    if let Some(p) = store.of_kind(2).next() {
        let orient = le16(p, 4).unwrap_or(0);
        let w = le32(p, 10).unwrap_or(15300) as f32 / UNITS_PER_INCH;
        let h = le32(p, 14).unwrap_or(19800) as f32 / UNITS_PER_INCH;
        out.width = w;
        out.height = h;
        out.landscape = orient == 1 || w > h;
    } else {
        out.width = 8.5;
        out.height = 11.0;
        out.notes.push("Paper size not found; using 8.5 × 11 in.".into());
    }

    // Preview picture: zlib RGB.
    if let Some(p) = store.of_kind(1).next() {
        let (w, h, n) = (le32(p, 0x30).unwrap_or(0), le32(p, 0x34).unwrap_or(0), le32(p, 0x38).unwrap_or(0) as usize);
        if w > 0 && h > 0 && w < 4096 && h < 4096 && 0x3c + n <= p.len() {
            let mut rgb = Vec::new();
            if flate2::read::ZlibDecoder::new(&p[0x3c..0x3c + n]).read_to_end(&mut rgb).is_ok() && rgb.len() >= (w * h * 3) as usize {
                out.preview = Some((w, h, rgb));
            }
        }
    }

    // Pictures stored inside: a whole BMP at +0x0b.
    for p in store.of_kind(11) {
        if let Some(b) = p.get(0x0b..)
            && b.starts_with(b"BM")
        {
            let size = le32(b, 2).unwrap_or(b.len() as u32) as usize;
            out.pictures.push(b[..size.min(b.len())].to_vec());
        }
    }

    // Fonts: u16 length + ASCII at +0x0c.
    for p in store.of_kind(3) {
        if let Some(n) = le16(p, 0x0c) {
            let n = n as usize;
            if let Some(s) = p.get(0x0e..0x0e + n) {
                let name: String = s.iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
                if !name.is_empty() && !out.fonts.contains(&name) {
                    out.fonts.push(name);
                }
            }
        }
    }

    // Text: UTF-16 runs of readable characters.
    for p in store.of_kind(21) {
        let t = utf16_text(p);
        if !t.trim().is_empty() {
            out.texts.push(t);
        }
    }

    // Page objects: (u16 type, u32 length, body) with bounds at body+0xa2.
    for p in store.of_kind(4) {
        let mut at = 0usize;
        while at + 6 <= p.len() {
            let (Some(t), Some(n)) = (le16(p, at), le32(p, at + 2)) else { break };
            let n = n as usize;
            if t == 0 || at + 6 + n > p.len() {
                break;
            }
            let body = &p[at + 6..at + 6 + n];
            if body.len() >= BOUNDS_AT + 16 && t < 1000 {
                let r: Vec<f32> = (0..4).filter_map(|i| lei32(body, BOUNDS_AT + i * 4)).map(|v| v as f32 / UNITS_PER_INCH).collect();
                let center = (lei32(body, CENTER_AT), lei32(body, CENTER_AT + 4));
                let centered = match center {
                    (Some(cx), Some(cy)) if r.len() == 4 => {
                        let (cx, cy) = (cx as f32 / UNITS_PER_INCH, cy as f32 / UNITS_PER_INCH);
                        (cx - (r[0] + r[2]) / 2.0).abs() < 0.05 && (cy - (r[1] + r[3]) / 2.0).abs() < 0.05
                    }
                    _ => false,
                };
                if centered && r[2] > r[0] && r[3] > r[1] {
                    out.placements.push((t, r[0], r[1], r[2], r[3]));
                }
            }
            at += 6 + n;
        }
    }
    Ok(out)
}

fn utf16_text(p: &[u8]) -> String {
    let mut best = String::new();
    let mut cur = String::new();
    for ch in p.chunks_exact(2) {
        let c = u16::from_le_bytes([ch[0], ch[1]]);
        let ok = (0x20..0xd800).contains(&c) || c == 0x0d || c == 0x0a || c == 0x09;
        if ok {
            let ch = char::from_u32(c as u32).unwrap_or(' ');
            cur.push(if ch == '\r' { '\n' } else { ch });
        } else {
            if cur.chars().filter(|c| c.is_alphabetic()).count() > best.chars().filter(|c| c.is_alphabetic()).count() {
                best = std::mem::take(&mut cur);
            }
            cur.clear();
        }
    }
    if cur.chars().filter(|c| c.is_alphabetic()).count() > best.chars().filter(|c| c.is_alphabetic()).count() {
        best = cur;
    }
    best.trim().to_string()
}

/// Turn an older project into a PrintCraft design. Positions of text boxes
/// aren't decoded yet, so text and pictures are laid out on the page in
/// order and the person is told so.
pub fn to_project(l: Legacy, kind: Kind) -> Project {
    let mut p = Project::new(kind, l.landscape);
    p.width = l.width;
    p.height = l.height;
    p.panels.truncate(1);
    let mut pics = Vec::new();
    for (i, bmp) in l.pictures.iter().enumerate() {
        // Store as PNG so the design file stays small.
        let png = image::load_from_memory(bmp)
            .ok()
            .and_then(|img| {
                let mut out = Vec::new();
                img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).ok().map(|_| out)
            })
            .unwrap_or_else(|| bmp.clone());
        let (w, h) = image::load_from_memory(bmp).map(|im| (im.width() as f32 / 96.0, im.height() as f32 / 96.0)).unwrap_or((2.5, 2.5));
        let id = p.add_asset(&format!("Picture {}", i + 1), png, "From the imported project");
        pics.push((id, (w.max(0.3), h.max(0.3))));
    }
    // Exact positions aren't decoded yet, so flow everything down the page:
    // text first, then pictures, adding pages when one fills up.
    let family = l.fonts.first().cloned().unwrap_or_else(|| crate::fonts::DEFAULT_FAMILY.into());
    let margin = 0.5;
    let width = p.width - 2.0 * margin;
    let size = 11.0;
    let chars_per_line = (width * 72.0 / (size * 0.5)).max(10.0);
    let line_h = size * 1.3 / 72.0;
    let mut pages: Vec<Vec<Item>> = vec![Vec::new()];
    let mut y = margin;
    let place = |p: &mut Project, pages: &mut Vec<Vec<Item>>, y: &mut f32, h: f32, w: f32, content: Content| {
        if *y + h > p.height - margin && *y > margin + 0.01 {
            pages.push(Vec::new());
            *y = margin;
        }
        let id = p.fresh_id();
        pages.last_mut().unwrap().push(Item::new(id, Frame::new(margin, *y, w, h.min(p.height - 2.0 * margin)), content));
        *y += h + 0.2;
    };
    for t in &l.texts {
        let lines: f32 = t.split('\n').map(|para| (para.chars().count() as f32 / chars_per_line).ceil().max(1.0)).sum();
        let h = lines * line_h + 0.15;
        let look = TextLook { family: family.clone(), size, ..Default::default() };
        place(&mut p, &mut pages, &mut y, h, width, Content::Text { text: t.clone(), look, shape: None, fit: false, runs: Vec::new() });
    }
    for (id, (w, h)) in pics {
        let k = (width / w).min(4.0 / h).min(1.0);
        place(&mut p, &mut pages, &mut y, h * k, w * k, Content::Picture { asset: id });
    }
    let template = p.panels[0].clone();
    p.panels = pages
        .into_iter()
        .enumerate()
        .map(|(i, items)| Panel { name: format!("Page {}", i + 1), items, ..template.clone() })
        .collect();
    let mut notes = vec![
        "Opened from an older project file. The paper size, words and pictures were brought in. Their exact positions and text styles can't be read yet, so they are laid out in order down the page; please move them where you want them.".to_string(),
    ];
    if !l.fonts.is_empty() {
        notes.push(format!("Fonts used in the imported project: {}.", l.fonts.join(", ")));
    }
    notes.extend(l.notes);
    p.notes = notes;
    if let Some((w, h, rgb)) = l.preview {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        // Stored top-down, 3 bytes per pixel in R, G, B order.
        for px in rgb.chunks_exact(3).take((w * h) as usize) {
            rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
        }
        let mut png = Vec::new();
        if let Some(img) = image::RgbaImage::from_raw(w, h, rgba)
            && image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).is_ok()
        {
            p.add_asset("Original preview", png, "Preview stored in the imported project");
        }
    }
    p
}
