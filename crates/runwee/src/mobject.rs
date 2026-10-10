//! The `Mobject` tree: the universal visual object (shapes, text, images, groups).

use crate::color::*;
use crate::math::*;
use crate::path::*;
use crate::style::*;
use std::sync::{Arc, RwLock};

pub const SMALL_BUFF: f64 = 0.1;
pub const MED_SMALL_BUFF: f64 = 0.25;
pub const MED_LARGE_BUFF: f64 = 0.5;
pub const LARGE_BUFF: f64 = 1.0;
pub const DEFAULT_BUFF: f64 = MED_SMALL_BUFF;
pub const EDGE_BUFF: f64 = MED_LARGE_BUFF;

static FRAME: RwLock<(f64, f64)> = RwLock::new((8.0 * 16.0 / 9.0, 8.0));

/// Current logical frame size (width, height) in scene units. Defaults to 16:9, 8 units tall.
pub fn frame_size() -> (f64, f64) {
    *FRAME.read().unwrap()
}
/// Change the logical frame size (e.g. for 9:16 vertical videos). Set by `Scene` automatically.
pub fn set_frame_size(w: f64, h: f64) {
    *FRAME.write().unwrap() = (w, h);
}
pub fn frame_width() -> f64 {
    frame_size().0
}
pub fn frame_height() -> f64 {
    frame_size().1
}

/// A raster image placed in scene space by an affine frame.
#[derive(Clone)]
pub struct ImageData {
    pub pixmap: Arc<tiny_skia::Pixmap>,
    /// Bottom-left corner in scene space.
    pub origin: Vec2,
    /// Scene-space vector along the image's width.
    pub x_axis: Vec2,
    /// Scene-space vector along the image's height (pointing up).
    pub y_axis: Vec2,
    pub opacity: f32,
}

impl std::fmt::Debug for ImageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ImageData({}x{} at {:?})",
            self.pixmap.width(),
            self.pixmap.height(),
            self.origin
        )
    }
}

impl PartialEq for ImageData {
    fn eq(&self, o: &Self) -> bool {
        Arc::ptr_eq(&self.pixmap, &o.pixmap)
            && self.origin == o.origin
            && self.x_axis == o.x_axis
            && self.y_axis == o.y_axis
            && self.opacity == o.opacity
    }
}

impl ImageData {
    pub fn corners(&self) -> [Vec2; 4] {
        [
            self.origin,
            self.origin + self.x_axis,
            self.origin + self.x_axis + self.y_axis,
            self.origin + self.y_axis,
        ]
    }
    pub fn transform(&mut self, a: &Affine) {
        let o = a.apply(self.origin);
        self.x_axis = a.apply(self.origin + self.x_axis) - o;
        self.y_axis = a.apply(self.origin + self.y_axis) - o;
        self.origin = o;
    }
}

/// A node in the visual tree. A node may carry its own path (drawn first), an image,
/// and any number of children (drawn after, in order).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mobject {
    pub path: Path,
    pub style: Style,
    pub image: Option<ImageData>,
    pub children: Vec<Mobject>,
    /// Free-form tag (TeX source of a formula part, element name, label…).
    pub tag: Option<Arc<str>>,
}

/// Build a group from children.
pub fn group(children: impl IntoIterator<Item = Mobject>) -> Mobject {
    Mobject::group(children)
}

