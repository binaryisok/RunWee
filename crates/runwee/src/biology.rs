//! Biology: cells (animal & plant) with organelles, mitochondrion, chloroplast,
//! DNA double helix (animated, base-paired), RNA, neuron with myelinated axon and
//! travelling action potential, virus, bacterium, red blood cell, chromosomes,
//! mitosis stages, phospholipid bilayer, enzyme lock-and-key, Punnett squares,
//! population growth models and labelled diagrams with leader lines.

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::shapes::*;
use crate::style::*;
use crate::text::text_sized;

fn blob(center: Vec2, rx: f64, ry: f64, wobble: f64, seed: u64) -> Path {
    let mut rng = Rng::new(seed);
    let k1 = rng.range(0.0, TAU);
    let k2 = rng.range(0.0, TAU);
    let pts: Vec<Vec2> = (0..48)
        .map(|i| {
            let a = TAU * i as f64 / 48.0;
            let r = 1.0 + wobble * ((3.0 * a + k1).sin() * 0.6 + (5.0 * a + k2).sin() * 0.4);
            center + v2(rx * r * a.cos(), ry * r * a.sin())
        })
        .collect();
    Path::smooth_through(&pts, true)
}

fn filled(p: Path, fill: Color, opacity: f32, stroke: Color, w: f32) -> Mobject {
    Mobject::from_path(p).fill(fill, opacity).stroke(stroke, w)
}

/// Mitochondrion with folded cristae.
pub fn mitochondrion(length: f64, color: Color) -> Mobject {
    let (w, h) = (length, length * 0.45);
    let outer = rounded_rectangle(w, h, h / 2.0)
        .fill(color, 0.9)
        .stroke(color.darker(0.4), 3.0);
    let inner_w = w * 0.86;
    let n = 7;
    let mut pts = Vec::new();
    for i in 0..=n * 4 {
        let x = -inner_w / 2.0 + inner_w * i as f64 / (n * 4) as f64;
        let y = match i % 4 {
            1 => h * 0.32,
            3 => -h * 0.32,
            _ => 0.0,
        };
        pts.push(v2(x, y));
    }
    let cristae = smooth_curve(&pts, false).stroke(color.lighter(0.45), 3.0);
    Mobject::group([outer, cristae]).with_tag("mitochondrion")
}

/// Chloroplast with grana stacks.
pub fn chloroplast(length: f64) -> Mobject {
    let (w, h) = (length, length * 0.5);
    let mut g = Mobject::group([ellipse(w, h).fill(GREEN_D, 0.95).stroke(GREEN_E, 3.0)]);
    for k in 0..4 {
        let x = -w * 0.3 + k as f64 * w * 0.2;
        for j in 0..3 {
            g.push(
                rounded_rectangle(w * 0.12, h * 0.1, h * 0.04)
                    .move_to(v2(x, -h * 0.12 + j as f64 * h * 0.12))
                    .fill(GREEN_A, 1.0)
                    .no_stroke(),
            );
        }
    }
    g.with_tag("chloroplast")
}

/// Golgi apparatus (stacked curved cisternae).
pub fn golgi(size: f64) -> Mobject {
    Mobject::group((0..5).map(|i| {
        let r = size * (0.6 + 0.12 * i as f64);
        arc_centered(v2(0.0, -size * 0.9), r, PI / 2.0 - 0.6, 1.2)
            .stroke(GOLD, 7.0)
            .line_cap(LineCap::Round)
    }))
    .with_tag("golgi")
}

/// Rough ER: wavy membranes studded with ribosomes.
pub fn endoplasmic_reticulum(size: f64, rough: bool) -> Mobject {
    let mut g = Mobject::empty();
    for j in 0..3 {
        let y = j as f64 * size * 0.22;
        let pts: Vec<Vec2> = (0..=12)
            .map(|i| {
                v2(
                    -size / 2.0 + size * i as f64 / 12.0,
                    y + size * 0.06 * (i as f64 * 1.3 + j as f64).sin(),
                )
            })
            .collect();
        g.push(smooth_curve(&pts, false).stroke(PINK, 4.0));
        if rough {
            for p in pts.iter().step_by(2) {
                g.push(dot_r(*p + UP * 0.05, size * 0.025, BLUE_E));
            }
        }
    }
    g.with_tag("er")
}

