//! Pixel effects. `blur` powers glow/shadows; [`Effect`] is a library of full-frame
//! post-processing effects modelled on `@remotion/effects` (colour grading,
//! blur/bloom, vignette, pixelate, grain, scanlines, halftone, chromatic
//! aberration, zoom blur, mirror, wave, fisheye, emboss, sharpen…).

use crate::color::Color;
use rayon::prelude::*;
use tiny_skia::Pixmap;

/// Approximate gaussian blur with standard deviation ≈ `sigma` pixels.
pub fn blur(pm: &mut Pixmap, sigma: f32) {
    if sigma < 0.5 {
        return;
    }
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let boxes = boxes_for_gauss(sigma, 3);
    let data = pm.data_mut();
    let mut tmp = vec![0u8; data.len()];
    for r in boxes {
        box_blur_h(data, &mut tmp, w, h, r);
        box_blur_v(&tmp, data, w, h, r);
    }
}

fn boxes_for_gauss(sigma: f32, n: usize) -> Vec<usize> {
    let w_ideal = ((12.0 * sigma * sigma / n as f32) + 1.0).sqrt();
    let mut wl = w_ideal.floor() as i32;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let m_ideal = (12.0 * sigma * sigma
        - (n as f32) * (wl * wl) as f32
        - 4.0 * n as f32 * wl as f32
        - 3.0 * n as f32)
        / (-4.0 * wl as f32 - 4.0);
    let m = m_ideal.round() as usize;
    (0..n)
        .map(|i| (((if i < m { wl } else { wu }) - 1) / 2).max(0) as usize)
        .collect()
}

fn box_blur_h(src: &[u8], dst: &mut [u8], w: usize, _h: usize, r: usize) {
    if r == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let div = (2 * r + 1) as u32;
    dst.par_chunks_mut(w * 4)
        .zip(src.par_chunks(w * 4))
        .for_each(|(drow, srow)| {
            for c in 0..4 {
                let at = |x: isize| -> u32 {
                    let x = x.clamp(0, w as isize - 1) as usize;
                    srow[x * 4 + c] as u32
                };
                let mut acc: u32 = 0;
                for k in -(r as isize)..=(r as isize) {
                    acc += at(k);
                }
                for x in 0..w {
                    drow[x * 4 + c] = ((acc + div / 2) / div) as u8;
                    acc += at(x as isize + r as isize + 1);
                    acc -= at(x as isize - r as isize);
                }
            }
        });
}

fn box_blur_v(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize) {
    if r == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let div = (2 * r + 1) as u32;
    for x in 0..w {
        for c in 0..4 {
            let at = |y: isize| -> u32 {
                let y = y.clamp(0, h as isize - 1) as usize;
                src[(y * w + x) * 4 + c] as u32
            };
            let mut acc: u32 = 0;
            for k in -(r as isize)..=(r as isize) {
                acc += at(k);
            }
            for y in 0..h {
                dst[(y * w + x) * 4 + c] = ((acc + div / 2) / div) as u8;
                acc += at(y as isize + r as isize + 1);
                acc -= at(y as isize - r as isize);
            }
        }
    }
}

/// Bilinear sample of a premultiplied RGBA buffer (clamped at the edges).
#[inline]
pub fn sample(data: &[u8], w: usize, h: usize, x: f32, y: f32) -> [f32; 4] {
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let px = |xx: usize, yy: usize, c: usize| data[(yy * w + xx) * 4 + c] as f32;
    let mut out = [0.0; 4];
    for (c, o) in out.iter_mut().enumerate() {
        let a = px(x0, y0, c) + (px(x1, y0, c) - px(x0, y0, c)) * fx;
        let b = px(x0, y1, c) + (px(x1, y1, c) - px(x0, y1, c)) * fx;
        *o = a + (b - a) * fy;
    }
    out
}

fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h =
        x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

