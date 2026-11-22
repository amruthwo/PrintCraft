//! The design document: pages ("panels") holding items. Lengths are inches,
//! angles degrees, font sizes points. Saved as JSON (".pcraft").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::kinds::Kind;

/// Banner sheets overlap by this at each join (inches), so the printer's
/// unprintable edge never cuts the design.
pub const BANNER_MARGIN: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const BLACK: Rgba = Rgba(0, 0, 0, 255);
    pub const WHITE: Rgba = Rgba(255, 255, 255, 255);
    pub const NONE: Rgba = Rgba(0, 0, 0, 0);
    pub fn hex(s: &str) -> Rgba {
        let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
        Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255)
    }
    pub fn is_clear(self) -> bool {
        self.3 == 0
    }
    pub fn with_alpha(self, a: u8) -> Rgba {
        Rgba(self.0, self.1, self.2, a)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Frame {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextLook {
    pub family: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Rgba,
    pub align: Align,
    pub valign: VAlign,
}

impl Default for TextLook {
    fn default() -> Self {
        Self {
            family: crate::fonts::DEFAULT_FAMILY.into(),
            size: 18.0,
            bold: false,
            italic: false,
            underline: false,
            color: Rgba::BLACK,
            align: Align::Left,
            valign: VAlign::Top,
        }
    }
}

/// Fancy-title effects, in the order the headline picker lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Flair {
    #[default]
    Plain,
    BlendDown,
    BlendRight,
    BlendLeft,
    RadialBlend,
    Outline,
    Ghost,
    Neon,
    Blur,
    ShadowRight,
    ShadowLeft,
    SoftShadowRight,
    SoftShadowLeft,
    StackRight,
    StackLeft,
    StackBottom,
    Perspective,
}

impl Flair {
    pub const ALL: [Flair; 17] = [
        Flair::Plain,
        Flair::BlendDown,
        Flair::BlendRight,
        Flair::BlendLeft,
        Flair::RadialBlend,
        Flair::Outline,
        Flair::Ghost,
        Flair::Neon,
        Flair::Blur,
        Flair::ShadowRight,
        Flair::ShadowLeft,
        Flair::SoftShadowRight,
        Flair::SoftShadowLeft,
        Flair::StackRight,
        Flair::StackLeft,
        Flair::StackBottom,
        Flair::Perspective,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Flair::Plain => "Plain",
            Flair::BlendDown => "Blend Down",
            Flair::BlendRight => "Blend Right",
            Flair::BlendLeft => "Blend Left",
            Flair::RadialBlend => "Radial Blend",
            Flair::Outline => "Outline",
            Flair::Ghost => "Ghost",
            Flair::Neon => "Neon",
            Flair::Blur => "Blur",
            Flair::ShadowRight => "Drop Shadow Right",
            Flair::ShadowLeft => "Drop Shadow Left",
            Flair::SoftShadowRight => "Soft Shadow Right",
            Flair::SoftShadowLeft => "Soft Shadow Left",
            Flair::StackRight => "Stack Right",
            Flair::StackLeft => "Stack Left",
            Flair::StackBottom => "Stack Bottom",
            Flair::Perspective => "Perspective",
        }
    }
}

/// Shape a headline's letters follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Warp {
    #[default]
    Straight,
    ArchUp,
    ArchDown,
    Wave,
    Bulge,
}

