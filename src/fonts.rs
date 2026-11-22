//! Font catalogue: the faces built into the app plus every font found on the
//! computer (and ones downloaded through "Online > Online Fonts"). Faces are read
//! from disk only when a design first uses them.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;

pub const DEFAULT_FAMILY: &str = "PrintCraft Sans";
const BUILTIN_MONO: &str = "PrintCraft Typewriter";
/// Bundled handwriting face (Dancing Script, SIL Open Font License).
pub const SCRIPT_FAMILY: &str = "Dancing Script";
const SCRIPT_REGULAR: &[u8] = include_bytes!("../assets/fonts/DancingScript-Regular.ttf");
const SCRIPT_BOLD: &[u8] = include_bytes!("../assets/fonts/DancingScript-Bold.ttf");
/// The default family's four faces (Nunito, SIL Open Font License).
const SANS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Nunito-Regular.ttf"),
    include_bytes!("../assets/fonts/Nunito-Bold.ttf"),
    include_bytes!("../assets/fonts/Nunito-Italic.ttf"),
    include_bytes!("../assets/fonts/Nunito-BoldItalic.ttf"),
];

#[derive(Clone, Default, Debug)]
pub struct Faces {
    pub regular: Option<PathBuf>,
    pub bold: Option<PathBuf>,
    pub italic: Option<PathBuf>,
    pub bold_italic: Option<PathBuf>,
}

#[derive(Default)]
struct Shared {
    families: BTreeMap<String, Faces>,
    scanned: bool,
}

#[derive(Clone)]
pub struct FontBook {
    shared: Arc<Mutex<Shared>>,
    registered: Arc<Mutex<HashSet<String>>>,
    bytes_cache: Arc<Mutex<BTreeMap<String, Arc<Vec<u8>>>>>,
}

/// Key egui knows a face by: family plus style suffix.
pub fn face_key(family: &str, bold: bool, italic: bool) -> String {
    format!("{family}|{}{}", if bold { "b" } else { "" }, if italic { "i" } else { "" })
}

pub fn user_font_dir() -> Option<PathBuf> {
    crate::platform::data_dir().map(|d| d.join("fonts"))
}

impl FontBook {
    pub fn new() -> Self {
        let book = Self {
            shared: Arc::new(Mutex::new(Shared::default())),
            registered: Arc::new(Mutex::new(HashSet::new())),
            bytes_cache: Arc::new(Mutex::new(BTreeMap::new())),
        };
        book.rescan();
        book
    }

    /// Look for fonts in the background; the list fills in when done.
    pub fn rescan(&self) {
        let shared = self.shared.clone();
        crate::platform::spawn(move || {
            let mut found: BTreeMap<String, Faces> = BTreeMap::new();
            for dir in font_dirs() {
                walk(&dir, 0, &mut |p| index_file(p, &mut found));
            }
            let mut s = shared.lock().unwrap();
            s.families = found;
            s.scanned = true;
        });
    }

    pub fn families(&self) -> Vec<String> {
        let mut v = vec![DEFAULT_FAMILY.to_string(), SCRIPT_FAMILY.to_string(), BUILTIN_MONO.to_string()];
        let s = self.shared.lock().unwrap();
        v.extend(s.families.iter().filter(|(k, f)| (f.regular.is_some() || f.bold.is_some()) && k.as_str() != SCRIPT_FAMILY).map(|(k, _)| k.clone()));
        v
    }

    pub fn scanning(&self) -> bool {
        !self.shared.lock().unwrap().scanned
    }

    /// The bytes of the best face for a style, falling back to the built-in face.
    pub fn face_bytes(&self, family: &str, bold: bool, italic: bool) -> Arc<Vec<u8>> {
        let key = face_key(family, bold, italic);
        if let Some(b) = self.bytes_cache.lock().unwrap().get(&key) {
            return b.clone();
        }
        let path = {
            let s = self.shared.lock().unwrap();
            s.families.get(family).and_then(|f| {
                let pick = match (bold, italic) {
                    (true, true) => f.bold_italic.clone().or(f.bold.clone()),
                    (true, false) => f.bold.clone(),
                    (false, true) => f.italic.clone(),
                    _ => None,
                };
                pick.or(f.regular.clone()).or(f.bold.clone())
            })
        };
        let bytes = path
            .and_then(|p| std::fs::read(p).ok())
            .unwrap_or_else(|| {
                if family == DEFAULT_FAMILY {
                    SANS[(bold as usize) | ((italic as usize) << 1)].to_vec()
                } else if family == SCRIPT_FAMILY {
                    if bold { SCRIPT_BOLD.to_vec() } else { SCRIPT_REGULAR.to_vec() }
                } else if family == BUILTIN_MONO {
                    epaint_default_fonts::HACK_REGULAR.to_vec()
                } else {
                    epaint_default_fonts::UBUNTU_LIGHT.to_vec()
                }
            });
        let bytes = Arc::new(bytes);
        self.bytes_cache.lock().unwrap().insert(key, bytes.clone());
        bytes
    }

