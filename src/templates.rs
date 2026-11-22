//! Ready-made designs to start from: PrintCraft's own, drawn with its
//! shapes, borders, headline effects and bundled fonts. Each has words to
//! change ("Your Name", "Date · Time · Place") and a size that Page Setup can
//! change later. New Project shows the ones for the chosen kind.

use crate::fonts::SCRIPT_FAMILY;
use crate::kinds::Kind;
use crate::model::*;

pub struct Design {
    pub name: &'static str,
    pub kind: Kind,
    pub build: fn() -> Project,
}

/// Every design, grouped by kind in New Project's order.
pub fn all() -> Vec<Design> {
    let d = |name, kind, build: fn() -> Project| Design { name, kind, build };
    vec![
        d("Bake Sale", Kind::Sign, crate::sample::poster),
        d("Yard Sale", Kind::Sign, yard_sale),
        d("Birthday Party", Kind::Sign, party_sign),
        d("Garden Party", Kind::Sign, garden_party),
        d("Welcome", Kind::Sign, welcome_sign),
        d("Lost Pet", Kind::Sign, lost_pet),
        d("Holiday Party", Kind::Sign, holiday_party),
        d("Happy Birthday", Kind::CardQuarter, birthday_card),
        d("Thank You", Kind::CardQuarter, thank_you_card),
        d("Season's Greetings", Kind::CardQuarter, holiday_card),
        d("Get Well Soon", Kind::CardQuarter, get_well_card),
        d("Congratulations", Kind::CardQuarter, congrats_card),
        d("Thinking of You", Kind::CardQuarter, thinking_card),
        d("Happy Anniversary", Kind::CardQuarter, anniversary_card),
        d("You're Invited", Kind::CardHalf, invitation_card),
        d("Birthday Flowers", Kind::CardHalf, flower_birthday_card),
        d("Floral Note", Kind::NoteCard, floral_note),
        d("Monogram Note", Kind::NoteCard, monogram_note),
        d("Certificate of Achievement", Kind::Certificate, achievement_certificate),
        d("Certificate of Appreciation", Kind::Certificate, appreciation_certificate),
        d("Happy Birthday Banner", Kind::Banner, birthday_banner),
        d("Welcome Home Banner", Kind::Banner, welcome_banner),
        d("Mailing Labels (address merge)", Kind::Label, mailing_label),
        d("Return Address", Kind::Label, return_label),
        d("Homemade Jar Label", Kind::Label, jar_label),
        d("Addressed Envelope (address merge)", Kind::Envelope, envelope),
        d("Simple Business Card", Kind::BusinessCard, business_card),
        d("Family Letterhead", Kind::Letterhead, letterhead),
        d("Family Newsletter", Kind::Newsletter, newsletter),
        d("Greetings Postcard", Kind::Postcard, postcard),
        d("Photo Memories Page", Kind::Scrapbook, scrapbook),
    ]
}

pub fn for_kind(kind: Kind) -> Vec<Design> {
    all().into_iter().filter(|d| d.kind == kind).collect()
}

// ----------------------------------------------------------------- builder

fn c(hex: &str) -> Rgba {
    if hex.is_empty() { Rgba::NONE } else { Rgba::hex(hex) }
}

struct B {
    p: Project,
    pg: usize,
}

