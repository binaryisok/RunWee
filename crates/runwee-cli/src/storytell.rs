//! Script bindings for the storytelling phases (T1–T5): characters and
//! critters, scenery, speech bubbles and narration, folk-art frames and
//! weather, poetry reveals / rhyme / meter / figures, grammar visuals and
//! classroom extras (story maps, plot mountains, mood meters, …).

use super::{col, f, strings, vec_of, N};
use rhai::{Array, Dynamic, Engine};
use runwee::grammar as gr;
use runwee::lesson as ls;
use runwee::path::Rect;
use runwee::poetry::{self as po, Figure, Meter, Poem, PoemLayout, Reveal};
use runwee::prelude::*;
use runwee::story::{self as st, BubbleKind, Character, Critter, Palette, Pose, TreeKind};
use runwee::storyart::{self as sa, FolkStyle};
use std::sync::Arc;

/// A poem laid out for animation.
#[derive(Clone)]
pub struct PoemLay(Arc<PoemLayout>);

fn rect(c: &N, w: &N, h: &N) -> Rect {
    Rect::from_center(vec_of(c), f(w), f(h))
}

fn seed(d: &N) -> u64 {
    f(d).max(0.0) as u64
}

fn pair(d: &Dynamic) -> (String, String) {
    let a = d.clone().try_cast::<Array>().unwrap_or_default();
    let s = |i: usize| a.get(i).map(|x| x.to_string()).unwrap_or_default();
    (s(0), s(1))
}

fn triple(d: &Dynamic) -> (String, String, String) {
    let a = d.clone().try_cast::<Array>().unwrap_or_default();
    let s = |i: usize| a.get(i).map(|x| x.to_string()).unwrap_or_default();
    (s(0), s(1), s(2))
}

fn label_value(d: &Dynamic) -> (String, f64) {
    let a = d.clone().try_cast::<Array>().unwrap_or_default();
    (
        a.first().map(|x| x.to_string()).unwrap_or_default(),
        a.get(1).map(f).unwrap_or(0.0),
    )
}

fn folk_style(s: &str) -> FolkStyle {
    match s.to_ascii_lowercase().as_str() {
        "warli" => FolkStyle::Warli,
        "gond" => FolkStyle::Gond,
        _ => FolkStyle::Madhubani,
    }
}

fn critter(kind: &str) -> Critter {
    match kind.to_ascii_lowercase().as_str() {
        "fox" => Critter::fox(),
        "sparrow" | "bird" => Critter::bird(rgb8(150, 104, 70), rgb8(236, 214, 180)),
        _ => Critter::crow(),
    }
}

fn gender(s: &str) -> gr::Gender {
    match s.to_ascii_lowercase().as_str() {
        "male" | "m" | "he" | "boy" => gr::Gender::Male,
        "plural" | "they" | "p" => gr::Gender::Plural,
        _ => gr::Gender::Female,
    }
}

fn tense(s: &str) -> gr::Tense {
    gr::Tense::parse(s).unwrap_or(gr::Tense::PresentSimple)
}

