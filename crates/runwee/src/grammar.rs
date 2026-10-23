//! Grammar visuals (phase T4).
//!
//! Small, dependable grammar helpers for Class 6–9 lessons, each paired with a
//! picture:
//!
//! * Parts of speech: [`Pos`], a lexicon-and-suffix tagger ([`tag_sentence`]),
//!   colour chips ([`pos_chips`]) and a legend ([`pos_legend`]).
//! * Structure: phrase-structure trees ([`parse_tree`], [`TreeLayout`],
//!   [`draw_tree`]) and Reed–Kellogg sentence diagrams ([`diagram_parts`],
//!   [`reed_kellogg`]).
//! * Verbs: the twelve [`Tense`]s, [`conjugate`] (with irregular verbs) and
//!   [`tense_timeline`] pictures.
//! * Transformations: [`passive_voice`], [`report_speech`] (backshift,
//!   questions and commands) and [`word_morph`], which flies shared words to
//!   their new places.
//! * Agreement: [`subject_number`], [`verb_agrees`], [`agree_verb`] and
//!   [`sva_check`].
//! * Scenes: [`article_for`] with its reason, [`article_card`] and
//!   [`preposition_scene`].
//! * Practice: [`punctuation_drop`], [`quiz_card`] and [`fill_blank`].
//!
//! The analysis is rule-based and aimed at the textbook sentences learners
//! meet; it is not a general parser.
use crate::color::*;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::path::Rect;
use crate::shapes::*;
use crate::story::{ease_out_back, font_text, measure};

fn clean(w: &str) -> String {
    w.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'' || *c == '-')
        .collect::<String>()
        .to_lowercase()
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

// ------------------------------------------------------------ parts of speech

/// A part of speech.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pos {
    Noun,
    Pronoun,
    Verb,
    Auxiliary,
    Adjective,
    Adverb,
    Preposition,
    Conjunction,
    Determiner,
    Interjection,
    Punctuation,
}

impl Pos {
    pub const ALL: [Pos; 10] = [
        Pos::Noun,
        Pos::Pronoun,
        Pos::Verb,
        Pos::Auxiliary,
        Pos::Adjective,
        Pos::Adverb,
        Pos::Preposition,
        Pos::Conjunction,
        Pos::Determiner,
        Pos::Interjection,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Pos::Noun => "noun",
            Pos::Pronoun => "pronoun",
            Pos::Verb => "verb",
            Pos::Auxiliary => "helping verb",
            Pos::Adjective => "adjective",
            Pos::Adverb => "adverb",
            Pos::Preposition => "preposition",
            Pos::Conjunction => "conjunction",
            Pos::Determiner => "article / determiner",
            Pos::Interjection => "interjection",
            Pos::Punctuation => "punctuation",
        }
    }
    /// Short tree label.
    pub fn short(self) -> &'static str {
        match self {
            Pos::Noun => "N",
            Pos::Pronoun => "Pron",
            Pos::Verb => "V",
            Pos::Auxiliary => "Aux",
            Pos::Adjective => "Adj",
            Pos::Adverb => "Adv",
            Pos::Preposition => "P",
            Pos::Conjunction => "Conj",
            Pos::Determiner => "Det",
            Pos::Interjection => "Intj",
            Pos::Punctuation => ".",
        }
    }
    pub fn color(self) -> Color {
        match self {
            Pos::Noun => rgb8(52, 120, 200),
            Pos::Pronoun => rgb8(96, 160, 220),
            Pos::Verb => rgb8(214, 72, 64),
            Pos::Auxiliary => rgb8(232, 128, 110),
            Pos::Adjective => rgb8(60, 160, 90),
            Pos::Adverb => rgb8(150, 90, 190),
            Pos::Preposition => rgb8(230, 150, 40),
            Pos::Conjunction => rgb8(200, 90, 150),
            Pos::Determiner => rgb8(120, 130, 140),
            Pos::Interjection => rgb8(220, 180, 30),
            Pos::Punctuation => rgb8(90, 90, 90),
        }
    }
}

const PRONOUNS: &[&str] = &[
    "i",
    "me",
    "you",
    "he",
    "him",
    "she",
    "her",
    "it",
    "we",
    "us",
    "they",
    "them",
    "myself",
    "yourself",
    "himself",
    "herself",
    "itself",
    "ourselves",
    "themselves",
    "who",
    "whom",
    "someone",
    "everyone",
    "nobody",
    "something",
    "everything",
    "nothing",
    "mine",
    "yours",
    "hers",
    "ours",
    "theirs",
];
const DETERMINERS: &[&str] = &[
    "a", "an", "the", "this", "that", "these", "those", "my", "your", "his", "its", "our", "their",
    "some", "any", "each", "every", "no", "many", "much", "few", "several", "all", "both",
    "either", "neither", "one", "two", "three",
];
const PREPOSITIONS: &[&str] = &[
    "in", "on", "at", "under", "over", "above", "below", "beside", "between", "behind", "into",
    "onto", "through", "across", "along", "around", "near", "with", "without", "from", "to", "of",
    "for", "by", "about", "after", "before", "during", "up", "down", "off", "towards", "toward",
    "upon", "among", "beneath", "inside", "outside", "past", "till", "until", "since", "against",
    "like",
];
const CONJUNCTIONS: &[&str] = &[
    "and", "but", "or", "nor", "so", "yet", "because", "although", "though", "while", "when", "if",
    "unless", "whereas", "than", "whether",
];
const AUXILIARIES: &[&str] = &[
    "am", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does",
    "did", "will", "would", "shall", "should", "can", "could", "may", "might", "must",
];
const INTERJECTIONS: &[&str] = &[
    "oh", "wow", "alas", "hurray", "hurrah", "ouch", "oops", "hello", "hey", "ah", "bravo",
];
const ADVERBS: &[&str] = &[
    "very",
    "too",
    "soon",
    "now",
    "then",
    "here",
    "there",
    "always",
    "never",
    "often",
    "sometimes",
    "seldom",
    "again",
    "already",
    "still",
    "just",
    "almost",
    "quite",
    "today",
    "tomorrow",
    "yesterday",
    "not",
    "fast",
    "hard",
    "well",
    "away",
    "together",
    "once",
    "everywhere",
];
const ADJECTIVES: &[&str] = &[
    "good", "bad", "big", "small", "little", "old", "new", "young", "tall", "short", "long",
    "clever", "brave", "kind", "happy", "sad", "thirsty", "hungry", "red", "blue", "green",
    "yellow", "white", "black", "golden", "bright", "dark", "cold", "hot", "warm", "quiet", "loud",
    "wise", "poor", "rich", "great", "high", "low", "deep", "sweet", "proud", "lazy", "busy",
    "gentle", "huge", "tiny", "wet", "dry", "empty", "full", "soft", "late", "early", "ready",
    "honest", "tired",
];
const VERBS: &[&str] = &[
    "go", "went", "come", "came", "see", "saw", "find", "found", "drop", "dropped", "eat", "ate",
    "drink", "drank", "run", "ran", "sing", "sang", "write", "wrote", "read", "play", "played",
    "make", "made", "take", "took", "give", "gave", "say", "said", "tell", "told", "look",
    "looked", "want", "like", "love", "live", "know", "knew", "think", "thought", "fly", "flew",
    "sit", "sat", "stand", "stood", "swim", "swam", "climb", "kick", "kicked", "rise", "rose",
    "fall", "fell", "bring", "brought", "build", "built", "teach", "taught", "catch", "caught",
    "keep", "kept", "carry", "open", "close", "help", "plant", "planted", "painted", "paint",
    "jump", "jumped", "walk", "walked", "bark", "barks", "shine", "shines", "dance", "danced",
    "laugh", "laughed", "cry", "cried", "lives", "sings",
];

/// Tags one word given the previous tag.
pub fn tag_word(word: &str, prev: Option<Pos>) -> Pos {
    if !word.chars().any(|c| c.is_alphanumeric()) {
        return Pos::Punctuation;
    }
    let w = clean(word);
    let w = w.as_str();
    let after_det = matches!(prev, Some(Pos::Determiner) | Some(Pos::Adjective));
    if w == "her" && matches!(prev, Some(Pos::Verb) | Some(Pos::Preposition)) {
        return Pos::Pronoun;
    }
    if DETERMINERS.contains(&w) && !(w == "that" && matches!(prev, Some(Pos::Verb))) {
        return Pos::Determiner;
    }
    if PRONOUNS.contains(&w) {
        return Pos::Pronoun;
    }
    if AUXILIARIES.contains(&w) {
        return Pos::Auxiliary;
    }
    if PREPOSITIONS.contains(&w) && !after_det {
        return Pos::Preposition;
    }
    if CONJUNCTIONS.contains(&w) {
        return Pos::Conjunction;
    }
    if INTERJECTIONS.contains(&w) && prev.is_none() {
        return Pos::Interjection;
    }
    if ADVERBS.contains(&w) {
        return Pos::Adverb;
    }
    if ADJECTIVES.contains(&w) {
        return Pos::Adjective;
    }
    if VERBS.contains(&w) && !after_det {
        return Pos::Verb;
    }
    if matches!(prev, Some(Pos::Auxiliary) | Some(Pos::Pronoun)) && !w.ends_with("ly") {
        return Pos::Verb;
    }
    if matches!(w, "lovely" | "ugly" | "friendly" | "lonely") {
        return Pos::Adjective;
    }
    if w.ends_with("ly") && w.len() > 4 && !matches!(w, "family" | "holy" | "jelly" | "lily") {
        return Pos::Adverb;
    }
    for suf in [
        "ous", "ful", "able", "ible", "ive", "less", "ish", "ic", "al", "est",
    ] {
        if w.ends_with(suf) && w.len() > suf.len() + 2 {
            return Pos::Adjective;
        }
    }
    for suf in [
        "tion", "sion", "ness", "ment", "ship", "hood", "ity", "er", "or",
    ] {
        if w.ends_with(suf) && w.len() > suf.len() + 2 {
            return Pos::Noun;
        }
    }
    if !after_det
        && (w.ends_with("ed") || w.ends_with("ing"))
        && w.len() > 4
        && matches!(
            prev,
            Some(Pos::Noun) | Some(Pos::Pronoun) | Some(Pos::Auxiliary) | Some(Pos::Adverb) | None
        )
    {
        return Pos::Verb;
    }
    if matches!(prev, Some(Pos::Pronoun)) {
        return Pos::Verb;
    }
    if matches!(prev, Some(Pos::Noun)) && w.ends_with('s') && !w.ends_with("ss") {
        return Pos::Verb;
    }
    Pos::Noun
}

/// Splits a sentence into words and punctuation and tags each one.
pub fn tag_sentence(s: &str) -> Vec<(String, Pos)> {
    let mut out: Vec<(String, Pos)> = Vec::new();
    let mut prev: Option<Pos> = None;
    for raw in s.split_whitespace() {
        let lead: String = raw.chars().take_while(|c| !c.is_alphanumeric()).collect();
        let rest = &raw[lead.len()..];
        let core_len = rest.trim_end_matches(|c: char| !c.is_alphanumeric()).len();
        let (core, trail) = rest.split_at(core_len);
        for c in lead.chars() {
            out.push((c.to_string(), Pos::Punctuation));
        }
        if !core.is_empty() {
            let p = tag_word(core, prev);
            prev = Some(p);
            out.push((core.to_string(), p));
        }
        for c in trail.chars() {
            out.push((c.to_string(), Pos::Punctuation));
        }
    }
    out
}

