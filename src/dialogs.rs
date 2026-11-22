//! The windows that open over the editor.

use std::collections::HashMap;

use eframe::egui::{self, Color32, RichText, TextureHandle};

use crate::app::{Act, App};
use crate::kinds::Kind;
use crate::model::*;
use crate::online::{Reply, WebPicture};

/// Google Images, opened in the person's web browser (Google only answers
/// real browsers). `kind` and the switches map to Google's own filters.
pub fn google_images_url(words: &str, kind: usize, free: bool, clear: bool) -> String {
    let q: String = words
        .trim()
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    if q.is_empty() {
        return "https://images.google.com/".into();
    }
    let mut tbs = Vec::new();
    match kind {
        1 => tbs.push("itp:clipart"),
        2 => tbs.push("itp:photo"),
        3 => tbs.push("itp:lineart"),
        _ => {}
    }
    if clear {
        tbs.push("ic:trans");
    }
    if free {
        tbs.push("sur:cl");
    }
    let mut url = format!("https://www.google.com/search?udm=2&q={q}");
    if !tbs.is_empty() {
        url.push_str("&tbs=");
        url.push_str(&tbs.join(","));
    }
    url
}

/// What's on the clipboard now: a fingerprint, plus a picture (RGBA) or a
/// picture address if there is one.
fn clipboard_look() -> Option<(u64, Option<(u32, u32, Vec<u8>)>, Option<String>)> {
    use std::hash::{Hash, Hasher};
    if crate::platform::WEB {
        return None;
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    if let Some((w, ht, rgba)) = crate::platform::clipboard_image() {
        rgba.hash(&mut h);
        (w, ht).hash(&mut h);
        return Some((h.finish(), Some((w, ht, rgba)), None));
    }
    let t = crate::platform::clipboard_text().unwrap_or_default();
    t.hash(&mut h);
    let t1 = t.trim();
    let url = ((t1.starts_with("https://") || t1.starts_with("http://")) && !t1.contains(char::is_whitespace)).then(|| t1.to_string());
    Some((h.finish(), None, url))
}

use crate::settings::Contact;

pub struct GalleryState {
    pub tab: u8,
    pub words: String,
    pub pack: Option<usize>,
    pub hits: Vec<usize>,
    pub page: usize,
    pub thumbs: HashMap<usize, Option<TextureHandle>>,
    /// Google Images search: 0 any picture, 1 clip art, 2 photos, 3 line drawings.
    pub web_kind: usize,
    /// Only pictures with open (Creative Commons) licenses.
    pub web_free: bool,
    /// Only pictures with see-through backgrounds.
    pub web_clear: bool,
    /// Watching the clipboard for a picture copied in the web browser:
    /// what was there before, when it was last checked, and what came in.
    pub clip_seen: Option<u64>,
    pub clip_poll: f64,
    pub clip_pic: Option<(Vec<u8>, TextureHandle)>,
    pub clip_url: Option<String>,
    /// Searching in the web browser (copy a picture to bring it in)
    /// rather than PrintCraft's own Google Images window.
    pub in_browser: bool,
    /// Also save picked pictures to My Art.
    pub keep: bool,
    pub mine: Vec<std::path::PathBuf>,
    pub searched: bool,
}

pub use crate::photo_editor::PhotoState;

/// Page Setup: the paper (or sheet, envelope, banner length) and direction.
pub struct PageSetupState {
    /// Index into kinds::PAPERS; PAPERS.len() means a custom size.
    pub paper: usize,
    pub custom: (f32, f32),
    pub landscape: bool,
    /// Scale and centre what's on the pages to the new size.
    pub fit: bool,
    /// Label/card sheet or envelope size.
    pub option: usize,
    pub sheets: u32,
    pub tall: bool,
}

pub struct HeadlineState {
    pub edit: Option<u64>,
    pub text: String,
    pub look: TextLook,
    pub flair: Flair,
    pub warp: Warp,
    pub second: Rgba,
    pub outline: Rgba,
    pub stretch: bool,
    pub bend: f32,
    pub bulge: f32,
    pub preview: Option<(u64, TextureHandle)>,
}

pub enum Dialog {
    Welcome,
    /// `option`: which label sheet or envelope size; `sheets`/`tall`: banners.
    /// `design`: 0 blank, n = the nth ready-made design for the kind.
    /// `cal`: the calendar maker's choices (calendars).
    NewProject { kind: Kind, landscape: bool, option: usize, sheets: u32, tall: bool, design: usize, cal: Box<crate::calendar::CalendarOptions> },
    /// Calendar Maker for the open calendar.
    CalendarMaker(Box<crate::calendar::CalendarOptions>),
    Message { title: String, body: String },
    Headline(Box<HeadlineState>),
    Gallery(Box<GalleryState>),
    Photo(Box<PhotoState>),
    PageSetup(Box<PageSetupState>),
    /// Where the new picture for Replace Picture comes from.
    ReplacePicture,
    /// Flourishes & Dividers to add to the page.
    Ornaments,
    Update(Box<crate::update::UpdateState>),
    Report(Box<crate::report::ReportState>),
    /// Shadow: amount = distance, angle, size = softness. Glow: size.
    /// Frame: style number.
    Effects { which: u8, amount: f32, angle: f32, size: f32, color: Rgba, style: u8 },
    TextShape,
    Backdrop,
    Print { copies: u32 },
    Preview { pages: Vec<TextureHandle>, at: usize },
    AddressBook { editing: Option<usize> },
    AddressPick,
    Reminders,
    ContentPacks,
    Fonts { list: Vec<(String, String)>, filter: String, error: Option<String>, previews: HashMap<String, Option<TextureHandle>> },
    Help { query: String },
    Settings,
    RenameTab { index: usize, name: String },
    RenamePage { index: usize, name: String },
    ConfirmCloseTab(usize),
    ConfirmQuit,
    Recover(Vec<(u64, String, Project)>),
    Shapes,
    Borders(Box<crate::windows::BorderState>),
    Signature(Box<crate::windows::SignatureState>),
    Ai(Box<crate::windows::AiState>),
    PrintPro(Box<crate::windows::PrintState>),
}

impl Dialog {
    pub fn welcome() -> Self {
        Dialog::Welcome
    }
    pub fn new_project() -> Self {
        Dialog::NewProject { kind: Kind::CardQuarter, landscape: Kind::CardQuarter.default_landscape(), option: 0, sheets: 4, tall: false, design: 0, cal: Box::default() }
    }
    pub fn message(title: &str, body: &str) -> Self {
        Dialog::Message { title: title.into(), body: body.into() }
    }
    pub fn print() -> Self {
        Dialog::print_window(false)
    }
    pub fn gallery(tab: u8) -> Self {
        Dialog::Gallery(Box::new(GalleryState {
            tab,
            words: String::new(),
            pack: None,
            hits: Vec::new(),
            page: 0,
            thumbs: HashMap::new(),
            web_kind: 0,
            web_free: false,
            web_clear: false,
            clip_seen: None,
            clip_poll: -10.0,
            clip_pic: None,
            clip_url: None,
            in_browser: false,
            keep: false,
            mine: crate::library::list(),
            searched: false,
        }))
    }
    /// Google Images in the web browser, watching the clipboard for a
    /// copied picture (when PrintCraft's own Google window can't open).
    pub fn google_in_browser(words: &str) -> Self {
        let mut d = Dialog::gallery(1);
        if let Dialog::Gallery(g) = &mut d {
            g.words = words.trim().to_string();
            g.in_browser = true;
            g.clip_seen = clipboard_look().map(|c| c.0);
            crate::platform::open_url(&google_images_url(&g.words, 0, false, false));
        }
        d
    }
    pub fn page_setup(app: &App) -> Self {
        let d = &app.doc;
        let paper = d.paper().unwrap_or((8.5, 11.0));
        let option = match d.kind {
            Kind::Envelope => crate::kinds::ENVELOPES.iter().position(|(_, w, h)| (w.max(*h) - d.width.max(d.height)).abs() < 0.02 && (w.min(*h) - d.width.min(d.height)).abs() < 0.02).unwrap_or(0),
            _ => crate::kinds::sheet_presets(d.kind).iter().position(|p| Some(&p.layout) == d.sheet.as_ref()).unwrap_or(0),
        };
        Dialog::PageSetup(Box::new(PageSetupState {
            paper: crate::kinds::paper_index(paper).unwrap_or(crate::kinds::PAPERS.len()),
            custom: (paper.0.min(paper.1), paper.0.max(paper.1)),
            landscape: d.width > d.height,
            fit: true,
            option,
            sheets: d.banner.as_ref().map(|b| b.sheets).unwrap_or(4),
            tall: d.banner.as_ref().is_some_and(|b| b.tall),
        }))
    }
    pub fn headline(_edit: Option<u64>, app: &App) -> Self {
        if let Some(it) = app.first_selected()
            && let Content::Headline { text, look, flair, warp, second, outline, stretch, bend, bulge } = &it.content
        {
            return Dialog::Headline(Box::new(HeadlineState {
                edit: Some(it.id),
                text: text.clone(),
                look: look.clone(),
                flair: *flair,
                warp: *warp,
                second: *second,
                outline: *outline,
                stretch: *stretch,
                bend: *bend,
                bulge: *bulge,
                preview: None,
            }));
        }
        Dialog::Headline(Box::new(HeadlineState {
            edit: None,
            text: "Headline".into(),
            look: TextLook { color: Rgba::hex("#d1495b"), bold: true, align: Align::Center, ..Default::default() },
            flair: Flair::BlendDown,
            warp: Warp::Straight,
            second: Rgba::hex("#f2b631"),
            outline: Rgba::hex("#5a1a12"),
            stretch: false,
            bend: 1.0,
            bulge: 0.0,
            preview: None,
        }))
    }
    pub fn effects(act: &Act) -> Self {
        use crate::effects as fx;
        let which = match act {
            Act::Shadow => 0,
            Act::Glow => 1,
            _ => 2,
        };
        let (size, color) = match which {
            0 => (fx::SHADOW_START_SOFTNESS, Rgba(0, 0, 0, (fx::SHADOW_START_STRENGTH * 120.0) as u8)),
            1 => (fx::GLOW_START_SIZE, Rgba::hex("#f2b631")),
            _ => (0.0, Rgba::BLACK),
        };
        Dialog::Effects { which, amount: fx::SHADOW_START_DISTANCE, angle: fx::SHADOW_START_ANGLE, size, color, style: fx::FrameStyle::Thin as u8 }
    }
    pub fn photo(item: u64, asset: u64, tab: u8, _app: &App, _ctx: &egui::Context) -> Self {
        Dialog::Photo(Box::new(PhotoState::new(item, asset, tab)))
    }
    pub fn preview(app: &App, ctx: &egui::Context) -> Self {
        let sheets = crate::export::sheets(&app.doc);
        let pages = sheets
            .iter()
            .take(8)
            .enumerate()
            .filter_map(|(i, s)| {
                let pm = crate::export::render_sheet(&app.doc, s, &app.fonts, &app.pics, 60.0)?;
                let rgba = crate::raster::pixmap_to_rgba(&pm);
                let img = egui::ColorImage::from_rgba_unmultiplied([pm.width() as usize, pm.height() as usize], &rgba);
                Some(ctx.load_texture(format!("sheet{i}"), img, egui::TextureOptions::LINEAR))
            })
            .collect();
        Dialog::Preview { pages, at: 0 }
    }
    pub fn fonts(app: &mut App, ctx: &egui::Context) -> Self {
        app.online.list_fonts(ctx);
        Dialog::Fonts { list: Vec::new(), filter: String::new(), error: None, previews: HashMap::new() }
    }
}

fn c32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0, c.1, c.2, c.3)
}

