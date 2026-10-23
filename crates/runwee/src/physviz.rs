//! Phase S2 — physics labs in the PhET / VPython spirit: a ripple tank with
//! interference, Doppler wavefronts and Mach cones, a 2D optical ray tracer
//! (mirrors, thin lenses, refracting glass with dispersion and total internal
//! reflection), Kepler orbits with equal-area sweeps, a gravity-well "rubber sheet",
//! phase portraits, a chaos fan of double pendulums, Maxwell–Boltzmann histograms
//! and charged particles in E/B fields.

use crate::color::*;
use crate::generative::field_image;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::path::Rect;
use crate::shapes::*;
use std::f64::consts::{PI, TAU};

// ───────────────────────────── waves ─────────────────────────────

/// A point source for [`ripple_tank`]: position and phase offset (radians).
#[derive(Clone, Copy, Debug)]
pub struct WaveSource {
    pub pos: Vec2,
    pub phase: f64,
    pub amp: f64,
}

pub fn wave_source(pos: Vec2) -> WaveSource {
    WaveSource {
        pos,
        phase: 0.0,
        amp: 1.0,
    }
}

/// Superposed circular waves (∝ cos(kr − ωt + φ)/√r) at `p`. One period per unit `t`.
pub fn ripple_height(sources: &[WaveSource], wavelength: f64, t: f64, p: Vec2) -> f64 {
    let k = TAU / wavelength;
    sources
        .iter()
        .map(|s| {
            let r = p.distance(s.pos);
            s.amp * (k * r - TAU * t + s.phase).cos() / (1.0 + r).sqrt()
        })
        .sum()
}

/// Time-averaged intensity of the superposition at `p` (phasor sum, |Σ|²).
pub fn interference_intensity(sources: &[WaveSource], wavelength: f64, p: Vec2) -> f64 {
    let k = TAU / wavelength;
    let (mut re, mut im) = (0.0, 0.0);
    for s in sources {
        let r = p.distance(s.pos);
        let a = s.amp / (1.0 + r).sqrt();
        re += a * (k * r + s.phase).cos();
        im += a * (k * r + s.phase).sin();
    }
    re * re + im * im
}

/// Ripple tank image: crests bright, troughs dark, sources glowing.
pub fn ripple_tank(
    rect: Rect,
    px_w: u32,
    sources: Vec<WaveSource>,
    wavelength: f64,
    t: f64,
) -> Mobject {
    let n = sources.len().max(1) as f64;
    field_image(rect, px_w, move |p| {
        let h = ripple_height(&sources, wavelength, t, p) / n.sqrt();
        let v = (h * 1.4).clamp(-1.0, 1.0) * 0.5 + 0.5;
        let near = sources
            .iter()
            .map(|s| p.distance(s.pos))
            .fold(f64::INFINITY, f64::min);
        let base = color_gradient(
            &[rgb8(4, 12, 40), rgb8(20, 90, 170), rgb8(190, 235, 255)],
            v,
        );
        base.mix(WHITE, smoothstep(0.12, 0.0, near) as f32)
    })
}

/// Wavefronts from a moving source: a circle emitted every `period` from where the
/// source was, growing at `wave_speed`. Faster-than-wave sources draw a Mach cone.
pub fn doppler_wavefronts(
    source: impl Fn(f64) -> Vec2,
    wave_speed: f64,
    period: f64,
    t: f64,
    max_age: f64,
    color: Color,
) -> Mobject {
    let mut kids = vec![];
    let mut k = (t / period).floor();
    while k >= 0.0 {
        let te = k * period;
        let age = t - te;
        if age > max_age {
            break;
        }
        let a = (1.0 - age / max_age).powf(0.7) as f32;
        if age > 1e-6 {
            kids.push(
                circle(wave_speed * age)
                    .move_to(source(te))
                    .stroke(color.with_alpha(a), 2.0),
            );
        }
        k -= 1.0;
    }
    kids.push(dot_r(source(t), 0.09, YELLOW).glow(0.25, None));
    group(kids)
}

