//! The rest of manim's animation & mobject catalogue: fades to/from points,
//! spiral-in, turn-inside-out, word-by-word text, type-with-cursor, increasing
//! subsets, phase flow, complex maps, save/restore, swap/cyclic-replace,
//! broadcast, traced paths, animated boundaries, polygrams, elbows, dashed
//! objects, glow dots, dot clouds and fractals.

use crate::anim::*;
use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::rate::Rate;
use crate::scene::{multiply_opacity, Obj, Scene};
use crate::shapes::*;
use crate::style::*;
use std::sync::Arc;

fn custom_anim(
    o: Obj,
    introduces: bool,
    removes: bool,
    f: impl Fn(&Mobject, f64) -> Mobject + Send + Sync + 'static,
) -> Animation {
    Animation::new(AnimKind::Custom {
        obj: o,
        f: Arc::new(f),
        introduces,
        removes,
    })
}

// ----------------------------------------------------------------- fades

/// Appear by growing out of `p` while fading in (manim's `FadeInFromPoint`).
pub fn fade_in_from_point(o: Obj, p: Vec2) -> Animation {
    custom_anim(o, true, false, move |m, x| {
        let c = m.center();
        multiply_opacity(
            m.clone()
                .shift((p - c) * (1.0 - x))
                .scale_about(x.max(1e-3), p.lerp(c, x)),
            x as f32,
        )
    })
}

/// Shrink into `p` while fading out (manim's `FadeOutToPoint`).
pub fn fade_out_to_point(o: Obj, p: Vec2) -> Animation {
    custom_anim(o, false, true, move |m, x| {
        let c = m.center();
        multiply_opacity(
            m.clone()
                .shift((p - c) * x)
                .scale_about((1.0 - x).max(1e-3), c.lerp(p, x)),
            1.0 - x as f32,
        )
    })
}

/// Fade in then back out (manim's `VFadeInThenOut`).
pub fn fade_in_then_out(o: Obj) -> Animation {
    custom_anim(o, true, true, |m, x| {
        multiply_opacity(m.clone(), Rate::ThereAndBack.eval(x) as f32)
    })
    .rate(Rate::Linear)
}

/// Pieces spiral in from around the object (manim's `SpiralIn`).
pub fn spiral_in(o: Obj) -> Animation {
    custom_anim(o, true, false, |m, x| {
        let c = m.center();
        let n = m.leaf_count().max(1);
        let mut i = 0;
        let mut out = m.clone();
        out.for_each_mut(&mut |node| {
            if !node.is_leaf_drawable() {
                return;
            }
            let a = crate::rate::lagged_alpha(x, i, n, 0.08);
            let lc = node.center();
            let angle = (1.0 - a) * TAU * 0.75;
            let r = (1.0 - a) * 6.0;
            let target = c + (lc - c).rotate(angle) + Vec2::from_angle(i as f64 * 2.4 + angle) * r;
            let mut leaf = std::mem::take(node);
            leaf.shift_mut(target - lc);
            leaf = leaf.scale(a.max(1e-3));
            leaf.style.opacity *= a as f32;
            *node = leaf;
            i += 1;
        });
        out
    })
    .run_time(2.0)
}

/// Morph every path into its own reverse (manim's `TurnInsideOut`).
pub fn turn_inside_out(o: Obj) -> Animation {
    custom_anim(o, false, false, |m, x| {
        let mut out = m.clone();
        out.for_each_mut(&mut |node| {
            let rev = Path {
                subpaths: node.path.subpaths.iter().map(|s| s.reversed()).collect(),
            };
            node.path = node.path.lerp(&rev, x);
        });
        out
    })
}

// ------------------------------------------------------------------- text

