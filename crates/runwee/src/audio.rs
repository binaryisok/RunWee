//! Audio: decoding (native WAV, anything else via ffmpeg), clips with Remotion-style
//! volume curves / trimming / playback rate / looping, a pure-Rust mixer, and
//! analysis helpers (duration, waveform, FFT spectrum) for audio visualisations.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

/// Internal sample rate (stereo, f32).
pub const SAMPLE_RATE: u32 = 48_000;

/// Decoded audio: interleaved stereo f32 at [`SAMPLE_RATE`].
#[derive(Clone, Debug, Default)]
pub struct AudioData {
    pub samples: Vec<f32>,
}

impl AudioData {
    /// Number of stereo frames.
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }
    pub fn duration(&self) -> f64 {
        self.frames() as f64 / SAMPLE_RATE as f64
    }
    /// From mono samples at `sr` (resampled linearly).
    pub fn from_mono(mono: &[f32], sr: u32) -> Self {
        let st: Vec<f32> = mono.iter().flat_map(|s| [*s, *s]).collect();
        Self::from_interleaved(&st, sr, 2)
    }
    /// From interleaved samples with `channels` channels at `sr`.
    pub fn from_interleaved(data: &[f32], sr: u32, channels: usize) -> Self {
        let ch = channels.max(1);
        let n_in = data.len() / ch;
        let get = |i: usize, c: usize| -> f32 {
            let c = if ch == 1 { 0 } else { c.min(ch - 1) };
            data[i * ch + c]
        };
        if sr == SAMPLE_RATE {
            let mut out = Vec::with_capacity(n_in * 2);
            for i in 0..n_in {
                out.push(get(i, 0));
                out.push(get(i, 1));
            }
            return Self { samples: out };
        }
        let ratio = sr as f64 / SAMPLE_RATE as f64;
        let n_out = (n_in as f64 / ratio) as usize;
        let mut out = Vec::with_capacity(n_out * 2);
        for j in 0..n_out {
            let x = j as f64 * ratio;
            let i = x.floor() as usize;
            let f = (x - i as f64) as f32;
            for c in 0..2 {
                let a = get(i.min(n_in - 1), c);
                let b = get((i + 1).min(n_in - 1), c);
                out.push(a + (b - a) * f);
            }
        }
        Self { samples: out }
    }
    /// Load a file. WAV is decoded natively; other formats (mp3, ogg, m4a, flac…) via ffmpeg.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
            if let Ok(a) = parse_wav(&bytes) {
                return Ok(a);
            }
        }
        decode_with_ffmpeg(path)
    }
    /// Stereo frame `i` (zero outside).
    #[inline]
    pub fn frame(&self, i: isize) -> (f32, f32) {
        if i < 0 || i as usize >= self.frames() {
            (0.0, 0.0)
        } else {
            let k = i as usize * 2;
            (self.samples[k], self.samples[k + 1])
        }
    }
    /// Linearly interpolated frame at a fractional position.
    pub fn sample_at(&self, pos: f64) -> (f32, f32) {
        let i = pos.floor();
        let f = (pos - i) as f32;
        let (a0, a1) = self.frame(i as isize);
        let (b0, b1) = self.frame(i as isize + 1);
        (a0 + (b0 - a0) * f, a1 + (b1 - a1) * f)
    }
    /// Mono mix (average of channels).
    pub fn mono(&self) -> Vec<f32> {
        self.samples
            .chunks(2)
            .map(|c| (c[0] + c[1]) * 0.5)
            .collect()
    }
    /// Peak-normalise to `peak`.
    pub fn normalized(mut self, peak: f32) -> Self {
        let m = self.samples.iter().fold(0.0f32, |a, s| a.max(s.abs()));
        if m > 1e-9 {
            let k = peak / m;
            for s in &mut self.samples {
                *s *= k;
            }
        }
        self
    }
    /// Concatenate.
    pub fn concat(mut self, other: &AudioData) -> Self {
        self.samples.extend_from_slice(&other.samples);
        self
    }
    /// Silence of `secs` seconds.
    pub fn silence(secs: f64) -> Self {
        Self {
            samples: vec![0.0; (secs * SAMPLE_RATE as f64) as usize * 2],
        }
    }
    pub fn save_wav(&self, path: impl AsRef<Path>) -> Result<()> {
        write_wav(path.as_ref(), &self.samples, SAMPLE_RATE, 2)
    }
}