impl Warp {
    pub const ALL: [Warp; 5] = [Warp::Straight, Warp::ArchUp, Warp::ArchDown, Warp::Wave, Warp::Bulge];
    pub fn label(self) -> &'static str {
        match self {
            Warp::Straight => "Straight",
            Warp::ArchUp => "Arch Up",
            Warp::ArchDown => "Arch Down",
            Warp::Wave => "Wave",
            Warp::Bulge => "Bulge",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outline {
    Rectangle,
    Rounded,
    Ellipse,
    Triangle,
    Diamond,
    Star,
    Heart,
    Arrow,
    Burst,
    Balloon,
    Pentagon,
    Hexagon,
    Octagon,
    Parallelogram,
    Trapezoid,
    Cross,
    Star6,
    Star8,
    Moon,
    Cloud,
    Lightning,
    Ribbon,
    Shield,
    Tag,
    Chevron,
    Teardrop,
    Ring,
    Thought,
}

impl Outline {
    pub const ALL: [Outline; 28] = [
        Outline::Rectangle,
        Outline::Rounded,
        Outline::Ellipse,
        Outline::Triangle,
        Outline::Diamond,
        Outline::Pentagon,
        Outline::Hexagon,
        Outline::Octagon,
        Outline::Parallelogram,
        Outline::Trapezoid,
        Outline::Star,
        Outline::Star6,
        Outline::Star8,
        Outline::Burst,
        Outline::Heart,
        Outline::Cross,
        Outline::Arrow,
        Outline::Chevron,
        Outline::Moon,
        Outline::Cloud,
        Outline::Lightning,
        Outline::Ribbon,
        Outline::Shield,
        Outline::Tag,
        Outline::Teardrop,
        Outline::Ring,
        Outline::Balloon,
        Outline::Thought,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Outline::Rectangle => "Rectangle",
            Outline::Rounded => "Rounded Box",
            Outline::Ellipse => "Oval",
            Outline::Triangle => "Triangle",
            Outline::Diamond => "Diamond",
            Outline::Star => "Star",
            Outline::Heart => "Heart",
            Outline::Arrow => "Arrow",
            Outline::Burst => "Starburst",
            Outline::Balloon => "Speech Bubble",
            Outline::Pentagon => "Pentagon",
            Outline::Hexagon => "Hexagon",
            Outline::Octagon => "Octagon",
            Outline::Parallelogram => "Slanted Box",
            Outline::Trapezoid => "Trapezoid",
            Outline::Cross => "Plus / Cross",
            Outline::Star6 => "6-Point Star",
            Outline::Star8 => "8-Point Star",
            Outline::Moon => "Crescent Moon",
            Outline::Cloud => "Cloud",
            Outline::Lightning => "Lightning Bolt",
            Outline::Ribbon => "Ribbon Banner",
            Outline::Shield => "Shield",
            Outline::Tag => "Gift Tag",
            Outline::Chevron => "Chevron",
            Outline::Teardrop => "Teardrop",
            Outline::Ring => "Ring",
            Outline::Thought => "Thought Bubble",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Content {
    Text {
        text: String,
        look: TextLook,
        shape: Option<Outline>,
        /// Grow or shrink the words to fill the box or shape.
        #[serde(default)]
        fit: bool,
        /// Words styled differently from the rest of the box.
        #[serde(default)]
        runs: Vec<Span>,
    },
    Headline {
        text: String,
        look: TextLook,
        flair: Flair,
        warp: Warp,
        second: Rgba,
        outline: Rgba,
        /// Fill the box even if letters get wider or narrower than normal.
        #[serde(default)]
        stretch: bool,
        /// How strongly the arch/wave/bulge bends (1 = normal, negative flips).
        #[serde(default = "one")]
        bend: f32,
        /// Extra bulge (fatter middle, or pinched when negative) that mixes
        /// with any arch or wave.
        #[serde(default)]
        bulge: f32,
    },
    /// Several items moved and sized together; children use inches from
    /// the group's top-left corner at the size the group had when made.
    Group { items: Vec<Item>, base_w: f32, base_h: f32 },
    Shape { outline: Outline },
    Line,
    Picture { asset: u64 },
    Border { style: u8 },
    /// A decorative divider or flourish (see raster::ORNAMENT_NAMES),
    /// drawn in the line color with the fill color as its accent.
    Ornament { style: u8 },
}

/// A stretch of letters (character positions start..end) with its own style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub underline: Option<bool>,
    #[serde(default)]
    pub color: Option<Rgba>,
    #[serde(default)]
    pub size: Option<f32>,
}

/// The style one letter ends up with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Rgba,
    pub size: f32,
}

pub fn char_styles(look: &TextLook, runs: &[Span], n: usize) -> Vec<CharStyle> {
    let base = CharStyle { bold: look.bold, italic: look.italic, underline: look.underline, color: look.color, size: look.size };
    let mut v = vec![base; n];
    for r in runs {
        for s in v.iter_mut().take(r.end.min(n)).skip(r.start) {
            if let Some(b) = r.bold {
                s.bold = b;
            }
            if let Some(b) = r.italic {
                s.italic = b;
            }
            if let Some(b) = r.underline {
                s.underline = b;
            }
            if let Some(c) = r.color {
                s.color = c;
            }
            if let Some(z) = r.size {
                s.size = z;
            }
        }
    }
    v
}

/// Keep spans on the same words after the text is edited: everything after
/// the changed part moves by how much it grew or shrank.
pub fn shift_runs(runs: &mut Vec<Span>, old: &str, new: &str) {
    let a: Vec<char> = old.chars().collect();
    let b: Vec<char> = new.chars().collect();
    let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let post = a.iter().rev().zip(b.iter().rev()).take_while(|(x, y)| x == y).count().min(a.len().min(b.len()) - pre);
    let (old_end, new_end) = (a.len() - post, b.len() - post);
    let delta = new_end as isize - old_end as isize;
    let map = |i: usize| -> usize {
        if i <= pre {
            i
        } else if i >= old_end {
            (i as isize + delta).max(pre as isize) as usize
        } else {
            new_end.min(i)
        }
    };
    for r in runs.iter_mut() {
        r.start = map(r.start);
        r.end = map(r.end);
    }
    runs.retain(|r| r.end > r.start);
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    /// Older designs stored a fixed offset; kept so they still open.
    #[serde(default)]
    pub dx: f32,
    #[serde(default)]
    pub dy: f32,
    pub color: Rgba,
    #[serde(default)]
    pub soft: bool,
    /// Distance slider 0..1 (scaled by the item's size).
    #[serde(default = "shadow_distance")]
    pub distance: f32,
    /// Direction in degrees (0 = right, 90 = down).
    #[serde(default = "shadow_angle")]
    pub angle: f32,
    /// Softness slider 0..1 (0 = hard edge).
    #[serde(default)]
    pub softness: f32,
}

impl Shadow {
    pub fn new(color: Rgba) -> Self {
        Self {
            dx: 0.0,
            dy: 0.0,
            color,
            soft: false,
            distance: crate::effects::SHADOW_START_DISTANCE,
            angle: crate::effects::SHADOW_START_ANGLE,
            softness: crate::effects::SHADOW_START_SOFTNESS,
        }
    }
}

fn shadow_distance() -> f32 {
    crate::effects::SHADOW_START_DISTANCE
}
fn shadow_angle() -> f32 {
    crate::effects::SHADOW_START_ANGLE
}
fn glow_size() -> f32 {
    crate::effects::GLOW_START_SIZE
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: u64,
    pub frame: Frame,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    pub fill: Rgba,
    pub stroke: Rgba,
    pub stroke_pt: f32,
    #[serde(default = "full_tint")]
    pub tint: f32,
    #[serde(default)]
    pub shadow: Option<Shadow>,
    #[serde(default)]
    pub glow: Option<Rgba>,
    #[serde(default = "glow_size")]
    pub glow_size: f32,
    /// Frame style number (0 none, 1 thin ... 8 drop shadow); see effects.rs.
    #[serde(default)]
    pub frame_style: u8,
    /// How lines and outlines are drawn (see raster::LINE_STYLES): solid,
    /// dashed, dotted, double, wavy and so on.
    #[serde(default)]
    pub line_style: u8,
    #[serde(default)]
    pub locked: bool,
    /// 1 = solid, 0 = invisible.
    #[serde(default = "full_tint")]
    pub opacity: f32,
    pub content: Content,
}

fn one() -> f32 {
    1.0
}

fn full_tint() -> f32 {
    1.0
}

impl Item {
    pub fn new(id: u64, frame: Frame, content: Content) -> Self {
        Self {
            id,
            frame,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            fill: Rgba::NONE,
            stroke: Rgba::NONE,
            stroke_pt: 1.0,
            tint: 1.0,
            shadow: None,
            glow: None,
            glow_size: crate::effects::GLOW_START_SIZE,
            frame_style: 0,
            line_style: 0,
            locked: false,
            opacity: 1.0,
            content,
        }
    }
    pub fn kind_name(&self) -> &'static str {
        match self.content {
            Content::Text { .. } => "Text Box",
            Content::Headline { .. } => "Headline",
            Content::Shape { .. } => "Shape",
            Content::Line => "Line",
            Content::Picture { .. } => "Picture",
            Content::Border { .. } => "Border",
            Content::Ornament { .. } => "Flourish",
            Content::Group { .. } => "Group",
        }
    }
    pub fn look_mut(&mut self) -> Option<&mut TextLook> {
        match &mut self.content {
            Content::Text { look, .. } | Content::Headline { look, .. } => Some(look),
            _ => None,
        }
    }
    pub fn look(&self) -> Option<&TextLook> {
        match &self.content {
            Content::Text { look, .. } | Content::Headline { look, .. } => Some(look),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Panel {
    pub name: String,
    pub background: Rgba,
    pub items: Vec<Item>,
    /// Background pattern number (0 = plain); see raster::PATTERN_NAMES.
    #[serde(default)]
    pub pattern: u8,
    #[serde(default = "pattern_ink")]
    pub pattern_color: Rgba,
    /// Size of the background pattern (1 = normal; 0.5 finer, 2 bigger).
    #[serde(default = "one")]
    pub pattern_scale: f32,
    /// Alignment guidelines placed from the rulers. On screen only: they
    /// never print or export.
    #[serde(default)]
    pub guides: Vec<Guide>,
}

/// A guideline across the page. `vertical` lines come from the top ruler
/// and sit at `at` inches from the left; others from the left ruler, `at`
/// inches from the top.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub vertical: bool,
    pub at: f32,
}

/// Round to the nearest 1/16 inch (the finest ruler mark).
pub fn sixteenth(v: f32) -> f32 {
    (v * 16.0).round() / 16.0
}

/// "2 3/16 in" style reading of a ruler position.
pub fn inches_label(v: f32) -> String {
    let n = (v * 16.0).round() as i64;
    let neg = n < 0;
    let n = n.abs();
    let (whole, mut frac, mut den) = (n / 16, n % 16, 16);
    while frac > 0 && frac % 2 == 0 {
        frac /= 2;
        den /= 2;
    }
    let s = match (whole, frac) {
        (w, 0) => format!("{w}"),
        (0, f) => format!("{f}/{den}"),
        (w, f) => format!("{w} {f}/{den}"),
    };
    format!("{}{s} in", if neg { "-" } else { "" })
}

fn pattern_ink() -> Rgba {
    Rgba(120, 150, 200, 255)
}

/// How many copies of a small design go on a sheet of paper (labels,
/// business cards). Inches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetLayout {
    pub name: String,
    pub cols: u32,
    pub rows: u32,
    pub left: f32,
    pub top: f32,
    pub gap_x: f32,
    pub gap_y: f32,
    /// The paper the sheet is printed on (US Letter unless stated).
    #[serde(default = "letter_w")]
    pub paper_w: f32,
    #[serde(default = "letter_h")]
    pub paper_h: f32,
    /// Rounded label corners (inches; 0 = square). Drawn as a guide only.
    #[serde(default)]
    pub corner: f32,
    /// Round labels.
    #[serde(default)]
    pub round: bool,
}

fn letter_w() -> f32 {
    8.5
}
fn letter_h() -> f32 {
    11.0
}

/// A banner printed across several sheets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BannerSetup {
    pub sheets: u32,
    /// Sheets stand upright (taller banner) instead of lying sideways.
    pub tall: bool,
}

/// A picture the document carries. `bytes` are PNG/JPEG/BMP/GIF as loaded.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    #[serde(with = "b64")]
    pub bytes: Vec<u8>,
    #[serde(default)]
    pub credit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format: u32,
    pub kind: Kind,
    pub width: f32,
    pub height: f32,
    pub landscape: bool,
    pub panels: Vec<Panel>,
    pub assets: BTreeMap<u64, Asset>,
    pub next_id: u64,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub sheet: Option<SheetLayout>,
    #[serde(default)]
    pub banner: Option<BannerSetup>,
}

