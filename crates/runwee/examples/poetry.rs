//! Phase T3 — poetry mode, using public-domain poems only:
//! Robert Frost, "The Road Not Taken" (1916), with a handwriting reveal and the
//! ABAAB rhyme scheme; Coates Kinney, "Rain on the Roof" (1849), typed out in
//! the rain with trochaic scansion, syllable counts and clapped beats;
//! William Wordsworth, "I Wandered Lonely as a Cloud" (1807), with
//! figure-of-speech callouts; flip-card vocabulary; Christina Rossetti, "Who Has
//! Seen the Wind?" (1872). The definitions and example sentences are original.
use runwee::poetry::*;
use runwee::prelude::story::*;
use runwee::prelude::*;
use runwee::storyart as sa;

const FW: f64 = 14.22;
const FH: f64 = 8.0;
const INK: Color = Color {
    r: 0.2,
    g: 0.16,
    b: 0.14,
    a: 1.0,
};

fn frame_rect() -> Rect {
    Rect::from_center(v2(0.0, 0.0), FW, FH)
}

const FROST: &str = "Two roads diverged in a yellow wood,
And sorry I could not travel both
And be one traveler, long I stood
And looked down one as far as I could
To where it bent in the undergrowth;";

const KINNEY: &str = "When the humid shadows hover
Over all the starry spheres
And the melancholy darkness
Gently weeps in rainy tears,";

const WORDSWORTH: &str = "I wandered lonely as a cloud
That floats on high o'er vales and hills,
When all at once I saw a crowd,
A host, of golden daffodils;
Beside the lake, beneath the trees,
Fluttering and dancing in the breeze.";

const ROSSETTI: &str = "Who has seen the wind?
Neither I nor you:
But when the leaves hang trembling,
The wind is passing through.";

fn title_block(title: &str, author: &str, at: Vec2, color: Color, p: f64) -> Mobject {
    let e = clamp01(p) as f32;
    group(vec![
        font_text(title, "story", 0.5, color).move_to_aligned(at, v2(-1.0, 0.0)),
        story_italic(author, 0.28, color.mix(WHITE, 0.3))
            .move_to_aligned(at + v2(0.02, -0.48), v2(-1.0, 0.0)),
    ])
    .opacity(e)
}

fn paper_page(r: Rect) -> Mobject {
    group(vec![
        rounded_rectangle(r.width(), r.height(), 0.12)
            .move_to(r.center() + v2(0.08, -0.1))
            .fill(BLACK, 0.18)
            .no_stroke(),
        rounded_rectangle(r.width(), r.height(), 0.12)
            .move_to(r.center())
            .fill(rgb8(252, 246, 232), 1.0)
            .stroke(rgb8(214, 196, 160), 1.5),
    ])
}

