//! Phase T2 — story art styles: the same kind of village story drawn six ways —
//! paper cutout, watercolor monsoon, ink & brush, shadow-puppet theatre, a triptych
//! of Indian folk-art frames (Madhubani, Warli, Gond) and a flat-vector day → night
//! cycle with fireflies. All text is original.
use runwee::prelude::story::*;
use runwee::prelude::*;
use runwee::storyart as sa;

const FW: f64 = 14.22;
const FH: f64 = 8.0;

fn frame_rect() -> Rect {
    Rect::from_center(v2(0.0, 0.0), FW, FH)
}

fn umbrella(handle: Vec2, r: f64, color: Color) -> Mobject {
    let top = handle + v2(0.0, r * 1.25);
    let mut scallops = vec![];
    for k in 0..4 {
        let cx = top.x - r + r * 0.25 + r * 0.5 * k as f64;
        scallops.push(
            circle(r * 0.25)
                .move_to(v2(cx, top.y))
                .fill(color.darker(0.15), 1.0)
                .no_stroke(),
        );
    }
    group(vec![
        line(handle, top + v2(0.0, r * 0.95)).stroke(rgb8(60, 40, 30), 3.0),
        arc(r * 0.12, PI, PI)
            .move_to(handle - v2(r * 0.06, r * 0.05))
            .fill(BLACK, 0.0)
            .stroke(rgb8(60, 40, 30), 3.0),
        group(scallops),
        sector(r, 0.0, PI).shift(top).fill(color, 1.0).no_stroke(),
    ])
}

// ── Act 1: paper cutout ─────────────────────────────────────────────────────
fn cutout_act(t: f64, fibres: &Mobject) -> Mobject {
    let r = frame_rect();
    let cs = sa::CutoutStyle::default();
    let mut parts = vec![
        sky(r, rgb8(255, 200, 150), rgb8(255, 236, 205)),
        sa::cutout(
            star(18, 1.05, 0.82)
                .rotate(t * 0.15)
                .move_to(v2(4.4, 2.5))
                .fill(rgb8(250, 190, 80), 1.0)
                .no_stroke(),
            cs.lift(0.05),
        ),
        sa::cutout(
            circle(0.72)
                .move_to(v2(4.4, 2.5))
                .fill(rgb8(244, 140, 50), 1.0)
                .no_stroke(),
            cs.seed(2).lift(0.06),
        ),
    ];
    for (i, (x, y, w)) in [(-4.5, 2.6, 2.2), (1.0, 3.1, 1.6), (6.6, 1.7, 1.8)]
        .into_iter()
        .enumerate()
    {
        let dx = ((t * 0.18 + i as f64 * 0.3) * 1.4).rem_euclid(FW + 3.0) - 1.5;
        parts.push(sa::cutout(
            cloud(v2(x + dx - 1.0, y), w, rgb8(255, 250, 240)),
            cs.seed(i as u64 + 1).lift(0.05),
        ));
    }
    parts.push(sa::cut_paper_hills(
        r,
        &[
            rgb8(240, 170, 120),
            rgb8(214, 120, 90),
            rgb8(170, 160, 80),
            rgb8(108, 140, 70),
        ],
        8,
    ));
    let ground = -2.4;
    parts.push(sa::cutout(
        tree(
            v2(-5.2, ground + 0.2),
            2.6,
            TreeKind::Round,
            rgb8(70, 130, 70),
            0.02 * (t * 1.2).sin(),
            4,
        ),
        cs.seed(9).lift(0.07),
    ));
    parts.push(sa::cutout(
        tree(
            v2(5.6, ground + 0.1),
            2.2,
            TreeKind::Pine,
            rgb8(40, 100, 80),
            0.0,
            5,
        ),
        cs.seed(10).lift(0.07),
    ));
    parts.push(sa::cutout(
        house(
            v2(-2.9, ground + 0.45),
            1.4,
            rgb8(250, 226, 180),
            rgb8(196, 64, 50),
            0.0,
        ),
        cs.seed(12).lift(0.06),
    ));
    let x = -1.5 + t * 0.55;
    let girl = Character::girl().draw_at(&Pose::walk(t * 0.9), t, 0.0, 1.0, v2(x, ground - 0.3));
    parts.push(sa::cutout(girl, cs.wobble(0.006).lift(0.08)));
    parts.push(sa::cutout(
        sa::falling_leaves(
            r,
            t,
            9,
            0.38,
            &[rgb8(230, 120, 40), rgb8(240, 180, 60), rgb8(190, 70, 50)],
            3,
        ),
        cs.wobble(0.0).lift(0.05),
    ));
    parts.push(sa::torn_strip(
        Rect::new(v2(-FW / 2.0, -FH / 2.0), v2(FW / 2.0, -3.25)),
        rgb8(120, 80, 60),
        6,
    ));
    parts.push(
        hand_text(
            "Paper cutout — every layer floats on its own shadow",
            0.3,
            rgb8(255, 240, 220),
        )
        .move_to(v2(0.0, -3.62)),
    );
    parts.push(fibres.clone().blend(Blend::Multiply));
    parts.push(sa::style_tag(
        "1 · Paper cutout",
        v2(-5.3, 3.4),
        rgb8(255, 248, 232),
        rgb8(120, 60, 40),
    ));
    group(parts)
}

