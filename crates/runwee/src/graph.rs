//! Coordinate systems and data visualisation: NumberLine, Axes, NumberPlane,
//! ComplexPlane, PolarPlane, function/parametric/implicit plots, areas, Riemann
//! sums, tangent lines, vector fields, stream lines, bar charts, tables,
//! matrices, decimal numbers and network graphs.

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::shapes::*;
use crate::style::*;
use crate::tex::{tex_sized, MathTex};
use crate::text::{text_sized, DEFAULT_FONT_SIZE};

/// Format a number for tick labels (drops trailing zeros).
pub fn fmt_num(v: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, v);
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    if s == "-0" {
        "0".into()
    } else {
        s
    }
}

// ------------------------------------------------------------------ NumberLine

/// A number line (manim's `NumberLine`).
#[derive(Clone, Debug)]
pub struct NumberLine {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub length: f64,
    pub center: Vec2,
    /// Direction of increasing values.
    pub dir: Vec2,
    pub color: Color,
    pub stroke_width: f32,
    pub tick_size: f64,
    pub include_numbers: bool,
    pub number_size: f64,
    pub decimals: usize,
    pub include_tip: bool,
    pub exclude_numbers: Vec<f64>,
    /// Logarithmic scale (values are exponents of `base` along the line).
    pub log_base: Option<f64>,
}

impl NumberLine {
    pub fn new(min: f64, max: f64, step: f64) -> Self {
        Self {
            min,
            max,
            step,
            length: (max - min).abs().max(1e-9),
            center: Vec2::ZERO,
            dir: RIGHT,
            color: GRAY_A,
            stroke_width: 2.5,
            tick_size: 0.1,
            include_numbers: false,
            number_size: 0.4,
            decimals: 2,
            include_tip: false,
            exclude_numbers: vec![],
            log_base: None,
        }
    }
    pub fn length(mut self, l: f64) -> Self {
        self.length = l;
        self
    }
    pub fn numbers(mut self, on: bool) -> Self {
        self.include_numbers = on;
        self
    }
    pub fn tip(mut self, on: bool) -> Self {
        self.include_tip = on;
        self
    }
    pub fn at(mut self, c: Vec2) -> Self {
        self.center = c;
        self
    }
    pub fn vertical(mut self) -> Self {
        self.dir = UP;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    /// manim's `UnitInterval`: 0 to 1 in steps of 0.1, numbered with 1 decimal.
    pub fn unit_interval(length: f64) -> Self {
        let mut nl = NumberLine::new(0.0, 1.0, 0.1).length(length);
        nl.decimals = 1;
        nl
    }
    /// Scene point for value `v` (manim's `n2p`).
    pub fn n2p(&self, v: f64) -> Vec2 {
        let u = (v - self.min) / (self.max - self.min);
        self.center + self.dir.normalize() * (self.length * (u - 0.5))
    }
    /// Value for a scene point (manim's `p2n`).
    pub fn p2n(&self, p: Vec2) -> f64 {
        let u = (p - self.center).dot(self.dir.normalize()) / self.length + 0.5;
        self.min + u * (self.max - self.min)
    }
    pub fn ticks(&self) -> Vec<f64> {
        arange(self.min, self.max + 1e-9, self.step)
    }
    pub fn build(&self) -> Mobject {
        let d = self.dir.normalize();
        let n = d.perp();
        let a = self.n2p(self.min);
        let b = self.n2p(self.max);
        let mut g = Mobject::empty();
        let main = if self.include_tip {
            arrow_styled(
                a,
                b + d * 0.25,
                ArrowStyle {
                    stroke_width: self.stroke_width,
                    tip_length: 0.2,
                    tip_width: 0.18,
                    color: self.color,
                    ..Default::default()
                },
            )
        } else {
            line(a, b).stroke(self.color, self.stroke_width)
        };
        g.push(main.with_tag("axis"));
        let mut ticks = Mobject::empty().with_tag("ticks");
        let mut nums = Mobject::empty().with_tag("numbers");
        for v in self.ticks() {
            let p = self.n2p(v);
            ticks.push(
                line(p - n * self.tick_size, p + n * self.tick_size)
                    .stroke(self.color, self.stroke_width),
            );
            if self.include_numbers && !self.exclude_numbers.iter().any(|e| (e - v).abs() < 1e-9) {
                let label = match self.log_base {
                    Some(base) => format!("{}^{{{}}}", fmt_num(base, 0), fmt_num(v, self.decimals)),
                    None => fmt_num(v, self.decimals),
                };
                let t = tex_sized(&label, self.number_size, self.color);
                let side = if d.y.abs() > 0.5 { LEFT } else { DOWN };
                nums.push(t.next_to_point(p, side, 0.18 + self.tick_size));
            }
        }
        g.push(ticks);
        g.push(nums);
        g
    }
}

// ------------------------------------------------------------------------ Axes

/// 2D axes with a coordinate mapping (manim's `Axes`).
#[derive(Clone, Debug)]
pub struct Axes {
    pub x_range: (f64, f64, f64),
    pub y_range: (f64, f64, f64),
    pub x_length: f64,
    pub y_length: f64,
    pub center: Vec2,
    pub color: Color,
    pub include_numbers: bool,
    pub include_tips: bool,
    pub number_size: f64,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub x_log: bool,
    pub y_log: bool,
}

impl Axes {
    pub fn new(x_range: (f64, f64, f64), y_range: (f64, f64, f64)) -> Self {
        Self {
            x_range,
            y_range,
            x_length: 10.0,
            y_length: 6.0,
            center: Vec2::ZERO,
            color: GRAY_A,
            include_numbers: true,
            include_tips: true,
            number_size: 0.36,
            x_label: None,
            y_label: None,
            x_log: false,
            y_log: false,
        }
    }
    pub fn size(mut self, w: f64, h: f64) -> Self {
        self.x_length = w;
        self.y_length = h;
        self
    }
    pub fn at(mut self, c: Vec2) -> Self {
        self.center = c;
        self
    }
    pub fn numbers(mut self, on: bool) -> Self {
        self.include_numbers = on;
        self
    }
    pub fn tips(mut self, on: bool) -> Self {
        self.include_tips = on;
        self
    }
    pub fn labels(mut self, x: &str, y: &str) -> Self {
        self.x_label = Some(x.into());
        self.y_label = Some(y.into());
        self
    }
    /// Logarithmic y axis (y_range given as exponents of 10).
    pub fn y_log(mut self) -> Self {
        self.y_log = true;
        self
    }
    pub fn x_log(mut self) -> Self {
        self.x_log = true;
        self
    }
    fn tx(&self, x: f64) -> f64 {
        if self.x_log {
            x.max(1e-300).log10()
        } else {
            x
        }
    }
    fn ty(&self, y: f64) -> f64 {
        if self.y_log {
            y.max(1e-300).log10()
        } else {
            y
        }
    }
    /// Coordinates → scene point (manim's `c2p`).
    pub fn c2p(&self, x: f64, y: f64) -> Vec2 {
        let (x0, x1, _) = self.x_range;
        let (y0, y1, _) = self.y_range;
        let u = (self.tx(x) - x0) / (x1 - x0);
        let v = (self.ty(y) - y0) / (y1 - y0);
        self.center + v2((u - 0.5) * self.x_length, (v - 0.5) * self.y_length)
    }
    pub fn c2p_v(&self, p: Vec2) -> Vec2 {
        self.c2p(p.x, p.y)
    }
    /// Scene point → coordinates (manim's `p2c`).
    pub fn p2c(&self, p: Vec2) -> Vec2 {
        let (x0, x1, _) = self.x_range;
        let (y0, y1, _) = self.y_range;
        let d = p - self.center;
        let mut x = x0 + (d.x / self.x_length + 0.5) * (x1 - x0);
        let mut y = y0 + (d.y / self.y_length + 0.5) * (y1 - y0);
        if self.x_log {
            x = 10f64.powf(x);
        }
        if self.y_log {
            y = 10f64.powf(y);
        }
        v2(x, y)
    }
    /// Scene point of the coordinate origin (clamped into the visible ranges).
    pub fn origin(&self) -> Vec2 {
        let ox = 0f64.clamp(self.x_range.0, self.x_range.1);
        let oy = 0f64.clamp(self.y_range.0, self.y_range.1);
        let (x0, x1, _) = self.x_range;
        let (y0, y1, _) = self.y_range;
        self.center
            + v2(
                ((ox - x0) / (x1 - x0) - 0.5) * self.x_length,
                ((oy - y0) / (y1 - y0) - 0.5) * self.y_length,
            )
    }
    pub fn x_axis(&self) -> NumberLine {
        let o = self.origin();
        let mut nl = NumberLine::new(self.x_range.0, self.x_range.1, self.x_range.2)
            .length(self.x_length)
            .at(v2(self.center.x, o.y))
            .numbers(self.include_numbers)
            .tip(self.include_tips)
            .color(self.color);
        nl.number_size = self.number_size;
        nl.exclude_numbers = vec![0.0];
        if self.x_log {
            nl.log_base = Some(10.0);
        }
        nl
    }
    pub fn y_axis(&self) -> NumberLine {
        let o = self.origin();
        let mut nl = NumberLine::new(self.y_range.0, self.y_range.1, self.y_range.2)
            .length(self.y_length)
            .at(v2(o.x, self.center.y))
            .vertical()
            .numbers(self.include_numbers)
            .tip(self.include_tips)
            .color(self.color);
        nl.number_size = self.number_size;
        nl.exclude_numbers = vec![0.0];
        if self.y_log {
            nl.log_base = Some(10.0);
        }
        nl
    }
    pub fn build(&self) -> Mobject {
        let mut g = Mobject::group([self.x_axis().build(), self.y_axis().build()]);
        if let Some(l) = &self.x_label {
            let t = tex_sized(l, 0.45, self.color);
            let p = self.c2p(self.x_range.1, 0f64.clamp(self.y_range.0, self.y_range.1));
            g.push(
                t.next_to_point(p + RIGHT * 0.25, RIGHT, 0.1)
                    .with_tag("x_label"),
            );
        }
        if let Some(l) = &self.y_label {
            let t = tex_sized(l, 0.45, self.color);
            let p = self.c2p(0f64.clamp(self.x_range.0, self.x_range.1), self.y_range.1);
            g.push(t.next_to_point(p + UP * 0.25, UP, 0.1).with_tag("y_label"));
        }
        g
    }

