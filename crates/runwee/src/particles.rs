//! Deterministic particle systems (phase A3).
//!
//! Particles are *not* simulated frame by frame: every particle's birth time,
//! initial velocity, size and colour come from a seeded hash of its index, and its
//! position is the closed-form solution of motion under gravity + linear drag, plus
//! a curl-noise turbulence offset. So any frame can be rendered independently and
//! in parallel, scrubbing works, and renders are reproducible.
use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::noise::{curl2, hash2, hash_unit};
use crate::scene::{Obj, Scene};
use crate::shapes::*;
use crate::style::Blend;

/// How a particle is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleShape {
    Dot,
    /// Soft glowing dot.
    Glow,
    Square,
    /// Line segment along the velocity (sparks, rain).
    Streak,
    /// Tumbling paper rectangle.
    Confetti,
    /// Six-pointed flake.
    Flake,
}

/// Where particles are born.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmitFrom {
    Point(Vec2),
    /// Uniformly along a segment.
    Line(Vec2, Vec2),
    /// Uniformly inside a disc.
    Disc(Vec2, f64),
    /// Uniformly inside a rectangle.
    Box(Vec2, Vec2),
}

/// A particle emitter. All times are scene seconds.
#[derive(Clone, Debug)]
pub struct Emitter {
    pub from: EmitFrom,
    /// Particles per second (ignored for bursts).
    pub rate: f64,
    pub start: f64,
    pub stop: f64,
    /// If set, emit `burst` particles at `start` instead of a stream.
    pub burst: Option<usize>,
    pub life: (f64, f64),
    pub speed: (f64, f64),
    /// Mean emission direction (radians) and full spread.
    pub angle: f64,
    pub spread: f64,
    pub gravity: Vec2,
    /// Linear drag coefficient (1/s).
    pub drag: f64,
    pub turbulence: f64,
    pub size: (f64, f64),
    /// Size multiplier at end of life.
    pub size_end: f64,
    /// Colour ramp over normalised age.
    pub colors: Vec<Color>,
    /// Pick a random ramp colour per particle instead of ageing through it.
    pub random_color: bool,
    pub shape: ParticleShape,
    pub blend: Blend,
    pub seed: u64,
}

/// State of one live particle.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    /// Normalised age in `[0,1)`.
    pub age: f64,
    pub size: f64,
    pub color: Color,
    pub rot: f64,
    pub id: u64,
}

impl Default for Emitter {
    fn default() -> Self {
        Self {
            from: EmitFrom::Point(Vec2::ZERO),
            rate: 60.0,
            start: 0.0,
            stop: f64::INFINITY,
            burst: None,
            life: (1.0, 2.0),
            speed: (1.0, 2.0),
            angle: std::f64::consts::FRAC_PI_2,
            spread: 0.6,
            gravity: v2(0.0, -2.0),
            drag: 0.5,
            turbulence: 0.0,
            size: (0.03, 0.06),
            size_end: 0.3,
            colors: vec![WHITE],
            random_color: false,
            shape: ParticleShape::Dot,
            blend: Blend::Normal,
            seed: 1,
        }
    }
}

