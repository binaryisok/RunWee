//! Decorative elements: `@remotion/rough-notation` (hand-drawn underline, box,
//! circle, highlight, strike-through, crossed-off, bracket), `@remotion/shapes`
//! extras (heart, callout, spark, block arrow, pie), `@remotion/light-leaks`,
//! `@remotion/starburst`, `@remotion/rounded-text-box`, and manim props
//! (speech/thought bubbles, clock, lightbulb, die face, speedometer, progress bar).

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::shapes::*;
use crate::style::*;
use crate::text::{text_sized, Text};

// ----------------------------------------------------------- rough notation

/// Annotation types from rough-notation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoughKind {
    Underline,
    Box,
    Circle,
    Highlight,
    StrikeThrough,
    CrossedOff,
    Bracket,
}

fn jitter(rng: &mut Rng, p: Vec2, amount: f64) -> Vec2 {
    p + v2(rng.range(-amount, amount), rng.range(-amount, amount))
}

/// A hand-drawn wobbly line from `a` to `b` (two passes like rough.js).
pub fn rough_line(a: Vec2, b: Vec2, roughness: f64, seed: u64) -> Mobject {
    let mut rng = Rng::new(seed);
    let mut g = Mobject::empty();
    let len = a.distance(b);
    let off = roughness * (0.02 + len * 0.012).min(0.08);
    for pass in 0..2 {
        let n = (len / 0.4).ceil().max(2.0) as usize;
        let pts: Vec<Vec2> = (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                let bow = (PI * t).sin() * off * if pass == 0 { 1.0 } else { -0.7 };
                jitter(
                    &mut rng,
                    a.lerp(b, t) + (b - a).normalize().perp() * bow,
                    off * 0.5,
                )
            })
            .collect();
        g.push(smooth_curve(&pts, false));
    }
    g
}

/// Rough-notation style annotation around `target`. Animate with `create`.
pub fn rough_annotation(target: &Mobject, kind: RoughKind, color: Color, seed: u64) -> Mobject {
    let b = target.bounds_or_origin();
    let pad = 0.12;
    let r = b.expand(pad);
    let mut rng = Rng::new(seed);
    let m = match kind {
        RoughKind::Underline => rough_line(v2(r.min.x, r.min.y), v2(r.max.x, r.min.y), 1.0, seed),
        RoughKind::StrikeThrough => {
            let y = b.center().y;
            rough_line(v2(r.min.x, y), v2(r.max.x, y), 1.0, seed)
        }
        RoughKind::CrossedOff => Mobject::group([
            rough_line(r.min, r.max, 1.0, seed),
            rough_line(v2(r.min.x, r.max.y), v2(r.max.x, r.min.y), 1.0, seed + 1),
        ]),
        RoughKind::Box => Mobject::group([
            rough_line(v2(r.min.x, r.max.y), r.max, 1.0, seed),
            rough_line(r.max, v2(r.max.x, r.min.y), 1.0, seed + 1),
            rough_line(v2(r.max.x, r.min.y), r.min, 1.0, seed + 2),
            rough_line(r.min, v2(r.min.x, r.max.y), 1.0, seed + 3),
        ]),
        RoughKind::Circle => {
            let c = r.center();
            let (rx, ry) = (r.width() * 0.62, r.height() * 0.75);
            let n = 40;
            let start = rng.range(0.0, TAU);
            let pts: Vec<Vec2> = (0..=n + 6)
                .map(|i| {
                    let a = start + TAU * i as f64 / n as f64;
                    let k = 1.0 + rng.range(-0.05, 0.05) + 0.04 * (i as f64 / n as f64);
                    c + v2(rx * k * a.cos(), ry * k * a.sin())
                })
                .collect();
            smooth_curve(&pts, false)
        }
        RoughKind::Highlight => {
            let mut g = Mobject::empty();
            let rows = 3;
            for i in 0..rows {
                let y = b.min.y + b.height() * (i as f64 + 0.5) / rows as f64;
                g.push(rough_line(
                    v2(r.min.x, y),
                    v2(r.max.x, y),
                    0.6,
                    seed + i as u64,
                ));
            }
            return g
                .stroke(color, (b.height() / rows as f64 * 135.0 * 1.1) as f32)
                .stroke_opacity(0.45)
                .line_cap(LineCap::Butt);
        }
        RoughKind::Bracket => {
            let w = 0.15;
            Mobject::group([
                rough_line(v2(r.min.x + w, r.max.y), v2(r.min.x, r.max.y), 0.5, seed),
                rough_line(v2(r.min.x, r.max.y), v2(r.min.x, r.min.y), 0.8, seed + 1),
                rough_line(
                    v2(r.min.x, r.min.y),
                    v2(r.min.x + w, r.min.y),
                    0.5,
                    seed + 2,
                ),
                rough_line(
                    v2(r.max.x - w, r.max.y),
                    v2(r.max.x, r.max.y),
                    0.5,
                    seed + 3,
                ),
                rough_line(v2(r.max.x, r.max.y), v2(r.max.x, r.min.y), 0.8, seed + 4),
                rough_line(
                    v2(r.max.x, r.min.y),
                    v2(r.max.x - w, r.min.y),
                    0.5,
                    seed + 5,
                ),
            ])
        }
    };
    m.stroke(color, 3.5)
}

