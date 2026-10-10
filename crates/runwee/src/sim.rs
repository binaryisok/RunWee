//! Simulation core: RK4 integration into time-indexed trajectories (so every frame
//! stays a pure function of time and can be rendered in parallel), plus ready-made
//! physical systems: pendulums, double pendulum, spring–mass, projectiles,
//! N-body gravity and an ideal gas with elastic collisions.

use crate::math::*;

/// Sampled solution of an ODE: `states[i]` at time `i * dt`.
#[derive(Clone, Debug)]
pub struct Trajectory {
    pub dt: f64,
    pub states: Vec<Vec<f64>>,
}

impl Trajectory {
    pub fn duration(&self) -> f64 {
        (self.states.len().saturating_sub(1)) as f64 * self.dt
    }
    /// Linearly interpolated state at time `t` (clamped).
    pub fn at(&self, t: f64) -> Vec<f64> {
        if self.states.is_empty() {
            return vec![];
        }
        let x = (t / self.dt).clamp(0.0, (self.states.len() - 1) as f64);
        let i = x.floor() as usize;
        let j = (i + 1).min(self.states.len() - 1);
        let f = x - i as f64;
        self.states[i]
            .iter()
            .zip(&self.states[j])
            .map(|(a, b)| a + (b - a) * f)
            .collect()
    }
    /// Component `k` at time `t`.
    pub fn get(&self, t: f64, k: usize) -> f64 {
        self.at(t).get(k).copied().unwrap_or(0.0)
    }
}

/// Integrate `dy/dt = f(t, y)` with classic RK4 from `y0` over `[0, t_end]`.
pub fn rk4(f: impl Fn(f64, &[f64]) -> Vec<f64>, y0: &[f64], t_end: f64, dt: f64) -> Trajectory {
    let n = (t_end / dt).ceil().max(1.0) as usize;
    let mut states = Vec::with_capacity(n + 1);
    let mut y = y0.to_vec();
    states.push(y.clone());
    let add = |a: &[f64], b: &[f64], k: f64| -> Vec<f64> {
        a.iter().zip(b).map(|(x, y)| x + y * k).collect()
    };
    for i in 0..n {
        let t = i as f64 * dt;
        let k1 = f(t, &y);
        let k2 = f(t + dt / 2.0, &add(&y, &k1, dt / 2.0));
        let k3 = f(t + dt / 2.0, &add(&y, &k2, dt / 2.0));
        let k4 = f(t + dt, &add(&y, &k3, dt));
        for j in 0..y.len() {
            y[j] += dt / 6.0 * (k1[j] + 2.0 * k2[j] + 2.0 * k3[j] + k4[j]);
        }
        states.push(y.clone());
    }
    Trajectory { dt, states }
}

/// Simple pendulum: state `[θ, ω]` (θ from vertical, radians).
pub fn pendulum(length: f64, theta0: f64, g: f64, damping: f64, duration: f64) -> Trajectory {
    rk4(
        move |_, y| vec![y[1], -g / length * y[0].sin() - damping * y[1]],
        &[theta0, 0.0],
        duration,
        1.0 / 600.0,
    )
}

/// Double pendulum: state `[θ1, ω1, θ2, ω2]` (equal masses by default).
pub fn double_pendulum(
    l1: f64,
    l2: f64,
    m1: f64,
    m2: f64,
    th1: f64,
    th2: f64,
    g: f64,
    duration: f64,
) -> Trajectory {
    rk4(
        move |_, y| {
            let (t1, w1, t2, w2) = (y[0], y[1], y[2], y[3]);
            let d = t1 - t2;
            let den = 2.0 * m1 + m2 - m2 * (2.0 * d).cos();
            let a1 = (-g * (2.0 * m1 + m2) * t1.sin()
                - m2 * g * (t1 - 2.0 * t2).sin()
                - 2.0 * d.sin() * m2 * (w2 * w2 * l2 + w1 * w1 * l1 * d.cos()))
                / (l1 * den);
            let a2 = (2.0
                * d.sin()
                * (w1 * w1 * l1 * (m1 + m2)
                    + g * (m1 + m2) * t1.cos()
                    + w2 * w2 * l2 * m2 * d.cos()))
                / (l2 * den);
            vec![w1, a1, w2, a2]
        },
        &[th1, 0.0, th2, 0.0],
        duration,
        1.0 / 1000.0,
    )
}

