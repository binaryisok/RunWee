//! Shape & morph toolkit (phase A2).
//!
//! * **Smart morph** — resamples every subpath by arc length, matches subpaths by
//!   area and picks the cyclic shift / orientation that minimises travel, so shapes
//!   melt into each other instead of crossing over (flubber-style). Optional *twist*
//!   swirls the points around the moving centroid mid-flight.
//! * **Shatter / assemble** — Voronoi-fractures any filled shape (Sutherland–Hodgman
//!   clipping of its outline against convex cells) and throws the shards ballistically.
//! * **Dissolve / materialize** — the shape crumbles into coloured dust and drifts away.
//! * **Neon write** — stroke draws on with a hot glowing pen tip, then the fill blooms.
//! * **Jelly** — decaying squash-and-stretch wobble.
//! * **Echo / motion trails** — ghost copies and tapered ribbons sampled from the past
//!   (pure functions of time, via `FrameCtx::get_at`).
//!
//! Plus the geometric helpers they are built on: flattening, resampling,
//! point-in-polygon, convex clipping and Voronoi cells.
use crate::anim::*;
use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::motion::noise1;
use crate::path::{Path, Rect, SubPath};
use crate::rate::Rate;
use crate::scene::{multiply_opacity, Obj, Scene};
use std::f64::consts::PI;
use std::sync::{Arc, OnceLock};

// ------------------------------------------------------------------ geometry helpers

/// Flatten a subpath into a polyline (`per_seg` samples per curved segment; straight
/// segments contribute just their end point). Closed subpaths do not repeat the start.
pub fn flatten_subpath(sp: &SubPath, per_seg: usize) -> Vec<Vec2> {
    let mut v = vec![sp.start];
    for (i, s) in sp.segs.iter().enumerate() {
        let p0 = sp.seg_start(i);
        let d = s.p - p0;
        let straight = d.length() > 1e-12
            && (s.c1 - p0).cross(d).abs() < 1e-9 * d.length()
            && (s.c2 - p0).cross(d).abs() < 1e-9 * d.length();
        let k = if straight { 1 } else { per_seg.max(1) };
        for j in 1..=k {
            v.push(s.eval(p0, j as f64 / k as f64));
        }
    }
    if sp.closed && v.len() > 1 && v[v.len() - 1].distance(v[0]) < 1e-9 {
        v.pop();
    }
    v
}

/// Resample a polyline to `n` points spaced uniformly by arc length.
pub fn resample_polyline(pts: &[Vec2], n: usize, closed: bool) -> Vec<Vec2> {
    if pts.is_empty() || n == 0 {
        return vec![];
    }
    let mut p: Vec<Vec2> = pts.to_vec();
    if closed {
        p.push(pts[0]);
    }
    let mut cum = vec![0.0];
    for w in p.windows(2) {
        cum.push(cum[cum.len() - 1] + w[0].distance(w[1]));
    }
    let total = *cum.last().unwrap();
    if total < 1e-12 {
        return vec![pts[0]; n];
    }
    let denom = if closed {
        n as f64
    } else {
        (n - 1).max(1) as f64
    };
    let mut out = Vec::with_capacity(n);
    let mut j = 0;
    for i in 0..n {
        let s = total * i as f64 / denom;
        while j + 1 < cum.len() - 1 && cum[j + 1] < s {
            j += 1;
        }
        let seg = (cum[j + 1] - cum[j]).max(1e-12);
        out.push(p[j].lerp(p[j + 1], ((s - cum[j]) / seg).clamp(0.0, 1.0)));
    }
    out
}

/// Signed polygon area (positive = counter-clockwise).
pub fn polygon_area(poly: &[Vec2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| poly[i].cross(poly[(i + 1) % n]))
        .sum::<f64>()
        * 0.5
}

/// Area-weighted centroid (falls back to the vertex mean for degenerate polygons).
pub fn polygon_centroid(poly: &[Vec2]) -> Vec2 {
    let a = polygon_area(poly);
    let n = poly.len();
    if n == 0 {
        return Vec2::ZERO;
    }
    if a.abs() < 1e-12 {
        return poly.iter().fold(Vec2::ZERO, |s, p| s + *p) / n as f64;
    }
    let mut c = Vec2::ZERO;
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        c += (p + q) * p.cross(q);
    }
    c / (6.0 * a)
}

