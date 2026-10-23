//! Kinetic typography (phase A3).
//!
//! Per-glyph effects for text mobjects (whose children are glyphs): sine waves,
//! decode/scramble reveals, RGB-split glitching with sliced displacement, text
//! laid out along any path, and staggered glyph fly-ins.
use crate::anim::*;
use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::noise::{hash2, hash_unit};
use crate::path::Path;
use crate::rate::Rate;
use crate::scene::{multiply_opacity, Obj};
use crate::style::Blend;
use crate::text::{text_sized, Text};

/// Glyph-level children of a text mobject (or the leaves of any mobject).
pub fn glyphs(m: &Mobject) -> Vec<Mobject> {
    if !m.children.is_empty() && m.children.iter().all(|c| c.children.is_empty()) {
        m.children.clone()
    } else {
        m.leaves()
    }
}

/// Travelling sine wave across the glyphs: vertical bob + slight rotation + scale.
pub fn glyph_wave(m: &Mobject, t: f64, amp: f64, wavelength: f64, speed: f64) -> Mobject {
    let gs = glyphs(m);
    Mobject::group(gs.into_iter().map(|g| {
        let c = g.center();
        let ph = TAU * (c.x / wavelength.max(1e-6) - speed * t);
        let dy = amp * ph.sin();
        g.rotate_about(-0.25 * ph.cos() * amp, c)
            .scale_about(1.0 + 0.15 * amp * ph.sin(), c)
            .shift(v2(0.0, dy))
    }))
}

/// Characters used by [`scramble_text`].
pub const SCRAMBLE_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#$%&*+=<>?/";

/// "Decode" reveal: characters resolve left to right from cycling random glyphs.
/// `t ∈ [0,1]`; monospace so the layout doesn't jump. Unresolved glyphs use `noise`.
pub fn scramble_text(
    target: &str,
    t: f64,
    size: f64,
    color: Color,
    noise: Color,
    seed: u64,
) -> Mobject {
    let chars: Vec<char> = target.chars().collect();
    let pool: Vec<char> = SCRAMBLE_CHARS.chars().collect();
    let n = chars.len().max(1) as f64;
    let tick = (t * 30.0).floor() as i64;
    let mut s = String::new();
    let mut resolved = vec![];
    for (i, ch) in chars.iter().enumerate() {
        let done = t >= 1.0 || t * 1.25 - 0.25 > i as f64 / n;
        resolved.push(done || ch.is_whitespace());
        if done || ch.is_whitespace() {
            s.push(*ch);
        } else {
            let h = hash2(i as i64, tick, seed);
            s.push(pool[(h % pool.len() as u64) as usize]);
        }
    }
    let mut m = Text::new(&s).font("mono").size(size).color(color).build();
    if m.children.len() == 1 && !m.children[0].children.is_empty() {
        m = m.children.into_iter().next().unwrap();
    }
    // colour unresolved glyphs (glyph children skip whitespace)
    let mut k = 0;
    for (i, ch) in s.chars().enumerate() {
        if ch.is_whitespace() {
            continue;
        }
        if let Some(g) = m.children.get_mut(k) {
            if !resolved[i] {
                g.style.fill = noise.into();
            }
        }
        k += 1;
    }
    m
}

/// Digital glitch: horizontal slices jump sideways and an RGB split appears for
/// short random bursts. `intensity` 0 = clean.
pub fn glitch(m: &Mobject, t: f64, intensity: f64, seed: u64) -> Mobject {
    let tick = (t * 24.0).floor() as i64;
    let burst = hash_unit(hash2(tick, 0, seed));
    if intensity <= 0.0 || burst > 0.35 + 0.4 * intensity.min(1.0) {
        return m.clone();
    }
    let b = m.bounds_or_origin();
    let k = intensity * (0.3 + burst);
    let mut out = Mobject::empty();
    // chromatic aberration ghosts (additive)
    for (col, dx) in [(rgb8(255, 40, 60), -0.06), (rgb8(40, 220, 255), 0.06)] {
        let mut ghost = m.clone().shift(v2(dx * k, 0.0));
        ghost.for_each_mut(&mut |n| {
            n.style.fill = col.into();
            n.style.stroke = col.into();
            n.style.blend = Blend::Add;
        });
        out.children.push(multiply_opacity(ghost, 0.7));
    }
    // sliced body
    let slices = 7;
    let h = b.height() / slices as f64;
    for i in 0..slices {
        let r = hash_unit(hash2(tick, i as i64 + 1, seed));
        let dx = if r < 0.4 { (r - 0.2) * 2.0 * k } else { 0.0 };
        let y0 = b.min.y + i as f64 * h;
        let mut slice = m.clone().shift(v2(dx, 0.0));
        let lo = v2(b.min.x - 2.0, y0);
        let hi = v2(b.max.x + 2.0, y0 + h + 1e-3);
        slice.for_each_mut(&mut |n| n.style.clip = Some((lo, hi)));
        out.children.push(slice);
    }
    out
}