// ── Act 2: watercolor monsoon ───────────────────────────────────────────────
fn watercolor_act(t: f64, bg: &Mobject) -> Mobject {
    let r = frame_rect();
    let ground = -2.3;
    let mut parts = vec![bg.clone()];
    let x = -3.0 + t * 0.6;
    let boy = Character::boy().draw_at(&Pose::walk(t * 0.85), t, 0.0, 1.0, v2(x, ground - 0.6));
    let hand = Character::boy()
        .joints(&Pose::walk(t * 0.85), v2(x, ground - 0.6), 1.0)
        .front_hand;
    parts.push(boy);
    parts.push(umbrella(hand, 0.75, rgb8(214, 64, 70)));
    parts.push(sa::rain(r, t, 240, rgb8(225, 235, 250), 0.18, 5));
    parts.push(sa::splashes(
        -FW / 2.0,
        FW / 2.0,
        ground - 0.6,
        t,
        26,
        rgb8(230, 240, 255),
        7,
    ));
    parts.push(
        hand_text(
            "Watercolor — glazes, blooms and granulating washes",
            0.3,
            rgb8(40, 60, 90),
        )
        .move_to(v2(0.0, -3.6)),
    );
    parts.push(sa::style_tag(
        "2 · Watercolor monsoon",
        v2(-4.9, 3.4),
        rgb8(240, 246, 252),
        rgb8(40, 70, 110),
    ));
    group(parts)
}

fn watercolor_bg() -> Mobject {
    let r = frame_rect();
    let paper = rgb8(250, 247, 240);
    let ground = -2.3;
    let mut parts = vec![
        rectangle(FW, FH).fill(paper, 1.0).no_stroke(),
        sa::watercolor_wash(
            r.expand(-0.15),
            rgb8(110, 140, 175),
            rgb8(200, 214, 220),
            3,
            720,
        ),
    ];
    let hill = |base: f64, amp: f64, seed: u64| hills(r, base, amp, WHITE, seed);
    parts.push(sa::watercolor(
        hill(0.4, 0.55, 2),
        rgb8(120, 150, 150),
        6,
        11,
    ));
    parts.push(sa::watercolor(
        hill(-0.8, 0.4, 3),
        rgb8(90, 140, 100),
        6,
        12,
    ));
    parts.push(sa::watercolor(
        hill(ground - 0.1, 0.25, 4),
        rgb8(80, 120, 70),
        6,
        13,
    ));
    for (i, (x, y, rr)) in [
        (-5.4, -0.2, 0.95),
        (-4.4, 0.25, 0.75),
        (4.8, 0.0, 1.0),
        (5.9, -0.4, 0.7),
        (3.9, -0.5, 0.6),
    ]
    .into_iter()
    .enumerate()
    {
        parts.push(
            rectangle(0.16, 1.6)
                .move_to(v2(x, y - rr - 0.5))
                .fill(rgb8(110, 80, 60), 0.75)
                .no_stroke(),
        );
        parts.push(sa::watercolor(
            circle(rr).move_to(v2(x, y)),
            rgb8(60, 120, 80),
            7,
            20 + i as u64,
        ));
    }
    parts.push(sa::watercolor(
        polygon(&[
            v2(-2.6, ground + 0.1),
            v2(-1.2, ground + 0.1),
            v2(-1.2, ground + 1.1),
            v2(-2.6, ground + 1.1),
        ]),
        rgb8(214, 170, 120),
        5,
        31,
    ));
    parts.push(sa::watercolor(
        polygon(&[
            v2(-2.85, ground + 1.05),
            v2(-0.95, ground + 1.05),
            v2(-1.9, ground + 1.8),
        ]),
        rgb8(170, 70, 60),
        5,
        32,
    ));
    parts.push(
        sa::paper_fibres(r, WHITE, 2, 720)
            .blend(Blend::Multiply)
            .opacity(0.6),
    );
    group(parts)
}

