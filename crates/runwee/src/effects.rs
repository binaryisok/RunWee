//! Pixel effects. `blur` powers glow/shadows; [`Effect`] is a library of full-frame
//! post-processing effects modelled on `@remotion/effects` (colour grading,
//! blur/bloom, vignette, pixelate, grain, scanlines, halftone, chromatic
//! aberration, zoom blur, mirror, wave, fisheye, emboss, sharpen…).

//!
//! Units: unless stated otherwise, lengths are fractions of the frame height,
//! positions are UV (`[0, 0]` = top-left, `[1, 1]` = bottom-right) and angles are
//! degrees. Remotion's pixel defaults are converted assuming a 1080p frame.
//! Every effect is a pure function of its parameters; animated looks take a
//! `progress`/`phase`/`seed` parameter.

use crate::color::Color;
use rayon::prelude::*;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;
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
    // ------------------------------------------------------------ stylize (phase A5)
    /// Ordered Bayer dithering; blocks of `cell` × height; quantise to `levels`
    /// per channel, or snap to `palette` when non-empty.
    Dither {
        levels: u8,
        cell: f32,
        palette: Vec<Color>,
    },
    /// Painterly Kuwahara filter, radius as a fraction of the height.
    Kuwahara(f32),
    /// Sort bright runs of pixels by luminance (glitch art).
    PixelSort {
        threshold: f32,
        vertical: bool,
    },
    /// ASCII art with cells of `cell` × height; `color` fixes the ink colour.
    Ascii {
        cell: f32,
        color: Option<Color>,
    },
    /// CRT monitor: barrel `curvature`, scanline strength, aperture mask, vignette.
    Crt {
        curvature: f32,
        scanlines: f32,
    },
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
    /// Fade the whole frame towards a colour (0..1). Also Remotion's `tint()`
    /// (a flat colour blended over the alpha mask).
    FadeTo(Color, f32),

    // ------------------------------------------------------------ colour grading
    /// Remotion `duotone()`: pixels below the luminance `threshold` become `dark`,
    /// the rest `light` (alpha preserved). [`Effect::Duotone`] is the smooth gradient map.
    DuotoneThreshold {
        dark: Color,
        light: Color,
        threshold: f32,
    },
    /// Boost (or reduce, `-1..0`) muted colours more than saturated ones (`-1..1`).
    Vibrance(f32),
    /// Blue↔amber `temperature` and green↔magenta `tint` (`-1..1`), in linear light.
    WhiteBalance {
        temperature: f32,
        tint: f32,
    },
    /// Lift/lower dark and bright tonal regions independently (`-1..1`, in stops).
    ShadowsHighlights {
        shadows: f32,
        highlights: f32,
    },
    /// Primary colour correction in one pass (Remotion `colorCorrection()`).
    ColorCorrection(ColorCorrectionParams),
    /// Luminance mapped to a heat-map colour ramp. An empty `palette` uses the
    /// default blue→green→yellow→red→white ramp.
    ThermalVision {
        amount: f32,
        palette: Vec<Color>,
    },
    /// Blend into a TV colour-bars test pattern (0..1). Alpha preserved.
    TvSignalOff(f32),
    /// Blend towards grayscale white noise (TV static). Animate `seed` per frame.
    WhiteNoise {
        amount: f32,
        seed: u32,
    },
    /// Centred additive noise (`amount` 0..1); `premultiply` scales it by the colour.
    Noise {
        amount: f32,
        seed: u32,
        premultiply: bool,
    },
    /// Chroma key: pixels close to `key` become transparent. `similarity`,
    /// `smoothness` and `spill` (spill suppression) are 0..1.
    ColorKey {
        key: Color,
        similarity: f32,
        smoothness: f32,
        spill: f32,
    },
    /// 3D (or 1D) colour lookup table from an Adobe `.cube` file. Compared by pointer.
    Lut(Arc<Lut>),
    /// Tint whose colour follows a two-stop linear gradient between UV points.
    LinearGradientTint {
        start: [f32; 2],
        end: [f32; 2],
        start_color: Color,
        end_color: Color,
        amount: f32,
    },
    /// Remotion-style vignette: `radius`/`feather` in centre-to-edge units
    /// (edges = 1), `roundness` 0 = rectangular … 1 = elliptical. `fade_alpha`
    /// fades edges to transparent instead of blending `color`.
    VignetteMask {
        amount: f32,
        radius: f32,
        feather: f32,
        roundness: f32,
        color: Color,
        fade_alpha: bool,
        center: [f32; 2],
    },

    // ------------------------------------------------------------ blur / light
    /// Blur only along the chosen axes (σ as a fraction of the frame height).
    BlurAxis {
        sigma: f32,
        horizontal: bool,
        vertical: bool,
    },
    /// Coloured halo around bright parts (Remotion `glow()`). `radius` is the
    /// blur σ as a fraction of the frame height; `intensity` ≥ 0; `threshold` 0..1.
    Glow {
        radius: f32,
        intensity: f32,
        threshold: f32,
        color: Color,
    },
    /// Directional streak of bright/opaque areas (Remotion `lightTrail()`, which is
    /// also single-frame). `direction` in degrees (0 = right, 90 = up),
    /// `distance` as a fraction of the frame height, `samples` 1..=64.
    LightTrail {
        direction: f32,
        distance: f32,
        intensity: f32,
        decay: f32,
        threshold: f32,
        samples: u32,
        color: Color,
    },
    /// Gaussian blur inside a (rounded) UV rectangle. `radius` is σ and `feather`
    /// the edge width, both fractions of the frame height; `roundness` 0..1.
    RegionBlur {
        top_left: [f32; 2],
        bottom_right: [f32; 2],
        radius: f32,
        feather: f32,
        roundness: f32,
    },
    /// Blur σ interpolated along a linear gradient between two UV points.
    LinearProgressiveBlur {
        start: [f32; 2],
        end: [f32; 2],
        start_blur: f32,
        end_blur: f32,
    },
    /// Blur σ interpolated from `center` to the edge of an ellipse (`width`,
    /// `height` in UV, `rotation` in degrees); `start` is where interpolation begins (0..1).
    RadialProgressiveBlur {
        center: [f32; 2],
        width: f32,
        height: f32,
        rotation: f32,
        start: f32,
        start_blur: f32,
        end_blur: f32,
    },
    /// Pixel block size (fraction of the frame height) along a linear gradient.
    LinearProgressivePixelate {
        start: [f32; 2],
        end: [f32; 2],
        start_block: f32,
        end_block: f32,
    },
    /// Pixel block size from `center` to the edge of an ellipse.
    RadialProgressivePixelate {
        center: [f32; 2],
        width: f32,
        height: f32,
        rotation: f32,
        start: f32,
        start_block: f32,
        end_block: f32,
    },

    // ------------------------------------------------------------ geometry
    /// CRT-style barrel curvature (0..1). Uncovered corners become transparent.
    BarrelDistortion(f32),
    /// Projective warp of the frame into the quad given by four UV corners.
    CornerPin {
        top_left: [f32; 2],
        top_right: [f32; 2],
        bottom_right: [f32; 2],
        bottom_left: [f32; 2],
    },
    /// Physically-shaped fisheye (Remotion `fisheye()`): `fov` in radians (0..π),
    /// `radius` of the lens (1 = full frame height), post-warp `zoom`.
    FisheyeLens {
        fov: f32,
        center: [f32; 2],
        radius: f32,
        zoom: f32,
    },
    /// Mirror around a line at `position` (UV). `horizontal` mirrors along x
    /// (right side shows the left); `invert` mirrors the other side.
    MirrorAt {
        horizontal: bool,
        position: f32,
        invert: bool,
    },
    /// Sine displacement (Remotion `wave()`): `amplitude`/`wavelength` as fractions
    /// of the frame height, `phase` in radians. `horizontal` = wave travels along x.
    WaveDisplace {
        amplitude: f32,
        wavelength: f32,
        phase: f32,
        horizontal: bool,
    },
    /// Linear RGB split: `amount` (fraction of the frame height) along `angle` degrees.
    ChromaticSplit {
        amount: f32,
        angle: f32,
    },
    /// Scale about the centre (1 = unchanged). Uncovered areas become transparent.
    Scale {
        x: f32,
        y: f32,
    },
    /// Skew by angles in degrees around a UV origin.
    Skew {
        x: f32,
        y: f32,
        origin: [f32; 2],
    },
    /// Move the frame by a UV offset (fractions of width / height).
    Translate {
        x: f32,
        y: f32,
    },
    /// Repeat the non-transparent bounding box of the frame, mirroring every other copy.
    Tile {
        horizontal: bool,
        vertical: bool,
    },
    /// Repeat a scaled copy of the frame as wallpaper (Remotion `pattern()`).
    Pattern(PatternParams),
    /// Tear along a zig-zag seam; the halves rotate outward. `angle` in degrees
    /// (0 = top→bottom), `rotation` max degrees, `jaggedness` fraction of height.
    Tear {
        progress: f32,
        angle: f32,
        rotation: f32,
        jaggedness: f32,
    },
    /// Procedural noise distortion of a circular area.
    NoiseDisplacement(NoiseDisplacementParams),

    // ------------------------------------------------------------ alpha / reveal
    /// Blurred shadow behind the alpha shape. `radius` (σ) and offsets are
    /// fractions of the frame height.
    DropShadow {
        radius: f32,
        offset_x: f32,
        offset_y: f32,
        opacity: f32,
        color: Color,
    },
    /// Solid outline around non-transparent pixels (`width` fraction of height).
    Outline {
        width: f32,
        color: Color,
        opacity: f32,
        outline_only: bool,
    },
    /// Noisy, paper-like alpha boundary. `border` fraction of height, `scale` 0.01..4.
    RoughenEdges {
        amount: f32,
        border: f32,
        scale: f32,
        seed: f32,
    },
    /// Procedural plastic wrap: wrinkles, glossy highlights, small displacement
    /// (`displacement` as a fraction of the frame height).
    Shrinkwrap {
        amount: f32,
        displacement: f32,
        highlight: f32,
        wrinkle_density: f32,
        edge_tension: f32,
        phase: f32,
        seed: f32,
    },
    /// Small random alpha holes. `size` is the max hole diameter (fraction of height).
    Speckle {
        density: f32,
        size: f32,
        randomness: f32,
        seed: u32,
    },
    /// Keep the frame only inside a grid of dots (or cut the dots out with `invert`).
    DotGrid {
        dot_size: f32,
        grid_size: f32,
        invert: bool,
    },
    /// Wipe reveal from an edge (`progress` 0 hidden … 1 revealed; `feather` 0..1).
    Evolve {
        progress: f32,
        from: RevealFrom,
        feather: f32,
    },
    /// Cells fade out in a seeded random order as `progress` goes 0 → 1.
    PixelDissolve {
        progress: f32,
        columns: u32,
        rows: u32,
        seed: u32,
        feather: f32,
    },
    /// Reveal through blinds (`progress` 0 hidden … 1 revealed).
    VenetianBlinds {
        progress: f32,
        vertical: bool,
        slats: u32,
    },
    /// Glossy light sweep; `progress` 0..1 moves the band across. Widths are
    /// fractions of the frame height, `angle` in degrees.
    Shine {
        progress: f32,
        angle: f32,
        halo: f32,
        core: f32,
        halo_intensity: f32,
        core_intensity: f32,
    },

    // ------------------------------------------------------------ generators
    // Every generator composites over the frame with `opacity`; `mask` keeps the
    // result inside the frame's alpha (Remotion `maskToSourceAlpha`). On a clear
    // pixmap they act as backgrounds. Lengths are fractions of the frame height.
    /// Stripes, colours assigned cyclically; `gap` is transparent.
    Lines {
        colors: Vec<Color>,
        vertical: bool,
        thickness: f32,
        gap: f32,
        angle: f32,
        offset: f32,
        opacity: f32,
        mask: bool,
    },
    Checkerboard {
        colors: Vec<Color>,
        cell: f32,
        gap: f32,
        angle: f32,
        offset: [f32; 2],
        opacity: f32,
        mask: bool,
    },
    /// Concentric bands around a UV `center`.
    Rings {
        colors: Vec<Color>,
        center: [f32; 2],
        thickness: f32,
        gap: f32,
        offset: f32,
        opacity: f32,
        mask: bool,
    },
    /// Wavy bands; `phase` in degrees.
    Waves {
        colors: Vec<Color>,
        vertical: bool,
        thickness: f32,
        gap: f32,
        angle: f32,
        offset: f32,
        amplitude: f32,
        wavelength: f32,
        phase: f32,
        opacity: f32,
        mask: bool,
    },
    /// Zig-zag bands.
    Zigzag {
        colors: Vec<Color>,
        vertical: bool,
        thickness: f32,
        gap: f32,
        angle: f32,
        offset: f32,
        amplitude: f32,
        wavelength: f32,
        opacity: f32,
        mask: bool,
    },
    /// Grid lines, optionally in 3D perspective.
    Gridlines(GridlinesParams),
    /// Retro rays from a UV `origin`; `smoothness` 0..1 blends ray edges.
    Starburst {
        rays: u32,
        colors: Vec<Color>,
        rotation: f32,
        smoothness: f32,
        origin: [f32; 2],
        opacity: f32,
    },
    /// Two-stop linear gradient between UV points (replaces the frame at opacity 1).
    LinearGradient {
        start: [f32; 2],
        end: [f32; 2],
        start_color: Color,
        end_color: Color,
        opacity: f32,
    },
    /// Remotion `halftone()`: luminance → dots/squares/lines of `size` on a grid of
    /// `spacing` (fractions of height), rotated by degrees. `color: None` uses the
    /// source colour. `invert` makes bright areas produce big dots.
    HalftoneGrid {
        shape: HalftoneShape,
        size: f32,
        spacing: f32,
        rotation: f32,
        offset: [f32; 2],
        color: Option<Color>,
        invert: bool,
    },
    /// Dot grid whose dot size follows a linear gradient between two UV stops.
    HalftoneGradient {
        start_size: f32,
        end_size: f32,
        start: [f32; 2],
        end: [f32; 2],
        grid: f32,
        color: Option<Color>,
        mask: bool,
    },
    /// Topographic contour lines over the frame.
    ContourLines(ContourLinesParams),
    /// Two-colour warped topographic bands (a background generator).
    LiquidContours(LiquidContoursParams),
    /// Warm light-leak overlay; reveals over `progress` 0..0.5, retracts 0.5..1.
    LightLeak {
        seed: f32,
        hue_shift: f32,
        progress: f32,
    },
    /// Woven burlap texture (`size` = strand spacing, fraction of height).
    Burlap {
        amount: f32,
        size: f32,
        roughness: f32,
        seed: u32,
        color: Color,
    },
    /// Plaid flannel fabric (`size` = plaid repeat, fraction of height).
    Flannel {
        amount: f32,
        size: f32,
        softness: f32,
        base: Color,
        stripe: Color,
    },
    /// Procedural paper: grain, fibres, crumples, folds and speckles.
    Paper(PaperParams),
    /// Remotion `emboss()`: a repeated raised-dash relief (sizes are fractions of
    /// height, angles degrees). [`Effect::Emboss`] is the classic 3×3 kernel.
    EmbossRelief {
        amount: f32,
        size: f32,
        line_width: f32,
        depth: f32,
        angle: f32,
        light_angle: f32,
        offset: f32,
    },
}

/// Edge an [`Effect::Evolve`] reveal starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RevealFrom {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

/// Primitive drawn by [`Effect::HalftoneGrid`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HalftoneShape {
    #[default]
    Circle,
    Square,
    Line,
}

/// Parameters of [`Effect::ColorCorrection`] (defaults = no change).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorCorrectionParams {
    /// Stops, in linear light.
    pub exposure: f32,
    pub contrast: f32,
    pub pivot: f32,
    pub shadows: f32,
    pub highlights: f32,
    pub whites: f32,
    pub blacks: f32,
    pub temperature: f32,
    pub tint: f32,
    pub saturation: f32,
    pub vibrance: f32,
}

impl Default for ColorCorrectionParams {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            pivot: 0.5,
            shadows: 0.0,
            highlights: 0.0,
            whites: 0.0,
            blacks: 0.0,
            temperature: 0.0,
            tint: 0.0,
            saturation: 1.0,
            vibrance: 0.0,
        }
    }
}

/// Parameters of [`Effect::Gridlines`]. Lengths are fractions of the frame height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridlinesParams {
    pub size: f32,
    pub line_width: f32,
    pub color: Color,
    pub background: Color,
    /// Degrees, in the image plane.
    pub rotation: f32,
    /// Degrees, tilting the grid plane (needs `perspective` > 0 to foreshorten).
    pub rotation_x: f32,
    pub rotation_y: f32,
    /// Camera distance; 0 = orthographic.
    pub perspective: f32,
    pub offset: [f32; 2],
    pub opacity: f32,
    pub mask: bool,
}

impl Default for GridlinesParams {
    fn default() -> Self {
        Self {
            size: 64.0 / 1080.0,
            line_width: 2.0 / 1080.0,
            color: crate::color::WHITE,
            background: Color::TRANSPARENT,
            rotation: 0.0,
            rotation_x: 0.0,
            rotation_y: 0.0,
            perspective: 0.0,
            offset: [0.0, 0.0],
            opacity: 1.0,
            mask: false,
        }
    }
}

/// Parameters of [`Effect::ContourLines`]. Lengths are fractions of the frame height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContourLinesParams {
    pub color: Color,
    pub line_width: f32,
    pub spacing: f32,
    pub scale: f32,
    pub complexity: f32,
    pub smoothness: f32,
    pub seed: f32,
    pub offset: [f32; 2],
    pub opacity: f32,
    pub mask: bool,
}

impl Default for ContourLinesParams {
    fn default() -> Self {
        Self {
            color: crate::color::WHITE,
            line_width: 1.5 / 1080.0,
            spacing: 36.0 / 1080.0,
            scale: 220.0 / 1080.0,
            complexity: 0.65,
            smoothness: 0.55,
            seed: 0.0,
            offset: [0.0, 0.0],
            opacity: 1.0,
            mask: false,
        }
    }
}

/// Parameters of [`Effect::LiquidContours`]. Lengths are fractions of the frame height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidContoursParams {
    pub first: Color,
    pub second: Color,
    pub spacing: f32,
    pub scale: f32,
    pub complexity: f32,
    pub smoothness: f32,
    pub seed: f32,
    pub offset: [f32; 2],
    pub phase: f32,
    pub opacity: f32,
}

impl Default for LiquidContoursParams {
    fn default() -> Self {
        Self {
            first: crate::color::rgb8(0xff, 0x1a, 0x0a),
            second: crate::color::rgb8(0x05, 0x05, 0x05),
            spacing: 62.0 / 1080.0,
            scale: 300.0 / 1080.0,
            complexity: 0.0,
            smoothness: 1.0,
            seed: 4.0,
            offset: [13.4 / 1080.0, 0.0],
            phase: 3.23,
            opacity: 1.0,
        }
    }
}

/// Parameters of [`Effect::Paper`] (Remotion `paper()` defaults).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaperParams {
    pub amount: f32,
    pub front: Color,
    pub back: Color,
    pub contrast: f32,
    pub roughness: f32,
    pub fiber: f32,
    pub fiber_size: f32,
    pub crumples: f32,
    pub crumple_size: f32,
    pub folds: f32,
    pub fold_count: u32,
    pub drops: f32,
    pub fade: f32,
    pub seed: f32,
    pub scale: f32,
}

