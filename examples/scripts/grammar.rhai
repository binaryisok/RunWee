// Grammar and classroom extras from a script, with original example
// sentences built around "The Thirsty Crow": parts-of-speech chips, a parse
// tree, a sentence diagram, tense timelines, voice and reported-speech morphs,
// agreement checks, prepositions, articles, a quiz, then a plot mountain,
// character web, mood meter, comprehension cards and a word cloud.
// runwee render examples/scripts/grammar.rhai -q medium
background("#f7f1e6");
let INK = "#2b2620";

fn board(title, sub, t) {
    group([
        rectangle(14.4, 1.05).color("#22355e").fill_opacity(1.0).move_to(vec2(0.0, 3.55)),
        story_text(title, 0.46, "#ffffff").move_to_aligned(vec2(-6.6, 3.6), vec2(-1.0, 0.0)),
        story_italic(sub, 0.26, "#c9d6f2").move_to_aligned(vec2(6.6, 3.55), vec2(1.0, 0.0)).opacity((t / 0.6).min(1.0)),
    ])
}

// 1 ── parts of speech + parse tree
let g1 = always_redraw(|t| {
    group([
        board("Parts of speech", "every word has a job", t),
        pos_chips("The clever crow quickly dropped small pebbles into the old jug.", vec2(0.0, 1.6), 0.34, (t - 0.4) / 3.0, true),
        pos_legend(vec2(0.6, -0.6), 0.3, (t - 3.2) / 1.2),
        hand_text(tree_bracket("The crow found a jug."), 0.28, "#5a6e9a").move_to(vec2(0.0, -2.6)).opacity(((t - 4.4) / 0.6).max(0.0).min(1.0)),
    ])
});
wait(6.5);
remove(g1);
let g2 = always_redraw(|t| {
    group([
        board("Sentence structure", "parse tree · Reed–Kellogg diagram", t),
        parse_tree("The thirsty crow found a jug.", vec2(-3.9, 2.5), 0.95, 0.32, (t - 0.3) / 3.0),
        sentence_diagram("The old farmer planted tall trees.", vec2(0.2, 1.2), 0.4, INK, (t - 1.8) / 3.0),
    ])
});
wait(6.5);
remove(g2);

// 2 ── tenses, voice and reported speech
let tenses = ["past simple", "present continuous", "future perfect"];
let g3 = always_redraw(|t| {
    let rows = [board("Tenses on a timeline", "when does the action happen?", t)];
    for i in 0..3 {
        let y = 2.0 - i * 1.9;
        let p = (t - 0.4 - i * 1.0) / 1.4;
        rows.push(tense_timeline(tenses[i], vec2(-2.4, y), 7.6, 0.72, p, "#d9822b"));
        rows.push(story_text(tense_sentence("The crow", "drop", "a pebble", tenses[i]), 0.32, INK)
            .move_to_aligned(vec2(1.9, y), vec2(-1.0, 0.0)).opacity(p.max(0.0).min(1.0)));
    }
    group(rows)
});
wait(6.0);
remove(g3);
let active = "The crow dropped a pebble.";
let passive = passive_voice("the crow", "drop", "a pebble", "past simple");
let direct = "The crow said, \"I am thirsty.\"";
let reported = reported_speech("The crow", "male", "", "I am thirsty.");
let g4 = always_redraw(|t| {
    let p1 = ((t - 1.0) / 2.2).max(0.0).min(1.0);
    let p2 = ((t - 4.4) / 2.2).max(0.0).min(1.0);
    group([
        board("Transformations", "active → passive · direct → reported", t),
        hand_text("ACTIVE → PASSIVE", 0.3, "#2e6f9e").move_to_aligned(vec2(-6.5, 2.4), vec2(-1.0, 0.0)),
        word_morph(active, passive, vec2(0.0, 1.75), vec2(0.0, 0.95), "story", 0.48, INK, "#2f8f4e", p1),
        hand_text("DIRECT → REPORTED", 0.3, "#2e6f9e").move_to_aligned(vec2(-6.5, -0.85), vec2(-1.0, 0.0)).opacity(((t - 3.8) / 0.4).max(0.0).min(1.0)),
        word_morph(direct, reported, vec2(0.0, -1.6), vec2(0.0, -2.45), "story", 0.48, INK, "#2f8f4e", p2),
    ])
});
wait(7.5);
remove(g4);