impl B {
    fn new(kind: Kind, landscape: bool) -> Self {
        Self { p: Project::new(kind, landscape), pg: 0 }
    }
    fn w(&self) -> f32 {
        self.p.width
    }
    fn h(&self) -> f32 {
        self.p.height
    }
    fn page(&mut self, i: usize) -> &mut Self {
        self.pg = i.min(self.p.panels.len() - 1);
        self
    }
    fn bg(&mut self, color: &str) -> &mut Self {
        self.p.panels[self.pg].background = c(color);
        self
    }
    fn pattern(&mut self, n: u8, color: &str) -> &mut Self {
        self.p.panels[self.pg].pattern = n;
        self.p.panels[self.pg].pattern_color = c(color);
        self
    }
    fn add(&mut self, f: Frame, content: Content) -> &mut Item {
        let id = self.p.fresh_id();
        let pg = self.pg;
        self.p.panels[pg].items.push(Item::new(id, f, content));
        self.p.panels[pg].items.last_mut().unwrap()
    }
    /// A page border `inset` inches from the edge.
    fn border(&mut self, style: u8, line: &str, fill: &str, inset: f32, pt: f32) -> &mut Item {
        let (w, h) = (self.w(), self.h());
        let it = self.add(Frame::new(inset, inset, w - 2.0 * inset, h - 2.0 * inset), Content::Border { style });
        it.stroke = c(line);
        it.fill = c(fill);
        it.stroke_pt = pt;
        it
    }
    fn shape(&mut self, o: Outline, x: f32, y: f32, w: f32, h: f32, fill: &str, stroke: &str) -> &mut Item {
        let it = self.add(Frame::new(x, y, w, h), Content::Shape { outline: o });
        it.fill = c(fill);
        it.stroke = c(stroke);
        it.stroke_pt = if stroke.is_empty() { 0.0 } else { 1.5 };
        it
    }
    #[allow(clippy::too_many_arguments)]
    fn head(&mut self, text: &str, x: f32, y: f32, w: f32, h: f32, color: &str, second: &str, flair: Flair, warp: Warp, script: bool) -> &mut Item {
        let look = TextLook { color: c(color), bold: !script, align: Align::Center, family: if script { SCRIPT_FAMILY.into() } else { crate::fonts::DEFAULT_FAMILY.into() }, ..Default::default() };
        self.add(Frame::new(x, y, w, h), Content::Headline { text: text.into(), look, flair, warp, second: c(second), outline: c(second), stretch: false, bend: 1.0, bulge: 0.0 })
    }
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, text: &str, x: f32, y: f32, w: f32, h: f32, size: f32, color: &str, align: Align) -> &mut Item {
        let look = TextLook { size, color: c(color), align, valign: VAlign::Middle, ..Default::default() };
        self.add(Frame::new(x, y, w, h), Content::Text { text: text.into(), look, shape: None, fit: false, runs: Vec::new() })
    }
    #[allow(clippy::too_many_arguments)]
    fn script(&mut self, text: &str, x: f32, y: f32, w: f32, h: f32, size: f32, color: &str) -> &mut Item {
        let look = TextLook { size, color: c(color), align: Align::Center, valign: VAlign::Middle, family: SCRIPT_FAMILY.into(), ..Default::default() };
        self.add(Frame::new(x, y, w, h), Content::Text { text: text.into(), look, shape: None, fit: false, runs: Vec::new() })
    }
    fn line(&mut self, x: f32, y: f32, w: f32, color: &str, pt: f32) -> &mut Item {
        let it = self.add(Frame::new(x, y, w, 0.02), Content::Line);
        it.stroke = c(color);
        it.stroke_pt = pt;
        it
    }
    /// A balloon: an oval with a string.
    fn balloon(&mut self, x: f32, y: f32, s: f32, fill: &str) {
        // The string: a line turned upright, hanging from the balloon.
        let (cx, top, len) = (x + s * 0.5, y + s * 1.18, s * 1.3);
        let it = self.add(Frame::new(cx - len / 2.0, top + len / 2.0, len, 0.02), Content::Line);
        it.stroke = c("#8a8a8a");
        it.stroke_pt = 1.0;
        it.rotation = 90.0;
        let it = self.shape(Outline::Ellipse, x, y, s, s * 1.2, fill, "");
        it.shadow = Some(Shadow { softness: 0.4, ..Shadow::new(Rgba(0, 0, 0, 60)) });
        self.shape(Outline::Ellipse, x + s * 0.2, y + s * 0.18, s * 0.18, s * 0.26, "#ffffff", "").opacity = 0.55;
    }
    /// A simple flower: five petals and a middle.
    fn flower(&mut self, cx: f32, cy: f32, r: f32, petal: &str, middle: &str) {
        for k in 0..5 {
            let a = k as f32 / 5.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let (px, py) = (cx + a.cos() * r * 0.55, cy + a.sin() * r * 0.55);
            self.shape(Outline::Ellipse, px - r * 0.42, py - r * 0.42, r * 0.84, r * 0.84, petal, "");
        }
        self.shape(Outline::Ellipse, cx - r * 0.3, cy - r * 0.3, r * 0.6, r * 0.6, middle, "");
    }
    /// The usual back of a card.
    fn card_back(&mut self, panel: usize, accent: &str) {
        self.page(panel);
        let (w, h) = (self.w(), self.h());
        self.shape(Outline::Heart, w / 2.0 - 0.18, h * 0.62, 0.36, 0.32, accent, "");
        self.script("Made especially for you\nby Your Name", 0.3, h * 0.7, w - 0.6, 0.8, 14.0, "#555555");
    }
    fn done(&mut self) -> Project {
        std::mem::replace(&mut self.p, Project::new(Kind::Sign, false))
    }
}

// ------------------------------------------------------------------- signs

fn yard_sale() -> Project {
    let mut b = B::new(Kind::Sign, false);
    b.bg("#fff3b0").border(9, "#222222", "", 0.3, 6.0);
    b.head("YARD SALE", 0.6, 0.9, 7.3, 2.0, "#d62828", "#111111", Flair::ShadowRight, Warp::Straight, false);
    b.shape(Outline::Arrow, 2.4, 3.3, 3.7, 1.6, "#2a9d8f", "#1d6f66");
    b.text("Saturday & Sunday\n8 am to 2 pm", 0.8, 5.4, 6.9, 1.4, 30.0, "#222222", Align::Center).content_bold();
    b.text("123 Your Street", 0.8, 6.9, 6.9, 0.8, 24.0, "#222222", Align::Center);
    b.text("Furniture · Toys · Books · Clothes · Kitchen", 0.8, 8.0, 6.9, 0.8, 18.0, "#444444", Align::Center);
    b.text("Everything must go!", 0.8, 9.0, 6.9, 0.8, 22.0, "#d62828", Align::Center).content_italic();
    b.done()
}