// ── Act 1 ──
fn road_illustration() -> Mobject {
    let r = Rect::new(v2(0.9, -3.2), v2(6.7, 3.2));
    let mut parts = vec![
        rounded_rectangle(r.width(), r.height(), 0.12)
            .move_to(r.center())
            .fill(rgb8(252, 248, 238), 1.0)
            .no_stroke(),
        sa::watercolor_wash(
            r.expand(-0.1),
            rgb8(250, 214, 150),
            rgb8(250, 238, 210),
            4,
            600,
        ),
    ];
    let ground = hills(r, -0.9, 0.2, WHITE, 3);
    parts.push(sa::watercolor(ground, rgb8(190, 160, 80), 6, 4));
    // the two roads diverging from the foreground
    let fork = v2(3.6, -1.6);
    parts.push(sa::watercolor(
        polygon(&[
            v2(2.8, -3.15),
            v2(4.6, -3.15),
            fork + v2(0.25, 0.0),
            fork - v2(0.25, 0.0),
        ]),
        rgb8(200, 170, 120),
        5,
        7,
    ));
    parts.push(sa::watercolor(
        polygon(&[
            fork - v2(0.25, 0.0),
            fork + v2(0.05, 0.05),
            v2(1.9, -0.75),
            v2(1.6, -0.8),
        ]),
        rgb8(200, 170, 120),
        5,
        8,
    ));
    parts.push(sa::watercolor(
        polygon(&[
            fork + v2(0.25, 0.0),
            fork - v2(0.05, -0.05),
            v2(5.4, -0.7),
            v2(5.75, -0.75),
        ]),
        rgb8(206, 186, 130),
        5,
        9,
    ));
    let autumn = [
        rgb8(226, 120, 40),
        rgb8(236, 170, 50),
        rgb8(200, 80, 40),
        rgb8(170, 140, 50),
    ];
    for (i, (x, y, rr)) in [
        (1.7, 0.1, 0.7),
        (2.7, 0.6, 0.8),
        (3.6, 0.3, 0.6),
        (4.6, 0.7, 0.85),
        (5.7, 0.1, 0.7),
        (1.6, -1.9, 0.55),
        (5.9, -2.0, 0.6),
    ]
    .into_iter()
    .enumerate()
    {
        parts.push(
            rectangle(0.13, 1.2)
                .move_to(v2(x, y - rr - 0.3))
                .fill(rgb8(110, 70, 50), 0.8)
                .no_stroke(),
        );
        parts.push(sa::watercolor(
            circle(rr).move_to(v2(x, y)),
            autumn[i % 4],
            7,
            20 + i as u64,
        ));
    }
    parts.push(
        sa::paper_fibres(r, WHITE, 5, 600)
            .blend(Blend::Multiply)
            .opacity(0.5),
    );
    let inner = clip_to(group(parts), r.expand(-0.06));
    let mut parts = vec![inner];
    parts.push(
        rounded_rectangle(r.width(), r.height(), 0.12)
            .move_to(r.center())
            .fill(WHITE, 0.0)
            .stroke(rgb8(214, 196, 160), 1.5),
    );
    group(parts)
}

fn act_road(t: f64, lay: &PoemLayout, ill: &Mobject) -> Mobject {
    let mut parts = vec![rectangle(FW, FH).fill(rgb8(240, 228, 206), 1.0).no_stroke()];
    parts.push(paper_page(Rect::new(v2(-6.8, -3.2), v2(0.5, 3.2))));
    parts.push(title_block(
        "The Road Not Taken",
        "Robert Frost, 1916 — stanza 1",
        v2(-6.3, 2.55),
        INK,
        t / 0.6,
    ));
    parts.push(reveal_poem(lay, (t - 0.8) / 6.0, Reveal::Handwriting, INK));
    if t > 7.3 {
        parts.push(rhyme_highlights(lay, (t - 7.3) / 2.4));
    }
    let sp = clamp01((t - 10.0) / 0.6);
    if sp > 0.0 {
        let letters: String = lay.scheme.iter().collect();
        let mut row = vec![font_text("Rhyme scheme:", "hand", 0.34, INK)
            .move_to_aligned(v2(-6.3, -2.55), v2(-1.0, 0.0))];
        for (k, c) in letters.chars().enumerate() {
            let col = scheme_color(c);
            row.push(
                group(vec![
                    rounded_rectangle(0.42, 0.46, 0.1)
                        .fill(col, 1.0)
                        .no_stroke(),
                    font_text(&c.to_string(), "sans", 0.28, WHITE),
                ])
                .move_to(v2(-3.7 + 0.52 * k as f64, -2.55))
                .scale(ease_out_back(clamp01(sp * 3.0 - k as f64 * 0.3)).max(0.01)),
            );
        }
        parts.push(group(row));
    }
    parts.push(ill.clone());
    parts.push(sa::falling_leaves(
        Rect::new(v2(1.0, -3.1), v2(6.6, 3.1)),
        t,
        7,
        0.22,
        &[rgb8(220, 110, 40), rgb8(240, 180, 60)],
        2,
    ));
    group(parts)
}

