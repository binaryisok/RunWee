//! Phase T1 — the storytelling engine: a retold Aesop fable (public domain) on a
//! layered landscape with rigged characters, timed narration, speech / thought /
//! shout bubbles and lip-flap; a picture-book spread with a drop capital and a
//! page turn; a comic page with poses and an iris-out; then day → night and an
//! ink-bleed ending. Every line of text here is original or public domain.
use runwee::prelude::story::*;
use runwee::prelude::*;
use runwee::story::bubble;
use std::sync::Arc;

const FW: f64 = 14.22;
const FH: f64 = 8.0;

fn frame_rect() -> Rect {
    Rect::from_center(v2(0.0, 0.0), FW, FH)
}

fn cheese(at: Vec2, s: f64) -> Mobject {
    group(vec![
        polygon(&[
            at + v2(-0.16, -0.08) * s,
            at + v2(0.18, -0.08) * s,
            at + v2(0.12, 0.1) * s,
        ])
        .fill(rgb8(255, 206, 72), 1.0)
        .stroke(rgb8(200, 140, 40), 1.5),
        circle(0.025 * s)
            .move_to(at + v2(0.04, -0.02) * s)
            .fill(rgb8(220, 160, 50), 1.0)
            .no_stroke(),
        circle(0.018 * s)
            .move_to(at + v2(-0.06, -0.04) * s)
            .fill(rgb8(220, 160, 50), 1.0)
            .no_stroke(),
    ])
}

/// Act 1 at local time `t` (also used as the content of the paper fold).
fn fable(t: f64, script: &StoryScript) -> Mobject {
    let pal = Palette::marigold();
    let rect = frame_rect();
    let ground = -2.75;
    let mut parts = vec![landscape(rect, &pal, t, 1.0, 4)];
    parts.push(tree(
        v2(-5.6, ground + 0.4),
        2.6,
        TreeKind::Pine,
        pal.ground[2].darker(0.1),
        0.02 * (t * 1.3).sin(),
        3,
    ));
    parts.push(house(
        v2(-3.8, ground + 0.55),
        1.2,
        rgb8(238, 214, 180),
        pal.accent[1],
        0.0,
    ));
    // the crow's tree with a sturdy branch
    let tree_x = 3.4;
    parts.push(tree(
        v2(tree_x, ground),
        4.6,
        TreeKind::Round,
        pal.ground[2],
        0.015 * (t * 1.1).sin(),
        8,
    ));
    parts.push(
        polygon(&[
            v2(tree_x, 0.25),
            v2(tree_x - 1.9, 0.62),
            v2(tree_x - 1.9, 0.52),
            v2(tree_x, 0.05),
        ])
        .fill(rgb8(112, 78, 56), 1.0)
        .no_stroke(),
    );
    let crow = Critter::crow().with_size(0.95);
    let crow_pos = v2(tree_x - 1.45, 0.58);
    let crow_talk = script.talking(1, t);
    let flap = if t > 10.5 && t < 11.6 { t * 4.0 } else { 0.0 };
    parts.push(crow.draw(t, crow_talk, flap, -1.0, crow_pos));
    // cheese in the beak, then falling after the CAW
    let beak = crow.mouth(-1.0, crow_pos) + v2(-0.08, -0.06);
    let fox_x = if t < 1.0 {
        -9.0
    } else {
        lerp(-9.0, -0.9, smoothstep(1.0, 4.6, t))
    };
    let fox = Critter::fox().with_size(1.05);
    let fox_pos = v2(fox_x, ground);
    let drop_t = clamp01((t - 10.9) / 0.7);
    let catch = fox.mouth(1.0, fox_pos) + v2(0.05, -0.02);
    let ch = if drop_t <= 0.0 {
        beak
    } else {
        let a = beak.lerp(catch, drop_t);
        a + v2(0.0, 0.6 * (PI * drop_t).sin())
    };
    let ch_rot = if drop_t > 0.0 { drop_t * 5.0 } else { 0.0 };
    parts.push(cheese(ch, 1.0).rotate_about(ch_rot, ch));
    let walking = t > 1.0 && t < 4.6;
    parts.push(fox.draw(
        t,
        script.talking(0, t),
        if walking { t * 1.6 } else { 0.0 },
        1.0,
        fox_pos,
    ));
    // grass tufts in the foreground
    let mut rng = Rng::new(77);
    let mut grass = vec![];
    for _ in 0..40 {
        let x = rng.range(-7.2, 7.2);
        let h = rng.range(0.12, 0.3);
        let lean = 0.06 * (t * 1.5 + x).sin();
        grass.push(
            polygon(&[
                v2(x - 0.04, ground - 0.4),
                v2(x + 0.04, ground - 0.4),
                v2(x + lean, ground - 0.4 + h),
            ])
            .fill(pal.ground[2].darker(0.15), 1.0)
            .no_stroke(),
        );
    }
    parts.push(group(grass));
    // narration and dialogue
    for (line, p) in script.active(t) {
        match line.speaker {
            None => parts.push(narration_box(
                &line.text,
                v2(0.0, 3.05),
                10.0,
                0.36,
                &pal,
                clamp01(p * 1.6),
            )),
            Some(0) => parts.push(bubble(
                line.kind,
                &line.text,
                v2(-2.6, 1.75),
                4.6,
                fox.mouth(1.0, fox_pos) + v2(0.1, 0.1),
                0.33,
                p,
            )),
            Some(_) => {
                let (c, w) = if line.kind == BubbleKind::Shout {
                    (v2(-0.2, 2.4), 2.0)
                } else {
                    (v2(-0.9, 2.75), 4.4)
                };
                parts.push(bubble(
                    line.kind,
                    &line.text,
                    c,
                    w,
                    crow.mouth(-1.0, crow_pos) + v2(-0.1, 0.12),
                    if line.kind == BubbleKind::Shout {
                        0.6
                    } else {
                        0.33
                    },
                    p,
                ))
            }
        }
    }
    group(parts)
}

