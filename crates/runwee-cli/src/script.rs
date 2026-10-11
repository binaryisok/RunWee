//! `.rhai` scene scripts: the RunWee API exposed to Rhai so videos can be written as
//! simple code files and rendered without compiling Rust.
//!
//! Numbers may be written as integers or floats everywhere (`circle(1)` works).
//! Script closures (`always_redraw(|t| ...)`, updaters, plots) are called from the
//! parallel renderer, so the engine is built with Rhai's `sync` feature.

use rhai::{Array, Dynamic, Engine, EvalAltResult, FnPtr, Map, NativeCallContext, Scope, AST};
use runwee::chemistry as chem;
use runwee::prelude::*;
use runwee::{biology as bio, physics as phys};
use std::cell::Cell;
use std::sync::{Arc, Mutex, OnceLock};

type N = Dynamic;
type R<T> = Result<T, Box<EvalAltResult>>;

fn f(d: &Dynamic) -> f64 {
    if let Ok(v) = d.as_float() {
        v
    } else if let Ok(i) = d.as_int() {
        i as f64
    } else if let Ok(b) = d.as_bool() {
        if b {
            1.0
        } else {
            0.0
        }
    } else {
        0.0
    }
}

fn col(d: &Dynamic) -> Color {
    if let Some(c) = d.clone().try_cast::<Color>() {
        return c;
    }
    if let Ok(s) = d.clone().into_immutable_string() {
        return Color::parse(&s).unwrap_or(WHITE);
    }
    WHITE
}

fn vec_of(d: &Dynamic) -> Vec2 {
    if let Some(v) = d.clone().try_cast::<Vec2>() {
        return v;
    }
    if let Some(a) = d.clone().try_cast::<Array>() {
        if a.len() >= 2 {
            return v2(f(&a[0]), f(&a[1]));
        }
    }
    Vec2::ZERO
}

fn floats(a: &Array) -> Vec<f64> {
    a.iter().map(f).collect()
}
fn strings(a: &Array) -> Vec<String> {
    a.iter().map(|d| d.to_string()).collect()
}
fn colors(a: &Array) -> Vec<Color> {
    a.iter().map(col).collect()
}
fn points(a: &Array) -> Vec<Vec2> {
    a.iter().map(vec_of).collect()
}
fn range3(a: &Array) -> (f64, f64, f64) {
    let v = floats(a);
    (
        *v.first().unwrap_or(&-5.0),
        *v.get(1).unwrap_or(&5.0),
        *v.get(2).unwrap_or(&1.0),
    )
}
fn mobs(a: &Array) -> Vec<Mobject> {
    a.iter()
        .filter_map(|d| d.clone().try_cast::<Mobject>())
        .collect()
}

/// Collect animations from a Dynamic (Animation, AnimateBuilder or array of them).
fn anims(d: Dynamic) -> Vec<Animation> {
    if let Some(a) = d.clone().try_cast::<Animation>() {
        return vec![a];
    }
    if let Some(b) = d.clone().try_cast::<AnimateBuilder>() {
        return vec![b.into()];
    }
    if let Some(arr) = d.try_cast::<Array>() {
        return arr.into_iter().flat_map(anims).collect();
    }
    vec![]
}

/// What a script produces.
pub struct ScriptOutput {
    pub scenes: Vec<(Scene, Option<Transition>)>,
    pub quality: Option<String>,
    pub vertical: bool,
}

impl ScriptOutput {
    /// Turn into a single composition (Movie when there are several scenes).
    pub fn composition(self) -> Box<dyn Composition> {
        if self.scenes.len() == 1 {
            return Box::new(self.scenes.into_iter().next().unwrap().0);
        }
        let mut m = Movie::new();
        for (s, t) in self.scenes {
            m = m.then(s);
            if let Some(t) = t {
                m = m.transition(t);
            }
        }
        Box::new(m)
    }
}

struct State {
    done: Vec<(Scene, Option<Transition>)>,
    cur: Scene,
    vertical: bool,
    quality: Option<String>,
    theme: Option<Theme>,
}

type Shared = Arc<Mutex<State>>;

static RUNTIME: OnceLock<(Arc<Engine>, Arc<AST>)> = OnceLock::new();

thread_local! {
    /// Frame context while a render-time callback runs.
    static FRAME: Cell<Option<*const FrameCtx<'static>>> = const { Cell::new(None) };
}

fn with_frame<T>(ctx: &FrameCtx, f: impl FnOnce() -> T) -> T {
    let ptr = (ctx as *const FrameCtx).cast::<FrameCtx<'static>>();
    let prev = FRAME.with(|c| c.replace(Some(ptr)));
    let out = f();
    FRAME.with(|c| c.set(prev));
    out
}

fn frame<T>(f: impl FnOnce(&FrameCtx) -> T) -> Option<T> {
    FRAME.with(|c| c.get()).map(|p| {
        // SAFETY: the pointer is set only for the duration of a callback in `with_frame`,
        // during which the referenced FrameCtx is alive on this thread's stack.
        f(unsafe { &*p })
    })
}

/// Call a script closure from render-time code. Extra trailing arguments are
/// dropped when the closure declares fewer parameters, so `|| ...`, `|t| ...`
/// and `|m, t| ...` all work as callbacks.
fn call_rt<T: Clone + Send + Sync + 'static>(fp: &FnPtr, args: impl rhai::FuncArgs) -> Option<T> {
    let (engine, ast) = RUNTIME.get()?;
    let mut list: Vec<Dynamic> = Vec::new();
    args.parse(&mut list);
    if let Some(n) = ast
        .iter_functions()
        .find(|m| m.name == fp.fn_name())
        .map(|m| m.params.len().saturating_sub(fp.curry().len()))
    {
        list.truncate(n);
    }
    match fp.call::<T>(engine, ast, list) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("script callback error: {e}");
            None
        }
    }
}

/// Wrapper types so scripts can hold builders.
#[derive(Clone)]
pub struct ScriptAxes(pub Axes);

fn register_math(engine: &mut Engine) {
    engine.register_type_with_name::<Vec2>("Vec2");
    engine.register_fn("vec2", |x: N, y: N| v2(f(&x), f(&y)));
    engine.register_fn("p", |x: N, y: N| v2(f(&x), f(&y)));
    engine.register_get("x", |v: &mut Vec2| v.x);
    engine.register_get("y", |v: &mut Vec2| v.y);
    engine.register_fn("+", |a: Vec2, b: Vec2| a + b);
    engine.register_fn("-", |a: Vec2, b: Vec2| a - b);
    engine.register_fn("-", |a: Vec2| -a);
    engine.register_fn("*", |a: Vec2, k: f64| a * k);
    engine.register_fn("*", |k: f64, a: Vec2| a * k);
    engine.register_fn("*", |a: Vec2, k: i64| a * k as f64);
    engine.register_fn("*", |k: i64, a: Vec2| a * k as f64);
    engine.register_fn("/", |a: Vec2, k: f64| a / k);
    engine.register_fn("length", |a: Vec2| a.length());
    engine.register_fn("normalize", |a: Vec2| a.normalize());
    engine.register_fn("rotate", |a: Vec2, t: N| a.rotate(f(&t)));
    engine.register_fn("polar", |r: N, t: N| Vec2::polar(f(&r), f(&t)));
    engine.register_fn("to_string", |v: &mut Vec2| format!("({}, {})", v.x, v.y));
    engine.register_type_with_name::<Vec3>("Vec3");
    engine.register_fn("vec3", |x: N, y: N, z: N| v3(f(&x), f(&y), f(&z)));
    engine.register_fn("+", |a: Vec3, b: Vec3| a + b);
    engine.register_fn("*", |a: Vec3, k: f64| a * k);
    // colours
    engine.register_type_with_name::<Color>("Color");
    engine.register_fn("rgb", |r: N, g: N, b: N| {
        rgb8(f(&r) as u8, f(&g) as u8, f(&b) as u8)
    });
    engine.register_fn("hex", |s: &str| Color::parse(s).unwrap_or(WHITE));
    engine.register_fn("color", |s: &str| Color::parse(s).unwrap_or(WHITE));
    engine.register_fn("hsv", |h: N, s: N, v: N| {
        Color::hsv(f(&h) as f32, f(&s) as f32, f(&v) as f32)
    });
    engine.register_fn("mix", |a: N, b: N, t: N| col(&a).mix(col(&b), f(&t) as f32));
    engine.register_fn("lighter", |a: N, t: N| col(&a).lighter(f(&t) as f32));
    engine.register_fn("darker", |a: N, t: N| col(&a).darker(f(&t) as f32));
    engine.register_fn("with_alpha", |a: N, t: N| col(&a).with_alpha(f(&t) as f32));
    // remotion utilities
    engine.register_fn("interpolate", |x: N, a: Array, b: Array| {
        interpolate(f(&x), &floats(&a), &floats(&b))
    });
    engine.register_fn("interpolate_colors", |x: N, a: Array, b: Array| {
        interpolate_colors(f(&x), &floats(&a), &colors(&b))
    });
    engine.register_fn("spring", |frame: N, fps: N| {
        spring(f(&frame), f(&fps), SpringConfig::default(), 0.0, 1.0)
    });
    engine.register_fn("spring", |frame: N, fps: N, damping: N, stiffness: N| {
        spring(
            f(&frame),
            f(&fps),
            SpringConfig::new(1.0, f(&damping), f(&stiffness)),
            0.0,
            1.0,
        )
    });
    engine.register_fn("ease", |name: &str, t: N| {
        Rate::from_name(name).unwrap_or(Rate::Smooth).eval(f(&t))
    });
    engine.register_fn("noise2d", |seed: &str, x: N, y: N| {
        noise2d(seed, f(&x), f(&y))
    });
    engine.register_fn("noise3d", |seed: &str, x: N, y: N, z: N| {
        noise3d(seed, f(&x), f(&y), f(&z))
    });
    engine.register_fn("random", |seed: &str| random(seed));
    engine.register_fn("lerp", |a: N, b: N, t: N| lerp(f(&a), f(&b), f(&t)));
    engine.register_fn("clamp", |x: N, a: N, b: N| f(&x).clamp(f(&a), f(&b)));
    engine.register_fn("deg", |x: N| f(&x) * DEGREES);
}