// ── Act 2 ──
fn act_rain(t: f64, lay: &PoemLayout) -> Mobject {
    let r = frame_rect();
    let mut parts = vec![sky(r, rgb8(24, 32, 60), rgb8(60, 74, 110))];
    // a roof line and lit window at the bottom right
    parts.push(
        polygon(&[
            v2(2.0, -4.1),
            v2(7.2, -4.1),
            v2(7.2, -1.6),
            v2(4.6, -0.2),
            v2(2.0, -1.6),
        ])
        .fill(rgb8(30, 26, 40), 1.0)
        .no_stroke(),
    );
    parts.push(
        rectangle(1.1, 1.0)
            .move_to(v2(4.6, -2.3))
            .fill(rgb8(255, 200, 110), 0.9)
            .stroke(rgb8(20, 18, 30), 4.0),
    );
    parts.push(line(v2(4.6, -2.8), v2(4.6, -1.8)).stroke(rgb8(20, 18, 30), 3.0));
    parts.push(sa::rain(r, t, 200, rgb8(200, 215, 240), 0.12, 3));
    let paper = Rect::new(v2(-6.7, -3.0), v2(1.5, 3.3));
    parts.push(paper_page(paper));
    parts.push(title_block(
        "Rain on the Roof",
        "Coates Kinney, 1849 — stanza 1",
        v2(-6.2, 2.7),
        INK,
        t / 0.5,
    ));
    parts.push(reveal_poem(lay, (t - 0.6) / 4.8, Reveal::Typewriter, INK));
    let accent = rgb8(196, 64, 50);
    if t > 5.8 {
        for i in 0..lay.lines.len() {
            parts.push(meter_marks(
                lay,
                i,
                Meter::Trochaic,
                accent,
                (t - 5.8 - 0.5 * i as f64) / 1.0,
            ));
        }
    }
    if t > 8.4 {
        parts.push(syllable_counts(
            lay,
            0.45,
            rgb8(46, 110, 160),
            (t - 8.4) / 1.0,
        ));
    }
    let lp = clamp01((t - 9.0) / 0.5);
    if lp > 0.0 {
        parts.push(
            font_text(
                "Trochaic tetrameter:  DUM-da  DUM-da  DUM-da  DUM-da",
                "hand",
                0.3,
                accent,
            )
            .move_to_aligned(v2(-6.2, -2.0), v2(-1.0, 0.0))
            .opacity(lp as f32),
        );
        parts.push(
            font_text("Clap it:", "hand", 0.3, INK)
                .move_to_aligned(v2(-6.2, -2.55), v2(-1.0, 0.0))
                .opacity(lp as f32),
        );
    }
    if t > 9.6 {
        parts.push(clap_beats(lay, 0, t - 9.6, 200.0, accent));
    }
    group(parts)
}

// ── Act 3 ──
fn daffodil(base: Vec2, h: f64, t: f64, seed: f64) -> Mobject {
    let sway = 0.12 * (t * 1.8 + seed).sin();
    let head = base + v2(0.0, h);
    let mut parts = vec![
        line(base, head).stroke(rgb8(70, 140, 60), 3.0),
        smooth_curve(
            &[base, base + v2(-0.12, h * 0.45), base + v2(-0.05, h * 0.75)],
            false,
        )
        .fill(rgb8(80, 150, 70), 0.0)
        .stroke(rgb8(80, 150, 70), 3.0),
    ];
    for k in 0..6 {
        let a = TAU * k as f64 / 6.0 + seed;
        parts.push(
            ellipse(0.2, 0.09)
                .rotate(a)
                .move_to(head + Vec2::polar(0.09, a))
                .fill(rgb8(255, 222, 60), 1.0)
                .no_stroke(),
        );
    }
    parts.push(
        circle(0.06)
            .move_to(head)
            .fill(rgb8(240, 150, 30), 1.0)
            .stroke(rgb8(220, 120, 20), 1.5),
    );
    group(parts).rotate_about(sway, base)
}

