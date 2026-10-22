//! Phase S4 — computer science & data: recorded traces of sorting algorithms,
//! grid path-finding (BFS, DFS, Dijkstra, A*), binary-search-tree layouts, a
//! neural-network forward pass, a Galton board, bar-chart races and cellular automata
//! (elementary rules and Conway's Life). Algorithms run once and record frames;
//! drawing a frame is a pure function, so scenes can scrub freely.

use crate::color::*;
use crate::generative::field_image;
use crate::math::*;
use crate::mobject::{group, Mobject};
use crate::path::Rect;
use crate::shapes::*;
use crate::text::text_sized;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

// ───────────────────────────── sorting ─────────────────────────────

/// One recorded step of a sort: the array and the indices being touched.
#[derive(Clone, Debug, PartialEq)]
pub struct SortFrame {
    pub values: Vec<f64>,
    pub active: Vec<usize>,
    /// Indices known to be in their final position.
    pub done: Vec<bool>,
}

struct Rec {
    frames: Vec<SortFrame>,
    done: Vec<bool>,
}
impl Rec {
    fn snap(&mut self, a: &[f64], active: &[usize]) {
        self.frames.push(SortFrame {
            values: a.to_vec(),
            active: active.to_vec(),
            done: self.done.clone(),
        });
    }
}

/// Names accepted by [`sort_trace`].
pub const SORT_ALGORITHMS: [&str; 6] =
    ["bubble", "insertion", "selection", "quick", "merge", "heap"];

/// Run `algo` on `data`, recording a frame for every comparison/swap/write.
pub fn sort_trace(algo: &str, data: &[f64]) -> Vec<SortFrame> {
    let mut a = data.to_vec();
    let n = a.len();
    let mut r = Rec {
        frames: vec![],
        done: vec![false; n],
    };
    r.snap(&a, &[]);
    match algo {
        "bubble" => {
            for i in 0..n {
                for j in 0..n.saturating_sub(i + 1) {
                    r.snap(&a, &[j, j + 1]);
                    if a[j] > a[j + 1] {
                        a.swap(j, j + 1);
                        r.snap(&a, &[j, j + 1]);
                    }
                }
                r.done[n - 1 - i] = true;
            }
        }
        "insertion" => {
            for i in 1..n {
                let mut j = i;
                while j > 0 && a[j - 1] > a[j] {
                    a.swap(j - 1, j);
                    r.snap(&a, &[j - 1, j]);
                    j -= 1;
                }
                r.snap(&a, &[j]);
            }
        }
        "selection" => {
            for i in 0..n {
                let mut m = i;
                for j in i + 1..n {
                    r.snap(&a, &[m, j]);
                    if a[j] < a[m] {
                        m = j;
                    }
                }
                a.swap(i, m);
                r.done[i] = true;
                r.snap(&a, &[i, m]);
            }
        }
        "quick" => {
            fn qs(a: &mut [f64], lo: isize, hi: isize, r: &mut Rec) {
                if lo >= hi {
                    if lo == hi {
                        r.done[lo as usize] = true;
                    }
                    return;
                }
                let p = a[hi as usize];
                let mut i = lo;
                for j in lo..hi {
                    r.snap(a, &[j as usize, hi as usize]);
                    if a[j as usize] < p {
                        a.swap(i as usize, j as usize);
                        r.snap(a, &[i as usize, j as usize]);
                        i += 1;
                    }
                }
                a.swap(i as usize, hi as usize);
                r.done[i as usize] = true;
                r.snap(a, &[i as usize]);
                qs(a, lo, i - 1, r);
                qs(a, i + 1, hi, r);
            }
            qs(&mut a, 0, n as isize - 1, &mut r);
        }
        "merge" => {
            let mut width = 1;
            while width < n {
                let mut lo = 0;
                while lo < n {
                    let mid = (lo + width).min(n);
                    let hi = (lo + 2 * width).min(n);
                    let (left, right) = (a[lo..mid].to_vec(), a[mid..hi].to_vec());
                    let (mut i, mut j) = (0, 0);
                    for k in lo..hi {
                        let take_left = j >= right.len() || (i < left.len() && left[i] <= right[j]);
                        a[k] = if take_left {
                            i += 1;
                            left[i - 1]
                        } else {
                            j += 1;
                            right[j - 1]
                        };
                        r.snap(&a, &[k]);
                    }
                    lo += 2 * width;
                }
                width *= 2;
            }
        }
        "heap" => {
            fn sift(a: &mut [f64], mut root: usize, end: usize, r: &mut Rec) {
                loop {
                    let mut c = 2 * root + 1;
                    if c >= end {
                        break;
                    }
                    if c + 1 < end && a[c + 1] > a[c] {
                        c += 1;
                    }
                    r.snap(a, &[root, c]);
                    if a[root] < a[c] {
                        a.swap(root, c);
                        r.snap(a, &[root, c]);
                        root = c;
                    } else {
                        break;
                    }
                }
            }
            for s in (0..n / 2).rev() {
                sift(&mut a, s, n, &mut r);
            }
            for end in (1..n).rev() {
                a.swap(0, end);
                r.done[end] = true;
                r.snap(&a, &[0, end]);
                sift(&mut a, 0, end, &mut r);
            }
        }
        _ => a.sort_by(|x, y| x.partial_cmp(y).unwrap()),
    }
    r.done = vec![true; n];
    r.snap(&a, &[]);
    r.frames
}

