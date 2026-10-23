//! Generative art toolkit (phase A3).
//!
//! * [`field_image`] — evaluate any `Fn(Vec2) -> Color` over a rectangle in parallel
//!   and drop it in the scene as an image (a CPU "fragment shader").
//! * Flow fields, L-systems, phyllotaxis, circle packing, Truchet tiles, Voronoi
//!   mosaics, strange attractors (Clifford / de Jong) rendered as log-density
//!   images, marching-squares iso-contours, and Gray–Scott reaction–diffusion.
//!
//! Everything is deterministic (seeded) and returns ordinary mobjects, so results
//! can be animated with `partial`, morphs, staggers, etc.
use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::morph::voronoi_cells;
use crate::motion::palette;
use crate::path::{Path, Rect};
use crate::shapes::*;
use rayon::prelude::*;
use std::sync::Arc;

// ------------------------------------------------------------------ images

/// Rasterise `f` over `rect` at `px_w` pixels wide and return it as an image mobject
/// covering `rect`. Rows are evaluated in parallel.
pub fn field_image(rect: Rect, px_w: u32, f: impl Fn(Vec2) -> Color + Sync) -> Mobject {
    let w = px_w.max(1);
    let h = ((rect.height() / rect.width().max(1e-9)) * w as f64)
        .round()
        .max(1.0) as u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    data.par_chunks_mut((w * 4) as usize)
        .enumerate()
        .for_each(|(j, row)| {
            let y = rect.max.y - (j as f64 + 0.5) / h as f64 * rect.height();
            for i in 0..w as usize {
                let x = rect.min.x + (i as f64 + 0.5) / w as f64 * rect.width();
                let c = f(v2(x, y));
                let a = c.a.clamp(0.0, 1.0);
                // premultiplied RGBA
                row[i * 4] = (c.r.clamp(0.0, 1.0) * a * 255.0 + 0.5) as u8;
                row[i * 4 + 1] = (c.g.clamp(0.0, 1.0) * a * 255.0 + 0.5) as u8;
                row[i * 4 + 2] = (c.b.clamp(0.0, 1.0) * a * 255.0 + 0.5) as u8;
                row[i * 4 + 3] = (a * 255.0 + 0.5) as u8;
            }
        });
    let pm = tiny_skia::Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h).unwrap())
        .expect("valid pixmap");
    Mobject::image(Arc::new(pm), rect.width()).move_to(rect.center())
}

/// Log-density image of a point cloud (attractors, particle exposures).
pub fn density_image(
    points: &[Vec2],
    rect: Rect,
    px_w: u32,
    colors: &[Color],
    gamma: f64,
) -> Mobject {
    let w = px_w.max(1) as usize;
    let h = ((rect.height() / rect.width().max(1e-9)) * w as f64)
        .round()
        .max(1.0) as usize;
    let mut hist = vec![0u32; w * h];
    for p in points {
        let i = ((p.x - rect.min.x) / rect.width() * w as f64) as isize;
        let j = ((rect.max.y - p.y) / rect.height() * h as f64) as isize;
        if i >= 0 && j >= 0 && (i as usize) < w && (j as usize) < h {
            hist[j as usize * w + i as usize] += 1;
        }
    }
    let max = (*hist.iter().max().unwrap_or(&1)).max(1) as f64;
    let lmax = (1.0 + max).ln();
    let hist = Arc::new(hist);
    let cs: Vec<Color> = colors.to_vec();
    let hh = hist.clone();
    field_image(rect, px_w, move |p| {
        let i = (((p.x - rect.min.x) / rect.width()) * w as f64) as usize;
        let j = (((rect.max.y - p.y) / rect.height()) * h as f64) as usize;
        let n = hh[j.min(h - 1) * w + i.min(w - 1)] as f64;
        if n == 0.0 {
            return Color::rgba(0.0, 0.0, 0.0, 0.0);
        }
        let v = ((1.0 + n).ln() / lmax).powf(gamma);
        gradient_at(&cs, v).with_alpha(v.sqrt() as f32)
    })
}

