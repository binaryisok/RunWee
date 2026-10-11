//! HTML Canvas 2D API on top of RunWee mobjects.
//!
//! [`Canvas2D`] follows the semantics of `CanvasRenderingContext2D`: a current
//! path built with `move_to`/`line_to`/`arc`/…, a transform and style state
//! with `save`/`restore`, and `fill`/`stroke`/`fill_text` calls that paint.
//! Every paint call becomes a vector [`Mobject`] placed in a target rectangle of
//! the scene, so code written for a browser canvas (y down, pixel units) can be
//! drawn by RunWee nearly line for line, at any resolution.
//!
//! ```
//! use runwee::prelude::*;
//! use runwee::canvas2d::Canvas2D;
//! let mut ctx = Canvas2D::new(800.0, 400.0, Rect::from_center(ORIGIN, 14.0, 7.0));
//! ctx.set_fill_style_css("#3b82f6");
//! ctx.fill_rect(100.0, 100.0, 80.0, 40.0);
//! ctx.begin_path();
//! ctx.arc(400.0, 200.0, 50.0, 0.0, std::f64::consts::TAU, false);
//! ctx.set_stroke_style_css("rgba(16,185,129,0.8)");
//! ctx.set_line_width(3.0);
//! ctx.stroke();
//! let drawing: Mobject = ctx.finish();
//! ```

use crate::color::*;
use crate::fonts::{find_font, glyph_path, layout_line, line_metrics, FontId};
use crate::math::*;
use crate::mobject::Mobject;
use crate::path::{Path, Rect};
use crate::style::*;
use std::f64::consts::{PI, TAU};

/// Stroke widths in RunWee are "pixels at 1080p for an 8-unit-tall frame".
const STROKE_UNITS_PER_SCENE_UNIT: f64 = 1080.0 / 8.0;

/// A canvas fill or stroke style.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasPaint {
    Color(Color),
    /// `createLinearGradient(x0, y0, x1, y1)` in user space at creation.
    Linear {
        p0: Vec2,
        p1: Vec2,
        stops: Vec<(f32, Color)>,
    },
    /// `createRadialGradient(x0, y0, r0, x1, y1, r1)`.
    Radial {
        c0: Vec2,
        r0: f64,
        c1: Vec2,
        r1: f64,
        stops: Vec<(f32, Color)>,
    },
}

impl CanvasPaint {
    fn first_color(&self) -> Color {
        match self {
            CanvasPaint::Color(c) => *c,
            CanvasPaint::Linear { stops, .. } | CanvasPaint::Radial { stops, .. } => {
                stops.first().map(|s| s.1).unwrap_or(BLACK)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBaseline {
    Alphabetic,
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Debug)]
struct State {
    /// User space → canvas pixels.
    ctm: Affine,
    fill: CanvasPaint,
    stroke: CanvasPaint,
    line_width: f64,
    cap: LineCap,
    join: LineJoin,
    dash: Vec<f64>,
    dash_offset: f64,
    alpha: f32,
    blend: Blend,
    font_px: f64,
    bold: bool,
    italic: bool,
    family: String,
    align: TextAlign,
    baseline: TextBaseline,
    shadow_color: Color,
    shadow_blur: f64,
    /// `shadowOffsetX/Y` in canvas pixels (not affected by the transform).
    shadow_offset: (f64, f64),
    /// Extra clip from `clip()` (scene rect), on top of the canvas bounds.
    clip: Option<(Vec2, Vec2)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            ctm: Affine::IDENTITY,
            fill: CanvasPaint::Color(BLACK),
            stroke: CanvasPaint::Color(BLACK),
            line_width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: vec![],
            dash_offset: 0.0,
            alpha: 1.0,
            blend: Blend::Normal,
            font_px: 10.0,
            bold: false,
            italic: false,
            family: "Inter".into(),
            align: TextAlign::Left,
            baseline: TextBaseline::Alphabetic,
            shadow_color: Color::TRANSPARENT,
            shadow_blur: 0.0,
            shadow_offset: (0.0, 0.0),
            clip: None,
        }
    }
}

/// A 2D canvas context that paints into RunWee mobjects.
#[derive(Clone)]
pub struct Canvas2D {
    width: f64,
    height: f64,
    /// Canvas pixels → scene units (y flipped).
    map: Affine,
    /// Scene units per canvas pixel.
    unit: f64,
    /// The canvas area in the scene; painting is clipped to it.
    bounds: (Vec2, Vec2),
    state: State,
    stack: Vec<State>,
    /// Current path, in canvas pixels (the CTM is applied as commands arrive).
    path: Path,
    /// Current point in user space (for arcTo / quadratic / relative logic).
    cur_user: Option<Vec2>,
    out: Vec<Mobject>,
}

impl Canvas2D {
    /// A `width`×`height` pixel canvas shown inside `target` (scene units). The
    /// canvas keeps its aspect ratio and is centred, like a CSS `contain` fit.
    pub fn new(width: f64, height: f64, target: Rect) -> Self {
        let s = (target.width() / width).min(target.height() / height);
        let c = target.center();
        let map = Affine::translate(c)
            .then_after(Affine::scale(s, -s))
            .then_after(Affine::translate(v2(-width / 2.0, -height / 2.0)));
        Self {
            width,
            height,
            map,
            unit: s,
            bounds: (map.apply(v2(0.0, height)), map.apply(v2(width, 0.0))),
            state: State::default(),
            stack: vec![],
            path: Path::new(),
            cur_user: None,
            out: vec![],
        }
    }

    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Canvas pixel → scene point.
    pub fn to_scene(&self, p: Vec2) -> Vec2 {
        self.map.apply(p)
    }

