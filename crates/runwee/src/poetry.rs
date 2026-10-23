//! Poetry mode (phase T3).
//!
//! Analysis and display tools for reading poems in class:
//!
//! * Prosody: [`count_syllables`], [`split_syllables`], [`Meter`] and
//!   [`stress_pattern`].
//! * Rhyme: [`rhyme_key`], [`rhymes`] and [`rhyme_scheme`], which labels a stanza
//!   (ABAB, AABB, ABCB …) with near-rhyme tolerance.
//! * Figures of speech: the [`Figure`] taxonomy and [`suggest_figures`] (similes,
//!   alliteration, onomatopoeia, repetition).
//! * Layout: [`Poem`] parsing (stanzas split on blank lines) and [`PoemLayout`],
//!   which records every word's position so overlays can line up.
//! * Animation: [`reveal_poem`] (typewriter, handwriting and fade-up reveals),
//!   [`rhyme_highlights`] (colour pills and scheme letters), [`meter_marks`]
//!   (˘ and / over each syllable, with foot bars), [`syllable_counts`],
//!   [`clap_beats`], [`figure_callout`], [`wavy_underline`] and [`vocab_card`]
//!   (a flip card).
use crate::color::*;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::path::Rect;
use crate::shapes::*;
use crate::story::{clip_to, ease_out_back, font_text, measure, typed};

// ------------------------------------------------------------------ prosody

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

/// Lower-case letters of `word` only (drops punctuation and apostrophes).
pub fn letters(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

const SYLLABLE_EXCEPTIONS: &[(&str, usize)] = &[
    ("poem", 2),
    ("poet", 2),
    ("poems", 2),
    ("quiet", 2),
    ("being", 2),
    ("lion", 2),
    ("science", 2),
    ("idea", 3),
    ("area", 3),
    ("piano", 3),
    ("violet", 3),
    ("radio", 3),
    ("video", 3),
    ("people", 2),
    ("fire", 1),
    ("hour", 1),
    ("every", 3),
    ("naive", 2),
    ("create", 2),
    ("cruel", 2),
    ("lyre", 1),
];

/// Estimated syllables in an English word (vowel-group heuristic with the usual
/// silent-e, -ed, -es and -le rules plus a small exception list). Returns 0 for
/// words with no letters.
pub fn count_syllables(word: &str) -> usize {
    let w = letters(word);
    if w.is_empty() {
        return 0;
    }
    if let Some((_, n)) = SYLLABLE_EXCEPTIONS.iter().find(|(e, _)| *e == w) {
        return *n;
    }
    let cs: Vec<char> = w.chars().collect();
    let mut groups = 0;
    let mut prev = false;
    for (i, &c) in cs.iter().enumerate() {
        let v = is_vowel(c) && !(c == 'y' && i == 0);
        if v && !prev {
            groups += 1;
        }
        prev = v;
    }
    let n = cs.len();
    let cons = |i: usize| !is_vowel(cs[i]);
    if groups > 1 {
        if w.ends_with('e') && n >= 3 && cons(n - 2) {
            let le = w.ends_with("le") && cs[n - 3] != 'l' && cons(n - 3);
            if !le {
                groups -= 1;
            }
        } else {
            let silent_ed =
                w.ends_with("ed") && n >= 4 && cons(n - 3) && !matches!(cs[n - 3], 't' | 'd');
            let silent_es = w.ends_with("es")
                && n >= 4
                && cons(n - 3)
                && !matches!(cs[n - 3], 's' | 'x' | 'z' | 'c' | 'g')
                && !w.ends_with("ches")
                && !w.ends_with("shes");
            let silent_ely = w.ends_with("ely") && n >= 5 && cons(n - 4);
            if silent_ed || silent_es || silent_ely {
                groups -= 1;
            }
        }
    }
    groups.max(1)
}

const DIGRAPHS: &[&str] = &["th", "sh", "ch", "ph", "wh", "ck", "gh", "qu"];

/// Split a word into approximate written syllables (for display; the number of
/// pieces always equals [`count_syllables`] of the word).
pub fn split_syllables(word: &str) -> Vec<String> {
    let cs: Vec<char> = word.chars().collect();
    let target = count_syllables(word);
    if target <= 1 || cs.len() < 3 {
        return vec![word.to_string()];
    }
    let lv: Vec<bool> = cs
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let c = c.to_ascii_lowercase();
            is_vowel(c) && !(c == 'y' && i == 0)
        })
        .collect();
    let mut cuts = vec![];
    let mut i = 0;
    while i < cs.len() {
        if lv[i] {
            let mut j = i;
            while j < cs.len() && lv[j] {
                j += 1;
            }
            let mut k = j;
            while k < cs.len() && !lv[k] {
                k += 1;
            }
            if k < cs.len() && j < k {
                let cl = k - j;
                let cut = if cl == 1 {
                    j
                } else {
                    let pair: String = cs[k - 2..k].iter().collect::<String>().to_ascii_lowercase();
                    if cl == 2 && DIGRAPHS.contains(&pair.as_str()) {
                        j
                    } else if DIGRAPHS.contains(&pair.as_str()) {
                        k - 2
                    } else {
                        k - 1
                    }
                };
                cuts.push(cut);
            }
            i = j;
        } else {
            i += 1;
        }
    }
    // merge from the end (silent e, -ed …) until the piece count matches
    while cuts.len() + 1 > target {
        cuts.pop();
    }
    let mut out = vec![];
    let mut start = 0;
    for c in cuts {
        out.push(cs[start..c].iter().collect());
        start = c;
    }
    out.push(cs[start..].iter().collect());
    out
}

