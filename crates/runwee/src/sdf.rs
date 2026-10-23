//! Signed-distance-field raymarcher (phase A4): a tiny CPU "fragment shader"
//! for 3D scenes built from SDF primitives and CSG, with soft shadows, ambient
//! occlusion, fog and a sky gradient. Images come out as image mobjects via
//! [`crate::generative::field_image`] (rows rendered in parallel).
//!
//! World space is **y-up** (shader convention).
use crate::color::*;
use crate::generative::field_image;
use crate::math::*;
use crate::mobject::Mobject;
use crate::path::Rect;

/// An SDF scene graph node.
#[derive(Clone, Debug)]
pub enum Sdf {
    Sphere(f64),
    /// Half-extents and corner radius.
    RoundBox(Vec3, f64),
    /// Major and minor radius (in the xz plane).
    Torus(f64, f64),
    /// Ground plane `y = h`.
    Plane(f64),
    /// Capsule between two points.
    Capsule(Vec3, Vec3, f64),
    /// Gyroid sheet of given scale and thickness, bounded by the inner node.
    Gyroid(f64, f64, Box<Sdf>),
    Translate(Vec3, Box<Sdf>),
    /// Rotate about the y axis then the x axis (radians).
    Rotate(f64, f64, Box<Sdf>),
    Scale(f64, Box<Sdf>),
    /// Twist around y by `k` radians per unit height.
    Twist(f64, Box<Sdf>),
    /// Infinite repetition with the given cell size (0 = no repetition on that axis).
    Repeat(Vec3, Box<Sdf>),
    Union(Box<Sdf>, Box<Sdf>),
    SmoothUnion(Box<Sdf>, Box<Sdf>, f64),
    Subtract(Box<Sdf>, Box<Sdf>),
    Intersect(Box<Sdf>, Box<Sdf>),
    /// Displace the surface by `amp * sin(freq x) sin(freq y) sin(freq z)`.
    Displace(f64, f64, Box<Sdf>),
    /// Material colour (and checker for planes when `checker` > 0).
    Material(Color, f64, Box<Sdf>),
}

fn vabs(p: Vec3) -> Vec3 {
    v3(p.x.abs(), p.y.abs(), p.z.abs())
}
fn vmax0(p: Vec3) -> Vec3 {
    v3(p.x.max(0.0), p.y.max(0.0), p.z.max(0.0))
}
fn modc(x: f64, c: f64) -> f64 {
    if c <= 0.0 {
        x
    } else {
        x - c * (x / c).round()
    }
}

/// Start an SDF: a sphere of radius `r` at the origin.
pub fn sd_sphere(r: f64) -> Sdf {
    Sdf::Sphere(r)
}
pub fn sd_box(half: Vec3, round: f64) -> Sdf {
    Sdf::RoundBox(half, round)
}
pub fn sd_torus(major: f64, minor: f64) -> Sdf {
    Sdf::Torus(major, minor)
}
pub fn sd_plane(h: f64) -> Sdf {
    Sdf::Plane(h)
}
pub fn sd_capsule(a: Vec3, b: Vec3, r: f64) -> Sdf {
    Sdf::Capsule(a, b, r)
}

impl Sdf {
    pub fn at(self, p: Vec3) -> Sdf {
        Sdf::Translate(p, Box::new(self))
    }
    pub fn rotate(self, yaw: f64, pitch: f64) -> Sdf {
        Sdf::Rotate(yaw, pitch, Box::new(self))
    }
    pub fn scaled(self, k: f64) -> Sdf {
        Sdf::Scale(k, Box::new(self))
    }
    pub fn twist(self, k: f64) -> Sdf {
        Sdf::Twist(k, Box::new(self))
    }
    pub fn repeat(self, cell: Vec3) -> Sdf {
        Sdf::Repeat(cell, Box::new(self))
    }
    pub fn union(self, o: Sdf) -> Sdf {
        Sdf::Union(Box::new(self), Box::new(o))
    }
    pub fn smooth_union(self, o: Sdf, k: f64) -> Sdf {
        Sdf::SmoothUnion(Box::new(self), Box::new(o), k)
    }
    pub fn subtract(self, o: Sdf) -> Sdf {
        Sdf::Subtract(Box::new(self), Box::new(o))
    }
    pub fn intersect(self, o: Sdf) -> Sdf {
        Sdf::Intersect(Box::new(self), Box::new(o))
    }
    pub fn displace(self, amp: f64, freq: f64) -> Sdf {
        Sdf::Displace(amp, freq, Box::new(self))
    }
    pub fn gyroid(self, scale: f64, thickness: f64) -> Sdf {
        Sdf::Gyroid(scale, thickness, Box::new(self))
    }
    pub fn color(self, c: Color) -> Sdf {
        Sdf::Material(c, 0.0, Box::new(self))
    }
    /// Colour with a checkerboard of cell size `cell`.
    pub fn checker(self, c: Color, cell: f64) -> Sdf {
        Sdf::Material(c, cell, Box::new(self))
    }