fn register_mobject(engine: &mut Engine) {
    engine.register_type_with_name::<Mobject>("Mobject");
    macro_rules! m1 {
        ($name:literal, $f:expr) => {
            engine.register_fn($name, $f);
        };
    }
    m1!("shift", |m: &mut Mobject, v: Vec2| m.clone().shift(v));
    m1!("move_to", |m: &mut Mobject, v: Vec2| m.clone().move_to(v));
    m1!("scale", |m: &mut Mobject, s: N| m.clone().scale(f(&s)));
    m1!("rotate", |m: &mut Mobject, a: N| m.clone().rotate(f(&a)));
    m1!("rotate_about", |m: &mut Mobject, a: N, p: Vec2| m
        .clone()
        .rotate_about(f(&a), p));
    m1!("stretch", |m: &mut Mobject, x: N, y: N| m
        .clone()
        .stretch(f(&x), f(&y)));
    m1!("flip", |m: &mut Mobject, axis: Vec2| m.clone().flip(axis));
    m1!("set_width", |m: &mut Mobject, w: N| m
        .clone()
        .set_width(f(&w)));
    m1!("set_height", |m: &mut Mobject, h: N| m
        .clone()
        .set_height(f(&h)));
    m1!("fit_within", |m: &mut Mobject, w: N, h: N| m
        .clone()
        .fit_within(f(&w), f(&h)));
    m1!("next_to", |m: &mut Mobject, o: Mobject, d: Vec2| m
        .clone()
        .next_to(&o, d, DEFAULT_BUFF));
    m1!("next_to", |m: &mut Mobject, o: Mobject, d: Vec2, b: N| m
        .clone()
        .next_to(&o, d, f(&b)));
    m1!("next_to_point", |m: &mut Mobject,
                          p: Vec2,
                          d: Vec2,
                          b: N| m
        .clone()
        .next_to_point(p, d, f(&b)));
    m1!("align_to", |m: &mut Mobject, o: Mobject, d: Vec2| m
        .clone()
        .align_to(&o, d));
    m1!("to_edge", |m: &mut Mobject, d: Vec2| m
        .clone()
        .to_edge(d, EDGE_BUFF));
    m1!("to_edge", |m: &mut Mobject, d: Vec2, b: N| m
        .clone()
        .to_edge(d, f(&b)));
    m1!("to_corner", |m: &mut Mobject, d: Vec2| m
        .clone()
        .to_corner(d, EDGE_BUFF));
    m1!("center", |m: &mut Mobject| m.clone().center_on_screen());
    m1!("arrange", |m: &mut Mobject, d: Vec2| m
        .clone()
        .arrange(d, DEFAULT_BUFF));
    m1!("arrange", |m: &mut Mobject, d: Vec2, b: N| m
        .clone()
        .arrange(d, f(&b)));
    m1!("arrange_aligned", |m: &mut Mobject,
                            d: Vec2,
                            b: N,
                            e: Vec2| m
        .clone()
        .arrange_aligned(d, f(&b), e));
    m1!("arrange_grid", |m: &mut Mobject, cols: N, hb: N, vb: N| m
        .clone()
        .arrange_in_grid(f(&cols) as usize, f(&hb), f(&vb)));
    m1!("color", |m: &mut Mobject, c: N| m.clone().color(col(&c)));
    m1!("fill", |m: &mut Mobject, c: N, o: N| m
        .clone()
        .fill(col(&c), f(&o) as f32));
    m1!("fill", |m: &mut Mobject, c: N| m.clone().fill(col(&c), 1.0));
    m1!("stroke", |m: &mut Mobject, c: N, w: N| m
        .clone()
        .stroke(col(&c), f(&w) as f32));
    m1!("stroke", |m: &mut Mobject, c: N| m
        .clone()
        .stroke_color(col(&c)));
    m1!("stroke_width", |m: &mut Mobject, w: N| m
        .clone()
        .stroke_width(f(&w) as f32));
    m1!("fill_opacity", |m: &mut Mobject, o: N| m
        .clone()
        .fill_opacity(f(&o) as f32));
    m1!("stroke_opacity", |m: &mut Mobject, o: N| m
        .clone()
        .stroke_opacity(f(&o) as f32));
    m1!("no_stroke", |m: &mut Mobject| m.clone().no_stroke());
    m1!("opacity", |m: &mut Mobject, o: N| m
        .clone()
        .opacity(f(&o) as f32));
    m1!("fade", |m: &mut Mobject, o: N| m.clone().fade(f(&o) as f32));
    m1!("dashed", |m: &mut Mobject| m.clone().dashed(0.12, 0.08));
    m1!("dashed", |m: &mut Mobject, d: N, g: N| m
        .clone()
        .dashed(f(&d), f(&g)));
    m1!("glow", |m: &mut Mobject, r: N| m.clone().glow(f(&r), None));
    m1!("glow", |m: &mut Mobject, r: N, c: N| m
        .clone()
        .glow(f(&r), Some(col(&c))));
    m1!("shadow", |m: &mut Mobject| m
        .clone()
        .shadow(Shadow::default()));
    m1!("sheen", |m: &mut Mobject, c: N| m
        .clone()
        .sheen(col(&c), UL));
    m1!("gradient", |m: &mut Mobject, cs: Array, d: Vec2| m
        .clone()
        .gradient(&colors(&cs), d));
    m1!("color_by_gradient", |m: &mut Mobject, cs: Array| m
        .clone()
        .color_by_gradient(&colors(&cs)));
    m1!("background_stroke", |m: &mut Mobject, c: N, w: N| m
        .clone()
        .background_stroke(col(&c), f(&w) as f32));
    m1!("blend_add", |m: &mut Mobject| m.clone().blend(Blend::Add));
    m1!("tag", |m: &mut Mobject, t: &str| m.clone().with_tag(t));
    m1!("add", |m: &mut Mobject, c: Mobject| m.clone().with_child(c));
    m1!("child", |m: &mut Mobject, i: N| m.child(f(&i) as i64));
    m1!("slice", |m: &mut Mobject, a: N, b: N| m
        .slice(f(&a) as usize, f(&b) as usize));
    m1!("flatten", |m: &mut Mobject| m.flattened());
    m1!("partial", |m: &mut Mobject, a: N, b: N| m
        .clone()
        .partial(f(&a), f(&b)));
    m1!("copy", |m: &mut Mobject| m.clone());
    m1!("len", |m: &mut Mobject| m.children.len() as i64);
    m1!("point_at", |m: &mut Mobject, a: N| m
        .point_from_proportion(f(&a)));
    m1!("dashed_pieces", |m: &mut Mobject, n: N| dashed_mobject(
        m,
        f(&n) as usize,
        0.5
    ));
    engine.register_get("width", |m: &mut Mobject| m.width());
    engine.register_get("height", |m: &mut Mobject| m.height());
    engine.register_get("center", |m: &mut Mobject| m.center());
    engine.register_get("top", |m: &mut Mobject| m.top());
    engine.register_get("bottom", |m: &mut Mobject| m.bottom());
    engine.register_get("left", |m: &mut Mobject| m.left());
    engine.register_get("right", |m: &mut Mobject| m.right());
    engine.register_get("start", |m: &mut Mobject| m.start_point());
    engine.register_get("end", |m: &mut Mobject| m.end_point());
    engine.register_fn("edge", |m: &mut Mobject, d: Vec2| m.edge(d));
    engine.register_fn("group", |a: Array| Mobject::group(mobs(&a)));
    engine.register_fn("empty", Mobject::empty);
    // booleans
    engine.register_fn("union", |a: Mobject, b: Mobject| union(&a, &b));
    engine.register_fn("intersection", |a: Mobject, b: Mobject| {
        intersection(&a, &b)
    });
    engine.register_fn("difference", |a: Mobject, b: Mobject| difference(&a, &b));
    engine.register_fn("exclusion", |a: Mobject, b: Mobject| exclusion(&a, &b));
    engine.register_fn("convex_hull", |a: Array| convex_hull(&points(&a)));
}

fn register_shapes(engine: &mut Engine) {
    macro_rules! r {
        ($name:literal, $f:expr) => {
            engine.register_fn($name, $f);
        };
    }
    r!("circle", |r: N| circle(f(&r)));
    r!("circle_at", |c: Vec2, r: N| circle_at(c, f(&r)));
    r!("dot", |p: Vec2| dot(p));
    r!("dot", |p: Vec2, r: N, c: N| dot_r(p, f(&r), col(&c)));
    r!("ellipse", |w: N, h: N| ellipse(f(&w), f(&h)));
    r!("arc", |r: N, s: N, a: N| arc(f(&r), f(&s), f(&a)));
    r!("arc_between", |a: Vec2, b: Vec2, ang: N| arc_between(
        a,
        b,
        f(&ang)
    ));
    r!("sector", |r: N, s: N, a: N| sector(f(&r), f(&s), f(&a)));
    r!("annulus", |a: N, b: N| annulus(f(&a), f(&b)));
    r!("rectangle", |w: N, h: N| rectangle(f(&w), f(&h)));
    r!("square", |s: N| square(f(&s)));
    r!("rounded_rectangle", |w: N, h: N, r: N| rounded_rectangle(
        f(&w),
        f(&h),
        f(&r)
    ));
    r!("polygon", |a: Array| polygon(&points(&a)));
    r!("polyline", |a: Array| polyline(&points(&a)));
    r!("smooth_curve", |a: Array| smooth_curve(&points(&a), false));
    r!("smooth_curve", |a: Array, closed: bool| smooth_curve(
        &points(&a),
        closed
    ));
    r!("regular_polygon", |n: N, r: N| regular_polygon(
        f(&n) as usize,
        f(&r)
    ));
    r!("triangle", |r: N| triangle(f(&r)));
    r!("star", |n: N, o: N, i: N| star(
        f(&n) as usize,
        f(&o),
        f(&i)
    ));
    r!("line", |a: Vec2, b: Vec2| line(a, b));
    r!("dashed_line", |a: Vec2, b: Vec2| dashed_line(a, b));
    r!("arrow", |a: Vec2, b: Vec2| arrow(a, b));
    r!("double_arrow", |a: Vec2, b: Vec2| double_arrow(a, b));
    r!("vector", |v: Vec2| vector(v));
    r!("curved_arrow", |a: Vec2, b: Vec2, ang: N| curved_arrow(
        a,
        b,
        f(&ang)
    ));
    r!("brace", |m: Mobject, d: Vec2| brace(&m, d, 0.1));
    r!("brace_label", |m: Mobject, t: &str, d: Vec2| brace_label(
        &m, t, d, WHITE
    ));
    r!("brace_text", |m: Mobject, t: &str, d: Vec2| brace_text(
        &m, t, d, WHITE
    ));
    r!("cross", |s: N| cross(f(&s)));
    r!("checkmark", |s: N| checkmark(f(&s)));
    r!("angle_arc", |v: Vec2, a: Vec2, b: Vec2, r: N| angle_arc(
        v,
        a,
        b,
        f(&r)
    ));
    r!(
        "right_angle",
        |v: Vec2, a: Vec2, b: Vec2, s: N| right_angle(v, a, b, f(&s))
    );
    r!("surrounding_rectangle", |m: Mobject| surrounding_rectangle(
        &m, SMALL_BUFF
    ));
    r!("surrounding_rectangle", |m: Mobject, b: N| {
        surrounding_rectangle(&m, f(&b))
    });
    r!("background_rectangle", |m: Mobject| background_rectangle(
        &m, BLACK, 0.75, 0.15
    ));
    r!("underline", |m: Mobject| underline(&m, 0.12));
    r!("grid", |w: N, h: N, s: N| grid(f(&w), f(&h), f(&s)));
    r!("zigzag", |a: Vec2, b: Vec2, n: N, amp: N| zigzag(
        a,
        b,
        f(&n) as usize,
        f(&amp),
        0.2
    ));
    r!("coil", |a: Vec2, b: Vec2, turns: N, r: N| coil(
        a,
        b,
        f(&turns),
        f(&r),
        0.2
    ));
    r!("wave_line", |a: Vec2, b: Vec2, wl: N, amp: N| wave_line(
        a,
        b,
        f(&wl),
        f(&amp),
        0.0
    ));
    r!("heart", |s: N| heart(f(&s)));
    r!("spark", |s: N| spark(f(&s)));
    r!("callout", |w: N, h: N, tail: Vec2| callout(
        f(&w),
        f(&h),
        0.25,
        tail
    ));
    r!("block_arrow", |l: N| block_arrow(
        f(&l),
        f(&l) * 0.45,
        f(&l) * 0.18,
        f(&l) * 0.3
    ));
    r!("pie", |r: N, p: N, c: N| pie(f(&r), f(&p), col(&c)));
    r!("polygram", |n: N, d: N, r: N| regular_polygram(
        f(&n) as usize,
        f(&d) as usize,
        f(&r)
    ));
    r!("glow_dot", |p: Vec2, r: N, c: N| glow_dot(
        p,
        f(&r),
        col(&c)
    ));
    r!("labeled_dot", |t: &str, p: Vec2, c: N| labeled_dot(
        t,
        p,
        col(&c)
    ));
    r!("labeled_line", |a: Vec2, b: Vec2, t: &str, c: N| {
        labeled_line(a, b, t, col(&c))
    });
    r!("labeled_arrow", |a: Vec2, b: Vec2, t: &str, c: N| {
        labeled_arrow(a, b, t, col(&c))
    });
    r!("speech_bubble", |t: &str| speech_bubble(t, 0.4));
    r!("thought_bubble", |t: &str| thought_bubble(t, 0.4));
    r!("clock", |r: N, h: N, m: N| clock(f(&r), f(&h), f(&m)));
    r!("lightbulb", |s: N, on: bool| lightbulb(f(&s), on));
    r!("die_face", |n: N, s: N| die_face(f(&n) as usize, f(&s)));
    r!("piano", |octaves: N, w: N, pressed: Array, c: N| {
        let keys: Vec<usize> = floats(&pressed).into_iter().map(|k| k as usize).collect();
        piano(f(&octaves) as usize, f(&w), &keys, col(&c))
    });
    r!("laptop", |w: N| laptop(f(&w), None));
    r!("laptop", |w: N, screen: Mobject| laptop(
        f(&w),
        Some(screen)
    ));
    r!("speedometer", |r: N, v: N| speedometer(f(&r), f(&v)));
    r!("progress_bar", |w: N, p: N, c: N| progress_bar(
        f(&w),
        f(&p),
        col(&c)
    ));
    r!("starburst", |n: N, rot: N| starburst(
        f(&n) as usize,
        &[],
        f(&rot),
        12.0
    ));
    r!("light_leak", |t: N, seed: &str| light_leak(
        f(&t),
        seed,
        0.7
    ));
    r!("rounded_text_box", |t: &str,
                            size: N,
                            fg: N,
                            bg: N,
                            w: N| {
        rounded_text_box(t, f(&size), col(&fg), col(&bg), f(&w))
    });
    r!("rough", |m: Mobject, kind: &str, c: N| {
        let k = match kind {
            "box" => RoughKind::Box,
            "circle" => RoughKind::Circle,
            "highlight" => RoughKind::Highlight,
            "strike" | "strike-through" | "strike_through" => RoughKind::StrikeThrough,
            "crossed" | "crossed-off" | "crossed_off" => RoughKind::CrossedOff,
            "bracket" => RoughKind::Bracket,
            _ => RoughKind::Underline,
        };
        rough_annotation(&m, k, col(&c), 7)
    });
    r!("mandelbrot", |w: N| mandelbrot(
        v2(-0.6, 0.0),
        3.2,
        800,
        120,
        &[],
        f(&w)
    ));
    r!("julia", |cre: N, cim: N, w: N| julia(
        v2(f(&cre), f(&cim)),
        3.2,
        800,
        160,
        &[],
        f(&w)
    ));
    engine.register_fn("image", |path: &str, w: N| -> R<Mobject> {
        image_mobject(path, f(&w)).map_err(|e| e.to_string().into())
    });
    engine.register_fn("svg", |path: &str, h: N| -> R<Mobject> {
        svg_mobject(path, f(&h)).map_err(|e| e.to_string().into())
    });
    engine.register_fn("svg_str", |s: &str, h: N| -> R<Mobject> {
        svg_from_str(s, f(&h)).map_err(|e| e.to_string().into())
    });
    engine.register_fn("path_d", |d: &str, h: N| path_mobject(d, f(&h)));
}