/// Syllables in a whole line.
pub fn line_syllables(line: &str) -> usize {
    line.split_whitespace().map(count_syllables).sum()
}

/// Classical metrical feet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meter {
    /// da-DUM
    Iambic,
    /// DUM-da
    Trochaic,
    /// da-da-DUM
    Anapestic,
    /// DUM-da-da
    Dactylic,
}

impl Meter {
    pub fn parse(s: &str) -> Meter {
        match s.to_ascii_lowercase().as_str() {
            "trochaic" | "trochee" => Meter::Trochaic,
            "anapestic" | "anapest" => Meter::Anapestic,
            "dactylic" | "dactyl" => Meter::Dactylic,
            _ => Meter::Iambic,
        }
    }
    /// One foot as stresses (`true` = stressed).
    pub fn foot(&self) -> &'static [bool] {
        match self {
            Meter::Iambic => &[false, true],
            Meter::Trochaic => &[true, false],
            Meter::Anapestic => &[false, false, true],
            Meter::Dactylic => &[true, false, false],
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Meter::Iambic => "iambic",
            Meter::Trochaic => "trochaic",
            Meter::Anapestic => "anapestic",
            Meter::Dactylic => "dactylic",
        }
    }
}

/// The idealised stress pattern of `n` syllables in `meter`.
pub fn stress_pattern(n: usize, meter: Meter) -> Vec<bool> {
    let f = meter.foot();
    (0..n).map(|i| f[i % f.len()]).collect()
}

/// Name a line length in feet ("tetrameter" …).
pub fn line_length_name(feet: usize) -> &'static str {
    match feet {
        1 => "monometer",
        2 => "dimeter",
        3 => "trimeter",
        4 => "tetrameter",
        5 => "pentameter",
        6 => "hexameter",
        7 => "heptameter",
        _ => "free verse",
    }
}

// ------------------------------------------------------------------ rhyme

/// The rhyming tail of a word: its last vowel sound plus trailing consonants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhymeKey {
    pub vowel: String,
    pub tail: String,
    /// A long vowel (silent-e or a vowel digraph).
    pub long: bool,
}

/// Compute the [`RhymeKey`] of the last word of `s`.
pub fn rhyme_key(s: &str) -> RhymeKey {
    let word = s.split_whitespace().last().unwrap_or("");
    let mut w = letters(word);
    if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") {
        w.pop();
    }
    let mut cs: Vec<char> = w.chars().collect();
    let mut long = false;
    let n = cs.len();
    if n >= 3
        && cs[n - 1] == 'e'
        && !is_vowel(cs[n - 2])
        && cs[..n - 2].iter().any(|c| is_vowel(*c))
    {
        cs.pop();
        long = true;
    }
    let mut end = cs.len();
    // silent "gh" after a vowel (through, though, high)
    if end >= 3 && cs[end - 1] == 'h' && cs[end - 2] == 'g' && is_vowel(cs[end - 3]) {
        end -= 2;
    }
    // a 'w' straight after a vowel belongs to the vowel (crowd, growth, saw)
    let vw = |k: usize| {
        (is_vowel(cs[k]) && !(k == 0 && cs[k] == 'y'))
            || (cs[k] == 'w' && k > 0 && is_vowel(cs[k - 1]))
    };
    let mut j = end;
    while j > 0 && !vw(j - 1) {
        j -= 1;
    }
    let mut i = j;
    while i > 0 && vw(i - 1) {
        i -= 1;
    }
    let vowel: String = cs[i..j].iter().collect();
    let mut tail = String::new();
    for &c in &cs[j..end] {
        let c = if c == 'z' { 's' } else { c };
        // the silent l of could / would / should
        if c == 'l' && vowel == "ou" && cs[j..end].iter().collect::<String>() == "ld" {
            continue;
        }
        if !tail.ends_with(c) {
            tail.push(c);
        }
    }
    if vowel.chars().count() >= 2 {
        long = true;
    }
    RhymeKey { vowel, tail, long }
}

/// Do two line endings rhyme (exactly, or as a near rhyme)?
pub fn rhymes(a: &str, b: &str) -> bool {
    if long_i(a) && long_i(b) {
        return true;
    }
    let (ka, kb) = (rhyme_key(a), rhyme_key(b));
    if ka.vowel.is_empty() || kb.vowel.is_empty() {
        return false;
    }
    if ka == kb {
        return true;
    }
    let long_ok = ka.long == kb.long || (ka.tail == kb.tail && ka.tail.chars().count() >= 2);
    let same_vowel = ka.vowel.chars().next() == kb.vowel.chars().next() && long_ok;
    let tails = ka.tail == kb.tail
        || (ka.tail.is_empty() && kb.tail == "s")
        || (kb.tail.is_empty() && ka.tail == "s");
    same_vowel && tails
}

/// One-syllable words ending in the long "i" sound spelt many ways:
/// I, eye, by, sky, high, pie, rye …
fn long_i(s: &str) -> bool {
    let w = letters(s.split_whitespace().last().unwrap_or(""));
    if w == "i" || w == "eye" || w == "aye" {
        return true;
    }
    let stem = if let Some(x) = w.strip_suffix("igh") {
        x
    } else if let Some(x) = w.strip_suffix("ie").or_else(|| w.strip_suffix("ye")) {
        x
    } else if let Some(x) = w.strip_suffix('y') {
        x
    } else {
        return false;
    };
    !stem.is_empty() && !stem.chars().any(|c| "aeiouy".contains(c))
}

