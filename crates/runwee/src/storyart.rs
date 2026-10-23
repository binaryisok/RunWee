//! Story art styles (phase T2).
//!
//! Turn plain vector story art into distinct, hand-made looks — every function is a
//! pure function of its inputs (and time `t`) so it works inside `always_redraw`:
//!
//! * **Paper cutout** — [`cutout`] (hand-cut wobble, white paper rim, drop shadow),
//!   [`paper_fibres`], [`torn_strip`], [`cut_paper_hills`].
//! * **Watercolor** — [`watercolor`] (layered translucent noise-deformed glazes with
//!   darkened edges), [`watercolor_wash`] (granulating gradient wash).
//! * **Ink & brush** — [`brush_stroke`] (tapered, dry-brush bristles, progressive
//!   reveal), [`ink_outline`], [`ink_mountains`], [`seal_stamp`].
//! * **Shadow puppets** — [`silhouette`], [`lamp_screen`], [`puppet_rods`].
//! * **Flat vector** — [`long_shadow`], [`recolor`].
//! * **Folk-art borders** — [`madhubani_border`], [`warli_border`],
//!   [`warli_figure`], [`warli_circle_dance`], [`gond_border`], [`gond_leaf`],
//!   [`gond_tree`], [`folk_frame`].
//! * **Ambience** — [`rain`], [`falling_leaves`], [`fireflies`], [`snowfall`],
//!   [`sky_colors`] (day–night cycle), [`night_overlay`].
use crate::color::*;
use crate::generative::field_image;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::noise::{fbm2, hash2, hash_unit};
use crate::path::{Path, Rect};
use crate::shapes::*;
use crate::story::{font_text, hand_text, shade, tint};
use crate::style::{Blend, Paint, Shadow};

/// Deterministic random number in [0, 1).
pub fn rnd(i: i64, j: i64, seed: u64) -> f64 {
    hash_unit(hash2(i, j, seed))
}

fn fract(x: f64) -> f64 {
    x - x.floor()
}

/// Map every solid fill/stroke colour of `m` through `f` (gradients are left alone).
pub fn recolor(m: Mobject, f: impl Fn(Color) -> Color) -> Mobject {
    m.map_style(|s| {
        if let Paint::Solid(c) = s.fill {
            s.fill = Paint::Solid(f(c));
        }
        if let Paint::Solid(c) = s.stroke {
            s.stroke = Paint::Solid(f(c));
        }
    })
}

/// Subdivide every subpath so that it has roughly `per_unit` segments per scene unit
/// (at least 8), so later deformations look organic rather than polygonal.
pub fn densify(mut m: Mobject, per_unit: f64) -> Mobject {
    m.for_each_mut(&mut |leaf| {
        for sp in leaf.path.subpaths.iter_mut() {
            let n = ((sp.length() * per_unit).ceil() as usize).clamp(8, 400);
            if n > sp.segs.len() {
                *sp = sp.resample(n);
            }
        }
    });
    m
}

/// Displace every point by a smooth noise field (amplitude `amp`, spatial frequency `freq`).
pub fn wobble(m: Mobject, amp: f64, freq: f64, seed: u64) -> Mobject {
    m.apply_function(move |p| {
        let q = p * freq;
        let dx = fbm2(q, 3, 2.0, 0.5, seed);
        let dy = fbm2(q + v2(17.3, -9.1), 3, 2.0, 0.5, seed.wrapping_add(7));
        p + v2(dx, dy) * amp
    })
}

// ------------------------------------------------------------------ paper cutout

/// Options for [`cutout`].
#[derive(Clone, Copy, Debug)]
pub struct CutoutStyle {
    /// Colour of the thin paper rim left by the scissors.
    pub edge: Color,
    /// Rim width in pixels (at 1080p).
    pub edge_width: f32,
    /// Drop-shadow offset (scene units) — larger means the layer floats higher.
    pub lift: f64,
    /// Hand-cut wobble amplitude.
    pub wobble: f64,
    pub seed: u64,
}

impl Default for CutoutStyle {
    fn default() -> Self {
        Self {
            edge: rgb8(255, 252, 244),
            edge_width: 3.0,
            lift: 0.05,
            wobble: 0.025,
            seed: 3,
        }
    }
}

impl CutoutStyle {
    pub fn lift(mut self, l: f64) -> Self {
        self.lift = l;
        self
    }
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }
    pub fn wobble(mut self, w: f64) -> Self {
        self.wobble = w;
        self
    }
    pub fn edge(mut self, c: Color, w: f32) -> Self {
        self.edge = c;
        self.edge_width = w;
        self
    }
}

/// Make `m` look like layered cut paper: hand-cut wobbly edges, a pale paper rim and a
/// soft drop shadow underneath, as if it were pinned slightly above the page.
pub fn cutout(m: Mobject, st: CutoutStyle) -> Mobject {
    let m = densify(m, 14.0);
    let m = if st.wobble > 0.0 {
        wobble(m, st.wobble, 2.3, st.seed)
    } else {
        m
    };
    let lift = st.lift;
    m.map_style(move |s| {
        if s.fill_opacity > 0.0 {
            s.background_stroke = st.edge;
            s.background_stroke_width = st.edge_width;
            if lift > 0.0 {
                s.shadow = Some(Shadow {
                    offset: v2(lift * 0.6, -lift),
                    blur: lift * 1.6 + 0.02,
                    color: BLACK.with_alpha(0.35),
                });
            }
        }
    })
}

/// A paper-fibre texture overlay (multiply it over a scene for a craft-paper feel).
pub fn paper_fibres(rect: Rect, base: Color, seed: u64, px_w: u32) -> Mobject {
    field_image(rect, px_w, move |p| {
        let fib = fbm2(v2(p.x * 1.5, p.y * 22.0), 3, 2.0, 0.5, seed).abs();
        let mott = fbm2(p * 0.8, 3, 2.0, 0.5, seed + 1);
        let grain = hash_unit(hash2((p.x * 300.0) as i64, (p.y * 300.0) as i64, seed));
        let k = (0.92 + mott * 0.06 - fib * 0.08 + (grain - 0.5) * 0.05).clamp(0.6, 1.05) as f32;
        Color {
            r: (base.r * k).min(1.0),
            g: (base.g * k).min(1.0),
            b: (base.b * k).min(1.0),
            a: 1.0,
        }
    })
}

/// A horizontal strip of torn paper: flat bottom at `rect.min.y`, ragged top edge
/// around `rect.max.y`, with a fibrous white tear line.
pub fn torn_strip(rect: Rect, color: Color, seed: u64) -> Mobject {
    let n = ((rect.width() * 18.0) as usize).max(12);
    let mut top = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let x = rect.min.x + rect.width() * i as f64 / n as f64;
        let jag = (rnd(i as i64, 0, seed) - 0.5) * 0.06;
        let roll = fbm2(v2(x * 0.9, 0.0), 3, 2.0, 0.5, seed) * 0.18;
        top.push(v2(x, rect.max.y + roll + jag));
    }
    let mut body = top.clone();
    body.push(v2(rect.max.x, rect.min.y));
    body.push(v2(rect.min.x, rect.min.y));
    let rim: Vec<Vec2> = top.iter().map(|p| *p + v2(0.0, 0.025)).collect();
    let mut rim_poly = rim.clone();
    for p in top.iter().rev() {
        rim_poly.push(*p - v2(0.0, 0.01));
    }
    group(vec![
        polygon(&rim_poly)
            .fill(rgb8(255, 253, 246), 1.0)
            .no_stroke(),
        polygon(&body).fill(color, 1.0).no_stroke(),
    ])
    .map_style(|s| {
        s.shadow = Some(Shadow {
            offset: v2(0.02, -0.04),
            blur: 0.08,
            color: BLACK.with_alpha(0.3),
        })
    })
}