fn register_text(engine: &mut Engine) {
    engine.register_fn("text", |s: &str| text(s));
    engine.register_fn("text", |s: &str, size: N| text_sized(s, f(&size), WHITE));
    engine.register_fn("text", |s: &str, size: N, c: N| {
        text_sized(s, f(&size), col(&c))
    });
    engine.register_fn("bold_text", |s: &str, size: N, c: N| {
        bold_text(s, f(&size), col(&c))
    });
    engine.register_fn("serif_text", |s: &str, size: N, c: N| {
        serif_text(s, f(&size), col(&c))
    });
    engine.register_fn("markup", |s: &str| Text::markup(s).build());
    engine.register_fn("markup", |s: &str, size: N| {
        Text::markup(s).size(f(&size)).build()
    });
    engine.register_fn("paragraph", |s: &str, w: N, size: N| {
        paragraph(s, f(&w), f(&size), Align::Left)
    });
    engine.register_fn("title", |s: &str| title(s));
    engine.register_fn("bullet_list", |a: Array, size: N| {
        let v = strings(&a);
        let r: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
        bullet_list(&r, f(&size), WHITE)
    });
    engine.register_fn("code", |s: &str, size: N| code_block(s, f(&size)));
    engine.register_fn("tex", |s: &str| tex(s));
    engine.register_fn("tex", |s: &str, size: N| tex_sized(s, f(&size), WHITE));
    engine.register_fn("tex", |s: &str, size: N, c: N| {
        tex_sized(s, f(&size), col(&c))
    });
    engine.register_fn("tex_colored", |s: &str, size: N, m: Map| {
        let mut t = MathTex::new(s).size(f(&size));
        for (k, v) in m {
            t = t.t2c(k.as_str(), col(&v));
        }
        t.build()
    });
    engine.register_fn("tex_parts", |a: Array| {
        let v = strings(&a);
        let r: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
        tex_parts(&r)
    });
    engine.register_fn("tex_text", |s: &str| tex_text(s));
    engine.register_fn("chem", |s: &str| chem(s));
    engine.register_fn("number", |v: N, decimals: N, size: N, c: N| {
        DecimalNumber::new(f(&decimals) as usize)
            .size(f(&size))
            .color(col(&c))
            .build(f(&v))
    });
    engine.register_fn(
        "fill_text_box",
        |t: &str, size: N, w: N, lines: N| -> Array {
            let (a, b) = fill_text_box(t, f(&size), f(&w), f(&lines) as usize);
            vec![a.into(), b.into()]
        },
    );
    engine.register_fn("fit_text", |s: &str, w: N| {
        fit_text(s, f(&w), "Inter", false)
    });
}

fn register_canvas(engine: &mut Engine) {
    use runwee::canvas2d::{Canvas2D, CanvasPaint};
    type C = Canvas2D;
    engine.register_type_with_name::<C>("Canvas2D");
    engine.register_type_with_name::<CanvasPaint>("CanvasPaint");
    // canvas2d(w, h): fills the whole frame; canvas2d(w, h, x0, y0, x1, y1): inside a scene rect.
    engine.register_fn("canvas2d", |w: N, h: N| {
        let (fw, fh) = frame_size();
        Canvas2D::new(f(&w), f(&h), Rect::from_center(ORIGIN, fw, fh))
    });
    engine.register_fn("canvas2d", |w: N, h: N, x0: N, y0: N, x1: N, y1: N| {
        Canvas2D::new(
            f(&w),
            f(&h),
            Rect::new(
                v2(f(&x0).min(f(&x1)), f(&y0).min(f(&y1))),
                v2(f(&x0).max(f(&x1)), f(&y0).max(f(&y1))),
            ),
        )
    });
    engine.register_fn("begin_path", |c: &mut C| c.begin_path());
    engine.register_fn("close_path", |c: &mut C| c.close_path());
    engine.register_fn("move_to", |c: &mut C, x: N, y: N| c.move_to(f(&x), f(&y)));
    engine.register_fn("line_to", |c: &mut C, x: N, y: N| c.line_to(f(&x), f(&y)));
    engine.register_fn("quadratic_curve_to", |c: &mut C, a: N, b: N, x: N, y: N| {
        c.quadratic_curve_to(f(&a), f(&b), f(&x), f(&y))
    });
    engine.register_fn(
        "bezier_curve_to",
        |c: &mut C, a: N, b: N, d: N, e: N, x: N, y: N| {
            c.bezier_curve_to(f(&a), f(&b), f(&d), f(&e), f(&x), f(&y))
        },
    );
    engine.register_fn("arc", |c: &mut C, x: N, y: N, r: N, a0: N, a1: N| {
        c.arc(f(&x), f(&y), f(&r), f(&a0), f(&a1), false)
    });
    engine.register_fn(
        "arc",
        |c: &mut C, x: N, y: N, r: N, a0: N, a1: N, ccw: bool| {
            c.arc(f(&x), f(&y), f(&r), f(&a0), f(&a1), ccw)
        },
    );
    engine.register_fn("arc_to", |c: &mut C, x1: N, y1: N, x2: N, y2: N, r: N| {
        c.arc_to(f(&x1), f(&y1), f(&x2), f(&y2), f(&r))
    });
    engine.register_fn(
        "ellipse",
        |c: &mut C, x: N, y: N, rx: N, ry: N, rot: N, a0: N, a1: N| {
            c.ellipse(f(&x), f(&y), f(&rx), f(&ry), f(&rot), f(&a0), f(&a1), false)
        },
    );
    engine.register_fn("rect", |c: &mut C, x: N, y: N, w: N, h: N| {
        c.rect(f(&x), f(&y), f(&w), f(&h))
    });
    engine.register_fn("round_rect", |c: &mut C, x: N, y: N, w: N, h: N, r: N| {
        c.round_rect(f(&x), f(&y), f(&w), f(&h), &[f(&r)])
    });
    engine.register_fn("fill", |c: &mut C| c.fill());
    engine.register_fn("fill", |c: &mut C, rule: &str| c.fill_rule(rule));
    engine.register_fn("stroke", |c: &mut C| c.stroke());
    engine.register_fn("clip", |c: &mut C| c.clip());
    engine.register_fn("fill_rect", |c: &mut C, x: N, y: N, w: N, h: N| {
        c.fill_rect(f(&x), f(&y), f(&w), f(&h))
    });
    engine.register_fn("stroke_rect", |c: &mut C, x: N, y: N, w: N, h: N| {
        c.stroke_rect(f(&x), f(&y), f(&w), f(&h))
    });
    engine.register_fn("clear_rect", |c: &mut C, x: N, y: N, w: N, h: N| {
        c.clear_rect(f(&x), f(&y), f(&w), f(&h))
    });
    engine.register_fn("fill_text", |c: &mut C, t: &str, x: N, y: N| {
        c.fill_text(t, f(&x), f(&y))
    });
    engine.register_fn("stroke_text", |c: &mut C, t: &str, x: N, y: N| {
        c.stroke_text(t, f(&x), f(&y))
    });
    engine.register_fn("measure_text", |c: &mut C, t: &str| c.measure_text(t));
    engine.register_fn("save", |c: &mut C| c.save());
    engine.register_fn("restore", |c: &mut C| c.restore());
    engine.register_fn("translate", |c: &mut C, x: N, y: N| {
        c.translate(f(&x), f(&y))
    });
    engine.register_fn("rotate", |c: &mut C, a: N| c.rotate(f(&a)));
    engine.register_fn("scale", |c: &mut C, x: N, y: N| c.scale(f(&x), f(&y)));
    engine.register_fn(
        "set_transform",
        |c: &mut C, a: N, b: N, cc: N, d: N, e: N, ff: N| {
            c.set_transform(f(&a), f(&b), f(&cc), f(&d), f(&e), f(&ff))
        },
    );
    engine.register_fn("reset_transform", |c: &mut C| c.reset_transform());
    engine.register_fn("set_line_dash", |c: &mut C, a: Array| {
        c.set_line_dash(&floats(&a))
    });
    engine.register_set("fill_style", |c: &mut C, v: N| {
        if let Some(p) = v.clone().try_cast::<CanvasPaint>() {
            c.set_fill_style(p)
        } else {
            c.set_fill_style(CanvasPaint::Color(col(&v)))
        }
    });
    engine.register_set("stroke_style", |c: &mut C, v: N| {
        if let Some(p) = v.clone().try_cast::<CanvasPaint>() {
            c.set_stroke_style(p)
        } else {
            c.set_stroke_style(CanvasPaint::Color(col(&v)))
        }
    });
    engine.register_set("line_width", |c: &mut C, v: N| c.set_line_width(f(&v)));
    engine.register_set("line_cap", |c: &mut C, v: &str| c.set_line_cap(v));
    engine.register_set("line_join", |c: &mut C, v: &str| c.set_line_join(v));
    engine.register_set("line_dash_offset", |c: &mut C, v: N| {
        c.set_line_dash_offset(f(&v))
    });
    engine.register_set("global_alpha", |c: &mut C, v: N| c.set_global_alpha(f(&v)));
    engine.register_set("global_composite_operation", |c: &mut C, v: &str| {
        c.set_global_composite_operation(v)
    });
    engine.register_set("font", |c: &mut C, v: &str| c.set_font(v));
    engine.register_set("text_align", |c: &mut C, v: &str| c.set_text_align(v));
    engine.register_set("text_baseline", |c: &mut C, v: &str| c.set_text_baseline(v));
    engine.register_set("shadow_color", |c: &mut C, v: &str| {
        c.set_shadow_color_css(v)
    });
    engine.register_set("shadow_blur", |c: &mut C, v: N| c.set_shadow_blur(f(&v)));
    // Gradients: stops = [[offset, colour], ...]
    let stops = |a: &Array| -> Vec<(f32, Color)> {
        a.iter()
            .filter_map(|d| d.clone().try_cast::<Array>())
            .filter(|p| p.len() >= 2)
            .map(|p| (f(&p[0]) as f32, col(&p[1])))
            .collect()
    };
    engine.register_fn(
        "linear_gradient",
        move |x0: N, y0: N, x1: N, y1: N, s: Array| CanvasPaint::Linear {
            p0: v2(f(&x0), f(&y0)),
            p1: v2(f(&x1), f(&y1)),
            stops: stops(&s),
        },
    );
    engine.register_fn(
        "radial_gradient",
        move |x0: N, y0: N, r0: N, x1: N, y1: N, r1: N, s: Array| CanvasPaint::Radial {
            c0: v2(f(&x0), f(&y0)),
            r0: f(&r0),
            c1: v2(f(&x1), f(&y1)),
            r1: f(&r1),
            stops: stops(&s),
        },
    );
    engine.register_fn("finish", |c: &mut C| c.take());
}

