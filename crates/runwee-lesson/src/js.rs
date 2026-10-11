//! QuickJS host for anims_bms lessons.

use anyhow::{anyhow, Context as _, Result};
use rquickjs::loader::{Loader, Resolver};
use rquickjs::module::Declared;
use rquickjs::{CatchResultExt, Context, Ctx, Function, Module, Object, Runtime, Value};
use std::path::{Path, PathBuf};

const HARNESS: &str = include_str!("../js/harness.js");

/// Globals a browser lesson may touch at module level or in callbacks.
const PRELUDE: &str = r#"
globalThis.window = globalThis;
globalThis.self = globalThis;
globalThis.console = globalThis.console || {
  log: (...a) => __rw_log(a.join(' ')), warn: (...a) => __rw_log(a.join(' ')),
  error: (...a) => __rw_log(a.join(' ')), info: () => {}, debug: () => {},
};
globalThis.performance = { now: () => 0 };
globalThis.requestAnimationFrame = () => 0;
globalThis.cancelAnimationFrame = () => {};
globalThis.setTimeout = () => 0;
globalThis.clearTimeout = () => {};
globalThis.setInterval = () => 0;
globalThis.clearInterval = () => {};
globalThis.devicePixelRatio = 1;
globalThis.navigator = { userAgent: 'runwee' };
globalThis.localStorage = { getItem: () => null, setItem: () => {}, removeItem: () => {} };
const __stubEl = () => ({
  style: {}, classList: { add() {}, remove() {}, toggle() {}, contains() { return false; } },
  appendChild() {}, removeChild() {}, setAttribute() {}, addEventListener() {},
  querySelector() { return null; }, querySelectorAll() { return []; }, innerHTML: '', textContent: '',
  getContext() { return null; },
});
globalThis.document = {
  getElementById: () => null, querySelector: () => null, querySelectorAll: () => [],
  createElement: () => __stubEl(), body: __stubEl(), addEventListener() {},
};
globalThis.addEventListener = () => {};
globalThis.parent = globalThis;
globalThis.__rw_math = (text, type, hl) => {
  const L = globalThis.__rwLines;
  if (!L || L.some(x => x.text === text)) return;
  L.push({ wi: globalThis.__rwWi | 0, text, type, hl });
};
globalThis.__rw_mathClear = () => {};
"#;

struct LessonResolver;

impl Resolver for LessonResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        if name.starts_with("runwee:") {
            return Ok(name.to_string());
        }
        let resolved = if name.starts_with('.') {
            let dir = Path::new(base).parent().unwrap_or(Path::new("/"));
            normalize(&dir.join(name))
        } else {
            PathBuf::from(name)
        };
        let s = resolved.to_string_lossy().replace('\\', "/");
        // The shared renderer module touches the DOM; lessons get the harness shim.
        if s.ends_with("/core/renderer.js") {
            return Ok("runwee:harness".into());
        }
        Ok(s)
    }
}

fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

struct LessonLoader;

impl Loader for LessonLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<rquickjs::loader::ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, Declared>> {
        if name == "runwee:harness" {
            return Module::declare(ctx.clone(), name, HARNESS);
        }
        let mut src = std::fs::read_to_string(name)
            .map_err(|e| rquickjs::Error::new_loading_message(name, e.to_string()))?;
        if name.ends_with(".json") {
            src = format!("export default {src};");
        }
        Module::declare(ctx.clone(), name, src)
    }
}

/// A loaded lesson module plus the harness that drives it.
pub struct JsLesson {
    _rt: Runtime,
    ctx: Context,
}

