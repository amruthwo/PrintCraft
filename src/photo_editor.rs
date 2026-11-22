//! The Photo Editor window: crop and turn, color, light and focus, red eye,
//! effects and cut-outs, on a large preview with handles where they help.
//!
//! The editor works on a small copy of the photo so every change shows at
//! once; Apply repeats the same changes on a large copy.

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, pos2, vec2};
use image::DynamicImage;

use crate::app::App;
use crate::model::{Content, Outline};
use crate::photo::{Adjust, Combine, Effect, Mask};

const WORK_SIDE: u32 = 700;
const FINAL_SIDE: u32 = 3000;
const TILE_SIDE: u32 = 140;

/// The tools across the top. Their order matches `Act::Photo(n)`.
const TOOLS: [(&str, &str); 6] = [("✂", "Crop & Turn"), ("🎨", "Color"), ("☀", "Light & Focus"), ("👁", "Red Eye"), ("✨", "Effects"), ("⭐", "Cut Out")];

/// Shapes offered for cut-outs.
const CUT_SHAPES: [Outline; 8] = [Outline::Ellipse, Outline::Rounded, Outline::Heart, Outline::Star, Outline::Diamond, Outline::Hexagon, Outline::Cloud, Outline::Burst];

/// Crop shapes: label and width ÷ height (the long side follows the box).
const CROP_SHAPES: [(&str, Option<f32>); 5] = [("Free", None), ("Square", Some(1.0)), ("4 × 6", Some(1.5)), ("5 × 7", Some(1.4)), ("8 × 10", Some(1.25))];

/// A box being dragged on the picture: which handle (0–3 corners from the
/// top left going clockwise, 4–7 edges top, right, bottom, left, 8 the whole
/// box), the box when the drag began, and where the pointer was.
#[derive(Clone, Copy)]
struct BoxDrag {
    handle: u8,
    start: [f32; 4],
    from: Pos2,
}

type OverlayKey = (usize, Option<Outline>, [f32; 4], u32, u32);

pub struct PhotoState {
    pub item: u64,
    pub asset: u64,
    pub tab: u8,
    pub adjust: Adjust,
    /// Cut-out tool: 0 shape, 1 draw around it, 2 magic wand, 3 pick a color, 4 remove background.
    pub tool: u8,
    pub how: Combine,
    pub tolerance: f32,
    pub soft: f32,
    pub lasso: Vec<(f32, f32)>,
    src: Option<DynamicImage>,
    small: Option<DynamicImage>,
    /// The turned and cropped picture the selection tools work on.
    work: Option<(([f32; 4], u8), image::RgbaImage)>,
    shown: Option<(Adjust, TextureHandle)>,
    original: Option<TextureHandle>,
    overlay: Option<(OverlayKey, TextureHandle)>,
    effect_tiles: Option<(Adjust, Vec<TextureHandle>)>,
    history: Vec<Adjust>,
    pending: Option<Adjust>,
    drag: Option<BoxDrag>,
    /// Crop shape (see CROP_SHAPES), or free.
    crop_shape: usize,
    /// Cut Out: show the finished picture instead of the faded preview.
    finished_view: bool,
}

impl PhotoState {
    pub fn new(item: u64, asset: u64, tab: u8) -> Self {
        Self {
            item,
            asset,
            tab,
            adjust: Adjust::default(),
            tool: 2,
            how: Combine::New,
            tolerance: 18.0,
            soft: 1.0,
            lasso: Vec::new(),
            src: None,
            small: None,
            work: None,
            shown: None,
            original: None,
            overlay: None,
            effect_tiles: None,
            history: Vec::new(),
            pending: None,
            drag: None,
            crop_shape: 0,
            finished_view: false,
        }
    }
}

impl PhotoState {
    /// For checking the look (PRINTCRAFT_OPEN=photo:5:<tool>): start with a
    /// heart shape (tool 0) or a magic-wand pick on the red ball (tool 2).
    pub fn preset_for_check(&mut self, tool: u8) {
        self.tool = tool;
        match tool {
            0 => {
                self.adjust.shape = Some(Outline::Heart);
                self.adjust.shape_rect = [0.15, 0.3, 0.55, 0.95];
            }
            2 => {
                let w = crate::photo::load(&crate::sample::test_photo(), WORK_SIDE).map(|i| i.to_rgba8());
                if let Some(w) = w {
                    let sel = crate::photo::wand(&w, 200, 285, self.tolerance);
                    self.adjust.mask = Some(Arc::new(crate::photo::combine(None, w.width(), w.height(), &sel, Combine::New)));
                    self.how = Combine::Add;
                }
            }
            _ => {}
        }
    }
}