fn parse_wav(b: &[u8]) -> Result<AudioData> {
    let u16le = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32le = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let mut i = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let size = u32le(i + 4) as usize;
        let body = i + 8;
        if id == b"fmt " {
            let mut format = u16le(body);
            let channels = u16le(body + 2);
            let sr = u32le(body + 4);
            let bits = u16le(body + 14);
            if format == 0xFFFE && size >= 26 {
                format = u16le(body + 24); // WAVE_FORMAT_EXTENSIBLE sub-format
            }
            fmt = Some((format, channels, sr, bits));
        } else if id == b"data" {
            let (format, channels, sr, bits) = fmt.context("wav: data before fmt")?;
            let end = (body + size).min(b.len());
            let d = &b[body..end];
            let samples: Vec<f32> = match (format, bits) {
                (1, 8) => d.iter().map(|&x| (x as f32 - 128.0) / 128.0).collect(),
                (1, 16) => d
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                    .collect(),
                (1, 24) => d
                    .chunks_exact(3)
                    .map(|c| (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0)
                    .collect(),
                (1, 32) => d
                    .chunks_exact(4)
                    .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0)
                    .collect(),
                (3, 32) => d
                    .chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect(),
                _ => bail!("unsupported wav format {format}/{bits}"),
            };
            return Ok(AudioData::from_interleaved(&samples, sr, channels as usize));
        }
        i = body + size + (size & 1);
    }
    bail!("wav: no data chunk")
}

