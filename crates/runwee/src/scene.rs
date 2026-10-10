//! The `Scene` timeline builder (manim-style `play`/`wait`) and frame evaluation.
//!
//! A scene records *segments* on per-object timelines. Rendering a frame at time
//! `t` evaluates every object independently, so frames can be rendered in any
//! order and in parallel.

use crate::anim::*;
use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::rate::*;
use crate::render::*;
use crate::shapes;
use crate::style::*;
use std::sync::Arc;

/// Handle to an object in a scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Obj(pub usize);

/// Handle to an animatable scalar (manim's `ValueTracker`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tracker(pub usize);

impl Obj {
    /// Start an `.animate()` builder.
    pub fn animate(self) -> AnimateBuilder {
        AnimateBuilder::new(self)
    }
}

impl Tracker {
    /// Animate the tracker to `value`.
    pub fn to(self, value: f64) -> Animation {
        Animation::new(AnimKind::TrackerTo {
            tracker: self,
            value,
        })
    }
}

/// Per-frame context given to dynamic objects and updaters.
pub struct FrameCtx<'a> {
    /// Absolute scene time in seconds.
    pub t: f64,
    /// Seconds since the updater / dynamic object was added.
    pub elapsed: f64,
    scene: &'a Scene,
    depth: usize,
}

impl FrameCtx<'_> {
    /// Current value of a tracker.
    pub fn value(&self, k: Tracker) -> f64 {
        self.scene.tracker_value_at(k, self.t)
    }
    /// State of another object at this frame.
    pub fn get(&self, o: Obj) -> Mobject {
        if self.depth > 16 {
            return Mobject::empty();
        }
        self.scene.eval_obj(o, self.t, self.depth + 1)
    }
    pub fn camera(&self) -> Camera {
        self.scene.camera_at(self.t)
    }
    /// State of another object at an arbitrary time (for trails/history).
    pub fn get_at(&self, o: Obj, t: f64) -> Mobject {
        if self.depth > 16 {
            return Mobject::empty();
        }
        self.scene.eval_obj(o, t, self.depth + 1)
    }
}

pub type DynFn = Arc<dyn Fn(&FrameCtx) -> Mobject + Send + Sync>;
pub type UpdaterFn = Arc<dyn Fn(Mobject, &FrameCtx) -> Mobject + Send + Sync>;

/// A compiled animation item acting on one object.
#[derive(Clone)]
struct Item {
    off: f64,
    dur: f64,
    rate: Rate,
    /// Gets *linear* progress; applies `rate` itself when relevant.
    f: Arc<dyn Fn(&Mobject, f64, &Rate) -> Mobject + Send + Sync>,
    /// Applied (at alpha 0) before it starts — for animations that introduce an object.
    introduces: bool,
}

#[derive(Clone)]
struct Segment {
    start: f64,
    end: f64,
    base: Arc<Mobject>,
    items: Vec<Item>,
    end_state: Arc<Mobject>,
}

fn eval_items(items: &[Item], base: &Mobject, s: f64) -> Mobject {
    let mut m = base.clone();
    for it in items {
        let local = s - it.off;
        if local < 0.0 {
            if it.introduces {
                m = (it.f)(&m, 0.0, &it.rate);
            }
            continue;
        }
        let a = if it.dur <= 1e-12 {
            1.0
        } else {
            (local / it.dur).min(1.0)
        };
        m = (it.f)(&m, a, &it.rate);
    }
    m
}

struct Updater {
    start: f64,
    end: f64,
    f: UpdaterFn,
}

struct Entry {
    segments: Vec<Segment>,
    dynamic: Option<(f64, DynFn)>,
    visible: Vec<(f64, f64)>,
    updaters: Vec<Updater>,
    z: f64,
    fixed: bool,
}

impl Entry {
    fn visible_at(&self, t: f64) -> bool {
        self.visible.iter().any(|&(a, b)| t >= a && t < b)
    }
    fn is_open(&self) -> bool {
        self.visible.last().is_some_and(|v| v.1.is_infinite())
    }
}

#[derive(Clone)]
struct ScalarSeg {
    start: f64,
    end: f64,
    from: f64,
    to: f64,
    rate: Rate,
    /// Custom value `f(from, eased_alpha)` (otherwise linear `from → to`).
    f: Option<ScalarFn>,
}

#[derive(Clone)]
struct CameraSeg {
    start: f64,
    end: f64,
    from: Camera,
    to: Camera,
    rate: Rate,
}

/// Named point in time (chapters, sync markers).
#[derive(Clone, Debug)]
pub struct Marker {
    pub name: String,
    pub time: f64,
}

/// Scene-wide settings.
#[derive(Clone, Copy, Debug)]
pub struct SceneConfig {
    pub background: Color,
    /// Visible height in scene units.
    pub frame_height: f64,
    /// Width / height of the output frame.
    pub aspect: f64,
    /// Colour adaptation for light backgrounds (set by [`Scene::theme`]).
    pub ink: Ink,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            background: BACKGROUND,
            frame_height: 8.0,
            aspect: 16.0 / 9.0,
            ink: Ink::Normal,
        }
    }
}

/// The timeline builder.
pub struct Scene {
    pub config: SceneConfig,
    entries: Vec<Entry>,
    trackers: Vec<(f64, Vec<ScalarSeg>)>,
    camera_segs: Vec<CameraSeg>,
    camera0: Camera,
    time: f64,
    pub markers: Vec<Marker>,
    pub audio: crate::audio::AudioTrack,
    pub captions: crate::captions::CaptionTrack,
    /// Extra time appended at the end (avoids cutting the last frame).
    pub tail: f64,
    post_effects: Vec<(f64, f64, PostFn)>,
    /// End of the latest animation started with [`Scene::play_background`].
    bg_end: f64,
    /// Background changes: (start time, cross-fade duration, background).
    backdrops: Vec<(f64, f64, crate::background::Background)>,
    pub(crate) saved: std::collections::HashMap<Obj, Mobject>,
}

