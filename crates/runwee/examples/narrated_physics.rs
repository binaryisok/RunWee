//! Narration-driven explainer: voice-over blocks pace the animation, captions are
//! generated automatically, sound effects mark events, music plays underneath.
//! Set RUNWEE_TTS (e.g. `espeak-ng -w {out} "{text}"`) to synthesise real speech.
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.captions.look = CaptionLook::default();
    s.background_music(sfx::ambient_pad(30.0, 45.0, 1.0), 0.25);

    let title = s.register(bold_text("Free Fall", 0.9, WHITE).to_edge(UP, 0.5));
    s.voiceover(
        Voiceover::new("Let's see how an object falls under gravity."),
        |s, _| {
            s.add_sound(sfx::swoosh_up(), 0.6);
            s.play(write(title));
        },
    );

    let ground = s.register(line(v2(-6.0, -3.0), v2(6.0, -3.0)).stroke(GRAY_B, 4.0));
    let ball = s.register(
        circle(0.3)
            .sheen(RED, UL)
            .no_stroke()
            .move_to(v2(-3.0, 2.4)),
    );
    s.voiceover(
        Voiceover::new(
            "We drop a ball from rest. <bookmark mark='drop'/> Gravity accelerates it downwards.",
        ),
        |s, vo| {
            s.play([create(ground), grow_from_center(ball)]);
            s.wait_until(vo.bookmark("drop"));
            s.play(
                ball.animate()
                    .move_to(v2(-3.0, -2.7))
                    .rate(Rate::InQuad)
                    .run_time(1.2),
            );
            s.add_sound(sfx::boom(), 0.5);
            s.play(indicate(ball));
        },
    );
    let eq = s.register(
        tex_sized(r"y(t) = y_0 - \tfrac{1}{2} g t^2", 0.7, YELLOW).shift(RIGHT * 2.5 + UP * 0.5),
    );
    s.voiceover(
        Voiceover::new("Its height follows y equals y naught minus one half g t squared."),
        |s, _| {
            s.play(write(eq));
            s.add_sound(sfx::ding(), 0.5);
            s.play(circumscribe(eq, YELLOW));
        },
    );
    s.wait(0.5);
    run(&s, "narrated_physics")
}
