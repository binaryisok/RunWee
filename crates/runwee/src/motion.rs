//! Motion core: keyframe tracks, physical springs, spring followers, After Effects style
//! expressions (`wiggle`, inertial bounce), GSAP-style staggers and timelines.
//!
//! Everything here is a pure function of time, so it works with RunWee's parallel,
//! random-access renderer.

use crate::anim::*;
use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::rate::Rate;
use crate::scene::{Obj, Scene, Tracker};
use std::sync::Arc;

// ------------------------------------------------------------------- Lerp

/// Values that can be interpolated by keyframe tracks.
pub trait Lerp: Clone + Send + Sync + 'static {
    fn lerp_to(&self, o: &Self, t: f64) -> Self;
    /// Catmull–Rom through `p0..p3` at `t ∈ [0,1]` between `p1` and `p2`.
    /// Defaults to linear interpolation for types without a vector space.
    fn catmull(_p0: &Self, p1: &Self, p2: &Self, _p3: &Self, t: f64) -> Self {
        p1.lerp_to(p2, t)
    }
}

fn cr(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

impl Lerp for f64 {
    fn lerp_to(&self, o: &Self, t: f64) -> Self {
        self + (o - self) * t
    }
    fn catmull(p0: &Self, p1: &Self, p2: &Self, p3: &Self, t: f64) -> Self {
        cr(*p0, *p1, *p2, *p3, t)
    }
}
impl Lerp for f32 {
    fn lerp_to(&self, o: &Self, t: f64) -> Self {
        self + (o - self) * t as f32
    }
}
impl Lerp for Vec2 {
    fn lerp_to(&self, o: &Self, t: f64) -> Self {
        self.lerp(*o, t)
    }
    fn catmull(p0: &Self, p1: &Self, p2: &Self, p3: &Self, t: f64) -> Self {
        v2(cr(p0.x, p1.x, p2.x, p3.x, t), cr(p0.y, p1.y, p2.y, p3.y, t))
    }
}
impl Lerp for Vec3 {
    fn lerp_to(&self, o: &Self, t: f64) -> Self {
        self.lerp(*o, t)
    }
    fn catmull(p0: &Self, p1: &Self, p2: &Self, p3: &Self, t: f64) -> Self {
        v3(
            cr(p0.x, p1.x, p2.x, p3.x, t),
            cr(p0.y, p1.y, p2.y, p3.y, t),
            cr(p0.z, p1.z, p2.z, p3.z, t),
        )
    }
}
impl Lerp for Color {
    fn lerp_to(&self, o: &Self, t: f64) -> Self {
        self.mix(*o, t as f32)
    }
}

// -------------------------------------------------------------- keyframes

/// How a track behaves after its last key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeyLoop {
    /// Hold the last value.
    #[default]
    Hold,
    /// Start over.
    Repeat,
    /// Play forwards then backwards.
    PingPong,
}

/// Interpolation between two keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Interp {
    /// Eased straight interpolation.
    #[default]
    Linear,
    /// Smooth Catmull–Rom spline through the neighbouring keys (AE "auto bezier").
    Smooth,
    /// Jump at the key (AE "hold" keyframe).
    Hold,
}

#[derive(Clone)]
struct Key<T> {
    t: f64,
    v: T,
    /// Easing used for the segment *arriving* at this key.
    ease: Rate,
    interp: Interp,
}

/// A keyframe track: values at times, eased per segment (After Effects / Lottie keyframes,
/// Motion Canvas tweens). `T` is anything implementing [`Lerp`].
///
/// ```
/// use runwee::prelude::*;
/// let k = Keyframes::new(0.0_f64).key(1.0, 10.0, Rate::Linear).key(2.0, 0.0, Rate::Smooth);
/// assert!((k.at(0.5) - 5.0).abs() < 1e-9);
/// ```
#[derive(Clone)]
pub struct Keyframes<T: Lerp> {
    keys: Vec<Key<T>>,
    pub looping: KeyLoop,
}

