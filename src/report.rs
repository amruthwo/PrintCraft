//! "Report a Problem or Suggest an Idea": a friendly form that fills in a
//! GitHub issue (with the details a fix needs) and gathers the attachments.
//!
//! GitHub doesn't let other programs attach files to a new issue, so the
//! picture of the window, the project and the details are saved together in
//! a folder that opens beside the browser, ready to drag into the form.
//! Sending needs a (free) GitHub account; Copy the Report is there for email.

use std::path::PathBuf;

use eframe::egui::{self, RichText, TextureHandle, vec2};

use crate::app::App;

const ISSUES_NEW: &str = "https://github.com/amruthwo/PrintCraft/issues/new";

pub struct ReportState {
    /// 0 something isn't working, 1 an idea, 2 a question.
    kind: u8,
    title: String,
    what: String,
    steps: String,
    include_shot: bool,
    include_project: bool,
    shot: Option<egui::ColorImage>,
    shot_tex: Option<TextureHandle>,
    details: String,
    install: crate::update::Install,
    /// After sending: the folder with the attachments, if one was made.
    sent: Option<Option<PathBuf>>,
}

impl ReportState {
    pub fn new(app: &App) -> Self {
        let install = crate::update::detect();
        let d = &app.doc;
        let details = format!(
            "PrintCraft {} · {} · {} · installed as: {}\nProject: {}, {:.2} × {:.2} in, {} page(s), {} item(s) on this page",
            crate::update::current_version(),
            os_details(),
            crate::update::arch(),
            install.report_label(),
            d.kind.label(),
            d.width,
            d.height,
            d.panels.len(),
            app.page().items.len(),
        );
        Self {
            kind: 0,
            title: String::new(),
            what: String::new(),
            steps: String::new(),
            include_shot: true,
            include_project: false,
            shot: app.report_shot.lock().unwrap().take(),
            shot_tex: None,
            details,
            install,
            sent: None,
        }
    }
}

/// The operating system and its version, as plainly as can be found.
fn os_details() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        return web_sys::window().and_then(|w| w.navigator().user_agent().ok()).map(|ua| format!("Web browser ({ua})")).unwrap_or_else(|| "Web browser".into());
    }
    #[cfg(target_os = "linux")]
    {
        let pretty = std::fs::read_to_string("/etc/os-release").ok().and_then(|t| t.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string())));
        return pretty.unwrap_or_else(|| "Linux".into());
    }
    #[cfg(target_os = "macos")]
    {
        let v = std::process::Command::new("sw_vers").arg("-productVersion").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
        return format!("macOS {v}").trim().to_string();
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("cmd").args(["/C", "ver"]).creation_flags(0x0800_0000).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
        // "Microsoft Windows [Version 10.0.22631.4317]": build 22000 and up is Windows 11.
        let ver = out.split("Version ").nth(1).map(|v| v.trim().trim_end_matches(']').to_string()).unwrap_or_default();
        let build: u32 = ver.split('.').nth(2).and_then(|b| b.parse().ok()).unwrap_or(0);
        let name = if build >= 22000 { "Windows 11" } else if build > 0 { "Windows 10" } else { "Windows" };
        return if ver.is_empty() { name.into() } else { format!("{name} ({ver})") };
    }
    #[allow(unreachable_code)]
    std::env::consts::OS.to_string()
}

fn os_label() -> &'static str {
    if crate::platform::WEB {
        "Web browser"
    } else {
        match std::env::consts::OS {
            "windows" => "Windows",
            "macos" => "macOS",
            "linux" => "Linux",
            _ => "Other",
        }
    }
}

/// Percent-encode for a web address.
fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Keep long text from making the address too long for the browser.
fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { s.chars().take(n).collect::<String>() + "…" }
}

impl ReportState {
    fn title_or_default(&self) -> String {
        let t = self.title.trim();
        if !t.is_empty() {
            return t.to_string();
        }
        let first = self.what.lines().next().unwrap_or("").trim();
        if !first.is_empty() {
            return clip(first, 80);
        }
        ["A problem in PrintCraft", "An idea for PrintCraft", "A question about PrintCraft"][self.kind as usize].into()
    }

