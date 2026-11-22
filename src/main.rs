//! PrintCraft: make cards, signs, banners, labels and more, then print them.
//!
//! Usage: `printcraft [project file]` or `printcraft --screenshot out.png`.

// A window app on Windows: no black console window behind it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
// The browser version leaves out the desktop-only parts (command line,
// printers, folders), so some code is unused there.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

mod app;
mod canvas;
mod colors;
mod effects;
mod help;
mod kind_icons;
mod library;
mod recovery;
mod ai;
mod windows;
mod widgets;
mod modern;
mod cli;
mod chrome;
mod dialogs;
mod export;
mod fonts;
mod icons;
mod kinds;
mod legacy;
mod model;
mod online;
mod photo;
mod platform;
mod raster;
mod sample;
mod settings;
mod theme;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui;

use app::{Act, App};

const SCREENSHOT_FRAMES: u32 = 120;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let mut args = std::env::args_os().skip(1);
    let mut screenshot = None;
    let mut open = None;
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        } else if a == "--export" {
            // printcraft --export <project> <out.png|out.pdf>
            let (Some(src), Some(dst)) = (args.next(), args.next()) else {
                eprintln!("usage: printcraft --export <project> <out.png|out.pdf>");
                std::process::exit(2);
            };
            std::process::exit(match cli::export(PathBuf::from(src), PathBuf::from(dst)) {
                Ok(msg) => {
                    println!("{msg}");
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            });
        } else if a == "--pack-sheet" {
            // printcraft --pack-sheet <content folder> [more folders...] <out.png>
            let rest: Vec<PathBuf> = args.by_ref().map(PathBuf::from).collect();
            std::process::exit(match cli::pack_sheet(&rest) {
                Ok(msg) => {
                    println!("{msg}");
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            });
        } else {
            open = Some(PathBuf::from(a));
        }
    }

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon_light.png")).ok();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("PrintCraft")
        .with_app_id("printcraft")
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([900.0, 600.0])
        // PrintCraft draws its own title bar (menus and window buttons share
        // one row) to save room.
        .with_decorations(false)
        .with_transparent(false);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let mut options = eframe::NativeOptions { viewport, ..Default::default() };
    if screenshot.is_some() {
        options.viewport = options.viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            options.viewport = options.viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
        #[cfg(target_os = "macos")]
        {
            options.event_loop_builder = Some(Box::new(|b| {
                use winit::platform::macos::EventLoopBuilderExtMacOS;
                b.with_activate_ignoring_other_apps(false);
            }));
        }
    }
    eframe::run_native("PrintCraft", options, Box::new(move |cc| Ok(Box::new(Shell::new(cc, screenshot, open)))))
}

/// The browser version: draws into the page's `printcraft` canvas (see
/// web/index.html; built with `trunk build --release web/index.html`).
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    wasm_bindgen_futures::spawn_local(async {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
        let Some(canvas) = doc.get_element_by_id("printcraft").and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else { return };
        let started = eframe::WebRunner::new().start(canvas, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(Shell::new(cc, None, None))))).await;
        if let Some(loading) = doc.get_element_by_id("loading") {
            match started {
                Ok(()) => loading.remove(),
                Err(e) => loading.set_inner_html(&format!("<p>PrintCraft couldn't start in this browser.</p><pre>{e:?}</pre>")),
            }
        }
    });
}

struct Shell {
    app: App,
    shot: Arc<Mutex<Option<Result<(), String>>>>,
    icon: Option<egui::TextureHandle>,
    icon_dark: Option<egui::TextureHandle>,
    /// The look the window icon was last set for.
    shown_look: Option<theme::Look>,
}

