//! The few things that differ between the desktop app and the browser
//! version: choosing and saving files, the clipboard, settings storage,
//! background work, web addresses and downloads from the internet.
//!
//! The desktop side uses native dialogs, files and threads. The browser
//! side uses an upload button, downloads and the browser's local storage;
//! what a browser can't do (printers, folders, content packs, the Google
//! Images window, internet downloads from other sites) reports that it
//! needs the desktop app.

use std::path::PathBuf;
use std::sync::Mutex;

use eframe::egui;

/// True in the browser version.
pub const WEB: bool = cfg!(target_arch = "wasm32");

/// Said when something only works in the desktop app.
pub const DESKTOP_ONLY: &str = "This needs the PrintCraft desktop app (it can't be done from a web browser).";

/// What a chosen file is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    /// A design to open in a tab.
    Project,
    /// A picture to put on the page.
    Picture,
    /// Decide by the file's extension (dropped files).
    Any,
}

/// A file the person chose or dropped in.
pub struct Opened {
    pub name: String,
    pub bytes: Vec<u8>,
    /// Where it lives on the computer (desktop only).
    pub path: Option<PathBuf>,
}

impl Opened {
    pub fn extension(&self) -> String {
        std::path::Path::new(&self.name).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
    }
    pub fn stem(&self) -> String {
        std::path::Path::new(&self.name).file_stem().and_then(|s| s.to_str()).unwrap_or("Picture").to_string()
    }
}

/// Where a saved file went (each version of PrintCraft uses one kind).
#[allow(dead_code)]
pub enum Saved {
    File(PathBuf),
    Downloaded,
}

static PICKED: Mutex<Vec<(Want, Opened)>> = Mutex::new(Vec::new());

fn deliver(want: Want, file: Opened) {
    PICKED.lock().unwrap().push((want, file));
}

/// Files chosen since the last call (the browser delivers them a moment
/// after the choice, so the app collects them every frame).
pub fn take_picked() -> Vec<(Want, Opened)> {
    std::mem::take(&mut *PICKED.lock().unwrap())
}

/// Run work in the background: a thread on the desktop. In the browser it
/// runs straight away; the only work sent here there is quick or reports
/// that it needs the desktop app.
pub fn spawn(f: impl FnOnce() + Send + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(f);
    #[cfg(target_arch = "wasm32")]
    f();
}

/// Milliseconds since 1970.
pub fn now_ms() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
}