    /// Distance only.
    pub fn dist(&self, p: Vec3) -> f64 {
        self.eval(p).0
    }

    /// Distance and surface colour at `p`.
    pub fn eval(&self, p: Vec3) -> (f64, Color) {
        const DEF: Color = Color::rgb(0.8, 0.8, 0.8);
        match self {
            Sdf::Sphere(r) => (p.length() - r, DEF),
            Sdf::RoundBox(b, r) => {
                let q = vabs(p) - *b + v3(*r, *r, *r);
                (vmax0(q).length() + q.x.max(q.y.max(q.z)).min(0.0) - r, DEF)
            }
            Sdf::Torus(a, b) => {
                let q = v2(v2(p.x, p.z).length() - a, p.y);
                (q.length() - b, DEF)
            }
            Sdf::Plane(h) => (p.y - h, DEF),
            Sdf::Capsule(a, b, r) => {
                let pa = p - *a;
                let ba = *b - *a;
                let h = (pa.dot(ba) / ba.dot(ba).max(1e-12)).clamp(0.0, 1.0);
                ((pa - ba * h).length() - r, DEF)
            }
            Sdf::Gyroid(s, th, inner) => {
                let q = p * *s;
                let g = (q.x.sin() * q.y.cos() + q.y.sin() * q.z.cos() + q.z.sin() * q.x.cos())
                    .abs()
                    / s
                    - th;
                let (d, c) = inner.eval(p);
                (d.max(g * 0.6), c)
            }
            Sdf::Translate(o, s) => s.eval(p - *o),
            Sdf::Rotate(yaw, pitch, s) => {
                let (sy, cy) = (-yaw).sin_cos();
                let q = v3(p.x * cy + p.z * sy, p.y, -p.x * sy + p.z * cy);
                let (sp, cp) = (-pitch).sin_cos();
                let q = v3(q.x, q.y * cp - q.z * sp, q.y * sp + q.z * cp);
                s.eval(q)
            }
            Sdf::Scale(k, s) => {
                let (d, c) = s.eval(p * (1.0 / k));
                (d * k, c)
            }
            Sdf::Twist(k, s) => {
                let (sn, cs) = (k * p.y).sin_cos();
                let q = v3(cs * p.x - sn * p.z, p.y, sn * p.x + cs * p.z);
                let (d, c) = s.eval(q);
                (d * 0.7, c)
            }
            Sdf::Repeat(cell, s) => {
                s.eval(v3(modc(p.x, cell.x), modc(p.y, cell.y), modc(p.z, cell.z)))
            }
            Sdf::Union(a, b) => {
                let (da, ca) = a.eval(p);
                let (db, cb) = b.eval(p);
                if da < db {
                    (da, ca)
                } else {
                    (db, cb)
                }
            }
            Sdf::SmoothUnion(a, b, k) => {
                let (da, ca) = a.eval(p);
                let (db, cb) = b.eval(p);
                let h = (0.5 + 0.5 * (db - da) / k).clamp(0.0, 1.0);
                let d = lerp(db, da, h) - k * h * (1.0 - h);
                (d, cb.mix(ca, h as f32))
            }
            Sdf::Subtract(a, b) => {
                let (da, ca) = a.eval(p);
                let db = b.dist(p);
                (da.max(-db), ca)
            }
            Sdf::Intersect(a, b) => {
                let (da, ca) = a.eval(p);
                let db = b.dist(p);
                (da.max(db), ca)
            }
            Sdf::Displace(amp, f, s) => {
                let (d, c) = s.eval(p);
                (
                    d + amp * (f * p.x).sin() * (f * p.y).sin() * (f * p.z).sin(),
                    c,
                )
            }
            Sdf::Material(col, cell, s) => {
                let (d, _) = s.eval(p);
                let c = if *cell > 0.0 {
                    let k = ((p.x / cell).floor() + (p.z / cell).floor()) as i64;
                    if k.rem_euclid(2) == 0 {
                        *col
                    } else {
                        col.mix(BLACK, 0.45)
                    }
                } else {
                    *col
                };
                (d, c)
            }
        }
    }