// ------------------------------------------------------------------ shapes

pub fn heart(size: f64) -> Mobject {
    let pts: Vec<Vec2> = (0..=120)
        .map(|i| {
            let t = TAU * i as f64 / 120.0;
            let x = 16.0 * t.sin().powi(3);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            v2(x, y) * (size / 32.0)
        })
        .collect();
    smooth_curve(&pts, true).fill(RED, 1.0).no_stroke()
}

/// Four-point sparkle (Remotion `<Spark>`).
pub fn spark(size: f64) -> Mobject {
    let r = size / 2.0;
    let k = r * 0.12;
    let mut p = Path::new();
    p.move_to(v2(0.0, r));
    p.quad_to(v2(k, k), v2(r, 0.0));
    p.quad_to(v2(k, -k), v2(0.0, -r));
    p.quad_to(v2(-k, -k), v2(-r, 0.0));
    p.quad_to(v2(-k, k), v2(0.0, r));
    p.close();
    Mobject::from_path(p).fill(YELLOW, 1.0).no_stroke()
}

/// Speech-callout box with a tail pointing at `tail` (relative to the box centre).
pub fn callout(width: f64, height: f64, radius: f64, tail: Vec2) -> Mobject {
    let body = rounded_rectangle(width, height, radius);
    let base_x = tail
        .x
        .clamp(-width / 2.0 + radius + 0.15, width / 2.0 - radius - 0.15);
    let side = if tail.y < 0.0 {
        -height / 2.0
    } else {
        height / 2.0
    };
    let tri = polygon(&[v2(base_x - 0.18, side), v2(base_x + 0.18, side), tail]);
    Mobject::group([body, tri]).fill(WHITE, 1.0).no_stroke()
}

/// Block arrow (Remotion `<Arrow>` shape) pointing right.
pub fn block_arrow(length: f64, head_width: f64, shaft_width: f64, head_length: f64) -> Mobject {
    let (h, s) = (head_width / 2.0, shaft_width / 2.0);
    let x0 = -length / 2.0;
    let xh = length / 2.0 - head_length;
    polygon(&[
        v2(x0, s),
        v2(xh, s),
        v2(xh, h),
        v2(length / 2.0, 0.0),
        v2(xh, -h),
        v2(xh, -s),
        v2(x0, -s),
    ])
    .fill(BLUE, 1.0)
    .no_stroke()
}

/// Pie slice showing `progress` (0..1) of a circle (Remotion `<Pie>`).
pub fn pie(radius: f64, progress: f64, color: Color) -> Mobject {
    if progress >= 0.9999 {
        return circle(radius).fill(color, 1.0).no_stroke();
    }
    sector(radius, PI / 2.0, -TAU * progress.clamp(0.0, 1.0))
        .fill(color, 1.0)
        .no_stroke()
}