    /// Graph of `y = f(x)` (manim's `plot`). Discontinuities (NaN/∞/jumps) split the curve.
    pub fn plot(&self, f: impl Fn(f64) -> f64) -> Mobject {
        self.plot_range(f, self.x_range.0, self.x_range.1)
    }
    pub fn plot_range(&self, f: impl Fn(f64) -> f64, x0: f64, x1: f64) -> Mobject {
        let n = 600;
        let (y0, y1, _) = self.y_range;
        let span = (y1 - y0).abs();
        // a little headroom above/below the visible range, like manim's default clipping
        let (lo, hi) = (y0 - span * 0.02, y1 + span * 0.02);
        let mut runs: Vec<Vec<Vec2>> = vec![vec![]];
        let mut prev: Option<(f64, f64)> = None; // (x, transformed y)
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i as f64 / n as f64;
            let ty = self.ty(f(x));
            let inside = ty.is_finite() && ty >= lo && ty <= hi;
            let jump = prev.is_some_and(|(_, py)| ty.is_finite() && (ty - py).abs() > span * 1.5);
            if let Some((px, py)) = prev {
                let prev_inside = py.is_finite() && py >= lo && py <= hi;
                if !jump && ty.is_finite() && py.is_finite() && prev_inside != inside {
                    // add the boundary crossing point
                    let edge = if (if inside { py } else { ty }) > hi {
                        hi
                    } else {
                        lo
                    };
                    let t = (edge - py) / (ty - py);
                    let cx = px + (x - px) * t;
                    runs.last_mut().unwrap().push(self.c2p_t(cx, edge));
                }
                if (!inside && prev_inside) || jump {
                    runs.push(vec![]);
                }
            }
            if inside {
                runs.last_mut().unwrap().push(self.c2p_t(x, ty));
            }
            prev = Some((x, ty));
        }
        let mut path = Path::new();
        for r in runs.into_iter().filter(|r| r.len() > 1) {
            path.subpaths
                .extend(Path::smooth_through(&r, false).subpaths);
        }
        Mobject::from_path(path)
            .stroke(YELLOW, 4.0)
            .with_tag("graph")
    }
    /// `c2p` with an already log-transformed y.
    fn c2p_t(&self, x: f64, ty: f64) -> Vec2 {
        let (x0, x1, _) = self.x_range;
        let (y0, y1, _) = self.y_range;
        let u = (self.tx(x) - x0) / (x1 - x0);
        let v = (ty - y0) / (y1 - y0);
        self.center + v2((u - 0.5) * self.x_length, (v - 0.5) * self.y_length)
    }
    /// Parametric curve `t ↦ (x(t), y(t))` in axis coordinates.
    pub fn plot_parametric(&self, f: impl Fn(f64) -> Vec2, t0: f64, t1: f64) -> Mobject {
        let pts: Vec<Vec2> = linspace(t0, t1, 300)
            .into_iter()
            .map(|t| self.c2p_v(f(t)))
            .collect();
        smooth_curve(&pts, false).stroke(BLUE, 4.0)
    }
    /// Polar curve `r(θ)` in axis coordinates.
    pub fn plot_polar(&self, r: impl Fn(f64) -> f64, th0: f64, th1: f64) -> Mobject {
        self.plot_parametric(|t| Vec2::polar(r(t), t), th0, th1)
    }
    /// Implicit curve `f(x, y) = 0` via marching squares.
    pub fn plot_implicit(&self, f: impl Fn(f64, f64) -> f64, resolution: usize) -> Mobject {
        let (x0, x1, _) = self.x_range;
        let (y0, y1, _) = self.y_range;
        let segs = marching_squares(&f, x0, x1, y0, y1, resolution);
        let mut p = Path::new();
        for (a, b) in segs {
            p.move_to(self.c2p_v(a));
            p.line_to(self.c2p_v(b));
        }
        Mobject::from_path(p)
            .stroke(TEAL, 4.0)
            .line_cap(LineCap::Round)
    }
    /// Scatter points.
    pub fn scatter(&self, pts: &[Vec2], color: Color, r: f64) -> Mobject {
        Mobject::group(pts.iter().map(|p| dot_r(self.c2p_v(*p), r, color)))
    }
    /// Polyline through data points (line chart).
    pub fn line_graph(&self, pts: &[Vec2], color: Color, with_dots: bool) -> Mobject {
        let sp: Vec<Vec2> = pts.iter().map(|p| self.c2p_v(*p)).collect();
        let mut g = Mobject::group([polyline(&sp).stroke(color, 4.0)]);
        if with_dots {
            for p in &sp {
                g.push(dot_r(*p, 0.07, color));
            }
        }
        g
    }
    /// Shaded area between `f` and `g` (or the x-axis) on [x0, x1].
    pub fn area(
        &self,
        f: impl Fn(f64) -> f64,
        x0: f64,
        x1: f64,
        g: Option<&dyn Fn(f64) -> f64>,
        color: Color,
        opacity: f32,
    ) -> Mobject {
        let n = 200;
        let mut pts: Vec<Vec2> = (0..=n)
            .map(|i| {
                let x = x0 + (x1 - x0) * i as f64 / n as f64;
                self.c2p(x, f(x))
            })
            .collect();
        for i in (0..=n).rev() {
            let x = x0 + (x1 - x0) * i as f64 / n as f64;
            let y = g.map_or(0.0, |g| g(x));
            pts.push(self.c2p(x, y));
        }
        polygon(&pts)
            .fill(color, opacity)
            .no_stroke()
            .with_tag("area")
    }
    /// Riemann rectangles (`method`: "left", "right", "mid").
    pub fn riemann(
        &self,
        f: impl Fn(f64) -> f64,
        x0: f64,
        x1: f64,
        n: usize,
        method: &str,
        colors: &[Color],
    ) -> Mobject {
        let dx = (x1 - x0) / n as f64;
        let cs = color_range(
            if colors.is_empty() {
                &[BLUE, GREEN]
            } else {
                colors
            },
            n,
        );
        Mobject::group((0..n).map(|i| {
            let a = x0 + i as f64 * dx;
            let sx = match method {
                "right" => a + dx,
                "mid" | "midpoint" => a + dx / 2.0,
                _ => a,
            };
            let h = f(sx);
            let p0 = self.c2p(a, 0.0);
            let p1 = self.c2p(a + dx, h);
            let w = p1.x - p0.x;
            let hh = p1.y - p0.y;
            rectangle(w.abs(), hh.abs())
                .move_to(v2(p0.x + w / 2.0, p0.y + hh / 2.0))
                .fill(cs[i], 0.7)
                .stroke(BLACK, 1.5)
        }))
    }
    /// Tangent line to `f` at `x`, `length` scene units long.
    pub fn tangent_line(&self, f: impl Fn(f64) -> f64, x: f64, length: f64) -> Mobject {
        let h = 1e-4 * (self.x_range.1 - self.x_range.0);
        let p = self.c2p(x, f(x));
        let q = self.c2p(x + h, f(x + h));
        let d = (q - p).normalize();
        line(p - d * (length / 2.0), p + d * (length / 2.0)).stroke(ORANGE, 3.5)
    }
    /// Secant line between x and x+dx with dx/dy legs (manim's `get_secant_slope_group`).
    pub fn secant_group(&self, f: impl Fn(f64) -> f64, x: f64, dx: f64, color: Color) -> Mobject {
        let p = self.c2p(x, f(x));
        let q = self.c2p(x + dx, f(x + dx));
        let corner = v2(q.x, p.y);
        let d = (q - p).normalize();
        Mobject::group([
            line(p, corner).stroke(GREEN, 3.0).with_tag("dx"),
            line(corner, q).stroke(RED, 3.0).with_tag("dy"),
            line(p - d * 1.0, q + d * 1.0)
                .stroke(color, 3.5)
                .with_tag("secant"),
        ])
    }
    /// Dashed lines from a point down to the x axis and across to the y axis.
    pub fn lines_to_point(&self, x: f64, y: f64, color: Color) -> Mobject {
        let p = self.c2p(x, y);
        Mobject::group([
            dashed_line(self.c2p(x, 0.0), p).stroke(color, 2.5),
            dashed_line(self.c2p(0.0, y), p).stroke(color, 2.5),
        ])
    }
    /// Label placed near the end of a graph.
    pub fn graph_label(&self, graph: &Mobject, label: &str, color: Color) -> Mobject {
        let end = graph.end_point();
        tex_sized(label, 0.45, color).next_to_point(end, UR, 0.1)
    }
    /// Background grid aligned with the ticks.
    pub fn grid(&self, color: Color, opacity: f32) -> Mobject {
        let mut g = Mobject::empty();
        for x in arange(self.x_range.0, self.x_range.1 + 1e-9, self.x_range.2) {
            g.push(line(
                self.c2p(x, self.y_range.0),
                self.c2p(x, self.y_range.1),
            ));
        }
        for y in arange(self.y_range.0, self.y_range.1 + 1e-9, self.y_range.2) {
            g.push(line(
                self.c2p(self.x_range.0, y),
                self.c2p(self.x_range.1, y),
            ));
        }
        g.stroke(color, 1.2).stroke_opacity(opacity)
    }
}