pub fn color_row(ui: &mut egui::Ui, settings: &mut crate::settings::Settings, dropper: &mut Option<u8>, label: &str, c: &mut Rgba) -> bool {
    ui.horizontal(|ui| {
        let mut wants = false;
        let changed = crate::colors::button(ui, label, label, c, settings, &mut wants);
        if wants {
            // The picked color lands in "Recently used" for this chooser.
            *dropper = Some(2);
        }
        changed
    })
    .inner
}

fn big_choice(ui: &mut egui::Ui, title: &str, sub: &str, color: Color32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 72.0), egui::Sense::click());
    let v = ui.visuals();
    ui.painter().rect_filled(rect, 14.0, if resp.hovered() { color.gamma_multiply(0.18) } else { v.widgets.inactive.weak_bg_fill });
    ui.painter().rect_stroke(rect, 14.0, egui::Stroke::new(if resp.hovered() { 2.0 } else { 1.0 }, if resp.hovered() { color } else { v.widgets.inactive.bg_stroke.color }), egui::StrokeKind::Inside);
    // A little picture for each choice, in the style of the project pictures.
    let pic = egui::Rect::from_min_size(rect.min + egui::vec2(10.0, 8.0), egui::vec2(56.0, 56.0));
    ui.painter().rect_filled(pic, 12.0, color.gamma_multiply(0.2));
    crate::kind_icons::welcome(ui.painter(), pic.shrink(6.0), title, color);
    ui.painter().text(rect.min + egui::vec2(80.0, 15.0), egui::Align2::LEFT_TOP, title, egui::FontId::proportional(18.0), v.text_color());
    ui.painter().text(rect.min + egui::vec2(80.0, 40.0), egui::Align2::LEFT_TOP, sub, egui::FontId::proportional(13.5), v.weak_text_color());
    resp.clicked()
}

/// Show every open dialog; returns actions they ask for.
pub fn show_all(ctx: &egui::Context, app: &mut App) -> Vec<Act> {
    let mut acts = Vec::new();
    MIDDLE.with(|m| m.set(ctx.content_rect().center()));
    let mut dialogs = std::mem::take(&mut app.dialogs);
    drain_online(app, &mut dialogs, ctx);
    // Esc closes the front window (unless someone is typing in it); Enter
    // closes simple message windows. Popups close on Esc by themselves.
    let typing = ctx.egui_wants_keyboard_input();
    let popup_open = egui::Popup::is_any_open(ctx);
    if !typing && !popup_open && app.editing.is_none() {
        let (esc, enter) = ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.key_pressed(egui::Key::Enter)));
        let close_front = match dialogs.last() {
            Some(Dialog::Message { .. } | Dialog::Welcome) => esc || enter,
            Some(Dialog::Recover(_)) => false,
            Some(_) => esc,
            None => false,
        };
        if close_front {
            dialogs.pop();
        } else if esc && dialogs.is_empty() {
            app.selected.clear();
            app.eyedropper = None;
        }
    }
    let mut keep = Vec::new();
    for mut d in dialogs.drain(..) {
        if show(ctx, app, &mut d, &mut acts) {
            keep.push(d);
        }
    }
    keep.append(&mut app.dialogs);
    app.dialogs = keep;
    acts
}

fn drain_online(app: &mut App, dialogs: &mut [Dialog], ctx: &egui::Context) {
    while let Ok(msg) = app.online.rx.try_recv() {
        app.online.busy = false;
        match msg {
            Reply::WebPick { src, alt } => {
                let words = app.online.web_words.trim().to_string();
                let name = if !alt.trim().is_empty() { alt.trim().chars().take(60).collect() } else if !words.is_empty() { words.clone() } else { "Picture".to_string() };
                let search = if words.is_empty() { "Google Images".to_string() } else { format!("a Google Images search for “{words}”") };
                if let Some(b64) = src.strip_prefix("data:").and_then(|d| d.split_once(";base64,")).map(|(_, b)| b) {
                    use base64::Engine;
                    match base64::engine::general_purpose::STANDARD.decode(b64.trim()) {
                        Ok(bytes) => {
                            let credit = format!("From {search}");
                            if app.online.web_keep {
                                let _ = crate::library::save(&name, &bytes, &credit);
                            }
                            app.add_picture_bytes(&name, bytes, &credit, ctx);
                            app.notify("Added the picture to your page.", ctx);
                        }
                        Err(_) => app.notify("That picture couldn't be read. Please try another.", ctx),
                    }
                } else {
                    app.notify("Getting the picture…", ctx);
                    app.keep_web = app.online.web_keep;
                    let credit = format!("From {search} ({src})");
                    app.online.fetch_picture(WebPicture { title: name, thumb_url: src.clone(), full_url: src, credit }, ctx);
                }
            }
            Reply::Full(r) => match r {
                Ok((pic, bytes)) => {
                    if std::mem::take(&mut app.keep_web) {
                        let _ = crate::library::save(&pic.title, &bytes, &pic.credit);
                    }
                    app.add_picture_bytes(&pic.title, bytes, &pic.credit, ctx)
                }
                Err(e) => app.notify(e, ctx),
            },
            Reply::Fonts(r) => {
                for d in dialogs.iter_mut() {
                    if let Dialog::Fonts { list, error, .. } = d {
                        match &r {
                            Ok(l) => *list = l.clone(),
                            Err(e) => *error = Some(e.clone()),
                        }
                    }
                }
            }
            Reply::FontPreview(id, bytes) => {
                let ink = Color32::WHITE; // tinted with the theme's text color when drawn
                let tex = crate::raster::text_image(&bytes, "Preview", 24.0, [ink.r(), ink.g(), ink.b()])
                    .map(|(w, h, rgba)| ctx.load_texture(format!("webfont-{id}"), egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba), egui::TextureOptions::LINEAR));
                for d in dialogs.iter_mut() {
                    if let Dialog::Fonts { previews, .. } = d {
                        previews.insert(id.clone(), tex.clone());
                    }
                }
            }
            Reply::FontSaved(r) => {
                match r {
                    Ok(f) => app.notify(format!("{f} is ready to use."), ctx),
                    Err(e) => app.notify(e, ctx),
                }
                app.fonts.rescan();
            }
        }
    }
}

thread_local! {
    static MIDDLE: std::cell::Cell<egui::Pos2> = const { std::cell::Cell::new(egui::pos2(640.0, 400.0)) };
}

/// Pop-up windows open near the middle and can be dragged anywhere by
/// their title, so they never have to cover what you're changing.
pub fn window(title: &str) -> egui::Window<'_> {
    egui::Window::new(title).collapsible(true).resizable(true).pivot(egui::Align2::CENTER_CENTER).default_pos(MIDDLE.with(|m| m.get())).constrain(true)
}