/// Bars for one sort frame in a `w × h` box centred at the origin (heights ∝ value/max).
pub fn sort_bars(
    frame: &SortFrame,
    w: f64,
    h: f64,
    base: Color,
    active: Color,
    done: Color,
) -> Mobject {
    let n = frame.values.len().max(1);
    let max = frame.values.iter().cloned().fold(1e-9, f64::max);
    let bw = w / n as f64;
    group(
        frame
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let bh = (v / max * h).max(0.02);
                let c = if frame.active.contains(&i) {
                    active
                } else if frame.done[i] {
                    done
                } else {
                    base
                };
                rectangle(bw * 0.82, bh)
                    .fill(c, 1.0)
                    .no_stroke()
                    .move_to(v2(-w / 2.0 + bw * (i as f64 + 0.5), -h / 2.0 + bh / 2.0))
            })
            .collect::<Vec<_>>(),
    )
}

// ───────────────────────────── grid search ─────────────────────────────

/// A grid of cell costs: 0 = wall, ≥ 1 = cost of entering the cell.
#[derive(Clone, Debug)]
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub cost: Vec<u32>,
}

impl Grid {
    /// Random obstacles from smooth noise, with heavier "mud" cells (cost 5).
    pub fn random(cols: usize, rows: usize, wall_density: f64, seed: u64) -> Grid {
        let mut rng = Rng::new(seed);
        let off = v2(rng.range(0.0, 100.0), rng.range(0.0, 100.0));
        let cost = (0..rows * cols)
            .map(|k| {
                let (x, y) = ((k % cols) as f64, (k / cols) as f64);
                let n = crate::noise::fbm2(v2(x, y) * 0.16 + off, 3, 2.0, 0.5, seed);
                let m = crate::noise::fbm2(v2(x, y) * 0.1 - off, 2, 2.0, 0.5, seed + 1);
                if rng.next_f64() < wall_density * (0.5 + n.max(0.0) * 2.0) {
                    0
                } else if m > 0.12 {
                    5
                } else {
                    1
                }
            })
            .collect();
        Grid { cols, rows, cost }
    }
    pub fn idx(&self, c: (usize, usize)) -> usize {
        c.1 * self.cols + c.0
    }
    pub fn open(&self, c: (usize, usize)) -> bool {
        self.cost[self.idx(c)] > 0
    }
    fn neighbours(&self, c: (usize, usize)) -> Vec<(usize, usize)> {
        let (x, y) = (c.0 as isize, c.1 as isize);
        [(1, 0), (0, 1), (-1, 0), (0, -1)]
            .iter()
            .map(|(dx, dy)| (x + dx, y + dy))
            .filter(|&(a, b)| {
                a >= 0 && b >= 0 && (a as usize) < self.cols && (b as usize) < self.rows
            })
            .map(|(a, b)| (a as usize, b as usize))
            .filter(|&c| self.open(c))
            .collect()
    }
}

/// Search algorithm for [`grid_search`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    Bfs,
    Dfs,
    Dijkstra,
    AStar,
}

/// The order cells were expanded in and the final path (empty if unreachable).
#[derive(Clone, Debug, Default)]
pub struct SearchTrace {
    pub visited: Vec<(usize, usize)>,
    pub path: Vec<(usize, usize)>,
    pub cost: u32,
}