fn texture(ctx: &egui::Context, name: &str, img: &DynamicImage) -> TextureHandle {
    let rgba = img.to_rgba8();
    ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw()), egui::TextureOptions::LINEAR)
}

/// The largest rectangle of `size`'s shape that fits in `area`, centred.
fn fit(area: Rect, size: egui::Vec2) -> Rect {
    let k = (area.width() / size.x).min(area.height() / size.y).max(0.01);
    Rect::from_center_size(area.center(), size * k)
}

fn checkerboard(painter: &egui::Painter, r: Rect) {
    let cell = 10.0;
    let (mut y, mut row) = (r.top(), 0);
    while y < r.bottom() {
        let (mut x, mut col) = (r.left(), 0);
        while x < r.right() {
            let c = if (row + col) % 2 == 0 { Color32::from_gray(222) } else { Color32::from_gray(250) };
            painter.rect_filled(Rect::from_min_max(pos2(x, y), pos2((x + cell).min(r.right()), (y + cell).min(r.bottom()))), 0.0, c);
            x += cell;
            col += 1;
        }
        y += cell;
        row += 1;
    }
}

fn box_rect(r: Rect, b: [f32; 4]) -> Rect {
    Rect::from_min_max(pos2(r.left() + b[0] * r.width(), r.top() + b[1] * r.height()), pos2(r.left() + b[2] * r.width(), r.top() + b[3] * r.height()))
}

fn handle_points(b: Rect) -> [Pos2; 8] {
    [b.left_top(), b.right_top(), b.right_bottom(), b.left_bottom(), b.center_top(), b.right_center(), b.center_bottom(), b.left_center()]
}