    /// The new-issue address with the right form and its fields filled in
    /// (the field names match .github/ISSUE_TEMPLATE).
    fn issue_url(&self, attached: bool) -> String {
        let note = if attached { "\n\n(Attachments: drag the files from the PrintCraft report folder into this box.)" } else { "" };
        let mut q: Vec<(&str, String)> = vec![("title", self.title_or_default())];
        match self.kind {
            0 => {
                q.push(("template", "bug_report.yml".into()));
                q.push(("what", clip(&self.what, 2500)));
                q.push(("steps", clip(&self.steps, 2000)));
                q.push(("install", self.install.report_label().into()));
                q.push(("files", format!("{}{note}", self.details)));
            }
            1 => {
                q.push(("template", "feature_request.yml".into()));
                q.push(("idea", clip(&self.what, 3000)));
                q.push(("why", clip(&self.steps, 2000)));
                q.push(("files", format!("{}{note}", self.details)));
            }
            _ => {
                q.push(("template", "question.yml".into()));
                q.push(("question", clip(&self.what, 3500)));
                q.push(("files", format!("{}{note}", self.details)));
            }
        }
        q.push(("version", crate::update::current_version().into()));
        q.push(("os", os_label().into()));
        q.push(("os_details", clip(&os_details(), 200)));
        let query: Vec<String> = q.iter().map(|(k, v)| format!("{k}={}", enc(v))).collect();
        format!("{ISSUES_NEW}?{}", query.join("&"))
    }

    /// The whole report as plain text, for email.
    fn as_text(&self) -> String {
        let kind = ["Problem", "Idea", "Question"][self.kind as usize];
        let mut t = format!("{kind}: {}\n\n{}\n", self.title_or_default(), self.what.trim());
        if !self.steps.trim().is_empty() {
            t += &format!("\n{}\n{}\n", if self.kind == 0 { "What I was doing:" } else { "Why it would help:" }, self.steps.trim());
        }
        t += &format!("\n{}\n", self.details);
        t
    }

    fn shot_png(&self) -> Option<Vec<u8>> {
        let s = self.shot.as_ref()?;
        let rgba: Vec<u8> = s.pixels.iter().flat_map(|c| c.to_srgba_unmultiplied()).collect();
        let img = image::RgbaImage::from_raw(s.size[0] as u32, s.size[1] as u32, rgba)?;
        Some(crate::photo::to_png(&image::DynamicImage::ImageRgba8(img)))
    }

    /// Save the attachments together (desktop: a folder in Downloads that
    /// opens; browser: downloads). Returns the folder.
    fn save_attachments(&self, app: &App) -> Result<Option<PathBuf>, String> {
        let mut files: Vec<(String, Vec<u8>)> = vec![("details.txt".into(), self.as_text().into_bytes())];
        if self.include_shot
            && let Some(png) = self.shot_png()
        {
            files.push(("window.png".into(), png));
        }
        if self.include_project {
            files.push((format!("{}.pcraft", app.display_name()), app.doc.to_json()?));
        }
        if crate::platform::WEB {
            for (name, bytes) in files.iter().skip(1) {
                crate::platform::save_file(&format!("PrintCraft report {name}"), &[], |_| Ok(bytes.clone()))?;
            }
            return Ok(None);
        }
        let base = crate::platform::downloads_dir().or_else(crate::platform::data_dir).ok_or("No folder to save the report in.")?;
        let dir = base.join(format!("PrintCraft report {}", crate::platform::now_ms() / 1000));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for (name, bytes) in &files {
            std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
        }
        crate::platform::open_path(&dir);
        Ok(Some(dir))
    }
}

