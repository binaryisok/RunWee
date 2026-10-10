//! Parallel frame pipeline and encoders (ffmpeg for video, PNG sequences, stills).

use crate::render::*;
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use tiny_skia::Pixmap;

/// Anything that can produce frames: a `Scene`, a `Movie`, a functional composition…
pub trait Composition: Send + Sync {
    /// Length in seconds.
    fn duration(&self) -> f64;
    /// Draw the frame at time `t` (seconds) into `pm` (already sized to `cfg`),
    /// clearing it with the composition's background first.
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig);
    /// Draw on top of existing pixels without clearing (used for layering).
    fn render_layer(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        let mut tmp = Pixmap::new(cfg.width, cfg.height).expect("bad size");
        self.render_frame(t, &mut tmp, cfg);
        pm.draw_pixmap(
            0,
            0,
            tmp.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
    }
    /// Audio clips of this composition (timeline relative to its start).
    fn audio_track(&self) -> crate::audio::AudioTrack {
        Default::default()
    }
    /// Caption cues of this composition (relative to its start).
    fn caption_cues(&self) -> Vec<crate::captions::Cue> {
        vec![]
    }
    /// Mixed audio (interleaved stereo f32 at `sample_rate`), if any.
    fn audio(&self, sample_rate: u32) -> Option<Vec<f32>> {
        let tr = self.audio_track();
        if tr.is_empty() {
            None
        } else {
            Some(tr.mix(sample_rate, self.duration()))
        }
    }
    /// Subtitles in SRT format, if any.
    fn subtitles_srt(&self) -> Option<String> {
        let cues = self.caption_cues();
        if cues.is_empty() {
            None
        } else {
            Some(crate::captions::serialize_srt_cues(&cues))
        }
    }
}

impl Composition for crate::scene::Scene {
    fn duration(&self) -> f64 {
        crate::scene::Scene::duration(self)
    }
    fn render_frame(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.render_into(t, pm, cfg);
    }
    fn render_layer(&self, t: f64, pm: &mut Pixmap, cfg: RenderConfig) {
        self.render_impl(t, pm, cfg, false);
    }
    fn audio_track(&self) -> crate::audio::AudioTrack {
        self.audio.clone()
    }
    fn caption_cues(&self) -> Vec<crate::captions::Cue> {
        self.captions.cues.clone()
    }
}

/// Output container / codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Mp4,
    WebM,
    Mov,
    Gif,
    PngSequence,
}

impl Format {
    pub fn from_path(p: &Path) -> Format {
        match p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
        {
            Some("webm") => Format::WebM,
            Some("mov") => Format::Mov,
            Some("gif") => Format::Gif,
            Some("mp4") | Some("m4v") | Some("mkv") => Format::Mp4,
            None => Format::PngSequence,
            _ => Format::Mp4,
        }
    }
}

/// How to render a composition.
#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub config: RenderConfig,
    pub output: PathBuf,
    /// x264/vp9 quality (lower = better). 16 is visually lossless for animation.
    pub crf: u8,
    /// x264 preset.
    pub preset: String,
    /// Keep the alpha channel (WebM VP9 / MOV ProRes 4444 / PNG).
    pub transparent: bool,
    /// Render only `[start, end)` seconds.
    pub start: f64,
    pub end: Option<f64>,
    pub progress: bool,
    /// Mux audio if the composition has any.
    pub audio: bool,
    /// Mux soft subtitles (mov_text) and write a sidecar `.srt`/`.vtt`.
    pub subtitles: bool,
    /// Frames rendered per parallel batch (0 = automatic).
    pub batch: usize,
}

impl RenderOptions {
    pub fn new(output: impl Into<PathBuf>, config: RenderConfig) -> Self {
        Self {
            config,
            output: output.into(),
            crf: 16,
            preset: "fast".into(),
            transparent: false,
            start: 0.0,
            end: None,
            progress: true,
            audio: true,
            subtitles: true,
            batch: 0,
        }
    }
    pub fn quality(output: impl Into<PathBuf>, q: Quality) -> Self {
        Self::new(output, q.config())
    }
}

/// Statistics returned after rendering.
#[derive(Clone, Debug)]
pub struct RenderStats {
    pub frames: usize,
    pub seconds: f64,
    pub fps: f64,
    pub output: PathBuf,
}

pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn unpremultiply(data: &mut [u8]) {
    for px in data.as_chunks_mut::<4>().0 {
        let a = px[3] as u32;
        if a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// Render a still frame at time `t` to PNG.
pub fn render_still(
    comp: &dyn Composition,
    t: f64,
    cfg: RenderConfig,
    path: impl AsRef<Path>,
) -> Result<()> {
    let mut pm = Pixmap::new(cfg.width, cfg.height).context("bad size")?;
    comp.render_frame(t, &mut pm, cfg);
    save_png(&pm, path)
}

/// Render a frame to an in-memory pixmap.
pub fn render_pixmap(comp: &dyn Composition, t: f64, cfg: RenderConfig) -> Pixmap {
    let mut pm = Pixmap::new(cfg.width, cfg.height).expect("bad size");
    comp.render_frame(t, &mut pm, cfg);
    pm
}

fn temp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("runwee-{}-{}", std::process::id(), name));
    p
}

