//! Script bindings for the STEM visualisation modules (phases S1–S4):
//! Fourier epicycles, linear maps, Taylor series, the unit circle, domain
//! colouring, number-theory art, ripple tanks, Doppler, ray optics, Kepler
//! orbits, hydrogen orbitals, reaction profiles, titration, SIR epidemics,
//! sorting traces, path-finding, BSTs, neural nets, Galton boards and automata.

use super::{col, colors, f, floats, points, vec_of, N};
use rhai::{Array, Dynamic, Engine};
use runwee::path::Rect;
use runwee::prelude::algoviz::{self as av, Search};
use runwee::prelude::chembio as cb;
use runwee::prelude::mathviz::{self as mv, cx, Cx};
use runwee::prelude::physviz::{self as pv, Optic};
use runwee::prelude::*;
use std::sync::Arc;

/// Fourier coefficients of a traced outline.
#[derive(Clone)]
pub struct Fourier(Arc<Vec<mv::Epicycle>>);
/// A recorded sorting run.
#[derive(Clone)]
pub struct SortRun(Arc<Vec<av::SortFrame>>);
/// A grid plus one search algorithm's trace over it.
#[derive(Clone)]
pub struct PathRun(Arc<(av::Grid, av::SearchTrace)>);
/// A laid-out binary search tree.
#[derive(Clone)]
pub struct Bst(Arc<Vec<(i64, Vec2, Option<usize>)>>);
/// A random feed-forward network.
#[derive(Clone)]
pub struct Net(Arc<av::NeuralNet>);
/// Pre-rolled Galton board paths.
#[derive(Clone)]
pub struct Galton(Arc<Vec<Vec<bool>>>);
/// A baked agent-based SIR epidemic.
#[derive(Clone)]
pub struct Epi(Arc<cb::Epidemic>);
/// Pre-computed Game-of-Life generations.
#[derive(Clone)]
pub struct Life(Arc<Vec<Vec<Vec<bool>>>>);

fn cpx_fn(name: &str) -> fn(Cx) -> Cx {
    match name {
        "1/z" => |z| cx(1.0, 0.0) / z,
        "sin" => |z| z.sin(),
        "exp" => |z| z.exp(),
        "ln" | "log" => |z| z.ln(),
        "z^2" => |z| z.powi(2),
        "rational" => |z| (z.powi(2) - cx(1.0, 0.0)) / (z.powi(2) + cx(1.0, 0.0)),
        _ => |z| z.powi(3) - cx(1.0, 0.0),
    }
}

fn search_of(name: &str) -> Search {
    match name.to_ascii_lowercase().as_str() {
        "dfs" => Search::Dfs,
        "dijkstra" => Search::Dijkstra,
        "astar" | "a*" => Search::AStar,
        _ => Search::Bfs,
    }
}

fn spectrum_rays(origin: Vec2, dir: Vec2, optics: &[Optic], frac: f64, len: f64) -> Vec<Mobject> {
    (0..7)
        .map(|k| {
            let nm = 420.0 + 40.0 * k as f64;
            let pts = pv::trace_ray(origin, dir, nm, optics, 6, len);
            pv::ray_mobject(&pts, frac, pv::wavelength_color(nm), 2.5)
        })
        .collect()
}