fn act_daffodils(t: f64, lay: &PoemLayout) -> Mobject {
    let r = frame_rect();
    let mut parts = vec![sky(r, rgb8(170, 210, 240), rgb8(240, 246, 236))];
    parts.push(hills(r, -2.6, 0.25, rgb8(150, 196, 110), 6));
    parts.push(hills(r, -3.2, 0.2, rgb8(120, 176, 90), 7));
    for k in 0..34 {
        let x = -7.0 + 14.0 * sa::rnd(k, 0, 5);
        let y = -3.9 + 0.9 * sa::rnd(k, 1, 5);
        parts.push(daffodil(
            v2(x, y),
            0.5 + 0.25 * sa::rnd(k, 2, 5),
            t,
            k as f64,
        ));
    }
    parts.push(cloud(v2(5.6 + 0.1 * t, 3.0), 1.8, WHITE));
    let page = Rect::new(v2(-6.8, -1.75), v2(0.2, 3.45));
    parts.push(paper_page(page));
    parts.push(title_block(
        "I Wandered Lonely as a Cloud",
        "William Wordsworth, 1807 — lines 1–6",
        v2(-6.4, 2.95),
        INK,
        t / 0.5,
    ));
    parts.push(reveal_poem(lay, (t - 0.5) / 3.5, Reveal::FadeUp, INK));
    let callouts = [
        (
            Figure::Simile,
            0usize,
            2usize,
            5usize,
            v2(2.6, 2.6),
            "'lonely as a cloud'",
        ),
        (
            Figure::Imagery,
            3,
            3,
            4,
            v2(5.3, 1.25),
            "a golden crowd of flowers",
        ),
        (
            Figure::Alliteration,
            4,
            0,
            3,
            v2(2.4, -0.1),
            "Beside … beneath: b, b",
        ),
        (
            Figure::Personification,
            5,
            0,
            2,
            v2(5.2, -1.3),
            "flowers that dance like people",
        ),
    ];
    for (k, (fig, line_i, a, b, at, note)) in callouts.into_iter().enumerate() {
        let target = lay.span_rect(line_i, a, b);
        parts.push(figure_callout(
            fig,
            target,
            at,
            note,
            (t - 4.4 - 1.5 * k as f64) / 1.0,
        ));
    }
    group(parts)
}

// ── Act 4 ──
fn act_vocab(t: f64) -> Mobject {
    let mut parts = vec![rectangle(FW, FH).fill(rgb8(244, 236, 220), 1.0).no_stroke()];
    parts.push(font_text("Word cards", "story", 0.6, INK).move_to(v2(0.0, 3.1)));
    parts.push(
        hand_text("tap to flip: word → meaning", 0.3, rgb8(140, 110, 90)).move_to(v2(0.0, 2.5)),
    );
    let cards = [
        (
            "diverged",
            "verb",
            "went in different directions from one point",
            "The river diverged into two streams.",
            rgb8(214, 96, 70),
        ),
        (
            "undergrowth",
            "noun",
            "small bushes and plants under tall trees",
            "A rabbit hid in the undergrowth.",
            rgb8(80, 150, 90),
        ),
        (
            "melancholy",
            "adjective",
            "quietly sad, often for a long time",
            "The grey evening felt melancholy.",
            rgb8(90, 110, 180),
        ),
        (
            "host",
            "noun",
            "a very large number of people or things",
            "A host of stars filled the sky.",
            rgb8(220, 160, 40),
        ),
    ];
    for (k, (w, pos, m, ex, col)) in cards.into_iter().enumerate() {
        let c = v2(-5.25 + 3.5 * k as f64, -0.5);
        let pop = ease_out_back(clamp01((t - 0.2 * k as f64) / 0.5));
        let flip = smoothstep(0.0, 1.0, clamp01((t - 2.0 - 0.7 * k as f64) / 0.7));
        parts.push(vocab_card(w, pos, m, ex, c, 3.1, 2.6, flip, col).scale(pop.max(0.01)));
    }
    group(parts)
}

