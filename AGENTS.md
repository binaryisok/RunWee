# RunWee — agent reference

Rust engine for STEM explainer videos (manim + Remotion feature set). Write a `.rhai` script,
render with the CLI. Every frame is a pure function of time and frames render in parallel.
Full signatures: `runwee api`. Examples: `examples/scripts/*.rhai`.

## CLI
```
runwee render <script.rhai> [-o out.mp4|.webm|.mov|.gif|.png|dir] [-q low|medium|high|2k|4k]
       [--still <t>] [--from t --to t] [--props '<json>'] [--theme <preset>] [--background <x>]
       [--vertical] [--transparent] [--no-audio] [--no-captions]
runwee lesson <anims_bms lesson dir> [--info] [--part intro|lecture] [--limit N] [-q ..] [-o ..]
runwee new <file.rhai> | runwee api | runwee info
```
Quality: low 480p15, medium 720p30, high 1080p60, 2k (default), 4k.
Env: `RUNWEE_TTS='piper -m voice.onnx -f {out}'` (or any command using `{out}`, `{text}`);
without it narration is timed from word count and shown as captions.

## Script model
```rust
quality("medium"); theme("whiteboard"); captions("karaoke");
let t = add(title("Energy"));           // add(Mobject) -> Obj (shown now)
let eq = register(tex(`E = mc^2`, 1.0)); // register: exists but hidden until animated
play(write(eq));                         // play(anim | [anims]) ; play(x, run_time)
play(eq.animate().shift(UP).scale(1.2).run_time(1.5).rate("smooth"));
voiceover("Mass and energy are equivalent.", |info| { play(indicate(eq)); });
let k = tracker(0.0); always_redraw(|| dot(vec2(k.value(), 0.0), 0.1, RED));
play(k.to(3.0));  wait(1.0);  next_scene();
```
- Positions: `vec2(x,y)` (frame ≈ 14.2 × 8, origin centre), constants `UP DOWN LEFT RIGHT ORIGIN PI TAU`.
- Colours: manim names (`RED`, `BLUE_B`, …), hex strings `"#e07a5f"`, `rgb()`, `hsv()`.
- Script `fn`s can't see globals; pass values in or use literals.
- `time()` = current timeline time; `play_bg(anim)` runs an animation without advancing time.

## Objects (Mobject builders; chain `.at() .shift() .scale() .rotate() .color() .fill() .stroke() .opacity() .glow() .shadow() .gradient() .dashed() .next_to() .to_edge() .to_corner() .move_to() .align_to() .arrange() .arrange_grid() .set_width() .set_height() .copy() .set_z()`)
- Shapes: circle, circle_at, dot, glow_dot, square, rectangle, rounded_rectangle, triangle, polygon, regular_polygon, star, polygram, ellipse, annulus, sector, arc, arc_between, line, dashed_line, arrow, double_arrow, curved_arrow, block_arrow, vector, polyline, smooth_curve, brace(_label/_text), angle_arc, right_angle, cross, checkmark, heart, zigzag, spiral, path_d (SVG path), union/intersection/difference/exclusion, convex_hull, surrounding_rectangle, background_rectangle, underline.
- Text: text, bold_text, serif_text, markup, paragraph, title, bullet_list, code, tex, tex_text, tex_colored, tex_parts, chem (`\ce{}`), number, counter, fit_text, rounded_text_box, callout, speech_bubble, thought_bubble.
- Graphs: axes([x0,x1,step],[y0,y1,step]) → `.size() .at() .labels() .build() .plot(f) .plot_parametric .plot_implicit .area .riemann .tangent .vector_field .stream_lines .c2p .p2c`; number_line, number_plane, polar_plane, complex_plane, bar_chart, pie_chart, line_graph, scatter, table, math_table, matrix, network.
- Assets: image(path, h), svg(path, h), svg_str, background_video, music_file, sound_file.
- Canvas 2D (HTML-canvas API → vectors): `let c = canvas2d(800, 400[, x0,y0,x1,y1]); c.fill_style = "#f00"; c.begin_path(); c.arc(...); c.fill(); add(c.finish());` — paths, arc_to, round_rect, transforms, save/restore, clip, gradients, dashes, shadows, fill_text/measure_text.
- 3D: camera3d(phi, theta) + add_3d / add_3d_dynamic, sphere, cube, prism, torus, cylinder, cone, platonic solids, axes3d, arrow3d, line3d, dot3d, surface_z(f, xr, yr, res), curve3d, molecule3d, ambient_rotation, wireframe.

