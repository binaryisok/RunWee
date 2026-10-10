// Constrained Motion — JEE Mains / Advanced / Olympiads
//
// A RunWee port of the "constrained-motion" lesson from anims_bms
// (topics/constrained-motion: the narration in index.js, the 8 presets in
// simulations.js, the equation overlays in mathOverlay.js and the
// derivations). The same 28 narration steps drive the same systems; every
// slider sweep in the original becomes a tracker animation under the voice.
//
//   runwee render examples/scripts/constrained_motion.rhai -q 2k
//   RUNWEE_TTS='piper -m en-us-lessac-medium.onnx -f {out}' runwee render ...   (spoken voice)
//
// Note: script `fn`s can't see global constants, so helpers use hex colours.

captions("karaoke");
background("spotlight");
music(0.05);

// ─────────────────────────── drawing helpers ───────────────────────────

fn ground(x0, x1, y) {
    let g = group([line(vec2(x0, y), vec2(x1, y)).stroke("#BBBBBB", 4.0)]);
    let x = x0 + 0.1;
    while x < x1 {
        g = g.add(line(vec2(x, y), vec2(x - 0.16, y - 0.2)).stroke("#555555", 1.6));
        x += 0.24;
    }
    g
}

fn ceiling(x0, x1, y) {
    let g = group([line(vec2(x0, y), vec2(x1, y)).stroke("#BBBBBB", 4.0)]);
    let x = x0 + 0.1;
    while x < x1 {
        g = g.add(line(vec2(x, y), vec2(x + 0.16, y + 0.2)).stroke("#555555", 1.6));
        x += 0.24;
    }
    g
}

fn wall(x, y0, y1) {
    let g = group([line(vec2(x, y0), vec2(x, y1)).stroke("#BBBBBB", 4.0)]);
    let y = y0 + 0.1;
    while y < y1 {
        g = g.add(line(vec2(x, y), vec2(x - 0.2, y - 0.16)).stroke("#555555", 1.6));
        y += 0.24;
    }
    g
}

fn pulley(c, r, col) {
    group([
        circle_at(c, r).fill("#12161E", 1.0).stroke(col, 4.0),
        dot(c, 0.06, col),
    ])
}

fn block(c, w, h, col, lbl) {
    group([
        rectangle(w, h).move_to(c).fill(col, 0.22).stroke(col, 3.0),
        tex(lbl, 0.42, col).move_to(c),
    ])
}

// A block resting on a slope: `p` is its lower corner on the surface, `dir` the
// unit vector down the slope, `n` the outward normal, `ang` the tilt (radians).
fn slope_block(p, dir, n, w, h, ang, col, lbl) {
    let c = p + dir * (w / 2.0) + n * (h / 2.0);
    group([
        rectangle(w, h).rotate(ang).move_to(c).fill(col, 0.22).stroke(col, 3.0),
        tex(lbl, 0.4, col).move_to(c),
    ])
}

// Vector arrow with a label just past its tip (nothing if it's tiny).
fn varrow(a, d, col, lbl) {
    if d.length() < 0.06 {
        return empty();
    }
    let b = a + d;
    group([
        arrow(a, b).color(col).stroke_width(5.0),
        tex(lbl, 0.36, col).move_to(b + d.normalize() * 0.34),
    ])
}

fn r2(x) {
    (x * 100.0).round() / 100.0
}

// ─────────────────────────── layout helpers ────────────────────────────

// Scene header (top-left) and the derivation board on the right.
fn scene_chrome(part, name) {
    let kicker = text(part, 0.22, "#888888");
    kicker = kicker.shift(vec2(-6.75 - kicker.left.x, 3.7 - kicker.top.y));
    let ttl = bold_text(name, 0.42, "#FFFFFF");
    ttl = ttl.shift(vec2(-6.75 - ttl.left.x, 3.38 - ttl.top.y));
    let panel = rounded_rectangle(5.15, 6.2, 0.18)
        .move_to(vec2(4.48, 0.0))
        .fill("#141924", 0.9)
        .stroke("#343E54", 1.5);
    let k = register(kicker);
    let t = register(ttl);
    let p = register(panel);
    play([fade_in(k, vec2(1.0, 0.0)), write(t), fade_in(p)], 0.8);
}

// Equations and notes appear on the board as the narration reaches them.
// items: [[fraction of the narration, kind, content], ...]
// kinds: "title", "text", "math", "key" (boxed result)
fn board(info, items) {
    let y = 2.82;
    let x0 = 2.15;
    let objs = [];
    for it in items {
        wait_until(info.start + info.duration * it[0]);
        let kind = it[1];
        let m = if kind == "title" {
            bold_text(it[2], 0.31, "#F7D96F")
        } else if kind == "text" {
            paragraph(it[2], 4.5, 0.25).color("#C8CDD5")
        } else if kind == "key" {
            tex(it[2], 0.54, "#83C167")
        } else {
            tex(it[2], 0.5, "#FFFFFF")
        };
        if m.width > 4.55 {
            m = m.set_width(4.55);
        }
        m = m.shift(vec2(x0 - m.left.x, y - m.top.y));
        y -= m.height + if kind == "title" { 0.3 } else { 0.19 };
        let o = register(m);
        play(fade_in(o, vec2(0.0, 1.0)), 0.45);
        objs.push(o);
        if kind == "key" {
            let b = register(surrounding_rectangle(m, 0.1).color("#83C167"));
            play(create(b), 0.4);
            objs.push(b);
            y -= 0.12;
        }
    }
    objs
}

fn clear_board(objs) {
    if objs.len() > 0 {
        play(objs.map(|o| fade_out(o)), 0.4);
    }
}

// ═════════════════════════════ TITLE ═══════════════════════════════════
let t1 = register(bold_text("Constrained Motion", 0.95, WHITE).shift(UP * 0.55));
let t2 = register(text("JEE Mains  ·  Advanced  ·  Olympiads", 0.34, GRAY_B).shift(DOWN * 0.35));
let t3 = register(text("Part 1 — What is a constraint?", 0.3, YELLOW).shift(DOWN * 1.1));
sound("whoosh", 0.4);
play(write(t1), 1.2);
play([fade_in(t2, UP), fade_in(t3, UP)], 0.8);
wait(1.4);

// ═══════════════════ PART 1 · INTRODUCTION (intro_1..10) ═══════════════