fn register_graph(engine: &mut Engine) {
    engine.register_type_with_name::<ScriptAxes>("Axes");
    engine.register_fn("axes", |x: Array, y: Array| {
        ScriptAxes(Axes::new(range3(&x), range3(&y)))
    });
    engine.register_fn("size", |a: &mut ScriptAxes, w: N, h: N| {
        ScriptAxes(a.0.clone().size(f(&w), f(&h)))
    });
    engine.register_fn("at", |a: &mut ScriptAxes, p: Vec2| {
        ScriptAxes(a.0.clone().at(p))
    });
    engine.register_fn("labels", |a: &mut ScriptAxes, x: &str, y: &str| {
        ScriptAxes(a.0.clone().labels(x, y))
    });
    engine.register_fn("numbers", |a: &mut ScriptAxes, b: bool| {
        ScriptAxes(a.0.clone().numbers(b))
    });
    engine.register_fn("tips", |a: &mut ScriptAxes, b: bool| {
        ScriptAxes(a.0.clone().tips(b))
    });
    engine.register_fn("build", |a: &mut ScriptAxes| a.0.build());
    engine.register_fn("c2p", |a: &mut ScriptAxes, x: N, y: N| {
        a.0.c2p(f(&x), f(&y))
    });
    engine.register_fn("p2c", |a: &mut ScriptAxes, p: Vec2| a.0.p2c(p));
    engine.register_fn("grid", |a: &mut ScriptAxes| a.0.grid(BLUE_D, 0.5));
    engine.register_fn(
        "plot",
        |ctx: NativeCallContext, a: &mut ScriptAxes, fp: FnPtr| {
            a.0.plot(|x| {
                fp.call_within_context::<f64>(&ctx, (x,))
                    .unwrap_or(f64::NAN)
            })
        },
    );
    engine.register_fn(
        "plot",
        |ctx: NativeCallContext, a: &mut ScriptAxes, fp: FnPtr, x0: N, x1: N| {
            a.0.plot_range(
                |x| {
                    fp.call_within_context::<f64>(&ctx, (x,))
                        .unwrap_or(f64::NAN)
                },
                f(&x0),
                f(&x1),
            )
        },
    );
    engine.register_fn(
        "plot_parametric",
        |ctx: NativeCallContext, a: &mut ScriptAxes, fp: FnPtr, t0: N, t1: N| {
            a.0.plot_parametric(
                |t| {
                    fp.call_within_context::<Vec2>(&ctx, (t,))
                        .unwrap_or_default()
                },
                f(&t0),
                f(&t1),
            )
        },
    );
    engine.register_fn(
        "plot_implicit",
        |ctx: NativeCallContext, a: &mut ScriptAxes, fp: FnPtr| {
            a.0.plot_implicit(
                |x, y| {
                    fp.call_within_context::<f64>(&ctx, (x, y))
                        .unwrap_or(f64::NAN)
                },
                120,
            )
        },
    );
    engine.register_fn(
        "area",
        |ctx: NativeCallContext, a: &mut ScriptAxes, fp: FnPtr, x0: N, x1: N, c: N| {
            a.0.area(
                |x| fp.call_within_context::<f64>(&ctx, (x,)).unwrap_or(0.0),
                f(&x0),
                f(&x1),
                None,
                col(&c),
                0.45,
            )
        },
    );
    engine.register_fn(
        "riemann",
        |ctx: NativeCallContext, a: &mut ScriptAxes, fp: FnPtr, x0: N, x1: N, n: N| {
            a.0.riemann(
                |x| fp.call_within_context::<f64>(&ctx, (x,)).unwrap_or(0.0),
                f(&x0),
                f(&x1),
                f(&n) as usize,
                "left",
                &[],
            )
        },
    );
    engine.register_fn(
        "tangent",
        |ctx: NativeCallContext, a: &mut ScriptAxes, fp: FnPtr, x: N, len: N| {
            a.0.tangent_line(
                |x| fp.call_within_context::<f64>(&ctx, (x,)).unwrap_or(0.0),
                f(&x),
                f(&len),
            )
        },
    );
    engine.register_fn("scatter", |a: &mut ScriptAxes, pts: Array, c: N| {
        a.0.scatter(&points(&pts), col(&c), 0.07)
    });
    engine.register_fn("line_graph", |a: &mut ScriptAxes, pts: Array, c: N| {
        a.0.line_graph(&points(&pts), col(&c), true)
    });
    engine.register_fn("number_line", |a: N, b: N, s: N, len: N| {
        NumberLine::new(f(&a), f(&b), f(&s))
            .length(f(&len))
            .numbers(true)
            .build()
    });
    engine.register_fn("number_plane", default_plane);
    engine.register_fn("number_plane", |x: Array, y: Array| {
        number_plane(range3(&x), range3(&y))
    });
    engine.register_fn("complex_plane", |r: N| complex_plane(f(&r)));
    engine.register_fn("polar_plane", |r: N| polar_plane(f(&r), 4, 12));
    engine.register_fn(
        "vector_field",
        |ctx: NativeCallContext, fp: FnPtr, x: Array, y: Array, k: N| {
            vector_field(
                |p| {
                    fp.call_within_context::<Vec2>(&ctx, (p,))
                        .unwrap_or_default()
                },
                range3(&x),
                range3(&y),
                f(&k),
                &[],
            )
        },
    );
    engine.register_fn(
        "stream_lines",
        |ctx: NativeCallContext, fp: FnPtr, x: Array, y: Array| {
            stream_lines(
                |p| {
                    fp.call_within_context::<Vec2>(&ctx, (p,))
                        .unwrap_or_default()
                },
                range3(&x),
                range3(&y),
                &[],
            )
        },
    );
    engine.register_fn("bar_chart", |v: Array, l: Array, w: N, h: N| {
        let ls = strings(&l);
        let r: Vec<&str> = ls.iter().map(|s| s.as_str()).collect();
        bar_chart(&floats(&v), &r, &[], f(&w), f(&h))
    });
    engine.register_fn("pie_chart", |v: Array, l: Array, r: N| {
        let ls = strings(&l);
        let rr: Vec<&str> = ls.iter().map(|s| s.as_str()).collect();
        pie_chart(&floats(&v), &rr, &[], f(&r))
    });
    fn rows_of(a: &Array) -> Vec<Vec<String>> {
        a.iter()
            .map(|r| {
                r.clone()
                    .try_cast::<Array>()
                    .map(|x| strings(&x))
                    .unwrap_or_default()
            })
            .collect()
    }
    engine.register_fn("table", |a: Array, size: N| {
        let rows = rows_of(&a);
        let r: Vec<Vec<&str>> = rows
            .iter()
            .map(|x| x.iter().map(|s| s.as_str()).collect())
            .collect();
        table(&r, true, f(&size))
    });
    engine.register_fn("math_table", |a: Array, size: N| {
        let rows = rows_of(&a);
        let r: Vec<Vec<&str>> = rows
            .iter()
            .map(|x| x.iter().map(|s| s.as_str()).collect())
            .collect();
        math_table(&r, true, f(&size))
    });
    engine.register_fn("matrix", |a: Array, size: N| {
        let rows = rows_of(&a);
        let r: Vec<Vec<&str>> = rows
            .iter()
            .map(|x| x.iter().map(|s| s.as_str()).collect())
            .collect();
        matrix(&r, f(&size))
    });
    engine.register_fn(
        "network",
        |labels: Array, edges: Array, layout: &str, directed: bool| {
            let ls = strings(&labels);
            let l: Vec<&str> = ls.iter().map(|s| s.as_str()).collect();
            let e: Vec<(usize, usize)> = edges
                .iter()
                .filter_map(|x| x.clone().try_cast::<Array>())
                .map(|p| (f(&p[0]) as usize, f(&p[1]) as usize))
                .collect();
            let lay = match layout {
                "spring" => Layout::Spring,
                "tree" => Layout::Tree,
                "grid" => Layout::Grid,
                _ => Layout::Circular,
            };
            network(&l, &e, lay, directed, 2.0)
        },
    );
}