    /// Everything painted so far, as one group (painter's order).
    pub fn finish(self) -> Mobject {
        Mobject::group(self.out)
    }

    /// Everything painted so far, leaving the canvas empty.
    pub fn take(&mut self) -> Mobject {
        Mobject::group(std::mem::take(&mut self.out))
    }

    // ───────────────────────────── state ─────────────────────────────

    pub fn save(&mut self) {
        self.stack.push(self.state.clone());
    }
    pub fn restore(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s;
        }
    }
    pub fn reset(&mut self) {
        self.state = State::default();
        self.stack.clear();
        self.path = Path::new();
        self.cur_user = None;
        self.out.clear();
    }

    pub fn translate(&mut self, x: f64, y: f64) {
        self.state.ctm = self.state.ctm.then_after(Affine::translate(v2(x, y)));
    }
    pub fn rotate(&mut self, angle: f64) {
        // Canvas angles are clockwise on screen (y down) — the same matrix.
        let (s, c) = angle.sin_cos();
        let r = Affine {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        };
        self.state.ctm = self.state.ctm.then_after(r);
    }
    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.state.ctm = self.state.ctm.then_after(Affine::scale(sx, sy));
    }
    /// `transform(a, b, c, d, e, f)`: multiply the CTM.
    pub fn transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.state.ctm = self.state.ctm.then_after(Affine { a, b, c, d, e, f });
    }
    /// `setTransform(a, b, c, d, e, f)`: replace the CTM.
    pub fn set_transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.state.ctm = Affine { a, b, c, d, e, f };
    }
    pub fn reset_transform(&mut self) {
        self.state.ctm = Affine::IDENTITY;
    }

    pub fn set_fill_style(&mut self, p: CanvasPaint) {
        self.state.fill = p;
    }
    pub fn set_stroke_style(&mut self, p: CanvasPaint) {
        self.state.stroke = p;
    }
    /// Any CSS colour string; invalid strings are ignored (as in browsers).
    pub fn set_fill_style_css(&mut self, css: &str) {
        if let Some(c) = css_color(css) {
            self.state.fill = CanvasPaint::Color(c);
        }
    }
    pub fn set_stroke_style_css(&mut self, css: &str) {
        if let Some(c) = css_color(css) {
            self.state.stroke = CanvasPaint::Color(c);
        }
    }
    pub fn set_line_width(&mut self, w: f64) {
        if w.is_finite() && w > 0.0 {
            self.state.line_width = w;
        }
    }
    pub fn set_line_cap(&mut self, cap: &str) {
        self.state.cap = match cap {
            "round" => LineCap::Round,
            "square" => LineCap::Square,
            _ => LineCap::Butt,
        };
    }
    pub fn set_line_join(&mut self, join: &str) {
        self.state.join = match join {
            "round" => LineJoin::Round,
            "bevel" => LineJoin::Bevel,
            _ => LineJoin::Miter,
        };
    }
    pub fn set_line_dash(&mut self, segments: &[f64]) {
        let mut v: Vec<f64> = segments
            .iter()
            .copied()
            .filter(|x| x.is_finite() && *x >= 0.0)
            .collect();
        if v.len() % 2 == 1 {
            let copy = v.clone();
            v.extend(copy);
        }
        self.state.dash = v;
    }
    pub fn set_line_dash_offset(&mut self, o: f64) {
        self.state.dash_offset = o;
    }
    pub fn set_global_alpha(&mut self, a: f64) {
        if a.is_finite() {
            self.state.alpha = a.clamp(0.0, 1.0) as f32;
        }
    }
    pub fn set_global_composite_operation(&mut self, op: &str) {
        self.state.blend = match op {
            "lighter" => Blend::Add,
            "screen" => Blend::Screen,
            "multiply" => Blend::Multiply,
            "overlay" => Blend::Overlay,
            _ => Blend::Normal,
        };
    }
    pub fn set_shadow_color_css(&mut self, css: &str) {
        if let Some(c) = css_color(css) {
            self.state.shadow_color = c;
        }
    }
    pub fn set_shadow_blur(&mut self, b: f64) {
        self.state.shadow_blur = b.max(0.0);
    }
    pub fn set_shadow_offset_x(&mut self, x: f64) {
        self.state.shadow_offset.0 = if x.is_finite() { x } else { 0.0 };
    }
    pub fn set_shadow_offset_y(&mut self, y: f64) {
        self.state.shadow_offset.1 = if y.is_finite() { y } else { 0.0 };
    }
    /// CSS font shorthand, e.g. `"bold 13px Inter"` or `"600 15px Manrope, sans-serif"`.
    pub fn set_font(&mut self, font: &str) {
        let f = parse_font(font);
        self.state.font_px = f.0;
        self.state.bold = f.1;
        self.state.italic = f.2;
        self.state.family = f.3;
    }
    pub fn set_text_align(&mut self, a: &str) {
        self.state.align = match a {
            "center" => TextAlign::Center,
            "right" | "end" => TextAlign::Right,
            _ => TextAlign::Left,
        };
    }
    pub fn set_text_baseline(&mut self, b: &str) {
        self.state.baseline = match b {
            "top" | "hanging" => TextBaseline::Top,
            "middle" => TextBaseline::Middle,
            "bottom" | "ideographic" => TextBaseline::Bottom,
            _ => TextBaseline::Alphabetic,
        };
    }

    // ───────────────────────────── paths ─────────────────────────────

    fn dev(&self, p: Vec2) -> Vec2 {
        self.state.ctm.apply(p)
    }

    pub fn begin_path(&mut self) {
        self.path = Path::new();
        self.cur_user = None;
    }
    pub fn move_to(&mut self, x: f64, y: f64) {
        if !(x.is_finite() && y.is_finite()) {
            return;
        }
        let p = self.dev(v2(x, y));
        self.path.move_to(p);
        self.cur_user = Some(v2(x, y));
    }
    pub fn line_to(&mut self, x: f64, y: f64) {
        if !(x.is_finite() && y.is_finite()) {
            return;
        }
        if self.cur_user.is_none() {
            return self.move_to(x, y);
        }
        let p = self.dev(v2(x, y));
        self.path.line_to(p);
        self.cur_user = Some(v2(x, y));
    }
    pub fn quadratic_curve_to(&mut self, cx: f64, cy: f64, x: f64, y: f64) {
        let Some(p0) = self.cur_user else {
            self.move_to(cx, cy);
            return self.line_to(x, y);
        };
        let c = v2(cx, cy);
        let p = v2(x, y);
        let c1 = p0 + (c - p0) * (2.0 / 3.0);
        let c2 = p + (c - p) * (2.0 / 3.0);
        self.cubic_user(c1, c2, p);
    }
    pub fn bezier_curve_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) {
        if self.cur_user.is_none() {
            self.move_to(c1x, c1y);
        }
        self.cubic_user(v2(c1x, c1y), v2(c2x, c2y), v2(x, y));
    }
    fn cubic_user(&mut self, c1: Vec2, c2: Vec2, p: Vec2) {
        if ![c1.x, c1.y, c2.x, c2.y, p.x, p.y]
            .iter()
            .all(|v| v.is_finite())
        {
            return;
        }
        let (d1, d2, dp) = (self.dev(c1), self.dev(c2), self.dev(p));
        self.path.cubic_to(d1, d2, dp);
        self.cur_user = Some(p);
    }
    pub fn close_path(&mut self) {
        if self.cur_user.is_some() {
            self.path.close();
            // After closePath the current point is the subpath start.
            if let Some(sp) = self.path.subpaths.last() {
                let start = sp.start;
                if let Some(inv) = invert(&self.state.ctm) {
                    self.cur_user = Some(inv.apply(start));
                }
            }
        }
    }
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.move_to(x, y);
        self.line_to(x + w, y);
        self.line_to(x + w, y + h);
        self.line_to(x, y + h);
        self.close_path();
    }
    /// `roundRect(x, y, w, h, radii)`; `radii` has 1–4 entries (CSS order
    /// top-left, top-right, bottom-right, bottom-left).
    pub fn round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, radii: &[f64]) {
        let r = |i: usize| -> f64 {
            let v = match radii.len() {
                0 => 0.0,
                1 => radii[0],
                2 => [radii[0], radii[1], radii[0], radii[1]][i],
                3 => [radii[0], radii[1], radii[2], radii[1]][i],
                _ => radii[i],
            };
            v.max(0.0).min(w.abs() / 2.0).min(h.abs() / 2.0)
        };
        let (tl, tr, br, bl) = (r(0), r(1), r(2), r(3));
        self.move_to(x + tl, y);
        self.line_to(x + w - tr, y);
        self.arc(x + w - tr, y + tr, tr, -PI / 2.0, 0.0, false);
        self.line_to(x + w, y + h - br);
        self.arc(x + w - br, y + h - br, br, 0.0, PI / 2.0, false);
        self.line_to(x + bl, y + h);
        self.arc(x + bl, y + h - bl, bl, PI / 2.0, PI, false);
        self.line_to(x, y + tl);
        self.arc(x + tl, y + tl, tl, PI, 1.5 * PI, false);
        self.close_path();
    }
    /// `arc(x, y, r, start, end, counterclockwise)` with canvas angle conventions.
    pub fn arc(&mut self, x: f64, y: f64, r: f64, a0: f64, a1: f64, ccw: bool) {
        self.ellipse(x, y, r, r, 0.0, a0, a1, ccw);
    }
    pub fn ellipse(
        &mut self,
        x: f64,
        y: f64,
        rx: f64,
        ry: f64,
        rotation: f64,
        a0: f64,
        a1: f64,
        ccw: bool,
    ) {
        if ![x, y, rx, ry, rotation, a0, a1]
            .iter()
            .all(|v| v.is_finite())
            || rx < 0.0
            || ry < 0.0
        {
            return;
        }
        let sweep = canvas_sweep(a0, a1, ccw);
        let (sr, cr) = rotation.sin_cos();
        let pt = |a: f64| -> Vec2 {
            let (s, c) = a.sin_cos();
            let (px, py) = (rx * c, ry * s);
            v2(x + px * cr - py * sr, y + px * sr + py * cr)
        };
        let start = pt(a0);
        if self.cur_user.is_some() {
            self.line_to(start.x, start.y);
        } else {
            self.move_to(start.x, start.y);
        }
        if sweep.abs() < 1e-12 || (rx == 0.0 && ry == 0.0) {
            return;
        }
        // Cubic approximation, ≤ 90° per segment.
        let n = (sweep.abs() / (PI / 2.0)).ceil().max(1.0) as usize;
        let step = sweep / n as f64;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let deriv = |a: f64| -> Vec2 {
            let (s, c) = a.sin_cos();
            let (dx, dy) = (-rx * s, ry * c);
            v2(dx * cr - dy * sr, dx * sr + dy * cr)
        };
        let mut a = a0;
        for _ in 0..n {
            let b = a + step;
            let (p0, p3) = (pt(a), pt(b));
            let c1 = p0 + deriv(a) * k;
            let c2 = p3 - deriv(b) * k;
            self.cubic_user(c1, c2, p3);
            a = b;
        }
    }
    /// `arcTo(x1, y1, x2, y2, r)`: a corner arc tangent to both lines.
    pub fn arc_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, r: f64) {
        let Some(p0) = self.cur_user else {
            return self.move_to(x1, y1);
        };
        let p1 = v2(x1, y1);
        let p2 = v2(x2, y2);
        let d0 = p0 - p1;
        let d2 = p2 - p1;
        let (l0, l2) = (d0.length(), d2.length());
        if r <= 0.0 || l0 < 1e-9 || l2 < 1e-9 {
            return self.line_to(x1, y1);
        }
        let (u0, u2) = (d0 * (1.0 / l0), d2 * (1.0 / l2));
        let cos = u0.dot(u2).clamp(-1.0, 1.0);
        let ang = cos.acos();
        if ang.abs() < 1e-9 || (PI - ang).abs() < 1e-9 {
            return self.line_to(x1, y1);
        }
        let t = r / (ang / 2.0).tan();
        let a = p1 + u0 * t;
        let b = p1 + u2 * t;
        let bis = (u0 + u2).normalize();
        let c = p1 + bis * (r / (ang / 2.0).sin());
        let a_start = (a - c).y.atan2((a - c).x);
        let a_end = (b - c).y.atan2((b - c).x);
        // Choose the short way round.
        let cross = u0.x * u2.y - u0.y * u2.x;
        self.line_to(a.x, a.y);
        self.arc(c.x, c.y, r, a_start, a_end, cross < 0.0);
    }

    // ───────────────────────────── painting ─────────────────────────────

    fn device_scale(&self) -> f64 {
        let m = &self.state.ctm;
        (m.a * m.d - m.b * m.c).abs().sqrt()
    }

    fn scene_paint(&self, p: &CanvasPaint, alpha: f32) -> (Paint, f32) {
        let to_scene = |q: Vec2| self.map.apply(self.state.ctm.apply(q));
        match p {
            CanvasPaint::Color(c) => (Paint::Solid(Color { a: 1.0, ..*c }), c.a * alpha),
            CanvasPaint::Linear { p0, p1, stops } => (
                Paint::Linear {
                    start: to_scene(*p0),
                    end: to_scene(*p1),
                    stops: stops.clone(),
                },
                alpha,
            ),
            CanvasPaint::Radial {
                c0, c1, r1, stops, ..
            } => (
                Paint::Radial {
                    center: to_scene(*c1),
                    radius: r1 * self.device_scale() * self.unit,
                    focal: to_scene(*c0),
                    stops: stops.clone(),
                },
                alpha,
            ),
        }
    }

    fn base_style(&self) -> Style {
        let mut st = Style {
            blend: self.state.blend,
            ..Default::default()
        };
        // Canvas shadows are drawn under the shape with normal blending; the
        // blur is a Gaussian with sigma = shadowBlur / 2 and, like the offset,
        // is in canvas pixels regardless of the current transform.
        let (ox, oy) = self.state.shadow_offset;
        if self.state.shadow_color.a > 0.0
            && (self.state.shadow_blur > 0.0 || ox != 0.0 || oy != 0.0)
        {
            st.shadow = Some(Shadow {
                offset: v2(ox * self.unit, -oy * self.unit),
                blur: self.state.shadow_blur * 0.5 * self.unit,
                color: self.state.shadow_color,
            });
        }
        st
    }

    /// Clip later painting to the current path (its bounding box — exact for
    /// rectangles, which is how canvas lessons use it).
    pub fn clip(&mut self) {
        let mut p = self.path.clone();
        p.transform(&self.map);
        if let Some(b) = p.bounds() {
            let r = (b.min, b.max);
            self.state.clip = Some(match self.state.clip {
                Some(c) => (
                    v2(c.0.x.max(r.0.x), c.0.y.max(r.0.y)),
                    v2(c.1.x.min(r.1.x), c.1.y.min(r.1.y)),
                ),
                None => r,
            });
        }
    }

    fn emit(&mut self, path: Path, mut style: Style) {
        if path.is_empty() {
            return;
        }
        let (lo, hi) = self.bounds;
        style.clip = Some(match self.state.clip {
            Some(c) => (
                v2(lo.x.max(c.0.x), lo.y.max(c.0.y)),
                v2(hi.x.min(c.1.x), hi.y.min(c.1.y)),
            ),
            None => (lo, hi),
        });
        let mut p = path;
        p.transform(&self.map);
        self.out.push(Mobject::from_path_styled(p, style));
    }

    /// Fill the current path (`"nonzero"` or `"evenodd"`).
    pub fn fill_rule(&mut self, rule: &str) {
        let (paint, op) = self.scene_paint(&self.state.fill.clone(), self.state.alpha);
        let mut st = self.base_style();
        st.fill = paint;
        st.fill_opacity = op;
        st.stroke_width = 0.0;
        st.stroke_opacity = 0.0;
        st.fill_rule = if rule == "evenodd" {
            FillRule::EvenOdd
        } else {
            FillRule::NonZero
        };
        let p = self.path.clone();
        self.emit(p, st);
    }
    pub fn fill(&mut self) {
        self.fill_rule("nonzero");
    }
    pub fn stroke(&mut self) {
        let p = self.path.clone();
        self.stroke_path(p);
    }
    fn stroke_path(&mut self, path: Path) {
        let (paint, op) = self.scene_paint(&self.state.stroke.clone(), self.state.alpha);
        let k = self.device_scale() * self.unit;
        let mut st = self.base_style();
        st.fill_opacity = 0.0;
        st.stroke = paint;
        st.stroke_opacity = op;
        st.stroke_width = (self.state.line_width * k * STROKE_UNITS_PER_SCENE_UNIT) as f32;
        st.line_cap = self.state.cap;
        st.line_join = self.state.join;
        if self.state.dash.len() >= 2 {
            let on = self.state.dash[0] * k;
            let off = self.state.dash[1] * k;
            if on > 0.0 || off > 0.0 {
                st.dash = Some((on.max(1e-4), off.max(1e-4)));
                st.dash_offset = self.state.dash_offset * k;
            }
        }
        self.emit(path, st);
    }
    pub fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let saved = std::mem::take(&mut self.path);
        let cur = self.cur_user.take();
        self.rect(x, y, w, h);
        self.fill();
        self.path = saved;
        self.cur_user = cur;
    }
    pub fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let saved = std::mem::take(&mut self.path);
        let cur = self.cur_user.take();
        self.rect(x, y, w, h);
        self.stroke();
        self.path = saved;
        self.cur_user = cur;
    }
    /// Vector output can't erase; clearing the whole canvas drops what was
    /// painted, smaller clears are ignored.
    pub fn clear_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let a = self.dev(v2(x, y));
        let b = self.dev(v2(x + w, y + h));
        if a.x.min(b.x) <= 0.5
            && a.y.min(b.y) <= 0.5
            && a.x.max(b.x) >= self.width - 0.5
            && a.y.max(b.y) >= self.height - 0.5
        {
            self.out.clear();
        }
    }

    // ───────────────────────────── text ─────────────────────────────

    fn font_id(&self) -> FontId {
        find_font(&self.state.family, self.state.bold, self.state.italic)
    }

    /// `measureText(text).width` in user units.
    pub fn measure_text(&self, text: &str) -> f64 {
        layout_line(self.font_id(), text).1 * self.state.font_px
    }

    fn text_path(&self, text: &str, x: f64, y: f64) -> Path {
        let id = self.font_id();
        let size = self.state.font_px;
        let (glyphs, adv) = layout_line(id, text);
        let width = adv * size;
        let (asc, desc) = line_metrics(id);
        let x0 = match self.state.align {
            TextAlign::Left => x,
            TextAlign::Center => x - width / 2.0,
            TextAlign::Right => x - width,
        };
        let base = match self.state.baseline {
            TextBaseline::Alphabetic => y,
            TextBaseline::Top => y + asc * size,
            TextBaseline::Middle => y + (asc - desc) / 2.0 * size,
            TextBaseline::Bottom => y - desc * size,
        };
        let mut out = Path::new();
        for g in glyphs {
            let gp = glyph_path(g.font, g.gid);
            if g.gid == 0 || gp.is_empty() {
                continue;
            }
            // Glyph outlines are in em units with y up.
            let to_user = Affine {
                a: size,
                b: 0.0,
                c: 0.0,
                d: -size,
                e: x0 + g.x * size,
                f: base - g.y * size,
            };
            let m = self.state.ctm.then_after(to_user);
            let mut p = (*gp).clone();
            p.transform(&m);
            out.subpaths.extend(p.subpaths);
        }
        out
    }

    pub fn fill_text(&mut self, text: &str, x: f64, y: f64) {
        if text.is_empty() || !(x.is_finite() && y.is_finite()) {
            return;
        }
        let p = self.text_path(text, x, y);
        let (paint, op) = self.scene_paint(&self.state.fill.clone(), self.state.alpha);
        let mut st = self.base_style();
        st.fill = paint;
        st.fill_opacity = op;
        st.stroke_width = 0.0;
        st.stroke_opacity = 0.0;
        self.emit(p, st);
    }
    pub fn stroke_text(&mut self, text: &str, x: f64, y: f64) {
        if text.is_empty() || !(x.is_finite() && y.is_finite()) {
            return;
        }
        let p = self.text_path(text, x, y);
        self.stroke_path(p);
    }

    /// The current fill colour (first stop for gradients).
    pub fn fill_color(&self) -> Color {
        self.state.fill.first_color()
    }
}

