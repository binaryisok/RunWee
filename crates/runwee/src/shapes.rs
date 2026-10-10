//! Geometric primitives (manim's geometry module and more).

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::style::*;

fn stroked(path: Path) -> Mobject {
    Mobject::from_path(path)
}

fn filled(path: Path, c: Color) -> Mobject {
    Mobject::from_path_styled(path, Style::filled(c))
}

pub fn circle(radius: f64) -> Mobject {
    stroked(Path::ellipse(Vec2::ZERO, radius, radius))
}

pub fn circle_at(center: Vec2, radius: f64) -> Mobject {
    stroked(Path::ellipse(center, radius, radius))
}

pub fn ellipse(width: f64, height: f64) -> Mobject {
    stroked(Path::ellipse(Vec2::ZERO, width / 2.0, height / 2.0))
}

pub const DEFAULT_DOT_RADIUS: f64 = 0.08;

/// Small filled disc.
pub fn dot(p: Vec2) -> Mobject {
    filled(
        Path::ellipse(p, DEFAULT_DOT_RADIUS, DEFAULT_DOT_RADIUS),
        WHITE,
    )
}

pub fn dot_r(p: Vec2, r: f64, c: Color) -> Mobject {
    filled(Path::ellipse(p, r, r), c)
}

/// Open circular arc centred at the origin.
pub fn arc(radius: f64, start: f64, angle: f64) -> Mobject {
    stroked(Path::arc(Vec2::ZERO, radius, radius, start, angle))
}

pub fn arc_centered(center: Vec2, radius: f64, start: f64, angle: f64) -> Mobject {
    stroked(Path::arc(center, radius, radius, start, angle))
}

/// Arc from `a` to `b` bending by `angle` (positive = counter-clockwise).
pub fn arc_between(a: Vec2, b: Vec2, angle: f64) -> Mobject {
    stroked(arc_between_path(a, b, angle))
}

pub fn arc_between_path(a: Vec2, b: Vec2, angle: f64) -> Path {
    if angle.abs() < 1e-6 {
        return Path::polyline(&[a, b], false);
    }
    let chord = b - a;
    let half = chord.length() / 2.0;
    let r = half / (angle / 2.0).sin();
    let mid = (a + b) / 2.0;
    let h = r * (angle / 2.0).cos();
    let center = mid + chord.normalize().perp() * h;
    let start = (a - center).angle();
    Path::arc(center, r.abs(), r.abs(), start, angle)
}

pub fn sector(radius: f64, start: f64, angle: f64) -> Mobject {
    let mut p = Path::new();
    p.move_to(Vec2::ZERO);
    p.line_to(Vec2::polar(radius, start));
    p.arc_to_center(Vec2::ZERO, angle);
    p.close();
    stroked(p).fill(WHITE, 1.0)
}

pub fn annular_sector(inner: f64, outer: f64, start: f64, angle: f64) -> Mobject {
    let mut p = Path::new();
    p.move_to(Vec2::polar(outer, start));
    p.arc_to_center(Vec2::ZERO, angle);
    p.line_to(Vec2::polar(inner, start + angle));
    p.arc_to_center(Vec2::ZERO, -angle);
    p.close();
    stroked(p).fill(WHITE, 1.0)
}

pub fn annulus(inner: f64, outer: f64) -> Mobject {
    let mut p = Path::ellipse(Vec2::ZERO, outer, outer);
    let inner_p = Path::ellipse(Vec2::ZERO, inner, inner).reversed();
    p.subpaths.extend(inner_p.subpaths);
    let mut m = stroked(p).fill(WHITE, 1.0);
    m.style.fill_rule = FillRule::EvenOdd;
    m
}

pub fn rectangle(width: f64, height: f64) -> Mobject {
    let (w, h) = (width / 2.0, height / 2.0);
    stroked(Path::polyline(
        &[v2(-w, h), v2(w, h), v2(w, -h), v2(-w, -h)],
        true,
    ))
}

pub fn square(side: f64) -> Mobject {
    rectangle(side, side)
}

pub fn rounded_rectangle(width: f64, height: f64, radius: f64) -> Mobject {
    stroked(rounded_rect_path(Vec2::ZERO, width, height, radius))
}

