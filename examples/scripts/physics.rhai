// Simple harmonic motion of a pendulum, integrated with RK4, plus a live graph.
quality("medium");
let header = add(title("The simple pendulum"));
play(write(header));

let sim = pendulum_sim(2.0, 0.6, 9.81, 0.0, 6.0);
let pivot = vec2(-4.0, 2.2);
let ax = axes([0.0, 6.0, 1.0], [-0.8, 0.8, 0.4]).size(6.0, 2.6).at(vec2(2.8, 0.4)).labels("t", `\theta`);
let frame = register(ax.build());
let curve = register(ax.plot(|t| sim.get(t, 0), 0.0, 6.0).color(YELLOW));
let law = register(tex(`\ddot{\theta} = -\frac{g}{L}\sin\theta`, 0.9).to_edge(DOWN, 0.5));
play([create(frame), write(law)]);

// The bob follows the simulation; the graph is drawn in lock-step.
let t0 = time();
always_redraw(|| pendulum(pivot, 2.0, sim.get(time() - t0, 0), BLUE));
sound("whoosh", 0.4);
play(create(curve).run_time(6.0).rate("linear"));
play(circumscribe(law, YELLOW));
wait(0.5);