/// Returns false when the dialog closes.
fn show(ctx: &egui::Context, app: &mut App, d: &mut Dialog, acts: &mut Vec<Act>) -> bool {
    if let Some(keep) = crate::windows::show(ctx, app, d, acts) {
        return keep;
    }
    let mut open = true;
    let mut close = false;
    match d {
        Dialog::Welcome => {
            window("Welcome to PrintCraft").open(&mut open).resizable(false).fixed_size([460.0, 320.0]).show(ctx, |ui| {
                ui.label(RichText::new("What would you like to do?").size(18.0));
                ui.add_space(8.0);
                if big_choice(ui, "Start a New Project", "Cards, signs, banners, labels and more", Color32::from_rgb(235, 106, 92)) {
                    acts.push(Act::NewProject);
                    close = true;
                }
                ui.add_space(6.0);
                if big_choice(ui, "Open a Saved Project", "Your PrintCraft designs and older projects", Color32::from_rgb(47, 143, 216)) {
                    acts.push(Act::Open);
                    close = true;
                }
                ui.add_space(6.0);
                if big_choice(ui, "Just Start Designing", "A blank sign you can change later", Color32::from_rgb(76, 182, 140)) {
                    close = true;
                }
            });
        }
        Dialog::NewProject { kind, landscape, option, sheets, tall, design, cal } => {
            let mut create = false;
            window("New Project").open(&mut open).default_size([900.0, 760.0]).show(ctx, |ui| {
              egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(RichText::new("What would you like to make?").size(17.0));
                ui.add_space(6.0);
                egui::Grid::new("kinds").spacing([8.0, 8.0]).show(ui, |ui| {
                    for (i, k) in Kind::ALL.iter().enumerate() {
                        let [r, g, b] = k.tile_color();
                        let on = *kind == *k;
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(134.0, 96.0), egui::Sense::click());
                        let v = ui.visuals();
                        ui.painter().rect_filled(rect, 10.0, if on { v.selection.bg_fill.gamma_multiply(0.35) } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
                        if on {
                            ui.painter().rect_stroke(rect, 10.0, egui::Stroke::new(2.0, v.selection.bg_fill), egui::StrokeKind::Inside);
                        }
                        let _ = (r, g, b);
                        let pic = egui::Rect::from_min_max(rect.min + egui::vec2(8.0, 6.0), egui::pos2(rect.right() - 8.0, rect.bottom() - 28.0));
                        crate::kind_icons::paint(ui.painter(), pic, *k);
                        let label = egui::WidgetText::from(RichText::new(crate::kind_icons::short(*k)).size(12.5)).into_galley(ui, Some(egui::TextWrapMode::Truncate), rect.width() - 10.0, egui::TextStyle::Body);
                        ui.painter().galley(rect.center_bottom() - egui::vec2(label.size().x / 2.0, 22.0), label, v.text_color());
                        if resp.clicked() {
                            *kind = *k;
                            *landscape = k.default_landscape();
                            *design = 0;
                            // Paper kinds start on their usual paper; others on their first size.
                            *option = Project::new(*k, *landscape).paper().and_then(crate::kinds::paper_index).unwrap_or(0);
                        }
                        if resp.double_clicked() {
                            create = true;
                        }
                        if i % 6 == 5 {
                            ui.end_row();
                        }
                    }
                });
                ui.add_space(8.0);
                // Size choices for kinds that come in standard sizes.
                let presets = crate::kinds::sheet_presets(*kind);
                if kind.paper_use().is_some() {
                    let papers = crate::kinds::PAPERS;
                    *option = (*option).min(papers.len() - 1);
                    egui::ComboBox::from_label("Paper").width(260.0).selected_text(papers[*option].0).show_ui(ui, |ui| {
                        for (i, p) in papers.iter().enumerate() {
                            ui.selectable_value(option, i, p.0);
                        }
                    });
                } else if !presets.is_empty() {
                    *option = (*option).min(presets.len() - 1);
                    egui::ComboBox::from_label("Sheet").width(380.0).selected_text(presets[*option].label).show_ui(ui, |ui| {
                        for (i, p) in presets.iter().enumerate() {
                            ui.selectable_value(option, i, p.label);
                        }
                    });
                } else if *kind == Kind::Envelope {
                    *option = (*option).min(crate::kinds::ENVELOPES.len() - 1);
                    egui::ComboBox::from_label("Envelope size").width(300.0).selected_text(crate::kinds::ENVELOPES[*option].0).show_ui(ui, |ui| {
                        for (i, e) in crate::kinds::ENVELOPES.iter().enumerate() {
                            ui.selectable_value(option, i, e.0);
                        }
                    });
                } else if *kind == Kind::Banner {
                    crate::widgets::stepper(ui, sheets, 1, 20, "Sheets of paper:");
                    ui.horizontal(|ui| {
                        ui.label("Each sheet:");
                        ui.radio_value(tall, false, "Sideways (banner 8½ in tall)");
                        ui.radio_value(tall, true, "Upright (banner 11 in tall)");
                    });
                    let b = crate::model::BannerSetup { sheets: *sheets, tall: *tall };
                    let (seg, h) = Project::banner_segment(&b, crate::model::BANNER_MARGIN);
                    ui.label(RichText::new(format!("Banner {:.1} × {:.1} in. Sheets overlap a little so nothing is cut off; trim and tape along the dashed lines.", seg * *sheets as f32, h)).weak().size(12.0));
                }
                ui.horizontal(|ui| {
                    ui.label("Orientation:");
                    ui.radio_value(landscape, false, "Tall");
                    ui.radio_value(landscape, true, "Wide");
                    let (w, h) = if kind.paper_use().is_some() {
                        let p = crate::kinds::PAPERS[(*option).min(crate::kinds::PAPERS.len() - 1)];
                        crate::kinds::page_for_paper(*kind, (p.1, p.2), *landscape)
                    } else {
                        let (mut w, mut h) = kind.size();
                        if *landscape != kind.default_landscape() {
                            std::mem::swap(&mut w, &mut h);
                        }
                        (w, h)
                    };
                    ui.label(RichText::new(format!("{} × {} in", (w * 100.0).round() / 100.0, (h * 100.0).round() / 100.0)).weak());
                });
                ui.add_space(6.0);
                if *kind == Kind::Calendar {
                    ui.label(RichText::new("Your calendar").strong());
                    calendar_options(ui, cal, *landscape);
                } else {
                    // Ready-made designs for this kind, or a blank page.
                    let designs = crate::templates::for_kind(*kind);
                    if !designs.is_empty() {
                        ui.label(RichText::new("Start with").strong());
                        egui::ScrollArea::horizontal().id_salt("designs").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if design_tile(ui, app, ctx, *design == 0, "Blank", None) {
                                    *design = 0;
                                }
                                for (i, d) in designs.iter().enumerate() {
                                    if design_tile(ui, app, ctx, *design == i + 1, d.name, Some(d)) {
                                        *design = i + 1;
                                    }
                                }
                            });
                        });
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Create").size(16.0)).min_size(egui::vec2(120.0, 32.0))).clicked() {
                        create = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
              });
            });
            if create {
                let mut p = Project::new(*kind, *landscape);
                let presets = crate::kinds::sheet_presets(*kind);
                if kind.paper_use().is_some() {
                    let paper = crate::kinds::PAPERS[(*option).min(crate::kinds::PAPERS.len() - 1)];
                    (p.width, p.height) = crate::kinds::page_for_paper(*kind, (paper.1, paper.2), *landscape);
                } else if let Some(pr) = presets.get(*option) {
                    p.width = pr.w;
                    p.height = pr.h;
                    p.sheet = Some(pr.layout.clone());
                } else if *kind == Kind::Envelope {
                    let (_, w, h) = crate::kinds::ENVELOPES[(*option).min(crate::kinds::ENVELOPES.len() - 1)];
                    (p.width, p.height) = if *landscape { (w, h) } else { (h, w) };
                } else if *kind == Kind::Banner {
                    p.set_banner(crate::model::BannerSetup { sheets: *sheets, tall: *tall });
                }
                if *kind == Kind::Calendar {
                    let people = crate::calendar::birthdays(&app.settings.contacts);
                    crate::calendar::fill(&mut p, cal, &people);
                } else if *design > 0
                    && let Some(d) = crate::templates::for_kind(*kind).get(*design - 1)
                {
                    // The design, sized to the paper and choices made here.
                    let mut q = (d.build)();
                    q.resize_pages(p.width, p.height, true);
                    q.landscape = p.landscape;
                    q.sheet = p.sheet.clone();
                    q.banner = p.banner.clone();
                    p = q;
                }
                app.open_tab(p, None, false);
                close = true;
            }
        }
        Dialog::CalendarMaker(cal) => {
            let mut make = false;
            let landscape = app.doc.width > app.doc.height;
            window("Calendar Maker").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label("Fills this calendar with months, one page each. It replaces what's on the pages now (Undo brings it back).");
                ui.add_space(6.0);
                calendar_options(ui, cal, landscape);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    make = ui.add(egui::Button::new(RichText::new("Make the Calendar").size(15.0)).min_size(egui::vec2(170.0, 30.0))).clicked();
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if make {
                app.checkpoint();
                let people = crate::calendar::birthdays(&app.settings.contacts);
                crate::calendar::fill(&mut app.doc, cal, &people);
                app.panel = 0;
                app.selected.clear();
                app.fit_pending = true;
                close = true;
            }
        }
        Dialog::Ornaments => {
            let mut pick: Option<u8> = None;
            window("Flourishes & Dividers").open(&mut open).default_size([560.0, 520.0]).show(ctx, |ui| {
                ui.label("Click one to add it to the page. Change its colors with the Line and Fill buttons, and its thickness with Line.");
                ui.add_space(6.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("ornaments").spacing([8.0, 8.0]).show(ui, |ui| {
                        for (i, name) in crate::raster::ORNAMENT_NAMES.iter().enumerate() {
                            let key = format!("ornament-{i}");
                            if !app.design_thumbs.contains_key(&key) {
                                let mut it = Item::new(0, Frame::new(0.0, 0.0, 3.0, 0.7), Content::Ornament { style: i as u8 });
                                it.stroke = Rgba::hex("#333333");
                                it.fill = Rgba::hex("#c9a227");
                                it.stroke_pt = 2.0;
                                if let Some(r) = crate::raster::render_item(&it, &app.doc, &app.fonts, &app.pics, 50.0) {
                                    let rgba = crate::raster::pixmap_to_rgba(&r.pixmap);
                                    let tex = ctx.load_texture(&key, egui::ColorImage::from_rgba_unmultiplied([r.pixmap.width() as usize, r.pixmap.height() as usize], &rgba), egui::TextureOptions::LINEAR);
                                    app.design_thumbs.insert(key.clone(), tex);
                                }
                            }
                            let (r, resp) = ui.allocate_exact_size(egui::vec2(170.0, 74.0), egui::Sense::click());
                            let v = ui.visuals();
                            ui.painter().rect_filled(r, 8.0, if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { Color32::WHITE });
                            if let Some(t) = app.design_thumbs.get(&key) {
                                let s = t.size_vec2();
                                let k = ((r.width() - 12.0) / s.x).min((r.height() - 26.0) / s.y);
                                ui.painter().image(t.id(), egui::Rect::from_center_size(r.center() - egui::vec2(0.0, 8.0), s * k), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                            }
                            ui.painter().text(r.center_bottom() - egui::vec2(0.0, 4.0), egui::Align2::CENTER_BOTTOM, *name, egui::FontId::proportional(12.0), Color32::from_gray(60));
                            if resp.clicked() {
                                pick = Some(i as u8);
                            }
                            if i % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
                });
            });
            if let Some(style) = pick {
                let w = (app.doc.width * 0.6).clamp(1.5, 6.0);
                let h = if style == 11 { w * 0.22 } else { w * 0.16 };
                let (stroke, fill) = (app.line_color, app.fill_color);
                let id = app.add_item(w, h, Content::Ornament { style });
                if let Some(it) = app.item_mut(id) {
                    it.stroke = stroke;
                    it.fill = fill;
                    it.stroke_pt = 2.0;
                }
                close = true;
            }
        }
        Dialog::ReplacePicture => {
            let mut chosen = false;
            window("Replace Picture").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label("Where is the new picture? It goes in the same place and keeps any shadow, frame or other effects.");
                ui.add_space(6.0);
                let big = |ui: &mut egui::Ui, t: &str| ui.add(egui::Button::new(RichText::new(t).size(15.0)).min_size(egui::vec2(300.0, 34.0))).clicked();
                if big(ui, "📁  A picture on this computer") {
                    crate::platform::pick_file(crate::platform::Want::Picture, "Pictures", &["png", "jpg", "jpeg", "bmp", "gif", "wmf"], ctx);
                    chosen = true;
                }
                if big(ui, "🔍  Google Images") {
                    acts.push(Act::Gallery(1));
                    chosen = true;
                }
                if big(ui, "⭐  My Art") {
                    acts.push(Act::MyArt);
                    chosen = true;
                }
                ui.label(RichText::new("You can also paste a picture now (Ctrl+V).").weak().size(12.0));
                if ui.button("Cancel").clicked() {
                    app.replace_target = None;
                    close = true;
                }
            });
            if chosen {
                close = true;
            }
            if !open {
                // Closed with ×: only call it off if nothing was picked.
                if !chosen {
                    app.replace_target = None;
                }
            }
        }
        Dialog::Message { title, body } => {
            window(title).open(&mut open).resizable(false).default_width(420.0).show(ctx, |ui| {
                ui.label(body.as_str());
                ui.add_space(8.0);
                if ui.button("OK").clicked() {
                    close = true;
                }
            });
        }
        Dialog::Headline(h) => {
            let mut done = false;
            window("Create a Headline").open(&mut open).default_size([620.0, 520.0]).show(ctx, |ui| {
                ui.label("Enter your headline text:");
                ui.add(egui::TextEdit::multiline(&mut h.text).desired_rows(2).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    egui::ComboBox::from_label("Font").selected_text(h.look.family.clone()).width(200.0).show_ui(ui, |ui| {
                        for f in app.fonts.families() {
                            if ui.selectable_label(f == h.look.family, &f).clicked() {
                                h.look.family = f;
                            }
                        }
                    });
                    ui.checkbox(&mut h.look.bold, "Bold");
                    ui.checkbox(&mut h.look.italic, "Italic");
                });
                ui.horizontal(|ui| {
                    color_row(ui, &mut app.settings, &mut app.eyedropper, "Color", &mut h.look.color);
                    color_row(ui, &mut app.settings, &mut app.eyedropper, "Second color", &mut h.second);
                    color_row(ui, &mut app.settings, &mut app.eyedropper, "Edge / shadow", &mut h.outline);
                });
                ui.horizontal(|ui| {
                    ui.label("Shape:");
                    for w in Warp::ALL {
                        ui.selectable_value(&mut h.warp, w, w.label());
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(h.warp != Warp::Straight, |ui| crate::widgets::slider(ui, &mut h.bend, -4.0..=4.0, |s| s.text("How much it bends")));
                    crate::widgets::slider(ui, &mut h.bulge, -2.0..=2.0, |s| s.text("Bulge (mixes with any shape)"));
                    ui.checkbox(&mut h.stretch, "Stretch letters to fill the box");
                });
                ui.label("Effect:");
                egui::Grid::new("flairs").spacing([4.0, 4.0]).show(ui, |ui| {
                    for (i, f) in Flair::ALL.iter().enumerate() {
                        if ui.add(egui::Button::selectable(h.flair == *f, f.label()).min_size(egui::vec2(132.0, 24.0))).clicked() {
                            h.flair = *f;
                        }
                        if i % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
                ui.add_space(6.0);
                // Live preview through the same renderer as the page.
                let item = Item::new(0, Frame::new(0.0, 0.0, 5.0, 1.4), Content::Headline { text: h.text.clone(), look: h.look.clone(), flair: h.flair, warp: h.warp, second: h.second, outline: h.outline, stretch: h.stretch, bend: h.bend, bulge: h.bulge });
                let key = {
                    use std::hash::{Hash, Hasher};
                    let mut s = std::collections::hash_map::DefaultHasher::new();
                    serde_json::to_string(&item).unwrap_or_default().hash(&mut s);
                    s.finish()
                };
                if h.preview.as_ref().is_none_or(|(k, _)| *k != key)
                    && let Some(r) = crate::raster::render_item(&item, &app.doc, &app.fonts, &app.pics, 100.0)
                {
                    let rgba = crate::raster::pixmap_to_rgba(&r.pixmap);
                    let img = egui::ColorImage::from_rgba_unmultiplied([r.pixmap.width() as usize, r.pixmap.height() as usize], &rgba);
                    h.preview = Some((key, ctx.load_texture("headline-preview", img, egui::TextureOptions::LINEAR)));
                }
                if let Some((_, t)) = &h.preview {
                    egui::Frame::new().fill(Color32::WHITE).corner_radius(8.0).inner_margin(6.0).show(ui, |ui| {
                        ui.add(egui::Image::new(t).max_width(ui.available_width()));
                    });
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("OK").min_size(egui::vec2(90.0, 28.0))).clicked() {
                        done = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if done {
                let content = Content::Headline { text: h.text.clone(), look: h.look.clone(), flair: h.flair, warp: h.warp, second: h.second, outline: h.outline, stretch: h.stretch, bend: h.bend, bulge: h.bulge };
                if let Some(id) = h.edit {
                    app.checkpoint();
                    if let Some(it) = app.item_mut(id) {
                        it.content = content;
                    }
                } else {
                    let w = (app.doc.width * 0.8).min(7.0);
                    app.add_item(w, w * 0.25, content);
                }
                close = true;
            }
        }
        Dialog::Gallery(g) => gallery(ctx, app, g, &mut open, &mut close),
        Dialog::Photo(p) => crate::photo_editor::show(ctx, app, p, &mut open, &mut close),
        Dialog::PageSetup(ps) => page_setup(ctx, app, ps, &mut open, &mut close),
        Dialog::Update(u) => crate::update::window(ctx, app, u, &mut open, &mut close),
        Dialog::Report(r) => crate::report::window(ctx, app, r, &mut open, &mut close),
        Dialog::Effects { which, amount, angle, size, color, style } => {
            let title = ["Apply Drop Shadow", "Apply Glow", "Apply Frame"][*which as usize];
            let mut apply = None;
            window(title).open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label(RichText::new("Drag this window by its title to see your item while you change it.").weak().size(12.0));
                let mut live = false;
                match which {
                    0 => {
                        live |= crate::widgets::slider(ui, amount, 0.0..=1.0, |s| s.text("Distance").show_value(false)).changed();
                        live |= crate::widgets::slider(ui, angle, 0.0..=360.0, |s| s.text("Direction").suffix("°")).changed();
                        live |= crate::widgets::slider(ui, size, 0.0..=1.0, |s| s.text("Softness").show_value(false)).changed();
                        live |= color_row(ui, &mut app.settings, &mut app.eyedropper, "Shadow color", color);
                        let mut a = color.3 as f32 / 255.0;
                        if crate::widgets::slider(ui, &mut a, 0.1..=1.0, |s| s.text("Darkness")).changed() {
                            color.3 = (a * 255.0) as u8;
                            live = true;
                        }
                    }
                    1 => {
                        live |= crate::widgets::slider(ui, size, 0.0..=1.0, |s| s.text("Size").show_value(false)).changed();
                        live |= color_row(ui, &mut app.settings, &mut app.eyedropper, "Glow color", color);
                    }
                    _ => {
                        live |= color_row(ui, &mut app.settings, &mut app.eyedropper, "Frame color", color);
                        egui::Grid::new("frames").spacing([6.0, 6.0]).show(ui, |ui| {
                            for (k, f) in crate::effects::FrameStyle::ALL.iter().enumerate() {
                                if ui.add(egui::Button::selectable(*style == *f as u8, f.label()).min_size(egui::vec2(120.0, 28.0))).clicked() {
                                    *style = *f as u8;
                                    live = true;
                                }
                                if k % 3 == 2 {
                                    ui.end_row();
                                }
                            }
                        });
                    }
                }
                if live {
                    apply = Some(true);
                }
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        apply = Some(true);
                    }
                    if ui.button("Remove").clicked() {
                        apply = Some(false);
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            });
            if let Some(on) = apply {
                app.checkpoint();
                let ids = app.selected.clone();
                let (c, amount, angle, size, style) = (*color, *amount, *angle, *size, *style);
                for it in app.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id)) {
                    match which {
                        0 => it.shadow = on.then_some(Shadow { distance: amount, angle, softness: size, ..Shadow::new(c) }),
                        1 => {
                            it.glow = on.then_some(c.with_alpha(220));
                            it.glow_size = size;
                        }
                        _ => {
                            it.frame_style = if on { style } else { 0 };
                            if on {
                                it.stroke = c.with_alpha(255);
                            }
                        }
                    }
                }
            }
        }
        Dialog::TextShape => {
            let mut pick: Option<Option<Outline>> = None;
            window("Choose a Shape").open(&mut open).resizable(false).show(ctx, |ui| {
                let is_text = matches!(app.first_selected().map(|i| &i.content), Some(Content::Text { .. }));
                ui.label(if is_text { "Shape for the selected text box:" } else { "Pick a shape to draw:" });
                egui::Grid::new("shapes").show(ui, |ui| {
                    for (i, o) in Outline::ALL.iter().enumerate() {
                        if ui.add(egui::Button::new(o.label()).min_size(egui::vec2(120.0, 30.0))).clicked() {
                            pick = Some(Some(*o));
                        }
                        if i % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
                if is_text && ui.button("No shape").clicked() {
                    pick = Some(None);
                }
            });
            if let Some(o) = pick {
                if matches!(app.first_selected().map(|i| &i.content), Some(Content::Text { .. })) {
                    app.checkpoint();
                    let ids = app.selected.clone();
                    for it in app.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id)) {
                        if let Content::Text { shape, .. } = &mut it.content {
                            *shape = o;
                            if o.is_some() && it.fill.is_clear() {
                                it.fill = Rgba::WHITE;
                            }
                        }
                    }
                } else if let Some(o) = o {
                    acts.push(Act::Shape(o));
                }
                close = true;
            }
        }
        Dialog::Backdrop => {
            window("Backdrop / Page Color").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label(RichText::new("Pattern").strong());
                let current = app.page().pattern;
                let mut pick_pattern = None;
                egui::Grid::new("patterns").spacing([6.0, 6.0]).show(ui, |ui| {
                    for (k, name) in crate::raster::PATTERN_NAMES.iter().enumerate() {
                        let key = egui::Id::new(("pattern-prev", k, app.page().pattern_color.0, app.page().pattern_color.1, app.page().pattern_color.2, app.page().background.0));
                        let tex: Option<TextureHandle> = ui.ctx().data(|d| d.get_temp(key)).or_else(|| {
                            let mut probe = app.page().clone();
                            probe.pattern = k as u8;
                            let pm = crate::raster::background_image(&probe, 1.0, 0.75, 96.0)?;
                            let rgba = crate::raster::pixmap_to_rgba(&pm);
                            let t = ctx.load_texture(format!("pat{k}"), egui::ColorImage::from_rgba_unmultiplied([pm.width() as usize, pm.height() as usize], &rgba), egui::TextureOptions::LINEAR);
                            ui.ctx().data_mut(|d| d.insert_temp(key, t.clone()));
                            Some(t)
                        });
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(96.0, 92.0), egui::Sense::click());
                        let v = ui.visuals();
                        ui.painter().rect_filled(r, 8.0, if current as usize == k { v.selection.bg_fill.gamma_multiply(0.4) } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
                        if let Some(t) = tex {
                            let img = egui::Rect::from_center_size(r.center_top() + egui::vec2(0.0, 36.0), egui::vec2(80.0, 60.0));
                            ui.painter().image(t.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                            ui.painter().rect_stroke(img, 2.0, egui::Stroke::new(1.0, Color32::from_black_alpha(60)), egui::StrokeKind::Outside);
                        }
                        ui.painter().text(r.center_bottom() - egui::vec2(0.0, 4.0), egui::Align2::CENTER_BOTTOM, *name, egui::FontId::proportional(11.5), v.text_color());
                        if resp.clicked() {
                            pick_pattern = Some(k as u8);
                        }
                        if k % 6 == 5 {
                            ui.end_row();
                        }
                    }
                });
                if let Some(k) = pick_pattern {
                    app.checkpoint();
                    app.page_mut().pattern = k;
                }
                let mut pc = app.page().pattern_color;
                if color_row(ui, &mut app.settings, &mut app.eyedropper, "Pattern color", &mut pc) {
                    app.checkpoint();
                    app.page_mut().pattern_color = pc;
                }
                // Finer or bigger stripes, dots, grids…
                let mut sc = app.page().pattern_scale;
                let r = crate::widgets::slider(ui, &mut sc, 0.25..=4.0, |s| s.text("Pattern size").logarithmic(true).suffix("×").fixed_decimals(2));
                if r.drag_started() || (r.changed() && !r.dragged()) {
                    app.checkpoint();
                }
                if r.changed() {
                    app.page_mut().pattern_scale = sc;
                }
                if ui.button("Use on every page").clicked() {
                    app.checkpoint();
                    let (bg, pat, col, scale) = (app.page().background, app.page().pattern, app.page().pattern_color, app.page().pattern_scale);
                    for p in app.doc.panels.iter_mut() {
                        p.background = bg;
                        p.pattern = pat;
                        p.pattern_color = col;
                        p.pattern_scale = scale;
                    }
                }
                ui.separator();
                let mut c = app.page().background;
                if color_row(ui, &mut app.settings, &mut app.eyedropper, "Page color", &mut c) {
                    app.checkpoint();
                    app.page_mut().background = c;
                }
                egui::Grid::new("bg").show(ui, |ui| {
                    for (i, hex) in ["#ffffff", "#fffaf0", "#fdf0d5", "#fde2e4", "#e2ece9", "#cddafd", "#f1faee", "#fff3b0", "#e9edc9", "#d8e2dc", "#2b2d42", "#14213d"].iter().enumerate() {
                        let c = Rgba::hex(hex);
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(34.0, 26.0), egui::Sense::click());
                        ui.painter().rect_filled(r, 5.0, c32(c));
                        ui.painter().rect_stroke(r, 5.0, egui::Stroke::new(1.0, Color32::from_black_alpha(50)), egui::StrokeKind::Inside);
                        if resp.clicked() {
                            app.checkpoint();
                            app.page_mut().background = c;
                        }
                        if i % 6 == 5 {
                            ui.end_row();
                        }
                    }
                });
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        }
        Dialog::Print { copies } => {
            let mut go = false;
            window("Print").open(&mut open).resizable(false).show(ctx, |ui| {
                let sheets = crate::export::sheets(&app.doc);
                ui.label(format!("{} — {} sheet{} of paper", app.doc.kind.label(), sheets.len(), if sheets.len() == 1 { "" } else { "s" }));
                if matches!(app.doc.kind, Kind::CardHalf | Kind::NoteCard) {
                    ui.label(RichText::new("After the first sheet prints, put it back in the printer blank side up for the inside.").weak());
                }
                crate::widgets::slider(ui, copies, 1..=50, |s| s.text("Copies"));
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new("Print").min_size(egui::vec2(90.0, 28.0))).clicked() {
                        go = true;
                    }
                    if ui.button("Print Preview").clicked() {
                        acts.push(Act::PrintPreview);
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if go {
                match crate::export::print_with(&app.doc, &app.fonts, &app.pics, &crate::export::PrintOptions { copies: *copies, ..Default::default() }) {
                    Ok(m) => app.notify(m, ctx),
                    Err(e) => app.dialogs.push(Dialog::message("Printing didn't work", &e)),
                }
                close = true;
            }
        }
        Dialog::Preview { pages, at } => {
            window("Print Preview").open(&mut open).default_size([560.0, 640.0]).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(*at > 0, egui::Button::new("◀ Previous")).clicked() {
                        *at -= 1;
                    }
                    ui.label(format!("Sheet {} of {}", *at + 1, pages.len()));
                    if ui.add_enabled(*at + 1 < pages.len(), egui::Button::new("Next ▶")).clicked() {
                        *at += 1;
                    }
                    if ui.button("Print...").clicked() {
                        acts.push(Act::Print);
                        close = true;
                    }
                });
                if let Some(t) = pages.get(*at) {
                    egui::Frame::new().fill(Color32::from_gray(200)).inner_margin(10.0).show(ui, |ui| {
                        ui.add(egui::Image::new(t).max_size(ui.available_size()).shrink_to_fit());
                    });
                }
            });
        }
        Dialog::AddressBook { editing } => {
            window("Address Book").open(&mut open).default_size([560.0, 420.0]).show(ctx, |ui| {
                let mut remove = None;
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    for (i, c) in app.settings.contacts.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.selectable_label(*editing == Some(i), if c.name.is_empty() { "(no name)" } else { &c.name }).clicked() {
                                *editing = Some(i);
                            }
                            ui.label(RichText::new(c.city.clone()).weak());
                            if ui.small_button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                });
                if let Some(i) = remove {
                    app.settings.contacts.remove(i);
                    *editing = None;
                    app.settings.save();
                }
                if ui.button("Add a person").clicked() {
                    app.settings.contacts.push(Contact::default());
                    *editing = Some(app.settings.contacts.len() - 1);
                }
                if let Some(i) = *editing
                    && let Some(c) = app.settings.contacts.get_mut(i)
                {
                    ui.separator();
                    let mut changed = false;
                    egui::Grid::new("contact").num_columns(2).show(ui, |ui| {
                        for (l, v) in [("Name", &mut c.name), ("Street", &mut c.street), ("City", &mut c.city), ("State", &mut c.region), ("ZIP", &mut c.postcode), ("Phone", &mut c.phone), ("Email", &mut c.email), ("Birthday (MM-DD)", &mut c.birthday)] {
                            ui.label(l);
                            changed |= ui.text_edit_singleline(v).changed();
                            ui.end_row();
                        }
                    });
                    if changed {
                        app.settings.save();
                    }
                }
            });
        }
        Dialog::AddressPick => {
            let mut pick = None;
            let mut field: Option<&str> = None;
            window("Insert an Address").open(&mut open).resizable(false).default_width(420.0).show(ctx, |ui| {
                ui.label(RichText::new("For everyone in the address book").strong());
                ui.label(RichText::new("Put in a merge field. When you print, choose the people and PrintCraft makes one for each: a label each, or an envelope each.").weak().size(12.0));
                ui.horizontal_wrapped(|ui| {
                    for (f, tip) in crate::merge::FIELDS {
                        if ui.button(f).on_hover_text(tip).clicked() {
                            field = Some(f);
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("One person's address").strong());
                if app.settings.contacts.is_empty() {
                    ui.label(RichText::new("Your address book is empty.").weak());
                    if ui.button("Open the Address Book").clicked() {
                        acts.push(Act::AddressBook);
                    }
                }
                for (i, c) in app.settings.contacts.iter().enumerate() {
                    if ui.button(if c.name.is_empty() { "(no name)" } else { &c.name }).clicked() {
                        pick = Some(i);
                    }
                }
            });
            let text = pick.map(|i| app.settings.contacts[i].mailing_block()).or(field.map(str::to_string));
            if let Some(text) = text {
                let look = TextLook { size: 14.0, ..Default::default() };
                let editing = app.editing.is_some();
                if editing {
                    // Typing in a text box: the field goes where the words are.
                    if let Some((_, buf)) = app.editing.as_mut() {
                        buf.push_str(&text);
                    }
                } else {
                    app.add_item(3.2, 1.0, Content::Text { text, look, shape: None, fit: false, runs: Vec::new() });
                }
                close = true;
            }
        }
        Dialog::Reminders => {
            window("Event Reminder").open(&mut open).resizable(false).show(ctx, |ui| {
                let list: Vec<&Contact> = app.settings.contacts.iter().filter(|c| !c.birthday.is_empty()).collect();
                if list.is_empty() {
                    ui.label("Add birthdays in the Address Book to see them here.");
                }
                for c in list {
                    ui.label(format!("🎂 {} — {}", c.name, c.birthday));
                }
            });
        }
        Dialog::ContentPacks => {
            window("My Content Packs").open(&mut open).default_size([560.0, 400.0]).show(ctx, |ui| {
                ui.label("PrintCraft can use .CDT art and photo collections you already own. Add the folder that holds them. If the big .CDT files and their small index files (.ini, .IDT) are in different folders, add both, or add a folder that contains both. Collections without index files still work; their pictures are just numbered instead of named.");
                ui.add_space(6.0);
                let mut remove = None;
                for (i, f) in app.settings.content_folders.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(f.display().to_string());
                        if ui.small_button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                let mut rescan = false;
                if let Some(i) = remove {
                    app.settings.content_folders.remove(i);
                    rescan = true;
                }
                if ui.button("Add a Folder...").clicked()
                    && let Some(f) = crate::platform::pick_folder()
                {
                    app.settings.content_folders.push(f);
                    rescan = true;
                }
                if rescan {
                    app.settings.save();
                    app.rescan_library();
                }
                ui.separator();
                if let Some(lib) = &app.library {
                    let ready = lib.packs.iter().filter(|p| p.data.is_some()).count();
                    ui.label(format!("{} packs found, {} ready to use, {} pictures.", lib.packs.len(), ready, lib.items.len()));
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for p in &lib.packs {
                            let state = if p.data.is_some() { "ready" } else { "picture data not found" };
                            ui.label(format!("{} — {} items ({state})", p.name, p.items));
                        }
                    });
                }
            });
        }
        Dialog::Fonts { list, filter, error, previews } => {
            let mut install = None;
            window("Online Fonts").open(&mut open).default_size([460.0, 480.0]).show(ctx, |ui| {
                ui.label("Free fonts with open licenses. Click one to add it to PrintCraft.");
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(filter);
                });
                if let Some(e) = error {
                    ui.colored_label(Color32::from_rgb(200, 60, 60), e.as_str());
                }
                if list.is_empty() && error.is_none() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Getting the font list…");
                    });
                }
                let f = filter.to_lowercase();
                let waiting = previews.values().filter(|v| v.is_none()).count();
                let mut ask = Vec::new();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (id, fam) in list.iter().filter(|(_, n)| f.is_empty() || n.to_lowercase().contains(&f)).take(300) {
                        let (r, row) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::click());
                        let v = ui.visuals();
                        if row.hovered() {
                            ui.painter().rect_filled(r, 6.0, v.widgets.hovered.weak_bg_fill);
                        }
                        ui.painter().text(r.left_center() + egui::vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, fam, egui::FontId::proportional(14.0), v.text_color());
                        match previews.get(id) {
                            Some(Some(t)) => {
                                let s = t.size_vec2();
                                let k = 24.0 / s.y.max(1.0);
                                let img = egui::Rect::from_min_size(egui::pos2(r.right() - s.x * k - 90.0, r.center().y - 12.0), s * k);
                                ui.painter().image(t.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), ui.visuals().text_color());
                            }
                            Some(None) => {}
                            None => {
                                if ui.is_rect_visible(r) && waiting + ask.len() < 6 {
                                    ask.push(id.clone());
                                }
                            }
                        }
                        ui.painter().text(r.right_center() - egui::vec2(10.0, 0.0), egui::Align2::RIGHT_CENTER, "+ Add", egui::FontId::proportional(13.0), v.hyperlink_color);
                        if row.on_hover_text("Click to add this font to PrintCraft").clicked() {
                            install = Some((id.clone(), fam.clone()));
                        }
                    }
                });
                for id in ask {
                    previews.insert(id.clone(), None);
                    app.online.font_preview(id, ctx);
                }
            });
            if let Some((id, fam)) = install {
                app.notify(format!("Downloading {fam}…"), ctx);
                app.online.install_font(id, fam, ctx);
            }
        }
        _ => {}
    }
    open && !close
}

