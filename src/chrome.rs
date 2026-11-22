//! The window frame around the page: a title bar that also holds the menus
//! and window buttons, the two toolbars, the left tool panel, project and
//! page tabs, and the status bar.

use eframe::egui::{self, Color32, Key, Modifiers, RichText, Sense, Stroke, vec2};

use crate::app::{Act, App};
use crate::colors::{self, Picked};
use crate::icons::{self, Icon};
use crate::model::*;
use crate::theme::{self, Look};

pub struct Section {
    pub title: &'static str,
    pub icon: Icon,
    pub tools: Vec<(&'static str, Act, &'static str)>,
}

pub fn sections() -> Vec<Section> {
    vec![
        Section {
            title: "New Project",
            icon: Icon::Sparkle,
            tools: vec![
                ("Start a New Project", Act::NewProject, "Pick what to make: a card, sign, banner, label and more"),
                ("New Tab", Act::NewTab, "Start another project in its own tab"),
                ("Open a Saved Project", Act::Open, "Open a PrintCraft design or an older project file"),
                ("Save This Project", Act::Save, "Save your work"),
            ],
        },
        Section {
            title: "Pictures",
            icon: Icon::Picture,
            tools: vec![
                ("Insert Art from Content Packs", Act::Gallery(0), "Choose art from your content packs"),
                ("Search Google Images", Act::Gallery(1), "Find photos and clip art with Google Images"),
                ("My Art", Act::MyArt, "Pictures you saved for reuse"),
                ("Insert Picture from File", Act::PictureFromFile, "Use a photo from your computer"),
                ("Create a Picture with AI", Act::AiPictures, "Describe a picture and have it drawn for you"),
                ("Save Picture to My Art", Act::SaveToMyArt, "Keep the selected picture for other projects"),
                ("Add Border...", Act::Borders, "Put a decorative border around the page"),
                ("Backdrop / Page Color", Act::Backdrop, "Change the page's background color"),
            ],
        },
        Section {
            title: "Text Tools",
            icon: Icon::Letter,
            tools: vec![
                ("Insert Text Box", Act::TextBox, "Add a box you can type in"),
                ("Insert Headline", Act::Headline, "Add a big decorative title"),
                ("Insert Text in a Shape", Act::TextInShape, "Text inside an oval, star, heart and more"),
                ("Insert Address Merge Field", Act::AddressMerge, "Put an address from your address book"),
                ("Apply Text Shape", Act::TextShape, "Give the selected text box a shape; the words follow it"),
                ("Fit Text to Its Shape", Act::FitText, "Make the words grow or shrink to fill the box or shape"),
                ("Apply Speech Bubble", Act::SpeechBubble, "Turn the selected text into a speech bubble"),
            ],
        },
        Section {
            title: "Effects",
            icon: Icon::Wand,
            tools: vec![
                ("Apply Drop Shadow", Act::Shadow, "A shadow behind the selected item"),
                ("Apply Glow", Act::Glow, "A soft glow around the selected item"),
                ("Apply Frame", Act::Frame, "An outline around the selected item"),
                ("See-Through 50%", Act::Opacity(0.5), "Make the selected item half see-through"),
                ("Solid Again", Act::Opacity(1.0), "Not see-through"),
                ("Tint 50%", Act::Tint(0.5), "Fade the colors toward white"),
                ("Tint 100%", Act::Tint(1.0), "Full color"),
            ],
        },
        Section {
            title: "Decorations",
            icon: Icon::Seal,
            tools: vec![
                ("Insert Seal", Act::Seal, "An award seal with your words"),
                ("Insert Initial Capital", Act::InitialCap, "A large fancy first letter for a paragraph"),
                ("Insert Number", Act::Number, "A number in a circle"),
                ("Insert Signature...", Act::Signature, "Your name written in handwriting"),
                ("Borders...", Act::Borders, "Choose and adjust page borders"),
            ],
        },
        Section {
            title: "Photo Tools",
            icon: Icon::Camera,
            tools: vec![
                ("Crop & Orientation", Act::Photo(0), "Trim or turn the selected photo"),
                ("Adjust Color & Recolor", Act::Photo(1), "Richer, softer, warmer, or change its colors"),
                ("Adjust Brightness & Focus", Act::Photo(2), "Lighten, darken or sharpen"),
                ("Fix Red Eye", Act::Photo(3), "Remove red eye"),
                ("Apply Photo Effects", Act::Photo(4), "Black & white, sepia and more"),
                ("Cut Out & Remove Background", Act::Photo(5), "Shapes, custom outline, magic wand, background removal"),
            ],
        },
        Section {
            title: "Drawing Tools",
            icon: Icon::Pencil,
            tools: vec![
                ("Draw a Line", Act::Line, "A straight line"),
                ("Draw a Rectangle", Act::Shape(Outline::Rectangle), "A box"),
                ("Draw an Oval", Act::Shape(Outline::Ellipse), "An oval or circle"),
                ("Draw a Star", Act::Shape(Outline::Star), "A star"),
                ("Draw a Heart", Act::Shape(Outline::Heart), "A heart"),
                ("More Shapes...", Act::MoreShapes, "Pick from all 28 shapes"),
                ("Group", Act::Group, "Keep the selected items together"),
                ("Ungroup", Act::Ungroup, "Separate a group into its items"),
            ],
        },
        Section {
            title: "Addresses & Dates",
            icon: Icon::Contacts,
            tools: vec![
                ("Address Book", Act::AddressBook, "Names and addresses for envelopes and labels"),
                ("Event Reminder", Act::Reminders, "Birthdays coming up"),
                ("Insert an Address", Act::AddressMerge, "Put an address on the page"),
            ],
        },
        Section {
            title: "Print & Share",
            icon: Icon::Flag,
            tools: vec![
                ("Print...", Act::Print, "Print your project"),
                ("Print Preview", Act::PrintPreview, "See exactly how it will print"),
                ("Save as PDF", Act::SavePdf, "Save a PDF to email or print elsewhere"),
                ("Save as Picture", Act::ExportPicture, "Save a PNG or JPEG picture"),
            ],
        },
        Section {
            title: "Online",
            icon: Icon::Cloud,
            tools: vec![
                ("Google Images", Act::Gallery(1), "Find photos and clip art with Google Images"),
                ("Online Fonts", Act::GetFonts, "Add free fonts"),
                ("My Content Packs", Act::ContentPacks, "Use art collections you own"),
                ("AI Pictures", Act::AiPictures, "Make pictures from a description"),
            ],
        },
    ]
}

