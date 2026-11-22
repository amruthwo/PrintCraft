//! Editor state and the actions that menus, toolbar buttons and the left
//! panel all share.

use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui;

use crate::dialogs::Dialog;
use crate::fonts::FontBook;
use crate::kinds::Kind;
use crate::legacy::collection::Library;
use crate::model::*;
use crate::online::Online;
use crate::platform::{Opened, Want};
use crate::raster::PictureCache;
use crate::settings::Settings;
use crate::theme::{self, Look};

/// Undo history length: at least 10 steps, more if the person's setting
/// asks (see effects::UNDO_MINIMUM).
pub fn undo_depth(settings: &Settings) -> usize {
    settings.undo_steps.max(crate::effects::UNDO_MINIMUM)
}

pub fn fresh_recovery_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let t = crate::platform::now_ms();
    t * 100 + NEXT.fetch_add(1, Ordering::Relaxed) % 100
}
pub const NUDGE: f32 = 1.0 / 16.0;

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    NewProject,
    Open,
    Save,
    SaveAs,
    SavePdf,
    ExportPicture,
    Print,
    PrintPreview,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Duplicate,
    SelectAll,
    Delete,
    ZoomIn,
    ZoomOut,
    Fit,
    Zoom(f32),
    ToggleRulers,
    Panel(usize),
    ToggleLook,
    Gallery(u8),
    PictureFromFile,
    Headline,
    TextBox,
    TextInShape,
    Border(u8),
    Backdrop,
    Shape(Outline),
    Line,
    Seal,
    InitialCap,
    Number,
    Signature,
    CenterPage,
    CenterH,
    CenterV,
    FullPage,
    Rotate(f32),
    FlipH,
    FlipV,
    ToFront,
    ToBack,
    Forward,
    Backward,
    Lock,
    Shadow,
    Glow,
    Frame,
    Tint(f32),
    Bold,
    Italic,
    Underline,
    Align(Align),
    VAlign(VAlign),
    TextShape,
    StretchText,
    SpeechBubble,
    AddressMerge,
    Photo(u8),
    AddressBook,
    Reminders,
    ContentPacks,
    GetFonts,
    About,
    Help,
    ProjectNotes,
    BackgroundColor,
    Group,
    Ungroup,
    AlignItems(u8),
    Distribute(bool),
    MoreShapes,
    Borders,
    Opacity(f32),
    FitText,
    Settings,
    HelpTopics,
    NewTab,
    CloseTab(usize),
    SwitchTab(usize),
    AddPanel,
    MyArt,
    SaveToMyArt,
    AiPictures,
    PickColorFromPage(bool),
    /// Remove the selected guideline.
    DeleteGuide,
    /// Remove every guideline on the page shown.
    ClearGuides,
    ToggleGuides,
    /// Switch between every page in the workspace and Legacy Pages.
    ToggleLegacyPages,
    ToggleTilePages,
}

/// One open project. The active tab's state lives in `App` itself; the
/// others wait here until switched to.
#[derive(Clone)]
pub struct Tab {
    pub name: Option<String>,
    pub doc: Project,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub panel: usize,
    pub zoom: f32,
    pub pan: egui::Vec2,
    pub selected: Vec<u64>,
    pub undo: Vec<Project>,
    pub redo: Vec<Project>,
    pub recovery_id: u64,
}

pub struct ItemTexture {
    pub key: u64,
    pub tex: egui::TextureHandle,
    pub pad: f32,
    pub ppi: f32,
}

pub enum Drag {
    Move { start: Vec<(u64, Frame)>, origin: (f32, f32) },
    Resize { id: u64, handle: usize, start: Frame },
    Rotate { id: u64, start: f32 },
    /// Dragging a headline's bend handle.
    Warp { id: u64, start: f32, origin: (f32, f32) },
    /// Dragging a headline's bulge handle.
    Bulge { id: u64, start: f32, origin: (f32, f32) },
    Marquee { origin: (f32, f32) },
}

/// A guideline handle being dragged on a ruler.
#[derive(Clone, Copy, Debug)]
pub struct GuideDrag {
    pub page: usize,
    pub index: usize,
    /// The pointer has left the ruler: letting go removes the guide.
    pub removing: bool,
}

pub struct App {
    pub settings: Settings,
    pub doc: Project,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub panel: usize,
    pub zoom: f32,
    pub pan: egui::Vec2,
    pub fit_pending: bool,
    pub selected: Vec<u64>,
    pub undo: Vec<Project>,
    pub redo: Vec<Project>,
    pub clipboard: Vec<Item>,
    pub fonts: FontBook,
    pub pics: PictureCache,
    pub textures: HashMap<u64, ItemTexture>,
    pub open_sections: Vec<bool>,
    pub dialogs: Vec<Dialog>,
    pub hint: String,
    pub toast: Option<(String, f64)>,
    pub editing: Option<(u64, String)>,
    pub drag: Option<Drag>,
    pub drag_snapshot: Option<Project>,
    pub online: Online,
    pub library: Option<Library>,
    pub pointer_in: Option<(f32, f32)>,
    pub screenshot: Option<(PathBuf, u32)>,
    pub fill_color: Rgba,
    pub line_color: Rgba,
    pub quit: bool,
    /// Snap guides being shown: (vertical?, position in inches).
    pub guides: Vec<(bool, f32)>,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub tab_name: Option<String>,
    pub recovery_id: u64,
    /// Waiting for a click on the page to pick a color (true = fill).
    pub eyedropper: Option<u8>,
    pub last_autosave: f64,
    pub confirm_close: bool,
    pub font_previews: HashMap<String, Option<egui::TextureHandle>>,
    pub modern_drawer: Option<usize>,
    /// Selected letters (start, end) in the text being edited.
    pub edit_sel: Option<(usize, usize)>,
    /// The current notice stays until the next click.
    pub toast_until_click: bool,
    /// Keep the next downloaded online picture in My Art.
    pub keep_web: bool,
    pub library_job: Option<std::sync::mpsc::Receiver<Library>>,
    /// The selected guideline (page, index); Delete removes it.
    pub guide_sel: Option<(usize, usize)>,
    pub guide_drag: Option<GuideDrag>,
    /// Scroll this page into view on the next frame (every-page view).
    pub scroll_to: Option<usize>,
}

/// Marker put on the system clipboard when copying items inside PrintCraft,
/// so a later paste knows whether to use them or something copied elsewhere.
const COPY_MARK: &str = "\u{2063}PrintCraft items\u{2063}";

const OLDER_FORMAT: &str = "PrintCraft can open older .sig-style project files, but can't save them yet: the way they store layout isn't fully worked out. Your design was not saved. Please save it as a PrintCraft design (.pcraft), or as a PDF to print anywhere.";

