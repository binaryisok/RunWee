//! Remotion utility parity: `interpolateColors`, `random(seed)`, `@remotion/noise`
//! (simplex 2D/3D/4D), `measureSpring`, `@remotion/paths` helpers (parse SVG path
//! data, evolve, interpolate, warp, length/point/tangent at length) and
//! `@remotion/layout-utils` (measure / fit text).

use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::path::*;
use crate::rate::{interpolate, SpringConfig};

/// Remotion's `interpolateColors(input, inputRange, outputRange)` (OKLab mixing).
pub fn interpolate_colors(input: f64, input_range: &[f64], colors: &[Color]) -> Color {
    assert_eq!(
        input_range.len(),
        colors.len(),
        "interpolate_colors: ranges must match"
    );
    let n = input_range.len();
    if n == 0 {
        return WHITE;
    }
    if input <= input_range[0] {
        return colors[0];
    }
    if input >= input_range[n - 1] {
        return colors[n - 1];
    }
    let i = (0..n - 1)
        .find(|&i| input < input_range[i + 1])
        .unwrap_or(n - 2);
    let t = interpolate(input, &[input_range[i], input_range[i + 1]], &[0.0, 1.0]);
    colors[i].mix(colors[i + 1], t as f32)
}

/// Remotion's `random(seed)`: deterministic value in [0, 1) from a string seed.
pub fn random(seed: &str) -> f64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in seed.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    Rng::new(h).next_f64()
}

/// `random` with a numeric seed.
pub fn random_n(seed: u64) -> f64 {
    Rng::new(seed).next_f64()
}

/// Remotion's `measureSpring`: seconds until the spring stays within `threshold`.
pub fn measure_spring(cfg: SpringConfig, threshold: f64) -> f64 {
    let mut t = 0.0;
    let dt = 1.0 / 600.0;
    let mut last = 0.0;
    while t < 60.0 {
        if (crate::rate::spring_value(t, cfg) - 1.0).abs() > threshold {
            last = t;
        }
        if t - last > 2.0 {
            break;
        }
        t += dt;
    }
    last
}

// ------------------------------------------------------------------- noise

const GRAD3: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

fn perm(seed: &str) -> std::sync::Arc<[u8; 512]> {
    use std::collections::HashMap;
    use std::sync::{Arc, OnceLock, RwLock};
    static CACHE: OnceLock<RwLock<HashMap<String, Arc<[u8; 512]>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(p) = cache.read().unwrap().get(seed) {
        return p.clone();
    }
    let p = Arc::new(build_perm(seed));
    cache.write().unwrap().insert(seed.to_string(), p.clone());
    p
}

fn build_perm(seed: &str) -> [u8; 512] {
    let mut p: Vec<u8> = (0..=255).collect();
    let mut rng = Rng::new((random(seed) * u64::MAX as f64) as u64 | 1);
    for i in (1..256).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        p.swap(i, j);
    }
    let mut out = [0u8; 512];
    for i in 0..512 {
        out[i] = p[i & 255];
    }
    out
}

/// Seeded 2D simplex noise in [-1, 1] (`noise2D(seed, x, y)`).
pub fn noise2d(seed: &str, x: f64, y: f64) -> f64 {
    let p = perm(seed);
    let f2 = 0.5 * (3f64.sqrt() - 1.0);
    let g2 = (3.0 - 3f64.sqrt()) / 6.0;
    let s = (x + y) * f2;
    let (i, j) = ((x + s).floor(), (y + s).floor());
    let t = (i + j) * g2;
    let (x0, y0) = (x - (i - t), y - (j - t));
    let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
    let (x1, y1) = (x0 - i1 as f64 + g2, y0 - j1 as f64 + g2);
    let (x2, y2) = (x0 - 1.0 + 2.0 * g2, y0 - 1.0 + 2.0 * g2);
    let (ii, jj) = ((i as i64 & 255) as usize, (j as i64 & 255) as usize);
    let corner = |gx: f64, gy: f64, g: usize| {
        let t = 0.5 - gx * gx - gy * gy;
        if t < 0.0 {
            0.0
        } else {
            let gr = GRAD3[g % 12];
            t.powi(4) * (gr[0] * gx + gr[1] * gy)
        }
    };
    let n0 = corner(x0, y0, p[ii + p[jj] as usize] as usize);
    let n1 = corner(x1, y1, p[ii + i1 + p[jj + j1] as usize] as usize);
    let n2 = corner(x2, y2, p[ii + 1 + p[jj + 1] as usize] as usize);
    70.0 * (n0 + n1 + n2)
}