/// Everything the search box can find: (words, action, kind).
pub fn search_index() -> Vec<(String, Act, &'static str)> {
    let mut v: Vec<(String, Act, &'static str)> = Vec::new();
    for s in sections() {
        for (label, act, tip) in s.tools {
            v.push((format!("{label} — {tip}"), act, "Tool"));
        }
    }
    let more: [(&str, Act, &str); 30] = [
        ("Undo", Act::Undo, "Edit"),
        ("Redo", Act::Redo, "Edit"),
        ("Duplicate", Act::Duplicate, "Edit"),
        ("Select all", Act::SelectAll, "Edit"),
        ("Align left edges", Act::AlignItems(0), "Arrange"),
        ("Align centers", Act::AlignItems(1), "Arrange"),
        ("Align right edges", Act::AlignItems(2), "Arrange"),
        ("Align tops", Act::AlignItems(3), "Arrange"),
        ("Align middles", Act::AlignItems(4), "Arrange"),
        ("Align bottoms", Act::AlignItems(5), "Arrange"),
        ("Space evenly across", Act::Distribute(true), "Arrange"),
        ("Space evenly down", Act::Distribute(false), "Arrange"),
        ("Bring to front", Act::ToFront, "Arrange"),
        ("Send to back", Act::ToBack, "Arrange"),
        ("Flip horizontal mirror", Act::FlipH, "Arrange"),
        ("Rotate turn 90", Act::Rotate(90.0), "Arrange"),
        ("Lock unlock", Act::Lock, "Arrange"),
        ("Add a page panel", Act::AddPanel, "Pages"),
        ("Zoom fit to window", Act::Fit, "View"),
        ("Rulers show hide", Act::ToggleRulers, "View"),
        ("Dark mode light mode theme", Act::ToggleLook, "View"),
        ("Settings preferences options snapping printable area margins autosave interface modern classic", Act::Settings, "Settings"),
        ("Help topics how do I", Act::HelpTopics, "Help"),
        ("Keyboard shortcuts", Act::Help, "Help"),
        ("About PrintCraft", Act::About, "Help"),
        ("Export picture PNG JPEG", Act::ExportPicture, "File"),
        ("PDF", Act::SavePdf, "File"),
        ("Save as copy", Act::SaveAs, "File"),
        ("Fonts download", Act::GetFonts, "Online"),
        ("Eyedropper pick color from page", Act::PickColorFromPage(true), "Color"),
    ];
    v.extend(more.into_iter().map(|(a, b, c)| (a.to_string(), b, c)));
    for (title, _) in crate::help::TOPICS {
        v.push((title.to_string(), Act::HelpTopics, "Help topic"));
    }
    v
}

fn item(ui: &mut egui::Ui, app: &mut App, label: &str, act: Act, out: &mut Option<Act>) {
    let on = app.enabled(&act);
    if ui.add_enabled(on, egui::Button::new(label)).clicked() {
        *out = Some(act);
        ui.close();
    }
}

fn menus(ui: &mut egui::Ui, app: &mut App, out: &mut Option<Act>) {
    ui.menu_button("File", |ui| {
        item(ui, app, "New...            Ctrl+N", Act::NewProject, out);
        item(ui, app, "New Tab           Ctrl+T", Act::NewTab, out);
        item(ui, app, "Open...           Ctrl+O", Act::Open, out);
        ui.separator();
        item(ui, app, "Save              Ctrl+S", Act::Save, out);
        item(ui, app, "Save As...", Act::SaveAs, out);
        item(ui, app, "Save as PDF...", Act::SavePdf, out);
        item(ui, app, "Export As Picture...", Act::ExportPicture, out);
        ui.separator();
        item(ui, app, "Print Preview", Act::PrintPreview, out);
        item(ui, app, "Print...          Ctrl+P", Act::Print, out);
        ui.separator();
        let recent = app.settings.recent.clone();
        ui.menu_button("Recent Projects", |ui| {
            if recent.is_empty() {
                ui.label("None yet");
            }
            for r in recent {
                let name = r.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string();
                if ui.button(name).clicked() {
                    app.open_path(r);
                    ui.close();
                }
            }
        });
        item(ui, app, "Close Tab         Ctrl+W", Act::CloseTab(app.active), out);
        item(ui, app, "Settings...", Act::Settings, out);
        item(ui, app, "Exit", Act::Exit, out);
    });
    ui.menu_button("Edit", |ui| {
        item(ui, app, "Undo              Ctrl+Z", Act::Undo, out);
        item(ui, app, "Redo              Ctrl+Y", Act::Redo, out);
        ui.separator();
        item(ui, app, "Cut               Ctrl+X", Act::Cut, out);
        item(ui, app, "Copy              Ctrl+C", Act::Copy, out);
        item(ui, app, "Paste             Ctrl+V", Act::Paste, out);
        item(ui, app, "Duplicate         Ctrl+D", Act::Duplicate, out);
        item(ui, app, "Select All        Ctrl+A", Act::SelectAll, out);
        item(ui, app, "Delete            Del", Act::Delete, out);
        ui.separator();
        item(ui, app, "Add a Page", Act::AddPanel, out);
        item(ui, app, "Project Notes...", Act::ProjectNotes, out);
    });
    ui.menu_button("View", |ui| {
        item(ui, app, "Zoom In           +", Act::ZoomIn, out);
        item(ui, app, "Zoom Out          -", Act::ZoomOut, out);
        item(ui, app, "Fit to Window     Ctrl+F", Act::Fit, out);
        ui.menu_button("Percent View", |ui| {
            for (l, z) in [("25%", 0.25), ("50%", 0.5), ("Actual Size", 1.0), ("200%", 2.0), ("400%", 4.0), ("600%", 6.0)] {
                item(ui, app, l, Act::Zoom(z), out);
            }
        });
        ui.separator();
        let rulers = if app.settings.show_rulers { "Hide Rulers" } else { "Show Rulers" };
        item(ui, app, rulers, Act::ToggleRulers, out);
        let guides = if app.settings.show_guides { "Hide Guidelines" } else { "Show Guidelines" };
        item(ui, app, guides, Act::ToggleGuides, out);
        item(ui, app, "Remove Guidelines on This Page", Act::ClearGuides, out);
        ui.separator();
        let pages = if app.settings.legacy_pages { "Show All Pages" } else { "One Page at a Time (Legacy Pages)" };
        item(ui, app, pages, Act::ToggleLegacyPages, out);
        if !app.settings.legacy_pages {
            let tile = if app.settings.tile_pages { "Pages in a List" } else { "Pages Side by Side" };
            item(ui, app, tile, Act::ToggleTilePages, out);
        }
        let look = if app.settings.look == Look::Light { "Dark Mode" } else { "Light Mode" };
        item(ui, app, look, Act::ToggleLook, out);
        ui.separator();
        for (i, p) in app.doc.panels.clone().iter().enumerate() {
            item(ui, app, &format!("{} Panel", p.name), Act::Panel(i), out);
        }
    });
    ui.menu_button("Insert", |ui| {
        item(ui, app, "From Content Packs...", Act::Gallery(0), out);
        item(ui, app, "From Google Images...", Act::Gallery(1), out);
        item(ui, app, "From My Art...", Act::MyArt, out);
        item(ui, app, "From File...", Act::PictureFromFile, out);
        item(ui, app, "AI Picture...", Act::AiPictures, out);
        ui.separator();
        item(ui, app, "Headline...", Act::Headline, out);
        item(ui, app, "Text Box", Act::TextBox, out);
        item(ui, app, "Text in a Shape", Act::TextInShape, out);
        ui.separator();
        item(ui, app, "Border...", Act::Borders, out);
        item(ui, app, "Backdrop...", Act::Backdrop, out);
        ui.menu_button("Custom Graphic", |ui| {
            item(ui, app, "Initial Capital", Act::InitialCap, out);
            item(ui, app, "Number", Act::Number, out);
            item(ui, app, "Seal", Act::Seal, out);
            item(ui, app, "Signature...", Act::Signature, out);
        });
        item(ui, app, "Line", Act::Line, out);
        item(ui, app, "Shape...", Act::MoreShapes, out);
        item(ui, app, "Page", Act::AddPanel, out);
    });
    ui.menu_button("Arrange", |ui| {
        ui.menu_button("Position", |ui| {
            item(ui, app, "Center on Page", Act::CenterPage, out);
            item(ui, app, "Center Horizontally", Act::CenterH, out);
            item(ui, app, "Center Vertically", Act::CenterV, out);
        });
        ui.menu_button("Align", |ui| {
            for (l, k) in [("Left Edges", 0), ("Centers", 1), ("Right Edges", 2), ("Tops", 3), ("Middles", 4), ("Bottoms", 5)] {
                item(ui, app, l, Act::AlignItems(k), out);
            }
            ui.separator();
            item(ui, app, "Space Evenly Across", Act::Distribute(true), out);
            item(ui, app, "Space Evenly Down", Act::Distribute(false), out);
        });
        ui.menu_button("Size", |ui| item(ui, app, "Size to Page", Act::FullPage, out));
        ui.menu_button("Rotate", |ui| {
            item(ui, app, "Left 90 degrees", Act::Rotate(-90.0), out);
            item(ui, app, "Right 90 degrees", Act::Rotate(90.0), out);
        });
        ui.menu_button("Flip", |ui| {
            item(ui, app, "Horizontal", Act::FlipH, out);
            item(ui, app, "Vertical", Act::FlipV, out);
        });
        ui.menu_button("Layer", |ui| {
            item(ui, app, "Bring Forward", Act::Forward, out);
            item(ui, app, "Bring to Front", Act::ToFront, out);
            item(ui, app, "Send Backward", Act::Backward, out);
            item(ui, app, "Send to Back", Act::ToBack, out);
        });
        item(ui, app, "Group             Ctrl+G", Act::Group, out);
        item(ui, app, "Ungroup           Ctrl+Shift+G", Act::Ungroup, out);
        item(ui, app, "Lock / Unlock", Act::Lock, out);
    });
    ui.menu_button("Effects", |ui| {
        item(ui, app, "Apply Drop Shadow...", Act::Shadow, out);
        item(ui, app, "Apply Glow...", Act::Glow, out);
        item(ui, app, "Apply Frame...", Act::Frame, out);
        ui.menu_button("Tint", |ui| {
            for t in (1..=10).rev() {
                item(ui, app, &format!("{}%", t * 10), Act::Tint(t as f32 / 10.0), out);
            }
        });
        ui.menu_button("See-Through", |ui| {
            for t in (1..=10).rev() {
                item(ui, app, &format!("{}% solid", t * 10), Act::Opacity(t as f32 / 10.0), out);
            }
        });
        ui.separator();
        item(ui, app, "Crop & Orientation...", Act::Photo(0), out);
        item(ui, app, "Adjust Color & Recolor...", Act::Photo(1), out);
        item(ui, app, "Adjust Brightness & Focus...", Act::Photo(2), out);
        item(ui, app, "Fix Red Eye...", Act::Photo(3), out);
        item(ui, app, "Apply Photo Effects...", Act::Photo(4), out);
        item(ui, app, "Cut Out & Remove Background...", Act::Photo(5), out);
    });
    ui.menu_button("Text", |ui| {
        item(ui, app, "Bold              Ctrl+B", Act::Bold, out);
        item(ui, app, "Italic            Ctrl+I", Act::Italic, out);
        item(ui, app, "Underline         Ctrl+U", Act::Underline, out);
        ui.menu_button("Horizontal Alignment", |ui| {
            item(ui, app, "Left", Act::Align(Align::Left), out);
            item(ui, app, "Center", Act::Align(Align::Center), out);
            item(ui, app, "Right", Act::Align(Align::Right), out);
            item(ui, app, "Full", Act::Align(Align::Justify), out);
        });
        ui.menu_button("Vertical Alignment", |ui| {
            item(ui, app, "Top", Act::VAlign(VAlign::Top), out);
            item(ui, app, "Middle", Act::VAlign(VAlign::Middle), out);
            item(ui, app, "Bottom", Act::VAlign(VAlign::Bottom), out);
        });
        item(ui, app, "Text Box Shape...", Act::TextShape, out);
        item(ui, app, "Fit Text to Its Shape", Act::FitText, out);
        item(ui, app, "Apply Speech Bubble", Act::SpeechBubble, out);
        item(ui, app, "Stretch Headline Letters", Act::StretchText, out);
        item(ui, app, "Insert Address Merge Field...", Act::AddressMerge, out);
    });
    ui.menu_button("Table", |ui| {
        ui.label(RichText::new("Tables are planned for a later version.\nSee Help > Help Topics > \"What's coming\".").weak());
    });
    ui.menu_button("Web", |ui| {
        ui.label(RichText::new("Web pages are planned for a later version.\nSee Help > Help Topics > \"What's coming\".").weak());
    });
    ui.menu_button("Tools", |ui| {
        item(ui, app, "Address Book...", Act::AddressBook, out);
        item(ui, app, "Event Reminder...", Act::Reminders, out);
        ui.separator();
        item(ui, app, "My Content Packs...", Act::ContentPacks, out);
        item(ui, app, "My Art...", Act::MyArt, out);
        item(ui, app, "Online Fonts...", Act::GetFonts, out);
        item(ui, app, "AI Pictures...", Act::AiPictures, out);
        ui.separator();
        item(ui, app, "Settings...", Act::Settings, out);
    });
    ui.menu_button("Window", |ui| {
        item(ui, app, "New Tab", Act::NewTab, out);
        let views = app.tab_views();
        for (i, (n, d)) in views.iter().enumerate() {
            item(ui, app, &format!("{}{}", if *d { "• " } else { "" }, n), Act::SwitchTab(i), out);
        }
        ui.separator();
        let look = if app.settings.look == Look::Light { "Switch to Dark Mode" } else { "Switch to Light Mode" };
        item(ui, app, look, Act::ToggleLook, out);
    });
    ui.menu_button("Help", |ui| {
        item(ui, app, "Help Topics...    F1", Act::HelpTopics, out);
        item(ui, app, "Getting Around", Act::Help, out);
        item(ui, app, "About PrintCraft...", Act::About, out);
    });
}

/// Title bar: app icon, menus, search, light/dark and window buttons.
/// Dragging empty space moves the window; double-click maximizes.
pub fn title_bar(ui: &mut egui::Ui, app: &mut App, icon: Option<&egui::TextureHandle>) -> Option<Act> {
    let mut out = None;
    let pal = theme::palette(app.settings.look);
    let bar = ui.max_rect();
    ui.horizontal(|ui| {
        ui.set_height(30.0);
        if let Some(t) = icon {
            ui.add(egui::Image::new(t).fit_to_exact_size(vec2(22.0, 22.0)));
        }
        // Menus take only the room they need.
        ui.scope(|ui| {
            ui.set_max_width(640.0);
            egui::MenuBar::new().ui(ui, |ui| menus(ui, app, &mut out));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // The browser version lives in a tab, so it has no window buttons.
            let buttons: &[(&str, &str, i32)] = if crate::platform::WEB { &[] } else { &[("✕", "Close", 2), ("☐", "Maximize", 1), ("–", "Minimize", 0)] };
            for &(glyph, tip, which) in buttons {
                let (r, resp) = ui.allocate_exact_size(vec2(44.0, 30.0), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r, 6.0, if which == 2 { Color32::from_rgb(220, 70, 70) } else { ui.visuals().widgets.hovered.weak_bg_fill });
                }
                let ink = if which == 2 && resp.hovered() { Color32::WHITE } else { pal.text };
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, glyph, egui::FontId::proportional(15.0), ink);
                if resp.on_hover_text(tip).clicked() {
                    let ctx = ui.ctx().clone();
                    match which {
                        0 => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                        1 => {
                            let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                        }
                        _ => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    }
                }
            }
            let (icon_l, tip) = if app.settings.look == Look::Light { (Icon::Moon, "Switch to dark mode") } else { (Icon::Sun, "Switch to light mode") };
            if icons::button(ui, icon_l, tip, true).clicked() {
                out = Some(Act::ToggleLook);
            }
            ui.add_space(6.0);
            search_box(ui, app, &mut out);
            // Whatever is left is a handle to move the window.
            let free = ui.available_width().max(0.0);
            let (drag_rect, drag) = ui.allocate_exact_size(vec2(free, 30.0), Sense::click_and_drag());
            let title = app.title();
            let galley = ui.painter().layout_no_wrap(title, egui::FontId::proportional(13.0), pal.muted);
            if galley.size().x < drag_rect.width() {
                ui.painter().galley(drag_rect.center() - galley.size() / 2.0, galley, pal.muted);
            }
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
        });
    });
    let _ = bar;
    out
}