pub fn rounded_rect_path(c: Vec2, width: f64, height: f64, radius: f64) -> Path {
    let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    let (w, h) = (width / 2.0, height / 2.0);
    let mut p = Path::new();
    p.move_to(c + v2(-w + r, h));
    p.line_to(c + v2(w - r, h));
    p.arc_to_center(c + v2(w - r, h - r), -PI / 2.0);
    p.line_to(c + v2(w, -h + r));
    p.arc_to_center(c + v2(w - r, -h + r), -PI / 2.0);
    p.line_to(c + v2(-w + r, -h));
    p.arc_to_center(c + v2(-w + r, -h + r), -PI / 2.0);
    p.line_to(c + v2(-w, h - r));
    p.arc_to_center(c + v2(-w + r, h - r), -PI / 2.0);
    p.close();
    p
}

pub fn polygon(points: &[Vec2]) -> Mobject {
    stroked(Path::polyline(points, true))
}

pub fn polyline(points: &[Vec2]) -> Mobject {
    stroked(Path::polyline(points, false))
}

/// Smooth curve through points.
pub fn smooth_curve(points: &[Vec2], closed: bool) -> Mobject {
    stroked(Path::smooth_through(points, closed))
}

/// Regular n-gon with circumradius `r`; a vertex points up.
pub fn regular_polygon(n: usize, r: f64) -> Mobject {
    let n = n.max(3);
    let pts: Vec<Vec2> = (0..n)
        .map(|i| Vec2::polar(r, PI / 2.0 + TAU * i as f64 / n as f64))
        .collect();
    polygon(&pts)
}

pub fn triangle(r: f64) -> Mobject {
    regular_polygon(3, r)
}

pub fn star(n: usize, outer: f64, inner: f64) -> Mobject {
    let n = n.max(2);
    let pts: Vec<Vec2> = (0..2 * n)
        .map(|i| {
            let r = if i % 2 == 0 { outer } else { inner };
            Vec2::polar(r, PI / 2.0 + PI * i as f64 / n as f64)
        })
        .collect();
    polygon(&pts)
}

pub fn line(a: Vec2, b: Vec2) -> Mobject {
    stroked(Path::polyline(&[a, b], false))
}

pub fn dashed_line(a: Vec2, b: Vec2) -> Mobject {
    line(a, b).dashed(0.12, 0.08)
}

pub fn cubic_bezier(p0: Vec2, c1: Vec2, c2: Vec2, p3: Vec2) -> Mobject {
    let mut p = Path::new();
    p.move_to(p0).cubic_to(c1, c2, p3);
    stroked(p)
}

/// Arrow-head shapes. The filled variants are manim's `ArrowTriangleFilledTip`,
/// `StealthTip`, `ArrowCircleFilledTip` and `ArrowSquareFilledTip`; the
/// `*Outline` variants are `ArrowTriangleTip`, `ArrowCircleTip` and
/// `ArrowSquareTip` (stroked, unfilled).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TipShape {
    #[default]
    Triangle,
    Stealth,
    Circle,
    Square,
    TriangleOutline,
    CircleOutline,
    SquareOutline,
    None,
}

impl TipShape {
    /// Stroked-only (unfilled) tip.
    pub fn is_outline(self) -> bool {
        matches!(
            self,
            TipShape::TriangleOutline | TipShape::CircleOutline | TipShape::SquareOutline
        )
    }
    /// How far back from the tip point the shaft has to stop.
    fn shaft_gap(self, length: f64, width: f64) -> f64 {
        match self {
            TipShape::TriangleOutline => length,
            TipShape::CircleOutline | TipShape::SquareOutline => width,
            _ => length * 0.85,
        }
    }
}

/// Arrow configuration.
#[derive(Clone, Copy, Debug)]
pub struct ArrowStyle {
    pub tip_length: f64,
    pub tip_width: f64,
    pub stroke_width: f32,
    pub tip: TipShape,
    pub start_tip: TipShape,
    /// Gap left at both ends (manim's `buff`).
    pub buff: f64,
    /// Cap the tip at this fraction of the arrow length.
    pub max_tip_ratio: f64,
    pub color: Color,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        Self {
            tip_length: 0.28,
            tip_width: 0.24,
            stroke_width: 5.0,
            tip: TipShape::Triangle,
            start_tip: TipShape::None,
            buff: 0.0,
            max_tip_ratio: 0.35,
            color: WHITE,
        }
    }
}