impl Default for PaperParams {
    fn default() -> Self {
        Self {
            amount: 1.0,
            front: crate::color::rgb8(0x9f, 0xad, 0xbc),
            back: crate::color::WHITE,
            contrast: 0.3,
            roughness: 0.4,
            fiber: 0.3,
            fiber_size: 0.2,
            crumples: 0.3,
            crumple_size: 0.35,
            folds: 0.65,
            fold_count: 5,
            drops: 0.2,
            fade: 0.0,
            seed: 6.0,
            scale: 0.6,
        }
    }
}

/// Parameters of [`Effect::Pattern`]. Crop and offsets/origin are UV; gaps and
/// stagger offsets are fractions of the frame height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatternParams {
    pub scale: f32,
    /// left, top, right, bottom (UV fractions cropped before tiling).
    pub crop: [f32; 4],
    pub gap: [f32; 2],
    pub offset: [f32; 2],
    pub row_offset: f32,
    pub row_offset_every: u32,
    pub column_offset: f32,
    pub column_offset_every: u32,
    pub origin: [f32; 2],
    pub wrap: bool,
}

impl Default for PatternParams {
    fn default() -> Self {
        Self {
            scale: 0.1,
            crop: [0.0; 4],
            gap: [0.0; 2],
            offset: [0.0; 2],
            row_offset: 0.0,
            row_offset_every: 0,
            column_offset: 0.0,
            column_offset_every: 0,
            origin: [0.0; 2],
            wrap: true,
        }
    }
}

/// Parameters of [`Effect::NoiseDisplacement`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoiseDisplacementParams {
    /// UV centre of the affected disc.
    pub center: [f32; 2],
    /// Radius relative to the shorter frame side.
    pub radius: f32,
    /// Max displacement (fraction of height).
    pub strength: f32,
    pub seed: f32,
    /// Noise cell size (fraction of height).
    pub grain_size: f32,
    pub passes: u32,
    /// Extra local blur (fraction of height).
    pub blur: f32,
    /// Edge softness as a fraction of the radius.
    pub feather: f32,
    /// Degrees.
    pub bias_direction: f32,
    pub bias_amount: f32,
}

impl Default for NoiseDisplacementParams {
    fn default() -> Self {
        Self {
            center: [0.5, 0.5],
            radius: 0.3,
            strength: 36.0 / 1080.0,
            seed: 0.0,
            grain_size: 8.0 / 1080.0,
            passes: 6,
            blur: 0.0,
            feather: 0.25,
            bias_direction: 0.0,
            bias_amount: 0.0,
        }
    }
}

/// A colour lookup table parsed from an Adobe `.cube` file (3D or 1D).
#[derive(Clone)]
pub struct Lut {
    /// Entries per axis.
    pub size: usize,
    /// `true` for a `LUT_1D_SIZE` table (one curve per channel).
    pub one_d: bool,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
    /// Output colours; for 3D, red varies fastest (`r + g*n + b*n*n`).
    pub table: Vec<[f32; 3]>,
}

impl std::fmt::Debug for Lut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Lut({}{} entries)",
            self.size,
            if self.one_d { "" } else { "³" }
        )
    }
}

/// LUTs compare by identity (two `Arc`s to the same table are equal).
impl PartialEq for Lut {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Lut {
    /// Parse the text of an Adobe `.cube` file.
    pub fn from_cube_str(s: &str) -> anyhow::Result<Lut> {
        let mut size = 0usize;
        let mut one_d = false;
        let mut dmin = [0.0f32; 3];
        let mut dmax = [1.0f32; 3];
        let mut table = Vec::new();
        for (ln, line) in s.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let mut it = line.split_whitespace();
            let key = it.next().unwrap_or("");
            let nums = |it: std::str::SplitWhitespace| -> anyhow::Result<Vec<f32>> {
                it.map(|t| {
                    t.parse::<f32>()
                        .map_err(|_| anyhow::anyhow!("line {}: bad number '{t}'", ln + 1))
                })
                .collect()
            };
            match key.to_ascii_uppercase().as_str() {
                "TITLE" => {}
                "LUT_3D_SIZE" | "LUT_1D_SIZE" => {
                    one_d = key.eq_ignore_ascii_case("LUT_1D_SIZE");
                    size = it
                        .next()
                        .and_then(|t| t.parse().ok())
                        .ok_or_else(|| anyhow::anyhow!("line {}: bad LUT size", ln + 1))?;
                }
                "DOMAIN_MIN" | "DOMAIN_MAX" => {
                    let v = nums(it)?;
                    anyhow::ensure!(v.len() == 3, "line {}: expected 3 values", ln + 1);
                    let d = [v[0], v[1], v[2]];
                    if key.eq_ignore_ascii_case("DOMAIN_MIN") {
                        dmin = d;
                    } else {
                        dmax = d;
                    }
                }
                "LUT_3D_INPUT_RANGE" | "LUT_1D_INPUT_RANGE" => {
                    let v = nums(it)?;
                    anyhow::ensure!(v.len() == 2, "line {}: expected 2 values", ln + 1);
                    dmin = [v[0]; 3];
                    dmax = [v[1]; 3];
                }
                _ if key.starts_with(|c: char| c.is_ascii_alphabetic()) => {}
                _ => {
                    let v = nums(line.split_whitespace())?;
                    anyhow::ensure!(v.len() == 3, "line {}: expected r g b", ln + 1);
                    table.push([v[0], v[1], v[2]]);
                }
            }
        }
        anyhow::ensure!(size >= 2, "missing or invalid LUT_3D_SIZE / LUT_1D_SIZE");
        let expected = if one_d { size } else { size * size * size };
        anyhow::ensure!(
            table.len() == expected,
            "expected {expected} LUT entries, found {}",
            table.len()
        );
        Ok(Lut {
            size,
            one_d,
            domain_min: dmin,
            domain_max: dmax,
            table,
        })
    }

    /// Load and parse a `.cube` file.
    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Lut> {
        let s = std::fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.as_ref().display()))?;
        Lut::from_cube_str(&s)
    }

    /// Build a 3D LUT of `size`³ entries from a colour function (inputs 0..1).
    pub fn from_fn(size: usize, f: impl Fn([f32; 3]) -> [f32; 3]) -> Lut {
        let size = size.max(2);
        let k = 1.0 / (size - 1) as f32;
        let mut table = Vec::with_capacity(size * size * size);
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    table.push(f([r as f32 * k, g as f32 * k, b as f32 * k]));
                }
            }
        }
        Lut {
            size,
            one_d: false,
            domain_min: [0.0; 3],
            domain_max: [1.0; 3],
            table,
        }
    }

    /// The identity LUT.
    pub fn identity(size: usize) -> Lut {
        Lut::from_fn(size, |c| c)
    }

    /// Look up a colour (trilinear for 3D, linear for 1D).
    pub fn lookup(&self, c: [f32; 3]) -> [f32; 3] {
        let n = self.size;
        let nf = (n - 1) as f32;
        let t: [f32; 3] = [0, 1, 2].map(|i| {
            ((c[i] - self.domain_min[i]) / (self.domain_max[i] - self.domain_min[i]).max(1e-6))
                .clamp(0.0, 1.0)
                * nf
        });
        if self.one_d {
            return [0, 1, 2].map(|i| {
                let i0 = (t[i].floor() as usize).min(n - 1);
                let i1 = (i0 + 1).min(n - 1);
                mix(self.table[i0][i], self.table[i1][i], t[i] - i0 as f32)
            });
        }
        let i0 = t.map(|v| (v.floor() as usize).min(n - 1));
        let i1 = i0.map(|v| (v + 1).min(n - 1));
        let f = [0, 1, 2].map(|i| t[i] - i0[i] as f32);
        let at = |r: usize, g: usize, b: usize| self.table[r + g * n + b * n * n];
        let mut out = [0.0; 3];
        for (ch, o) in out.iter_mut().enumerate() {
            let c00 = mix(
                at(i0[0], i0[1], i0[2])[ch],
                at(i1[0], i0[1], i0[2])[ch],
                f[0],
            );
            let c10 = mix(
                at(i0[0], i1[1], i0[2])[ch],
                at(i1[0], i1[1], i0[2])[ch],
                f[0],
            );
            let c01 = mix(
                at(i0[0], i0[1], i1[2])[ch],
                at(i1[0], i0[1], i1[2])[ch],
                f[0],
            );
            let c11 = mix(
                at(i0[0], i1[1], i1[2])[ch],
                at(i1[0], i1[1], i1[2])[ch],
                f[0],
            );
            *o = mix(mix(c00, c10, f[1]), mix(c01, c11, f[1]), f[2]);
        }
        out
    }
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
                // in linear light, like Remotion's `exposure()`
                let k = 2f32.powf(*ev);
                color_map(pm, move |c| {
                    c.map(|v| linear_to_srgb(srgb_to_linear(v) * k))
                })
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
                            let a = row[x * 4 + 3] as f32;
                            row[x * 4] = r[0].min(a) as u8;
                            row[x * 4 + 2] = b[2].min(a) as u8;
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
            other => other.apply_ext(pm),
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

// ===================================================================== helpers

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    if e1 == e0 {
        return if x < e0 { 0.0 } else { 1.0 };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [mix(a[0], b[0], t), mix(a[1], b[1], t), mix(a[2], b[2], t)]
}

#[inline]
fn mix4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        mix(a[0], b[0], t),
        mix(a[1], b[1], t),
        mix(a[2], b[2], t),
        mix(a[3], b[3], t),
    ]
}

#[inline]
fn fract(x: f32) -> f32 {
    x - x.floor()
}

#[inline]
fn rgb(c: Color) -> [f32; 3] {
    [c.r, c.g, c.b]
}

#[inline]
fn rgba(c: Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

fn srgb_to_linear(c: f32) -> f32 {
    if c < 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    let c = c.max(0.0);
    if c < 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn luma601(c: [f32; 3]) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

fn white_balance_linear(l: [f32; 3], temperature: f32, tint: f32) -> [f32; 3] {
    let g = [
        (0.3 * temperature + 0.15 * tint).exp2(),
        (-0.3 * tint).exp2(),
        (-0.3 * temperature + 0.15 * tint).exp2(),
    ];
    let lg = 0.2126 * g[0] + 0.7152 * g[1] + 0.0722 * g[2];
    [l[0] * g[0] / lg, l[1] * g[1] / lg, l[2] * g[2] / lg]
}

fn white_balance(c: [f32; 3], temperature: f32, tint: f32) -> [f32; 3] {
    white_balance_linear(c.map(srgb_to_linear), temperature, tint).map(linear_to_srgb)
}

fn shadows_highlights_stops(c: [f32; 3], shadows: f32, highlights: f32) -> f32 {
    let l = luma(c);
    let sw = (1.0 - smoothstep(0.0, 0.6, l)).powi(2);
    let hw = smoothstep(0.4, 1.0, l).powi(2);
    shadows * sw + highlights * hw
}

fn exposure_stops(c: [f32; 3], stops: f32) -> [f32; 3] {
    let k = stops.exp2();
    c.map(|v| linear_to_srgb(srgb_to_linear(v) * k))
}

fn vibrance(c: [f32; 3], amount: f32) -> [f32; 3] {
    let mx = c[0].max(c[1]).max(c[2]);
    let mn = c[0].min(c[1]).min(c[2]);
    let l = (mx + mn) * 0.5;
    let den = 1.0 - (2.0 * l - 1.0).abs();
    let s = if den <= 0.0 { 0.0 } else { (mx - mn) / den };
    if s <= 1e-6 {
        return c;
    }
    let target = if amount >= 0.0 {
        s + amount * (1.0 - s)
    } else {
        s * (1.0 + amount)
    };
    c.map(|v| (l + (v - l) * (target / s)).clamp(0.0, 1.0))
}

fn color_correct(c: [f32; 3], p: &ColorCorrectionParams) -> [f32; 3] {
    let lin = c.map(|v| srgb_to_linear(v) * p.exposure.exp2());
    let mut c = white_balance_linear(lin, p.temperature, p.tint).map(linear_to_srgb);
    let st = shadows_highlights_stops(c, p.shadows, p.highlights);
    c = exposure_stops(c, st);
    // endpoint tones
    let l = luma(c);
    let bw = (1.0 - smoothstep(0.0, 0.35, l)).powi(2);
    let ww = smoothstep(0.65, 1.0, l).powi(2);
    c = c.map(|mut v| {
        if p.blacks >= 0.0 {
            v += (1.0 - v) * p.blacks * bw * 0.2;
        } else {
            v *= 1.0 + p.blacks * bw;
        }
        if p.whites >= 0.0 {
            v += (1.0 - v) * p.whites * ww;
        } else {
            v *= 1.0 + p.whites * ww * 0.2;
        }
        v.clamp(0.0, 1.0)
    });
    c = c.map(|v| ((v - p.pivot) * p.contrast + p.pivot).clamp(0.0, 1.0));
    let l = 0.213 * c[0] + 0.715 * c[1] + 0.072 * c[2];
    c = c.map(|v| (l + (v - l) * p.saturation).clamp(0.0, 1.0));
    vibrance(c, p.vibrance)
}

fn default_thermal_palette() -> Vec<Color> {
    use crate::color::rgb8;
    vec![
        rgb8(0x02, 0x06, 0x17),
        rgb8(0x12, 0x38, 0xff),
        rgb8(0x00, 0xa6, 0xff),
        rgb8(0x00, 0xc8, 0x53),
        rgb8(0xd6, 0xf5, 0x42),
        rgb8(0xff, 0xb0, 0x00),
        rgb8(0xff, 0x2f, 0x00),
        rgb8(0xff, 0xff, 0xff),
    ]
}

/// Interpolate along a colour ramp (t in 0..1).
fn ramp(colors: &[Color], t: f32) -> [f32; 3] {
    match colors.len() {
        0 => [t; 3],
        1 => rgb(colors[0]),
        n => {
            let p = t.clamp(0.0, 1.0) * (n - 1) as f32;
            let i = (p.floor() as usize).min(n - 2);
            mix3(rgb(colors[i]), rgb(colors[i + 1]), p - i as f32)
        }
    }
}

fn hashi(x: i32, y: i32, seed: u32) -> f32 {
    hash(x as u32, y as u32, seed)
}

/// Fold a float seed into an integer hash seed.
fn fseed(seed: f32) -> u32 {
    seed.to_bits() ^ 0x9e37_79b9
}

/// Smooth (cubic) value noise in 0..1.
fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let (fx0, fy0) = (x.floor(), y.floor());
    let (fx, fy) = (x - fx0, y - fy0);
    let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (ix, iy) = (fx0 as i32, fy0 as i32);
    let a = hashi(ix, iy, seed);
    let b = hashi(ix.wrapping_add(1), iy, seed);
    let c = hashi(ix, iy.wrapping_add(1), seed);
    let d = hashi(ix.wrapping_add(1), iy.wrapping_add(1), seed);
    mix(mix(a, b, ux), mix(c, d, ux), uy)
}

/// Quintic-interpolated value noise in 0..1.
fn vnoise5(x: f32, y: f32, seed: u32) -> f32 {
    let (fx0, fy0) = (x.floor(), y.floor());
    let (fx, fy) = (x - fx0, y - fy0);
    let q = |f: f32| f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let (ux, uy) = (q(fx), q(fy));
    let (ix, iy) = (fx0 as i32, fy0 as i32);
    let a = hashi(ix, iy, seed);
    let b = hashi(ix.wrapping_add(1), iy, seed);
    let c = hashi(ix, iy.wrapping_add(1), seed);
    let d = hashi(ix.wrapping_add(1), iy.wrapping_add(1), seed);
    mix(mix(a, b, ux), mix(c, d, ux), uy)
}

/// Sum of `octaves` value-noise layers (not normalised).
fn fbm(mut x: f32, mut y: f32, octaves: u32, lacunarity: f32, gain: f32, seed: u32) -> f32 {
    let mut v = 0.0;
    let mut a = 0.5;
    for i in 0..octaves {
        v += vnoise(x, y, seed.wrapping_add(i * 7919)) * a;
        x = x * lacunarity + 17.1;
        y = y * lacunarity + 31.7;
        a *= gain;
    }
    v
}

/// Deterministic pseudo-random offset in 0..1 for a float seed.
fn seed_offset(seed: f32, salt: f32) -> [f32; 2] {
    let s = (seed + salt) as f64;
    let f = |v: f64| {
        let r = v.sin() * 43758.5453;
        (r - r.floor()) as f32
    };
    [f(s * 12.9898 + 78.233), f(s * 39.3468 + 11.135)]
}

#[inline]
fn px_at(src: &[u8], w: usize, x: usize, y: usize) -> [f32; 4] {
    let i = (y * w + x) * 4;
    [
        src[i] as f32 / 255.0,
        src[i + 1] as f32 / 255.0,
        src[i + 2] as f32 / 255.0,
        src[i + 3] as f32 / 255.0,
    ]
}

/// Bilinear sample at continuous pixel coordinates (pixel centres at `+0.5`),
/// clamped at the edges; normalised premultiplied RGBA.
#[inline]
fn tex(src: &[u8], w: usize, h: usize, fx: f32, fy: f32) -> [f32; 4] {
    let fx = if fx.is_finite() { fx } else { 0.0 };
    let fy = if fy.is_finite() { fy } else { 0.0 };
    sample(src, w, h, fx - 0.5, fy - 0.5).map(|v| v / 255.0)
}

/// Like [`tex`] but transparent outside the frame.
#[inline]
fn tex_clip(src: &[u8], w: usize, h: usize, fx: f32, fy: f32) -> [f32; 4] {
    if !(fx >= 0.0 && fy >= 0.0 && fx <= w as f32 && fy <= h as f32) {
        return [0.0; 4];
    }
    tex(src, w, h, fx, fy)
}

#[inline]
fn unpremul(c: [f32; 4]) -> [f32; 3] {
    if c[3] <= 1e-4 {
        [0.0; 3]
    } else {
        [c[0] / c[3], c[1] / c[3], c[2] / c[3]]
    }
}

#[inline]
fn premul(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0] * a, c[1] * a, c[2] * a, a]
}

#[inline]
fn store(p: &mut [u8; 4], c: [f32; 4]) {
    let a = if c[3].is_finite() {
        c[3].clamp(0.0, 1.0)
    } else {
        0.0
    };
    for (o, v) in p.iter_mut().zip(c).take(3) {
        let v = if v.is_finite() { v.clamp(0.0, a) } else { 0.0 };
        *o = (v * 255.0 + 0.5) as u8;
    }
    p[3] = (a * 255.0 + 0.5) as u8;
}

/// Rebuild every pixel from a copy of the source. `f(x, y, src)` returns
/// normalised premultiplied RGBA.
fn remap(pm: &mut Pixmap, f: impl Fn(usize, usize, &[u8]) -> [f32; 4] + Sync) {
    let w = pm.width() as usize;
    let src = pm.data().to_vec();
    pm.data_mut()
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                store(p, f(x, y, &src));
            }
        });
}

