//! Animation descriptions. They are compiled into timeline segments by [`crate::scene::Scene::play`].

use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::path::Path;
use crate::rate::Rate;
use crate::render::Camera;
use crate::scene::{Obj, Tracker};
use std::sync::Arc;

/// Point-wise map used by `ApplyFunction`.
pub type PointFn = Arc<dyn Fn(Vec2) -> Vec2 + Send + Sync>;
/// Homotopy `(point, t) -> point`.
pub type HomotopyFn = Arc<dyn Fn(Vec2, f64) -> Vec2 + Send + Sync>;
/// Custom per-frame animation `(start_state, eased_alpha) -> state`.
pub type MobFn = Arc<dyn Fn(&Mobject, f64) -> Mobject + Send + Sync>;
/// Custom mobject → mobject function (used by `.animate().apply(...)`).
pub type MapFn = Arc<dyn Fn(Mobject) -> Mobject + Send + Sync>;

/// Where a "grow" animation starts from.
#[derive(Clone, Copy, Debug)]
pub enum GrowFrom {
    Center,
    Point(Vec2),
    Edge(Vec2),
    /// The first point of the path (arrows, lines).
    Start,
}

/// A deferred modification for `.animate()` style animations.
#[derive(Clone)]
pub enum Op {
    Shift(Vec2),
    MoveTo(Vec2),
    MoveToAligned(Vec2, Vec2),
    Scale(f64),
    ScaleAbout(f64, Vec2),
    Stretch(f64, f64),
    Rotate(f64),
    RotateAbout(f64, Vec2),
    SetWidth(f64),
    SetHeight(f64),
    NextTo(crate::path::Rect, Vec2, f64),
    ToEdge(Vec2, f64),
    Color(Color),
    Fill(Color, f32),
    FillOpacity(f32),
    Stroke(Color, f32),
    StrokeWidth(f32),
    Opacity(f32),
    Glow(f64, Option<Color>),
    /// Arbitrary change (falls back to a morph).
    Apply(MapFn),
}

/// Camera animation target; `None` fields keep their current value.
#[derive(Clone, Copy, Debug, Default)]
pub struct CameraTarget {
    pub center: Option<Vec2>,
    pub frame_height: Option<f64>,
    pub rotation: Option<f64>,
    pub background: Option<Color>,
}

impl CameraTarget {
    pub fn apply(&self, c: &Camera) -> Camera {
        Camera {
            center: self.center.unwrap_or(c.center),
            frame_height: self.frame_height.unwrap_or(c.frame_height),
            rotation: self.rotation.unwrap_or(c.rotation),
            background: self.background.unwrap_or(c.background),
        }
    }
}

#[derive(Clone)]
pub enum AnimKind {
    Create(Obj),
    Uncreate(Obj),
    Write(Obj),
    Unwrite(Obj),
    DrawBorderThenFill(Obj),
    FadeIn {
        obj: Obj,
        shift: Vec2,
        scale: f64,
    },
    FadeOut {
        obj: Obj,
        shift: Vec2,
        scale: f64,
    },
    Grow {
        obj: Obj,
        from: GrowFrom,
    },
    SpinIn(Obj),
    ShrinkToCenter(Obj),
    Transform {
        obj: Obj,
        target: Mobject,
        arc: f64,
    },
    ReplacementTransform {
        from: Obj,
        to: Obj,
        arc: f64,
    },
    TransformFromCopy {
        from: Obj,
        to: Obj,
        arc: f64,
    },
    FadeTransform {
        obj: Obj,
        target: Mobject,
    },
    TransformMatchingShapes {
        obj: Obj,
        target: Mobject,
    },
    TransformMatching {
        obj: Obj,
        target: Mobject,
    },
    Animate {
        obj: Obj,
        ops: Vec<Op>,
    },
    Rotate {
        obj: Obj,
        angle: f64,
        about: Option<Vec2>,
    },
    MoveAlongPath {
        obj: Obj,
        path: Path,
        orient: bool,
    },
    ApplyFunction {
        obj: Obj,
        f: PointFn,
    },
    Homotopy {
        obj: Obj,
        h: HomotopyFn,
    },
    Indicate {
        obj: Obj,
        color: Color,
        scale: f64,
    },
    Flash {
        point: Vec2,
        color: Color,
        radius: f64,
        lines: usize,
    },
    Circumscribe {
        obj: Obj,
        color: Color,
        circle: bool,
    },
    ShowPassingFlash {
        obj: Obj,
        width: f64,
    },
    Wiggle {
        obj: Obj,
        scale: f64,
        angle: f64,
        wiggles: f64,
    },
    FocusOn {
        point: Vec2,
        color: Color,
    },
    ApplyWave {
        obj: Obj,
        amplitude: f64,
    },
    Pulse {
        obj: Obj,
        scale: f64,
    },
    Blink(Obj),
    Typewriter(Obj),
    Highlight {
        obj: Obj,
        color: Color,
    },
    TrackerTo {
        tracker: Tracker,
        value: f64,
    },
    Camera(CameraTarget),
    Wait,
    Group {
        anims: Vec<Animation>,
        lag: f64,
    },
    Custom {
        obj: Obj,
        f: MobFn,
        introduces: bool,
        removes: bool,
    },
}

