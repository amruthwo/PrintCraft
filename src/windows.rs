//! More pop-up windows: settings, help, tabs and pages, save reminders,
//! recovery, shapes, borders, signatures, AI pictures and printing.

use eframe::egui::{self, Color32, RichText, TextureHandle};

use crate::app::{Act, App};
use crate::dialogs::{Dialog, window};
use crate::export::PrintOptions;
use crate::model::*;

pub struct BorderState {
    pub style: u8,
    pub line: Rgba,
    pub fill: Rgba,
    pub thickness: f32,
    pub inset: f32,
    pub previews: Vec<Option<TextureHandle>>,
}

pub struct PrintState {
    pub opts: PrintOptions,
    pub printers: Vec<String>,
    pub which: u8,
    pub range: String,
    pub quality: u8,
    pub pages: Vec<TextureHandle>,
    pub at: usize,
    pub preview_key: Option<PrintOptions>,
    pub zoom: f32,
}

pub struct AiState {
    pub prompt: String,
    pub key: String,
    pub provider: String,
    pub job: Option<crate::ai::Slot>,
    pub error: Option<String>,
}

pub struct SignatureState {
    pub name: String,
    pub family: String,
    pub size: f32,
    pub color: Rgba,
}

impl Dialog {
    pub fn borders() -> Self {
        Dialog::Borders(Box::new(BorderState { style: 0, line: Rgba::hex("#2f6fb0"), fill: Rgba::hex("#f2b631"), thickness: 18.0, inset: 0.25, previews: Vec::new() }))
    }
    pub fn ai() -> Self {
        Dialog::Ai(Box::new(AiState { prompt: String::new(), key: String::new(), provider: String::new(), job: None, error: None }))
    }
    pub fn print_window(preview_first: bool) -> Self {
        Dialog::PrintPro(Box::new(PrintState {
            opts: PrintOptions::default(),
            printers: crate::export::printers(),
            which: 0,
            range: String::new(),
            quality: 1,
            pages: Vec::new(),
            at: 0,
            preview_key: None,
            zoom: 1.0,
        }))
    }
    pub fn signature() -> Self {
        Dialog::Signature(Box::new(SignatureState { name: "Your Name".into(), family: crate::fonts::SCRIPT_FAMILY.into(), size: 32.0, color: Rgba::hex("#1d3f8a") }))
    }
}

fn preview_of(app: &App, ctx: &egui::Context, item: &Item, ppi: f32, name: String) -> Option<TextureHandle> {
    let r = crate::raster::render_item(item, &app.doc, &app.fonts, &app.pics, ppi)?;
    let rgba = crate::raster::pixmap_to_rgba(&r.pixmap);
    Some(ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([r.pixmap.width() as usize, r.pixmap.height() as usize], &rgba), egui::TextureOptions::LINEAR))
}

