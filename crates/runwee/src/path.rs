//! Cubic bezier paths: construction, measurement, partials and alignment for morphing.

use crate::math::*;

/// One cubic segment; the start point is the end of the previous segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cubic {
    pub c1: Vec2,
    pub c2: Vec2,
    pub p: Vec2,
}

impl Cubic {
    pub fn line(from: Vec2, to: Vec2) -> Self {
        Cubic {
            c1: from.lerp(to, 1.0 / 3.0),
            c2: from.lerp(to, 2.0 / 3.0),
            p: to,
        }
    }
    #[inline]
    pub fn eval(&self, p0: Vec2, t: f64) -> Vec2 {
        let mt = 1.0 - t;
        p0 * (mt * mt * mt)
            + self.c1 * (3.0 * mt * mt * t)
            + self.c2 * (3.0 * mt * t * t)
            + self.p * (t * t * t)
    }
    pub fn derivative(&self, p0: Vec2, t: f64) -> Vec2 {
        let mt = 1.0 - t;
        (self.c1 - p0) * (3.0 * mt * mt)
            + (self.c2 - self.c1) * (6.0 * mt * t)
            + (self.p - self.c2) * (3.0 * t * t)
    }
    /// Sub-curve between parameters `t0..t1`. Returns (new start point, cubic).
    pub fn sub(&self, p0: Vec2, t0: f64, t1: f64) -> (Vec2, Cubic) {
        // Split at t1, keep the left part; then split that at t0/t1.
        let (_, left) = split(p0, self, t1);
        if t1 <= 1e-12 {
            let s = p0;
            return (s, Cubic { c1: s, c2: s, p: s });
        }
        let u = (t0 / t1).clamp(0.0, 1.0);
        let (mid, right) = split_right(p0, &left, u);
        (mid, right)
    }
    pub fn transform(&self, a: &Affine) -> Cubic {
        Cubic {
            c1: a.apply(self.c1),
            c2: a.apply(self.c2),
            p: a.apply(self.p),
        }
    }
    pub fn lerp(&self, o: &Cubic, t: f64) -> Cubic {
        Cubic {
            c1: self.c1.lerp(o.c1, t),
            c2: self.c2.lerp(o.c2, t),
            p: self.p.lerp(o.p, t),
        }
    }
    /// Arc length via 8-point Gauss–Legendre quadrature of |B'(t)|.
    pub fn approx_length(&self, p0: Vec2) -> f64 {
        const X: [f64; 4] = [
            0.1834346424956498,
            0.5255324099163290,
            0.7966664774136267,
            0.9602898564975363,
        ];
        const W: [f64; 4] = [
            0.3626837833783620,
            0.3137066458778873,
            0.2223810344533745,
            0.1012285362903763,
        ];
        let mut sum = 0.0;
        for k in 0..4 {
            for sgn in [-1.0, 1.0] {
                let t = 0.5 + 0.5 * sgn * X[k];
                sum += W[k] * self.derivative(p0, t).length();
            }
        }
        sum * 0.5
    }
}

/// Split a cubic at `t`, returning the left piece as (start, cubic) — start is always p0.
fn split(p0: Vec2, c: &Cubic, t: f64) -> (Vec2, Cubic) {
    let p01 = p0.lerp(c.c1, t);
    let p12 = c.c1.lerp(c.c2, t);
    let p23 = c.c2.lerp(c.p, t);
    let p012 = p01.lerp(p12, t);
    let p123 = p12.lerp(p23, t);
    let m = p012.lerp(p123, t);
    (
        p0,
        Cubic {
            c1: p01,
            c2: p012,
            p: m,
        },
    )
}

/// Right piece of a split at `t`: returns (split point, cubic from split point to end).
fn split_right(p0: Vec2, c: &Cubic, t: f64) -> (Vec2, Cubic) {
    let p01 = p0.lerp(c.c1, t);
    let p12 = c.c1.lerp(c.c2, t);
    let p23 = c.c2.lerp(c.p, t);
    let p012 = p01.lerp(p12, t);
    let p123 = p12.lerp(p23, t);
    let m = p012.lerp(p123, t);
    (
        m,
        Cubic {
            c1: p123,
            c2: p23,
            p: c.p,
        },
    )
}

