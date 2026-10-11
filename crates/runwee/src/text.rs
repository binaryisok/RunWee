//! Text objects: shaped, kerned text converted to bezier glyphs (so it can be
//! written, morphed and coloured like any other shape).

use crate::color::*;
use crate::fonts::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::shapes;
use crate::style::*;

/// Default em size in scene units.
pub const DEFAULT_FONT_SIZE: f64 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

/// A styled run inside a text.
#[derive(Clone, Debug)]
pub struct Span {
    pub text: String,
    pub color: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub font: Option<String>,
    pub scale: f64,
}

impl Span {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            color: None,
            bold: None,
            italic: None,
            font: None,
            scale: 1.0,
        }
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
        self
    }
    pub fn bold(mut self) -> Self {
        self.bold = Some(true);
        self
    }
    pub fn italic(mut self) -> Self {
        self.italic = Some(true);
        self
    }
    pub fn font(mut self, f: &str) -> Self {
        self.font = Some(f.to_string());
        self
    }
}

/// Text builder.
#[derive(Clone, Debug)]
pub struct Text {
    pub spans: Vec<Span>,
    pub font: String,
    pub size: f64,
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
    pub align: Align,
    pub line_spacing: f64,
    pub max_width: Option<f64>,
    pub letter_spacing: f64,
    /// substring → colour (manim's `t2c`)
    pub t2c: Vec<(String, Color)>,
}

impl Text {
    pub fn new(s: &str) -> Self {
        Self {
            spans: vec![Span::new(s)],
            font: "Inter".into(),
            size: DEFAULT_FONT_SIZE,
            color: WHITE,
            bold: false,
            italic: false,
            align: Align::Left,
            line_spacing: 1.3,
            max_width: None,
            letter_spacing: 0.0,
            t2c: vec![],
        }
    }
    /// Text from styled spans.
    pub fn spans(spans: Vec<Span>) -> Self {
        let mut t = Self::new("");
        t.spans = spans;
        t
    }
    /// Lightweight markup: `**bold**`, `*italic*`, `[color=RED]…[/color]` (any colour name or #hex).
    pub fn markup(s: &str) -> Self {
        Self::spans(parse_markup(s))
    }
    pub fn font(mut self, f: &str) -> Self {
        self.font = f.into();
        self
    }
    pub fn size(mut self, s: f64) -> Self {
        self.size = s;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }
    pub fn line_spacing(mut self, l: f64) -> Self {
        self.line_spacing = l;
        self
    }
    pub fn max_width(mut self, w: f64) -> Self {
        self.max_width = Some(w);
        self
    }
    pub fn letter_spacing(mut self, l: f64) -> Self {
        self.letter_spacing = l;
        self
    }
    pub fn t2c(mut self, sub: &str, c: Color) -> Self {
        self.t2c.push((sub.to_string(), c));
        self
    }