/// Even-odd point-in-polygon over several rings.
pub fn point_in_rings(p: Vec2, rings: &[Vec<Vec2>]) -> bool {
    let mut inside = false;
    for poly in rings {
        let n = poly.len();
        let mut j = n.wrapping_sub(1);
        for i in 0..n {
            let (a, b) = (poly[i], poly[j]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
            j = i;
        }
    }
    inside
}

/// Keep the part of `poly` where `dot(n, x) <= c`.
pub fn clip_halfplane(poly: &[Vec2], n: Vec2, c: f64) -> Vec<Vec2> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    let m = poly.len();
    for i in 0..m {
        let (a, b) = (poly[i], poly[(i + 1) % m]);
        let (da, db) = (n.dot(a) - c, n.dot(b) - c);
        if da <= 0.0 {
            out.push(a);
        }
        if (da <= 0.0) != (db <= 0.0) {
            out.push(a.lerp(b, da / (da - db)));
        }
    }
    out
}

/// Sutherland–Hodgman: clip any polygon against a convex polygon.
pub fn clip_convex(poly: &[Vec2], convex: &[Vec2]) -> Vec<Vec2> {
    let mut cv = convex.to_vec();
    if polygon_area(&cv) < 0.0 {
        cv.reverse();
    }
    let mut out = poly.to_vec();
    let m = cv.len();
    for i in 0..m {
        if out.is_empty() {
            break;
        }
        let (a, b) = (cv[i], cv[(i + 1) % m]);
        let n = -(b - a).perp();
        out = clip_halfplane(&out, n, n.dot(a));
    }
    out
}

/// Voronoi cells of `seeds` clipped to `bounds` (cell `i` belongs to seed `i`).
pub fn voronoi_cells(seeds: &[Vec2], bounds: Rect) -> Vec<Vec<Vec2>> {
    let rect = vec![
        bounds.min,
        v2(bounds.max.x, bounds.min.y),
        bounds.max,
        v2(bounds.min.x, bounds.max.y),
    ];
    seeds
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let mut cell = rect.clone();
            for (j, &q) in seeds.iter().enumerate() {
                if i == j || cell.is_empty() {
                    continue;
                }
                let n = q - p;
                if n.length_sq() < 1e-18 {
                    continue;
                }
                cell = clip_halfplane(&cell, n, n.dot((p + q) * 0.5));
            }
            cell
        })
        .collect()
}

/// Rings (flattened subpaths) of one leaf.
pub fn leaf_rings(m: &Mobject, per_seg: usize) -> Vec<Vec<Vec2>> {
    m.path
        .subpaths
        .iter()
        .map(|s| flatten_subpath(s, per_seg))
        .filter(|r| r.len() >= 2)
        .collect()
}

fn ring_path(pts: &[Vec2], closed: bool) -> SubPath {
    Path::polyline(pts, closed)
        .subpaths
        .into_iter()
        .next()
        .unwrap_or_else(|| SubPath::new(Vec2::ZERO))
}

// ------------------------------------------------------------------ smart morph

/// Options for [`morph_mobject`].
#[derive(Clone, Copy, Debug)]
pub struct MorphOpts {
    /// Samples per subpath.
    pub samples: usize,
    /// Swirl angle (radians) applied mid-flight around the moving centroid.
    pub twist: f64,
    /// Sideways arc of the flight path (fraction of the travel distance).
    pub arc: f64,
}

impl Default for MorphOpts {
    fn default() -> Self {
        Self {
            samples: 120,
            twist: 0.0,
            arc: 0.0,
        }
    }
}

/// Reorder `b` (cyclic shift and/or reversal) so that it best matches `a`.
pub fn best_alignment(a: &[Vec2], b: &[Vec2], closed: bool) -> Vec<Vec2> {
    let n = a.len().min(b.len());
    if n == 0 {
        return b.to_vec();
    }
    let ca = a.iter().fold(Vec2::ZERO, |s, p| s + *p) / n as f64;
    let cb = b.iter().fold(Vec2::ZERO, |s, p| s + *p) / n as f64;
    let rev: Vec<Vec2> = b.iter().rev().copied().collect();
    let mut best = (f64::INFINITY, b.to_vec());
    for cand in [b, &rev[..]] {
        let shifts = if closed { n } else { 1 };
        for k in 0..shifts {
            let mut cost = 0.0;
            for i in 0..n {
                // compare shapes relative to their centroids so position doesn't dominate
                cost += ((a[i] - ca) - (cand[(i + k) % n] - cb)).length_sq();
                if cost >= best.0 {
                    break;
                }
            }
            if cost < best.0 {
                best = (cost, (0..n).map(|i| cand[(i + k) % n]).collect());
            }
        }
    }
    best.1
}

