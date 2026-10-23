//! `runwee` — render `.rhai` scene scripts to video.
//!
//! ```text
//! runwee render scene.rhai [-o out.mp4] [-q 2k] [--still 1.5] [--props '{"n":3}'] [--vertical]
//! runwee new my_scene.rhai
//! runwee info
//! runwee api
//! ```

mod script;

use anyhow::{bail, Context, Result};
use runwee::lesson::{self, Attach};
use runwee::prelude::*;
use std::path::{Path, PathBuf};

const USAGE: &str = "\
RunWee — fast STEM animation engine

USAGE:
    runwee render <script.rhai> [options]   Render a scene script
    runwee lesson <dir> [options]           Render an anims_bms JavaScript lesson (index.js)
                                            --part intro|lecture|all, --limit N, --info
    runwee new <script.rhai>                Write a starter script
    runwee info                             Show encoder / feature info
    runwee api                              List every script function

RENDER OPTIONS:
    -o, --output <path>     .mp4 .webm .mov .gif .png or a directory for PNG frames
    -q, --quality <q>       low | medium | high | 1080p30 | 2k | 4k   (default 2k)
                            high = 1080p60, 1080p30 = 1920x1080 @ 30 fps
    --still <t>             Render a single PNG at time t
    --props <json>          JSON object exposed to the script as PROPS
    --theme <name>          restyle every scene (dark, light, chalkboard, blueprint, paper, …)
    --background <name>     background preset, colour (#1e1e2e) or image/video path
    --vertical              9:16 portrait frame
    --from <t> --to <t>     Render only part of the timeline
    --transparent           Keep alpha (webm / mov / png)
    --no-audio              Skip the audio track
    --no-captions           Skip soft subtitles
    --audio-json <file>     If the script scheduled no audio, mux these clips in order
                            and extend each step/scene to the decoded clip duration.
                            See docs/LESSON_API.md for the JSON shapes.
";

const TEMPLATE: &str = r#"// RunWee scene script — render with: runwee render this_file.rhai -q medium
// `runwee api` lists every available function.
let head = add(title("Hello, RunWee!"));
play(write(head));

let eq = register(tex(`e^{i\pi} + 1 = 0`, 1.6));
play(fade_in(eq, UP));

let shape = register(circle(1.0).color(BLUE).fill_opacity(0.5).shift(DOWN * 2.2));
play(create(shape));
play(transform(shape, square(2.0).color(GREEN).fill_opacity(0.5).shift(DOWN * 2.2)));

voiceover("Every frame is a pure function of time.", |info| {
    play(indicate(eq));
});
wait(1.0);
"#;

struct Args {
    cmd: String,
    file: Option<PathBuf>,
    output: Option<String>,
    quality: Option<String>,
    still: Option<f64>,
    props: Option<String>,
    vertical: bool,
    from: f64,
    to: Option<f64>,
    transparent: bool,
    audio: bool,
    captions: bool,
    theme: Option<String>,
    background: Option<String>,
    part: Option<String>,
    limit: Option<usize>,
    info: bool,
    audio_json: Option<String>,
}

fn parse_args() -> Result<Args> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut a = Args {
        cmd: String::new(),
        file: None,
        output: None,
        quality: None,
        still: None,
        props: None,
        vertical: false,
        from: 0.0,
        to: None,
        transparent: false,
        audio: true,
        captions: true,
        theme: None,
        background: None,
        part: None,
        limit: None,
        info: false,
        audio_json: None,
    };
    let mut i = 0;
    let value = |i: &mut usize, flag: &str| -> Result<String> {
        *i += 1;
        raw.get(*i)
            .cloned()
            .with_context(|| format!("{flag} needs a value"))
    };
    let num = |s: String, flag: &str| -> Result<f64> {
        s.parse()
            .with_context(|| format!("{flag}: '{s}' is not a number"))
    };
    while i < raw.len() {
        let arg = raw[i].clone();
        match arg.as_str() {
            "-o" | "--output" => a.output = Some(value(&mut i, &arg)?),
            "-q" | "--quality" => a.quality = Some(value(&mut i, &arg)?),
            "--still" => a.still = Some(num(value(&mut i, &arg)?, &arg)?),
            "--props" => a.props = Some(value(&mut i, &arg)?),
            "--from" => a.from = num(value(&mut i, &arg)?, &arg)?,
            "--to" => a.to = Some(num(value(&mut i, &arg)?, &arg)?),
            "--vertical" => a.vertical = true,
            "--theme" => a.theme = Some(value(&mut i, &arg)?),
            "--background" | "--bg" => a.background = Some(value(&mut i, &arg)?),
            "--transparent" => a.transparent = true,
            "--no-audio" => a.audio = false,
            "--no-captions" => a.captions = false,
            "--part" => a.part = Some(value(&mut i, &arg)?),
            "--limit" => a.limit = Some(num(value(&mut i, &arg)?, &arg)? as usize),
            "--info" => a.info = true,
            "--audio-json" => a.audio_json = Some(value(&mut i, &arg)?),
            "-h" | "--help" => a.cmd = "help".into(),
            s if s.starts_with('-') => bail!("unknown option {s}\n\n{USAGE}"),
            s if a.cmd.is_empty() => a.cmd = s.to_string(),
            s if a.file.is_none() => a.file = Some(PathBuf::from(s)),
            s => bail!("unexpected argument {s}"),
        }
        i += 1;
    }
    Ok(a)
}