/// Rolling hills cut from coloured paper, back (first colour) to front.
pub fn cut_paper_hills(rect: Rect, colors: &[Color], seed: u64) -> Mobject {
    let n = colors.len().max(1);
    group(colors.iter().enumerate().map(|(i, c)| {
        let base = rect.min.y + rect.height() * (0.55 - 0.4 * i as f64 / n as f64);
        let hill = crate::story::hills(rect, base, 0.4 - 0.06 * i as f64, *c, seed + i as u64);
        cutout(
            hill,
            CutoutStyle::default()
                .seed(seed + 11 * i as u64)
                .lift(0.04 + 0.015 * i as f64),
        )
    }))
}

// ------------------------------------------------------------------ watercolor

/// Paint `m` as a watercolor glaze: `layers` translucent copies, each deformed by its
/// own noise field so edges bleed irregularly, plus a faint darker pigment rim.
pub fn watercolor(m: Mobject, color: Color, layers: usize, seed: u64) -> Mobject {
    let base = densify(m, 16.0);
    let layers = layers.max(1);
    let size = base
        .family_path()
        .bounds()
        .map(|r| r.width().max(r.height()))
        .unwrap_or(1.0);
    let mut out = Vec::with_capacity(layers + 1);
    for k in 0..layers {
        let amp = size * (0.05 + 0.04 * rnd(k as i64, 1, seed));
        let layer = wobble(
            base.clone(),
            amp,
            1.6 / size.max(0.2) + 0.8,
            seed + 31 * k as u64,
        );
        let hue = color.mix(shade(color, 0.25), rnd(k as i64, 2, seed) as f32 * 0.5);
        out.push(layer.fill(hue, 0.16).no_stroke());
    }
    let rim = wobble(base, size * 0.03, 2.0, seed + 999)
        .fill(color, 0.0)
        .stroke(shade(color, 0.3), 1.6)
        .map_style(|s| s.stroke_opacity = 0.35);
    out.push(rim);
    group(out)
}

/// A granulating watercolor wash from `top` to `bottom` filling `rect`, with
/// blooms ("cauliflowers"), pigment granulation and soft irregular margins.
pub fn watercolor_wash(rect: Rect, top: Color, bottom: Color, seed: u64, px_w: u32) -> Mobject {
    let (y0, h) = (rect.min.y, rect.height().max(1e-6));
    let c = rect.center();
    let (hw, hh) = (rect.width() * 0.5, rect.height() * 0.5);
    field_image(rect, px_w, move |p| {
        let t = ((p.y - y0) / h + fbm2(p * 0.7, 3, 2.0, 0.5, seed) * 0.15).clamp(0.0, 1.0);
        let col = bottom.mix(top, t as f32);
        let bloom = fbm2(p * 1.8, 4, 2.0, 0.55, seed + 3);
        let gran = hash_unit(hash2((p.x * 260.0) as i64, (p.y * 260.0) as i64, seed + 5));
        let dens = (0.78 + bloom * 0.35 + (gran - 0.5) * 0.12).clamp(0.3, 1.2);
        // irregular margin: fade near the rectangle edge, modulated by noise
        let q = v2((p.x - c.x).abs(), (p.y - c.y).abs());
        let edge = ((1.0 - q.x / hw).min(1.0 - q.y / hh) * 9.0
            + fbm2(p * 3.0, 2, 2.0, 0.5, seed + 9) * 1.5)
            .clamp(0.0, 1.0);
        let rim = 1.0 + (1.0 - edge) * 0.25 * edge * 4.0; // pigment pools at the edge
        Color {
            a: (col.a as f64 * dens * edge * rim).clamp(0.0, 1.0) as f32,
            ..col
        }
    })
}

// ------------------------------------------------------------------ ink & brush

/// A tapered brush stroke through `pts` (smoothly interpolated), `width` at its widest.
/// `dry` ∈ [0, 1] splits it into bristle streaks with gaps (dry-brush). `p` ∈ [0, 1]
/// reveals the stroke progressively as if being painted.
pub fn brush_stroke(
    pts: &[Vec2],
    width: f64,
    color: Color,
    dry: f64,
    seed: u64,
    p: f64,
) -> Mobject {
    if pts.len() < 2 || p <= 0.0 {
        return Mobject::empty();
    }
    let path = Path::smooth_through(pts, false);
    let len = path.length().max(1e-6);
    let n = ((len * 40.0) as usize).clamp(16, 600);
    let p = p.clamp(0.0, 1.0);
    let profile = |s: f64| -> f64 {
        // fast attack, long tail like a real brush; plus pressure noise
        let attack = (s / 0.08).min(1.0).powf(0.6);
        let tail = (1.0 - s).max(0.0).powf(0.45);
        let press = 1.0 + 0.18 * fbm2(v2(s * 6.0, 0.0), 2, 2.0, 0.5, seed);
        width * 0.5 * attack * tail * press
    };
    let m_end = ((n as f64) * p).ceil() as usize;
    let samples: Vec<(Vec2, Vec2, f64)> = (0..=m_end.max(1))
        .map(|i| {
            let s = (i as f64 / n as f64).min(p);
            let c = path.point_from_proportion(s);
            let tan = path.tangent_from_proportion(s);
            let nrm = if tan.length() < 1e-9 {
                v2(0.0, 1.0)
            } else {
                v2(-tan.y, tan.x).normalize()
            };
            (c, nrm, profile(s))
        })
        .collect();
    let ribbon = |lo: f64, hi: f64| -> Vec<Vec2> {
        let mut left: Vec<Vec2> = samples
            .iter()
            .map(|(c, nr, w)| *c + *nr * (*w * hi))
            .collect();
        let right: Vec<Vec2> = samples
            .iter()
            .rev()
            .map(|(c, nr, w)| *c + *nr * (*w * lo))
            .collect();
        left.extend(right);
        left
    };
    if dry <= 0.0 {
        return polygon(&ribbon(-1.0, 1.0)).fill(color, 1.0).no_stroke();
    }
    // dry brush: bristle bands; each band drops out where its own noise is low.
    let bands = 7;
    let mut out = vec![];
    for b in 0..bands {
        let lo = -1.0 + 2.0 * b as f64 / bands as f64;
        let hi = lo + 2.0 / bands as f64 * 0.92;
        let mut run: Vec<usize> = vec![];
        let flush = |run: &mut Vec<usize>, out: &mut Vec<Mobject>| {
            if run.len() >= 2 {
                let mut poly: Vec<Vec2> = run
                    .iter()
                    .map(|&i| samples[i].0 + samples[i].1 * (samples[i].2 * hi))
                    .collect();
                poly.extend(
                    run.iter()
                        .rev()
                        .map(|&i| samples[i].0 + samples[i].1 * (samples[i].2 * lo)),
                );
                out.push(polygon(&poly).fill(color, 1.0).no_stroke());
            }
            run.clear();
        };
        for i in 0..samples.len() {
            let s = i as f64 / n as f64;
            // gaps grow towards the end of the stroke (the brush runs out of ink)
            let v = fbm2(v2(s * 9.0, b as f64 * 3.7), 2, 2.0, 0.5, seed + b as u64) + 0.5;
            if v > dry * (0.25 + 0.75 * s) {
                run.push(i);
            } else {
                flush(&mut run, &mut out);
            }
        }
        flush(&mut run, &mut out);
    }
    group(out)
}