/// Animal cell with nucleus, nucleolus, mitochondria, ER, Golgi, ribosomes, lysosomes.
pub fn animal_cell(radius: f64) -> Mobject {
    let r = radius;
    let mut rng = Rng::new(42);
    let membrane = filled(
        blob(Vec2::ZERO, r, r * 0.82, 0.05, 7),
        rgb8(250, 200, 160),
        0.35,
        rgb8(240, 150, 110),
        5.0,
    )
    .with_tag("membrane");
    let nucleus = Mobject::group([
        circle(r * 0.3).fill(PURPLE_D, 0.85).stroke(PURPLE_A, 3.0),
        circle(r * 0.1)
            .shift(v2(r * 0.07, r * 0.05))
            .fill(PURPLE_E, 1.0)
            .no_stroke()
            .with_tag("nucleolus"),
    ])
    .shift(v2(-r * 0.05, r * 0.05))
    .with_tag("nucleus");
    let mut g = Mobject::group([membrane, nucleus]);
    for (p, a) in [
        (v2(r * 0.5, r * 0.3), 0.4),
        (v2(-r * 0.55, -r * 0.3), -0.6),
        (v2(r * 0.35, -r * 0.45), 1.2),
    ] {
        g.push(mitochondrion(r * 0.35, ORANGE).rotate(a).move_to(p));
    }
    g.push(
        endoplasmic_reticulum(r * 0.5, true)
            .rotate(0.3)
            .move_to(v2(-r * 0.15, -r * 0.48)),
    );
    g.push(golgi(r * 0.2).move_to(v2(r * 0.55, -r * 0.05)));
    for _ in 0..30 {
        let a = rng.range(0.0, TAU);
        let d = rng.range(r * 0.38, r * 0.72);
        g.push(dot_r(v2(d * a.cos(), d * a.sin() * 0.82), r * 0.012, BLUE_E).with_tag("ribosome"));
    }
    for p in [v2(-r * 0.6, r * 0.35), v2(r * 0.1, r * 0.55)] {
        g.push(
            circle_at(p, r * 0.06)
                .fill(MAROON, 0.9)
                .stroke(MAROON_E, 2.0)
                .with_tag("lysosome"),
        );
    }
    g
}

/// Plant cell with wall, large central vacuole, chloroplasts and nucleus.
pub fn plant_cell(width: f64, height: f64) -> Mobject {
    let wall = rounded_rectangle(width, height, width * 0.06)
        .fill(GREEN_E, 0.25)
        .stroke(GREEN_E, 8.0)
        .with_tag("cell wall");
    let membrane = rounded_rectangle(width * 0.94, height * 0.92, width * 0.05)
        .fill(GREEN_A, 0.18)
        .stroke(GREEN_B, 3.0)
        .with_tag("membrane");
    let vacuole = filled(
        blob(v2(width * 0.05, 0.0), width * 0.3, height * 0.3, 0.06, 3),
        BLUE_B,
        0.45,
        BLUE_D,
        3.0,
    )
    .with_tag("vacuole");
    let nucleus = circle(width * 0.1)
        .move_to(v2(-width * 0.32, height * 0.25))
        .fill(PURPLE_D, 0.9)
        .stroke(PURPLE_A, 3.0)
        .with_tag("nucleus");
    let mut g = Mobject::group([wall, membrane, vacuole, nucleus]);
    for (i, p) in [
        v2(-0.38, -0.3),
        v2(0.38, 0.32),
        v2(0.4, -0.3),
        v2(-0.05, 0.38),
        v2(-0.36, 0.02),
    ]
    .iter()
    .enumerate()
    {
        g.push(
            chloroplast(width * 0.16)
                .rotate(i as f64 * 0.7)
                .move_to(v2(p.x * width, p.y * height)),
        );
    }
    g.push(mitochondrion(width * 0.12, ORANGE).move_to(v2(width * 0.15, -height * 0.38)));
    g
}