fn register_anims(engine: &mut Engine) {
    engine.register_type_with_name::<Obj>("Obj");
    engine.register_type_with_name::<Tracker>("Tracker");
    engine.register_type_with_name::<Animation>("Animation");
    engine.register_type_with_name::<AnimateBuilder>("AnimateBuilder");
    engine.register_fn("run_time", |a: &mut Animation, t: N| {
        a.clone().run_time(f(&t))
    });
    engine.register_fn("rate", |a: &mut Animation, r: &str| {
        a.clone().rate(Rate::from_name(r).unwrap_or(Rate::Smooth))
    });
    engine.register_fn("lag", |a: &mut Animation, l: N| a.clone().lag(f(&l)));
    engine.register_fn("delay", |a: &mut Animation, d: N| a.clone().delay(f(&d)));
    engine.register_fn("to", |k: &mut Tracker, v: N| k.to(f(&v)));
    // .animate()
    engine.register_fn("animate", |o: &mut Obj| o.animate());
    macro_rules! ab {
        ($name:literal, $f:expr) => {
            engine.register_fn($name, $f);
        };
    }
    ab!("shift", |b: &mut AnimateBuilder, v: Vec2| b
        .clone()
        .shift(v));
    ab!("move_to", |b: &mut AnimateBuilder, v: Vec2| b
        .clone()
        .move_to(v));
    ab!("scale", |b: &mut AnimateBuilder, s: N| b
        .clone()
        .scale(f(&s)));
    ab!("rotate", |b: &mut AnimateBuilder, a: N| b
        .clone()
        .rotate(f(&a)));
    ab!("stretch", |b: &mut AnimateBuilder, x: N, y: N| b
        .clone()
        .stretch(f(&x), f(&y)));
    ab!("set_width", |b: &mut AnimateBuilder, w: N| b
        .clone()
        .set_width(f(&w)));
    ab!("set_height", |b: &mut AnimateBuilder, h: N| b
        .clone()
        .set_height(f(&h)));
    ab!("next_to", |b: &mut AnimateBuilder,
                    m: Mobject,
                    d: Vec2,
                    buff: N| b.clone().next_to(
        &m,
        d,
        f(&buff)
    ));
    ab!("to_edge", |b: &mut AnimateBuilder, d: Vec2| b
        .clone()
        .to_edge(d, EDGE_BUFF));
    ab!("color", |b: &mut AnimateBuilder, c: N| b
        .clone()
        .color(col(&c)));
    ab!("fill", |b: &mut AnimateBuilder, c: N, o: N| b
        .clone()
        .fill(col(&c), f(&o) as f32));
    ab!("stroke", |b: &mut AnimateBuilder, c: N, w: N| b
        .clone()
        .stroke(col(&c), f(&w) as f32));
    ab!("opacity", |b: &mut AnimateBuilder, o: N| b
        .clone()
        .opacity(f(&o) as f32));
    ab!("glow", |b: &mut AnimateBuilder, r: N| b
        .clone()
        .glow(f(&r), None));
    ab!("run_time", |b: &mut AnimateBuilder, t: N| b
        .clone()
        .run_time(f(&t)));
    ab!("rate", |b: &mut AnimateBuilder, r: &str| b
        .clone()
        .rate(Rate::from_name(r).unwrap_or(Rate::Smooth)));
    // animation constructors
    macro_rules! a1 {
        ($name:literal, $f:expr) => {
            engine.register_fn($name, |o: Obj| $f(o));
        };
    }
    a1!("create", create);
    a1!("uncreate", uncreate);
    a1!("write", write);
    a1!("unwrite", unwrite);
    a1!("draw_border_then_fill", draw_border_then_fill);
    a1!("fade_in", fade_in);
    a1!("fade_out", fade_out);
    a1!("grow_from_center", grow_from_center);
    a1!("grow_arrow", grow_arrow);
    a1!("spin_in", spin_in);
    a1!("shrink_to_center", shrink_to_center);
    a1!("indicate", indicate);
    a1!("wiggle", wiggle);
    a1!("apply_wave", apply_wave);
    a1!("pulse", pulse);
    a1!("blink", blink);
    a1!("typewriter", typewriter);
    a1!("spiral_in", spiral_in);
    a1!("turn_inside_out", turn_inside_out);
    a1!("add_text_word_by_word", add_text_word_by_word);
    a1!("remove_text_letter_by_letter", remove_text_letter_by_letter);
    a1!(
        "show_creation_then_destruction",
        show_creation_then_destruction
    );
    a1!("show_creation_then_fade_out", show_creation_then_fade_out);
    a1!("show_increasing_subsets", show_increasing_subsets);
    a1!("show_submobjects_one_by_one", show_submobjects_one_by_one);
    a1!("fade_in_then_out", fade_in_then_out);
    engine.register_fn("fade_in", |o: Obj, v: Vec2| fade_in_shift(o, v));
    engine.register_fn("fade_out", |o: Obj, v: Vec2| fade_out_shift(o, v));
    engine.register_fn("fade_in_scale", |o: Obj, s: N| fade_in_scale(o, f(&s)));
    engine.register_fn("fade_out_scale", |o: Obj, s: N| fade_out_scale(o, f(&s)));
    engine.register_fn("fade_in_from_point", |o: Obj, p: Vec2| {
        fade_in_from_point(o, p)
    });
    engine.register_fn("fade_out_to_point", |o: Obj, p: Vec2| {
        fade_out_to_point(o, p)
    });
    engine.register_fn("grow_from_point", |o: Obj, p: Vec2| grow_from_point(o, p));
    engine.register_fn("grow_from_edge", |o: Obj, d: Vec2| grow_from_edge(o, d));
    engine.register_fn("transform", |o: Obj, m: Mobject| transform(o, m));
    engine.register_fn("transform", |o: Obj, m: Mobject, arc: N| {
        transform_arc(o, m, f(&arc))
    });
    engine.register_fn("replacement_transform", |a: Obj, b: Obj| {
        replacement_transform(a, b)
    });
    engine.register_fn("transform_from_copy", |a: Obj, b: Obj| {
        transform_from_copy(a, b)
    });
    engine.register_fn("transform_matching_from_copy", |a: Obj, b: Obj| {
        transform_matching_from_copy(a, b)
    });
    engine.register_fn("fade_transform", |o: Obj, m: Mobject| fade_transform(o, m));
    engine.register_fn("transform_matching", |o: Obj, m: Mobject| {
        transform_matching(o, m)
    });
    engine.register_fn("transform_matching_shapes", |o: Obj, m: Mobject| {
        transform_matching_shapes(o, m)
    });
    engine.register_fn("rotate", |o: Obj, a: N| rotate(o, f(&a)));
    engine.register_fn("rotate", |o: Obj, a: N, about: Vec2| {
        rotate_about(o, f(&a), about)
    });
    engine.register_fn("rotating", |o: Obj, a: N| rotating(o, f(&a), None));
    engine.register_fn("move_along_path", |o: Obj, m: Mobject| {
        move_along_path(o, &m)
    });
    engine.register_fn("apply_matrix", |o: Obj, m: Array| {
        let v = floats(&m);
        apply_matrix(o, [[v[0], v[1]], [v[2], v[3]]])
    });
    engine.register_fn("apply_function", |o: Obj, fp: FnPtr| {
        apply_function(o, move |p| call_rt::<Vec2>(&fp, (p,)).unwrap_or(p))
    });
    engine.register_fn("homotopy", |o: Obj, fp: FnPtr| {
        homotopy(o, move |p, t| call_rt::<Vec2>(&fp, (p, t)).unwrap_or(p))
    });
    engine.register_fn("phase_flow", |o: Obj, fp: FnPtr, t: N| {
        phase_flow(
            o,
            move |p| call_rt::<Vec2>(&fp, (p,)).unwrap_or_default(),
            f(&t),
        )
    });
    engine.register_fn("indicate", |o: Obj, c: N| indicate_with(o, col(&c), 1.2));
    engine.register_fn("flash", |p: Vec2, c: N| flash(p, col(&c)));
    engine.register_fn("circumscribe", |o: Obj, c: N| circumscribe(o, col(&c)));
    engine.register_fn("circumscribe_circle", |o: Obj, c: N| {
        circumscribe_circle(o, col(&c))
    });
    engine.register_fn("focus_on", |p: Vec2| focus_on(p));
    engine.register_fn("highlight", |o: Obj, c: N| highlight(o, col(&c)));
    engine.register_fn("show_passing_flash", |o: Obj, w: N| {
        show_passing_flash(o, f(&w))
    });
    engine.register_fn("type_with_cursor", |o: Obj| type_with_cursor(o, WHITE));
    engine.register_fn("untype_with_cursor", |o: Obj| untype_with_cursor(o, WHITE));
    engine.register_fn("sweep_highlight", |o: Obj, c: N| {
        sweep_highlight(o, col(&c))
    });
    engine.register_fn("together", |a: Array| {
        together(a.into_iter().flat_map(anims).collect())
    });
    engine.register_fn("lagged_start", |a: Array, l: N| {
        lagged_start(a.into_iter().flat_map(anims).collect(), f(&l))
    });
    engine.register_fn("succession", |a: Array| {
        succession(a.into_iter().flat_map(anims).collect())
    });
    engine.register_fn("wait_anim", |d: N| Animation::wait(f(&d)));
    engine.register_fn("zoom_to", |c: Vec2, h: N| zoom_to(c, f(&h)));
    engine.register_fn("pan_to", |c: Vec2| pan_to(c));
    engine.register_fn("camera_reset", camera_reset);
}

