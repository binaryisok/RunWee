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
use runwee::prelude::*;
use std::path::{Path, PathBuf};

const USAGE: &str = "\
RunWee — fast STEM animation engine

USAGE:
    runwee render <script.rhai> [options]   Render a scene script
    runwee new <script.rhai>                Write a starter script
    runwee info                             Show encoder / feature info
    runwee api                              List every script function

RENDER OPTIONS:
    -o, --output <path>     .mp4 .webm .mov .gif .png or a directory for PNG frames
    -q, --quality <q>       low | medium | high | 2k | 4k   (default 2k)
    --still <t>             Render a single PNG at time t
    --props <json>          JSON object exposed to the script as PROPS
    --vertical              9:16 portrait frame
    --from <t> --to <t>     Render only part of the timeline
    --transparent           Keep alpha (webm / mov / png)
    --no-audio              Skip the audio track
    --no-captions           Skip soft subtitles
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
            "--transparent" => a.transparent = true,
            "--no-audio" => a.audio = false,
            "--no-captions" => a.captions = false,
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

fn render_cmd(a: &Args) -> Result<()> {
    let file = a.file.as_ref().context("render: missing <script.rhai>")?;
    let src =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let props = props_map(a.props.as_deref())?;
    if a.vertical {
        set_frame_size(8.0 * 9.0 / 16.0, 8.0);
    }
    let out = script::run_script(&src, props)?;
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
    println!("qualities: low 480p15 · medium 720p30 · high 1080p60 · 2k 1440p60 · 4k 2160p60");
    println!("formats: mp4 (h264) · webm (vp9, alpha) · mov (prores) · gif · png / png sequence");
    println!("script functions: {}", script::api_signatures().len());
}

fn main() -> Result<()> {
    let a = parse_args()?;
    match a.cmd.as_str() {
        "render" => render_cmd(&a),
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
