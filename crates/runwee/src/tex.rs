//! A self-contained TeX math typesetter (no LaTeX installation needed).
//!
//! Supports the subset used in science videos: fractions, roots, sub/superscripts,
//! big operators with limits, `\left…\right` with extensible delimiters, accents,
//! matrices / `aligned` / `cases` environments, braces, colours, spacing, text,
//! font switches (`\mathbb`, `\mathcal`, `\mathbf`…), arrows with labels,
//! `{{…}}` parts (manim-style) and `\ce{…}` chemistry.

use crate::color::*;
use crate::fonts::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::style::*;
use std::sync::Arc;

// ------------------------------------------------------------------ metrics

const X_HEIGHT: f64 = 0.431;
const AXIS: f64 = 0.25;
const RULE: f64 = 0.04;
const NUM1: f64 = 0.677;
const NUM2: f64 = 0.394;
const DENOM1: f64 = 0.686;
const DENOM2: f64 = 0.345;
const SUP1: f64 = 0.413;
const SUP2: f64 = 0.363;
const SUP3: f64 = 0.289;
const SUB1: f64 = 0.15;
const SUB2: f64 = 0.247;
const SUP_DROP: f64 = 0.386;
const SUB_DROP: f64 = 0.05;
const BIG_OP_SPACING1: f64 = 0.111;
const BIG_OP_SPACING2: f64 = 0.166;
const BIG_OP_SPACING3: f64 = 0.2;
const BIG_OP_SPACING4: f64 = 0.6;
const BIG_OP_SPACING5: f64 = 0.1;
const NULL_DELIM: f64 = 0.12;
const SCRIPT_SPACE: f64 = 0.05;
const MU: f64 = 1.0 / 18.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MStyle {
    Display,
    Text,
    Script,
    ScriptScript,
}