    /// Does the family really have this style, or must it be drawn slanted
    /// or thickened from the regular face?
    pub fn has_face(&self, family: &str, bold: bool, italic: bool) -> bool {
        if family == DEFAULT_FAMILY || (!bold && !italic) {
            return true;
        }
        if family == SCRIPT_FAMILY {
            return !italic;
        }
        let s = self.shared.lock().unwrap();
        s.families.get(family).is_some_and(|f| match (bold, italic) {
            (true, true) => f.bold_italic.is_some(),
            (true, false) => f.bold.is_some(),
            _ => f.italic.is_some(),
        })
    }

    /// Make sure egui can draw this face; returns the egui family to use.
    pub fn ensure(&self, ctx: &egui::Context, family: &str, bold: bool, italic: bool) -> egui::FontFamily {
        let key = face_key(family, bold, italic);
        let fam = egui::FontFamily::Name(key.clone().into());
        if self.registered.lock().unwrap().contains(&key) {
            return fam;
        }
        let bytes = self.face_bytes(family, bold, italic);
        // egui reads the whole font table again on set_fonts, so collect all
        // registered faces into one definition.
        let mut reg = self.registered.lock().unwrap();
        reg.insert(key.clone());
        let mut defs = base_definitions();
        let fallback: Vec<String> = defs.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
        for k in reg.iter() {
            let (f, style) = k.split_once('|').unwrap_or((k, ""));
            let b = if k == &key { bytes.clone() } else { self.face_bytes(f, style.contains('b'), style.contains('i')) };
            defs.font_data.insert(k.clone(), Arc::new(egui::FontData::from_owned((*b).clone())));
            let mut chain = vec![k.clone()];
            chain.extend(fallback.iter().cloned());
            defs.families.insert(egui::FontFamily::Name(k.clone().into()), chain);
        }
        ctx.set_fonts(defs);
        fam
    }
}

pub fn base_definitions() -> egui::FontDefinitions {
    egui::FontDefinitions::default()
}

fn font_dirs() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("C:\\Windows\\Fonts"),
    ];
    if let Some(home) = crate::platform::home_dir() {
        v.push(home.join(".fonts"));
        v.push(home.join(".local/share/fonts"));
        v.push(home.join("Library/Fonts"));
    }
    if let Some(d) = user_font_dir() {
        v.push(d);
    }
    v
}

fn walk(dir: &Path, depth: usize, f: &mut impl FnMut(&Path)) {
    if depth > 6 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, depth + 1, f);
        } else {
            f(&p);
        }
    }
}

fn index_file(p: &Path, found: &mut BTreeMap<String, Faces>) {
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext != "ttf" && ext != "otf" {
        return;
    }
    let Ok(bytes) = std::fs::read(p) else { return };
    let Ok(face) = ttf_parser::Face::parse(&bytes, 0) else { return };
    let mut family = None;
    let mut typographic = None;
    for n in face.names() {
        if n.name_id == ttf_parser::name_id::TYPOGRAPHIC_FAMILY && typographic.is_none() {
            typographic = n.to_string();
        }
        if n.name_id == ttf_parser::name_id::FAMILY && family.is_none() {
            family = n.to_string();
        }
    }
    let Some(name) = typographic.or(family) else { return };
    let bold = face.is_bold() || face.weight().to_number() >= 600;
    let italic = face.is_italic() || face.is_oblique();
    // Skip very light/heavy variants as the "regular" pick.
    let w = face.weight().to_number();
    let entry = found.entry(name).or_default();
    let slot = match (bold, italic) {
        (true, true) => &mut entry.bold_italic,
        (true, false) => &mut entry.bold,
        (false, true) => &mut entry.italic,
        (false, false) => &mut entry.regular,
    };
    let better = slot.is_none() || (!bold && w == 400);
    if better {
        *slot = Some(p.to_path_buf());
    }
}
