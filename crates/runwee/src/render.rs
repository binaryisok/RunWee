//! Rasteriser: draws Mobjects onto a tiny-skia pixmap through a camera.

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::Rect;
use crate::style::*;
use tiny_skia as sk;

/// Output resolution & frame rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Quality::TwoK.config()
    }
}

/// Quality presets (manim's -ql/-qm/-qh/-qp/-qk plus 2K).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    /// 854×480 @ 15 fps — fast previews.
    Low,
    /// 1280×720 @ 30 fps.
    Medium,
    /// 1920×1080 @ 60 fps.
    High,
    /// 2560×1440 @ 60 fps (default).
    TwoK,
    /// 3840×2160 @ 60 fps.
    FourK,
}

impl Quality {
    pub fn config(self) -> RenderConfig {
        let (width, height, fps) = match self {
            Quality::Low => (854, 480, 15.0),
            Quality::Medium => (1280, 720, 30.0),
            Quality::High => (1920, 1080, 60.0),
            Quality::TwoK => (2560, 1440, 60.0),
            Quality::FourK => (3840, 2160, 60.0),
        };
        RenderConfig { width, height, fps }
    }
    pub fn parse(s: &str) -> Option<Quality> {
        Some(match s.to_ascii_lowercase().as_str() {
            "l" | "low" | "480p" | "ql" => Quality::Low,
            "m" | "medium" | "720p" | "qm" => Quality::Medium,
            "h" | "high" | "1080p" | "qh" | "fhd" => Quality::High,
            "2k" | "1440p" | "qhd" | "qp" => Quality::TwoK,
            "4k" | "2160p" | "uhd" | "qk" => Quality::FourK,
            _ => return None,
        })
    }
}

impl RenderConfig {
    pub fn new(width: u32, height: u32, fps: f64) -> Self {
        Self { width, height, fps }
    }
    /// Same quality but portrait (e.g. 9:16 shorts / reels).
    pub fn portrait(self) -> Self {
        Self {
            width: self.height,
            height: self.width,
            fps: self.fps,
        }
    }
    pub fn aspect(&self) -> f64 {
        self.width as f64 / self.height as f64
    }
    pub fn frame_count(&self, duration: f64) -> usize {
        (duration * self.fps).round().max(1.0) as usize
    }
}

/// What part of the scene is visible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub center: Vec2,
    /// Visible height in scene units (8 by default — manim's frame).
    pub frame_height: f64,
    pub rotation: f64,
    pub background: Color,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            frame_height: 8.0,
            rotation: 0.0,
            background: BACKGROUND,
        }
    }
}

impl Camera {
    pub fn lerp(&self, o: &Camera, t: f64) -> Camera {
        // Interpolate zoom geometrically for a natural feel.
        let fh = (self.frame_height.ln() * (1.0 - t) + o.frame_height.ln() * t).exp();
        Camera {
            center: self.center.lerp(o.center, t),
            frame_height: fh,
            rotation: lerp(self.rotation, o.rotation, t),
            background: self.background.mix(o.background, t as f32),
        }
    }
    /// Scene → pixel transform.
    pub fn to_pixels(&self, cfg: &RenderConfig) -> Affine {
        let s = cfg.height as f64 / self.frame_height;
        Affine::translate(v2(cfg.width as f64 / 2.0, cfg.height as f64 / 2.0))
            .then_after(Affine::scale(s, -s))
            .then_after(Affine::rotate(-self.rotation))
            .then_after(Affine::translate(-self.center))
    }
    /// Pixels per scene unit.
    pub fn pixel_scale(&self, cfg: &RenderConfig) -> f64 {
        cfg.height as f64 / self.frame_height
    }
    /// Visible region in scene units (ignoring rotation).
    pub fn visible_rect(&self, cfg: &RenderConfig) -> Rect {
        let h = self.frame_height;
        let w = h * cfg.aspect();
        Rect::from_center(self.center, w, h)
    }
}