// ------------------------------------------------------- light leaks & burst

/// Soft colourful light leak (additive blobs) for time `t`; use with `always_redraw`.
pub fn light_leak(t: f64, seed: &str, intensity: f32) -> Mobject {
    let (w, h) = frame_size();
    let palette = [
        rgb8(255, 140, 50),
        rgb8(255, 70, 110),
        rgb8(255, 210, 120),
        rgb8(160, 90, 255),
    ];
    let mut g = Mobject::empty();
    for i in 0..4 {
        let s = format!("{seed}{i}");
        let x = crate::util::noise2d(&s, t * 0.15, 0.0) * w * 0.6;
        let y = crate::util::noise2d(&s, 0.0, t * 0.12 + 3.0) * h * 0.6;
        let r = h * (0.45 + 0.25 * crate::util::noise2d(&s, t * 0.2, 7.0).abs());
        let c = palette[i % palette.len()];
        let mut m = circle_at(v2(x, y), r).no_stroke();
        m.style.fill = Paint::Radial {
            center: v2(x, y),
            radius: r,
            focal: v2(x, y),
            stops: vec![
                (0.0, c.with_alpha(0.75 * intensity)),
                (1.0, c.with_alpha(0.0)),
            ],
        };
        m.style.fill_opacity = 1.0;
        m.style.blend = Blend::Screen;
        g.push(m);
    }
    g
}

/// Rotating sunburst rays (Remotion `<Starburst>`), behind content.
pub fn starburst(rays: usize, colors: &[Color], rotation: f64, radius: f64) -> Mobject {
    let cs = if colors.is_empty() {
        vec![rgb8(30, 40, 70), rgb8(20, 26, 46)]
    } else {
        colors.to_vec()
    };
    let n = rays.max(2) * 2;
    Mobject::group((0..n).map(|i| {
        let a0 = rotation + TAU * i as f64 / n as f64;
        sector(radius, a0, TAU / n as f64)
            .fill(cs[i % cs.len()], 1.0)
            .no_stroke()
    }))
}

/// Text on a rounded background that hugs each line (`@remotion/rounded-text-box`).
pub fn rounded_text_box(text: &str, size: f64, fg: Color, bg: Color, max_width: f64) -> Mobject {
    let t = Text::new(text)
        .size(size)
        .color(fg)
        .max_width(max_width)
        .align(crate::text::Align::Center)
        .build();
    let mut g = Mobject::empty();
    let pad = size * 0.35;
    for line in &t.children {
        if line.children.is_empty() {
            continue;
        }
        let b = line.bounds_or_origin();
        let lh = size * 1.25;
        g.push(
            rounded_rectangle(b.width() + 2.0 * pad, lh + pad, size * 0.35)
                .move_to(v2(b.center().x, b.center().y))
                .fill(bg, 1.0)
                .no_stroke(),
        );
    }
    g.push(t);
    g
}

// -------------------------------------------------------------- manim props

/// Speech bubble with text (manim's `SpeechBubble`).
pub fn speech_bubble(text: &str, size: f64) -> Mobject {
    let t = crate::text::paragraph(text, 4.0, size, crate::text::Align::Center).color(BLACK);
    let (w, h) = (t.width() + size * 1.4, t.height() + size * 1.2);
    let c = callout(w, h, size * 0.6, v2(-w * 0.3, -h / 2.0 - size * 1.2));
    Mobject::group([c, t])
}