/// Segments approximating the zero set of `f` on a grid.
pub fn marching_squares(
    f: &dyn Fn(f64, f64) -> f64,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    n: usize,
) -> Vec<(Vec2, Vec2)> {
    let n = n.max(4);
    let dx = (x1 - x0) / n as f64;
    let dy = (y1 - y0) / n as f64;
    let val = |i: usize, j: usize| f(x0 + i as f64 * dx, y0 + j as f64 * dy);
    let mut grid = vec![vec![0.0; n + 1]; n + 1];
    for (i, row) in grid.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = val(i, j);
        }
    }
    let mut segs = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let corners = [
                (v2(x0 + i as f64 * dx, y0 + j as f64 * dy), grid[i][j]),
                (
                    v2(x0 + (i + 1) as f64 * dx, y0 + j as f64 * dy),
                    grid[i + 1][j],
                ),
                (
                    v2(x0 + (i + 1) as f64 * dx, y0 + (j + 1) as f64 * dy),
                    grid[i + 1][j + 1],
                ),
                (
                    v2(x0 + i as f64 * dx, y0 + (j + 1) as f64 * dy),
                    grid[i][j + 1],
                ),
            ];
            let mut pts = Vec::new();
            for k in 0..4 {
                let (pa, va) = corners[k];
                let (pb, vb) = corners[(k + 1) % 4];
                if (va < 0.0) != (vb < 0.0) && va.is_finite() && vb.is_finite() {
                    let t = va / (va - vb);
                    pts.push(pa.lerp(pb, t));
                }
            }
            if pts.len() >= 2 {
                segs.push((pts[0], pts[1]));
            }
            if pts.len() == 4 {
                segs.push((pts[2], pts[3]));
            }
        }
    }
    segs
}

// ----------------------------------------------------------------- planes

/// Cartesian grid with axes (manim's `NumberPlane`).
pub fn number_plane(x_range: (f64, f64, f64), y_range: (f64, f64, f64)) -> Mobject {
    let ax = Axes::new(x_range, y_range)
        .size(x_range.1 - x_range.0, y_range.1 - y_range.0)
        .numbers(false)
        .tips(false);
    let mut faint = Mobject::empty();
    let half_x = arange(x_range.0, x_range.1 + 1e-9, x_range.2 / 2.0);
    let half_y = arange(y_range.0, y_range.1 + 1e-9, y_range.2 / 2.0);
    for x in &half_x {
        faint.push(line(ax.c2p(*x, y_range.0), ax.c2p(*x, y_range.1)));
    }
    for y in &half_y {
        faint.push(line(ax.c2p(x_range.0, *y), ax.c2p(x_range.1, *y)));
    }
    let faint = faint.stroke(BLUE_D, 1.0).stroke_opacity(0.35);
    let main = ax.grid(BLUE_D, 0.9).stroke_width(2.0);
    let axes = Mobject::group([
        line(ax.c2p(x_range.0, 0.0), ax.c2p(x_range.1, 0.0)),
        line(ax.c2p(0.0, y_range.0), ax.c2p(0.0, y_range.1)),
    ])
    .stroke(WHITE, 2.5);
    Mobject::group([
        faint.with_tag("faint"),
        main.with_tag("grid"),
        axes.with_tag("axes"),
    ])
}