/// Observed frequency for a source moving at `vs` towards (+) / away (−) from a
/// stationary observer: f' = f · c / (c − vs).
pub fn doppler_shift(f: f64, c: f64, vs: f64) -> f64 {
    f * c / (c - vs)
}

// ───────────────────────────── geometric optics ─────────────────────────────

/// An element on the optical bench.
#[derive(Clone, Debug)]
pub enum Optic {
    /// Flat mirror from `a` to `b`.
    Mirror { a: Vec2, b: Vec2 },
    /// Ideal thin lens: centre, unit axis direction, half-height, focal length
    /// (negative = diverging).
    ThinLens {
        center: Vec2,
        axis: Vec2,
        half_height: f64,
        focal: f64,
    },
    /// Closed polygon of glass with index `n` (at 550 nm) and Cauchy dispersion `b`
    /// (in µm²): n(λ) = n + b·(1/λ² − 1/0.55²).
    Glass { poly: Vec<Vec2>, n: f64, b: f64 },
    /// Glass disc (a raindrop, a ball lens).
    Disc {
        center: Vec2,
        r: f64,
        n: f64,
        b: f64,
    },
    /// Absorbs rays (a screen or wall).
    Screen { a: Vec2, b: Vec2 },
}

/// Refractive index at `nm` for Cauchy-dispersive glass.
pub fn cauchy_index(n: f64, b: f64, nm: f64) -> f64 {
    let l = nm / 1000.0;
    n + b * (1.0 / (l * l) - 1.0 / (0.55 * 0.55))
}

/// Approximate visible colour of light of wavelength `nm` (380–780).
pub fn wavelength_color(nm: f64) -> Color {
    let (r, g, b) = match nm {
        x if x < 440.0 => (-(x - 440.0) / 60.0, 0.0, 1.0),
        x if x < 490.0 => (0.0, (x - 440.0) / 50.0, 1.0),
        x if x < 510.0 => (0.0, 1.0, -(x - 510.0) / 20.0),
        x if x < 580.0 => ((x - 510.0) / 70.0, 1.0, 0.0),
        x if x < 645.0 => (1.0, -(x - 645.0) / 65.0, 0.0),
        _ => (1.0, 0.0, 0.0),
    };
    let f = match nm {
        x if x < 420.0 => 0.3 + 0.7 * (x - 380.0) / 40.0,
        x if x > 700.0 => 0.3 + 0.7 * (780.0 - x) / 80.0,
        _ => 1.0,
    }
    .clamp(0.3, 1.0);
    Color::rgb(
        (r * f).clamp(0.0, 1.0).powf(0.8) as f32,
        (g * f).clamp(0.0, 1.0).powf(0.8) as f32,
        (b * f).clamp(0.0, 1.0).powf(0.8) as f32,
    )
}

fn seg_hit(o: Vec2, d: Vec2, a: Vec2, b: Vec2) -> Option<(f64, Vec2)> {
    let e = b - a;
    let den = d.cross(e);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (a - o).cross(e) / den;
    let u = (a - o).cross(d) / den;
    if t > 1e-7 && (-1e-9..=1.0 + 1e-9).contains(&u) {
        let n = e.perp().normalize();
        Some((t, n))
    } else {
        None
    }
}

/// Snell refraction of unit `d` at a surface with unit normal `n` (any side),
/// going from index `n1` to `n2`; `None` means total internal reflection.
pub fn refract(d: Vec2, n: Vec2, n1: f64, n2: f64) -> Option<Vec2> {
    let n = if d.dot(n) > 0.0 { -n } else { n };
    let eta = n1 / n2;
    let cos_i = -d.dot(n);
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
    (k >= 0.0).then(|| (d * eta + n * (eta * cos_i - k.sqrt())).normalize())
}

pub fn reflect(d: Vec2, n: Vec2) -> Vec2 {
    d - n * (2.0 * d.dot(n))
}

fn inside_poly(p: Vec2, poly: &[Vec2]) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
    }
    c
}

