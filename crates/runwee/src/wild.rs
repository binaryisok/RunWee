//! Wild extras (phase A5): baked deterministic simulations (Verlet cloth and
//! ropes, Reynolds boids) and a tiny step-sequencer drum machine that both
//! synthesises a beat and returns exact hit times, so visuals can be perfectly
//! audio-reactive without any analysis lag.
use std::sync::Arc;

use crate::audio::{AudioData, SAMPLE_RATE};
use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::noise::{curl2, hash2, hash_unit};
use crate::shapes::{line, polygon};

// ─────────────────────────────────────────────────────────── baked frames

/// Positions of every particle at a fixed rate; sampled with linear interpolation.
#[derive(Clone, Debug, Default)]
pub struct Baked {
    pub fps: f64,
    pub frames: Vec<Vec<Vec2>>,
}

impl Baked {
    /// Positions at time `t` (seconds since the start of the bake).
    pub fn at(&self, t: f64) -> Vec<Vec2> {
        if self.frames.is_empty() {
            return vec![];
        }
        let f = (t.max(0.0) * self.fps).min((self.frames.len() - 1) as f64);
        let i = f.floor() as usize;
        let j = (i + 1).min(self.frames.len() - 1);
        let u = f - i as f64;
        self.frames[i]
            .iter()
            .zip(&self.frames[j])
            .map(|(a, b)| a.lerp(*b, u))
            .collect()
    }
    /// Approximate velocities at `t`.
    pub fn velocity(&self, t: f64) -> Vec<Vec2> {
        let dt = 1.0 / self.fps;
        let a = self.at(t - dt);
        let b = self.at(t + dt);
        a.iter()
            .zip(&b)
            .map(|(p, q)| (*q - *p) * (0.5 / dt))
            .collect()
    }
}

// ─────────────────────────────────────────────────────────── Verlet

/// A position-based Verlet system with distance constraints and pins.
#[derive(Clone, Default)]
pub struct Verlet {
    pub pos: Vec<Vec2>,
    pub prev: Vec<Vec2>,
    pub pinned: Vec<bool>,
    /// (a, b, rest length, stiffness 0..1)
    pub links: Vec<(usize, usize, f64, f64)>,
    pub gravity: Vec2,
    pub damping: f64,
    pub iterations: usize,
    /// Optional circular colliders (centre, radius) as functions of time.
    pub colliders: Vec<Arc<dyn Fn(f64) -> (Vec2, f64) + Send + Sync>>,
    /// Wind / force field `(p, t) -> acceleration`.
    pub wind: Option<Arc<dyn Fn(Vec2, f64) -> Vec2 + Send + Sync>>,
    /// Links that tear when stretched beyond this factor (0 = never).
    pub tear: f64,
}