/// Rhyme-scheme letters for a stanza (`A`, `B`, … in order of first appearance).
pub fn rhyme_scheme<S: AsRef<str>>(lines: &[S]) -> Vec<char> {
    let mut out: Vec<char> = vec![];
    let mut next = b'A';
    for (i, l) in lines.iter().enumerate() {
        let found = (0..i).find(|&j| rhymes(lines[j].as_ref(), l.as_ref()));
        match found {
            Some(j) => out.push(out[j]),
            None => {
                out.push(next as char);
                next = next.saturating_add(1);
            }
        }
    }
    out
}

/// A pleasant, distinct colour per scheme letter.
pub fn scheme_color(letter: char) -> Color {
    const C: [(u8, u8, u8); 6] = [
        (232, 96, 76),
        (64, 140, 210),
        (236, 176, 48),
        (90, 170, 110),
        (160, 100, 200),
        (40, 170, 170),
    ];
    let i = (letter as u8).saturating_sub(b'A') as usize % C.len();
    rgb8(C[i].0, C[i].1, C[i].2)
}

// ------------------------------------------------------------------ figures of speech

/// Figures of speech taught in middle school.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Figure {
    Simile,
    Metaphor,
    Personification,
    Alliteration,
    Onomatopoeia,
    Hyperbole,
    Repetition,
    Imagery,
    Rhyme,
    Oxymoron,
}

impl Figure {
    pub fn parse(s: &str) -> Figure {
        match s.to_ascii_lowercase().as_str() {
            "metaphor" => Figure::Metaphor,
            "personification" => Figure::Personification,
            "alliteration" => Figure::Alliteration,
            "onomatopoeia" => Figure::Onomatopoeia,
            "hyperbole" => Figure::Hyperbole,
            "repetition" => Figure::Repetition,
            "imagery" => Figure::Imagery,
            "rhyme" => Figure::Rhyme,
            "oxymoron" => Figure::Oxymoron,
            _ => Figure::Simile,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Figure::Simile => "Simile",
            Figure::Metaphor => "Metaphor",
            Figure::Personification => "Personification",
            Figure::Alliteration => "Alliteration",
            Figure::Onomatopoeia => "Onomatopoeia",
            Figure::Hyperbole => "Hyperbole",
            Figure::Repetition => "Repetition",
            Figure::Imagery => "Imagery",
            Figure::Rhyme => "Rhyme",
            Figure::Oxymoron => "Oxymoron",
        }
    }
    /// One-line definition for the callout.
    pub fn hint(&self) -> &'static str {
        match self {
            Figure::Simile => "compares using 'like' or 'as'",
            Figure::Metaphor => "says one thing IS another",
            Figure::Personification => "gives human actions to a non-human",
            Figure::Alliteration => "repeats the first consonant sound",
            Figure::Onomatopoeia => "a word that sounds like its meaning",
            Figure::Hyperbole => "exaggerates for effect",
            Figure::Repetition => "repeats a word or phrase",
            Figure::Imagery => "paints a picture for the senses",
            Figure::Rhyme => "matching end sounds",
            Figure::Oxymoron => "puts opposites side by side",
        }
    }
    pub fn color(&self) -> Color {
        match self {
            Figure::Simile => rgb8(56, 132, 200),
            Figure::Metaphor => rgb8(120, 84, 190),
            Figure::Personification => rgb8(222, 96, 70),
            Figure::Alliteration => rgb8(40, 150, 120),
            Figure::Onomatopoeia => rgb8(232, 150, 30),
            Figure::Hyperbole => rgb8(206, 60, 120),
            Figure::Repetition => rgb8(90, 110, 140),
            Figure::Imagery => rgb8(200, 150, 30),
            Figure::Rhyme => rgb8(232, 96, 76),
            Figure::Oxymoron => rgb8(110, 110, 110),
        }
    }
}

const SMALL_WORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "of", "to", "in", "on", "at", "by", "for", "is", "it", "as",
    "i", "but", "so", "that", "with", "o'er",
];

const ONOMATOPOEIA: &[&str] = &[
    "buzz", "hiss", "splash", "crash", "bang", "pitter", "patter", "tick", "tock", "caw", "moo",
    "chirp", "drip", "whoosh", "rustle", "murmur", "croak", "boom", "clang", "thud", "sizzle",
    "tinkle", "honk", "purr", "hum",
];