/// The default full-frame plane.
pub fn default_plane() -> Mobject {
    let (w, h) = frame_size();
    let hx = (w / 2.0).ceil();
    let hy = (h / 2.0).ceil();
    number_plane((-hx, hx, 1.0), (-hy, hy, 1.0))
}

/// Complex plane with `Re`/`Im` labels.
pub fn complex_plane(range: f64) -> Mobject {
    let mut g = number_plane((-range, range, 1.0), (-range * 0.6, range * 0.6, 1.0));
    for k in 1..=range as i64 {
        for (v, s) in [(k as f64, format!("{k}")), (-(k as f64), format!("-{k}"))] {
            g.push(tex_sized(&s, 0.32, GRAY_A).next_to_point(v2(v, 0.0), DL, 0.05));
        }
    }
    for k in 1..=(range * 0.6) as i64 {
        g.push(tex_sized(&format!("{k}i"), 0.32, GRAY_A).next_to_point(
            v2(0.0, k as f64),
            DL,
            0.05,
        ));
        g.push(tex_sized(&format!("-{k}i"), 0.32, GRAY_A).next_to_point(
            v2(0.0, -(k as f64)),
            DL,
            0.05,
        ));
    }
    g
}

/// Polar grid: concentric circles and radial lines.
pub fn polar_plane(radius: f64, rings: usize, spokes: usize) -> Mobject {
    let mut g = Mobject::empty();
    for i in 1..=rings {
        g.push(
            circle(radius * i as f64 / rings as f64)
                .stroke(BLUE_D, 1.5)
                .stroke_opacity(0.8),
        );
    }
    for k in 0..spokes {
        let a = TAU * k as f64 / spokes as f64;
        g.push(
            line(Vec2::ZERO, Vec2::polar(radius, a))
                .stroke(BLUE_D, 1.2)
                .stroke_opacity(0.6),
        );
    }
    for k in 0..spokes {
        let a = TAU * k as f64 / spokes as f64;
        let deg = (a / DEGREES).round();
        g.push(
            tex_sized(&format!("{}^\\circ", deg), 0.28, GRAY_B)
                .move_to(Vec2::polar(radius + 0.3, a)),
        );
    }
    g
}

// ------------------------------------------------------------ vector fields

/// Arrow field sampled on a grid (manim's `ArrowVectorField`).
pub fn vector_field(
    f: impl Fn(Vec2) -> Vec2,
    x_range: (f64, f64, f64),
    y_range: (f64, f64, f64),
    length_scale: f64,
    colors: &[Color],
) -> Mobject {
    let mut samples = Vec::new();
    let mut max_mag: f64 = 1e-9;
    for x in arange(x_range.0, x_range.1 + 1e-9, x_range.2) {
        for y in arange(y_range.0, y_range.1 + 1e-9, y_range.2) {
            let p = v2(x, y);
            let v = f(p);
            if v.is_finite() {
                max_mag = max_mag.max(v.length());
                samples.push((p, v));
            }
        }
    }
    let palette: Vec<Color> = if colors.is_empty() {
        vec![BLUE_E, GREEN, YELLOW, RED]
    } else {
        colors.to_vec()
    };
    let step = x_range.2.min(y_range.2);
    Mobject::group(
        samples
            .into_iter()
            .filter(|(_, v)| v.length() > 1e-9)
            .map(|(p, v)| {
                let mag = v.length();
                let len = (mag * length_scale).min(step * 0.9).max(step * 0.15);
                let d = v.normalize() * len;
                let c = color_gradient(&palette, (mag / max_mag).sqrt());
                arrow_styled(
                    p - d / 2.0,
                    p + d / 2.0,
                    ArrowStyle {
                        stroke_width: 3.0,
                        tip_length: (len * 0.35).min(0.18),
                        tip_width: (len * 0.3).min(0.15),
                        color: c,
                        max_tip_ratio: 0.45,
                        ..Default::default()
                    },
                )
            }),
    )
}

/// Trace a stream line from `start` with RK4.
pub fn stream_line(
    f: &dyn Fn(Vec2) -> Vec2,
    start: Vec2,
    dt: f64,
    steps: usize,
    bounds: Rect,
) -> Vec<Vec2> {
    let mut pts = vec![start];
    let mut p = start;
    for _ in 0..steps {
        let k1 = f(p);
        let k2 = f(p + k1 * (dt / 2.0));
        let k3 = f(p + k2 * (dt / 2.0));
        let k4 = f(p + k3 * dt);
        let v = (k1 + k2 * 2.0 + k3 * 2.0 + k4) / 6.0;
        if !v.is_finite() || v.length() < 1e-6 {
            break;
        }
        // normalised step so lines have even sampling
        p += v.normalize() * dt;
        if !bounds.contains(p) {
            break;
        }
        pts.push(p);
    }
    pts
}

/// Stream lines seeded on a jittered grid (manim's `StreamLines`).
pub fn stream_lines(
    f: impl Fn(Vec2) -> Vec2,
    x_range: (f64, f64, f64),
    y_range: (f64, f64, f64),
    colors: &[Color],
) -> Mobject {
    let bounds = Rect::new(v2(x_range.0, y_range.0), v2(x_range.1, y_range.1));
    let mut rng = Rng::new(7);
    let palette: Vec<Color> = if colors.is_empty() {
        vec![BLUE_D, TEAL, YELLOW]
    } else {
        colors.to_vec()
    };
    let mut g = Mobject::empty();
    let mut max_mag: f64 = 1e-9;
    for x in arange(x_range.0, x_range.1, x_range.2) {
        for y in arange(y_range.0, y_range.1, y_range.2) {
            max_mag = max_mag.max(f(v2(x, y)).length());
        }
    }
    for x in arange(x_range.0, x_range.1, x_range.2) {
        for y in arange(y_range.0, y_range.1, y_range.2) {
            let s = v2(x + rng.range(0.0, x_range.2), y + rng.range(0.0, y_range.2));
            let pts = stream_line(&f, s, 0.05, 120, bounds);
            if pts.len() < 4 {
                continue;
            }
            let mag = f(s).length();
            let c = color_gradient(&palette, (mag / max_mag).sqrt());
            g.push(smooth_curve(&pts, false).stroke(c, 2.0));
        }
    }
    g
}

/// A vector field of `f(p, t)` drawn at time `t` (manim's `TimeVaryingVectorField`;
/// see [`crate::scene::Scene::time_varying_vector_field`] for a live object).
pub fn time_varying_vector_field(
    f: impl Fn(Vec2, f64) -> Vec2,
    t: f64,
    x_range: (f64, f64, f64),
    y_range: (f64, f64, f64),
    length_scale: f64,
    colors: &[Color],
) -> Mobject {
    vector_field(|p| f(p, t), x_range, y_range, length_scale, colors)
}

