//! Printing and saving as a picture or PDF. Panels are arranged on paper
//! the way each project kind needs (folding cards, label sheets, banners).

use std::io::Write;
use std::path::{Path, PathBuf};

use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::fonts::FontBook;
use crate::kinds::Kind;
use crate::model::Project;
use crate::raster::{PictureCache, render_panel};

pub struct Place {
    pub panel: usize,
    pub x: f32,
    pub y: f32,
    pub upside_down: bool,
    /// Address merge: the person whose details fill this copy.
    pub record: Option<crate::settings::Contact>,
}

pub struct Sheet {
    pub w: f32,
    pub h: f32,
    pub places: Vec<Place>,
}

/// How a project goes onto paper (inches). Fold layouts: see docs/DESIGN.md.
pub fn sheets(p: &Project) -> Vec<Sheet> {
    let one = |panel| Place { panel, x: 0.0, y: 0.0, upside_down: false, record: None };
    match p.kind {
        Kind::CardQuarter if p.panels.len() >= 3 => {
            // Letter, portrait: fold top half back, then fold in half.
            let (pw, ph) = (p.width, p.height);
            vec![Sheet {
                w: pw * 2.0,
                h: ph * 2.0,
                places: vec![
                    Place { panel: 0, x: pw, y: ph, upside_down: false, record: None },
                    Place { panel: 2, x: 0.0, y: ph, upside_down: false, record: None },
                    Place { panel: 1, x: 0.0, y: 0.0, upside_down: true, record: None },
                ],
            }]
        }
        Kind::CardHalf if p.panels.len() >= 3 && p.width > p.height => {
            // A wide card folds at the top: back (upside down) above the
            // fold, front below it; the inside goes on the lower half of the
            // other side.
            let (pw, ph) = (p.width, p.height);
            vec![
                Sheet { w: pw, h: ph * 2.0, places: vec![Place { panel: 2, x: 0.0, y: 0.0, upside_down: true, record: None }, Place { panel: 0, x: 0.0, y: ph, upside_down: false, record: None }] },
                Sheet { w: pw, h: ph * 2.0, places: vec![Place { panel: 1, x: 0.0, y: ph, upside_down: false, record: None }] },
            ]
        }
        Kind::NoteCard if p.panels.len() >= 3 => {
            // Two note cards to a letter sheet, to cut apart. A wide card
            // folds at the top (each card is a 5½ × 8½ half sheet: back
            // upside down above the fold, front below); a tall card folds
            // at the side (back left, front right).
            let (pw, ph) = (p.width, p.height);
            if pw > ph {
                let halves = [0.0, pw];
                vec![
                    Sheet { w: pw * 2.0, h: ph * 2.0, places: halves.iter().flat_map(|&x| [Place { panel: 2, x, y: 0.0, upside_down: true, record: None }, Place { panel: 0, x, y: ph, upside_down: false, record: None }]).collect() },
                    Sheet { w: pw * 2.0, h: ph * 2.0, places: halves.iter().map(|&x| Place { panel: 1, x, y: ph, upside_down: false, record: None }).collect() },
                ]
            } else {
                let halves = [0.0, ph];
                vec![
                    Sheet { w: pw * 2.0, h: ph * 2.0, places: halves.iter().flat_map(|&y| [Place { panel: 2, x: 0.0, y, upside_down: false, record: None }, Place { panel: 0, x: pw, y, upside_down: false, record: None }]).collect() },
                    Sheet { w: pw * 2.0, h: ph * 2.0, places: halves.iter().map(|&y| Place { panel: 1, x: pw, y, upside_down: false, record: None }).collect() },
                ]
            }
        }
        Kind::CardHalf if p.panels.len() >= 3 => {
            let (pw, ph) = (p.width, p.height);
            vec![
                Sheet { w: pw * 2.0, h: ph, places: vec![Place { panel: 2, x: 0.0, y: 0.0, upside_down: false, record: None }, Place { panel: 0, x: pw, y: 0.0, upside_down: false, record: None }] },
                Sheet { w: pw * 2.0, h: ph, places: vec![Place { panel: 1, x: pw, y: 0.0, upside_down: false, record: None }] },
            ]
        }
        Kind::Label | Kind::BusinessCard | Kind::Postcard => {
            // The chosen sheet (labels 30-up, business cards 10-up...).
            let l = p.sheet.clone().or_else(|| crate::kinds::sheet_presets(p.kind).first().map(|x| x.layout.clone()));
            let Some(l) = l else { return (0..p.panels.len()).map(|i| Sheet { w: p.width, h: p.height, places: vec![one(i)] }).collect() };
            let grid = |panel_of: &dyn Fn(usize) -> usize| {
                let mut places = Vec::new();
                for r in 0..l.rows {
                    for c in 0..l.cols {
                        let n = (r * l.cols + c) as usize;
                        places.push(Place { panel: panel_of(n), x: l.left + c as f32 * (p.width + l.gap_x), y: l.top + r as f32 * (p.height + l.gap_y), upside_down: false, record: None });
                    }
                }
                Sheet { w: l.paper_w, h: l.paper_h, places }
            };
            if p.kind == Kind::Label {
                // Each panel can be its own label; cycle through them.
                let n = p.panels.len().max(1);
                vec![grid(&|k| k % n)]
            } else {
                // Cards: one sheet of fronts, then one of backs.
                (0..p.panels.len().max(1)).map(|i| grid(&|_| i)).collect()
            }
        }
        Kind::Banner => {
            // Sheets overlap by the printer margin at each join so nothing
            // is lost; trim and tape along the dashed join lines.
            let b = p.banner.clone().unwrap_or(crate::model::BannerSetup { sheets: (p.width / 10.5).ceil().max(1.0) as u32, tall: false });
            let (seg, _) = Project::banner_segment(&b, crate::model::BANNER_MARGIN);
            let (sw, sh): (f32, f32) = if b.tall { (8.5, 11.0) } else { (11.0, 8.5) };
            let n = (p.width / seg).ceil().max(1.0) as usize;
            (0..n)
                .map(|i| Sheet { w: sw, h: sh.max(p.height), places: vec![Place { panel: 0, x: crate::model::BANNER_MARGIN - i as f32 * seg, y: 0.0, upside_down: false, record: None }] })
                .collect()
        }
        _ => (0..p.panels.len()).map(|i| Sheet { w: p.width, h: p.height, places: vec![one(i)] }).collect(),
    }
}

