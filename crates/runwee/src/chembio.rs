//! Phase S3 — chemistry & biology models that go beyond the existing diagrams:
//! hydrogen-atom orbitals from the real wavefunction ψ(n, l, m), reaction energy
//! profiles (with a catalysed path), acid–base titration curves solved from the
//! charge balance, indicator colours, an agent-based SIR epidemic beside the SIR ODE,
//! and radioactive decay of individual nuclei.

use crate::color::*;
use crate::generative::field_image;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::path::Rect;
use crate::shapes::*;
use crate::text::text_sized;
use std::f64::consts::PI;

// ───────────────────────────── hydrogen orbitals ─────────────────────────────

fn factorial(n: u32) -> f64 {
    (1..=n).map(|k| k as f64).product()
}

/// Generalised Laguerre polynomial L_k^α(x) (three-term recurrence).
pub fn laguerre(k: u32, alpha: f64, x: f64) -> f64 {
    let (mut l0, mut l1) = (1.0, 1.0 + alpha - x);
    if k == 0 {
        return l0;
    }
    for j in 1..k {
        let j = j as f64;
        let l2 = ((2.0 * j + 1.0 + alpha - x) * l1 - (j + alpha) * l0) / (j + 1.0);
        l0 = l1;
        l1 = l2;
    }
    l1
}

/// Associated Legendre function P_l^m(x) for m ≥ 0 (Condon–Shortley phase).
pub fn assoc_legendre(l: u32, m: u32, x: f64) -> f64 {
    let mut pmm = 1.0;
    if m > 0 {
        let s = ((1.0 - x) * (1.0 + x)).max(0.0).sqrt();
        let mut f = 1.0;
        for _ in 0..m {
            pmm *= -f * s;
            f += 2.0;
        }
    }
    if l == m {
        return pmm;
    }
    let mut pmm1 = x * (2 * m + 1) as f64 * pmm;
    if l == m + 1 {
        return pmm1;
    }
    let mut pll = 0.0;
    for ll in m + 2..=l {
        pll = (x * (2 * ll - 1) as f64 * pmm1 - (ll + m - 1) as f64 * pmm) / (ll - m) as f64;
        pmm = pmm1;
        pmm1 = pll;
    }
    pll
}

/// Normalised hydrogen radial function R_nl(r), r in Bohr radii.
pub fn hydrogen_radial(n: u32, l: u32, r: f64) -> f64 {
    let nf = n as f64;
    let rho = 2.0 * r / nf;
    let norm = ((2.0 / nf).powi(3) * factorial(n - l - 1) / (2.0 * nf * factorial(n + l))).sqrt();
    norm * (-rho / 2.0).exp() * rho.powi(l as i32) * laguerre(n - l - 1, (2 * l + 1) as f64, rho)
}

/// Real spherical harmonic Y_lm(θ, φ) (m < 0 → sin(|m|φ), m > 0 → cos(mφ)).
pub fn real_ylm(l: u32, m: i32, theta: f64, phi: f64) -> f64 {
    let am = m.unsigned_abs();
    let k = ((2 * l + 1) as f64 / (4.0 * PI) * factorial(l - am) / factorial(l + am)).sqrt();
    let p = assoc_legendre(l, am, theta.cos());
    match m {
        0 => k * p,
        m if m > 0 => 2f64.sqrt() * k * p * (m as f64 * phi).cos(),
        _ => 2f64.sqrt() * k * p * (am as f64 * phi).sin(),
    }
}

/// Real hydrogen wavefunction ψ_nlm at Cartesian (x, y, z) in Bohr radii.
pub fn hydrogen_psi(n: u32, l: u32, m: i32, x: f64, y: f64, z: f64) -> f64 {
    let r = (x * x + y * y + z * z).sqrt();
    let theta = if r > 0.0 {
        (z / r).clamp(-1.0, 1.0).acos()
    } else {
        0.0
    };
    let phi = y.atan2(x);
    hydrogen_radial(n, l, r) * real_ylm(l, m, theta, phi)
}

/// Conventional orbital label such as "3d" for (n, l).
pub fn orbital_name(n: u32, l: u32) -> String {
    let letter = ['s', 'p', 'd', 'f', 'g', 'h']
        .get(l as usize)
        .copied()
        .unwrap_or('?');
    format!("{n}{letter}")
}