/// Stream lines that keep flowing: each line shows a travelling flash of stroke
/// width with its own phase (manim's `AnimatedStreamLines`). Build once, then
/// call [`AnimatedStreamLines::frame`] with the elapsed time (pure function of time).
#[derive(Clone, Debug)]
pub struct AnimatedStreamLines {
    /// (line, phase offset in seconds, cycle duration)
    pub lines: Vec<(Mobject, f64, f64)>,
    pub time_width: f64,
}

impl AnimatedStreamLines {
    /// `lines` = children of e.g. [`stream_lines`]; lines start within `lag_range`
    /// seconds; each cycle lasts `virtual_time / rate_multiple`.
    pub fn new(lines: &Mobject, lag_range: f64, virtual_time: f64, rate_multiple: f64) -> Self {
        let mut rng = Rng::new(11);
        let run = virtual_time / rate_multiple.max(1e-9);
        Self {
            lines: lines
                .children
                .iter()
                .map(|l| (l.clone(), lag_range * rng.next_f64(), run))
                .collect(),
            time_width: 1.0,
        }
    }
    /// The flowing lines `t` seconds after start.
    pub fn frame(&self, t: f64) -> Mobject {
        Mobject::group(self.lines.iter().map(|(l, phase, run)| {
            let lt = (t - phase).max(0.0);
            if lt <= 0.0 {
                return Mobject::empty();
            }
            let a = (lt % run) / run;
            crate::anim::v_passing_flash_frame(l, a, self.time_width, 0.05)
        }))
    }
}

impl crate::scene::Scene {
    /// Live `AnimatedStreamLines` for the field `f` (manim's defaults: lag range 4 s,
    /// 3 s per cycle).
    pub fn animated_stream_lines(
        &mut self,
        f: impl Fn(Vec2) -> Vec2,
        x_range: (f64, f64, f64),
        y_range: (f64, f64, f64),
        colors: &[Color],
    ) -> crate::scene::Obj {
        let sl =
            AnimatedStreamLines::new(&stream_lines(f, x_range, y_range, colors), 4.0, 3.0, 1.0);
        self.always_redraw(move |ctx| sl.frame(ctx.elapsed))
    }
    /// A live vector field of `f(p, t)` with `t` = seconds since it was added
    /// (manim's `TimeVaryingVectorField`).
    pub fn time_varying_vector_field(
        &mut self,
        f: impl Fn(Vec2, f64) -> Vec2 + Send + Sync + 'static,
        x_range: (f64, f64, f64),
        y_range: (f64, f64, f64),
        length_scale: f64,
        colors: &[Color],
    ) -> crate::scene::Obj {
        let colors = colors.to_vec();
        self.always_redraw(move |ctx| {
            time_varying_vector_field(&f, ctx.elapsed, x_range, y_range, length_scale, &colors)
        })
    }
}

// ----------------------------------------------------------------- charts

/// Vertical bar chart (manim's `BarChart`). Children: axes, bars, labels, values.
pub fn bar_chart(
    values: &[f64],
    labels: &[&str],
    colors: &[Color],
    width: f64,
    height: f64,
) -> Mobject {
    let n = values.len().max(1);
    let max = values.iter().cloned().fold(0.0, f64::max).max(1e-9);
    let nice = nice_step(max / 5.0);
    let top = (max / nice).ceil() * nice;
    let ax = Axes::new((0.0, n as f64, 1.0), (0.0, top, nice))
        .size(width, height)
        .numbers(false)
        .tips(false);
    let mut g = Mobject::empty();
    let mut axes = Mobject::group([
        line(ax.c2p(0.0, 0.0), ax.c2p(n as f64, 0.0)),
        line(ax.c2p(0.0, 0.0), ax.c2p(0.0, top)),
    ])
    .stroke(GRAY_A, 2.5);
    for y in arange(0.0, top + 1e-9, nice) {
        axes.push(tex_sized(&fmt_num(y, 2), 0.3, GRAY_A).next_to_point(ax.c2p(0.0, y), LEFT, 0.15));
        axes.push(line(ax.c2p(0.0, y), ax.c2p(n as f64, y)).stroke(GRAY_D, 1.0));
    }
    g.push(axes.with_tag("axes"));
    let cs = if colors.is_empty() {
        color_range(&[BLUE, GREEN, YELLOW, RED], n)
    } else {
        colors.to_vec()
    };
    let mut bars = Mobject::empty().with_tag("bars");
    let mut labs = Mobject::empty().with_tag("labels");
    let mut vals = Mobject::empty().with_tag("values");
    for (i, v) in values.iter().enumerate() {
        let p0 = ax.c2p(i as f64 + 0.15, 0.0);
        let p1 = ax.c2p(i as f64 + 0.85, *v);
        let bar = rectangle((p1.x - p0.x).abs(), (p1.y - p0.y).abs().max(1e-4))
            .move_to((p0 + p1) / 2.0)
            .fill(cs[i % cs.len()], 0.9)
            .stroke(cs[i % cs.len()].lighter(0.3), 2.0);
        if let Some(l) = labels.get(i) {
            labs.push(text_sized(l, 0.3, GRAY_A).next_to_point(
                ax.c2p(i as f64 + 0.5, 0.0),
                DOWN,
                0.15,
            ));
        }
        vals.push(tex_sized(&fmt_num(*v, 2), 0.3, WHITE).next_to(&bar, UP, 0.1));
        bars.push(bar);
    }
    g.push(bars);
    g.push(labs);
    g.push(vals);
    g
}

/// Pie chart with optional labels (children are sectors).
pub fn pie_chart(values: &[f64], labels: &[&str], colors: &[Color], radius: f64) -> Mobject {
    let total: f64 = values.iter().sum::<f64>().max(1e-12);
    let cs = if colors.is_empty() {
        color_range(
            &[BLUE, TEAL, GREEN, YELLOW, ORANGE, RED, PURPLE],
            values.len().max(2),
        )
    } else {
        colors.to_vec()
    };
    let mut a = PI / 2.0;
    let mut g = Mobject::empty();
    for (i, v) in values.iter().enumerate() {
        let sweep = -TAU * v / total;
        let s = sector(radius, a, sweep)
            .fill(cs[i % cs.len()], 0.95)
            .stroke(BACKGROUND, 3.0);
        let mut grp = Mobject::group([s]);
        if let Some(l) = labels.get(i) {
            let mid = a + sweep / 2.0;
            grp.push(text_sized(l, 0.3, WHITE).move_to(Vec2::polar(radius * 0.65, mid)));
        }
        g.push(grp);
        a += sweep;
    }
    g
}

fn nice_step(x: f64) -> f64 {
    let e = 10f64.powf(x.log10().floor());
    let f = x / e;
    let n = if f < 1.5 {
        1.0
    } else if f < 3.0 {
        2.0
    } else if f < 7.0 {
        5.0
    } else {
        10.0
    };
    n * e
}

// --------------------------------------------------------- tables & matrices

/// Table of TeX entries (manim's `MathTable`); children: grid lines, then rows of cells.
pub fn math_table(rows: &[Vec<&str>], header: bool, size: f64) -> Mobject {
    let cells: Vec<Vec<Mobject>> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            r.iter()
                .map(|s| tex_sized(s, size, if header && i == 0 { YELLOW } else { WHITE }))
                .collect()
        })
        .collect();
    table_from(cells, size)
}