fn word_groups(m: &Mobject) -> Vec<usize> {
    // Word boundaries = horizontal gaps clearly larger than the typical letter gap,
    // or a jump to a new line.
    let boxes: Vec<Rect> = m.leaves().iter().map(|l| l.bounds_or_origin()).collect();
    let mut gaps: Vec<f64> = boxes
        .windows(2)
        .map(|w| w[1].min.x - w[0].max.x)
        .filter(|g| *g > -0.5)
        .collect();
    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = gaps.get(gaps.len() / 2).copied().unwrap_or(0.05).max(0.01);
    let threshold = median * 2.0;
    let mut ids = Vec::with_capacity(boxes.len());
    let mut word = 0;
    for (i, b) in boxes.iter().enumerate() {
        if i > 0 {
            let p = boxes[i - 1];
            let new_line = b.max.x < p.min.x || b.max.y < p.min.y - p.height() * 0.5;
            if new_line || b.min.x - p.max.x > threshold {
                word += 1;
            }
        }
        ids.push(word);
    }
    ids
}

/// Reveal text one word at a time (manim's `AddTextWordByWord`).
pub fn add_text_word_by_word(o: Obj) -> Animation {
    custom_anim(o, true, false, |m, x| {
        let ids = word_groups(m);
        let n = ids.last().map_or(1, |w| w + 1);
        let shown = x * n as f64;
        let mut i = 0;
        let mut out = m.clone();
        out.for_each_mut(&mut |node| {
            if node.is_leaf_drawable() {
                let vis = (shown - ids[i] as f64).clamp(0.0, 1.0) as f32;
                node.style.opacity *= vis;
                i += 1;
            }
        });
        out
    })
    .rate(Rate::Linear)
}

/// Remove letters from the end (manim's `RemoveTextLetterByLetter`).
pub fn remove_text_letter_by_letter(o: Obj) -> Animation {
    custom_anim(o, false, true, |m, x| {
        let n = m.leaf_count().max(1);
        let keep = (1.0 - x) * n as f64;
        let mut i = 0;
        let mut out = m.clone();
        out.for_each_mut(&mut |node| {
            if node.is_leaf_drawable() {
                node.style.opacity *= (keep - i as f64).clamp(0.0, 1.0) as f32;
                i += 1;
            }
        });
        out
    })
    .rate(Rate::Linear)
}

fn cursor_after(m: &Mobject, visible: usize, color: Color, blink_on: bool) -> Mobject {
    let leaves = m.leaves();
    let h = m.height().max(0.3);
    let x = if visible == 0 {
        leaves.first().map_or(m.left().x, |l| l.left().x)
    } else {
        leaves[(visible - 1).min(leaves.len() - 1)].right().x + h * 0.06
    };
    let y = if visible == 0 {
        leaves.first().map_or(m.center().y, |l| l.center().y)
    } else {
        leaves[(visible - 1).min(leaves.len() - 1)].center().y
    };
    rectangle(h * 0.08, h * 1.1)
        .move_to(v2(x + h * 0.04, y))
        .fill(color, if blink_on { 1.0 } else { 0.0 })
        .no_stroke()
}

/// Typewriter with a blinking cursor (manim's `TypeWithCursor`).
pub fn type_with_cursor(o: Obj, cursor: Color) -> Animation {
    custom_anim(o, true, false, move |m, x| {
        let n = m.leaf_count().max(1);
        let vis = (x * n as f64).floor() as usize;
        let mut i = 0;
        let mut out = m.clone();
        out.for_each_mut(&mut |node| {
            if node.is_leaf_drawable() {
                if i >= vis {
                    node.style.opacity = 0.0;
                }
                i += 1;
            }
        });
        if x < 1.0 {
            out.push(cursor_after(m, vis, cursor, true));
        }
        out
    })
    .rate(Rate::Linear)
}

/// Delete text with a cursor (manim's `UntypeWithCursor`).
pub fn untype_with_cursor(o: Obj, cursor: Color) -> Animation {
    custom_anim(o, false, true, move |m, x| {
        let n = m.leaf_count().max(1);
        let vis = ((1.0 - x) * n as f64).ceil() as usize;
        let mut i = 0;
        let mut out = m.clone();
        out.for_each_mut(&mut |node| {
            if node.is_leaf_drawable() {
                if i >= vis {
                    node.style.opacity = 0.0;
                }
                i += 1;
            }
        });
        out.push(cursor_after(m, vis, cursor, true));
        out
    })
    .rate(Rate::Linear)
}

