//! Hand-drawn rendering (rough.js-style), phase A2.
//!
//! [`sketchy`] redraws any mobject as wobbly double-pass pencil strokes with
//! overshooting ends and optional hachure (or cross-hatch / zig-zag) fill;
//! [`boil`] re-seeds it a few times per second for the classic "line boil" of
//! hand-drawn animation. Everything is deterministic for a given seed.
use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::morph::{leaf_rings, resample_polyline};
use crate::motion::noise1;
use crate::path::Path;
use crate::style::Style;

/// Fill style for [`sketchy`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SketchFill {
    None,
    Hachure,
    CrossHatch,
    ZigZag,
}

/// Parameters for hand-drawn rendering.
#[derive(Clone, Copy, Debug)]
pub struct SketchOpts {
    /// Wobble amplitude in scene units.
    pub roughness: f64,
    /// Number of overlapping pencil passes.
    pub passes: usize,
    pub fill: SketchFill,
    /// Distance between hachure lines.
    pub gap: f64,
    /// Hachure angle (radians).
    pub angle: f64,
    pub stroke_width: f32,
    pub fill_width: f32,
    pub seed: u64,
}

impl Default for SketchOpts {
    fn default() -> Self {
        Self {
            roughness: 0.04,
            passes: 2,
            fill: SketchFill::Hachure,
            gap: 0.09,
            angle: -0.7,
            stroke_width: 3.0,
            fill_width: 2.0,
            seed: 1,
        }
    }
}

impl SketchOpts {
    pub fn roughness(mut self, r: f64) -> Self {
        self.roughness = r;
        self
    }
    pub fn fill(mut self, f: SketchFill) -> Self {
        self.fill = f;
        self
    }
    pub fn gap(mut self, g: f64) -> Self {
        self.gap = g;
        self
    }
    pub fn angle(mut self, a: f64) -> Self {
        self.angle = a;
        self
    }
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }
    pub fn passes(mut self, n: usize) -> Self {
        self.passes = n.max(1);
        self
    }
}

fn stroke_mob(path: Path, color: Color, width: f32, opacity: f32) -> Mobject {
    let style = Style {
        fill_opacity: 0.0,
        stroke: crate::style::Paint::Solid(color),
        stroke_width: width,
        stroke_opacity: opacity,
        ..Default::default()
    };
    Mobject {
        path,
        style,
        ..Default::default()
    }
}

/// One wobbly pencil pass along `pts`.
fn rough_line(pts: &[Vec2], closed: bool, rough: f64, seed: u64) -> Path {
    if pts.len() < 2 {
        return Path::new();
    }
    let len: f64 = pts.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>()
        + if closed {
            pts[0].distance(pts[pts.len() - 1])
        } else {
            0.0
        };
    let n = ((len / 0.12).ceil() as usize).clamp(4, 400);
    let mut r = resample_polyline(pts, n, closed);
    if closed {
        // overshoot past the start like a real pen stroke
        let extra = (n / 12).max(1);
        let head: Vec<Vec2> = r[..extra.min(r.len())].to_vec();
        r.extend(head);
    } else {
        let d0 = (r[0] - r[1]).normalize() * rough * 1.5;
        let d1 = (r[n - 1] - r[n - 2]).normalize() * rough * 1.5;
        r.insert(0, r[0] + d0);
        let last = r[r.len() - 1];
        r.push(last + d1);
    }
    let s = seed as f64 * 13.7;
    let jittered: Vec<Vec2> = r
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let u = i as f64 * 0.35 + s;
            *p + v2(noise1(u, seed), noise1(u + 91.0, seed ^ 0x55)) * rough
        })
        .collect();
    Path::smooth_through(&jittered, false)
}