/// Table of plain text entries (manim's `Table`).
pub fn table(rows: &[Vec<&str>], header: bool, size: f64) -> Mobject {
    let cells: Vec<Vec<Mobject>> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            r.iter()
                .map(|s| {
                    if header && i == 0 {
                        crate::text::bold_text(s, size, YELLOW)
                    } else {
                        text_sized(s, size, WHITE)
                    }
                })
                .collect()
        })
        .collect();
    table_from(cells, size)
}

/// Table from arbitrary mobjects.
pub fn table_from(cells: Vec<Vec<Mobject>>, size: f64) -> Mobject {
    let nr = cells.len();
    let nc = cells.iter().map(|r| r.len()).max().unwrap_or(0);
    let pad = size * 0.6;
    let mut colw = vec![0.0f64; nc];
    let mut rowh = vec![0.0f64; nr];
    for (i, r) in cells.iter().enumerate() {
        for (j, c) in r.iter().enumerate() {
            colw[j] = colw[j].max(c.width() + 2.0 * pad);
            rowh[i] = rowh[i].max(c.height() + 2.0 * pad);
        }
    }
    let tw: f64 = colw.iter().sum();
    let th: f64 = rowh.iter().sum();
    let mut lines = Mobject::empty().with_tag("lines");
    let mut body = Mobject::empty().with_tag("cells");
    let mut y = th / 2.0;
    for (i, r) in cells.into_iter().enumerate() {
        let mut x = -tw / 2.0;
        let mut row = Mobject::empty();
        for (j, c) in r.into_iter().enumerate() {
            row.push(c.move_to(v2(x + colw[j] / 2.0, y - rowh[i] / 2.0)));
            x += colw[j];
        }
        body.push(row);
        y -= rowh[i];
    }
    let mut y = th / 2.0;
    for i in 0..=nr {
        lines.push(line(v2(-tw / 2.0, y), v2(tw / 2.0, y)));
        if i < nr {
            y -= rowh[i];
        }
    }
    let mut x = -tw / 2.0;
    for j in 0..=nc {
        lines.push(line(v2(x, th / 2.0), v2(x, -th / 2.0)));
        if j < nc {
            x += colw[j];
        }
    }
    Mobject::group([lines.stroke(GRAY_B, 2.0), body])
}

/// Matrix with brackets; children: [entries (rows), left bracket, right bracket].
pub fn matrix(rows: &[Vec<&str>], size: f64) -> Mobject {
    let cells: Vec<Vec<Mobject>> = rows
        .iter()
        .map(|r| r.iter().map(|s| tex_sized(s, size, WHITE)).collect())
        .collect();
    mobject_matrix(cells, size)
}

/// Matrix of arbitrary mobjects with brackets (manim's `MobjectMatrix`);
/// children: [entries (rows), left bracket, right bracket].
pub fn mobject_matrix(cells: Vec<Vec<Mobject>>, size: f64) -> Mobject {
    let nr = cells.len();
    let nc = cells.iter().map(|r| r.len()).max().unwrap_or(0);
    let cw = cells
        .iter()
        .flatten()
        .map(|c| c.width())
        .fold(size * 0.6, f64::max);
    let ch = cells
        .iter()
        .flatten()
        .map(|c| c.height())
        .fold(size * 0.6, f64::max);
    let hgap = size * 1.2;
    let vgap = size * 0.6;
    let w = nc as f64 * cw + (nc as f64 - 1.0).max(0.0) * hgap;
    let h = nr as f64 * ch + (nr as f64 - 1.0).max(0.0) * vgap;
    let mut entries = Mobject::empty().with_tag("entries");
    for (i, r) in cells.into_iter().enumerate() {
        let mut row = Mobject::empty();
        for (j, c) in r.into_iter().enumerate() {
            let x = -w / 2.0 + cw / 2.0 + j as f64 * (cw + hgap);
            let y = h / 2.0 - ch / 2.0 - i as f64 * (ch + vgap);
            row.push(c.move_to(v2(x, y)));
        }
        entries.push(row);
    }
    let bh = h + size * 0.7;
    let bx = w / 2.0 + size * 0.45;
    let serif = size * 0.25;
    let lb = polyline(&[
        v2(-bx + serif, bh / 2.0),
        v2(-bx, bh / 2.0),
        v2(-bx, -bh / 2.0),
        v2(-bx + serif, -bh / 2.0),
    ])
    .stroke(WHITE, 3.0);
    let rb = polyline(&[
        v2(bx - serif, bh / 2.0),
        v2(bx, bh / 2.0),
        v2(bx, -bh / 2.0),
        v2(bx - serif, -bh / 2.0),
    ])
    .stroke(WHITE, 3.0);
    Mobject::group([entries, lb.with_tag("left"), rb.with_tag("right")])
}

/// Numeric matrix.
pub fn decimal_matrix(values: &[Vec<f64>], decimals: usize, size: f64) -> Mobject {
    let strs: Vec<Vec<String>> = values
        .iter()
        .map(|r| r.iter().map(|v| fmt_num(*v, decimals)).collect())
        .collect();
    let refs: Vec<Vec<&str>> = strs
        .iter()
        .map(|r| r.iter().map(|s| s.as_str()).collect())
        .collect();
    matrix(&refs, size)
}

/// Integer matrix (manim's `IntegerMatrix`).
pub fn integer_matrix(values: &[Vec<i64>], size: f64) -> Mobject {
    let strs: Vec<Vec<String>> = values
        .iter()
        .map(|r| r.iter().map(|v| v.to_string()).collect())
        .collect();
    let refs: Vec<Vec<&str>> = strs
        .iter()
        .map(|r| r.iter().map(|s| s.as_str()).collect())
        .collect();
    matrix(&refs, size)
}

/// Table of integers (manim's `IntegerTable`).
pub fn integer_table(values: &[Vec<i64>], size: f64) -> Mobject {
    table_from(
        values
            .iter()
            .map(|r| {
                r.iter()
                    .map(|v| tex_sized(&v.to_string(), size, WHITE))
                    .collect()
            })
            .collect(),
        size,
    )
}

/// Table of decimals with a fixed number of places (manim's `DecimalTable`).
pub fn decimal_table(values: &[Vec<f64>], decimals: usize, size: f64) -> Mobject {
    let fmt = DecimalNumber::new(decimals).size(size);
    table_from(
        values
            .iter()
            .map(|r| r.iter().map(|v| fmt.build(*v)).collect())
            .collect(),
        size,
    )
}

/// Table of arbitrary mobjects (manim's `MobjectTable`); same as [`table_from`].
pub fn mobject_table(cells: Vec<Vec<Mobject>>, size: f64) -> Mobject {
    table_from(cells, size)
}

// ----------------------------------------------------------------- numbers

/// A formatted number (manim's `DecimalNumber`/`Integer`). Build each frame with
/// `always_redraw` + a tracker to animate counting.
#[derive(Clone, Debug)]
pub struct DecimalNumber {
    pub decimals: usize,
    pub size: f64,
    pub color: Color,
    pub prefix: String,
    pub suffix: String,
    pub show_sign: bool,
    pub group_thousands: bool,
}

impl Default for DecimalNumber {
    fn default() -> Self {
        Self {
            decimals: 2,
            size: DEFAULT_FONT_SIZE,
            color: WHITE,
            prefix: String::new(),
            suffix: String::new(),
            show_sign: false,
            group_thousands: false,
        }
    }
}

