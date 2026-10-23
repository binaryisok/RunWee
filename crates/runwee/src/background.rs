//! Scene backgrounds and themes.
//!
//! A [`Background`] is drawn in screen space behind everything else, every frame, as a
//! pure function of time (so animated backgrounds still render in parallel). It can be a
//! solid colour, a gradient, an image or video, a pattern (grid, graph paper, dots,
//! notebook, …), an animated field (aurora, starfield, bokeh, flowing noise) or a stack
//! of layers. [`Background::preset`] names ready-made looks; a [`Theme`] pairs a
//! background with the ink mode that keeps every other colour readable on it.

use crate::assets::VideoSource;
use crate::color::*;
use crate::math::*;
use std::sync::Arc;
use tiny_skia as sk;

/// How an image or video fills the frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Fit {
    /// Scale to cover the whole frame (cropping), like CSS `object-fit: cover`.
    #[default]
    Cover,
    /// Scale to fit inside the frame (letterboxing on the base colour).
    Contain,
    /// Stretch to the frame size.
    Stretch,
    /// Repeat at native pixel size.
    Tile,
}

/// Repeating patterns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pattern {
    /// Square grid lines.
    Grid,
    /// Fine grid with a heavier line every 5 cells.
    GraphPaper,
    /// Dot grid.
    Dots,
    /// Horizontal ruled lines.
    Ruled,
    /// Ruled lines with a red margin line.
    Notebook,
    /// Checkerboard of `base` and `ink`.
    Checker,
    /// Diagonal stripes.
    Stripes,
    /// Small plus signs on a grid.
    Crosses,
    /// Isometric (triangular) grid.
    Isometric,
}

/// A scene background. Cheap to clone (images and videos are shared).
#[derive(Clone)]
pub enum Background {
    Solid(Color),
    /// Linear gradient at `angle` degrees (0 = left→right, 90 = bottom→top).
    Linear {
        angle: f64,
        stops: Vec<(f32, Color)>,
    },
    /// Radial gradient centred at `center` (0..1 frame coordinates, y down) with
    /// `radius` as a fraction of the frame diagonal.
    Radial {
        center: (f64, f64),
        radius: f64,
        stops: Vec<(f32, Color)>,
    },
    /// Smooth four-corner gradient: top-left, top-right, bottom-left, bottom-right.
    Corners([Color; 4]),
    /// A still image. `dim` darkens it (0..1) so content on top stays readable.
    Image {
        pixmap: Arc<sk::Pixmap>,
        fit: Fit,
        dim: f32,
    },
    /// A video, looped. `dim` as for images.
    Video {
        source: Arc<VideoSource>,
        fit: Fit,
        dim: f32,
    },
    /// A repeating pattern. `spacing` is in frame heights (0.05 = 20 cells tall),
    /// `width` is the line width in pixels at 1080p, `drift` scrolls it (frame heights/s).
    Pattern {
        kind: Pattern,
        base: Color,
        ink: Color,
        spacing: f64,
        width: f32,
        drift: Vec2,
    },
    /// Soft flowing aurora bands over `base`.
    Aurora {
        base: Color,
        colors: Vec<Color>,
        speed: f64,
    },
    /// Twinkling stars drifting over `base`; `density` is stars per frame-height².
    Starfield {
        base: Color,
        density: f64,
        speed: f64,
    },
    /// Large soft out-of-focus lights drifting over `base`.
    Bokeh {
        base: Color,
        colors: Vec<Color>,
        count: usize,
        speed: f64,
    },
    /// Animated noise gradient blending `colors` (like a lava lamp / mesh gradient).
    Flow {
        colors: Vec<Color>,
        scale: f64,
        speed: f64,
    },
    /// Several backgrounds drawn in order; each with an opacity.
    Layers(Vec<(Background, f32)>),
    /// Anything else: draw into the pixmap given the time in seconds.
    Custom(Arc<dyn Fn(f64, &mut sk::Pixmap) + Send + Sync>),
}

impl std::fmt::Debug for Background {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Background::Solid(_) => "Solid",
            Background::Linear { .. } => "Linear",
            Background::Radial { .. } => "Radial",
            Background::Corners(_) => "Corners",
            Background::Image { .. } => "Image",
            Background::Video { .. } => "Video",
            Background::Pattern { .. } => "Pattern",
            Background::Aurora { .. } => "Aurora",
            Background::Starfield { .. } => "Starfield",
            Background::Bokeh { .. } => "Bokeh",
            Background::Flow { .. } => "Flow",
            Background::Layers(_) => "Layers",
            Background::Custom(_) => "Custom",
        };
        write!(f, "Background::{name}")
    }
}

impl From<Color> for Background {
    fn from(c: Color) -> Self {
        Background::Solid(c)
    }
}