/// Shows the windows this module owns. Returns None for other dialogs.
pub fn show(ctx: &egui::Context, app: &mut App, d: &mut Dialog, acts: &mut Vec<Act>) -> Option<bool> {
    let mut open = true;
    let mut close = false;
    match d {
        Dialog::Help { query } => {
            window("Help Topics").open(&mut open).default_size([560.0, 520.0]).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(egui::TextEdit::singleline(query).hint_text("What do you need help with?").desired_width(f32::INFINITY));
                });
                ui.separator();
                let q = query.to_lowercase();
                let words: Vec<&str> = q.split_whitespace().collect();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut any = false;
                    for (title, body) in crate::help::TOPICS {
                        let text = format!("{title} {body}").to_lowercase();
                        if words.iter().all(|w| text.contains(w)) {
                            any = true;
                            egui::CollapsingHeader::new(RichText::new(title).strong().size(15.0)).default_open(!words.is_empty()).show(ui, |ui| {
                                ui.label(body);
                            });
                        }
                    }
                    if !any {
                        ui.label("No help topic matches. Try fewer or different words.");
                    }
                });
            });
        }
        Dialog::Settings => settings(ctx, app, &mut open),
        Dialog::RenameTab { index, name } | Dialog::RenamePage { index, name } => {
            let page = matches!(d, Dialog::RenamePage { .. });
            let (index, name) = match d {
                Dialog::RenameTab { index, name } | Dialog::RenamePage { index, name } => (*index, name),
                _ => unreachable!(),
            };
            let mut ok = false;
            window(if page { "Rename Page" } else { "Rename Tab" }).open(&mut open).resizable(false).show(ctx, |ui| {
                let r = ui.text_edit_singleline(name);
                r.request_focus();
                ok = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    ok |= ui.button("OK").clicked();
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if ok {
                if page {
                    if index < app.doc.panels.len() {
                        app.checkpoint();
                        app.doc.panels[index].name = name.clone();
                    }
                } else {
                    app.rename_tab(index, name.clone());
                }
                close = true;
            }
        }
        Dialog::ConfirmCloseTab(i) => {
            let i = *i;
            let name = app.tab_views().get(i).map(|t| t.0.clone()).unwrap_or_default();
            window("Save changes?").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
                ui.label(format!("“{name}” has changes that aren't saved yet."));
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Save").strong()).min_size(egui::vec2(90.0, 28.0))).clicked() {
                        app.switch_tab(i);
                        app.save(ctx, false);
                        if !app.dirty {
                            app.close_tab(app.active);
                        }
                        close = true;
                    }
                    if ui.button("Don't Save").clicked() {
                        app.close_tab(i);
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        }
        Dialog::ConfirmQuit => {
            let unsaved = app.unsaved();
            window("Save before closing?").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
                ui.label("These projects have changes that aren't saved yet:");
                for n in &unsaved {
                    ui.label(format!("  • {n}"));
                }
                ui.label(RichText::new("PrintCraft also keeps a safety copy, offered back next time if you don't save.").weak());
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Save All").strong()).min_size(egui::vec2(100.0, 28.0))).clicked() {
                        for i in 0..app.tabs.len() {
                            app.switch_tab(i);
                            if app.dirty {
                                app.save(ctx, false);
                            }
                        }
                        if app.unsaved().is_empty() {
                            app.quit = true;
                        }
                        close = true;
                    }
                    if ui.button("Close Without Saving").clicked() {
                        for t in &app.tabs {
                            crate::recovery::forget(t.recovery_id);
                        }
                        crate::recovery::forget(app.recovery_id);
                        app.quit = true;
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        }
        Dialog::Recover(list) => {
            let mut done = false;
            window("Bring back your work?").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label("PrintCraft didn't close normally last time. These projects were saved for you:");
                let mut restore = None;
                let mut discard = None;
                for (k, (id, name, doc)) in list.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("• {name} ({})", doc.kind.label()));
                        if ui.button("Bring Back").clicked() {
                            restore = Some(k);
                        }
                        if ui.button("Throw Away").clicked() {
                            discard = Some((k, *id));
                        }
                    });
                }
                if let Some(k) = restore {
                    let (id, name, doc) = list.remove(k);
                    app.open_tab(doc, None, true);
                    app.rename_tab(app.active, format!("{name} (recovered)"));
                    crate::recovery::forget(id);
                }
                if let Some((k, id)) = discard {
                    list.remove(k);
                    crate::recovery::forget(id);
                }
                ui.add_space(6.0);
                if ui.button("Bring Back All").clicked() {
                    for (id, name, doc) in list.drain(..) {
                        app.open_tab(doc, None, true);
                        app.rename_tab(app.active, format!("{name} (recovered)"));
                        crate::recovery::forget(id);
                    }
                }
                done = list.is_empty();
            });
            close |= done;
        }
        Dialog::Shapes => {
            let mut pick = None;
            window("Shapes").open(&mut open).default_size([520.0, 520.0]).show(ctx, |ui| {
                let is_text = matches!(app.first_selected().map(|i| &i.content), Some(Content::Text { .. }));
                ui.label(if is_text { "Pick a shape for the selected text; the words will follow it." } else { "Click a shape to put it on the page." });
                let fill = app.fill_color;
                let line = app.line_color;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for o in Outline::ALL {
                            let key = egui::Id::new(("shape-prev", o.label(), fill.0, fill.1, fill.2));
                            let tex: Option<TextureHandle> = ui.ctx().data(|d| d.get_temp(key)).or_else(|| {
                                let mut it = Item::new(0, Frame::new(0.0, 0.0, 0.7, 0.7), Content::Shape { outline: o });
                                it.fill = if fill.is_clear() { Rgba::hex("#2f8fd8") } else { fill };
                                it.stroke = line;
                                it.stroke_pt = 1.5;
                                let t = preview_of(app, ctx, &it, 100.0, format!("shape-{}", o.label()));
                                if let Some(t) = &t {
                                    ui.ctx().data_mut(|d| d.insert_temp(key, t.clone()));
                                }
                                t
                            });
                            let (r, resp) = ui.allocate_exact_size(egui::vec2(96.0, 104.0), egui::Sense::click());
                            let v = ui.visuals();
                            ui.painter().rect_filled(r, 8.0, if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
                            if let Some(t) = tex {
                                ui.painter().image(t.id(), egui::Rect::from_center_size(r.center_top() + egui::vec2(0.0, 42.0), egui::vec2(70.0, 70.0)), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                            }
                            ui.painter().text(r.center_bottom() - egui::vec2(0.0, 6.0), egui::Align2::CENTER_BOTTOM, o.label(), egui::FontId::proportional(11.5), v.text_color());
                            if resp.clicked() {
                                pick = Some((o, is_text));
                            }
                        }
                    });
                });
            });
            if let Some((o, is_text)) = pick {
                if is_text {
                    app.checkpoint();
                    let ids = app.selected.clone();
                    for it in app.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id)) {
                        if let Content::Text { shape, .. } = &mut it.content {
                            *shape = Some(o);
                            if it.fill.is_clear() {
                                it.fill = Rgba::WHITE;
                            }
                        }
                    }
                } else {
                    acts.push(Act::Shape(o));
                }
            }
        }
        Dialog::Borders(b) => borders(ctx, app, b, &mut open),
        Dialog::Signature(s) => {
            let mut add = false;
            window("Signature").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.label("Type the name to write:");
                ui.text_edit_singleline(&mut s.name);
                ui.horizontal(|ui| {
                    ui.label("Handwriting:");
                    egui::ComboBox::from_id_salt("sigfont").selected_text(s.family.clone()).width(200.0).show_ui(ui, |ui| {
                        let script_like = ["script", "hand", "brush", "signature", "pacifico", "satisfy", "allura", "vibes", "cursive", "caveat", "kalam", "sacramento", "parisienne", "tangerine", "z003", "chancery"];
                        for f in app.fonts.families() {
                            let l = f.to_lowercase();
                            if script_like.iter().any(|k| l.contains(k))
                                && ui.selectable_label(f == s.family, &f).clicked()
                            {
                                s.family = f;
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut s.size, 14.0..=72.0).text("Size"));
                    crate::dialogs::color_row(ui, &mut app.settings, &mut app.eyedropper, "Ink", &mut s.color);
                });
                ui.label(RichText::new("More handwriting fonts: Online > Online Fonts (try Great Vibes, Pacifico, Satisfy or Allura).").weak().size(12.0));
                let look = TextLook { family: s.family.clone(), size: s.size, color: s.color, ..Default::default() };
                let it = Item::new(0, Frame::new(0.0, 0.0, 4.0, s.size / 72.0 * 1.6 + 0.1), Content::Text { text: s.name.clone(), look, shape: None, fit: false, runs: Vec::new() });
                if let Some(t) = preview_of(app, ctx, &it, 90.0, "sig-preview".into()) {
                    egui::Frame::new().fill(Color32::WHITE).corner_radius(8.0).inner_margin(8.0).show(ui, |ui| {
                        ui.add(egui::Image::new(&t).max_width(380.0));
                    });
                }
                add = ui.add(egui::Button::new("Add to Page").min_size(egui::vec2(110.0, 28.0))).clicked();
            });
            if add {
                let look = TextLook { family: s.family.clone(), size: s.size, color: s.color, ..Default::default() };
                let h = s.size / 72.0 * 1.6 + 0.1;
                app.add_item(4.0, h, Content::Text { text: s.name.clone(), look, shape: None, fit: false, runs: Vec::new() });
                close = true;
            }
        }
        Dialog::Ai(a) => ai_window(ctx, app, a, &mut open),
        Dialog::PrintPro(p) => print_window(ctx, app, p, &mut open, &mut close),
        _ => return None,
    }
    Some(open && !close)
}