/// A full-frame post-processing effect. Distances are fractions of the frame height,
/// so effects look the same at every resolution.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Multiply RGB (1 = unchanged).
    Brightness(f32),
    Contrast(f32),
    Saturation(f32),
    /// Rotate hue by degrees.
    HueRotate(f32),
    Grayscale(f32),
    Invert(f32),
    Sepia(f32),
    /// Map luminance between two colours.
    Duotone(Color, Color),
    Tint(Color, f32),
    /// Exposure in stops.
    Exposure(f32),
    Levels {
        black: f32,
        white: f32,
        gamma: f32,
    },
    Posterize(u32),
    Vignette {
        strength: f32,
        radius: f32,
    },
    /// Block size as a fraction of the frame height.
    Pixelate(f32),
    /// Animated film grain.
    Grain {
        amount: f32,
        seed: u32,
    },
    Scanlines {
        spacing: f32,
        strength: f32,
    },
    Halftone {
        cell: f32,
    },
    ChromaticAberration(f32),
    Bloom {
        threshold: f32,
        radius: f32,
        strength: f32,
    },
    Blur(f32),
    ZoomBlur {
        strength: f32,
        cx: f32,
        cy: f32,
    },
    /// Mirror the left half onto the right (`true`) or top onto bottom (`false`).
    Mirror(bool),
    Wave {
        amplitude: f32,
        frequency: f32,
        phase: f32,
    },
    Fisheye(f32),
    Emboss,
    Sharpen(f32),
    /// Cinematic bars for the given aspect ratio (e.g. 2.39).
    Letterbox(f32),
    /// Fade the whole frame towards a colour (0..1).
    FadeTo(Color, f32),
}

