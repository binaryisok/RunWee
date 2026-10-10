//! Rate functions (manim) and easing curves / `interpolate` / `spring` (Remotion).

use crate::math::{clamp01, PI};
use std::sync::Arc;

/// A timing curve mapping linear progress `t ∈ [0,1]` to eased progress.
#[derive(Clone, Default)]
pub enum Rate {
    Linear,
    /// manim's default: smooth S-curve.
    #[default]
    Smooth,
    Smoother,
    RushInto,
    RushFrom,
    SlowInto,
    DoubleSmooth,
    ThereAndBack,
    ThereAndBackWithPause(f64),
    RunningStart(f64),
    NotQuiteThere(f64),
    Wiggle(f64),
    Lingering,
    ExponentialDecay(f64),
    InSine,
    OutSine,
    InOutSine,
    InQuad,
    OutQuad,
    InOutQuad,
    InCubic,
    OutCubic,
    InOutCubic,
    InQuart,
    OutQuart,
    InOutQuart,
    InQuint,
    OutQuint,
    InOutQuint,
    InExpo,
    OutExpo,
    InOutExpo,
    InCirc,
    OutCirc,
    InOutCirc,
    InBack,
    OutBack,
    InOutBack,
    InElastic,
    OutElastic,
    InOutElastic,
    InBounce,
    OutBounce,
    InOutBounce,
    /// CSS / Remotion `Easing.bezier(x1, y1, x2, y2)`.
    Bezier(f64, f64, f64, f64),
    /// Remotion-style spring as a rate curve (damping ratio-ish config).
    Spring(SpringConfig),
    /// Discrete steps.
    Steps(u32),
    /// Apply an inner rate inside the window `[a, b]` of the animation.
    Squish(Box<Rate>, f64, f64),
    Custom(Arc<dyn Fn(f64) -> f64 + Send + Sync>),
}

impl std::fmt::Debug for Rate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rate::Custom(_) => write!(f, "Custom"),
            Rate::Squish(r, a, b) => write!(f, "Squish({r:?}, {a}, {b})"),
            Rate::Bezier(a, b, c, d) => write!(f, "Bezier({a},{b},{c},{d})"),
            other => write!(f, "{}", other.name()),
        }
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn smooth(t: f64, inflection: f64) -> f64 {
    let err = sigmoid(-inflection / 2.0);
    clamp01((sigmoid(inflection * (t - 0.5)) - err) / (1.0 - 2.0 * err))
}

fn bounce_out(t: f64) -> f64 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

impl Rate {
    pub fn custom(f: impl Fn(f64) -> f64 + Send + Sync + 'static) -> Self {
        Rate::Custom(Arc::new(f))
    }

    pub fn squish(self, a: f64, b: f64) -> Self {
        Rate::Squish(Box::new(self), a, b)
    }