/// Names accepted by [`Background::preset`].
pub const BACKGROUND_PRESETS: &[&str] = &[
    "dark",
    "black",
    "manim",
    "white",
    "light",
    "paper",
    "lesson",
    "chalkboard",
    "blackboard",
    "whiteboard",
    "blueprint",
    "graph-paper",
    "grid",
    "dots",
    "notebook",
    "terminal",
    "midnight",
    "sunset",
    "ocean",
    "forest",
    "candy",
    "spotlight",
    "nebula",
    "aurora",
    "starfield",
    "space",
    "bokeh",
    "flow",
    "lava",
];

impl Background {
    pub fn solid(c: Color) -> Self {
        Background::Solid(c)
    }
    /// Two-colour linear gradient at `angle` degrees.
    pub fn linear(a: Color, b: Color, angle: f64) -> Self {
        Background::Linear {
            angle,
            stops: vec![(0.0, a), (1.0, b)],
        }
    }
    /// Two-colour radial gradient from the centre.
    pub fn radial(inner: Color, outer: Color) -> Self {
        Background::Radial {
            center: (0.5, 0.5),
            radius: 0.6,
            stops: vec![(0.0, inner), (1.0, outer)],
        }
    }
    /// Image file (png/jpg/gif first frame) covering the frame.
    pub fn image(path: &str) -> anyhow::Result<Self> {
        Ok(Background::Image {
            pixmap: Arc::new(crate::render::load_image(path)?),
            fit: Fit::Cover,
            dim: 0.0,
        })
    }
    /// Video file (decoded with ffmpeg) looping behind the scene.
    pub fn video(path: &str) -> anyhow::Result<Self> {
        Ok(Background::Video {
            source: Arc::new(VideoSource::open(path, 1920)?),
            fit: Fit::Cover,
            dim: 0.0,
        })
    }
    pub fn pattern(kind: Pattern, base: Color, ink: Color) -> Self {
        let spacing = match kind {
            Pattern::GraphPaper => 1.0 / 40.0,
            Pattern::Dots | Pattern::Crosses => 1.0 / 18.0,
            Pattern::Ruled | Pattern::Notebook => 1.0 / 16.0,
            Pattern::Checker => 1.0 / 8.0,
            _ => 1.0 / 16.0,
        };
        Background::Pattern {
            kind,
            base,
            ink,
            spacing,
            width: 1.5,
            drift: Vec2::ZERO,
        }
    }
    /// Darken an image/video background (no effect on other kinds).
    pub fn dim(self, amount: f32) -> Self {
        match self {
            Background::Image { pixmap, fit, .. } => Background::Image {
                pixmap,
                fit,
                dim: amount,
            },
            Background::Video { source, fit, .. } => Background::Video {
                source,
                fit,
                dim: amount,
            },
            other => other,
        }
    }
    /// Change how an image/video fills the frame.
    pub fn fit(self, fit: Fit) -> Self {
        match self {
            Background::Image { pixmap, dim, .. } => Background::Image { pixmap, fit, dim },
            Background::Video { source, dim, .. } => Background::Video { source, fit, dim },
            other => other,
        }
    }
    /// Scroll a pattern (frame heights per second).
    pub fn drift(self, v: Vec2) -> Self {
        match self {
            Background::Pattern {
                kind,
                base,
                ink,
                spacing,
                width,
                ..
            } => Background::Pattern {
                kind,
                base,
                ink,
                spacing,
                width,
                drift: v,
            },
            other => other,
        }
    }
    /// Draw `top` over this background at `opacity`.
    pub fn with_layer(self, top: Background, opacity: f32) -> Self {
        match self {
            Background::Layers(mut v) => {
                v.push((top, opacity));
                Background::Layers(v)
            }
            other => Background::Layers(vec![(other, 1.0), (top, opacity)]),
        }
    }

