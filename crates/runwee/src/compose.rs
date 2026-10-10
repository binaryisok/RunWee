//! Remotion-style composition: `Movie` (a `<Series>` / `<TransitionSeries>` of
//! scenes with transitions), `Layers` (`<AbsoluteFill>` stacking), `Sequence`
//! (time offset/trim), `Loop`, `Freeze`, `Speed`, `Still`, functional
//! compositions (frame = f(t)), motion blur / trails, and post effects.

use crate::audio::AudioTrack;
use crate::captions::Cue;
use crate::color::*;
use crate::effects::Effect;
use crate::mobject::Mobject;
use crate::output::Composition;
use crate::render::{Camera, Canvas, RenderConfig};
use crate::transition::{composite, Transition};
use std::sync::Arc;
use tiny_skia::Pixmap;

fn offset_cues(mut cues: Vec<Cue>, dt: f64) -> Vec<Cue> {
    for c in &mut cues {
        c.start += dt;
        c.end += dt;
        for w in &mut c.words {
            w.start += dt;
            w.end += dt;
        }
    }
    cues
}

type Comp = Arc<dyn Composition>;

/// A sequence of compositions played one after another, optionally with
/// transitions that overlap neighbours (Remotion's `<TransitionSeries>`).
#[derive(Default, Clone)]
pub struct Movie {
    clips: Vec<Comp>,
    /// `transitions[i]` sits between clip `i` and `i + 1`.
    transitions: Vec<Option<Transition>>,
}

impl Movie {
    pub fn new() -> Self {
        Self::default()
    }
    /// Append a composition.
    pub fn then(mut self, c: impl Composition + 'static) -> Self {
        if !self.clips.is_empty() && self.transitions.len() < self.clips.len() {
            self.transitions.push(None);
        }
        self.clips.push(Arc::new(c));
        self
    }
    pub fn then_arc(mut self, c: Comp) -> Self {
        if !self.clips.is_empty() && self.transitions.len() < self.clips.len() {
            self.transitions.push(None);
        }
        self.clips.push(c);
        self
    }
    /// Transition between the last added clip and the next one.
    pub fn transition(mut self, t: Transition) -> Self {
        if self.transitions.len() < self.clips.len() {
            self.transitions.push(Some(t));
        }
        self
    }
    /// Start time of each clip.
    pub fn starts(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.clips.len());
        let mut t = 0.0;
        for (i, c) in self.clips.iter().enumerate() {
            out.push(t);
            let overlap = self
                .transitions
                .get(i)
                .and_then(|x| x.as_ref())
                .map_or(0.0, |x| x.duration.min(c.duration()));
            t += c.duration() - overlap;
        }
        out
    }
}

impl Composition for Movie {
    fn duration(&self) -> f64 {
        let s = self.starts();
        self.clips
            .iter()
            .zip(&s)
            .map(|(c, st)| st + c.duration())
            .fold(0.0, f64::max)
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        let starts = self.starts();
        // find the clip whose window contains t (last one wins), and a transition partner
        let mut idx = 0;
        for (i, s) in starts.iter().enumerate() {
            if t >= *s {
                idx = i;
            }
        }
        if self.clips.is_empty() {
            pm.fill(tiny_skia::Color::BLACK);
            return;
        }
        // are we inside the transition into clip idx?
        if idx > 0 {
            if let Some(Some(tr)) = self.transitions.get(idx - 1) {
                let into = t - starts[idx];
                if into < tr.duration {
                    let p = tr.rate.eval(into / tr.duration.max(1e-9)) as f32;
                    let mut a = Pixmap::new(cfg.width, cfg.height).unwrap();
                    let mut b = Pixmap::new(cfg.width, cfg.height).unwrap();
                    let prev = &self.clips[idx - 1];
                    prev.render_frame(
                        (t - starts[idx - 1]).min(prev.duration() - 1e-6),
                        &mut a,
                        cfg,
                    );
                    self.clips[idx].render_frame(into, &mut b, cfg);
                    composite(tr.presentation, &a, &b, p, pm);
                    return;
                }
            }
        }
        let c = &self.clips[idx];
        c.render_frame((t - starts[idx]).min(c.duration() - 1e-6).max(0.0), pm, cfg);
    }
    fn audio_track(&self) -> AudioTrack {
        let mut out = AudioTrack::default();
        for (c, s) in self.clips.iter().zip(self.starts()) {
            let mut tr = c.audio_track();
            tr.offset(s);
            for clip in tr.clips {
                out.add(clip);
            }
        }
        out
    }
    fn caption_cues(&self) -> Vec<Cue> {
        self.clips
            .iter()
            .zip(self.starts())
            .flat_map(|(c, s)| offset_cues(c.caption_cues(), s))
            .collect()
    }
}

