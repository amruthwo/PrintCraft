//! The page area: rulers, the page itself, items, selection handles and
//! mouse editing (move, resize, turn, double-click to type).

use std::hash::{Hash, Hasher};

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2, pos2, vec2};

use crate::app::{Act, App, Drag, ItemTexture};
use crate::model::*;
use crate::theme;

const RULER: f32 = 22.0;
const SCREEN_PPI: f32 = 96.0;
/// Drawn size of a resize handle (points).
const HANDLE: f32 = 10.0;
/// How close the pointer must be to grab a handle (points).
const GRAB: f32 = 13.0;

struct View {
    origin: Pos2,
    scale: f32, // screen points per inch
}

impl View {
    fn to_screen(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.origin.x + x * self.scale, self.origin.y + y * self.scale)
    }
    fn to_page(&self, p: Pos2) -> (f32, f32) {
        ((p.x - self.origin.x) / self.scale, (p.y - self.origin.y) / self.scale)
    }
}

fn rot(p: (f32, f32), c: (f32, f32), deg: f32) -> (f32, f32) {
    let (s, co) = deg.to_radians().sin_cos();
    let (dx, dy) = (p.0 - c.0, p.1 - c.1);
    (c.0 + dx * co - dy * s, c.1 + dx * s + dy * co)
}

/// Is the page point inside the item (taking its turn into account)?
fn hit(item: &Item, p: (f32, f32)) -> bool {
    let c = item.frame.center();
    let (x, y) = rot(p, c, -item.rotation);
    let slack = if matches!(item.content, Content::Line) { 0.08 } else { 0.0 };
    let f = item.frame;
    if let Content::Border { .. } = item.content {
        // Only the border band itself is clickable, so things inside it stay reachable.
        let band = 0.35;
        let inside = x >= f.x && x <= f.x + f.w && y >= f.y && y <= f.y + f.h;
        let core = x > f.x + band && x < f.x + f.w - band && y > f.y + band && y < f.y + f.h - band;
        return inside && !core;
    }
    x >= f.x - slack && x <= f.x + f.w + slack && y >= f.y - slack && y <= f.y + f.h + slack
}

fn handles(f: Frame) -> [(f32, f32); 8] {
    let (x0, y0, x1, y1) = (f.x, f.y, f.x + f.w, f.y + f.h);
    let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    [(x0, y0), (mx, y0), (x1, y0), (x1, my), (x1, y1), (mx, y1), (x0, y1), (x0, my)]
}

fn content_key(item: &Item, ppi: f32) -> u64 {
    let mut probe = item.clone();
    probe.frame.x = 0.0;
    probe.frame.y = 0.0;
    probe.rotation = 0.0;
    probe.flip_x = false;
    probe.flip_y = false;
    probe.id = 0;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    serde_json::to_string(&probe).unwrap_or_default().hash(&mut h);
    (ppi as u32).hash(&mut h);
    h.finish()
}

fn texture_for(app: &mut App, ctx: &egui::Context, page: usize, item: &Item, want_ppi: f32) -> Option<(egui::TextureId, f32, f32)> {
    // Render resolution in steps so zooming doesn't redraw every frame.
    let mut ppi = (want_ppi / 24.0).ceil().max(1.0) * 24.0;
    let longest = item.frame.w.max(item.frame.h).max(0.01);
    ppi = ppi.min(3000.0 / longest).max(8.0);
    let key = content_key(item, ppi);
    let at = slot(page, item.id);
    if let Some(t) = app.textures.get(&at)
        && t.key == key
    {
        return Some((t.tex.id(), t.pad, t.ppi));
    }
    let r = crate::raster::render_item(item, &app.doc, &app.fonts, &app.pics, ppi)?;
    let rgba = crate::raster::pixmap_to_rgba(&r.pixmap);
    let img = egui::ColorImage::from_rgba_unmultiplied([r.pixmap.width() as usize, r.pixmap.height() as usize], &rgba);
    let tex = ctx.load_texture(format!("item{page}-{}", item.id), img, egui::TextureOptions::LINEAR);
    let id = tex.id();
    app.textures.insert(at, ItemTexture { key, tex, pad: r.pad, ppi });
    Some((id, r.pad, ppi))
}

/// Room for a page's name above it, and the gap between pages (points).
const LABEL_H: f32 = 24.0;
const PAGE_GAP: f32 = 30.0;
/// The "Add a Page" button under the last page.
const ADD_H: f32 = 40.0;
/// Guideline color.
const GUIDE: Color32 = Color32::from_rgb(0, 166, 214);

/// Texture slot for an item on a page. Item ids are unique within a
/// project; the page number keeps older files that repeat ids apart.
fn slot(page: usize, id: u64) -> u64 {
    id ^ ((page as u64 + 1) << 44)
}

/// Where each page goes, before scrolling: (page, rect), the size of
/// everything together, and the Add a Page button.
fn lay_out(app: &App, area: Rect, scale: f32) -> (Vec<(usize, Rect)>, Vec2, Option<Rect>) {
    let (pw, ph) = (app.doc.width * scale, app.doc.height * scale);
    if app.settings.legacy_pages {
        return (vec![(app.panel, Rect::from_min_size(Pos2::ZERO, vec2(pw, ph)))], vec2(pw, ph), None);
    }
    let n = app.doc.panels.len();
    let cols = if app.settings.tile_pages { (((area.width() - 40.0 + PAGE_GAP) / (pw + PAGE_GAP)).floor() as usize).clamp(1, n) } else { 1 };
    let rows = n.div_ceil(cols);
    let cell = vec2(pw + PAGE_GAP, LABEL_H + ph + PAGE_GAP);
    let pages = (0..n).map(|i| (i, Rect::from_min_size(pos2((i % cols) as f32 * cell.x, (i / cols) as f32 * cell.y + LABEL_H), vec2(pw, ph)))).collect();
    let w = cols as f32 * cell.x - PAGE_GAP;
    let h = rows as f32 * cell.y;
    let bw = pw.clamp(160.0, 320.0_f32.max(pw * 0.5)).min(w.max(160.0));
    let add = Rect::from_min_size(pos2((w - bw) / 2.0, h), vec2(bw, ADD_H));
    (pages, vec2(w.max(bw), h + ADD_H + 16.0), Some(add))
}