/// Drawing context for one frame.
pub struct Canvas<'a> {
    pub pixmap: &'a mut sk::Pixmap,
    pub cfg: RenderConfig,
    pub camera: Camera,
    xf: Affine,
    stroke_scale: f64,
}

impl<'a> Canvas<'a> {
    pub fn new(pixmap: &'a mut sk::Pixmap, cfg: RenderConfig, camera: Camera) -> Self {
        let xf = camera.to_pixels(&cfg);
        // Stroke widths are "pixels at 1080p for the default 8-unit frame".
        let stroke_scale = cfg.height as f64 / 1080.0 * (8.0 / camera.frame_height);
        Self {
            pixmap,
            cfg,
            camera,
            xf,
            stroke_scale,
        }
    }

    pub fn clear(&mut self) {
        self.pixmap.fill(self.camera.background.to_skia());
    }

    pub fn transform(&self) -> Affine {
        self.xf
    }

    /// Draw a mobject and its whole family.
    pub fn draw(&mut self, m: &Mobject) {
        self.draw_with_opacity(m, 1.0);
    }

    pub fn draw_with_opacity(&mut self, m: &Mobject, opacity: f32) {
        if m.is_leaf_drawable() {
            self.draw_node(m, opacity);
        }
        for c in &m.children {
            self.draw_with_opacity(c, opacity);
        }
    }

    fn build_path(&self, m: &Mobject, offset: Vec2) -> Option<sk::Path> {
        let mut pb = sk::PathBuilder::new();
        let xf = Affine::translate(offset).then_after(self.xf);
        for sp in &m.path.subpaths {
            if !sp.start.is_finite()
                || sp
                    .segs
                    .iter()
                    .any(|c| !(c.c1.is_finite() && c.c2.is_finite() && c.p.is_finite()))
            {
                continue;
            }
            let p = xf.apply(sp.start);
            pb.move_to(p.x as f32, p.y as f32);
            if sp.segs.is_empty() {
                // a lone point: make a zero-length line so round caps draw a dot
                pb.line_to(p.x as f32 + 0.001, p.y as f32);
                continue;
            }
            for c in &sp.segs {
                let c1 = xf.apply(c.c1);
                let c2 = xf.apply(c.c2);
                let e = xf.apply(c.p);
                pb.cubic_to(
                    c1.x as f32,
                    c1.y as f32,
                    c2.x as f32,
                    c2.y as f32,
                    e.x as f32,
                    e.y as f32,
                );
            }
            if sp.closed {
                pb.close();
            }
        }
        pb.finish()
    }