type PostFn = Arc<dyn Fn(f64) -> Vec<crate::effects::Effect> + Send + Sync>;

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self::with_config(SceneConfig::default())
    }
    pub fn with_config(config: SceneConfig) -> Self {
        set_frame_size(config.frame_height * config.aspect, config.frame_height);
        Self {
            config,
            entries: Vec::new(),
            trackers: Vec::new(),
            camera_segs: Vec::new(),
            camera0: Camera {
                background: config.background,
                frame_height: config.frame_height,
                ..Camera::default()
            },
            time: 0.0,
            markers: Vec::new(),
            audio: Default::default(),
            captions: Default::default(),
            tail: 0.0,
            post_effects: Vec::new(),
            backdrops: Vec::new(),
            bg_end: 0.0,
            saved: Default::default(),
        }
    }
    /// Vertical 9:16 scene (shorts / reels), 14.22 units tall, 8 wide.
    pub fn vertical() -> Self {
        Self::with_config(SceneConfig {
            frame_height: 8.0 * 16.0 / 9.0,
            aspect: 9.0 / 16.0,
            ..Default::default()
        })
    }
    pub fn background(mut self, c: Color) -> Self {
        self.config.background = c;
        self.camera0.background = c;
        self
    }

    /// Use `bg` (colour, gradient, image, video, pattern, animated preset, …) from the
    /// current time on. At time 0 it is the scene's background; later calls switch to it.
    pub fn set_background(&mut self, bg: impl Into<crate::background::Background>) {
        self.background_fade(bg, 0.0);
    }

    /// Replace the background from time 0 (keeps later background changes).
    pub fn base_background(&mut self, bg: impl Into<crate::background::Background>) {
        let bg = bg.into();
        let c = bg.base_color();
        self.config.background = c;
        self.camera0.background = c;
        match self.backdrops.first_mut() {
            Some(first) if first.0 <= 1e-9 => first.2 = bg,
            _ => self.backdrops.insert(0, (0.0, 0.0, bg)),
        }
    }

    /// Replace the theme from time 0 (used by the CLI's `--theme`).
    pub fn base_theme(&mut self, theme: crate::background::Theme) {
        self.config.ink = theme.ink;
        self.base_background(theme.background);
    }

    /// Cross-fade to `bg` over `duration` seconds starting now.
    pub fn background_fade(&mut self, bg: impl Into<crate::background::Background>, duration: f64) {
        let bg = bg.into();
        if self.time <= 1e-9 && self.backdrops.is_empty() {
            let c = bg.base_color();
            self.config.background = c;
            self.camera0.background = c;
        }
        self.backdrops.retain(|b| b.0 < self.time - 1e-9);
        self.backdrops.push((self.time, duration.max(0.0), bg));
    }

    /// A named background preset (`"blueprint"`, `"aurora"`, `"graph-paper"`, a colour, …).
    pub fn background_preset(&mut self, name: &str) -> anyhow::Result<()> {
        let bg = crate::background::Background::preset(name).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown background '{name}' (try one of: {})",
                crate::background::BACKGROUND_PRESETS.join(", ")
            )
        })?;
        self.set_background(bg);
        Ok(())
    }

    /// Apply a theme: its background plus the ink mode that keeps colours readable
    /// (light themes darken white text and pastel accents automatically).
    pub fn theme(&mut self, theme: crate::background::Theme) {
        self.config.ink = theme.ink;
        self.set_background(theme.background);
    }

    /// Builder form of [`Scene::theme`].
    pub fn with_theme(mut self, theme: crate::background::Theme) -> Self {
        self.theme(theme);
        self
    }

    fn draw_backdrop(&self, t: f64, pm: &mut tiny_skia::Pixmap) {
        let Some(i) = self.backdrops.iter().rposition(|b| b.0 <= t + 1e-9) else {
            if let Some(first) = self.backdrops.first() {
                first.2.draw(t, pm);
            }
            return;
        };
        let (start, fade, bg) = &self.backdrops[i];
        let p = if *fade > 0.0 {
            ((t - start) / fade).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if p < 1.0 && i > 0 {
            self.backdrops[i - 1].2.draw(t, pm);
            if let Some(mut layer) = tiny_skia::Pixmap::new(pm.width(), pm.height()) {
                bg.draw(t, &mut layer);
                pm.draw_pixmap(
                    0,
                    0,
                    layer.as_ref(),
                    &tiny_skia::PixmapPaint {
                        opacity: crate::rate::Rate::Smooth.eval(p) as f32,
                        ..Default::default()
                    },
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        } else {
            bg.draw(t, pm);
        }
    }

    /// Current time of the builder cursor.
    pub fn time(&self) -> f64 {
        self.time
    }
    /// Total duration.
    pub fn duration(&self) -> f64 {
        let audio_end = self.audio.end_time();
        (self.time + self.tail)
            .max(audio_end)
            .max(self.bg_end)
            .max(1.0 / 120.0)
    }
    /// Record a named marker at the current time.
    pub fn marker(&mut self, name: &str) {
        self.markers.push(Marker {
            name: name.to_string(),
            time: self.time,
        });
    }
    /// Move the cursor back to an earlier time so following animations overlap work
    /// already scheduled (used for background camera motion). Only rewind over
    /// objects/trackers you don't animate again in that window.
    pub fn rewind_to(&mut self, t: f64) {
        self.time = t.max(0.0);
    }
    /// Move the cursor to an absolute time (only forward).
    pub fn seek(&mut self, t: f64) {
        if t > self.time {
            self.time = t;
        }
    }

    // ------------------------------------------------------------ objects

    fn new_entry(&mut self, m: Mobject, from: f64, to: f64, fixed: bool) -> Obj {
        let m = Arc::new(m);
        self.entries.push(Entry {
            segments: vec![Segment {
                start: from,
                end: from,
                base: m.clone(),
                items: vec![],
                end_state: m,
            }],
            dynamic: None,
            visible: if to > from { vec![(from, to)] } else { vec![] },
            updaters: vec![],
            z: 0.0,
            fixed,
        });
        Obj(self.entries.len() - 1)
    }

    /// Add a mobject (visible immediately) and return its handle.
    pub fn add(&mut self, m: Mobject) -> Obj {
        self.new_entry(m, self.time, f64::INFINITY, false)
    }
    /// Register a mobject without showing it (it appears when an animation introduces it).
    pub fn register(&mut self, m: Mobject) -> Obj {
        self.new_entry(m, self.time, self.time, false)
    }
    /// Add an object that ignores the camera (titles, HUD, watermarks).
    pub fn add_fixed(&mut self, m: Mobject) -> Obj {
        self.new_entry(m, self.time, f64::INFINITY, true)
    }
    pub fn register_fixed(&mut self, m: Mobject) -> Obj {
        self.new_entry(m, self.time, self.time, true)
    }
    pub fn add_all(&mut self, ms: impl IntoIterator<Item = Mobject>) -> Vec<Obj> {
        ms.into_iter().map(|m| self.add(m)).collect()
    }
    /// Object rebuilt every frame from a closure (manim's `always_redraw`).
    pub fn always_redraw(
        &mut self,
        f: impl Fn(&FrameCtx) -> Mobject + Send + Sync + 'static,
    ) -> Obj {
        let o = self.new_entry(Mobject::empty(), self.time, f64::INFINITY, false);
        self.entries[o.0].dynamic = Some((self.time, Arc::new(f)));
        o
    }
    /// Like `always_redraw` but not shown until introduced by an animation.
    pub fn register_redraw(
        &mut self,
        f: impl Fn(&FrameCtx) -> Mobject + Send + Sync + 'static,
    ) -> Obj {
        let o = self.new_entry(Mobject::empty(), self.time, self.time, false);
        self.entries[o.0].dynamic = Some((self.time, Arc::new(f)));
        o
    }
    pub fn set_fixed(&mut self, o: Obj, fixed: bool) {
        self.entries[o.0].fixed = fixed;
    }
    pub fn is_present(&self, o: Obj) -> bool {
        self.entries[o.0].is_open()
    }
    fn show(&mut self, o: Obj, t: f64) {
        let e = &mut self.entries[o.0];
        if !e.is_open() {
            e.visible.push((t, f64::INFINITY));
        }
    }
    fn hide(&mut self, o: Obj, t: f64) {
        let e = &mut self.entries[o.0];
        if let Some(v) = e.visible.last_mut() {
            if v.1.is_infinite() {
                v.1 = t.max(v.0);
            }
        }
    }
    /// Remove from view now.
    pub fn remove(&mut self, o: Obj) {
        let t = self.time;
        self.hide(o, t);
    }
    /// Show a previously removed or registered object now.
    pub fn show_now(&mut self, o: Obj) {
        let t = self.time;
        self.show(o, t);
    }
    pub fn remove_all(&mut self) {
        let t = self.time;
        for i in 0..self.entries.len() {
            self.hide(Obj(i), t);
        }
    }
    /// Objects currently on screen.
    pub fn present(&self) -> Vec<Obj> {
        (0..self.entries.len())
            .filter(|&i| self.entries[i].is_open())
            .map(Obj)
            .collect()
    }
    pub fn set_z(&mut self, o: Obj, z: f64) {
        self.entries[o.0].z = z;
    }
    pub fn bring_to_front(&mut self, o: Obj) {
        let z = self.entries.iter().map(|e| e.z).fold(0.0, f64::max);
        self.entries[o.0].z = z + 1.0;
    }
    pub fn bring_to_back(&mut self, o: Obj) {
        let z = self.entries.iter().map(|e| e.z).fold(0.0, f64::min);
        self.entries[o.0].z = z - 1.0;
    }

    /// The builder's current state of an object (manim: the mobject variable itself).
    pub fn get(&self, o: Obj) -> Mobject {
        self.eval_raw(o, self.time, 0)
    }
    /// Replace an object's state instantly.
    pub fn set(&mut self, o: Obj, m: Mobject) {
        let t = self.time;
        let m = Arc::new(m);
        self.entries[o.0].segments.push(Segment {
            start: t,
            end: t,
            base: m.clone(),
            items: vec![],
            end_state: m,
        });
    }
    /// Modify an object instantly.
    pub fn modify(&mut self, o: Obj, f: impl FnOnce(Mobject) -> Mobject) {
        let m = f(self.get(o));
        self.set(o, m);
    }
    /// Attach an updater from now on (manim's `add_updater`).
    pub fn add_updater(
        &mut self,
        o: Obj,
        f: impl Fn(Mobject, &FrameCtx) -> Mobject + Send + Sync + 'static,
    ) {
        let t = self.time;
        self.entries[o.0].updaters.push(Updater {
            start: t,
            end: f64::INFINITY,
            f: Arc::new(f),
        });
    }
    /// Stop all updaters of an object now (the object keeps its last updated look).
    pub fn clear_updaters(&mut self, o: Obj) {
        let t = self.time;
        let frozen = self.eval_obj(o, t, 0);
        for u in &mut self.entries[o.0].updaters {
            if u.end.is_infinite() {
                u.end = t;
            }
        }
        if self.entries[o.0].dynamic.is_some() {
            // freeze the dynamic object into a static one
            self.entries[o.0].dynamic = None;
        }
        self.set(o, frozen);
    }

    // ----------------------------------------------------------- trackers

    pub fn tracker(&mut self, value: f64) -> Tracker {
        self.trackers.push((value, Vec::new()));
        Tracker(self.trackers.len() - 1)
    }
    /// A live number bound to a tracker (manim's `DecimalNumber` + `ChangingDecimal`).
    /// Animate it with `k.to(value)`.
    pub fn counter(&mut self, k: Tracker, fmt: crate::graph::DecimalNumber, at: Vec2) -> Obj {
        self.always_redraw(move |ctx| fmt.build(ctx.value(k)).move_to(at))
    }

    pub fn set_value(&mut self, k: Tracker, v: f64) {
        let t = self.time;
        self.trackers[k.0].1.push(ScalarSeg {
            start: t,
            end: t,
            from: v,
            to: v,
            rate: Rate::Linear,
            f: None,
        });
    }
    pub fn value(&self, k: Tracker) -> f64 {
        self.tracker_value_at(k, self.time)
    }
    pub fn tracker_value_at(&self, k: Tracker, t: f64) -> f64 {
        let (init, segs) = &self.trackers[k.0];
        let idx = segs.partition_point(|s| s.start <= t);
        if idx == 0 {
            return *init;
        }
        let s = &segs[idx - 1];
        if t >= s.end || s.end <= s.start {
            s.to
        } else {
            let a = s.rate.eval((t - s.start) / (s.end - s.start));
            match &s.f {
                Some(f) => f(s.from, a),
                None => lerp(s.from, s.to, a),
            }
        }
    }

    // ------------------------------------------------------------- camera

    pub fn camera_at(&self, t: f64) -> Camera {
        let idx = self.camera_segs.partition_point(|s| s.start <= t);
        if idx == 0 {
            return self.camera0;
        }
        let s = &self.camera_segs[idx - 1];
        if t >= s.end || s.end <= s.start {
            s.to
        } else {
            s.from
                .lerp(&s.to, s.rate.eval((t - s.start) / (s.end - s.start)))
        }
    }
    pub fn set_camera(&mut self, target: CameraTarget) {
        let t = self.time;
        let c = target.apply(&self.camera_at(t));
        self.camera_segs.push(CameraSeg {
            start: t,
            end: t,
            from: c,
            to: c,
            rate: Rate::Linear,
        });
    }

    // ---------------------------------------------------------- evaluation

    fn eval_raw(&self, o: Obj, t: f64, depth: usize) -> Mobject {
        let e = &self.entries[o.0];
        let idx = e.segments.partition_point(|s| s.start <= t);
        let seg = if idx == 0 {
            &e.segments[0]
        } else {
            &e.segments[idx - 1]
        };
        if let Some((t0, f)) = &e.dynamic {
            let ctx = FrameCtx {
                t,
                elapsed: t - t0,
                scene: self,
                depth,
            };
            let live = f(&ctx);
            if t < seg.end && !seg.items.is_empty() {
                return eval_items(&seg.items, &live, t - seg.start);
            }
            return live;
        }
        if t < seg.end && !seg.items.is_empty() {
            eval_items(&seg.items, &seg.base, t - seg.start)
        } else {
            (*seg.end_state).clone()
        }
    }

    /// Full state of an object at time `t` (timeline + updaters).
    pub fn eval_obj(&self, o: Obj, t: f64, depth: usize) -> Mobject {
        let mut m = self.eval_raw(o, t, depth);
        let e = &self.entries[o.0];
        for u in &e.updaters {
            if t >= u.start && t < u.end {
                let ctx = FrameCtx {
                    t,
                    elapsed: t - u.start,
                    scene: self,
                    depth,
                };
                m = (u.f)(m, &ctx);
            }
        }
        m
    }

    /// Visible mobjects at time `t`, in draw order, with their "fixed in frame" flag.
    pub fn frame_mobjects(&self, t: f64) -> Vec<(Mobject, bool)> {
        let mut idx: Vec<usize> = (0..self.entries.len())
            .filter(|&i| self.entries[i].visible_at(t))
            .collect();
        idx.sort_by(|&a, &b| {
            self.entries[a]
                .z
                .partial_cmp(&self.entries[b].z)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });
        idx.into_iter()
            .map(|i| (self.eval_obj(Obj(i), t, 0), self.entries[i].fixed))
            .collect()
    }

    /// Draw the frame at time `t`.
    pub fn render_into(&self, t: f64, pm: &mut tiny_skia::Pixmap, cfg: RenderConfig) {
        self.render_impl(t, pm, cfg, true);
    }

    /// Draw the frame; `clear` fills the background first (false = draw as a layer).
    pub fn render_impl(&self, t: f64, pm: &mut tiny_skia::Pixmap, cfg: RenderConfig, clear: bool) {
        let cam = self.camera_at(t);
        let mobs = self.frame_mobjects(t);
        let cfg = RenderConfig {
            ink: self.config.ink,
            ..cfg
        };
        if clear && !self.backdrops.is_empty() {
            self.draw_backdrop(t, pm);
        }
        let mut canvas = Canvas::new(pm, cfg, cam);
        if clear && self.backdrops.is_empty() {
            canvas.clear();
        }
        let any_fixed = mobs.iter().any(|m| m.1);
        for (m, fixed) in &mobs {
            if !fixed {
                canvas.draw(m);
            }
        }
        if any_fixed {
            let fixed_cam = Camera {
                background: cam.background,
                frame_height: self.config.frame_height,
                ..Camera::default()
            };
            let mut canvas = Canvas::new(pm, cfg, fixed_cam);
            for (m, fixed) in &mobs {
                if *fixed {
                    canvas.draw(m);
                }
            }
        }
        // frame effects (Remotion @remotion/effects style)
        for pe in &self.post_effects {
            if t >= pe.0 && t < pe.1 {
                for e in (pe.2)(t - pe.0) {
                    e.apply(pm);
                }
            }
        }
        // burned-in captions are drawn in screen space on top
        self.captions.draw(t, pm, cfg, self.config.frame_height);
    }

    /// Apply frame effects between `from` and `to` (seconds); the closure gets the
    /// time since `from` so effect parameters can animate.
    pub fn post_effect(
        &mut self,
        from: f64,
        to: f64,
        f: impl Fn(f64) -> Vec<crate::effects::Effect> + Send + Sync + 'static,
    ) {
        self.post_effects.push((from, to, Arc::new(f)));
    }
    /// Apply constant frame effects for the whole video.
    pub fn effects(&mut self, effects: Vec<crate::effects::Effect>) {
        self.post_effect(0.0, f64::INFINITY, move |_| effects.clone());
    }

    // ------------------------------------------------------------- timing

    pub fn wait(&mut self, d: f64) {
        self.time += d.max(0.0);
    }

    /// Play animations together (manim's `self.play`). Returns the duration.
    pub fn play(&mut self, anims: impl IntoAnims) -> f64 {
        let anims = anims.into_anims();
        let t0 = self.time;
        let mut ctx = Compile::default();
        for a in &anims {
            let d = a.run_time.unwrap_or_else(|| self.natural_duration(a));
            self.compile(a, a.delay, d, &mut ctx);
        }
        let total = self.commit(t0, ctx);
        self.time = t0 + total;
        total
    }

    /// Start animations now without moving the cursor, so whatever is played next
    /// runs at the same time (e.g. a slow camera move or a parameter sweep under a
    /// narration). Don't animate the same objects again inside that window.
    pub fn play_background(&mut self, run_time: f64, anims: impl IntoAnims) -> f64 {
        let t0 = self.time;
        let d = self.play_for(run_time, anims);
        self.bg_end = self.bg_end.max(t0 + d);
        self.rewind_to(t0);
        d
    }

    /// Play with an explicit total run time.
    pub fn play_for(&mut self, run_time: f64, anims: impl IntoAnims) -> f64 {
        let anims: Vec<Animation> = anims
            .into_anims()
            .into_iter()
            .map(|a| {
                if a.run_time.is_none() {
                    a.run_time(run_time)
                } else {
                    a
                }
            })
            .collect();
        self.play(anims)
    }

    fn natural_duration(&self, a: &Animation) -> f64 {
        match &a.kind {
            AnimKind::Write(o) | AnimKind::Unwrite(o) => {
                let n = self.get(*o).leaf_count();
                if n < 15 {
                    1.0
                } else {
                    (1.0 + n as f64 / 40.0).min(3.0)
                }
            }
            AnimKind::Typewriter(o) => (self.get(*o).leaf_count() as f64 * 0.05).clamp(0.5, 6.0),
            AnimKind::Group { anims, lag } => {
                let (_, total) = self.group_timings(anims, *lag);
                total
            }
            AnimKind::Wait => 1.0,
            AnimKind::TimeWarp {
                anim, time_scale, ..
            } => anim.run_time.unwrap_or_else(|| self.natural_duration(anim)) * time_scale,
            AnimKind::TransformAnimations { from, to } => {
                let d = |a: &Animation| a.run_time.unwrap_or_else(|| self.natural_duration(a));
                d(from).max(d(to))
            }
            _ => 1.0,
        }
    }

    fn group_timings(&self, anims: &[Animation], lag: f64) -> (Vec<(f64, f64)>, f64) {
        let mut out = Vec::new();
        let mut start = 0.0;
        let mut total: f64 = 0.0;
        for a in anims {
            let d = a.run_time.unwrap_or_else(|| self.natural_duration(a));
            let s = start + a.delay;
            out.push((s, d));
            total = total.max(s + d);
            start = s + lag * d;
        }
        (out, total)
    }

    /// State of `o` as seen by an item starting at play offset `off`.
    fn snapshot(&self, o: Obj, ctx: &Compile, off: f64) -> Mobject {
        let base = self.eval_raw(o, self.time, 0);
        match ctx.items.iter().find(|(x, _)| *x == o) {
            Some((_, items)) => eval_items(items, &base, off),
            None => base,
        }
    }

    fn compile(&mut self, a: &Animation, off: f64, dur: f64, ctx: &mut Compile) {
        let rate = a.rate.clone();
        let lag = a.lag_ratio;
        let push = |ctx: &mut Compile, o: Obj, introduces: bool, f: ItemFn| {
            ctx.push(
                o,
                Item {
                    off,
                    dur,
                    rate: rate.clone(),
                    f,
                    introduces,
                },
            );
        };
        match &a.kind {
            AnimKind::Wait => ctx.extend(off + dur),
            AnimKind::Group { anims, lag: l } => {
                let (timings, natural) = self.group_timings(anims, *l);
                let k = if natural > 0.0 { dur / natural } else { 1.0 };
                for (child, (s, d)) in anims.iter().zip(timings) {
                    self.compile(child, off + s * k, d * k, ctx);
                }
                ctx.extend(off + dur);
            }
            AnimKind::Create(o) => {
                ctx.introduce(*o, self.is_present(*o));
                let lag = lag.unwrap_or(0.0);
                push(
                    ctx,
                    *o,
                    true,
                    Arc::new(move |m, a, r| create_partial(m, r.eval(a), lag)),
                );
            }
            AnimKind::Uncreate(o) => {
                let lag = lag.unwrap_or(0.0);
                push(
                    ctx,
                    *o,
                    false,
                    Arc::new(move |m, a, r| create_partial(m, 1.0 - r.eval(a), lag)),
                );
                ctx.remove(*o, off + dur);
            }
            AnimKind::Write(o) | AnimKind::DrawBorderThenFill(o) => {
                ctx.introduce(*o, self.is_present(*o));
                let n = self.snapshot(*o, ctx, off).leaf_count();
                let lag = lag.unwrap_or_else(|| {
                    if matches!(a.kind, AnimKind::DrawBorderThenFill(_)) {
                        0.0
                    } else {
                        (4.0 / (n as f64 + 1.0)).min(0.2)
                    }
                });
                push(
                    ctx,
                    *o,
                    true,
                    Arc::new(move |m, a, r| write_effect(m, r.eval(a), lag)),
                );
            }
            AnimKind::Unwrite(o) => {
                let n = self.snapshot(*o, ctx, off).leaf_count();
                let lag = lag.unwrap_or((4.0 / (n as f64 + 1.0)).min(0.2));
                push(
                    ctx,
                    *o,
                    false,
                    Arc::new(move |m, a, r| write_effect(m, 1.0 - r.eval(a), lag)),
                );
                ctx.remove(*o, off + dur);
            }
            AnimKind::FadeIn { obj, shift, scale } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let (shift, scale) = (*shift, *scale);
                push(
                    ctx,
                    *obj,
                    true,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        let mut out = m.clone().shift(-shift * (1.0 - x));
                        if (scale - 1.0).abs() > 1e-9 {
                            out = out.scale(lerp(scale, 1.0, x).max(1e-4));
                        }
                        multiply_opacity(out, x as f32)
                    }),
                );
            }
            AnimKind::FadeOut { obj, shift, scale } => {
                let (shift, scale) = (*shift, *scale);
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        let mut out = m.clone().shift(shift * x);
                        if (scale - 1.0).abs() > 1e-9 {
                            out = out.scale(lerp(1.0, scale, x).max(1e-4));
                        }
                        multiply_opacity(out, 1.0 - x as f32)
                    }),
                );
                ctx.remove(*obj, off + dur);
            }
            AnimKind::Grow { obj, from } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let snap = self.snapshot(*obj, ctx, off);
                let p = match from {
                    GrowFrom::Center => snap.center(),
                    GrowFrom::Point(p) => *p,
                    GrowFrom::Edge(d) => snap.edge(*d),
                    GrowFrom::Start => snap.start_point(),
                };
                push(
                    ctx,
                    *obj,
                    true,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        if x <= 0.0 {
                            return multiply_opacity(m.clone(), 0.0);
                        }
                        m.clone().scale_about(x, p)
                    }),
                );
            }
            AnimKind::SpinIn(o) => {
                ctx.introduce(*o, self.is_present(*o));
                push(
                    ctx,
                    *o,
                    true,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        if x <= 0.0 {
                            return multiply_opacity(m.clone(), 0.0);
                        }
                        let c = m.center();
                        m.clone()
                            .rotate_about(PI / 2.0 * (1.0 - x), c)
                            .scale_about(x, c)
                    }),
                );
            }
            AnimKind::ShrinkToCenter(o) => {
                push(
                    ctx,
                    *o,
                    false,
                    Arc::new(move |m, a, r| {
                        let x = 1.0 - r.eval(a);
                        m.clone().scale(x.max(1e-4))
                    }),
                );
                ctx.remove(*o, off + dur);
            }
            AnimKind::Transform { obj, target, arc } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let snap = self.snapshot(*obj, ctx, off);
                push(ctx, *obj, false, morph_fn(&snap, target, *arc));
            }
            AnimKind::ReplacementTransform { from, to, arc } => {
                let src = self.snapshot(*from, ctx, off);
                let dst = self.snapshot(*to, ctx, off);
                ctx.introduce(*from, self.is_present(*from));
                push(ctx, *from, false, morph_fn(&src, &dst, *arc));
                ctx.remove(*from, off + dur);
                ctx.show_at(*to, off + dur);
            }
            AnimKind::TransformFromCopy { from, to, arc } => {
                let src = self.snapshot(*from, ctx, off);
                let dst = self.snapshot(*to, ctx, off);
                ctx.introduce(*to, self.is_present(*to));
                push(ctx, *to, true, morph_fn(&src, &dst, *arc));
            }
            AnimKind::FadeTransform { obj, target } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let src = self.snapshot(*obj, ctx, off);
                push(ctx, *obj, false, fade_transform_fn(&src, target));
            }
            AnimKind::TransformMatching { obj, target } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let src = self.snapshot(*obj, ctx, off);
                push(ctx, *obj, false, matching_fn(&src, target));
            }
            AnimKind::TransformMatchingFromCopy { from, to } => {
                let src = self.snapshot(*from, ctx, off).flattened();
                let dst = self.snapshot(*to, ctx, off);
                ctx.introduce(*to, self.is_present(*to));
                let f = matching_fn(&src, &dst.flattened());
                let target = Arc::new(dst);
                push(
                    ctx,
                    *to,
                    true,
                    Arc::new(move |m, a, r| {
                        if a >= 1.0 {
                            (*target).clone()
                        } else {
                            f(m, a, r)
                        }
                    }),
                );
            }
            AnimKind::TransformMatchingShapes { obj, target } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let src = self.snapshot(*obj, ctx, off).flattened();
                push(ctx, *obj, false, matching_fn(&src, &target.flattened()));
            }
            AnimKind::Animate { obj, ops } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let snap = self.snapshot(*obj, ctx, off);
                push(ctx, *obj, false, animate_fn(&snap, ops));
            }
            AnimKind::Rotate { obj, angle, about } => {
                let snap = self.snapshot(*obj, ctx, off);
                let c = about.unwrap_or_else(|| snap.center());
                let angle = *angle;
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |m, a, r| m.clone().rotate_about(angle * r.eval(a), c)),
                );
            }
            AnimKind::MoveAlongPath { obj, path, orient } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let path = Arc::new(path.clone());
                let orient = *orient;
                let snap = self.snapshot(*obj, ctx, off);
                let a0 = path.tangent_from_proportion(0.0).angle();
                let c0 = snap.center();
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        let p = path.point_from_proportion(x);
                        let mut out = m.clone();
                        if orient {
                            let ang = path.tangent_from_proportion(x).angle() - a0;
                            out = out.rotate_about(ang, c0);
                        }
                        let c = out.center();
                        out.shift(p - c)
                    }),
                );
            }
            AnimKind::ApplyFunction { obj, f } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let snap = subdivide_for_warp(&self.snapshot(*obj, ctx, off));
                let target = snap.clone().apply_function(|p| f(p));
                push(ctx, *obj, false, pointwise_fn(&snap, &target));
            }
            AnimKind::Homotopy { obj, h } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let snap = Arc::new(subdivide_for_warp(&self.snapshot(*obj, ctx, off)));
                let h = h.clone();
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |_, a, r| {
                        let x = r.eval(a);
                        (*snap).clone().apply_function(|p| h(p, x))
                    }),
                );
            }
            AnimKind::Indicate { obj, color, scale } => {
                let (color, scale) = (*color, *scale);
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        tint(m.clone(), color, x as f32).scale(lerp(1.0, scale, x))
                    }),
                );
            }
            AnimKind::Pulse { obj, scale } => {
                let scale = *scale;
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |m, a, r| m.clone().scale(lerp(1.0, scale, r.eval(a)))),
                );
            }
            AnimKind::Blink(obj) => {
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        let c = m.center();
                        m.clone().stretch(1.0, (1.0 - x).max(0.05)).move_to(c)
                    }),
                );
            }
            AnimKind::Wiggle {
                obj,
                scale,
                angle,
                wiggles,
            } => {
                let (scale, angle, wiggles) = (*scale, *angle, *wiggles);
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |m, a, _r| {
                        let s = Rate::ThereAndBack.eval(a);
                        let w = Rate::Wiggle(wiggles).eval(a);
                        m.clone().scale(lerp(1.0, scale, s)).rotate(angle * w)
                    }),
                );
            }
            AnimKind::ApplyWave { obj, amplitude } => {
                let snap = self.snapshot(*obj, ctx, off);
                let b = snap.bounds_or_origin();
                let amp = *amplitude;
                push(
                    ctx,
                    *obj,
                    false,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        let w = b.width().max(1e-6);
                        let env = Rate::ThereAndBack.eval(a);
                        m.clone().apply_function(|p| {
                            let u = (p.x - b.min.x) / w; // 0..1 across the object
                            let phase = u - (x * 1.6 - 0.3);
                            let bump = (-(phase * phase) / 0.02).exp();
                            p + UP * (amp * bump * env)
                        })
                    }),
                );
            }
            AnimKind::ShowPassingFlash { obj, width } => {
                ctx.introduce(*obj, self.is_present(*obj));
                let w = *width;
                push(
                    ctx,
                    *obj,
                    true,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a) * (1.0 + w);
                        m.clone()
                            .partial((x - w).clamp(0.0, 1.0), x.clamp(0.0, 1.0))
                    }),
                );
                ctx.remove(*obj, off + dur);
            }
            AnimKind::Typewriter(obj) => {
                ctx.introduce(*obj, self.is_present(*obj));
                push(
                    ctx,
                    *obj,
                    true,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        let n = m.leaf_count().max(1);
                        let shown = x * n as f64;
                        let mut i = 0usize;
                        let mut out = m.clone();
                        out.for_each_mut(&mut |node| {
                            if node.is_leaf_drawable() {
                                let vis = (shown - i as f64).clamp(0.0, 1.0) as f32;
                                node.style.opacity *= vis;
                                if let Some(img) = &mut node.image {
                                    img.opacity *= vis;
                                }
                                i += 1;
                            }
                        });
                        out
                    }),
                );
            }
            AnimKind::Highlight { obj, color } => {
                let snap = self.snapshot(*obj, ctx, off);
                let b = snap.bounds_or_origin().expand(0.12);
                let bg = shapes::rounded_rectangle(b.width(), b.height(), 0.08)
                    .move_to(b.center())
                    .fill(*color, 0.35)
                    .no_stroke();
                let temp = self.new_entry(bg, self.time, self.time, self.entries[obj.0].fixed);
                self.entries[temp.0].z = self.entries[obj.0].z - 0.001;
                ctx.temp(temp, off, off + dur);
                push(
                    ctx,
                    temp,
                    true,
                    Arc::new(move |m, a, _r| {
                        let x = Rate::ThereAndBackWithPause(0.6).eval(a);
                        let c = m.center();
                        multiply_opacity(m.clone(), x as f32)
                            .stretch(x.max(0.01), 1.0)
                            .move_to(c)
                    }),
                );
            }
            AnimKind::Flash {
                point,
                color,
                radius,
                lines,
            } => {
                let mut g = Mobject::empty();
                for i in 0..*lines {
                    let d = Vec2::from_angle(TAU * i as f64 / *lines as f64);
                    g.push(
                        shapes::line(*point + d * *radius, *point + d * (*radius + 0.45))
                            .stroke(*color, 3.0),
                    );
                }
                let temp = self.new_entry(g, self.time, self.time, false);
                self.bring_to_front(temp);
                ctx.temp(temp, off, off + dur);
                push(
                    ctx,
                    temp,
                    true,
                    Arc::new(move |m, a, _r| {
                        let x = Rate::Smooth.eval(a);
                        m.clone()
                            .partial((2.0 * x - 1.0).clamp(0.0, 1.0), (2.0 * x).clamp(0.0, 1.0))
                    }),
                );
            }
            AnimKind::Circumscribe { obj, color, circle } => {
                let snap = self.snapshot(*obj, ctx, off);
                let shape = if *circle {
                    let b = snap.bounds_or_origin();
                    shapes::circle((b.width().max(b.height()) / 2.0) * 1.15 + 0.1)
                        .move_to(b.center())
                } else {
                    shapes::surrounding_rectangle(&snap, SMALL_BUFF)
                }
                .stroke(*color, 4.0);
                let temp = self.new_entry(shape, self.time, self.time, self.entries[obj.0].fixed);
                self.bring_to_front(temp);
                ctx.temp(temp, off, off + dur);
                push(
                    ctx,
                    temp,
                    true,
                    Arc::new(move |m, a, _r| {
                        let x = Rate::Smooth.eval(a);
                        if x < 0.5 {
                            m.clone().partial(0.0, 2.0 * x)
                        } else {
                            m.clone().partial(2.0 * x - 1.0, 1.0)
                        }
                    }),
                );
            }
            AnimKind::FocusOn { point, color } => {
                let (w, h) = frame_size();
                let r0 = w.max(h);
                let c = shapes::circle_at(*point, r0).fill(*color, 0.25).no_stroke();
                let temp = self.new_entry(c, self.time, self.time, false);
                self.bring_to_front(temp);
                ctx.temp(temp, off, off + dur);
                let p = *point;
                push(
                    ctx,
                    temp,
                    true,
                    Arc::new(move |m, a, r| {
                        let x = r.eval(a);
                        multiply_opacity(
                            m.clone().scale_about((1.0 - x).max(0.01), p),
                            (x * 1.0) as f32,
                        )
                    }),
                );
            }
            AnimKind::Custom {
                obj,
                f,
                introduces,
                removes,
            } => {
                if *introduces {
                    ctx.introduce(*obj, self.is_present(*obj));
                }
                let f = f.clone();
                push(
                    ctx,
                    *obj,
                    *introduces,
                    Arc::new(move |m, a, r| f(m, r.eval(a))),
                );
                if *removes {
                    ctx.remove(*obj, off + dur);
                }
            }
            AnimKind::TrackerTo { tracker, value } => {
                ctx.trackers
                    .push((*tracker, off, dur, *value, rate.clone(), None));
                ctx.extend(off + dur);
            }
            AnimKind::TrackerWith { tracker, f } => {
                ctx.trackers
                    .push((*tracker, off, dur, 0.0, rate.clone(), Some(f.clone())));
                ctx.extend(off + dur);
            }
            AnimKind::Overlay {
                obj,
                build,
                f,
                follow,
            } => {
                let snap = self.snapshot(*obj, ctx, off);
                let fixed = self.entries[obj.0].fixed;
                let temp = self.new_entry(build(&snap), self.time, self.time, fixed);
                if *follow {
                    let (o, b) = (*obj, build.clone());
                    self.entries[temp.0].dynamic =
                        Some((self.time, Arc::new(move |c: &FrameCtx| b(&c.get(o)))));
                }
                self.bring_to_front(temp);
                ctx.temp(temp, off, off + dur);
                let f = f.clone();
                push(ctx, temp, true, Arc::new(move |m, a, r| f(m, r.eval(a))));
            }
            AnimKind::TimeWarp { anim, warp, .. } => {
                let mut sub = Compile::default();
                self.compile(anim, 0.0, dur, &mut sub);
                // outer progress u is eased by this animation's rate, then warped
                let rate = rate.clone();
                let w: WarpFn = {
                    let warp = warp.clone();
                    Arc::new(move |u| warp(rate.eval(u.clamp(0.0, 1.0))))
                };
                ctx.merge_warped(sub, off, dur, w);
            }
            AnimKind::TransformAnimations { from, to } => {
                let mut sa = Compile::default();
                let mut sb = Compile::default();
                self.compile(from, 0.0, dur, &mut sa);
                self.compile(to, 0.0, dur, &mut sb);
                let (Some((oa, ia)), Some((ob, ib))) =
                    (sa.items.into_iter().next(), sb.items.into_iter().next())
                else {
                    ctx.extend(off + dur);
                    return;
                };
                let base_a = Arc::new(self.snapshot(oa, ctx, off));
                let base_b = Arc::new(self.snapshot(ob, ctx, off));
                let final_b = Arc::new(eval_items(&ib, &base_b, dur));
                let (ia, ib) = (Arc::new(ia), Arc::new(ib));
                ctx.introduce(oa, self.is_present(oa));
                push(
                    ctx,
                    oa,
                    false,
                    Arc::new(move |_m, a, r| {
                        if a >= 1.0 {
                            return (*final_b).clone();
                        }
                        let x = r.eval(a);
                        let p = eval_items(&ia, &base_a, a * dur);
                        let q = eval_items(&ib, &base_b, a * dur);
                        let (la, lb) = Mobject::align_for_transform(&p, &q);
                        Mobject::group(
                            la.iter()
                                .zip(lb.iter())
                                .map(|(u, v)| Mobject::lerp_leaf(u, v, x)),
                        )
                    }),
                );
            }
            AnimKind::Camera(target) => {
                ctx.cameras.push((*target, off, dur, rate.clone()));
                ctx.extend(off + dur);
            }
        }
    }

    fn commit(&mut self, t0: f64, ctx: Compile) -> f64 {
        let total = ctx.total;
        for (o, items) in ctx.items {
            let base = Arc::new(self.eval_raw(o, t0, 0));
            let end = items.iter().map(|i| i.off + i.dur).fold(0.0, f64::max);
            let end_state = Arc::new(eval_items(&items, &base, end));
            self.entries[o.0].segments.push(Segment {
                start: t0,
                end: t0 + end,
                base,
                items,
                end_state,
            });
        }
        for o in ctx.introduced {
            self.show(o, t0);
        }
        for (o, a, b) in ctx.temps {
            self.entries[o.0].visible.push((t0 + a, t0 + b));
        }
        for (o, t) in ctx.shows {
            self.show(o, t0 + t);
        }
        for (o, t) in ctx.removes {
            self.hide(o, t0 + t);
        }
        // trackers (sorted by start)
        let mut trs = ctx.trackers;
        trs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for (k, off, dur, value, rate, f) in trs {
            let start = t0 + off;
            let from = self.tracker_value_at(k, start);
            let to = match &f {
                Some(f) => f(from, rate.eval(1.0)),
                None => value,
            };
            self.trackers[k.0].1.push(ScalarSeg {
                start,
                end: start + dur,
                from,
                to,
                rate,
                f,
            });
        }
        let mut cams = ctx.cameras;
        cams.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for (target, off, dur, rate) in cams {
            let start = t0 + off;
            let from = self.camera_at(start);
            self.camera_segs.push(CameraSeg {
                start,
                end: start + dur,
                from,
                to: target.apply(&from),
                rate,
            });
        }
        total
    }
}