// ── Act 3: ink & brush ──────────────────────────────────────────────────────
fn ink_act(t: f64, paper: &Mobject) -> Mobject {
    let r = frame_rect();
    let ink = rgb8(26, 24, 28);
    let mut parts = vec![paper.clone()];
    parts.push(
        circle(0.62)
            .move_to(v2(4.9, 1.0))
            .fill(rgb8(214, 70, 50), 0.85)
            .no_stroke(),
    );
    parts.push(sa::ink_mountains(
        Rect::new(v2(-FW / 2.0, -FH / 2.0), v2(FW / 2.0, 0.8)),
        4,
        ink,
        7,
        clamp01(t / 3.2),
    ));
    // a bough of pine painted stroke by stroke
    let bp = clamp01((t - 2.6) / 1.4);
    parts.push(sa::brush_stroke(
        &[
            v2(-7.3, 3.1),
            v2(-5.8, 2.75),
            v2(-4.3, 2.85),
            v2(-2.6, 2.45),
        ],
        0.24,
        ink,
        0.4,
        3,
        bp,
    ));
    for k in 0..6u64 {
        let base = v2(
            -6.8 + 0.78 * k as f64,
            2.83 - 0.05 * k as f64 + 0.06 * (k % 2) as f64,
        );
        let p = clamp01((t - 3.2 - 0.15 * k as f64) / 0.45);
        // a radiating fan of needles, like a sumi-e pine cluster
        for j in 0..9u64 {
            let a = PI * 0.08 + PI * 0.84 * j as f64 / 8.0;
            let len = 0.42 + 0.1 * sa::rnd(k as i64, j as i64, 4);
            let tip = base + Vec2::polar(len, a);
            parts.push(sa::brush_stroke(
                &[base, base.lerp(tip, 0.5) + v2(0.02, -0.02), tip],
                0.045,
                ink,
                0.0,
                30 + j + k * 11,
                p,
            ));
        }
    }
    // a crow outlined with the ink brush, grey wash inside
    let crow = Critter::crow()
        .with_size(1.3)
        .draw(t, 0.0, 0.0, -1.0, v2(-3.5, 2.68));
    let cp = clamp01((t - 4.0) / 0.8) as f32;
    parts.push(sa::silhouette(crow.clone(), rgb8(60, 58, 64)).opacity(cp * 0.85));
    parts.push(sa::ink_outline(&crow, 0.035, ink, 5).opacity(cp));
    let lines = [
        "Mist on the far hills —",
        "one crow on a pine bough",
        "keeps the morning still.",
    ];
    for (i, l) in lines.iter().enumerate() {
        let p = clamp01((t - 1.0 - 0.8 * i as f64) / 1.4);
        let txt = typed(l, p);
        parts.push(
            font_text(&txt, "story", 0.42, ink)
                .move_to_aligned(v2(0.2, 2.75 - 0.6 * i as f64), v2(-1.0, 0.0)),
        );
    }
    parts.push(
        sa::seal_stamp("RW", v2(5.9, -2.9), 0.6, rgb8(196, 40, 36), 4)
            .opacity(clamp01((t - 4.6) / 0.3) as f32),
    );
    parts.push(sa::style_tag(
        "3 · Ink & brush",
        v2(5.4, 3.4),
        rgb8(250, 246, 236),
        ink,
    ));
    let _ = r;
    group(parts)
}