    /// Lay out and convert to a Mobject. Children are lines; each line's children are glyphs.
    pub fn build(&self) -> Mobject {
        // Flatten spans into styled characters, then into lines.
        struct Ch {
            c: char,
            color: Color,
            font: Option<FontId>,
            scale: f64,
            mark: bool,
        }
        let full: String = self.spans.iter().map(|s| s.text.as_str()).collect();
        let mut chars: Vec<Ch> = Vec::new();
        for sp in &self.spans {
            let fam = sp.font.clone().unwrap_or_else(|| self.font.clone());
            let id = find_font(
                &fam,
                sp.bold.unwrap_or(self.bold),
                sp.italic.unwrap_or(self.italic),
            );
            for c in sp.text.chars() {
                chars.push(Ch {
                    c,
                    color: sp.color.unwrap_or(self.color),
                    // Per-character fallback so symbols the font lacks still draw.
                    font: font_for_char(id, c),
                    scale: sp.scale,
                    mark: is_combining(c),
                });
            }
        }
        // t2c colouring on the concatenated string
        for (sub, col) in &self.t2c {
            if sub.is_empty() {
                continue;
            }
            let fc: Vec<char> = full.chars().collect();
            let sc: Vec<char> = sub.chars().collect();
            let mut i = 0;
            while i + sc.len() <= fc.len() {
                if fc[i..i + sc.len()] == sc[..] {
                    for ch in &mut chars[i..i + sc.len()] {
                        ch.color = *col;
                    }
                    i += sc.len();
                } else {
                    i += 1;
                }
            }
        }
        // Characters no bundled font can draw are skipped.
        chars.retain(|c| c.font.is_some());
        // Split into runs of equal (font, scale, color) per line, shape them.
        struct G {
            path: Option<std::sync::Arc<Path>>,
            x: f64,
            y: f64,
            adv: f64,
            color: Color,
            scale: f64,
            ch: char,
            mark: bool,
        }
        let mut lines: Vec<Vec<G>> = vec![vec![]];
        let mut i = 0;
        while i < chars.len() {
            if chars[i].c == '\n' {
                lines.push(vec![]);
                i += 1;
                continue;
            }
            let mut j = i;
            while j < chars.len()
                && chars[j].c != '\n'
                && chars[j].font == chars[i].font
                && chars[j].scale == chars[i].scale
                && chars[j].color == chars[i].color
                && chars[j].mark == chars[i].mark
            {
                j += 1;
            }
            let run: String = chars[i..j].iter().map(|c| c.c).collect();
            let run_font = chars[i].font.unwrap_or(FontId(0));
            let (glyphs, _) = shape(run_font, &run);
            // map byte cluster → char index
            let byte_to_char: Vec<usize> = {
                let mut v = vec![0; run.len() + 1];
                for (ci, (bi, _)) in run.char_indices().enumerate() {
                    v[bi] = ci;
                }
                v
            };
            let mut run_pen = 0.0;
            for g in glyphs {
                let xoff = g.x - run_pen;
                run_pen += g.advance;
                let ci = i + byte_to_char[g.cluster.min(run.len())];
                let ch = chars[ci].c;
                let p = glyph_path(run_font, g.gid);
                lines.last_mut().unwrap().push(G {
                    path: if p.is_empty() { None } else { Some(p) },
                    x: xoff,
                    y: g.y,
                    adv: g.advance,
                    color: chars[ci].color,
                    scale: chars[i].scale,
                    ch,
                    mark: chars[i].mark,
                });
            }
            i = j;
        }
        // Combining marks shaped on their own (from a fallback font): centre each
        // over the previous glyph, just above it, taking no advance.
        for line in &mut lines {
            for k in 0..line.len() {
                if !line[k].mark {
                    continue;
                }
                let Some(mb) = line[k].path.as_ref().and_then(|p| p.bounds()) else {
                    continue;
                };
                let prev = (0..k).rev().find(|&q| !line[q].mark);
                let Some(pb) = prev.and_then(|q| {
                    line[q]
                        .path
                        .as_ref()
                        .and_then(|p| p.bounds())
                        .map(|b| (q, b))
                }) else {
                    line[k].adv = 0.0;
                    continue;
                };
                let (q, b) = pb;
                // Pen positions between q and k (other marks have zero advance).
                let back: f64 = line[q..k].iter().map(|g| g.adv).sum();
                let cx = line[q].x + (b.min.x + b.max.x) / 2.0 - back;
                line[k].x = cx - (mb.min.x + mb.max.x) / 2.0;
                line[k].y = line[q].y + b.max.y + 0.06 - mb.min.y;
                line[k].adv = 0.0;
            }
        }
        // Word wrap
        if let Some(maxw) = self.max_width {
            let limit = maxw / self.size;
            let mut wrapped: Vec<Vec<G>> = Vec::new();
            for line in lines {
                let mut cur: Vec<G> = Vec::new();
                let mut width = 0.0;
                let mut last_space: Option<usize> = None;
                for g in line {
                    let w = g.adv * g.scale + self.letter_spacing;
                    if g.ch == ' ' {
                        last_space = Some(cur.len());
                    }
                    if width + w > limit && g.ch != ' ' && !cur.is_empty() {
                        if let Some(sp) = last_space {
                            let rest: Vec<G> = cur.drain(sp..).skip(1).collect();
                            wrapped.push(std::mem::take(&mut cur));
                            cur = rest;
                            width = cur
                                .iter()
                                .map(|g| g.adv * g.scale + self.letter_spacing)
                                .sum();
                            last_space = None;
                        } else {
                            wrapped.push(std::mem::take(&mut cur));
                            width = 0.0;
                        }
                    }
                    width += w;
                    cur.push(g);
                }
                wrapped.push(cur);
            }
            lines = wrapped;
        }
        // Build mobjects
        let (asc, desc) = line_metrics(find_font(&self.font, self.bold, self.italic));
        let line_h = (asc + desc) * self.line_spacing;
        let mut out = Mobject::empty();
        let mut widths = Vec::new();
        for (li, line) in lines.iter().enumerate() {
            let mut lm = Mobject::empty();
            let mut x = 0.0;
            let y = -(li as f64) * line_h;
            for g in line {
                if let Some(p) = &g.path {
                    let mut path = (**p).clone();
                    let s = self.size * g.scale;
                    path.transform(
                        &Affine::translate(v2(
                            (x + g.x * g.scale) * self.size,
                            (y + g.y) * self.size,
                        ))
                        .then_after(Affine::scale(s, s)),
                    );
                    let mut m = Mobject::from_path_styled(path, Style::filled(g.color));
                    m.tag = Some(g.ch.to_string().into());
                    lm.push(m);
                }
                x += g.adv * g.scale + self.letter_spacing;
            }
            widths.push(x * self.size);
            out.push(lm);
        }
        // alignment
        let maxw = widths.iter().cloned().fold(0.0, f64::max);
        for (lm, w) in out.children.iter_mut().zip(&widths) {
            let dx = match self.align {
                Align::Left => 0.0,
                Align::Center => (maxw - w) / 2.0,
                Align::Right => maxw - w,
            };
            if dx != 0.0 {
                lm.shift_mut(v2(dx, 0.0));
            }
        }
        // centre the block at the origin (vertically by the first-line baseline box)
        let n = out.children.len() as f64;
        let block_h = (n - 1.0) * line_h * self.size;
        out.shift(v2(
            -maxw / 2.0,
            block_h / 2.0 - (asc - desc) * self.size / 2.0,
        ))
    }
}