struct LeafPlan {
    a: Mobject,
    b: Mobject,
    rings: Vec<(Vec<Vec2>, Vec<Vec2>, bool)>,
}

/// Precomputed correspondence between two mobjects.
pub struct MorphPlan {
    leaves: Vec<LeafPlan>,
    ca: Vec2,
    cb: Vec2,
    radius: f64,
    target: Mobject,
    opts: MorphOpts,
}

fn pick<T: Clone>(v: &[T], i: usize, n: usize) -> T {
    v[(i * v.len() / n).min(v.len() - 1)].clone()
}

impl MorphPlan {
    pub fn new(src: &Mobject, dst: &Mobject, opts: MorphOpts) -> Self {
        let la = src.leaves();
        let lb = dst.leaves();
        let ns = opts.samples.max(8);
        let mut leaves = vec![];
        if !la.is_empty() && !lb.is_empty() {
            let n = la.len().max(lb.len());
            for i in 0..n {
                let (a, b) = (pick(&la, i, n), pick(&lb, i, n));
                leaves.push(Self::plan_leaf(a, b, ns));
            }
        }
        let (ra, rb) = (src.bounds_or_origin(), dst.bounds_or_origin());
        MorphPlan {
            leaves,
            ca: ra.center(),
            cb: rb.center(),
            radius: (ra.width().max(ra.height()) + rb.width().max(rb.height())) * 0.25 + 1e-6,
            target: dst.clone(),
            opts,
        }
    }

    fn plan_leaf(a: Mobject, b: Mobject, ns: usize) -> LeafPlan {
        let sort = |m: &Mobject| {
            let mut v: Vec<(Vec<Vec2>, bool)> = m
                .path
                .subpaths
                .iter()
                .map(|s| (flatten_subpath(s, 16), s.closed))
                .filter(|(r, _)| !r.is_empty())
                .collect();
            v.sort_by(|x, y| {
                polygon_area(&y.0)
                    .abs()
                    .total_cmp(&polygon_area(&x.0).abs())
            });
            v
        };
        let (sa, sb) = (sort(&a), sort(&b));
        let n = sa.len().max(sb.len());
        let mut rings = vec![];
        for i in 0..n {
            let ra = sa.get(i);
            let rb = sb.get(i);
            let (pa, pb, closed) = match (ra, rb) {
                (Some(x), Some(y)) => {
                    let closed = x.1 && y.1;
                    let pa = resample_polyline(&x.0, ns, closed);
                    let pb = resample_polyline(&y.0, ns, closed);
                    let pb = best_alignment(&pa, &pb, closed);
                    (pa, pb, closed)
                }
                (Some(x), None) => {
                    let pa = resample_polyline(&x.0, ns, x.1);
                    let c = polygon_centroid(&pa);
                    // collapse onto the nearest surviving ring's centroid
                    let tgt = nearest_centroid(c, &sb).unwrap_or(c);
                    (pa, vec![tgt; ns], x.1)
                }
                (None, Some(y)) => {
                    let pb = resample_polyline(&y.0, ns, y.1);
                    let c = polygon_centroid(&pb);
                    let src = nearest_centroid(c, &sa).unwrap_or(c);
                    (vec![src; ns], pb, y.1)
                }
                (None, None) => continue,
            };
            rings.push((pa, pb, closed));
        }
        LeafPlan { a, b, rings }
    }