impl App {
    pub fn new(ctx: &egui::Context, screenshot: Option<PathBuf>, open: Option<PathBuf>) -> Self {
        let mut settings = Settings::load();
        if screenshot.is_some() {
            settings.look = if std::env::var("PRINTCRAFT_LOOK").is_ok_and(|v| v == "dark") { Look::Dark } else { Look::Light };
            // The check picture always shows the standard layout.
            settings.legacy_pages = std::env::var("PRINTCRAFT_PAGES").is_ok_and(|v| v == "legacy");
            settings.tile_pages = std::env::var("PRINTCRAFT_PAGES").is_ok_and(|v| v == "tiled");
            settings.show_guides = true;
        }
        theme::apply(ctx, settings.look);
        ctx.set_fonts(crate::fonts::base_definitions());
        let mut app = Self {
            doc: Project::new(Kind::Sign, false),
            settings,
            path: None,
            dirty: false,
            panel: 0,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            fit_pending: true,
            selected: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            clipboard: Vec::new(),
            fonts: FontBook::new(),
            pics: PictureCache::default(),
            textures: HashMap::new(),
            open_sections: vec![false, false, true, false, false, false, false, false, false, false],
            dialogs: Vec::new(),
            hint: String::new(),
            toast: None,
            editing: None,
            drag: None,
            drag_snapshot: None,
            online: Online::default(),
            library: None,
            pointer_in: None,
            screenshot: screenshot.map(|p| (p, 0)),
            fill_color: Rgba::hex("#2f8fd8"),
            line_color: Rgba::BLACK,
            quit: false,
            guides: Vec::new(),
            tabs: Vec::new(),
            active: 0,
            tab_name: None,
            recovery_id: fresh_recovery_id(),
            eyedropper: None,
            last_autosave: 0.0,
            confirm_close: false,
            font_previews: HashMap::new(),
            modern_drawer: None,
            edit_sel: None,
            toast_until_click: false,
            keep_web: false,
            library_job: None,
            guide_sel: None,
            guide_drag: None,
            scroll_to: None,
        };
        app.tabs.push(app.stash());
        app.rescan_library();
        if app.screenshot.is_some() {
            app.doc = if std::env::var("PRINTCRAFT_SAMPLE").is_ok_and(|v| v == "card") { crate::sample::card() } else { crate::sample::poster() };
            if let Some(id) = app.doc.panels[0].items.get(2).map(|i| i.id) {
                app.selected = vec![id];
            }
            // For checking a window's look: PRINTCRAFT_OPEN=google.
            if std::env::var("PRINTCRAFT_OPEN").is_ok_and(|v| v == "google") {
                app.dialogs.push(Dialog::gallery(1));
            }
        } else if let Some(p) = open {
            // A design or older project opens; a picture goes on the page.
            app.add_file(p, ctx);
        } else {
            app.dialogs.push(Dialog::welcome());
        }
        if app.screenshot.is_none() {
            let left = crate::recovery::leftovers();
            if !left.is_empty() {
                app.dialogs.push(Dialog::Recover(left));
            }
        }
        app
    }

    /// Look through the content folders in the background.
    pub fn rescan_library(&mut self) {
        if self.settings.content_folders.is_empty() {
            self.library = None;
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let folders = self.settings.content_folders.clone();
        crate::platform::spawn(move || {
            let _ = tx.send(Library::scan(&folders));
        });
        self.library_job = Some(rx);
    }

    pub fn poll_library(&mut self) {
        if let Some(rx) = &self.library_job
            && let Ok(lib) = rx.try_recv()
        {
            self.library = Some(lib);
            self.library_job = None;
        }
    }

    /// A file from outside: a picture goes on the page, a project opens.
    pub fn add_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        match std::fs::read(&path) {
            Ok(bytes) => {
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("Picture").to_string();
                self.add_opened(Want::Any, Opened { name, bytes, path: Some(path) }, ctx);
            }
            Err(e) => self.notify(format!("Couldn't open {}: {e}", path.display()), ctx),
        }
    }

    /// A chosen or dropped file, used for what it was chosen for (or, for
    /// dropped files, what its extension says it is).
    pub fn add_opened(&mut self, want: Want, f: Opened, ctx: &egui::Context) {
        let ext = f.extension();
        let project = ext == "pcraft" || Kind::from_legacy_extension(&ext).is_some();
        match want {
            Want::Project => self.open_file(f),
            Want::Any if project => self.open_file(f),
            Want::Picture | Want::Any => {
                let name = f.stem();
                self.add_picture_bytes(&name, f.bytes, "", ctx);
            }
        }
    }

    /// Files chosen in a file window (the browser delivers them a moment
    /// after the choice).
    pub fn poll_picked(&mut self, ctx: &egui::Context) {
        for (want, f) in crate::platform::take_picked() {
            self.add_opened(want, f, ctx);
        }
    }

    /// Paste from the system clipboard: a picture, copied picture files, or
    /// text. Returns false when PrintCraft's own copied items should paste.
    fn paste_external(&mut self, ctx: &egui::Context) -> bool {
        if crate::platform::WEB {
            return false;
        }
        if let Some(t) = crate::platform::clipboard_text() {
            if t == COPY_MARK {
                return false;
            }
            let files: Vec<PathBuf> = t
                .lines()
                .map(|l| l.trim())
                .filter_map(|l| l.strip_prefix("file://").map(|p| PathBuf::from(percent_decode(p))).or_else(|| (l.starts_with('/') && std::path::Path::new(l).exists()).then(|| PathBuf::from(l))))
                .collect();
            if !files.is_empty() {
                for f in files {
                    self.add_file(f, ctx);
                }
                return true;
            }
            // A copied picture address from a web browser: download it.
            let t1 = t.trim();
            if (t1.starts_with("https://") || t1.starts_with("http://")) && !t1.contains(char::is_whitespace) {
                let pic = crate::online::WebPicture { title: "Picture from the web".into(), thumb_url: t1.into(), full_url: t1.into(), credit: format!("From {t1}") };
                self.notify("Getting the picture…", ctx);
                self.online.fetch_picture(pic, ctx);
                return true;
            }
        }
        if let Some((w, h, img)) = crate::platform::clipboard_image() {
            if let Some(rgba) = image::RgbaImage::from_raw(w, h, img) {
                let png = crate::photo::to_png(&image::DynamicImage::ImageRgba8(rgba));
                self.add_picture_bytes("Pasted picture", png, "", ctx);
                return true;
            }
        }
        if let Some(t) = crate::platform::clipboard_text()
            && !t.trim().is_empty()
            && self.clipboard.is_empty()
        {
            let look = TextLook { size: 16.0, ..Default::default() };
            let lines = t.lines().count().max(1) as f32;
            self.add_item(4.0, (lines * 0.3).clamp(0.5, 6.0), Content::Text { text: t, look, shape: None, fit: false, runs: Vec::new() });
            return true;
        }
        false
    }

