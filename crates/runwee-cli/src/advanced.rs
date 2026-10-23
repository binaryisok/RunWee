//! Script bindings for the advanced animation & graphics modules (phases A1–A5):
//! springs, wiggle, palettes, smart morphs, shatter / dust, sketchy rendering,
//! noise, generative art, particles, kinetic text, cinematic camera moves,
//! Verlet ropes, boids and the synth drum machine.

use super::{col, f, points, vec_of, Shared, N};
use rhai::{Engine, Map};
use runwee::prelude::*;
use std::sync::Arc;

/// A synthesised beat that has been placed on the scene's audio track.
#[derive(Clone)]
pub struct Beat {
    hits: Arc<Vec<Hit>>,
    duration: f64,
}

fn voice_of(name: &str) -> Option<Voice> {
    Some(match name.to_ascii_lowercase().as_str() {
        "kick" | "bd" => Voice::Kick,
        "snare" | "sd" => Voice::Snare,
        "hat" | "hh" | "hihat" => Voice::Hat,
        "clap" | "cp" => Voice::Clap,
        "bass" => Voice::Bass,
        _ => return None,
    })
}

fn spring_of(name: &str) -> Spring {
    match name {
        "bouncy" => Spring::bouncy(),
        "snappy" => Spring::snappy(),
        "wobbly" => Spring::wobbly(),
        _ => Spring::gentle(),
    }
}

fn emitter_of(name: &str, p: Vec2) -> Emitter {
    match name {
        "fire" => Emitter::fire(p),
        "sparks" => Emitter::sparks(p),
        "smoke" => Emitter::smoke(p),
        "confetti" => Emitter::confetti(p, std::f64::consts::FRAC_PI_2),
        "snow" => Emitter::snow(v2(-7.5, 4.3), v2(7.5, 4.5)),
        _ => Emitter::firework(p, 0.0, YELLOW, 1),
    }
}

fn rect_full() -> runwee::path::Rect {
    runwee::path::Rect::from_center(Vec2::ZERO, 14.3, 8.1)
}

