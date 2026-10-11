// Spring, pulley and hanging mass, solved with the general ODE solver.
//
// A block (m1) on a smooth table is tied to a wall by a spring (k) and, over a
// pulley, to a hanging mass (m2). With x the stretch of the spring:
//
//     (m1 + m2) x'' = m2 g − k x − c x'
//
// `ode(f, y0, t_end, dt)` integrates dy/dt = f(t, y) with RK4 and returns a
// Trajectory; `state(t)`, `get(t, k)`, `trace(kx, ky)` (a state-space path) and
// `min/max(k)` read it.
//
//   runwee render examples/scripts/ode_pulley.rhai -q 2k

quality("medium");
background("spotlight");

let m1 = 2.0;
let m2 = 1.0;
let k = 6.0;
let c = 0.6;
let g = 9.81;
let T = 10.0;
let sim = ode(|t, y| [y[1], (m2 * g - k * y[0] - c * y[1]) / (m1 + m2)], [0.0, 0.0], T, 0.002);
let x_eq = m2 * g / k;

let header = add(title("Spring + pulley: solved numerically").to_edge(UP, 0.35));
play(write(header));

// geometry
let y_b = 1.4;
let wall = -6.6;
let x0 = -3.2;
let pc = vec2(2.6, y_b - 0.05);
let r = 0.35;
let rope_y = pc.y + r;
let law = register(tex(`(m_1 + m_2)\,\ddot x = m_2 g - k x - c\dot x`, 0.62).move_to(vec2(-2.2, 0.0)));

fn hanging(p) {
    group([
        square(0.6).move_to(p).fill("#E07A5F", 0.9).stroke("#F2A48D", 3.0),
        tex(`m_2`, 0.42, "#FFFFFF").move_to(p),
    ])
}

let t0 = time();
always_redraw(|| {
    let x = 0.5 * sim.get(time() - t0, 0); // drawn at half scale
    let bx = x0 + x;
    let drop = 0.5 + x;
    group([
        spring_block(wall, y_b, bx, "#3D9BE9"),
        tex(`m_1`, 0.42, "#FFFFFF").move_to(vec2(bx, y_b)),
        rope_over_pulley(vec2(bx + 0.4, rope_y), pc, r, vec2(pc.x + r, pc.y - drop), true, "#E9C46A"),
        fixed_pulley(pc, r, y_b + 0.85, "#8D99AE"),
        hanging(vec2(pc.x + r, pc.y - drop - 0.3)),
    ])
});

// x(t) graph and phase portrait
let ax = axes([0.0, T, 2.0], [0.0, 3.5, 1.0]).size(7.0, 2.2).at(vec2(-2.6, -2.05)).labels("t", "x");
let frame = register(ax.build());
let curve = register(ax.plot(|t| sim.get(t, 0), 0.0, T).color(YELLOW));
let eq_line = register(ax.plot(|t| x_eq, 0.0, T).color(GRAY).opacity(0.6));

let pp = axes([0.0, 3.5, 1.0], [-2.5, 2.5, 1.0]).size(2.8, 2.0).at(vec2(5.4, -2.0)).labels("x", "v");
let pp_frame = register(pp.build());
let portrait = register(pp.plot_parametric(|t| vec2(sim.get(t, 0), sim.get(t, 1)), 0.0, T).color("#2EC4B6"));

play([create(frame), create(pp_frame), write(law)]);
play([create(curve).run_time(T).rate("linear"), create(portrait).run_time(T).rate("linear"), fade_in(eq_line)]);
wait(0.5);