/// Axis aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }
    pub fn from_center(c: Vec2, w: f64, h: f64) -> Self {
        Self::new(c - v2(w / 2.0, h / 2.0), c + v2(w / 2.0, h / 2.0))
    }
    pub fn point(p: Vec2) -> Self {
        Self { min: p, max: p }
    }
    pub fn union(&self, o: &Rect) -> Rect {
        Rect::new(self.min.min(o.min), self.max.max(o.max))
    }
    pub fn include(&mut self, p: Vec2) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }
    /// Point on the box in direction `dir` (manim's critical point): each
    /// component -1 → min, 0 → centre, 1 → max.
    pub fn critical_point(&self, dir: Vec2) -> Vec2 {
        let c = self.center();
        let pick = |d: f64, lo: f64, mid: f64, hi: f64| {
            if d < -1e-9 {
                lo
            } else if d > 1e-9 {
                hi
            } else {
                mid
            }
        };
        v2(
            pick(dir.x, self.min.x, c.x, self.max.x),
            pick(dir.y, self.min.y, c.y, self.max.y),
        )
    }
    pub fn expand(&self, by: f64) -> Rect {
        Rect::new(self.min - v2(by, by), self.max + v2(by, by))
    }
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.min.x <= o.max.x
            && self.max.x >= o.min.x
            && self.min.y <= o.max.y
            && self.max.y >= o.min.y
    }
}

/// A connected run of cubic segments.
#[derive(Clone, Debug, PartialEq)]
pub struct SubPath {
    pub start: Vec2,
    pub segs: Vec<Cubic>,
    pub closed: bool,
}

impl SubPath {
    pub fn new(start: Vec2) -> Self {
        Self {
            start,
            segs: Vec::new(),
            closed: false,
        }
    }
    pub fn end(&self) -> Vec2 {
        self.segs.last().map(|s| s.p).unwrap_or(self.start)
    }
    /// Start point of segment `i`.
    #[inline]
    pub fn seg_start(&self, i: usize) -> Vec2 {
        if i == 0 {
            self.start
        } else {
            self.segs[i - 1].p
        }
    }
    pub fn is_degenerate(&self) -> bool {
        self.segs
            .iter()
            .all(|s| s.p.distance(self.start) < 1e-9 && s.c1.distance(self.start) < 1e-9)
    }
    pub fn seg_lengths(&self) -> Vec<f64> {
        (0..self.segs.len())
            .map(|i| self.segs[i].approx_length(self.seg_start(i)))
            .collect()
    }
    pub fn length(&self) -> f64 {
        self.seg_lengths().iter().sum()
    }
    pub fn transform(&mut self, a: &Affine) {
        self.start = a.apply(self.start);
        for s in &mut self.segs {
            *s = s.transform(a);
        }
    }
    pub fn map_points(&mut self, f: &dyn Fn(Vec2) -> Vec2) {
        self.start = f(self.start);
        for s in &mut self.segs {
            s.c1 = f(s.c1);
            s.c2 = f(s.c2);
            s.p = f(s.p);
        }
    }
    pub fn reversed(&self) -> SubPath {
        let mut out = SubPath::new(self.end());
        for i in (0..self.segs.len()).rev() {
            let s = &self.segs[i];
            out.segs.push(Cubic {
                c1: s.c2,
                c2: s.c1,
                p: self.seg_start(i),
            });
        }
        out.closed = self.closed;
        out
    }
    /// Ensure a closed subpath ends exactly at its start.
    pub fn close_explicitly(&mut self) {
        if self.closed && self.end().distance(self.start) > 1e-9 {
            let e = self.end();
            self.segs.push(Cubic::line(e, self.start));
        }
    }