impl JsLesson {
    pub fn load(index_js: &Path) -> Result<Self> {
        let index = std::fs::canonicalize(index_js)
            .with_context(|| format!("lesson module not found: {}", index_js.display()))?;
        let rt = Runtime::new()?;
        rt.set_memory_limit(1 << 30);
        rt.set_max_stack_size(4 << 20);
        rt.set_loader(LessonResolver, LessonLoader);
        let ctx = Context::full(&rt)?;
        ctx.with(|ctx| -> Result<()> {
            let g = ctx.globals();
            g.set(
                "__rw_measure",
                Function::new(ctx.clone(), |font: String, text: String| -> f64 {
                    measure_text(&font, &text)
                })?,
            )?;
            g.set(
                "__rw_log",
                Function::new(ctx.clone(), |msg: String| {
                    eprintln!("[lesson] {msg}");
                })?,
            )?;
            ctx.eval::<(), _>(PRELUDE)
                .catch(&ctx)
                .map_err(|e| anyhow!("prelude: {e}"))?;
            let entry = format!(
                "import * as H from 'runwee:harness';\n\
                 import * as L from '{}';\n\
                 globalThis.__H = H;\n\
                 globalThis.__L = L;\n",
                index
                    .to_string_lossy()
                    .replace('\\', "/")
                    .replace('\'', "\\'")
            );
            let promise = Module::evaluate(ctx.clone(), "runwee:entry", entry)
                .catch(&ctx)
                .map_err(|e| anyhow!("loading lesson: {e}"))?;
            promise
                .finish::<Value>()
                .catch(&ctx)
                .map_err(|e| anyhow!("loading lesson: {e}"))?;
            Ok(())
        })?;
        Ok(Self { _rt: rt, ctx })
    }

    fn call_harness(&self, name: &str, args: Vec<HarnessArg>) -> Result<String> {
        self.ctx.with(|ctx| -> Result<String> {
            let h: Object = ctx.globals().get("__H")?;
            let f: Function = h.get(name)?;
            let mut a = rquickjs::function::Args::new(ctx.clone(), args.len());
            for arg in args {
                match arg {
                    HarnessArg::Str(s) => a.push_arg(s)?,
                    HarnessArg::Num(n) => a.push_arg(n)?,
                    HarnessArg::Lesson => {
                        let l: Value = ctx.globals().get("__L")?;
                        a.push_arg(l)?
                    }
                }
            }
            let v: Value = f
                .call_arg(a)
                .catch(&ctx)
                .map_err(|e| anyhow!("{name}: {e}"))?;
            Ok(v.as_string()
                .map(|s| s.to_string())
                .transpose()?
                .unwrap_or_default())
        })
    }

    /// The lesson manifest as JSON (title, timelines, presets, derivations).
    pub fn info(&self) -> Result<String> {
        self.call_harness("load", vec![HarnessArg::Lesson])
    }
    pub fn begin_step(&self, part: &str, index: usize) -> Result<()> {
        self.call_harness(
            "beginStep",
            vec![HarnessArg::Str(part.into()), HarnessArg::Num(index as f64)],
        )
        .map(|_| ())
    }
    /// Advance one frame; returns the frame's canvas commands as JSON.
    pub fn tick(&self, dt: f64, progress: f64) -> Result<String> {
        self.call_harness("tick", vec![HarnessArg::Num(dt), HarnessArg::Num(progress)])
    }
    /// A preset in its default state after `frames` frames (canvas commands JSON).
    pub fn preview_preset(&self, id: &str, frames: usize, dt: f64) -> Result<String> {
        self.call_harness(
            "previewPreset",
            vec![
                HarnessArg::Str(id.into()),
                HarnessArg::Num(frames as f64),
                HarnessArg::Num(dt),
            ],
        )
    }
    /// Math-overlay lines of a step (JSON array of {wi, text, type, hl}).
    pub fn overlay(&self, step_id: &str, max_words: usize) -> Result<String> {
        self.call_harness(
            "overlay",
            vec![
                HarnessArg::Str(step_id.into()),
                HarnessArg::Num(max_words as f64),
            ],
        )
    }
}

enum HarnessArg {
    Str(String),
    Num(f64),
    Lesson,
}

/// Text width for `measureText`, using the same fonts as the replay.
pub fn measure_text(font: &str, text: &str) -> f64 {
    runwee::canvas2d::text_width(font, text)
}
