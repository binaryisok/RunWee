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

// ------------------------------------------------------- more manim 3D mobjects

/// Flat square in the xy-plane (manim's `Square3D`).
pub fn square3d(side: f64, color: Color) -> Mesh {
    let h = side / 2.0;
    Mesh {
        faces: vec![face(
            vec![
                v3(-h, -h, 0.0),
                v3(h, -h, 0.0),
                v3(h, h, 0.0),
                v3(-h, h, 0.0),
            ],
            color,
        )],
        ..Default::default()
    }
}

/// Flat disk in the xy-plane (manim's `Disk3D`).
pub fn disk3d(radius: f64, res: usize, color: Color) -> Mesh {
    let n = res.max(8);
    Mesh {
        faces: vec![face(
            (0..n)
                .map(|i| {
                    let a = TAU * i as f64 / n as f64;
                    v3(radius * a.cos(), radius * a.sin(), 0.0)
                })
                .collect(),
            color,
        )],
        ..Default::default()
    }
}

/// Box built from flat faces with an outline (manim's `VPrism`).
pub fn v_prism(dx: f64, dy: f64, dz: f64, fill: Color, stroke: Option<(Color, f32)>) -> Mesh {
    let mut m = prism(dx, dy, dz, fill);
    for f in &mut m.faces {
        f.stroke = stroke;
    }
    m
}

/// Cube built from flat faces with an outline (manim's `VCube`).
pub fn v_cube(side: f64, fill: Color, stroke: Option<(Color, f32)>) -> Mesh {
    v_prism(side, side, side, fill, stroke)
}

fn sample_subpath(sp: &SubPath) -> Vec<Vec2> {
    let mut pts = vec![sp.start];
    for i in 0..sp.segs.len() {
        let s = sp.segs[i];
        let p0 = sp.seg_start(i);
        let straight = (s.c1 - p0.lerp(s.p, 1.0 / 3.0)).length() < 1e-9
            && (s.c2 - p0.lerp(s.p, 2.0 / 3.0)).length() < 1e-9;
        let k = if straight { 1 } else { 8 };
        for j in 1..=k {
            pts.push(s.eval(p0, j as f64 / k as f64));
        }
    }
    if pts.len() > 2 && pts[0].distance(*pts.last().unwrap()) < 1e-9 {
        pts.pop();
    }
    pts
}

/// Extrude a 2D mobject's outline into a prism (manim's `Prismify`): the shape
/// at z = 0, a copy shifted by `depth * dir`, and side walls. Curves are sampled.
pub fn prismify(m: &Mobject, depth: f64, dir: Vec3, color: Color) -> Mesh {
    let v = dir.normalize() * depth;
    let mut mesh = Mesh::new();
    for sp in &m.family_path().subpaths {
        let pts = sample_subpath(sp);
        if pts.len() < 2 {
            continue;
        }
        let bottom: Vec<Vec3> = pts.iter().map(|p| p.extend(0.0)).collect();
        let n = bottom.len();
        let walls = if sp.closed || n > 2 { n } else { n - 1 };
        for i in 0..walls {
            let (a, b) = (bottom[i], bottom[(i + 1) % n]);
            mesh.faces.push(face(vec![a, b, b + v, a + v], color));
        }
        if n >= 3 {
            mesh.faces.push(face(bottom.clone(), color));
            mesh.faces
                .push(face(bottom.iter().rev().map(|p| *p + v).collect(), color));
        }
    }
    mesh
}

/// Wireframe grid lines over a parametric surface (manim's `SurfaceMesh`):
/// `res.0` lines of constant u and `res.1` lines of constant v.
/// (`Mesh::wireframe` instead outlines every face of a surface mesh.)
pub fn surface_mesh(
    f: impl Fn(f64, f64) -> Vec3,
    u: (f64, f64),
    v: (f64, f64),
    res: (usize, usize),
    color: Color,
    width: f32,
) -> Mesh {
    let samples = 48;
    let mut m = Mesh::new();
    for uu in linspace(u.0, u.1, res.0.max(2)) {
        let pts: Vec<Vec3> = linspace(v.0, v.1, samples)
            .into_iter()
            .map(|vv| f(uu, vv))
            .collect();
        m.add(curve3d(&pts, color, width));
    }
    for vv in linspace(v.0, v.1, res.1.max(2)) {
        let pts: Vec<Vec3> = linspace(u.0, u.1, samples)
            .into_iter()
            .map(|uu| f(uu, vv))
            .collect();
        m.add(curve3d(&pts, color, width));
    }
    m
}

