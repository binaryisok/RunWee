//! Render anims_bms lessons (JavaScript canvas simulations driven by narrated
//! timelines) as RunWee videos, without porting any lesson code.
//!
//! The lesson's own `index.js` / `simulations.js` / `mathOverlay.js` run in an
//! embedded QuickJS engine. The intro and lecture timelines are replayed with the
//! same rules as the browser app (preset switch and reset per step, mode, slider
//! tweens chained through callbacks, `step(dt)` every frame), and every frame's
//! `draw()` is recorded and replayed through [`runwee::canvas2d::Canvas2D`] as
//! vector graphics. Narration comes from the lesson's audio files when present,
//! otherwise from the TTS command in `RUNWEE_TTS` (or timing estimates). Math
//! overlay lines appear on a board as the narration reaches them.

mod display;
mod js;

pub use display::{parse as parse_commands, replay, Cmd};
pub use js::JsLesson;

use anyhow::Result;
use runwee::background::Background;
use runwee::canvas2d::Canvas2D;
use runwee::captions::CaptionLook;
use runwee::prelude::*;
use runwee::voice::Voiceover;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct StepInfo {
    pub text: String,
    pub preset: String,
    pub mode: String,
    #[serde(rename = "audioId")]
    pub audio_id: Option<String>,
    #[serde(rename = "audioDir")]
    pub audio_dir: String,
    #[serde(rename = "pauseAfterMs")]
    pub pause_after_ms: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PresetInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "constraintEq")]
    pub constraint_eq: String,
    pub tip: String,
    pub question: String,
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Derivation {
    pub title: String,
    pub math: String,
    pub desc: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LessonInfo {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub intro: Vec<StepInfo>,
    pub lecture: Vec<StepInfo>,
    pub presets: Vec<PresetInfo>,
    pub derivations: HashMap<String, Vec<Derivation>>,
}

#[derive(Debug, Clone, Deserialize)]
struct OverlayLine {
    wi: usize,
    text: String,
    #[serde(rename = "type")]
    kind: String,
    hl: bool,
}

/// Which timelines to include.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parts {
    Intro,
    Lecture,
    Both,
}

impl Parts {
    pub fn parse(s: &str) -> Option<Parts> {
        Some(match s {
            "intro" => Parts::Intro,
            "lecture" | "examples" => Parts::Lecture,
            "all" | "both" => Parts::Both,
            _ => return None,
        })
    }
}

pub struct LessonOptions {
    /// The lesson folder (containing index.js) or the index.js file itself.
    pub path: PathBuf,
    /// Repository root used to resolve `audioDir` (defaults to the folder that
    /// contains `topics/`).
    pub repo_root: Option<PathBuf>,
    pub parts: Parts,
    /// Frame rate of the output — the simulation is stepped at exactly this rate.
    pub fps: f64,
    /// Seconds of silence after each step (the app waits `pauseAfterMs`).
    pub max_pause: f64,
    /// Only the first `n` steps of each part (for quick previews).
    pub limit_steps: Option<usize>,
}

impl LessonOptions {
    pub fn new(path: impl Into<PathBuf>, fps: f64) -> Self {
        Self {
            path: path.into(),
            repo_root: None,
            parts: Parts::Both,
            fps,
            max_pause: 0.8,
            limit_steps: None,
        }
    }
}

/// Read the lesson manifest without rendering anything.
pub fn inspect(path: &Path) -> Result<LessonInfo> {
    let lesson = JsLesson::load(&index_path(path))?;
    Ok(serde_json::from_str(&lesson.info()?)?)
}

fn index_path(p: &Path) -> PathBuf {
    if p.is_dir() {
        p.join("index.js")
    } else {
        p.to_path_buf()
    }
}

fn find_repo_root(index: &Path) -> Option<PathBuf> {
    let mut cur = index.parent();
    while let Some(d) = cur {
        if d.join("topics").is_dir() {
            return Some(d.to_path_buf());
        }
        cur = d.parent();
    }
    None
}

// Layout (scene units, 16:9 frame 14.22 × 8).
/// Recorded canvas commands per output frame (shared between identical frames).
type FrameCmds = Vec<Option<Arc<Vec<Cmd>>>>;

const CANVAS: (f64, f64, f64, f64) = (-7.0, -2.35, 3.2, 2.75); // x0, y0, x1, y1 (2:1)
const BOARD_X0: f64 = 3.5;
const BOARD_X1: f64 = 6.95;
const BOARD_TOP: f64 = 2.85;
const BOARD_BOTTOM: f64 = -2.45;

fn ink() -> Color {
    Color::parse("#0f172a").unwrap()
}
fn muted() -> Color {
    Color::parse("#475569").unwrap()
}
fn accent() -> Color {
    Color::parse("#4f46e5").unwrap()
}
fn good() -> Color {
    Color::parse("#047857").unwrap()
}