impl<T: Lerp> Keyframes<T> {
    /// A track starting at `v` at time 0.
    pub fn new(v: T) -> Self {
        Self {
            keys: vec![Key {
                t: 0.0,
                v,
                ease: Rate::Linear,
                interp: Interp::Linear,
            }],
            looping: KeyLoop::Hold,
        }
    }
    /// Add a key at absolute time `t` reached with `ease`.
    pub fn key(mut self, t: f64, v: T, ease: Rate) -> Self {
        self.push(t, v, ease, Interp::Linear);
        self
    }
    /// Add a key reached along a smooth spline (Catmull–Rom through the neighbours).
    pub fn smooth_key(mut self, t: f64, v: T) -> Self {
        self.push(t, v, Rate::Linear, Interp::Smooth);
        self
    }
    /// Add a key that the value jumps to at `t`.
    pub fn hold_key(mut self, t: f64, v: T) -> Self {
        self.push(t, v, Rate::Linear, Interp::Hold);
        self
    }
    /// Add a key `dt` seconds after the last one.
    pub fn then(self, dt: f64, v: T, ease: Rate) -> Self {
        let t = self.duration() + dt;
        self.key(t, v, ease)
    }
    /// Stay at the current value for `dt` seconds.
    pub fn hold(self, dt: f64) -> Self {
        let v = self.keys.last().unwrap().v.clone();
        self.then(dt, v, Rate::Linear)
    }
    pub fn looped(mut self, l: KeyLoop) -> Self {
        self.looping = l;
        self
    }
    fn push(&mut self, t: f64, v: T, ease: Rate, interp: Interp) {
        let k = Key { t, v, ease, interp };
        let i = self.keys.partition_point(|x| x.t <= t);
        self.keys.insert(i, k);
    }
    /// Time of the last key.
    pub fn duration(&self) -> f64 {
        self.keys.last().map(|k| k.t).unwrap_or(0.0)
    }
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    fn local_time(&self, t: f64) -> f64 {
        let d = self.duration();
        if d <= 0.0 || t <= d {
            return t;
        }
        match self.looping {
            KeyLoop::Hold => t,
            KeyLoop::Repeat => t.rem_euclid(d),
            KeyLoop::PingPong => {
                let u = t.rem_euclid(2.0 * d);
                if u > d {
                    2.0 * d - u
                } else {
                    u
                }
            }
        }
    }
    /// Value at time `t`.
    pub fn at(&self, t: f64) -> T {
        let t = self.local_time(t);
        let ks = &self.keys;
        if t <= ks[0].t {
            return ks[0].v.clone();
        }
        let i = ks.partition_point(|k| k.t <= t);
        if i >= ks.len() {
            return ks[ks.len() - 1].v.clone();
        }
        let (a, b) = (&ks[i - 1], &ks[i]);
        let span = (b.t - a.t).max(1e-12);
        let u = b.ease.eval((t - a.t) / span);
        match b.interp {
            Interp::Hold => a.v.clone(),
            Interp::Linear => a.v.lerp_to(&b.v, u),
            Interp::Smooth => {
                let p0 = if i >= 2 { &ks[i - 2].v } else { &a.v };
                let p3 = if i + 1 < ks.len() { &ks[i + 1].v } else { &b.v };
                T::catmull(p0, &a.v, &b.v, p3, u)
            }
        }
    }
    /// The track as a shareable closure.
    pub fn into_fn(self) -> Arc<dyn Fn(f64) -> T + Send + Sync> {
        Arc::new(move |t| self.at(t))
    }
}

// ------------------------------------------------------------------ spring

/// A damped harmonic oscillator solved in closed form (`m x'' + c x' + k x = 0`).
/// Unlike Remotion's frame-stepped `spring()`, this one takes an initial velocity and
/// handles under-, critically- and over-damped regimes exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub mass: f64,
    pub stiffness: f64,
    pub damping: f64,
}

impl Default for Spring {
    fn default() -> Self {
        Spring::new(1.0, 170.0, 26.0)
    }
}