impl ArrowStyle {
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn double(mut self) -> Self {
        self.start_tip = self.tip;
        self
    }
    pub fn buff(mut self, b: f64) -> Self {
        self.buff = b;
        self
    }
    pub fn thickness(mut self, w: f32) -> Self {
        self.stroke_width = w;
        self.tip_length *= (w / 5.0).sqrt() as f64;
        self.tip_width *= (w / 5.0).sqrt() as f64;
        self
    }
    pub fn tip(mut self, t: TipShape) -> Self {
        self.tip = t;
        self
    }
    pub fn start_tip(mut self, t: TipShape) -> Self {
        self.start_tip = t;
        self
    }
}

/// Tip shape pointing in direction `dir` with its point at `tip`.
pub fn arrow_tip(
    tip: Vec2,
    dir: Vec2,
    length: f64,
    width: f64,
    shape: TipShape,
    color: Color,
) -> Mobject {
    let d = dir.normalize();
    let n = d.perp();
    let base = tip - d * length;
    let path = match shape {
        TipShape::Triangle | TipShape::TriangleOutline => Path::polyline(
            &[tip, base + n * (width / 2.0), base - n * (width / 2.0)],
            true,
        ),
        TipShape::Stealth => {
            let back = tip - d * (length * 0.72);
            let mut p = Path::new();
            p.move_to(tip);
            p.quad_to(
                base + n * (width * 0.2) + d * (length * 0.3),
                base + n * (width / 2.0),
            );
            p.line_to(back);
            p.line_to(base - n * (width / 2.0));
            p.quad_to(base - n * (width * 0.2) + d * (length * 0.3), tip);
            p.close();
            p
        }
        TipShape::Circle | TipShape::CircleOutline => {
            Path::ellipse(tip - d * (width / 2.0), width / 2.0, width / 2.0)
        }
        TipShape::Square | TipShape::SquareOutline => {
            let c = tip - d * (width / 2.0);
            let h = width / 2.0;
            Path::polyline(
                &[
                    c + d * h + n * h,
                    c - d * h + n * h,
                    c - d * h - n * h,
                    c + d * h - n * h,
                ],
                true,
            )
        }
        TipShape::None => Path::new(),
    };
    let mut m = if shape.is_outline() {
        let mut m = Mobject::from_path_styled(path, Style::stroked(color, 3.0));
        m.style.line_join = LineJoin::Miter;
        m
    } else {
        let mut m = Mobject::from_path_styled(path, Style::filled(color));
        m.style.stroke_width = 0.0;
        m
    };
    m.tag = Some("tip".into());
    m
}

/// Arrow along an arbitrary path (the shaft follows the path; tips at its ends).
pub fn arrow_along(path: Path, st: ArrowStyle) -> Mobject {
    let len = path.length().max(1e-9);
    let tip_len = st.tip_length.min(len * st.max_tip_ratio);
    let tip_w = st.tip_width * tip_len / st.tip_length.max(1e-9);
    let has_end = st.tip != TipShape::None;
    let has_start = st.start_tip != TipShape::None;
    let a = if has_start {
        st.start_tip.shaft_gap(tip_len, tip_w) / len
    } else {
        0.0
    };
    let b = if has_end {
        1.0 - st.tip.shaft_gap(tip_len, tip_w) / len
    } else {
        1.0
    };
    let mut shaft = Mobject::from_path(path.partial(a, b.max(a)));
    shaft.style = Style::stroked(st.color, st.stroke_width);
    shaft.style.line_cap = LineCap::Butt;
    let mut g = Mobject::group([shaft]);
    if has_end {
        let tip = path.point_from_proportion(1.0);
        let dir = path.tangent_from_proportion(1.0);
        g.push(arrow_tip(tip, dir, tip_len, tip_w, st.tip, st.color));
    }
    if has_start {
        let tip = path.point_from_proportion(0.0);
        let dir = -path.tangent_from_proportion(0.0);
        g.push(arrow_tip(tip, dir, tip_len, tip_w, st.start_tip, st.color));
    }
    g
}

pub fn arrow_styled(a: Vec2, b: Vec2, st: ArrowStyle) -> Mobject {
    let d = (b - a).normalize();
    let (a2, b2) = (a + d * st.buff, b - d * st.buff);
    arrow_along(Path::polyline(&[a2, b2], false), st)
}

