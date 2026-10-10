//! Boolean shape operations (manim's `Union`, `Intersection`, `Difference`,
//! `Exclusion`), `ConvexHull` and `Cutout`.
//!
//! Shapes are rasterised into high-resolution anti-aliased coverage masks, combined
//! per pixel and traced back into smooth outlines with sub-pixel marching squares.
//! This is robust for any input (self-intersections, holes, curves).

use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::path::*;
use crate::style::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    Union,
    Intersection,
    Difference,
    Exclusion,
}

fn coverage(m: &Mobject, bounds: Rect, res: usize) -> (Vec<f32>, usize, usize) {
    let s = res as f64 / bounds.width().max(bounds.height()).max(1e-9);
    let w = ((bounds.width() * s).ceil() as usize + 4).max(4);
    let h = ((bounds.height() * s).ceil() as usize + 4).max(4);
    let mut mask = tiny_skia::Mask::new(w as u32, h as u32).unwrap();
    let to_px = |p: Vec2| -> (f32, f32) {
        (
            ((p.x - bounds.min.x) * s + 2.0) as f32,
            ((bounds.max.y - p.y) * s + 2.0) as f32,
        )
    };
    m.for_each(&mut |n| {
        if n.path.subpaths.is_empty() {
            return;
        }
        let mut pb = tiny_skia::PathBuilder::new();
        for sp in &n.path.subpaths {
            let (x, y) = to_px(sp.start);
            pb.move_to(x, y);
            for c in &sp.segs {
                let (a, b) = to_px(c.c1);
                let (cx, cy) = to_px(c.c2);
                let (dx, dy) = to_px(c.p);
                pb.cubic_to(a, b, cx, cy, dx, dy);
            }
            pb.close();
        }
        if let Some(p) = pb.finish() {
            let rule = match n.style.fill_rule {
                FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
                FillRule::NonZero => tiny_skia::FillRule::Winding,
            };
            mask.fill_path(&p, rule, true, tiny_skia::Transform::identity());
        }
    });
    (
        mask.data().iter().map(|v| *v as f32 / 255.0).collect(),
        w,
        h,
    )
}

/// Trace the 0.5 iso-contour of a coverage grid into closed loops (grid coordinates).
fn trace(grid: &[f32], w: usize, h: usize) -> Vec<Vec<Vec2>> {
    let at = |x: usize, y: usize| grid[y * w + x] - 0.5;
    // edge keys → points; segments as pairs of edge ids
    let edge_pt = |x0: usize, y0: usize, x1: usize, y1: usize| -> Vec2 {
        let (a, b) = (at(x0, y0), at(x1, y1));
        let t = (a / (a - b)).clamp(0.0, 1.0) as f64;
        v2(
            x0 as f64 + (x1 as f64 - x0 as f64) * t,
            y0 as f64 + (y1 as f64 - y0 as f64) * t,
        )
    };
    // edge id: horizontal edges (x,y)-(x+1,y) = 2*(y*w+x), vertical = 2*(y*w+x)+1
    let hid = |x: usize, y: usize| 2 * (y * w + x);
    let vid = |x: usize, y: usize| 2 * (y * w + x) + 1;
    let mut next: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut pts: std::collections::HashMap<usize, Vec2> = std::collections::HashMap::new();
    for y in 0..h - 1 {
        for x in 0..w - 1 {
            let c = [
                at(x, y) > 0.0,
                at(x + 1, y) > 0.0,
                at(x + 1, y + 1) > 0.0,
                at(x, y + 1) > 0.0,
            ];
            let idx = (c[0] as u8) | (c[1] as u8) << 1 | (c[2] as u8) << 2 | (c[3] as u8) << 3;
            // edges: top(0) right(1) bottom(2) left(3)
            let e = |k: usize| -> usize {
                match k {
                    0 => hid(x, y),
                    1 => vid(x + 1, y),
                    2 => hid(x, y + 1),
                    _ => vid(x, y),
                }
            };
            let p = |k: usize| -> Vec2 {
                match k {
                    0 => edge_pt(x, y, x + 1, y),
                    1 => edge_pt(x + 1, y, x + 1, y + 1),
                    2 => edge_pt(x, y + 1, x + 1, y + 1),
                    _ => edge_pt(x, y, x, y + 1),
                }
            };
            // oriented segments so that "inside" is on the left
            let segs: &[(usize, usize)] = match idx {
                1 => &[(3, 0)],
                2 => &[(0, 1)],
                3 => &[(3, 1)],
                4 => &[(1, 2)],
                5 => &[(3, 2), (1, 0)],
                6 => &[(0, 2)],
                7 => &[(3, 2)],
                8 => &[(2, 3)],
                9 => &[(2, 0)],
                10 => &[(0, 3), (2, 1)],
                11 => &[(2, 1)],
                12 => &[(1, 3)],
                13 => &[(1, 0)],
                14 => &[(0, 3)],
                _ => &[],
            };
            for &(a, b) in segs {
                let (ea, eb) = (e(a), e(b));
                pts.entry(ea).or_insert_with(|| p(a));
                pts.entry(eb).or_insert_with(|| p(b));
                next.insert(ea, eb);
            }
        }
    }
    let mut loops = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let keys: Vec<usize> = next.keys().copied().collect();
    for start in keys {
        if seen.contains(&start) {
            continue;
        }
        let mut lp = Vec::new();
        let mut cur = start;
        while seen.insert(cur) {
            lp.push(pts[&cur]);
            match next.get(&cur) {
                Some(n) => cur = *n,
                None => break,
            }
        }
        if lp.len() > 2 {
            loops.push(lp);
        }
    }
    loops
}

