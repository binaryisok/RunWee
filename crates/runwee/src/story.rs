//! Storytelling engine (phase T1).
//!
//! Building blocks for picture-book and comic style lessons, all pure functions of
//! time so they slot into `always_redraw`:
//!
//! * [`Palette`] — warm, limited colour palettes (marigold, monsoon, dusk, …).
//! * [`Character`] — a rigged vector person: skeletal [`Pose`]s (idle, wave, walk
//!   cycle, point, cheer, think, sit), blinking, lip-flap, mood brows, facing.
//! * [`Critter`] — a crow/bird and a fox with flap / walk / talk.
//! * Backdrops & props — sky, layered hills, sun, crescent moon, trees (round,
//!   pine, banyan), houses with lit windows, clouds.
//! * [`bubble`] — speech, thought, shout and whisper bubbles with tails and a
//!   pop-in + typed reveal; [`narration_box`] captions; [`StoryScript`] timing
//!   (who is talking when → lip-flap envelope).
//! * Transitions — [`page_turn`], [`iris`], [`ink_bleed`], [`paper_fold`].
//! * Page layout — [`open_book`], [`drop_cap_paragraph`], [`comic_panels`].
use crate::color::*;
use crate::generative::field_image;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::noise::{fbm2, hash2, hash_unit};
use crate::path::{Cubic, Path, Rect};
use crate::shapes::*;
use crate::style::{FillRule, Paint, Shadow};
use crate::text::Text;

// ------------------------------------------------------------------ helpers

