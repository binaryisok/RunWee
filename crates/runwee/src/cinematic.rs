//! Cinematic camera & depth (phase A4).
//!
//! * Camera **modifiers** layered on top of the keyed camera: trauma-based
//!   [`Scene::camera_shake`], lag-smoothed [`Scene::camera_follow`], Catmull–Rom
//!   [`Scene::camera_spline`] dolly paths, or any [`Scene::camera_mod`] closure.
//! * **Parallax** layers that react to camera pans/zooms by depth, with optional
//!   depth-of-field blur via [`rasterize`] (layer → blurred image, cached).
//! * Pseudo-3D generators: a hyperspace [`warp_starfield`] and a synthwave
//!   [`terrain_grid`] fly-over projected through a perspective [`Camera3D`].
use std::sync::{Arc, OnceLock};

use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::noise::{fbm2, hash2, hash_unit, perlin2};
use crate::path::Rect;
use crate::render::{render_mobjects, Camera, RenderConfig};
use crate::scene::{Obj, Scene};
use crate::shapes::{line, polygon, polyline};
use crate::three_d::Camera3D;

/// Catmull–Rom interpolation between `p1` and `p2`.
pub fn catmull_rom(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, u: f64) -> Vec2 {
    let u2 = u * u;
    let u3 = u2 * u;
    (p1 * 2.0
        + (p2 - p0) * u
        + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * u2
        + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * u3)
        * 0.5
}

/// Smooth noise shake: (offset, roll) for a given `trauma ∈ [0,1]`.
/// Uses trauma² so small shakes stay subtle (the classic game-feel recipe).
pub fn shake_offset(t: f64, trauma: f64, freq: f64, amp: f64, seed: u64) -> (Vec2, f64) {
    let k = trauma.clamp(0.0, 1.0).powi(2);
    let s = seed as f64 * 17.13;
    let x = perlin2(v2(t * freq, s), seed) * amp * k;
    let y = perlin2(v2(t * freq, s + 31.7), seed) * amp * k;
    let r = perlin2(v2(t * freq, s + 77.1), seed) * 0.08 * k;
    (v2(x, y), r)
}

/// A key on a spline camera path.
#[derive(Clone, Copy, Debug)]
pub struct CamKey {
    pub t: f64,
    pub center: Vec2,
    pub height: f64,
    pub rotation: f64,
}

/// `CamKey` shorthand.
pub fn cam_key(t: f64, center: Vec2, height: f64, rotation: f64) -> CamKey {
    CamKey {
        t,
        center,
        height,
        rotation,
    }
}

/// Evaluate a key list at `t` with Catmull–Rom on centre, rotation and log-zoom.
pub fn spline_camera(keys: &[CamKey], t: f64, base: Camera) -> Camera {
    if keys.is_empty() {
        return base;
    }
    if t <= keys[0].t || keys.len() == 1 {
        let k = keys[0];
        return Camera {
            center: k.center,
            frame_height: k.height,
            rotation: k.rotation,
            ..base
        };
    }
    let n = keys.len();
    if t >= keys[n - 1].t {
        let k = keys[n - 1];
        return Camera {
            center: k.center,
            frame_height: k.height,
            rotation: k.rotation,
            ..base
        };
    }
    let i = keys.partition_point(|k| k.t <= t) - 1;
    let g = |j: isize| keys[j.clamp(0, n as isize - 1) as usize];
    let (k0, k1, k2, k3) = (
        g(i as isize - 1),
        g(i as isize),
        g(i as isize + 1),
        g(i as isize + 2),
    );
    let u = smootherstep(0.0, 1.0, (t - k1.t) / (k2.t - k1.t).max(1e-9));
    let c = catmull_rom(k0.center, k1.center, k2.center, k3.center, u);
    let zr = catmull_rom(
        v2(k0.height.ln(), k0.rotation),
        v2(k1.height.ln(), k1.rotation),
        v2(k2.height.ln(), k2.rotation),
        v2(k3.height.ln(), k3.rotation),
        u,
    );
    Camera {
        center: c,
        frame_height: zr.x.exp(),
        rotation: zr.y,
        ..base
    }
}

fn smootherstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

