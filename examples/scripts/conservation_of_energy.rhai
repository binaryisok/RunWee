// Law of conservation of energy — a narrated explainer in three scenes:
//   1. the statement of the law,
//   2. a ball rolling on a frictionless track with live KE / PE / total-energy bars,
//   3. a worked exam-style problem.
//
// Render:  runwee render examples/scripts/conservation_of_energy.rhai -q 2k
// Narration is timed from the text and shown as captions; set RUNWEE_TTS to also speak it.

captions("karaoke");
music(0.12);
effect("vignette", 0.35);

// ───────────────────────────── Scene 1: the law ─────────────────────────────
let head = register(title("Conservation of Energy"));
voiceover("Here is one of the most powerful ideas in all of physics.", |info| {
    play(write(head));
});

let law = register(
    rounded_text_box("Energy cannot be created or destroyed — only changed from one form to another.",
                     0.42, WHITE, rgb(33, 43, 64), 9.0).shift(UP * 0.9)
);
voiceover("Energy cannot be created or destroyed. It can only change from one form to another.", |info| {
    play(fade_in(law, UP));
});

let eq = register(tex(`E_{\text{total}} = KE + PE = \text{constant}`, 0.8).shift(DOWN * 0.7));
let ke_def = register(tex(`KE = \tfrac{1}{2}mv^2`, 0.6).color(RED).shift(DOWN * 1.9 + LEFT * 2.4));
let pe_def = register(tex(`PE = mgh`, 0.6).color(BLUE).shift(DOWN * 1.9 + RIGHT * 2.4));
voiceover("For a moving object, the total of kinetic energy, from motion, and potential energy, from height, stays the same.", |info| {
    play(write(eq));
    play(lagged_start([fade_in(ke_def, UP), fade_in(pe_def, UP)], 0.3));
    play(circumscribe(eq, YELLOW));
});
wait(0.5);

// ─────────────────────── Scene 2: ball on a frictionless track ───────────────────────
next_scene("slide:from-right", 0.7);

let m = 2.0;          // kg
let g = 9.8;          // m/s²
let k = 0.3;          // track shape: y = k x²
let amp = 3.2;        // release point (m from the bottom)
let cx = -3.0;        // track centre on screen
let base = -2.2;      // track bottom on screen
let r = 0.28;         // ball radius
let h_max = k * amp * amp;
let e_total = m * g * h_max;

let header = register(bold_text("A ball on a frictionless track", 0.55, WHITE).to_edge(UP, 0.45));

// The track: a parabola with a ground line and a dashed "release height" guide.
let pts = [];
for i in 0..=60 {
    let x = -3.6 + 7.2 * i / 60.0;
    pts.push(vec2(cx + x, base + k * x * x));
}
let track = register(smooth_curve(pts).stroke(GRAY_A, 5.0));
let guide = register(dashed_line(vec2(cx - 3.8, base + h_max), vec2(cx + 3.8, base + h_max)).color(GRAY));
let guide_label = register(tex(`h_{\max}`, 0.6).color(GRAY).move_to(vec2(cx - 2.2, base + h_max + 0.35)));

voiceover("Let's release a two kilogram ball from rest on a smooth, frictionless track.", |info| {
    play(write(header));
    play(create(track), 1.2);
    play([create(guide), fade_in(guide_label)]);
});

// One tracker drives everything: phase of the oscillation (x = A cos φ).
let phase = tracker(0.0);
let omega = (2.0 * g * k).sqrt();   // small-slope oscillation frequency

// Ball, velocity arrow and energy readouts are all recomputed every frame from `phase`.
always_redraw(|| {
    let p = value(phase);
    let x = amp * p.cos();
    let y = k * x * x;
    let slope = 2.0 * k * x;
    let n = vec2(-slope, 1.0).normalize();
    let c = vec2(cx + x, base + y) + n * r;
    let ball = circle(r).color(YELLOW).fill_opacity(1.0).move_to(c);
    // Velocity along the track: v = √(2 g (h_max − h))
    let v = (2.0 * g * (h_max - y)).max(0.0).sqrt();
    let dir = if p.sin() > 0.0 { -1.0 } else { 1.0 };
    let t = vec2(1.0, slope).normalize() * dir;
    let g_ = group([ball]);
    if v > 0.4 {
        g_ = g_.add(arrow(c, c + t * (v * 0.22)).color(RED));
    }
    g_
});