    /// Evaluate the curve. Input is clamped to `[0,1]`.
    pub fn eval(&self, t: f64) -> f64 {
        let t = clamp01(t);
        use Rate::*;
        let c1 = 1.70158;
        let c2 = c1 * 1.525;
        let c3 = c1 + 1.0;
        let c4 = (2.0 * PI) / 3.0;
        let c5 = (2.0 * PI) / 4.5;
        match self {
            Linear => t,
            Smooth => {
                // manim's 6t^5 - 15t^4 + 10t^3 variant ("smootherstep") is "Smoother";
                // "smooth" is the sigmoid-based curve with inflection 10.
                smooth(t, 10.0)
            }
            Smoother => t * t * t * (10.0 - 15.0 * t + 6.0 * t * t),
            RushInto => 2.0 * smooth(t / 2.0, 10.0),
            RushFrom => 2.0 * smooth(t / 2.0 + 0.5, 10.0) - 1.0,
            SlowInto => (1.0 - (1.0 - t) * (1.0 - t)).sqrt(),
            DoubleSmooth => {
                if t < 0.5 {
                    0.5 * smooth(2.0 * t, 10.0)
                } else {
                    0.5 * (1.0 + smooth(2.0 * t - 1.0, 10.0))
                }
            }
            ThereAndBack => {
                let nt = if t < 0.5 { 2.0 * t } else { 2.0 * (1.0 - t) };
                smooth(nt, 10.0)
            }
            ThereAndBackWithPause(p) => {
                let a = 1.0 / p.max(1e-6);
                if t < 0.5 - p / 2.0 {
                    smooth(a * t, 10.0)
                } else if t < 0.5 + p / 2.0 {
                    1.0
                } else {
                    smooth(a - a * t, 10.0)
                }
            }
            RunningStart(pull) => {
                // cubic bezier with control values 0, 0, -pull, -pull, 1, 1
                let p = [0.0, 0.0, -pull, -pull, 1.0, 1.0, 1.0];
                bezier_scalar(&p, t)
            }
            NotQuiteThere(p) => p * smooth(t, 10.0),
            Wiggle(n) => (smooth(t, 10.0) * (1.0 - smooth(t, 10.0)) * 4.0) * (n * PI * t).sin(),
            Lingering => smooth((t / 0.8).min(1.0), 10.0),
            ExponentialDecay(h) => 1.0 - (-t / h).exp(),
            InSine => 1.0 - (t * PI / 2.0).cos(),
            OutSine => (t * PI / 2.0).sin(),
            InOutSine => -((PI * t).cos() - 1.0) / 2.0,
            InQuad => t * t,
            OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            InOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            InCubic => t.powi(3),
            OutCubic => 1.0 - (1.0 - t).powi(3),
            InOutCubic => {
                if t < 0.5 {
                    4.0 * t.powi(3)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            InQuart => t.powi(4),
            OutQuart => 1.0 - (1.0 - t).powi(4),
            InOutQuart => {
                if t < 0.5 {
                    8.0 * t.powi(4)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            InQuint => t.powi(5),
            OutQuint => 1.0 - (1.0 - t).powi(5),
            InOutQuint => {
                if t < 0.5 {
                    16.0 * t.powi(5)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            }
            InExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2f64.powf(10.0 * t - 10.0)
                }
            }
            OutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2f64.powf(-10.0 * t)
                }
            }
            InOutExpo => {
                if t == 0.0 || t == 1.0 {
                    t
                } else if t < 0.5 {
                    2f64.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2f64.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            InCirc => 1.0 - (1.0 - t * t).sqrt(),
            OutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
            InOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }
            InBack => c3 * t.powi(3) - c1 * t * t,
            OutBack => 1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2),
            InOutBack => {
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }
            InElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    -(2f64.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
                }
            }
            OutElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    2f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
            InOutElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else if t < 0.5 {
                    -(2f64.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0
                } else {
                    (2f64.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0 + 1.0
                }
            }
            InBounce => 1.0 - bounce_out(1.0 - t),
            OutBounce => bounce_out(t),
            InOutBounce => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }
            Bezier(x1, y1, x2, y2) => cubic_bezier_ease(*x1, *y1, *x2, *y2, t),
            Spring(cfg) => {
                // Map t ∈ [0,1] onto the settle time of the spring.
                let settle = cfg.settle_time();
                spring_value(t * settle, *cfg)
            }
            Steps(n) => {
                let n = (*n).max(1) as f64;
                (t * n).floor().min(n) / n
            }
            Squish(inner, a, b) => {
                if (b - a).abs() < 1e-12 {
                    if t < *a {
                        inner.eval(0.0)
                    } else {
                        inner.eval(1.0)
                    }
                } else {
                    inner.eval((t - a) / (b - a))
                }
            }
            Custom(f) => f(t),
        }
    }