/// Re-draw every subpath outline of `m` as a tapered ink brush line.
pub fn ink_outline(m: &Mobject, width: f64, color: Color, seed: u64) -> Mobject {
    let mut out = vec![];
    let mut k = 0u64;
    let mut visit = |leaf: &Mobject| {
        for sp in &leaf.path.subpaths {
            if sp.segs.is_empty() {
                continue;
            }
            let n = ((sp.length() * 10.0) as usize).clamp(6, 80);
            // start each closed loop at a random point so tapers don't line up
            let off = rnd(k as i64, 3, seed);
            let mut pts: Vec<Vec2> = (0..=n)
                .map(|i| sp.point_at_length_prop(fract(off + i as f64 / n as f64 * 0.97)))
                .collect();
            if !sp.closed {
                pts = (0..=n)
                    .map(|i| sp.point_at_length_prop(i as f64 / n as f64))
                    .collect();
            }
            out.push(brush_stroke(&pts, width, color, 0.25, seed + k, 1.0));
            k += 1;
        }
    };
    fn walk(m: &Mobject, f: &mut dyn FnMut(&Mobject)) {
        if !m.path.subpaths.is_empty() {
            f(m);
        }
        for c in &m.children {
            walk(c, f);
        }
    }
    walk(m, &mut visit);
    group(out)
}

/// Misty ink-wash mountain ranges (sumi-e style): `layers` ridges fading into the
/// distance, painted progressively as `p` goes 0 → 1.
pub fn ink_mountains(rect: Rect, layers: usize, ink: Color, seed: u64, p: f64) -> Mobject {
    let layers = layers.max(1);
    let mut out = vec![];
    for l in 0..layers {
        let lp = ((p * layers as f64) - l as f64).clamp(0.0, 1.0);
        if lp <= 0.0 {
            continue;
        }
        let depth = l as f64 / layers as f64; // 0 = far
        let base = rect.min.y + rect.height() * (0.42 - 0.3 * depth);
        let amp = rect.height() * (0.46 - 0.2 * depth);
        let n = 90;
        let mut ridge = vec![];
        let mut peak_y = base;
        for i in 0..=n {
            let x = rect.min.x + rect.width() * i as f64 / n as f64;
            let fx = x * (0.13 + 0.09 * depth) + l as f64 * 7.1;
            // ridged noise gives sharp peaks and soft valleys
            let ridged =
                (1.0 - fbm2(v2(fx, 0.0), 3, 2.0, 0.5, seed + l as u64).abs() * 2.2).max(0.0);
            let detail = fbm2(v2(fx * 6.0, 1.0), 2, 2.0, 0.5, seed + 9) * 0.05;
            let y = base + amp * (ridged * ridged + detail);
            peak_y = peak_y.max(y);
            ridge.push(v2(x, y));
        }
        let shown = ((ridge.len() as f64) * lp).ceil() as usize;
        let ridge = &ridge[..shown.max(2)];
        let mut body: Vec<Vec2> = ridge.to_vec();
        let last = ridge[ridge.len() - 1];
        // mist: the wash fades downward via a vertical gradient
        body.push(v2(last.x, rect.min.y));
        body.push(v2(rect.min.x, rect.min.y));
        let alpha = 0.22 + 0.6 * depth as f32;
        let top_y = peak_y;
        let fade_y = base - rect.height() * 0.12;
        let wash = polygon(&body)
            .fill(ink, alpha)
            .no_stroke()
            .map_style(move |s| {
                s.fill = Paint::linear(
                    v2(0.0, top_y),
                    v2(0.0, fade_y),
                    &[
                        ink.with_alpha(alpha),
                        ink.with_alpha(alpha * 0.7),
                        ink.with_alpha(0.0),
                    ],
                )
            });
        out.push(wash);
        out.push(brush_stroke(
            ridge,
            0.04 + 0.04 * depth,
            ink.with_alpha(0.5 + 0.45 * depth as f32),
            0.5,
            seed + 50 + l as u64,
            1.0,
        ));
    }
    group(out)
}

/// A square red artist's seal (chop) carrying `text` (one or two characters works best).
pub fn seal_stamp(text: &str, pos: Vec2, size: f64, color: Color, seed: u64) -> Mobject {
    let sq = densify(rounded_rectangle(size, size, size * 0.08), 30.0);
    let sq = wobble(sq, size * 0.03, 6.0, seed)
        .move_to(pos)
        .fill(color, 0.92)
        .no_stroke();
    let inner = rectangle(size * 0.8, size * 0.8)
        .move_to(pos)
        .fill(color, 0.0)
        .stroke(rgb8(250, 240, 225), 1.6);
    let label = font_text(text, "story", size * 0.42, rgb8(252, 244, 230)).move_to(pos);
    group(vec![sq, inner, label])
}

// ------------------------------------------------------------------ shadow puppets

/// Paint everything in `m` a single silhouette colour (keeps relative opacity).
pub fn silhouette(m: Mobject, color: Color) -> Mobject {
    m.map_style(move |s| {
        s.fill = Paint::Solid(color);
        if s.stroke_width > 0.0 {
            s.stroke = Paint::Solid(color);
        }
        s.background_stroke_width = 0.0;
        s.glow = 0.0;
        s.shadow = None;
    })
}

/// A back-lit cloth screen for shadow-puppet theatre: warm radial lamp glow with a
/// gentle flicker over time `t`, faint weave and a carved wooden frame.
pub fn lamp_screen(rect: Rect, t: f64, warm: Color, seed: u64) -> Mobject {
    let c = rect.center();
    let flick = 1.0
        + 0.035 * (t * 7.3).sin()
        + 0.025 * (t * 13.1 + 1.0).sin()
        + 0.02 * fbm2(v2(t * 3.0, 0.5), 2, 2.0, 0.5, seed);
    let lamp = c + v2(0.25 * (t * 0.7).sin(), 0.15);
    let r = rect.width().max(rect.height()) * 0.62 * flick;
    let glow = rectangle(rect.width(), rect.height())
        .move_to(c)
        .fill(warm, 1.0)
        .no_stroke()
        .map_style(move |s| {
            s.fill = Paint::radial(
                lamp,
                r,
                &[tint(warm, 0.55), warm, shade(warm, 0.45), shade(warm, 0.8)],
            )
        });
    let mut weave = vec![];
    let step = 0.09;
    let mut x = rect.min.x;
    while x < rect.max.x {
        weave.push(
            line(v2(x, rect.min.y), v2(x, rect.max.y))
                .stroke(BLACK, 0.6)
                .map_style(|s| s.stroke_opacity = 0.05),
        );
        x += step;
    }
    let mut y = rect.min.y;
    while y < rect.max.y {
        weave.push(
            line(v2(rect.min.x, y), v2(rect.max.x, y))
                .stroke(BLACK, 0.6)
                .map_style(|s| s.stroke_opacity = 0.04),
        );
        y += step;
    }
    let wood = rgb8(92, 52, 30);
    let fr = 0.16;
    let frame = group(vec![
        rectangle(rect.width() + 2.0 * fr, fr).move_to(v2(c.x, rect.max.y + fr * 0.5)),
        rectangle(rect.width() + 2.0 * fr, fr).move_to(v2(c.x, rect.min.y - fr * 0.5)),
        rectangle(fr, rect.height()).move_to(v2(rect.min.x - fr * 0.5, c.y)),
        rectangle(fr, rect.height()).move_to(v2(rect.max.x + fr * 0.5, c.y)),
    ])
    .fill(wood, 1.0)
    .stroke(shade(wood, 0.4), 2.0);
    let mut studs = vec![];
    let n = 14;
    for i in 0..=n {
        let x = rect.min.x - fr * 0.5 + (rect.width() + fr) * i as f64 / n as f64;
        for yy in [rect.max.y + fr * 0.5, rect.min.y - fr * 0.5] {
            studs.push(
                circle(0.025)
                    .move_to(v2(x, yy))
                    .fill(rgb8(222, 170, 80), 1.0)
                    .no_stroke(),
            );
        }
    }
    group(vec![glow, group(weave), frame, group(studs)])
}