/// Colour chips for a tagged sentence, centred on `center`. `p` pops chips in
/// one after another; with `labels` the short tag sits under each chip.
pub fn pos_chips(
    tagged: &[(String, Pos)],
    center: Vec2,
    size: f64,
    p: f64,
    labels: bool,
) -> Mobject {
    let gap = size * 0.28;
    let widths: Vec<f64> = tagged
        .iter()
        .map(|(w, pos)| {
            if *pos == Pos::Punctuation {
                measure(w, "sans", size) + 0.05
            } else {
                measure(w, "sans", size) + size * 0.6
            }
        })
        .collect();
    let total: f64 = widths.iter().sum::<f64>() + gap * (tagged.len().saturating_sub(1)) as f64;
    let mut x = center.x - total * 0.5;
    let n = tagged.len().max(1) as f64;
    let mut parts = Vec::new();
    for (i, ((w, pos), cw)) in tagged.iter().zip(&widths).enumerate() {
        let c = v2(x + cw * 0.5, center.y);
        x += cw + gap;
        let local = clamp01(p * (n + 2.0) - i as f64);
        if local <= 0.0 {
            continue;
        }
        let e = ease_out_back(local).max(0.01);
        let col = pos.color();
        let mut chip = Vec::new();
        if *pos == Pos::Punctuation {
            chip.push(font_text(w, "sans", size, rgb8(60, 60, 60)).move_to(c));
        } else {
            chip.push(
                rounded_rectangle(*cw, size * 1.5, size * 0.35)
                    .move_to(c)
                    .fill(col.mix(WHITE, 0.82), 1.0)
                    .stroke(col, 2.2),
            );
            chip.push(font_text(w, "sans", size, col.darker(0.35)).move_to(c));
            if labels {
                chip.push(
                    font_text(pos.short(), "sans", size * 0.5, col)
                        .move_to(c + v2(0.0, -size * 1.25)),
                );
            }
        }
        parts.push(group(chip).scale_about(e, c));
    }
    group(parts)
}

/// A two-row legend of the part-of-speech colours.
pub fn pos_legend(center: Vec2, size: f64, p: f64) -> Mobject {
    let mut parts = Vec::new();
    let cols = 5;
    for (i, pos) in Pos::ALL.iter().enumerate() {
        let local = clamp01(p * 12.0 - i as f64);
        if local <= 0.0 {
            continue;
        }
        let r = i / cols;
        let c = i % cols;
        let at = center + v2((c as f64 - 2.0) * size * 9.0, -(r as f64) * size * 1.7);
        parts.push(
            group(vec![
                circle(size * 0.32)
                    .move_to(at - v2(size * 3.2, 0.0))
                    .fill(pos.color(), 1.0)
                    .no_stroke(),
                font_text(pos.name(), "sans", size * 0.8, rgb8(60, 56, 52))
                    .move_to_aligned(at - v2(size * 2.6, 0.0), v2(-1.0, 0.0)),
            ])
            .opacity(local as f32),
        );
    }
    group(parts)
}

// ----------------------------------------------------------------- parse tree

/// A phrase-structure tree node.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub label: String,
    pub word: Option<String>,
    pub pos: Option<Pos>,
    pub children: Vec<Tree>,
}

impl Tree {
    fn leaf(w: &str, p: Pos) -> Tree {
        Tree {
            label: p.short().to_string(),
            word: Some(w.to_string()),
            pos: Some(p),
            children: vec![],
        }
    }
    fn node(l: &str, children: Vec<Tree>) -> Tree {
        Tree {
            label: l.to_string(),
            word: None,
            pos: None,
            children,
        }
    }
    pub fn depth(&self) -> usize {
        1 + self.children.iter().map(|c| c.depth()).max().unwrap_or(0)
    }
    /// The words at the leaves, left to right.
    pub fn words(&self) -> Vec<String> {
        if let Some(w) = &self.word {
            return vec![w.clone()];
        }
        self.children.iter().flat_map(|c| c.words()).collect()
    }
    /// Bracketed form, e.g. `[S [NP [Det The] [N crow]] [VP [V flew]]]`.
    pub fn bracket(&self) -> String {
        match &self.word {
            Some(w) => format!("[{} {}]", self.label, w),
            None => format!(
                "[{} {}]",
                self.label,
                self.children
                    .iter()
                    .map(|c| c.bracket())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        }
    }
}

type Tagged = [(String, Pos)];

fn parse_np(t: &Tagged, i: &mut usize) -> Option<Tree> {
    let start = *i;
    let mut kids = Vec::new();
    if *i < t.len() && t[*i].1 == Pos::Pronoun {
        kids.push(Tree::leaf(&t[*i].0, Pos::Pronoun));
        *i += 1;
        return Some(Tree::node("NP", kids));
    }
    if *i < t.len() && t[*i].1 == Pos::Determiner {
        kids.push(Tree::leaf(&t[*i].0, Pos::Determiner));
        *i += 1;
    }
    while *i + 1 < t.len() && t[*i].1 == Pos::Adjective {
        kids.push(Tree::leaf(&t[*i].0, t[*i].1));
        *i += 1;
    }
    let mut got_noun = false;
    while *i < t.len() && t[*i].1 == Pos::Noun {
        kids.push(Tree::leaf(&t[*i].0, Pos::Noun));
        *i += 1;
        got_noun = true;
    }
    if !got_noun {
        *i = start;
        return None;
    }
    let mut np = Tree::node("NP", kids);
    while *i + 1 < t.len() && t[*i].1 == Pos::Preposition && t[*i].0.eq_ignore_ascii_case("of") {
        let mut j = *i + 1;
        match parse_np(t, &mut j) {
            Some(obj) => {
                let pp = Tree::node("PP", vec![Tree::leaf(&t[*i].0, Pos::Preposition), obj]);
                np = Tree::node("NP", vec![np, pp]);
                *i = j;
            }
            None => break,
        }
    }
    Some(np)
}

fn parse_pp(t: &Tagged, i: &mut usize) -> Option<Tree> {
    if *i < t.len() && t[*i].1 == Pos::Preposition {
        let mut j = *i + 1;
        if let Some(np) = parse_np(t, &mut j) {
            let pp = Tree::node("PP", vec![Tree::leaf(&t[*i].0, Pos::Preposition), np]);
            *i = j;
            return Some(pp);
        }
    }
    None
}

fn vp_word(w: &str, p: Pos) -> Tree {
    if p == Pos::Adverb {
        Tree::node("AdvP", vec![Tree::leaf(w, p)])
    } else {
        Tree::leaf(w, p)
    }
}

/// Builds a phrase-structure tree: S → NP VP, VP → (Aux) (Adv) V (NP) (PP)*
/// (Adv)*. Leftover words are attached to the VP.
pub fn parse_tree(sentence: &str) -> Tree {
    let tagged: Vec<(String, Pos)> = tag_sentence(sentence)
        .into_iter()
        .filter(|(_, p)| *p != Pos::Punctuation)
        .collect();
    let mut i = 0;
    let mut s_kids = Vec::new();
    if let Some(np) = parse_np(&tagged, &mut i) {
        s_kids.push(np);
    }
    let mut vp = Vec::new();
    while i < tagged.len() && matches!(tagged[i].1, Pos::Auxiliary | Pos::Adverb) {
        vp.push(vp_word(&tagged[i].0, tagged[i].1));
        i += 1;
    }
    if i < tagged.len() && tagged[i].1 == Pos::Verb {
        vp.push(Tree::leaf(&tagged[i].0, Pos::Verb));
        i += 1;
    }
    while i < tagged.len() {
        if let Some(np) = parse_np(&tagged, &mut i) {
            vp.push(np);
        } else if let Some(pp) = parse_pp(&tagged, &mut i) {
            vp.push(pp);
        } else {
            vp.push(vp_word(&tagged[i].0, tagged[i].1));
            i += 1;
        }
    }
    if !vp.is_empty() {
        s_kids.push(Tree::node("VP", vp));
    }
    Tree::node("S", s_kids)
}

/// One placed node of a [`TreeLayout`].
#[derive(Clone, Debug)]
pub struct LNode {
    pub label: String,
    pub word: Option<String>,
    pub pos: Option<Pos>,
    pub at: Vec2,
    pub parent: Option<usize>,
    pub depth: usize,
}

/// A tree laid out top-down with the words along the bottom.
#[derive(Clone, Debug)]
pub struct TreeLayout {
    pub nodes: Vec<LNode>,
    pub levels: usize,
    pub word_y: f64,
    pub size: f64,
}

struct Placer<'a> {
    xs: &'a [f64],
    leaf_i: usize,
    top: Vec2,
    level_h: f64,
    levels: usize,
    nodes: Vec<LNode>,
}

impl Placer<'_> {
    fn walk(&mut self, t: &Tree, depth: usize, parent: Option<usize>) -> f64 {
        let idx = self.nodes.len();
        self.nodes.push(LNode {
            label: t.label.clone(),
            word: t.word.clone(),
            pos: t.pos,
            at: Vec2::ZERO,
            parent,
            depth,
        });
        let x = if t.word.is_some() {
            let x = self.xs[self.leaf_i];
            self.leaf_i += 1;
            x
        } else {
            let cx: Vec<f64> = t
                .children
                .iter()
                .map(|c| self.walk(c, depth + 1, Some(idx)))
                .collect();
            cx.iter().sum::<f64>() / cx.len().max(1) as f64
        };
        // part-of-speech labels share the last row, just above the words
        let row = if t.word.is_some() {
            self.levels - 1
        } else {
            depth
        };
        self.nodes[idx].at = v2(x, self.top.y - row as f64 * self.level_h);
        x
    }
}

impl TreeLayout {
    /// Lays `tree` out with its root at `top` and level spacing `level_h`.
    pub fn new(tree: &Tree, top: Vec2, level_h: f64, size: f64) -> TreeLayout {
        let words = tree.words();
        let gap = size * 0.9;
        let widths: Vec<f64> = words
            .iter()
            .map(|w| measure(w, "story", size * 1.05).max(size * 1.6))
            .collect();
        let total: f64 = widths.iter().sum::<f64>() + gap * (words.len().saturating_sub(1)) as f64;
        let mut xs = Vec::new();
        let mut x = top.x - total * 0.5;
        for w in &widths {
            xs.push(x + w * 0.5);
            x += w + gap;
        }
        let levels = tree.depth();
        let mut pl = Placer {
            xs: &xs,
            leaf_i: 0,
            top,
            level_h,
            levels,
            nodes: Vec::new(),
        };
        pl.walk(tree, 0, None);
        let word_y = top.y - (levels as f64 - 1.0) * level_h - level_h * 0.8;
        TreeLayout {
            nodes: pl.nodes,
            levels,
            word_y,
            size,
        }
    }
}

fn phrase_color(label: &str) -> Color {
    match label {
        "S" => rgb8(70, 60, 80),
        "NP" => Pos::Noun.color(),
        "VP" => Pos::Verb.color(),
        "PP" => Pos::Preposition.color(),
        "AdvP" => Pos::Adverb.color(),
        _ => rgb8(110, 110, 110),
    }
}

/// Draws a laid-out tree growing from the root (`p` 0→1): each level's
/// branches extend and then its labels pop in; the words drop in last.
pub fn draw_tree(lay: &TreeLayout, p: f64) -> Mobject {
    let levels = lay.levels as f64 + 1.0;
    let s = lay.size;
    let mut lines = Vec::new();
    let mut labels = Vec::new();
    let word_p = clamp01((p - (lay.levels as f64) / levels) * levels);
    for n in &lay.nodes {
        let local = clamp01((p - n.depth as f64 / levels) * levels);
        if local <= 0.0 {
            continue;
        }
        let col = n
            .pos
            .map(|q| q.color())
            .unwrap_or_else(|| phrase_color(&n.label));
        if let Some(pi) = n.parent {
            let a = lay.nodes[pi].at - v2(0.0, s * 0.42);
            let b = n.at + v2(0.0, s * 0.42);
            let grow = clamp01(local / 0.6);
            lines.push(line(a, a.lerp(b, grow.max(0.001))).stroke(rgb8(150, 140, 130), 2.0));
        }
        let lp = clamp01((local - 0.45) / 0.55);
        if lp <= 0.0 {
            continue;
        }
        let e = ease_out_back(lp).max(0.01);
        let w = measure(&n.label, "sans", s * 0.6) + s * 0.7;
        labels.push(
            group(vec![
                rounded_rectangle(w, s * 0.85, s * 0.3)
                    .move_to(n.at)
                    .fill(col, 1.0)
                    .no_stroke(),
                font_text(&n.label, "sans", s * 0.6, WHITE).move_to(n.at),
            ])
            .scale_about(e, n.at),
        );
        if let (Some(w), Some(pos), true) = (&n.word, n.pos, word_p > 0.0) {
            let at = v2(n.at.x, lay.word_y);
            lines.push(
                dashed_line(n.at - v2(0.0, s * 0.45), at + v2(0.0, s * 0.5))
                    .stroke(pos.color().mix(WHITE, 0.4), 1.5),
            );
            labels.push(
                font_text(w, "story", s * 1.05, pos.color().darker(0.3))
                    .move_to(at + v2(0.0, 0.3 * (1.0 - ease_out_back(word_p))))
                    .opacity(word_p as f32),
            );
        }
    }
    group(vec![group(lines), group(labels)])
}