impl Scene {
    /// Add a camera modifier `(scene, t, camera) -> camera`.
    pub fn camera_mod(
        &mut self,
        f: impl Fn(&Scene, f64, Camera) -> Camera + Send + Sync + 'static,
    ) {
        self.camera_mods.push(Arc::new(f));
    }

    /// Screen shake starting now: `trauma` decays linearly to 0 over `duration`.
    pub fn camera_shake(&mut self, duration: f64, trauma: f64, seed: u64) {
        let t0 = self.time();
        self.camera_mod(move |_, t, mut c| {
            let u = (t - t0) / duration.max(1e-9);
            if !(0.0..1.0).contains(&u) {
                return c;
            }
            let (off, roll) =
                shake_offset(t, trauma * (1.0 - u), 18.0, 0.06 * c.frame_height, seed);
            c.center += off;
            c.rotation += roll;
            c
        });
    }

    /// Follow `o` between now and `until` with an exponential lag (seconds); the
    /// view eases on/off over `ease` seconds. `height` optionally overrides zoom.
    pub fn camera_follow(&mut self, o: Obj, until: f64, lag: f64, ease: f64, height: Option<f64>) {
        let t0 = self.time();
        self.camera_mod(move |s, t, mut c| {
            if t < t0 || t > until + ease {
                return c;
            }
            let n = 20;
            let dt = 3.0 * lag.max(1e-3) / n as f64;
            let (mut acc, mut wsum) = (Vec2::ZERO, 0.0);
            for k in 0..=n {
                let tt = (t - k as f64 * dt).max(t0);
                let w = (-(k as f64) * dt / lag.max(1e-3)).exp();
                acc += s.eval_obj(o, tt.min(until), 1).center() * w;
                wsum += w;
            }
            let target = acc * (1.0 / wsum);
            let a = smootherstep(0.0, 1.0, (t - t0) / ease.max(1e-9))
                * (1.0 - smootherstep(until, until + ease, t));
            c.center = c.center.lerp(target, a);
            if let Some(h) = height {
                c.frame_height = (c.frame_height.ln() * (1.0 - a) + h.ln() * a).exp();
            }
            c
        });
    }

    /// Drive the camera along a Catmull–Rom path through `keys` (absolute times).
    /// Outside the key range the keyed camera is untouched.
    pub fn camera_spline(&mut self, keys: Vec<CamKey>) {
        self.camera_mod(move |_, t, c| {
            if keys.is_empty() || t < keys[0].t || t > keys[keys.len() - 1].t {
                return c;
            }
            spline_camera(&keys, t, c)
        });
    }
}

/// Rasterise `m` into an image mobject at `px_per_unit` resolution with an
/// optional gaussian blur (in scene units). Strokes keep their on-screen width.
pub fn rasterize(m: &Mobject, px_per_unit: f64, blur: f64) -> Mobject {
    let Some(b) = m.bounds() else {
        return Mobject::empty();
    };
    let pad = blur * 3.0 + 0.1;
    let (w, h) = (b.width() + 2.0 * pad, b.height() + 2.0 * pad);
    let pw = (w * px_per_unit).ceil().clamp(1.0, 4096.0) as u32;
    let ph = (h * px_per_unit).ceil().clamp(1.0, 4096.0) as u32;
    let cam = Camera {
        center: b.center(),
        frame_height: ph as f64 / px_per_unit,
        rotation: 0.0,
        background: Color::rgba(0.0, 0.0, 0.0, 0.0),
    };
    let mut pm = render_mobjects(&[m], RenderConfig::new(pw, ph, 30.0), cam);
    if blur > 0.0 {
        crate::effects::blur(&mut pm, (blur * px_per_unit) as f32);
    }
    Mobject::image(Arc::new(pm), pw as f64 / px_per_unit).move_to(b.center())
}

/// Where a depth-`depth` layer should be drawn so that, seen through `cam`, it
/// moves `1/depth` as fast as the foreground relative to the `base` camera, and
/// zooms as `zoom^(1/depth)`. `depth = 1` is the normal scene plane.
pub fn parallax_view(m: &Mobject, depth: f64, cam: &Camera, base: &Camera) -> Mobject {
    let d = depth.max(1e-3);
    let z = base.frame_height / cam.frame_height;
    let zl = z.powf(1.0 / d);
    let delta = cam.center - base.center;
    m.clone()
        .shift(-(base.center + delta * (1.0 / d)))
        .scale_about(zl / z, Vec2::ZERO)
        .shift(cam.center)
}

