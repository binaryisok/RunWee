//! Assets: SVG import (manim's `SVGMobject`), images (`<Img>`/`ImageMobject`),
//! animated GIFs (`<Gif>`), and video clips (`<OffthreadVideo>`/`VideoMobject`).

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::Path;
use crate::scene::{Obj, Scene};
use crate::style::*;
use anyhow::{bail, Context, Result};
use std::sync::Arc;
use tiny_skia::Pixmap;

// --------------------------------------------------------------------- SVG

fn svg_color(c: usvg::Color, opacity: f32) -> Color {
    rgb8(c.red, c.green, c.blue).with_alpha(opacity)
}

fn svg_paint(p: &usvg::Paint, xf: &Affine) -> Paint {
    match p {
        usvg::Paint::Color(c) => Paint::Solid(svg_color(*c, 1.0)),
        usvg::Paint::LinearGradient(g) => {
            let t = g.transform();
            let gt = Affine {
                a: t.sx as f64,
                b: t.ky as f64,
                c: t.kx as f64,
                d: t.sy as f64,
                e: t.tx as f64,
                f: t.ty as f64,
            };
            let m = xf.then_after(gt);
            Paint::Linear {
                start: m.apply(v2(g.x1() as f64, g.y1() as f64)),
                end: m.apply(v2(g.x2() as f64, g.y2() as f64)),
                stops: g
                    .stops()
                    .iter()
                    .map(|s| (s.offset().get(), svg_color(s.color(), s.opacity().get())))
                    .collect(),
            }
        }
        usvg::Paint::RadialGradient(g) => {
            let t = g.transform();
            let gt = Affine {
                a: t.sx as f64,
                b: t.ky as f64,
                c: t.kx as f64,
                d: t.sy as f64,
                e: t.tx as f64,
                f: t.ty as f64,
            };
            let m = xf.then_after(gt);
            Paint::Radial {
                center: m.apply(v2(g.cx() as f64, g.cy() as f64)),
                radius: g.r().get() as f64 * m.mean_scale(),
                focal: m.apply(v2(g.fx() as f64, g.fy() as f64)),
                stops: g
                    .stops()
                    .iter()
                    .map(|s| (s.offset().get(), svg_color(s.color(), s.opacity().get())))
                    .collect(),
            }
        }
        usvg::Paint::Pattern(_) => Paint::Solid(GRAY),
    }
}

fn convert_group(g: &usvg::Group, out: &mut Mobject, group_opacity: f32) {
    let op = group_opacity * g.opacity().get();
    for node in g.children() {
        match node {
            usvg::Node::Group(sub) => {
                let mut m = Mobject::empty();
                if !sub.id().is_empty() {
                    m.tag = Some(sub.id().into());
                }
                convert_group(sub, &mut m, op);
                out.push(m);
            }
            usvg::Node::Path(p) => {
                if !p.is_visible() {
                    continue;
                }
                let t = p.abs_transform();
                let xf = Affine {
                    a: t.sx as f64,
                    b: t.ky as f64,
                    c: t.kx as f64,
                    d: t.sy as f64,
                    e: t.tx as f64,
                    f: t.ty as f64,
                };
                let mut path = Path::new();
                for seg in p.data().segments() {
                    use tiny_skia::PathSegment as S;
                    let pt = |q: tiny_skia::Point| xf.apply(v2(q.x as f64, q.y as f64));
                    match seg {
                        S::MoveTo(a) => {
                            path.move_to(pt(a));
                        }
                        S::LineTo(a) => {
                            path.line_to(pt(a));
                        }
                        S::QuadTo(a, b) => {
                            path.quad_to(pt(a), pt(b));
                        }
                        S::CubicTo(a, b, c) => {
                            path.cubic_to(pt(a), pt(b), pt(c));
                        }
                        S::Close => {
                            path.close();
                        }
                    }
                }
                let mut style = Style {
                    fill_opacity: 0.0,
                    stroke_width: 0.0,
                    ..Default::default()
                };
                if let Some(f) = p.fill() {
                    style.fill = svg_paint(f.paint(), &xf);
                    style.fill_opacity = f.opacity().get();
                    style.fill_rule = match f.rule() {
                        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
                        usvg::FillRule::NonZero => FillRule::NonZero,
                    };
                }
                if let Some(s) = p.stroke() {
                    style.stroke = svg_paint(s.paint(), &xf);
                    style.stroke_opacity = s.opacity().get();
                    // SVG user units → px@1080 is resolved after scaling (see svg_mobject)
                    style.stroke_width = (s.width().get() as f64 * xf.mean_scale()) as f32;
                }
                style.opacity = op;
                let mut m = Mobject::from_path_styled(path, style);
                if !p.id().is_empty() {
                    m.tag = Some(p.id().into());
                }
                out.push(m);
            }
            usvg::Node::Image(img) => {
                let data = match img.kind() {
                    usvg::ImageKind::PNG(d) | usvg::ImageKind::JPEG(d) => Some(d.clone()),
                    _ => None,
                };
                if let Some(d) = data.and_then(|d| decode_image_bytes(&d).ok()) {
                    let b = img.abs_bounding_box();
                    let mut m = Mobject::image(Arc::new(d), b.width() as f64);
                    m = m.move_to(v2(
                        (b.left() + b.right()) as f64 / 2.0,
                        (b.top() + b.bottom()) as f64 / 2.0,
                    ));
                    // y flip happens with the whole tree below; pre-flip image to keep it upright
                    m = m.flip(RIGHT);
                    out.push(m);
                }
            }
            usvg::Node::Text(_) => {}
        }
    }
}