/// Stack compositions on top of each other (`<AbsoluteFill>` layers). The first
/// layer provides the background.
#[derive(Default, Clone)]
pub struct Layers {
    layers: Vec<(Comp, f64)>,
}

impl Layers {
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a layer starting at `from` seconds (Remotion `<Sequence from>`).
    pub fn layer(mut self, c: impl Composition + 'static, from: f64) -> Self {
        self.layers.push((Arc::new(c), from));
        self
    }
}

impl Composition for Layers {
    fn duration(&self) -> f64 {
        self.layers
            .iter()
            .map(|(c, f)| f + c.duration())
            .fold(0.0, f64::max)
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        pm.fill(tiny_skia::Color::BLACK);
        for (i, (c, from)) in self.layers.iter().enumerate() {
            let lt = t - from;
            if lt < 0.0 || lt >= c.duration() {
                continue;
            }
            if i == 0 {
                c.render_frame(lt, pm, cfg);
            } else {
                c.render_layer(lt, pm, cfg);
            }
        }
    }
    fn audio_track(&self) -> AudioTrack {
        let mut out = AudioTrack::default();
        for (c, from) in &self.layers {
            let mut tr = c.audio_track();
            tr.offset(*from);
            for clip in tr.clips {
                out.add(clip);
            }
        }
        out
    }
    fn caption_cues(&self) -> Vec<Cue> {
        self.layers
            .iter()
            .flat_map(|(c, f)| offset_cues(c.caption_cues(), *f))
            .collect()
    }
}

/// Play a window `[trim_start, trim_start + duration)` of a composition (`<Sequence>`
/// with `durationInFrames` / `trimBefore`).
pub struct Sequence {
    pub inner: Comp,
    pub trim_start: f64,
    pub duration: f64,
}

impl Sequence {
    pub fn new(c: impl Composition + 'static, trim_start: f64, duration: f64) -> Self {
        Self {
            inner: Arc::new(c),
            trim_start,
            duration,
        }
    }
}

impl Composition for Sequence {
    fn duration(&self) -> f64 {
        self.duration
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.inner.render_frame(t + self.trim_start, pm, cfg)
    }
    fn render_layer(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.inner.render_layer(t + self.trim_start, pm, cfg)
    }
    fn audio_track(&self) -> AudioTrack {
        let mut tr = self.inner.audio_track();
        tr.offset(-self.trim_start);
        tr
    }
    fn caption_cues(&self) -> Vec<Cue> {
        offset_cues(self.inner.caption_cues(), -self.trim_start)
            .into_iter()
            .filter(|c| c.end > 0.0 && c.start < self.duration)
            .collect()
    }
}

/// Repeat a composition `times` times (`<Loop>`).
pub struct Loop {
    pub inner: Comp,
    pub times: usize,
}

impl Loop {
    pub fn new(c: impl Composition + 'static, times: usize) -> Self {
        Self {
            inner: Arc::new(c),
            times: times.max(1),
        }
    }
}

impl Composition for Loop {
    fn duration(&self) -> f64 {
        self.inner.duration() * self.times as f64
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.inner.render_frame(t % self.inner.duration(), pm, cfg)
    }
    fn render_layer(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.inner.render_layer(t % self.inner.duration(), pm, cfg)
    }
    fn audio_track(&self) -> AudioTrack {
        let mut out = AudioTrack::default();
        for k in 0..self.times {
            let mut tr = self.inner.audio_track();
            tr.offset(k as f64 * self.inner.duration());
            for c in tr.clips {
                out.add(c);
            }
        }
        out
    }
}

/// Hold a single frame of a composition for `duration` seconds (`<Freeze>`).
pub struct Freeze {
    pub inner: Comp,
    pub at: f64,
    pub duration: f64,
}

impl Freeze {
    pub fn new(c: impl Composition + 'static, at: f64, duration: f64) -> Self {
        Self {
            inner: Arc::new(c),
            at,
            duration,
        }
    }
}

impl Composition for Freeze {
    fn duration(&self) -> f64 {
        self.duration
    }
    fn render_frame(&self, _t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.inner.render_frame(self.at, pm, cfg)
    }
}

/// Change playback speed (manim's `ChangeSpeed`, Remotion's `playbackRate`).
pub struct Speed {
    pub inner: Comp,
    pub rate: f64,
}