pub(crate) fn register_story(engine: &mut Engine) {
    // ── text ──
    engine.register_fn("move_to_aligned", |m: &mut Mobject, p: N, align: N| {
        m.clone().move_to_aligned(vec_of(&p), vec_of(&align))
    });
    engine.register_fn("story_text", |s: &str, size: N, c: N| {
        st::story_text(s, f(&size), col(&c))
    });
    engine.register_fn("story_italic", |s: &str, size: N, c: N| {
        st::story_italic(s, f(&size), col(&c))
    });
    engine.register_fn("hand_text", |s: &str, size: N, c: N| {
        st::hand_text(s, f(&size), col(&c))
    });
    engine.register_fn("font_text", |s: &str, font: &str, size: N, c: N| {
        st::font_text(s, font, f(&size), col(&c))
    });
    engine.register_fn("typed", |s: &str, p: N| st::typed(s, f(&p)));
    engine.register_fn(
        "drop_cap",
        |s: &str, top_left: N, w: N, size: N, ink: N, cap: N| {
            st::drop_cap_paragraph(s, vec_of(&top_left), f(&w), f(&size), col(&ink), col(&cap))
        },
    );

    // ── characters ──
    engine.register_type_with_name::<Character>("Character");
    engine.register_fn("character", |name: &str| Character::named(name));
    engine.register_fn("tall", |c: Character, h: N| c.with_height(f(&h)));
    engine.register_fn(
        "draw",
        |c: &mut Character, pose: &str, t: N, talking: N, facing: N, pos: N| {
            let t = f(&t);
            c.draw_at(
                &Pose::named(pose, t),
                t,
                f(&talking),
                f(&facing),
                vec_of(&pos),
            )
        },
    );
    engine.register_type_with_name::<Critter>("Critter");
    engine.register_fn("critter", critter);
    engine.register_fn("sized", |c: Critter, s: N| c.with_size(f(&s)));
    engine.register_fn(
        "draw",
        |c: &mut Critter, t: N, talking: N, motion: N, facing: N, pos: N| {
            c.draw(f(&t), f(&talking), f(&motion), f(&facing), vec_of(&pos))
        },
    );

    // ── scenery ──
    engine.register_fn(
        "landscape",
        |c: N, w: N, h: N, pal: &str, t: N, day: N, sd: N| {
            st::landscape(
                rect(&c, &w, &h),
                &Palette::named(pal),
                f(&t),
                f(&day),
                seed(&sd),
            )
        },
    );
    engine.register_fn("layered_hills", |c: N, w: N, h: N, pal: &str, sd: N| {
        st::layered_hills(rect(&c, &w, &h), &Palette::named(pal), seed(&sd))
    });
    engine.register_fn("sky", |c: N, w: N, h: N, top: N, horizon: N| {
        st::sky(rect(&c, &w, &h), col(&top), col(&horizon))
    });
    engine.register_fn(
        "tree",
        |pos: N, h: N, kind: &str, leaf: N, sway: N, sd: N| {
            st::tree(
                vec_of(&pos),
                f(&h),
                TreeKind::parse(kind),
                col(&leaf),
                f(&sway),
                seed(&sd),
            )
        },
    );
    engine.register_fn("house", |pos: N, w: N, wall: N, roof: N, lit: N| {
        st::house(vec_of(&pos), f(&w), col(&wall), col(&roof), f(&lit))
    });
    engine.register_fn("cloud", |pos: N, w: N, c: N| {
        st::cloud(vec_of(&pos), f(&w), col(&c))
    });
    engine.register_fn("sun", |pos: N, r: N, c: N| {
        st::sun(vec_of(&pos), f(&r), col(&c))
    });
    engine.register_fn("moon", |pos: N, r: N, c: N| {
        st::moon(vec_of(&pos), f(&r), col(&c))
    });

    // ── bubbles, narration, transitions ──
    engine.register_fn(
        "bubble",
        |kind: &str, s: &str, center: N, max_w: N, tail: N, size: N, p: N| {
            st::bubble(
                BubbleKind::parse(kind),
                s,
                vec_of(&center),
                f(&max_w),
                vec_of(&tail),
                f(&size),
                f(&p),
            )
        },
    );
    engine.register_fn(
        "narration",
        |s: &str, center: N, w: N, size: N, pal: &str, p: N| {
            st::narration_box(
                s,
                vec_of(&center),
                f(&w),
                f(&size),
                &Palette::named(pal),
                f(&p),
            )
        },
    );
    engine.register_fn("iris", |p: N, center: N, c: N| {
        st::iris(f(&p), vec_of(&center), col(&c))
    });

    // ── art styles & weather ──
    engine.register_fn(
        "folk_frame",
        |style: &str, c: N, w: N, h: N, band: N, t: N, sd: N| {
            sa::folk_frame(
                folk_style(style),
                rect(&c, &w, &h),
                f(&band),
                f(&t),
                seed(&sd),
            )
        },
    );
    engine.register_fn("warli_dance", |center: N, r: N, rings: N, t: N, c: N| {
        sa::warli_circle_dance(
            vec_of(&center),
            f(&r),
            f(&rings).max(1.0) as usize,
            f(&t),
            col(&c),
        )
    });
    engine.register_fn(
        "ink_mountains",
        |c: N, w: N, h: N, layers: N, ink: N, sd: N, p: N| {
            sa::ink_mountains(
                rect(&c, &w, &h),
                f(&layers).max(1.0) as usize,
                col(&ink),
                seed(&sd),
                f(&p),
            )
        },
    );
    engine.register_fn(
        "watercolor_wash",
        |c: N, w: N, h: N, top: N, bottom: N, sd: N| {
            sa::watercolor_wash(rect(&c, &w, &h), col(&top), col(&bottom), seed(&sd), 480)
        },
    );
    engine.register_fn(
        "rain",
        |c: N, w: N, h: N, t: N, n: N, color: N, wind: N, sd: N| {
            sa::rain(
                rect(&c, &w, &h),
                f(&t),
                f(&n) as usize,
                col(&color),
                f(&wind),
                seed(&sd),
            )
        },
    );
    engine.register_fn(
        "fireflies",
        |c: N, w: N, h: N, t: N, n: N, color: N, sd: N| {
            sa::fireflies(
                rect(&c, &w, &h),
                f(&t),
                f(&n) as usize,
                col(&color),
                seed(&sd),
            )
        },
    );
    engine.register_fn("snowfall", |c: N, w: N, h: N, t: N, n: N, sd: N| {
        sa::snowfall(rect(&c, &w, &h), f(&t), f(&n) as usize, seed(&sd))
    });
    engine.register_fn(
        "falling_leaves",
        |c: N, w: N, h: N, t: N, n: N, size: N, cs: Array, sd: N| {
            let cs: Vec<Color> = cs.iter().map(col).collect();
            sa::falling_leaves(
                rect(&c, &w, &h),
                f(&t),
                f(&n) as usize,
                f(&size),
                &cs,
                seed(&sd),
            )
        },
    );

    // ── poetry ──
    engine.register_type_with_name::<PoemLay>("PoemLayout");
    engine.register_fn(
        "poem",
        |title: &str, author: &str, text: &str, font: &str, size: N, top_left: N, leading: N| {
            let p = Poem::parse(title, author, text);
            PoemLay(Arc::new(PoemLayout::new(
                &p,
                font,
                f(&size),
                vec_of(&top_left),
                f(&leading),
                0.8,
            )))
        },
    );
    engine.register_fn("reveal_poem", |l: &mut PoemLay, p: N, mode: &str, c: N| {
        po::reveal_poem(&l.0, f(&p), Reveal::parse(mode), col(&c))
    });
    engine.register_fn("scheme", |l: &mut PoemLay| {
        l.0.scheme.iter().collect::<String>()
    });
    engine.register_fn("rhyme_highlights", |l: &mut PoemLay, p: N| {
        po::rhyme_highlights(&l.0, f(&p))
    });
    engine.register_fn(
        "meter_marks",
        |l: &mut PoemLay, line: N, meter: &str, c: N, p: N| {
            po::meter_marks(&l.0, f(&line) as usize, Meter::parse(meter), col(&c), f(&p))
        },
    );
    engine.register_fn("syllable_counts", |l: &mut PoemLay, dx: N, c: N, p: N| {
        po::syllable_counts(&l.0, f(&dx), col(&c), f(&p))
    });
    engine.register_fn(
        "clap_beats",
        |l: &mut PoemLay, line: N, t: N, bpm: N, c: N| {
            po::clap_beats(&l.0, f(&line) as usize, f(&t), f(&bpm), col(&c))
        },
    );
    engine.register_fn(
        "figure_callout",
        |l: &mut PoemLay, fig: &str, line: N, a: N, b: N, label_at: N, note: &str, p: N| {
            let r =
                l.0.span_rect(f(&line) as usize, f(&a) as usize, f(&b) as usize);
            po::figure_callout(Figure::parse(fig), r, vec_of(&label_at), note, f(&p))
        },
    );
    engine.register_fn("count_syllables", |w: &str| po::count_syllables(w) as i64);
    engine.register_fn("rhymes", po::rhymes);
    engine.register_fn("rhyme_scheme", |lines: Array| {
        po::rhyme_scheme(&strings(&lines))
            .into_iter()
            .collect::<String>()
    });
    engine.register_fn(
        "vocab_card",
        |word: &str,
         pos: &str,
         meaning: &str,
         example: &str,
         center: N,
         w: N,
         h: N,
         flip: N,
         accent: N| {
            po::vocab_card(
                word,
                pos,
                meaning,
                example,
                vec_of(&center),
                f(&w),
                f(&h),
                f(&flip),
                col(&accent),
            )
        },
    );
}