// ------------------------------------------------------- Reed–Kellogg diagram

/// The pieces of a simple clause for a Reed–Kellogg diagram.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiagramParts {
    pub subject: String,
    pub subject_mods: Vec<String>,
    pub verb: String,
    pub verb_mods: Vec<String>,
    pub object: Option<String>,
    pub object_mods: Vec<String>,
    /// (preposition, object, object's modifiers) hanging under the verb.
    pub preps: Vec<(String, String, Vec<String>)>,
}

fn diagram_np(t: &Tagged, i: &mut usize) -> (String, Vec<String>) {
    let mut mods = Vec::new();
    let mut head = String::new();
    while *i < t.len() {
        match t[*i].1 {
            Pos::Determiner | Pos::Adjective => mods.push(t[*i].0.clone()),
            Pos::Noun | Pos::Pronoun => {
                if !head.is_empty() {
                    mods.push(head.clone());
                }
                head = t[*i].0.clone();
                if t[*i].1 == Pos::Pronoun
                    || t.get(*i + 1).map(|x| x.1 != Pos::Noun).unwrap_or(true)
                {
                    *i += 1;
                    break;
                }
            }
            _ => break,
        }
        *i += 1;
    }
    (head, mods)
}

/// Splits a simple sentence into diagram parts using the tagger.
pub fn diagram_parts(sentence: &str) -> DiagramParts {
    let t: Vec<(String, Pos)> = tag_sentence(sentence)
        .into_iter()
        .filter(|(_, p)| *p != Pos::Punctuation)
        .collect();
    let mut d = DiagramParts::default();
    let mut i = 0;
    let (s, sm) = diagram_np(&t, &mut i);
    d.subject = s;
    d.subject_mods = sm;
    let mut verb = Vec::new();
    while i < t.len() && matches!(t[i].1, Pos::Auxiliary | Pos::Verb | Pos::Adverb) {
        if t[i].1 == Pos::Adverb {
            d.verb_mods.push(t[i].0.clone());
        } else {
            verb.push(t[i].0.clone());
        }
        i += 1;
    }
    d.verb = verb.join(" ");
    while i < t.len() {
        match t[i].1 {
            Pos::Preposition => {
                let p = t[i].0.clone();
                i += 1;
                let (o, om) = diagram_np(&t, &mut i);
                d.preps.push((p, o, om));
            }
            Pos::Adverb => {
                d.verb_mods.push(t[i].0.clone());
                i += 1;
            }
            Pos::Determiner | Pos::Adjective | Pos::Noun | Pos::Pronoun if d.object.is_none() => {
                let (o, om) = diagram_np(&t, &mut i);
                d.object = Some(o);
                d.object_mods = om;
            }
            _ => i += 1,
        }
    }
    d
}

/// Draws a Reed–Kellogg diagram with its baseline starting at `origin`.
/// Modifiers hang on slanted lines below their word; `p` draws it in.
pub fn reed_kellogg(d: &DiagramParts, origin: Vec2, size: f64, ink: Color, p: f64) -> Mobject {
    let pad = size * 0.8;
    let ang: f64 = -0.95;
    let slant_len = size * 2.6;
    let dir = v2(ang.cos(), ang.sin());
    let ws = |s: &str| measure(s, "story", size).max(size * 2.0) + 2.0 * pad;
    let mods_w = |n: usize| n as f64 * size * 2.2 + pad + slant_len * dir.x;
    let sw = ws(&d.subject).max(mods_w(d.subject_mods.len()));
    let vw = ws(&d.verb).max(mods_w(d.verb_mods.len()) + d.preps.len() as f64 * size * 2.0);
    let ow = d
        .object
        .as_ref()
        .map(|o| ws(o).max(mods_w(d.object_mods.len())))
        .unwrap_or(0.0);
    let total = sw + vw + ow;
    let y = origin.y;
    let x0 = origin.x;
    let mut parts = Vec::new();
    let stage = |k: f64| clamp01(p * 5.0 - k);
    parts.push(line(v2(x0, y), v2(x0 + total * stage(0.0).max(0.001), y)).stroke(ink, 3.0));
    let div = stage(0.6);
    if div > 0.0 {
        parts.push(
            line(
                v2(x0 + sw, y + size * 1.1),
                v2(x0 + sw, y + size * 1.1 - size * 2.0 * div),
            )
            .stroke(ink, 3.0),
        );
        if ow > 0.0 {
            parts.push(
                line(v2(x0 + sw + vw, y), v2(x0 + sw + vw, y + size * 1.1 * div)).stroke(ink, 3.0),
            );
        }
    }
    let wv = stage(1.0) as f32;
    let word =
        |s: &str, c: Vec2, col: Color| font_text(s, "story", size, col).move_to(c).opacity(wv);
    parts.push(word(
        &d.subject,
        v2(x0 + sw * 0.5, y + size * 0.6),
        Pos::Noun.color().darker(0.2),
    ));
    parts.push(word(
        &d.verb,
        v2(x0 + sw + vw * 0.5, y + size * 0.6),
        Pos::Verb.color().darker(0.2),
    ));
    if let Some(o) = &d.object {
        parts.push(word(
            o,
            v2(x0 + sw + vw + ow * 0.5, y + size * 0.6),
            Pos::Noun.color().darker(0.2),
        ));
    }
    let slant = |start: Vec2, text: &str, col: Color, k: f64| -> Mobject {
        let r = clamp01(p * 5.0 - 2.0 - k * 0.25);
        if r <= 0.0 {
            return Mobject::empty();
        }
        let end = start + dir * slant_len;
        let mid = start.lerp(end, 0.5) + dir.perp() * (size * 0.4);
        group(vec![
            line(start, start.lerp(end, r)).stroke(ink, 2.2),
            font_text(text, "story", size * 0.8, col)
                .rotate(ang)
                .move_to(mid)
                .opacity(r as f32),
        ])
    };
    let hang = |list: &[String], from: f64, out: &mut Vec<Mobject>, base_k: f64| {
        for (k, m) in list.iter().enumerate() {
            let x = from + pad * 0.6 + k as f64 * size * 2.2;
            out.push(slant(
                v2(x, y),
                m,
                tag_word(m, None).color().darker(0.25),
                base_k + k as f64,
            ));
        }
    };
    hang(&d.subject_mods, x0, &mut parts, 0.0);
    hang(&d.verb_mods, x0 + sw, &mut parts, 1.0);
    if d.object.is_some() {
        hang(&d.object_mods, x0 + sw + vw, &mut parts, 2.0);
    }
    // prepositional phrases: a slant with the preposition, then a short line
    for (k, (prep, obj, om)) in d.preps.iter().enumerate() {
        let r = clamp01(p * 5.0 - 3.0 - k as f64 * 0.3);
        if r <= 0.0 {
            continue;
        }
        let start = v2(
            x0 + sw + pad * 0.6 + (d.verb_mods.len() + k) as f64 * size * 2.2,
            y,
        );
        let knee = start + dir * slant_len * 1.75;
        let ow2 = ws(obj).max(mods_w(om.len()));
        let end = knee + v2(ow2, 0.0);
        let mid = start.lerp(knee, 0.5) + dir.perp() * (size * 0.4);
        parts.push(line(start, start.lerp(knee, r)).stroke(ink, 2.2));
        parts.push(line(knee, knee.lerp(end, r.max(0.001))).stroke(ink, 2.2));
        parts.push(
            font_text(
                prep,
                "story",
                size * 0.8,
                Pos::Preposition.color().darker(0.2),
            )
            .rotate(ang)
            .move_to(mid)
            .opacity(r as f32),
        );
        parts.push(
            font_text(obj, "story", size, Pos::Noun.color().darker(0.2))
                .move_to(knee + v2(ow2 * 0.5 + pad * 0.3, size * 0.6))
                .opacity(r as f32),
        );
        for (j, m) in om.iter().enumerate() {
            let x = knee.x + pad * 0.6 + j as f64 * size * 2.2;
            parts.push(slant(
                v2(x, knee.y),
                m,
                tag_word(m, None).color().darker(0.25),
                4.0 + j as f64,
            ));
        }
    }
    group(parts)
}

// --------------------------------------------------------------------- verbs

/// The twelve tenses of English.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tense {
    PresentSimple,
    PresentContinuous,
    PresentPerfect,
    PresentPerfectContinuous,
    PastSimple,
    PastContinuous,
    PastPerfect,
    PastPerfectContinuous,
    FutureSimple,
    FutureContinuous,
    FuturePerfect,
    FuturePerfectContinuous,
}

/// Where a tense's reference point lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Time {
    Past,
    Present,
    Future,
}

/// The aspect of a tense.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aspect {
    Simple,
    Continuous,
    Perfect,
    PerfectContinuous,
}