impl Emitter {
    pub fn at(mut self, p: Vec2) -> Self {
        self.from = EmitFrom::Point(p);
        self
    }
    pub fn from(mut self, f: EmitFrom) -> Self {
        self.from = f;
        self
    }
    pub fn between(mut self, start: f64, stop: f64) -> Self {
        self.start = start;
        self.stop = stop;
        self
    }
    /// Start emitting at `t` (keeps `stop`).
    pub fn starting(mut self, t: f64) -> Self {
        self.start = t;
        self
    }
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }
    pub fn rate(mut self, r: f64) -> Self {
        self.rate = r;
        self
    }
    pub fn colors(mut self, c: &[Color]) -> Self {
        self.colors = c.to_vec();
        self
    }

    // ---- presets
    /// Rising additive flames.
    pub fn fire(p: Vec2) -> Self {
        Self {
            from: EmitFrom::Disc(p, 0.3),
            rate: 200.0,
            life: (0.8, 1.6),
            speed: (0.6, 1.4),
            spread: 0.6,
            gravity: v2(0.0, 2.6),
            drag: 0.9,
            turbulence: 0.7,
            size: (0.12, 0.22),
            size_end: 0.25,
            colors: vec![
                rgb8(255, 230, 160).with_alpha(0.55),
                rgb8(255, 160, 40).with_alpha(0.5),
                rgb8(220, 60, 20).with_alpha(0.35),
                rgb8(120, 20, 40).with_alpha(0.0),
            ],
            shape: ParticleShape::Glow,
            blend: Blend::Add,
            ..Default::default()
        }
    }
    /// Fast streaking sparks that fall under gravity.
    pub fn sparks(p: Vec2) -> Self {
        Self {
            from: EmitFrom::Point(p),
            rate: 90.0,
            life: (0.5, 1.2),
            speed: (3.0, 6.0),
            spread: 1.4,
            gravity: v2(0.0, -7.0),
            drag: 0.6,
            size: (0.015, 0.03),
            size_end: 0.5,
            colors: vec![WHITE, rgb8(255, 220, 120), rgb8(255, 120, 30)],
            shape: ParticleShape::Streak,
            blend: Blend::Add,
            ..Default::default()
        }
    }
    /// Gently drifting snow over a box.
    pub fn snow(lo: Vec2, hi: Vec2) -> Self {
        Self {
            from: EmitFrom::Line(v2(lo.x, hi.y), hi),
            rate: 40.0,
            life: (5.0, 7.0),
            speed: (0.2, 0.5),
            angle: -std::f64::consts::FRAC_PI_2,
            spread: 0.4,
            gravity: v2(0.0, -0.3),
            drag: 0.8,
            turbulence: 0.4,
            size: (0.03, 0.07),
            size_end: 1.0,
            colors: vec![WHITE, rgb8(200, 225, 255)],
            random_color: true,
            shape: ParticleShape::Flake,
            ..Default::default()
        }
    }
    /// A celebratory confetti cannon.
    pub fn confetti(p: Vec2, angle: f64) -> Self {
        Self {
            from: EmitFrom::Point(p),
            burst: Some(180),
            life: (2.5, 3.5),
            speed: (5.0, 9.0),
            angle,
            spread: 0.7,
            gravity: v2(0.0, -4.0),
            drag: 1.6,
            turbulence: 0.5,
            size: (0.06, 0.1),
            size_end: 1.0,
            colors: vec![
                rgb8(255, 85, 120),
                rgb8(255, 205, 60),
                rgb8(80, 220, 160),
                rgb8(80, 160, 255),
                rgb8(190, 110, 255),
            ],
            random_color: true,
            shape: ParticleShape::Confetti,
            ..Default::default()
        }
    }
    /// A firework shell bursting at `p` at time `t`.
    pub fn firework(p: Vec2, t: f64, color: Color, seed: u64) -> Self {
        Self {
            from: EmitFrom::Point(p),
            start: t,
            burst: Some(140),
            life: (1.2, 1.8),
            speed: (2.0, 3.0),
            spread: TAU,
            gravity: v2(0.0, -1.5),
            drag: 1.4,
            size: (0.025, 0.04),
            size_end: 0.4,
            colors: vec![WHITE, color, color.with_alpha(0.0)],
            shape: ParticleShape::Streak,
            blend: Blend::Add,
            seed,
            ..Default::default()
        }
    }
    /// Slow, expanding smoke puffs.
    pub fn smoke(p: Vec2) -> Self {
        Self {
            from: EmitFrom::Disc(p, 0.2),
            rate: 25.0,
            life: (2.5, 4.0),
            speed: (0.3, 0.6),
            spread: 0.6,
            gravity: v2(0.25, 0.4),
            drag: 0.6,
            turbulence: 0.5,
            size: (0.15, 0.25),
            size_end: 3.0,
            colors: vec![
                rgb8(130, 130, 140).with_alpha(0.35),
                rgb8(60, 60, 70).with_alpha(0.0),
            ],
            shape: ParticleShape::Glow,
            ..Default::default()
        }
    }

    fn count_upto(&self, t: f64) -> (usize, usize) {
        // (first possibly-alive index, one past last born)
        if t < self.start {
            return (0, 0);
        }
        if let Some(n) = self.burst {
            return (0, n);
        }
        let end = t.min(self.stop);
        let born = ((end - self.start) * self.rate).floor() as usize + 1;
        let first = (((t - self.life.1 - self.start) * self.rate)
            .floor()
            .max(0.0)) as usize;
        (first.min(born), born)
    }

    /// Analytic state of every particle alive at time `t`.
    pub fn particles(&self, t: f64) -> Vec<Particle> {
        let (first, last) = self.count_upto(t);
        let mut out = Vec::with_capacity(last - first);
        for i in first..last {
            let id = i as u64;
            let r = |k: i64| hash_unit(hash2(i as i64, k, self.seed));
            let birth = match self.burst {
                Some(_) => self.start + r(0) * 0.05,
                None => self.start + i as f64 / self.rate,
            };
            let life = lerp(self.life.0, self.life.1, r(1));
            let tau = t - birth;
            if tau < 0.0 || tau >= life {
                continue;
            }
            let p0 = match self.from {
                EmitFrom::Point(p) => p,
                EmitFrom::Line(a, b) => a.lerp(b, r(2)),
                EmitFrom::Disc(c, rad) => c + Vec2::polar(rad * r(2).sqrt(), r(3) * TAU),
                EmitFrom::Box(a, b) => v2(lerp(a.x, b.x, r(2)), lerp(a.y, b.y, r(3))),
            };
            let ang = self.angle + (r(4) - 0.5) * self.spread;
            let v0 = Vec2::from_angle(ang) * lerp(self.speed.0, self.speed.1, r(5));
            let k = self.drag.max(1e-6);
            let e = (-k * tau).exp();
            // x(τ) = x0 + v0 (1-e)/k + g/k (τ - (1-e)/k);  v(τ) = v0 e + g (1-e)/k
            let one = (1.0 - e) / k;
            let mut pos = p0 + v0 * one + self.gravity * ((tau - one) / k);
            let vel = v0 * e + self.gravity * one;
            if self.turbulence > 0.0 {
                pos += curl2(p0 * 0.6 + v2(id as f64 * 0.013, 0.0), t * 0.4, self.seed)
                    * (self.turbulence * tau);
            }
            let age = tau / life;
            let color = if self.random_color {
                self.colors[(r(6) * self.colors.len() as f64) as usize % self.colors.len()]
                    .fade(1.0 - smoothstep(0.75, 1.0, age) as f32)
            } else {
                crate::generative::gradient_at(&self.colors, age)
            };
            let size = lerp(self.size.0, self.size.1, r(7)) * lerp(1.0, self.size_end, age);
            out.push(Particle {
                pos,
                vel,
                age,
                size,
                color,
                rot: r(8) * TAU + tau * (r(9) - 0.5) * 12.0,
                id,
            });
        }
        out
    }

    /// Draw the particles alive at `t`.
    pub fn render(&self, t: f64) -> Mobject {
        let mut g = Mobject::empty();
        for p in self.particles(t) {
            let m = match self.shape {
                ParticleShape::Dot => dot_r(p.pos, p.size, p.color),
                ParticleShape::Glow => {
                    let mut d = circle(p.size).move_to(p.pos).no_stroke();
                    d.style.fill = crate::style::Paint::radial(
                        p.pos,
                        p.size,
                        &[p.color, p.color.with_alpha(0.0)],
                    );
                    d.style.fill_opacity = 1.0;
                    d
                }
                ParticleShape::Square => square(p.size * 2.0)
                    .fill(p.color, 1.0)
                    .no_stroke()
                    .rotate(p.rot)
                    .move_to(p.pos),
                ParticleShape::Streak => {
                    let tail = p.pos - p.vel * 0.04;
                    line(tail, p.pos).stroke(p.color, (p.size * 120.0) as f32)
                }
                ParticleShape::Confetti => {
                    // tumbling: squash one axis by |cos| of a spin angle
                    let flip = (p.rot * 1.7).cos().abs().max(0.08);
                    rectangle(p.size * 2.0, p.size * 1.2 * flip)
                        .fill(p.color, 1.0)
                        .no_stroke()
                        .rotate(p.rot)
                        .move_to(p.pos)
                }
                ParticleShape::Flake => {
                    let mut kids = vec![];
                    for k in 0..3 {
                        let d = Vec2::from_angle(p.rot + k as f64 * std::f64::consts::PI / 3.0)
                            * p.size;
                        kids.push(line(p.pos - d, p.pos + d).stroke(p.color, 2.0));
                    }
                    Mobject::group(kids)
                }
            };
            g.children.push(m);
        }
        if self.blend != Blend::Normal {
            g.for_each_mut(&mut |n| n.style.blend = self.blend);
        }
        g
    }
}

