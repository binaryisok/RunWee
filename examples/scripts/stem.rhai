// STEM showcase from a script: Fourier epicycles, linear maps, domain colouring,
// number-theory art, ripple tanks, optics, orbits, hydrogen orbitals, titration,
// epidemics, sorting, path-finding, neural nets, Galton boards and automata.
// runwee render examples/scripts/stem.rhai -q medium
background("#090a12");

// 1 ── maths: epicycles draw a shape, a matrix bends the plane
let h1 = add(bold_text("Maths", 0.45, WHITE).to_edge(UP, 0.25));
let fr = fourier(text("∑", 3.0), 60);
let epi = always_redraw(|t| {
    let u = (t / 5.0).min(1.0);
    group([epicycle_trace(fr, u, vec2(-4.2, 0.0), "#ffd166"), epicycles(fr, u, vec2(-4.2, 0.0), "#6fa8ff")])
});
let lm = always_redraw(|t| linear_map([1.0, 1.0, 0.5, 1.5], (t / 3.0).min(1.0), 3.0, true).scale(0.6).move_to(vec2(0.2, -0.2)));
let dc = add(domain_coloring("rational", vec2(4.6, 0.9), 3.6, 2.6, 0.9, 150));
let tt = always_redraw(|t| times_table(160, 2.0 + t * 0.4, 1.0, ["#ff6fa0", "#9b8cff", "#4fd1c5"]).move_to(vec2(4.6, -2.2)));
wait(6.0);
remove(h1); remove(epi); remove(lm); remove(dc); remove(tt);

// 2 ── physics: interference, Doppler, dispersion, Kepler
let h2 = add(bold_text("Physics", 0.45, WHITE).to_edge(UP, 0.25));
let rt = always_redraw(|t| ripple_tank([vec2(-5.0, 0.6), vec2(-5.0, -0.6)], vec2(-4.6, 0.0), 4.2, 4.2, 0.45, t * 1.2));
let dp = always_redraw(|t| doppler(vec2(-1.2, 1.3), vec2(0.45, 0.0), t, "#7fdbff"));
let pr = always_redraw(|t| prism_demo(vec2(0.2, -1.6), 0.9, (t / 2.5).min(1.0)));
let kp = always_redraw(|t| kepler_orbit(2.0, 0.6, vec2(4.6, 0.0), t / 3.0, "#ffd166"));
wait(6.0);
remove(h2); remove(rt); remove(dp); remove(pr); remove(kp);

// 3 ── chemistry & biology: orbitals, titration, epidemic
let h3 = add(bold_text("Chemistry & biology", 0.45, WHITE).to_edge(UP, 0.25));
let o1 = add(orbital(2, 1, 0, vec2(-5.6, 1.4), 2.4));
let o2 = add(orbital(3, 2, 0, vec2(-3.0, 1.4), 2.4));
let o3 = add(orbital(4, 3, 1, vec2(-5.6, -1.6), 2.4));
let o4 = add(orbital(3, 2, 2, vec2(-3.0, -1.6), 2.4));
let fl = always_redraw(|t| {
    let vb = t * 9.0;
    let ph = titration_ph(0.1, 25.0, 0.1, vb, 1.8e-5);
    group([
        circle(0.8).color(indicator_color("phenolphthalein", ph)).fill_opacity(0.85).move_to(vec2(0.6, 0.4)),
        text("NaOH " + (vb.round()) + " mL  →  pH " + (ph * 10.0).round() / 10.0, 0.3).move_to(vec2(0.6, -0.9)),
    ])
});
let ep = epidemic(180, vec2(4.4, 0.0), 4.6, 4.6, 6.0, 3);
let ev = always_redraw(|t| epidemic_view(ep, t, 0.06));
wait(6.0);
remove(h3); remove(o1); remove(o2); remove(o3); remove(o4); remove(fl); remove(ev);

// 4 ── computer science: sorting, A*, a neural net, Galton, automata, Life
let h4 = add(bold_text("Computer science", 0.45, WHITE).to_edge(UP, 0.25));
let data = [];
for i in 0..32 { data.push(((i * 37 + 11) % 32 + 1) * 3.0); }
let qs = sort_trace("quick", data);
let srt = always_redraw(|t| sort_bars(qs, t * 60.0, 3.8, 2.2).move_to(vec2(-4.6, 1.5)));
let pa = path_search("astar", 30, 16, 0.2, 5);
let pth = always_redraw(|t| path_view(pa, t * 70.0, 0.13).move_to(vec2(0.0, 1.5)));
let net = neural_net([4, 6, 6, 3], 8);
let nnv = always_redraw(|t| nn_view(net, [0.5, -0.3, 0.9, 0.1], 3.6, 2.4, t).move_to(vec2(4.6, 1.5)));
let gb = galton(150, 9, 4);
let gv = always_redraw(|t| galton_view(gb, t, 0.03).move_to(vec2(-4.6, -2.0)));
let ca = add(cellular(30, 101, 50, vec2(0.0, -2.0), 3.8, "#ffc85a"));
let lf = life_gun(200);
let lv = always_redraw(|t| life_view(lf, t * 33.0, vec2(4.6, -2.0), 3.8, 2.4, "#8cffaa"));
wait(6.0);