/// Composite a generated straight-alpha layer `f(x, y) -> [r, g, b, a]` over the
/// frame (or atop it when `mask`), optionally 2×2 supersampled.
fn generate(
    pm: &mut Pixmap,
    opacity: f32,
    mask: bool,
    supersample: bool,
    f: impl Fn(f32, f32) -> [f32; 4] + Sync,
) {
    let w = pm.width() as usize;
    let op = opacity.clamp(0.0, 1.0);
    const SS: [(f32, f32); 4] = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];
    pm.data_mut()
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let mut o = [0.0f32; 4];
                let taps: &[(f32, f32)] = if supersample { &SS } else { &[(0.5, 0.5)] };
                let k = 1.0 / taps.len() as f32;
                for (dx, dy) in taps {
                    let c = f(x as f32 + dx, y as f32 + dy);
                    let a = c[3].clamp(0.0, 1.0) * k;
                    o[0] += c[0] * a;
                    o[1] += c[1] * a;
                    o[2] += c[2] * a;
                    o[3] += a;
                }
                let o = o.map(|v| v * op);
                let d = p.map(|v| v as f32 / 255.0);
                let out = if mask {
                    [
                        o[0] * d[3] + d[0] * (1.0 - o[3]),
                        o[1] * d[3] + d[1] * (1.0 - o[3]),
                        o[2] * d[3] + d[2] * (1.0 - o[3]),
                        d[3],
                    ]
                } else {
                    [
                        o[0] + d[0] * (1.0 - o[3]),
                        o[1] + d[1] * (1.0 - o[3]),
                        o[2] + d[2] * (1.0 - o[3]),
                        o[3] + d[3] * (1.0 - o[3]),
                    ]
                };
                store(p, out);
            }
        });
}

/// Colour of a cyclic band pattern at `pos` (transparent in the gaps).
fn band(colors: &[Color], pos: f32, thickness: f32, gap: f32) -> [f32; 4] {
    if colors.is_empty() {
        return [0.0; 4];
    }
    let spacing = (thickness + gap).max(1e-3);
    let p = pos.rem_euclid(spacing * colors.len() as f32);
    let idx = ((p / spacing).floor() as usize).min(colors.len() - 1);
    if p - idx as f32 * spacing > thickness {
        return [0.0; 4];
    }
    rgba(colors[idx])
}

/// Rotate a vector by degrees.
#[inline]
fn rot(x: f32, y: f32, deg: f32) -> (f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (x * c - y * s, x * s + y * c)
}

/// Projection of `(u, v)` onto the segment `a → b` (0..1, clamped).
fn grad_t(u: f32, v: f32, a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let d = dx * dx + dy * dy;
    if d <= 0.0 {
        return 0.0;
    }
    (((u - a[0]) * dx + (v - a[1]) * dy) / d).clamp(0.0, 1.0)
}

/// Normalised elliptical distance (1 = on the ellipse edge).
fn ellipse_t(x: f32, y: f32, w: f32, h: f32, center: [f32; 2], ew: f32, eh: f32, deg: f32) -> f32 {
    let (dx, dy) = rot(x - center[0] * w, y - center[1] * h, -deg);
    let (rx, ry) = ((ew * w * 0.5).max(1e-3), (eh * h * 0.5).max(1e-3));
    ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt()
}

/// Blur a pixmap with a per-pixel σ (pixels) in `0..=max`, by blending a stack
/// of uniformly blurred copies.
fn progressive_blur(pm: &mut Pixmap, max: f32, sigma: impl Fn(f32, f32) -> f32 + Sync) {
    if max < 0.5 {
        return;
    }
    const LEVELS: usize = 6;
    let mut levels = vec![pm.data().to_vec()];
    for k in 1..=LEVELS {
        let mut l = pm.clone();
        blur(&mut l, max * k as f32 / LEVELS as f32);
        levels.push(l.data().to_vec());
    }
    let w = pm.width() as usize;
    pm.data_mut()
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let s = sigma(x as f32 + 0.5, y as f32 + 0.5).clamp(0.0, max);
                let f = s / max * LEVELS as f32;
                let i = (f.floor() as usize).min(LEVELS - 1);
                let a = px_at(&levels[i], w, x, y);
                let b = px_at(&levels[i + 1], w, x, y);
                store(p, mix4(a, b, f - i as f32));
            }
        });
}

/// Pixelate with a per-pixel block size (pixels).
fn progressive_pixelate(pm: &mut Pixmap, block: impl Fn(f32, f32) -> f32 + Sync) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    remap(pm, |x, y, src| {
        let b = block(x as f32 + 0.5, y as f32 + 0.5).max(1.0).round();
        if b <= 1.0 {
            return px_at(src, w, x, y);
        }
        let cx = ((x as f32 / b).floor() + 0.5) * b;
        let cy = ((y as f32 / b).floor() + 0.5) * b;
        let sx = (cx as usize).min(w - 1);
        let sy = (cy as usize).min(h - 1);
        px_at(src, w, sx, sy)
    });
}

/// 1D squared Euclidean distance transform (Felzenszwalb & Huttenlocher).
fn edt_1d(f: &[f32], out: &mut [f32]) {
    let n = f.len();
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f32; n + 1];
    let mut k = 0usize;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    for q in 1..n {
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * (q - p) as f32);
            if s <= z[k] && k > 0 {
                k -= 1;
            } else {
                k += 1;
                v[k] = q;
                z[k] = s;
                z[k + 1] = f32::INFINITY;
                break;
            }
        }
    }
    k = 0;
    for (q, o) in out.iter_mut().enumerate() {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let d = q as f32 - v[k] as f32;
        *o = d * d + f[v[k]];
    }
}

/// Distance (pixels) from each pixel to the nearest `inside` pixel.
fn distance_to(inside: &[bool], w: usize, h: usize) -> Vec<f32> {
    const BIG: f32 = 1e12;
    // columns
    let cols: Vec<Vec<f32>> = (0..w)
        .into_par_iter()
        .map(|x| {
            let f: Vec<f32> = (0..h)
                .map(|y| if inside[y * w + x] { 0.0 } else { BIG })
                .collect();
            let mut o = vec![0.0; h];
            edt_1d(&f, &mut o);
            o
        })
        .collect();
    let mut out = vec![0.0f32; w * h];
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let f: Vec<f32> = (0..w).map(|x| cols[x][y]).collect();
        edt_1d(&f, row);
        for v in row.iter_mut() {
            *v = v.sqrt();
        }
    });
    out
}

fn tv_pattern(u: f32, v: f32) -> [f32; 3] {
    const TOP: [[f32; 3]; 7] = [
        [0.74, 0.74, 0.74],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    const MID: [[f32; 3]; 7] = [
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.74, 0.74, 0.74],
    ];
    let i = ((u * 7.0).floor().max(0.0) as usize).min(6);
    if v < 0.64 {
        TOP[i]
    } else if v < 0.76 {
        MID[i]
    } else if v < 0.88 {
        if u < 0.58 {
            [1.0; 3]
        } else {
            [1.0 - smoothstep(0.58, 1.0, u); 3]
        }
    } else if u < 0.58 {
        [(u * 12.0).floor() / 11.0; 3]
    } else {
        [0.0; 3]
    }
}

fn leak_pattern(u: f32, v: f32, s: f32, t: f32) -> [f32; 3] {
    let mut px = u * 0.8 + (s * 1.61803).sin() * 5.0;
    let mut py = v * 0.8 + (s * std::f32::consts::E).cos() * 5.0;
    for i in 1..5 {
        let fi = i as f32;
        let phase = s * 0.7 * fi;
        let nx = px + 0.6 / fi * (fi * py + t * 0.7 + 0.3 * fi + phase).cos() + 20.0;
        let ny = py + 0.6 / fi * (fi * px + t * 0.7 + 0.3 * (fi + 10.0) + phase).cos() - 5.0;
        px = nx;
        py = ny;
    }
    let v1 = 0.5 * (2.0 * px).sin() + 0.5;
    let v2 = 0.5 * (2.0 * py).sin() + 0.5;
    let blend = (px + py).sin() * 0.5 + 0.5;
    let brightness = v1 * 0.5 + v2 * 0.5;
    [brightness, blend, brightness * 0.6 + blend * 0.4]
}

// ===================================================================== new effects