/// Spot simple figures in a line: `(figure, first word index, last word index)`.
pub fn suggest_figures(line: &str) -> Vec<(Figure, usize, usize)> {
    let words: Vec<String> = line.split_whitespace().map(letters).collect();
    let mut out = vec![];
    for (i, w) in words.iter().enumerate() {
        if w == "like" && i > 0 && i + 1 < words.len() {
            out.push((Figure::Simile, i - 1, (i + 2).min(words.len() - 1)));
        }
        if w == "as" && i + 1 < words.len() {
            // "as ADJ as NOUN" or "lonely as a cloud"
            if let Some(k) = words.iter().skip(i + 2).position(|x| x == "as") {
                out.push((Figure::Simile, i, (i + 3 + k).min(words.len() - 1)));
            } else if i > 0 && !SMALL_WORDS.contains(&words[i - 1].as_str()) {
                out.push((Figure::Simile, i - 1, (i + 2).min(words.len() - 1)));
            }
        }
        if ONOMATOPOEIA.contains(&w.as_str()) {
            out.push((Figure::Onomatopoeia, i, i));
        }
    }
    // alliteration: ≥2 content words in a window sharing their first consonant
    let content: Vec<(usize, char)> = words
        .iter()
        .enumerate()
        .filter(|(_, w)| !w.is_empty() && !SMALL_WORDS.contains(&w.as_str()))
        .filter_map(|(i, w)| w.chars().next().filter(|c| !is_vowel(*c)).map(|c| (i, c)))
        .collect();
    let mut used = vec![false; content.len()];
    for k in 0..content.len() {
        if used[k] {
            continue;
        }
        let mut last = k;
        for j in k + 1..content.len() {
            if content[j].0 - content[last].0 > 3 {
                break;
            }
            if content[j].1 == content[k].1 {
                used[j] = true;
                last = j;
            }
        }
        if last > k {
            out.push((Figure::Alliteration, content[k].0, content[last].0));
        }
    }
    // repetition of a content word
    for (i, w) in words.iter().enumerate() {
        if w.len() > 2 && !SMALL_WORDS.contains(&w.as_str()) {
            if let Some(j) = words.iter().skip(i + 1).position(|x| x == w) {
                out.push((Figure::Repetition, i, i + 1 + j));
                break;
            }
        }
    }
    out
}

// ------------------------------------------------------------------ poem & layout

/// A poem: title, author and stanzas of lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Poem {
    pub title: String,
    pub author: String,
    pub stanzas: Vec<Vec<String>>,
}

impl Poem {
    /// Parse `text`, splitting stanzas on blank lines.
    pub fn parse(title: &str, author: &str, text: &str) -> Poem {
        let mut stanzas = vec![];
        let mut cur: Vec<String> = vec![];
        for l in text.lines() {
            let l = l.trim();
            if l.is_empty() {
                if !cur.is_empty() {
                    stanzas.push(std::mem::take(&mut cur));
                }
            } else {
                cur.push(l.to_string());
            }
        }
        if !cur.is_empty() {
            stanzas.push(cur);
        }
        Poem {
            title: title.into(),
            author: author.into(),
            stanzas,
        }
    }
    pub fn lines(&self) -> Vec<&str> {
        self.stanzas.iter().flatten().map(|s| s.as_str()).collect()
    }
    /// Rhyme scheme per stanza, e.g. `["ABAAB"]`.
    pub fn schemes(&self) -> Vec<String> {
        self.stanzas
            .iter()
            .map(|s| rhyme_scheme(s).into_iter().collect())
            .collect()
    }
}

/// A word placed on the page.
#[derive(Clone, Debug)]
pub struct WordBox {
    pub text: String,
    pub line: usize,
    pub x0: f64,
    pub x1: f64,
    pub y: f64,
}

/// Every line and word position of a poem set in a given font — computed once,
/// then reused by every overlay each frame.
#[derive(Clone, Debug)]
pub struct PoemLayout {
    pub lines: Vec<String>,
    pub line_y: Vec<f64>,
    pub line_w: Vec<f64>,
    pub stanza_of: Vec<usize>,
    pub scheme: Vec<char>,
    pub words: Vec<WordBox>,
    pub left: f64,
    pub font: String,
    pub size: f64,
}

impl PoemLayout {
    /// Lay out `poem` from `top_left` (first line's centre-line `size/2` below it),
    /// `leading` × size between lines and an extra `stanza_gap` × size between stanzas.
    pub fn new(
        poem: &Poem,
        font: &str,
        size: f64,
        top_left: Vec2,
        leading: f64,
        stanza_gap: f64,
    ) -> PoemLayout {
        let mut lines = vec![];
        let mut line_y = vec![];
        let mut line_w = vec![];
        let mut stanza_of = vec![];
        let mut scheme = vec![];
        let mut words = vec![];
        let mut y = top_left.y - size * 0.5;
        for (si, st) in poem.stanzas.iter().enumerate() {
            if si > 0 {
                y -= size * stanza_gap;
            }
            let sch = rhyme_scheme(st);
            for (li, l) in st.iter().enumerate() {
                let idx = lines.len();
                let mut prefix = String::new();
                for w in l.split_whitespace() {
                    if !prefix.is_empty() {
                        prefix.push(' ');
                    }
                    prefix.push_str(w);
                    let x1 = top_left.x + measure(&prefix, font, size);
                    let x0 = x1 - measure(w, font, size);
                    words.push(WordBox {
                        text: w.to_string(),
                        line: idx,
                        x0,
                        x1,
                        y,
                    });
                }
                lines.push(l.clone());
                line_y.push(y);
                line_w.push(measure(l, font, size));
                stanza_of.push(si);
                scheme.push(sch[li]);
                y -= size * leading;
            }
        }
        PoemLayout {
            lines,
            line_y,
            line_w,
            stanza_of,
            scheme,
            words,
            left: top_left.x,
            font: font.into(),
            size,
        }
    }
    /// Words on line `i`.
    pub fn line_words(&self, i: usize) -> Vec<&WordBox> {
        self.words.iter().filter(|w| w.line == i).collect()
    }
    /// The x of the widest line's right edge.
    pub fn right(&self) -> f64 {
        self.left + self.line_w.iter().cloned().fold(0.0, f64::max)
    }
    /// Bounding box of words `a..=b` on line `i` (word indices within the line).
    pub fn span_rect(&self, i: usize, a: usize, b: usize) -> Rect {
        let ws = self.line_words(i);
        if ws.is_empty() {
            return Rect::point(v2(self.left, self.line_y.get(i).copied().unwrap_or(0.0)));
        }
        let a = a.min(ws.len() - 1);
        let b = b.min(ws.len() - 1).max(a);
        let y = ws[a].y;
        Rect::new(
            v2(ws[a].x0, y - self.size * 0.45),
            v2(ws[b].x1, y + self.size * 0.45),
        )
    }
    /// A single line as text, positioned.
    pub fn line_mob(&self, i: usize, color: Color) -> Mobject {
        font_text(&self.lines[i], &self.font, self.size, color)
            .move_to_aligned(v2(self.left, self.line_y[i]), v2(-1.0, 0.0))
    }
}