fn party_sign() -> Project {
    let mut b = B::new(Kind::Sign, false);
    b.bg("#ffffff").pattern(11, "#f6c8d8").border(13, "#e84a5f", "#2a9df4", 0.3, 14.0);
    b.head("Birthday Party!", 0.6, 1.0, 7.3, 1.9, "#e84a5f", "#7b2cbf", Flair::BlendDown, Warp::ArchUp, false);
    for (x, col) in [(1.0, "#ff6b6b"), (3.3, "#ffd166"), (5.6, "#4ecdc4")] {
        b.balloon(x, 3.3, 1.7, col);
    }
    b.text("Join us to celebrate Your Name", 0.8, 7.2, 6.9, 0.8, 24.0, "#333333", Align::Center);
    b.text("Date · Time · Place", 0.8, 8.1, 6.9, 0.7, 20.0, "#555555", Align::Center);
    b.text("Cake, games and fun for everyone!", 0.8, 8.9, 6.9, 0.7, 18.0, "#7b2cbf", Align::Center).content_italic();
    b.done()
}

fn garden_party() -> Project {
    let mut b = B::new(Kind::Sign, false);
    b.bg("#f6fbf2").border(11, "#4f8a3c", "#8cc084", 0.3, 16.0);
    b.script("Garden Party", 0.6, 1.0, 7.3, 1.8, 72.0, "#3d7a46");
    for (x, y, pc) in [(1.4, 3.7, "#f4a6c0"), (4.25, 3.4, "#f7d26a"), (7.1, 3.7, "#c7a6f4")] {
        b.flower(x, y, 0.8, pc, "#f29e38");
    }
    b.text("Please join us for lunch among the roses", 0.8, 5.2, 6.9, 0.9, 22.0, "#2d4a2f", Align::Center);
    b.text("Saturday, June 6 · 12 noon\nYour Name's garden, 123 Your Street", 0.8, 6.3, 6.9, 1.3, 20.0, "#2d4a2f", Align::Center);
    b.text("Bring a dish to share", 0.8, 8.2, 6.9, 0.7, 18.0, "#4f8a3c", Align::Center).content_italic();
    b.done()
}

fn welcome_sign() -> Project {
    let mut b = B::new(Kind::Sign, false);
    b.bg("#fffaf2").border(6, "#c97b63", "#f2c6b4", 0.3, 18.0);
    b.script("Welcome", 0.5, 2.6, 7.5, 2.4, 110.0, "#8c4a3a");
    b.line(2.25, 5.3, 4.0, "#c97b63", 2.0);
    b.text("to our home", 0.8, 5.5, 6.9, 0.8, 28.0, "#8c4a3a", Align::Center);
    for (x, y) in [(1.5, 7.6), (4.25, 8.0), (7.0, 7.6)] {
        b.flower(x, y, 0.6, "#f2a7a0", "#f6d365");
    }
    b.text("The Your Name Family", 0.8, 9.1, 6.9, 0.7, 20.0, "#8c4a3a", Align::Center);
    b.done()
}

fn lost_pet() -> Project {
    let mut b = B::new(Kind::Sign, false);
    b.bg("#ffffff");
    b.head("LOST CAT", 0.5, 0.4, 7.5, 1.6, "#c1121f", "#5a0b10", Flair::Plain, Warp::Straight, false);
    let it = b.text("Put a photo of your pet here\n(Pictures on the left, or paste one in)", 1.25, 2.2, 6.0, 4.5, 16.0, "#888888", Align::Center);
    if let Content::Text { shape, .. } = &mut it.content {
        *shape = Some(Outline::Rounded);
    }
    it.fill = c("#f3f3f3");
    it.stroke = c("#bbbbbb");
    it.stroke_pt = 1.5;
    b.text("Name: Whiskers", 0.8, 6.9, 6.9, 0.7, 26.0, "#222222", Align::Center).content_bold();
    b.text("Gray and white, green eyes, very friendly.\nLast seen near Your Street on Date.", 0.8, 7.6, 6.9, 1.2, 18.0, "#333333", Align::Center);
    b.text("Please call 555-123-4567", 0.8, 9.0, 6.9, 0.9, 30.0, "#c1121f", Align::Center).content_bold();
    b.done()
}