    fn shader(&self, p: &Paint, alpha: f32) -> sk::Shader<'static> {
        let stops = |st: &Vec<(f32, Color)>| -> Vec<sk::GradientStop> {
            st.iter()
                .map(|(pos, c)| sk::GradientStop::new(pos.clamp(0.0, 1.0), c.fade(alpha).to_skia()))
                .collect()
        };
        match p {
            Paint::Solid(c) => sk::Shader::SolidColor(c.fade(alpha).to_skia()),
            Paint::Linear {
                start,
                end,
                stops: st,
            } => {
                let a = self.xf.apply(*start);
                let b = self.xf.apply(*end);
                sk::LinearGradient::new(
                    sk::Point::from_xy(a.x as f32, a.y as f32),
                    sk::Point::from_xy(b.x as f32, b.y as f32),
                    stops(st),
                    sk::SpreadMode::Pad,
                    sk::Transform::identity(),
                )
                .unwrap_or(sk::Shader::SolidColor(p.color().fade(alpha).to_skia()))
            }
            Paint::Radial {
                center,
                radius,
                focal,
                stops: st,
            } => {
                let c = self.xf.apply(*center);
                let f = self.xf.apply(*focal);
                let r = (*radius * self.camera.pixel_scale(&self.cfg)) as f32;
                sk::RadialGradient::new(
                    sk::Point::from_xy(f.x as f32, f.y as f32),
                    0.0,
                    sk::Point::from_xy(c.x as f32, c.y as f32),
                    r.max(0.01),
                    stops(st),
                    sk::SpreadMode::Pad,
                    sk::Transform::identity(),
                )
                .unwrap_or(sk::Shader::SolidColor(p.color().fade(alpha).to_skia()))
            }
        }
    }

    fn sk_stroke(&self, st: &Style, width_px: f32) -> sk::Stroke {
        let mut s = sk::Stroke {
            width: width_px,
            line_cap: match st.line_cap {
                LineCap::Butt => sk::LineCap::Butt,
                LineCap::Round => sk::LineCap::Round,
                LineCap::Square => sk::LineCap::Square,
            },
            line_join: match st.line_join {
                LineJoin::Miter => sk::LineJoin::Miter,
                LineJoin::Round => sk::LineJoin::Round,
                LineJoin::Bevel => sk::LineJoin::Bevel,
            },
            ..Default::default()
        };
        if let Some((d, g)) = st.dash {
            let k = self.camera.pixel_scale(&self.cfg);
            let (d, g) = ((d * k) as f32, (g * k) as f32);
            if d > 0.0 && g > 0.0 {
                s.dash = sk::StrokeDash::new(vec![d, g], (st.dash_offset * k) as f32);
            }
        }
        s
    }

    fn blend(b: Blend) -> sk::BlendMode {
        match b {
            Blend::Normal => sk::BlendMode::SourceOver,
            Blend::Add => sk::BlendMode::Plus,
            Blend::Screen => sk::BlendMode::Screen,
            Blend::Multiply => sk::BlendMode::Multiply,
            Blend::Overlay => sk::BlendMode::Overlay,
        }
    }

    fn pixel_bounds(&self, m: &Mobject) -> Option<Rect> {
        let b = m.path.bounds()?;
        let xs = [b.min, b.max, v2(b.min.x, b.max.y), v2(b.max.x, b.min.y)];
        let mut r = Rect::point(self.xf.apply(xs[0]));
        for p in &xs[1..] {
            r.include(self.xf.apply(*p));
        }
        Some(r)
    }

    fn draw_node(&mut self, m: &Mobject, parent_opacity: f32) {
        let st = &m.style;
        let opacity = st.opacity * parent_opacity;
        if let Some(img) = &m.image {
            self.draw_image(img, opacity, st.blend);
        }
        if m.path.subpaths.is_empty() || opacity <= 0.0 {
            return;
        }
        let stroke_px = (st.stroke_width as f64 * self.stroke_scale) as f32;
        let bg_px = (st.background_stroke_width as f64 * self.stroke_scale) as f32;
        let glow_px = st.glow * self.camera.pixel_scale(&self.cfg);
        // Cull objects entirely outside the frame.
        let Some(pb) = self.pixel_bounds(m) else {
            return;
        };
        let margin = stroke_px.max(bg_px) as f64 + glow_px * 3.0 + 4.0;
        let screen = Rect::new(
            v2(-margin, -margin),
            v2(
                self.cfg.width as f64 + margin,
                self.cfg.height as f64 + margin,
            ),
        );
        if !screen.intersects(&pb) {
            return;
        }
        let Some(path) = self.build_path(m, Vec2::ZERO) else {
            return;
        };
        let fill_rule = match st.fill_rule {
            FillRule::NonZero => sk::FillRule::Winding,
            FillRule::EvenOdd => sk::FillRule::EvenOdd,
        };
        let has_fill = st.fill_opacity > 0.0;
        let has_stroke = st.stroke_opacity > 0.0 && stroke_px > 0.0;

        if let Some(sh) = st.shadow {
            self.draw_effect_layer(
                m,
                &pb,
                sh.offset,
                sh.blur,
                sh.color.fade(opacity),
                stroke_px,
                has_fill,
                has_stroke,
                sk::BlendMode::SourceOver,
                1.0,
            );
        }
        if st.glow > 0.0 {
            let gc = st.glow_color.unwrap_or(if has_stroke {
                st.stroke.color()
            } else {
                st.fill.color()
            });
            let strength = st.glow_strength * opacity;
            self.draw_effect_layer(
                m,
                &pb,
                Vec2::ZERO,
                st.glow,
                gc,
                stroke_px * 1.5,
                has_fill,
                true,
                sk::BlendMode::Plus,
                strength,
            );
            self.draw_effect_layer(
                m,
                &pb,
                Vec2::ZERO,
                st.glow * 0.35,
                gc,
                stroke_px * 1.2,
                has_fill,
                true,
                sk::BlendMode::Plus,
                strength * 0.8,
            );
        }
        let blend = Self::blend(st.blend);
        if bg_px > 0.0 {
            let paint = sk::Paint {
                shader: sk::Shader::SolidColor(st.background_stroke.fade(opacity).to_skia()),
                anti_alias: true,
                blend_mode: blend,
                ..Default::default()
            };
            let stroke = self.sk_stroke(st, bg_px);
            self.pixmap
                .stroke_path(&path, &paint, &stroke, sk::Transform::identity(), None);
        }
        if has_fill {
            let paint = sk::Paint {
                shader: self.shader(&st.fill, st.fill_opacity * opacity),
                anti_alias: true,
                blend_mode: blend,
                ..Default::default()
            };
            self.pixmap
                .fill_path(&path, &paint, fill_rule, sk::Transform::identity(), None);
        }
        if has_stroke {
            let paint = sk::Paint {
                shader: self.shader(&st.stroke, st.stroke_opacity * opacity),
                anti_alias: true,
                blend_mode: blend,
                ..Default::default()
            };
            let stroke = self.sk_stroke(st, stroke_px);
            self.pixmap
                .stroke_path(&path, &paint, &stroke, sk::Transform::identity(), None);
        }
    }

    /// Render the silhouette of `m` in `color` into an offscreen layer, blur it and composite.
    fn draw_effect_layer(
        &mut self,
        m: &Mobject,
        pb: &Rect,
        offset: Vec2,
        blur: f64,
        color: Color,
        stroke_px: f32,
        fill: bool,
        stroke: bool,
        blend: sk::BlendMode,
        strength: f32,
    ) {
        let k = self.camera.pixel_scale(&self.cfg);
        let blur_px = (blur * k).max(0.5);
        let pad = (blur_px * 3.0 + stroke_px as f64 + 2.0).ceil();
        let off_px = self.xf.apply_vector(offset);
        let x0 = (pb.min.x + off_px.x - pad).floor().max(0.0);
        let y0 = (pb.min.y + off_px.y - pad).floor().max(0.0);
        let x1 = (pb.max.x + off_px.x + pad)
            .ceil()
            .min(self.cfg.width as f64);
        let y1 = (pb.max.y + off_px.y + pad)
            .ceil()
            .min(self.cfg.height as f64);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
        let Some(mut layer) = sk::Pixmap::new(w, h) else {
            return;
        };
        // Path in layer coordinates: pixel transform followed by a shift.
        let Some(path) = self.build_path(m, offset) else {
            return;
        };
        let tr = sk::Transform::from_translate(-x0 as f32, -y0 as f32);
        let paint = sk::Paint {
            shader: sk::Shader::SolidColor(color.to_skia()),
            anti_alias: true,
            ..Default::default()
        };
        if fill {
            layer.fill_path(&path, &paint, sk::FillRule::Winding, tr, None);
        }
        if stroke && stroke_px > 0.0 {
            let s = sk::Stroke {
                width: stroke_px,
                line_cap: sk::LineCap::Round,
                line_join: sk::LineJoin::Round,
                ..Default::default()
            };
            layer.stroke_path(&path, &paint, &s, tr, None);
        }
        crate::effects::blur(&mut layer, blur_px as f32);
        let pp = sk::PixmapPaint {
            opacity: strength.clamp(0.0, 1.0),
            blend_mode: blend,
            quality: sk::FilterQuality::Nearest,
        };
        self.pixmap.draw_pixmap(
            x0 as i32,
            y0 as i32,
            layer.as_ref(),
            &pp,
            sk::Transform::identity(),
            None,
        );
    }

    fn draw_image(&mut self, img: &ImageData, opacity: f32, blend: Blend) {
        let o = opacity * img.opacity;
        if o <= 0.0 {
            return;
        }
        let (w, h) = (img.pixmap.width() as f64, img.pixmap.height() as f64);
        // image pixel (u, v) → scene: origin + x_axis*u/w + y_axis*(1 - v/h)
        let to_scene = Affine {
            a: img.x_axis.x / w,
            b: img.x_axis.y / w,
            c: -img.y_axis.x / h,
            d: -img.y_axis.y / h,
            e: img.origin.x + img.y_axis.x,
            f: img.origin.y + img.y_axis.y,
        };
        let t = self.xf.then_after(to_scene);
        let tr = sk::Transform::from_row(
            t.a as f32, t.b as f32, t.c as f32, t.d as f32, t.e as f32, t.f as f32,
        );
        let pp = sk::PixmapPaint {
            opacity: o.clamp(0.0, 1.0),
            blend_mode: Self::blend(blend),
            quality: sk::FilterQuality::Bicubic,
        };
        self.pixmap
            .draw_pixmap(0, 0, img.pixmap.as_ref().as_ref(), &pp, tr, None);
    }
}

