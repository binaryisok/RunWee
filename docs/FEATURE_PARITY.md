# Feature parity: manim + Remotion → RunWee

This checklist comes from reading the source of
[3b1b/manim](https://github.com/3b1b/manim), [ManimCommunity/manim](https://github.com/ManimCommunity/manim)
and [remotion-dev/remotion](https://github.com/remotion-dev/remotion) (every `class` in
`manimlib/animation`, `manimlib/mobject`, `manim/animation`, `manim/mobject`, and the public
exports of the Remotion packages). Each item maps to a RunWee API. `[x]` = implemented (the phase
that added it is in the right column). Notes in *italics* name the RunWee equivalent where
the API differs.

Out of scope by design: browser-only pieces (Remotion Player/Studio UI, Lambda/Cloud Run,
React-specific hooks, Tailwind, Skia/Three/Rive/Lottie bindings) and manim's interactive
OpenGL window. RunWee covers the same ground natively (fast parallel renderer, stills at any
time, `.rhai` scripts instead of React components).

## Audit summary

Re-checked against freshly cloned sources (October 2026) by listing every class and export and
matching each one to RunWee code:

| Source | Items | In RunWee | Not ported |
|---|---|---|---|
| manim (3b1b `manimlib/animation`, `manimlib/mobject`) + ManimCommunity (`manim/animation`, `manim/mobject`), deduplicated, OpenGL duplicates removed | 319 classes | 298 | 21 interactive/legacy (see below) |
| `@remotion/transitions` presentations | 20 | 20 | — |
| `@remotion/effects` | 74 | 74 | — |
| `@remotion/shapes`, `paths`, `noise`, `layout-utils`, `motion-blur`, `captions`, `media-utils`, `light-leaks`, `starburst`, `rounded-text-box`, `rough-notation`, `gif` | all exports | all | — |
| `@remotion/sfx` | 32 named clips | 11 of the names (whip, whoosh, page turn, UI switch, mouse click, modern and old shutter, ding, vine boom, price-is-right fail, record scratch), all synthesised; RunWee has about 30 sounds in total | 21 meme recordings (copyrighted audio; load any file with `sound_file`) |

**manim classes not ported (21):** interactive-window widgets (Button, Checkbox, Slider,
ColorSliders, ControlPanel, ControlMobject, EnableDisableButton, LinearNumberSlider,
MotionMobject, Textbox), legacy duplicates (OldTex, OldTexText, OldSpeechBubble,
OldThoughtBubble), Typst/MathTypst (a different typesetting engine; RunWee has its own TeX),
VectorizedEarth (3b1b asset), Sprite, ImageMobjectFromCamera, ManimBanner, and the abstract
base CoordinateSystem.

**Remotion pieces that don't apply to a native renderer:** Player, Studio, Lambda/Cloud Run/
serverless, the bundler/CLI/React hooks, Tailwind, GSAP, and the Three.js / Skia / Rive / Lottie
bindings. RunWee has its own 3D engine and renders frames directly. Two gaps worth knowing
about: Lottie files can't be played, and speech-to-text captions (Remotion's Whisper packages)
aren't built in. RunWee captions come from the narration text, or from `.srt`/`.json` files.

## manim — animations