    /// A ready-made background by name (see [`BACKGROUND_PRESETS`]).
    pub fn preset(name: &str) -> Option<Background> {
        let c = |h: &str| Color::parse(h).unwrap_or(BLACK);
        let n = name.to_ascii_lowercase().replace('_', "-");
        Some(match n.as_str() {
            "dark" | "default" => Background::Solid(BACKGROUND),
            "black" | "manim" | "3b1b" => Background::Solid(c("#000000")),
            "white" => Background::Solid(c("#FFFFFF")),
            "light" => Background::linear(c("#FFFFFF"), c("#EEF1F6"), -90.0),
            // Warm studio paper used by lesson scripts (`theme("lesson")`).
            "lesson" => Background::radial(c("#FFFDF8"), c("#E4DCCE")).with_layer(
                Background::Pattern {
                    kind: Pattern::Dots,
                    base: Color::TRANSPARENT,
                    ink: c("#D5CBB8"),
                    spacing: 1.0 / 52.0,
                    width: 1.3,
                    drift: Vec2::ZERO,
                },
                0.4,
            ),
            "paper" => Background::Solid(c("#F7F3E8")).with_layer(
                Background::Pattern {
                    kind: Pattern::Dots,
                    base: Color::TRANSPARENT,
                    ink: c("#D9CFB8"),
                    spacing: 1.0 / 60.0,
                    width: 1.2,
                    drift: Vec2::ZERO,
                },
                0.5,
            ),
            "chalkboard" | "blackboard" => Background::Radial {
                center: (0.5, 0.45),
                radius: 0.75,
                stops: vec![(0.0, c("#2F4A3A")), (1.0, c("#1B2B22"))],
            },
            "whiteboard" => Background::radial(c("#FFFFFF"), c("#E9ECEF")),
            "blueprint" => Background::Solid(c("#0F3D7A")).with_layer(
                Background::Pattern {
                    kind: Pattern::GraphPaper,
                    base: Color::TRANSPARENT,
                    ink: c("#5B8FD6"),
                    spacing: 1.0 / 40.0,
                    width: 1.0,
                    drift: Vec2::ZERO,
                },
                0.55,
            ),
            "graph-paper" => Background::pattern(Pattern::GraphPaper, c("#FFFFFF"), c("#9CC3E6")),
            "grid" => Background::Pattern {
                kind: Pattern::Grid,
                base: BACKGROUND,
                ink: c("#1F2733"),
                spacing: 1.0 / 16.0,
                width: 1.5,
                drift: Vec2::ZERO,
            },
            "dots" => Background::Pattern {
                kind: Pattern::Dots,
                base: BACKGROUND,
                ink: c("#2A3442"),
                spacing: 1.0 / 18.0,
                width: 2.5,
                drift: Vec2::ZERO,
            },
            "notebook" => Background::pattern(Pattern::Notebook, c("#FFFEF8"), c("#A9C7E8")),
            "terminal" => Background::radial(c("#0B1F12"), c("#020804")),
            "midnight" => Background::linear(c("#0F2027"), c("#2C5364"), -60.0),
            "sunset" => Background::Linear {
                angle: -90.0,
                stops: vec![
                    (0.0, c("#2B1055")),
                    (0.55, c("#7A2E6E")),
                    (1.0, c("#D16B4A")),
                ],
            },
            "ocean" => Background::linear(c("#0B3D5C"), c("#04131F"), -90.0),
            "forest" => Background::linear(c("#134E3A"), c("#06140F"), -90.0),
            "candy" => {
                Background::Corners([c("#FF9A9E"), c("#FAD0C4"), c("#A18CD1"), c("#FBC2EB")])
            }
            "spotlight" => Background::Radial {
                center: (0.5, 0.42),
                radius: 0.7,
                stops: vec![(0.0, c("#2A3140")), (1.0, c("#07090D"))],
            },
            "nebula" => {
                Background::Corners([c("#1A1033"), c("#0E2140"), c("#2B0F2E"), c("#0A0F1F")])
                    .with_layer(
                        Background::Starfield {
                            base: Color::TRANSPARENT,
                            density: 120.0,
                            speed: 0.01,
                        },
                        1.0,
                    )
            }
            "aurora" => Background::Aurora {
                base: c("#050A14"),
                colors: vec![c("#1FD1A5"), c("#3A7BD5"), c("#8E44AD")],
                speed: 0.08,
            },
            "starfield" | "space" => Background::Starfield {
                base: c("#03040A"),
                density: 220.0,
                speed: 0.02,
            },
            "bokeh" => Background::Bokeh {
                base: c("#0A0D14"),
                colors: vec![BLUE, PURPLE, TEAL, GOLD],
                count: 28,
                speed: 0.03,
            },
            "flow" | "mesh" => Background::Flow {
                colors: vec![c("#0F172A"), c("#1E3A8A"), c("#312E81"), c("#0F766E")],
                scale: 1.4,
                speed: 0.06,
            },
            "lava" => Background::Flow {
                colors: vec![c("#1A0505"), c("#7F1D1D"), c("#C2410C"), c("#451A03")],
                scale: 1.8,
                speed: 0.08,
            },
            _ => {
                if let Some(col) = Color::parse(name) {
                    return Some(Background::Solid(col));
                }
                return None;
            }
        })
    }

