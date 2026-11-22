//! The "Modern" layout: a slim rail of big labelled buttons on the left
//! that opens a drawer of choices, and one context bar that shows only
//! what fits what's selected.

use eframe::egui::{self, RichText, Sense, vec2};

use crate::app::{Act, App};
use crate::icons::{self, Icon};
use crate::model::*;
use crate::theme;

const RAIL: [(&str, Icon); 8] = [
    ("Design", Icon::Sparkle),
    ("Elements", Icon::Seal),
    ("Text", Icon::Letter),
    ("Photos", Icon::Picture),
    ("My Art", Icon::Cloud),
    ("Draw", Icon::Pencil),
    ("Effects", Icon::Wand),
    ("Print", Icon::Flag),
];

pub fn rail(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let pal = theme::palette(app.settings.look);
    for (i, (label, icon)) in RAIL.iter().enumerate() {
        let on = app.modern_drawer == Some(i);
        let (r, resp) = ui.allocate_exact_size(vec2(72.0, 64.0), Sense::click());
        if on || resp.hovered() {
            ui.painter().rect_filled(r, 12.0, if on { pal.section_open } else { pal.section.gamma_multiply(0.6) });
        }
        icons::paint(ui.painter(), egui::Rect::from_center_size(r.center() - vec2(0.0, 9.0), vec2(26.0, 26.0)), *icon, pal.text);
        ui.painter().text(r.center_bottom() - vec2(0.0, 8.0), egui::Align2::CENTER_BOTTOM, *label, egui::FontId::proportional(12.0), pal.text);
        if resp.clicked() {
            app.modern_drawer = if on { None } else { Some(i) };
        }
        ui.add_space(2.0);
    }
    None
}

fn tile(ui: &mut egui::Ui, label: &str, sub: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
    let v = ui.visuals();
    ui.painter().rect_filled(r, 10.0, if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
    ui.painter().text(r.left_top() + vec2(12.0, 7.0), egui::Align2::LEFT_TOP, label, egui::FontId::proportional(14.5), v.text_color());
    ui.painter().text(r.left_top() + vec2(12.0, 26.0), egui::Align2::LEFT_TOP, sub, egui::FontId::proportional(11.5), v.weak_text_color());
    ui.add_space(4.0);
    resp.clicked()
}

pub fn drawer(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    let Some(k) = app.modern_drawer else { return None };
    ui.heading(RAIL[k].0);
    ui.add_space(6.0);
    let mut t = |ui: &mut egui::Ui, l: &str, s: &str, a: Act| {
        if tile(ui, l, s) {
            out = Some(a);
        }
    };
    egui::ScrollArea::vertical().show(ui, |ui| match k {
        0 => {
            t(ui, "New design", "Card, sign, banner, label and more", Act::NewProject);
            t(ui, "New tab", "Another project side by side", Act::NewTab);
            t(ui, "Open", "A saved or older project", Act::Open);
            t(ui, "Page color", "Background color for this page", Act::Backdrop);
            t(ui, "Add a page", "Another page or panel", Act::AddPanel);
        }
        1 => {
            t(ui, "Shapes", "28 shapes, from stars to clouds", Act::MoreShapes);
            t(ui, "Borders", "16 page borders you can adjust", Act::Borders);
            t(ui, "Seal", "An award seal", Act::Seal);
            t(ui, "Number", "A number in a circle", Act::Number);
            t(ui, "Line", "A straight line", Act::Line);
        }
        2 => {
            t(ui, "Add a headline", "Big decorative title", Act::Headline);
            t(ui, "Add a text box", "Words you can type", Act::TextBox);
            t(ui, "Text in a shape", "Words that follow a shape", Act::TextInShape);
            t(ui, "Signature", "Your name in handwriting", Act::Signature);
            t(ui, "Initial capital", "A fancy first letter", Act::InitialCap);
            t(ui, "More fonts", "Free fonts online", Act::GetFonts);
        }
        3 => {
            t(ui, "Google Images", "Find photos and clip art", Act::Gallery(1));
            t(ui, "My content packs", "Art collections you own", Act::Gallery(0));
            t(ui, "From my computer", "A picture file", Act::PictureFromFile);
            t(ui, "Create with AI", "Describe it, get a picture", Act::AiPictures);
        }
        4 => {
            t(ui, "My Art", "Pictures you saved", Act::MyArt);
            t(ui, "Save selected picture", "Keep it for later", Act::SaveToMyArt);
            if ui.button("Open the My Art folder").clicked() {
                crate::library::open_folder();
            }
        }
        5 => {
            for o in [Outline::Rectangle, Outline::Ellipse, Outline::Star, Outline::Heart, Outline::Arrow, Outline::Cloud, Outline::Lightning] {
                t(ui, o.label(), "Click to add", Act::Shape(o));
            }
        }
        6 => {
            t(ui, "Shadow", "A shadow behind it", Act::Shadow);
            t(ui, "Glow", "A soft glow around it", Act::Glow);
            t(ui, "Frame", "An outline around it", Act::Frame);
            t(ui, "Photo Editor", "Crop, color, cut out, remove background", Act::Photo(0));
        }
        _ => {
            t(ui, "Print", "Printer, copies, quality", Act::Print);
            t(ui, "Preview", "See it before printing", Act::PrintPreview);
            t(ui, "Save as PDF", "To email or print elsewhere", Act::SavePdf);
            t(ui, "Save as picture", "PNG or JPEG", Act::ExportPicture);
        }
    });
    out
}

/// One bar: file basics, then only what fits the selection.
pub fn context_bar(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    ui.horizontal(|ui| {
        for (icon, tip, act) in [(Icon::Undo, "Undo", Act::Undo), (Icon::Redo, "Redo", Act::Redo), (Icon::Save, "Save", Act::Save), (Icon::Print, "Print", Act::Print)] {
            if icons::button(ui, icon, tip, app.enabled(&act)).clicked() {
                out = Some(act);
            }
        }
        ui.separator();
        let sel = app.first_selected().map(|i| i.content.clone());
        match sel {
            None => {
                ui.label(RichText::new("Click something on the page to change it, or pick from the side bar.").weak());
            }
            Some(c) => {
                if matches!(c, Content::Text { .. } | Content::Headline { .. }) || app.editing.is_some() {
                    if let Some(a) = crate::chrome::text_bar(ui, app) {
                        out = Some(a);
                    }
                    ui.separator();
                }
                if matches!(c, Content::Picture { .. }) && ui.button("Photo Editor").clicked() {
                    out = Some(Act::Photo(0));
                }
                if matches!(c, Content::Headline { .. }) && ui.button("Edit headline").clicked() {
                    out = Some(Act::Headline);
                }
                for (l, a) in [("Shadow", Act::Shadow), ("Group", Act::Group), ("Ungroup", Act::Ungroup), ("Front", Act::ToFront), ("Back", Act::ToBack), ("Delete", Act::Delete)] {
                    if app.enabled(&a) && ui.button(l).clicked() {
                        out = Some(a);
                    }
                }
            }
        }
    });
    out
}