fn search_box(ui: &mut egui::Ui, app: &mut App, out: &mut Option<Act>) {
    let id = egui::Id::new("search-text");
    let mut q: String = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
    let resp = ui.add(egui::TextEdit::singleline(&mut q).hint_text("🔍 Search tools, settings, help").desired_width(220.0));
    let wants = ui.ctx().input(|i| i.modifiers.command && i.key_pressed(Key::K));
    if wants {
        resp.request_focus();
    }
    let ql = q.to_lowercase();
    let words: Vec<&str> = ql.split_whitespace().collect();
    let hits: Vec<(String, Act, &str)> = if words.is_empty() {
        Vec::new()
    } else {
        search_index().into_iter().filter(|(t, _, _)| words.iter().all(|w| t.to_lowercase().contains(w))).take(12).collect()
    };
    let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
    if enter && let Some((label, act, kind)) = hits.first().cloned() {
        *out = Some(open_hit(app, act, kind, &label));
        q.clear();
    }
    if (resp.has_focus() || resp.lost_focus()) && !hits.is_empty() {
        egui::Area::new(egui::Id::new("search-results")).order(egui::Order::Foreground).fixed_pos(resp.rect.left_bottom() + vec2(0.0, 4.0)).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(360.0);
                for (label, act, kind) in &hits {
                    let r = ui.add(egui::Button::new(format!("{label}   ·  {kind}")).frame(false).min_size(vec2(350.0, 22.0)));
                    if r.clicked() {
                        *out = Some(open_hit(app, act.clone(), kind, label));
                        q.clear();
                    }
                }
            });
        });
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, q));
}

