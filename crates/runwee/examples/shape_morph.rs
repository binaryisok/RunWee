//! Phase A2 — shape & morph: smart (optimal-correspondence) morphs, twist morphs,
//! echo + motion trails, neon write-on, jelly wobble, Voronoi shatter / assemble,
//! dust dissolve and rough.js-style hand-drawn rendering with line boil.
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(10, 12, 22));
    let title = s.add(bold_text("Shape & morph", 0.45, WHITE).to_edge(UP, 0.3));
    s.set_fixed(title, true);

    // ---- 1. smart morph chain with an echo trail --------------------------------
    let blob = s.add(
        circle(1.1)
            .fill(BLUE, 0.9)
            .stroke(BLUE_A, 4.0)
            .move_to(v2(-3.2, 0.0)),
    );
    echo_trail(&mut s, blob, 4, 0.08, 0.25);
    let cap = s.add(text_sized("smart_morph", 0.26, GRAY_B).move_to(v2(-3.2, -2.3)));
    s.wait(0.3);
    s.play(vec![smart_morph(
        blob,
        star(5, 1.3, 0.55)
            .fill(YELLOW, 0.9)
            .stroke(YELLOW_A, 4.0)
            .move_to(v2(-3.2, 0.0)),
    )]);
    s.play(vec![twist_morph(
        blob,
        rounded_rectangle(2.0, 2.0, 0.3)
            .fill(PINK, 0.9)
            .stroke(PINK, 4.0)
            .move_to(v2(0.0, 0.2)),
        2.5,
    )]);
    s.play(vec![morph_with(
        blob,
        bold_text("RunWee", 1.1, TEAL).move_to(v2(3.0, 0.0)),
        MorphOpts {
            arc: 0.25,
            ..Default::default()
        },
    )]);

    // a dot on a lissajous path with a tapered motion trail
    let probe = s.add(dot_r(v2(0.0, -2.0), 0.1, ORANGE).glow(0.2, None));
    motion_trail(&mut s, probe, 0.6, 10.0, ORANGE);
    let t0 = s.time();
    s.add_updater(probe, move |m, ctx| {
        let u = (ctx.t - t0).max(0.0) * 1.6;
        m.move_to(v2(4.5 * (u * 0.9).sin(), -2.2 + 0.5 * (u * 1.8).cos()))
    });
    s.play(vec![jelly(blob, 0.25, 3.0).run_time(1.6)]);
    s.wait(0.6);
    s.clear_updaters(probe);
    s.play(vec![fade_out(blob), fade_out(probe), fade_out(cap)]);

    // ---- 2. neon write + shatter / assemble / dissolve ------------------------------
    let neon = s.register(
        bold_text("NEON", 1.2, PINK.with_alpha(1.0))
            .fill(rgb8(255, 70, 160), 0.85)
            .move_to(v2(-3.2, 1.0)),
    );
    let glass = s.register(bold_text("SHATTER", 1.0, TEAL).move_to(v2(2.6, 1.0)));
    s.play(vec![neon_write(neon, rgb8(255, 70, 160)), fade_in(glass)]);
    s.wait(0.3);
    s.play(vec![shatter(
        glass,
        ShatterOpts {
            pieces: 60,
            seed: 4,
            ..Default::default()
        },
    )]);
    let glass2 = s.register(bold_text("ASSEMBLE", 0.9, YELLOW).move_to(v2(2.6, -1.2)));
    s.play(vec![
        assemble(
            glass2,
            ShatterOpts {
                pieces: 50,
                seed: 9,
                force: 2.0,
                ..Default::default()
            },
        ),
        dissolve(neon, 900, 3),
    ]);
    s.wait(0.4);
    s.play(vec![dissolve(glass2, 900, 5)]);

    // ---- 3. hand-drawn panel with line boil -------------------------------------
    let shapes = vec![
        (
            circle(0.9)
                .fill(RED, 0.8)
                .stroke(WHITE, 3.0)
                .move_to(v2(-4.5, 0.2)),
            SketchFill::Hachure,
        ),
        (
            star(5, 1.0, 0.45)
                .fill(YELLOW, 0.8)
                .stroke(WHITE, 3.0)
                .move_to(v2(-1.5, 0.2)),
            SketchFill::CrossHatch,
        ),
        (
            rounded_rectangle(1.8, 1.4, 0.2)
                .fill(TEAL, 0.8)
                .stroke(WHITE, 3.0)
                .move_to(v2(1.5, 0.2)),
            SketchFill::ZigZag,
        ),
        (
            bold_text("hi!", 1.2, BLUE).move_to(v2(4.4, 0.2)),
            SketchFill::Hachure,
        ),
    ];
    let t_sk = s.time();
    let mut ids = vec![];
    for (k, (m, fill)) in shapes.into_iter().enumerate() {
        let opts = SketchOpts::default()
            .fill(fill)
            .seed(k as u64 + 1)
            .gap(if k == 3 { 0.05 } else { 0.1 });
        ids.push(s.always_redraw(move |ctx| {
            let reveal = ((ctx.t - t_sk) * 1.5 - k as f64 * 0.3).clamp(0.0, 1.0);
            boil(&m, ctx.t, 8.0, opts).partial(0.0, reveal)
        }));
    }
    let lbl = s.add(
        text_sized(
            "sketchy() · hachure · cross-hatch · zig-zag · boil",
            0.26,
            GRAY_B,
        )
        .move_to(v2(0.0, -1.8)),
    );
    s.wait(3.5);
    let _ = (ids, lbl);
    run(&s, "shape_morph")
}