fn holiday_party() -> Project {
    let mut b = B::new(Kind::Sign, false);
    b.bg("#14325c").border(5, "#f2c94c", "#ffffff", 0.3, 14.0);
    for (x, y, s) in [(1.1, 1.0, 0.7), (6.5, 1.3, 0.5), (3.9, 0.7, 0.4), (1.6, 8.6, 0.5), (6.3, 8.4, 0.8)] {
        b.shape(Outline::Star8, x, y, s, s, "#ffffff", "").opacity = 0.85;
    }
    b.script("Holiday Party", 0.5, 2.0, 7.5, 1.9, 80.0, "#f2c94c");
    b.shape(Outline::Star, 3.6, 4.2, 1.3, 1.3, "#f2c94c", "").glow = Some(c("#fff2b0"));
    b.text("Friday, December 18 · 7 pm", 0.8, 6.0, 6.9, 0.8, 26.0, "#ffffff", Align::Center);
    b.text("Your Name's House\n123 Your Street", 0.8, 6.9, 6.9, 1.2, 22.0, "#dce6f5", Align::Center);
    b.text("Food · Music · Good company", 0.8, 8.0, 6.9, 0.6, 18.0, "#f2c94c", Align::Center).content_italic();
    b.done()
}

// ------------------------------------------------------------------- cards

fn birthday_card() -> Project {
    let mut b = B::new(Kind::CardQuarter, false);
    let (w, h) = (b.w(), b.h());
    b.page(0).bg("#fff7fb").pattern(11, "#f5c2d7");
    b.head("Happy Birthday!", 0.2, 0.35, w - 0.4, 1.2, "#e84a5f", "#7b2cbf", Flair::BlendDown, Warp::ArchUp, false);
    // A cake.
    b.shape(Outline::Rounded, 1.0, 3.2, 2.25, 1.0, "#f7b2c4", "#d16b8a");
    b.shape(Outline::Rounded, 1.25, 2.55, 1.75, 0.75, "#fde2ea", "#d16b8a");
    for x in [1.55, 2.08, 2.6] {
        b.shape(Outline::Rectangle, x, 2.1, 0.1, 0.45, "#4ecdc4", "");
        b.shape(Outline::Teardrop, x - 0.04, 1.85, 0.18, 0.25, "#ffb627", "");
    }
    b.shape(Outline::Rounded, 0.8, 4.15, 2.65, 0.15, "#c9c9c9", "");
    b.page(1);
    b.script("Wishing you a wonderful year\nfilled with joy and laughter!", 0.3, 1.8, w - 0.6, 1.4, 20.0, "#7b2cbf");
    b.script("Love, Your Name", 0.3, 3.6, w - 0.6, 0.7, 18.0, "#e84a5f");
    b.card_back(2, "#e84a5f");
    let _ = h;
    b.done()
}

fn thank_you_card() -> Project {
    let mut b = B::new(Kind::CardQuarter, false);
    let w = b.w();
    b.page(0).bg("#f4f8f3").border(11, "#6a9a5b", "#b5d3a7", 0.2, 10.0);
    b.script("Thank You", 0.3, 1.6, w - 0.6, 1.5, 54.0, "#3d6b35");
    for (x, pc) in [(1.2, "#f4a6c0"), (2.1, "#f7d26a"), (3.0, "#c7a6f4")] {
        b.flower(x, 3.9, 0.35, pc, "#f29e38");
    }
    b.page(1);
    b.script("Thank you so much for\nyour kindness.\nIt meant the world to me.", 0.3, 1.5, w - 0.6, 1.8, 19.0, "#3d6b35");
    b.script("Your Name", 0.3, 3.6, w - 0.6, 0.6, 18.0, "#555555");
    b.card_back(2, "#6a9a5b");
    b.done()
}

fn holiday_card() -> Project {
    let mut b = B::new(Kind::CardQuarter, false);
    let w = b.w();
    b.page(0).bg("#1e3a5f");
    for (x, y, s) in [(0.4, 0.4, 0.45), (3.2, 0.6, 0.35), (0.35, 2.7, 0.3), (3.4, 2.9, 0.4), (2.0, 0.3, 0.25)] {
        b.shape(Outline::Star8, x, y, s, s, "#ffffff", "").opacity = 0.8;
    }
    // A tree from three triangles.
    for (k, col) in ["#2f7d4f", "#3a925c", "#46a86a"].iter().enumerate() {
        let k = k as f32;
        b.shape(Outline::Triangle, 1.2 + k * 0.15, 1.3 + k * 0.6, 1.85 - k * 0.3, 1.2, col, "");
    }
    b.shape(Outline::Rectangle, 1.95, 3.55, 0.35, 0.35, "#7a4b2a", "");
    b.shape(Outline::Star, 1.88, 1.0, 0.5, 0.5, "#f2c94c", "");
    b.script("Season's Greetings", 0.2, 3.95, w - 0.4, 0.9, 30.0, "#f2c94c");
    b.page(1);
    b.script("Wishing you peace, joy\nand love this season\nand all year long.", 0.3, 1.5, w - 0.6, 1.8, 19.0, "#1e3a5f");
    b.script("Your Name", 0.3, 3.6, w - 0.6, 0.6, 18.0, "#b23a48");
    b.card_back(2, "#b23a48");
    b.done()
}