impl Effect {
    /// The effects added for `@remotion/effects` parity (called from [`Effect::apply`]).
    fn apply_ext(&self, pm: &mut Pixmap) {
        let (w, h) = (pm.width() as usize, pm.height() as usize);
        let (wf, hf) = (w as f32, h as f32);
        match self {
            Effect::DuotoneThreshold {
                dark,
                light,
                threshold,
            } => {
                let (d, l, t) = (rgb(*dark), rgb(*light), *threshold);
                color_map(pm, move |c| if luma601(c) < t { d } else { l })
            }
            Effect::Vibrance(a) => {
                let a = a.clamp(-1.0, 1.0);
                color_map(pm, move |c| vibrance(c, a))
            }
            Effect::WhiteBalance { temperature, tint } => {
                let (t, n) = (*temperature, *tint);
                color_map(pm, move |c| white_balance(c, t, n))
            }
            Effect::ShadowsHighlights {
                shadows,
                highlights,
            } => {
                let (s, hl) = (*shadows, *highlights);
                color_map(pm, move |c| {
                    exposure_stops(c, shadows_highlights_stops(c, s, hl))
                })
            }
            Effect::ColorCorrection(p) => {
                let p = *p;
                color_map(pm, move |c| color_correct(c, &p))
            }
            Effect::ThermalVision { amount, palette } => {
                let pal = if palette.is_empty() {
                    default_thermal_palette()
                } else {
                    palette.clone()
                };
                let a = amount.clamp(0.0, 1.0);
                color_map(pm, move |c| mix3(c, ramp(&pal, luma601(c)), a))
            }
            Effect::TvSignalOff(a) => {
                let a = a.clamp(0.0, 1.0);
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    let bars = tv_pattern((x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf);
                    premul(mix3(unpremul(s), bars, a), s[3])
                })
            }
            Effect::WhiteNoise { amount, seed } => {
                let (a, seed) = (amount.clamp(0.0, 1.0), *seed);
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    let n = hash(x as u32, y as u32, seed);
                    premul(mix3(unpremul(s), [n; 3], a), s[3])
                })
            }
            Effect::Noise {
                amount,
                seed,
                premultiply,
            } => {
                let (a, seed, pmul) = (*amount, *seed, *premultiply);
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    if s[3] <= 0.001 {
                        return [0.0; 4];
                    }
                    let n = hash(x as u32, y as u32, seed ^ 0x51ed) - 0.5;
                    let c = unpremul(s);
                    let o = c.map(|v| (v + if pmul { v * n } else { n } * a).clamp(0.0, 1.0));
                    premul(o, s[3])
                })
            }
            Effect::ColorKey {
                key,
                similarity,
                smoothness,
                spill,
            } => {
                let k = rgb(*key);
                let (sim, smooth, spill) = (*similarity, *smoothness, *spill);
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    if s[3] <= 0.001 {
                        return [0.0; 4];
                    }
                    let mut c = unpremul(s);
                    let d = ((c[0] - k[0]).powi(2) + (c[1] - k[1]).powi(2) + (c[2] - k[2]).powi(2))
                        .sqrt()
                        / 3f32.sqrt();
                    let (e0, e1) = ((sim - smooth).max(0.0), (sim + smooth).min(1.0));
                    let keep = if smooth <= 1e-4 || e1 <= e0 + 1e-4 {
                        if d < sim {
                            0.0
                        } else {
                            1.0
                        }
                    } else {
                        smoothstep(e0, e1, d)
                    };
                    if spill > 0.0 {
                        // suppress the dominant key channel
                        let dom = if k[1] > k[0] && k[1] > k[2] {
                            Some((1, 0, 2))
                        } else if k[0] > k[1] && k[0] > k[2] {
                            Some((0, 1, 2))
                        } else if k[2] > k[0] && k[2] > k[1] {
                            Some((2, 0, 1))
                        } else {
                            None
                        };
                        if let Some((i, a, b)) = dom {
                            let limit = (c[a] + c[b]) * 0.5;
                            c[i] = mix(c[i], c[i].min(limit), spill);
                        }
                    }
                    premul(c, s[3] * keep)
                })
            }
            Effect::Lut(lut) => {
                let lut = lut.clone();
                color_map(pm, move |c| lut.lookup(c))
            }
            Effect::LinearGradientTint {
                start,
                end,
                start_color,
                end_color,
                amount,
            } => {
                let (a, b, amt) = (rgba(*start_color), rgba(*end_color), amount.clamp(0.0, 1.0));
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    let t = grad_t((x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf, *start, *end);
                    let col = mix4(a, b, t);
                    premul(
                        mix3(unpremul(s), [col[0], col[1], col[2]], amt * col[3]),
                        s[3],
                    )
                })
            }
            Effect::VignetteMask {
                amount,
                radius,
                feather,
                roundness,
                color,
                fade_alpha,
                center,
            } => {
                let col = rgba(*color);
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    let cx = ((x as f32 + 0.5) / wf - center[0]).abs() * 2.0;
                    let cy = ((y as f32 + 0.5) / hf - center[1]).abs() * 2.0;
                    let d = mix(cx.max(cy), (cx * cx + cy * cy).sqrt(), *roundness);
                    let m = if *feather <= 1e-4 {
                        if d >= *radius {
                            *amount
                        } else {
                            0.0
                        }
                    } else {
                        smoothstep(*radius, radius + feather, d) * amount
                    };
                    if *fade_alpha {
                        s.map(|v| v * (1.0 - m))
                    } else {
                        let oa = m * col[3];
                        [
                            col[0] * oa + s[0] * (1.0 - oa),
                            col[1] * oa + s[1] * (1.0 - oa),
                            col[2] * oa + s[2] * (1.0 - oa),
                            oa + s[3] * (1.0 - oa),
                        ]
                    }
                })
            }

            // ------------------------------------------------------------ blur / light
            Effect::BlurAxis {
                sigma,
                horizontal,
                vertical,
            } => {
                let s = sigma * hf;
                if s < 0.5 {
                    return;
                }
                let boxes = boxes_for_gauss(s, 3);
                let data = pm.data_mut();
                let mut tmp = vec![0u8; data.len()];
                for r in boxes {
                    if *horizontal {
                        box_blur_h(data, &mut tmp, w, h, r);
                        data.copy_from_slice(&tmp);
                    }
                    if *vertical {
                        box_blur_v(data, &mut tmp, w, h, r);
                        data.copy_from_slice(&tmp);
                    }
                }
            }
            Effect::Glow {
                radius,
                intensity,
                threshold,
                color,
            } => {
                let col = rgba(*color);
                let th = threshold.clamp(0.0, 1.0);
                let mut layer = pm.clone();
                for p in layer.data_mut().as_chunks_mut::<4>().0 {
                    let s = p.map(|v| v as f32 / 255.0);
                    if s[3] <= 0.001 {
                        *p = [0; 4];
                        continue;
                    }
                    let l = luma601(unpremul(s));
                    let contr = if th >= 1.0 {
                        if l >= 1.0 {
                            1.0
                        } else {
                            0.0
                        }
                    } else {
                        ((l - th) / (1.0 - th).max(1e-4)).clamp(0.0, 1.0)
                    };
                    let ga = s[3] * contr * col[3];
                    store(p, premul([col[0], col[1], col[2]], ga));
                }
                blur(&mut layer, radius * hf);
                let k = intensity.max(0.0);
                let g = layer.data();
                for (p, q) in pm
                    .data_mut()
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(g.as_chunks::<4>().0)
                {
                    let s = p.map(|v| v as f32 / 255.0);
                    let o = [0, 1, 2, 3].map(|i| (s[i] + q[i] as f32 / 255.0 * k).min(1.0));
                    store(p, o);
                }
            }
            Effect::LightTrail {
                direction,
                distance,
                intensity,
                decay,
                threshold,
                samples,
                color,
            } => {
                let (sn, cs) = direction.to_radians().sin_cos();
                let dir = (cs, -sn);
                let dist = distance * hf;
                let n = (*samples).clamp(1, 64);
                let col = rgba(*color);
                let th = threshold.clamp(0.0, 1.0);
                let dec = decay.clamp(0.0, 1.0);
                remap(pm, |x, y, src| {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let s = px_at(src, w, x, y);
                    let mut trail = [0.0f32; 4];
                    for i in 1..=n {
                        let pr = i as f32 / n as f32;
                        let c = tex(src, w, h, fx - dir.0 * dist * pr, fy - dir.1 * dist * pr);
                        if c[3] <= 0.001 {
                            continue;
                        }
                        let src_v = c[3].max(luma601(unpremul(c)));
                        let contr = if th >= 1.0 {
                            if src_v >= 1.0 {
                                1.0
                            } else {
                                0.0
                            }
                        } else {
                            ((src_v - th) / (1.0 - th).max(1e-4)).clamp(0.0, 1.0)
                        };
                        let wgt = dec.powi(i as i32 - 1) * contr;
                        let a = c[3] * col[3] * wgt;
                        trail[0] += col[0] * a;
                        trail[1] += col[1] * a;
                        trail[2] += col[2] * a;
                        trail[3] += a;
                    }
                    let k = intensity.max(0.0) / n as f32;
                    [0, 1, 2, 3].map(|i| (s[i] + trail[i] * k).min(1.0))
                })
            }
            Effect::RegionBlur {
                top_left,
                bottom_right,
                radius,
                feather,
                roundness,
            } => {
                let mut blurred = pm.clone();
                blur(&mut blurred, radius * hf);
                let b = blurred.data().to_vec();
                let (x0, y0) = (top_left[0] * wf, top_left[1] * hf);
                let (x1, y1) = (bottom_right[0] * wf, bottom_right[1] * hf);
                let (cx, cy) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
                let (hx, hy) = (((x1 - x0) * 0.5).abs(), ((y1 - y0) * 0.5).abs());
                let r = roundness.clamp(0.0, 1.0) * hx.min(hy);
                let fe = (feather * hf).max(1.0);
                remap(pm, |x, y, src| {
                    let (px, py) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                    let qx = px.abs() - hx + r;
                    let qy = py.abs() - hy + r;
                    let sdf = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt()
                        + qx.max(qy).min(0.0)
                        - r;
                    let m = 1.0 - smoothstep(-fe * 0.5, fe * 0.5, sdf);
                    mix4(px_at(src, w, x, y), px_at(&b, w, x, y), m)
                })
            }
            Effect::LinearProgressiveBlur {
                start,
                end,
                start_blur,
                end_blur,
            } => {
                let (s0, s1) = (start_blur * hf, end_blur * hf);
                progressive_blur(pm, s0.max(s1), |x, y| {
                    mix(s0, s1, grad_t(x / wf, y / hf, *start, *end))
                })
            }
            Effect::RadialProgressiveBlur {
                center,
                width,
                height,
                rotation,
                start,
                start_blur,
                end_blur,
            } => {
                let (s0, s1) = (start_blur * hf, end_blur * hf);
                progressive_blur(pm, s0.max(s1), |x, y| {
                    let e = ellipse_t(x, y, wf, hf, *center, *width, *height, *rotation);
                    let t = ((e - start) / (1.0 - start).max(1e-4)).clamp(0.0, 1.0);
                    mix(s0, s1, t)
                })
            }
            Effect::LinearProgressivePixelate {
                start,
                end,
                start_block,
                end_block,
            } => {
                let (b0, b1) = (start_block * hf, end_block * hf);
                progressive_pixelate(pm, |x, y| mix(b0, b1, grad_t(x / wf, y / hf, *start, *end)))
            }
            Effect::RadialProgressivePixelate {
                center,
                width,
                height,
                rotation,
                start,
                start_block,
                end_block,
            } => {
                let (b0, b1) = (start_block * hf, end_block * hf);
                progressive_pixelate(pm, |x, y| {
                    let e = ellipse_t(x, y, wf, hf, *center, *width, *height, *rotation);
                    let t = ((e - start) / (1.0 - start).max(1e-4)).clamp(0.0, 1.0);
                    mix(b0, b1, t)
                })
            }

            // ------------------------------------------------------------ geometry
            Effect::BarrelDistortion(a) => remap(pm, |x, y, src| {
                let cx = (x as f32 + 0.5) / wf * 2.0 - 1.0;
                let cy = (y as f32 + 0.5) / hf * 2.0 - 1.0;
                let u = (cx * (1.0 + a * cy * cy)) * 0.5 + 0.5;
                let v = (cy * (1.0 + a * cx * cx)) * 0.5 + 0.5;
                if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                    return [0.0; 4];
                }
                tex(src, w, h, u * wf, v * hf)
            }),
            Effect::CornerPin {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            } => {
                let inv = corner_pin_inverse(*top_left, *top_right, *bottom_right, *bottom_left);
                remap(pm, |x, y, src| {
                    let Some(m) = inv else { return [0.0; 4] };
                    let (px, py) = ((x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf);
                    let ww = m[6] * px + m[7] * py + m[8];
                    if ww.abs() < 1e-7 {
                        return [0.0; 4];
                    }
                    let u = (m[0] * px + m[1] * py + m[2]) / ww;
                    let v = (m[3] * px + m[4] * py + m[5]) / ww;
                    if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                        return [0.0; 4];
                    }
                    tex(src, w, h, u * wf, v * hf)
                })
            }
            Effect::FisheyeLens {
                fov,
                center,
                radius,
                zoom,
            } => {
                let aspect = wf / hf;
                remap(pm, |x, y, src| {
                    let (u, v) = ((x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf);
                    let (tx, ty) = ((u - center[0]) * aspect, v - center[1]);
                    let dist = (tx * tx + ty * ty).sqrt();
                    let r = dist / radius.max(1e-4);
                    if r > 1.0 || *fov <= 1e-4 {
                        return px_at(src, w, x, y);
                    }
                    let half = (fov * 0.5).min(PI * 0.5 - 1e-3);
                    let warped = (r * half).tan() / half.tan();
                    let sc = if dist > 1e-5 {
                        warped * radius / dist
                    } else {
                        0.0
                    };
                    let su = center[0] + tx * sc / aspect / zoom.max(1e-4);
                    let sv = center[1] + ty * sc / zoom.max(1e-4);
                    if !(0.0..=1.0).contains(&su) || !(0.0..=1.0).contains(&sv) {
                        return [0.0; 4];
                    }
                    tex(src, w, h, su * wf, sv * hf)
                })
            }
            Effect::MirrorAt {
                horizontal,
                position,
                invert,
            } => remap(pm, |x, y, src| {
                let (mut u, mut v) = ((x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf);
                let c = if *horizontal { u } else { v };
                let flip = if *invert {
                    c < *position
                } else {
                    c > *position
                };
                if flip {
                    let m = 2.0 * position - c;
                    if *horizontal {
                        u = m;
                    } else {
                        v = m;
                    }
                }
                tex(src, w, h, u.clamp(0.0, 1.0) * wf, v.clamp(0.0, 1.0) * hf)
            }),
            Effect::WaveDisplace {
                amplitude,
                wavelength,
                phase,
                horizontal,
            } => {
                let (a, wl) = (amplitude * hf, (wavelength * hf).max(1e-3));
                remap(pm, |x, y, src| {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    if *horizontal {
                        let o = (fx / wl * TAU + phase).sin() * a;
                        tex(src, w, h, fx, fy - o)
                    } else {
                        let o = (fy / wl * TAU + phase).sin() * a;
                        tex(src, w, h, fx - o, fy)
                    }
                })
            }
            Effect::ChromaticSplit { amount, angle } => {
                let (sn, cs) = angle.to_radians().sin_cos();
                let (ox, oy) = (cs * amount * hf, -sn * amount * hf);
                remap(pm, |x, y, src| {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let c = px_at(src, w, x, y);
                    let r = tex(src, w, h, fx - ox, fy - oy);
                    let b = tex(src, w, h, fx + ox, fy + oy);
                    [r[0], c[1], b[2], c[3].max(r[3]).max(b[3])]
                })
            }
            Effect::Scale { x: sx, y: sy } => remap(pm, |x, y, src| {
                let (cx, cy) = (wf * 0.5, hf * 0.5);
                let fx = cx + (x as f32 + 0.5 - cx) / if sx.abs() < 1e-4 { 1e-4 } else { *sx };
                let fy = cy + (y as f32 + 0.5 - cy) / if sy.abs() < 1e-4 { 1e-4 } else { *sy };
                tex_clip(src, w, h, fx, fy)
            }),
            Effect::Skew {
                x: ax,
                y: ay,
                origin,
            } => {
                let (kx, ky) = (ax.to_radians().tan(), ay.to_radians().tan());
                remap(pm, |x, y, src| {
                    let dx = x as f32 + 0.5 - origin[0] * wf;
                    let dy = y as f32 + 0.5 - origin[1] * hf;
                    let sy = dy - ky * dx;
                    let sx = dx - kx * sy;
                    tex_clip(src, w, h, sx + origin[0] * wf, sy + origin[1] * hf)
                })
            }
            Effect::Translate { x: tx, y: ty } => remap(pm, |x, y, src| {
                tex_clip(
                    src,
                    w,
                    h,
                    x as f32 + 0.5 - tx * wf,
                    y as f32 + 0.5 - ty * hf,
                )
            }),
            Effect::Tile {
                horizontal,
                vertical,
            } => tile(pm, *horizontal, *vertical),
            Effect::Pattern(p) => pattern(pm, p),
            Effect::Tear {
                progress,
                angle,
                rotation,
                jaggedness,
            } => tear(pm, *progress, *angle, *rotation, jaggedness * hf),
            Effect::NoiseDisplacement(p) => noise_displacement(pm, p),

            // ------------------------------------------------------------ alpha / reveal
            Effect::DropShadow {
                radius,
                offset_x,
                offset_y,
                opacity,
                color,
            } => {
                let col = rgba(*color);
                let (ox, oy) = (offset_x * hf, offset_y * hf);
                let k = opacity.clamp(0.0, 1.0) * col[3];
                let mut shadow = pm.clone();
                remap(&mut shadow, |x, y, src| {
                    let a = tex_clip(src, w, h, x as f32 + 0.5 - ox, y as f32 + 0.5 - oy)[3] * k;
                    premul([col[0], col[1], col[2]], a)
                });
                blur(&mut shadow, radius * hf);
                let sh = shadow.data();
                for (p, q) in pm
                    .data_mut()
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(sh.as_chunks::<4>().0)
                {
                    let s = p.map(|v| v as f32 / 255.0);
                    let d = q.map(|v| v as f32 / 255.0);
                    store(p, [0, 1, 2, 3].map(|i| s[i] + d[i] * (1.0 - s[3])));
                }
            }
            Effect::Outline {
                width,
                color,
                opacity,
                outline_only,
            } => {
                let inside: Vec<bool> = pm
                    .data()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|p| p[3] >= 128)
                    .collect();
                let dist = distance_to(&inside, w, h);
                let wd = width * hf;
                let col = rgba(*color);
                let op = opacity.clamp(0.0, 1.0) * col[3];
                let c3 = [col[0], col[1], col[2]];
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    let d = dist[y * w + x];
                    let cov = if d > 1e9 {
                        0.0
                    } else {
                        (wd - d + 1.0).clamp(0.0, 1.0)
                    };
                    if *outline_only {
                        premul(c3, cov.max(s[3]) * op)
                    } else {
                        let o = premul(c3, cov * op);
                        [0, 1, 2, 3].map(|i| s[i] + o[i] * (1.0 - s[3]))
                    }
                })
            }
            Effect::RoughenEdges {
                amount,
                border,
                scale,
                seed,
            } => roughen_edges(pm, *amount, border * hf, *scale, *seed),
            Effect::Shrinkwrap {
                amount,
                displacement,
                highlight,
                wrinkle_density,
                edge_tension,
                phase,
                seed,
            } => shrinkwrap(
                pm,
                &ShrinkwrapCfg {
                    amount: *amount,
                    displacement: displacement * hf,
                    highlight: *highlight,
                    density: wrinkle_density.clamp(0.0, 1.0),
                    edge_tension: edge_tension.clamp(0.0, 1.0),
                    phase: *phase,
                    seed: *seed,
                },
            ),
            Effect::Speckle {
                density,
                size,
                randomness,
                seed,
            } => {
                let sz = size * hf;
                if *density <= 0.0 || sz <= 0.0 {
                    return;
                }
                let cell = (sz * 2.5).max(1.0);
                let seed = *seed;
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let (bx, by) = ((fx / cell).floor() as i32, (fy / cell).floor() as i32);
                    let mut hole = 0.0f32;
                    for j in -1..=1 {
                        for i in -1..=1 {
                            let (cx, cy) = (bx + i, by + j);
                            if hashi(cx, cy, seed) > *density {
                                continue;
                            }
                            let ox = (hashi(cx, cy, seed ^ 0x1171) - 0.5) * randomness;
                            let oy = (hashi(cx, cy, seed ^ 0x8371) - 0.5) * randomness;
                            let px = (cx as f32 + 0.5 + ox) * cell;
                            let py = (cy as f32 + 0.5 + oy) * cell;
                            let var =
                                mix(1.0, 0.35 + hashi(cx, cy, seed ^ 0x2943) * 0.65, *randomness);
                            let r = sz * 0.5 * var;
                            let e = (r * 0.5).clamp(0.35, 1.0);
                            let d = ((fx - px).powi(2) + (fy - py).powi(2)).sqrt();
                            hole = hole.max(1.0 - smoothstep(r - e, r + e, d));
                        }
                    }
                    s.map(|v| v * (1.0 - hole.clamp(0.0, 1.0)))
                })
            }
            Effect::DotGrid {
                dot_size,
                grid_size,
                invert,
            } => {
                let g = (grid_size * hf).max(1e-3);
                let r = dot_size * hf * 0.5;
                remap(pm, |x, y, src| {
                    let gx = x as f32 + 0.5 - wf * 0.5;
                    let gy = y as f32 + 0.5 - hf * 0.5;
                    let dx = gx - (gx / g + 0.5).floor() * g;
                    let dy = gy - (gy / g + 0.5).floor() * g;
                    let cov = if r <= 0.005 {
                        0.0
                    } else {
                        1.0 - smoothstep(r - 0.75, r + 0.75, (dx * dx + dy * dy).sqrt())
                    };
                    let cov = if *invert { 1.0 - cov } else { cov };
                    px_at(src, w, x, y).map(|v| v * cov)
                })
            }
            Effect::Evolve {
                progress,
                from,
                feather,
            } => {
                let p = progress.clamp(0.0, 1.0);
                let f = feather.max(0.0);
                remap(pm, |x, y, src| {
                    let (u, v) = ((x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf);
                    let t = match from {
                        RevealFrom::Left => u,
                        RevealFrom::Right => 1.0 - u,
                        RevealFrom::Top => v,
                        RevealFrom::Bottom => 1.0 - v,
                    };
                    let m = if p <= 1e-6 {
                        0.0
                    } else if p >= 1.0 - 1e-6 {
                        1.0
                    } else if f <= 1e-4 {
                        if t <= p {
                            1.0
                        } else {
                            0.0
                        }
                    } else {
                        1.0 - smoothstep(p, (p + f).min(1.0), t)
                    };
                    px_at(src, w, x, y).map(|c| c * m)
                })
            }
            Effect::PixelDissolve {
                progress,
                columns,
                rows,
                seed,
                feather,
            } => {
                let (cols, rws) = ((*columns).max(1) as f32, (*rows).max(1) as f32);
                let e = feather.max(0.0);
                let th = 1.0 - progress.clamp(0.0, 1.0) * (1.0 + e);
                remap(pm, |x, y, src| {
                    let cx = ((x as f32 + 0.5) / wf * cols).floor().min(cols - 1.0) as i32;
                    let cy = ((y as f32 + 0.5) / hf * rws).floor().min(rws - 1.0) as i32;
                    let n = hashi(cx, cy, *seed ^ 0xd155);
                    let m = if e <= 1e-4 {
                        if n < th {
                            1.0
                        } else {
                            0.0
                        }
                    } else {
                        1.0 - smoothstep(th, th + e, n)
                    };
                    px_at(src, w, x, y).map(|c| c * m)
                })
            }
            Effect::VenetianBlinds {
                progress,
                vertical,
                slats,
            } => {
                let p = progress.clamp(0.0, 1.0);
                let n = (*slats).max(1) as f32;
                remap(pm, |x, y, src| {
                    let axis = if *vertical {
                        (x as f32 + 0.5) / wf
                    } else {
                        (y as f32 + 0.5) / hf
                    };
                    let d = (fract(axis * n) - 0.5).abs() * 2.0;
                    let m = if p <= 1e-6 {
                        0.0
                    } else if p >= 1.0 - 1e-6 || d <= p {
                        1.0
                    } else {
                        0.0
                    };
                    px_at(src, w, x, y).map(|c| c * m)
                })
            }
            Effect::Shine {
                progress,
                angle,
                halo,
                core,
                halo_intensity,
                core_intensity,
            } => {
                let (sn, cs) = angle.to_radians().sin_cos();
                let (nx, ny) = (cs, -sn);
                let (hs, cs_) = ((halo * hf).max(1e-3), (core * hf).max(1e-3));
                let half = (nx.abs() * wf + ny.abs() * hf) / 2.0 + 3.0 * hs;
                let bt = -half + progress * half * 2.0;
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    let d =
                        (x as f32 + 0.5 - wf * 0.5) * nx + (y as f32 + 0.5 - hf * 0.5) * ny - bt;
                    let i = ((-(d * d) / (hs * hs)).exp() * halo_intensity
                        + (-(d * d) / (cs_ * cs_)).exp() * core_intensity)
                        .clamp(0.0, 1.0);
                    [
                        s[0] * (1.0 - i) + i * s[3],
                        s[1] * (1.0 - i) + i * s[3],
                        s[2] * (1.0 - i) + i * s[3],
                        s[3],
                    ]
                })
            }

            // ------------------------------------------------------------ generators
            Effect::Lines {
                colors,
                vertical,
                thickness,
                gap,
                angle,
                offset,
                opacity,
                mask,
            } => {
                let (t, g, o) = (thickness * hf, gap * hf, offset * hf);
                generate(pm, *opacity, *mask, true, |x, y| {
                    let (rx, ry) = rot(x - wf * 0.5, y - hf * 0.5, *angle);
                    band(colors, if *vertical { rx } else { ry } + o, t, g)
                })
            }
            Effect::Checkerboard {
                colors,
                cell,
                gap,
                angle,
                offset,
                opacity,
                mask,
            } => {
                let (c, g) = (cell * hf, gap * hf);
                let sp = (c + g).max(1e-3);
                generate(pm, *opacity, *mask, true, |x, y| {
                    if colors.is_empty() {
                        return [0.0; 4];
                    }
                    let (rx, ry) = rot(x - wf * 0.5, y - hf * 0.5, *angle);
                    let (px, py) = (rx + offset[0] * hf, ry + offset[1] * hf);
                    let (ix, iy) = ((px / sp).floor(), (py / sp).floor());
                    if px - ix * sp > c || py - iy * sp > c {
                        return [0.0; 4];
                    }
                    let k = ((ix + iy) as i64).rem_euclid(colors.len() as i64) as usize;
                    rgba(colors[k])
                })
            }
            Effect::Rings {
                colors,
                center,
                thickness,
                gap,
                offset,
                opacity,
                mask,
            } => {
                let (t, g, o) = (thickness * hf, gap * hf, offset * hf);
                generate(pm, *opacity, *mask, true, |x, y| {
                    let r = ((x - center[0] * wf).powi(2) + (y - center[1] * hf).powi(2)).sqrt();
                    band(colors, r + o, t, g)
                })
            }
            Effect::Waves {
                colors,
                vertical,
                thickness,
                gap,
                angle,
                offset,
                amplitude,
                wavelength,
                phase,
                opacity,
                mask,
            } => {
                let (t, g, o) = (thickness * hf, gap * hf, offset * hf);
                let (a, wl) = (amplitude * hf, (wavelength * hf).max(1e-3));
                let ph = phase.to_radians();
                generate(pm, *opacity, *mask, true, |x, y| {
                    let (rx, ry) = rot(x - wf * 0.5, y - hf * 0.5, *angle);
                    let (base, along) = if *vertical { (rx, ry) } else { (ry, rx) };
                    band(colors, base + (along / wl * TAU + ph).sin() * a + o, t, g)
                })
            }
            Effect::Zigzag {
                colors,
                vertical,
                thickness,
                gap,
                angle,
                offset,
                amplitude,
                wavelength,
                opacity,
                mask,
            } => {
                let (t, g, o) = (thickness * hf, gap * hf, offset * hf);
                let (a, wl) = (amplitude * hf, (wavelength * hf).max(1e-3));
                generate(pm, *opacity, *mask, true, |x, y| {
                    let (rx, ry) = rot(x - wf * 0.5, y - hf * 0.5, *angle);
                    let (base, along) = if *vertical { (rx, ry) } else { (ry, rx) };
                    let ph = fract(along / wl);
                    let z = (1.0 - (ph * 2.0 - 1.0).abs() * 2.0) * a;
                    band(colors, base + z + o, t, g)
                })
            }
            Effect::Gridlines(p) => gridlines(pm, p),
            Effect::Starburst {
                rays,
                colors,
                rotation,
                smoothness,
                origin,
                opacity,
            } => {
                let n = (*rays).max(2) as f32;
                let nc = colors.len().max(1);
                let col = |i: f32| -> [f32; 3] {
                    if colors.is_empty() {
                        return [1.0; 3];
                    }
                    rgb(colors[(i as i64).rem_euclid(nc as i64) as usize])
                };
                let rot_rad = rotation.to_radians();
                let edge = smoothness.clamp(0.0, 1.0) * 0.5;
                generate(pm, *opacity, false, true, |x, y| {
                    let cx = (x / wf - origin[0]) * wf / hf;
                    let cy = y / hf - origin[1];
                    let a = cy.atan2(cx) + rot_rad;
                    let sector = (a + PI) / TAU * n;
                    let ri = sector.floor().rem_euclid(n);
                    let fs = fract(sector);
                    let mut c = col(ri);
                    c = mix3(c, col(ri + 1.0), smoothstep(1.0 - edge, 1.0, fs));
                    c = mix3(c, col(ri - 1.0), smoothstep(edge, 0.0, fs));
                    [c[0], c[1], c[2], 1.0]
                })
            }
            Effect::LinearGradient {
                start,
                end,
                start_color,
                end_color,
                opacity,
            } => {
                let (a, b) = (rgba(*start_color), rgba(*end_color));
                generate(pm, *opacity, false, false, |x, y| {
                    mix4(a, b, grad_t(x / wf, y / hf, *start, *end))
                })
            }
            Effect::HalftoneGrid {
                shape,
                size,
                spacing,
                rotation,
                offset,
                color,
                invert,
            } => {
                let ds = size * hf;
                let sp = (spacing * hf).max(1.0);
                let (sr, cr) = rotation.to_radians().sin_cos();
                let (ox, oy) = (offset[0] * hf, offset[1] * hf);
                let dot_scale = |c: [f32; 4]| {
                    let l = luma601(unpremul(c)) * c[3] + (1.0 - c[3]);
                    if *invert {
                        l
                    } else {
                        1.0 - l
                    }
                };
                remap(pm, |x, y, src| {
                    let (dx, dy) = (x as f32 + 0.5 - wf * 0.5, y as f32 + 0.5 - hf * 0.5);
                    let (gx, gy) = (dx * cr + dy * sr, -dx * sr + dy * cr);
                    let (ix, iy) = (
                        ((gx + ox) / sp + 0.5).floor(),
                        ((gy + oy) / sp + 0.5).floor(),
                    );
                    let (ccx, ccy) = (ix * sp - ox, iy * sp - oy);
                    let px = wf * 0.5 + ccx * cr - ccy * sr;
                    let py = hf * 0.5 + ccx * sr + ccy * cr;
                    let tc = tex(src, w, h, px.clamp(0.0, wf), py.clamp(0.0, hf));
                    let k = dot_scale(tc);
                    if k <= 0.01 {
                        return [0.0; 4];
                    }
                    let (ddx, ddy) = (gx - ccx, gy - ccy);
                    let half = ds * 0.5;
                    let cov = match shape {
                        HalftoneShape::Circle => {
                            let r = half * k;
                            1.0 - smoothstep(r - 0.75, r + 0.75, (ddx * ddx + ddy * ddy).sqrt())
                        }
                        HalftoneShape::Square => {
                            let s = ds * k * 0.5;
                            (1.0 - smoothstep(s - 0.5, s + 0.5, ddx.abs()))
                                * (1.0 - smoothstep(s - 0.5, s + 0.5, ddy.abs()))
                        }
                        HalftoneShape::Line => {
                            if dot_scale(px_at(src, w, x, y)) <= 0.12 {
                                return [0.0; 4];
                            }
                            let lh = ds * k * 0.5;
                            if lh <= 1.0 {
                                return [0.0; 4];
                            }
                            (1.0 - smoothstep(half - 0.5, half + 0.5, ddx.abs()))
                                * (1.0 - smoothstep(lh - 0.5, lh + 0.5, ddy.abs()))
                        }
                    };
                    match color {
                        Some(c) => premul(rgb(*c), c.a * cov),
                        None => tc.map(|v| v * cov),
                    }
                })
            }
            Effect::HalftoneGradient {
                start_size,
                end_size,
                start,
                end,
                grid,
                color,
                mask,
            } => {
                let g = (grid * hf).max(1e-3);
                remap(pm, |x, y, src| {
                    let (gx, gy) = (x as f32 + 0.5 - wf * 0.5, y as f32 + 0.5 - hf * 0.5);
                    let (ccx, ccy) = ((gx / g + 0.5).floor() * g, (gy / g + 0.5).floor() * g);
                    let (cu, cv) = ((ccx + wf * 0.5) / wf, (ccy + hf * 0.5) / hf);
                    let t = grad_t(cu, cv, *start, *end);
                    let ds = mix(*start_size, *end_size, t) * hf;
                    let s = px_at(src, w, x, y);
                    let tc = tex(src, w, h, cu.clamp(0.0, 1.0) * wf, cv.clamp(0.0, 1.0) * hf);
                    let cov = if ds <= 0.01 {
                        0.0
                    } else {
                        let r = ds * 0.5;
                        let d = ((gx - ccx).powi(2) + (gy - ccy).powi(2)).sqrt();
                        1.0 - smoothstep(r - 0.75, r + 0.75, d)
                    };
                    match color {
                        None => tc.map(|v| v * cov),
                        Some(c) => {
                            let o = premul(rgb(*c), c.a * cov);
                            if *mask {
                                [
                                    o[0] * s[3] + s[0] * (1.0 - o[3]),
                                    o[1] * s[3] + s[1] * (1.0 - o[3]),
                                    o[2] * s[3] + s[2] * (1.0 - o[3]),
                                    s[3],
                                ]
                            } else {
                                [0, 1, 2, 3].map(|i| o[i] + s[i] * (1.0 - o[3]))
                            }
                        }
                    }
                })
            }
            Effect::ContourLines(p) => contour_lines(pm, p),
            Effect::LiquidContours(p) => liquid_contours(pm, p),
            Effect::LightLeak {
                seed,
                hue_shift,
                progress,
            } => {
                let ev = (progress * 2.0).clamp(0.0, 1.0);
                let re = (progress * 2.0 - 1.0).clamp(0.0, 1.0);
                let (sa, ca) = hue_shift.to_radians().sin_cos();
                let (a, b, c) = (
                    ca + (1.0 - ca) / 3.0,
                    (1.0 - ca) / 3.0 - sa * 0.57735,
                    (1.0 - ca) / 3.0 + sa * 0.57735,
                );
                let (mu, mv) = (1.92, 1.92 * hf / wf);
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    let (u, v) = ((x as f32 + 0.5) / wf * mu, (y as f32 + 0.5) / hf * mv);
                    let pa = leak_pattern(u, v, *seed, ev * PI);
                    let ta = 1.0 - ev;
                    let reveal = smoothstep(ta, ta + 0.3, pa[2]);
                    let pb = leak_pattern(mu - u, mv - v, seed + 42.0, re * PI);
                    let tb = 1.0 - re;
                    let erase = smoothstep(tb, tb + 0.3, pb[2]);
                    let al = reveal * (1.0 - erase);
                    let mut col = mix3([1.0, 0.85, 0.2], [1.0, 0.5, 0.05], pa[1]);
                    col = col.map(|v| v * (0.6 + 0.6 * pa[0]));
                    let hr = [
                        (a * col[0] + c * col[1] + b * col[2]).clamp(0.0, 1.0),
                        (b * col[0] + a * col[1] + c * col[2]).clamp(0.0, 1.0),
                        (c * col[0] + b * col[1] + a * col[2]).clamp(0.0, 1.0),
                    ];
                    let l = premul(hr, al);
                    [0, 1, 2, 3].map(|i| l[i] + s[i] * (1.0 - al))
                })
            }
            Effect::Burlap {
                amount,
                size,
                roughness,
                seed,
                color,
            } => {
                let sz = (size * hf).max(1e-3);
                let col = rgba(*color);
                let (amt, rough, seed) = (amount.clamp(0.0, 1.0), *roughness, *seed);
                let strand = |v: f32| 1.0 - smoothstep(0.2, 0.5, (fract(v) - 0.5).abs());
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    if s[3] <= 0.001 || amt <= 0.0 {
                        return s;
                    }
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let (wx, wy) = (fx / sz, fy / sz);
                    let rj = (hashi(wy.floor() as i32, 13, seed) - 0.5) * rough * 0.28;
                    let cj = (hashi(71, wx.floor() as i32, seed) - 0.5) * rough * 0.28;
                    let vert = strand(wx + rj);
                    let hor = strand(wy + cj);
                    let parity = (wx.floor() + wy.floor()).rem_euclid(2.0);
                    let over = mix(vert, hor, parity);
                    let ridge = (vert.max(hor) * 0.45).max(over * 0.75);
                    let gaps = (1.0 - vert) * (1.0 - hor);
                    let coarse = vnoise(fx / (sz * 7.0), fy / (sz * 7.0), seed ^ 0xc0a5);
                    let fine = vnoise(
                        fx / (sz * 0.55).max(0.5),
                        fy / (sz * 0.55).max(0.5),
                        seed ^ 0xf1e,
                    );
                    let dash_h = smoothstep(
                        0.64,
                        0.98,
                        hashi((wx * 2.0).floor() as i32, wy.floor() as i32, seed ^ 0x239),
                    ) * hor;
                    let dash_v = smoothstep(
                        0.64,
                        0.98,
                        hashi(wx.floor() as i32, (wy * 2.0).floor() as i32, seed ^ 0x914),
                    ) * vert;
                    let tx = (ridge - 0.42) * 0.22 - gaps * 0.18
                        + (coarse - 0.5) * 0.18 * rough
                        + (fine - 0.5) * 0.16 * rough
                        - (dash_h + dash_v) * 0.08 * rough;
                    let c = unpremul(s);
                    let dark = (-tx * 3.2 + gaps * 0.12 + (dash_h + dash_v) * 0.18).clamp(0.0, 1.0);
                    let shaded = mix3(c, [col[0], col[1], col[2]], dark * col[3]);
                    let textured = shaded.map(|v| (v * (1.0 + tx.max(0.0) * 0.45)).clamp(0.0, 1.0));
                    premul(mix3(c, textured, amt), s[3])
                })
            }
            Effect::Flannel {
                amount,
                size,
                softness,
                base,
                stripe,
            } => {
                let sz = (size * hf).max(1e-3);
                let (bc, sc) = (rgba(*base), rgba(*stripe));
                let edge = mix(0.002, 0.08, *softness);
                let st = |c: f32, width: f32, soft: f32| {
                    let d = (fract(c) - 0.5).abs();
                    1.0 - smoothstep(width * 0.5, width * 0.5 + soft, d)
                };
                let amt = amount.clamp(0.0, 1.0);
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    if s[3] <= 0.001 || amt <= 0.0 {
                        return s;
                    }
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let (px, py) = (fx / sz, fy / sz);
                    let vw = st(px, 0.34, edge);
                    let hw = st(py, 0.34, edge);
                    let vf = st(px * 2.0, 0.07, edge * 0.5);
                    let hfine = st(py * 2.0, 0.07, edge * 0.5);
                    let bands = vw.max(hw).max(vf.max(hfine) * 0.72);
                    let crossing = (vw * hw).max(vf * hfine);
                    let tx = 0.5 + 0.5 * (fx * PI).sin();
                    let ty = 0.5 + 0.5 * (fy * PI).sin();
                    let weave = (tx - 0.5) * (ty - 0.5) * 0.12;
                    let c = unpremul(s);
                    let l = luma(c);
                    let mut pc = mix3([bc[0], bc[1], bc[2]], [sc[0], sc[1], sc[2]], bands * sc[3]);
                    pc = mix3(
                        pc,
                        [sc[0] * 0.72, sc[1] * 0.72, sc[2] * 0.72],
                        crossing * sc[3],
                    );
                    let k = mix(0.58, 1.18, l) + weave;
                    pc = pc.map(|v| (v * k).clamp(0.0, 1.0));
                    premul(mix3(c, pc, amt), s[3])
                })
            }
            Effect::Paper(p) => paper(pm, p),
            Effect::EmbossRelief {
                amount,
                size,
                line_width,
                depth,
                angle,
                light_angle,
                offset,
            } => {
                let sz = (size * hf).max(1e-3);
                let lw = (line_width * hf).clamp(1e-3, sz * 0.85);
                let (sa, ca) = angle.to_radians().sin_cos();
                let off = offset * hf;
                let height = |fx: f32, fy: f32| -> f32 {
                    let (dx, dy) = (fx - wf * 0.5, fy - hf * 0.5);
                    let px = ca * dx + sa * dy + off;
                    let py = -sa * dx + ca * dy;
                    let (cx, cy) = ((px / sz).floor(), (py / sz).floor());
                    let (lx, ly) = ((fract(px / sz) - 0.5) * sz, (fract(py / sz) - 0.5) * sz);
                    let alt = (cx + cy).rem_euclid(2.0);
                    let da: f32 = if alt >= 0.5 { -PI / 4.0 } else { PI / 4.0 };
                    let (s2, c2) = da.sin_cos();
                    let (qx, qy) = (c2 * lx + s2 * ly, -s2 * lx + c2 * ly);
                    let r = lw * 0.5;
                    let (bx, by) = (qx.abs() - sz * 0.36 + r, qy.abs() - lw * 0.5 + r);
                    let sdf = (bx.max(0.0).powi(2) + by.max(0.0).powi(2)).sqrt()
                        + bx.max(by).min(0.0)
                        - r;
                    1.0 - smoothstep(-1.0, 1.0, sdf)
                };
                let (sl, cl) = light_angle.to_radians().sin_cos();
                let ln = {
                    let v = [cl, -sl, 0.7];
                    let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                    v.map(|c| c / m)
                };
                let (amt, dp) = (*amount, *depth);
                remap(pm, |x, y, src| {
                    let s = px_at(src, w, x, y);
                    if s[3] <= 0.001 || amt <= 0.0 || dp <= 0.0 {
                        return s;
                    }
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let hc = height(fx, fy);
                    let (hl, hr) = (height(fx - 1.0, fy), height(fx + 1.0, fy));
                    let (hu, hd) = (height(fx, fy - 1.0), height(fx, fy + 1.0));
                    let n = [(hl - hr) * dp * 3.8, (hu - hd) * dp * 3.8, 1.0];
                    let m = (n[0] * n[0] + n[1] * n[1] + 1.0).sqrt();
                    let lit = (n[0] * ln[0] + n[1] * ln[1] + n[2] * ln[2]) / m;
                    let bevel = (lit - 0.64) * 0.72 * dp;
                    let plateau = (hc - 0.5) * 0.08 * dp;
                    let edge = ((hr - hl).abs() + (hd - hu).abs()).min(1.0);
                    let c = unpremul(s);
                    let sh = c.map(|v| (v + bevel + plateau - edge * 0.035 * dp).clamp(0.0, 1.0));
                    premul(mix3(c, sh, amt), s[3])
                })
            }
            other => {
                crate::stylize::apply(other, pm);
            }
        }
    }

    /// Resolve an effect by its kebab-case name (every `@remotion/effects` name plus
    /// RunWee's own; `_`/spaces are accepted as `-`). `amount` sets the main intensity:
    /// a strength/opacity 0..1, a `progress` for reveals and transitions, or a length
    /// as a fraction of the frame height. Names RunWee had before keep their old
    /// meaning (`brightness`/`contrast`/`saturation` are multipliers, `hue` degrees,
    /// `blur`/`pixelate`/`halftone` fractions of the height). `lut` with no file
    /// gives a built-in teal/orange grade mixed by `amount`; load real `.cube`
    /// files with [`Lut::load`].
    pub fn from_name(name: &str, amount: f64) -> Option<Effect> {
        use crate::color::{rgb8, BLACK, WHITE};
        let a = amount as f32;
        let px = |v: f32| v / 1080.0;
        let blues = || vec![rgb8(0xdf, 0xf4, 0xff), rgb8(0x7c, 0xc6, 0xff)];
        let key = name.trim().to_ascii_lowercase().replace(['_', ' '], "-");
        Some(match key.as_str() {
            // ---- phase A5 stylize filters
            "dither" => Effect::Dither {
                levels: 2,
                cell: if a > 0.0 { a } else { 0.004 },
                palette: vec![],
            },
            "kuwahara" | "painterly" => Effect::Kuwahara(if a > 0.0 { a } else { 0.008 }),
            "pixel-sort" | "pixelsort" => Effect::PixelSort {
                threshold: if a > 0.0 { a } else { 0.45 },
                vertical: false,
            },
            "ascii" => Effect::Ascii {
                cell: if a > 0.0 { a } else { 0.02 },
                color: None,
            },
            "crt" => Effect::Crt {
                curvature: 0.08,
                scanlines: if a > 0.0 { a } else { 0.5 },
            },
            // ---- RunWee originals (and Remotion names with the same meaning)
            "brightness" => Effect::Brightness(a),
            "contrast" => Effect::Contrast(a),
            "saturation" | "saturate" => Effect::Saturation(a),
            "hue" | "hue-rotate" => Effect::HueRotate(a),
            "grayscale" | "greyscale" => Effect::Grayscale(a),
            "invert" => Effect::Invert(a),
            "sepia" => Effect::Sepia(a),
            "duotone-gradient" => Effect::Duotone(crate::color::BLUE_E, crate::color::YELLOW),
            "colorize" | "tint-luma" => Effect::Tint(crate::color::ORANGE, a),
            "exposure" => Effect::Exposure(a),
            "levels" => Effect::Levels {
                black: a,
                white: 1.0,
                gamma: 1.0,
            },
            "posterize" => Effect::Posterize(a.max(2.0) as u32),
            "vignette" => Effect::Vignette {
                strength: a,
                radius: 0.35,
            },
            "pixelate" => Effect::Pixelate(a),
            "grain" => Effect::Grain { amount: a, seed: 1 },
            "scanlines" => Effect::Scanlines {
                spacing: 0.006,
                strength: a,
            },
            "halftone" => Effect::Halftone { cell: a },
            "chromatic" | "chromatic-aberration" => Effect::ChromaticAberration(a),
            "bloom" => Effect::Bloom {
                threshold: 0.6,
                radius: 0.02,
                strength: a,
            },
            "blur" => Effect::Blur(a),
            "zoom-blur" => Effect::ZoomBlur {
                strength: a,
                cx: 0.5,
                cy: 0.5,
            },
            "mirror-half" => Effect::Mirror(a >= 0.5),
            "wave-rows" => Effect::Wave {
                amplitude: a,
                frequency: 3.0,
                phase: 0.0,
            },
            "fisheye" => Effect::Fisheye(a),
            "emboss-kernel" => Effect::Emboss,
            "sharpen" => Effect::Sharpen(a),
            "letterbox" => Effect::Letterbox(a),
            "fade" | "fade-to" | "fade-to-black" => Effect::FadeTo(BLACK, a),
            "tint" => Effect::FadeTo(rgb8(0xff, 0x80, 0x00), a),

            // ---- colour
            "duotone" | "duotone-threshold" => Effect::DuotoneThreshold {
                dark: BLACK,
                light: WHITE,
                threshold: a,
            },
            "vibrance" => Effect::Vibrance(a),
            "white-balance" => Effect::WhiteBalance {
                temperature: a,
                tint: 0.0,
            },
            "shadows-highlights" => Effect::ShadowsHighlights {
                shadows: a,
                highlights: -a,
            },
            "color-correction" | "colour-correction" => {
                Effect::ColorCorrection(ColorCorrectionParams {
                    contrast: 1.0 + 0.3 * a,
                    saturation: 1.0 + 0.2 * a,
                    vibrance: 0.5 * a,
                    ..Default::default()
                })
            }
            "thermal-vision" | "thermal" => Effect::ThermalVision {
                amount: a,
                palette: Vec::new(),
            },
            "tv-signal-off" | "color-bars" => Effect::TvSignalOff(a),
            "white-noise" | "static" => Effect::WhiteNoise { amount: a, seed: 0 },
            "noise" => Effect::Noise {
                amount: a,
                seed: 0,
                premultiply: false,
            },
            "color-key" | "chroma-key" | "colour-key" => Effect::ColorKey {
                key: rgb8(0, 255, 0),
                similarity: a,
                smoothness: 0.08,
                spill: 0.25,
            },
            "lut" => Effect::Lut(Arc::new(Lut::from_fn(17, move |c| {
                let l = luma(c);
                let g = [
                    c[0] + (l - 0.5) * 0.25,
                    c[1] + (l - 0.5) * 0.05,
                    c[2] - (l - 0.5) * 0.25,
                ];
                mix3(c, g.map(|v| v.clamp(0.0, 1.0)), a.clamp(0.0, 1.0))
            }))),
            "linear-gradient-tint" => Effect::LinearGradientTint {
                start: [0.0, 0.5],
                end: [1.0, 0.5],
                start_color: BLACK,
                end_color: WHITE,
                amount: a,
            },
            "vignette-mask" => Effect::VignetteMask {
                amount: a,
                radius: 0.65,
                feather: 0.35,
                roundness: 1.0,
                color: BLACK,
                fade_alpha: false,
                center: [0.5, 0.5],
            },

            // ---- blur / light
            "blur-axis" | "directional-blur" => Effect::BlurAxis {
                sigma: a,
                horizontal: true,
                vertical: false,
            },
            "glow" => Effect::Glow {
                radius: px(20.0) / 3.0,
                intensity: a,
                threshold: 0.0,
                color: WHITE,
            },
            "light-trail" => Effect::LightTrail {
                direction: 180.0,
                distance: px(80.0),
                intensity: a,
                decay: 0.9,
                threshold: 0.0,
                samples: 32,
                color: WHITE,
            },
            "region-blur" => Effect::RegionBlur {
                top_left: [0.3, 0.3],
                bottom_right: [0.7, 0.7],
                radius: a,
                feather: 0.0,
                roundness: 0.0,
            },
            "linear-progressive-blur" => Effect::LinearProgressiveBlur {
                start: [0.0, 0.5],
                end: [1.0, 0.5],
                start_blur: 0.0,
                end_blur: a,
            },
            "radial-progressive-blur" => Effect::RadialProgressiveBlur {
                center: [0.5, 0.5],
                width: 1.0,
                height: 1.0,
                rotation: 0.0,
                start: 0.0,
                start_blur: 0.0,
                end_blur: a,
            },
            "linear-progressive-pixelate" => Effect::LinearProgressivePixelate {
                start: [0.0, 0.5],
                end: [1.0, 0.5],
                start_block: 0.0,
                end_block: a,
            },
            "radial-progressive-pixelate" => Effect::RadialProgressivePixelate {
                center: [0.5, 0.5],
                width: 1.0,
                height: 1.0,
                rotation: 0.0,
                start: 0.0,
                start_block: 0.0,
                end_block: a,
            },

            // ---- geometry
            "barrel-distortion" | "barrel" => Effect::BarrelDistortion(a),
            "corner-pin" => Effect::CornerPin {
                top_left: [a, 0.0],
                top_right: [1.0 - a, 0.0],
                bottom_right: [1.0, 1.0],
                bottom_left: [0.0, 1.0],
            },
            "fisheye-lens" => Effect::FisheyeLens {
                fov: a,
                center: [0.5, 0.5],
                radius: 1.0,
                zoom: 1.0,
            },
            "mirror" => Effect::MirrorAt {
                horizontal: true,
                position: a,
                invert: false,
            },
            "wave" => Effect::WaveDisplace {
                amplitude: a,
                wavelength: px(240.0),
                phase: 0.0,
                horizontal: true,
            },
            "chromatic-split" => Effect::ChromaticSplit {
                amount: a,
                angle: 0.0,
            },
            "scale" => Effect::Scale { x: a, y: a },
            "skew" => Effect::Skew {
                x: a,
                y: 0.0,
                origin: [0.5, 0.5],
            },
            "translate" | "xy-translate" | "uv-translate" => Effect::Translate { x: a, y: 0.0 },
            "tile" => Effect::Tile {
                horizontal: true,
                vertical: true,
            },
            "pattern" => Effect::Pattern(PatternParams {
                scale: a,
                ..Default::default()
            }),
            "tear" => Effect::Tear {
                progress: a,
                angle: 0.0,
                rotation: 20.0,
                jaggedness: px(20.0),
            },
            "noise-displacement" => Effect::NoiseDisplacement(NoiseDisplacementParams {
                strength: a,
                ..Default::default()
            }),

            // ---- alpha / reveal
            "drop-shadow" => Effect::DropShadow {
                radius: px(12.0) / 3.0,
                offset_x: px(8.0),
                offset_y: px(8.0),
                opacity: a,
                color: BLACK,
            },
            "outline" => Effect::Outline {
                width: a,
                color: WHITE,
                opacity: 1.0,
                outline_only: false,
            },
            "roughen-edges" => Effect::RoughenEdges {
                amount: a,
                border: px(26.5),
                scale: 0.07,
                seed: 231.2,
            },
            "shrinkwrap" => Effect::Shrinkwrap {
                amount: a,
                displacement: px(5.0),
                highlight: 0.75,
                wrinkle_density: 0.42,
                edge_tension: 0.45,
                phase: 0.0,
                seed: 0.0,
            },
            "speckle" => Effect::Speckle {
                density: a,
                size: px(4.0),
                randomness: 1.0,
                seed: 0,
            },
            "dot-grid" => Effect::DotGrid {
                dot_size: a,
                grid_size: a * 1.25,
                invert: false,
            },
            "evolve" | "wipe" => Effect::Evolve {
                progress: a,
                from: RevealFrom::Left,
                feather: 0.1,
            },
            "pixel-dissolve" => Effect::PixelDissolve {
                progress: a,
                columns: 10,
                rows: 10,
                seed: 0,
                feather: 0.15,
            },
            "venetian-blinds" | "blinds" => Effect::VenetianBlinds {
                progress: a,
                vertical: true,
                slats: 12,
            },
            "shine" => Effect::Shine {
                progress: a,
                angle: 30.0,
                halo: px(200.0),
                core: px(65.0),
                halo_intensity: 0.3,
                core_intensity: 0.4,
            },

            // ---- generators
            "lines" | "stripes" => Effect::Lines {
                colors: blues(),
                vertical: false,
                thickness: px(40.0),
                gap: 0.0,
                angle: 0.0,
                offset: 0.0,
                opacity: a,
                mask: false,
            },
            "checkerboard" => Effect::Checkerboard {
                colors: blues(),
                cell: px(80.0),
                gap: 0.0,
                angle: 0.0,
                offset: [0.0, 0.0],
                opacity: a,
                mask: false,
            },
            "rings" => Effect::Rings {
                colors: blues(),
                center: [0.5, 0.5],
                thickness: px(40.0),
                gap: 0.0,
                offset: 0.0,
                opacity: a,
                mask: false,
            },
            "waves" => Effect::Waves {
                colors: blues(),
                vertical: false,
                thickness: px(40.0),
                gap: 0.0,
                angle: 0.0,
                offset: 0.0,
                amplitude: px(24.0),
                wavelength: px(160.0),
                phase: 0.0,
                opacity: a,
                mask: false,
            },
            "zigzag" => Effect::Zigzag {
                colors: blues(),
                vertical: false,
                thickness: px(40.0),
                gap: 0.0,
                angle: 0.0,
                offset: 0.0,
                amplitude: px(40.0),
                wavelength: px(160.0),
                opacity: a,
                mask: false,
            },
            "gridlines" | "grid" => Effect::Gridlines(GridlinesParams {
                opacity: a,
                ..Default::default()
            }),
            "starburst" => Effect::Starburst {
                rays: 16,
                colors: vec![rgb8(0xff, 0xc8, 0x3d), rgb8(0xff, 0x8a, 0x1f)],
                rotation: 0.0,
                smoothness: 0.0,
                origin: [0.5, 0.5],
                opacity: a,
            },
            "linear-gradient" | "gradient" => Effect::LinearGradient {
                start: [0.0, 0.5],
                end: [1.0, 0.5],
                start_color: BLACK,
                end_color: WHITE,
                opacity: a,
            },
            "halftone-grid" => Effect::HalftoneGrid {
                shape: HalftoneShape::Circle,
                size: a,
                spacing: a,
                rotation: 0.0,
                offset: [0.0, 0.0],
                color: Some(rgb8(255, 0, 0)),
                invert: false,
            },
            "halftone-linear-gradient" => Effect::HalftoneGradient {
                start_size: 0.0,
                end_size: a,
                start: [0.0, 0.5],
                end: [1.0, 0.5],
                grid: px(24.0),
                color: Some(BLACK),
                mask: false,
            },
            "contour-lines" | "contours" => Effect::ContourLines(ContourLinesParams {
                opacity: a,
                ..Default::default()
            }),
            "liquid-contours" => Effect::LiquidContours(LiquidContoursParams {
                opacity: a,
                ..Default::default()
            }),
            "light-leak" => Effect::LightLeak {
                seed: 0.0,
                hue_shift: 0.0,
                progress: a,
            },
            "burlap" => Effect::Burlap {
                amount: a,
                size: px(5.0),
                roughness: 0.7,
                seed: 0,
                color: BLACK,
            },
            "flannel" => Effect::Flannel {
                amount: a,
                size: px(96.0),
                softness: 0.18,
                base: rgb8(0xc9, 0x2f, 0x3d),
                stripe: rgb8(0x24, 0x10, 0x15),
            },
            "paper" => Effect::Paper(PaperParams {
                amount: a,
                ..Default::default()
            }),
            "emboss" | "emboss-relief" => Effect::EmbossRelief {
                amount: a,
                size: px(26.0),
                line_width: px(7.0),
                depth: 0.75,
                angle: 0.0,
                light_angle: 135.0,
                offset: 0.0,
            },
            _ => return None,
        })
    }
}

