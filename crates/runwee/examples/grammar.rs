//! Phase T4 — grammar visuals: colour-coded parts of speech, a growing parse
//! tree, a Reed–Kellogg diagram, the twelve tenses on timelines, active→passive
//! and direct→reported morphs, subject–verb agreement checks, prepositions and
//! articles as little scenes, punctuation drop-ins, a quiz card and a
//! fill-in-the-blank. All example sentences are original.
use runwee::grammar::*;
use runwee::prelude::story::*;
use runwee::prelude::*;

const FW: f64 = 14.22;
const FH: f64 = 8.0;
const INK: Color = Color {
    r: 0.2,
    g: 0.18,
    b: 0.17,
    a: 1.0,
};

fn board(title: &str, subtitle: &str, t: f64) -> Mobject {
    let e = clamp01(t / 0.5) as f32;
    group(vec![
        rectangle(FW, FH).fill(rgb8(247, 242, 233), 1.0).no_stroke(),
        rectangle(FW, 0.9)
            .move_to(v2(0.0, FH * 0.5 - 0.45))
            .fill(rgb8(52, 70, 96), 1.0)
            .no_stroke(),
        font_text(title, "story", 0.42, WHITE)
            .move_to_aligned(v2(-6.7, FH * 0.5 - 0.45), v2(-1.0, 0.0))
            .opacity(e),
        font_text(subtitle, "hand", 0.28, rgb8(200, 214, 236))
            .move_to_aligned(v2(6.7, FH * 0.5 - 0.45), v2(1.0, 0.0))
            .opacity(e),
    ])
}

fn act_pos(t: f64) -> Mobject {
    let s1 = tag_sentence("The clever crow quickly dropped small pebbles into the old jug.");
    let s2 = tag_sentence("Wow! She and I sang happily because we had won.");
    group(vec![
        board("Parts of speech", "every word has a job", t),
        pos_chips(&s1, v2(0.0, 1.7), 0.34, (t - 0.5) / 3.0, true),
        pos_chips(&s2, v2(0.0, -0.1), 0.34, (t - 2.5) / 3.0, true),
        pos_legend(v2(0.6, -2.3), 0.3, (t - 4.0) / 1.5),
    ])
}

fn act_tree(t: f64, lay: &TreeLayout) -> Mobject {
    group(vec![
        board("Sentence tree", "S → NP + VP", t),
        draw_tree(lay, (t - 0.5) / 5.0),
        font_text(
            "NP = noun phrase   VP = verb phrase   PP = prepositional phrase",
            "hand",
            0.26,
            rgb8(110, 100, 92),
        )
        .move_to(v2(0.0, -3.65))
        .opacity(clamp01((t - 6.0) / 0.5) as f32),
    ])
}

fn act_diagram(t: f64, d: &DiagramParts) -> Mobject {
    let sentence = "The old farmer slowly planted green rice in the wet field.";
    group(vec![
        board("Sentence diagram", "Reed–Kellogg style", t),
        pos_chips(
            &tag_sentence(sentence),
            v2(0.0, 2.6),
            0.28,
            (t - 0.3) / 1.5,
            false,
        ),
        reed_kellogg(d, v2(-4.5, 1.3), 0.5, INK, (t - 1.8) / 4.0),
        font_text(
            "subject | verb | object on the line; describing words hang below",
            "hand",
            0.26,
            rgb8(110, 100, 92),
        )
        .move_to(v2(0.0, -3.4))
        .opacity(clamp01((t - 5.5) / 0.5) as f32),
    ])
}

