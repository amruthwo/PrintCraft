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
use crate::photo::{Adjust, Effect};
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

pub struct PhotoState {
    pub item: u64,
    pub asset: u64,
    pub tab: u8,
    pub adjust: Adjust,
    pub preview: Option<TextureHandle>,
    pub previewed: Option<Adjust>,
    /// Cut-out tool: 0 shape, 1 draw outline, 2 magic wand, 3 pick a color, 4 remove background.
    pub tool: u8,
    pub how: crate::photo::Combine,
    pub tolerance: f32,
    pub soft: f32,
    pub lasso: Vec<(f32, f32)>,
    pub work: Option<(([f32; 4], u8), image::RgbaImage)>,
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
    NewProject { kind: Kind, landscape: bool, option: usize, sheets: u32, tall: bool },
    Message { title: String, body: String },
    Headline(Box<HeadlineState>),
    Gallery(Box<GalleryState>),
    Photo(Box<PhotoState>),
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
        Dialog::NewProject { kind: Kind::CardQuarter, landscape: Kind::CardQuarter.default_landscape(), option: 0, sheets: 4, tall: false }
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
        Dialog::Photo(Box::new(PhotoState {
            item,
            asset,
            tab,
            adjust: Adjust::default(),
            preview: None,
            previewed: None,
            tool: 0,
            how: crate::photo::Combine::New,
            tolerance: 18.0,
            soft: 1.0,
            lasso: Vec::new(),
            work: None,
        }))
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
        Dialog::NewProject { kind, landscape, option, sheets, tall } => {
            let mut create = false;
            window("New Project").open(&mut open).default_size([680.0, 640.0]).show(ctx, |ui| {
                ui.label(RichText::new("What would you like to make?").size(17.0));
                ui.add_space(6.0);
                egui::Grid::new("kinds").spacing([8.0, 8.0]).show(ui, |ui| {
                    for (i, k) in Kind::ALL.iter().enumerate() {
                        let [r, g, b] = k.tile_color();
                        let on = *kind == *k;
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(150.0, 112.0), egui::Sense::click());
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
                        }
                        if resp.double_clicked() {
                            create = true;
                        }
                        if i % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
                ui.add_space(8.0);
                // Size choices for kinds that come in standard sizes.
                let presets = crate::kinds::sheet_presets(*kind);
                if !presets.is_empty() {
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
                    let (mut w, mut h) = kind.size();
                    if *landscape != kind.default_landscape() {
                        std::mem::swap(&mut w, &mut h);
                    }
                    ui.label(RichText::new(format!("{w} × {h} in")).weak());
                });
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
            if create {
                let mut p = Project::new(*kind, *landscape);
                let presets = crate::kinds::sheet_presets(*kind);
                if let Some(pr) = presets.get(*option) {
                    p.width = pr.w;
                    p.height = pr.h;
                    p.sheet = Some(pr.layout.clone());
                } else if *kind == Kind::Envelope {
                    let (_, w, h) = crate::kinds::ENVELOPES[(*option).min(crate::kinds::ENVELOPES.len() - 1)];
                    (p.width, p.height) = if *landscape { (w, h) } else { (h, w) };
                } else if *kind == Kind::Banner {
                    p.set_banner(crate::model::BannerSetup { sheets: *sheets, tall: *tall });
                }
                app.open_tab(p, None, false);
                close = true;
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
                    ui.add_enabled(h.warp != Warp::Straight, egui::Slider::new(&mut h.bend, -4.0..=4.0).text("How much it bends"));
                    ui.add(egui::Slider::new(&mut h.bulge, -2.0..=2.0).text("Bulge (mixes with any shape)"));
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
        Dialog::Photo(p) => photo(ctx, app, p, &mut open, &mut close),
        Dialog::Effects { which, amount, angle, size, color, style } => {
            let title = ["Apply Drop Shadow", "Apply Glow", "Apply Frame"][*which as usize];
            let mut apply = None;
            window(title).open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label(RichText::new("Drag this window by its title to see your item while you change it.").weak().size(12.0));
                let mut live = false;
                match which {
                    0 => {
                        live |= ui.add(egui::Slider::new(amount, 0.0..=1.0).text("Distance").show_value(false)).changed();
                        live |= ui.add(egui::Slider::new(angle, 0.0..=360.0).text("Direction").suffix("°")).changed();
                        live |= ui.add(egui::Slider::new(size, 0.0..=1.0).text("Softness").show_value(false)).changed();
                        live |= color_row(ui, &mut app.settings, &mut app.eyedropper, "Shadow color", color);
                        let mut a = color.3 as f32 / 255.0;
                        if ui.add(egui::Slider::new(&mut a, 0.1..=1.0).text("Darkness")).changed() {
                            color.3 = (a * 255.0) as u8;
                            live = true;
                        }
                    }
                    1 => {
                        live |= ui.add(egui::Slider::new(size, 0.0..=1.0).text("Size").show_value(false)).changed();
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
                if ui.button("Use on every page").clicked() {
                    app.checkpoint();
                    let (bg, pat, col) = (app.page().background, app.page().pattern, app.page().pattern_color);
                    for p in app.doc.panels.iter_mut() {
                        p.background = bg;
                        p.pattern = pat;
                        p.pattern_color = col;
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
                ui.add(egui::Slider::new(copies, 1..=50).text("Copies"));
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
            window("Insert an Address").open(&mut open).resizable(false).show(ctx, |ui| {
                for (i, c) in app.settings.contacts.iter().enumerate() {
                    if ui.button(if c.name.is_empty() { "(no name)" } else { &c.name }).clicked() {
                        pick = Some(i);
                    }
                }
            });
            if let Some(i) = pick {
                let block = app.settings.contacts[i].mailing_block();
                let look = TextLook { size: 14.0, ..Default::default() };
                app.add_item(3.2, 1.0, Content::Text { text: block, look, shape: None, fit: false, runs: Vec::new() });
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
            ui.selectable_value(&mut g.tab, 0, RichText::new("My Content Packs").size(15.0));
            ui.selectable_value(&mut g.tab, 1, RichText::new("Google Images").size(15.0));
            ui.selectable_value(&mut g.tab, 2, RichText::new("My Art").size(15.0));
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
                let search = ui.add(egui::Button::new(RichText::new("🔍  Search Google Images").size(15.0)).min_size(egui::vec2(200.0, 34.0))).on_hover_text("Opens Google Images in your web browser");
                let url = google_images_url(&g.words, g.web_kind, g.web_free, g.web_clear);
                if go || search.clicked() {
                    app.online.web_words = g.words.clone();
                    app.online.web_keep = g.keep;
                    match app.online.google_window(&url, ctx) {
                        Ok(()) => g.in_browser = false,
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
                ui.add_space(4.0);
                if ui.link("Use my web browser instead").on_hover_text("Opens Google Images in your usual web browser; copy a picture there and it shows up here").clicked() {
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
    let steps: [&str; 3] = if g.in_browser {
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

const WORK_SIDE: u32 = 700;

fn photo(ctx: &egui::Context, app: &mut App, p: &mut PhotoState, open: &mut bool, close: &mut bool) {
    use crate::photo::Combine;
    let tabs = ["Crop & Turn", "Color & Recolor", "Brightness & Focus", "Red Eye", "Photo Effects", "Cut Out"];
    let mut done = false;
    // The cropped, turned picture the selection tools work on.
    let key = (p.adjust.crop, p.adjust.turns);
    if p.work.as_ref().is_none_or(|(k, _)| *k != key) {
        if p.work.is_some() {
            // The picture changed shape; an old selection no longer fits.
            p.adjust.mask = None;
            p.lasso.clear();
        }
        p.work = app.doc.assets.get(&p.asset).and_then(|a| crate::photo::geometry(&a.bytes, &p.adjust, WORK_SIDE)).map(|w| (key, w));
    }
    let mut picked: Option<(u32, u32, bool)> = None; // pixel and "double click"
    window("Photo Editor").open(open).default_size([900.0, 620.0]).show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, t) in tabs.iter().enumerate() {
                ui.selectable_value(&mut p.tab, i as u8, *t);
            }
        });
        ui.separator();
        egui::Panel::left("photo-tools").exact_size(280.0).resizable(false).show(ui, |ui| {
            let a = &mut p.adjust;
            match p.tab {
                0 => {
                    ui.label("Trim the edges:");
                    for (i, l) in ["Left", "Top", "Right", "Bottom"].iter().enumerate() {
                        ui.add(egui::Slider::new(&mut a.crop[i], 0.0..=0.45).text(*l).show_value(false));
                    }
                    if ui.button("⟳ Rotate 90 Degrees").clicked() {
                        a.turns = (a.turns + 1) % 4;
                    }
                }
                1 => {
                    ui.label(RichText::new("Adjust color").strong());
                    ui.add(egui::Slider::new(&mut a.saturation, -1.0..=1.0).text("Color strength"));
                    ui.add(egui::Slider::new(&mut a.warmth, -1.0..=1.0).text("Cool ↔ Warm"));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Recolor").strong());
                    ui.label(RichText::new("Turn every color around the color wheel: red becomes orange, then yellow, green, blue...").weak().size(12.0));
                    ui.add(egui::Slider::new(&mut a.hue, -180.0..=180.0).text("Shift colors").suffix("°"));
                    for (k, n) in ["More red", "More green", "More blue"].iter().enumerate() {
                        ui.add(egui::Slider::new(&mut a.rgb[k], -1.0..=1.0).text(*n).show_value(false));
                    }
                    ui.horizontal_wrapped(|ui| {
                        for (n, h) in [("Warmer", 20.0), ("Cooler", -20.0), ("Opposite", 180.0)] {
                            if ui.button(n).clicked() {
                                a.hue = h;
                            }
                        }
                    });
                }
                2 => {
                    ui.add(egui::Slider::new(&mut a.brightness, -100..=100).text("Brightness"));
                    ui.add(egui::Slider::new(&mut a.contrast, -50.0..=50.0).text("Contrast"));
                    ui.add(egui::Slider::new(&mut a.sharpen, 0.0..=1.0).text("Sharpen"));
                }
                3 => {
                    ui.checkbox(&mut a.red_eye, "Remove red eye");
                }
                4 => {
                    for e in Effect::ALL {
                        ui.radio_value(&mut a.effect, e, e.label());
                    }
                }
                _ => {
                    ui.label(RichText::new("How to cut out").strong());
                    let tools = ["A shape", "Draw my own outline", "Magic wand", "Pick a color", "Remove background"];
                    for (k, t) in tools.iter().enumerate() {
                        ui.radio_value(&mut p.tool, k as u8, *t);
                    }
                    ui.separator();
                    match p.tool {
                        0 => {
                            ui.radio_value(&mut a.shape, None, "No shape (rectangle)");
                            for o in [Outline::Ellipse, Outline::Rounded, Outline::Heart, Outline::Star, Outline::Diamond, Outline::Hexagon, Outline::Cloud, Outline::Burst] {
                                ui.radio_value(&mut a.shape, Some(o), o.label());
                            }
                        }
                        4 => {
                            ui.label(RichText::new("PrintCraft looks at the edges of the photo and removes the background color that touches them.").weak().size(12.0));
                            ui.add(egui::Slider::new(&mut p.tolerance, 2.0..=60.0).text("How much to remove"));
                            if ui.add(egui::Button::new("✨ Remove Background").min_size(egui::vec2(180.0, 30.0))).clicked()
                                && let Some((_, w)) = &p.work
                            {
                                let keep = crate::photo::remove_background(w, p.tolerance);
                                let m = crate::photo::combine(None, w.width(), w.height(), &keep, Combine::New);
                                a.mask = Some(std::sync::Arc::new(crate::photo::feather(&m, p.soft as u32)));
                            }
                        }
                        _ => {
                            ui.label(RichText::new(match p.tool {
                                1 => "Click around what you want to keep, point by point. Double-click (or click the first point) to finish.",
                                2 => "Click an area; the magic wand selects the touching area of similar color.",
                                _ => "Click a color; every part of the photo in that color is selected.",
                            }).weak().size(12.0));
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut p.how, Combine::New, "New");
                                ui.selectable_value(&mut p.how, Combine::Add, "Add");
                                ui.selectable_value(&mut p.how, Combine::Subtract, "Subtract");
                            });
                            if p.tool != 1 {
                                ui.add(egui::Slider::new(&mut p.tolerance, 2.0..=60.0).text("How similar"));
                            }
                        }
                    }
                    if p.tool != 0 {
                        ui.add(egui::Slider::new(&mut p.soft, 0.0..=8.0).text("Soft edges"));
                        ui.horizontal(|ui| {
                            if ui.button("Keep the opposite").on_hover_text("Swap what's kept and what's removed").clicked()
                                && let Some(m) = &a.mask
                            {
                                let mut n = (**m).clone();
                                for v in n.data.iter_mut() {
                                    *v = 255 - *v;
                                }
                                a.mask = Some(std::sync::Arc::new(n));
                            }
                            if ui.button("Clear").clicked() {
                                a.mask = None;
                                p.lasso.clear();
                            }
                        });
                    }
                }
            }
            ui.add_space(10.0);
            if ui.button("Undo All").clicked() {
                *a = Adjust::default();
                p.lasso.clear();
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            if p.previewed.as_ref() != Some(&p.adjust) {
                if let Some(asset) = app.doc.assets.get(&p.asset)
                    && let Some(img) = crate::photo::apply(&asset.bytes, &p.adjust, WORK_SIDE)
                {
                    let rgba = img.to_rgba8();
                    let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                    p.preview = Some(ctx.load_texture("photo-preview", ci, egui::TextureOptions::LINEAR));
                }
                p.previewed = Some(p.adjust.clone());
            }
            if let Some(t) = &p.preview {
                let avail = ui.available_size() - egui::vec2(16.0, 56.0);
                let s = t.size_vec2();
                let k = (avail.x / s.x).min(avail.y / s.y).max(0.05);
                let (r, resp) = ui.allocate_exact_size(s * k, egui::Sense::click());
                // Checkerboard shows what's been cut away.
                let cell = 10.0;
                let (mut y, mut row) = (r.top(), 0);
                while y < r.bottom() {
                    let (mut x, mut col) = (r.left(), 0);
                    while x < r.right() {
                        let c = if (row + col) % 2 == 0 { Color32::from_gray(225) } else { Color32::from_gray(250) };
                        ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(x, y), egui::pos2((x + cell).min(r.right()), (y + cell).min(r.bottom()))), 0.0, c);
                        x += cell;
                        col += 1;
                    }
                    y += cell;
                    row += 1;
                }
                ui.painter().image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                if p.tab == 5 && (1..=3).contains(&p.tool) {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    if let Some(pos) = resp.interact_pointer_pos()
                        && (resp.clicked() || resp.double_clicked())
                        && let Some((_, w)) = &p.work
                    {
                        let px = ((pos.x - r.left()) / r.width() * w.width() as f32).clamp(0.0, w.width() as f32 - 1.0);
                        let py = ((pos.y - r.top()) / r.height() * w.height() as f32).clamp(0.0, w.height() as f32 - 1.0);
                        picked = Some((px as u32, py as u32, resp.double_clicked()));
                    }
                    // Outline being drawn.
                    if p.tool == 1 && !p.lasso.is_empty()
                        && let Some((_, w)) = &p.work
                    {
                        let to_screen = |q: (f32, f32)| egui::pos2(r.left() + q.0 / w.width() as f32 * r.width(), r.top() + q.1 / w.height() as f32 * r.height());
                        let mut pts: Vec<egui::Pos2> = p.lasso.iter().map(|q| to_screen(*q)).collect();
                        if let Some(h) = resp.hover_pos() {
                            pts.push(h);
                        }
                        ui.painter().add(egui::Shape::line(pts.clone(), egui::Stroke::new(3.0, Color32::WHITE)));
                        ui.painter().add(egui::Shape::line(pts, egui::Stroke::new(1.5, Color32::from_rgb(222, 64, 160))));
                        ui.painter().circle(to_screen(p.lasso[0]), 6.0, Color32::from_rgb(222, 64, 160), egui::Stroke::new(1.5, Color32::WHITE));
                    }
                }
            }
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("OK").min_size(egui::vec2(90.0, 28.0))).clicked() {
                    done = true;
                }
                if ui.button("Cancel").clicked() {
                    *close = true;
                }
            });
        });
    });
    // Apply a click from the selection tools.
    if let Some((x, y, double)) = picked
        && let Some((_, w)) = &p.work
    {
        let (ww, wh) = (w.width(), w.height());
        let sel = match p.tool {
            1 => {
                let first = p.lasso.first().copied();
                let near_first = first.is_some_and(|f| ((f.0 - x as f32).powi(2) + (f.1 - y as f32).powi(2)).sqrt() < ww.max(wh) as f32 * 0.02);
                if (double || near_first) && p.lasso.len() >= 3 {
                    let s = crate::photo::lasso(ww, wh, &p.lasso);
                    p.lasso.clear();
                    Some(s)
                } else {
                    p.lasso.push((x as f32, y as f32));
                    None
                }
            }
            2 => Some(crate::photo::wand(w, x, y, p.tolerance)),
            _ => Some(crate::photo::color_select(w, w.get_pixel(x, y).0, p.tolerance)),
        };
        if let Some(sel) = sel {
            let m = crate::photo::combine(p.adjust.mask.as_deref(), ww, wh, &sel, p.how);
            p.adjust.mask = Some(std::sync::Arc::new(crate::photo::feather(&m, p.soft as u32)));
            if p.how == Combine::New {
                p.how = Combine::Add;
            }
        }
    }
    if done {
        if p.adjust != Adjust::default()
            && let Some(asset) = app.doc.assets.get(&p.asset).cloned()
            && let Some(img) = crate::photo::apply(&asset.bytes, &p.adjust, 3000)
        {
            let (w, h) = (img.width() as f32, img.height() as f32);
            let png = crate::photo::to_png(&img);
            app.checkpoint();
            let new_id = app.doc.add_asset(&asset.name, png, &asset.credit);
            if let Some(it) = app.item_mut(p.item) {
                it.content = Content::Picture { asset: new_id };
                // Keep the width, follow the new shape.
                it.frame.h = it.frame.w * h / w.max(1.0);
            }
        }
        *close = true;
    }
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