    /// The morph at progress `t ∈ [0,1]`.
    pub fn at(&self, t: f64) -> Mobject {
        if (t - 1.0).abs() < 1e-12 {
            return self.target.clone();
        }
        let c = self.ca.lerp(self.cb, t);
        let bump = (PI * t.clamp(0.0, 1.0)).sin();
        let travel = self.cb - self.ca;
        let arc_off = travel.perp() * (self.opts.arc * bump);
        let mut g = Mobject::empty();
        for lp in &self.leaves {
            let mut leaf = Mobject::lerp_leaf(&lp.a, &lp.b, t);
            let mut path = Path::new();
            for (pa, pb, closed) in &lp.rings {
                let pts: Vec<Vec2> = pa
                    .iter()
                    .zip(pb)
                    .map(|(p, q)| {
                        let mut x = p.lerp(*q, t) + arc_off;
                        if self.opts.twist != 0.0 {
                            let r = (x - c).length() / self.radius;
                            x = x.rotate_about(
                                self.opts.twist * bump * (1.0 - 0.5 * r.min(1.0)),
                                c + arc_off,
                            );
                        }
                        x
                    })
                    .collect();
                path.subpaths.push(ring_path(&pts, *closed));
            }
            leaf.path = path;
            g.children.push(leaf);
        }
        g
    }
}

fn nearest_centroid(c: Vec2, rings: &[(Vec<Vec2>, bool)]) -> Option<Vec2> {
    rings
        .iter()
        .map(|(r, _)| polygon_centroid(r))
        .min_by(|a, b| a.distance(c).total_cmp(&b.distance(c)))
}

/// Pure morph between two mobjects (rebuilds the plan; cache a [`MorphPlan`] in hot loops).
pub fn morph_mobject(src: &Mobject, dst: &Mobject, t: f64, opts: MorphOpts) -> Mobject {
    MorphPlan::new(src, dst, opts).at(t)
}

/// Animation built from a plan computed lazily from the object's state at play time.
fn lazy_custom(
    o: Obj,
    introduces: bool,
    removes: bool,
    build: impl Fn(&Mobject) -> Box<dyn Fn(f64) -> Mobject + Send + Sync> + Send + Sync + 'static,
) -> Animation {
    let cache: Arc<OnceLock<Box<dyn Fn(f64) -> Mobject + Send + Sync>>> = Arc::new(OnceLock::new());
    let mut a = custom(o, move |m, t| (cache.get_or_init(|| build(m)))(t));
    if let AnimKind::Custom {
        introduces: i,
        removes: r,
        ..
    } = &mut a.kind
    {
        *i = introduces;
        *r = removes;
    }
    a
}

/// Morph `o` into `target` with optimal point correspondence.
pub fn smart_morph(o: Obj, target: Mobject) -> Animation {
    morph_with(o, target, MorphOpts::default())
}

/// Smart morph that swirls around the centroid mid-flight.
pub fn twist_morph(o: Obj, target: Mobject, twist: f64) -> Animation {
    morph_with(
        o,
        target,
        MorphOpts {
            twist,
            ..Default::default()
        },
    )
}

pub fn morph_with(o: Obj, target: Mobject, opts: MorphOpts) -> Animation {
    let target = Arc::new(target);
    lazy_custom(o, false, false, move |m| {
        let plan = MorphPlan::new(m, &target, opts);
        Box::new(move |t| plan.at(t))
    })
    .run_time(1.5)
}

// ------------------------------------------------------------------ shatter

/// One fragment of a shattered mobject.
#[derive(Clone, Debug)]
pub struct Shard {
    pub mob: Mobject,
    pub centroid: Vec2,
    pub vel: Vec2,
    pub spin: f64,
}

/// Physical parameters of a shatter.
#[derive(Clone, Copy, Debug)]
pub struct ShatterOpts {
    pub pieces: usize,
    pub seed: u64,
    /// Outward speed (units / normalised time).
    pub force: f64,
    pub gravity: f64,
    pub spin: f64,
}

impl Default for ShatterOpts {
    fn default() -> Self {
        Self {
            pieces: 40,
            seed: 1,
            force: 3.0,
            gravity: 6.0,
            spin: 6.0,
        }
    }
}