fn gallery(ctx: &egui::Context, app: &mut App, g: &mut GalleryState, open: &mut bool, close: &mut bool) {
    const PER_PAGE: usize = 48;
    window("Pictures").open(open).default_size([900.0, 620.0]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut g.tab, 1, RichText::new("Google Images").size(15.0));
            ui.selectable_value(&mut g.tab, 2, RichText::new("My Art").size(15.0));
            if !crate::platform::WEB {
                ui.selectable_value(&mut g.tab, 0, RichText::new("Content Packs").size(15.0));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if g.tab != 2 {
                    ui.checkbox(&mut g.keep, "Also keep a copy in My Art");
                }
            });
        });
        ui.separator();
        let mut chosen: Option<usize> = None;
        egui::Panel::left("gallery-side").resizable(false).exact_size(220.0).show(ui, |ui| {
            ui.label(if g.tab == 1 { "What are you looking for?" } else { "Enter search words, then click Go:" });
            let mut go = false;
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut g.words).desired_width(ui.available_width() - 44.0));
                go |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                go |= ui.button("Go").clicked();
            });
            if g.tab == 2 {
                ui.add_space(6.0);
                ui.label(RichText::new("Pictures you saved for reuse. They live in a normal folder, so you can rename or delete them there.").weak().size(12.0));
                if ui.button("Open My Art Folder").clicked() {
                    crate::library::open_folder();
                }
                if ui.button("Refresh").clicked() || go {
                    g.mine = crate::library::list();
                    g.thumbs.retain(|k, _| *k < 1_000_000_000);
                }
            } else if g.tab == 0 {
                ui.add_space(6.0);
                ui.label("Choose a pack:");
                let packs = app.library.as_ref().map(|l| l.packs.clone()).unwrap_or_default();
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    if ui.selectable_label(g.pack.is_none(), "All packs").clicked() {
                        g.pack = None;
                        go = true;
                    }
                    for (i, p) in packs.iter().enumerate().filter(|(_, p)| p.data.is_some() && p.items > 0) {
                        if ui.selectable_label(g.pack == Some(i), &p.name).clicked() {
                            g.pack = Some(i);
                            go = true;
                        }
                    }
                });
                if go || !g.searched {
                    if let Some(lib) = &app.library {
                        g.hits = lib.search(&g.words, g.pack).into_iter().filter(|i| lib.packs[lib.items[*i].pack].data.is_some()).collect();
                    }
                    g.page = 0;
                    g.searched = true;
                }
                if ui.button("Add content folders...").clicked() {
                    app.dialogs.push(Dialog::ContentPacks);
                }
            } else {
                ui.add_space(6.0);
                ui.radio_value(&mut g.web_kind, 0, "Any kind of picture");
                ui.radio_value(&mut g.web_kind, 1, "Clip art");
                ui.radio_value(&mut g.web_kind, 2, "Photos");
                ui.radio_value(&mut g.web_kind, 3, "Line drawings");
                ui.add_space(4.0);
                ui.checkbox(&mut g.web_clear, "See-through background");
                ui.checkbox(&mut g.web_free, "Free to reuse (open licenses)");
                ui.add_space(8.0);
                let search = ui.add(egui::Button::new(RichText::new("🔍  Search Google Images").size(15.0)).min_size(egui::vec2(200.0, 34.0))).on_hover_text("Opens Google Images; pictures you pick go straight onto your page");
                let url = google_images_url(&g.words, g.web_kind, g.web_free, g.web_clear);
                if go || search.clicked() {
                    app.online.web_words = g.words.clone();
                    app.online.web_keep = g.keep;
                    if crate::platform::WEB {
                        // In the browser version a pasted or dropped picture goes
                        // straight onto the page, so this window isn't needed.
                        crate::platform::open_url(&url);
                        app.web_search_hint(ctx);
                        *close = true;
                    } else {
                        match app.online.google_window(&url, ctx) {
                        Ok(()) => {
                            // Picks go straight onto the page; this window has done its job.
                            g.in_browser = false;
                            app.google_window_hint(ctx);
                            *close = true;
                        }
                        Err(e) => {
                            // No web engine here: use the web browser instead.
                            app.notify(format!("{e} Opening your web browser instead."), ctx);
                            g.in_browser = true;
                            crate::ai::open_in_browser(&url);
                            g.clip_seen = clipboard_look().map(|c| c.0);
                            g.clip_pic = None;
                            g.clip_url = None;
                        }
                        }
                    }
                }
                ui.add_space(4.0);
                if !crate::platform::WEB && ui.link("Use my web browser instead").on_hover_text("Opens Google Images in your usual web browser; copy a picture there and it shows up here").clicked() {
                    g.in_browser = true;
                    crate::ai::open_in_browser(&url);
                    g.clip_seen = clipboard_look().map(|c| c.0);
                    g.clip_pic = None;
                    g.clip_url = None;
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Pictures on Google belong to the people who made them. For things you share or sell, tick \"Free to reuse\".").weak().size(12.0));
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            if g.tab == 2 {
                let words = g.words.to_lowercase();
                let list: Vec<(usize, std::path::PathBuf)> = g.mine.iter().cloned().enumerate().filter(|(_, p)| words.is_empty() || p.to_string_lossy().to_lowercase().contains(&words)).collect();
                if list.is_empty() {
                    ui.label("Nothing here yet. Save a picture to My Art from Pictures, or tick \"Also keep a copy in My Art\" when you pick one.");
                }
                let mut budget = 4;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (i, path) in list {
                            let key = 1_000_000_000 + i;
                            if !g.thumbs.contains_key(&key) && budget > 0 {
                                budget -= 1;
                                let tex = std::fs::read(&path).ok().and_then(|b| thumb(&b)).map(|ci| ctx.load_texture(format!("mine{i}"), ci, egui::TextureOptions::LINEAR));
                                g.thumbs.insert(key, tex);
                            }
                            let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Picture").to_string();
                            if tile(ui, g.thumbs.get(&key).and_then(|t| t.as_ref()), &title).clicked() {
                                app.add_file(path.clone(), ctx);
                                *close = true;
                            }
                        }
                    });
                });
                if budget == 0 {
                    ctx.request_repaint();
                }
                return;
            }
            if g.tab == 0 {
                let Some(lib) = app.library.as_mut() else {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label("No content packs yet.");
                        if ui.button("Add My Content Packs...").clicked() {
                            app.dialogs.push(Dialog::ContentPacks);
                        }
                    });
                    return;
                };
                let total = g.hits.len();
                ui.horizontal(|ui| {
                    ui.label(format!("{total} pictures"));
                    if ui.add_enabled(g.page > 0, egui::Button::new("◀")).clicked() {
                        g.page -= 1;
                    }
                    ui.label(format!("Page {} of {}", g.page + 1, total.div_ceil(PER_PAGE).max(1)));
                    if ui.add_enabled((g.page + 1) * PER_PAGE < total, egui::Button::new("▶")).clicked() {
                        g.page += 1;
                    }
                });
                let mut budget = 4; // thumbnails made per frame
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for &hit in g.hits.iter().skip(g.page * PER_PAGE).take(PER_PAGE) {
                            if !g.thumbs.contains_key(&hit) && budget > 0 {
                                budget -= 1;
                                let item = lib.items[hit].clone();
                                let tex = lib.load(&item).ok().and_then(|b| thumb(&b)).map(|ci| ctx.load_texture(format!("pk{hit}"), ci, egui::TextureOptions::LINEAR));
                                g.thumbs.insert(hit, tex);
                            }
                            let title = lib.items[hit].title.clone();
                            if tile(ui, g.thumbs.get(&hit).and_then(|t| t.as_ref()), &title).clicked() {
                                chosen = Some(hit);
                            }
                        }
                    });
                });
                if budget == 0 {
                    ctx.request_repaint();
                }
            } else {
                google_tab(ctx, ui, app, g, close);
            }
        });
        if let Some(hit) = chosen
            && let Some(lib) = app.library.as_mut()
        {
            let item = lib.items[hit].clone();
            let pack = lib.packs[item.pack].name.clone();
            match lib.load(&item) {
                Ok(bytes) => {
                    if g.keep {
                        let _ = crate::library::save(&item.title, &bytes, &format!("From your content pack: {pack}"));
                    }
                    app.add_picture_bytes(&item.title, bytes, &format!("From your content pack: {pack}"), ctx);
                    *close = true;
                }
                Err(e) => app.notify(e, ctx),
            }
        }
    });
}