/// Run a search from `start` to `goal`. BFS/DFS ignore cell costs; Dijkstra and A*
/// (Manhattan heuristic) minimise total cost.
pub fn grid_search(
    g: &Grid,
    start: (usize, usize),
    goal: (usize, usize),
    algo: Search,
) -> SearchTrace {
    let n = g.cols * g.rows;
    let mut parent = vec![usize::MAX; n];
    let mut dist = vec![u32::MAX; n];
    let mut seen = vec![false; n];
    let mut visited = vec![];
    let h = |c: (usize, usize)| (c.0.abs_diff(goal.0) + c.1.abs_diff(goal.1)) as u32;
    let s = g.idx(start);
    dist[s] = 0;
    match algo {
        Search::Bfs | Search::Dfs => {
            let mut q = VecDeque::from([start]);
            seen[s] = true;
            while let Some(c) = if algo == Search::Bfs {
                q.pop_front()
            } else {
                q.pop_back()
            } {
                visited.push(c);
                if c == goal {
                    break;
                }
                for nb in g.neighbours(c) {
                    let k = g.idx(nb);
                    if !seen[k] {
                        seen[k] = true;
                        parent[k] = g.idx(c);
                        dist[k] = dist[g.idx(c)] + g.cost[k];
                        q.push_back(nb);
                    }
                }
            }
        }
        Search::Dijkstra | Search::AStar => {
            let mut pq = BinaryHeap::new();
            pq.push(Reverse((
                if algo == Search::AStar { h(start) } else { 0 },
                0u32,
                start,
            )));
            while let Some(Reverse((_, d, c))) = pq.pop() {
                let k = g.idx(c);
                if seen[k] {
                    continue;
                }
                seen[k] = true;
                visited.push(c);
                if c == goal {
                    break;
                }
                for nb in g.neighbours(c) {
                    let kk = g.idx(nb);
                    let nd = d + g.cost[kk];
                    if nd < dist[kk] {
                        dist[kk] = nd;
                        parent[kk] = k;
                        let pri = nd + if algo == Search::AStar { h(nb) } else { 0 };
                        pq.push(Reverse((pri, nd, nb)));
                    }
                }
            }
        }
    }
    let gi = g.idx(goal);
    let mut path = vec![];
    if dist[gi] != u32::MAX {
        let mut k = gi;
        while k != usize::MAX {
            path.push((k % g.cols, k / g.cols));
            k = parent[k];
        }
        path.reverse();
    }
    SearchTrace {
        visited,
        cost: if path.is_empty() { 0 } else { dist[gi] },
        path,
    }
}

/// Draw the grid with the first `step` expansions and (once finished) the path.
/// Cell (0, 0) is top-left; the picture is centred at the origin.
pub fn search_mobject(
    g: &Grid,
    tr: &SearchTrace,
    step: usize,
    cell: f64,
    frontier: Color,
) -> Mobject {
    let w = g.cols as f64 * cell;
    let hgt = g.rows as f64 * cell;
    let at = |c: (usize, usize)| {
        v2(
            -w / 2.0 + (c.0 as f64 + 0.5) * cell,
            hgt / 2.0 - (c.1 as f64 + 0.5) * cell,
        )
    };
    let shown = step.min(tr.visited.len());
    let mut order = vec![usize::MAX; g.cost.len()];
    for (i, c) in tr.visited[..shown].iter().enumerate() {
        order[g.idx(*c)] = i;
    }
    let rect = Rect::from_center(Vec2::ZERO, w, hgt);
    let cols = g.cols;
    let cost = g.cost.clone();
    let img = field_image(rect, (g.cols * 6) as u32, move |p| {
        let x = ((p.x + w / 2.0) / cell) as usize;
        let y = ((hgt / 2.0 - p.y) / cell) as usize;
        let k = y.min(cost.len() / cols - 1) * cols + x.min(cols - 1);
        let base = match cost[k] {
            0 => rgb8(30, 32, 44),
            1 => rgb8(232, 232, 238),
            _ => rgb8(196, 168, 120),
        };
        if order[k] != usize::MAX {
            let age = (shown - order[k]) as f64 / 60.0;
            base.mix(frontier.mix(rgb8(60, 110, 200), age.min(1.0) as f32), 0.7)
        } else {
            base
        }
    });
    let mut kids = vec![img];
    if shown >= tr.visited.len() && tr.path.len() > 1 {
        let pts: Vec<Vec2> = tr.path.iter().map(|c| at(*c)).collect();
        kids.push(polyline(&pts).stroke(YELLOW, 3.5).glow(0.12, None));
    }
    if let (Some(a), Some(b)) = (tr.visited.first(), tr.path.last().or(tr.visited.last())) {
        kids.push(dot_r(at(*a), cell * 0.45, GREEN));
        kids.push(dot_r(at(*b), cell * 0.45, RED));
    }
    group(kids)
}