/// How lines appear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reveal {
    /// Characters typed with a blinking block cursor.
    Typewriter,
    /// A pen sweeps each line left to right.
    Handwriting,
    /// Each line rises and fades in.
    FadeUp,
}

impl Reveal {
    pub fn parse(s: &str) -> Reveal {
        match s.to_ascii_lowercase().as_str() {
            "typewriter" | "type" => Reveal::Typewriter,
            "fade" | "fadeup" | "fade_up" => Reveal::FadeUp,
            _ => Reveal::Handwriting,
        }
    }
}

/// Reveal the poem: `p` ∈ [0, 1] over all lines (each line's share ∝ its length).
pub fn reveal_poem(lay: &PoemLayout, p: f64, mode: Reveal, color: Color) -> Mobject {
    let total: usize = lay.lines.iter().map(|l| l.chars().count() + 4).sum();
    let mut shown = clamp01(p) * total as f64;
    let mut out = vec![];
    for (i, l) in lay.lines.iter().enumerate() {
        let n = (l.chars().count() + 4) as f64;
        let lp = clamp01(shown / n);
        shown -= n;
        if lp <= 0.0 {
            break;
        }
        let y = lay.line_y[i];
        match mode {
            Reveal::Typewriter => {
                let txt = typed(l, lp * 1.1);
                let typed_w = measure(&txt, &lay.font, lay.size);
                if !txt.is_empty() {
                    out.push(
                        font_text(&txt, &lay.font, lay.size, color)
                            .move_to_aligned(v2(lay.left, y), v2(-1.0, 0.0)),
                    );
                }
                let blink = lp < 1.0 || (p * 40.0).sin() > 0.0;
                if lp < 1.0 || (blink && shown < 0.0 && p < 1.0) {
                    let cx = lay.left + typed_w + lay.size * 0.12;
                    out.push(
                        rectangle(lay.size * 0.08, lay.size * 0.9)
                            .move_to(v2(cx, y))
                            .fill(color, 0.85)
                            .no_stroke(),
                    );
                }
            }
            Reveal::Handwriting => {
                let w = lay.line_w[i];
                let x = lay.left - 0.05 + (w + 0.1) * smoothstep(0.0, 1.0, lp);
                let clip = Rect::new(v2(lay.left - 0.2, y - lay.size), v2(x, y + lay.size));
                out.push(clip_to(lay.line_mob(i, color), clip));
                if lp < 1.0 {
                    let nib = v2(x, y - lay.size * 0.1 + lay.size * 0.18 * (lp * 60.0).sin());
                    out.push(nib_pen(nib, lay.size, color));
                }
            }
            Reveal::FadeUp => {
                let e = smoothstep(0.0, 1.0, lp);
                out.push(
                    lay.line_mob(i, color)
                        .shift(v2(0.0, -0.25 * (1.0 - e)))
                        .opacity(e as f32),
                );
            }
        }
    }
    group(out)
}

/// A small fountain-pen nib whose tip is at `tip`.
pub fn nib_pen(tip: Vec2, size: f64, ink: Color) -> Mobject {
    let s = size * 1.4;
    let body = polygon(&[
        tip,
        tip + v2(0.18 * s, 0.5 * s),
        tip + v2(0.32 * s, 0.42 * s),
    ])
    .fill(rgb8(70, 60, 70), 1.0)
    .no_stroke();
    let shaft = polygon(&[
        tip + v2(0.18 * s, 0.5 * s),
        tip + v2(0.32 * s, 0.42 * s),
        tip + v2(0.75 * s, 1.1 * s),
        tip + v2(0.6 * s, 1.18 * s),
    ])
    .fill(rgb8(180, 60, 50), 1.0)
    .no_stroke();
    group(vec![
        shaft,
        body,
        circle(size * 0.04).move_to(tip).fill(ink, 1.0).no_stroke(),
    ])
}

/// Colour pills behind each line's rhyming word and scheme letters at the right
/// margin; line `i` appears when `p` passes its share.
pub fn rhyme_highlights(lay: &PoemLayout, p: f64) -> Mobject {
    let n = lay.lines.len().max(1);
    let right = lay.right() + lay.size * 0.9;
    let mut out = vec![];
    for i in 0..lay.lines.len() {
        let lp = clamp01(p * n as f64 - i as f64);
        if lp <= 0.0 {
            continue;
        }
        let e = ease_out_back(lp);
        let letter = lay.scheme[i];
        let col = scheme_color(letter);
        if let Some(w) = lay.line_words(i).last() {
            let lw = letters(&w.text);
            // pill only behind the letters (not trailing punctuation)
            let x1 = if lw.len() < w.text.len() {
                w.x0 + measure(
                    w.text.trim_end_matches(|c: char| !c.is_alphanumeric()),
                    &lay.font,
                    lay.size,
                )
            } else {
                w.x1
            };
            let pw = (x1 - w.x0) + lay.size * 0.3;
            out.push(
                rounded_rectangle(pw * e.max(0.0), lay.size * 1.0, lay.size * 0.25)
                    .move_to(v2(w.x0 - lay.size * 0.15 + pw * e.max(0.0) * 0.5, w.y))
                    .fill(col, 0.28)
                    .no_stroke(),
            );
        }
        let badge = group(vec![
            circle(lay.size * 0.42).fill(col, 1.0).no_stroke(),
            font_text(&letter.to_string(), "sans", lay.size * 0.5, WHITE),
        ])
        .move_to(v2(right, lay.line_y[i]))
        .scale(e.max(0.01));
        out.push(badge);
    }
    group(out)
}