/// Thin control rods from the puppet's hands (`holds`) down to below the screen bottom.
pub fn puppet_rods(holds: &[Vec2], bottom: f64, color: Color) -> Mobject {
    group(holds.iter().enumerate().map(|(i, h)| {
        let foot = v2(h.x + 0.35 * (i as f64 - 0.5), bottom - 0.3);
        line(*h, foot).stroke(color, 2.2)
    }))
}

/// Soften a silhouette as if it were `dist` (0 = pressed on the cloth) from the screen.
pub fn screen_blur(m: Mobject, color: Color, dist: f64) -> Mobject {
    m.map_style(move |s| {
        s.shadow = Some(Shadow {
            offset: v2(0.0, 0.0),
            blur: 0.02 + dist * 0.25,
            color: color.with_alpha(0.9),
        });
        s.opacity *= (1.0 - dist * 0.4).clamp(0.3, 1.0) as f32;
    })
}

// ------------------------------------------------------------------ flat vector

/// A flat-design "long shadow": `m` extruded along `angle` for `len` in `color`.
pub fn long_shadow(m: &Mobject, angle: f64, len: f64, color: Color) -> Mobject {
    let steps = 24;
    let dir = Vec2::polar(1.0, angle);
    let sil = silhouette(m.clone(), color);
    group(
        (1..=steps)
            .rev()
            .map(|i| sil.clone().shift(dir * (len * i as f64 / steps as f64))),
    )
}

// ------------------------------------------------------------------ folk-art borders

/// Folk painting traditions available for [`folk_frame`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolkStyle {
    /// Mithila (Bihar): double ink lines, fish and lotus motifs, bright fills.
    Madhubani,
    /// Warli (Maharashtra): white stick figures of triangles on terracotta.
    Warli,
    /// Gond (Madhya Pradesh): leaves and forms filled with dots and dashes.
    Gond,
}

impl FolkStyle {
    pub fn parse(s: &str) -> FolkStyle {
        match s.to_ascii_lowercase().as_str() {
            "warli" => FolkStyle::Warli,
            "gond" => FolkStyle::Gond,
            _ => FolkStyle::Madhubani,
        }
    }
}

/// Where motifs sit around a frame: `(center, rotation)` for cells of size `cell`
/// along the band midline; corners are excluded (they get their own motif).
pub fn border_cells(rect: Rect, band: f64, cell: f64) -> Vec<(Vec2, f64)> {
    let mut out = vec![];
    let inner = Rect::new(rect.min + v2(band, band), rect.max - v2(band, band));
    let mid = band * 0.5;
    let mut side = |a: Vec2, b: Vec2, rot: f64| {
        let len = (b - a).length();
        let n = (len / cell).floor().max(1.0) as usize;
        let dir = (b - a) / len;
        let step = len / n as f64;
        for i in 0..n {
            out.push((a + dir * (step * (i as f64 + 0.5)), rot));
        }
    };
    let (x0, x1, y0, y1) = (inner.min.x, inner.max.x, inner.min.y, inner.max.y);
    side(v2(x0, rect.max.y - mid), v2(x1, rect.max.y - mid), 0.0); // top
    side(v2(x1, rect.min.y + mid), v2(x0, rect.min.y + mid), PI); // bottom
    side(
        v2(rect.max.x - mid, y1),
        v2(rect.max.x - mid, y0),
        -PI / 2.0,
    ); // right
    side(v2(rect.min.x + mid, y0), v2(rect.min.x + mid, y1), PI / 2.0); // left
    out
}

fn corners(rect: Rect, band: f64) -> [Vec2; 4] {
    let m = band * 0.5;
    [
        v2(rect.min.x + m, rect.max.y - m),
        v2(rect.max.x - m, rect.max.y - m),
        v2(rect.max.x - m, rect.min.y + m),
        v2(rect.min.x + m, rect.min.y + m),
    ]
}

fn ring(rect: Rect, band: f64) -> Mobject {
    let mut path = Path::polyline(
        &[
            rect.min,
            v2(rect.max.x, rect.min.y),
            rect.max,
            v2(rect.min.x, rect.max.y),
        ],
        true,
    );
    let i0 = rect.min + v2(band, band);
    let i1 = rect.max - v2(band, band);
    let inner = Path::polyline(&[i0, v2(i0.x, i1.y), i1, v2(i1.x, i0.y)], true);
    path.subpaths.extend(inner.subpaths);
    Mobject::from_path(path).map_style(|s| s.fill_rule = crate::style::FillRule::EvenOdd)
}

fn outline_rect(r: Rect, color: Color, w: f32) -> Mobject {
    rectangle(r.width(), r.height())
        .move_to(r.center())
        .fill(color, 0.0)
        .stroke(color, w)
}

/// A Madhubani fish (local frame: nose at +x), `s` long.
pub fn madhubani_fish(s: f64, body: Color, ink: Color) -> Mobject {
    let b = smooth_curve(
        &[
            v2(0.5 * s, 0.0),
            v2(0.1 * s, 0.2 * s),
            v2(-0.3 * s, 0.1 * s),
            v2(-0.3 * s, -0.1 * s),
            v2(0.1 * s, -0.2 * s),
        ],
        true,
    )
    .fill(body, 1.0)
    .stroke(ink, 2.0);
    let tail = polygon(&[
        v2(-0.28 * s, 0.0),
        v2(-0.5 * s, 0.17 * s),
        v2(-0.46 * s, 0.0),
        v2(-0.5 * s, -0.17 * s),
    ])
    .fill(shade(body, 0.2), 1.0)
    .stroke(ink, 2.0);
    let mut scales = vec![];
    for k in 0..3 {
        let x = -0.15 * s + 0.12 * s * k as f64;
        scales.push(
            arc(0.07 * s, -PI / 2.0, PI)
                .move_to(v2(x, 0.0))
                .fill(body, 0.0)
                .stroke(ink, 1.4),
        );
    }
    let gill = arc(0.12 * s, -PI / 2.6, PI / 1.3)
        .shift(v2(0.22 * s, 0.0))
        .fill(body, 0.0)
        .stroke(ink, 1.6);
    let eye = group(vec![
        circle(0.055 * s)
            .move_to(v2(0.36 * s, 0.04 * s))
            .fill(WHITE, 1.0)
            .stroke(ink, 1.4),
        circle(0.025 * s)
            .move_to(v2(0.37 * s, 0.04 * s))
            .fill(ink, 1.0)
            .no_stroke(),
    ]);
    group(vec![tail, b, group(scales), gill, eye])
}

/// A Madhubani lotus / sun-flower rosette for corners.
pub fn madhubani_lotus(r: f64, petals: usize, fill: Color, centre: Color, ink: Color) -> Mobject {
    let mut parts = vec![];
    for i in 0..petals {
        let a = TAU * i as f64 / petals as f64;
        let tip = Vec2::polar(r, a);
        let l = Vec2::polar(r * 0.38, a + 0.45);
        let rr = Vec2::polar(r * 0.38, a - 0.45);
        parts.push(
            smooth_curve(&[v2(0.0, 0.0), l, tip, rr], true)
                .fill(fill, 1.0)
                .stroke(ink, 1.6),
        );
        parts.push(line(Vec2::polar(r * 0.3, a), Vec2::polar(r * 0.75, a)).stroke(ink, 1.0));
    }
    parts.push(circle(r * 0.32).fill(centre, 1.0).stroke(ink, 1.6));
    parts.push(circle(r * 0.14).fill(ink, 1.0).no_stroke());
    group(parts)
}