    /// The colour this background is closest to (used for clearing and fades).
    pub fn base_color(&self) -> Color {
        match self {
            Background::Solid(c) => *c,
            Background::Linear { stops, .. } | Background::Radial { stops, .. } => {
                stops.iter().fold(stops[0].1, |acc, s| acc.mix(s.1, 0.5))
            }
            Background::Corners(c) => c[0].mix(c[3], 0.5),
            Background::Image { .. } | Background::Video { .. } => BLACK,
            Background::Pattern { base, .. }
            | Background::Aurora { base, .. }
            | Background::Starfield { base, .. }
            | Background::Bokeh { base, .. } => *base,
            Background::Flow { colors, .. } => colors.first().copied().unwrap_or(BLACK),
            Background::Layers(v) => v.first().map(|l| l.0.base_color()).unwrap_or(BLACK),
            Background::Custom(_) => BLACK,
        }
    }

    /// Is this a light background (mean luminance above one half)?
    pub fn is_light(&self) -> bool {
        self.base_color().to_oklab()[0] > 0.7
    }

    /// Draw over the whole pixmap at time `t` (seconds).
    pub fn draw(&self, t: f64, pm: &mut sk::Pixmap) {
        self.draw_with(t, pm, 1.0);
    }

    fn draw_with(&self, t: f64, pm: &mut sk::Pixmap, opacity: f32) {
        let (w, h) = (pm.width() as f32, pm.height() as f32);
        let full = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
        let fill = |pm: &mut sk::Pixmap, shader: sk::Shader| {
            let paint = sk::Paint {
                shader,
                anti_alias: false,
                ..Default::default()
            };
            pm.fill_rect(full, &paint, sk::Transform::identity(), None);
        };
        let stops = |st: &[(f32, Color)]| -> Vec<sk::GradientStop> {
            st.iter()
                .map(|(p, c)| sk::GradientStop::new(*p, c.fade(opacity).to_skia()))
                .collect()
        };
        match self {
            Background::Solid(c) => fill(pm, sk::Shader::SolidColor(c.fade(opacity).to_skia())),
            Background::Linear { angle, stops: st } => {
                let a = angle.to_radians();
                let (dx, dy) = (a.cos() as f32, -a.sin() as f32);
                // Project the frame corners onto the direction for an edge-to-edge ramp.
                let half = (w * dx.abs() + h * dy.abs()) / 2.0;
                let (cx, cy) = (w / 2.0, h / 2.0);
                if let Some(s) = sk::LinearGradient::new(
                    sk::Point::from_xy(cx - dx * half, cy - dy * half),
                    sk::Point::from_xy(cx + dx * half, cy + dy * half),
                    stops(st),
                    sk::SpreadMode::Pad,
                    sk::Transform::identity(),
                ) {
                    fill(pm, s);
                }
            }
            Background::Radial {
                center,
                radius,
                stops: st,
            } => {
                let r = (w.hypot(h) as f64 * radius) as f32;
                let p = sk::Point::from_xy(center.0 as f32 * w, center.1 as f32 * h);
                if let Some(s) = sk::RadialGradient::new(
                    p,
                    0.0,
                    p,
                    r.max(1.0),
                    stops(st),
                    sk::SpreadMode::Pad,
                    sk::Transform::identity(),
                ) {
                    fill(pm, s);
                }
            }
            Background::Corners(c) => {
                let lab: Vec<[f32; 3]> = c.iter().map(|c| c.to_oklab()).collect();
                draw_field(pm, opacity, |u, v| {
                    let s = |i: usize| {
                        let top = lab[0][i] + (lab[1][i] - lab[0][i]) * u;
                        let bot = lab[2][i] + (lab[3][i] - lab[2][i]) * u;
                        top + (bot - top) * v
                    };
                    Color::from_oklab([s(0), s(1), s(2)])
                });
            }
            Background::Image { pixmap, fit, dim } => {
                draw_image(pm, pixmap, *fit, opacity);
                if *dim > 0.0 {
                    fill(
                        pm,
                        sk::Shader::SolidColor(BLACK.fade(dim * opacity).to_skia()),
                    );
                }
            }
            Background::Video { source, fit, dim } => {
                let d = source.duration().max(1e-6);
                if let Some(frame) = source.frame(t.rem_euclid(d)) {
                    draw_image(pm, &frame, *fit, opacity);
                }
                if *dim > 0.0 {
                    fill(
                        pm,
                        sk::Shader::SolidColor(BLACK.fade(dim * opacity).to_skia()),
                    );
                }
            }
            Background::Pattern {
                kind,
                base,
                ink,
                spacing,
                width,
                drift,
            } => {
                if base.a > 0.0 {
                    fill(pm, sk::Shader::SolidColor(base.fade(opacity).to_skia()));
                }
                draw_pattern(
                    pm,
                    *kind,
                    *base,
                    ink.fade(opacity),
                    *spacing,
                    *width,
                    *drift * t,
                );
            }
            Background::Aurora {
                base,
                colors,
                speed,
            } => {
                fill(pm, sk::Shader::SolidColor(base.fade(opacity).to_skia()));
                let z = t * speed;
                let cols = colors.clone();
                let base = *base;
                draw_field(pm, opacity, move |u, v| {
                    let (u, v) = (u as f64, v as f64);
                    let mut c = base;
                    for (i, col) in cols.iter().enumerate() {
                        let k = i as f64 * 1.7;
                        // A wavy horizontal band whose centre and brightness drift with noise.
                        let centre =
                            0.35 + 0.18 * crate::util::noise2d("aurora-c", u * 1.3 + k, z + k);
                        let d = (v - centre - 0.1 * i as f64) / 0.13;
                        let band = (-d * d).exp();
                        let shimmer = 0.5
                            + 0.5 * crate::util::noise3d("aurora-s", u * 2.2, v * 0.8 + k, z * 1.5);
                        let a = (band * shimmer * 0.85) as f32;
                        c = c.mix_srgb(*col, a.clamp(0.0, 1.0));
                    }
                    c
                });
            }
            Background::Starfield {
                base,
                density,
                speed,
            } => {
                if base.a > 0.0 {
                    fill(pm, sk::Shader::SolidColor(base.fade(opacity).to_skia()));
                }
                draw_stars(pm, *density, *speed, t, opacity);
            }
            Background::Bokeh {
                base,
                colors,
                count,
                speed,
            } => {
                fill(pm, sk::Shader::SolidColor(base.fade(opacity).to_skia()));
                draw_bokeh(pm, colors, *count, *speed, t, opacity);
            }
            Background::Flow {
                colors,
                scale,
                speed,
            } => {
                let lab: Vec<[f32; 3]> = colors.iter().map(|c| c.to_oklab()).collect();
                let (scale, z) = (*scale, t * speed);
                let aspect = (w / h) as f64;
                draw_field(pm, opacity, move |u, v| {
                    if lab.is_empty() {
                        return BLACK;
                    }
                    let (x, y) = (u as f64 * scale * aspect, v as f64 * scale);
                    // Domain-warped noise gives soft, organic colour regions.
                    let wx = crate::util::noise3d("flow-wx", x, y, z);
                    let wy = crate::util::noise3d("flow-wy", x + 5.2, y + 1.3, z);
                    let n = crate::util::noise3d("flow", x + wx * 0.8, y + wy * 0.8, z * 0.7);
                    let f = ((n * 0.5 + 0.5) * (lab.len() - 1) as f64)
                        .clamp(0.0, (lab.len() - 1) as f64);
                    let i = (f.floor() as usize).min(lab.len().saturating_sub(2));
                    let j = (i + 1).min(lab.len() - 1);
                    let k = (f - i as f64) as f32;
                    let k = k * k * (3.0 - 2.0 * k);
                    let m = |a: usize| lab[i][a] + (lab[j][a] - lab[i][a]) * k;
                    Color::from_oklab([m(0), m(1), m(2)])
                });
            }
            Background::Layers(layers) => {
                for (b, o) in layers {
                    b.draw_with(t, pm, opacity * o);
                }
            }
            Background::Custom(f) => {
                if opacity >= 0.999 {
                    f(t, pm);
                } else if let Some(mut layer) = sk::Pixmap::new(pm.width(), pm.height()) {
                    f(t, &mut layer);
                    pm.draw_pixmap(
                        0,
                        0,
                        layer.as_ref(),
                        &sk::PixmapPaint {
                            opacity,
                            ..Default::default()
                        },
                        sk::Transform::identity(),
                        None,
                    );
                }
            }
        }
    }
}