/// Probability-density slice of ψ_nlm through the nucleus. `extent` is the half-width
/// of the view in Bohr radii; `plane_xy` slices z = 0 instead of y = 0. Brightness is
/// |ψ|² (gamma-compressed), hue shows the sign of ψ (orange +, blue −).
pub fn orbital_slice(
    rect: Rect,
    px_w: u32,
    n: u32,
    l: u32,
    m: i32,
    extent: f64,
    plane_xy: bool,
) -> Mobject {
    let c = rect.center();
    let s = extent / (rect.width() / 2.0);
    let map = move |p: Vec2| {
        let q = (p - c) * s;
        if plane_xy {
            hydrogen_psi(n, l, m, q.x, q.y, 0.0)
        } else {
            hydrogen_psi(n, l, m, q.x, 0.0, q.y)
        }
    };
    let mut peak = 1e-30f64;
    for i in 0..90 {
        for j in 0..90 {
            let p = v2(
                lerp(rect.min.x, rect.max.x, i as f64 / 89.0),
                lerp(rect.min.y, rect.max.y, j as f64 / 89.0),
            );
            peak = peak.max(map(p).powi(2));
        }
    }
    field_image(rect, px_w, move |p| {
        let v = map(p);
        let d = (v * v / peak).powf(0.35) as f32;
        let hue = if v >= 0.0 {
            rgb8(255, 150, 60)
        } else {
            rgb8(70, 150, 255)
        };
        rgb8(4, 6, 14)
            .mix(hue, d.min(1.0))
            .mix(WHITE, (d - 0.8).max(0.0) * 1.5)
    })
}

// ───────────────────────────── reaction energetics ─────────────────────────────

/// Potential energy along a reaction coordinate x ∈ [0, 1]: flat reactants, a
/// smooth barrier peaking at `ts` (x = 0.5) and flat products.
pub fn reaction_energy(x: f64, reactants: f64, ts: f64, products: f64) -> f64 {
    let bump = |u: f64| (u.clamp(0.0, 1.0) * PI / 2.0).sin().powi(2);
    if x <= 0.5 {
        reactants + (ts - reactants) * bump((x - 0.12) / 0.38)
    } else {
        products + (ts - products) * bump((0.88 - x) / 0.38)
    }
}

/// An annotated energy diagram `w × h` centred at the origin: the uncatalysed curve,
/// an optional catalysed barrier, Ea and ΔH arrows and labels.
pub fn energy_profile(
    w: f64,
    h: f64,
    reactants: f64,
    ts: f64,
    products: f64,
    catalysed_ts: Option<f64>,
    progress: f64,
) -> Mobject {
    let lo = reactants.min(products).min(ts) - 0.15 * (ts - reactants.min(products)).abs();
    let hi = ts + 0.2 * (ts - lo).abs();
    let map = |x: f64, e: f64| v2(-w / 2.0 + w * x, -h / 2.0 + h * (e - lo) / (hi - lo));
    let mut kids = vec![
        arrow(map(0.0, lo), map(0.0, hi)).color(GRAY_B),
        line(map(0.0, lo), map(1.0, lo)).stroke(GRAY_B, 2.0),
        text_sized("energy", 0.24, GRAY_B)
            .rotate(PI / 2.0)
            .move_to(map(0.0, (lo + hi) / 2.0) + v2(-0.25, 0.0)),
        text_sized("reaction coordinate", 0.24, GRAY_B).move_to(map(0.5, lo) + v2(0.0, -0.25)),
    ];
    let curve = |tsv: f64, upto: f64| -> Vec<Vec2> {
        (0..=200)
            .map(|k| k as f64 / 200.0 * upto)
            .map(|x| map(x, reaction_energy(x, reactants, tsv, products)))
            .collect()
    };
    let pr = progress.clamp(0.0, 1.0);
    kids.push(polyline(&curve(ts, pr.max(0.005))).stroke(WHITE, 3.5));
    if let Some(c) = catalysed_ts {
        kids.push(
            polyline(&curve(c, pr.max(0.005)))
                .stroke(GREEN_B, 3.0)
                .dashed(0.12, 0.08),
        );
        if pr > 0.5 {
            kids.push(text_sized("catalysed", 0.24, GREEN_B).move_to(map(0.5, c) + v2(0.0, -0.25)));
        }
    }
    if pr >= 1.0 {
        let ea_x = 0.3;
        kids.push(dashed_line(map(0.0, reactants), map(0.55, reactants)).stroke(GRAY_B, 1.5));
        kids.push(dashed_line(map(0.45, ts), map(0.62, ts)).stroke(GRAY_B, 1.5));
        kids.push(arrow(map(ea_x, reactants), map(ea_x, ts)).color(YELLOW));
        kids.push(
            text_sized("Ea", 0.3, YELLOW)
                .move_to(map(ea_x, (reactants + ts) / 2.0) + v2(-0.32, 0.0)),
        );
        let dh_x = 0.95;
        kids.push(dashed_line(map(0.55, reactants), map(1.0, reactants)).stroke(GRAY_B, 1.5));
        kids.push(
            arrow(map(dh_x, reactants), map(dh_x, products)).color(if products < reactants {
                RED
            } else {
                BLUE
            }),
        );
        kids.push(
            text_sized(&format!("ΔH = {:+.0}", products - reactants), 0.26, WHITE)
                .move_to(map(dh_x, (reactants + products) / 2.0) + v2(-0.75, 0.0)),
        );
    }
    group(kids)
}

