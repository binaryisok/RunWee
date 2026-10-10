//! Question-solving videos: a `Problem` template (statement → given/find → worked
//! steps with auto-scrolling → boxed answer), narration-aware pacing, algebraic
//! derivations with matching transforms, multiple-choice quizzes, and step
//! generators (quadratic equations, uniformly accelerated motion, Ohm's law,
//! stoichiometry).

use crate::anim::*;
use crate::color::*;
use crate::extra::add_text_word_by_word;
use crate::math::*;
use crate::mobject::*;
use crate::rate::Rate;
use crate::scene::{Obj, Scene};
use crate::shapes::*;
use crate::tex::{tex_sized, MathTex};
use crate::text::{bold_text, paragraph, text_sized, Align};
use crate::voice::Voiceover;

/// One worked step.
#[derive(Clone, Debug, Default)]
pub struct Step {
    /// Short explanation shown above the equation (plain text).
    pub explain: String,
    /// The equation (TeX). Empty = text-only step.
    pub tex: String,
    /// TeX substrings to highlight in the equation.
    pub highlight: Vec<String>,
    /// Spoken narration (falls back to `explain`).
    pub narration: Option<String>,
    /// Morph from the previous equation instead of writing fresh.
    pub transform_from_previous: bool,
}

impl Step {
    pub fn new(explain: &str, tex: &str) -> Self {
        Self {
            explain: explain.into(),
            tex: tex.into(),
            ..Default::default()
        }
    }
    pub fn highlight(mut self, sub: &str) -> Self {
        self.highlight.push(sub.into());
        self
    }
    pub fn say(mut self, narration: &str) -> Self {
        self.narration = Some(narration.into());
        self
    }
    pub fn morph(mut self) -> Self {
        self.transform_from_previous = true;
        self
    }
}

/// A complete problem with its solution.
#[derive(Clone, Default)]
pub struct Problem {
    pub title: String,
    /// Problem statement; may contain inline `$math$`.
    pub statement: String,
    pub given: Vec<String>,
    pub find: Vec<String>,
    pub diagram: Option<Mobject>,
    pub steps: Vec<Step>,
    /// Final answer (TeX).
    pub answer: String,
    pub accent: Option<Color>,
    /// Narrate statement/steps (captions + pacing; real audio via RUNWEE_TTS).
    pub narrate: bool,
    /// Seconds to hold each step when not narrated.
    pub step_pause: f64,
}

impl Problem {
    pub fn new(title: &str, statement: &str) -> Self {
        Self {
            title: title.into(),
            statement: statement.into(),
            step_pause: 1.0,
            narrate: true,
            ..Default::default()
        }
    }
    pub fn given(mut self, g: &[&str]) -> Self {
        self.given = g.iter().map(|s| s.to_string()).collect();
        self
    }
    pub fn find(mut self, f: &[&str]) -> Self {
        self.find = f.iter().map(|s| s.to_string()).collect();
        self
    }
    pub fn diagram(mut self, d: Mobject) -> Self {
        self.diagram = Some(d);
        self
    }
    pub fn step(mut self, s: Step) -> Self {
        self.steps.push(s);
        self
    }
    pub fn steps(mut self, s: Vec<Step>) -> Self {
        self.steps.extend(s);
        self
    }
    pub fn answer(mut self, a: &str) -> Self {
        self.answer = a.into();
        self
    }
    pub fn accent(mut self, c: Color) -> Self {
        self.accent = Some(c);
        self
    }
    pub fn silent(mut self) -> Self {
        self.narrate = false;
        self
    }
}