/// Fracture a mobject into Voronoi shards (impact point = centre biased by `seed`).
pub fn shatter_pieces(m: &Mobject, opts: ShatterOpts) -> Vec<Shard> {
    let b = m.bounds_or_origin().expand(0.05);
    let mut rng = Rng::new(opts.seed);
    // denser fracture near the impact point
    let impact = b.center()
        + v2(
            rng.range(-0.15, 0.15) * b.width(),
            rng.range(-0.15, 0.15) * b.height(),
        );
    let seeds: Vec<Vec2> = (0..opts.pieces.max(1))
        .map(|_| {
            let u = rng.next_f64().powf(0.8);
            let a = rng.range(0.0, TAU);
            let p = impact + v2(a.cos() * u * b.width(), a.sin() * u * b.height());
            p.max(b.min).min(b.max)
        })
        .collect();
    let cells = voronoi_cells(&seeds, b);
    let leaves = m.leaves();
    let mut out = vec![];
    for cell in cells.iter().filter(|c| c.len() >= 3) {
        let cc = polygon_centroid(cell);
        let mut group = Mobject::empty();
        for leaf in &leaves {
            let mut path = Path::new();
            for sp in &leaf.path.subpaths {
                let ring = flatten_subpath(sp, 12);
                if sp.closed || leaf.style.fill_opacity > 0.0 {
                    let c = clip_convex(&ring, cell);
                    if c.len() >= 3 && polygon_area(&c).abs() > 1e-6 {
                        path.subpaths.push(ring_path(&c, true));
                    }
                } else {
                    // open stroke: keep runs of points inside the cell
                    let inside = |p: Vec2| point_in_rings(p, std::slice::from_ref(cell));
                    let mut run: Vec<Vec2> = vec![];
                    for p in ring
                        .iter()
                        .copied()
                        .chain(std::iter::once(Vec2::new(f64::NAN, 0.0)))
                    {
                        if p.is_finite() && inside(p) {
                            run.push(p);
                        } else {
                            if run.len() >= 2 {
                                path.subpaths.push(ring_path(&run, false));
                            }
                            run.clear();
                        }
                    }
                }
            }
            if !path.subpaths.is_empty() {
                group.children.push(Mobject {
                    path,
                    style: leaf.style.clone(),
                    image: None,
                    children: vec![],
                    tag: leaf.tag.clone(),
                });
            }
        }
        if group.children.is_empty() {
            continue;
        }
        let away = (cc - impact).normalize();
        let d = (cc - impact).length() / (b.width().max(b.height()) * 0.5);
        let vel = (away * (0.6 + 0.8 * rng.next_f64())
            + v2(rng.normal() * 0.25, 0.5 + rng.next_f64() * 0.6))
            * opts.force
            * (1.2 - 0.5 * d.min(1.0));
        out.push(Shard {
            mob: group,
            centroid: cc,
            vel,
            spin: rng.normal() * opts.spin,
        });
    }
    out
}

/// Shards at normalised time `t` (0 = intact, 1 = scattered and faded).
pub fn shatter_frame(shards: &[Shard], t: f64, gravity: f64) -> Mobject {
    let mut g = Mobject::empty();
    let fade = 1.0 - smoothstep(0.55, 1.0, t) as f32;
    for s in shards {
        let off = s.vel * t + v2(0.0, -0.5 * gravity * t * t);
        let m = s
            .mob
            .clone()
            .rotate_about(s.spin * t, s.centroid)
            .scale_about(1.0 - 0.35 * t, s.centroid)
            .shift(off);
        g.children.push(multiply_opacity(m, fade));
    }
    g
}

/// Explode `o` into Voronoi shards (removes it afterwards).
pub fn shatter(o: Obj, opts: ShatterOpts) -> Animation {
    lazy_custom(o, false, true, move |m| {
        let shards = shatter_pieces(m, opts);
        Box::new(move |t| shatter_frame(&shards, t, opts.gravity))
    })
    .run_time(1.6)
    .rate(Rate::Linear)
}

/// Reverse shatter: shards fly in and fuse into `o`.
pub fn assemble(o: Obj, opts: ShatterOpts) -> Animation {
    lazy_custom(o, true, false, move |m| {
        let shards = shatter_pieces(m, opts);
        let whole = m.clone();
        Box::new(move |t| {
            if t >= 1.0 {
                whole.clone()
            } else {
                shatter_frame(&shards, 1.0 - t, opts.gravity * 0.3)
            }
        })
    })
    .run_time(1.6)
    .rate(Rate::OutCubic)
}

// ------------------------------------------------------------------ dissolve