/// A Mithila-style (Madhubani) frame: double ink lines, a band of alternating fish and
/// bud motifs in red / yellow / green / blue, lotus corners and a sawtooth inner trim.
pub fn madhubani_border(rect: Rect, band: f64, ground: Color) -> Mobject {
    let ink = rgb8(30, 22, 20);
    let fills = [
        rgb8(214, 54, 44),
        rgb8(244, 186, 40),
        rgb8(40, 140, 90),
        rgb8(36, 92, 168),
    ];
    let mut parts = vec![ring(rect, band).fill(ground, 1.0).no_stroke()];
    parts.push(outline_rect(rect, ink, 3.0));
    parts.push(outline_rect(
        Rect::new(rect.min + v2(0.04, 0.04), rect.max - v2(0.04, 0.04)),
        ink,
        1.4,
    ));
    let inner = Rect::new(rect.min + v2(band, band), rect.max - v2(band, band));
    parts.push(outline_rect(inner, ink, 3.0));
    for (i, (c, rot)) in border_cells(rect, band, band * 1.25)
        .into_iter()
        .enumerate()
    {
        let col = fills[i % fills.len()];
        let motif = if i % 2 == 0 {
            madhubani_fish(band * 0.95, col, ink)
                .rotate_about(if (i / 2) % 2 == 0 { 0.0 } else { PI }, v2(0.0, 0.0))
        } else {
            // a tri-petal bud on a stem
            let s = band;
            group(vec![
                line(v2(0.0, -0.32 * s), v2(0.0, 0.05 * s)).stroke(ink, 1.6),
                madhubani_lotus(0.2 * s, 3, col, fills[(i + 1) % 4], ink)
                    .rotate_about(PI / 2.0, v2(0.0, 0.0))
                    .shift(v2(0.0, 0.1 * s)),
                circle(0.04 * s)
                    .move_to(v2(-0.2 * s, -0.25 * s))
                    .fill(ink, 1.0)
                    .no_stroke(),
                circle(0.04 * s)
                    .move_to(v2(0.2 * s, -0.25 * s))
                    .fill(ink, 1.0)
                    .no_stroke(),
            ])
        };
        parts.push(motif.rotate_about(rot, v2(0.0, 0.0)).shift(c));
    }
    for (k, c) in corners(rect, band).into_iter().enumerate() {
        parts.push(madhubani_lotus(band * 0.42, 8, fills[k % 4], fills[(k + 1) % 4], ink).shift(c));
    }
    // sawtooth trim just inside the band
    let tooth = band * 0.18;
    let mut teeth = vec![];
    let mut x = inner.min.x;
    let mut k = 0;
    while x + tooth <= inner.max.x + 1e-9 {
        for (y, dir) in [(inner.max.y, -1.0), (inner.min.y, 1.0)] {
            teeth.push(
                polygon(&[
                    v2(x, y),
                    v2(x + tooth, y),
                    v2(x + tooth * 0.5, y + dir * tooth),
                ])
                .fill(if k % 2 == 0 { fills[0] } else { ink }, 1.0)
                .no_stroke(),
            );
        }
        x += tooth;
        k += 1;
    }
    let mut y = inner.min.y + tooth;
    while y + tooth <= inner.max.y - tooth + 1e-9 {
        for (xx, dir) in [(inner.min.x, 1.0), (inner.max.x, -1.0)] {
            teeth.push(
                polygon(&[
                    v2(xx, y),
                    v2(xx, y + tooth),
                    v2(xx + dir * tooth, y + tooth * 0.5),
                ])
                .fill(if k % 2 == 0 { fills[0] } else { ink }, 1.0)
                .no_stroke(),
            );
        }
        y += tooth;
        k += 1;
    }
    parts.push(group(teeth));
    group(parts)
}

/// Joint positions of a Warli figure (for hand-holding chains).
#[derive(Clone, Copy, Debug)]
pub struct WarliPose {
    pub head: Vec2,
    pub left_hand: Vec2,
    pub right_hand: Vec2,
}

/// The arm reach of a Warli figure of height `s` (half the hand-to-hand span).
pub fn warli_reach(s: f64) -> f64 {
    0.36 * s
}

fn warli_parts(pos: Vec2, s: f64, t: f64, phase: f64, color: Color) -> (Mobject, WarliPose) {
    let bob = 0.03 * s * (t * 6.0 + phase).sin();
    let waist = pos + v2(0.0, bob);
    let sh = waist + v2(0.0, 0.26 * s);
    let hip = waist - v2(0.0, 0.2 * s);
    let head = sh + v2(0.0, 0.13 * s);
    let up = polygon(&[waist, sh + v2(-0.13 * s, 0.0), sh + v2(0.13 * s, 0.0)])
        .fill(color, 1.0)
        .no_stroke();
    let lo = polygon(&[waist, hip + v2(-0.13 * s, 0.0), hip + v2(0.13 * s, 0.0)])
        .fill(color, 1.0)
        .no_stroke();
    let hd = circle(0.085 * s).move_to(head).fill(color, 1.0).no_stroke();
    let swing = 0.06 * s * (t * 6.0 + phase + 1.0).sin();
    let reach = warli_reach(s);
    let lh = v2(pos.x - reach, sh.y - 0.02 * s + swing);
    let rh = v2(pos.x + reach, sh.y - 0.02 * s - swing);
    let le = sh + v2(-0.2 * s, -0.12 * s);
    let re = sh + v2(0.2 * s, -0.12 * s);
    let w = (s * 6.0).clamp(1.2, 3.0) as f32;
    let step = 0.05 * s * (t * 6.0 + phase).cos();
    let lk = hip + v2(-0.12 * s + step, -0.13 * s);
    let rk = hip + v2(0.12 * s + step, -0.13 * s);
    let limbs = group(vec![
        polyline(&[sh + v2(-0.11 * s, 0.0), le, lh]),
        polyline(&[sh + v2(0.11 * s, 0.0), re, rh]),
        polyline(&[hip + v2(-0.08 * s, 0.0), lk, lk + v2(-0.04 * s, -0.17 * s)]),
        polyline(&[hip + v2(0.08 * s, 0.0), rk, rk + v2(0.04 * s, -0.17 * s)]),
    ])
    .fill(color, 0.0)
    .stroke(color, w);
    (
        group(vec![limbs, up, lo, hd]),
        WarliPose {
            head,
            left_hand: lh,
            right_hand: rh,
        },
    )
}

/// A Warli stick figure (two triangles, circle head) of height `s`, dancing with time.
pub fn warli_figure(pos: Vec2, s: f64, t: f64, phase: f64, color: Color) -> Mobject {
    warli_parts(pos, s, t, phase, color).0
}

/// Joint positions of [`warli_figure`] with the same arguments.
pub fn warli_pose(pos: Vec2, s: f64, t: f64, phase: f64) -> WarliPose {
    warli_parts(pos, s, t, phase, WHITE).1
}