    pub fn name(&self) -> &'static str {
        use Rate::*;
        match self {
            Linear => "linear",
            Smooth => "smooth",
            Smoother => "smoother",
            RushInto => "rush_into",
            RushFrom => "rush_from",
            SlowInto => "slow_into",
            DoubleSmooth => "double_smooth",
            ThereAndBack => "there_and_back",
            ThereAndBackWithPause(_) => "there_and_back_with_pause",
            RunningStart(_) => "running_start",
            NotQuiteThere(_) => "not_quite_there",
            Wiggle(_) => "wiggle",
            Lingering => "lingering",
            ExponentialDecay(_) => "exponential_decay",
            InSine => "ease_in_sine",
            OutSine => "ease_out_sine",
            InOutSine => "ease_in_out_sine",
            InQuad => "ease_in_quad",
            OutQuad => "ease_out_quad",
            InOutQuad => "ease_in_out_quad",
            InCubic => "ease_in_cubic",
            OutCubic => "ease_out_cubic",
            InOutCubic => "ease_in_out_cubic",
            InQuart => "ease_in_quart",
            OutQuart => "ease_out_quart",
            InOutQuart => "ease_in_out_quart",
            InQuint => "ease_in_quint",
            OutQuint => "ease_out_quint",
            InOutQuint => "ease_in_out_quint",
            InExpo => "ease_in_expo",
            OutExpo => "ease_out_expo",
            InOutExpo => "ease_in_out_expo",
            InCirc => "ease_in_circ",
            OutCirc => "ease_out_circ",
            InOutCirc => "ease_in_out_circ",
            InBack => "ease_in_back",
            OutBack => "ease_out_back",
            InOutBack => "ease_in_out_back",
            InElastic => "ease_in_elastic",
            OutElastic => "ease_out_elastic",
            InOutElastic => "ease_in_out_elastic",
            InBounce => "ease_in_bounce",
            OutBounce => "ease_out_bounce",
            InOutBounce => "ease_in_out_bounce",
            Bezier(..) => "bezier",
            Spring(_) => "spring",
            Steps(_) => "steps",
            Squish(..) => "squish",
            Custom(_) => "custom",
        }
    }

    /// Look up a rate function by its name (as used in scripts).
    pub fn from_name(name: &str) -> Option<Rate> {
        use Rate::*;
        let all = [
            Linear,
            Smooth,
            Smoother,
            RushInto,
            RushFrom,
            SlowInto,
            DoubleSmooth,
            ThereAndBack,
            ThereAndBackWithPause(1.0 / 3.0),
            RunningStart(0.2),
            NotQuiteThere(0.7),
            Wiggle(2.0),
            Lingering,
            ExponentialDecay(0.1),
            InSine,
            OutSine,
            InOutSine,
            InQuad,
            OutQuad,
            InOutQuad,
            InCubic,
            OutCubic,
            InOutCubic,
            InQuart,
            OutQuart,
            InOutQuart,
            InQuint,
            OutQuint,
            InOutQuint,
            InExpo,
            OutExpo,
            InOutExpo,
            InCirc,
            OutCirc,
            InOutCirc,
            InBack,
            OutBack,
            InOutBack,
            InElastic,
            OutElastic,
            InOutElastic,
            InBounce,
            OutBounce,
            InOutBounce,
            Spring(SpringConfig::default()),
        ];
        let n = name.to_ascii_lowercase().replace('-', "_");
        let n = n.as_str();
        let n = match n {
            "ease" => return Some(Bezier(0.25, 0.1, 0.25, 1.0)),
            "ease_in" => return Some(Bezier(0.42, 0.0, 1.0, 1.0)),
            "ease_out" => return Some(Bezier(0.0, 0.0, 0.58, 1.0)),
            "ease_in_out" => return Some(Bezier(0.42, 0.0, 0.58, 1.0)),
            "smoothstep" => "smoother",
            other => other,
        };
        all.into_iter().find(|r| r.name() == n)
    }
}

fn bezier_scalar(points: &[f64], t: f64) -> f64 {
    // De Casteljau on scalar control points.
    let mut p = points.to_vec();
    let n = p.len();
    for k in 1..n {
        for i in 0..n - k {
            p[i] = p[i] * (1.0 - t) + p[i + 1] * t;
        }
    }
    p[0]
}