pub fn arrow(a: Vec2, b: Vec2) -> Mobject {
    arrow_styled(a, b, ArrowStyle::default())
}

pub fn double_arrow(a: Vec2, b: Vec2) -> Mobject {
    arrow_styled(a, b, ArrowStyle::default().double())
}

/// Arrow from the origin (manim's `Vector`).
pub fn vector(v: Vec2) -> Mobject {
    arrow(Vec2::ZERO, v)
}

pub fn curved_arrow(a: Vec2, b: Vec2, angle: f64) -> Mobject {
    arrow_along(arc_between_path(a, b, angle), ArrowStyle::default())
}

/// Curved arrow with tips at both ends (manim's `CurvedDoubleArrow`).
pub fn curved_double_arrow(a: Vec2, b: Vec2, angle: f64) -> Mobject {
    arrow_along(
        arc_between_path(a, b, angle),
        ArrowStyle::default().double(),
    )
}

/// Smaller dot (manim's `SmallDot`, radius 0.04).
pub fn small_dot(p: Vec2) -> Mobject {
    dot_r(p, 0.04, WHITE)
}

/// Brace along the segment `a → b` (manim's `LineBrace`). `side` picks the side
/// relative to the line: `UP` = the left of `a → b`, `DOWN` = the right.
pub fn line_brace(a: Vec2, b: Vec2, side: Vec2, buff: f64) -> Mobject {
    let d = (b - a).normalize();
    let left = side.y >= 0.0;
    let n = if left { d.perp() } else { -d.perp() };
    let (p, q) = (a + n * buff, b + n * buff);
    // brace_between bulges to the right of its direction
    if left {
        brace_between(q, p)
    } else {
        brace_between(p, q)
    }
}

/// Brace curved around a circular arc (manim's `ArcBrace`). `outside = true`
/// puts it outside the arc (manim's default `RIGHT`), otherwise inside (`LEFT`).
pub fn arc_brace(center: Vec2, radius: f64, start: f64, angle: f64, outside: bool) -> Mobject {
    let len = (radius * angle.abs()).max(1e-6);
    let buff = 0.2_f64.min(radius * 0.3);
    let mut m = brace_between(Vec2::ZERO, v2(len, 0.0));
    // local (x along the arc, y < 0 towards the tip) -> polar around the centre
    m.apply_function_mut(&|p: Vec2| {
        let theta = start + angle * (p.x / len);
        let r = if outside {
            radius + buff - p.y
        } else {
            radius - buff + p.y
        };
        center + Vec2::polar(r.max(0.0), theta)
    });
    m
}

/// Arc of `radius` tangent to two lines (manim's `TangentialArc`). Lines are given
/// by two points each; `corner = (s1, s2)` (each ±1) chooses which of the four
/// corners by the direction along each line.
pub fn tangential_arc(
    line1: (Vec2, Vec2),
    line2: (Vec2, Vec2),
    radius: f64,
    corner: (f64, f64),
) -> Mobject {
    let d1 = (line1.1 - line1.0).normalize() * corner.0.signum();
    let d2 = (line2.1 - line2.0).normalize() * corner.1.signum();
    let den = (line1.1 - line1.0).cross(line2.1 - line2.0);
    if den.abs() < 1e-12 {
        return Mobject::empty();
    }
    let t = (line2.0 - line1.0).cross(line2.1 - line2.0) / den;
    let x = line1.0 + (line1.1 - line1.0) * t;
    let half = d1.dot(d2).clamp(-1.0, 1.0).acos() / 2.0;
    if half < 1e-9 {
        return Mobject::empty();
    }
    let k = radius / half.tan();
    let (t1, t2) = (x + d1 * k, x + d2 * k);
    // sweep from t1 to t2 around the centre (the arc is concave towards the corner)
    let sweep = (PI - 2.0 * half) * if d1.cross(d2) > 0.0 { -1.0 } else { 1.0 };
    arc_between(t1, t2, sweep)
}