| manim | RunWee | Phase |
|---|---|---|
| [x] Create / ShowCreation, Uncreate, ShowPartial | `create`, `uncreate` | 4 |
| [x] Write, Unwrite, DrawBorderThenFill | `write`, `unwrite`, `draw_border_then_fill` | 4 |
| [x] FadeIn / FadeOut (shift, scale) | `fade_in*`, `fade_out*` | 4 |
| [x] FadeInFromPoint, FadeOutToPoint, VFadeIn/Out, VFadeInThenOut | `fade_in_from_point`, `fade_out_to_point`, `fade_in_then_out` | 9 |
| [x] GrowFromCenter / Point / Edge, GrowArrow, SpinInFromNothing, ShrinkToCenter | `grow_*`, `spin_in`, `shrink_to_center` | 4 |
| [x] SpiralIn, TurnInsideOut, Broadcast | `spiral_in`, `turn_inside_out`, `broadcast` | 9 |
| [x] Transform, ReplacementTransform, TransformFromCopy, ClockwiseTransform (path_arc) | `transform`, `transform_arc`, … | 4 |
| [x] FadeTransform, TransformMatchingTex/Parts/Strings | `fade_transform`, `transform_matching` | 4 |
| [x] TransformMatchingShapes, FadeTransformPieces, CyclicReplace, Swap | `transform_matching_shapes`, `transform_matching_from_copy`, `cyclic_replace`, `swap` | 9 |
| [x] ApplyMethod / `.animate`, ApplyFunction, ApplyMatrix, Homotopy | `.animate()`, `apply_function`, … | 4 |
| [x] ApplyComplexFunction, ComplexHomotopy, PhaseFlow, SmoothedVectorizedHomotopy | `apply_complex_function`, `homotopy` (*complex & smoothed variants via `homotopy` with your own map*), `phase_flow` | 9 |
| [x] Rotate, MoveAlongPath | `rotate`, `move_along_path` | 4 |
| [x] Rotating (continuous), ScaleInPlace, MoveToTarget, Restore (save_state) | `rotating`, `scale_in_place`, `transform` (*target = any mobject*), `save_state`/`restore` | 9 |
| [x] Indicate, Flash, Circumscribe, FocusOn, Wiggle, ApplyWave, ShowPassingFlash, Blink | | 4 |
| [x] FlashAround/Under, ShowCreationThenDestruction(Around), ShowCreationThenFadeOut, CircleIndicate, AnimatedBoundary | `circumscribe` / `show_passing_flash`, `show_creation_then_destruction`, `show_creation_then_fade_out`, `circumscribe_circle`, `animated_boundary` | 9 |
| [x] LaggedStart, AnimationGroup, Succession, Wait | `lagged_start`, `together`, `succession` | 4 |
| [x] LaggedStartMap, ChangeSpeed | `lagged_start_map`, `.rate(Rate::…)` / `compose::Speed` | 9 |
| [x] UpdateFromFunc / UpdateFromAlphaFunc / add_updater / always_redraw | `add_updater`, `custom`, `always_redraw` | 4 |
| [x] ShowIncreasingSubsets, ShowSubmobjectsOneByOne | `show_increasing_subsets`, `show_submobjects_one_by_one` | 9 |
| [x] AddTextLetterByLetter (typewriter) | `typewriter` | 4 |
| [x] AddTextWordByWord, RemoveTextLetterByLetter, TypeWithCursor, UntypeWithCursor | `add_text_word_by_word`, `remove_text_letter_by_letter`, `type_with_cursor`, `untype_with_cursor` | 9 |
| [x] ChangingDecimal, ChangeDecimalToValue, CountInFrom | `Scene::counter` + `tracker.to(v)` | 6 |
| [x] TracedPath, TracingTail, MaintainPositionRelativeTo | `traced_path` (*dissipating = tail*), `maintain_position_relative_to` | 9 |

## manim — mobjects

| manim | RunWee | Phase |
|---|---|---|
| [x] Circle, Dot, Ellipse, Arc, ArcBetweenPoints, Annulus, Sector, AnnularSector | `shapes::*` | 2 |
| [x] Square, Rectangle, RoundedRectangle, Polygon, RegularPolygon, Triangle, Star | | 2 |
| [x] Line, DashedLine, Arrow, DoubleArrow, Vector, CurvedArrow, CubicBezier, arrow tips | | 2 |
| [x] Brace, BraceBetweenPoints, Cross, Checkmark, Angle, RightAngle, SurroundingRectangle, BackgroundRectangle, Underline | | 2 |
| [x] Elbow, TangentialArc, ArcPolygon, Polygram, RegularPolygram, Cutout, Exmark, ScreenRectangle | `elbow`, `arc_between`, `arc_polygon`, `polygram`, `regular_polygram`, `cutout`, `exmark`, `screen_rectangle` | 9 |
| [x] Boolean ops Union / Intersection / Difference / Exclusion, ConvexHull | `boolean::*` | 9 |
| [x] LabeledDot, LabeledLine, LabeledArrow, AnnotationDot, BraceLabel / BraceText | `labeled_dot`, `labeled_line`, `labeled_arrow`, `annotation_dot`, `brace_label`, `brace_text` | 6 |
| [x] Text, MarkupText, Paragraph, Title, BulletedList, Code | `text`, `Text::markup`, `code_block` | 5 |
| [x] MathTex, Tex, SingleStringMathTex (no LaTeX install needed) | `tex`, `tex_text`, `MathTex` | 5 |
| [x] DecimalNumber, Integer, Variable, ValueTracker display | `DecimalNumber` (*0 decimals = Integer*), `variable`, `Scene::counter` | 6 |
| [x] NumberLine, UnitInterval, Axes, NumberPlane, ComplexPlane, PolarPlane, ThreeDAxes | `graph::*`, `three_d::axes3d` | 6 |
| [x] FunctionGraph, ParametricFunction, ImplicitFunction, TangentLine, area, Riemann rectangles | | 6 |
| [x] VectorField, ArrowVectorField, StreamLines, AnimatedStreamLines | | 6 |
| [x] BarChart, SampleSpace, Table / MathTable / DecimalTable / IntegerTable | | 6 |
| [x] Matrix, DecimalMatrix, IntegerMatrix, MobjectMatrix | | 6 |
| [x] Graph / DiGraph (networks with layouts) | `network` (circular/spring/grid/tree layouts, directed edges) | 6 |
| [x] SVGMobject, ImageMobject, VideoMobject, DashedVMobject, CurvesAsSubmobjects | `svg_mobject`, `image_mobject`, `Scene::add_video`, `dashed_mobject`, `curves_as_submobjects` | 8 |
| [x] SpeechBubble, ThoughtBubble, Clock, Lightbulb, Laptop, DieFace, Speedometer, Piano | `deco::*` (`laptop`, `piano`, …) | 9 |
| [x] GlowDot, DotCloud, TrueDot, VHighlight | `glow_dot`, `dot_cloud`, `dot`, `v_highlight` | 9 |
| [x] Surface, Sphere, Cube, Prism, Cone, Cylinder, Torus, Line3D, Arrow3D, Dot3D, polyhedra | `three_d::*` | 9 |
| [x] MandelbrotFractal, JuliaFractal, NewtonFractal | `mandelbrot`, `julia`, `newton_fractal` | 9 |
| [x] Camera: MovingCamera zoom/pan/rotate, fixed-in-frame | `zoom_to`, `add_fixed` | 4 |
| [x] ThreeDCamera orientation, ambient rotation | `Scene::camera3d`, `CameraRig`, `ambient_rotation` | 9 |

