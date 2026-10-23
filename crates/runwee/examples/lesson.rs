//! Phase T5 — classroom extras around one public-domain fable (Aesop's
//! "The Lion and the Mouse", retold in original words): plot mountain,
//! story map, character web, mood meter, comprehension cards, summary &
//! moral, a diary page, an informal letter and a word cloud.
use runwee::lesson::*;
use runwee::prelude::story::*;
use runwee::prelude::*;

const FW: f64 = 14.22;
const FH: f64 = 8.0;

fn board(title: &str, subtitle: &str, t: f64) -> Mobject {
    let e = clamp01(t / 0.5) as f32;
    group(vec![
        rectangle(FW, FH).fill(rgb8(247, 242, 233), 1.0).no_stroke(),
        rectangle(FW, 0.9)
            .move_to(v2(0.0, FH * 0.5 - 0.45))
            .fill(rgb8(120, 70, 50), 1.0)
            .no_stroke(),
        font_text(title, "story", 0.42, WHITE)
            .move_to_aligned(v2(-6.7, FH * 0.5 - 0.45), v2(-1.0, 0.0))
            .opacity(e),
        font_text(subtitle, "hand", 0.28, rgb8(250, 222, 196))
            .move_to_aligned(v2(6.7, FH * 0.5 - 0.45), v2(1.0, 0.0))
            .opacity(e),
    ])
}

type Act = fn(f64) -> Mobject;

const SUB: &str = "The Lion and the Mouse · Aesop";

fn act_plot(t: f64) -> Mobject {
    let stages = [
        ("Exposition", "A lion naps; a mouse scampers over his nose."),
        (
            "Rising action",
            "The lion grabs the mouse, laughs, and lets him go.",
        ),
        ("Climax", "Days later the lion is caught in a hunter's net."),
        (
            "Falling action",
            "The mouse hears the roar and gnaws the ropes.",
        ),
        ("Resolution", "The lion is free; the two become friends."),
    ];
    group(vec![
        board("Plot mountain", SUB, t),
        plot_mountain(
            &stages,
            Rect::from_center(v2(0.0, -0.75), 8.8, 5.6),
            (t - 0.4) / 6.0,
            t,
        ),
    ])
}

fn act_map(t: f64) -> Mobject {
    let cells = [
        ("Characters", "a proud lion, a tiny mouse, a hunter"),
        ("Setting", "a warm forest, long ago"),
        ("Problem", "the mouse wakes the lion and is caught"),
        ("Turning point", "the lion is trapped in a net"),
        ("Solution", "the mouse chews through the ropes"),
        ("Theme", "small friends can be great friends"),
    ];
    group(vec![
        board("Story map", SUB, t),
        story_map(
            &cells,
            Rect::from_center(v2(0.0, -0.45), 12.6, 6.0),
            (t - 0.3) / 4.5,
        ),
    ])
}

fn act_web(t: f64) -> Mobject {
    let others = [
        ("Lion", "spared by", "proud → grateful"),
        ("Hunter", "outwits", "sly, patient"),
        ("Sisters", "hurries home to", "worried"),
        ("Forest", "lives in", "full of danger"),
    ];
    group(vec![
        board("Character web", SUB, t),
        character_web(
            "Mouse",
            "small but brave",
            &others,
            v2(0.0, -0.35),
            4.6,
            2.1,
            (t - 0.3) / 3.5,
            t,
        ),
    ])
}

fn act_mood(t: f64) -> Mobject {
    let pts = [
        ("playing", 0.7),
        ("caught", -0.9),
        ("let go", 0.6),
        ("hears roar", -0.45),
        ("frees lion", 1.0),
    ];
    let p = (t - 0.4) / 5.0;
    // dial follows the graph
    let u = clamp01(p * 1.1) * (pts.len() - 1) as f64;
    let i = (u.floor() as usize).min(pts.len() - 2);
    let mood = lerp(pts[i].1, pts[i + 1].1, smoothstep(0.0, 1.0, u - i as f64));
    group(vec![
        board("Mood meter", SUB, t),
        mood_meter(
            "the mouse",
            &pts,
            Rect::from_center(v2(-2.2, -0.45), 9.2, 6.0),
            p,
        ),
        mood_dial("right now", mood, v2(4.95, 0.1), 1.65),
    ])
}