/// DNA base pairing colours.
pub fn base_color(b: char) -> Color {
    match b {
        'A' => RED_C,
        'T' => YELLOW_D,
        'G' => GREEN_C,
        'C' => BLUE_C,
        'U' => ORANGE,
        _ => GRAY,
    }
}

pub fn complement(b: char) -> char {
    match b {
        'A' => 'T',
        'T' => 'A',
        'G' => 'C',
        'C' => 'G',
        _ => 'N',
    }
}

/// Double helix along x of `length`, `turns` twists, rotated by `phase` (animate it).
/// Strands are depth-shaded; rungs show the base pairs of `sequence` (repeats).
pub fn dna_helix(length: f64, radius: f64, turns: f64, phase: f64, sequence: &str) -> Mobject {
    let seq: Vec<char> = if sequence.is_empty() {
        "ATGCGTAC".chars().collect()
    } else {
        sequence.chars().collect()
    };
    let n = 160;
    let strand = |offset: f64| -> Vec<(Vec2, f64)> {
        (0..=n)
            .map(|i| {
                let x = -length / 2.0 + length * i as f64 / n as f64;
                let a = TAU * turns * i as f64 / n as f64 + phase + offset;
                (v2(x, radius * a.sin()), a.cos())
            })
            .collect()
    };
    let s1 = strand(0.0);
    let s2 = strand(PI * 0.75);
    let mut back = Mobject::empty();
    let mut front = Mobject::empty();
    let rungs = (turns * 10.0).round() as usize;
    for k in 0..rungs {
        let i = (k * n / rungs.max(1)).min(n);
        let (p1, z1) = s1[i];
        let (p2, z2) = s2[i];
        let b = seq[k % seq.len()];
        let mid = p1.lerp(p2, 0.5);
        let depth = (z1 + z2) / 2.0;
        let op = (0.55 + 0.45 * depth.max(-1.0)) as f32;
        let rung = Mobject::group([
            line(p1, mid).stroke(base_color(b), 5.0).stroke_opacity(op),
            line(mid, p2)
                .stroke(base_color(complement(b)), 5.0)
                .stroke_opacity(op),
        ]);
        if depth >= 0.0 {
            front.push(rung);
        } else {
            back.push(rung);
        }
    }
    let draw_strand = |s: &[(Vec2, f64)], c: Color| -> (Mobject, Mobject) {
        let (mut b, mut f) = (Mobject::empty(), Mobject::empty());
        for w in s.windows(2) {
            let z = (w[0].1 + w[1].1) / 2.0;
            let col = c.darker(((1.0 - z) * 0.3) as f32);
            let seg = line(w[0].0, w[1].0).stroke(col, 7.0 + 3.0 * z as f32);
            if z >= 0.0 {
                f.push(seg);
            } else {
                b.push(seg);
            }
        }
        (b, f)
    };
    let (b1, f1) = draw_strand(&s1, BLUE_B);
    let (b2, f2) = draw_strand(&s2, PURPLE_A);
    Mobject::group([b1, b2, back, front, f1, f2]).with_tag("dna")
}

/// Single-stranded RNA with bases.
pub fn rna_strand(sequence: &str, spacing: f64) -> Mobject {
    let n = sequence.chars().count();
    let pts: Vec<Vec2> = (0..n)
        .map(|i| v2(i as f64 * spacing, 0.15 * (i as f64 * 0.9).sin()))
        .collect();
    let mut g = Mobject::group([smooth_curve(&pts, false).stroke(ORANGE, 5.0)]);
    for (i, b) in sequence.chars().enumerate() {
        g.push(line(pts[i], pts[i] + UP * 0.45).stroke(base_color(b), 6.0));
        g.push(text_sized(&b.to_string(), 0.25, WHITE).move_to(pts[i] + UP * 0.65));
    }
    let c = g.center();
    g.shift(-c)
}

