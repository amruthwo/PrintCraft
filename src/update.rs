//! Update checker: asks GitHub for the newest release (once a day, or when
//! asked), picks the file that matches how this copy was installed (the
//! same package type and processor), downloads it with a progress bar,
//! checks it against GitHub's SHA-256, and installs it where that can be
//! done safely. Desktop only; the browser version is always current.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const REPO: &str = "amruthwo/PrintCraft";
pub const RELEASES_PAGE: &str = "https://github.com/amruthwo/PrintCraft/releases/latest";
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// How this copy of PrintCraft was installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Install {
    WindowsSetup,
    WindowsMsi,
    WindowsPortable,
    MacApp,
    AppImage,
    Flatpak,
    Deb,
    Rpm,
    Tarball,
    /// Built from source, or the browser version: no package to update.
    Other,
}

impl Install {
    /// The words used in the bug report form's "How did you install it?".
    pub fn report_label(self) -> &'static str {
        match self {
            Install::WindowsSetup | Install::WindowsMsi => "Windows installer (.msi / setup .exe)",
            Install::WindowsPortable => "Portable .zip (Windows)",
            Install::MacApp => "macOS .dmg / .app",
            Install::AppImage => "AppImage",
            Install::Flatpak => "Flatpak",
            Install::Deb => ".deb",
            Install::Rpm => ".rpm",
            Install::Tarball => "Portable .tar.gz (Linux)",
            Install::Other if crate::platform::WEB => "Web version",
            Install::Other => "Built from source",
        }
    }
}

/// The processor in release file names.
pub fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        a => a,
    }
}