/// Draw one page: paper, background, items, printable area, folds and
/// guidelines. `skip` is the item being typed into.
fn paint_page(app: &mut App, ctx: &egui::Context, painter: &egui::Painter, pg: usize, view: &View, pal: &theme::Palette) {
    let (pw, ph) = (app.doc.width, app.doc.height);
    let scale = view.scale;
    let page_rect = Rect::from_min_size(view.origin, vec2(pw * scale, ph * scale));
    for i in 1..=4 {
        let f = i as f32 * 2.0;
        painter.rect_filled(page_rect.translate(vec2(f * 0.6, f * 0.8)).expand(f * 0.3), 4.0, pal.page_shadow.gamma_multiply(0.35));
    }
    let (bg, pattern) = {
        let p = &app.doc.panels[pg];
        (p.background, p.pattern)
    };
    painter.rect_filled(page_rect, 0.0, Color32::from_rgba_unmultiplied(bg.0, bg.1, bg.2, 255));
    if pattern != 0 {
        // Patterned background, drawn by the same renderer the printer uses.
        let ppi = (scale * ctx.pixels_per_point()).min(2400.0 / pw.max(ph)).max(8.0);
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            let p = &app.doc.panels[pg];
            (p.pattern, p.pattern_color.0, p.pattern_color.1, p.pattern_color.2, bg.0, bg.1, bg.2, (ppi / 8.0) as u32, (pw * 100.0) as u32, (ph * 100.0) as u32).hash(&mut h);
            h.finish()
        };
        let bslot = u64::MAX - 7 - pg as u64;
        if app.textures.get(&bslot).is_none_or(|t| t.key != key)
            && let Some(pm) = crate::raster::background_image(&app.doc.panels[pg], pw, ph, ppi)
        {
            let rgba = crate::raster::pixmap_to_rgba(&pm);
            let tex = ctx.load_texture(format!("page-bg{pg}"), egui::ColorImage::from_rgba_unmultiplied([pm.width() as usize, pm.height() as usize], &rgba), egui::TextureOptions::LINEAR);
            app.textures.insert(bslot, ItemTexture { key, tex, pad: 0.0, ppi });
        }
        if let Some(t) = app.textures.get(&bslot) {
            painter.image(t.tex.id(), page_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
    }

    let ppp = ctx.pixels_per_point();
    let typing = if pg == app.panel { app.editing.as_ref().map(|(id, _)| *id) } else { None };
    let items = app.doc.panels[pg].items.clone();
    for item in &items {
        if typing == Some(item.id) {
            continue;
        }
        if let Content::Text { text, .. } = &item.content
            && text.is_empty()
        {
            let r = Rect::from_min_size(view.to_screen(item.frame.x, item.frame.y), vec2(item.frame.w * scale, item.frame.h * scale));
            painter.rect_stroke(r, 2.0, Stroke::new(1.0, pal.muted.gamma_multiply(0.6)), egui::StrokeKind::Inside);
            painter.text(r.left_top() + vec2(6.0, 4.0), egui::Align2::LEFT_TOP, "Type your text here", egui::FontId::proportional(13.0), pal.muted);
        }
        let Some((tex, pad, ppi)) = texture_for(app, ctx, pg, item, scale * ppp) else { continue };
        let k = scale / ppi; // texture px -> screen points
        let tl = view.to_screen(item.frame.x, item.frame.y) - vec2(pad * k, pad * k);
        let size = vec2(item.frame.w * scale + 2.0 * pad * k, item.frame.h * scale + 2.0 * pad * k);
        let rect = Rect::from_min_size(tl, size);
        let mut uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        if item.flip_x {
            std::mem::swap(&mut uv.min.x, &mut uv.max.x);
        }
        if item.flip_y {
            std::mem::swap(&mut uv.min.y, &mut uv.max.y);
        }
        let mut mesh = egui::Mesh::with_texture(tex);
        mesh.add_rect_with_uv(rect, uv, Color32::WHITE.gamma_multiply(item.opacity.clamp(0.0, 1.0)));
        if item.rotation.abs() > 0.01 {
            let c = view.to_screen(item.frame.center().0, item.frame.center().1);
            mesh.rotate(egui::emath::Rot2::from_angle(item.rotation.to_radians()), c);
        }
        painter.add(egui::Shape::mesh(mesh));
    }

    // Printable area: most printers can't reach the very edge of the paper.
    let m = app.settings.printable_margin;
    if app.settings.show_printable && m > 0.0 {
        let r = Rect::from_min_max(view.to_screen(m, m), view.to_screen(pw - m, ph - m));
        let dash = |a: Pos2, b: Pos2| {
            painter.extend(egui::Shape::dashed_line(&[a, b], Stroke::new(1.0, Color32::from_rgba_unmultiplied(214, 70, 90, 150)), 5.0, 4.0));
        };
        dash(r.left_top(), r.right_top());
        dash(r.right_top(), r.right_bottom());
        dash(r.right_bottom(), r.left_bottom());
        dash(r.left_bottom(), r.left_top());
    }
    // Fold lines (brochures) and sheet joins (banners).
    let fold = Stroke::new(1.2, Color32::from_rgba_unmultiplied(80, 120, 200, 170));
    let vline = |x: f32, label: &str| {
        let (a, b) = (view.to_screen(x, 0.0), view.to_screen(x, ph));
        painter.extend(egui::Shape::dashed_line(&[a, b], fold, 8.0, 5.0));
        painter.text(a + vec2(4.0, 4.0), egui::Align2::LEFT_TOP, label, egui::FontId::proportional(11.0), fold.color);
    };
    match app.doc.kind {
        crate::kinds::Kind::Brochure => {
            vline(pw / 3.0, "fold");
            vline(pw * 2.0 / 3.0, "fold");
        }
        crate::kinds::Kind::Banner => {
            if let Some(b) = &app.doc.banner {
                let (seg, _) = Project::banner_segment(b, crate::model::BANNER_MARGIN);
                let mut x = seg;
                while x < pw - 0.01 {
                    vline(x, "trim & tape");
                    x += seg;
                }
            }
        }
        _ => {}
    }
    // Guidelines placed from the rulers (screen only, never printed).
    if app.settings.show_guides {
        let over = 10.0;
        for (i, g) in app.doc.panels[pg].guides.iter().enumerate() {
            let selected = app.guide_sel == Some((pg, i));
            let dragged = app.guide_drag.filter(|d| d.page == pg && d.index == i);
            let s = view.to_screen(g.at, g.at);
            let (a, b) = if g.vertical { (pos2(s.x, page_rect.top() - over), pos2(s.x, page_rect.bottom() + over)) } else { (pos2(page_rect.left() - over, s.y), pos2(page_rect.right() + over, s.y)) };
            if dragged.is_some_and(|d| d.removing) {
                painter.extend(egui::Shape::dashed_line(&[a, b], Stroke::new(1.0, Color32::from_rgba_unmultiplied(214, 70, 90, 160)), 6.0, 4.0));
            } else {
                let w = if selected || dragged.is_some() { 1.8 } else { 1.0 };
                painter.line_segment([a, b], Stroke::new(w, if selected { GUIDE } else { GUIDE.gamma_multiply(0.85) }));
            }
        }
    }
}

pub fn show(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    let ctx = ui.ctx().clone();
    let pal = theme::palette(app.settings.look);
    let full = ui.available_rect_before_wrap();
    let ruler = if app.settings.show_rulers { RULER } else { 0.0 };
    let area = Rect::from_min_max(full.min + vec2(ruler, ruler), full.max);
    ui.painter().rect_filled(full, 0.0, pal.workspace);
    app.panel = app.panel.min(app.doc.panels.len() - 1);
    let legacy = app.settings.legacy_pages;

    let (pw, ph) = (app.doc.width, app.doc.height);
    if app.fit_pending && area.width() > 50.0 {
        let label = if legacy { 0.0 } else { LABEL_H };
        let (w, h) = (area.width(), area.height());
        let n = app.doc.panels.len();
        let one = ((w - 60.0) / (pw * SCREEN_PPI)).min((h - 60.0 - label) / (ph * SCREEN_PPI));
        let s = if legacy || n == 1 {
            one
        } else if app.settings.tile_pages {
            // As many pages as fit, side by side, all in view if possible.
            (1..=n)
                .map(|c| {
                    let rows = n.div_ceil(c) as f32;
                    let c = c as f32;
                    ((w - 40.0 - (c - 1.0) * PAGE_GAP) / (c * pw * SCREEN_PPI)).min((h - 40.0 - rows * LABEL_H - (rows - 1.0) * PAGE_GAP) / (rows * ph * SCREEN_PPI))
                })
                .fold(0.0, f32::max)
                .max(one * 0.25)
        } else {
            // One page, with the top of the next one peeking out below.
            ((w - 60.0) / (pw * SCREEN_PPI)).min((h - 2.0 * LABEL_H - PAGE_GAP - 70.0) / (ph * SCREEN_PPI))
        };
        app.zoom = s.clamp(0.05, 6.0);
        app.pan = Vec2::ZERO;
        app.fit_pending = false;
        if !legacy {
            app.scroll_to = Some(app.panel);
        }
    }
    let scale = SCREEN_PPI * app.zoom;

    // Lay the pages out. Every-page view: pages hang from the top of the
    // workspace and scroll; Legacy Pages: the one page sits in the middle.
    let (mut placed, size, mut add_rect) = lay_out(app, area, scale);
    let base_of = |pan: Vec2| if legacy { area.center() - size / 2.0 + pan } else { pos2(area.center().x - size.x / 2.0 + pan.x, area.top() + pan.y) };
    {
        // Don't let the pages scroll out of reach.
        let b = base_of(app.pan);
        let keep = 80.0;
        let mut d = Vec2::ZERO;
        if b.y + size.y < area.top() + keep {
            d.y = area.top() + keep - (b.y + size.y);
        } else if b.y > area.bottom() - keep {
            d.y = area.bottom() - keep - b.y;
        }
        if b.x + size.x < area.left() + keep {
            d.x = area.left() + keep - (b.x + size.x);
        } else if b.x > area.right() - keep {
            d.x = area.right() - keep - b.x;
        }
        app.pan += d;
    }
    let mut shift = base_of(app.pan).to_vec2();
    let active_rect = |placed: &[(usize, Rect)], panel: usize| placed.iter().find(|p| p.0 == panel).or(placed.first()).map(|p| p.1).unwrap_or(Rect::NOTHING);
    // Bring a page into view, or keep the spot under the pointer still while zooming.
    let memo = egui::Id::new("canvas-memo");
    let prev: Option<(f32, Pos2, usize, usize)> = ctx.data(|d| d.get_temp(memo));
    let mut extra = Vec2::ZERO;
    if let Some(i) = app.scroll_to.take() {
        let r = active_rect(&placed, i).translate(shift);
        if !legacy {
            let single = app.doc.panels.len() == 1;
            let mut want = r.min;
            // Everything fits: show it all, centred. Otherwise bring the page
            // to the top (or the middle, for a one-page project).
            want.x = if size.x <= area.width() - 20.0 { r.min.x - app.pan.x } else { area.center().x - r.width() / 2.0 };
            want.y = if size.y <= area.height() - 20.0 {
                r.min.y + (area.center().y - size.y / 2.0) - (area.top() + app.pan.y)
            } else if single && r.height() + LABEL_H <= area.height() - 24.0 {
                area.center().y + LABEL_H / 2.0 - r.height() / 2.0
            } else {
                area.top() + 12.0 + LABEL_H
            };
            extra = want - r.min;
        }
    } else if let Some((ps, po, pp, pt)) = prev
        && (ps - scale).abs() > 0.001
        && pp == app.panel
        && pt == app.active
    {
        let anchor = ctx.input(|i| i.pointer.hover_pos()).filter(|p| area.contains(*p)).unwrap_or(area.center());
        let r = active_rect(&placed, app.panel).translate(shift);
        let u = (anchor - po) / ps;
        extra = (anchor - u * scale) - r.min;
    }
    app.pan += extra;
    shift += extra;
    for p in placed.iter_mut() {
        p.1 = p.1.translate(shift);
    }
    if let Some(r) = add_rect.as_mut() {
        *r = r.translate(shift);
    }

    let resp = ui.interact(area, ui.scope_id().with("canvas"), Sense::click_and_drag());
    let painter = ui.painter_at(area);

    // A press on another page makes it the page being worked on.
    let page_at = |p: Pos2| placed.iter().find(|(_, r)| r.expand2(vec2(PAGE_GAP / 2.0, (PAGE_GAP + LABEL_H) / 2.0)).contains(p)).map(|x| x.0);
    if !legacy && app.drag.is_none() && app.guide_drag.is_none() {
        let (any, pos) = ctx.input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos()));
        if any
            && let Some(p) = pos
            && area.contains(p)
            && resp.hovered()
            && let Some(pg) = page_at(p)
            && pg != app.panel
        {
            app.commit_edit();
            app.panel = pg;
            app.selected.clear();
            app.guide_sel = None;
        }
    }

    for (pg, r) in placed.clone() {
        if r.expand(24.0).intersects(area) {
            let v = View { origin: r.min, scale };
            paint_page(app, &ctx, &painter, pg, &v, &pal);
        }
    }
    let page_rect = active_rect(&placed, app.panel);
    let view = View { origin: page_rect.min, scale };

    // Page names above each page, with the page menu; Add a Page below.
    if !legacy {
        let many = app.doc.panels.len() > 1;
        for (pg, r) in placed.clone() {
            let label = Rect::from_min_max(pos2(r.left(), r.top() - LABEL_H), pos2(r.right(), r.top() - 3.0));
            if !label.intersects(area) {
                continue;
            }
            let active = pg == app.panel;
            let name = app.doc.panels[pg].name.clone();
            let lr = ui.interact(label.intersect(area), ui.scope_id().with(("page-label", pg)), Sense::click());
            let color = if active && many { pal.accent } else if lr.hovered() { pal.text } else { pal.muted };
            let font = egui::FontId::proportional(13.0);
            painter.text(label.left_center(), egui::Align2::LEFT_CENTER, &name, font.clone(), color);
            if many {
                let n = format!("{} of {}", pg + 1, app.doc.panels.len());
                painter.text(label.right_center() - vec2(30.0, 0.0), egui::Align2::RIGHT_CENTER, n, egui::FontId::proportional(11.0), pal.muted);
            }
            if lr.clicked() && !active {
                out = Some(Act::Panel(pg));
            }
            lr.on_hover_text("Click to work on this page. Right-click for page choices.").context_menu(|ui| crate::chrome::page_menu(ui, app, pg));
            let dots = Rect::from_min_max(pos2(label.right() - 24.0, label.top()), label.right_bottom());
            if dots.intersects(area) {
                ui.scope_builder(egui::UiBuilder::new().max_rect(dots), |ui| {
                    ui.spacing_mut().button_padding = vec2(4.0, 0.0);
                    ui.menu_button("⋯", |ui| crate::chrome::page_menu(ui, app, pg)).response.on_hover_text("Page choices");
                });
            }
        }
        if let Some(r) = add_rect
            && r.intersects(area)
        {
            let ar = ui.interact(r, ui.scope_id().with("add-page"), Sense::click());
            let c = if ar.hovered() { pal.accent } else { pal.muted };
            let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
            painter.extend(egui::Shape::dashed_line(&pts, Stroke::new(1.2, c), 6.0, 4.0));
            painter.text(r.center(), egui::Align2::CENTER_CENTER, "+  Add a Page", egui::FontId::proportional(14.0), c);
            if ar.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                out = Some(Act::AddPanel);
            }
        }
    }

    // Snap guides while dragging.
    if app.drag.is_some() {
        for (vertical, at) in &app.guides {
            let (a, b) = if *vertical { (view.to_screen(*at, -0.3), view.to_screen(*at, ph + 0.3)) } else { (view.to_screen(-0.3, *at), view.to_screen(pw + 0.3, *at)) };
            painter.line_segment([a, b], Stroke::new(1.2, Color32::from_rgb(222, 64, 160)));
        }
    }

    let items = app.page().items.clone();
    // Selection outlines and handles.
    let sel: Vec<Item> = items.iter().filter(|i| app.selected.contains(&i.id)).cloned().collect();
    for it in &sel {
        let c = it.frame.center();
        let corners = [(it.frame.x, it.frame.y), (it.frame.x + it.frame.w, it.frame.y), (it.frame.x + it.frame.w, it.frame.y + it.frame.h), (it.frame.x, it.frame.y + it.frame.h)];
        let pts: Vec<Pos2> = corners.iter().map(|p| rot(*p, c, it.rotation)).map(|(x, y)| view.to_screen(x, y)).collect();
        painter.add(egui::Shape::closed_line(pts, Stroke::new(1.5, pal.selection)));
        if sel.len() == 1 && !it.locked {
            for (x, y) in handles(it.frame) {
                let (x, y) = rot((x, y), c, it.rotation);
                let p = view.to_screen(x, y);
                painter.rect(Rect::from_center_size(p, vec2(HANDLE, HANDLE)), 2.0, Color32::WHITE, Stroke::new(1.5, pal.selection), egui::StrokeKind::Middle);
            }
            let top = rot((c.0, it.frame.y), c, it.rotation);
            let knob = rotate_knob(&view, it);
            painter.line_segment([view.to_screen(top.0, top.1), knob], Stroke::new(1.0, pal.selection));
            painter.circle(knob, 7.0, pal.selection, Stroke::new(2.0, Color32::WHITE));
            painter.text(knob, egui::Align2::CENTER_CENTER, "⟳", egui::FontId::proportional(10.0), Color32::WHITE);
            if let Some(b) = bulge_handle(&view, it) {
                painter.circle(b, 7.0, Color32::from_rgb(76, 182, 140), Stroke::new(2.0, Color32::WHITE));
                painter.text(b, egui::Align2::CENTER_CENTER, "↔", egui::FontId::proportional(9.0), Color32::WHITE);
            }
            if let Some(w) = warp_handle(&view, it) {
                let pts = vec![w + vec2(0.0, -8.0), w + vec2(8.0, 0.0), w + vec2(0.0, 8.0), w + vec2(-8.0, 0.0)];
                painter.add(egui::Shape::convex_polygon(pts, Color32::from_rgb(222, 64, 160), Stroke::new(1.5, Color32::WHITE)));
            }
        }
        if it.locked {
            let p = view.to_screen(it.frame.x, it.frame.y);
            painter.text(p + vec2(4.0, 4.0), egui::Align2::LEFT_TOP, "🔒", egui::FontId::proportional(14.0), pal.selection);
        }
    }
    if let Some(Drag::Marquee { origin }) = &app.drag
        && ctx.input(|i| i.pointer.primary_down())
        && let Some(p) = ctx.input(|i| i.pointer.interact_pos())
    {
        let a = view.to_screen(origin.0, origin.1);
        painter.rect(Rect::from_two_pos(a, p), 0.0, pal.selection.gamma_multiply(0.1), Stroke::new(1.0, pal.selection), egui::StrokeKind::Middle);
    }

    // Rulers, measured from the page being worked on, and their guidelines.
    if app.settings.show_rulers {
        let at = resp.hover_pos();
        rulers(ui, full, &view, app, &pal, page_rect, at);
        if let Some(a) = guide_rulers(ui, app, full, &view, &pal) {
            out = Some(a);
        }
    }
    ctx.data_mut(|d| d.insert_temp(memo, (scale, page_rect.min, app.panel, app.active)));

    // Mouse: press -> move -> release, using where the button went down.
    let hover = resp.hover_pos();
    app.pointer_in = hover
        .and_then(|p| {
            let r = page_at(p).and_then(|pg| placed.iter().find(|x| x.0 == pg)).map(|x| x.1).unwrap_or(page_rect);
            View { origin: r.min, scale }.to_page(p).into()
        })
        .filter(|(x, y): &(f32, f32)| *x >= 0.0 && *y >= 0.0 && *x <= pw && *y <= ph);
    let (pressed, sec_pressed, released, down, origin, now) = ctx.input(|i| {
        (i.pointer.primary_pressed(), i.pointer.secondary_pressed(), i.pointer.primary_released(), i.pointer.primary_down(), i.pointer.press_origin(), i.pointer.interact_pos())
    });
    let over_canvas = |p: Pos2| area.contains(p) && resp.hovered();
    if pressed
        && let Some(p) = origin
        && over_canvas(p)
    {
        press(app, &view, p, &items, false, &ctx);
    }
    if sec_pressed
        && let Some(p) = now
        && over_canvas(p)
    {
        press(app, &view, p, &items, true, &ctx);
    }
    if down
        && let Some(p) = now
        && app.drag.is_some()
    {
        drag_to(app, &view, p, &ctx);
    }
    if released || (!down && app.drag.is_some()) {
        if let Some(Drag::Marquee { origin }) = &app.drag {
            if let Some(p) = now {
                let (a, b) = (*origin, view.to_page(p));
                let (x0, y0, x1, y1) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
                if (x1 - x0) * view.scale > 4.0 || (y1 - y0) * view.scale > 4.0 {
                    app.selected = items
                        .iter()
                        .filter(|i| !i.locked && i.frame.x < x1 && i.frame.y < y1 && i.frame.x + i.frame.w > x0 && i.frame.y + i.frame.h > y0 && !matches!(i.content, Content::Border { .. }))
                        .map(|i| i.id)
                        .collect();
                }
            }
            app.drag = None;
        } else {
            app.end_drag();
        }
    }
    // Cursor: show what a press here would do.
    let cursor = match &app.drag {
        Some(Drag::Move { .. }) => Some(egui::CursorIcon::Grabbing),
        Some(Drag::Rotate { .. }) => Some(egui::CursorIcon::Grabbing),
        Some(Drag::Resize { handle, id, .. }) => app.item(*id).map(|it| resize_cursor(*handle, it.rotation)),
        Some(Drag::Marquee { .. }) => Some(egui::CursorIcon::Crosshair),
        Some(Drag::Warp { .. }) => Some(egui::CursorIcon::ResizeVertical),
        Some(Drag::Bulge { .. }) => Some(egui::CursorIcon::ResizeHorizontal),
        None => hover.and_then(|p| {
            if sel.len() == 1
                && let Some(h) = grab_handle(&view, &sel[0], p)
            {
                if h == 8 {
                    app.hint = "Drag to turn it. Hold Alt for any angle.".into();
                    return Some(egui::CursorIcon::Alias);
                }
                app.hint = "Drag to resize. Hold Shift to keep its shape.".into();
                return Some(resize_cursor(h, sel[0].rotation));
            }
            items.iter().rev().find(|i| hit(i, view.to_page(p))).filter(|i| !i.locked).map(|_| egui::CursorIcon::Grab)
        }),
    };
    let turning = matches!(app.drag, Some(Drag::Rotate { .. })) || cursor == Some(egui::CursorIcon::Alias);
    if app.eyedropper.is_some() && hover.is_some() {
        ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        app.hint = "Click anything on the page to pick its color. Press Esc to stop.".into();
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.eyedropper = None;
        }
    } else if turning && let Some(p) = ctx.input(|i| i.pointer.hover_pos()) {
        // A drawn "turn" cursor: a circular arrow.
        ctx.set_cursor_icon(egui::CursorIcon::None);
        let fg = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("turn-cursor")));
        let pts: Vec<Pos2> = (0..=20).map(|i| {
            let a = (i as f32 / 20.0) * 1.6 * std::f32::consts::PI - 0.3;
            p + vec2(9.0 * a.cos(), 9.0 * a.sin())
        }).collect();
        let tip = pts[pts.len() - 1];
        fg.add(egui::Shape::line(pts.clone(), Stroke::new(4.0, Color32::WHITE)));
        fg.add(egui::Shape::line(pts, Stroke::new(2.0, Color32::BLACK)));
        fg.add(egui::Shape::convex_polygon(vec![tip + vec2(-5.0, -4.0), tip + vec2(5.0, -3.0), tip + vec2(0.0, 5.0)], Color32::BLACK, Stroke::new(1.0, Color32::WHITE)));
    } else if let Some(c) = cursor {
        ctx.set_cursor_icon(c);
    }
    if resp.double_clicked()
        && let Some(p) = resp.interact_pointer_pos()
        && let Some(it) = items.iter().rev().find(|i| hit(i, view.to_page(p)))
    {
        match &it.content {
            Content::Text { text, .. } => {
                app.checkpoint();
                app.editing = Some((it.id, text.clone()));
                app.edit_sel = None;
            }
            Content::Headline { .. } => out = Some(Act::Headline),
            Content::Picture { .. } => out = Some(Act::Photo(0)),
            _ => {}
        }
    }
    if resp.hovered() {
        let (scroll, zoom_mod) = ctx.input(|i| (i.smooth_scroll_delta, i.modifiers.command));
        if zoom_mod && scroll.y.abs() > 0.0 {
            app.zoom = (app.zoom * (1.0 + scroll.y * 0.002)).clamp(0.05, 6.0);
        } else if scroll != Vec2::ZERO {
            app.pan += scroll;
        }
    }

    // Right-click menu for the item under the pointer.
    resp.context_menu(|ui| {
        let pick = |ui: &mut egui::Ui, out: &mut Option<Act>, l: &str, a: Act| {
            if ui.add_enabled(app.enabled(&a), egui::Button::new(l)).clicked() {
                *out = Some(a);
                ui.close();
            }
        };
        if app.selected.is_empty() {
            pick(ui, &mut out, "Paste", Act::Paste);
            pick(ui, &mut out, "Insert Text Box", Act::TextBox);
            pick(ui, &mut out, "Insert Headline...", Act::Headline);
            pick(ui, &mut out, "Insert Art...", Act::Gallery(0));
            pick(ui, &mut out, "Page Color...", Act::BackgroundColor);
            return;
        }
        pick(ui, &mut out, "Cut", Act::Cut);
        pick(ui, &mut out, "Copy", Act::Copy);
        pick(ui, &mut out, "Duplicate", Act::Duplicate);
        pick(ui, &mut out, "Delete", Act::Delete);
        ui.separator();
        ui.menu_button("Position", |ui| {
            if ui.button("Center on Page").clicked() {
                out = Some(Act::CenterPage);
            }
            if ui.button("Center Horizontally").clicked() {
                out = Some(Act::CenterH);
            }
            if ui.button("Center Vertically").clicked() {
                out = Some(Act::CenterV);
            }
        });
        ui.menu_button("Size", |ui| {
            if ui.button("Full Page").clicked() {
                out = Some(Act::FullPage);
            }
        });
        ui.menu_button("Flip", |ui| {
            if ui.button("Horizontal").clicked() {
                out = Some(Act::FlipH);
            }
            if ui.button("Vertical").clicked() {
                out = Some(Act::FlipV);
            }
        });
        ui.menu_button("Layer", |ui| {
            for (l, a) in [("Bring to Front", Act::ToFront), ("Send to Back", Act::ToBack), ("Forward One Layer", Act::Forward), ("Back One Layer", Act::Backward)] {
                if ui.button(l).clicked() {
                    out = Some(a);
                }
            }
        });
        ui.separator();
        pick(ui, &mut out, "Apply Drop Shadow...", Act::Shadow);
        pick(ui, &mut out, "Apply Glow...", Act::Glow);
        pick(ui, &mut out, "Apply Frame...", Act::Frame);
        match app.first_selected().map(|i| &i.content) {
            Some(Content::Text { .. }) => {
                pick(ui, &mut out, "Text Box Shape...", Act::TextShape);
                pick(ui, &mut out, "Apply Speech Bubble", Act::SpeechBubble);
                pick(ui, &mut out, "Apply Stretch Text", Act::StretchText);
            }
            Some(Content::Headline { .. }) => pick(ui, &mut out, "Edit Headline...", Act::Headline),
            Some(Content::Picture { .. }) => pick(ui, &mut out, "Photo Editor...", Act::Photo(0)),
            _ => {}
        }
        if app.enabled(&Act::Group) {
            pick(ui, &mut out, "Group", Act::Group);
        }
        if app.enabled(&Act::Ungroup) {
            pick(ui, &mut out, "Ungroup", Act::Ungroup);
        }
        pick(ui, &mut out, "Lock / Unlock", Act::Lock);
    });

    // Inline text editor: shows the same fonts, sizes, colors and spans
    // the page will print, and remembers the selection so the toolbar can
    // style just the selected words.
    if let Some((id, mut text)) = app.editing.clone() {
        if let Some(it) = app.item(id).cloned() {
            let r = Rect::from_min_size(view.to_screen(it.frame.x, it.frame.y), vec2(it.frame.w * scale, it.frame.h * scale));
            let look = it.look().cloned().unwrap_or_default();
            let runs = match &it.content {
                Content::Text { runs, .. } => runs.clone(),
                _ => Vec::new(),
            };
            let fams: Vec<egui::FontFamily> = [(false, false), (true, false), (false, true), (true, true)].iter().map(|(b, i)| app.fonts.ensure(&ctx, &look.family, *b, *i)).collect();
            painter.rect_filled(r, 2.0, Color32::from_white_alpha(240));
            let to_pts = scale / 72.0;
            let align = match look.align {
                Align::Center => egui::Align2::CENTER_TOP,
                Align::Right => egui::Align2::RIGHT_TOP,
                _ => egui::Align2::LEFT_TOP,
            };
            let look2 = look.clone();
            let mut layouter = move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
                let s = buf.as_str();
                let chars: Vec<char> = s.chars().collect();
                let styles = char_styles(&look2, &runs, chars.len());
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = wrap;
                job.halign = match look2.align {
                    Align::Center => egui::Align::Center,
                    Align::Right => egui::Align::Max,
                    _ => egui::Align::Min,
                };
                let mut byte = 0;
                let mut k = 0;
                while k < chars.len() {
                    let st = styles[k];
                    let start = byte;
                    while k < chars.len() && styles[k] == st {
                        byte += chars[k].len_utf8();
                        k += 1;
                    }
                    let fam = fams[(st.bold as usize) | ((st.italic as usize) << 1)].clone();
                    let c = Color32::from_rgba_unmultiplied(st.color.0, st.color.1, st.color.2, 255);
                    job.append(&s[start..byte], 0.0, egui::TextFormat {
                        font_id: egui::FontId::new((st.size * to_pts).max(4.0), fam),
                        color: c,
                        underline: if st.underline { Stroke::new(1.0, c) } else { Stroke::NONE },
                        ..Default::default()
                    });
                }
                if chars.is_empty() {
                    job.append("", 0.0, egui::TextFormat { font_id: egui::FontId::new((look2.size * to_pts).max(4.0), fams[0].clone()), ..Default::default() });
                }
                ui.fonts_mut(|f| f.layout_job(job))
            };
            let edit = egui::TextEdit::multiline(&mut text)
                .layouter(&mut layouter)
                .hint_text("Type your text here")
                .align(align)
                .frame(egui::Frame::NONE)
                .background_color(Color32::TRANSPARENT)
                .desired_width(r.width());
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r));
            let out_edit = edit.show(&mut child);
            let er = out_edit.response.response.clone();
            if !er.has_focus() && !er.lost_focus() {
                er.request_focus();
            }
            if let Some(cr) = out_edit.cursor_range {
                let (a, b) = (cr.primary.index.min(cr.secondary.index), cr.primary.index.max(cr.secondary.index));
                if a != b {
                    app.edit_sel = Some((a.into(), b.into()));
                } else if er.has_focus() {
                    app.edit_sel = None;
                }
            }
            // Keep the item's text (and its spans) in step as you type.
            if let Some(Content::Text { text: t, runs, .. }) = app.item(id).map(|i| i.content.clone())
                && t != text
            {
                let mut runs = runs;
                shift_runs(&mut runs, &t, &text);
                if let Some(Content::Text { text: tt, runs: rr, .. }) = app.item_mut(id).map(|i| &mut i.content) {
                    *tt = text.clone();
                    *rr = runs;
                }
                app.dirty = true;
            }
            if let Some((_, t)) = app.editing.as_mut() {
                *t = text;
            }
            let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            // Only clicks on the page area end editing, so the toolbar can be used.
            let clicked_away = ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| area.contains(p) && !r.contains(p));
            if escape || clicked_away {
                app.commit_edit();
            }
        } else {
            app.editing = None;
        }
    }
    out
}

