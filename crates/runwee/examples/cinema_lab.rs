//! Phase A4 — cinematic camera & 3D: parallax layers with depth-of-field on a
//! Catmull–Rom dolly, a lag-smoothed follow cam with screen shake, a synthwave
//! terrain fly-over, a hyperspace warp and a raymarched SDF scene.
use runwee::prelude::*;

fn ridge(y0: f64, amp: f64, freq: f64, seed: u64, color: Color) -> Mobject {
    let mut pts = vec![v2(-16.0, -9.0)];
    for i in 0..=320 {
        let x = -16.0 + 32.0 * i as f64 / 320.0;
        let h = ridged2(v2(x * freq, seed as f64 * 3.1), 5, seed);
        pts.push(v2(x, y0 + amp * h));
    }
    pts.push(v2(16.0, -9.0));
    polygon(&pts).fill(color, 1.0).no_stroke()
}

fn pine(x: f64, y: f64, h: f64, c: Color) -> Mobject {
    group(vec![
        polygon(&[
            v2(x - h * 0.28, y + h * 0.2),
            v2(x + h * 0.28, y + h * 0.2),
            v2(x, y + h),
        ])
        .fill(c, 1.0)
        .no_stroke(),
        polygon(&[v2(x - h * 0.34, y), v2(x + h * 0.34, y), v2(x, y + h * 0.7)])
            .fill(c, 1.0)
            .no_stroke(),
        rectangle(h * 0.06, h * 0.25)
            .fill(c, 1.0)
            .no_stroke()
            .move_to(v2(x, y - h * 0.08)),
    ])
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(8, 8, 16));

    // ── Act 1: parallax valley with depth of field on a spline dolly ────
    let t0 = s.time();
    let sky = field_image(Rect::from_center(Vec2::ZERO, 40.0, 22.0), 64, |p| {
        gradient_at(
            &[
                rgb8(250, 190, 140),
                rgb8(230, 120, 140),
                rgb8(70, 60, 130),
                rgb8(20, 20, 50),
            ],
            (p.y + 2.0) / 9.0,
        )
    });
    let mut layers = vec![parallax_layer(&mut s, sky, 40.0, None)];
    let sun = circle(1.1)
        .fill(rgb8(255, 235, 200), 1.0)
        .no_stroke()
        .move_to(v2(2.5, 2.3))
        .glow(0.6, Some(rgb8(255, 200, 150)));
    layers.push(parallax_layer(&mut s, sun, 20.0, None));
    let specs = [
        (8.0, -0.6, 3.2, 0.22, 1, rgb8(165, 120, 170)),
        (4.5, -1.6, 2.8, 0.3, 2, rgb8(110, 75, 135)),
        (2.5, -2.6, 2.2, 0.42, 3, rgb8(65, 42, 95)),
    ];
    for (depth, y0, amp, f, seed, c) in specs {
        layers.push(parallax_layer(
            &mut s,
            ridge(y0, amp, f, seed, c),
            depth,
            Some((1.0, 0.12)),
        ));
    }
    let mut forest = vec![ridge(-3.0, 0.6, 0.6, 9, rgb8(30, 18, 45))];
    for i in 0..30 {
        let x = -15.0 + i as f64 * 1.03 + 0.4 * (i as f64 * 2.7).sin();
        forest.push(pine(
            x,
            -3.0 + 0.2 * (i as f64 * 1.3).cos(),
            1.3 + 0.6 * (i as f64 * 0.77).sin().abs(),
            rgb8(30, 18, 45),
        ));
    }
    layers.push(parallax_layer(
        &mut s,
        group(forest),
        1.0,
        Some((1.0, 0.12)),
    ));
    let mut grass = vec![];
    for i in 0..70 {
        let x = -14.0 + i as f64 * 0.4;
        let h = 0.7 + 1.0 * hash_unit(hash2(i, 1, 4));
        grass.push(
            polygon(&[
                v2(x - 0.14, -4.3),
                v2(x + 0.14, -4.3),
                v2(x + 0.3 * (i as f64).sin(), -4.3 + h),
            ])
            .fill(rgb8(12, 8, 18), 1.0)
            .no_stroke(),
        );
    }
    layers.push(parallax_layer(
        &mut s,
        group(grass),
        0.55,
        Some((1.0, 0.12)),
    ));
    let title = s.add(bold_text("Parallax + depth of field", 0.4, WHITE).to_edge(UP, 0.25));
    s.set_fixed(title, true);
    s.camera_spline(vec![
        cam_key(t0, v2(-3.0, 0.0), 8.0, 0.0),
        cam_key(t0 + 2.5, v2(1.0, 0.4), 6.5, 0.02),
        cam_key(t0 + 5.0, v2(4.0, -0.2), 7.5, -0.01),
    ]);
    s.wait(5.0);
    s.remove(title);
    for l in layers {
        s.remove(l);
    }

    // ── Act 2: follow cam + screen shake over a big world ───────────────
    let t1 = s.time();
    let mut world = vec![];
    for i in -20..=20 {
        let k = i as f64;
        world.push(line(v2(k, -12.0), v2(k, 12.0)).stroke(GRAY_E, 1.0));
        world.push(line(v2(-20.0, k * 0.6), v2(20.0, k * 0.6)).stroke(GRAY_E, 1.0));
    }
    for i in 0..18 {
        let p = v2(-12.0 + i as f64 * 1.5, 3.0 * (i as f64 * 0.9).sin());
        world.push(
            regular_polygon(5, 0.35)
                .rotate(i as f64)
                .fill(palette("sunset", i as f64 / 18.0), 0.7)
                .no_stroke()
                .move_to(p),
        );
    }
    let w = s.add(group(world));
    let hero = s.always_redraw(move |ctx| {
        let u = (ctx.t - t1).max(0.0);
        let p = v2(-10.0 + 4.6 * u, 2.6 * (u * 1.7).sin());
        let trail: Vec<Vec2> = (0..30)
            .map(|k| {
                let uu = (u - k as f64 * 0.03).max(0.0);
                v2(-10.0 + 4.6 * uu, 2.6 * (uu * 1.7).sin())
            })
            .collect();
        group(vec![
            polyline(&trail).stroke(rgb8(120, 220, 255), 5.0),
            circle(0.28)
                .fill(rgb8(120, 220, 255), 1.0)
                .no_stroke()
                .move_to(p)
                .glow(0.5, Some(rgb8(120, 220, 255))),
        ])
    });
    let label =
        s.add(bold_text("Follow cam (lag 0.35 s) + trauma shake", 0.4, WHITE).to_edge(UP, 0.25));
    s.set_fixed(label, true);
    s.camera_follow(hero, t1 + 4.2, 0.35, 0.6, Some(6.0));
    s.wait(2.0);
    s.camera_shake(0.9, 1.0, 7);
    let boom = s.add(
        Text::new("BOOM")
            .size(1.0)
            .color(rgb8(255, 120, 80))
            .build()
            .move_to(v2(-10.0 + 4.6 * 2.0, 2.6 * (3.4f64).sin() + 1.0)),
    );
    s.wait(2.2);
    for o in [w, hero, label, boom] {
        s.remove(o);
    }

    // ── Act 3: synthwave terrain fly-over + warp ────────────────────────
    let t2 = s.time();
    let cam3 = Camera3D {
        phi: 78.0 * DEGREES,
        theta: -90.0 * DEGREES,
        distance: 7.0,
        zoom: 1.0,
        focus: v3(0.0, 5.0, 1.2),
        ..Default::default()
    };
    let syn = s.always_redraw(move |ctx| {
        let t = ctx.t - t2;
        group(vec![
            rectangle(16.0, 4.5)
                .fill_paint(
                    Paint::linear(
                        v2(0.0, 2.25),
                        v2(0.0, -2.25),
                        &[rgb8(20, 10, 40), rgb8(90, 20, 90)],
                    ),
                    1.0,
                )
                .no_stroke()
                .move_to(v2(0.0, 1.75)),
            retro_sun(v2(0.0, 1.0), 2.0, rgb8(255, 230, 90), rgb8(255, 50, 140), t),
            rectangle(16.0, 4.6)
                .fill(rgb8(10, 5, 25), 1.0)
                .no_stroke()
                .move_to(v2(0.0, -2.3)),
            terrain_grid(
                t,
                &cam3,
                34,
                28,
                (22.0, 24.0),
                3.0,
                2.4,
                rgb8(255, 70, 200),
                5,
            ),
        ])
    });
    let lbl = s.add(bold_text("Synthwave terrain (perspective 3D)", 0.4, WHITE).to_edge(UP, 0.25));
    s.wait(4.0);
    s.remove(syn);
    s.remove(lbl);
    let t3 = s.time();
    let warp = s.always_redraw(move |ctx| {
        let u = ctx.t - t3;
        let speed = 2.0 + 30.0 * smoothstep(0.0, 2.2, u);
        warp_starfield(
            u * (2.0 + 10.0 * smoothstep(0.0, 2.2, u)),
            500,
            speed,
            Vec2::ZERO,
            1.6,
            rgb8(120, 160, 255),
            3,
        )
    });
    let lbl = s.add(bold_text("Hyperspace", 0.4, WHITE).to_edge(UP, 0.25));
    s.wait(3.0);
    s.remove(warp);
    s.remove(lbl);

    // ── Act 4: raymarched SDF scene ─────────────────────────────────────
    let t4 = s.time();
    let rm = s.always_redraw(move |ctx| {
        let u = ctx.t - t4;
        let bob = 0.25 * (u * 2.0).sin();
        let blobs = sd_sphere(0.8)
            .at(v3(-0.55, 1.0 + bob, 0.0))
            .smooth_union(sd_sphere(0.65).at(v3(0.6, 1.0 - bob, 0.2)), 0.5)
            .color(rgb8(255, 120, 90));
        let ring = sd_torus(0.9, 0.22)
            .rotate(u * 0.8, 1.2)
            .twist(1.0)
            .at(v3(2.6, 1.2, -0.6))
            .color(rgb8(90, 200, 255));
        let lattice = sd_sphere(1.0)
            .gyroid(5.0, 0.06)
            .at(v3(-2.7, 1.0, -0.4))
            .color(rgb8(250, 220, 120));
        let cube = sd_box(v3(0.45, 0.45, 0.45), 0.08)
            .subtract(sd_sphere(0.58))
            .rotate(u, 0.6)
            .at(v3(0.3, 0.45, 1.9))
            .color(rgb8(170, 255, 160));
        let floor = sd_plane(0.0).checker(rgb8(200, 200, 215), 0.8);
        let scene = blobs.union(ring).union(lattice).union(cube).union(floor);
        let cam = RayCam::orbit(v3(0.0, 0.9, 0.0), 7.8, 2.9, -1.2 + u * 0.35, 45.0);
        let opts = MarchOpts {
            fog: 0.015,
            ..Default::default()
        };
        raymarch_image(
            &scene,
            &cam,
            &opts,
            Rect::from_center(Vec2::ZERO, 8.0 * 16.0 / 9.0, 8.0),
            400,
        )
    });
    let lbl = s.add(
        bold_text(
            "Raymarched SDFs: smooth union, twist, gyroid, CSG",
            0.36,
            WHITE,
        )
        .to_edge(UP, 0.25),
    );
    s.wait(5.0);
    s.remove(rm);
    s.remove(lbl);
    run(&s, "cinema_lab")
}