/// Draw a breve (unstressed) or ictus (stressed) mark centred at `c`.
pub fn stress_mark(c: Vec2, stressed: bool, size: f64, color: Color) -> Mobject {
    if stressed {
        line(
            c + v2(-0.12 * size, -0.18 * size),
            c + v2(0.12 * size, 0.2 * size),
        )
        .stroke(color, 3.0)
    } else {
        arc(0.14 * size, PI * 1.05, PI * 0.9)
            .move_to(c)
            .fill(color, 0.0)
            .stroke(color, 2.4)
    }
}

/// Scansion marks above line `i`: one mark per syllable (positions spread across each
/// word by syllable length), thin bars between feet. `p` reveals marks left → right.
pub fn meter_marks(lay: &PoemLayout, i: usize, meter: Meter, color: Color, p: f64) -> Mobject {
    let ws = lay.line_words(i);
    let mut centers = vec![];
    for w in &ws {
        let sy = split_syllables(&w.text);
        let total: usize = sy.iter().map(|s| s.chars().count()).sum::<usize>().max(1);
        let mut acc = 0usize;
        for s in &sy {
            let len = s.chars().count();
            let mid = (acc as f64 + len as f64 * 0.5) / total as f64;
            centers.push(w.x0 + (w.x1 - w.x0) * mid);
            acc += len;
        }
    }
    let pattern = stress_pattern(centers.len(), meter);
    let y = lay.line_y[i] + lay.size * 0.85;
    let foot = meter.foot().len();
    let n = centers.len().max(1);
    let mut out = vec![];
    for (k, (x, st)) in centers.iter().zip(&pattern).enumerate() {
        let lp = clamp01(p * n as f64 - k as f64);
        if lp <= 0.0 {
            break;
        }
        out.push(stress_mark(v2(*x, y), *st, lay.size, color).opacity(lp as f32));
        if (k + 1) % foot == 0 && k + 1 < centers.len() {
            let bx = (x + centers[k + 1]) * 0.5;
            out.push(
                line(v2(bx, y - lay.size * 0.25), v2(bx, y + lay.size * 0.25))
                    .stroke(color, 1.4)
                    .map_style(|s| s.stroke_opacity = 0.5),
            );
        }
    }
    group(out)
}

/// Syllable counts in small circles at the right margin (offset `dx` past the text).
pub fn syllable_counts(lay: &PoemLayout, dx: f64, color: Color, p: f64) -> Mobject {
    let n = lay.lines.len().max(1);
    let x = lay.right() + dx;
    group(lay.lines.iter().enumerate().filter_map(|(i, l)| {
        let lp = clamp01(p * n as f64 - i as f64);
        if lp <= 0.0 {
            return None;
        }
        let c = line_syllables(l);
        Some(
            group(vec![
                circle(lay.size * 0.38).fill(color, 0.0).stroke(color, 2.0),
                font_text(&c.to_string(), "sans", lay.size * 0.45, color),
            ])
            .move_to(v2(x, lay.line_y[i]))
            .opacity(lp as f32),
        )
    }))
}

/// "Clap the syllables": a dot under every syllable of line `i`; the beat at time `t`
/// (`bpm` beats per minute) pops. Returns the dots group.
pub fn clap_beats(lay: &PoemLayout, i: usize, t: f64, bpm: f64, color: Color) -> Mobject {
    let ws = lay.line_words(i);
    let mut xs = vec![];
    for w in &ws {
        let sy = split_syllables(&w.text);
        let total: usize = sy.iter().map(|s| s.chars().count()).sum::<usize>().max(1);
        let mut acc = 0usize;
        for s in &sy {
            let len = s.chars().count();
            xs.push(w.x0 + (w.x1 - w.x0) * ((acc as f64 + len as f64 * 0.5) / total as f64));
            acc += len;
        }
    }
    let beat = t * bpm / 60.0;
    let cur = beat.floor() as i64;
    let frac = beat - beat.floor();
    let y = lay.line_y[i] - lay.size * 0.8;
    group(xs.iter().enumerate().map(|(k, x)| {
        let active = cur == k as i64;
        let past = (k as i64) < cur;
        let r = lay.size
            * if active {
                0.16 + 0.1 * (1.0 - frac)
            } else {
                0.1
            };
        let a = if active {
            1.0
        } else if past {
            0.75
        } else {
            0.25
        };
        circle(r).move_to(v2(*x, y)).fill(color, a).no_stroke()
    }))
}

/// A wavy underline under `r` (`p` draws it left → right).
pub fn wavy_underline(r: Rect, color: Color, p: f64) -> Mobject {
    let p = clamp01(p);
    if p <= 0.0 {
        return Mobject::empty();
    }
    let w = r.width() * p;
    let n = ((w * 30.0) as usize).max(4);
    let y = r.min.y - 0.04;
    let pts: Vec<Vec2> = (0..=n)
        .map(|k| {
            let x = r.min.x + w * k as f64 / n as f64;
            v2(x, y + 0.035 * ((x - r.min.x) * 22.0).sin())
        })
        .collect();
    polyline(&pts).fill(color, 0.0).stroke(color, 2.6)
}