/// Overshooting ease (pop-in).
pub fn ease_out_back(t: f64) -> f64 {
    let t = clamp01(t);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Smooth in-out ease.
pub fn ease_io(t: f64) -> f64 {
    smoothstep(0.0, 1.0, t)
}

/// Text in a named font family ("story", "hand", "courier", "sans", …).
pub fn font_text(s: &str, font: &str, size: f64, color: Color) -> Mobject {
    Text::new(s).font(font).size(size).color(color).build()
}

/// Book-serif text (Crimson Text).
pub fn story_text(s: &str, size: f64, color: Color) -> Mobject {
    font_text(s, "story", size, color)
}

/// Italic book-serif text.
pub fn story_italic(s: &str, size: f64, color: Color) -> Mobject {
    Text::new(s)
        .font("story")
        .italic()
        .size(size)
        .color(color)
        .build()
}

/// Handwriting text (Patrick Hand).
pub fn hand_text(s: &str, size: f64, color: Color) -> Mobject {
    font_text(s, "hand", size, color)
}

/// The first `round(p · n)` characters of `s` (typewriter reveal).
pub fn typed(s: &str, p: f64) -> String {
    let n = s.chars().count();
    let k = (clamp01(p) * n as f64).round() as usize;
    s.chars().take(k).collect()
}

/// Width of `s` set in `font` at `size`.
pub fn measure(s: &str, font: &str, size: f64) -> f64 {
    if s.trim().is_empty() {
        return 0.0;
    }
    font_text(s, font, size, WHITE).width()
}

/// Greedy word wrap with a per-line width (`width(i)` for line `i`).
pub fn wrap_lines(text: &str, font: &str, size: f64, width: &dyn Fn(usize) -> f64) -> Vec<String> {
    let mut lines = vec![];
    let mut cur = String::new();
    for w in text.split_whitespace() {
        let cand = if cur.is_empty() {
            w.to_string()
        } else {
            format!("{cur} {w}")
        };
        if !cur.is_empty() && measure(&cand, font, size) > width(lines.len()) {
            lines.push(std::mem::take(&mut cur));
            cur = w.to_string();
        } else {
            cur = cand;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Split the first `shown` characters of `text` over already-wrapped `lines`.
pub fn reveal_lines(lines: &[String], shown: usize) -> Vec<String> {
    let mut remaining = shown;
    let mut out = vec![];
    for l in lines {
        let n = l.chars().count();
        out.push(l.chars().take(remaining.min(n)).collect::<String>());
        remaining = remaining.saturating_sub(n + 1);
    }
    out
}

/// Lay out lines left-aligned; the first line's centre-line sits `size/2`
/// below `top_left`.
pub fn text_block(
    lines: &[String],
    font: &str,
    size: f64,
    color: Color,
    top_left: Vec2,
    leading: f64,
) -> Mobject {
    group(lines.iter().enumerate().filter_map(|(i, l)| {
        if l.trim().is_empty() {
            return None;
        }
        let m = font_text(l, font, size, color);
        let y = top_left.y - size * 0.5 - i as f64 * size * leading;
        Some(m.move_to_aligned(v2(top_left.x, y), v2(-1.0, 0.0)))
    }))
}

fn capsule(a: Vec2, b: Vec2, r: f64, c: Color) -> Mobject {
    let d = b - a;
    let ang = if d.length() < 1e-9 { 0.0 } else { d.angle() };
    let mut pts = Vec::with_capacity(26);
    for k in 0..=12 {
        let t = ang + PI / 2.0 + PI * k as f64 / 12.0;
        pts.push(a + Vec2::polar(r, t));
    }
    for k in 0..=12 {
        let t = ang - PI / 2.0 + PI * k as f64 / 12.0;
        pts.push(b + Vec2::polar(r, t));
    }
    polygon(&pts).fill(c, 1.0).no_stroke()
}

fn blob(c: Vec2, rx: f64, ry: f64, col: Color) -> Mobject {
    ellipse(2.0 * rx, 2.0 * ry)
        .move_to(c)
        .fill(col, 1.0)
        .no_stroke()
}

/// Darken a colour by `k` ∈ [0, 1].
pub fn shade(c: Color, k: f32) -> Color {
    c.mix(BLACK, k)
}

/// Lighten a colour by `k` ∈ [0, 1].
pub fn tint(c: Color, k: f32) -> Color {
    c.mix(WHITE, k)
}

// ------------------------------------------------------------------ palettes

/// A limited, warm colour palette for a whole story.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub name: &'static str,
    /// Sky gradient (top, horizon).
    pub sky: [Color; 2],
    /// Page / paper colour.
    pub paper: Color,
    /// Ink for text and outlines.
    pub ink: Color,
    /// Ground layers, far → near.
    pub ground: [Color; 3],
    /// Accents (character clothes, props).
    pub accent: [Color; 4],
}

impl Palette {
    pub fn marigold() -> Self {
        Palette {
            name: "marigold",
            sky: [rgb8(255, 214, 165), rgb8(255, 240, 214)],
            paper: rgb8(250, 243, 227),
            ink: rgb8(58, 42, 36),
            ground: [rgb8(196, 205, 150), rgb8(150, 172, 104), rgb8(108, 138, 78)],
            accent: [
                rgb8(240, 150, 40),
                rgb8(204, 72, 56),
                rgb8(46, 134, 130),
                rgb8(64, 76, 140),
            ],
        }
    }
    pub fn monsoon() -> Self {
        Palette {
            name: "monsoon",
            sky: [rgb8(120, 140, 160), rgb8(196, 208, 210)],
            paper: rgb8(236, 240, 236),
            ink: rgb8(34, 44, 54),
            ground: [rgb8(120, 160, 140), rgb8(80, 128, 104), rgb8(52, 96, 78)],
            accent: [
                rgb8(238, 190, 70),
                rgb8(200, 84, 84),
                rgb8(70, 150, 190),
                rgb8(120, 90, 160),
            ],
        }
    }
    pub fn dusk() -> Self {
        Palette {
            name: "dusk",
            sky: [rgb8(92, 74, 140), rgb8(250, 168, 120)],
            paper: rgb8(248, 236, 228),
            ink: rgb8(48, 32, 58),
            ground: [rgb8(150, 104, 140), rgb8(104, 72, 116), rgb8(66, 46, 84)],
            accent: [
                rgb8(255, 196, 92),
                rgb8(232, 98, 98),
                rgb8(110, 190, 180),
                rgb8(250, 230, 200),
            ],
        }
    }
    pub fn indigo_night() -> Self {
        Palette {
            name: "indigo_night",
            sky: [rgb8(14, 20, 52), rgb8(52, 64, 120)],
            paper: rgb8(232, 230, 244),
            ink: rgb8(20, 22, 40),
            ground: [rgb8(40, 52, 96), rgb8(28, 38, 74), rgb8(18, 26, 52)],
            accent: [
                rgb8(255, 220, 120),
                rgb8(230, 110, 120),
                rgb8(110, 200, 210),
                rgb8(190, 160, 240),
            ],
        }
    }
    pub fn forest() -> Self {
        Palette {
            name: "forest",
            sky: [rgb8(182, 222, 214), rgb8(236, 244, 222)],
            paper: rgb8(244, 242, 228),
            ink: rgb8(36, 48, 36),
            ground: [rgb8(150, 190, 120), rgb8(96, 150, 90), rgb8(56, 108, 66)],
            accent: [
                rgb8(214, 150, 60),
                rgb8(190, 80, 60),
                rgb8(80, 120, 170),
                rgb8(240, 210, 120),
            ],
        }
    }
    pub fn terracotta() -> Self {
        Palette {
            name: "terracotta",
            sky: [rgb8(236, 196, 160), rgb8(246, 226, 200)],
            paper: rgb8(246, 236, 220),
            ink: rgb8(70, 36, 24),
            ground: [rgb8(206, 140, 100), rgb8(176, 100, 70), rgb8(138, 70, 50)],
            accent: [
                rgb8(250, 246, 236),
                rgb8(60, 50, 44),
                rgb8(220, 170, 60),
                rgb8(70, 110, 100),
            ],
        }
    }
    pub fn all() -> Vec<Palette> {
        vec![
            Self::marigold(),
            Self::monsoon(),
            Self::dusk(),
            Self::indigo_night(),
            Self::forest(),
            Self::terracotta(),
        ]
    }
    /// Palette by name (falls back to marigold).
    pub fn named(name: &str) -> Self {
        Self::all()
            .into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(Self::marigold)
    }
}

// ------------------------------------------------------------------ poses

/// Joint angles (radians). Limb angles are measured from hanging straight down;
/// positive swings *forward* (towards the facing direction). Elbows/knees are
/// relative bends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub back_shoulder: f64,
    pub back_elbow: f64,
    pub front_shoulder: f64,
    pub front_elbow: f64,
    pub back_hip: f64,
    pub back_knee: f64,
    pub front_hip: f64,
    pub front_knee: f64,
    /// Whole-body lean (positive = forward).
    pub lean: f64,
    pub head_tilt: f64,
    /// Vertical bob in body heights.
    pub bob: f64,
}

impl Pose {
    pub fn idle(t: f64) -> Pose {
        let br = (t * 1.6).sin() * 0.03;
        Pose {
            back_shoulder: -0.12 + br,
            back_elbow: 0.25,
            front_shoulder: 0.12 - br,
            front_elbow: 0.3,
            back_hip: -0.05,
            front_hip: 0.05,
            head_tilt: (t * 0.9).sin() * 0.04,
            bob: br * 0.1,
            ..Default::default()
        }
    }
    pub fn wave(t: f64) -> Pose {
        Pose {
            front_shoulder: 1.95,
            front_elbow: 0.85 + 0.35 * (t * 9.0).sin(),
            head_tilt: 0.08,
            ..Pose::idle(t)
        }
    }
    pub fn point() -> Pose {
        Pose {
            front_shoulder: 1.45,
            front_elbow: 0.05,
            ..Pose::idle(0.0)
        }
    }
    pub fn cheer(t: f64) -> Pose {
        let j = (t * 6.0).sin().abs();
        Pose {
            back_shoulder: -2.6,
            back_elbow: -0.2,
            front_shoulder: 2.6,
            front_elbow: 0.2,
            back_knee: -0.3 * j,
            front_knee: -0.3 * j,
            bob: 0.04 * j,
            head_tilt: -0.1,
            ..Default::default()
        }
    }
    pub fn think() -> Pose {
        Pose {
            front_shoulder: 0.55,
            front_elbow: 2.35,
            back_shoulder: 0.35,
            back_elbow: 1.3,
            head_tilt: 0.18,
            ..Pose::idle(0.0)
        }
    }
    pub fn sit() -> Pose {
        Pose {
            back_hip: 1.5,
            back_knee: -1.5,
            front_hip: 1.55,
            front_knee: -1.55,
            back_shoulder: 0.4,
            back_elbow: 0.6,
            front_shoulder: 0.5,
            front_elbow: 0.7,
            bob: -0.2,
            ..Default::default()
        }
    }
    /// Walk cycle; `phase` in cycles (1.0 = two steps).
    pub fn walk(phase: f64) -> Pose {
        let a = phase * TAU;
        let s = a.sin();
        Pose {
            front_hip: 0.42 * s,
            back_hip: -0.42 * s,
            front_knee: -(0.55 * (a + 0.9).sin()).max(0.0) - 0.05,
            back_knee: -(0.55 * (a + 0.9 + PI).sin()).max(0.0) - 0.05,
            front_shoulder: -0.38 * s,
            back_shoulder: 0.38 * s,
            front_elbow: 0.35,
            back_elbow: 0.35,
            lean: 0.04,
            bob: 0.012 * (2.0 * a).cos(),
            head_tilt: 0.0,
        }
    }
    pub fn lerp(&self, o: &Pose, t: f64) -> Pose {
        let l = |a: f64, b: f64| lerp(a, b, t);
        Pose {
            back_shoulder: l(self.back_shoulder, o.back_shoulder),
            back_elbow: l(self.back_elbow, o.back_elbow),
            front_shoulder: l(self.front_shoulder, o.front_shoulder),
            front_elbow: l(self.front_elbow, o.front_elbow),
            back_hip: l(self.back_hip, o.back_hip),
            back_knee: l(self.back_knee, o.back_knee),
            front_hip: l(self.front_hip, o.front_hip),
            front_knee: l(self.front_knee, o.front_knee),
            lean: l(self.lean, o.lean),
            head_tilt: l(self.head_tilt, o.head_tilt),
            bob: l(self.bob, o.bob),
        }
    }
    /// Pose by name: idle, wave, point, cheer, think, sit, walk.
    pub fn named(name: &str, t: f64) -> Pose {
        match name {
            "wave" => Pose::wave(t),
            "point" => Pose::point(),
            "cheer" => Pose::cheer(t),
            "think" => Pose::think(),
            "sit" => Pose::sit(),
            "walk" => Pose::walk(t * 0.9),
            _ => Pose::idle(t),
        }
    }
}

/// Facial state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expression {
    /// 0 = open, 1 = closed.
    pub blink: f64,
    /// 0 = closed, 1 = wide open.
    pub mouth: f64,
    /// −1 sad … +1 happy.
    pub smile: f64,
    /// −1 frown/angry … +1 surprised.
    pub brows: f64,
    /// +1 faces right, −1 faces left.
    pub facing: f64,
}

impl Default for Expression {
    fn default() -> Self {
        Expression {
            blink: 0.0,
            mouth: 0.0,
            smile: 0.6,
            brows: 0.0,
            facing: 1.0,
        }
    }
}

/// Deterministic blinking: closes for ~0.14 s every 2.4–4.6 s.
pub fn blink_amount(t: f64, seed: u64) -> f64 {
    let mut start = hash_unit(hash2(0, 0, seed)) * 2.0;
    let mut k = 0i64;
    while start < t + 1.0 && k < 10_000 {
        let d = t - start;
        if (0.0..0.14).contains(&d) {
            return (1.0 - (d / 0.07 - 1.0).abs()).clamp(0.0, 1.0);
        }
        k += 1;
        start += 2.4 + 2.2 * hash_unit(hash2(k, 7, seed));
    }
    0.0
}

/// Lip-flap mouth opening for a talking envelope `talking` ∈ [0, 1].
pub fn lip_flap(t: f64, talking: f64) -> f64 {
    if talking <= 0.0 {
        return 0.0;
    }
    let syl = (t * 11.0).sin().abs() * (0.65 + 0.35 * (t * 3.7).sin());
    (talking * syl).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HairStyle {
    Short,
    Long,
    Bun,
    Plait,
    Cap,
    Bald,
}

/// A rigged vector character. Feet at the origin, height `height`, faces +x.
#[derive(Clone, Debug, PartialEq)]
pub struct Character {
    pub skin: Color,
    pub hair: Color,
    pub outfit: Color,
    pub legs: Color,
    pub accent: Color,
    pub height: f64,
    pub hair_style: HairStyle,
    pub moustache: bool,
    pub glasses: bool,
    pub seed: u64,
}

const HEAD_R: f64 = 0.125;
const NECK_Y: f64 = 0.76;
const HIP_Y: f64 = 0.47;
const THIGH: f64 = 0.235;
const SHIN: f64 = 0.225;
const UPPER_ARM: f64 = 0.165;
const FOREARM: f64 = 0.155;
/// Forward offset of the front shoulder (three-quarter view).
const SHOULDER_DX: f64 = 0.05;

/// World positions of key points (for attaching props / bubble tails).
#[derive(Clone, Copy, Debug)]
pub struct Joints {
    pub head: Vec2,
    pub mouth: Vec2,
    pub front_hand: Vec2,
    pub back_hand: Vec2,
    pub hip: Vec2,
    pub top: Vec2,
}

fn limb_points(root: Vec2, a1: f64, a2: f64, l1: f64, l2: f64) -> (Vec2, Vec2) {
    let d1 = v2(a1.sin(), -a1.cos());
    let j = root + d1 * l1;
    let a = a1 + a2;
    let d2 = v2(a.sin(), -a.cos());
    (j, j + d2 * l2)
}

impl Character {
    pub fn new(skin: Color, hair: Color, outfit: Color) -> Self {
        Character {
            skin,
            hair,
            outfit,
            legs: rgb8(70, 62, 84),
            accent: rgb8(250, 200, 80),
            height: 2.0,
            hair_style: HairStyle::Short,
            moustache: false,
            glasses: false,
            seed: 1,
        }
    }
    /// A girl with a plait in a marigold kurta.
    pub fn girl() -> Self {
        Character {
            hair_style: HairStyle::Plait,
            legs: rgb8(60, 84, 120),
            accent: rgb8(204, 72, 56),
            seed: 11,
            ..Character::new(rgb8(176, 120, 86), rgb8(36, 26, 26), rgb8(240, 156, 52))
        }
    }
    /// A boy in a teal shirt.
    pub fn boy() -> Self {
        Character {
            hair_style: HairStyle::Short,
            legs: rgb8(64, 70, 98),
            accent: rgb8(250, 220, 120),
            seed: 23,
            ..Character::new(rgb8(150, 100, 70), rgb8(30, 24, 24), rgb8(46, 140, 136))
        }
    }
    /// A grandfather with white hair, moustache and glasses.
    pub fn grandpa() -> Self {
        Character {
            hair_style: HairStyle::Bald,
            moustache: true,
            glasses: true,
            legs: rgb8(236, 228, 210),
            accent: rgb8(160, 110, 60),
            height: 2.1,
            seed: 37,
            ..Character::new(rgb8(168, 116, 84), rgb8(240, 238, 232), rgb8(232, 224, 204))
        }
    }
    /// A teacher with a bun and glasses.
    pub fn teacher() -> Self {
        Character {
            hair_style: HairStyle::Bun,
            glasses: true,
            legs: rgb8(150, 60, 90),
            accent: rgb8(250, 200, 80),
            height: 2.15,
            seed: 41,
            ..Character::new(rgb8(186, 132, 98), rgb8(40, 28, 30), rgb8(170, 64, 100))
        }
    }
    /// Preset by name: girl, boy, grandpa, teacher.
    pub fn named(name: &str) -> Self {
        match name {
            "boy" => Self::boy(),
            "grandpa" | "grandfather" => Self::grandpa(),
            "teacher" => Self::teacher(),
            _ => Self::girl(),
        }
    }
    pub fn with_height(mut self, h: f64) -> Self {
        self.height = h;
        self
    }

    /// Key points for `pose` at `pos` facing `facing` (+1 right, −1 left).
    pub fn joints(&self, pose: &Pose, pos: Vec2, facing: f64) -> Joints {
        let h = self.height;
        let f = if facing < 0.0 { -1.0 } else { 1.0 };
        let tf = |p: Vec2| pos + v2(p.x * f, p.y);
        let hip = v2(0.0, HIP_Y * h + pose.bob * h);
        let lean = |p: Vec2| hip + (p - hip).rotate(-pose.lean);
        let neck = lean(v2(0.0, NECK_Y * h + pose.bob * h));
        let head = neck + v2(0.0, HEAD_R * h * 1.05).rotate(-pose.lean - pose.head_tilt);
        let sh = neck + v2(0.0, -0.035 * h);
        let (_, fh) = limb_points(
            sh + v2(SHOULDER_DX * h, 0.0).rotate(-pose.lean),
            pose.front_shoulder - pose.lean,
            pose.front_elbow,
            UPPER_ARM * h,
            FOREARM * h,
        );
        let (_, bh) = limb_points(
            sh - v2(SHOULDER_DX * h, 0.0).rotate(-pose.lean),
            pose.back_shoulder - pose.lean,
            pose.back_elbow,
            UPPER_ARM * h,
            FOREARM * h,
        );
        Joints {
            head: tf(head),
            mouth: tf(head + v2(0.35 * HEAD_R * h, -0.4 * HEAD_R * h)),
            front_hand: tf(fh),
            back_hand: tf(bh),
            hip: tf(hip),
            top: tf(head + v2(0.0, HEAD_R * h * 1.15)),
        }
    }

    /// Draw the character with an explicit expression.
    pub fn draw(&self, pose: &Pose, ex: &Expression, pos: Vec2) -> Mobject {
        let h = self.height;
        let mut parts: Vec<Mobject> = vec![];
        let hip = v2(0.0, HIP_Y * h + pose.bob * h);
        let lean = |p: Vec2| hip + (p - hip).rotate(-pose.lean);
        let neck = lean(v2(0.0, NECK_Y * h + pose.bob * h));
        let sh = neck + v2(0.0, -0.035 * h);
        let head_c = neck + v2(0.0, HEAD_R * h * 1.05).rotate(-pose.lean - pose.head_tilt);
        let limb_r = 0.034 * h;
        let arm_r = 0.03 * h;
        // soft contact shadow
        parts.push(
            ellipse(0.42 * h, 0.06 * h)
                .move_to(v2(0.0, 0.005))
                .fill(BLACK, 0.16)
                .no_stroke(),
        );
        let legs_back = shade(self.legs, 0.18);
        let skin_back = shade(self.skin, 0.12);
        let shoe = shade(self.legs, 0.55);
        let draw_leg = |parts: &mut Vec<Mobject>, a1: f64, a2: f64, col: Color| {
            let root = hip + v2(0.0, -0.02 * h);
            let (k, f) = limb_points(root, a1, a2, THIGH * h, SHIN * h);
            parts.push(capsule(root, k, limb_r * 1.15, col));
            parts.push(capsule(k, f, limb_r, col));
            parts.push(blob(
                f + v2(0.035 * h, -0.005 * h),
                0.06 * h,
                0.03 * h,
                shoe,
            ));
        };
        let draw_arm =
            |parts: &mut Vec<Mobject>, a1: f64, a2: f64, sleeve: Color, skin: Color, dx: f64| {
                let sh = sh + v2(dx * h, 0.0).rotate(-pose.lean);
                let (e, hnd) = limb_points(sh, a1 - pose.lean, a2, UPPER_ARM * h, FOREARM * h);
                parts.push(capsule(sh, e, arm_r * 1.25, sleeve));
                parts.push(capsule(e, hnd, arm_r, skin));
                parts.push(circle(0.04 * h).move_to(hnd).fill(skin, 1.0).no_stroke());
            };
        // hair behind the head
        match self.hair_style {
            HairStyle::Long => parts.push(
                rounded_rectangle(0.3 * h, 0.36 * h, 0.12 * h)
                    .move_to(head_c + v2(-0.04 * h, -0.1 * h))
                    .fill(self.hair, 1.0)
                    .no_stroke(),
            ),
            HairStyle::Plait => {
                let mut prev = head_c + v2(-0.09 * h, -0.02 * h);
                for _ in 0..5 {
                    let c = prev + v2(-0.012 * h, -0.05 * h);
                    parts.push(blob(c, 0.032 * h, 0.034 * h, self.hair));
                    prev = c;
                }
                parts.push(blob(
                    prev + v2(0.0, -0.04 * h),
                    0.022 * h,
                    0.022 * h,
                    self.accent,
                ));
            }
            _ => {}
        }
        draw_arm(
            &mut parts,
            pose.back_shoulder,
            pose.back_elbow,
            shade(self.outfit, 0.2),
            skin_back,
            -SHOULDER_DX,
        );
        draw_leg(&mut parts, pose.back_hip, pose.back_knee, legs_back);
        // torso: a kurta flaring to the hem
        let b = pose.bob * h;
        let hem = 0.33 * h + b;
        let sw = 0.115 * h;
        let torso = [
            lean(v2(-sw * 0.85, NECK_Y * h - 0.03 * h + b)),
            lean(v2(sw * 0.85, NECK_Y * h - 0.03 * h + b)),
            lean(v2(sw * 1.05, HIP_Y * h + b)),
            lean(v2(sw * 1.25, hem)),
            lean(v2(-sw * 1.25, hem)),
            lean(v2(-sw * 1.05, HIP_Y * h + b)),
        ];
        parts.push(
            polygon(&torso)
                .fill(self.outfit, 1.0)
                .stroke(shade(self.outfit, 0.3), 1.5),
        );
        parts.push(
            polygon(&[
                lean(v2(-0.045 * h, NECK_Y * h - 0.025 * h + b)),
                lean(v2(0.045 * h, NECK_Y * h - 0.025 * h + b)),
                lean(v2(0.0, NECK_Y * h - 0.1 * h + b)),
            ])
            .fill(self.accent, 1.0)
            .no_stroke(),
        );
        parts.push(
            line(
                lean(v2(-sw * 1.22, hem + 0.03 * h)),
                lean(v2(sw * 1.22, hem + 0.03 * h)),
            )
            .stroke(self.accent, 3.0),
        );
        draw_leg(&mut parts, pose.front_hip, pose.front_knee, self.legs);
        // neck + head
        parts.push(capsule(
            neck + v2(0.0, -0.02 * h),
            head_c,
            0.03 * h,
            skin_back,
        ));
        let r = HEAD_R * h;
        parts.push(circle(r).move_to(head_c).fill(self.skin, 1.0).no_stroke());
        parts.push(blob(
            head_c + v2(-0.2 * r, -0.05 * r),
            0.16 * r,
            0.2 * r,
            skin_back,
        ));
        // hair on top
        match self.hair_style {
            HairStyle::Short | HairStyle::Long | HairStyle::Bun | HairStyle::Plait => {
                let mut pts = vec![];
                for k in 0..=24 {
                    let a = PI * 1.12 - (PI * 1.3) * k as f64 / 24.0;
                    pts.push(head_c + Vec2::polar(r * 1.06, a));
                }
                pts.push(head_c + v2(0.55 * r, 0.38 * r));
                pts.push(head_c + v2(0.1 * r, 0.5 * r));
                pts.push(head_c + v2(-0.35 * r, 0.25 * r));
                pts.push(head_c + v2(-0.75 * r, -0.2 * r));
                parts.push(polygon(&pts).fill(self.hair, 1.0).no_stroke());
                if self.hair_style == HairStyle::Bun {
                    parts.push(blob(
                        head_c + v2(-0.75 * r, 0.75 * r),
                        0.42 * r,
                        0.42 * r,
                        self.hair,
                    ));
                }
            }
            HairStyle::Cap => {
                parts.push(
                    sector(r * 1.08, 0.0, PI)
                        .move_to(head_c + v2(0.0, 0.35 * r))
                        .fill(self.accent, 1.0)
                        .no_stroke(),
                );
                parts.push(blob(
                    head_c + v2(0.8 * r, 0.32 * r),
                    0.55 * r,
                    0.12 * r,
                    shade(self.accent, 0.2),
                ));
            }
            HairStyle::Bald => {
                parts.push(blob(
                    head_c + v2(-0.6 * r, 0.2 * r),
                    0.42 * r,
                    0.32 * r,
                    self.hair,
                ));
            }
        }
        // face (three-quarter view towards +x)
        let ink = rgb8(40, 28, 26);
        let fx = 0.28 * r;
        let eye_y = 0.08 * r;
        let brow_lift = 0.05 * r * ex.brows;
        let brow_col = if self.hair_style == HairStyle::Bald {
            shade(self.hair, 0.45)
        } else {
            self.hair
        };
        for (k, ex_off) in [(0, -0.22 * r), (1, 0.34 * r)] {
            let ec = head_c + v2(fx + ex_off, eye_y);
            let es = if k == 0 { 0.85 } else { 1.0 };
            if ex.blink > 0.6 {
                parts.push(
                    arc_centered(ec + v2(0.0, 0.03 * r), 0.09 * r * es, PI * 1.15, PI * 0.7)
                        .stroke(ink, 2.2),
                );
            } else {
                let eh = 0.24 * r * (1.0 - ex.blink) * es;
                parts.push(blob(ec, 0.085 * r * es, eh * 0.5 + 0.01 * r, ink));
                parts.push(blob(ec + v2(0.03 * r, 0.05 * r), 0.03 * r, 0.03 * r, WHITE));
            }
            let tilt = -ex.brows.min(0.0) * 0.35 * if k == 0 { -1.0 } else { 1.0 };
            let bc = ec + v2(0.0, 0.22 * r + brow_lift);
            let bd = Vec2::polar(0.09 * r * es, tilt);
            parts.push(line(bc - bd, bc + bd).stroke(brow_col, 2.4));
        }
        parts.push(
            arc_centered(
                head_c + v2(fx + 0.12 * r, -0.12 * r),
                0.07 * r,
                -PI * 0.5,
                PI * 0.8,
            )
            .stroke(shade(self.skin, 0.35), 1.8),
        );
        parts.push(blob(
            head_c + v2(fx + 0.38 * r, -0.22 * r),
            0.11 * r,
            0.07 * r,
            rgb8(240, 120, 110).with_alpha(0.35),
        ));
        let mc = head_c + v2(fx + 0.07 * r, -0.42 * r);
        if self.moustache {
            parts.push(blob(
                mc + v2(-0.02 * r, 0.1 * r),
                0.2 * r,
                0.06 * r,
                self.hair,
            ));
        }
        if ex.mouth > 0.04 {
            let mh = 0.05 * r + 0.24 * r * ex.mouth;
            parts.push(blob(mc, 0.12 * r, mh * 0.5, rgb8(120, 40, 44)));
            parts.push(blob(
                mc + v2(0.0, -mh * 0.22),
                0.07 * r,
                mh * 0.2,
                rgb8(220, 110, 110),
            ));
        } else {
            let s = ex.smile;
            let w = 0.14 * r;
            let pts = [
                mc + v2(-w, 0.03 * r * s),
                mc + v2(0.0, -0.05 * r * s),
                mc + v2(w, 0.03 * r * s),
            ];
            parts.push(smooth_curve(&pts, false).stroke(ink, 2.2).fill(ink, 0.0));
        }
        if self.glasses {
            let gl = rgb8(60, 50, 50);
            for ex_off in [-0.22 * r, 0.34 * r] {
                parts.push(
                    circle(0.15 * r)
                        .move_to(head_c + v2(fx + ex_off, eye_y))
                        .fill(WHITE, 0.08)
                        .stroke(gl, 1.8),
                );
            }
            parts.push(
                line(
                    head_c + v2(fx - 0.07 * r, eye_y),
                    head_c + v2(fx + 0.19 * r, eye_y),
                )
                .stroke(gl, 1.6),
            );
        }
        draw_arm(
            &mut parts,
            pose.front_shoulder,
            pose.front_elbow,
            self.outfit,
            self.skin,
            SHOULDER_DX,
        );
        let m = group(parts);
        let m = if ex.facing < 0.0 {
            m.apply_function(|p| v2(-p.x, p.y))
        } else {
            m
        };
        m.shift(pos)
    }

    /// Draw at time `t` with automatic blinking and lip-flap from `talking`.
    pub fn draw_at(&self, pose: &Pose, t: f64, talking: f64, facing: f64, pos: Vec2) -> Mobject {
        let ex = Expression {
            blink: blink_amount(t, self.seed),
            mouth: lip_flap(t, talking),
            facing,
            ..Default::default()
        };
        self.draw(pose, &ex, pos)
    }
}

// ------------------------------------------------------------------ critters

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CritterKind {
    Bird,
    Fox,
}

/// Simple animal actors (fables!).
#[derive(Clone, Debug, PartialEq)]
pub struct Critter {
    pub kind: CritterKind,
    pub body: Color,
    pub belly: Color,
    pub size: f64,
    pub seed: u64,
}

impl Critter {
    /// A glossy crow.
    pub fn crow() -> Self {
        Critter {
            kind: CritterKind::Bird,
            body: rgb8(46, 48, 62),
            belly: rgb8(84, 88, 110),
            size: 1.0,
            seed: 5,
        }
    }
    pub fn bird(body: Color, belly: Color) -> Self {
        Critter {
            kind: CritterKind::Bird,
            body,
            belly,
            size: 1.0,
            seed: 9,
        }
    }
    pub fn fox() -> Self {
        Critter {
            kind: CritterKind::Fox,
            body: rgb8(222, 112, 44),
            belly: rgb8(250, 238, 222),
            size: 1.0,
            seed: 13,
        }
    }
    pub fn with_size(mut self, s: f64) -> Self {
        self.size = s;
        self
    }

    /// Draw standing at `pos`. `motion` is a wing-flap (bird) or walk (fox)
    /// phase in cycles (0 = still); `talking` ∈ [0, 1] opens the beak / jaw.
    pub fn draw(&self, t: f64, talking: f64, motion: f64, facing: f64, pos: Vec2) -> Mobject {
        let s = self.size;
        let blink = blink_amount(t, self.seed);
        let open = lip_flap(t, talking);
        let ink = rgb8(26, 22, 24);
        let mut parts: Vec<Mobject> = vec![];
        match self.kind {
            CritterKind::Bird => {
                let c = v2(0.0, 0.42 * s);
                parts.push(
                    polygon(&[
                        c + v2(-0.25 * s, 0.02 * s),
                        c + v2(-0.72 * s, -0.12 * s),
                        c + v2(-0.66 * s, -0.24 * s),
                        c + v2(-0.22 * s, -0.12 * s),
                    ])
                    .fill(shade(self.body, 0.2), 1.0)
                    .no_stroke(),
                );
                for dx in [-0.06, 0.08] {
                    parts.push(
                        line(c + v2(dx * s, -0.2 * s), v2(dx * s + 0.02 * s, 0.0))
                            .stroke(rgb8(90, 70, 50), 3.0),
                    );
                }
                parts.push(blob(c, 0.36 * s, 0.25 * s, self.body).rotate_about(0.12, c));
                parts.push(blob(
                    c + v2(0.1 * s, -0.07 * s),
                    0.22 * s,
                    0.14 * s,
                    self.belly,
                ));
                let wa = 0.35 * (motion * TAU).sin();
                let wing = smooth_curve(
                    &[
                        c + v2(0.08 * s, 0.08 * s),
                        c + v2(-0.22 * s, 0.12 * s),
                        c + v2(-0.42 * s, -0.04 * s),
                        c + v2(-0.1 * s, -0.06 * s),
                    ],
                    true,
                )
                .fill(shade(self.body, 0.3), 1.0)
                .no_stroke()
                .rotate_about(wa, c + v2(0.05 * s, 0.08 * s));
                parts.push(wing);
                let hc = c + v2(0.3 * s, 0.2 * s);
                parts.push(
                    circle(0.17 * s)
                        .move_to(hc)
                        .fill(self.body, 1.0)
                        .no_stroke(),
                );
                let bb = hc + v2(0.13 * s, -0.02 * s);
                let gape = 0.3 * open;
                parts.push(
                    polygon(&[
                        bb + v2(0.0, 0.05 * s),
                        bb + v2(0.24 * s, 0.0),
                        bb + v2(0.0, -0.005 * s),
                    ])
                    .fill(rgb8(232, 170, 60), 1.0)
                    .no_stroke()
                    .rotate_about(gape * 0.5, bb),
                );
                parts.push(
                    polygon(&[
                        bb + v2(0.0, -0.005 * s),
                        bb + v2(0.2 * s, -0.01 * s),
                        bb + v2(0.0, -0.05 * s),
                    ])
                    .fill(rgb8(200, 140, 50), 1.0)
                    .no_stroke()
                    .rotate_about(-gape, bb),
                );
                let ec = hc + v2(0.05 * s, 0.05 * s);
                if blink > 0.6 {
                    parts.push(
                        line(ec - v2(0.035 * s, 0.0), ec + v2(0.035 * s, 0.0)).stroke(WHITE, 2.0),
                    );
                } else {
                    parts.push(circle(0.045 * s).move_to(ec).fill(WHITE, 1.0).no_stroke());
                    parts.push(
                        circle(0.025 * s)
                            .move_to(ec + v2(0.01 * s, 0.0))
                            .fill(ink, 1.0)
                            .no_stroke(),
                    );
                }
            }
            CritterKind::Fox => {
                let a = motion * TAU;
                let c = v2(0.0, 0.5 * s);
                let sway = 0.12 * (t * 2.0).sin();
                let tail = [
                    c + v2(-0.4 * s, 0.05 * s),
                    c + v2(-0.75 * s, 0.08 * s + sway * s),
                    c + v2(-s, 0.32 * s + sway * s),
                    c + v2(-0.8 * s, 0.2 * s),
                    c + v2(-0.45 * s, -0.08 * s),
                ];
                parts.push(smooth_curve(&tail, true).fill(self.body, 1.0).no_stroke());
                parts.push(blob(
                    c + v2(-0.95 * s, 0.3 * s + sway * s),
                    0.09 * s,
                    0.07 * s,
                    self.belly,
                ));
                for (k, dx) in [-0.32f64, -0.2, 0.22, 0.34].iter().enumerate() {
                    let ph = a + if k % 2 == 0 { 0.0 } else { PI };
                    let sw = if motion == 0.0 { 0.0 } else { 0.35 * ph.sin() };
                    let top = c + v2(dx * s, -0.05 * s);
                    let foot = top + v2(0.42 * s * sw.sin(), -0.45 * s * sw.cos());
                    let col = if k == 0 || k == 2 {
                        shade(self.body, 0.25)
                    } else {
                        self.body
                    };
                    parts.push(capsule(top, foot, 0.045 * s, col));
                    parts.push(blob(
                        foot + v2(0.03 * s, 0.0),
                        0.05 * s,
                        0.03 * s,
                        rgb8(60, 40, 34),
                    ));
                }
                parts.push(blob(c, 0.48 * s, 0.2 * s, self.body));
                parts.push(blob(
                    c + v2(0.12 * s, -0.08 * s),
                    0.3 * s,
                    0.1 * s,
                    self.belly,
                ));
                let hc = c + v2(0.5 * s, 0.22 * s);
                for (ex, col) in [(-0.1, shade(self.body, 0.2)), (0.06, self.body)] {
                    parts.push(
                        polygon(&[
                            hc + v2((ex - 0.07) * s, 0.08 * s),
                            hc + v2(ex * s, 0.34 * s),
                            hc + v2((ex + 0.08) * s, 0.1 * s),
                        ])
                        .fill(col, 1.0)
                        .no_stroke(),
                    );
                }
                parts.push(
                    circle(0.17 * s)
                        .move_to(hc)
                        .fill(self.body, 1.0)
                        .no_stroke(),
                );
                let jaw = 0.3 * open;
                parts.push(
                    polygon(&[
                        hc + v2(0.05 * s, 0.06 * s),
                        hc + v2(0.36 * s, -0.04 * s),
                        hc + v2(0.05 * s, -0.08 * s),
                    ])
                    .fill(self.belly, 1.0)
                    .no_stroke(),
                );
                parts.push(
                    polygon(&[
                        hc + v2(0.05 * s, -0.08 * s),
                        hc + v2(0.3 * s, -0.07 * s),
                        hc + v2(0.05 * s, -0.14 * s),
                    ])
                    .fill(shade(self.belly, 0.1), 1.0)
                    .no_stroke()
                    .rotate_about(-jaw, hc + v2(0.05 * s, -0.08 * s)),
                );
                parts.push(
                    circle(0.03 * s)
                        .move_to(hc + v2(0.36 * s, -0.04 * s))
                        .fill(ink, 1.0)
                        .no_stroke(),
                );
                let ec = hc + v2(0.08 * s, 0.05 * s);
                if blink > 0.6 {
                    parts.push(
                        line(ec - v2(0.03 * s, 0.0), ec + v2(0.03 * s, 0.0)).stroke(ink, 2.0),
                    );
                } else {
                    parts.push(blob(ec, 0.028 * s, 0.04 * s, ink));
                }
            }
        }
        let m = group(parts);
        let m = if facing < 0.0 {
            m.apply_function(|p| v2(-p.x, p.y))
        } else {
            m
        };
        m.shift(pos)
    }

    /// Where the mouth is (bubble tails).
    pub fn mouth(&self, facing: f64, pos: Vec2) -> Vec2 {
        let s = self.size;
        let f = if facing < 0.0 { -1.0 } else { 1.0 };
        let p = match self.kind {
            CritterKind::Bird => v2(0.6 * s, 0.6 * s),
            CritterKind::Fox => v2(0.85 * s, 0.68 * s),
        };
        pos + v2(p.x * f, p.y)
    }
}

// ------------------------------------------------------------------ backdrops & props

/// Vertical sky gradient filling `rect`.
pub fn sky(rect: Rect, top: Color, horizon: Color) -> Mobject {
    let (lo, hi) = (rect.min.y, rect.max.y);
    rectangle(rect.width(), rect.height())
        .move_to(rect.center())
        .fill(top, 1.0)
        .no_stroke()
        .map_style(move |s| s.fill = Paint::linear(v2(0.0, lo), v2(0.0, hi), &[horizon, top]))
}

/// A rolling hill band across `rect` with its ridge at `base` ± `amp`.
pub fn hills(rect: Rect, base: f64, amp: f64, color: Color, seed: u64) -> Mobject {
    let n = 10;
    let mut ridge = vec![];
    for i in 0..=n {
        let x = rect.min.x - 0.3 + (rect.width() + 0.6) * i as f64 / n as f64;
        let y = base + amp * fbm2(v2(x * 0.32, seed as f64 * 3.1), 3, 2.0, 0.5, seed) * 1.6;
        ridge.push(v2(x, y));
    }
    let mut path = Path::smooth_through(&ridge, false);
    let bl = v2(rect.min.x - 0.3, rect.min.y - 0.3);
    let br = v2(rect.max.x + 0.3, rect.min.y - 0.3);
    if let Some(sp) = path.subpaths.first_mut() {
        let e = sp.end();
        sp.segs.push(Cubic::line(e, br));
        sp.segs.push(Cubic::line(br, bl));
        sp.segs.push(Cubic::line(bl, ridge[0]));
        sp.closed = true;
    }
    Mobject::from_path(path).fill(color, 1.0).no_stroke()
}

/// Three hill layers in the palette's ground colours (atmospheric perspective).
pub fn layered_hills(rect: Rect, pal: &Palette, seed: u64) -> Mobject {
    let h = rect.height();
    group(vec![
        hills(rect, rect.min.y + 0.46 * h, 0.45, pal.ground[0], seed),
        hills(rect, rect.min.y + 0.32 * h, 0.35, pal.ground[1], seed + 1),
        hills(rect, rect.min.y + 0.17 * h, 0.22, pal.ground[2], seed + 2),
    ])
}

/// A glowing sun.
pub fn sun(pos: Vec2, r: f64, color: Color) -> Mobject {
    group(vec![
        circle(r * 2.0).move_to(pos).fill(color, 0.1).no_stroke(),
        circle(r * 1.45).move_to(pos).fill(color, 0.2).no_stroke(),
        circle(r).move_to(pos).fill(color, 1.0).no_stroke(),
    ])
}

/// The outline of a crescent: the disc of radius `r` at the origin minus a
/// disc of radius `rc` centred at `cut`.
pub fn crescent_points(r: f64, cut: Vec2, rc: f64, n: usize) -> Vec<Vec2> {
    let n = n.max(16);
    // outer arc: points of the moon circle outside the cutter, CCW
    let outer: Vec<Vec2> = (0..n)
        .map(|k| Vec2::polar(r, TAU * k as f64 / n as f64))
        .collect();
    let keep: Vec<bool> = outer.iter().map(|p| p.distance(cut) > rc).collect();
    let start = (0..n)
        .find(|&k| keep[k] && !keep[(k + n - 1) % n])
        .unwrap_or(0);
    let mut pts = vec![];
    for k in 0..n {
        let i = (start + k) % n;
        if keep[i] {
            pts.push(outer[i]);
        } else {
            break;
        }
    }
    if pts.len() == n {
        return pts; // no overlap: full disc
    }
    // inner arc: points of the cutter inside the moon, joined end → start
    let inner: Vec<Vec2> = (0..n)
        .map(|k| cut + Vec2::polar(rc, TAU * k as f64 / n as f64))
        .collect();
    let ik: Vec<bool> = inner.iter().map(|p| p.length() < r).collect();
    let istart = (0..n).find(|&k| ik[k] && !ik[(k + n - 1) % n]).unwrap_or(0);
    let mut arc = vec![];
    for k in 0..n {
        let i = (istart + k) % n;
        if ik[i] {
            arc.push(inner[i]);
        } else {
            break;
        }
    }
    let last = *pts.last().unwrap();
    if !arc.is_empty() && arc[0].distance(last) > arc[arc.len() - 1].distance(last) {
        arc.reverse();
    }
    pts.extend(arc);
    pts
}

/// A crescent moon with a soft halo.
pub fn moon(pos: Vec2, r: f64, color: Color) -> Mobject {
    let pts = crescent_points(r, v2(0.45 * r, 0.25 * r), 0.85 * r, 160);
    group(vec![
        circle(r * 1.9).move_to(pos).fill(color, 0.07).no_stroke(),
        circle(r * 1.35).move_to(pos).fill(color, 0.08).no_stroke(),
        polygon(&pts).shift(pos).fill(color, 1.0).no_stroke(),
    ])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeKind {
    Round,
    Pine,
    Banyan,
}

impl TreeKind {
    pub fn parse(s: &str) -> TreeKind {
        match s {
            "pine" => TreeKind::Pine,
            "banyan" | "neem" => TreeKind::Banyan,
            _ => TreeKind::Round,
        }
    }
}

/// A tree rooted at `pos`. `sway` (radians) bends the canopy for wind.
pub fn tree(pos: Vec2, height: f64, kind: TreeKind, leaf: Color, sway: f64, seed: u64) -> Mobject {
    let trunk = rgb8(112, 78, 56);
    let h = height;
    let mut parts = vec![];
    let mut rng = Rng::new(seed);
    match kind {
        TreeKind::Round => {
            parts.push(
                polygon(&[
                    pos + v2(-0.06 * h, 0.0),
                    pos + v2(0.06 * h, 0.0),
                    pos + v2(0.03 * h, 0.58 * h),
                    pos + v2(-0.03 * h, 0.58 * h),
                ])
                .fill(trunk, 1.0)
                .no_stroke(),
            );
            let c = pos + v2(0.0, 0.7 * h);
            let mut canopy = vec![];
            for k in 0..7 {
                let a = k as f64 / 7.0 * TAU + 0.3;
                let p = c + Vec2::polar(0.17 * h, a) + v2(rng.range(-0.03, 0.03) * h, 0.0);
                let col = if k % 2 == 0 { leaf } else { shade(leaf, 0.12) };
                canopy.push(circle(0.17 * h).move_to(p).fill(col, 1.0).no_stroke());
            }
            canopy.push(
                circle(0.2 * h)
                    .move_to(c + v2(0.03 * h, 0.04 * h))
                    .fill(tint(leaf, 0.1), 1.0)
                    .no_stroke(),
            );
            parts.push(group(canopy).rotate_about(-sway, pos + v2(0.0, 0.5 * h)));
        }
        TreeKind::Pine => {
            parts.push(
                rectangle(0.06 * h, 0.25 * h)
                    .move_to(pos + v2(0.0, 0.12 * h))
                    .fill(trunk, 1.0)
                    .no_stroke(),
            );
            let mut tiers = vec![];
            for k in 0..3 {
                let y0 = 0.18 * h + k as f64 * 0.24 * h;
                let w = (0.3 - 0.07 * k as f64) * h;
                let col = if k % 2 == 0 { leaf } else { shade(leaf, 0.1) };
                tiers.push(
                    polygon(&[
                        pos + v2(-w, y0),
                        pos + v2(w, y0),
                        pos + v2(0.0, y0 + 0.4 * h),
                    ])
                    .fill(col, 1.0)
                    .no_stroke(),
                );
            }
            parts.push(group(tiers).rotate_about(-sway, pos));
        }
        TreeKind::Banyan => {
            parts.push(
                polygon(&[
                    pos + v2(-0.12 * h, 0.0),
                    pos + v2(0.12 * h, 0.0),
                    pos + v2(0.07 * h, 0.58 * h),
                    pos + v2(-0.07 * h, 0.58 * h),
                ])
                .fill(trunk, 1.0)
                .no_stroke(),
            );
            let mut canopy = vec![];
            for k in 0..7 {
                let x = -0.5 * h + k as f64 * 0.16 * h + rng.range(-0.03, 0.03) * h;
                let y0 = rng.range(0.05, 0.3) * h;
                canopy.push(
                    line(pos + v2(x, 0.62 * h), pos + v2(x + 0.01 * h, y0))
                        .stroke(shade(trunk, 0.1), 1.6),
                );
            }
            for k in 0..11 {
                let x = -0.55 * h + k as f64 * 0.11 * h;
                let y = 0.7 * h
                    + 0.12 * h * (1.0 - (x / (0.6 * h)).powi(2))
                    + rng.range(-0.04, 0.04) * h;
                let col = if k % 3 == 0 { shade(leaf, 0.12) } else { leaf };
                canopy.push(
                    circle(rng.range(0.13, 0.17) * h)
                        .move_to(pos + v2(x, y))
                        .fill(col, 1.0)
                        .no_stroke(),
                );
            }
            parts.push(group(canopy).rotate_about(-sway * 0.5, pos + v2(0.0, 0.5 * h)));
        }
    }
    group(parts)
}

/// A little house; `lit` ∈ [0, 1] warms the window (night).
pub fn house(pos: Vec2, w: f64, wall: Color, roof: Color, lit: f64) -> Mobject {
    let h = 0.7 * w;
    let warm = rgb8(255, 210, 110);
    let win = rgb8(80, 70, 70).mix(warm, lit as f32);
    let mut parts = vec![
        rectangle(w, h)
            .move_to(pos + v2(0.0, h / 2.0))
            .fill(wall, 1.0)
            .no_stroke(),
        polygon(&[
            pos + v2(-0.62 * w, h),
            pos + v2(0.62 * w, h),
            pos + v2(0.0, h + 0.5 * w),
        ])
        .fill(roof, 1.0)
        .no_stroke(),
        rounded_rectangle(0.2 * w, 0.42 * h, 0.03 * w)
            .move_to(pos + v2(-0.18 * w, 0.21 * h))
            .fill(shade(roof, 0.3), 1.0)
            .no_stroke(),
        rectangle(0.22 * w, 0.22 * w)
            .move_to(pos + v2(0.22 * w, 0.55 * h))
            .fill(win, 1.0)
            .stroke(shade(wall, 0.35), 2.0),
        line(
            pos + v2(0.22 * w, 0.55 * h - 0.11 * w),
            pos + v2(0.22 * w, 0.55 * h + 0.11 * w),
        )
        .stroke(shade(wall, 0.35), 1.5),
    ];
    if lit > 0.05 {
        parts.push(
            circle(0.3 * w)
                .move_to(pos + v2(0.22 * w, 0.55 * h))
                .fill(warm, (0.2 * lit) as f32)
                .no_stroke(),
        );
    }
    group(parts)
}

/// A puffy cloud of width `w`.
pub fn cloud(pos: Vec2, w: f64, color: Color) -> Mobject {
    let r = w * 0.18;
    group(vec![
        circle(r)
            .move_to(pos + v2(-0.25 * w, 0.0))
            .fill(color, 1.0)
            .no_stroke(),
        circle(r * 1.4)
            .move_to(pos + v2(-0.02 * w, 0.08 * w))
            .fill(color, 1.0)
            .no_stroke(),
        circle(r * 1.1)
            .move_to(pos + v2(0.24 * w, 0.03 * w))
            .fill(color, 1.0)
            .no_stroke(),
        rounded_rectangle(0.8 * w, 0.22 * w, 0.1 * w)
            .move_to(pos + v2(0.0, -0.06 * w))
            .fill(color, 1.0)
            .no_stroke(),
    ])
}

/// A full landscape for `rect`: sky, sun (day) or moon and twinkling stars
/// (night), drifting clouds and three hill layers. `day` ∈ [0, 1], 0 = night.
pub fn landscape(rect: Rect, pal: &Palette, t: f64, day: f64, seed: u64) -> Mobject {
    let day = clamp01(day);
    let night = Palette::indigo_night();
    let mix = |a: Color, b: Color| b.mix(a, day as f32);
    let w = rect.width();
    let h = rect.height();
    let mut parts = vec![sky(
        rect,
        mix(pal.sky[0], night.sky[0]),
        mix(pal.sky[1], night.sky[1]),
    )];
    if day > 0.02 {
        parts.push(
            sun(
                v2(rect.min.x + 0.78 * w, rect.min.y + lerp(0.35, 0.8, day) * h),
                0.065 * h,
                rgb8(255, 240, 180),
            )
            .opacity(day as f32),
        );
    }
    if day < 0.98 {
        let mut rng = Rng::new(seed + 99);
        let mut stars = vec![];
        for _ in 0..70 {
            let p = v2(
                rng.range(rect.min.x, rect.max.x),
                rng.range(rect.min.y + 0.45 * h, rect.max.y),
            );
            let tw = 0.5 + 0.5 * (t * rng.range(1.0, 3.0) + rng.range(0.0, TAU)).sin();
            let r = 0.012 + 0.014 * tw;
            stars.push(
                circle(r)
                    .move_to(p)
                    .fill(WHITE, ((1.0 - day) * (0.35 + 0.65 * tw)) as f32)
                    .no_stroke(),
            );
        }
        parts.push(group(stars));
        parts.push(
            moon(
                v2(rect.min.x + 0.22 * w, rect.min.y + 0.8 * h),
                0.06 * h,
                rgb8(255, 244, 210),
            )
            .opacity((1.0 - day) as f32),
        );
    }
    let cc = tint(pal.paper, 0.4).mix(rgb8(110, 120, 170), (1.0 - day) as f32 * 0.7);
    for k in 0..3 {
        let span = w + 2.0;
        let x = rect.min.x - 1.0
            + ((0.15 + 0.33 * k as f64) * w + t * 0.15 * (1.0 + k as f64 * 0.3)).rem_euclid(span);
        let y = rect.max.y - (0.14 + 0.06 * k as f64) * h;
        parts.push(cloud(v2(x, y), 1.0 + 0.3 * k as f64, cc).opacity(0.92));
    }
    let mut hp = *pal;
    for (i, g) in hp.ground.iter_mut().enumerate() {
        *g = g.mix(night.ground[i], (1.0 - day) as f32 * 0.8);
    }
    parts.push(layered_hills(rect, &hp, seed));
    group(parts)
}

// ------------------------------------------------------------------ bubbles & captions

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BubbleKind {
    Speech,
    Thought,
    Shout,
    Whisper,
}

impl BubbleKind {
    pub fn parse(s: &str) -> BubbleKind {
        match s.to_ascii_lowercase().as_str() {
            "thought" | "think" => BubbleKind::Thought,
            "shout" | "yell" => BubbleKind::Shout,
            "whisper" => BubbleKind::Whisper,
            _ => BubbleKind::Speech,
        }
    }
}

/// A comic bubble with wrapped handwriting text centred at `center`, its tail
/// pointing at `tail`. `p` ∈ [0, 1]: the bubble pops in over the first 25 %,
/// then the text types on.
pub fn bubble(
    kind: BubbleKind,
    text: &str,
    center: Vec2,
    max_width: f64,
    tail: Vec2,
    size: f64,
    p: f64,
) -> Mobject {
    let ink = rgb8(40, 32, 36);
    let fill = match kind {
        BubbleKind::Shout => rgb8(255, 238, 160),
        BubbleKind::Thought => rgb8(244, 247, 255),
        _ => WHITE,
    };
    let font_col = if kind == BubbleKind::Whisper {
        rgb8(96, 96, 118)
    } else {
        ink
    };
    let lead = 1.18;
    let lines = wrap_lines(text, "hand", size, &|_| max_width);
    let text_h = (lines.len() as f64 - 1.0) * size * lead + size;
    let tw = lines
        .iter()
        .map(|l| measure(l, "hand", size))
        .fold(0.0, f64::max);
    let bw = tw + 0.55 + size;
    let bh = text_h + 0.45;
    let pop = ease_out_back(p / 0.25);
    let text_p = clamp01((p - 0.25) / 0.6);
    let mut parts: Vec<Mobject> = vec![];
    let to_tail = tail - center;
    let tail_dir = if to_tail.length() < 1e-9 {
        v2(0.0, -1.0)
    } else {
        to_tail.normalize()
    };
    match kind {
        BubbleKind::Speech | BubbleKind::Whisper => {
            let base = center + v2(tail_dir.x * bw * 0.22, tail_dir.y * bh * 0.3);
            let n = tail_dir.perp() * (0.2 + 0.08 * size);
            let style = |m: Mobject| {
                let m = m.fill(fill, 1.0).stroke(ink, 2.6);
                if kind == BubbleKind::Whisper {
                    m.dashed(0.09, 0.07)
                } else {
                    m
                }
            };
            parts.push(style(polygon(&[base + n, tail, base - n])));
            parts.push(style(
                rounded_rectangle(bw, bh, (bh * 0.5).min(0.42)).move_to(center),
            ));
            // erase the seam where the tail joins
            let inner = (bh * 0.5 - 0.03).max(0.0);
            let seam_end = center + v2(tail_dir.x * (bw * 0.5 + 0.2), tail_dir.y * (inner + 0.2));
            let _ = seam_end;
            parts.push(
                polygon(&[
                    base + n * 0.82,
                    base + (tail - base) * 0.35,
                    base - n * 0.82,
                ])
                .fill(fill, 1.0)
                .no_stroke(),
            );
        }
        BubbleKind::Thought => {
            let per = (((bw + bh) * 2.0) / 0.4).round().max(10.0) as usize;
            for k in 0..per {
                let a = k as f64 / per as f64 * TAU;
                let q = center + v2(a.cos() * bw * 0.5, a.sin() * bh * 0.5);
                let r = 0.19 + 0.05 * ((k * 7) % 3) as f64;
                parts.push(circle(r).move_to(q).fill(fill, 1.0).stroke(ink, 2.2));
            }
            parts.push(
                ellipse(bw * 1.0, bh * 1.0)
                    .move_to(center)
                    .fill(fill, 1.0)
                    .no_stroke(),
            );
            let edge = center + v2(tail_dir.x * bw * 0.5, tail_dir.y * bh * 0.5);
            for k in 0..3 {
                let f = 0.3 + 0.28 * k as f64;
                let q = edge.lerp(tail, f);
                parts.push(
                    circle(0.12 - 0.03 * k as f64)
                        .move_to(q)
                        .fill(fill, 1.0)
                        .stroke(ink, 2.0),
                );
            }
        }
        BubbleKind::Shout => {
            let spikes = 18;
            let mut pts = vec![];
            for k in 0..spikes * 2 {
                let a = k as f64 / (spikes * 2) as f64 * TAU;
                let jitter = 1.0 + 0.08 * hash_unit(hash2(k as i64, 3, 17));
                let rr = if k % 2 == 0 { 1.2 * jitter } else { 0.96 };
                pts.push(center + v2(a.cos() * bw * 0.6 * rr, a.sin() * bh * 0.66 * rr));
            }
            let base = center + v2(tail_dir.x * bw * 0.45, tail_dir.y * bh * 0.45);
            let n = tail_dir.perp() * 0.16;
            parts.push(
                polygon(&[base + n, tail, base - n])
                    .fill(fill, 1.0)
                    .stroke(ink, 2.6),
            );
            parts.push(polygon(&pts).fill(fill, 1.0).stroke(ink, 3.0));
        }
    }
    let shown = typed(text, text_p).chars().count();
    let tl = reveal_lines(&lines, shown);
    let top_left = v2(center.x - tw / 2.0, center.y + text_h / 2.0);
    parts.push(text_block(&tl, "hand", size, font_col, top_left, lead));
    let m = group(parts);
    if p >= 0.25 {
        m
    } else {
        m.scale_about(pop.max(0.001), center.lerp(tail, 0.3))
            .opacity(clamp01(p / 0.1) as f32)
    }
}

/// A storybook caption: parchment box, book serif, typed reveal, corner
/// flourishes in the palette's accent.
pub fn narration_box(
    text: &str,
    center: Vec2,
    width: f64,
    size: f64,
    pal: &Palette,
    p: f64,
) -> Mobject {
    let lead = 1.3;
    let lines = wrap_lines(text, "story", size, &|_| width - 0.7);
    let text_h = (lines.len() as f64 - 1.0) * size * lead + size;
    let h = text_h + 0.45;
    let shown = typed(text, p).chars().count();
    let tl = reveal_lines(&lines, shown);
    let paper = pal.paper;
    let mut parts = vec![rounded_rectangle(width, h, 0.12)
        .move_to(center)
        .fill(paper, 0.97)
        .stroke(shade(paper, 0.35), 1.6)
        .shadow(Shadow {
            offset: v2(0.05, -0.06),
            blur: 0.12,
            color: BLACK.with_alpha(0.3),
        })];
    let inset = 0.09;
    for (sx, sy) in [(-1.0f64, 1.0f64), (1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
        let c = center + v2(sx * (width / 2.0 - inset), sy * (h / 2.0 - inset));
        let a = v2(-sx, 0.0) * 0.22;
        let b = v2(0.0, -sy) * 0.22;
        parts.push(
            polyline(&[c + a, c, c + b])
                .stroke(pal.accent[1], 2.0)
                .fill(paper, 0.0),
        );
        parts.push(
            circle(0.025)
                .move_to(c + (a + b) * 0.35)
                .fill(pal.accent[0], 1.0)
                .no_stroke(),
        );
    }
    let top_left = v2(center.x - width / 2.0 + 0.35, center.y + text_h / 2.0);
    parts.push(text_block(&tl, "story", size, pal.ink, top_left, lead));
    group(parts)
}

/// One timed line of a story: who says it (`None` = narrator), when, and how.
#[derive(Clone, Debug, PartialEq)]
pub struct StoryLine {
    pub speaker: Option<usize>,
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub kind: BubbleKind,
}

/// A timed script: narration + dialogue → captions, bubbles and lip-flap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StoryScript {
    pub lines: Vec<StoryLine>,
}

impl StoryScript {
    pub fn new() -> Self {
        Self::default()
    }
    /// Narrator caption.
    pub fn narrate(mut self, start: f64, end: f64, text: &str) -> Self {
        self.lines.push(StoryLine {
            speaker: None,
            start,
            end,
            text: text.to_string(),
            kind: BubbleKind::Speech,
        });
        self
    }
    /// Dialogue for character `who`.
    pub fn say(mut self, who: usize, start: f64, end: f64, kind: BubbleKind, text: &str) -> Self {
        self.lines.push(StoryLine {
            speaker: Some(who),
            start,
            end,
            text: text.to_string(),
            kind,
        });
        self
    }
    /// Lines active at `t`, with local progress ∈ [0, 1).
    pub fn active(&self, t: f64) -> Vec<(&StoryLine, f64)> {
        self.lines
            .iter()
            .filter(|l| t >= l.start && t < l.end)
            .map(|l| (l, (t - l.start) / (l.end - l.start).max(1e-9)))
            .collect()
    }
    /// Talking envelope for `who`: on while that line's text types
    /// (bubble progress 0.25–0.85, matching [`bubble`]). Thoughts are silent.
    pub fn talking(&self, who: usize, t: f64) -> f64 {
        self.lines
            .iter()
            .filter(|l| l.speaker == Some(who) && l.kind != BubbleKind::Thought)
            .map(|l| {
                let d = l.end - l.start;
                let a = l.start + 0.25 * d;
                let b = l.start + 0.85 * d;
                smoothstep(a, a + 0.08, t) * (1.0 - smoothstep(b - 0.08, b, t))
            })
            .fold(0.0, f64::max)
    }
    pub fn duration(&self) -> f64 {
        self.lines.iter().map(|l| l.end).fold(0.0, f64::max)
    }
}

// ------------------------------------------------------------------ transitions

/// Iris wipe: everything outside a shrinking circle around `center` is
/// `color`. `p` = 0 fully open, 1 fully closed.
pub fn iris(p: f64, center: Vec2, color: Color) -> Mobject {
    let (fw, fh) = crate::mobject::frame_size();
    let rmax = (v2(fw, fh) * 0.5 + v2(center.x.abs(), center.y.abs())).length();
    let r = rmax * (1.0 - ease_io(p));
    let mut path = Path::polyline(&[v2(-fw, -fh), v2(fw, -fh), v2(fw, fh), v2(-fw, fh)], true);
    if r > 1e-3 {
        path.subpaths.extend(Path::ellipse(center, r, r).subpaths);
    }
    Mobject::from_path(path)
        .fill(color, 1.0)
        .no_stroke()
        .map_style(|s| s.fill_rule = FillRule::EvenOdd)
}

/// Ink bleed: noisy pigment blots spread from a few seeds until `rect` is
/// covered at `p` = 1; edges carry darker pigment like wet ink.
pub fn ink_bleed(p: f64, rect: Rect, color: Color, seed: u64, px_w: u32) -> Mobject {
    let mut rng = Rng::new(seed);
    let seeds: Vec<Vec2> = (0..4)
        .map(|_| {
            v2(
                rng.range(rect.min.x + 1.0, rect.max.x - 1.0),
                rng.range(rect.min.y + 1.0, rect.max.y - 1.0),
            )
        })
        .collect();
    let reach = rect.width().max(rect.height()) * 0.75;
    let thr = ease_io(p) * reach * 1.3 - 0.4;
    let dark = shade(color, 0.4);
    field_image(rect, px_w, move |q| {
        if p <= 0.0 {
            return Color::TRANSPARENT;
        }
        if p >= 1.0 {
            return color;
        }
        let d = seeds
            .iter()
            .map(|s| s.distance(q))
            .fold(f64::INFINITY, f64::min);
        let n = fbm2(q * 0.7, 4, 2.0, 0.5, seed) + fbm2(q * 3.0, 2, 2.0, 0.5, seed + 1) * 0.12;
        let v = thr - d + n * reach * 0.2;
        if v <= 0.0 {
            Color::TRANSPARENT
        } else {
            let edge = (v / 0.03).min(1.0) as f32;
            let rim = (1.0 - (v / 0.4).min(1.0)) as f32;
            color.mix(dark, rim * 0.85).with_alpha(edge)
        }
    })
}

/// Page turn: `current` (laid out in `rect`) curls from the right edge to the
/// left, revealing `next`. `p` ∈ [0, 1].
pub fn page_turn(current: &Mobject, next: &Mobject, rect: Rect, p: f64, paper: Color) -> Mobject {
    let p = ease_io(p);
    let (l, r) = (rect.min.x, rect.max.x);
    let xf = lerp(r, l, p);
    let flap_l = (2.0 * xf - r).max(l);
    let (y0, y1) = (rect.min.y - 0.02, rect.max.y + 0.02);
    let clip =
        |m: &Mobject, x0: f64, x1: f64| clip_to(m.clone(), Rect::new(v2(x0, y0), v2(x1, y1)));
    let mut parts = vec![];
    if p >= 1.0 {
        return next.clone();
    }
    if p <= 0.0 {
        return current.clone();
    }
    parts.push(clip(next, xf, r));
    let sw = (0.6 * (r - xf)).min(0.7);
    if sw > 1e-3 {
        parts.push(
            rectangle(sw, rect.height())
                .move_to(v2(xf + sw / 2.0, rect.center().y))
                .fill(BLACK, 1.0)
                .no_stroke()
                .map_style(move |s| {
                    s.fill = Paint::linear(
                        v2(xf, 0.0),
                        v2(xf + sw, 0.0),
                        &[BLACK.with_alpha(0.35), BLACK.with_alpha(0.0)],
                    );
                }),
        );
    }
    parts.push(clip(current, l, xf));
    if xf - flap_l > 1e-3 {
        let bow = 0.18 * (PI * p).sin();
        let n = 18;
        let mut pts = vec![v2(xf, rect.min.y), v2(xf, rect.max.y)];
        for k in 0..=n {
            let y = rect.max.y - rect.height() * k as f64 / n as f64;
            let u = (y - rect.center().y) / (rect.height() / 2.0);
            pts.push(v2(flap_l - bow * (1.0 - u * u) * 0.5 + bow * 0.5, y));
        }
        let fl = flap_l;
        parts.push(
            polygon(&pts)
                .fill(paper, 1.0)
                .stroke(shade(paper, 0.3), 1.0)
                .map_style(move |s| {
                    s.fill = Paint::linear(
                        v2(fl, 0.0),
                        v2(xf, 0.0),
                        &[shade(paper, 0.06), tint(paper, 0.35), shade(paper, 0.25)],
                    );
                })
                .shadow(Shadow {
                    offset: v2(-0.08, -0.05),
                    blur: 0.18,
                    color: BLACK.with_alpha(0.35),
                }),
        );
        // the printed side faintly shows through the back of the page
        let ghost = current
            .clone()
            .apply_affine(Affine {
                a: -1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 2.0 * xf,
                f: 0.0,
            })
            .opacity(0.07);
        parts.push(clip_to(ghost, Rect::new(v2(flap_l + 0.05, y0), v2(xf, y1))));
    }
    group(parts)
}

/// Paper fold (accordion): `content` in `rect` folds into `strips` panels
/// collapsing to the left as `p` → 1, with alternating light/dark creases.
pub fn paper_fold(content: &Mobject, rect: Rect, p: f64, strips: usize) -> Mobject {
    let p = ease_io(p);
    let n = strips.max(2);
    let sw = rect.width() / n as f64;
    let cw = (sw * (p * PI / 2.0).cos()).max(1e-3);
    let mut parts = vec![];
    for i in 0..n {
        let x0 = rect.min.x + i as f64 * sw;
        let nx0 = rect.min.x + i as f64 * cw;
        let strip = clip_to(
            content.clone(),
            Rect::new(v2(x0, rect.min.y), v2(x0 + sw, rect.max.y)),
        )
        .apply_affine(
            Affine::translate(v2(-x0, 0.0))
                .then(Affine::scale(cw / sw, 1.0))
                .then(Affine::translate(v2(nx0, 0.0))),
        );
        parts.push(strip);
        let dark = if i % 2 == 0 { 0.08 } else { 0.42 };
        parts.push(
            rectangle(cw, rect.height())
                .move_to(v2(nx0 + cw / 2.0, rect.center().y))
                .fill(BLACK, (dark * p) as f32)
                .no_stroke(),
        );
    }
    group(parts)
}

// ------------------------------------------------------------------ page layout

/// Clip `m` to the axis-aligned rect `r` (intersected with any existing clip,
/// so nested clips — a panel inside a page inside a page turn — compose).
pub fn clip_to(m: Mobject, r: Rect) -> Mobject {
    m.map_style(move |s| {
        s.clip = Some(match s.clip {
            Some((lo, hi)) => {
                let a = v2(lo.x.max(r.min.x), lo.y.max(r.min.y));
                let b = v2(hi.x.min(r.max.x), hi.y.min(r.max.y));
                (a, v2(b.x.max(a.x), b.y.max(a.y)))
            }
            None => (r.min, r.max),
        })
    })
}

/// An open picture book filling `rect`: cover board, stacked page edges and two
/// cream pages that shade into the gutter. Returns (book, left page, right page).
pub fn open_book(rect: Rect, paper: Color, cover: Color) -> (Mobject, Rect, Rect) {
    let c = rect.center();
    let w = rect.width();
    let h = rect.height();
    let mut parts = vec![rounded_rectangle(w, h, 0.14)
        .move_to(c)
        .fill(cover, 1.0)
        .stroke(shade(cover, 0.4), 2.0)
        .shadow(Shadow {
            offset: v2(0.08, -0.14),
            blur: 0.3,
            color: BLACK.with_alpha(0.45),
        })];
    let pw = w / 2.0 - 0.24;
    let ph = h - 0.4;
    for k in (1..5).rev() {
        let off = k as f64 * 0.028;
        for side in [-1.0, 1.0] {
            parts.push(
                rectangle(pw, ph)
                    .move_to(c + v2(side * (pw / 2.0 + 0.02 + off * 0.5), -off))
                    .fill(shade(paper, 0.05 + 0.035 * k as f32), 1.0)
                    .stroke(shade(paper, 0.32), 0.8),
            );
        }
    }
    let lp = Rect::from_center(c + v2(-(pw / 2.0 + 0.02), 0.0), pw, ph);
    let rp = Rect::from_center(c + v2(pw / 2.0 + 0.02, 0.0), pw, ph);
    for (r, dir) in [(lp, 1.0f64), (rp, -1.0f64)] {
        let edge = if dir > 0.0 { r.max.x } else { r.min.x };
        parts.push(
            rectangle(pw, ph)
                .move_to(r.center())
                .fill(paper, 1.0)
                .stroke(shade(paper, 0.25), 0.8)
                .map_style(move |s| {
                    s.fill = Paint::linear(
                        v2(edge - dir * pw, 0.0),
                        v2(edge, 0.0),
                        &[paper, paper, paper, shade(paper, 0.2)],
                    );
                }),
        );
    }
    parts.push(line(c + v2(0.0, -ph / 2.0), c + v2(0.0, ph / 2.0)).stroke(shade(paper, 0.45), 1.2));
    (group(parts), lp, rp)
}

/// Faint paper-grain overlay for `rect` (expensive: compute once and reuse).
pub fn paper_grain(rect: Rect, seed: u64, px_w: u32) -> Mobject {
    field_image(rect, px_w, move |q| {
        let n =
            fbm2(q * 6.0, 3, 2.0, 0.5, seed) * 0.6 + fbm2(q * 30.0, 1, 2.0, 0.5, seed + 3) * 0.4;
        let a = (n.abs() * 0.14) as f32;
        if n > 0.0 {
            WHITE.with_alpha(a)
        } else {
            rgb8(120, 90, 60).with_alpha(a)
        }
    })
}

/// A paragraph with an illuminated drop capital spanning three lines; the
/// text wraps around it. Returns the laid-out group.
pub fn drop_cap_paragraph(
    text: &str,
    top_left: Vec2,
    width: f64,
    size: f64,
    ink: Color,
    cap: Color,
) -> Mobject {
    let mut chars = text.chars();
    let first = chars.next().unwrap_or(' ');
    let rest: String = chars.collect();
    let lead = 1.35;
    let cap_lines = 3usize;
    let cap_h = size * lead * (cap_lines as f64 - 1.0) + size;
    let letter = Text::new(&first.to_string())
        .font("story")
        .bold()
        .size(cap_h)
        .color(tint(cap, 0.85))
        .build()
        .set_height(cap_h * 0.62);
    let box_w = cap_h;
    let box_c = top_left + v2(box_w / 2.0, -cap_h / 2.0);
    let decor = group(vec![
        rounded_rectangle(box_w, cap_h, 0.06)
            .move_to(box_c)
            .fill(cap, 1.0)
            .stroke(shade(cap, 0.35), 1.5),
        rounded_rectangle(box_w - 0.12, cap_h - 0.12, 0.04)
            .move_to(box_c)
            .fill(cap, 0.0)
            .stroke(tint(cap, 0.55), 1.2),
        letter.move_to(box_c),
    ]);
    let indent = box_w + 0.18;
    let lines = wrap_lines(&rest, "story", size, &|i| {
        if i < cap_lines {
            width - indent
        } else {
            width
        }
    });
    let mut parts = vec![decor];
    for (i, l) in lines.iter().enumerate() {
        let x = if i < cap_lines {
            top_left.x + indent
        } else {
            top_left.x
        };
        let y = top_left.y - size * 0.5 - i as f64 * size * lead;
        parts.push(story_text(l, size, ink).move_to_aligned(v2(x, y), v2(-1.0, 0.0)));
    }
    group(parts)
}

/// Comic panel rectangles in a `rows × cols` grid inside `rect`, row-major
/// from the top-left.
pub fn comic_panels(rect: Rect, rows: usize, cols: usize, gutter: f64) -> Vec<Rect> {
    let rows = rows.max(1);
    let cols = cols.max(1);
    let pw = (rect.width() - gutter * (cols as f64 - 1.0)) / cols as f64;
    let ph = (rect.height() - gutter * (rows as f64 - 1.0)) / rows as f64;
    let mut out = vec![];
    for r in 0..rows {
        for c in 0..cols {
            let x0 = rect.min.x + c as f64 * (pw + gutter);
            let y1 = rect.max.y - r as f64 * (ph + gutter);
            out.push(Rect::new(v2(x0, y1 - ph), v2(x0 + pw, y1)));
        }
    }
    out
}

/// An inked comic panel border.
pub fn panel_frame(r: Rect, ink: Color) -> Mobject {
    rectangle(r.width(), r.height())
        .move_to(r.center())
        .fill(WHITE, 0.0)
        .stroke(ink, 4.0)
}

/// A small caption tab in a panel's top-left corner ("Meanwhile…").
pub fn panel_caption(r: Rect, text: &str, size: f64, bg: Color, ink: Color) -> Mobject {
    let t = story_italic(text, size, ink);
    let w = t.width() + 0.3;
    let h = size + 0.18;
    let c = v2(r.min.x + w / 2.0, r.max.y - h / 2.0);
    group(vec![
        rectangle(w, h).move_to(c).fill(bg, 1.0).stroke(ink, 2.5),
        t.move_to(c),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_prefix() {
        assert_eq!(typed("hello", 0.0), "");
        assert_eq!(typed("hello", 0.4), "he");
        assert_eq!(typed("hello", 1.0), "hello");
        let lines = vec!["ab cd".to_string(), "ef".to_string()];
        assert_eq!(reveal_lines(&lines, 7), vec!["ab cd", "e"]);
    }

    #[test]
    fn wrap_respects_width() {
        let text =
            "Once upon a time a thirsty crow found a pitcher with a little water at the bottom";
        let lines = wrap_lines(text, "story", 0.3, &|_| 3.0);
        assert!(lines.len() >= 3);
        for l in &lines {
            assert!(measure(l, "story", 0.3) <= 3.0 + 1e-6);
        }
        assert_eq!(lines.join(" "), text);
    }

    #[test]
    fn blink_is_brief_and_periodic() {
        let closed = (0..2000)
            .filter(|i| blink_amount(*i as f64 * 0.01, 3) > 0.5)
            .count();
        assert!(closed > 0 && closed < 80, "{closed}");
    }

    #[test]
    fn walk_cycle_is_periodic_and_lerp_endpoints() {
        let a = Pose::walk(0.25);
        let b = Pose::walk(1.25);
        assert!((a.front_hip - b.front_hip).abs() < 1e-9);
        let i = Pose::idle(0.0);
        let w = Pose::wave(0.0);
        assert_eq!(i.lerp(&w, 0.0), i);
        let e = i.lerp(&w, 1.0);
        assert!((e.front_shoulder - w.front_shoulder).abs() < 1e-12);
        assert!((e.front_elbow - w.front_elbow).abs() < 1e-12);
    }

    #[test]
    fn joints_mirror_with_facing() {
        let c = Character::girl();
        let p = Pose::point();
        let r = c.joints(&p, v2(0.0, 0.0), 1.0);
        let l = c.joints(&p, v2(0.0, 0.0), -1.0);
        assert!((r.front_hand.x + l.front_hand.x).abs() < 1e-9);
        assert!(r.front_hand.x > 0.3);
        assert!(r.top.y > 1.9 && r.top.y < 2.3, "{}", r.top.y);
    }

    #[test]
    fn script_talking_envelope() {
        let s = StoryScript::new()
            .narrate(0.0, 2.0, "Once upon a time")
            .say(0, 2.0, 5.0, BubbleKind::Speech, "Hello!")
            .say(1, 5.0, 8.0, BubbleKind::Thought, "Hmm");
        assert_eq!(s.talking(0, 1.0), 0.0);
        assert!(s.talking(0, 3.5) > 0.9);
        assert_eq!(s.talking(1, 6.0), 0.0);
        assert_eq!(s.active(2.5).len(), 1);
        assert_eq!(s.duration(), 8.0);
    }

    #[test]
    fn panels_tile_the_rect() {
        let r = Rect::from_center(v2(0.0, 0.0), 10.0, 6.0);
        let ps = comic_panels(r, 2, 3, 0.2);
        assert_eq!(ps.len(), 6);
        assert!((ps[0].min.x - r.min.x).abs() < 1e-9);
        assert!((ps[5].max.x - r.max.x).abs() < 1e-9);
        assert!((ps[5].min.y - r.min.y).abs() < 1e-9);
    }

    #[test]
    fn crescent_is_inside_moon_and_outside_cutter() {
        let cut = v2(0.45, 0.25);
        let pts = crescent_points(1.0, cut, 0.85, 120);
        assert!(pts.len() > 40);
        for p in &pts {
            assert!(p.length() <= 1.0 + 1e-6);
            assert!(p.distance(cut) >= 0.85 - 0.05);
        }
    }

    #[test]
    fn palettes_by_name() {
        assert_eq!(Palette::named("Monsoon").name, "monsoon");
        assert_eq!(Palette::named("nope").name, "marigold");
        assert_eq!(Palette::all().len(), 6);
    }

    #[test]
    fn drawing_produces_geometry() {
        let m = Character::grandpa().draw_at(&Pose::walk(0.3), 1.0, 1.0, -1.0, v2(1.0, -2.0));
        assert!(m.leaf_count() > 20);
        let f = Critter::fox().draw(0.5, 1.0, 0.3, 1.0, v2(0.0, 0.0));
        assert!(f.leaf_count() > 10);
        let b = bubble(
            BubbleKind::Thought,
            "What a lovely day",
            v2(0.0, 0.0),
            3.0,
            v2(-2.0, -2.0),
            0.3,
            1.0,
        );
        assert!(b.leaf_count() > 10);
        let pt = page_turn(
            &circle(1.0),
            &square(1.0),
            Rect::from_center(v2(0.0, 0.0), 4.0, 3.0),
            0.5,
            WHITE,
        );
        assert!(pt.leaf_count() >= 4);
    }
}
