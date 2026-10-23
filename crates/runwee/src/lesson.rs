//! Classroom extras for literature lessons: comprehension cards, story maps,
//! character webs, summary/moral cards, a plot mountain, a mood meter,
//! diary and letter pages, and word clouds.
//!
//! Every builder is a pure function of its inputs plus a progress value
//! `p ∈ [0,1]` (and sometimes a clock `t`), so it slots straight into
//! `always_redraw` acts and Rhai scripts.

use crate::color::*;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::path::Rect;
use crate::shapes::*;
use crate::story::{ease_out_back, font_text, measure};

const INK: Color = Color {
    r: 0.2,
    g: 0.18,
    b: 0.17,
    a: 1.0,
};
const CARD: Color = Color {
    r: 1.0,
    g: 0.988,
    b: 0.96,
    a: 1.0,
};

/// Greedy word wrap using real glyph widths.
pub fn wrap_text(s: &str, font: &str, size: f64, max_w: f64) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for w in s.split_whitespace() {
        let trial = if cur.is_empty() {
            w.to_string()
        } else {
            format!("{cur} {w}")
        };
        if !cur.is_empty() && measure(&trial, font, size) > max_w {
            lines.push(std::mem::take(&mut cur));
            cur = w.to_string();
        } else {
            cur = trial;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Left-aligned (or centred) wrapped paragraph whose first line sits at `top_left.y`.
#[allow(clippy::too_many_arguments)]
pub fn paragraph(
    s: &str,
    font: &str,
    size: f64,
    color: Color,
    anchor: Vec2,
    max_w: f64,
    line_h: f64,
    centered: bool,
) -> Mobject {
    let lines = wrap_text(s, font, size, max_w);
    let parts: Vec<Mobject> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let at = anchor + v2(0.0, -(i as f64) * line_h);
            if centered {
                font_text(l, font, size, color).move_to(at)
            } else {
                font_text(l, font, size, color).move_to_aligned(at, v2(-1.0, 0.0))
            }
        })
        .collect();
    group(parts)
}

/// Number of wrapped lines `s` would take.
pub fn line_count(s: &str, font: &str, size: f64, max_w: f64) -> usize {
    wrap_text(s, font, size, max_w).len().max(1)
}

/// Simple emoji-style face; `mood` in [-1, 1] bends the mouth from frown to smile.
pub fn face(center: Vec2, r: f64, mood: f64, color: Color) -> Mobject {
    let m = mood.clamp(-1.0, 1.0);
    let ink = color.darker(0.55);
    let eye_y = center.y + r * 0.22;
    let mouth_y = center.y - r * 0.32 + m * r * 0.08;
    let curve = -m * r * 0.32;
    let mouth = smooth_curve(
        &[
            v2(center.x - r * 0.42, mouth_y),
            v2(center.x, mouth_y + curve),
            v2(center.x + r * 0.42, mouth_y),
        ],
        false,
    )
    .fill(ink, 0.0)
    .stroke(ink, (r * 9.0).max(1.4) as f32);
    let brow = |sx: f64| {
        let tilt = if m < -0.3 { -sx * r * 0.08 } else { 0.0 };
        line(
            v2(center.x + sx * r * 0.5, eye_y + r * 0.26 - tilt),
            v2(center.x + sx * r * 0.18, eye_y + r * 0.26 + tilt),
        )
        .stroke(ink, (r * 6.0).max(1.0) as f32)
    };
    group(vec![
        circle(r)
            .move_to(center)
            .fill(color, 1.0)
            .stroke(ink, (r * 5.0).max(1.0) as f32),
        ellipse(r * 0.16, r * 0.24)
            .move_to(v2(center.x - r * 0.33, eye_y))
            .fill(ink, 1.0)
            .no_stroke(),
        ellipse(r * 0.16, r * 0.24)
            .move_to(v2(center.x + r * 0.33, eye_y))
            .fill(ink, 1.0)
            .no_stroke(),
        brow(-1.0),
        brow(1.0),
        mouth,
    ])
}

/// Colour that walks from cool blue (sad) through sand (neutral) to sunny yellow (happy).
pub fn mood_color(mood: f64) -> Color {
    let m = mood.clamp(-1.0, 1.0);
    let sad = rgb8(120, 160, 220);
    let mid = rgb8(236, 214, 170);
    let happy = rgb8(255, 205, 70);
    if m < 0.0 {
        mid.mix(sad, (-m) as f32)
    } else {
        mid.mix(happy, m as f32)
    }
}

fn card_bg(center: Vec2, w: f64, h: f64, accent: Color) -> Vec<Mobject> {
    vec![
        rounded_rectangle(w, h, 0.16)
            .move_to(center + v2(0.06, -0.07))
            .fill(BLACK, 0.14)
            .no_stroke(),
        rounded_rectangle(w, h, 0.16)
            .move_to(center)
            .fill(CARD, 1.0)
            .stroke(accent, 2.0),
    ]
}

/// Height `comprehension_card` will use for these texts at width `w`
/// (handy for top-aligning a row of cards).
pub fn comprehension_card_height(question: &str, answer: &str, w: f64) -> f64 {
    let size = 0.27;
    let lh = size * 1.3;
    let qn = line_count(question, "story", size, w - 0.6) as f64;
    let an = line_count(answer, "hand", size, w - 0.8) as f64;
    0.75 + qn * lh + 0.35 + an * lh + 0.35
}