/// Inverse of the unit-square → quad homography, as a row-major 3×3 matrix
/// mapping output UV to source UV.
fn corner_pin_inverse(tl: [f32; 2], tr: [f32; 2], br: [f32; 2], bl: [f32; 2]) -> Option<[f32; 9]> {
    let (x0, y0, x1, y1, x2, y2, x3, y3) = (
        tl[0] as f64,
        tl[1] as f64,
        tr[0] as f64,
        tr[1] as f64,
        br[0] as f64,
        br[1] as f64,
        bl[0] as f64,
        bl[1] as f64,
    );
    let (sx, sy) = (x0 - x1 + x2 - x3, y0 - y1 + y2 - y3);
    let (g, hh) = if sx.abs() < 1e-12 && sy.abs() < 1e-12 {
        (0.0, 0.0)
    } else {
        let (dx1, dx2, dy1, dy2) = (x1 - x2, x3 - x2, y1 - y2, y3 - y2);
        let den = dx1 * dy2 - dx2 * dy1;
        if den.abs() < 1e-12 {
            return None;
        }
        ((sx * dy2 - dx2 * sy) / den, (dx1 * sy - sx * dy1) / den)
    };
    let (a, b, c) = (x1 - x0 + g * x1, x3 - x0 + hh * x3, x0);
    let (d, e, f) = (y1 - y0 + g * y1, y3 - y0 + hh * y3, y0);
    let m = [
        e - f * hh,
        c * hh - b,
        b * f - c * e,
        f * g - d,
        a - c * g,
        c * d - a * f,
        d * hh - e * g,
        b * g - a * hh,
        a * e - b * d,
    ];
    if m.iter().all(|v| v.abs() < 1e-15) {
        return None;
    }
    Some(m.map(|v| v as f32))
}

