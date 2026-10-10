//! Narration & audio sync on `Scene` (manim-voiceover style).
//!
//! ```ignore
//! scene.voiceover(Voiceover::new("The area of a circle is <bookmark mark='f'/> pi r squared")
//!     .audio("vo/area.wav"), |s, vo| {
//!     s.play(create(circle));
//!     s.wait_until(vo.bookmark("f"));   // sync to the spoken word
//!     s.play(write(formula));
//! });
//! ```
//! The block lasts at least as long as the narration; captions are generated from
//! the text with word timings (proportional, or exact from word-level JSON).

use crate::audio::{AudioClip, AudioData};
use crate::captions::{captions_from_json, Cue, Word};
use crate::scene::Scene;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// A narration segment.
#[derive(Clone, Debug)]
pub struct Voiceover {
    pub text: String,
    pub audio: Option<Arc<AudioData>>,
    pub audio_path: Option<PathBuf>,
    /// Word-level timings in JSON (`[{"text","startMs","endMs"}]`, e.g. Whisper output).
    pub words_json: Option<String>,
    /// Speaking rate used when there is no audio (words per minute).
    pub wpm: f64,
    pub volume: f32,
    /// Show captions for this narration.
    pub captions: bool,
    /// Max words per caption cue.
    pub max_words: usize,
    /// Use the TTS command from `RUNWEE_TTS` when no audio is given.
    pub tts: bool,
}

impl Voiceover {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            audio: None,
            audio_path: None,
            words_json: None,
            wpm: 155.0,
            volume: 1.0,
            captions: true,
            max_words: 7,
            tts: true,
        }
    }
    pub fn audio(mut self, path: impl Into<PathBuf>) -> Self {
        self.audio_path = Some(path.into());
        self
    }
    pub fn audio_data(mut self, d: Arc<AudioData>) -> Self {
        self.audio = Some(d);
        self
    }
    pub fn words_json(mut self, json: &str) -> Self {
        self.words_json = Some(json.to_string());
        self
    }
    pub fn wpm(mut self, w: f64) -> Self {
        self.wpm = w;
        self
    }
    pub fn volume(mut self, v: f32) -> Self {
        self.volume = v;
        self
    }
    pub fn no_captions(mut self) -> Self {
        self.captions = false;
        self
    }
    pub fn no_tts(mut self) -> Self {
        self.tts = false;
        self
    }
}

/// Timing information handed to a voice-over block.
#[derive(Clone, Debug)]
pub struct VoiceInfo {
    pub start: f64,
    pub duration: f64,
    pub words: Vec<Word>,
    pub bookmarks: HashMap<String, f64>,
}

impl VoiceInfo {
    pub fn end(&self) -> f64 {
        self.start + self.duration
    }
    /// Absolute time of a `<bookmark mark='name'/>` (falls back to the start).
    pub fn bookmark(&self, name: &str) -> f64 {
        self.bookmarks.get(name).copied().unwrap_or(self.start)
    }
    /// Absolute time when the `n`-th word (0-based) is spoken.
    pub fn word_time(&self, n: usize) -> f64 {
        self.words.get(n).map_or(self.end(), |w| w.start)
    }
    /// Seconds remaining after time `t`.
    pub fn remaining(&self, t: f64) -> f64 {
        (self.end() - t).max(0.0)
    }
}

