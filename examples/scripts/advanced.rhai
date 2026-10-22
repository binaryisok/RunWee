// Advanced animation & art from a script: springs, smart morphs, sketchy
// rendering, generative art, particles, kinetic text, a synth beat driving
// visuals, Verlet ropes, boids, camera shake and stylize filters.
// runwee render examples/scripts/advanced.rhai -q medium
background("#0c0c14");

// 1 ── springs, smart morph, neon
let star_o = register(star(5, 1.6, 0.7).color(palette("neon", 0.2)).fill_opacity(0.8));
play(spring_pop_in(star_o, "bouncy"), 1.0);
play(smart_morph(star_o, circle(1.4).color(palette("neon", 0.6)).fill_opacity(0.8)), 1.2);
play(twist_morph(star_o, square(2.4).color(palette("neon", 0.9)).fill_opacity(0.8), 1.5), 1.2);
play(spring_shift(star_o, LEFT * 4.0, "wobbly"), 1.0);
let ttl = register(text("Advanced RunWee", 0.8).shift(RIGHT * 2.0 + UP * 0.2));
play(neon_write(ttl, "#ff4fa0"), 1.5);
play(shatter(star_o, 40), 1.2);
play(dissolve(ttl, 1500), 1.4);

// 2 ── sketchy shapes + generative art
let art = add(warp_image(0.35, 3.0, 0.0, "sunset").fade(0.35));
let sk = always_redraw(|t| sketchy(circle(1.3).color(WHITE).fill_opacity(0.6).shift(LEFT * 3.5), 0.05, (t * 8.0).floor()));
let tree = add(lsystem("tree", 4, 3.2, "#7a4b2a", "#8cf28c").shift(RIGHT * 3.0 + DOWN * 0.5));
let lab = add(glyph_wave(text("sketchy · L-system · domain warp", 0.4), 0.0, 0.0).shift(DOWN * 3.4));
wait(2.5);
remove(sk); remove(tree); remove(lab); remove(art);

// 3 ── synth beat → reactive visuals, with particles and a camera shake
let b = beat(120, 2, #{kick: "X...x...X..x.x..", snare: "....X.......X...", hat: "x.xxx.x.x.xxx.xX", bass: "1..1..5.3..1..6."});
let viz = always_redraw(|t| {
    let k = beat_env(b, "kick", t, 0.12);
    let s = beat_env(b, "snare", t, 0.2);
    let ring = circle(1.2 + 0.6 * k).color(palette("neon", t * 0.2)).fill_opacity(0.7);
    let halo = circle(2.6 + 0.4 * s).color(WHITE).stroke_width(2.0 + 10.0 * s);
    group([halo, ring, glitch(text("BEAT", 0.9), t, s * 0.8)])
});
let sparks = particles("sparks", vec2(0.0, -2.5));
camera_shake(0.6, 0.6);
wait(b.duration);
remove(viz); remove(sparks);

// 4 ── simulations + stylize
let r1 = rope(vec2(-5.0, 3.5), vec2(-3.0, 3.5), "#ffcc55", 4.0);
let r2 = rope(vec2(-3.5, 3.5), vec2(-1.0, 3.2), "#ff7799", 4.0);
let fl = boids(120, 4.0);
effect_between("crt", 0.12, time() + 2.0, time() + 4.0);
wait(4.0);