impl Verlet {
    fn base() -> Self {
        Self {
            gravity: v2(0.0, -9.0),
            damping: 0.995,
            iterations: 12,
            ..Default::default()
        }
    }
    /// A rope of `n` segments from `a` to `b`, pinned at `a`.
    pub fn rope(a: Vec2, b: Vec2, n: usize) -> Self {
        let mut v = Self::base();
        let n = n.max(1);
        for i in 0..=n {
            let p = a.lerp(b, i as f64 / n as f64);
            v.pos.push(p);
            v.prev.push(p);
            v.pinned.push(i == 0);
        }
        let rest = (b - a).length() / n as f64;
        for i in 0..n {
            v.links.push((i, i + 1, rest, 1.0));
        }
        v
    }
    /// A `cols × rows` cloth hanging from its top row with pins every `pin_every` columns.
    pub fn cloth(top_left: Vec2, cols: usize, rows: usize, spacing: f64, pin_every: usize) -> Self {
        let mut v = Self::base();
        for j in 0..rows {
            for i in 0..cols {
                let p = top_left + v2(i as f64 * spacing, -(j as f64) * spacing);
                v.pos.push(p);
                v.prev.push(p);
                v.pinned
                    .push(j == 0 && (i % pin_every.max(1) == 0 || i == cols - 1));
            }
        }
        let id = |i: usize, j: usize| j * cols + i;
        for j in 0..rows {
            for i in 0..cols {
                if i + 1 < cols {
                    v.links.push((id(i, j), id(i + 1, j), spacing, 1.0));
                }
                if j + 1 < rows {
                    v.links.push((id(i, j), id(i, j + 1), spacing, 1.0));
                }
            }
        }
        v
    }
    pub fn with_wind(mut self, f: impl Fn(Vec2, f64) -> Vec2 + Send + Sync + 'static) -> Self {
        self.wind = Some(Arc::new(f));
        self
    }
    pub fn with_collider(mut self, f: impl Fn(f64) -> (Vec2, f64) + Send + Sync + 'static) -> Self {
        self.colliders.push(Arc::new(f));
        self
    }
    /// One substep of length `dt` at time `t`.
    pub fn step(&mut self, dt: f64, t: f64) {
        for i in 0..self.pos.len() {
            if self.pinned[i] {
                continue;
            }
            let p = self.pos[i];
            let mut acc = self.gravity;
            if let Some(w) = &self.wind {
                acc += w(p, t);
            }
            let next = p + (p - self.prev[i]) * self.damping + acc * (dt * dt);
            self.prev[i] = p;
            self.pos[i] = next;
        }
        for _ in 0..self.iterations {
            let mut k = 0;
            while k < self.links.len() {
                let (a, b, rest, stiff) = self.links[k];
                let d = self.pos[b] - self.pos[a];
                let len = d.length().max(1e-9);
                if self.tear > 0.0 && len > rest * self.tear {
                    self.links.swap_remove(k);
                    continue;
                }
                let corr = d * ((len - rest) / len * 0.5 * stiff);
                match (self.pinned[a], self.pinned[b]) {
                    (false, false) => {
                        self.pos[a] += corr;
                        self.pos[b] -= corr;
                    }
                    (true, false) => self.pos[b] -= corr * 2.0,
                    (false, true) => self.pos[a] += corr * 2.0,
                    _ => {}
                }
                k += 1;
            }
            for c in &self.colliders {
                let (cc, r) = c(t);
                for i in 0..self.pos.len() {
                    let d = self.pos[i] - cc;
                    let l = d.length();
                    if l < r && !self.pinned[i] {
                        self.pos[i] = cc + d * (r / l.max(1e-9));
                    }
                }
            }
        }
    }
    /// Simulate for `duration` seconds with `substeps` per output frame.
    pub fn bake(mut self, duration: f64, fps: f64, substeps: usize) -> (Baked, Self) {
        let n = (duration * fps).ceil() as usize + 1;
        let dt = 1.0 / fps / substeps.max(1) as f64;
        let mut frames = Vec::with_capacity(n);
        let mut t = 0.0;
        for _ in 0..n {
            frames.push(self.pos.clone());
            for _ in 0..substeps.max(1) {
                self.step(dt, t);
                t += dt;
            }
        }
        (Baked { fps, frames }, self)
    }
}

/// Draw a cloth (`cols × rows` grid of positions) as shaded quads: faces lit by
/// how stretched / tilted they are, alternating stripes from `colors`.
pub fn cloth_mobject(
    pos: &[Vec2],
    cols: usize,
    rows: usize,
    colors: &[Color],
    spacing: f64,
) -> Mobject {
    let mut out = Mobject::empty();
    if pos.len() < cols * rows || colors.is_empty() {
        return out;
    }
    let id = |i: usize, j: usize| j * cols + i;
    for j in 0..rows - 1 {
        for i in 0..cols - 1 {
            let (a, b, c, d) = (
                pos[id(i, j)],
                pos[id(i + 1, j)],
                pos[id(i + 1, j + 1)],
                pos[id(i, j + 1)],
            );
            let area = ((b - a).cross(d - a)).abs() / (spacing * spacing);
            let shade = (0.45 + 0.55 * area.clamp(0.0, 1.2) / 1.2) as f32;
            let col = colors[(i / 3) % colors.len()];
            let lit = col.mix(BLACK, 1.0 - shade);
            out.children
                .push(polygon(&[a, b, c, d]).fill(lit, 1.0).stroke(lit, 0.6));
        }
    }
    out
}

/// Draw a rope polyline with beads.
pub fn rope_mobject(pos: &[Vec2], color: Color, width: f32) -> Mobject {
    let mut out = Mobject::empty();
    for w in pos.windows(2) {
        out.children.push(line(w[0], w[1]).stroke(color, width));
    }
    out
}