/// Ramer–Douglas–Peucker simplification of a closed loop.
fn simplify(pts: &[Vec2], eps: f64) -> Vec<Vec2> {
    fn rdp(p: &[Vec2], eps: f64, out: &mut Vec<Vec2>) {
        if p.len() < 3 {
            out.extend_from_slice(&p[..p.len().saturating_sub(1)]);
            return;
        }
        let (a, b) = (p[0], p[p.len() - 1]);
        let d = b - a;
        let (mut idx, mut dmax) = (0, 0.0);
        for (i, q) in p.iter().enumerate().skip(1).take(p.len() - 2) {
            let dist = if d.length() < 1e-12 {
                q.distance(a)
            } else {
                (*q - a).cross(d).abs() / d.length()
            };
            if dist > dmax {
                dmax = dist;
                idx = i;
            }
        }
        if dmax > eps {
            rdp(&p[..=idx], eps, out);
            rdp(&p[idx..], eps, out);
        } else {
            out.push(a);
        }
    }
    let mut out = Vec::new();
    let mut closed = pts.to_vec();
    closed.push(pts[0]);
    rdp(&closed, eps, &mut out);
    out
}

/// Combine two shapes. Result keeps `a`'s style (filled) with even-odd fill.
pub fn boolean(a: &Mobject, b: &Mobject, op: BoolOp) -> Mobject {
    let ba = a.bounds_or_origin();
    let bb = b.bounds_or_origin();
    let bounds = ba.union(&bb);
    let res = 900;
    let (ca, w, h) = coverage(a, bounds, res);
    let (cb, _, _) = coverage(b, bounds, res);
    let grid: Vec<f32> = ca
        .iter()
        .zip(&cb)
        .map(|(x, y)| match op {
            BoolOp::Union => x.max(*y),
            BoolOp::Intersection => x.min(*y),
            BoolOp::Difference => x.min(1.0 - y),
            BoolOp::Exclusion => x.max(*y).min(1.0 - x.min(*y)),
        })
        .collect();
    let s = res as f64 / bounds.width().max(bounds.height()).max(1e-9);
    let mut path = Path::new();
    for lp in trace(&grid, w, h) {
        let simp = simplify(&lp, 0.35);
        if simp.len() < 3 {
            continue;
        }
        let scene: Vec<Vec2> = simp
            .iter()
            .map(|p| {
                v2(
                    bounds.min.x + (p.x - 2.0) / s,
                    bounds.max.y - (p.y - 2.0) / s,
                )
            })
            .collect();
        path.subpaths.extend(Path::polyline(&scene, true).subpaths);
    }
    let mut style = a
        .leaves()
        .first()
        .map(|l| l.style.clone())
        .unwrap_or_default();
    if style.fill_opacity <= 0.0 {
        style.fill_opacity = 0.5;
    }
    style.fill_rule = FillRule::EvenOdd;
    Mobject::from_path_styled(path, style)
}