fn act_tenses(t: f64) -> Mobject {
    let mut parts = vec![board("The twelve tenses", "Ravi + play + football", t)];
    let cw = 4.5;
    let ch = 1.45;
    let accent = [
        rgb8(52, 120, 200),
        rgb8(214, 72, 64),
        rgb8(60, 160, 90),
        rgb8(150, 90, 190),
    ];
    for (i, tense) in Tense::ALL.iter().enumerate() {
        let (row, col) = (i / 3, i % 3);
        let c = v2(
            (col as f64 - 1.0) * (cw + 0.15),
            2.92 - ch * 0.5 - row as f64 * (ch + 0.1),
        );
        let local = clamp01((t - 0.4 - 0.45 * i as f64) / 0.8);
        if local <= 0.0 {
            continue;
        }
        let a = accent[row];
        let e = ease_out_back(local).max(0.01);
        let card = group(vec![
            rounded_rectangle(cw, ch, 0.12)
                .move_to(c)
                .fill(WHITE, 1.0)
                .stroke(a.mix(WHITE, 0.4), 1.6),
            font_text(&tense.name(), "sans", 0.2, a.darker(0.2))
                .move_to(c + v2(0.0, ch * 0.5 - 0.2)),
            tense_timeline(
                *tense,
                Rect::from_center(c + v2(0.0, 0.0), cw - 0.4, 0.72),
                local,
                a,
            ),
            font_text(
                &tense_sentence("Ravi", "play", "football", *tense),
                "story",
                0.22,
                INK,
            )
            .move_to(c + v2(0.0, -ch * 0.5 + 0.2)),
        ]);
        parts.push(card.scale_about(e, c).opacity(clamp01(local * 2.0) as f32));
    }
    group(parts)
}

fn act_morph(t: f64) -> Mobject {
    let active = "The boy kicked the ball.";
    let passive =
        passive_voice("The boy", "kick", "the ball", Tense::PastSimple).unwrap_or_default();
    let direct = "Asha said to me, \"I am tired.\"";
    let reported = report_speech("Asha", Gender::Female, Some("me"), "I am tired.");
    let blue = rgb8(52, 120, 200);
    let green = rgb8(46, 140, 80);
    let label = |s: &str, at: Vec2, c: Color, a: f64| {
        font_text(s, "sans", 0.24, c)
            .move_to_aligned(at, v2(-1.0, 0.0))
            .opacity(clamp01(a) as f32)
    };
    let p1 = (t - 1.0) / 3.0;
    let p2 = (t - 5.5) / 3.0;
    group(vec![
        board("Changing sentences", "watch the words move", t),
        label("ACTIVE", v2(-6.5, 2.55), blue, t / 0.5),
        label("→ PASSIVE", v2(-5.2, 2.55), green, p1 * 2.0 - 0.6),
        word_morph(
            active,
            &passive,
            v2(0.0, 1.85),
            v2(0.0, 1.0),
            "story",
            0.48,
            INK,
            green,
            p1,
        ),
        rounded_rectangle(13.0, 0.02, 0.0)
            .move_to(v2(0.0, -0.25))
            .fill(rgb8(210, 200, 186), 1.0)
            .no_stroke(),
        label("DIRECT SPEECH", v2(-6.5, -0.9), blue, (t - 4.8) / 0.5),
        label("→ REPORTED", v2(-4.1, -0.9), green, p2 * 2.0 - 0.6),
        word_morph(
            direct,
            &reported,
            v2(0.0, -1.65),
            v2(0.0, -2.55),
            "story",
            0.48,
            INK,
            green,
            p2,
        ),
        font_text(
            "I → she   am → was   said to → told   , \" \" → that",
            "hand",
            0.26,
            rgb8(150, 100, 60),
        )
        .move_to(v2(0.0, -3.45))
        .opacity(clamp01((t - 8.6) / 0.5) as f32),
    ])
}

fn act_sva(t: f64) -> Mobject {
    let rows = [
        ("The dogs", "barks", "at night."),
        ("Each of the boys", "has", "a bicycle."),
        ("The teacher along with her students", "are", "here."),
        ("Ram and Shyam", "play", "cricket."),
    ];
    let mut parts = vec![board(
        "Subject–verb agreement",
        "singular subject → singular verb",
        t,
    )];
    for (i, (s, v, r)) in rows.iter().enumerate() {
        parts.push(sva_check(
            s,
            v,
            r,
            v2(0.0, 2.05 - i as f64 * 1.75),
            0.36,
            (t - 0.4 - 1.2 * i as f64) / 1.6,
        ));
    }
    group(parts)
}