/// Add an always-redrawn particle system driven by scene time.
pub fn particle_system(s: &mut Scene, e: Emitter) -> Obj {
    s.always_redraw(move |ctx| e.render(ctx.t))
}

/// Several emitters as one redrawn object (e.g. a firework show).
pub fn particle_systems(s: &mut Scene, es: Vec<Emitter>) -> Obj {
    s.always_redraw(move |ctx| Mobject::group(es.iter().map(|e| e.render(ctx.t))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_bounded_counts() {
        let e = Emitter::default().rate(50.0).between(1.0, 3.0);
        assert!(e.particles(0.5).is_empty());
        let a = e.particles(2.0);
        let b = e.particles(2.0);
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(x, y)| x.pos == y.pos));
        // at most rate * max-life alive
        assert!(a.len() <= 101 && a.len() > 20, "{}", a.len());
        // emission stops; everything dead after stop + max life
        assert!(e.particles(5.1).is_empty());
    }

    #[test]
    fn analytic_motion_matches_integration() {
        let e = Emitter {
            burst: Some(1),
            spread: 0.0,
            speed: (2.0, 2.0),
            life: (10.0, 10.0),
            angle: 0.0,
            drag: 0.7,
            gravity: v2(0.0, -3.0),
            ..Default::default()
        };
        let birth = e.start + hash_unit(hash2(0, 0, e.seed)) * 0.05;
        let p = e.particles(birth + 1.5)[0];
        // explicit Euler with tiny steps
        let (mut x, mut v, dt) = (Vec2::ZERO, v2(2.0, 0.0), 1e-4);
        for _ in 0..15000 {
            v = v + (e.gravity - v * e.drag) * dt;
            x += v * dt;
        }
        assert!(p.pos.distance(x) < 1e-2, "{:?} vs {:?}", p.pos, x);
        assert!(p.vel.distance(v) < 1e-2);
    }

    #[test]
    fn presets_render() {
        for e in [
            Emitter::fire(Vec2::ZERO),
            Emitter::sparks(Vec2::ZERO),
            Emitter::snow(v2(-1.0, -1.0), v2(1.0, 1.0)),
            Emitter::confetti(Vec2::ZERO, 1.2),
            Emitter::firework(Vec2::ZERO, 0.0, RED, 3),
            Emitter::smoke(Vec2::ZERO),
        ] {
            assert!(!e.render(0.8).children.is_empty());
        }
    }
}