impl Spring {
    pub fn new(mass: f64, stiffness: f64, damping: f64) -> Self {
        Self {
            mass: mass.max(1e-6),
            stiffness: stiffness.max(1e-6),
            damping: damping.max(0.0),
        }
    }
    /// From perceptual parameters: `response` ≈ period in seconds, `damping_ratio` ζ
    /// (1 = critical, < 1 bouncy) — the SwiftUI / Apple spring model.
    pub fn from_response(response: f64, damping_ratio: f64) -> Self {
        let w = TAU / response.max(1e-3);
        Spring::new(1.0, w * w, 2.0 * damping_ratio * w)
    }
    pub fn bouncy() -> Self {
        Spring::from_response(0.5, 0.35)
    }
    pub fn snappy() -> Self {
        Spring::from_response(0.3, 0.85)
    }
    pub fn gentle() -> Self {
        Spring::from_response(0.9, 0.9)
    }
    pub fn wobbly() -> Self {
        Spring::from_response(0.7, 0.18)
    }
    pub fn omega(&self) -> f64 {
        (self.stiffness / self.mass).sqrt()
    }
    pub fn zeta(&self) -> f64 {
        self.damping / (2.0 * (self.stiffness * self.mass).sqrt())
    }
    /// Displacement and velocity at time `t` for initial displacement `x0` (from rest
    /// position) and velocity `v0`.
    pub fn state(&self, t: f64, x0: f64, v0: f64) -> (f64, f64) {
        if t <= 0.0 {
            return (x0, v0);
        }
        let w = self.omega();
        let z = self.zeta();
        if (z - 1.0).abs() < 1e-6 {
            // critical
            let b = v0 + w * x0;
            let e = (-w * t).exp();
            let x = (x0 + b * t) * e;
            let v = (b - w * (x0 + b * t)) * e;
            (x, v)
        } else if z < 1.0 {
            let wd = w * (1.0 - z * z).sqrt();
            let a = x0;
            let b = (v0 + z * w * x0) / wd;
            let e = (-z * w * t).exp();
            let (s, c) = (wd * t).sin_cos();
            let x = e * (a * c + b * s);
            let v = e * ((-z * w) * (a * c + b * s) + (-a * wd * s + b * wd * c));
            (x, v)
        } else {
            let r = w * (z * z - 1.0).sqrt();
            let r1 = -z * w + r;
            let r2 = -z * w - r;
            let c2 = (v0 - r1 * x0) / (r2 - r1);
            let c1 = x0 - c2;
            let (e1, e2) = ((r1 * t).exp(), (r2 * t).exp());
            (c1 * e1 + c2 * e2, c1 * r1 * e1 + c2 * r2 * e2)
        }
    }
    /// Value moving from `from` to `to` starting at rest (or with `velocity`).
    pub fn value(&self, t: f64, from: f64, to: f64, velocity: f64) -> f64 {
        to + self.state(t, from - to, velocity).0
    }
    /// Progress 0 → 1 (overshoots for bouncy springs).
    pub fn progress(&self, t: f64) -> f64 {
        self.value(t, 0.0, 1.0, 0.0)
    }
    /// Time after which the motion stays within `eps` of rest.
    pub fn settle_time(&self, eps: f64) -> f64 {
        let mut t = 0.0;
        let mut last = 0.0;
        let dt = 1.0 / 240.0;
        while t < 30.0 {
            let (x, v) = self.state(t, 1.0, 0.0);
            if x.abs() > eps || v.abs() > eps * 10.0 {
                last = t;
            }
            if t - last > 1.0 {
                break;
            }
            t += dt;
        }
        last
    }
    /// The spring as an easing curve over `duration` seconds (for `.rate(...)`).
    pub fn rate(self, duration: f64) -> Rate {
        Rate::custom(move |u| self.progress(u * duration))
    }
    /// Use the spring's own settle time as the curve length.
    pub fn as_rate(self) -> Rate {
        let d = self.settle_time(1e-3).max(0.05);
        self.rate(d)
    }
}

/// A spring that *chases* a moving target `target(t)`, like Rive / Framer Motion
/// `useSpring`. Integrated deterministically from `t0` with a fixed step, so the value at
/// any time is reproducible (and frames still render in any order).
pub fn spring_follow(target: &dyn Fn(f64) -> f64, spring: Spring, t0: f64, t: f64) -> f64 {
    let dt = 1.0 / 240.0;
    let mut x = target(t0);
    let mut v = 0.0;
    let mut tt = t0;
    let (k, c, m) = (spring.stiffness, spring.damping, spring.mass);
    while tt + dt <= t {
        let goal = target(tt);
        // semi-implicit Euler: stable for stiff springs at this step
        let a = (-k * (x - goal) - c * v) / m;
        v += a * dt;
        x += v * dt;
        tt += dt;
    }
    let rem = t - tt;
    if rem > 0.0 {
        let a = (-k * (x - target(tt)) - c * v) / m;
        let v2 = v + a * rem;
        x += v2 * rem;
    }
    x
}

/// 2D version of [`spring_follow`].
pub fn spring_follow2(target: &dyn Fn(f64) -> Vec2, spring: Spring, t0: f64, t: f64) -> Vec2 {
    let x = spring_follow(&|s| target(s).x, spring, t0, t);
    let y = spring_follow(&|s| target(s).y, spring, t0, t);
    v2(x, y)
}

// ------------------------------------------------------- AE expressions