fn props_map(json: Option<&str>) -> Result<rhai::Map> {
    let Some(json) = json else {
        return Ok(rhai::Map::new());
    };
    let v: serde_json::Value = serde_json::from_str(json).context("--props is not valid JSON")?;
    if !v.is_object() {
        bail!("--props must be a JSON object");
    }
    let d: rhai::Dynamic = rhai::serde::to_dynamic(&v).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(d.cast::<rhai::Map>())
}

/// `runwee lesson <topic-dir>`: render an anims_bms JavaScript lesson as a video.
fn lesson_cmd(a: &Args) -> Result<()> {
    let dir = a
        .file
        .as_ref()
        .context("lesson: missing <lesson folder or index.js>")?;
    if a.info {
        let info = runwee_lesson::inspect(dir)?;
        println!("{} — {}", info.title, info.subtitle);
        println!(
            "presets: {}",
            info.presets
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "intro steps: {}, lecture steps: {}",
            info.intro.len(),
            info.lecture.len()
        );
        let words: usize = info
            .intro
            .iter()
            .chain(&info.lecture)
            .map(|s| s.text.split_whitespace().count())
            .sum();
        println!(
            "narration: {words} words (≈ {:.1} min)",
            words as f64 / 155.0
        );
        return Ok(());
    }
    let q = match a.quality.as_deref() {
        Some(s) => Quality::parse(s).with_context(|| format!("unknown quality '{s}'"))?,
        None => Quality::TwoK,
    };
    let cfg = q.config();
    let mut opts = runwee_lesson::LessonOptions::new(dir, cfg.fps);
    if let Some(p) = &a.part {
        opts.parts = runwee_lesson::Parts::parse(p)
            .with_context(|| format!("--part must be intro, lecture or all (got '{p}')"))?;
    }
    opts.limit_steps = a.limit;
    let started = std::time::Instant::now();
    let mut scene = runwee_lesson::build(&opts)?;
    eprintln!("lesson prepared in {:.1}s", started.elapsed().as_secs_f64());
    if let Some(name) = &a.theme {
        let th = Theme::preset(name).with_context(|| format!("unknown theme '{name}'"))?;
        scene.base_theme(th);
    }
    let stem = {
        let p = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.clone());
        let p = if p.file_name().is_some_and(|f| f == "index.js") {
            p.parent().map(|x| x.to_path_buf()).unwrap_or(p)
        } else {
            p
        };
        p.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("lesson")
            .to_string()
    };
    if let Some(t) = a.still {
        let p = a
            .output
            .clone()
            .unwrap_or_else(|| format!("media/{stem}.png"));
        if let Some(d) = Path::new(&p).parent() {
            if !d.as_os_str().is_empty() {
                std::fs::create_dir_all(d)?;
            }
        }
        render_still(&scene, t, cfg, &p)?;
        eprintln!("wrote {p}");
        return Ok(());
    }
    let p = a
        .output
        .clone()
        .unwrap_or_else(|| format!("media/{stem}.mp4"));
    eprintln!(
        "RunWee lesson: {} → {} ({}x{} @ {} fps, {:.1}s)",
        dir.display(),
        p,
        cfg.width,
        cfg.height,
        cfg.fps,
        scene.duration()
    );
    let mut ro = RenderOptions::new(&p, cfg);
    ro.start = a.from;
    ro.end = a.to;
    ro.audio = a.audio;
    ro.subtitles = a.captions;
    let stats = render(&scene, &ro)?;
    eprintln!(
        "done: {} frames in {:.2}s ({:.1} fps)",
        stats.frames, stats.seconds, stats.fps
    );
    Ok(())
}