fn get_well_card() -> Project {
    let mut b = B::new(Kind::CardQuarter, false);
    let w = b.w();
    b.page(0).bg("#eaf6ff");
    b.shape(Outline::Burst, 1.1, 0.6, 2.0, 2.0, "#ffd35c", "#f2a93b");
    b.shape(Outline::Ellipse, 1.6, 1.1, 1.0, 1.0, "#ffe58a", "");
    b.shape(Outline::Cloud, 2.5, 2.2, 1.4, 0.8, "#ffffff", "#c9dff0");
    b.head("Get Well Soon", 0.2, 3.3, w - 0.4, 1.1, "#2b7bb9", "#1b4f78", Flair::SoftShadowRight, Warp::Straight, false);
    b.page(1);
    b.script("Sending sunshine and\nwarm wishes your way.\nFeel better soon!", 0.3, 1.5, w - 0.6, 1.8, 19.0, "#2b7bb9");
    b.script("Your Name", 0.3, 3.6, w - 0.6, 0.6, 18.0, "#555555");
    b.card_back(2, "#2b7bb9");
    b.done()
}

fn congrats_card() -> Project {
    let mut b = B::new(Kind::CardQuarter, false);
    let w = b.w();
    b.page(0).bg("#fffdf5").border(13, "#f2b631", "#e84a5f", 0.15, 10.0);
    b.shape(Outline::Star, 1.35, 0.7, 1.55, 1.55, "#f2b631", "#c78a12").shadow = Some(Shadow { softness: 0.5, ..Shadow::new(Rgba(0, 0, 0, 70)) });
    b.head("Congratulations!", 0.2, 2.6, w - 0.4, 1.0, "#c0392b", "#f2b631", Flair::SoftShadowRight, Warp::Straight, false);
    b.text("You did it!", 0.3, 3.8, w - 0.6, 0.6, 18.0, "#555555", Align::Center).content_italic();
    b.page(1);
    b.script("So proud of you\nand all you've achieved.\nHere's to what comes next!", 0.3, 1.5, w - 0.6, 1.8, 19.0, "#c0392b");
    b.script("Your Name", 0.3, 3.6, w - 0.6, 0.6, 18.0, "#555555");
    b.card_back(2, "#f2b631");
    b.done()
}

fn thinking_card() -> Project {
    let mut b = B::new(Kind::CardQuarter, false);
    let w = b.w();
    b.page(0).bg("#f5f3fa").border(0, "#9b8bbf", "", 0.25, 2.0);
    b.script("Thinking of You", 0.3, 1.0, w - 0.6, 1.2, 36.0, "#5d4b8a");
    for (x, y) in [(1.0, 2.9), (2.1, 2.6), (3.2, 2.9)] {
        b.shape(Outline::Teardrop, x - 0.2, y, 0.4, 0.6, "#a7c4a0", "").rotation = 180.0;
    }
    b.flower(2.12, 3.9, 0.45, "#c9b6e4", "#f6e7a1");
    b.page(1);
    b.script("With love and sympathy,\nholding you close\nin my heart.", 0.3, 1.5, w - 0.6, 1.8, 19.0, "#5d4b8a");
    b.script("Your Name", 0.3, 3.6, w - 0.6, 0.6, 18.0, "#555555");
    b.card_back(2, "#9b8bbf");
    b.done()
}

fn anniversary_card() -> Project {
    let mut b = B::new(Kind::CardQuarter, false);
    let w = b.w();
    b.page(0).bg("#fff8f8").border(4, "#d64f6a", "#f3b6c2", 0.2, 10.0);
    b.shape(Outline::Ring, 1.0, 1.0, 1.3, 1.3, "#e0b84a", "");
    b.shape(Outline::Ring, 1.9, 1.0, 1.3, 1.3, "#d1d1d1", "");
    b.script("Happy\nAnniversary", 0.3, 2.6, w - 0.6, 1.8, 36.0, "#b23a5a");
    b.page(1);
    b.script("Here's to love, laughter\nand happily ever after.", 0.3, 1.6, w - 0.6, 1.4, 19.0, "#b23a5a");
    b.script("With love, Your Name", 0.3, 3.6, w - 0.6, 0.6, 18.0, "#555555");
    b.card_back(2, "#d64f6a");
    b.done()
}

fn invitation_card() -> Project {
    let mut b = B::new(Kind::CardHalf, false);
    let (w, h) = (b.w(), b.h());
    b.page(0).bg("#fffaf0").border(14, "#2a6f97", "#f2b631", 0.25, 12.0);
    b.script("You're Invited!", 0.4, 1.4, w - 0.8, 1.6, 54.0, "#2a6f97");
    for (x, col) in [(1.2, "#ff6b6b"), (2.35, "#ffd166"), (3.5, "#4ecdc4")] {
        b.balloon(x, 3.6, 0.85, col);
    }
    b.text("to a celebration", 0.4, 6.4, w - 0.8, 0.7, 22.0, "#555555", Align::Center).content_italic();
    b.page(1);
    b.head("Please Join Us", 0.4, 0.8, w - 0.8, 1.0, "#2a6f97", "#1b4965", Flair::Plain, Warp::Straight, false);
    b.text("for Occasion\n\nDate\nTime\n\nPlace\nStreet, City", 0.5, 2.1, w - 1.0, 3.8, 20.0, "#333333", Align::Center);
    b.text("RSVP to Your Name · 555-123-4567", 0.5, h - 1.6, w - 1.0, 0.7, 16.0, "#2a6f97", Align::Center);
    b.card_back(2, "#2a6f97");
    b.done()
}