/// Width of `text` in pixels for a CSS font (same fonts and fallbacks as painting).
pub fn text_width(font: &str, text: &str) -> f64 {
    let (px, bold, italic, family) = parse_font(font);
    layout_line(find_font(&family, bold, italic), text).1 * px
}

/// Signed sweep for canvas `arc`/`ellipse` (angles clockwise on screen).
fn canvas_sweep(a0: f64, a1: f64, ccw: bool) -> f64 {
    let d = a1 - a0;
    if !ccw {
        if d >= TAU {
            TAU
        } else {
            let s = d.rem_euclid(TAU);
            if s == 0.0 && d != 0.0 {
                TAU
            } else {
                s
            }
        }
    } else if -d >= TAU {
        -TAU
    } else {
        let s = (-d).rem_euclid(TAU);
        if s == 0.0 && d != 0.0 {
            -TAU
        } else {
            -s
        }
    }
}

fn invert(m: &Affine) -> Option<Affine> {
    let det = m.a * m.d - m.b * m.c;
    if det.abs() < 1e-12 {
        return None;
    }
    let (a, b, c, d) = (m.d / det, -m.b / det, -m.c / det, m.a / det);
    Some(Affine {
        a,
        b,
        c,
        d,
        e: -(a * m.e + c * m.f),
        f: -(b * m.e + d * m.f),
    })
}

