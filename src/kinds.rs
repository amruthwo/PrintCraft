//! Project kinds offered by "New Project", with their page sizes (inches) and
//! panels. Sizes: see docs/DESIGN.md "Project sizes".

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    Sign,
    Banner,
    CardQuarter,
    CardHalf,
    Calendar,
    Label,
    Envelope,
    BusinessCard,
    Certificate,
    NoteCard,
    Letterhead,
    Newsletter,
    Brochure,
    Postcard,
    FaxCover,
    Scrapbook,
    Sticker,
    IronOn,
}

impl Kind {
    pub const ALL: [Kind; 18] = [
        Kind::CardQuarter,
        Kind::CardHalf,
        Kind::Sign,
        Kind::Banner,
        Kind::Calendar,
        Kind::Certificate,
        Kind::Label,
        Kind::Envelope,
        Kind::BusinessCard,
        Kind::NoteCard,
        Kind::Postcard,
        Kind::Letterhead,
        Kind::Newsletter,
        Kind::Brochure,
        Kind::FaxCover,
        Kind::Scrapbook,
        Kind::Sticker,
        Kind::IronOn,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Sign => "Sign / Poster",
            Kind::Banner => "Banner",
            Kind::CardQuarter => "Greeting Card (quarter-fold)",
            Kind::CardHalf => "Greeting Card (half-fold)",
            Kind::Calendar => "Calendar",
            Kind::Label => "Label",
            Kind::Envelope => "Envelope",
            Kind::BusinessCard => "Business Card",
            Kind::Certificate => "Certificate",
            Kind::NoteCard => "Note Card",
            Kind::Letterhead => "Letterhead",
            Kind::Newsletter => "Newsletter",
            Kind::Brochure => "Brochure",
            Kind::Postcard => "Postcard",
            Kind::FaxCover => "Fax Cover",
            Kind::Scrapbook => "Scrapbook Page",
            Kind::Sticker => "Sticker",
            Kind::IronOn => "Iron-On Transfer",
        }
    }

    /// Page size (width, height) in the kind's usual orientation.
    pub fn size(self) -> (f32, f32) {
        match self {
            Kind::Sign | Kind::Letterhead | Kind::Newsletter | Kind::FaxCover | Kind::Sticker | Kind::IronOn => (8.5, 11.0),
            Kind::Banner => (44.0, 8.5),
            Kind::CardQuarter => (4.25, 5.5),
            Kind::CardHalf => (5.5, 8.5),
            Kind::Calendar | Kind::Certificate | Kind::Brochure => (11.0, 8.5),
            Kind::Label => (2.625, 1.0),
            Kind::Envelope => (9.5, 4.125),
            Kind::BusinessCard => (3.5, 2.0),
            Kind::NoteCard => (5.5, 4.25),
            Kind::Postcard => (6.0, 4.25),
            Kind::Scrapbook => (12.0, 12.0),
        }
    }

    pub fn default_landscape(self) -> bool {
        let (w, h) = self.size();
        w > h
    }

    pub fn panel_names(self) -> &'static [&'static str] {
        match self {
            Kind::CardQuarter | Kind::CardHalf | Kind::NoteCard => &["Front", "Inside", "Back"],
            Kind::Brochure => &["Outside", "Inside"],
            Kind::Postcard => &["Front", "Back"],
            _ => &["Page 1"],
        }
    }

    /// Native file extension per kind is one shared ".pcraft"; the legacy
    /// extension is the one older project files of this kind use.
    pub fn legacy_extension(self) -> &'static str {
        match self {
            Kind::Sign => "sig",
            Kind::Banner => "ban",
            Kind::CardQuarter => "car",
            Kind::CardHalf => "hcr",
            Kind::Calendar => "cal",
            Kind::Label => "lbl",
            Kind::Envelope => "env",
            Kind::BusinessCard => "biz",
            Kind::Certificate => "cer",
            Kind::NoteCard => "not",
            Kind::Letterhead => "let",
            Kind::Newsletter => "nws",
            Kind::Brochure => "bro",
            Kind::Postcard => "pcr",
            Kind::FaxCover => "fax",
            Kind::Scrapbook => "sbp",
            Kind::Sticker => "sti",
            Kind::IronOn => "tsh",
        }
    }

    pub fn from_legacy_extension(ext: &str) -> Option<Kind> {
        let ext = ext.to_ascii_lowercase();
        Kind::ALL.into_iter().find(|k| k.legacy_extension() == ext)
    }

    /// One color per kind for the picker tiles.
    pub fn tile_color(self) -> [u8; 3] {
        match self {
            Kind::CardQuarter | Kind::CardHalf | Kind::NoteCard => [235, 106, 92],
            Kind::Sign | Kind::Certificate | Kind::Scrapbook => [62, 151, 214],
            Kind::Banner | Kind::Calendar => [245, 185, 60],
            Kind::Label | Kind::Envelope | Kind::BusinessCard | Kind::Sticker => [88, 186, 150],
            _ => [150, 112, 210],
        }
    }

    pub fn legacy_extensions() -> Vec<&'static str> {
        Kind::ALL.iter().map(|k| k.legacy_extension()).collect()
    }
}