/// Turn TeX into readable/speakable text ("v = 13\\,\\mathrm{m/s}" → "v = 13 m/s",
/// "x^2" → "x squared", "\\frac{a}{b}" → "a over b").
pub fn tex_to_speech(src: &str) -> String {
    let mut s = src.to_string();
    let reps: &[(&str, &str)] = &[
        ("\\quad", ", "),
        ("\\qquad", ", "),
        ("\\,", " "),
        ("\\;", " "),
        ("\\:", " "),
        ("\\ ", " "),
        ("\\!", ""),
        ("\\times", " times "),
        ("\\cdot", " times "),
        ("\\approx", " approximately "),
        ("\\pm", " plus or minus "),
        ("\\Delta", "delta"),
        ("\\Omega", " ohms"),
        ("\\theta", "theta"),
        ("\\pi", "pi"),
        ("\\sqrt", "square root of "),
        ("\\to", " to "),
        ("\\rightarrow", " gives "),
        ("\\longrightarrow", " gives "),
        ("^2", " squared"),
        ("^3", " cubed"),
        ("m/s squared", "meters per second squared"),
        ("m/s", "meters per second"),
    ];
    // \frac{a}{b} / \tfrac / \dfrac → (a over b)
    for cmd in ["\\frac", "\\tfrac", "\\dfrac"] {
        while let Some(p) = s.find(cmd) {
            let rest = &s[p + cmd.len()..];
            let (a, r1) = take_group(rest);
            let (b, r2) = take_group(r1);
            let consumed = s.len() - p - r2.len();
            s.replace_range(p..p + consumed, &format!(" {a} over {b} "));
        }
    }
    for (a, b) in reps {
        s = s.replace(a, b);
    }
    // strip remaining commands like \mathrm, \text, \textcolor{..}
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                while chars.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                    chars.next();
                }
            }
            '{' | '}' | '$' => {}
            '_' => out.push(' '),
            '^' => out.push_str(" to the power "),
            _ => out.push(c),
        }
    }
    let mut r = out
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" ,", ",")
        .replace(" .", ".");
    while r.contains(",,") {
        r = r.replace(",,", ",");
    }
    r
}

fn take_group(s: &str) -> (String, &str) {
    let t = s.trim_start();
    if !t.starts_with('{') {
        let mut it = t.chars();
        return match it.next() {
            Some(c) => (c.to_string(), it.as_str()),
            None => (String::new(), t),
        };
    }
    let mut depth = 0;
    for (i, c) in t.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return (t[1..i].to_string(), &t[i + 1..]);
                }
            }
            _ => {}
        }
    }
    (t.to_string(), "")
}

/// Render a statement with inline `$math$` as a wrapped block (text + math mixed).
pub fn rich_statement(s: &str, width: f64, size: f64) -> Mobject {
    if !s.contains('$') {
        return paragraph(s, width, size, Align::Left);
    }
    // lay out tokens (words / math segments) with simple greedy wrapping
    let mut tokens: Vec<Mobject> = Vec::new();
    for (i, seg) in s.split('$').enumerate() {
        if seg.is_empty() {
            continue;
        }
        if i % 2 == 1 {
            tokens.push(MathTex::new(seg).size(size).inline().build());
        } else {
            for w in seg.split_whitespace() {
                tokens.push(text_sized(w, size, WHITE));
            }
        }
    }
    let gap = size * 0.3;
    let line_h = size * 1.55;
    let mut out = Mobject::empty();
    let (mut x, mut y) = (0.0, 0.0);
    for t in tokens {
        let w = t.width();
        if x > 0.0 && x + w > width {
            x = 0.0;
            y -= line_h;
        }
        out.push(t.move_to_aligned(v2(x, y), LEFT));
        x += w + gap;
    }
    let c = out.center();
    out.shift(-c)
}

const WORK_TOP: f64 = 2.3;
const WORK_BOTTOM: f64 = -2.75;

impl Scene {
    fn narrated(&mut self, enabled: bool, text: &str, fallback: f64, f: impl FnOnce(&mut Scene)) {
        if enabled && !text.trim().is_empty() {
            self.voiceover(Voiceover::new(text), |s, _| f(s));
        } else {
            let t0 = self.time();
            f(self);
            let used = self.time() - t0;
            if used < fallback {
                self.wait(fallback - used);
            }
        }
    }

