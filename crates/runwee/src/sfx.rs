//! Procedurally synthesised sound effects (Remotion's `@remotion/sfx`, but generated
//! in code: no downloads, deterministic, royalty free) plus simple music beds.

use crate::audio::{AudioData, SAMPLE_RATE};
use crate::math::Rng;
use std::f32::consts::PI;
use std::sync::Arc;

const SR: f32 = SAMPLE_RATE as f32;

fn render(secs: f32, mut f: impl FnMut(f32) -> f32) -> Arc<AudioData> {
    let n = (secs * SR) as usize;
    let mono: Vec<f32> = (0..n).map(|i| f(i as f32 / SR)).collect();
    Arc::new(AudioData::from_mono(&mono, SAMPLE_RATE))
}

fn env_ad(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack {
        t / attack
    } else {
        (-(t - attack) / decay).exp()
    }
}

/// One-pole low-pass filter state.
struct Lp(f32);
impl Lp {
    fn run(&mut self, x: f32, cutoff: f32) -> f32 {
        let a = 1.0 - (-2.0 * PI * cutoff / SR).exp();
        self.0 += a * (x - self.0);
        self.0
    }
}

/// Plain sine tone with soft attack/release.
pub fn tone(freq: f32, secs: f32, gain: f32) -> Arc<AudioData> {
    render(secs, |t| {
        let e = (t / 0.01).min(1.0) * ((secs - t) / 0.03).clamp(0.0, 1.0);
        gain * e * (2.0 * PI * freq * t).sin()
    })
}

/// Frequency of a MIDI note.
pub fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

/// UI click.
pub fn click() -> Arc<AudioData> {
    let mut rng = Rng::new(3);
    render(0.05, move |t| {
        let n = rng.range(-1.0, 1.0) as f32 * (-t / 0.002).exp();
        let s = (2.0 * PI * 3200.0 * t).sin() * (-t / 0.012).exp();
        0.5 * n + 0.45 * s
    })
}

/// Mouse click (two-stage).
pub fn mouse_click() -> Arc<AudioData> {
    let a = click();
    let mut d = (*a).clone();
    let gap = AudioData::silence(0.045);
    d = d.concat(&gap).concat(&click());
    Arc::new(d)
}

/// Bubble pop.
pub fn pop() -> Arc<AudioData> {
    render(0.12, |t| {
        let f = 350.0 + 900.0 * (t / 0.06).min(1.0);
        0.7 * env_ad(t, 0.002, 0.03) * (2.0 * PI * f * t).sin()
    })
}

/// Wooden tick (metronome / counter).
pub fn tick() -> Arc<AudioData> {
    render(0.08, |t| {
        0.6 * env_ad(t, 0.001, 0.015) * (2.0 * PI * 1250.0 * t).sin()
    })
}

/// Bell "ding".
pub fn ding() -> Arc<AudioData> {
    let f = 1046.5; // C6
    render(1.6, move |t| {
        let e = env_ad(t, 0.003, 0.45);
        0.45 * e
            * ((2.0 * PI * f * t).sin()
                + 0.5 * (2.0 * PI * f * 2.76 * t).sin() * (-t / 0.2).exp()
                + 0.25 * (2.0 * PI * f * 5.4 * t).sin() * (-t / 0.1).exp())
    })
}

/// Rising three-note success chime.
pub fn success() -> Arc<AudioData> {
    let notes = [72.0, 76.0, 79.0, 84.0];
    render(1.4, move |t| {
        let mut s = 0.0;
        for (i, n) in notes.iter().enumerate() {
            let t0 = i as f32 * 0.09;
            if t >= t0 {
                let tt = t - t0;
                let f = midi(*n);
                s += env_ad(tt, 0.004, 0.35)
                    * ((2.0 * PI * f * tt).sin() + 0.3 * (4.0 * PI * f * tt).sin());
            }
        }
        0.25 * s
    })
}