/// Resize cursor for a handle (0 = top-left, clockwise), allowing for turn.
fn resize_cursor(handle: usize, rotation: f32) -> egui::CursorIcon {
    let steps = ((rotation / 45.0).round() as i32).rem_euclid(8) as usize;
    match (handle + steps) % 4 {
        0 => egui::CursorIcon::ResizeNwSe,
        1 => egui::CursorIcon::ResizeVertical,
        2 => egui::CursorIcon::ResizeNeSw,
        _ => egui::CursorIcon::ResizeHorizontal,
    }
}

fn rotate_knob(view: &View, it: &Item) -> Pos2 {
    let c = it.frame.center();
    let above = 30.0 / view.scale;
    let (x, y) = rot((c.0, it.frame.y - above), c, it.rotation);
    view.to_screen(x, y)
}

/// Which handle (0..8, or 8 for the turn knob) is under the pointer.
fn grab_handle(view: &View, it: &Item, p: Pos2) -> Option<usize> {
    if it.locked {
        return None;
    }
    if rotate_knob(view, it).distance(p) < GRAB + 2.0 {
        return Some(8);
    }
    let c = it.frame.center();
    handles(it.frame).iter().position(|h| {
        let (x, y) = rot(*h, c, it.rotation);
        view.to_screen(x, y).distance(p) < GRAB
    })
}