fn open_hit(app: &mut App, act: Act, kind: &str, label: &str) -> Act {
    if kind == "Help topic" {
        app.dialogs.push(crate::dialogs::Dialog::Help { query: label.to_string() });
        return Act::Zoom(app.zoom);
    }
    act
}

/// Row 1: file and editing buttons, colors, tint and see-through, help.
pub fn standard_bar(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    ui.horizontal(|ui| {
        let b = |ui: &mut egui::Ui, app: &App, out: &mut Option<Act>, icon, tip: &str, act: Act| {
            if icons::button(ui, icon, tip, app.enabled(&act)).clicked() {
                *out = Some(act);
            }
        };
        b(ui, app, &mut out, Icon::New, "New project (Ctrl+N)", Act::NewProject);
        b(ui, app, &mut out, Icon::Open, "Open (Ctrl+O)", Act::Open);
        b(ui, app, &mut out, Icon::Save, "Save (Ctrl+S)", Act::Save);
        b(ui, app, &mut out, Icon::Print, "Print (Ctrl+P)", Act::Print);
        b(ui, app, &mut out, Icon::Preview, "Print preview", Act::PrintPreview);
        ui.separator();
        b(ui, app, &mut out, Icon::Cut, "Cut (Ctrl+X)", Act::Cut);
        b(ui, app, &mut out, Icon::Copy, "Copy (Ctrl+C)", Act::Copy);
        b(ui, app, &mut out, Icon::Paste, "Paste (Ctrl+V)", Act::Paste);
        ui.separator();
        for (icon, label, tip, act) in [(Icon::Undo, "Undo", "Undo the last change (Ctrl+Z)", Act::Undo), (Icon::Redo, "Redo", "Redo (Ctrl+Y)", Act::Redo)] {
            let on = app.enabled(&act);
            let r = ui.add_enabled(on, egui::Button::new(RichText::new(format!("      {label}"))).min_size(vec2(76.0, 26.0))).on_hover_text(tip);
            icons::paint(ui.painter(), egui::Rect::from_min_size(r.rect.min + vec2(4.0, 4.0), vec2(18.0, 18.0)), icon, if on { ui.visuals().text_color() } else { ui.visuals().weak_text_color() });
            if r.clicked() {
                out = Some(act);
            }
        }
        ui.separator();
        color_button(ui, app, true);
        color_button(ui, app, false);
        tint_button(ui, app);
        ui.separator();
        b(ui, app, &mut out, Icon::Help, "Help (F1)", Act::HelpTopics);
        if ui.button("⚙ Settings").on_hover_text("Settings").clicked() {
            out = Some(Act::Settings);
        }
    });
    out
}

