//! Phase A3 — generative art gallery: an animated domain-warped noise "shader",
//! a curl-noise flow field, a growing L-system, a Clifford attractor density
//! image, a phyllotaxis sunflower and Gray–Scott reaction–diffusion coral.
use runwee::prelude::*;
use std::sync::Arc;

const W: f64 = 4.3;
const H: f64 = 3.0;

fn panel(c: Vec2) -> Rect {
    Rect::from_center(c, W, H)
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(8, 9, 16));
    s.add(bold_text("Generative art", 0.42, WHITE).to_edge(UP, 0.22));
    let centers = [
        v2(-4.6, 1.55),
        v2(0.0, 1.55),
        v2(4.6, 1.55),
        v2(-4.6, -2.1),
        v2(0.0, -2.1),
        v2(4.6, -2.1),
    ];
    let names = [
        "domain-warped fBm",
        "curl-noise flow field",
        "L-system",
        "Clifford attractor",
        "phyllotaxis",
        "Gray–Scott reaction–diffusion",
    ];

    // 1. animated domain warp "fragment shader"
    let r1 = panel(centers[0]);
    s.always_redraw(move |ctx| {
        let t = ctx.t;
        field_image(r1, 150, move |p| {
            let q = (p - r1.center()) * 0.55;
            let v = domain_warp2(q, 3.5, t * 2.0, 7) * 0.5 + 0.5;
            gradient_at(
                &[
                    rgb8(10, 14, 40),
                    rgb8(60, 40, 140),
                    rgb8(230, 80, 120),
                    rgb8(250, 190, 90),
                    rgb8(255, 245, 230),
                ],
                v * 1.05,
            )
        })
    });

    // 2. flow field drawn on progressively
    let r2 = panel(centers[1]);
    let flow = Arc::new(flow_field(
        Rect::from_center(r2.center(), W - 0.1, H - 0.1),
        260,
        move |p| curl2(p * 0.45, 0.0, 3).angle(),
        "ocean",
        11,
    ));
    s.always_redraw(move |ctx| {
        let k = (ctx.t / 3.0).clamp(0.0, 1.0);
        flow.as_ref().clone().partial(0.0, k)
    });

    // 3. L-system plant growing
    let r3 = panel(centers[2]);
    let plant = Arc::new(
        LSystem::preset("plant")
            .mobject(5, 80.0, 2.7, rgb8(120, 80, 40), rgb8(140, 230, 110))
            .move_to(r3.center()),
    );
    s.always_redraw(move |ctx| {
        let k = (ctx.t / 4.0).clamp(0.0, 1.0);
        plant.as_ref().clone().partial(0.0, Rate::InOutSine.eval(k))
    });

    // 4. Clifford attractor (400k iterations → log-density image)
    let r4 = panel(centers[3]);
    let pts: Vec<Vec2> = clifford(-1.7, 1.3, -0.1, -1.21, 400_000)
        .into_iter()
        .map(|p| r4.center() + v2(p.x * 1.0, p.y * 0.72))
        .collect();
    let attractor = density_image(
        &pts,
        r4,
        420,
        &[
            rgb8(40, 10, 80),
            rgb8(200, 40, 140),
            rgb8(255, 170, 80),
            WHITE,
        ],
        0.8,
    );
    let at = s.register(attractor);
    s.play(vec![fade_in(at).run_time(0.01)]);

    // 5. phyllotaxis sunflower blooming outwards
    let r5 = panel(centers[4]);
    let seeds = phyllotaxis(600, 0.055);
    s.always_redraw(move |ctx| {
        let shown = ((ctx.t / 3.5).clamp(0.0, 1.0) * seeds.len() as f64) as usize;
        Mobject::group(seeds[..shown].iter().enumerate().map(|(i, p)| {
            let c = palette("sunset", (i as f64 / 600.0 + ctx.t * 0.1) % 1.0);
            dot_r(r5.center() + *p, 0.028 + 0.02 * (i as f64 / 600.0), c)
        }))
    });

    // 6. reaction–diffusion coral (pre-baked, revealed with a wipe)
    let r6 = panel(centers[5]);
    let v = gray_scott(172, 120, 3000, 0.0545, 0.062, 5);
    let coral = grid_image(
        &v,
        172,
        120,
        r6,
        &[
            rgb8(5, 20, 30),
            rgb8(20, 120, 130),
            rgb8(160, 240, 200),
            WHITE,
        ],
    );
    let rd = s.register(coral);
    s.play(vec![fade_in(rd).run_time(0.01)]);

    for (c, name) in centers.iter().zip(names) {
        s.add(
            rounded_rectangle(W, H, 0.12)
                .stroke(GRAY_D, 2.0)
                .move_to(*c),
        );
        s.add(text_sized(name, 0.2, GRAY_A).move_to(*c + v2(0.0, -H / 2.0 - 0.17)));
    }
    s.wait(5.5);
    run(&s, "art_gallery")
}