/// The Warli *tarpa* dance: `rings` concentric circles of hand-holding dancers turning
/// slowly around a musician, heads pointing outward.
pub fn warli_circle_dance(center: Vec2, r: f64, rings: usize, t: f64, color: Color) -> Mobject {
    let mut out = vec![];
    for ring_i in 0..rings.max(1) {
        let rr = r * (1.0 - 0.3 * ring_i as f64);
        let s = rr * 0.42;
        let n = ((TAU * rr) / (2.0 * warli_reach(s))).floor().max(3.0) as usize;
        let spin = t * 0.25 * if ring_i % 2 == 0 { 1.0 } else { -1.0 };
        for i in 0..n {
            let a = TAU * i as f64 / n as f64 + spin;
            let p = center + Vec2::polar(rr, a);
            let fig = warli_figure(v2(0.0, 0.0), s, t, i as f64 * 0.7, color);
            out.push(fig.rotate_about(a - PI / 2.0, v2(0.0, 0.0)).shift(p));
        }
    }
    // the musician with his tarpa (a long horn) in the middle
    let ms = r * 0.4;
    out.push(warli_figure(center, ms, t * 0.5, 0.0, color));
    let mouth = center + v2(0.03 * ms, 0.36 * ms);
    out.push(
        polygon(&[
            mouth,
            mouth + v2(0.42 * ms, -0.12 * ms),
            mouth + v2(0.46 * ms, -0.02 * ms),
        ])
        .fill(color, 1.0)
        .no_stroke(),
    );
    group(out)
}

/// A Warli frame: terracotta band with chains of dancers along top and bottom, sun
/// and moon roundels in the corners and chevron "mountains" up the sides.
pub fn warli_border(rect: Rect, band: f64, ground: Color, ink: Color, t: f64) -> Mobject {
    let mut parts = vec![ring(rect, band).fill(ground, 1.0).no_stroke()];
    let inner = Rect::new(rect.min + v2(band, band), rect.max - v2(band, band));
    parts.push(outline_rect(rect.expand(-0.03), ink, 1.6));
    parts.push(outline_rect(inner.expand(0.03), ink, 1.6));
    let s = band * 0.82;
    let span = 2.0 * warli_reach(s);
    let n = ((inner.width() - band * 0.2) / span).floor().max(1.0) as usize;
    let x0 = inner.center().x - span * (n as f64 - 1.0) * 0.5;
    for (row, y) in [(0, rect.max.y - band * 0.5), (1, rect.min.y + band * 0.5)] {
        for i in 0..n {
            let p = v2(x0 + span * i as f64, y + 0.03 * s);
            parts.push(warli_figure(p, s, t, (i + row) as f64 * 0.9, ink));
        }
    }
    let ch = band * 0.34;
    for x in [rect.min.x + band * 0.5, rect.max.x - band * 0.5] {
        let mut y = inner.min.y + ch;
        let mut k = 0;
        while y + ch < inner.max.y {
            let tri = polygon(&[v2(x - ch * 0.8, y), v2(x + ch * 0.8, y), v2(x, y + ch)]);
            parts.push(if k % 2 == 0 {
                tri.fill(ink, 1.0).no_stroke()
            } else {
                tri.fill(ink, 0.0).stroke(ink, 1.6)
            });
            y += ch * 1.25;
            k += 1;
        }
    }
    for (k, c) in corners(rect, band).into_iter().enumerate() {
        let r = band * 0.3;
        parts.push(
            circle(r)
                .move_to(c)
                .fill(ink, if k % 2 == 0 { 1.0 } else { 0.0 })
                .stroke(ink, 1.8),
        );
        for j in 0..12 {
            let a = TAU * j as f64 / 12.0;
            parts.push(
                line(c + Vec2::polar(r * 1.2, a), c + Vec2::polar(r * 1.55, a)).stroke(ink, 1.4),
            );
        }
    }
    group(parts)
}

/// A Gond leaf: coloured almond with a midrib and veins made of dots and dashes.
pub fn gond_leaf(
    center: Vec2,
    len: f64,
    angle: f64,
    base: Color,
    ink: Color,
    seed: u64,
) -> Mobject {
    let hw = len * 0.24;
    let tip = v2(len * 0.5, 0.0);
    let back = v2(-len * 0.5, 0.0);
    let body = smooth_curve(&[back, v2(0.0, hw), tip, v2(0.0, -hw)], true)
        .fill(base, 1.0)
        .stroke(ink, 1.6);
    let mut marks = vec![line(back, tip).stroke(ink, 1.4)];
    let veins = 5;
    for v in 0..veins {
        let x = -len * 0.32 + len * 0.62 * v as f64 / (veins - 1) as f64;
        let w = hw * (1.0 - (x / (len * 0.5)).powi(2)).max(0.0).sqrt() * 0.82;
        for side in [-1.0, 1.0] {
            let dots = 4;
            for d in 1..=dots {
                let f = d as f64 / (dots + 1) as f64;
                let p = v2(x + f * w * 0.6, side * f * w);
                if (v + d) % 2 == 0 || rnd(v as i64, d as i64, seed) > 0.7 {
                    marks.push(circle(len * 0.018).move_to(p).fill(ink, 1.0).no_stroke());
                } else {
                    let q = p + v2(len * 0.05, side * len * 0.03);
                    marks.push(line(p, q).stroke(ink, 1.3));
                }
            }
        }
    }
    group(vec![body, group(marks)])
        .rotate_about(angle, v2(0.0, 0.0))
        .shift(center)
}

/// A Gond frame: dotted rails with a band of patterned leaves in alternating colours.
pub fn gond_border(rect: Rect, band: f64, ground: Color, seed: u64) -> Mobject {
    let ink = rgb8(28, 26, 34);
    let fills = [
        rgb8(230, 92, 60),
        rgb8(250, 196, 60),
        rgb8(60, 160, 140),
        rgb8(120, 90, 190),
        rgb8(236, 112, 160),
    ];
    let mut parts = vec![ring(rect, band).fill(ground, 1.0).no_stroke()];
    let inner = Rect::new(rect.min + v2(band, band), rect.max - v2(band, band));
    parts.push(outline_rect(rect.expand(-0.02), ink, 2.2));
    parts.push(outline_rect(inner.expand(0.02), ink, 2.2));
    for (i, (c, rot)) in border_cells(rect, band, band * 0.9).into_iter().enumerate() {
        let tilt = if i % 2 == 0 { 0.5 } else { -0.5 };
        parts.push(gond_leaf(
            c,
            band * 0.95,
            rot + tilt + PI / 2.0,
            fills[i % fills.len()],
            ink,
            seed + i as u64,
        ));
    }
    for (k, c) in corners(rect, band).into_iter().enumerate() {
        let r = band * 0.36;
        parts.push(
            circle(r)
                .move_to(c)
                .fill(fills[(k + 2) % 5], 1.0)
                .stroke(ink, 2.0),
        );
        for j in 0..10 {
            let a = TAU * j as f64 / 10.0;
            parts.push(
                circle(band * 0.03)
                    .move_to(c + Vec2::polar(r * 0.65, a))
                    .fill(ink, 1.0)
                    .no_stroke(),
            );
        }
        parts.push(circle(r * 0.25).move_to(c).fill(ink, 1.0).no_stroke());
    }
    group(parts)
}