fn branch(g: &mut Mobject, p: Vec2, angle: f64, len: f64, depth: u32, rng: &mut Rng, color: Color) {
    if depth == 0 {
        return;
    }
    let q = p + Vec2::polar(len, angle);
    let mid = p.lerp(q, 0.5) + Vec2::polar(len * 0.12, angle + PI / 2.0) * rng.range(-1.0, 1.0);
    g.push(smooth_curve(&[p, mid, q], false).stroke(color, 1.5 + depth as f32 * 1.6));
    for s in [-1.0, 1.0] {
        branch(
            g,
            q,
            angle + s * rng.range(0.3, 0.6),
            len * 0.65,
            depth - 1,
            rng,
            color,
        );
    }
}

/// Neuron: soma with dendrites, myelinated axon (Schwann cells, nodes of Ranvier),
/// axon terminals. `signal` in [0,1] shows an action potential travelling the axon.
pub fn neuron(length: f64, signal: Option<f64>) -> Mobject {
    let soma_c = v2(-length * 0.42, 0.0);
    let soma_r = length * 0.07;
    let col = rgb8(120, 200, 240);
    let mut rng = Rng::new(9);
    let mut dend = Mobject::empty();
    for k in 0..6 {
        let a = PI * 0.5 + PI * k as f64 / 5.0;
        branch(
            &mut dend,
            soma_c + Vec2::polar(soma_r * 0.9, a),
            a,
            length * 0.09,
            3,
            &mut rng,
            col,
        );
    }
    let axon_start = soma_c + RIGHT * soma_r;
    let axon_end = v2(length * 0.4, 0.0);
    let axon = line(axon_start, axon_end).stroke(col, 5.0);
    let mut myelin = Mobject::empty();
    let segs = 5;
    let span = axon_end.x - axon_start.x;
    for i in 0..segs {
        let x0 = axon_start.x + span * (i as f64 + 0.15) / segs as f64;
        let w = span / segs as f64 * 0.75;
        myelin.push(
            rounded_rectangle(w, length * 0.05, length * 0.025)
                .move_to(v2(x0 + w / 2.0, 0.0))
                .fill(GOLD_E, 0.95)
                .no_stroke(),
        );
    }
    let mut terminals = Mobject::empty();
    for k in -2..=2 {
        let a = k as f64 * 0.35;
        let end = axon_end + Vec2::polar(length * 0.08, a);
        terminals.push(line(axon_end, end).stroke(col, 4.0));
        terminals.push(dot_r(end, length * 0.014, col.lighter(0.3)));
    }
    let soma = Mobject::group([
        circle_at(soma_c, soma_r)
            .fill(col, 0.85)
            .stroke(col.lighter(0.3), 3.0),
        circle_at(soma_c, soma_r * 0.4)
            .fill(BLUE_E, 1.0)
            .no_stroke(),
    ]);
    let mut g = Mobject::group([
        dend.with_tag("dendrites"),
        axon,
        myelin.with_tag("myelin"),
        terminals.with_tag("terminals"),
        soma.with_tag("soma"),
    ]);
    if let Some(s) = signal {
        let x = axon_start.x + span * s.clamp(0.0, 1.0);
        g.push(
            dot_r(v2(x, 0.0), length * 0.025, YELLOW)
                .glow(length * 0.04, Some(YELLOW))
                .with_tag("signal"),
        );
    }
    g
}

/// Virus: icosahedral-ish capsid with surface spikes.
pub fn virus(radius: f64, color: Color) -> Mobject {
    let mut g = Mobject::empty();
    let spikes = 14;
    for i in 0..spikes {
        let a = TAU * i as f64 / spikes as f64;
        let p = Vec2::polar(radius, a);
        let q = Vec2::polar(radius * 1.35, a);
        g.push(line(p, q).stroke(color.lighter(0.2), 4.0));
        g.push(dot_r(q, radius * 0.08, color.lighter(0.4)));
    }
    g.push(
        regular_polygon(6, radius)
            .rotate(PI / 6.0)
            .no_stroke()
            .sheen(color, UL),
    );
    g.push(
        regular_polygon(6, radius * 0.6)
            .rotate(PI / 6.0)
            .stroke(color.darker(0.4), 2.0)
            .fill_opacity(0.0),
    );
    g.with_tag("virus")
}

