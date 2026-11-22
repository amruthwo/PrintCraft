//! .CDT content packs (art and photo collections), read straight from the
//! folders the person points PrintCraft at (desktop only). A pack is `name.ini` + `name.IDT` (items) +
//! `name.CDT` (data); the data file may live in a different folder (the
//! separate "contents" download), so several folders can be searched.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::{le16, le32};

#[derive(Clone, Debug)]
pub struct PackItem {
    pub pack: usize,
    pub index: usize,
    pub file_name: String,
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct Pack {
    pub name: String,
    pub kind: String,
    pub data: Option<PathBuf>,
    pub items: usize,
}

#[derive(Default)]
pub struct Library {
    pub packs: Vec<Pack>,
    pub items: Vec<PackItem>,
    offsets: HashMap<usize, Vec<u64>>,
}

impl Library {
    /// Index every pack found in `folders` (non-recursive plus one level).
    pub fn scan(folders: &[PathBuf]) -> Library {
        let mut lib = Library::default();
        let mut data_files: HashMap<String, PathBuf> = HashMap::new();
        let mut inis = Vec::new();
        for f in folders {
            for p in files_in(f) {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
                match ext.as_str() {
                    "cdt" => {
                        data_files.insert(stem, p.clone());
                    }
                    "ini" => inis.push(p.clone()),
                    _ => {}
                }
            }
        }
        // Data files whose index (name.ini + name.IDT) isn't next to them:
        // look in nearby folders, since installs often keep the two apart.
        let have: std::collections::HashSet<String> = inis.iter().filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase())).collect();
        let missing: Vec<String> = data_files.keys().filter(|k| !have.contains(*k)).cloned().collect();
        if !missing.is_empty() {
            let mut roots: Vec<PathBuf> = Vec::new();
            for f in folders {
                if let Some(r) = f.ancestors().nth(3) {
                    if !roots.iter().any(|x| r.starts_with(x)) {
                        roots.retain(|x| !x.starts_with(r));
                        roots.push(r.to_path_buf());
                    }
                }
            }
            for r in roots {
                for p in files_in(&r) {
                    let is_ini = p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("ini"));
                    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
                    if is_ini && missing.contains(&stem) && !inis.iter().any(|i| i.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case(&stem))) {
                        inis.push(p);
                    }
                }
            }
        }
        for ini in inis {
            let stem = ini.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let idt = ini.with_extension("IDT");
            let idt = if idt.exists() { idt } else { ini.with_extension("idt") };
            let Ok(bytes) = std::fs::read(&idt) else { continue };
            let (name, kind) = read_ini(&ini, &stem);
            let pack_no = lib.packs.len();
            let mut count = 0;
            let mut at = 0usize;
            while at + 6 <= bytes.len() {
                let (Some(len), Some(n_keys)) = (le32(&bytes, at), le16(&bytes, at + 4)) else { break };
                let body_start = at + 6;
                let body_end = body_start + len as usize;
                if body_end > bytes.len() {
                    break;
                }
                let rest = bytes.get(body_start + 4 * n_keys as usize..body_end).unwrap_or(&[]);
                let mut parts = rest.split(|b| *b == 0);
                let file_name = String::from_utf8_lossy(parts.next().unwrap_or(&[])).to_string();
                let raw_title = String::from_utf8_lossy(parts.next().unwrap_or(&[])).to_string();
                lib.items.push(PackItem { pack: pack_no, index: count, file_name, title: clean_title(&raw_title) });
                count += 1;
                at = body_end;
            }
            lib.packs.push(Pack { name, kind, data: data_files.remove(&stem.to_ascii_lowercase()), items: count });
        }
        // Data files with no index next to them: still usable, numbered.
        let mut orphans: Vec<(String, PathBuf)> = data_files.into_iter().collect();
        orphans.sort();
        for (stem, path) in orphans {
            let Ok(offs) = walk_offsets(&path) else { continue };
            if offs.is_empty() {
                continue;
            }
            let pack_no = lib.packs.len();
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or(&stem).replace('_', " ");
            for i in 0..offs.len() {
                lib.items.push(PackItem { pack: pack_no, index: i, file_name: String::new(), title: format!("{name} #{}", i + 1) });
            }
            lib.packs.push(Pack { name, kind: String::new(), data: Some(path), items: offs.len() });
            lib.offsets.insert(pack_no, offs);
        }
        lib
    }

    /// The bytes of one item: JPEG/BMP/PNG or a Windows metafile.
    pub fn load(&mut self, item: &PackItem) -> Result<Vec<u8>, String> {
        let pack = self.packs.get(item.pack).ok_or("unknown pack")?;
        let path = pack.data.clone().ok_or_else(|| format!("The picture data for “{}” wasn't found. Add the folder holding its .CDT file.", pack.name))?;
        if !self.offsets.contains_key(&item.pack) {
            let offs = walk_offsets(&path).map_err(|e| e.to_string())?;
            self.offsets.insert(item.pack, offs);
        }
        let off = *self.offsets[&item.pack].get(item.index).ok_or("This picture is missing from its pack.")?;
        let mut f = File::open(&path).map_err(|e| e.to_string())?;
        f.seek(SeekFrom::Start(off)).map_err(|e| e.to_string())?;
        let mut head = [0u8; 10];
        f.read_exact(&mut head).map_err(|e| e.to_string())?;
        let full = u32::from_le_bytes([head[0], head[1], head[2], head[3]]) as usize;
        let stored = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        let method = u16::from_le_bytes([head[8], head[9]]);
        if stored > 200_000_000 || full > 400_000_000 {
            return Err("This picture is too large.".into());
        }
        let mut data = vec![0; stored];
        f.read_exact(&mut data).map_err(|e| e.to_string())?;
        match method {
            0 => Ok(data),
            2 => {
                let mut out = Vec::with_capacity(full);
                flate2::read::DeflateDecoder::new(&data[..]).read_to_end(&mut out).map_err(|e| e.to_string())?;
                Ok(out)
            }
            m => Err(format!("This picture is stored in a way PrintCraft can't read yet (method {m}).")),
        }
    }

    pub fn search(&self, words: &str, pack: Option<usize>) -> Vec<usize> {
        let words: Vec<String> = words.split_whitespace().map(|w| w.to_lowercase()).collect();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, it)| pack.is_none_or(|p| it.pack == p))
            .filter(|(_, it)| {
                let t = it.title.to_lowercase();
                words.iter().all(|w| t.contains(w.as_str()))
            })
            .map(|(i, _)| i)
            .collect()
    }
}