/// A comprehension question card. The answer slides into a tinted panel as
/// `reveal` goes 0→1; `p` pops the card in.
pub fn comprehension_card(
    number: usize,
    question: &str,
    answer: &str,
    center: Vec2,
    w: f64,
    p: f64,
    reveal: f64,
) -> Mobject {
    let p = clamp01(p);
    if p <= 0.0 {
        return Mobject::empty();
    }
    let accent = rgb8(60, 110, 190);
    let green = rgb8(40, 130, 80);
    let size = 0.27;
    let lh = size * 1.3;
    let inner = w - 0.6;
    let qn = line_count(question, "story", size, inner) as f64;
    let an = line_count(answer, "hand", size, inner - 0.2) as f64;
    let h = 0.75 + qn * lh + 0.35 + an * lh + 0.35;
    let top = center.y + h * 0.5;
    let left = center.x - w * 0.5 + 0.3;
    let mut parts = card_bg(center, w, h, accent);
    parts.push(
        circle(0.2)
            .move_to(v2(left + 0.2, top - 0.35))
            .fill(accent, 1.0)
            .no_stroke(),
    );
    parts.push(
        font_text(&number.to_string(), "sans", 0.22, WHITE).move_to(v2(left + 0.2, top - 0.35)),
    );
    parts.push(
        font_text("Think & answer", "sans", 0.18, accent)
            .move_to_aligned(v2(left + 0.55, top - 0.35), v2(-1.0, 0.0)),
    );
    parts.push(paragraph(
        question,
        "story",
        size,
        INK,
        v2(left, top - 0.8),
        inner,
        lh,
        false,
    ));
    let r = clamp01(reveal);
    let ay = top - 0.75 - qn * lh - 0.2;
    let ah = an * lh + 0.3;
    if r > 0.0 {
        let e = smoothstep(0.0, 1.0, r);
        parts.push(
            rounded_rectangle(inner + 0.1, ah, 0.1)
                .move_to(v2(center.x, ay - ah * 0.5))
                .fill(rgb8(222, 242, 228), 1.0)
                .no_stroke()
                .opacity(e as f32),
        );
        let n = answer.chars().count();
        let shown: String = answer
            .chars()
            .take((n as f64 * clamp01(r * 1.3)).ceil() as usize)
            .collect();
        parts.push(paragraph(
            &shown,
            "hand",
            size,
            green,
            v2(left + 0.1, ay - 0.22 - size * 0.4),
            inner - 0.2,
            lh,
            false,
        ));
    } else {
        parts.push(
            font_text("? ? ?", "sans", 0.22, rgb8(170, 160, 150))
                .move_to(v2(center.x, ay - ah * 0.5)),
        );
    }
    let e = ease_out_back(p).max(0.01);
    group(parts)
        .scale_about(e, center)
        .opacity(clamp01(p * 2.0) as f32)
}

/// Summary card with a ribbon-banner "moral" that unfurls at the bottom.
pub fn summary_card(
    title: &str,
    summary: &str,
    moral: &str,
    center: Vec2,
    w: f64,
    p: f64,
) -> Mobject {
    let p = clamp01(p);
    if p <= 0.0 {
        return Mobject::empty();
    }
    let accent = rgb8(150, 96, 52);
    let size = 0.3;
    let lh = size * 1.35;
    let inner = w - 0.9;
    let n = line_count(summary, "story", size, inner) as f64;
    let h = 1.0 + n * lh + 1.6;
    let top = center.y + h * 0.5;
    let mut parts = card_bg(center, w, h, accent);
    parts.push(
        rounded_rectangle(w - 0.3, h - 0.3, 0.1)
            .move_to(center)
            .fill(WHITE, 0.0)
            .stroke(accent.lighter(0.4), 1.0),
    );
    parts.push(font_text(title, "story", 0.4, accent.darker(0.2)).move_to(v2(center.x, top - 0.5)));
    // summary types on
    let k = clamp01(p * 1.6);
    let chars = summary.chars().count();
    let shown: String = summary
        .chars()
        .take((chars as f64 * k).ceil() as usize)
        .collect();
    let full = wrap_text(summary, "story", size, inner);
    let mut used = 0usize;
    for (i, l) in full.iter().enumerate() {
        let len = l.chars().count() + 1;
        let take = shown.chars().count().saturating_sub(used).min(len);
        used += len;
        if take == 0 {
            break;
        }
        let part: String = l.chars().take(take).collect();
        parts.push(font_text(&part, "story", size, INK).move_to_aligned(
            v2(center.x - inner * 0.5, top - 1.05 - i as f64 * lh),
            v2(-1.0, 0.0),
        ));
    }
    // ribbon banner
    let rb = clamp01(p * 2.5 - 1.5);
    if rb > 0.0 {
        let by = center.y - h * 0.5 + 0.75;
        let mw = (measure(&format!("Moral: {moral}"), "hand", 0.32) + 1.2).min(w + 0.6)
            * smoothstep(0.0, 1.0, rb);
        let red = rgb8(196, 64, 58);
        let bh = 0.62;
        let tail = |sx: f64| {
            let x = center.x + sx * mw * 0.5;
            polygon(&[
                v2(x - sx * 0.1, by + bh * 0.3),
                v2(x + sx * 0.55, by + bh * 0.3),
                v2(x + sx * 0.35, by),
                v2(x + sx * 0.55, by - bh * 0.3 - 0.1),
                v2(x - sx * 0.1, by - bh * 0.3 - 0.1),
            ])
            .fill(red.darker(0.25), 1.0)
            .no_stroke()
        };
        parts.push(tail(-1.0));
        parts.push(tail(1.0));
        parts.push(
            rectangle(mw.max(0.01), bh)
                .move_to(v2(center.x, by))
                .fill(red, 1.0)
                .no_stroke(),
        );
        parts.push(
            font_text(&format!("Moral: {moral}"), "hand", 0.32, WHITE)
                .move_to(v2(center.x, by))
                .opacity(clamp01(rb * 2.0 - 1.0) as f32),
        );
    }
    let e = ease_out_back(clamp01(p * 3.0)).max(0.01);
    group(parts).scale_about(e, center)
}