/// Sample a multi-stop colour ramp at `t ∈ [0,1]` (OKLab mixing).
pub fn gradient_at(colors: &[Color], t: f64) -> Color {
    match colors.len() {
        0 => WHITE,
        1 => colors[0],
        n => {
            let x = t.clamp(0.0, 1.0) * (n - 1) as f64;
            let i = (x.floor() as usize).min(n - 2);
            colors[i].mix(colors[i + 1], (x - i as f64) as f32)
        }
    }
}

// ------------------------------------------------------------------ flow fields

/// Trace `n` streamlines through an angle field (radians) starting at seeded points.
pub fn flow_lines(
    rect: Rect,
    n: usize,
    steps: usize,
    step: f64,
    angle: impl Fn(Vec2) -> f64,
    seed: u64,
) -> Vec<Vec<Vec2>> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let mut p = v2(
                rng.range(rect.min.x, rect.max.x),
                rng.range(rect.min.y, rect.max.y),
            );
            let mut pts = vec![p];
            for _ in 0..steps {
                // midpoint (RK2) integration
                let a1 = angle(p);
                let mid = p + Vec2::from_angle(a1) * (step * 0.5);
                p += Vec2::from_angle(angle(mid)) * step;
                if !rect.contains(p) {
                    break;
                }
                pts.push(p);
            }
            pts
        })
        .filter(|l| l.len() > 2)
        .collect()
}

/// Flow-field drawing: streamlines coloured from a named palette, tapered by length.
pub fn flow_field(
    rect: Rect,
    n: usize,
    angle: impl Fn(Vec2) -> f64,
    pal: &str,
    seed: u64,
) -> Mobject {
    let lines = flow_lines(rect, n, 120, 0.04, angle, seed);
    let mut rng = Rng::new(seed ^ 0xF10);
    Mobject::group(lines.into_iter().map(|l| {
        let c = palette(pal, rng.next_f64());
        let w = 1.0 + 2.5 * rng.next_f64() as f32;
        Mobject::from_path_styled(
            Path::smooth_through(&l, false),
            crate::style::Style::stroked(c, w),
        )
        .opacity(0.85)
    }))
}

// ------------------------------------------------------------------ L-systems

/// A Lindenmayer system with turtle interpretation:
/// `F`/`G` draw forward, `f` moves, `+`/`-` turn, `[`/`]` push/pop, `|` turn 180°.
#[derive(Clone, Debug)]
pub struct LSystem {
    pub axiom: String,
    pub rules: Vec<(char, String)>,
    pub angle: f64,
}