// ─────────────────────────────────────────────────────────── boids

/// Reynolds flocking parameters.
#[derive(Clone, Copy, Debug)]
pub struct BoidOpts {
    pub count: usize,
    pub bounds: (Vec2, Vec2),
    pub max_speed: f64,
    pub view: f64,
    pub separation: f64,
    pub alignment: f64,
    pub cohesion: f64,
    /// Curl-noise wander strength.
    pub wander: f64,
    pub seed: u64,
}

impl Default for BoidOpts {
    fn default() -> Self {
        Self {
            count: 120,
            bounds: (v2(-7.0, -4.0), v2(7.0, 4.0)),
            max_speed: 2.8,
            view: 0.9,
            separation: 2.2,
            alignment: 1.0,
            cohesion: 0.7,
            wander: 0.6,
            seed: 1,
        }
    }
}

/// Bake a flock: positions over time (soft walls keep boids inside `bounds`).
/// An optional `attractor(t)` pulls the flock (e.g. a moving target).
pub fn bake_boids(
    o: BoidOpts,
    duration: f64,
    fps: f64,
    attractor: Option<&dyn Fn(f64) -> Vec2>,
) -> Baked {
    let (lo, hi) = o.bounds;
    let r = |i: usize, k: i64| hash_unit(hash2(i as i64, k, o.seed));
    let mut p: Vec<Vec2> = (0..o.count)
        .map(|i| v2(lerp(lo.x, hi.x, r(i, 0)), lerp(lo.y, hi.y, r(i, 1))))
        .collect();
    let mut v: Vec<Vec2> = (0..o.count)
        .map(|i| Vec2::from_angle(r(i, 2) * TAU) * o.max_speed * 0.5)
        .collect();
    let sub = 2;
    let dt = 1.0 / fps / sub as f64;
    let n = (duration * fps).ceil() as usize + 1;
    let mut frames = Vec::with_capacity(n);
    let mut t = 0.0;
    for _ in 0..n {
        frames.push(p.clone());
        for _ in 0..sub {
            let mut acc = vec![Vec2::ZERO; o.count];
            for i in 0..o.count {
                let (mut sep, mut ali, mut coh, mut cnt) =
                    (Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, 0.0);
                for j in 0..o.count {
                    if i == j {
                        continue;
                    }
                    let d = p[j] - p[i];
                    let l = d.length();
                    if l < o.view {
                        cnt += 1.0;
                        ali += v[j];
                        coh += p[j];
                        if l < o.view * 0.45 {
                            sep -= d * (1.0 / (l * l).max(1e-3));
                        }
                    }
                }
                let mut a = sep * (o.separation * 0.1);
                if cnt > 0.0 {
                    a += (ali * (1.0 / cnt) - v[i]) * o.alignment;
                    a += (coh * (1.0 / cnt) - p[i]) * o.cohesion;
                }
                a += curl2(p[i] * 0.3, t * 0.2, o.seed) * o.wander;
                if let Some(f) = attractor {
                    a += (f(t) - p[i]) * 0.35;
                }
                // soft walls
                let m = 1.0;
                if p[i].x < lo.x + m {
                    a.x += (lo.x + m - p[i].x) * 6.0;
                }
                if p[i].x > hi.x - m {
                    a.x -= (p[i].x - hi.x + m) * 6.0;
                }
                if p[i].y < lo.y + m {
                    a.y += (lo.y + m - p[i].y) * 6.0;
                }
                if p[i].y > hi.y - m {
                    a.y -= (p[i].y - hi.y + m) * 6.0;
                }
                acc[i] = a;
            }
            for i in 0..o.count {
                v[i] += acc[i] * dt;
                let s = v[i].length();
                if s > o.max_speed {
                    v[i] *= o.max_speed / s;
                } else if s < o.max_speed * 0.3 {
                    v[i] *= o.max_speed * 0.3 / s.max(1e-6);
                }
                p[i] += v[i] * dt;
            }
            t += dt;
        }
    }
    Baked { fps, frames }
}