/// Sheets with print-time choices applied (label start position).
pub fn arranged_sheets(p: &Project, o: &PrintOptions) -> Vec<Sheet> {
    if !o.merge.is_empty() && crate::merge::has_fields(p) {
        return merged_sheets(p, o);
    }
    let mut all = sheets(p);
    if o.start_at > 1
        && matches!(p.kind, Kind::Label | Kind::BusinessCard)
        && let Some(first) = all.first_mut()
    {
        let skip = (o.start_at as usize - 1).min(first.places.len().saturating_sub(1));
        first.places.drain(..skip);
    }
    all
}

/// Address merge: labels take one person each, filling the sheet in order
/// (from the "start at" position); anything else prints its whole set of
/// sheets once per person.
fn merged_sheets(p: &Project, o: &PrintOptions) -> Vec<Sheet> {
    let base = sheets(p);
    if p.kind == Kind::Label
        && let Some(grid) = base.first()
    {
        let slots: Vec<&Place> = grid.places.iter().collect();
        let mut out: Vec<Sheet> = Vec::new();
        let mut slot = (o.start_at.max(1) as usize - 1).min(slots.len().saturating_sub(1));
        for c in &o.merge {
            if out.is_empty() || slot >= slots.len() {
                if !out.is_empty() {
                    slot = 0;
                }
                out.push(Sheet { w: grid.w, h: grid.h, places: Vec::new() });
            }
            let s = slots[slot];
            if let Some(sheet) = out.last_mut() {
                sheet.places.push(Place { panel: s.panel, x: s.x, y: s.y, upside_down: s.upside_down, record: Some(c.clone()) });
            }
            slot += 1;
        }
        return out;
    }
    o.merge
        .iter()
        .flat_map(|c| base.iter().map(move |s| Sheet { w: s.w, h: s.h, places: s.places.iter().map(|pl| Place { panel: pl.panel, x: pl.x, y: pl.y, upside_down: pl.upside_down, record: Some(c.clone()) }).collect() }))
        .collect()
}