fn tile(pm: &mut Pixmap, horizontal: bool, vertical: bool) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let d = pm.data();
    let alpha = |x: usize, y: usize| d[(y * w + x) * 4 + 3];
    let row_max = |y: usize| (0..w).map(|x| alpha(x, y)).max().unwrap_or(0);
    let Some(mut top) = (0..h).find(|&y| row_max(y) > 0) else {
        pm.fill(tiny_skia::Color::TRANSPARENT);
        return;
    };
    let mut bottom = (top..h).rev().find(|&y| row_max(y) > 0).unwrap_or(top);
    let col_max = |x: usize, t: usize, b: usize| (t..=b).map(|y| alpha(x, y)).max().unwrap_or(0);
    let mut left = (0..w).find(|&x| col_max(x, top, bottom) > 0).unwrap_or(0);
    let mut right = (left..w)
        .rev()
        .find(|&x| col_max(x, top, bottom) > 0)
        .unwrap_or(left);
    // trim an antialiased fringe so copies meet without seams
    if vertical && top < bottom {
        if row_max(top) < row_max(top + 1) {
            top += 1;
        }
        if row_max(bottom) < row_max(bottom - 1) {
            bottom -= 1;
        }
    }
    if horizontal && left < right {
        if col_max(left, top, bottom) < col_max(left + 1, top, bottom) {
            left += 1;
        }
        if col_max(right, top, bottom) < col_max(right - 1, top, bottom) {
            right -= 1;
        }
    }
    let (tw, th) = ((right - left + 1) as i64, (bottom - top + 1) as i64);
    let (l, t) = (left as i64, top as i64);
    remap(pm, |x, y, src| {
        let fold = |p: i64, start: i64, size: i64, rep: bool| -> Option<usize> {
            let rel = p - start;
            if !rep {
                return (0..size).contains(&rel).then_some((start + rel) as usize);
            }
            let k = rel.div_euclid(size);
            let mut local = rel.rem_euclid(size);
            if k.rem_euclid(2) == 1 {
                local = size - 1 - local;
            }
            Some((start + local) as usize)
        };
        match (
            fold(x as i64, l, tw, horizontal),
            fold(y as i64, t, th, vertical),
        ) {
            (Some(sx), Some(sy)) => px_at(src, w, sx, sy),
            _ => [0.0; 4],
        }
    });
}

fn pattern(pm: &mut Pixmap, p: &PatternParams) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let scale = p.scale.max(1e-4);
    let crop_start = (p.crop[0] * wf, p.crop[1] * hf);
    let crop_end = (wf - p.crop[2] * wf, hf - p.crop[3] * hf);
    let crop_size = (
        (crop_end.0 - crop_start.0).max(1e-3),
        (crop_end.1 - crop_start.1).max(1e-3),
    );
    let tile = (crop_size.0 * scale, crop_size.1 * scale);
    let pitch = (
        (tile.0 + p.gap[0] * hf).max(1e-3),
        (tile.1 + p.gap[1] * hf).max(1e-3),
    );
    let origin = (
        (p.origin[0] + p.offset[0]) * wf,
        (p.origin[1] + p.offset[1]) * hf,
    );
    let phase = |i: f32, every: u32| {
        if every == 0 {
            i
        } else {
            i.rem_euclid(every as f32)
        }
    };
    let (ro, co) = (p.row_offset * hf, p.column_offset * hf);
    remap(pm, |x, y, src| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let mut cell = (
            ((fx - origin.0) / pitch.0).floor(),
            ((fy - origin.1) / pitch.1).floor(),
        );
        for _ in 0..4 {
            let st = (
                phase(cell.1, p.row_offset_every) * ro,
                phase(cell.0, p.column_offset_every) * co,
            );
            let next = (
                ((fx - origin.0 - st.0) / pitch.0).floor(),
                ((fy - origin.1 - st.1) / pitch.1).floor(),
            );
            if next == cell {
                break;
            }
            cell = next;
        }
        for j in -4..=4 {
            for i in -4..=4 {
                let c = (cell.0 + i as f32, cell.1 + j as f32);
                if !p.wrap && (c.0 < 0.0 || c.1 < 0.0) {
                    continue;
                }
                let st = (
                    phase(c.1, p.row_offset_every) * ro,
                    phase(c.0, p.column_offset_every) * co,
                );
                let lx = fx - origin.0 - st.0 - c.0 * pitch.0;
                let ly = fy - origin.1 - st.1 - c.1 * pitch.1;
                if lx < 0.0 || ly < 0.0 || lx >= tile.0 || ly >= tile.1 {
                    continue;
                }
                return tex_clip(
                    src,
                    w,
                    h,
                    crop_start.0 + lx / scale,
                    crop_start.1 + ly / scale,
                );
            }
        }
        [0.0; 4]
    });
}

/// Tear along a zig-zag seam (single rigid rotation per half; Remotion also bends
/// the pieces near the tip, which is not reproduced).
fn tear(pm: &mut Pixmap, progress: f32, angle: f32, rotation: f32, jag: f32) {
    let pr = progress.max(0.0);
    if pr <= 0.0 {
        return;
    }
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let (sa, ca) = angle.to_radians().sin_cos();
    let d = (sa, ca); // rip direction (0° = downwards)
    let n = (ca, -sa); // perpendicular
    let ext_d = d.0.abs() * wf * 0.5 + d.1.abs() * hf * 0.5;
    let ext_n = n.0.abs() * wf * 0.5 + n.1.abs() * hf * 0.5;
    let tip_u = -ext_d + pr.min(1.0) * 2.0 * ext_d;
    let theta = rotation.clamp(0.0, 90.0).to_radians() * pr.min(1.0);
    let (st, ct) = theta.sin_cos();
    let sep = (pr - 1.0).max(0.0) * ext_n;
    let amp = jag.min(0.9 * ext_n).max(0.0);
    let period = (amp * 2.0).max(6.0);
    let seam = |u: f32| {
        if amp <= 0.0 {
            0.0
        } else {
            amp * (1.0 - 4.0 * (fract(u / period) - 0.5).abs())
        }
    };
    let tip_v = seam(tip_u);
    let (cx, cy) = (wf * 0.5, hf * 0.5);
    let to_xy = |u: f32, v: f32| (cx + u * d.0 + v * n.0, cy + u * d.1 + v * n.1);
    remap(pm, |x, y, src| {
        let (rx, ry) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
        let (u, v) = (rx * d.0 + ry * d.1, rx * n.0 + ry * n.1);
        let inside = |sx: f32, sy: f32| sx >= 0.0 && sy >= 0.0 && sx <= wf && sy <= hf;
        // left piece (v < seam): forward = rotate +θ about the tip, shift −sep
        let (qu, qv) = (u - tip_u, v + sep - tip_v);
        let (su, sv) = (qu * ct + qv * st + tip_u, -qu * st + qv * ct + tip_v);
        if su < tip_u && sv < seam(su) {
            let (sx, sy) = to_xy(su, sv);
            if inside(sx, sy) {
                return tex(src, w, h, sx, sy);
            }
        }
        // right piece: rotate −θ, shift +sep
        let (qu, qv) = (u - tip_u, v - sep - tip_v);
        let (su, sv) = (qu * ct - qv * st + tip_u, qu * st + qv * ct + tip_v);
        if su < tip_u && sv >= seam(su) {
            let (sx, sy) = to_xy(su, sv);
            if inside(sx, sy) {
                return tex(src, w, h, sx, sy);
            }
        }
        if u >= tip_u {
            px_at(src, w, x, y)
        } else {
            [0.0; 4]
        }
    });
}

fn noise_displacement(pm: &mut Pixmap, p: &NoiseDisplacementParams) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let strength = p.strength * hf;
    if strength <= 0.0 {
        return;
    }
    const DISC: [(f32, f32, f32); 9] = [
        (0.0, 0.0, 1.0),
        (1.0, 0.0, 0.7),
        (-1.0, 0.0, 0.7),
        (0.0, 1.0, 0.7),
        (0.0, -1.0, 0.7),
        (0.707, 0.707, 0.5),
        (-0.707, 0.707, 0.5),
        (0.707, -0.707, 0.5),
        (-0.707, -0.707, 0.5),
    ];
    let (cx, cy) = (p.center[0] * wf, p.center[1] * hf);
    let radius = p.radius * wf.min(hf);
    let grain = (p.grain_size * hf).max(1e-3);
    let seed = fseed(p.seed);
    let (bs, bc) = p.bias_direction.to_radians().sin_cos();
    let passes = p.passes.clamp(1, 12);
    let blur_px = p.blur * hf;
    remap(pm, |x, y, src| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
        if dist >= radius {
            return px_at(src, w, x, y);
        }
        let mask = if p.feather <= 1e-4 {
            1.0
        } else {
            1.0 - smoothstep(radius * (1.0 - p.feather), radius, dist)
        };
        if mask <= 0.0 {
            return px_at(src, w, x, y);
        }
        let (nx, ny) = (fx / grain, fy / grain);
        let dx = (vnoise(nx + 17.13, ny + 71.79, seed) * 2.0 - 1.0 + bc * p.bias_amount)
            * strength
            * mask;
        let dy = (vnoise(nx + 83.41, ny + 29.67, seed) * 2.0 - 1.0 - bs * p.bias_amount)
            * strength
            * mask;
        let br = blur_px * mask;
        let taps = if br > 0.5 { 9 } else { 1 };
        let mut acc = [0.0f32; 4];
        let mut tw = 0.0;
        for s in 0..passes {
            let t = if passes == 1 {
                1.0
            } else {
                s as f32 / (passes - 1) as f32
            };
            let pw = 1.0 - t * 0.6;
            for &(ox, oy, dw) in DISC.iter().take(taps) {
                let wgt = pw * dw;
                let c = tex(src, w, h, fx + dx * t + ox * br, fy + dy * t + oy * br);
                for (a, v) in acc.iter_mut().zip(c) {
                    *a += v * wgt;
                }
                tw += wgt;
            }
        }
        acc.map(|v| v / tw)
    });
}