// ───────────────────────────── trees ─────────────────────────────

/// Layout of a binary search tree built by inserting `keys` in order: each node is
/// placed at (in-order index, depth). Returns (key, position, parent index).
pub fn bst_layout(keys: &[i64], dx: f64, dy: f64) -> Vec<(i64, Vec2, Option<usize>)> {
    let mut left: Vec<Option<usize>> = vec![];
    let mut right: Vec<Option<usize>> = vec![];
    let mut parent: Vec<Option<usize>> = vec![];
    let mut depth: Vec<usize> = vec![];
    for (i, &k) in keys.iter().enumerate() {
        left.push(None);
        right.push(None);
        if i == 0 {
            parent.push(None);
            depth.push(0);
            continue;
        }
        let mut cur = 0;
        let mut d = 1;
        loop {
            let slot = if k < keys[cur] {
                &mut left[cur]
            } else {
                &mut right[cur]
            };
            match *slot {
                Some(nx) => {
                    cur = nx;
                    d += 1;
                }
                None => {
                    *slot = Some(i);
                    break;
                }
            }
        }
        parent.push(Some(cur));
        depth.push(d);
    }
    // in-order rank
    let mut rank = vec![0usize; keys.len()];
    let mut counter = 0;
    fn walk(
        n: Option<usize>,
        l: &[Option<usize>],
        r: &[Option<usize>],
        rank: &mut [usize],
        c: &mut usize,
    ) {
        if let Some(i) = n {
            walk(l[i], l, r, rank, c);
            rank[i] = *c;
            *c += 1;
            walk(r[i], l, r, rank, c);
        }
    }
    if !keys.is_empty() {
        walk(Some(0), &left, &right, &mut rank, &mut counter);
    }
    let mid = (keys.len() as f64 - 1.0) / 2.0;
    keys.iter()
        .enumerate()
        .map(|(i, &k)| {
            (
                k,
                v2((rank[i] as f64 - mid) * dx, -(depth[i] as f64) * dy),
                parent[i],
            )
        })
        .collect()
}

/// Draw the first `upto` inserted nodes of a [`bst_layout`] (fractional `upto` fades
/// the newest node in), highlighting `highlight` keys.
pub fn bst_mobject(
    layout: &[(i64, Vec2, Option<usize>)],
    upto: f64,
    r: f64,
    highlight: &[i64],
) -> Mobject {
    let n = (upto.ceil() as usize).min(layout.len());
    let mut edges = vec![];
    let mut nodes = vec![];
    for (i, (k, p, par)) in layout.iter().enumerate().take(n) {
        let a = if i + 1 == n && upto.fract() > 0.0 {
            upto.fract() as f32
        } else {
            1.0
        };
        if let Some(pi) = par {
            edges.push(line(layout[*pi].1, *p).stroke(GRAY_B.with_alpha(a), 2.0));
        }
        let c = if highlight.contains(k) {
            ORANGE
        } else {
            BLUE_D
        };
        nodes.push(
            group(vec![
                circle(r).fill(c, a).stroke(WHITE.with_alpha(a), 1.5),
                text_sized(&k.to_string(), r * 1.0, WHITE.with_alpha(a)),
            ])
            .move_to(*p),
        );
    }
    edges.extend(nodes);
    group(edges)
}

// ───────────────────────────── neural network ─────────────────────────────

/// A dense feed-forward network with tanh hidden layers and deterministic weights.
#[derive(Clone, Debug)]
pub struct NeuralNet {
    pub sizes: Vec<usize>,
    /// weights[l][j][i]: from neuron i of layer l to neuron j of layer l+1.
    pub weights: Vec<Vec<Vec<f64>>>,
    pub biases: Vec<Vec<f64>>,
}

