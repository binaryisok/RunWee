//! Stylize filters (phase A5): ordered dithering, Kuwahara painterly filter,
//! pixel sorting, ASCII art and a CRT monitor. Exposed as [`Effect`] variants
//! (`Effect::Dither`, `Effect::Kuwahara`, `Effect::PixelSort`, `Effect::Ascii`,
//! `Effect::Crt`) so they work everywhere post effects do.
use rayon::prelude::*;
use tiny_skia::Pixmap;

use crate::color::Color;
use crate::effects::Effect;

/// 8×8 Bayer threshold matrix (values 0..63).
pub const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Density ramp used by the ASCII filter, darkest first.
pub const ASCII_RAMP: &str = " .:-=+*#%@";

const GLYPHS: [[&str; 7]; 10] = [
    [
        "     ", "     ", "     ", "     ", "     ", "     ", "     ",
    ],
    [
        "     ", "     ", "     ", "     ", "     ", " ##  ", " ##  ",
    ],
    [
        "     ", " ##  ", " ##  ", "     ", " ##  ", " ##  ", "     ",
    ],
    [
        "     ", "     ", "     ", "#### ", "     ", "     ", "     ",
    ],
    [
        "     ", "     ", "#####", "     ", "#####", "     ", "     ",
    ],
    [
        "     ", "  #  ", "  #  ", "#####", "  #  ", "  #  ", "     ",
    ],
    [
        "     ", "# # #", " ### ", "#####", " ### ", "# # #", "     ",
    ],
    [
        " # # ", " # # ", "#####", " # # ", "#####", " # # ", " # # ",
    ],
    [
        "##  #", "##  #", "   # ", "  #  ", " #   ", "#  ##", "#  ##",
    ],
    [
        " ### ", "#   #", "# ###", "# # #", "# ###", "#    ", " ####",
    ],
];

fn px(d: &[u8], i: usize) -> [f32; 3] {
    let a = d[i + 3] as f32 / 255.0;
    if a <= 0.0 {
        return [0.0; 3];
    }
    [
        d[i] as f32 / 255.0 / a,
        d[i + 1] as f32 / 255.0 / a,
        d[i + 2] as f32 / 255.0 / a,
    ]
}
fn put(d: &mut [u8], i: usize, c: [f32; 3]) {
    // writes an opaque pixel
    d[i] = (c[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    d[i + 1] = (c[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    d[i + 2] = (c[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    d[i + 3] = 255;
}
fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}
fn rgbf(c: Color) -> [f32; 3] {
    [c.r, c.g, c.b]
}

/// Apply a stylize effect. Returns `false` for effects not handled here.
pub fn apply(e: &Effect, pm: &mut Pixmap) -> bool {
    match e {
        Effect::Dither {
            levels,
            cell,
            palette,
        } => dither(pm, *levels, *cell, palette),
        Effect::Kuwahara(r) => kuwahara(pm, *r),
        Effect::PixelSort {
            threshold,
            vertical,
        } => pixel_sort(pm, *threshold, *vertical),
        Effect::Ascii { cell, color } => ascii(pm, *cell, *color),
        Effect::Crt {
            curvature,
            scanlines,
        } => crt(pm, *curvature, *scanlines),
        _ => return false,
    }
    true
}

/// Ordered (Bayer) dithering in blocks of `cell` × frame height pixels. With a
/// non-empty `palette` colours snap to the nearest entry, else each channel is
/// quantised to `levels` steps.
pub fn dither(pm: &mut Pixmap, levels: u8, cell: f32, palette: &[Color]) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let bs = ((cell * h as f32).round() as usize).max(1);
    let src = pm.data().to_vec();
    let pal: Vec<[f32; 3]> = palette.iter().map(|c| rgbf(*c)).collect();
    let lv = levels.max(2) as f32;
    pm.data_mut()
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let by = y / bs * bs + bs / 2;
            for x in 0..w {
                let bx = (x / bs * bs + bs / 2).min(w - 1);
                let c = px(&src, (by.min(h - 1) * w + bx) * 4);
                let th = (BAYER8[(y / bs) % 8][(x / bs) % 8] as f32 + 0.5) / 64.0 - 0.5;
                let out = if pal.is_empty() {
                    c.map(|v| {
                        ((v + th / (lv - 1.0)) * (lv - 1.0))
                            .round()
                            .clamp(0.0, lv - 1.0)
                            / (lv - 1.0)
                    })
                } else {
                    let spread = 0.35;
                    let cc = c.map(|v| v + th * spread);
                    *pal.iter()
                        .min_by(|a, b| {
                            let da: f32 = (0..3).map(|k| (a[k] - cc[k]).powi(2)).sum();
                            let db: f32 = (0..3).map(|k| (b[k] - cc[k]).powi(2)).sum();
                            da.total_cmp(&db)
                        })
                        .unwrap()
                };
                put(row, x * 4, out);
            }
        });
}

/// Kuwahara filter (painterly look). `radius` is a fraction of the frame height.
/// Uses summed-area tables so cost is independent of the radius.
pub fn kuwahara(pm: &mut Pixmap, radius: f32) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let r = ((radius * h as f32).round() as i64).max(1);
    let src = pm.data();
    // SAT of r,g,b,luma² with a zero border: (w+1)×(h+1)
    let sw = w + 1;
    let mut sat = vec![[0f64; 5]; sw * (h + 1)];
    for y in 0..h {
        let mut acc = [0f64; 5];
        for x in 0..w {
            let c = px(src, (y * w + x) * 4);
            let l = luma(c) as f64;
            acc[0] += c[0] as f64;
            acc[1] += c[1] as f64;
            acc[2] += c[2] as f64;
            acc[3] += l;
            acc[4] += l * l;
            let up = sat[y * sw + x + 1];
            sat[(y + 1) * sw + x + 1] = [
                up[0] + acc[0],
                up[1] + acc[1],
                up[2] + acc[2],
                up[3] + acc[3],
                up[4] + acc[4],
            ];
        }
    }
    let rect = |x0: i64, y0: i64, x1: i64, y1: i64| -> ([f64; 5], f64) {
        let (x0, y0) = (
            x0.clamp(0, w as i64) as usize,
            y0.clamp(0, h as i64) as usize,
        );
        let (x1, y1) = (
            x1.clamp(0, w as i64) as usize,
            y1.clamp(0, h as i64) as usize,
        );
        let n = ((x1 - x0) * (y1 - y0)).max(1) as f64;
        let (a, b, c, d) = (
            sat[y0 * sw + x0],
            sat[y0 * sw + x1],
            sat[y1 * sw + x0],
            sat[y1 * sw + x1],
        );
        let mut s = [0f64; 5];
        for k in 0..5 {
            s[k] = d[k] - b[k] - c[k] + a[k];
        }
        (s, n)
    };
    pm.data_mut()
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let y = y as i64;
            for x in 0..w as i64 {
                let mut best = (f64::INFINITY, [0f32; 3]);
                for (dx0, dy0) in [(-r, -r), (0, -r), (-r, 0), (0, 0)] {
                    let (s, n) = rect(x + dx0, y + dy0, x + dx0 + r + 1, y + dy0 + r + 1);
                    let mean_l = s[3] / n;
                    let var = s[4] / n - mean_l * mean_l;
                    if var < best.0 {
                        best = (
                            var,
                            [(s[0] / n) as f32, (s[1] / n) as f32, (s[2] / n) as f32],
                        );
                    }
                }
                put(row, x as usize * 4, best.1);
            }
        });
}