fn hash11(n: i64, seed: u64) -> f64 {
    let mut h =
        (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ seed.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 29;
    (h >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// Smooth 1D value noise in [-1, 1].
pub fn noise1(x: f64, seed: u64) -> f64 {
    let i = x.floor();
    let f = x - i;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = hash11(i as i64, seed);
    let b = hash11(i as i64 + 1, seed);
    a + (b - a) * u
}

/// After Effects' `wiggle_value(freq, amp)`: smooth fractal random motion (2 octaves),
/// `freq` wiggles per second, peak amplitude about `amp`.
pub fn wiggle_value(t: f64, freq: f64, amp: f64, seed: u64) -> f64 {
    let x = t * freq;
    (noise1(x, seed) * 0.75 + noise1(x * 2.0 + 17.3, seed ^ 0xA5) * 0.25) * amp
}

/// 2D wiggle (independent channels).
pub fn wiggle_vec(t: f64, freq: f64, amp: f64, seed: u64) -> Vec2 {
    v2(
        wiggle_value(t, freq, amp, seed),
        wiggle_value(t, freq, amp, seed.wrapping_add(7919)),
    )
}

/// The classic AE "inertial bounce" expression: after an impulse at `t = 0` with
/// velocity `v`, oscillate with `freq` Hz and exponential `decay`.
pub fn inertial_bounce(t: f64, v: f64, freq: f64, decay: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    v * (freq * t * TAU).sin() / (decay * t).exp() / (freq * TAU)
}

/// Periodic waveforms in [-1, 1] with period 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Triangle,
    Square,
    Saw,
}

pub fn oscillate(t: f64, freq: f64, wave: Wave) -> f64 {
    let p = (t * freq).rem_euclid(1.0);
    match wave {
        Wave::Sine => (p * TAU).sin(),
        Wave::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
        Wave::Square => {
            if p < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Wave::Saw => 2.0 * p - 1.0,
    }
}

/// Cosine palette by Inigo Quilez: `a + b·cos(2π(c·t + d))` per channel.
pub fn cosine_palette(t: f64, a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> Color {
    let ch = |i: usize| (a[i] + b[i] * (TAU * (c[i] * t + d[i])).cos()).clamp(0.0, 1.0) as f32;
    Color::rgb(ch(0), ch(1), ch(2))
}

/// A few ready-made cosine palettes ("rainbow", "sunset", "ocean", "neon", "fire", "earth").
pub fn palette(name: &str, t: f64) -> Color {
    let (a, b, c, d) = match name {
        "sunset" => (
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 0.5],
            [0.8, 0.9, 0.3],
        ),
        "ocean" => (
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.3, 0.2, 0.2],
        ),
        "neon" => (
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [2.0, 1.0, 0.0],
            [0.5, 0.2, 0.25],
        ),
        "fire" => (
            [0.5, 0.3, 0.1],
            [0.5, 0.4, 0.2],
            [1.0, 1.0, 1.0],
            [0.0, 0.1, 0.2],
        ),
        "earth" => (
            [0.8, 0.5, 0.4],
            [0.2, 0.4, 0.2],
            [2.0, 1.0, 1.0],
            [0.0, 0.25, 0.25],
        ),
        "ice" => (
            [0.6, 0.75, 0.9],
            [0.3, 0.25, 0.1],
            [1.0, 1.0, 1.0],
            [0.0, 0.1, 0.2],
        ),
        _ => (
            [0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 1.0],
            [0.0, 0.33, 0.67],
        ),
    };
    cosine_palette(t, a, b, c, d)
}

// ---------------------------------------------------------------- stagger

/// Where a stagger wave starts (GSAP `stagger.from`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StaggerFrom {
    Start,
    End,
    Center,
    Edges,
    /// Seeded shuffle.
    Random(u64),
    /// Distance from item `i`.
    Index(usize),
    /// Distance from a scene point (needs positions: [`Stagger::delays_for_points`]).
    Point(Vec2),
}

/// Distributes start delays over many items (GSAP's `stagger`).
#[derive(Clone)]
pub struct Stagger {
    /// Delay between neighbouring items (seconds).
    pub each: f64,
    pub from: StaggerFrom,
    /// Optional easing of the delay distribution (bunch delays together at one end).
    pub ease: Rate,
    /// Grid shape `(cols, rows)`: distances are measured in 2D.
    pub grid: Option<(usize, usize)>,
}

impl Stagger {
    pub fn each(each: f64) -> Self {
        Self {
            each,
            from: StaggerFrom::Start,
            ease: Rate::Linear,
            grid: None,
        }
    }
    pub fn from(mut self, f: StaggerFrom) -> Self {
        self.from = f;
        self
    }
    pub fn ease(mut self, r: Rate) -> Self {
        self.ease = r;
        self
    }
    pub fn grid(mut self, cols: usize, rows: usize) -> Self {
        self.grid = Some((cols.max(1), rows.max(1)));
        self
    }
    fn shape(&self, dist: Vec<f64>) -> Vec<f64> {
        let max = dist.iter().cloned().fold(0.0, f64::max);
        if max <= 0.0 {
            return vec![0.0; dist.len()];
        }
        let total = self.each * max;
        dist.iter()
            .map(|d| self.ease.eval(d / max) * total)
            .collect()
    }
    /// Start delays for `n` items.
    pub fn delays(&self, n: usize) -> Vec<f64> {
        if n == 0 {
            return vec![];
        }
        if let Some((cols, _)) = self.grid {
            let pts: Vec<Vec2> = (0..n)
                .map(|i| v2((i % cols) as f64, -((i / cols) as f64)))
                .collect();
            let origin = match self.from {
                StaggerFrom::Start => pts[0],
                StaggerFrom::End => pts[n - 1],
                StaggerFrom::Index(i) => pts[i.min(n - 1)],
                StaggerFrom::Point(p) => p,
                StaggerFrom::Center | StaggerFrom::Edges | StaggerFrom::Random(_) => {
                    pts.iter().fold(Vec2::ZERO, |a, b| a + *b) / n as f64
                }
            };
            let mut d: Vec<f64> = pts.iter().map(|p| p.distance(origin)).collect();
            if let StaggerFrom::Edges = self.from {
                let m = d.iter().cloned().fold(0.0, f64::max);
                d.iter_mut().for_each(|x| *x = m - *x);
            }
            if let StaggerFrom::Random(seed) = self.from {
                return self.random_order(n, seed);
            }
            return self.shape(d);
        }
        let last = (n - 1) as f64;
        let d: Vec<f64> = match self.from {
            StaggerFrom::Start => (0..n).map(|i| i as f64).collect(),
            StaggerFrom::End => (0..n).map(|i| last - i as f64).collect(),
            StaggerFrom::Center => (0..n).map(|i| (i as f64 - last / 2.0).abs()).collect(),
            StaggerFrom::Edges => (0..n)
                .map(|i| last / 2.0 - (i as f64 - last / 2.0).abs())
                .collect(),
            StaggerFrom::Index(k) => (0..n).map(|i| (i as f64 - k as f64).abs()).collect(),
            StaggerFrom::Point(_) => (0..n).map(|i| i as f64).collect(),
            StaggerFrom::Random(seed) => return self.random_order(n, seed),
        };
        self.shape(d)
    }
    fn random_order(&self, n: usize, seed: u64) -> Vec<f64> {
        let mut idx: Vec<usize> = (0..n).collect();
        let mut rng = Rng::new(seed);
        for i in (1..n).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            idx.swap(i, j);
        }
        let mut d = vec![0.0; n];
        for (rank, &i) in idx.iter().enumerate() {
            d[i] = rank as f64;
        }
        self.shape(d)
    }
    /// Delays from distances to `from` (a point, or the centroid/extremes of `pts`).
    pub fn delays_for_points(&self, pts: &[Vec2]) -> Vec<f64> {
        let n = pts.len();
        if n == 0 {
            return vec![];
        }
        let centroid = pts.iter().fold(Vec2::ZERO, |a, b| a + *b) / n as f64;
        let origin = match self.from {
            StaggerFrom::Point(p) => p,
            StaggerFrom::Index(i) => pts[i.min(n - 1)],
            StaggerFrom::Start => pts[0],
            StaggerFrom::End => pts[n - 1],
            StaggerFrom::Random(seed) => return self.random_order(n, seed),
            _ => centroid,
        };
        let mut d: Vec<f64> = pts.iter().map(|p| p.distance(origin)).collect();
        if let StaggerFrom::Edges = self.from {
            let m = d.iter().cloned().fold(0.0, f64::max);
            d.iter_mut().for_each(|x| *x = m - *x);
        }
        // scene distances → "item units" so `each` keeps meaning roughly "per item"
        let spacing = {
            let mut s: Vec<f64> = d.iter().cloned().filter(|x| *x > 1e-9).collect();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            s.first().cloned().unwrap_or(1.0)
        };
        self.shape(d.into_iter().map(|x| x / spacing).collect())
    }
}