/// Evaluate a smooth colour field at low resolution and upscale it (bilinear), which is
/// far cheaper than per-pixel evaluation and invisible for soft gradients.
fn draw_field(pm: &mut sk::Pixmap, opacity: f32, f: impl Fn(f32, f32) -> Color + Sync) {
    use rayon::prelude::*;
    let (w, h) = (pm.width(), pm.height());
    let lw = (w / 8).clamp(16, 400);
    let lh = (h / 8).clamp(9, 225);
    let Some(mut low) = sk::Pixmap::new(lw, lh) else {
        return;
    };
    low.data_mut()
        .par_chunks_mut(lw as usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            for (x, px) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let c = f(x as f32 / (lw - 1) as f32, y as f32 / (lh - 1) as f32);
                let c = c.to_skia().premultiply().to_color_u8();
                *px = [c.red(), c.green(), c.blue(), c.alpha()];
            }
        });
    let tr = sk::Transform::from_scale(w as f32 / lw as f32, h as f32 / lh as f32);
    pm.draw_pixmap(
        0,
        0,
        low.as_ref(),
        &sk::PixmapPaint {
            opacity,
            quality: sk::FilterQuality::Bilinear,
            ..Default::default()
        },
        tr,
        None,
    );
}

fn draw_image(pm: &mut sk::Pixmap, img: &sk::Pixmap, fit: Fit, opacity: f32) {
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let paint = sk::PixmapPaint {
        opacity,
        quality: sk::FilterQuality::Bicubic,
        ..Default::default()
    };
    match fit {
        Fit::Tile => {
            let mut y = 0.0;
            while y < h {
                let mut x = 0.0;
                while x < w {
                    pm.draw_pixmap(
                        x as i32,
                        y as i32,
                        img.as_ref(),
                        &paint,
                        sk::Transform::identity(),
                        None,
                    );
                    x += iw;
                }
                y += ih;
            }
        }
        _ => {
            let (sx, sy) = match fit {
                Fit::Cover => {
                    let s = (w / iw).max(h / ih);
                    (s, s)
                }
                Fit::Contain => {
                    let s = (w / iw).min(h / ih);
                    (s, s)
                }
                _ => (w / iw, h / ih),
            };
            let tr =
                sk::Transform::from_row(sx, 0.0, 0.0, sy, (w - iw * sx) / 2.0, (h - ih * sy) / 2.0);
            pm.draw_pixmap(0, 0, img.as_ref(), &paint, tr, None);
        }
    }
}