impl LSystem {
    pub fn new(axiom: &str, rules: &[(char, &str)], angle_deg: f64) -> Self {
        Self {
            axiom: axiom.into(),
            rules: rules.iter().map(|(c, s)| (*c, s.to_string())).collect(),
            angle: angle_deg.to_radians(),
        }
    }
    /// Classic presets: "plant", "tree", "koch", "dragon", "sierpinski", "hilbert", "bush".
    pub fn preset(name: &str) -> Self {
        match name {
            "tree" => Self::new("F", &[('F', "FF+[+F-F-F]-[-F+F+F]")], 22.5),
            "koch" => Self::new("F--F--F", &[('F', "F+F--F+F")], 60.0),
            "dragon" => Self::new("FX", &[('X', "X+YF+"), ('Y', "-FX-Y")], 90.0),
            "sierpinski" => Self::new("F-G-G", &[('F', "F-G+F+G-F"), ('G', "GG")], 120.0),
            "hilbert" => Self::new("A", &[('A', "+BF-AFA-FB+"), ('B', "-AF+BFB+FA-")], 90.0),
            "bush" => Self::new("Y", &[('X', "X[-FFF][+FFF]FX"), ('Y', "YFX[+Y][-Y]")], 25.7),
            _ => Self::new("X", &[('X', "F+[[X]-X]-F[-FX]+X"), ('F', "FF")], 25.0),
        }
    }
    /// Rewrite `n` generations.
    pub fn expand(&self, n: usize) -> String {
        let mut s = self.axiom.clone();
        for _ in 0..n {
            let mut out = String::with_capacity(s.len() * 3);
            for ch in s.chars() {
                match self.rules.iter().find(|(c, _)| *c == ch) {
                    Some((_, r)) => out.push_str(r),
                    None => out.push(ch),
                }
            }
            s = out;
            if s.len() > 2_000_000 {
                break;
            }
        }
        s
    }
    /// Turtle-interpret `n` generations into polylines with their branch depth,
    /// normalised to fit inside a `size`×`size` box centred at the origin.
    pub fn polylines(&self, n: usize, heading_deg: f64, size: f64) -> Vec<(Vec<Vec2>, usize)> {
        let prog = self.expand(n);
        let mut lines: Vec<(Vec<Vec2>, usize)> = vec![];
        let mut cur = vec![Vec2::ZERO];
        let (mut p, mut a) = (Vec2::ZERO, heading_deg.to_radians());
        let mut stack: Vec<(Vec2, f64)> = vec![];
        for ch in prog.chars() {
            match ch {
                'F' | 'G' => {
                    p += Vec2::from_angle(a);
                    cur.push(p);
                }
                'f' => {
                    p += Vec2::from_angle(a);
                    if cur.len() > 1 {
                        lines.push((std::mem::take(&mut cur), stack.len()));
                    }
                    cur = vec![p];
                }
                '+' => a += self.angle,
                '-' => a -= self.angle,
                '|' => a += std::f64::consts::PI,
                '[' => {
                    stack.push((p, a));
                }
                ']' => {
                    if cur.len() > 1 {
                        lines.push((std::mem::take(&mut cur), stack.len()));
                    }
                    if let Some((q, b)) = stack.pop() {
                        p = q;
                        a = b;
                    }
                    cur = vec![p];
                }
                _ => {}
            }
        }
        if cur.len() > 1 {
            lines.push((cur, stack.len()));
        }
        // normalise
        let mut r = Rect::point(Vec2::ZERO);
        for (l, _) in &lines {
            for q in l {
                r.include(*q);
            }
        }
        let s = size / r.width().max(r.height()).max(1e-9);
        let c = r.center();
        for (l, _) in &mut lines {
            for q in l.iter_mut() {
                *q = (*q - c) * s;
            }
        }
        lines
    }
    /// Draw as a mobject: stroke width and colour vary with branch depth.
    pub fn mobject(
        &self,
        n: usize,
        heading_deg: f64,
        size: f64,
        trunk: Color,
        tip: Color,
    ) -> Mobject {
        let lines = self.polylines(n, heading_deg, size);
        let maxd = lines.iter().map(|(_, d)| *d).max().unwrap_or(0).max(1) as f64;
        Mobject::group(lines.into_iter().map(|(l, d)| {
            let k = d as f64 / maxd;
            Mobject::from_path_styled(
                Path::polyline(&l, false),
                crate::style::Style::stroked(
                    trunk.mix(tip, k as f32),
                    (3.0 * (1.0 - k) + 1.0) as f32,
                ),
            )
        }))
    }
}

// ------------------------------------------------------------------ phyllotaxis & packing

/// Vogel's sunflower model: `n` points at the golden angle, spacing `c`.
pub fn phyllotaxis(n: usize, c: f64) -> Vec<Vec2> {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    (0..n)
        .map(|i| Vec2::polar(c * (i as f64).sqrt(), i as f64 * golden))
        .collect()
}

/// Random-insertion circle packing inside `rect` (optionally constrained by `inside`).
/// Returns `(centre, radius)` sorted largest first.
pub fn circle_packing(
    rect: Rect,
    attempts: usize,
    r_min: f64,
    r_max: f64,
    inside: impl Fn(Vec2) -> bool,
    seed: u64,
) -> Vec<(Vec2, f64)> {
    let mut rng = Rng::new(seed);
    let mut out: Vec<(Vec2, f64)> = vec![];
    let pad = r_min * 0.15;
    for _ in 0..attempts {
        let p = v2(
            rng.range(rect.min.x, rect.max.x),
            rng.range(rect.min.y, rect.max.y),
        );
        if !inside(p) {
            continue;
        }
        // largest radius that fits (bounded by neighbours and the rect)
        let mut r = r_max
            .min(p.x - rect.min.x)
            .min(rect.max.x - p.x)
            .min(p.y - rect.min.y)
            .min(rect.max.y - p.y);
        for (q, rq) in &out {
            r = r.min(p.distance(*q) - rq - pad);
            if r < r_min {
                break;
            }
        }
        if r >= r_min {
            out.push((p, r));
        }
    }
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    out
}

