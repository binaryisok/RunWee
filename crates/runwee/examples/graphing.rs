//! Graphing & data: axes, plots, areas, Riemann sums, tangents, vector fields,
//! charts, tables, matrices, live counters and network graphs.
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    let title = s.register_fixed(title("Graphing & Data"));
    s.play(write(title));

    let ax = Axes::new((-1.0, 5.0, 1.0), (-1.0, 9.0, 2.0))
        .size(7.0, 5.2)
        .at(v2(-2.6, -0.6))
        .labels("x", "f(x)");
    let axes = s.register(ax.build());
    s.play(create(axes).run_time(1.5));
    let f = |x: f64| 0.4 * x * x + 0.5;
    let graph = s.register(ax.plot(f).stroke(YELLOW, 5.0));
    s.play(create(graph).run_time(1.5));
    let area = s.register(ax.area(f, 1.0, 4.0, None, BLUE, 0.35));
    s.play(fade_in(area));
    let rects = s.register(ax.riemann(f, 1.0, 4.0, 6, "left", &[BLUE_D, TEAL]));
    s.play(lagged_start(
        vec![fade_out(area), create(rects).lag(0.1)],
        0.3,
    ));

    // moving tangent driven by a tracker
    let k = s.tracker(1.0);
    let axc = ax.clone();
    let tangent = s.always_redraw(move |ctx| {
        let x = ctx.value(k);
        Mobject::group([
            axc.tangent_line(f, x, 3.0),
            dot_r(axc.c2p(x, f(x)), 0.09, ORANGE),
        ])
    });
    let label = s.counter(
        k,
        DecimalNumber::new(2).prefix("x = ").color(ORANGE),
        v2(2.8, 0.5),
    );
    s.play(k.to(4.0).run_time(2.5));
    s.play([fade_out(tangent), fade_out(label), fade_out(rects)]);

    // charts on the right
    let chart = s.register(
        bar_chart(
            &[3.0, 7.0, 4.5, 9.0],
            &["Q1", "Q2", "Q3", "Q4"],
            &[],
            4.6,
            3.0,
        )
        .shift(v2(4.0, 1.2)),
    );
    s.play(create(chart).run_time(1.5));
    let tbl = s.register(
        math_table(
            &[
                vec!["x", "x^2", "\\sqrt{x}"],
                vec!["1", "1", "1"],
                vec!["4", "16", "2"],
            ],
            true,
            0.36,
        )
        .shift(v2(4.0, -2.4)),
    );
    s.play(fade_in_shift(tbl, UP * 0.3));
    s.wait(1.0);
    s.play([
        fade_out(chart),
        fade_out(tbl),
        fade_out(axes),
        fade_out(graph),
    ]);

    // vector field + stream lines
    let field = |p: Vec2| v2(p.y, -p.x * 0.6 - p.y * 0.2);
    let vf = s.register(
        vector_field(field, (-6.0, 6.0, 0.75), (-3.0, 3.0, 0.75), 0.4, &[]).shift(DOWN * 0.4),
    );
    s.play(lagged_start(vec![create(vf).lag(0.01)], 0.0).run_time(1.5));
    let sl = s.register(stream_lines(field, (-6.0, 6.0, 0.8), (-3.4, 2.6, 0.8), &[]));
    s.play([fade_out(vf), create(sl).run_time(2.0)]);
    s.play(fade_out(sl));

    // matrix & network
    let m = s.register(matrix(&[vec!["2", "-1"], vec!["1", "3"]], 0.6).shift(LEFT * 3.5));
    let net = s.register(
        network(
            &["A", "B", "C", "D", "E"],
            &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 0), (0, 2)],
            Layout::Circular,
            true,
            1.8,
        )
        .shift(RIGHT * 3.0 + DOWN * 0.3),
    );
    s.play([write(m), create(net).run_time(1.5)]);
    s.wait(1.0);
    run(&s, "graphing")
}