/// Wrong-answer buzz.
pub fn error() -> Arc<AudioData> {
    render(0.5, |t| {
        let on = if t < 0.18 || (0.24..0.42).contains(&t) {
            1.0
        } else {
            0.0
        };
        let sq = if (2.0 * PI * 140.0 * t).sin() >= 0.0 {
            1.0
        } else {
            -1.0
        };
        0.25 * on * sq * (1.0 - (t / 0.5))
    })
}

/// Air whoosh (band-swept noise).
pub fn whoosh() -> Arc<AudioData> {
    let mut rng = Rng::new(11);
    let mut lp1 = Lp(0.0);
    let mut lp2 = Lp(0.0);
    let dur = 0.7;
    render(dur, move |t| {
        let u = t / dur;
        let env = (PI * u).sin().powf(1.6);
        let cutoff = 300.0 + 4500.0 * (PI * u).sin();
        let n = rng.range(-1.0, 1.0) as f32;
        let lo = lp1.run(n, cutoff);
        let band = lo - lp2.run(lo, cutoff * 0.25);
        1.6 * env * band
    })
}

/// Fast whip crack.
pub fn whip() -> Arc<AudioData> {
    let mut rng = Rng::new(5);
    let mut lp = Lp(0.0);
    render(0.35, move |t| {
        let n = rng.range(-1.0, 1.0) as f32;
        let cutoff = 9000.0 * (1.0 - t / 0.35) + 500.0;
        let x = n - lp.run(n, cutoff * 0.2);
        let crack = if t < 0.25 {
            0.0
        } else {
            env_ad(t - 0.25, 0.001, 0.02) * rng.range(-1.0, 1.0) as f32
        };
        0.6 * x
            * (t / 0.25).min(1.0).powi(2)
            * (if t < 0.25 {
                1.0
            } else {
                (-(t - 0.25) / 0.03).exp()
            })
            + 0.8 * crack
    })
}

/// Toggle switch.
pub fn switch() -> Arc<AudioData> {
    let a = tick();
    Arc::new(
        (*a).clone()
            .concat(&AudioData::silence(0.06))
            .concat(&click()),
    )
}

/// Paper page turn.
pub fn page_turn() -> Arc<AudioData> {
    let mut rng = Rng::new(17);
    let mut lp = Lp(0.0);
    render(0.55, move |t| {
        let e = (PI * (t / 0.55)).sin().powi(2) * (1.0 + 0.6 * (40.0 * t).sin());
        let n = rng.range(-1.0, 1.0) as f32;
        0.5 * e * (n - lp.run(n, 1200.0))
    })
}

/// Camera shutter.
pub fn shutter() -> Arc<AudioData> {
    let mut rng = Rng::new(23);
    render(0.25, move |t| {
        let a = env_ad(t, 0.001, 0.012);
        let b = if t > 0.09 {
            env_ad(t - 0.09, 0.001, 0.02)
        } else {
            0.0
        };
        0.7 * (a + b) * rng.range(-1.0, 1.0) as f32
    })
}

/// Keyboard typing for `secs` seconds.
pub fn typing(secs: f32) -> Arc<AudioData> {
    let mut rng = Rng::new(31);
    let mut next = 0.0f32;
    let mut last = -1.0f32;
    let mut pitch = 2500.0f32;
    let mut noise = Rng::new(2);
    render(secs, move |t| {
        if t >= next {
            last = t;
            next = t + 0.06 + rng.range(0.0, 0.11) as f32;
            pitch = 1800.0 + rng.range(0.0, 1500.0) as f32;
        }
        let tt = t - last;
        0.4 * env_ad(tt, 0.0005, 0.01)
            * ((2.0 * PI * pitch * tt).sin() * 0.6 + noise.range(-0.4, 0.4) as f32)
    })
}

/// Deep cinematic boom.
pub fn boom() -> Arc<AudioData> {
    render(1.8, |t| {
        let f = 40.0 + 80.0 * (-t / 0.08).exp();
        let phase = 2.0 * PI * (40.0 * t + 80.0 * 0.08 * (1.0 - (-t / 0.08).exp()));
        let _ = f;
        0.9 * env_ad(t, 0.004, 0.5) * phase.sin()
    })
}