/// The Google Images tab: Google opens in the browser; a picture copied
/// there (Copy Image, or Copy Image Address) shows up here to add.
fn google_tab(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut App, g: &mut GalleryState, close: &mut bool) {
    // Look at the clipboard about once a second while this tab is open.
    let now = ctx.input(|i| i.time);
    if now - g.clip_poll > 0.9 {
        g.clip_poll = now;
        if let Some((print, pic, url)) = clipboard_look() {
            match g.clip_seen {
                None => g.clip_seen = Some(print),
                Some(seen) if seen != print => {
                    g.clip_seen = Some(print);
                    if let Some((w, h, rgba)) = pic
                        && let Some(img) = image::RgbaImage::from_raw(w, h, rgba)
                    {
                        let dynimg = image::DynamicImage::ImageRgba8(img);
                        let t = dynimg.thumbnail(260, 260).to_rgba8();
                        let tex = ctx.load_texture("copied-picture", egui::ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], t.as_raw()), egui::TextureOptions::LINEAR);
                        g.clip_pic = Some((crate::photo::to_png(&dynimg), tex));
                        g.clip_url = None;
                    } else if let Some(u) = url {
                        g.clip_url = Some(u);
                        g.clip_pic = None;
                    }
                }
                _ => {}
            }
        }
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(1000));

    let strong = ui.visuals().strong_text_color();
    ui.add_space(10.0);
    ui.label(RichText::new("Find a picture with Google Images").size(20.0).color(strong));
    ui.add_space(10.0);
    let steps: [&str; 3] = if crate::platform::WEB {
        [
            "Type what you're looking for on the left, then click Search Google Images. Google opens in a new browser tab.",
            "Right-click a picture you like and choose Copy Image.",
            "Come back to PrintCraft and press Ctrl+V (⌘V on a Mac): the picture goes onto your page. You can also drag a picture from the Google tab onto your page.",
        ]
    } else if g.in_browser {
        [
            "Google Images is open in your web browser.",
            "Right-click a picture you like and choose Copy Image.",
            "Come back to PrintCraft. The picture shows up below. Click Add to Page.",
        ]
    } else {
        [
            "Type what you're looking for on the left, then click Search Google Images. Google opens in a PrintCraft window.",
            "Point at a picture you like and click \"+ Add to my project\". (Click the picture first for a bigger, sharper copy.)",
            "The picture goes straight onto your page. Add as many as you like, then close the Google window.",
        ]
    };
    for (n, step) in steps
    .iter()
    .enumerate()
    {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 12.0, ui.visuals().selection.bg_fill);
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, format!("{}", n + 1), egui::FontId::proportional(14.0), Color32::WHITE);
            ui.add(egui::Label::new(RichText::new(*step).size(15.0)).wrap());
        });
        ui.add_space(6.0);
    }
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    let credit = if g.words.trim().is_empty() { "Copied from Google Images".to_string() } else { format!("Copied from a Google Images search for “{}”", g.words.trim()) };
    let name = if g.words.trim().is_empty() { "Picture".to_string() } else { g.words.trim().to_string() };
    if let Some((png, tex)) = g.clip_pic.clone() {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(&tex).max_size(egui::vec2(260.0, 260.0)));
            ui.vertical(|ui| {
                ui.label(RichText::new("You copied this picture.").size(15.0));
                ui.add_space(8.0);
                if ui.add(egui::Button::new(RichText::new("Add to Page").size(15.0)).min_size(egui::vec2(140.0, 34.0))).clicked() {
                    if g.keep {
                        let _ = crate::library::save(&name, &png, &credit);
                    }
                    app.add_picture_bytes(&name, png.clone(), &credit, ctx);
                    *close = true;
                }
                if ui.button("Not this one").clicked() {
                    g.clip_pic = None;
                }
            });
        });
    } else if let Some(url) = g.clip_url.clone() {
        ui.label(RichText::new("You copied a picture's address.").size(15.0));
        ui.label(RichText::new(&url).weak().size(11.0));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new("Add to Page").size(15.0)).min_size(egui::vec2(140.0, 34.0))).clicked() {
                app.notify("Getting the picture…", ctx);
                app.keep_web = g.keep;
                app.online.fetch_picture(WebPicture { title: name.clone(), thumb_url: url.clone(), full_url: url.clone(), credit: format!("{credit} ({url})") }, ctx);
                *close = true;
            }
            if ui.button("Not this one").clicked() {
                g.clip_url = None;
            }
        });
    } else if g.in_browser {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new("Waiting for a picture you copy…").size(15.0).weak());
        });
        ui.add_space(6.0);
        ui.label(RichText::new("You can also drag a saved picture file onto PrintCraft, or press Ctrl+V on the page.").weak().size(12.0));
    } else {
        ui.label(RichText::new("Pictures you copy in a web browser show up here too. You can also drag a saved picture file onto PrintCraft, or press Ctrl+V on the page.").weak().size(12.0));
    }
}