impl Tense {
    /// All twelve, in a grid order: rows are aspects, columns past/present/future.
    pub const ALL: [Tense; 12] = [
        Tense::PastSimple,
        Tense::PresentSimple,
        Tense::FutureSimple,
        Tense::PastContinuous,
        Tense::PresentContinuous,
        Tense::FutureContinuous,
        Tense::PastPerfect,
        Tense::PresentPerfect,
        Tense::FuturePerfect,
        Tense::PastPerfectContinuous,
        Tense::PresentPerfectContinuous,
        Tense::FuturePerfectContinuous,
    ];
    pub fn new(time: Time, aspect: Aspect) -> Tense {
        let row = match aspect {
            Aspect::Simple => 0,
            Aspect::Continuous => 1,
            Aspect::Perfect => 2,
            Aspect::PerfectContinuous => 3,
        };
        let col = match time {
            Time::Past => 0,
            Time::Present => 1,
            Time::Future => 2,
        };
        Tense::ALL[row * 3 + col]
    }
    fn index(self) -> usize {
        Tense::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
    pub fn time(self) -> Time {
        [Time::Past, Time::Present, Time::Future][self.index() % 3]
    }
    pub fn aspect(self) -> Aspect {
        [
            Aspect::Simple,
            Aspect::Continuous,
            Aspect::Perfect,
            Aspect::PerfectContinuous,
        ][self.index() / 3]
    }
    pub fn name(self) -> String {
        let t = match self.time() {
            Time::Past => "Past",
            Time::Present => "Present",
            Time::Future => "Future",
        };
        let a = match self.aspect() {
            Aspect::Simple => "Simple",
            Aspect::Continuous => "Continuous",
            Aspect::Perfect => "Perfect",
            Aspect::PerfectContinuous => "Perfect Continuous",
        };
        format!("{t} {a}")
    }
    /// Parses names such as "past perfect" or "present-continuous".
    pub fn parse(s: &str) -> Option<Tense> {
        let s = s.to_lowercase().replace(['-', '_'], " ");
        let time = if s.contains("past") {
            Time::Past
        } else if s.contains("future") {
            Time::Future
        } else if s.contains("present") {
            Time::Present
        } else {
            return None;
        };
        let cont = s.contains("continuous") || s.contains("progressive");
        let aspect = match (s.contains("perfect"), cont) {
            (true, true) => Aspect::PerfectContinuous,
            (true, false) => Aspect::Perfect,
            (false, true) => Aspect::Continuous,
            _ => Aspect::Simple,
        };
        Some(Tense::new(time, aspect))
    }
}

const IRREGULAR: &[(&str, &str, &str)] = &[
    ("be", "was", "been"),
    ("have", "had", "had"),
    ("do", "did", "done"),
    ("go", "went", "gone"),
    ("eat", "ate", "eaten"),
    ("write", "wrote", "written"),
    ("see", "saw", "seen"),
    ("take", "took", "taken"),
    ("give", "gave", "given"),
    ("sing", "sang", "sung"),
    ("run", "ran", "run"),
    ("swim", "swam", "swum"),
    ("read", "read", "read"),
    ("come", "came", "come"),
    ("find", "found", "found"),
    ("make", "made", "made"),
    ("say", "said", "said"),
    ("tell", "told", "told"),
    ("know", "knew", "known"),
    ("think", "thought", "thought"),
    ("fly", "flew", "flown"),
    ("sit", "sat", "sat"),
    ("stand", "stood", "stood"),
    ("bring", "brought", "brought"),
    ("build", "built", "built"),
    ("teach", "taught", "taught"),
    ("catch", "caught", "caught"),
    ("keep", "kept", "kept"),
    ("drink", "drank", "drunk"),
    ("rise", "rose", "risen"),
    ("fall", "fell", "fallen"),
    ("break", "broke", "broken"),
    ("speak", "spoke", "spoken"),
    ("steal", "stole", "stolen"),
    ("drive", "drove", "driven"),
    ("ride", "rode", "ridden"),
    ("draw", "drew", "drawn"),
    ("grow", "grew", "grown"),
    ("throw", "threw", "thrown"),
    ("buy", "bought", "bought"),
    ("sell", "sold", "sold"),
    ("send", "sent", "sent"),
    ("leave", "left", "left"),
    ("meet", "met", "met"),
    ("sleep", "slept", "slept"),
    ("win", "won", "won"),
    ("lose", "lost", "lost"),
    ("feel", "felt", "felt"),
    ("hear", "heard", "heard"),
    ("hold", "held", "held"),
    ("get", "got", "got"),
    ("put", "put", "put"),
    ("cut", "cut", "cut"),
    ("hit", "hit", "hit"),
    ("begin", "began", "begun"),
    ("forget", "forgot", "forgotten"),
    ("choose", "chose", "chosen"),
    ("wear", "wore", "worn"),
    ("tear", "tore", "torn"),
];

fn irregular(base: &str) -> Option<(&'static str, &'static str)> {
    IRREGULAR
        .iter()
        .find(|(b, _, _)| *b == base)
        .map(|(_, p, pp)| (*p, *pp))
}

fn doubles_final(v: &str) -> bool {
    let cs: Vec<char> = v.chars().collect();
    let n = cs.len();
    if n < 3 {
        return false;
    }
    let groups = cs
        .windows(2)
        .filter(|w| !is_vowel(w[0]) && is_vowel(w[1]))
        .count()
        + usize::from(is_vowel(cs[0]));
    !is_vowel(cs[n - 1])
        && !matches!(cs[n - 1], 'w' | 'x' | 'y')
        && is_vowel(cs[n - 2])
        && !is_vowel(cs[n - 3])
        && groups == 1
}

fn regular_ed(base: &str) -> String {
    let cs: Vec<char> = base.chars().collect();
    let n = cs.len();
    if base.ends_with('e') {
        format!("{base}d")
    } else if n >= 2 && cs[n - 1] == 'y' && !is_vowel(cs[n - 2]) {
        format!("{}ied", &base[..n - 1])
    } else if doubles_final(base) {
        format!("{base}{}ed", cs[n - 1])
    } else {
        format!("{base}ed")
    }
}

/// Simple past form.
pub fn past(base: &str) -> String {
    irregular(base)
        .map(|(p, _)| p.to_string())
        .unwrap_or_else(|| regular_ed(base))
}

/// Past participle.
pub fn past_participle(base: &str) -> String {
    irregular(base)
        .map(|(_, pp)| pp.to_string())
        .unwrap_or_else(|| regular_ed(base))
}

/// The -ing form.
pub fn ing(base: &str) -> String {
    let n = base.len();
    if base == "be" || base.ends_with("ee") {
        return format!("{base}ing");
    }
    if base.ends_with("ie") {
        return format!("{}ying", &base[..n - 2]);
    }
    if base.ends_with('e') && n > 2 {
        return format!("{}ing", &base[..n - 1]);
    }
    if doubles_final(base) {
        let last = base.chars().last().unwrap_or('x');
        return format!("{base}{last}ing");
    }
    format!("{base}ing")
}

/// Third-person singular present (he/she/it form).
pub fn third_person(base: &str) -> String {
    match base {
        "be" => return "is".into(),
        "have" => return "has".into(),
        "do" => return "does".into(),
        "go" => return "goes".into(),
        _ => {}
    }
    let cs: Vec<char> = base.chars().collect();
    let n = cs.len();
    if ["s", "x", "z", "ch", "sh", "o"]
        .iter()
        .any(|s| base.ends_with(s))
    {
        format!("{base}es")
    } else if n >= 2 && cs[n - 1] == 'y' && !is_vowel(cs[n - 2]) {
        format!("{}ies", &base[..n - 1])
    } else {
        format!("{base}s")
    }
}

/// Recovers a base form from a present or simple-past form (best effort).
pub fn base_form(v: &str) -> String {
    let v = v.to_lowercase();
    match v.as_str() {
        "is" | "am" | "are" | "was" | "were" => return "be".into(),
        "has" => return "have".into(),
        "does" => return "do".into(),
        "goes" => return "go".into(),
        _ => {}
    }
    if let Some((b, _, _)) = IRREGULAR.iter().find(|(_, p, _)| *p == v) {
        return b.to_string();
    }
    if v.ends_with("ies") && v.len() > 4 {
        return format!("{}y", &v[..v.len() - 3]);
    }
    if ["ches", "shes", "sses", "xes", "zes", "oes"]
        .iter()
        .any(|s| v.ends_with(s))
    {
        return v[..v.len() - 2].to_string();
    }
    if v.ends_with('s') && !v.ends_with("ss") && v.len() > 2 {
        return v[..v.len() - 1].to_string();
    }
    v
}

/// Grammatical person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Person {
    First,
    Second,
    Third,
}

/// Person and number (`true` = plural) of a subject phrase. Handles
/// pronouns, "X and Y", "each/every/either/neither …", "X along with / as
/// well as Y", "either X or Y" (nearer subject), uncountable "-s" nouns and
/// common irregular plurals.
pub fn subject_number(subject: &str) -> (Person, bool) {
    let words: Vec<String> = subject.split_whitespace().map(clean).collect();
    if words.is_empty() {
        return (Person::Third, false);
    }
    if words.len() == 1 {
        match words[0].as_str() {
            "i" => return (Person::First, false),
            "we" => return (Person::First, true),
            "you" => return (Person::Second, true),
            "they" => return (Person::Third, true),
            "he" | "she" | "it" => return (Person::Third, false),
            _ => {}
        }
    }
    if let Some(k) = words.iter().position(|w| w == "or" || w == "nor") {
        return subject_number(&words[k + 1..].join(" "));
    }
    if matches!(
        words[0].as_str(),
        "each" | "every" | "everyone" | "everybody" | "nobody" | "someone" | "either" | "neither"
    ) {
        return (Person::Third, false);
    }
    for (k, w) in words.iter().enumerate() {
        let cut = matches!(
            w.as_str(),
            "along" | "with" | "together" | "besides" | "including"
        ) || (w == "as" && words.get(k + 1).map(|n| n == "well").unwrap_or(false));
        if cut && k > 0 {
            return subject_number(&words[..k].join(" "));
        }
    }
    if let Some(k) = words.iter().position(|w| w == "of") {
        if k > 0 && words[0] == "one" {
            return (Person::Third, false);
        }
        if k > 0 {
            return subject_number(&words[..k].join(" "));
        }
    }
    if words.iter().any(|w| w == "and") {
        return (Person::Third, true);
    }
    let head = words.last().cloned().unwrap_or_default();
    const SING_S: &[&str] = &[
        "news",
        "mathematics",
        "physics",
        "economics",
        "bus",
        "glass",
        "class",
        "gas",
        "lens",
    ];
    const PLURAL: &[&str] = &[
        "children",
        "people",
        "men",
        "women",
        "mice",
        "teeth",
        "feet",
        "geese",
        "oxen",
        "police",
        "cattle",
        "scissors",
        "trousers",
        "spectacles",
    ];
    if PLURAL.contains(&head.as_str()) {
        return (Person::Third, true);
    }
    if SING_S.contains(&head.as_str()) || head.ends_with("ss") || head.ends_with("us") {
        return (Person::Third, false);
    }
    (Person::Third, head.ends_with('s') && head.len() > 2)
}

fn be_present(person: Person, plural: bool) -> &'static str {
    match (person, plural) {
        (Person::First, false) => "am",
        (Person::Third, false) => "is",
        _ => "are",
    }
}

fn be_past(person: Person, plural: bool) -> &'static str {
    match (person, plural) {
        (Person::First, false) | (Person::Third, false) => "was",
        _ => "were",
    }
}

/// The verb phrase for `subject` + `base` in `tense`, e.g.
/// `conjugate("She", "write", Tense::PresentPerfectContinuous)` is
/// "has been writing".
pub fn conjugate(subject: &str, base: &str, tense: Tense) -> String {
    let (per, pl) = subject_number(subject);
    let third_sg = per == Person::Third && !pl;
    let has = if third_sg { "has" } else { "have" };
    let b = base.to_lowercase();
    match tense {
        Tense::PresentSimple if b == "be" => be_present(per, pl).into(),
        Tense::PresentSimple if third_sg => third_person(&b),
        Tense::PresentSimple => b,
        Tense::PastSimple if b == "be" => be_past(per, pl).into(),
        Tense::PastSimple => past(&b),
        Tense::FutureSimple => format!("will {b}"),
        Tense::PresentContinuous => format!("{} {}", be_present(per, pl), ing(&b)),
        Tense::PastContinuous => format!("{} {}", be_past(per, pl), ing(&b)),
        Tense::FutureContinuous => format!("will be {}", ing(&b)),
        Tense::PresentPerfect => format!("{has} {}", past_participle(&b)),
        Tense::PastPerfect => format!("had {}", past_participle(&b)),
        Tense::FuturePerfect => format!("will have {}", past_participle(&b)),
        Tense::PresentPerfectContinuous => format!("{has} been {}", ing(&b)),
        Tense::PastPerfectContinuous => format!("had been {}", ing(&b)),
        Tense::FuturePerfectContinuous => format!("will have been {}", ing(&b)),
    }
}

/// A full sentence: subject + conjugated verb + rest + full stop.
pub fn tense_sentence(subject: &str, base: &str, rest: &str, tense: Tense) -> String {
    let vp = conjugate(subject, base, tense);
    match rest.trim() {
        "" => format!("{subject} {vp}."),
        r => format!("{subject} {vp} {r}."),
    }
}

fn wavy(a: Vec2, b: Vec2, amp: f64, waves: f64) -> Vec<Vec2> {
    (0..=60)
        .map(|k| {
            let u = k as f64 / 60.0;
            a.lerp(b, u) + v2(0.0, amp * (u * waves * TAU).sin())
        })
        .collect()
}

fn cross_mark(c: Vec2, r: f64, col: Color) -> Mobject {
    group(vec![
        line(c + v2(-r, -r), c + v2(r, r)).stroke(col, 3.0),
        line(c + v2(-r, r), c + v2(r, -r)).stroke(col, 3.0),
    ])
}