    /// Build the whole problem-solving sequence. Returns the answer object.
    pub fn solve(&mut self, p: &Problem) -> Obj {
        let accent = p.accent.unwrap_or(YELLOW);
        let (fw, _) = frame_size();
        // 1. title + statement
        let header = bold_text(&p.title, 0.55, accent)
            .to_edge(UP, 0.35)
            .to_edge(LEFT, 0.6);
        let header_o = self.register(header.clone());
        let bar = self.register(
            line(v2(-fw / 2.0 + 0.6, 0.0), v2(fw / 2.0 - 0.6, 0.0))
                .stroke(accent, 2.0)
                .next_to(&header, DOWN, 0.15)
                .align_to(&header, LEFT),
        );
        self.play([write(header_o), create(bar)]);
        let has_diagram = p.diagram.is_some();
        let st_width = if has_diagram { fw * 0.5 } else { fw - 1.6 };
        let statement = rich_statement(&p.statement, st_width, 0.36)
            .to_edge(LEFT, 0.7)
            .shift(UP * 1.0);
        let st = self.register(statement.clone());
        let mut diag_o = None;
        let plain: String = tex_to_speech(&p.statement);
        self.narrated(p.narrate, &plain, 2.5, |s| {
            s.play(
                add_text_word_by_word(st)
                    .run_time((plain.split_whitespace().count() as f64 * 0.12).clamp(1.0, 4.0)),
            );
            if let Some(d) = &p.diagram {
                let d = d
                    .clone()
                    .fit_within(fw * 0.38, 3.6)
                    .move_to(v2(fw * 0.25, 0.4));
                let o = s.register(d);
                s.play(create(o).run_time(1.5));
                diag_o = Some(o);
            }
        });
        // 2. given / find panel (left column), statement moves out
        let mut panel = Mobject::empty();
        if !p.given.is_empty() {
            panel.push(bold_text("Given", 0.38, accent));
            for g in &p.given {
                panel.push(tex_sized(g, 0.42, WHITE));
            }
        }
        if !p.find.is_empty() {
            panel.push(bold_text("Find", 0.38, accent));
            for f in &p.find {
                panel.push(tex_sized(f, 0.42, WHITE));
            }
        }
        let panel = panel
            .arrange_aligned(DOWN, 0.22, LEFT)
            .to_edge(LEFT, 0.6)
            .shift(UP * (2.3 - 0.0));
        let panel = panel.clone().shift(UP * (WORK_TOP - panel.top().y));
        let panel_bg = rounded_rectangle(panel.width() + 0.5, panel.height() + 0.5, 0.15)
            .move_to(panel.center())
            .fill(rgb8(24, 30, 44), 0.95)
            .stroke(accent.with_alpha(0.6), 2.0);
        let panel_o = self.register(Mobject::group([panel_bg, panel.clone()]));
        let mut out_anims = vec![fade_out_shift(st, UP * 0.4)];
        if let Some(o) = diag_o {
            // keep the diagram, shrink it under the panel
            let target = p
                .diagram
                .clone()
                .unwrap()
                .fit_within(3.6, 2.4)
                .move_to(v2(-fw / 2.0 + 2.5, -2.4));
            out_anims.push(transform(o, target));
        }
        if !p.given.is_empty() || !p.find.is_empty() {
            self.play(out_anims);
            self.play(fade_in_shift(panel_o, RIGHT * 0.4));
        } else {
            self.play(out_anims);
        }
        // 3. steps in the work area (right of the panel)
        let work_left = panel.right().x.max(-fw / 2.0 + 3.6) + 0.7;
        let mut y = WORK_TOP;
        let mut work: Vec<(Obj, f64)> = Vec::new(); // (object, top y)
        let mut prev_eq: Option<Obj> = None;
        for step in &p.steps {
            let mut block = Mobject::empty();
            let expl = if step.explain.is_empty() {
                None
            } else {
                Some(text_sized(&step.explain, 0.3, GRAY_A))
            };
            let mut eq = None;
            if !step.tex.is_empty() {
                let mut m = MathTex::new(&step.tex).size(0.52);
                for h in &step.highlight {
                    m = m.t2c(h, accent);
                }
                eq = Some(m.build());
            }
            if let Some(e) = &expl {
                block.push(e.clone());
            }
            if let Some(e) = &eq {
                block.push(e.clone());
            }
            let block = block.arrange_aligned(DOWN, 0.15, LEFT);
            let h = block.height();
            y = self.scroll_work(&mut work, y, h);
            let placed = block.move_to_aligned(v2(work_left, y), UL);
            let narration = step.narration.clone().unwrap_or_else(|| {
                if step.tex.is_empty() {
                    step.explain.clone()
                } else {
                    format!("{}: {}.", step.explain, tex_to_speech(&step.tex))
                }
            });
            let mut expl_o = None;
            let mut eq_o = None;
            let n_children = placed.children.len();
            let mut idx = 0;
            if let Some(_e) = &expl {
                expl_o = Some(self.register(placed.children[idx].clone()));
                idx += 1;
            }
            if eq.is_some() && idx < n_children {
                eq_o = Some(self.register(placed.children[idx].clone()));
            }
            let morph_from = if step.transform_from_previous {
                prev_eq
            } else {
                None
            };
            self.narrated(p.narrate, &narration, self_pause(p), |s| {
                if let Some(o) = expl_o {
                    s.play(fade_in_shift(o, RIGHT * 0.2).run_time(0.5));
                }
                if let Some(o) = eq_o {
                    match morph_from {
                        Some(prev) => s.play(transform_matching_from_copy(prev, o).run_time(1.2)),
                        None => s.play(write(o)),
                    };
                    if !step.highlight.is_empty() {
                        s.play(indicate_with(o, accent, 1.06));
                    }
                }
            });
            for o in [expl_o, eq_o].into_iter().flatten() {
                work.push((o, y));
            }
            if eq_o.is_some() {
                prev_eq = eq_o;
            }
            y -= h + 0.35;
        }
        // 4. answer
        let ans = MathTex::new(&p.answer).size(0.62).color(accent).build();
        let boxed = Mobject::group([
            surrounding_rounded(&ans, 0.25, 0.12)
                .stroke(accent, 4.0)
                .glow(0.12, Some(accent)),
            ans,
        ]);
        let y = self.scroll_work(&mut work, y, boxed.height() + 0.1);
        let boxed = boxed.move_to_aligned(v2(work_left, y - 0.1), UL);
        let a = self.register(boxed);
        let answer_text = format!("So the answer is {}.", tex_to_speech(&p.answer));
        self.narrated(p.narrate, &answer_text, 1.5, |s| {
            s.add_sound(crate::sfx::success(), 0.5);
            s.play(draw_border_then_fill(a).run_time(1.2));
            s.play(flash(s.get(a).right(), accent));
        });
        a
    }