impl MStyle {
    fn scale(self) -> f64 {
        match self {
            MStyle::Display | MStyle::Text => 1.0,
            MStyle::Script => 0.7,
            MStyle::ScriptScript => 0.5,
        }
    }
    fn sup(self) -> MStyle {
        match self {
            MStyle::Display | MStyle::Text => MStyle::Script,
            _ => MStyle::ScriptScript,
        }
    }
    fn frac_num(self) -> MStyle {
        match self {
            MStyle::Display => MStyle::Text,
            MStyle::Text => MStyle::Script,
            _ => MStyle::ScriptScript,
        }
    }
    fn is_script(self) -> bool {
        matches!(self, MStyle::Script | MStyle::ScriptScript)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MFont {
    Main,
    MainBold,
    MainItalic,
    MathItalic,
    MathBold,
    Ams,
    Cal,
    Frak,
    Sans,
    Mono,
    Size(u8),
    Inter,
}

impl MFont {
    fn id(self) -> FontId {
        match self {
            MFont::Main => find_font("KaTeX_Main", false, false),
            MFont::MainBold => find_font("KaTeX_Main", true, false),
            MFont::MainItalic => find_font("KaTeX_Main", false, true),
            MFont::MathItalic => find_font("KaTeX_Math", false, true),
            MFont::MathBold => find_font("KaTeX_Math", true, true),
            MFont::Ams => find_font("KaTeX_AMS", false, false),
            MFont::Cal => find_font("KaTeX_Caligraphic", false, false),
            MFont::Frak => find_font("KaTeX_Fraktur", false, false),
            MFont::Sans => find_font("KaTeX_SansSerif", false, false),
            MFont::Mono => find_font("KaTeX_Typewriter", false, false),
            MFont::Size(1) => find_font("KaTeX_Size1", false, false),
            MFont::Size(2) => find_font("KaTeX_Size2", false, false),
            MFont::Size(3) => find_font("KaTeX_Size3", false, false),
            MFont::Size(_) => find_font("KaTeX_Size4", false, false),
            MFont::Inter => find_font("Inter", false, false),
        }
    }
}

/// Resolve a glyph, falling back through the math fonts.
fn resolve(ch: char, font: MFont) -> Option<(FontId, u16)> {
    let order = [
        font,
        MFont::Main,
        MFont::MathItalic,
        MFont::Ams,
        MFont::Size(1),
        MFont::MainBold,
        MFont::Inter,
    ];
    for f in order {
        let id = f.id();
        if let Some(g) = glyph_index(id, ch) {
            return Some((id, g));
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccentKind {
    Hat,
    WideHat,
    Tilde,
    Bar,
    Overline,
    Underline,
    Vec,
    OverRightArrow,
    OverLeftArrow,
    OverLeftRightArrow,
    Dot,
    DDot,
    Acute,
    Grave,
    Breve,
    Check,
    Ring,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowKind {
    Right,
    Left,
    LeftRight,
    RightLeftHarpoons,
    DoubleRight,
}

#[derive(Clone, Debug)]
pub enum Node {
    Sym {
        ch: char,
        font: MFont,
        class: Class,
    },
    Text {
        s: String,
        font: MFont,
    },
    Group(Vec<Node>),
    Scripts {
        base: Box<Node>,
        sup: Option<Box<Node>>,
        sub: Option<Box<Node>>,
    },
    Frac {
        num: Box<Node>,
        den: Box<Node>,
        style: Option<MStyle>,
        rule: bool,
        delims: Option<(char, char)>,
    },
    Sqrt {
        index: Option<Box<Node>>,
        body: Box<Node>,
    },
    Delimited {
        left: char,
        right: char,
        body: Box<Node>,
    },
    BigDelim {
        ch: char,
        size: u8,
        class: Class,
    },
    BigOp {
        ch: char,
        limits: Option<bool>,
    },
    OpName {
        name: String,
        limits: bool,
    },
    Accent {
        kind: AccentKind,
        body: Box<Node>,
    },
    Space(f64),
    Color(Color, Vec<Node>),
    Array {
        rows: Vec<Vec<Node>>,
        align: Vec<char>,
        col_gap: f64,
        left: char,
        right: char,
        display: bool,
        pair_align: bool,
    },
    Boxed(Box<Node>),
    Cancel(Box<Node>),
    Brace {
        body: Box<Node>,
        label: Option<Box<Node>>,
        over: bool,
    },
    Phantom(Box<Node>),
    StyleChange(MStyle),
    Part(usize, Vec<Node>),
    Overset {
        base: Box<Node>,
        over: Option<Box<Node>>,
        under: Option<Box<Node>>,
        class: Class,
    },
    XArrow {
        kind: ArrowKind,
        over: Option<Box<Node>>,
        under: Option<Box<Node>>,
    },
}

// ------------------------------------------------------------------- parser

struct Parser {
    s: Vec<char>,
    i: usize,
    parts: Vec<String>,
}

#[derive(Debug, PartialEq)]
enum Stop {
    Eof,
    Brace,
    Right(char),
    Middle,
    End(String),
    Amp,
    Newline,
}

fn greek(name: &str) -> Option<(char, MFont)> {
    let lower = [
        ("alpha", 'α'),
        ("beta", 'β'),
        ("gamma", 'γ'),
        ("delta", 'δ'),
        ("epsilon", 'ϵ'),
        ("varepsilon", 'ε'),
        ("zeta", 'ζ'),
        ("eta", 'η'),
        ("theta", 'θ'),
        ("vartheta", 'ϑ'),
        ("iota", 'ι'),
        ("kappa", 'κ'),
        ("lambda", 'λ'),
        ("mu", 'μ'),
        ("nu", 'ν'),
        ("xi", 'ξ'),
        ("omicron", 'ο'),
        ("pi", 'π'),
        ("varpi", 'ϖ'),
        ("rho", 'ρ'),
        ("varrho", 'ϱ'),
        ("sigma", 'σ'),
        ("varsigma", 'ς'),
        ("tau", 'τ'),
        ("upsilon", 'υ'),
        ("phi", 'ϕ'),
        ("varphi", 'φ'),
        ("chi", 'χ'),
        ("psi", 'ψ'),
        ("omega", 'ω'),
    ];
    let upper = [
        ("Gamma", 'Γ'),
        ("Delta", 'Δ'),
        ("Theta", 'Θ'),
        ("Lambda", 'Λ'),
        ("Xi", 'Ξ'),
        ("Pi", 'Π'),
        ("Sigma", 'Σ'),
        ("Upsilon", 'Υ'),
        ("Phi", 'Φ'),
        ("Psi", 'Ψ'),
        ("Omega", 'Ω'),
    ];
    if let Some((_, c)) = lower.iter().find(|(n, _)| *n == name) {
        return Some((*c, MFont::MathItalic));
    }
    upper
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| (*c, MFont::Main))
}

fn symbol(name: &str) -> Option<(char, MFont, Class)> {
    use Class::*;
    use MFont::*;
    let t: &[(&str, char, MFont, Class)] = &[
        ("pm", '±', Main, Bin),
        ("mp", '∓', Main, Bin),
        ("times", '×', Main, Bin),
        ("div", '÷', Main, Bin),
        ("cdot", '⋅', Main, Bin),
        ("ast", '∗', Main, Bin),
        ("star", '⋆', Main, Bin),
        ("circ", '∘', Main, Bin),
        ("bullet", '∙', Main, Bin),
        ("oplus", '⊕', Main, Bin),
        ("ominus", '⊖', Main, Bin),
        ("otimes", '⊗', Main, Bin),
        ("odot", '⊙', Main, Bin),
        ("cup", '∪', Main, Bin),
        ("cap", '∩', Main, Bin),
        ("wedge", '∧', Main, Bin),
        ("land", '∧', Main, Bin),
        ("vee", '∨', Main, Bin),
        ("lor", '∨', Main, Bin),
        ("setminus", '∖', Main, Bin),
        ("dagger", '†', Main, Bin),
        ("leq", '≤', Main, Rel),
        ("le", '≤', Main, Rel),
        ("geq", '≥', Main, Rel),
        ("ge", '≥', Main, Rel),
        ("leqslant", '⩽', Ams, Rel),
        ("geqslant", '⩾', Ams, Rel),
        ("neq", '≠', Main, Rel),
        ("ne", '≠', Main, Rel),
        ("approx", '≈', Main, Rel),
        ("equiv", '≡', Main, Rel),
        ("sim", '∼', Main, Rel),
        ("simeq", '≃', Main, Rel),
        ("cong", '≅', Main, Rel),
        ("propto", '∝', Main, Rel),
        ("ll", '≪', Main, Rel),
        ("gg", '≫', Main, Rel),
        ("in", '∈', Main, Rel),
        ("notin", '∉', Main, Rel),
        ("ni", '∋', Main, Rel),
        ("subset", '⊂', Main, Rel),
        ("supset", '⊃', Main, Rel),
        ("subseteq", '⊆', Main, Rel),
        ("supseteq", '⊇', Main, Rel),
        ("to", '→', Main, Rel),
        ("rightarrow", '→', Main, Rel),
        ("leftarrow", '←', Main, Rel),
        ("gets", '←', Main, Rel),
        ("leftrightarrow", '↔', Main, Rel),
        ("Rightarrow", '⇒', Main, Rel),
        ("Leftarrow", '⇐', Main, Rel),
        ("Leftrightarrow", '⇔', Main, Rel),
        ("implies", '⟹', Main, Rel),
        ("impliedby", '⟸', Main, Rel),
        ("iff", '⟺', Main, Rel),
        ("longrightarrow", '⟶', Main, Rel),
        ("longleftarrow", '⟵', Main, Rel),
        ("Longrightarrow", '⟹', Main, Rel),
        ("mapsto", '↦', Main, Rel),
        ("uparrow", '↑', Main, Rel),
        ("downarrow", '↓', Main, Rel),
        ("updownarrow", '↕', Main, Rel),
        ("nearrow", '↗', Main, Rel),
        ("searrow", '↘', Main, Rel),
        ("rightleftharpoons", '⇌', Main, Rel),
        ("leftrightharpoons", '⇋', Ams, Rel),
        ("rightharpoonup", '⇀', Main, Rel),
        ("perp", '⊥', Main, Rel),
        ("parallel", '∥', Main, Rel),
        ("mid", '∣', Main, Rel),
        ("models", '⊨', Main, Rel),
        ("vdash", '⊢', Main, Rel),
        ("doteq", '≐', Main, Rel),
        ("asymp", '≍', Main, Rel),
        ("therefore", '∴', Ams, Rel),
        ("because", '∵', Ams, Rel),
        ("lesssim", '≲', Ams, Rel),
        ("gtrsim", '≳', Ams, Rel),
        ("infty", '∞', Main, Ord),
        ("partial", '∂', Main, Ord),
        ("nabla", '∇', Main, Ord),
        ("hbar", 'ℏ', Main, Ord),
        ("ell", 'ℓ', Main, Ord),
        ("forall", '∀', Main, Ord),
        ("exists", '∃', Main, Ord),
        ("nexists", '∄', Ams, Ord),
        ("emptyset", '∅', Main, Ord),
        ("varnothing", '∅', Ams, Ord),
        ("neg", '¬', Main, Ord),
        ("lnot", '¬', Main, Ord),
        ("angle", '∠', Main, Ord),
        ("measuredangle", '∡', Ams, Ord),
        ("triangle", '△', Main, Ord),
        ("square", '□', Ams, Ord),
        ("Box", '□', Ams, Ord),
        ("prime", '′', Main, Ord),
        ("Re", 'ℜ', Main, Ord),
        ("Im", 'ℑ', Main, Ord),
        ("aleph", 'ℵ', Main, Ord),
        ("wp", '℘', Main, Ord),
        ("top", '⊤', Main, Ord),
        ("bot", '⊥', Main, Ord),
        ("checkmark", '✓', Ams, Ord),
        ("degree", '°', Main, Ord),
        ("ldots", '…', Main, Inner),
        ("dots", '…', Main, Inner),
        ("cdots", '⋯', Main, Inner),
        ("vdots", '⋮', Main, Ord),
        ("ddots", '⋱', Main, Inner),
        ("colon", ':', Main, Punct),
        ("lbrace", '{', Main, Open),
        ("rbrace", '}', Main, Close),
        ("langle", '⟨', Main, Open),
        ("rangle", '⟩', Main, Close),
        ("lfloor", '⌊', Main, Open),
        ("rfloor", '⌋', Main, Close),
        ("lceil", '⌈', Main, Open),
        ("rceil", '⌉', Main, Close),
        ("vert", '∣', Main, Ord),
        ("lvert", '∣', Main, Open),
        ("rvert", '∣', Main, Close),
        ("Vert", '∥', Main, Ord),
        ("lVert", '∥', Main, Open),
        ("rVert", '∥', Main, Close),
        ("backslash", '\\', Main, Ord),
        ("ddagger", '‡', Main, Bin),
        ("S", '§', Main, Ord),
        ("P", '¶', Main, Ord),
        ("copyright", '©', Main, Ord),
        ("pounds", '£', MainItalic, Ord),
        ("AA", 'Å', Main, Ord),
        ("circledast", '⊛', Ams, Bin),
        ("diamond", '⋄', Main, Bin),
        ("heartsuit", '♡', Main, Ord),
        ("spadesuit", '♠', Main, Ord),
        ("clubsuit", '♣', Main, Ord),
        ("diamondsuit", '♢', Main, Ord),
        ("sharp", '♯', Main, Ord),
        ("flat", '♭', Main, Ord),
        ("natural", '♮', Main, Ord),
        ("imath", 'ı', MathItalic, Ord),
        ("jmath", 'ȷ', MathItalic, Ord),
        ("lt", '<', Main, Rel),
        ("gt", '>', Main, Rel),
        ("%", '%', Main, Ord),
        ("$", '$', Main, Ord),
        ("#", '#', Main, Ord),
        ("&", '&', Main, Ord),
        ("_", '_', Main, Ord),
        ("{", '{', Main, Open),
        ("}", '}', Main, Close),
        ("|", '∥', Main, Ord),
    ];
    if let Some((_, c, f, k)) = t.iter().find(|x| x.0 == name) {
        return Some((*c, *f, *k));
    }
    greek(name).map(|(c, f)| (c, f, Ord))
}

fn big_op(name: &str) -> Option<(char, bool)> {
    Some(match name {
        "sum" => ('∑', true),
        "prod" => ('∏', true),
        "coprod" => ('∐', true),
        "int" => ('∫', false),
        "iint" => ('∬', false),
        "iiint" => ('∭', false),
        "oint" => ('∮', false),
        "bigcup" => ('⋃', true),
        "bigcap" => ('⋂', true),
        "bigoplus" => ('⨁', true),
        "bigotimes" => ('⨂', true),
        "bigvee" => ('⋁', true),
        "bigwedge" => ('⋀', true),
        "bigodot" => ('⨀', true),
        _ => return None,
    })
}

fn op_name(name: &str) -> Option<bool> {
    const NO_LIMITS: &[&str] = &[
        "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh",
        "tanh", "coth", "log", "ln", "lg", "exp", "dim", "ker", "deg", "arg", "hom", "sgn", "Tr",
        "tr", "erf", "Var", "Cov", "cis", "rank", "curl", "div", "grad",
    ];
    const LIMITS: &[&str] = &[
        "lim", "limsup", "liminf", "max", "min", "sup", "inf", "det", "gcd", "Pr", "argmax",
        "argmin",
    ];
    if NO_LIMITS.contains(&name) {
        Some(false)
    } else if LIMITS.contains(&name) {
        Some(true)
    } else {
        None
    }
}

fn accent(name: &str) -> Option<AccentKind> {
    use AccentKind::*;
    Some(match name {
        "hat" => Hat,
        "widehat" => WideHat,
        "tilde" | "widetilde" => Tilde,
        "bar" => Bar,
        "overline" => Overline,
        "underline" => Underline,
        "vec" => Vec,
        "overrightarrow" => OverRightArrow,
        "overleftarrow" => OverLeftArrow,
        "overleftrightarrow" => OverLeftRightArrow,
        "dot" => Dot,
        "ddot" => DDot,
        "acute" => Acute,
        "grave" => Grave,
        "breve" => Breve,
        "check" => Check,
        "mathring" => Ring,
        _ => return None,
    })
}

impl Parser {
    fn new(src: &str) -> Self {
        Self {
            s: src.chars().collect(),
            i: 0,
            parts: vec![],
        }
    }
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }
    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_whitespace()) {
            self.i += 1;
        }
    }
    fn read_command(&mut self) -> String {
        // after the backslash
        let mut name = String::new();
        if let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() {
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphabetic() {
                        name.push(c);
                        self.i += 1;
                    } else {
                        break;
                    }
                }
                // \operatorname* etc.
                if self.peek() == Some('*') && (name == "operatorname") {
                    self.i += 1;
                }
            } else {
                name.push(c);
                self.i += 1;
            }
        }
        name
    }
    /// Raw text inside `{...}` (balanced).
    fn read_raw_group(&mut self) -> String {
        self.skip_ws();
        if self.peek() != Some('{') {
            // single token
            let mut s = String::new();
            if let Some(c) = self.peek() {
                s.push(c);
                self.i += 1;
            }
            return s;
        }
        self.i += 1;
        let mut depth = 1;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            s.push(c);
        }
        s
    }
    fn read_optional(&mut self) -> Option<String> {
        self.skip_ws();
        if self.peek() != Some('[') {
            return None;
        }
        self.i += 1;
        let mut depth = 1;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            s.push(c);
        }
        Some(s)
    }
    fn sub_parse(&mut self, src: &str) -> Node {
        let mut p = Parser::new(src);
        p.parts = std::mem::take(&mut self.parts);
        let (nodes, _) = p.parse_list();
        self.parts = p.parts;
        Node::Group(nodes)
    }
    /// One argument: a group or a single token.
    fn arg(&mut self) -> Node {
        self.skip_ws();
        match self.peek() {
            Some('{') => {
                self.i += 1;
                let (nodes, _) = self.parse_list();
                Node::Group(nodes)
            }
            Some(_) => self.atom().unwrap_or(Node::Group(vec![])),
            None => Node::Group(vec![]),
        }
    }
    fn delim_char(&mut self) -> char {
        self.skip_ws();
        match self.peek() {
            Some('\\') => {
                self.i += 1;
                let n = self.read_command();
                match n.as_str() {
                    "{" | "lbrace" => '{',
                    "}" | "rbrace" => '}',
                    "langle" => '⟨',
                    "rangle" => '⟩',
                    "lfloor" => '⌊',
                    "rfloor" => '⌋',
                    "lceil" => '⌈',
                    "rceil" => '⌉',
                    "vert" | "lvert" | "rvert" => '|',
                    "Vert" | "|" | "lVert" | "rVert" => '‖',
                    "uparrow" => '↑',
                    "downarrow" => '↓',
                    "backslash" => '\\',
                    _ => '.',
                }
            }
            Some(c) => {
                self.i += 1;
                c
            }
            None => '.',
        }
    }

    /// Parse until a stop token; returns nodes and the stop.
    fn parse_list(&mut self) -> (Vec<Node>, Stop) {
        let mut out: Vec<Node> = Vec::new();
        loop {
            self.skip_ws();
            let Some(c) = self.peek() else {
                return (out, Stop::Eof);
            };
            match c {
                '}' => {
                    self.i += 1;
                    return (out, Stop::Brace);
                }
                '&' => {
                    self.i += 1;
                    return (out, Stop::Amp);
                }
                '^' | '_' | '\'' => {
                    let base = out.pop().unwrap_or(Node::Group(vec![]));
                    let n = self.scripts(base);
                    out.push(n);
                    continue;
                }
                '\\' => {
                    // check stop commands
                    let save = self.i;
                    self.i += 1;
                    let name = self.read_command();
                    match name.as_str() {
                        "\\" | "cr" | "newline" => {
                            let _ = self.read_optional();
                            return (out, Stop::Newline);
                        }
                        "right" => {
                            let d = self.delim_char();
                            return (out, Stop::Right(d));
                        }
                        "middle" => {
                            let _ = self.delim_char();
                            return (out, Stop::Middle);
                        }
                        "end" => {
                            let env = self.read_raw_group();
                            return (out, Stop::End(env));
                        }
                        "color" => {
                            // \color{c} applies to the rest of the group
                            let cname = self.read_raw_group();
                            let col = Color::parse(&cname).unwrap_or(WHITE);
                            if self.peek() == Some('{') {
                                // KaTeX-compatible \color{c}{body}
                                let body = self.arg();
                                out.push(Node::Color(col, vec![body]));
                            } else {
                                let (rest, stop) = self.parse_list();
                                out.push(Node::Color(col, rest));
                                return (out, stop);
                            }
                            continue;
                        }
                        "displaystyle" | "textstyle" | "scriptstyle" | "scriptscriptstyle" => {
                            out.push(Node::StyleChange(match name.as_str() {
                                "displaystyle" => MStyle::Display,
                                "textstyle" => MStyle::Text,
                                "scriptstyle" => MStyle::Script,
                                _ => MStyle::ScriptScript,
                            }));
                            continue;
                        }
                        _ => {
                            self.i = save;
                        }
                    }
                    if let Some(n) = self.atom() {
                        out.push(n);
                    }
                }
                _ => {
                    if let Some(n) = self.atom() {
                        out.push(n);
                    }
                }
            }
        }
    }

    fn scripts(&mut self, base: Node) -> Node {
        let mut sup: Option<Node> = None;
        let mut sub: Option<Node> = None;
        let (base, mut primes) = match base {
            Node::Scripts {
                base,
                sup: s,
                sub: b,
            } => {
                sup = s.map(|x| *x);
                sub = b.map(|x| *x);
                (*base, 0)
            }
            b => (b, 0),
        };
        loop {
            match self.peek() {
                Some('^') => {
                    self.i += 1;
                    let a = self.arg();
                    sup = Some(match (sup.take(), primes) {
                        (Some(Node::Group(mut g)), _) => {
                            g.push(a);
                            Node::Group(g)
                        }
                        _ if primes > 0 => {
                            let mut g: Vec<Node> = (0..primes)
                                .map(|_| Node::Sym {
                                    ch: '′',
                                    font: MFont::Main,
                                    class: Class::Ord,
                                })
                                .collect();
                            g.push(a);
                            primes = 0;
                            Node::Group(g)
                        }
                        _ => a,
                    });
                }
                Some('_') => {
                    self.i += 1;
                    sub = Some(self.arg());
                }
                Some('\'') => {
                    self.i += 1;
                    primes += 1;
                }
                _ => break,
            }
            self.skip_ws();
        }
        if primes > 0 {
            let mut g: Vec<Node> = match sup.take() {
                Some(Node::Group(g)) => g,
                Some(n) => vec![n],
                None => vec![],
            };
            for _ in 0..primes {
                g.insert(
                    0,
                    Node::Sym {
                        ch: '′',
                        font: MFont::Main,
                        class: Class::Ord,
                    },
                );
            }
            sup = Some(Node::Group(g));
        }
        Node::Scripts {
            base: Box::new(base),
            sup: sup.map(Box::new),
            sub: sub.map(Box::new),
        }
    }

    fn atom(&mut self) -> Option<Node> {
        self.skip_ws();
        let c = self.peek()?;
        self.i += 1;
        use Class::*;
        Some(match c {
            '{' => {
                // manim-style {{ part }}
                if self.peek() == Some('{') {
                    let start = self.i + 1;
                    self.i += 1;
                    let (nodes, _) = self.parse_list();
                    let end = self.i.saturating_sub(1);
                    if self.peek() == Some('}') {
                        self.i += 1;
                    }
                    let src: String = self.s[start..end.max(start)].iter().collect();
                    self.parts.push(src.trim().to_string());
                    return Some(Node::Part(self.parts.len() - 1, nodes));
                }
                let (nodes, _) = self.parse_list();
                Node::Group(nodes)
            }
            '\\' => return self.command(),
            'a'..='z' | 'A'..='Z' => Node::Sym {
                ch: c,
                font: MFont::MathItalic,
                class: Ord,
            },
            '0'..='9' | '.' => Node::Sym {
                ch: c,
                font: MFont::Main,
                class: Ord,
            },
            '+' => Node::Sym {
                ch: '+',
                font: MFont::Main,
                class: Bin,
            },
            '-' => Node::Sym {
                ch: '−',
                font: MFont::Main,
                class: Bin,
            },
            '*' => Node::Sym {
                ch: '∗',
                font: MFont::Main,
                class: Bin,
            },
            '=' | '<' | '>' | ':' => Node::Sym {
                ch: c,
                font: MFont::Main,
                class: Rel,
            },
            '(' | '[' => Node::Sym {
                ch: c,
                font: MFont::Main,
                class: Open,
            },
            ')' | ']' => Node::Sym {
                ch: c,
                font: MFont::Main,
                class: Close,
            },
            ',' | ';' => Node::Sym {
                ch: c,
                font: MFont::Main,
                class: Punct,
            },
            '!' | '?' => Node::Sym {
                ch: c,
                font: MFont::Main,
                class: Close,
            },
            '~' => Node::Space(0.333),
            '|' => Node::Sym {
                ch: '∣',
                font: MFont::Main,
                class: Ord,
            },
            'α'..='ω' => Node::Sym {
                ch: c,
                font: MFont::MathItalic,
                class: Ord,
            },
            '→' | '←' | '⇌' | '≤' | '≥' | '≠' | '≈' | '⇒' | '∈' => Node::Sym {
                ch: c,
                font: MFont::Main,
                class: Rel,
            },
            '±' | '×' | '÷' | '·' | '∓' => Node::Sym {
                ch: if c == '·' { '⋅' } else { c },
                font: MFont::Main,
                class: Bin,
            },
            '°' => Node::Sym {
                ch: '°',
                font: MFont::Main,
                class: Ord,
            },
            _ => Node::Sym {
                ch: c,
                font: MFont::Main,
                class: Ord,
            },
        })
    }

    fn command(&mut self) -> Option<Node> {
        let name = self.read_command();
        use Class::*;
        let n = name.as_str();
        // spacing
        let space = match n {
            "," | "thinspace" => Some(3.0 * MU),
            ":" | ">" | "medspace" => Some(4.0 * MU),
            ";" | "thickspace" => Some(5.0 * MU),
            "!" | "negthinspace" => Some(-3.0 * MU),
            " " => Some(0.333),
            "quad" => Some(1.0),
            "qquad" => Some(2.0),
            "enspace" => Some(0.5),
            _ => None,
        };
        if let Some(s) = space {
            return Some(Node::Space(s));
        }
        if let Some((ch, limits)) = big_op(n) {
            let mut lim = None;
            self.skip_ws();
            if self.s[self.i..].starts_with(&['\\', 'l', 'i', 'm', 'i', 't', 's']) {
                self.i += 7;
                lim = Some(true);
            } else if self.s[self.i..].starts_with(&['\\', 'n', 'o', 'l', 'i', 'm', 'i', 't', 's'])
            {
                self.i += 9;
                lim = Some(false);
            }
            return Some(Node::BigOp {
                ch,
                limits: lim.or(if limits { None } else { Some(false) }),
            });
        }
        if let Some(limits) = op_name(n) {
            let mut limits = limits;
            self.skip_ws();
            if self.s[self.i..].starts_with(&['\\', 'l', 'i', 'm', 'i', 't', 's']) {
                self.i += 7;
                limits = true;
            }
            return Some(Node::OpName {
                name: name.clone(),
                limits,
            });
        }
        if let Some(k) = accent(n) {
            let body = self.arg();
            return Some(Node::Accent {
                kind: k,
                body: Box::new(body),
            });
        }
        if let Some((ch, font, class)) = symbol(n) {
            return Some(Node::Sym { ch, font, class });
        }
        Some(match n {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let num = self.arg();
                let den = self.arg();
                Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    style: match n {
                        "dfrac" | "cfrac" => Some(MStyle::Display),
                        "tfrac" => Some(MStyle::Text),
                        _ => None,
                    },
                    rule: true,
                    delims: None,
                }
            }
            "binom" | "dbinom" | "tbinom" | "choose" => {
                let num = self.arg();
                let den = self.arg();
                Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    style: match n {
                        "dbinom" => Some(MStyle::Display),
                        "tbinom" => Some(MStyle::Text),
                        _ => None,
                    },
                    rule: false,
                    delims: Some(('(', ')')),
                }
            }
            "sqrt" => {
                let idx = self.read_optional().map(|s| Box::new(self.sub_parse(&s)));
                let body = self.arg();
                Node::Sqrt {
                    index: idx,
                    body: Box::new(body),
                }
            }
            "left" => {
                let l = self.delim_char();
                let mut body = Vec::new();
                let r = loop {
                    let (nodes, stop) = self.parse_list();
                    body.extend(nodes);
                    match stop {
                        Stop::Right(r) => break r,
                        Stop::Middle => {
                            body.push(Node::Sym {
                                ch: '∣',
                                font: MFont::Main,
                                class: Rel,
                            });
                        }
                        _ => break '.',
                    }
                };
                Node::Delimited {
                    left: l,
                    right: r,
                    body: Box::new(Node::Group(body)),
                }
            }
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "Bigl" | "biggl" | "Biggl" | "bigr"
            | "Bigr" | "biggr" | "Biggr" | "bigm" | "Bigm" => {
                let ch = self.delim_char();
                let size = if n.starts_with("Bigg") {
                    4
                } else if n.starts_with("bigg") {
                    3
                } else if n.starts_with("Big") {
                    2
                } else {
                    1
                };
                let class = if n.ends_with('l') {
                    Open
                } else if n.ends_with('r') {
                    Close
                } else if n.ends_with('m') {
                    Rel
                } else {
                    Ord
                };
                Node::BigDelim { ch, size, class }
            }
            "text" | "textrm" | "mbox" | "textnormal" | "textup" | "hbox" => Node::Text {
                s: self.read_raw_group(),
                font: MFont::Main,
            },
            "textbf" => Node::Text {
                s: self.read_raw_group(),
                font: MFont::MainBold,
            },
            "textit" | "emph" => Node::Text {
                s: self.read_raw_group(),
                font: MFont::MainItalic,
            },
            "textsf" => Node::Text {
                s: self.read_raw_group(),
                font: MFont::Inter,
            },
            "texttt" => Node::Text {
                s: self.read_raw_group(),
                font: MFont::Mono,
            },
            "operatorname" => {
                let s = self.read_raw_group();
                Node::OpName {
                    name: s,
                    limits: false,
                }
            }
            "mathrm" | "mathbf" | "mathit" | "mathbb" | "mathcal" | "mathfrak" | "mathsf"
            | "mathtt" | "boldsymbol" | "bm" | "mathscr" => {
                let font = match n {
                    "mathrm" => MFont::Main,
                    "mathbf" => MFont::MainBold,
                    "mathit" => MFont::MainItalic,
                    "mathbb" => MFont::Ams,
                    "mathcal" | "mathscr" => MFont::Cal,
                    "mathfrak" => MFont::Frak,
                    "mathsf" => MFont::Sans,
                    "mathtt" => MFont::Mono,
                    _ => MFont::MathBold,
                };
                let a = self.arg();
                refont(a, font)
            }
            "textcolor" => {
                let c = self.read_raw_group();
                let body = self.arg();
                Node::Color(Color::parse(&c).unwrap_or(WHITE), vec![body])
            }
            "boxed" | "fbox" => Node::Boxed(Box::new(self.arg())),
            "cancel" | "bcancel" | "xcancel" | "sout" => Node::Cancel(Box::new(self.arg())),
            "phantom" | "hphantom" | "vphantom" => Node::Phantom(Box::new(self.arg())),
            "overbrace" | "underbrace" => {
                let body = self.arg();
                let over = n == "overbrace";
                self.skip_ws();
                let label = match self.peek() {
                    Some('^') if over => {
                        self.i += 1;
                        Some(Box::new(self.arg()))
                    }
                    Some('_') if !over => {
                        self.i += 1;
                        Some(Box::new(self.arg()))
                    }
                    _ => None,
                };
                Node::Brace {
                    body: Box::new(body),
                    label,
                    over,
                }
            }
            "overset" | "stackrel" => {
                let over = self.arg();
                let base = self.arg();
                Node::Overset {
                    base: Box::new(base),
                    over: Some(Box::new(over)),
                    under: None,
                    class: if n == "stackrel" { Rel } else { Ord },
                }
            }
            "underset" => {
                let under = self.arg();
                let base = self.arg();
                Node::Overset {
                    base: Box::new(base),
                    over: None,
                    under: Some(Box::new(under)),
                    class: Ord,
                }
            }
            "xrightarrow" | "xleftarrow" | "xleftrightarrow" | "xrightleftharpoons"
            | "xRightarrow" => {
                let under = self.read_optional().map(|s| Box::new(self.sub_parse(&s)));
                let over = self.arg();
                let over = match &over {
                    Node::Group(g) if g.is_empty() => None,
                    _ => Some(Box::new(over)),
                };
                Node::XArrow {
                    kind: match n {
                        "xleftarrow" => ArrowKind::Left,
                        "xleftrightarrow" => ArrowKind::LeftRight,
                        "xrightleftharpoons" => ArrowKind::RightLeftHarpoons,
                        "xRightarrow" => ArrowKind::DoubleRight,
                        _ => ArrowKind::Right,
                    },
                    over,
                    under,
                }
            }
            "ce" => {
                let s = self.read_raw_group();
                let tex = chem_to_tex(&s);
                self.sub_parse(&tex)
            }
            "celsius" => self.sub_parse("{}^{\\circ}\\mathrm{C}"),
            "ohm" | "Omega_unit" => Node::Sym {
                ch: 'Ω',
                font: MFont::Main,
                class: Ord,
            },
            "begin" => {
                let env = self.read_raw_group();
                self.environment(&env)
            }
            "label" | "tag" | "nonumber" | "notag" => {
                if n == "label" || n == "tag" {
                    let _ = self.read_raw_group();
                }
                Node::Group(vec![])
            }
            "not" => {
                let a = self.atom()?;
                Node::Cancel(Box::new(a))
            }
            "limits" | "nolimits" | "left." | "relax" => Node::Group(vec![]),
            other => Node::Text {
                s: format!("\\{other}"),
                font: MFont::Main,
            },
        })
    }

    fn environment(&mut self, env: &str) -> Node {
        let env = env.trim_end_matches('*');
        let mut align: Vec<char> = Vec::new();
        if env == "array" {
            let spec = self.read_raw_group();
            align = spec
                .chars()
                .filter(|c| matches!(c, 'l' | 'c' | 'r'))
                .collect();
        }
        // parse rows
        let mut rows: Vec<Vec<Node>> = vec![vec![]];
        loop {
            let (nodes, stop) = self.parse_list();
            rows.last_mut().unwrap().push(Node::Group(nodes));
            match stop {
                Stop::Amp => {}
                Stop::Newline => rows.push(vec![]),
                _ => break,
            }
        }
        // drop trailing empty row
        if rows.len() > 1 {
            if let Some(last) = rows.last() {
                if last.len() == 1 {
                    if let Node::Group(g) = &last[0] {
                        if g.is_empty() {
                            rows.pop();
                        }
                    }
                }
            }
        }
        let (left, right) = match env {
            "pmatrix" => ('(', ')'),
            "bmatrix" => ('[', ']'),
            "Bmatrix" => ('{', '}'),
            "vmatrix" => ('|', '|'),
            "Vmatrix" => ('‖', '‖'),
            "cases" | "dcases" => ('{', '.'),
            "rcases" => ('.', '}'),
            _ => ('.', '.'),
        };
        let pair_align = matches!(
            env,
            "aligned" | "align" | "split" | "alignat" | "eqnarray" | "alignedat"
        );
        let display = pair_align || matches!(env, "gathered" | "gather" | "dcases");
        let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1);
        if align.is_empty() {
            align = match env {
                "cases" | "dcases" | "rcases" => vec!['l'; cols],
                "gathered" | "gather" => vec!['c'; cols],
                _ if pair_align => (0..cols)
                    .map(|i| if i % 2 == 0 { 'r' } else { 'l' })
                    .collect(),
                _ => vec!['c'; cols],
            };
        }
        let col_gap = match env {
            "cases" | "dcases" | "rcases" => 1.0,
            _ if pair_align => 0.0,
            "smallmatrix" => 0.5,
            _ => 1.0,
        };
        Node::Array {
            rows,
            align,
            col_gap,
            left,
            right,
            display,
            pair_align,
        }
    }
}