pub fn render_sheet(p: &Project, sheet: &Sheet, fonts: &FontBook, pics: &PictureCache, ppi: f32) -> Option<Pixmap> {
    let mut out = Pixmap::new((sheet.w * ppi).round() as u32, (sheet.h * ppi).round() as u32)?;
    out.fill(tiny_skia::Color::WHITE);
    let mut cache: Vec<Option<Pixmap>> = (0..p.panels.len()).map(|_| None).collect();
    for pl in &sheet.places {
        // A merged copy is drawn with that person's details filled in.
        let merged = pl.record.as_ref().and_then(|c| render_panel(&crate::merge::fill(p, c), pl.panel, fonts, pics, ppi));
        if merged.is_none() && cache[pl.panel].is_none() {
            cache[pl.panel] = render_panel(p, pl.panel, fonts, pics, ppi);
        }
        let Some(img) = merged.as_ref().or(cache[pl.panel].as_ref()) else { continue };
        let (x, y) = (pl.x * ppi, pl.y * ppi);
        let t = if pl.upside_down {
            Transform::from_row(-1.0, 0.0, 0.0, -1.0, x + img.width() as f32, y + img.height() as f32)
        } else {
            Transform::from_translate(x, y)
        };
        out.draw_pixmap(0, 0, img.as_ref(), &PixmapPaint::default(), t, None);
    }
    Some(out)
}

pub fn save_png(p: &Project, panel: usize, fonts: &FontBook, pics: &PictureCache, path: &Path, ppi: f32) -> Result<(), String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
    let bytes = picture_bytes(p, panel, fonts, pics, &ext, ppi)?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// One panel as a picture file: PNG, or JPEG / BMP when `ext` asks for it.
pub fn picture_bytes(p: &Project, panel: usize, fonts: &FontBook, pics: &PictureCache, ext: &str, ppi: f32) -> Result<Vec<u8>, String> {
    let pm = render_panel(p, panel, fonts, pics, ppi).ok_or("Nothing to save.")?;
    let rgba = crate::raster::pixmap_to_rgba(&pm);
    let img = image::RgbaImage::from_raw(pm.width(), pm.height(), rgba).ok_or("Picture error")?;
    let fmt = image::ImageFormat::from_extension(ext).unwrap_or(image::ImageFormat::Png);
    let mut out = std::io::Cursor::new(Vec::new());
    if fmt == image::ImageFormat::Jpeg || fmt == image::ImageFormat::Bmp {
        image::DynamicImage::ImageRgba8(img).to_rgb8().write_to(&mut out, fmt).map_err(|e| e.to_string())?;
    } else {
        img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    }
    Ok(out.into_inner())
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintOptions {
    pub printer: Option<String>,
    pub copies: u32,
    /// Sheets to print (0-based); empty = all.
    pub sheets: Vec<usize>,
    pub dpi: f32,
    pub gray: bool,
    /// Shrink each sheet to sit inside the printable margin.
    pub fit_margin: Option<f32>,
    pub cut_marks: bool,
    pub two_sided: bool,
    /// Flip along the short edge instead of the long one.
    pub short_edge: bool,
    /// Mirror each sheet (iron-on transfers).
    pub mirror: bool,
    /// Move everything by this much (inches) to correct a printer's offset.
    pub nudge: (f32, f32),
    /// Last sheet first (for printers that stack face up).
    pub reverse: bool,
    /// Labels and cards: start at this position on the first sheet (1 = first).
    pub start_at: u32,
    /// Address merge: print a copy for each of these people.
    pub merge: Vec<crate::settings::Contact>,
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            printer: None,
            copies: 1,
            sheets: Vec::new(),
            dpi: 200.0,
            gray: false,
            fit_margin: None,
            cut_marks: false,
            two_sided: false,
            short_edge: false,
            mirror: false,
            nudge: (0.0, 0.0),
            reverse: false,
            start_at: 1,
            merge: Vec::new(),
        }
    }
}