/// Build the lesson video as a [`Scene`].
pub fn build(opts: &LessonOptions) -> Result<Scene> {
    let index = index_path(&opts.path);
    let lesson = JsLesson::load(&index)?;
    let info: LessonInfo = serde_json::from_str(&lesson.info()?)?;
    let repo = opts
        .repo_root
        .clone()
        .or_else(|| find_repo_root(&std::fs::canonicalize(&index).ok()?))
        .unwrap_or_else(|| PathBuf::from("."));
    let fps = opts.fps;

    let mut s = Scene::new();
    s.set_background(Background::Solid(Color::parse("#f6f7fb").unwrap()));
    s.captions.look = CaptionLook::default();

    // Recorded canvas frames by frame index; `None` = no simulation on screen.
    let mut frames: Vec<Option<Arc<Vec<Cmd>>>> = Vec::new();
    let put = |frames: &mut Vec<Option<Arc<Vec<Cmd>>>>, idx: usize, v: Option<Arc<Vec<Cmd>>>| {
        if frames.len() <= idx {
            frames.resize(idx + 1, None);
        }
        frames[idx] = v;
    };

    // The simulation layer exists from t = 0; its frames are filled in below.
    let frame_store: Arc<std::sync::OnceLock<FrameCmds>> = Arc::new(std::sync::OnceLock::new());
    {
        let store = frame_store.clone();
        let (cx0, cy0, cx1, cy1) = CANVAS;
        let rect = Rect::new(v2(cx0, cy0), v2(cx1, cy1));
        let layer = s.always_redraw(move |ctx| {
            let Some(frames) = store.get() else {
                return Mobject::empty();
            };
            let idx = (ctx.t * fps).round().max(0.0) as usize;
            let Some(Some(cmds)) = frames.get(idx.min(frames.len().saturating_sub(1))) else {
                return Mobject::empty();
            };
            let mut c = Canvas2D::new(800.0, 400.0, rect);
            display::replay(cmds, &mut c);
            c.finish()
        });
        // Above the white canvas panel that is added later.
        s.set_z(layer, 5.0);
    }

    // ── title card ──
    let title = s.register(bold_text(&info.title, 0.8, ink()).shift(UP * 0.45));
    let sub = s.register(text_sized(&info.subtitle, 0.32, muted()).shift(DOWN * 0.4));
    s.play([write(title), fade_in_shift(sub, UP)]);
    s.wait(1.4);
    s.play([fade_out(title), fade_out(sub)]);

    let parts: Vec<(&str, &str, &Vec<StepInfo>)> = match opts.parts {
        Parts::Intro => vec![("intro", "Introduction", &info.intro)],
        Parts::Lecture => vec![("lecture", "Concepts & examples", &info.lecture)],
        Parts::Both => vec![
            ("intro", "Introduction", &info.intro),
            ("lecture", "Concepts & examples", &info.lecture),
        ],
    };

    // Panels behind the canvas and the board.
    let (cx0, cy0, cx1, cy1) = CANVAS;
    let canvas_panel = rounded_rectangle(cx1 - cx0 + 0.2, cy1 - cy0 + 0.2, 0.15)
        .move_to(v2((cx0 + cx1) / 2.0, (cy0 + cy1) / 2.0))
        .fill(WHITE, 1.0)
        .stroke(Color::parse("#e2e8f0").unwrap(), 1.5);
    let board_panel = rounded_rectangle(
        BOARD_X1 - BOARD_X0 + 0.2,
        BOARD_TOP - BOARD_BOTTOM + 0.2,
        0.15,
    )
    .move_to(v2(
        (BOARD_X0 + BOARD_X1) / 2.0,
        (BOARD_TOP + BOARD_BOTTOM) / 2.0,
    ))
    .fill(WHITE, 1.0)
    .stroke(Color::parse("#e2e8f0").unwrap(), 1.5);
    let mut panels_shown = false;
    let mut hidden: Vec<(f64, f64)> = vec![];
    let mut header: Vec<Obj> = vec![];
    let mut last_header = String::new();

    for (part, part_title, steps) in parts {
        if steps.is_empty() {
            continue;
        }
        // part card (the simulation is hidden meanwhile)
        let card_start = s.time();
        let pc = s.register(bold_text(part_title, 0.6, accent()));
        s.play(fade_in_shift(pc, UP));
        s.wait(0.8);
        s.play(fade_out(pc));
        hidden.push((card_start, s.time()));
        if !panels_shown {
            let a = s.register(canvas_panel.clone());
            let b = s.register(board_panel.clone());
            s.play([fade_in(a), fade_in(b)]);
            panels_shown = true;
        }
        let n_steps = opts.limit_steps.unwrap_or(usize::MAX).min(steps.len());
        for (i, step) in steps.iter().enumerate().take(n_steps) {
            let preset = info
                .presets
                .iter()
                .find(|p| p.id == step.preset)
                .cloned()
                .unwrap_or_else(|| info.presets[0].clone());

            // Header: lesson · part, preset name, constraint chip.
            let key = format!("{part}|{}", preset.id);
            if key != last_header {
                if !header.is_empty() {
                    let outs: Vec<Animation> = header.iter().map(|o| fade_out(*o)).collect();
                    s.play_for(0.3, outs);
                }
                header = header_objects(&mut s, &info.title, part_title, &preset);
                let ins: Vec<Animation> = header.iter().map(|o| fade_in(*o)).collect();
                s.play_for(0.4, ins);
                last_header = key;
            }

            // Narration: the lesson's own audio file if it exists, else TTS/estimate.
            let mut vo = Voiceover::new(&step.text);
            if let Some(id) = &step.audio_id {
                for ext in ["mp3", "wav"] {
                    let p = repo.join(&step.audio_dir).join(format!("audio_{id}.{ext}"));
                    if p.is_file() {
                        vo = vo.audio(p);
                        break;
                    }
                }
            }
            let v = s.begin_voiceover(&vo);
            let pause = (step.pause_after_ms / 1000.0).clamp(0.2, opts.max_pause);
            let step_len = v.duration + pause;

            // Simulation: step the lesson frame by frame and record draw().
            lesson.begin_step(part, i)?;
            let first = (v.start * fps).round() as usize;
            let n = (step_len * fps).round().max(1.0) as usize;
            let mut prev_json = String::new();
            let mut prev: Option<Arc<Vec<Cmd>>> = None;
            for k in 0..n {
                let progress = ((k as f64 / fps) / v.duration.max(1e-6)).min(1.0);
                let json = lesson.tick(1.0 / fps, progress)?;
                let cmds = if json == prev_json {
                    prev.clone()
                } else {
                    let c = Arc::new(display::parse(&json));
                    prev_json = json;
                    prev = Some(c.clone());
                    Some(c)
                };
                put(&mut frames, first + k, cmds);
            }

            // Board: the step's math overlay, else the preset's derivation.
            let nwords = step.text.split_whitespace().count().max(1);
            let step_id = step.audio_id.clone().unwrap_or_else(|| (i + 1).to_string());
            let lines: Vec<OverlayLine> =
                serde_json::from_str(&lesson.overlay(&step_id, (nwords * 3).max(160))?)
                    .unwrap_or_default();
            let mut items: Vec<(f64, Mobject, bool)> = vec![];
            if !preset.question.is_empty() {
                items.push((0.0, question_box(&preset.question), false));
            }
            if !lines.is_empty() {
                let max_wi = lines.iter().map(|l| l.wi).max().unwrap_or(0);
                let denom = (nwords.max(max_wi + 4)) as f64;
                for l in &lines {
                    let frac = (l.wi as f64 / denom).min(0.95);
                    items.push((frac, overlay_mobject(l), l.hl));
                }
            } else if let Some(ds) = info.derivations.get(&preset.id).or_else(|| {
                let pi = info.presets.iter().position(|p| p.id == preset.id)?;
                info.derivations.get(&format!("#{pi}"))
            }) {
                let n = ds.len().max(1) as f64;
                for (k, d) in ds.iter().enumerate() {
                    let frac = 0.08 + 0.8 * k as f64 / n;
                    items.push((frac, derivation_mobject(d), false));
                }
            }
            let shown = place_board(&mut s, items, v.start, v.duration);
            s.end_voiceover(&v);
            s.wait_until(v.start + step_len - 0.3);
            if !shown.is_empty() {
                let outs: Vec<Animation> = shown.iter().map(|o| fade_out(*o)).collect();
                s.play_for(0.3, outs);
            }
            s.wait_until(v.start + step_len);
        }
    }
    s.wait(0.6);

    // Hold the last recorded frame across gaps (header changes, board fades).
    let mut last: Option<Arc<Vec<Cmd>>> = None;
    let mut started = false;
    for f in frames.iter_mut() {
        if f.is_some() {
            last = f.clone();
            started = true;
        } else if started {
            *f = last.clone();
        }
    }
    for (a, b) in hidden {
        let (i0, i1) = ((a * fps).floor() as usize, (b * fps).ceil() as usize);
        let n = frames.len();
        for f in frames.iter_mut().take(i1.min(n)).skip(i0) {
            *f = None;
        }
    }
    let _ = frame_store.set(frames);
    Ok(s)
}

