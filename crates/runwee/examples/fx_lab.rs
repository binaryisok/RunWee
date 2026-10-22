//! Phase A3 — FX lab: particle systems (fire, sparks, snow, confetti, fireworks,
//! smoke), conic gradients, the full blend-mode set and kinetic typography
//! (decode/scramble, glyph waves, glitch, text on a path, glyph fly-ins).
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(10, 10, 18));

    // ── Act 1: kinetic typography ───────────────────────────────────────
    let t0 = s.time();
    let decode = s.always_redraw(move |ctx| {
        let u = ((ctx.t - t0) / 2.2).clamp(0.0, 1.0);
        scramble_text("DECODING THE SIGNAL", u, 0.62, WHITE, rgb8(80, 255, 160), 4)
            .move_to(v2(0.0, 2.6))
    });
    let wave_src = bold_text("waves of glyphs", 0.7, rgb8(255, 200, 90)).move_to(v2(0.0, 1.0));
    let wave = s.always_redraw(move |ctx| glyph_wave(&wave_src, ctx.t, 0.18, 2.4, 0.7));
    let glitch_src = bold_text("GLITCH", 1.0, WHITE).move_to(v2(-3.4, -1.3));
    let gl = s.always_redraw(move |ctx| glitch(&glitch_src, ctx.t, 1.0, 9));
    let circle_path = {
        let mut p = Path::new();
        let c = v2(3.4, -1.3);
        let n = 64;
        for i in 0..=n {
            let a = PI - TAU * i as f64 / n as f64;
            let q = c + v2(a.cos(), a.sin()) * 1.25;
            if i == 0 {
                p.move_to(q);
            } else {
                p.line_to(q);
            }
        }
        p
    };
    let ring = s.always_redraw(move |ctx| {
        text_on_path(
            "text on a path • text on a path • text on a path • ",
            &circle_path,
            0.3,
            rgb8(120, 200, 255),
            -((ctx.t * 0.06) % 1.0) * 0.0,
        )
        .rotate_about(-ctx.t * 0.8, v2(3.4, -1.3))
    });
    s.wait(4.0);
    let fly = bold_text("fly-in!", 0.8, rgb8(255, 110, 170)).move_to(v2(0.0, -3.1));
    let fly_o = s.register(fly);
    s.play(vec![glyph_fly_in(fly_o, v2(0.0, 2.5), 2.0, 0.1, 0.12)]);
    s.wait(1.2);
    for o in [decode, wave, gl, ring, fly_o] {
        s.remove(o);
    }

    // ── Act 2: conic gradients + blend modes ────────────────────────────
    let t1 = s.time();
    let title = s.add(bold_text("Conic gradients & 18 blend modes", 0.42, WHITE).to_edge(UP, 0.25));
    let cone = s.always_redraw(move |ctx| {
        let a = (ctx.t - t1) * 1.4;
        let rainbow = [RED, YELLOW, GREEN, TEAL, BLUE, PURPLE, RED];
        let c = v2(-4.6, 0.3);
        group(vec![
            circle(1.6)
                .fill_paint(Paint::conic(Vec2::ZERO, a, &rainbow), 1.0)
                .no_stroke()
                .move_to(c),
            annulus(0.6, 1.0)
                .fill_paint(
                    Paint::conic(Vec2::ZERO, -2.0 * a, &[WHITE, rgb8(20, 20, 40), WHITE]),
                    1.0,
                )
                .no_stroke()
                .move_to(c),
        ])
    });
    let blends = s.always_redraw(move |ctx| {
        let t = ctx.t - t1;
        let mut cells = vec![];
        for (i, b) in Blend::ALL.iter().enumerate() {
            let (col, row) = (i % 6, i / 6);
            let c = v2(-1.75 + col as f64 * 1.25, 1.3 - row as f64 * 1.55);
            let wob = 0.12 * (t * 2.0 + i as f64).sin();
            let base = rectangle(1.05, 0.95)
                .fill_paint(
                    Paint::linear(
                        v2(-0.5, 0.0),
                        v2(0.5, 0.0),
                        &[rgb8(255, 120, 40), rgb8(40, 120, 255)],
                    ),
                    1.0,
                )
                .no_stroke()
                .move_to(c);
            let mut top = circle(0.36)
                .fill(rgb8(120, 255, 140), 1.0)
                .no_stroke()
                .move_to(c + v2(0.18 + wob, 0.1));
            top.style.blend = *b;
            let top2 = {
                let mut m = circle(0.3)
                    .fill(rgb8(230, 60, 220), 1.0)
                    .no_stroke()
                    .move_to(c + v2(-0.2, -0.15 - wob));
                m.style.blend = *b;
                m
            };
            cells.push(group(vec![
                group(vec![base, top, top2]),
                text_sized(b.name(), 0.16, GRAY_A).move_to(c + v2(0.0, -0.62)),
            ]));
        }
        group(cells)
    });
    s.wait(4.0);
    for o in [title, cone, blends] {
        s.remove(o);
    }

    // ── Act 3: particles ────────────────────────────────────────────────
    let t2 = s.time();
    let title = s.add(bold_text("Particles", 0.42, WHITE).to_edge(UP, 0.25));
    let fire = Emitter::fire(v2(-4.8, -3.2)).seed(1).starting(t2);
    let smoke = Emitter::smoke(v2(-4.8, -1.2)).seed(2).starting(t2);
    let sparks = Emitter::sparks(v2(-1.6, -3.0)).seed(3).starting(t2);
    let snow = Emitter::snow(v2(1.3, 4.3), v2(6.9, 4.4))
        .seed(4)
        .starting(t2 - 6.0);
    let confetti = Emitter::confetti(v2(1.4, -3.4), 1.75).seed(5).starting(t2);
    let mut fw = vec![fire, smoke, sparks, snow, confetti];
    for (i, (p, c)) in [
        (v2(-2.2, 2.0), rgb8(255, 120, 60)),
        (v2(0.6, 2.4), rgb8(120, 200, 255)),
        (v2(3.3, 1.7), rgb8(255, 90, 200)),
        (v2(5.4, 2.6), rgb8(255, 230, 110)),
    ]
    .into_iter()
    .enumerate()
    {
        fw.push(Emitter::firework(
            p,
            t2 + 0.4 + 0.7 * i as f64,
            c,
            10 + i as u64,
        ));
        fw.push(Emitter::firework(
            p + v2(0.4, -0.6),
            t2 + 2.8 + 0.6 * i as f64,
            c,
            20 + i as u64,
        ));
    }
    let ps = particle_systems(&mut s, fw);
    s.wait(5.5);
    s.remove(ps);
    s.remove(title);
    run(&s, "fx_lab")
}