/// A box over the picture: drag inside to move it, drag a corner or edge to
/// resize or stretch it (Shift keeps its shape). `aspect` (width ÷ height
/// in screen terms) fixes the shape. Returns true when the box changed.
fn box_editor(ui: &egui::Ui, resp: &egui::Response, r: Rect, bx: &mut [f32; 4], aspect: Option<f32>, drag: &mut Option<BoxDrag>, shape: Option<Outline>, dim: bool) -> bool {
    let screen = box_rect(r, *bx);
    let grab = 12.0;
    let corners_only = aspect.is_some();
    let handle_at = |p: Pos2| -> Option<u8> {
        let pts = handle_points(screen);
        (0..8u8).filter(|&i| !(corners_only && i >= 4)).find(|&i| pts[i as usize].distance(p) < grab).or_else(|| screen.contains(p).then_some(8))
    };
    if let Some(h) = resp.hover_pos().and_then(handle_at) {
        ui.ctx().set_cursor_icon(match h {
            0 | 2 => egui::CursorIcon::ResizeNwSe,
            1 | 3 => egui::CursorIcon::ResizeNeSw,
            4 | 6 => egui::CursorIcon::ResizeVertical,
            5 | 7 => egui::CursorIcon::ResizeHorizontal,
            _ => egui::CursorIcon::Grab,
        });
    }
    if resp.drag_started()
        && let Some(p) = resp.interact_pointer_pos()
        && let Some(h) = handle_at(p)
    {
        *drag = Some(BoxDrag { handle: h, start: *bx, from: p });
    }
    let mut changed = false;
    if let (Some(d), Some(p)) = (*drag, resp.interact_pointer_pos())
        && resp.dragged()
    {
        let (dx, dy) = ((p.x - d.from.x) / r.width(), (p.y - d.from.y) / r.height());
        let s = d.start;
        let mut n = s;
        const MIN: f32 = 0.04;
        match d.handle {
            8 => {
                let (w, h) = (s[2] - s[0], s[3] - s[1]);
                n[0] = (s[0] + dx).clamp(0.0, 1.0 - w);
                n[1] = (s[1] + dy).clamp(0.0, 1.0 - h);
                n[2] = n[0] + w;
                n[3] = n[1] + h;
            }
            h => {
                if matches!(h, 0 | 3 | 7) {
                    n[0] = (s[0] + dx).clamp(0.0, s[2] - MIN);
                }
                if matches!(h, 1 | 2 | 5) {
                    n[2] = (s[2] + dx).clamp(s[0] + MIN, 1.0);
                }
                if matches!(h, 0 | 1 | 4) {
                    n[1] = (s[1] + dy).clamp(0.0, s[3] - MIN);
                }
                if matches!(h, 2 | 3 | 6) {
                    n[3] = (s[3] + dy).clamp(s[1] + MIN, 1.0);
                }
                // Keep the shape: from the fixed crop shape, or the starting box with Shift.
                let keep = aspect.or_else(|| (ui.input(|i| i.modifiers.shift) && h < 4).then(|| ((s[2] - s[0]) * r.width()) / ((s[3] - s[1]) * r.height()).max(1e-3)));
                if let Some(ar) = keep
                    && h < 4
                {
                    let w = n[2] - n[0];
                    let mut hh = w * r.width() / ar / r.height();
                    // Too tall for the picture: shrink the width instead.
                    let room = if matches!(h, 0 | 1) { s[3] } else { 1.0 - s[1] };
                    let mut ww = w;
                    if hh > room {
                        hh = room;
                        ww = hh * r.height() * ar / r.width();
                    }
                    if matches!(h, 0 | 3) {
                        n[0] = n[2] - ww;
                    } else {
                        n[2] = n[0] + ww;
                    }
                    if matches!(h, 0 | 1) {
                        n[1] = s[3] - hh;
                        n[3] = s[3];
                    } else {
                        n[1] = s[1];
                        n[3] = s[1] + hh;
                    }
                }
            }
        }
        if n != *bx {
            *bx = n;
            changed = true;
        }
    }
    if resp.drag_stopped() {
        *drag = None;
    }
    // Draw: dim the outside, outline the box (and its shape), then the handles.
    let screen = box_rect(r, *bx);
    let painter = ui.painter();
    if dim {
        let veil = Color32::from_black_alpha(130);
        painter.rect_filled(Rect::from_min_max(r.left_top(), pos2(r.right(), screen.top())), 0.0, veil);
        painter.rect_filled(Rect::from_min_max(pos2(r.left(), screen.bottom()), r.right_bottom()), 0.0, veil);
        painter.rect_filled(Rect::from_min_max(pos2(r.left(), screen.top()), pos2(screen.left(), screen.bottom())), 0.0, veil);
        painter.rect_filled(Rect::from_min_max(pos2(screen.right(), screen.top()), pos2(r.right(), screen.bottom())), 0.0, veil);
        // Thirds, a help for placing things.
        for k in [1.0 / 3.0, 2.0 / 3.0] {
            let x = screen.left() + screen.width() * k;
            let y = screen.top() + screen.height() * k;
            painter.line_segment([pos2(x, screen.top()), pos2(x, screen.bottom())], Stroke::new(1.0, Color32::from_white_alpha(90)));
            painter.line_segment([pos2(screen.left(), y), pos2(screen.right(), y)], Stroke::new(1.0, Color32::from_white_alpha(90)));
        }
    }
    if let Some(o) = shape {
        let pts: Vec<Pos2> = crate::raster::outline_points(o, screen.width(), screen.height()).into_iter().map(|(x, y)| pos2(screen.left() + x, screen.top() + y)).collect();
        painter.add(egui::Shape::closed_line(pts.clone(), Stroke::new(3.0, Color32::from_black_alpha(140))));
        painter.add(egui::Shape::closed_line(pts, Stroke::new(1.5, Color32::WHITE)));
        painter.rect_stroke(screen, 0.0, Stroke::new(1.0, Color32::from_white_alpha(150)), egui::StrokeKind::Middle);
    } else {
        painter.rect_stroke(screen, 0.0, Stroke::new(3.0, Color32::from_black_alpha(140)), egui::StrokeKind::Middle);
        painter.rect_stroke(screen, 0.0, Stroke::new(1.5, Color32::WHITE), egui::StrokeKind::Middle);
    }
    for (i, p) in handle_points(screen).iter().enumerate() {
        if corners_only && i >= 4 {
            continue;
        }
        let h = Rect::from_center_size(*p, vec2(11.0, 11.0));
        painter.rect_filled(h, 2.0, Color32::WHITE);
        painter.rect_stroke(h, 2.0, Stroke::new(1.0, Color32::from_gray(60)), egui::StrokeKind::Inside);
    }
    changed
}

/// A big, friendly choice button for the side panel: an icon, a name and a
/// short line about it.
fn choice(ui: &mut egui::Ui, on: bool, icon: &str, title: &str, sub: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    let v = ui.visuals();
    let fill = if on { v.selection.bg_fill.gamma_multiply(0.3) } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill };
    ui.painter().rect_filled(r, 8.0, fill);
    if on {
        ui.painter().rect_stroke(r, 8.0, Stroke::new(1.5, v.selection.bg_fill), egui::StrokeKind::Inside);
    }
    ui.painter().text(r.left_center() + vec2(18.0, 0.0), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(18.0), v.text_color());
    ui.painter().text(r.left_top() + vec2(36.0, 6.0), egui::Align2::LEFT_TOP, title, egui::FontId::proportional(14.0), v.strong_text_color());
    ui.painter().text(r.left_top() + vec2(36.0, 25.0), egui::Align2::LEFT_TOP, sub, egui::FontId::proportional(11.5), v.weak_text_color());
    ui.add_space(3.0);
    resp.clicked()
}