/// The color buttons; the popup is the shared color chooser.
fn color_button(ui: &mut egui::Ui, app: &mut App, fill: bool) {
    let current = if fill { app.fill_color } else { app.line_color };
    let (rect, resp) = ui.allocate_exact_size(vec2(60.0, 28.0), Sense::click());
    let v = ui.visuals().clone();
    if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, v.widgets.hovered.weak_bg_fill);
    }
    let icon_r = egui::Rect::from_min_size(rect.min + vec2(3.0, 4.0), vec2(20.0, 20.0));
    icons::paint(ui.painter(), icon_r, if fill { Icon::FillColor } else { Icon::LineColor }, v.text_color());
    let sw = egui::Rect::from_min_max(egui::pos2(rect.left() + 26.0, rect.top() + 6.0), egui::pos2(rect.right() - 4.0, rect.bottom() - 6.0));
    if current.is_clear() {
        ui.painter().rect_stroke(sw, 3.0, Stroke::new(1.0, v.weak_text_color()), egui::StrokeKind::Inside);
        ui.painter().line_segment([sw.left_bottom(), sw.right_top()], Stroke::new(1.0, Color32::RED));
    } else {
        ui.painter().rect_filled(sw, 3.0, colors::c32(current));
    }
    let tip = if fill { "Fill color for the selected item" } else { "Line and text color for the selected item" };
    let resp = resp.on_hover_text(tip);
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        ui.label(RichText::new(if fill { "Fill color" } else { "Line and text color" }).size(15.0));
        match colors::chooser(ui, if fill { "fill" } else { "line" }, current, &mut app.settings, true) {
            Some(Picked::Color(c, done)) => {
                apply_color(app, fill, c);
                if done {
                    ui.close();
                }
            }
            Some(Picked::Eyedropper) => {
                app.eyedropper = Some(if fill { 0 } else { 1 });
                ui.close();
            }
            None => {}
        }
    });
}

/// Tint (fade toward white) and see-through, as one toolbar button.
fn tint_button(ui: &mut egui::Ui, app: &mut App) {
    let (tint, opacity) = app.first_selected().map(|i| (i.tint, i.opacity)).unwrap_or((1.0, 1.0));
    let (rect, resp) = ui.allocate_exact_size(vec2(70.0, 28.0), Sense::click());
    let v = ui.visuals().clone();
    if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, v.widgets.hovered.weak_bg_fill);
    }
    let bar = rect.shrink2(vec2(6.0, 8.0));
    for i in 0..20 {
        let t = i as f32 / 20.0;
        let g = (40.0 + 215.0 * t) as u8;
        let seg = egui::Rect::from_min_size(egui::pos2(bar.left() + bar.width() * t, bar.top()), vec2(bar.width() / 20.0 + 0.5, bar.height()));
        ui.painter().rect_filled(seg, 0.0, Color32::from_gray(g));
    }
    ui.painter().rect_stroke(bar, 2.0, Stroke::new(1.0, v.weak_text_color()), egui::StrokeKind::Outside);
    let resp = resp.on_hover_text("Tint and see-through for the selected item");
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        ui.set_min_width(260.0);
        if app.selected.is_empty() {
            ui.label("Select something on the page first.");
            return;
        }
        let mut t = tint * 100.0;
        let mut o = opacity * 100.0;
        ui.label("Tint (fades colors toward white)");
        let rt = ui.add(egui::Slider::new(&mut t, 10.0..=100.0).suffix("%").step_by(5.0));
        ui.label("Solid ↔ see-through");
        let ro = ui.add(egui::Slider::new(&mut o, 5.0..=100.0).suffix("% solid").step_by(5.0));
        if rt.drag_started() || ro.drag_started() || ((rt.changed() || ro.changed()) && !rt.dragged() && !ro.dragged()) {
            app.checkpoint();
        }
        if rt.changed() || ro.changed() {
            let ids = app.selected.clone();
            for it in app.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id)) {
                it.tint = t / 100.0;
                it.opacity = o / 100.0;
            }
        }
    });
}

pub fn apply_color(app: &mut App, fill: bool, c: Rgba) {
    if fill {
        app.fill_color = c;
    } else {
        app.line_color = c;
        // Selected words only, when some are selected.
        if app.style_selection(&|s, _| s.color = Some(c)) {
            return;
        }
    }
    if app.selected.is_empty() {
        return;
    }
    app.checkpoint();
    let ids = app.selected.clone();
    for it in app.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id)) {
        match (&mut it.content, fill) {
            (Content::Text { look, .. }, false) | (Content::Headline { look, .. }, false) => look.color = c,
            (Content::Headline { second, .. }, true) => *second = c,
            (_, true) => it.fill = c,
            (_, false) => it.stroke = c,
        }
        if !fill && it.stroke_pt <= 0.0 {
            it.stroke_pt = 1.0;
        }
    }
}

