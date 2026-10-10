//! Tour of the core animation set (manim-style).
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();

    let c = s.register(
        circle(1.2)
            .stroke(BLUE, 5.0)
            .fill(BLUE_E, 0.6)
            .shift(LEFT * 4.0),
    );
    let sq = s.register(square(2.0).stroke(YELLOW, 5.0).fill(YELLOW_E, 0.4));
    let tri = s.register(
        triangle(1.3)
            .stroke(GREEN, 5.0)
            .fill(GREEN_E, 0.5)
            .shift(RIGHT * 4.0),
    );
    s.play(lagged_start(
        vec![create(c), draw_border_then_fill(sq), grow_from_center(tri)],
        0.3,
    ));
    s.wait(0.3);

    // .animate() builder with rigid motion + style
    s.play([
        c.animate().shift(UP * 1.5).scale(0.7).color(RED).build(),
        rotate(sq, PI / 2.0),
        tri.animate().shift(DOWN * 1.5).fill(PURPLE, 0.8).build(),
    ]);
    // morphs
    s.play(transform(
        sq,
        circle(1.0).stroke(TEAL, 5.0).fill(TEAL_E, 0.5),
    ));
    s.play(transform_arc(
        tri,
        star(5, 1.2, 0.5)
            .shift(RIGHT * 4.0 + DOWN * 1.5)
            .fill(GOLD, 1.0)
            .no_stroke(),
        PI / 2.0,
    ));
    s.play(indicate(c));
    s.play(circumscribe(sq, YELLOW));
    s.play(flash(v2(4.0, -1.5), GOLD));
    s.play(wiggle(tri));

    // value tracker + always_redraw
    let k = s.tracker(0.0);
    let dot_path = s.always_redraw(move |ctx| {
        let a = ctx.value(k);
        let p = v2(-4.0 + 8.0 * a, -3.0 + 0.6 * (a * TAU * 2.0).sin());
        dot_r(p, 0.12, ORANGE).glow(0.15, None)
    });
    s.play(k.to(1.0).run_time(2.0).rate(Rate::Linear));

    // camera zoom & pan
    s.play(zoom_to(v2(4.0, -1.5), 4.0).run_time(1.5));
    s.play(camera_reset().run_time(1.5));

    s.play([
        fade_out(c),
        fade_out_shift(sq, DOWN),
        shrink_to_center(tri),
        fade_out(dot_path),
    ]);
    s.wait(0.5);
    run(&s, "animations_tour")
}