/// Closed shape made of arcs joined end to start by straight lines (manim's
/// `ArcPolygonFromArcs`). The arcs are kept as children and drawn too, as in
/// manim; give them zero stroke width to hide them.
pub fn arc_polygon_from_arcs(arcs: &[Mobject]) -> Mobject {
    let mut p = Path::new();
    for (i, a) in arcs.iter().enumerate() {
        let ap = a.family_path();
        for (j, sp) in ap.subpaths.iter().enumerate() {
            if i == 0 && j == 0 {
                p.move_to(sp.start);
            } else if p
                .subpaths
                .last()
                .is_some_and(|l| l.end().distance(sp.start) > 1e-9)
            {
                p.line_to(sp.start);
            }
            if let Some(last) = p.subpaths.last_mut() {
                last.segs.extend(sp.segs.iter().copied());
            }
        }
    }
    p.close();
    let mut m = Mobject::from_path(p);
    m.children = arcs.to_vec();
    m
}

/// Tapered curly brace spanning `a → b`, bulging to the right of the direction a→b
/// (i.e. for a→b pointing right, the tip points down).
pub fn brace_between(a: Vec2, b: Vec2) -> Mobject {
    let w = a.distance(b);
    let depth = 0.26_f64.min(w * 0.3).max(0.1);
    let thick = 0.075_f64.min(w * 0.08).max(0.02);
    // Centre line in local frame: x along [0,w], y downwards negative.
    let n = 80;
    let r = (depth * 0.5).min(w / 4.0);
    let half = w / 2.0;
    let centre = |x: f64| -> f64 {
        // left half profile mirrored for right half
        let x = (if x > half { w - x } else { x }).clamp(0.0, half);
        let y1 = depth * 0.5;
        if x < r {
            // quarter-ellipse rise from 0 to y1
            let u = (x / r).clamp(0.0, 1.0);
            y1 * (1.0 - (1.0 - u) * (1.0 - u)).sqrt()
        } else if x > half - r {
            let u = ((half - x) / r).clamp(0.0, 1.0);
            y1 + (depth - y1) * (1.0 - (1.0 - (1.0 - u) * (1.0 - u)).sqrt())
        } else {
            y1
        }
    };
    let pts: Vec<Vec2> = (0..=n)
        .map(|i| {
            let x = w * i as f64 / n as f64;
            v2(x, -centre(x))
        })
        .collect();
    let thickness = |x: f64| -> f64 {
        let u = (if x > half { w - x } else { x }) / half; // 0 at ends, 1 at middle
        thick * (0.25 + 0.75 * (u * PI).sin().abs().max(0.0)).max(0.3)
    };
    let mut upper = Vec::new();
    let mut lower = Vec::new();
    for i in 0..pts.len() {
        let p = pts[i];
        let q = if i + 1 < pts.len() {
            pts[i + 1]
        } else {
            pts[i]
        };
        let o = if i > 0 { pts[i - 1] } else { pts[i] };
        let t = (q - o).normalize();
        let nrm = t.perp();
        let th = thickness(p.x) / 2.0;
        upper.push(p + nrm * th);
        lower.push(p - nrm * th);
    }
    lower.reverse();
    upper.extend(lower);
    let mut path = Path::polyline(&upper, true);
    let ang = (b - a).angle();
    path.transform(&Affine::translate(a).then_after(Affine::rotate(ang)));
    let mut m = Mobject::from_path_styled(path, Style::filled(WHITE));
    m.style.stroke_width = 0.0;
    m
}

/// Brace hugging `target` on the side `dir` (manim's `Brace`).
pub fn brace(target: &Mobject, dir: Vec2, buff: f64) -> Mobject {
    let b = target.bounds_or_origin().expand(buff);
    let d = dir.normalize();
    // pick endpoints so that the brace bulges towards `dir`
    let (p, q) = if d.y.abs() >= d.x.abs() {
        if d.y < 0.0 {
            (v2(b.min.x, b.min.y), v2(b.max.x, b.min.y))
        } else {
            (v2(b.max.x, b.max.y), v2(b.min.x, b.max.y))
        }
    } else if d.x > 0.0 {
        (v2(b.max.x, b.min.y), v2(b.max.x, b.max.y))
    } else {
        (v2(b.min.x, b.max.y), v2(b.min.x, b.min.y))
    };
    brace_between(p, q)
}

/// Tip point of a brace made by [`brace`] (where a label should go).
pub fn brace_tip(brace: &Mobject, dir: Vec2) -> Vec2 {
    brace.edge(dir)
}

pub fn cross(size: f64) -> Mobject {
    let h = size / 2.0;
    Mobject::group([line(v2(-h, -h), v2(h, h)), line(v2(-h, h), v2(h, -h))]).stroke(RED, 6.0)
}