    /// Scroll the work column up so a block of height `h` fits below `y`; returns the new `y`.
    fn scroll_work(&mut self, work: &mut Vec<(Obj, f64)>, y: f64, h: f64) -> f64 {
        if y - h >= WORK_BOTTOM {
            return y;
        }
        let delta = (WORK_BOTTOM - (y - h)) + 0.1;
        let mut anims = Vec::new();
        let mut keep = Vec::new();
        for (o, top) in work.drain(..) {
            if top + delta > WORK_TOP + 0.2 {
                anims.push(fade_out_shift(o, UP * delta));
            } else {
                anims.push(o.animate().shift(UP * delta).build());
                keep.push((o, top + delta));
            }
        }
        *work = keep;
        if !anims.is_empty() {
            self.play(together(anims).run_time(0.6));
        }
        y + delta
    }

    /// Algebra derivation: each line morphs from the previous, matching equal parts.
    pub fn derivation(&mut self, lines: &[&str], at: Vec2, size: f64) -> Vec<Obj> {
        let mut objs = Vec::new();
        let mut y = at.y;
        let mut prev: Option<Obj> = None;
        for l in lines {
            let m = MathTex::new(l).size(size).build().move_to(v2(at.x, y));
            let o = self.register(m);
            match prev {
                Some(p) => {
                    self.play(transform_matching_from_copy(p, o).run_time(1.0));
                }
                None => {
                    self.play(write(o));
                }
            }
            prev = Some(o);
            objs.push(o);
            y -= size * 1.6;
            self.wait(0.4);
        }
        objs
    }