fn stroke_line(
    pm: &mut sk::Pixmap,
    a: (f32, f32),
    b: (f32, f32),
    paint: &sk::Paint,
    stroke: &sk::Stroke,
) {
    let mut pb = sk::PathBuilder::new();
    pb.move_to(a.0, a.1);
    pb.line_to(b.0, b.1);
    if let Some(p) = pb.finish() {
        pm.stroke_path(&p, paint, stroke, sk::Transform::identity(), None);
    }
}

fn draw_pattern(
    pm: &mut sk::Pixmap,
    kind: Pattern,
    base: Color,
    ink: Color,
    spacing: f64,
    width: f32,
    offset: Vec2,
) {
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let px = h / 1080.0;
    let step = (spacing as f32 * h).max(2.0);
    let (ox, oy) = (
        (offset.x as f32 * h).rem_euclid(step),
        (-offset.y as f32 * h).rem_euclid(step),
    );
    let paint = |c: Color| sk::Paint {
        shader: sk::Shader::SolidColor(c.to_skia()),
        anti_alias: true,
        ..Default::default()
    };
    let stroke = |wid: f32| sk::Stroke {
        width: (wid * px).max(0.5),
        ..Default::default()
    };
    let p = paint(ink);
    let xs = || (-1..=((w / step) as i32 + 1)).map(move |i| i as f32 * step + ox);
    let ys = || (-1..=((h / step) as i32 + 1)).map(move |i| i as f32 * step + oy);
    match kind {
        Pattern::Grid => {
            let s = stroke(width);
            xs().for_each(|x| stroke_line(pm, (x, 0.0), (x, h), &p, &s));
            ys().for_each(|y| stroke_line(pm, (0.0, y), (w, y), &p, &s));
        }
        Pattern::GraphPaper => {
            let thin = stroke(width * 0.6);
            let thick = stroke(width * 1.4);
            let major = paint(ink);
            let minor = paint(ink.with_alpha(ink.a * 0.55));
            let i0 = |o: f32| -((o / step).floor() as i32);
            for (k, x) in xs().enumerate() {
                let major_line = (k as i32 - 1 + i0(ox)).rem_euclid(5) == 0;
                let (pp, ss) = if major_line {
                    (&major, &thick)
                } else {
                    (&minor, &thin)
                };
                stroke_line(pm, (x, 0.0), (x, h), pp, ss);
            }
            for (k, y) in ys().enumerate() {
                let major_line = (k as i32 - 1 + i0(oy)).rem_euclid(5) == 0;
                let (pp, ss) = if major_line {
                    (&major, &thick)
                } else {
                    (&minor, &thin)
                };
                stroke_line(pm, (0.0, y), (w, y), pp, ss);
            }
        }
        Pattern::Dots => {
            let r = (width * px).max(0.8);
            let mut pb = sk::PathBuilder::new();
            for x in xs() {
                for y in ys() {
                    pb.push_circle(x, y, r);
                }
            }
            if let Some(path) = pb.finish() {
                pm.fill_path(
                    &path,
                    &p,
                    sk::FillRule::Winding,
                    sk::Transform::identity(),
                    None,
                );
            }
        }
        Pattern::Ruled | Pattern::Notebook => {
            let s = stroke(width);
            ys().for_each(|y| stroke_line(pm, (0.0, y), (w, y), &p, &s));
            if kind == Pattern::Notebook {
                let m = w * 0.09;
                stroke_line(
                    pm,
                    (m, 0.0),
                    (m, h),
                    &paint(Color::parse("#E57373").unwrap_or(RED)),
                    &stroke(width * 1.3),
                );
            }
        }
        Pattern::Checker => {
            let mut pb = sk::PathBuilder::new();
            for (i, x) in xs().enumerate() {
                for (j, y) in ys().enumerate() {
                    if (i + j) % 2 == 0 {
                        if let Some(r) = sk::Rect::from_xywh(x, y, step, step) {
                            pb.push_rect(r);
                        }
                    }
                }
            }
            if let Some(path) = pb.finish() {
                pm.fill_path(
                    &path,
                    &p,
                    sk::FillRule::Winding,
                    sk::Transform::identity(),
                    None,
                );
            }
            let _ = base;
        }
        Pattern::Stripes => {
            let s = stroke(width.max(step / px * 0.25));
            let n = ((w + h) / step) as i32 + 2;
            for i in -1..n {
                let x = i as f32 * step + ox - h;
                stroke_line(pm, (x, h), (x + h, 0.0), &p, &s);
            }
        }
        Pattern::Crosses => {
            let s = stroke(width);
            let a = step * 0.12;
            for x in xs() {
                for y in ys() {
                    stroke_line(pm, (x - a, y), (x + a, y), &p, &s);
                    stroke_line(pm, (x, y - a), (x, y + a), &p, &s);
                }
            }
        }
        Pattern::Isometric => {
            let s = stroke(width);
            ys().for_each(|y| stroke_line(pm, (0.0, y), (w, y), &p, &s));
            let dx = step / 3f32.sqrt() * 2.0;
            let n = ((w + h * 2.0) / dx) as i32 + 2;
            for i in -n..n {
                let x = i as f32 * dx + ox;
                let run = h / 3f32.sqrt();
                stroke_line(pm, (x, 0.0), (x + run, h), &p, &s);
                stroke_line(pm, (x, 0.0), (x - run, h), &p, &s);
            }
        }
    }
}