pub use imp::*;

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    use std::path::Path;

    fn dirs() -> Option<directories::ProjectDirs> {
        directories::ProjectDirs::from("org", "PrintCraft", "PrintCraft")
    }

    /// PrintCraft's own data folder (fonts, recovery copies, web cookies).
    pub fn data_dir() -> Option<PathBuf> {
        dirs().map(|d| d.data_dir().to_path_buf())
    }

    pub fn home_dir() -> Option<PathBuf> {
        directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
    }

    /// The person's Pictures folder.
    pub fn pictures_dir() -> Option<PathBuf> {
        directories::UserDirs::new().and_then(|u| u.picture_dir().map(|p| p.to_path_buf()))
    }

    /// The person's Downloads folder.
    pub fn downloads_dir() -> Option<PathBuf> {
        directories::UserDirs::new().and_then(|u| u.download_dir().map(|p| p.to_path_buf()))
    }

    fn settings_file() -> Option<PathBuf> {
        dirs().map(|d| d.config_dir().join("settings.json"))
    }

    pub fn load_settings() -> Option<Vec<u8>> {
        settings_file().and_then(|f| std::fs::read(f).ok())
    }

    pub fn store_settings(bytes: &[u8]) {
        if let Some(f) = settings_file() {
            if let Some(dir) = f.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(f, bytes);
        }
    }

    fn opener() -> &'static str {
        if cfg!(target_os = "windows") {
            "explorer"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        }
    }

    /// Open a web address in the person's browser.
    pub fn open_url(url: &str) {
        let _ = std::process::Command::new(opener()).arg(url).spawn();
    }

    /// Show a folder (or open a file) with the computer's own program.
    pub fn open_path(path: &Path) {
        let _ = std::process::Command::new(opener()).arg(path).spawn();
    }

    /// Ask for a file to open; it arrives through `take_picked`.
    pub fn pick_file(want: Want, filter: &str, exts: &[&str], _ctx: &egui::Context) {
        let mut d = rfd::FileDialog::new();
        if !exts.is_empty() {
            // Accept upper-case extensions too (older files often use them).
            let mut all: Vec<String> = exts.iter().map(|e| e.to_string()).collect();
            all.extend(exts.iter().map(|e| e.to_uppercase()));
            d = d.add_filter(filter, &all).add_filter("All files", &["*"]);
        }
        if let Some(p) = d.pick_file() {
            match std::fs::read(&p) {
                Ok(bytes) => {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
                    deliver(want, Opened { name, bytes, path: Some(p) });
                }
                Err(e) => eprintln!("couldn't read {}: {e}", p.display()),
            }
        }
    }

    pub fn pick_folder() -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }

    /// Ask where to save, then write what `make` produces for the chosen
    /// file's extension. `Ok(None)` when the person cancelled.
    pub fn save_file(name: &str, filters: &[(&str, &[&str])], make: impl FnOnce(&str) -> Result<Vec<u8>, String>) -> Result<Option<Saved>, String> {
        let mut d = rfd::FileDialog::new().set_file_name(name);
        for (label, exts) in filters {
            d = d.add_filter(*label, exts);
        }
        let Some(mut path) = d.save_file() else { return Ok(None) };
        if path.extension().is_none()
            && let Some(first) = filters.first().and_then(|f| f.1.first())
        {
            path.set_extension(first);
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let bytes = make(&ext)?;
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        Ok(Some(Saved::File(path)))
    }

    /// Write straight to a known file (Save on a design that has a file).
    pub fn write_path(path: &Path, bytes: &[u8]) -> Result<(), String> {
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    pub fn clipboard_text() -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }

    /// A picture on the clipboard: width, height, RGBA.
    pub fn clipboard_image() -> Option<(u32, u32, Vec<u8>)> {
        let img = arboard::Clipboard::new().ok()?.get_image().ok()?;
        Some((img.width as u32, img.height as u32, img.bytes.into_owned()))
    }

    pub fn set_clipboard_text(_ctx: &egui::Context, text: &str) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(text);
        }
    }

    /// Download from the internet (at most 60 MB), with a time limit.
    pub fn http_get(url: &str, agent: &str, seconds: u64) -> Result<Vec<u8>, String> {
        let resp = ureq::get(url).set("User-Agent", agent).timeout(std::time::Duration::from_secs(seconds)).call().map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut std::io::Read::take(resp.into_reader(), 60_000_000), &mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }

    /// The reasons a JSON request can fail.
    pub enum PostError {
        /// The service answered with an error code and maybe a JSON body.
        Status(u16, Option<serde_json::Value>),
        /// The service couldn't be reached.
        Unreachable,
    }

    /// Send JSON and read the JSON answer.
    pub fn http_post_json(url: &str, auth: Option<&str>, body: serde_json::Value, seconds: u64) -> Result<serde_json::Value, PostError> {
        let mut req = ureq::post(url).set("Content-Type", "application/json").timeout(std::time::Duration::from_secs(seconds));
        if let Some(a) = auth {
            req = req.set("Authorization", &format!("Bearer {a}"));
        }
        match req.send_json(body) {
            Ok(r) => r.into_json().map_err(|_| PostError::Unreachable),
            Err(ureq::Error::Status(code, r)) => Err(PostError::Status(code, r.into_json::<serde_json::Value>().ok())),
            Err(_) => Err(PostError::Unreachable),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use std::path::Path;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    const SETTINGS_KEY: &str = "printcraft.settings";

    fn window() -> Option<web_sys::Window> {
        web_sys::window()
    }

    pub fn data_dir() -> Option<PathBuf> {
        None
    }

    pub fn home_dir() -> Option<PathBuf> {
        None
    }

    pub fn pictures_dir() -> Option<PathBuf> {
        None
    }

    pub fn downloads_dir() -> Option<PathBuf> {
        None
    }

    fn storage() -> Option<web_sys::Storage> {
        window()?.local_storage().ok().flatten()
    }

    /// Settings live in the browser's local storage for this site.
    pub fn load_settings() -> Option<Vec<u8>> {
        storage()?.get_item(SETTINGS_KEY).ok().flatten().map(String::into_bytes)
    }

    pub fn store_settings(bytes: &[u8]) {
        if let (Some(s), Ok(text)) = (storage(), std::str::from_utf8(bytes)) {
            let _ = s.set_item(SETTINGS_KEY, text);
        }
    }

    /// Open a web address in a new browser tab.
    pub fn open_url(url: &str) {
        if let Some(w) = window() {
            let _ = w.open_with_url_and_target(url, "_blank");
        }
    }

    pub fn open_path(_path: &Path) {}

    /// Show the browser's file chooser; the file arrives through
    /// `take_picked` once the browser has read it.
    pub fn pick_file(want: Want, _filter: &str, exts: &[&str], ctx: &egui::Context) {
        let Some(doc) = window().and_then(|w| w.document()) else { return };
        let Ok(input) = doc.create_element("input").map(|e| e.unchecked_into::<web_sys::HtmlInputElement>()) else { return };
        input.set_type("file");
        if !exts.is_empty() {
            let accept: Vec<String> = exts.iter().map(|e| format!(".{e}")).collect();
            input.set_accept(&accept.join(","));
        }
        let ctx = ctx.clone();
        let picked = input.clone();
        let on_change = Closure::<dyn FnMut()>::new(move || {
            let Some(file) = picked.files().and_then(|l| l.get(0)) else { return };
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(bytes) = read_file(&file).await {
                    deliver(want, Opened { name: file.name(), bytes, path: None });
                    ctx.request_repaint();
                }
            });
        });
        input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
        on_change.forget();
        input.click();
    }

    async fn read_file(file: &web_sys::File) -> Option<Vec<u8>> {
        let buf = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await.ok()?;
        Some(js_sys::Uint8Array::new(&buf).to_vec())
    }

    /// Pictures pasted with Ctrl+V (for example after Copy Image on another
    /// site) arrive like chosen files. Text pastes are left to egui.
    pub fn listen_for_pasted_pictures(ctx: &egui::Context) {
        let Some(doc) = window().and_then(|w| w.document()) else { return };
        let ctx = ctx.clone();
        let on_paste = Closure::<dyn FnMut(web_sys::ClipboardEvent)>::new(move |e: web_sys::ClipboardEvent| {
            let Some(files) = e.clipboard_data().and_then(|d| d.files()) else { return };
            let mut took = false;
            for i in 0..files.length() {
                let Some(file) = files.get(i) else { continue };
                if !file.type_().starts_with("image/") {
                    continue;
                }
                took = true;
                let ctx = ctx.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(bytes) = read_file(&file).await {
                        let name = if file.name().is_empty() || file.name() == "image.png" { "Pasted picture.png".to_string() } else { file.name() };
                        deliver(Want::Picture, Opened { name, bytes, path: None });
                        ctx.request_repaint();
                    }
                });
            }
            if took {
                e.prevent_default();
                e.stop_propagation();
            }
        });
        let _ = doc.add_event_listener_with_callback_and_bool("paste", on_paste.as_ref().unchecked_ref(), true);
        on_paste.forget();
    }

    /// Files dropped on the page: read them, then hand them over.
    pub fn take_dropped(files: Vec<egui::DroppedFileHandle>, ctx: &egui::Context) {
        for f in files {
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let name = f.path().file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
                if let Ok(bytes) = f.bytes_async().await {
                    deliver(Want::Any, Opened { name, bytes, path: None });
                    ctx.request_repaint();
                }
            });
        }
    }

    pub fn pick_folder() -> Option<PathBuf> {
        None
    }

    /// Browsers can't ask where to save: the file is downloaded under its
    /// suggested name.
    pub fn save_file(name: &str, _filters: &[(&str, &[&str])], make: impl FnOnce(&str) -> Result<Vec<u8>, String>) -> Result<Option<Saved>, String> {
        let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let bytes = make(&ext)?;
        download(name, &bytes)?;
        Ok(Some(Saved::Downloaded))
    }

    pub fn write_path(path: &Path, bytes: &[u8]) -> Result<(), String> {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("My Project.pcraft");
        download(name, bytes)
    }

    fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
        let fail = |_| "The browser didn't allow the download.".to_string();
        let doc = window().and_then(|w| w.document()).ok_or("No page to download from.")?;
        let parts = js_sys::Array::new();
        parts.push(&js_sys::Uint8Array::from(bytes));
        let opts = web_sys::BlobPropertyBag::new();
        opts.set_type("application/octet-stream");
        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(fail)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(fail)?;
        let a = doc.create_element("a").map_err(fail)?.unchecked_into::<web_sys::HtmlAnchorElement>();
        a.set_href(&url);
        a.set_download(name);
        a.click();
        let _ = web_sys::Url::revoke_object_url(&url);
        Ok(())
    }

    /// The browser only hands over clipboard contents with a paste
    /// (Ctrl+V), which egui handles for text boxes.
    pub fn clipboard_text() -> Option<String> {
        None
    }

    pub fn clipboard_image() -> Option<(u32, u32, Vec<u8>)> {
        None
    }

    pub fn set_clipboard_text(ctx: &egui::Context, text: &str) {
        ctx.copy_text(text.to_string());
    }

    pub fn http_get(_url: &str, _agent: &str, _seconds: u64) -> Result<Vec<u8>, String> {
        Err(DESKTOP_ONLY.into())
    }

    #[allow(dead_code)]
    pub enum PostError {
        Status(u16, Option<serde_json::Value>),
        Unreachable,
    }

    pub fn http_post_json(_url: &str, _auth: Option<&str>, _body: serde_json::Value, _seconds: u64) -> Result<serde_json::Value, PostError> {
        Err(PostError::Unreachable)
    }
}