/// Trace one ray of wavelength `nm` through the bench. Returns the polyline of the
/// ray (origin first), stopping after `max_events` interactions or at `max_len`.
pub fn trace_ray(
    origin: Vec2,
    dir: Vec2,
    nm: f64,
    optics: &[Optic],
    max_events: usize,
    max_len: f64,
) -> Vec<Vec2> {
    let mut pts = vec![origin];
    let mut o = origin;
    let mut d = dir.normalize();
    let mut travelled = 0.0;
    for _ in 0..max_events {
        // nearest hit
        let mut best: Option<(f64, Vec2, usize)> = None;
        for (i, op) in optics.iter().enumerate() {
            let hits: Vec<(f64, Vec2)> = match op {
                Optic::Mirror { a, b } | Optic::Screen { a, b } => {
                    seg_hit(o, d, *a, *b).into_iter().collect()
                }
                Optic::ThinLens {
                    center,
                    axis,
                    half_height,
                    ..
                } => {
                    let up = axis.perp().normalize();
                    seg_hit(
                        o,
                        d,
                        *center - up * *half_height,
                        *center + up * *half_height,
                    )
                    .into_iter()
                    .collect()
                }
                Optic::Glass { poly, .. } => (0..poly.len())
                    .filter_map(|k| seg_hit(o, d, poly[k], poly[(k + 1) % poly.len()]))
                    .collect(),
                Optic::Disc { center, r, .. } => {
                    let f = o - *center;
                    let bq = f.dot(d);
                    let cq = f.dot(f) - r * r;
                    let disc = bq * bq - cq;
                    if disc < 0.0 {
                        vec![]
                    } else {
                        let s = disc.sqrt();
                        [-bq - s, -bq + s]
                            .into_iter()
                            .filter(|&t| t > 1e-7)
                            .map(|t| (t, (o + d * t - *center).normalize()))
                            .collect()
                    }
                }
            };
            for (t, n) in hits {
                if best.is_none_or(|(bt, _, _)| t < bt) {
                    best = Some((t, n, i));
                }
            }
        }
        let Some((t, n, i)) = best else {
            pts.push(o + d * (max_len - travelled).max(0.0));
            break;
        };
        if travelled + t > max_len {
            pts.push(o + d * (max_len - travelled));
            break;
        }
        travelled += t;
        let p = o + d * t;
        pts.push(p);
        match &optics[i] {
            Optic::Screen { .. } => break,
            Optic::Mirror { .. } => d = reflect(d, n),
            Optic::ThinLens {
                center,
                axis,
                focal,
                ..
            } => {
                // ideal lens: rays through the same point of the focal plane stay parallel
                let ax = axis.normalize() * if d.dot(*axis) >= 0.0 { 1.0 } else { -1.0 };
                let up = ax.perp();
                let h = (p - *center).dot(up);
                let slope_in = d.dot(up) / d.dot(ax);
                let slope_out = slope_in - h / focal;
                d = (ax + up * slope_out).normalize();
            }
            Optic::Glass { poly, n: n0, b } => {
                let ng = cauchy_index(*n0, *b, nm);
                let entering = !inside_poly(p - n * 1e-6 * n.dot(d).signum(), poly);
                let (n1, n2) = if entering { (1.0, ng) } else { (ng, 1.0) };
                d = refract(d, n, n1, n2).unwrap_or_else(|| reflect(d, n));
            }
            Optic::Disc {
                center,
                r,
                n: n0,
                b,
                ..
            } => {
                let ng = cauchy_index(*n0, *b, nm);
                let entering = (o - *center).length() > *r - 1e-6 && d.dot(n) < 0.0;
                let (n1, n2) = if entering { (1.0, ng) } else { (ng, 1.0) };
                d = refract(d, n, n1, n2).unwrap_or_else(|| reflect(d, n));
            }
        }
        o = p + d * 1e-6;
    }
    pts
}

