//! Vector maths, affine transforms and scalar helpers.
//!
//! The scene coordinate system follows manim: the origin is the centre of the
//! frame, `y` points up, and the default frame is 8 units tall.

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = std::f64::consts::TAU;
pub const DEGREES: f64 = PI / 180.0;

/// 2D vector / point in scene units.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

pub const ORIGIN: Vec2 = Vec2 { x: 0.0, y: 0.0 };
pub const UP: Vec2 = Vec2 { x: 0.0, y: 1.0 };
pub const DOWN: Vec2 = Vec2 { x: 0.0, y: -1.0 };
pub const LEFT: Vec2 = Vec2 { x: -1.0, y: 0.0 };
pub const RIGHT: Vec2 = Vec2 { x: 1.0, y: 0.0 };
pub const UL: Vec2 = Vec2 { x: -1.0, y: 1.0 };
pub const UR: Vec2 = Vec2 { x: 1.0, y: 1.0 };
pub const DL: Vec2 = Vec2 { x: -1.0, y: -1.0 };
pub const DR: Vec2 = Vec2 { x: 1.0, y: -1.0 };

/// Shorthand constructor.
#[inline]
pub const fn v2(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

impl Vec2 {
    pub const ZERO: Vec2 = ORIGIN;

    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    #[inline]
    pub fn from_angle(theta: f64) -> Self {
        Self::new(theta.cos(), theta.sin())
    }
    #[inline]
    pub fn polar(r: f64, theta: f64) -> Self {
        Self::new(r * theta.cos(), r * theta.sin())
    }
    #[inline]
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y
    }
    #[inline]
    pub fn cross(self, o: Self) -> f64 {
        self.x * o.y - self.y * o.x
    }
    #[inline]
    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }
    #[inline]
    pub fn length_sq(self) -> f64 {
        self.dot(self)
    }
    #[inline]
    pub fn distance(self, o: Self) -> f64 {
        (self - o).length()
    }
    pub fn normalize(self) -> Self {
        let l = self.length();
        if l < 1e-12 {
            Self::ZERO
        } else {
            self / l
        }
    }
    /// Counter-clockwise perpendicular.
    #[inline]
    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }
    #[inline]
    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }
    pub fn rotate(self, theta: f64) -> Self {
        let (s, c) = theta.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
    pub fn rotate_about(self, theta: f64, about: Vec2) -> Self {
        (self - about).rotate(theta) + about
    }
    #[inline]
    pub fn lerp(self, o: Self, t: f64) -> Self {
        self + (o - self) * t
    }
    #[inline]
    pub fn mul_elem(self, o: Self) -> Self {
        Self::new(self.x * o.x, self.y * o.y)
    }
    pub fn min(self, o: Self) -> Self {
        Self::new(self.x.min(o.x), self.y.min(o.y))
    }
    pub fn max(self, o: Self) -> Self {
        Self::new(self.x.max(o.x), self.y.max(o.y))
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
    pub fn extend(self, z: f64) -> Vec3 {
        Vec3::new(self.x, self.y, z)
    }
}

macro_rules! impl_vec_ops {
    ($t:ident { $($f:ident),+ }) => {
        impl Add for $t { type Output = $t; #[inline] fn add(self, o: $t) -> $t { $t { $($f: self.$f + o.$f),+ } } }
        impl Sub for $t { type Output = $t; #[inline] fn sub(self, o: $t) -> $t { $t { $($f: self.$f - o.$f),+ } } }
        impl Mul<f64> for $t { type Output = $t; #[inline] fn mul(self, s: f64) -> $t { $t { $($f: self.$f * s),+ } } }
        impl Mul<$t> for f64 { type Output = $t; #[inline] fn mul(self, v: $t) -> $t { v * self } }
        impl Div<f64> for $t { type Output = $t; #[inline] fn div(self, s: f64) -> $t { $t { $($f: self.$f / s),+ } } }
        impl Neg for $t { type Output = $t; #[inline] fn neg(self) -> $t { $t { $($f: -self.$f),+ } } }
        impl AddAssign for $t { #[inline] fn add_assign(&mut self, o: $t) { $(self.$f += o.$f;)+ } }
        impl SubAssign for $t { #[inline] fn sub_assign(&mut self, o: $t) { $(self.$f -= o.$f;)+ } }
        impl MulAssign<f64> for $t { #[inline] fn mul_assign(&mut self, s: f64) { $(self.$f *= s;)+ } }
    };
}
impl_vec_ops!(Vec2 { x, y });
impl_vec_ops!(Vec3 { x, y, z });

impl From<(f64, f64)> for Vec2 {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}
impl From<[f64; 2]> for Vec2 {
    fn from([x, y]: [f64; 2]) -> Self {
        Self::new(x, y)
    }
}

/// 3D vector (used by the 3D module and physics simulations).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[inline]
pub const fn v3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    pub const ZERO: Vec3 = v3(0.0, 0.0, 0.0);
    pub const X: Vec3 = v3(1.0, 0.0, 0.0);
    pub const Y: Vec3 = v3(0.0, 1.0, 0.0);
    pub const Z: Vec3 = v3(0.0, 0.0, 1.0);

    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    #[inline]
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    #[inline]
    pub fn cross(self, o: Self) -> Self {
        Self::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    #[inline]
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn normalize(self) -> Self {
        let l = self.length();
        if l < 1e-12 {
            Self::ZERO
        } else {
            self / l
        }
    }
    #[inline]
    pub fn lerp(self, o: Self, t: f64) -> Self {
        self + (o - self) * t
    }
    pub fn xy(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
    /// Rotation about an arbitrary axis (Rodrigues).
    pub fn rotate_axis(self, axis: Vec3, theta: f64) -> Self {
        let k = axis.normalize();
        let (s, c) = theta.sin_cos();
        self * c + k.cross(self) * s + k * (k.dot(self) * (1.0 - c))
    }
}

/// 2D affine transform `[a c e; b d f; 0 0 1]` (same layout as SVG / tiny-skia).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Default for Affine {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };
    pub fn translate(v: Vec2) -> Self {
        Self {
            e: v.x,
            f: v.y,
            ..Self::IDENTITY
        }
    }
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            d: sy,
            ..Self::IDENTITY
        }
    }
    pub fn rotate(theta: f64) -> Self {
        let (s, c) = theta.sin_cos();
        Self {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }
    pub fn about(self, p: Vec2) -> Self {
        Affine::translate(p)
            .then_after(self)
            .then_after(Affine::translate(-p))
    }
    /// `self ∘ other`: applies `other` first, then `self`.
    pub fn then_after(self, o: Affine) -> Affine {
        Affine {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }
    /// Applies `self` first, then `o`.
    pub fn then(self, o: Affine) -> Affine {
        o.then_after(self)
    }
    #[inline]
    pub fn apply(&self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }
    pub fn apply_vector(&self, p: Vec2) -> Vec2 {
        Vec2::new(self.a * p.x + self.c * p.y, self.b * p.x + self.d * p.y)
    }
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }
    pub fn invert(&self) -> Option<Affine> {
        let det = self.determinant();
        if det.abs() < 1e-15 {
            return None;
        }
        let inv = 1.0 / det;
        let a = self.d * inv;
        let b = -self.b * inv;
        let c = -self.c * inv;
        let d = self.a * inv;
        Some(Affine {
            a,
            b,
            c,
            d,
            e: -(a * self.e + c * self.f),
            f: -(b * self.e + d * self.f),
        })
    }
    /// Mean scale factor (used to scale stroke widths).
    pub fn mean_scale(&self) -> f64 {
        self.determinant().abs().sqrt()
    }
}

#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[inline]
pub fn clamp01(t: f64) -> f64 {
    t.clamp(0.0, 1.0)
}

/// Inverse of [`lerp`].
#[inline]
pub fn inverse_lerp(a: f64, b: f64, v: f64) -> f64 {
    if (b - a).abs() < 1e-15 {
        0.0
    } else {
        (v - a) / (b - a)
    }
}

/// Smooth Hermite step between `e0` and `e1`.
pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = clamp01(inverse_lerp(e0, e1, x));
    t * t * (3.0 - 2.0 * t)
}

/// `n` evenly spaced samples from `a` to `b` inclusive.
pub fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    match n {
        0 => vec![],
        1 => vec![a],
        _ => (0..n)
            .map(|i| a + (b - a) * i as f64 / (n - 1) as f64)
            .collect(),
    }
}

/// Values from `a` (inclusive) to `b` (inclusive when it lands on the grid) with step `s`.
pub fn arange(a: f64, b: f64, s: f64) -> Vec<f64> {
    let mut out = Vec::new();
    if s <= 0.0 {
        return out;
    }
    let n = ((b - a) / s + 1e-9).floor() as i64;
    for i in 0..=n {
        out.push(a + i as f64 * s);
    }
    out
}

/// Binomial coefficient as f64.
pub fn choose(n: u32, k: u32) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut r = 1.0;
    for i in 0..k {
        r = r * (n - i) as f64 / (i + 1) as f64;
    }
    r
}

/// Deterministic pseudo random generator (xorshift64*), so renders are reproducible.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next_f64()
    }
    /// Standard normal sample (Box–Muller).
    pub fn normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_roundtrip() {
        let t = Affine::rotate(0.7)
            .then(Affine::scale(2.0, 3.0))
            .then(Affine::translate(v2(1.0, -2.0)));
        let p = v2(0.3, 0.9);
        let q = t.invert().unwrap().apply(t.apply(p));
        assert!((p - q).length() < 1e-12);
    }

    #[test]
    fn rotation_about_point() {
        let p = v2(2.0, 1.0).rotate_about(PI / 2.0, v2(1.0, 1.0));
        assert!((p - v2(1.0, 2.0)).length() < 1e-12);
        let a = Affine::rotate(PI / 2.0).about(v2(1.0, 1.0));
        assert!((a.apply(v2(2.0, 1.0)) - v2(1.0, 2.0)).length() < 1e-12);
    }

    #[test]
    fn helpers() {
        assert_eq!(linspace(0.0, 1.0, 3), vec![0.0, 0.5, 1.0]);
        assert_eq!(arange(0.0, 1.0, 0.5).len(), 3);
        assert_eq!(choose(5, 2), 10.0);
        let mut r = Rng::new(1);
        let x = r.next_f64();
        assert!((0.0..1.0).contains(&x));
        assert!((v3(1.0, 0.0, 0.0).rotate_axis(Vec3::Z, PI / 2.0) - Vec3::Y).length() < 1e-12);
    }
}