/// Draw boids as little arrowheads oriented along their velocity.
pub fn boids_mobject(pos: &[Vec2], vel: &[Vec2], size: f64, colors: &[Color]) -> Mobject {
    let mut out = Mobject::empty();
    for (i, (p, v)) in pos.iter().zip(vel).enumerate() {
        let f = if v.length() > 1e-9 {
            v.normalize()
        } else {
            v2(1.0, 0.0)
        };
        let s = v2(-f.y, f.x);
        let col = colors[i % colors.len().max(1)];
        out.children.push(
            polygon(&[
                *p + f * size,
                *p - f * (size * 0.6) + s * (size * 0.5),
                *p - f * (size * 0.3),
                *p - f * (size * 0.6) - s * (size * 0.5),
            ])
            .fill(col, 1.0)
            .no_stroke(),
        );
    }
    out
}

// ─────────────────────────────────────────────────────────── drum machine

/// Voices of the [`DrumMachine`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Voice {
    Kick,
    Snare,
    Hat,
    Clap,
    /// Bass note; the step pattern char picks a scale degree (`1`–`9`).
    Bass,
}

/// A step sequencer: patterns are strings of 16 steps per bar, `x`/`X` = hit
/// (`X` accented), `.` = rest, digits for bass scale degrees.
#[derive(Clone, Debug)]
pub struct DrumMachine {
    pub bpm: f64,
    pub bars: usize,
    pub tracks: Vec<(Voice, String)>,
    pub root_midi: f32,
    pub swing: f64,
}

/// One scheduled hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub t: f64,
    pub voice: Voice,
    pub velocity: f32,
    pub note: f32,
}

impl DrumMachine {
    pub fn new(bpm: f64, bars: usize) -> Self {
        Self {
            bpm,
            bars,
            tracks: vec![],
            root_midi: 36.0,
            swing: 0.0,
        }
    }
    pub fn track(mut self, v: Voice, pattern: &str) -> Self {
        self.tracks.push((
            v,
            pattern
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '|')
                .collect(),
        ));
        self
    }
    pub fn step_len(&self) -> f64 {
        60.0 / self.bpm / 4.0
    }
    pub fn duration(&self) -> f64 {
        self.step_len() * 16.0 * self.bars as f64
    }
    /// All hits in time order.
    pub fn hits(&self) -> Vec<Hit> {
        let minor = [0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0, 12.0, 14.0];
        let mut out = vec![];
        let total = 16 * self.bars;
        for (v, pat) in &self.tracks {
            let chars: Vec<char> = pat.chars().collect();
            if chars.is_empty() {
                continue;
            }
            for s in 0..total {
                let c = chars[s % chars.len()];
                let sw = if s % 2 == 1 {
                    self.swing * self.step_len()
                } else {
                    0.0
                };
                let t = s as f64 * self.step_len() + sw;
                let (vel, note) = match c {
                    'x' => (0.75, 0.0),
                    'X' => (1.0, 0.0),
                    d if d.is_ascii_digit() && d != '0' => (0.9, minor[(d as u8 - b'1') as usize]),
                    _ => continue,
                };
                out.push(Hit {
                    t,
                    voice: *v,
                    velocity: vel,
                    note: self.root_midi + note,
                });
            }
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }
    /// Synthesise the pattern (stereo, [`SAMPLE_RATE`]).
    pub fn render(&self) -> Arc<AudioData> {
        let sr = SAMPLE_RATE as f64;
        let n = ((self.duration() + 1.0) * sr) as usize;
        let mut buf = vec![0f32; n];
        for h in self.hits() {
            let start = (h.t * sr) as usize;
            let len = match h.voice {
                Voice::Kick => 0.45,
                Voice::Snare | Voice::Clap => 0.3,
                Voice::Hat => 0.06,
                Voice::Bass => self.step_len() * 1.8,
            };
            let m = (len * sr) as usize;
            let mut lp = 0.0f32;
            for i in 0..m {
                if start + i >= n {
                    break;
                }
                let t = i as f64 / sr;
                let noise = (hash_unit(hash2(start as i64 + i as i64, 7, 99)) * 2.0 - 1.0) as f32;
                let s = match h.voice {
                    Voice::Kick => {
                        let f = 45.0 + 110.0 * (-t * 28.0).exp();
                        let ph = TAU * (45.0 * t + 110.0 / 28.0 * (1.0 - (-t * 28.0).exp()));
                        let _ = f;
                        (ph.sin() * (-t * 7.0).exp()) as f32 * 0.9
                    }
                    Voice::Snare => {
                        let tone = (TAU * 190.0 * t).sin() * (-t * 25.0).exp();
                        (tone * 0.35) as f32 + noise * ((-t * 16.0).exp() as f32) * 0.45
                    }
                    Voice::Clap => {
                        let env =
                            ((-t * 30.0).exp() + 0.6 * (-(t - 0.012).abs() * 120.0).exp()) as f32;
                        noise * env * 0.45
                    }
                    Voice::Hat => {
                        // crude high-pass: noise minus its low-passed self
                        lp += 0.35 * (noise - lp);
                        (noise - lp) * ((-t * 60.0).exp() as f32) * 0.35
                    }
                    Voice::Bass => {
                        let f = 440.0 * 2f64.powf((h.note as f64 - 69.0) / 12.0);
                        let saw = 2.0 * (f * t).fract() - 1.0;
                        let x = saw as f32;
                        lp += 0.08 * (x - lp);
                        let env = ((t / 0.005).min(1.0) * (-t * 3.0).exp()) as f32;
                        lp * env * 0.6
                    }
                };
                buf[start + i] += s * h.velocity;
            }
        }
        let peak = buf.iter().fold(0f32, |m, v| m.max(v.abs())).max(1e-6);
        for v in &mut buf {
            *v = (*v / peak * 0.9).tanh();
        }
        Arc::new(AudioData::from_mono(&buf, SAMPLE_RATE))
    }
}