/// Thought bubble (manim's `ThoughtBubble`).
pub fn thought_bubble(text: &str, size: f64) -> Mobject {
    let t = crate::text::paragraph(text, 4.0, size, crate::text::Align::Center).color(BLACK);
    let (w, h) = (t.width() + size * 1.8, t.height() + size * 1.6);
    let mut g = Mobject::empty();
    let n = 14;
    for i in 0..n {
        let a = TAU * i as f64 / n as f64;
        let p = v2(w / 2.0 * a.cos(), h / 2.0 * a.sin());
        g.push(circle_at(p, size * 0.6 + 0.1 * ((i * 7) % 3) as f64 * size));
    }
    g.push(ellipse(w, h));
    for (k, r) in [(1.0, 0.18), (1.6, 0.12), (2.1, 0.08)] {
        g.push(circle_at(v2(-w * 0.35 - k * 0.3, -h / 2.0 - k * 0.35), r));
    }
    Mobject::group([g.fill(WHITE, 1.0).no_stroke(), t])
}

/// Body of a speech bubble around `content`: rounded box plus a stem towards
/// the lower left (or lower right when `stem_right`), merged into one outline.
fn speech_bubble_body(content: &Mobject, buff: f64, stem_right: bool) -> Mobject {
    let b = content.bounds_or_origin().expand(buff);
    let r = (b.height() * 0.25).min(0.25);
    let rect = Mobject::from_path(rounded_rect_path(b.center(), b.width(), b.height(), r));
    let (lp, rp) = (v2(b.min.x, b.min.y), v2(b.max.x, b.min.y));
    let stem_h = 0.5 * b.height();
    let tri = polygon(&[lp.lerp(rp, 0.2), lp.lerp(rp, 0.3), lp + DOWN * stem_h]);
    let mut body = crate::boolean::union(&rect.fill(WHITE, 1.0), &tri.fill(WHITE, 1.0));
    if stem_right {
        // the stem lies within the box's x-range, so this mirrors in place
        body = body.flip(UP);
    }
    body
}

/// manim's `Bubble`/`SpeechBubble` around any content: black body (opacity 0.8)
/// with a white outline; the stem points down-left, or down-right with
/// `stem_right`. Children: [body, content]. The stem tip is `bubble.edge(DL)` /
/// `edge(DR)`.
pub fn bubble(content: Mobject, buff: f64, stem_right: bool) -> Mobject {
    let body = speech_bubble_body(&content, buff, stem_right)
        .fill(BLACK, 0.8)
        .stroke(WHITE, 3.0);
    Mobject::group([body.with_tag("body"), content])
}

/// Two overlapping speech bubbles (manim's `DoubleSpeechBubble`): `left` speaks
/// from the lower left, `right` (behind, offset up-right) from the lower right.
pub fn double_speech_bubble(left: &str, right: &str, size: f64) -> Mobject {
    let tl = text_sized(left, size, WHITE);
    let tr = text_sized(right, size, WHITE);
    let back = bubble(tr, size * 0.6, true);
    let front = bubble(tl, size * 0.6, false);
    let back = back.move_to(front.center() + v2(front.width() * 0.45, front.height() * 0.55));
    Mobject::group([back, front])
}

/// Dartboard (manim's `Dartboard`): 20 alternating sectors with double and
/// treble rings and a bullseye; children: [sectors, treble ring, double ring, bullseyes].
pub fn dartboard(radius: f64) -> Mobject {
    let n = 20;
    let a = TAU / n as f64;
    let ring = |colors: [Color; 2], inner: f64, outer: f64| {
        Mobject::group((0..n).map(|i| {
            annular_sector(inner * radius, outer * radius, i as f64 * a - a / 2.0, a)
                .fill(colors[i % 2], 1.0)
                .no_stroke()
        }))
    };
    let mut g = Mobject::empty();
    g.push(ring([GRAY_B, GRAY_E], 0.0, 1.0));
    g.push(ring([GREEN_E, RED_E], 0.5, 0.55));
    g.push(ring([GREEN_E, RED_E], 0.95, 1.0));
    g.push(Mobject::group([
        circle(0.07 * radius).fill(GREEN_E, 1.0).no_stroke(),
        circle(0.035 * radius)
            .fill(RED_E, 1.0)
            .no_stroke()
            .with_tag("bullseye"),
    ]));
    g
}