/// A decorative Gond tree whose branches carry patterned leaves that sway with `t`.
pub fn gond_tree(pos: Vec2, h: f64, t: f64, seed: u64) -> Mobject {
    let ink = rgb8(28, 26, 34);
    let fills = [
        rgb8(230, 92, 60),
        rgb8(250, 196, 60),
        rgb8(60, 160, 140),
        rgb8(120, 90, 190),
        rgb8(236, 112, 160),
    ];
    let trunk_c = rgb8(150, 90, 50);
    let top = pos + v2(0.0, h * 0.55);
    let trunk = polygon(&[
        pos + v2(-0.09 * h, 0.0),
        pos + v2(0.09 * h, 0.0),
        top + v2(0.04 * h, 0.0),
        top + v2(-0.04 * h, 0.0),
    ])
    .fill(trunk_c, 1.0)
    .stroke(ink, 2.0);
    let mut parts = vec![trunk];
    // dash bands on the trunk
    let mut y = pos.y + 0.04 * h;
    while y < top.y - 0.02 * h {
        let f = (y - pos.y) / (top.y - pos.y);
        let w = 0.09 * h * (1.0 - f) + 0.04 * h * f;
        parts
            .push(line(v2(pos.x - w * 0.8, y), v2(pos.x + w * 0.8, y + 0.02 * h)).stroke(ink, 1.2));
        y += 0.045 * h;
    }
    let branches = 7;
    for b in 0..branches {
        let a = PI * 0.12 + PI * 0.76 * b as f64 / (branches - 1) as f64;
        let sway = 0.05 * (t * 1.4 + b as f64).sin();
        let start = top - v2(0.0, 0.06 * h * (b % 2) as f64);
        let len = h * (0.34 + 0.08 * rnd(b, 1, seed));
        let end = start + Vec2::polar(len, a + sway);
        let mid = start.lerp(end, 0.5) + Vec2::polar(0.05 * h, a + PI / 2.0);
        parts.push(
            smooth_curve(&[start, mid, end], false)
                .fill(ink, 0.0)
                .stroke(ink, 2.4),
        );
        let leaves = 4;
        for l in 0..leaves {
            let f = 0.35 + 0.65 * l as f64 / (leaves - 1) as f64;
            let p = start.lerp(end, f);
            for side in [-1.0, 1.0] {
                let la = a + sway + side * 0.9 + 0.1 * (t * 2.0 + l as f64).sin();
                let ll = h * 0.12 * (1.0 - 0.3 * f);
                let c = p + Vec2::polar(ll * 0.5, la);
                let idx = (b as usize * 3 + l + if side > 0.0 { 1 } else { 0 }) % fills.len();
                parts.push(gond_leaf(
                    c,
                    ll,
                    la,
                    fills[idx],
                    ink,
                    seed + (b * 10 + l as i64) as u64,
                ));
            }
        }
    }
    group(parts)
}

/// A complete folk-art frame in the requested tradition around `rect`.
pub fn folk_frame(style: FolkStyle, rect: Rect, band: f64, t: f64, seed: u64) -> Mobject {
    match style {
        FolkStyle::Madhubani => madhubani_border(rect, band, rgb8(250, 238, 205)),
        FolkStyle::Warli => warli_border(rect, band, rgb8(150, 64, 40), rgb8(250, 242, 228), t),
        FolkStyle::Gond => gond_border(rect, band, rgb8(250, 244, 230), seed),
    }
}

// ------------------------------------------------------------------ ambience

/// Falling rain streaks inside `rect`: `n` drops, slanted by `wind` (units per unit fall).
pub fn rain(rect: Rect, t: f64, n: usize, color: Color, wind: f64, seed: u64) -> Mobject {
    let h = rect.height();
    let w = rect.width();
    group((0..n).map(|i| {
        let depth = rnd(i as i64, 0, seed); // 0 far … 1 near
        let speed = 5.0 + 5.0 * depth;
        let len = 0.12 + 0.22 * depth;
        let fall = fract(rnd(i as i64, 1, seed) + t * speed / (h + len)) * (h + len);
        let y = rect.max.y - fall + len;
        let x = rect.min.x + fract(rnd(i as i64, 2, seed) + (wind * fall) / w) * w;
        let a = v2(x, y.min(rect.max.y));
        let b = v2(x - wind * len, (y - len).max(rect.min.y));
        line(a, b)
            .stroke(color, (0.8 + 1.6 * depth) as f32)
            .map_style(move |s| s.stroke_opacity = (0.25 + 0.55 * depth) as f32)
    }))
}

/// Splash rings where raindrops hit the ground line `y` across `[x0, x1]`.
pub fn splashes(x0: f64, x1: f64, y: f64, t: f64, n: usize, color: Color, seed: u64) -> Mobject {
    group((0..n).map(|i| {
        let rate = 1.2 + rnd(i as i64, 5, seed);
        let ph = fract(t * rate + rnd(i as i64, 6, seed));
        let cycle = (t * rate + rnd(i as i64, 6, seed)).floor() as i64;
        let x = x0 + (x1 - x0) * rnd(i as i64, cycle, seed + 1);
        let yy = y - 0.25 * rnd(i as i64, cycle, seed + 2);
        let w = 0.04 + 0.26 * ph;
        let a = (1.0 - ph) as f32 * 0.8;
        let mut parts = vec![ellipse(w, w * 0.28)
            .move_to(v2(x, yy))
            .fill(color, 0.0)
            .stroke(color, 1.4)
            .map_style(move |s| s.stroke_opacity = a)];
        if ph < 0.5 {
            let up = (ph * 2.0 * PI).sin() * 0.08;
            for d in [-1.0, 1.0] {
                parts.push(
                    circle(0.012)
                        .move_to(v2(x + d * w * 0.4, yy + up))
                        .fill(color, a)
                        .no_stroke(),
                );
            }
        }
        group(parts)
    }))
}

/// A simple leaf (local frame: stem at origin pointing −x), `s` long.
pub fn leaf_shape(s: f64, color: Color) -> Mobject {
    let body = smooth_curve(
        &[
            v2(0.0, 0.0),
            v2(0.45 * s, 0.2 * s),
            v2(s, 0.0),
            v2(0.45 * s, -0.2 * s),
        ],
        true,
    )
    .fill(color, 1.0)
    .no_stroke();
    let rib = line(v2(-0.08 * s, 0.0), v2(0.85 * s, 0.0)).stroke(shade(color, 0.3), 1.2);
    group(vec![body, rib])
}

/// Leaves (about `size` long) tumbling down through `rect` (sway, spin and a 3-D flip).
pub fn falling_leaves(
    rect: Rect,
    t: f64,
    n: usize,
    size: f64,
    colors: &[Color],
    seed: u64,
) -> Mobject {
    let h = rect.height() + 0.6;
    group((0..n).map(|i| {
        let speed = 0.45 + 0.4 * rnd(i as i64, 0, seed);
        let ph = rnd(i as i64, 1, seed) * TAU;
        let y = rect.max.y + 0.3 - fract(rnd(i as i64, 2, seed) + t * speed / h) * h;
        let x = rect.min.x + rect.width() * rnd(i as i64, 3, seed) + 0.45 * (t * 1.1 + ph).sin();
        let s = size * (0.75 + 0.5 * rnd(i as i64, 4, seed));
        let c = colors[i % colors.len().max(1)];
        let flip = (t * 2.3 + ph).cos();
        let ang = 0.9 * (t * 1.1 + ph).cos() + t * 0.6 * (rnd(i as i64, 5, seed) - 0.5);
        leaf_shape(s, c)
            .stretch(1.0, flip.abs().max(0.15))
            .shift(v2(-s * 0.5, 0.0))
            .rotate_about(ang, v2(0.0, 0.0))
            .shift(v2(x, y))
    }))
}

/// Blinking fireflies wandering inside `rect`.
pub fn fireflies(rect: Rect, t: f64, n: usize, color: Color, seed: u64) -> Mobject {
    group((0..n).filter_map(|i| {
        let base = v2(
            rect.min.x + rect.width() * rnd(i as i64, 0, seed),
            rect.min.y + rect.height() * rnd(i as i64, 1, seed),
        );
        let drift = v2(
            fbm2(v2(t * 0.25, i as f64 * 1.7), 2, 2.0, 0.5, seed),
            fbm2(v2(i as f64 * 2.3, t * 0.25), 2, 2.0, 0.5, seed + 1),
        ) * 0.9;
        let p = base + drift;
        let blink =
            ((t * (0.8 + 0.6 * rnd(i as i64, 2, seed)) + rnd(i as i64, 3, seed)) * TAU).sin();
        let b = ((blink - 0.1) / 0.9).clamp(0.0, 1.0).powf(1.5) as f32;
        if b < 0.02 {
            return None;
        }
        let r = 0.22;
        let halo = circle(r)
            .move_to(p)
            .fill(color, 1.0)
            .no_stroke()
            .map_style(move |s| {
                s.fill = Paint::radial(p, r, &[color.with_alpha(0.55 * b), color.with_alpha(0.0)])
            });
        Some(group(vec![
            halo,
            circle(0.03)
                .move_to(p)
                .fill(tint(color, 0.6), b)
                .no_stroke(),
        ]))
    }))
}

