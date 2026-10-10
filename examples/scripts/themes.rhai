// Backgrounds and themes.
//   theme("name")                 — background + colour adaptation (light themes darken white text automatically)
//   background("preset" | colour) — just the background, from now on
//   background_fade("preset", s)  — cross-fade to another background
//   background_gradient(a, b, angle), background_radial(inner, outer), background_corners(a, b, c, d)
//   background_image(path, dim), background_video(path, dim), background_pattern(kind, base, ink, spacing)
// Or restyle any script from the command line:  runwee render x.rhai --theme chalkboard
theme("blueprint");
let t = register(title("Themes & backgrounds"));
play(write(t));
let eq = register(tex(`\nabla \cdot \mathbf{E} = \frac{\rho}{\varepsilon_0}`, 1.4));
play(fade_in(eq, UP));
wait(0.5);

background_fade("aurora", 1.5);
wait(2.0);

next_scene("fade", 0.6);
theme("whiteboard");
let h = register(title("Light theme"));
play(write(h));
let c = register(circle(1.2).color(YELLOW).fill_opacity(0.6).shift(LEFT * 3.0));
let s = register(square(2.0).color(BLUE).fill_opacity(0.6).shift(RIGHT * 3.0));
play([create(c), create(s)]);
play(transform(c, square(2.0).color(RED).fill_opacity(0.6).shift(LEFT * 3.0)));

background_pattern("graph-paper", WHITE, rgb(156, 195, 230), 0.025);
wait(1.0);