fn settings(ctx: &egui::Context, app: &mut App, open: &mut bool) {
    window("Settings").open(open).default_size([520.0, 560.0]).show(ctx, |ui| {
        let s = &mut app.settings;
        let mut changed = false;
        let mut refit = false;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Look");
            ui.horizontal(|ui| {
                ui.label("Theme:");
                changed |= ui.checkbox(&mut s.follow_system, "Same as my computer").changed();
                let mut look = s.look;
                changed |= ui.radio_value(&mut look, crate::theme::Look::Light, "Light").changed();
                changed |= ui.radio_value(&mut look, crate::theme::Look::Dark, "Dark").changed();
                if look != s.look {
                    s.follow_system = false;
                    s.look = look;
                    crate::theme::apply(ctx, look);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Layout:");
                changed |= ui.radio_value(&mut s.modern, false, "Classic (tool panel and toolbars)").changed();
                changed |= ui.radio_value(&mut s.modern, true, "Modern (side bar with big buttons)").changed();
            });
            changed |= ui.checkbox(&mut s.show_rulers, "Show rulers").changed();
            ui.add_space(8.0);
            ui.heading("Pages");
            let was = (s.legacy_pages, s.tile_pages);
            changed |= ui.checkbox(&mut s.legacy_pages, "Legacy Pages (one page at a time, page buttons along the bottom)").changed();
            ui.add_enabled_ui(!s.legacy_pages, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Show pages:");
                    changed |= ui.radio_value(&mut s.tile_pages, false, "In a list, one above the other").changed();
                    changed |= ui.radio_value(&mut s.tile_pages, true, "Side by side").changed();
                });
            });
            refit |= was != (s.legacy_pages, s.tile_pages);
            ui.add_space(8.0);
            ui.heading("Page guides");
            changed |= ui.checkbox(&mut s.show_printable, "Show the printable area (dotted line)").changed();
            ui.horizontal(|ui| {
                ui.label("Printer can't reach the outer");
                changed |= ui.add(egui::DragValue::new(&mut s.printable_margin).range(0.0..=1.0).speed(0.01).suffix(" in")).changed();
                ui.label("of the paper");
            });
            ui.label(RichText::new("Not sure? Print a test page from the Print window.").weak().size(12.0));
            ui.add_space(8.0);
            ui.heading("Snapping");
            changed |= ui.add(egui::Slider::new(&mut s.snap_strength, 0.0..=20.0).text("How strongly things line up (0 = off)")).changed();
            changed |= ui.checkbox(&mut s.snap_grid, "Also snap to ruler marks (1/16 inch)").changed();
            changed |= ui.checkbox(&mut s.show_guides, "Show guidelines (click a ruler to add one; they never print)").changed();
            ui.label(RichText::new("Hold Alt while dragging to move freely.").weak().size(12.0));
            ui.add_space(8.0);
            ui.heading("Safety");
            ui.horizontal(|ui| {
                ui.label("Save a safety copy every");
                changed |= ui.add(egui::DragValue::new(&mut s.autosave_minutes).range(1..=30)).changed();
                ui.label("minutes");
            });
            ui.horizontal(|ui| {
                ui.label("Undo can go back");
                changed |= ui.add(egui::DragValue::new(&mut s.undo_steps).range(crate::effects::UNDO_MINIMUM..=500)).changed();
                ui.label("steps");
            });
            ui.add_space(8.0);
            ui.heading("Pictures");
            ui.horizontal(|ui| {
                ui.label(format!("My Art folder: {}", crate::library::dir().display()));
            });
            if ui.button("Open My Art Folder").on_hover_text("Rename, delete or add pictures there").clicked() {
                crate::library::open_folder();
            }
            ui.add_space(8.0);
            ui.heading("AI pictures");
            ui.horizontal(|ui| {
                ui.label("Service:");
                changed |= ui.radio_value(&mut s.ai_provider, String::new(), "Google Gemini").changed();
                changed |= ui.radio_value(&mut s.ai_provider, "openai".to_string(), "OpenAI").changed();
            });
            ui.horizontal(|ui| {
                ui.label("Key:");
                changed |= ui.add(egui::TextEdit::singleline(&mut s.ai_key).password(true).desired_width(260.0)).changed();
                if ui.button("Remove").clicked() {
                    s.ai_key.clear();
                    changed = true;
                }
            });
        });
        if changed {
            app.settings.save();
        }
        if refit {
            app.fit_pending = true;
        }
    });
}