/// Six friendly accent colours used across the lesson widgets.
pub const LESSON_COLORS: [Color; 6] = [
    Color {
        r: 0.24,
        g: 0.47,
        b: 0.78,
        a: 1.0,
    },
    Color {
        r: 0.86,
        g: 0.42,
        b: 0.24,
        a: 1.0,
    },
    Color {
        r: 0.24,
        g: 0.6,
        b: 0.38,
        a: 1.0,
    },
    Color {
        r: 0.6,
        g: 0.36,
        b: 0.74,
        a: 1.0,
    },
    Color {
        r: 0.84,
        g: 0.64,
        b: 0.12,
        a: 1.0,
    },
    Color {
        r: 0.78,
        g: 0.28,
        b: 0.4,
        a: 1.0,
    },
];

/// A story map: titled cards (Characters, Setting, Problem, …) laid out in
/// a snake that flows left→right then right→left, linked by arrows.
pub fn story_map(cells: &[(&str, &str)], rect: Rect, p: f64) -> Mobject {
    let n = cells.len();
    if n == 0 {
        return Mobject::empty();
    }
    let cols = if n <= 4 { n } else { n.div_ceil(2) };
    let rows = n.div_ceil(cols);
    let gap = 0.55;
    let cw = (rect.width() - gap * (cols as f64 - 1.0)) / cols as f64;
    let ch = (rect.height() - gap * (rows as f64 - 1.0)) / rows as f64;
    let c0 = rect.center();
    let pos = |i: usize| {
        let row = i / cols;
        let mut col = i % cols;
        if row % 2 == 1 {
            col = cols - 1 - col;
        }
        v2(
            c0.x - rect.width() * 0.5 + cw * 0.5 + col as f64 * (cw + gap),
            c0.y + rect.height() * 0.5 - ch * 0.5 - row as f64 * (ch + gap),
        )
    };
    let mut arrows = Vec::new();
    let mut cards = Vec::new();
    let step = 1.0 / n as f64;
    for (i, (title, body)) in cells.iter().enumerate() {
        let local = clamp01((p - i as f64 * step * 0.85) / (step * 1.2));
        if local <= 0.0 {
            continue;
        }
        let c = pos(i);
        let col = LESSON_COLORS[i % LESSON_COLORS.len()];
        let mut m = card_bg(c, cw, ch, col);
        m.push(
            rounded_rectangle(cw, 0.5, 0.16)
                .move_to(v2(c.x, c.y + ch * 0.5 - 0.25))
                .fill(col, 1.0)
                .no_stroke(),
        );
        m.push(
            rectangle(cw, 0.2)
                .move_to(v2(c.x, c.y + ch * 0.5 - 0.4))
                .fill(col, 1.0)
                .no_stroke(),
        );
        m.push(font_text(title, "sans", 0.22, WHITE).move_to(v2(c.x, c.y + ch * 0.5 - 0.27)));
        let size = 0.3;
        let lines = line_count(body, "hand", size, cw - 0.4) as f64;
        let lh = size * 1.25;
        let body_top = c.y + ch * 0.5 - 0.5 - (ch - 0.5) * 0.5 + (lines - 1.0) * lh * 0.5;
        m.push(paragraph(
            body,
            "hand",
            size,
            INK,
            v2(c.x, body_top),
            cw - 0.4,
            lh,
            true,
        ));
        let e = ease_out_back(local).max(0.01);
        cards.push(
            group(m)
                .scale_about(e, c)
                .opacity(clamp01(local * 3.0) as f32),
        );
        if i > 0 {
            let a = pos(i - 1);
            let (from, to) = if (a.y - c.y).abs() < 1e-6 {
                let s = (c.x - a.x).signum();
                (a + v2(s * cw * 0.5, 0.0), c - v2(s * cw * 0.5, 0.0))
            } else {
                (a - v2(0.0, ch * 0.5), c + v2(0.0, ch * 0.5))
            };
            let ar = clamp01(local * 2.0);
            let tip = from.lerp(to, ar);
            if (tip - from).length() > 0.08 {
                arrows.push(
                    arrow(from, tip)
                        .stroke(rgb8(150, 140, 128), 2.5)
                        .fill(rgb8(150, 140, 128), 1.0),
                );
            }
        }
    }
    arrows.extend(cards);
    group(arrows)
}