/// A coloured point sampled from a shape.
#[derive(Clone, Copy, Debug)]
pub struct Grain {
    pub pos: Vec2,
    pub color: Color,
    pub dir: Vec2,
    pub r: f64,
    pub delay: f64,
}

/// Sample `n` grains inside filled leaves (or along stroked outlines).
pub fn sample_grains(m: &Mobject, n: usize, seed: u64) -> Vec<Grain> {
    let leaves = m.leaves();
    let mut rng = Rng::new(seed);
    let b = m.bounds_or_origin();
    let mut out = vec![];
    if leaves.is_empty() {
        return out;
    }
    let ringsets: Vec<Vec<Vec<Vec2>>> = leaves.iter().map(|l| leaf_rings(l, 10)).collect();
    let mut tries = 0;
    while out.len() < n && tries < n * 60 {
        tries += 1;
        let i = (rng.next_u64() as usize) % leaves.len();
        let leaf = &leaves[i];
        let filled = leaf.style.fill_opacity > 0.05;
        let (p, col) = if filled {
            let p = v2(rng.range(b.min.x, b.max.x), rng.range(b.min.y, b.max.y));
            if !point_in_rings(p, &ringsets[i]) {
                continue;
            }
            (p, leaf.style.fill.color())
        } else {
            (
                leaf.path.point_from_proportion(rng.next_f64()),
                leaf.style.stroke.color(),
            )
        };
        let x_norm = (p.x - b.min.x) / b.width().max(1e-6);
        out.push(Grain {
            pos: p,
            color: col,
            dir: v2(0.4 + rng.next_f64(), rng.normal() * 0.6 + 0.4),
            r: 0.012 + 0.02 * rng.next_f64(),
            delay: x_norm * 0.45 + rng.next_f64() * 0.1,
        });
    }
    out
}

/// Dust at normalised time `t` (0 = in place, 1 = blown away).
pub fn dust_frame(grains: &[Grain], t: f64, wind: f64, seed: u64) -> Mobject {
    let mut g = Mobject::empty();
    for (i, gr) in grains.iter().enumerate() {
        let u = ((t - gr.delay) / 0.5).clamp(0.0, 1.0);
        if u >= 1.0 {
            continue;
        }
        let e = u * u;
        let k = i as f64 * 0.37;
        let swirl = v2(
            noise1(k + u * 3.0, seed),
            noise1(k + 50.0 + u * 3.0, seed ^ 7),
        ) * 0.4
            * u;
        let p = gr.pos + gr.dir * (wind * e) + swirl;
        let a = (1.0 - u) as f32;
        g.children
            .push(crate::shapes::dot_r(p, gr.r * (1.0 - 0.5 * u), gr.color).opacity(a));
    }
    g
}

fn dissolve_fn(m: &Mobject, seed: u64, t: f64, grains: &[Grain]) -> Mobject {
    let body = multiply_opacity(m.clone(), 1.0 - smoothstep(0.0, 0.35, t) as f32);
    Mobject::group([body, dust_frame(grains, t, 3.0, seed)])
}

/// Crumble `o` into drifting dust (Thanos snap). Removes it afterwards.
pub fn dissolve(o: Obj, grains: usize, seed: u64) -> Animation {
    lazy_custom(o, false, true, move |m| {
        let gs = sample_grains(m, grains, seed);
        let m = m.clone();
        Box::new(move |t| dissolve_fn(&m, seed, t, &gs))
    })
    .run_time(2.0)
    .rate(Rate::Linear)
}

/// Dust converges and the shape condenses out of it.
pub fn materialize(o: Obj, grains: usize, seed: u64) -> Animation {
    lazy_custom(o, true, false, move |m| {
        let gs = sample_grains(m, grains, seed);
        let m = m.clone();
        Box::new(move |t| {
            if t >= 1.0 {
                m.clone()
            } else {
                dissolve_fn(&m, seed, 1.0 - t, &gs)
            }
        })
    })
    .run_time(2.0)
    .rate(Rate::Linear)
}

// ------------------------------------------------------------------ neon write / jelly

