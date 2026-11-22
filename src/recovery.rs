//! Crash safety: every couple of minutes each changed tab is written to a
//! recovery folder; a clean save or close removes it. On the next start,
//! anything left behind is offered back.

use std::path::PathBuf;

use crate::model::Project;

fn dir() -> Option<PathBuf> {
    crate::platform::data_dir().map(|d| d.join("recovery"))
}

pub fn write(id: u64, name: &str, doc: &Project) {
    let Some(d) = dir() else { return };
    let _ = std::fs::create_dir_all(&d);
    let Ok(bytes) = doc.to_json() else { return };
    let tmp = d.join(format!("{id}.tmp"));
    if std::fs::write(&tmp, bytes).is_ok() {
        let _ = std::fs::rename(&tmp, d.join(format!("{id}.pcraft")));
        let _ = std::fs::write(d.join(format!("{id}.name")), name);
    }
}

pub fn forget(id: u64) {
    if let Some(d) = dir() {
        let _ = std::fs::remove_file(d.join(format!("{id}.pcraft")));
        let _ = std::fs::remove_file(d.join(format!("{id}.name")));
    }
}

/// Designs left from a run that didn't close normally: (id, name, design).
pub fn leftovers() -> Vec<(u64, String, Project)> {
    let Some(d) = dir() else { return Vec::new() };
    let Ok(rd) = std::fs::read_dir(&d) else { return Vec::new() };
    let mut v = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("pcraft") {
            continue;
        }
        let Some(id) = p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse().ok()) else { continue };
        let name = std::fs::read_to_string(p.with_extension("name")).unwrap_or_else(|_| "Untitled".into());
        if let Some(doc) = std::fs::read(&p).ok().and_then(|b| Project::from_json(&b).ok()) {
            v.push((id, name, doc));
        }
    }
    v
}