fn storybook_page_left(pal: &Palette, r: Rect) -> Mobject {
    let ink = pal.ink;
    let title = Text::new("The Banyan by the Well")
        .font("story")
        .bold()
        .size(0.42)
        .color(ink)
        .build()
        .move_to(v2(r.center().x, r.max.y - 0.55));
    let rule = group(vec![
        line(
            v2(r.center().x - 1.6, r.max.y - 0.95),
            v2(r.center().x - 0.15, r.max.y - 0.95),
        )
        .stroke(pal.accent[0], 2.0),
        circle(0.05)
            .move_to(v2(r.center().x, r.max.y - 0.95))
            .fill(pal.accent[1], 1.0)
            .no_stroke(),
        line(
            v2(r.center().x + 0.15, r.max.y - 0.95),
            v2(r.center().x + 1.6, r.max.y - 0.95),
        )
        .stroke(pal.accent[0], 2.0),
    ]);
    let para = drop_cap_paragraph(
        "Every evening, Meera and her grandfather walked to the old banyan tree by the village well. \
         Its roots hung down like curtains, and the birds above argued about the day. \
         Grandfather would sit on the cool stone, and Meera would ask for one more story.",
        v2(r.min.x + 0.45, r.max.y - 1.3),
        r.width() - 0.9,
        0.27,
        ink,
        pal.accent[1],
    );
    let para2_lines = wrap_lines(
        "“Tell me about the crow who sang,” she said, and the old man laughed, because she already knew how it ended.",
        "story",
        0.27,
        &|_| r.width() - 0.9,
    );
    let para2 = text_block(
        &para2_lines,
        "story",
        0.27,
        ink,
        v2(r.min.x + 0.45, r.max.y - 3.55),
        1.35,
    );
    // a small folk-style vignette: a crow on a sprig between two rules
    let vy = r.min.y + 1.35;
    let vc = r.center().x;
    let vignette = group(vec![
        line(v2(vc - 2.2, vy), v2(vc - 0.7, vy)).stroke(pal.accent[0].with_alpha(0.7), 1.5),
        line(v2(vc + 0.7, vy), v2(vc + 2.2, vy)).stroke(pal.accent[0].with_alpha(0.7), 1.5),
        cubic_bezier(
            v2(vc - 0.6, vy - 0.15),
            v2(vc - 0.2, vy + 0.05),
            v2(vc + 0.2, vy - 0.2),
            v2(vc + 0.6, vy - 0.05),
        )
        .stroke(rgb8(112, 78, 56), 2.5)
        .fill(WHITE, 0.0),
        Critter::crow()
            .with_size(0.5)
            .draw(0.0, 0.0, 0.0, 1.0, v2(vc - 0.05, vy - 0.13)),
        circle(0.05)
            .move_to(v2(vc + 0.45, vy - 0.02))
            .fill(pal.accent[1], 1.0)
            .no_stroke(),
        circle(0.04)
            .move_to(v2(vc - 0.45, vy - 0.08))
            .fill(pal.accent[0], 1.0)
            .no_stroke(),
    ]);
    let num = story_italic("— 12 —", 0.22, ink.mix(pal.paper, 0.4))
        .move_to(v2(r.center().x, r.min.y + 0.3));
    group(vec![title, rule, para, para2, vignette, num])
}