fn borders(ctx: &egui::Context, app: &mut App, b: &mut BorderState, open: &mut bool) {
    let mut add = false;
    window("Borders").open(open).default_size([620.0, 600.0]).show(ctx, |ui| {
        ui.label("Pick a border, then adjust its colors and thickness.");
        let mut redo = b.previews.len() != crate::raster::BORDER_NAMES.len();
        ui.horizontal(|ui| {
            redo |= crate::dialogs::color_row(ui, &mut app.settings, &mut app.eyedropper, "Main color", &mut b.line);
            redo |= crate::dialogs::color_row(ui, &mut app.settings, &mut app.eyedropper, "Second color", &mut b.fill);
        });
        redo |= ui.add(egui::Slider::new(&mut b.thickness, 6.0..=60.0).text("Thickness (points)")).changed();
        ui.add(egui::Slider::new(&mut b.inset, 0.0..=1.0).text("Distance from the paper edge (in)"));
        if redo {
            b.previews = crate::raster::BORDER_NAMES
                .iter()
                .enumerate()
                .map(|(k, n)| {
                    let mut it = Item::new(0, Frame::new(0.0, 0.0, 1.5, 1.9), Content::Border { style: k as u8 });
                    it.stroke = b.line;
                    it.fill = b.fill;
                    it.stroke_pt = b.thickness * 0.35;
                    preview_of(app, ctx, &it, 70.0, format!("border-{n}"))
                })
                .collect();
        }
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (k, n) in crate::raster::BORDER_NAMES.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(120.0, 160.0), egui::Sense::click());
                    let v = ui.visuals();
                    let on = b.style as usize == k;
                    ui.painter().rect_filled(r, 8.0, if on { v.selection.bg_fill.gamma_multiply(0.35) } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
                    if let Some(Some(t)) = b.previews.get(k) {
                        let img = egui::Rect::from_center_size(r.center_top() + egui::vec2(0.0, 68.0), egui::vec2(98.0, 124.0));
                        ui.painter().rect_filled(img, 2.0, Color32::WHITE);
                        ui.painter().image(t.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    ui.painter().text(r.center_bottom() - egui::vec2(0.0, 5.0), egui::Align2::CENTER_BOTTOM, *n, egui::FontId::proportional(11.5), v.text_color());
                    if resp.clicked() {
                        b.style = k as u8;
                    }
                    if resp.double_clicked() {
                        add = true;
                    }
                }
            });
        });
        ui.horizontal(|ui| {
            add |= ui.add(egui::Button::new("Add Border").min_size(egui::vec2(110.0, 28.0))).clicked();
            let has = app.first_selected().is_some_and(|i| matches!(i.content, Content::Border { .. }));
            if ui.add_enabled(has, egui::Button::new("Change Selected Border")).clicked() {
                app.checkpoint();
                let ids = app.selected.clone();
                let (st, line, fill, th) = (b.style, b.line, b.fill, b.thickness);
                for it in app.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id)) {
                    if let Content::Border { style } = &mut it.content {
                        *style = st;
                        it.stroke = line;
                        it.fill = fill;
                        it.stroke_pt = th;
                    }
                }
            }
        });
    });
    if add {
        app.checkpoint();
        let id = app.doc.fresh_id();
        let (w, h) = (app.doc.width, app.doc.height);
        let m = b.inset.min(w.min(h) / 3.0);
        let mut it = Item::new(id, Frame::new(m, m, w - 2.0 * m, h - 2.0 * m), Content::Border { style: b.style });
        it.stroke = b.line;
        it.fill = b.fill;
        it.stroke_pt = b.thickness;
        app.page_mut().items.insert(0, it);
        app.selected = vec![id];
    }
}