/// Add a parallax layer. With `dof = Some((focus_depth, strength))` the layer is
/// rasterised once and blurred by `strength * |1/depth - 1/focus|` scene units.
pub fn parallax_layer(s: &mut Scene, m: Mobject, depth: f64, dof: Option<(f64, f64)>) -> Obj {
    let base = s.camera_at(s.time());
    let cache: Arc<OnceLock<Mobject>> = Arc::new(OnceLock::new());
    s.always_redraw(move |ctx| {
        let layer = cache.get_or_init(|| match dof {
            Some((focus, k)) => {
                let blur = k * (1.0 / depth - 1.0 / focus).abs();
                if blur > 1e-3 {
                    rasterize(&m, 90.0, blur)
                } else {
                    m.clone()
                }
            }
            None => m.clone(),
        });
        parallax_view(layer, depth, &ctx.camera(), &base)
    })
}

/// Hyperspace star warp: `n` stars flying at `speed` towards the viewer.
/// Streak length grows with speed; colours tinted by `tint`.
pub fn warp_starfield(
    t: f64,
    n: usize,
    speed: f64,
    center: Vec2,
    scale: f64,
    tint: Color,
    seed: u64,
) -> Mobject {
    let mut out = Mobject::empty();
    let depth = 20.0;
    for i in 0..n {
        let r = |k: i64| hash_unit(hash2(i as i64, k, seed));
        let ang = r(0) * TAU;
        let rad = 0.3 + r(1) * 6.0;
        let (x, y) = (rad * ang.cos(), rad * ang.sin());
        let z = depth * (1.0 - (r(2) + t * speed / depth).fract()) + 0.05;
        let tail_z = (z + 0.05 + speed * 0.06).min(depth);
        let p = center + v2(x, y) * (scale / z);
        let q = center + v2(x, y) * (scale / tail_z);
        let bright = (1.0 - z / depth).powf(1.5) as f32;
        let col = WHITE.mix(tint, r(3) as f32 * 0.8).with_alpha(bright);
        let w = 1.0 + 4.0 * bright;
        out.children.push(line(q, p).stroke(col, w));
    }
    out
}

/// Synthwave terrain: a noise heightfield grid scrolling towards the camera,
/// projected in perspective and fading into the distance. Valley in the middle.
#[allow(clippy::too_many_arguments)]
pub fn terrain_grid(
    t: f64,
    cam: &Camera3D,
    cols: usize,
    rows: usize,
    size: (f64, f64),
    speed: f64,
    amp: f64,
    color: Color,
    seed: u64,
) -> Mobject {
    let (w, l) = size;
    let dy = l / rows as f64;
    let scroll = t * speed;
    let off = scroll % dy;
    let height = |x: f64, yw: f64| {
        let valley = (x.abs() / (w * 0.5)).powf(1.6);
        (fbm2(v2(x * 0.35, (yw + scroll) * 0.35), 4, 2.0, 0.5, seed) * 0.5 + 0.6)
            * amp
            * (0.08 + valley)
    };
    let pt = |i: usize, j: usize| {
        let x = -w / 2.0 + w * i as f64 / cols as f64;
        let y = j as f64 * dy - off;
        v3(x, y, height(x, y))
    };
    let fade = |y: f64| (1.0 - (y / l)).clamp(0.0, 1.0).powf(0.8) as f32;
    let mut out = Mobject::empty();
    // rows (across), back to front
    for j in (0..=rows).rev() {
        let pts: Vec<Vec2> = (0..=cols).map(|i| cam.project(pt(i, j)).0).collect();
        let y = j as f64 * dy - off;
        let mut m = polyline(&pts);
        m = m.stroke(color.with_alpha(fade(y)), 2.0);
        out.children.push(m);
    }
    // columns (along)
    for i in 0..=cols {
        for j in 0..rows {
            let a = cam.project(pt(i, j)).0;
            let b = cam.project(pt(i, j + 1)).0;
            let y = j as f64 * dy - off;
            out.children
                .push(line(a, b).stroke(color.with_alpha(fade(y) * 0.8), 1.5));
        }
    }
    out
}