fn roughen_edges(pm: &mut Pixmap, amount: f32, border: f32, scale: f32, seed: f32) {
    if amount <= 0.0 || border <= 0.0 {
        return;
    }
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let reach = border.max(1.0);
    let greach = (border * 0.25).max(1.0);
    let sd = fseed(seed);
    let edge_noise = |u: f32, v: f32, salt: f32| {
        let o = seed_offset(seed, salt);
        let k = 7.0 / scale.max(1e-3);
        let px = (u - 0.5) * wf / hf.max(1.0) * k + 64.0 * o[0];
        let py = (v - 0.5) * k + 64.0 * o[1];
        fbm(px, py, 4, 2.03, 0.55, sd) / 0.9
    };
    remap(pm, |x, y, src| {
        let s = px_at(src, w, x, y);
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let al = |dx: f32, dy: f32| {
            tex(
                src,
                w,
                h,
                (fx + dx).clamp(0.0, wf),
                (fy + dy).clamp(0.0, hf),
            )[3]
        };
        let (mut mn, mut mx) = (s[3], s[3]);
        for (dx, dy) in [
            (reach, 0.0),
            (-reach, 0.0),
            (0.0, reach),
            (0.0, -reach),
            (reach, reach),
            (-reach, reach),
            (reach, -reach),
            (-reach, -reach),
        ] {
            let a = al(dx, dy);
            mn = mn.min(a);
            mx = mx.max(a);
        }
        let prox = smoothstep(0.02, 0.35, mx - mn);
        if prox <= 0.001 {
            return s;
        }
        let (u, v) = (fx / wf, fy / hf);
        let gx = al(greach, 0.0) - al(-greach, 0.0);
        let gy = al(0.0, greach) - al(0.0, -greach);
        let nv = (
            edge_noise(u + 0.17, v, 2.0) * 2.0 - 1.0,
            edge_noise(u, v + 0.31, 3.0) * 2.0 - 1.0,
        );
        let mut dir = (gx + 0.7 * nv.0, gy + 0.7 * nv.1);
        let mut len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
        if len <= 1e-4 {
            dir = nv;
            len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
            if len <= 1e-4 {
                dir = (1.0, 0.0);
                len = 1.0;
            }
        }
        let scalar = edge_noise(u, v, 4.0) * 2.0 - 1.0;
        let off = (dir.0 / len * scalar * border, dir.1 / len * scalar * border);
        let r = tex(
            src,
            w,
            h,
            (fx + off.0).clamp(0.0, wf),
            (fy + off.1).clamp(0.0, hf),
        );
        mix4(s, r, (prox * amount).clamp(0.0, 1.0))
    });
}

struct ShrinkwrapCfg {
    amount: f32,
    displacement: f32,
    highlight: f32,
    density: f32,
    edge_tension: f32,
    phase: f32,
    seed: f32,
}

fn shrinkwrap(pm: &mut Pixmap, c: &ShrinkwrapCfg) {
    if c.amount <= 0.0 {
        return;
    }
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let ms = wf.min(hf);
    let sd = fseed(c.seed);
    let ph = c.phase;
    let fb = |x: f32, y: f32| fbm(x, y, 4, 2.03, 0.5, sd);
    let warp = |x: f32, y: f32, scale: f32, amt: f32, so: f32| {
        (
            (fb(x / scale + ph * 0.21 + so, y / scale + ph * 0.13 + 1.7) - 0.5) * amt,
            (fb(
                x / scale - ph * 0.17 + so + 9.1,
                y / scale + ph * 0.29 + 6.4,
            ) - 0.5)
                * amt,
        )
    };
    let band = |coord: f32, spacing: f32, width: f32| {
        let cdist = (fract(coord / spacing) - 0.5).abs() * spacing;
        1.0 - smoothstep(width, width * 3.8, cdist)
    };
    let fold = |x: f32, y: f32, base: f32, spacing: f32, width: f32, so: f32| {
        let an = (fb(
            x / (spacing * 5.8) + ph * 0.08 + so,
            y / (spacing * 5.8) - ph * 0.11 + 3.1,
        ) - 0.5)
            * 1.35;
        let (wx, wy) = warp(
            x + so * 13.0,
            y + so * 7.0,
            spacing * 5.4,
            spacing * 0.95,
            so,
        );
        let (qx, qy) = rot(x + wx, y + wy, -(base + an).to_degrees());
        let phs = (fb(
            qx / (spacing * 3.4) - ph * 0.14 + so,
            qy / (spacing * 3.4) + ph * 0.19 + 8.6,
        ) - 0.5)
            * spacing
            * 1.3;
        let b = band(qy + phs, spacing, width);
        let brk = smoothstep(
            0.24,
            0.88,
            fb(
                qx / (spacing * 2.6) + ph * 0.12 + so,
                qy / (spacing * 7.5) + ph * 0.07 + so,
            ),
        );
        let soft = 0.65
            + fb(
                qx / (spacing * 8.2) - ph * 0.05 + so,
                qy / (spacing * 8.2) + ph * 0.16 + 2.3,
            ) * 0.35;
        b * brk * soft
    };
    let edge_mask = |fx: f32, fy: f32| {
        let ex = fx.min(wf - fx);
        let ey = fy.min(hf - fy);
        let side = 1.0 - smoothstep(0.0, ms * 0.16, ex.min(ey));
        let corner = 1.0 - smoothstep(0.0, ms * 0.23, (ex * ex + ey * ey).sqrt());
        (side * 0.55 + corner * 0.85).clamp(0.0, 1.0)
    };
    let spacing = mix(ms * 0.31, ms * 0.13, c.density);
    let width = mix(ms * 0.03, ms * 0.013, c.density);
    let height_at = |fx: f32, fy: f32| -> f32 {
        let (bx, by) = warp(fx, fy, ms * 0.42, ms * 0.06, c.seed + 1.0);
        let (dx, dy) = warp(fx, fy, ms * 0.17, ms * 0.018, c.seed + 6.0);
        let (x, y) = (fx + bx + dx, fy + by + dy);
        let mut hgt = (fb(
            x / (ms * 0.26) + ph * 0.11 + c.seed * 0.17,
            y / (ms * 0.26) - ph * 0.09 + c.seed * 0.17,
        ) - 0.5)
            * 0.08;
        hgt += fold(x, y, -0.74, spacing, width, 3.1) * 0.22;
        hgt += fold(
            x + bx * 0.7,
            y + by * 0.7,
            0.31,
            spacing * 1.9,
            width * 1.55,
            9.7,
        ) * 0.11;
        hgt += fold(
            x - dx * 0.6,
            y - dy * 0.6,
            1.04,
            spacing * 0.66,
            width * 0.62,
            15.3,
        ) * 0.045;
        hgt += (1.0
            - (fb(
                x / (ms * 0.11) - ph * 0.23 + c.seed * 0.3,
                y / (ms * 0.11) + ph * 0.27 + c.seed * 0.3,
            ) * 2.0
                - 1.0)
                .abs())
            * 0.018;
        let e = edge_mask(fx, fy) * c.edge_tension;
        let en = fb(
            x / (ms * 0.18) + ph * 0.18 + c.seed * 0.41,
            y / (ms * 0.18) + ph * 0.21 + 8.0,
        );
        hgt + e * (0.16 + en * 0.11)
    };
    // height field once per pixel; gradients from neighbours
    let mut hmap = vec![0.0f32; w * h];
    hmap.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, v) in row.iter_mut().enumerate() {
            *v = height_at(x as f32 + 0.5, y as f32 + 0.5);
        }
    });
    let hm = |x: isize, y: isize| {
        hmap[(y.clamp(0, h as isize - 1) as usize) * w + x.clamp(0, w as isize - 1) as usize]
    };
    let relief = mix(42.0, 115.0, c.density);
    let warp_k = relief * 0.32 * c.displacement * c.amount;
    let light = {
        let v = [-0.42f32, -0.58, 0.72];
        let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|q| q / m)
    };
    let half = {
        let v = [light[0], light[1], light[2] + 1.0];
        let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|q| q / m)
    };
    let dl = {
        let m = (0.45f32 * 0.45 + 0.72 * 0.72).sqrt();
        (-0.45 / m, -0.72 / m)
    };
    remap(pm, |x, y, src| {
        let (xi, yi) = (x as isize, y as isize);
        let g = (
            hm(xi + 1, yi) - hm(xi - 1, yi),
            hm(xi, yi + 1) - hm(xi, yi - 1),
        );
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let s = tex(
            src,
            w,
            h,
            (fx - g.0 * warp_k).clamp(0.0, wf),
            (fy - g.1 * warp_k).clamp(0.0, hf),
        );
        if s[3] <= 0.001 {
            return [0.0; 4];
        }
        let mut col = unpremul(s);
        let n = [-g.0 * relief, -g.1 * relief, 1.0];
        let nm = (n[0] * n[0] + n[1] * n[1] + 1.0).sqrt();
        let spec = ((n[0] * half[0] + n[1] * half[1] + n[2] * half[2]) / nm)
            .max(0.0)
            .powi(24);
        let dir = g.0 * dl.0 + g.1 * dl.1;
        let ch = smoothstep(0.02, 0.07, dir);
        let cs = smoothstep(0.02, 0.07, -dir);
        let e = edge_mask(fx, fy) * c.edge_tension;
        let glint = e * smoothstep(
            0.35,
            0.95,
            fb(
                fx / 90.0 + ph * 0.31 + c.seed,
                fy / 90.0 - ph * 0.24 + c.seed,
            ),
        );
        let hl =
            ((spec * 0.55 + ch * 0.4 + glint * 0.18) * c.highlight * c.amount).clamp(0.0, 1.15);
        let shadow = ((cs * 0.1 + e * 0.06) * c.amount).clamp(0.0, 0.25);
        col = col.map(|v| {
            let v = v * (1.0 - shadow);
            let v = v + (1.0 - v) * hl.min(1.0);
            (v + (hl - 1.0).max(0.0) * 0.35).clamp(0.0, 1.0)
        });
        premul(col, s[3])
    });
}

fn gridlines(pm: &mut Pixmap, p: &GridlinesParams) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let (sx, cx) = p.rotation_x.to_radians().sin_cos();
    let (sy, cy) = p.rotation_y.to_radians().sin_cos();
    let (sz, cz) = p.rotation.to_radians().sin_cos();
    // R = Rz · Ry · Rx (row-major)
    let rxm = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let rym = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rzm = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
    let mul = |a: [[f32; 3]; 3], b: [[f32; 3]; 3]| {
        let mut o = [[0.0f32; 3]; 3];
        for (i, row) in o.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (0..3).map(|k| a[i][k] * b[k][j]).sum();
            }
        }
        o
    };
    let r = mul(mul(rzm, rym), rxm);
    let mv = |m: &[[f32; 3]; 3], v: [f32; 3]| {
        [
            m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
            m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
            m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
        ]
    };
    let rt = [
        [r[0][0], r[1][0], r[2][0]],
        [r[0][1], r[1][1], r[2][1]],
        [r[0][2], r[1][2], r[2][2]],
    ];
    let persp = p.perspective * hf;
    let off = (p.offset[0] * hf, p.offset[1] * hf);
    let gc = |x: f32, y: f32| -> Option<(f32, f32)> {
        let c = [x - wf * 0.5, y - hf * 0.5, 0.0];
        if persp <= 0.0 {
            let v = mv(&r, c);
            return Some((v[0] + off.0, v[1] + off.1));
        }
        let cam = [0.0, 0.0, persp];
        let m = (c[0] * c[0] + c[1] * c[1] + persp * persp).sqrt();
        let ray = [c[0] / m, c[1] / m, -persp / m];
        let n = mv(&r, [0.0, 0.0, 1.0]);
        let den = ray[0] * n[0] + ray[1] * n[1] + ray[2] * n[2];
        if den.abs() < 1e-4 {
            return None;
        }
        let t = -(cam[2] * n[2]) / den;
        if t <= 0.0 {
            return None;
        }
        let wp = [
            cam[0] + ray[0] * t,
            cam[1] + ray[1] * t,
            cam[2] + ray[2] * t,
        ];
        let l = mv(&rt, wp);
        Some((l[0] + off.0, l[1] + off.1))
    };
    let size = (p.size * hf).max(1e-3);
    let lw = p.line_width * hf;
    let lc = rgba(p.color);
    let bg = rgba(p.background);
    generate(pm, p.opacity, p.mask, false, |x, y| {
        let cov = match gc(x, y) {
            None => 0.0,
            Some(g) => {
                let gx1 = gc(x + 1.0, y).unwrap_or(g);
                let gy1 = gc(x, y + 1.0).unwrap_or(g);
                let fw = (
                    ((gx1.0 - g.0).abs() + (gy1.0 - g.0).abs()).max(1e-3),
                    ((gx1.1 - g.1).abs() + (gy1.1 - g.1).abs()).max(1e-3),
                );
                let cell = (
                    (fract(g.0 / size + 0.5) - 0.5).abs() * size,
                    (fract(g.1 / size + 0.5) - 0.5).abs() * size,
                );
                let half = lw * 0.5;
                if half <= 0.001 {
                    0.0
                } else {
                    let lc_ = |c: f32, f: f32| {
                        let projected = size / f;
                        let fade = smoothstep((lw * 1.5).max(1.0), (lw * 4.0).max(2.0), projected);
                        (1.0 - smoothstep(half * f - f, half * f + f, c)) * fade
                    };
                    lc_(cell.0, fw.0).max(lc_(cell.1, fw.1))
                }
            }
        };
        // line over background, as straight alpha
        let la = lc[3] * cov;
        let a = la + bg[3] * (1.0 - la);
        if a <= 1e-6 {
            return [0.0; 4];
        }
        let c = [0, 1, 2].map(|i| (lc[i] * la + bg[i] * bg[3] * (1.0 - la)) / a);
        [c[0], c[1], c[2], a]
    });
}

fn contour_fbm(mut x: f32, mut y: f32, complexity: f32, smoothness: f32, seed: u32) -> f32 {
    let (mut v, mut amp, mut tot) = (0.0, 0.5, 0.0);
    let pers = mix(0.18, 0.58, complexity);
    for i in 0..5 {
        let oct = i as f32 / 4.0;
        let dw = mix(1.0, 1.0 - smoothstep(0.15, 1.0, oct), smoothness);
        v += amp * dw * vnoise5(x, y, seed);
        tot += amp * dw;
        let (nx, ny) = (0.8 * x - 0.6 * y, 0.6 * x + 0.8 * y);
        x = nx * 2.04 + 13.17;
        y = ny * 2.04 + 7.31;
        amp *= pers;
    }
    v / tot
}

fn contour_lines(pm: &mut Pixmap, p: &ContourLinesParams) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let scale = (p.scale * hf).max(1e-3);
    let cx = p.complexity.clamp(0.0, 1.0);
    let sm = p.smoothness.clamp(0.0, 1.0);
    let seed = fseed(p.seed);
    let levels = (wf.max(hf) / ((p.spacing * hf).max(1e-3) * 2.0)).max(1.0);
    let field = |x: f32, y: f32| {
        let px = (x + p.offset[0] * hf) / scale + p.seed * 0.137;
        let py = (y + p.offset[1] * hf) / scale + p.seed * 0.193;
        let wx = contour_fbm(px + 5.2, py + 1.3, cx, sm, seed) - 0.5;
        let wy = contour_fbm(px + 1.7, py + 9.2, cx, sm, seed) - 0.5;
        let k = cx * mix(2.8, 1.2, sm);
        contour_fbm(px + wx * k, py + wy * k, cx, sm, seed) * levels
    };
    let lw = p.line_width * hf;
    let col = rgba(p.color);
    generate(pm, p.opacity, p.mask, false, |x, y| {
        let c = field(x, y);
        let deriv = ((field(x + 1.0, y) - c).abs() + (field(x, y + 1.0) - c).abs()).max(1e-4);
        let fr = fract(c);
        let dist = fr.min(1.0 - fr);
        let half = lw.max(1e-3) * 0.5 * deriv;
        let cov = 1.0 - smoothstep(half, half + deriv, dist);
        [col[0], col[1], col[2], col[3] * cov]
    });
}

fn liquid_contours(pm: &mut Pixmap, p: &LiquidContoursParams) {
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let scale = (p.scale * hf).max(1e-3);
    let cx = p.complexity.clamp(0.0, 1.0);
    let seed = fseed(p.seed);
    let levels = (wf.max(hf) / (p.spacing * hf).max(1e-3)).max(1.0);
    // liquid fbm: detail weight depends on complexity only
    let lfbm = |mut x: f32, mut y: f32| {
        let (mut v, mut amp, mut tot) = (0.0, 0.5, 0.0);
        let pers = mix(0.2, 0.58, cx);
        let detail = mix(0.45, 1.0, cx);
        for i in 0..5 {
            let oct = i as f32 / 4.0;
            let ow = mix(1.0, 1.0 - smoothstep(0.4, 1.0, oct), 1.0 - detail);
            v += amp * ow * vnoise5(x, y, seed);
            tot += amp * ow;
            let (nx, ny) = (0.8 * x - 0.6 * y, 0.6 * x + 0.8 * y);
            x = nx * 2.03 + 13.17;
            y = ny * 2.03 + 7.31;
            amp *= pers;
        }
        v / tot
    };
    let wave = |x: f32, y: f32| {
        let px = (x + p.offset[0] * hf) / scale + p.seed * 0.137;
        let py = (y + p.offset[1] * hf) / scale + p.seed * 0.193;
        let wx = lfbm(px + 5.2, py + 1.3) - 0.5;
        let wy = lfbm(px + 1.7, py + 9.2) - 0.5;
        let (rx, ry) = (
            std::f32::consts::FRAC_1_SQRT_2 * px - std::f32::consts::FRAC_1_SQRT_2 * py,
            std::f32::consts::FRAC_1_SQRT_2 * px + std::f32::consts::FRAC_1_SQRT_2 * py,
        );
        let rounded = mix(
            vnoise5(px * 0.9, py * 0.9, seed),
            vnoise5(rx * 0.78 + 8.3, ry * 0.78 + 2.7, seed),
            0.45,
        );
        let k = mix(0.35, 1.8, cx);
        let detailed = lfbm(px + wx * k, py + wy * k);
        let field = mix(rounded, detailed, smoothstep(0.0, 1.0, cx));
        ((field * levels + p.phase) * TAU).sin()
    };
    let (c1, c2) = (rgba(p.first), rgba(p.second));
    generate(pm, p.opacity, false, false, |x, y| {
        let v = wave(x, y);
        let fw = (wave(x + 1.0, y) - v).abs() + (wave(x, y + 1.0) - v).abs();
        let e = (fw * mix(0.15, 1.4, p.smoothness)).max(1e-4);
        let sel = smoothstep(-e, e, v);
        // premultiplied mix, back to straight
        let a = mix(c1[3], c2[3], sel);
        if a <= 1e-6 {
            return [0.0; 4];
        }
        let c = [0, 1, 2].map(|i| mix(c1[i] * c1[3], c2[i] * c2[3], sel) / a);
        [c[0], c[1], c[2], a]
    });
}