pub fn checkmark(size: f64) -> Mobject {
    let s = size;
    polyline(&[
        v2(-0.5 * s, 0.0),
        v2(-0.15 * s, -0.35 * s),
        v2(0.5 * s, 0.4 * s),
    ])
    .stroke(GREEN, 8.0)
}

/// Arc marking the angle at `vertex` from direction `d1` to `d2` (counter-clockwise).
pub fn angle_arc(vertex: Vec2, d1: Vec2, d2: Vec2, radius: f64) -> Mobject {
    let a1 = d1.angle();
    let mut sweep = d2.angle() - a1;
    while sweep < 0.0 {
        sweep += TAU;
    }
    arc_centered(vertex, radius, a1, sweep)
}

/// Square corner marker for a right angle.
pub fn right_angle(vertex: Vec2, d1: Vec2, d2: Vec2, size: f64) -> Mobject {
    let a = d1.normalize() * size;
    let b = d2.normalize() * size;
    polyline(&[vertex + a, vertex + a + b, vertex + b])
}

pub fn surrounding_rectangle(target: &Mobject, buff: f64) -> Mobject {
    let b = target.bounds_or_origin().expand(buff);
    rectangle(b.width(), b.height())
        .move_to(b.center())
        .stroke(YELLOW, 4.0)
}

pub fn surrounding_rounded(target: &Mobject, buff: f64, radius: f64) -> Mobject {
    let b = target.bounds_or_origin().expand(buff);
    stroked(rounded_rect_path(b.center(), b.width(), b.height(), radius)).stroke(YELLOW, 4.0)
}

pub fn background_rectangle(target: &Mobject, color: Color, opacity: f32, buff: f64) -> Mobject {
    let b = target.bounds_or_origin().expand(buff);
    rectangle(b.width(), b.height())
        .move_to(b.center())
        .fill(color, opacity)
        .no_stroke()
}

pub fn underline(target: &Mobject, buff: f64) -> Mobject {
    let b = target.bounds_or_origin();
    line(v2(b.min.x, b.min.y - buff), v2(b.max.x, b.min.y - buff))
}

/// Rectangular grid of lines.
pub fn grid(width: f64, height: f64, step: f64) -> Mobject {
    let mut g = Mobject::empty();
    let (w, h) = (width / 2.0, height / 2.0);
    for x in arange(-w, w + 1e-9, step) {
        g.push(line(v2(x, -h), v2(x, h)));
    }
    for y in arange(-h, h + 1e-9, step) {
        g.push(line(v2(-w, y), v2(w, y)));
    }
    g.stroke(GRAY_D, 1.5)
}

/// Zig-zag (resistor / spring style) between two points.
pub fn zigzag(a: Vec2, b: Vec2, teeth: usize, amplitude: f64, lead: f64) -> Mobject {
    let d = b - a;
    let len = d.length();
    let u = d.normalize();
    let n = u.perp();
    let lead = lead.min(len * 0.45);
    let mut pts = vec![a, a + u * lead];
    let body = len - 2.0 * lead;
    let k = teeth.max(1) * 2;
    for i in 1..k {
        let s = lead + body * i as f64 / k as f64;
        let side = if i % 2 == 1 { 1.0 } else { -1.0 };
        pts.push(a + u * s + n * (amplitude * side));
    }
    pts.push(b - u * lead);
    pts.push(b);
    polyline(&pts)
}

/// Helical coil (spring seen from the side) between two points.
pub fn coil(a: Vec2, b: Vec2, turns: f64, radius: f64, lead: f64) -> Mobject {
    let d = b - a;
    let len = d.length();
    let u = d.normalize();
    let n = u.perp();
    let lead = lead.min(len * 0.3);
    let body = len - 2.0 * lead;
    let samples = (turns * 24.0).ceil() as usize + 2;
    let mut pts = vec![a];
    for i in 0..=samples {
        let t = i as f64 / samples as f64;
        let ang = t * turns * TAU;
        // slight lean gives a 3D coil look
        let s = lead + body * t + radius * 0.35 * ang.cos();
        pts.push(a + u * s + n * (radius * ang.sin()));
    }
    pts.push(b);
    smooth_curve(&pts, false)
}