fn register_scene(engine: &mut Engine, st: Shared) {
    macro_rules! s {
        ($st:ident, $body:expr) => {{
            let mut g = $st.lock().unwrap();
            $body(&mut g.cur)
        }};
    }
    let st1 = st.clone();
    engine.register_fn("add", move |m: Mobject| s!(st1, |sc: &mut Scene| sc.add(m)));
    let st1 = st.clone();
    engine.register_fn("register", move |m: Mobject| {
        s!(st1, |sc: &mut Scene| sc.register(m))
    });
    let st1 = st.clone();
    engine.register_fn("add_fixed", move |m: Mobject| {
        s!(st1, |sc: &mut Scene| sc.add_fixed(m))
    });
    let st1 = st.clone();
    engine.register_fn("remove", move |o: Obj| {
        s!(st1, |sc: &mut Scene| sc.remove(o))
    });
    let st1 = st.clone();
    engine.register_fn("show", move |o: Obj| {
        s!(st1, |sc: &mut Scene| sc.show_now(o))
    });
    let st1 = st.clone();
    engine.register_fn("play", move |a: Dynamic| {
        s!(st1, |sc: &mut Scene| sc.play(anims(a)))
    });
    let st1 = st.clone();
    engine.register_fn("play", move |a: Dynamic, t: N| {
        s!(st1, |sc: &mut Scene| sc.play_for(f(&t), anims(a)))
    });
    let st1 = st.clone();
    engine.register_fn("play_bg", move |a: Dynamic, t: N| {
        s!(st1, |sc: &mut Scene| sc.play_background(f(&t), anims(a)))
    });
    let st1 = st.clone();
    engine.register_fn("wait", move |d: N| s!(st1, |sc: &mut Scene| sc.wait(f(&d))));
    let st1 = st.clone();
    engine.register_fn("wait", move || s!(st1, |sc: &mut Scene| sc.wait(1.0)));
    let st1 = st.clone();
    engine.register_fn("wait_until", move |t: N| {
        s!(st1, |sc: &mut Scene| sc.wait_until(f(&t)))
    });
    let st1 = st.clone();
    engine.register_fn("time", move || {
        frame(|c| c.t).unwrap_or_else(|| s!(st1, |sc: &mut Scene| sc.time()))
    });
    let st1 = st.clone();
    engine.register_fn("get", move |o: Obj| {
        frame(|c| c.get(o)).unwrap_or_else(|| s!(st1, |sc: &mut Scene| sc.get(o)))
    });
    let st1 = st.clone();
    engine.register_fn("set", move |o: Obj, m: Mobject| {
        s!(st1, |sc: &mut Scene| sc.set(o, m))
    });
    let st1 = st.clone();
    engine.register_fn("tracker", move |v: N| {
        s!(st1, |sc: &mut Scene| sc.tracker(f(&v)))
    });
    let st1 = st.clone();
    engine.register_fn("value", move |k: Tracker| {
        frame(|c| c.value(k)).unwrap_or_else(|| s!(st1, |sc: &mut Scene| sc.value(k)))
    });
    let st1 = st.clone();
    engine.register_fn("set_value", move |k: Tracker, v: N| {
        s!(st1, |sc: &mut Scene| sc.set_value(k, f(&v)))
    });
    let st1 = st.clone();
    engine.register_fn("counter", move |k: Tracker, decimals: N, at: Vec2, c: N| {
        let d = DecimalNumber::new(f(&decimals) as usize).color(col(&c));
        s!(st1, |sc: &mut Scene| sc.counter(k, d, at))
    });
    let st1 = st.clone();
    engine.register_fn(
        "counter",
        move |k: Tracker, decimals: N, at: Vec2, c: N, prefix: &str, suffix: &str| {
            let d = DecimalNumber::new(f(&decimals) as usize)
                .color(col(&c))
                .prefix(prefix)
                .unit(suffix);
            s!(st1, |sc: &mut Scene| sc.counter(k, d, at))
        },
    );
    let st1 = st.clone();
    engine.register_fn("always_redraw", move |fp: FnPtr| {
        s!(st1, |sc: &mut Scene| sc.always_redraw(move |ctx| {
            with_frame(ctx, || call_rt::<Mobject>(&fp, (ctx.elapsed,))).unwrap_or_default()
        }))
    });
    let st1 = st.clone();
    engine.register_fn("add_updater", move |o: Obj, fp: FnPtr| {
        s!(st1, |sc: &mut Scene| sc.add_updater(o, move |m, ctx| {
            let fallback = m.clone();
            with_frame(ctx, || call_rt::<Mobject>(&fp, (m, ctx.elapsed))).unwrap_or(fallback)
        }))
    });
    let st1 = st.clone();
    engine.register_fn("clear_updaters", move |o: Obj| {
        s!(st1, |sc: &mut Scene| sc.clear_updaters(o))
    });
    let st1 = st.clone();
    engine.register_fn("bring_to_front", move |o: Obj| {
        s!(st1, |sc: &mut Scene| sc.bring_to_front(o))
    });
    let st1 = st.clone();
    engine.register_fn("bring_to_back", move |o: Obj| {
        s!(st1, |sc: &mut Scene| sc.bring_to_back(o))
    });
    let st1 = st.clone();
    engine.register_fn("set_z", move |o: Obj, z: N| {
        s!(st1, |sc: &mut Scene| sc.set_z(o, f(&z)))
    });
    let st1 = st.clone();
    // Backgrounds & themes. A string is a preset name or a colour; a colour value works too.
    engine.register_fn("background", move |c: N| -> R<()> {
        let bg = if let Some(name) = c.clone().try_cast::<rhai::ImmutableString>() {
            Background::preset(&name).ok_or_else(|| {
                format!(
                    "unknown background '{name}' (presets: {})",
                    BACKGROUND_PRESETS.join(", ")
                )
            })?
        } else {
            Background::Solid(col(&c))
        };
        s!(st1, |sc: &mut Scene| sc.set_background(bg));
        Ok(())
    });
    let st1 = st.clone();
    engine.register_fn("background_fade", move |name: &str, dur: N| -> R<()> {
        let bg = Background::preset(name).ok_or_else(|| format!("unknown background '{name}'"))?;
        s!(st1, |sc: &mut Scene| sc.background_fade(bg, f(&dur)));
        Ok(())
    });
    let st1 = st.clone();
    engine.register_fn("background_gradient", move |a: N, b: N, angle: N| {
        let bg = Background::linear(col(&a), col(&b), f(&angle));
        s!(st1, |sc: &mut Scene| sc.set_background(bg))
    });
    let st1 = st.clone();
    engine.register_fn("background_gradient", move |list: Array, angle: N| {
        let cs = colors(&list);
        let n = cs.len().max(2) - 1;
        let stops = cs
            .iter()
            .enumerate()
            .map(|(i, c)| (i as f32 / n as f32, *c))
            .collect();
        s!(st1, |sc: &mut Scene| sc.set_background(
            Background::Linear {
                angle: f(&angle),
                stops
            }
        ))
    });
    let st1 = st.clone();
    engine.register_fn("background_radial", move |inner: N, outer: N| {
        let bg = Background::radial(col(&inner), col(&outer));
        s!(st1, |sc: &mut Scene| sc.set_background(bg))
    });
    let st1 = st.clone();
    engine.register_fn("background_corners", move |a: N, b: N, c: N, d: N| {
        let bg = Background::Corners([col(&a), col(&b), col(&c), col(&d)]);
        s!(st1, |sc: &mut Scene| sc.set_background(bg))
    });
    let st1 = st.clone();
    engine.register_fn("background_image", move |path: &str, dim: N| -> R<()> {
        let bg = Background::image(path)
            .map_err(|e| e.to_string())?
            .dim(f(&dim) as f32);
        s!(st1, |sc: &mut Scene| sc.set_background(bg));
        Ok(())
    });
    let st1 = st.clone();
    engine.register_fn("background_video", move |path: &str, dim: N| -> R<()> {
        let bg = Background::video(path)
            .map_err(|e| e.to_string())?
            .dim(f(&dim) as f32);
        s!(st1, |sc: &mut Scene| sc.set_background(bg));
        Ok(())
    });
    let st1 = st.clone();
    engine.register_fn(
        "background_pattern",
        move |kind: &str, base: N, ink: N, spacing: N| -> R<()> {
            let k = match kind.to_ascii_lowercase().replace('_', "-").as_str() {
                "grid" => Pattern::Grid,
                "graph-paper" | "graph" => Pattern::GraphPaper,
                "dots" => Pattern::Dots,
                "ruled" | "lines" => Pattern::Ruled,
                "notebook" => Pattern::Notebook,
                "checker" | "checkerboard" => Pattern::Checker,
                "stripes" => Pattern::Stripes,
                "crosses" | "plus" => Pattern::Crosses,
                "isometric" => Pattern::Isometric,
                _ => return Err(format!("unknown pattern '{kind}'").into()),
            };
            let bg = match Background::pattern(k, col(&base), col(&ink)) {
                Background::Pattern {
                    kind,
                    base,
                    ink,
                    width,
                    drift,
                    ..
                } => Background::Pattern {
                    kind,
                    base,
                    ink,
                    spacing: f(&spacing),
                    width,
                    drift,
                },
                other => other,
            };
            s!(st1, |sc: &mut Scene| sc.set_background(bg));
            Ok(())
        },
    );
    let st1 = st.clone();
    engine.register_fn("theme", move |name: &str| -> R<()> {
        let th = Theme::preset(name).ok_or_else(|| {
            format!(
                "unknown theme '{name}' (any background preset works: {})",
                BACKGROUND_PRESETS.join(", ")
            )
        })?;
        let mut g = st1.lock().unwrap();
        g.theme = Some(th.clone());
        g.cur.theme(th);
        Ok(())
    });
    engine.register_fn("backgrounds", || -> Array {
        BACKGROUND_PRESETS
            .iter()
            .map(|s| Dynamic::from(s.to_string()))
            .collect()
    });
    let st1 = st.clone();
    engine.register_fn("vertical", move || {
        let mut g = st1.lock().unwrap();
        g.vertical = true;
        g.cur = Scene::vertical();
        if let Some(th) = g.theme.clone() {
            g.cur.theme(th);
        }
    });
    let st1 = st.clone();
    engine.register_fn("quality", move |q: &str| {
        st1.lock().unwrap().quality = Some(q.to_string())
    });
    let st1 = st.clone();
    engine.register_fn("marker", move |n: &str| {
        s!(st1, |sc: &mut Scene| sc.marker(n))
    });
    let st1 = st.clone();
    engine.register_fn("save_state", move |o: Obj| {
        s!(st1, |sc: &mut Scene| sc.save_state(o))
    });
    let st1 = st.clone();
    engine.register_fn("restore", move |o: Obj| {
        s!(st1, |sc: &mut Scene| sc.restore(o))
    });
    let st1 = st.clone();
    engine.register_fn("swap", move |a: Obj, b: Obj| {
        s!(st1, |sc: &mut Scene| sc.swap(a, b))
    });
    let st1 = st.clone();
    engine.register_fn("cyclic_replace", move |a: Array| {
        let objs: Vec<Obj> = a.into_iter().filter_map(|d| d.try_cast::<Obj>()).collect();
        s!(st1, |sc: &mut Scene| sc.cyclic_replace(&objs))
    });
    let st1 = st.clone();
    engine.register_fn("broadcast", move |p: Vec2, c: N| {
        s!(st1, |sc: &mut Scene| sc.broadcast(p, col(&c), 4, 2.5))
    });
    let st1 = st.clone();
    engine.register_fn("traced_path", move |o: Obj, c: N, w: N| {
        s!(st1, |sc: &mut Scene| sc.traced_path(
            o,
            col(&c),
            f(&w) as f32,
            None
        ))
    });
    let st1 = st.clone();
    engine.register_fn("traced_path", move |o: Obj, c: N, w: N, fade: N| {
        s!(st1, |sc: &mut Scene| sc.traced_path(
            o,
            col(&c),
            f(&w) as f32,
            Some(f(&fade))
        ))
    });
    let st1 = st.clone();
    engine.register_fn("animated_boundary", move |o: Obj| {
        s!(st1, |sc: &mut Scene| sc.animated_boundary(
            o,
            vec![BLUE, PURPLE, PINK, YELLOW],
            2.0
        ))
    });
    let st1 = st.clone();
    engine.register_fn("maintain_position", move |follower: Obj, leader: Obj| {
        s!(st1, |sc: &mut Scene| sc
            .maintain_position_relative_to(follower, leader))
    });
    let st1 = st.clone();
    engine.register_fn("derivation", move |lines: Array, at: Vec2, size: N| {
        let ls = strings(&lines);
        let r: Vec<&str> = ls.iter().map(|s| s.as_str()).collect();
        s!(st1, |sc: &mut Scene| sc.derivation(&r, at, f(&size)));
    });
    let st1 = st.clone();
    engine.register_fn("quiz", move |q: &str, opts: Array, correct: N, think: N| {
        let os = strings(&opts);
        let r: Vec<&str> = os.iter().map(|s| s.as_str()).collect();
        s!(st1, |sc: &mut Scene| {
            sc.quiz(q, &r, f(&correct) as usize, f(&think));
        })
    });
    // audio & narration
    let st1 = st.clone();
    engine.register_fn("narrate", move |t: &str| {
        s!(st1, |sc: &mut Scene| {
            sc.narrate(t);
        })
    });
    let st1 = st.clone();
    engine.register_fn("voiceover", move |t: &str, fp: FnPtr| -> R<()> {
        let info = s!(st1, |sc: &mut Scene| sc.begin_voiceover(&Voiceover::new(t)));
        let mut m = Map::new();
        m.insert("start".into(), info.start.into());
        m.insert("duration".into(), info.duration.into());
        m.insert("end".into(), info.end().into());
        let mut b = Map::new();
        for (k, v) in &info.bookmarks {
            b.insert(k.as_str().into(), (*v).into());
        }
        m.insert("bookmarks".into(), b.into());
        let (engine, ast) = RUNTIME.get().ok_or("runtime not ready")?;
        let _ = fp.call::<Dynamic>(engine, ast, (m,))?;
        s!(st1, |sc: &mut Scene| sc.end_voiceover(&info));
        Ok(())
    });
    let st1 = st.clone();
    engine.register_fn(
        "voiceover_audio",
        move |t: &str, path: &str, fp: FnPtr| -> R<()> {
            let info = s!(st1, |sc: &mut Scene| sc
                .begin_voiceover(&Voiceover::new(t).audio(path)));
            let mut m = Map::new();
            m.insert("start".into(), info.start.into());
            m.insert("end".into(), info.end().into());
            let (engine, ast) = RUNTIME.get().ok_or("runtime not ready")?;
            let _ = fp.call::<Dynamic>(engine, ast, (m,))?;
            s!(st1, |sc: &mut Scene| sc.end_voiceover(&info));
            Ok(())
        },
    );
    let st1 = st.clone();
    engine.register_fn("sound", move |name: &str, vol: N| -> R<()> {
        let d = sfx::by_name(name).ok_or_else(|| format!("unknown sound effect '{name}'"))?;
        s!(st1, |sc: &mut Scene| sc.add_sound(d, f(&vol) as f32));
        Ok(())
    });
    let st1 = st.clone();
    engine.register_fn("sound_file", move |path: &str, vol: N| -> R<f64> {
        s!(st1, |sc: &mut Scene| sc
            .add_sound_file(path, f(&vol) as f32))
        .map_err(|e| e.to_string().into())
    });
    let st1 = st.clone();
    engine.register_fn("music", move |vol: N| {
        s!(st1, |sc: &mut Scene| sc.background_music(
            sfx::ambient_pad(16.0, 45.0, 1.0),
            f(&vol) as f32
        ))
    });
    let st1 = st.clone();
    engine.register_fn("music_file", move |path: &str, vol: N| -> R<()> {
        let d = AudioData::load(path).map_err(|e| e.to_string())?;
        s!(st1, |sc: &mut Scene| sc
            .background_music(Arc::new(d), f(&vol) as f32));
        Ok(())
    });
    let st1 = st.clone();
    engine.register_fn("captions", move |style: &str| {
        s!(st1, |sc: &mut Scene| {
            sc.captions.look = match style {
                "tiktok" | "highlight" => CaptionLook::tiktok(),
                "karaoke" => CaptionLook::karaoke(),
                "hidden" | "none" | "off" => CaptionLook {
                    style: CaptionStyle::Hidden,
                    ..Default::default()
                },
                _ => CaptionLook::default(),
            }
        })
    });
    let st1 = st.clone();
    engine.register_fn("effect", move |name: &str, v: N| -> R<()> {
        let x = f(&v) as f32;
        let e = match name {
            "vignette" => Effect::Vignette {
                strength: x,
                radius: 0.35,
            },
            "grain" => Effect::Grain { amount: x, seed: 1 },
            "bloom" => Effect::Bloom {
                threshold: 0.6,
                radius: 0.02,
                strength: x,
            },
            "grayscale" => Effect::Grayscale(x),
            "sepia" => Effect::Sepia(x),
            "invert" => Effect::Invert(x),
            "saturation" => Effect::Saturation(x),
            "contrast" => Effect::Contrast(x),
            "brightness" => Effect::Brightness(x),
            "hue" => Effect::HueRotate(x),
            "blur" => Effect::Blur(x),
            "pixelate" => Effect::Pixelate(x),
            "scanlines" => Effect::Scanlines {
                spacing: 0.006,
                strength: x,
            },
            "chromatic" | "chromatic_aberration" => Effect::ChromaticAberration(x),
            "letterbox" => Effect::Letterbox(x),
            "halftone" => Effect::Halftone { cell: x },
            "posterize" => Effect::Posterize(x as u32),
            "exposure" => Effect::Exposure(x),
            "fisheye" => Effect::Fisheye(x),
            "sharpen" => Effect::Sharpen(x),
            // Every other Remotion @remotion/effects name (and aliases).
            _ => {
                Effect::from_name(name, f(&v)).ok_or_else(|| format!("unknown effect '{name}'"))?
            }
        };
        s!(st1, |sc: &mut Scene| {
            if matches!(e, Effect::Grain { .. }) {
                sc.post_effect(0.0, f64::INFINITY, move |t| {
                    vec![Effect::Grain {
                        amount: x,
                        seed: (t * 60.0) as u32,
                    }]
                });
            } else {
                sc.effects(vec![e.clone()]);
            }
        });
        Ok(())
    });
    // Effect whose main amount animates from `a` to `b` over [start, start + dur]
    // (e.g. effect_anim("evolve", 0.0, 1.0, time(), 2.0) or a "shine" sweep).
    let st1 = st.clone();
    engine.register_fn(
        "effect_anim",
        move |name: &str, a: N, b: N, start: N, dur: N| -> R<()> {
            let (a, b, start, dur) = (f(&a), f(&b), f(&start), f(&dur).max(1e-6));
            if Effect::from_name(name, a).is_none() {
                return Err(format!("unknown effect '{name}'").into());
            }
            let name = name.to_string();
            s!(st1, |sc: &mut Scene| sc.post_effect(
                start,
                f64::INFINITY,
                move |t| {
                    let k = Rate::Smooth.eval((t / dur).clamp(0.0, 1.0));
                    Effect::from_name(&name, a + (b - a) * k)
                        .into_iter()
                        .collect()
                }
            ));
            Ok(())
        },
    );
    // Effect only between two times.
    let st1 = st.clone();
    engine.register_fn(
        "effect_between",
        move |name: &str, v: N, from: N, to: N| -> R<()> {
            let e =
                Effect::from_name(name, f(&v)).ok_or_else(|| format!("unknown effect '{name}'"))?;
            s!(st1, |sc: &mut Scene| sc.post_effect(
                f(&from),
                f(&to),
                move |_| vec![e.clone()]
            ));
            Ok(())
        },
    );
    // Colour grade with an Adobe .cube LUT file, mixed by `amount`.
    let st1 = st.clone();
    engine.register_fn("lut", move |path: &str, amount: N| -> R<()> {
        use runwee::effects::Lut;
        let loaded = Lut::load(path).map_err(|e| e.to_string())?;
        let k = (f(&amount) as f32).clamp(0.0, 1.0);
        let lut = if k >= 0.999 {
            loaded
        } else {
            Lut::from_fn(33, move |c| {
                let g = loaded.lookup(c);
                [
                    c[0] + (g[0] - c[0]) * k,
                    c[1] + (g[1] - c[1]) * k,
                    c[2] + (g[2] - c[2]) * k,
                ]
            })
        };
        let e = Effect::Lut(std::sync::Arc::new(lut));
        s!(st1, |sc: &mut Scene| sc.effects(vec![e]));
        Ok(())
    });
    // movie building
    let st1 = st.clone();
    engine.register_fn("next_scene", move |name: &str, dur: N| -> R<()> {
        let p = Presentation::parse(name).ok_or_else(|| format!("unknown transition '{name}'"))?;
        let mut g = st1.lock().unwrap();
        let fresh = if g.vertical {
            Scene::vertical()
        } else {
            Scene::new()
        };
        let done = std::mem::replace(&mut g.cur, fresh);
        g.done.push((done, Some(Transition::new(p, f(&dur)))));
        Ok(())
    });
    let st1 = st.clone();
    engine.register_fn("next_scene", move || {
        let mut g = st1.lock().unwrap();
        let fresh = if g.vertical {
            Scene::vertical()
        } else {
            Scene::new()
        };
        let done = std::mem::replace(&mut g.cur, fresh);
        g.done.push((done, None));
    });
    // 3D
    engine.register_type_with_name::<CameraRig>("CameraRig");
    engine.register_type_with_name::<Mesh>("Mesh");
    let st1 = st.clone();
    engine.register_fn("camera3d", move |phi: N, theta: N| {
        s!(st1, |sc: &mut Scene| sc.camera3d(f(&phi), f(&theta)))
    });
    engine.register_get("phi", |r: &mut CameraRig| r.phi);
    engine.register_get("theta", |r: &mut CameraRig| r.theta);
    engine.register_get("zoom", |r: &mut CameraRig| r.zoom);
    let st1 = st.clone();
    engine.register_fn("add_3d", move |rig: CameraRig, m: Mesh| {
        s!(st1, |sc: &mut Scene| sc.add_3d(rig, m))
    });
    let st1 = st.clone();
    engine.register_fn("add_3d_dynamic", move |rig: CameraRig, fp: FnPtr| {
        s!(st1, |sc: &mut Scene| sc.add_3d_dynamic(rig, move |ctx| {
            with_frame(ctx, || call_rt::<Mesh>(&fp, (ctx.elapsed,))).unwrap_or_default()
        }))
    });
    let st1 = st.clone();
    engine.register_fn(
        "ambient_rotation",
        move |rig: CameraRig, rate: N, dur: N| {
            s!(st1, |sc: &mut Scene| sc.ambient_rotation(
                rig,
                f(&rate),
                f(&dur)
            ))
        },
    );
    // problem solving
    engine.register_type_with_name::<Problem>("Problem");
    engine.register_fn("problem", |title: &str, statement: &str| {
        Problem::new(title, statement)
    });
    engine.register_fn("given", |p: &mut Problem, a: Array| {
        let v = strings(&a);
        let r: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
        p.clone().given(&r)
    });
    engine.register_fn("find", |p: &mut Problem, a: Array| {
        let v = strings(&a);
        let r: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
        p.clone().find(&r)
    });
    engine.register_fn("step", |p: &mut Problem, explain: &str, tex: &str| {
        p.clone().step(Step::new(explain, tex))
    });
    engine.register_fn("step_morph", |p: &mut Problem, explain: &str, tex: &str| {
        p.clone().step(Step::new(explain, tex).morph())
    });
    engine.register_fn(
        "step_highlight",
        |p: &mut Problem, explain: &str, tex: &str, hl: &str| {
            p.clone().step(Step::new(explain, tex).highlight(hl))
        },
    );
    engine.register_fn("answer", |p: &mut Problem, a: &str| p.clone().answer(a));
    engine.register_fn("diagram", |p: &mut Problem, m: Mobject| {
        p.clone().diagram(m)
    });
    engine.register_fn("accent", |p: &mut Problem, c: N| p.clone().accent(col(&c)));
    engine.register_fn("silent", |p: &mut Problem| p.clone().silent());
    engine.register_fn("quadratic_problem", |a: N, b: N, c: N| {
        quadratic_problem(f(&a), f(&b), f(&c))
    });
    engine.register_fn("kinematics_problem", |u: N, a: N, t: N| {
        kinematics_problem(f(&u), f(&a), f(&t))
    });
    engine.register_fn("ohms_law_problem", |v: N, r: N| {
        ohms_law_problem(f(&v), f(&r))
    });
    engine.register_fn(
        "stoichiometry_problem",
        |r: Array, p: Array, given: &str, mass: N, wanted: &str| -> R<Problem> {
            let (rs, ps) = (strings(&r), strings(&p));
            let rr: Vec<&str> = rs.iter().map(|s| s.as_str()).collect();
            let pp: Vec<&str> = ps.iter().map(|s| s.as_str()).collect();
            stoichiometry_problem(&rr, &pp, given, f(&mass), wanted)
                .ok_or_else(|| "cannot balance this reaction".into())
        },
    );
    let st1 = st.clone();
    engine.register_fn("solve", move |p: Problem| {
        s!(st1, |sc: &mut Scene| sc.solve(&p))
    });
}