    /// Point at parameter `t ∈ [0,1]` distributed uniformly over segments.
    pub fn point_at_param(&self, t: f64) -> Vec2 {
        if self.segs.is_empty() {
            return self.start;
        }
        let n = self.segs.len() as f64;
        let x = (t.clamp(0.0, 1.0) * n).min(n - 1e-9);
        let i = x.floor() as usize;
        self.segs[i].eval(self.seg_start(i), x - i as f64)
    }

    /// Split each segment so the subpath has exactly `n` segments (n ≥ current count).
    pub fn resample(&self, n: usize) -> SubPath {
        let m = self.segs.len();
        if m == n {
            return self.clone();
        }
        let mut out = SubPath::new(self.start);
        out.closed = self.closed;
        if m == 0 {
            for _ in 0..n {
                out.segs.push(Cubic {
                    c1: self.start,
                    c2: self.start,
                    p: self.start,
                });
            }
            return out;
        }
        if n < m {
            // Can't reduce exactly; keep as is (callers always ask for >=).
            return self.clone();
        }
        // Distribute the n pieces over the m segments as evenly as possible.
        let base = n / m;
        let extra = n % m;
        for i in 0..m {
            let k = base + usize::from(i < extra);
            let p0 = self.seg_start(i);
            let c = &self.segs[i];
            for j in 0..k {
                let t0 = j as f64 / k as f64;
                let t1 = (j + 1) as f64 / k as f64;
                let (_, piece) = c.sub(p0, t0, t1);
                out.segs.push(piece);
            }
        }
        out
    }

    /// Portion of the subpath between arc-length proportions `a..b`.
    pub fn partial(&self, a: f64, b: f64) -> SubPath {
        let a = a.clamp(0.0, 1.0);
        let b = b.clamp(0.0, 1.0);
        if self.segs.is_empty() || b <= a {
            let p = self.point_at_length_prop(a);
            let mut s = SubPath::new(p);
            s.segs.push(Cubic { c1: p, c2: p, p });
            return s;
        }
        if a <= 0.0 && b >= 1.0 {
            return self.clone();
        }
        let lens = self.seg_lengths();
        let total: f64 = lens.iter().sum();
        if total < 1e-12 {
            return self.clone();
        }
        let (ia, ta) = locate(&lens, total * a);
        let (ib, tb) = locate(&lens, total * b);
        let mut out = SubPath::new(Vec2::ZERO);
        out.closed = false;
        for i in ia..=ib {
            let p0 = self.seg_start(i);
            let t0 = if i == ia { ta } else { 0.0 };
            let t1 = if i == ib { tb } else { 1.0 };
            let (st, piece) = self.segs[i].sub(p0, t0, t1);
            if i == ia {
                out.start = st;
            }
            out.segs.push(piece);
        }
        if a <= 0.0 && b >= 1.0 {
            out.closed = self.closed;
        }
        out
    }

    pub fn point_at_length_prop(&self, a: f64) -> Vec2 {
        if self.segs.is_empty() {
            return self.start;
        }
        let lens = self.seg_lengths();
        let total: f64 = lens.iter().sum();
        if total < 1e-12 {
            return self.start;
        }
        let (i, t) = locate(&lens, total * a.clamp(0.0, 1.0));
        self.segs[i].eval(self.seg_start(i), t)
    }

    pub fn tangent_at_length_prop(&self, a: f64) -> Vec2 {
        if self.segs.is_empty() {
            return RIGHT;
        }
        let lens = self.seg_lengths();
        let total: f64 = lens.iter().sum();
        let (i, t) = locate(&lens, total * a.clamp(0.0, 1.0));
        let d = self.segs[i].derivative(self.seg_start(i), t);
        if d.length() < 1e-12 {
            (self.segs[i].p - self.seg_start(i)).normalize()
        } else {
            d.normalize()
        }
    }