/// Change the look of the text being edited or the selected text items.
fn change_look(app: &mut App, checkpoint: bool, f: &dyn Fn(&mut TextLook)) {
    if checkpoint {
        app.checkpoint();
    }
    let ids: Vec<u64> = app.editing.as_ref().map(|(id, _)| vec![*id]).unwrap_or_else(|| app.selected.clone());
    for it in app.page_mut().items.iter_mut().filter(|i| ids.contains(&i.id)) {
        if let Some(l) = it.look_mut() {
            f(l);
        }
    }
}

/// "Preview" written in a font, for the font list.
fn font_sample(app: &mut App, ctx: &egui::Context, family: &str, budget: &mut usize) -> Option<egui::TextureHandle> {
    if let Some(t) = app.font_previews.get(family) {
        return t.clone();
    }
    if *budget == 0 {
        return None;
    }
    *budget -= 1;
    let bytes = app.fonts.face_bytes(family, false, false);
    let ink = Color32::WHITE; // tinted with the theme's text color when drawn
    let tex = crate::raster::text_image(&bytes, "Preview", 22.0, [ink.r(), ink.g(), ink.b()])
        .map(|(w, h, rgba)| ctx.load_texture(format!("fontprev-{family}"), egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba), egui::TextureOptions::LINEAR));
    app.font_previews.insert(family.to_string(), tex.clone());
    tex
}

/// Font list with previews. Arrow keys move through it and the text on
/// the page changes as you go; Enter or a click keeps the choice.
fn font_picker(ui: &mut egui::Ui, app: &mut App, current: &str) {
    let resp = ui.add(egui::Button::new(RichText::new(format!("{current}  ▾"))).min_size(vec2(210.0, 26.0))).on_hover_text("Font (arrow keys try each one on your text)");
    let state = egui::Id::new("font-picker-open");
    let families = app.fonts.families();
    let mut budget = 6;
    egui::Popup::from_toggle_button_response(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
        ui.set_min_width(360.0);
        // One undo step for the whole browse.
        if !ui.ctx().data(|d| d.get_temp::<bool>(state).unwrap_or(false)) {
            ui.ctx().data_mut(|d| d.insert_temp(state, true));
            app.checkpoint();
        }
        let mut idx = families.iter().position(|f| f == current).unwrap_or(0);
        let (down, up, enter) = ui.input_mut(|i| (i.consume_key(Modifiers::NONE, Key::ArrowDown), i.consume_key(Modifiers::NONE, Key::ArrowUp), i.consume_key(Modifiers::NONE, Key::Enter)));
        let mut moved = false;
        if down && idx + 1 < families.len() {
            idx += 1;
            moved = true;
        }
        if up && idx > 0 {
            idx -= 1;
            moved = true;
        }
        if moved {
            let f = families[idx].clone();
            change_look(app, false, &|l| l.family = f.clone());
        }
        if enter {
            ui.close();
        }
        ui.label(RichText::new("↑ ↓ to try fonts on your text, Enter to keep").weak().size(12.0));
        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            for (i, f) in families.iter().enumerate() {
                let tex = font_sample(app, ui.ctx(), f, &mut budget);
                let (r, row) = ui.allocate_exact_size(vec2(340.0, 30.0), Sense::click());
                let v = ui.visuals();
                if i == idx {
                    ui.painter().rect_filled(r, 4.0, v.selection.bg_fill.gamma_multiply(0.5));
                } else if row.hovered() {
                    ui.painter().rect_filled(r, 4.0, v.widgets.hovered.weak_bg_fill);
                }
                ui.painter().text(r.left_center() + vec2(8.0, 0.0), egui::Align2::LEFT_CENTER, f, egui::FontId::proportional(14.0), v.text_color());
                if let Some(t) = tex {
                    let s = t.size_vec2();
                    let k = 22.0 / s.y.max(1.0);
                    let img = egui::Rect::from_min_size(egui::pos2(r.right() - s.x * k - 8.0, r.center().y - 11.0), s * k);
                    ui.painter().image(t.id(), img, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), ui.visuals().text_color());
                }
                if i == idx && moved {
                    row.scroll_to_me(Some(egui::Align::Center));
                }
                if row.clicked() {
                    let f = f.clone();
                    change_look(app, false, &|l| l.family = f.clone());
                    ui.close();
                }
            }
            if app.fonts.scanning() {
                ui.label(RichText::new("Looking for more fonts…").weak());
            }
        });
        if budget == 0 {
            ui.ctx().request_repaint();
        }
    });
    if !egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp)) {
        ui.ctx().data_mut(|d| d.insert_temp(state, false));
    }
}

/// One box for font size: type a number, scroll on it, or pick from ▾.
fn size_box(ui: &mut egui::Ui, app: &mut App, size: f32) {
    let id = egui::Id::new("size-box");
    let mut text: String = ui.ctx().data(|d| d.get_temp::<(String, f32)>(id)).filter(|v| (v.1 - size).abs() < 0.01).map(|v| v.0).unwrap_or_else(|| format!("{}", (size * 10.0).round() / 10.0));
    let frame = egui::Frame::new().fill(ui.visuals().extreme_bg_color).corner_radius(6.0).stroke(ui.visuals().widgets.inactive.bg_stroke).inner_margin(egui::Margin::symmetric(4, 1));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(38.0).frame(egui::Frame::NONE)).on_hover_text("Font size in points: type, scroll, or pick ▾");
        let mut pick = None;
        if r.lost_focus()
            && let Ok(v) = text.trim().parse::<f32>()
        {
            pick = Some(v);
        }
        if r.hovered() {
            let s = ui.input(|i| i.smooth_scroll_delta.y);
            if s.abs() > 0.5 {
                pick = Some(size + s.signum());
            }
        }
        let arrow = ui.add(egui::Button::new("▾").frame(false).min_size(vec2(18.0, 20.0)));
        egui::Popup::from_toggle_button_response(&arrow).show(|ui| {
            for s in [8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 36.0, 48.0, 60.0, 72.0, 96.0, 120.0] {
                if ui.selectable_label((s - size).abs() < 0.1, format!("{s}")).clicked() {
                    pick = Some(s);
                    ui.close();
                }
            }
        });
        if let Some(v) = pick {
            let v = v.clamp(4.0, 400.0);
            if !app.style_selection(&|s, _| s.size = Some(v)) {
                change_look(app, true, &|l| l.size = v);
            }
            text = format!("{v}");
        }
        ui.ctx().data_mut(|d| d.insert_temp(id, (text, size)));
    });
}