fn act_comprehension(t: f64) -> Mobject {
    let qs = [
        (
            "Why did the lion let the mouse go?",
            "He found it funny that such a tiny creature promised to help him one day.",
        ),
        (
            "How did the mouse know the lion was in trouble?",
            "He heard the lion's angry roar ringing through the forest.",
        ),
        (
            "Find a word that means 'to bite at something again and again'.",
            "gnaw",
        ),
    ];
    let mut parts = vec![board("Comprehension", "read · think · answer", t)];
    for (i, (q, a)) in qs.iter().enumerate() {
        let c = v2(
            -4.5 + i as f64 * 4.5,
            2.25 - comprehension_card_height(q, a, 4.15) * 0.5,
        );
        parts.push(comprehension_card(
            i + 1,
            q,
            a,
            c,
            4.15,
            (t - 0.3 - 0.3 * i as f64) / 0.6,
            (t - 2.0 - 1.6 * i as f64) / 1.4,
        ));
    }
    parts.push(
        font_text("Vocabulary check", "sans", 0.24, rgb8(110, 100, 92))
            .move_to(v2(0.0, -1.75))
            .opacity(clamp01((t - 6.0) / 0.4) as f32),
    );
    parts.push(runwee::grammar::fill_blank(
        "The brave mouse began to",
        "gnaw",
        "the thick ropes.",
        v2(0.0, -2.6),
        0.46,
        (t - 6.2) / 1.4,
    ));
    group(parts)
}

fn act_summary(t: f64) -> Mobject {
    let summary = "A sleepy lion catches a mouse who has run across his nose. The mouse begs to be freed and promises to repay the favour. The lion laughs and lets him go. Later the lion is trapped in a hunter's net, and the little mouse gnaws through the ropes to set him free.";
    group(vec![
        board("Summary & moral", SUB, t),
        summary_card(
            "In a nutshell",
            summary,
            "No act of kindness is ever wasted.",
            v2(0.0, -0.4),
            10.5,
            (t - 0.3) / 4.0,
        ),
    ])
}

fn act_writing(t: f64) -> Mobject {
    let entry = "What a day! The lion laughed at my promise, but tonight I kept it. My teeth still ache from those ropes, yet I have never felt so tall.";
    let body = "You will not believe what happened in the forest this week. A tiny mouse saved a lion! I learnt that nobody is too small to help. Write back soon and tell me about your school trip.";
    group(vec![
        board("Write it!", "a diary entry and an informal letter", t),
        diary_page(
            "Friday, 9 June",
            entry,
            "Mouse",
            Rect::from_center(v2(-3.55, 0.25), 6.5, 5.1),
            (t - 0.3) / 3.4,
        ),
        letter_page(
            &["14 Lake View Road", "Shimla"],
            "12 June",
            "Dear Ravi,",
            body,
            "Your loving friend,",
            "Asha",
            Rect::from_center(v2(3.55, 0.25), 6.5, 5.1),
            (t - 2.0) / 4.0,
        ),
        font_text(
            "date · Dear Diary · my feelings, in first person · sign-off",
            "hand",
            0.25,
            rgb8(110, 100, 92),
        )
        .move_to(v2(-3.55, -2.85))
        .opacity(clamp01((t - 1.0) / 0.5) as f32),
        font_text(
            "address · date · greeting · body · closing · name",
            "hand",
            0.25,
            rgb8(110, 100, 92),
        )
        .move_to(v2(3.55, -2.85))
        .opacity(clamp01((t - 2.5) / 0.5) as f32),
    ])
}

fn act_cloud(t: f64) -> Mobject {
    let words = [
        ("kindness", 9.0),
        ("lion", 7.0),
        ("mouse", 7.0),
        ("net", 5.0),
        ("promise", 5.0),
        ("gnaw", 4.0),
        ("roar", 4.0),
        ("hunter", 3.5),
        ("forest", 3.0),
        ("friends", 4.5),
        ("tiny", 2.5),
        ("proud", 2.5),
        ("grateful", 3.0),
        ("ropes", 2.5),
        ("laugh", 2.0),
        ("brave", 3.0),
        ("free", 3.0),
        ("sleepy", 1.5),
        ("paws", 1.5),
        ("whiskers", 1.5),
        ("trap", 2.0),
        ("help", 2.5),
    ];
    group(vec![
        board("Word cloud", "the words that carry the story", t),
        word_cloud(&words, v2(0.0, -0.45), 12.6, 6.4, 7, (t - 0.3) / 3.0),
    ])
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(247, 242, 233));
    let acts: [(Act, f64); 8] = [
        (act_plot, 8.0),
        (act_map, 7.0),
        (act_web, 6.5),
        (act_mood, 7.0),
        (act_comprehension, 8.0),
        (act_summary, 6.5),
        (act_writing, 8.0),
        (act_cloud, 6.0),
    ];
    for (i, (f, d)) in acts.iter().enumerate() {
        let f = *f;
        s.always_redraw(move |ctx| f(ctx.elapsed));
        s.wait(*d);
        if i + 1 < acts.len() {
            s.remove_all();
        }
    }
    run(&s, "lesson")
}