/// Render the composition to a file.
pub fn render(comp: &dyn Composition, opts: &RenderOptions) -> Result<RenderStats> {
    let cfg = opts.config;
    let fps = cfg.fps;
    let duration = comp.duration();
    let end = opts.end.unwrap_or(duration).min(duration);
    let first = (opts.start * fps).round() as usize;
    let last = ((end * fps).round() as usize).max(first + 1);
    let total = last - first;
    let format = Format::from_path(&opts.output);
    let started = Instant::now();
    if let Some(dir) = opts.output.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }

    let batch = if opts.batch > 0 {
        opts.batch
    } else {
        (rayon::current_num_threads() * 2).max(2)
    };
    let render_batch = |frames: &[usize]| -> Vec<Vec<u8>> {
        frames
            .par_iter()
            .map_init(
                || Pixmap::new(cfg.width, cfg.height).expect("bad size"),
                |pm, &i| {
                    let t = i as f64 / fps;
                    comp.render_frame(t, pm, cfg);
                    let mut data = pm.data().to_vec();
                    if opts.transparent {
                        unpremultiply(&mut data);
                    }
                    data
                },
            )
            .collect()
    };
    let indices: Vec<usize> = (first..last).collect();
    let mut done = 0usize;
    let mut last_report = Instant::now();
    let mut report = |done: usize| {
        if opts.progress && (last_report.elapsed().as_secs_f64() > 1.0 || done == total) {
            last_report = Instant::now();
            let el = started.elapsed().as_secs_f64();
            let rate = done as f64 / el.max(1e-9);
            let eta = (total - done) as f64 / rate.max(1e-9);
            eprint!(
                "\r  rendering {:>6}/{} frames  {:>6.1} fps  eta {:>5.1}s   ",
                done, total, rate, eta
            );
            if done == total {
                eprintln!();
            }
        }
    };

    if format == Format::PngSequence {
        std::fs::create_dir_all(&opts.output)?;
        for chunk in indices.chunks(batch) {
            let frames = render_batch(chunk);
            frames.into_par_iter().zip(chunk.par_iter()).try_for_each(
                |(data, &i)| -> Result<()> {
                    let img =
                        image::RgbaImage::from_raw(cfg.width, cfg.height, data).context("frame")?;
                    let mut d = img;
                    if !opts.transparent {
                        for p in d.pixels_mut() {
                            p.0[3] = 255;
                        }
                    }
                    d.save(opts.output.join(format!("frame_{i:06}.png")))?;
                    Ok(())
                },
            )?;
            done += chunk.len();
            report(done);
        }
        return Ok(RenderStats {
            frames: total,
            seconds: started.elapsed().as_secs_f64(),
            fps: total as f64 / started.elapsed().as_secs_f64(),
            output: opts.output.clone(),
        });
    }

    if !ffmpeg_available() {
        bail!("ffmpeg not found in PATH (needed for {:?} output). Install ffmpeg or render a PNG sequence (output path without extension).", format);
    }

    // Audio & subtitles side files.
    let mut audio_file = None;
    if opts.audio && format != Format::Gif {
        if let Some(samples) = comp.audio(crate::audio::SAMPLE_RATE) {
            let s0 = (opts.start * crate::audio::SAMPLE_RATE as f64) as usize * 2;
            let s1 = ((end * crate::audio::SAMPLE_RATE as f64) as usize * 2).min(samples.len());
            let p = temp_path("audio.wav");
            crate::audio::write_wav(&p, &samples[s0.min(s1)..s1], crate::audio::SAMPLE_RATE, 2)?;
            audio_file = Some(p);
        }
    }
    let mut srt_file = None;
    if opts.subtitles {
        if let Some(srt) = comp.subtitles_srt() {
            let side = opts.output.with_extension("srt");
            std::fs::write(&side, &srt)?;
            if let Some(vtt) = crate::captions::srt_to_vtt(&srt) {
                std::fs::write(opts.output.with_extension("vtt"), vtt)?;
            }
            if format == Format::Mp4 && opts.start == 0.0 {
                srt_file = Some(side);
            }
        }
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
    cmd.args(["-f", "rawvideo", "-pix_fmt", "rgba"]);
    cmd.args(["-s", &format!("{}x{}", cfg.width, cfg.height)]);
    cmd.args(["-r", &format!("{fps}")]);
    cmd.args(["-i", "-"]);
    if let Some(a) = &audio_file {
        cmd.arg("-i").arg(a);
    }
    if let Some(s) = &srt_file {
        cmd.arg("-i").arg(s);
    }
    let bt709 = [
        "-colorspace",
        "bt709",
        "-color_primaries",
        "bt709",
        "-color_trc",
        "bt709",
    ];
    match format {
        Format::Mp4 => {
            cmd.args([
                "-vf",
                "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
            ]);
            cmd.args([
                "-c:v",
                "libx264",
                "-preset",
                &opts.preset,
                "-crf",
                &opts.crf.to_string(),
            ]);
            cmd.args(["-tune", "animation", "-movflags", "+faststart"]);
            cmd.args(bt709);
        }
        Format::WebM => {
            if opts.transparent {
                cmd.args(["-pix_fmt", "yuva420p"]);
            } else {
                cmd.args([
                    "-vf",
                    "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
                ]);
                cmd.args(bt709);
            }
            cmd.args([
                "-c:v",
                "libvpx-vp9",
                "-b:v",
                "0",
                "-crf",
                &(opts.crf as u32 + 14).to_string(),
            ]);
            cmd.args(["-row-mt", "1", "-deadline", "good", "-cpu-used", "4"]);
        }
        Format::Mov => {
            if opts.transparent {
                cmd.args([
                    "-c:v",
                    "prores_ks",
                    "-profile:v",
                    "4444",
                    "-pix_fmt",
                    "yuva444p10le",
                ]);
            } else {
                cmd.args([
                    "-c:v",
                    "prores_ks",
                    "-profile:v",
                    "3",
                    "-pix_fmt",
                    "yuv422p10le",
                ]);
            }
        }
        Format::Gif => {
            cmd.args([
                "-vf",
                "split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=sierra2_4a",
            ]);
        }
        Format::PngSequence => unreachable!(),
    }
    if audio_file.is_some() {
        let codec = if format == Format::WebM {
            "libopus"
        } else {
            "aac"
        };
        cmd.args(["-c:a", codec, "-b:a", "192k"]);
        cmd.args(["-map", "0:v:0", "-map", "1:a:0"]);
        if srt_file.is_some() {
            cmd.args([
                "-map",
                "2:s:0",
                "-c:s",
                "mov_text",
                "-metadata:s:s:0",
                "language=eng",
            ]);
        }
    } else if srt_file.is_some() {
        cmd.args([
            "-map",
            "0:v:0",
            "-map",
            "1:s:0",
            "-c:s",
            "mov_text",
            "-metadata:s:s:0",
            "language=eng",
        ]);
    }
    cmd.arg(&opts.output);
    cmd.stdin(Stdio::piped());
    let mut child = cmd.spawn().context("failed to start ffmpeg")?;
    let mut stdin = child.stdin.take().unwrap();

    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<Vec<u8>>>(2);
    let writer = std::thread::spawn(move || -> std::io::Result<()> {
        for frames in rx {
            for f in frames {
                stdin.write_all(&f)?;
            }
        }
        Ok(())
    });
    for chunk in indices.chunks(batch) {
        let frames = render_batch(chunk);
        if tx.send(frames).is_err() {
            break;
        }
        done += chunk.len();
        report(done);
    }
    drop(tx);
    let write_res = writer.join().expect("writer thread panicked");
    let status = child.wait()?;
    if let Some(a) = audio_file {
        let _ = std::fs::remove_file(a);
    }
    write_res.context("writing frames to ffmpeg")?;
    if !status.success() {
        bail!("ffmpeg failed with {status}");
    }
    let secs = started.elapsed().as_secs_f64();
    Ok(RenderStats {
        frames: total,
        seconds: secs,
        fps: total as f64 / secs.max(1e-9),
        output: opts.output.clone(),
    })
}