fn storybook_page_right(r: Rect, t: f64, day: f64, caption: &str) -> Mobject {
    let pal = Palette::dusk();
    let ill = Rect::from_center(
        r.center() + v2(0.0, 0.35),
        r.width() - 0.7,
        r.height() - 1.5,
    );
    let ground = ill.min.y + 0.55;
    let mut scene = vec![landscape(ill, &pal, t, day, 21)];
    scene.push(tree(
        v2(ill.center().x + 1.0, ground - 0.1),
        2.9,
        TreeKind::Banyan,
        rgb8(104, 150, 92).mix(rgb8(40, 60, 70), (1.0 - day) as f32 * 0.6),
        0.02 * (t * 1.2).sin(),
        5,
    ));
    scene.push(house(
        v2(ill.min.x + 0.8, ground + 0.15),
        0.8,
        rgb8(236, 210, 190),
        pal.accent[1],
        1.0 - day,
    ));
    let gp = Character::grandpa().with_height(1.55);
    scene.push(
        rounded_rectangle(0.9, 0.3, 0.06)
            .move_to(v2(ill.center().x + 0.55, ground + 0.05))
            .fill(rgb8(150, 140, 130), 1.0)
            .no_stroke(),
    );
    scene.push(gp.draw_at(
        &Pose::sit(),
        t,
        0.0,
        -1.0,
        v2(ill.center().x + 0.55, ground + 0.2),
    ));
    let girl = Character::girl().with_height(1.35);
    scene.push(girl.draw_at(
        &Pose::wave(t),
        t + 0.7,
        0.0,
        1.0,
        v2(ill.center().x - 0.7, ground - 0.15),
    ));
    let framed = group(vec![
        clip_to(group(scene), ill),
        rectangle(ill.width(), ill.height())
            .move_to(ill.center())
            .fill(WHITE, 0.0)
            .stroke(rgb8(90, 70, 60), 2.0),
    ]);
    let cap =
        story_italic(caption, 0.25, rgb8(70, 50, 50)).move_to(v2(r.center().x, ill.min.y - 0.35));
    group(vec![framed, cap])
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(64, 44, 36));

    // ── Act 1: The Crow and the Fox (0 – 14 s) ──
    let script = Arc::new(
        StoryScript::new()
            .narrate(0.0, 4.4, "One sunny morning, a crow found a piece of cheese and flew up to a branch to enjoy it.")
            .say(0, 4.6, 8.2, BubbleKind::Speech, "Good morning, Crow! Your feathers shine so bright — surely your voice is just as lovely?")
            .say(1, 8.3, 10.4, BubbleKind::Thought, "Nobody has ever praised my singing before…")
            .say(1, 10.5, 12.4, BubbleKind::Shout, "CAW!"),
    );
    {
        let sc = script.clone();
        s.always_redraw(move |ctx| {
            let t = ctx.elapsed;
            let scene = fable(t.min(12.4), &sc);
            let p = clamp01((t - 12.4) / 1.5);
            if p <= 0.0 {
                scene
            } else {
                paper_fold(&scene, frame_rect(), p, 6)
            }
        });
    }
    s.wait(14.0);
    s.remove_all();

    // ── Act 2: a picture-book spread with a page turn (14 – 24 s) ──
    let paper = rgb8(250, 242, 224);
    let (book, lp, rp) = open_book(
        Rect::from_center(v2(0.0, -0.05), 13.4, 7.5),
        paper,
        rgb8(128, 44, 52),
    );
    let grain_l = Arc::new(paper_grain(lp, 3, 360));
    let grain_r = Arc::new(paper_grain(rp, 4, 360));
    s.add(book);
    s.add(storybook_page_left(&Palette::marigold(), lp));
    s.add((*grain_l).clone());
    {
        let gr = grain_r.clone();
        s.always_redraw(move |ctx| {
            let t = ctx.elapsed;
            let cur = group(vec![
                storybook_page_right(rp, t, 1.0, "Meera waves to the evening birds."),
                (*gr).clone(),
            ]);
            let p = clamp01((t - 5.0) / 2.6);
            if p <= 0.0 {
                return cur;
            }
            let next = group(vec![
                storybook_page_right(rp, t, 0.0, "That night the stars came out, one by one."),
                (*gr).clone(),
            ]);
            page_turn(&cur, &next, rp, p, paper)
        });
    }
    s.wait(10.0);
    s.remove_all();

    // ── Act 3: a comic page (24 – 34 s) ──
    s.set_background(rgb8(246, 240, 228));
    let page = Rect::from_center(v2(0.0, 0.0), 13.6, 7.5);
    let panels = comic_panels(page, 2, 2, 0.22);
    let ink = rgb8(30, 26, 30);
    s.always_redraw(move |ctx| {
        let t = ctx.elapsed;
        let pals = [
            Palette::marigold(),
            Palette::forest(),
            Palette::monsoon(),
            Palette::dusk(),
        ];
        let mut parts = vec![];
        for (i, r) in panels.iter().enumerate() {
            let pal = pals[i];
            let local = (t - i as f64 * 1.4).max(0.0);
            let ground = r.min.y + 0.45;
            let mut sc = vec![
                sky(*r, pal.sky[0], pal.sky[1]),
                hills(*r, r.min.y + 0.9, 0.25, pal.ground[1], i as u64 + 2),
                hills(*r, r.min.y + 0.45, 0.15, pal.ground[2], i as u64 + 9),
            ];
            let cx = r.center().x;
            let (who, pose, text, kind, cap): (Character, Pose, &str, BubbleKind, &str) = match i {
                0 => (
                    Character::girl(),
                    Pose::wave(t),
                    "Hi! I'm Meera.",
                    BubbleKind::Speech,
                    "Monday morning…",
                ),
                1 => (
                    Character::boy(),
                    Pose::think(),
                    "What rhymes with “tree”?",
                    BubbleKind::Thought,
                    "In class",
                ),
                2 => (
                    Character::teacher(),
                    Pose::point(),
                    "“Bee”, “free”, “sea”…",
                    BubbleKind::Whisper,
                    "A hint",
                ),
                _ => (
                    Character::boy(),
                    Pose::cheer(t),
                    "I've got it!",
                    BubbleKind::Shout,
                    "Later",
                ),
            };
            let pos = v2(cx - 1.6, ground - 0.1);
            if i == 1 {
                sc.push(tree(
                    v2(cx + 2.0, ground),
                    2.4,
                    TreeKind::Round,
                    pal.ground[2],
                    0.03 * (t * 1.5).sin(),
                    4,
                ));
            }
            if i == 2 {
                // a small chalkboard
                let b = Rect::from_center(v2(cx + 1.5, r.center().y - 0.45), 2.3, 1.3);
                sc.push(
                    rounded_rectangle(b.width(), b.height(), 0.05)
                        .move_to(b.center())
                        .fill(rgb8(40, 70, 60), 1.0)
                        .stroke(rgb8(150, 110, 70), 5.0),
                );
                sc.push(
                    hand_text("tree · bee · sea", 0.3, rgb8(240, 240, 230)).move_to(b.center()),
                );
            }
            let talk = if kind == BubbleKind::Thought {
                0.0
            } else {
                smoothstep(0.4, 0.5, local) * (1.0 - smoothstep(1.8, 1.9, local))
            };
            sc.push(
                who.clone()
                    .with_height(2.0)
                    .draw_at(&pose, t, talk, 1.0, pos),
            );
            let j = who.with_height(2.0).joints(&pose, pos, 1.0);
            if local > 0.0 {
                sc.push(bubble(
                    kind,
                    text,
                    v2(cx + 1.15, r.max.y - 1.15),
                    3.2,
                    j.mouth + v2(0.25, 0.25),
                    if kind == BubbleKind::Shout {
                        0.42
                    } else {
                        0.32
                    },
                    clamp01(local / 2.2),
                ));
            }
            sc.push(panel_caption(*r, cap, 0.24, rgb8(255, 236, 170), ink));
            parts.push(clip_to(group(sc), *r));
            parts.push(panel_frame(*r, ink));
        }
        let page_m = group(parts);
        let p = clamp01((t - 7.6) / 2.0);
        if p > 0.0 {
            let c = panels[3].center() + v2(-1.6, 0.9);
            group(vec![page_m, iris(p, c, rgb8(16, 14, 20))])
        } else {
            page_m
        }
    });
    s.wait(10.0);
    s.remove_all();

    // ── Act 4: day → night, then an ink-bleed ending (34 – 44 s) ──
    s.set_background(rgb8(16, 14, 20));
    s.always_redraw(move |ctx| {
        let t = ctx.elapsed;
        let pal = Palette::forest();
        let rect = frame_rect();
        let day = 1.0 - smoothstep(0.5, 4.5, t);
        let ground = -2.6;
        let mut parts = vec![landscape(rect, &pal, t, day, 31)];
        parts.push(tree(
            v2(-4.8, ground),
            4.0,
            TreeKind::Banyan,
            pal.ground[2].mix(rgb8(30, 40, 70), (1.0 - day) as f32 * 0.6),
            0.02 * (t * 1.2).sin(),
            6,
        ));
        parts.push(house(
            v2(4.6, ground + 0.1),
            2.0,
            rgb8(238, 214, 180).mix(rgb8(90, 90, 130), (1.0 - day) as f32 * 0.6),
            rgb8(170, 70, 60).mix(rgb8(60, 40, 70), (1.0 - day) as f32 * 0.5),
            1.0 - day,
        ));
        let x = lerp(-3.0, 3.2, smoothstep(0.3, 6.5, t));
        let walking = t > 0.3 && t < 6.5;
        let pose = if walking {
            Pose::walk(t * 0.9)
        } else {
            Pose::idle(t)
        };
        let girl = Character::girl().with_height(1.8);
        parts.push(girl.draw_at(&pose, t, 0.0, 1.0, v2(x, ground - 0.3)));
        let gp = Character::grandpa().with_height(1.95);
        parts.push(gp.draw_at(
            &if walking {
                Pose::walk(t * 0.9 + 0.3)
            } else {
                Pose::idle(t)
            },
            t + 0.4,
            0.0,
            1.0,
            v2(x - 1.2, ground - 0.25),
        ));
        if t > 1.0 && t < 5.0 {
            parts.push(narration_box(
                "They walked home as the sky turned from gold to indigo.",
                v2(0.0, 3.1),
                9.0,
                0.34,
                &Palette::dusk(),
                clamp01((t - 1.0) / 2.0),
            ));
        }
        let bleed = clamp01((t - 6.6) / 2.2);
        if bleed > 0.0 {
            parts.push(ink_bleed(bleed, rect, rgb8(28, 24, 44), 12, 640));
        }
        let end_p = clamp01((t - 8.4) / 0.8);
        if end_p > 0.0 {
            parts.push(
                Text::new("The End")
                    .font("story")
                    .italic()
                    .size(0.9)
                    .color(rgb8(250, 236, 200))
                    .build()
                    .move_to(v2(0.0, 0.35))
                    .opacity(end_p as f32),
            );
            parts.push(
                hand_text(
                    "Moral: think twice when someone flatters you.",
                    0.36,
                    rgb8(255, 200, 120),
                )
                .move_to(v2(0.0, -0.6))
                .opacity(end_p as f32),
            );
            let gold = rgb8(230, 180, 90);
            let mut orn = vec![];
            for side in [-1.0, 1.0] {
                orn.push(line(v2(side * 0.9, 1.45), v2(side * 3.2, 1.45)).stroke(gold, 1.6));
                orn.push(line(v2(side * 0.9, -1.25), v2(side * 3.2, -1.25)).stroke(gold, 1.6));
                orn.push(
                    star(5, 0.09, 0.04)
                        .move_to(v2(side * 0.5, 1.45))
                        .fill(gold, 1.0)
                        .no_stroke(),
                );
            }
            orn.push(
                star(6, 0.12, 0.05)
                    .move_to(v2(0.0, 1.45))
                    .fill(gold, 1.0)
                    .no_stroke(),
            );
            orn.push(
                Critter::crow()
                    .with_size(0.55)
                    .draw(0.0, 0.0, 0.0, 1.0, v2(-0.2, -1.55)),
            );
            parts.push(group(orn).opacity(end_p as f32));
        }
        group(parts)
    });
    s.wait(10.0);
    run(&s, "story_engine")
}