fn register_3d_and_stem(engine: &mut Engine) {
    engine.register_fn("merge", |a: &mut Mesh, b: Mesh| a.clone().merge(b));
    engine.register_fn("+", |a: Mesh, b: Mesh| a.merge(b));
    engine.register_fn("shift", |a: &mut Mesh, v: Vec3| a.clone().shift(v));
    engine.register_fn("scale", |a: &mut Mesh, s: N| a.clone().scale(f(&s)));
    engine.register_fn("rotate", |a: &mut Mesh, axis: Vec3, ang: N| {
        a.clone().rotate(axis, f(&ang))
    });
    engine.register_fn("color", |a: &mut Mesh, c: N| a.clone().color(col(&c)));
    engine.register_fn("opacity", |a: &mut Mesh, o: N| {
        a.clone().opacity(f(&o) as f32)
    });
    engine.register_fn("wireframe", |a: &mut Mesh, c: N| {
        a.clone().wireframe(col(&c), 1.5)
    });
    engine.register_fn("sphere", |r: N, c: N| sphere(f(&r), 16, col(&c)));
    engine.register_fn("cube", |s: N, c: N| cube(f(&s), col(&c)));
    engine.register_fn("prism", |x: N, y: N, z: N, c: N| {
        prism(f(&x), f(&y), f(&z), col(&c))
    });
    engine.register_fn("torus", |a: N, b: N, c: N| torus(f(&a), f(&b), 20, col(&c)));
    engine.register_fn("cylinder", |r: N, h: N, c: N| {
        cylinder(f(&r), f(&h), 24, col(&c))
    });
    engine.register_fn("cone", |r: N, h: N, c: N| cone(f(&r), f(&h), 24, col(&c)));
    engine.register_fn("tetrahedron", |r: N, c: N| tetrahedron(f(&r), col(&c)));
    engine.register_fn("octahedron", |r: N, c: N| octahedron(f(&r), col(&c)));
    engine.register_fn("icosahedron", |r: N, c: N| icosahedron(f(&r), col(&c)));
    engine.register_fn("dodecahedron", |r: N, c: N| dodecahedron(f(&r), col(&c)));
    engine.register_fn("axes3d", |r: N, s: N| axes3d(f(&r), f(&s)));
    engine.register_fn("arrow3d", |a: Vec3, b: Vec3, c: N| arrow3d(a, b, col(&c)));
    engine.register_fn("line3d", |a: Vec3, b: Vec3, c: N| {
        line3d(a, b, col(&c), 4.0)
    });
    engine.register_fn("dot3d", |p: Vec3, r: N, c: N| dot3d(p, f(&r), col(&c)));
    engine.register_fn(
        "surface_z",
        |ctx: NativeCallContext, fp: FnPtr, x: Array, y: Array, res: N| {
            let (x, y) = (range3(&x), range3(&y));
            surface_z(
                |a, b| fp.call_within_context::<f64>(&ctx, (a, b)).unwrap_or(0.0),
                (x.0, x.1),
                (y.0, y.1),
                f(&res) as usize,
            )
        },
    );
    engine.register_fn(
        "curve3d",
        |ctx: NativeCallContext, fp: FnPtr, t0: N, t1: N, c: N| {
            parametric_curve3d(
                |t| {
                    fp.call_within_context::<Vec3>(&ctx, (t,))
                        .unwrap_or_default()
                },
                f(&t0),
                f(&t1),
                300,
                col(&c),
            )
        },
    );
    // physics
    engine.register_type_with_name::<sim::Trajectory>("Trajectory");
    engine.register_fn("pendulum_sim", |l: N, th: N, g: N, damping: N, dur: N| {
        sim::pendulum(f(&l), f(&th), f(&g), f(&damping), f(&dur))
    });
    engine.register_fn("spring_sim", |k: N, m: N, x0: N, damping: N, dur: N| {
        sim::spring_mass(f(&k), f(&m), f(&x0), 0.0, f(&damping), f(&dur))
    });
    engine.register_fn("get", |tr: &mut sim::Trajectory, t: N, k: N| {
        tr.get(f(&t), f(&k) as usize)
    });
    engine.register_fn("projectile", |p0: Vec2, v: N, ang: N, g: N, t: N| {
        sim::projectile(p0, f(&v), f(&ang), f(&g), f(&t))
    });
    engine.register_fn("pendulum", |pivot: Vec2, l: N, th: N, c: N| {
        phys::pendulum_mobject(pivot, f(&l), f(&th), 0.22, col(&c))
    });
    engine.register_fn("spring_block", |wall: N, y: N, x: N, c: N| {
        phys::spring_block(f(&wall), f(&y), f(&x), 0.8, col(&c))
    });
    engine.register_fn("incline", |o: Vec2, len: N, ang: N, c: N| {
        phys::incline(o, f(&len), f(&ang), col(&c))
    });
    engine.register_fn("block_on_incline", |o: Vec2, ang: N, s: N, c: N| {
        phys::block_on_incline(o, f(&ang), f(&s), 0.6, col(&c))
    });
    engine.register_fn("force_arrow", |from: Vec2, v: Vec2, label: &str, c: N| {
        phys::force_arrow(from, v, label, col(&c))
    });
    engine.register_fn("free_body", |center: Vec2, forces: Array| {
        let fs: Vec<(Vec2, String, Color)> = forces
            .iter()
            .filter_map(|x| x.clone().try_cast::<Array>())
            .map(|a| (vec_of(&a[0]), a[1].to_string(), col(&a[2])))
            .collect();
        let r: Vec<(Vec2, &str, Color)> = fs.iter().map(|(v, l, c)| (*v, l.as_str(), *c)).collect();
        phys::free_body_diagram(center, 1.0, &r)
    });
    engine.register_fn("projectile_path", |p0: Vec2, v: N, ang: N, g: N, c: N| {
        phys::projectile_path(p0, f(&v), f(&ang), f(&g), col(&c))
    });
    engine.register_fn("vector_components", |o: Vec2, v: Vec2, c: N| {
        phys::vector_components(o, v, col(&c), ("v", "v_x", "v_y"))
    });
    engine.register_fn("energy_bars", |ke: N, pe: N, max: N| {
        phys::energy_bars(f(&ke), f(&pe), f(&max), 2.5)
    });
    engine.register_fn(
        "travelling_wave",
        |x0: N, x1: N, amp: N, wl: N, freq: N, t: N, c: N| {
            phys::travelling_wave(f(&x0), f(&x1), f(&amp), f(&wl), f(&freq), f(&t), col(&c))
        },
    );
    engine.register_fn(
        "standing_wave",
        |x0: N, len: N, amp: N, n: N, t: N, c: N| {
            phys::standing_wave(f(&x0), f(&len), f(&amp), f(&n) as u32, 1.0, f(&t), col(&c))
        },
    );
    engine.register_fn("wavefronts", |src: Vec2, wl: N, t: N, c: N| {
        phys::wavefronts(src, f(&wl), f(&t), 1.0, 6.0, col(&c))
    });
    engine.register_fn("lens_diagram", |focal: N, d_o: N, h_o: N| {
        phys::lens_ray_diagram(Vec2::ZERO, f(&focal), f(&d_o), f(&h_o))
    });
    engine.register_fn("refraction", |n1: N, n2: N, deg: N| {
        phys::refraction_diagram(f(&n1), f(&n2), f(&deg))
    });
    engine.register_fn("prism_dispersion", |s: N| {
        phys::prism_dispersion(Vec2::ZERO, f(&s))
    });
    engine.register_fn("resistor", |a: Vec2, b: Vec2| phys::resistor(a, b));
    engine.register_fn("capacitor", |a: Vec2, b: Vec2| phys::capacitor(a, b));
    engine.register_fn("inductor", |a: Vec2, b: Vec2| phys::inductor(a, b));
    engine.register_fn("battery", |a: Vec2, b: Vec2| phys::battery(a, b));
    engine.register_fn("bulb", |a: Vec2, b: Vec2, on: N| phys::bulb(a, b, f(&on)));
    engine.register_fn("switch", |a: Vec2, b: Vec2, closed: bool| {
        phys::switch(a, b, closed)
    });
    engine.register_fn("meter", |a: Vec2, b: Vec2, l: &str| phys::meter(a, b, l));
    engine.register_fn("wire", |pts: Array| phys::wire(&points(&pts)));
    engine.register_fn("current_dots", |pts: Array, t: N, speed: N| {
        phys::current_dots(
            &phys::circuit_loop(&points(&pts)),
            f(&t),
            f(&speed),
            0.6,
            YELLOW,
        )
    });
    fn charges_of(a: &Array) -> Vec<(Vec2, f64)> {
        a.iter()
            .filter_map(|x| x.clone().try_cast::<Array>())
            .map(|c| (v2(f(&c[0]), f(&c[1])), f(&c[2])))
            .collect()
    }
    engine.register_fn("point_charge", |p: Vec2, q: N| phys::point_charge(p, f(&q)));
    engine.register_fn("field_lines", |charges: Array| {
        phys::field_lines(
            &charges_of(&charges),
            12,
            runwee::path::Rect::from_center(Vec2::ZERO, 14.0, 8.0),
            BLUE_B,
        )
    });
    engine.register_fn("equipotentials", |charges: Array, levels: Array| {
        phys::equipotentials(
            &charges_of(&charges),
            &floats(&levels),
            runwee::path::Rect::from_center(Vec2::ZERO, 14.0, 8.0),
            YELLOW_E,
        )
    });
    engine.register_fn("magnetic_field", |p: Vec2, out: bool| {
        phys::wire_magnetic_field(p, out, 4, BLUE)
    });
    engine.register_fn("bar_magnet", |p: Vec2, len: N| phys::bar_magnet(p, f(&len)));
    // chemistry
    engine.register_fn("periodic_table", |cell: N| chem::periodic_table(f(&cell)));
    engine.register_fn("element_tile", |z: N, size: N| {
        chem::element_tile(f(&z) as u32, f(&size))
    });
    engine.register_fn("element_name", |z: N| {
        chem::element(f(&z) as u32)
            .map(|e| e.name.to_string())
            .unwrap_or_default()
    });
    engine.register_fn("bohr_atom", |z: N, r: N, phase: N| {
        chem::bohr_atom(f(&z) as u32, f(&r), f(&phase))
    });
    engine.register_fn("electron_config", |z: N| {
        chem::electron_configuration_tex(f(&z) as u32, false)
    });
    engine.register_fn("electron_config_short", |z: N| {
        chem::electron_configuration_tex(f(&z) as u32, true)
    });
    engine.register_fn("orbital_boxes", |z: N, size: N| {
        chem::orbital_box_diagram(f(&z) as u32, f(&size))
    });
    engine.register_fn("orbital", |kind: &str, size: N| {
        chem::orbital_shape(kind, f(&size))
    });
    engine.register_fn("lewis", |sym: &str, e: N, size: N| {
        chem::lewis_dot(sym, f(&e) as u32, f(&size))
    });
    engine.register_fn(
        "molecule",
        |name: &str, scale: N, skeletal: bool| -> R<Mobject> {
            chem::molecule_2d(name)
                .map(|m| m.render(f(&scale), skeletal))
                .ok_or_else(|| format!("unknown molecule '{name}'").into())
        },
    );
    engine.register_fn("molecule3d", |name: &str, scale: N| -> R<Mesh> {
        chem::molecule_3d(name)
            .map(|m| m.mesh(f(&scale)))
            .ok_or_else(|| format!("unknown molecule '{name}'").into())
    });
    engine.register_fn("molecule_xyz", |xyz: &str, scale: N| {
        chem::Molecule3D::from_xyz(xyz).mesh(f(&scale))
    });
    engine.register_fn("balance", |r: Array, p: Array| -> R<String> {
        let (rs, ps) = (strings(&r), strings(&p));
        let rr: Vec<&str> = rs.iter().map(|s| s.as_str()).collect();
        let pp: Vec<&str> = ps.iter().map(|s| s.as_str()).collect();
        chem::balanced_equation(&rr, &pp).ok_or_else(|| "cannot balance".into())
    });
    engine.register_fn("molar_mass", |s: &str| chem::molar_mass(s));
    engine.register_fn("beaker", |w: N, h: N, level: N, c: N| {
        chem::beaker(f(&w), f(&h), f(&level), col(&c))
    });
    engine.register_fn("flask", |s: N, level: N, c: N| {
        chem::erlenmeyer_flask(f(&s), f(&level), col(&c))
    });
    engine.register_fn("test_tube", |h: N, level: N, c: N| {
        chem::test_tube(f(&h), f(&level), col(&c))
    });
    engine.register_fn("burette", |h: N, level: N, c: N| {
        chem::burette(f(&h), f(&level), col(&c))
    });
    engine.register_fn("burner", |s: N, t: N| chem::bunsen_burner(f(&s), f(&t)));
    engine.register_fn("ph_scale", |w: N, ph: N| chem::ph_scale(f(&w), f(&ph)));
    // biology
    engine.register_fn("animal_cell", |r: N| bio::animal_cell(f(&r)));
    engine.register_fn("plant_cell", |w: N, h: N| bio::plant_cell(f(&w), f(&h)));
    engine.register_fn("mitochondrion", |l: N| bio::mitochondrion(f(&l), ORANGE));
    engine.register_fn("chloroplast", |l: N| bio::chloroplast(f(&l)));
    engine.register_fn("dna", |len: N, r: N, turns: N, phase: N, seq: &str| {
        bio::dna_helix(f(&len), f(&r), f(&turns), f(&phase), seq)
    });
    engine.register_fn("rna", |seq: &str| bio::rna_strand(seq, 0.4));
    engine.register_fn("neuron", |len: N, signal: N| {
        bio::neuron(f(&len), Some(f(&signal)))
    });
    engine.register_fn("virus", |r: N, c: N| bio::virus(f(&r), col(&c)));
    engine.register_fn("bacterium", |l: N, c: N| bio::bacterium(f(&l), col(&c)));
    engine.register_fn("red_blood_cell", |r: N| bio::red_blood_cell(f(&r)));
    engine.register_fn("chromosome", |h: N, c: N| {
        bio::chromosome(f(&h), col(&c), false)
    });
    engine.register_fn("mitosis", |t: N, size: N| bio::mitosis(f(&t), f(&size)));
    engine.register_fn("mitosis_stage", |t: N| {
        bio::mitosis_stage_name(f(&t)).to_string()
    });
    engine.register_fn("punnett", |a: &str, b: &str, cell: N| {
        bio::punnett_square(a, b, f(&cell))
    });
    engine.register_fn("lipid_bilayer", |w: N, n: N| {
        bio::lipid_bilayer(f(&w), f(&n) as usize)
    });
    engine.register_fn("enzyme", |s: N, bound: N| {
        bio::enzyme_substrate(f(&s), f(&bound))
    });
    engine.register_fn("labeled_diagram", |m: Mobject, labels: Array, size: N| {
        let ls: Vec<(String, Vec2)> = labels
            .iter()
            .filter_map(|x| x.clone().try_cast::<Array>())
            .map(|a| (a[0].to_string(), vec_of(&a[1])))
            .collect();
        let r: Vec<(&str, Vec2)> = ls.iter().map(|(s, p)| (s.as_str(), *p)).collect();
        bio::labeled_diagram(m, &r, f(&size))
    });
    engine.register_fn("logistic", |t: N, k: N, r: N, n0: N| {
        bio::logistic(f(&t), f(&k), f(&r), f(&n0))
    });
}