/// Row 2: text style, font, size, bold/italic/underline, alignment.
pub fn text_bar(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    let look = app.editing.as_ref().and_then(|(id, _)| app.item(*id)).or(app.first_selected()).and_then(|i| i.look().cloned());
    let enabled = look.is_some();
    let look = look.unwrap_or_default();
    ui.horizontal(|ui| {
        ui.add_enabled_ui(enabled, |ui| {
            let mut style_pick: Option<(f32, bool)> = None;
            egui::ComboBox::from_id_salt("style").width(110.0).selected_text("Text Style").show_ui(ui, |ui| {
                for (n, s, b) in [("Poster", 48.0, true), ("Heading", 28.0, true), ("Subhead", 20.0, true), ("Body Copy", 12.0, false), ("Small Copy", 9.0, false), ("Card", 16.0, false)] {
                    if ui.button(n).clicked() {
                        style_pick = Some((s, b));
                    }
                }
            });
            if let Some((s, b)) = style_pick {
                change_look(app, true, &|l| {
                    l.size = s;
                    l.bold = b;
                });
            }
            font_picker(ui, app, &look.family);
            size_box(ui, app, look.size);
            let toggle = |ui: &mut egui::Ui, text: RichText, on: bool, tip: &str| ui.add(egui::Button::selectable(on, text).min_size(vec2(28.0, 24.0))).on_hover_text(tip).clicked();
            if toggle(ui, RichText::new("B").strong(), look.bold, "Bold (Ctrl+B)") {
                out = Some(Act::Bold);
            }
            if toggle(ui, RichText::new("I").italics(), look.italic, "Italic (Ctrl+I)") {
                out = Some(Act::Italic);
            }
            if toggle(ui, RichText::new("U").underline(), look.underline, "Underline (Ctrl+U)") {
                out = Some(Act::Underline);
            }
            ui.separator();
            for (a, l, tip) in [(Align::Left, "⬅", "Align left"), (Align::Center, "↔", "Center"), (Align::Right, "➡", "Align right"), (Align::Justify, "☰", "Full justify")] {
                if toggle(ui, RichText::new(l), look.align == a, tip) {
                    out = Some(Act::Align(a));
                }
            }
            ui.separator();
            for (a, l, tip) in [(VAlign::Top, "⬆", "Top"), (VAlign::Middle, "⬍", "Middle"), (VAlign::Bottom, "⬇", "Bottom")] {
                if toggle(ui, RichText::new(l), look.valign == a, tip) {
                    out = Some(Act::VAlign(a));
                }
            }
            let fitting = matches!(app.first_selected().map(|i| &i.content), Some(Content::Text { fit: true, .. }));
            if toggle(ui, RichText::new("⤢ Fit"), fitting, "Fit the words to fill the box or shape") {
                out = Some(Act::FitText);
            }
        });
    });
    out
}

/// The left tool panel: tinted section bars that open to a list of tools.
pub fn tool_panel(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    let pal = theme::palette(app.settings.look);
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(6.0);
        for (si, sec) in sections().into_iter().enumerate() {
            let open = app.open_sections.get(si).copied().unwrap_or(false);
            // Each section has its own color from the app icon.
            const HUES: [Color32; 5] = [
                Color32::from_rgb(235, 106, 92),
                Color32::from_rgb(58, 154, 217),
                Color32::from_rgb(242, 182, 49),
                Color32::from_rgb(76, 182, 140),
                Color32::from_rgb(150, 112, 210),
            ];
            let hue = HUES[si % HUES.len()];
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
            let fill = if open { hue.gamma_multiply(if app.settings.look == Look::Light { 0.22 } else { 0.4 }) } else if resp.hovered() { pal.section_open.gamma_multiply(0.7) } else { pal.section };
            ui.painter().rect_filled(rect, 12.0, fill);
            ui.painter().rect_stroke(rect, 12.0, Stroke::new(1.0, if open { hue } else { ui.visuals().widgets.inactive.bg_stroke.color }), egui::StrokeKind::Inside);
            let badge = egui::Rect::from_center_size(rect.left_center() + vec2(20.0, 0.0), vec2(28.0, 28.0));
            ui.painter().circle_filled(badge.center(), 14.0, hue.gamma_multiply(if open { 1.0 } else { 0.28 }));
            icons::paint(ui.painter(), badge.shrink(5.0), sec.icon, if open { Color32::WHITE } else { pal.text });
            ui.painter().text(rect.left_center() + vec2(42.0, 0.0), egui::Align2::LEFT_CENTER, sec.title, egui::FontId::proportional(14.5), pal.text);
            let chevron = if open { "▴" } else { "▾" };
            ui.painter().text(rect.right_center() - vec2(12.0, 0.0), egui::Align2::CENTER_CENTER, chevron, egui::FontId::proportional(14.0), pal.muted);
            if resp.clicked()
                && let Some(o) = app.open_sections.get_mut(si)
            {
                *o = !*o;
            }
            if open {
                ui.add_space(2.0);
                for (label, act, tip) in sec.tools {
                    let on = app.enabled(&act);
                    let text = RichText::new(label).size(13.0).color(if on { pal.text } else { pal.muted.gamma_multiply(0.7) });
                    let r = ui.add_enabled(on, egui::Button::new(text).frame(false).min_size(vec2(ui.available_width(), 22.0)));
                    if r.hovered() {
                        app.hint = tip.to_string();
                    }
                    if r.clicked() {
                        out = Some(act);
                    }
                }
                ui.add_space(4.0);
            }
            ui.add_space(4.0);
        }
    });
    out
}