pub fn union(a: &Mobject, b: &Mobject) -> Mobject {
    boolean(a, b, BoolOp::Union)
}
pub fn intersection(a: &Mobject, b: &Mobject) -> Mobject {
    boolean(a, b, BoolOp::Intersection)
}
pub fn difference(a: &Mobject, b: &Mobject) -> Mobject {
    boolean(a, b, BoolOp::Difference)
}
pub fn exclusion(a: &Mobject, b: &Mobject) -> Mobject {
    boolean(a, b, BoolOp::Exclusion)
}

/// Main shape with holes cut out by other shapes (manim's `Cutout`).
pub fn cutout(main: &Mobject, holes: &[Mobject]) -> Mobject {
    holes
        .iter()
        .fold(main.clone(), |acc, h| difference(&acc, h))
}

/// Convex hull of points (Andrew's monotone chain).
pub fn convex_hull_points(points: &[Vec2]) -> Vec<Vec2> {
    let mut p: Vec<Vec2> = points.to_vec();
    p.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap()
            .then(a.y.partial_cmp(&b.y).unwrap())
    });
    p.dedup_by(|a, b| (*a - *b).length() < 1e-12);
    if p.len() < 3 {
        return p;
    }
    let mut lower: Vec<Vec2> = Vec::new();
    for q in &p {
        while lower.len() >= 2
            && (lower[lower.len() - 1] - lower[lower.len() - 2]).cross(*q - lower[lower.len() - 2])
                <= 0.0
        {
            lower.pop();
        }
        lower.push(*q);
    }
    let mut upper: Vec<Vec2> = Vec::new();
    for q in p.iter().rev() {
        while upper.len() >= 2
            && (upper[upper.len() - 1] - upper[upper.len() - 2]).cross(*q - upper[upper.len() - 2])
                <= 0.0
        {
            upper.pop();
        }
        upper.push(*q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Convex hull polygon around points or around a mobject's outline (manim's `ConvexHull`).
pub fn convex_hull(points: &[Vec2]) -> Mobject {
    crate::shapes::polygon(&convex_hull_points(points)).stroke(YELLOW, 4.0)
}

pub fn convex_hull_of(m: &Mobject) -> Mobject {
    let p = m.family_path();
    let pts: Vec<Vec2> = (0..=400)
        .map(|i| p.point_from_proportion(i as f64 / 400.0))
        .collect();
    convex_hull(&pts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::*;

    fn area(m: &Mobject) -> f64 {
        m.path
            .subpaths
            .iter()
            .map(|sp| {
                let mut a = 0.0;
                let mut prev = sp.start;
                for c in &sp.segs {
                    a += prev.cross(c.p);
                    prev = c.p;
                }
                a / 2.0
            })
            .map(f64::abs)
            .sum()
    }

    #[test]
    fn boolean_areas() {
        let a = square(2.0).fill(BLUE, 1.0);
        let b = square(2.0).shift(RIGHT).fill(RED, 1.0);
        let u = union(&a, &b);
        let i = intersection(&a, &b);
        let d = difference(&a, &b);
        assert!((area(&u) - 6.0).abs() < 0.08, "union {}", area(&u));
        assert!((area(&i) - 2.0).abs() < 0.05, "intersection {}", area(&i));
        assert!((area(&d) - 2.0).abs() < 0.05, "difference {}", area(&d));
        let x = exclusion(&a, &b);
        assert!((area(&x) - 4.0).abs() < 0.1, "exclusion {}", area(&x));
        let ring = cutout(&circle(2.0).fill(WHITE, 1.0), &[circle(1.0)]);
        assert_eq!(ring.path.subpaths.len(), 2);
        let hull = convex_hull_points(&[
            v2(0.0, 0.0),
            v2(1.0, 0.0),
            v2(0.5, 0.2),
            v2(1.0, 1.0),
            v2(0.0, 1.0),
        ]);
        assert_eq!(hull.len(), 4);
        assert!(convex_hull_of(&star(5, 1.0, 0.4)).anchors().len() >= 5);
    }
}