/// Rod bacterium with flagella.
pub fn bacterium(length: f64, color: Color) -> Mobject {
    let body = rounded_rectangle(length, length * 0.38, length * 0.19)
        .no_stroke()
        .sheen(color, UL);
    let mut g = Mobject::empty();
    for k in -1..=1 {
        let start = v2(length / 2.0, k as f64 * length * 0.08);
        g.push(
            wave_line(
                start,
                start + v2(length * 0.8, k as f64 * length * 0.15),
                length * 0.25,
                length * 0.04,
                0.0,
            )
            .stroke(color.lighter(0.3), 2.5),
        );
    }
    g.push(body);
    g.with_tag("bacterium")
}

/// Red blood cell (biconcave disc look).
pub fn red_blood_cell(radius: f64) -> Mobject {
    let mut outer = circle(radius).no_stroke();
    outer.style.fill = Paint::Radial {
        center: Vec2::ZERO,
        radius,
        focal: Vec2::ZERO,
        stops: vec![
            (0.0, RED_E.darker(0.3)),
            (0.55, RED_D),
            (0.8, RED_B),
            (1.0, RED_E),
        ],
    };
    outer.style.fill_opacity = 1.0;
    outer.with_tag("rbc")
}

/// X-shaped (replicated) chromosome; `single` draws one chromatid.
pub fn chromosome(height: f64, color: Color, single: bool) -> Mobject {
    let w = height * 0.18;
    let arm = |dx: f64, tilt: f64| {
        rounded_rectangle(w, height, w / 2.0)
            .rotate(tilt)
            .shift(v2(dx, 0.0))
            .fill(color, 0.95)
            .stroke(color.darker(0.4), 2.0)
    };
    let mut g = if single {
        Mobject::group([arm(0.0, 0.0)])
    } else {
        Mobject::group([arm(-w * 0.45, 0.2), arm(w * 0.45, -0.2)])
    };
    g.push(dot_r(Vec2::ZERO, w * 0.35, color.darker(0.5)).with_tag("centromere"));
    g
}

/// Mitosis stage drawing for `t` in [0, 5): 0 interphase, 1 prophase, 2 metaphase,
/// 3 anaphase, 4 telophase/cytokinesis (continuous in between).
pub fn mitosis(t: f64, size: f64) -> Mobject {
    let stage = t.clamp(0.0, 4.999);
    let cols = [RED_C, BLUE_C];
    let pinch = ((stage - 3.6) / 1.2).clamp(0.0, 1.0);
    let w = size * (1.0 + 0.35 * ((stage - 3.0) / 1.0).clamp(0.0, 1.0));
    // cell outline pinches during cytokinesis
    let pts: Vec<Vec2> = (0..64)
        .map(|i| {
            let a = TAU * i as f64 / 64.0;
            let squeeze = 1.0 - 0.85 * pinch * (-(a.cos() * 3.0).powi(2)).exp();
            v2(w * 0.5 * a.cos(), size * 0.42 * a.sin() * squeeze)
        })
        .collect();
    let mut g = Mobject::group([smooth_curve(&pts, true)
        .fill(rgb8(250, 200, 160), 0.3)
        .stroke(rgb8(240, 150, 110), 4.0)]);
    if stage < 1.0 {
        g.push(circle(size * 0.2).fill(PURPLE_D, 0.8).stroke(PURPLE_A, 3.0));
        return g;
    }
    // spindle fibres from poles during metaphase/anaphase
    let pole = w * 0.42;
    let sep = if stage < 3.0 {
        0.0
    } else {
        ((stage - 3.0) / 1.0).clamp(0.0, 1.0) * w * 0.28
    };
    if (1.5..4.5).contains(&stage) {
        for k in -3..=3 {
            let y = k as f64 * size * 0.05;
            g.push(
                line(v2(-pole, 0.0), v2(-sep * 0.8, y))
                    .stroke(GRAY_B, 1.5)
                    .stroke_opacity(0.6),
            );
            g.push(
                line(v2(pole, 0.0), v2(sep * 0.8, y))
                    .stroke(GRAY_B, 1.5)
                    .stroke_opacity(0.6),
            );
        }
    }
    let n = 4;
    for i in 0..n {
        let c = cols[i % 2];
        let y_meta = (i as f64 - 1.5) * size * 0.1;
        let scattered = v2(
            ((i * 37) % 7) as f64 / 7.0 * size * 0.3 - size * 0.15,
            ((i * 53) % 5) as f64 / 5.0 * size * 0.3 - size * 0.15,
        );
        let to_plate = ((stage - 1.0) / 1.0).clamp(0.0, 1.0);
        let p = scattered.lerp(v2(0.0, y_meta), to_plate);
        if stage < 3.0 {
            g.push(
                chromosome(size * 0.13, c, false)
                    .rotate(PI / 2.0 * to_plate)
                    .move_to(p),
            );
        } else {
            for s in [-1.0, 1.0] {
                g.push(
                    chromosome(size * 0.11, c, true)
                        .rotate(PI / 2.0)
                        .move_to(v2(s * sep, y_meta)),
                );
            }
        }
    }
    if stage > 4.2 {
        for s in [-1.0, 1.0] {
            g.push(
                circle(size * 0.12)
                    .move_to(v2(s * w * 0.28, 0.0))
                    .stroke(PURPLE_A, 2.5)
                    .stroke_opacity(((stage - 4.2) / 0.5).min(1.0) as f32),
            );
        }
    }
    g
}

