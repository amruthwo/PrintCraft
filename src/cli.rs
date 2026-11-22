//! Command-line helpers: convert a project to a picture or PDF without
//! opening the window, and make a contact sheet of a content pack.

use std::path::{Path, PathBuf};

use crate::fonts::FontBook;
use crate::kinds::Kind;
use crate::model::Project;
use crate::raster::PictureCache;

pub fn load(path: &Path) -> Result<Project, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext == "pcraft" || bytes.starts_with(b"{") {
        Project::from_json(&bytes)
    } else {
        let kind = Kind::from_legacy_extension(&ext).unwrap_or(Kind::Sign);
        crate::legacy::sig::read(&bytes).map(|l| crate::legacy::sig::to_project(l, kind))
    }
}

pub fn export(src: PathBuf, dst: PathBuf) -> Result<String, String> {
    let p = load(&src)?;
    let fonts = FontBook::new();
    // Give the background font search a moment so named fonts are found.
    for _ in 0..50 {
        if !fonts.scanning() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let pics = PictureCache::default();
    let ext = dst.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
    if ext == "pdf" {
        crate::export::save_pdf(&p, &fonts, &pics, &dst)?;
    } else if ext == "pcraft" {
        std::fs::write(&dst, p.to_json()?).map_err(|e| e.to_string())?;
    } else {
        crate::export::save_png(&p, 0, &fonts, &pics, &dst, 100.0)?;
    }
    let items: usize = p.panels.iter().map(|x| x.items.len()).sum();
    Ok(format!(
        "{} {:.2}x{:.2} in, {} panel(s), {} item(s), {} picture(s)\n{}",
        p.kind.label(),
        p.width,
        p.height,
        p.panels.len(),
        items,
        p.assets.len(),
        p.notes.join("\n")
    ))
}

pub fn pack_sheet(args: &[PathBuf]) -> Result<String, String> {
    let (out, folders) = args.split_last().ok_or("usage: --pack-sheet <folders...> <out.png>")?;
    let mut lib = crate::legacy::collection::Library::scan(folders);
    let ready: Vec<usize> = (0..lib.items.len()).filter(|i| lib.packs[lib.items[*i].pack].data.is_some()).collect();
    let cell = 140u32;
    let cols = 8u32;
    let picks: Vec<usize> = ready.iter().step_by((ready.len() / 32).max(1)).take(32).copied().collect();
    let rows = (picks.len() as u32).div_ceil(cols).max(1);
    let mut sheet = tiny_skia::Pixmap::new(cell * cols, cell * rows).ok_or("size")?;
    sheet.fill(tiny_skia::Color::WHITE);
    let mut ok = 0;
    for (n, i) in picks.iter().enumerate() {
        let item = lib.items[*i].clone();
        let Ok(bytes) = lib.load(&item) else { continue };
        let Some(pm) = crate::raster::decode_picture(&bytes) else { continue };
        let k = (cell as f32 - 10.0) / pm.width().max(pm.height()) as f32;
        let (x, y) = ((n as u32 % cols) * cell + 5, (n as u32 / cols) * cell + 5);
        sheet.draw_pixmap(0, 0, pm.as_ref(), &tiny_skia::PixmapPaint { quality: tiny_skia::FilterQuality::Bilinear, ..Default::default() }, tiny_skia::Transform::from_row(k, 0.0, 0.0, k, x as f32, y as f32), None);
        ok += 1;
    }
    sheet.save_png(out).map_err(|e| e.to_string())?;
    Ok(format!("{} packs, {} items ({} with data); drew {ok} of {}", lib.packs.len(), lib.items.len(), ready.len(), picks.len()))
}

/// Every ready-made design, each page side by side, on one picture (for
/// checking how they look): `printcraft --design-sheet out.png`.
pub fn design_sheet(out: PathBuf) -> Result<String, String> {
    let fonts = FontBook::new();
    for _ in 0..100 {
        if !fonts.scanning() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let pics = PictureCache::default();
    let designs = crate::templates::all();
    let row_h = 260u32;
    let rows: Vec<Vec<tiny_skia::Pixmap>> = designs
        .iter()
        .map(|d| {
            let p = (d.build)();
            let ppi = (row_h - 20) as f32 / p.height.max(p.width / 4.0);
            (0..p.panels.len()).filter_map(|i| crate::raster::render_panel(&p, i, &fonts, &pics, ppi)).collect()
        })
        .collect();
    let width = rows.iter().map(|r| r.iter().map(|pm| pm.width() + 12).sum::<u32>() + 12).max().unwrap_or(100);
    let mut sheet = tiny_skia::Pixmap::new(width, row_h * rows.len() as u32).ok_or("too big")?;
    sheet.fill(tiny_skia::Color::from_rgba8(225, 225, 225, 255));
    for (k, r) in rows.iter().enumerate() {
        let mut x = 12;
        for pm in r {
            sheet.draw_pixmap(x, (k as u32 * row_h + 10) as i32, pm.as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
            x += pm.width() as i32 + 12;
        }
    }
    sheet.save_png(&out).map_err(|e| e.to_string())?;
    Ok(format!("{} designs drawn to {}", designs.len(), out.display()))
}