/// Seeded 3D simplex noise in [-1, 1] (`noise3D(seed, x, y, z)`).
pub fn noise3d(seed: &str, x: f64, y: f64, z: f64) -> f64 {
    let p = perm(seed);
    let f3 = 1.0 / 3.0;
    let g3 = 1.0 / 6.0;
    let s = (x + y + z) * f3;
    let (i, j, k) = ((x + s).floor(), (y + s).floor(), (z + s).floor());
    let t = (i + j + k) * g3;
    let (x0, y0, z0) = (x - (i - t), y - (j - t), z - (k - t));
    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else if y0 < z0 {
        (0, 0, 1, 0, 1, 1)
    } else if x0 < z0 {
        (0, 1, 0, 0, 1, 1)
    } else {
        (0, 1, 0, 1, 1, 0)
    };
    let offs = [
        (0.0, 0.0, 0.0, 0usize, 0usize, 0usize),
        (i1 as f64 - g3, j1 as f64 - g3, k1 as f64 - g3, i1, j1, k1),
        (
            i2 as f64 - 2.0 * g3,
            j2 as f64 - 2.0 * g3,
            k2 as f64 - 2.0 * g3,
            i2,
            j2,
            k2,
        ),
        (1.0 - 3.0 * g3, 1.0 - 3.0 * g3, 1.0 - 3.0 * g3, 1, 1, 1),
    ];
    let (ii, jj, kk) = (
        (i as i64 & 255) as usize,
        (j as i64 & 255) as usize,
        (k as i64 & 255) as usize,
    );
    let mut n = 0.0;
    for (dx, dy, dz, a, b, c) in offs {
        let (px, py, pz) = (x0 - dx, y0 - dy, z0 - dz);
        let tt = 0.6 - px * px - py * py - pz * pz;
        if tt > 0.0 {
            let g = p[ii + a + p[jj + b + p[kk + c] as usize] as usize] as usize % 12;
            let gr = GRAD3[g];
            n += tt.powi(4) * (gr[0] * px + gr[1] * py + gr[2] * pz);
        }
    }
    32.0 * n
}

/// Seeded 4D noise in [-1, 1] (`noise4D`), built from two decorrelated 3D fields.
pub fn noise4d(seed: &str, x: f64, y: f64, z: f64, w: f64) -> f64 {
    let a = noise3d(seed, x + w * 0.5, y - w * 0.3, z + w * 0.7);
    let b = noise3d(&format!("{seed}#4d"), x - w * 0.6, y + w * 0.8, z - w * 0.2);
    (a + b) * 0.5 * std::f64::consts::SQRT_2
}

// ------------------------------------------------------------------- paths

