//! Phase S1 — math visuals in the 3Blue1Brown / Desmos / GeoGebra tradition:
//! Fourier epicycles, linear-map views with eigenvectors, Taylor partial sums,
//! unit-circle trigonometry, slope fields, domain colouring and number-theory toys
//! (Ulam spiral, modular times tables, Pascal's triangle mod p, Collatz trees).
//!
//! Everything is a pure function of its arguments, so it is safe to call from
//! `always_redraw` and render in parallel.

use crate::color::*;
use crate::generative::field_image;
use crate::graph::Axes;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::morph::{leaf_rings, resample_polyline};
use crate::path::Rect;
use crate::shapes::*;
use crate::text::text_sized;
use std::f64::consts::{PI, TAU};
use std::ops::{Add, Div, Mul, Neg, Sub};

// ───────────────────────────── complex numbers ─────────────────────────────

/// A small complex number type for domain colouring and Fourier work.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Cx {
    pub re: f64,
    pub im: f64,
}

pub const fn cx(re: f64, im: f64) -> Cx {
    Cx { re, im }
}

impl Cx {
    pub fn from_polar(r: f64, theta: f64) -> Cx {
        cx(r * theta.cos(), r * theta.sin())
    }
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
    pub fn conj(self) -> Cx {
        cx(self.re, -self.im)
    }
    pub fn exp(self) -> Cx {
        Cx::from_polar(self.re.exp(), self.im)
    }
    pub fn ln(self) -> Cx {
        cx(self.abs().ln(), self.arg())
    }
    pub fn sin(self) -> Cx {
        cx(
            self.re.sin() * self.im.cosh(),
            self.re.cos() * self.im.sinh(),
        )
    }
    pub fn cos(self) -> Cx {
        cx(
            self.re.cos() * self.im.cosh(),
            -self.re.sin() * self.im.sinh(),
        )
    }
    pub fn powi(self, n: i32) -> Cx {
        Cx::from_polar(self.abs().powi(n), self.arg() * n as f64)
    }
    pub fn powf(self, p: f64) -> Cx {
        if self.abs() == 0.0 {
            return Cx::default();
        }
        Cx::from_polar(self.abs().powf(p), self.arg() * p)
    }
    pub fn to_vec(self) -> Vec2 {
        v2(self.re, self.im)
    }
}
impl From<Vec2> for Cx {
    fn from(v: Vec2) -> Cx {
        cx(v.x, v.y)
    }
}
impl From<f64> for Cx {
    fn from(v: f64) -> Cx {
        cx(v, 0.0)
    }
}
impl Add for Cx {
    type Output = Cx;
    fn add(self, o: Cx) -> Cx {
        cx(self.re + o.re, self.im + o.im)
    }
}
impl Sub for Cx {
    type Output = Cx;
    fn sub(self, o: Cx) -> Cx {
        cx(self.re - o.re, self.im - o.im)
    }
}
impl Mul for Cx {
    type Output = Cx;
    fn mul(self, o: Cx) -> Cx {
        cx(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
}
impl Mul<f64> for Cx {
    type Output = Cx;
    fn mul(self, k: f64) -> Cx {
        cx(self.re * k, self.im * k)
    }
}
impl Div for Cx {
    type Output = Cx;
    fn div(self, o: Cx) -> Cx {
        let d = o.re * o.re + o.im * o.im;
        if d == 0.0 {
            return cx(f64::INFINITY, f64::INFINITY);
        }
        cx(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
}
impl Neg for Cx {
    type Output = Cx;
    fn neg(self) -> Cx {
        cx(-self.re, -self.im)
    }
}

// ───────────────────────────── Fourier epicycles ─────────────────────────────

/// One rotating circle of a Fourier series: `amp · e^{i(2π·freq·t + phase)}`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Epicycle {
    pub freq: i32,
    pub amp: f64,
    pub phase: f64,
}

/// Discrete Fourier transform of a closed curve sampled at equal parameter steps.
/// Returns every frequency in `-N/2..N/2`, the constant term first, then by
/// decreasing amplitude (the order epicycles are drawn in).
pub fn dft(points: &[Vec2]) -> Vec<Epicycle> {
    let n = points.len();
    if n == 0 {
        return vec![];
    }
    let lo = -(n as i32) / 2;
    let mut out: Vec<Epicycle> = (lo..lo + n as i32)
        .map(|k| {
            let mut c = Cx::default();
            for (j, p) in points.iter().enumerate() {
                let a = -TAU * k as f64 * j as f64 / n as f64;
                c = c + Cx::from(*p) * Cx::from_polar(1.0, a);
            }
            let c = c * (1.0 / n as f64);
            Epicycle {
                freq: k,
                amp: c.abs(),
                phase: c.arg(),
            }
        })
        .collect();
    out.sort_by(|a, b| {
        (a.freq != 0).cmp(&(b.freq != 0)).then(
            b.amp
                .partial_cmp(&a.amp)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    out
}

/// The `terms` most important epicycles of a closed curve (constant term included).
pub fn fourier_series(points: &[Vec2], terms: usize) -> Vec<Epicycle> {
    let mut c = dft(points);
    c.truncate(terms.max(1));
    c
}

/// Sample a shape's outline (all of its rings, joined) at `n` equally spaced points.
pub fn outline_points(m: &Mobject, n: usize) -> Vec<Vec2> {
    let joined: Vec<Vec2> = m
        .leaves()
        .iter()
        .flat_map(|l| leaf_rings(l, 12))
        .flatten()
        .collect();
    resample_polyline(&joined, n.max(3), true)
}

/// Tip of the epicycle chain at normalised time `t` (one full turn per unit).
pub fn epicycle_point(cs: &[Epicycle], t: f64) -> Vec2 {
    cs.iter()
        .map(|e| Vec2::polar(e.amp, TAU * e.freq as f64 * t + e.phase))
        .fold(Vec2::ZERO, |a, b| a + b)
}

/// The rotating circles and arms at time `t`, offset by `center`.
pub fn epicycles(cs: &[Epicycle], t: f64, center: Vec2, color: Color) -> Mobject {
    let mut p = center;
    let mut kids = vec![];
    for (i, e) in cs.iter().enumerate() {
        let q = p + Vec2::polar(e.amp, TAU * e.freq as f64 * t + e.phase);
        if i > 0 || e.freq != 0 {
            let fade = (1.0 - i as f32 / cs.len().max(1) as f32 * 0.7).max(0.25);
            if e.amp > 0.02 {
                kids.push(
                    circle(e.amp)
                        .move_to(p)
                        .stroke(color.with_alpha(0.35 * fade), 1.5),
                );
            }
            kids.push(line(p, q).stroke(WHITE.with_alpha(0.8 * fade), 2.0));
        }
        p = q;
    }
    kids.push(dot_r(p, 0.05, YELLOW));
    group(kids)
}

/// The curve drawn so far by the epicycles, `t` in `[0, 1]`.
pub fn epicycle_trace(cs: &[Epicycle], t: f64, center: Vec2, color: Color, width: f32) -> Mobject {
    let steps = ((t.clamp(0.0, 1.0) * 600.0) as usize).max(2);
    let pts: Vec<Vec2> = (0..=steps)
        .map(|k| center + epicycle_point(cs, t * k as f64 / steps as f64))
        .collect();
    polyline(&pts).stroke(color, width)
}

// ───────────────────────────── linear algebra ─────────────────────────────

pub type Mat2 = [[f64; 2]; 2];

pub fn mat_apply(m: Mat2, v: Vec2) -> Vec2 {
    v2(m[0][0] * v.x + m[0][1] * v.y, m[1][0] * v.x + m[1][1] * v.y)
}
pub fn mat_det(m: Mat2) -> f64 {
    m[0][0] * m[1][1] - m[0][1] * m[1][0]
}
/// `(1 − t)·I + t·M`, the straight-line path manim uses for `ApplyMatrix`.
pub fn mat_lerp(m: Mat2, t: f64) -> Mat2 {
    [
        [lerp(1.0, m[0][0], t), lerp(0.0, m[0][1], t)],
        [lerp(0.0, m[1][0], t), lerp(1.0, m[1][1], t)],
    ]
}

/// Real eigenpairs `(λ, unit eigenvector)` of a 2×2 matrix (empty if complex).
pub fn eigen2(m: Mat2) -> Vec<(f64, Vec2)> {
    let (a, b, c, d) = (m[0][0], m[0][1], m[1][0], m[1][1]);
    let tr = a + d;
    let disc = tr * tr / 4.0 - (a * d - b * c);
    if disc < -1e-12 {
        return vec![];
    }
    let s = disc.max(0.0).sqrt();
    let mut out = vec![];
    for l in [tr / 2.0 + s, tr / 2.0 - s] {
        let v = if b.abs() > 1e-12 {
            v2(b, l - a)
        } else if c.abs() > 1e-12 {
            v2(l - d, c)
        } else if (l - a).abs() < 1e-12 {
            v2(1.0, 0.0)
        } else {
            v2(0.0, 1.0)
        };
        let v = v.normalize();
        if !out
            .iter()
            .any(|(_, u): &(f64, Vec2)| u.cross(v).abs() < 1e-9)
        {
            out.push((l, v));
        }
    }
    out
}

/// Clip segment `a→b` to the square `[-r, r]²` (Liang–Barsky).
pub fn clip_segment(a: Vec2, b: Vec2, r: f64) -> Option<(Vec2, Vec2)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-d.x, a.x + r),
        (d.x, r - a.x),
        (-d.y, a.y + r),
        (d.y, r - a.y),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 < t1).then(|| (a + d * t0, a + d * t1))
}

/// 3Blue1Brown-style view of the map `M` at progress `t`: the transformed grid over a
/// faint fixed one, î (green) and ĵ (red), the unit square whose area is `det`, and
/// (optionally) eigenvector spans, which only stretch.
pub fn linear_map_view(m: Mat2, t: f64, range: f64, show_eigen: bool) -> Mobject {
    let mt = mat_lerp(m, t);
    let mut kids = vec![];
    let n = range.ceil() as i32;
    for k in -n..=n {
        let k = k as f64;
        if k.abs() > range {
            continue;
        }
        for (a, b) in [(v2(k, -range), v2(k, range)), (v2(-range, k), v2(range, k))] {
            kids.push(line(a, b).stroke(GRAY.with_alpha(0.25), 1.0));
        }
    }
    let far = range * 3.0;
    for k in -(3 * n)..=(3 * n) {
        let k = k as f64;
        let axis = k == 0.0;
        let c = if axis { WHITE } else { BLUE_C.with_alpha(0.75) };
        let w = if axis { 2.5 } else { 1.5 };
        for (a, b) in [(v2(k, -far), v2(k, far)), (v2(-far, k), v2(far, k))] {
            if let Some((p, q)) = clip_segment(mat_apply(mt, a), mat_apply(mt, b), range) {
                kids.push(line(p, q).stroke(c, w));
            }
        }
    }
    let sq = [v2(0.0, 0.0), v2(1.0, 0.0), v2(1.0, 1.0), v2(0.0, 1.0)].map(|p| mat_apply(mt, p));
    kids.push(polygon(&sq).fill(YELLOW, 0.35).stroke(YELLOW, 2.0));
    if show_eigen {
        for (l, v) in eigen2(m) {
            kids.push(
                line(v * -range, v * range)
                    .stroke(PURPLE_B.with_alpha(0.8), 2.0)
                    .dashed(0.15, 0.1),
            );
            let tip = mat_apply(mt, v * 1.5);
            kids.push(arrow(Vec2::ZERO, tip).color(PURPLE_A));
            kids.push(
                text_sized(&format!("λ = {l:.2}"), 0.3, PURPLE_A).move_to(tip + v.perp() * 0.35),
            );
        }
    }
    kids.push(arrow(Vec2::ZERO, mat_apply(mt, v2(1.0, 0.0))).color(GREEN));
    kids.push(arrow(Vec2::ZERO, mat_apply(mt, v2(0.0, 1.0))).color(RED));
    group(kids)
}

// ───────────────────────────── Taylor series ─────────────────────────────

/// Maclaurin coefficient `k` of a named function: sin, cos, exp, sinh, cosh,
/// ln1p (ln(1+x)), atan, geometric (1/(1−x)).
pub fn taylor_coeff(name: &str, k: usize) -> f64 {
    let fact = |n: usize| (1..=n).map(|i| i as f64).product::<f64>();
    let odd = k % 2 == 1;
    let sign = |j: usize| if j.is_multiple_of(2) { 1.0 } else { -1.0 };
    match name {
        "sin" if odd => sign((k - 1) / 2) / fact(k),
        "cos" if !odd => sign(k / 2) / fact(k),
        "exp" => 1.0 / fact(k),
        "sinh" if odd => 1.0 / fact(k),
        "cosh" if !odd => 1.0 / fact(k),
        "ln1p" if k > 0 => sign(k - 1) / k as f64,
        "atan" if odd => sign((k - 1) / 2) / k as f64,
        "geometric" => 1.0,
        _ => 0.0,
    }
}

/// Degree-`n` Maclaurin polynomial of `name` at `x`.
pub fn taylor_poly(name: &str, n: usize, x: f64) -> f64 {
    (0..=n)
        .rev()
        .fold(0.0, |acc, k| acc * x + taylor_coeff(name, k))
}

/// The exact function behind a [`taylor_coeff`] name.
pub fn taylor_target(name: &str, x: f64) -> f64 {
    match name {
        "sin" => x.sin(),
        "cos" => x.cos(),
        "exp" => x.exp(),
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "ln1p" => (1.0 + x).ln(),
        "atan" => x.atan(),
        _ => 1.0 / (1.0 - x),
    }
}

// ───────────────────────────── trigonometry ─────────────────────────────

/// Unit circle at `center` (radius `r`) with angle `theta`, its cos (blue) and sin
/// (red) projections and the sine wave traced to the right from `wave_x`.
pub fn unit_circle_trig(theta: f64, center: Vec2, r: f64, wave_x: f64, wave_scale: f64) -> Mobject {
    let p = center + Vec2::polar(r, theta);
    let foot = v2(p.x, center.y);
    let wave_end = wave_x + theta * wave_scale;
    let steps = ((theta.abs() * 30.0) as usize).max(2);
    let wave: Vec<Vec2> = (0..=steps)
        .map(|k| {
            let a = theta * k as f64 / steps as f64;
            v2(wave_x + a * wave_scale, center.y + r * a.sin())
        })
        .collect();
    group(vec![
        line(
            v2(center.x - r * 1.2, center.y),
            v2(wave_x + TAU * wave_scale + 0.3, center.y),
        )
        .stroke(GRAY, 1.5),
        line(
            v2(center.x, center.y - r * 1.2),
            v2(center.x, center.y + r * 1.2),
        )
        .stroke(GRAY, 1.5),
        circle(r).move_to(center).stroke(WHITE, 2.5),
        arc(r * 0.25, 0.0, theta).shift(center).stroke(YELLOW, 2.5),
        line(center, p).stroke(WHITE, 3.0),
        line(center, foot).stroke(BLUE, 5.0),
        line(foot, p).stroke(RED, 5.0),
        dashed_line(p, v2(wave_end, p.y)).stroke(GRAY_B, 1.5),
        polyline(&wave).stroke(RED_B, 3.5),
        dot_r(p, 0.07, YELLOW),
        dot_r(v2(wave_end, p.y), 0.07, RED),
    ])
}

// ───────────────────────────── differential equations ─────────────────────────────

/// Slope field of `dy/dx = f(x, y)` on `ax`: short segments every `step` units.
pub fn slope_field(ax: &Axes, f: &dyn Fn(f64, f64) -> f64, step: f64, color: Color) -> Mobject {
    let (x0, x1, _) = ax.x_range;
    let (y0, y1, _) = ax.y_range;
    let unit = ax.c2p(1.0, 0.0) - ax.c2p(0.0, 0.0);
    let seg = step
        * unit
            .x
            .abs()
            .min((ax.c2p(0.0, 1.0) - ax.c2p(0.0, 0.0)).y.abs())
        * 0.38;
    let mut kids = vec![];
    for x in arange(x0 + step / 2.0, x1, step) {
        for y in arange(y0 + step / 2.0, y1, step) {
            let s = f(x, y);
            let c = ax.c2p(x, y);
            let d = (ax.c2p(x + 1e-3, y + s * 1e-3) - c).normalize() * seg;
            let k = (s.atan() / PI + 0.5) as f32;
            kids.push(line(c - d, c + d).stroke(color.mix(WHITE, k * 0.6), 2.0));
        }
    }
    group(kids)
}

/// RK4 solution curve of `dy/dx = f(x, y)` through `(x0, y0)`, traced both ways
/// across the axes' x-range and cut where it leaves the y-range. Scene coordinates.
pub fn ode_curve(ax: &Axes, f: &dyn Fn(f64, f64) -> f64, x0: f64, y0: f64) -> Vec<Vec2> {
    let (xa, xb, _) = ax.x_range;
    let (ya, yb, _) = ax.y_range;
    let h = (xb - xa) / 400.0;
    let run = |dir: f64| {
        let (mut x, mut y) = (x0, y0);
        let mut pts = vec![];
        while x >= xa && x <= xb && y >= ya - 1.0 && y <= yb + 1.0 {
            pts.push(v2(x, y));
            let hh = h * dir;
            let k1 = f(x, y);
            let k2 = f(x + hh / 2.0, y + hh / 2.0 * k1);
            let k3 = f(x + hh / 2.0, y + hh / 2.0 * k2);
            let k4 = f(x + hh, y + hh * k3);
            y += hh / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4);
            x += hh;
        }
        pts
    };
    let mut back = run(-1.0);
    back.reverse();
    let fwd = run(1.0);
    back.extend(fwd.into_iter().skip(1));
    back.into_iter()
        .filter(|p| p.y >= ya && p.y <= yb)
        .map(|p| ax.c2p(p.x, p.y))
        .collect()
}

// ───────────────────────────── complex functions ─────────────────────────────

/// Domain colouring of `f` over `rect` (scene units = complex units): hue is the
/// argument, brightness bands mark |f| doubling, thin lines mark integer Re/Im of f.
pub fn domain_coloring(rect: Rect, px_w: u32, f: impl Fn(Cx) -> Cx + Sync) -> Mobject {
    field_image(rect, px_w, move |p| {
        let w = f(Cx::from(p));
        if !w.re.is_finite() || !w.im.is_finite() {
            return WHITE;
        }
        let hue = (w.arg().to_degrees() as f32).rem_euclid(360.0);
        let m = w.abs().max(1e-12).log2();
        let band = (m - m.floor()) as f32;
        let v = 0.55 + 0.45 * band;
        let grid = |x: f64| {
            let d = (x - x.round()).abs();
            smoothstep(0.03, 0.0, d) as f32
        };
        let g = grid(w.re).max(grid(w.im)) * 0.5;
        Color::hsv(hue, 0.85, v).mix(BLACK, g)
    })
}

// ───────────────────────────── number theory ─────────────────────────────

/// Sieve of Eratosthenes: `is_prime[k]` for `k ≤ n`.
pub fn sieve(n: usize) -> Vec<bool> {
    let mut p = vec![true; n + 1];
    p[0] = false;
    if n >= 1 {
        p[1] = false;
    }
    let mut i = 2;
    while i * i <= n {
        if p[i] {
            let mut j = i * i;
            while j <= n {
                p[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    p
}

/// Grid coordinate of `k ≥ 1` on the Ulam (square) spiral; 1 is at the origin.
pub fn ulam_coord(k: u64) -> (i64, i64) {
    if k <= 1 {
        return (0, 0);
    }
    let k = k as i64;
    let r = (((k as f64).sqrt() - 1.0) / 2.0).ceil() as i64;
    let side = 2 * r;
    let max = (2 * r + 1) * (2 * r + 1);
    let d = max - k;
    if d < side {
        (r - d, -r)
    } else if d < 2 * side {
        (-r, -r + (d - side))
    } else if d < 3 * side {
        (-r + (d - 2 * side), r)
    } else {
        (r, r - (d - 3 * side))
    }
}

/// The first `n` integers on the Ulam spiral; primes are filled squares.
pub fn ulam_spiral(n: usize, cell: f64, prime: Color, other: Color) -> Mobject {
    let is_p = sieve(n);
    let mut kids = vec![];
    for (k, &ip) in is_p.iter().enumerate().skip(1) {
        let (x, y) = ulam_coord(k as u64);
        let c = v2(x as f64, y as f64) * cell;
        if ip {
            kids.push(square(cell * 0.86).fill(prime, 1.0).no_stroke().move_to(c));
        } else if other.a > 0.0 {
            kids.push(dot_r(c, cell * 0.12, other));
        }
    }
    group(kids)
}

/// Modular times table: `n` points on a circle, chord from `i` to `i·k mod n`.
/// `k` may be fractional, which makes it animate smoothly (k = 2 is a cardioid).
pub fn times_table(n: usize, k: f64, radius: f64, colors: &[Color]) -> Mobject {
    let pt = |i: f64| Vec2::polar(radius, PI + TAU * i / n as f64);
    let mut kids = vec![circle(radius).stroke(GRAY_B, 1.5)];
    for i in 0..n {
        let j = (i as f64 * k).rem_euclid(n as f64);
        let c = color_gradient(colors, i as f64 / n as f64);
        kids.push(line(pt(i as f64), pt(j)).stroke(c.with_alpha(0.8), 1.4));
    }
    group(kids)
}

/// Pascal's triangle mod `p` (Sierpiński for p = 2): cells with non-zero residue are
/// coloured by residue.
pub fn pascal_mod(rows: usize, p: u64, cell: f64, colors: &[Color]) -> Mobject {
    let mut row = vec![1u64];
    let mut kids = vec![];
    let h = cell * 3f64.sqrt() / 2.0;
    for n in 0..rows {
        for (k, &v) in row.iter().enumerate() {
            if v != 0 {
                let x = (k as f64 - n as f64 / 2.0) * cell;
                let y = -(n as f64) * h + rows as f64 * h / 2.0;
                let c = color_gradient(colors, (v - 1) as f64 / (p.max(2) - 1).max(1) as f64);
                kids.push(
                    regular_polygon(6, cell * 0.55)
                        .rotate(PI / 6.0)
                        .fill(c, 1.0)
                        .no_stroke()
                        .move_to(v2(x, y)),
                );
            }
        }
        let mut next = vec![1u64; n + 2];
        for k in 1..=n {
            next[k] = (row[k - 1] + row[k]) % p;
        }
        row = next;
    }
    group(kids)
}

/// The Collatz sequence from `n` down to 1.
pub fn collatz(mut n: u64) -> Vec<u64> {
    let mut out = vec![n];
    while n > 1 {
        n = if n.is_multiple_of(2) {
            n / 2
        } else {
            3 * n + 1
        };
        out.push(n);
    }
    out
}

/// Collatz "coral": each start value's path is drawn from 1 outwards, turning by
/// `even_turn` on even steps and `odd_turn` on odd steps.
pub fn collatz_tree(
    starts: &[u64],
    seg: f64,
    even_turn: f64,
    odd_turn: f64,
    colors: &[Color],
) -> Mobject {
    let mut kids = vec![];
    for (i, &s) in starts.iter().enumerate() {
        let mut seq = collatz(s);
        seq.reverse();
        let mut p = Vec2::ZERO;
        let mut a = PI / 2.0;
        let mut pts = vec![p];
        for v in seq.iter().skip(1) {
            a += if v.is_multiple_of(2) {
                even_turn
            } else {
                odd_turn
            };
            p += Vec2::from_angle(a) * seg;
            pts.push(p);
        }
        let c = color_gradient(colors, i as f64 / starts.len().max(1) as f64);
        kids.push(polyline(&pts).stroke(c.with_alpha(0.35), 1.2));
    }
    group(kids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dft_reconstructs_samples() {
        let pts: Vec<Vec2> = (0..32)
            .map(|k| {
                let a = TAU * k as f64 / 32.0;
                v2(
                    a.cos() * 2.0 + 0.3 * (3.0 * a).cos(),
                    a.sin() - 0.5 * (2.0 * a).sin(),
                )
            })
            .collect();
        let cs = dft(&pts);
        for (k, p) in pts.iter().enumerate() {
            let q = epicycle_point(&cs, k as f64 / 32.0);
            assert!(p.distance(q) < 1e-9, "{k}: {p:?} vs {q:?}");
        }
        assert_eq!(cs[0].freq, 0);
        assert_eq!(cs[1].freq, 1);
        assert!((cs[1].amp - 1.5).abs() < 1e-9);
    }

    #[test]
    fn eigen_and_det() {
        let m = [[3.0, 1.0], [0.0, 2.0]];
        let e = eigen2(m);
        assert_eq!(e.len(), 2);
        for (l, v) in e {
            assert!(mat_apply(m, v).distance(v * l) < 1e-9);
        }
        assert!(eigen2([[0.0, -1.0], [1.0, 0.0]]).is_empty());
        assert_eq!(mat_det(m), 6.0);
    }

    #[test]
    fn taylor_converges() {
        for name in ["sin", "cos", "exp", "atan", "ln1p"] {
            let x = 0.5;
            let err = (taylor_poly(name, 15, x) - taylor_target(name, x)).abs();
            assert!(err < 1e-5, "{name}: {err}");
        }
    }

    #[test]
    fn number_theory() {
        let p = sieve(30);
        let primes: Vec<usize> = (0..=30).filter(|&k| p[k]).collect();
        assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
        assert_eq!(ulam_coord(1), (0, 0));
        assert_eq!(ulam_coord(2), (1, 0));
        assert_eq!(ulam_coord(3), (1, 1));
        assert_eq!(ulam_coord(5), (-1, 1));
        assert_eq!(ulam_coord(9), (1, -1));
        assert_eq!(ulam_coord(10), (2, -1));
        // every cell used once
        let mut seen = std::collections::HashSet::new();
        for k in 1..=121 {
            assert!(seen.insert(ulam_coord(k)));
        }
        assert_eq!(collatz(6), vec![6, 3, 10, 5, 16, 8, 4, 2, 1]);
    }

    #[test]
    fn complex_ops() {
        let a = cx(1.0, 2.0);
        let b = cx(-0.5, 0.25);
        let q = (a * b) / b;
        assert!((q - a).abs() < 1e-12);
        assert!((cx(0.0, PI).exp() + cx(1.0, 0.0)).abs() < 1e-12);
    }
}