/// Pixel sorting: runs of pixels brighter than `threshold` are sorted by
/// brightness along rows (or columns when `vertical`).
pub fn pixel_sort(pm: &mut Pixmap, threshold: f32, vertical: bool) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let src = pm.data().to_vec();
    let (lines, len) = if vertical { (w, h) } else { (h, w) };
    let idx = |line: usize, k: usize| {
        if vertical {
            (k * w + line) * 4
        } else {
            (line * w + k) * 4
        }
    };
    let sorted: Vec<Vec<[u8; 4]>> = (0..lines)
        .into_par_iter()
        .map(|line| {
            let mut out: Vec<[u8; 4]> = (0..len)
                .map(|k| {
                    let i = idx(line, k);
                    [src[i], src[i + 1], src[i + 2], src[i + 3]]
                })
                .collect();
            let l = |p: &[u8; 4]| {
                (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0
            };
            let mut k = 0;
            while k < len {
                if l(&out[k]) > threshold {
                    let s = k;
                    while k < len && l(&out[k]) > threshold {
                        k += 1;
                    }
                    out[s..k].sort_by(|a, b| l(a).total_cmp(&l(b)));
                } else {
                    k += 1;
                }
            }
            out
        })
        .collect();
    let d = pm.data_mut();
    for (line, px) in sorted.iter().enumerate() {
        for (k, p) in px.iter().enumerate() {
            let i = idx(line, k);
            d[i..i + 4].copy_from_slice(p);
        }
    }
}

/// ASCII art: each cell (fraction of the height) becomes a glyph from
/// [`ASCII_RAMP`] by brightness, tinted by the cell colour (or `color`).
pub fn ascii(pm: &mut Pixmap, cell: f32, color: Option<Color>) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let ch = ((cell * h as f32).round() as usize).max(7);
    let cw = (ch * 5 / 7).max(5);
    let src = pm.data().to_vec();
    let fixed = color.map(rgbf);
    pm.data_mut()
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let cy = y / ch;
            let gy = ((y % ch) * 7 / ch).min(6);
            for x in 0..w {
                let cx = x / cw;
                // cell average from a 3×3 sample grid
                let mut acc = [0f32; 3];
                for sy in 0..3 {
                    for sx in 0..3 {
                        let px_ = (cx * cw + (sx * 2 + 1) * cw / 6).min(w - 1);
                        let py_ = (cy * ch + (sy * 2 + 1) * ch / 6).min(h - 1);
                        let c = px(&src, (py_ * w + px_) * 4);
                        for k in 0..3 {
                            acc[k] += c[k] / 9.0;
                        }
                    }
                }
                let l = luma(acc).clamp(0.0, 1.0).powf(0.8);
                let g = ((l * (GLYPHS.len() as f32 - 0.01)) as usize).min(GLYPHS.len() - 1);
                let gx = ((x % cw) * 5 / cw).min(4);
                let on = GLYPHS[g][gy].as_bytes()[gx] == b'#';
                let tint = fixed.unwrap_or_else(|| {
                    let m = acc[0].max(acc[1]).max(acc[2]).max(1e-3);
                    acc.map(|v| (v / m) * 0.85 + 0.15)
                });
                put(row, x * 4, if on { tint } else { [0.0; 3] });
            }
        });
}