/// Build one animation per object and start them with staggered delays.
pub fn stagger_anims(
    objs: &[Obj],
    stagger: &Stagger,
    f: impl Fn(Obj) -> Animation,
) -> Vec<Animation> {
    let delays = stagger.delays(objs.len());
    objs.iter()
        .zip(delays)
        .map(|(o, d)| {
            let a = f(*o);
            let base = a.delay;
            a.delay(base + d)
        })
        .collect()
}

// --------------------------------------------------------------- timeline

/// Placement of a timeline entry (GSAP position parameter).
#[derive(Clone, Debug, PartialEq)]
pub enum Pos {
    /// After everything so far (`">"`, default).
    End,
    /// With the start of the previous entry (`"<"`).
    WithPrev,
    /// After the previous entry ends (`">"` relative to previous).
    AfterPrev,
    /// At an absolute time.
    At(f64),
    /// Relative to the end of the timeline (`"+=0.5"`, `"-=0.2"`).
    Offset(f64),
    /// At a label (plus offset).
    Label(String, f64),
}

/// A GSAP / Motion Canvas style timeline: place animations with position parameters and
/// labels, then play them in one go with [`Timeline::play`].
#[derive(Clone, Default)]
pub struct Timeline {
    items: Vec<(Animation, Pos)>,
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(mut self, a: impl Into<Animation>, pos: Pos) -> Self {
        self.items.push((a.into(), pos));
        self
    }
    /// Append after everything so far.
    pub fn then(self, a: impl Into<Animation>) -> Self {
        self.add(a, Pos::End)
    }
    /// Start together with the previous entry.
    pub fn with(self, a: impl Into<Animation>) -> Self {
        self.add(a, Pos::WithPrev)
    }
    /// Append with an offset from the end (negative overlaps).
    pub fn offset(self, a: impl Into<Animation>, dt: f64) -> Self {
        self.add(a, Pos::Offset(dt))
    }
    pub fn at(self, a: impl Into<Animation>, t: f64) -> Self {
        self.add(a, Pos::At(t))
    }
    /// Mark a label at the current end (or any position).
    pub fn label(mut self, name: &str) -> Self {
        // a zero-length sentinel entry records the label at the current end
        self.items.push((
            Animation::wait(0.0),
            Pos::Label(format!("\u{1}{name}"), 0.0),
        ));
        self
    }
    pub fn at_label(self, a: impl Into<Animation>, name: &str, offset: f64) -> Self {
        self.add(a, Pos::Label(name.to_string(), offset))
    }
    /// Resolve start times given a duration function. Returns `(start, duration)` per item.
    pub fn schedule(&self, dur: &dyn Fn(&Animation) -> f64) -> Vec<(f64, f64)> {
        let mut out = Vec::with_capacity(self.items.len());
        let mut end: f64 = 0.0;
        let mut prev = (0.0, 0.0);
        let mut labels: Vec<(String, f64)> = vec![];
        for (a, pos) in &self.items {
            let d = dur(a);
            let start = match pos {
                Pos::End => end,
                Pos::WithPrev => prev.0,
                Pos::AfterPrev => prev.0 + prev.1,
                Pos::At(t) => *t,
                Pos::Offset(o) => end + o,
                Pos::Label(name, off) => {
                    if let Some(n) = name.strip_prefix('\u{1}') {
                        labels.push((n.to_string(), end));
                        out.push((end, 0.0));
                        continue;
                    }
                    labels
                        .iter()
                        .rev()
                        .find(|l| &l.0 == name)
                        .map(|l| l.1)
                        .unwrap_or(end)
                        + off
                }
            }
            .max(0.0);
            out.push((start, d));
            prev = (start, d);
            end = end.max(start + d);
        }
        out
    }
    /// Total length.
    pub fn duration(&self, dur: &dyn Fn(&Animation) -> f64) -> f64 {
        self.schedule(dur)
            .iter()
            .map(|(s, d)| s + d)
            .fold(0.0, f64::max)
    }
    /// Flatten into delayed animations (each with an explicit run time).
    pub fn build(&self, dur: &dyn Fn(&Animation) -> f64) -> Vec<Animation> {
        let sched = self.schedule(dur);
        self.items
            .iter()
            .zip(sched)
            .filter(|((_, p), _)| !matches!(p, Pos::Label(n, _) if n.starts_with('\u{1}')))
            .map(|((a, _), (s, d))| a.clone().run_time(d).delay(s))
            .collect()
    }
    /// Play the whole timeline on a scene; returns its duration.
    pub fn play(&self, scene: &mut Scene) -> f64 {
        let anims = {
            let s: &Scene = scene;
            self.build(&|a| s.anim_duration(a))
        };
        if anims.is_empty() {
            return 0.0;
        }
        scene.play(anims)
    }
}