/// Single-line glyph group (children are glyphs, not lines) — what most callers want.
pub fn text(s: &str) -> Mobject {
    flatten_lines(Text::new(s).build())
}

pub fn text_sized(s: &str, size: f64, color: Color) -> Mobject {
    flatten_lines(Text::new(s).size(size).color(color).build())
}

pub fn bold_text(s: &str, size: f64, color: Color) -> Mobject {
    flatten_lines(Text::new(s).size(size).color(color).bold().build())
}

/// Serif (Computer Modern) text.
pub fn serif_text(s: &str, size: f64, color: Color) -> Mobject {
    flatten_lines(Text::new(s).font("serif").size(size).color(color).build())
}

/// Merge line groups into one glyph list if there is a single line.
fn flatten_lines(m: Mobject) -> Mobject {
    if m.children.len() == 1 {
        m.children.into_iter().next().unwrap()
    } else {
        m
    }
}

/// Multi-line paragraph with word wrapping.
pub fn paragraph(s: &str, width: f64, size: f64, align: Align) -> Mobject {
    Text::new(s)
        .size(size)
        .max_width(width)
        .align(align)
        .build()
}

/// Title with an underline bar, placed at the top of the frame.
pub fn title(s: &str) -> Mobject {
    let t = bold_text(s, 0.8, WHITE);
    let bar = shapes::line(t.left() + v2(0.0, -0.25), t.right() + v2(0.0, -0.25))
        .stroke(BLUE, 3.0)
        .shift(DOWN * (t.height() / 2.0));
    Mobject::group([t, bar]).to_edge(UP, 0.4)
}

/// Bulleted list (children = items).
pub fn bullet_list(items: &[&str], size: f64, color: Color) -> Mobject {
    let rows: Vec<Mobject> = items
        .iter()
        .map(|it| {
            let t = text_sized(it, size, color);
            let dot = shapes::dot_r(Vec2::ZERO, size * 0.12, color).next_to(&t, LEFT, size * 0.5);
            Mobject::group([dot, t])
        })
        .collect();
    Mobject::group(rows).arrange_aligned(DOWN, size * 0.55, LEFT)
}

fn parse_markup(s: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut colors: Vec<Color> = Vec::new();
    let mut cur = String::new();
    let flush =
        |cur: &mut String, spans: &mut Vec<Span>, bold: bool, italic: bool, colors: &Vec<Color>| {
            if !cur.is_empty() {
                let mut sp = Span::new(cur);
                if bold {
                    sp.bold = Some(true);
                }
                if italic {
                    sp.italic = Some(true);
                }
                sp.color = colors.last().copied();
                spans.push(sp);
                cur.clear();
            }
        };
    let b = s.as_bytes();
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest.starts_with("**") {
            flush(&mut cur, &mut spans, bold, italic, &colors);
            bold = !bold;
            i += 2;
        } else if rest.starts_with('*') {
            flush(&mut cur, &mut spans, bold, italic, &colors);
            italic = !italic;
            i += 1;
        } else if rest.starts_with("[color=") {
            if let Some(end) = rest.find(']') {
                flush(&mut cur, &mut spans, bold, italic, &colors);
                let name = &rest[7..end];
                colors.push(Color::parse(name).unwrap_or(WHITE));
                i += end + 1;
            } else {
                cur.push('[');
                i += 1;
            }
        } else if rest.starts_with("[/color]") {
            flush(&mut cur, &mut spans, bold, italic, &colors);
            colors.pop();
            i += 8;
        } else {
            let ch = rest.chars().next().unwrap();
            cur.push(ch);
            i += ch.len_utf8();
        }
        let _ = b;
    }
    flush(&mut cur, &mut spans, bold, italic, &colors);
    spans
}