/// Render a list of mobjects to a new pixmap.
pub fn render_mobjects(mobjects: &[&Mobject], cfg: RenderConfig, camera: Camera) -> sk::Pixmap {
    let mut pm = sk::Pixmap::new(cfg.width, cfg.height).expect("invalid size");
    {
        let mut c = Canvas::new(&mut pm, cfg, camera);
        c.clear();
        for m in mobjects {
            c.draw(m);
        }
    }
    pm
}

/// Save a pixmap as PNG.
pub fn save_png(pm: &sk::Pixmap, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
    let data = pm.encode_png()?;
    if let Some(dir) = path.as_ref().parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    std::fs::write(path, data)?;
    Ok(())
}

/// Load an image file (PNG/JPEG) as a premultiplied pixmap.
pub fn load_image(path: impl AsRef<std::path::Path>) -> anyhow::Result<sk::Pixmap> {
    let img = image::open(path.as_ref())?.to_rgba8();
    let (w, h) = img.dimensions();
    let mut data = img.into_raw();
    for px in data.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
    sk::Pixmap::from_vec(data, sk::IntSize::from_wh(w, h).unwrap())
        .ok_or_else(|| anyhow::anyhow!("invalid image size"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::*;

    #[test]
    fn camera_maps_origin_to_centre() {
        let cfg = RenderConfig::new(1920, 1080, 30.0);
        let cam = Camera::default();
        let p = cam.to_pixels(&cfg).apply(Vec2::ZERO);
        assert!((p - v2(960.0, 540.0)).length() < 1e-9);
        let q = cam.to_pixels(&cfg).apply(v2(0.0, 4.0));
        assert!(q.y.abs() < 1e-9);
    }

    #[test]
    fn draws_pixels() {
        let cfg = RenderConfig::new(320, 180, 30.0);
        let c = circle(1.0)
            .fill(RED, 1.0)
            .stroke(WHITE, 4.0)
            .glow(0.2, None);
        let pm = render_mobjects(&[&c], cfg, Camera::default());
        let px = pm.pixel(160, 90).unwrap();
        assert!(px.red() > 200 && px.green() < 150);
        let bg = pm.pixel(5, 5).unwrap();
        assert!(bg.red() < 30);
    }
}
