//! Phase S4 — computer science & data: six sorting algorithms racing on the same
//! data, BFS / DFS / Dijkstra / A* exploring the same terrain, then a gallery of a
//! growing binary search tree, a neural-network forward pass, a Galton board, a
//! bar-chart race, elementary cellular automata and Conway's Life.
use runwee::prelude::algoviz::*;
use runwee::prelude::*;
use std::sync::Arc;

fn caption(s: &str, at: Vec2) -> Mobject {
    text_sized(s, 0.26, GRAY_A).move_to(at)
}

fn frame(c: Vec2, w: f64, h: f64) -> Mobject {
    rounded_rectangle(w, h, 0.12).move_to(c).stroke(GRAY_C, 1.5)
}

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    s.set_background(rgb8(9, 10, 18));

    // ── Act 1: sorting race (0 – 9 s) ──
    s.add(bold_text("Sorting: same data, six algorithms", 0.42, WHITE).to_edge(UP, 0.25));
    let mut rng = Rng::new(21);
    let data: Vec<f64> = (0..40).map(|_| rng.range(1.0, 100.0)).collect();
    let centers = [
        v2(-4.6, 1.45),
        v2(0.0, 1.45),
        v2(4.6, 1.45),
        v2(-4.6, -2.1),
        v2(0.0, -2.1),
        v2(4.6, -2.1),
    ];
    let rate = 170.0;
    for (k, algo) in SORT_ALGORITHMS.iter().enumerate() {
        let c = centers[k];
        s.add(frame(c, 4.3, 3.0));
        let tr = Arc::new(sort_trace(algo, &data));
        let name = algo.to_string();
        s.always_redraw(move |ctx| {
            let i = ((ctx.elapsed * rate) as usize).min(tr.len() - 1);
            let finished = i == tr.len() - 1;
            group(vec![
                sort_bars(
                    &tr[i],
                    3.9,
                    2.3,
                    rgb8(90, 110, 160),
                    YELLOW,
                    rgb8(90, 220, 150),
                )
                .move_to(c + v2(0.0, -0.1)),
                text_sized(
                    &format!(
                        "{name} sort · step {i}/{}{}",
                        tr.len() - 1,
                        if finished { "  ✓" } else { "" }
                    ),
                    0.25,
                    if finished { GREEN_B } else { GRAY_A },
                )
                .move_to(c + v2(0.0, -1.72)),
            ])
        });
    }
    s.wait(9.0);
    s.remove_all();

    // ── Act 2: path-finding (9 – 18 s) ──
    s.add(bold_text("Path-finding on the same terrain", 0.42, WHITE).to_edge(UP, 0.25));
    let mut grid = Grid::random(38, 20, 0.2, 5);
    let (start, goal) = ((1, 10), (36, 9));
    for c in [start, goal] {
        let k = grid.idx(c);
        grid.cost[k] = 1;
    }
    let grid = Arc::new(grid);
    let algos = [
        (Search::Bfs, "BFS (ignores cost)"),
        (Search::Dfs, "DFS"),
        (Search::Dijkstra, "Dijkstra"),
        (Search::AStar, "A* (Manhattan heuristic)"),
    ];
    for (k, (alg, name)) in algos.into_iter().enumerate() {
        let c = v2(-3.4 + 6.8 * (k % 2) as f64, 1.2 - 3.4 * (k / 2) as f64);
        let tr = Arc::new(grid_search(&grid, start, goal, alg));
        let g = grid.clone();
        s.always_redraw(move |ctx| {
            let step = (ctx.elapsed * 130.0) as usize;
            let done = step >= tr.visited.len();
            group(vec![
                search_mobject(&g, &tr, step, 0.13, rgb8(255, 120, 160)).move_to(c),
                text_sized(
                    &format!(
                        "{name}: expanded {}{}",
                        step.min(tr.visited.len()),
                        if done {
                            format!(", path cost {}", tr.cost)
                        } else {
                            String::new()
                        }
                    ),
                    0.25,
                    if done { WHITE } else { GRAY_A },
                )
                .move_to(c + v2(0.0, -1.55)),
            ])
        });
    }
    s.add(
        caption(
            "white = cost 1 · sand = cost 5 · dark = wall · green → red",
            v2(0.0, 3.0),
        )
        .scale(0.85),
    );
    s.wait(9.0);
    s.remove_all();

    // ── Act 3: gallery (18 – 28 s) ──
    s.add(bold_text("Data structures, learning & emergence", 0.42, WHITE).to_edge(UP, 0.22));
    let names = [
        "binary search tree insertion",
        "neural network forward pass",
        "Galton board → binomial",
        "bar-chart race",
        "rule 30 / rule 90 automata",
        "Life: Gosper glider gun",
    ];
    for (c, n) in centers.iter().zip(names) {
        s.add(frame(*c, 4.3, 3.0));
        s.add(caption(n, *c + v2(0.0, -1.7)));
    }
    // BST
    let keys = [50, 25, 75, 12, 37, 62, 87, 6, 18, 31, 43, 56, 68, 81, 93];
    let lay = Arc::new(bst_layout(&keys, 0.255, 0.62));
    let c0 = centers[0] + v2(0.0, 1.0);
    s.always_redraw(move |ctx| {
        let upto = (ctx.elapsed * 2.2).min(lay.len() as f64);
        let newest = keys[(upto.ceil() as usize).clamp(1, keys.len()) - 1];
        // search path for the newest key
        let path: Vec<i64> = {
            let mut p = vec![];
            let mut cur = 0usize;
            loop {
                p.push(lay[cur].0);
                if lay[cur].0 == newest {
                    break;
                }
                match lay
                    .iter()
                    .position(|n| n.2 == Some(cur) && ((newest < lay[cur].0) == (n.0 < lay[cur].0)))
                {
                    Some(nx) => cur = nx,
                    None => break,
                }
            }
            p
        };
        bst_mobject(&lay, upto, 0.15, &path).shift(c0)
    });
    // neural net
    let nn = Arc::new(NeuralNet::random(&[4, 7, 7, 3], 8));
    let c1 = centers[1];
    s.always_redraw(move |ctx| {
        let phase = ctx.elapsed / 3.3;
        let k = phase.floor();
        let mut r = Rng::new(k as u64 + 3);
        let input: Vec<f64> = (0..4).map(|_| r.range(-1.0, 1.0)).collect();
        let acts = nn.forward(&input);
        nn.mobject(&acts, 3.5, 2.4, (phase - k) * 4.0)
            .move_to(c1 + v2(0.0, 0.05))
    });
    // Galton
    let paths = Arc::new(galton_paths(200, 9, 4));
    let c2 = centers[2];
    s.always_redraw(move |ctx| {
        galton_mobject(&paths, ctx.elapsed, 0.04, 1.3, 0.3, 0.13, 0.028, 1.0)
            .shift(c2 + v2(0.0, 0.15))
    });
    // bar-chart race
    let series: Arc<Vec<(&str, Color, Vec<f64>)>> = Arc::new(
        [
            ("Rust", ORANGE),
            ("Python", BLUE),
            ("Julia", PURPLE_A),
            ("R", TEAL),
            ("Go", GREEN_B),
            ("C++", PINK),
        ]
        .iter()
        .enumerate()
        .map(|(i, (n, c))| {
            let mut r = Rng::new(i as u64 + 40);
            let mut v = 20.0 + i as f64 * 6.0;
            let growth = r.range(0.0, 14.0);
            let vals = (0..12)
                .map(|_| {
                    v += growth + r.range(0.0, 10.0);
                    v
                })
                .collect();
            (*n, *c, vals)
        })
        .collect(),
    );
    let c3 = centers[3];
    s.always_redraw(move |ctx| {
        let t = (ctx.elapsed * 1.2).min(10.99);
        group(vec![
            bar_chart_race(&series, t, 2.6, 0.32, 6).shift(c3 + v2(-1.0, 1.05)),
            text_sized(&format!("{}", 2015 + t as i32), 0.4, GRAY_C).move_to(c3 + v2(1.5, -1.2)),
        ])
    });
    // elementary CA
    let c4 = centers[4];
    let r30 = elementary_ca(30, 121, 60, None);
    let r90 = elementary_ca(90, 121, 60, None);
    let r30 = Arc::new(r30);
    let r90 = Arc::new(r90);
    s.always_redraw(move |ctx| {
        let rows = ((ctx.elapsed * 12.0) as usize).clamp(1, 60);
        group(vec![
            cells_image(
                Rect::from_center(c4 + v2(-1.02, 0.1), 2.0, 2.0 * rows as f64 / 121.0 * 2.0),
                &r30[..rows],
                rgb8(255, 200, 90),
                rgb8(14, 16, 26),
            )
            .move_to(c4 + v2(-1.02, 1.25 - rows as f64 / 60.0 * 1.0)),
            cells_image(
                Rect::from_center(c4 + v2(1.02, 0.1), 2.0, 2.0 * rows as f64 / 121.0 * 2.0),
                &r90[..rows],
                rgb8(120, 200, 255),
                rgb8(14, 16, 26),
            )
            .move_to(c4 + v2(1.02, 1.25 - rows as f64 / 60.0 * 1.0)),
        ])
    });
    // Life
    let mut life = vec![gosper_glider_gun(64, 40, 2, 2)];
    for _ in 0..330 {
        let next = life_step(life.last().unwrap());
        life.push(next);
    }
    let life = Arc::new(life);
    let c5 = centers[5];
    s.always_redraw(move |ctx| {
        let g = ((ctx.elapsed * 33.0) as usize).min(life.len() - 1);
        cells_image(
            Rect::from_center(c5 + v2(0.0, 0.05), 4.0, 2.5),
            &life[g],
            rgb8(140, 255, 170),
            rgb8(12, 14, 24),
        )
    });
    s.wait(10.0);
    run(&s, "stem_cs")
}