/// Damped spring–mass: state `[x, v]`.
pub fn spring_mass(k: f64, m: f64, x0: f64, v0: f64, damping: f64, duration: f64) -> Trajectory {
    rk4(
        move |_, y| vec![y[1], (-k * y[0] - damping * y[1]) / m],
        &[x0, v0],
        duration,
        1.0 / 600.0,
    )
}

/// Driven damped oscillator `m x'' + c x' + k x = F cos(ω t)`.
pub fn driven_oscillator(
    k: f64,
    m: f64,
    c: f64,
    force: f64,
    omega: f64,
    duration: f64,
) -> Trajectory {
    rk4(
        move |t, y| vec![y[1], (force * (omega * t).cos() - c * y[1] - k * y[0]) / m],
        &[0.0, 0.0],
        duration,
        1.0 / 600.0,
    )
}

/// Ideal projectile position at time `t` (no drag).
pub fn projectile(p0: Vec2, speed: f64, angle: f64, g: f64, t: f64) -> Vec2 {
    p0 + v2(
        speed * angle.cos() * t,
        speed * angle.sin() * t - 0.5 * g * t * t,
    )
}

/// Projectile with quadratic air drag: state `[x, y, vx, vy]`.
pub fn projectile_drag(
    p0: Vec2,
    speed: f64,
    angle: f64,
    g: f64,
    drag: f64,
    duration: f64,
) -> Trajectory {
    rk4(
        move |_, y| {
            let v = (y[2] * y[2] + y[3] * y[3]).sqrt();
            vec![y[2], y[3], -drag * v * y[2], -g - drag * v * y[3]]
        },
        &[p0.x, p0.y, speed * angle.cos(), speed * angle.sin()],
        duration,
        1.0 / 600.0,
    )
}

/// Time of flight, range and max height of an ideal projectile launched from y = 0.
pub fn projectile_stats(speed: f64, angle: f64, g: f64) -> (f64, f64, f64) {
    let tof = 2.0 * speed * angle.sin() / g;
    (
        tof,
        speed * angle.cos() * tof,
        (speed * angle.sin()).powi(2) / (2.0 * g),
    )
}

/// N-body gravity. `bodies` = (mass, position, velocity). State per body: x, y, vx, vy.
pub fn n_body(
    bodies: &[(f64, Vec2, Vec2)],
    g: f64,
    softening: f64,
    duration: f64,
    dt: f64,
) -> Trajectory {
    let masses: Vec<f64> = bodies.iter().map(|b| b.0).collect();
    let y0: Vec<f64> = bodies
        .iter()
        .flat_map(|b| [b.1.x, b.1.y, b.2.x, b.2.y])
        .collect();
    rk4(
        move |_, y| {
            let n = masses.len();
            let mut d = vec![0.0; 4 * n];
            for i in 0..n {
                d[4 * i] = y[4 * i + 2];
                d[4 * i + 1] = y[4 * i + 3];
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let dx = y[4 * j] - y[4 * i];
                    let dy = y[4 * j + 1] - y[4 * i + 1];
                    let r2 = dx * dx + dy * dy + softening * softening;
                    let f = g * masses[j] / (r2 * r2.sqrt());
                    d[4 * i + 2] += f * dx;
                    d[4 * i + 3] += f * dy;
                }
            }
            d
        },
        &y0,
        duration,
        dt,
    )
}

/// Body `i` position from an [`n_body`] trajectory.
pub fn body_pos(tr: &Trajectory, i: usize, t: f64) -> Vec2 {
    let s = tr.at(t);
    v2(s[4 * i], s[4 * i + 1])
}

/// Ideal gas of hard discs in a box with elastic collisions (precomputed).
#[derive(Clone, Debug)]
pub struct Gas {
    pub dt: f64,
    pub radius: f64,
    pub bounds: crate::path::Rect,
    /// positions per frame
    pub frames: Vec<Vec<Vec2>>,
    pub velocities: Vec<Vec<Vec2>>,
}

