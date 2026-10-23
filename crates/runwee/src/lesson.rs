//! Narrated-lesson helpers used by `.rhai` scripts and `runwee render --audio-json`.
//!
//! Clip timing uses the decoded sample length (ffmpeg for mp3, native for wav) so the
//! scene timeline matches the samples that get muxed. [`probe_duration`](crate::audio::probe_duration)
//! is the ffprobe fast path exposed to scripts as `audio_duration`.

use crate::audio::{probe_duration, AudioData};
use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::scene::Scene;
use crate::shapes::*;
use crate::tex::tex_sized;
use crate::text::{bold_text, text_sized};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// One decoded clip on a step timeline. `start` is seconds from the start of the group.
#[derive(Clone)]
pub struct ClipSpan {
    pub path: PathBuf,
    pub start: f64,
    pub duration: f64,
    pub data: Arc<AudioData>,
}

impl ClipSpan {
    pub fn end(&self) -> f64 {
        self.start + self.duration
    }
}

/// End of the last clip, or 0 when `clips` is empty.
pub fn clips_end(clips: &[ClipSpan]) -> f64 {
    clips.last().map(ClipSpan::end).unwrap_or(0.0)
}

/// Decode each file and lay the clips end to end.
pub fn load_clips(paths: &[PathBuf]) -> Result<Vec<ClipSpan>> {
    let mut start = 0.0;
    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        let data =
            Arc::new(AudioData::load(path).with_context(|| format!("loading {}", path.display()))?);
        let duration = data.duration();
        out.push(ClipSpan {
            path: path.clone(),
            start,
            duration,
            data,
        });
        start += duration;
    }
    Ok(out)
}

/// Place `clips` on `scene` beginning at absolute time `at`.
pub fn place_clips(scene: &mut Scene, clips: &[ClipSpan], at: f64) {
    for clip in clips {
        scene.add_sound_at(clip.data.clone(), at + clip.start, 1.0);
    }
}

/// Place clips at t = 0 and extend the scene so its duration is at least the audio.
/// Returns that audio length. A scene that already ran longer is left long and a warning
/// is printed — the timeline is never shortened under existing animation.
pub fn apply_clips(scene: &mut Scene, paths: &[PathBuf]) -> Result<f64> {
    if paths.is_empty() {
        return Ok(0.0);
    }
    let clips = load_clips(paths)?;
    let total = clips_end(&clips);
    place_clips(scene, &clips, 0.0);
    let visual = scene.time();
    if visual > total + 1.0 / 30.0 {
        eprintln!(
            "warning: scene runs {visual:.3}s but its audio is {total:.3}s — the video will be longer than the clips"
        );
    }
    scene.seek(total);
    Ok(total)
}

/// Why [`attach_audio_plan`] did or did not change the scenes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attach {
    Applied,
    SkippedExistingAudio,
    Empty,
}

/// One narration step from `audio.json`.
#[derive(Clone, Debug)]
pub struct StepAudio {
    pub id: String,
    pub clips: Vec<PathBuf>,
}

/// Parse an audio plan.
///
/// Accepted shapes (clip order is preserved):
/// - `{ "steps": [ { "id": "intro_1", "clips": ["audio/a.mp3"] } ] }`
/// - `[ { "id": "intro_1", "clips": ["audio/a.mp3"] } ]`
/// - `{ "intro_1": ["audio/a.mp3"], "1": ["audio/b.mp3"] }`
/// - `{ "clips": ["audio/a.mp3", "audio/b.mp3"] }`
/// - `["audio/a.mp3", "audio/b.mp3"]` (one step)
///
/// A clip is a string or `{ "file" | "path" | "src": "..." }`. Relative paths are
/// resolved against `base` (the JSON file's directory), then the process working
/// directory, then `RUNWEE_SCRIPT_DIR`.
pub fn parse_audio_plan(text: &str, base: &Path) -> Result<Vec<StepAudio>> {
    let value: Value = serde_json::from_str(text).context("audio.json is not valid JSON")?;
    parse_audio_value(&value, base)
}