/// Strip `<bookmark mark='x'/>` tags; returns clean text and (word index → name).
fn extract_bookmarks(text: &str) -> (String, Vec<(usize, String)>) {
    let mut clean = String::new();
    let mut marks = Vec::new();
    let mut rest = text;
    while let Some(p) = rest.find("<bookmark") {
        clean.push_str(&rest[..p]);
        let tail = &rest[p..];
        let end = tail.find("/>").map(|e| e + 2).unwrap_or(tail.len());
        let tag = &tail[..end];
        let name = tag.split(['\'', '"']).nth(1).unwrap_or("").to_string();
        marks.push((clean.split_whitespace().count(), name));
        rest = &tail[end..];
    }
    clean.push_str(rest);
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    (clean, marks)
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Run the TTS command template from `RUNWEE_TTS` (e.g. `espeak-ng -w {out} "{text}"`
/// or `piper --model en.onnx --output_file {out}` with text on stdin). Cached by text.
pub fn synthesize_tts(text: &str) -> Option<Arc<AudioData>> {
    let template = std::env::var("RUNWEE_TTS").ok()?;
    let dir = std::env::temp_dir().join("runwee-tts");
    std::fs::create_dir_all(&dir).ok()?;
    let out = dir.join(format!("{:016x}.wav", fnv(&(template.clone() + text))));
    if !out.exists() {
        let escaped = text.replace('"', "\\\"");
        let cmd = template
            .replace("{out}", &out.to_string_lossy())
            .replace("{text}", &escaped);
        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .ok()?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        if !child.wait().ok()?.success() {
            return None;
        }
    }
    AudioData::load(&out).ok().map(Arc::new)
}

impl Scene {
    /// Move the cursor forward to absolute time `t` (no-op if already past it).
    pub fn wait_until(&mut self, t: f64) {
        self.seek(t);
    }

    /// Place a sound at the current time.
    pub fn add_sound(&mut self, data: Arc<AudioData>, volume: f32) {
        let t = self.time();
        self.audio.add(AudioClip::new(data, t).volume(volume));
    }
    /// Place a sound at an absolute time.
    pub fn add_sound_at(&mut self, data: Arc<AudioData>, t: f64, volume: f32) {
        self.audio.add(AudioClip::new(data, t).volume(volume));
    }
    /// Load and place an audio file (wav/mp3/ogg/…) at the current time.
    pub fn add_sound_file(&mut self, path: &str, volume: f32) -> Result<f64> {
        let d = Arc::new(AudioData::load(path)?);
        let dur = d.duration();
        self.add_sound(d, volume);
        Ok(dur)
    }
    /// Add a fully configured clip (volume curves, trim, rate, loop…).
    pub fn add_clip(&mut self, clip: AudioClip) {
        self.audio.add(clip);
    }
    /// Background music looped for the whole video, faded in/out.
    pub fn background_music(&mut self, data: Arc<AudioData>, volume: f32) {
        let mut c = AudioClip::new(data, 0.0).volume(volume).fades(1.5, 2.0);
        c.looping = true;
        c.duration = Some(f64::INFINITY);
        self.audio.add(c);
    }

    /// Narrate `vo` while running `f`; the block lasts until both finish.
    pub fn voiceover(
        &mut self,
        vo: Voiceover,
        f: impl FnOnce(&mut Scene, &VoiceInfo),
    ) -> VoiceInfo {
        let info = self.begin_voiceover(&vo);
        f(self, &info);
        self.end_voiceover(&info);
        info
    }

    /// First half of [`Scene::voiceover`]: schedules audio and captions at the
    /// current time and returns the timing (useful when the body runs elsewhere,
    /// e.g. in a script).
    pub fn begin_voiceover(&mut self, vo: &Voiceover) -> VoiceInfo {
        let start = self.time();
        let (clean, marks) = extract_bookmarks(&vo.text);
        let audio = vo
            .audio
            .clone()
            .or_else(|| {
                vo.audio_path
                    .as_ref()
                    .and_then(|p| match AudioData::load(p) {
                        Ok(d) => Some(Arc::new(d)),
                        Err(e) => {
                            eprintln!("warning: voiceover audio {}: {e}", p.display());
                            None
                        }
                    })
            })
            .or_else(|| if vo.tts { synthesize_tts(&clean) } else { None });
        let n_words = clean.split_whitespace().count();
        let estimated = (n_words as f64 / vo.wpm * 60.0).max(0.8);
        let duration = audio.as_ref().map_or(estimated, |a| a.duration());
        if let Some(a) = &audio {
            self.audio
                .add(AudioClip::new(a.clone(), start).volume(vo.volume));
        }
        let words: Vec<Word> = match vo
            .words_json
            .as_deref()
            .and_then(|j| captions_from_json(j).ok())
        {
            Some(caps) => caps
                .iter()
                .map(|c| Word {
                    text: c.text.trim().to_string(),
                    start: start + c.start_ms / 1000.0,
                    end: start + c.end_ms / 1000.0,
                })
                .collect(),
            None => Cue::auto(&clean, start, start + duration).words,
        };
        let mut bookmarks = HashMap::new();
        for (idx, name) in marks {
            let t = words.get(idx).map_or(start + duration, |w| w.start);
            bookmarks.insert(name, t);
        }
        if vo.captions && !words.is_empty() {
            for chunk in chunk_words(&words, vo.max_words) {
                self.captions.add(Cue {
                    start: chunk[0].start,
                    end: chunk.last().unwrap().end,
                    text: chunk
                        .iter()
                        .map(|w| w.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    words: chunk.to_vec(),
                });
            }
        }
        VoiceInfo {
            start,
            duration,
            words,
            bookmarks,
        }
    }

    /// Second half of [`Scene::voiceover`]: wait until the narration has finished.
    pub fn end_voiceover(&mut self, info: &VoiceInfo) {
        let end = info.end();
        if self.time() < end {
            self.wait_until(end);
        }
    }

    /// Narrate without animations (just waits for the narration).
    pub fn narrate(&mut self, text: &str) -> VoiceInfo {
        self.voiceover(Voiceover::new(text), |_, _| {})
    }
}

/// Split words into caption chunks: break at sentence ends, then into balanced
/// pieces of at most `max` words (no orphaned single words).
pub fn chunk_words(words: &[Word], max: usize) -> Vec<&[Word]> {
    let max = max.max(1);
    let mut out = Vec::new();
    let mut s = 0;
    for i in 0..words.len() {
        if words[i].text.ends_with(['.', '!', '?', ';', ':']) || i + 1 == words.len() {
            let sentence = &words[s..=i];
            let n = sentence.len();
            let parts = n.div_ceil(max);
            let size = n.div_ceil(parts);
            for c in sentence.chunks(size) {
                out.push(c);
            }
            s = i + 1;
        }
    }
    out
}

/// Spectrum bars for an audio visualiser at time `t` (Remotion's `visualizeAudio`).
pub fn spectrum_bars(
    data: &AudioData,
    t: f64,
    bars: usize,
    width: f64,
    height: f64,
    colors: &[crate::color::Color],
) -> crate::mobject::Mobject {
    use crate::math::v2;
    let sp = crate::audio::spectrum_at(data, t, bars);
    let palette = if colors.is_empty() {
        vec![crate::color::BLUE, crate::color::PURPLE, crate::color::PINK]
    } else {
        colors.to_vec()
    };
    let bw = width / bars as f64;
    crate::mobject::Mobject::group(sp.iter().enumerate().map(|(i, v)| {
        let h = (*v as f64 * height).max(0.04);
        let c = crate::color::color_gradient(&palette, i as f64 / bars.max(1) as f64);
        crate::shapes::rounded_rectangle(bw * 0.7, h, bw * 0.3)
            .move_to(v2(-width / 2.0 + bw * (i as f64 + 0.5), h / 2.0))
            .fill(c, 1.0)
            .no_stroke()
    }))
}

/// Static waveform outline of a whole clip (`width` × `height`, centred).
pub fn waveform_mobject(
    data: &AudioData,
    samples: usize,
    width: f64,
    height: f64,
    color: crate::color::Color,
) -> crate::mobject::Mobject {
    use crate::math::v2;
    let w = crate::audio::waveform(data, samples);
    let dx = width / samples.max(1) as f64;
    crate::mobject::Mobject::group(w.iter().enumerate().map(|(i, a)| {
        let x = -width / 2.0 + dx * (i as f64 + 0.5);
        let h = (*a as f64 * height).max(0.02);
        crate::shapes::line(v2(x, -h / 2.0), v2(x, h / 2.0)).stroke(color, 2.0)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::*;
    use crate::shapes::circle;

    #[test]
    fn voiceover_paces_scene() {
        let mut s = Scene::new();
        let c = s.register(circle(1.0));
        let info = s.voiceover(
            Voiceover::new("A circle has <bookmark mark='r'/> a radius and an area.")
                .no_tts()
                .wpm(120.0),
            |s, vo| {
                s.play(create(c));
                s.wait_until(vo.bookmark("r"));
            },
        );
        assert_eq!(info.words.len(), 8);
        assert!((s.time() - info.end()).abs() < 1e-9);
        assert!(info.bookmark("r") > info.start);
        assert!(!s.captions.is_empty());
        let before = s.time();
        s.narrate("short");
        assert!(s.time() > before);
    }

    #[test]
    fn audio_in_scene() {
        let mut s = Scene::new();
        s.add_sound(crate::sfx::ding(), 0.8);
        s.background_music(crate::sfx::ambient_pad(2.0, 48.0, 1.0), 0.3);
        s.wait(1.0);
        assert!(s.duration() >= 1.6, "audio extends the duration");
        let mix = s.audio.mix(48_000, s.duration());
        assert!(mix.iter().any(|x| x.abs() > 0.05));
    }

    #[test]
    fn bookmarks_parse() {
        let (clean, marks) = extract_bookmarks("one <bookmark mark=\"a\"/> two three");
        assert_eq!(clean, "one two three");
        assert_eq!(marks, vec![(1, "a".to_string())]);
    }
}