fn render_cmd(a: &Args) -> Result<()> {
    let file = a.file.as_ref().context("render: missing <script.rhai>")?;
    let src =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let props = props_map(a.props.as_deref())?;
    if let Some(dir) = file.parent() {
        if !dir.as_os_str().is_empty() {
            std::env::set_var("RUNWEE_SCRIPT_DIR", dir);
        }
    }
    if a.vertical {
        set_frame_size(8.0 * 9.0 / 16.0, 8.0);
    }
    let mut out = script::run_script(&src, props)?;
    if let Some(name) = &a.theme {
        let th = Theme::preset(name).with_context(|| {
            format!(
                "unknown theme '{name}' (any of: {})",
                BACKGROUND_PRESETS.join(", ")
            )
        })?;
        for (scene, _) in &mut out.scenes {
            scene.base_theme(th.clone());
        }
    }
    if let Some(name) = &a.background {
        let bg = match Background::preset(name) {
            Some(bg) => bg,
            None if Path::new(name).exists() => {
                let ext = Path::new(name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if matches!(ext.as_str(), "mp4" | "mov" | "webm" | "mkv" | "avi") {
                    Background::video(name)?.dim(0.35)
                } else {
                    Background::image(name)?.dim(0.35)
                }
            }
            None => bail!(
                "unknown background '{name}' (presets: {})",
                BACKGROUND_PRESETS.join(", ")
            ),
        };
        for (scene, _) in &mut out.scenes {
            scene.base_background(bg.clone());
        }
    }
    if a.still.is_none() {
        if let Some(json_path) = &a.audio_json {
            attach_audio_json(&mut out, Path::new(json_path))?;
        }
    }
    let vertical = a.vertical || out.vertical;
    let q_name = a.quality.clone().or_else(|| out.quality.clone());
    let q = match q_name.as_deref() {
        Some(s) => Quality::parse(s).with_context(|| format!("unknown quality '{s}'"))?,
        None => Quality::TwoK,
    };
    let mut cfg = q.config();
    if vertical {
        cfg = cfg.portrait();
    }
    let comp = out.composition();
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("scene")
        .to_string();

    if let Some(t) = a.still {
        let p = a
            .output
            .clone()
            .unwrap_or_else(|| format!("media/{stem}.png"));
        if let Some(dir) = Path::new(&p).parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir)?;
            }
        }
        render_still(comp.as_ref(), t, cfg, &p)?;
        eprintln!("wrote {p}");
        return Ok(());
    }
    let p = a
        .output
        .clone()
        .unwrap_or_else(|| format!("media/{stem}.mp4"));
    eprintln!(
        "RunWee: {} → {} ({}x{} @ {} fps, {:.1}s)",
        file.display(),
        p,
        cfg.width,
        cfg.height,
        cfg.fps,
        comp.duration()
    );
    let mut opts = RenderOptions::new(&p, cfg);
    opts.start = a.from;
    opts.end = a.to;
    opts.transparent = a.transparent;
    opts.audio = a.audio;
    opts.subtitles = a.captions;
    let stats = render(comp.as_ref(), &opts)?;
    eprintln!(
        "done: {} frames in {:.2}s ({:.1} fps)",
        stats.frames, stats.seconds, stats.fps
    );
    Ok(())
}

fn attach_audio_json(out: &mut script::ScriptOutput, json_path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(json_path)
        .with_context(|| format!("reading {}", json_path.display()))?;
    let base = json_path.parent().unwrap_or_else(|| Path::new("."));
    let steps = lesson::parse_audio_plan(&text, base)
        .with_context(|| format!("parsing {}", json_path.display()))?;
    let mut scenes: Vec<&mut runwee::scene::Scene> =
        out.scenes.iter_mut().map(|(scene, _)| scene).collect();
    match lesson::attach_audio_plan(&mut scenes, &steps)? {
        Attach::Applied => {
            let clips: usize = steps.iter().map(|s| s.clips.len()).sum();
            eprintln!(
                "audio-json: {} step(s), {} clip(s) placed on the timeline",
                steps.len(),
                clips
            );
        }
        Attach::SkippedExistingAudio => {
            eprintln!("audio-json: script already scheduled audio; leaving it unchanged");
        }
        Attach::Empty => eprintln!("audio-json: no clips listed"),
    }
    Ok(())
}