/// The round handle on a headline's right side that sets its bulge.
fn bulge_handle(view: &View, it: &Item) -> Option<Pos2> {
    let Content::Headline { bulge, .. } = &it.content else { return None };
    if it.locked {
        return None;
    }
    let c = it.frame.center();
    let (x, y) = rot((it.frame.x + it.frame.w + 0.25 * view_inches(view, 18.0) + bulge * 0.1, c.1), c, it.rotation);
    Some(view.to_screen(x, y))
}

fn view_inches(view: &View, points: f32) -> f32 {
    points / view.scale
}

/// The diamond that bends an arched, wavy or bulging headline.
fn warp_handle(view: &View, it: &Item) -> Option<Pos2> {
    let Content::Headline { warp, bend, .. } = &it.content else { return None };
    if *warp == Warp::Straight || it.locked {
        return None;
    }
    let c = it.frame.center();
    let (x, y) = rot((c.0, it.frame.y + it.frame.h * (0.5 - 0.11 * bend)), c, it.rotation);
    Some(view.to_screen(x, y))
}

fn press(app: &mut App, view: &View, p: Pos2, items: &[Item], secondary: bool, ctx: &egui::Context) {
    app.guide_sel = None;
    if app.editing.is_some() {
        app.commit_edit();
    }
    let at = view.to_page(p);
    if let Some(mode) = app.eyedropper.take() {
        let ppi = 60.0;
        if let Some(pm) = crate::raster::render_panel(&app.doc, app.panel, &app.fonts, &app.pics, ppi) {
            let (x, y) = ((at.0 * ppi) as i64, (at.1 * ppi) as i64);
            if x >= 0 && y >= 0 && (x as u32) < pm.width() && (y as u32) < pm.height() {
                let i = (y as usize * pm.width() as usize + x as usize) * 4;
                let d = pm.data();
                let c = Rgba(d[i], d[i + 1], d[i + 2], 255);
                app.settings.remember_color(c);
                match mode {
                    0 => crate::chrome::apply_color(app, true, c),
                    1 => crate::chrome::apply_color(app, false, c),
                    _ => {}
                }
                app.notify(format!("Picked {}. It's in Recently Used.", crate::colors::hex(c)), ctx);
            }
        }
        return;
    }
    if !secondary && app.selected.len() == 1 {
        let id = app.selected[0];
        if let Some(it) = items.iter().find(|i| i.id == id)
            && let Some(b) = bulge_handle(view, it)
            && b.distance(p) < GRAB
            && let Content::Headline { bulge, .. } = &it.content
        {
            app.drag_snapshot = Some(app.doc.clone());
            app.drag = Some(Drag::Bulge { id, start: *bulge, origin: at });
            return;
        }
        if let Some(it) = items.iter().find(|i| i.id == id)
            && let Some(w) = warp_handle(view, it)
            && w.distance(p) < GRAB
            && let Content::Headline { bend, .. } = &it.content
        {
            app.drag_snapshot = Some(app.doc.clone());
            app.drag = Some(Drag::Warp { id, start: *bend, origin: at });
            return;
        }
    }
    let additive = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
    if !secondary && app.selected.len() == 1 {
        let id = app.selected[0];
        if let Some(it) = items.iter().find(|i| i.id == id)
            && let Some(h) = grab_handle(view, it, p)
        {
            app.drag_snapshot = Some(app.doc.clone());
            app.drag = Some(if h == 8 { Drag::Rotate { id, start: it.rotation } } else { Drag::Resize { id, handle: h, start: it.frame } });
            return;
        }
    }
    match items.iter().rev().find(|i| hit(i, at)) {
        Some(it) => {
            if additive && !secondary {
                if let Some(pos) = app.selected.iter().position(|s| *s == it.id) {
                    app.selected.remove(pos);
                } else {
                    app.selected.push(it.id);
                }
            } else if !app.selected.contains(&it.id) {
                app.selected = vec![it.id];
            }
            if !secondary && !it.locked {
                app.begin_move(at);
            }
        }
        None => {
            if !secondary {
                app.selected.clear();
                app.drag = Some(Drag::Marquee { origin: at });
            } else {
                app.selected.clear();
            }
        }
    }
}

