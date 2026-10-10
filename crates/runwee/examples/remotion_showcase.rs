//! Remotion-style composition: a Movie of scenes joined by transitions, a
//! functional (frame = f(t)) intro with springs, rough-notation annotations,
//! starburst + light leaks, SVG import and frame effects.
use runwee::prelude::*;

const LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 120">
  <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0" stop-color="#58C4DD"/><stop offset="1" stop-color="#9A72AC"/></linearGradient></defs>
  <circle cx="60" cy="60" r="54" fill="url(#g)"/>
  <path d="M38 80 L60 30 L82 80 Z" fill="none" stroke="#fff" stroke-width="7" stroke-linejoin="round"/>
</svg>"##;

fn intro() -> FnComposition {
    // Every frame is computed from t, like a Remotion component.
    FnComposition::new(3.0, |f| {
        let burst = starburst(14, &[rgb8(24, 32, 58), rgb8(16, 20, 38)], f.t * 0.15, 12.0);
        let s = spring(f.frame, f.fps, SpringConfig::bouncy(), 0.0, 1.0);
        let logo = svg_from_str(LOGO, 2.2)
            .unwrap()
            .scale(s.max(0.01))
            .shift(UP * 0.6);
        let title_y = interpolate(f.t, &[0.4, 1.2], &[-4.5, -1.6]);
        let title = bold_text("RunWee", 1.0, WHITE).move_to(v2(0.0, title_y));
        let sub_op = interpolate(f.t, &[1.0, 1.8], &[0.0, 1.0]) as f32;
        let sub = text_sized(
            "manim + Remotion, in Rust",
            0.4,
            interpolate_colors(f.t, &[1.0, 2.5], &[BLUE, YELLOW]),
        )
        .move_to(v2(0.0, -2.5))
        .opacity(sub_op);
        vec![burst, logo, title, sub, light_leak(f.t, "intro", 0.6)]
    })
}

fn annotations() -> Scene {
    let mut s = Scene::new();
    let words = [
        "Underline",
        "Box",
        "Circle",
        "Highlight",
        "Strike",
        "Crossed",
        "Bracket",
    ];
    let kinds = [
        RoughKind::Underline,
        RoughKind::Box,
        RoughKind::Circle,
        RoughKind::Highlight,
        RoughKind::StrikeThrough,
        RoughKind::CrossedOff,
        RoughKind::Bracket,
    ];
    let row = Mobject::group(words.iter().map(|w| text_sized(w, 0.45, WHITE)))
        .arrange_in_grid(4, 1.0, 1.0);
    let objs: Vec<Obj> = row.children.iter().map(|m| s.add(m.clone())).collect();
    let colors = [YELLOW, RED, GREEN, YELLOW, RED, ORANGE, TEAL];
    for (i, k) in kinds.iter().enumerate() {
        let a = s.register(rough_annotation(
            &row.children[i],
            *k,
            colors[i],
            i as u64 + 1,
        ));
        s.play(create(a).run_time(0.5));
        let _ = objs[i];
    }
    s.wait(0.8);
    s
}

fn shapes_scene() -> Scene {
    let mut s = Scene::new();
    let items = Mobject::group([
        heart(1.4),
        spark(1.4),
        block_arrow(2.0, 1.0, 0.4, 0.7),
        pie(0.8, 0.7, TEAL),
        die_face(5, 1.2),
        clock(0.8, 10.0, 10.0),
        lightbulb(1.6, true),
        speedometer(0.9, 0.65),
    ])
    .arrange_in_grid(4, 1.0, 1.0);
    let o = s.register(items);
    s.play(lagged_start(vec![grow_from_center(o).lag(0.15)], 0.0).run_time(2.0));
    let tb = s.register(
        rounded_text_box(
            "Rounded text boxes hug every line of text",
            0.4,
            WHITE,
            PURPLE_E,
            6.0,
        )
        .to_edge(DOWN, 0.4),
    );
    s.play(fade_in_shift(tb, UP * 0.3));
    s.wait(1.0);
    s
}

fn main() -> anyhow::Result<()> {
    let fx = WithEffects::new(shapes_scene(), |t| {
        vec![
            Effect::Vignette {
                strength: 0.5,
                radius: 0.35,
            },
            Effect::Grain {
                amount: 0.03,
                seed: (t * 60.0) as u32,
            },
        ]
    });
    let movie = Movie::new()
        .then(intro())
        .transition(Transition::new(Presentation::Iris, 0.8))
        .then(annotations())
        .transition(Transition::slide(Dir::FromRight, 0.7))
        .then(fx)
        .transition(Transition::new(Presentation::FilmBurn, 0.9))
        .then(Still::new(bold_text("Thanks!", 1.0, YELLOW), 1.5));
    run(&movie, "remotion_showcase")
}