/// Move `m` so its top-left corner sits at (x, y).
fn place_tl(m: Mobject, x: f64, y: f64) -> Mobject {
    let d = v2(x - m.left().x, y - m.top().y);
    m.shift(d)
}

fn header_objects(s: &mut Scene, title: &str, part: &str, p: &PresetInfo) -> Vec<Obj> {
    let mut out = vec![];
    let kicker = text_sized(
        &format!("{}  ·  {}", title.to_uppercase(), part),
        0.2,
        muted(),
    );
    let kicker = place_tl(kicker, -7.0, 3.75);
    out.push(s.register(kicker));
    let name = bold_text(&p.name, 0.4, ink());
    let name = place_tl(name, -7.0, 3.45);
    out.push(s.register(name));
    if !p.constraint_eq.is_empty() {
        let t = text_sized(&p.constraint_eq, 0.27, accent());
        let w = t.width() + 0.5;
        let chip = group([
            rounded_rectangle(w, 0.46, 0.12)
                .fill(Color::parse("#eef2ff").unwrap(), 1.0)
                .stroke(Color::parse("#c7d2fe").unwrap(), 1.2),
            t,
        ]);
        let d = v2(6.95 - chip.right().x, 3.62 - chip.top().y);
        let chip = chip.shift(d);
        out.push(s.register(chip));
    }
    out
}