pub(crate) fn register_classroom(engine: &mut Engine) {
    // ── grammar ──
    engine.register_fn(
        "pos_chips",
        |s: &str, center: N, size: N, p: N, labels: bool| {
            gr::pos_chips(
                &gr::tag_sentence(s),
                vec_of(&center),
                f(&size),
                f(&p),
                labels,
            )
        },
    );
    engine.register_fn("pos_tags", |s: &str| {
        gr::tag_sentence(s)
            .into_iter()
            .map(|(_, p)| Dynamic::from(p.short().to_string()))
            .collect::<Array>()
    });
    engine.register_fn("pos_legend", |center: N, size: N, p: N| {
        gr::pos_legend(vec_of(&center), f(&size), f(&p))
    });
    engine.register_fn(
        "parse_tree",
        |s: &str, top: N, level_h: N, size: N, p: N| {
            let lay = gr::TreeLayout::new(&gr::parse_tree(s), vec_of(&top), f(&level_h), f(&size));
            gr::draw_tree(&lay, f(&p))
        },
    );
    engine.register_fn("tree_bracket", |s: &str| gr::parse_tree(s).bracket());
    engine.register_fn(
        "sentence_diagram",
        |s: &str, origin: N, size: N, ink: N, p: N| {
            gr::reed_kellogg(
                &gr::diagram_parts(s),
                vec_of(&origin),
                f(&size),
                col(&ink),
                f(&p),
            )
        },
    );
    engine.register_fn(
        "tense_timeline",
        |name: &str, c: N, w: N, h: N, p: N, accent: N| {
            gr::tense_timeline(tense(name), rect(&c, &w, &h), f(&p), col(&accent))
        },
    );
    engine.register_fn(
        "tense_sentence",
        |subject: &str, verb: &str, rest: &str, name: &str| {
            gr::tense_sentence(subject, verb, rest, tense(name))
        },
    );
    engine.register_fn(
        "passive_voice",
        |subject: &str, verb: &str, object: &str, name: &str| {
            gr::passive_voice(subject, verb, object, tense(name)).unwrap_or_default()
        },
    );
    engine.register_fn(
        "reported_speech",
        |speaker: &str, g: &str, listener: &str, quote: &str| {
            let l = (!listener.trim().is_empty()).then_some(listener);
            gr::report_speech(speaker, gender(g), l, quote)
        },
    );
    engine.register_fn(
        "word_morph",
        |a: &str, b: &str, ca: N, cb: N, font: &str, size: N, ink: N, new_color: N, p: N| {
            gr::word_morph(
                a,
                b,
                vec_of(&ca),
                vec_of(&cb),
                font,
                f(&size),
                col(&ink),
                col(&new_color),
                f(&p),
            )
        },
    );
    engine.register_fn("verb_agrees", gr::verb_agrees);
    engine.register_fn(
        "sva_check",
        |s: &str, v: &str, rest: &str, center: N, size: N, p: N| {
            gr::sva_check(s, v, rest, vec_of(&center), f(&size), f(&p))
        },
    );
    engine.register_fn("article_for", |w: &str| gr::article_for(w).0.to_string());
    engine.register_fn("article_card", |w: &str, center: N, width: N, p: N| {
        gr::article_card(w, vec_of(&center), f(&width), f(&p))
    });
    engine.register_fn("preposition_scene", |prep: &str, center: N, s: N, t: N| {
        gr::preposition_scene(prep, vec_of(&center), f(&s), f(&t))
    });
    engine.register_fn(
        "punctuation_drop",
        |words: Array, marks: Array, center: N, size: N, p: N| {
            let words = strings(&words);
            let words: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
            let marks: Vec<(usize, String)> = marks
                .iter()
                .map(|m| {
                    let (i, s) = pair(m);
                    (i.parse::<f64>().unwrap_or(0.0) as usize, s)
                })
                .collect();
            let marks: Vec<(usize, &str)> = marks.iter().map(|(i, s)| (*i, s.as_str())).collect();
            gr::punctuation_drop(&words, &marks, vec_of(&center), f(&size), f(&p))
        },
    );
    engine.register_fn(
        "quiz_card",
        |q: &str, options: Array, correct: N, chosen: N, center: N, w: N, reveal: N, t: N| {
            let opts = strings(&options);
            let opts: Vec<&str> = opts.iter().map(|s| s.as_str()).collect();
            let ch = f(&chosen);
            let ch = (ch >= 0.0).then_some(ch as usize);
            gr::quiz_card(
                q,
                &opts,
                f(&correct) as usize,
                ch,
                vec_of(&center),
                f(&w),
                f(&reveal),
                f(&t),
            )
        },
    );
    engine.register_fn(
        "fill_blank",
        |before: &str, answer: &str, after: &str, center: N, size: N, p: N| {
            gr::fill_blank(before, answer, after, vec_of(&center), f(&size), f(&p))
        },
    );

    // ── classroom extras ──
    engine.register_fn(
        "comprehension_card",
        |n: N, q: &str, a: &str, center: N, w: N, p: N, reveal: N| {
            ls::comprehension_card(
                f(&n) as usize,
                q,
                a,
                vec_of(&center),
                f(&w),
                f(&p),
                f(&reveal),
            )
        },
    );
    engine.register_fn("story_map", |cells: Array, c: N, w: N, h: N, p: N| {
        let cells: Vec<(String, String)> = cells.iter().map(pair).collect();
        let cells: Vec<(&str, &str)> = cells
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        ls::story_map(&cells, rect(&c, &w, &h), f(&p))
    });
    engine.register_fn(
        "character_web",
        |main: &str, tr: &str, others: Array, center: N, rx: N, ry: N, p: N, t: N| {
            let o: Vec<(String, String, String)> = others.iter().map(triple).collect();
            let o: Vec<(&str, &str, &str)> = o
                .iter()
                .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
                .collect();
            ls::character_web(main, tr, &o, vec_of(&center), f(&rx), f(&ry), f(&p), f(&t))
        },
    );
    engine.register_fn(
        "summary_card",
        |title: &str, summary: &str, moral: &str, center: N, w: N, p: N| {
            ls::summary_card(title, summary, moral, vec_of(&center), f(&w), f(&p))
        },
    );
    engine.register_fn(
        "plot_mountain",
        |stages: Array, c: N, w: N, h: N, p: N, t: N| {
            let s: Vec<(String, String)> = stages.iter().map(pair).collect();
            let s: Vec<(&str, &str)> = s.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
            ls::plot_mountain(&s, rect(&c, &w, &h), f(&p), f(&t))
        },
    );
    engine.register_fn(
        "mood_meter",
        |who: &str, points: Array, c: N, w: N, h: N, p: N| {
            let pts: Vec<(String, f64)> = points.iter().map(label_value).collect();
            let pts: Vec<(&str, f64)> = pts.iter().map(|(a, v)| (a.as_str(), *v)).collect();
            ls::mood_meter(who, &pts, rect(&c, &w, &h), f(&p))
        },
    );
    engine.register_fn("mood_dial", |label: &str, mood: N, center: N, r: N| {
        ls::mood_dial(label, f(&mood), vec_of(&center), f(&r))
    });
    engine.register_fn("mood_face", |center: N, r: N, mood: N| {
        let m = f(&mood);
        ls::face(vec_of(&center), f(&r), m, ls::mood_color(m))
    });
    engine.register_fn(
        "diary_page",
        |date: &str, entry: &str, sig: &str, c: N, w: N, h: N, p: N| {
            ls::diary_page(date, entry, sig, rect(&c, &w, &h), f(&p))
        },
    );
    engine.register_fn(
        "letter_page",
        |address: Array,
         date: &str,
         salutation: &str,
         body: &str,
         closing: &str,
         sig: &str,
         c: N,
         w: N,
         h: N,
         p: N| {
            let a = strings(&address);
            let a: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
            ls::letter_page(
                &a,
                date,
                salutation,
                body,
                closing,
                sig,
                rect(&c, &w, &h),
                f(&p),
            )
        },
    );
    engine.register_fn(
        "word_cloud",
        |words: Array, center: N, w: N, h: N, sd: N, p: N| {
            let ws: Vec<(String, f64)> = words.iter().map(label_value).collect();
            let ws: Vec<(&str, f64)> = ws.iter().map(|(a, v)| (a.as_str(), *v)).collect();
            ls::word_cloud(&ws, vec_of(&center), f(&w), f(&h), seed(&sd), f(&p))
        },
    );
}
