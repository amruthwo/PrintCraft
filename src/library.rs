//! "My Art": pictures the person keeps for reuse, as plain files in a
//! folder they can open and tidy themselves.

use std::path::PathBuf;

pub fn dir() -> PathBuf {
    let base = crate::platform::pictures_dir().or_else(crate::platform::data_dir).unwrap_or_default();
    base.join("PrintCraft Art")
}

fn clean(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_alphanumeric() || " -_".contains(c) { c } else { ' ' }).collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.is_empty() { "Picture".into() } else { s.chars().take(60).collect() }
}

pub fn save(name: &str, bytes: &[u8], credit: &str) -> Result<PathBuf, String> {
    if crate::platform::WEB {
        return Err(crate::platform::DESKTOP_ONLY.into());
    }
    let d = dir();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let ext = match image::guess_format(bytes) {
        Ok(image::ImageFormat::Jpeg) => "jpg",
        Ok(image::ImageFormat::Gif) => "gif",
        Ok(image::ImageFormat::Bmp) => "bmp",
        _ => "png",
    };
    let base = clean(name);
    let mut path = d.join(format!("{base}.{ext}"));
    let mut n = 2;
    while path.exists() {
        path = d.join(format!("{base} {n}.{ext}"));
        n += 1;
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    if !credit.is_empty() {
        let _ = std::fs::write(path.with_extension("credit.txt"), credit);
    }
    Ok(path)
}

pub fn list() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir())
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "wmf")))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

pub fn open_folder() {
    let d = dir();
    let _ = std::fs::create_dir_all(&d);
    crate::platform::open_path(&d);
}
