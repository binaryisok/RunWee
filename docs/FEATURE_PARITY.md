# Feature parity: manim + Remotion → RunWee

This checklist comes from reading the source of
[3b1b/manim](https://github.com/3b1b/manim), [ManimCommunity/manim](https://github.com/ManimCommunity/manim)
and [remotion-dev/remotion](https://github.com/remotion-dev/remotion) (every `class` in
`manimlib/animation`, `manimlib/mobject`, `manim/animation`, `manim/mobject`, and the public
exports of the Remotion packages). Each item maps to a RunWee API. `[x]` = implemented,
`[ ]` = scheduled (the phase number is in the right column).

Out of scope by design: browser-only pieces (Remotion Player/Studio UI, Lambda/Cloud Run,
React-specific hooks, Tailwind, Skia/Three/Rive/Lottie bindings) and manim's interactive
OpenGL window. RunWee covers the same ground natively (fast parallel renderer, stills at any
time, `.rhai` scripts instead of React components).

## manim — animations

| manim | RunWee | Phase |
|---|---|---|
| [x] Create / ShowCreation, Uncreate, ShowPartial | `create`, `uncreate` | 4 |
| [x] Write, Unwrite, DrawBorderThenFill | `write`, `unwrite`, `draw_border_then_fill` | 4 |
| [x] FadeIn / FadeOut (shift, scale) | `fade_in*`, `fade_out*` | 4 |
| [ ] FadeInFromPoint, FadeOutToPoint, VFadeIn/Out, VFadeInThenOut | `fade_in_from_point`, … | 9 |
| [x] GrowFromCenter / Point / Edge, GrowArrow, SpinInFromNothing, ShrinkToCenter | `grow_*`, `spin_in`, `shrink_to_center` | 4 |
| [ ] SpiralIn, TurnInsideOut, Broadcast | `spiral_in`, `turn_inside_out`, `broadcast` | 9 |
| [x] Transform, ReplacementTransform, TransformFromCopy, ClockwiseTransform (path_arc) | `transform`, `transform_arc`, … | 4 |
| [x] FadeTransform, TransformMatchingTex/Parts/Strings | `fade_transform`, `transform_matching` | 4 |
| [ ] TransformMatchingShapes, FadeTransformPieces, CyclicReplace, Swap | | 9 |
| [x] ApplyMethod / `.animate`, ApplyFunction, ApplyMatrix, Homotopy | `.animate()`, `apply_function`, … | 4 |
| [ ] ApplyComplexFunction, ComplexHomotopy, PhaseFlow, SmoothedVectorizedHomotopy | | 9 |
| [x] Rotate, MoveAlongPath | `rotate`, `move_along_path` | 4 |
| [ ] Rotating (continuous), ScaleInPlace, MoveToTarget, Restore (save_state) | | 9 |
| [x] Indicate, Flash, Circumscribe, FocusOn, Wiggle, ApplyWave, ShowPassingFlash, Blink | | 4 |
| [ ] FlashAround/Under, ShowCreationThenDestruction(Around), ShowCreationThenFadeOut, CircleIndicate, AnimatedBoundary | | 9 |
| [x] LaggedStart, AnimationGroup, Succession, Wait | `lagged_start`, `together`, `succession` | 4 |
| [ ] LaggedStartMap, ChangeSpeed | | 9 |
| [x] UpdateFromFunc / UpdateFromAlphaFunc / add_updater / always_redraw | `add_updater`, `custom`, `always_redraw` | 4 |
| [ ] ShowIncreasingSubsets, ShowSubmobjectsOneByOne | | 9 |
| [x] AddTextLetterByLetter (typewriter) | `typewriter` | 4 |
| [ ] AddTextWordByWord, RemoveTextLetterByLetter, TypeWithCursor, UntypeWithCursor | | 9 |
| [ ] ChangingDecimal, ChangeDecimalToValue, CountInFrom | `count_to` | 6 |
| [ ] TracedPath, TracingTail, MaintainPositionRelativeTo | | 9 |

## manim — mobjects

| manim | RunWee | Phase |
|---|---|---|
| [x] Circle, Dot, Ellipse, Arc, ArcBetweenPoints, Annulus, Sector, AnnularSector | `shapes::*` | 2 |
| [x] Square, Rectangle, RoundedRectangle, Polygon, RegularPolygon, Triangle, Star | | 2 |
| [x] Line, DashedLine, Arrow, DoubleArrow, Vector, CurvedArrow, CubicBezier, arrow tips | | 2 |
| [x] Brace, BraceBetweenPoints, Cross, Checkmark, Angle, RightAngle, SurroundingRectangle, BackgroundRectangle, Underline | | 2 |
| [ ] Elbow, TangentialArc, ArcPolygon, Polygram, RegularPolygram, Cutout, Exmark, Elbow, ScreenRectangle | | 9 |
| [ ] Boolean ops Union / Intersection / Difference / Exclusion, ConvexHull | `boolean::*` | 9 |
| [ ] LabeledDot, LabeledLine, LabeledArrow, AnnotationDot, BraceLabel / BraceText, ArcBrace | | 6 |
| [x] Text, MarkupText, Paragraph, Title, BulletedList, Code | `text`, `Text::markup`, `code_block` | 5 |
| [x] MathTex, Tex, SingleStringMathTex (no LaTeX install needed) | `tex`, `tex_text`, `MathTex` | 5 |
| [ ] DecimalNumber, Integer, Variable, ValueTracker display | `DecimalNumber` | 6 |
| [ ] NumberLine, UnitInterval, Axes, NumberPlane, ComplexPlane, PolarPlane, ThreeDAxes | `graph::*` | 6 |
| [ ] FunctionGraph, ParametricFunction, ImplicitFunction, TangentLine, area, Riemann rectangles | | 6 |
| [ ] VectorField, ArrowVectorField, StreamLines, AnimatedStreamLines | | 6 |
| [ ] BarChart, SampleSpace, Table / MathTable / DecimalTable / IntegerTable | | 6 |
| [ ] Matrix, DecimalMatrix, IntegerMatrix, MobjectMatrix | | 6 |
| [ ] Graph / DiGraph (networks with layouts) | | 6 |
| [ ] SVGMobject, ImageMobject (have), VideoMobject, DashedVMobject, CurvesAsSubmobjects | | 8 |
| [ ] SpeechBubble, ThoughtBubble, Clock, Lightbulb, Laptop, DieFace, Speedometer, Piano | `props::*` | 9 |
| [ ] GlowDot, DotCloud, TrueDot, VHighlight | | 9 |
| [ ] Surface, Sphere, Cube, Prism, Cone, Cylinder, Torus, Line3D, Arrow3D, Dot3D, polyhedra | `three_d::*` | 9 |
| [ ] MandelbrotFractal, JuliaFractal, NewtonFractal | | 9 |
| [x] Camera: MovingCamera zoom/pan/rotate, fixed-in-frame | `zoom_to`, `add_fixed` | 4 |
| [ ] ThreeDCamera orientation, ambient rotation | | 9 |

## Remotion — core & utilities

| Remotion | RunWee | Phase |
|---|---|---|
| [x] `useCurrentFrame` / frame = f(t) rendering, renderStill, renderMedia | `Composition`, `render_still`, `render` | 4 |
| [x] `interpolate` (+ extrapolate), `spring`, `Easing.*` incl. `bezier` | `rate::*` | 1 |
| [ ] `measureSpring`, `Easing.poly`, `Easing.step0/step1` | | 8 |
| [ ] `interpolateColors`, `random(seed)`, `@remotion/noise` (2D/3D/4D) | | 8 |
| [ ] `<Sequence>`, `<Series>`, `<Loop>`, `<Freeze>`, `<AbsoluteFill>`, `<Still>` | `compose::*` | 8 |
| [ ] Composition props (`getInputProps`, JSON props) | `--props` | 14 |
| [ ] `<Img>`, `<Video>`/`OffthreadVideo`, `<Gif>`, `staticFile` | | 8 |
| [ ] `<Audio>` with volume curve, trim, playbackRate, loop | `audio::*` | 7 |
| [ ] `@remotion/transitions`: fade, slide, wipe, flip, clock-wipe, iris, none, cross-zoom, zoom-in-out, push-cut, swap, dissolve, blur-slide, linear-blur, zoom-blur, ripple, crosswarp, dreamy-zoom, film-burn, book-flip | `transition::*` | 8 |
| [ ] `@remotion/effects` (CPU subset): blur, brightness, contrast, saturation, hue, grayscale, invert, sepia/duotone, tint, vignette, pixelate, noise, scanlines, halftone, chromatic aberration, glow, drop-shadow, zoom-blur, mirror, wave, fisheye, emboss, levels, exposure, posterize | `effects::*` | 8 |
| [ ] `@remotion/shapes`: arrow, callout, circle, ellipse, heart, pie, polygon, rect, spark, star, triangle | | 8 |
| [ ] `@remotion/paths`: evolvePath, interpolatePath, warpPath, getLength, getPointAtLength, getTangentAtLength, reversePath, parsePath | | 8 |
| [ ] `@remotion/layout-utils`: measureText, fitText, fitTextOnNLines, fillTextBox | | 8 |
| [ ] `@remotion/motion-blur`: Trail, CameraMotionBlur | | 8 |
| [ ] `@remotion/rough-notation`: underline, box, circle, highlight, strike-through, crossed-off, bracket | | 8 |
| [ ] `@remotion/light-leaks`, `@remotion/starburst`, `@remotion/rounded-text-box` | | 8 |
| [ ] `@remotion/captions`: Caption type, parseSrt, serializeSrt, createTikTokStyleCaptions, ensureMaxCharactersPerLine | `captions::*` | 7 |
| [ ] `@remotion/media-utils`: audio duration, waveform, visualizeAudio (FFT spectrum bars) | | 7 |
| [ ] `@remotion/sfx`: whoosh, whip, ding, click, switch, page-turn, shutter… (synthesised, no downloads) | `sfx::*` | 7 |
| [ ] `@remotion/google-fonts` → register any TTF/OTF (`register_font_file`) | | 5 ✔ via files |
| [x] Codecs: h264, vp9 (+alpha), prores (+alpha), gif, png sequence | `Format` | 4 |