    /// Multiple-choice question: shows options, then reveals the correct one.
    pub fn quiz(
        &mut self,
        question: &str,
        options: &[&str],
        correct: usize,
        think_time: f64,
    ) -> Vec<Obj> {
        let (fw, _) = frame_size();
        let q = rich_statement(question, fw - 2.0, 0.42).to_edge(UP, 0.8);
        let qo = self.register(q);
        self.play(write(qo));
        let letters = ["A", "B", "C", "D", "E", "F"];
        let cards: Vec<Mobject> = options
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let label = bold_text(letters[i % letters.len()], 0.4, BLACK);
                let badge = Mobject::group([circle(0.28).fill(YELLOW, 1.0).no_stroke(), label]);
                let body = if o.contains('$') {
                    rich_statement(o, 4.0, 0.38)
                } else {
                    tex_sized(o, 0.5, WHITE)
                };
                let row = Mobject::group([badge, body]).arrange(RIGHT, 0.3);
                let bg = rounded_rectangle(5.6, row.height() + 0.5, 0.15)
                    .move_to(row.center())
                    .fill(rgb8(28, 34, 50), 1.0)
                    .stroke(GRAY_D, 2.0);
                let left_edge = bg.left().x;
                let row = row.clone().shift(RIGHT * (left_edge + 0.25 - row.left().x));
                Mobject::group([bg, row])
            })
            .collect();
        let grid = Mobject::group(cards)
            .arrange_in_grid(2, 0.4, 0.35)
            .shift(DOWN * 0.6);
        let objs: Vec<Obj> = grid
            .children
            .iter()
            .map(|c| self.register(c.clone()))
            .collect();
        self.play(lagged_start(
            objs.iter().map(|o| fade_in_shift(*o, UP * 0.3)).collect(),
            0.15,
        ));
        // countdown bar
        let bar = self.register(progress_bar_obj(fw - 3.0, 0.0).to_edge(DOWN, 0.4));
        self.play(fade_in(bar));
        let target = progress_bar_obj(fw - 3.0, 1.0).to_edge(DOWN, 0.4);
        self.play(
            transform(bar, target)
                .run_time(think_time)
                .rate(Rate::Linear),
        );
        self.add_sound(crate::sfx::ding(), 0.5);
        let mut reveal = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            if i == correct {
                reveal.push(o.animate().stroke(GREEN, 5.0).scale(1.05).build());
            } else {
                reveal.push(o.animate().opacity(0.35).build());
            }
        }
        reveal.push(fade_out(bar));
        self.play(reveal);
        let c = self.get(objs[correct]);
        let tick = self.register(checkmark(0.5).next_to(&c, RIGHT, -0.7));
        self.play(grow_from_center(tick));
        let mut all = vec![qo, tick];
        all.extend(objs);
        all
    }
}

fn self_pause(p: &Problem) -> f64 {
    p.step_pause
}

fn progress_bar_obj(width: f64, progress: f64) -> Mobject {
    let h = 0.14;
    let track = rounded_rectangle(width, h, h / 2.0)
        .fill(GRAY_D, 1.0)
        .no_stroke();
    let fill = rounded_rectangle((width * progress).max(h), h, h / 2.0)
        .move_to_aligned(v2(-width / 2.0, 0.0), LEFT)
        .fill(YELLOW, 1.0)
        .no_stroke();
    Mobject::group([track, fill])
}

// ------------------------------------------------------------ step generators

fn num(v: f64) -> String {
    crate::graph::fmt_num(v, 3)
}