fn decode_with_ffmpeg(path: &Path) -> Result<AudioData> {
    let out = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(path)
        .args([
            "-f",
            "f32le",
            "-ac",
            "2",
            "-ar",
            &SAMPLE_RATE.to_string(),
            "-",
        ])
        .output()
        .context("ffmpeg is required to decode non-WAV audio")?;
    if !out.status.success() {
        bail!(
            "ffmpeg failed to decode {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let samples = out
        .stdout
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    Ok(AudioData { samples })
}

/// Duration of an audio file in seconds (Remotion's `getAudioDurationInSeconds`).
/// Decodes the file, so the result matches the samples [`AudioData::load`] will mux.
pub fn audio_duration(path: impl AsRef<Path>) -> Result<f64> {
    Ok(AudioData::load(path)?.duration())
}

/// Container duration from `ffprobe` (`format=duration`), without decoding samples.
pub fn probe_duration(path: impl AsRef<Path>) -> Result<f64> {
    let path = path.as_ref();
    let out = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .with_context(|| format!("ffprobe {}", path.display()))?;
    if !out.status.success() {
        bail!(
            "ffprobe failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .parse::<f64>()
        .with_context(|| format!("ffprobe duration for {} ({text:?})", path.display()))
}

/// Volume of a clip: constant or a function of the clip-local time in seconds
/// (Remotion's `volume={(f) => interpolate(f, …)}`).
#[derive(Clone)]
pub enum Volume {
    Const(f32),
    Curve(Arc<dyn Fn(f64) -> f32 + Send + Sync>),
}

impl std::fmt::Debug for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Volume::Const(v) => write!(f, "Const({v})"),
            Volume::Curve(_) => write!(f, "Curve"),
        }
    }
}

impl Volume {
    pub fn at(&self, t: f64) -> f32 {
        match self {
            Volume::Const(v) => *v,
            Volume::Curve(f) => f(t),
        }
    }
}

/// An audio clip placed on the timeline (Remotion's `<Audio>` / `<Html5Audio>`).
#[derive(Clone, Debug)]
pub struct AudioClip {
    pub data: Arc<AudioData>,
    /// Scene time at which the clip starts.
    pub start: f64,
    /// Skip this many seconds of the source (`trimBefore`).
    pub trim_start: f64,
    /// Stop after this source time (`trimAfter`).
    pub trim_end: Option<f64>,
    pub volume: Volume,
    /// Playback rate (`playbackRate`); also shifts pitch, like Remotion.
    pub rate: f64,
    /// Loop the (trimmed) source until `duration`.
    pub looping: bool,
    /// Clip length on the timeline when looping (or to cut it short).
    pub duration: Option<f64>,
    pub fade_in: f64,
    pub fade_out: f64,
    /// Stereo pan, -1 (left) … 1 (right).
    pub pan: f32,
}

impl AudioClip {
    pub fn new(data: Arc<AudioData>, start: f64) -> Self {
        Self {
            data,
            start,
            trim_start: 0.0,
            trim_end: None,
            volume: Volume::Const(1.0),
            rate: 1.0,
            looping: false,
            duration: None,
            fade_in: 0.0,
            fade_out: 0.0,
            pan: 0.0,
        }
    }
    pub fn volume(mut self, v: f32) -> Self {
        self.volume = Volume::Const(v);
        self
    }
    pub fn volume_curve(mut self, f: impl Fn(f64) -> f32 + Send + Sync + 'static) -> Self {
        self.volume = Volume::Curve(Arc::new(f));
        self
    }
    pub fn trim(mut self, start: f64, end: Option<f64>) -> Self {
        self.trim_start = start;
        self.trim_end = end;
        self
    }
    pub fn rate(mut self, r: f64) -> Self {
        self.rate = r.max(0.01);
        self
    }
    pub fn looped(mut self, duration: f64) -> Self {
        self.looping = true;
        self.duration = Some(duration);
        self
    }
    pub fn duration(mut self, d: f64) -> Self {
        self.duration = Some(d);
        self
    }
    pub fn fades(mut self, fade_in: f64, fade_out: f64) -> Self {
        self.fade_in = fade_in;
        self.fade_out = fade_out;
        self
    }
    pub fn pan(mut self, p: f32) -> Self {
        self.pan = p.clamp(-1.0, 1.0);
        self
    }
    fn source_len(&self) -> f64 {
        let end = self
            .trim_end
            .unwrap_or(self.data.duration())
            .min(self.data.duration());
        (end - self.trim_start).max(0.0)
    }
    /// Length on the timeline.
    pub fn timeline_len(&self) -> f64 {
        let natural = self.source_len() / self.rate;
        match (self.duration, self.looping) {
            (Some(d), true) => d,
            (Some(d), false) => d.min(natural),
            (None, _) => natural,
        }
    }
    pub fn end(&self) -> f64 {
        self.start + self.timeline_len()
    }
}

/// All audio of a composition.
#[derive(Clone, Debug, Default)]
pub struct AudioTrack {
    pub clips: Vec<AudioClip>,
    /// Master gain.
    pub gain: f32,
}

impl AudioTrack {
    pub fn add(&mut self, clip: AudioClip) {
        if self.gain == 0.0 {
            self.gain = 1.0;
        }
        self.clips.push(clip);
    }
    /// End of the last clip (looping background beds don't extend the timeline).
    pub fn end_time(&self) -> f64 {
        self.clips
            .iter()
            .map(|c| c.end())
            .filter(|e| e.is_finite())
            .fold(0.0, f64::max)
    }
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }
    /// Shift every clip by `dt` seconds (used when nesting scenes in a movie).
    pub fn offset(&mut self, dt: f64) {
        for c in &mut self.clips {
            c.start += dt;
        }
    }
    /// Mix to interleaved stereo at `sample_rate` covering `[0, duration)`.
    pub fn mix(&self, sample_rate: u32, duration: f64) -> Vec<f32> {
        let n = (duration * sample_rate as f64).ceil() as usize;
        let mut out = vec![0.0f32; n * 2];
        let sr = sample_rate as f64;
        for c in &self.clips {
            let mut len = c.timeline_len().min(duration - c.start);
            if !c.timeline_len().is_finite() {
                // background bed: fade out at the end of the composition
                len = duration - c.start;
            }
            let src_len = c.source_len();
            if len <= 0.0 || src_len <= 0.0 {
                continue;
            }
            let i0 = (c.start * sr).max(0.0) as usize;
            let i1 = (((c.start + len) * sr) as usize).min(n);
            let (pl, pr) = ((1.0 - c.pan).min(1.0), (1.0 + c.pan).min(1.0));
            for i in i0..i1 {
                let t = i as f64 / sr - c.start;
                let mut src_t = t * c.rate;
                if c.looping {
                    src_t %= src_len;
                } else if src_t >= src_len {
                    break;
                }
                let pos = (c.trim_start + src_t) * SAMPLE_RATE as f64;
                let (l, r) = c.data.sample_at(pos);
                let mut g = c.volume.at(t);
                if c.fade_in > 0.0 && t < c.fade_in {
                    g *= (t / c.fade_in) as f32;
                }
                if c.fade_out > 0.0 && t > len - c.fade_out {
                    g *= ((len - t) / c.fade_out).max(0.0) as f32;
                }
                out[i * 2] += l * g * pl;
                out[i * 2 + 1] += r * g * pr;
            }
        }
        let gain = if self.gain == 0.0 { 1.0 } else { self.gain };
        // soft limiter: transparent below ~0.8, smooth above
        for s in &mut out {
            let x = *s * gain;
            *s = if x.abs() < 0.8 {
                x
            } else {
                x.signum() * (0.8 + 0.2 * ((x.abs() - 0.8) / 0.2).tanh())
            };
        }
        out
    }
}

/// Write interleaved f32 samples as 16-bit PCM WAV.
pub fn write_wav(path: &Path, samples: &[f32], sample_rate: u32, channels: u16) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let data_len = (samples.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&channels.to_le_bytes())?;
    f.write_all(&sample_rate.to_le_bytes())?;
    f.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())?;
    f.write_all(&(channels * 2).to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        f.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}

// ------------------------------------------------------------ analysis

