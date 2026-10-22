# RunWee

**A fast STEM animation engine in Rust.** It pairs manim's maths animation model
(`play`, `Create`, `Write`, `Transform`, TeX, graphs, 3D) with Remotion's composition
model (frame = f(t), sequences, transitions, `interpolate`, `spring`, audio and captions).
It ships ready-made libraries for **maths, physics, chemistry and biology**, plus a
**question-solving** template.

* **Fast.** Every frame is a pure function of time, so frames render in parallel across
  all cores and stream straight into ffmpeg. The defaults are 2K at 60 fps; 4K works too.
  On 4 cores a 1440p60 scene renders at about 47 fps including H.264 encoding, and
  preview quality runs at about 250 fps.
* **No toolchain sprawl.** You don't need Python, LaTeX or a browser. A built-in TeX engine
  uses bundled Computer Modern (KaTeX) fonts, and Inter is bundled for UI text.
* **Two ways to write scenes.** You can write a Rust program, or a `.rhai` script that the
  `runwee` CLI renders without compiling.
* **Audio and captions.** Voice-over blocks pace the animation, an optional TTS hook
  speaks them, and sound effects are synthesised.
  Captions can be burned in (bar, TikTok or karaoke styles) and are also written as
  `.srt`/`.vtt` and muxed into the MP4 as soft subtitles.

## Quick start

```bash
cargo build --release
./target/release/runwee new hello.rhai
./target/release/runwee render hello.rhai -q medium        # → media/hello.mp4
./target/release/runwee render hello.rhai --still 2.5      # → media/hello.png
```

ffmpeg must be on `PATH` for video, GIF and audio output. PNG stills and PNG sequences
work without it.

### A script

```rust
// examples/scripts/physics.rhai
let header = add(title("The simple pendulum"));
play(write(header));

let sim = pendulum_sim(2.0, 0.6, 9.81, 0.0, 6.0);           // RK4: length, θ0, g, damping, duration
let ax = axes([0.0, 6.0, 1.0], [-0.8, 0.8, 0.4]).size(6.0, 2.6).at(vec2(2.8, 0.4)).labels("t", `\theta`);
let frame = register(ax.build());
let curve = register(ax.plot(|t| sim.get(t, 0), 0.0, 6.0).color(YELLOW));
let law = register(tex(`\ddot{\theta} = -\frac{g}{L}\sin\theta`, 0.9).to_edge(DOWN, 0.5));
play([create(frame), write(law)]);

let t0 = time();
always_redraw(|| pendulum(vec2(-4.0, 2.2), 2.0, sim.get(time() - t0, 0), BLUE));
play(create(curve).run_time(6.0).rate("linear"));
```

There are more scripts in [`examples/scripts`](examples/scripts): `hello`, `physics`,
`chemistry`, `quadratic` (a worked solution) and `narrated` (voice-over, captions,
effects and transitions). Run `runwee api` to list all ~480 script functions.

### The same thing in Rust

```rust
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    let eq = s.register(tex(r"e^{i\pi} + 1 = 0").scale(1.6));
    s.play(write(eq));
    s.voiceover(Voiceover::new("Euler's identity links five constants."), |s, _| {
        s.play(circumscribe(eq, YELLOW));
    });
    run(&s, "euler") // -o out.mp4, -q low|medium|high|2k|4k, --still t
}
```

The Rust examples live in [`crates/runwee/examples`](crates/runwee/examples):
`animations_tour`, `graphing`, `narrated_physics`, `remotion_showcase`, `three_d`,
`physics_lab`, `chemistry_lab`, `biology_tour` and `question_solving`.

```bash
cargo run --release -p runwee --example physics_lab -- -q high
```

## Backgrounds and themes

```rust
theme("whiteboard");                  // background plus automatic colour adaptation
background("aurora");                 // or: "blueprint", "graph-paper", "chalkboard", "#1e1e2e", …
background_fade("starfield", 1.5);    // cross-fade mid-scene
background_gradient(["#0f2027", "#2c5364"], -60);
background_image("lab.jpg", 0.4);     // dimmed so content stays readable
background_pattern("dots", "#0E1117", "#2A3442", 0.05);
```

There are 28 presets: dark, black/manim, white, light, paper, chalkboard, whiteboard,
blueprint, graph-paper, grid, dots, notebook, terminal, midnight, sunset, ocean, forest, candy,
spotlight, nebula, aurora, starfield, bokeh, flow and lava.

Light themes darken white text, greys and pastel accents at draw time, so a scene written for a
dark background works unchanged. Any script can be restyled from the command line:
`runwee render x.rhai --theme chalkboard` or `--background photo.jpg`.