/// A labelled callout for a figure of speech: a rounded tag at `label_at` with the
/// figure's name and `note`, a curved arrow to `target`, and a wavy underline of it.
pub fn figure_callout(fig: Figure, target: Rect, label_at: Vec2, note: &str, p: f64) -> Mobject {
    let p = clamp01(p);
    if p <= 0.0 {
        return Mobject::empty();
    }
    let col = fig.color();
    let e = ease_out_back(clamp01(p / 0.5));
    let name = font_text(fig.name(), "story", 0.3, WHITE);
    let note_m = font_text(note, "hand", 0.24, rgb8(50, 44, 40));
    let w = name.width().max(note_m.width()) + 0.4;
    let h = 0.92;
    let head_h = 0.42;
    let card = group(vec![
        rounded_rectangle(w, h, 0.12)
            .move_to(label_at)
            .fill(rgb8(255, 252, 245), 1.0)
            .stroke(col, 2.4),
        rounded_rectangle(w, head_h, 0.12)
            .move_to(label_at + v2(0.0, (h - head_h) * 0.5))
            .fill(col, 1.0)
            .no_stroke(),
        name.move_to(label_at + v2(0.0, (h - head_h) * 0.5)),
        note_m.move_to(label_at + v2(0.0, -0.2)),
    ])
    .scale(e.max(0.01));
    // arrow from the card edge to the target's nearest side
    let tc = target.center();
    let above = label_at.y > tc.y;
    let start = label_at + v2(0.0, if above { -h * 0.5 } else { h * 0.5 });
    let end = v2(
        tc.x,
        if above {
            target.max.y + 0.06
        } else {
            target.min.y - 0.12
        },
    );
    let ap = clamp01((p - 0.3) / 0.4);
    let mut out = vec![card];
    if ap > 0.0 {
        let mid = start.lerp(end, 0.5) + v2(0.35 * (end.y - start.y).signum(), 0.0);
        let path = smooth_curve(&[start, mid, end], false);
        let shown = path.clone().map_style(|s| s.dash = None);
        let mut curve = shown.fill(col, 0.0).stroke(col, 2.2);
        if ap < 1.0 {
            let mut c2 = curve.clone();
            c2.path = c2.path.partial(0.0, ap);
            curve = c2;
        }
        out.push(curve);
        if ap >= 1.0 {
            let dir = (end - mid).normalize();
            let nrm = v2(-dir.y, dir.x);
            out.push(
                polygon(&[
                    end,
                    end - dir * 0.16 + nrm * 0.07,
                    end - dir * 0.16 - nrm * 0.07,
                ])
                .fill(col, 1.0)
                .no_stroke(),
            );
        }
    }
    out.push(wavy_underline(target, col, clamp01((p - 0.5) / 0.5)));
    group(out)
}

/// A vocabulary flip card. `flip` 0 = front (word + part of speech), 1 = back
/// (meaning + example sentence); the card turns about its vertical axis.
#[allow(clippy::too_many_arguments)]
pub fn vocab_card(
    word: &str,
    pos: &str,
    meaning: &str,
    example: &str,
    center: Vec2,
    w: f64,
    h: f64,
    flip: f64,
    accent: Color,
) -> Mobject {
    let f = clamp01(flip);
    let ang = f * PI;
    let sx = ang.cos().abs().max(0.02);
    let back = ang > PI / 2.0;
    let paper = rgb8(255, 252, 244);
    let ink = rgb8(50, 42, 38);
    let mut parts = vec![rounded_rectangle(w, h, 0.18)
        .move_to(center)
        .fill(if back { paper } else { tint_c(accent, 0.85) }, 1.0)
        .stroke(accent, 3.0)];
    if !back {
        parts.push(font_text(word, "story", 0.5, ink).move_to(center + v2(0.0, 0.15)));
        parts.push(font_text(pos, "hand", 0.26, accent).move_to(center + v2(0.0, -0.38)));
        parts.push(
            rectangle(w * 0.5, 0.025)
                .move_to(center + v2(0.0, -0.15))
                .fill(accent, 0.6)
                .no_stroke(),
        );
    } else {
        parts.push(
            rounded_rectangle(w, 0.42, 0.18)
                .move_to(center + v2(0.0, h * 0.5 - 0.21))
                .fill(accent, 1.0)
                .no_stroke(),
        );
        parts.push(font_text(word, "story", 0.26, WHITE).move_to(center + v2(0.0, h * 0.5 - 0.21)));
        let lines = crate::story::wrap_lines(meaning, "story", 0.24, &|_| w - 0.4);
        let mut y = center.y + h * 0.5 - 0.62;
        for l in &lines {
            parts.push(
                font_text(l, "story", 0.24, ink)
                    .move_to_aligned(v2(center.x - w * 0.5 + 0.2, y), v2(-1.0, 0.0)),
            );
            y -= 0.32;
        }
        y -= 0.06;
        for l in crate::story::wrap_lines(&format!("“{example}”"), "hand", 0.22, &|_| w - 0.4) {
            parts.push(
                font_text(&l, "hand", 0.22, accent.mix(ink, 0.4))
                    .move_to_aligned(v2(center.x - w * 0.5 + 0.2, y), v2(-1.0, 0.0)),
            );
            y -= 0.29;
        }
    }
    let card = group(parts).stretch(sx, 1.0);
    // a soft shadow that shrinks as the card lifts mid-flip
    let lift = (ang).sin();
    let shadow = rounded_rectangle(w * sx, h, 0.18)
        .move_to(center + v2(0.06 + 0.1 * lift, -0.08 - 0.1 * lift))
        .fill(BLACK, (0.18 - 0.08 * lift) as f32)
        .no_stroke();
    group(vec![shadow, card])
}

