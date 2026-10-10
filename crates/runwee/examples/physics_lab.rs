//! Physics lab: simulated pendulum with live energy bars, projectile motion,
//! thin-lens ray diagram, a lit circuit with flowing current, and dipole fields.
use runwee::physics::*;
use runwee::prelude::*;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    let header = s.register_fixed(title("Physics Lab"));
    s.play(write(header));

    // 1. pendulum with energy bars, driven by an RK4 trajectory
    let (l, g, th0) = (2.6, 9.81, 0.9);
    let tr = Arc::new(sim::pendulum(l, th0, g, 0.0, 30.0));
    let t0 = s.time();
    let pivot = v2(-3.0, 2.2);
    let tr2 = tr.clone();
    let pend = s.always_redraw(move |ctx| {
        let th = tr2.get(ctx.t - t0, 0);
        pendulum_mobject(pivot, l, th, 0.25, RED)
    });
    let tr3 = tr.clone();
    let bars = s.always_redraw(move |ctx| {
        let st = tr3.at(ctx.t - t0);
        let ke = 0.5 * (l * st[1]).powi(2);
        let pe = g * l * (1.0 - st[0].cos());
        let emax = g * l * (1.0 - th0.cos());
        energy_bars(ke, pe, emax, 2.5).shift(v2(2.5, -2.0))
    });
    let eq = s.register(tex_sized(r"T = 2\pi\sqrt{L/g}", 0.6, YELLOW).move_to(v2(3.4, 2.0)));
    s.play(write(eq));
    s.wait(4.0);
    s.play([fade_out(pend), fade_out(bars), fade_out(eq)]);

    // 2. projectile
    let ground = s.register(line(v2(-6.5, -2.5), v2(6.5, -2.5)).stroke(GRAY_B, 3.0));
    let p0 = v2(-6.0, -2.5);
    let (v, ang, gg) = (8.5, 55.0 * DEGREES, 7.5);
    let path = s.register(projectile_path(p0, v, ang, gg, YELLOW));
    s.play([create(ground), create(path)]);
    let k = s.tracker(0.0);
    let (tof, _, _) = sim::projectile_stats(v, ang, gg);
    let ball = s.always_redraw(move |ctx| {
        let t = ctx.value(k);
        let p = sim::projectile(p0, v, ang, gg, t);
        let vel = v2(v * ang.cos(), v * ang.sin() - gg * t) * 0.25;
        Mobject::group([
            circle_at(p, 0.18).no_stroke().sheen(ORANGE, UL),
            arrow(p, p + v2(vel.x, 0.0)).color(RED_B),
            arrow(p, p + v2(0.0, vel.y)).color(GREEN_B),
        ])
    });
    s.play(k.to(tof).run_time(3.0).rate(Rate::Linear));
    s.play([fade_out(ball), fade_out(path), fade_out(ground)]);

    // 3. lens ray diagram
    let rays = s.register(lens_ray_diagram(v2(0.0, -0.6), 1.8, 4.0, 1.4));
    s.play(create(rays).run_time(2.5));
    s.wait(1.0);
    s.play(fade_out(rays));

    // 4. circuit with current flow and a bulb that lights up
    let (a, b, c, d) = (v2(-3.0, -2.2), v2(3.0, -2.2), v2(3.0, 1.6), v2(-3.0, 1.6));
    let circuit = s.register(Mobject::group([
        battery(a, d),
        wire(&[d, v2(-1.0, 1.6)]),
        resistor(v2(-1.0, 1.6), v2(1.4, 1.6)),
        wire(&[v2(1.4, 1.6), c]),
        bulb(c, b, 0.0),
        wire(&[b, v2(0.6, -2.2)]),
        switch(v2(0.6, -2.2), v2(-0.8, -2.2), true),
        wire(&[v2(-0.8, -2.2), a]),
    ]));
    s.play(create(circuit).run_time(2.0));
    let lit = s.register(bulb(c, b, 1.0));
    s.play(fade_in(lit));
    let loop_path = circuit_loop(&[a, d, c, b]);
    let t1 = s.time();
    let dots = s.always_redraw(move |ctx| current_dots(&loop_path, ctx.t - t1, 1.5, 0.6, YELLOW));
    s.wait(3.0);
    s.play([fade_out(dots), fade_out(circuit), fade_out(lit)]);

    // 5. dipole field lines + equipotentials
    let charges = [(v2(-1.6, -0.5), 1.0), (v2(1.6, -0.5), -1.0)];
    let bounds = Rect::from_center(v2(0.0, -0.5), 13.0, 6.8);
    let lines = s.register(field_lines(&charges, 14, bounds, BLUE_B));
    let eqp = s.register(equipotentials(
        &charges,
        &[-0.6, -0.3, -0.12, 0.12, 0.3, 0.6],
        bounds,
        YELLOW_E,
    ));
    let qs = s.register(Mobject::group(
        charges.iter().map(|(p, q)| point_charge(*p, *q)),
    ));
    s.play(grow_from_center(qs));
    s.play([create(lines).run_time(2.5), create(eqp).run_time(2.5)]);
    s.wait(1.5);
    run(&s, "physics_lab")
}