/// Clip horizontal scanlines against even-odd rings (rings already rotated).
fn scan_segments(rings: &[Vec<Vec2>], gap: f64, phase: f64) -> Vec<(Vec2, Vec2)> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for r in rings {
        for p in r {
            lo = lo.min(p.y);
            hi = hi.max(p.y);
        }
    }
    let mut out = vec![];
    if !lo.is_finite() {
        return out;
    }
    let mut y = lo + gap * (0.5 + phase);
    while y < hi {
        let mut xs = vec![];
        for r in rings {
            let n = r.len();
            for i in 0..n {
                let (a, b) = (r[i], r[(i + 1) % n]);
                if (a.y > y) != (b.y > y) {
                    xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
        }
        xs.sort_by(f64::total_cmp);
        for pair in xs.chunks(2) {
            if pair.len() == 2 && pair[1] - pair[0] > 1e-4 {
                out.push((v2(pair[0], y), v2(pair[1], y)));
            }
        }
        y += gap;
    }
    out
}

/// Hachure lines filling `rings` at `angle`.
pub fn hachure_lines(rings: &[Vec<Vec2>], gap: f64, angle: f64) -> Vec<(Vec2, Vec2)> {
    let rot: Vec<Vec<Vec2>> = rings
        .iter()
        .map(|r| r.iter().map(|p| p.rotate(-angle)).collect())
        .collect();
    scan_segments(&rot, gap, 0.0)
        .into_iter()
        .map(|(a, b)| (a.rotate(angle), b.rotate(angle)))
        .collect()
}

/// Redraw a mobject in a hand-drawn style.
pub fn sketchy(m: &Mobject, o: SketchOpts) -> Mobject {
    let mut g = Mobject::empty();
    let mut rng = Rng::new(o.seed);
    for leaf in m.leaves() {
        let rings = leaf_rings(&leaf, 12);
        let st = &leaf.style;
        let opacity = st.opacity;
        // ---- fill
        if st.fill_opacity > 0.02 && o.fill != SketchFill::None {
            let fc = st.fill.color();
            let mut fill_path = Path::new();
            let angles: Vec<f64> = match o.fill {
                SketchFill::CrossHatch => vec![o.angle, o.angle + std::f64::consts::FRAC_PI_2],
                _ => vec![o.angle],
            };
            for ang in angles {
                let lines = hachure_lines(&rings, o.gap, ang);
                if o.fill == SketchFill::ZigZag {
                    let mut pts = vec![];
                    for (i, (a, b)) in lines.iter().enumerate() {
                        if i % 2 == 0 {
                            pts.extend([*a, *b]);
                        } else {
                            pts.extend([*b, *a]);
                        }
                    }
                    let p = rough_line(&pts, false, o.roughness * 0.5, rng.next_u64() % 10_000);
                    fill_path.subpaths.extend(p.subpaths);
                } else {
                    for (a, b) in lines {
                        // slightly bowed strokes with random overshoot
                        let d = b - a;
                        let bow = d.perp().normalize() * rng.normal() * o.roughness * 0.6;
                        let a2 = a - d.normalize() * rng.range(0.0, o.roughness);
                        let b2 = b + d.normalize() * rng.range(0.0, o.roughness);
                        let mut p = Path::new();
                        p.move_to(a2)
                            .cubic_to(a2 + d / 3.0 + bow, a2 + d * (2.0 / 3.0) + bow, b2);
                        fill_path.subpaths.extend(p.subpaths);
                    }
                }
            }
            g.children.push(stroke_mob(
                fill_path,
                fc,
                o.fill_width,
                st.fill_opacity.max(0.6) * opacity,
            ));
        }
        // ---- outline
        let (sc, sw) = if st.stroke_width > 0.0 && st.stroke_opacity > 0.02 {
            (st.stroke.color(), o.stroke_width.max(st.stroke_width * 0.6))
        } else {
            (st.fill.color(), o.stroke_width)
        };
        let mut outline = Path::new();
        for (sp, ring) in leaf.path.subpaths.iter().zip(&rings) {
            for _ in 0..o.passes {
                let p = rough_line(ring, sp.closed, o.roughness, rng.next_u64() % 10_000);
                outline.subpaths.extend(p.subpaths);
            }
        }
        g.children.push(stroke_mob(outline, sc, sw, 0.9 * opacity));
    }
    g
}

/// Re-seeded sketch at `fps` drawings per second ("line boil").
pub fn boil(m: &Mobject, t: f64, fps: f64, o: SketchOpts) -> Mobject {
    let frame = (t * fps).floor().max(0.0) as u64;
    sketchy(m, o.seed(o.seed.wrapping_mul(31).wrapping_add(frame)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morph::polygon_area;
    use crate::shapes::*;

    #[test]
    fn hachure_covers_square() {
        let sq = vec![v2(-1.0, -1.0), v2(1.0, -1.0), v2(1.0, 1.0), v2(-1.0, 1.0)];
        let lines = hachure_lines(std::slice::from_ref(&sq), 0.1, 0.0);
        assert_eq!(lines.len(), 20);
        assert!(lines
            .iter()
            .all(|(a, b)| (a.distance(*b) - 2.0).abs() < 1e-9));
        // hole leaves gaps (even-odd)
        let hole: Vec<Vec2> = sq.iter().map(|p| *p * 0.5).collect();
        let l2 = hachure_lines(&[sq.clone(), hole], 0.1, 0.0);
        assert!(l2.len() > lines.len());
        assert!(polygon_area(&sq) > 0.0);
    }

    #[test]
    fn sketch_is_deterministic_and_boils() {
        let c = circle(1.0).fill(RED, 0.5).stroke(WHITE, 4.0);
        let a = sketchy(&c, SketchOpts::default());
        let b = sketchy(&c, SketchOpts::default());
        assert_eq!(a, b);
        assert_eq!(a.children.len(), 2);
        let f0 = boil(&c, 0.0, 8.0, SketchOpts::default());
        let f1 = boil(&c, 0.2, 8.0, SketchOpts::default());
        assert_ne!(f0, f1);
        assert!(a.center().distance(c.center()) < 0.1);
    }
}