/// Video icon (manim's `VideoIcon`): a rounded screen with a play triangle cut out.
pub fn video_icon(width: f64, color: Color) -> Mobject {
    let h = width * 0.62;
    let mut p = rounded_rect_path(Vec2::ZERO, width, h, h * 0.18);
    let t = h * 0.22;
    p.subpaths.extend(
        Path::polyline(&[v2(-t * 0.8, t), v2(-t * 0.8, -t), v2(t * 1.1, 0.0)], true).subpaths,
    );
    let mut m = Mobject::from_path_styled(p, Style::filled(color));
    m.style.fill_rule = FillRule::EvenOdd;
    m
}

/// Analog clock showing `hours:minutes` (manim's `Clock`).
pub fn clock(radius: f64, hours: f64, minutes: f64) -> Mobject {
    let mut g = Mobject::group([circle(radius).stroke(WHITE, 4.0).fill(GRAY_E, 1.0)]);
    for i in 0..12 {
        let a = PI / 2.0 - TAU * i as f64 / 12.0;
        let d = Vec2::from_angle(a);
        g.push(line(d * radius * 0.85, d * radius * 0.97).stroke(WHITE, 3.0));
    }
    let ma = PI / 2.0 - TAU * minutes / 60.0;
    let ha = PI / 2.0 - TAU * (hours + minutes / 60.0) / 12.0;
    g.push(
        line(Vec2::ZERO, Vec2::polar(radius * 0.8, ma))
            .stroke(WHITE, 4.0)
            .with_tag("minute"),
    );
    g.push(
        line(Vec2::ZERO, Vec2::polar(radius * 0.5, ha))
            .stroke(WHITE, 6.0)
            .with_tag("hour"),
    );
    g.push(dot_r(Vec2::ZERO, radius * 0.05, WHITE));
    g
}

/// Light bulb icon, glowing when `on` (manim's `Lightbulb`).
pub fn lightbulb(size: f64, on: bool) -> Mobject {
    let s = size;
    let glass = smooth_curve(
        &[
            v2(-0.18 * s, -0.25 * s),
            v2(-0.32 * s, 0.05 * s),
            v2(-0.3 * s, 0.3 * s),
            v2(0.0, 0.48 * s),
            v2(0.3 * s, 0.3 * s),
            v2(0.32 * s, 0.05 * s),
            v2(0.18 * s, -0.25 * s),
        ],
        false,
    );
    let col = if on { YELLOW } else { GRAY_B };
    let mut glass = glass
        .stroke(col, 4.0)
        .fill(col, if on { 0.85 } else { 0.15 });
    if on {
        glass = glass.glow(0.25 * s, Some(YELLOW));
    }
    let base = Mobject::group([
        rectangle(0.36 * s, 0.08 * s).shift(v2(0.0, -0.3 * s)),
        rectangle(0.32 * s, 0.08 * s).shift(v2(0.0, -0.4 * s)),
        rectangle(0.22 * s, 0.07 * s).shift(v2(0.0, -0.49 * s)),
    ])
    .fill(GRAY_B, 1.0)
    .no_stroke();
    Mobject::group([glass, base])
}

/// Die face with `n` pips (manim's `DieFace`).
pub fn die_face(n: usize, size: f64) -> Mobject {
    let mut g = Mobject::group([rounded_rectangle(size, size, size * 0.15)
        .fill(WHITE, 1.0)
        .stroke(GRAY_D, 3.0)]);
    let o = size * 0.27;
    let spots: &[(f64, f64)] = match n {
        1 => &[(0.0, 0.0)],
        2 => &[(-1.0, 1.0), (1.0, -1.0)],
        3 => &[(-1.0, 1.0), (0.0, 0.0), (1.0, -1.0)],
        4 => &[(-1.0, 1.0), (1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)],
        5 => &[
            (-1.0, 1.0),
            (1.0, 1.0),
            (0.0, 0.0),
            (-1.0, -1.0),
            (1.0, -1.0),
        ],
        _ => &[
            (-1.0, 1.0),
            (1.0, 1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-1.0, -1.0),
            (1.0, -1.0),
        ],
    };
    for (x, y) in spots {
        g.push(dot_r(v2(x * o, y * o), size * 0.09, BLACK));
    }
    g
}