/// Name of the mitosis stage for `t` (see [`mitosis`]).
pub fn mitosis_stage_name(t: f64) -> &'static str {
    match t as i64 {
        i64::MIN..=0 => "Interphase",
        1 => "Prophase",
        2 => "Metaphase",
        3 => "Anaphase",
        _ => "Telophase",
    }
}

/// Phospholipid bilayer strip (heads + tails).
pub fn lipid_bilayer(width: f64, units: usize) -> Mobject {
    let mut g = Mobject::empty();
    let dx = width / units as f64;
    for i in 0..units {
        let x = -width / 2.0 + dx * (i as f64 + 0.5);
        for side in [1.0, -1.0] {
            let head = v2(x, side * 0.42);
            g.push(dot_r(head, dx * 0.38, ORANGE));
            g.push(
                line(
                    head - UP * side * 0.08 + LEFT * dx * 0.12,
                    head - UP * side * 0.4 + LEFT * dx * 0.12,
                )
                .stroke(YELLOW_E, 2.0),
            );
            g.push(
                line(
                    head - UP * side * 0.08 + RIGHT * dx * 0.12,
                    head - UP * side * 0.4 + RIGHT * dx * 0.12,
                )
                .stroke(YELLOW_E, 2.0),
            );
        }
    }
    g
}

/// Enzyme + substrate (lock and key); `bound` in [0,1] slides the substrate in.
pub fn enzyme_substrate(size: f64, bound: f64) -> Mobject {
    let s = size;
    let mut p = Path::new();
    p.move_to(v2(-s, -s * 0.6));
    p.line_to(v2(s, -s * 0.6));
    p.line_to(v2(s, s * 0.4));
    p.line_to(v2(s * 0.3, s * 0.4));
    p.line_to(v2(s * 0.3, s * 0.05));
    p.line_to(v2(-s * 0.3, s * 0.05));
    p.line_to(v2(-s * 0.3, s * 0.4));
    p.line_to(v2(-s, s * 0.4));
    p.close();
    let enzyme = Mobject::from_path(p)
        .no_stroke()
        .sheen(TEAL, UL)
        .with_tag("enzyme");
    let y = lerp(s * 1.3, s * 0.22, bound.clamp(0.0, 1.0));
    let substrate = rectangle(s * 0.58, s * 0.34)
        .move_to(v2(0.0, y))
        .no_stroke()
        .sheen(ORANGE, UL)
        .with_tag("substrate");
    Mobject::group([enzyme, substrate])
}