/// Apply a font switch to letters/digits in a parsed argument.
fn refont(n: Node, font: MFont) -> Node {
    match n {
        Node::Sym { ch, class, .. } if ch.is_alphanumeric() => Node::Sym { ch, font, class },
        Node::Group(g) => Node::Group(g.into_iter().map(|x| refont(x, font)).collect()),
        Node::Scripts { base, sup, sub } => Node::Scripts {
            base: Box::new(refont(*base, font)),
            sup,
            sub,
        },
        other => other,
    }
}

/// Parse a TeX string. Top-level `\\` makes a centred multi-line block
/// (aligned at `&` when present).
pub fn parse(src: &str) -> (Node, Vec<String>) {
    let mut p = Parser::new(src);
    let mut rows: Vec<Vec<Node>> = vec![vec![]];
    let mut has_amp = false;
    loop {
        let (nodes, stop) = p.parse_list();
        rows.last_mut().unwrap().push(Node::Group(nodes));
        match stop {
            Stop::Eof => break,
            Stop::Newline => rows.push(vec![]),
            Stop::Amp => has_amp = true,
            _ => {}
        }
    }
    let node = if rows.len() == 1 && rows[0].len() == 1 {
        rows.pop().unwrap().pop().unwrap()
    } else {
        let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1);
        Node::Array {
            rows,
            align: if has_amp {
                (0..cols)
                    .map(|i| if i % 2 == 0 { 'r' } else { 'l' })
                    .collect()
            } else {
                vec!['c'; cols]
            },
            col_gap: 0.0,
            left: '.',
            right: '.',
            display: true,
            pair_align: has_amp,
        }
    };
    (node, p.parts)
}