/// Monospace code block with simple syntax highlighting and a rounded background.
pub fn code_block(src: &str, size: f64) -> Mobject {
    const KW: &[&str] = &[
        "fn", "let", "mut", "pub", "use", "struct", "impl", "for", "in", "while", "if", "else",
        "return", "def", "class", "import", "from", "as", "const", "match", "loop", "true",
        "false", "None", "True", "False", "self", "async", "await", "var", "function", "int",
        "float", "double", "void", "char", "print",
    ];
    let mut spans = Vec::new();
    for (li, line) in src.lines().enumerate() {
        if li > 0 {
            spans.push(Span::new("\n"));
        }
        // comments
        let (code, comment) = match line.find("//").or_else(|| line.find('#')) {
            Some(p) => (&line[..p], &line[p..]),
            None => (line, ""),
        };
        let mut word = String::new();
        let mut in_str: Option<char> = None;
        let push_word = |w: &mut String, spans: &mut Vec<Span>| {
            if w.is_empty() {
                return;
            }
            let c = if KW.contains(&w.as_str()) {
                Some(PURPLE_A)
            } else if w.chars().all(|c| c.is_ascii_digit() || c == '.') {
                Some(ORANGE)
            } else if w.chars().next().is_some_and(|c| c.is_uppercase()) {
                Some(TEAL_B)
            } else {
                None
            };
            let mut s = Span::new(w);
            s.color = c;
            spans.push(s);
            w.clear();
        };
        for c in code.chars() {
            if let Some(q) = in_str {
                word.push(c);
                if c == q {
                    spans.push(Span::new(&word).color(GREEN_B));
                    word.clear();
                    in_str = None;
                }
                continue;
            }
            if c == '"' || c == '\'' {
                push_word(&mut word, &mut spans);
                in_str = Some(c);
                word.push(c);
            } else if c.is_alphanumeric()
                || c == '_'
                || (c == '.' && word.chars().all(|d| d.is_ascii_digit()) && !word.is_empty())
            {
                word.push(c);
            } else {
                push_word(&mut word, &mut spans);
                let is_call = c == '(';
                let mut s = Span::new(&c.to_string());
                if is_call {
                    // colour the previous word as a function call
                    if let Some(last) = spans.last_mut() {
                        if last.color.is_none() {
                            last.color = Some(BLUE_B);
                        }
                    }
                }
                s.color = if "+-*/=<>!&|%^".contains(c) {
                    Some(RED_B)
                } else {
                    None
                };
                spans.push(s);
            }
        }
        if in_str.is_some() {
            spans.push(Span::new(&word).color(GREEN_B));
        } else {
            push_word(&mut word, &mut spans);
        }
        if !comment.is_empty() {
            spans.push(Span::new(comment).color(GRAY_B).italic());
        }
    }
    let t = Text::spans(spans)
        .font("mono")
        .size(size)
        .color(GRAY_A)
        .line_spacing(1.35)
        .build();
    let bg = shapes::rounded_rectangle(t.width() + size * 1.6, t.height() + size * 1.4, size * 0.4)
        .move_to(t.center())
        .fill(rgb8(0x1b, 0x1f, 0x29), 1.0)
        .stroke(rgb8(0x2d, 0x33, 0x40), 2.0);
    Mobject::group([bg, t])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_layout() {
        let t = text("Hello");
        assert_eq!(t.children.len(), 5);
        assert!(t.center().length() < 0.2);
        assert!(t.width() > 1.0);
        let two = Text::new("a\nbb").build();
        assert_eq!(two.children.len(), 2);
        let wrapped = Text::new("the quick brown fox jumps over the lazy dog")
            .max_width(3.0)
            .build();
        assert!(wrapped.children.len() >= 3);
        assert!(wrapped.width() <= 3.2);
        let m = Text::markup("a **b** [color=RED]c[/color]").build();
        assert_eq!(m.leaf_count(), 3);
        let colored = Text::new("E = mc2").t2c("mc", RED).build();
        assert_eq!(colored.leaf_count(), 5);
        assert!(code_block("fn main() {\n  let x = 1; // hi\n}", 0.3).leaf_count() > 10);
    }
}