## CLI

```
runwee render <script.rhai> [options]
    -o, --output <path>     .mp4 .webm .mov .gif .png, or a directory for a PNG sequence
    -q, --quality <q>       low (480p15) | medium (720p30) | high (1080p60) | 2k (default) | 4k
    --still <t>             render one PNG at time t
    --props <json>          JSON object available to the script as PROPS
    --theme <name>          restyle every scene (any background preset)
    --background <name>     preset, colour, or image/video file
    --vertical              9:16 portrait frame (shorts / reels)
    --from <t> --to <t>     render part of the timeline
    --transparent           keep alpha (webm vp9 / mov prores 4444 / png)
    --no-audio, --no-captions
runwee lesson <lesson-dir> [options]
    --part intro|lecture    render one timeline only (default: both)
    --limit <n>             first n narration steps (for quick previews)
    --info                  print title, presets, steps and narration length
    -o, -q, --still, --theme as above
runwee new <file.rhai>      write a starter script
runwee info                 ffmpeg / thread / format info
runwee api                  list every script function signature
```

## Running existing canvas lessons

`runwee lesson` renders a lesson written for the anims_bms web app (`index.js` with presets,
`simulations.js` with canvas `draw()` code, `mathOverlay.js` and optional narration audio)
without porting it. The original ES modules run unchanged in an embedded QuickJS engine; the
`core/renderer.js` import is swapped for a shim, and browser globals are stubbed.