/// Lines things can snap to: page edges and middle, printable margin and the
/// edges and middles of other items. (vertical?, position)
fn snap_targets(app: &App, skip: &[u64]) -> Vec<(bool, f32)> {
    let (w, h) = (app.doc.width, app.doc.height);
    let m = app.settings.printable_margin;
    let mut t = vec![(true, 0.0), (true, w / 2.0), (true, w), (false, 0.0), (false, h / 2.0), (false, h)];
    if app.settings.show_printable && m > 0.0 {
        t.extend([(true, m), (true, w - m), (false, m), (false, h - m)]);
    }
    if app.settings.show_guides {
        t.extend(app.page().guides.iter().map(|g| (g.vertical, g.at)));
    }
    for it in app.page().items.iter().filter(|i| !skip.contains(&i.id)) {
        let f = it.frame;
        t.extend([(true, f.x), (true, f.x + f.w / 2.0), (true, f.x + f.w), (false, f.y), (false, f.y + f.h / 2.0), (false, f.y + f.h)]);
    }
    t
}

/// Best pull for a set of edges along one axis: (shift, guide position).
fn pull(edges: &[f32], targets: &[(bool, f32)], vertical: bool, reach: f32) -> Option<(f32, f32)> {
    let mut best: Option<(f32, f32)> = None;
    for e in edges {
        for (v, t) in targets {
            if *v != vertical {
                continue;
            }
            let d = t - e;
            if d.abs() <= reach && best.is_none_or(|(b, _)| d.abs() < b.abs()) {
                best = Some((d, *t));
            }
        }
    }
    best
}