/// CRT monitor: barrel distortion, RGB aperture mask, scanlines and vignette.
pub fn crt(pm: &mut Pixmap, curvature: f32, scanlines: f32) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let src = pm.data().to_vec();
    let period = (h as f32 / 240.0).max(2.0);
    pm.data_mut()
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w {
                let u = (x as f32 + 0.5) / w as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / h as f32 * 2.0 - 1.0;
                let r2 = u * u + v * v;
                let k = 1.0 + curvature * r2;
                let (su, sv) = (u * k, v * k);
                if su.abs() > 1.0 || sv.abs() > 1.0 {
                    put(row, x * 4, [0.0; 3]);
                    continue;
                }
                let sx = (((su + 1.0) * 0.5 * w as f32) as usize).min(w - 1);
                let sy = (((sv + 1.0) * 0.5 * h as f32) as usize).min(h - 1);
                let mut c = px(&src, (sy * w + sx) * 4);
                // slight horizontal colour bleed
                let c2 = px(&src, (sy * w + (sx + 1).min(w - 1)) * 4);
                c = [c[0] * 0.7 + c2[0] * 0.3, c[1], c[2] * 0.8 + c2[2] * 0.2];
                let scan = 1.0
                    - scanlines * 0.5 * (1.0 + (std::f32::consts::TAU * sy as f32 / period).cos());
                let mut mask = [0.8f32; 3];
                mask[x % 3] = 1.15;
                let edge = (1.0 - su.abs().powi(8)) * (1.0 - sv.abs().powi(8));
                let vig = (1.0 - 0.35 * r2).max(0.0) * edge;
                put(
                    row,
                    x * 4,
                    [0, 1, 2].map(|i| c[i] * scan * mask[i] * vig * 1.15),
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::{Color as SkColor, Pixmap};

    fn gradient(w: u32, h: u32) -> Pixmap {
        let mut pm = Pixmap::new(w, h).unwrap();
        pm.fill(SkColor::from_rgba8(0, 0, 0, 255));
        let d = pm.data_mut();
        for y in 0..h as usize {
            for x in 0..w as usize {
                let v = (x * 255 / (w as usize - 1)) as u8;
                let i = (y * w as usize + x) * 4;
                d[i] = v;
                d[i + 1] = (y * 255 / (h as usize - 1)) as u8;
                d[i + 2] = 128;
            }
        }
        pm
    }

    #[test]
    fn dither_quantises() {
        let mut pm = gradient(64, 32);
        dither(&mut pm, 2, 0.0, &[]);
        assert!(pm
            .data()
            .chunks(4)
            .all(|p| p[..3].iter().all(|&v| v == 0 || v == 255)));
    }

    #[test]
    fn pixel_sort_keeps_pixels_and_orders_runs() {
        let mut pm = gradient(64, 8);
        // reverse each row so it's descending
        let w = 64;
        let d = pm.data_mut();
        for y in 0..8 {
            let row = &mut d[y * w * 4..(y + 1) * w * 4];
            let mut px: Vec<[u8; 4]> = row.chunks(4).map(|c| [c[0], c[1], c[2], c[3]]).collect();
            px.reverse();
            for (k, p) in px.iter().enumerate() {
                row[k * 4..k * 4 + 4].copy_from_slice(p);
            }
        }
        let before: u64 = pm.data().iter().map(|&v| v as u64).sum();
        pixel_sort(&mut pm, 0.0, false);
        let after: u64 = pm.data().iter().map(|&v| v as u64).sum();
        assert_eq!(before, after);
        let r: Vec<u8> = pm.data()[..w * 4].chunks(4).map(|c| c[0]).collect();
        assert!(r.windows(2).all(|p| p[0] <= p[1]));
    }

    #[test]
    fn kuwahara_ascii_crt_run() {
        for e in [
            Effect::Kuwahara(0.02),
            Effect::Ascii {
                cell: 0.1,
                color: None,
            },
            Effect::Crt {
                curvature: 0.1,
                scanlines: 0.5,
            },
        ] {
            let mut pm = gradient(80, 60);
            e.apply(&mut pm);
            assert!(pm.data().chunks(4).all(|p| p[3] == 255));
        }
        // a flat image is a fixed point of kuwahara
        let mut pm = Pixmap::new(20, 20).unwrap();
        pm.fill(SkColor::from_rgba8(10, 200, 30, 255));
        kuwahara(&mut pm, 0.1);
        assert!(pm.data().chunks(4).all(|p| p == [10, 200, 30, 255]));
    }
}
