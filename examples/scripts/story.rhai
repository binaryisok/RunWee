// Storytelling from a script: a folk tale ("The Thirsty Crow", retold in
// original words) with scenery, critters, speech bubbles, narration, an iris
// transition, then a dusk scene with characters, fireflies and a folk-art frame.
// runwee render examples/scripts/story.rhai -q medium
background("#f6e7c8");

// 1 ── a hot afternoon: the crow searches for water
let s1 = always_redraw(|t| {
    let x = -8.0 + t * 2.2;
    group([
        landscape(vec2(0.0, 0.0), 14.4, 8.2, "marigold", t, 1.0, 3),
        critter("crow").sized(0.9).draw(t, 0.0, 1.0, 1.0, vec2(x.min(3.0), 1.6 + (t * 3.0).sin() * 0.25)),
        narration(typed("It was the hottest day of summer. A crow flew far and wide, looking for a single drop of water.", t / 3.0),
                  vec2(-2.6, -2.9), 8.0, 0.3, "marigold", (t / 0.6).min(1.0)),
    ])
});
wait(6.5);
remove(s1);

// 2 ── the jug: too little water, then pebble after pebble
let s2 = always_redraw(|t| {
    let drops = (t / 0.9).floor().min(6.0);
    let level = -1.9 + drops * 0.24;
    let pebs = [];
    for i in 0..6 {
        if t > i * 0.9 {
            let fall = ((t - i * 0.9) / 0.7).min(1.0);
            let rest = vec2(0.75 + (i % 3) * 0.45, -2.4 + (i / 3) * 0.26);
            let start = vec2(1.6, 1.0);
            pebs.push(circle(0.13).color("#6e655c").fill_opacity(1.0)
                .move_to(start + (rest - start) * (fall * fall)));
        }
    }
    let jug = group([
        rectangle(1.9, 2.7).color("#b5643c").fill_opacity(1.0).move_to(vec2(1.2, -1.35)),
        rectangle(1.6, 2.4).color("#5b2e1a").fill_opacity(1.0).move_to(vec2(1.2, -1.35)),
        rectangle(1.6, level + 2.55).color("#5aa9e6").fill_opacity(0.9).move_to(vec2(1.2, -2.55 + (level + 2.55) / 2.0)),
        rectangle(2.2, 0.22).color("#8a4526").fill_opacity(1.0).move_to(vec2(1.2, 0.05)),
    ]);
    let happy = if drops >= 6.0 { 1.0 } else { 0.0 };
    group([
        sky(vec2(0.0, 0.0), 14.4, 8.2, "#ffe3a3", "#ffc27a"),
        rectangle(14.4, 1.4).color("#c98a4b").fill_opacity(1.0).move_to(vec2(0.0, -3.4)),
        tree(vec2(-5.4, -2.7), 4.2, "banyan", "#4f7d3a", (t * 0.8).sin() * 0.04, 5),
        jug, group(pebs),
        text("water level ↑", 0.26, "#5b2e1a").move_to(vec2(-0.6, level)).opacity(if t > 1.0 { 1.0 } else { 0.0 }),
        critter("crow").sized(1.1).draw(t, happy, 0.0, -1.0, vec2(2.6, 0.75)),
        if happy > 0.0 {
            bubble("speech", "At last — water!", vec2(4.6, 2.6), 3.0, vec2(3.0, 1.5), 0.32, ((t - 5.6) / 0.4).min(1.0))
        } else {
            bubble("thought", "The water is too low… what if I drop pebbles in?", vec2(4.4, 2.6), 3.6, vec2(3.0, 1.6), 0.28, (t / 0.5).min(1.0))
        },
    ])
});
wait(7.5);
let ir = always_redraw(|t| iris(t / 0.9, vec2(3.0, 0.9), "#1b1530"));
wait(0.9);
remove(s2); remove(ir);

// 3 ── dusk under the banyan: Grandpa tells the moral
let s3 = always_redraw(|t| {
    group([
        landscape(vec2(0.0, 0.0), 14.4, 8.2, "dusk", t, 0.2, 7),
        tree(vec2(-4.6, -2.4), 4.6, "banyan", "#2f4a33", (t * 0.7).sin() * 0.03, 2),
        house(vec2(4.6, -2.3), 2.2, "#e8c39e", "#8a3b2a", 1.0),
        fireflies(vec2(0.0, 0.0), 14.0, 7.0, t, 26, "#fff6a0", 4),
        character("grandpa").draw("point", t, if t > 0.8 && t < 4.5 { 1.0 } else { 0.0 }, 1.0, vec2(-2.2, -2.6)),
        character("girl").draw(if t > 5.0 { "cheer" } else { "idle" }, t, if t > 5.0 { 1.0 } else { 0.0 }, -1.0, vec2(0.6, -2.6)),
        bubble("speech", "Where there is a will, there is a way.", vec2(-2.4, 1.4), 4.2, vec2(-2.1, -0.8), 0.32, ((t - 0.8) / 0.5).max(0.0).min(1.0)),
        bubble("shout", "I'll try again tomorrow!", vec2(2.6, 1.1), 3.4, vec2(0.7, -0.7), 0.3, ((t - 5.0) / 0.4).max(0.0).min(1.0)),
        iris(1.0 - t / 0.9, vec2(-1.0, -1.0), "#1b1530"),
    ])
});
wait(8.0);
remove(s3);

// 4 ── the end, framed in Warli art
let s4 = always_redraw(|t| {
    group([
        rectangle(14.4, 8.2).color("#7a3e22").fill_opacity(1.0),
        folk_frame("warli", vec2(0.0, 0.0), 13.6, 7.4, 0.7, t, 9),
        warli_dance(vec2(0.0, -0.5), 1.8, 2, t, "#fdf3e1"),
        story_text("The End", 0.8, "#fdf3e1").move_to(vec2(0.0, 2.3)).opacity((t / 1.0).min(1.0)),
    ])
});
wait(5.0);