/// Scope with constants (directions, PI/TAU, palette colours).
pub fn default_scope(props: Map) -> Scope<'static> {
    let mut scope = Scope::new();
    for (name, v) in [
        ("ORIGIN", ORIGIN),
        ("UP", UP),
        ("DOWN", DOWN),
        ("LEFT", LEFT),
        ("RIGHT", RIGHT),
        ("UL", UL),
        ("UR", UR),
        ("DL", DL),
        ("DR", DR),
    ] {
        scope.push_constant(name, v);
    }
    for (name, v) in [("X", Vec3::X), ("Y", Vec3::Y), ("Z", Vec3::Z)] {
        scope.push_constant(name, v);
    }
    scope.push_constant("PI", PI);
    scope.push_constant("TAU", TAU);
    scope.push_constant("DEG", DEGREES);
    for (name, c) in PALETTE {
        scope.push_constant(*name, *c);
    }
    scope.push_constant("PROPS", props);
    scope
}

/// Build an engine bound to a fresh state.
fn build_engine() -> (Engine, Shared) {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(256, 256);
    engine.set_max_call_levels(256);
    let st: Shared = Arc::new(Mutex::new(State {
        done: vec![],
        cur: Scene::new(),
        vertical: false,
        quality: None,
        theme: None,
    }));
    register_math(&mut engine);
    register_mobject(&mut engine);
    register_shapes(&mut engine);
    register_text(&mut engine);
    register_graph(&mut engine);
    register_canvas(&mut engine);
    register_anims(&mut engine);
    register_scene(&mut engine, st.clone());
    register_3d_and_stem(&mut engine);
    engine.on_print(|s| eprintln!("[script] {s}"));
    (engine, st)
}

#[cfg(test)]
pub fn test_engine() -> (Engine, ()) {
    (build_engine().0, ())
}

/// Run a script source and collect its scenes.
pub fn run_script(src: &str, props: Map) -> anyhow::Result<ScriptOutput> {
    let (engine, st) = build_engine();
    let ast = engine
        .compile(src)
        .map_err(|e| anyhow::anyhow!("script syntax error: {e}"))?;
    let engine = Arc::new(engine);
    let ast = Arc::new(ast);
    if RUNTIME.set((engine.clone(), ast.clone())).is_err() {
        anyhow::bail!("only one script can run per process");
    }
    let mut scope = default_scope(props);
    engine
        .run_ast_with_scope(&mut scope, &ast)
        .map_err(|e| anyhow::anyhow!("script error: {e}"))?;
    let mut g = st.lock().unwrap();
    let cur = std::mem::take(&mut g.cur);
    let mut scenes = std::mem::take(&mut g.done);
    scenes.push((cur, None));
    Ok(ScriptOutput {
        scenes,
        quality: g.quality.clone(),
        vertical: g.vertical,
    })
}

/// Function signatures exposed to scripts (for `runwee api`).
pub fn api_signatures() -> Vec<String> {
    let (engine, _) = build_engine();
    let mut v = engine.gen_fn_signatures(false);
    v.sort();
    v.dedup();
    v
}
