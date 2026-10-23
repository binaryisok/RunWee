// Poetry from a script, using public-domain poems only: Christina Rossetti's
// "Who Has Seen the Wind?" (1872) and R. L. Stevenson's "The Wind" (1885).
// Typewriter / handwriting reveals, rhyme colours + scheme, meter marks,
// syllable counts, figure-of-speech callouts and flip-in vocabulary cards.
// runwee render examples/scripts/poem.rhai -q medium
background("#f0e4ce");
let INK = "#3a2a1c";

let rossetti = "Who has seen the wind?\nNeither I nor you:\nBut when the leaves hang trembling,\nThe wind is passing through.\n\nWho has seen the wind?\nNeither you nor I:\nBut when the trees bow down their heads,\nThe wind is passing by.";
let stevenson = "I saw you toss the kites on high\nAnd blow the birds about the sky;\nAnd all around I heard you pass,\nLike ladies' skirts across the grass—\nO wind, a-blowing all day long,\nO wind, that sings so loud a song!";

// 1 ── Rossetti: typewriter reveal, rhyme colours, wind in the leaves
let l1 = poem("Who Has Seen the Wind?", "Christina Rossetti", rossetti, "story", 0.32, vec2(-6.3, 1.9), 1.6);
let scheme1 = scheme(l1);
let a1 = always_redraw(|t| {
    group([
        rounded_rectangle(7.4, 7.0, 0.15).color("#d6c4a0").fill_opacity(0.0).move_to(vec2(-3.0, 0.0)),
        story_text("Who Has Seen the Wind?", 0.5, INK).move_to_aligned(vec2(-6.3, 3.05), vec2(-1.0, 0.0)),
        story_italic("Christina Rossetti, Sing-Song (1872)", 0.28, "#8a6d4b").move_to_aligned(vec2(-6.3, 2.6), vec2(-1.0, 0.0)),
        reveal_poem(l1, (t - 0.4) / 5.0, "typewriter", INK),
        if t > 5.6 { rhyme_highlights(l1, (t - 5.6) / 1.8) } else { group([]) },
        hand_text("Rhyme scheme: " + scheme1, 0.34, INK).move_to_aligned(vec2(-6.3, -3.3), vec2(-1.0, 0.0)).opacity(((t - 7.0) / 0.5).max(0.0).min(1.0)),
        watercolor_wash(vec2(4.3, 0.0), 5.2, 7.0, "#cfe3f1", "#f4ead7", 4),
        tree(vec2(3.4, -3.0), 4.4, "round", "#5d8a45", (t * 2.1).sin() * 0.12, 3),
        tree(vec2(5.6, -3.0), 3.2, "round", "#73a052", (t * 2.4 + 1.0).sin() * 0.14, 8),
        falling_leaves(vec2(4.3, 0.0), 5.0, 6.6, t, 16, 0.16, ["#d98a2b", "#a3c45a", "#e0b84a"], 6),
    ])
});
wait(10.0);
remove(a1);

// 2 ── Stevenson: handwriting, meter, syllables, figures of speech
let l2 = poem("The Wind", "R. L. Stevenson", stevenson, "story", 0.4, vec2(-6.4, 1.7), 1.75);
let a2 = always_redraw(|t| {
    group([
        story_text("The Wind", 0.55, INK).move_to_aligned(vec2(-6.4, 3.05), vec2(-1.0, 0.0)),
        story_italic("R. L. Stevenson, A Child's Garden of Verses (1885)", 0.26, "#8a6d4b").move_to_aligned(vec2(-6.4, 2.6), vec2(-1.0, 0.0)),
        reveal_poem(l2, (t - 0.3) / 4.5, "handwriting", INK),
        if t > 5.0 { meter_marks(l2, 0, "iambic", "#c0392b", (t - 5.0) / 1.2) } else { group([]) },
        if t > 5.0 { syllable_counts(l2, 0.4, "#2e6f9e", (t - 5.6) / 1.2) } else { group([]) },
        if t > 5.0 { clap_beats(l2, 0, t - 5.0, 96.0, "#c0392b") } else { group([]) },
        figure_callout(l2, "simile", 3, 0, 5, vec2(4.4, 1.2), "skirts brushing grass = the wind's sound", (t - 7.2) / 0.9),
        figure_callout(l2, "personification", 5, 3, 7, vec2(4.4, -1.6), "the wind sings like a person", (t - 8.6) / 0.9),
        figure_callout(l2, "repetition", 4, 0, 1, vec2(4.4, -3.1), "\"O wind\" — a refrain you can chant", (t - 10.0) / 0.9),
    ])
});
wait(13.0);
remove(a2);

// 3 ── word cards from the two poems
let a3 = always_redraw(|t| {
    let f0 = ((t - 1.6) / 0.7).max(0.0).min(1.0);
    let f1 = ((t - 2.4) / 0.7).max(0.0).min(1.0);
    let f2 = ((t - 3.2) / 0.7).max(0.0).min(1.0);
    group([
        story_text("Word cards", 0.6, INK).move_to(vec2(0.0, 3.1)),
        vocab_card("trembling", "verb", "shaking slightly", "The leaves were trembling in the breeze.", vec2(-4.4, 0.2), 3.6, 3.0, f0, "#2e6f9e"),
        vocab_card("toss", "verb", "throw lightly up into the air", "The wind tossed the kites on high.", vec2(0.0, 0.2), 3.6, 3.0, f1, "#c0392b"),
        vocab_card("a-blowing", "verb (old style)", "blowing; the a- adds rhythm", "O wind, a-blowing all day long.", vec2(4.4, 0.2), 3.6, 3.0, f2, "#5d8a45"),
        hand_text("syllables: trem-bling = " + count_syllables("trembling") + ", passing = " + count_syllables("passing"), 0.32, INK)
            .move_to(vec2(0.0, -2.8)).opacity(((t - 4.5) / 0.5).max(0.0).min(1.0)),
    ])
});
wait(7.0);