/// Parts appear cumulatively without fading (manim's `ShowIncreasingSubsets`).
pub fn show_increasing_subsets(o: Obj) -> Animation {
    custom_anim(o, true, false, |m, x| {
        let n = m.children.len().max(1);
        let k = (x * n as f64).ceil() as usize;
        let mut out = m.clone();
        for (i, c) in out.children.iter_mut().enumerate() {
            if i >= k {
                *c = multiply_opacity(std::mem::take(c), 0.0);
            }
        }
        out
    })
    .rate(Rate::Linear)
}

/// Show one part at a time (manim's `ShowSubmobjectsOneByOne`).
pub fn show_submobjects_one_by_one(o: Obj) -> Animation {
    custom_anim(o, true, false, |m, x| {
        let n = m.children.len().max(1);
        let k = ((x * n as f64).ceil() as usize).clamp(1, n) - 1;
        let mut out = m.clone();
        for (i, c) in out.children.iter_mut().enumerate() {
            if i != k {
                *c = multiply_opacity(std::mem::take(c), 0.0);
            }
        }
        out
    })
    .rate(Rate::Linear)
}

/// Draw then erase (manim's `ShowCreationThenDestruction`).
pub fn show_creation_then_destruction(o: Obj) -> Animation {
    custom_anim(o, true, true, |m, x| {
        if x < 0.5 {
            m.clone().partial(0.0, 2.0 * x)
        } else {
            m.clone().partial(2.0 * x - 1.0, 1.0)
        }
    })
    .run_time(2.0)
}

/// Draw then fade out (manim's `ShowCreationThenFadeOut`).
pub fn show_creation_then_fade_out(o: Obj) -> Animation {
    custom_anim(o, true, true, |m, x| {
        if x < 0.5 {
            m.clone().partial(0.0, 2.0 * x)
        } else {
            multiply_opacity(m.clone(), (2.0 - 2.0 * x) as f32)
        }
    })
    .run_time(2.0)
}

// ----------------------------------------------------------- flows & maps

/// Complex number helpers on `Vec2` (x = real, y = imaginary).
pub mod complex {
    use crate::math::*;
    pub fn mul(a: Vec2, b: Vec2) -> Vec2 {
        v2(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x)
    }
    pub fn div(a: Vec2, b: Vec2) -> Vec2 {
        let d = b.length_sq().max(1e-300);
        v2((a.x * b.x + a.y * b.y) / d, (a.y * b.x - a.x * b.y) / d)
    }
    pub fn exp(z: Vec2) -> Vec2 {
        Vec2::polar(z.x.exp(), z.y)
    }
    pub fn ln(z: Vec2) -> Vec2 {
        v2(z.length().ln(), z.angle())
    }
    pub fn powf(z: Vec2, p: f64) -> Vec2 {
        Vec2::polar(z.length().powf(p), z.angle() * p)
    }
    pub fn sin(z: Vec2) -> Vec2 {
        v2(z.x.sin() * z.y.cosh(), z.x.cos() * z.y.sinh())
    }
    pub fn cos(z: Vec2) -> Vec2 {
        v2(z.x.cos() * z.y.cosh(), -z.x.sin() * z.y.sinh())
    }
}

/// Apply a complex function z ↦ f(z) (manim's `ApplyComplexFunction`).
pub fn apply_complex_function(
    o: Obj,
    f: impl Fn(Vec2) -> Vec2 + Send + Sync + 'static,
) -> Animation {
    apply_function(o, f)
}