fn act_scenes(t: f64) -> Mobject {
    let mut parts = vec![board(
        "Prepositions & articles",
        "where is the ball?  a or an?",
        t,
    )];
    let preps = [
        "in", "on", "under", "behind", "between", "over", "into", "around",
    ];
    for (i, p) in preps.iter().enumerate() {
        let (row, col) = (i / 4, i % 4);
        let c = v2(-5.1 + col as f64 * 3.4, 2.05 - row as f64 * 2.35);
        let local = clamp01((t - 0.2 - 0.2 * i as f64) / 0.5);
        if local > 0.0 {
            parts.push(
                preposition_scene(p, c, 1.05, t).scale_about(ease_out_back(local).max(0.01), c),
            );
        }
    }
    for (i, w) in ["hour", "umbrella", "university", "one-rupee"]
        .iter()
        .enumerate()
    {
        let c = v2(-5.25 + i as f64 * 3.5, -2.95);
        parts.push(
            article_card(w, c, 3.25, (t - 3.5 - 0.6 * i as f64) / 1.0)
                .opacity(clamp01((t - 3.3 - 0.6 * i as f64) * 3.0) as f32),
        );
    }
    group(parts)
}

fn act_practice(t: f64) -> Mobject {
    let words = [
        "Ravi", "bought", "mangoes", "apples", "and", "bananas", "Did", "he", "share", "them",
    ];
    let marks = [(2, ","), (3, ","), (5, "."), (9, "?")];
    let reveal = clamp01((t - 5.5) / 0.8);
    group(vec![
        board("Practice time", "punctuate · fill in · choose", t),
        font_text("Add the punctuation:", "sans", 0.24, rgb8(110, 100, 92))
            .move_to_aligned(v2(-6.5, 2.75), v2(-1.0, 0.0)),
        punctuation_drop(&words, &marks, v2(0.0, 2.0), 0.44, (t - 0.6) / 2.5),
        font_text("Fill in the blank:", "sans", 0.24, rgb8(110, 100, 92))
            .move_to_aligned(v2(-6.5, 0.75), v2(-1.0, 0.0)),
        fill_blank(
            "She",
            "goes",
            "to school every day.",
            v2(0.0, 0.1),
            0.44,
            (t - 2.6) / 1.6,
        ),
        quiz_card(
            "Choose the right article:   ___ honest man",
            &["a", "an", "the", "no article"],
            1,
            Some(0),
            v2(0.0, -2.0),
            8.0,
            (t - 4.6) / 1.0,
            t,
        ),
        font_text(article_for("honest").1, "hand", 0.26, rgb8(30, 120, 60))
            .move_to(v2(0.0, -3.75))
            .opacity(reveal as f32),
    ])
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(247, 242, 233));
    s.always_redraw(|ctx| act_pos(ctx.elapsed));
    s.wait(7.0);
    s.remove_all();

    let tree = parse_tree("The thirsty crow found a jug of water.");
    let lay = TreeLayout::new(&tree, v2(0.0, 2.75), 0.98, 0.36);
    s.always_redraw(move |ctx| act_tree(ctx.elapsed, &lay));
    s.wait(8.0);
    s.remove_all();

    let d = diagram_parts("The old farmer slowly planted green rice in the wet field.");
    s.always_redraw(move |ctx| act_diagram(ctx.elapsed, &d));
    s.wait(7.5);
    s.remove_all();

    s.always_redraw(|ctx| act_tenses(ctx.elapsed));
    s.wait(9.0);
    s.remove_all();

    s.always_redraw(|ctx| act_morph(ctx.elapsed));
    s.wait(10.0);
    s.remove_all();

    s.always_redraw(|ctx| act_sva(ctx.elapsed));
    s.wait(7.0);
    s.remove_all();

    s.always_redraw(|ctx| act_scenes(ctx.elapsed));
    s.wait(8.0);
    s.remove_all();

    s.always_redraw(|ctx| act_practice(ctx.elapsed));
    s.wait(8.0);
    run(&s, "grammar")
}