fn parse_audio_value(value: &Value, base: &Path) -> Result<Vec<StepAudio>> {
    match value {
        Value::Array(items) => {
            if items
                .iter()
                .all(|v| v.is_string() || clip_object(v).is_some())
                && items.iter().any(|v| !v.is_object())
            {
                let clips = string_list(items, base)?;
                return Ok(vec![StepAudio {
                    id: "all".into(),
                    clips,
                }]);
            }
            if items.iter().all(|v| v.is_object()) {
                return items
                    .iter()
                    .enumerate()
                    .map(|(i, v)| step_from_object(v, i + 1, base))
                    .collect();
            }
            // Mixed array: every element is a clip path.
            let clips = string_list(items, base)?;
            Ok(vec![StepAudio {
                id: "all".into(),
                clips,
            }])
        }
        Value::Object(map) => {
            if let Some(steps) = map.get("steps").and_then(Value::as_array) {
                return steps
                    .iter()
                    .enumerate()
                    .map(|(i, v)| step_from_object(v, i + 1, base))
                    .collect();
            }
            if let Some(clips) = first_clip_array(map) {
                return Ok(vec![StepAudio {
                    id: "all".into(),
                    clips: string_list(clips, base)?,
                }]);
            }
            let mut steps = Vec::new();
            for (key, val) in map {
                if key == "steps" || key == "clips" || key == "audio" || key == "files" {
                    continue;
                }
                if let Some(arr) = val.as_array() {
                    if arr
                        .iter()
                        .all(|v| v.is_string() || clip_object(v).is_some())
                    {
                        steps.push(StepAudio {
                            id: key.clone(),
                            clips: string_list(arr, base)?,
                        });
                    }
                }
            }
            if steps.is_empty() {
                bail!(
                    "audio.json object has no steps. Use {{\"steps\":[{{\"id\",\"clips\"}}]}} or {{\"stepId\":[\"audio/a.mp3\"]}}"
                );
            }
            Ok(steps)
        }
        _ => bail!("audio.json must be an object or an array"),
    }
}

fn first_clip_array(map: &serde_json::Map<String, Value>) -> Option<&Vec<Value>> {
    for key in ["clips", "audio", "files"] {
        if let Some(arr) = map.get(key).and_then(Value::as_array) {
            if arr
                .iter()
                .all(|v| v.is_string() || clip_object(v).is_some())
            {
                return Some(arr);
            }
        }
    }
    None
}

fn step_from_object(value: &Value, index: usize, base: &Path) -> Result<StepAudio> {
    let obj = value
        .as_object()
        .with_context(|| format!("audio step {index} is not an object"))?;
    let id = match obj.get("id").or_else(|| obj.get("step")) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => index.to_string(),
    };
    let arr = first_clip_array(obj)
        .with_context(|| format!("audio step '{id}' needs a clips array of paths"))?;
    Ok(StepAudio {
        id,
        clips: string_list(arr, base)?,
    })
}

fn clip_object(value: &Value) -> Option<&str> {
    let obj = value.as_object()?;
    for key in ["file", "path", "src"] {
        if let Some(s) = obj.get(key).and_then(Value::as_str) {
            return Some(s);
        }
    }
    None
}

fn string_list(items: &[Value], base: &Path) -> Result<Vec<PathBuf>> {
    items
        .iter()
        .map(|v| match v {
            Value::String(s) => Ok(resolve_clip(base, s)),
            other => clip_object(other)
                .map(|s| resolve_clip(base, s))
                .with_context(|| format!("not a clip path: {other}")),
        })
        .collect()
}

/// Resolve a clip path. Existing files win (cwd-relative scripts keep working).
pub fn resolve_clip(base: &Path, path: &str) -> PathBuf {
    let raw = PathBuf::from(path);
    if raw.is_absolute() || raw.exists() {
        return raw;
    }
    let beside = base.join(&raw);
    if beside.exists() {
        return beside;
    }
    if let Some(dir) = std::env::var_os("RUNWEE_SCRIPT_DIR") {
        let script = PathBuf::from(dir).join(&raw);
        if script.exists() {
            return script;
        }
    }
    beside
}

/// Duration via ffprobe, falling back to a full decode.
pub fn media_duration(path: &Path) -> Result<f64> {
    match probe_duration(path) {
        Ok(d) if d.is_finite() && d > 0.0 => Ok(d),
        _ => Ok(AudioData::load(path)?.duration()),
    }
}

/// Put `steps` onto `scenes`.
///
/// - One scene: every clip, in order, from t = 0, and the scene is extended to the total.
/// - Scene count equal to step count: each step's clips start at t = 0 of that scene.
/// - Otherwise steps and scenes are paired in order; leftover steps are appended to the
///   last scene, and leftover scenes stay silent.
///
/// If any scene already has audio, nothing is added ([`Attach::SkippedExistingAudio`]).
pub fn attach_audio_plan(scenes: &mut [&mut Scene], steps: &[StepAudio]) -> Result<Attach> {
    if scenes.is_empty() || steps.is_empty() {
        return Ok(Attach::Empty);
    }
    if scenes.iter().any(|s| !s.audio.is_empty()) {
        return Ok(Attach::SkippedExistingAudio);
    }
    let n = scenes.len();
    let m = steps.len();
    if n == m {
        for (scene, step) in scenes.iter_mut().zip(steps.iter()) {
            apply_clips(scene, &step.clips)?;
        }
    } else if n == 1 {
        let flat: Vec<PathBuf> = steps.iter().flat_map(|s| s.clips.clone()).collect();
        apply_clips(scenes[0], &flat)?;
    } else {
        eprintln!("runwee: {n} scenes and {m} audio steps; pairing in order");
        if m < n {
            for (scene, step) in scenes.iter_mut().zip(steps.iter()) {
                apply_clips(scene, &step.clips)?;
            }
        } else {
            for i in 0..n - 1 {
                apply_clips(scenes[i], &steps[i].clips)?;
            }
            let flat: Vec<PathBuf> = steps[n - 1..]
                .iter()
                .flat_map(|s| s.clips.clone())
                .collect();
            apply_clips(scenes[n - 1], &flat)?;
        }
    }
    Ok(Attach::Applied)
}