/// An animation plus its timing parameters.
#[derive(Clone)]
pub struct Animation {
    pub kind: AnimKind,
    pub run_time: Option<f64>,
    pub rate: Rate,
    pub lag_ratio: Option<f64>,
    pub delay: f64,
}

impl Animation {
    pub fn new(kind: AnimKind) -> Self {
        let rate = match &kind {
            AnimKind::Indicate { .. } | AnimKind::Pulse { .. } | AnimKind::Blink(_) => {
                Rate::ThereAndBack
            }
            AnimKind::Wiggle { .. } | AnimKind::Typewriter(_) | AnimKind::Wait => Rate::Linear,
            AnimKind::ShowPassingFlash { .. } | AnimKind::Flash { .. } => Rate::Linear,
            AnimKind::MoveAlongPath { .. } => Rate::Smooth,
            _ => Rate::Smooth,
        };
        Self {
            kind,
            run_time: None,
            rate,
            lag_ratio: None,
            delay: 0.0,
        }
    }
    pub fn run_time(mut self, t: f64) -> Self {
        self.run_time = Some(t);
        self
    }
    pub fn rate(mut self, r: Rate) -> Self {
        self.rate = r;
        self
    }
    pub fn lag(mut self, l: f64) -> Self {
        self.lag_ratio = Some(l);
        self
    }
    /// Start this animation `d` seconds into the `play` call.
    pub fn delay(mut self, d: f64) -> Self {
        self.delay = d;
        self
    }
    pub fn wait(d: f64) -> Self {
        Animation::new(AnimKind::Wait).run_time(d)
    }
}

/// Things accepted by `Scene::play`.
pub trait IntoAnims {
    fn into_anims(self) -> Vec<Animation>;
}
impl IntoAnims for Animation {
    fn into_anims(self) -> Vec<Animation> {
        vec![self]
    }
}
impl IntoAnims for Vec<Animation> {
    fn into_anims(self) -> Vec<Animation> {
        self
    }
}
impl<const N: usize> IntoAnims for [Animation; N] {
    fn into_anims(self) -> Vec<Animation> {
        self.into()
    }
}
impl IntoAnims for AnimateBuilder {
    fn into_anims(self) -> Vec<Animation> {
        vec![self.into()]
    }
}

// ---------------------------------------------------------------- builders