fn draw_stars(pm: &mut sk::Pixmap, density: f64, speed: f64, t: f64, opacity: f32) {
    let (w, h) = (pm.width() as f64, pm.height() as f64);
    let aspect = w / h;
    let n = (density * aspect) as usize;
    let mut rng = Rng::new(7);
    let px = h / 1080.0;
    for _ in 0..n {
        let (x0, y0) = (rng.next_f64(), rng.next_f64());
        let depth = 0.3 + 0.7 * rng.next_f64();
        let size = (0.6 + 1.8 * rng.next_f64().powi(3)) * depth;
        let phase = rng.next_f64() * TAU;
        let rate = 0.5 + 2.0 * rng.next_f64();
        // Parallax drift to the left; nearer stars move faster.
        let x = (x0 - t * speed * depth / aspect).rem_euclid(1.0) * w;
        let y = y0 * h;
        let tw = 0.65 + 0.35 * (t * rate + phase).sin();
        let a = (depth * tw) as f32 * opacity;
        let tint = if rng.next_f64() < 0.15 {
            Color::parse("#BFD7FF").unwrap_or(WHITE)
        } else {
            WHITE
        };
        let r = (size * px) as f32;
        if let Some(s) = sk::RadialGradient::new(
            sk::Point::from_xy(x as f32, y as f32),
            0.0,
            sk::Point::from_xy(x as f32, y as f32),
            r * 2.5,
            vec![
                sk::GradientStop::new(0.0, tint.fade(a).to_skia()),
                sk::GradientStop::new(0.4, tint.fade(a * 0.5).to_skia()),
                sk::GradientStop::new(1.0, tint.fade(0.0).to_skia()),
            ],
            sk::SpreadMode::Pad,
            sk::Transform::identity(),
        ) {
            let paint = sk::Paint {
                shader: s,
                anti_alias: true,
                ..Default::default()
            };
            if let Some(rect) =
                sk::Rect::from_xywh(x as f32 - r * 2.5, y as f32 - r * 2.5, r * 5.0, r * 5.0)
            {
                pm.fill_rect(rect, &paint, sk::Transform::identity(), None);
            }
        }
    }
}

