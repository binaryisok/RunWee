# RunWee — Implementation Plan

RunWee is a Rust animation engine for STEM explainer videos. It combines the
**mathematical animation model of manim** (vector objects, `play()`/`wait()`,
`Create`/`Write`/`Transform`, LaTeX-style math, graphs) with the **composition
model of Remotion** (frame = f(t), sequences, transitions, `interpolate()`,
`spring()`, audio with volume curves, captions), and the **asset handling** of
video editors (images, SVG, video clips, sound effects).

Design goals:

| Goal | How |
|------|-----|
| Ultra efficient | Every frame is a pure function of time → frames are rendered **in parallel** (rayon) and streamed in order to ffmpeg. Analytic anti-aliased CPU rasteriser (tiny-skia), no Python, no LaTeX install, no browser. |
| ≥ manim quality | 2K (2560×1440 @ 60 fps) is the default preset, 4K supported. Anti-aliased bezier rendering, gradients, glow, shadows, Computer-Modern math fonts bundled (KaTeX fonts), Inter for UI text. |
| All of STEM | Libraries for maths, physics (mechanics, waves, optics, circuits, fields, simulations), chemistry (periodic table, atoms, molecules, reactions, glassware), biology (cells, DNA, neurons, organelles). |
| Question solving | `Solution` template: problem → given → steps (equations + explanations) → boxed answer, auto-paced to narration. |
| Simple code files | Scenes can be written in Rust **or** as `.rhai` script files rendered by the `runwee` CLI without compiling. |
| Audio + captions | Voice-over blocks that stretch animations to narration length, sound effects, pure-Rust mixing, SRT/VTT export, burned-in captions (subtitle and word-highlight styles), soft subtitles muxed into MP4. |

## Architecture

```
crates/
  runwee/          core library
    core/          math (Vec2/Vec3/affine), color, rate functions & easing, interpolate/spring
    geom/          bezier paths, path ops (partial, resample, align), shapes
    mobject/       Mobject tree (shapes, images, groups), style, layout ops
    text/          font registry, shaping (rustybuzz), Text, TeX-subset math engine, \ce chemistry
    graph/         NumberLine, Axes, NumberPlane, plots, vector fields, charts, tables, matrices
    scene/         timeline builder (play/wait/add/remove), animations, trackers, updaters, camera
    render/        camera, rasteriser, effects (blur/glow), parallel frame pipeline
    output/        ffmpeg encoder (mp4/webm/mov/gif), png sequence, stills
    audio/         wav io, synthesized sfx, mixer, voiceover
    captions/      caption model, SRT/VTT, burned-in caption styles
    compose/       Remotion-style compositions, sequences, transitions, assets (image, svg, video)
    three_d/       3D camera, projection, surfaces, spheres, molecules in 3D
    sim/           ODE solvers (RK4), particle systems
    stem/          physics, chemistry (118 elements), biology, question-solving templates
  runwee-cli/      `runwee` binary: render/still/new/info, Rhai script bindings
examples/          Rust + .rhai example scenes covering every subject
```

### Time model

`Scene` is a *builder*: calls to `play`, `wait`, `add` record **segments** on
per-object timelines. A segment stores the state at its start and an
interpolation closure. Rendering frame *t* evaluates each object's active
segment — no frame depends on the previous one, which gives free parallelism
and random access (instant stills at any time, Remotion style).

## Phases (each phase = one commit, validated by CI)

Status: phases 0–15 are complete. See [FEATURE_PARITY.md](FEATURE_PARITY.md) for the API map.

| # | Phase | Deliverables |
|---|-------|--------------|
| 0 | Scaffold | Workspace, fonts, CI (fmt, clippy, tests, example renders), this plan |
| 1 | Core maths | Vec2/Vec3, affine transforms, colors (manim palette, OKLab mixing), rate functions (manim) + easing (Remotion), `interpolate`, `spring` |
| 2 | Geometry & Mobjects | Cubic bezier paths, arc-length partials, resampling/alignment for morphing, Mobject tree, style, layout (`next_to`, `arrange`, `to_edge`…), 30+ shapes |
| 3 | Renderer | Camera, tiny-skia rasteriser, gradients, dashes, glow & shadow, images, PNG stills |
| 4 | Scene, animations, video | Timeline builder, 40+ animations (Create, Write, Transform, FadeIn/Out, Indicate, Flash, Circumscribe, MoveAlongPath, LaggedStart, Succession, `.animate()` builder…), ValueTrackers, updaters/always_redraw, camera moves, parallel pipeline, ffmpeg mp4/webm/mov/gif/png-seq |
| 5 | Text & math | Font registry, shaped text (kerning, any script), TeX subset (fractions, roots, scripts, big operators, delimiters, matrices, accents, aligned, cases, colors), `\ce{}` chemistry, `MathTex` parts & TransformMatchingTex |
| 6 | Graphing | NumberLine, Axes, NumberPlane, polar plane, function/parametric/implicit plots, area, Riemann sums, tangent lines, vector & stream fields, bar charts, tables, matrices, DecimalNumber/counters |
| 7 | Audio & captions | WAV io, SFX synth, mixer with volume curves, voice-over blocks with automatic timing, TTS hook, SRT/VTT, burned-in captions (bar + word highlight), soft subtitles |
| 8 | Compositions & transitions | Remotion-style `Movie` of sequences, transitions (fade, slide, wipe, iris, clock-wipe, zoom, flip, push), functional compositions, image/SVG/video assets, backgrounds |
| 9 | 3D | 3D camera (orbit/zoom animation), spheres, cubes, prisms, arrows, parametric surfaces with lighting, 3D axes, depth sorting |
| 10 | Physics | Simulations (RK4, pendulum, springs, projectiles, orbits, gas particles), diagrams (incline, pulley, FBD, springs), waves, optics ray tracing, circuits, electric field lines |
| 11 | Chemistry | Periodic table (118 elements), Bohr atoms, electron configuration, molecules (2D structural & 3D ball-and-stick), Lewis dots, reaction equations, glassware, orbitals |
| 12 | Biology | Cells (animal/plant), organelles, DNA helix, neuron, virus, membrane, labelled diagrams, cell division |
| 13 | Question solving | `Solution` template with problem/given/steps/answer, highlight & step transforms, narration-aware pacing |
| 14 | Scripting & CLI | Rhai bindings for the whole API (~480 functions), `runwee render/new/info/api`, `--still`, `--props`, `--vertical`, quality presets |
| 15 | Examples & docs | Example gallery for every subject, README, CI renders examples and uploads videos |