/// Parse SVG path data (`d` attribute) into a [`Path`], in SVG coordinates (y down).
/// Supports M L H V C S Q T A Z (absolute and relative).
pub fn parse_path(d: &str) -> Path {
    let mut toks: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev_e = false;
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() && ch != 'e' && ch != 'E' {
            if !cur.is_empty() {
                toks.push(std::mem::take(&mut cur));
            }
            toks.push(ch.to_string());
        } else if ch == ',' || ch.is_whitespace() {
            if !cur.is_empty() {
                toks.push(std::mem::take(&mut cur));
            }
        } else if ((ch == '-' && !cur.is_empty()) || (ch == '.' && cur.contains('.'))) && !prev_e {
            toks.push(std::mem::take(&mut cur));
            cur.push(ch);
        } else {
            cur.push(ch);
        }
        prev_e = ch == 'e' || ch == 'E';
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    let mut path = Path::new();
    let mut i = 0;
    let mut cmd = 'M';
    let mut pos = Vec2::ZERO;
    let mut start = Vec2::ZERO;
    let mut last_ctrl: Option<Vec2> = None;
    let mut last_q: Option<Vec2> = None;
    let num = |toks: &Vec<String>, i: &mut usize| -> f64 {
        let v = toks.get(*i).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        *i += 1;
        v
    };
    while i < toks.len() {
        if toks[i].len() == 1 && toks[i].chars().next().unwrap().is_ascii_alphabetic() {
            cmd = toks[i].chars().next().unwrap();
            i += 1;
            if cmd == 'Z' || cmd == 'z' {
                path.close();
                pos = start;
                continue;
            }
        }
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { pos } else { Vec2::ZERO };
        match cmd.to_ascii_uppercase() {
            'M' => {
                let p = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                path.move_to(p);
                pos = p;
                start = p;
                cmd = if rel { 'l' } else { 'L' };
                last_ctrl = None;
                last_q = None;
            }
            'L' => {
                let p = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                path.line_to(p);
                pos = p;
                last_ctrl = None;
                last_q = None;
            }
            'H' => {
                let x = num(&toks, &mut i) + if rel { pos.x } else { 0.0 };
                pos = v2(x, pos.y);
                path.line_to(pos);
            }
            'V' => {
                let y = num(&toks, &mut i) + if rel { pos.y } else { 0.0 };
                pos = v2(pos.x, y);
                path.line_to(pos);
            }
            'C' => {
                let c1 = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                let c2 = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                let p = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                path.cubic_to(c1, c2, p);
                last_ctrl = Some(c2);
                pos = p;
            }
            'S' => {
                let c1 = last_ctrl.map_or(pos, |c| pos * 2.0 - c);
                let c2 = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                let p = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                path.cubic_to(c1, c2, p);
                last_ctrl = Some(c2);
                pos = p;
            }
            'Q' => {
                let c = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                let p = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                path.quad_to(c, p);
                last_q = Some(c);
                pos = p;
            }
            'T' => {
                let c = last_q.map_or(pos, |q| pos * 2.0 - q);
                let p = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                path.quad_to(c, p);
                last_q = Some(c);
                pos = p;
            }
            'A' => {
                let (rx, ry) = (num(&toks, &mut i).abs(), num(&toks, &mut i).abs());
                let rot = num(&toks, &mut i).to_radians();
                let large = num(&toks, &mut i) != 0.0;
                let sweep = num(&toks, &mut i) != 0.0;
                let p = base + v2(num(&toks, &mut i), num(&toks, &mut i));
                svg_arc(&mut path, pos, p, rx, ry, rot, large, sweep);
                pos = p;
            }
            _ => {
                i += 1;
            }
        }
    }
    path
}

/// Endpoint-parameterised elliptical arc → cubic segments.
fn svg_arc(
    path: &mut Path,
    p0: Vec2,
    p1: Vec2,
    mut rx: f64,
    mut ry: f64,
    rot: f64,
    large: bool,
    sweep: bool,
) {
    if rx < 1e-9 || ry < 1e-9 {
        path.line_to(p1);
        return;
    }
    let d = (p0 - p1) / 2.0;
    let x1 = d.rotate(-rot);
    let lam = (x1.x / rx).powi(2) + (x1.y / ry).powi(2);
    if lam > 1.0 {
        rx *= lam.sqrt();
        ry *= lam.sqrt();
    }
    let num = (rx * rx * ry * ry - rx * rx * x1.y * x1.y - ry * ry * x1.x * x1.x).max(0.0);
    let den = rx * rx * x1.y * x1.y + ry * ry * x1.x * x1.x;
    let mut coef = (num / den.max(1e-12)).sqrt();
    if large == sweep {
        coef = -coef;
    }
    let c1 = v2(coef * rx * x1.y / ry, -coef * ry * x1.x / rx);
    let center = c1.rotate(rot) + (p0 + p1) / 2.0;
    let ang = |u: Vec2, v: Vec2| u.cross(v).atan2(u.dot(v));
    let u = v2((x1.x - c1.x) / rx, (x1.y - c1.y) / ry);
    let v = v2((-x1.x - c1.x) / rx, (-x1.y - c1.y) / ry);
    let start = ang(RIGHT, u);
    let mut delta = ang(u, v);
    if !sweep && delta > 0.0 {
        delta -= TAU;
    } else if sweep && delta < 0.0 {
        delta += TAU;
    }
    if let Some(sp) = path.subpaths.last_mut() {
        append_arc(&mut sp.segs, center, rx, ry, start, delta, rot);
    }
}

/// `evolvePath(progress, path)`: the first `progress` of the path's length.
pub fn evolve_path(p: &Path, progress: f64) -> Path {
    p.partial(0.0, progress)
}

/// `interpolatePath(t, a, b)`.
pub fn interpolate_path(t: f64, a: &Path, b: &Path) -> Path {
    let (x, y) = Path::align(a, b);
    x.lerp(&y, t)
}