impl NeuralNet {
    pub fn random(sizes: &[usize], seed: u64) -> NeuralNet {
        let mut rng = Rng::new(seed);
        let mut weights = vec![];
        let mut biases = vec![];
        for w in sizes.windows(2) {
            let s = 1.0 / (w[0] as f64).sqrt();
            weights.push(
                (0..w[1])
                    .map(|_| (0..w[0]).map(|_| rng.normal() * s * 1.6).collect())
                    .collect(),
            );
            biases.push((0..w[1]).map(|_| rng.normal() * 0.2).collect());
        }
        NeuralNet {
            sizes: sizes.to_vec(),
            weights,
            biases,
        }
    }
    /// Activations of every layer for `input` (tanh hidden, softmax output).
    pub fn forward(&self, input: &[f64]) -> Vec<Vec<f64>> {
        let mut acts = vec![input.to_vec()];
        for (l, (w, b)) in self.weights.iter().zip(&self.biases).enumerate() {
            let prev = acts.last().unwrap();
            let z: Vec<f64> = w
                .iter()
                .zip(b)
                .map(|(row, bj)| row.iter().zip(prev).map(|(a, x)| a * x).sum::<f64>() + bj)
                .collect();
            let out = if l + 1 == self.weights.len() {
                let m = z.iter().cloned().fold(f64::MIN, f64::max);
                let e: Vec<f64> = z.iter().map(|v| (v - m).exp()).collect();
                let s: f64 = e.iter().sum();
                e.iter().map(|v| v / s).collect()
            } else {
                z.iter().map(|v| v.tanh()).collect()
            };
            acts.push(out);
        }
        acts
    }
    /// Network diagram `w × h` at the origin; `progress` ∈ [0, layers] sweeps a
    /// signal through the layers, lighting neurons by activation.
    pub fn mobject(&self, acts: &[Vec<f64>], w: f64, h: f64, progress: f64) -> Mobject {
        let layers = self.sizes.len();
        let pos = |l: usize, i: usize| {
            let n = self.sizes[l];
            let x = -w / 2.0 + w * l as f64 / (layers - 1).max(1) as f64;
            let gap = (h / n.max(1) as f64).min(0.55);
            v2(x, (i as f64 - (n as f64 - 1.0) / 2.0) * gap)
        };
        let mut kids = vec![];
        for (l, wl) in self.weights.iter().enumerate() {
            let lit = (progress - l as f64).clamp(0.0, 1.0);
            for (j, row) in wl.iter().enumerate() {
                for (i, wt) in row.iter().enumerate() {
                    let base = if *wt > 0.0 {
                        rgb8(255, 140, 80)
                    } else {
                        rgb8(90, 150, 255)
                    };
                    let strength = (wt.abs() / 1.5).min(1.0) as f32;
                    let signal = (acts[l][i].abs() as f32).min(1.0);
                    let a = 0.08 + 0.5 * strength * (lit as f32) * signal;
                    let (p, q) = (pos(l, i), pos(l + 1, j));
                    kids.push(line(p, q).stroke(base.with_alpha(a), 0.6 + 1.6 * strength));
                    if lit > 0.0 && lit < 1.0 && strength > 0.4 {
                        kids.push(dot_r(p.lerp(q, lit), 0.025, base.mix(WHITE, 0.5)));
                    }
                }
            }
        }
        for l in 0..layers {
            let lit = (progress - l as f64 + 1.0).clamp(0.0, 1.0) as f32;
            for i in 0..self.sizes[l] {
                let a = acts[l][i];
                let c = if l + 1 == layers {
                    rgb8(30, 30, 40).mix(YELLOW, a as f32 * lit)
                } else if a >= 0.0 {
                    rgb8(30, 30, 40).mix(rgb8(255, 160, 90), a.min(1.0) as f32 * lit)
                } else {
                    rgb8(30, 30, 40).mix(rgb8(100, 160, 255), (-a).min(1.0) as f32 * lit)
                };
                kids.push(
                    circle(0.16)
                        .fill(c, 1.0)
                        .stroke(WHITE, 1.5)
                        .move_to(pos(l, i)),
                );
            }
        }
        group(kids)
    }
}

// ───────────────────────────── probability ─────────────────────────────