/// Piano keyboard with `octaves` octaves (manim's `Piano`). Keys listed in `pressed`
/// (0 = leftmost white key's C, counting every semitone) are highlighted.
pub fn piano(octaves: usize, width: f64, pressed: &[usize], highlight: Color) -> Mobject {
    let whites = 7 * octaves.max(1);
    let kw = width / whites as f64;
    let kh = kw * 4.5;
    let x0 = -width / 2.0 + kw / 2.0;
    // Semitone offsets of the white keys and of the black keys (with the white key they follow).
    const WHITE_KEYS: [usize; 7] = [0, 2, 4, 5, 7, 9, 11];
    const BLACK_KEYS: [(usize, usize); 5] = [(1, 0), (3, 1), (6, 3), (8, 4), (10, 5)];
    let mut white = Mobject::group([]);
    let mut black = Mobject::group([]);
    for o in 0..octaves.max(1) {
        for (i, semi) in WHITE_KEYS.iter().enumerate() {
            let on = pressed.contains(&(o * 12 + semi));
            let x = x0 + (o * 7 + i) as f64 * kw;
            white.push(
                rectangle(kw, kh)
                    .fill(if on { highlight } else { WHITE }, 1.0)
                    .stroke(GRAY_D, 2.0)
                    .move_to(v2(x, 0.0)),
            );
        }
        for (semi, after) in BLACK_KEYS {
            let on = pressed.contains(&(o * 12 + semi));
            let x = x0 + (o * 7 + after) as f64 * kw + kw / 2.0;
            black.push(
                rectangle(kw * 0.6, kh * 0.62)
                    .fill(if on { highlight } else { BLACK }, 1.0)
                    .stroke(BLACK, 1.0)
                    .move_to(v2(x, kh * 0.19)),
            );
        }
    }
    Mobject::group([white, black])
}

/// Open laptop seen from the front (3b1b's `Laptop` prop); `screen` is placed on the display.
pub fn laptop(width: f64, screen: Option<Mobject>) -> Mobject {
    let sw = width * 0.86;
    let sh = sw * 0.62;
    let lid = rounded_rectangle(sw, sh, width * 0.03)
        .fill(GRAY_E, 1.0)
        .stroke(GRAY_B, 3.0)
        .shift(v2(0.0, sh / 2.0));
    let display = rectangle(sw * 0.92, sh * 0.86)
        .fill(BLACK, 1.0)
        .no_stroke()
        .shift(v2(0.0, sh / 2.0));
    let bh = width * 0.06;
    let base = polygon(&[
        v2(-sw / 2.0, 0.0),
        v2(sw / 2.0, 0.0),
        v2(width / 2.0, -bh),
        v2(-width / 2.0, -bh),
    ])
    .fill(GRAY_C, 1.0)
    .stroke(GRAY_B, 3.0);
    let notch = rounded_rectangle(width * 0.16, bh * 0.35, bh * 0.15)
        .fill(GRAY_D, 1.0)
        .no_stroke()
        .shift(v2(0.0, -bh * 0.3));
    let mut g = Mobject::group([lid, display.clone(), base, notch]);
    if let Some(m) = screen {
        let (dw, dh) = (sw * 0.88, sh * 0.8);
        let s = (dw / m.width().max(1e-9)).min(dh / m.height().max(1e-9));
        g.push(m.scale(s).move_to(display.center()));
    }
    g
}