/// Character web: the main character in the middle, others around an ellipse,
/// each spoke labelled with the relationship and each node with a trait.
#[allow(clippy::too_many_arguments)]
pub fn character_web(
    main: &str,
    main_trait: &str,
    others: &[(&str, &str, &str)],
    center: Vec2,
    rx: f64,
    ry: f64,
    p: f64,
    t: f64,
) -> Mobject {
    let p = clamp01(p);
    let n = others.len().max(1);
    let mut spokes = Vec::new();
    let mut nodes = Vec::new();
    let hub_r = 0.85;
    let pe = ease_out_back(clamp01(p * 4.0)).max(0.01);
    let hub = group(vec![
        circle(hub_r + 0.08)
            .move_to(center)
            .fill(rgb8(255, 226, 150), 1.0)
            .no_stroke(),
        circle(hub_r)
            .move_to(center)
            .fill(rgb8(255, 244, 214), 1.0)
            .stroke(rgb8(214, 150, 40), 3.0),
        font_text(main, "story", 0.42, INK).move_to(center + v2(0.0, 0.12)),
        font_text(main_trait, "hand", 0.24, rgb8(170, 110, 30)).move_to(center + v2(0.0, -0.3)),
    ])
    .scale_about(pe, center);
    for (i, (name, link, tr)) in others.iter().enumerate() {
        let local = clamp01((p - 0.2 - i as f64 * 0.6 / n as f64) / 0.3);
        if local <= 0.0 {
            continue;
        }
        let ang = std::f64::consts::FRAC_PI_2 - i as f64 * std::f64::consts::TAU / n as f64;
        let bob = (t * 1.6 + i as f64).sin() * 0.04;
        let at = center + v2(rx * ang.cos(), ry * ang.sin() + bob);
        let col = LESSON_COLORS[i % LESSON_COLORS.len()];
        let dir = (at - center).normalize();
        let from = center + dir * (hub_r + 0.05);
        let node_r = 0.42;
        let to = at - dir * (node_r + 0.05);
        let lr = smoothstep(0.0, 0.6, local);
        spokes.push(line(from, from.lerp(to, lr)).stroke(col.mix(WHITE, 0.2), 3.0));
        if lr > 0.6 {
            let mid = from.lerp(to, 0.5);
            let lw = measure(link, "sans", 0.18) + 0.3;
            spokes.push(
                group(vec![
                    rounded_rectangle(lw, 0.34, 0.17)
                        .move_to(mid)
                        .fill(WHITE, 1.0)
                        .stroke(col, 1.5),
                    font_text(link, "sans", 0.18, col.darker(0.25)).move_to(mid),
                ])
                .opacity(clamp01((lr - 0.6) * 4.0) as f32),
            );
        }
        let ne = ease_out_back(clamp01(local * 1.5 - 0.4)).max(0.0);
        if ne > 0.0 {
            let initial: String = name.chars().take(1).collect();
            let below = at.y < center.y - 0.1;
            let text_y = if below {
                at.y - node_r - 0.3
            } else {
                at.y + node_r + 0.5
            };
            let node = group(vec![
                circle(node_r).move_to(at).fill(col, 1.0).stroke(WHITE, 3.0),
                font_text(&initial, "story", 0.42, WHITE).move_to(at),
                font_text(name, "story", 0.3, col.darker(0.3)).move_to(v2(at.x, text_y)),
                font_text(tr, "hand", 0.22, rgb8(110, 100, 92)).move_to(v2(at.x, text_y - 0.3)),
            ]);
            nodes.push(node.scale_about(ne.max(0.01), at));
        }
    }
    spokes.extend(nodes);
    spokes.push(hub);
    group(spokes)
}

/// Sample points of the plot-mountain path through the key points, with flat
/// tangents at every key point (smoothstep easing between them).
pub fn plot_path(keys: &[Vec2], per_seg: usize) -> Vec<Vec2> {
    let mut pts = Vec::new();
    for w in keys.windows(2) {
        for k in 0..per_seg {
            let u = k as f64 / per_seg as f64;
            pts.push(v2(
                lerp(w[0].x, w[1].x, u),
                lerp(w[0].y, w[1].y, smoothstep(0.0, 1.0, u)),
            ));
        }
    }
    if let Some(l) = keys.last() {
        pts.push(*l);
    }
    pts
}