/// A timeline picture for one tense inside `rect`: past ← now → future, a
/// reference point, and the action drawn as a mark (simple; repeated marks
/// for the present-simple habit), a wave (continuous), an arc reaching the
/// reference point (perfect) or a wave that runs right up to it (perfect
/// continuous).
pub fn tense_timeline(tense: Tense, rect: Rect, p: f64, accent: Color) -> Mobject {
    let e = smoothstep(0.0, 1.0, clamp01(p));
    let w = rect.width();
    let y = rect.center().y - rect.height() * 0.12;
    let x_of = |u: f64| rect.min.x + w * u;
    let ink = rgb8(90, 84, 80);
    let fs = (rect.height() * 0.14).min(0.18);
    let mut parts = vec![arrow(v2(x_of(0.03), y), v2(x_of(0.98), y))
        .stroke(ink, 2.0)
        .fill(ink, 1.0)];
    let now = x_of(0.5);
    parts.push(line(v2(now, y - 0.1), v2(now, y + 0.1)).stroke(ink, 2.5));
    parts.push(font_text("now", "sans", fs, ink).move_to(v2(now, y - fs * 1.6)));
    parts.push(
        font_text("past", "sans", fs * 0.9, ink.mix(WHITE, 0.4))
            .move_to(v2(x_of(0.1), y - fs * 1.6)),
    );
    parts.push(
        font_text("future", "sans", fs * 0.9, ink.mix(WHITE, 0.4))
            .move_to(v2(x_of(0.88), y - fs * 1.6)),
    );
    let r = match tense.time() {
        Time::Past => x_of(0.25),
        Time::Present => now,
        Time::Future => x_of(0.75),
    };
    if tense.time() != Time::Present {
        parts.push(
            dashed_line(v2(r, y - 0.18), v2(r, y + 0.42)).stroke(accent.mix(WHITE, 0.3), 1.5),
        );
    }
    let up = y + 0.2;
    match tense.aspect() {
        Aspect::Simple if tense == Tense::PresentSimple => {
            for k in 0..5 {
                let a = clamp01(e * 5.0 - k as f64);
                parts.push(
                    cross_mark(v2(x_of(0.18 + 0.16 * k as f64), up), 0.07, accent)
                        .opacity(a as f32),
                );
            }
        }
        Aspect::Simple => parts.push(cross_mark(
            v2(r, up),
            0.11 * ease_out_back(e).max(0.01),
            accent,
        )),
        Aspect::Continuous => {
            let pts = wavy(v2(r - w * 0.12, up), v2(r + w * 0.12, up), 0.06, 3.0);
            parts.push(
                polyline(&pts)
                    .fill(accent, 0.0)
                    .stroke(accent, 3.0)
                    .partial(0.0, e.max(0.001)),
            );
        }
        Aspect::Perfect => {
            let a = v2(r - w * 0.2, up);
            parts.push(cross_mark(a, 0.09, accent).opacity(clamp01(e * 3.0) as f32));
            let peak = v2(r - w * 0.1, up + 0.13);
            parts.push(
                smooth_curve(&[a + v2(0.05, 0.08), peak, v2(r, y + 0.06)], false)
                    .fill(accent, 0.0)
                    .stroke(accent, 2.2)
                    .partial(0.0, clamp01(e * 1.4 - 0.3).max(0.001)),
            );
        }
        Aspect::PerfectContinuous => {
            let pts = wavy(v2(r - w * 0.24, up), v2(r, up), 0.06, 3.5);
            parts.push(
                polyline(&pts)
                    .fill(accent, 0.0)
                    .stroke(accent, 3.0)
                    .partial(0.0, e.max(0.001)),
            );
            parts.push(
                line(v2(r, y + 0.05), v2(r, y + 0.36))
                    .stroke(accent, 3.0)
                    .opacity(clamp01(e * 2.0 - 1.0) as f32),
            );
        }
    }
    group(parts)
}

// ----------------------------------------------------------- transformations

fn object_pronoun(w: &str) -> String {
    match w.to_lowercase().as_str() {
        "i" => "me".into(),
        "he" => "him".into(),
        "she" => "her".into(),
        "we" => "us".into(),
        "they" => "them".into(),
        _ => w.to_string(),
    }
}

fn subject_pronoun(w: &str) -> String {
    match w.to_lowercase().as_str() {
        "me" | "i" => "I".into(),
        "him" => "he".into(),
        "her" => "she".into(),
        "us" => "we".into(),
        "them" => "they".into(),
        _ => w.to_string(),
    }
}

/// Lower-cases a phrase moved into mid-sentence unless it starts with a name
/// or "I".
fn demote(np: &str) -> String {
    let fl = np.split_whitespace().next().unwrap_or("").to_lowercase();
    if DETERMINERS.contains(&fl.as_str()) || (PRONOUNS.contains(&fl.as_str()) && fl != "i") {
        let mut c = np.chars();
        match c.next() {
            Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    } else {
        np.to_string()
    }
}

/// Turns an active clause (subject, verb base, object) into the passive.
/// Returns `None` for tenses with no everyday passive (future continuous and
/// the perfect continuous tenses).
pub fn passive_voice(subject: &str, base: &str, object: &str, tense: Tense) -> Option<String> {
    let new_subj = capitalize(&subject_pronoun(&demote(object)));
    let (per, pl) = subject_number(&new_subj);
    let has = if per == Person::Third && !pl {
        "has"
    } else {
        "have"
    };
    let aux = match tense {
        Tense::PresentSimple => be_present(per, pl).to_string(),
        Tense::PastSimple => be_past(per, pl).to_string(),
        Tense::FutureSimple => "will be".into(),
        Tense::PresentContinuous => format!("{} being", be_present(per, pl)),
        Tense::PastContinuous => format!("{} being", be_past(per, pl)),
        Tense::PresentPerfect => format!("{has} been"),
        Tense::PastPerfect => "had been".into(),
        Tense::FuturePerfect => "will have been".into(),
        _ => return None,
    };
    let agent = object_pronoun(&demote(subject));
    Some(format!(
        "{new_subj} {aux} {} by {agent}.",
        past_participle(&base.to_lowercase())
    ))
}

/// The speaker's gender, for pronoun shifts in reported speech.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gender {
    Female,
    Male,
    Plural,
}

fn backshift_aux(w: &str) -> Option<&'static str> {
    Some(match w {
        "am" | "is" => "was",
        "are" => "were",
        "was" | "were" => "had been",
        "have" | "has" => "had",
        "will" => "would",
        "shall" => "should",
        "can" => "could",
        "may" => "might",
        "do" | "does" => "did",
        "must" => "had to",
        _ => return None,
    })
}

/// was/were chosen to agree with the (shifted) subject.
fn agree_be_past(subject: &str, aux: &str) -> String {
    if aux == "was" || aux == "were" {
        let (per, pl) = subject_number(subject);
        be_past(per, pl).to_string()
    } else {
        aux.to_string()
    }
}

fn time_word(w: &str) -> Option<&'static str> {
    Some(match w {
        "now" => "then",
        "today" => "that day",
        "tonight" => "that night",
        "tomorrow" => "the next day",
        "yesterday" => "the day before",
        "here" => "there",
        "this" => "that",
        "these" => "those",
        "ago" => "before",
        _ => return None,
    })
}

struct Shift {
    subj: &'static str,
    obj: &'static str,
    poss: &'static str,
    poss_pron: &'static str,
    lis_subj: String,
    lis_obj: String,
    lis_poss: String,
}

impl Shift {
    fn word(&self, w: &str, subject_slot: bool) -> String {
        let wl = w.to_lowercase();
        match wl.as_str() {
            "i" => self.subj.into(),
            "me" => self.obj.into(),
            "my" => self.poss.into(),
            "mine" => self.poss_pron.into(),
            "we" => "they".into(),
            "us" => "them".into(),
            "our" => "their".into(),
            "you" if subject_slot => self.lis_subj.clone(),
            "you" => self.lis_obj.clone(),
            "your" => self.lis_poss.clone(),
            _ => time_word(&wl)
                .map(|t| t.to_string())
                .unwrap_or_else(|| w.to_string()),
        }
    }
}

/// Converts direct speech into reported speech, e.g.
/// `report_speech("Asha", Gender::Female, Some("me"), "I am tired.")` gives
/// "Asha told me that she was tired." Handles statements (said/told … that),
/// yes/no questions (asked if), wh-questions (asked where …) and commands
/// (told … to / not to), with pronoun, tense and time-word shifts.
pub fn report_speech(speaker: &str, gender: Gender, listener: Option<&str>, quote: &str) -> String {
    let q = quote.trim();
    let is_q = q.ends_with('?');
    let body = q.trim_end_matches(['.', '?', '!']);
    let toks: Vec<String> = body
        .split_whitespace()
        .map(|s| s.trim_matches(',').to_string())
        .collect();
    if toks.is_empty() {
        return format!("{speaker} said nothing.");
    }
    let (subj, obj, poss, poss_pron) = match gender {
        Gender::Female => ("she", "her", "her", "hers"),
        Gender::Male => ("he", "him", "his", "his"),
        Gender::Plural => ("they", "them", "their", "theirs"),
    };
    let lis = listener.unwrap_or("them").to_string();
    let lis_l = lis.to_lowercase();
    let lis_poss = match lis_l.as_str() {
        "me" => "my".to_string(),
        "him" => "his".into(),
        "her" => "her".into(),
        "us" => "our".into(),
        "them" => "their".into(),
        _ => format!("{lis}'s"),
    };
    let sh = Shift {
        subj,
        obj,
        poss,
        poss_pron,
        lis_subj: subject_pronoun(&lis),
        lis_obj: if PRONOUNS.contains(&lis_l.as_str()) {
            lis_l.clone()
        } else {
            lis.clone()
        },
        lis_poss,
    };
    let lower: Vec<String> = toks.iter().map(|w| w.to_lowercase()).collect();
    // commands
    const IMPERATIVE: &[&str] = &[
        "come", "go", "bring", "open", "close", "shut", "sit", "stand", "take", "give", "help",
        "wait", "listen", "look", "stop", "run", "read", "write", "speak", "tell", "be", "please",
        "don't", "do", "keep", "put", "call", "eat", "touch",
    ];
    let first = lower[0].as_str();
    if !is_q
        && IMPERATIVE.contains(&first)
        && !(first == "do" && lower.get(1).map(|w| w != "not").unwrap_or(true))
    {
        let mut rest: Vec<String> = toks.clone();
        let polite = first == "please";
        if polite {
            rest.remove(0);
        }
        let neg = match rest.first().map(|w| w.to_lowercase()) {
            Some(w) if w == "don't" => {
                rest.remove(0);
                true
            }
            Some(w) if w == "do" && rest.get(1).map(|x| x == "not").unwrap_or(false) => {
                rest.drain(0..2);
                true
            }
            _ => false,
        };
        let shifted: Vec<String> = rest
            .iter()
            .map(|w| sh.word(w, false).to_lowercase())
            .collect();
        let verb = if polite { "requested" } else { "told" };
        return format!(
            "{speaker} {verb} {lis} {}to {}.",
            if neg { "not " } else { "" },
            shifted.join(" ")
        );
    }
    const WH: &[&str] = &[
        "what", "where", "when", "why", "who", "how", "which", "whom", "whose",
    ];
    if is_q {
        let wh = WH.contains(&first).then(|| lower[0].clone());
        let start = usize::from(wh.is_some());
        let mut out: Vec<String> = Vec::new();
        if start < toks.len() && AUXILIARIES.contains(&lower[start].as_str()) {
            let aux = lower[start].clone();
            let rest = &toks[start + 1..];
            // the subject: a pronoun, a name, or a determiner phrase
            let mut k = 0;
            if k < rest.len() && DETERMINERS.contains(&rest[k].to_lowercase().as_str()) {
                k += 1;
                while k < rest.len() && tag_word(&rest[k], Some(Pos::Determiner)) == Pos::Adjective
                {
                    k += 1;
                }
            }
            let subj_end = (k + 1).min(rest.len());
            let new_subj: Vec<String> = rest[..subj_end].iter().map(|w| sh.word(w, true)).collect();
            let tail: Vec<String> = rest[subj_end..].iter().map(|w| sh.word(w, false)).collect();
            let subj_s = new_subj.join(" ");
            out.extend(new_subj);
            match aux.as_str() {
                "do" | "does" | "did" => {
                    if let Some((v, more)) = tail.split_first() {
                        let v = v.to_lowercase();
                        if aux == "did" {
                            out.push("had".into());
                            out.push(past_participle(&v));
                        } else {
                            out.push(past(&v));
                        }
                        out.extend(more.iter().cloned());
                    }
                }
                _ => {
                    out.push(agree_be_past(&subj_s, backshift_aux(&aux).unwrap_or("was")));
                    out.extend(tail);
                }
            }
        } else {
            // "Who broke the jug?" → who had broken the jug
            if let Some((v, more)) = toks[start..].split_first() {
                out.push("had".into());
                out.push(past_participle(&base_form(v)));
                out.extend(more.iter().map(|w| sh.word(w, false)));
            }
        }
        let lead = wh.unwrap_or_else(|| "if".into());
        let who = listener.map(|l| format!(" {l}")).unwrap_or_default();
        return format!("{speaker} asked{who} {lead} {}.", out.join(" "));
    }
    // statements: subject, then the first verb is backshifted
    let mut subj_end = 1;
    if DETERMINERS.contains(&first) {
        while subj_end < toks.len() && tag_word(&toks[subj_end], Some(Pos::Determiner)) != Pos::Noun
        {
            subj_end += 1;
        }
        subj_end = (subj_end + 1).min(toks.len());
    }
    let new_subj: Vec<String> = toks[..subj_end].iter().map(|w| sh.word(w, true)).collect();
    let subj_s = new_subj.join(" ");
    let mut out = new_subj;
    let mut verb_done = false;
    for w in &toks[subj_end..] {
        let wl = w.to_lowercase();
        if !verb_done {
            if let Some(b) = backshift_aux(&wl) {
                out.push(agree_be_past(&subj_s, b));
                verb_done = true;
                continue;
            }
            if !ADVERBS.contains(&wl.as_str()) {
                out.push(past(&base_form(&wl)));
                verb_done = true;
                continue;
            }
        }
        out.push(sh.word(w, false));
    }
    let verb = match listener {
        Some(l) => format!("told {l}"),
        None => "said".into(),
    };
    format!("{speaker} {verb} that {}.", out.join(" "))
}