/// Parse SVG text into a Mobject `height` units tall, centred at the origin.
/// Children mirror the SVG groups/paths (ids become tags).
pub fn svg_from_str(svg: &str, height: f64) -> Result<Mobject> {
    let tree =
        usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).context("invalid SVG")?;
    let mut root = Mobject::empty();
    convert_group(tree.root(), &mut root, 1.0);
    // SVG y points down: flip, then normalise size
    let mut m = root.apply_affine(Affine::scale(1.0, -1.0));
    let h = m.height().max(1e-9);
    let k = height / h;
    m = m.scale_about(k, Vec2::ZERO);
    // stroke widths: SVG units → px@1080 for an 8-unit frame (135 px per unit)
    m.for_each_mut(&mut |n| n.style.stroke_width *= (k * 135.0) as f32);
    let c = m.center();
    Ok(m.shift(-c))
}

/// Load an SVG file (manim's `SVGMobject`).
pub fn svg_mobject(path: &str, height: f64) -> Result<Mobject> {
    let s = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    svg_from_str(&s, height)
}

// ------------------------------------------------------------------ images

fn decode_image_bytes(bytes: &[u8]) -> Result<Pixmap> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    let (w, h) = img.dimensions();
    let mut data = img.into_raw();
    premultiply(&mut data);
    Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h).context("size")?).context("pixmap")
}

fn premultiply(data: &mut [u8]) {
    for px in data.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
}

/// Image file as a Mobject `width` units wide (manim's `ImageMobject`, Remotion's `<Img>`).
pub fn image_mobject(path: &str, width: f64) -> Result<Mobject> {
    let pm = crate::render::load_image(path)?;
    Ok(Mobject::image(Arc::new(pm), width))
}

/// Image from an in-memory pixmap.
pub fn pixmap_mobject(pm: Pixmap, width: f64) -> Mobject {
    Mobject::image(Arc::new(pm), width)
}

// --------------------------------------------------------------------- GIF

/// Decoded animated GIF.
#[derive(Clone)]
pub struct Gif {
    pub frames: Vec<Arc<Pixmap>>,
    /// Delay of each frame in seconds.
    pub delays: Vec<f64>,
}

impl Gif {
    pub fn open(path: &str) -> Result<Gif> {
        use image::AnimationDecoder;
        let f = std::io::BufReader::new(std::fs::File::open(path)?);
        let dec = image::codecs::gif::GifDecoder::new(f)?;
        let mut frames = Vec::new();
        let mut delays = Vec::new();
        for fr in dec.into_frames() {
            let fr = fr?;
            let (n, d) = fr.delay().numer_denom_ms();
            delays.push((n as f64 / d.max(1) as f64 / 1000.0).max(0.02));
            let buf = fr.into_buffer();
            let (w, h) = buf.dimensions();
            let mut data = buf.into_raw();
            premultiply(&mut data);
            frames.push(Arc::new(
                Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(w, h).context("size")?)
                    .context("pixmap")?,
            ));
        }
        if frames.is_empty() {
            bail!("gif has no frames");
        }
        Ok(Gif { frames, delays })
    }
    pub fn duration(&self) -> f64 {
        self.delays.iter().sum()
    }
    /// Frame index at time `t` (looping).
    pub fn frame_at(&self, t: f64) -> usize {
        let total = self.duration().max(1e-9);
        let mut x = t.rem_euclid(total);
        for (i, d) in self.delays.iter().enumerate() {
            if x < *d {
                return i;
            }
            x -= d;
        }
        self.frames.len() - 1
    }
}

// ------------------------------------------------------------------- video

/// A video file decoded once (via ffmpeg) into a frame cache for random access.
pub struct VideoSource {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub frames: usize,
    file: std::fs::File,
    path: std::path::PathBuf,
}