impl Effect {
    /// Apply the effect in place.
    pub fn apply(&self, pm: &mut Pixmap) {
        let (w, h) = (pm.width() as usize, pm.height() as usize);
        let hf = h as f32;
        match self {
            Effect::Brightness(k) => color_map(pm, |c| [c[0] * k, c[1] * k, c[2] * k]),
            Effect::Contrast(k) => color_map(pm, |c| c.map(|v| (v - 0.5) * k + 0.5)),
            Effect::Saturation(k) => color_map(pm, |c| {
                let l = luma(c);
                c.map(|v| l + (v - l) * k)
            }),
            Effect::HueRotate(deg) => {
                let a = deg.to_radians();
                let (cs, sn) = (a.cos(), a.sin());
                let m = [
                    [
                        0.213 + cs * 0.787 - sn * 0.213,
                        0.715 - cs * 0.715 - sn * 0.715,
                        0.072 - cs * 0.072 + sn * 0.928,
                    ],
                    [
                        0.213 - cs * 0.213 + sn * 0.143,
                        0.715 + cs * 0.285 + sn * 0.140,
                        0.072 - cs * 0.072 - sn * 0.283,
                    ],
                    [
                        0.213 - cs * 0.213 - sn * 0.787,
                        0.715 - cs * 0.715 + sn * 0.715,
                        0.072 + cs * 0.928 + sn * 0.072,
                    ],
                ];
                color_map(pm, |c| {
                    [0, 1, 2].map(|i| m[i][0] * c[0] + m[i][1] * c[1] + m[i][2] * c[2])
                })
            }
            Effect::Grayscale(k) => color_map(pm, |c| {
                let l = luma(c);
                c.map(|v| v + (l - v) * k)
            }),
            Effect::Invert(k) => color_map(pm, |c| c.map(|v| v + (1.0 - 2.0 * v) * k)),
            Effect::Sepia(k) => color_map(pm, |c| {
                let s = [
                    0.393 * c[0] + 0.769 * c[1] + 0.189 * c[2],
                    0.349 * c[0] + 0.686 * c[1] + 0.168 * c[2],
                    0.272 * c[0] + 0.534 * c[1] + 0.131 * c[2],
                ];
                [0, 1, 2].map(|i| c[i] + (s[i] - c[i]) * k)
            }),
            Effect::Duotone(a, b) => {
                let (a, b) = (*a, *b);
                color_map(pm, move |c| {
                    let l = luma(c);
                    [
                        a.r + (b.r - a.r) * l,
                        a.g + (b.g - a.g) * l,
                        a.b + (b.b - a.b) * l,
                    ]
                })
            }
            Effect::Tint(col, k) => {
                let col = *col;
                color_map(pm, move |c| {
                    let l = luma(c);
                    [
                        c[0] + (l * col.r - c[0]) * k,
                        c[1] + (l * col.g - c[1]) * k,
                        c[2] + (l * col.b - c[2]) * k,
                    ]
                })
            }
            Effect::Exposure(ev) => {
                let k = 2f32.powf(*ev);
                color_map(pm, move |c| c.map(|v| v * k))
            }
            Effect::Levels {
                black,
                white,
                gamma,
            } => {
                let (b, wt, g) = (*black, *white, *gamma);
                color_map(pm, move |c| {
                    c.map(|v| {
                        ((v - b) / (wt - b).max(1e-4))
                            .clamp(0.0, 1.0)
                            .powf(1.0 / g.max(1e-3))
                    })
                })
            }
            Effect::Posterize(n) => {
                let n = (*n).max(2) as f32 - 1.0;
                color_map(pm, move |c| c.map(|v| (v * n).round() / n))
            }
            Effect::Vignette { strength, radius } => {
                let (s, r) = (*strength, *radius);
                let (cx, cy) = (w as f32 / 2.0, hf / 2.0);
                let norm = (cx * cx + cy * cy).sqrt();
                pm.data_mut()
                    .par_chunks_mut(w * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        for x in 0..w {
                            let d = (((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt()
                                / norm
                                - r)
                                .max(0.0)
                                / (1.0 - r).max(1e-3);
                            let k = 1.0 - s * d * d * (3.0 - 2.0 * d.min(1.0));
                            for c in 0..3 {
                                row[x * 4 + c] = (row[x * 4 + c] as f32 * k.max(0.0)) as u8;
                            }
                        }
                    });
            }
            Effect::Pixelate(size) => {
                let b = ((size * hf).round() as usize).max(1);
                let src = pm.data().to_vec();
                let data = pm.data_mut();
                for by in (0..h).step_by(b) {
                    for bx in (0..w).step_by(b) {
                        let (cx, cy) = ((bx + b / 2).min(w - 1), (by + b / 2).min(h - 1));
                        let i = (cy * w + cx) * 4;
                        let px = [src[i], src[i + 1], src[i + 2], src[i + 3]];
                        for y in by..(by + b).min(h) {
                            for x in bx..(bx + b).min(w) {
                                data[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&px);
                            }
                        }
                    }
                }
            }
            Effect::Grain { amount, seed } => {
                let (a, seed) = (*amount, *seed);
                pm.data_mut()
                    .par_chunks_mut(w * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        for x in 0..w {
                            let n = (hash(x as u32, y as u32, seed) - 0.5) * 2.0 * a * 255.0;
                            let al = row[x * 4 + 3] as f32;
                            for c in 0..3 {
                                row[x * 4 + c] = (row[x * 4 + c] as f32 + n).clamp(0.0, al) as u8;
                            }
                        }
                    });
            }
            Effect::Scanlines { spacing, strength } => {
                let sp = (spacing * hf).max(2.0);
                let st = *strength;
                pm.data_mut()
                    .par_chunks_mut(w * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        let k =
                            1.0 - st * (0.5 + 0.5 * (std::f32::consts::TAU * y as f32 / sp).cos());
                        for x in 0..w {
                            for c in 0..3 {
                                row[x * 4 + c] = (row[x * 4 + c] as f32 * k) as u8;
                            }
                        }
                    });
            }
            Effect::Halftone { cell } => {
                let cs = (cell * hf).max(3.0);
                let src = pm.data().to_vec();
                pm.data_mut()
                    .par_chunks_mut(w * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        for x in 0..w {
                            let (gx, gy) = ((x as f32 / cs).floor(), (y as f32 / cs).floor());
                            let (ccx, ccy) = ((gx + 0.5) * cs, (gy + 0.5) * cs);
                            let s = sample(&src, w, h, ccx, ccy);
                            let l = (0.2126 * s[0] + 0.7152 * s[1] + 0.0722 * s[2]) / 255.0;
                            let r = l.sqrt() * cs * 0.6;
                            let d = ((x as f32 - ccx).powi(2) + (y as f32 - ccy).powi(2)).sqrt();
                            let cov = (r - d + 0.5).clamp(0.0, 1.0);
                            for c in 0..3 {
                                row[x * 4 + c] = (s[c] / l.max(1e-3) * cov).min(255.0) as u8;
                            }
                            row[x * 4 + 3] = 255;
                        }
                    });
            }
            Effect::ChromaticAberration(off) => {
                let o = off * hf;
                let src = pm.data().to_vec();
                let (cx, cy) = (w as f32 / 2.0, hf / 2.0);
                pm.data_mut()
                    .par_chunks_mut(w * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        for x in 0..w {
                            let (dx, dy) = ((x as f32 - cx) / cx, (y as f32 - cy) / cy);
                            let r = sample(&src, w, h, x as f32 + dx * o, y as f32 + dy * o);
                            let b = sample(&src, w, h, x as f32 - dx * o, y as f32 - dy * o);
                            row[x * 4] = r[0] as u8;
                            row[x * 4 + 2] = b[2] as u8;
                        }
                    });
            }
            Effect::Bloom {
                threshold,
                radius,
                strength,
            } => {
                let mut bright = pm.clone();
                let th = threshold * 255.0;
                for px in bright.data_mut().as_chunks_mut::<4>().0 {
                    let l = 0.2126 * px[0] as f32 + 0.7152 * px[1] as f32 + 0.0722 * px[2] as f32;
                    let k = ((l - th) / (255.0 - th).max(1.0)).clamp(0.0, 1.0);
                    for c in px.iter_mut() {
                        *c = (*c as f32 * k) as u8;
                    }
                }
                blur(&mut bright, radius * hf);
                let paint = tiny_skia::PixmapPaint {
                    opacity: strength.clamp(0.0, 1.0),
                    blend_mode: tiny_skia::BlendMode::Screen,
                    quality: tiny_skia::FilterQuality::Nearest,
                };
                pm.draw_pixmap(
                    0,
                    0,
                    bright.as_ref(),
                    &paint,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
            Effect::Blur(sigma) => blur(pm, sigma * hf),
            Effect::ZoomBlur { strength, cx, cy } => {
                let src = pm.data().to_vec();
                let (cxp, cyp) = (cx * w as f32, cy * hf);
                let n = 12;
                let st = *strength;
                pm.data_mut()
                    .par_chunks_mut(w * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        for x in 0..w {
                            let mut acc = [0.0f32; 4];
                            for i in 0..n {
                                let k = 1.0 - st * i as f32 / n as f32;
                                let s = sample(
                                    &src,
                                    w,
                                    h,
                                    cxp + (x as f32 - cxp) * k,
                                    cyp + (y as f32 - cyp) * k,
                                );
                                for c in 0..4 {
                                    acc[c] += s[c];
                                }
                            }
                            for c in 0..4 {
                                row[x * 4 + c] = (acc[c] / n as f32) as u8;
                            }
                        }
                    });
            }
            Effect::Mirror(horizontal) => {
                let data = pm.data_mut();
                if *horizontal {
                    for y in 0..h {
                        for x in w / 2..w {
                            let sx = w - 1 - x;
                            for c in 0..4 {
                                data[(y * w + x) * 4 + c] = data[(y * w + sx) * 4 + c];
                            }
                        }
                    }
                } else {
                    for y in h / 2..h {
                        let sy = h - 1 - y;
                        let (a, b) = data.split_at_mut(y * w * 4);
                        b[..w * 4].copy_from_slice(&a[sy * w * 4..sy * w * 4 + w * 4]);
                    }
                }
            }
            Effect::Wave {
                amplitude,
                frequency,
                phase,
            } => {
                let src = pm.data().to_vec();
                let (a, f, p) = (amplitude * hf, *frequency, *phase);
                pm.data_mut()
                    .par_chunks_mut(w * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        let dx = a * (std::f32::consts::TAU * f * y as f32 / hf + p).sin();
                        for x in 0..w {
                            let s = sample(&src, w, h, x as f32 + dx, y as f32);
                            for c in 0..4 {
                                row[x * 4 + c] = s[c] as u8;
                            }
                        }
                    });
            }
            Effect::Fisheye(k) => {
                let src = pm.data().to_vec();
                let (cx, cy) = (w as f32 / 2.0, hf / 2.0);
                let k = *k;
                pm.data_mut()
                    .par_chunks_mut(w * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        for x in 0..w {
                            let (nx, ny) = ((x as f32 - cx) / cy, (y as f32 - cy) / cy);
                            let r2 = nx * nx + ny * ny;
                            let f = 1.0 + k * r2;
                            let s = sample(&src, w, h, cx + nx * f * cy, cy + ny * f * cy);
                            for c in 0..4 {
                                row[x * 4 + c] = s[c] as u8;
                            }
                        }
                    });
            }
            Effect::Emboss => convolve(pm, [[-2.0, -1.0, 0.0], [-1.0, 1.0, 1.0], [0.0, 1.0, 2.0]]),
            Effect::Sharpen(k) => {
                let k = *k;
                convolve(
                    pm,
                    [[0.0, -k, 0.0], [-k, 1.0 + 4.0 * k, -k], [0.0, -k, 0.0]],
                )
            }
            Effect::Letterbox(ratio) => {
                let bar = ((hf - w as f32 / ratio.max(0.1)) / 2.0).max(0.0) as usize;
                let data = pm.data_mut();
                for y in (0..bar).chain(h.saturating_sub(bar)..h) {
                    for x in 0..w {
                        data[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&[0, 0, 0, 255]);
                    }
                }
            }
            Effect::FadeTo(col, k) => {
                let (col, k) = (*col, *k);
                color_map(pm, move |c| {
                    [
                        c[0] + (col.r - c[0]) * k,
                        c[1] + (col.g - c[1]) * k,
                        c[2] + (col.b - c[2]) * k,
                    ]
                })
            }
        }
    }
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// Apply a colour function to every pixel (straight RGB in 0..1).
fn color_map(pm: &mut Pixmap, f: impl Fn([f32; 3]) -> [f32; 3] + Sync) {
    let w = pm.width() as usize;
    pm.data_mut().par_chunks_mut(w * 4).for_each(|row| {
        for px in row.as_chunks_mut::<4>().0 {
            let a = px[3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let c = [
                px[0] as f32 / 255.0 / a,
                px[1] as f32 / 255.0 / a,
                px[2] as f32 / 255.0 / a,
            ];
            let o = f(c);
            for i in 0..3 {
                px[i] = (o[i].clamp(0.0, 1.0) * a * 255.0).round() as u8;
            }
        }
    });
}

fn convolve(pm: &mut Pixmap, k: [[f32; 3]; 3]) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let src = pm.data().to_vec();
    pm.data_mut()
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w {
                let mut acc = [0.0f32; 3];
                for (j, kr) in k.iter().enumerate() {
                    for (i, kv) in kr.iter().enumerate() {
                        let sx = (x as isize + i as isize - 1).clamp(0, w as isize - 1) as usize;
                        let sy = (y as isize + j as isize - 1).clamp(0, h as isize - 1) as usize;
                        for c in 0..3 {
                            acc[c] += src[(sy * w + sx) * 4 + c] as f32 * kv;
                        }
                    }
                }
                let a = row[x * 4 + 3] as f32;
                for c in 0..3 {
                    row[x * 4 + c] = acc[c].clamp(0.0, a) as u8;
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::*;

    #[test]
    fn blur_spreads_energy() {
        let mut pm = Pixmap::new(41, 41).unwrap();
        let i = (20 * 41 + 20) * 4;
        pm.data_mut()[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
        blur(&mut pm, 3.0);
        let d = pm.data();
        assert!(d[i + 3] < 255);
        assert!(d[((20 * 41) + 23) * 4 + 3] > 0);
    }

    #[test]
    fn every_effect_runs() {
        let effects = vec![
            Effect::Brightness(1.2),
            Effect::Contrast(1.3),
            Effect::Saturation(0.0),
            Effect::HueRotate(90.0),
            Effect::Grayscale(1.0),
            Effect::Invert(1.0),
            Effect::Sepia(1.0),
            Effect::Duotone(BLUE_E, YELLOW),
            Effect::Tint(RED, 0.5),
            Effect::Exposure(0.5),
            Effect::Levels {
                black: 0.1,
                white: 0.9,
                gamma: 1.2,
            },
            Effect::Posterize(4),
            Effect::Vignette {
                strength: 0.6,
                radius: 0.3,
            },
            Effect::Pixelate(0.05),
            Effect::Grain {
                amount: 0.1,
                seed: 1,
            },
            Effect::Scanlines {
                spacing: 0.01,
                strength: 0.3,
            },
            Effect::Halftone { cell: 0.03 },
            Effect::ChromaticAberration(0.005),
            Effect::Bloom {
                threshold: 0.6,
                radius: 0.02,
                strength: 0.8,
            },
            Effect::Blur(0.01),
            Effect::ZoomBlur {
                strength: 0.2,
                cx: 0.5,
                cy: 0.5,
            },
            Effect::Mirror(true),
            Effect::Mirror(false),
            Effect::Wave {
                amplitude: 0.01,
                frequency: 3.0,
                phase: 0.0,
            },
            Effect::Fisheye(0.3),
            Effect::Emboss,
            Effect::Sharpen(0.5),
            Effect::Letterbox(2.39),
            Effect::FadeTo(BLACK, 0.5),
        ];
        for e in effects {
            let mut pm = Pixmap::new(64, 36).unwrap();
            pm.fill(tiny_skia::Color::from_rgba8(200, 80, 40, 255));
            e.apply(&mut pm);
            assert!(pm.data().iter().any(|v| *v > 0), "{e:?}");
        }
        let mut pm = Pixmap::new(4, 4).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        Effect::Grayscale(1.0).apply(&mut pm);
        let p = pm.pixel(1, 1).unwrap();
        assert!(p.red() == p.green() && p.green() == p.blue());
    }
}