/// Galton board: for each ball, its left/right choices at each of `rows` peg rows.
pub fn galton_paths(balls: usize, rows: usize, seed: u64) -> Vec<Vec<bool>> {
    let mut rng = Rng::new(seed);
    (0..balls)
        .map(|_| (0..rows).map(|_| rng.next_f64() < 0.5).collect())
        .collect()
}

/// Galton board picture at time `t`: balls are released every `interval` and take
/// `fall` seconds to drop; landed balls stack into bins next to the binomial curve.
#[allow(clippy::too_many_arguments)]
pub fn galton_mobject(
    paths: &[Vec<bool>],
    t: f64,
    interval: f64,
    fall: f64,
    peg_dx: f64,
    peg_dy: f64,
    ball_r: f64,
    bin_h: f64,
) -> Mobject {
    let rows = paths.first().map_or(0, |p| p.len());
    let top = v2(0.0, rows as f64 * peg_dy / 2.0 + 0.3);
    let peg = |r: usize, k: usize| {
        top + v2(
            (k as f64 - r as f64 / 2.0) * peg_dx,
            -(r as f64 + 1.0) * peg_dy,
        )
    };
    let floor = top.y - (rows as f64 + 1.0) * peg_dy - bin_h;
    let mut kids = vec![];
    for r in 0..rows {
        for k in 0..=r {
            kids.push(dot_r(peg(r, k), 0.035, GRAY_B));
        }
    }
    for b in 0..=rows + 1 {
        let x = (b as f64 - (rows as f64 + 1.0) / 2.0) * peg_dx;
        kids.push(line(v2(x, floor), v2(x, floor + bin_h)).stroke(GRAY_C, 1.2));
    }
    kids.push(
        line(
            v2(-(rows as f64 + 1.0) / 2.0 * peg_dx, floor),
            v2((rows as f64 + 1.0) / 2.0 * peg_dx, floor),
        )
        .stroke(GRAY_B, 2.0),
    );
    let mut counts = vec![0usize; rows + 1];
    let stack = ball_r * 1.8;
    // balls pack several abreast inside each bin
    let per_row = ((peg_dx * 0.9) / (2.0 * ball_r)).floor().max(1.0) as usize;
    for (i, p) in paths.iter().enumerate() {
        let age = t - i as f64 * interval;
        if age < 0.0 {
            break;
        }
        let bin = p.iter().filter(|&&r| r).count();
        let c = color_gradient(&[BLUE_B, TEAL, YELLOW], bin as f64 / rows.max(1) as f64);
        if age >= fall {
            let col = counts[bin] % per_row;
            let x = (bin as f64 - rows as f64 / 2.0) * peg_dx
                + (col as f64 - (per_row as f64 - 1.0) / 2.0) * 2.0 * ball_r;
            let y = floor + ball_r + (counts[bin] / per_row) as f64 * stack;
            counts[bin] += 1;
            kids.push(dot_r(v2(x, y), ball_r, c));
        } else {
            // bounce from peg to peg
            let u = age / fall * (rows as f64 + 1.0);
            let r = u.floor() as usize;
            let f = u.fract();
            let k_at = |r: usize| p[..r.min(rows)].iter().filter(|&&x| x).count();
            let a = if r == 0 { top } else { peg(r - 1, k_at(r - 1)) } + v2(0.0, ball_r);
            let b = if r < rows {
                peg(r, k_at(r)) + v2(0.0, ball_r)
            } else {
                v2(
                    (bin as f64 - rows as f64 / 2.0) * peg_dx,
                    floor + bin_h * 0.6,
                )
            };
            let pos = a.lerp(b, f) + v2(0.0, 0.12 * (f * std::f64::consts::PI).sin());
            kids.push(dot_r(pos, ball_r, c));
        }
    }
    // expected binomial profile
    let landed: usize = counts.iter().sum();
    if landed > 0 {
        let pts: Vec<Vec2> = (0..=rows)
            .map(|k| {
                let e = landed as f64 * crate::math::choose(rows as u32, k as u32)
                    / 2f64.powi(rows as i32);
                v2(
                    (k as f64 - rows as f64 / 2.0) * peg_dx,
                    floor + e / per_row as f64 * stack + ball_r,
                )
            })
            .collect();
        kids.push(polyline(&pts).stroke(PINK, 2.5));
    }
    group(kids)
}