/// Project tabs: click to switch, × to close, right-click for more, drag
/// to reorder, + for a new one.
pub fn tab_bar(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    let pal = theme::palette(app.settings.look);
    let views = app.tab_views();
    let drag_id = egui::Id::new("tab-drag");
    let mut rects = Vec::new();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, (name, dirty)) in views.iter().enumerate() {
            let active = i == app.active;
            let label = format!("{}{}", if *dirty { "• " } else { "" }, name);
            let galley = ui.painter().layout_no_wrap(label.clone(), egui::FontId::proportional(13.5), pal.text);
            let w = (galley.size().x + 44.0).clamp(90.0, 240.0);
            let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click_and_drag());
            rects.push(r);
            let fill = if active { pal.section } else if resp.hovered() { pal.section_open.gamma_multiply(0.6) } else { pal.panel };
            ui.painter().rect_filled(r, egui::CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, fill);
            if active {
                ui.painter().rect_filled(egui::Rect::from_min_size(r.left_top(), vec2(r.width(), 3.0)), 2.0, pal.accent);
            }
            ui.painter().text(r.left_center() + vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, &label, egui::FontId::proportional(13.5), if active { pal.text } else { pal.muted });
            let close = egui::Rect::from_center_size(r.right_center() - vec2(14.0, 0.0), vec2(18.0, 18.0));
            let over_close = resp.hover_pos().is_some_and(|p| close.contains(p));
            if over_close {
                ui.painter().rect_filled(close, 4.0, pal.section_open);
            }
            ui.painter().text(close.center(), egui::Align2::CENTER_CENTER, "×", egui::FontId::proportional(14.0), pal.muted);
            if resp.clicked() {
                out = Some(if over_close { Act::CloseTab(i) } else { Act::SwitchTab(i) });
            }
            if resp.middle_clicked() {
                out = Some(Act::CloseTab(i));
            }
            if resp.drag_started() {
                ui.ctx().data_mut(|d| d.insert_temp(drag_id, i));
            }
            resp.context_menu(|ui| {
                if ui.button("Rename...").clicked() {
                    app.dialogs.push(crate::dialogs::Dialog::RenameTab { index: i, name: name.clone() });
                    ui.close();
                }
                if ui.button("Duplicate").clicked() {
                    app.duplicate_tab(i);
                    ui.close();
                }
                if ui.button("Close").clicked() {
                    out = Some(Act::CloseTab(i));
                    ui.close();
                }
            });
        }
        if ui.add(egui::Button::new("+").min_size(vec2(30.0, 28.0))).on_hover_text("New tab (Ctrl+T)").clicked() {
            out = Some(Act::NewTab);
        }
    });
    // Drag to reorder.
    if let Some(from) = ui.ctx().data(|d| d.get_temp::<usize>(drag_id)) {
        let (down, pos) = ui.ctx().input(|i| (i.pointer.primary_down(), i.pointer.interact_pos()));
        if let Some(p) = pos {
            let to = rects.iter().position(|r| p.x < r.center().x).unwrap_or(rects.len().saturating_sub(1));
            if down {
                if let Some(r) = rects.get(to) {
                    ui.painter().vline(if to <= from { r.left() - 2.0 } else { r.right() + 2.0 }, r.y_range(), Stroke::new(3.0, pal.accent));
                }
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else {
                app.move_tab(from, to.min(rects.len().saturating_sub(1)));
                ui.ctx().data_mut(|d| d.remove::<usize>(drag_id));
            }
        } else if !down {
            ui.ctx().data_mut(|d| d.remove::<usize>(drag_id));
        }
    }
    out
}

/// Page tabs under the page: Front / Inside / Back or Page 1, 2, ...
pub fn page_tabs(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    let pal = theme::palette(app.settings.look);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Pages:").weak());
        for (i, p) in app.doc.panels.clone().iter().enumerate() {
            let active = i == app.panel;
            let r = ui.add(egui::Button::selectable(active, RichText::new(&p.name).color(if active { pal.text } else { pal.muted })).min_size(vec2(70.0, 24.0)));
            if r.clicked() {
                out = Some(Act::Panel(i));
            }
            r.context_menu(|ui| page_menu(ui, app, i));
        }
        if ui.button("+ Page").on_hover_text("Add a page").clicked() {
            out = Some(Act::AddPanel);
        }
    });
    out
}

/// Rename, duplicate or delete a page (the page buttons' and the page
/// labels' right-click menu).
pub fn page_menu(ui: &mut egui::Ui, app: &mut App, i: usize) {
    let Some(name) = app.doc.panels.get(i).map(|p| p.name.clone()) else { return };
    if ui.button("Rename...").clicked() {
        app.dialogs.push(crate::dialogs::Dialog::RenamePage { index: i, name });
        ui.close();
    }
    if ui.button("Duplicate Page").clicked() {
        app.checkpoint();
        let mut copy = app.doc.panels[i].clone();
        copy.name = format!("{} copy", copy.name);
        for it in copy.items.iter_mut() {
            it.id = app.doc.fresh_id();
        }
        app.doc.panels.insert(i + 1, copy);
        app.guide_sel = None;
        ui.close();
    }
    if ui.add_enabled(app.doc.panels.len() > 1, egui::Button::new("Delete Page")).clicked() {
        app.checkpoint();
        app.doc.panels.remove(i);
        app.panel = app.panel.min(app.doc.panels.len() - 1);
        app.selected.clear();
        app.guide_sel = None;
        ui.close();
    }
    ui.separator();
    let guides = app.doc.panels.get(i).is_some_and(|p| !p.guides.is_empty());
    if ui.add_enabled(guides, egui::Button::new("Remove Guidelines")).clicked() {
        app.checkpoint();
        app.doc.panels[i].guides.clear();
        app.guide_sel = None;
        ui.close();
    }
}

pub fn status_bar(ui: &mut egui::Ui, app: &mut App) -> Option<Act> {
    let mut out = None;
    ui.horizontal(|ui| {
        if icons::button(ui, Icon::ZoomOut, "Zoom out", true).clicked() {
            out = Some(Act::ZoomOut);
        }
        ui.label(format!("{:.0}%", app.zoom * 100.0));
        if icons::button(ui, Icon::ZoomIn, "Zoom in", true).clicked() {
            out = Some(Act::ZoomIn);
        }
        if ui.button("Fit").on_hover_text("Fit the page in the window").clicked() {
            out = Some(Act::Fit);
        }
        ui.separator();
        let hint = if app.hint.is_empty() { "For help, press F1 · Search with Ctrl+K".to_string() } else { app.hint.clone() };
        ui.label(RichText::new(hint).weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let size = format!("{:.2} × {:.2} in", app.doc.width, app.doc.height);
            ui.label(RichText::new(size).weak());
            if app.doc.panels.len() > 1 {
                ui.separator();
                let name = app.page().name.clone();
                ui.label(format!("{name} · page {} of {}", app.panel + 1, app.doc.panels.len()));
            }
            if let Some((x, y)) = app.pointer_in {
                ui.separator();
                ui.label(format!("{x:.2}, {y:.2} in"));
            }
        });
    });
    out
}