/// Lay `s` out along `path`, starting at arc-length proportion `offset`, glyphs
/// rotated to the tangent. Characters past the end are dropped.
pub fn text_on_path(s: &str, path: &Path, size: f64, color: Color, offset: f64) -> Mobject {
    let total = path.length().max(1e-9);
    let mut pos = offset * total;
    let mut out = Mobject::empty();
    for ch in s.chars() {
        let adv = if ch == ' ' {
            size * 0.3
        } else {
            let g = text_sized(&ch.to_string(), size, color);
            let w = g.width();
            let mid = (pos + w / 2.0) / total;
            if mid > 1.0 {
                break;
            }
            if mid >= 0.0 {
                let p = path.point_from_proportion(mid);
                let tan = path.tangent_from_proportion(mid);
                let base = g.center();
                // keep the baseline on the path: centre sits size*0.3 above it
                out.children.push(
                    g.shift(-base + v2(0.0, size * 0.3))
                        .rotate_about(tan.angle(), Vec2::ZERO)
                        .shift(p),
                );
            }
            w + size * 0.06
        };
        pos += adv;
    }
    out
}

/// Glyphs fly in one after another from `offset`, spinning by `spin` and scaling
/// up from `scale`, with an overshoot ease.
pub fn glyph_fly_in(o: Obj, offset: Vec2, spin: f64, scale: f64, lag: f64) -> Animation {
    let mut a = custom(o, move |m, t| {
        glyph_fly_frame(m, t, offset, spin, scale, lag)
    });
    if let AnimKind::Custom { introduces, .. } = &mut a.kind {
        *introduces = true;
    }
    a.rate(Rate::Linear).run_time(1.8)
}

/// Pure frame of [`glyph_fly_in`].
pub fn glyph_fly_frame(
    m: &Mobject,
    t: f64,
    offset: Vec2,
    spin: f64,
    scale: f64,
    lag: f64,
) -> Mobject {
    let gs = glyphs(m);
    let n = gs.len().max(1);
    let span = 1.0 + lag * (n - 1) as f64;
    Mobject::group(gs.into_iter().enumerate().map(|(i, g)| {
        let u = ((t * span - lag * i as f64).clamp(0.0, 1.0)).min(1.0);
        let e = Rate::OutBack.eval(u);
        let c = g.center();
        let k = 1.0 - e;
        multiply_opacity(
            g.scale_about(lerp(scale, 1.0, e).max(0.0), c)
                .rotate_about(spin * k, c)
                .shift(offset * k),
            smoothstep(0.0, 0.4, u) as f32,
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::bold_text;

    #[test]
    fn scramble_resolves() {
        let a = scramble_text("HELLO WORLD", 1.0, 0.5, WHITE, GREEN, 1);
        let b = Text::new("HELLO WORLD")
            .font("mono")
            .size(0.5)
            .color(WHITE)
            .build();
        assert!((a.width() - b.width()).abs() < 1e-6);
        let mid = scramble_text("HELLO WORLD", 0.5, 0.5, WHITE, GREEN, 1);
        assert_eq!(mid.children.len(), 10);
        // monospace: same width mid-scramble
        assert!((mid.width() - a.width()).abs() < 0.05);
    }

    #[test]
    fn wave_and_fly_in_endpoints() {
        let m = bold_text("Wave", 0.6, WHITE);
        let w = glyph_wave(&m, 0.0, 0.0, 1.0, 1.0);
        assert!(w.center().distance(m.center()) < 1e-9);
        let f = glyph_fly_frame(&m, 1.0, v2(0.0, 3.0), 1.0, 0.2, 0.1);
        assert!(f.center().distance(m.center()) < 1e-6);
        let f0 = glyph_fly_frame(&m, 0.0, v2(0.0, 3.0), 1.0, 0.2, 0.1);
        assert!(f0.center().y > m.center().y + 2.0);
    }

    #[test]
    fn text_follows_path() {
        let mut p = Path::new();
        p.move_to(v2(-3.0, 0.0)).line_to(v2(3.0, 0.0));
        let m = text_on_path("abc", &p, 0.5, WHITE, 0.0);
        assert_eq!(m.children.len(), 3);
        assert!(m.children[0].center().x < m.children[2].center().x);
        assert!(glitch(&m, 0.0, 0.0, 1) == m);
    }
}