fn tint_c(c: Color, k: f32) -> Color {
    c.mix(WHITE, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllables() {
        let cases = [
            ("the", 1),
            ("roads", 1),
            ("yellow", 2),
            ("traveler", 3),
            ("diverged", 2),
            ("undergrowth;", 3),
            ("lonely", 2),
            ("daffodils;", 3),
            ("fluttering", 3),
            ("melancholy", 4),
            ("spheres", 1),
            ("trembling,", 2),
            ("looked", 1),
            ("wanted", 2),
            ("table", 2),
            ("breeze.", 1),
            ("o'er", 1),
            ("poem", 2),
        ];
        for (w, n) in cases {
            assert_eq!(count_syllables(w), n, "{w}");
        }
        assert_eq!(count_syllables("—"), 0);
    }

    #[test]
    fn split_matches_count() {
        for w in [
            "melancholy",
            "traveler",
            "daffodils",
            "yellow",
            "undergrowth",
            "humid",
            "shadows",
            "the",
            "wandered",
        ] {
            let s = split_syllables(w);
            assert_eq!(s.len(), count_syllables(w), "{w}: {s:?}");
            assert_eq!(s.concat(), w);
        }
        assert_eq!(split_syllables("humid"), vec!["hu", "mid"]);
    }

    #[test]
    fn schemes_of_public_domain_stanzas() {
        let frost = [
            "Two roads diverged in a yellow wood,",
            "And sorry I could not travel both",
            "And be one traveler, long I stood",
            "And looked down one as far as I could",
            "To where it bent in the undergrowth;",
        ];
        assert_eq!(rhyme_scheme(&frost).iter().collect::<String>(), "ABAAB");
        let rossetti = [
            "Who has seen the wind?",
            "Neither I nor you:",
            "But when the leaves hang trembling,",
            "The wind is passing through.",
        ];
        assert_eq!(rhyme_scheme(&rossetti).iter().collect::<String>(), "ABCB");
        let ww = [
            "I wandered lonely as a cloud",
            "That floats on high o'er vales and hills,",
            "When all at once I saw a crowd,",
            "A host, of golden daffodils;",
            "Beside the lake, beneath the trees,",
            "Fluttering and dancing in the breeze.",
        ];
        assert_eq!(rhyme_scheme(&ww).iter().collect::<String>(), "ABABCC");
        let kinney = [
            "When the humid shadows hover",
            "Over all the starry spheres",
            "And the melancholy darkness",
            "Gently weeps in rainy tears,",
        ];
        assert_eq!(rhyme_scheme(&kinney).iter().collect::<String>(), "ABCB");
    }

    #[test]
    fn meter_patterns() {
        assert_eq!(
            stress_pattern(4, Meter::Iambic),
            vec![false, true, false, true]
        );
        assert_eq!(stress_pattern(3, Meter::Dactylic), vec![true, false, false]);
        assert_eq!(Meter::parse("trochee"), Meter::Trochaic);
        assert_eq!(line_syllables("When the humid shadows hover"), 8);
        assert_eq!(line_length_name(4), "tetrameter");
    }

    #[test]
    fn figure_detection() {
        let f = suggest_figures("I wandered lonely as a cloud");
        assert!(f
            .iter()
            .any(|(k, a, b)| *k == Figure::Simile && *a == 2 && *b == 5));
        let a = suggest_figures("Beside the lake, beneath the trees,");
        assert!(a
            .iter()
            .any(|(k, a, b)| *k == Figure::Alliteration && *a == 0 && *b == 3));
        assert!(suggest_figures("the bees buzz")
            .iter()
            .any(|(k, _, _)| *k == Figure::Onomatopoeia));
        assert_eq!(Figure::parse("Hyperbole"), Figure::Hyperbole);
    }

    #[test]
    fn poem_parse_and_layout() {
        let p = Poem::parse("T", "A", "one two\nthree four\n\nfive six\nseven\n");
        assert_eq!(p.stanzas.len(), 2);
        assert_eq!(p.lines().len(), 4);
        let lay = PoemLayout::new(&p, "story", 0.4, v2(-3.0, 2.0), 1.5, 0.8);
        assert_eq!(lay.lines.len(), 4);
        assert_eq!(lay.words.len(), 7);
        assert!(lay.line_y[2] < lay.line_y[1] - 0.4 * 1.5);
        let ws = lay.line_words(0);
        assert!(ws[0].x1 <= ws[1].x0 + 1e-6);
        assert!(!reveal_poem(&lay, 0.5, Reveal::Typewriter, BLACK)
            .children
            .is_empty());
        assert!(!meter_marks(&lay, 0, Meter::Iambic, BLACK, 1.0)
            .children
            .is_empty());
    }

    #[test]
    fn long_i_spellings_rhyme() {
        assert!(rhymes("Neither you nor I:", "The wind is passing by."));
        assert!(rhymes("toss the kites on high", "about the sky;"));
        assert!(!rhymes("I", "you"));
    }
}