/// Sine wave from `a` to `b`.
pub fn wave_line(a: Vec2, b: Vec2, wavelength: f64, amplitude: f64, phase: f64) -> Mobject {
    let d = b - a;
    let len = d.length();
    let u = d.normalize();
    let n = u.perp();
    let samples = ((len / wavelength) * 16.0).ceil().max(8.0) as usize;
    let pts: Vec<Vec2> = (0..=samples)
        .map(|i| {
            let s = len * i as f64 / samples as f64;
            a + u * s + n * (amplitude * (TAU * s / wavelength + phase).sin())
        })
        .collect();
    smooth_curve(&pts, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_have_expected_bounds() {
        assert!((circle(1.0).width() - 2.0).abs() < 1e-6);
        assert!((rectangle(4.0, 2.0).height() - 2.0).abs() < 1e-9);
        assert_eq!(regular_polygon(6, 1.0).anchors().len(), 7);
        let a = arrow(v2(0.0, 0.0), v2(2.0, 0.0));
        assert_eq!(a.children.len(), 2);
        assert!((a.right().x - 2.0).abs() < 1e-6);
        let arc = arc_between(v2(-1.0, 0.0), v2(1.0, 0.0), PI);
        assert!((arc.start_point() - v2(-1.0, 0.0)).length() < 1e-6);
        assert!((arc.end_point() - v2(1.0, 0.0)).length() < 1e-6);
        let b = brace(&rectangle(2.0, 1.0), DOWN, 0.1);
        assert!(b.top().y <= -0.5);
        assert!((b.width() - 2.2).abs() < 0.05);
    }

    #[test]
    fn new_geometry() {
        // arc tangent to the x and y axes, radius 1, in the first quadrant
        let arc = tangential_arc(
            (v2(-3.0, 0.0), v2(3.0, 0.0)),
            (v2(0.0, -3.0), v2(0.0, 3.0)),
            1.0,
            (1.0, 1.0),
        );
        for p in arc.anchors() {
            assert!((p.distance(v2(1.0, 1.0)) - 1.0).abs() < 1e-6, "{p:?}");
        }
        let ends = [arc.start_point(), arc.end_point()];
        assert!(ends.iter().any(|p| (*p - v2(1.0, 0.0)).length() < 1e-6));
        assert!(ends.iter().any(|p| (*p - v2(0.0, 1.0)).length() < 1e-6));
        // brace around an arc: outside lies beyond the radius, inside within
        let out = arc_brace(Vec2::ZERO, 2.0, 0.0, PI / 2.0, true);
        assert!(out.anchors().iter().all(|p| p.length() > 2.0 - 1e-6));
        let inn = arc_brace(Vec2::ZERO, 2.0, 0.0, PI / 2.0, false);
        assert!(inn.anchors().iter().all(|p| p.length() < 2.0 + 1e-6));
        // line brace on the left side of a rightward line is above it
        let lb = line_brace(v2(-1.0, 0.0), v2(1.0, 0.0), UP, 0.1);
        assert!(lb.bottom().y >= 0.09 && (lb.width() - 2.0).abs() < 0.05);
        let lb = line_brace(v2(-1.0, 0.0), v2(1.0, 0.0), DOWN, 0.1);
        assert!(lb.top().y <= -0.09);
        // reuleaux triangle from arcs
        let (a, b, c) = (v2(-1.0, 0.0), v2(1.0, 0.0), v2(0.0, 3f64.sqrt()));
        let r = arc_polygon_from_arcs(&[
            arc_between(a, b, PI / 3.0),
            arc_between(b, c, PI / 3.0),
            arc_between(c, a, PI / 3.0),
        ]);
        assert_eq!(r.children.len(), 3);
        assert!(r.path.subpaths[0].closed);
        assert!((r.width() - 2.0).abs() < 1e-3);
        // outlined tips are stroked and the shaft stops at the tip base
        let ar = arrow_styled(
            v2(0.0, 0.0),
            v2(3.0, 0.0),
            ArrowStyle::default().tip(TipShape::TriangleOutline),
        );
        let tip = &ar.children[1];
        assert!(tip.style.fill_opacity == 0.0 && tip.style.stroke_width > 0.0);
        assert!(ar.children[0].right().x <= 3.0 - 0.28 + 1e-6);
        let d = curved_double_arrow(a, b, 1.0);
        assert_eq!(d.children.len(), 3);
        assert!((small_dot(Vec2::ZERO).width() - 0.08).abs() < 1e-9);
    }
}