pub(crate) fn register_advanced(engine: &mut Engine, st: Shared) {
    // ── motion helpers ──
    engine.register_fn("wiggle_at", |t: N, freq: N, amp: N, seed: N| {
        wiggle_value(f(&t), f(&freq), f(&amp), f(&seed) as u64)
    });
    engine.register_fn("wiggle_vec", |t: N, freq: N, amp: N, seed: N| {
        wiggle_vec(f(&t), f(&freq), f(&amp), f(&seed) as u64)
    });
    engine.register_fn("spring_value", |name: &str, t: N| {
        spring_of(name).progress(f(&t))
    });
    engine.register_fn("palette", |name: &str, t: N| palette(name, f(&t)));
    engine.register_fn("spring_shift", |o: Obj, by: Vec2, name: &str| {
        spring_shift(o, by, spring_of(name))
    });
    engine.register_fn("spring_pop_in", |o: Obj, name: &str| {
        spring_pop_in(o, spring_of(name))
    });
    engine.register_fn("wiggle_anim", |o: Obj, freq: N, amp: N, dur: N| {
        wiggle_anim(o, f(&freq), f(&amp), f(&dur), 7)
    });
    // ── shape & morph ──
    engine.register_fn("smart_morph", |o: Obj, m: Mobject| smart_morph(o, m));
    engine.register_fn("twist_morph", |o: Obj, m: Mobject, tw: N| {
        twist_morph(o, m, f(&tw))
    });
    engine.register_fn("shatter", |o: Obj, pieces: N| {
        shatter(
            o,
            ShatterOpts {
                pieces: f(&pieces) as usize,
                ..Default::default()
            },
        )
    });
    engine.register_fn("assemble", |o: Obj, pieces: N| {
        assemble(
            o,
            ShatterOpts {
                pieces: f(&pieces) as usize,
                ..Default::default()
            },
        )
    });
    engine.register_fn("dissolve", |o: Obj, n: N| dissolve(o, f(&n) as usize, 3));
    engine.register_fn("materialize", |o: Obj, n: N| {
        materialize(o, f(&n) as usize, 3)
    });
    engine.register_fn("neon_write", |o: Obj, c: N| neon_write(o, col(&c)));
    engine.register_fn("jelly", |o: Obj, amp: N, wobbles: N| {
        jelly(o, f(&amp), f(&wobbles))
    });
    engine.register_fn("sketchy", |m: Mobject, rough: N, seed: N| {
        sketchy(
            &m,
            SketchOpts::default()
                .roughness(f(&rough))
                .seed(f(&seed) as u64),
        )
    });
    engine.register_fn("boil", |m: Mobject, t: N, rough: N| {
        boil(&m, f(&t), 8.0, SketchOpts::default().roughness(f(&rough)))
    });
    // ── noise ──
    engine.register_fn("perlin", |p: N, seed: N| {
        perlin2(vec_of(&p), f(&seed) as u64)
    });
    engine.register_fn("fbm", |p: N, octaves: N, seed: N| {
        fbm2(vec_of(&p), f(&octaves) as u32, 2.0, 0.5, f(&seed) as u64)
    });
    engine.register_fn("ridged", |p: N, octaves: N, seed: N| {
        ridged2(vec_of(&p), f(&octaves) as u32, f(&seed) as u64)
    });
    engine.register_fn("curl", |p: N, t: N, seed: N| {
        curl2(vec_of(&p), f(&t), f(&seed) as u64)
    });
    engine.register_fn("domain_warp", |p: N, strength: N, t: N, seed: N| {
        domain_warp2(vec_of(&p), f(&strength), f(&t), f(&seed) as u64)
    });
    // ── generative art ──
    engine.register_fn("warp_image", |scale: N, strength: N, t: N, pal: &str| {
        let (sc, k, t) = (f(&scale), f(&strength), f(&t));
        let pal = pal.to_string();
        field_image(rect_full(), 320, move |p| {
            palette(&pal, domain_warp2(p * sc, k, t, 1) * 0.5 + 0.5)
        })
    });
    engine.register_fn("lsystem", |name: &str, n: N, size: N, trunk: N, tip: N| {
        LSystem::preset(name).mobject(f(&n) as usize, 90.0, f(&size), col(&trunk), col(&tip))
    });
    engine.register_fn("truchet", |cell: N, c: N, seed: N| {
        truchet(rect_full(), f(&cell), col(&c), 3.0, f(&seed) as u64)
    });
    engine.register_fn("voronoi_mosaic", |n: N, pal: &str, seed: N| {
        voronoi_mosaic(rect_full(), f(&n) as usize, pal, f(&seed) as u64)
    });
    engine.register_fn("phyllotaxis", |n: N, c: N, size: N, pal: &str| {
        let pts = phyllotaxis(f(&n) as usize, f(&c));
        let len = pts.len().max(1) as f64;
        group(
            pts.iter()
                .enumerate()
                .map(|(i, p)| dot_r(*p, f(&size), palette(pal, i as f64 / len)))
                .collect::<Vec<_>>(),
        )
    });
    engine.register_fn("clifford", |a: N, b: N, c: N, d: N, scale: N| {
        let pts = clifford(f(&a), f(&b), f(&c), f(&d), 120_000);
        density_image(
            &pts,
            rect_full(),
            480,
            &[BLACK, BLUE, TEAL, YELLOW, WHITE],
            f(&scale),
        )
    });
    // ── kinetic text ──
    engine.register_fn("glyph_wave", |m: Mobject, t: N, amp: N| {
        glyph_wave(&m, f(&t), f(&amp), 3.0, 4.0)
    });
    engine.register_fn("glitch", |m: Mobject, t: N, k: N| {
        glitch(&m, f(&t), f(&k), 5)
    });
    engine.register_fn("scramble", |s: &str, t: N, size: N, c: N| {
        scramble_text(s, f(&t), f(&size), col(&c), GRAY, 9)
    });
    engine.register_fn("glyph_fly_in", |o: Obj, by: Vec2, spin: N| {
        glyph_fly_in(o, by, f(&spin), 0.3, 0.6)
    });
    engine.register_fn("warp_starfield", |t: N, speed: N, tint: N| {
        warp_starfield(f(&t), 500, f(&speed), Vec2::ZERO, 8.0, col(&tint), 4)
    });
    engine.register_fn("retro_sun", |c: Vec2, r: N, t: N| {
        retro_sun(c, f(&r), rgb8(255, 220, 90), rgb8(255, 50, 150), f(&t))
    });
    engine.register_fn("rope_line", |pts: rhai::Array, c: N, w: N| {
        rope_mobject(&points(&pts), col(&c), f(&w) as f32)
    });

    // ── scene-level: particles, camera moves, trails, simulations, beats ──
    let st1 = st.clone();
    engine.register_fn("particles", move |name: &str, p: N| {
        let e = emitter_of(name, vec_of(&p));
        let mut g = st1.lock().unwrap();
        particle_system(&mut g.cur, e)
    });
    let st1 = st.clone();
    engine.register_fn("camera_shake", move |dur: N, trauma: N| {
        st1.lock()
            .unwrap()
            .cur
            .camera_shake(f(&dur), f(&trauma), 11)
    });
    let st1 = st.clone();
    engine.register_fn("camera_follow", move |o: Obj, until: N, lag: N| {
        st1.lock()
            .unwrap()
            .cur
            .camera_follow(o, f(&until), f(&lag), 0.6, None)
    });
    let st1 = st.clone();
    engine.register_fn("echo_trail", move |o: Obj, copies: N, spacing: N| {
        let mut g = st1.lock().unwrap();
        echo_trail(&mut g.cur, o, f(&copies) as usize, f(&spacing), 0.5)
    });
    let st1 = st.clone();
    engine.register_fn("motion_trail", move |o: Obj, len: N, c: N| {
        let mut g = st1.lock().unwrap();
        motion_trail(&mut g.cur, o, f(&len), 6.0, col(&c))
    });
    let st1 = st.clone();
    // A Verlet rope hanging from `a`, simulated for `dur` seconds and drawn live.
    engine.register_fn("rope", move |a: Vec2, b: Vec2, c: N, dur: N| {
        let (baked, _) = Verlet::rope(a, b, 24).bake(f(&dur), 60.0, 4);
        let c = col(&c);
        let mut g = st1.lock().unwrap();
        g.cur.always_redraw(move |ctx| {
            let p = baked.at(ctx.elapsed);
            group(vec![
                rope_mobject(&p, c, 4.0),
                circle(0.15)
                    .fill(c, 1.0)
                    .no_stroke()
                    .move_to(*p.last().unwrap_or(&Vec2::ZERO)),
            ])
        })
    });
    let st1 = st.clone();
    engine.register_fn("boids", move |count: N, dur: N| {
        let flock = bake_boids(
            BoidOpts {
                count: f(&count) as usize,
                ..Default::default()
            },
            f(&dur),
            30.0,
            None,
        );
        let mut g = st1.lock().unwrap();
        g.cur.always_redraw(move |ctx| {
            let t = ctx.elapsed;
            boids_mobject(
                &flock.at(t),
                &flock.velocity(t),
                0.14,
                &[rgb8(120, 220, 255), rgb8(255, 140, 200)],
            )
        })
    });
    engine.register_type_with_name::<Beat>("Beat");
    let st1 = st.clone();
    // beat(bpm, bars, #{kick: "X...", snare: "....X...", ...}) — synthesises the
    // pattern, puts it on the audio track at the current time and returns a Beat.
    engine.register_fn("beat", move |bpm: N, bars: N, tracks: Map| {
        let mut dm = DrumMachine::new(f(&bpm), f(&bars).max(1.0) as usize);
        for (k, v) in tracks.iter() {
            if let Some(voice) = voice_of(k) {
                dm = dm.track(voice, &v.to_string());
            }
        }
        let audio = dm.render();
        let mut g = st1.lock().unwrap();
        let t0 = g.cur.time();
        g.cur.audio.add(AudioClip::new(audio, t0));
        Beat {
            hits: Arc::new(dm.hits()),
            duration: dm.duration(),
        }
    });
    engine.register_get("duration", |b: &mut Beat| b.duration);
    // beat_env(beat, "kick", t, decay): 1.0 on each hit of that voice, decaying after.
    engine.register_fn("beat_env", |b: Beat, voice: &str, t: N, decay: N| {
        voice_of(voice)
            .map(|v| hit_envelope(&b.hits, v, f(&t), f(&decay).max(1e-3)))
            .unwrap_or(0.0)
    });
}