// ── A block on a table, then off the edge (intro_1..3) ──
next_scene("fade", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 1", "What is a constraint?");
let tt = tracker(0.0);
always_redraw(|| {
    let tv = value(tt);
    let xe = -1.6;
    let ty = 0.0;
    let fy = -2.5;
    let g = group([
        ground(-6.9, 1.4, fy),
        rectangle(0.18, ty - fy).move_to(vec2(xe - 0.09, (ty + fy) / 2.0)).fill(GRAY_D, 1.0).no_stroke(),
        rectangle(0.18, ty - fy).move_to(vec2(-6.35, (ty + fy) / 2.0)).fill(GRAY_D, 1.0).no_stroke(),
        line(vec2(-6.6, ty), vec2(xe, ty)).stroke(GRAY_A, 6.0),
    ]);
    let land = (2.2 / 1.2).sqrt();
    let falling = tv > 4.0;
    let bx = xe - 0.45 - (4.0 - tv) * 1.0;
    let by = ty;
    if falling {
        let d = tv - 4.0;
        if d > land { d = land; }
        bx = xe - 0.45 + 1.0 * d;
        by = ty - 1.2 * d * d;
        let pts = [];
        let k = 0;
        while k <= 24 {
            let dd = d * k / 24.0;
            pts.push(vec2(xe - 0.45 + dd, ty + 0.3 - 1.2 * dd * dd));
            k += 1;
        }
        g = g.add(polyline(pts).stroke(RED, 2.5).opacity(0.55));
    }
    let c = vec2(bx, by + 0.3);
    g = g.add(block(c, 0.9, 0.6, BLUE, `m`));
    if !falling {
        g = g.add(varrow(c + vec2(0.45, 0.0), vec2(1.1, 0.0), GREEN, `F`));
        g = g.add(text("Constraint: moves horizontally on the table  (y = const)", 0.25, GRAY_A).move_to(vec2(-2.75, 1.3)));
    } else {
        g = g.add(varrow(c, vec2(0.0, -1.0), RED, `mg`));
        g = g.add(text("Constraint ends → projectile motion (free fall)", 0.25, RED).move_to(vec2(-2.75, 1.3)));
    }
    g
});

voiceover("Let's discuss constrained motion. When we talk about constrained motion, we can simply state it is the motion of a body restricted under some physical conditions, or governed by some mathematical rules. As you know, in layman's terms, the word constrained means some restriction.", |info| {
    let b = board(info, [
        [0.0, "title", "Constrained Motion"],
        [0.1, "text", "Motion restricted by physical conditions,"],
        [0.38, "text", "or governed by mathematical constraint equations."],
        [0.7, "text", "A constraint removes degrees of freedom."],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("Consider a horizontal plane on which a body is placed. If it is pulled by a horizontal force F, the body will move in a straight line. Due to the ground, the body is constrained to move along a straight line.", |info| {
    play_bg(tt.to(4.0).rate("linear"), info.duration);
    let b = board(info, [
        [0.0, "title", "Horizontal table constraint"],
        [0.15, "text", "Block of mass m pulled horizontally by F."],
        [0.5, "text", "The ground stops any vertical motion:"],
        [0.62, "math", `y = \text{const} \;\Rightarrow\; v_y = 0 \;\Rightarrow\; a_y = 0`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("But as soon as the body reaches the corner of the table, its constraint for moving in a horizontal straight line will no longer exist, and it will start falling in a projectile like motion.", |info| {
    play_bg(tt.to(8.0).rate("linear"), info.duration);
    let b = board(info, [
        [0.0, "title", "The constraint ends"],
        [0.18, "text", "At the edge the normal force N drops to zero."],
        [0.45, "text", "Now it is in free fall under gravity:"],
        [0.62, "math", `y(t) = y_0 - \tfrac{1}{2} g t^2, \qquad a_y = -g`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
wait(0.3);

// ── Block on an incline (intro_4) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 1", "Inclined-plane constraint");
let ti = tracker(0.0);
always_redraw(|| {
    let tv = value(ti);
    let al = 30.0 * DEG;
    let fy = -2.5;
    let len = 5.6;
    let peak = vec2(-6.1, fy + len * al.sin());
    let foot = vec2(-6.1 + len * al.cos(), fy);
    let dir = vec2(al.cos(), -al.sin());
    let n = vec2(al.sin(), al.cos());
    let g = group([
        ground(-6.9, 1.3, fy),
        polygon([peak, foot, vec2(-6.1, fy)]).fill(PURPLE, 0.12).stroke(PURPLE, 3.0),
        angle_arc(foot, vec2(0.0, 0.0) - dir, vec2(-1.0, 0.0), 0.7).color(GOLD),
        tex(`\alpha = 30^\circ`, 0.4, GOLD).move_to(foot + vec2(-1.35, 0.25)),
    ]);
    let s = 0.4 + 3.3 * (tv / 8.0) * (tv / 8.0);
    let p = peak + dir * s;
    g = g.add(slope_block(p, dir, n, 0.95, 0.62, -al, BLUE, `m`));
    let c = p + dir * 0.475 + n * 0.31;
    g = g.add(varrow(c, vec2(0.0, -1.1), RED, `mg`));
    g = g.add(varrow(c, n * 0.95, YELLOW, `N`));
    g = g.add(varrow(c, dir * 0.75, GREEN, `mg\sin\alpha`));
    g = g.add(text("Constraint: slides along the incline only", 0.25, GRAY_A).move_to(vec2(-2.75, 2.2)));
    g
});
voiceover("Similarly, if there is an inclined plane having an angle of inclination alpha, and a body is placed on it. Due to m g sine alpha, the body starts sliding along the inclined plane. Here the body is constrained to move along the inclined surface only.", |info| {
    play_bg(ti.to(8.0).rate("linear"), info.duration);
    let b = board(info, [
        [0.0, "title", "Inclined plane constraint"],
        [0.25, "text", "Gravity acts vertically down:"],
        [0.3, "math", `\vec{F}_g = -mg\,\hat{j}`],
        [0.4, "text", "Resolve along and normal to the incline:"],
        [0.5, "math", `F_{\parallel} = mg\sin\alpha, \qquad N = mg\cos\alpha`],
        [0.75, "text", "No motion off the surface, so:"],
        [0.82, "key", `a_{\perp} = 0 \;\Rightarrow\; a_{\parallel} = g\sin\alpha`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
wait(0.3);

// ── Pulley with two blocks (intro_5) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 1", "String constraint");
let tp = tracker(0.0);
always_redraw(|| {
    let tv = value(tp);
    let pc = vec2(-3.4, 1.7);
    let r = 0.5;
    let dy = 1.3 * (tv / 8.0) * (tv / 8.0);
    let ya = -0.4 + dy;
    let yb = -0.4 - dy;
    let xa = pc.x - r;
    let xb = pc.x + r;
    let g = group([
        ceiling(-4.7, -2.1, 2.9),
        line(vec2(pc.x, 2.9), pc).stroke(GRAY_C, 4.0),
        line(vec2(xa, ya), vec2(xa, pc.y)).stroke(GRAY_B, 3.0),
        line(vec2(xb, yb), vec2(xb, pc.y)).stroke(GRAY_B, 3.0),
        arc(r, 0.0, PI).shift(pc).stroke(GRAY_B, 3.0),
        pulley(pc, r, BLUE),
        block(vec2(xa, ya - 0.4), 0.8, 0.8, GREEN, `A`),
        block(vec2(xb, yb - 0.4), 0.8, 0.8, RED, `B`),
        dashed_line(vec2(xa - 1.2, -0.4), vec2(xb + 1.2, -0.4)).color(GRAY_D),
    ]);
    if dy > 0.08 {
        g = g.add(varrow(vec2(xa - 0.95, -0.4), vec2(0.0, dy), GREEN, `y_A`));
        g = g.add(varrow(vec2(xb + 0.95, -0.4), vec2(0.0, -dy), RED, `y_B`));
    }
    g = g.add(text("String constraint: equal and opposite displacements", 0.25, GRAY_A).move_to(vec2(-2.75, -2.6)));
    g
});
voiceover("Similarly, if we talk about a pulley connected to the ceiling with two blocks A and B: mass of block B is greater than A. Block B will go down and block A will go up. As they are connected by a string, their displacements are equal and opposite, and so are their speeds and accelerations.", |info| {
    play_bg(tp.to(8.0).rate("linear"), info.duration);
    let b = board(info, [
        [0.0, "title", "Pulley and string constraint"],
        [0.2, "text", "A and B hang from one inextensible string."],
        [0.4, "text", "Measure y_A, y_B downward from the pulley:"],
        [0.5, "math", `L = y_A + y_B + \text{const}`],
        [0.65, "math", `\Delta y_A + \Delta y_B = 0 \;\Rightarrow\; \Delta y_A = -\Delta y_B`],
        [0.82, "key", `v_A = -v_B, \qquad a_A = -a_B`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
wait(0.3);

// ── Block on a moving wedge (intro_6, intro_7) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 1", "Wedge constraint");
let tw = tracker(0.0);
always_redraw(|| {
    let tv = value(tw);
    let th = 30.0 * DEG;
    let fy = -2.4;
    let ww = 4.6;
    let wh = ww * th.tan();
    let wx = -4.9 - 0.3 * tv;
    let apex = vec2(wx, fy + wh);
    let dir = vec2(th.cos(), -th.sin());
    let n = vec2(th.sin(), th.cos());
    let g = group([
        ground(-6.9, 1.3, fy),
        polygon([vec2(wx, fy), vec2(wx + ww, fy), apex]).fill(PINK, 0.12).stroke(PINK, 3.0),
        tex(`A`, 0.45, PINK).move_to(vec2(wx + 1.0, fy + 0.45)),
    ]);
    let s = 0.3 + 3.0 * (tv / 8.0) * (tv / 8.0);
    let p = apex + dir * s;
    g = g.add(slope_block(p, dir, n, 0.9, 0.6, -th, BLUE, `B`));
    let c = p + dir * 0.45 + n * 0.3;
    g = g.add(varrow(c + n * 0.35, vec2(-1.2, 0.0), PINK, `a_1`));
    g = g.add(varrow(c, dir * 1.4, GREEN, `a_2`));
    if tv > 2.0 {
        let a2 = dir * 1.4;
        g = g.add(dashed_line(c, c + vec2(a2.x, 0.0)).color(TEAL));
        g = g.add(dashed_line(c + vec2(a2.x, 0.0), c + a2).color(TEAL));
        g = g.add(tex(`a_2\cos\theta`, 0.34, TEAL).move_to(c + vec2(a2.x / 2.0, 0.28)));
        g = g.add(tex(`a_2\sin\theta`, 0.34, TEAL).move_to(c + vec2(a2.x + 0.75, a2.y / 2.0)));
    }
    g
});
voiceover("Now consider another context: a triangular wedge block A on a horizontal plane, over which there is another block B. If A moves toward the left with acceleration a1, and B moves on the surface of A with acceleration a2, then B is also moving with acceleration a1 toward the left.", |info| {
    play_bg(tw.to(2.0).rate("linear"), info.duration);
    let b = board(info, [
        [0.0, "title", "Wedge and block"],
        [0.25, "text", "Wedge A accelerates to the left:"],
        [0.32, "math", `\vec{a}_A = -a_1\,\hat{i}`],
        [0.5, "text", "B slides along A's face (relative acceleration):"],
        [0.58, "math", `\vec{a}_{B/A} = a_2 \text{ along the incline}`],
        [0.78, "key", `\vec{a}_B = \vec{a}_A + \vec{a}_{B/A}`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("If we carefully analyze the net acceleration of block B: its relative acceleration a2 makes an angle theta with the horizontal. Its horizontal component is a2 cosine theta, and its vertical component is a2 sine theta. Net horizontal acceleration is a2 cosine theta minus a1. Net acceleration magnitude is the square root of a x squared plus a y squared.", |info| {
    play_bg(tw.to(8.0).rate("linear"), info.duration);
    let b = board(info, [
        [0.0, "title", "Components of B's acceleration"],
        [0.15, "math", `\vec{a}_{B/A} = a_2\cos\theta\,\hat{i} - a_2\sin\theta\,\hat{j}`],
        [0.42, "text", "Horizontal: relative part minus the wedge's"],
        [0.5, "math", `a_{Bx} = a_2\cos\theta - a_1`],
        [0.62, "math", `a_{By} = -a_2\sin\theta`],
        [0.82, "key", `a_{\text{net}} = \sqrt{a_{Bx}^2 + a_{By}^2}`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
wait(0.3);

// ── Two blocks joined by a string (intro_8..10) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 1", "Constraint relations");
let ts = tracker(0.0);
always_redraw(|| {
    let tv = value(ts);
    let fy = -0.6;
    let xb0 = -2.4;
    let xb = xb0 + 0.25 * tv + 0.02 * tv * tv;
    let xa = xb - 2.9;
    let xa0 = xb0 - 2.9;
    let g = group([
        ground(-6.9, 1.4, fy),
        line(vec2(xa + 0.55, fy + 0.35), vec2(xb - 0.55, fy + 0.35)).stroke(BLUE, 3.5),
        block(vec2(xa, fy + 0.35), 1.1, 0.7, PURPLE, `A`),
        block(vec2(xb, fy + 0.35), 1.1, 0.7, PURPLE, `B`),
        varrow(vec2(xb + 0.55, fy + 0.35), vec2(1.0, 0.0), GREEN, `F`),
    ]);
    if xb - xb0 > 0.08 {
        g = g.add(arrow(vec2(xa0, fy - 0.7), vec2(xa, fy - 0.7)).color(TEAL));
        g = g.add(tex(`y`, 0.38, TEAL).move_to(vec2((xa0 + xa) / 2.0, fy - 1.0)));
        g = g.add(arrow(vec2(xb0, fy - 1.4), vec2(xb, fy - 1.4)).color(GOLD));
        g = g.add(tex(`x`, 0.38, GOLD).move_to(vec2((xb0 + xb) / 2.0, fy - 1.7)));
    }
    g = g.add(text("Inextensible string:  y = x", 0.25, GRAY_A).move_to(vec2(-2.75, 1.3)));
    g
});
voiceover("Now let's talk about constrained relations. These are mathematical relations by which the motion of two or more bodies are related. If A moves with velocity v A and B moves with velocity v B, and B moves by distance x and A by distance y, we can state y equals x since the string is inextensible.", |info| {
    play_bg(ts.to(3.0).rate("linear"), info.duration);
    let b = board(info, [
        [0.0, "title", "Two blocks, one string"],
        [0.25, "text", "A and B on the ground, linked by a taut string."],
        [0.5, "text", "The string length cannot change:"],
        [0.58, "math", `x_B - x_A = L \quad (\text{const})`],
        [0.8, "math", `\Delta x_B - \Delta x_A = 0 \;\Rightarrow\; x = y`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("If we differentiate the relation with respect to time, the time derivative of y equals the time derivative of x, meaning v A equals v B. Differentiating again gives acceleration of A equals acceleration of B.", |info| {
    play_bg(ts.to(8.0).rate("linear"), info.duration);
    let b = board(info, [
        [0.0, "title", "Differentiate with time"],
        [0.2, "math", `\frac{dy}{dt} = \frac{dx}{dt} \;\Rightarrow\; v_A = v_B`],
        [0.6, "math", `\frac{dv_A}{dt} = \frac{dv_B}{dt} \;\Rightarrow\; a_A = a_B`],
        [0.85, "key", `\Delta x_A = \Delta x_B \Rightarrow v_A = v_B \Rightarrow a_A = a_B`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("So, keep in mind: to develop constrained relations of velocity and acceleration for any given bodies, first try to relate their displacement and then differentiate with respect to time. This will give you the constrained relation of velocities and acceleration.", |info| {
    let b = board(info, [
        [0.0, "title", "The method"],
        [0.25, "text", "1.  Write the geometric (displacement) relation."],
        [0.45, "text", "2.  Differentiate once: velocity relation."],
        [0.6, "text", "3.  Differentiate again: acceleration relation."],
        [0.8, "key", `\text{relate} \;\to\; \tfrac{d}{dt} \;\to\; \tfrac{d^2}{dt^2}`],
    ]);
    sound("tada", 0.35);
    wait_until(info.end + 0.6);
    clear_board(b);
});

// ═══════════════ PART 2 · SYSTEMS (lecture steps 1..18) ════════════════
next_scene("fade", 0.7);
let p2 = register(bold_text("Part 2", 0.7, YELLOW).shift(UP * 0.5));
let p2b = register(text("Constraints in action: seven classic systems", 0.36, GRAY_A).shift(DOWN * 0.3));
sound("whoosh", 0.4);
play([fade_in(p2, UP), fade_in(p2b, UP)], 0.9);
wait(1.4);

// ── Three-mass Atwood system (steps 1..3) ──
next_scene("fade", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 2", "Three-mass connected pulleys");
let ty3 = tracker(0.0);
always_redraw(|| {
    let d3 = value(ty3);
    let r = 0.32;
    let fl = vec2(-4.6, 2.3);
    let fr = vec2(-2.0, 2.3);
    let mpx = fl.x - r;
    let rm = 0.46;
    let yp = 0.7 + d3;
    let m3top = 0.5 - d3;
    let m1top = yp - 1.25;
    let m2top = yp - 1.85;
    let x1 = mpx - rm;
    let x2 = mpx + rm;
    let x3 = fr.x + r;
    let mp = vec2(mpx, yp);
    let g = group([
        ceiling(-5.6, -1.0, 2.95),
        line(vec2(fl.x, 2.95), fl).stroke(GRAY_C, 3.0),
        line(vec2(fr.x, 2.95), fr).stroke(GRAY_C, 3.0),
        // blue string: movable pulley → over both fixed pulleys → m3
        line(mp, vec2(mpx, fl.y)).stroke(BLUE, 3.0),
        arc(r, PI / 2.0, PI / 2.0).shift(fl).stroke(BLUE, 3.0),
        line(vec2(fl.x, fl.y + r), vec2(fr.x, fr.y + r)).stroke(BLUE, 3.0),
        arc(r, 0.0, PI / 2.0).shift(fr).stroke(BLUE, 3.0),
        line(vec2(x3, fr.y), vec2(x3, m3top)).stroke(BLUE, 3.0),
        // green string: m1 → over the movable pulley → m2
        line(vec2(x1, m1top), vec2(x1, yp)).stroke(GREEN, 3.0),
        arc(rm, 0.0, PI).shift(mp).stroke(GREEN, 3.0),
        line(vec2(x2, yp), vec2(x2, m2top)).stroke(GREEN, 3.0),
        pulley(fl, r, BLUE),
        pulley(fr, r, BLUE),
        pulley(mp, rm, GREEN),
        block(vec2(x1, m1top - 0.3), 0.56, 0.6, GREEN, `m_1`),
        block(vec2(x2, m2top - 0.3), 0.56, 0.6, GREEN, `m_2`),
        block(vec2(x3, m3top - 0.32), 0.62, 0.62, BLUE, `m_3`),
    ]);
    if d3 > 0.05 {
        g = g.add(varrow(vec2(x3 + 0.7, 0.5), vec2(0.0, -d3), BLUE, `\Delta y_3`));
        g = g.add(varrow(vec2(mpx - 1.3, 0.7), vec2(0.0, d3), GREEN, `\Delta y_p`));
    }
    let dy1 = -d3;
    let dy2 = -d3;
    let chk = r2(dy1 + dy2 + 2.0 * d3);
    g = g.add(tex(`\Delta y_1 = ${r2(dy1)},\;\; \Delta y_2 = ${r2(dy2)},\;\; \Delta y_3 = ${r2(-d3)}`, 0.36, GRAY_A).move_to(vec2(-3.0, -2.35)));
    g = g.add(tex(`\Delta y_1 + \Delta y_2 + 2\,\Delta y_3 = ${chk}`, 0.4, YELLOW).move_to(vec2(-3.0, -2.85)));
    g
});
voiceover("Welcome to the constrained motion tutorial. Let's study how constraints link the motions of connected physical systems. We will start with a three mass connected Atwood system.", |info| {
    let b = board(info, [
        [0.0, "title", "Three-mass connected system"],
        [0.3, "text", "y measured downward from the ceiling."],
        [0.55, "text", "Two strings: one over the fixed pulleys,"],
        [0.65, "text", "one over the movable pulley."],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("Observe what happens when mass 3 moves downwards. The pulley P rises, altering the heights of connected masses m1 and m2. We will slide the mass 3 position down.", |info| {
    play_bg(ty3.to(1.2).rate("smooth"), info.duration);
    let b = board(info, [
        [0.0, "title", "Blue string"],
        [0.25, "math", `y_3 + (\text{depth of pulley}) = \text{const}`],
        [0.45, "math", `\Delta y_p = -\Delta y_3`],
        [0.6, "title", "Green string"],
        [0.7, "math", `(y_1 - y_p) + (y_2 - y_p) = \text{const}`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("This geometric constraint links all three displacements. The total sum: delta y1 plus delta y2 plus 2 times delta y3 equals zero, linking their accelerations directly.", |info| {
    let b = board(info, [
        [0.0, "title", "The constraint"],
        [0.15, "math", `L = y_1 + y_2 + 2\,y_3 = \text{const}`],
        [0.35, "math", `v_1 + v_2 + 2\,v_3 = 0`],
        [0.6, "key", `a_1 + a_2 + 2\,a_3 = 0`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
wait(0.3);

// ── Ring sliding on a rod (steps 4, 5) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 2", "Ring sliding on a rod");
let tr = tracker(3.5);
always_redraw(|| {
    let x = value(tr);
    let h = 2.0;
    let pr = 0.28;
    let pc = vec2(-0.9, 0.0);
    let rody = pc.y + h;
    let ring = vec2(pc.x - x, rody);
    let tot = h + 2.5;
    let hang = tot - (x * x + h * h).sqrt();
    let by = pc.y - hang;
    let th = (h / x).atan();
    let g = group([
        line(vec2(-6.6, rody), vec2(-0.3, rody)).stroke(GRAY_B, 6.0),
        rectangle(0.16, 0.5).move_to(vec2(-6.68, rody)).fill(GRAY_C, 1.0).no_stroke(),
        line(ring, pc + vec2(-pr * th.sin(), pr * th.cos())).stroke(GRAY_B, 2.5),
        line(vec2(pc.x + pr, pc.y), vec2(pc.x + pr, by)).stroke(GRAY_B, 2.5),
        pulley(pc, pr, BLUE),
        rounded_rectangle(0.6, 0.3, 0.08).move_to(ring).fill(BLUE, 0.35).stroke(BLUE, 3.0),
        block(vec2(pc.x + pr, by - 0.3), 0.6, 0.6, PURPLE, `m`),
        angle_arc(ring, pc - ring, vec2(1.0, 0.0), 0.7).color(GOLD),
        tex(`\theta = ${(th / DEG).round().to_int()}^\circ`, 0.36, GOLD).move_to(ring + vec2(1.25, -0.42)),
    ]);
    g = g.add(varrow(ring + vec2(0.0, 0.3), vec2(1.0, 0.0), BLUE, `v_r`));
    g = g.add(varrow(vec2(pc.x + pr + 0.5, by - 0.3), vec2(0.0, -th.cos() * 1.0), GREEN, `v_b`));
    g = g.add(tex(`\cos\theta = ${r2(th.cos())} \;\Rightarrow\; v_b = ${r2(th.cos())}\, v_r`, 0.4, YELLOW).move_to(vec2(-3.4, -2.6)));
    g
});
voiceover("Now let's switch to the ring sliding on a rod. Here, the constraint relation is angle dependent, dictated by the geometry of the string connection.", |info| {
    let b = board(info, [
        [0.0, "title", "Geometric string length"],
        [0.3, "math", `L = \sqrt{x^2 + h^2} + y`],
        [0.55, "text", "x: ring to pulley,  h: rod height,  y: hanging part"],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("As we slide the ring closer to the pulley, watch the string angle theta. The block's downward velocity is always equal to the ring's velocity times the cosine of theta.", |info| {
    play_bg(tr.to(1.0).rate("smooth"), info.duration);
    let b = board(info, [
        [0.0, "title", "Differentiate with time"],
        [0.15, "math", `\frac{x}{\sqrt{x^2+h^2}}\,\frac{dx}{dt} + \frac{dy}{dt} = 0`],
        [0.4, "math", `\cos\theta = \frac{x}{\sqrt{x^2 + h^2}}`],
        [0.7, "key", `v_{\text{block}} = v_{\text{ring}}\cos\theta`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
wait(0.3);

// ── Wedge and sliding block (steps 6, 7) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 2", "Wedge and sliding block");
let twd = tracker(0.8);
always_redraw(|| {
    let s = value(twd);
    let th = 30.0 * DEG;
    let fy = -2.4;
    let ww = 4.4;
    let wh = ww * th.tan();
    let wx = -3.9 - 0.4 * (s - 0.8);
    let apex = vec2(wx, fy + wh);
    let dir = vec2(th.cos(), -th.sin());
    let n = vec2(th.sin(), th.cos());
    let p = apex + dir * s;
    let c = p + dir * 0.42 + n * 0.28;
    let g = group([
        ground(-6.9, 1.4, fy),
        polygon([vec2(wx, fy), vec2(wx + ww, fy), apex]).fill(BLUE, 0.12).stroke(BLUE, 3.0),
        tex(`M`, 0.45, BLUE).move_to(vec2(wx + 1.0, fy + 0.4)),
        tex(`\theta`, 0.4, GOLD).move_to(vec2(wx + ww - 0.85, fy + 0.22)),
        slope_block(p, dir, n, 0.84, 0.56, -th, PURPLE, `m`),
        dashed_line(c - n * 1.6, c + n * 1.4).color(GRAY_C),
        tex(`\hat{n}`, 0.36, GRAY_B).move_to(c + n * 1.65),
    ]);
    // a_w = mg sinθ cosθ / (M + m sin²θ) with M = 5, m = 1.5 (the original's defaults)
    let aw = 1.5 * 9.8 * th.sin() * th.cos() / (5.0 + 1.5 * th.sin() * th.sin());
    let arel = 9.8 * th.sin() + aw * th.cos();
    let ab = dir * arel + vec2(-aw, 0.0);
    g = g.add(varrow(vec2(wx + 1.6, fy + 0.45), vec2(-aw * 0.8, 0.0), BLUE, `a_w`));
    g = g.add(varrow(c, ab * 0.22, RED, `a_{\text{block}}`));
    g
});
voiceover("Next, we have the wedge and block constraint. For contact to remain, components of acceleration perpendicular to the contact plane must match.", |info| {
    let b = board(info, [
        [0.0, "title", "Contact (normal) constraint"],
        [0.3, "text", "The block can't leave or sink into the face:"],
        [0.55, "key", `\vec{a}_{\text{block}}\cdot\hat{n} = \vec{a}_{\text{wedge}}\cdot\hat{n}`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("As the block slides down the wedge, it exerts a normal force that pushes the wedge horizontally to the left. Observe how the relative slide s couples with the wedge horizontal displacement.", |info| {
    play_bg(twd.to(3.2).rate("in_quad"), info.duration);
    let b = board(info, [
        [0.0, "title", "Positions"],
        [0.12, "math", `x_b = X_w + s\cos\theta, \quad y_b = H - s\sin\theta`],
        [0.4, "math", `a_{bx} = a_w + a_{\text{rel}}\cos\theta,\quad a_{by} = -a_{\text{rel}}\sin\theta`],
        [0.7, "key", `a_{by} = (a_{bx} - a_w)\tan\theta`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
wait(0.3);

// ── Leaning rod and the ICR (steps 8, 9) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 2", "Leaning rod and the ICR");
let ta = tracker(60.0);
let icr_on = tracker(0.0);
always_redraw(|| {
    let th = value(ta) * DEG;
    let o = vec2(-6.2, -2.4);
    let l = 4.6;
    let top = vec2(o.x, o.y + l * th.sin());
    let bot = vec2(o.x + l * th.cos(), o.y);
    let icr = vec2(bot.x, top.y);
    let g = group([
        wall(o.x, o.y, 2.7),
        ground(o.x, 0.9, o.y),
    ]);
    let on = value(icr_on);
    if on > 0.01 {
        g = g.add(arc(l, 0.0, PI / 2.0).shift(o).stroke(PURPLE, 2.0).opacity(0.35 * on));
        g = g.add(dashed_line(top, icr).color(PURPLE).opacity(on));
        g = g.add(dashed_line(bot, icr).color(PURPLE).opacity(on));
        g = g.add(glow_dot(icr, 0.12, PURPLE).opacity(on));
        g = g.add(text("ICR", 0.26, PURPLE).move_to(icr + vec2(0.45, 0.25)).opacity(on));
    }
    g = g.add(line(top, bot).stroke(GRAY_B, 10.0));
    g = g.add(dot(top, 0.11, BLUE));
    g = g.add(dot(bot, 0.11, BLUE));
    g = g.add(angle_arc(bot, top - bot, vec2(-1.0, 0.0), 0.6).color(GOLD));
    g = g.add(tex(`\theta = ${(th / DEG).round().to_int()}^\circ`, 0.36, GOLD).move_to(bot + vec2(-1.25, 0.32)));
    let vy = 0.9 / th.tan();
    let room = top.y - o.y - 0.1;
    if vy > room { vy = room; }
    if vy > 2.2 { vy = 2.2; }
    g = g.add(varrow(bot + vec2(0.0, -0.32), vec2(0.9, 0.0), GREEN, `v_x`));
    g = g.add(varrow(top + vec2(0.35, 0.0), vec2(0.0, -vy), RED, `v_y`));
    g = g.add(tex(`v_y = v_x\cot\theta = ${r2(1.0 / th.tan())}\,v_x`, 0.4, YELLOW).move_to(vec2(-2.4, 2.4)));
    g
});
voiceover("Let's look at the leaning rod. A rigid rod slides against a vertical wall and a horizontal floor. We will activate the instantaneous center of rotation, or ICR.", |info| {
    let b = board(info, [
        [0.0, "title", "Rigid rod constraint"],
        [0.2, "math", `x^2 + y^2 = L^2`],
        [0.45, "math", `2x\,\dot{x} + 2y\,\dot{y} = 0`],
    ]);
    wait_until(info.start + info.duration * 0.8);
    play(icr_on.to(1.0), 0.8);
    wait_until(info.end);
    clear_board(b);
});
voiceover("As the rod slides down, every point on the rod performs pure rotation about the ICR. Notice how the ICR position coordinates track a circular arc.", |info| {
    play_bg(ta.to(15.0).rate("smooth"), info.duration);
    let b = board(info, [
        [0.0, "title", "Velocity and acceleration"],
        [0.1, "math", `x\,v_x + y\,v_y = 0 \;\Rightarrow\; v_y = -v_x\cot\theta`],
        [0.4, "math", `a_y = -a_x\cot\theta + \frac{v_x^2}{y}\,(1 + \cot^2\theta)`],
        [0.7, "key", `\text{ICR} = (x, y), \quad x^2 + y^2 = L^2`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
wait(0.3);

// ── Rolling on a moving plank (steps 10, 11) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 2", "Rolling on a moving plank");
let t0 = time();
always_redraw(|| {
    let u = time() - t0;
    let fy = -2.4;
    let rr = 0.85;
    let ph = 0.26;
    // Positions are closed-form, so rolling without slipping is exact:
    // the cylinder's turn angle is (x_cm − x_p) / R.
    let xp = -3.6 + 0.9 * (0.8 * u).sin();
    let vp = 0.72 * (0.8 * u).cos();
    let rel = 1.15 * (0.55 * u).sin();
    let vrel = 0.6325 * (0.55 * u).cos();
    let xc = xp + rel;
    let vc = vp + vrel;
    let w = vrel / rr;
    let phi = -rel / rr;
    let cc = vec2(xc, fy + ph + rr);
    let g = group([
        ground(-6.9, 1.4, fy),
        circle_at(vec2(xp - 1.6, fy + 0.09), 0.09).fill(GRAY_C, 1.0).no_stroke(),
        circle_at(vec2(xp + 1.6, fy + 0.09), 0.09).fill(GRAY_C, 1.0).no_stroke(),
        rounded_rectangle(5.0, ph, 0.06).move_to(vec2(xp, fy + 0.18 + ph / 2.0 - 0.05)).fill(GOLD, 0.85).stroke(ORANGE, 2.0),
        circle_at(cc, rr).fill(BLUE, 0.15).stroke(BLUE, 4.0),
    ]);
    let k = 0;
    while k < 4 {
        let a = phi + k * PI / 2.0;
        g = g.add(line(cc, cc + vec2(a.cos(), a.sin()) * rr).stroke(BLUE, 2.0));
        k += 1;
    }
    g = g.add(dot(cc, 0.07, BLUE));
    g = g.add(varrow(cc, vec2(vc * 1.4, 0.0), BLUE, `v_{cm}`));
    g = g.add(varrow(vec2(xp, fy - 0.45), vec2(vp * 1.4, 0.0), ORANGE, `v_p`));
    g = g.add(varrow(cc + vec2(0.0, -rr), vec2(-w * rr * 1.4, 0.0), RED, `\omega R`));
    g = g.add(tex(`v_{cm} = ${r2(vc)},\quad \omega R = ${r2(w * rr)},\quad v_p = ${r2(vp)}`, 0.38, GRAY_A).move_to(vec2(-2.9, 2.35)));
    g = g.add(tex(`v_{cm} - \omega R = ${r2(vc - w * rr)} = v_p`, 0.42, YELLOW).move_to(vec2(-2.9, 1.8)));
    g
});
voiceover("Finally, we have the rolling on a moving plank. Here, rotational and translational constraints couple at the point of contact.", |info| {
    let b = board(info, [
        [0.0, "title", "Velocities (ground frame)"],
        [0.25, "text", "v_cm: centre,  v_p: plank,  ω: angular velocity"],
        [0.55, "math", `v_{\text{contact}} = v_{cm} - \omega R`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("Observe how changing the plank speed changes the cylinder's angular velocity. The velocity of the contact point relative to ground must equal the plank velocity. That is, v c m minus omega R equals v p.", |info| {
    let b = board(info, [
        [0.0, "title", "No slipping"],
        [0.3, "math", `v_{\text{contact}} = v_p`],
        [0.6, "key", `\omega = \frac{v_{cm} - v_p}{R}`],
    ]);
    wait_until(info.end + 1.0);
    clear_board(b);
});
wait(0.3);

// ── Pulley-wedge system (steps 12..14) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 2", "Pulley–wedge system");
let tpw = tracker(0.8);
let show_acc = tracker(0.0);
always_redraw(|| {
    let s = value(tpw);
    let th = 30.0 * DEG;
    let fy = -2.4;
    let ww = 4.0;
    let wh = ww * th.tan();
    let wx = -2.2 - (s - 0.8);
    let apex = vec2(wx, fy + wh);
    let dir = vec2(th.cos(), -th.sin());
    let n = vec2(th.sin(), th.cos());
    let pr = 0.17;
    let p = apex + dir * s;
    let g = group([
        ground(-6.9, 1.4, fy),
        wall(-6.5, fy, fy + 2.9),
        polygon([vec2(wx, fy), apex, vec2(wx + ww, fy)]).fill(PINK, 0.12).stroke(PINK, 3.0),
        tex(`M`, 0.45, PINK).move_to(vec2(wx + 0.75, fy + 0.45)),
        tex(`\theta`, 0.4, GOLD).move_to(vec2(wx + ww - 0.85, fy + 0.22)),
        line(vec2(-6.5, apex.y + pr), vec2(apex.x, apex.y + pr)).stroke(ORANGE, 2.5),
        line(apex + n * pr, p + n * 0.28).stroke(ORANGE, 2.5),
        pulley(apex, pr, GRAY_B),
        slope_block(p, dir, n, 0.8, 0.54, -th, PURPLE, `m`),
    ]);
    let on = value(show_acc);
    if on > 0.01 {
        // a = mg sinθ / (M + 2m(1 − cosθ)), M = 5, m = 1.5
        let a = 1.5 * 9.8 * th.sin() / (5.0 + 3.0 * (1.0 - th.cos()));
        let c = p + dir * 0.4 + n * 0.27;
        g = g.add(varrow(vec2(wx + 1.4, fy + 0.45), vec2(-a * 0.6, 0.0), PINK, `a_w`).opacity(on));
        g = g.add(varrow(c + n * 0.35, dir * a * 0.6, GREEN, `a_{\text{rel}}`).opacity(on));
        g = g.add(varrow(c, (dir * a + vec2(-a, 0.0)) * 0.6, RED, `a_{\text{block}}`).opacity(on));
    }
    g = g.add(tex(`\Delta x_{\text{wedge}} = ${r2(s - 0.8)}, \quad \Delta s_{\text{block}} = ${r2(s - 0.8)}`, 0.38, YELLOW).move_to(vec2(-3.4, 2.5)));
    g
});
voiceover("Let's move to our first example: the pulley wedge system. Constrained motion can feel like a tangled mess of equations at first, but mastering it is a game changer for JEE mechanics. The secret lies in breaking the problem down into strict geometric relationships.", |info| {
    let b = board(info, [
        [0.0, "title", "The pulley–wedge system"],
        [0.25, "text", "Find the wedge's acceleration and the block's"],
        [0.3, "text", "acceleration relative to the wedge."],
        [0.6, "text", "Equation of constraint:"],
        [0.68, "key", `x_{\text{wedge}} + s_{\text{block}} = L`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("This classic question tests both string constraints and wedge constraints simultaneously. When the wedge moves left by x, the conservation of string length forces the block to slide down the incline by the exact same distance x relative to the wedge.", |info| {
    play_bg(tpw.to(3.2).rate("smooth"), info.duration);
    let b = board(info, [
        [0.0, "title", "Kinematic constraint"],
        [0.3, "math", `\Delta x_{\text{wedge}} = -x`],
        [0.55, "text", "The incline part of the string grows by the same x:"],
        [0.62, "math", `s = x`],
        [0.8, "key", `a_{\text{rel}} = a_{\text{wedge}}`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("By using the wedge as a non inertial reference frame, we apply a horizontal pseudo force. The wedge horizontal acceleration, and the block relative acceleration, are both equal to m g sine theta divided by M plus 2 m times 1 minus cosine theta.", |info| {
    play(show_acc.to(1.0), 0.6);
    let b = board(info, [
        [0.0, "title", "Dynamics"],
        [0.1, "text", "In the wedge frame add the pseudo-force m a_w:"],
        [0.25, "math", `mg\sin\theta + m a_w\cos\theta - T = m a_{\text{rel}}`],
        [0.45, "math", `T(1 - \cos\theta) + N\sin\theta = M a_w`],
        [0.7, "key", `a_w = a_{\text{rel}} = \frac{m g\sin\theta}{M + 2m(1 - \cos\theta)}`],
    ]);
    wait_until(info.end + 0.8);
    clear_board(b);
});
wait(0.3);

// ── Bar and wedge contact (steps 15..18) ──
next_scene("slide:from-right", 0.6);
scene_chrome("CONSTRAINED MOTION · PART 2", "Bar and wedge contact");
let tbx = tracker(0.0);
let forces = tracker(0.0);
always_redraw(|| {
    let x = value(tbx);
    let th = 30.0 * DEG;
    let fy = -2.4;
    let ww = 5.0;
    let wh = ww * th.tan();
    let bx = -3.4;
    let d = 3.6;
    let wxs = bx - d + x;
    let wxe = wxs + ww;
    let cy = fy + (d - x) * th.tan();
    let n = vec2(-th.sin(), th.cos());
    let wr = 0.15;
    let wc = vec2(bx, cy) + n * wr;
    let g = group([
        ground(-6.9, 1.4, fy),
        polygon([vec2(wxs, fy), vec2(wxe, fy), vec2(wxe, fy + wh)]).fill(PINK, 0.18).stroke(PINK, 3.0),
        tex(`M`, 0.45, PINK).move_to(vec2(wxe - 0.7, fy + 0.5)),
        tex(`\theta`, 0.4, GOLD).move_to(vec2(wxs + 0.95, fy + 0.22)),
        line(vec2(bx - 0.32, 0.6), vec2(bx - 0.32, 2.8)).stroke(GRAY_C, 3.0),
        line(vec2(bx + 0.32, 0.6), vec2(bx + 0.32, 2.8)).stroke(GRAY_C, 3.0),
        rectangle(0.3, 2.85 - wc.y).move_to(vec2(bx, (2.85 + wc.y) / 2.0)).fill(TEAL, 0.5).stroke(TEAL, 2.5),
        circle_at(wc, wr).fill(GRAY_C, 1.0).stroke(GRAY_B, 2.0),
    ]);
    for ry in [1.0, 1.7, 2.4] {
        g = g.add(dot(vec2(bx - 0.25, ry), 0.07, GRAY_B));
        g = g.add(dot(vec2(bx + 0.25, ry), 0.07, GRAY_B));
    }
    let on = value(forces);
    if on > 0.01 {
        let cp = vec2(bx, cy);
        g = g.add(varrow(cp, n * 1.0, YELLOW, `N`).opacity(on));
        g = g.add(varrow(cp, n * -0.8, ORANGE, `N`).opacity(on));
        g = g.add(varrow(vec2(bx, 1.2), vec2(0.0, -0.9), RED, `mg`).opacity(on));
        g = g.add(varrow(vec2(wxe - 0.7, fy + 1.0), vec2(0.0, -0.8), RED, `Mg`).opacity(on));
    }
    g = g.add(tex(`y = (d - x)\tan\theta = ${r2((d - x) * th.tan())}`, 0.36, YELLOW).move_to(vec2(-0.7, 1.55)));
    g
});
voiceover("Let's examine a classic JEE main constraint problem: the bar and wedge contact system. A vertical bar is constrained by guide rollers to move only vertically, resting on an inclined wedge that slides horizontally. This forms a simple, visually intuitive linear constraint.", |info| {
    let b = board(info, [
        [0.0, "title", "Bar and wedge"],
        [0.3, "text", "The bar moves only vertically (guide rollers)."],
        [0.45, "text", "The wedge moves only horizontally."],
        [0.7, "text", "Contact constraint:"],
        [0.76, "math", `y = (d - x)\tan\theta`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("The vertical height of the bar is linked directly to the horizontal displacement of the wedge by the incline angle theta. Taking the time derivative, we find that the bar's vertical velocity equals the wedge's horizontal velocity times tangent theta.", |info| {
    play_bg(tbx.to(2.0).rate("smooth"), info.duration);
    let b = board(info, [
        [0.0, "title", "Contact constraint"],
        [0.4, "math", `\frac{dy}{dt} = -\frac{dx}{dt}\,\tan\theta`],
        [0.7, "key", `v_y = -v_x\tan\theta`],
    ]);
    wait_until(info.end);
    clear_board(b);
});
voiceover("By applying Newton's second law to both the bar and the wedge, we can solve for the normal force between them. The resulting horizontal acceleration of the wedge is equal to m g sine theta cosine theta divided by M cosine squared theta plus m sine squared theta.", |info| {
    play(forces.to(1.0), 0.6);
    play_bg(tbx.to(3.4).rate("in_quad"), info.duration);
    let b = board(info, [
        [0.0, "title", "Dynamics"],
        [0.15, "math", `N\sin\theta = M a_x, \qquad mg - N\cos\theta = m a_y`],
        [0.5, "math", `a_x = \frac{m g\sin\theta\cos\theta}{M\cos^2\theta + m\sin^2\theta}`],
        [0.75, "key", `N = \frac{m M g\cos\theta}{M\cos^2\theta + m\sin^2\theta}`],
    ]);
    wait_until(info.end + 0.6);
    clear_board(b);
});
voiceover("Thank you for attending this tutorial on constrained motion. We have covered the fundamental string, ring, wedge, plank, and curved contact constraints. You can now experiment with the simulation controls manually to build your intuition.", |info| {
    let b = board(info, [
        [0.0, "title", "Summary"],
        [0.25, "text", "1. Three-mass pulley system"],
        [0.3, "text", "2. Ring sliding on a rod"],
        [0.35, "text", "3. Wedge and sliding block"],
        [0.4, "text", "4. Leaning rod and ICR"],
        [0.45, "text", "5. Rolling on a moving plank"],
        [0.5, "text", "6. Pulley–wedge system"],
        [0.55, "text", "7. Bar and wedge contact"],
        [0.75, "key", `\text{relate} \to \tfrac{d}{dt} \to \tfrac{d^2}{dt^2}`],
    ]);
    sound("sparkle", 0.3);
    wait_until(info.end + 1.5);
});