/// CSS cubic-bezier easing: solves x(s) = t by Newton + bisection, returns y(s).
pub fn cubic_bezier_ease(x1: f64, y1: f64, x2: f64, y2: f64, t: f64) -> f64 {
    let bx = |s: f64| 3.0 * (1.0 - s).powi(2) * s * x1 + 3.0 * (1.0 - s) * s * s * x2 + s.powi(3);
    let by = |s: f64| 3.0 * (1.0 - s).powi(2) * s * y1 + 3.0 * (1.0 - s) * s * s * y2 + s.powi(3);
    let dbx = |s: f64| {
        3.0 * (1.0 - s).powi(2) * x1 + 6.0 * (1.0 - s) * s * (x2 - x1) + 3.0 * s * s * (1.0 - x2)
    };
    let mut s = t;
    for _ in 0..8 {
        let err = bx(s) - t;
        if err.abs() < 1e-7 {
            return by(s);
        }
        let d = dbx(s);
        if d.abs() < 1e-6 {
            break;
        }
        s = (s - err / d).clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    s = t;
    for _ in 0..40 {
        let x = bx(s);
        if (x - t).abs() < 1e-7 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) / 2.0;
    }
    by(s)
}

/// How values outside the input range behave in [`interpolate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Extrapolate {
    #[default]
    Extend,
    Clamp,
    Identity,
}

/// Remotion's `interpolate(input, inputRange, outputRange, {easing, extrapolateLeft, extrapolateRight})`.
pub fn interpolate_with(
    input: f64,
    input_range: &[f64],
    output_range: &[f64],
    easing: &Rate,
    left: Extrapolate,
    right: Extrapolate,
) -> f64 {
    assert!(
        input_range.len() == output_range.len() && input_range.len() >= 2,
        "interpolate: ranges must have the same length (>= 2)"
    );
    let n = input_range.len();
    // find segment
    let mut i = 0;
    while i < n - 2 && input >= input_range[i + 1] {
        i += 1;
    }
    let (i0, i1) = (input_range[i], input_range[i + 1]);
    let (o0, o1) = (output_range[i], output_range[i + 1]);
    let mut x = input;
    if x < i0 {
        match left {
            Extrapolate::Clamp => return o0,
            Extrapolate::Identity => return x,
            Extrapolate::Extend => {}
        }
    }
    if x > i1 {
        match right {
            Extrapolate::Clamp => return o1,
            Extrapolate::Identity => return x,
            Extrapolate::Extend => {}
        }
    }
    if (i1 - i0).abs() < 1e-15 {
        return o0;
    }
    x = (x - i0) / (i1 - i0);
    let eased = if (0.0..=1.0).contains(&x) {
        easing.eval(x)
    } else {
        x
    };
    o0 + (o1 - o0) * eased
}

/// `interpolate` with linear easing and clamping on both sides (the common case).
pub fn interpolate(input: f64, input_range: &[f64], output_range: &[f64]) -> f64 {
    interpolate_with(
        input,
        input_range,
        output_range,
        &Rate::Linear,
        Extrapolate::Clamp,
        Extrapolate::Clamp,
    )
}

/// Physical spring parameters (Remotion defaults).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringConfig {
    pub mass: f64,
    pub damping: f64,
    pub stiffness: f64,
    pub overshoot_clamping: bool,
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            mass: 1.0,
            damping: 10.0,
            stiffness: 100.0,
            overshoot_clamping: false,
        }
    }
}

impl SpringConfig {
    pub fn new(mass: f64, damping: f64, stiffness: f64) -> Self {
        Self {
            mass,
            damping,
            stiffness,
            overshoot_clamping: false,
        }
    }
    /// A bouncy preset.
    pub fn bouncy() -> Self {
        Self::new(1.0, 6.0, 120.0)
    }
    /// A critically damped preset (no overshoot).
    pub fn gentle() -> Self {
        Self::new(1.0, 20.0, 100.0)
    }
    /// Time (seconds) after which the spring stays within 0.1% of its target.
    pub fn settle_time(&self) -> f64 {
        let mut t = 0.0;
        let dt = 1.0 / 240.0;
        let mut last_out = 0.0;
        while t < 30.0 {
            if (spring_value(t, *self) - 1.0).abs() > 1e-3 {
                last_out = t;
            }
            t += dt;
            if t - last_out > 1.0 {
                break;
            }
        }
        last_out.max(dt)
    }
}

