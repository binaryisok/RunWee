//! Phase S1 — math visuals: Fourier epicycles drawing π, a linear map with its
//! eigenvectors and determinant, Taylor partial sums of sin, unit-circle
//! trigonometry, then a gallery of a slope field, domain colouring, the Ulam spiral,
//! a modular times table, Pascal's triangle mod 3 and a Collatz coral.
use runwee::prelude::mathviz::*;
use runwee::prelude::*;
use std::sync::Arc;

fn caption(s: &str, at: Vec2) -> Mobject {
    text_sized(s, 0.26, GRAY_A).move_to(at)
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(10, 12, 22));

    // ── Act 1: epicycles + linear map (0 – 9 s) ──
    s.add(bold_text("Fourier epicycles & linear maps", 0.42, WHITE).to_edge(UP, 0.25));
    let shape = tex("\\pi").set_height(4.4).move_to(Vec2::ZERO);
    let pts = outline_points(&shape, 512);
    let cs = Arc::new(fourier_series(&pts, 90));
    let c1 = v2(-3.6, -0.35);
    let cs1 = cs.clone();
    s.always_redraw(move |ctx| {
        let t = (ctx.elapsed / 7.0).min(1.0);
        group(vec![
            epicycle_trace(&cs1, t, c1, rgb8(255, 210, 90), 4.0),
            epicycles(&cs1, t, c1, BLUE_B),
        ])
    });
    s.add(caption(
        "90 rotating circles = DFT of the outline",
        v2(-3.6, -3.55),
    ));

    let m = [[1.0, 1.0], [0.5, 1.5]];
    let c2 = v2(3.6, -0.35);
    s.always_redraw(move |ctx| {
        let t = smoothstep(1.0, 5.0, ctx.elapsed);
        let det = mat_det(mat_lerp(m, t));
        group(vec![
            linear_map_view(m, t, 3.0, t > 0.98).scale(0.95).move_to(c2),
            text_sized(&format!("det = {det:.2}"), 0.3, YELLOW).move_to(c2 + v2(-1.9, 2.5)),
        ])
    });
    s.add(caption(
        "M = [[1, 1], [0.5, 1.5]]  ·  eigenvectors only stretch",
        v2(3.6, -3.55),
    ));
    s.wait(9.0);
    s.remove_all();

    // ── Act 2: Taylor series + unit circle (9 – 18 s) ──
    s.add(bold_text("Taylor series & the unit circle", 0.42, WHITE).to_edge(UP, 0.25));
    let ax = Arc::new(
        Axes::new((-7.0, 7.0, 2.0), (-3.0, 3.0, 1.0))
            .size(6.6, 4.6)
            .at(v2(-3.4, -0.4)),
    );
    s.add(ax.build());
    s.add(ax.plot(|x| x.sin()).stroke(WHITE, 3.0));
    let ax2 = ax.clone();
    s.always_redraw(move |ctx| {
        let n = 1 + 2 * ((ctx.elapsed / 0.9) as usize).min(8);
        let c = color_gradient(&[RED, ORANGE, YELLOW, GREEN, TEAL, BLUE], n as f64 / 17.0);
        group(vec![
            ax2.plot(move |x| taylor_poly("sin", n, x)).stroke(c, 4.0),
            text_sized(&format!("degree {n} Maclaurin polynomial of sin x"), 0.3, c)
                .move_to(v2(-3.4, -3.3)),
        ])
    });
    s.always_redraw(move |ctx| {
        let theta = (ctx.elapsed * TAU / 4.5).rem_euclid(TAU);
        unit_circle_trig(theta, v2(1.6, -0.4), 1.25, 3.2, 0.55)
    });
    s.add(caption(
        "cos θ (blue), sin θ (red) and the traced sine wave",
        v2(4.0, -3.3),
    ));
    s.wait(9.0);
    s.remove_all();

    // ── Act 3: gallery (18 – 26 s) ──
    s.add(bold_text("Math gallery", 0.42, WHITE).to_edge(UP, 0.22));
    const W: f64 = 4.3;
    const H: f64 = 3.0;
    let centers = [
        v2(-4.6, 1.5),
        v2(0.0, 1.5),
        v2(4.6, 1.5),
        v2(-4.6, -2.1),
        v2(0.0, -2.1),
        v2(4.6, -2.1),
    ];
    let names = [
        "slope field  y' = x − y",
        "domain colouring  (z³ − 1)/(z − a)",
        "Ulam spiral (primes)",
        "times table mod 200",
        "Pascal's triangle mod 3",
        "Collatz coral (1 – 1500)",
    ];
    for (c, n) in centers.iter().zip(names) {
        s.add(
            rounded_rectangle(W, H, 0.12)
                .move_to(*c)
                .stroke(GRAY_C, 1.5),
        );
        s.add(caption(n, *c + v2(0.0, -H / 2.0 - 0.2)));
    }

    // slope field with solution curves drawn on
    let sax = Arc::new(
        Axes::new((-3.0, 3.0, 1.0), (-2.0, 2.0, 1.0))
            .size(3.9, 2.6)
            .at(centers[0]),
    );
    let f = |x: f64, y: f64| x - y;
    s.add(sax.build());
    s.add(slope_field(&sax, &f, 0.4, BLUE_D));
    let curves: Arc<Vec<Vec<Vec2>>> = Arc::new(
        [-1.8, -1.0, 0.0, 1.0, 1.8]
            .iter()
            .map(|&y0| ode_curve(&sax, &f, -3.0, y0))
            .collect(),
    );
    s.always_redraw(move |ctx| {
        let p = (ctx.elapsed / 3.0).min(1.0);
        group(
            curves
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let k = ((c.len() as f64 * p) as usize).max(2).min(c.len());
                    polyline(&c[..k])
                        .stroke(color_gradient(&[YELLOW, ORANGE, PINK], i as f64 / 4.0), 3.0)
                })
                .collect::<Vec<_>>(),
        )
    });

    // domain colouring with a wandering pole
    let r2 = Rect::from_center(centers[1], W - 0.1, H - 0.1);
    s.always_redraw(move |ctx| {
        let a = Cx::from_polar(0.9, ctx.elapsed * 0.8);
        let o = r2.center();
        domain_coloring(r2, 170, move |z| {
            let z = (z - Cx::from(o)) * 1.1;
            (z.powi(3) - cx(1.0, 0.0)) / (z - a)
        })
    });

    // Ulam spiral growing
    let c3 = centers[2];
    s.always_redraw(move |ctx| {
        let n = ((ctx.elapsed * 2500.0) as usize).clamp(2, 10201);
        // 1 sits at the panel centre while the spiral grows outwards
        ulam_spiral(
            n,
            2.75 / 101.0,
            rgb8(120, 220, 255),
            Color::rgba(0.0, 0.0, 0.0, 0.0),
        )
        .shift(c3)
    });

    // times table with continuously varying k
    let c4 = centers[3];
    s.always_redraw(move |ctx| {
        let k = 2.0 + ctx.elapsed * 0.45;
        group(vec![
            times_table(200, k, 1.3, &[PINK, PURPLE_A, BLUE, TEAL]).move_to(c4),
            text_sized(&format!("k = {k:.2}"), 0.24, WHITE).move_to(c4 + v2(1.65, 1.2)),
        ])
    });

    // Pascal mod 3
    s.add(
        pascal_mod(30, 3, 0.095, &[rgb8(255, 120, 90), rgb8(255, 220, 120)])
            .fit_within(W - 0.4, H - 0.3)
            .move_to(centers[4]),
    );

    // Collatz coral
    let starts: Vec<u64> = (1..=1500).collect();
    s.add(
        collatz_tree(&starts, 0.05, -0.11, 0.21, &[TEAL, GREEN_B, YELLOW, ORANGE])
            .fit_within(W - 0.3, H - 0.25)
            .move_to(centers[5]),
    );
    s.wait(8.0);
    run(&s, "stem_math")
}