impl Gas {
    pub fn simulate(
        n: usize,
        bounds: crate::path::Rect,
        radius: f64,
        speed: f64,
        duration: f64,
        seed: u64,
    ) -> Gas {
        let mut rng = Rng::new(seed);
        let mut pos: Vec<Vec2> = Vec::with_capacity(n);
        while pos.len() < n {
            let p = v2(
                rng.range(bounds.min.x + radius, bounds.max.x - radius),
                rng.range(bounds.min.y + radius, bounds.max.y - radius),
            );
            if pos.iter().all(|q| q.distance(p) > 2.2 * radius) || pos.len() > n * 50 {
                pos.push(p);
            }
        }
        // Maxwell-ish: gaussian components
        let mut vel: Vec<Vec2> = (0..n)
            .map(|_| v2(rng.normal(), rng.normal()) * (speed / 2f64.sqrt()))
            .collect();
        let dt = 1.0 / 240.0;
        let steps = (duration / dt).ceil() as usize;
        let mut frames = Vec::with_capacity(steps + 1);
        let mut vels = Vec::with_capacity(steps + 1);
        frames.push(pos.clone());
        vels.push(vel.clone());
        for _ in 0..steps {
            for i in 0..n {
                pos[i] += vel[i] * dt;
                if pos[i].x < bounds.min.x + radius && vel[i].x < 0.0
                    || pos[i].x > bounds.max.x - radius && vel[i].x > 0.0
                {
                    vel[i].x = -vel[i].x;
                }
                if pos[i].y < bounds.min.y + radius && vel[i].y < 0.0
                    || pos[i].y > bounds.max.y - radius && vel[i].y > 0.0
                {
                    vel[i].y = -vel[i].y;
                }
            }
            for i in 0..n {
                for j in i + 1..n {
                    let d = pos[j] - pos[i];
                    let dist = d.length();
                    if dist < 2.0 * radius && dist > 1e-9 {
                        let nrm = d / dist;
                        let rel = (vel[i] - vel[j]).dot(nrm);
                        if rel > 0.0 {
                            vel[i] -= nrm * rel;
                            vel[j] += nrm * rel;
                        }
                    }
                }
            }
            frames.push(pos.clone());
            vels.push(vel.clone());
        }
        Gas {
            dt,
            radius,
            bounds,
            frames,
            velocities: vels,
        }
    }
    pub fn positions(&self, t: f64) -> &[Vec2] {
        let i = ((t / self.dt).round() as usize).min(self.frames.len() - 1);
        &self.frames[i]
    }
    pub fn speeds(&self, t: f64) -> Vec<f64> {
        let i = ((t / self.dt).round() as usize).min(self.velocities.len() - 1);
        self.velocities[i].iter().map(|v| v.length()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pendulum_period() {
        // small angle: T = 2π sqrt(L/g)
        let tr = pendulum(1.0, 0.05, 9.81, 0.0, 4.0);
        let period = TAU * (1.0f64 / 9.81).sqrt();
        assert!((tr.get(period, 0) - 0.05).abs() < 1e-3);
        assert!((tr.get(period / 2.0, 0) + 0.05).abs() < 1e-3);
    }

    #[test]
    fn energy_conserved_in_spring() {
        let tr = spring_mass(4.0, 1.0, 1.0, 0.0, 0.0, 10.0);
        let e = |t: f64| {
            let s = tr.at(t);
            0.5 * 4.0 * s[0] * s[0] + 0.5 * s[1] * s[1]
        };
        assert!((e(0.0) - e(9.7)).abs() < 1e-6);
    }

    #[test]
    fn projectile_and_orbit() {
        let (tof, range, h) = projectile_stats(10.0, PI / 4.0, 9.81);
        assert!((range - 100.0 / 9.81).abs() < 1e-9);
        assert!((projectile(Vec2::ZERO, 10.0, PI / 4.0, 9.81, tof).y).abs() < 1e-9);
        assert!(h > 2.5);
        // circular orbit: v = sqrt(GM/r)
        let tr = n_body(
            &[
                (1000.0, Vec2::ZERO, Vec2::ZERO),
                (1.0, v2(2.0, 0.0), v2(0.0, (1000.0f64 / 2.0).sqrt())),
            ],
            1.0,
            0.0,
            1.0,
            1e-4,
        );
        let r = (body_pos(&tr, 1, 0.9) - body_pos(&tr, 0, 0.9)).length();
        assert!((r - 2.0).abs() < 0.02, "{r}");
        let dp = double_pendulum(1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 9.81, 1.0);
        assert_eq!(dp.at(0.5).len(), 4);
    }

    #[test]
    fn gas_stays_in_box_and_conserves_energy() {
        let b = crate::path::Rect::new(v2(-2.0, -2.0), v2(2.0, 2.0));
        let g = Gas::simulate(30, b, 0.08, 2.0, 2.0, 1);
        let e0: f64 = g.speeds(0.0).iter().map(|v| v * v).sum();
        let e1: f64 = g.speeds(2.0).iter().map(|v| v * v).sum();
        assert!((e0 - e1).abs() / e0 < 1e-9);
        assert!(g.positions(1.5).iter().all(|p| b.expand(0.05).contains(*p)));
    }
}