/// Analytic damped harmonic oscillator going from 0 to 1, `t` in seconds.
pub fn spring_value(t: f64, cfg: SpringConfig) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    let m = cfg.mass.max(1e-6);
    let w0 = (cfg.stiffness / m).sqrt();
    let zeta = cfg.damping / (2.0 * (cfg.stiffness * m).sqrt());
    let x = if zeta < 1.0 {
        let wd = w0 * (1.0 - zeta * zeta).sqrt();
        1.0 - (-zeta * w0 * t).exp() * ((wd * t).cos() + (zeta * w0 / wd) * (wd * t).sin())
    } else if (zeta - 1.0).abs() < 1e-9 {
        1.0 - (-w0 * t).exp() * (1.0 + w0 * t)
    } else {
        let s = (zeta * zeta - 1.0).sqrt();
        let r1 = -w0 * (zeta - s);
        let r2 = -w0 * (zeta + s);
        let c2 = r1 / (r2 - r1);
        let c1 = -1.0 - c2;
        1.0 + c1 * (r1 * t).exp() + c2 * (r2 * t).exp()
    };
    if cfg.overshoot_clamping {
        x.min(1.0)
    } else {
        x
    }
}

/// Remotion's `spring({frame, fps, config, from, to, delay})`.
pub fn spring(frame: f64, fps: f64, cfg: SpringConfig, from: f64, to: f64) -> f64 {
    from + (to - from) * spring_value(frame / fps, cfg)
}

/// Progress of child `i` of `n` in a lagged group (manim's `lag_ratio`).
pub fn lagged_alpha(alpha: f64, i: usize, n: usize, lag_ratio: f64) -> f64 {
    if n <= 1 || lag_ratio <= 0.0 {
        return clamp01(alpha);
    }
    let full = 1.0 + (n - 1) as f64 * lag_ratio;
    let start = i as f64 * lag_ratio / full;
    let end = start + 1.0 / full;
    clamp01((alpha - start) / (end - start))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints() {
        for r in [
            Rate::Linear,
            Rate::Smooth,
            Rate::Smoother,
            Rate::RushInto,
            Rate::RushFrom,
            Rate::InOutCubic,
            Rate::OutBounce,
            Rate::OutElastic,
            Rate::InOutBack,
            Rate::Bezier(0.42, 0.0, 0.58, 1.0),
            Rate::DoubleSmooth,
        ] {
            assert!(r.eval(0.0).abs() < 1e-6, "{r:?} at 0");
            assert!((r.eval(1.0) - 1.0).abs() < 1e-6, "{r:?} at 1");
        }
        assert!(Rate::ThereAndBack.eval(1.0).abs() < 1e-6);
        assert!((Rate::ThereAndBack.eval(0.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn interpolate_like_remotion() {
        assert_eq!(interpolate(15.0, &[0.0, 30.0], &[0.0, 1.0]), 0.5);
        assert_eq!(interpolate(45.0, &[0.0, 30.0], &[0.0, 1.0]), 1.0);
        let v = interpolate_with(
            45.0,
            &[0.0, 30.0],
            &[0.0, 1.0],
            &Rate::Linear,
            Extrapolate::Extend,
            Extrapolate::Extend,
        );
        assert_eq!(v, 1.5);
        assert_eq!(
            interpolate(75.0, &[0.0, 50.0, 100.0], &[0.0, 1.0, 0.0]),
            0.5
        );
    }

    #[test]
    fn springs_settle() {
        let cfg = SpringConfig::default();
        assert!((spring_value(5.0, cfg) - 1.0).abs() < 1e-3);
        assert!(spring(0.0, 30.0, cfg, 0.0, 1.0).abs() < 1e-9);
        let overdamped = SpringConfig::new(1.0, 40.0, 100.0);
        assert!((spring_value(10.0, overdamped) - 1.0).abs() < 1e-3);
        assert!((Rate::Spring(cfg).eval(1.0) - 1.0).abs() < 2e-3);
    }

    #[test]
    fn lag() {
        assert_eq!(lagged_alpha(0.0, 2, 3, 0.5), 0.0);
        assert_eq!(lagged_alpha(1.0, 2, 3, 0.5), 1.0);
        assert!(lagged_alpha(0.3, 0, 3, 0.5) > lagged_alpha(0.3, 2, 3, 0.5));
        assert!(Rate::from_name("ease_in_out_cubic").is_some());
    }
}