fn tokens(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_string()).collect()
}

/// Morphs sentence `a` (centred at `ca`) into `b` (centred at `cb`). Words
/// shared by both fly along arcs to their new places; dropped words are
/// struck through and fade; new words fade in with `new_color`. `p` runs 0→1.
#[allow(clippy::too_many_arguments)]
pub fn word_morph(
    a: &str,
    b: &str,
    ca: Vec2,
    cb: Vec2,
    font: &str,
    size: f64,
    ink: Color,
    new_color: Color,
    p: f64,
) -> Mobject {
    let p = clamp01(p);
    let ta = tokens(a);
    let tb = tokens(b);
    let sp = measure("n", font, size) * 0.9;
    let place = |ts: &[String], c: Vec2| -> Vec<Vec2> {
        let ws: Vec<f64> = ts.iter().map(|t| measure(t, font, size)).collect();
        let total = ws.iter().sum::<f64>() + sp * ts.len().saturating_sub(1) as f64;
        let mut x = c.x - total * 0.5;
        ws.iter()
            .map(|w| {
                let at = v2(x + w * 0.5, c.y);
                x += w + sp;
                at
            })
            .collect()
    };
    let pa = place(&ta, ca);
    let pb = place(&tb, cb);
    let mut used = vec![false; ta.len()];
    let mut parts = Vec::new();
    for (j, tbw) in tb.iter().enumerate() {
        let kb = clean(tbw);
        let src = (0..ta.len()).find(|&i| !used[i] && !kb.is_empty() && clean(&ta[i]) == kb);
        match src {
            Some(i) => {
                used[i] = true;
                let stagger = (i as f64 * 0.03).min(0.25);
                let u = smoothstep(0.0, 1.0, clamp01((p - 0.2 - stagger) / 0.5));
                let sign = if j % 2 == 0 { 1.0 } else { -1.0 };
                let lift = (u * PI).sin() * size * (1.0 + (j % 3) as f64 * 0.5) * sign;
                let txt = if u < 0.5 { &ta[i] } else { tbw };
                parts.push(
                    font_text(txt, font, size, ink).move_to(pa[i].lerp(pb[j], u) + v2(0.0, lift)),
                );
            }
            None => {
                let u = clamp01((p - 0.55) / 0.35);
                if u > 0.0 {
                    parts.push(
                        font_text(tbw, font, size, new_color)
                            .move_to(pb[j] + v2(0.0, size * 0.6 * (1.0 - u)))
                            .opacity(u as f32),
                    );
                }
            }
        }
    }
    let red = rgb8(200, 70, 60);
    for (i, taw) in ta.iter().enumerate() {
        if used[i] {
            continue;
        }
        let strike = clamp01(p / 0.3);
        let fade = 1.0 - clamp01((p - 0.3) / 0.25);
        if fade <= 0.0 {
            continue;
        }
        let w = measure(taw, font, size);
        let s0 = pa[i] - v2(w * 0.55, 0.0);
        parts.push(
            font_text(taw, font, size, red)
                .move_to(pa[i])
                .opacity(fade as f32),
        );
        parts.push(
            line(s0, s0 + v2(w * 1.1 * strike.max(0.001), 0.0))
                .stroke(red, 2.5)
                .opacity(fade as f32),
        );
    }
    group(parts)
}

// ----------------------------------------------------------------- agreement

/// The verb form that agrees with `subject` (present tense, or was/were).
pub fn agree_verb(subject: &str, verb: &str) -> String {
    let v = verb.to_lowercase();
    let (per, pl) = subject_number(subject);
    match v.as_str() {
        "is" | "am" | "are" => return be_present(per, pl).into(),
        "was" | "were" => return be_past(per, pl).into(),
        _ => {}
    }
    if IRREGULAR.iter().any(|(_, p, _)| *p == v) || (v.ends_with("ed") && v.len() > 3) {
        return v;
    }
    let base = base_form(&v);
    if per == Person::Third && !pl {
        third_person(&base)
    } else {
        base
    }
}

/// Whether a finite verb agrees with `subject`.
pub fn verb_agrees(subject: &str, verb: &str) -> bool {
    agree_verb(subject, verb) == verb.to_lowercase()
}

/// Subject and verb joined by an arc with a ✓ or ✗. A wrong verb is struck
/// through and the right one drops in beneath it.
pub fn sva_check(
    subject: &str,
    verb: &str,
    rest: &str,
    center: Vec2,
    size: f64,
    p: f64,
) -> Mobject {
    let p = clamp01(p);
    let ok = verb_agrees(subject, verb);
    let good = rgb8(46, 160, 90);
    let bad = rgb8(214, 72, 64);
    let ink = rgb8(50, 46, 44);
    let sw = measure(subject, "story", size);
    let vw = measure(verb, "story", size);
    let rw = measure(rest, "story", size);
    let sp = size * 0.5;
    let x0 = center.x - (sw + vw + rw + 2.0 * sp) * 0.5;
    let sc = v2(x0 + sw * 0.5, center.y);
    let vc = v2(x0 + sw + sp + vw * 0.5, center.y);
    let rc = v2(x0 + sw + vw + 2.0 * sp + rw * 0.5, center.y);
    let mut parts = vec![
        font_text(subject, "story", size, Pos::Noun.color().darker(0.2)).move_to(sc),
        font_text(
            verb,
            "story",
            size,
            if ok || p < 0.35 {
                Pos::Verb.color().darker(0.2)
            } else {
                bad
            },
        )
        .move_to(vc),
        font_text(rest, "story", size, ink).move_to(rc),
    ];
    let link = clamp01(p / 0.35);
    if link > 0.0 {
        let a = sc + v2(0.0, size * 0.75);
        let b = vc + v2(0.0, size * 0.75);
        let mid = a.lerp(b, 0.5) + v2(0.0, size * 1.0);
        let col = if ok { good } else { bad };
        parts.push(
            smooth_curve(&[a, mid, b], false)
                .fill(col, 0.0)
                .stroke(col, 2.5)
                .partial(0.0, link),
        );
        let bp = clamp01((p - 0.35) / 0.2);
        if bp > 0.0 {
            let at = mid + v2(0.0, size * 0.4);
            let r = size * 0.38;
            let mark = if ok {
                polyline(&[
                    at + v2(-r * 0.5, 0.0),
                    at + v2(-r * 0.1, -r * 0.4),
                    at + v2(r * 0.55, r * 0.45),
                ])
            } else {
                group(vec![
                    line(at + v2(-r * 0.4, -r * 0.4), at + v2(r * 0.4, r * 0.4)),
                    line(at + v2(-r * 0.4, r * 0.4), at + v2(r * 0.4, -r * 0.4)),
                ])
            };
            parts.push(
                group(vec![
                    circle(r).move_to(at).fill(col, 1.0).no_stroke(),
                    mark.fill(WHITE, 0.0).stroke(WHITE, 3.0),
                ])
                .scale_about(ease_out_back(bp).max(0.01), at),
            );
        }
    }
    if !ok {
        let fix = clamp01((p - 0.6) / 0.3);
        if fix > 0.0 {
            let s0 = vc - v2(vw * 0.55, 0.0);
            parts.push(line(s0, s0 + v2(vw * 1.1 * fix, 0.0)).stroke(bad, 3.0));
            parts.push(
                font_text(&agree_verb(subject, verb), "hand", size * 0.95, good)
                    .move_to(vc + v2(0.0, -size * (1.1 + 0.5 * (1.0 - fix))))
                    .opacity(fix as f32),
            );
        }
    }
    group(parts)
}

// -------------------------------------------------------------------- scenes

/// "a" or "an" for a word, with the reason. Goes by sound rather than
/// spelling: an hour, a university, a one-rupee coin.
pub fn article_for(word: &str) -> (&'static str, &'static str) {
    let w = clean(word);
    const SILENT_H: &[&str] = &["hour", "honest", "honour", "honor", "heir"];
    const YOO: &[&str] = &[
        "uni", "use", "usu", "uten", "euro", "eu", "ewe", "ukulele", "uran",
    ];
    if SILENT_H.iter().any(|s| w.starts_with(s)) {
        return ("an", "silent h: it starts with a vowel sound");
    }
    if w.starts_with("one") || w.starts_with("once") {
        return ("a", "it starts with a 'w' sound");
    }
    if YOO.iter().any(|s| w.starts_with(s)) {
        return ("a", "'yoo' is a consonant sound");
    }
    match w.chars().next() {
        Some(c) if is_vowel(c) => ("an", "it starts with a vowel sound"),
        _ => ("a", "it starts with a consonant sound"),
    }
}

/// A card showing the right article for `word`: the first letter is tinted,
/// the article chip drops in and the reason appears underneath.
pub fn article_card(word: &str, center: Vec2, w: f64, p: f64) -> Mobject {
    let p = clamp01(p);
    let (art, why) = article_for(word);
    let col = if art == "an" {
        rgb8(150, 90, 190)
    } else {
        rgb8(46, 130, 170)
    };
    let h = 1.5;
    let mut parts = vec![
        rounded_rectangle(w, h, 0.14)
            .move_to(center + v2(0.05, -0.06))
            .fill(BLACK, 0.15)
            .no_stroke(),
        rounded_rectangle(w, h, 0.14)
            .move_to(center)
            .fill(rgb8(255, 252, 245), 1.0)
            .stroke(col, 2.0),
    ];
    let size = 0.42;
    let split = word.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
    let (head, tail) = word.split_at(split);
    let ww = measure(word, "story", size);
    let aw = measure("an", "story", size) + 0.3;
    let x0 = center.x - (ww + aw + 0.15) * 0.5 + aw + 0.15;
    let hw = measure(head, "story", size);
    let wy = center.y + 0.18;
    parts.push(font_text(head, "story", size, col).move_to_aligned(v2(x0, wy), v2(-1.0, 0.0)));
    parts.push(
        font_text(tail, "story", size, rgb8(50, 46, 44))
            .move_to_aligned(v2(x0 + hw + 0.01, wy), v2(-1.0, 0.0)),
    );
    let drop = clamp01(p / 0.5);
    if drop > 0.0 {
        let at = v2(x0 - 0.15 - aw * 0.5, wy + 0.8 * (1.0 - ease_out_back(drop)));
        parts.push(
            group(vec![
                rounded_rectangle(aw, 0.5, 0.12)
                    .move_to(at)
                    .fill(col, 1.0)
                    .no_stroke(),
                font_text(art, "story", size * 0.9, WHITE).move_to(at),
            ])
            .opacity(clamp01(drop * 3.0) as f32),
        );
    }
    let rp = clamp01((p - 0.5) / 0.4);
    if rp > 0.0 {
        parts.push(
            font_text(why, "hand", 0.2, col.darker(0.2))
                .move_to(center + v2(0.0, -0.42))
                .opacity(rp as f32),
        );
    }
    group(parts)
}