/// Draw the outline with a glowing pen tip, then bloom the fill in.
pub fn neon_write(o: Obj, color: Color) -> Animation {
    let mut a = custom(o, move |m, t| {
        let draw = (t / 0.8).min(1.0);
        let mut outline = m.clone().partial(0.0, draw);
        outline.for_each_mut(&mut |n| {
            n.style.fill_opacity *= smoothstep(0.75, 1.0, t) as f32;
            if n.style.stroke_width < 2.0 {
                n.style.stroke_width = 2.0;
            }
            n.style.stroke_opacity = n.style.stroke_opacity.max(0.9);
        });
        let outline = outline.glow(0.12, Some(color));
        if draw >= 1.0 {
            return outline;
        }
        let tip = m.point_from_proportion(draw);
        let tip_dot = crate::shapes::dot_r(tip, 0.06, WHITE).glow(0.35, Some(color));
        Mobject::group([outline, tip_dot])
    });
    if let AnimKind::Custom { introduces, .. } = &mut a.kind {
        *introduces = true;
    }
    a.run_time(2.0).rate(Rate::Linear)
}

/// Squash-and-stretch jelly wobble that decays to rest.
pub fn jelly(o: Obj, amplitude: f64, wobbles: f64) -> Animation {
    custom(o, move |m, t| jelly_shape(m, t, amplitude, wobbles))
        .run_time(1.4)
        .rate(Rate::Linear)
}

/// Pure jelly deformation at progress `t`.
pub fn jelly_shape(m: &Mobject, t: f64, amplitude: f64, wobbles: f64) -> Mobject {
    if t <= 0.0 || t >= 1.0 {
        return m.clone();
    }
    let c = m.center();
    let k = amplitude * (TAU * wobbles * t).sin() * (1.0 - t).powi(2);
    let mut out = m.clone();
    out.for_each_mut(&mut |n| {
        n.path = n.path.subdivided(4);
        n.path.map_points(&|p| {
            let d = p - c;
            // area-preserving-ish: stretch x, squash y, plus a slight radial ripple
            let ripple = 1.0 + 0.25 * k * (3.0 * d.angle() + t * 10.0).sin();
            v2(
                c.x + d.x * (1.0 + k) * ripple,
                c.y + d.y / (1.0 + k) * ripple,
            )
        });
    });
    out
}

// ------------------------------------------------------------------ trails

/// Ghost copies of `o` from the recent past (`copies` ghosts `spacing` seconds apart).
pub fn echo_trail(s: &mut Scene, o: Obj, copies: usize, spacing: f64, opacity: f32) -> Obj {
    s.always_redraw(move |ctx| {
        let now = ctx.get(o);
        let c0 = now.center();
        let mut g = Mobject::empty();
        for k in (1..=copies).rev() {
            let past = ctx.get_at(o, (ctx.t - k as f64 * spacing).max(0.0));
            if past.center().distance(c0) < 1e-3 || past.leaf_count() == 0 {
                continue;
            }
            let a = opacity * (1.0 - (k - 1) as f32 / copies as f32);
            g.children.push(multiply_opacity(past, a));
        }
        g
    })
}