/// Bar-chart race frame: `series` of (name, colour, values over time); `t` is a
/// continuous index into the values. Ranks and lengths are interpolated smoothly.
pub fn bar_chart_race(
    series: &[(&str, Color, Vec<f64>)],
    t: f64,
    w: f64,
    bar_h: f64,
    top_n: usize,
) -> Mobject {
    let at = |v: &[f64]| {
        let i = (t.floor() as usize).min(v.len() - 1);
        let j = (i + 1).min(v.len() - 1);
        lerp(v[i], v[j], t - t.floor())
    };
    let vals: Vec<f64> = series.iter().map(|s| at(&s.2)).collect();
    // smooth rank: rank at floor and ceil, interpolated
    let rank_at = |tt: f64| -> Vec<f64> {
        let v: Vec<f64> = series
            .iter()
            .map(|s| s.2[(tt as usize).min(s.2.len() - 1)])
            .collect();
        (0..v.len())
            .map(|i| v.iter().filter(|&&x| x > v[i]).count() as f64)
            .collect()
    };
    let (r0, r1) = (rank_at(t.floor()), rank_at(t.floor() + 1.0));
    let f = smoothstep(0.0, 1.0, t - t.floor());
    let max = vals.iter().cloned().fold(1e-9, f64::max);
    let mut kids = vec![];
    for (i, (name, c, _)) in series.iter().enumerate() {
        let rank = lerp(r0[i], r1[i], f);
        if rank > top_n as f64 - 0.01 {
            continue;
        }
        let len = vals[i] / max * w;
        let y = -rank * bar_h * 1.25;
        kids.push(
            rectangle(len.max(0.01), bar_h)
                .fill(*c, 0.95)
                .no_stroke()
                .move_to(v2(len / 2.0, y)),
        );
        kids.push(text_sized(name, bar_h * 0.55, WHITE).move_to(v2(-0.55, y)));
        kids.push(
            text_sized(&format!("{:.0}", vals[i]), bar_h * 0.5, GRAY_A).move_to(v2(len + 0.4, y)),
        );
    }
    group(kids)
}

// ───────────────────────────── cellular automata ─────────────────────────────

/// Elementary (1-D, 2-state, radius 1) cellular automaton history from a single
/// live centre cell (or `init` when given). Wraps at the edges.
pub fn elementary_ca(
    rule: u8,
    width: usize,
    steps: usize,
    init: Option<Vec<bool>>,
) -> Vec<Vec<bool>> {
    let mut row = init.unwrap_or_else(|| {
        let mut r = vec![false; width];
        r[width / 2] = true;
        r
    });
    let mut out = vec![row.clone()];
    for _ in 1..steps {
        let n = row.len();
        row = (0..n)
            .map(|i| {
                let l = row[(i + n - 1) % n] as u8;
                let c = row[i] as u8;
                let r = row[(i + 1) % n] as u8;
                (rule >> (l << 2 | c << 1 | r)) & 1 == 1
            })
            .collect();
        out.push(row.clone());
    }
    out
}

/// One generation of Conway's Game of Life on a torus.
pub fn life_step(g: &[Vec<bool>]) -> Vec<Vec<bool>> {
    let h = g.len();
    let w = g.first().map_or(0, |r| r.len());
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    let mut n = 0;
                    for dy in [h - 1, 0, 1] {
                        for dx in [w - 1, 0, 1] {
                            if (dx, dy) != (0, 0) && g[(y + dy) % h][(x + dx) % w] {
                                n += 1;
                            }
                        }
                    }
                    n == 3 || (n == 2 && g[y][x])
                })
                .collect()
        })
        .collect()
}

/// Gosper glider gun placed at (x, y) in a `w × h` Life grid.
pub fn gosper_glider_gun(w: usize, h: usize, x: usize, y: usize) -> Vec<Vec<bool>> {
    const GUN: [(usize, usize); 36] = [
        (24, 0),
        (22, 1),
        (24, 1),
        (12, 2),
        (13, 2),
        (20, 2),
        (21, 2),
        (34, 2),
        (35, 2),
        (11, 3),
        (15, 3),
        (20, 3),
        (21, 3),
        (34, 3),
        (35, 3),
        (0, 4),
        (1, 4),
        (10, 4),
        (16, 4),
        (20, 4),
        (21, 4),
        (0, 5),
        (1, 5),
        (10, 5),
        (14, 5),
        (16, 5),
        (17, 5),
        (22, 5),
        (24, 5),
        (10, 6),
        (16, 6),
        (24, 6),
        (11, 7),
        (15, 7),
        (12, 8),
        (13, 8),
    ];
    let mut g = vec![vec![false; w]; h];
    for (dx, dy) in GUN {
        if y + dy < h && x + dx < w {
            g[y + dy][x + dx] = true;
        }
    }
    g
}