    /// Surface normal by central differences.
    pub fn normal(&self, p: Vec3) -> Vec3 {
        let e = 1e-3;
        v3(
            self.dist(p + v3(e, 0.0, 0.0)) - self.dist(p - v3(e, 0.0, 0.0)),
            self.dist(p + v3(0.0, e, 0.0)) - self.dist(p - v3(0.0, e, 0.0)),
            self.dist(p + v3(0.0, 0.0, e)) - self.dist(p - v3(0.0, 0.0, e)),
        )
        .normalize()
    }
}

/// Pinhole look-at camera for the raymarcher.
#[derive(Clone, Copy, Debug)]
pub struct RayCam {
    pub eye: Vec3,
    pub target: Vec3,
    /// Vertical field of view in radians.
    pub fov: f64,
}

impl RayCam {
    pub fn new(eye: Vec3, target: Vec3, fov_deg: f64) -> Self {
        Self {
            eye,
            target,
            fov: fov_deg * DEGREES,
        }
    }
    /// Orbiting camera at `angle` around `target`.
    pub fn orbit(target: Vec3, radius: f64, height: f64, angle: f64, fov_deg: f64) -> Self {
        Self::new(
            target + v3(radius * angle.cos(), height, radius * angle.sin()),
            target,
            fov_deg,
        )
    }
    /// Ray direction for normalised screen coords (`uv.y` in `[-1,1]`, `uv.x` scaled by aspect).
    pub fn ray(&self, uv: Vec2) -> Vec3 {
        let f = (self.target - self.eye).normalize();
        let r = f.cross(v3(0.0, 1.0, 0.0)).normalize();
        let u = r.cross(f);
        let k = (self.fov / 2.0).tan();
        (f + r * (uv.x * k) + u * (uv.y * k)).normalize()
    }
}

/// Lighting / quality options.
#[derive(Clone, Copy, Debug)]
pub struct MarchOpts {
    /// Direction *towards* the sun.
    pub sun: Vec3,
    pub sun_color: Color,
    pub sky_top: Color,
    pub sky_horizon: Color,
    /// Exponential fog density (0 = none).
    pub fog: f64,
    pub shadows: bool,
    pub ao: bool,
    pub max_steps: usize,
    pub max_dist: f64,
    /// Rim / fresnel light strength.
    pub rim: f64,
}

impl Default for MarchOpts {
    fn default() -> Self {
        Self {
            sun: v3(0.6, 0.8, 0.4).normalize(),
            sun_color: Color::rgb(1.0, 0.95, 0.85),
            sky_top: rgb8(40, 70, 140),
            sky_horizon: rgb8(230, 170, 140),
            fog: 0.02,
            shadows: true,
            ao: true,
            max_steps: 110,
            max_dist: 60.0,
            rim: 0.35,
        }
    }
}

fn sky(o: &MarchOpts, rd: Vec3) -> Color {
    let t = rd.y.clamp(0.0, 1.0).powf(0.6) as f32;
    let base = o.sky_horizon.mix(o.sky_top, t);
    let sun = rd.dot(o.sun).max(0.0).powf(200.0) as f32;
    base.mix(o.sun_color, sun.min(1.0))
}

/// March one ray; returns the hit distance.
pub fn march(sdf: &Sdf, ro: Vec3, rd: Vec3, o: &MarchOpts) -> Option<f64> {
    let mut t = 0.0;
    for _ in 0..o.max_steps {
        let d = sdf.dist(ro + rd * t);
        if d < 1e-3 * (1.0 + t) {
            return Some(t);
        }
        t += d;
        if t > o.max_dist {
            break;
        }
    }
    None
}

fn soft_shadow(sdf: &Sdf, ro: Vec3, rd: Vec3, k: f64) -> f64 {
    let mut res: f64 = 1.0;
    let mut t = 0.02;
    for _ in 0..48 {
        let h = sdf.dist(ro + rd * t);
        if h < 1e-4 {
            return 0.0;
        }
        res = res.min(k * h / t);
        t += h.clamp(0.02, 0.5);
        if t > 20.0 {
            break;
        }
    }
    res.clamp(0.0, 1.0)
}