fn crate_box(c: Vec2, s: f64) -> (Mobject, Mobject) {
    // back (drawn before the contents) and front (drawn after)
    let wood = rgb8(196, 140, 80);
    let back = group(vec![
        rectangle(s, s * 0.75)
            .move_to(c)
            .fill(wood.darker(0.35), 1.0)
            .no_stroke(),
        polygon(&[
            c + v2(-s * 0.5, s * 0.375),
            c + v2(-s * 0.35, s * 0.5),
            c + v2(s * 0.65, s * 0.5),
            c + v2(s * 0.5, s * 0.375),
        ])
        .fill(wood.darker(0.5), 1.0)
        .no_stroke(),
    ]);
    let front = group(vec![
        rectangle(s, s * 0.45)
            .move_to(c - v2(0.0, s * 0.15))
            .fill(wood, 1.0)
            .stroke(wood.darker(0.4), 2.0),
        line(c + v2(-s * 0.5, -s * 0.15), c + v2(s * 0.5, -s * 0.15)).stroke(wood.darker(0.3), 1.5),
        polygon(&[
            c + v2(s * 0.5, -s * 0.375),
            c + v2(s * 0.65, -s * 0.25),
            c + v2(s * 0.65, s * 0.5),
            c + v2(s * 0.5, s * 0.375),
        ])
        .fill(wood.darker(0.2), 1.0)
        .stroke(wood.darker(0.4), 1.5),
    ]);
    (back, front)
}

fn ball(c: Vec2, r: f64) -> Mobject {
    group(vec![
        circle(r)
            .move_to(c)
            .fill(rgb8(226, 70, 70), 1.0)
            .stroke(rgb8(150, 40, 40), 1.5),
        circle(r * 0.3)
            .move_to(c + v2(-r * 0.35, r * 0.35))
            .fill(WHITE, 0.6)
            .no_stroke(),
    ])
}

fn ground_shadow(c: Vec2, w: f64, a: f32) -> Mobject {
    ellipse(w, w * 0.22).move_to(c).fill(BLACK, a).no_stroke()
}

fn half_ellipse(c: Vec2, rx: f64, ry: f64, a0: f64, col: Color) -> Mobject {
    let pts: Vec<Vec2> = (0..=30)
        .map(|k| {
            c + v2(
                rx * (a0 + PI * k as f64 / 30.0).cos(),
                ry * (a0 + PI * k as f64 / 30.0).sin(),
            )
        })
        .collect();
    polyline(&pts).fill(col, 0.0).stroke(col, 5.0)
}

/// A little scene for a preposition: a red ball and a wooden crate.
/// Supported: in, on, under, beside, behind, in front of, between, above,
/// over, into, around and through (anything else is drawn as "beside"). `t`
/// animates the moving ones.
pub fn preposition_scene(prep: &str, center: Vec2, s: f64, t: f64) -> Mobject {
    let floor = center.y - s * 0.5;
    let bc = v2(center.x, floor + s * 0.375);
    let r = s * 0.17;
    let (back, front) = crate_box(bc, s);
    let mut parts = vec![ground_shadow(v2(center.x + s * 0.07, floor), s * 1.3, 0.12)];
    let bob = (t * 3.0).sin().abs();
    match prep.to_lowercase().as_str() {
        "in" => {
            parts.push(back);
            parts.push(ball(bc + v2(0.0, s * 0.12 + 0.03 * bob), r));
            parts.push(front);
        }
        "on" => {
            parts.push(back);
            parts.push(front);
            parts.push(ball(bc + v2(0.05, s * 0.43 + r), r));
        }
        "under" => {
            let raised = bc + v2(0.0, s * 0.4);
            let (b2, f2) = crate_box(raised, s * 0.85);
            let leg = rgb8(120, 80, 40);
            for dx in [-0.36, 0.36, 0.5] {
                parts.push(
                    line(raised + v2(s * dx, -s * 0.3), v2(raised.x + s * dx, floor))
                        .stroke(leg, 4.0),
                );
            }
            parts.push(ball(v2(center.x, floor + r), r));
            parts.push(b2);
            parts.push(f2);
        }
        "behind" => {
            parts.push(ball(bc + v2(s * 0.42, s * 0.5), r * 0.95));
            parts.push(back);
            parts.push(front);
        }
        "in front of" => {
            parts.push(back);
            parts.push(front);
            parts.push(ground_shadow(
                v2(center.x - s * 0.1, floor - s * 0.12),
                r * 2.4,
                0.2,
            ));
            parts.push(ball(
                v2(center.x - s * 0.1, floor - s * 0.12 + r * 1.1),
                r * 1.1,
            ));
        }
        "between" => {
            let (b1, f1) = crate_box(v2(center.x - s * 0.62, floor + s * 0.225), s * 0.6);
            let (b2, f2) = crate_box(v2(center.x + s * 0.62, floor + s * 0.225), s * 0.6);
            parts = vec![
                ground_shadow(v2(center.x, floor), s * 2.0, 0.1),
                b1,
                f1,
                b2,
                f2,
            ];
            parts.push(ball(v2(center.x + s * 0.02, floor + r), r));
        }
        "above" => {
            parts.push(back);
            parts.push(front);
            let h = s * 1.0 + 0.05 * (t * 2.0).sin();
            parts.push(ground_shadow(bc + v2(0.05, s * 0.44), r * 1.6, 0.15));
            parts.push(ball(bc + v2(0.05, h), r));
        }
        "over" => {
            parts.push(back);
            parts.push(front);
            let u = (t * 0.6).fract();
            let arc_at = |uu: f64| {
                v2(
                    center.x + s * (-1.0 + 2.0 * uu),
                    floor + r + (uu * PI).sin() * s * 1.25,
                )
            };
            let trail: Vec<Vec2> = (0..=20).map(|k| arc_at(u * k as f64 / 20.0)).collect();
            parts.push(
                polyline(&trail)
                    .fill(WHITE, 0.0)
                    .stroke(rgb8(226, 70, 70), 1.5)
                    .opacity(0.5),
            );
            parts.push(ball(arc_at(u), r));
        }
        "into" => {
            parts.push(back);
            let u = (t * 0.5).fract();
            let y = bc.y + s * 1.3 - s * 1.2 * smoothstep(0.0, 1.0, (u * 1.4).min(1.0));
            parts.push(ball(v2(bc.x, y), r));
            parts.push(front);
            let g = rgb8(120, 110, 100);
            parts.push(
                arrow(bc + v2(-s * 0.6, s * 1.25), bc + v2(-s * 0.3, s * 0.55))
                    .stroke(g, 2.0)
                    .fill(g, 1.0),
            );
        }
        "around" => {
            let a = t * 2.0;
            let pos = bc + v2(s * 0.8 * a.cos(), s * 0.25 * a.sin() - s * 0.1);
            let behind = a.sin() > 0.0;
            if behind {
                parts.push(ball(pos, r * 0.9));
            }
            parts.push(back);
            parts.push(front);
            if !behind {
                parts.push(ball(pos, r));
            }
        }
        "through" => {
            let hc = v2(center.x, floor + s * 0.45);
            let blue = rgb8(60, 140, 200);
            parts = vec![ground_shadow(v2(center.x, floor), s * 1.0, 0.1)];
            parts.push(half_ellipse(
                hc,
                s * 0.16,
                s * 0.45,
                -PI * 0.5,
                blue.darker(0.25),
            ));
            let u = (t * 0.5).fract();
            parts.push(ball(v2(center.x + s * (-1.0 + 2.0 * u), floor + r), r));
            parts.push(half_ellipse(hc, s * 0.16, s * 0.45, PI * 0.5, blue));
        }
        _ => {
            parts.push(back);
            parts.push(front);
            parts.push(ball(v2(center.x + s * 0.9, floor + r), r));
        }
    }
    parts.push(
        font_text(prep, "story", s * 0.3, rgb8(60, 56, 52)).move_to(v2(center.x, floor - s * 0.32)),
    );
    group(parts)
}

// ------------------------------------------------------------------ practice

/// Name of a punctuation mark.
pub fn punct_name(mark: &str) -> &'static str {
    match mark {
        "." => "full stop",
        "," => "comma",
        "?" => "question mark",
        "!" => "exclamation mark",
        "'" | "’" => "apostrophe",
        "\"" | "“" | "”" => "inverted commas",
        ":" => "colon",
        ";" => "semicolon",
        "-" => "hyphen",
        "—" => "dash",
        _ => "mark",
    }
}

/// Words laid out with room for punctuation; each mark falls in after its
/// word index, bounces into place and is labelled with its name.
pub fn punctuation_drop(
    words: &[&str],
    marks: &[(usize, &str)],
    center: Vec2,
    size: f64,
    p: f64,
) -> Mobject {
    let p = clamp01(p);
    let sp = measure("n", "story", size) * 0.9;
    let mark_w = |m: &str| measure(m, "story", size) + 0.04;
    let widths: Vec<f64> = words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            measure(w, "story", size)
                + marks
                    .iter()
                    .filter(|(k, _)| *k == i)
                    .map(|(_, m)| mark_w(m))
                    .sum::<f64>()
        })
        .collect();
    let total = widths.iter().sum::<f64>() + sp * words.len().saturating_sub(1) as f64;
    let mut x = center.x - total * 0.5;
    let ink = rgb8(50, 46, 44);
    let col = rgb8(214, 72, 64);
    let mut parts = Vec::new();
    let mut slots = Vec::new();
    for (i, w) in words.iter().enumerate() {
        parts
            .push(font_text(w, "story", size, ink).move_to_aligned(v2(x, center.y), v2(-1.0, 0.0)));
        let mut mx = x + measure(w, "story", size) + 0.02;
        for (_, m) in marks.iter().filter(|(k, _)| *k == i) {
            slots.push((mx, *m));
            mx += mark_w(m);
        }
        x += widths[i] + sp;
    }
    let n = slots.len().max(1) as f64;
    for (j, (mx, m)) in slots.iter().enumerate() {
        let local = clamp01(p * (n + 1.0) - j as f64);
        if local <= 0.0 {
            parts.push(
                font_text("^", "sans", size * 0.6, col.mix(WHITE, 0.5))
                    .move_to(v2(*mx + 0.04, center.y - size * 0.7)),
            );
            continue;
        }
        let fall = if local < 0.6 {
            let u = local / 0.6;
            1.0 - u * u
        } else {
            0.18 * ((local - 0.6) / 0.4 * PI).sin()
        };
        let mark = font_text(m, "story", size, col);
        let mh = mark.height();
        let base_y = if matches!(*m, "'" | "’" | "\"" | "“" | "”") {
            center.y + size * 0.3
        } else {
            center.y - size * 0.25 + mh * 0.5
        };
        parts.push(mark.move_to_aligned(v2(*mx, base_y + fall * size * 3.0), v2(-1.0, 0.0)));
        if local >= 0.9 {
            parts.push(
                font_text(punct_name(m), "hand", size * 0.45, col)
                    .move_to(v2(
                        *mx + 0.05,
                        center.y - size * (1.05 + 0.55 * (j % 2) as f64),
                    ))
                    .opacity(clamp01((local - 0.9) * 10.0) as f32),
            );
        }
    }
    group(parts)
}