/// Render a boolean grid as a crisp image filling `rect` (row 0 at the top).
pub fn cells_image(rect: Rect, cells: &[Vec<bool>], on: Color, off: Color) -> Mobject {
    let h = cells.len().max(1);
    let w = cells.first().map_or(1, |r| r.len().max(1));
    let cells = cells.to_vec();
    let px = (w * (1200 / w).clamp(1, 6)) as u32;
    field_image(rect, px, move |p| {
        let x = (((p.x - rect.min.x) / rect.width()) * w as f64) as usize;
        let y = (((rect.max.y - p.y) / rect.height()) * h as f64) as usize;
        if cells[y.min(h - 1)][x.min(w - 1)] {
            on
        } else {
            off
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sort_sorts() {
        let mut rng = Rng::new(4);
        let data: Vec<f64> = (0..37).map(|_| rng.range(0.0, 100.0)).collect();
        let mut expect = data.clone();
        expect.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for algo in SORT_ALGORITHMS {
            let tr = sort_trace(algo, &data);
            assert_eq!(tr.last().unwrap().values, expect, "{algo}");
            assert!(tr.len() > 37, "{algo} recorded too few frames");
        }
    }

    #[test]
    fn searches_agree_on_reachability() {
        let mut g = Grid::random(30, 20, 0.18, 9);
        g.cost[0] = 1;
        let last = g.cost.len() - 1;
        g.cost[last] = 1;
        let (s, t) = ((0, 0), (29, 19));
        let bfs = grid_search(&g, s, t, Search::Bfs);
        let dij = grid_search(&g, s, t, Search::Dijkstra);
        let ast = grid_search(&g, s, t, Search::AStar);
        let dfs = grid_search(&g, s, t, Search::Dfs);
        assert_eq!(bfs.path.is_empty(), dij.path.is_empty());
        assert_eq!(dij.cost, ast.cost);
        if !dij.path.is_empty() {
            assert!(dij.cost <= dfs.cost);
            assert!(ast.visited.len() <= dij.visited.len());
            assert_eq!(*dij.path.first().unwrap(), s);
            assert_eq!(*dij.path.last().unwrap(), t);
        }
    }

    #[test]
    fn bst_inorder_is_sorted() {
        let keys = [50, 30, 70, 20, 40, 60, 80, 35];
        let lay = bst_layout(&keys, 1.0, 1.0);
        let mut by_x = lay.clone();
        by_x.sort_by(|a, b| a.1.x.partial_cmp(&b.1.x).unwrap());
        let ks: Vec<i64> = by_x.iter().map(|n| n.0).collect();
        assert_eq!(ks, vec![20, 30, 35, 40, 50, 60, 70, 80]);
        assert_eq!(lay[7].1.y, -3.0);
    }

    #[test]
    fn automata() {
        let r90 = elementary_ca(90, 31, 16, None);
        // rule 90 from one cell is Pascal mod 2 → row 2^k has exactly 2 cells
        assert_eq!(r90[8].iter().filter(|&&b| b).count(), 2);
        let mut g = vec![vec![false; 6]; 6];
        for (x, y) in [(1, 2), (2, 2), (3, 2)] {
            g[y][x] = true;
        }
        let g2 = life_step(&life_step(&g));
        assert_eq!(g, g2); // blinker period 2
        let gun = gosper_glider_gun(40, 12, 1, 1);
        assert_eq!(gun.iter().flatten().filter(|&&b| b).count(), 36);
    }

    #[test]
    fn nn_softmax() {
        let nn = NeuralNet::random(&[4, 6, 5, 3], 2);
        let acts = nn.forward(&[0.5, -0.2, 0.9, 0.1]);
        assert_eq!(acts.len(), 4);
        assert!((acts[3].iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn galton_bins() {
        let p = galton_paths(500, 10, 1);
        let mean = p
            .iter()
            .map(|b| b.iter().filter(|&&x| x).count() as f64)
            .sum::<f64>()
            / 500.0;
        assert!((mean - 5.0).abs() < 0.3);
    }
}