/// Gauge / speedometer at `value` in [0, 1] (manim's `Speedometer`).
pub fn speedometer(radius: f64, value: f64) -> Mobject {
    let mut g = Mobject::group([arc(radius, -PI / 6.0, 4.0 * PI / 3.0).stroke(GRAY_B, 6.0)]);
    let filled = arc(
        radius,
        PI + PI / 6.0,
        -4.0 * PI / 3.0 * value.clamp(0.0, 1.0),
    )
    .stroke(GREEN, 8.0);
    g.push(filled);
    for i in 0..=10 {
        let a = PI + PI / 6.0 - 4.0 * PI / 3.0 * i as f64 / 10.0;
        let d = Vec2::from_angle(a);
        g.push(line(d * radius * 0.82, d * radius * 0.95).stroke(WHITE, 2.5));
    }
    let a = PI + PI / 6.0 - 4.0 * PI / 3.0 * value.clamp(0.0, 1.0);
    g.push(arrow(Vec2::ZERO, Vec2::polar(radius * 0.75, a)).color(RED));
    g.push(dot_r(Vec2::ZERO, radius * 0.07, WHITE));
    g
}

/// Horizontal progress bar with a percentage label.
pub fn progress_bar(width: f64, progress: f64, color: Color) -> Mobject {
    let h = 0.22;
    let p = progress.clamp(0.0, 1.0);
    let track = rounded_rectangle(width, h, h / 2.0)
        .fill(GRAY_D, 1.0)
        .no_stroke();
    let fill = rounded_rectangle((width * p).max(h), h, h / 2.0)
        .move_to_aligned(v2(-width / 2.0, 0.0), LEFT)
        .fill(color, 1.0)
        .no_stroke();
    let label = text_sized(&format!("{}%", (p * 100.0).round()), 0.3, WHITE).next_to_point(
        v2(width / 2.0, 0.0),
        RIGHT,
        0.2,
    );
    Mobject::group([track, fill, label])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorations_build() {
        let t = text_sized("Important", 0.5, WHITE);
        for k in [
            RoughKind::Underline,
            RoughKind::Box,
            RoughKind::Circle,
            RoughKind::Highlight,
            RoughKind::StrikeThrough,
            RoughKind::CrossedOff,
            RoughKind::Bracket,
        ] {
            let a = rough_annotation(&t, k, RED, 1);
            assert!(a.leaf_count() >= 1, "{k:?}");
            assert!(
                a.bounds()
                    .unwrap()
                    .intersects(&t.bounds().unwrap().expand(0.3)),
                "{k:?}"
            );
        }
        for m in [
            heart(1.0),
            spark(1.0),
            callout(3.0, 1.5, 0.3, v2(-1.0, -1.4)),
            block_arrow(2.0, 0.8, 0.3, 0.6),
            pie(1.0, 0.3, BLUE),
            light_leak(1.0, "s", 1.0),
            starburst(12, &[], 0.0, 8.0),
            rounded_text_box(
                "Hello world, this wraps onto lines",
                0.4,
                WHITE,
                BLUE_E,
                3.0,
            ),
            speech_bubble("Hi!", 0.4),
            thought_bubble("Hmm", 0.4),
            clock(1.0, 3.0, 15.0),
            lightbulb(1.0, true),
            die_face(5, 1.0),
            speedometer(1.0, 0.7),
            progress_bar(4.0, 0.4, GREEN),
        ] {
            assert!(m.leaf_count() > 0 && m.width() > 0.0);
        }
    }

    #[test]
    fn manim_drawings() {
        let d = dartboard(3.0);
        assert_eq!(d.children.len(), 4);
        assert_eq!(d.children[0].children.len(), 20);
        assert!((d.width() - 6.0).abs() < 0.05);
        assert!(video_icon(1.2, BLUE_A).width() > 1.19);
        let b = bubble(text_sized("hi", 0.4, WHITE), 0.25, false);
        assert!(b.children[0].bottom().y < b.children[1].bottom().y - 0.3);
        let b2 = bubble(text_sized("hi", 0.4, WHITE), 0.25, true);
        // stem on the other side
        assert!(b.children[0].edge(DL).x < b2.children[0].edge(DL).x + 1e-6);
        assert_eq!(double_speech_bubble("a", "b", 0.4).children.len(), 2);
    }
}