// ------------------------------------------------------------------ tiles & mosaics

/// Smith-style Truchet tiling (quarter-circle arcs) over `rect`.
pub fn truchet(rect: Rect, cell: f64, color: Color, width: f32, seed: u64) -> Mobject {
    let mut rng = Rng::new(seed);
    let nx = (rect.width() / cell).ceil() as usize;
    let ny = (rect.height() / cell).ceil() as usize;
    let h = cell / 2.0;
    let mut kids = vec![];
    for j in 0..ny {
        for i in 0..nx {
            let o = rect.min + v2(i as f64 * cell, j as f64 * cell);
            let flip = rng.next_f64() < 0.5;
            let (c1, c2, a1, a2) = if flip {
                (o, o + v2(cell, cell), 0.0, std::f64::consts::PI)
            } else {
                (
                    o + v2(cell, 0.0),
                    o + v2(0.0, cell),
                    std::f64::consts::FRAC_PI_2,
                    -std::f64::consts::FRAC_PI_2,
                )
            };
            kids.push(Mobject::from_path_styled(
                Path::arc(c1, h, h, a1, std::f64::consts::FRAC_PI_2),
                crate::style::Style::stroked(color, width),
            ));
            kids.push(Mobject::from_path_styled(
                Path::arc(c2, h, h, a2, std::f64::consts::FRAC_PI_2),
                crate::style::Style::stroked(color, width),
            ));
        }
    }
    Mobject::group(kids)
}

/// Voronoi stained-glass mosaic: `n` jittered cells coloured from a palette.
pub fn voronoi_mosaic(rect: Rect, n: usize, pal: &str, seed: u64) -> Mobject {
    let mut rng = Rng::new(seed);
    let seeds: Vec<Vec2> = (0..n)
        .map(|_| {
            v2(
                rng.range(rect.min.x, rect.max.x),
                rng.range(rect.min.y, rect.max.y),
            )
        })
        .collect();
    let cells = voronoi_cells(&seeds, rect);
    Mobject::group(cells.into_iter().filter(|c| c.len() >= 3).map(|c| {
        let col = palette(pal, rng.next_f64());
        polygon(&c).fill(col, 0.9).stroke(rgb8(15, 15, 20), 3.0)
    }))
}

// ------------------------------------------------------------------ attractors

/// Iterate the Clifford attractor `x' = sin(a y) + c cos(a x)`, `y' = sin(b x) + d cos(b y)`.
pub fn clifford(a: f64, b: f64, c: f64, d: f64, n: usize) -> Vec<Vec2> {
    let mut p = v2(0.1, 0.1);
    (0..n)
        .map(|_| {
            p = v2(
                (a * p.y).sin() + c * (a * p.x).cos(),
                (b * p.x).sin() + d * (b * p.y).cos(),
            );
            p
        })
        .collect()
}

/// Peter de Jong attractor `x' = sin(a y) − cos(b x)`, `y' = sin(c x) − cos(d y)`.
pub fn de_jong(a: f64, b: f64, c: f64, d: f64, n: usize) -> Vec<Vec2> {
    let mut p = v2(0.1, 0.1);
    (0..n)
        .map(|_| {
            p = v2(
                (a * p.y).sin() - (b * p.x).cos(),
                (c * p.x).sin() - (d * p.y).cos(),
            );
            p
        })
        .collect()
}

// ------------------------------------------------------------------ iso-contours