/// A tapered ribbon following the centre of `o` over the last `length` seconds.
pub fn motion_trail(s: &mut Scene, o: Obj, length: f64, width: f32, color: Color) -> Obj {
    s.always_redraw(move |ctx| {
        let n = 40;
        let pts: Vec<Vec2> = (0..=n)
            .map(|i| {
                let tt = ctx.t - length * (1.0 - i as f64 / n as f64);
                ctx.get_at(o, tt.max(0.0)).center()
            })
            .collect();
        Mobject::group(
            pts.windows(2)
                .enumerate()
                .filter(|(_, w)| w[0].distance(w[1]) > 1e-6)
                .map(|(i, w)| {
                    let k = (i + 1) as f32 / n as f32;
                    crate::shapes::line(w[0], w[1]).stroke(color.with_alpha(k * k), width * k)
                }),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::*;

    fn square_ring() -> Vec<Vec2> {
        vec![v2(-1.0, -1.0), v2(1.0, -1.0), v2(1.0, 1.0), v2(-1.0, 1.0)]
    }

    #[test]
    fn polygon_basics() {
        let sq = square_ring();
        assert!((polygon_area(&sq) - 4.0).abs() < 1e-12);
        assert!(polygon_centroid(&sq).length() < 1e-12);
        assert!(point_in_rings(v2(0.2, 0.3), std::slice::from_ref(&sq)));
        assert!(!point_in_rings(v2(1.2, 0.3), std::slice::from_ref(&sq)));
        // hole via even-odd
        let hole: Vec<Vec2> = sq.iter().map(|p| *p * 0.5).collect();
        assert!(!point_in_rings(v2(0.1, 0.1), &[sq.clone(), hole]));
    }

    #[test]
    fn convex_clip_and_voronoi_partition() {
        let sq = square_ring();
        let half = clip_halfplane(&sq, v2(1.0, 0.0), 0.0);
        assert!((polygon_area(&half) - 2.0).abs() < 1e-12);
        let tri = vec![v2(0.0, 0.0), v2(3.0, 0.0), v2(0.0, 3.0)];
        let c = clip_convex(&sq, &tri);
        assert!((polygon_area(&c).abs() - 1.0).abs() < 1e-9);
        let seeds = vec![v2(-0.5, -0.5), v2(0.5, 0.2), v2(0.1, 0.7), v2(-0.6, 0.6)];
        let cells = voronoi_cells(&seeds, Rect::new(v2(-1.0, -1.0), v2(1.0, 1.0)));
        let total: f64 = cells.iter().map(|c| polygon_area(c).abs()).sum();
        assert!((total - 4.0).abs() < 1e-9, "cells tile the box: {total}");
        for (cell, s) in cells.iter().zip(&seeds) {
            assert!(point_in_rings(*s, std::slice::from_ref(cell)));
        }
    }

    #[test]
    fn resample_is_uniform() {
        let r = resample_polyline(&square_ring(), 8, true);
        assert_eq!(r.len(), 8);
        for i in 0..8 {
            assert!((r[i].distance(r[(i + 1) % 8]) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn alignment_finds_shift_and_orientation() {
        let a: Vec<Vec2> = (0..12)
            .map(|i| Vec2::from_angle(i as f64 * TAU / 12.0))
            .collect();
        let mut b: Vec<Vec2> = a.iter().rev().copied().collect();
        b.rotate_left(5);
        let al = best_alignment(&a, &b, true);
        for (p, q) in a.iter().zip(&al) {
            assert!(p.distance(*q) < 1e-9);
        }
    }

    #[test]
    fn morph_endpoints() {
        let a = circle(1.0).fill(BLUE, 1.0);
        let b = square(2.0).shift(v2(3.0, 0.0)).fill(RED, 1.0);
        let plan = MorphPlan::new(&a, &b, MorphOpts::default());
        let m0 = plan.at(0.0);
        assert!(m0.center().distance(a.center()) < 0.02);
        assert!((m0.width() - 2.0).abs() < 0.02);
        let mid = plan.at(0.5);
        assert!((mid.center().x - 1.5).abs() < 0.05);
        assert_eq!(plan.at(1.0), b);
    }

    #[test]
    fn shatter_conserves_area() {
        let sq = square(2.0).fill(TEAL, 1.0);
        let shards = shatter_pieces(
            &sq,
            ShatterOpts {
                pieces: 25,
                ..Default::default()
            },
        );
        assert!(shards.len() > 10);
        let area: f64 = shards
            .iter()
            .flat_map(|s| s.mob.leaves())
            .flat_map(|l| leaf_rings(&l, 1))
            .map(|r| polygon_area(&r).abs())
            .sum();
        assert!((area - 4.0).abs() < 0.02, "area {area}");
        let f0 = shatter_frame(&shards, 0.0, 6.0);
        assert!(f0.center().distance(sq.center()) < 1e-6);
    }

    #[test]
    fn grains_land_inside() {
        let c = circle(1.0).fill(YELLOW, 1.0);
        let gs = sample_grains(&c, 200, 3);
        assert_eq!(gs.len(), 200);
        assert!(gs.iter().all(|g| g.pos.length() <= 1.01));
    }

    #[test]
    fn jelly_rests_at_ends() {
        let s = square(1.0);
        assert_eq!(jelly_shape(&s, 0.0, 0.3, 3.0), s);
        assert_eq!(jelly_shape(&s, 1.0, 0.3, 3.0), s);
        let mid = jelly_shape(&s, 0.1, 0.3, 3.0);
        assert!((mid.width() - 1.0).abs() > 0.01);
    }
}