## Remotion — core & utilities

| Remotion | RunWee | Phase |
|---|---|---|
| [x] `useCurrentFrame` / frame = f(t) rendering, renderStill, renderMedia | `Composition`, `render_still`, `render` | 4 |
| [x] `interpolate` (+ extrapolate), `spring`, `Easing.*` incl. `bezier` | `rate::*` | 1 |
| [x] `measureSpring`, `Easing.poly`, `Easing.step0/step1` | `measure_spring`, `Rate::Poly`, `Rate::Step0/Step1` | 8 |
| [x] `interpolateColors`, `random(seed)`, `@remotion/noise` (2D/3D/4D) | `interpolate_colors`, `random`, `noise2d/3d/4d` | 8 |
| [x] `<Sequence>`, `<Series>`, `<Loop>`, `<Freeze>`, `<AbsoluteFill>`, `<Still>` | `compose::*` | 8 |
| [x] Composition props (`getInputProps`, JSON props) | `--props` | 14 |
| [x] `<Img>`, `<Video>`/`OffthreadVideo`, `<Gif>`, `staticFile` | `image_mobject`, `VideoSource` / `add_video`, `Gif` / `add_gif`, plain paths | 8 |
| [x] `<Audio>` with volume curve, trim, playbackRate, loop | `audio::*` | 7 |
| [x] `@remotion/transitions`: fade, slide, wipe, flip, clock-wipe, iris, none, cross-zoom, zoom-in-out, push-cut, swap, dissolve, blur-slide, linear-blur, zoom-blur, ripple, crosswarp, dreamy-zoom, film-burn, book-flip | `transition::*` | 8 |
| [x] `@remotion/effects`: all 74, including colour grading (vibrance, white balance, shadows/highlights, colour correction, LUT `.cube` files, colour key, thermal, duotone), blur family (gaussian, axis, region, linear/radial progressive blur and pixelate, zoom blur), geometry (barrel, fisheye, corner pin, skew, scale, translate, mirror, wave(s), tile, pattern, tear, noise displacement), alpha (drop shadow, outline, roughen edges, shrinkwrap), reveals (evolve, pixel dissolve, venetian blinds, shine, light trail) and generators (checkerboard, gridlines, rings, lines, zigzag, dot grid, starburst, light leak, halftone, contour/liquid contours, paper, burlap, flannel) | `Effect::*`, `Effect::from_name("kebab-name", amount)`; scripts: `effect`, `effect_anim`, `effect_between`, `lut` | 8, 16 |
| [x] `@remotion/shapes`: arrow, callout, circle, ellipse, heart, pie, polygon, rect, spark, star, triangle | `block_arrow`, `callout`, `heart`, `pie`, `spark`, `shapes::*` | 8 |
| [x] `@remotion/paths`: evolvePath, interpolatePath, warpPath, getLength, getPointAtLength, getTangentAtLength, reversePath, parsePath | `util::*`, `Path::length`, `Path::reversed` | 8 |
| [x] `@remotion/layout-utils`: measureText, fitText, fitTextOnNLines, fillTextBox | `measure_text`, `fit_text`, `fit_text_on_n_lines`, `FillTextBox` / `fill_text_box` | 8 |
| [x] `@remotion/motion-blur`: Trail, CameraMotionBlur | `compose::MotionBlur` (trail mode) | 8 |
| [x] `@remotion/rough-notation`: underline, box, circle, highlight, strike-through, crossed-off, bracket | `rough_annotation(RoughKind::…)` | 8 |
| [x] `@remotion/light-leaks`, `@remotion/starburst`, `@remotion/rounded-text-box` | `light_leak`, `starburst`, `rounded_text_box` | 8 |
| [x] `@remotion/captions`: Caption type, parseSrt, serializeSrt, createTikTokStyleCaptions, ensureMaxCharactersPerLine | `captions::*` | 7 |
| [x] `@remotion/media-utils`: audio duration, waveform, visualizeAudio (FFT spectrum bars) | `AudioData::duration`, `waveform`, `spectrum_at`, `spectrum_bars` | 7 |
| [x] `@remotion/sfx`: whoosh, whip, ding, click, switch, page-turn, shutter… (synthesised, no downloads) | `sfx::*` | 7 |
| [x] `@remotion/google-fonts` → register any TTF/OTF | `register_font_file` | 5 |
| [x] Codecs: h264, vp9 (+alpha), prores (+alpha), gif, png sequence | `Format` | 4 |