pub fn create(o: Obj) -> Animation {
    Animation::new(AnimKind::Create(o))
}
pub fn uncreate(o: Obj) -> Animation {
    Animation::new(AnimKind::Uncreate(o))
}
pub fn write(o: Obj) -> Animation {
    Animation::new(AnimKind::Write(o))
}
pub fn unwrite(o: Obj) -> Animation {
    Animation::new(AnimKind::Unwrite(o))
}
pub fn draw_border_then_fill(o: Obj) -> Animation {
    Animation::new(AnimKind::DrawBorderThenFill(o)).run_time(2.0)
}
pub fn fade_in(o: Obj) -> Animation {
    Animation::new(AnimKind::FadeIn {
        obj: o,
        shift: Vec2::ZERO,
        scale: 1.0,
    })
}
/// Fade in while moving along `shift` (arrives at its position).
pub fn fade_in_shift(o: Obj, shift: Vec2) -> Animation {
    Animation::new(AnimKind::FadeIn {
        obj: o,
        shift,
        scale: 1.0,
    })
}
pub fn fade_in_scale(o: Obj, scale: f64) -> Animation {
    Animation::new(AnimKind::FadeIn {
        obj: o,
        shift: Vec2::ZERO,
        scale,
    })
}
pub fn fade_out(o: Obj) -> Animation {
    Animation::new(AnimKind::FadeOut {
        obj: o,
        shift: Vec2::ZERO,
        scale: 1.0,
    })
}
pub fn fade_out_shift(o: Obj, shift: Vec2) -> Animation {
    Animation::new(AnimKind::FadeOut {
        obj: o,
        shift,
        scale: 1.0,
    })
}
pub fn fade_out_scale(o: Obj, scale: f64) -> Animation {
    Animation::new(AnimKind::FadeOut {
        obj: o,
        shift: Vec2::ZERO,
        scale,
    })
}
pub fn grow_from_center(o: Obj) -> Animation {
    Animation::new(AnimKind::Grow {
        obj: o,
        from: GrowFrom::Center,
    })
}
pub fn grow_from_point(o: Obj, p: Vec2) -> Animation {
    Animation::new(AnimKind::Grow {
        obj: o,
        from: GrowFrom::Point(p),
    })
}
pub fn grow_from_edge(o: Obj, dir: Vec2) -> Animation {
    Animation::new(AnimKind::Grow {
        obj: o,
        from: GrowFrom::Edge(dir),
    })
}
pub fn grow_arrow(o: Obj) -> Animation {
    Animation::new(AnimKind::Grow {
        obj: o,
        from: GrowFrom::Start,
    })
}
pub fn spin_in(o: Obj) -> Animation {
    Animation::new(AnimKind::SpinIn(o))
}
pub fn shrink_to_center(o: Obj) -> Animation {
    Animation::new(AnimKind::ShrinkToCenter(o))
}
pub fn transform(o: Obj, target: Mobject) -> Animation {
    Animation::new(AnimKind::Transform {
        obj: o,
        target,
        arc: 0.0,
    })
}
/// Transform where points travel along arcs of angle `arc` (manim's `path_arc`).
pub fn transform_arc(o: Obj, target: Mobject, arc: f64) -> Animation {
    Animation::new(AnimKind::Transform {
        obj: o,
        target,
        arc,
    })
}
pub fn replacement_transform(from: Obj, to: Obj) -> Animation {
    Animation::new(AnimKind::ReplacementTransform { from, to, arc: 0.0 })
}
pub fn transform_from_copy(from: Obj, to: Obj) -> Animation {
    Animation::new(AnimKind::TransformFromCopy { from, to, arc: 0.0 })
}
pub fn fade_transform(o: Obj, target: Mobject) -> Animation {
    Animation::new(AnimKind::FadeTransform { obj: o, target })
}
/// Morph matching parts by tag (TeX parts): manim's `TransformMatchingTex`.
pub fn transform_matching(o: Obj, target: Mobject) -> Animation {
    Animation::new(AnimKind::TransformMatching { obj: o, target })
}
/// Morph matching individual glyphs/pieces by tag; the rest fade (manim's
/// `TransformMatchingShapes`).
pub fn transform_matching_shapes(o: Obj, target: Mobject) -> Animation {
    Animation::new(AnimKind::TransformMatchingShapes { obj: o, target })
}
/// Apply the same animation constructor to many objects with lag (manim's `LaggedStartMap`).
pub fn lagged_start_map(objs: &[Obj], f: impl Fn(Obj) -> Animation, lag_ratio: f64) -> Animation {
    lagged_start(objs.iter().map(|o| f(*o)).collect(), lag_ratio)
}
pub fn rotate(o: Obj, angle: f64) -> Animation {
    Animation::new(AnimKind::Rotate {
        obj: o,
        angle,
        about: None,
    })
}
pub fn rotate_about(o: Obj, angle: f64, about: Vec2) -> Animation {
    Animation::new(AnimKind::Rotate {
        obj: o,
        angle,
        about: Some(about),
    })
}
pub fn move_along_path(o: Obj, path: &Mobject) -> Animation {
    Animation::new(AnimKind::MoveAlongPath {
        obj: o,
        path: path.family_path(),
        orient: false,
    })
}
pub fn move_along_path_oriented(o: Obj, path: &Mobject) -> Animation {
    Animation::new(AnimKind::MoveAlongPath {
        obj: o,
        path: path.family_path(),
        orient: true,
    })
}
pub fn apply_function(o: Obj, f: impl Fn(Vec2) -> Vec2 + Send + Sync + 'static) -> Animation {
    Animation::new(AnimKind::ApplyFunction {
        obj: o,
        f: Arc::new(f),
    })
    .run_time(3.0)
}
/// Apply the 2×2 matrix `[[a, b], [c, d]]` about the origin (manim's `ApplyMatrix`).
pub fn apply_matrix(o: Obj, m: [[f64; 2]; 2]) -> Animation {
    apply_function(o, move |p| {
        v2(m[0][0] * p.x + m[0][1] * p.y, m[1][0] * p.x + m[1][1] * p.y)
    })
}
pub fn homotopy(o: Obj, h: impl Fn(Vec2, f64) -> Vec2 + Send + Sync + 'static) -> Animation {
    Animation::new(AnimKind::Homotopy {
        obj: o,
        h: Arc::new(h),
    })
    .run_time(3.0)
}
pub fn indicate(o: Obj) -> Animation {
    Animation::new(AnimKind::Indicate {
        obj: o,
        color: YELLOW,
        scale: 1.2,
    })
}
pub fn indicate_with(o: Obj, color: Color, scale: f64) -> Animation {
    Animation::new(AnimKind::Indicate {
        obj: o,
        color,
        scale,
    })
}
pub fn flash(point: Vec2, color: Color) -> Animation {
    Animation::new(AnimKind::Flash {
        point,
        color,
        radius: 0.3,
        lines: 12,
    })
}
pub fn circumscribe(o: Obj, color: Color) -> Animation {
    Animation::new(AnimKind::Circumscribe {
        obj: o,
        color,
        circle: false,
    })
}
pub fn circumscribe_circle(o: Obj, color: Color) -> Animation {
    Animation::new(AnimKind::Circumscribe {
        obj: o,
        color,
        circle: true,
    })
}
pub fn show_passing_flash(o: Obj, width: f64) -> Animation {
    Animation::new(AnimKind::ShowPassingFlash { obj: o, width })
}
pub fn wiggle(o: Obj) -> Animation {
    Animation::new(AnimKind::Wiggle {
        obj: o,
        scale: 1.1,
        angle: 0.01 * TAU,
        wiggles: 6.0,
    })
    .run_time(2.0)
}
pub fn focus_on(point: Vec2) -> Animation {
    Animation::new(AnimKind::FocusOn { point, color: GRAY })
}
pub fn apply_wave(o: Obj) -> Animation {
    Animation::new(AnimKind::ApplyWave {
        obj: o,
        amplitude: 0.2,
    })
}
pub fn pulse(o: Obj) -> Animation {
    Animation::new(AnimKind::Pulse {
        obj: o,
        scale: 1.15,
    })
    .run_time(0.6)
}
pub fn blink(o: Obj) -> Animation {
    Animation::new(AnimKind::Blink(o)).run_time(0.4)
}
/// Reveal glyphs/parts one by one (Remotion-style typewriter).
pub fn typewriter(o: Obj) -> Animation {
    Animation::new(AnimKind::Typewriter(o))
}
/// Flash a translucent highlight box behind the object.
pub fn highlight(o: Obj, color: Color) -> Animation {
    Animation::new(AnimKind::Highlight { obj: o, color })
}
pub fn custom(o: Obj, f: impl Fn(&Mobject, f64) -> Mobject + Send + Sync + 'static) -> Animation {
    Animation::new(AnimKind::Custom {
        obj: o,
        f: Arc::new(f),
        introduces: false,
        removes: false,
    })
}
pub fn camera_to(target: CameraTarget) -> Animation {
    Animation::new(AnimKind::Camera(target))
}
/// Zoom & pan the camera so that `frame_height` units around `center` are visible.
pub fn zoom_to(center: Vec2, frame_height: f64) -> Animation {
    camera_to(CameraTarget {
        center: Some(center),
        frame_height: Some(frame_height),
        ..Default::default()
    })
}
pub fn pan_to(center: Vec2) -> Animation {
    camera_to(CameraTarget {
        center: Some(center),
        ..Default::default()
    })
}
/// Restore the default camera.
pub fn camera_reset() -> Animation {
    camera_to(CameraTarget {
        center: Some(Vec2::ZERO),
        frame_height: Some(8.0),
        rotation: Some(0.0),
        background: None,
    })
}
/// Run animations together, each starting `lag` × (previous duration) after the previous.
pub fn group_anims(anims: Vec<Animation>, lag: f64) -> Animation {
    Animation::new(AnimKind::Group { anims, lag })
}
/// All at once.
pub fn together(anims: Vec<Animation>) -> Animation {
    group_anims(anims, 0.0)
}
pub fn lagged_start(anims: Vec<Animation>, lag_ratio: f64) -> Animation {
    group_anims(anims, lag_ratio)
}
/// One after another.
pub fn succession(anims: Vec<Animation>) -> Animation {
    group_anims(anims, 1.0)
}