impl Project {
    pub fn new(kind: Kind, landscape: bool) -> Self {
        let (mut w, mut h) = kind.size();
        if landscape != kind.default_landscape() {
            std::mem::swap(&mut w, &mut h);
        }
        let panels = kind
            .panel_names()
            .iter()
            .map(|n| Panel { name: (*n).into(), background: Rgba::WHITE, items: Vec::new(), pattern: 0, pattern_color: pattern_ink(), pattern_scale: 1.0, guides: Vec::new() })
            .collect();
        let mut p = Self { format: 1, kind, width: w, height: h, landscape, panels, assets: BTreeMap::new(), next_id: 1, notes: Vec::new(), sheet: None, banner: None };
        if kind == Kind::Banner {
            p.set_banner(BannerSetup { sheets: 4, tall: false });
        }
        if let Some(l) = crate::kinds::sheet_presets(kind).first() {
            p.sheet = Some(l.layout.clone());
            p.width = l.w;
            p.height = l.h;
        }
        p
    }

    /// Usable length of one banner sheet (its long or short side, less the
    /// printer margin at both ends so nothing is lost where sheets overlap).
    pub fn banner_segment(b: &BannerSetup, margin: f32) -> (f32, f32) {
        let (along, across) = if b.tall { (8.5, 11.0) } else { (11.0, 8.5) };
        (along - 2.0 * margin, across)
    }

