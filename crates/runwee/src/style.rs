//! Visual style of a shape: fill/stroke paints, gradients, dashes, glow, shadow.

use crate::color::*;
use crate::math::*;

/// A paint source. Gradient coordinates are in scene units and move with the object.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Solid(Color),
    Linear {
        start: Vec2,
        end: Vec2,
        stops: Vec<(f32, Color)>,
    },
    Radial {
        center: Vec2,
        radius: f64,
        /// Optional focal point for spherical highlights.
        focal: Vec2,
        stops: Vec<(f32, Color)>,
    },
}

impl Default for Paint {
    fn default() -> Self {
        Paint::Solid(WHITE)
    }
}

impl From<Color> for Paint {
    fn from(c: Color) -> Self {
        Paint::Solid(c)
    }
}

impl Paint {
    pub fn linear(start: Vec2, end: Vec2, colors: &[Color]) -> Paint {
        let n = colors.len().max(2) - 1;
        Paint::Linear {
            start,
            end,
            stops: colors
                .iter()
                .enumerate()
                .map(|(i, c)| (i as f32 / n as f32, *c))
                .collect(),
        }
    }
    pub fn radial(center: Vec2, radius: f64, colors: &[Color]) -> Paint {
        let n = colors.len().max(2) - 1;
        Paint::Radial {
            center,
            radius,
            focal: center,
            stops: colors
                .iter()
                .enumerate()
                .map(|(i, c)| (i as f32 / n as f32, *c))
                .collect(),
        }
    }
    /// Representative solid colour (first stop for gradients).
    pub fn color(&self) -> Color {
        match self {
            Paint::Solid(c) => *c,
            Paint::Linear { stops, .. } | Paint::Radial { stops, .. } => {
                stops.first().map(|s| s.1).unwrap_or(WHITE)
            }
        }
    }
    pub fn transform(&mut self, a: &Affine) {
        match self {
            Paint::Solid(_) => {}
            Paint::Linear { start, end, .. } => {
                *start = a.apply(*start);
                *end = a.apply(*end);
            }
            Paint::Radial {
                center,
                radius,
                focal,
                ..
            } => {
                *center = a.apply(*center);
                *focal = a.apply(*focal);
                *radius *= a.mean_scale();
            }
        }
    }
    pub fn map_colors(&mut self, f: &dyn Fn(Color) -> Color) {
        match self {
            Paint::Solid(c) => *c = f(*c),
            Paint::Linear { stops, .. } | Paint::Radial { stops, .. } => {
                for s in stops {
                    s.1 = f(s.1)
                }
            }
        }
    }
    pub fn lerp(&self, o: &Paint, t: f64) -> Paint {
        let tf = t as f32;
        match (self, o) {
            (Paint::Solid(a), Paint::Solid(b)) => Paint::Solid(a.mix(*b, tf)),
            (
                Paint::Linear {
                    start: s1,
                    end: e1,
                    stops: st1,
                },
                Paint::Linear {
                    start: s2,
                    end: e2,
                    stops: st2,
                },
            ) if st1.len() == st2.len() => Paint::Linear {
                start: s1.lerp(*s2, t),
                end: e1.lerp(*e2, t),
                stops: st1
                    .iter()
                    .zip(st2)
                    .map(|(a, b)| (a.0 + (b.0 - a.0) * tf, a.1.mix(b.1, tf)))
                    .collect(),
            },
            (
                Paint::Radial {
                    center: c1,
                    radius: r1,
                    focal: f1,
                    stops: st1,
                },
                Paint::Radial {
                    center: c2,
                    radius: r2,
                    focal: f2,
                    stops: st2,
                },
            ) if st1.len() == st2.len() => Paint::Radial {
                center: c1.lerp(*c2, t),
                radius: lerp(*r1, *r2, t),
                focal: f1.lerp(*f2, t),
                stops: st1
                    .iter()
                    .zip(st2)
                    .map(|(a, b)| (a.0 + (b.0 - a.0) * tf, a.1.mix(b.1, tf)))
                    .collect(),
            },
            // Mixed kinds: blend through the representative colour.
            (a, b) => {
                if t < 0.5 {
                    let mut p = a.clone();
                    let target = b.color();
                    p.map_colors(&|c| c.mix(target, tf));
                    p
                } else {
                    let mut p = b.clone();
                    let src = a.color();
                    p.map_colors(&|c| src.mix(c, tf));
                    p
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineCap {
    Butt,
    #[default]
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineJoin {
    Miter,
    #[default]
    Round,
    Bevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Blend {
    #[default]
    Normal,
    Add,
    Screen,
    Multiply,
    Overlay,
}

/// Drop shadow parameters (scene units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub offset: Vec2,
    pub blur: f64,
    pub color: Color,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            offset: v2(0.06, -0.08),
            blur: 0.12,
            color: BLACK.with_alpha(0.6),
        }
    }
}

/// Complete style for a shape.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub fill: Paint,
    pub fill_opacity: f32,
    pub stroke: Paint,
    /// Stroke width in pixels at 1080p (manim convention, default 4).
    pub stroke_width: f32,
    pub stroke_opacity: f32,
    /// Extra stroke painted *behind* the fill (manim's background stroke, for legible text).
    pub background_stroke_width: f32,
    pub background_stroke: Color,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub fill_rule: FillRule,
    /// Dash pattern (dash length, gap length) in scene units.
    pub dash: Option<(f64, f64)>,
    pub dash_offset: f64,
    /// Glow radius in scene units (0 = none).
    pub glow: f64,
    pub glow_color: Option<Color>,
    pub glow_strength: f32,
    pub shadow: Option<Shadow>,
    /// Whole-object opacity multiplier (used by fades).
    pub opacity: f32,
    pub blend: Blend,
    /// Only paint inside this axis-aligned scene rectangle (min, max).
    pub clip: Option<(Vec2, Vec2)>,
}

pub const DEFAULT_STROKE_WIDTH: f32 = 4.0;

impl Default for Style {
    fn default() -> Self {
        Self {
            fill: Paint::Solid(WHITE),
            fill_opacity: 0.0,
            stroke: Paint::Solid(WHITE),
            stroke_width: DEFAULT_STROKE_WIDTH,
            stroke_opacity: 1.0,
            background_stroke_width: 0.0,
            background_stroke: BLACK,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            fill_rule: FillRule::NonZero,
            dash: None,
            dash_offset: 0.0,
            glow: 0.0,
            glow_color: None,
            glow_strength: 1.0,
            shadow: None,
            clip: None,
            opacity: 1.0,
            blend: Blend::Normal,
        }
    }
}

impl Style {
    pub fn stroked(color: Color, width: f32) -> Self {
        Self {
            stroke: Paint::Solid(color),
            stroke_width: width,
            fill: Paint::Solid(color),
            ..Default::default()
        }
    }
    pub fn filled(color: Color) -> Self {
        Self {
            fill: Paint::Solid(color),
            fill_opacity: 1.0,
            stroke: Paint::Solid(color),
            stroke_width: 0.0,
            ..Default::default()
        }
    }
    pub fn has_fill(&self) -> bool {
        self.fill_opacity > 0.0 && self.opacity > 0.0
    }
    pub fn has_stroke(&self) -> bool {
        self.stroke_opacity > 0.0 && self.stroke_width > 0.0 && self.opacity > 0.0
    }
    pub fn is_visible(&self) -> bool {
        self.has_fill() || self.has_stroke() || (self.glow > 0.0 && self.opacity > 0.0)
    }
    pub fn transform(&mut self, a: &Affine) {
        self.fill.transform(a);
        self.stroke.transform(a);
        if let Some((lo, hi)) = self.clip {
            let pts = [
                a.apply(lo),
                a.apply(hi),
                a.apply(v2(lo.x, hi.y)),
                a.apply(v2(hi.x, lo.y)),
            ];
            let mn = pts
                .iter()
                .fold(pts[0], |m, p| v2(m.x.min(p.x), m.y.min(p.y)));
            let mx = pts
                .iter()
                .fold(pts[0], |m, p| v2(m.x.max(p.x), m.y.max(p.y)));
            self.clip = Some((mn, mx));
        }
    }
    pub fn lerp(&self, o: &Style, t: f64) -> Style {
        let f = |a: f32, b: f32| lerp_f32(a, b, t);
        Style {
            fill: self.fill.lerp(&o.fill, t),
            fill_opacity: f(self.fill_opacity, o.fill_opacity),
            stroke: self.stroke.lerp(&o.stroke, t),
            stroke_width: f(self.stroke_width, o.stroke_width),
            stroke_opacity: f(self.stroke_opacity, o.stroke_opacity),
            background_stroke_width: f(self.background_stroke_width, o.background_stroke_width),
            background_stroke: self.background_stroke.mix(o.background_stroke, t as f32),
            line_cap: if t < 0.5 { self.line_cap } else { o.line_cap },
            line_join: if t < 0.5 { self.line_join } else { o.line_join },
            fill_rule: if t < 0.5 { self.fill_rule } else { o.fill_rule },
            dash: match (self.dash, o.dash) {
                (Some(a), Some(b)) => Some((lerp(a.0, b.0, t), lerp(a.1, b.1, t))),
                (a, b) => {
                    if t < 0.5 {
                        a
                    } else {
                        b
                    }
                }
            },
            dash_offset: lerp(self.dash_offset, o.dash_offset, t),
            glow: lerp(self.glow, o.glow, t),
            glow_color: match (self.glow_color, o.glow_color) {
                (Some(a), Some(b)) => Some(a.mix(b, t as f32)),
                (a, b) => {
                    if t < 0.5 {
                        a
                    } else {
                        b
                    }
                }
            },
            glow_strength: f(self.glow_strength, o.glow_strength),
            shadow: match (self.shadow, o.shadow) {
                (Some(a), Some(b)) => Some(Shadow {
                    offset: a.offset.lerp(b.offset, t),
                    blur: lerp(a.blur, b.blur, t),
                    color: a.color.mix(b.color, t as f32),
                }),
                (Some(a), None) => Some(Shadow {
                    color: a.color.fade(1.0 - t as f32),
                    ..a
                }),
                (None, Some(b)) => Some(Shadow {
                    color: b.color.fade(t as f32),
                    ..b
                }),
                (None, None) => None,
            },
            opacity: f(self.opacity, o.opacity),
            blend: if t < 0.5 { self.blend } else { o.blend },
            clip: if t < 0.5 { self.clip } else { o.clip },
        }
    }
}