The runner replays the web app's timeline. Each narration step switches preset and mode,
runs the step's slider tweens, and calls `step()` then `draw()` once per output frame. Every
canvas call is recorded and replayed through RunWee's Canvas 2D implementation (paths, arcs,
gradients, dashes, text, transforms, clipping), so frames come out the same as in the browser.
Around the canvas it lays out a header with the constraint equation, an equation board timed to
the math overlay (falling back to the preset's derivations), voice-over from the lesson's audio
files or TTS, and captions.

```bash
runwee lesson ../anims_bms/topics/mechanics/04-constrained-motion/01-basics --info
runwee lesson ../anims_bms/topics/mechanics/04-constrained-motion/01-basics --limit 3 -q medium
RUNWEE_TTS='piper -m en-us-lessac-medium.onnx -f {out}' runwee lesson <dir> -q 2k
```

The same Canvas 2D API is available to scripts (`let c = canvas2d(800, 400); c.begin_path(); …;
add(c.finish());`), as is a general ODE solver:

```rust
// x'' = -k/m x - c x'  →  y = [x, v]
let sim = ode(|t, y| [y[1], -4.0 * y[0] - 0.3 * y[1]], [1.0, 0.0], 10.0);
always_redraw(|| spring_block(-5.0, 0.0, sim.get(time(), 0), BLUE));
add(sim.trace(0, 1));                 // phase portrait
```

## What's inside

| Area | Highlights |
|---|---|
| Animations | 70+ manim animations, including Create, Write, DrawBorderThenFill, all fades and grows, Transform with path arcs, TransformMatchingTex/Shapes, Indicate, Flash, Circumscribe, ShowPassingFlash, Wiggle, ApplyWave, Homotopy, PhaseFlow, LaggedStart, Succession, typewriter and cursor typing, TracedPath and save_state/restore. Also `.animate()` builders, ValueTrackers, updaters, `always_redraw`, and camera zoom, pan and rotate. |
| Timing | manim rate functions, Remotion easings, cubic Bézier, `spring()` / `measure_spring`, `interpolate` with extrapolation, `Rate::Steps`. |
| Text & math | Shaped text with kerning, markup, wrapping and code blocks; a TeX subset (fractions, roots, scripts, big operators, delimiters, matrices, cases, aligned, accents, colours); `\ce{}` chemistry; MathTex parts. |
| Graphing | NumberLine, Axes, NumberPlane, Complex and Polar planes, function, parametric and implicit plots, area, Riemann sums, tangents, vector and stream fields, bar and pie charts, tables, matrices, networks. |
| Look | Linear and radial gradients, glow, shadows, dashes, sheen and blend modes. All 74 Remotion effects, plus RunWee's own (LUT grading, colour key, light leaks, paper texture, shine, pixel dissolve, ...); any of them can be animated over time. 28 background presets and light/dark themes. |
| Composition | `Movie` with 20 transitions (fade, slide, wipe, iris, clock-wipe, flip, cross-zoom, ripple, film-burn, ...), Sequence, Layers, Loop, Freeze, Speed, motion blur, and image, SVG, GIF and video assets. |
| Audio | WAV and ffmpeg decode, volume curves, trims and loops, 17 synthesised SFX, an ambient music bed, a mixer with soft limiter, waveform and FFT spectrum visualisers. |
| Captions | SRT/VTT parse and serialise, TikTok-style word grouping, max chars per line, three burned-in styles, soft subtitles. |
| 3D | Orbit camera, Lambert-shaded meshes, primitives and polyhedra, surfaces, 3D axes, ambient rotation. |
| Physics | RK4 pendulums, springs, projectiles, N-body and ideal gas; FBDs, inclines, Atwood machines; waves; lenses, mirrors, refraction and prisms; circuits with animated current; field lines and equipotentials. |
| Chemistry | 118 elements, periodic table, electron configurations, Bohr atoms, orbital boxes, Lewis dots, molar mass, an exact equation balancer, 2D skeletal and 3D ball-and-stick molecules, lattices, glassware, pH scale. |
| Biology | Animal and plant cells, DNA helix, RNA, neuron with action potential, mitosis, virus, bacterium, membrane, enzymes, Punnett squares, population models, labelled diagrams. |
| Question solving | `Problem` (given/find, steps that morph line to line, highlights, an auto-scrolling work area, a boxed answer, narration); quizzes; derivations; problem generators (quadratic, kinematics, Ohm's law, stoichiometry). |

A compact list of every feature and script function for coding agents is in [AGENTS.md](AGENTS.md).
For the full manim and Remotion mapping, see [docs/FEATURE_PARITY.md](docs/FEATURE_PARITY.md).
For the architecture and phase plan, see [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md).

## Advanced motion, graphics and art

A second layer of features inspired by After Effects, GSAP, Motion Canvas, rough.js,
p5.js, Shadertoy and fframes (research notes in [docs/ROADMAP_ADVANCED.md](docs/ROADMAP_ADVANCED.md)).
Everything is still a pure function of time: simulations are baked once and sampled,
so stills are random-access and frames render in parallel.

| Phase | Module(s) | Highlights |
|---|---|---|
| A1 Motion core | `motion` | `Keyframes<T>` for any lerpable value (per-key easing, holds, Catmull-Rom smooth keys, repeat / ping-pong loops); closed-form damped `Spring` (bouncy / snappy / gentle / wobbly presets, initial velocity) and `spring_follow` chasing a moving target; AE-style `wiggle`, oscillators and inertial bounce; cosine palettes; GSAP-style `Stagger` (start / end / centre / edges / random / grid / point) and `Timeline` with position parameters and labels; `keyframe_anim`, `spring_pop_in`, `spring_shift`, `wiggle_anim`. |
| A2 Shape & morph | `morph`, `sketch` | flubber-style `smart_morph` (arc-length resampling, best rotation and orientation, multi-subpath), `twist_morph`, Voronoi `shatter` / `assemble`, sand-grain `dissolve` / `materialize`, `neon_write` with a glowing pen tip, `jelly` squash-and-stretch, `echo_trail`, `motion_trail`, rough.js-style `sketchy()` with hachure / cross-hatch / zig-zag fills and line `boil()`. |
| A3 Graphics & art | `noise`, `generative`, `particles`, `kinetic`, `style` | Conic (sweep) gradients; 18 blend modes (`Blend::ALL`); seeded Perlin / simplex / fbm / ridged / turbulence / Worley / curl noise and domain warping; parallel per-pixel `field_image` and `density_image`; flow fields, L-systems, phyllotaxis, circle packing, truchet, Voronoi mosaic, Clifford / De Jong attractors, contour maps, Gray-Scott reaction-diffusion; closed-form particle emitters (fire, sparks, smoke, snow, confetti, fireworks); kinetic type (scramble / decode, glyph waves, RGB-split glitch, text on path, per-glyph fly-in). |
| A4 Camera & 3D | `cinematic`, `sdf` | Per-frame camera modifiers on top of keyed cameras: trauma-based `camera_shake`, lagging `camera_follow`, Catmull-Rom `camera_spline` dolly; `parallax_layer` with cached depth-of-field blur; synthwave `terrain_grid` + `retro_sun`; hyperspace `warp_starfield`; an SDF raymarcher (spheres, rounded boxes, tori, capsules, gyroids, smooth union / subtract / intersect, twist, repeat, displacement, checker materials, soft shadows, AO, fog) rendered in parallel with `raymarch_image`. |
| A5 Wild extras | `wild`, `stylize` | Verlet physics (`Verlet::rope`, `Verlet::cloth` with wind, moving colliders and tearing) and `bake_boids` flocking, baked for deterministic playback; a synthesised `DrumMachine` (kick / snare / hat / clap / bass, step patterns, swing) that writes its audio to the scene and exposes exact hit times, so `hit_envelope` drives visuals in perfect sync; stylize post effects `Effect::Dither` (Bayer, custom palettes), `Kuwahara` (painterly), `PixelSort`, `Ascii`, `Crt`, also by name: `"dither"`, `"kuwahara"`, `"pixel-sort"`, `"ascii"`, `"crt"`. |
| A6 Scripting | `runwee-cli` | Rhai bindings for the headline features (see below) and the `examples/scripts/advanced.rhai` showcase. |

### Run the showcase examples

```bash
# each renders an mp4; add --still 4.5 -o frame.png for a single frame
cargo run --release -p runwee --example motion_lab  -- -q medium -o motion_lab.mp4   # keyframes, springs, stagger, timeline
cargo run --release -p runwee --example shape_morph -- -q medium -o shape_morph.mp4  # smart morph, shatter, dissolve, neon, sketch
cargo run --release -p runwee --example art_gallery -- -q medium -o art_gallery.mp4  # noise fields, flow fields, L-systems, attractors, reaction-diffusion
cargo run --release -p runwee --example fx_lab      -- -q medium -o fx_lab.mp4       # kinetic text, conic gradients, blend modes, particles
cargo run --release -p runwee --example cinema_lab  -- -q medium -o cinema_lab.mp4   # parallax DOF, shake, follow-cam, synthwave, warp, SDF raymarching
cargo run --release -p runwee --example wild_lab    -- -q medium -o wild_lab.mp4     # cloth, ropes, boids, drum machine with sound, stylize filters
cargo run --release -p runwee-cli -- render examples/scripts/advanced.rhai -q medium -o advanced.mp4
```

### Script functions

```js
spring_pop_in(o, "bouncy")   spring_shift(o, LEFT * 3.0, "wobbly")   wiggle_anim(o, 3.0, 0.2, 2.0)
smart_morph(o, circle(1))    twist_morph(o, square(2), 1.5)           shatter(o, 40)  assemble(o, 40)
dissolve(o, 1500)            materialize(o, 1500)  neon_write(o, "#ff4fa0")  jelly(o, 0.2, 3.0)
sketchy(m, 0.05, seed)       boil(m, t, 0.05)      echo_trail(o, 5, 0.08)    motion_trail(o, 0.5, TEAL)
perlin(p, seed)  fbm(p, 5, seed)  ridged(p, 5, seed)  curl(p, t, seed)  domain_warp(p, 3.0, t, seed)
wiggle_at(t, freq, amp, seed)  wiggle_vec(t, freq, amp, seed)  spring_value("snappy", t)  palette("neon", x)
warp_image(0.35, 3.0, t, "sunset")  lsystem("tree", 4, 3.0, DARK_BROWN, GREEN)  truchet(0.5, WHITE, 1)
voronoi_mosaic(80, "ocean", 1)  phyllotaxis(600, 0.12, 0.05, "sunset")  clifford(-1.4, 1.6, 1.0, 0.7, 0.45)
glyph_wave(m, t, 0.2)  glitch(m, t, 0.8)  scramble("DECODE", t, 0.8, WHITE)  glyph_fly_in(o, DOWN, 1.0)
warp_starfield(t, 1.0, WHITE)  retro_sun(vec2(0, 1), 2.0, t)
particles("fire" | "sparks" | "smoke" | "snow" | "confetti" | "firework", vec2(0, -2))
camera_shake(0.6, 0.7)  camera_follow(o, until, 0.3)
rope(vec2(-2, 3), vec2(0, 3), "#ffcc55", 4.0)  boids(150, 5.0)
let b = beat(120, 2, #{kick: "X...x...", snare: "....X...", hat: "x.x.x.x.", bass: "1..5..3."});
always_redraw(|t| circle(1.0 + 0.5 * beat_env(b, "kick", t, 0.12)));   wait(b.duration);
effect_between("kuwahara" | "dither" | "pixel-sort" | "ascii" | "crt", amount, from, to)
```

## Text-to-speech

Narration renders without TTS: it is timed from word counts and shown as captions. To get
real speech, set a command template:

```bash
export RUNWEE_TTS='espeak-ng -w {out} "{text}"'      # or piper, say, an API wrapper...
```

## CI

GitHub Actions runs fmt, clippy (`-D warnings`) and the tests, then renders every Rust
example and `.rhai` script. The videos are uploaded as a build artifact.

## Licence

The code is MIT/Apache-2.0. The bundled fonts keep their own licences; see
[assets/fonts/LICENSE.md](assets/fonts/LICENSE.md).