/// Upward tonal swoosh (good for transitions in).
pub fn swoosh_up() -> Arc<AudioData> {
    render(0.45, |t| {
        let u = t / 0.45;
        let phase = 2.0 * PI * (300.0 * t + 1200.0 * t * u / 2.0);
        0.35 * (PI * u).sin() * phase.sin()
    })
}

/// Downward tonal swoosh.
pub fn swoosh_down() -> Arc<AudioData> {
    render(0.45, |t| {
        let u = t / 0.45;
        let phase = 2.0 * PI * (1500.0 * t - 1200.0 * t * u / 2.0);
        0.35 * (PI * u).sin() * phase.sin()
    })
}

/// Soft notification "blip-blip".
pub fn notification() -> Arc<AudioData> {
    render(0.4, |t| {
        let f = if t < 0.12 { midi(81.0) } else { midi(88.0) };
        let tt = if t < 0.12 { t } else { t - 0.12 };
        0.35 * env_ad(tt, 0.003, 0.08) * (2.0 * PI * f * tt).sin()
    })
}

/// Look up an effect by name (for scripts): click, mouse_click, pop, tick, ding, success,
/// error, whoosh, whip, switch, page_turn, shutter, typing, boom, swoosh_up, swoosh_down, notification.
/// Vinyl record scratch (a pitch-swept noisy rasp, forward then back).
pub fn record_scratch() -> Arc<AudioData> {
    let mut rng = Rng::new(41);
    let mut lp = Lp(0.0);
    let mut phase = 0.0f32;
    render(0.45, move |t| {
        // Two strokes: fast forward pull, then a slower pull back.
        let (u, dir) = if t < 0.18 {
            (t / 0.18, 1.0)
        } else {
            ((t - 0.18) / 0.27, -1.0)
        };
        let speed = (PI * u).sin() * dir;
        phase += (300.0 + 900.0 * speed.abs()) / SR;
        let tone = (2.0 * PI * phase).sin() * 0.5;
        let noise = lp.run(rng.range(-1.0, 1.0) as f32, 1200.0 + 2500.0 * speed.abs());
        let e = env_ad(t, 0.005, 0.5) * (1.0 - (t / 0.45).powi(4));
        0.8 * e * (noise * 1.6 + tone * speed.abs())
    })
}

/// Old mechanical camera: shutter clack followed by a film-advance whir.
pub fn shutter_old() -> Arc<AudioData> {
    let mut rng = Rng::new(43);
    let mut lp = Lp(0.0);
    render(0.9, move |t| {
        let clack = env_ad(t, 0.001, 0.02)
            + 0.6 * env_ad((t - 0.07).max(0.0), 0.001, 0.03) * (t > 0.07) as u8 as f32;
        let n = rng.range(-1.0, 1.0) as f32;
        let whir = if t > 0.25 {
            let u = t - 0.25;
            let ratchet = ((u * 38.0).fract() < 0.25) as u8 as f32;
            0.35 * ratchet * lp.run(n, 3000.0) * ((0.65 - u) / 0.1).clamp(0.0, 1.0)
        } else {
            0.0
        };
        0.8 * clack * n + whir
    })
}

/// "Sad trombone" fail: four descending wah notes.
pub fn fail() -> Arc<AudioData> {
    let notes = [midi(58.0), midi(57.0), midi(56.0), midi(55.0)];
    let mut phase = 0.0f32;
    render(1.9, move |t| {
        let i = ((t / 0.38) as usize).min(3);
        let local = t - i as f32 * 0.38;
        let len = if i == 3 { 0.75 } else { 0.36 };
        let vib = if i == 3 {
            1.0 + 0.012 * (2.0 * PI * 6.0 * local).sin()
        } else {
            1.0
        };
        phase += notes[i] * vib / SR;
        let x = 2.0 * PI * phase;
        // Brassy: a few odd/even harmonics, with a "wah" brightness envelope.
        let wah = 0.4 + 0.6 * (local / 0.12).min(1.0);
        let v = x.sin()
            + 0.5 * wah * (2.0 * x).sin()
            + 0.3 * wah * (3.0 * x).sin()
            + 0.15 * wah * (4.0 * x).sin();
        let e = (local / 0.02).min(1.0) * ((len - local) / 0.05).clamp(0.0, 1.0);
        0.22 * e * v
    })
}