fn flower_birthday_card() -> Project {
    let mut b = B::new(Kind::CardHalf, false);
    let w = b.w();
    b.page(0).bg("#fffdf8");
    for (x, y, r, pc) in [(1.4, 2.0, 0.7, "#f4a6c0"), (2.75, 1.6, 0.85, "#f7d26a"), (4.1, 2.1, 0.7, "#c7a6f4"), (2.0, 3.2, 0.55, "#9fd8cb"), (3.5, 3.3, 0.6, "#f8b195")] {
        b.flower(x, y, r, pc, "#f29e38");
    }
    b.script("Happy Birthday", 0.3, 4.6, w - 0.6, 1.4, 50.0, "#b5446e");
    b.text("to someone special", 0.4, 6.1, w - 0.8, 0.6, 20.0, "#777777", Align::Center).content_italic();
    b.page(1);
    b.script("May your day bloom\nwith happiness!", 0.4, 2.2, w - 0.8, 1.6, 26.0, "#b5446e");
    b.script("Love, Your Name", 0.4, 4.4, w - 0.8, 0.8, 20.0, "#555555");
    b.card_back(2, "#b5446e");
    b.done()
}

fn floral_note() -> Project {
    let mut b = B::new(Kind::NoteCard, true);
    let (w, h) = (b.w(), b.h());
    b.page(0).bg("#fffdf8").border(4, "#d98ca3", "#f6d6df", 0.2, 8.0);
    for (x, pc) in [(w * 0.3, "#f4a6c0"), (w * 0.5, "#f7d26a"), (w * 0.7, "#c7a6f4")] {
        b.flower(x, h * 0.42, 0.45, pc, "#f29e38");
    }
    b.script("A note for you", 0.3, h * 0.65, w - 0.6, 0.8, 26.0, "#a35a74");
    b.page(1);
    b.script("Dear ,", 0.4, 0.4, w - 0.8, 0.6, 18.0, "#555555");
    b.card_back(2, "#d98ca3");
    b.done()
}

fn monogram_note() -> Project {
    let mut b = B::new(Kind::NoteCard, true);
    let (w, h) = (b.w(), b.h());
    b.page(0).bg("#f7f7f2");
    b.shape(Outline::Ellipse, w / 2.0 - 1.0, h / 2.0 - 1.0, 2.0, 2.0, "", "#2f4858").stroke_pt = 2.0;
    b.script("A", w / 2.0 - 1.0, h / 2.0 - 0.85, 2.0, 1.7, 72.0, "#2f4858");
    b.text("THE YOUR NAME FAMILY", 0.3, h - 0.75, w - 0.6, 0.4, 11.0, "#2f4858", Align::Center);
    b.page(1);
    b.card_back(2, "#2f4858");
    b.done()
}

// ------------------------------------------------------------ certificates

fn certificate(title: &str, line: &str, border: u8, ink: &str, light: &str) -> Project {
    let mut b = B::new(Kind::Certificate, true);
    let (w, h) = (b.w(), b.h());
    b.bg("#fffdf6").border(border, ink, light, 0.35, 14.0);
    b.head(title, 1.0, 0.9, w - 2.0, 1.2, ink, "#000000", Flair::Plain, Warp::Straight, false);
    b.text("is proudly presented to", 1.0, 2.3, w - 2.0, 0.5, 18.0, "#555555", Align::Center).content_italic();
    b.script("Name Here", 1.0, 2.9, w - 2.0, 1.3, 60.0, "#222222");
    b.line(2.5, 4.25, w - 5.0, ink, 1.5);
    b.text(line, 1.5, 4.45, w - 3.0, 0.9, 18.0, "#333333", Align::Center);
    let seal = b.text("Award\nof\nMerit", w / 2.0 - 0.8, 5.5, 1.6, 1.6, 14.0, "#5a3b00", Align::Center);
    if let Content::Text { shape, look, .. } = &mut seal.content {
        *shape = Some(Outline::Burst);
        look.bold = true;
    }
    seal.fill = c("#f2b631");
    seal.stroke = c("#c78a12");
    seal.stroke_pt = 1.5;
    for (x, label) in [(1.3, "Signature"), (w - 4.3, "Date")] {
        b.line(x, h - 1.4, 3.0, "#333333", 1.0);
        b.text(label, x, h - 1.3, 3.0, 0.4, 14.0, "#555555", Align::Center);
    }
    b.done()
}