// ------------------------------------------------------------------- layout

#[derive(Clone, Debug)]
enum LItem {
    Glyph {
        font: FontId,
        gid: u16,
        scale: f64,
        ch: char,
    },
    /// Filled path in em units relative to the item position.
    Shape(Arc<Path>),
}

#[derive(Clone, Debug)]
struct Placed {
    x: f64,
    y: f64,
    item: LItem,
    color: Option<Color>,
    part: Option<usize>,
}

#[derive(Clone, Debug, Default)]
struct LBox {
    w: f64,
    h: f64,
    d: f64,
    items: Vec<Placed>,
}

impl LBox {
    fn place(&mut self, b: &LBox, dx: f64, dy: f64) {
        for it in &b.items {
            self.items.push(Placed {
                x: it.x + dx,
                y: it.y + dy,
                ..it.clone()
            });
        }
    }
    fn hspace(w: f64) -> LBox {
        LBox {
            w,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy)]
struct Ctx {
    style: MStyle,
    color: Option<Color>,
    part: Option<usize>,
    cramped: bool,
}

impl Ctx {
    fn s(&self) -> f64 {
        self.style.scale()
    }
    fn with_style(self, st: MStyle) -> Ctx {
        Ctx { style: st, ..self }
    }
}

fn rect_path(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
    Path::polyline(&[v2(x0, y0), v2(x1, y0), v2(x1, y1), v2(x0, y1)], true)
}

/// Thick line segment as a filled quad (extended by half thickness at the ends).
fn thick_segment(p: Vec2, q: Vec2, t: f64, ext: bool) -> SubPath {
    let d = (q - p).normalize();
    let n = d.perp() * (t / 2.0);
    let e = if ext { d * (t / 2.0) } else { Vec2::ZERO };
    let (a, b) = (p - e, q + e);
    let path = Path::polyline(&[a + n, b + n, b - n, a - n], true);
    path.subpaths.into_iter().next().unwrap()
}

fn shape_item(path: Path, ctx: &Ctx) -> Placed {
    Placed {
        x: 0.0,
        y: 0.0,
        item: LItem::Shape(Arc::new(path)),
        color: ctx.color,
        part: ctx.part,
    }
}

fn glyph_box(ch: char, font: MFont, ctx: &Ctx) -> LBox {
    let s = ctx.s();
    match resolve(ch, font) {
        Some((fid, gid)) => {
            let (h, d) = glyph_extent(fid, gid);
            let w = advance(fid, gid);
            LBox {
                w: w * s,
                h: h * s,
                d: d * s,
                items: vec![Placed {
                    x: 0.0,
                    y: 0.0,
                    item: LItem::Glyph {
                        font: fid,
                        gid,
                        scale: s,
                        ch,
                    },
                    color: ctx.color,
                    part: ctx.part,
                }],
            }
        }
        None => LBox::hspace(0.5 * s),
    }
}

fn text_box(s: &str, font: MFont, ctx: &Ctx) -> LBox {
    let mut b = LBox::default();
    for ch in s.chars() {
        if ch == ' ' || ch == '~' {
            b.w += 0.333 * ctx.s();
            continue;
        }
        let g = glyph_box(ch, font, ctx);
        b.place(&g, b.w, 0.0);
        b.w += g.w;
        b.h = b.h.max(g.h);
        b.d = b.d.max(g.d);
    }
    b
}

fn spacing(l: Class, r: Class, script: bool) -> f64 {
    use Class::*;
    let thin = 3.0 * MU;
    let med = 4.0 * MU;
    let thick = 5.0 * MU;
    let v = match (l, r) {
        (Ord, Op)
        | (Op, Ord)
        | (Op, Op)
        | (Ord, Inner)
        | (Op, Inner)
        | (Close, Op)
        | (Close, Inner) => thin,
        (Inner, Ord) | (Inner, Op) | (Inner, Open) | (Inner, Punct) | (Inner, Inner) => thin,
        (Punct, _) => {
            if r == Bin {
                0.0
            } else {
                thin
            }
        }
        (Ord, Bin)
        | (Bin, Ord)
        | (Bin, Op)
        | (Bin, Open)
        | (Bin, Inner)
        | (Close, Bin)
        | (Inner, Bin) => med,
        (Ord, Rel)
        | (Op, Rel)
        | (Rel, Ord)
        | (Rel, Op)
        | (Rel, Open)
        | (Rel, Inner)
        | (Close, Rel)
        | (Inner, Rel) => thick,
        _ => 0.0,
    };
    let thin_ok = matches!((l, r), (Ord, Op) | (Op, Ord) | (Op, Op) | (Close, Op));
    if script && !(v == thin && thin_ok) {
        0.0
    } else {
        v
    }
}

fn node_class(n: &Node) -> Option<Class> {
    Some(match n {
        Node::Sym { class, .. } => *class,
        Node::Scripts { base, .. } => node_class(base).unwrap_or(Class::Ord),
        Node::Frac { .. } | Node::Delimited { .. } => Class::Inner,
        Node::BigOp { .. } | Node::OpName { .. } => Class::Op,
        Node::BigDelim { class, .. } => *class,
        Node::Space(_) | Node::StyleChange(_) => return None,
        Node::Overset { class, .. } => *class,
        Node::XArrow { .. } => Class::Rel,
        Node::Color(_, v) | Node::Part(_, v) if v.len() == 1 => return node_class(&v[0]),
        _ => Class::Ord,
    })
}

/// Flatten parts/colours into (node, ctx) pairs so spacing is computed across them.
fn flatten<'a>(nodes: &'a [Node], ctx: Ctx, out: &mut Vec<(&'a Node, Ctx)>) {
    let mut ctx = ctx;
    for n in nodes {
        match n {
            Node::Part(i, v) => flatten(
                v,
                Ctx {
                    part: Some(*i),
                    ..ctx
                },
                out,
            ),
            Node::Color(c, v) => flatten(
                v,
                Ctx {
                    color: Some(*c),
                    ..ctx
                },
                out,
            ),
            Node::StyleChange(st) => ctx.style = *st,
            _ => out.push((n, ctx)),
        }
    }
}

fn layout_list(nodes: &[Node], ctx: Ctx) -> LBox {
    let mut flat = Vec::new();
    flatten(nodes, ctx, &mut flat);
    // classes with bin→ord adjustments
    let mut classes: Vec<Option<Class>> = flat.iter().map(|(n, _)| node_class(n)).collect();
    let mut prev: Option<Class> = None;
    for c in classes.iter_mut().flatten() {
        if *c == Class::Bin
            && matches!(
                prev,
                None | Some(Class::Bin)
                    | Some(Class::Op)
                    | Some(Class::Rel)
                    | Some(Class::Open)
                    | Some(Class::Punct)
            )
        {
            *c = Class::Ord;
        }
        prev = Some(*c);
    }
    // bin followed by rel/close/punct/end → ord
    let idx: Vec<usize> = (0..classes.len())
        .filter(|&i| classes[i].is_some())
        .collect();
    for (k, &i) in idx.iter().enumerate() {
        if classes[i] == Some(Class::Bin) {
            let next = idx.get(k + 1).and_then(|&j| classes[j]);
            if matches!(
                next,
                None | Some(Class::Rel) | Some(Class::Close) | Some(Class::Punct)
            ) {
                classes[i] = Some(Class::Ord);
            }
        }
    }
    let mut b = LBox::default();
    let mut prev: Option<Class> = None;
    for ((n, c), cls) in flat.iter().zip(&classes) {
        if let (Some(p), Some(cl)) = (prev, cls) {
            b.w += spacing(p, *cl, c.style.is_script()) * c.s();
        }
        let nb = layout_node(n, *c);
        b.place(&nb, b.w, 0.0);
        b.w += nb.w;
        b.h = b.h.max(nb.h);
        b.d = b.d.max(nb.d);
        if cls.is_some() {
            prev = *cls;
        }
    }
    b
}

fn layout_node(n: &Node, ctx: Ctx) -> LBox {
    let s = ctx.s();
    match n {
        Node::Sym { ch, font, .. } => glyph_box(*ch, *font, &ctx),
        Node::Text { s: txt, font } => text_box(txt, *font, &ctx),
        Node::Group(v) => layout_list(v, ctx),
        Node::Part(..) | Node::Color(..) => layout_list(std::slice::from_ref(n), ctx),
        Node::Space(w) => LBox::hspace(w * s),
        Node::StyleChange(_) => LBox::default(),
        Node::Phantom(body) => {
            let b = layout_node(body, ctx);
            LBox { items: vec![], ..b }
        }
        Node::OpName { name, .. } => text_box(name, MFont::Main, &ctx),
        Node::BigOp { ch, .. } => {
            let font = if ctx.style == MStyle::Display {
                MFont::Size(2)
            } else {
                MFont::Size(1)
            };
            let mut g = glyph_box(*ch, font, &ctx);
            // centre on the math axis
            let shift = (g.h - g.d) / 2.0 - AXIS * s;
            for it in &mut g.items {
                it.y -= shift;
            }
            g.h -= shift;
            g.d += shift;
            g
        }
        Node::Scripts { base, sup, sub } => {
            layout_scripts(base, sup.as_deref(), sub.as_deref(), ctx)
        }
        Node::Frac {
            num,
            den,
            style,
            rule,
            delims,
        } => layout_frac(num, den, *style, *rule, *delims, ctx),
        Node::Sqrt { index, body } => layout_sqrt(index.as_deref(), body, ctx),
        Node::Delimited { left, right, body } => {
            let inner = layout_node(body, ctx);
            let delta = (inner.h - AXIS * s).max(inner.d + AXIS * s);
            let need = (delta * 2.0 * 0.901).max(2.0 * delta - 0.5 * s);
            let lb = delimiter(*left, need, ctx);
            let rb = delimiter(*right, need, ctx);
            hjoin(&[lb, inner, rb])
        }
        Node::BigDelim { ch, size, .. } => {
            let need = [1.2, 1.8, 2.4, 3.0][(*size as usize - 1).min(3)] * s;
            delimiter(*ch, need, ctx)
        }
        Node::Accent { kind, body } => layout_accent(*kind, body, ctx),
        Node::Array {
            rows,
            align,
            col_gap,
            left,
            right,
            display,
            pair_align,
        } => layout_array(
            rows,
            align,
            *col_gap,
            *left,
            *right,
            *display,
            *pair_align,
            ctx,
        ),
        Node::Boxed(body) => {
            let b = layout_node(body, ctx);
            let pad = 0.3 * s;
            let t = RULE * s;
            let (x0, x1) = (0.0, b.w + 2.0 * pad);
            let (y0, y1) = (-b.d - pad, b.h + pad);
            let mut out = LBox {
                w: x1,
                h: y1,
                d: -y0,
                items: vec![],
            };
            out.place(&b, pad, 0.0);
            let mut p = rect_path(x0, y0, x1, y0 + t);
            p.subpaths.extend(rect_path(x0, y1 - t, x1, y1).subpaths);
            p.subpaths.extend(rect_path(x0, y0, x0 + t, y1).subpaths);
            p.subpaths.extend(rect_path(x1 - t, y0, x1, y1).subpaths);
            out.items.push(shape_item(p, &ctx));
            out
        }
        Node::Cancel(body) => {
            let mut b = layout_node(body, ctx);
            let seg = thick_segment(v2(0.0, -b.d), v2(b.w, b.h), RULE * s * 1.2, false);
            b.items.push(shape_item(
                Path {
                    subpaths: vec![seg],
                },
                &ctx,
            ));
            b
        }
        Node::Brace { body, label, over } => {
            let b = layout_node(body, ctx);
            let gap = 0.1 * s;
            let bm = if *over {
                crate::shapes::brace_between(v2(b.w, b.h + gap), v2(0.0, b.h + gap))
            } else {
                crate::shapes::brace_between(v2(0.0, -b.d - gap), v2(b.w, -b.d - gap))
            };
            let bb = bm.bounds_or_origin();
            let mut out = LBox {
                w: b.w,
                h: b.h.max(bb.max.y),
                d: b.d.max(-bb.min.y),
                items: vec![],
            };
            out.place(&b, 0.0, 0.0);
            out.items.push(shape_item(bm.path.clone(), &ctx));
            if let Some(l) = label {
                let lb = layout_node(l, ctx.with_style(ctx.style.sup()));
                let x = (b.w - lb.w) / 2.0;
                if *over {
                    let y = bb.max.y + 0.08 * s + lb.d;
                    out.place(&lb, x, y);
                    out.h = out.h.max(y + lb.h);
                } else {
                    let y = bb.min.y - 0.08 * s - lb.h;
                    out.place(&lb, x, y);
                    out.d = out.d.max(-(y - lb.d));
                }
                if lb.w > out.w {
                    // widen symmetrically
                    let extra = (lb.w - out.w) / 2.0;
                    for it in &mut out.items {
                        it.x += extra;
                    }
                    out.w = lb.w;
                }
            }
            out
        }
        Node::Overset {
            base, over, under, ..
        } => {
            let b = layout_node(base, ctx);
            let small = ctx.with_style(ctx.style.sup());
            let o = over.as_ref().map(|x| layout_node(x, small));
            let u = under.as_ref().map(|x| layout_node(x, small));
            let w =
                b.w.max(o.as_ref().map_or(0.0, |x| x.w))
                    .max(u.as_ref().map_or(0.0, |x| x.w));
            let mut out = LBox {
                w,
                h: b.h,
                d: b.d,
                items: vec![],
            };
            out.place(&b, (w - b.w) / 2.0, 0.0);
            if let Some(o) = o {
                let y = b.h + 0.1 * s + o.d;
                out.place(&o, (w - o.w) / 2.0, y);
                out.h = y + o.h;
            }
            if let Some(u) = u {
                let y = -b.d - 0.1 * s - u.h;
                out.place(&u, (w - u.w) / 2.0, y);
                out.d = -(y - u.d);
            }
            out
        }
        Node::XArrow { kind, over, under } => {
            let small = ctx.with_style(ctx.style.sup());
            let o = over.as_ref().map(|x| layout_node(x, small));
            let u = under.as_ref().map(|x| layout_node(x, small));
            let lw = o
                .as_ref()
                .map_or(0.0, |x| x.w)
                .max(u.as_ref().map_or(0.0, |x| x.w));
            let w = (lw + 0.8 * s).max(1.6 * s);
            let mut out = LBox {
                w: w + 0.2 * s,
                h: AXIS * s + 0.25 * s,
                d: 0.1 * s,
                items: vec![],
            };
            let p = arrow_shape(*kind, 0.1 * s, w + 0.1 * s, AXIS * s, s);
            out.items.push(shape_item(p, &ctx));
            if let Some(o) = o {
                let y = AXIS * s + 0.22 * s + o.d;
                out.place(&o, (out.w - o.w) / 2.0, y);
                out.h = y + o.h;
            }
            if let Some(u) = u {
                let y = AXIS * s - 0.22 * s - u.h;
                out.place(&u, (out.w - u.w) / 2.0, y);
                out.d = out.d.max(-(y - u.d));
            }
            out
        }
    }
}

fn hjoin(boxes: &[LBox]) -> LBox {
    let mut out = LBox::default();
    for b in boxes {
        out.place(b, out.w, 0.0);
        out.w += b.w;
        out.h = out.h.max(b.h);
        out.d = out.d.max(b.d);
    }
    out
}

fn is_single_char(n: &Node) -> bool {
    match n {
        Node::Sym { .. } => true,
        Node::Group(v) => v.len() == 1 && is_single_char(&v[0]),
        _ => false,
    }
}

fn layout_scripts(base: &Node, sup: Option<&Node>, sub: Option<&Node>, ctx: Ctx) -> LBox {
    let s = ctx.s();
    // limits for big operators in display style
    let limits = match base {
        Node::BigOp { limits, .. } => limits.unwrap_or(ctx.style == MStyle::Display),
        Node::OpName { limits, .. } => *limits && ctx.style == MStyle::Display,
        _ => false,
    };
    let b = layout_node(base, ctx);
    let sctx = ctx.with_style(ctx.style.sup());
    let sup_b = sup.map(|n| layout_node(n, sctx));
    let sub_b = sub.map(|n| {
        layout_node(
            n,
            Ctx {
                cramped: true,
                ..sctx
            },
        )
    });
    if limits {
        let w =
            b.w.max(sup_b.as_ref().map_or(0.0, |x| x.w))
                .max(sub_b.as_ref().map_or(0.0, |x| x.w));
        let mut out = LBox {
            w,
            h: b.h,
            d: b.d,
            items: vec![],
        };
        out.place(&b, (w - b.w) / 2.0, 0.0);
        if let Some(u) = sup_b {
            let gap = (BIG_OP_SPACING1 * s).max(BIG_OP_SPACING3 * s - u.d);
            let y = b.h + gap + u.d;
            out.place(&u, (w - u.w) / 2.0, y);
            out.h = y + u.h + BIG_OP_SPACING5 * s;
        }
        if let Some(l) = sub_b {
            let gap = (BIG_OP_SPACING2 * s).max(BIG_OP_SPACING4 * s - l.h);
            let y = -b.d - gap - l.h;
            out.place(&l, (w - l.w) / 2.0, y);
            out.d = -(y - l.d) + BIG_OP_SPACING5 * s;
        }
        return out;
    }
    let ss = sctx.s();
    let (mut u, mut v) = if is_single_char(base) {
        (0.0, 0.0)
    } else {
        (b.h - SUP_DROP * ss, b.d + SUB_DROP * ss)
    };
    let mut out = LBox {
        w: b.w,
        h: b.h,
        d: b.d,
        items: vec![],
    };
    out.place(&b, 0.0, 0.0);
    let x = b.w;
    // integrals: italic correction pushes the upper limit right, lower limit left
    let (sup_dx, sub_dx) = match base {
        Node::BigOp {
            ch: '∫' | '∬' | '∭' | '∮',
            ..
        } => {
            if ctx.style == MStyle::Display {
                (0.22 * s, -0.04 * s)
            } else {
                (0.1 * s, -0.02 * s)
            }
        }
        _ => (0.0, 0.0),
    };
    let sup_min = if ctx.style == MStyle::Display {
        SUP1
    } else if ctx.cramped {
        SUP3
    } else {
        SUP2
    } * s;
    match (&sup_b, &sub_b) {
        (Some(p), None) => {
            u = u.max(sup_min).max(p.d + X_HEIGHT * s / 4.0);
            out.place(p, x + sup_dx, u);
            out.w = x + sup_dx + p.w + SCRIPT_SPACE * s;
            out.h = out.h.max(u + p.h);
        }
        (None, Some(q)) => {
            v = v.max(SUB1 * s).max(q.h - 0.8 * X_HEIGHT * s);
            out.place(q, x + sub_dx, -v);
            out.w = x + (q.w + sub_dx).max(0.0) + SCRIPT_SPACE * s;
            out.d = out.d.max(v + q.d);
        }
        (Some(p), Some(q)) => {
            u = u.max(sup_min).max(p.d + X_HEIGHT * s / 4.0);
            v = v.max(SUB2 * s);
            let theta = RULE * s;
            let gap = (u - p.d) - (q.h - v);
            if gap < 4.0 * theta {
                v += 4.0 * theta - gap;
                let psi = 0.8 * X_HEIGHT * s - (u - p.d);
                if psi > 0.0 {
                    u += psi;
                    v -= psi;
                }
            }
            out.place(p, x + sup_dx, u);
            out.place(q, x + sub_dx, -v);
            out.w = x + (p.w + sup_dx).max(q.w + sub_dx) + SCRIPT_SPACE * s;
            out.h = out.h.max(u + p.h);
            out.d = out.d.max(v + q.d);
        }
        (None, None) => {}
    }
    out
}

fn layout_frac(
    num: &Node,
    den: &Node,
    style: Option<MStyle>,
    rule: bool,
    delims: Option<(char, char)>,
    ctx: Ctx,
) -> LBox {
    let st = style.unwrap_or(ctx.style);
    let ctx = ctx.with_style(st);
    let s = ctx.s();
    let inner = ctx.with_style(st.frac_num());
    let nb = layout_node(num, inner);
    let db = layout_node(
        den,
        Ctx {
            cramped: true,
            ..inner
        },
    );
    let display = st == MStyle::Display;
    let theta = if rule { RULE * s } else { 0.0 };
    let (mut u, mut v) = if display {
        (NUM1 * s, DENOM1 * s)
    } else {
        (NUM2 * s, DENOM2 * s)
    };
    let phi = if display { 3.0 * RULE * s } else { RULE * s };
    let axis = AXIS * s;
    if rule {
        let d1 = (u - nb.d) - (axis + theta / 2.0);
        if d1 < phi {
            u += phi - d1;
        }
        let d2 = (axis - theta / 2.0) - (db.h - v);
        if d2 < phi {
            v += phi - d2;
        }
    } else {
        let clr = if display {
            7.0 * RULE * s
        } else {
            3.0 * RULE * s
        };
        let gap = (u - nb.d) - (db.h - v);
        if gap < clr {
            u += (clr - gap) / 2.0;
            v += (clr - gap) / 2.0;
        }
    }
    let pad = NULL_DELIM * s;
    let w = nb.w.max(db.w);
    let mut out = LBox {
        w: w + 2.0 * pad,
        h: u + nb.h,
        d: v + db.d,
        items: vec![],
    };
    out.place(&nb, pad + (w - nb.w) / 2.0, u);
    out.place(&db, pad + (w - db.w) / 2.0, -v);
    if rule {
        let p = rect_path(
            pad * 0.5,
            axis - theta / 2.0,
            w + pad * 1.5,
            axis + theta / 2.0,
        );
        out.items.push(shape_item(p, &ctx));
    }
    if let Some((l, r)) = delims {
        let need = (out.h.max(out.d) * 2.0).max(1.2 * s);
        let lb = delimiter(l, need, ctx);
        let rb = delimiter(r, need, ctx);
        return hjoin(&[lb, out, rb]);
    }
    out
}

fn layout_sqrt(index: Option<&Node>, body: &Node, ctx: Ctx) -> LBox {
    let s = ctx.s();
    let b = layout_node(
        body,
        Ctx {
            cramped: true,
            ..ctx
        },
    );
    let theta = RULE * s;
    let phi = if ctx.style == MStyle::Display {
        X_HEIGHT * s
    } else {
        theta
    };
    let clr = theta + phi / 4.0;
    let top = b.h.max(0.6 * s) + clr + theta;
    let bottom = -(b.d.max(0.0) + 0.08 * s);
    let total = top - bottom;
    // radical geometry
    let hook_y = bottom + (0.5 * total).min(0.5 * s);
    let a = v2(0.02 * s, hook_y);
    let bk = v2(0.13 * s, hook_y + 0.07 * s);
    let vtx = v2(0.38 * s, bottom);
    let rw = 0.62 * s + 0.07 * total;
    let e = v2(rw, top - theta / 2.0);
    let mut p = Path::new();
    p.subpaths.push(thick_segment(a, bk, 0.045 * s, true));
    p.subpaths.push(thick_segment(bk, vtx, 0.095 * s, true));
    p.subpaths.push(thick_segment(vtx, e, theta, true));
    let x_end = rw + b.w + 0.08 * s;
    p.subpaths
        .extend(rect_path(rw - theta * 0.3, top - theta, x_end, top).subpaths);
    let mut out = LBox {
        w: x_end + 0.05 * s,
        h: top,
        d: -bottom,
        items: vec![],
    };
    out.items.push(shape_item(p, &ctx));
    out.place(&b, rw + 0.04 * s, 0.0);
    if let Some(ix) = index {
        let ib = layout_node(ix, ctx.with_style(MStyle::ScriptScript));
        let raise = bottom + 0.6 * total;
        let x_needed = ib.w - (bk.x + 0.12 * s);
        let shift = x_needed.max(0.0);
        let mut shifted = LBox {
            w: out.w + shift,
            h: out.h.max(raise + ib.h),
            d: out.d,
            items: vec![],
        };
        shifted.place(&out, shift, 0.0);
        shifted.place(&ib, (shift + bk.x + 0.12 * s - ib.w).max(0.0), raise);
        return shifted;
    }
    out
}

/// Extensible pieces (top, extension, middle, bottom) in the Size4 font.
fn ext_pieces(ch: char) -> Option<(char, char, Option<char>, char)> {
    Some(match ch {
        '(' => ('⎛', '⎜', None, '⎝'),
        ')' => ('⎞', '⎟', None, '⎠'),
        '[' => ('⎡', '⎢', None, '⎣'),
        ']' => ('⎤', '⎥', None, '⎦'),
        '{' => ('⎧', '⎪', Some('⎨'), '⎩'),
        '}' => ('⎫', '⎪', Some('⎬'), '⎭'),
        '⌈' => ('⎡', '⎢', None, '⎢'),
        '⌉' => ('⎤', '⎥', None, '⎥'),
        '⌊' => ('⎢', '⎢', None, '⎣'),
        '⌋' => ('⎥', '⎥', None, '⎦'),
        '|' | '∣' => ('∣', '∣', None, '∣'),
        '‖' | '∥' => ('∥', '∥', None, '∥'),
        _ => return None,
    })
}

fn delimiter(ch: char, need: f64, ctx: Ctx) -> LBox {
    let s = ctx.s();
    if ch == '.' {
        return LBox::hspace(NULL_DELIM * s);
    }
    let ch = match ch {
        '|' => '∣',
        '<' => '⟨',
        '>' => '⟩',
        c => c,
    };
    let center = |mut g: LBox| -> LBox {
        let shift = (g.h - g.d) / 2.0 - AXIS * s;
        for it in &mut g.items {
            it.y -= shift;
        }
        g.h -= shift;
        g.d += shift;
        g
    };
    for f in [
        MFont::Main,
        MFont::Size(1),
        MFont::Size(2),
        MFont::Size(3),
        MFont::Size(4),
    ] {
        if let Some(gid) = glyph_index(f.id(), ch) {
            let (h, d) = glyph_extent(f.id(), gid);
            if (h + d) * s >= need * 0.98 || matches!(f, MFont::Size(4)) && ext_pieces(ch).is_none()
            {
                let g = LBox {
                    w: advance(f.id(), gid) * s,
                    h: h * s,
                    d: d * s,
                    items: vec![Placed {
                        x: 0.0,
                        y: 0.0,
                        item: LItem::Glyph {
                            font: f.id(),
                            gid,
                            scale: s,
                            ch,
                        },
                        color: ctx.color,
                        part: ctx.part,
                    }],
                };
                return center(g);
            }
        }
    }
    // extensible stack
    let Some((top, ext, mid, bot)) = ext_pieces(ch) else {
        return glyph_box(ch, MFont::Main, &ctx);
    };
    let f4 = if matches!(ch, '|' | '∣' | '‖' | '∥') {
        MFont::Size(1)
    } else {
        MFont::Size(4)
    };
    let piece = |c: char| -> Option<(u16, f64, f64, f64)> {
        let gid = glyph_index(f4.id(), c).or_else(|| glyph_index(MFont::Main.id(), c))?;
        let (h, d) = glyph_extent(f4.id(), gid);
        Some((gid, h, d, advance(f4.id(), gid)))
    };
    let (Some(t), Some(e), Some(b)) = (piece(top), piece(ext), piece(bot)) else {
        return glyph_box(ch, MFont::Main, &ctx);
    };
    let m = mid.and_then(piece);
    let w = t.3.max(b.3).max(e.3) * s;
    let total = need.max((t.1 + t.2 + b.1 + b.2) * s);
    let y_top = AXIS * s + total / 2.0;
    let y_bot = AXIS * s - total / 2.0;
    let mut out = LBox {
        w,
        h: y_top,
        d: -y_bot,
        items: vec![],
    };
    let glyph = |gid: u16, x: f64, y: f64| Placed {
        x,
        y,
        item: LItem::Glyph {
            font: f4.id(),
            gid,
            scale: s,
            ch,
        },
        color: ctx.color,
        part: ctx.part,
    };
    // top piece: its top at y_top
    out.items.push(glyph(t.0, 0.0, y_top - t.1 * s));
    out.items.push(glyph(b.0, 0.0, y_bot + b.2 * s));
    let ext_h = (e.1 + e.2) * s;
    let fill = |out: &mut LBox, from: f64, to: f64| {
        if ext_h <= 1e-6 {
            return;
        }
        let mut y = from;
        while y < to {
            let yy = y.min(to - ext_h);
            out.items.push(glyph(e.0, 0.0, yy + e.2 * s));
            y += ext_h * 0.95;
        }
    };
    let top_bottom = y_top - (t.1 + t.2) * s;
    let bot_top = y_bot + (b.1 + b.2) * s;
    if let Some(mm) = m {
        let mh = (mm.1 + mm.2) * s;
        let mid_bot = AXIS * s - mh / 2.0;
        out.items.push(glyph(mm.0, 0.0, mid_bot + mm.2 * s));
        fill(&mut out, bot_top - 0.01, mid_bot + 0.01);
        fill(&mut out, mid_bot + mh - 0.01, top_bottom + 0.01);
    } else {
        fill(&mut out, bot_top - 0.01, top_bottom + 0.01);
    }
    out
}

fn arrow_shape(kind: ArrowKind, x0: f64, x1: f64, y: f64, s: f64) -> Path {
    let t = RULE * s * 1.1;
    let head = 0.16 * s;
    let mut p = Path::new();
    let mut add_arrow = |from: f64, to: f64, yy: f64, harpoon: Option<bool>| {
        let dir = if to > from { 1.0 } else { -1.0 };
        let tip = v2(to, yy);
        p.subpaths.push(thick_segment(
            v2(from, yy),
            tip - v2(dir * t * 0.5, 0.0),
            t,
            false,
        ));
        match harpoon {
            None => {
                p.subpaths.push(thick_segment(
                    tip,
                    tip + v2(-dir * head, head * 0.75),
                    t,
                    true,
                ));
                p.subpaths.push(thick_segment(
                    tip,
                    tip + v2(-dir * head, -head * 0.75),
                    t,
                    true,
                ));
            }
            Some(up) => {
                let sgn = if up { 1.0 } else { -1.0 };
                p.subpaths.push(thick_segment(
                    tip,
                    tip + v2(-dir * head, sgn * head * 0.75),
                    t,
                    true,
                ));
            }
        }
    };
    match kind {
        ArrowKind::Right => add_arrow(x0, x1, y, None),
        ArrowKind::Left => add_arrow(x1, x0, y, None),
        ArrowKind::LeftRight => {
            add_arrow(x0 + (x1 - x0) / 2.0, x1, y, None);
            add_arrow(x0 + (x1 - x0) / 2.0, x0, y, None);
        }
        ArrowKind::RightLeftHarpoons => {
            add_arrow(x0, x1, y + 0.09 * s, Some(true));
            add_arrow(x1, x0, y - 0.09 * s, Some(false));
        }
        ArrowKind::DoubleRight => {
            add_arrow(x0, x1 - 0.05 * s, y + 0.07 * s, Some(true));
            add_arrow(x0, x1 - 0.05 * s, y - 0.07 * s, Some(false));
        }
    }
    p
}

fn layout_accent(kind: AccentKind, body: &Node, ctx: Ctx) -> LBox {
    use AccentKind::*;
    let s = ctx.s();
    let b = layout_node(
        body,
        Ctx {
            cramped: true,
            ..ctx
        },
    );
    let italic_skew = match body {
        Node::Sym {
            font: MFont::MathItalic,
            ..
        } => 0.08 * s,
        _ => 0.0,
    };
    let mut out = LBox {
        w: b.w,
        h: b.h,
        d: b.d,
        items: vec![],
    };
    out.place(&b, 0.0, 0.0);
    let theta = RULE * s;
    match kind {
        Overline | Bar => {
            let (x0, x1) = if kind == Bar && is_single_char(body) {
                (b.w * 0.12 + italic_skew, b.w * 0.88 + italic_skew)
            } else {
                (0.0, b.w)
            };
            let y = b.h.max(X_HEIGHT * s) + 0.12 * s;
            out.items
                .push(shape_item(rect_path(x0, y, x1, y + theta), &ctx));
            out.h = y + theta + 0.04 * s;
        }
        Underline => {
            let y = -b.d - 0.12 * s;
            out.items
                .push(shape_item(rect_path(0.0, y - theta, b.w, y), &ctx));
            out.d = -(y - theta);
        }
        Vec | OverRightArrow | OverLeftArrow | OverLeftRightArrow => {
            let y = b.h.max(X_HEIGHT * s) + 0.15 * s;
            let (x0, x1) = if kind == Vec {
                let c = b.w / 2.0 + italic_skew;
                let half = (b.w * 0.45).clamp(0.22 * s, 0.6 * s);
                (c - half, c + half)
            } else {
                (0.0, b.w)
            };
            let k = match kind {
                OverLeftArrow => ArrowKind::Left,
                OverLeftRightArrow => ArrowKind::LeftRight,
                _ => ArrowKind::Right,
            };
            let p = arrow_shape(k, x0, x1, y, s * 0.8);
            out.items.push(shape_item(p, &ctx));
            out.h = y + 0.15 * s;
        }
        WideHat => {
            let y = b.h + 0.08 * s;
            let mut p = Path::new();
            let peak = v2(b.w / 2.0, y + 0.18 * s);
            p.subpaths
                .push(thick_segment(v2(0.0, y), peak, theta * 1.3, true));
            p.subpaths
                .push(thick_segment(peak, v2(b.w, y), theta * 1.3, true));
            out.items.push(shape_item(p, &ctx));
            out.h = y + 0.22 * s;
        }
        _ => {
            let ch = match kind {
                Hat => 'ˆ',
                Tilde => '˜',
                Dot => '˙',
                DDot => '¨',
                Acute => 'ˊ',
                Grave => 'ˋ',
                Breve => '˘',
                Check => 'ˇ',
                Ring => '˚',
                _ => 'ˆ',
            };
            let g = glyph_box(ch, MFont::Main, &ctx);
            let raise = (b.h - X_HEIGHT * s).max(0.0);
            let x = (b.w - g.w) / 2.0 + italic_skew;
            out.place(&g, x, raise);
            out.h = out.h.max(raise + g.h);
        }
    }
    out
}

fn layout_array(
    rows: &[Vec<Node>],
    align: &[char],
    col_gap: f64,
    left: char,
    right: char,
    display: bool,
    pair_align: bool,
    ctx: Ctx,
) -> LBox {
    let cctx = if display {
        ctx.with_style(MStyle::Display)
    } else if ctx.style == MStyle::Display {
        ctx.with_style(MStyle::Text)
    } else {
        ctx
    };
    let s = ctx.s();
    let cells: Vec<Vec<LBox>> = rows
        .iter()
        .map(|r| r.iter().map(|c| layout_node(c, cctx)).collect())
        .collect();
    let ncols = cells.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut colw = vec![0.0f64; ncols];
    for r in &cells {
        for (j, c) in r.iter().enumerate() {
            colw[j] = colw[j].max(c.w);
        }
    }
    let strut_h = 0.85 * s;
    let strut_d = 0.35 * s;
    let row_gap = if display { 0.3 * s } else { 0.12 * s };
    let gap_between = |j: usize| -> f64 {
        if pair_align {
            if j % 2 == 1 {
                0.0
            } else {
                1.5 * s
            }
        } else {
            col_gap * s
        }
    };
    let mut xs = vec![0.0; ncols];
    let mut x = 0.0;
    for j in 0..ncols {
        if j > 0 {
            x += gap_between(j);
        }
        xs[j] = x;
        x += colw[j];
    }
    let total_w = x;
    // vertical layout
    let mut row_hd: Vec<(f64, f64)> = cells
        .iter()
        .map(|r| {
            let h = r.iter().map(|c| c.h).fold(strut_h, f64::max);
            let d = r.iter().map(|c| c.d).fold(strut_d, f64::max);
            (h, d)
        })
        .collect();
    if row_hd.is_empty() {
        row_hd.push((strut_h, strut_d));
    }
    let total_h: f64 =
        row_hd.iter().map(|(h, d)| h + d).sum::<f64>() + row_gap * (row_hd.len() as f64 - 1.0);
    let top = AXIS * s + total_h / 2.0;
    let mut out = LBox {
        w: total_w,
        h: top,
        d: total_h - top,
        items: vec![],
    };
    let mut y = top;
    for (i, r) in cells.iter().enumerate() {
        let (h, d) = row_hd[i];
        let base = y - h;
        for (j, c) in r.iter().enumerate() {
            let a = align.get(j).copied().unwrap_or('c');
            let cx = match a {
                'l' => xs[j],
                'r' => xs[j] + colw[j] - c.w,
                _ => xs[j] + (colw[j] - c.w) / 2.0,
            };
            out.place(c, cx, base);
        }
        y = base - d - row_gap;
    }
    if left == '.' && right == '.' {
        return out;
    }
    let need = total_h + 0.2 * s;
    let lb = delimiter(left, need, ctx);
    let rb = delimiter(right, need, ctx);
    let pad = LBox::hspace(0.12 * s);
    hjoin(&[lb, pad.clone(), out, pad, rb])
}

// -------------------------------------------------------------- mobject build

/// Builder for typeset math.
#[derive(Clone, Debug)]
pub struct MathTex {
    pub src: String,
    pub size: f64,
    pub color: Color,
    pub display: bool,
    pub t2c: Vec<(String, Color)>,
}

impl MathTex {
    pub fn new(src: &str) -> Self {
        Self {
            src: src.to_string(),
            size: crate::text::DEFAULT_FONT_SIZE,
            color: WHITE,
            display: true,
            t2c: vec![],
        }
    }
    /// From explicit parts (each becomes one child, tagged with its TeX).
    pub fn parts(parts: &[&str]) -> Self {
        let src = parts
            .iter()
            .map(|p| format!("{{{{{p}}}}}"))
            .collect::<Vec<_>>()
            .join(" ");
        Self::new(&src)
    }
    pub fn size(mut self, s: f64) -> Self {
        self.size = s;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn inline(mut self) -> Self {
        self.display = false;
        self
    }
    /// Colour every occurrence of a TeX substring (manim's `tex_to_color_map`).
    pub fn t2c(mut self, sub: &str, c: Color) -> Self {
        self.t2c.push((sub.to_string(), c));
        self
    }

    pub fn build(&self) -> Mobject {
        let mut src = self.src.clone();
        for (sub, c) in &self.t2c {
            src = color_substring(&src, sub, *c);
        }
        let (node, parts) = parse(&src);
        let ctx = Ctx {
            style: if self.display {
                MStyle::Display
            } else {
                MStyle::Text
            },
            color: None,
            part: None,
            cramped: false,
        };
        let b = layout_node(&node, ctx);
        let size = self.size;
        let mut part_groups: Vec<Mobject> =
            parts.iter().map(|p| Mobject::empty().with_tag(p)).collect();
        let mut loose = Mobject::empty();
        for it in &b.items {
            let color = it.color.unwrap_or(self.color);
            let (mut path, tag) = match &it.item {
                LItem::Glyph {
                    font,
                    gid,
                    scale,
                    ch,
                } => {
                    let mut p = (*glyph_path(*font, *gid)).clone();
                    p.transform(&Affine::scale(*scale, *scale));
                    (p, ch.to_string())
                }
                LItem::Shape(p) => ((**p).clone(), "rule".to_string()),
            };
            if path.is_empty() {
                continue;
            }
            path.transform(
                &Affine::scale(size, size).then_after(Affine::translate(v2(it.x, it.y))),
            );
            let mut m = Mobject::from_path_styled(path, Style::filled(color));
            m.tag = Some(tag.into());
            match it.part {
                Some(i) if i < part_groups.len() => part_groups[i].push(m),
                _ => loose.push(m),
            }
        }
        let mut out = if parts.is_empty() {
            loose
        } else {
            if !loose.children.is_empty() {
                part_groups.push(loose);
            }
            Mobject::group(part_groups)
        };
        out.tag = Some(self.src.clone().into());
        let c = out.center();
        out.shift(-c)
    }
}

/// Wrap occurrences of `sub` (outside command names) in `\textcolor`.
fn color_substring(src: &str, sub: &str, c: Color) -> String {
    if sub.is_empty() {
        return src.to_string();
    }
    let chars: Vec<char> = src.chars().collect();
    let sc: Vec<char> = sub.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let hex = c.to_hex();
    while i < chars.len() {
        if i + sc.len() <= chars.len() && chars[i..i + sc.len()] == sc[..] {
            // inside a command name?
            let mut j = i;
            while j > 0 && chars[j - 1].is_ascii_alphabetic() {
                j -= 1;
            }
            let in_cmd = j > 0 && chars[j - 1] == '\\' && sc[0].is_ascii_alphabetic();
            let next_alpha = chars
                .get(i + sc.len())
                .is_some_and(|c| c.is_ascii_alphabetic());
            let starts_cmd = sc[0] == '\\';
            if !in_cmd && !(starts_cmd && next_alpha && sc.last().unwrap().is_ascii_alphabetic()) {
                out.push_str(&format!("\\textcolor{{{hex}}}{{{sub}}}"));
                i += sc.len();
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Typeset display math.
pub fn tex(src: &str) -> Mobject {
    MathTex::new(src).build()
}

pub fn tex_sized(src: &str, size: f64, color: Color) -> Mobject {
    MathTex::new(src).size(size).color(color).build()
}

/// Math built from parts; children are the parts (manim's `MathTex("a", "+", "b")`).
pub fn tex_parts(parts: &[&str]) -> Mobject {
    MathTex::parts(parts).build()
}

/// Text with inline `$math$` (manim's `Tex`). Rendered in Computer Modern.
pub fn tex_text(src: &str) -> Mobject {
    let mut tex = String::new();
    for (i, seg) in src.split('$').enumerate() {
        if seg.is_empty() {
            continue;
        }
        if i % 2 == 0 {
            let esc = seg.replace('{', "\\{").replace('}', "\\}");
            tex.push_str(&format!("\\text{{{esc}}}"));
        } else {
            tex.push_str(seg);
        }
    }
    MathTex::new(&tex).inline().build()
}

/// Chemical formula / equation (mhchem-style syntax): `chem("2H2 + O2 -> 2H2O")`.
pub fn chem(src: &str) -> Mobject {
    MathTex::new(&chem_to_tex(src)).build()
}

/// Convert mhchem-like notation to TeX.
pub fn chem_to_tex(src: &str) -> String {
    let mut out = String::new();
    for (wi, word) in src.split_whitespace().enumerate() {
        if wi > 0 {
            out.push(' ');
        }
        // arrows / operators
        if let Some(rest) = word.strip_prefix("->") {
            if let Some(label) = rest.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                out.push_str(&format!("\\xrightarrow{{\\text{{{label}}}}}"));
            } else {
                out.push_str("\\longrightarrow");
            }
            continue;
        }
        match word {
            "+" => {
                out.push('+');
                continue;
            }
            "<=>" => {
                out.push_str("\\xrightleftharpoons{}");
                continue;
            }
            "<->" => {
                out.push_str("\\leftrightarrow");
                continue;
            }
            "<-" => {
                out.push_str("\\longleftarrow");
                continue;
            }
            "^" => {
                out.push_str("\\uparrow");
                continue;
            }
            "v" => {
                out.push_str("\\downarrow");
                continue;
            }
            "=" => {
                out.push('=');
                continue;
            }
            _ => {}
        }
        out.push_str(&species_to_tex(word));
    }
    out
}

fn species_to_tex(w: &str) -> String {
    let c: Vec<char> = w.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    // leading stoichiometric coefficient
    while i < c.len() && (c[i].is_ascii_digit() || c[i] == '/') {
        out.push(c[i]);
        i += 1;
    }
    if i > 0 {
        out.push_str("\\,");
    }
    let mut prev_atomlike = false;
    while i < c.len() {
        let ch = c[i];
        if ch.is_ascii_uppercase() {
            let mut sym = ch.to_string();
            i += 1;
            while i < c.len() && c[i].is_ascii_lowercase() {
                sym.push(c[i]);
                i += 1;
            }
            out.push_str(&format!("\\mathrm{{{sym}}}"));
            prev_atomlike = true;
        } else if ch.is_ascii_digit() && prev_atomlike {
            let mut n = String::new();
            while i < c.len() && c[i].is_ascii_digit() {
                n.push(c[i]);
                i += 1;
            }
            // trailing charge like SO4^2- handled by '^'; "Na+" style handled below
            out.push_str(&format!("_{{{n}}}"));
        } else if ch == '^' {
            i += 1;
            let mut ch_s = String::new();
            if i < c.len() && c[i] == '{' {
                i += 1;
                while i < c.len() && c[i] != '}' {
                    ch_s.push(c[i]);
                    i += 1;
                }
                i += 1;
            } else {
                while i < c.len() && (c[i].is_ascii_digit() || c[i] == '+' || c[i] == '-') {
                    ch_s.push(c[i]);
                    i += 1;
                }
            }
            out.push_str(&format!("^{{{}}}", ch_s));
            prev_atomlike = false;
        } else if (ch == '+' || ch == '-') && i == c.len() - 1 && prev_atomlike {
            out.push_str(&format!("^{{{ch}}}"));
            i += 1;
        } else if ch == '(' {
            // state symbols
            let rest: String = c[i..].iter().collect();
            if let Some(st) = ["(aq)", "(s)", "(l)", "(g)"]
                .iter()
                .find(|s| rest.starts_with(**s))
            {
                out.push_str(&format!("\\,\\mathrm{{{st}}}"));
                i += st.chars().count();
                prev_atomlike = false;
            } else {
                out.push('(');
                i += 1;
                prev_atomlike = false;
            }
        } else if ch == ')' || ch == ']' {
            out.push(ch);
            i += 1;
            prev_atomlike = true;
        } else if ch == '.' || ch == '*' {
            out.push_str("\\cdot ");
            i += 1;
            prev_atomlike = false;
        } else {
            out.push(ch);
            i += 1;
            prev_atomlike = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_lays_out() {
        for src in [
            "E = mc^2",
            "\\frac{a}{b} + \\sqrt{x^2 + y^2}",
            "\\int_0^\\infty e^{-x^2}\\,dx = \\frac{\\sqrt{\\pi}}{2}",
            "\\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6}",
            "\\left( \\frac{a}{b} \\right)^2",
            "\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}",
            "f(x) = \\begin{cases} x & x > 0 \\\\ -x & x \\le 0 \\end{cases}",
            "\\vec{F} = m\\vec{a}",
            "\\hat{x}, \\bar{y}, \\dot{z}, \\ddot{\\theta}",
            "\\lim_{x \\to 0} \\frac{\\sin x}{x} = 1",
            "\\overbrace{a + b}^{n} \\underbrace{c}_{m}",
            "\\xrightarrow[\\Delta]{\\text{heat}}",
            "\\mathbb{R}^n \\mathcal{L} \\mathbf{v}",
            "\\textcolor{red}{x} + \\color{blue} y",
            "a \\\\ b + c",
            "x &= 1 \\\\ y &= 2",
            "\\sqrt[3]{8} = 2",
            "\\boxed{F = ma}",
            "\\ce{2H2 + O2 -> 2H2O}",
        ] {
            let m = tex(src);
            assert!(m.leaf_count() > 0, "{src}");
            assert!(m.width() > 0.0, "{src}");
        }
    }

    #[test]
    fn parts_and_colors() {
        let m = tex_parts(&["a^2", "+", "b^2", "=", "c^2"]);
        assert_eq!(m.children.len(), 5);
        assert_eq!(m.children[0].tag(), Some("a^2"));
        let c = MathTex::new("x + y").t2c("x", RED).build();
        assert_eq!(c.children[0].fill_color(), RED);
        assert!(chem_to_tex("SO4^2-").contains("^{2-}"));
        assert!(chem_to_tex("H2O(l)").contains("_{2}"));
    }

    #[test]
    fn frac_is_taller_than_inline() {
        let f = tex("\\frac{1}{2}");
        let x = tex("1");
        assert!(f.height() > x.height() * 1.5);
    }
}