/// Convenience entry point for example programs and quick scripts.
///
/// Reads `-o <file>`, `-q <low|medium|high|2k|4k>`, `--still <t>`, `--vertical`
/// from the command line (or `RUNWEE_OUT` / `RUNWEE_QUALITY`), then renders.
pub fn run(comp: &dyn Composition, default_name: &str) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut out = std::env::var("RUNWEE_OUT").ok();
    let mut quality = std::env::var("RUNWEE_QUALITY").ok();
    let mut still: Option<f64> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                out = args.get(i + 1).cloned();
                i += 1;
            }
            "-q" | "--quality" => {
                quality = args.get(i + 1).cloned();
                i += 1;
            }
            "--still" => {
                still = args.get(i + 1).and_then(|s| s.parse().ok());
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    let q = quality
        .as_deref()
        .and_then(Quality::parse)
        .unwrap_or(Quality::TwoK);
    let mut cfg = q.config();
    let (fw, fh) = crate::mobject::frame_size();
    if fh > fw {
        cfg = cfg.portrait();
    }
    if let Some(t) = still {
        let p = out.unwrap_or_else(|| format!("media/{default_name}.png"));
        render_still(comp, t, cfg, &p)?;
        eprintln!("wrote {p}");
        return Ok(());
    }
    let p = out.unwrap_or_else(|| format!("media/{default_name}.mp4"));
    eprintln!(
        "RunWee: {} → {} ({}x{} @ {} fps, {:.1}s)",
        default_name,
        p,
        cfg.width,
        cfg.height,
        cfg.fps,
        comp.duration()
    );
    let stats = render(comp, &RenderOptions::new(&p, cfg))?;
    eprintln!(
        "done: {} frames in {:.2}s ({:.1} fps)",
        stats.frames, stats.seconds, stats.fps
    );
    Ok(())
}