/// Work out how this copy was installed.
pub fn detect() -> Install {
    if crate::platform::WEB {
        return Install::Other;
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let text = exe.to_string_lossy().to_string();
    if text.contains("/target/") || text.contains("\\target\\") {
        return Install::Other;
    }
    if cfg!(target_os = "windows") {
        let dir = exe.parent().map(|d| d.to_path_buf()).unwrap_or_default();
        if dir.join("uninstall.exe").exists() || dir.join("Uninstall.exe").exists() {
            return Install::WindowsSetup;
        }
        let program_files = ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"].iter().filter_map(|v| std::env::var(v).ok()).any(|p| text.to_lowercase().starts_with(&p.to_lowercase()));
        return if program_files { Install::WindowsMsi } else { Install::WindowsPortable };
    }
    if cfg!(target_os = "macos") {
        return if text.contains(".app/Contents/MacOS") { Install::MacApp } else { Install::Other };
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return Install::AppImage;
    }
    if std::env::var_os("FLATPAK_ID").is_some() || std::path::Path::new("/.flatpak-info").exists() {
        return Install::Flatpak;
    }
    if text.starts_with("/usr/") {
        if std::path::Path::new("/var/lib/dpkg/info/printcraft.list").exists() {
            return Install::Deb;
        }
        if std::process::Command::new("rpm").args(["-q", "printcraft"]).output().is_ok_and(|o| o.status.success()) {
            return Install::Rpm;
        }
        return Install::Deb;
    }
    Install::Tarball
}

/// The end of the release file name for this install (see
/// .github/scripts/release-names.sh), best first.
fn wanted(install: Install) -> Vec<String> {
    let a = arch();
    match install {
        Install::WindowsSetup => vec![format!("windows-{a}-setup.exe")],
        // There's no .msi for arm64; the setup .exe does the same job.
        Install::WindowsMsi => vec![format!("windows-{a}.msi"), format!("windows-{a}-setup.exe")],
        Install::WindowsPortable => vec![format!("windows-{a}-portable.zip")],
        Install::MacApp => vec!["macos-universal.dmg".into()],
        Install::AppImage => vec![format!("linux-{a}.AppImage")],
        Install::Flatpak => vec![format!("linux-{a}.flatpak")],
        Install::Deb => vec![format!("linux-{a}.deb")],
        Install::Rpm => vec![format!("linux-{a}.rpm")],
        Install::Tarball => vec![format!("linux-{a}.tar.gz")],
        Install::Other => Vec::new(),
    }
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    /// "sha256:..." from GitHub, when it has one.
    pub digest: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub page: String,
    pub assets: Vec<Asset>,
}

impl Release {
    /// The file for this install, if the release has one.
    pub fn asset_for(&self, install: Install) -> Option<Asset> {
        wanted(install).iter().find_map(|end| self.assets.iter().find(|a| a.name.ends_with(end.as_str())).cloned())
    }
}

/// "0.1.10" → [0, 1, 10], ignoring a leading v and anything after a dash.
fn version_parts(v: &str) -> Vec<u64> {
    v.trim().trim_start_matches('v').split('-').next().unwrap_or("").split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    version_parts(candidate) > version_parts(current)
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Ask GitHub about the newest release.
pub fn latest() -> Result<Release, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let agent = concat!("PrintCraft/", env!("CARGO_PKG_VERSION"), " (update check)");
    let body = crate::platform::http_get(&url, agent, 20).map_err(|e| if e == crate::platform::DESKTOP_ONLY { e } else { "PrintCraft couldn't reach GitHub to check for updates. Check the internet connection and try again.".into() })?;
    let v: serde_json::Value = serde_json::from_slice(&body).map_err(|_| "GitHub's answer couldn't be read.".to_string())?;
    let tag = v["tag_name"].as_str().ok_or("No release found.")?;
    let assets = v["assets"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|a| {
            Some(Asset {
                name: a["name"].as_str()?.to_string(),
                url: a["browser_download_url"].as_str()?.to_string(),
                size: a["size"].as_u64().unwrap_or(0),
                digest: a["digest"].as_str().map(str::to_string),
            })
        })
        .collect();
    Ok(Release {
        version: tag.trim_start_matches('v').to_string(),
        notes: v["body"].as_str().unwrap_or("").to_string(),
        page: v["html_url"].as_str().unwrap_or(RELEASES_PAGE).to_string(),
        assets,
    })
}

/// What the update window is showing.
#[derive(Clone)]
pub enum Stage {
    Checking,
    UpToDate,
    Available(Release),
    Downloading { got: Arc<AtomicU64>, total: u64 },
    /// Downloaded and checked; what to do next depends on the install.
    Ready(PathBuf),
    Done(String),
    Failed(String),
}

/// A check or download running in the background; the result lands here.
pub type Slot = Arc<Mutex<Option<Stage>>>;

pub fn due(settings: &crate::settings::Settings) -> bool {
    !crate::platform::WEB && settings.check_updates && crate::platform::now_ms().saturating_sub(settings.last_update_check) > DAY_MS
}

/// Start a check in the background.
pub fn check(ctx: &eframe::egui::Context) -> Slot {
    let slot: Slot = Arc::new(Mutex::new(None));
    let (out, ctx) = (slot.clone(), ctx.clone());
    crate::platform::spawn(move || {
        let stage = match latest() {
            Ok(r) if is_newer(&r.version, current_version()) => Stage::Available(r),
            Ok(_) => Stage::UpToDate,
            Err(e) => Stage::Failed(e),
        };
        *out.lock().unwrap() = Some(stage);
        ctx.request_repaint();
    });
    slot
}

/// Where downloads go: the Downloads folder (or PrintCraft's own folder).
fn download_dir() -> PathBuf {
    crate::platform::downloads_dir().or_else(crate::platform::data_dir).unwrap_or_else(|| PathBuf::from("."))
}

/// Download in the background, counting bytes into `got`.
pub fn download(asset: Asset, got: Arc<AtomicU64>, ctx: &eframe::egui::Context) -> Slot {
    let slot: Slot = Arc::new(Mutex::new(None));
    let (out, ctx) = (slot.clone(), ctx.clone());
    crate::platform::spawn(move || {
        let r = fetch_to_file(&asset, &got, &ctx);
        *out.lock().unwrap() = Some(match r {
            Ok(p) => Stage::Ready(p),
            Err(e) => Stage::Failed(e),
        });
        ctx.request_repaint();
    });
    slot
}

#[cfg(target_arch = "wasm32")]
fn fetch_to_file(_asset: &Asset, _got: &AtomicU64, _ctx: &eframe::egui::Context) -> Result<PathBuf, String> {
    Err(crate::platform::DESKTOP_ONLY.into())
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_to_file(asset: &Asset, got: &AtomicU64, ctx: &eframe::egui::Context) -> Result<PathBuf, String> {
    use sha2::Digest;
    use std::io::{Read, Write};
    let dir = download_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(&asset.name);
    let part = dir.join(format!("{}.part", asset.name));
    let agent = concat!("PrintCraft/", env!("CARGO_PKG_VERSION"), " (update)");
    let resp = ureq::get(&asset.url).set("User-Agent", agent).timeout(std::time::Duration::from_secs(600)).call().map_err(|_| "The download couldn't start. Check the internet connection and try again.".to_string())?;
    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(&part).map_err(|e| format!("Couldn't save the download: {e}"))?;
    let mut hash = sha2::Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(|_| "The download stopped part way. Please try again.".to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| format!("Couldn't save the download: {e}"))?;
        hash.update(&buf[..n]);
        got.fetch_add(n as u64, Ordering::Relaxed);
        ctx.request_repaint();
    }
    drop(file);
    let sum: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if let Some(expected) = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:"))
        && !expected.eq_ignore_ascii_case(&sum)
    {
        let _ = std::fs::remove_file(&part);
        return Err("The download didn't match GitHub's checksum, so it wasn't used. Please try again.".into());
    }
    std::fs::rename(&part, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Put the downloaded update in place, or hand it to the system's
/// installer. Returns what to tell the person, and whether PrintCraft
/// should close so the installer can replace it.
#[cfg(target_arch = "wasm32")]
pub fn install(_install: Install, _file: &std::path::Path) -> Result<(String, bool), String> {
    Err(crate::platform::DESKTOP_ONLY.into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install(install: Install, file: &std::path::Path) -> Result<(String, bool), String> {
    use std::process::Command;
    let shown = file.display().to_string();
    match install {
        Install::WindowsSetup => {
            Command::new(file).spawn().map_err(|e| format!("The installer couldn't start: {e}"))?;
            Ok(("The installer is starting. PrintCraft will close so it can be updated.".into(), true))
        }
        Install::WindowsMsi => {
            Command::new("msiexec").arg("/i").arg(file).spawn().map_err(|e| format!("The installer couldn't start: {e}"))?;
            Ok(("The installer is starting. PrintCraft will close so it can be updated.".into(), true))
        }
        Install::AppImage => {
            // Swap the AppImage file in place; the new one runs next time.
            let target = PathBuf::from(std::env::var_os("APPIMAGE").ok_or("Couldn't find the AppImage file.")?);
            let tmp = target.with_extension("new");
            std::fs::copy(file, &tmp).map_err(|e| format!("Couldn't replace the AppImage: {e}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
            }
            std::fs::rename(&tmp, &target).map_err(|e| format!("Couldn't replace the AppImage: {e}"))?;
            let _ = std::fs::remove_file(file);
            Ok(("Updated. Close PrintCraft and open it again to use the new version.".into(), false))
        }
        Install::Flatpak => {
            let ok = Command::new("flatpak-spawn").args(["--host", "flatpak", "install", "--user", "-y", "--bundle"]).arg(file).status().is_ok_and(|s| s.success());
            if ok {
                Ok(("Updated. Close PrintCraft and open it again to use the new version.".into(), false))
            } else {
                crate::platform::open_path(file.parent().unwrap_or(file));
                Ok((format!("The update was downloaded to {shown}. Install it with: flatpak install --user {shown}"), false))
            }
        }
        Install::Deb | Install::Rpm => {
            // The system's software installer asks for the password.
            crate::platform::open_path(file);
            let cmd = if install == Install::Deb { "sudo apt install" } else { "sudo dnf install" };
            Ok((format!("The update opened in your software installer. If it didn't, run: {cmd} {shown}"), false))
        }
        Install::MacApp => {
            crate::platform::open_path(file);
            Ok(("The update opened. Drag PrintCraft into Applications and choose Replace, then open it again.".into(), true))
        }
        Install::WindowsPortable | Install::Tarball => {
            crate::platform::open_path(file.parent().unwrap_or(file));
            Ok((format!("The new version was saved to {shown}. Unpack it over your PrintCraft folder (close PrintCraft first)."), false))
        }
        Install::Other => Err("This copy of PrintCraft wasn't installed from a release, so it can't update itself.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
    }

    #[test]
    fn picks_the_matching_file() {
        let names = ["printcraft-0.2.0-windows-x86_64-setup.exe", "printcraft-0.2.0-windows-arm64-setup.exe", "printcraft-0.2.0-linux-x86_64.AppImage", "printcraft-0.2.0-linux-arm64.deb", "printcraft-0.2.0-macos-universal.dmg"];
        let r = Release { version: "0.2.0".into(), notes: String::new(), page: String::new(), assets: names.iter().map(|n| Asset { name: (*n).into(), url: String::new(), size: 0, digest: None }).collect() };
        assert_eq!(r.asset_for(Install::MacApp).map(|a| a.name), Some("printcraft-0.2.0-macos-universal.dmg".into()));
        // No arm64 .msi: falls back to the setup .exe.
        if arch() == "arm64" {
            assert_eq!(r.asset_for(Install::WindowsMsi).map(|a| a.name), Some("printcraft-0.2.0-windows-arm64-setup.exe".into()));
        }
        assert!(r.asset_for(Install::Other).is_none());
    }
}

/// The update window's state.
pub struct UpdateState {
    pub stage: Stage,
    pub slot: Option<Slot>,
    pub install: Install,
    /// Shown when Install Now has to wait for projects to be saved.
    pub save_first: bool,
}

impl UpdateState {
    /// A check the person asked for.
    pub fn checking(ctx: &eframe::egui::Context) -> Self {
        Self { stage: Stage::Checking, slot: Some(check(ctx)), install: detect(), save_first: false }
    }
    /// A new version found by the daily check.
    pub fn found(r: Release) -> Self {
        Self { stage: Stage::Available(r), slot: None, install: detect(), save_first: false }
    }
}

/// Release notes are Markdown; show them as plain, readable lines.
fn plain_notes(md: &str) -> String {
    md.lines()
        .map(|l| {
            let l = l.trim_end();
            let l = l.trim_start_matches('#').trim_start();
            let l = if let Some(rest) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) { format!("•  {rest}") } else { l.to_string() };
            l.replace("**", "").replace('`', "")
        })
        .filter(|l| !l.starts_with("> [!"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn window(ctx: &eframe::egui::Context, app: &mut crate::app::App, st: &mut UpdateState, open: &mut bool, close: &mut bool) {
    use eframe::egui::{self, RichText};
    let arrived = st.slot.as_ref().and_then(|slot| slot.lock().unwrap().take());
    if let Some(stage) = arrived {
        if !matches!(stage, Stage::Failed(_)) {
            app.settings.last_update_check = crate::platform::now_ms();
            app.settings.save();
        }
        st.stage = stage;
        st.slot = None;
    }
    let mut next: Option<Stage> = None;
    crate::dialogs::window("PrintCraft Updates").open(open).default_size([520.0, 420.0]).show(ctx, |ui| {
        match &st.stage {
            Stage::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Looking for a new version…");
                });
            }
            Stage::UpToDate => {
                ui.label(RichText::new(format!("You have the newest version of PrintCraft ({}).", current_version())).size(15.0));
                if ui.button("OK").clicked() {
                    *close = true;
                }
            }
            Stage::Available(r) => {
                ui.label(RichText::new(format!("PrintCraft {} is ready", r.version)).size(18.0).strong());
                ui.label(format!("You have version {}.", current_version()));
                ui.add_space(6.0);
                if !r.notes.trim().is_empty() {
                    ui.label(RichText::new("What's new").strong());
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        ui.label(plain_notes(&r.notes));
                    });
                    ui.add_space(6.0);
                }
                match r.asset_for(st.install) {
                    Some(asset) => {
                        let mb = asset.size as f64 / 1_000_000.0;
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new(RichText::new("Download and Install").size(15.0)).min_size(egui::vec2(180.0, 32.0))).on_hover_text(asset.name.clone()).clicked() {
                                let got = Arc::new(AtomicU64::new(0));
                                st.slot = Some(download(asset.clone(), got.clone(), ctx));
                                next = Some(Stage::Downloading { got, total: asset.size });
                            }
                            ui.label(RichText::new(format!("{mb:.0} MB")).weak());
                        });
                    }
                    None => {
                        ui.label("Get the new version from the download page:");
                        if ui.button("Open the Download Page").clicked() {
                            crate::platform::open_url(&r.page);
                        }
                    }
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Remind Me Later").clicked() {
                        *close = true;
                    }
                    if ui.button("Skip This Version").on_hover_text("Don't remind me about this version again").clicked() {
                        app.settings.skip_version = r.version.clone();
                        app.settings.save();
                        *close = true;
                    }
                });
            }
            Stage::Downloading { got, total } => {
                let n = got.load(Ordering::Relaxed);
                let k = if *total > 0 { n as f32 / *total as f32 } else { 0.0 };
                ui.label("Downloading the new version…");
                ui.add(egui::ProgressBar::new(k).show_percentage());
            }
            Stage::Ready(path) => {
                ui.label(RichText::new("The new version is downloaded and checked.").size(15.0));
                if st.save_first {
                    ui.colored_label(ui.visuals().warn_fg_color, "Please save your projects first (File > Save), then click Install Now.");
                }
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Install Now").size(15.0)).min_size(egui::vec2(140.0, 32.0))).clicked() {
                        let closes = matches!(st.install, Install::WindowsSetup | Install::WindowsMsi | Install::MacApp);
                        if closes && !app.unsaved().is_empty() {
                            st.save_first = true;
                        } else {
                            next = Some(match install(st.install, path) {
                                Ok((msg, quit)) => {
                                    if quit {
                                        app.quit = true;
                                    }
                                    Stage::Done(msg)
                                }
                                Err(e) => Stage::Failed(e),
                            });
                        }
                    }
                    if ui.button("Later").clicked() {
                        *close = true;
                    }
                });
            }
            Stage::Done(msg) => {
                ui.label(RichText::new(msg).size(15.0));
                if ui.button("OK").clicked() {
                    *close = true;
                }
            }
            Stage::Failed(e) => {
                ui.label(RichText::new(e).size(15.0));
                ui.horizontal(|ui| {
                    if ui.button("Try Again").clicked() {
                        st.slot = Some(check(ctx));
                        next = Some(Stage::Checking);
                    }
                    if ui.button("Open the Download Page").clicked() {
                        crate::platform::open_url(RELEASES_PAGE);
                    }
                });
            }
        }
    });
    if let Some(n) = next {
        st.stage = n;
    }
    if matches!(st.stage, Stage::Downloading { .. } | Stage::Checking) {
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}