/// Parse a CSS font shorthand: (size px, bold, italic, family).
pub fn parse_font(font: &str) -> (f64, bool, bool, String) {
    let mut size = 10.0;
    let mut bold = false;
    let mut italic = false;
    let mut family = "Inter".to_string();
    let tokens: Vec<&str> = font.split_whitespace().collect();
    for (i, t) in tokens.iter().enumerate() {
        let tl = t.to_ascii_lowercase();
        if tl == "bold" || tl == "bolder" {
            bold = true;
        } else if tl == "italic" || tl == "oblique" {
            italic = true;
        } else if let Ok(w) = tl.parse::<u32>() {
            bold = w >= 600;
        } else if let Some(px) = tl.split('/').next().and_then(|s| {
            s.strip_suffix("px")
                .and_then(|n| n.parse::<f64>().ok())
                .or_else(|| {
                    s.strip_suffix("pt")
                        .and_then(|n| n.parse::<f64>().ok())
                        .map(|v| v * 4.0 / 3.0)
                })
                .or_else(|| {
                    s.strip_suffix("rem")
                        .and_then(|n| n.parse::<f64>().ok())
                        .map(|v| v * 16.0)
                })
        }) {
            size = px;
            let rest = tokens[i + 1..].join(" ");
            let first = rest
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches(|c| c == '"' || c == '\'');
            family = map_family(first);
            break;
        }
    }
    (size, bold, italic, family)
}