pub(crate) fn register_stem(engine: &mut Engine) {
    // ── maths ──
    engine.register_type_with_name::<Fourier>("Fourier");
    engine.register_fn("fourier", |m: Mobject, terms: N| {
        let pts = mv::outline_points(&m, 512);
        Fourier(Arc::new(mv::fourier_series(
            &pts,
            f(&terms).max(1.0) as usize,
        )))
    });
    engine.register_fn("epicycles", |fr: Fourier, t: N, center: N, c: N| {
        mv::epicycles(&fr.0, f(&t), vec_of(&center), col(&c))
    });
    engine.register_fn("epicycle_trace", |fr: Fourier, t: N, center: N, c: N| {
        mv::epicycle_trace(&fr.0, f(&t), vec_of(&center), col(&c), 3.0)
    });
    engine.register_fn("linear_map", |m: Array, t: N, range: N, eigen: bool| {
        let v = floats(&m);
        let g = |i: usize| {
            v.get(i)
                .copied()
                .unwrap_or(if i == 0 || i == 3 { 1.0 } else { 0.0 })
        };
        mv::linear_map_view([[g(0), g(1)], [g(2), g(3)]], f(&t), f(&range), eigen)
    });
    engine.register_fn("taylor", |name: &str, n: N, x: N| {
        mv::taylor_poly(name, f(&n) as usize, f(&x))
    });
    engine.register_fn("taylor_target", |name: &str, x: N| {
        mv::taylor_target(name, f(&x))
    });
    engine.register_fn(
        "unit_circle",
        |theta: N, center: N, r: N, wave_x: N, wave_scale: N| {
            mv::unit_circle_trig(
                f(&theta),
                vec_of(&center),
                f(&r),
                f(&wave_x),
                f(&wave_scale),
            )
        },
    );
    engine.register_fn(
        "domain_coloring",
        |name: &str, center: N, w: N, h: N, scale: N, px: N| {
            let (c, s, func) = (vec_of(&center), f(&scale).max(1e-6), cpx_fn(name));
            mv::domain_coloring(
                Rect::from_center(c, f(&w), f(&h)),
                f(&px).max(16.0) as u32,
                move |z| func((z - Cx::from(c)) * (1.0 / s)),
            )
        },
    );
    engine.register_fn("ulam_spiral", |n: N, cell: N, prime: N, other: N| {
        mv::ulam_spiral(f(&n) as usize, f(&cell), col(&prime), col(&other))
    });
    engine.register_fn("times_table", |n: N, k: N, radius: N, cs: Array| {
        mv::times_table(f(&n) as usize, f(&k), f(&radius), &colors(&cs))
    });
    engine.register_fn("pascal_mod", |rows: N, p: N, cell: N, cs: Array| {
        mv::pascal_mod(
            f(&rows) as usize,
            f(&p).max(2.0) as u64,
            f(&cell),
            &colors(&cs),
        )
    });
    engine.register_fn("collatz_tree", |count: N, seg: N, cs: Array| {
        let starts: Vec<u64> = (1..=f(&count).max(1.0) as u64).collect();
        mv::collatz_tree(&starts, f(&seg), -0.11, 0.21, &colors(&cs))
    });

    // ── physics ──
    engine.register_fn(
        "ripple_tank",
        |srcs: Array, center: N, w: N, h: N, wavelength: N, t: N| {
            let sources = points(&srcs).into_iter().map(pv::wave_source).collect();
            pv::ripple_tank(
                Rect::from_center(vec_of(&center), f(&w), f(&h)),
                200,
                sources,
                f(&wavelength),
                f(&t),
            )
        },
    );
    engine.register_fn("doppler", |start: N, vel: N, t: N, c: N| {
        let (p0, v) = (vec_of(&start), vec_of(&vel));
        pv::doppler_wavefronts(move |t| p0 + v * t, 1.0, 0.3, f(&t), 1.6, col(&c))
    });
    engine.register_fn("doppler_shift", |freq: N, c: N, vs: N| {
        pv::doppler_shift(f(&freq), f(&c), f(&vs))
    });
    engine.register_fn("wavelength_color", |nm: N| pv::wavelength_color(f(&nm)));
    engine.register_fn("prism_demo", |center: N, size: N, frac: N| {
        let (c, s) = (vec_of(&center), f(&size));
        let poly = vec![
            c + v2(-s, -0.6 * s),
            c + v2(s, -0.6 * s),
            c + v2(0.0, 1.1 * s),
        ];
        let optics = vec![Optic::Glass {
            poly,
            n: 1.5,
            b: 0.05,
        }];
        let mut kids = vec![pv::optics_mobject(&optics)];
        kids.extend(spectrum_rays(
            c + v2(-3.0 * s, -0.3 * s),
            v2(1.0, 0.22),
            &optics,
            f(&frac),
            6.0 * s,
        ));
        group(kids)
    });
    engine.register_fn("lens_demo", |center: N, focal: N, rays: N, frac: N| {
        let c = vec_of(&center);
        let optics = vec![Optic::ThinLens {
            center: c,
            axis: v2(1.0, 0.0),
            half_height: 1.3,
            focal: f(&focal),
        }];
        let n = f(&rays).max(2.0) as usize;
        let mut kids = vec![pv::optics_mobject(&optics)];
        for k in 0..n {
            let h = -1.1 + 2.2 * k as f64 / (n - 1) as f64;
            let pts = pv::trace_ray(c + v2(-2.5, h), v2(1.0, 0.0), 550.0, &optics, 3, 6.0);
            kids.push(pv::ray_mobject(
                &pts,
                f(&frac),
                color_gradient(&[YELLOW, ORANGE], k as f64 / n as f64),
                2.0,
            ));
        }
        group(kids)
    });
    engine.register_fn("kepler_orbit", |a: N, e: N, focus: N, phase: N, c: N| {
        let (a, e, fo) = (f(&a), f(&e), vec_of(&focus));
        let p = fo + pv::kepler_position(a, e, f(&phase));
        group(vec![
            pv::kepler_ellipse(a, e, fo, GRAY_B),
            dot_r(fo, 0.12, YELLOW),
            line(fo, p).stroke(WHITE.with_alpha(0.5), 1.5),
            dot_r(p, 0.09, col(&c)),
        ])
    });
    engine.register_fn("kepler_position", |a: N, e: N, phase: N| {
        pv::kepler_position(f(&a), f(&e), f(&phase))
    });
    engine.register_fn(
        "gravity_well",
        |masses: Array, half: N, depth: N, tilt: N, c: N| {
            let ms: Vec<(Vec2, f64)> = masses
                .iter()
                .map(|d| {
                    let a = d.clone().try_cast::<Array>().unwrap_or_default();
                    let g = |i: usize| a.get(i).map(f).unwrap_or(0.0);
                    (v2(g(0), g(1)), g(2))
                })
                .collect();
            pv::gravity_well(&ms, f(&half), 21, f(&depth), f(&tilt), col(&c))
        },
    );

    // ── chemistry & biology ──
    engine.register_fn("orbital", |n: N, l: N, m: N, center: N, size: N| {
        let (n, l, m) = (f(&n).max(1.0) as u32, f(&l).max(0.0) as u32, f(&m) as i32);
        let l = l.min(n - 1);
        let m = m.clamp(-(l as i32), l as i32);
        let ext = 2.5 * (n * n) as f64 + 3.0;
        let s = f(&size);
        cb::orbital_slice(
            Rect::from_center(vec_of(&center), s, s),
            180,
            n,
            l,
            m,
            ext,
            false,
        )
    });
    engine.register_fn("orbital_name", |n: N, l: N| {
        cb::orbital_name(f(&n) as u32, f(&l) as u32)
    });
    engine.register_fn(
        "energy_profile",
        |w: N, h: N, r: N, ts: N, p: N, cat: N, progress: N| {
            let cat = f(&cat);
            cb::energy_profile(
                f(&w),
                f(&h),
                f(&r),
                f(&ts),
                f(&p),
                (cat > 0.0).then_some(cat),
                f(&progress),
            )
        },
    );
    engine.register_fn("titration_ph", |ca: N, va: N, cb_: N, vb: N, ka: N| {
        let ka = f(&ka);
        cb::titration_ph(f(&ca), f(&va), f(&cb_), f(&vb), (ka > 0.0).then_some(ka))
    });
    engine.register_fn("indicator_color", |name: &str, ph: N| {
        cb::indicator_color(name, f(&ph))
    });
    engine.register_type_with_name::<Epi>("Epidemic");
    engine.register_fn(
        "epidemic",
        |n: N, center: N, w: N, h: N, duration: N, seed: N| {
            let bx = Rect::from_center(vec_of(&center), f(&w), f(&h));
            Epi(Arc::new(cb::Epidemic::simulate(
                f(&n) as usize,
                bx,
                0.7,
                0.16,
                0.25,
                2.2,
                f(&duration),
                f(&seed) as u64,
            )))
        },
    );
    engine.register_fn("epidemic_view", |e: Epi, t: N, r: N| {
        let (pos, st) = e.0.state(f(&t));
        group(
            pos.iter()
                .zip(st)
                .map(|(p, s)| dot_r(*p, f(&r), cb::sir_color(*s)))
                .collect::<Vec<_>>(),
        )
    });
    engine.register_fn("epidemic_counts", |e: Epi, t: N| {
        let (s, i, r) = e.0.counts(f(&t));
        vec![
            Dynamic::from(s as i64),
            Dynamic::from(i as i64),
            Dynamic::from(r as i64),
        ]
    });

    // ── computer science & data ──
    engine.register_type_with_name::<SortRun>("SortRun");
    engine.register_fn("sort_trace", |algo: &str, data: Array| {
        SortRun(Arc::new(av::sort_trace(algo, &floats(&data))))
    });
    engine.register_get("len", |s: &mut SortRun| s.0.len() as i64);
    engine.register_fn("sort_bars", |s: SortRun, step: N, w: N, h: N| {
        let i = (f(&step).max(0.0) as usize).min(s.0.len().saturating_sub(1));
        av::sort_bars(
            &s.0[i],
            f(&w),
            f(&h),
            rgb8(90, 110, 160),
            YELLOW,
            rgb8(90, 220, 150),
        )
    });
    engine.register_type_with_name::<PathRun>("PathRun");
    engine.register_fn(
        "path_search",
        |algo: &str, cols: N, rows: N, walls: N, seed: N| {
            let (c, r) = (f(&cols).max(4.0) as usize, f(&rows).max(3.0) as usize);
            let mut g = av::Grid::random(c, r, f(&walls), f(&seed) as u64);
            let (s, t) = ((1, r / 2), (c - 2, r / 2));
            for p in [s, t] {
                let k = g.idx(p);
                g.cost[k] = 1;
            }
            let tr = av::grid_search(&g, s, t, search_of(algo));
            PathRun(Arc::new((g, tr)))
        },
    );
    engine.register_get("expanded", |p: &mut PathRun| p.0 .1.visited.len() as i64);
    engine.register_get("cost", |p: &mut PathRun| p.0 .1.cost as i64);
    engine.register_fn("path_view", |p: PathRun, step: N, cell: N| {
        av::search_mobject(
            &p.0 .0,
            &p.0 .1,
            f(&step).max(0.0) as usize,
            f(&cell),
            rgb8(255, 120, 160),
        )
    });
    engine.register_type_with_name::<Bst>("Bst");
    engine.register_fn("bst", |keys: Array, dx: N, dy: N| {
        let k: Vec<i64> = floats(&keys).into_iter().map(|x| x as i64).collect();
        Bst(Arc::new(av::bst_layout(&k, f(&dx), f(&dy))))
    });
    engine.register_fn("bst_view", |b: Bst, upto: N, r: N| {
        av::bst_mobject(&b.0, f(&upto), f(&r), &[])
    });
    engine.register_type_with_name::<Net>("NeuralNet");
    engine.register_fn("neural_net", |sizes: Array, seed: N| {
        let s: Vec<usize> = floats(&sizes)
            .into_iter()
            .map(|x| x.max(1.0) as usize)
            .collect();
        Net(Arc::new(av::NeuralNet::random(&s, f(&seed) as u64)))
    });
    engine.register_fn(
        "nn_view",
        |n: Net, input: Array, w: N, h: N, progress: N| {
            let acts = n.0.forward(&floats(&input));
            n.0.mobject(&acts, f(&w), f(&h), f(&progress))
        },
    );
    engine.register_type_with_name::<Galton>("Galton");
    engine.register_fn("galton", |balls: N, rows: N, seed: N| {
        Galton(Arc::new(av::galton_paths(
            f(&balls) as usize,
            f(&rows).max(1.0) as usize,
            f(&seed) as u64,
        )))
    });
    engine.register_fn("galton_view", |g: Galton, t: N, interval: N| {
        av::galton_mobject(&g.0, f(&t), f(&interval), 1.3, 0.3, 0.13, 0.028, 1.0)
    });
    engine.register_fn(
        "cellular",
        |rule: N, width: N, rows: N, center: N, w: N, c: N| {
            let (wd, rw) = (f(&width).max(3.0) as usize, f(&rows).max(1.0) as usize);
            let cells = av::elementary_ca(f(&rule) as u8, wd, rw, None);
            let ww = f(&w);
            av::cells_image(
                Rect::from_center(vec_of(&center), ww, ww * rw as f64 / wd as f64),
                &cells,
                col(&c),
                rgb8(14, 16, 26),
            )
        },
    );
    engine.register_type_with_name::<Life>("Life");
    engine.register_fn("life_gun", |gens: N| {
        let mut v = vec![av::gosper_glider_gun(64, 40, 2, 2)];
        for _ in 0..f(&gens) as usize {
            let n = av::life_step(v.last().unwrap());
            v.push(n);
        }
        Life(Arc::new(v))
    });
    engine.register_fn(
        "life_view",
        |l: Life, gen: N, center: N, w: N, h: N, c: N| {
            let g = (f(&gen).max(0.0) as usize).min(l.0.len() - 1);
            av::cells_image(
                Rect::from_center(vec_of(&center), f(&w), f(&h)),
                &l.0[g],
                col(&c),
                rgb8(12, 14, 24),
            )
        },
    );
}