fn draw_bokeh(
    pm: &mut sk::Pixmap,
    colors: &[Color],
    count: usize,
    speed: f64,
    t: f64,
    opacity: f32,
) {
    if colors.is_empty() {
        return;
    }
    let (w, h) = (pm.width() as f64, pm.height() as f64);
    let mut rng = Rng::new(11);
    for i in 0..count {
        let (x0, y0) = (rng.next_f64(), rng.next_f64());
        let r = (0.04 + 0.11 * rng.next_f64()) * h;
        let (vx, vy) = (rng.next_f64() - 0.5, rng.next_f64() - 0.5);
        let phase = rng.next_f64() * TAU;
        let col = colors[i % colors.len()];
        let x = (x0 + vx * t * speed).rem_euclid(1.2) * w - 0.1 * w;
        let y = (y0 + vy * t * speed).rem_euclid(1.2) * h - 0.1 * h;
        let a = (0.10 + 0.08 * (t * 0.7 + phase).sin()) as f32 * opacity;
        if let Some(s) = sk::RadialGradient::new(
            sk::Point::from_xy(x as f32, y as f32),
            0.0,
            sk::Point::from_xy(x as f32, y as f32),
            r as f32,
            vec![
                sk::GradientStop::new(0.0, col.fade(a).to_skia()),
                sk::GradientStop::new(0.8, col.fade(a * 0.8).to_skia()),
                sk::GradientStop::new(1.0, col.fade(0.0).to_skia()),
            ],
            sk::SpreadMode::Pad,
            sk::Transform::identity(),
        ) {
            let paint = sk::Paint {
                shader: s,
                anti_alias: true,
                ..Default::default()
            };
            let r = r as f32;
            if let Some(rect) = sk::Rect::from_xywh(x as f32 - r, y as f32 - r, r * 2.0, r * 2.0) {
                pm.fill_rect(rect, &paint, sk::Transform::identity(), None);
            }
        }
    }
}

/// A look for a whole video: background plus the ink mode that keeps colours readable.
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub background: Background,
    pub ink: Ink,
}

/// Names accepted by [`Theme::preset`] (every background preset works too).
pub const THEME_PRESETS: &[&str] = &[
    "dark",
    "light",
    "manim",
    "chalkboard",
    "whiteboard",
    "blueprint",
    "paper",
    "lesson",
    "notebook",
    "space",
    "sunset",
];

impl Theme {
    pub fn new(background: Background) -> Self {
        let ink = if background.is_light() {
            Ink::Light
        } else {
            Ink::Normal
        };
        Self {
            name: "custom".into(),
            background,
            ink,
        }
    }
    /// A named theme; any background preset name is accepted and gets the right ink.
    pub fn preset(name: &str) -> Option<Theme> {
        let bg = Background::preset(name)?;
        let mut t = Theme::new(bg);
        t.name = name.to_string();
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(pm: &sk::Pixmap, x: u32, y: u32) -> [u8; 4] {
        let c = pm.pixel(x, y).unwrap();
        [c.red(), c.green(), c.blue(), c.alpha()]
    }

    #[test]
    fn every_preset_draws() {
        for name in BACKGROUND_PRESETS {
            let bg = Background::preset(name).unwrap_or_else(|| panic!("{name}"));
            let mut pm = sk::Pixmap::new(160, 90).unwrap();
            bg.draw(1.25, &mut pm);
            // Opaque everywhere (no holes).
            assert!(
                pm.pixels().iter().all(|p| p.alpha() == 255),
                "{name} left transparent pixels"
            );
        }
        for name in THEME_PRESETS {
            assert!(Theme::preset(name).is_some(), "{name}");
        }
    }

    #[test]
    fn linear_gradient_runs_edge_to_edge() {
        let bg = Background::linear(BLACK, WHITE, 0.0);
        let mut pm = sk::Pixmap::new(100, 10).unwrap();
        bg.draw(0.0, &mut pm);
        assert!(px(&pm, 0, 5)[0] < 10);
        assert!(px(&pm, 99, 5)[0] > 245);
        // Vertical: 90° goes bottom → top.
        let bg = Background::linear(BLACK, WHITE, 90.0);
        let mut pm = sk::Pixmap::new(10, 100).unwrap();
        bg.draw(0.0, &mut pm);
        assert!(px(&pm, 5, 99)[0] < 10 && px(&pm, 5, 0)[0] > 245);
    }

    #[test]
    fn light_detection_and_ink() {
        assert!(Theme::preset("whiteboard").unwrap().ink == Ink::Light);
        assert!(Theme::preset("dark").unwrap().ink == Ink::Normal);
        // White text becomes dark ink; a dark grey becomes light; accents get darker.
        let w = Ink::Light.apply(WHITE).to_oklab()[0];
        assert!(w < 0.3, "{w}");
        let blue = Ink::Light.apply(BLUE).to_oklab()[0];
        assert!(blue < BLUE.to_oklab()[0]);
        assert_eq!(Ink::Normal.apply(BLUE), BLUE);
    }

    #[test]
    fn animated_backgrounds_change_over_time_but_are_deterministic() {
        let bg = Background::preset("flow").unwrap();
        let mut a = sk::Pixmap::new(64, 36).unwrap();
        let mut b = sk::Pixmap::new(64, 36).unwrap();
        let mut c = sk::Pixmap::new(64, 36).unwrap();
        bg.draw(0.0, &mut a);
        bg.draw(0.0, &mut b);
        bg.draw(8.0, &mut c);
        assert_eq!(a.data(), b.data());
        assert_ne!(a.data(), c.data());
    }
}
