//! Online pictures and fonts. Picture search happens in Google Images in
//! the person's own browser (see dialogs::google_images_url); this module
//! downloads a copied picture address and fetches open-licensed fonts from
//! Fontsource.

use std::sync::mpsc::{Receiver, Sender, channel};

const AGENT: &str = concat!("PrintCraft/", env!("CARGO_PKG_VERSION"), " (open-source design app; https://github.com/amruthwo/PrintCraft)");

#[derive(Clone, Debug)]
pub struct WebPicture {
    pub title: String,
    pub thumb_url: String,
    pub full_url: String,
    pub credit: String,
}

pub enum Reply {
    Full(Result<(WebPicture, Vec<u8>), String>),
    Fonts(Result<Vec<(String, String)>, String>),
    FontSaved(Result<String, String>),
    FontPreview(String, Vec<u8>),
    /// A picture chosen in the Google Images window: its address (or the
    /// picture itself as a data: address) and Google's description.
    WebPick { src: String, alt: String },
}

pub struct Online {
    tx: Sender<Reply>,
    pub rx: Receiver<Reply>,
    pub busy: bool,
    /// The Google Images window, while it's open.
    web_window: Option<std::process::Child>,
    /// What was searched for, for the picture's name and credit.
    pub web_words: String,
    /// Also keep pictures chosen there in My Art.
    pub web_keep: bool,
}

impl Default for Online {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx, busy: false, web_window: None, web_words: String::new(), web_keep: false }
    }
}

fn get(url: &str) -> Result<Vec<u8>, String> {
    get_as(url, AGENT)
}

/// Picture hosts found through Google often turn away anything that
/// doesn't look like a web browser.
const BROWSER: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";

fn get_as(url: &str, agent: &str) -> Result<Vec<u8>, String> {
    crate::platform::http_get(url, agent, 30).map_err(|e| friendly(&e))
}

fn friendly(e: &str) -> String {
    if e == crate::platform::DESKTOP_ONLY {
        e.into()
    } else if e.contains("dns") || e.contains("resolve") || e.contains("onnect") {
        "PrintCraft couldn't reach the internet. Check the connection and try again.".into()
    } else {
        let code = e.split("status code ").nth(1).unwrap_or("").trim_end_matches(|c: char| !c.is_ascii_digit()).to_string();
        if code.is_empty() {
            "The picture service didn't answer. Please try again in a moment.".into()
        } else {
            format!("The picture service is busy or unavailable right now (code {code}). Please try again in a moment.")
        }
    }
}

impl Drop for Online {
    fn drop(&mut self) {
        if let Some(mut c) = self.web_window.take() {
            let _ = c.kill();
        }
    }
}

impl Online {
    /// Open Google Images in PrintCraft's own window (a small companion
    /// program beside PrintCraft that uses the computer's web engine).
    /// Fails when the companion or the web engine isn't there.
    #[cfg(target_arch = "wasm32")]
    pub fn google_window(&mut self, _url: &str, _ctx: &eframe::egui::Context) -> Result<(), String> {
        Err(crate::platform::DESKTOP_ONLY.into())
    }

    /// Open Google Images in PrintCraft's own window (see above).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn google_window(&mut self, url: &str, ctx: &eframe::egui::Context) -> Result<(), String> {
        use std::io::BufRead;
        use std::process::{Command, Stdio};
        if let Some(mut c) = self.web_window.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let helper = exe.with_file_name(if cfg!(windows) { "printcraft-web.exe" } else { "printcraft-web" });
        if !helper.exists() {
            return Err("The Google Images window isn't installed beside PrintCraft.".into());
        }
        let mut child = Command::new(&helper).arg(url).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
        let out = child.stdout.take().ok_or("no output")?;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line)
                    && let Some(src) = v["src"].as_str()
                {
                    let _ = tx.send(Reply::WebPick { src: src.to_string(), alt: v["alt"].as_str().unwrap_or("").to_string() });
                    ctx.request_repaint();
                }
            }
        });
        // If the web engine is missing the companion stops at once.
        std::thread::sleep(std::time::Duration::from_millis(400));
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("The Google Images window couldn't start ({status})."));
        }
        self.web_window = Some(child);
        Ok(())
    }

    pub fn fetch_picture(&mut self, pic: WebPicture, ctx: &eframe::egui::Context) {
        self.busy = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        crate::platform::spawn(move || {
            let r = get_as(&pic.full_url, BROWSER).and_then(|b| {
                image::guess_format(&b).map_err(|_| "That address didn't give a picture.".to_string())?;
                Ok((pic, b))
            });
            let _ = tx.send(Reply::Full(r));
            ctx.request_repaint();
        });
    }

    pub fn list_fonts(&mut self, ctx: &eframe::egui::Context) {
        self.busy = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        crate::platform::spawn(move || {
            let r = get("https://api.fontsource.org/v1/fonts").and_then(|b| {
                let v: serde_json::Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
                Ok(v.as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter(|f| f["subsets"].as_array().is_some_and(|s| s.iter().any(|x| x == "latin")))
                    .filter_map(|f| Some((f["id"].as_str()?.to_string(), f["family"].as_str()?.to_string())))
                    .collect())
            });
            let _ = tx.send(Reply::Fonts(r));
            ctx.request_repaint();
        });
    }

    /// Fetch just the regular face of a font to show what it looks like.
    pub fn font_preview(&self, id: String, ctx: &eframe::egui::Context) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        crate::platform::spawn(move || {
            let url = format!("https://cdn.jsdelivr.net/fontsource/fonts/{id}@latest/latin-400-normal.ttf");
            let bytes = get(&url).unwrap_or_default();
            let _ = tx.send(Reply::FontPreview(id, bytes));
            ctx.request_repaint();
        });
    }

    /// Download regular, bold, italic and bold-italic faces where offered.
    pub fn install_font(&mut self, id: String, family: String, ctx: &eframe::egui::Context) {
        self.busy = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        crate::platform::spawn(move || {
            let r = (|| {
                let dir = crate::fonts::user_font_dir().ok_or("No place to keep fonts.")?;
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let mut got = 0;
                for (weight, style) in [(400, "normal"), (700, "normal"), (400, "italic"), (700, "italic")] {
                    let url = format!("https://cdn.jsdelivr.net/fontsource/fonts/{id}@latest/latin-{weight}-{style}.ttf");
                    if let Ok(b) = get(&url)
                        && ttf_parser::Face::parse(&b, 0).is_ok()
                    {
                        std::fs::write(dir.join(format!("{id}-{weight}-{style}.ttf")), b).map_err(|e| e.to_string())?;
                        got += 1;
                    }
                }
                if got == 0 { Err(format!("{family} couldn't be downloaded.")) } else { Ok(family.clone()) }
            })();
            let _ = tx.send(Reply::FontSaved(r));
            ctx.request_repaint();
        });
    }
}