    pub fn set_banner(&mut self, b: BannerSetup) {
        let (seg, h) = Self::banner_segment(&b, BANNER_MARGIN);
        self.width = seg * b.sheets.max(1) as f32;
        self.height = h;
        self.banner = Some(b);
    }

    /// The sheet of paper the design prints on, for kinds whose page comes
    /// from a whole sheet (see `Kind::paper_use`).
    pub fn paper(&self) -> Option<(f32, f32)> {
        use crate::kinds::PaperUse;
        Some(match self.kind.paper_use()? {
            PaperUse::Whole => (self.width, self.height),
            PaperUse::Quarter => (self.width * 2.0, self.height * 2.0),
            PaperUse::Half if self.width <= self.height => (self.width * 2.0, self.height),
            PaperUse::Half => (self.width, self.height * 2.0),
        })
    }

    /// Change the page size. With `fit`, everything on every page is scaled
    /// evenly and centred so it still fits; borders keep their distance
    /// from the edge of the page.
    pub fn resize_pages(&mut self, w: f32, h: f32, fit: bool) {
        let (ow, oh) = (self.width, self.height);
        self.width = w;
        self.height = h;
        if !fit || ow <= 0.0 || oh <= 0.0 || ((w - ow).abs() < 1e-4 && (h - oh).abs() < 1e-4) {
            return;
        }
        let k = (w / ow).min(h / oh);
        let (dx, dy) = ((w - ow * k) / 2.0, (h - oh * k) / 2.0);
        for p in &mut self.panels {
            for it in &mut p.items {
                if matches!(it.content, Content::Border { .. }) {
                    it.frame.w = (w - (ow - it.frame.w)).max(0.1);
                    it.frame.h = (h - (oh - it.frame.h)).max(0.1);
                    continue;
                }
                it.frame = Frame::new(it.frame.x * k + dx, it.frame.y * k + dy, it.frame.w * k, it.frame.h * k);
                if let Content::Text { look, runs, .. } = &mut it.content {
                    look.size *= k;
                    for r in runs.iter_mut() {
                        if let Some(s) = &mut r.size {
                            *s *= k;
                        }
                    }
                } else if let Some(l) = it.look_mut() {
                    l.size *= k;
                }
            }
            for g in &mut p.guides {
                g.at = if g.vertical { g.at * k + dx } else { g.at * k + dy };
            }
        }
    }