pub fn window(ctx: &egui::Context, app: &mut App, st: &mut ReportState, open: &mut bool, close: &mut bool) {
    if st.shot_tex.is_none()
        && let Some(s) = &st.shot
    {
        st.shot_tex = Some(ctx.load_texture("report-shot", s.clone(), egui::TextureOptions::LINEAR));
    }
    crate::dialogs::window("Report a Problem or Suggest an Idea").open(open).default_size([620.0, 640.0]).show(ctx, |ui| {
        if let Some(folder) = &st.sent {
            sent_page(ui, st, folder.clone(), close);
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(RichText::new("What would you like to tell us?").size(17.0));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (k, (icon, name)) in [("🐞", "Something isn't working"), ("💡", "I have an idea"), ("❓", "A question")].iter().enumerate() {
                    let b = egui::Button::new(RichText::new(format!("{icon}  {name}")).size(14.0)).min_size(vec2(180.0, 36.0)).selected(st.kind == k as u8);
                    if ui.add(b).clicked() {
                        st.kind = k as u8;
                    }
                }
            });
            ui.add_space(8.0);
            let (what_label, what_hint, steps_label, steps_hint) = match st.kind {
                0 => ("What happened?", "What did you expect, and what happened instead?", "What were you doing just before?", "For example: I opened my birthday card, clicked the picture, then chose Photo Editor…"),
                1 => ("What's your idea?", "What would you like PrintCraft to do?", "How would it help you?", "For example: I make place cards for parties and…"),
                _ => ("What's your question?", "Ask anything about using PrintCraft.", "", ""),
            };
            ui.label(RichText::new("In a few words, what's it about?").strong());
            ui.add(egui::TextEdit::singleline(&mut st.title).hint_text("For example: Pictures turn black when I print").desired_width(f32::INFINITY));
            ui.add_space(6.0);
            ui.label(RichText::new(what_label).strong());
            ui.add(egui::TextEdit::multiline(&mut st.what).hint_text(what_hint).desired_rows(4).desired_width(f32::INFINITY));
            if !steps_label.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(steps_label).strong());
                ui.add(egui::TextEdit::multiline(&mut st.steps).hint_text(steps_hint).desired_rows(3).desired_width(f32::INFINITY));
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Include").strong());
            ui.horizontal(|ui| {
                ui.add_enabled(st.shot.is_some(), egui::Checkbox::new(&mut st.include_shot, "A picture of the PrintCraft window"));
                if let Some(t) = &st.shot_tex {
                    ui.add(egui::Image::new(t).max_size(vec2(160.0, 100.0)));
                }
            });
            ui.checkbox(&mut st.include_project, "The project I'm working on (.pcraft file)");
            ui.label(RichText::new("These details are always included, to help find the problem:").weak().size(12.0));
            ui.label(RichText::new(&st.details).monospace().size(11.5));
            ui.add_space(8.0);
            ui.label(RichText::new("Reports go to PrintCraft's page on GitHub, where anyone can read them. Please leave out private information. You'll need a free GitHub account to send one.").weak().size(12.0));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ready = !st.what.trim().is_empty() || !st.title.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new(RichText::new("Continue on GitHub ➜").size(15.0)).min_size(vec2(190.0, 34.0))).on_hover_text("Opens the report on GitHub with everything filled in").clicked() {
                    match st.save_attachments(app) {
                        Ok(folder) => {
                            crate::platform::open_url(&st.issue_url(folder.is_some()));
                            st.sent = Some(folder);
                        }
                        Err(e) => app.notify(format!("Couldn't save the report's files: {e}"), ctx),
                    }
                }
                if ui.button("Copy the Report").on_hover_text("To paste into an email instead").clicked() {
                    crate::platform::set_clipboard_text(ctx, &st.as_text());
                    app.notify("The report is copied. Paste it into an email.", ctx);
                }
                if ui.button("Cancel").clicked() {
                    *close = true;
                }
            });
        });
    });
}

fn sent_page(ui: &mut egui::Ui, st: &ReportState, folder: Option<PathBuf>, close: &mut bool) {
    ui.label(RichText::new("Almost done: finish on GitHub").size(18.0).strong());
    ui.add_space(8.0);
    let mut steps = vec!["Your web browser opened GitHub's report form with your words already filled in.".to_string(), "Sign in to GitHub if it asks (or make a free account).".to_string()];
    match &folder {
        Some(dir) => steps.push(format!("A folder also opened: “{}”. Drag the files from it into the box near the bottom of the form.", dir.file_name().and_then(|n| n.to_str()).unwrap_or("PrintCraft report"))),
        None if st.include_shot || st.include_project => steps.push("The pictures and files were downloaded. Drag them into the box near the bottom of the form.".into()),
        None => {}
    }
    steps.push("Click Create (or Submit new issue) at the bottom. Thank you!".into());
    for (n, s) in steps.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{}.", n + 1)).strong().size(15.0));
            ui.add(egui::Label::new(RichText::new(s).size(15.0)).wrap());
        });
        ui.add_space(4.0);
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if let Some(dir) = &folder
            && ui.button("Open the Folder Again").clicked()
        {
            crate::platform::open_path(dir);
        }
        if ui.button("Open GitHub Again").clicked() {
            crate::platform::open_url(&st.issue_url(folder.is_some()));
        }
        if ui.button("Done").clicked() {
            *close = true;
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn addresses_are_encoded() {
        assert_eq!(super::enc("a b&c/é"), "a%20b%26c%2F%C3%A9");
    }
}