impl Drop for VideoSource {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl VideoSource {
    /// Decode `path` scaled to `max_width` pixels wide (keeps aspect).
    pub fn open(path: &str, max_width: u32) -> Result<VideoSource> {
        let probe = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height,r_frame_rate",
                "-of",
                "csv=p=0",
                path,
            ])
            .output()
            .context("ffprobe is required for video assets")?;
        let s = String::from_utf8_lossy(&probe.stdout);
        let parts: Vec<&str> = s.trim().split(',').collect();
        if parts.len() < 3 {
            bail!("could not probe video {path}");
        }
        let (w0, h0): (u32, u32) = (parts[0].parse()?, parts[1].parse()?);
        let fps = match parts[2].split_once('/') {
            Some((a, b)) => a.parse::<f64>()? / b.parse::<f64>()?.max(1.0),
            None => parts[2].parse()?,
        };
        let w = w0.min(max_width.max(16)) & !1;
        let h = ((h0 as f64 * w as f64 / w0 as f64).round() as u32).max(2) & !1;
        let cache = std::env::temp_dir().join(format!(
            "runwee-video-{}-{}.raw",
            std::process::id(),
            crate::math::Rng::new(w as u64 ^ (fps as u64)).next_u64()
        ));
        let st = std::process::Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i", path])
            .args([
                "-vf",
                &format!("scale={w}:{h}"),
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
            ])
            .arg(&cache)
            .status()?;
        if !st.success() {
            bail!("ffmpeg failed to decode {path}");
        }
        let file = std::fs::File::open(&cache)?;
        let len = file.metadata()?.len() as usize;
        let frames = len / (w as usize * h as usize * 4);
        Ok(VideoSource {
            width: w,
            height: h,
            fps,
            frames,
            file,
            path: cache,
        })
    }
    pub fn duration(&self) -> f64 {
        self.frames as f64 / self.fps
    }
    /// Frame at time `t` (clamped).
    pub fn frame(&self, t: f64) -> Option<Pixmap> {
        if self.frames == 0 {
            return None;
        }
        let i = ((t * self.fps).floor().max(0.0) as usize).min(self.frames - 1);
        let size = self.width as usize * self.height as usize * 4;
        let mut buf = vec![0u8; size];
        read_at(&self.file, &mut buf, (i * size) as u64).ok()?;
        Pixmap::from_vec(buf, tiny_skia::IntSize::from_wh(self.width, self.height)?)
    }
}

#[cfg(unix)]
fn read_at(f: &std::fs::File, buf: &mut [u8], off: u64) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    f.read_exact_at(buf, off)
}

#[cfg(windows)]
fn read_at(f: &std::fs::File, buf: &mut [u8], off: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    let mut done = 0;
    while done < buf.len() {
        let n = f.seek_read(&mut buf[done..], off + done as u64)?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        done += n;
    }
    Ok(())
}

impl Scene {
    /// Place an animated GIF (`width` units wide at `at`), looping from now.
    pub fn add_gif(&mut self, path: &str, width: f64, at: Vec2) -> Result<Obj> {
        let gif = Arc::new(Gif::open(path)?);
        Ok(self.always_redraw(move |ctx| {
            let i = gif.frame_at(ctx.elapsed);
            Mobject::image(gif.frames[i].clone(), width).move_to(at)
        }))
    }
    /// Place a video clip playing from now (`<OffthreadVideo>`), optionally with its audio.
    pub fn add_video(
        &mut self,
        path: &str,
        width: f64,
        at: Vec2,
        with_audio: bool,
        volume: f32,
    ) -> Result<Obj> {
        let src = Arc::new(VideoSource::open(path, 1920)?);
        if with_audio {
            if let Ok(a) = crate::audio::AudioData::load(path) {
                self.add_sound(Arc::new(a), volume);
            }
        }
        let dur = src.duration();
        Ok(
            self.always_redraw(move |ctx| match src.frame(ctx.elapsed.min(dur)) {
                Some(pm) => Mobject::image(Arc::new(pm), width).move_to(at),
                None => Mobject::empty(),
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_import() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50">
            <defs><linearGradient id="g"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs>
            <rect id="box" x="0" y="0" width="100" height="50" fill="url(#g)"/>
            <circle cx="25" cy="25" r="10" fill="none" stroke="#0f0" stroke-width="2"/>
            <path d="M60 10 L90 40" stroke="black"/>
        </svg>"##;
        let m = svg_from_str(svg, 2.0).unwrap();
        assert!((m.height() - 2.0).abs() < 0.05);
        assert!((m.width() - 4.0).abs() < 0.1);
        assert_eq!(m.leaf_count(), 3);
        assert_eq!(m.children[0].tag(), Some("box"));
        assert!(matches!(m.children[0].style.fill, Paint::Linear { .. }));
        // y flipped: the circle (top-left in SVG) is in the upper-left quadrant
        assert!(m.children[1].center().x < 0.0 && m.children[1].center().y > -0.1);
    }

    #[test]
    fn gif_and_image_roundtrip() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("runwee-img-{}.png", std::process::id()));
        let mut pm = Pixmap::new(8, 4).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        crate::render::save_png(&pm, &p).unwrap();
        let m = image_mobject(p.to_str().unwrap(), 2.0).unwrap();
        assert!((m.width() - 2.0).abs() < 1e-9 && (m.height() - 1.0).abs() < 1e-9);
        std::fs::remove_file(p).ok();
    }
}