/// Freytag-style plot mountain: exposition, rising action, climax, falling
/// action, resolution (any number of stages ≥ 2 works). Each stage is
/// `(label, event)`. A walker climbs the path as `p` grows. The first and last
/// stage captions sit just outside `rect` (left/right), so leave ~2.5 units there.
pub fn plot_mountain(stages: &[(&str, &str)], rect: Rect, p: f64, t: f64) -> Mobject {
    let n = stages.len();
    if n < 2 {
        return Mobject::empty();
    }
    let p = clamp01(p);
    let c = rect.center();
    let (w, h) = (rect.width(), rect.height());
    let base = c.y - h * 0.5;
    let peak_i = n / 2;
    let keys: Vec<Vec2> = (0..n)
        .map(|i| {
            let u = i as f64 / (n - 1) as f64;
            let x = c.x - w * 0.5 + w * u;
            let lift = if i <= peak_i {
                i as f64 / peak_i as f64
            } else {
                1.0 - (i - peak_i) as f64 / (n - 1 - peak_i) as f64 * 0.8
            };
            let lift = if i == 0 { 0.06 } else { lift };
            v2(x, base + lift * h * 0.62)
        })
        .collect();
    let pts = plot_path(&keys, 40);
    let mut parts = Vec::new();
    // hill body
    let mut body = pts.clone();
    body.push(v2(c.x + w * 0.5, base - 0.05));
    body.push(v2(c.x - w * 0.5, base - 0.05));
    parts.push(
        polygon(&body)
            .fill(rgb8(196, 226, 190), 0.55 * clamp01(p * 3.0) as f32)
            .no_stroke(),
    );
    parts.push(
        line(
            v2(c.x - w * 0.5 - 0.2, base - 0.05),
            v2(c.x + w * 0.5 + 0.2, base - 0.05),
        )
        .stroke(rgb8(150, 140, 128), 2.0),
    );
    let q = p * 1.25;
    let draw = clamp01(q);
    parts.push(
        polyline(&pts)
            .fill(BLACK, 0.0)
            .stroke(rgb8(70, 120, 70), 4.0)
            .partial(0.0, draw.max(0.001)),
    );
    // walker
    let idx = ((pts.len() - 1) as f64 * draw).round() as usize;
    let wp = pts[idx.min(pts.len() - 1)];
    let hop = (t * 9.0).sin().abs() * 0.06 * (if draw < 1.0 { 1.0 } else { 0.0 });
    parts.push(
        circle(0.13)
            .move_to(wp + v2(0.0, 0.16 + hop))
            .fill(rgb8(214, 72, 64), 1.0)
            .stroke(WHITE, 2.0)
            .opacity(clamp01((1.12 - q) * 8.0) as f32),
    );
    for (i, (label, event)) in stages.iter().enumerate() {
        let u = i as f64 / (n - 1) as f64;
        let local = clamp01((q - u) * 5.0);
        if local <= 0.0 {
            continue;
        }
        let k = keys[i];
        let col = LESSON_COLORS[i % LESSON_COLORS.len()];
        parts.push(dot_r(k, 0.1 * ease_out_back(local).max(0.01), col));
        if i == peak_i {
            let fh = 0.55 * smoothstep(0.0, 1.0, local);
            parts.push(line(k, k + v2(0.0, fh)).stroke(INK, 2.0));
            let wave = (t * 4.0).sin() * 0.04;
            parts.push(
                polygon(&[
                    k + v2(0.0, fh),
                    k + v2(0.38, fh - 0.1 + wave),
                    k + v2(0.0, fh - 0.2),
                ])
                .fill(rgb8(214, 72, 64), 1.0)
                .no_stroke(),
            );
        }
        let (anchor, align) = if i == 0 {
            (k + v2(-0.3, 0.05), 1.0)
        } else if i == n - 1 {
            (k + v2(0.3, 0.05), -1.0)
        } else if i < peak_i {
            (k + v2(-0.25, 0.45), 1.0)
        } else if i > peak_i {
            (k + v2(0.25, 0.45), -1.0)
        } else {
            (k + v2(0.0, 0.85), 0.0)
        };
        let size = 0.23;
        let maxw = (w / n as f64) * 1.25;
        let lines = wrap_text(event, "hand", size, maxw);
        let mut lab = vec![
            font_text(&label.to_uppercase(), "sans", 0.19, col.darker(0.2)).move_to_aligned(
                anchor + v2(0.0, lines.len() as f64 * size * 1.2 + 0.08),
                v2(align, 0.0),
            ),
        ];
        for (j, l) in lines.iter().enumerate() {
            lab.push(font_text(l, "hand", size, INK).move_to_aligned(
                anchor + v2(0.0, (lines.len() - 1 - j) as f64 * size * 1.2),
                v2(align, 0.0),
            ));
        }
        parts.push(group(lab).opacity(local as f32));
    }
    group(parts)
}

/// Mood meter: a line graph of a character's feelings through the story.
/// `points` are `(moment label, mood ∈ [-1,1])`; faces ride on each point.
pub fn mood_meter(character: &str, points: &[(&str, f64)], rect: Rect, p: f64) -> Mobject {
    let n = points.len();
    if n == 0 {
        return Mobject::empty();
    }
    let p = clamp01(p);
    let c = rect.center();
    let (w, h) = (rect.width(), rect.height());
    let left = c.x - w * 0.5 + 0.9;
    let right = c.x + w * 0.5 - 0.3;
    let top = c.y + h * 0.5 - 0.5;
    let bottom = c.y - h * 0.5 + 0.75;
    let mid = (top + bottom) * 0.5;
    let ymap = |m: f64| mid + m.clamp(-1.0, 1.0) * (top - bottom) * 0.42;
    let mut parts = vec![rounded_rectangle(w, h, 0.18)
        .move_to(c)
        .fill(CARD, 1.0)
        .stroke(rgb8(214, 200, 180), 1.5)];
    // background bands
    for (lo, hi, col) in [
        (0.0, 1.0, rgb8(255, 244, 200)),
        (-1.0, 0.0, rgb8(224, 236, 250)),
    ] {
        parts.push(
            rectangle(right - left, ymap(hi) - ymap(lo))
                .move_to(v2((left + right) * 0.5, (ymap(hi) + ymap(lo)) * 0.5))
                .fill(col, 0.8)
                .no_stroke(),
        );
    }
    parts.push(dashed_line(v2(left, mid), v2(right, mid)).stroke(rgb8(180, 170, 160), 1.2));
    parts.push(face(v2(left - 0.45, ymap(0.85)), 0.2, 1.0, mood_color(1.0)));
    parts.push(face(
        v2(left - 0.45, ymap(-0.85)),
        0.2,
        -1.0,
        mood_color(-1.0),
    ));
    parts.push(
        font_text(
            &format!("How does {character} feel?"),
            "sans",
            0.22,
            rgb8(110, 100, 92),
        )
        .move_to(v2(c.x, c.y + h * 0.5 - 0.25)),
    );
    let xs: Vec<f64> = (0..n)
        .map(|i| {
            if n == 1 {
                (left + right) * 0.5
            } else {
                left + 0.5 + (right - left - 1.0) * i as f64 / (n - 1) as f64
            }
        })
        .collect();
    let keys: Vec<Vec2> = (0..n).map(|i| v2(xs[i], ymap(points[i].1))).collect();
    let pts = plot_path(&keys, 30);
    let draw = clamp01(p * 1.1);
    if pts.len() > 1 {
        parts.push(
            polyline(&pts)
                .fill(BLACK, 0.0)
                .stroke(rgb8(214, 110, 60), 4.0)
                .partial(0.0, draw.max(0.001)),
        );
    }
    for (i, (label, m)) in points.iter().enumerate() {
        let u = if n == 1 {
            0.0
        } else {
            i as f64 / (n - 1) as f64
        };
        let local = clamp01((draw - u * 0.97) * 7.0);
        parts.push(
            font_text(label, "hand", 0.22, INK)
                .move_to(v2(xs[i], bottom - 0.35))
                .opacity(clamp01(p * 4.0) as f32),
        );
        if local > 0.0 {
            let e = ease_out_back(local).max(0.01);
            parts.push(face(keys[i], 0.27 * e, *m, mood_color(*m)));
        }
    }
    group(parts)
}