/// Fully worked quadratic `a x² + b x + c = 0` via the quadratic formula.
pub fn quadratic_problem(a: f64, b: f64, c: f64) -> Problem {
    let eq = format!(
        "{}x^2 {} {}x {} {} = 0",
        num(a),
        if b < 0.0 { "-" } else { "+" },
        num(b.abs()),
        if c < 0.0 { "-" } else { "+" },
        num(c.abs())
    );
    let d = b * b - 4.0 * a * c;
    let mut steps = vec![
        Step::new(
            "Use the quadratic formula",
            r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        ),
        Step::new(
            "Compute the discriminant",
            &format!(
                r"\Delta = b^2 - 4ac = ({})^2 - 4({})({}) = {}",
                num(b),
                num(a),
                num(c),
                num(d)
            ),
        )
        .highlight(r"\Delta"),
    ];
    let answer = if d < 0.0 {
        let re = -b / (2.0 * a);
        let im = (-d).sqrt() / (2.0 * a);
        steps.push(Step::new(
            "Negative discriminant: complex roots",
            &format!(
                r"x = \frac{{{} \pm \sqrt{{{}}}}}{{{}}}",
                num(-b),
                num(d),
                num(2.0 * a)
            ),
        ));
        format!(r"x = {} \pm {}i", num(re), num(im.abs()))
    } else {
        let sq = d.sqrt();
        steps.push(
            Step::new(
                "Substitute the values",
                &format!(
                    r"x = \frac{{{} \pm \sqrt{{{}}}}}{{{}}}",
                    num(-b),
                    num(d),
                    num(2.0 * a)
                ),
            )
            .morph(),
        );
        steps.push(
            Step::new(
                "Simplify",
                &format!(
                    r"x = \frac{{{} \pm {}}}{{{}}}",
                    num(-b),
                    num(sq),
                    num(2.0 * a)
                ),
            )
            .morph(),
        );
        let (x1, x2) = ((-b + sq) / (2.0 * a), (-b - sq) / (2.0 * a));
        if d == 0.0 {
            format!(r"x = {}", num(x1))
        } else {
            format!(r"x_1 = {},\quad x_2 = {}", num(x1), num(x2))
        }
    };
    Problem::new("Solve the quadratic", &format!("Solve ${eq}$ for $x$."))
        .given(&[
            &format!("a = {}", num(a)),
            &format!("b = {}", num(b)),
            &format!("c = {}", num(c)),
        ])
        .find(&["x"])
        .steps(steps)
        .answer(&answer)
}

/// Uniform acceleration (SUVAT): given u, a, t → v and s, with the full working.
pub fn kinematics_problem(u: f64, a: f64, t: f64) -> Problem {
    let v = u + a * t;
    let s = u * t + 0.5 * a * t * t;
    Problem::new(
        "Uniformly accelerated motion",
        &format!(
            "A car moving at ${}\\,\\mathrm{{m/s}}$ accelerates uniformly at ${}\\,\\mathrm{{m/s^2}}$ for ${}\\,\\mathrm{{s}}$. Find its final velocity and the distance covered.",
            num(u),
            num(a),
            num(t)
        ),
    )
    .given(&[
        &format!(r"u = {}\ \mathrm{{m/s}}", num(u)),
        &format!(r"a = {}\ \mathrm{{m/s^2}}", num(a)),
        &format!(r"t = {}\ \mathrm{{s}}", num(t)),
    ])
    .find(&["v", "s"])
    .steps(vec![
        Step::new("First equation of motion", "v = u + at").highlight("v"),
        Step::new("Substitute", &format!("v = {} + ({})({})", num(u), num(a), num(t))).morph(),
        Step::new("Final velocity", &format!(r"v = {}\ \mathrm{{m/s}}", num(v))).morph(),
        Step::new("Second equation of motion", r"s = ut + \tfrac{1}{2}at^2").highlight("s"),
        Step::new("Substitute", &format!(r"s = ({})({}) + \tfrac{{1}}{{2}}({})({})^2", num(u), num(t), num(a), num(t))).morph(),
    ])
    .answer(&format!(r"v = {}\ \mathrm{{m/s}},\quad s = {}\ \mathrm{{m}}", num(v), num(s)))
    .diagram(kinematics_diagram(u, v))
}

