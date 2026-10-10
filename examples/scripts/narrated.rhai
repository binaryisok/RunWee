// Remotion-style piece: voiceover-timed animation, captions, effects and transitions.
// Without a TTS command (RUNWEE_TTS) narration is timed from word counts and shown as captions.
captions("tiktok");
music(0.15);
effect("vignette", 0.5);

let head = register(bold_text("Newton's second law", 60, WHITE));
voiceover("Force equals mass times acceleration.", |info| {
    play(write(head), 1.2);
    let eq = register(tex(`F = ma`, 2.0).shift(DOWN * 1.2));
    play(fade_in(eq, UP));
    sound("pop", 0.6);
});

next_scene("wipe:from-left", 0.7);
effect("grain", 0.04);
let block = register(rounded_rectangle(1.6, 1.0, 0.15).color(BLUE).fill_opacity(0.8).shift(LEFT * 4.0));
let push = register(labeled_arrow(vec2(-6.2, 0.0), vec2(-4.9, 0.0), "F", RED));
voiceover("Push a block, and it speeds up in the direction of the force.", |info| {
    play([fade_in(block), grow_arrow(push)]);
    play([animate(block).shift(RIGHT * 7.0).rate("ease_in_quad"), fade_out(push)], 2.0);
});
wait(0.5);