    pub fn bounds(&self) -> Rect {
        let mut r = Rect::point(self.start);
        for i in 0..self.segs.len() {
            let p0 = self.seg_start(i);
            let c = &self.segs[i];
            r.include(c.p);
            for t in cubic_extrema(p0.x, c.c1.x, c.c2.x, c.p.x)
                .into_iter()
                .chain(cubic_extrema(p0.y, c.c1.y, c.c2.y, c.p.y))
                .flatten()
            {
                r.include(c.eval(p0, t));
            }
        }
        r
    }
}

/// Locate arc-length `s` → (segment index, local parameter).
fn locate(lens: &[f64], s: f64) -> (usize, f64) {
    let mut acc = 0.0;
    for (i, &l) in lens.iter().enumerate() {
        if s <= acc + l || i == lens.len() - 1 {
            let t = if l < 1e-12 {
                0.0
            } else {
                ((s - acc) / l).clamp(0.0, 1.0)
            };
            return (i, t);
        }
        acc += l;
    }
    (lens.len() - 1, 1.0)
}

/// Parameters in (0,1) where a 1D cubic has zero derivative.
fn cubic_extrema(p0: f64, p1: f64, p2: f64, p3: f64) -> [Option<f64>; 2] {
    let a = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let b = 2.0 * (p0 - 2.0 * p1 + p2);
    let c = p1 - p0;
    let ok = |t: f64| if t > 0.0 && t < 1.0 { Some(t) } else { None };
    if a.abs() < 1e-12 {
        if b.abs() < 1e-12 {
            return [None, None];
        }
        return [ok(-c / b), None];
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return [None, None];
    }
    let sq = disc.sqrt();
    [ok((-b + sq) / (2.0 * a)), ok((-b - sq) / (2.0 * a))]
}