/// Retro sun with horizontal cut-out bands, centred at `c`.
pub fn retro_sun(c: Vec2, r: f64, top: Color, bottom: Color, t: f64) -> Mobject {
    let mut out = Mobject::empty();
    let bands = 18;
    for k in 0..bands {
        let y0 = -r + 2.0 * r * k as f64 / bands as f64;
        let y1 = y0 + 2.0 * r / bands as f64;
        // gaps widen towards the bottom and scroll down slowly
        let u = (k as f64 + 0.5) / bands as f64;
        let gap = if u < 0.55 {
            0.0
        } else {
            (0.55 - u).abs() * 0.5 * (1.0 + 0.3 * (t * 2.0 + k as f64).sin())
        };
        let y0g = y0 + gap * (y1 - y0) * 2.0;
        if y0g >= y1 {
            continue;
        }
        let mut pts = vec![];
        let steps = 24;
        for s in 0..=steps {
            let y = y0g + (y1 - y0g) * s as f64 / steps as f64;
            let x = (r * r - y * y).max(0.0).sqrt();
            pts.push(c + v2(-x, -y));
        }
        for s in (0..=steps).rev() {
            let y = y0g + (y1 - y0g) * s as f64 / steps as f64;
            let x = (r * r - y * y).max(0.0).sqrt();
            pts.push(c + v2(x, -y));
        }
        let col = top.mix(bottom, u as f32);
        out.children.push(polygon(&pts).fill(col, 1.0).no_stroke());
    }
    out
}

/// Bounds of the visible frame for a camera (scene units, ignoring rotation).
pub fn camera_rect(c: &Camera, aspect: f64) -> Rect {
    Rect::from_center(c.center, c.frame_height * aspect, c.frame_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::circle;

    #[test]
    fn catmull_rom_interpolates_endpoints() {
        let (a, b, c, d) = (v2(0.0, 0.0), v2(1.0, 0.0), v2(2.0, 1.0), v2(3.0, 1.0));
        assert!(catmull_rom(a, b, c, d, 0.0).distance(b) < 1e-12);
        assert!(catmull_rom(a, b, c, d, 1.0).distance(c) < 1e-12);
    }

    #[test]
    fn spline_hits_keys_and_parallax_identity() {
        let base = Camera::default();
        let keys = vec![
            cam_key(0.0, v2(0.0, 0.0), 8.0, 0.0),
            cam_key(1.0, v2(4.0, 1.0), 4.0, 0.3),
            cam_key(2.0, v2(0.0, 0.0), 8.0, 0.0),
        ];
        let c = spline_camera(&keys, 1.0, base);
        assert!(c.center.distance(v2(4.0, 1.0)) < 1e-9 && (c.frame_height - 4.0).abs() < 1e-9);
        // depth 1 is the scene plane: no change
        let m = circle(1.0).shift(v2(2.0, 1.0));
        let cam = Camera {
            center: v2(3.0, -1.0),
            frame_height: 5.0,
            ..base
        };
        let v = parallax_view(&m, 1.0, &cam, &base);
        assert!(v.center().distance(m.center()) < 1e-9);
        // infinitely far layers stick to the screen (same screen position & size)
        let far = parallax_view(&m, 1e6, &cam, &base);
        let z = base.frame_height / cam.frame_height;
        assert!(((far.center() - cam.center) * z).distance(m.center()) < 1e-3);
    }

    #[test]
    fn shake_and_stars_are_deterministic() {
        let a = shake_offset(1.3, 1.0, 10.0, 0.5, 3);
        assert_eq!(a, shake_offset(1.3, 1.0, 10.0, 0.5, 3));
        assert_eq!(shake_offset(1.3, 0.0, 10.0, 0.5, 3).0, Vec2::ZERO);
        let s = warp_starfield(0.5, 50, 4.0, Vec2::ZERO, 1.0, BLUE, 1);
        assert_eq!(s.children.len(), 50);
        let r = rasterize(&circle(1.0), 40.0, 0.1);
        assert!(r.image.is_some());
    }
}