fn achievement_certificate() -> Project {
    certificate("Certificate of Achievement", "for outstanding effort and accomplishment", 15, "#1f4e79", "#c9a227")
}

fn appreciation_certificate() -> Project {
    certificate("Certificate of Appreciation", "with heartfelt thanks for all you do", 12, "#7a2e3a", "#d4a24c")
}

// ----------------------------------------------------------------- banners

fn birthday_banner() -> Project {
    let mut b = B::new(Kind::Banner, true);
    let (w, h) = (b.w(), b.h());
    b.bg("#fffaf5").border(2, "#e84a5f", "#2a9df4", 0.25, 16.0);
    b.head("HAPPY BIRTHDAY!", 1.0, 1.4, w - 2.0, h - 2.8, "#e84a5f", "#7b2cbf", Flair::BlendDown, Warp::Straight, false);
    b.done()
}

fn welcome_banner() -> Project {
    let mut b = B::new(Kind::Banner, true);
    let (w, h) = (b.w(), b.h());
    b.bg("#f3f9ff").border(5, "#2f6fb0", "#f2b631", 0.25, 16.0);
    b.head("WELCOME HOME!", 1.0, 1.4, w - 2.0, h - 2.8, "#2f6fb0", "#123c69", Flair::ShadowRight, Warp::Straight, false);
    b.done()
}

// ------------------------------------------------------------------ labels

fn mailing_label() -> Project {
    let mut b = B::new(Kind::Label, false);
    let (w, h) = (b.w(), b.h());
    b.text("«Address»", 0.12, 0.08, w - 0.24, h - 0.16, 10.0, "#000000", Align::Left);
    b.done()
}

fn return_label() -> Project {
    let mut b = B::new(Kind::Label, false);
    let (w, h) = (b.w(), b.h());
    b.shape(Outline::Heart, 0.12, h / 2.0 - 0.18, 0.4, 0.36, "#e84a5f", "");
    b.text("Your Name\n123 Your Street\nYour City, ST 12345", 0.62, 0.06, w - 0.72, h - 0.12, 9.0, "#333333", Align::Left);
    b.done()
}

fn jar_label() -> Project {
    let mut b = B::new(Kind::Label, false);
    let (w, h) = (b.w(), b.h());
    let it = b.text("", 0.05, 0.05, w - 0.1, h - 0.1, 9.0, "", Align::Center);
    if let Content::Text { shape, .. } = &mut it.content {
        *shape = Some(Outline::Rounded);
    }
    it.fill = c("#fff3e0");
    it.stroke = c("#c0392b");
    it.stroke_pt = 1.0;
    b.script("Homemade Jam", 0.1, 0.06, w - 0.2, 0.55, 18.0, "#c0392b");
    b.text("Strawberry · Summer 2027", 0.1, 0.6, w - 0.2, 0.3, 8.0, "#7a4b2a", Align::Center);
    b.done()
}

// ------------------------------------------------- envelope, cards, papers

fn envelope() -> Project {
    let mut b = B::new(Kind::Envelope, true);
    let (w, h) = (b.w(), b.h());
    b.shape(Outline::Heart, 0.3, 0.32, 0.3, 0.28, "#e84a5f", "");
    b.text("Your Name\n123 Your Street\nYour City, ST 12345", 0.7, 0.25, 3.0, 0.75, 10.0, "#333333", Align::Left);
    b.text("«Address»", w * 0.42, h * 0.45, w * 0.5, 1.2, 14.0, "#000000", Align::Left);
    b.done()
}

fn business_card() -> Project {
    let mut b = B::new(Kind::BusinessCard, true);
    let (w, h) = (b.w(), b.h());
    b.shape(Outline::Rectangle, 0.0, 0.0, 0.18, h, "#2a9d8f", "");
    b.text("Your Name", 0.35, 0.3, w - 0.5, 0.45, 16.0, "#1d3557", Align::Left).content_bold();
    b.text("What You Do", 0.35, 0.72, w - 0.5, 0.3, 10.0, "#2a9d8f", Align::Left);
    b.text("555-123-4567\nyou@example.com\nYour City, ST", 0.35, 1.1, w - 0.5, 0.7, 9.0, "#444444", Align::Left);
    b.done()
}

fn letterhead() -> Project {
    let mut b = B::new(Kind::Letterhead, false);
    let (w, h) = (b.w(), b.h());
    b.script("The Your Name Family", 0.6, 0.45, w - 1.2, 0.9, 36.0, "#2f4858");
    b.text("123 Your Street · Your City, ST 12345 · 555-123-4567", 0.6, 1.3, w - 1.2, 0.4, 12.0, "#555555", Align::Center);
    b.line(0.75, 1.8, w - 1.5, "#86a8b8", 1.5);
    b.line(0.75, h - 0.9, w - 1.5, "#86a8b8", 1.0);
    b.flower(w / 2.0, h - 0.55, 0.22, "#c9dbe3", "#86a8b8");
    b.done()
}