impl Speed {
    pub fn new(c: impl Composition + 'static, rate: f64) -> Self {
        Self {
            inner: Arc::new(c),
            rate: rate.max(1e-3),
        }
    }
}

impl Composition for Speed {
    fn duration(&self) -> f64 {
        self.inner.duration() / self.rate
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.inner.render_frame(t * self.rate, pm, cfg)
    }
    fn audio_track(&self) -> AudioTrack {
        let mut tr = self.inner.audio_track();
        for c in &mut tr.clips {
            c.start /= self.rate;
            c.rate *= self.rate;
        }
        tr
    }
}

/// A single static image held for a duration (`<Still>`).
pub struct Still {
    pub mobjects: Vec<Mobject>,
    pub duration: f64,
    pub background: Color,
}

impl Still {
    pub fn new(m: Mobject, duration: f64) -> Self {
        Self {
            mobjects: vec![m],
            duration,
            background: BACKGROUND,
        }
    }
}

impl Composition for Still {
    fn duration(&self) -> f64 {
        self.duration
    }
    fn render_frame(&self, _t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        let cam = Camera {
            background: self.background,
            ..Camera::default()
        };
        let mut c = Canvas::new(pm, cfg, cam);
        c.clear();
        for m in &self.mobjects {
            c.draw(m);
        }
    }
}

/// Functional composition: every frame is built from scratch by a closure,
/// exactly like a Remotion component reading `useCurrentFrame()`.
pub struct FnComposition {
    pub duration: f64,
    pub background: Color,
    pub fps_hint: f64,
    pub f: Arc<dyn Fn(&FrameInfo) -> Vec<Mobject> + Send + Sync>,
}

/// What a functional composition sees each frame.
#[derive(Clone, Copy, Debug)]
pub struct FrameInfo {
    /// Seconds since start.
    pub t: f64,
    /// Frame number at `fps`.
    pub frame: f64,
    pub fps: f64,
    pub duration: f64,
}

impl FnComposition {
    pub fn new(
        duration: f64,
        f: impl Fn(&FrameInfo) -> Vec<Mobject> + Send + Sync + 'static,
    ) -> Self {
        Self {
            duration,
            background: BACKGROUND,
            fps_hint: 30.0,
            f: Arc::new(f),
        }
    }
    pub fn background(mut self, c: Color) -> Self {
        self.background = c;
        self
    }
}

impl Composition for FnComposition {
    fn duration(&self) -> f64 {
        self.duration
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        let cam = Camera {
            background: self.background,
            ..Camera::default()
        };
        let info = FrameInfo {
            t,
            frame: t * cfg.fps,
            fps: cfg.fps,
            duration: self.duration,
        };
        let mobs = (self.f)(&info);
        let mut c = Canvas::new(pm, cfg, cam);
        c.clear();
        for m in &mobs {
            c.draw(m);
        }
    }
    fn render_layer(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        let info = FrameInfo {
            t,
            frame: t * cfg.fps,
            fps: cfg.fps,
            duration: self.duration,
        };
        let mobs = (self.f)(&info);
        let mut c = Canvas::new(pm, cfg, Camera::default());
        for m in &mobs {
            c.draw(m);
        }
    }
}

/// Motion blur by temporal supersampling (`@remotion/motion-blur` `CameraMotionBlur`),
/// or a decaying trail when `trail` is set (`<Trail>`).
pub struct MotionBlur {
    pub inner: Comp,
    pub samples: usize,
    /// Fraction of a frame the shutter stays open (0..1+).
    pub shutter: f64,
    pub trail: bool,
}

impl MotionBlur {
    pub fn new(c: impl Composition + 'static, samples: usize, shutter: f64) -> Self {
        Self {
            inner: Arc::new(c),
            samples: samples.max(1),
            shutter,
            trail: false,
        }
    }
    /// Trail: earlier samples are drawn with decreasing opacity over a longer window.
    pub fn trail(c: impl Composition + 'static, layers: usize, lag_frames: f64) -> Self {
        Self {
            inner: Arc::new(c),
            samples: layers.max(1),
            shutter: lag_frames,
            trail: true,
        }
    }
}