/// A semicircular gauge whose needle swings to `mood ∈ [-1,1]`.
pub fn mood_dial(label: &str, mood: f64, center: Vec2, r: f64) -> Mobject {
    let mut parts = Vec::new();
    let segs = 24;
    for k in 0..segs {
        let a0 = std::f64::consts::PI * (1.0 - k as f64 / segs as f64);
        let a1 = std::f64::consts::PI * (1.0 - (k + 1) as f64 / segs as f64);
        let m = -1.0 + 2.0 * (k as f64 + 0.5) / segs as f64;
        parts.push(
            polygon(&[
                center + Vec2::polar(r, a0),
                center + Vec2::polar(r, a1),
                center + Vec2::polar(r * 0.68, a1),
                center + Vec2::polar(r * 0.68, a0),
            ])
            .fill(mood_color(m), 1.0)
            .stroke(mood_color(m), 0.5),
        );
    }
    let m = mood.clamp(-1.0, 1.0);
    let a = std::f64::consts::FRAC_PI_2 * (1.0 - m);
    let tip = center + Vec2::polar(r * 0.9, a);
    parts.push(line(center, tip).stroke(INK, 4.0));
    parts.push(circle(r * 0.08).move_to(center).fill(INK, 1.0).no_stroke());
    parts.push(face(
        center + v2(0.0, -r * 0.35),
        r * 0.22,
        m,
        mood_color(m),
    ));
    parts.push(
        font_text(label, "sans", r * 0.16, rgb8(110, 100, 92)).move_to(center + v2(0.0, -r * 0.78)),
    );
    group(parts)
}

fn reveal_lines(lines: &[String], k: f64) -> Vec<String> {
    let total: usize = lines.iter().map(|l| l.chars().count()).sum();
    let mut left = (total as f64 * clamp01(k)).ceil() as usize;
    let mut out = Vec::new();
    for l in lines {
        if left == 0 {
            break;
        }
        let n = l.chars().count();
        let take = left.min(n);
        out.push(l.chars().take(take).collect());
        left -= take;
    }
    out
}

/// A ruled notebook diary page (spiral holes, red margin) whose entry is
/// "handwritten" as `p` grows.
pub fn diary_page(date: &str, entry: &str, signature: &str, rect: Rect, p: f64) -> Mobject {
    let c = rect.center();
    let (w, h) = (rect.width(), rect.height());
    let top = c.y + h * 0.5;
    let left = c.x - w * 0.5;
    let rule = 0.47;
    let mut parts = vec![
        rectangle(w, h)
            .move_to(c + v2(0.07, -0.08))
            .fill(BLACK, 0.15)
            .no_stroke(),
        rectangle(w, h)
            .move_to(c)
            .fill(rgb8(253, 251, 243), 1.0)
            .stroke(rgb8(214, 206, 190), 1.0),
    ];
    let mut y = top - 0.9;
    while y > c.y - h * 0.5 + 0.2 {
        parts.push(line(v2(left + 0.1, y), v2(left + w - 0.1, y)).stroke(rgb8(170, 200, 230), 1.0));
        y -= rule;
    }
    parts.push(
        line(v2(left + 0.85, top), v2(left + 0.85, top - h)).stroke(rgb8(226, 120, 120), 1.4),
    );
    let mut hy = top - 0.4;
    while hy > c.y - h * 0.5 + 0.2 {
        parts.push(
            circle(0.08)
                .move_to(v2(left + 0.35, hy))
                .fill(rgb8(220, 214, 200), 1.0)
                .no_stroke(),
        );
        hy -= 0.6;
    }
    let ink = rgb8(40, 60, 130);
    let size = 0.31;
    parts.push(
        font_text(date, "hand", 0.26, rgb8(200, 80, 80))
            .move_to_aligned(v2(left + w - 0.3, top - 0.5), v2(1.0, 0.0)),
    );
    let mut lines = vec!["Dear Diary,".to_string()];
    lines.extend(wrap_text(entry, "hand", size, w - 1.4));
    lines.push(format!("— {signature}"));
    let shown = reveal_lines(&lines, p);
    let nl = lines.len();
    for (i, l) in shown.iter().enumerate() {
        let y = top - 0.9 - rule * (i as f64 + 1.0) + 0.06;
        let x = if i == nl - 1 {
            left + w - 0.4 - measure(&lines[i], "hand", size)
        } else {
            left + 1.05
        };
        let (at, al) = (v2(x, y), v2(-1.0, -1.0));
        parts.push(font_text(l, "hand", size, ink).move_to_aligned(at, al));
    }
    group(parts)
}