impl Mobject {
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn from_path(path: Path) -> Self {
        Self {
            path,
            ..Default::default()
        }
    }
    pub fn from_path_styled(path: Path, style: Style) -> Self {
        Self {
            path,
            style,
            ..Default::default()
        }
    }
    pub fn group(children: impl IntoIterator<Item = Mobject>) -> Self {
        Self {
            children: children.into_iter().collect(),
            ..Default::default()
        }
    }
    /// An image of `width` scene units centred at the origin.
    pub fn image(pixmap: Arc<tiny_skia::Pixmap>, width: f64) -> Self {
        let aspect = pixmap.height() as f64 / pixmap.width().max(1) as f64;
        let h = width * aspect;
        Self {
            image: Some(ImageData {
                pixmap,
                origin: v2(-width / 2.0, -h / 2.0),
                x_axis: v2(width, 0.0),
                y_axis: v2(0.0, h),
                opacity: 1.0,
            }),
            style: Style {
                stroke_width: 0.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    // ------------------------------------------------------------------ tree

    pub fn with_child(mut self, child: Mobject) -> Self {
        self.children.push(child);
        self
    }
    pub fn push(&mut self, child: Mobject) {
        self.children.push(child);
    }
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tag = Some(Arc::from(tag));
        self
    }
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }
    pub fn len(&self) -> usize {
        self.children.len()
    }
    pub fn is_empty(&self) -> bool {
        self.children.is_empty() && self.path.is_empty() && self.image.is_none()
    }
    /// Child `i` (negative indices count from the end). Returns an empty mobject if out of range.
    pub fn child(&self, i: i64) -> Mobject {
        let n = self.children.len() as i64;
        let idx = if i < 0 { n + i } else { i };
        if idx >= 0 && idx < n {
            self.children[idx as usize].clone()
        } else {
            Mobject::empty()
        }
    }
    /// Children `[a, b)` as a new group.
    pub fn slice(&self, a: usize, b: usize) -> Mobject {
        let b = b.min(self.children.len());
        Mobject::group(self.children[a.min(b)..b].iter().cloned())
    }
    /// Is this node drawable by itself?
    pub fn is_leaf_drawable(&self) -> bool {
        !self.path.subpaths.is_empty() || self.image.is_some()
    }
    /// Visit every node (pre-order).
    pub fn for_each(&self, f: &mut dyn FnMut(&Mobject)) {
        f(self);
        for c in &self.children {
            c.for_each(f);
        }
    }
    pub fn for_each_mut(&mut self, f: &mut dyn FnMut(&mut Mobject)) {
        f(self);
        for c in &mut self.children {
            c.for_each_mut(f);
        }
    }
    /// Drawable nodes (without their children) in draw order.
    pub fn leaves(&self) -> Vec<Mobject> {
        let mut out = Vec::new();
        self.for_each(&mut |m| {
            if m.is_leaf_drawable() {
                out.push(Mobject {
                    path: m.path.clone(),
                    style: m.style.clone(),
                    image: m.image.clone(),
                    children: vec![],
                    tag: m.tag.clone(),
                })
            }
        });
        out
    }
    pub fn leaf_count(&self) -> usize {
        let mut n = 0;
        self.for_each(&mut |m| {
            if m.is_leaf_drawable() {
                n += 1
            }
        });
        n
    }
    /// Flattened copy: a group whose children are all the drawable leaves.
    pub fn flattened(&self) -> Mobject {
        Mobject::group(self.leaves())
    }

    // -------------------------------------------------------------- geometry

    pub fn bounds(&self) -> Option<Rect> {
        let mut r: Option<Rect> = None;
        self.for_each(&mut |m| {
            let mut add = |b: Rect| r = Some(r.map_or(b, |x| x.union(&b)));
            if let Some(b) = m.path.bounds() {
                add(b);
            }
            if let Some(img) = &m.image {
                let c = img.corners();
                let mut b = Rect::point(c[0]);
                for p in &c[1..] {
                    b.include(*p);
                }
                add(b);
            }
        });
        r
    }
    pub fn bounds_or_origin(&self) -> Rect {
        self.bounds().unwrap_or(Rect::point(Vec2::ZERO))
    }
    pub fn center(&self) -> Vec2 {
        self.bounds_or_origin().center()
    }
    pub fn width(&self) -> f64 {
        self.bounds_or_origin().width()
    }
    pub fn height(&self) -> f64 {
        self.bounds_or_origin().height()
    }
    /// manim's `get_critical_point`: e.g. `UP` → top centre, `UR` → top-right corner.
    pub fn edge(&self, dir: Vec2) -> Vec2 {
        self.bounds_or_origin().critical_point(dir)
    }
    pub fn top(&self) -> Vec2 {
        self.edge(UP)
    }
    pub fn bottom(&self) -> Vec2 {
        self.edge(DOWN)
    }
    pub fn left(&self) -> Vec2 {
        self.edge(LEFT)
    }
    pub fn right(&self) -> Vec2 {
        self.edge(RIGHT)
    }
    /// First point of the first drawable path.
    pub fn start_point(&self) -> Vec2 {
        let mut p = None;
        self.for_each(&mut |m| {
            if p.is_none() {
                p = m.path.start();
            }
        });
        p.unwrap_or_else(|| self.center())
    }
    /// Last point of the last drawable path.
    pub fn end_point(&self) -> Vec2 {
        let mut p = None;
        self.for_each(&mut |m| {
            if let Some(e) = m.path.end() {
                p = Some(e);
            }
        });
        p.unwrap_or_else(|| self.center())
    }
    /// All path anchors of the family.
    pub fn anchors(&self) -> Vec<Vec2> {
        let mut v = Vec::new();
        self.for_each(&mut |m| v.extend(m.path.anchors()));
        v
    }
    /// Concatenated path of the whole family.
    pub fn family_path(&self) -> Path {
        let mut p = Path::new();
        self.for_each(&mut |m| p.subpaths.extend(m.path.subpaths.iter().cloned()));
        p
    }
    pub fn point_from_proportion(&self, a: f64) -> Vec2 {
        self.family_path().point_from_proportion(a)
    }
    pub fn length(&self) -> f64 {
        self.family_path().length()
    }

    // ------------------------------------------------------------ transforms

    pub fn apply_affine_mut(&mut self, a: &Affine) {
        let s = a.mean_scale();
        self.for_each_mut(&mut |m| {
            m.path.transform(a);
            m.style.transform(a);
            if let Some(d) = &mut m.style.dash {
                d.0 *= s;
                d.1 *= s;
            }
            if let Some(img) = &mut m.image {
                img.transform(a);
            }
        });
    }
    pub fn apply_affine(mut self, a: Affine) -> Self {
        self.apply_affine_mut(&a);
        self
    }
    /// Apply an arbitrary point-wise function (manim's `apply_function`).
    pub fn apply_function(mut self, f: impl Fn(Vec2) -> Vec2) -> Self {
        self.apply_function_mut(&f);
        self
    }
    pub fn apply_function_mut(&mut self, f: &dyn Fn(Vec2) -> Vec2) {
        self.for_each_mut(&mut |m| {
            m.path.map_points(f);
            if let Some(img) = &mut m.image {
                let o = f(img.origin);
                img.x_axis = f(img.origin + img.x_axis) - o;
                img.y_axis = f(img.origin + img.y_axis) - o;
                img.origin = o;
            }
        });
    }
    pub fn shift_mut(&mut self, v: Vec2) {
        self.apply_affine_mut(&Affine::translate(v));
    }
    pub fn shift(mut self, v: Vec2) -> Self {
        self.shift_mut(v);
        self
    }
    pub fn move_to(self, p: Vec2) -> Self {
        let c = self.center();
        self.shift(p - c)
    }
    /// Move so that the critical point `aligned_edge` lands on `p`.
    pub fn move_to_aligned(self, p: Vec2, aligned_edge: Vec2) -> Self {
        let c = self.edge(aligned_edge);
        self.shift(p - c)
    }
    pub fn scale(self, s: f64) -> Self {
        let c = self.center();
        self.scale_about(s, c)
    }
    pub fn scale_about(self, s: f64, about: Vec2) -> Self {
        self.apply_affine(Affine::scale(s, s).about(about))
    }
    pub fn stretch(self, sx: f64, sy: f64) -> Self {
        let c = self.center();
        self.apply_affine(Affine::scale(sx, sy).about(c))
    }
    pub fn rotate(self, angle: f64) -> Self {
        let c = self.center();
        self.rotate_about(angle, c)
    }
    pub fn rotate_about(self, angle: f64, about: Vec2) -> Self {
        self.apply_affine(Affine::rotate(angle).about(about))
    }
    /// Mirror across the axis through the centre along `axis` (`UP` flips left↔right).
    pub fn flip(self, axis: Vec2) -> Self {
        let c = self.center();
        let a = axis.normalize();
        // reflection matrix across line with direction a
        let (x, y) = (a.x, a.y);
        let m = Affine {
            a: 2.0 * x * x - 1.0,
            b: 2.0 * x * y,
            c: 2.0 * x * y,
            d: 2.0 * y * y - 1.0,
            e: 0.0,
            f: 0.0,
        };
        self.apply_affine(m.about(c))
    }
    pub fn set_width(self, w: f64) -> Self {
        let cw = self.width();
        if cw < 1e-12 {
            return self;
        }
        self.scale(w / cw)
    }
    pub fn set_height(self, h: f64) -> Self {
        let ch = self.height();
        if ch < 1e-12 {
            return self;
        }
        self.scale(h / ch)
    }
    pub fn stretch_to_fit_width(self, w: f64) -> Self {
        let cw = self.width();
        if cw < 1e-12 {
            return self;
        }
        self.stretch(w / cw, 1.0)
    }
    pub fn stretch_to_fit_height(self, h: f64) -> Self {
        let ch = self.height();
        if ch < 1e-12 {
            return self;
        }
        self.stretch(1.0, h / ch)
    }
    /// Scale down (never up) so the object fits inside `w × h`.
    pub fn fit_within(self, w: f64, h: f64) -> Self {
        let (cw, ch) = (self.width(), self.height());
        let s = (w / cw.max(1e-12)).min(h / ch.max(1e-12));
        if s < 1.0 {
            self.scale(s)
        } else {
            self
        }
    }

    // ---------------------------------------------------------------- layout

    /// Place next to a point/rect in direction `dir` with spacing `buff`.
    pub fn next_to_rect(self, target: Rect, dir: Vec2, buff: f64, aligned_edge: Vec2) -> Self {
        let tp = target.critical_point(dir + aligned_edge);
        let sp = self.edge(-dir + aligned_edge);
        self.shift(tp - sp + dir * buff)
    }
    /// manim's `next_to(mobject, direction, buff)`.
    pub fn next_to(self, target: &Mobject, dir: Vec2, buff: f64) -> Self {
        self.next_to_rect(target.bounds_or_origin(), dir, buff, Vec2::ZERO)
    }
    pub fn next_to_aligned(
        self,
        target: &Mobject,
        dir: Vec2,
        buff: f64,
        aligned_edge: Vec2,
    ) -> Self {
        self.next_to_rect(target.bounds_or_origin(), dir, buff, aligned_edge)
    }
    pub fn next_to_point(self, p: Vec2, dir: Vec2, buff: f64) -> Self {
        self.next_to_rect(Rect::point(p), dir, buff, Vec2::ZERO)
    }
    /// Align the edge in `dir` with the same edge of `target`.
    pub fn align_to(self, target: &Mobject, dir: Vec2) -> Self {
        let t = target.edge(dir);
        let s = self.edge(dir);
        let d = v2(
            if dir.x.abs() > 1e-9 { t.x - s.x } else { 0.0 },
            if dir.y.abs() > 1e-9 { t.y - s.y } else { 0.0 },
        );
        self.shift(d)
    }
    pub fn to_edge(self, dir: Vec2, buff: f64) -> Self {
        let (w, h) = frame_size();
        let frame = Rect::from_center(Vec2::ZERO, w, h);
        let target = frame.critical_point(dir);
        let s = self.edge(dir);
        let d = v2(
            if dir.x.abs() > 1e-9 {
                target.x - s.x - dir.x.signum() * buff
            } else {
                0.0
            },
            if dir.y.abs() > 1e-9 {
                target.y - s.y - dir.y.signum() * buff
            } else {
                0.0
            },
        );
        self.shift(d)
    }
    pub fn to_corner(self, dir: Vec2, buff: f64) -> Self {
        self.to_edge(dir, buff)
    }
    pub fn center_on_screen(self) -> Self {
        self.move_to(Vec2::ZERO)
    }
    /// Lay children out one after another along `dir`, then centre the group.
    pub fn arrange(mut self, dir: Vec2, buff: f64) -> Self {
        let c = self.center();
        for i in 1..self.children.len() {
            let prev = self.children[i - 1].bounds_or_origin();
            let cur = std::mem::take(&mut self.children[i]);
            self.children[i] = cur.next_to_rect(prev, dir, buff, Vec2::ZERO);
        }
        self.move_to(c)
    }
    /// Arrange along `dir` aligning children on `aligned_edge` (e.g. `LEFT` for left-aligned columns).
    pub fn arrange_aligned(mut self, dir: Vec2, buff: f64, aligned_edge: Vec2) -> Self {
        let c = self.center();
        for i in 1..self.children.len() {
            let prev = self.children[i - 1].bounds_or_origin();
            let cur = std::mem::take(&mut self.children[i]);
            self.children[i] = cur.next_to_rect(prev, dir, buff, aligned_edge);
        }
        self.move_to(c)
    }
    /// Grid layout with `cols` columns (cells sized to the largest child).
    pub fn arrange_in_grid(mut self, cols: usize, h_buff: f64, v_buff: f64) -> Self {
        let cols = cols.max(1);
        let n = self.children.len();
        if n == 0 {
            return self;
        }
        let rows = n.div_ceil(cols);
        let cw = self.children.iter().map(|c| c.width()).fold(0.0, f64::max);
        let ch = self.children.iter().map(|c| c.height()).fold(0.0, f64::max);
        let total_w = cols as f64 * cw + (cols - 1) as f64 * h_buff;
        let total_h = rows as f64 * ch + (rows - 1) as f64 * v_buff;
        let c = self.center();
        for (i, child) in self.children.iter_mut().enumerate() {
            let (r, k) = (i / cols, i % cols);
            let x = -total_w / 2.0 + cw / 2.0 + k as f64 * (cw + h_buff);
            let y = total_h / 2.0 - ch / 2.0 - r as f64 * (ch + v_buff);
            let m = std::mem::take(child);
            *child = m.move_to(c + v2(x, y));
        }
        self
    }

    // ----------------------------------------------------------------- style

    pub fn set_style_mut(&mut self, f: &dyn Fn(&mut Style)) {
        self.for_each_mut(&mut |m| f(&mut m.style));
    }
    pub fn map_style(mut self, f: impl Fn(&mut Style)) -> Self {
        self.set_style_mut(&f);
        self
    }
    /// Set both fill and stroke colour (opacities unchanged).
    pub fn color(self, c: Color) -> Self {
        self.map_style(|s| {
            s.fill = Paint::Solid(c);
            s.stroke = Paint::Solid(c);
        })
    }
    pub fn fill(self, c: Color, opacity: f32) -> Self {
        self.map_style(|s| {
            s.fill = Paint::Solid(c);
            s.fill_opacity = opacity;
        })
    }
    pub fn fill_paint(self, p: Paint, opacity: f32) -> Self {
        self.map_style(|s| {
            s.fill = p.clone();
            s.fill_opacity = opacity;
        })
    }
    pub fn fill_opacity(self, opacity: f32) -> Self {
        self.map_style(|s| s.fill_opacity = opacity)
    }
    pub fn stroke(self, c: Color, width: f32) -> Self {
        self.map_style(|s| {
            s.stroke = Paint::Solid(c);
            s.stroke_width = width;
        })
    }
    pub fn stroke_color(self, c: Color) -> Self {
        self.map_style(|s| s.stroke = Paint::Solid(c))
    }
    pub fn stroke_width(self, w: f32) -> Self {
        self.map_style(|s| s.stroke_width = w)
    }
    pub fn stroke_opacity(self, o: f32) -> Self {
        self.map_style(|s| s.stroke_opacity = o)
    }
    pub fn no_stroke(self) -> Self {
        self.stroke_width(0.0)
    }
    /// Whole-object opacity multiplier.
    pub fn opacity(mut self, o: f32) -> Self {
        self.for_each_mut(&mut |m| {
            m.style.opacity = o;
            if let Some(i) = &mut m.image {
                i.opacity = o;
            }
        });
        self
    }
    pub fn fade(mut self, amount: f32) -> Self {
        self.for_each_mut(&mut |m| {
            m.style.opacity *= 1.0 - amount;
            if let Some(i) = &mut m.image {
                i.opacity *= 1.0 - amount;
            }
        });
        self
    }
    pub fn dashed(self, dash: f64, gap: f64) -> Self {
        self.map_style(|s| s.dash = Some((dash, gap)))
    }
    pub fn glow(self, radius: f64, color: Option<Color>) -> Self {
        self.map_style(|s| {
            s.glow = radius;
            s.glow_color = color;
        })
    }
    pub fn shadow(self, shadow: Shadow) -> Self {
        self.map_style(|s| s.shadow = Some(shadow))
    }
    pub fn background_stroke(self, color: Color, width: f32) -> Self {
        self.map_style(|s| {
            s.background_stroke = color;
            s.background_stroke_width = width;
        })
    }
    pub fn blend(self, b: Blend) -> Self {
        self.map_style(|s| s.blend = b)
    }
    pub fn line_cap(self, c: LineCap) -> Self {
        self.map_style(|s| s.line_cap = c)
    }
    /// Colour each drawable leaf along a gradient (manim's `set_color_by_gradient`).
    pub fn color_by_gradient(mut self, colors: &[Color]) -> Self {
        let n = self.leaf_count();
        let cs = color_range(colors, n);
        let mut i = 0;
        self.for_each_mut(&mut |m| {
            if m.is_leaf_drawable() {
                let c = cs[i.min(cs.len() - 1)];
                m.style.fill = Paint::Solid(c);
                m.style.stroke = Paint::Solid(c);
                i += 1;
            }
        });
        self
    }
    /// Linear gradient across the whole object in direction `dir` for both fill and stroke.
    pub fn gradient(self, colors: &[Color], dir: Vec2) -> Self {
        let b = self.bounds_or_origin();
        let start = b.critical_point(-dir);
        let end = b.critical_point(dir);
        let p = Paint::linear(start, end, colors);
        self.map_style(|s| {
            s.fill = p.clone();
            s.stroke = p.clone();
        })
    }
    /// Radial "sheen" fill – a lit sphere look, highlight towards `light_dir`.
    pub fn sheen(self, base: Color, light_dir: Vec2) -> Self {
        let b = self.bounds_or_origin();
        let c = b.center();
        let r = b.width().max(b.height()) * 0.6;
        let focal = c + light_dir.normalize() * r * 0.45;
        let p = Paint::Radial {
            center: focal,
            radius: r * 1.2,
            focal,
            stops: vec![
                (0.0, base.lighter(0.55)),
                (0.45, base),
                (1.0, base.darker(0.55)),
            ],
        };
        self.map_style(|s| {
            s.fill = p.clone();
            s.fill_opacity = 1.0;
        })
    }
    pub fn fill_color(&self) -> Color {
        let mut c = None;
        self.for_each(&mut |m| {
            if c.is_none() && m.is_leaf_drawable() {
                c = Some(m.style.fill.color());
            }
        });
        c.unwrap_or(WHITE)
    }
    pub fn stroke_color_value(&self) -> Color {
        let mut c = None;
        self.for_each(&mut |m| {
            if c.is_none() && m.is_leaf_drawable() {
                c = Some(m.style.stroke.color());
            }
        });
        c.unwrap_or(WHITE)
    }

    // --------------------------------------------------------- interpolation

    /// Keep only the arc-length portion `[a, b]` of every drawable path (manim's
    /// `pointwise_become_partial`), for each leaf independently.
    pub fn partial(mut self, a: f64, b: f64) -> Self {
        self.for_each_mut(&mut |m| {
            if !m.path.subpaths.is_empty() {
                if b - a < 1e-6 {
                    m.path = Path::new();
                } else {
                    m.path = m.path.partial(a, b);
                }
            }
        });
        self
    }

    /// Prepare two objects for morphing: returns equally long lists of aligned leaves.
    pub fn align_for_transform(a: &Mobject, b: &Mobject) -> (Vec<Mobject>, Vec<Mobject>) {
        let mut la = a.leaves();
        let mut lb = b.leaves();
        let ca = a.center();
        let cb = b.center();
        if la.is_empty() {
            la.push(point_leaf(ca, lb.first()));
        }
        if lb.is_empty() {
            lb.push(point_leaf(cb, la.first()));
        }
        let n = la.len().max(lb.len());
        let la = repeat_to(la, n);
        let lb = repeat_to(lb, n);
        let mut out_a = Vec::with_capacity(n);
        let mut out_b = Vec::with_capacity(n);
        for (x, y) in la.into_iter().zip(lb) {
            let (pa, pb) = Path::align(&x.path, &y.path);
            out_a.push(Mobject { path: pa, ..x });
            out_b.push(Mobject { path: pb, ..y });
        }
        (out_a, out_b)
    }

    /// Interpolate two aligned leaves.
    pub fn lerp_leaf(a: &Mobject, b: &Mobject, t: f64) -> Mobject {
        let image = match (&a.image, &b.image) {
            (Some(x), Some(y)) => {
                let (src, op) = if t < 0.5 {
                    (x, x.opacity * (1.0 - 2.0 * t as f32))
                } else {
                    (y, y.opacity * (2.0 * t as f32 - 1.0))
                };
                Some(ImageData {
                    origin: x.origin.lerp(y.origin, t),
                    x_axis: x.x_axis.lerp(y.x_axis, t),
                    y_axis: x.y_axis.lerp(y.y_axis, t),
                    opacity: op,
                    pixmap: src.pixmap.clone(),
                })
            }
            (Some(x), None) => Some(ImageData {
                opacity: x.opacity * (1.0 - t as f32),
                ..x.clone()
            }),
            (None, Some(y)) => Some(ImageData {
                opacity: y.opacity * t as f32,
                ..y.clone()
            }),
            (None, None) => None,
        };
        Mobject {
            path: a.path.lerp(&b.path, t),
            style: a.style.lerp(&b.style, t),
            image,
            children: vec![],
            tag: if t < 0.5 {
                a.tag.clone()
            } else {
                b.tag.clone()
            },
        }
    }
}

fn point_leaf(p: Vec2, like: Option<&Mobject>) -> Mobject {
    let mut sp = SubPath::new(p);
    sp.segs.push(Cubic { c1: p, c2: p, p });
    let mut style = like.map(|m| m.style.clone()).unwrap_or_default();
    style.fill_opacity = 0.0;
    style.stroke_opacity = 0.0;
    Mobject {
        path: Path { subpaths: vec![sp] },
        style,
        ..Default::default()
    }
}

/// Repeat items so the list has length `n`, keeping order (manim's `repeat_submobjects`).
fn repeat_to(v: Vec<Mobject>, n: usize) -> Vec<Mobject> {
    let m = v.len();
    if m == n {
        return v;
    }
    (0..n).map(|i| v[i * m / n].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(side: f64) -> Mobject {
        let h = side / 2.0;
        Mobject::from_path(Path::polyline(
            &[v2(-h, -h), v2(h, -h), v2(h, h), v2(-h, h)],
            true,
        ))
    }

    #[test]
    fn layout_ops() {
        let a = square(2.0);
        let b = square(1.0).next_to(&a, RIGHT, 0.5);
        assert!((b.center() - v2(2.0, 0.0)).length() < 1e-9);
        let c = square(1.0).to_edge(LEFT, 0.5);
        assert!((c.left().x - (-frame_width() / 2.0 + 0.5)).abs() < 1e-9);
        let g = group([square(1.0), square(1.0), square(1.0)]).arrange(RIGHT, 0.5);
        assert!((g.width() - 4.0).abs() < 1e-9);
        assert!(g.center().length() < 1e-9);
        let s = square(1.0).scale(2.0).rotate(PI / 4.0);
        assert!((s.width() - 2.0 * 2f64.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn transform_alignment() {
        let a = square(1.0);
        let b = group([square(1.0).shift(LEFT), square(1.0).shift(RIGHT)]);
        let (x, y) = Mobject::align_for_transform(&a, &b);
        assert_eq!(x.len(), 2);
        assert_eq!(y.len(), 2);
        let mid = Mobject::lerp_leaf(&x[0], &y[0], 0.5);
        assert!((mid.center() - v2(-0.5, 0.0)).length() < 1e-6);
    }

    #[test]
    fn partial_draw() {
        let s = square(2.0).partial(0.0, 0.5);
        assert!((s.length() - 4.0).abs() < 1e-6);
    }
}