fn newsletter() -> Project {
    let mut b = B::new(Kind::Newsletter, false);
    let (w, h) = (b.w(), b.h());
    b.shape(Outline::Rectangle, 0.4, 0.4, w - 0.8, 1.4, "#2f6fb0", "");
    b.head("Family News", 0.6, 0.5, w - 1.2, 1.0, "#ffffff", "#123c69", Flair::Plain, Warp::Straight, false);
    b.text("Season 2027 · Volume 1", 0.6, 1.45, w - 1.2, 0.3, 11.0, "#dbe8f6", Align::Center);
    let col = (w - 1.0) / 2.0;
    for (k, (title, body)) in [("What's New", "Write your news here. Births, weddings, new jobs, new homes and the little things that made you smile."), ("Coming Up", "Birthdays, reunions and get-togethers. Add the dates so everyone can plan ahead.")].iter().enumerate() {
        let x = 0.4 + k as f32 * (col + 0.2);
        b.text(title, x, 2.1, col, 0.5, 18.0, "#2f6fb0", Align::Left).content_bold();
        b.text(body, x, 2.65, col, 3.0, 12.0, "#333333", Align::Left);
    }
    let it = b.text("Put a photo here", 0.4, 6.0, w - 0.8, 3.0, 14.0, "#888888", Align::Center);
    if let Content::Text { shape, .. } = &mut it.content {
        *shape = Some(Outline::Rounded);
    }
    it.fill = c("#f3f3f3");
    it.stroke = c("#bbbbbb");
    b.text("From Your Name · you@example.com", 0.4, h - 0.9, w - 0.8, 0.4, 11.0, "#777777", Align::Center);
    b.done()
}

fn postcard() -> Project {
    let mut b = B::new(Kind::Postcard, true);
    let (w, h) = (b.w(), b.h());
    b.page(0).bg("#87c6e8");
    b.shape(Outline::Ellipse, w - 1.6, 0.3, 1.0, 1.0, "#ffd35c", "");
    b.shape(Outline::Rectangle, 0.0, h * 0.65, w, h * 0.35, "#e9d29c", "");
    b.shape(Outline::Cloud, 0.6, 0.5, 1.4, 0.7, "#ffffff", "");
    b.script("Greetings from", 0.3, 1.2, w - 0.6, 0.7, 26.0, "#1e5a8a");
    b.head("Your Town", 0.3, 1.8, w - 0.6, 1.0, "#d62828", "#7a1010", Flair::ShadowRight, Warp::ArchUp, false);
    b.page(1);
    // The divider between message and address: an upright line.
    let len = h - 0.8;
    b.line(w / 2.0 - len / 2.0, h / 2.0, len, "#999999", 1.0).rotation = 90.0;
    b.shape(Outline::Rectangle, w - 1.1, 0.3, 0.8, 0.95, "", "#999999");
    for k in 0..4 {
        b.line(w / 2.0 + 0.25, h * 0.45 + k as f32 * 0.4, w / 2.0 - 0.55, "#999999", 0.75);
    }
    b.done()
}

fn scrapbook() -> Project {
    let mut b = B::new(Kind::Scrapbook, false);
    let (w, _) = (b.w(), b.h());
    b.bg("#fbf6ec").pattern(10, "#d9c7a5");
    b.script("Our Summer", 0.5, 0.4, w - 1.0, 1.2, 64.0, "#7a4b2a");
    for (x, y, pw, ph, rot) in [(0.9, 2.0, 5.0, 3.8, -3.0), (6.2, 2.5, 4.8, 3.6, 2.5), (1.4, 6.6, 4.6, 3.6, 2.0), (6.6, 6.8, 4.4, 3.4, -2.0)] {
        let it = b.text("Photo", x, y, pw, ph, 14.0, "#999999", Align::Center);
        it.fill = c("#ffffff");
        it.stroke = c("#dddddd");
        it.rotation = rot;
        it.shadow = Some(Shadow { softness: 0.5, ..Shadow::new(Rgba(0, 0, 0, 70)) });
    }
    b.script("Write a memory here…", 0.8, 10.6, w - 1.6, 0.8, 26.0, "#7a4b2a");
    b.done()
}

// Small helpers for the bold/italic tweaks above.
trait Tweak {
    fn content_bold(&mut self) -> &mut Self;
    fn content_italic(&mut self) -> &mut Self;
}

impl Tweak for Item {
    fn content_bold(&mut self) -> &mut Self {
        if let Some(l) = self.look_mut() {
            l.bold = true;
        }
        self
    }
    fn content_italic(&mut self) -> &mut Self {
        if let Some(l) = self.look_mut() {
            l.italic = true;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_design_builds_with_its_own_kind() {
        for d in super::all() {
            let p = (d.build)();
            assert_eq!(p.kind, d.kind, "{}", d.name);
            assert!(p.panels.iter().any(|pn| !pn.items.is_empty()), "{} is empty", d.name);
        }
    }
}