/// Parametric surface coloured from an image (manim's `TexturedSurface`): each
/// face takes the pixel at its (u, v) centre, u → image x, v → image y (bottom up).
pub fn textured_surface(
    f: impl Fn(f64, f64) -> Vec3,
    u: (f64, f64),
    v: (f64, f64),
    res: (usize, usize),
    image: &tiny_skia::Pixmap,
) -> Mesh {
    let (nu, nv) = (res.0.max(1), res.1.max(1));
    let (w, h) = (image.width().max(1), image.height().max(1));
    let mut m = surface(f, u, v, (nu, nv), &[WHITE]);
    // `surface` pushes faces in (i, j) order
    for (k, fc) in m.faces.iter_mut().enumerate() {
        let (i, j) = (k / nv, k % nv);
        let su = (i as f64 + 0.5) / nu as f64;
        let sv = (j as f64 + 0.5) / nv as f64;
        let px = ((su * w as f64) as u32).min(w - 1);
        let py = (((1.0 - sv) * h as f64) as u32).min(h - 1);
        if let Some(c) = image.pixel(px, py) {
            let c = c.demultiply();
            fc.color = rgb8(c.red(), c.green(), c.blue());
            fc.opacity = c.alpha() as f32 / 255.0;
        }
    }
    m
}

/// Parse a Wavefront OBJ (`v` and `f` records; `vt`/`vn` indices, negative
/// indices and polygons are supported; materials ignored). Coordinates are kept as-is.
pub fn parse_obj(src: &str, color: Color) -> anyhow::Result<Mesh> {
    let mut verts: Vec<Vec3> = Vec::new();
    let mut mesh = Mesh::new();
    for (ln, line) in src.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let c: Vec<f64> = it
                    .take(3)
                    .map(|x| x.parse::<f64>())
                    .collect::<Result<_, _>>()
                    .map_err(|e| anyhow::anyhow!("line {}: bad vertex: {e}", ln + 1))?;
                if c.len() < 3 {
                    anyhow::bail!("line {}: vertex needs 3 coordinates", ln + 1);
                }
                verts.push(v3(c[0], c[1], c[2]));
            }
            Some("f") => {
                let mut fv = Vec::new();
                for tok in it {
                    let idx: i64 = tok
                        .split('/')
                        .next()
                        .unwrap_or("")
                        .parse()
                        .map_err(|e| anyhow::anyhow!("line {}: bad face index: {e}", ln + 1))?;
                    let i = if idx < 0 {
                        verts.len() as i64 + idx
                    } else {
                        idx - 1
                    };
                    let p = verts
                        .get(usize::try_from(i).unwrap_or(usize::MAX))
                        .ok_or_else(|| {
                            anyhow::anyhow!("line {}: face index out of range", ln + 1)
                        })?;
                    fv.push(*p);
                }
                if fv.len() >= 3 {
                    mesh.faces.push(face(fv, color));
                }
            }
            _ => {}
        }
    }
    Ok(mesh)
}