/// A sheet as it will come out of the printer, with the options applied.
pub fn finished_sheet(p: &Project, sheet: &Sheet, fonts: &FontBook, pics: &PictureCache, o: &PrintOptions) -> Option<Pixmap> {
    let mut pm = render_sheet(p, sheet, fonts, pics, o.dpi)?;
    if o.mirror || o.nudge != (0.0, 0.0) {
        let mut out = Pixmap::new(pm.width(), pm.height())?;
        out.fill(tiny_skia::Color::WHITE);
        let (nx, ny) = (o.nudge.0 * o.dpi, o.nudge.1 * o.dpi);
        let t = if o.mirror { Transform::from_row(-1.0, 0.0, 0.0, 1.0, pm.width() as f32 + nx, ny) } else { Transform::from_translate(nx, ny) };
        out.draw_pixmap(0, 0, pm.as_ref(), &PixmapPaint::default(), t, None);
        pm = out;
    }
    if let Some(m) = o.fit_margin {
        let mpx = m * o.dpi;
        let k = ((pm.width() as f32 - 2.0 * mpx) / pm.width() as f32).min((pm.height() as f32 - 2.0 * mpx) / pm.height() as f32).max(0.5);
        let mut out = Pixmap::new(pm.width(), pm.height())?;
        out.fill(tiny_skia::Color::WHITE);
        let ox = (pm.width() as f32 * (1.0 - k)) / 2.0;
        let oy = (pm.height() as f32 * (1.0 - k)) / 2.0;
        out.draw_pixmap(0, 0, pm.as_ref(), &PixmapPaint { quality: tiny_skia::FilterQuality::Bilinear, ..Default::default() }, Transform::from_row(k, 0.0, 0.0, k, ox, oy), None);
        pm = out;
    }
    if o.cut_marks {
        let mut pb = tiny_skia::PathBuilder::new();
        let len = 0.15 * o.dpi;
        for pl in &sheet.places {
            let (x0, y0) = (pl.x * o.dpi, pl.y * o.dpi);
            let (x1, y1) = (x0 + p.width * o.dpi, y0 + p.height * o.dpi);
            for (x, y, dx, dy) in [(x0, y0, -1.0, -1.0), (x1, y0, 1.0, -1.0), (x0, y1, -1.0, 1.0), (x1, y1, 1.0, 1.0)] {
                pb.move_to(x + dx * 2.0, y);
                pb.line_to(x + dx * len, y);
                pb.move_to(x, y + dy * 2.0);
                pb.line_to(x, y + dy * len);
            }
        }
        if let Some(path) = pb.finish() {
            let paint = tiny_skia::Paint { shader: tiny_skia::Shader::SolidColor(tiny_skia::Color::from_rgba8(120, 120, 120, 255)), anti_alias: true, ..Default::default() };
            pm.stroke_path(&path, &paint, &tiny_skia::Stroke { width: 1.0, ..Default::default() }, Transform::identity(), None);
        }
    }
    if o.gray {
        for px in pm.data_mut().chunks_exact_mut(4) {
            let y = (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32) as u8;
            px[0] = y;
            px[1] = y;
            px[2] = y;
        }
    }
    Some(pm)
}

/// A PDF with one page per sheet, each page a picture of the sheet.
pub fn save_pdf(p: &Project, fonts: &FontBook, pics: &PictureCache, path: &Path) -> Result<(), String> {
    save_pdf_with(p, fonts, pics, path, &PrintOptions::default())
}

pub fn save_pdf_with(p: &Project, fonts: &FontBook, pics: &PictureCache, path: &Path, o: &PrintOptions) -> Result<(), String> {
    std::fs::write(path, pdf_bytes(p, fonts, pics, o)?).map_err(|e| e.to_string())
}