    pub fn fresh_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_asset(&mut self, name: &str, bytes: Vec<u8>, credit: &str) -> u64 {
        let id = self.fresh_id();
        self.assets.insert(id, Asset { name: name.into(), bytes, credit: credit.into() });
        id
    }

    pub fn to_json(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("This file could not be read: {e}"))
    }
}

mod b64 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod guide_tests {
    use super::*;

    #[test]
    fn sixteenths_snap_and_read() {
        assert_eq!(sixteenth(2.19), 2.1875);
        assert_eq!(inches_label(2.1875), "2 3/16 in");
        assert_eq!(inches_label(0.5), "1/2 in");
        assert_eq!(inches_label(4.0), "4 in");
        assert_eq!(inches_label(-0.25), "-1/4 in");
    }

    #[test]
    fn older_files_without_guides_still_load() {
        let json = r#"{"name":"Front","background":[255,255,255,255],"items":[]}"#;
        let p: Panel = serde_json::from_str(json).unwrap();
        assert!(p.guides.is_empty());
    }
}

#[cfg(test)]
mod page_size_tests {
    use super::*;
    use crate::kinds::Kind;

    #[test]
    fn paper_matches_the_page() {
        assert_eq!(Project::new(Kind::Sign, false).paper(), Some((8.5, 11.0)));
        assert_eq!(Project::new(Kind::CardQuarter, false).paper(), Some((8.5, 11.0)));
        assert_eq!(Project::new(Kind::CardHalf, false).paper(), Some((11.0, 8.5)));
        assert_eq!(Project::new(Kind::Label, false).paper(), None);
    }

    #[test]
    fn resizing_scales_and_centres() {
        let mut p = Project::new(Kind::Sign, false);
        let look = TextLook { size: 20.0, ..Default::default() };
        p.panels[0].items.push(Item::new(1, Frame::new(1.0, 1.0, 2.0, 2.0), Content::Text { text: "Hi".into(), look, shape: None, fit: false, runs: Vec::new() }));
        p.panels[0].items.push(Item::new(2, Frame::new(0.25, 0.25, 8.0, 10.5), Content::Border { style: 0 }));
        // Letter to Tabloid: twice as tall would overflow sideways, so it grows by 11/8.5 and is centred down the page.
        p.resize_pages(11.0, 17.0, true);
        let k = 11.0 / 8.5;
        let t = &p.panels[0].items[0];
        assert!((t.frame.w - 2.0 * k).abs() < 1e-4);
        assert!((t.frame.y - (1.0 * k + (17.0 - 11.0 * k) / 2.0)).abs() < 1e-4);
        assert!((t.look().unwrap().size - 20.0 * k).abs() < 1e-3);
        // The border keeps its quarter-inch margins.
        let b = &p.panels[0].items[1];
        assert!((b.frame.w - 10.5).abs() < 1e-4 && (b.frame.h - 16.5).abs() < 1e-4);
    }
}
