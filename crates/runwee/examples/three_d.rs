//! 3D: orbit camera, shaded surfaces, polyhedra, ambient rotation; plus boolean
//! shape operations and fractals.
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    let rig = s.camera3d(70.0, -45.0);
    let world = axes3d(3.0, 1.0)
        .merge(
            surface_z(
                |x, y| (x * x + y * y).sqrt().sin() * 0.8,
                (-3.0, 3.0),
                (-3.0, 3.0),
                28,
            )
            .opacity(0.92),
        )
        .merge(dot3d(v3(0.0, 0.0, 1.6), 0.18, RED));
    let w = s.add_3d(rig, world);
    s.remove(w);
    s.play(fade_in(w));
    s.ambient_rotation(rig, 0.35, 6.0);
    s.play(rig.phi.to(55.0 * DEGREES).run_time(2.0));
    s.wait(1.5);
    s.play(fade_out(w));

    let solids = cube(1.2, BLUE)
        .shift(v3(-3.0, 0.0, 0.0))
        .merge(icosahedron(0.9, TEAL).shift(v3(-1.0, 0.0, 0.0)))
        .merge(dodecahedron(0.9, GOLD).shift(v3(1.0, 0.0, 0.0)))
        .merge(torus(0.6, 0.22, 16, PURPLE).shift(v3(3.0, 0.0, 0.0)));
    let rig2 = s.camera3d(65.0, -80.0);
    let sol = s.add_3d(rig2, solids);
    s.ambient_rotation(rig2, 0.6, 4.0);
    s.wait(4.0);
    s.play(fade_out(sol));

    // boolean ops & fractal
    let a = circle(1.1)
        .fill(BLUE, 0.6)
        .stroke(BLUE, 3.0)
        .shift(LEFT * 0.6);
    let b = square(1.8)
        .fill(RED, 0.6)
        .stroke(RED, 3.0)
        .shift(RIGHT * 0.6);
    let ops = Mobject::group([
        union(&a, &b).fill(TEAL, 0.8),
        intersection(&a, &b).fill(YELLOW, 0.8),
        difference(&a, &b).fill(GREEN, 0.8),
        exclusion(&a, &b).fill(PURPLE, 0.8),
    ])
    .arrange(RIGHT, 0.6)
    .scale(0.8)
    .to_edge(UP, 0.8);
    let o = s.register(ops);
    s.play(lagged_start(vec![draw_border_then_fill(o)], 0.0));
    let m = s.register(
        mandelbrot(v2(-0.6, 0.0), 3.2, 900, 120, &[], 6.0)
            .to_edge(DOWN, 0.4)
            .shift(LEFT * 3.2),
    );
    let j = s.register(
        julia(v2(-0.8, 0.156), 3.2, 900, 160, &[], 6.0)
            .to_edge(DOWN, 0.4)
            .shift(RIGHT * 3.2),
    );
    s.play([fade_in(m), fade_in(j)]);
    s.wait(1.0);
    run(&s, "three_d")
}