/// Genotype offspring grid for a monohybrid/dihybrid cross, e.g. ("Aa", "Aa") or ("AaBb", "AaBb").
pub fn punnett_square(p1: &str, p2: &str, cell: f64) -> Mobject {
    let gametes = |g: &str| -> Vec<String> {
        let c: Vec<char> = g.chars().collect();
        match c.len() {
            2 => vec![c[0].to_string(), c[1].to_string()],
            4 => {
                let mut v = Vec::new();
                for a in [c[0], c[1]] {
                    for b in [c[2], c[3]] {
                        v.push(format!("{a}{b}"));
                    }
                }
                v
            }
            _ => vec![g.to_string()],
        }
    };
    let (g1, g2) = (gametes(p1), gametes(p2));
    let mut out = Mobject::empty();
    let n = g1.len();
    let m = g2.len();
    for (j, a) in g1.iter().enumerate() {
        out.push(text_sized(a, cell * 0.35, YELLOW).move_to(v2((j as f64 + 1.0) * cell, 0.0)));
    }
    for (i, b) in g2.iter().enumerate() {
        out.push(text_sized(b, cell * 0.35, YELLOW).move_to(v2(0.0, -(i as f64 + 1.0) * cell)));
    }
    for (i, b) in g2.iter().enumerate() {
        for (j, a) in g1.iter().enumerate() {
            let geno = combine_alleles(a, b);
            let dominant = geno.chars().any(|c| c.is_uppercase());
            let c = v2((j as f64 + 1.0) * cell, -(i as f64 + 1.0) * cell);
            out.push(
                square(cell)
                    .move_to(c)
                    .fill(if dominant { BLUE_E } else { MAROON_E }, 0.6)
                    .stroke(WHITE, 2.0)
                    .with_tag(&geno),
            );
            out.push(text_sized(&geno, cell * 0.3, WHITE).move_to(c));
        }
    }
    let _ = (n, m);
    let c = out.center();
    out.shift(-c)
}

/// Combine two gametes into a sorted genotype per gene ("aA" → "Aa").
pub fn combine_alleles(a: &str, b: &str) -> String {
    let (ca, cb): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut out = String::new();
    for k in 0..ca.len().min(cb.len()) {
        let mut pair = [ca[k], cb[k]];
        pair.sort_by_key(|c| (c.to_ascii_lowercase(), c.is_lowercase()));
        out.push(pair[0]);
        out.push(pair[1]);
    }
    out
}

/// Phenotype ratio counts of a cross (dominant count per gene pattern), e.g. 3:1 or 9:3:3:1.
pub fn phenotype_ratio(p1: &str, p2: &str) -> Vec<(String, usize)> {
    let sq = punnett_square(p1, p2, 1.0);
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    sq.for_each(&mut |m| {
        if let Some(t) = m.tag() {
            if t.len() >= 2 && m.path.subpaths.len() == 1 && m.style.fill_opacity > 0.0 {
                let pheno: String = t
                    .as_bytes()
                    .chunks(2)
                    .map(|g| {
                        let up = g.iter().any(|c| (*c as char).is_uppercase());
                        let letter = (g[0] as char).to_ascii_uppercase();
                        if up {
                            letter.to_string()
                        } else {
                            letter.to_ascii_lowercase().to_string()
                        }
                    })
                    .collect();
                *counts.entry(pheno).or_default() += 1;
            }
        }
    });
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    v
}

/// Logistic population N(t) = K / (1 + ((K − N0)/N0) e^{−rt}).
pub fn logistic(t: f64, k: f64, r: f64, n0: f64) -> f64 {
    k / (1.0 + ((k - n0) / n0) * (-r * t).exp())
}

/// Exponential growth N0 e^{rt}.
pub fn exponential_growth(t: f64, r: f64, n0: f64) -> f64 {
    n0 * (r * t).exp()
}

