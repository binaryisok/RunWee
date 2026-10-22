# RunWee Advanced: motion, morphing, generative art, camera & wild extras

RunWee already covers manim's object/animation model and Remotion's composition
model (see `FEATURE_PARITY.md`). This document is the research + design note for the
next layer: the things motion designers reach for in After Effects, GSAP, Motion Canvas,
Lottie/Rive, p5.js/Processing and Shadertoy, built in RunWee's style: **every frame is
still a pure function of time**, so everything below renders in parallel and supports
random-access stills.

## What we studied

| Library | Ideas worth taking | How RunWee adopts it |
|---|---|---|
| **Manim (3b1b / CE)** | `ValueTracker` + updaters, `Transform` via point alignment, `Write`/`Create` stroke reveal, 3D `ThreeDScene`, camera frame animation | Already present. New: smarter morph alignment (rotation/orientation-optimal), updater-free keyframe tracks, spring-follow trackers. |
| **Remotion** | `interpolate()`, `spring()`, `<Sequence>`, frame = f(t), `@remotion/motion-blur`, `@remotion/noise` | Already present. New: keyframe tracks with per-key easing, stagger helpers, continuous spring *followers* (spring chasing a moving target), shader layers. |
| **Motion Canvas** | Generator timelines (`all`, `sequence`, `chain`, `loop`), signals, `tween` with custom interpolators | `Timeline` builder with GSAP-style position parameters; `Keyframes<T: Lerp>` for any interpolatable value. |
| **GSAP** | Timeline position syntax (`"<"`, `">"`, `"+=0.5"`, labels), `stagger: {each, from: "center"/"edges"/"random", grid}` | `Timeline::add(anim, Pos::…)`, `Stagger { each, from, grid, ease }`. |
| **fframes** (Rust) | Pure `render_frame`, compile-time timelines (`timeline!`), caching static subtrees, Shadertoy shader layers | Pure per-frame model already; new `ShaderLayer` (per-pixel closures rendered in parallel, resolution aware). |
| **Lottie / After Effects** | Track mattes, layer blur, `wiggle()` expression, echo effect, inertial bounce | `LayerFx` (group-level blur / masks / blend / colour effects), `wiggle()`, `echo_trail()`, `inertial_bounce()`. |
| **Rive** | State-machine springs, mesh deformation | Spring physics followers; jelly/noise deformation animations. |
| **flubber** (shape morph) | Equal-arclength resampling + best rotation of start point | `morph::smart_morph` with multi-subpath matching. |
| **rough.js** | Hand-drawn strokes, hachure fills, "boiling" lines | `sketch::sketchy()` + animated boil. |
| **p5.js / Processing / The Nature of Code** | Particles, flow fields, L-systems, Verlet physics, boids, reaction–diffusion, attractors | `particles`, `generative`, `dynamics` modules. |
| **Shadertoy / Inigo Quilez** | SDF raymarching, domain warping, fbm, palettes (`a + b·cos(2π(c·t+d))`) | `noise` (fbm, ridged, domain warp, curl, Worley), `shader` layers, SDF raymarcher, cosine palettes. |

## Design principles

1. **Purity.** No new feature may depend on the previous frame. Simulations
   (Verlet, boids, reaction–diffusion) are pre-baked once at build time into snapshots and
   *sampled* per frame; continuous followers integrate deterministically from 0 to `t`.
2. **Fit the idioms.** New looks are `Mobject`s or `Style` fields, new motion is `Animation`
   (often via `custom`), new screen-space work is `Effect`/`Composition`, Rhai bindings follow
   the existing `register_*` pattern.
3. **Resolution independence.** Raster features (shader layers, layer blur, reaction–diffusion)
   read the active render resolution so 480p previews stay fast and 4K stays sharp.

## Phases

| # | Phase | Contents |
|---|---|---|
| A1 | **Motion core** (`motion.rs`) | `Lerp` trait; `Keyframes<T>` with per-key easing, holds, loops (repeat/ping-pong), Catmull–Rom smooth paths; physical `Spring` (closed form under/critical/over-damped with initial velocity) and `spring_follow` (spring chasing any moving target); AE expressions `wiggle`, `inertial_bounce`, oscillators; GSAP `Stagger` (start/end/center/edges/random/index/grid/point); `Timeline` with position params and labels; `keyframe_anim` (position/scale/rotation/opacity tracks on an object), `tracker_keys`. |
| A2 | **Shape & morph** (`morph.rs`, `sketch.rs`) | Smart morph (arclength resample, best cyclic shift & orientation, subpath matching, spiral morph), Voronoi `shatter`/`assemble`, `dissolve` into particles, `neon_write` (stroke reveal with glowing pen tip), `jelly` wobble, `echo_trail`, hand-drawn `sketchy` rendering with hachure fills and line boil. |
| A3 | **Graphics & art** (`noise.rs`, `layer`, `particles.rs`, `generative.rs`, `kinetic.rs`) | Conic (sweep) gradients and procedural `Field` paints (mesh/freeform gradients, noise fills); 13 new blend modes; `LayerFx` group effects (gaussian blur, drop shadow, outer glow, masks/track mattes, blend, colour effects); fast seeded noise (Perlin, simplex, fbm, ridged, Worley, curl, domain warp); particle emitters (fire, sparks, snow, confetti, fireworks); generative art (flow fields, L-systems, phyllotaxis, circle packing, truchet, Voronoi mosaic, strange attractors, iso-contours, reaction–diffusion); kinetic typography (per-glyph wave, decode/scramble, glitch, text on path, staggered fly-in). |
| A4 | **Camera & 3D** (`camera_fx.rs`, `sdf.rs`) | Camera modifiers (trauma-based shake, spring follow-cam, dolly), parallax layers with depth-of-field blur, SDF raymarcher (spheres, boxes, tori, smooth-union metaballs, soft shadows, AO, fresnel) as a shader layer, 3D starfield / particle tunnels. |
| A5 | **Wild extras** (`shader.rs`, `reactive.rs`, `dynamics.rs`, `stylize.rs`) | Shadertoy-style `ShaderLayer` + built-in shaders (plasma, marble domain warp, aurora, liquid chrome); audio analysis (band energies, onsets, beat pulse) and audio-reactive visualisers; Verlet physics (cloth, soft bodies, ropes, collisions) and boids flocking; artistic post effects (ASCII, ordered dither with palettes, Kuwahara painterly, toon ink edges, pixel sort, RGB block glitch, CRT). |
| A6 | **Scripting & docs** | Rhai bindings for the headline features, `.rhai` showcase, README section, showcase renders. |

Each phase ships unit tests, at least one example in `crates/runwee/examples/`, renders that
are checked frame-by-frame, and its own commit(s).