impl DecimalNumber {
    pub fn new(decimals: usize) -> Self {
        Self {
            decimals,
            ..Default::default()
        }
    }
    /// Zero decimal places (manim's `Integer`).
    pub fn integer() -> Self {
        Self::new(0)
    }
    pub fn size(mut self, s: f64) -> Self {
        self.size = s;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn unit(mut self, suffix: &str) -> Self {
        self.suffix = suffix.into();
        self
    }
    pub fn prefix(mut self, p: &str) -> Self {
        self.prefix = p.into();
        self
    }
    pub fn thousands(mut self) -> Self {
        self.group_thousands = true;
        self
    }
    pub fn format(&self, v: f64) -> String {
        let mut s = format!("{:.*}", self.decimals, v);
        if self.group_thousands {
            let (int, frac) = match s.find('.') {
                Some(i) => (s[..i].to_string(), s[i..].to_string()),
                None => (s.clone(), String::new()),
            };
            let neg = int.starts_with('-');
            let digits: Vec<char> = int.trim_start_matches('-').chars().collect();
            let mut out = String::new();
            for (i, c) in digits.iter().enumerate() {
                if i > 0 && (digits.len() - i).is_multiple_of(3) {
                    out.push_str("{,}");
                }
                out.push(*c);
            }
            s = format!("{}{}{}", if neg { "-" } else { "" }, out, frac);
        }
        if self.show_sign && v >= 0.0 {
            s = format!("+{s}");
        }
        format!("{}{}{}", self.prefix, s, self.suffix)
    }
    pub fn build(&self, v: f64) -> Mobject {
        MathTex::new(&self.format(v))
            .size(self.size)
            .color(self.color)
            .build()
    }
}

/// An integer display (manim's `Integer`).
pub fn integer(v: i64) -> Mobject {
    DecimalNumber::integer().build(v as f64)
}

/// `label = value` display (manim's `Variable`).
pub fn variable(label: &str, value: f64, decimals: usize, color: Color) -> Mobject {
    let l = tex_sized(&format!("{label} ="), DEFAULT_FONT_SIZE, color);
    let v = DecimalNumber::new(decimals)
        .color(color)
        .build(value)
        .next_to(&l, RIGHT, 0.15);
    Mobject::group([l, v])
}

// ------------------------------------------------------------ labelled shapes

/// Dot with a label inside (manim's `LabeledDot`).
pub fn labeled_dot(label: &str, p: Vec2, color: Color) -> Mobject {
    let t = tex_sized(label, 0.4, color.contrasting());
    let r = (t.width().max(t.height()) / 2.0 + 0.08).max(0.18);
    Mobject::group([circle_at(p, r).fill(color, 1.0).no_stroke(), t.move_to(p)])
}

/// Line with a label at its midpoint on a background box (manim's `LabeledLine`).
pub fn labeled_line(a: Vec2, b: Vec2, label: &str, color: Color) -> Mobject {
    let l = line(a, b).stroke(color, 4.0);
    let t = tex_sized(label, 0.4, WHITE).move_to((a + b) / 2.0);
    let bg = background_rectangle(&t, BACKGROUND, 0.9, 0.06);
    Mobject::group([l, bg, t])
}

pub fn labeled_arrow(a: Vec2, b: Vec2, label: &str, color: Color) -> Mobject {
    let l = arrow(a, b).color(color);
    let t = tex_sized(label, 0.4, WHITE).move_to((a + b) / 2.0);
    let bg = background_rectangle(&t, BACKGROUND, 0.9, 0.06);
    Mobject::group([l, bg, t])
}

/// Brace with a TeX label (manim's `BraceLabel`).
pub fn brace_label(target: &Mobject, label: &str, dir: Vec2, color: Color) -> Mobject {
    let b = brace(target, dir, 0.1).color(color);
    let t = tex_sized(label, 0.5, color).next_to(&b, dir, 0.12);
    Mobject::group([b, t])
}

/// Brace with a plain text label (manim's `BraceText`).
pub fn brace_text(target: &Mobject, label: &str, dir: Vec2, color: Color) -> Mobject {
    let b = brace(target, dir, 0.1).color(color);
    let t = text_sized(label, 0.4, color).next_to(&b, dir, 0.12);
    Mobject::group([b, t])
}

// ------------------------------------------------------------ network graphs

/// Layout algorithms for [`network`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Circular,
    Spring,
    Tree,
    Grid,
    Shell,
}

/// Compute node positions for a graph.
pub fn graph_layout(n: usize, edges: &[(usize, usize)], layout: Layout, scale: f64) -> Vec<Vec2> {
    match layout {
        Layout::Circular | Layout::Shell => (0..n)
            .map(|i| Vec2::polar(scale, PI / 2.0 - TAU * i as f64 / n.max(1) as f64))
            .collect(),
        Layout::Grid => {
            let cols = (n as f64).sqrt().ceil().max(1.0) as usize;
            let rows = n.div_ceil(cols);
            (0..n)
                .map(|i| {
                    let (r, c) = (i / cols, i % cols);
                    let fx = if cols > 1 {
                        c as f64 / (cols - 1) as f64
                    } else {
                        0.5
                    };
                    let fy = if rows > 1 {
                        r as f64 / (rows - 1) as f64
                    } else {
                        0.5
                    };
                    v2((fx - 0.5) * 2.0 * scale, (0.5 - fy) * 2.0 * scale)
                })
                .collect()
        }
        Layout::Tree => {
            // BFS levels from node 0
            let mut level = vec![usize::MAX; n];
            if n > 0 {
                level[0] = 0;
            }
            let mut queue = std::collections::VecDeque::from([0usize]);
            let mut order = Vec::new();
            while let Some(u) = queue.pop_front() {
                order.push(u);
                for &(a, b) in edges {
                    let v = if a == u {
                        b
                    } else if b == u {
                        a
                    } else {
                        continue;
                    };
                    if v < n && level[v] == usize::MAX {
                        level[v] = level[u] + 1;
                        queue.push_back(v);
                    }
                }
            }
            for l in level.iter_mut() {
                if *l == usize::MAX {
                    *l = 0;
                }
            }
            let depth = level.iter().copied().max().unwrap_or(0);
            let mut pos = vec![Vec2::ZERO; n];
            for d in 0..=depth {
                let nodes: Vec<usize> = order.iter().copied().filter(|&u| level[u] == d).collect();
                let k = nodes.len();
                for (i, u) in nodes.into_iter().enumerate() {
                    let x = if k > 1 {
                        (i as f64 / (k - 1) as f64 - 0.5) * 2.0 * scale
                    } else {
                        0.0
                    };
                    let y = scale - d as f64 * (2.0 * scale / depth.max(1) as f64);
                    pos[u] = v2(x, y);
                }
            }
            pos
        }
        Layout::Spring => {
            // Fruchterman–Reingold, deterministic start on a circle
            let mut pos: Vec<Vec2> = (0..n)
                .map(|i| Vec2::polar(scale * 0.5, TAU * i as f64 / n.max(1) as f64 + 0.3))
                .collect();
            let k = scale * 1.6 / (n.max(1) as f64).sqrt();
            let mut temp = scale * 0.3;
            for _ in 0..300 {
                let mut disp = vec![Vec2::ZERO; n];
                for i in 0..n {
                    for j in 0..n {
                        if i != j {
                            let d = pos[i] - pos[j];
                            let dist = d.length().max(1e-3);
                            disp[i] += d / dist * (k * k / dist);
                        }
                    }
                }
                for &(a, b) in edges {
                    if a >= n || b >= n {
                        continue;
                    }
                    let d = pos[a] - pos[b];
                    let dist = d.length().max(1e-3);
                    let f = d / dist * (dist * dist / k);
                    disp[a] -= f;
                    disp[b] += f;
                }
                for i in 0..n {
                    let l = disp[i].length().max(1e-9);
                    pos[i] += disp[i] / l * l.min(temp);
                }
                temp *= 0.985;
            }
            // normalise into [-scale, scale]
            let mut b = Rect::point(pos.first().copied().unwrap_or_default());
            for p in &pos {
                b.include(*p);
            }
            let s = (2.0 * scale) / b.width().max(b.height()).max(1e-9);
            pos.iter().map(|p| (*p - b.center()) * s).collect()
        }
    }
}