/// Arrhenius rate constant k = A·exp(−Ea / RT) (Ea in J/mol).
pub fn arrhenius(a: f64, ea: f64, temp_k: f64) -> f64 {
    a * (-ea / (8.314 * temp_k)).exp()
}

// ───────────────────────────── acid–base titration ─────────────────────────────

/// pH after adding `vb` mL of strong base (conc `cb`) to `va` mL of acid (conc `ca`).
/// `ka = None` means a strong acid; otherwise a monoprotic weak acid. Solved exactly
/// from the charge balance [H⁺] + [Na⁺] = [OH⁻] + [A⁻] by bisection.
pub fn titration_ph(ca: f64, va: f64, cb: f64, vb: f64, ka: Option<f64>) -> f64 {
    let kw = 1e-14;
    let vt = va + vb;
    let a_tot = ca * va / vt;
    let na = cb * vb / vt;
    let balance = |ph: f64| {
        let h = 10f64.powf(-ph);
        let a = match ka {
            Some(k) => a_tot * k / (k + h),
            None => a_tot,
        };
        h + na - kw / h - a
    };
    let (mut lo, mut hi) = (-1.0, 15.0);
    for _ in 0..80 {
        let mid = (lo + hi) / 2.0;
        if balance(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Colour of an indicator solution at `ph`: "phenolphthalein", "methyl orange",
/// "bromothymol blue" or "universal".
pub fn indicator_color(name: &str, ph: f64) -> Color {
    let ramp = |a: f64, b: f64, ca: Color, cb: Color| ca.mix(cb, smoothstep(a, b, ph) as f32);
    match name {
        "phenolphthalein" => ramp(
            8.2,
            10.0,
            Color::rgba(0.9, 0.95, 1.0, 0.35),
            rgb8(235, 60, 160),
        ),
        "methyl orange" => ramp(3.1, 4.4, rgb8(220, 40, 40), rgb8(250, 200, 40)),
        "bromothymol blue" => ramp(6.0, 7.6, rgb8(235, 210, 40), rgb8(40, 90, 220)),
        _ => color_gradient(
            &[
                rgb8(220, 30, 30),
                rgb8(250, 140, 30),
                rgb8(245, 225, 40),
                rgb8(80, 190, 60),
                rgb8(40, 120, 220),
                rgb8(110, 50, 170),
            ],
            ph / 14.0,
        ),
    }
}

// ───────────────────────────── epidemics ─────────────────────────────

/// Compartment of an agent in [`Epidemic`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sir {
    S,
    I,
    R,
}

/// Agent-based SIR epidemic: random walkers in a box, infection on contact within
/// `radius` with probability `p` per step, recovery after `recover` seconds.
/// Precomputed; sample with [`Epidemic::state`].
#[derive(Clone, Debug)]
pub struct Epidemic {
    pub dt: f64,
    pub positions: Vec<Vec<Vec2>>,
    pub states: Vec<Vec<Sir>>,
}

impl Epidemic {
    #[allow(clippy::too_many_arguments)]
    pub fn simulate(
        n: usize,
        bounds: Rect,
        speed: f64,
        radius: f64,
        p: f64,
        recover: f64,
        duration: f64,
        seed: u64,
    ) -> Epidemic {
        let mut rng = Rng::new(seed);
        let dt = 1.0 / 30.0;
        let mut pos: Vec<Vec2> = (0..n)
            .map(|_| {
                v2(
                    rng.range(bounds.min.x, bounds.max.x),
                    rng.range(bounds.min.y, bounds.max.y),
                )
            })
            .collect();
        let mut vel: Vec<Vec2> = (0..n)
            .map(|_| Vec2::from_angle(rng.range(0.0, PI * 2.0)) * speed)
            .collect();
        let mut st = vec![Sir::S; n];
        let mut sick_for = vec![0.0; n];
        for s in st.iter_mut().take(3.min(n)) {
            *s = Sir::I;
        }
        let steps = (duration / dt).ceil() as usize;
        let (mut positions, mut states) = (vec![pos.clone()], vec![st.clone()]);
        for _ in 0..steps {
            for i in 0..n {
                vel[i] = vel[i].rotate(rng.range(-0.4, 0.4));
                pos[i] += vel[i] * dt;
                if pos[i].x < bounds.min.x || pos[i].x > bounds.max.x {
                    vel[i].x = -vel[i].x;
                    pos[i].x = pos[i].x.clamp(bounds.min.x, bounds.max.x);
                }
                if pos[i].y < bounds.min.y || pos[i].y > bounds.max.y {
                    vel[i].y = -vel[i].y;
                    pos[i].y = pos[i].y.clamp(bounds.min.y, bounds.max.y);
                }
            }
            let prev = st.clone();
            for i in 0..n {
                match prev[i] {
                    Sir::I => {
                        sick_for[i] += dt;
                        if sick_for[i] >= recover {
                            st[i] = Sir::R;
                        }
                    }
                    Sir::S => {
                        let exposed =
                            (0..n).any(|j| prev[j] == Sir::I && pos[j].distance(pos[i]) < radius);
                        if exposed && rng.next_f64() < p {
                            st[i] = Sir::I;
                        }
                    }
                    Sir::R => {}
                }
            }
            positions.push(pos.clone());
            states.push(st.clone());
        }
        Epidemic {
            dt,
            positions,
            states,
        }
    }

    fn idx(&self, t: f64) -> usize {
        ((t / self.dt).round().max(0.0) as usize).min(self.states.len() - 1)
    }
    /// Positions and compartments at time `t`.
    pub fn state(&self, t: f64) -> (&[Vec2], &[Sir]) {
        let i = self.idx(t);
        (&self.positions[i], &self.states[i])
    }
    /// (S, I, R) counts at time `t`.
    pub fn counts(&self, t: f64) -> (usize, usize, usize) {
        let s = &self.states[self.idx(t)];
        let c = |x: Sir| s.iter().filter(|&&y| y == x).count();
        (c(Sir::S), c(Sir::I), c(Sir::R))
    }
}

/// Colour convention for SIR compartments.
pub fn sir_color(s: Sir) -> Color {
    match s {
        Sir::S => rgb8(90, 170, 255),
        Sir::I => rgb8(255, 80, 80),
        Sir::R => rgb8(150, 150, 165),
    }
}

/// Classic SIR ODE (fractions), returning (S, I, R) at `t` via fixed-step RK4.
pub fn sir_ode(beta: f64, gamma: f64, i0: f64, t: f64) -> (f64, f64, f64) {
    let f = |y: [f64; 3]| {
        [
            -beta * y[0] * y[1],
            beta * y[0] * y[1] - gamma * y[1],
            gamma * y[1],
        ]
    };
    let mut y = [1.0 - i0, i0, 0.0];
    let steps = ((t / 0.01).ceil() as usize).max(1);
    let h = t / steps as f64;
    let add =
        |a: [f64; 3], b: [f64; 3], k: f64| [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k];
    for _ in 0..steps {
        let k1 = f(y);
        let k2 = f(add(y, k1, h / 2.0));
        let k3 = f(add(y, k2, h / 2.0));
        let k4 = f(add(y, k3, h));
        for j in 0..3 {
            y[j] += h / 6.0 * (k1[j] + 2.0 * k2[j] + 2.0 * k3[j] + k4[j]);
        }
    }
    (y[0], y[1], y[2])
}

// ───────────────────────────── radioactive decay ─────────────────────────────

/// Deterministic decay times (exponentially distributed) for `n` nuclei.
pub fn decay_times(n: usize, half_life: f64, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let tau = half_life / std::f64::consts::LN_2;
    (0..n)
        .map(|_| -tau * (1.0 - rng.next_f64()).max(1e-12).ln())
        .collect()
}

/// A `cols × rows` grid of nuclei at time `t`: undecayed glow, decayed ones fade and
/// a brief flash marks each decay.
pub fn decay_grid(
    times: &[f64],
    cols: usize,
    cell: f64,
    t: f64,
    alive: Color,
    dead: Color,
) -> Mobject {
    let rows = times.len().div_ceil(cols.max(1));
    let off = v2(
        (cols as f64 - 1.0) * cell / 2.0,
        (rows as f64 - 1.0) * cell / 2.0,
    );
    group(
        times
            .iter()
            .enumerate()
            .map(|(k, &td)| {
                let p =
                    v2((k % cols) as f64 * cell, -((k / cols) as f64) * cell) - v2(off.x, -off.y);
                if t < td {
                    dot_r(p, cell * 0.36, alive)
                } else {
                    let flash = (1.0 - (t - td) / 0.35).max(0.0) as f32;
                    dot_r(p, cell * (0.3 + 0.4 * flash as f64), dead.mix(WHITE, flash))
                }
            })
            .collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_normalised() {
        for (n, l) in [(1, 0), (2, 1), (3, 0), (3, 2), (4, 1)] {
            let dr = 0.005;
            let s: f64 = (1..12000)
                .map(|k| {
                    let r = k as f64 * dr;
                    (hydrogen_radial(n, l, r) * r).powi(2) * dr
                })
                .sum();
            assert!((s - 1.0).abs() < 1e-3, "{n}{l}: {s}");
        }
        // 1s: R = 2 e^{-r}
        assert!((hydrogen_radial(1, 0, 1.0) - 2.0 * (-1.0f64).exp()).abs() < 1e-12);
        // 2s has a radial node at r = 2
        assert!(hydrogen_radial(2, 0, 2.0).abs() < 1e-12);
    }

    #[test]
    fn spherical_harmonic_values() {
        let y00 = real_ylm(0, 0, 0.3, 0.2);
        assert!((y00 - 0.5 / PI.sqrt()).abs() < 1e-12);
        // p_z ∝ cos θ, zero on the equator
        assert!(real_ylm(1, 0, PI / 2.0, 0.0).abs() < 1e-12);
        assert_eq!(orbital_name(3, 2), "3d");
    }

    #[test]
    fn titration_landmarks() {
        // strong/strong: equivalence at pH 7
        let ph = titration_ph(0.1, 25.0, 0.1, 25.0, None);
        assert!((ph - 7.0).abs() < 1e-3, "{ph}");
        // weak acid: half-equivalence pH = pKa
        let ka: f64 = 1.8e-5;
        let ph = titration_ph(0.1, 25.0, 0.1, 12.5, Some(ka));
        assert!((ph + ka.log10()).abs() < 0.02, "{ph}");
        // initial strong acid 0.1 M → pH 1
        assert!((titration_ph(0.1, 25.0, 0.1, 0.0, None) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn energy_and_rates() {
        assert_eq!(reaction_energy(0.0, 10.0, 80.0, -20.0), 10.0);
        assert_eq!(reaction_energy(0.5, 10.0, 80.0, -20.0), 80.0);
        assert_eq!(reaction_energy(1.0, 10.0, 80.0, -20.0), -20.0);
        assert!(arrhenius(1.0, 50_000.0, 350.0) > arrhenius(1.0, 50_000.0, 300.0));
    }

    #[test]
    fn sir_conserves_population() {
        let (s, i, r) = sir_ode(0.5, 0.1, 0.01, 40.0);
        assert!((s + i + r - 1.0).abs() < 1e-9);
        let e = Epidemic::simulate(
            80,
            Rect::from_center(Vec2::ZERO, 4.0, 3.0),
            0.6,
            0.2,
            0.3,
            3.0,
            6.0,
            1,
        );
        let (a, b, c) = e.counts(6.0);
        assert_eq!(a + b + c, 80);
        assert!(e.counts(0.0).1 == 3);
    }

    #[test]
    fn decay_half_life() {
        let t = decay_times(20000, 2.0, 3);
        let left = t.iter().filter(|&&x| x > 2.0).count() as f64 / 20000.0;
        assert!((left - 0.5).abs() < 0.02, "{left}");
    }
}