// ── Act 5 ──
fn act_wind(t: f64, lay: &PoemLayout) -> Mobject {
    let r = frame_rect();
    let mut parts = vec![sky(r, rgb8(120, 160, 200), rgb8(230, 236, 230))];
    parts.push(layered_hills(r, &Palette::forest(), 4));
    let gust = 0.08 + 0.06 * (t * 2.2).sin();
    for (x, h) in [(-5.8, 2.6), (4.8, 3.0), (6.3, 2.2)] {
        parts.push(tree(
            v2(x, -2.3),
            h,
            TreeKind::Round,
            rgb8(70, 130, 70),
            gust,
            x as u64,
        ));
    }
    parts.push(
        sa::falling_leaves(
            Rect::new(v2(-7.5, -3.5), v2(7.5, 3.5)),
            t * 1.6,
            14,
            0.24,
            &[rgb8(90, 160, 70), rgb8(200, 180, 60), rgb8(220, 120, 50)],
            8,
        )
        .shift(v2(t * 0.6 - 2.0, 0.0)),
    );
    let page = Rect::new(v2(-3.9, -2.1), v2(3.9, 2.6));
    parts.push(paper_page(page));
    parts.push(title_block(
        "Who Has Seen the Wind?",
        "Christina Rossetti, 1872",
        v2(-3.4, 2.05),
        INK,
        t / 0.5,
    ));
    parts.push(reveal_poem(lay, (t - 0.4) / 3.2, Reveal::Handwriting, INK));
    if t > 3.8 {
        parts.push(rhyme_highlights(lay, (t - 3.8) / 1.4));
    }
    let tgt = lay.span_rect(3, 0, 4);
    parts.push(figure_callout(
        Figure::Personification,
        tgt,
        v2(0.0, -3.1),
        "the wind 'passes through' like a visitor",
        (t - 5.4) / 1.0,
    ));
    group(parts)
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(240, 228, 206));

    let frost = Poem::parse("The Road Not Taken", "Robert Frost", FROST);
    let lay1 = PoemLayout::new(&frost, "story", 0.32, v2(-6.3, 1.65), 1.75, 0.8);
    let ill = road_illustration();
    s.always_redraw(move |ctx| act_road(ctx.elapsed, &lay1, &ill));
    s.wait(13.0);
    s.remove_all();

    let kinney = Poem::parse("Rain on the Roof", "Coates Kinney", KINNEY);
    let lay2 = PoemLayout::new(&kinney, "courier", 0.3, v2(-6.2, 1.55), 2.2, 0.8);
    s.always_redraw(move |ctx| act_rain(ctx.elapsed, &lay2));
    s.wait(12.0);
    s.remove_all();

    let ww = Poem::parse(
        "I Wandered Lonely as a Cloud",
        "William Wordsworth",
        WORDSWORTH,
    );
    let lay3 = PoemLayout::new(&ww, "story", 0.32, v2(-6.4, 2.05), 1.6, 0.8);
    s.always_redraw(move |ctx| act_daffodils(ctx.elapsed, &lay3));
    s.wait(11.0);
    s.remove_all();

    s.always_redraw(|ctx| act_vocab(ctx.elapsed));
    s.wait(6.0);
    s.remove_all();

    let ros = Poem::parse("Who Has Seen the Wind?", "Christina Rossetti", ROSSETTI);
    let lay5 = PoemLayout::new(&ros, "story", 0.38, v2(-3.4, 1.3), 1.6, 0.8);
    s.always_redraw(move |ctx| act_wind(ctx.elapsed, &lay5));
    s.wait(8.0);
    run(&s, "poetry")
}