fn heading(ui: &mut egui::Ui, title: &str, help: &str) {
    ui.label(RichText::new(title).size(17.0).strong());
    if !help.is_empty() {
        ui.label(RichText::new(help).weak().size(12.5));
    }
    ui.add_space(6.0);
}

/// Shows a shape's outline as a small tile for picking it.
fn shape_tile(ui: &mut egui::Ui, on: bool, shape: Option<Outline>, tip: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(52.0, 52.0), Sense::click());
    let v = ui.visuals();
    ui.painter().rect_filled(r, 8.0, if on { v.selection.bg_fill.gamma_multiply(0.3) } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
    if on {
        ui.painter().rect_stroke(r, 8.0, Stroke::new(1.5, v.selection.bg_fill), egui::StrokeKind::Inside);
    }
    let inner = r.shrink(10.0);
    match shape {
        Some(o) => {
            let pts: Vec<Pos2> = crate::raster::outline_points(o, inner.width(), inner.height()).into_iter().map(|(x, y)| pos2(inner.left() + x, inner.top() + y)).collect();
            ui.painter().add(egui::Shape::closed_line(pts, Stroke::new(1.8, v.text_color())));
        }
        None => {
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "None", egui::FontId::proportional(12.0), v.text_color());
        }
    }
    resp.on_hover_text(tip).clicked()
}

/// The faded look of what will be cut away: a checkerboard, mostly opaque,
/// over the cut-away parts; clear over what stays.
fn overlay_image(keep: &[u8], w: u32, h: u32) -> egui::ColorImage {
    let mut px = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let k = keep[(y * w + x) as usize];
            let g = if ((x / 8) + (y / 8)) % 2 == 0 { 222 } else { 252 };
            let a = ((255 - k) as f32 * 0.8) as u8;
            px.push(Color32::from_rgba_unmultiplied(g, g, g, a));
        }
    }
    egui::ColorImage { size: [w as usize, h as usize], pixels: px, source_size: vec2(w as f32, h as f32) }
}

