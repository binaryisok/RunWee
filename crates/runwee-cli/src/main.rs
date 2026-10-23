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
    runwee lesson <dir> [options]           Render an anims_bms JavaScript lesson (index.js)
                                            --part intro|lecture|all, --limit N, --info
    runwee new <script.rhai>                Write a starter script
    runwee info                             Show encoder / feature info
    runwee api                              List every script function

RENDER OPTIONS:
    -o, --output <path>     .mp4 .webm .mov .gif .png or a directory for PNG frames
    -q, --quality <q>       low | medium | high | 2k | 4k   (default 2k)
    --still <t>             Render a single PNG at time t
    --props <json>          JSON object exposed to the script as PROPS
    --theme <name>          restyle every scene (dark, light, chalkboard, blueprint, paper, …)
    --background <name>     background preset, colour (#1e1e2e) or image/video path
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
    theme: Option<String>,
    background: Option<String>,
    part: Option<String>,
    limit: Option<usize>,
    info: bool,
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
    fn advanced_bindings_run() {
        let src = r#"
            let o = register(star(5, 1.5, 0.7));
            play(spring_pop_in(o, "bouncy"), 0.5);
            play(smart_morph(o, circle(1.0)), 0.5);
            let b = beat(120, 1, #{kick: "X...X...X...X...", hat: "x.x.x.x.x.x.x.x."});
            let k = beat_env(b, "kick", 0.0, 0.1);
            if k < 0.9 { throw "kick envelope should start at 1"; }
            let n = fbm([0.3, 0.4], 4, 1) + perlin([1.2, 0.1], 2) + wiggle_at(1.0, 2.0, 1.0, 3);
            add(sketchy(square(2.0), 0.05, 1));
            add(lsystem("koch", 2, 2.0, WHITE, BLUE));
            particles("fire", vec2(0.0, -2.0));
            rope(vec2(0.0, 3.0), vec2(2.0, 3.0), RED, 1.0);
            effect_between("ascii", 0.02, 0.0, 1.0);
            wait(b.duration);
        "#;
        // Only one script may run per process, so the STEM bindings ride along.
        let stem = r#"
            let fr = fourier(circle(1.0), 8);
            add(epicycles(fr, 0.5, vec2(0.0, 0.0), WHITE));
            add(linear_map([1.0, 1.0, 0.0, 1.0], 0.5, 2.0, true));
            if (taylor("exp", 8, 1.0) - taylor_target("exp", 1.0)).abs() > 1e-3 { throw "taylor"; }
            add(domain_coloring("z^2", vec2(0.0, 0.0), 2.0, 2.0, 1.0, 32));
            add(ulam_spiral(121, 0.05, YELLOW, GRAY));
            add(ripple_tank([vec2(-1.0, 0.0), vec2(1.0, 0.0)], vec2(0.0, 0.0), 2.0, 2.0, 0.4, 0.5));
            add(prism_demo(vec2(0.0, 0.0), 1.0, 1.0));
            add(kepler_orbit(2.0, 0.5, vec2(0.0, 0.0), 0.25, RED));
            add(orbital(2, 1, 0, vec2(0.0, 0.0), 2.0));
            let ph = titration_ph(0.1, 25.0, 0.1, 30.0, 0.0);
            if ph < 11.0 { throw "excess strong base should be basic"; }
            let ep = epidemic(30, vec2(0.0, 0.0), 3.0, 3.0, 2.0, 1);
            let c = epidemic_counts(ep, 1.0);
            if c[0] + c[1] + c[2] != 30 { throw "SIR counts must sum to N"; }
            let st = sort_trace("merge", [5, 3, 9, 1, 7]);
            add(sort_bars(st, st.len, 2.0, 1.0));
            let p = path_search("astar", 12, 8, 0.1, 2);
            let q = path_search("dijkstra", 12, 8, 0.1, 2);
            if p.cost != q.cost { throw "A* must match Dijkstra's optimal cost"; }
            add(path_view(p, 1000, 0.1));
            add(bst_view(bst([4, 2, 6, 1, 3], 0.3, 0.5), 5, 0.1));
            add(nn_view(neural_net([2, 3, 1], 1), [0.5, -0.5], 2.0, 1.0, 1.0));
            add(galton_view(galton(20, 5, 1), 2.0, 0.05));
            add(cellular(110, 31, 10, vec2(0.0, 0.0), 2.0, WHITE));
            add(life_view(life_gun(10), 5, vec2(0.0, 0.0), 2.0, 1.2, GREEN));
            wait(0.5);
        "#;
        // Storytelling, poetry, grammar and classroom bindings ride along too.
        let story = r##"
            let c = character("grandpa");
            add(c.draw("wave", 0.5, 1.0, 1.0, vec2(0.0, 0.0)));
            add(critter("crow").sized(0.8).draw(0.2, 0.0, 1.0, 1.0, vec2(1.0, 1.0)));
            add(landscape(vec2(0.0, 0.0), 14.0, 8.0, "dusk", 1.0, 0.3, 2));
            add(bubble("thought", "Hmm…", vec2(0.0, 2.0), 3.0, vec2(0.0, 0.5), 0.3, 1.0));
            add(folk_frame("madhubani", vec2(0.0, 0.0), 12.0, 7.0, 0.6, 0.0, 1));
            add(rain(vec2(0.0, 0.0), 6.0, 4.0, 1.0, 30, "#88aaff", 0.2, 3));
            let lay = poem("T", "A", "Neither you nor I:\nThe wind is passing by.", "story", 0.3, vec2(-5.0, 2.0), 1.6);
            if scheme(lay) != "AA" { throw "long-i rhyme"; }
            add(reveal_poem(lay, 0.5, "typewriter", BLACK));
            add(figure_callout(lay, "simile", 0, 0, 1, vec2(3.0, 0.0), "note", 1.0));
            if count_syllables("trembling") != 2 { throw "syllables"; }
            let pv = passive_voice("the boy", "kick", "the ball", "past simple");
            if pv != "The ball was kicked by the boy." { throw "passive: " + pv; }
            if article_for("hour") != "an" { throw "article"; }
            if !verb_agrees("She", "goes") { throw "agreement"; }
            add(pos_chips("The crow drank water.", vec2(0.0, 0.0), 0.3, 1.0, true));
            add(parse_tree("The crow found a jug.", vec2(0.0, 3.0), 0.9, 0.3, 1.0));
            add(quiz_card("Q?", ["a", "an"], 1, -1, vec2(0.0, 0.0), 5.0, 1.0, 0.0));
            add(plot_mountain([["Start", "a"], ["Peak", "b"], ["End", "c"]], vec2(0.0, 0.0), 8.0, 5.0, 1.0, 0.0));
            add(mood_meter("x", [["a", -0.5], ["b", 0.5]], vec2(0.0, 0.0), 6.0, 4.0, 1.0));
            add(word_cloud([["kind", 3.0], ["brave", 2.0]], vec2(0.0, 0.0), 6.0, 4.0, 1, 1.0));
            add(letter_page(["1 Road"], "1 May", "Dear Sam,", "Hello.", "Yours,", "Asha", vec2(0.0, 0.0), 8.0, 6.0, 1.0));
            wait(0.2);
        "##;
        let src = format!("{src}\n{stem}\n{story}");
        let out = script::run_script(&src, rhai::Map::new()).unwrap();
        assert!(!out.scenes.is_empty());
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