fn question_box(q: &str) -> Mobject {
    let plain = q.replace('$', "");
    let body = paragraph(&plain, BOARD_X1 - BOARD_X0 - 0.4, 0.2, Align::Left)
        .color(Color::parse("#92400e").unwrap());
    let w = BOARD_X1 - BOARD_X0;
    let h = body.height() + 0.3;
    let bx = rounded_rectangle(w, h, 0.1)
        .fill(Color::parse("#fef3c7").unwrap(), 1.0)
        .stroke(Color::parse("#f59e0b").unwrap(), 1.2);
    let body = body.move_to(bx.center());
    group([bx, body])
}

fn overlay_mobject(l: &OverlayLine) -> Mobject {
    let w = BOARD_X1 - BOARD_X0;
    match l.kind.as_str() {
        "title" => paragraph(&l.text, w, 0.27, Align::Left).color(accent()),
        "math" => {
            let col = if l.hl { good() } else { ink() };
            let m = tex_sized(&l.text, 0.4, col);
            if m.width() < 1e-6 {
                paragraph(&l.text, w, 0.22, Align::Left).color(col)
            } else {
                m
            }
        }
        _ => {
            let col = if l.hl { good() } else { muted() };
            paragraph(&l.text, w, 0.21, Align::Left).color(col)
        }
    }
}

fn derivation_mobject(d: &Derivation) -> Mobject {
    let w = BOARD_X1 - BOARD_X0;
    let mut parts = vec![paragraph(&d.title, w, 0.22, Align::Left).color(accent())];
    if !d.math.is_empty() {
        parts.push(paragraph(&d.math, w, 0.24, Align::Left).color(ink()));
    }
    if !d.desc.is_empty() {
        parts.push(paragraph(&d.desc, w, 0.18, Align::Left).color(muted()));
    }
    Mobject::group(parts).arrange_aligned(DOWN, 0.08, LEFT)
}

/// Lay the items out top-down (scaled to fit) and fade each in at its time.
fn place_board(s: &mut Scene, items: Vec<(f64, Mobject, bool)>, start: f64, dur: f64) -> Vec<Obj> {
    if items.is_empty() {
        return vec![];
    }
    let gap = 0.16;
    let avail = BOARD_TOP - BOARD_BOTTOM - 0.25;
    let width = BOARD_X1 - BOARD_X0;
    let mut mobs: Vec<(f64, Mobject, bool)> = items
        .into_iter()
        .map(|(t, m, hl)| {
            let m = if m.width() > width {
                m.set_width(width)
            } else {
                m
            };
            (t, m, hl)
        })
        .collect();
    let total: f64 = mobs.iter().map(|(_, m, _)| m.height() + gap).sum();
    let k = if total > avail { avail / total } else { 1.0 };
    let mut y = BOARD_TOP - 0.1;
    let mut out = vec![];
    for (t, m, hl) in mobs.iter_mut() {
        let mm = std::mem::take(m).scale(k);
        let mm = place_tl(mm, BOARD_X0, y);
        y -= mm.height() + gap * k;
        s.wait_until(start + dur * *t);
        let boxed = if *hl {
            Some(surrounding_rectangle(&mm, 0.08).stroke(good(), 2.0))
        } else {
            None
        };
        let o = s.register(mm);
        s.play_for(0.35, fade_in_shift(o, UP * 0.3));
        out.push(o);
        if let Some(b) = boxed {
            let bo = s.register(b);
            s.play_for(0.3, create(bo));
            out.push(bo);
        }
    }
    out
}