/// Envelope of the most recent hit of `voice` at time `t`: 1 at the hit, decaying
/// with time constant `decay` seconds. Hits must be sorted by time.
pub fn hit_envelope(hits: &[Hit], voice: Voice, t: f64, decay: f64) -> f64 {
    let idx = hits.partition_point(|h| h.t <= t);
    hits[..idx]
        .iter()
        .rev()
        .find(|h| h.voice == voice)
        .map(|h| h.velocity as f64 * (-(t - h.t) / decay.max(1e-6)).exp())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rope_keeps_length_and_hangs() {
        let (b, end) = Verlet::rope(v2(0.0, 0.0), v2(3.0, 0.0), 20).bake(3.0, 30.0, 4);
        let last = b.frames.last().unwrap();
        assert!(last[0].distance(v2(0.0, 0.0)) < 1e-12, "pinned point stays");
        assert!(last[20].y < -1.0, "rope swings down");
        let len: f64 = last.windows(2).map(|w| w[0].distance(w[1])).sum();
        assert!((len - 3.0).abs() < 0.15, "length {len}");
        assert_eq!(end.links.len(), 20);
        // interpolation
        let mid = b.at(0.5 / 30.0);
        assert!(mid[10].distance(b.frames[0][10].lerp(b.frames[1][10], 0.5)) < 1e-9);
    }

    #[test]
    fn boids_are_deterministic_and_bounded() {
        let o = BoidOpts {
            count: 30,
            ..Default::default()
        };
        let a = bake_boids(o, 2.0, 15.0, None);
        let b = bake_boids(o, 2.0, 15.0, None);
        assert_eq!(a.frames.last(), b.frames.last());
        assert!(a
            .frames
            .last()
            .unwrap()
            .iter()
            .all(|p| p.x.abs() < 8.5 && p.y.abs() < 5.5));
    }

    #[test]
    fn drum_machine_hits_and_audio() {
        let dm = DrumMachine::new(120.0, 1)
            .track(Voice::Kick, "x...x...x...x...")
            .track(Voice::Bass, "1.......5.......");
        let hits = dm.hits();
        assert_eq!(hits.iter().filter(|h| h.voice == Voice::Kick).count(), 4);
        assert!((dm.duration() - 2.0).abs() < 1e-12);
        assert!(
            (hits
                .iter()
                .find(|h| h.voice == Voice::Kick && h.t > 0.1)
                .unwrap()
                .t
                - 0.5)
                .abs()
                < 1e-12
        );
        let audio = dm.render();
        assert!(audio.duration() > 2.0);
        assert!(hit_envelope(&hits, Voice::Kick, 0.5, 0.1) > 0.7);
        assert!(hit_envelope(&hits, Voice::Kick, 0.95, 0.1) < 0.02);
    }
}