    fn stash(&self) -> Tab {
        Tab {
            name: self.tab_name.clone(),
            doc: self.doc.clone(),
            path: self.path.clone(),
            dirty: self.dirty,
            panel: self.panel,
            zoom: self.zoom,
            pan: self.pan,
            selected: self.selected.clone(),
            undo: self.undo.clone(),
            redo: self.redo.clone(),
            recovery_id: self.recovery_id,
        }
    }

    fn load(&mut self, t: Tab) {
        self.tab_name = t.name;
        self.doc = t.doc;
        self.path = t.path;
        self.dirty = t.dirty;
        self.panel = t.panel;
        self.zoom = t.zoom;
        self.pan = t.pan;
        self.selected = t.selected;
        self.undo = t.undo;
        self.redo = t.redo;
        self.recovery_id = t.recovery_id;
        self.textures.clear();
        self.pics = PictureCache::default();
        self.editing = None;
        self.drag = None;
        self.guide_sel = None;
        self.guide_drag = None;
    }

    /// Tab list with the live state of the active one.
    pub fn tab_views(&self) -> Vec<(String, bool)> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if i == self.active {
                    (self.display_name(), self.dirty)
                } else {
                    (t.name.clone().or_else(|| t.path.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).map(str::to_string)).unwrap_or_else(|| format!("Untitled {}", i + 1)), t.dirty)
                }
            })
            .collect()
    }

    pub fn display_name(&self) -> String {
        self.tab_name
            .clone()
            .or_else(|| self.path.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).map(str::to_string))
            .unwrap_or_else(|| format!("Untitled {}", self.active + 1))
    }

    pub fn switch_tab(&mut self, i: usize) {
        if i == self.active || i >= self.tabs.len() {
            return;
        }
        self.tabs[self.active] = self.stash();
        self.active = i;
        let t = self.tabs[i].clone();
        self.load(t);
    }

    /// Open a design in a new tab, or in this one if it's an untouched blank.
    pub fn open_tab(&mut self, doc: Project, path: Option<PathBuf>, dirty: bool) {
        let blank = !self.dirty && self.path.is_none() && self.doc.panels.iter().all(|p| p.items.is_empty());
        if !blank {
            self.tabs[self.active] = self.stash();
            self.tabs.push(self.stash());
            self.active = self.tabs.len() - 1;
            self.recovery_id = fresh_recovery_id();
        }
        let t = Tab { name: None, doc, path, dirty, panel: 0, zoom: 1.0, pan: egui::Vec2::ZERO, selected: Vec::new(), undo: Vec::new(), redo: Vec::new(), recovery_id: self.recovery_id };
        self.load(t);
        self.tabs[self.active] = self.stash();
        self.fit_pending = true;
    }

    pub fn close_tab(&mut self, i: usize) {
        if i >= self.tabs.len() {
            return;
        }
        let rid = if i == self.active { self.recovery_id } else { self.tabs[i].recovery_id };
        crate::recovery::forget(rid);
        if self.tabs.len() == 1 {
            self.open_tab(Project::new(Kind::Sign, false), None, false);
            self.tabs[0] = self.stash();
            self.load(Tab { name: None, doc: Project::new(Kind::Sign, false), path: None, dirty: false, panel: 0, zoom: 1.0, pan: egui::Vec2::ZERO, selected: Vec::new(), undo: Vec::new(), redo: Vec::new(), recovery_id: fresh_recovery_id() });
            self.tabs[0] = self.stash();
            self.fit_pending = true;
            return;
        }
        if i == self.active {
            self.tabs.remove(i);
            self.active = i.min(self.tabs.len() - 1);
            let t = self.tabs[self.active].clone();
            self.load(t);
            self.fit_pending = true;
        } else {
            self.tabs[self.active] = self.stash();
            self.tabs.remove(i);
            if i < self.active {
                self.active -= 1;
            }
        }
    }

    pub fn duplicate_tab(&mut self, i: usize) {
        self.tabs[self.active] = self.stash();
        let mut t = self.tabs[i].clone();
        t.name = Some(format!("{} copy", if i == self.active { self.display_name() } else { self.tab_views()[i].0.clone() }));
        t.path = None;
        t.dirty = true;
        t.recovery_id = fresh_recovery_id();
        self.tabs.insert(i + 1, t);
        if self.active > i {
            self.active += 1;
        }
    }

    pub fn move_tab(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        self.tabs[self.active] = self.stash();
        let t = self.tabs.remove(from);
        self.tabs.insert(to, t);
        if self.active == from {
            self.active = to;
        } else if from < self.active && to >= self.active {
            self.active -= 1;
        } else if from > self.active && to <= self.active {
            self.active += 1;
        }
    }

    pub fn rename_tab(&mut self, i: usize, name: String) {
        let name = (!name.trim().is_empty()).then_some(name);
        if i == self.active {
            self.tab_name = name;
        } else if let Some(t) = self.tabs.get_mut(i) {
            t.name = name;
        }
    }

    /// Every open tab with unsaved changes (keeps the active tab's live state).
    pub fn unsaved(&self) -> Vec<String> {
        self.tab_views().into_iter().filter(|(_, d)| *d).map(|(n, _)| n).collect()
    }

    pub fn page(&self) -> &Panel {
        &self.doc.panels[self.panel.min(self.doc.panels.len() - 1)]
    }

    pub fn page_mut(&mut self) -> &mut Panel {
        let i = self.panel.min(self.doc.panels.len() - 1);
        &mut self.doc.panels[i]
    }

    pub fn item(&self, id: u64) -> Option<&Item> {
        self.page().items.iter().find(|i| i.id == id)
    }

    pub fn item_mut(&mut self, id: u64) -> Option<&mut Item> {
        self.page_mut().items.iter_mut().find(|i| i.id == id)
    }

    pub fn first_selected(&self) -> Option<&Item> {
        self.selected.first().and_then(|id| self.item(*id))
    }

    pub fn checkpoint(&mut self) {
        self.undo.push(self.doc.clone());
        if self.undo.len() > undo_depth(&self.settings) {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    pub fn notify(&mut self, msg: impl Into<String>, ctx: &egui::Context) {
        self.toast = Some((msg.into(), ctx.input(|i| i.time) + 4.0));
    }

    pub fn set_look(&mut self, ctx: &egui::Context, look: Look) {
        self.settings.look = look;
        theme::apply(ctx, look);
        self.settings.save();
    }

    /// Put a new item in the middle of the page and select it.
    pub fn add_item(&mut self, w: f32, h: f32, content: Content) -> u64 {
        self.checkpoint();
        let id = self.doc.fresh_id();
        let (pw, ph) = (self.doc.width, self.doc.height);
        let (w, h) = (w.min(pw * 0.95), h.min(ph * 0.95));
        let item = Item::new(id, Frame::new((pw - w) / 2.0, (ph - h) / 2.0, w, h), content);
        self.page_mut().items.push(item);
        self.selected = vec![id];
        id
    }

    pub fn add_picture_bytes(&mut self, name: &str, bytes: Vec<u8>, credit: &str, ctx: &egui::Context) {
        let Some(pm) = crate::raster::decode_picture(&bytes) else {
            self.notify("That picture couldn't be opened.", ctx);
            return;
        };
        let aspect = pm.width() as f32 / pm.height().max(1) as f32;
        let side = (self.doc.width.min(self.doc.height) * 0.45).max(1.0);
        let (w, h) = if aspect >= 1.0 { (side, side / aspect) } else { (side * aspect, side) };
        // Metafiles become pictures at a good printing size.
        let bytes = if bytes.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) {
            let rgba = crate::raster::pixmap_to_rgba(&pm);
            image::RgbaImage::from_raw(pm.width(), pm.height(), rgba).map(|i| crate::photo::to_png(&image::DynamicImage::ImageRgba8(i))).unwrap_or(bytes)
        } else {
            bytes
        };
        let asset = self.doc.add_asset(name, bytes, credit);
        self.add_item(w, h, Content::Picture { asset });
    }

    pub fn open_path(&mut self, path: PathBuf) {
        let Ok(bytes) = std::fs::read(&path) else {
            self.dialogs.push(Dialog::message("Couldn't open", &format!("PrintCraft couldn't read {}.", path.display())));
            return;
        };
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("project").to_string();
        self.open_file(Opened { name, bytes, path: Some(path) });
    }

    /// Open a design (or an older project) in a new tab.
    pub fn open_file(&mut self, f: Opened) {
        let ext = f.extension();
        let bytes = f.bytes;
        let result = if ext == "pcraft" || bytes.starts_with(b"{") {
            Project::from_json(&bytes).map(|p| (p, true))
        } else {
            let kind = Kind::from_legacy_extension(&ext).unwrap_or(Kind::Sign);
            crate::legacy::sig::read(&bytes).map(|l| (crate::legacy::sig::to_project(l, kind), false))
        };
        match result {
            Ok((doc, native)) => {
                let notes = doc.notes.clone();
                self.open_tab(doc, if native { f.path.clone() } else { None }, !native);
                if let Some(path) = f.path {
                    self.settings.remember(path);
                }
                self.settings.save();
                if !native {
                    self.toast = Some(("Opened your older project. Its words and pictures are on the page — arrange them as you like. (Edit > Project Notes has details.)".into(), f64::MAX));
                    self.toast_until_click = true;
                }
            }
            Err(e) => self.dialogs.push(Dialog::message("Couldn't open", &e)),
        }
    }

    pub fn save(&mut self, ctx: &egui::Context, ask: bool) {
        // Save over the design's own file, or ask where (in the browser the
        // design is downloaded).
        if let (Some(path), false) = (self.path.clone(), ask) {
            match self.doc.to_json().and_then(|b| crate::platform::write_path(&path, &b)) {
                Ok(()) => self.saved_design(Some(path), ctx),
                Err(e) => self.dialogs.push(Dialog::message("Couldn't save", &e)),
            }
            return;
        }
        let filters: [(&str, &[&str]); 3] = [
            ("PrintCraft design (.pcraft)", &["pcraft"]),
            ("PDF for printing or email (.pdf)", &["pdf"]),
            ("Picture (.png, .jpg)", &["png", "jpg", "jpeg"]),
        ];
        let mut chosen = String::from("pcraft");
        let r = crate::platform::save_file(&format!("{}.pcraft", self.display_name()), &filters, |ext| {
            chosen = ext.to_string();
            match ext {
                // Other formats are copies; the design itself stays as it was.
                "pdf" => crate::export::pdf_bytes(&self.doc, &self.fonts, &self.pics, &crate::export::PrintOptions::default()),
                "png" | "jpg" | "jpeg" => crate::export::picture_bytes(&self.doc, self.panel, &self.fonts, &self.pics, ext, 300.0),
                e if Kind::from_legacy_extension(e).is_some() => Err(OLDER_FORMAT.into()),
                _ => self.doc.to_json(),
            }
        });
        match r {
            Ok(None) => {}
            Ok(Some(_)) if matches!(chosen.as_str(), "pdf" | "png" | "jpg" | "jpeg") => {
                self.notify(format!("Saved a copy as {}.", chosen.to_uppercase()), ctx);
            }
            Ok(Some(crate::platform::Saved::File(path))) => self.saved_design(Some(path), ctx),
            Ok(Some(crate::platform::Saved::Downloaded)) => self.saved_design(None, ctx),
            Err(e) if e == OLDER_FORMAT => self.dialogs.push(Dialog::message("Older project format", OLDER_FORMAT)),
            Err(e) => self.dialogs.push(Dialog::message("Couldn't save", &e)),
        }
    }

    fn saved_design(&mut self, path: Option<PathBuf>, ctx: &egui::Context) {
        self.dirty = false;
        if let Some(path) = path {
            self.path = Some(path.clone());
            self.settings.remember(path);
            self.settings.save();
            self.notify("Saved.", ctx);
        } else {
            self.notify("Downloaded your design. Open it again with File > Open.", ctx);
        }
    }

    fn selection_frames(&self) -> Vec<(u64, Frame)> {
        self.selected.iter().filter_map(|id| self.item(*id).map(|i| (*id, i.frame))).collect()
    }

    fn each_selected(&mut self, f: impl Fn(&mut Item, f32, f32)) {
        if self.selected.is_empty() {
            return;
        }
        self.checkpoint();
        let (pw, ph) = (self.doc.width, self.doc.height);
        let ids = self.selected.clone();
        for it in self.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id)) {
            f(it, pw, ph);
        }
    }

    fn each_text(&mut self, f: impl Fn(&mut TextLook)) {
        if let Some((id, _)) = &self.editing {
            let id = *id;
            self.checkpoint();
            if let Some(l) = self.item_mut(id).and_then(|i| i.look_mut()) {
                f(l);
            }
            return;
        }
        self.each_selected(|i, _, _| {
            if let Some(l) = i.look_mut() {
                f(l)
            }
        });
    }

    pub fn needs_selection(act: &Act) -> bool {
        matches!(
            act,
            Act::Cut
                | Act::Copy
                | Act::Duplicate
                | Act::Delete
                | Act::CenterPage
                | Act::CenterH
                | Act::CenterV
                | Act::FullPage
                | Act::Rotate(_)
                | Act::FlipH
                | Act::FlipV
                | Act::ToFront
                | Act::ToBack
                | Act::Forward
                | Act::Backward
                | Act::Lock
                | Act::Shadow
                | Act::Glow
                | Act::Frame
                | Act::Tint(_)
        )
    }

    pub fn enabled(&self, act: &Act) -> bool {
        let sel = self.first_selected();
        match act {
            Act::Undo => !self.undo.is_empty(),
            Act::Redo => !self.redo.is_empty(),
            Act::Paste => true,
            Act::Photo(_) => matches!(sel.map(|i| &i.content), Some(Content::Picture { .. })),
            Act::TextShape | Act::SpeechBubble | Act::StretchText => matches!(sel.map(|i| &i.content), Some(Content::Text { .. })),
            Act::Bold | Act::Italic | Act::Underline | Act::Align(_) | Act::VAlign(_) => self.editing.is_some() || sel.is_some_and(|i| i.look().is_some()),
            Act::AddressMerge => !self.settings.contacts.is_empty(),
            Act::Group | Act::Distribute(_) => self.selected.len() >= 2,
            Act::Ungroup => matches!(sel.map(|i| &i.content), Some(Content::Group { .. })),
            Act::FitText => matches!(sel.map(|i| &i.content), Some(Content::Text { .. })),
            Act::SaveToMyArt => matches!(sel.map(|i| &i.content), Some(Content::Picture { .. })),
            Act::AlignItems(_) | Act::Opacity(_) => sel.is_some(),
            a if Self::needs_selection(a) => sel.is_some(),
            _ => true,
        }
    }

    pub fn run(&mut self, act: Act, ctx: &egui::Context) {
        if !self.enabled(&act) {
            return;
        }
        match act {
            Act::NewProject => self.dialogs.push(Dialog::new_project()),
            Act::Open => {
                let mut exts = vec!["pcraft"];
                exts.extend(Kind::legacy_extensions());
                crate::platform::pick_file(Want::Project, "Projects (PrintCraft and older)", &exts, ctx);
            }
            Act::Save => self.save(ctx, false),
            Act::SaveAs => self.save(ctx, true),
            Act::SavePdf => {
                match crate::platform::save_file("My Project.pdf", &[("PDF", &["pdf"])], |_| crate::export::pdf_bytes(&self.doc, &self.fonts, &self.pics, &crate::export::PrintOptions::default())) {
                    Ok(Some(_)) => self.notify("Saved as PDF.", ctx),
                    Ok(None) => {}
                    Err(e) => self.dialogs.push(Dialog::message("Couldn't save the PDF", &e)),
                }
            }
            Act::ExportPicture => {
                match crate::platform::save_file("My Project.png", &[("Picture", &["png", "jpg", "bmp"])], |ext| crate::export::picture_bytes(&self.doc, self.panel, &self.fonts, &self.pics, ext, 300.0)) {
                    Ok(Some(_)) => self.notify("Picture saved.", ctx),
                    Ok(None) => {}
                    Err(e) => self.dialogs.push(Dialog::message("Couldn't save the picture", &e)),
                }
            }
            Act::Print => self.dialogs.push(Dialog::print()),
            Act::PrintPreview => self.dialogs.push(Dialog::print_window(true)),
            Act::Exit => self.quit = true,
            Act::Undo => {
                if let Some(prev) = self.undo.pop() {
                    self.redo.push(std::mem::replace(&mut self.doc, prev));
                    self.after_history();
                }
            }
            Act::Redo => {
                if let Some(next) = self.redo.pop() {
                    self.undo.push(std::mem::replace(&mut self.doc, next));
                    self.after_history();
                }
            }
            Act::Copy | Act::Cut => {
                self.clipboard = self.page().items.iter().filter(|i| self.selected.contains(&i.id)).cloned().collect();
                if !crate::platform::WEB {
                    crate::platform::set_clipboard_text(ctx, COPY_MARK);
                }
                if act == Act::Cut {
                    self.run(Act::Delete, ctx);
                }
            }
            Act::Paste if self.paste_external(ctx) => {}
            Act::Paste | Act::Duplicate => {
                let items: Vec<Item> = if act == Act::Duplicate {
                    self.page().items.iter().filter(|i| self.selected.contains(&i.id)).cloned().collect()
                } else {
                    self.clipboard.clone()
                };
                if items.is_empty() {
                    return;
                }
                self.checkpoint();
                let mut new_ids = Vec::new();
                for mut it in items {
                    it.id = self.doc.fresh_id();
                    it.frame.x += 0.25;
                    it.frame.y += 0.25;
                    new_ids.push(it.id);
                    self.page_mut().items.push(it);
                }
                self.selected = new_ids;
            }
            Act::SelectAll => self.selected = self.page().items.iter().map(|i| i.id).collect(),
            Act::Delete => {
                self.checkpoint();
                let ids = self.selected.clone();
                self.page_mut().items.retain(|i| !ids.contains(&i.id) || i.locked);
                self.selected.clear();
            }
            Act::ZoomIn => self.zoom = (self.zoom * 1.25).min(6.0),
            Act::ZoomOut => self.zoom = (self.zoom / 1.25).max(0.1),
            Act::Fit => self.fit_pending = true,
            Act::Zoom(z) => self.zoom = z,
            Act::ToggleRulers => {
                self.settings.show_rulers = !self.settings.show_rulers;
                self.settings.save();
            }
            Act::Panel(i) => {
                self.panel = i.min(self.doc.panels.len() - 1);
                self.selected.clear();
                self.editing = None;
                self.guide_sel = None;
                self.scroll_to = Some(self.panel);
            }
            Act::ToggleLook => {
                // Choosing yourself stops following the computer's setting.
                self.settings.follow_system = false;
                let next = if self.settings.look == Look::Light { Look::Dark } else { Look::Light };
                self.set_look(ctx, next);
            }
            Act::Gallery(tab) => self.dialogs.push(Dialog::gallery(tab)),
            Act::PictureFromFile => {
                crate::platform::pick_file(Want::Picture, "Pictures", &["png", "jpg", "jpeg", "bmp", "gif", "wmf"], ctx);
            }
            Act::Headline => self.dialogs.push(Dialog::headline(None, self)),
            Act::TextBox => {
                let look = TextLook { size: 18.0, ..Default::default() };
                let id = self.add_item(3.5, 1.2, Content::Text { text: String::new(), look, shape: None, fit: false, runs: Vec::new() });
                self.editing = Some((id, String::new()));
            }
            Act::TextInShape => {
                let look = TextLook { size: 16.0, align: Align::Center, valign: VAlign::Middle, ..Default::default() };
                let id = self.add_item(3.0, 2.2, Content::Text { text: "Your words here".into(), look, shape: Some(Outline::Ellipse), fit: false, runs: Vec::new() });
                if let Some(it) = self.item_mut(id) {
                    it.fill = Rgba::hex("#fff4c2");
                    it.stroke = Rgba::hex("#2f6fb0");
                    it.stroke_pt = 2.0;
                }
            }
            Act::Border(style) => {
                self.checkpoint();
                let id = self.doc.fresh_id();
                let (w, h) = (self.doc.width, self.doc.height);
                let mut it = Item::new(id, Frame::new(0.25, 0.25, w - 0.5, h - 0.5), Content::Border { style });
                it.stroke = self.line_color;
                it.fill = self.fill_color;
                self.page_mut().items.insert(0, it);
                self.selected = vec![id];
            }
            Act::Backdrop | Act::BackgroundColor => self.dialogs.push(Dialog::Backdrop),
            Act::Shape(o) => {
                let fill = self.fill_color;
                let stroke = self.line_color;
                let id = self.add_item(2.0, 2.0, Content::Shape { outline: o });
                if let Some(it) = self.item_mut(id) {
                    it.fill = fill;
                    it.stroke = stroke;
                    it.stroke_pt = 1.5;
                }
            }
            Act::Line => {
                let c = self.line_color;
                let id = self.add_item(3.0, 0.02, Content::Line);
                if let Some(it) = self.item_mut(id) {
                    it.stroke = c;
                    it.stroke_pt = crate::effects::FRAME_MEDIUM * 72.0;
                }
            }
            Act::Seal => {
                let look = TextLook { size: 20.0, bold: true, align: Align::Center, valign: VAlign::Middle, color: Rgba::hex("#7a4b00"), ..Default::default() };
                let id = self.add_item(2.2, 2.2, Content::Text { text: "Award\nof Merit".into(), look, shape: Some(Outline::Burst), fit: false, runs: Vec::new() });
                if let Some(it) = self.item_mut(id) {
                    it.fill = Rgba::hex("#f2b631");
                    it.stroke = Rgba::hex("#a06a00");
                    it.stroke_pt = 2.0;
                }
            }
            Act::InitialCap => {
                let look = TextLook { color: Rgba::hex("#c0392b"), ..Default::default() };
                self.add_item(1.2, 1.4, Content::Headline { text: "A".into(), look, flair: Flair::ShadowRight, warp: Warp::Straight, second: Rgba::hex("#f2b631"), outline: Rgba::hex("#5a1a12"), stretch: false, bend: 1.0, bulge: 0.0 });
            }
            Act::Number => {
                let look = TextLook { size: 40.0, bold: true, align: Align::Center, valign: VAlign::Middle, color: Rgba::WHITE, ..Default::default() };
                let id = self.add_item(1.4, 1.4, Content::Text { text: "1".into(), look, shape: Some(Outline::Ellipse), fit: false, runs: Vec::new() });
                if let Some(it) = self.item_mut(id) {
                    it.fill = Rgba::hex("#2f8fd8");
                    it.stroke = Rgba::hex("#1d5f94");
                }
            }
            Act::Signature => self.dialogs.push(Dialog::signature()),
            Act::CenterPage => self.each_selected(|i, pw, ph| {
                i.frame.x = (pw - i.frame.w) / 2.0;
                i.frame.y = (ph - i.frame.h) / 2.0;
            }),
            Act::CenterH => self.each_selected(|i, pw, _| i.frame.x = (pw - i.frame.w) / 2.0),
            Act::CenterV => self.each_selected(|i, _, ph| i.frame.y = (ph - i.frame.h) / 2.0),
            Act::FullPage => self.each_selected(|i, pw, ph| i.frame = Frame::new(0.0, 0.0, pw, ph)),
            Act::Rotate(d) => self.each_selected(|i, _, _| i.rotation = (i.rotation + d).rem_euclid(360.0)),
            Act::FlipH => self.each_selected(|i, _, _| i.flip_x = !i.flip_x),
            Act::FlipV => self.each_selected(|i, _, _| i.flip_y = !i.flip_y),
            Act::Lock => self.each_selected(|i, _, _| i.locked = !i.locked),
            Act::ToFront | Act::ToBack | Act::Forward | Act::Backward => {
                self.checkpoint();
                let ids = self.selected.clone();
                let items = &mut self.page_mut().items;
                for id in ids {
                    let Some(pos) = items.iter().position(|i| i.id == id) else { continue };
                    let it = items.remove(pos);
                    let to = match act {
                        Act::ToFront => items.len(),
                        Act::ToBack => 0,
                        Act::Forward => (pos + 1).min(items.len()),
                        _ => pos.saturating_sub(1),
                    };
                    items.insert(to, it);
                }
            }
            Act::Shadow | Act::Glow | Act::Frame => self.dialogs.push(Dialog::effects(&act)),
            Act::Tint(t) => self.each_selected(|i, _, _| i.tint = t),
            Act::Bold => {
                if !self.style_selection(&|s, cs| s.bold = Some(!cs.iter().all(|c| c.bold))) {
                    self.each_text(|l| l.bold = !l.bold)
                }
            }
            Act::Italic => {
                if !self.style_selection(&|s, cs| s.italic = Some(!cs.iter().all(|c| c.italic))) {
                    self.each_text(|l| l.italic = !l.italic)
                }
            }
            Act::Underline => {
                if !self.style_selection(&|s, cs| s.underline = Some(!cs.iter().all(|c| c.underline))) {
                    self.each_text(|l| l.underline = !l.underline)
                }
            }
            Act::Align(a) => self.each_text(|l| l.align = a),
            Act::VAlign(a) => self.each_text(|l| l.valign = a),
            Act::TextShape => self.dialogs.push(Dialog::Shapes),
            Act::SpeechBubble => self.each_selected(|i, _, _| {
                if let Content::Text { shape, .. } = &mut i.content {
                    *shape = Some(Outline::Balloon);
                    if i.fill.is_clear() {
                        i.fill = Rgba::WHITE;
                    }
                    if i.stroke.is_clear() {
                        i.stroke = Rgba::BLACK;
                        i.stroke_pt = 2.0;
                    }
                }
            }),
            Act::StretchText => self.each_selected(|i, _, _| {
                if let Content::Headline { stretch, .. } = &mut i.content {
                    *stretch = !*stretch;
                    return;
                }
                if let Content::Text { fit, .. } = &mut i.content {
                    *fit = !*fit;
                }
            }),
            Act::AddressMerge => self.dialogs.push(Dialog::AddressPick),
            Act::Photo(tab) => {
                if let Some(Content::Picture { asset }) = self.first_selected().map(|i| i.content.clone()) {
                    let id = self.selected[0];
                    self.dialogs.push(Dialog::photo(id, asset, tab, self, ctx));
                }
            }
            Act::AddressBook => self.dialogs.push(Dialog::AddressBook { editing: None }),
            Act::Reminders => self.dialogs.push(Dialog::Reminders),
            Act::ContentPacks => self.dialogs.push(Dialog::ContentPacks),
            Act::GetFonts => {
                let d = Dialog::fonts(self, ctx);
                self.dialogs.push(d);
            }
            Act::About => self.dialogs.push(Dialog::message(
                "About PrintCraft",
                &format!(
                    "PrintCraft {}\n\nMake cards, signs, banners, labels and more, then print them at home.\n\nPrintCraft can open older .sig-style project files and use art collections you already own.\n\nPicture search uses Google Images (pictures there belong to the people who made them); online fonts come from Fontsource (open licenses). The bundled fonts are Nunito and Dancing Script (SIL Open Font License).\n\nFree software under the GNU GPL, version 3 or later.\nhttps://github.com/amruthwo/PrintCraft",
                    env!("CARGO_PKG_VERSION")
                ),
            )),
            Act::Help => self.dialogs.push(Dialog::message(
                "Getting around",
                "• Pick a tool group on the left to open it, then click a tool.\n• Click something on the page to select it; drag to move it; drag the small squares to resize; drag the round handle to turn it.\n• Double-click text to change the words. Use the bar above the page for font, size and style.\n• Right-click anything for more choices.\n• Ctrl+Z undoes, Ctrl+Y redoes. Ctrl+P prints.",
            )),
            Act::Group => {
                self.checkpoint();
                let ids = self.selected.clone();
                let items = &mut self.page_mut().items;
                let picked: Vec<(usize, Item)> = items.iter().enumerate().filter(|(_, i)| ids.contains(&i.id)).map(|(n, i)| (n, i.clone())).collect();
                let x0 = picked.iter().map(|(_, i)| i.frame.x).fold(f32::MAX, f32::min);
                let y0 = picked.iter().map(|(_, i)| i.frame.y).fold(f32::MAX, f32::min);
                let x1 = picked.iter().map(|(_, i)| i.frame.x + i.frame.w).fold(f32::MIN, f32::max);
                let y1 = picked.iter().map(|(_, i)| i.frame.y + i.frame.h).fold(f32::MIN, f32::max);
                let at = picked.iter().map(|(n, _)| *n).max().unwrap_or(0) + 1 - picked.len();
                items.retain(|i| !ids.contains(&i.id));
                let children: Vec<Item> = picked
                    .into_iter()
                    .map(|(_, mut i)| {
                        i.frame.x -= x0;
                        i.frame.y -= y0;
                        i
                    })
                    .collect();
                let id = self.doc.fresh_id();
                let g = Item::new(id, Frame::new(x0, y0, x1 - x0, y1 - y0), Content::Group { items: children, base_w: x1 - x0, base_h: y1 - y0 });
                let items = &mut self.page_mut().items;
                items.insert(at.min(items.len()), g);
                self.selected = vec![id];
            }
            Act::Ungroup => {
                let Some(g) = self.first_selected().cloned() else { return };
                let Content::Group { items: children, base_w, base_h } = g.content else { return };
                self.checkpoint();
                let (sx, sy) = (g.frame.w / base_w.max(0.01), g.frame.h / base_h.max(0.01));
                let c = g.frame.center();
                let mut out = Vec::new();
                for mut ch in children {
                    ch.frame = Frame::new(g.frame.x + ch.frame.x * sx, g.frame.y + ch.frame.y * sy, ch.frame.w * sx, ch.frame.h * sy);
                    if let Some(l) = ch.look_mut() {
                        l.size *= sx.min(sy);
                    }
                    if g.rotation.abs() > 0.01 {
                        let (s, co) = g.rotation.to_radians().sin_cos();
                        let (cx, cy) = ch.frame.center();
                        let (dx, dy) = (cx - c.0, cy - c.1);
                        let (nx, ny) = (c.0 + dx * co - dy * s, c.1 + dx * s + dy * co);
                        ch.frame.x += nx - cx;
                        ch.frame.y += ny - cy;
                        ch.rotation = (ch.rotation + g.rotation).rem_euclid(360.0);
                    }
                    ch.id = self.doc.fresh_id();
                    out.push(ch);
                }
                let ids: Vec<u64> = out.iter().map(|i| i.id).collect();
                let items = &mut self.page_mut().items;
                let at = items.iter().position(|i| i.id == g.id).unwrap_or(items.len());
                items.remove(at.min(items.len().saturating_sub(1)));
                for (k, it) in out.into_iter().enumerate() {
                    items.insert((at + k).min(items.len()), it);
                }
                self.selected = ids;
            }
            Act::AlignItems(how) => {
                let frames: Vec<Frame> = self.selected.iter().filter_map(|id| self.item(*id).map(|i| i.frame)).collect();
                let (x0, y0, x1, y1) = if frames.len() <= 1 {
                    (0.0, 0.0, self.doc.width, self.doc.height)
                } else {
                    (
                        frames.iter().map(|f| f.x).fold(f32::MAX, f32::min),
                        frames.iter().map(|f| f.y).fold(f32::MAX, f32::min),
                        frames.iter().map(|f| f.x + f.w).fold(f32::MIN, f32::max),
                        frames.iter().map(|f| f.y + f.h).fold(f32::MIN, f32::max),
                    )
                };
                self.each_selected(move |i, _, _| match how {
                    0 => i.frame.x = x0,
                    1 => i.frame.x = (x0 + x1 - i.frame.w) / 2.0,
                    2 => i.frame.x = x1 - i.frame.w,
                    3 => i.frame.y = y0,
                    4 => i.frame.y = (y0 + y1 - i.frame.h) / 2.0,
                    _ => i.frame.y = y1 - i.frame.h,
                });
            }
            Act::Distribute(across) => {
                let mut frames: Vec<(u64, Frame)> = self.selected.iter().filter_map(|id| self.item(*id).map(|i| (*id, i.frame))).collect();
                if frames.len() < 3 {
                    return;
                }
                let key = |f: &Frame| if across { f.x + f.w / 2.0 } else { f.y + f.h / 2.0 };
                frames.sort_by(|a, b| key(&a.1).partial_cmp(&key(&b.1)).unwrap_or(std::cmp::Ordering::Equal));
                let (first, last) = (key(&frames[0].1), key(&frames[frames.len() - 1].1));
                let step = (last - first) / (frames.len() - 1) as f32;
                self.checkpoint();
                for (k, (id, _)) in frames.iter().enumerate() {
                    let target = first + step * k as f32;
                    if let Some(it) = self.item_mut(*id) {
                        if across {
                            it.frame.x = target - it.frame.w / 2.0;
                        } else {
                            it.frame.y = target - it.frame.h / 2.0;
                        }
                    }
                }
            }
            Act::MoreShapes => self.dialogs.push(Dialog::Shapes),
            Act::Borders => self.dialogs.push(Dialog::borders()),
            Act::Opacity(o) => self.each_selected(move |i, _, _| i.opacity = o),
            Act::FitText => self.each_selected(|i, _, _| {
                if let Content::Text { fit, .. } = &mut i.content {
                    *fit = !*fit;
                }
            }),
            Act::Settings => self.dialogs.push(Dialog::Settings),
            Act::HelpTopics => self.dialogs.push(Dialog::Help { query: String::new() }),
            Act::NewTab => {
                self.tabs[self.active] = self.stash();
                self.tabs.push(self.stash());
                self.active = self.tabs.len() - 1;
                self.load(Tab { name: None, doc: Project::new(Kind::Sign, false), path: None, dirty: false, panel: 0, zoom: 1.0, pan: egui::Vec2::ZERO, selected: Vec::new(), undo: Vec::new(), redo: Vec::new(), recovery_id: fresh_recovery_id() });
                self.tabs[self.active] = self.stash();
                self.fit_pending = true;
                self.dialogs.push(Dialog::new_project());
            }
            Act::CloseTab(i) => {
                let dirty = if i == self.active { self.dirty } else { self.tabs.get(i).is_some_and(|t| t.dirty) };
                if dirty {
                    self.dialogs.push(Dialog::ConfirmCloseTab(i));
                } else {
                    self.close_tab(i);
                }
            }
            Act::SwitchTab(i) => self.switch_tab(i),
            Act::AddPanel => {
                self.checkpoint();
                let n = self.doc.panels.len() + 1;
                let bg = self.page().background;
                let mut page = self.page().clone();
                page.name = format!("Page {n}");
                page.background = bg;
                page.items.clear();
                page.guides.clear();
                self.doc.panels.push(page);
                self.panel = self.doc.panels.len() - 1;
                self.selected.clear();
                self.guide_sel = None;
                self.scroll_to = Some(self.panel);
            }
            Act::MyArt => self.dialogs.push(Dialog::gallery(2)),
            Act::SaveToMyArt => {
                if let Some(Content::Picture { asset }) = self.first_selected().map(|i| i.content.clone())
                    && let Some(a) = self.doc.assets.get(&asset).cloned()
                {
                    match crate::library::save(&a.name, &a.bytes, &a.credit) {
                        Ok(_) => self.notify("Saved to My Art.", ctx),
                        Err(e) => self.notify(e, ctx),
                    }
                }
            }
            Act::AiPictures => self.dialogs.push(Dialog::ai()),
            Act::PickColorFromPage(fill) => {
                self.eyedropper = Some(if fill { 0 } else { 1 });
                self.notify("Click anywhere on the page to pick that color.", ctx);
            }
            Act::DeleteGuide => {
                if let Some((p, i)) = self.guide_sel.take()
                    && self.doc.panels.get(p).is_some_and(|pg| i < pg.guides.len())
                {
                    self.checkpoint();
                    self.doc.panels[p].guides.remove(i);
                }
            }
            Act::ClearGuides => {
                if !self.page().guides.is_empty() {
                    self.checkpoint();
                    self.page_mut().guides.clear();
                    self.guide_sel = None;
                    self.notify("Guidelines removed from this page.", ctx);
                }
            }
            Act::ToggleGuides => {
                self.settings.show_guides = !self.settings.show_guides;
                self.settings.save();
            }
            Act::ToggleLegacyPages => {
                self.settings.legacy_pages = !self.settings.legacy_pages;
                self.settings.save();
                self.fit_pending = true;
            }
            Act::ToggleTilePages => {
                self.settings.tile_pages = !self.settings.tile_pages;
                self.settings.save();
                self.fit_pending = true;
            }
            Act::ProjectNotes => {
                let notes = if self.doc.notes.is_empty() { "No notes for this project.".to_string() } else { self.doc.notes.join("\n\n") };
                self.dialogs.push(Dialog::message("About this project", &notes));
            }
        }
    }

    fn after_history(&mut self) {
        let ids: Vec<u64> = self.page().items.iter().map(|i| i.id).collect();
        self.selected.retain(|s| ids.contains(s));
        self.panel = self.panel.min(self.doc.panels.len() - 1);
        self.editing = None;
        self.pics = PictureCache::default();
        if self.guide_sel.is_some_and(|(p, i)| self.doc.panels.get(p).is_none_or(|pg| i >= pg.guides.len())) {
            self.guide_sel = None;
        }
    }

    pub fn nudge(&mut self, dx: f32, dy: f32) {
        if self.selected.is_empty() {
            return;
        }
        // Group consecutive nudges into one undo step.
        if self.undo.last().is_none_or(|u| u != &self.doc) {
            self.checkpoint();
        }
        let ids = self.selected.clone();
        for it in self.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id) && !i.locked) {
            it.frame.x += dx;
            it.frame.y += dy;
        }
    }

    pub fn begin_move(&mut self, at: (f32, f32)) {
        self.drag_snapshot = Some(self.doc.clone());
        self.drag = Some(Drag::Move { start: self.selection_frames(), origin: at });
    }

    pub fn end_drag(&mut self) {
        if let Some(snap) = self.drag_snapshot.take()
            && snap != self.doc
        {
            self.undo.push(snap);
            if self.undo.len() > undo_depth(&self.settings) {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.dirty = true;
        }
        self.drag = None;
    }

    /// Finish typing. The text was kept in step while typing (one undo
    /// step was taken when editing began); drop that step if nothing changed.
    pub fn commit_edit(&mut self) {
        if self.editing.take().is_some() {
            if self.undo.last().is_some_and(|u| u == &self.doc) {
                self.undo.pop();
            }
            self.edit_sel = None;
        }
    }

    /// Style only the selected words of the text being edited. Returns
    /// false when nothing is selected (the whole box is styled instead).
    pub fn style_selection(&mut self, f: &dyn Fn(&mut Span, &[CharStyle])) -> bool {
        let (Some((id, _)), Some((a, b))) = (self.editing.clone(), self.edit_sel) else { return false };
        if a >= b {
            return false;
        }
        self.checkpoint();
        if let Some(Content::Text { text, look, runs, .. }) = self.item_mut(id).map(|i| &mut i.content) {
            let n = text.chars().count();
            let (a, b) = (a.min(n), b.min(n));
            let styles = char_styles(look, runs, n);
            let mut span = Span { start: a, end: b, ..Default::default() };
            f(&mut span, &styles[a..b]);
            runs.push(span);
        }
        true
    }

    pub fn title(&self) -> String {
        let name = self.path.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).unwrap_or("Untitled");
        format!("{}{} – {} – PrintCraft", if self.dirty { "• " } else { "" }, name, self.doc.kind.label())
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
