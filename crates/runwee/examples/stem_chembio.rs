//! Phase S3 — chemistry & biology: hydrogen orbitals computed from ψ(n, l, m),
//! a reaction energy profile with a catalysed path, strong- vs weak-acid titration
//! curves with a live indicator flask, an agent-based SIR epidemic with its curves,
//! and radioactive decay of 192 individual nuclei.
use runwee::prelude::chembio::*;
use runwee::prelude::*;
use std::sync::Arc;

fn caption(s: &str, at: Vec2) -> Mobject {
    text_sized(s, 0.26, GRAY_A).move_to(at)
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(8, 10, 18));

    // ── Act 1: orbitals (0 – 8 s) ──
    s.add(bold_text("Hydrogen orbitals from ψ(n, l, m)", 0.42, WHITE).to_edge(UP, 0.25));
    let orbitals: [(u32, u32, i32, f64, &str); 8] = [
        (1, 0, 0, 4.0, ""),
        (2, 0, 0, 13.0, ""),
        (2, 1, 0, 12.0, "z"),
        (3, 1, 0, 22.0, "z"),
        (3, 2, 0, 20.0, "z²"),
        (3, 2, 1, 20.0, "xz"),
        (4, 3, 0, 32.0, "z³"),
        (4, 2, 0, 30.0, "z²"),
    ];
    let size = 2.75;
    for (k, &(n, l, m, ext, sub)) in orbitals.iter().enumerate() {
        let c = v2(-5.1 + 3.4 * (k % 4) as f64, 1.45 - 3.2 * (k / 4) as f64);
        let rect = Rect::from_center(c, size, size);
        let img = orbital_slice(rect, 260, n, l, m, ext, false);
        let label = text_sized(
            &format!("{}{}   (n={n}, l={l}, m={m})", orbital_name(n, l), sub),
            0.25,
            WHITE,
        )
        .move_to(c + v2(0.0, -size / 2.0 - 0.18));
        let o = s.register(img);
        let lo = s.register(label);
        s.play(vec![fade_in(o).run_time(0.45), fade_in(lo).run_time(0.45)]);
    }
    s.add(caption(
        "xz-plane slices of |ψ|²; orange ψ > 0, blue ψ < 0 (nodes are dark)",
        v2(0.0, -3.72),
    ));
    s.wait(4.4);
    s.remove_all();

    // ── Act 2: energetics & titration (8 – 17 s) ──
    s.add(bold_text("Reaction energy & acid–base titration", 0.42, WHITE).to_edge(UP, 0.25));
    s.always_redraw(|ctx| {
        energy_profile(5.6, 4.6, 40.0, 120.0, -30.0, Some(78.0), ctx.elapsed / 3.0)
            .move_to(v2(-3.6, -0.3))
    });
    s.add(caption(
        "exothermic reaction; a catalyst lowers Ea, not ΔH",
        v2(-3.6, -3.35),
    ));
    let ax = Arc::new(
        Axes::new((0.0, 50.0, 10.0), (0.0, 14.0, 2.0))
            .size(5.4, 4.6)
            .at(v2(3.7, -0.3)),
    );
    s.add(ax.build());
    s.add(text_sized("pH", 0.26, GRAY_A).move_to(ax.c2p(0.0, 14.0) + v2(0.4, 0.3)));
    s.add(text_sized("mL of 0.1 M NaOH added", 0.24, GRAY_A).move_to(ax.c2p(40.0, 1.0)));
    s.add(dashed_line(ax.c2p(0.0, 7.0), ax.c2p(50.0, 7.0)).stroke(GRAY_D, 1.2));
    s.add(caption(
        "25 mL of 0.1 M HCl (white) vs acetic acid (orange)",
        v2(3.7, -3.35),
    ));
    let ka = 1.8e-5;
    let ax2 = ax.clone();
    s.always_redraw(move |ctx| {
        let vmax = (ctx.elapsed * 6.0).min(50.0);
        let curve = |weak: Option<f64>| -> Vec<Vec2> {
            (0..=300)
                .map(|k| vmax * k as f64 / 300.0)
                .map(|v| ax2.c2p(v, titration_ph(0.1, 25.0, 0.1, v, weak)))
                .collect()
        };
        let ph_weak = titration_ph(0.1, 25.0, 0.1, vmax, Some(ka));
        let flask_at = ax2.c2p(8.0, 10.6);
        let mut kids = vec![
            polyline(&curve(None)).stroke(WHITE, 3.0),
            polyline(&curve(Some(ka))).stroke(ORANGE, 3.0),
            dot_r(ax2.c2p(vmax, ph_weak), 0.07, ORANGE),
            chemistry::erlenmeyer_flask(1.3, 0.45, indicator_color("phenolphthalein", ph_weak))
                .move_to(flask_at),
            text_sized(&format!("pH {ph_weak:.2}"), 0.26, WHITE).move_to(flask_at + v2(1.25, 0.0)),
        ];
        if vmax > 13.0 {
            let p = ax2.c2p(12.5, -ka.log10());
            kids.push(dot_r(p, 0.06, YELLOW));
            kids.push(text_sized("pH = pKa", 0.22, YELLOW).move_to(p + v2(0.0, 0.32)));
        }
        if vmax > 26.0 {
            let p = ax2.c2p(25.0, 7.0);
            kids.push(dot_r(p, 0.06, WHITE));
            kids.push(text_sized("equivalence", 0.22, WHITE).move_to(p + v2(1.0, -0.25)));
        }
        group(kids)
    });
    s.wait(9.0);
    s.remove_all();

    // ── Act 3: epidemic + decay (17 – 27 s) ──
    s.add(bold_text("Epidemics & radioactive decay", 0.42, WHITE).to_edge(UP, 0.25));
    let bx = Rect::from_center(v2(-3.7, -0.35), 5.6, 5.4);
    s.add(rectangle(5.7, 5.5).move_to(bx.center()).stroke(GRAY_C, 1.5));
    let epi = Arc::new(Epidemic::simulate(240, bx, 0.7, 0.16, 0.25, 2.2, 10.0, 11));
    let e1 = epi.clone();
    s.always_redraw(move |ctx| {
        let (pos, st) = e1.state(ctx.elapsed);
        group(
            pos.iter()
                .zip(st)
                .map(|(p, s)| dot_r(*p, 0.055, sir_color(*s)))
                .collect::<Vec<_>>(),
        )
    });
    s.add(caption(
        "240 random walkers; contact within 0.16 may infect",
        v2(-3.7, -3.35),
    ));
    let cax = Arc::new(
        Axes::new((0.0, 10.0, 2.0), (0.0, 1.0, 0.25))
            .size(5.4, 2.4)
            .at(v2(3.7, 1.25)),
    );
    s.add(cax.build());
    let e2 = epi.clone();
    let cax2 = cax.clone();
    s.always_redraw(move |ctx| {
        let tmax = ctx.elapsed.min(10.0);
        let n = 240.0;
        let series = |k: usize| -> Vec<Vec2> {
            (0..=200)
                .map(|j| tmax * j as f64 / 200.0)
                .map(|t| {
                    let c = e2.counts(t);
                    let v = [c.0, c.1, c.2][k] as f64 / n;
                    cax2.c2p(t, v)
                })
                .collect()
        };
        let (a, b, c) = e2.counts(tmax);
        group(vec![
            polyline(&series(0)).stroke(sir_color(Sir::S), 3.0),
            polyline(&series(1)).stroke(sir_color(Sir::I), 3.0),
            polyline(&series(2)).stroke(sir_color(Sir::R), 3.0),
            text_sized(&format!("S {a}   I {b}   R {c}"), 0.26, WHITE).move_to(v2(3.7, 2.85)),
        ])
    });
    let times = Arc::new(decay_times(192, 2.0, 5));
    s.always_redraw(move |ctx| {
        let t = ctx.elapsed;
        let left = times.iter().filter(|&&d| d > t).count();
        group(vec![
            decay_grid(&times, 24, 0.21, t, rgb8(120, 255, 150), rgb8(60, 60, 75))
                .move_to(v2(3.7, -1.65)),
            text_sized(
                &format!("t = {t:.1} s   N = {left}/192   (½ every 2 s)"),
                0.26,
                WHITE,
            )
            .move_to(v2(3.7, -2.75)),
        ])
    });
    s.add(caption(
        "each nucleus decays at a random, memoryless time",
        v2(3.7, -3.35),
    ));
    s.wait(10.0);
    run(&s, "stem_chembio")
}