fn kinematics_diagram(u: f64, v: f64) -> Mobject {
    let road = line(v2(-3.0, -0.6), v2(3.0, -0.6)).stroke(GRAY_B, 3.0);
    let car = |x: f64| {
        Mobject::group([
            rounded_rectangle(1.2, 0.45, 0.12)
                .move_to(v2(x, -0.12))
                .fill(BLUE, 1.0)
                .no_stroke(),
            circle_at(v2(x - 0.35, -0.4), 0.15)
                .fill(GRAY_E, 1.0)
                .stroke(GRAY_B, 2.0),
            circle_at(v2(x + 0.35, -0.4), 0.15)
                .fill(GRAY_E, 1.0)
                .stroke(GRAY_B, 2.0),
        ])
    };
    Mobject::group([
        road,
        car(-2.2),
        car(2.0).fade(0.4),
        arrow(v2(-2.2, 0.5), v2(-2.2 + 0.25 * u.clamp(0.5, 6.0), 0.5)).color(GREEN),
        tex_sized("u", 0.4, GREEN).move_to(v2(-2.2, 0.85)),
        arrow(v2(2.0, 0.5), v2(2.0 + 0.25 * v.clamp(0.5, 6.0), 0.5)).color(ORANGE),
        tex_sized("v", 0.4, ORANGE).move_to(v2(2.0, 0.85)),
    ])
}

/// Ohm's law / power problem for a resistor across a battery.
pub fn ohms_law_problem(voltage: f64, resistance: f64) -> Problem {
    let i = voltage / resistance;
    let p = voltage * i;
    let (a, b, c, d) = (v2(-2.0, -1.2), v2(2.0, -1.2), v2(2.0, 1.2), v2(-2.0, 1.2));
    let diagram = Mobject::group([
        crate::physics::battery(a, d),
        crate::physics::wire(&[d, v2(-0.9, 1.2)]),
        crate::physics::resistor(v2(-0.9, 1.2), v2(0.9, 1.2)),
        crate::physics::wire(&[v2(0.9, 1.2), c, b, a]),
        tex_sized(&format!(r"{}\,\Omega", num(resistance)), 0.4, YELLOW).move_to(v2(0.0, 1.75)),
        tex_sized(&format!(r"{}\,\mathrm{{V}}", num(voltage)), 0.4, YELLOW).move_to(v2(-2.7, 0.0)),
    ]);
    Problem::new(
        "Ohm's law",
        &format!("A ${}\\,\\Omega$ resistor is connected to a ${}\\,\\mathrm{{V}}$ battery. Find the current and the power dissipated.", num(resistance), num(voltage)),
    )
    .given(&[&format!(r"V = {}\ \mathrm{{V}}", num(voltage)), &format!(r"R = {}\ \Omega", num(resistance))])
    .find(&["I", "P"])
    .diagram(diagram)
    .steps(vec![
        Step::new("Ohm's law", "I = \\frac{V}{R}").highlight("I"),
        Step::new("Substitute", &format!(r"I = \frac{{{}}}{{{}}} = {}\ \mathrm{{A}}", num(voltage), num(resistance), num(i))).morph(),
        Step::new("Electrical power", "P = VI").highlight("P"),
        Step::new("Substitute", &format!(r"P = ({})({}) = {}\ \mathrm{{W}}", num(voltage), num(i), num(p))).morph(),
    ])
    .answer(&format!(r"I = {}\ \mathrm{{A}},\quad P = {}\ \mathrm{{W}}", num(i), num(p)))
}

