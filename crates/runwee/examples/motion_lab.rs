//! Phase A1 — motion core: keyframe tracks with smooth (auto-bezier) paths, physical
//! springs in three damping regimes, a spring follower chasing a jumping target,
//! GSAP-style grid staggers, AE `wiggle`, and a timeline with position parameters.
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(12, 14, 24));

    let header = s.add(bold_text("Motion core", 0.5, WHITE).to_edge(UP, 0.35));
    s.set_fixed(header, true);

    // --- 1. grid stagger from the centre with spring pop-in --------------------------
    let mut cells = vec![];
    for i in 0..25 {
        let (c, r) = (i % 5, i / 5);
        let col = palette("neon", i as f64 / 25.0);
        let m = rounded_rectangle(0.42, 0.42, 0.08)
            .fill(col, 1.0)
            .no_stroke()
            .move_to(v2(-5.4 + c as f64 * 0.52, 1.4 - r as f64 * 0.52));
        cells.push(s.register(m));
    }
    let st = Stagger::each(0.06).grid(5, 5).from(StaggerFrom::Center);
    let pops = stagger_anims(&cells, &st, |o| spring_pop_in(o, Spring::bouncy()));
    let lbl1 =
        s.add(text_sized("stagger · grid from centre", 0.22, GRAY_B).move_to(v2(-4.36, -1.45)));
    s.play(pops);

    // --- 2. spring regimes ----------------------------------------------------------
    let t_spring = s.time();
    let springs = [
        ("bouncy ζ=0.35", Spring::bouncy(), PINK),
        ("snappy ζ=0.85", Spring::snappy(), TEAL),
        ("overdamped", Spring::new(1.0, 120.0, 45.0), YELLOW),
    ];
    for (k, (name, sp, c)) in springs.iter().enumerate() {
        let (sp, c) = (*sp, *c);
        let y = 1.6 - k as f64 * 0.75;
        s.add(text_sized(name, 0.2, GRAY_B).move_to(v2(-0.8, y + 0.28)));
        s.add(line(v2(-1.9, y), v2(1.6, y)).stroke(GRAY_D, 2.0));
        s.add(
            line(v2(1.2, y - 0.2), v2(1.2, y + 0.2))
                .stroke(GRAY_B, 2.0)
                .dashed(0.05, 0.04),
        );
        s.always_redraw(move |ctx| {
            let t = (ctx.t - t_spring).max(0.0) % 3.0;
            let x = sp.value(t, -1.8, 1.2, 0.0);
            dot_r(v2(x, y), 0.14, c).glow(0.15, None)
        });
    }

    // --- 3. smooth keyframe path + spring follower ------------------------------------
    let t_path = s.time();
    let pts = [
        v2(2.6, 1.6),
        v2(4.0, 2.2),
        v2(5.6, 1.2),
        v2(4.6, -0.2),
        v2(3.0, 0.4),
        v2(2.6, 1.6),
    ];
    let mut track = Keyframes::new(pts[0]);
    for (i, p) in pts.iter().enumerate().skip(1) {
        track = track.smooth_key(i as f64 * 0.9, *p);
    }
    let track = track.looped(KeyLoop::Repeat);
    for p in &pts[..5] {
        s.add(dot_r(*p, 0.05, GRAY_B));
    }
    let tr1 = track.clone();
    s.always_redraw(move |ctx| {
        let t = (ctx.t - t_path).max(0.0);
        let n = 60;
        let pts: Vec<Vec2> = (0..n)
            .map(|i| tr1.at((t - 0.9 + 0.9 * i as f64 / n as f64).max(0.0)))
            .collect();
        group(pts.windows(2).enumerate().map(|(i, w)| {
            let k = i as f32 / n as f32;
            line(w[0], w[1]).stroke(BLUE_B.with_alpha(k), 1.0 + 5.0 * k)
        }))
    });
    let tr2 = track.clone();
    s.always_redraw(move |ctx| {
        let t = (ctx.t - t_path).max(0.0);
        dot_r(tr2.at(t), 0.13, BLUE_B).glow(0.2, None)
    });
    // a target that jumps between corners (hold keys) and a wobbly follower
    let jumps = Keyframes::new(v2(-4.8, -2.4))
        .hold_key(1.0, v2(-2.2, -2.4))
        .hold_key(2.0, v2(-2.2, -3.4))
        .hold_key(3.0, v2(-4.8, -3.4))
        .hold_key(4.0, v2(-4.8, -2.4))
        .looped(KeyLoop::Repeat);
    let j1 = jumps.clone();
    s.always_redraw(move |ctx| {
        let t = (ctx.t - t_path).max(0.0);
        square(0.34).stroke(ORANGE, 3.0).move_to(j1.at(t))
    });
    let j2 = jumps.clone();
    s.always_redraw(move |ctx| {
        let t = (ctx.t - t_path).max(0.0);
        let p = spring_follow2(&|u| j2.at(u), Spring::from_response(0.9, 0.3), 0.0, t);
        dot_r(p, 0.12, ORANGE).glow(0.18, None)
    });
    s.add(text_sized("spring follower", 0.22, GRAY_B).move_to(v2(-3.5, -1.95)));
    s.add(text_sized("smooth keyframe path", 0.22, GRAY_B).move_to(v2(4.1, -0.75)));

    // --- 4. wiggle + timeline -------------------------------------------------------
    let word = s.register(bold_text("wiggle()", 0.42, YELLOW).move_to(v2(1.2, -2.9)));
    let a = s.register(text_sized("timeline", 0.32, TEAL).move_to(v2(4.3, -2.3)));
    let b = s.register(text_sized("positions", 0.32, PINK).move_to(v2(4.3, -2.8)));
    let c = s.register(text_sized("& labels", 0.32, YELLOW).move_to(v2(4.3, -3.3)));
    let tl = Timeline::new()
        .then(fade_in_shift(word, UP * 0.3).run_time(0.5))
        .then(fade_in_shift(a, RIGHT * 0.5).run_time(0.6))
        .offset(fade_in_shift(b, RIGHT * 0.5).run_time(0.6), -0.3)
        .label("third")
        .at_label(fade_in_shift(c, RIGHT * 0.5).run_time(0.6), "third", -0.2)
        .with(wiggle_anim(word, 4.0, 0.08, 4.0, 7));
    tl.play(&mut s);
    s.play(keyframe_anim(
        lbl1,
        KeyTransform::new().scale(Keyframes::new(1.0_f64).key(0.3, 1.25, Rate::OutBack).key(
            0.8,
            1.0,
            Rate::InOutSine,
        )),
    ));
    s.wait(1.5);
    run(&s, "motion_lab")
}