let bars_at = vec2(3.3, -2.3);
always_redraw(|| {
    let x = amp * value(phase).cos();
    let h = k * x * x;
    let pe = m * g * h;
    let ke = e_total - pe;
    let bars = energy_bars(ke, pe, e_total).scale(1.25).move_to(bars_at + vec2(0.0, 1.55));
    let readout = tex(`\begin{aligned}KE &= ${(e_total.round() - pe.round()).to_int()}\,\mathrm{J}\\ PE &= ${pe.round().to_int()}\,\mathrm{J}\\ E &= ${e_total.round().to_int()}\,\mathrm{J}\end{aligned}`, 0.42)
        .move_to(bars_at + vec2(2.75, 1.6));
    group([bars, readout])
});

voiceover("At the top, the ball is not moving, so all of its energy is potential energy.", |info| {
    wait(info.duration);
});

voiceover("As it rolls down, height turns into speed. <bookmark mark='bottom'/> At the very bottom, the energy is all kinetic. <bookmark mark='up'/> Climbing the other side, kinetic energy turns back into potential energy.", |info| {
    let quarter = PI / 2.0;
    play(phase.to(quarter).rate("in_quad"), info.bookmarks.bottom - info.start);
    sound("ding", 0.35);
    play(flash(vec2(cx, base), YELLOW));
    wait_until(info.bookmarks.up);
    play(phase.to(PI).rate("out_quad"), info.end - time());
});

let note = register(markup("[color=#83C167]**Green bar never changes:**[/color] total energy is conserved", 0.36)
    .shift(UP * 2.75));
voiceover("Watch the green bar. No matter where the ball is, the total energy never changes. That is the law of conservation of energy.", |info| {
    play(fade_in(note, UP));
    play(phase.to(3.0 * PI).rate("linear"), info.end - time());
});
play(phase.to(4.0 * PI).rate("linear"), 2.0 * PI / omega);
wait(0.3);

// ───────────────────────────── Scene 3: worked problem ─────────────────────────────
next_scene("fade", 0.6);

let slide = group([
    line(vec2(-2.0, 1.2), vec2(1.2, -1.0)).stroke(GRAY_A, 4.0),
    line(vec2(-2.4, -1.0), vec2(2.4, -1.0)).stroke(GRAY_B, 3.0),
    circle(0.22).color(YELLOW).fill_opacity(1.0).move_to(vec2(-1.8, 1.45)),
    tex(`h = 5\,\mathrm{m}`, 0.55).move_to(vec2(-2.6, 0.1)),
    dashed_line(vec2(-2.0, 1.2), vec2(-2.0, -1.0)).color(GRAY),
]);

let p = problem("Speed at the bottom of a slide",
    "A $2\\,\\mathrm{kg}$ ball is released from rest at a height of $5\\,\\mathrm{m}$ on a frictionless slide. How fast is it moving at the bottom?")
    .given([`m = 2\ \mathrm{kg}`, `h = 5\ \mathrm{m}`, `g = 9.8\ \mathrm{m/s^2}`])
    .find(["v"])
    .step("Energy at the top equals energy at the bottom", `mgh = \tfrac{1}{2}mv^2`)
    .step_morph("The mass cancels on both sides", `gh = \tfrac{1}{2}v^2`)
    .step_morph("Solve for v", `v = \sqrt{2gh}`)
    .step_morph("Substitute the values", `v = \sqrt{2 \times 9.8 \times 5}`)
    .step_morph("Evaluate", `v \approx 9.9\ \mathrm{m/s}`)
    .answer(`v \approx 9.9\ \mathrm{m/s}`)
    .diagram(slide);
solve(p);
wait(1.5);