fn info_cmd() {
    println!("RunWee {}", env!("CARGO_PKG_VERSION"));
    let ff = std::process::Command::new("ffmpeg")
        .arg("-version")
        .output();
    match ff {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            println!("ffmpeg: {}", s.lines().next().unwrap_or("found"));
        }
        _ => println!("ffmpeg: NOT FOUND (needed for mp4/webm/mov/gif and audio; PNG still works)"),
    }
    println!(
        "threads: {}",
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    );
    println!(
        "qualities: low 480p15 · medium 720p30 · high 1080p60 · 1080p30 · 2k 1440p60 · 4k 2160p60"
    );
    println!("backgrounds / themes: {}", BACKGROUND_PRESETS.join(", "));
    println!("formats: mp4 (h264) · webm (vp9, alpha) · mov (prores) · gif · png / png sequence");
    println!("script functions: {}", script::api_signatures().len());
}

fn main() -> Result<()> {
    let a = parse_args()?;
    match a.cmd.as_str() {
        "render" => render_cmd(&a),
        "lesson" => lesson_cmd(&a),
        "new" => {
            let p = a
                .file
                .clone()
                .unwrap_or_else(|| PathBuf::from("scene.rhai"));
            if p.exists() {
                bail!("{} already exists", p.display());
            }
            std::fs::write(&p, TEMPLATE)?;
            println!(
                "wrote {} — render it with: runwee render {} -q medium",
                p.display(),
                p.display()
            );
            Ok(())
        }
        "info" => {
            info_cmd();
            Ok(())
        }
        "api" => {
            for s in script::api_signatures() {
                println!("{s}");
            }
            Ok(())
        }
        "" | "help" => {
            print!("{USAGE}");
            Ok(())
        }
        other => bail!("unknown command '{other}'\n\n{USAGE}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_ode_solver() {
        let (engine, _) = script::test_engine();
        // x'' = -x from x = 1: x(pi) = -1, and the state array round-trips.
        let x: f64 = engine
            .eval("let tr = ode(|t, y| [y[1], -y[0]], [1.0, 0.0], 3.2, 0.001); tr.state(3.141592653589793)[0]")
            .unwrap();
        assert!((x + 1.0).abs() < 1e-4, "{x}");
        let lo: f64 = engine
            .eval("ode(|t, y| [y[1], -y[0]], [1.0, 0.0], 7.0).min(0)")
            .unwrap();
        assert!((lo + 1.0).abs() < 1e-3, "{lo}");
        assert!(engine
            .eval::<f64>("ode(|t, y| [y[0]], [1.0, 0.0], 1.0).duration()")
            .is_err());
    }

    #[test]
    fn example_scripts_compile() {
        let (engine, _) = script::test_engine();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/scripts");
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "rhai") {
                let src = std::fs::read_to_string(&p).unwrap();
                engine
                    .compile(&src)
                    .unwrap_or_else(|err| panic!("{}: {err}", p.display()));
                n += 1;
            }
        }
        assert!(n >= 4);
        engine.compile(TEMPLATE).unwrap();
    }

    #[test]
    fn lesson_audio_and_board_run() {
        let path = std::env::temp_dir().join("runwee-cli-audio-duration.wav");
        runwee::audio::AudioData::silence(0.4)
            .save_wav(&path)
            .unwrap();
        let (engine, _) = script::test_engine();
        let src = format!("audio_duration(\"{}\")", path.display());
        let d: f64 = engine.eval(&src).unwrap();
        assert!((d - 0.4).abs() < 0.02, "{d}");
        engine
            .run(
                r##"
                math_board(vec2(3.2, 0.4), 5.0, 4.5);
                let row = board_line("math", "q = ne");
                let arrows = field_arrows([[0.0, 0.0, 1.0], [1.5, 0.0, -1.0]], -1.0, 2.0, -1.0, 1.0, 4, 3, "#2F6FED");
                let v = vector_arrow(vec2(-1.0, -1.0), vec2(0.8, 0.4), "E", "#E24A3B");
            "##,
            )
            .unwrap();
    }

    #[test]
    fn api_is_large() {
        assert!(script::api_signatures().len() > 300);
    }

    #[test]
    fn props_parse() {
        let m = props_map(Some(r#"{"n": 3, "name": "x"}"#)).unwrap();
        assert_eq!(m.get("n").unwrap().as_int().unwrap(), 3);
        assert!(props_map(Some("[1]")).is_err());
    }
}