pub fn show(ctx: &egui::Context, app: &mut App, p: &mut PhotoState, open: &mut bool, close: &mut bool) {
    // A small copy of the photo to work on, read once.
    if p.src.is_none() {
        let Some(asset) = app.doc.assets.get(&p.asset) else {
            *close = true;
            return;
        };
        p.src = crate::photo::load(&asset.bytes, WORK_SIDE);
        p.small = p.src.as_ref().map(|s| s.thumbnail(TILE_SIDE, TILE_SIDE));
        p.original = p.src.as_ref().map(|s| texture(ctx, "photo-original", s));
    }
    let Some(src) = p.src.clone() else {
        app.notify("This picture couldn't be opened in the Photo Editor.", ctx);
        *close = true;
        return;
    };
    // The cropped, turned picture the selection tools work on.
    let key = (p.adjust.crop, p.adjust.turns);
    if p.work.as_ref().is_none_or(|(k, _)| *k != key) {
        if p.work.is_some() {
            // The picture changed shape; an old selection no longer fits.
            p.adjust.mask = None;
            p.lasso.clear();
        }
        p.work = Some((key, crate::photo::turn_and_crop(&src, &p.adjust).to_rgba8()));
    }
    let before = p.adjust.clone();
    let mut apply = false;
    let mut picked: Option<(u32, u32, bool)> = None; // pixel, and "double click"
    let mut lasso_done = false;
    crate::dialogs::window("Photo Editor").open(open).default_size([1040.0, 700.0]).min_size([760.0, 520.0]).show(ctx, |ui| {
        // ---- the tools across the top
        ui.horizontal(|ui| {
            for (i, (icon, name)) in TOOLS.iter().enumerate() {
                let on = p.tab == i as u8;
                let text = RichText::new(format!("{icon}  {name}")).size(14.5);
                let b = egui::Button::new(if on { text.strong() } else { text }).min_size(vec2(118.0, 34.0)).corner_radius(17.0).selected(on);
                if ui.add(b).clicked() {
                    p.tab = i as u8;
                    p.drag = None;
                }
            }
        });
        ui.add_space(4.0);
        ui.separator();

        // ---- the bar under the picture
        let mut start_over = false;
        let mut undo = false;
        let mut comparing = false;
        egui::Panel::bottom("photo-actions").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                undo = ui.add_enabled(!p.history.is_empty(), egui::Button::new("↶ Undo").min_size(vec2(80.0, 30.0))).on_hover_text("Take back the last change (Ctrl+Z)").clicked();
                start_over = ui.add(egui::Button::new("Start Over").min_size(vec2(90.0, 30.0))).on_hover_text("Go back to the photo as it was").clicked();
                let cmp = ui.add(egui::Button::new("👁 Hold to Compare").min_size(vec2(140.0, 30.0)).sense(Sense::click_and_drag())).on_hover_text("Press and hold to see the photo as it was");
                comparing = cmp.is_pointer_button_down_on();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let ok = egui::Button::new(RichText::new("✔ Apply").size(15.0).strong().color(ui.visuals().selection.stroke.color)).fill(ui.visuals().selection.bg_fill).min_size(vec2(110.0, 32.0));
                    if ui.add(ok).on_hover_text("Keep these changes").clicked() {
                        apply = true;
                    }
                    if ui.add(egui::Button::new("Cancel").min_size(vec2(80.0, 32.0))).clicked() {
                        *close = true;
                    }
                });
            });
            ui.add_space(2.0);
        });
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            undo = true;
        }
        if undo && let Some(prev) = p.history.pop() {
            p.adjust = prev;
            p.pending = None;
        }
        if start_over && p.adjust != Adjust::default() {
            p.history.push(p.adjust.clone());
            p.adjust = Adjust::default();
            p.lasso.clear();
            p.crop_shape = 0;
        }

        // ---- the side panel for the current tool
        egui::Panel::left("photo-side").exact_size(300.0).resizable(false).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                side_panel(ui, ctx, p, &src);
            });
        });

        // ---- the picture
        egui::CentralPanel::default().show(ui, |ui| {
            if p.tab == 5 {
                ui.horizontal(|ui| {
                    ui.label("Show:");
                    ui.selectable_value(&mut p.finished_view, false, "What will be cut (faded)");
                    ui.selectable_value(&mut p.finished_view, true, "The finished picture");
                });
            }
            let area = ui.available_rect_before_wrap().shrink(12.0);
            let cutting = p.tab == 5 && !p.finished_view;
            // Which picture to show: the photo before cropping (crop tool),
            // before the cut-out (cut-out tool), or the finished result.
            let mut view = p.adjust.clone();
            if p.tab == 0 {
                view.crop = [0.0; 4];
            }
            if p.tab == 0 || cutting {
                view.mask = None;
                view.shape = None;
            }
            if p.shown.as_ref().is_none_or(|(a, _)| *a != view) {
                let img = crate::photo::apply_to(&src, &view);
                p.shown = Some((view, texture(ctx, "photo-view", &img)));
            }
            let tex = if comparing { p.original.clone() } else { p.shown.as_ref().map(|(_, t)| t.clone()) };
            let Some(tex) = tex else { return };
            let r = fit(area, tex.size_vec2());
            let sense = if comparing { Sense::hover() } else { Sense::click_and_drag() };
            let resp = ui.allocate_rect(r, sense);
            checkerboard(ui.painter(), r);
            ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            if comparing {
                ui.painter().text(r.left_top() + vec2(10.0, 10.0), egui::Align2::LEFT_TOP, "Before", egui::FontId::proportional(15.0), Color32::WHITE);
                return;
            }
            match p.tab {
                0 => {
                    // The crop box: what's kept.
                    let mut b = [p.adjust.crop[0], p.adjust.crop[1], 1.0 - p.adjust.crop[2], 1.0 - p.adjust.crop[3]];
                    let aspect = CROP_SHAPES[p.crop_shape].1.map(|k| if (b[2] - b[0]) * r.width() >= (b[3] - b[1]) * r.height() { k } else { 1.0 / k });
                    if box_editor(ui, &resp, r, &mut b, aspect, &mut p.drag, None, true) {
                        p.adjust.crop = [b[0], b[1], 1.0 - b[2], 1.0 - b[3]];
                    }
                }
                5 if cutting => {
                    // Faded checkerboard over what will be cut away.
                    if let Some((_, w)) = &p.work {
                        let (ww, wh) = (w.width(), w.height());
                        let okey: OverlayKey = (p.adjust.mask.as_ref().map(|m| Arc::as_ptr(m) as usize).unwrap_or(0), p.adjust.shape, p.adjust.shape_rect, ww, wh);
                        if p.adjust.mask.is_some() || p.adjust.shape.is_some() {
                            if p.overlay.as_ref().is_none_or(|(k, _)| *k != okey) {
                                let keep = crate::photo::keep_map(&p.adjust, ww, wh);
                                p.overlay = Some((okey, ctx.load_texture("photo-cut", overlay_image(&keep, ww, wh), egui::TextureOptions::LINEAR)));
                            }
                            if let Some((_, t)) = &p.overlay {
                                ui.painter().image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            }
                        }
                        match p.tool {
                            0 => {
                                if p.adjust.shape.is_some() {
                                    let mut b = p.adjust.shape_rect;
                                    if box_editor(ui, &resp, r, &mut b, None, &mut p.drag, p.adjust.shape, false) {
                                        p.adjust.shape_rect = b;
                                    }
                                }
                            }
                            1..=3 => {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                                let to_px = |q: Pos2| (((q.x - r.left()) / r.width() * ww as f32).clamp(0.0, ww as f32 - 1.0), ((q.y - r.top()) / r.height() * wh as f32).clamp(0.0, wh as f32 - 1.0));
                                if let Some(pos) = resp.interact_pointer_pos() {
                                    if resp.clicked() || resp.double_clicked() {
                                        let (x, y) = to_px(pos);
                                        picked = Some((x as u32, y as u32, resp.double_clicked()));
                                    } else if p.tool == 1 {
                                        // Drag around it: a freehand outline.
                                        let q = to_px(pos);
                                        if resp.drag_started() {
                                            p.lasso.clear();
                                            p.lasso.push(q);
                                        } else if resp.dragged() && p.lasso.last().is_none_or(|l| (l.0 - q.0).hypot(l.1 - q.1) > 3.0) {
                                            p.lasso.push(q);
                                        }
                                        if resp.drag_stopped() && p.lasso.len() >= 3 {
                                            lasso_done = true;
                                        }
                                    }
                                }
                                if p.tool == 1 && !p.lasso.is_empty() {
                                    let to_screen = |q: &(f32, f32)| pos2(r.left() + q.0 / ww as f32 * r.width(), r.top() + q.1 / wh as f32 * r.height());
                                    let mut pts: Vec<Pos2> = p.lasso.iter().map(to_screen).collect();
                                    if !resp.dragged()
                                        && let Some(h) = resp.hover_pos()
                                    {
                                        pts.push(h);
                                    }
                                    ui.painter().add(egui::Shape::line(pts.clone(), Stroke::new(3.0, Color32::WHITE)));
                                    ui.painter().add(egui::Shape::line(pts, Stroke::new(1.5, Color32::from_rgb(222, 64, 160))));
                                    ui.painter().circle(to_screen(&p.lasso[0]), 6.0, Color32::from_rgb(222, 64, 160), Stroke::new(1.5, Color32::WHITE));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        });
    });

    // Apply a click or a finished outline from the selection tools.
    if let Some((_, w)) = &p.work {
        let (ww, wh) = (w.width(), w.height());
        let sel = if lasso_done {
            let s = crate::photo::lasso(ww, wh, &p.lasso);
            p.lasso.clear();
            Some(s)
        } else if let Some((x, y, double)) = picked {
            match p.tool {
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
            }
        } else {
            None
        };
        if let Some(sel) = sel {
            let m = crate::photo::combine(p.adjust.mask.as_deref(), ww, wh, &sel, p.how);
            p.adjust.mask = Some(Arc::new(crate::photo::feather(&m, p.soft as u32)));
            if p.how == Combine::New {
                p.how = Combine::Add;
            }
        }
    }

    // One undo step per change (a whole slider drag is one step).
    if p.adjust != before && p.pending.is_none() {
        p.pending = Some(before);
    }
    if !ctx.input(|i| i.pointer.any_down())
        && let Some(prev) = p.pending.take()
        && prev != p.adjust
    {
        p.history.push(prev);
        if p.history.len() > 60 {
            p.history.remove(0);
        }
    }

    if apply {
        if p.adjust != Adjust::default()
            && let Some(asset) = app.doc.assets.get(&p.asset).cloned()
            && let Some(img) = crate::photo::apply(&asset.bytes, &p.adjust, FINAL_SIDE)
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

fn side_panel(ui: &mut egui::Ui, ctx: &egui::Context, p: &mut PhotoState, src: &DynamicImage) {
    use crate::widgets::slider;
    // Room for each slider's name beside it.
    ui.spacing_mut().slider_width = 130.0;
    let a = &mut p.adjust;
    match p.tab {
        0 => {
            heading(ui, "Crop & Turn", "Drag the corners or edges of the box on the photo to trim it. Drag inside the box to move it.");
            ui.label(RichText::new("Shape of the crop").strong());
            ui.horizontal_wrapped(|ui| {
                for (i, (name, ratio)) in CROP_SHAPES.iter().enumerate() {
                    if ui.selectable_label(p.crop_shape == i, *name).clicked() {
                        p.crop_shape = i;
                        if let Some(k) = ratio {
                            // The biggest box of that shape, centred, the way the photo lies.
                            let (iw, ih) = turned_size(src, a.turns);
                            let k = if iw >= ih { *k } else { 1.0 / *k };
                            let (mut bw, mut bh) = (iw, iw / k);
                            if bh > ih {
                                bh = ih;
                                bw = ih * k;
                            }
                            let (fw, fh) = (bw / iw, bh / ih);
                            a.crop = [(1.0 - fw) / 2.0, (1.0 - fh) / 2.0, (1.0 - fw) / 2.0, (1.0 - fh) / 2.0];
                        }
                    }
                }
            });
            ui.add_space(10.0);
            ui.label(RichText::new("Turn").strong());
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("⟲ Turn Left").min_size(vec2(118.0, 30.0))).clicked() {
                    a.turns = (a.turns + 3) % 4;
                    for _ in 0..3 {
                        a.crop = crate::photo::turn_crop_clockwise(a.crop);
                    }
                }
                if ui.add(egui::Button::new("⟳ Turn Right").min_size(vec2(118.0, 30.0))).clicked() {
                    a.turns = (a.turns + 1) % 4;
                    a.crop = crate::photo::turn_crop_clockwise(a.crop);
                }
            });
            ui.add_space(10.0);
            if ui.button("Don't crop").clicked() {
                a.crop = [0.0; 4];
                p.crop_shape = 0;
            }
        }
        1 => {
            heading(ui, "Color", "Scroll over a slider, or click it and use the arrow keys, for small steps.");
            slider(ui, &mut a.saturation, -1.0..=1.0, |s| s.text("Color strength"));
            slider(ui, &mut a.warmth, -1.0..=1.0, |s| s.text("Cool ↔ Warm"));
            ui.add_space(10.0);
            ui.label(RichText::new("Recolor").strong());
            ui.label(RichText::new("Turn every color around the color wheel: red becomes orange, then yellow, green, blue…").weak().size(12.0));
            slider(ui, &mut a.hue, -180.0..=180.0, |s| s.text("Shift colors").suffix("°"));
            for (k, n) in ["More red", "More green", "More blue"].iter().enumerate() {
                slider(ui, &mut a.rgb[k], -1.0..=1.0, |s| s.text(*n).show_value(false));
            }
            ui.horizontal_wrapped(|ui| {
                for (n, h) in [("Warmer", 20.0), ("Cooler", -20.0), ("Opposite", 180.0)] {
                    if ui.button(n).clicked() {
                        a.hue = h;
                    }
                }
            });
            ui.add_space(10.0);
            if ui.button("Reset Color").clicked() {
                (a.saturation, a.warmth, a.hue, a.rgb) = (0.0, 0.0, 0.0, [0.0; 3]);
            }
        }
        2 => {
            heading(ui, "Light & Focus", "Lighten a dark photo, add punch, or sharpen a soft one.");
            slider(ui, &mut a.brightness, -100..=100, |s| s.text("Brightness"));
            slider(ui, &mut a.contrast, -50.0..=50.0, |s| s.text("Contrast"));
            slider(ui, &mut a.sharpen, 0.0..=1.0, |s| s.text("Sharpen"));
            ui.add_space(10.0);
            if ui.button("Reset Light & Focus").clicked() {
                (a.brightness, a.contrast, a.sharpen) = (0, 0.0, 0.0);
            }
        }
        3 => {
            heading(ui, "Red Eye", "Turns the red glow in eyes from a camera flash back to a natural dark color.");
            ui.checkbox(&mut a.red_eye, RichText::new("Fix red eyes").size(15.0));
        }
        4 => {
            heading(ui, "Effects", "Click a look to try it on your photo.");
            // Small previews of each effect on this photo.
            let mut base = a.clone();
            base.effect = Effect::None;
            base.mask = None;
            base.shape = None;
            if p.effect_tiles.as_ref().is_none_or(|(k, _)| *k != base)
                && let Some(small) = &p.small
            {
                let tiles = Effect::ALL
                    .iter()
                    .map(|e| {
                        let mut t = base.clone();
                        t.effect = *e;
                        texture(ctx, &format!("fx-{e:?}"), &crate::photo::apply_to(small, &t))
                    })
                    .collect();
                p.effect_tiles = Some((base, tiles));
            }
            egui::Grid::new("effects").spacing([8.0, 8.0]).show(ui, |ui| {
                for (i, e) in Effect::ALL.iter().enumerate() {
                    let on = a.effect == *e;
                    let (r, resp) = ui.allocate_exact_size(vec2(132.0, 118.0), Sense::click());
                    let v = ui.visuals();
                    ui.painter().rect_filled(r, 8.0, if on { v.selection.bg_fill.gamma_multiply(0.3) } else if resp.hovered() { v.widgets.hovered.weak_bg_fill } else { v.widgets.inactive.weak_bg_fill });
                    if on {
                        ui.painter().rect_stroke(r, 8.0, Stroke::new(2.0, v.selection.bg_fill), egui::StrokeKind::Inside);
                    }
                    if let Some(t) = p.effect_tiles.as_ref().and_then(|(_, t)| t.get(i)) {
                        let img = fit(Rect::from_min_max(r.min + vec2(6.0, 6.0), pos2(r.right() - 6.0, r.bottom() - 24.0)), t.size_vec2());
                        ui.painter().image(t.id(), img, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    ui.painter().text(r.center_bottom() - vec2(0.0, 6.0), egui::Align2::CENTER_BOTTOM, e.label(), egui::FontId::proportional(12.5), v.text_color());
                    if resp.clicked() {
                        a.effect = *e;
                    }
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
        }
        _ => {
            heading(ui, "Cut Out", "Keep part of the photo and make the rest see-through. Faded, checkered areas will be cut away.");
            let tools: [(&str, &str, &str); 5] = [
                ("⭐", "A shape", "Heart, star, oval… move and stretch it"),
                ("✏", "Draw around it", "Drag around what you want to keep"),
                ("✨", "Magic wand", "Click areas of similar color"),
                ("💧", "Pick a color", "Every part in that color"),
                ("🖼", "Remove background", "One click, for a plain background"),
            ];
            for (k, (icon, title, sub)) in tools.iter().enumerate() {
                if choice(ui, p.tool == k as u8, icon, title, sub) {
                    p.tool = k as u8;
                    p.drag = None;
                }
            }
            ui.add_space(6.0);
            ui.separator();
            match p.tool {
                0 => {
                    ui.label(RichText::new("Pick a shape").strong());
                    ui.horizontal_wrapped(|ui| {
                        if shape_tile(ui, a.shape.is_none(), None, "No shape") {
                            a.shape = None;
                        }
                        for o in CUT_SHAPES {
                            if shape_tile(ui, a.shape == Some(o), Some(o), o.label()) {
                                a.shape = Some(o);
                            }
                        }
                    });
                    if a.shape.is_some() {
                        ui.label(RichText::new("On the photo, drag the shape to move it, and drag the white squares to make it bigger, smaller, wider or taller. Hold Shift to keep its shape.").weak().size(12.0));
                        if ui.button("Fit the shape to the whole photo").clicked() {
                            a.shape_rect = [0.0, 0.0, 1.0, 1.0];
                        }
                    }
                }
                4 => {
                    ui.label(RichText::new("PrintCraft looks at the edges of the photo and removes the background color that touches them.").weak().size(12.0));
                    slider(ui, &mut p.tolerance, 2.0..=60.0, |s| s.text("How much to remove"));
                    if ui.add(egui::Button::new("✨ Remove Background").min_size(vec2(200.0, 32.0))).clicked()
                        && let Some((_, w)) = &p.work
                    {
                        let keep = crate::photo::remove_background(w, p.tolerance);
                        let m = crate::photo::combine(None, w.width(), w.height(), &keep, Combine::New);
                        a.mask = Some(Arc::new(crate::photo::feather(&m, p.soft as u32)));
                    }
                }
                t => {
                    ui.label(RichText::new(match t {
                        1 => "Press and drag around what you want to keep, then let go. (Or click point by point, and double-click to finish.)",
                        2 => "Click an area: the wand picks the touching area of similar color. Keep clicking to add more.",
                        _ => "Click a color: every part of the photo in that color is picked.",
                    }).weak().size(12.0));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut p.how, Combine::New, "Start new").on_hover_text("Replace what's picked");
                        ui.selectable_value(&mut p.how, Combine::Add, "Add more").on_hover_text("Keep more of the photo");
                        ui.selectable_value(&mut p.how, Combine::Subtract, "Take away").on_hover_text("Cut away more of the photo");
                    });
                    if t != 1 {
                        slider(ui, &mut p.tolerance, 2.0..=60.0, |s| s.text("How similar"));
                    }
                }
            }
            if p.tool != 0 {
                ui.add_space(4.0);
                slider(ui, &mut p.soft, 0.0..=8.0, |s| s.text("Soft edges"));
                ui.horizontal(|ui| {
                    if ui.button("Swap kept and cut").on_hover_text("Keep what's cut away now, and cut away what's kept").clicked()
                        && let Some(m) = &a.mask
                    {
                        let mut n: Mask = (**m).clone();
                        for v in n.data.iter_mut() {
                            *v = 255 - *v;
                        }
                        a.mask = Some(Arc::new(n));
                    }
                    if ui.button("Clear").on_hover_text("Keep the whole photo again").clicked() {
                        a.mask = None;
                        p.lasso.clear();
                        p.how = Combine::New;
                    }
                });
            }
        }
    }
}

/// The photo's size after turning (in pixels of the working copy).
fn turned_size(src: &DynamicImage, turns: u8) -> (f32, f32) {
    let (w, h) = (src.width() as f32, src.height() as f32);
    if turns % 2 == 1 { (h, w) } else { (w, h) }
}