/// Bright rising "ta-da" arpeggio (a correct answer, a reveal).
pub fn tada() -> Arc<AudioData> {
    let notes = [midi(72.0), midi(76.0), midi(79.0), midi(84.0)];
    render(1.3, move |t| {
        let mut v = 0.0;
        for (i, f) in notes.iter().enumerate() {
            let start = i as f32 * 0.07;
            if t >= start {
                let u = t - start;
                let e = env_ad(u, 0.005, if i == 3 { 0.6 } else { 0.25 });
                v += e * ((2.0 * PI * f * u).sin() + 0.3 * (4.0 * PI * f * u).sin());
            }
        }
        0.18 * v
    })
}

/// Arcade coin pickup (two quick square-ish blips).
pub fn coin() -> Arc<AudioData> {
    render(0.35, |t| {
        let f = if t < 0.07 { midi(83.0) } else { midi(88.0) };
        let e = if t < 0.07 {
            1.0
        } else {
            env_ad(t - 0.07, 0.002, 0.12)
        };
        0.18 * e * (2.0 * PI * f * t).sin().signum() * 0.6
    })
}

/// Sci-fi laser zap (fast downward pitch sweep).
pub fn laser() -> Arc<AudioData> {
    let mut phase = 0.0f32;
    render(0.3, move |t| {
        let f = 1800.0 * (-t * 12.0).exp() + 200.0;
        phase += f / SR;
        0.3 * env_ad(t, 0.002, 0.12) * (2.0 * PI * phase).sin()
    })
}

/// Magical sparkle: a shimmer of random high bell partials.
pub fn sparkle() -> Arc<AudioData> {
    let mut rng = Rng::new(47);
    let bells: Vec<(f32, f32)> = (0..14)
        .map(|i| (i as f32 * 0.045, 1800.0 + rng.range(0.0, 2600.0) as f32))
        .collect();
    render(1.1, move |t| {
        let mut v = 0.0;
        for (start, f) in &bells {
            if t >= *start {
                let u = t - start;
                v += env_ad(u, 0.002, 0.18) * (2.0 * PI * f * u).sin();
            }
        }
        0.08 * v
    })
}

/// Level-up / power-up: rising square-wave run.
pub fn level_up() -> Arc<AudioData> {
    render(0.7, |t| {
        let step = ((t / 0.06) as usize).min(9);
        let f = midi(60.0 + [0.0, 4.0, 7.0, 12.0, 16.0, 19.0, 24.0, 28.0, 31.0, 36.0][step]);
        let e = ((0.7 - t) / 0.15).clamp(0.0, 1.0);
        0.12 * e * (2.0 * PI * f * t).sin().signum()
    })
}

/// Countdown beep (`final_beep` = the longer, higher last beep).
pub fn countdown_beep(final_beep: bool) -> Arc<AudioData> {
    let (f, len) = if final_beep {
        (1320.0, 0.6)
    } else {
        (880.0, 0.15)
    };
    render(len, move |t| {
        let e = (t / 0.005).min(1.0) * ((len - t) / 0.02).clamp(0.0, 1.0);
        0.25 * e * (2.0 * PI * f * t).sin()
    })
}

/// Heartbeat "lub-dub" (one beat).
pub fn heartbeat() -> Arc<AudioData> {
    render(0.8, |t| {
        let thump = |u: f32, f: f32| {
            if u >= 0.0 {
                env_ad(u, 0.004, 0.06) * (2.0 * PI * f * u).sin()
            } else {
                0.0
            }
        };
        0.7 * (thump(t, 55.0) + 0.7 * thump(t - 0.2, 48.0))
    })
}