/// `warpPath(path, f)`: apply a point warp after subdividing for smoothness.
pub fn warp_path(p: &Path, f: impl Fn(Vec2) -> Vec2) -> Path {
    let mut q = p.subdivided(8);
    q.map_points(&f);
    q
}

/// `getPointAtLength` / `getTangentAtLength` (length in path units).
pub fn point_at_length(p: &Path, len: f64) -> Vec2 {
    p.point_from_proportion(len / p.length().max(1e-12))
}
pub fn tangent_at_length(p: &Path, len: f64) -> Vec2 {
    p.tangent_from_proportion(len / p.length().max(1e-12))
}

/// Mobject from SVG path data, `height` units tall, centred (y flipped to scene space).
pub fn path_mobject(d: &str, height: f64) -> Mobject {
    let mut p = parse_path(d);
    p.transform(&Affine::scale(1.0, -1.0));
    let m = Mobject::from_path(p);
    let m = m.set_height(height);
    let c = m.center();
    m.shift(-c)
}

// ------------------------------------------------------------- layout utils

/// `measureText`: (width, height) of a single line in scene units.
pub fn measure_text(text: &str, size: f64, font: &str, bold: bool) -> (f64, f64) {
    let id = crate::fonts::find_font(font, bold, false);
    let (_, w) = crate::fonts::shape(id, text);
    let (a, d) = crate::fonts::line_metrics(id);
    (w * size, (a + d) * size)
}

/// `fitText`: largest font size so `text` fits in `width` on one line.
pub fn fit_text(text: &str, width: f64, font: &str, bold: bool) -> f64 {
    let (w, _) = measure_text(text, 1.0, font, bold);
    width / w.max(1e-9)
}

/// `fitTextOnNLines`: largest size so the text wraps into at most `lines` lines of `width`.
pub fn fit_text_on_n_lines(text: &str, width: f64, lines: usize, font: &str) -> f64 {
    let (mut lo, mut hi) = (
        0.01,
        fit_text(text, width, font, false).max(0.02) * lines as f64,
    );
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        let m = crate::text::Text::new(text)
            .font(font)
            .size(mid)
            .max_width(width)
            .build();
        if m.children.len() <= lines && m.width() <= width * 1.001 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_random_noise() {
        assert_eq!(interpolate_colors(0.0, &[0.0, 1.0], &[RED, BLUE]), RED);
        assert_eq!(interpolate_colors(2.0, &[0.0, 1.0], &[RED, BLUE]), BLUE);
        assert_eq!(random("a"), random("a"));
        assert_ne!(random("a"), random("b"));
        let n: Vec<f64> = (0..200)
            .map(|i| noise2d("x", i as f64 * 0.37, i as f64 * 0.11))
            .collect();
        assert!(n.iter().all(|v| v.abs() <= 1.0001));
        assert!(n.iter().any(|v| *v > 0.2) && n.iter().any(|v| *v < -0.2));
        assert!(noise3d("x", 0.3, 0.7, 1.1).abs() <= 1.0);
        assert!(noise4d("x", 0.3, 0.7, 1.1, 2.0).abs() <= 1.5);
        let t = measure_spring(SpringConfig::default(), 0.005);
        assert!(t > 0.3 && t < 2.0, "{t}");
    }

    #[test]
    fn svg_path_parsing() {
        let p = parse_path("M10 10 H 90 V 90 H 10 Z");
        assert!((p.length() - 320.0).abs() < 1e-6);
        let c = parse_path("M 0 0 A 10 10 0 0 1 20 0");
        assert!((c.length() - std::f64::consts::PI * 10.0).abs() < 0.05);
        let q = parse_path("m0,0c10,0 10,10 20,10s10-10 20-10q5,5 10,0t10,0l-5-5z");
        assert!(q.num_segments() > 4);
        let m = path_mobject("M0 0 L10 0 L5 10 Z", 2.0);
        assert!((m.height() - 2.0).abs() < 1e-9);
        let (w, h) = measure_text("Hello", 1.0, "Inter", false);
        assert!(w > 2.0 && h > 0.9);
        let s = fit_text("Hello", 4.0, "Inter", false);
        assert!((measure_text("Hello", s, "Inter", false).0 - 4.0).abs() < 1e-6);
        assert!(fit_text_on_n_lines("the quick brown fox jumps", 3.0, 2, "Inter") > 0.1);
    }
}