fn map_family(f: &str) -> String {
    match f.to_ascii_lowercase().as_str() {
        "monospace" | "courier" | "courier new" | "consolas" | "jetbrains mono" | "menlo"
        | "mono" => "mono".into(),
        "serif" | "times" | "times new roman" | "georgia" | "cambria" => "serif".into(),
        _ => "Inter".into(),
    }
}

/// CSS colour: hex, `rgb()/rgba()` (numbers or %), `hsl()/hsla()`, named colours.
pub fn css_color(s: &str) -> Option<Color> {
    let t = s.trim().to_ascii_lowercase();
    if t.is_empty() {
        return None;
    }
    if t == "transparent" {
        return Some(Color::TRANSPARENT);
    }
    if let Some(h) = t.strip_prefix('#') {
        if h.len() == 4 {
            let d = |i: usize| u8::from_str_radix(&h[i..i + 1], 16).ok().map(|v| v * 17);
            return Some(rgb8(d(0)?, d(1)?, d(2)?).with_alpha(d(3)? as f32 / 255.0));
        }
        return Color::parse(&t);
    }
    let func = |name: &str| -> Option<Vec<String>> {
        let rest = t.strip_prefix(name)?;
        let inner = rest.trim().strip_prefix('(')?.strip_suffix(')')?;
        Some(
            inner
                .split([',', ' ', '/'])
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect(),
        )
    };
    let num = |x: &str, full: f64| -> Option<f64> {
        if let Some(p) = x.strip_suffix('%') {
            p.parse::<f64>().ok().map(|v| v / 100.0 * full)
        } else {
            x.parse::<f64>().ok()
        }
    };
    if let Some(v) = func("rgba").or_else(|| func("rgb")) {
        if v.len() >= 3 {
            let r = num(&v[0], 255.0)?;
            let g = num(&v[1], 255.0)?;
            let b = num(&v[2], 255.0)?;
            let a = v.get(3).and_then(|x| num(x, 1.0)).unwrap_or(1.0);
            return Some(Color::rgba(
                (r / 255.0).clamp(0.0, 1.0) as f32,
                (g / 255.0).clamp(0.0, 1.0) as f32,
                (b / 255.0).clamp(0.0, 1.0) as f32,
                a.clamp(0.0, 1.0) as f32,
            ));
        }
    }
    if let Some(v) = func("hsla").or_else(|| func("hsl")) {
        if v.len() >= 3 {
            let h = v[0].trim_end_matches("deg").parse::<f64>().ok()?;
            let s = num(&v[1], 1.0)?;
            let l = num(&v[2], 1.0)?;
            let a = v.get(3).and_then(|x| num(x, 1.0)).unwrap_or(1.0);
            let (r, g, b) = hsl_to_rgb(h, s, l);
            return Some(Color::rgba(
                r as f32,
                g as f32,
                b as f32,
                a.clamp(0.0, 1.0) as f32,
            ));
        }
    }
    let hex = match t.as_str() {
        "black" => 0x000000,
        "white" => 0xFFFFFF,
        "red" => 0xFF0000,
        "green" => 0x008000,
        "lime" => 0x00FF00,
        "blue" => 0x0000FF,
        "yellow" => 0xFFFF00,
        "orange" => 0xFFA500,
        "purple" => 0x800080,
        "magenta" | "fuchsia" => 0xFF00FF,
        "cyan" | "aqua" => 0x00FFFF,
        "gray" | "grey" => 0x808080,
        "darkgray" | "darkgrey" => 0xA9A9A9,
        "lightgray" | "lightgrey" => 0xD3D3D3,
        "silver" => 0xC0C0C0,
        "navy" => 0x000080,
        "teal" => 0x008080,
        "maroon" => 0x800000,
        "olive" => 0x808000,
        "pink" => 0xFFC0CB,
        "brown" => 0xA52A2A,
        "gold" => 0xFFD700,
        "crimson" => 0xDC143C,
        "indigo" => 0x4B0082,
        "violet" => 0xEE82EE,
        "coral" => 0xFF7F50,
        "tomato" => 0xFF6347,
        "salmon" => 0xFA8072,
        "skyblue" => 0x87CEEB,
        "steelblue" => 0x4682B4,
        "royalblue" => 0x4169E1,
        "dodgerblue" => 0x1E90FF,
        "slategray" | "slategrey" => 0x708090,
        "darkslategray" => 0x2F4F4F,
        "whitesmoke" => 0xF5F5F5,
        "darkred" => 0x8B0000,
        "darkgreen" => 0x006400,
        "darkblue" => 0x00008B,
        "darkorange" => 0xFF8C00,
        "limegreen" => 0x32CD32,
        "forestgreen" => 0x228B22,
        "seagreen" => 0x2E8B57,
        "turquoise" => 0x40E0D0,
        "khaki" => 0xF0E68C,
        "beige" => 0xF5F5DC,
        "ivory" => 0xFFFFF0,
        _ => return Color::parse(&t),
    };
    Some(rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8))
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let h = h.rem_euclid(360.0) / 360.0;
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    if s == 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let f = |mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> Canvas2D {
        // 800×400 canvas mapped 1 px → 0.01 scene units, centred on the origin.
        Canvas2D::new(800.0, 400.0, Rect::from_center(ORIGIN, 8.0, 4.0))
    }

    #[test]
    fn mapping_flips_y_and_centres() {
        let c = canvas();
        assert!((c.to_scene(v2(0.0, 0.0)) - v2(-4.0, 2.0)).length() < 1e-9);
        assert!((c.to_scene(v2(800.0, 400.0)) - v2(4.0, -2.0)).length() < 1e-9);
    }

    #[test]
    fn fill_rect_respects_transform() {
        let mut c = canvas();
        c.translate(400.0, 200.0);
        c.scale(2.0, 2.0);
        c.set_fill_style_css("#ff0000");
        c.fill_rect(0.0, 0.0, 10.0, 10.0);
        let m = c.finish();
        let b = m.children[0].path.bounds().unwrap();
        assert!((b.min - v2(0.0, -0.2)).length() < 1e-9, "{:?}", b);
        assert!((b.max - v2(0.2, 0.0)).length() < 1e-9, "{:?}", b);
        assert_eq!(m.children[0].style.fill, Paint::Solid(rgb8(255, 0, 0)));
    }

    #[test]
    fn shadows_are_drop_shadows_in_canvas_pixels() {
        let mut c = canvas();
        c.scale(3.0, 3.0); // shadows ignore the transform
        c.set_shadow_color_css("rgba(0, 0, 0, 0.5)");
        c.set_shadow_blur(10.0);
        c.set_shadow_offset_x(4.0);
        c.set_shadow_offset_y(6.0);
        c.fill_rect(0.0, 0.0, 10.0, 10.0);
        c.set_shadow_color_css("transparent");
        c.fill_rect(20.0, 0.0, 10.0, 10.0);
        let m = c.finish();
        let st = &m.children[0].style;
        let sh = st.shadow.expect("shadow");
        assert!(
            (sh.offset - v2(0.04, -0.06)).length() < 1e-9,
            "{:?}",
            sh.offset
        );
        assert!((sh.blur - 0.05).abs() < 1e-9);
        assert_eq!(st.glow, 0.0);
        assert!(m.children[1].style.shadow.is_none());
    }

    #[test]
    fn arc_directions_match_canvas() {
        // Clockwise on screen from 0 to π/2 goes through the bottom-right quadrant.
        let mut c = canvas();
        c.begin_path();
        c.arc(400.0, 200.0, 100.0, 0.0, PI / 2.0, false);
        c.stroke();
        let b = c.finish().children[0].path.bounds().unwrap();
        assert!(b.max.y < 1e-6 && b.min.y < -0.9, "{:?}", b); // below the centre (y up in scene)
        assert!(b.min.x > -1e-6, "{:?}", b);
        // Full circle.
        assert!((canvas_sweep(0.0, TAU, false) - TAU).abs() < 1e-12);
        assert!((canvas_sweep(0.0, -PI / 2.0, false) - 1.5 * PI).abs() < 1e-12);
        assert!((canvas_sweep(0.0, PI / 2.0, true) + 1.5 * PI).abs() < 1e-12);
    }

    #[test]
    fn save_restore_and_styles() {
        let mut c = canvas();
        c.set_line_width(4.0);
        c.save();
        c.set_line_width(10.0);
        c.restore();
        c.begin_path();
        c.move_to(0.0, 0.0);
        c.line_to(100.0, 0.0);
        c.set_stroke_style_css("rgba(0, 0, 255, 0.5)");
        c.set_line_dash(&[5.0, 5.0]);
        c.stroke();
        let m = c.finish();
        let st = &m.children[0].style;
        // 4 px * 0.01 units/px * 135 = 5.4
        assert!((st.stroke_width - 5.4).abs() < 1e-4);
        assert!((st.stroke_opacity - 0.5).abs() < 1e-6);
        assert!(st.dash.is_some());
    }

    #[test]
    fn css_colors() {
        assert_eq!(css_color("red"), Some(rgb8(255, 0, 0)));
        assert_eq!(css_color("#0f0"), Some(rgb8(0, 255, 0)));
        let c = css_color("rgba(16,185,129,0.08)").unwrap();
        assert!((c.a - 0.08).abs() < 1e-6 && (c.g - 185.0 / 255.0).abs() < 1e-6);
        let h = css_color("hsl(120, 100%, 50%)").unwrap();
        assert!((h.g - 1.0).abs() < 1e-6 && h.r.abs() < 1e-6);
        assert!(css_color("not-a-colour").is_none());
    }

    #[test]
    fn fonts_and_text() {
        let f = parse_font("bold 13px Inter");
        assert_eq!((f.0, f.1), (13.0, true));
        let f = parse_font("600 15px Manrope, sans-serif");
        assert_eq!((f.0, f.1, f.3.as_str()), (15.0, true, "Inter"));
        let f = parse_font("10px monospace");
        assert_eq!(f.3, "mono");
        let mut c = canvas();
        c.set_font("20px Inter");
        let w = c.measure_text("Hello");
        assert!(w > 30.0 && w < 70.0, "{w}");
        c.set_text_align("center");
        c.fill_text("Hello", 400.0, 200.0);
        let b = c.finish().children[0].path.bounds().unwrap();
        // Centred on x = 400 px → scene x ≈ 0.
        assert!((b.center().x).abs() < 0.05, "{:?}", b);
        // Alphabetic baseline at y = 200 px → glyphs sit above scene y = 0.
        assert!(b.min.y > -0.02 && b.max.y > 0.08, "{:?}", b);
    }

    #[test]
    fn round_rect_and_arc_to() {
        let mut c = canvas();
        c.begin_path();
        c.round_rect(100.0, 100.0, 200.0, 100.0, &[20.0]);
        c.fill();
        c.begin_path();
        c.move_to(0.0, 0.0);
        c.arc_to(100.0, 0.0, 100.0, 100.0, 20.0);
        c.line_to(100.0, 100.0);
        c.stroke();
        let m = c.finish();
        assert_eq!(m.children.len(), 2);
        let b = m.children[0].path.bounds().unwrap();
        assert!((b.width() - 2.0).abs() < 1e-6 && (b.height() - 1.0).abs() < 1e-6);
    }
}