/// Builder returned by [`Obj::animate`]: records changes, then animates to the result.
#[derive(Clone)]
pub struct AnimateBuilder {
    pub obj: Obj,
    pub ops: Vec<Op>,
    pub run_time: Option<f64>,
    pub rate: Option<Rate>,
}

impl AnimateBuilder {
    pub fn new(obj: Obj) -> Self {
        Self {
            obj,
            ops: vec![],
            run_time: None,
            rate: None,
        }
    }
    fn op(mut self, o: Op) -> Self {
        self.ops.push(o);
        self
    }
    pub fn shift(self, v: Vec2) -> Self {
        self.op(Op::Shift(v))
    }
    pub fn move_to(self, p: Vec2) -> Self {
        self.op(Op::MoveTo(p))
    }
    pub fn move_to_aligned(self, p: Vec2, edge: Vec2) -> Self {
        self.op(Op::MoveToAligned(p, edge))
    }
    pub fn scale(self, s: f64) -> Self {
        self.op(Op::Scale(s))
    }
    pub fn scale_about(self, s: f64, p: Vec2) -> Self {
        self.op(Op::ScaleAbout(s, p))
    }
    pub fn stretch(self, sx: f64, sy: f64) -> Self {
        self.op(Op::Stretch(sx, sy))
    }
    pub fn rotate(self, a: f64) -> Self {
        self.op(Op::Rotate(a))
    }
    pub fn rotate_about(self, a: f64, p: Vec2) -> Self {
        self.op(Op::RotateAbout(a, p))
    }
    pub fn set_width(self, w: f64) -> Self {
        self.op(Op::SetWidth(w))
    }
    pub fn set_height(self, h: f64) -> Self {
        self.op(Op::SetHeight(h))
    }
    pub fn next_to(self, target: &Mobject, dir: Vec2, buff: f64) -> Self {
        self.op(Op::NextTo(target.bounds_or_origin(), dir, buff))
    }
    pub fn to_edge(self, dir: Vec2, buff: f64) -> Self {
        self.op(Op::ToEdge(dir, buff))
    }
    pub fn color(self, c: Color) -> Self {
        self.op(Op::Color(c))
    }
    pub fn fill(self, c: Color, o: f32) -> Self {
        self.op(Op::Fill(c, o))
    }
    pub fn fill_opacity(self, o: f32) -> Self {
        self.op(Op::FillOpacity(o))
    }
    pub fn stroke(self, c: Color, w: f32) -> Self {
        self.op(Op::Stroke(c, w))
    }
    pub fn stroke_width(self, w: f32) -> Self {
        self.op(Op::StrokeWidth(w))
    }
    pub fn opacity(self, o: f32) -> Self {
        self.op(Op::Opacity(o))
    }
    pub fn glow(self, r: f64, c: Option<Color>) -> Self {
        self.op(Op::Glow(r, c))
    }
    pub fn apply(self, f: impl Fn(Mobject) -> Mobject + Send + Sync + 'static) -> Self {
        self.op(Op::Apply(Arc::new(f)))
    }
    pub fn run_time(mut self, t: f64) -> Self {
        self.run_time = Some(t);
        self
    }
    pub fn rate(mut self, r: Rate) -> Self {
        self.rate = Some(r);
        self
    }
    pub fn build(self) -> Animation {
        self.into()
    }
}