fn drag_to(app: &mut App, view: &View, p: Pos2, ctx: &egui::Context) {
    let at = view.to_page(p);
    let free = ctx.input(|i| i.modifiers.alt);
    let reach = if free { 0.0 } else { app.settings.snap_strength / view.scale };
    let grid = app.settings.snap_grid && !free;
    let snap8 = |v: f32| if grid { (v * 16.0).round() / 16.0 } else { v };
    app.guides.clear();
    match app.drag.as_ref() {
        Some(Drag::Move { start, origin }) => {
            let (mut dx, mut dy) = (snap8(at.0 - origin.0), snap8(at.1 - origin.1));
            let start = start.clone();
            if reach > 0.0 && !start.is_empty() {
                let ids: Vec<u64> = start.iter().map(|(id, _)| *id).collect();
                let x0 = start.iter().map(|(_, f)| f.x).fold(f32::MAX, f32::min) + dx;
                let y0 = start.iter().map(|(_, f)| f.y).fold(f32::MAX, f32::min) + dy;
                let x1 = start.iter().map(|(_, f)| f.x + f.w).fold(f32::MIN, f32::max) + dx;
                let y1 = start.iter().map(|(_, f)| f.y + f.h).fold(f32::MIN, f32::max) + dy;
                let targets = snap_targets(app, &ids);
                if let Some((d, g)) = pull(&[x0, (x0 + x1) / 2.0, x1], &targets, true, reach) {
                    dx += d;
                    app.guides.push((true, g));
                }
                if let Some((d, g)) = pull(&[y0, (y0 + y1) / 2.0, y1], &targets, false, reach) {
                    dy += d;
                    app.guides.push((false, g));
                }
            }
            for (id, f) in start {
                if let Some(it) = app.item_mut(id) {
                    it.frame.x = f.x + dx;
                    it.frame.y = f.y + dy;
                }
            }
        }
        Some(Drag::Resize { id, handle, start }) => {
            let (id, handle, start) = (*id, *handle, *start);
            let keep_shape = ctx.input(|i| i.modifiers.shift);
            let targets = if reach > 0.0 { snap_targets(app, &[id]) } else { Vec::new() };
            let mut guides = Vec::new();
            if let Some(it) = app.item_mut(id) {
                let c = start.center();
                // Work in the item's own (unturned) frame.
                let (mut lx, mut ly) = rot(at, c, -it.rotation);
                if it.rotation.abs() < 0.01 {
                    if let Some((d, g)) = pull(&[lx], &targets, true, reach) {
                        lx += d;
                        guides.push((true, g));
                    }
                    if let Some((d, g)) = pull(&[ly], &targets, false, reach) {
                        ly += d;
                        guides.push((false, g));
                    }
                }
                let (mut x0, mut y0, mut x1, mut y1) = (start.x, start.y, start.x + start.w, start.y + start.h);
                let min = 0.1;
                match handle {
                    0 => (x0, y0) = (lx.min(x1 - min), ly.min(y1 - min)),
                    1 => y0 = ly.min(y1 - min),
                    2 => (x1, y0) = (lx.max(x0 + min), ly.min(y1 - min)),
                    3 => x1 = lx.max(x0 + min),
                    4 => (x1, y1) = (lx.max(x0 + min), ly.max(y0 + min)),
                    5 => y1 = ly.max(y0 + min),
                    6 => (x0, y1) = (lx.min(x1 - min), ly.max(y0 + min)),
                    _ => x0 = lx.min(x1 - min),
                }
                let is_line = matches!(it.content, Content::Line);
                let keeps = matches!(it.content, Content::Picture { .. } | Content::Group { .. });
                if (keep_shape ^ keeps) && [0, 2, 4, 6].contains(&handle) && !is_line {
                    let aspect = start.w / start.h.max(0.001);
                    let w = x1 - x0;
                    let h = w / aspect;
                    if handle == 0 || handle == 2 {
                        y0 = y1 - h;
                    } else {
                        y1 = y0 + h;
                    }
                }
                it.frame = Frame::new(snap8(x0), snap8(y0), (snap8(x1) - snap8(x0)).max(min), (snap8(y1) - snap8(y0)).max(if is_line { 0.0 } else { min }));
            }
            app.guides = guides;
        }
        Some(Drag::Rotate { id, .. }) => {
            let id = *id;
            if let Some(it) = app.item_mut(id) {
                let c = it.frame.center();
                let mut deg = (at.1 - c.1).atan2(at.0 - c.0).to_degrees() + 90.0;
                if !free {
                    // Gentle pull to every 15 degrees.
                    let near = (deg / 15.0).round() * 15.0;
                    if (deg - near).abs() < 4.0 {
                        deg = near;
                    }
                }
                it.rotation = deg.rem_euclid(360.0);
            }
        }
        Some(Drag::Bulge { id, start, origin }) => {
            let (id, start, origin) = (*id, *start, *origin);
            if let Some(it) = app.item_mut(id) {
                let w = it.frame.w.max(0.1);
                if let Content::Headline { bulge, .. } = &mut it.content {
                    *bulge = (start + (at.0 - origin.0) / w * 4.0).clamp(-2.0, 2.0);
                }
            }
        }
        Some(Drag::Warp { id, start, origin }) => {
            let (id, start, origin) = (*id, *start, *origin);
            if let Some(it) = app.item_mut(id) {
                let h = it.frame.h.max(0.1);
                if let Content::Headline { bend, .. } = &mut it.content {
                    *bend = (start - (at.1 - origin.1) / (h * 0.11)).clamp(-4.0, 4.0);
                }
            }
        }
        _ => {}
    }
}

