// The basics: text, TeX, shapes, transforms and an updater-driven counter.
// runwee render examples/scripts/hello.rhai -q medium
let title = add(title("Hello, RunWee"));
play(write(title));

let eq = register(tex(`e^{i\pi} + 1 = 0`, 1.6));
play(fade_in(eq, UP));
play(indicate(eq));

let shape = register(circle(1.0).color(BLUE).fill_opacity(0.5).shift(DOWN * 2.2));
play(create(shape));
play(transform(shape, square(2.0).color(GREEN).fill_opacity(0.5).shift(DOWN * 2.2)));
play(animate(shape).rotate(PI / 4.0).scale(0.6).shift(LEFT * 4.0));

// A tracker drives a live number and a dot that follows a sine curve.
let k = tracker(0.0);
counter(k, 2, vec2(4.5, -2.2), YELLOW, "x = ", "");
always_redraw(|| {
    let x = value(k);
    dot(vec2(2.0 + x, -2.2 + 0.8 * (x * 3.0).sin()), 0.12, YELLOW)
});
play(k.to(PI), 3.0);
wait(1.0);
play(fade_out(eq));
