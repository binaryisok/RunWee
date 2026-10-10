//! Scene transitions — every presentation of `@remotion/transitions`
//! (fade, slide, wipe, flip, clock-wipe, iris, none, cross-zoom, zoom-in-out,
//! push-cut, swap, dissolve, blur-slide, linear-blur, zoom-blur, ripple,
//! crosswarp, dreamy-zoom, film-burn, book-flip) as frame compositors.

use crate::effects::{blur, sample, Effect};
use crate::rate::{Rate, SpringConfig};
use rayon::prelude::*;
use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};

/// Direction an incoming scene enters from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Dir {
    #[default]
    FromRight,
    FromLeft,
    FromTop,
    FromBottom,
}

impl Dir {
    /// Unit vector (pixels, y down) pointing from where the scene comes from towards centre.
    fn vec(self) -> (f32, f32) {
        match self {
            Dir::FromRight => (1.0, 0.0),
            Dir::FromLeft => (-1.0, 0.0),
            Dir::FromTop => (0.0, -1.0),
            Dir::FromBottom => (0.0, 1.0),
        }
    }
    pub fn parse(s: &str) -> Dir {
        match s.to_ascii_lowercase().as_str() {
            "from-left" | "from_left" | "left" => Dir::FromLeft,
            "from-top" | "from_top" | "top" => Dir::FromTop,
            "from-bottom" | "from_bottom" | "bottom" => Dir::FromBottom,
            _ => Dir::FromRight,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Presentation {
    None,
    Fade,
    Slide(Dir),
    Wipe(Dir),
    Flip(Dir),
    ClockWipe,
    Iris,
    CrossZoom,
    ZoomInOut,
    PushCut(Dir),
    Swap(Dir),
    Dissolve,
    BlurSlide(Dir),
    LinearBlur,
    ZoomBlur,
    Ripple,
    Crosswarp,
    DreamyZoom,
    FilmBurn,
    BookFlip,
}

impl Presentation {
    pub fn parse(name: &str) -> Option<Presentation> {
        let n = name.to_ascii_lowercase().replace('_', "-");
        let (base, dir) = match n.split_once(':') {
            Some((b, d)) => (b.to_string(), Dir::parse(d)),
            None => (n.clone(), Dir::FromRight),
        };
        Some(match base.as_str() {
            "none" | "cut" => Presentation::None,
            "fade" => Presentation::Fade,
            "slide" => Presentation::Slide(dir),
            "wipe" => Presentation::Wipe(dir),
            "flip" => Presentation::Flip(dir),
            "clock-wipe" | "clockwipe" => Presentation::ClockWipe,
            "iris" => Presentation::Iris,
            "cross-zoom" => Presentation::CrossZoom,
            "zoom-in-out" => Presentation::ZoomInOut,
            "push-cut" | "push" => Presentation::PushCut(dir),
            "swap" => Presentation::Swap(dir),
            "dissolve" => Presentation::Dissolve,
            "blur-slide" => Presentation::BlurSlide(dir),
            "linear-blur" | "blur" => Presentation::LinearBlur,
            "zoom-blur" => Presentation::ZoomBlur,
            "ripple" => Presentation::Ripple,
            "crosswarp" => Presentation::Crosswarp,
            "dreamy-zoom" => Presentation::DreamyZoom,
            "film-burn" => Presentation::FilmBurn,
            "book-flip" => Presentation::BookFlip,
            _ => return None,
        })
    }
    pub fn all() -> Vec<Presentation> {
        use Presentation::*;
        vec![
            None,
            Fade,
            Slide(Dir::FromRight),
            Wipe(Dir::FromLeft),
            Flip(Dir::FromRight),
            ClockWipe,
            Iris,
            CrossZoom,
            ZoomInOut,
            PushCut(Dir::FromRight),
            Swap(Dir::FromRight),
            Dissolve,
            BlurSlide(Dir::FromBottom),
            LinearBlur,
            ZoomBlur,
            Ripple,
            Crosswarp,
            DreamyZoom,
            FilmBurn,
            BookFlip,
        ]
    }
}

/// A transition between two consecutive sequences: presentation + timing
/// (Remotion's `linearTiming` / `springTiming`).
#[derive(Clone, Debug)]
pub struct Transition {
    pub presentation: Presentation,
    pub duration: f64,
    pub rate: Rate,
}

impl Transition {
    pub fn new(p: Presentation, duration: f64) -> Self {
        Self {
            presentation: p,
            duration,
            rate: Rate::InOutCubic,
        }
    }
    pub fn fade(d: f64) -> Self {
        Self::new(Presentation::Fade, d)
    }
    pub fn slide(dir: Dir, d: f64) -> Self {
        Self::new(Presentation::Slide(dir), d)
    }
    pub fn wipe(dir: Dir, d: f64) -> Self {
        Self::new(Presentation::Wipe(dir), d)
    }
    pub fn linear(mut self) -> Self {
        self.rate = Rate::Linear;
        self
    }
    /// Spring timing (Remotion's `springTiming`).
    pub fn spring(mut self, cfg: SpringConfig) -> Self {
        self.rate = Rate::Spring(cfg);
        self
    }
    pub fn rate(mut self, r: Rate) -> Self {
        self.rate = r;
        self
    }
}

fn paint(opacity: f32) -> PixmapPaint {
    PixmapPaint {
        opacity: opacity.clamp(0.0, 1.0),
        quality: tiny_skia::FilterQuality::Bilinear,
        ..Default::default()
    }
}

fn draw(out: &mut Pixmap, src: &Pixmap, tr: Transform, opacity: f32, mask: Option<&Mask>) {
    out.draw_pixmap(0, 0, src.as_ref(), &paint(opacity), tr, mask);
}

fn scale_about(s: f32, sy: f32, cx: f32, cy: f32) -> Transform {
    Transform::from_row(s, 0.0, 0.0, sy, cx - s * cx, cy - sy * cy)
}

fn rect_mask(w: u32, h: u32, x0: f32, y0: f32, x1: f32, y1: f32) -> Option<Mask> {
    let mut m = Mask::new(w, h)?;
    let r = Rect::from_ltrb(x0.max(0.0), y0.max(0.0), x1.min(w as f32), y1.min(h as f32))?;
    let path = PathBuilder::from_rect(r);
    m.fill_path(&path, FillRule::Winding, true, Transform::identity());
    Some(m)
}

fn hash(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

/// Smooth value noise on a lattice with cell size 1.
fn value_noise(x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let h = |i: f32, j: f32| hash(i as i32 as u32, j as i32 as u32);
    let a = h(x0, y0) + (h(x0 + 1.0, y0) - h(x0, y0)) * sx;
    let b = h(x0, y0 + 1.0) + (h(x0 + 1.0, y0 + 1.0) - h(x0, y0 + 1.0)) * sx;
    a + (b - a) * sy
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Composite outgoing `a` and incoming `b` at progress `p ∈ [0,1]` into `out`.
pub fn composite(pres: Presentation, a: &Pixmap, b: &Pixmap, p: f32, out: &mut Pixmap) {
    let (w, h) = (out.width(), out.height());
    let (wf, hf) = (w as f32, h as f32);
    let (cx, cy) = (wf / 2.0, hf / 2.0);
    let p = p.clamp(0.0, 1.0);
    let id = Transform::identity();
    out.fill(tiny_skia::Color::BLACK);
    use Presentation::*;
    match pres {
        None => draw(out, if p < 0.5 { a } else { b }, id, 1.0, Option::None),
        Fade => {
            draw(out, a, id, 1.0, Option::None);
            draw(out, b, id, p, Option::None);
        }
        Slide(d) | BlurSlide(d) => {
            let (dx, dy) = d.vec();
            draw(
                out,
                a,
                Transform::from_translate(-dx * wf * p, -dy * hf * p),
                1.0,
                Option::None,
            );
            draw(
                out,
                b,
                Transform::from_translate(dx * wf * (1.0 - p), dy * hf * (1.0 - p)),
                1.0,
                Option::None,
            );
            if matches!(pres, BlurSlide(_)) {
                blur(out, (std::f32::consts::PI * p).sin() * hf * 0.02);
            }
        }
        PushCut(d) => {
            let (dx, dy) = d.vec();
            let q = smoothstep(0.0, 1.0, p);
            draw(
                out,
                a,
                Transform::from_translate(-dx * wf * 0.35 * q, -dy * hf * 0.35 * q),
                1.0 - q * 0.6,
                Option::None,
            );
            draw(
                out,
                b,
                Transform::from_translate(dx * wf * (1.0 - q), dy * hf * (1.0 - q)),
                1.0,
                Option::None,
            );
        }
        Wipe(d) => {
            draw(out, a, id, 1.0, Option::None);
            let m = match d {
                Dir::FromLeft => rect_mask(w, h, 0.0, 0.0, wf * p, hf),
                Dir::FromRight => rect_mask(w, h, wf * (1.0 - p), 0.0, wf, hf),
                Dir::FromTop => rect_mask(w, h, 0.0, 0.0, wf, hf * p),
                Dir::FromBottom => rect_mask(w, h, 0.0, hf * (1.0 - p), wf, hf),
            };
            if let Some(m) = m {
                draw(out, b, id, 1.0, Some(&m));
            } else if p >= 1.0 {
                draw(out, b, id, 1.0, Option::None);
            }
        }
        Flip(d) => {
            let horizontal = matches!(d, Dir::FromLeft | Dir::FromRight);
            let (src, k) = if p < 0.5 {
                (a, (std::f32::consts::PI * p).cos())
            } else {
                (b, -(std::f32::consts::PI * p).cos())
            };
            let k = k.max(0.001);
            let tr = if horizontal {
                scale_about(k, 1.0, cx, cy)
            } else {
                scale_about(1.0, k, cx, cy)
            };
            draw(out, src, tr, 1.0, Option::None);
            Effect::Brightness(0.55 + 0.45 * k).apply(out);
        }
        ClockWipe => {
            draw(out, a, id, 1.0, Option::None);
            if p > 0.0 {
                let r = (wf * wf + hf * hf).sqrt();
                let mut pb = PathBuilder::new();
                pb.move_to(cx, cy);
                let steps = 64;
                for i in 0..=steps {
                    let ang = -std::f32::consts::FRAC_PI_2
                        + std::f32::consts::TAU * p * i as f32 / steps as f32;
                    pb.line_to(cx + r * ang.cos(), cy + r * ang.sin());
                }
                pb.close();
                if let (Some(path), Some(mut m)) = (pb.finish(), Mask::new(w, h)) {
                    m.fill_path(&path, FillRule::Winding, true, id);
                    draw(out, b, id, 1.0, Some(&m));
                }
            }
        }
        Iris => {
            draw(out, a, id, 1.0, Option::None);
            let r = p * (wf * wf + hf * hf).sqrt() / 2.0;
            if r > 0.5 {
                if let (Some(path), Some(mut m)) =
                    (PathBuilder::from_circle(cx, cy, r), Mask::new(w, h))
                {
                    m.fill_path(&path, FillRule::Winding, true, id);
                    draw(out, b, id, 1.0, Some(&m));
                }
            }
        }
        CrossZoom => {
            let q = smoothstep(0.0, 1.0, p);
            draw(
                out,
                a,
                scale_about(1.0 + 1.5 * q, 1.0 + 1.5 * q, cx, cy),
                1.0 - q,
                Option::None,
            );
            let s = 0.4 + 0.6 * q;
            draw(out, b, scale_about(s, s, cx, cy), q, Option::None);
            Effect::ZoomBlur {
                strength: 0.35 * (std::f32::consts::PI * p).sin(),
                cx: 0.5,
                cy: 0.5,
            }
            .apply(out);
        }
        ZoomInOut => {
            if p < 0.5 {
                let q = p * 2.0;
                draw(
                    out,
                    a,
                    scale_about(1.0 + q * 0.6, 1.0 + q * 0.6, cx, cy),
                    1.0 - q * 0.8,
                    Option::None,
                );
            } else {
                let q = (p - 0.5) * 2.0;
                let s = 1.6 - 0.6 * q;
                draw(
                    out,
                    b,
                    scale_about(s, s, cx, cy),
                    0.2 + 0.8 * q,
                    Option::None,
                );
            }
        }
        Swap(d) => {
            let (dx, dy) = d.vec();
            let s_a = 1.0 - 0.15 * (std::f32::consts::PI * p).sin();
            let ta = Transform::from_translate(-dx * wf * p, -dy * hf * p)
                .pre_concat(scale_about(s_a, s_a, cx, cy));
            let tb = Transform::from_translate(dx * wf * (1.0 - p), dy * hf * (1.0 - p))
                .pre_concat(scale_about(s_a, s_a, cx, cy));
            draw(out, a, ta, 1.0 - 0.4 * p, Option::None);
            draw(out, b, tb, 1.0, Option::None);
        }
        Dissolve => {
            let (ad, bd) = (a.data(), b.data());
            let wu = w as usize;
            out.data_mut()
                .par_chunks_mut(wu * 4)
                .enumerate()
                .for_each(|(y, row)| {
                    for x in 0..wu {
                        let n = hash(x as u32 / 3, y as u32 / 3);
                        let src = if n < p { bd } else { ad };
                        let i = (y * wu + x) * 4;
                        row[x * 4..x * 4 + 4].copy_from_slice(&src[i..i + 4]);
                    }
                });
        }
        LinearBlur | ZoomBlur => {
            draw(out, a, id, 1.0, Option::None);
            draw(out, b, id, smoothstep(0.2, 0.8, p), Option::None);
            let k = (std::f32::consts::PI * p).sin();
            if matches!(pres, LinearBlur) {
                blur(out, k * hf * 0.025);
            } else {
                Effect::ZoomBlur {
                    strength: 0.4 * k,
                    cx: 0.5,
                    cy: 0.5,
                }
                .apply(out);
            }
        }
        Ripple | Crosswarp | FilmBurn => {
            let (ad, bd) = (a.data(), b.data());
            let (wu, hu) = (w as usize, h as usize);
            out.data_mut()
                .par_chunks_mut(wu * 4)
                .enumerate()
                .for_each(|(y, row)| {
                    for x in 0..wu {
                        let (u, v) = (x as f32 / wf, y as f32 / hf);
                        let px: [f32; 4] = match pres {
                            Ripple => {
                                let (dx, dy) = (u - 0.5, (v - 0.5) * hf / wf);
                                let d = (dx * dx + dy * dy).sqrt().max(1e-4);
                                let amp = 0.03 * (std::f32::consts::PI * p).sin();
                                let off = (d * 60.0 - p * 30.0).sin() * amp;
                                let (sx, sy) = ((u + dx / d * off) * wf, (v + dy / d * off) * hf);
                                let sa = sample(ad, wu, hu, sx, sy);
                                let sb = sample(bd, wu, hu, sx, sy);
                                let k = smoothstep(0.3, 0.7, p);
                                [0, 1, 2, 3].map(|c| sa[c] + (sb[c] - sa[c]) * k)
                            }
                            Crosswarp => {
                                let xk = smoothstep(0.0, 1.0, p * 2.0 + u - 1.0);
                                let sa = sample(
                                    ad,
                                    wu,
                                    hu,
                                    ((u - 0.5) * (1.0 - xk) + 0.5) * wf,
                                    ((v - 0.5) * (1.0 - xk) + 0.5) * hf,
                                );
                                let sb = sample(
                                    bd,
                                    wu,
                                    hu,
                                    ((u - 0.5) * xk + 0.5) * wf,
                                    ((v - 0.5) * xk + 0.5) * hf,
                                );
                                [0, 1, 2, 3].map(|c| sa[c] + (sb[c] - sa[c]) * xk)
                            }
                            _ => {
                                // film burn: warm additive flare sweeping across
                                let sa = sample(ad, wu, hu, x as f32, y as f32);
                                let sb = sample(bd, wu, hu, x as f32, y as f32);
                                let k = smoothstep(0.35, 0.65, p);
                                let flare_pos = p * 1.6 - 0.3;
                                let n = (value_noise(u * 6.0, v * 4.0) * 0.6
                                    + value_noise(u * 17.0, v * 11.0) * 0.4)
                                    * 0.45;
                                let f = (1.0 - ((u * 0.7 + v * 0.3 - flare_pos).abs() * 3.0 - n))
                                    .clamp(0.0, 1.0)
                                    * (std::f32::consts::PI * p).sin();
                                let base = [0, 1, 2, 3].map(|c| sa[c] + (sb[c] - sa[c]) * k);
                                [
                                    (base[0] + f * 255.0).min(255.0),
                                    (base[1] + f * 150.0).min(255.0),
                                    (base[2] + f * 40.0).min(255.0),
                                    base[3],
                                ]
                            }
                        };
                        for c in 0..4 {
                            row[x * 4 + c] = px[c] as u8;
                        }
                    }
                });
        }
        DreamyZoom => {
            let q = smoothstep(0.0, 1.0, p);
            let rot = |deg: f32, s: f32| {
                Transform::from_translate(cx, cy)
                    .pre_concat(Transform::from_rotate(deg))
                    .pre_concat(Transform::from_scale(s, s))
                    .pre_concat(Transform::from_translate(-cx, -cy))
            };
            draw(out, a, rot(8.0 * q, 1.0 + 0.6 * q), 1.0 - q, Option::None);
            draw(
                out,
                b,
                rot(-8.0 * (1.0 - q), 1.0 + 0.6 * (1.0 - q)),
                q,
                Option::None,
            );
            Effect::Brightness(1.0 + 0.5 * (std::f32::consts::PI * p).sin()).apply(out);
        }
        BookFlip => {
            if p < 0.5 {
                draw(out, b, id, 1.0, Option::None);
                if let Some(m) = rect_mask(w, h, 0.0, 0.0, cx, hf) {
                    draw(out, a, id, 1.0, Some(&m));
                }
                let k = (std::f32::consts::PI * p).cos().max(0.001);
                if let Some(m) = rect_mask(w, h, cx, 0.0, cx + cx * k, hf) {
                    let tr = scale_about(k, 1.0, cx, cy);
                    let mut page = Pixmap::new(w, h).unwrap();
                    draw(&mut page, a, tr, 1.0, Some(&m));
                    Effect::Brightness(0.6 + 0.4 * k).apply(&mut page);
                    draw(out, &page, id, 1.0, Option::None);
                }
            } else {
                draw(out, b, id, 1.0, Option::None);
                if let Some(m) = rect_mask(w, h, 0.0, 0.0, cx, hf) {
                    draw(out, a, id, 1.0, Some(&m));
                }
                let k = (-(std::f32::consts::PI * p).cos()).max(0.001);
                if let Some(m) = rect_mask(w, h, cx - cx * k, 0.0, cx, hf) {
                    let tr = scale_about(k, 1.0, cx, cy);
                    let mut page = Pixmap::new(w, h).unwrap();
                    draw(&mut page, b, tr, 1.0, Some(&m));
                    Effect::Brightness(0.6 + 0.4 * k).apply(&mut page);
                    draw(out, &page, id, 1.0, Option::None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_hit_endpoints() {
        let mut a = Pixmap::new(48, 27).unwrap();
        a.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        let mut b = Pixmap::new(48, 27).unwrap();
        b.fill(tiny_skia::Color::from_rgba8(0, 0, 255, 255));
        for pres in Presentation::all() {
            let mut out = Pixmap::new(48, 27).unwrap();
            composite(pres, &a, &b, 1.0, &mut out);
            let px = out.pixel(24, 13).unwrap();
            assert!(px.blue() > 150 && px.red() < 100, "{pres:?} end {px:?}");
            composite(pres, &a, &b, 0.0, &mut out);
            let px = out.pixel(24, 13).unwrap();
            assert!(px.red() > 150, "{pres:?} start {px:?}");
            composite(pres, &a, &b, 0.5, &mut out);
        }
        assert_eq!(
            Presentation::parse("slide:from-left"),
            Some(Presentation::Slide(Dir::FromLeft))
        );
    }
}