/// Flow points along a vector field for `virtual_time` (manim's `PhaseFlow`).
pub fn phase_flow(
    o: Obj,
    field: impl Fn(Vec2) -> Vec2 + Send + Sync + 'static,
    virtual_time: f64,
) -> Animation {
    let field = Arc::new(field);
    homotopy(o, move |p, t| {
        let steps = (t * virtual_time * 60.0).ceil().max(1.0) as usize;
        let dt = t * virtual_time / steps as f64;
        let mut q = p;
        for _ in 0..steps {
            let k1 = field(q);
            let k2 = field(q + k1 * (dt / 2.0));
            let k3 = field(q + k2 * (dt / 2.0));
            let k4 = field(q + k3 * dt);
            q += (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (dt / 6.0);
        }
        q
    })
    .rate(Rate::Linear)
}

/// Continuous linear rotation (manim's `Rotating`).
pub fn rotating(o: Obj, angle: f64, about: Option<Vec2>) -> Animation {
    let a = match about {
        Some(p) => rotate_about(o, angle, p),
        None => rotate(o, angle),
    };
    a.rate(Rate::Linear)
}

/// manim's `ScaleInPlace`.
pub fn scale_in_place(o: Obj, s: f64) -> Animation {
    o.animate().scale(s).build()
}

/// Pass a vertical highlight band across an object (manim's `FlashyFadeIn` light).
pub fn sweep_highlight(o: Obj, color: Color) -> Animation {
    custom_anim(o, false, false, move |m, x| {
        let b = m.bounds_or_origin();
        let pos = b.min.x + (b.width() + 1.0) * x - 0.5;
        let mut out = m.clone();
        out.for_each_mut(&mut |n| {
            if n.is_leaf_drawable() {
                let d = ((n.center().x - pos) / 0.4).abs();
                let k = (1.0 - d).clamp(0.0, 1.0) as f32;
                n.style.fill.map_colors(&|c| c.mix(color, k));
                n.style.stroke.map_colors(&|c| c.mix(color, k));
            }
        });
        out
    })
    .rate(Rate::Linear)
}

// --------------------------------------------------------- scene helpers

impl Scene {
    /// Remember an object's state (manim's `save_state`).
    pub fn save_state(&mut self, o: Obj) {
        let m = self.get(o);
        self.saved.insert(o, m);
    }
    /// Animate back to the saved state (manim's `Restore`).
    pub fn restore(&mut self, o: Obj) -> Animation {
        let m = self.saved.get(&o).cloned().unwrap_or_else(|| self.get(o));
        transform(o, m)
    }
    /// Each object moves to the next one's position along an arc (manim's `CyclicReplace`).
    pub fn cyclic_replace(&mut self, objs: &[Obj]) -> Animation {
        let centers: Vec<Vec2> = objs.iter().map(|o| self.get(*o).center()).collect();
        let n = objs.len();
        together(
            objs.iter()
                .enumerate()
                .map(|(i, o)| {
                    let target = self.get(*o).move_to(centers[(i + 1) % n]);
                    transform_arc(*o, target, PI / 2.0)
                })
                .collect(),
        )
    }
    /// Swap two objects' positions (manim's `Swap`).
    pub fn swap(&mut self, a: Obj, b: Obj) -> Animation {
        self.cyclic_replace(&[a, b])
    }
    /// Expanding rings from a point (manim's `Broadcast`).
    pub fn broadcast(&mut self, p: Vec2, color: Color, rings: usize, radius: f64) -> Animation {
        let anims: Vec<Animation> = (0..rings)
            .map(|_| {
                let o = self.register(circle_at(p, radius).stroke(color, 3.0));
                custom_anim(o, true, true, move |m, x| {
                    multiply_opacity(m.clone().scale_about(x.max(0.01), p), (1.0 - x) as f32)
                })
                .rate(Rate::OutCubic)
                .run_time(1.5)
            })
            .collect();
        lagged_start(anims, 0.25)
    }
    /// Rectangle around an object whose rainbow outline keeps cycling (manim's `AnimatedBoundary`).
    pub fn animated_boundary(&mut self, target: Obj, colors: Vec<Color>, cycle: f64) -> Obj {
        self.always_redraw(move |ctx| {
            let b = ctx.get(target).bounds_or_origin().expand(0.15);
            let base = rectangle(b.width(), b.height()).move_to(b.center());
            let n = colors.len().max(1);
            let phase = (ctx.elapsed / cycle).fract();
            Mobject::group((0..n).map(|i| {
                let a = (phase + i as f64 / n as f64).fract();
                base.clone()
                    .partial(a, (a + 0.5 / n as f64).min(1.0))
                    .stroke(colors[i], 5.0)
            }))
        })
    }
    /// A line tracing where `target`'s centre has been (manim's `TracedPath`).
    /// `fade_after`: only show the last N seconds, fading (dissipating trail).
    pub fn traced_path(
        &mut self,
        target: Obj,
        color: Color,
        width: f32,
        fade_after: Option<f64>,
    ) -> Obj {
        let t0 = self.time();
        self.always_redraw(move |ctx| {
            let dt = 1.0 / 30.0;
            let start = match fade_after {
                Some(w) => (ctx.t - w).max(t0),
                None => t0,
            };
            let n = ((ctx.t - start) / dt).floor() as usize;
            if n < 2 {
                return Mobject::empty();
            }
            let pts: Vec<Vec2> = (0..=n)
                .map(|i| {
                    let t = (start + i as f64 * dt).min(ctx.t);
                    ctx.get_at(target, t).center()
                })
                .collect();
            match fade_after {
                None => polyline(&pts).stroke(color, width),
                Some(_) => Mobject::group(pts.windows(2).enumerate().map(|(i, w)| {
                    let k = (i as f32 + 1.0) / (pts.len() as f32);
                    line(w[0], w[1]).stroke(color, width * k).stroke_opacity(k)
                })),
            }
        })
    }
    /// Keep `follower` at a fixed offset from `leader` (manim's `MaintainPositionRelativeTo`).
    pub fn maintain_position_relative_to(&mut self, follower: Obj, leader: Obj) {
        let offset = self.get(follower).center() - self.get(leader).center();
        self.add_updater(follower, move |m, ctx| {
            let target = ctx.get(leader).center() + offset;
            let c = m.center();
            m.shift(target - c)
        });
    }
}

// ---------------------------------------------------------------- mobjects

/// Star polygon {n/k} (manim's `RegularPolygram`).
pub fn regular_polygram(n: usize, density: usize, r: f64) -> Mobject {
    let pts: Vec<Vec2> = (0..n)
        .map(|i| Vec2::polar(r, PI / 2.0 + TAU * i as f64 / n as f64))
        .collect();
    let k = density.max(1);
    let g = gcd(n, k);
    let mut path = Path::new();
    for s in 0..g {
        let mut idx = s;
        path.move_to(pts[idx]);
        for _ in 0..n / g {
            idx = (idx + k) % n;
            path.line_to(pts[idx]);
        }
        path.close();
    }
    Mobject::from_path(path)
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Several closed polylines as one object (manim's `Polygram`).
pub fn polygram(loops: &[Vec<Vec2>]) -> Mobject {
    let mut p = Path::new();
    for l in loops {
        p.subpaths.extend(Path::polyline(l, true).subpaths);
    }
    Mobject::from_path(p)
}

/// Polygon whose edges are arcs bulging by `angle` (manim's `ArcPolygon`).
pub fn arc_polygon(points: &[Vec2], angle: f64) -> Mobject {
    let mut p = Path::new();
    if points.is_empty() {
        return Mobject::empty();
    }
    p.move_to(points[0]);
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let seg = arc_between_path(a, b, angle);
        if let (Some(last), Some(sp)) = (p.subpaths.last_mut(), seg.subpaths.first()) {
            last.segs.extend(sp.segs.iter().copied());
        }
    }
    p.close();
    Mobject::from_path(p)
}

/// Right-angle "elbow" mark (manim's `Elbow`).
pub fn elbow(size: f64, angle: f64) -> Mobject {
    polyline(&[v2(size, 0.0), v2(size, size), v2(0.0, size)]).rotate_about(angle, Vec2::ZERO)
}

/// Arc tangent to a circle from an external point (manim's `TangentialArc` simplified).
pub fn tangent_lines_to_circle(p: Vec2, center: Vec2, r: f64) -> Mobject {
    let d = p.distance(center);
    if d <= r {
        return Mobject::empty();
    }
    let a = (r / d).asin();
    let base = (center - p).angle();
    let len = (d * d - r * r).sqrt();
    Mobject::group([
        line(p, p + Vec2::from_angle(base + a) * len),
        line(p, p + Vec2::from_angle(base - a) * len),
    ])
}

/// Red X (manim's `Exmark`).
pub fn exmark(size: f64) -> Mobject {
    cross(size)
}

/// Rectangle with the frame's aspect ratio (manim's `ScreenRectangle`).
pub fn screen_rectangle(height: f64) -> Mobject {
    let (w, h) = frame_size();
    rectangle(height * w / h, height)
}

/// Rectangle covering the whole frame (manim's `FullScreenRectangle`).
pub fn full_screen_rectangle(color: Color, opacity: f32) -> Mobject {
    let (w, h) = frame_size();
    rectangle(w, h).fill(color, opacity).no_stroke()
}

/// Cut a path into real dash geometry (manim's `DashedVMobject`) — unlike a dash
/// *style*, the dashes can be created/animated individually.
pub fn dashed_mobject(m: &Mobject, num_dashes: usize, dash_ratio: f64) -> Mobject {
    let p = m.family_path();
    let n = num_dashes.max(1);
    let style = m
        .leaves()
        .first()
        .map(|l| l.style.clone())
        .unwrap_or_default();
    Mobject::group((0..n).map(|i| {
        let a = i as f64 / n as f64;
        let b = a + dash_ratio / n as f64;
        Mobject::from_path_styled(p.partial(a, b.min(1.0)), style.clone())
    }))
}

/// One child per bezier segment (manim's `CurvesAsSubmobjects`) — great with
/// `color_by_gradient`.
pub fn curves_as_submobjects(m: &Mobject) -> Mobject {
    let mut g = Mobject::empty();
    let style = m
        .leaves()
        .first()
        .map(|l| l.style.clone())
        .unwrap_or_default();
    for sp in &m.family_path().subpaths {
        for i in 0..sp.segs.len() {
            let mut s = SubPath::new(sp.seg_start(i));
            s.segs.push(sp.segs[i]);
            g.push(Mobject::from_path_styled(
                Path { subpaths: vec![s] },
                style.clone(),
            ));
        }
    }
    g
}

/// Soft glowing dot (manim's `GlowDot`).
pub fn glow_dot(p: Vec2, r: f64, color: Color) -> Mobject {
    let mut halo = circle_at(p, r * 3.0).no_stroke();
    halo.style.fill = Paint::Radial {
        center: p,
        radius: r * 3.0,
        focal: p,
        stops: vec![
            (0.0, color.with_alpha(0.9)),
            (0.35, color.with_alpha(0.35)),
            (1.0, color.with_alpha(0.0)),
        ],
    };
    halo.style.fill_opacity = 1.0;
    halo.style.blend = Blend::Screen;
    Mobject::group([halo, dot_r(p, r * 0.6, color.lighter(0.5))])
}

/// Many small dots (manim's `DotCloud`).
pub fn dot_cloud(points: &[Vec2], r: f64, color: Color) -> Mobject {
    Mobject::group(points.iter().map(|p| dot_r(*p, r, color)))
}

/// Dot with a coloured outline ring (manim's `AnnotationDot`).
pub fn annotation_dot(p: Vec2, fill: Color, ring: Color) -> Mobject {
    circle_at(p, 0.13).fill(fill, 1.0).stroke(ring, 4.0)
}

/// Layered widening strokes behind an object (manim's `VHighlight`).
pub fn v_highlight(m: &Mobject, color: Color, layers: usize) -> Mobject {
    let mut g = Mobject::empty();
    for i in (1..=layers).rev() {
        let k = i as f32 / layers as f32;
        g.push(
            m.clone()
                .stroke(color, 4.0 + 10.0 * k)
                .stroke_opacity(0.18 * (1.0 - k) + 0.05)
                .fill_opacity(0.0),
        );
    }
    g.push(m.clone());
    g
}

/// Probability sample space: a square split by a proportion (manim's `SampleSpace`).
pub fn sample_space(p: f64, labels: (&str, &str), colors: (Color, Color), side: f64) -> Mobject {
    let left = rectangle(side * p, side)
        .move_to(v2(-side / 2.0 + side * p / 2.0, 0.0))
        .fill(colors.0, 0.8)
        .stroke(WHITE, 2.0);
    let right = rectangle(side * (1.0 - p), side)
        .move_to(v2(side / 2.0 - side * (1.0 - p) / 2.0, 0.0))
        .fill(colors.1, 0.8)
        .stroke(WHITE, 2.0);
    let la = crate::tex::tex_sized(labels.0, 0.45, WHITE).move_to(left.center());
    let lb = crate::tex::tex_sized(labels.1, 0.45, WHITE).move_to(right.center());
    Mobject::group([left, right, la, lb])
}

// ----------------------------------------------------------------- fractals

fn fractal_image(w: u32, h: u32, f: impl Fn(f64, f64) -> Color + Sync) -> tiny_skia::Pixmap {
    use rayon::prelude::*;
    let mut data = vec![0u8; (w * h * 4) as usize];
    data.par_chunks_mut((w * 4) as usize)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w as usize {
                let c = f(x as f64 / w as f64, y as f64 / h as f64).to_rgba8();
                row[x * 4..x * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        });
    tiny_skia::Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h).unwrap()).unwrap()
}