/// An informal letter laid out the way the school syllabus teaches it:
/// address & date top-right, salutation, body, closing and signature.
#[allow(clippy::too_many_arguments)]
pub fn letter_page(
    address: &[&str],
    date: &str,
    salutation: &str,
    body: &str,
    closing: &str,
    signature: &str,
    rect: Rect,
    p: f64,
) -> Mobject {
    let c = rect.center();
    let (w, h) = (rect.width(), rect.height());
    let top = c.y + h * 0.5;
    let left = c.x - w * 0.5 + 0.45;
    let right = c.x + w * 0.5 - 0.45;
    let size = 0.26;
    let lh = size * 1.35;
    let ink = rgb8(60, 50, 44);
    let mut parts = vec![
        rectangle(w, h)
            .move_to(c + v2(0.07, -0.08))
            .fill(BLACK, 0.15)
            .no_stroke(),
        rectangle(w, h)
            .move_to(c)
            .fill(rgb8(250, 244, 228), 1.0)
            .stroke(rgb8(214, 196, 160), 1.2),
    ];
    // (text, x anchor, right-aligned?, y)
    let mut items: Vec<(String, f64, bool, f64)> = Vec::new();
    let mut y = top - 0.5;
    for a in address.iter().chain(std::iter::once(&date)) {
        items.push((a.to_string(), right, true, y));
        y -= lh;
    }
    y -= lh * 0.5;
    items.push((salutation.to_string(), left, false, y));
    y -= lh * 1.2;
    for (i, l) in wrap_text(body, "story", size, right - left - 0.4)
        .into_iter()
        .enumerate()
    {
        let x = if i == 0 { left + 0.4 } else { left };
        items.push((l, x, false, y));
        y -= lh;
    }
    y -= lh * 0.4;
    items.push((closing.to_string(), right, true, y));
    y -= lh;
    items.push((signature.to_string(), right, true, y));
    let lines: Vec<String> = items.iter().map(|i| i.0.clone()).collect();
    let shown = reveal_lines(&lines, p);
    for (k, s) in shown.iter().enumerate() {
        let (full, x, ra, y) = &items[k];
        let font = if k == items.len() - 1 {
            "hand"
        } else {
            "story"
        };
        let sz = if font == "hand" { size * 1.25 } else { size };
        let m = if *ra {
            let fw = measure(full, font, sz);
            font_text(s, font, sz, ink).move_to_aligned(v2(x - fw, *y), v2(-1.0, 0.0))
        } else {
            font_text(s, font, sz, ink).move_to_aligned(v2(*x, *y), v2(-1.0, 0.0))
        };
        parts.push(m);
    }
    group(parts)
}

/// One placed word of a [`word_cloud`].
#[derive(Clone, Debug)]
pub struct CloudWord {
    pub index: usize,
    pub rank: usize,
    pub pos: Vec2,
    pub size: f64,
    pub font: &'static str,
    pub w: f64,
    pub h: f64,
}

/// Lays out a word cloud: words sized by weight, placed on an outward spiral
/// without overlaps (axis-aligned box test). Deterministic for a given `seed`;
/// words that do not fit inside `w × h` are dropped.
pub fn word_cloud_layout(
    words: &[(&str, f64)],
    center: Vec2,
    w: f64,
    h: f64,
    seed: u64,
) -> Vec<CloudWord> {
    let mut sorted: Vec<(usize, &str, f64)> = words
        .iter()
        .enumerate()
        .map(|(i, (s, wt))| (i, *s, *wt))
        .collect();
    sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let maxw = sorted.first().map(|s| s.2).unwrap_or(1.0).max(1e-6);
    let fonts = ["story", "hand", "sans"];
    let aspect = (w / h).sqrt();
    let mut out: Vec<CloudWord> = Vec::new();
    for (rank, (i, word, wt)) in sorted.iter().enumerate() {
        let size = 0.3 + 0.85 * (wt / maxw).max(0.0).sqrt();
        let font = fonts[(i + seed as usize) % 3];
        let bw = measure(word, font, size) + 0.18;
        let bh = size * 1.05;
        let mut a = (seed as f64 * 0.7 + *i as f64 * 2.3) % std::f64::consts::TAU;
        let mut r = 0.0;
        for _ in 0..1500 {
            let cand = center + v2(r * a.cos() * aspect, r * a.sin() / aspect * 0.9);
            let fits = (cand.x - bw * 0.5) >= center.x - w * 0.5
                && (cand.x + bw * 0.5) <= center.x + w * 0.5
                && (cand.y - bh * 0.5) >= center.y - h * 0.5
                && (cand.y + bh * 0.5) <= center.y + h * 0.5;
            let free = out.iter().all(|o| {
                (cand.x - o.pos.x).abs() * 2.0 >= bw + o.w
                    || (cand.y - o.pos.y).abs() * 2.0 >= bh + o.h
            });
            if fits && free {
                out.push(CloudWord {
                    index: *i,
                    rank,
                    pos: cand,
                    size,
                    font,
                    w: bw,
                    h: bh,
                });
                break;
            }
            a += 0.35;
            r += 0.012;
        }
    }
    out
}