/// Snare drum roll for `secs` seconds ending with an accent.
pub fn drum_roll(secs: f32) -> Arc<AudioData> {
    let mut rng = Rng::new(53);
    let mut lp = Lp(0.0);
    render(secs + 0.4, move |t| {
        let n = rng.range(-1.0, 1.0) as f32;
        if t < secs {
            let hit = (t * 28.0).fract();
            let swell = 0.4 + 0.6 * t / secs;
            0.35 * swell * (-hit * 9.0).exp() * lp.run(n, 5000.0)
        } else {
            let u = t - secs;
            0.9 * env_ad(u, 0.001, 0.12) * (n * 0.7 + 0.3 * (2.0 * PI * 180.0 * u).sin())
        }
    })
}

pub fn by_name(name: &str) -> Option<Arc<AudioData>> {
    Some(
        match name.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "click" => click(),
            "mouse_click" => mouse_click(),
            "pop" => pop(),
            "tick" => tick(),
            "ding" | "bell" => ding(),
            "success" | "correct" | "chime" => success(),
            "error" | "wrong" | "buzz" => error(),
            "whoosh" => whoosh(),
            "whip" => whip(),
            "switch" | "ui_switch" => switch(),
            "page_turn" => page_turn(),
            "shutter" => shutter(),
            "typing" => typing(1.5),
            "boom" => boom(),
            "swoosh_up" | "rise" => swoosh_up(),
            "swoosh_down" | "fall" => swoosh_down(),
            "notification" => notification(),
            "record_scratch" | "scratch" => record_scratch(),
            "shutter_old" => shutter_old(),
            "shutter_modern" => shutter(),
            "fail" | "sad_trombone" | "price_is_right_fail" => fail(),
            "tada" | "ta_da" | "reveal" => tada(),
            "coin" => coin(),
            "laser" | "zap" => laser(),
            "sparkle" | "magic" => sparkle(),
            "level_up" | "power_up" => level_up(),
            "beep" | "countdown" => countdown_beep(false),
            "beep_final" | "countdown_final" => countdown_beep(true),
            "heartbeat" => heartbeat(),
            "drum_roll" => drum_roll(1.5),
            "vine_boom" | "dramatic_boom" => boom(),
            _ => return None,
        },
    )
}

/// Calm ambient pad (I–V–vi–IV) as a background bed of `secs` seconds.
pub fn ambient_pad(secs: f32, root_midi: f32, gain: f32) -> Arc<AudioData> {
    let chords: [[f32; 3]; 4] = [
        [0.0, 4.0, 7.0],
        [7.0, 11.0, 14.0],
        [9.0, 12.0, 16.0],
        [5.0, 9.0, 12.0],
    ];
    let bar = 4.0f32;
    render(secs, move |t| {
        let k = ((t / bar) as usize) % 4;
        let local = t % bar;
        let xf = (local / 0.6).min(1.0) * ((bar - local) / 0.6).min(1.0);
        let mut s = 0.0;
        for n in chords[k] {
            let f = midi(root_midi + n);
            s += (2.0 * PI * f * t).sin()
                + 0.3 * (2.0 * PI * f * 2.0 * t + 0.4 * (0.5 * t).sin()).sin();
        }
        let fade = (t / 2.0).min(1.0) * ((secs - t) / 2.0).clamp(0.0, 1.0);
        gain * 0.12 * s * (0.6 + 0.4 * xf) * fade
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_effects_render_and_are_bounded() {
        for name in [
            "click",
            "mouse_click",
            "pop",
            "tick",
            "ding",
            "success",
            "error",
            "whoosh",
            "whip",
            "switch",
            "page_turn",
            "shutter",
            "typing",
            "boom",
            "swoosh_up",
            "swoosh_down",
            "notification",
            "record_scratch",
            "shutter_old",
            "fail",
            "tada",
            "coin",
            "laser",
            "sparkle",
            "level_up",
            "beep",
            "beep_final",
            "heartbeat",
            "drum_roll",
        ] {
            let a = by_name(name).unwrap();
            assert!(a.duration() > 0.03, "{name}");
            let peak = a.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.01 && peak < 2.0, "{name} peak {peak}");
            assert!(a.samples.iter().all(|s| s.is_finite()), "{name}");
        }
        assert!((ambient_pad(3.0, 48.0, 1.0).duration() - 3.0).abs() < 1e-3);
    }
}