/// Stoichiometry: mass of product from a given mass of reactant in a balanced reaction.
pub fn stoichiometry_problem(
    reactants: &[&str],
    products: &[&str],
    given: &str,
    given_mass: f64,
    wanted: &str,
) -> Option<Problem> {
    let coefs = crate::chemistry::balance(reactants, products)?;
    let all: Vec<&str> = reactants.iter().chain(products).copied().collect();
    let gi = all.iter().position(|s| *s == given)?;
    let wi = all.iter().position(|s| *s == wanted)?;
    let (mg, mw) = (
        crate::chemistry::molar_mass(given),
        crate::chemistry::molar_mass(wanted),
    );
    let n_given = given_mass / mg;
    let n_wanted = n_given * coefs[wi] as f64 / coefs[gi] as f64;
    let mass = n_wanted * mw;
    let eq = crate::chemistry::balanced_equation(reactants, products)?;
    let tex_eq = crate::tex::chem_to_tex(&eq);
    let ce = |s: &str| crate::tex::chem_to_tex(s);
    Some(
        Problem::new(
            "Stoichiometry",
            &format!(
                "How many grams of ${}$ form from ${}\\,\\mathrm{{g}}$ of ${}$?",
                ce(wanted),
                num(given_mass),
                ce(given)
            ),
        )
        .given(&[&format!(
            r"m({}) = {}\ \mathrm{{g}}",
            ce(given),
            num(given_mass)
        )])
        .find(&[&format!("m({})", ce(wanted))])
        .steps(vec![
            Step::new("Balance the equation", &tex_eq),
            Step::new(
                "Moles of reactant",
                &format!(
                    r"n = \frac{{m}}{{M}} = \frac{{{}}}{{{}}} = {}\ \mathrm{{mol}}",
                    num(given_mass),
                    num(mg),
                    num(n_given)
                ),
            ),
            Step::new(
                "Use the mole ratio",
                &format!(
                    r"n({}) = {} \times \frac{{{}}}{{{}}} = {}\ \mathrm{{mol}}",
                    ce(wanted),
                    num(n_given),
                    coefs[wi],
                    coefs[gi],
                    num(n_wanted)
                ),
            ),
            Step::new(
                "Convert to mass",
                &format!(
                    r"m = nM = {} \times {} = {}\ \mathrm{{g}}",
                    num(n_wanted),
                    num(mw),
                    num(mass)
                ),
            ),
        ])
        .answer(&format!(
            r"m({}) \approx {}\ \mathrm{{g}}",
            ce(wanted),
            num(mass)
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_render_into_scenes() {
        for p in [
            quadratic_problem(1.0, -3.0, 2.0),
            quadratic_problem(1.0, 2.0, 5.0),
            kinematics_problem(5.0, 2.0, 4.0).silent(),
            ohms_law_problem(12.0, 4.0),
            stoichiometry_problem(&["H2", "O2"], &["H2O"], "H2", 4.0, "H2O").unwrap(),
        ] {
            let mut s = Scene::new();
            let a = s.solve(&p);
            assert!(s.duration() > 5.0, "{}", p.title);
            assert!(s.is_present(a));
            if p.narrate {
                assert!(!s.captions.is_empty());
            }
        }
        let q = quadratic_problem(1.0, -3.0, 2.0);
        assert_eq!(q.answer, "x_1 = 2,\\quad x_2 = 1");
        let st = stoichiometry_problem(&["H2", "O2"], &["H2O"], "H2", 4.0, "H2O").unwrap();
        assert!(st.answer.contains("35.7"), "{}", st.answer);
        let mut s = Scene::new();
        s.derivation(&["2x + 3 = 7", "2x = 4", "x = 2"], v2(0.0, 2.0), 0.6);
        let objs = s.quiz("What is $2 + 2$?", &["3", "4", "5", "22"], 1, 2.0);
        assert!(objs.len() >= 6);
        assert_eq!(
            tex_to_speech(r"v = 13\ \mathrm{m/s},\quad s = 36\ \mathrm{m}"),
            "v = 13 meters per second, s = 36 m"
        );
        assert_eq!(tex_to_speech(r"x = \frac{-b}{2a}"), "x = -b over 2a");
        assert!(
            rich_statement(
                "A ball of mass $m$ falls a height $h$ near the surface of the Earth",
                5.0,
                0.4
            )
            .leaf_count()
                > 10
        );
    }
}