fn thumb(bytes: &[u8]) -> Option<egui::ColorImage> {
    let pm = if bytes.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) {
        crate::legacy::wmf::render(bytes, 150)?
    } else {
        let img = image::load_from_memory(bytes).ok()?.thumbnail(150, 150).to_rgba8();
        return Some(egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()));
    };
    let rgba = crate::raster::pixmap_to_rgba(&pm);
    Some(egui::ColorImage::from_rgba_unmultiplied([pm.width() as usize, pm.height() as usize], &rgba))
}

fn tile(ui: &mut egui::Ui, tex: Option<&TextureHandle>, title: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(128.0, 150.0), egui::Sense::click());
    let v = ui.visuals();
    ui.painter().rect_filled(rect, 8.0, if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
    let img_r = egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(112.0, 108.0));
    ui.painter().rect_filled(img_r, 4.0, Color32::WHITE);
    if let Some(t) = tex {
        let s = t.size_vec2();
        let k = (img_r.width() / s.x).min(img_r.height() / s.y);
        let r = egui::Rect::from_center_size(img_r.center(), s * k);
        ui.painter().image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
    }
    let mut t = title.to_string();
    if t.chars().count() > 18 {
        t = t.chars().take(17).collect::<String>() + "…";
    }
    ui.painter().text(rect.center_bottom() - egui::vec2(0.0, 8.0), egui::Align2::CENTER_BOTTOM, t, egui::FontId::proportional(12.0), v.text_color());
    resp.on_hover_text(title)
}