fn paper(pm: &mut Pixmap, p: &PaperParams) {
    if p.amount <= 0.0 {
        return;
    }
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let (wf, hf) = (w as f32, h as f32);
    let sd = fseed(p.seed);
    let hr = |x: f32, y: f32| hashi(x.floor() as i32, y.floor() as i32, sd ^ 0x1001);
    let hg = |x: f32, y: f32| hashi(x.floor() as i32, y.floor() as i32, sd ^ 0x2002);
    let hb = |x: f32, y: f32| hashi(x.floor() as i32, y.floor() as i32, sd ^ 0x3003);
    let vn = |x: f32, y: f32, hfn: &dyn Fn(f32, f32) -> f32| {
        let (ix, iy) = (x.floor(), y.floor());
        let (fx, fy) = (x - ix, y - iy);
        let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
        mix(
            mix(hfn(ix, iy), hfn(ix + 1.0, iy), ux),
            mix(hfn(ix, iy + 1.0), hfn(ix + 1.0, iy + 1.0), ux),
            uy,
        )
    };
    let fbm3 = |mut x: f32, mut y: f32| {
        let (mut t, mut a) = (0.0, 0.4);
        for _ in 0..3 {
            t += vn(x, y, &hr) * a;
            x *= 1.99;
            y *= 1.99;
            a *= 0.65;
        }
        t
    };
    let rough_noise = |x: f32, y: f32| {
        let (mut x, mut y) = (x * 0.1, y * 0.1);
        let mut o = 0.0;
        for _ in 0..3 {
            let (fx, fy) = (fract(x), fract(y));
            let (x0, y0, x1, y1) = (x.floor(), y.floor(), x.ceil(), y.ceil());
            o += mix(
                mix(hg(x0, y0), hg(x0, y1), fy),
                mix(hg(x1, y0), hg(x1, y1), fy),
                fx,
            );
            o += 0.2 / (2.0 * (0.2 * x + 0.5 * y).sin().abs()).exp();
            x *= 2.1;
            y *= 2.1;
        }
        o / 3.0
    };
    let fiber_fbm = |x: f32, y: f32, so: [f32; 2]| {
        let (mut x, mut y) = (x, y);
        let (mut t, mut a) = (0.0, 1.0);
        let (s7, c7) = 0.7f32.sin_cos();
        for _ in 0..4 {
            let (nx, ny) = (c7 * x - s7 * y, s7 * x + c7 * y);
            x = nx;
            y = ny;
            t += vn(x + so[0], y + so[1], &hb) * a;
            x *= 2.0;
            y *= 2.0;
            a *= 0.6;
        }
        t
    };
    let fiber_noise = |x: f32, y: f32, so: [f32; 2]| {
        let e = 0.001;
        let dx = fiber_fbm(x + e, y, so) - fiber_fbm(x - e, y, so);
        let dy = fiber_fbm(x, y + e, so) - fiber_fbm(x, y - e, so);
        (dx * dx + dy * dy).sqrt() / (2.0 * e)
    };
    let crumpled = |tx: f32, ty: f32, pw: f32| {
        let (px, py) = (tx.floor(), ty.floor());
        let (mut ws, mut cl) = (0.0, 0.0);
        for j in -1..=1 {
            for i in -1..=1 {
                let (qx, qy) = (px + i as f32, py + j as f32);
                let (q2x, q2y) = (qx.rem_euclid(8.0), qy.rem_euclid(8.0));
                let (cx, cy) = (qx + hg(q2x, q2y), qy + hb(q2x, q2y));
                let (rx, ry) = (cx - tx, cy - ty);
                let wgt = smoothstep(0.0, 1.0, 1.0 - rx.abs()).powf(pw)
                    * smoothstep(0.0, 1.0, 1.0 - ry.abs()).powf(pw);
                cl += (0.5 + 0.5 * ((q2x + q2y * 5.0) * 8.0).sin()) * wgt;
                ws += wgt;
            }
        }
        (if ws != 0.0 { cl / ws } else { 0.0 }).sqrt() * 2.0
    };
    let crumple_shape =
        |x: f32, y: f32| crumpled(x * 0.25, y * 0.25, 16.0) * crumpled(x * 0.5, y * 0.5, 2.0);
    let fold_count = p.fold_count.min(15);
    let folds = |x: f32, y: f32| {
        let (mut best, mut pp) = (9.0f32, (0.0f32, 0.0f32));
        for i in 0..fold_count {
            let fi = i as f32;
            let (r0, r1) = (hg(fi, fi * p.seed), hb(fi, fi * p.seed));
            let an = r0 * TAU;
            let (fx, fy) = (an.cos() * r1, an.sin() * r1);
            let d = ((x - fx).powi(2) + (y - fy).powi(2)).sqrt();
            if d < best {
                best = d;
                pp = (x - fx, y - fy);
            }
        }
        let k = 1.0 - best.max(0.0).powf(0.25);
        (pp.0 * k, pp.1 * k)
    };
    let drops = |x: f32, y: f32| {
        let (ix, iy) = (x.floor(), y.floor());
        let (fx, fy) = (x - ix, y - iy);
        let mut md = 1.0f32;
        for j in -1..=1 {
            for i in -1..=1 {
                let (nx, ny) = (i as f32, j as f32);
                let ox = 0.5 + 0.5 * (10.0 * p.seed + TAU * hg(ix + nx, iy + ny)).sin();
                let oy = 0.5 + 0.5 * (10.0 * p.seed + TAU * hb(ix + nx, iy + ny)).sin();
                let d = ((nx + ox - fx).powi(2) + (ny + oy - fy).powi(2)).sqrt();
                md = md.min(md * d);
            }
        }
        1.0 - smoothstep(0.05, 0.09, md.sqrt())
    };
    let so1 = seed_offset(p.seed, 1.0);
    let so2 = seed_offset(p.seed, 2.0).map(|v| v * 64.0);
    let front = mix3([1.0; 3], rgb(p.front), p.front.a);
    let back = mix3([1.0; 3], rgb(p.back), p.back.a);
    let lp = {
        let m = 6f32.sqrt();
        [1.0 / m, 2.0 / m, 1.0 / m]
    };
    let contrast = p.contrast.max(0.0);
    let aspect = wf / hf;
    remap(pm, |x, y, src| {
        let s = px_at(src, w, x, y);
        if s[3] <= 0.001 {
            return s;
        }
        let src_rgb = unpremul(s);
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let (u, v) = (fx / wf, 1.0 - fy / hf);
        let sc = p.scale.max(1e-3);
        let (pu, pv) = ((u - 0.5) / sc * 5.0 * aspect, (v - 0.5) / sc * 5.0);
        let (rux, ruy) = (
            1.5 * (fx - 0.5 * wf) + 128.0 * so1[0],
            1.5 * (hf - fy - 0.5 * hf) + 128.0 * so1[1],
        );
        let mut roughness = rough_noise(rux + 1.0, ruy) - rough_noise(rux - 1.0, ruy);
        let cs = p.crumple_size.max(1e-3);
        let (cu, cv) = (
            fract(pu * 0.02 / cs - p.seed) * 32.0,
            fract(pv * 0.02 / cs - p.seed) * 32.0,
        );
        let mut crumples = p.crumples * (crumple_shape(cu + 0.05, cv) - crumple_shape(cu, cv));
        let fs = 2.0 / p.fiber_size.max(1e-3);
        let mut fiber = 0.5 * p.fiber * (fiber_noise(pu * fs, pv * fs, so2) - 1.0);
        let (s4, c4) = (4.0 * p.seed).sin_cos();
        let (fu, fv) = (pu * 0.12, pv * 0.12);
        let (fu, fv) = (c4 * fu - s4 * fv, s4 * fu + c4 * fv);
        let mut w1 = folds(fu, fv);
        let (a2, b2) = (fu + 0.007 * p.seed.cos(), fv + 0.007 * p.seed.cos());
        let th = 0.01 * p.seed.sin();
        let (st, ct) = th.sin_cos();
        let mut w2 = folds(ct * a2 - st * b2, st * a2 + ct * b2);
        let mut drop = p.drops * drops(pu * 2.0, pv * 2.0);
        let mut fade = p.fade * fbm3(0.17 * pu + 10.0 * p.seed, 0.17 * pv + 10.0 * p.seed);
        fade = (8.0 * fade * fade * fade).clamp(0.0, 1.0);
        w1 = (w1.0 * (1.0 - fade), w1.1 * (1.0 - fade));
        w2 = (w2.0 * (1.0 - fade), w2.1 * (1.0 - fade));
        crumples *= 1.0 - fade;
        drop *= 1.0 - fade;
        fiber *= mix(1.0, 0.5, fade);
        roughness *= mix(1.0, 0.5, fade);
        let fk = p.folds * (5.0 * contrast).min(1.0) * 4.0;
        let mut n = ((w1.0 + w2.0).max(0.0) * fk, (w1.1 + w2.1).max(0.0) * fk);
        let mut ni = (p.folds * 2.0 * w1.0, p.folds * 2.0 * w1.1);
        n.0 += crumples + 3.0 * drop + p.roughness * 1.5 * roughness + fiber;
        n.1 += crumples + 3.0 * drop + p.roughness * 1.5 * roughness + fiber;
        ni.0 += 1.5 * crumples + 0.2 * drop + p.roughness * 0.75 * roughness + 0.2 * fiber;
        ni.1 += 1.5 * crumples + 0.2 * drop + p.roughness * 0.75 * roughness + 0.2 * fiber;
        let nz = 9.5 - 9.0 * contrast.powf(0.1);
        let nm = (n.0 * n.0 + n.1 * n.1 + nz * nz).sqrt().max(1e-6);
        let res = (n.0 * lp[0] + n.1 * lp[1] + nz * lp[2]) / nm;
        let (iu, iv) = (u + 0.02 * ni.0, v + 0.02 * ni.1);
        let frame = if (0.0..=1.0).contains(&iu) && (0.0..=1.0).contains(&iv) {
            1.0
        } else {
            0.0
        };
        let ds = tex(
            src,
            w,
            h,
            iu.clamp(0.0, 1.0) * wf,
            (1.0 - iv.clamp(0.0, 1.0)) * hf,
        );
        let mut drgb = if ds[3] > 0.001 { unpremul(ds) } else { src_rgb };
        let lift = 0.6 * contrast.powf(0.4) * (res - 0.7);
        drgb = drgb.map(|c| c + lift);
        let tone = mix3(back, front, res.clamp(0.0, 1.0));
        let mut t = drgb.map(|c| c * (0.72 + 0.42 * res));
        t = [0, 1, 2].map(|i| mix(t[i], t[i] * tone[i], 0.65) - 0.007 * drop);
        t = mix3(src_rgb, t, frame);
        premul(
            mix3(
                src_rgb,
                t.map(|c| c.clamp(0.0, 1.0)),
                p.amount.clamp(0.0, 1.0),
            ),
            s[3],
        )
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

    const REMOTION_NAMES: [&str; 74] = [
        "barrel-distortion",
        "blur",
        "burlap",
        "flannel",
        "checkerboard",
        "chromatic-aberration",
        "color-key",
        "color-correction",
        "brightness",
        "contrast",
        "contour-lines",
        "drop-shadow",
        "duotone",
        "emboss",
        "evolve",
        "exposure",
        "fisheye",
        "corner-pin",
        "glow",
        "gridlines",
        "halftone",
        "halftone-linear-gradient",
        "pixel-dissolve",
        "pixelate",
        "grayscale",
        "hue",
        "invert",
        "lines",
        "linear-gradient",
        "linear-gradient-tint",
        "liquid-contours",
        "linear-progressive-blur",
        "linear-progressive-pixelate",
        "levels",
        "lut",
        "light-leak",
        "light-trail",
        "dot-grid",
        "mirror",
        "noise",
        "noise-displacement",
        "outline",
        "paper",
        "roughen-edges",
        "pattern",
        "radial-progressive-blur",
        "radial-progressive-pixelate",
        "region-blur",
        "rings",
        "saturation",
        "scanlines",
        "scale",
        "shadows-highlights",
        "shine",
        "shrinkwrap",
        "skew",
        "speckle",
        "starburst",
        "tear",
        "thermal-vision",
        "tile",
        "tint",
        "translate",
        "tv-signal-off",
        "venetian-blinds",
        "vibrance",
        "vignette",
        "wave",
        "waves",
        "white-balance",
        "white-noise",
        "zoom-blur",
        "zigzag",
        "light_trail",
    ];

    /// A colourful 64×48 frame with a transparent border (exercises alpha paths).
    fn test_frame() -> Pixmap {
        let (w, h) = (64usize, 48usize);
        let mut pm = Pixmap::new(w as u32, h as u32).unwrap();
        for (i, p) in pm.data_mut().as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let (x, y) = (i % w, i / w);
            let a: u32 = if x < 4 || y < 4 || x >= w - 4 { 0 } else { 255 };
            let r = (x * 255 / w) as u32;
            let g = (y * 255 / h) as u32;
            let b = ((x + y) * 4 % 256) as u32;
            *p = [r * a / 255, g * a / 255, b * a / 255, a].map(|v| v as u8);
        }
        pm
    }

    #[test]
    fn from_name_covers_remotion() {
        for n in REMOTION_NAMES {
            assert!(Effect::from_name(n, 0.5).is_some(), "{n}");
        }
        assert!(Effect::from_name("definitely-not-an-effect", 1.0).is_none());
        assert_eq!(
            Effect::from_name("Chromatic_Aberration", 0.01),
            Some(Effect::ChromaticAberration(0.01))
        );
    }

    #[test]
    fn every_named_effect_runs_and_stays_premultiplied() {
        let extra = [
            "posterize",
            "sepia",
            "bloom",
            "sharpen",
            "letterbox",
            "grain",
            "fade",
            "halftone-grid",
            "chromatic-split",
            "fisheye-lens",
            "vignette-mask",
            "emboss-kernel",
            "blur-axis",
            "duotone-gradient",
            "colorize",
            "mirror-half",
        ];
        let mut bad = Vec::new();
        for n in REMOTION_NAMES.iter().chain(extra.iter()) {
            for amount in [0.0, 0.05, 0.5, 1.0] {
                let e = Effect::from_name(n, amount).unwrap();
                let mut pm = test_frame();
                e.apply(&mut pm);
                for p in pm.data().as_chunks::<4>().0 {
                    if !(p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]) {
                        bad.push(format!("{n} {amount}"));
                        break;
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{bad:?}");
    }

    #[test]
    fn invert_twice_is_identity() {
        let orig = test_frame();
        let mut pm = orig.clone();
        Effect::Invert(1.0).apply(&mut pm);
        Effect::Invert(1.0).apply(&mut pm);
        for (a, b) in pm.data().iter().zip(orig.data()) {
            assert!((*a as i32 - *b as i32).abs() <= 1);
        }
    }

    #[test]
    fn color_key_removes_key() {
        let mut pm = Pixmap::new(8, 8).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(0, 255, 0, 255));
        Effect::from_name("color-key", 0.2).unwrap().apply(&mut pm);
        assert!(pm.data().iter().all(|v| *v == 0));
        let mut pm = Pixmap::new(8, 8).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(200, 40, 90, 255));
        Effect::from_name("color-key", 0.2).unwrap().apply(&mut pm);
        assert_eq!(pm.pixel(3, 3).unwrap().alpha(), 255);
    }

    #[test]
    fn identity_cube_lut_is_identity() {
        let mut cube = String::from("TITLE \"id\"\n# comment\nLUT_3D_SIZE 2\n");
        for b in 0..2 {
            for g in 0..2 {
                for r in 0..2 {
                    cube.push_str(&format!("{r}.0 {g}.0 {b}.0\n"));
                }
            }
        }
        let lut = Lut::from_cube_str(&cube).unwrap();
        assert_eq!(lut.size, 2);
        let l = Arc::new(lut);
        assert_eq!(Effect::Lut(l.clone()), Effect::Lut(l.clone()));
        assert_ne!(
            Effect::Lut(l.clone()),
            Effect::Lut(Arc::new(Lut::identity(2)))
        );
        let orig = test_frame();
        let mut pm = orig.clone();
        Effect::Lut(l).apply(&mut pm);
        for (a, b) in pm.data().iter().zip(orig.data()) {
            assert!((*a as i32 - *b as i32).abs() <= 1);
        }
        assert!(Lut::from_cube_str("LUT_3D_SIZE 2\n0 0 0\n").is_err());
        let one_d = Lut::from_cube_str("LUT_1D_SIZE 2\n1 1 1\n0 0 0\n").unwrap();
        assert_eq!(one_d.lookup([0.0, 0.25, 1.0]), [1.0, 0.75, 0.0]);
    }

    #[test]
    fn identity_corner_pin_is_identity() {
        let orig = test_frame();
        let mut pm = orig.clone();
        Effect::CornerPin {
            top_left: [0.0, 0.0],
            top_right: [1.0, 0.0],
            bottom_right: [1.0, 1.0],
            bottom_left: [0.0, 1.0],
        }
        .apply(&mut pm);
        for (a, b) in pm.data().iter().zip(orig.data()) {
            assert!((*a as i32 - *b as i32).abs() <= 1);
        }
        // a shrunken pin leaves the corners transparent
        let mut pm = orig.clone();
        Effect::from_name("corner-pin", 0.3).unwrap().apply(&mut pm);
        assert_eq!(pm.pixel(5, 1).unwrap().alpha(), 0);
    }

    #[test]
    fn reveals_hide_and_show() {
        for n in ["evolve", "venetian-blinds"] {
            let mut pm = test_frame();
            Effect::from_name(n, 0.0).unwrap().apply(&mut pm);
            assert!(pm.data().iter().all(|v| *v == 0), "{n}");
            let mut pm = test_frame();
            Effect::from_name(n, 1.0).unwrap().apply(&mut pm);
            assert_eq!(pm.data(), test_frame().data(), "{n}");
        }
        let mut pm = test_frame();
        Effect::from_name("pixel-dissolve", 1.0)
            .unwrap()
            .apply(&mut pm);
        assert!(pm.data().iter().all(|v| *v == 0));
    }

    #[test]
    fn outline_and_shadow_grow_alpha() {
        for e in [
            Effect::Outline {
                width: 0.05,
                color: WHITE,
                opacity: 1.0,
                outline_only: false,
            },
            Effect::DropShadow {
                radius: 0.01,
                offset_x: -0.05,
                offset_y: 0.0,
                opacity: 1.0,
                color: BLACK,
            },
        ] {
            let mut pm = test_frame();
            e.apply(&mut pm);
            assert!(pm.pixel(2, 20).unwrap().alpha() > 0, "{e:?}");
        }
    }

    #[test]
    fn generators_fill_a_clear_frame() {
        for n in [
            "lines",
            "checkerboard",
            "rings",
            "starburst",
            "linear-gradient",
            "liquid-contours",
        ] {
            let mut pm = Pixmap::new(32, 24).unwrap();
            Effect::from_name(n, 1.0).unwrap().apply(&mut pm);
            assert!(
                pm.data().as_chunks::<4>().0.iter().all(|p| p[3] == 255),
                "{n}"
            );
        }
    }
}
