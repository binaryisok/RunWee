//! 3D: camera (manim's `ThreeDCamera` with phi/theta/zoom, perspective), shaded
//! meshes projected to 2D with depth sorting, primitives (sphere, cube, prism,
//! cone, cylinder, torus, polyhedra, surfaces, 3D lines/arrows/dots, axes) and
//! scene helpers for orbiting / ambient camera rotation.

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::scene::{FrameCtx, Obj, Scene, Tracker};
use crate::shapes;
use crate::style::*;

/// Orbit camera. `phi` = angle from the +z axis, `theta` = azimuth (manim conventions:
/// phi = 0 looks straight down, theta = -90° shows the x axis pointing right).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera3D {
    pub phi: f64,
    pub theta: f64,
    pub gamma: f64,
    pub zoom: f64,
    /// Distance of the eye from the focal point (perspective strength); `f64::INFINITY` = orthographic.
    pub distance: f64,
    pub focus: Vec3,
    /// Direction *towards* the light in world space.
    pub light: Vec3,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            phi: 70.0 * DEGREES,
            theta: -45.0 * DEGREES,
            gamma: 0.0,
            zoom: 1.0,
            distance: 20.0,
            focus: Vec3::ZERO,
            light: v3(-3.0, -4.0, 8.0).normalize(),
        }
    }
}

impl Camera3D {
    pub fn new(phi_deg: f64, theta_deg: f64) -> Self {
        Self {
            phi: phi_deg * DEGREES,
            theta: theta_deg * DEGREES,
            ..Default::default()
        }
    }
    /// World → camera space (x right, y up, z towards the viewer).
    pub fn to_camera(&self, p: Vec3) -> Vec3 {
        let q = p - self.focus;
        // rotate about z by -(theta + 90°) so theta = -90° gives the standard view
        let a = -(self.theta + PI / 2.0);
        let (s, c) = a.sin_cos();
        let q = v3(q.x * c - q.y * s, q.x * s + q.y * c, q.z);
        // tilt about x by -phi
        let (s, c) = (-self.phi).sin_cos();
        let q = v3(q.x, q.y * c - q.z * s, q.y * s + q.z * c);
        // roll
        if self.gamma != 0.0 {
            let (s, c) = self.gamma.sin_cos();
            return v3(q.x * c - q.y * s, q.x * s + q.y * c, q.z);
        }
        q
    }
    /// Project to the 2D scene plane; returns (point, depth) where larger depth = closer.
    pub fn project(&self, p: Vec3) -> (Vec2, f64) {
        let q = self.to_camera(p);
        let k = if self.distance.is_finite() {
            self.distance / (self.distance - q.z).max(0.05)
        } else {
            1.0
        };
        (v2(q.x, q.y) * (k * self.zoom), q.z)
    }
    /// View direction (from the scene towards the eye) in world space.
    pub fn eye_dir(&self) -> Vec3 {
        let (sp, cp) = self.phi.sin_cos();
        let (st, ct) = self.theta.sin_cos();
        v3(sp * ct, sp * st, cp)
    }
    pub fn lerp(&self, o: &Camera3D, t: f64) -> Camera3D {
        Camera3D {
            phi: lerp(self.phi, o.phi, t),
            theta: lerp(self.theta, o.theta, t),
            gamma: lerp(self.gamma, o.gamma, t),
            zoom: lerp(self.zoom, o.zoom, t),
            distance: if self.distance.is_finite() && o.distance.is_finite() {
                lerp(self.distance, o.distance, t)
            } else {
                o.distance
            },
            focus: self.focus.lerp(o.focus, t),
            light: self.light.lerp(o.light, t).normalize(),
        }
    }
}

/// A shaded polygon.
#[derive(Clone, Debug)]
pub struct Face {
    pub verts: Vec<Vec3>,
    pub color: Color,
    pub opacity: f32,
    pub stroke: Option<(Color, f32)>,
}