/// The calendar maker's choices (New Project and Tools > Calendar Maker).
fn calendar_options(ui: &mut egui::Ui, cal: &mut crate::calendar::CalendarOptions, landscape: bool) {
    use crate::calendar::{MONTHS, SCHEMES};
    egui::Grid::new("calendar-options").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Starting with:");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("cal-month").width(120.0).selected_text(MONTHS[cal.month as usize - 1]).show_ui(ui, |ui| {
                for (i, m) in MONTHS.iter().enumerate() {
                    ui.selectable_value(&mut cal.month, i as u32 + 1, *m);
                }
            });
            crate::widgets::number(ui, &mut cal.year, 1900..=2200, 1.0, |d| d.speed(0.2));
        });
        ui.end_row();
        ui.label("How many months:");
        crate::widgets::stepper(ui, &mut cal.months, 1, 12, "");
        ui.end_row();
        ui.label("Weeks start on:");
        ui.horizontal(|ui| {
            ui.radio_value(&mut cal.monday_first, false, "Sunday");
            ui.radio_value(&mut cal.monday_first, true, "Monday");
        });
        ui.end_row();
        ui.label("Colors:");
        ui.horizontal_wrapped(|ui| {
            for (i, s) in SCHEMES.iter().enumerate() {
                let fill = Color32::from_rgb(u8::from_str_radix(&s.2[1..3], 16).unwrap_or(0), u8::from_str_radix(&s.2[3..5], 16).unwrap_or(0), u8::from_str_radix(&s.2[5..7], 16).unwrap_or(0));
                let ink = Color32::from_rgb(u8::from_str_radix(&s.3[1..3], 16).unwrap_or(255), u8::from_str_radix(&s.3[3..5], 16).unwrap_or(255), u8::from_str_radix(&s.3[5..7], 16).unwrap_or(255));
                let b = egui::Button::new(RichText::new(s.0).color(ink)).fill(fill).selected(cal.scheme == i);
                if ui.add(b).clicked() {
                    cal.scheme = i;
                }
            }
        });
        ui.end_row();
    });
    ui.checkbox(&mut cal.holidays, "Show holidays (US)");
    ui.checkbox(&mut cal.birthdays, "Show birthdays from my address book");
    if !landscape {
        ui.checkbox(&mut cal.picture_space, "Leave room for a photo above each month");
    } else {
        ui.label(RichText::new("Tip: choose Tall for room for a photo above each month.").weak().size(12.0));
    }
}

