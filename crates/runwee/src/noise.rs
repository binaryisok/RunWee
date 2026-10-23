//! Fast, table-free procedural noise (phase A3).
//!
//! Integer-hash gradient noise that is cheap enough to evaluate per pixel:
//! Perlin (2D/3D), simplex (2D), fBm, ridged multifractal, turbulence, Worley /
//! cellular, curl noise (divergence-free flow), and Inigo Quilez-style domain
//! warping. All functions are pure and deterministic in `(input, seed)`.
use crate::math::*;

#[inline]
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Hash of an integer lattice point.
#[inline]
pub fn hash2(x: i64, y: i64, seed: u64) -> u64 {
    mix64(
        (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ seed.wrapping_mul(0x1656_67B1_9E37_79F9),
    )
}

#[inline]
pub fn hash3(x: i64, y: i64, z: i64, seed: u64) -> u64 {
    mix64(hash2(x, y, seed) ^ (z as u64).wrapping_mul(0x27D4_EB2F_1656_67C5))
}

/// Uniform `[0,1)` from a hash.
#[inline]
pub fn hash_unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn grad2(h: u64, x: f64, y: f64) -> f64 {
    // 8 unit-ish gradient directions
    match h & 7 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x * std::f64::consts::SQRT_2,
        5 => -x * std::f64::consts::SQRT_2,
        6 => y * std::f64::consts::SQRT_2,
        _ => -y * std::f64::consts::SQRT_2,
    }
}

