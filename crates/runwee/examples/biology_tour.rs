//! Biology: labelled animal cell, plant cell, rotating DNA, a firing neuron,
//! mitosis, Punnett square and predator–prey dynamics.
use runwee::biology::*;
use runwee::prelude::*;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    let cell = animal_cell(2.6);
    let labeled = labeled_diagram(
        cell.clone(),
        &[
            ("Nucleus", v2(-0.1, 0.1)),
            ("Mitochondrion", v2(1.3, 0.78)),
            ("Golgi apparatus", v2(1.43, -0.13)),
            ("Rough ER", v2(-0.4, -1.25)),
            ("Lysosome", v2(-1.56, 0.91)),
            ("Membrane", v2(-2.55, -0.2)),
        ],
        0.32,
    );
    let c = s.register(labeled);
    s.play(draw_border_then_fill(c).run_time(2.5));
    s.wait(1.0);
    s.play(fade_out(c));

    // DNA rotating
    let t0 = s.time();
    let dna = s.always_redraw(move |ctx| {
        dna_helix(11.0, 0.9, 3.5, (ctx.t - t0) * 2.0, "ATGCGATTACAGGC").shift(UP * 0.5)
    });
    let caption = s.register(
        tex_sized(
            r"\text{A}\!=\!\text{T},\ \text{G}\!\equiv\!\text{C}",
            0.5,
            WHITE,
        )
        .to_edge(DOWN, 1.0),
    );
    s.play(fade_in(caption));
    s.wait(3.0);
    s.play([fade_out(dna), fade_out(caption)]);

    // neuron firing
    let k = s.tracker(0.0);
    let n = s.always_redraw(move |ctx| neuron(11.0, Some(ctx.value(k))));
    s.play(k.to(1.0).run_time(2.5).rate(Rate::InOutSine));
    s.play(fade_out(n));

    // mitosis
    let m = s.tracker(0.0);
    let stage = s.always_redraw(move |ctx| {
        let t = ctx.value(m);
        Mobject::group([
            mitosis(t, 5.0),
            bold_text(mitosis_stage_name(t), 0.5, YELLOW).to_edge(UP, 0.6),
        ])
    });
    s.play(m.to(4.99).run_time(5.0).rate(Rate::Linear));
    s.play(fade_out(stage));

    // genetics + ecology
    let ps = s.register(punnett_square("Aa", "Aa", 1.1).shift(LEFT * 3.6));
    s.play(create(ps).run_time(1.5));
    let tr = Arc::new(predator_prey(1.0, 0.1, 0.075, 1.5, 10.0, 5.0, 15.0));
    let ax = Axes::new((0.0, 15.0, 5.0), (0.0, 40.0, 10.0))
        .size(6.0, 4.0)
        .at(v2(3.0, 0.0))
        .labels("t", "N");
    let axes = s.register(ax.build());
    s.play(create(axes));
    let (tr1, tr2) = (tr.clone(), tr.clone());
    let prey = s.register(ax.plot(move |t| tr1.get(t, 0)).stroke(GREEN, 4.0));
    let pred = s.register(ax.plot(move |t| tr2.get(t, 1)).stroke(RED, 4.0));
    s.play([create(prey).run_time(2.5), create(pred).run_time(2.5)]);
    s.wait(1.0);
    run(&s, "biology_tour")
}