/// A ready-made design (or Blank) as a little picture to click.
fn design_tile(ui: &mut egui::Ui, app: &mut App, ctx: &egui::Context, on: bool, name: &str, d: Option<&crate::templates::Design>) -> bool {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(150.0, 168.0), egui::Sense::click());
    let v = ui.visuals().clone();
    ui.painter().rect_filled(r, 10.0, if on { v.selection.bg_fill.gamma_multiply(0.3) } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
    if on {
        ui.painter().rect_stroke(r, 10.0, egui::Stroke::new(2.0, v.selection.bg_fill), egui::StrokeKind::Inside);
    }
    let pic = egui::Rect::from_min_max(r.min + egui::vec2(8.0, 8.0), egui::pos2(r.right() - 8.0, r.bottom() - 36.0));
    match d {
        Some(d) => {
            if !app.design_thumbs.contains_key(d.name) {
                let p = (d.build)();
                let ppi = 130.0 / p.width.max(p.height);
                if let Some(pm) = crate::raster::render_panel(&p, 0, &app.fonts, &app.pics, ppi) {
                    let rgba = crate::raster::pixmap_to_rgba(&pm);
                    let tex = ctx.load_texture(format!("design-{}", d.name), egui::ColorImage::from_rgba_unmultiplied([pm.width() as usize, pm.height() as usize], &rgba), egui::TextureOptions::LINEAR);
                    app.design_thumbs.insert(d.name.to_string(), tex);
                }
            }
            if let Some(t) = app.design_thumbs.get(d.name) {
                let k = (pic.width() / t.size_vec2().x).min(pic.height() / t.size_vec2().y);
                let img = egui::Rect::from_center_size(pic.center(), t.size_vec2() * k);
                ui.painter().rect_filled(img.expand(1.0), 2.0, Color32::from_gray(200));
                ui.painter().image(t.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            }
        }
        None => {
            let img = egui::Rect::from_center_size(pic.center(), egui::vec2(pic.width() * 0.6, pic.height() * 0.8));
            ui.painter().rect_filled(img, 2.0, Color32::WHITE);
            ui.painter().rect_stroke(img, 2.0, egui::Stroke::new(1.0, Color32::from_gray(180)), egui::StrokeKind::Inside);
            ui.painter().text(img.center(), egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(30.0), Color32::from_gray(170));
        }
    }
    let label = egui::WidgetText::from(RichText::new(name).size(12.5)).into_galley(ui, Some(egui::TextWrapMode::Wrap), r.width() - 12.0, egui::TextStyle::Body);
    ui.painter().galley(egui::pos2(r.center().x - label.size().x / 2.0, r.bottom() - 32.0), label, v.text_color());
    resp.on_hover_text(name).clicked()
}

/// Page Setup: paper size and direction for kinds made from a sheet of
/// paper; the sheet, envelope or banner length for the others.
fn page_setup(ctx: &egui::Context, app: &mut App, s: &mut PageSetupState, open: &mut bool, close: &mut bool) {
    use crate::kinds::{PAPERS, page_for_paper};
    let kind = app.doc.kind;
    let mut ok = false;
    window("Page Setup").open(open).default_size([520.0, 420.0]).show(ctx, |ui| {
        ui.label(RichText::new(format!("{} — page size", kind.label())).size(16.0));
        ui.add_space(6.0);
        let mut page = (app.doc.width, app.doc.height);
        if kind.paper_use().is_some() {
            egui::Grid::new("paper").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                ui.label("Paper:");
                let name = PAPERS.get(s.paper).map(|p| p.0).unwrap_or("Custom size");
                egui::ComboBox::from_id_salt("paper-size").width(260.0).selected_text(name).show_ui(ui, |ui| {
                    for (i, p) in PAPERS.iter().enumerate() {
                        ui.selectable_value(&mut s.paper, i, p.0);
                    }
                    ui.selectable_value(&mut s.paper, PAPERS.len(), "Custom size");
                });
                ui.end_row();
                if s.paper >= PAPERS.len() {
                    ui.label("Size:");
                    ui.horizontal(|ui| {
                        crate::widgets::number(ui, &mut s.custom.0, 2.0..=24.0, 0.25, |d| d.speed(0.05).suffix(" in"));
                        ui.label("×");
                        crate::widgets::number(ui, &mut s.custom.1, 2.0..=48.0, 0.25, |d| d.speed(0.05).suffix(" in"));
                    });
                    ui.end_row();
                }
                ui.label("Direction:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.landscape, false, "Tall");
                    ui.radio_value(&mut s.landscape, true, "Wide");
                });
                ui.end_row();
            });
            let paper = PAPERS.get(s.paper).map(|p| (p.1, p.2)).unwrap_or(s.custom);
            page = page_for_paper(kind, paper, s.landscape);
            if kind.paper_use() != Some(crate::kinds::PaperUse::Whole) {
                ui.label(RichText::new(format!("Each panel of the card: {:.2} × {:.2} in (the paper is folded).", page.0, page.1)).weak().size(12.0));
            }
        } else if kind == Kind::Envelope {
            let list = crate::kinds::ENVELOPES;
            s.option = s.option.min(list.len() - 1);
            egui::ComboBox::from_label("Envelope size").width(300.0).selected_text(list[s.option].0).show_ui(ui, |ui| {
                for (i, e) in list.iter().enumerate() {
                    ui.selectable_value(&mut s.option, i, e.0);
                }
            });
            ui.horizontal(|ui| {
                ui.radio_value(&mut s.landscape, true, "Wide (address side)");
                ui.radio_value(&mut s.landscape, false, "Tall");
            });
            let (_, w, h) = list[s.option];
            page = if s.landscape { (w.max(h), w.min(h)) } else { (w.min(h), w.max(h)) };
        } else if kind == Kind::Banner {
            crate::widgets::stepper(ui, &mut s.sheets, 1, 20, "Sheets of paper:");
            ui.horizontal(|ui| {
                ui.label("Each sheet:");
                ui.radio_value(&mut s.tall, false, "Sideways (banner 8½ in tall)");
                ui.radio_value(&mut s.tall, true, "Upright (banner 11 in tall)");
            });
            let b = crate::model::BannerSetup { sheets: s.sheets, tall: s.tall };
            let (seg, h) = Project::banner_segment(&b, crate::model::BANNER_MARGIN);
            page = (seg * s.sheets as f32, h);
        } else {
            let presets = crate::kinds::sheet_presets(kind);
            if !presets.is_empty() {
                s.option = s.option.min(presets.len() - 1);
                egui::ComboBox::from_label("Sheet").width(380.0).selected_text(presets[s.option].label).show_ui(ui, |ui| {
                    for (i, p) in presets.iter().enumerate() {
                        ui.selectable_value(&mut s.option, i, p.label);
                    }
                });
                page = (presets[s.option].w, presets[s.option].h);
            } else {
                ui.label("This kind of project has one size.");
            }
        }
        ui.add_space(8.0);
        // A small picture of the new page next to the old one.
        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(460.0), 140.0), egui::Sense::hover());
        let scale = 100.0 / page.0.max(page.1).max(app.doc.width).max(app.doc.height);
        let v = ui.visuals().clone();
        let old = egui::Rect::from_min_size(r.left_top() + egui::vec2(10.0, 10.0), egui::vec2(app.doc.width, app.doc.height) * scale);
        let new = egui::Rect::from_min_size(r.left_top() + egui::vec2(180.0, 10.0), egui::vec2(page.0, page.1) * scale);
        ui.painter().rect_stroke(old, 2.0, egui::Stroke::new(1.0, v.weak_text_color()), egui::StrokeKind::Inside);
        ui.painter().text(old.center_bottom() + egui::vec2(0.0, 4.0), egui::Align2::CENTER_TOP, format!("Now: {:.2} × {:.2} in", app.doc.width, app.doc.height), egui::FontId::proportional(12.0), v.weak_text_color());
        ui.painter().rect_filled(new, 2.0, Color32::WHITE);
        ui.painter().rect_stroke(new, 2.0, egui::Stroke::new(1.5, v.selection.bg_fill), egui::StrokeKind::Inside);
        ui.painter().text(new.center_bottom() + egui::vec2(0.0, 4.0), egui::Align2::CENTER_TOP, format!("New: {:.2} × {:.2} in", page.0, page.1), egui::FontId::proportional(12.0), v.strong_text_color());
        ui.add_space(4.0);
        ui.checkbox(&mut s.fit, "Fit what's on the page to the new size");
        ui.label(RichText::new("Everything is made larger or smaller together and centered; borders follow the edge of the page.").weak().size(12.0));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new("OK").size(15.0)).min_size(egui::vec2(90.0, 30.0))).clicked() {
                ok = true;
            }
            if ui.button("Cancel").clicked() {
                *close = true;
            }
        });
        if ok {
            app.checkpoint();
            if kind == Kind::Banner {
                let ow = app.doc.width;
                app.doc.set_banner(crate::model::BannerSetup { sheets: s.sheets, tall: s.tall });
                let (nw, nh) = (app.doc.width, app.doc.height);
                app.doc.width = ow;
                app.doc.resize_pages(nw, nh, s.fit);
            } else {
                if let Some(pr) = crate::kinds::sheet_presets(kind).get(s.option) {
                    app.doc.sheet = Some(pr.layout.clone());
                }
                app.doc.landscape = page.0 > page.1;
                app.doc.resize_pages(page.0, page.1, s.fit);
            }
            app.fit_pending = true;
            *close = true;
        }
    });
}

#[cfg(test)]
mod google_tests {
    #[test]
    fn search_address() {
        assert_eq!(super::google_images_url("  ", 0, false, false), "https://images.google.com/");
        assert_eq!(super::google_images_url("red barn", 0, false, false), "https://www.google.com/search?udm=2&q=red+barn");
        assert_eq!(super::google_images_url("cat & dog", 1, true, true), "https://www.google.com/search?udm=2&q=cat+%26+dog&tbs=itp:clipart,ic:trans,sur:cl");
    }
}