/// Draw the optical elements.
pub fn optics_mobject(optics: &[Optic]) -> Mobject {
    let glass = rgb8(150, 210, 255);
    group(
        optics
            .iter()
            .map(|op| match op {
                Optic::Mirror { a, b } => group(vec![
                    line(*a, *b).stroke(GRAY_A, 5.0),
                    crate::physics::hatched_surface(*a, *b, (*b - *a).perp().normalize() * -1.0),
                ]),
                Optic::Screen { a, b } => line(*a, *b).stroke(GRAY_C, 6.0),
                Optic::ThinLens {
                    center,
                    axis,
                    half_height,
                    focal,
                } => {
                    let up = axis.perp().normalize();
                    let ax = axis.normalize();
                    let (top, bot) = (*center + up * *half_height, *center - up * *half_height);
                    let bulge = if *focal > 0.0 { 0.18 } else { -0.12 };
                    group(vec![
                        polygon(&[
                            top + ax * 0.04,
                            *center + ax * (bulge + 0.04),
                            bot + ax * 0.04,
                            bot - ax * 0.04,
                            *center - ax * (bulge + 0.04),
                            top - ax * 0.04,
                        ])
                        .fill(glass, 0.25)
                        .stroke(glass, 2.0),
                        dot_r(*center + ax * *focal, 0.05, glass),
                        dot_r(*center - ax * *focal, 0.05, glass),
                    ])
                }
                Optic::Glass { poly, .. } => polygon(poly).fill(glass, 0.18).stroke(glass, 2.0),
                Optic::Disc { center, r, .. } => circle(*r)
                    .move_to(*center)
                    .fill(glass, 0.18)
                    .stroke(glass, 2.0),
            })
            .collect::<Vec<_>>(),
    )
}

