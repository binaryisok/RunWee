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

## CLI

```
runwee render <script.rhai> [options]
    -o, --output <path>     .mp4 .webm .mov .gif .png, or a directory for a PNG sequence
    -q, --quality <q>       low (480p15) | medium (720p30) | high (1080p60) | 2k (default) | 4k
    --still <t>             render one PNG at time t
    --props <json>          JSON object available to the script as PROPS
    --vertical              9:16 portrait frame (shorts / reels)
    --from <t> --to <t>     render part of the timeline
    --transparent           keep alpha (webm vp9 / mov prores 4444 / png)
    --no-audio, --no-captions
runwee new <file.rhai>      write a starter script
runwee info                 ffmpeg / thread / format info
runwee api                  list every script function signature
```

## What's inside

| Area | Highlights |
|---|---|
| Animations | 70+ manim animations, including Create, Write, DrawBorderThenFill, all fades and grows, Transform with path arcs, TransformMatchingTex/Shapes, Indicate, Flash, Circumscribe, ShowPassingFlash, Wiggle, ApplyWave, Homotopy, PhaseFlow, LaggedStart, Succession, typewriter and cursor typing, TracedPath and save_state/restore. Also `.animate()` builders, ValueTrackers, updaters, `always_redraw`, and camera zoom, pan and rotate. |
| Timing | manim rate functions, Remotion easings, cubic Bézier, `spring()` / `measure_spring`, `interpolate` with extrapolation, `Rate::Steps`. |
| Text & math | Shaped text with kerning, markup, wrapping and code blocks; a TeX subset (fractions, roots, scripts, big operators, delimiters, matrices, cases, aligned, accents, colours); `\ce{}` chemistry; MathTex parts. |
| Graphing | NumberLine, Axes, NumberPlane, Complex and Polar planes, function, parametric and implicit plots, area, Riemann sums, tangents, vector and stream fields, bar and pie charts, tables, matrices, networks. |
| Look | Linear and radial gradients, glow, shadows, dashes, sheen, blend modes, and 29 post effects (grain, vignette, blur, chromatic aberration, halftone, scanlines, ...). |
| Composition | `Movie` with 20 transitions (fade, slide, wipe, iris, clock-wipe, flip, cross-zoom, ripple, film-burn, ...), Sequence, Layers, Loop, Freeze, Speed, motion blur, and image, SVG, GIF and video assets. |
| Audio | WAV and ffmpeg decode, volume curves, trims and loops, 17 synthesised SFX, an ambient music bed, a mixer with soft limiter, waveform and FFT spectrum visualisers. |
| Captions | SRT/VTT parse and serialise, TikTok-style word grouping, max chars per line, three burned-in styles, soft subtitles. |
| 3D | Orbit camera, Lambert-shaded meshes, primitives and polyhedra, surfaces, 3D axes, ambient rotation. |
| Physics | RK4 pendulums, springs, projectiles, N-body and ideal gas; FBDs, inclines, Atwood machines; waves; lenses, mirrors, refraction and prisms; circuits with animated current; field lines and equipotentials. |
| Chemistry | 118 elements, periodic table, electron configurations, Bohr atoms, orbital boxes, Lewis dots, molar mass, an exact equation balancer, 2D skeletal and 3D ball-and-stick molecules, lattices, glassware, pH scale. |
| Biology | Animal and plant cells, DNA helix, RNA, neuron with action potential, mitosis, virus, bacterium, membrane, enzymes, Punnett squares, population models, labelled diagrams. |
| Question solving | `Problem` (given/find, steps that morph line to line, highlights, an auto-scrolling work area, a boxed answer, narration); quizzes; derivations; problem generators (quadratic, kinematics, Ohm's law, stoichiometry). |

For the full manim and Remotion mapping, see [docs/FEATURE_PARITY.md](docs/FEATURE_PARITY.md).
For the architecture and phase plan, see [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md).

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