// --------------------------------------------------- keyframed transforms

/// Keyframe tracks applied to an object relative to its state when the animation starts
/// (Lottie / AE transform properties). Rotation and scale are about the object's centre.
#[derive(Clone, Default)]
pub struct KeyTransform {
    pub offset: Option<Keyframes<Vec2>>,
    pub scale: Option<Keyframes<f64>>,
    /// Non-uniform scale (x, y) multipliers.
    pub stretch: Option<Keyframes<Vec2>>,
    pub rotation: Option<Keyframes<f64>>,
    pub opacity: Option<Keyframes<f64>>,
    pub color: Option<Keyframes<Color>>,
}

impl KeyTransform {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn offset(mut self, k: Keyframes<Vec2>) -> Self {
        self.offset = Some(k);
        self
    }
    pub fn scale(mut self, k: Keyframes<f64>) -> Self {
        self.scale = Some(k);
        self
    }
    pub fn stretch(mut self, k: Keyframes<Vec2>) -> Self {
        self.stretch = Some(k);
        self
    }
    pub fn rotation(mut self, k: Keyframes<f64>) -> Self {
        self.rotation = Some(k);
        self
    }
    pub fn opacity(mut self, k: Keyframes<f64>) -> Self {
        self.opacity = Some(k);
        self
    }
    pub fn color(mut self, k: Keyframes<Color>) -> Self {
        self.color = Some(k);
        self
    }
    pub fn duration(&self) -> f64 {
        let mut d: f64 = 0.0;
        if let Some(k) = &self.offset {
            d = d.max(k.duration());
        }
        if let Some(k) = &self.scale {
            d = d.max(k.duration());
        }
        if let Some(k) = &self.stretch {
            d = d.max(k.duration());
        }
        if let Some(k) = &self.rotation {
            d = d.max(k.duration());
        }
        if let Some(k) = &self.opacity {
            d = d.max(k.duration());
        }
        if let Some(k) = &self.color {
            d = d.max(k.duration());
        }
        d
    }
    /// Apply the tracks at local time `t` to `m`.
    pub fn apply(&self, m: &Mobject, t: f64) -> Mobject {
        let mut m = m.clone();
        let c = m.center();
        if let Some(k) = &self.stretch {
            let s = k.at(t);
            m = m.stretch(s.x, s.y).move_to(c);
        }
        if let Some(k) = &self.scale {
            m = m.scale_about(k.at(t), c);
        }
        if let Some(k) = &self.rotation {
            m = m.rotate_about(k.at(t), c);
        }
        if let Some(k) = &self.offset {
            m = m.shift(k.at(t));
        }
        if let Some(k) = &self.color {
            m = m.color(k.at(t));
        }
        if let Some(k) = &self.opacity {
            m = crate::scene::multiply_opacity(m, k.at(t).clamp(0.0, 1.0) as f32);
        }
        m
    }
}