impl Shell {
    fn shortcuts(&mut self, ctx: &egui::Context) -> Vec<Act> {
        let typing = self.app.editing.is_some() || ctx.egui_wants_keyboard_input();
        let mut acts = Vec::new();
        ctx.input_mut(|i| {
            use egui::{Key, KeyboardShortcut as K, Modifiers as M};
            let cmd = M::COMMAND;
            let shift_cmd = M::COMMAND | M::SHIFT;
            if i.consume_shortcut(&K::new(shift_cmd, Key::G)) {
                acts.push(Act::Ungroup);
            }
            for (k, a) in [(K::new(cmd, Key::T), Act::NewTab), (K::new(cmd, Key::W), Act::CloseTab(self.app.active)), (K::new(cmd, Key::G), Act::Group)] {
                if i.consume_shortcut(&k) {
                    acts.push(a);
                }
            }
            let table: [(K, Act); 13] = [
                (K::new(cmd, Key::N), Act::NewProject),
                (K::new(cmd, Key::O), Act::Open),
                (K::new(cmd, Key::S), Act::Save),
                (K::new(cmd, Key::P), Act::Print),
                (K::new(cmd, Key::Z), Act::Undo),
                (K::new(cmd, Key::Y), Act::Redo),
                (K::new(cmd, Key::D), Act::Duplicate),
                (K::new(cmd, Key::F), Act::Fit),
                (K::new(cmd, Key::B), Act::Bold),
                (K::new(cmd, Key::I), Act::Italic),
                (K::new(cmd, Key::U), Act::Underline),
                (K::new(M::NONE, Key::F1), Act::HelpTopics),
                (K::new(cmd, Key::A), Act::SelectAll),
            ];
            for (k, a) in table {
                let text_key = matches!(a, Act::SelectAll | Act::Undo | Act::Redo);
                if (!typing || !text_key) && i.consume_shortcut(&k) {
                    acts.push(a);
                }
            }
            if !typing {
                if i.consume_key(M::NONE, Key::Delete) || i.consume_key(M::NONE, Key::Backspace) {
                    acts.push(if self.app.guide_sel.is_some() { Act::DeleteGuide } else { Act::Delete });
                }
                if self.app.guide_sel.is_some() && i.consume_key(M::NONE, Key::Escape) {
                    self.app.guide_sel = None;
                }
                for (key, dx, dy) in [(Key::ArrowLeft, -1.0, 0.0), (Key::ArrowRight, 1.0, 0.0), (Key::ArrowUp, 0.0, -1.0), (Key::ArrowDown, 0.0, 1.0)] {
                    if i.consume_key(M::NONE, key) {
                        self.app.nudge(dx * app::NUDGE, dy * app::NUDGE);
                    }
                }
                if i.consume_key(M::NONE, Key::Plus) || i.consume_key(M::NONE, Key::Equals) {
                    acts.push(Act::ZoomIn);
                }
                if i.consume_key(M::NONE, Key::Minus) {
                    acts.push(Act::ZoomOut);
                }
            }
            let mut paste = false;
            for e in &i.events {
                match e {
                    egui::Event::Copy if !typing => acts.push(Act::Copy),
                    egui::Event::Cut if !typing => acts.push(Act::Cut),
                    egui::Event::Paste(_) if !typing => paste = true,
                    // Pictures on the clipboard don't come as a Paste event.
                    egui::Event::Key { key: Key::V, pressed: true, modifiers, .. } if !typing && modifiers.command => paste = true,
                    _ => {}
                }
            }
            if paste {
                acts.push(Act::Paste);
            }
        });
        acts
    }