/// How many marks per inch the rulers can show at this size (16 = every
/// 1/16 inch), keeping marks at least a few points apart.
fn ruler_divisions(scale: f32) -> f32 {
    [16.0, 8.0, 4.0, 2.0, 1.0].into_iter().find(|d| scale / d >= 5.0).unwrap_or(1.0)
}

/// Mark length for a position given in 1/16 inch steps.
fn mark_len(sixteenths: i64) -> f32 {
    let r = sixteenths.rem_euclid(16);
    RULER
        * if r == 0 {
            0.62
        } else if r % 8 == 0 {
            0.46
        } else if r % 4 == 0 {
            0.34
        } else if r % 2 == 0 {
            0.25
        } else {
            0.16
        }
}

fn rulers(ui: &egui::Ui, full: Rect, view: &View, app: &App, pal: &theme::Palette, page: Rect, pointer: Option<Pos2>) {
    let painter = ui.painter_at(full);
    let top = Rect::from_min_max(full.min + vec2(RULER, 0.0), pos2(full.max.x, full.min.y + RULER));
    let left = Rect::from_min_max(full.min + vec2(0.0, RULER), pos2(full.min.x + RULER, full.max.y));
    painter.rect_filled(top, 0.0, pal.ruler);
    painter.rect_filled(left, 0.0, pal.ruler);
    painter.rect_filled(Rect::from_min_size(full.min, vec2(RULER, RULER)), 0.0, pal.ruler);
    // The page's own stretch of each ruler is lighter, as on paper.
    let band = pal.ruler_text.gamma_multiply(0.07);
    painter.rect_filled(Rect::from_min_max(pos2(page.left().max(top.left()), top.top()), pos2(page.right().min(top.right()), top.bottom())), 0.0, band);
    painter.rect_filled(Rect::from_min_max(pos2(left.left(), page.top().max(left.top())), pos2(left.right(), page.bottom().min(left.bottom()))), 0.0, band);
    let _ = app;
    let ink = Stroke::new(1.0, pal.ruler_text.gamma_multiply(0.7));
    let scale = view.scale;
    let div = ruler_divisions(scale);
    let per = (16.0 / div) as i64; // sixteenths between marks
    // Number every inch, or every few when zoomed far out.
    let every = [1i64, 2, 5, 10, 20, 50].into_iter().find(|e| scale * *e as f32 >= 26.0).unwrap_or(100);
    let font = egui::FontId::proportional(10.0);
    let along = |vertical: bool| {
        let (lo, hi, o) = if vertical { (left.top(), left.bottom(), view.origin.y) } else { (top.left(), top.right(), view.origin.x) };
        let first = (((lo - o) / scale) * 16.0).floor() as i64;
        let last = (((hi - o) / scale) * 16.0).ceil() as i64;
        let mut k = first - first.rem_euclid(per);
        while k <= last {
            let v = o + k as f32 / 16.0 * scale;
            let whole = k.rem_euclid(16) == 0;
            let inch = k.div_euclid(16);
            // Far out, only the numbered inches get marks.
            let show = div > 1.0 || scale >= 5.0 || (whole && inch % every == 0);
            if v >= lo && show {
                let len = mark_len(k);
                if vertical {
                    painter.line_segment([pos2(left.right(), v), pos2(left.right() - len, v)], ink);
                } else {
                    painter.line_segment([pos2(v, top.bottom()), pos2(v, top.bottom() - len)], ink);
                }
                if whole && inch % every == 0 {
                    if vertical {
                        painter.text(pos2(left.left() + 2.0, v + 2.0), egui::Align2::LEFT_TOP, format!("{inch}"), font.clone(), pal.ruler_text);
                    } else {
                        painter.text(pos2(v + 3.0, top.top() + 1.0), egui::Align2::LEFT_TOP, format!("{inch}"), font.clone(), pal.ruler_text);
                    }
                }
            }
            k += per;
        }
    };
    along(false);
    along(true);
    if let Some(p) = pointer {
        painter.line_segment([pos2(p.x, top.top()), pos2(p.x, top.bottom())], Stroke::new(1.0, pal.accent));
        painter.line_segment([pos2(left.left(), p.y), pos2(left.right(), p.y)], Stroke::new(1.0, pal.accent));
    }
}