/// Cut a polyline where it first leaves `rect` (keeps rays inside a bench panel).
pub fn clip_polyline_to_rect(pts: &[Vec2], rect: Rect) -> Vec<Vec2> {
    let inside =
        |p: Vec2| p.x >= rect.min.x && p.x <= rect.max.x && p.y >= rect.min.y && p.y <= rect.max.y;
    let mut out = vec![];
    for (i, &p) in pts.iter().enumerate() {
        if inside(p) {
            out.push(p);
            continue;
        }
        if i > 0 && inside(pts[i - 1]) {
            // binary search the exit point
            let (mut a, mut b) = (pts[i - 1], p);
            for _ in 0..40 {
                let m = a.lerp(b, 0.5);
                if inside(m) {
                    a = m;
                } else {
                    b = m;
                }
            }
            out.push(a);
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

/// Draw the first `frac` of a traced ray (for "light switching on" animations).
pub fn ray_mobject(pts: &[Vec2], frac: f64, color: Color, width: f32) -> Mobject {
    if pts.is_empty() {
        return group(vec![]);
    }
    let total: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total * frac.clamp(0.0, 1.0);
    let mut out = vec![pts[0]];
    for w in pts.windows(2) {
        let l = w[0].distance(w[1]);
        if l <= left {
            out.push(w[1]);
            left -= l;
        } else {
            out.push(w[0].lerp(w[1], left / l.max(1e-12)));
            break;
        }
    }
    if out.len() < 2 {
        out.push(pts[0]);
    }
    polyline(&out).stroke(color, width)
}

// ───────────────────────────── orbits & gravity ─────────────────────────────

/// Solve Kepler's equation M = E − e·sin E (Newton iterations).
pub fn eccentric_anomaly(m: f64, e: f64) -> f64 {
    let mut ea = if e > 0.8 { PI } else { m };
    for _ in 0..30 {
        let f = ea - e * ea.sin() - m;
        ea -= f / (1.0 - e * ea.cos());
        if f.abs() < 1e-13 {
            break;
        }
    }
    ea
}

/// Position on a Kepler ellipse (semi-major `a`, eccentricity `e`) with the attracting
/// focus at the origin, periapsis on +x, after `phase` ∈ [0,1) of a period.
pub fn kepler_position(a: f64, e: f64, phase: f64) -> Vec2 {
    let m = TAU * phase.rem_euclid(1.0);
    let ea = eccentric_anomaly(m, e);
    v2(a * (ea.cos() - e), a * (1.0 - e * e).sqrt() * ea.sin())
}

/// The orbit ellipse (focus at `focus`) as a closed outline.
pub fn kepler_ellipse(a: f64, e: f64, focus: Vec2, color: Color) -> Mobject {
    let pts: Vec<Vec2> = (0..240)
        .map(|k| focus + kepler_position(a, e, k as f64 / 240.0))
        .collect();
    polygon(&pts).stroke(color, 2.0)
}

/// Kepler's second law: `n` wedges swept in equal times, alternately coloured.
pub fn equal_area_sectors(
    a: f64,
    e: f64,
    focus: Vec2,
    n: usize,
    dphase: f64,
    colors: &[Color],
) -> Mobject {
    group(
        (0..n)
            .map(|i| {
                let p0 = i as f64 / n as f64;
                let mut pts = vec![focus];
                for k in 0..=24 {
                    pts.push(focus + kepler_position(a, e, p0 + dphase * k as f64 / 24.0));
                }
                let c = color_gradient(colors, i as f64 / n.max(2) as f64);
                polygon(&pts).fill(c, 0.45).stroke(c, 1.0)
            })
            .collect::<Vec<_>>(),
    )
}

/// "Rubber sheet" picture of gravity: a grid whose points are pulled into each mass,
/// drawn with a tilted perspective so the wells read as depth.
pub fn gravity_well(
    masses: &[(Vec2, f64)],
    half: f64,
    lines: usize,
    depth: f64,
    tilt: f64,
    color: Color,
) -> Mobject {
    let sink = |p: Vec2| -> f64 {
        masses
            .iter()
            .map(|(c, m)| -m / (p.distance(*c).powi(2) + 0.35).sqrt())
            .sum::<f64>()
    };
    let proj = |p: Vec2| {
        let z = sink(p) * depth;
        let s = 1.0 / (1.0 + p.y * tilt * 0.12);
        v2(p.x * s, p.y * tilt * s + z)
    };
    let mut kids = vec![];
    let n = lines.max(2);
    for i in 0..=n {
        let u = -half + 2.0 * half * i as f64 / n as f64;
        let row: Vec<Vec2> = (0..=80)
            .map(|k| proj(v2(-half + 2.0 * half * k as f64 / 80.0, u)))
            .collect();
        let col: Vec<Vec2> = (0..=80)
            .map(|k| proj(v2(u, -half + 2.0 * half * k as f64 / 80.0)))
            .collect();
        kids.push(polyline(&row).stroke(color.with_alpha(0.75), 1.4));
        kids.push(polyline(&col).stroke(color.with_alpha(0.75), 1.4));
    }
    for (c, m) in masses {
        kids.push(
            circle(0.08 + 0.06 * m.sqrt())
                .fill(ORANGE, 1.0)
                .no_stroke()
                .move_to(proj(*c) + v2(0.0, 0.08))
                .glow(0.3, Some(YELLOW)),
        );
    }
    group(kids)
}

// ───────────────────────────── dynamics ─────────────────────────────

/// RK4 trajectory of the planar ODE `p' = f(p)`.
pub fn flow_path(f: &dyn Fn(Vec2) -> Vec2, p0: Vec2, dt: f64, steps: usize) -> Vec<Vec2> {
    let mut p = p0;
    let mut out = vec![p];
    for _ in 0..steps {
        let k1 = f(p);
        let k2 = f(p + k1 * (dt / 2.0));
        let k3 = f(p + k2 * (dt / 2.0));
        let k4 = f(p + k3 * dt);
        p += (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (dt / 6.0);
        out.push(p);
        if !p.x.is_finite() || p.length() > 1e6 {
            break;
        }
    }
    out
}

/// Phase portrait: small arrows of `f` on a grid in `rect` plus trajectories from
/// `starts`. `f` works in the rect's own coordinates (scene units).
pub fn phase_portrait(
    rect: Rect,
    f: &dyn Fn(Vec2) -> Vec2,
    step: f64,
    starts: &[Vec2],
    t_max: f64,
    colors: &[Color],
) -> Mobject {
    let mut kids = vec![];
    let (lo, hi) = (rect.min, rect.max);
    let mut y = lo.y + step / 2.0;
    while y < hi.y {
        let mut x = lo.x + step / 2.0;
        while x < hi.x {
            let p = v2(x, y);
            let v = f(p);
            let l = v.length();
            if l > 1e-9 {
                let d = v / l * (step * 0.35);
                let c = GRAY_B.mix(BLUE_B, (l / (l + 1.0)) as f32);
                kids.push(line(p - d, p + d).stroke(c.with_alpha(0.7), 1.4));
                kids.push(dot_r(p + d, 0.018, c));
            }
            x += step;
        }
        y += step;
    }
    let inside = |p: &Vec2| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y;
    for (i, s) in starts.iter().enumerate() {
        let path: Vec<Vec2> = flow_path(f, *s, 0.01, (t_max / 0.01) as usize)
            .into_iter()
            .take_while(inside)
            .collect();
        if path.len() > 1 {
            let c = color_gradient(colors, i as f64 / starts.len().max(2) as f64);
            kids.push(polyline(&path).stroke(c, 2.5));
        }
    }
    group(kids)
}

/// Two-arm pendulum figure for angles from the vertical.
pub fn double_pendulum_arms(
    pivot: Vec2,
    l1: f64,
    l2: f64,
    th1: f64,
    th2: f64,
    color: Color,
) -> Mobject {
    let p1 = pivot + v2(th1.sin(), -th1.cos()) * l1;
    let p2 = p1 + v2(th2.sin(), -th2.cos()) * l2;
    group(vec![
        line(pivot, p1).stroke(color.with_alpha(0.75), 2.0),
        line(p1, p2).stroke(color.with_alpha(0.75), 2.0),
        dot_r(p2, 0.05, color),
    ])
}

/// Bob-2 position of a double pendulum.
pub fn double_pendulum_tip(pivot: Vec2, l1: f64, l2: f64, th1: f64, th2: f64) -> Vec2 {
    pivot + v2(th1.sin(), -th1.cos()) * l1 + v2(th2.sin(), -th2.cos()) * l2
}

/// 2-D Maxwell–Boltzmann speed density f(v) = v/a · exp(−v²/2a), `a = kT/m`.
pub fn maxwell_boltzmann_2d(v: f64, a: f64) -> f64 {
    v / a * (-v * v / (2.0 * a)).exp()
}

/// 3-D Maxwell–Boltzmann speed density.
pub fn maxwell_boltzmann_3d(v: f64, a: f64) -> f64 {
    (2.0 / PI).sqrt() * v * v / a.powf(1.5) * (-v * v / (2.0 * a)).exp()
}

/// Normalised histogram (densities) of `values` in `bins` buckets over `[0, max]`.
pub fn histogram_density(values: &[f64], bins: usize, max: f64) -> Vec<f64> {
    let mut h = vec![0.0; bins.max(1)];
    let w = max / bins.max(1) as f64;
    for v in values {
        let i = (v / w) as usize;
        if i < h.len() {
            h[i] += 1.0;
        }
    }
    let n = values.len().max(1) as f64;
    h.iter().map(|c| c / (n * w)).collect()
}

/// Path of a charge in the plane with in-plane `e(p)` and out-of-plane `b(p)`
/// (Lorentz force a = q/m (E + v × B)), integrated with the Boris pusher.
pub fn lorentz_path(
    q_over_m: f64,
    p0: Vec2,
    v0: Vec2,
    e: &dyn Fn(Vec2) -> Vec2,
    b: &dyn Fn(Vec2) -> f64,
    dt: f64,
    steps: usize,
) -> Vec<Vec2> {
    let (mut p, mut v) = (p0, v0);
    let mut out = vec![p];
    for _ in 0..steps {
        let h = q_over_m * dt / 2.0;
        let vm = v + e(p) * h;
        let t = b(p) * h;
        let s = 2.0 * t / (1.0 + t * t);
        let vp = vm + v2(vm.y * t, -vm.x * t);
        let vplus = vm + v2(vp.y * s, -vp.x * s);
        v = vplus + e(p) * h;
        p += v * dt;
        out.push(p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snell_and_tir() {
        let d = v2(1.0, -1.0).normalize();
        let n = v2(0.0, 1.0);
        let r = refract(d, n, 1.0, 1.5).unwrap();
        let s_in = d.x.abs();
        let s_out = r.x.abs();
        assert!((s_in - 1.5 * s_out).abs() < 1e-12);
        assert!(refract(d, n, 1.5, 1.0).is_none()); // 45° > critical angle 41.8°
    }

    #[test]
    fn thin_lens_focuses_parallel_rays() {
        let optics = [Optic::ThinLens {
            center: Vec2::ZERO,
            axis: v2(1.0, 0.0),
            half_height: 2.0,
            focal: 2.0,
        }];
        for h in [-1.0, -0.4, 0.5, 1.2] {
            let pts = trace_ray(v2(-3.0, h), v2(1.0, 0.0), 550.0, &optics, 4, 5.0);
            let (a, b) = (pts[1], pts[2]);
            // where does it cross the axis?
            let t = -a.y / (b.y - a.y);
            let x = a.x + t * (b.x - a.x);
            assert!((x - 2.0).abs() < 1e-9, "{x}");
        }
    }

    #[test]
    fn prism_disperses() {
        let poly = vec![v2(-1.0, -0.8), v2(1.0, -0.8), v2(0.0, 1.0)];
        let optics = [Optic::Glass {
            poly,
            n: 1.5,
            b: 0.02,
        }];
        let end = |nm| {
            *trace_ray(v2(-3.0, -0.6), v2(1.0, 0.25), nm, &optics, 6, 8.0)
                .last()
                .unwrap()
        };
        assert!(end(420.0).distance(end(680.0)) > 0.05);
        assert!(cauchy_index(1.5, 0.02, 420.0) > cauchy_index(1.5, 0.02, 680.0));
    }

    #[test]
    fn kepler() {
        let (a, e) = (2.0, 0.6);
        assert!(kepler_position(a, e, 0.0).distance(v2(a * (1.0 - e), 0.0)) < 1e-9);
        assert!(kepler_position(a, e, 0.5).distance(v2(-a * (1.0 + e), 0.0)) < 1e-9);
        // equal areas: wedges at periapsis and apoapsis sweep the same area
        let area = |p0: f64| {
            let mut s = 0.0;
            for k in 0..2000 {
                let p = kepler_position(a, e, p0 + 0.05 * k as f64 / 2000.0);
                let q = kepler_position(a, e, p0 + 0.05 * (k + 1) as f64 / 2000.0);
                s += 0.5 * p.cross(q);
            }
            s
        };
        assert!((area(0.0) - area(0.45)).abs() < 1e-6);
    }

    #[test]
    fn cyclotron_radius() {
        // r = m v / (q B)
        let pts = lorentz_path(
            1.0,
            Vec2::ZERO,
            v2(1.0, 0.0),
            &|_| Vec2::ZERO,
            &|_| 2.0,
            0.001,
            4000,
        );
        let maxd = pts.iter().map(|p| p.length()).fold(0.0, f64::max);
        assert!((maxd - 1.0).abs() < 1e-3, "{maxd}");
    }

    #[test]
    fn mb_normalised() {
        let s: f64 = (0..20000)
            .map(|k| maxwell_boltzmann_2d(k as f64 * 0.001, 1.3) * 0.001)
            .sum();
        assert!((s - 1.0).abs() < 1e-3);
        let h = histogram_density(&[0.1, 0.2, 0.3, 0.9], 4, 1.0);
        assert!((h.iter().sum::<f64>() * 0.25 - 1.0).abs() < 1e-12);
    }
}