impl Composition for MotionBlur {
    fn duration(&self) -> f64 {
        self.inner.duration()
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        let n = self.samples;
        let dt = self.shutter / cfg.fps;
        let mut acc = vec![0.0f32; (cfg.width * cfg.height * 4) as usize];
        let mut tmp = Pixmap::new(cfg.width, cfg.height).unwrap();
        let mut wsum = 0.0f32;
        for i in 0..n {
            let k = if n > 1 {
                i as f64 / (n - 1) as f64
            } else {
                0.0
            };
            let st = (t - dt * k).max(0.0);
            let w = if self.trail {
                (1.0 - k as f32 * 0.85).max(0.05)
            } else {
                1.0
            };
            self.inner.render_frame(st, &mut tmp, cfg);
            for (a, v) in acc.iter_mut().zip(tmp.data()) {
                *a += *v as f32 * w;
            }
            wsum += w;
        }
        for (o, a) in pm.data_mut().iter_mut().zip(&acc) {
            *o = (a / wsum).round().clamp(0.0, 255.0) as u8;
        }
    }
    fn audio_track(&self) -> AudioTrack {
        self.inner.audio_track()
    }
    fn caption_cues(&self) -> Vec<Cue> {
        self.inner.caption_cues()
    }
}

/// Apply frame effects (optionally animated) to a whole composition.
pub struct WithEffects {
    pub inner: Comp,
    pub f: Arc<dyn Fn(f64) -> Vec<Effect> + Send + Sync>,
}

impl WithEffects {
    pub fn new(
        c: impl Composition + 'static,
        f: impl Fn(f64) -> Vec<Effect> + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(c),
            f: Arc::new(f),
        }
    }
}

impl Composition for WithEffects {
    fn duration(&self) -> f64 {
        self.inner.duration()
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.inner.render_frame(t, pm, cfg);
        for e in (self.f)(t) {
            e.apply(pm);
        }
    }
    fn audio_track(&self) -> AudioTrack {
        self.inner.audio_track()
    }
    fn caption_cues(&self) -> Vec<Cue> {
        self.inner.caption_cues()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::*;
    use crate::scene::Scene;
    use crate::shapes::*;
    use crate::transition::Presentation;

    fn red_scene(d: f64) -> Scene {
        let mut s = Scene::new().background(RED);
        s.add(circle(1.0));
        s.wait(d);
        s
    }

    #[test]
    fn movie_timing_and_render() {
        let cfg = RenderConfig::new(64, 36, 30.0);
        let a = red_scene(2.0);
        let mut b = Scene::new().background(BLUE);
        let c = b.register(square(1.0));
        b.play(fade_in(c));
        b.wait(1.0);
        b.narrate("hello world");
        let m = Movie::new()
            .then(a)
            .transition(Transition::fade(0.5))
            .then(b);
        assert!((m.starts()[1] - 1.5).abs() < 1e-9);
        let total = m.duration();
        assert!(total > 3.4);
        let mut pm = Pixmap::new(64, 36).unwrap();
        m.render_frame(0.2, &mut pm, cfg);
        assert!(pm.pixel(2, 2).unwrap().red() > 200);
        m.render_frame(total - 0.1, &mut pm, cfg);
        assert!(pm.pixel(2, 2).unwrap().blue() > 150);
        m.render_frame(1.75, &mut pm, cfg); // mid transition
        let px = pm.pixel(2, 2).unwrap();
        assert!(px.red() > 60 && px.blue() > 60);
        assert!(!m.caption_cues().is_empty());
        assert!(m.caption_cues()[0].start >= 1.5);
    }

    #[test]
    fn wrappers() {
        let cfg = RenderConfig::new(32, 18, 30.0);
        let l = Loop::new(red_scene(1.0), 3);
        assert!((l.duration() - 3.0).abs() < 1e-9);
        let sp = Speed::new(red_scene(2.0), 2.0);
        assert!((sp.duration() - 1.0).abs() < 1e-9);
        let st = Still::new(circle(1.0), 1.0);
        let fz = Freeze::new(red_scene(1.0), 0.5, 2.0);
        let f = FnComposition::new(1.0, |fi| vec![circle(0.5 + fi.t)]);
        let layered = Layers::new().layer(st, 0.0).layer(f, 0.0);
        let mut pm = Pixmap::new(32, 18).unwrap();
        layered.render_frame(0.5, &mut pm, cfg);
        fz.render_frame(1.0, &mut pm, cfg);
        let mb = MotionBlur::new(red_scene(1.0), 4, 0.5);
        mb.render_frame(0.5, &mut pm, cfg);
        let fx = WithEffects::new(red_scene(1.0), |_| vec![Effect::Grayscale(1.0)]);
        fx.render_frame(0.5, &mut pm, cfg);
        let p = pm.pixel(1, 1).unwrap();
        assert_eq!(p.red(), p.green());
        let seq = Sequence::new(red_scene(3.0), 1.0, 1.0);
        assert_eq!(seq.duration(), 1.0);
        let _ = Presentation::all();
    }
}