## Beyond manim and Remotion

| Feature | RunWee | Phase |
|---|---|---|
| [x] Physics: RK4 simulations (pendulums, springs, projectiles with drag, N-body, ideal gas), FBDs, inclines, waves, optics ray diagrams, circuits with current flow, field lines | `sim`, `physics` | 10 |
| [x] Chemistry: 118 elements, periodic table, electron configurations, Bohr atoms, orbital boxes, Lewis dots, formula parser, molar mass, equation balancer, 2D/3D molecules, glassware | `chemistry` | 11 |
| [x] Biology: animal/plant cells, DNA/RNA, neuron with action potential, mitosis, virus, bacterium, Punnett squares, population models | `biology` | 12 |
| [x] Question-solving videos: given/find panel, worked steps with glyph-matching morphs, auto-scrolling work, boxed answer, narration, quizzes, problem generators | `solution` | 13 |
| [x] `.rhai` scene scripts rendered without compiling, JSON props, multi-scene movies | `runwee` CLI | 14 |
| [x] Backgrounds: solid, gradients (linear, radial, four-corner), image/video (cover/contain/tile, dim), patterns (grid, graph paper, dots, ruled, notebook, checker, stripes, crosses, isometric), animated (aurora, starfield, bokeh, flowing noise), layers, cross-fades between backgrounds; 28 presets | `Background`, `Scene::set_background`, `background_fade`; scripts: `background(...)`; CLI `--background` | 16 |
| [x] Themes: one call restyles a whole video; light themes remap colours automatically so existing scenes stay readable | `Theme`, `Scene::theme`; scripts: `theme("whiteboard")`; CLI `--theme` | 16 |

## Added in the October 2026 re-audit (phase 16)

manim: `flash_around`, `flash_under`, `show_passing_flash_around`,
`show_creation_then_destruction_around`, `show_creation_then_fade_around`, `v_show_passing_flash`,
`change_speed` / `time_warp`, `flashy_fade_in`, `fade_to_color`, `clockwise_transform`,
`counterclockwise_transform`, `wiggle_out_then_in`, `v_fade_in/out/in_then_out`,
`generate_target` + `move_to_target`, `apply_function_to_center`, `transform_animations`,
`Tracker::changing / count_in_from / with`, `exponential_tracker`, `complex_tracker`, `arc_brace`,
`line_brace`, `tangential_arc`, `arc_polygon_from_arcs`, `labeled_polygram`, outline arrow tips,
`curved_double_arrow`, `small_dot`, `true_dot`, `glow_dots`, `point_cloud_dot`, `dartboard`,
`video_icon`, `bubble`, `double_speech_bubble`, `unit_interval`, `integer`, `integer_matrix`,
`mobject_matrix`, `integer_table`, `decimal_table`, `mobject_table`, `AnimatedStreamLines`,
`time_varying_vector_field`, `digraph`, `textured_surface`, `three_d_model` (.obj), `convex_hull_3d`,
`square3d`, `disk3d`, `v_cube`, `v_prism`, `prismify`, `surface_mesh`.

Remotion: 45 new effects (above), 13 new synthesised sounds (record scratch, old shutter, sad
trombone, ta-da, coin, laser, sparkle, level-up, countdown beeps, heartbeat, drum roll), and the
background/theme system.