/// A 3D line segment (curves are chains of segments).
#[derive(Clone, Debug)]
pub struct Seg3 {
    pub a: Vec3,
    pub b: Vec3,
    pub color: Color,
    pub width: f32,
}

/// A shaded sphere-ish dot rendered as a lit disc (atoms, particles).
#[derive(Clone, Debug)]
pub struct Ball {
    pub center: Vec3,
    pub radius: f64,
    pub color: Color,
}

/// A 2D mobject billboarded at a 3D point (labels).
#[derive(Clone, Debug)]
pub struct Label3 {
    pub at: Vec3,
    pub mobject: Mobject,
}

/// A collection of 3D primitives.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub faces: Vec<Face>,
    pub segs: Vec<Seg3>,
    pub balls: Vec<Ball>,
    pub labels: Vec<Label3>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn merge(mut self, o: Mesh) -> Self {
        self.faces.extend(o.faces);
        self.segs.extend(o.segs);
        self.balls.extend(o.balls);
        self.labels.extend(o.labels);
        self
    }
    pub fn add(&mut self, o: Mesh) {
        self.faces.extend(o.faces);
        self.segs.extend(o.segs);
        self.balls.extend(o.balls);
        self.labels.extend(o.labels);
    }
    pub fn map_points(mut self, f: impl Fn(Vec3) -> Vec3) -> Self {
        for fc in &mut self.faces {
            for v in &mut fc.verts {
                *v = f(*v);
            }
        }
        for s in &mut self.segs {
            s.a = f(s.a);
            s.b = f(s.b);
        }
        for b in &mut self.balls {
            b.center = f(b.center);
        }
        for l in &mut self.labels {
            l.at = f(l.at);
        }
        self
    }
    pub fn shift(self, v: Vec3) -> Self {
        self.map_points(move |p| p + v)
    }
    pub fn scale(self, s: f64) -> Self {
        self.map_points(move |p| p * s)
    }
    pub fn rotate(self, axis: Vec3, angle: f64) -> Self {
        self.map_points(move |p| p.rotate_axis(axis, angle))
    }
    pub fn color(mut self, c: Color) -> Self {
        for f in &mut self.faces {
            f.color = c;
        }
        for s in &mut self.segs {
            s.color = c;
        }
        for b in &mut self.balls {
            b.color = c;
        }
        self
    }
    pub fn opacity(mut self, o: f32) -> Self {
        for f in &mut self.faces {
            f.opacity = o;
        }
        self
    }
    /// Colour faces by height (manim's `set_fill_by_value` on z).
    pub fn color_by_z(mut self, colors: &[Color]) -> Self {
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for f in &self.faces {
            for v in &f.verts {
                lo = lo.min(v.z);
                hi = hi.max(v.z);
            }
        }
        for f in &mut self.faces {
            let z = f.verts.iter().map(|v| v.z).sum::<f64>() / f.verts.len().max(1) as f64;
            f.color = color_gradient(colors, inverse_lerp(lo, hi, z));
        }
        self
    }
    pub fn wireframe(mut self, c: Color, w: f32) -> Self {
        for f in &mut self.faces {
            f.stroke = Some((c, w));
        }
        self
    }

    /// Project to a 2D mobject with lighting and painter's-algorithm depth sorting.
    pub fn render(&self, cam: &Camera3D) -> Mobject {
        enum Item<'a> {
            F(&'a Face, f64),
            S(&'a Seg3),
            B(&'a Ball),
            L(&'a Label3),
        }
        let mut items: Vec<(f64, Item)> =
            Vec::with_capacity(self.faces.len() + self.segs.len() + self.balls.len());
        let eye = cam.eye_dir();
        for f in &self.faces {
            if f.verts.len() < 3 {
                continue;
            }
            let c = f.verts.iter().fold(Vec3::ZERO, |a, v| a + *v) / f.verts.len() as f64;
            let n = (f.verts[1] - f.verts[0])
                .cross(f.verts[2] - f.verts[0])
                .normalize();
            // two-sided lighting
            let n = if n.dot(eye) < 0.0 { -n } else { n };
            let diffuse = n.dot(cam.light).max(0.0);
            let shade = 0.35 + 0.65 * diffuse;
            items.push((cam.to_camera(c).z, Item::F(f, shade)));
        }
        for s in &self.segs {
            items.push((cam.to_camera((s.a + s.b) / 2.0).z, Item::S(s)));
        }
        for b in &self.balls {
            items.push((cam.to_camera(b.center).z + b.radius * 0.5, Item::B(b)));
        }
        for l in &self.labels {
            items.push((cam.to_camera(l.at).z + 0.01, Item::L(l)));
        }
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut out = Mobject::empty();
        for (_, it) in items {
            match it {
                Item::F(f, shade) => {
                    let pts: Vec<Vec2> = f.verts.iter().map(|v| cam.project(*v).0).collect();
                    let col = Color {
                        r: f.color.r * shade as f32,
                        g: f.color.g * shade as f32,
                        b: f.color.b * shade as f32,
                        a: f.color.a,
                    };
                    let mut m = Mobject::from_path(Path::polyline(&pts, true));
                    m.style.fill = Paint::Solid(col);
                    m.style.fill_opacity = f.opacity;
                    match f.stroke {
                        Some((sc, w)) => {
                            m.style.stroke = Paint::Solid(sc);
                            m.style.stroke_width = w;
                            m.style.stroke_opacity = f.opacity.max(0.3);
                        }
                        None => {
                            // hairline in the face colour hides seams between faces
                            m.style.stroke = Paint::Solid(col);
                            m.style.stroke_width = 0.6;
                            m.style.stroke_opacity = f.opacity;
                        }
                    }
                    m.style.line_join = LineJoin::Round;
                    out.push(m);
                }
                Item::S(s) => {
                    let (a, _) = cam.project(s.a);
                    let (b, _) = cam.project(s.b);
                    out.push(shapes::line(a, b).stroke(s.color, s.width));
                }
                Item::B(b) => {
                    let (p, _) = cam.project(b.center);
                    let (q, _) = cam.project(b.center + cam_right(cam) * b.radius);
                    let r = p.distance(q).max(1e-4);
                    let light2 = {
                        let l = cam.to_camera(b.center + cam.light) - cam.to_camera(b.center);
                        v2(l.x, l.y)
                    };
                    out.push(shapes::circle_at(p, r).no_stroke().sheen(b.color, light2));
                }
                Item::L(l) => {
                    let (p, _) = cam.project(l.at);
                    out.push(l.mobject.clone().move_to(p));
                }
            }
        }
        out
    }
}

fn cam_right(cam: &Camera3D) -> Vec3 {
    let a = cam.theta + PI / 2.0;
    v3(a.cos(), a.sin(), 0.0)
}

fn face(verts: Vec<Vec3>, color: Color) -> Face {
    Face {
        verts,
        color,
        opacity: 1.0,
        stroke: None,
    }
}

/// Parametric surface `(u, v) -> point` (manim's `Surface`).
pub fn surface(
    f: impl Fn(f64, f64) -> Vec3,
    u: (f64, f64),
    v: (f64, f64),
    res: (usize, usize),
    colors: &[Color],
) -> Mesh {
    let (nu, nv) = (res.0.max(1), res.1.max(1));
    let mut m = Mesh::new();
    let pal = if colors.is_empty() {
        vec![BLUE_D, BLUE_E]
    } else {
        colors.to_vec()
    };
    for i in 0..nu {
        for j in 0..nv {
            let (u0, u1) = (
                lerp(u.0, u.1, i as f64 / nu as f64),
                lerp(u.0, u.1, (i + 1) as f64 / nu as f64),
            );
            let (v0, v1) = (
                lerp(v.0, v.1, j as f64 / nv as f64),
                lerp(v.0, v.1, (j + 1) as f64 / nv as f64),
            );
            let c = pal[(i + j) % pal.len()];
            m.faces
                .push(face(vec![f(u0, v0), f(u1, v0), f(u1, v1), f(u0, v1)], c));
        }
    }
    m
}

/// Graph of `z = f(x, y)`.
pub fn surface_z(f: impl Fn(f64, f64) -> f64, x: (f64, f64), y: (f64, f64), res: usize) -> Mesh {
    surface(move |u, v| v3(u, v, f(u, v)), x, y, (res, res), &[])
        .color_by_z(&[BLUE_E, TEAL, YELLOW, RED])
}

pub fn sphere(r: f64, res: usize, color: Color) -> Mesh {
    surface(
        |u, v| v3(r * v.sin() * u.cos(), r * v.sin() * u.sin(), r * v.cos()),
        (0.0, TAU),
        (0.0, PI),
        (res * 2, res),
        &[color],
    )
}

pub fn torus(major: f64, minor: f64, res: usize, color: Color) -> Mesh {
    surface(
        |u, v| {
            v3(
                (major + minor * v.cos()) * u.cos(),
                (major + minor * v.cos()) * u.sin(),
                minor * v.sin(),
            )
        },
        (0.0, TAU),
        (0.0, TAU),
        (res * 2, res),
        &[color],
    )
}

pub fn cylinder(r: f64, h: f64, res: usize, color: Color) -> Mesh {
    let mut m = surface(
        |u, v| v3(r * u.cos(), r * u.sin(), v),
        (0.0, TAU),
        (-h / 2.0, h / 2.0),
        (res, 1),
        &[color],
    );
    for z in [-h / 2.0, h / 2.0] {
        m.faces.push(face(
            (0..res)
                .map(|i| {
                    v3(
                        r * (TAU * i as f64 / res as f64).cos(),
                        r * (TAU * i as f64 / res as f64).sin(),
                        z,
                    )
                })
                .collect(),
            color,
        ));
    }
    m
}

pub fn cone(r: f64, h: f64, res: usize, color: Color) -> Mesh {
    let mut m = surface(
        |u, v| v3(r * (1.0 - v) * u.cos(), r * (1.0 - v) * u.sin(), v * h),
        (0.0, TAU),
        (0.0, 1.0),
        (res, 1),
        &[color],
    );
    m.faces.push(face(
        (0..res)
            .map(|i| {
                v3(
                    r * (TAU * i as f64 / res as f64).cos(),
                    r * (TAU * i as f64 / res as f64).sin(),
                    0.0,
                )
            })
            .collect(),
        color,
    ));
    m
}

/// Axis-aligned box centred at the origin (manim's `Prism`; a cube when all equal).
pub fn prism(dx: f64, dy: f64, dz: f64, color: Color) -> Mesh {
    let (x, y, z) = (dx / 2.0, dy / 2.0, dz / 2.0);
    let p = |a: f64, b: f64, c: f64| v3(a * x, b * y, c * z);
    let quads = [
        [
            p(-1., -1., -1.),
            p(1., -1., -1.),
            p(1., 1., -1.),
            p(-1., 1., -1.),
        ],
        [
            p(-1., -1., 1.),
            p(1., -1., 1.),
            p(1., 1., 1.),
            p(-1., 1., 1.),
        ],
        [
            p(-1., -1., -1.),
            p(1., -1., -1.),
            p(1., -1., 1.),
            p(-1., -1., 1.),
        ],
        [
            p(-1., 1., -1.),
            p(1., 1., -1.),
            p(1., 1., 1.),
            p(-1., 1., 1.),
        ],
        [
            p(-1., -1., -1.),
            p(-1., 1., -1.),
            p(-1., 1., 1.),
            p(-1., -1., 1.),
        ],
        [
            p(1., -1., -1.),
            p(1., 1., -1.),
            p(1., 1., 1.),
            p(1., -1., 1.),
        ],
    ];
    Mesh {
        faces: quads.iter().map(|q| face(q.to_vec(), color)).collect(),
        ..Default::default()
    }
}

pub fn cube(side: f64, color: Color) -> Mesh {
    prism(side, side, side, color)
}

/// Convex polyhedron from vertices and face index lists.
pub fn polyhedron(verts: &[Vec3], faces: &[Vec<usize>], color: Color) -> Mesh {
    Mesh {
        faces: faces
            .iter()
            .map(|f| face(f.iter().map(|&i| verts[i]).collect(), color))
            .collect(),
        ..Default::default()
    }
}

pub fn tetrahedron(r: f64, color: Color) -> Mesh {
    let k = r / 3f64.sqrt();
    let v = [v3(k, k, k), v3(k, -k, -k), v3(-k, k, -k), v3(-k, -k, k)];
    polyhedron(
        &v,
        &[vec![0, 1, 2], vec![0, 3, 1], vec![0, 2, 3], vec![1, 3, 2]],
        color,
    )
}

pub fn octahedron(r: f64, color: Color) -> Mesh {
    let v = [
        v3(r, 0., 0.),
        v3(-r, 0., 0.),
        v3(0., r, 0.),
        v3(0., -r, 0.),
        v3(0., 0., r),
        v3(0., 0., -r),
    ];
    let f = vec![
        vec![0, 2, 4],
        vec![2, 1, 4],
        vec![1, 3, 4],
        vec![3, 0, 4],
        vec![2, 0, 5],
        vec![1, 2, 5],
        vec![3, 1, 5],
        vec![0, 3, 5],
    ];
    polyhedron(&v, &f, color)
}

pub fn icosahedron(r: f64, color: Color) -> Mesh {
    let t = (1.0 + 5f64.sqrt()) / 2.0;
    let raw = [
        v3(-1., t, 0.),
        v3(1., t, 0.),
        v3(-1., -t, 0.),
        v3(1., -t, 0.),
        v3(0., -1., t),
        v3(0., 1., t),
        v3(0., -1., -t),
        v3(0., 1., -t),
        v3(t, 0., -1.),
        v3(t, 0., 1.),
        v3(-t, 0., -1.),
        v3(-t, 0., 1.),
    ];
    let v: Vec<Vec3> = raw.iter().map(|p| p.normalize() * r).collect();
    let f: Vec<Vec<usize>> = [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ]
    .iter()
    .map(|a| a.to_vec())
    .collect();
    polyhedron(&v, &f, color)
}

/// Dodecahedron as the dual of the icosahedron.
pub fn dodecahedron(r: f64, color: Color) -> Mesh {
    let ico = icosahedron(1.0, color);
    let centers: Vec<Vec3> = ico
        .faces
        .iter()
        .map(|f| (f.verts.iter().fold(Vec3::ZERO, |a, v| a + *v) / 3.0).normalize() * r)
        .collect();
    // each icosahedron vertex → a pentagon of adjacent face centres (sorted by angle)
    let mut verts: Vec<Vec3> = Vec::new();
    for f in &ico.faces {
        for v in &f.verts {
            if !verts.iter().any(|u| (*u - *v).length() < 1e-9) {
                verts.push(*v);
            }
        }
    }
    let mut faces = Vec::new();
    for v in verts {
        let mut adj: Vec<usize> = ico
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.verts.iter().any(|u| (*u - v).length() < 1e-9))
            .map(|(i, _)| i)
            .collect();
        let n = v.normalize();
        let a = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let e1 = n.cross(a).normalize();
        let e2 = n.cross(e1);
        adj.sort_by(|&i, &j| {
            let ai = centers[i].dot(e2).atan2(centers[i].dot(e1));
            let aj = centers[j].dot(e2).atan2(centers[j].dot(e1));
            ai.partial_cmp(&aj).unwrap()
        });
        faces.push(face(adj.iter().map(|&i| centers[i]).collect(), color));
    }
    Mesh {
        faces,
        ..Default::default()
    }
}

pub fn line3d(a: Vec3, b: Vec3, color: Color, width: f32) -> Mesh {
    Mesh {
        segs: vec![Seg3 { a, b, color, width }],
        ..Default::default()
    }
}

/// Polyline through 3D points (parametric curves, trajectories).
pub fn curve3d(points: &[Vec3], color: Color, width: f32) -> Mesh {
    Mesh {
        segs: points
            .windows(2)
            .map(|w| Seg3 {
                a: w[0],
                b: w[1],
                color,
                width,
            })
            .collect(),
        ..Default::default()
    }
}

pub fn parametric_curve3d(
    f: impl Fn(f64) -> Vec3,
    t0: f64,
    t1: f64,
    samples: usize,
    color: Color,
) -> Mesh {
    let pts: Vec<Vec3> = linspace(t0, t1, samples.max(2))
        .into_iter()
        .map(f)
        .collect();
    curve3d(&pts, color, 4.0)
}

/// 3D arrow: shaft + cone tip.
pub fn arrow3d(a: Vec3, b: Vec3, color: Color) -> Mesh {
    let d = b - a;
    let len = d.length().max(1e-9);
    let tip = (0.25f64).min(len * 0.35);
    let base = b - d / len * tip;
    let dir = d / len;
    let tmp = if dir.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let e1 = dir.cross(tmp).normalize();
    let e2 = dir.cross(e1);
    let r = tip * 0.4;
    let n = 12;
    let mut m = line3d(a, base, color, 5.0);
    for i in 0..n {
        let a0 = TAU * i as f64 / n as f64;
        let a1 = TAU * (i + 1) as f64 / n as f64;
        let p0 = base + (e1 * a0.cos() + e2 * a0.sin()) * r;
        let p1 = base + (e1 * a1.cos() + e2 * a1.sin()) * r;
        m.faces.push(face(vec![p0, p1, b], color));
    }
    m
}

pub fn dot3d(p: Vec3, r: f64, color: Color) -> Mesh {
    Mesh {
        balls: vec![Ball {
            center: p,
            radius: r,
            color,
        }],
        ..Default::default()
    }
}

pub fn label3d(at: Vec3, m: Mobject) -> Mesh {
    Mesh {
        labels: vec![Label3 { at, mobject: m }],
        ..Default::default()
    }
}

/// 3D axes with ticks and x/y/z labels (manim's `ThreeDAxes`).
pub fn axes3d(range: f64, step: f64) -> Mesh {
    let mut m = Mesh::new();
    let c = GRAY_A;
    for (dir, name) in [(Vec3::X, "x"), (Vec3::Y, "y"), (Vec3::Z, "z")] {
        m.add(arrow3d(-dir * range, dir * (range + 0.4), c));
        for k in arange(-range, range + 1e-9, step) {
            if k.abs() < 1e-9 {
                continue;
            }
            let p = dir * k;
            let off = if dir.z.abs() > 0.5 {
                Vec3::X * 0.08
            } else {
                Vec3::Z * 0.08
            };
            m.segs.push(Seg3 {
                a: p - off,
                b: p + off,
                color: c,
                width: 2.0,
            });
        }
        m.add(label3d(
            dir * (range + 0.8),
            crate::tex::tex_sized(name, 0.5, c),
        ));
    }
    m
}

/// Scene-side 3D camera rig: trackers for phi, theta, zoom (animate with `.to()`).
#[derive(Clone, Copy, Debug)]
pub struct CameraRig {
    pub phi: Tracker,
    pub theta: Tracker,
    pub zoom: Tracker,
    pub base: Camera3D,
}

impl CameraRig {
    pub fn at(&self, ctx: &FrameCtx) -> Camera3D {
        Camera3D {
            phi: ctx.value(self.phi),
            theta: ctx.value(self.theta),
            zoom: ctx.value(self.zoom),
            ..self.base
        }
    }
}

impl Scene {
    /// Create a 3D camera rig (angles in degrees).
    pub fn camera3d(&mut self, phi_deg: f64, theta_deg: f64) -> CameraRig {
        let base = Camera3D::new(phi_deg, theta_deg);
        CameraRig {
            phi: self.tracker(base.phi),
            theta: self.tracker(base.theta),
            zoom: self.tracker(1.0),
            base,
        }
    }
    /// Add static 3D geometry viewed through `rig` (re-projected every frame).
    pub fn add_3d(&mut self, rig: CameraRig, mesh: Mesh) -> Obj {
        let mesh = std::sync::Arc::new(mesh);
        self.always_redraw(move |ctx| mesh.render(&rig.at(ctx)))
    }
    /// 3D geometry rebuilt every frame (moving objects, simulations).
    pub fn add_3d_dynamic(
        &mut self,
        rig: CameraRig,
        f: impl Fn(&FrameCtx) -> Mesh + Send + Sync + 'static,
    ) -> Obj {
        self.always_redraw(move |ctx| f(ctx).render(&rig.at(ctx)))
    }
    /// Ambient camera rotation (`begin_ambient_camera_rotation`): spin theta at
    /// `rate` rad/s for `duration` seconds in the background (cursor does not move).
    pub fn ambient_rotation(&mut self, rig: CameraRig, rate: f64, duration: f64) {
        let t0 = self.time();
        let start = self.value(rig.theta);
        self.play(
            rig.theta
                .to(start + rate * duration)
                .run_time(duration)
                .rate(crate::rate::Rate::Linear),
        );
        // rewind the cursor: the rotation runs underneath whatever is played next
        self.rewind_to(t0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_projection() {
        let cam = Camera3D {
            phi: 0.0,
            theta: -PI / 2.0,
            distance: f64::INFINITY,
            ..Default::default()
        };
        // looking straight down with theta=-90°: x right, y up
        let (p, _) = cam.project(v3(1.0, 2.0, 0.0));
        assert!((p - v2(1.0, 2.0)).length() < 1e-9, "{p:?}");
        let side = Camera3D::new(90.0, -90.0);
        let (q, _) = side.project(v3(0.0, 0.0, 1.0));
        assert!(q.y > 0.9, "z points up in a side view: {q:?}");
    }

    #[test]
    fn meshes_render() {
        let cam = Camera3D::default();
        for m in [
            sphere(1.0, 8, BLUE),
            cube(1.0, RED),
            torus(1.0, 0.3, 8, GREEN),
            cylinder(0.5, 1.0, 12, TEAL),
            cone(0.5, 1.0, 12, YELLOW),
            tetrahedron(1.0, RED),
            octahedron(1.0, RED),
            icosahedron(1.0, RED),
            dodecahedron(1.0, RED),
            surface_z(|x, y| (x * x - y * y) * 0.3, (-1.0, 1.0), (-1.0, 1.0), 6),
            axes3d(3.0, 1.0),
            arrow3d(Vec3::ZERO, Vec3::X, WHITE),
            dot3d(Vec3::Z, 0.2, RED),
        ] {
            let r = m.render(&cam);
            assert!(r.leaf_count() > 0);
        }
        assert_eq!(dodecahedron(1.0, RED).faces.len(), 12);
        assert!(dodecahedron(1.0, RED)
            .faces
            .iter()
            .all(|f| f.verts.len() == 5));
    }

    #[test]
    fn rig_in_scene() {
        let mut s = Scene::new();
        let rig = s.camera3d(70.0, -45.0);
        let o = s.add_3d(rig, cube(2.0, BLUE));
        s.ambient_rotation(rig, 0.5, 4.0);
        s.wait(1.0);
        assert!((s.time() - 1.0).abs() < 1e-9);
        let a = s.eval_obj(o, 0.0, 0);
        let b = s.eval_obj(o, 3.0, 0);
        assert!((a.center() - b.center()).length() < 1.0);
        assert_ne!(a.anchors(), b.anchors());
    }
}