fn walk_offsets(path: &Path) -> std::io::Result<Vec<u64>> {
    let mut f = File::open(path)?;
    let size = f.metadata()?.len();
    let mut offs = Vec::new();
    let mut at = 0u64;
    let mut head = [0u8; 10];
    while at + 10 <= size {
        f.seek(SeekFrom::Start(at))?;
        f.read_exact(&mut head)?;
        let stored = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as u64;
        if stored == 0 || at + 10 + stored > size {
            break;
        }
        offs.push(at);
        at += 10 + stored;
    }
    Ok(offs)
}

/// Files in a folder and its subfolders (a few levels down).
fn files_in(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, v: &mut Vec<PathBuf>) {
        if depth > 6 {
            return;
        }
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, depth + 1, v);
                } else {
                    v.push(p);
                }
            }
        }
    }
    let mut v = Vec::new();
    walk(dir, 0, &mut v);
    v
}

fn read_ini(path: &Path, stem: &str) -> (String, String) {
    let text = std::fs::read(path).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
    let mut name = stem.to_string();
    let mut kind = String::new();
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("Friendly Name=") {
            name = v.trim().to_string();
        }
        if let Some(v) = line.strip_prefix("Type=") {
            kind = v.trim().to_string();
        }
    }
    (name, kind)
}

/// Titles end with a space and one capital letter (a category flag).
fn clean_title(t: &str) -> String {
    let t = t.trim_end();
    let b = t.as_bytes();
    if b.len() > 2 && b[b.len() - 2] == b' ' && b[b.len() - 1].is_ascii_uppercase() {
        return t[..t.len() - 2].trim_end().to_string();
    }
    t.to_string()
}