/// The PDF itself, for saving or downloading.
pub fn pdf_bytes(p: &Project, fonts: &FontBook, pics: &PictureCache, o: &PrintOptions) -> Result<Vec<u8>, String> {
    let all = arranged_sheets(p, o);
    let mut picked: Vec<&Sheet> = all.iter().enumerate().filter(|(i, _)| o.sheets.is_empty() || o.sheets.contains(i)).map(|(_, s)| s).collect();
    if o.reverse {
        picked.reverse();
    }
    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let tree = Ref::new(2);
    let mut next = 3;
    let mut kids = Vec::new();
    let mut parts = Vec::new();
    for s in picked {
        let page = Ref::new(next);
        let img = Ref::new(next + 1);
        let content = Ref::new(next + 2);
        next += 3;
        kids.push(page);
        let pm = finished_sheet(p, s, fonts, pics, o).ok_or("Couldn't draw the page.")?;
        let rgb: Vec<u8> = pm.data().chunks_exact(4).flat_map(|c| [c[0], c[1], c[2]]).collect();
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&rgb).map_err(|e| e.to_string())?;
        let packed = z.finish().map_err(|e| e.to_string())?;
        parts.push((page, img, content, s.w * 72.0, s.h * 72.0, pm.width(), pm.height(), packed));
    }
    if parts.is_empty() {
        return Err("No sheets were chosen to print.".into());
    }
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree).kids(kids.iter().copied()).count(kids.len() as i32);
    for (page, img, content, w, h, iw, ih, packed) in parts {
        let mut pg = pdf.page(page);
        pg.media_box(Rect::new(0.0, 0.0, w, h));
        pg.parent(tree);
        pg.contents(content);
        pg.resources().x_objects().pair(Name(b"Sheet"), img);
        pg.finish();
        let mut x = pdf.image_xobject(img, &packed);
        x.filter(Filter::FlateDecode);
        x.width(iw as i32);
        x.height(ih as i32);
        x.color_space().device_rgb();
        x.bits_per_component(8);
        x.finish();
        let mut c = Content::new();
        c.save_state();
        c.transform([w, 0.0, 0.0, h, 0.0, 0.0]);
        c.x_object(Name(b"Sheet"));
        c.restore_state();
        pdf.stream(content, &c.finish());
    }
    Ok(pdf.finish())
}

/// Printers the system knows about (empty if it can't tell).
pub fn printers() -> Vec<String> {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))]
    {
        let out = std::process::Command::new("lpstat").arg("-e").output();
        if let Ok(o) = out {
            return String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("powershell").args(["-NoProfile", "-Command", "Get-Printer | Select-Object -ExpandProperty Name"]).creation_flags(0x0800_0000).output();
        if let Ok(o) = out {
            return String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        }
    }
    Vec::new()
}

/// A page with rulers in from each edge, to see what the printer can reach.
pub fn test_page() -> Project {
    use crate::model::*;
    let mut p = Project::new(crate::kinds::Kind::Sign, false);
    let mut add = |frame: Frame, content: Content, stroke: Rgba| {
        let id = p.fresh_id();
        let mut it = Item::new(id, frame, content);
        it.stroke = stroke;
        it.stroke_pt = 0.75;
        p.panels[0].items.push(it);
    };
    for (k, inset) in [0.125f32, 0.25, 0.375, 0.5].iter().enumerate() {
        let c = [Rgba::hex("#d1495b"), Rgba::hex("#f2b631"), Rgba::hex("#2a9d8f"), Rgba::hex("#2f8fd8")][k];
        add(Frame::new(*inset, *inset, 8.5 - 2.0 * inset, 11.0 - 2.0 * inset), Content::Shape { outline: Outline::Rectangle }, c);
    }
    let look = TextLook { size: 14.0, align: Align::Center, ..Default::default() };
    add(
        Frame::new(1.0, 4.2, 6.5, 2.4),
        Content::Text {
            text: "PrintCraft printer test\n\nThe colored boxes are 1/8, 1/4, 3/8 and 1/2 inch from the edge of the paper (red, gold, green, blue).\nThe first box that prints all the way around shows your printer's margin. Set it in Settings > Printable area.".into(),
            look,
            shape: None,
            fit: false,
            runs: Vec::new(),
        },
        Rgba::NONE,
    );
    p
}

