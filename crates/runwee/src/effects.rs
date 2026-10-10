//! Pixel effects: fast gaussian-like blur (3× box), used for glow and shadows.

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

fn box_blur_h(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize) {
    if r == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let div = (2 * r + 1) as u32;
    for y in 0..h {
        let row = y * w * 4;
        for c in 0..4 {
            let at = |x: isize| -> u32 {
                let x = x.clamp(0, w as isize - 1) as usize;
                src[row + x * 4 + c] as u32
            };
            let mut acc: u32 = 0;
            for k in -(r as isize)..=(r as isize) {
                acc += at(k);
            }
            for x in 0..w {
                dst[row + x * 4 + c] = ((acc + div / 2) / div) as u8;
                acc += at(x as isize + r as isize + 1);
                acc -= at(x as isize - r as isize);
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