use crate::model::SheetLayout;

/// A ready-made paper layout (standard published sheet sizes).
pub struct Preset {
    pub label: &'static str,
    pub w: f32,
    pub h: f32,
    pub layout: SheetLayout,
}

/// One sheet layout: label size, how many across and down, where the first
/// one starts and the step from one to the next (inches).
struct Sheet {
    label: &'static str,
    w: f32,
    h: f32,
    cols: u32,
    rows: u32,
    left: f32,
    top: f32,
    step_x: f32,
    step_y: f32,
    corner: f32,
    round: bool,
    paper: (f32, f32),
}

const LETTER: (f32, f32) = (8.5, 11.0);
const A4: (f32, f32) = (8.26805, 11.6931);

#[allow(clippy::too_many_arguments)]
const fn sheet(label: &'static str, w: f32, h: f32, cols: u32, rows: u32, left: f32, top: f32, step_x: f32, step_y: f32, corner: f32) -> Sheet {
    Sheet { label, w, h, cols, rows, left, top, step_x, step_y, corner, round: false, paper: LETTER }
}

const fn round(s: Sheet) -> Sheet {
    Sheet { round: true, ..s }
}

const fn on(paper: (f32, f32), s: Sheet) -> Sheet {
    Sheet { paper, ..s }
}

impl Sheet {
    fn preset(&self) -> Preset {
        Preset {
            label: self.label,
            w: self.w,
            h: self.h,
            layout: SheetLayout {
                name: self.label.into(),
                cols: self.cols,
                rows: self.rows,
                left: self.left,
                top: self.top,
                gap_x: (self.step_x - self.w).max(0.0),
                gap_y: (self.step_y - self.h).max(0.0),
                paper_w: self.paper.0,
                paper_h: self.paper.1,
                corner: self.corner,
                round: self.round,
            },
        }
    }
}

// Sheet geometry (inches): label size, places across × down, first label's
// left and top edge, and the step to the next label. The numbers follow the
// published sheet sizes, with the labels centred across the page; the
// product number is the familiar stock number printed on the package. See
// docs/DESIGN.md "Paper sheets".
const LABEL_SHEETS: [Sheet; 17] = [
    sheet("Address 2⅝ × 1 in, 30 per sheet (5160 size)", 2.625, 1.0, 3, 10, 0.1875, 0.5, 2.75, 1.0, 0.1875),
    sheet("Address 2⅝ × 1 in, 30 per sheet (8160 size)", 2.625, 1.0, 3, 10, 0.1875, 0.5, 2.75, 1.0, 0.25),
    sheet("Address 4 × 1 in, 20 per sheet (5161 size)", 4.0, 1.0, 2, 10, 0.15625, 0.5, 4.1875, 1.0, 0.1875),
    sheet("Address 4 × 1⅓ in, 14 per sheet (5162 size)", 4.0, 1.33333, 2, 7, 0.15625, 0.83333, 4.1875, 1.33333, 0.1875),
    sheet("Address 4 × 1½ in, 14 per sheet (5159 size)", 4.0, 1.5, 2, 7, 0.15625, 0.25, 4.1875, 1.5, 0.1875),
    sheet("Shipping 4 × 2 in, 10 per sheet (5163 size)", 4.0, 2.0, 2, 5, 0.15625, 0.5, 4.1875, 2.0, 0.375),
    sheet("Shipping 4 × 3⅓ in, 6 per sheet (5164 size)", 4.0, 3.33333, 2, 3, 0.15625, 0.5, 4.1875, 3.33333, 0.375),
    sheet("Shipping 3½ × 5 in, 4 per sheet (5168 size)", 3.5, 5.0, 2, 2, 0.5, 0.5, 4.0, 5.0, 0.09375),
    sheet("Return address 1¾ × ½ in, 80 per sheet (5267 size)", 1.75, 0.5, 4, 20, 0.28125, 0.5, 2.0625, 0.5, 0.1875),
    sheet("Return address 2¼ × ¾ in, 30 per sheet (8257 size)", 2.25, 0.75, 3, 10, 0.375, 0.625, 2.75, 1.0, 0.09375),
    sheet("File folder 3 7/16 × ⅔ in, 30 per sheet (5266 size)", 3.4375, 0.66667, 2, 15, 0.53125, 0.5, 4.0, 0.66667, 0.25),
    sheet("Name badge 3⅜ × 2⅓ in, 8 per sheet (5395 size)", 3.375, 2.33333, 2, 4, 0.6875, 0.58333, 3.75, 2.5, 0.25),
    round(sheet("Round 1⅝ in, 24 per sheet (5293 size)", 1.625, 1.625, 4, 6, 0.4375, 0.5, 2.0, 1.66667, 0.0)),
    round(sheet("Round 2½ in, 12 per sheet (5294 size)", 2.5, 2.5, 3, 4, 0.25, 0.5, 2.75, 2.5, 0.0)),
    sheet("Full sheet 8½ × 11 in (5165 size)", 8.5, 11.0, 1, 1, 0.0, 0.0, 8.5, 11.0, 0.0),
    on(A4, sheet("A4 address 63.5 × 38.1 mm, 21 per sheet (L7160 size)", 2.5, 1.5, 3, 7, 0.28386, 0.59646, 2.6, 1.5, 0.23622)),
    on(A4, sheet("A4 address 99.1 × 38.1 mm, 14 per sheet (L7163 size)", 3.90157, 1.5, 2, 7, 0.18307, 0.59646, 4.0, 1.5, 0.23622)),
];