/// One row of a math board: `title`, `text`, `math` / `tex` / `eq`, or `highlight`.
/// The row is left unpositioned; the script layer stacks it.
pub fn math_line(kind: &str, content: &str, max_width: f64) -> Mobject {
    let content = if content.is_empty() { " " } else { content };
    let kind = kind.to_ascii_lowercase();
    let mut row = match kind.as_str() {
        "title" => bold_text(content, 0.40, WHITE),
        "math" | "tex" | "eq" => tex_sized(content, 0.46, WHITE),
        "highlight" | "hl" => {
            let label = tex_sized(content, 0.46, GOLD);
            let bar = rectangle(label.width().max(0.4), 0.045)
                .no_stroke()
                .fill(GOLD, 1.0)
                .next_to(&label, DOWN, 0.05);
            Mobject::group([label, bar])
        }
        _ => text_sized(content, 0.32, WHITE),
    };
    if max_width > 0.05 && row.width() > max_width {
        row = row.set_width(max_width);
    }
    row
}

/// Frosted card behind a math board. Dark neutrals flip to a light panel on light themes,
/// and white type flips to ink, so the same card reads on `theme("lesson")` and on dark themes.
pub fn math_card(width: f64, height: f64) -> Mobject {
    rounded_rectangle(width.max(0.4), height.max(0.4), 0.16)
        .fill(GRAY_D, 0.96)
        .stroke(GOLD_E, 2.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::field_arrow_grid;

    #[test]
    fn parses_step_objects_flat_lists_and_maps() {
        let base = Path::new("/lessons/demo");
        let steps = parse_audio_plan(
            r#"{"steps":[{"id":"intro_1","clips":["audio/a.mp3",{"file":"audio/b.mp3"}]},{"id":2,"audio":["audio/c.mp3"]}]}"#,
            base,
        )
        .unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].id, "intro_1");
        assert_eq!(steps[0].clips.len(), 2);
        assert!(steps[0].clips[0].ends_with("audio/a.mp3"));
        assert_eq!(steps[1].id, "2");

        let flat = parse_audio_plan(r#"["audio/a.mp3","audio/b.mp3"]"#, base).unwrap();
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].clips.len(), 2);

        let map = parse_audio_plan(
            r#"{"intro_1":["audio/a.mp3"],"lecture_1":["audio/b.mp3"]}"#,
            base,
        )
        .unwrap();
        assert_eq!(map[0].id, "intro_1");
        assert_eq!(map[1].id, "lecture_1");
    }

    #[test]
    fn decoded_clips_set_the_scene_duration() {
        let dir = std::env::temp_dir().join("runwee-lesson-clips");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.wav");
        let b = dir.join("b.wav");
        AudioData::silence(0.50).save_wav(&a).unwrap();
        AudioData::silence(0.25).save_wav(&b).unwrap();
        let probed = probe_duration(&a).unwrap();
        assert!((probed - 0.50).abs() < 0.02, "{probed}");

        let mut scene = Scene::new();
        let total = apply_clips(&mut scene, &[a, b]).unwrap();
        assert!((total - 0.75).abs() < 1e-3, "{total}");
        assert!(
            (scene.duration() - 0.75).abs() < 1e-3,
            "{}",
            scene.duration()
        );
        assert!((scene.audio.end_time() - 0.75).abs() < 1e-3);

        let mut occupied = Scene::new();
        occupied.add_sound_at(Arc::new(AudioData::silence(0.1)), 0.0, 1.0);
        let mut scenes: [&mut Scene; 1] = [&mut occupied];
        let plan = vec![StepAudio {
            id: "x".into(),
            clips: vec![dir.join("a.wav")],
        }];
        assert_eq!(
            attach_audio_plan(&mut scenes, &plan).unwrap(),
            Attach::SkippedExistingAudio
        );
    }

    #[test]
    fn field_arrows_and_math_line_have_extent() {
        let arrows = field_arrow_grid(
            &[(Vec2::ZERO, 1.0), (v2(2.0, 0.0), -1.0)],
            -1.0,
            3.0,
            -1.0,
            1.0,
            5,
            3,
            BLUE_B,
        );
        assert!(arrows.width() > 0.5, "{}", arrows.width());
        let row = math_line("math", "q = ne", 4.0);
        assert!(row.width() > 0.2 && row.width() <= 4.0 + 1e-6);
        let card = math_card(4.0, 3.0);
        assert!(card.width() > 3.0 && card.height() > 2.0);
    }
}