## Animations (`.run_time(s) .rate(name) .delay(s) .lag(x)`)
create, uncreate, write, unwrite, draw_border_then_fill, fade_in/out (+ _scale, _from_point, _to_point), fade_in_then_out, grow_from_center/edge/point, grow_arrow, spin_in, spiral_in, shrink_to_center, transform, replacement_transform, transform_from_copy, transform_matching(_shapes/_from_copy), fade_transform, cyclic_replace, swap, move_along_path, rotating, apply_function, apply_matrix, apply_wave, homotopy, phase_flow, indicate, flash, circumscribe, focus_on, wiggle, pulse, blink, show_passing_flash, show_creation_then_fade_out, show_increasing_subsets, show_submobjects_one_by_one, typewriter, type_with_cursor, add_text_word_by_word, remove_text_letter_by_letter, sweep_highlight, highlight, traced_path, save_state/restore, lagged_start([..], lag), succession([..]), wait_anim, `obj.animate()` builder. Rates: manim names, Remotion easings, `"spring"`, cubic-bezier.
Camera: zoom, zoom_to, pan_to, camera_reset.

## Look
- `background(preset|colour|image)`, background_gradient/radial/pattern/image/video/fade, `theme(name)` (light themes auto-adapt colours). 28 presets: dark, black, white, light, paper, chalkboard, whiteboard, blueprint, graph-paper, grid, dots, notebook, terminal, midnight, sunset, ocean, forest, candy, spotlight, nebula, aurora, starfield, bokeh, flow, lava, …
- Effects (74 Remotion + extra): `effect(name, amount)`, `effect_anim(name, from, to, start, dur)`, `effect_between`, `lut(name, amount)`, light_leak.
- Scene transitions: `next_scene(name, dur)` with fade, slide, wipe, iris, clock-wipe, flip, cross-zoom, ripple, film-burn, …

## Audio / narration / captions
voiceover(text, |info| {...}) (animations inside run during the speech; `info` = #{start, duration, end, bookmarks}), voiceover_audio(text, file, ||{}), narrate(text), music(volume) (ambient bed), music_file, sound(name, vol) (17 synth SFX: whoosh, pop, click, ding, …), sound_file. captions("default"|"karaoke"|"tiktok"|"off") → burned-in + .srt/.vtt files.

## STEM helpers
- Physics sim (RK4 → Trajectory with `.get(t,k) .state(t) .trace(kx,ky) .min(k) .max(k) .duration()`): `ode(|t, y| [...], y0, t_end[, dt])`, pendulum_sim, double_pendulum_sim, spring_sim, driven_oscillator_sim, projectile_drag_sim, n_body_sim + body_pos, projectile, projectile_stats.
- Physics drawing: pendulum, spring_block, hanging_spring, incline, block_on_incline, incline_point, hatched_surface, pulley, fixed_pulley, rope_over_pulley, atwood, force_arrow, free_body, vector_components, energy_bars, projectile_path, travelling_wave, standing_wave, wave_fn, wavefronts, lens, lens_diagram, thin_lens, curved_mirror, refraction, prism_dispersion, resistor, capacitor, inductor, battery, ac_source, bulb, switch, meter, ground, junction, wire, current_dots, point_charge, field_lines, equipotentials, e_field, e_potential, magnetic_field, bar_magnet.
- Chemistry: periodic_table, element_tile, element_name, bohr_atom, electron_config(_short), orbital, orbital_boxes, lewis, molecule (2D/skeletal), molecule3d, molecule_xyz, molar_mass, balance(reactants, products), stoichiometry_problem, beaker, flask, test_tube, burette, burner, ph_scale.
- Biology: animal_cell, plant_cell, mitochondrion, chloroplast, dna, rna, chromosome, mitosis / mitosis_stage, neuron, virus, bacterium, red_blood_cell, lipid_bilayer, enzyme, punnett, logistic, labeled_diagram.
- Maths: mandelbrot, julia, noise2d/3d, lerp, clamp, interpolate, deg.
- Question solving: `problem(title, question).given([...]).diagram(m).step(text, tex).step_morph(..).answer(tex)` then `solve(p)`; quiz(q, options, correct, secs); derivation([tex...], pos, size); quadratic/kinematics/ohms_law/stoichiometry_problem generators.
- Deco: clock, speedometer, progress_bar, die_face, lightbulb, laptop, piano, starburst, spark, marker.

## Lesson runner (anims_bms canvas lessons)
`runwee lesson <dir>` runs a lesson's original `index.js` / `simulations.js` / `mathOverlay.js` in embedded QuickJS, records canvas calls per frame and lays out canvas + header chip + equation board + narration + captions. Lesson audio `audio/audio_<id>.mp3|wav` is used if present, else TTS.

## Rust API (crate `runwee`)
`use runwee::prelude::*;` — `Scene` (add, play, wait, voiceover, always_redraw, set_background, render), mobjects/shapes/text/tex, `graph`, `anim`, `rate`, `effects`, `transition`/`compose` (Movie, Sequence, Layers), `audio`, `captions`, `voice`, `sim`, `physics`, `chemistry`, `biology`, `three_d`, `canvas2d`, `background`. Examples in `crates/runwee/examples/`.

## Dev
`cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`; CI also renders all examples (`scripts/render_examples.sh`). Toolchain 1.97; needs ffmpeg on PATH.