/// Network graph (manim's `Graph` / `DiGraph`). Children: [edges group, vertices group].
pub fn network(
    labels: &[&str],
    edges: &[(usize, usize)],
    layout: Layout,
    directed: bool,
    scale: f64,
) -> Mobject {
    let n = labels.len();
    let pos = graph_layout(n, edges, layout, scale);
    let node_r = 0.28;
    let mut es = Mobject::empty().with_tag("edges");
    for &(a, b) in edges {
        if a >= n || b >= n {
            continue;
        }
        let (p, q) = (pos[a], pos[b]);
        let d = (q - p).normalize();
        let e = if directed {
            arrow_styled(
                p + d * node_r,
                q - d * node_r,
                ArrowStyle::default().color(GRAY_B).thickness(3.0),
            )
        } else {
            line(p + d * node_r, q - d * node_r).stroke(GRAY_B, 3.0)
        };
        es.push(e);
    }
    let mut vs = Mobject::empty().with_tag("vertices");
    for (i, l) in labels.iter().enumerate() {
        vs.push(labeled_dot(l, pos[i], BLUE_D).with_tag(l));
    }
    Mobject::group([es, vs])
}

/// Directed network graph (manim's `DiGraph`): [`network`] with arrow edges.
pub fn digraph(labels: &[&str], edges: &[(usize, usize)], layout: Layout, scale: f64) -> Mobject {
    network(labels, edges, layout, true, scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_mapping_roundtrip() {
        let ax = Axes::new((-2.0, 4.0, 1.0), (0.0, 10.0, 2.0))
            .size(6.0, 5.0)
            .at(v2(1.0, 0.0));
        let p = ax.c2p(1.0, 5.0);
        assert!((p - v2(1.0, 0.0)).length() < 1e-9);
        let c = ax.p2c(ax.c2p(3.0, 7.0));
        assert!((c - v2(3.0, 7.0)).length() < 1e-9);
        assert!(ax.build().leaf_count() > 10);
        let g = ax.plot(|x| x * x);
        assert!(g.length() > 1.0);
        let tan = ax.plot(|x| (x * 3.0).tan());
        assert!(
            tan.path.subpaths.len() > 1,
            "discontinuities split the curve"
        );
        assert!(ax.area(|x| x, 0.0, 2.0, None, BLUE, 0.5).leaf_count() == 1);
        assert_eq!(
            ax.riemann(|x| x, 0.0, 2.0, 4, "left", &[]).children.len(),
            4
        );
    }

    #[test]
    fn number_line_and_charts() {
        let nl = NumberLine::new(-3.0, 3.0, 1.0).length(6.0).numbers(true);
        assert!((nl.n2p(3.0) - v2(3.0, 0.0)).length() < 1e-9);
        assert!((nl.p2n(v2(-1.5, 0.0)) + 1.5).abs() < 1e-9);
        assert!(nl.build().leaf_count() > 7);
        let bc = bar_chart(&[1.0, 3.0, 2.0], &["a", "b", "c"], &[], 6.0, 4.0);
        assert_eq!(bc.children[1].children.len(), 3);
        assert_eq!(
            pie_chart(&[1.0, 2.0], &["x", "y"], &[], 2.0).children.len(),
            2
        );
        let t = math_table(&[vec!["x", "y"], vec!["1", "2"]], true, 0.4);
        assert_eq!(t.children[1].children.len(), 2);
        let m = matrix(&[vec!["1", "0"], vec!["0", "1"]], 0.5);
        assert_eq!(m.children[0].children.len(), 2);
        assert_eq!(
            DecimalNumber::new(1).thousands().format(1234567.0),
            "1{,}234{,}567.0"
        );
        let net = network(
            &["A", "B", "C"],
            &[(0, 1), (1, 2)],
            Layout::Spring,
            false,
            2.0,
        );
        assert_eq!(net.children[1].children.len(), 3);
        let ms = marching_squares(&|x, y| x * x + y * y - 1.0, -2.0, 2.0, -2.0, 2.0, 40);
        assert!(ms.len() > 20);
        let vf = vector_field(|p| p.perp(), (-2.0, 2.0, 1.0), (-2.0, 2.0, 1.0), 0.5, &[]);
        assert!(vf.children.len() >= 20);
    }

    #[test]
    fn numbers_tables_fields() {
        assert_eq!(DecimalNumber::integer().format(41.6), "42");
        assert!(integer(7).leaf_count() >= 1);
        let m = integer_matrix(&[vec![1, -2], vec![3, 4]], 0.5);
        assert_eq!(m.children[0].children.len(), 2);
        let mm = mobject_matrix(vec![vec![circle(0.2), square(0.3)]], 0.5);
        assert_eq!(mm.children[0].children[0].children.len(), 2);
        assert_eq!(
            integer_table(&[vec![1, 2, 3]], 0.4).children[1].children[0]
                .children
                .len(),
            3
        );
        assert_eq!(
            decimal_table(&[vec![1.5], vec![2.25]], 2, 0.4).children[1]
                .children
                .len(),
            2
        );
        assert_eq!(
            mobject_table(vec![vec![dot(Vec2::ZERO)]], 0.4)
                .children
                .len(),
            2
        );
        let ui = NumberLine::unit_interval(10.0);
        assert_eq!(ui.ticks().len(), 11);
        assert!((ui.n2p(1.0).x - 5.0).abs() < 1e-9);
        let g = digraph(&["a", "b"], &[(0, 1)], Layout::Circular, 1.5);
        assert_eq!(
            g.children[0].children[0].children.len(),
            2,
            "edge is an arrow"
        );
        let f0 = time_varying_vector_field(
            |p, t| p * t,
            0.0,
            (-1.0, 1.0, 1.0),
            (-1.0, 1.0, 1.0),
            1.0,
            &[],
        );
        let f1 = time_varying_vector_field(
            |p, t| p * t,
            1.0,
            (-1.0, 1.0, 1.0),
            (-1.0, 1.0, 1.0),
            1.0,
            &[],
        );
        assert!(f0.children.is_empty() && f1.children.len() == 8);
        let sl = stream_lines(|p| p.perp(), (-2.0, 2.0, 1.0), (-2.0, 2.0, 1.0), &[]);
        let asl = AnimatedStreamLines::new(&sl, 1.0, 3.0, 1.0);
        assert!(asl.frame(0.0).leaf_count() == 0);
        assert!(asl.frame(2.5).leaf_count() > 0);
        assert_eq!(asl.frame(2.5), asl.frame(2.5));
    }
}