/// A multiple-choice card. As `reveal` runs 0→1 a wrong `chosen` option
/// shakes red and the correct one turns green.
#[allow(clippy::too_many_arguments)]
pub fn quiz_card(
    question: &str,
    options: &[&str],
    correct: usize,
    chosen: Option<usize>,
    center: Vec2,
    w: f64,
    reveal: f64,
    t: f64,
) -> Mobject {
    let rows = options.len().div_ceil(2);
    let h = 1.35 + rows as f64 * 0.72;
    let accent = rgb8(60, 110, 190);
    let mut parts = vec![
        rounded_rectangle(w, h, 0.18)
            .move_to(center + v2(0.07, -0.08))
            .fill(BLACK, 0.16)
            .no_stroke(),
        rounded_rectangle(w, h, 0.18)
            .move_to(center)
            .fill(rgb8(255, 252, 245), 1.0)
            .stroke(accent, 2.2),
        font_text("QUIZ", "sans", 0.18, accent)
            .move_to_aligned(center + v2(-w * 0.5 + 0.3, h * 0.5 - 0.28), v2(-1.0, 0.0)),
        font_text(question, "story", 0.3, rgb8(40, 36, 34))
            .move_to(center + v2(0.0, h * 0.5 - 0.7)),
    ];
    let ow = (w - 0.9) * 0.5;
    let r = clamp01(reveal);
    for (k, opt) in options.iter().enumerate() {
        let mut at = center
            + v2(
                ((k % 2) as f64 - 0.5) * (ow + 0.3),
                h * 0.5 - 1.42 - (k / 2) as f64 * 0.72,
            );
        let (mut fill, mut ink, mut edge) =
            (rgb8(240, 236, 228), rgb8(60, 56, 52), rgb8(200, 192, 180));
        if r > 0.0 && Some(k) == chosen && k != correct {
            fill = rgb8(250, 214, 210);
            ink = rgb8(180, 50, 40);
            edge = ink;
            at.x += (t * 40.0).sin() * 0.06 * (1.0 - r);
        }
        if r > 0.3 && k == correct {
            fill = rgb8(214, 240, 218);
            ink = rgb8(30, 120, 60);
            edge = ink;
        }
        let letter = (b'A' + k as u8) as char;
        parts.push(
            rounded_rectangle(ow, 0.54, 0.14)
                .move_to(at)
                .fill(fill, 1.0)
                .stroke(edge, 1.8),
        );
        parts.push(font_text(&format!("{letter}.  {opt}"), "sans", 0.22, ink).move_to(at));
    }
    group(parts)
}

/// A fill-in-the-blank line: the answer is typed into the underlined blank as
/// `p` runs 0→1, with a tick once it is complete.
pub fn fill_blank(
    before: &str,
    answer: &str,
    after: &str,
    center: Vec2,
    size: f64,
    p: f64,
) -> Mobject {
    let p = clamp01(p);
    let ink = rgb8(50, 46, 44);
    let col = rgb8(46, 130, 90);
    let bw = measure(before, "story", size);
    let aw = measure(answer, "hand", size * 1.05).max(size * 2.5) + size * 0.6;
    let fw = measure(after, "story", size);
    let sp = size * 0.3;
    let x0 = center.x - (bw + aw + fw + 2.0 * sp) * 0.5;
    let bx = x0 + bw + sp;
    let mut parts = vec![
        font_text(before, "story", size, ink).move_to_aligned(v2(x0, center.y), v2(-1.0, 0.0)),
        line(
            v2(bx, center.y - size * 0.45),
            v2(bx + aw, center.y - size * 0.45),
        )
        .stroke(col, 2.5),
        font_text(after, "story", size, ink)
            .move_to_aligned(v2(bx + aw + sp, center.y), v2(-1.0, 0.0)),
    ];
    let n = answer.chars().count();
    let shown: String = answer
        .chars()
        .take((p * 1.25 * n as f64).floor() as usize)
        .collect();
    if !shown.is_empty() {
        parts.push(
            font_text(&shown, "hand", size * 1.05, col)
                .move_to_aligned(v2(bx + size * 0.3, center.y), v2(-1.0, 0.0)),
        );
    }
    if p > 0.85 {
        let at = v2(bx + aw + size * 0.2, center.y + size * 0.9);
        let rr = size * 0.3;
        parts.push(
            polyline(&[
                at + v2(-rr * 0.6, 0.0),
                at + v2(-rr * 0.1, -rr * 0.5),
                at + v2(rr * 0.7, rr * 0.55),
            ])
            .fill(col, 0.0)
            .stroke(col, 3.0)
            .scale_about(ease_out_back((p - 0.85) / 0.15).max(0.01), at),
        );
    }
    group(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tagging() {
        let t = tag_sentence("The clever crow quickly dropped small pebbles into the old jug.");
        let tags: Vec<Pos> = t.iter().map(|x| x.1).collect();
        use Pos::*;
        assert_eq!(
            tags,
            vec![
                Determiner,
                Adjective,
                Noun,
                Adverb,
                Verb,
                Adjective,
                Noun,
                Preposition,
                Determiner,
                Adjective,
                Noun,
                Punctuation
            ]
        );
        assert_eq!(tag_word("happiness", None), Noun);
        assert_eq!(tag_word("wonderful", Some(Determiner)), Adjective);
        assert_eq!(tag_sentence("She sings sweetly.")[1].1, Verb);
        assert_eq!(tag_sentence("\"Wow!\" said Ravi.")[0].1, Punctuation);
    }

    #[test]
    fn tree_shape() {
        let t = parse_tree("The thirsty crow found a jug of water.");
        assert_eq!(
            t.bracket(),
            "[S [NP [Det The] [Adj thirsty] [N crow]] [VP [V found] [NP [NP [Det a] [N jug]] [PP [P of] [NP [N water]]]]]]"
        );
        let lay = TreeLayout::new(&t, v2(0.0, 3.0), 0.9, 0.4);
        assert_eq!(lay.nodes.len(), 15);
        let ys: Vec<f64> = lay
            .nodes
            .iter()
            .filter(|n| n.word.is_some())
            .map(|n| n.at.y)
            .collect();
        assert!(ys.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-9));
        let xs: Vec<f64> = lay
            .nodes
            .iter()
            .filter(|n| n.word.is_some())
            .map(|n| n.at.x)
            .collect();
        assert!(xs.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn diagram() {
        let d = diagram_parts("The old farmer slowly planted green rice in the wet field.");
        assert_eq!(d.subject, "farmer");
        assert_eq!(d.subject_mods, vec!["The", "old"]);
        assert_eq!(d.verb, "planted");
        assert_eq!(d.verb_mods, vec!["slowly"]);
        assert_eq!(d.object.as_deref(), Some("rice"));
        assert_eq!(d.object_mods, vec!["green"]);
        assert_eq!(
            d.preps,
            vec![(
                "in".to_string(),
                "field".to_string(),
                vec!["the".to_string(), "wet".to_string()]
            )]
        );
    }

    #[test]
    fn verb_forms() {
        assert_eq!(past("stop"), "stopped");
        assert_eq!(past("carry"), "carried");
        assert_eq!(past("play"), "played");
        assert_eq!(past("visit"), "visited");
        assert_eq!(past("go"), "went");
        assert_eq!(ing("run"), "running");
        assert_eq!(ing("write"), "writing");
        assert_eq!(ing("lie"), "lying");
        assert_eq!(ing("see"), "seeing");
        assert_eq!(third_person("watch"), "watches");
        assert_eq!(third_person("fly"), "flies");
        assert_eq!(base_form("carries"), "carry");
        assert_eq!(
            conjugate("She", "write", Tense::PresentPerfectContinuous),
            "has been writing"
        );
        assert_eq!(
            conjugate("They", "play", Tense::PastContinuous),
            "were playing"
        );
        assert_eq!(conjugate("I", "be", Tense::PresentSimple), "am");
        assert_eq!(
            conjugate("Ravi", "eat", Tense::FuturePerfect),
            "will have eaten"
        );
        assert_eq!(
            tense_sentence("The birds", "sing", "", Tense::PresentSimple),
            "The birds sing."
        );
        assert_eq!(
            Tense::parse("past perfect continuous"),
            Some(Tense::PastPerfectContinuous)
        );
        for t in Tense::ALL {
            assert_eq!(Tense::parse(&t.name()), Some(t));
            assert_eq!(Tense::new(t.time(), t.aspect()), t);
        }
    }

    #[test]
    fn passive() {
        assert_eq!(
            passive_voice("The boy", "kick", "the ball", Tense::PastSimple).unwrap(),
            "The ball was kicked by the boy."
        );
        assert_eq!(
            passive_voice("She", "write", "letters", Tense::PresentSimple).unwrap(),
            "Letters are written by her."
        );
        assert_eq!(
            passive_voice("Mohan", "plant", "a tree", Tense::PresentPerfect).unwrap(),
            "A tree has been planted by Mohan."
        );
        assert_eq!(
            passive_voice("They", "help", "me", Tense::PresentContinuous).unwrap(),
            "I am being helped by them."
        );
        assert!(passive_voice("They", "help", "me", Tense::PastPerfectContinuous).is_none());
    }

    #[test]
    fn reported() {
        assert_eq!(
            report_speech("Asha", Gender::Female, Some("me"), "I am tired."),
            "Asha told me that she was tired."
        );
        assert_eq!(
            report_speech("Ravi", Gender::Male, None, "I like mangoes."),
            "Ravi said that he liked mangoes."
        );
        assert_eq!(
            report_speech(
                "The teacher",
                Gender::Female,
                Some("us"),
                "Open your books."
            ),
            "The teacher told us to open our books."
        );
        assert_eq!(
            report_speech("Mother", Gender::Female, Some("me"), "Are you hungry?"),
            "Mother asked me if I was hungry."
        );
        assert_eq!(
            report_speech(
                "The old man",
                Gender::Male,
                Some("her"),
                "Where do you live?"
            ),
            "The old man asked her where she lived."
        );
        assert_eq!(
            report_speech("Raju", Gender::Male, Some("him"), "Don't touch the stove."),
            "Raju told him not to touch the stove."
        );
        assert_eq!(
            report_speech("Meena", Gender::Female, None, "I will come tomorrow."),
            "Meena said that she would come the next day."
        );
        assert_eq!(
            report_speech("Sita", Gender::Female, Some("me"), "You are late."),
            "Sita told me that I was late."
        );
    }

    #[test]
    fn agreement() {
        assert_eq!(subject_number("The children"), (Person::Third, true));
        assert_eq!(subject_number("Each of the boys"), (Person::Third, false));
        assert_eq!(subject_number("Ram and Shyam"), (Person::Third, true));
        assert_eq!(
            subject_number("The teacher along with her students"),
            (Person::Third, false)
        );
        assert_eq!(subject_number("The news"), (Person::Third, false));
        assert_eq!(
            subject_number("Neither the cat nor the dogs"),
            (Person::Third, true)
        );
        assert!(verb_agrees("She", "sings"));
        assert!(!verb_agrees("The dogs", "barks"));
        assert_eq!(agree_verb("The dogs", "barks"), "bark");
        assert_eq!(agree_verb("Everyone", "are"), "is");
        assert_eq!(agree_verb("He", "watch"), "watches");
        assert!(verb_agrees("They", "went"));
    }

    #[test]
    fn articles() {
        assert_eq!(article_for("hour").0, "an");
        assert_eq!(article_for("university").0, "a");
        assert_eq!(article_for("umbrella").0, "an");
        assert_eq!(article_for("one-rupee").0, "a");
        assert_eq!(article_for("elephant").0, "an");
        assert_eq!(article_for("horse").0, "a");
    }

    #[test]
    fn drawing_smoke() {
        let lay = TreeLayout::new(&parse_tree("Birds fly."), v2(0.0, 2.0), 0.9, 0.4);
        assert!(draw_tree(&lay, 1.0).leaf_count() > 4);
        let m = word_morph(
            "I am happy",
            "she was happy",
            v2(0.0, 1.0),
            v2(0.0, -1.0),
            "story",
            0.4,
            BLACK,
            RED,
            0.6,
        );
        assert!(m.leaf_count() > 2);
        for prep in [
            "in",
            "on",
            "under",
            "behind",
            "in front of",
            "between",
            "above",
            "over",
            "into",
            "around",
            "through",
            "near",
        ] {
            assert!(
                preposition_scene(prep, Vec2::ZERO, 1.5, 0.7).leaf_count() > 3,
                "{prep}"
            );
        }
        for t in Tense::ALL {
            assert!(
                tense_timeline(t, Rect::from_center(Vec2::ZERO, 4.0, 1.2), 1.0, RED).leaf_count()
                    > 4
            );
        }
        assert!(
            quiz_card(
                "Pick one",
                &["a", "an"],
                1,
                Some(0),
                Vec2::ZERO,
                5.0,
                1.0,
                0.2
            )
            .leaf_count()
                > 4
        );
        assert!(
            punctuation_drop(
                &["Hello", "Ravi"],
                &[(0, ","), (1, "!")],
                Vec2::ZERO,
                0.4,
                1.0
            )
            .leaf_count()
                > 3
        );
        assert!(
            reed_kellogg(
                &diagram_parts("Birds sing sweetly."),
                Vec2::ZERO,
                0.4,
                BLACK,
                1.0
            )
            .leaf_count()
                > 3
        );
    }
}