/// Word cloud whose words pop in from the heaviest to the lightest as `p` grows.
pub fn word_cloud(
    words: &[(&str, f64)],
    center: Vec2,
    w: f64,
    h: f64,
    seed: u64,
    p: f64,
) -> Mobject {
    let lay = word_cloud_layout(words, center, w, h, seed);
    let n = words.len() as f64;
    let mut parts = Vec::new();
    for cw in &lay {
        let col = LESSON_COLORS[(cw.index + seed as usize) % LESSON_COLORS.len()]
            .darker(0.08 * (cw.rank % 3) as f32);
        let local = clamp01(p * (n * 0.35 + 1.0) - cw.rank as f64 * 0.35);
        if local > 0.0 {
            let e = ease_out_back(local).max(0.01);
            parts.push(
                font_text(words[cw.index].0, cw.font, cw.size, col)
                    .move_to(cw.pos)
                    .scale_about(e, cw.pos)
                    .opacity(clamp01(local * 2.0) as f32),
            );
        }
    }
    group(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping() {
        let lines = wrap_text(
            "the quick brown fox jumps over the lazy dog again and again",
            "story",
            0.3,
            2.5,
        );
        assert!(lines.len() >= 3);
        for l in &lines {
            assert!(measure(l, "story", 0.3) <= 2.5 || !l.contains(' '));
        }
        assert_eq!(
            lines.join(" "),
            "the quick brown fox jumps over the lazy dog again and again"
        );
    }

    #[test]
    fn path_and_mood() {
        let keys = [v2(0.0, 0.0), v2(1.0, 2.0), v2(2.0, 0.5)];
        let pts = plot_path(&keys, 10);
        assert_eq!(pts.len(), 21);
        assert!((pts[10] - keys[1]).length() < 1e-9);
        assert!(pts.iter().all(|q| q.y <= 2.0 + 1e-9));
        let happy = mood_color(1.0);
        let sad = mood_color(-1.0);
        assert!(happy.r > sad.r && sad.b > happy.b);
    }

    #[test]
    fn reveal_counts() {
        let lines = vec!["abc".to_string(), "de".to_string()];
        assert!(reveal_lines(&lines, 0.0).is_empty());
        assert_eq!(reveal_lines(&lines, 0.6), vec!["abc".to_string()]);
        assert_eq!(reveal_lines(&lines, 1.0), lines);
    }

    #[test]
    fn cloud_places_without_overlap() {
        let words = [
            ("kindness", 5.0),
            ("lion", 4.0),
            ("mouse", 4.0),
            ("net", 3.0),
            ("hunter", 2.0),
            ("gnaw", 2.0),
            ("roar", 1.5),
            ("small", 1.0),
            ("friend", 3.0),
        ];
        let lay = word_cloud_layout(&words, Vec2::ZERO, 9.0, 4.0, 3);
        assert!(lay.len() >= 8, "placed {}", lay.len());
        for (k, a) in lay.iter().enumerate() {
            assert!(
                a.pos.x.abs() + a.w * 0.5 <= 4.5 + 1e-9 && a.pos.y.abs() + a.h * 0.5 <= 2.0 + 1e-9
            );
            for b in &lay[k + 1..] {
                let apart = (a.pos.x - b.pos.x).abs() * 2.0 >= a.w + b.w - 1e-9
                    || (a.pos.y - b.pos.y).abs() * 2.0 >= a.h + b.h - 1e-9;
                assert!(apart, "{} overlaps {}", a.index, b.index);
            }
        }
        assert_eq!(lay[0].index, 0, "heaviest word goes first");
    }

    #[test]
    fn widgets_smoke() {
        let r = Rect::from_center(Vec2::ZERO, 10.0, 5.0);
        assert!(
            comprehension_card(
                1,
                "Why did the lion laugh?",
                "Because the mouse was tiny.",
                Vec2::ZERO,
                5.0,
                1.0,
                1.0
            )
            .leaf_count()
                > 4
        );
        assert!(
            summary_card(
                "Summary",
                "A lion spares a mouse.",
                "Kindness is never wasted",
                Vec2::ZERO,
                8.0,
                1.0
            )
            .leaf_count()
                > 4
        );
        assert!(
            story_map(
                &[
                    ("Who", "Lion"),
                    ("Where", "Forest"),
                    ("Problem", "Net"),
                    ("End", "Free")
                ],
                r,
                1.0
            )
            .leaf_count()
                > 8
        );
        assert!(
            character_web(
                "Lion",
                "proud",
                &[("Mouse", "spares", "tiny"), ("Hunter", "traps", "sly")],
                Vec2::ZERO,
                3.0,
                2.0,
                1.0,
                0.0
            )
            .leaf_count()
                > 6
        );
        assert!(
            plot_mountain(
                &[("a", "x"), ("b", "y"), ("c", "z"), ("d", "w"), ("e", "v")],
                r,
                1.0,
                0.0
            )
            .leaf_count()
                > 10
        );
        assert!(
            mood_meter(
                "Mouse",
                &[("start", 0.2), ("caught", -0.9), ("free", 1.0)],
                r,
                1.0
            )
            .leaf_count()
                > 5
        );
        assert!(mood_dial("now", 0.5, Vec2::ZERO, 1.0).leaf_count() > 20);
        assert!(diary_page("Monday", "Today I met a lion.", "Mouse", r, 1.0).leaf_count() > 5);
        assert!(
            letter_page(
                &["12 Park Road"],
                "1 May",
                "Dear Ravi,",
                "Hello there.",
                "Yours lovingly,",
                "Asha",
                r,
                1.0
            )
            .leaf_count()
                > 5
        );
        assert!(comprehension_card(2, "Q", "A", Vec2::ZERO, 5.0, 0.0, 0.0).leaf_count() == 0);
    }
}