/// Peak amplitudes in `n` equal buckets (Remotion's `getWaveformPortion`).
pub fn waveform(data: &AudioData, n: usize) -> Vec<f32> {
    let mono = data.mono();
    if mono.is_empty() || n == 0 {
        return vec![0.0; n];
    }
    let per = (mono.len() as f64 / n as f64).max(1.0);
    (0..n)
        .map(|i| {
            let a = (i as f64 * per) as usize;
            let b = (((i + 1) as f64 * per) as usize).min(mono.len()).max(a + 1);
            mono[a.min(mono.len() - 1)..b]
                .iter()
                .fold(0.0f32, |m, s| m.max(s.abs()))
        })
        .collect()
}

/// RMS loudness at time `t` over a short window (0..1-ish).
pub fn loudness_at(data: &AudioData, t: f64, window: f64) -> f32 {
    let c = (t * SAMPLE_RATE as f64) as isize;
    let w = (window * SAMPLE_RATE as f64 / 2.0) as isize;
    let mut acc = 0.0f32;
    let mut n = 0;
    for i in (c - w)..(c + w) {
        let (l, r) = data.frame(i);
        acc += (l * l + r * r) * 0.5;
        n += 1;
    }
    (acc / n.max(1) as f32).sqrt()
}

/// In-place radix-2 FFT on (re, im) pairs; length must be a power of two.
pub fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let nr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nr;
            }
        }
        len <<= 1;
    }
}

/// Frequency spectrum at time `t` in `bars` log-spaced bands, normalised to 0..1
/// (Remotion's `visualizeAudio`).
pub fn spectrum_at(data: &AudioData, t: f64, bars: usize) -> Vec<f32> {
    const N: usize = 2048;
    let c = (t * SAMPLE_RATE as f64) as isize - N as isize / 2;
    let mut re = vec![0.0f32; N];
    let mut im = vec![0.0f32; N];
    for i in 0..N {
        let (l, r) = data.frame(c + i as isize);
        let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (N - 1) as f32).cos();
        re[i] = (l + r) * 0.5 * w;
    }
    fft(&mut re, &mut im);
    let mags: Vec<f32> = (0..N / 2)
        .map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt())
        .collect();
    let (f_lo, f_hi) = (40.0f32, 16000.0f32);
    let bin = |f: f32| ((f / SAMPLE_RATE as f32) * N as f32) as usize;
    (0..bars)
        .map(|b| {
            let a = f_lo * (f_hi / f_lo).powf(b as f32 / bars as f32);
            let z = f_lo * (f_hi / f_lo).powf((b + 1) as f32 / bars as f32);
            let (ka, kz) = (bin(a).max(1), bin(z).max(bin(a) + 1).min(N / 2));
            let m = mags[ka.min(N / 2 - 1)..kz]
                .iter()
                .fold(0.0f32, |x, y| x.max(*y));
            // perceptual compression
            ((1.0 + m).ln() / 4.0).min(1.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_roundtrip_and_mix() {
        let mono: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        let a = AudioData::from_mono(&mono, 48_000);
        assert!((a.duration() - 0.1).abs() < 1e-6);
        let dir = std::env::temp_dir().join(format!("runwee-test-{}.wav", std::process::id()));
        a.save_wav(&dir).unwrap();
        let b = AudioData::load(&dir).unwrap();
        assert_eq!(b.frames(), a.frames());
        std::fs::remove_file(&dir).ok();
        let resampled = AudioData::from_mono(&mono, 24_000);
        assert!((resampled.duration() - 0.2).abs() < 1e-3);

        let mut tr = AudioTrack::default();
        let data = Arc::new(a);
        tr.add(AudioClip::new(data.clone(), 0.5).volume(0.5));
        tr.add(AudioClip::new(data, 1.0).looped(1.0).rate(2.0));
        assert!((tr.end_time() - 2.0).abs() < 1e-9);
        let mixed = tr.mix(48_000, 2.0);
        assert_eq!(mixed.len(), 96_000 * 2);
        assert!(mixed[..48_000].iter().all(|s| *s == 0.0));
        assert!(mixed[50_000..60_000].iter().any(|s| s.abs() > 0.1));
    }

    #[test]
    fn analysis() {
        let mono: Vec<f32> = (0..48_000)
            .map(|i| (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / 48_000.0).sin())
            .collect();
        let a = AudioData::from_mono(&mono, 48_000);
        let w = waveform(&a, 10);
        assert!(w.iter().all(|v| *v > 0.9));
        let sp = spectrum_at(&a, 0.5, 16);
        let peak = sp
            .iter()
            .enumerate()
            .fold((0, 0.0), |m, (i, v)| if *v > m.1 { (i, *v) } else { m });
        assert!(
            peak.0 > 2 && peak.0 < 10,
            "440Hz lands in a low-mid band, got {}",
            peak.0
        );
        assert!(loudness_at(&a, 0.5, 0.05) > 0.5);
    }
}