#[inline]
fn grad3(h: u64, x: f64, y: f64, z: f64) -> f64 {
    match h % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

/// Classic (improved) Perlin noise in 2D, roughly in `[-1, 1]`.
pub fn perlin2(p: Vec2, seed: u64) -> f64 {
    let (xi, yi) = (p.x.floor(), p.y.floor());
    let (xf, yf) = (p.x - xi, p.y - yi);
    let (x0, y0) = (xi as i64, yi as i64);
    let (u, v) = (fade(xf), fade(yf));
    let n00 = grad2(hash2(x0, y0, seed), xf, yf);
    let n10 = grad2(hash2(x0 + 1, y0, seed), xf - 1.0, yf);
    let n01 = grad2(hash2(x0, y0 + 1, seed), xf, yf - 1.0);
    let n11 = grad2(hash2(x0 + 1, y0 + 1, seed), xf - 1.0, yf - 1.0);
    let a = lerp(n00, n10, u);
    let b = lerp(n01, n11, u);
    (lerp(a, b, v) * std::f64::consts::FRAC_1_SQRT_2).clamp(-1.0, 1.0)
}

/// Perlin noise in 3D (use `z` as time for animated 2D fields), roughly `[-1, 1]`.
pub fn perlin3(x: f64, y: f64, z: f64, seed: u64) -> f64 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let (xf, yf, zf) = (x - xi, y - yi, z - zi);
    let (x0, y0, z0) = (xi as i64, yi as i64, zi as i64);
    let (u, v, w) = (fade(xf), fade(yf), fade(zf));
    let g = |dx: i64, dy: i64, dz: i64| {
        grad3(
            hash3(x0 + dx, y0 + dy, z0 + dz, seed),
            xf - dx as f64,
            yf - dy as f64,
            zf - dz as f64,
        )
    };
    let x00 = lerp(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = lerp(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = lerp(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = lerp(g(0, 1, 1), g(1, 1, 1), u);
    (lerp(lerp(x00, x10, v), lerp(x01, x11, v), w) * 0.85).clamp(-1.0, 1.0)
}

/// 2D simplex noise, roughly `[-1, 1]`.
pub fn simplex2(p: Vec2, seed: u64) -> f64 {
    const F2: f64 = 0.366_025_403_784_438_6; // (sqrt(3)-1)/2
    const G2: f64 = 0.211_324_865_405_187_1; // (3-sqrt(3))/6
    let s = (p.x + p.y) * F2;
    let (i, j) = ((p.x + s).floor(), (p.y + s).floor());
    let t = (i + j) * G2;
    let (x0, y0) = (p.x - (i - t), p.y - (j - t));
    let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
    let (x1, y1) = (x0 - i1 as f64 + G2, y0 - j1 as f64 + G2);
    let (x2, y2) = (x0 - 1.0 + 2.0 * G2, y0 - 1.0 + 2.0 * G2);
    let (ii, jj) = (i as i64, j as i64);
    let corner = |h: u64, x: f64, y: f64| {
        let t = 0.5 - x * x - y * y;
        if t < 0.0 {
            0.0
        } else {
            let t2 = t * t;
            t2 * t2 * grad2(h, x, y)
        }
    };
    let n = corner(hash2(ii, jj, seed), x0, y0)
        + corner(hash2(ii + i1, jj + j1, seed), x1, y1)
        + corner(hash2(ii + 1, jj + 1, seed), x2, y2);
    (n * 70.0 / std::f64::consts::SQRT_2).clamp(-1.0, 1.0)
}

/// Fractal Brownian motion over Perlin noise (normalised to roughly `[-1, 1]`).
pub fn fbm2(p: Vec2, octaves: u32, lacunarity: f64, gain: f64, seed: u64) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves.max(1) {
        sum += amp * perlin2(p * freq, seed.wrapping_add(o as u64 * 101));
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    sum / norm
}

/// Animated fBm: 3D Perlin with `z = time`.
pub fn fbm3(p: Vec2, z: f64, octaves: u32, seed: u64) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves.max(1) {
        sum += amp
            * perlin3(
                p.x * freq,
                p.y * freq,
                z * freq,
                seed.wrapping_add(o as u64 * 101),
            );
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Ridged multifractal in `[0, 1]` — sharp mountain ridges / lightning veins.
pub fn ridged2(p: Vec2, octaves: u32, seed: u64) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm, mut prev) = (0.0, 0.5, 1.0, 0.0, 1.0);
    for o in 0..octaves.max(1) {
        let n = 1.0 - perlin2(p * freq, seed.wrapping_add(o as u64 * 131)).abs();
        let n = n * n;
        sum += n * amp * prev;
        norm += amp;
        prev = n;
        amp *= 0.5;
        freq *= 2.0;
    }
    (sum / norm).clamp(0.0, 1.0)
}

/// Turbulence (sum of |noise|) in `[0, 1]` — marble / fire textures.
pub fn turbulence2(p: Vec2, octaves: u32, seed: u64) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves.max(1) {
        sum += amp * perlin2(p * freq, seed.wrapping_add(o as u64 * 17)).abs();
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    (sum / norm).clamp(0.0, 1.0)
}

/// Worley / cellular noise: distances to the nearest and second-nearest feature
/// points, plus a stable id of the nearest cell.
pub fn worley2(p: Vec2, seed: u64) -> (f64, f64, u64) {
    let (xi, yi) = (p.x.floor() as i64, p.y.floor() as i64);
    let (mut f1, mut f2, mut id) = (f64::INFINITY, f64::INFINITY, 0);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (cx, cy) = (xi + dx, yi + dy);
            let h = hash2(cx, cy, seed);
            let fp = v2(cx as f64 + hash_unit(h), cy as f64 + hash_unit(mix64(h)));
            let d = fp.distance(p);
            if d < f1 {
                f2 = f1;
                f1 = d;
                id = h;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2, id)
}

/// Curl of an animated Perlin potential: a divergence-free (swirly, incompressible)
/// 2D velocity field. Typical magnitude ~1–3.
pub fn curl2(p: Vec2, t: f64, seed: u64) -> Vec2 {
    let e = 1e-3;
    let n = |x: f64, y: f64| perlin3(x, y, t, seed);
    let dndx = (n(p.x + e, p.y) - n(p.x - e, p.y)) / (2.0 * e);
    let dndy = (n(p.x, p.y + e) - n(p.x, p.y - e)) / (2.0 * e);
    v2(dndy, -dndx)
}

/// Domain-warped fBm (`f(p + k·fbm(p + k·fbm(p)))`), IQ style, in roughly `[-1, 1]`.
pub fn domain_warp2(p: Vec2, strength: f64, t: f64, seed: u64) -> f64 {
    let q = v2(
        fbm3(p, t * 0.15, 4, seed),
        fbm3(p + v2(5.2, 1.3), t * 0.15, 4, seed ^ 0x1234),
    );
    let r = v2(
        fbm3(p + q * strength + v2(1.7, 9.2), t * 0.126, 4, seed ^ 0x9876),
        fbm3(p + q * strength + v2(8.3, 2.8), t * 0.126, 4, seed ^ 0x5555),
    );
    fbm3(p + r * strength, t * 0.1, 5, seed ^ 0x4242)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> impl Iterator<Item = Vec2> {
        (0..400).map(|i| v2(i as f64 * 0.137 - 20.0, (i * 7 % 61) as f64 * 0.291 - 9.0))
    }

    #[test]
    fn ranges_and_determinism() {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in samples() {
            for v in [
                perlin2(p, 3),
                simplex2(p, 3),
                fbm2(p, 5, 2.0, 0.5, 3),
                perlin3(p.x, p.y, 0.3, 3),
            ] {
                assert!((-1.0..=1.0).contains(&v));
                lo = lo.min(v);
                hi = hi.max(v);
            }
            assert!((0.0..=1.0).contains(&ridged2(p, 5, 1)));
            assert!((0.0..=1.0).contains(&turbulence2(p, 5, 1)));
            assert_eq!(perlin2(p, 9), perlin2(p, 9));
        }
        assert!(
            lo < -0.3 && hi > 0.3,
            "noise should use its range: {lo} {hi}"
        );
        // integer lattice points of Perlin are zero
        assert!(perlin2(v2(3.0, -4.0), 1).abs() < 1e-12);
    }

    #[test]
    fn continuity() {
        for p in samples() {
            let d = v2(1e-4, -1e-4);
            assert!((perlin2(p, 1) - perlin2(p + d, 1)).abs() < 1e-3);
            assert!((simplex2(p, 1) - simplex2(p + d, 1)).abs() < 2e-3);
        }
    }

    #[test]
    fn worley_order_and_curl_divergence() {
        for p in samples() {
            let (f1, f2, _) = worley2(p, 5);
            assert!(f1 <= f2 && f1 < 1.5);
            // numerical divergence of curl noise ≈ 0
            let e = 1e-3;
            let div = (curl2(p + v2(e, 0.0), 0.5, 2).x - curl2(p - v2(e, 0.0), 0.5, 2).x
                + curl2(p + v2(0.0, e), 0.5, 2).y
                - curl2(p - v2(0.0, e), 0.5, 2).y)
                / (2.0 * e);
            assert!(div.abs() < 0.05, "div {div}");
        }
    }

    #[test]
    fn domain_warp_varies() {
        let a = domain_warp2(v2(0.3, 0.4), 4.0, 0.0, 1);
        let b = domain_warp2(v2(0.3, 0.4), 4.0, 2.0, 1);
        assert!(a.is_finite() && b.is_finite() && a != b);
    }
}