const CARD_SHEETS: [Sheet; 2] = [
    sheet("Business cards 3½ × 2 in, 10 per sheet (8871 size)", 3.5, 2.0, 2, 5, 0.75, 0.5, 3.5, 2.0, 0.0),
    sheet("One card per sheet (centered)", 3.5, 2.0, 1, 1, 2.5, 4.5, 3.5, 2.0, 0.0),
];

const POSTCARD_SHEETS: [Sheet; 5] = [
    sheet("Postcards 6 × 4 in, 2 per sheet (8386 size)", 6.0, 4.0, 1, 2, 1.25, 1.25, 6.0, 4.5, 0.0),
    sheet("Postcards 4¼ × 5½ in, 4 per sheet (8387 size)", 4.25, 5.5, 2, 2, 0.0, 0.0, 4.25, 5.5, 0.0),
    sheet("Index cards 5 × 3 in, 3 per sheet (5388 size)", 5.0, 3.0, 1, 3, 1.75, 1.0, 5.0, 3.0, 0.0),
    on((4.0, 6.0), sheet("4 × 6 in postcard stock (one per sheet)", 6.0, 4.0, 1, 1, 0.0, 0.0, 6.0, 4.0, 0.0)),
    on(A4, sheet("A4 postcards 146.5 × 110.4 mm, 2 per sheet", 5.7677, 4.3465, 1, 2, 1.25, 1.25, 5.7677, 4.8465, 0.0)),
];

/// Label and card sheets for kinds printed several to a page.
pub fn sheet_presets(kind: Kind) -> Vec<Preset> {
    let list: &[Sheet] = match kind {
        Kind::Label => &LABEL_SHEETS,
        Kind::BusinessCard => &CARD_SHEETS,
        Kind::Postcard => &POSTCARD_SHEETS,
        _ => &[],
    };
    list.iter().map(Sheet::preset).collect()
}

/// Envelope sizes (long side × short side, inches): the standard US and
/// ISO envelope sizes.
pub const ENVELOPES: [(&str, f32, f32); 15] = [
    ("#10 business (9½ × 4⅛ in)", 9.5, 4.125),
    ("#9 (8⅞ × 3⅞ in)", 8.875, 3.875),
    ("#11 (10⅜ × 4½ in)", 10.375, 4.5),
    ("#12 (11 × 4½ in)", 11.0, 4.5),
    ("#14 (11½ × 5 in)", 11.5, 5.0),
    ("Personal (6½ × 3⅝ in)", 6.5, 3.625),
    ("Monarch (7½ × 3⅞ in)", 7.5, 3.875),
    ("Card (5¾ × 4⅜ in)", 5.75, 4.375),
    ("Half-fold card (8⅞ × 5½ in)", 8.875, 5.5),
    ("A6 (6½ × 4¾ in)", 6.5, 4.75),
    ("A7 invitation (7¼ × 5¼ in)", 7.25, 5.25),
    ("DL (220 × 110 mm)", 8.66142, 4.33071),
    ("C5 (229 × 162 mm)", 9.01575, 6.37795),
    ("C6 (162 × 114 mm)", 6.37795, 4.48819),
    ("C65 (229 × 114 mm)", 9.01575, 4.48819),
];