/// Marching squares: line segments of `f(x, y) = level` over an `nx × ny` grid.
pub fn iso_segments(
    f: &dyn Fn(Vec2) -> f64,
    rect: Rect,
    nx: usize,
    ny: usize,
    level: f64,
) -> Vec<(Vec2, Vec2)> {
    let (dx, dy) = (rect.width() / nx as f64, rect.height() / ny as f64);
    let grid: Vec<Vec<f64>> = (0..=ny)
        .map(|j| {
            (0..=nx)
                .map(|i| f(rect.min + v2(i as f64 * dx, j as f64 * dy)) - level)
                .collect()
        })
        .collect();
    let mut segs = vec![];
    for j in 0..ny {
        for i in 0..nx {
            let p = |a: usize, b: usize| rect.min + v2((i + a) as f64 * dx, (j + b) as f64 * dy);
            let v = [
                grid[j][i],
                grid[j][i + 1],
                grid[j + 1][i + 1],
                grid[j + 1][i],
            ];
            let c = [p(0, 0), p(1, 0), p(1, 1), p(0, 1)];
            let mut pts = vec![];
            for e in 0..4 {
                let (a, b) = (e, (e + 1) % 4);
                if (v[a] < 0.0) != (v[b] < 0.0) {
                    let t = v[a] / (v[a] - v[b]);
                    pts.push(c[a].lerp(c[b], t));
                }
            }
            match pts.len() {
                2 => segs.push((pts[0], pts[1])),
                4 => {
                    // saddle: disambiguate with the centre value
                    let centre = (v[0] + v[1] + v[2] + v[3]) / 4.0;
                    if (centre < 0.0) == (v[0] < 0.0) {
                        segs.push((pts[0], pts[1]));
                        segs.push((pts[2], pts[3]));
                    } else {
                        segs.push((pts[0], pts[3]));
                        segs.push((pts[1], pts[2]));
                    }
                }
                _ => {}
            }
        }
    }
    segs
}

/// Topographic map: iso-lines of `f` at `levels` coloured along a palette.
pub fn contour_map(
    f: &dyn Fn(Vec2) -> f64,
    rect: Rect,
    res: usize,
    levels: &[f64],
    pal: &str,
) -> Mobject {
    let ny = ((rect.height() / rect.width()) * res as f64).ceil() as usize;
    let n = levels.len().max(2) - 1;
    Mobject::group(levels.iter().enumerate().map(|(k, &lv)| {
        let mut p = Path::new();
        for (a, b) in iso_segments(f, rect, res, ny.max(1), lv) {
            p.move_to(a).line_to(b);
        }
        let c = palette(pal, k as f64 / n as f64);
        Mobject::from_path_styled(
            p,
            crate::style::Style::stroked(c, if k % 4 == 0 { 2.5 } else { 1.2 }),
        )
    }))
}

// ------------------------------------------------------------------ reaction–diffusion

/// Gray–Scott reaction–diffusion on a `w × h` torus; returns the `v` concentration
/// after `steps` iterations (coral / mitosis / maze patterns depending on feed/kill).
pub fn gray_scott(w: usize, h: usize, steps: usize, feed: f64, kill: f64, seed: u64) -> Vec<f32> {
    let n = w * h;
    let mut u = vec![1.0f32; n];
    let mut v = vec![0.0f32; n];
    let mut rng = Rng::new(seed);
    for _ in 0..(n / 400).max(4) {
        let cx = (rng.next_u64() as usize) % w;
        let cy = (rng.next_u64() as usize) % h;
        for dy in 0..6 {
            for dx in 0..6 {
                let i = ((cy + dy) % h) * w + (cx + dx) % w;
                u[i] = 0.5;
                v[i] = 0.25;
            }
        }
    }
    let (du, dv, f, k) = (1.0f32, 0.5f32, feed as f32, kill as f32);
    let mut u2 = u.clone();
    let mut v2v = v.clone();
    for _ in 0..steps {
        u2.par_chunks_mut(w)
            .zip(v2v.par_chunks_mut(w))
            .enumerate()
            .for_each(|(y, (ru, rv))| {
                let ym = ((y + h - 1) % h) * w;
                let yp = ((y + 1) % h) * w;
                let yc = y * w;
                for x in 0..w {
                    let xm = (x + w - 1) % w;
                    let xp = (x + 1) % w;
                    let i = yc + x;
                    let lap = |a: &[f32]| {
                        a[yc + xm] * 0.2
                            + a[yc + xp] * 0.2
                            + a[ym + x] * 0.2
                            + a[yp + x] * 0.2
                            + a[ym + xm] * 0.05
                            + a[ym + xp] * 0.05
                            + a[yp + xm] * 0.05
                            + a[yp + xp] * 0.05
                            - a[i]
                    };
                    let (uu, vv) = (u[i], v[i]);
                    let uvv = uu * vv * vv;
                    ru[x] = (uu + du * lap(&u) - uvv + f * (1.0 - uu)).clamp(0.0, 1.0);
                    rv[x] = (vv + dv * lap(&v) + uvv - (f + k) * vv).clamp(0.0, 1.0);
                }
            });
        std::mem::swap(&mut u, &mut u2);
        std::mem::swap(&mut v, &mut v2v);
    }
    v
}

