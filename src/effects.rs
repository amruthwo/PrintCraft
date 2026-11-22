//! Numbers and rules for frames, shadows, glows, tint and undo (see
//! docs/DESIGN.md "Frames and soft effects").

use crate::model::Rgba;

/// Lengths in inches.
pub const PT: f32 = 1.0 / 72.0;

/// Frame styles by number (the numbers are stored in saved designs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStyle {
    None = 0,
    Thin = 1,
    Medium = 2,
    Thick = 3,
    DoubleThin = 4,
    DoubleMedium = 5,
    DoubleThick = 6,
    RoundCorner = 7,
    DropShadow = 8,
}

impl FrameStyle {
    pub const ALL: [FrameStyle; 8] = [
        FrameStyle::Thin,
        FrameStyle::Medium,
        FrameStyle::Thick,
        FrameStyle::DoubleThin,
        FrameStyle::DoubleMedium,
        FrameStyle::DoubleThick,
        FrameStyle::RoundCorner,
        FrameStyle::DropShadow,
    ];
    pub fn from_code(n: u8) -> FrameStyle {
        FrameStyle::ALL.into_iter().find(|f| *f as u8 == n).unwrap_or(FrameStyle::None)
    }
    pub fn label(self) -> &'static str {
        match self {
            FrameStyle::None => "No Frame",
            FrameStyle::Thin => "Thin Line",
            FrameStyle::Medium => "Medium Line",
            FrameStyle::Thick => "Thick Line",
            FrameStyle::DoubleThin => "Double Thin",
            FrameStyle::DoubleMedium => "Double Medium",
            FrameStyle::DoubleThick => "Double Thick",
            FrameStyle::RoundCorner => "Round Corners",
            FrameStyle::DropShadow => "Drop Shadow",
        }
    }
    /// (line, gap) in inches; a gap means two lines.
    pub fn lines(self) -> (f32, f32) {
        match self {
            FrameStyle::None => (0.0, 0.0),
            FrameStyle::Thin | FrameStyle::DropShadow => (FRAME_THIN, 0.0),
            FrameStyle::Medium => (FRAME_MEDIUM, 0.0),
            FrameStyle::Thick => (FRAME_THICK, 0.0),
            FrameStyle::DoubleThin => (FRAME_THIN, FRAME_GAP_THIN),
            FrameStyle::DoubleMedium => (FRAME_MEDIUM, FRAME_GAP_MEDIUM),
            FrameStyle::DoubleThick => (FRAME_THICK, FRAME_GAP_THICK),
            FrameStyle::RoundCorner => (FRAME_THIN, 0.0),
        }
    }
    /// How far the frame reaches outside the item, in inches.
    pub fn reach(self) -> f32 {
        let (l, g) = self.lines();
        match self {
            FrameStyle::DropShadow => l + FRAME_SHADOW_OFFSET,
            _ if g > 0.0 => 2.0 * l + g,
            _ => l,
        }
    }
}

/// Frame line widths: 1 pt, 1.5 pt and 2.25 pt.
pub const FRAME_THIN: f32 = 0.013888889;
pub const FRAME_MEDIUM: f32 = 0.020833334;
pub const FRAME_THICK: f32 = 0.03125;
/// Space between the two lines of the double frames: 3, 4.5 and 6 pt.
pub const FRAME_GAP_THIN: f32 = 0.041666668;
pub const FRAME_GAP_MEDIUM: f32 = 0.0625;
pub const FRAME_GAP_THICK: f32 = 0.083333336;
/// Round-corner frame: corner radius = 1/12 in × 0.5 + 1 pt.
pub const FRAME_CORNER: f32 = 0.083333336;
pub const FRAME_CORNER_SCALE: f32 = 0.5;
/// Drop-shadow frame: the solid shadow bars are 1/16 in.
pub const FRAME_SHADOW_OFFSET: f32 = 0.0625;

pub fn corner_radius() -> f32 {
    FRAME_CORNER * FRAME_CORNER_SCALE + FRAME_THIN
}

/// Sliders for soft effects run 0..1 and follow two straight pieces:
/// low→middle over the first half, middle→high over the second.
pub const SLIDER_SPLIT: f32 = 0.5;

pub fn slider(t: f32, table: [f32; 3]) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < SLIDER_SPLIT {
        table[0] + (table[1] - table[0]) * (t * 2.0)
    } else {
        table[1] + (table[2] - table[1]) * ((t - SLIDER_SPLIT) * 2.0)
    }
}

/// Shadow distance, as a share of the item's size.
pub const SHADOW_DISTANCE: [f32; 3] = [0.029, 0.125, 0.22];
/// Shadow softness (blur), as a share of the item's size.
pub const SHADOW_BLUR: [f32; 3] = [0.0, 0.03, 0.06];
/// Extra blur in screen pixels added to any soft shadow.
pub const SHADOW_BLUR_EXTRA_PX: f32 = 4.0;
/// Glow reach, as a share of the item's size (counted on both sides).
pub const GLOW_SPREAD: [f32; 3] = [0.04, 0.135, 0.23];

/// Settings a new shadow starts with.
pub const SHADOW_START_DISTANCE: f32 = 0.08;
pub const SHADOW_START_ANGLE: f32 = 45.0;
pub const SHADOW_START_SOFTNESS: f32 = 0.0;
pub const SHADOW_START_STRENGTH: f32 = 1.0;
/// Settings a new glow starts with.
pub const GLOW_START_SIZE: f32 = 0.3;

/// The size soft effects scale with: the item's longer side (inches).
pub fn effect_size(w: f32, h: f32) -> f32 {
    w.max(h)
}

/// Where the shadow sits (inches) for a distance slider and an angle.
pub fn shadow_offset(distance: f32, angle_deg: f32, size: f32) -> (f32, f32) {
    let d = slider(distance, SHADOW_DISTANCE) * size;
    let a = angle_deg.to_radians();
    (d * a.cos(), d * a.sin())
}

/// Shadow blur in inches (0 = hard edge).
pub fn shadow_blur(softness: f32, size: f32) -> f32 {
    if softness <= 0.0 {
        return 0.0;
    }
    slider(softness, SHADOW_BLUR) * size + SHADOW_BLUR_EXTRA_PX / 96.0
}

/// How far a glow reaches past the item (inches).
pub fn glow_reach(t: f32, size: f32) -> f32 {
    2.0 * slider(t, GLOW_SPREAD) * size
}

/// Tint is a whole percent; below 100 each channel moves toward white by
/// (100 − tint) × 0.01 of the way.
pub const TINT_STEP: f32 = 0.01;

pub fn tint(c: Rgba, tint: f32) -> Rgba {
    let percent = (tint.clamp(0.0, 1.0) * 100.0).round();
    if percent >= 100.0 {
        return c;
    }
    let k = (100.0 - percent) * TINT_STEP;
    let up = |v: u8| (v as f32 + (255.0 - v as f32) * k).round().min(255.0) as u8;
    Rgba(up(c.0), up(c.1), up(c.2), c.3)
}

/// Undo keeps at least this many steps; a setting can raise it.
pub const UNDO_MINIMUM: usize = 10;