/// Animate an object through keyframe tracks. Runs for the tracks' duration.
pub fn keyframe_anim(o: Obj, kt: KeyTransform) -> Animation {
    let d = kt.duration().max(1e-3);
    let kt = Arc::new(kt);
    custom(o, move |m, a| kt.apply(m, a * d))
        .rate(Rate::Linear)
        .run_time(d)
}

/// Drive a tracker through a keyframe track (absolute values).
pub fn tracker_keys(k: Tracker, track: Keyframes<f64>) -> Animation {
    let d = track.duration().max(1e-3);
    Animation::new(AnimKind::TrackerWith {
        tracker: k,
        f: Arc::new(move |_start, a| track.at(a * d)),
    })
    .rate(Rate::Linear)
    .run_time(d)
}

/// Spring an object's position by `shift` with a physical spring (overshoot and settle),
/// lasting the spring's settle time.
pub fn spring_shift(o: Obj, shift: Vec2, spring: Spring) -> Animation {
    let d = spring.settle_time(1e-3).max(0.1);
    custom(o, move |m, a| {
        m.clone().shift(shift * spring.progress(a * d))
    })
    .rate(Rate::Linear)
    .run_time(d)
}

/// Spring-scale an object in from zero (pop-in with overshoot).
pub fn spring_pop_in(o: Obj, spring: Spring) -> Animation {
    let d = spring.settle_time(1e-3).max(0.1);
    let mut a = custom(o, move |m, a| {
        let s = spring.progress(a * d).max(0.0);
        let c = m.center();
        m.clone().scale_about(s.max(1e-4), c)
    })
    .rate(Rate::Linear)
    .run_time(d);
    if let AnimKind::Custom { introduces, .. } = &mut a.kind {
        *introduces = true;
    }
    a
}

