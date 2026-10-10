//! Captions: Remotion's caption model (`Caption`, `parseSrt`, `serializeSrt`,
//! `createTikTokStyleCaptions`, `ensureMaxCharactersPerLine`), SRT/WebVTT export and
//! burned-in rendering (classic subtitles, TikTok word-highlight pages, karaoke).

use crate::color::*;
use crate::math::*;
use crate::mobject::Mobject;
use crate::render::{Camera, Canvas, RenderConfig};
use crate::shapes;
use crate::text::{Span, Text};

/// A timed token (Remotion's `Caption`): usually one word with leading space.
#[derive(Clone, Debug, PartialEq)]
pub struct Caption {
    pub text: String,
    pub start_ms: f64,
    pub end_ms: f64,
    pub timestamp_ms: Option<f64>,
    pub confidence: Option<f64>,
}

/// A word with timing (seconds).
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

/// One caption line/page shown on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub words: Vec<Word>,
}

impl Cue {
    /// Cue with word timings distributed proportionally to word length.
    pub fn auto(text: &str, start: f64, end: f64) -> Cue {
        let words: Vec<&str> = text.split_whitespace().collect();
        let weights: Vec<f64> = words
            .iter()
            .map(|w| w.chars().count() as f64 + 2.0)
            .collect();
        let total: f64 = weights.iter().sum::<f64>().max(1e-9);
        let mut t = start;
        let ws = words
            .iter()
            .zip(&weights)
            .map(|(w, k)| {
                let d = (end - start) * k / total;
                let wd = Word {
                    text: w.to_string(),
                    start: t,
                    end: t + d,
                };
                t += d;
                wd
            })
            .collect();
        Cue {
            start,
            end,
            text: text.to_string(),
            words: ws,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CaptionStyle {
    /// Only exported as sidecar / soft subtitles.
    Hidden,
    /// Classic subtitle bar.
    #[default]
    Subtitle,
    /// TikTok/Reels: page of words, active word highlighted & popped.
    Highlight,
    /// Words fill with colour as they are spoken.
    Karaoke,
}

/// Look of burned-in captions.
#[derive(Clone, Debug)]
pub struct CaptionLook {
    pub style: CaptionStyle,
    /// Em size in scene units (for an 8-unit tall frame).
    pub size: f64,
    pub color: Color,
    pub highlight: Color,
    pub background: Option<(Color, f32)>,
    /// Distance of the caption's bottom from the frame bottom (scene units).
    pub bottom: f64,
    /// Max width as a fraction of the frame width.
    pub max_width: f64,
    pub uppercase: bool,
    pub bold: bool,
    pub font: String,
}

impl Default for CaptionLook {
    fn default() -> Self {
        Self {
            style: CaptionStyle::Subtitle,
            size: 0.36,
            color: WHITE,
            highlight: YELLOW,
            background: Some((BLACK, 0.6)),
            bottom: 0.45,
            max_width: 0.82,
            uppercase: false,
            bold: false,
            font: "Inter SemiBold".into(),
        }
    }
}

impl CaptionLook {
    /// Big bold word-highlight captions for shorts.
    pub fn tiktok() -> Self {
        Self {
            style: CaptionStyle::Highlight,
            size: 0.6,
            uppercase: true,
            bold: true,
            background: None,
            bottom: 2.2,
            font: "Inter".into(),
            ..Default::default()
        }
    }
    pub fn karaoke() -> Self {
        Self {
            style: CaptionStyle::Karaoke,
            ..Default::default()
        }
    }
}

/// All captions of a composition.
#[derive(Clone, Debug, Default)]
pub struct CaptionTrack {
    pub cues: Vec<Cue>,
    pub look: CaptionLook,
}

impl CaptionTrack {
    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }
    pub fn add(&mut self, cue: Cue) {
        let pos = self.cues.partition_point(|c| c.start <= cue.start);
        self.cues.insert(pos, cue);
    }
    /// Add text spoken between `start` and `end`, split into readable cues of at most
    /// `max_words` words each.
    pub fn add_text(&mut self, text: &str, start: f64, end: f64, max_words: usize) {
        let all = Cue::auto(text, start, end);
        for chunk in split_sentences(&all.words, max_words.max(1)) {
            let s = chunk.first().map_or(start, |w| w.start);
            let e = chunk.last().map_or(end, |w| w.end);
            self.add(Cue {
                start: s,
                end: e,
                text: chunk
                    .iter()
                    .map(|w| w.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                words: chunk.to_vec(),
            });
        }
    }
    pub fn offset(&mut self, dt: f64) {
        for c in &mut self.cues {
            c.start += dt;
            c.end += dt;
            for w in &mut c.words {
                w.start += dt;
                w.end += dt;
            }
        }
    }
    pub fn active(&self, t: f64) -> Option<&Cue> {
        self.cues.iter().rev().find(|c| t >= c.start && t < c.end)
    }
    pub fn to_srt(&self) -> String {
        serialize_srt_cues(&self.cues)
    }
    pub fn to_vtt(&self) -> String {
        let mut s = String::from("WEBVTT\n\n");
        for c in &self.cues {
            s.push_str(&format!(
                "{} --> {}\n{}\n\n",
                ts(c.start, '.'),
                ts(c.end, '.'),
                c.text
            ));
        }
        s
    }

    /// Build the caption overlay for time `t` in frame coordinates (8-unit tall frame).
    pub fn overlay(&self, t: f64, frame_w: f64, frame_h: f64) -> Option<Mobject> {
        if self.look.style == CaptionStyle::Hidden {
            return None;
        }
        let cue = self.active(t)?;
        let look = &self.look;
        let case = |s: &str| {
            if look.uppercase {
                s.to_uppercase()
            } else {
                s.to_string()
            }
        };
        let active_idx = cue.words.iter().position(|w| t >= w.start && t < w.end);
        let mut spans = Vec::new();
        for (i, w) in cue.words.iter().enumerate() {
            if i > 0 {
                spans.push(Span::new(" "));
            }
            let mut sp = Span::new(&case(&w.text));
            sp.color = Some(match look.style {
                CaptionStyle::Highlight if Some(i) == active_idx => look.highlight,
                CaptionStyle::Karaoke if t >= w.start => look.highlight,
                _ => look.color,
            });
            spans.push(sp);
        }
        if cue.words.is_empty() {
            spans.push(Span::new(&case(&cue.text)).color(look.color));
        }
        let mut txt = Text::spans(spans)
            .font(&look.font)
            .size(look.size)
            .align(crate::text::Align::Center)
            .max_width(frame_w * look.max_width);
        if look.bold {
            txt = txt.bold();
        }
        let mut m = txt.build();
        if look.style == CaptionStyle::Highlight {
            m = m.background_stroke(BLACK, 10.0);
            // pop-in of the page
            let age = t - cue.start;
            let s = 0.85 + 0.15 * crate::rate::Rate::OutBack.eval((age / 0.18).min(1.0));
            m = m.scale(s);
        }
        let y = -frame_h / 2.0 + look.bottom + m.height() / 2.0;
        let m = m.move_to(v2(0.0, y));
        let mut g = Mobject::empty();
        if let Some((c, o)) = look.background {
            let b = m.bounds_or_origin().expand(look.size * 0.35);
            g.push(
                shapes::rounded_rectangle(b.width(), b.height(), look.size * 0.3)
                    .move_to(b.center())
                    .fill(c, o)
                    .no_stroke(),
            );
        }
        g.push(m);
        Some(g)
    }

    /// Burn captions into a frame.
    pub fn draw(&self, t: f64, pm: &mut tiny_skia::Pixmap, cfg: RenderConfig, frame_height: f64) {
        let fw = frame_height * cfg.aspect();
        if let Some(m) = self.overlay(t, fw, frame_height) {
            let cam = Camera {
                frame_height,
                ..Camera::default()
            };
            let mut c = Canvas::new(pm, cfg, cam);
            c.draw(&m);
        }
    }
}

fn split_sentences(words: &[Word], max_words: usize) -> Vec<&[Word]> {
    crate::voice::chunk_words(words, max_words)
}

fn ts(t: f64, sep: char) -> String {
    let ms = (t.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}{}{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        sep,
        ms % 1000
    )
}

fn parse_ts(s: &str) -> Option<f64> {
    let s = s.trim().replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    let (h, m, sec) = match parts.len() {
        3 => (
            parts[0].parse::<f64>().ok()?,
            parts[1].parse::<f64>().ok()?,
            parts[2].parse::<f64>().ok()?,
        ),
        2 => (
            0.0,
            parts[0].parse::<f64>().ok()?,
            parts[1].parse::<f64>().ok()?,
        ),
        _ => return None,
    };
    Some(h * 3600.0 + m * 60.0 + sec)
}

/// Parse SRT (or WebVTT) into cues.
pub fn parse_srt_cues(input: &str) -> Vec<Cue> {
    let mut cues = Vec::new();
    let norm = input.replace("\r\n", "\n");
    for block in norm.split("\n\n") {
        let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
        let Some(ti) = lines.iter().position(|l| l.contains("-->")) else {
            continue;
        };
        let mut it = lines[ti].split("-->");
        let (Some(a), Some(b)) = (it.next(), it.next()) else {
            continue;
        };
        let b = b.split_whitespace().next().unwrap_or("");
        let (Some(s), Some(e)) = (parse_ts(a), parse_ts(b)) else {
            continue;
        };
        let text = lines[ti + 1..].join("\n");
        cues.push(Cue::auto(&text, s, e));
    }
    cues
}

/// Remotion's `parseSrt`: one `Caption` per cue.
pub fn parse_srt(input: &str) -> Vec<Caption> {
    parse_srt_cues(input)
        .into_iter()
        .map(|c| Caption {
            text: c.text,
            start_ms: c.start * 1000.0,
            end_ms: c.end * 1000.0,
            timestamp_ms: Some((c.start + c.end) * 500.0),
            confidence: Some(1.0),
        })
        .collect()
}

pub fn serialize_srt_cues(cues: &[Cue]) -> String {
    let mut s = String::new();
    for (i, c) in cues.iter().enumerate() {
        s.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            ts(c.start, ','),
            ts(c.end, ','),
            c.text
        ));
    }
    s
}

/// Remotion's `serializeSrt({lines})`: each line is a list of captions shown together.
pub fn serialize_srt(lines: &[Vec<Caption>]) -> String {
    let cues: Vec<Cue> = lines
        .iter()
        .filter(|l| !l.is_empty())
        .map(|l| Cue {
            start: l[0].start_ms / 1000.0,
            end: l.last().unwrap().end_ms / 1000.0,
            text: l
                .iter()
                .map(|c| c.text.as_str())
                .collect::<String>()
                .trim()
                .to_string(),
            words: vec![],
        })
        .collect();
    serialize_srt_cues(&cues)
}

/// Convert SRT text to WebVTT.
pub fn srt_to_vtt(srt: &str) -> Option<String> {
    let t = CaptionTrack {
        cues: parse_srt_cues(srt),
        look: CaptionLook::default(),
    };
    Some(t.to_vtt())
}

/// A TikTok-style page (Remotion's `TikTokPage`).
#[derive(Clone, Debug, PartialEq)]
pub struct TikTokPage {
    pub text: String,
    pub start_ms: f64,
    pub duration_ms: f64,
    pub tokens: Vec<Caption>,
}

/// Remotion's `createTikTokStyleCaptions`: group tokens into pages that are shown
/// together, starting a new page when a token starts with a space and the page has
/// lasted longer than `combine_ms`.
pub fn create_tiktok_style_captions(captions: &[Caption], combine_ms: f64) -> Vec<TikTokPage> {
    let mut pages: Vec<TikTokPage> = Vec::new();
    let mut cur: Vec<Caption> = Vec::new();
    let flush = |cur: &mut Vec<Caption>, pages: &mut Vec<TikTokPage>| {
        if cur.is_empty() {
            return;
        }
        let text: String = cur
            .iter()
            .map(|c| c.text.as_str())
            .collect::<String>()
            .trim_start()
            .to_string();
        if let Some(prev) = pages.last_mut() {
            prev.duration_ms = cur[0].start_ms - prev.start_ms;
        }
        pages.push(TikTokPage {
            text,
            start_ms: cur[0].start_ms,
            duration_ms: f64::INFINITY,
            tokens: std::mem::take(cur),
        });
    };
    for c in captions {
        if let Some(first) = cur.first() {
            let elapsed = cur.last().unwrap().end_ms - first.start_ms;
            if c.text.starts_with(' ') && elapsed > combine_ms {
                flush(&mut cur, &mut pages);
            }
        }
        cur.push(c.clone());
    }
    flush(&mut cur, &mut pages);
    if let (Some(last), Some(tok)) = (pages.last_mut(), captions.last()) {
        last.duration_ms = tok.end_ms - last.start_ms;
    }
    pages
}

/// Remotion's `ensureMaxCharactersPerLine`: insert line breaks so no line exceeds `max` chars.
pub fn ensure_max_characters_per_line(text: &str, max: usize) -> String {
    let mut lines: Vec<String> = vec![String::new()];
    for w in text.split_whitespace() {
        let cur = lines.last_mut().unwrap();
        if !cur.is_empty() && cur.chars().count() + 1 + w.chars().count() > max {
            lines.push(w.to_string());
        } else {
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(w);
        }
    }
    lines.join("\n")
}

/// Convert word-level captions (e.g. from Whisper JSON) into cues of `max_words` words.
pub fn captions_to_cues(captions: &[Caption], max_words: usize) -> Vec<Cue> {
    let words: Vec<Word> = captions
        .iter()
        .map(|c| Word {
            text: c.text.trim().to_string(),
            start: c.start_ms / 1000.0,
            end: c.end_ms / 1000.0,
        })
        .collect();
    split_sentences(&words, max_words.max(1))
        .into_iter()
        .map(|ws| Cue {
            start: ws[0].start,
            end: ws.last().unwrap().end,
            text: ws
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            words: ws.to_vec(),
        })
        .collect()
}

/// Load word-level captions from Remotion/Whisper-style JSON: `[{"text","startMs","endMs"}]`.
pub fn captions_from_json(json: &str) -> anyhow::Result<Vec<Caption>> {
    let v: serde_json::Value = serde_json::from_str(json)?;
    let arr = v
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("expected an array of captions"))?;
    Ok(arr
        .iter()
        .filter_map(|o| {
            Some(Caption {
                text: o.get("text")?.as_str()?.to_string(),
                start_ms: o.get("startMs").or_else(|| o.get("start_ms"))?.as_f64()?,
                end_ms: o.get("endMs").or_else(|| o.get("end_ms"))?.as_f64()?,
                timestamp_ms: o.get("timestampMs").and_then(|x| x.as_f64()),
                confidence: o.get("confidence").and_then(|x| x.as_f64()),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srt_roundtrip() {
        let mut t = CaptionTrack::default();
        t.add_text("Hello world. This is a test of captions", 1.0, 4.0, 4);
        assert_eq!(t.cues.len(), 3);
        assert!(t.cues[0].text.ends_with('.'));
        let srt = t.to_srt();
        assert!(srt.contains("00:00:01,000 -->"));
        let back = parse_srt_cues(&srt);
        assert_eq!(back.len(), 3);
        assert!((back[2].end - 4.0).abs() < 1e-3);
        let vtt = srt_to_vtt("1\n00:00:01,500 --> 00:00:02,000\nA, B\n").unwrap();
        assert!(vtt.contains("00:00:01.500") && vtt.contains("A, B"));
        assert_eq!(parse_srt(&srt).len(), 3);
    }

    #[test]
    fn tiktok_pages_and_wrapping() {
        let caps: Vec<Caption> = ["Hi", " there", " how", " are", " you"]
            .iter()
            .enumerate()
            .map(|(i, w)| Caption {
                text: w.to_string(),
                start_ms: i as f64 * 400.0,
                end_ms: i as f64 * 400.0 + 350.0,
                timestamp_ms: None,
                confidence: None,
            })
            .collect();
        let pages = create_tiktok_style_captions(&caps, 700.0);
        assert!(pages.len() >= 2);
        assert_eq!(pages[0].text, "Hi there");
        assert_eq!(
            ensure_max_characters_per_line("aaa bbb ccc", 7),
            "aaa bbb\nccc"
        );
        let cues = captions_to_cues(&caps, 3);
        assert_eq!(cues.len(), 2);
        let json = r#"[{"text":"Hi","startMs":0,"endMs":300}]"#;
        assert_eq!(captions_from_json(json).unwrap().len(), 1);
    }

    #[test]
    fn overlay_builds() {
        let mut t = CaptionTrack::default();
        t.add_text("Energy is conserved", 0.0, 2.0, 8);
        t.look = CaptionLook::tiktok();
        let m = t.overlay(0.5, 14.2, 8.0).unwrap();
        assert!(m.bottom().y > -4.0);
        assert!(t.overlay(3.0, 14.2, 8.0).is_none());
    }
}