// ── Act 4: shadow puppets ───────────────────────────────────────────────────
fn puppet_act(t: f64) -> Mobject {
    let screen = Rect::new(v2(-5.6, -2.6), v2(5.6, 3.0));
    let warm = rgb8(236, 150, 60);
    let sil = rgb8(30, 16, 10);
    let mut parts = vec![
        rectangle(FW, FH).fill(rgb8(26, 14, 10), 1.0).no_stroke(),
        sa::lamp_screen(screen, t, warm, 3),
    ];
    let floor = screen.min.y + 0.35;
    let tr = tree(
        v2(3.6, floor),
        3.2,
        TreeKind::Banyan,
        sil,
        0.015 * (t * 1.1).sin(),
        3,
    );
    parts.push(sa::screen_blur(sa::silhouette(tr, sil), sil, 0.35));
    let x = -4.4 + t * 0.75;
    let pose = Pose::walk(t * 0.8);
    let girl_c = Character::girl().with_height(2.0);
    let girl = girl_c.draw_at(&pose, t, 0.0, 1.0, v2(x, floor));
    let j = girl_c.joints(&pose, v2(x, floor), 1.0);
    parts.push(sa::screen_blur(sa::silhouette(girl, sil), sil, 0.05));
    let fx = 2.0 - (t - 3.0).max(0.0) * 0.2;
    let fox = Critter::fox().with_size(1.6).draw(
        t,
        if t > 4.5 { 1.0 } else { 0.0 },
        0.6,
        -1.0,
        v2(fx, floor + 0.05),
    );
    parts.push(sa::screen_blur(sa::silhouette(fox, sil), sil, 0.12));
    let bird = Critter::crow().with_size(0.9).draw(
        t,
        0.0,
        1.0,
        -1.0,
        v2(4.6 - t * 0.9, 2.2 + 0.2 * (t * 2.0).sin()),
    );
    parts.push(sa::screen_blur(sa::silhouette(bird, sil), sil, 0.2));
    parts.push(sa::puppet_rods(
        &[j.front_hand, j.back_hand],
        screen.min.y,
        rgb8(60, 34, 20),
    ));
    // the audience's row of heads
    for k in 0..9 {
        let hx = -6.4 + 1.6 * k as f64 + 0.2 * (k as f64 * 1.7).sin();
        parts.push(
            ellipse(1.1, 1.2)
                .move_to(v2(hx, -3.95))
                .fill(rgb8(12, 8, 6), 1.0)
                .no_stroke(),
        );
    }
    parts.push(sa::style_tag(
        "4 · Shadow puppets",
        v2(-5.1, 3.55),
        rgb8(250, 232, 200),
        rgb8(80, 40, 20),
    ));
    group(parts)
}

// ── Act 5: folk-art triptych ────────────────────────────────────────────────
fn folk_act(t: f64) -> Mobject {
    let mut parts = vec![rectangle(FW, FH).fill(rgb8(246, 236, 214), 1.0).no_stroke()];
    let w = 4.3;
    let h = 6.0;
    let band = 0.52;
    let titles = ["Madhubani", "Warli", "Gond"];
    for (i, style) in [
        sa::FolkStyle::Madhubani,
        sa::FolkStyle::Warli,
        sa::FolkStyle::Gond,
    ]
    .into_iter()
    .enumerate()
    {
        let cx = -4.6 + 4.6 * i as f64;
        let rect = Rect::from_center(v2(cx, 0.35), w, h);
        let pop = ease_out_back(clamp01((t - 0.3 * i as f64) / 0.6));
        let inner = rect.expand(-band);
        let mut panel = vec![];
        match style {
            sa::FolkStyle::Madhubani => {
                panel.push(
                    rectangle(inner.width(), inner.height())
                        .move_to(inner.center())
                        .fill(rgb8(252, 242, 214), 1.0)
                        .no_stroke(),
                );
                // a pond of fish circling a lotus
                for k in 0..3 {
                    let a = t * 0.6 + TAU * k as f64 / 3.0;
                    let p = inner.center() + v2(1.05 * a.cos(), 1.6 * a.sin());
                    let col = [rgb8(214, 54, 44), rgb8(244, 186, 40), rgb8(36, 92, 168)][k];
                    panel.push(
                        sa::madhubani_fish(1.0, col, rgb8(30, 22, 20))
                            .rotate_about(a + PI / 2.0, v2(0.0, 0.0))
                            .shift(p),
                    );
                }
                panel.push(
                    sa::madhubani_lotus(
                        0.55,
                        10,
                        rgb8(236, 112, 150),
                        rgb8(244, 186, 40),
                        rgb8(30, 22, 20),
                    )
                    .rotate_about(t * 0.2, v2(0.0, 0.0))
                    .shift(inner.center()),
                );
            }
            sa::FolkStyle::Warli => {
                panel.push(
                    rectangle(inner.width(), inner.height())
                        .move_to(inner.center())
                        .fill(rgb8(132, 54, 34), 1.0)
                        .no_stroke(),
                );
                panel.push(sa::warli_circle_dance(
                    inner.center() + v2(0.0, 0.2),
                    1.45,
                    2,
                    t,
                    rgb8(250, 242, 228),
                ));
            }
            sa::FolkStyle::Gond => {
                panel.push(
                    rectangle(inner.width(), inner.height())
                        .move_to(inner.center())
                        .fill(rgb8(252, 246, 232), 1.0)
                        .no_stroke(),
                );
                panel.push(sa::gond_tree(
                    v2(inner.center().x, inner.min.y + 0.15),
                    4.4,
                    t,
                    9,
                ));
            }
        }
        panel.push(sa::folk_frame(style, rect, band, t, 5));
        let p = group(panel)
            .scale(0.4 + 0.6 * pop)
            .opacity(clamp01(pop * 2.0) as f32);
        parts.push(p);
        parts.push(
            font_text(titles[i], "story", 0.36, rgb8(80, 40, 30))
                .move_to(v2(cx, -3.15))
                .opacity(clamp01((t - 0.6 - 0.3 * i as f64) / 0.4) as f32),
        );
    }
    parts.push(sa::style_tag(
        "5 · Folk-art frames",
        v2(0.0, 3.68),
        rgb8(255, 248, 232),
        rgb8(120, 50, 30),
    ));
    group(parts)
}