/// Send to the printer with options; falls back to the PDF viewer.
pub fn print_with(p: &Project, fonts: &FontBook, pics: &PictureCache, o: &PrintOptions) -> Result<String, String> {
    // A browser can't reach the printer directly: hand over the PDF.
    #[cfg(target_arch = "wasm32")]
    {
        crate::platform::save_file("PrintCraft printout.pdf", &[("PDF", &["pdf"])], |_| pdf_bytes(p, fonts, pics, o))?;
        return Ok("Your printout was downloaded as a PDF. Open it and print from there.".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path: PathBuf = std::env::temp_dir().join(format!("printcraft-print-{}.pdf", std::process::id()));
        save_pdf_with(p, fonts, pics, &path, o)?;
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))]
        {
            let mut cmd = std::process::Command::new("lp");
            cmd.arg("-n").arg(o.copies.max(1).to_string());
            if let Some(pr) = &o.printer {
                cmd.arg("-d").arg(pr);
            }
            if o.two_sided {
                cmd.arg("-o").arg(if o.short_edge { "sides=two-sided-short-edge" } else { "sides=two-sided-long-edge" });
            }
            cmd.arg("-o").arg("fit-to-page=false");
            if cmd.arg(&path).status().map(|s| s.success()).unwrap_or(false) {
                return Ok(format!("Sent to {}.", o.printer.clone().unwrap_or_else(|| "the printer".into())));
            }
            let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
            let _ = std::process::Command::new(opener).arg(&path).spawn();
            Ok("Your printout opened in the document viewer; print it from there.".into())
        }
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("cmd").args(["/C", "start", ""]).arg(&path).creation_flags(0x0800_0000).spawn();
            Ok("Your printout opened in the PDF viewer; print it from there.".into())
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd", target_os = "windows")))]
        {
            Ok(format!("Saved for printing at {}", path.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Contact;

    fn person(name: &str) -> Contact {
        Contact { name: name.into(), street: "1 Main St".into(), city: "Mesa".into(), region: "AZ".into(), postcode: "85201".into(), ..Default::default() }
    }

    #[test]
    fn labels_take_one_person_each() {
        let mut p = Project::new(Kind::Label, false);
        let look = crate::model::TextLook::default();
        p.panels[0].items.push(crate::model::Item::new(1, crate::model::Frame::new(0.1, 0.1, 2.0, 0.8), crate::model::Content::Text { text: "«Address»".into(), look, shape: None, fit: false, runs: Vec::new() }));
        let people: Vec<Contact> = (0..35).map(|i| person(&format!("Person {i}"))).collect();
        // 30 to a sheet, starting at the 3rd label: 28 on the first sheet, 7 on the second.
        let o = PrintOptions { merge: people, start_at: 3, ..Default::default() };
        let s = arranged_sheets(&p, &o);
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].places.len(), s[1].places.len()), (28, 7));
        assert_eq!(s[1].places[0].record.as_ref().map(|c| c.name.as_str()), Some("Person 28"));
    }

    #[test]
    fn envelopes_print_once_per_person() {
        let mut p = Project::new(Kind::Envelope, true);
        let look = crate::model::TextLook::default();
        p.panels[0].items.push(crate::model::Item::new(1, crate::model::Frame::new(4.0, 2.0, 4.0, 1.0), crate::model::Content::Text { text: "«Name»".into(), look, shape: None, fit: false, runs: Vec::new() }));
        let o = PrintOptions { merge: vec![person("A"), person("B"), person("C")], ..Default::default() };
        assert_eq!(arranged_sheets(&p, &o).len(), 3);
        // Without merge fields, choosing people changes nothing.
        p.panels[0].items.clear();
        assert_eq!(arranged_sheets(&p, &o).len(), 1);
    }

    #[test]
    fn note_cards_print_two_to_a_letter_sheet() {
        for landscape in [true, false] {
            let p = Project::new(Kind::NoteCard, landscape);
            let s = sheets(&p);
            assert_eq!(s.len(), 2);
            let (w, h) = (s[0].w, s[0].h);
            assert!((w.max(h) - 11.0).abs() < 1e-3 && (w.min(h) - 8.5).abs() < 1e-3, "{w} × {h}");
            assert_eq!((s[0].places.len(), s[1].places.len()), (4, 2));
        }
    }
}