fn ai_window(ctx: &egui::Context, app: &mut App, a: &mut AiState, open: &mut bool) {
    if a.provider.is_empty() && a.key.is_empty() {
        a.provider = if app.settings.ai_provider == "openai" { "openai".into() } else { "gemini".into() };
        a.key = app.settings.ai_key.clone();
    }
    let mut result = None;
    if let Some(job) = &a.job
        && let Some(r) = job.lock().unwrap().take()
    {
        result = Some(r);
    }
    if let Some(r) = result {
        a.job = None;
        match r {
            Ok(bytes) => {
                let name: String = a.prompt.chars().take(40).collect();
                let _ = crate::library::save(&name, &bytes, "Made with AI");
                app.add_picture_bytes(&name, bytes, "Made with AI", ctx);
            }
            Err(e) => a.error = Some(e),
        }
    }
    window("AI Pictures").open(open).default_size([520.0, 420.0]).show(ctx, |ui| {
        ui.label("Describe the picture you'd like, and it will be drawn for you.");
        ui.horizontal(|ui| {
            ui.label("Service:");
            ui.radio_value(&mut a.provider, "gemini".to_string(), "Google Gemini");
            ui.radio_value(&mut a.provider, "openai".to_string(), "OpenAI (paid)");
        });
        let needs_key = true;
        if needs_key && a.key.trim().is_empty() {
            egui::Frame::new().fill(ui.visuals().widgets.inactive.weak_bg_fill).corner_radius(8.0).inner_margin(10.0).show(ui, |ui| {
                ui.label(RichText::new("One-time setup").strong());
                if a.provider == "openai" {
                    ui.label("1. Click the button to open OpenAI in your browser and sign in.\n2. Create a key and copy it.\n3. Paste it below. (OpenAI charges a few cents per picture.)");
                } else {
                    ui.label("1. Click the button to open Google AI Studio in your browser and sign in with your Google account.\n2. Click \"Create API key\" and copy it.\n3. Paste it below. It's free to start with.");
                }
                if ui.button("Open the sign-in page").clicked() {
                    crate::ai::open_in_browser(if a.provider == "openai" { crate::ai::OPENAI_KEY_PAGE } else { crate::ai::GEMINI_KEY_PAGE });
                }
            });
        }
        if needs_key {
            ui.horizontal(|ui| {
                ui.label("Key:");
                ui.add(egui::TextEdit::singleline(&mut a.key).password(true).desired_width(280.0));
                if ui.button("Paste").clicked()
                    && let Some(t) = crate::platform::clipboard_text()
                {
                    a.key = t.trim().to_string();
                }
            });
        }
        ui.add_space(6.0);
        ui.label("Your picture:");
        ui.add(egui::TextEdit::multiline(&mut a.prompt).hint_text("e.g. a watercolor of a red barn with sunflowers, white background").desired_rows(3).desired_width(f32::INFINITY));
        if let Some(e) = &a.error {
            ui.colored_label(Color32::from_rgb(200, 60, 60), e.as_str());
        }
        ui.horizontal(|ui| {
            let busy = a.job.is_some();
            if busy {
                ui.spinner();
                ui.label("Drawing… this can take up to a minute.");
            } else if ui.add_enabled(!a.prompt.trim().is_empty() && (!needs_key || !a.key.trim().is_empty()), egui::Button::new("Create Picture").min_size(egui::vec2(130.0, 28.0))).clicked() {
                app.settings.ai_provider = a.provider.clone();
                app.settings.ai_key = a.key.trim().to_string();
                app.settings.save();
                a.error = None;
                a.job = Some(crate::ai::generate(&a.provider, &a.key, &a.prompt, ctx));
            }
        });
        ui.label(RichText::new("Pictures are also saved to My Art. Your key stays on this computer and is only sent to the service you chose.").weak().size(12.0));
    });
}