impl From<AnimateBuilder> for Animation {
    fn from(b: AnimateBuilder) -> Self {
        let mut a = Animation::new(AnimKind::Animate {
            obj: b.obj,
            ops: b.ops,
        });
        a.run_time = b.run_time;
        if let Some(r) = b.rate {
            a.rate = r;
        }
        a
    }
}

/// Apply ops to a mobject (used to compute `.animate()` targets).
pub fn apply_op(m: Mobject, op: &Op) -> Mobject {
    match op {
        Op::Shift(v) => m.shift(*v),
        Op::MoveTo(p) => m.move_to(*p),
        Op::MoveToAligned(p, e) => m.move_to_aligned(*p, *e),
        Op::Scale(s) => m.scale(*s),
        Op::ScaleAbout(s, p) => m.scale_about(*s, *p),
        Op::Stretch(x, y) => m.stretch(*x, *y),
        Op::Rotate(a) => m.rotate(*a),
        Op::RotateAbout(a, p) => m.rotate_about(*a, *p),
        Op::SetWidth(w) => m.set_width(*w),
        Op::SetHeight(h) => m.set_height(*h),
        Op::NextTo(r, d, b) => m.next_to_rect(*r, *d, *b, Vec2::ZERO),
        Op::ToEdge(d, b) => m.to_edge(*d, *b),
        Op::Color(c) => m.color(*c),
        Op::Fill(c, o) => m.fill(*c, *o),
        Op::FillOpacity(o) => m.fill_opacity(*o),
        Op::Stroke(c, w) => m.stroke(*c, *w),
        Op::StrokeWidth(w) => m.stroke_width(*w),
        Op::Opacity(o) => m.opacity(*o),
        Op::Glow(r, c) => m.glow(*r, *c),
        Op::Apply(f) => f(m),
    }
}