fn smooth_escape(mut z: Vec2, c: Vec2, max_iter: u32) -> Option<f64> {
    for i in 0..max_iter {
        z = v2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
        let m = z.length_sq();
        if m > 256.0 {
            return Some(i as f64 + 1.0 - (m.ln() / 2.0).ln() / std::f64::consts::LN_2);
        }
    }
    None
}

fn palette_at(colors: &[Color], v: f64) -> Color {
    color_gradient(colors, (v * 0.03).fract())
}

/// Mandelbrot set image over the complex rectangle `center ± (width/2, height/2)`.
pub fn mandelbrot(
    center: Vec2,
    width: f64,
    px: u32,
    max_iter: u32,
    colors: &[Color],
    scene_width: f64,
) -> Mobject {
    let (fw, fh) = frame_size();
    let aspect = fh / fw;
    let h = (px as f64 * aspect) as u32;
    let pal = if colors.is_empty() {
        vec![BLUE_E, TEAL, YELLOW, RED, PURPLE, BLUE_E]
    } else {
        colors.to_vec()
    };
    let img = fractal_image(px, h, |u, v| {
        let c = center + v2((u - 0.5) * width, (0.5 - v) * width * aspect);
        match smooth_escape(Vec2::ZERO, c, max_iter) {
            Some(n) => palette_at(&pal, n),
            None => BLACK,
        }
    });
    Mobject::image(Arc::new(img), scene_width)
}