// ── Act 6: flat vector, day → night, fireflies ──────────────────────────────
fn night_act(t: f64) -> Mobject {
    let r = frame_rect();
    let phase = 0.55 + 0.5 * clamp01(t / 5.0);
    let (top, hor) = sa::sky_colors(phase);
    let dark = clamp01((phase - 0.72) / 0.18);
    let mut parts = vec![sky(r, top, hor)];
    if dark > 0.0 {
        for k in 0..40 {
            let p = v2(-7.0 + 14.0 * sa::rnd(k, 0, 3), 0.5 + 3.4 * sa::rnd(k, 1, 3));
            let tw = 0.6 + 0.4 * (t * 3.0 + k as f64).sin();
            parts.push(
                star(5, 0.05, 0.02)
                    .move_to(p)
                    .fill(WHITE, (dark * tw) as f32)
                    .no_stroke(),
            );
        }
        parts.push(moon(v2(-4.6, 2.7), 0.5, rgb8(255, 246, 214)).opacity(dark as f32));
    }
    let sun_y = 2.6 - 4.5 * clamp01(t / 3.6);
    parts.push(sun(v2(4.4, sun_y), 0.6, rgb8(255, 200, 90)));
    let pal = Palette::forest();
    parts.push(layered_hills(r, &pal, 12));
    let ground = -2.5;
    let h = house(
        v2(-2.0, ground + 0.6),
        1.6,
        rgb8(244, 220, 180),
        rgb8(200, 80, 60),
        dark,
    );
    parts.push(sa::long_shadow(
        &h,
        -0.6,
        0.9,
        BLACK.with_alpha(0.06 * (1.0 - dark) as f32),
    ));
    parts.push(h);
    for (x, s) in [(-5.4, 2.4), (3.0, 2.8), (5.4, 2.0)] {
        let tr = tree(
            v2(x, ground + 0.2),
            s,
            TreeKind::Round,
            pal.ground[2].darker(0.15),
            0.02 * (t + x).sin(),
            x as u64,
        );
        parts.push(sa::long_shadow(
            &tr,
            -0.6,
            0.7,
            BLACK.with_alpha(0.05 * (1.0 - dark) as f32),
        ));
        parts.push(tr);
    }
    parts.push(sa::night_overlay(r, dark));
    if dark > 0.0 {
        let ff = sa::fireflies(
            Rect::new(v2(-6.5, -3.5), v2(6.5, 0.8)),
            t,
            40,
            rgb8(220, 255, 120),
            4,
        );
        parts.push(ff.opacity(dark as f32));
    }
    parts.push(
        hand_text(
            "Flat vector · day → night · fireflies",
            0.32,
            WHITE.mix(rgb8(255, 240, 200), 0.5),
        )
        .move_to(v2(0.0, -3.6)),
    );
    parts.push(sa::style_tag(
        "6 · Day to night",
        v2(-5.3, 3.4),
        rgb8(250, 246, 236),
        rgb8(40, 50, 90),
    ));
    group(parts)
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(246, 236, 214));

    let fibres = sa::paper_fibres(frame_rect(), WHITE, 9, 960);
    s.always_redraw(move |ctx| cutout_act(ctx.elapsed, &fibres));
    s.wait(7.0);
    s.remove_all();

    let bg = watercolor_bg();
    s.always_redraw(move |ctx| watercolor_act(ctx.elapsed, &bg));
    s.wait(7.0);
    s.remove_all();

    let paper = sa::paper_fibres(frame_rect(), rgb8(246, 238, 220), 21, 960);
    s.always_redraw(move |ctx| ink_act(ctx.elapsed, &paper));
    s.wait(7.0);
    s.remove_all();

    s.always_redraw(|ctx| puppet_act(ctx.elapsed));
    s.wait(7.0);
    s.remove_all();

    s.always_redraw(|ctx| folk_act(ctx.elapsed));
    s.wait(7.0);
    s.remove_all();

    s.always_redraw(|ctx| night_act(ctx.elapsed));
    s.wait(7.0);
    run(&s, "storyart")
}