fn ambient_occlusion(sdf: &Sdf, p: Vec3, n: Vec3) -> f64 {
    let mut occ = 0.0;
    let mut w = 1.0;
    for i in 1..=5 {
        let h = 0.03 + 0.12 * i as f64;
        occ += (h - sdf.dist(p + n * h)) * w;
        w *= 0.7;
    }
    (1.0 - 2.0 * occ).clamp(0.0, 1.0)
}

/// Shade one pixel for screen coords `uv`.
pub fn shade(sdf: &Sdf, cam: &RayCam, o: &MarchOpts, uv: Vec2) -> Color {
    let rd = cam.ray(uv);
    let ro = cam.eye;
    let Some(t) = march(sdf, ro, rd, o) else {
        return sky(o, rd);
    };
    let p = ro + rd * t;
    let n = sdf.normal(p);
    let (_, albedo) = sdf.eval(p);
    let diff = n.dot(o.sun).max(0.0);
    let sh = if o.shadows && diff > 0.0 {
        soft_shadow(sdf, p + n * 2e-3, o.sun, 10.0)
    } else {
        1.0
    };
    let ao = if o.ao {
        ambient_occlusion(sdf, p, n)
    } else {
        1.0
    };
    let half = (o.sun - rd).normalize();
    let spec = n.dot(half).max(0.0).powf(48.0) * sh;
    let sky_amb = 0.5 + 0.5 * n.y;
    let fres = (1.0 + n.dot(rd)).clamp(0.0, 1.0).powf(3.0) * o.rim;
    let mut c = [albedo.r as f64, albedo.g as f64, albedo.b as f64];
    let sc = [
        o.sun_color.r as f64,
        o.sun_color.g as f64,
        o.sun_color.b as f64,
    ];
    let kc = [o.sky_top.r as f64, o.sky_top.g as f64, o.sky_top.b as f64];
    for i in 0..3 {
        c[i] = c[i] * (sc[i] * diff * sh * 1.1 + kc[i] * sky_amb * 0.45 * ao + 0.04)
            + sc[i] * spec * 0.6
            + fres * kc[i];
    }
    let lit = Color::rgb(c[0] as f32, c[1] as f32, c[2] as f32);
    // fog towards the horizon colour; simple filmic-ish tone curve
    let fog = 1.0 - (-o.fog * t * t * 0.1).exp();
    let out = lit.mix(sky(o, rd), fog as f32);
    let tm = |x: f32| (x / (1.0 + x * 0.35)).powf(0.95);
    Color::rgb(tm(out.r), tm(out.g), tm(out.b))
}

/// Render the SDF scene into `rect` at `px_w` pixels wide.
pub fn raymarch_image(sdf: &Sdf, cam: &RayCam, o: &MarchOpts, rect: Rect, px_w: u32) -> Mobject {
    let c = rect.center();
    let hh = rect.height() / 2.0;
    field_image(rect, px_w, |p| shade(sdf, cam, o, (p - c) * (1.0 / hh)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_distance() {
        assert!((sd_sphere(1.0).dist(v3(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-12);
        assert!((sd_box(v3(1.0, 1.0, 1.0), 0.0).dist(v3(3.0, 0.0, 0.0)) - 2.0).abs() < 1e-12);
        assert!((sd_torus(2.0, 0.5).dist(v3(2.0, 0.0, 0.0)) + 0.5).abs() < 1e-12);
        let moved = sd_sphere(1.0).at(v3(0.0, 5.0, 0.0));
        assert!(moved.dist(v3(0.0, 5.0, 0.0)) < -0.99);
        let cut = sd_sphere(1.0).subtract(sd_sphere(0.5));
        assert!(cut.dist(Vec3::ZERO) > 0.0);
        let r = sd_sphere(0.2).repeat(v3(1.0, 0.0, 1.0));
        assert!(r.dist(v3(3.0, 0.0, -2.0)) < -0.19);
    }

    #[test]
    fn march_hits_sphere_and_misses_sky() {
        let s = sd_sphere(1.0);
        let o = MarchOpts::default();
        let t = march(&s, v3(0.0, 0.0, -5.0), v3(0.0, 0.0, 1.0), &o).unwrap();
        assert!((t - 4.0).abs() < 1e-2);
        assert!(march(&s, v3(0.0, 3.0, -5.0), v3(0.0, 0.0, 1.0), &o).is_none());
        let n = s.normal(v3(1.0, 0.0, 0.0));
        assert!((n - v3(1.0, 0.0, 0.0)).length() < 1e-3);
    }
}