/// Guideline markers on the rulers: click a ruler to add one; drag a
/// marker along the ruler to move it (1/16-inch steps); drag it off the
/// ruler, click its ×, or select it and press Delete to remove it.
fn guide_rulers(ui: &mut egui::Ui, app: &mut App, full: Rect, view: &View, pal: &theme::Palette) -> Option<Act> {
    let mut out = None;
    let ctx = ui.ctx().clone();
    let top = Rect::from_min_max(full.min + vec2(RULER, 0.0), pos2(full.max.x, full.min.y + RULER));
    let left = Rect::from_min_max(full.min + vec2(0.0, RULER), pos2(full.min.x + RULER, full.max.y));
    let pg = app.panel;
    let (pw, ph) = (app.doc.width, app.doc.height);
    let painter = ui.painter_at(full);
    let hover = ctx.input(|i| i.pointer.hover_pos());
    let along = |vertical: bool, p: Pos2| {
        let (x, y) = view.to_page(p);
        sixteenth(if vertical { x } else { y }).clamp(-1.0, if vertical { pw } else { ph } + 1.0)
    };

    // An empty part of a ruler: click (or press and slide) to place one.
    for (vertical, strip) in [(true, top), (false, left)] {
        let r = ui.interact(strip, ui.scope_id().with(("ruler", vertical)), Sense::click_and_drag());
        if r.hovered() && app.guide_drag.is_none() {
            app.hint = "Click the ruler to add a guideline (they never print). Drag its marker to move it; drag it off the ruler, click its ×, or press Delete to remove it.".into();
            if let Some(p) = hover {
                let s = view.to_screen(along(vertical, p), along(vertical, p));
                let faint = Stroke::new(1.0, GUIDE.gamma_multiply(0.6));
                if vertical {
                    painter.line_segment([pos2(s.x, strip.top()), pos2(s.x, strip.bottom())], faint);
                } else {
                    painter.line_segment([pos2(strip.left(), s.y), pos2(strip.right(), s.y)], faint);
                }
            }
        }
        let start = if r.drag_started() { ctx.input(|i| i.pointer.press_origin()) } else if r.clicked() { r.interact_pointer_pos() } else { None };
        if let Some(p) = start {
            if r.drag_started() {
                app.drag_snapshot = Some(app.doc.clone());
            } else {
                app.checkpoint();
            }
            app.commit_edit();
            app.page_mut().guides.push(Guide { vertical, at: along(vertical, p) });
            let index = app.page().guides.len() - 1;
            app.guide_sel = Some((pg, index));
            app.selected.clear();
            if r.drag_started() {
                app.guide_drag = Some(crate::app::GuideDrag { page: pg, index, removing: false });
            }
            if !app.settings.show_guides {
                app.settings.show_guides = true;
                app.settings.save();
            }
        }
        r.context_menu(|ui| {
            if ui.add_enabled(!app.page().guides.is_empty(), egui::Button::new("Remove Guidelines on This Page")).clicked() {
                out = Some(Act::ClearGuides);
                ui.close();
            }
            if ui.button(if app.settings.show_guides { "Hide Guidelines" } else { "Show Guidelines" }).clicked() {
                out = Some(Act::ToggleGuides);
                ui.close();
            }
            if ui.button("Hide Rulers").clicked() {
                out = Some(Act::ToggleRulers);
                ui.close();
            }
        });
    }

    // The markers themselves.
    if app.settings.show_guides {
        let guides = app.page().guides.clone();
        let mut remove = None;
        for (i, g) in guides.iter().enumerate() {
            let s = view.to_screen(g.at, g.at);
            let (strip, pos) = if g.vertical { (top, s.x) } else { (left, s.y) };
            let (lo, hi) = if g.vertical { (strip.left(), strip.right()) } else { (strip.top(), strip.bottom()) };
            if pos < lo - 6.0 || pos > hi + 6.0 {
                continue;
            }
            let (handle, xbtn) = if g.vertical {
                (Rect::from_center_size(pos2(pos, strip.center().y), vec2(12.0, RULER)), Rect::from_center_size(pos2(pos + 15.0, strip.center().y), vec2(15.0, 15.0)))
            } else {
                (Rect::from_center_size(pos2(strip.center().x, pos), vec2(RULER, 12.0)), Rect::from_center_size(pos2(strip.center().x, pos + 15.0), vec2(15.0, 15.0)))
            };
            let selected = app.guide_sel == Some((pg, i));
            let dragging = app.guide_drag.is_some_and(|d| d.page == pg && d.index == i);
            let near = hover.is_some_and(|p| handle.union(xbtn).expand(3.0).contains(p));
            let hr = ui.interact(handle, ui.scope_id().with(("guide", pg, i)), Sense::click_and_drag());
            if hr.hovered() && !dragging {
                app.hint = format!("Guideline at {}. Drag along the ruler to move it; drag it off the ruler or click × to remove it.", inches_label(g.at));
                ctx.set_cursor_icon(if g.vertical { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::ResizeVertical });
            }
            if hr.clicked() {
                app.commit_edit();
                app.guide_sel = Some((pg, i));
                app.selected.clear();
            }
            if hr.drag_started() {
                app.commit_edit();
                app.drag_snapshot = Some(app.doc.clone());
                app.guide_drag = Some(crate::app::GuideDrag { page: pg, index: i, removing: false });
                app.guide_sel = Some((pg, i));
                app.selected.clear();
            }
            hr.context_menu(|ui| {
                if ui.button("Remove This Guideline").clicked() {
                    remove = Some(i);
                    ui.close();
                }
                if ui.button("Remove Guidelines on This Page").clicked() {
                    out = Some(Act::ClearGuides);
                    ui.close();
                }
            });
            // The marker: a small tab pointing at the page.
            let fill = if selected || dragging { GUIDE } else { GUIDE.gamma_multiply(0.8) };
            let pts = if g.vertical {
                vec![pos2(pos - 5.0, strip.top() + 3.0), pos2(pos + 5.0, strip.top() + 3.0), pos2(pos + 5.0, strip.bottom() - 7.0), pos2(pos, strip.bottom() - 1.0), pos2(pos - 5.0, strip.bottom() - 7.0)]
            } else {
                vec![pos2(strip.left() + 3.0, pos - 5.0), pos2(strip.right() - 7.0, pos - 5.0), pos2(strip.right() - 1.0, pos), pos2(strip.right() - 7.0, pos + 5.0), pos2(strip.left() + 3.0, pos + 5.0)]
            };
            painter.add(egui::Shape::convex_polygon(pts, fill, Stroke::new(if selected { 1.6 } else { 1.0 }, Color32::WHITE)));
            if (selected || near) && !dragging {
                let xr = ui.interact(xbtn, ui.scope_id().with(("guide-x", pg, i)), Sense::click());
                let hot = xr.hovered();
                let c = xbtn.center();
                painter.circle(c, 7.0, if hot { Color32::from_rgb(214, 64, 80) } else { pal.ruler_text.gamma_multiply(0.75) }, Stroke::new(1.0, Color32::WHITE));
                let d = 2.8;
                let x = Stroke::new(1.6, Color32::WHITE);
                painter.line_segment([c + vec2(-d, -d), c + vec2(d, d)], x);
                painter.line_segment([c + vec2(-d, d), c + vec2(d, -d)], x);
                if xr.on_hover_text("Remove this guideline").clicked() {
                    remove = Some(i);
                }
            }
        }
        if let Some(i) = remove
            && i < app.page().guides.len()
        {
            app.checkpoint();
            app.page_mut().guides.remove(i);
            app.guide_sel = None;
        }
    }

    // Moving a marker: along the ruler it follows the pointer in 1/16 inch
    // steps; pulled off the ruler it turns red and letting go removes it.
    if let Some(mut gd) = app.guide_drag {
        let (down, pos) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.latest_pos()));
        let valid = app.doc.panels.get(gd.page).is_some_and(|p| gd.index < p.guides.len());
        if !valid {
            app.guide_drag = None;
            app.drag_snapshot = None;
            return out;
        }
        let vertical = app.doc.panels[gd.page].guides[gd.index].vertical;
        if let Some(p) = pos {
            let away = if vertical { p.y - top.bottom() } else { p.x - left.right() };
            gd.removing = away > 40.0;
            if !gd.removing {
                app.doc.panels[gd.page].guides[gd.index].at = along(vertical, p);
            }
            let at = app.doc.panels[gd.page].guides[gd.index].at;
            let text = if gd.removing { "Let go to remove".to_string() } else { inches_label(at) };
            let tip = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("guide-tip")));
            let galley = tip.layout_no_wrap(text, egui::FontId::proportional(12.0), Color32::WHITE);
            let r = Rect::from_min_size(p + vec2(14.0, 14.0), galley.size() + vec2(12.0, 6.0));
            tip.rect_filled(r, 5.0, if gd.removing { Color32::from_rgb(196, 56, 72) } else { Color32::from_rgb(30, 110, 150) });
            tip.galley(r.min + vec2(6.0, 3.0), galley, Color32::WHITE);
            ctx.set_cursor_icon(if gd.removing {
                egui::CursorIcon::NotAllowed
            } else if vertical {
                egui::CursorIcon::ResizeHorizontal
            } else {
                egui::CursorIcon::ResizeVertical
            });
        }
        app.guide_drag = Some(gd);
        if !down {
            if gd.removing {
                app.doc.panels[gd.page].guides.remove(gd.index);
                app.guide_sel = None;
            }
            app.guide_drag = None;
            app.end_drag();
        }
    }
    out
}