type ItemFn = Arc<dyn Fn(&Mobject, f64, &Rate) -> Mobject + Send + Sync>;

#[derive(Default)]
struct Compile {
    items: Vec<(Obj, Vec<Item>)>,
    introduced: Vec<Obj>,
    removes: Vec<(Obj, f64)>,
    shows: Vec<(Obj, f64)>,
    temps: Vec<(Obj, f64, f64)>,
    trackers: Vec<(Tracker, f64, f64, f64, Rate, Option<ScalarFn>)>,
    cameras: Vec<(CameraTarget, f64, f64, Rate)>,
    total: f64,
}

impl Compile {
    fn push(&mut self, o: Obj, it: Item) {
        self.extend(it.off + it.dur);
        if let Some((_, v)) = self.items.iter_mut().find(|(x, _)| *x == o) {
            let pos = v.partition_point(|x| x.off <= it.off);
            v.insert(pos, it);
        } else {
            self.items.push((o, vec![it]));
        }
    }
    fn extend(&mut self, t: f64) {
        self.total = self.total.max(t);
    }
    fn introduce(&mut self, o: Obj, present: bool) {
        if !present && !self.introduced.contains(&o) {
            self.introduced.push(o);
        }
    }
    fn remove(&mut self, o: Obj, t: f64) {
        self.removes.push((o, t));
    }
    fn show_at(&mut self, o: Obj, t: f64) {
        self.shows.push((o, t));
    }
    fn temp(&mut self, o: Obj, a: f64, b: f64) {
        self.temps.push((o, a, b));
    }
    /// Merge a sub-compilation laid out over `[0, dur]` into this one at `off`,
    /// remapping its timeline: at outer progress `u` it shows inner time `warp(u) * dur`.
    fn merge_warped(&mut self, sub: Compile, off: f64, dur: f64, warp: WarpFn) {
        let d = dur.max(1e-12);
        // smallest u with warp(u) >= p (warp is monotone)
        let inv = {
            let warp = warp.clone();
            move |p: f64| -> f64 {
                if p <= 0.0 {
                    return 0.0;
                }
                let (mut lo, mut hi) = (0.0f64, 1.0f64);
                for _ in 0..50 {
                    let mid = 0.5 * (lo + hi);
                    if warp(mid) >= p - 1e-12 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                hi
            }
        };
        let map_t = |t: f64| off + inv((t / d).clamp(0.0, 1.0)) * d;
        // local alpha of an inner span [o, o + l] given outer local progress s over [s0, s1]
        let local = {
            let warp = warp.clone();
            move |o: f64, l: f64, s0: f64, s1: f64, s: f64| -> f64 {
                let u = (s0 + s * (s1 - s0)) / d;
                let it = warp(u.clamp(0.0, 1.0)) * d;
                if l <= 1e-12 {
                    if it >= o {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    ((it - o) / l).clamp(0.0, 1.0)
                }
            }
        };
        let local = Arc::new(local);
        for (o, items) in sub.items {
            for it in items {
                let s0 = map_t(it.off) - off;
                let s1 = (map_t(it.off + it.dur) - off).max(s0);
                let (io, il, f) = (it.off, it.dur, it.f.clone());
                let lc = local.clone();
                self.push(
                    o,
                    Item {
                        off: off + s0,
                        dur: s1 - s0,
                        rate: it.rate,
                        introduces: it.introduces,
                        f: Arc::new(move |m, a, r| f(m, lc(io, il, s0, s1, a), r)),
                    },
                );
            }
        }
        for o in sub.introduced {
            if !self.introduced.contains(&o) {
                self.introduced.push(o);
            }
        }
        for (o, t) in sub.removes {
            self.removes.push((o, map_t(t)));
        }
        for (o, t) in sub.shows {
            self.shows.push((o, map_t(t)));
        }
        for (o, a, b) in sub.temps {
            self.temps.push((o, map_t(a), map_t(b)));
        }
        let warp_rate = |io: f64, il: f64, rate: Rate| -> (f64, f64, Rate) {
            let s0 = map_t(io) - off;
            let s1 = (map_t(io + il) - off).max(s0);
            let lc = local.clone();
            let r = Rate::Custom(Arc::new(move |s| rate.eval(lc(io, il, s0, s1, s))));
            (off + s0, s1 - s0, r)
        };
        for (k, io, il, v, rate, f) in sub.trackers {
            let (o2, d2, r) = warp_rate(io, il, rate);
            self.trackers.push((k, o2, d2, v, r, f));
        }
        for (target, io, il, rate) in sub.cameras {
            let (o2, d2, r) = warp_rate(io, il, rate);
            self.cameras.push((target, o2, d2, r));
        }
        self.extend(off + dur);
    }
}

// ------------------------------------------------------------------ effects

/// Multiply the opacity of every node (and image) by `k`.
pub fn multiply_opacity(mut m: Mobject, k: f32) -> Mobject {
    m.for_each_mut(&mut |n| {
        n.style.opacity *= k;
        if let Some(i) = &mut n.image {
            i.opacity *= k;
        }
    });
    m
}

fn tint(mut m: Mobject, c: Color, k: f32) -> Mobject {
    m.for_each_mut(&mut |n| {
        n.style.fill.map_colors(&|x| x.mix(c, k));
        n.style.stroke.map_colors(&|x| x.mix(c, k));
    });
    m
}

fn create_partial(m: &Mobject, x: f64, lag: f64) -> Mobject {
    let n = m.leaf_count();
    let mut i = 0;
    let mut out = m.clone();
    out.for_each_mut(&mut |node| {
        if node.is_leaf_drawable() {
            let a = lagged_alpha(x, i, n, lag);
            i += 1;
            if !node.path.subpaths.is_empty() {
                node.path = if a <= 1e-6 {
                    Path::new()
                } else {
                    node.path.partial(0.0, a)
                };
                // fill fades in as the outline completes (avoids odd partial fills)
                node.style.fill_opacity *= (a * a) as f32;
            }
            if let Some(img) = &mut node.image {
                img.opacity *= a as f32;
            }
        }
    });
    out
}

/// manim's Write / DrawBorderThenFill for each leaf with lag.
fn write_effect(m: &Mobject, x: f64, lag: f64) -> Mobject {
    let n = m.leaf_count();
    let mut i = 0;
    let mut out = m.clone();
    out.for_each_mut(&mut |node| {
        if !node.is_leaf_drawable() {
            return;
        }
        let a = lagged_alpha(x, i, n, lag);
        i += 1;
        if let Some(img) = &mut node.image {
            img.opacity *= a as f32;
        }
        if node.path.subpaths.is_empty() {
            return;
        }
        let orig = node.style.clone();
        let outline_color = if orig.fill_opacity > 0.0 {
            orig.fill.color()
        } else {
            orig.stroke.color()
        };
        let outline_w = if orig.stroke_width > 0.0 {
            orig.stroke_width.max(1.0)
        } else {
            2.0
        };
        if a <= 1e-6 {
            node.path = Path::new();
        } else if a < 0.5 {
            let k = 2.0 * a;
            node.path = node.path.partial(0.0, k);
            node.style.stroke = Paint::Solid(outline_color);
            node.style.stroke_width = outline_w;
            node.style.stroke_opacity = 1.0;
            node.style.fill_opacity = 0.0;
        } else {
            let k = (2.0 * a - 1.0) as f32;
            node.style.fill_opacity = orig.fill_opacity * k;
            node.style.stroke = orig
                .stroke
                .lerp(&Paint::Solid(outline_color), 1.0 - k as f64);
            node.style.stroke_width = outline_w + (orig.stroke_width - outline_w) * k;
            node.style.stroke_opacity = 1.0 + (orig.stroke_opacity - 1.0) * k;
        }
    });
    out
}

fn arc_point(angle: f64) -> impl Fn(Vec2, Vec2, f64) -> Vec2 {
    move |p0, p1, t| {
        let v = p1 - p0;
        if v.length() < 1e-9 {
            return p0;
        }
        let center = p0 + v * 0.5 + v.perp() / (2.0 * (angle / 2.0).tan());
        center + (p0 - center).rotate(angle * t)
    }
}

/// Morph closure from `src` to `dst`.
fn morph_fn(src: &Mobject, dst: &Mobject, arc: f64) -> ItemFn {
    let (la, lb) = Mobject::align_for_transform(src, dst);
    let la = Arc::new(la);
    let lb = Arc::new(lb);
    let target = Arc::new(dst.clone());
    Arc::new(move |_m, a, r| {
        if a >= 1.0 {
            return (*target).clone();
        }
        let x = r.eval(a);
        let mut g = Mobject::empty();
        g.children = la
            .iter()
            .zip(lb.iter())
            .map(|(p, q)| {
                let mut leaf = Mobject::lerp_leaf(p, q, x);
                if arc.abs() > 1e-6 {
                    leaf.path = p.path.lerp_with(&q.path, x, &arc_point(arc));
                }
                leaf
            })
            .collect();
        g
    })
}

fn fade_transform_fn(src: &Mobject, dst: &Mobject) -> ItemFn {
    let src = Arc::new(src.clone());
    let dst = Arc::new(dst.clone());
    Arc::new(move |_m, a, r| {
        if a >= 1.0 {
            return (*dst).clone();
        }
        let x = r.eval(a);
        let sb = src.bounds_or_origin();
        let db = dst.bounds_or_origin();
        let fit = |m: &Mobject, from: Rect, to: Rect, k: f64| -> Mobject {
            let sx = lerp(1.0, to.width().max(1e-6) / from.width().max(1e-6), k);
            let sy = lerp(1.0, to.height().max(1e-6) / from.height().max(1e-6), k);
            let c = from.center().lerp(to.center(), k);
            m.clone().stretch(sx, sy).move_to(c)
        };
        let a1 = multiply_opacity(fit(&src, sb, db, x), 1.0 - x as f32);
        let b1 = multiply_opacity(fit(&dst, db, sb, 1.0 - x), x as f32);
        Mobject::group([a1, b1])
    })
}

fn matching_fn(src: &Mobject, dst: &Mobject) -> ItemFn {
    // Match top-level parts by tag; unmatched parts fade.
    let sparts: Vec<Mobject> = if src.children.is_empty() {
        vec![src.clone()]
    } else {
        src.children.clone()
    };
    let dparts: Vec<Mobject> = if dst.children.is_empty() {
        vec![dst.clone()]
    } else {
        dst.children.clone()
    };
    let mut used = vec![false; dparts.len()];
    let mut pairs = Vec::new();
    let mut fading_out = Vec::new();
    for s in &sparts {
        let key = s.tag.clone();
        let found = key.as_ref().and_then(|k| {
            (0..dparts.len()).find(|&j| !used[j] && dparts[j].tag.as_ref() == Some(k))
        });
        match found {
            Some(j) => {
                used[j] = true;
                pairs.push(Mobject::align_for_transform(s, &dparts[j]));
            }
            None => fading_out.push(s.clone()),
        }
    }
    let fading_in: Vec<Mobject> = dparts
        .iter()
        .enumerate()
        .filter(|(j, _)| !used[*j])
        .map(|(_, m)| m.clone())
        .collect();
    let pairs = Arc::new(pairs);
    let fo = Arc::new(fading_out);
    let fi = Arc::new(fading_in);
    let target = Arc::new(dst.clone());
    Arc::new(move |_m, a, r| {
        if a >= 1.0 {
            return (*target).clone();
        }
        let x = r.eval(a);
        let mut g = Mobject::empty();
        for (la, lb) in pairs.iter() {
            for (p, q) in la.iter().zip(lb) {
                g.push(Mobject::lerp_leaf(p, q, x));
            }
        }
        for m in fo.iter() {
            g.push(multiply_opacity(m.clone(), (1.0 - 2.0 * x).max(0.0) as f32));
        }
        for m in fi.iter() {
            g.push(multiply_opacity(m.clone(), (2.0 * x - 1.0).max(0.0) as f32));
        }
        g
    })
}

fn pointwise_fn(src: &Mobject, dst: &Mobject) -> ItemFn {
    // identical structure: interpolate all points directly
    let src = Arc::new(src.clone());
    let dst = Arc::new(dst.clone());
    Arc::new(move |_m, a, r| {
        if a >= 1.0 {
            return (*dst).clone();
        }
        let x = r.eval(a);
        lerp_same_structure(&src, &dst, x)
    })
}

fn lerp_same_structure(a: &Mobject, b: &Mobject, t: f64) -> Mobject {
    let mut out = a.clone();
    out.path = a.path.lerp(&b.path, t);
    out.style = a.style.lerp(&b.style, t);
    if let (Some(x), Some(y)) = (&a.image, &b.image) {
        let mut img = x.clone();
        img.origin = x.origin.lerp(y.origin, t);
        img.x_axis = x.x_axis.lerp(y.x_axis, t);
        img.y_axis = x.y_axis.lerp(y.y_axis, t);
        img.opacity = x.opacity + (y.opacity - x.opacity) * t as f32;
        out.image = Some(img);
    }
    out.children = a
        .children
        .iter()
        .zip(&b.children)
        .map(|(x, y)| lerp_same_structure(x, y, t))
        .collect();
    out
}

fn subdivide_for_warp(m: &Mobject) -> Mobject {
    let mut out = m.clone();
    out.for_each_mut(&mut |n| {
        let segs = n.path.num_segments();
        if segs > 0 && segs < 400 {
            let k = (64 / segs).clamp(1, 8);
            n.path = n.path.subdivided(k);
        }
    });
    out
}

#[derive(Clone)]
enum Step {
    Translate(Vec2),
    Scale(f64, f64, Vec2),
    Rotate(f64, Vec2),
}

/// `.animate()` closure: rigid steps are interpolated parametrically, styles blended;
/// anything else falls back to a morph.
fn animate_fn(snap: &Mobject, ops: &[Op]) -> ItemFn {
    let needs_morph = ops.iter().any(|o| matches!(o, Op::Apply(_)));
    let target = ops.iter().fold(snap.clone(), apply_op);
    if needs_morph {
        return morph_fn(snap, &target, 0.0);
    }
    let mut steps = Vec::new();
    let mut cur = snap.clone();
    for op in ops {
        let step = match op {
            Op::Shift(v) => Some(Step::Translate(*v)),
            Op::MoveTo(p) => Some(Step::Translate(*p - cur.center())),
            Op::MoveToAligned(p, e) => Some(Step::Translate(*p - cur.edge(*e))),
            Op::Scale(s) => Some(Step::Scale(*s, *s, cur.center())),
            Op::ScaleAbout(s, p) => Some(Step::Scale(*s, *s, *p)),
            Op::Stretch(x, y) => Some(Step::Scale(*x, *y, cur.center())),
            Op::Rotate(a) => Some(Step::Rotate(*a, cur.center())),
            Op::RotateAbout(a, p) => Some(Step::Rotate(*a, *p)),
            Op::SetWidth(w) => {
                let s = w / cur.width().max(1e-12);
                Some(Step::Scale(s, s, cur.center()))
            }
            Op::SetHeight(h) => {
                let s = h / cur.height().max(1e-12);
                Some(Step::Scale(s, s, cur.center()))
            }
            Op::NextTo(..) | Op::ToEdge(..) => {
                let moved = apply_op(cur.clone(), op);
                Some(Step::Translate(moved.center() - cur.center()))
            }
            _ => None,
        };
        cur = apply_op(cur, op);
        if let Some(s) = step {
            steps.push(s);
        }
    }
    let style_target = Arc::new(target);
    let snap = Arc::new(snap.clone());
    let steps = Arc::new(steps);
    Arc::new(move |_m, a, r| {
        let x = r.eval(a);
        let mut m = (*snap).clone();
        lerp_styles(&mut m, &style_target, x);
        for s in steps.iter() {
            m = match s {
                Step::Translate(v) => m.shift(*v * x),
                Step::Scale(sx, sy, p) => {
                    m.apply_affine(Affine::scale(lerp(1.0, *sx, x), lerp(1.0, *sy, x)).about(*p))
                }
                Step::Rotate(ang, p) => m.rotate_about(ang * x, *p),
            };
        }
        m
    })
}

fn lerp_styles(m: &mut Mobject, to: &Mobject, t: f64) {
    m.style = m.style.lerp(&to.style, t);
    if let (Some(a), Some(b)) = (&mut m.image, &to.image) {
        a.opacity += (b.opacity - a.opacity) * t as f32;
    }
    for (c, d) in m.children.iter_mut().zip(&to.children) {
        lerp_styles(c, d, t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::*;

    #[test]
    fn play_and_wait_advance_time() {
        let mut s = Scene::new();
        let c = s.register(circle(1.0));
        s.play(create(c));
        assert!((s.time() - 1.0).abs() < 1e-9);
        s.wait(0.5);
        s.play(c.animate().shift(RIGHT * 2.0).run_time(2.0));
        assert!((s.time() - 3.5).abs() < 1e-9);
        assert!((s.get(c).center() - v2(2.0, 0.0)).length() < 1e-6);
        // halfway through the move (smooth rate is symmetric)
        let mid = s.eval_obj(c, 2.5, 0);
        assert!((mid.center() - v2(1.0, 0.0)).length() < 1e-6);
        // not visible before it was created
        assert!(s.frame_mobjects(0.0).len() == 1);
    }

    #[test]
    fn fade_out_removes() {
        let mut s = Scene::new();
        let c = s.add(square(1.0));
        s.play(fade_out(c));
        assert!(!s.is_present(c));
        assert!(s.frame_mobjects(0.5).len() == 1);
        assert!(s.frame_mobjects(1.5).is_empty());
    }

    #[test]
    fn transforms_and_trackers() {
        let mut s = Scene::new();
        let a = s.add(square(1.0));
        let b = s.register(circle(1.0).shift(RIGHT * 3.0));
        s.play(replacement_transform(a, b));
        assert!(!s.is_present(a));
        assert!(s.is_present(b));
        let k = s.tracker(0.0);
        let d = s.always_redraw(move |ctx| circle(0.1 + ctx.value(k)));
        s.play(k.to(1.0));
        assert!((s.value(k) - 1.0).abs() < 1e-9);
        assert!((s.get(d).width() - 2.2).abs() < 1e-6);
        // lagged succession
        let x = s.register(dot(Vec2::ZERO));
        let y = s.register(dot(RIGHT));
        let dur = s.play(succession(vec![fade_in(x), fade_in(y)]));
        assert!((dur - 2.0).abs() < 1e-9);
    }

    #[test]
    fn camera_animation() {
        let mut s = Scene::new();
        s.play(zoom_to(v2(1.0, 1.0), 4.0));
        let c = s.camera_at(s.time());
        assert!((c.frame_height - 4.0).abs() < 1e-9);
        assert!((c.center - v2(1.0, 1.0)).length() < 1e-9);
    }

    #[test]
    fn rotate_is_rigid() {
        let mut s = Scene::new();
        let sq = s.add(square(2.0));
        s.play(rotate(sq, PI / 2.0).rate(Rate::Linear));
        let mid = s.eval_obj(sq, 0.5, 0);
        // a rigidly rotated square keeps its side length (diagonal bounds at 45°)
        assert!((mid.width() - 2.0 * 2f64.sqrt()).abs() < 1e-3);
    }

    #[test]
    fn background_play_overlaps_and_extends_duration() {
        let mut s = Scene::new();
        let k = s.tracker(0.0);
        s.play_background(4.0, k.to(10.0).rate(Rate::Linear));
        assert_eq!(s.time(), 0.0);
        s.wait(1.0);
        assert!((s.tracker_value_at(k, 2.0) - 5.0).abs() < 1e-6);
        assert!(s.duration() >= 4.0);
    }
}