/// Load a Wavefront `.obj` file as a mesh (manim's `ThreeDModel`): converted from
/// the OBJ y-up convention to z-up, centred, and scaled to `height` along z.
pub fn three_d_model(
    path: impl AsRef<std::path::Path>,
    height: f64,
    color: Color,
) -> anyhow::Result<Mesh> {
    let src = std::fs::read_to_string(path.as_ref())?;
    let m = parse_obj(&src, color)?.map_points(|p| v3(p.x, -p.z, p.y));
    let (mut lo, mut hi) = (
        v3(f64::MAX, f64::MAX, f64::MAX),
        v3(f64::MIN, f64::MIN, f64::MIN),
    );
    for f in &m.faces {
        for p in &f.verts {
            lo = v3(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
            hi = v3(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
        }
    }
    if m.faces.is_empty() {
        return Ok(m);
    }
    let c = (lo + hi) * 0.5;
    let s = height / (hi.z - lo.z).max(1e-9);
    Ok(m.map_points(move |p| (p - c) * s))
}

/// Convex hull of 3D points as a polyhedron (manim's `ConvexHull3D`); coplanar
/// triangles are merged into polygon faces. Incremental algorithm, O(n²).
pub fn convex_hull_3d(points: &[Vec3], color: Color) -> Mesh {
    let tris = hull_triangles(points);
    // group triangles by plane
    let mut planes: Vec<(Vec3, f64, Vec<usize>)> = Vec::new();
    let scale = points.iter().map(|p| p.length()).fold(1e-9, f64::max);
    let eps = 1e-7 * scale;
    for t in &tris {
        let (a, b, c) = (points[t[0]], points[t[1]], points[t[2]]);
        let n = (b - a).cross(c - a).normalize();
        let d = n.dot(a);
        match planes
            .iter_mut()
            .find(|(m, e, _)| (*m - n).length() < 1e-6 && (e - d).abs() < eps)
        {
            Some(pl) => pl.2.extend_from_slice(t),
            None => planes.push((n, d, t.to_vec())),
        }
    }
    let mut mesh = Mesh::new();
    for (n, _, mut idx) in planes {
        idx.sort_unstable();
        idx.dedup();
        let pts: Vec<Vec3> = idx.iter().map(|&i| points[i]).collect();
        let c = pts.iter().fold(Vec3::ZERO, |a, p| a + *p) / pts.len() as f64;
        let a = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let e1 = n.cross(a).normalize();
        let e2 = n.cross(e1);
        let flat: Vec<Vec2> = pts
            .iter()
            .map(|p| v2((*p - c).dot(e1), (*p - c).dot(e2)))
            .collect();
        let hull = crate::boolean::convex_hull_points(&flat);
        let verts: Vec<Vec3> = hull.iter().map(|q| c + e1 * q.x + e2 * q.y).collect();
        if verts.len() >= 3 {
            mesh.faces.push(face(verts, color));
        }
    }
    mesh
}

/// Outward-oriented hull triangles (indices into `pts`).
fn hull_triangles(pts: &[Vec3]) -> Vec<[usize; 3]> {
    let n = pts.len();
    if n < 4 {
        return if n == 3 { vec![[0, 1, 2]] } else { vec![] };
    }
    let scale = pts.iter().map(|p| p.length()).fold(1e-9, f64::max);
    let eps = 1e-9 * scale;
    // initial tetrahedron from extreme points
    let i0 = 0;
    let i1 = (0..n)
        .max_by(|&a, &b| {
            (pts[a] - pts[i0])
                .length()
                .total_cmp(&(pts[b] - pts[i0]).length())
        })
        .unwrap();
    let dir = (pts[i1] - pts[i0]).normalize();
    let line_d = |p: Vec3| {
        let q = p - pts[i0];
        (q - dir * q.dot(dir)).length()
    };
    let i2 = (0..n)
        .max_by(|&a, &b| line_d(pts[a]).total_cmp(&line_d(pts[b])))
        .unwrap();
    let nrm = (pts[i1] - pts[i0]).cross(pts[i2] - pts[i0]).normalize();
    let plane_d = |p: Vec3| (p - pts[i0]).dot(nrm).abs();
    let i3 = (0..n)
        .max_by(|&a, &b| plane_d(pts[a]).total_cmp(&plane_d(pts[b])))
        .unwrap();
    if line_d(pts[i2]) < eps || plane_d(pts[i3]) < eps {
        // degenerate (coplanar): a single flat polygon via the plane grouping
        let c = pts.iter().fold(Vec3::ZERO, |a, p| a + *p) / n as f64;
        let a = if nrm.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let e1 = nrm.cross(a).normalize();
        let e2 = nrm.cross(e1);
        let flat: Vec<Vec2> = pts
            .iter()
            .map(|p| v2((*p - c).dot(e1), (*p - c).dot(e2)))
            .collect();
        let hull = crate::boolean::convex_hull_points(&flat);
        let idx: Vec<usize> = hull
            .iter()
            .filter_map(|q| flat.iter().position(|f| (*f - *q).length() < 1e-12))
            .collect();
        return (1..idx.len().saturating_sub(1))
            .map(|k| [idx[0], idx[k], idx[k + 1]])
            .collect();
    }
    let interior = (pts[i0] + pts[i1] + pts[i2] + pts[i3]) * 0.25;
    let orient = |t: [usize; 3]| -> [usize; 3] {
        let (a, b, c) = (pts[t[0]], pts[t[1]], pts[t[2]]);
        if (b - a).cross(c - a).dot(a - interior) < 0.0 {
            [t[0], t[2], t[1]]
        } else {
            t
        }
    };
    let mut faces: Vec<[usize; 3]> = vec![
        orient([i0, i1, i2]),
        orient([i0, i1, i3]),
        orient([i0, i2, i3]),
        orient([i1, i2, i3]),
    ];
    let seeds = [i0, i1, i2, i3];
    for p in 0..n {
        if seeds.contains(&p) {
            continue;
        }
        let visible: Vec<bool> = faces
            .iter()
            .map(|t| {
                let (a, b, c) = (pts[t[0]], pts[t[1]], pts[t[2]]);
                (b - a).cross(c - a).normalize().dot(pts[p] - a) > eps
            })
            .collect();
        if !visible.iter().any(|v| *v) {
            continue;
        }
        let mut edges = std::collections::HashSet::new();
        for (t, _) in faces.iter().zip(&visible).filter(|(_, v)| **v) {
            for k in 0..3 {
                edges.insert((t[k], t[(k + 1) % 3]));
            }
        }
        let horizon: Vec<(usize, usize)> = edges
            .iter()
            .copied()
            .filter(|(a, b)| !edges.contains(&(*b, *a)))
            .collect();
        faces = faces
            .into_iter()
            .zip(visible)
            .filter(|(_, v)| !v)
            .map(|(t, _)| t)
            .collect();
        for (a, b) in horizon {
            faces.push([a, b, p]);
        }
    }
    faces
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

    #[test]
    fn obj_parsing() {
        let src =
            "# cube\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\nv 1 1 1\nv 0 1 1\n\
            vt 0 0\nvn 0 0 1\nf 1/1/1 2/1/1 3/1/1 4/1/1\nf -4 -3 -2 -1\nf 1//1 2//1 6//1\n";
        let m = parse_obj(src, BLUE).unwrap();
        assert_eq!(m.faces.len(), 3);
        assert_eq!(m.faces[0].verts.len(), 4);
        assert!((m.faces[1].verts[0] - v3(0.0, 0.0, 1.0)).length() < 1e-12);
        assert!(parse_obj("f 1 2 3", BLUE).is_err());
        let dir = std::env::temp_dir().join(format!("runwee-obj-{}.obj", std::process::id()));
        std::fs::write(&dir, src).unwrap();
        let model = three_d_model(&dir, 3.0, RED).unwrap();
        let zs: Vec<f64> = model
            .faces
            .iter()
            .flat_map(|f| f.verts.iter().map(|v| v.z))
            .collect();
        let (lo, hi) = zs
            .iter()
            .fold((f64::MAX, f64::MIN), |a, z| (a.0.min(*z), a.1.max(*z)));
        assert!((hi - lo - 3.0).abs() < 1e-9 && (hi + lo).abs() < 1e-9);
        let _ = std::fs::remove_file(dir);
    }

    #[test]
    fn hull_and_extra_meshes() {
        let mut pts = Vec::new();
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    pts.push(v3(x, y, z));
                }
            }
        }
        let mut rng = Rng::new(3);
        for _ in 0..40 {
            pts.push(v3(
                rng.range(-0.9, 0.9),
                rng.range(-0.9, 0.9),
                rng.range(-0.9, 0.9),
            ));
        }
        let h = convex_hull_3d(&pts, BLUE);
        assert_eq!(h.faces.len(), 6);
        assert!(h.faces.iter().all(|f| f.verts.len() == 4));
        // random cloud: every point lies inside every face plane
        let cloud: Vec<Vec3> = (0..60)
            .map(|_| v3(rng.normal(), rng.normal(), rng.normal()))
            .collect();
        let h = convex_hull_3d(&cloud, RED);
        let c = cloud.iter().fold(Vec3::ZERO, |a, p| a + *p) / 60.0;
        for f in &h.faces {
            let n = (f.verts[1] - f.verts[0])
                .cross(f.verts[2] - f.verts[0])
                .normalize();
            let n = if n.dot(f.verts[0] - c) < 0.0 { -n } else { n };
            for p in &cloud {
                assert!(n.dot(*p - f.verts[0]) < 1e-7);
            }
        }
        // Euler: V − E + F = 2 for the triangulated hull
        let tris = hull_triangles(&cloud);
        let mut vs: Vec<usize> = tris.iter().flatten().copied().collect();
        vs.sort_unstable();
        vs.dedup();
        assert_eq!(
            vs.len() as i64 - (tris.len() as i64 * 3 / 2) + tris.len() as i64,
            2
        );

        let p = prismify(&crate::shapes::square(2.0), 1.0, Vec3::Z, TEAL);
        assert_eq!(p.faces.len(), 6);
        let p = prismify(&crate::shapes::circle(1.0), 0.5, -Vec3::Z, TEAL);
        assert!(p.faces.len() > 20);
        assert_eq!(square3d(2.0, RED).faces.len(), 1);
        assert_eq!(disk3d(1.0, 32, RED).faces[0].verts.len(), 32);
        assert!(v_cube(1.0, BLUE, Some((WHITE, 2.0)))
            .faces
            .iter()
            .all(|f| f.stroke.is_some()));
        let sm = surface_mesh(
            |u, v| v3(u, v, 0.0),
            (0.0, 1.0),
            (0.0, 1.0),
            (5, 3),
            GRAY,
            1.0,
        );
        assert_eq!(sm.segs.len(), (5 + 3) * 47);
        let mut img = tiny_skia::Pixmap::new(2, 1).unwrap();
        img.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        img.pixels_mut()[1] = tiny_skia::ColorU8::from_rgba(0, 0, 255, 255).premultiply();
        let ts = textured_surface(|u, v| v3(u, v, 0.0), (0.0, 1.0), (0.0, 1.0), (2, 2), &img);
        assert!(ts.faces[0].color.r > 0.9 && ts.faces[3].color.b > 0.9);
    }
}