/// Gently drifting snow.
pub fn snowfall(rect: Rect, t: f64, n: usize, seed: u64) -> Mobject {
    let h = rect.height() + 0.2;
    group((0..n).map(|i| {
        let depth = rnd(i as i64, 0, seed);
        let speed = 0.3 + 0.6 * depth;
        let y = rect.max.y + 0.1 - fract(rnd(i as i64, 1, seed) + t * speed / h) * h;
        let x =
            rect.min.x + rect.width() * rnd(i as i64, 2, seed) + 0.15 * (t * 1.3 + i as f64).sin();
        circle(0.012 + 0.025 * depth)
            .move_to(v2(x, y))
            .fill(WHITE, (0.5 + 0.5 * depth) as f32)
            .no_stroke()
    }))
}

/// Sky gradient `(top, horizon)` for a full day cycle: `phase` 0 = midnight,
/// 0.25 = dawn, 0.5 = noon, 0.75 = dusk, 1 = midnight again.
pub fn sky_colors(phase: f64) -> (Color, Color) {
    let keys: [(f64, Color, Color); 5] = [
        (0.0, rgb8(14, 18, 48), rgb8(34, 40, 86)),
        (0.25, rgb8(96, 110, 180), rgb8(255, 170, 130)),
        (0.5, rgb8(96, 170, 230), rgb8(214, 236, 250)),
        (0.75, rgb8(84, 60, 140), rgb8(255, 140, 80)),
        (1.0, rgb8(14, 18, 48), rgb8(34, 40, 86)),
    ];
    let p = fract(phase);
    for w in keys.windows(2) {
        let (a, b) = (w[0], w[1]);
        if p <= b.0 {
            let f = ((p - a.0) / (b.0 - a.0)) as f32;
            let f = f * f * (3.0 - 2.0 * f);
            return (a.1.mix(b.1, f), a.2.mix(b.2, f));
        }
    }
    (keys[0].1, keys[0].2)
}

/// Multiply a deep-blue night tint over `rect` (`darkness` ∈ [0, 1]).
pub fn night_overlay(rect: Rect, darkness: f64) -> Mobject {
    rectangle(rect.width(), rect.height())
        .move_to(rect.center())
        .fill(rgb8(40, 46, 110), (0.75 * darkness.clamp(0.0, 1.0)) as f32)
        .no_stroke()
        .blend(Blend::Multiply)
}

/// A hand-lettered style label (paper tab) used by the art-style demos.
pub fn style_tag(text: &str, pos: Vec2, bg: Color, fg: Color) -> Mobject {
    let label = hand_text(text, 0.3, fg);
    let w = label.width() + 0.4;
    group(vec![
        cutout(
            rounded_rectangle(w, 0.5, 0.1)
                .move_to(pos)
                .fill(bg, 1.0)
                .no_stroke(),
            CutoutStyle::default().lift(0.03).wobble(0.01),
        ),
        label.move_to(pos),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaves(m: &Mobject) -> usize {
        m.leaf_count()
    }

    #[test]
    fn folk_parse() {
        assert_eq!(FolkStyle::parse("WARLI"), FolkStyle::Warli);
        assert_eq!(FolkStyle::parse("gond"), FolkStyle::Gond);
        assert_eq!(FolkStyle::parse("?"), FolkStyle::Madhubani);
    }

    #[test]
    fn border_cells_stay_in_band() {
        let r = Rect::new(v2(-4.0, -2.0), v2(4.0, 2.0));
        let cells = border_cells(r, 0.5, 0.6);
        assert!(cells.len() > 20);
        let inner = r.expand(-0.5);
        for (c, _) in cells {
            assert!(r.contains(c) && !inner.contains(c), "{c:?}");
        }
    }

    #[test]
    fn rain_stays_inside() {
        let r = Rect::new(v2(-3.0, -2.0), v2(3.0, 2.0));
        let m = rain(r, 1.7, 80, WHITE, 0.2, 4);
        assert_eq!(m.children.len(), 80);
        let b = m.family_path().bounds().unwrap();
        assert!(b.min.y >= -2.0 - 1e-6 && b.max.y <= 2.0 + 1e-6);
    }

    #[test]
    fn brush_tapers_and_reveals() {
        let pts = [v2(-2.0, 0.0), v2(0.0, 0.3), v2(2.0, 0.0)];
        let full = brush_stroke(&pts, 0.3, BLACK, 0.0, 1, 1.0);
        let b = full.family_path().bounds().unwrap();
        assert!(b.height() < 0.3 + 0.3 + 0.1);
        let half = brush_stroke(&pts, 0.3, BLACK, 0.0, 1, 0.5);
        let hb = half.family_path().bounds().unwrap();
        assert!(hb.max.x < 0.4, "half stroke ends near the middle: {hb:?}");
        assert!(leaves(&brush_stroke(&pts, 0.3, BLACK, 0.8, 1, 1.0)) > 3);
        assert_eq!(leaves(&brush_stroke(&pts, 0.3, BLACK, 0.0, 1, 0.0)), 0);
    }

    #[test]
    fn silhouette_is_monochrome() {
        let m = crate::story::Character::girl().draw_at(
            &crate::story::Pose::idle(0.0),
            0.0,
            0.0,
            1.0,
            v2(0.0, 0.0),
        );
        let s = silhouette(m, BLACK);
        let mut ok = true;
        let mut s2 = s.clone();
        s2.for_each_mut(&mut |l| {
            if let Paint::Solid(c) = l.style.fill {
                ok &= c == BLACK;
            }
        });
        assert!(ok);
    }

    #[test]
    fn warli_hands_meet_neighbours() {
        let s = 1.0;
        let span = 2.0 * warli_reach(s);
        let a = warli_pose(v2(0.0, 0.0), s, 0.0, 0.0);
        let b = warli_pose(v2(span, 0.0), s, 0.0, 0.0);
        assert!((a.right_hand.x - b.left_hand.x).abs() < 1e-9);
        assert!(a.head.y > 0.3);
    }

    #[test]
    fn sky_cycle_wraps_and_brightens_at_noon() {
        let (m0, _) = sky_colors(0.0);
        let (m1, _) = sky_colors(1.0);
        let (noon, _) = sky_colors(0.5);
        assert!((m0.r - m1.r).abs() < 1e-4);
        assert!(noon.luminance() > m0.luminance() + 0.2);
    }

    #[test]
    fn watercolor_and_cutout_build() {
        let m = circle(1.0).fill(RED, 1.0);
        assert_eq!(watercolor(m.clone(), RED, 5, 2).children.len(), 6);
        let c = cutout(m, CutoutStyle::default());
        assert!(c.style.shadow.is_some());
        assert!(c.path.subpaths[0].segs.len() >= 8);
        let fr = folk_frame(
            FolkStyle::Gond,
            Rect::new(v2(-3.0, -2.0), v2(3.0, 2.0)),
            0.5,
            0.0,
            1,
        );
        assert!(leaves(&fr) > 50);
    }
}