/// Julia set image for parameter `c`.
pub fn julia(
    c: Vec2,
    width: f64,
    px: u32,
    max_iter: u32,
    colors: &[Color],
    scene_width: f64,
) -> Mobject {
    let (fw, fh) = frame_size();
    let aspect = fh / fw;
    let h = (px as f64 * aspect) as u32;
    let pal = if colors.is_empty() {
        vec![PURPLE_E, PINK, GOLD, WHITE, PURPLE_E]
    } else {
        colors.to_vec()
    };
    let img = fractal_image(px, h, |u, v| {
        let z = v2((u - 0.5) * width, (0.5 - v) * width * aspect);
        match smooth_escape(z, c, max_iter) {
            Some(n) => palette_at(&pal, n),
            None => BLACK,
        }
    });
    Mobject::image(Arc::new(img), scene_width)
}

/// Newton fractal for z³ − 1 (roots coloured, shaded by iteration count).
pub fn newton_fractal(width: f64, px: u32, scene_width: f64) -> Mobject {
    let (fw, fh) = frame_size();
    let aspect = fh / fw;
    let h = (px as f64 * aspect) as u32;
    let roots = [
        v2(1.0, 0.0),
        Vec2::polar(1.0, TAU / 3.0),
        Vec2::polar(1.0, -TAU / 3.0),
    ];
    let cols = [BLUE, YELLOW, RED];
    let img = fractal_image(px, h, |u, v| {
        let mut z = v2((u - 0.5) * width, (0.5 - v) * width * aspect);
        for i in 0..40 {
            let z2 = complex::mul(z, z);
            let z3 = complex::mul(z2, z);
            z -= complex::div(z3 - v2(1.0, 0.0), z2 * 3.0);
            for (k, r) in roots.iter().enumerate() {
                if (z - *r).length() < 1e-3 {
                    return cols[k].darker((i as f32 / 40.0).min(0.8));
                }
            }
        }
        BLACK
    });
    Mobject::image(Arc::new(img), scene_width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::text;

    #[test]
    fn extra_anims_compile_and_run() {
        let mut s = Scene::new();
        let a = s.register(square(1.0));
        let b = s.register(circle(0.5).shift(RIGHT * 2.0));
        let t = s.register(text("Hello world again"));
        s.play(fade_in_from_point(a, v2(-3.0, 0.0)));
        s.play(spiral_in(b));
        s.play(add_text_word_by_word(t));
        assert_eq!(*word_groups(&s.get(t)).last().unwrap(), 2);
        s.play(turn_inside_out(a));
        s.save_state(a);
        s.play(a.animate().shift(UP));
        let r = s.restore(a);
        s.play(r);
        assert!(s.get(a).center().length() < 1e-6);
        let sw = s.swap(a, b);
        s.play(sw);
        assert!((s.get(a).center() - v2(2.0, 0.0)).length() < 1e-6);
        let br = s.broadcast(Vec2::ZERO, BLUE, 3, 2.0);
        s.play(br);
        let tp = s.traced_path(b, YELLOW, 3.0, Some(1.0));
        s.play(b.animate().shift(UP * 2.0));
        assert!(s.eval_obj(tp, s.time() - 0.1, 0).leaf_count() > 3);
        s.play(type_with_cursor(t, WHITE));
        s.play(phase_flow(a, |p| p.perp(), 1.0));
        s.play(show_creation_then_destruction(b));
        s.play(fade_out_to_point(a, Vec2::ZERO));
        let bd = s.animated_boundary(t, vec![RED, GREEN, BLUE], 2.0);
        s.wait(1.0);
        assert!(s.eval_obj(bd, s.time() - 0.5, 0).leaf_count() == 3);
    }

    #[test]
    fn extra_mobjects() {
        assert_eq!(regular_polygram(5, 2, 1.0).path.subpaths.len(), 1);
        assert_eq!(regular_polygram(6, 2, 1.0).path.subpaths.len(), 2);
        assert!(arc_polygon(&[v2(0.0, 0.0), v2(1.0, 0.0), v2(0.5, 1.0)], 0.5).length() > 3.0);
        assert_eq!(dashed_mobject(&circle(1.0), 10, 0.5).children.len(), 10);
        assert!(curves_as_submobjects(&circle(1.0)).children.len() >= 4);
        assert!(glow_dot(Vec2::ZERO, 0.1, BLUE).leaf_count() == 2);
        let m = mandelbrot(v2(-0.5, 0.0), 3.0, 64, 50, &[], 4.0);
        assert!(m.image.is_some());
        assert!(julia(v2(-0.8, 0.156), 3.0, 32, 50, &[], 4.0)
            .image
            .is_some());
        assert!(newton_fractal(3.0, 32, 4.0).image.is_some());
        assert_eq!(
            tangent_lines_to_circle(v2(3.0, 0.0), Vec2::ZERO, 1.0)
                .children
                .len(),
            2
        );
        assert_eq!(
            sample_space(0.3, ("A", "B"), (BLUE, RED), 3.0)
                .children
                .len(),
            4
        );
    }
}