/// A collection of subpaths filled with one style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    pub subpaths: Vec<SubPath>,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_empty(&self) -> bool {
        self.subpaths.iter().all(|s| s.segs.is_empty())
    }
    pub fn num_segments(&self) -> usize {
        self.subpaths.iter().map(|s| s.segs.len()).sum()
    }

    // ---- builder API ----
    pub fn move_to(&mut self, p: Vec2) -> &mut Self {
        self.subpaths.push(SubPath::new(p));
        self
    }
    fn cur(&mut self) -> &mut SubPath {
        if self.subpaths.is_empty() {
            self.subpaths.push(SubPath::new(Vec2::ZERO));
        }
        self.subpaths.last_mut().unwrap()
    }
    pub fn line_to(&mut self, p: Vec2) -> &mut Self {
        let s = self.cur();
        let e = s.end();
        s.segs.push(Cubic::line(e, p));
        self
    }
    pub fn quad_to(&mut self, c: Vec2, p: Vec2) -> &mut Self {
        let s = self.cur();
        let e = s.end();
        s.segs.push(Cubic {
            c1: e + (c - e) * (2.0 / 3.0),
            c2: p + (c - p) * (2.0 / 3.0),
            p,
        });
        self
    }
    pub fn cubic_to(&mut self, c1: Vec2, c2: Vec2, p: Vec2) -> &mut Self {
        self.cur().segs.push(Cubic { c1, c2, p });
        self
    }
    /// Arc around `center` from the current point sweeping `angle` radians.
    pub fn arc_to_center(&mut self, center: Vec2, angle: f64) -> &mut Self {
        let s = self.cur();
        let e = s.end();
        let r = e.distance(center);
        let a0 = (e - center).angle();
        append_arc(&mut s.segs, center, r, r, a0, angle, 0.0);
        self
    }
    pub fn close(&mut self) -> &mut Self {
        let s = self.cur();
        s.closed = true;
        s.close_explicitly();
        self
    }

    // ---- constructors ----
    pub fn polyline(points: &[Vec2], closed: bool) -> Path {
        let mut p = Path::new();
        if points.is_empty() {
            return p;
        }
        p.move_to(points[0]);
        for &q in &points[1..] {
            p.line_to(q);
        }
        if closed {
            p.close();
        }
        p
    }

    /// Smooth curve through `points` (Catmull-Rom → bezier), like manim's `set_points_smoothly`.
    pub fn smooth_through(points: &[Vec2], closed: bool) -> Path {
        let n = points.len();
        let mut p = Path::new();
        if n == 0 {
            return p;
        }
        if n < 3 {
            return Path::polyline(points, closed);
        }
        p.move_to(points[0]);
        let get = |i: isize| -> Vec2 {
            if closed {
                points[i.rem_euclid(n as isize) as usize]
            } else {
                points[i.clamp(0, n as isize - 1) as usize]
            }
        };
        let segs = if closed { n } else { n - 1 };
        for i in 0..segs as isize {
            let p0 = get(i - 1);
            let p1 = get(i);
            let p2 = get(i + 1);
            let p3 = get(i + 2);
            let c1 = p1 + (p2 - p0) / 6.0;
            let c2 = p2 - (p3 - p1) / 6.0;
            p.cubic_to(c1, c2, p2);
        }
        if closed {
            p.subpaths.last_mut().unwrap().closed = true;
        }
        p
    }

    /// Elliptical arc as a standalone path.
    pub fn arc(center: Vec2, rx: f64, ry: f64, start: f64, angle: f64) -> Path {
        let mut sp = SubPath::new(center + v2(rx * start.cos(), ry * start.sin()));
        append_arc(&mut sp.segs, center, rx, ry, start, angle, 0.0);
        Path { subpaths: vec![sp] }
    }

    pub fn ellipse(center: Vec2, rx: f64, ry: f64) -> Path {
        let mut p = Path::arc(center, rx, ry, 0.0, TAU);
        p.subpaths[0].closed = true;
        p
    }

    // ---- geometry ----
    pub fn transform(&mut self, a: &Affine) {
        for s in &mut self.subpaths {
            s.transform(a);
        }
    }
    pub fn map_points(&mut self, f: &dyn Fn(Vec2) -> Vec2) {
        for s in &mut self.subpaths {
            s.map_points(f);
        }
    }
    pub fn bounds(&self) -> Option<Rect> {
        let mut it = self.subpaths.iter();
        let first = it.next()?.bounds();
        Some(it.fold(first, |r, s| r.union(&s.bounds())))
    }
    pub fn length(&self) -> f64 {
        self.subpaths.iter().map(|s| s.length()).sum()
    }
    pub fn reversed(&self) -> Path {
        Path {
            subpaths: self.subpaths.iter().rev().map(|s| s.reversed()).collect(),
        }
    }
    pub fn start(&self) -> Option<Vec2> {
        self.subpaths.first().map(|s| s.start)
    }
    pub fn end(&self) -> Option<Vec2> {
        self.subpaths.last().map(|s| s.end())
    }
    /// All anchor points (segment end points) including subpath starts.
    pub fn anchors(&self) -> Vec<Vec2> {
        let mut v = Vec::new();
        for s in &self.subpaths {
            v.push(s.start);
            v.extend(s.segs.iter().map(|c| c.p));
        }
        v
    }

    /// Portion of the whole path between arc-length proportions (over all subpaths).
    pub fn partial(&self, a: f64, b: f64) -> Path {
        if a <= 0.0 && b >= 1.0 {
            return self.clone();
        }
        let lens: Vec<f64> = self.subpaths.iter().map(|s| s.length()).collect();
        let total: f64 = lens.iter().sum();
        if total < 1e-12 {
            return if b > a { self.clone() } else { Path::new() };
        }
        let (sa, sb) = (a * total, b * total);
        let mut out = Path::new();
        let mut acc = 0.0;
        for (s, &l) in self.subpaths.iter().zip(&lens) {
            let (s0, s1) = (acc, acc + l);
            acc = s1;
            if s1 < sa || s0 > sb || l < 1e-12 {
                continue;
            }
            let pa = ((sa - s0) / l).clamp(0.0, 1.0);
            let pb = ((sb - s0) / l).clamp(0.0, 1.0);
            if pb - pa < 1e-9 {
                continue;
            }
            out.subpaths.push(s.partial(pa, pb));
        }
        out
    }

    pub fn point_from_proportion(&self, a: f64) -> Vec2 {
        let lens: Vec<f64> = self.subpaths.iter().map(|s| s.length()).collect();
        let total: f64 = lens.iter().sum();
        if self.subpaths.is_empty() {
            return Vec2::ZERO;
        }
        let target = a.clamp(0.0, 1.0) * total;
        let mut acc = 0.0;
        for (s, &l) in self.subpaths.iter().zip(&lens) {
            if target <= acc + l {
                return s.point_at_length_prop(if l > 0.0 { (target - acc) / l } else { 0.0 });
            }
            acc += l;
        }
        self.subpaths.last().unwrap().end()
    }

    pub fn tangent_from_proportion(&self, a: f64) -> Vec2 {
        let lens: Vec<f64> = self.subpaths.iter().map(|s| s.length()).collect();
        let total: f64 = lens.iter().sum();
        let target = a.clamp(0.0, 1.0) * total;
        let mut acc = 0.0;
        for (s, &l) in self.subpaths.iter().zip(&lens) {
            if target <= acc + l {
                return s.tangent_at_length_prop(if l > 0.0 { (target - acc) / l } else { 0.0 });
            }
            acc += l;
        }
        self.subpaths
            .last()
            .map(|s| s.tangent_at_length_prop(1.0))
            .unwrap_or(RIGHT)
    }

    /// Interpolate two *aligned* paths (same subpath and segment counts).
    pub fn lerp(&self, o: &Path, t: f64) -> Path {
        Path {
            subpaths: self
                .subpaths
                .iter()
                .zip(&o.subpaths)
                .map(|(a, b)| SubPath {
                    start: a.start.lerp(b.start, t),
                    segs: a
                        .segs
                        .iter()
                        .zip(&b.segs)
                        .map(|(x, y)| x.lerp(y, t))
                        .collect(),
                    closed: if t < 0.5 { a.closed } else { b.closed },
                })
                .collect(),
        }
    }

    /// Interpolate aligned paths with a custom point interpolation (e.g. along arcs).
    pub fn lerp_with(&self, o: &Path, t: f64, f: &dyn Fn(Vec2, Vec2, f64) -> Vec2) -> Path {
        Path {
            subpaths: self
                .subpaths
                .iter()
                .zip(&o.subpaths)
                .map(|(a, b)| SubPath {
                    start: f(a.start, b.start, t),
                    segs: a
                        .segs
                        .iter()
                        .zip(&b.segs)
                        .map(|(x, y)| Cubic {
                            c1: f(x.c1, y.c1, t),
                            c2: f(x.c2, y.c2, t),
                            p: f(x.p, y.p, t),
                        })
                        .collect(),
                    closed: if t < 0.5 { a.closed } else { b.closed },
                })
                .collect(),
        }
    }

    /// Subdivide every segment into `k` pieces (smoother non-linear warps).
    pub fn subdivided(&self, k: usize) -> Path {
        Path {
            subpaths: self
                .subpaths
                .iter()
                .map(|s| s.resample(s.segs.len() * k.max(1)))
                .collect(),
        }
    }

    /// Make two paths structurally compatible for interpolation.
    pub fn align(a: &Path, b: &Path) -> (Path, Path) {
        let mut a = a.clone();
        let mut b = b.clone();
        for s in a.subpaths.iter_mut().chain(b.subpaths.iter_mut()) {
            s.close_explicitly();
        }
        let center_a = a.bounds().map(|r| r.center()).unwrap_or_default();
        let center_b = b.bounds().map(|r| r.center()).unwrap_or_default();
        if a.subpaths.is_empty() {
            a.subpaths.push(SubPath::new(center_b));
        }
        if b.subpaths.is_empty() {
            b.subpaths.push(SubPath::new(center_a));
        }
        let n = a.subpaths.len().max(b.subpaths.len());
        pad_subpaths(&mut a, n);
        pad_subpaths(&mut b, n);
        for (x, y) in a.subpaths.iter_mut().zip(b.subpaths.iter_mut()) {
            let m = x.segs.len().max(y.segs.len()).max(1);
            *x = x.resample(m);
            *y = y.resample(m);
        }
        (a, b)
    }
}