/// Render a scalar grid (row-major, top row first) through a colour ramp.
pub fn grid_image(values: &[f32], w: usize, h: usize, rect: Rect, colors: &[Color]) -> Mobject {
    let (lo, hi) = values
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    let span = (hi - lo).max(1e-6);
    let vals: Vec<f32> = values.to_vec();
    let cs = colors.to_vec();
    field_image(rect, (w * 2) as u32, move |p| {
        let fx = ((p.x - rect.min.x) / rect.width() * w as f64).clamp(0.0, w as f64 - 1.001);
        let fy = ((rect.max.y - p.y) / rect.height() * h as f64).clamp(0.0, h as f64 - 1.001);
        let (x0, y0) = (fx as usize, fy as usize);
        let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
        let g = |x: usize, y: usize| vals[y.min(h - 1) * w + x.min(w - 1)];
        let a = g(x0, y0) * (1.0 - tx) + g(x0 + 1, y0) * tx;
        let b = g(x0, y0 + 1) * (1.0 - tx) + g(x0 + 1, y0 + 1) * tx;
        gradient_at(&cs, ((a * (1.0 - ty) + b * ty - lo) / span) as f64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsystem_expansion() {
        let koch = LSystem::preset("koch");
        assert_eq!(koch.expand(1), "F+F--F+F--F+F--F+F--F+F--F+F");
        let lines = LSystem::preset("plant").polylines(4, 90.0, 4.0);
        assert!(lines.len() > 10);
        let mut r = Rect::point(lines[0].0[0]);
        for (l, _) in &lines {
            for p in l {
                r.include(*p);
            }
        }
        assert!((r.width().max(r.height()) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn phyllotaxis_and_packing() {
        let p = phyllotaxis(100, 0.1);
        assert_eq!(p.len(), 100);
        assert!((p[99].length() - 0.1 * 99f64.sqrt()).abs() < 1e-9);
        let rect = Rect::new(v2(-2.0, -2.0), v2(2.0, 2.0));
        let cs = circle_packing(rect, 3000, 0.05, 0.6, |_| true, 1);
        assert!(cs.len() > 30);
        for (i, (a, ra)) in cs.iter().enumerate() {
            for (b, rb) in &cs[i + 1..] {
                assert!(a.distance(*b) >= ra + rb - 1e-9, "circles overlap");
            }
        }
    }

    #[test]
    fn marching_squares_circle() {
        let f = |p: Vec2| p.length();
        let segs = iso_segments(&f, Rect::new(v2(-2.0, -2.0), v2(2.0, 2.0)), 40, 40, 1.0);
        assert!(segs.len() > 20);
        let len: f64 = segs.iter().map(|(a, b)| a.distance(*b)).sum();
        assert!((len - TAU).abs() < 0.1, "perimeter {len}");
        for (a, _) in &segs {
            assert!((a.length() - 1.0).abs() < 0.02);
        }
    }

    #[test]
    fn attractor_and_reaction_diffusion() {
        let pts = clifford(-1.4, 1.6, 1.0, 0.7, 1000);
        assert!(pts.iter().all(|p| p.x.abs() <= 2.0 && p.y.abs() <= 2.0));
        let v = gray_scott(32, 32, 50, 0.055, 0.062, 1);
        assert_eq!(v.len(), 1024);
        assert!(v.iter().any(|x| *x > 0.05));
        let img = field_image(Rect::new(v2(0.0, 0.0), v2(2.0, 1.0)), 20, |p| {
            Color::rgba(p.x as f32 / 2.0, 0.0, 0.0, 1.0)
        });
        let im = img.image.as_ref().unwrap();
        assert_eq!((im.pixmap.width(), im.pixmap.height()), (20, 10));
        assert!((img.width() - 2.0).abs() < 1e-9);
    }
}