fn print_window(ctx: &egui::Context, app: &mut App, p: &mut PrintState, open: &mut bool, close: &mut bool) {
    if p.preview_key.is_none() && app.doc.kind == crate::kinds::Kind::IronOn {
        p.opts.mirror = true;
    }
    let sheets = crate::export::arranged_sheets(&app.doc, &p.opts);
    p.opts.dpi = [150.0, 200.0, 300.0][p.quality as usize];
    p.opts.sheets = match p.which {
        1 => (0..sheets.len()).filter(|i| sheets[*i].places.iter().any(|pl| pl.panel == app.panel)).collect(),
        2 => parse_range(&p.range, sheets.len()),
        _ => Vec::new(),
    };
    // Rebuild the preview when anything that shows changes.
    let mut key = p.opts.clone();
    key.copies = 1;
    key.printer = None;
    key.dpi = 0.0;
    if p.preview_key.as_ref() != Some(&key) {
        let mut o = p.opts.clone();
        o.dpi = 60.0;
        p.pages = sheets
            .iter()
            .enumerate()
            .filter(|(i, _)| o.sheets.is_empty() || o.sheets.contains(i))
            .take(12)
            .filter_map(|(i, s)| {
                let pm = crate::export::finished_sheet(&app.doc, s, &app.fonts, &app.pics, &o)?;
                let rgba = crate::raster::pixmap_to_rgba(&pm);
                Some(ctx.load_texture(format!("print-sheet{i}"), egui::ColorImage::from_rgba_unmultiplied([pm.width() as usize, pm.height() as usize], &rgba), egui::TextureOptions::LINEAR))
            })
            .collect();
        p.at = p.at.min(p.pages.len().saturating_sub(1));
        p.preview_key = Some(key);
    }
    let mut go = false;
    let mut test = false;
    window("Print").open(open).default_size([900.0, 640.0]).show(ctx, |ui| {
        egui::Panel::left("print-options").exact_size(300.0).resizable(false).show(ui, |ui| {
            ui.heading(app.doc.kind.label());
            ui.label(RichText::new(format!("{} sheet{} of paper", sheets.len(), if sheets.len() == 1 { "" } else { "s" })).weak());
            ui.add_space(6.0);
            ui.label(RichText::new("Printer").strong());
            let current = p.opts.printer.clone().unwrap_or_else(|| "System default".into());
            egui::ComboBox::from_id_salt("printer").selected_text(current).width(260.0).show_ui(ui, |ui| {
                if ui.selectable_label(p.opts.printer.is_none(), "System default").clicked() {
                    p.opts.printer = None;
                }
                for pr in &p.printers {
                    if ui.selectable_label(p.opts.printer.as_deref() == Some(pr.as_str()), pr).clicked() {
                        p.opts.printer = Some(pr.clone());
                    }
                }
            });
            crate::widgets::stepper(ui, &mut p.opts.copies, 1, 99, "Copies:");
            ui.add_space(6.0);
            ui.label(RichText::new("What to print").strong());
            ui.radio_value(&mut p.which, 0, "Everything");
            ui.radio_value(&mut p.which, 1, "Just the page I'm on");
            ui.horizontal(|ui| {
                ui.radio_value(&mut p.which, 2, "Sheets:");
                ui.add_enabled(p.which == 2, egui::TextEdit::singleline(&mut p.range).hint_text("e.g. 1-2, 4").desired_width(100.0));
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Quality").strong());
            ui.horizontal(|ui| {
                ui.radio_value(&mut p.quality, 0, "Draft");
                ui.radio_value(&mut p.quality, 1, "Normal");
                ui.radio_value(&mut p.quality, 2, "Best");
            });
            ui.checkbox(&mut p.opts.gray, "Black and white (saves color ink)");
            let mut fit = p.opts.fit_margin.is_some();
            if ui.checkbox(&mut fit, "Shrink to fit the printable area").on_hover_text("Use if edges get cut off").changed() {
                p.opts.fit_margin = fit.then_some(app.settings.printable_margin);
            }
            if matches!(app.doc.kind, crate::kinds::Kind::Label | crate::kinds::Kind::BusinessCard | crate::kinds::Kind::CardQuarter | crate::kinds::Kind::CardHalf | crate::kinds::Kind::NoteCard) {
                ui.checkbox(&mut p.opts.cut_marks, "Add cut marks");
            }
            ui.checkbox(&mut p.opts.two_sided, "Print on both sides (if your printer can)");
            if p.opts.two_sided {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.radio_value(&mut p.opts.short_edge, false, "Flip on long edge");
                    ui.radio_value(&mut p.opts.short_edge, true, "Flip on short edge");
                });
                ui.label(RichText::new("Long edge: turn pages like a book. Short edge: flip up like a notepad (used for folded cards).").weak().size(12.0));
            }
            if matches!(app.doc.kind, crate::kinds::Kind::Label | crate::kinds::Kind::BusinessCard) {
                crate::widgets::stepper(ui, &mut p.opts.start_at, 1, 80, "Start at position:").then(|| ());
                ui.label(RichText::new("Use the rest of a partly used sheet: count across, then down.").weak().size(12.0));
            }
            egui::CollapsingHeader::new("More printing choices").show(ui, |ui| {
                ui.checkbox(&mut p.opts.mirror, "Mirror (for iron-on transfers)");
                ui.checkbox(&mut p.opts.reverse, "Print the last sheet first");
                ui.label("Line-up adjustment (if prints come out shifted):");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut p.opts.nudge.0).range(-0.5..=0.5).speed(0.01).prefix("Right ").suffix(" in"));
                    ui.add(egui::DragValue::new(&mut p.opts.nudge.1).range(-0.5..=0.5).speed(0.01).prefix("Down ").suffix(" in"));
                });
            });
            if matches!(app.doc.kind, crate::kinds::Kind::CardHalf | crate::kinds::Kind::NoteCard) && !p.opts.two_sided {
                ui.label(RichText::new("Tip: after the first sheet prints, put it back in blank side up for the inside.").weak().size(12.0));
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                go = ui.add(egui::Button::new(RichText::new("🖨 Print").size(16.0).strong()).min_size(egui::vec2(110.0, 34.0))).clicked();
                if ui.button("Cancel").clicked() {
                    *close = true;
                }
            });
            ui.add_space(6.0);
            if ui.button("Save as PDF Instead...").clicked() {
                match crate::platform::save_file("My Project.pdf", &[("PDF", &["pdf"])], |_| crate::export::pdf_bytes(&app.doc, &app.fonts, &app.pics, &p.opts)) {
                    Ok(Some(_)) => app.notify("Saved as PDF.", ctx),
                    Ok(None) => {}
                    Err(e) => app.notify(e, ctx),
                }
            }
            test = ui.button("Print a Test Page").on_hover_text("Shows how close to the edge your printer can print").clicked();
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.add_enabled(p.at > 0, egui::Button::new("◀")).clicked() {
                    p.at -= 1;
                }
                ui.label(format!("Sheet {} of {}", (p.at + 1).min(p.pages.len()), p.pages.len()));
                if ui.add_enabled(p.at + 1 < p.pages.len(), egui::Button::new("▶")).clicked() {
                    p.at += 1;
                }
                ui.separator();
                if ui.button("−").on_hover_text("Smaller").clicked() {
                    p.zoom = (p.zoom / 1.25).max(0.5);
                }
                ui.label(format!("{:.0}%", p.zoom * 100.0));
                if ui.button("+").on_hover_text("Bigger").clicked() {
                    p.zoom = (p.zoom * 1.25).min(4.0);
                }
                if ui.button("Fit").clicked() {
                    p.zoom = 1.0;
                }
            });
            // Fit the whole sheet in the space left (zoom 100% = fits).
            let thumbs = if p.pages.len() > 1 { 90.0 } else { 0.0 };
            let avail = ui.available_size() - egui::vec2(24.0, 30.0 + thumbs);
            egui::ScrollArea::both().max_height(avail.y + 24.0).show(ui, |ui| {
                if let Some(t) = p.pages.get(p.at) {
                    let s = t.size_vec2();
                    let k = (avail.x / s.x).min(avail.y / s.y).max(0.05) * p.zoom;
                    egui::Frame::new().fill(Color32::from_gray(150)).inner_margin(10.0).show(ui, |ui| {
                        let (r, _) = ui.allocate_exact_size(s * k, egui::Sense::hover());
                        ui.painter().rect_filled(r.translate(egui::vec2(3.0, 4.0)), 0.0, Color32::from_black_alpha(80));
                        ui.painter().image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                        // Printable area.
                        let m = app.settings.printable_margin * 60.0 * k;
                        if app.settings.show_printable && m > 0.0 {
                            ui.painter().rect_stroke(r.shrink(m), 0.0, egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 60, 60, 140)), egui::StrokeKind::Middle);
                        }
                    });
                }
            });
            if p.pages.len() > 1 {
                ui.horizontal_wrapped(|ui| {
                    for (i, t) in p.pages.iter().enumerate() {
                        let r = ui.add(egui::Button::image(egui::Image::new(t).max_height(70.0)).selected(i == p.at));
                        if r.clicked() {
                            p.at = i;
                        }
                    }
                });
            }
        });
    });
    if go {
        match crate::export::print_with(&app.doc, &app.fonts, &app.pics, &p.opts) {
            Ok(m) => app.notify(m, ctx),
            Err(e) => app.dialogs.push(Dialog::message("Printing didn't work", &e)),
        }
        *close = true;
    }
    if test {
        let t = crate::export::test_page();
        let mut o = p.opts.clone();
        o.sheets.clear();
        o.fit_margin = None;
        match crate::export::print_with(&t, &app.fonts, &app.pics, &o) {
            Ok(m) => app.notify(format!("Test page: {m}"), ctx),
            Err(e) => app.notify(e, ctx),
        }
    }
}

fn parse_range(s: &str, n: usize) -> Vec<usize> {
    let mut v = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if let Some((a, b)) = part.split_once('-') {
            if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
                v.extend((a.max(1)..=b.min(n)).map(|i| i - 1));
            }
        } else if let Ok(a) = part.parse::<usize>()
            && a >= 1
            && a <= n
        {
            v.push(a - 1);
        }
    }
    if v.is_empty() { (0..n).collect() } else { v }
}
