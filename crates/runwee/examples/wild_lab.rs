//! Phase A5 — wild extras: Verlet cloth & ropes, a boids flock, a procedurally
//! synthesised beat with perfectly synced audio-reactive visuals, and stylize
//! filters (Kuwahara, Game Boy dither, pixel sort, ASCII, CRT).
use runwee::audio::spectrum_at;
use runwee::effects::Effect;
use runwee::prelude::*;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(12, 12, 20));

    // ── Act 1: Verlet cloth & ropes ─────────────────────────────────────
    let t0 = s.time();
    let (cols, rows, sp) = (26, 17, 0.2);
    let ball = |t: f64| {
        let u = ((t - 0.8) / 3.2).clamp(0.0, 1.0);
        (
            v2(-3.0 + 1.2 * (u * TAU).sin(), -1.2 + 1.9 * (u * PI).sin()),
            0.6,
        )
    };
    let cloth = Verlet::cloth(v2(-5.6, 2.9), cols, rows, sp, 5)
        .with_wind(|p, t| {
            v2(
                3.0 + 3.0 * (t * 1.3 + p.y).sin(),
                0.6 * (t * 2.1 + p.x * 0.7).cos(),
            ) + curl2(p * 0.5, t, 4) * 2.0
        })
        .with_collider(ball);
    let (cloth_b, _) = cloth.bake(5.0, 60.0, 4);
    let cloth_b = Arc::new(cloth_b);
    let mut ropes = vec![];
    for k in 0..5 {
        let a = v2(1.6 + k as f64 * 1.0, 3.2);
        let mut r = Verlet::rope(a, a + v2(1.6 + 0.3 * k as f64, 0.0), 24);
        r.damping = 0.998;
        let (b, _) = r.bake(5.0, 60.0, 4);
        ropes.push(b);
    }
    let ropes = Arc::new(ropes);
    let sim = s.always_redraw(move |ctx| {
        let t = ctx.t - t0;
        let mut kids = vec![cloth_mobject(
            &cloth_b.at(t),
            cols,
            rows,
            &[rgb8(230, 70, 90), rgb8(250, 240, 230), rgb8(60, 110, 220)],
            sp,
        )];
        let (c, r) = ball(t);
        kids.push(
            circle(r - 0.05)
                .fill(rgb8(255, 200, 80), 1.0)
                .no_stroke()
                .move_to(c),
        );
        let rope_cols = [
            rgb8(255, 200, 80),
            rgb8(255, 120, 90),
            rgb8(240, 90, 200),
            rgb8(120, 160, 255),
            rgb8(90, 230, 190),
        ];
        for (k, b) in ropes.iter().enumerate() {
            let p = b.at(t);
            kids.push(rope_mobject(&p, rope_cols[k], 4.0));
            kids.push(
                circle(0.16)
                    .fill(rope_cols[k], 1.0)
                    .no_stroke()
                    .move_to(*p.last().unwrap()),
            );
        }
        kids.push(line(v2(1.2, 3.2), v2(6.0, 3.2)).stroke(GRAY_C, 3.0));
        group(kids)
    });
    let lbl = s.add(
        bold_text("Verlet cloth & ropes (baked, deterministic)", 0.36, WHITE).to_edge(DOWN, 0.25),
    );
    s.wait(5.0);
    s.remove(sim);
    s.remove(lbl);

    // ── Act 2: boids ─────────────────────────────────────────────────────
    let t1 = s.time();
    let target = |t: f64| v2(4.5 * (t * 0.7).sin(), 2.2 * (t * 1.1).sin());
    let flock = Arc::new(bake_boids(
        BoidOpts {
            count: 160,
            ..Default::default()
        },
        5.0,
        30.0,
        Some(&target),
    ));
    let fl = s.always_redraw(move |ctx| {
        let t = ctx.t - t1;
        let p = flock.at(t);
        let v = flock.velocity(t);
        let cols = [
            rgb8(120, 220, 255),
            rgb8(160, 140, 255),
            rgb8(255, 140, 200),
        ];
        let mut kids = vec![];
        for k in 1..6 {
            let pp = flock.at(t - k as f64 * 0.06);
            kids.push(group(
                pp.iter()
                    .map(|q| {
                        dot_r(
                            *q,
                            0.025,
                            rgb8(120, 160, 255).with_alpha(0.35 - 0.05 * k as f32),
                        )
                    })
                    .collect::<Vec<_>>(),
            ));
        }
        kids.push(boids_mobject(&p, &v, 0.14, &cols));
        group(kids)
    });
    let lbl = s.add(
        bold_text("Boids: separation · alignment · cohesion", 0.36, WHITE).to_edge(DOWN, 0.25),
    );
    s.wait(4.5);
    s.remove(fl);
    s.remove(lbl);

    // ── Act 3: drum machine + audio-reactive visuals ─────────────────────
    let t2 = s.time();
    let dm = DrumMachine::new(118.0, 3)
        .track(Voice::Kick, "X...x...X..x.x..")
        .track(Voice::Snare, "....X.......X...")
        .track(Voice::Hat, "x.xxx.x.x.xxx.xX")
        .track(Voice::Bass, "1..1..5.3..1..6.");
    let audio = dm.render();
    s.audio.add(AudioClip::new(audio.clone(), t2));
    let hits = Arc::new(dm.hits());
    let dur = dm.duration();
    let viz = s.always_redraw(move |ctx| {
        let t = ctx.t - t2;
        let kick = hit_envelope(&hits, Voice::Kick, t, 0.12);
        let snare = hit_envelope(&hits, Voice::Snare, t, 0.18);
        let hat = hit_envelope(&hits, Voice::Hat, t, 0.05);
        let bass = hit_envelope(&hits, Voice::Bass, t, 0.3);
        let spec = spectrum_at(&audio, t.max(0.0), 64);
        let mut kids = vec![];
        if snare > 0.05 {
            kids.push(
                rectangle(16.0, 9.0)
                    .fill(rgb8(255, 255, 255), (snare * 0.18) as f32)
                    .no_stroke(),
            );
        }
        // spectrum ring
        let n = spec.len() * 2;
        for i in 0..n {
            let j = if i < spec.len() { i } else { n - 1 - i };
            let v = &spec[j];
            let a = TAU * i as f64 / n as f64 + t * 0.3;
            let r0 = 1.9 + 0.3 * kick;
            let len = 0.1 + 1.8 * *v as f64;
            let d = Vec2::from_angle(a);
            kids.push(
                line(d * r0, d * (r0 + len))
                    .stroke(palette("sunset", j as f64 / spec.len() as f64), 5.0),
            );
        }
        // kick core
        kids.push(
            circle(1.2 + 0.45 * kick)
                .fill_paint(
                    Paint::conic(
                        Vec2::ZERO,
                        t * 2.0,
                        &[
                            rgb8(255, 80, 140),
                            rgb8(120, 90, 255),
                            rgb8(80, 220, 255),
                            rgb8(255, 80, 140),
                        ],
                    ),
                    1.0,
                )
                .no_stroke()
                .glow(0.3 + 0.5 * kick, Some(rgb8(255, 120, 200))),
        );
        // bass waveform orbit
        let wave: Vec<Vec2> = (0..=120)
            .map(|k| {
                let a = TAU * k as f64 / 120.0;
                let r = 3.4 + 0.25 * bass * (a * 6.0 + t * 8.0).sin();
                Vec2::from_angle(a) * r
            })
            .collect();
        kids.push(polyline(&wave).stroke(
            rgb8(120, 255, 200).with_alpha((0.3 + 0.7 * bass) as f32),
            3.0,
        ));
        // hats sparkle
        for k in 0..12 {
            let a = TAU * k as f64 / 12.0 - t;
            kids.push(dot_r(
                Vec2::from_angle(a) * 3.9,
                0.04 + 0.08 * hat,
                WHITE.with_alpha((0.3 + 0.7 * hat) as f32),
            ));
        }
        // step indicator
        let step = ((t / (60.0 / 118.0 / 4.0)).floor() as i64).rem_euclid(16);
        for k in 0..16 {
            let on = k == step;
            kids.push(
                rectangle(0.32, 0.12)
                    .fill(if on { rgb8(255, 220, 90) } else { GRAY_E }, 1.0)
                    .no_stroke()
                    .move_to(v2(-3.0 + k as f64 * 0.4, -3.7)),
            );
        }
        group(kids)
    });
    let lbl = s.add(
        bold_text(
            "Synth drum machine → exact-hit audio-reactive visuals",
            0.34,
            WHITE,
        )
        .to_edge(UP, 0.2),
    );
    s.wait(dur);
    s.remove(viz);
    s.remove(lbl);

    // ── Act 4: stylize filters ──────────────────────────────────────────
    let t3 = s.time();
    let bg = field_image(Rect::from_center(Vec2::ZERO, 14.3, 8.1), 360, |p| {
        let v = domain_warp2(p * 0.35, 3.0, 0.0, 2) * 0.5 + 0.5;
        gradient_at(
            &[
                rgb8(20, 30, 90),
                rgb8(40, 150, 200),
                rgb8(250, 200, 90),
                rgb8(240, 80, 120),
            ],
            v,
        )
    });
    s.add(bg.clone());
    let names = [
        "Kuwahara (painterly)",
        "Bayer dither · Game Boy",
        "Pixel sort",
        "ASCII",
        "CRT monitor",
    ];
    let seg = 1.6;
    let shapes = s.always_redraw(move |ctx| {
        let t = ctx.t - t3;
        let k = ((t / seg).floor() as usize).min(names.len() - 1);
        group(vec![
            star(5, 2.1, 0.95)
                .fill(rgb8(255, 240, 220), 1.0)
                .stroke(rgb8(30, 20, 40), 6.0)
                .rotate(t * 0.6)
                .move_to(v2(-3.6, 0.3)),
            circle(1.4)
                .fill_paint(
                    Paint::radial(
                        v2(-0.4, 0.4),
                        1.6,
                        &[WHITE, rgb8(255, 90, 140), rgb8(80, 0, 60)],
                    ),
                    1.0,
                )
                .no_stroke()
                .move_to(v2(3.6, 0.2 + 0.3 * (t * 2.0).sin())),
            bold_text(names[k], 0.6, WHITE).move_to(v2(0.0, -2.8)),
        ])
    });
    let gb = vec![
        rgb8(15, 56, 15),
        rgb8(48, 98, 48),
        rgb8(139, 172, 15),
        rgb8(155, 188, 15),
    ];
    s.post_effect(t3, t3 + seg * 5.0, move |u| {
        let k = ((u / seg).floor() as usize).min(4);
        vec![match k {
            0 => Effect::Kuwahara(0.012),
            1 => Effect::Dither {
                levels: 2,
                cell: 0.005,
                palette: gb.clone(),
            },
            2 => Effect::PixelSort {
                threshold: 0.35 + 0.25 * ((u - 2.0 * seg) / seg).clamp(0.0, 1.0) as f32,
                vertical: false,
            },
            3 => Effect::Ascii {
                cell: 0.022,
                color: None,
            },
            _ => Effect::Crt {
                curvature: 0.12,
                scanlines: 0.55,
            },
        }]
    });
    s.wait(seg * 5.0);
    s.remove(shapes);
    run(&s, "wild_lab")
}