/// Continuous wiggle of an object's position (AE `wiggle_value(freq, amp)`) for `duration` s.
pub fn wiggle_anim(o: Obj, freq: f64, amp: f64, duration: f64, seed: u64) -> Animation {
    custom(o, move |m, a| {
        m.clone().shift(wiggle_vec(a * duration, freq, amp, seed))
    })
    .rate(Rate::Linear)
    .run_time(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyframes_interpolate_and_loop() {
        let k = Keyframes::new(0.0_f64)
            .key(1.0, 10.0, Rate::Linear)
            .key(2.0, 0.0, Rate::Linear);
        assert!((k.at(0.5) - 5.0).abs() < 1e-9);
        assert!((k.at(1.5) - 5.0).abs() < 1e-9);
        assert_eq!(k.at(5.0), 0.0);
        let r = k.clone().looped(KeyLoop::Repeat);
        assert!((r.at(2.5) - 5.0).abs() < 1e-9);
        let p = Keyframes::new(0.0_f64)
            .key(1.0, 1.0, Rate::Linear)
            .looped(KeyLoop::PingPong);
        assert!((p.at(1.25) - 0.75).abs() < 1e-9);
        let h = Keyframes::new(0.0_f64).hold_key(1.0, 5.0);
        assert_eq!(h.at(0.99), 0.0);
        assert_eq!(h.at(1.0), 5.0);
    }

    #[test]
    fn smooth_keys_pass_through_keys() {
        let k = Keyframes::new(v2(0.0, 0.0))
            .smooth_key(1.0, v2(1.0, 1.0))
            .smooth_key(2.0, v2(2.0, 0.0));
        assert!((k.at(1.0) - v2(1.0, 1.0)).length() < 1e-9);
        // smooth spline is above the straight chord near the middle key
        assert!(k.at(0.9).y > 0.9);
    }

    #[test]
    fn spring_regimes_settle() {
        for s in [
            Spring::bouncy(),
            Spring::snappy(),
            Spring::new(1.0, 100.0, 20.0),
            Spring::new(1.0, 100.0, 60.0),
        ] {
            assert!((s.progress(10.0) - 1.0).abs() < 1e-3, "{s:?}");
            assert!(s.progress(0.0).abs() < 1e-12);
        }
        // underdamped overshoots, overdamped doesn't
        let max = |s: Spring| {
            (0..400)
                .map(|i| s.progress(i as f64 / 100.0))
                .fold(0.0, f64::max)
        };
        assert!(max(Spring::bouncy()) > 1.05);
        assert!(max(Spring::new(1.0, 100.0, 60.0)) <= 1.0 + 1e-9);
        // velocity is consistent with position
        let s = Spring::bouncy();
        let (x0, _) = s.state(0.3, 1.0, 0.0);
        let (x1, v) = s.state(0.3 + 1e-5, 1.0, 0.0);
        assert!(((x1 - x0) / 1e-5 - v).abs() < 1e-2);
    }

    #[test]
    fn spring_follow_tracks_target() {
        let target = |t: f64| if t < 1.0 { 0.0 } else { 5.0 };
        let v = spring_follow(&target, Spring::snappy(), 0.0, 4.0);
        assert!((v - 5.0).abs() < 1e-2);
        assert!(spring_follow(&target, Spring::snappy(), 0.0, 1.05) < 2.5);
    }

    #[test]
    fn stagger_patterns() {
        let s = Stagger::each(0.1);
        assert_eq!(s.delays(3), vec![0.0, 0.1, 0.2]);
        let c = Stagger::each(0.1).from(StaggerFrom::Center).delays(5);
        assert!((c[2]).abs() < 1e-12 && (c[0] - 0.2).abs() < 1e-12 && (c[4] - 0.2).abs() < 1e-12);
        let e = Stagger::each(0.1).from(StaggerFrom::Edges).delays(5);
        assert!(e[0].abs() < 1e-12 && (e[2] - 0.2).abs() < 1e-12);
        let mut r = Stagger::each(0.1).from(StaggerFrom::Random(3)).delays(6);
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((r[5] - 0.5).abs() < 1e-9);
        let g = Stagger::each(0.1)
            .grid(3, 3)
            .from(StaggerFrom::Center)
            .delays(9);
        assert!(g[4].abs() < 1e-12 && g[0] > g[1]);
    }

    #[test]
    fn timeline_positions() {
        let a = || Animation::wait(1.0);
        let tl = Timeline::new()
            .then(a())
            .then(a())
            .with(a())
            .offset(a(), -0.5)
            .label("x")
            .at_label(a(), "x", 1.0)
            .at(a(), 0.25);
        let sched = tl.schedule(&|a| a.run_time.unwrap_or(1.0));
        let starts: Vec<f64> = sched.iter().map(|s| s.0).collect();
        assert_eq!(starts, vec![0.0, 1.0, 1.0, 1.5, 2.5, 3.5, 0.25]);
        assert!((tl.duration(&|_| 1.0) - 4.5).abs() < 1e-9);
    }

    #[test]
    fn wiggle_is_smooth_and_bounded() {
        let mut prev = wiggle_value(0.0, 3.0, 1.0, 1);
        for i in 1..1000 {
            let w = wiggle_value(i as f64 / 100.0, 3.0, 1.0, 1);
            assert!(w.abs() <= 1.0 + 1e-9);
            assert!((w - prev).abs() < 0.2);
            prev = w;
        }
        assert!(inertial_bounce(0.0, 1.0, 2.0, 4.0) == 0.0);
    }
}
