//! Phase S2 — physics labs: a ripple tank with two-source interference, Doppler
//! wavefronts (sub- and supersonic), a 2D optical bench (thin lens, Porro prism with
//! total internal reflection, dispersive prism, ball-lens caustic), then a gallery
//! of Kepler's equal areas, a gravity well, a damped-pendulum phase portrait, a
//! chaos fan of double pendulums, Maxwell–Boltzmann relaxation and E×B drift.
use runwee::prelude::physviz::*;
use runwee::prelude::*;
use std::sync::Arc;

/// Optical elements plus the traced rays (points and colour) of one panel.
type Bench = (Vec<Optic>, Vec<(Vec<Vec2>, Color)>);

fn caption(s: &str, at: Vec2) -> Mobject {
    text_sized(s, 0.26, GRAY_A).move_to(at)
}

fn frame(c: Vec2, w: f64, h: f64) -> Mobject {
    rounded_rectangle(w, h, 0.12).move_to(c).stroke(GRAY_C, 1.5)
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(9, 11, 20));

    // ── Act 1: waves (0 – 9 s) ──
    s.add(bold_text("Waves: interference & Doppler", 0.42, WHITE).to_edge(UP, 0.25));
    let tank = Rect::from_center(v2(-3.5, -0.35), 6.3, 5.6);
    s.always_redraw(move |ctx| {
        let sep = 0.55 + 0.35 * (ctx.elapsed * 0.5).sin();
        let src = vec![
            wave_source(v2(-3.5 - sep, -0.35)),
            wave_source(v2(-3.5 + sep, -0.35)),
        ];
        ripple_tank(tank, 230, src, 0.45, ctx.elapsed * 1.2)
    });
    s.add(frame(tank.center(), 6.3, 5.6));
    s.add(caption(
        "ripple tank: nodal lines fan out as the sources separate",
        v2(-3.5, -3.45),
    ));

    for (i, (c, vs, label)) in [
        (0.8, 0.5, "subsonic v = 0.6c: fronts bunch up ahead"),
        (0.4, 0.6, "supersonic v = 1.5c: a Mach cone"),
    ]
    .into_iter()
    .enumerate()
    {
        let center = v2(3.6, 1.15 - 3.05 * i as f64);
        s.add(frame(center, 6.3, 2.7));
        s.add(caption(label, center + v2(0.0, -1.55)));
        let col = if i == 0 { BLUE_B } else { ORANGE };
        s.always_redraw(move |ctx| {
            let src = move |t: f64| center + v2(-2.6 + vs * t.min(8.8), 0.0);
            doppler_wavefronts(src, c, 0.28, ctx.elapsed, 1.2 / c, col)
        });
    }
    s.wait(9.0);
    s.remove_all();

    // ── Act 2: optical bench (9 – 18 s) ──
    s.add(bold_text("Optical bench: a 2D ray tracer", 0.42, WHITE).to_edge(UP, 0.25));
    let panels = [
        v2(-3.6, 1.45),
        v2(3.6, 1.45),
        v2(-3.6, -2.05),
        v2(3.6, -2.05),
    ];
    let names = [
        "thin lens: parallel rays meet at the focus",
        "Porro prism: total internal reflection, twice",
        "dispersion: n(λ) bends violet more than red",
        "ball lens: spherical aberration caustic",
    ];
    for (c, n) in panels.iter().zip(names) {
        s.add(frame(*c, 6.6, 3.1));
        s.add(caption(n, *c + v2(0.0, -1.75)));
    }
    let mut benches: Vec<Bench> = vec![];
    // 1. converging lens
    {
        let c = panels[0];
        let optics = vec![
            Optic::ThinLens {
                center: c + v2(-0.8, 0.0),
                axis: v2(1.0, 0.0),
                half_height: 1.3,
                focal: 1.6,
            },
            Optic::Screen {
                a: c + v2(2.9, -1.3),
                b: c + v2(2.9, 1.3),
            },
        ];
        let rays = (0..9)
            .map(|k| {
                let h = -1.1 + 2.2 * k as f64 / 8.0;
                (
                    trace_ray(c + v2(-3.1, h), v2(1.0, 0.0), 550.0, &optics, 4, 9.0),
                    color_gradient(&[YELLOW, ORANGE], k as f64 / 8.0),
                )
            })
            .collect();
        benches.push((optics, rays));
    }
    // 2. Porro prism (right-angle prism, light enters the hypotenuse face)
    {
        let c = panels[1] + v2(0.6, 0.0);
        let poly = vec![c + v2(-1.3, -0.3), c + v2(1.3, -0.3), c + v2(0.0, 1.0)];
        let poly: Vec<Vec2> = poly.into_iter().map(|p| p - v2(0.0, 0.2)).collect();
        let optics = vec![Optic::Glass {
            poly,
            n: 1.52,
            b: 0.0,
        }];
        let rays = (0..7)
            .map(|k| {
                let x = -1.15 + 0.17 * k as f64;
                (
                    trace_ray(c + v2(x, -1.35), v2(0.0, 1.0), 550.0, &optics, 6, 4.6),
                    color_gradient(&[TEAL, GREEN_B], k as f64 / 6.0),
                )
            })
            .collect();
        benches.push((optics, rays));
    }
    // 3. dispersive prism
    {
        let c = panels[2];
        let poly = vec![c + v2(-1.8, -1.0), c + v2(0.0, -1.0), c + v2(-0.9, 0.85)];
        let optics = vec![
            Optic::Glass {
                poly,
                n: 1.55,
                b: 0.06,
            },
            Optic::Screen {
                a: c + v2(2.9, -1.4),
                b: c + v2(2.9, 1.4),
            },
        ];
        let rays = (0..16)
            .map(|k| {
                let nm = 400.0 + 300.0 * k as f64 / 15.0;
                (
                    trace_ray(c + v2(-3.2, -0.75), v2(1.0, 0.3), nm, &optics, 6, 9.0),
                    wavelength_color(nm).with_alpha(0.85),
                )
            })
            .collect();
        benches.push((optics, rays));
    }
    // 4. ball lens
    {
        let c = panels[3];
        let optics = vec![Optic::Disc {
            center: c + v2(-1.2, 0.0),
            r: 1.15,
            n: 1.5,
            b: 0.0,
        }];
        let rays = (0..25)
            .map(|k| {
                let h = -1.05 + 2.1 * k as f64 / 24.0;
                (
                    trace_ray(c + v2(-3.2, h), v2(1.0, 0.0), 550.0, &optics, 4, 6.4),
                    color_gradient(&[PINK, YELLOW, PINK], k as f64 / 24.0).with_alpha(0.75),
                )
            })
            .collect();
        benches.push((optics, rays));
    }
    for (k, (_, rays)) in benches.iter_mut().enumerate() {
        let r = Rect::from_center(panels[k], 6.5, 3.0);
        for (pts, _) in rays.iter_mut() {
            *pts = clip_polyline_to_rect(pts, r);
        }
    }
    // the white beam entering the dispersive prism
    let beam = benches[2].1[0].0[..2].to_vec();
    benches.push((vec![], vec![(beam, WHITE)]));
    let benches = Arc::new(benches);
    for (optics, _) in benches.iter() {
        s.add(optics_mobject(optics));
    }
    let b2 = benches.clone();
    s.always_redraw(move |ctx| {
        let f = smoothstep(0.3, 3.5, ctx.elapsed);
        group(
            b2.iter()
                .flat_map(|(_, rays)| {
                    rays.iter()
                        .map(move |(pts, c)| ray_mobject(pts, f, *c, 2.2))
                })
                .collect::<Vec<_>>(),
        )
    });
    s.wait(9.0);
    s.remove_all();

    // ── Act 3: gallery (18 – 27 s) ──
    s.add(bold_text("Mechanics & fields", 0.42, WHITE).to_edge(UP, 0.22));
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
        "Kepler: equal areas in equal times",
        "gravity well (rubber-sheet picture)",
        "phase portrait: damped pendulum",
        "chaos: 40 pendulums, Δθ = 10⁻⁴",
        "gas → Maxwell–Boltzmann",
        "charges in E×B fields: drift",
    ];
    for (c, n) in centers.iter().zip(names) {
        s.add(frame(*c, W, H));
        s.add(caption(n, *c + v2(0.0, -H / 2.0 - 0.2)));
    }

    // Kepler
    let (a, e) = (1.6, 0.62);
    let focus = centers[0] + v2(a * e, 0.0);
    s.add(equal_area_sectors(
        a,
        e,
        focus,
        6,
        0.07,
        &[BLUE_C, TEAL, GREEN_B, YELLOW, ORANGE, PINK],
    ));
    s.add(kepler_ellipse(a, e, focus, GRAY_B));
    s.add(dot_r(focus, 0.13, YELLOW).glow(0.35, Some(ORANGE)));
    s.always_redraw(move |ctx| {
        let p = focus + kepler_position(a, e, ctx.elapsed / 4.0);
        group(vec![
            line(focus, p).stroke(WHITE.with_alpha(0.5), 1.5),
            dot_r(p, 0.08, BLUE_B),
        ])
    });

    // gravity well
    let c2 = centers[1];
    s.always_redraw(move |ctx| {
        let moon = v2(1.25, 0.0).rotate(ctx.elapsed * 0.9);
        gravity_well(
            &[(Vec2::ZERO, 1.0), (moon, 0.25)],
            2.2,
            14,
            0.55,
            0.55,
            BLUE_B,
        )
        .scale(0.85)
        .shift(c2 + v2(0.0, 0.25))
    });

    // phase portrait (θ, ω) for a damped pendulum, built in local units then scaled
    let starts: Vec<Vec2> = [-3.0, -2.0, 2.0, 3.0]
        .iter()
        .flat_map(|&w| [v2(-6.0, w), v2(6.0, -w)])
        .chain([v2(-2.5, 0.0), v2(2.5, 0.0), v2(0.0, 2.6), v2(0.0, -2.6)])
        .collect();
    let portrait = phase_portrait(
        Rect::from_center(Vec2::ZERO, 13.0, 9.0),
        &|p: Vec2| v2(p.y, -p.x.sin() - 0.25 * p.y),
        0.8,
        &starts,
        30.0,
        &[YELLOW, ORANGE, PINK, PURPLE_A],
    )
    .scale(0.31)
    .move_to(centers[2]);
    s.add(portrait);

    // chaos fan
    let c4 = centers[3] + v2(0.0, 0.05);
    let fan: Arc<Vec<sim::Trajectory>> = Arc::new(
        (0..40)
            .map(|i| {
                sim::double_pendulum(0.65, 0.65, 1.0, 1.0, 2.6 + i as f64 * 1e-4, 2.4, 9.81, 10.0)
            })
            .collect(),
    );
    s.always_redraw(move |ctx| {
        let t = ctx.elapsed;
        group(
            fan.iter()
                .enumerate()
                .map(|(i, tr)| {
                    let c = color_gradient(
                        &[RED, ORANGE, YELLOW, GREEN, TEAL, BLUE, PURPLE_A],
                        i as f64 / 39.0,
                    );
                    let trail: Vec<Vec2> = (0..40)
                        .map(|k| {
                            let tt = (t - k as f64 * 0.02).max(0.0);
                            double_pendulum_tip(c4, 0.65, 0.65, tr.get(tt, 0), tr.get(tt, 2))
                        })
                        .collect();
                    group(vec![
                        polyline(&trail).stroke(c.with_alpha(0.45), 1.2),
                        double_pendulum_arms(c4, 0.65, 0.65, tr.get(t, 0), tr.get(t, 2), c),
                    ])
                })
                .collect::<Vec<_>>(),
        )
    });

    // gas + histogram
    let c5 = centers[4];
    let gbox = Rect::from_center(c5 + v2(-1.15, 0.0), 1.8, 2.6);
    let gas = Arc::new(sim::Gas::simulate_with(260, gbox, 0.035, 1.0, 9.5, 7, true));
    let bins = 12;
    let vmax = 3.0;
    let hist_origin = c5 + v2(0.2, -1.2);
    let (hw, hh) = (1.85, 2.3);
    s.add(
        rectangle(1.8, 2.6)
            .move_to(gbox.center())
            .stroke(GRAY_B, 1.5),
    );
    s.add(line(hist_origin, hist_origin + v2(hw, 0.0)).stroke(GRAY_B, 1.5));
    let curve: Vec<Vec2> = (0..=60)
        .map(|k| {
            let v = vmax * k as f64 / 60.0;
            hist_origin + v2(hw * v / vmax, maxwell_boltzmann_2d(v, 0.5) * hh / 1.4)
        })
        .collect();
    s.add(polyline(&curve).stroke(YELLOW, 2.5));
    s.always_redraw(move |ctx| {
        let t = ctx.elapsed;
        let speeds = gas.speeds(t);
        let dens = histogram_density(&speeds, bins, vmax);
        let bw = hw / bins as f64;
        let mut kids: Vec<Mobject> = gas
            .positions(t)
            .iter()
            .zip(&speeds)
            .map(|(p, v)| {
                dot_r(
                    *p,
                    0.035,
                    color_gradient(&[BLUE, TEAL, YELLOW, RED], v / 2.2),
                )
            })
            .collect();
        for (i, d) in dens.iter().enumerate() {
            let h = (d * hh / 1.4).min(hh);
            if h > 0.0 {
                kids.push(
                    rectangle(bw * 0.86, h)
                        .fill(TEAL, 0.7)
                        .no_stroke()
                        .move_to(hist_origin + v2(bw * (i as f64 + 0.5), h / 2.0)),
                );
            }
        }
        group(kids)
    });

    // E×B drift
    let c6 = centers[5];
    let paths: Arc<Vec<(Vec<Vec2>, Color)>> = Arc::new(
        [
            (1.0, BLUE_B, 0.6),
            (2.0, TEAL, 0.0),
            (-1.5, ORANGE, -0.6),
            (4.0, PINK, 1.0),
        ]
        .iter()
        .map(|&(qm, c, y)| {
            let pts = lorentz_path(
                qm,
                v2(1.7, y - 0.15),
                v2(0.0, 0.0),
                &|_| v2(0.0, -0.45),
                &|_| 1.6,
                0.002,
                4500,
            )
            .into_iter()
            .map(|p| p + c6)
            .collect();
            (pts, c)
        })
        .collect(),
    );
    s.add(text_sized("B ⊙ out of page, E ↓", 0.22, GRAY_B).move_to(c6 + v2(-0.95, 1.25)));
    s.always_redraw(move |ctx| {
        let f = (ctx.elapsed / 7.0).min(1.0);
        group(
            paths
                .iter()
                .map(|(pts, c)| {
                    let k = ((pts.len() as f64 * f) as usize).clamp(2, pts.len());
                    group(vec![
                        polyline(&pts[..k]).stroke(*c, 2.2),
                        dot_r(pts[k - 1], 0.06, *c),
                    ])
                })
                .collect::<Vec<_>>(),
        )
    });
    s.wait(9.0);
    run(&s, "stem_physics")
}