    fn screenshot_step(&mut self, ctx: &egui::Context) {
        let Some((path, frames)) = self.app.screenshot.as_mut() else { return };
        *frames += 1;
        ctx.request_repaint();
        if *frames == SCREENSHOT_FRAMES {
            let path = path.clone();
            let slot = self.shot.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img| {
                let r = image::RgbaImage::from_raw(img.size[0] as u32, img.size[1] as u32, img.as_raw().to_vec())
                    .ok_or_else(|| "bad screenshot".to_string())
                    .and_then(|im| {
                        if let Some(dir) = path.parent() {
                            let _ = std::fs::create_dir_all(dir);
                        }
                        im.save(&path).map_err(|e| e.to_string())
                    });
                *slot.lock().unwrap() = Some(r);
            })));
        }
        let done = self.shot.lock().unwrap().take();
        if let Some(r) = done {
            if let Err(e) = r {
                eprintln!("screenshot failed: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if *frames > SCREENSHOT_FRAMES + 600 {
            eprintln!("screenshot timed out");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// Edge zones for resizing the window (it has no system frame).
fn resize_edges(ctx: &egui::Context) {
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let r = ctx.content_rect();
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    let m = 6.0;
    let (l, rt, t, b) = (p.x < r.left() + m, p.x > r.right() - m, p.y < r.top() + m, p.y > r.bottom() - m);
    use egui::{CursorIcon as C, ResizeDirection as D};
    let dir = match (l, rt, t, b) {
        (true, _, true, _) => Some((D::NorthWest, C::ResizeNwSe)),
        (_, true, true, _) => Some((D::NorthEast, C::ResizeNeSw)),
        (true, _, _, true) => Some((D::SouthWest, C::ResizeNeSw)),
        (_, true, _, true) => Some((D::SouthEast, C::ResizeNwSe)),
        (true, ..) => Some((D::West, C::ResizeHorizontal)),
        (_, true, ..) => Some((D::East, C::ResizeHorizontal)),
        (_, _, true, _) => Some((D::North, C::ResizeVertical)),
        (_, _, _, true) => Some((D::South, C::ResizeVertical)),
        _ => None,
    };
    if let Some((d, c)) = dir {
        ctx.set_cursor_icon(c);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(d));
        }
    }
}

impl Shell {
    fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>, open: Option<PathBuf>) -> Self {
        let load = |bytes: &[u8], name: &str| {
            image::load_from_memory(bytes).ok().map(|i| {
                let i = i.thumbnail(64, 64).to_rgba8();
                cc.egui_ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([i.width() as usize, i.height() as usize], i.as_raw()), egui::TextureOptions::LINEAR)
            })
        };
        let icon_dark = load(include_bytes!("../assets/icon_dark.png"), "app-icon-dark");
        let icon = load(include_bytes!("../assets/icon_light.png"), "app-icon");
        Shell { app: App::new(&cc.egui_ctx, screenshot, open), shot: Arc::new(Mutex::new(None)), icon, icon_dark, shown_look: None }
    }

    /// Files dropped in the browser are read there, then arrive with the
    /// chosen files.
    #[cfg(target_arch = "wasm32")]
    fn dropped(&mut self, ctx: &egui::Context) {
        let files = ctx.input(|i| i.raw.dropped_files.clone());
        if !files.is_empty() {
            crate::platform::take_dropped(files, ctx);
        }
        self.drop_hint(ctx);
    }

    /// Files dragged in from the file manager: pictures go on the page,
    /// projects open in a tab.
    #[cfg(not(target_arch = "wasm32"))]
    fn dropped(&mut self, ctx: &egui::Context) {
        let files = ctx.input(|i| i.raw.dropped_files.clone());
        for f in files {
            let path = f.path().to_path_buf();
            if path.as_os_str().is_empty() {
                if let Ok(bytes) = f.bytes() {
                    self.app.add_picture_bytes("Dropped picture", bytes, "", ctx);
                }
                continue;
            }
            self.app.add_file(path, ctx);
        }
        self.drop_hint(ctx);
    }

    fn drop_hint(&self, ctx: &egui::Context) {
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let pal = theme::palette(self.app.settings.look);
            egui::Area::new(egui::Id::unique("drop-hint")).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
                egui::Frame::new().fill(pal.accent).corner_radius(14.0).inner_margin(egui::Margin::symmetric(26, 18)).show(ui, |ui| {
                    ui.label(egui::RichText::new("Drop to add it to your project").color(pal.accent_text).size(20.0));
                });
            });
        }
    }

    /// Safety copies of changed tabs, and a save reminder on close.
    fn safety(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let every = self.app.settings.autosave_minutes.max(1) as f64 * 60.0;
        if self.app.screenshot.is_none() && now - self.app.last_autosave > every {
            self.app.last_autosave = now;
            if self.app.dirty {
                crate::recovery::write(self.app.recovery_id, &self.app.display_name(), &self.app.doc);
            }
            for (i, t) in self.app.tabs.iter().enumerate() {
                if i != self.app.active && t.dirty {
                    let name = t.name.clone().unwrap_or_else(|| "Untitled".into());
                    crate::recovery::write(t.recovery_id, &name, &t.doc);
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.app.quit && self.app.screenshot.is_none() && !self.app.unsaved().is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.app.dialogs.iter().any(|d| matches!(d, dialogs::Dialog::ConfirmQuit)) {
                self.app.dialogs.push(dialogs::Dialog::ConfirmQuit);
            }
        }
        if self.app.quit {
            crate::recovery::forget(self.app.recovery_id);
            for t in &self.app.tabs {
                if !t.dirty {
                    crate::recovery::forget(t.recovery_id);
                }
            }
        }
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Follow the computer's light/dark setting when asked to.
        if self.app.settings.follow_system
            && self.app.screenshot.is_none()
            && let Some(t) = ctx.system_theme()
        {
            let want = if t == egui::Theme::Dark { theme::Look::Dark } else { theme::Look::Light };
            if want != self.app.settings.look {
                self.app.set_look(&ctx, want);
            }
        }
        // The icon that suits the look, in the title bar and for the window.
        if self.shown_look != Some(self.app.settings.look) {
            self.shown_look = Some(self.app.settings.look);
            #[cfg(not(target_arch = "wasm32"))]
            {
                let bytes: &[u8] = if self.app.settings.look == theme::Look::Dark { include_bytes!("../assets/icon_dark.png") } else { include_bytes!("../assets/icon_light.png") };
                if let Ok(i) = eframe::icon_data::from_png_bytes(bytes) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(std::sync::Arc::new(i))));
                }
            }
        }
        let icon = if self.app.settings.look == theme::Look::Dark { self.icon_dark.clone().or(self.icon.clone()) } else { self.icon.clone() };
        let app = &mut self.app;
        app.hint.clear();
        let mut acts = Vec::new();
        let pal = theme::palette(app.settings.look);
        let bar = |m: (i8, i8)| egui::Frame::new().fill(pal.panel).inner_margin(egui::Margin::symmetric(m.0, m.1));

        egui::Panel::top("titlebar").frame(egui::Frame::new().fill(pal.chrome).inner_margin(egui::Margin { left: 8, right: 0, top: 2, bottom: 2 })).show(ui, |ui| {
            acts.extend(chrome::title_bar(ui, app, icon.as_ref()));
        });
        if app.settings.modern {
            egui::Panel::top("contextbar").frame(bar((10, 6))).show(ui, |ui| {
                acts.extend(modern::context_bar(ui, app));
            });
        } else {
            egui::Panel::top("toolbars").frame(bar((10, 5))).show(ui, |ui| {
                acts.extend(chrome::standard_bar(ui, app));
                ui.add_space(3.0);
                acts.extend(chrome::text_bar(ui, app));
            });
        }
        egui::Panel::bottom("status").frame(bar((10, 4))).show(ui, |ui| {
            acts.extend(chrome::status_bar(ui, app));
        });
        if app.settings.modern {
            egui::Panel::left("rail").resizable(false).exact_size(84.0).frame(egui::Frame::new().fill(pal.chrome).inner_margin(egui::Margin::symmetric(6, 8))).show(ui, |ui| {
                acts.extend(modern::rail(ui, app));
            });
            if app.modern_drawer.is_some() {
                egui::Panel::left("drawer").resizable(false).exact_size(300.0).frame(bar((12, 10))).show(ui, |ui| {
                    acts.extend(modern::drawer(ui, app));
                });
            }
        } else {
            egui::Panel::left("tools").resizable(false).exact_size(236.0).frame(bar((8, 4))).show(ui, |ui| {
                acts.extend(chrome::tool_panel(ui, app));
            });
        }
        egui::Panel::top("tabs").frame(egui::Frame::new().fill(pal.panel).inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 0 })).show(ui, |ui| {
            acts.extend(chrome::tab_bar(ui, app));
        });
        // Legacy Pages: one page at a time with page buttons along the
        // bottom. Otherwise every page is laid out in the workspace.
        if app.settings.legacy_pages {
            egui::Panel::bottom("pages").frame(egui::Frame::new().fill(pal.workspace).inner_margin(egui::Margin::symmetric(10, 4))).show(ui, |ui| {
                acts.extend(chrome::page_tabs(ui, app));
            });
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(pal.workspace)).show(ui, |ui| {
            acts.extend(canvas::show(ui, app));
        });
        acts.extend(dialogs::show_all(&ctx, app));

        if app.toast_until_click && ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.time) > 1.0 {
            app.toast = None;
            app.toast_until_click = false;
        }
        if let Some((msg, until)) = app.toast.clone() {
            if ctx.input(|i| i.time) > until {
                app.toast = None;
            } else {
                egui::Area::new(egui::Id::unique("toast")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -56.0)).show(&ctx, |ui| {
                    egui::Frame::new().fill(pal.accent).corner_radius(12.0).inner_margin(egui::Margin::symmetric(18, 10)).show(ui, |ui| {
                        ui.label(egui::RichText::new(msg).color(pal.accent_text).size(15.0));
                    });
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
        }

        acts.extend(self.shortcuts(&ctx));
        for a in acts {
            self.app.run(a, &ctx);
        }
        self.dropped(&ctx);
        self.app.poll_picked(&ctx);
        if !crate::platform::WEB {
            resize_edges(&ctx);
        }
        self.safety(&ctx);
        self.app.poll_library();
        // Make sure fonts used on this page are loaded for typing.
        if let Some((id, _)) = &self.app.editing
            && let Some(l) = self.app.item(*id).and_then(|i| i.look().cloned())
        {
            self.app.fonts.ensure(&ctx, &l.family, l.bold, l.italic);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.app.title()));
        if self.app.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.screenshot_step(&ctx);
    }
}