/// Lotka–Volterra predator–prey trajectory: state `[prey, predator]`.
pub fn predator_prey(
    alpha: f64,
    beta: f64,
    delta: f64,
    gamma: f64,
    x0: f64,
    y0: f64,
    duration: f64,
) -> crate::sim::Trajectory {
    crate::sim::rk4(
        move |_, s| {
            vec![
                alpha * s[0] - beta * s[0] * s[1],
                delta * s[0] * s[1] - gamma * s[1],
            ]
        },
        &[x0, y0],
        duration,
        1.0 / 200.0,
    )
}

/// Attach labels with leader lines to points of a diagram. Labels go to the left or
/// right column depending on which side the point is on.
pub fn labeled_diagram(diagram: Mobject, labels: &[(&str, Vec2)], size: f64) -> Mobject {
    let b = diagram.bounds_or_origin();
    let mut g = Mobject::group([diagram]);
    let (mut left, mut right): (Vec<&(&str, Vec2)>, Vec<&(&str, Vec2)>) =
        labels.iter().partition(|(_, p)| p.x < b.center().x);
    left.sort_by(|a, b| b.1.y.partial_cmp(&a.1.y).unwrap());
    right.sort_by(|a, b| b.1.y.partial_cmp(&a.1.y).unwrap());
    for (side, list) in [(-1.0, left), (1.0, right)] {
        let n = list.len().max(1);
        for (i, (name, p)) in list.iter().enumerate() {
            let y = b.max.y - b.height() * (i as f64 + 0.5) / n as f64;
            let x = if side < 0.0 {
                b.min.x - 0.6
            } else {
                b.max.x + 0.6
            };
            let t = text_sized(name, size, WHITE);
            let t = if side < 0.0 {
                t.move_to_aligned(v2(x, y), RIGHT)
            } else {
                t.move_to_aligned(v2(x, y), LEFT)
            };
            g.push(Mobject::group([
                dot_r(*p, 0.05, YELLOW),
                polyline(&[*p, v2(x - side * 0.15, y), v2(x - side * 0.05, y)]).stroke(GRAY_A, 2.0),
                t,
            ]));
        }
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biology_builds() {
        for m in [
            animal_cell(3.0),
            plant_cell(5.0, 3.5),
            mitochondrion(1.5, ORANGE),
            chloroplast(1.5),
            golgi(1.0),
            endoplasmic_reticulum(1.5, true),
            dna_helix(8.0, 0.8, 3.0, 0.4, "ATGC"),
            rna_strand("AUGGCU", 0.4),
            neuron(8.0, Some(0.5)),
            virus(1.0, PURPLE),
            bacterium(2.0, GREEN),
            red_blood_cell(0.8),
            chromosome(1.5, RED, false),
            lipid_bilayer(6.0, 20),
            enzyme_substrate(1.0, 0.5),
        ] {
            assert!(m.leaf_count() > 0 && m.width() > 0.0);
        }
        for t in [0.5, 1.5, 2.5, 3.5, 4.5] {
            assert!(mitosis(t, 4.0).leaf_count() > 1, "{t}");
        }
        assert_eq!(mitosis_stage_name(2.3), "Metaphase");
    }

    #[test]
    fn genetics() {
        assert_eq!(combine_alleles("a", "A"), "Aa");
        assert_eq!(combine_alleles("ab", "AB"), "AaBb");
        let r = phenotype_ratio("Aa", "Aa");
        assert_eq!(r, vec![("A".to_string(), 3), ("a".to_string(), 1)]);
        let r2 = phenotype_ratio("AaBb", "AaBb");
        assert_eq!(r2.iter().map(|x| x.1).collect::<Vec<_>>(), vec![9, 3, 3, 1]);
        assert!((logistic(100.0, 1000.0, 0.5, 10.0) - 1000.0).abs() < 1e-6);
        let pp = predator_prey(1.0, 0.1, 0.075, 1.5, 10.0, 5.0, 5.0);
        assert!(pp.at(2.5).iter().all(|v| *v > 0.0));
        let d = labeled_diagram(
            animal_cell(2.0),
            &[("Nucleus", Vec2::ZERO), ("Membrane", v2(2.0, 0.0))],
            0.3,
        );
        assert_eq!(d.children.len(), 3);
    }
}