/// Pad with degenerate subpaths collapsed onto the end of the last real one.
fn pad_subpaths(p: &mut Path, n: usize) {
    while p.subpaths.len() < n {
        let last = p.subpaths.last().unwrap();
        let pt = last.end();
        let k = last.segs.len().max(1);
        let mut s = SubPath::new(pt);
        for _ in 0..k {
            s.segs.push(Cubic {
                c1: pt,
                c2: pt,
                p: pt,
            });
        }
        s.closed = last.closed;
        p.subpaths.push(s);
    }
}

/// Append cubic approximations of an elliptic arc (rotation `rot`) to `segs`.
pub fn append_arc(
    segs: &mut Vec<Cubic>,
    center: Vec2,
    rx: f64,
    ry: f64,
    start: f64,
    sweep: f64,
    rot: f64,
) {
    if sweep.abs() < 1e-12 {
        return;
    }
    let n = (sweep.abs() / (PI / 4.0)).ceil().max(1.0) as usize;
    let d = sweep / n as f64;
    let k = 4.0 / 3.0 * (d / 4.0).tan();
    let pt = |a: f64| v2(rx * a.cos(), ry * a.sin()).rotate(rot) + center;
    let tg = |a: f64| v2(-rx * a.sin(), ry * a.cos()).rotate(rot);
    for i in 0..n {
        let a0 = start + d * i as f64;
        let a1 = a0 + d;
        segs.push(Cubic {
            c1: pt(a0) + tg(a0) * k,
            c2: pt(a1) - tg(a1) * k,
            p: pt(a1),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_length_and_bounds() {
        let c = Path::ellipse(Vec2::ZERO, 1.0, 1.0);
        assert!((c.length() - TAU).abs() < 1e-3);
        let b = c.bounds().unwrap();
        assert!((b.width() - 2.0).abs() < 1e-3 && (b.height() - 2.0).abs() < 1e-3);
    }

    #[test]
    fn partial_line() {
        let p = Path::polyline(&[v2(0.0, 0.0), v2(2.0, 0.0), v2(2.0, 2.0)], false);
        let half = p.partial(0.0, 0.5);
        assert!((half.end().unwrap() - v2(2.0, 0.0)).length() < 1e-6);
        assert!((p.point_from_proportion(0.75) - v2(2.0, 1.0)).length() < 1e-6);
        let mid = p.partial(0.25, 0.75);
        assert!((mid.length() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn align_and_lerp() {
        let sq = Path::polyline(
            &[v2(-1.0, -1.0), v2(1.0, -1.0), v2(1.0, 1.0), v2(-1.0, 1.0)],
            true,
        );
        let c = Path::ellipse(Vec2::ZERO, 1.0, 1.0);
        let (a, b) = Path::align(&sq, &c);
        assert_eq!(a.num_segments(), b.num_segments());
        let m = a.lerp(&b, 0.5);
        assert_eq!(m.num_segments(), a.num_segments());
        // multi-subpath alignment
        let mut two = Path::ellipse(v2(3.0, 0.0), 0.5, 0.5);
        two.subpaths
            .extend(Path::ellipse(v2(-3.0, 0.0), 0.5, 0.5).subpaths);
        let (x, y) = Path::align(&sq, &two);
        assert_eq!(x.subpaths.len(), 2);
        assert_eq!(x.subpaths[1].segs.len(), y.subpaths[1].segs.len());
    }

    #[test]
    fn smooth_curve_passes_through_points() {
        let pts = [v2(0.0, 0.0), v2(1.0, 1.0), v2(2.0, 0.0), v2(3.0, 1.0)];
        let p = Path::smooth_through(&pts, false);
        assert_eq!(p.anchors(), pts.to_vec());
    }
}