// 3 ── agreement, prepositions, articles, a quiz
let g5 = always_redraw(|t| {
    let preps = ["in", "on", "under", "beside"];
    let parts = [board("Small words, big jobs", "agreement · prepositions · articles", t)];
    parts.push(sva_check("The crow", "drink", "water.", vec2(-3.4, 2.1), 0.32, (t - 0.3) / 1.6));
    parts.push(sva_check("The pebbles", "sink", "fast.", vec2(-3.4, 0.6), 0.32, (t - 1.3) / 1.6));
    for i in 0..4 {
        parts.push(preposition_scene(preps[i], vec2(1.0 + i * 1.55, 1.4), 0.95, t).opacity(((t - 2.0 - i * 0.3) / 0.4).max(0.0).min(1.0)));
    }
    parts.push(article_card("hour", vec2(-4.6, -1.3), 3.2, (t - 3.4) / 1.0));
    parts.push(article_card("unicorn", vec2(-1.2, -1.3), 3.2, (t - 3.9) / 1.0));
    parts.push(quiz_card("The crow saw ___ empty jug.", ["a", "an", "the", "no article"], 1, 1, vec2(3.6, -1.6), 5.6, (t - 5.0) / 1.0, t));
    group(parts)
});
wait(8.0);
remove(g5);

// 4 ── classroom extras for the same story
let stages = [
    ["Exposition", "A hot day; a crow is thirsty."],
    ["Rising action", "It finds a jug with little water."],
    ["Climax", "It drops pebbles, one by one."],
    ["Falling action", "The water rises to the brim."],
    ["Resolution", "The crow drinks and flies away."],
];
let g6 = always_redraw(|t| {
    group([
        board("Plot mountain", "The Thirsty Crow · a folk tale", t),
        plot_mountain(stages, vec2(0.0, -0.75), 8.8, 5.4, (t - 0.3) / 4.0, t),
    ])
});
wait(6.0);
remove(g6);
let g7 = always_redraw(|t| {
    group([
        board("Characters & feelings", "web · mood meter", t),
        character_web("Crow", "clever & patient", [["Jug", "holds", "almost empty"], ["Pebbles", "used as", "tools"], ["Sun", "makes thirsty", "harsh"]],
                      vec2(-3.4, -0.4), 2.6, 1.8, (t - 0.3) / 3.0, t),
        mood_meter("the crow", [["thirsty", -0.6], ["finds jug", 0.3], ["water low", -0.7], ["pebbles", 0.4], ["drinks", 0.95]],
                   vec2(3.6, -0.4), 6.0, 5.6, (t - 1.0) / 3.0),
    ])
});
wait(6.5);
remove(g7);
let g8 = always_redraw(|t| {
    group([
        board("Check & remember", "comprehension · word cloud", t),
        comprehension_card(1, "Why did the crow drop pebbles into the jug?", "To raise the water level so it could reach it.", vec2(-4.2, 1.0), 5.2, (t - 0.3) / 1.0, (t - 2.0) / 0.8),
        comprehension_card(2, "Which word best describes the crow?", "Resourceful — it solved the problem with what it had.", vec2(-4.2, -1.7), 5.2, (t - 0.9) / 1.0, (t - 2.8) / 0.8),
        word_cloud([["patience", 9.0], ["pebbles", 7.0], ["thirst", 6.0], ["jug", 5.0], ["clever", 8.0], ["water", 6.0], ["summer", 3.0], ["effort", 5.0], ["idea", 4.0], ["rise", 3.0]],
                   vec2(3.0, -0.4), 7.0, 5.6, 11, (t - 1.2) / 3.0),
    ])
});
wait(7.0);
