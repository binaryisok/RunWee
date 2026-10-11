//! Physics diagrams: mechanics (pendulum, spring–mass, incline, free-body diagram,
//! pulley, projectile path, vector components, energy bars), waves (travelling,
//! standing, superposition), optics (thin lenses, mirrors, ray diagrams, Snell
//! refraction, prism dispersion), circuits (resistor, capacitor, inductor, battery,
//! bulb, switch, meters, ground, wires, current-flow dots) and fields (point
//! charges, electric field lines, equipotentials, wire magnetic fields).

use crate::color::*;
use crate::math::*;
use crate::mobject::*;
use crate::path::*;
use crate::shapes::*;
use crate::style::*;
use crate::tex::tex_sized;
use crate::text::text_sized;

// ------------------------------------------------------------- mechanics

/// Ceiling/wall hatching between `a` and `b` (hatch on the side of `normal`).
pub fn hatched_surface(a: Vec2, b: Vec2, normal: Vec2) -> Mobject {
    let d = b - a;
    let n = (d.length() / 0.25).ceil() as usize;
    let mut g = Mobject::group([line(a, b).stroke(GRAY_B, 4.0)]);
    let nn = normal.normalize();
    let along = d.normalize();
    for i in 0..=n {
        let p = a + d * (i as f64 / n.max(1) as f64);
        g.push(line(p, p + nn * 0.18 - along * 0.18).stroke(GRAY_C, 2.0));
    }
    g
}

/// Pendulum at angle `theta` (from vertical) hanging from `pivot`.
pub fn pendulum_mobject(
    pivot: Vec2,
    length: f64,
    theta: f64,
    bob_radius: f64,
    color: Color,
) -> Mobject {
    let bob = pivot + v2(length * theta.sin(), -length * theta.cos());
    Mobject::group([
        hatched_surface(pivot + LEFT * 0.6, pivot + RIGHT * 0.6, UP),
        line(pivot, bob).stroke(GRAY_A, 3.0).with_tag("rod"),
        dot_r(pivot, 0.05, WHITE),
        circle_at(bob, bob_radius)
            .no_stroke()
            .sheen(color, UL)
            .with_tag("bob"),
    ])
}

/// Horizontal spring–mass: wall at `wall_x`, block of width `block` centred at `x`.
pub fn spring_block(wall_x: f64, y: f64, x: f64, block: f64, color: Color) -> Mobject {
    let wall = hatched_surface(v2(wall_x, y - 0.7), v2(wall_x, y + 0.7), LEFT);
    let left_face = x - block / 2.0;
    let spring = coil(v2(wall_x, y), v2(left_face, y), 9.0, 0.22, 0.25).stroke(GRAY_A, 3.0);
    let blk = square(block)
        .move_to(v2(x, y))
        .fill(color, 0.9)
        .stroke(color.lighter(0.3), 3.0);
    let floor = line(
        v2(wall_x, y - block / 2.0),
        v2(wall_x + 9.0, y - block / 2.0),
    )
    .stroke(GRAY_B, 3.0);
    Mobject::group([
        wall,
        floor,
        spring.with_tag("spring"),
        blk.with_tag("block"),
    ])
}

/// Vertical spring hanging from a ceiling with a mass at `y_mass`.
pub fn hanging_spring(top: Vec2, y_mass: f64, mass_size: f64, color: Color) -> Mobject {
    let bottom = v2(top.x, y_mass + mass_size / 2.0);
    Mobject::group([
        hatched_surface(top + LEFT * 0.8, top + RIGHT * 0.8, UP),
        coil(top, bottom, 8.0, 0.22, 0.2).stroke(GRAY_A, 3.0),
        square(mass_size)
            .move_to(v2(top.x, y_mass))
            .fill(color, 0.9)
            .stroke(color.lighter(0.3), 3.0),
    ])
}

/// Inclined plane (right triangle) with angle `angle` and base `length`, bottom-left at `origin`.
/// Returns (mobject, function placing a block at distance `s` up the slope).
pub fn incline(origin: Vec2, length: f64, angle: f64, color: Color) -> Mobject {
    let h = length * angle.tan();
    let tri = polygon(&[origin, origin + v2(length, 0.0), origin + v2(length, h)])
        .fill(color, 0.35)
        .stroke(color, 3.0);
    let ang = angle_arc(
        origin + v2(length, 0.0),
        LEFT,
        v2(-1.0, -angle.tan()).normalize() * -1.0,
        0.6,
    );
    let _ = ang;
    let arc_mark = arc_centered(origin, 0.7, 0.0, angle).stroke(YELLOW, 3.0);
    let label = tex_sized("\\theta", 0.45, YELLOW).move_to(origin + Vec2::polar(1.0, angle / 2.0));
    Mobject::group([tri, arc_mark, label])
}

/// Point on the slope of an [`incline`] at distance `s` from the bottom, raised by `lift`.
pub fn incline_point(origin: Vec2, angle: f64, s: f64, lift: f64) -> Vec2 {
    origin + Vec2::from_angle(angle) * s + Vec2::from_angle(angle).perp() * lift
}

/// Block resting on an incline at slope distance `s`.
pub fn block_on_incline(origin: Vec2, angle: f64, s: f64, size: f64, color: Color) -> Mobject {
    square(size)
        .rotate(angle)
        .move_to(incline_point(origin, angle, s, size / 2.0))
        .fill(color, 0.9)
        .stroke(color.lighter(0.3), 3.0)
}

/// A force arrow with a TeX label.
pub fn force_arrow(from: Vec2, vec: Vec2, label: &str, color: Color) -> Mobject {
    let a = arrow_styled(
        from,
        from + vec,
        ArrowStyle::default().color(color).thickness(6.0),
    );
    let dir = vec.normalize();
    let l = tex_sized(label, 0.42, color).next_to_point(from + vec, dir, 0.12);
    Mobject::group([a, l])
}

/// Free-body diagram: a body (square) with labelled force arrows from its centre.
pub fn free_body_diagram(center: Vec2, size: f64, forces: &[(Vec2, &str, Color)]) -> Mobject {
    let mut g = Mobject::group([
        square(size)
            .move_to(center)
            .fill(GRAY_D, 1.0)
            .stroke(GRAY_A, 3.0),
        dot_r(center, 0.06, WHITE),
    ]);
    for (v, l, c) in forces {
        g.push(force_arrow(center, *v, l, *c));
    }
    g
}

/// Pulley wheel with a rope hanging two masses at depths `h1`, `h2` below the axle.
pub fn atwood_machine(axle: Vec2, r: f64, h1: f64, h2: f64, c1: Color, c2: Color) -> Mobject {
    let wheel = Mobject::group([
        circle_at(axle, r).fill(GRAY_D, 1.0).stroke(GRAY_A, 4.0),
        dot_r(axle, 0.06, WHITE),
        line(axle, axle + UP * (r + 0.6)).stroke(GRAY_B, 4.0),
        hatched_surface(
            axle + UP * (r + 0.6) + LEFT * 0.8,
            axle + UP * (r + 0.6) + RIGHT * 0.8,
            UP,
        ),
    ]);
    let l = axle + LEFT * r;
    let rr = axle + RIGHT * r;
    let m1 = square(0.6)
        .move_to(l + DOWN * h1)
        .fill(c1, 0.9)
        .stroke(c1.lighter(0.3), 3.0);
    let m2 = square(0.6)
        .move_to(rr + DOWN * h2)
        .fill(c2, 0.9)
        .stroke(c2.lighter(0.3), 3.0);
    let rope = Mobject::group([
        line(l, l + DOWN * (h1 - 0.3)),
        line(rr, rr + DOWN * (h2 - 0.3)),
        arc_centered(axle, r, 0.0, PI),
    ])
    .stroke(GOLD_E, 3.0);
    Mobject::group([rope, wheel, m1.with_tag("m1"), m2.with_tag("m2")])
}

/// A pulley wheel with axle and rim (no mount).
pub fn pulley(center: Vec2, r: f64, color: Color) -> Mobject {
    Mobject::group([
        circle_at(center, r)
            .fill(color, 0.85)
            .stroke(color.lighter(0.35), 3.0),
        circle_at(center, r * 0.55).stroke(color.lighter(0.2), 2.0),
        dot_r(center, (r * 0.18).max(0.04), WHITE),
    ])
}

/// A pulley hung from a ceiling at `ceiling_y` by a short rod.
pub fn fixed_pulley(center: Vec2, r: f64, ceiling_y: f64, color: Color) -> Mobject {
    let top = v2(center.x, ceiling_y);
    Mobject::group([
        hatched_surface(top + LEFT * (r + 0.5), top + RIGHT * (r + 0.5), UP),
        line(top, center).stroke(GRAY_B, 4.0),
        pulley(center, r, color),
    ])
}

/// Tangent points from `p` to the circle `(c, r)`; `None` when `p` is inside.
fn tangent_angles(p: Vec2, c: Vec2, r: f64) -> Option<(f64, f64)> {
    let d = p - c;
    let len = d.length();
    if len <= r {
        return None;
    }
    let base = d.y.atan2(d.x);
    let a = (r / len).acos();
    Some((base + a, base - a))
}

/// A rope from `a`, wrapped around the pulley `(c, r)`, to `b`. `over` picks the
/// upper wrap (string over the pulley) or the lower one (pulley riding on the string).
pub fn rope_over_pulley(a: Vec2, c: Vec2, r: f64, b: Vec2, over: bool, color: Color) -> Mobject {
    let (Some(ta), Some(tb)) = (tangent_angles(a, c, r), tangent_angles(b, c, r)) else {
        return polyline(&[a, b]).stroke(color, 3.0);
    };
    let side = if over { 1.0 } else { -1.0 };
    let pick = |(t1, t2): (f64, f64)| {
        if t1.sin() * side >= t2.sin() * side {
            t1
        } else {
            t2
        }
    };
    let (ta, tb) = (pick(ta), pick(tb));
    let ccw = (tb - ta).rem_euclid(TAU);
    let cw = ccw - TAU;
    let sweep = if (ta + ccw / 2.0).sin() * side >= (ta + cw / 2.0).sin() * side {
        ccw
    } else {
        cw
    };
    let pa = c + Vec2::from_angle(ta) * r;
    let pb = c + Vec2::from_angle(tb) * r;
    Mobject::group([line(a, pa), arc_centered(c, r, ta, sweep), line(pb, b)]).stroke(color, 3.0)
}

/// Dashed parabolic path of an ideal projectile, plus ground line.
pub fn projectile_path(p0: Vec2, speed: f64, angle: f64, g: f64, color: Color) -> Mobject {
    let (tof, _, _) = crate::sim::projectile_stats(speed, angle, g);
    let pts: Vec<Vec2> = linspace(0.0, tof, 80)
        .into_iter()
        .map(|t| crate::sim::projectile(p0, speed, angle, g, t))
        .collect();
    smooth_curve(&pts, false)
        .stroke(color, 3.0)
        .dashed(0.15, 0.1)
}

/// Vector with dashed x/y components and labels.
pub fn vector_components(
    origin: Vec2,
    v: Vec2,
    color: Color,
    labels: (&str, &str, &str),
) -> Mobject {
    let tip = origin + v;
    Mobject::group([
        arrow(origin, tip).color(color),
        arrow(origin, origin + v2(v.x, 0.0))
            .color(RED_B)
            .with_tag("vx"),
        arrow(origin, origin + v2(0.0, v.y))
            .color(GREEN_B)
            .with_tag("vy"),
        dashed_line(origin + v2(v.x, 0.0), tip).stroke(GRAY_B, 2.0),
        dashed_line(origin + v2(0.0, v.y), tip).stroke(GRAY_B, 2.0),
        tex_sized(labels.0, 0.4, color).next_to_point(
            origin + v * 0.55,
            v.perp().normalize(),
            0.15,
        ),
        tex_sized(labels.1, 0.4, RED_B).next_to_point(origin + v2(v.x / 2.0, 0.0), DOWN, 0.15),
        tex_sized(labels.2, 0.4, GREEN_B).next_to_point(origin + v2(0.0, v.y / 2.0), LEFT, 0.15),
    ])
}

/// Energy bar chart (KE, PE, total) with values normalised to `max`.
pub fn energy_bars(ke: f64, pe: f64, max: f64, height: f64) -> Mobject {
    let vals = [(ke, "KE", RED), (pe, "PE", BLUE), (ke + pe, "E", GREEN)];
    let mut g = Mobject::empty();
    for (i, (v, l, c)) in vals.iter().enumerate() {
        let h = (v / max.max(1e-9) * height).max(0.01);
        let x = i as f64 * 0.8;
        g.push(
            rectangle(0.5, h)
                .move_to(v2(x, h / 2.0))
                .fill(*c, 0.9)
                .no_stroke(),
        );
        g.push(tex_sized(&format!("\\mathrm{{{l}}}"), 0.35, *c).move_to(v2(x, -0.3)));
    }
    g.push(line(v2(-0.4, 0.0), v2(2.0, 0.0)).stroke(GRAY_B, 2.0));
    g
}

// ------------------------------------------------------------------ waves

/// Travelling sine wave y = A sin(kx − ωt + φ) drawn over `[x0, x1]` at time `t`.
pub fn travelling_wave(
    x0: f64,
    x1: f64,
    amplitude: f64,
    wavelength: f64,
    freq: f64,
    t: f64,
    color: Color,
) -> Mobject {
    let k = TAU / wavelength;
    let w = TAU * freq;
    let pts: Vec<Vec2> = linspace(x0, x1, 200)
        .into_iter()
        .map(|x| v2(x, amplitude * (k * x - w * t).sin()))
        .collect();
    smooth_curve(&pts, false).stroke(color, 4.0)
}

/// Standing wave on a string of length L (fixed ends) in harmonic `n`.
pub fn standing_wave(
    x0: f64,
    length: f64,
    amplitude: f64,
    n: u32,
    freq: f64,
    t: f64,
    color: Color,
) -> Mobject {
    let k = PI * n as f64 / length;
    let pts: Vec<Vec2> = linspace(0.0, length, 200)
        .into_iter()
        .map(|x| v2(x0 + x, amplitude * (k * x).sin() * (TAU * freq * t).cos()))
        .collect();
    Mobject::group([
        smooth_curve(&pts, false).stroke(color, 4.0),
        dot_r(v2(x0, 0.0), 0.08, WHITE),
        dot_r(v2(x0 + length, 0.0), 0.08, WHITE),
    ])
}

/// Any wave `y = f(x, t)` sampled on `[x0, x1]`.
pub fn wave_fn(f: impl Fn(f64, f64) -> f64, x0: f64, x1: f64, t: f64, color: Color) -> Mobject {
    let pts: Vec<Vec2> = linspace(x0, x1, 300)
        .into_iter()
        .map(|x| v2(x, f(x, t)))
        .collect();
    smooth_curve(&pts, false).stroke(color, 4.0)
}

/// Circular wavefronts from a point source (ripple tank / Doppler style).
pub fn wavefronts(
    source: Vec2,
    wavelength: f64,
    t: f64,
    speed: f64,
    max_r: f64,
    color: Color,
) -> Mobject {
    let phase = (speed * t) % wavelength;
    let mut g = Mobject::empty();
    let mut r = phase;
    while r < max_r {
        let fade = (1.0 - r / max_r) as f32;
        g.push(
            circle_at(source, r.max(0.01))
                .stroke(color, 2.5)
                .stroke_opacity(fade),
        );
        r += wavelength;
    }
    g
}

// ----------------------------------------------------------------- optics

/// Thin lens symbol (converging if f>0: double arrow outward, else inward).
pub fn lens(center: Vec2, height: f64, converging: bool) -> Mobject {
    let h = height / 2.0;
    let bulge = if converging { 0.25 } else { -0.15 };
    let mut p = Path::new();
    p.move_to(center + v2(0.0, h));
    p.quad_to(center + v2(bulge * 2.0 + 0.05, 0.0), center + v2(0.0, -h));
    p.quad_to(center + v2(-bulge * 2.0 - 0.05, 0.0), center + v2(0.0, h));
    p.close();
    Mobject::from_path(p).fill(BLUE_B, 0.25).stroke(BLUE_B, 3.0)
}

/// Principal-ray diagram for a thin lens. Object of height `h_o` at distance `d_o`
/// left of a lens with focal length `f` (negative = diverging), centred at `center`.
pub fn lens_ray_diagram(center: Vec2, f: f64, d_o: f64, h_o: f64) -> Mobject {
    let axis = line(center + LEFT * 7.0, center + RIGHT * 7.0).stroke(GRAY_C, 2.0);
    let l = lens(center, 4.0, f > 0.0);
    let obj_base = center + LEFT * d_o;
    let obj_tip = obj_base + UP * h_o;
    let object = arrow(obj_base, obj_tip).color(GREEN);
    let mut g = Mobject::group([axis, l, object]);
    for s in [-1.0, 1.0] {
        g.push(dot_r(center + RIGHT * (s * f.abs()), 0.07, YELLOW));
        g.push(tex_sized("F", 0.36, YELLOW).next_to_point(
            center + RIGHT * (s * f.abs()),
            DOWN,
            0.12,
        ));
    }
    // image via thin lens equation
    let d_i = if (d_o - f).abs() < 1e-9 {
        f64::INFINITY
    } else {
        1.0 / (1.0 / f - 1.0 / d_o)
    };
    let m = -d_i / d_o;
    let img_tip = center + v2(d_i, h_o * m);
    let lens_hit = center + UP * h_o;
    let far = 7.0;
    // ray 1: parallel then through focal point
    let r1_dir = (center + RIGHT * f - lens_hit).normalize();
    g.push(
        Mobject::group([
            line(obj_tip, lens_hit),
            line(lens_hit, lens_hit + r1_dir * far),
        ])
        .stroke(RED, 2.5),
    );
    // ray 2: through the centre undeviated
    let r2_dir = (center - obj_tip).normalize();
    g.push(line(obj_tip, obj_tip + r2_dir * (d_o + far)).stroke(YELLOW, 2.5));
    // ray 3: through near focal point, exits parallel
    let near_f = center + LEFT * f;
    let r3_dir = (near_f - obj_tip).normalize();
    if r3_dir.x.abs() > 1e-6 {
        let hit_y = obj_tip.y + r3_dir.y * ((center.x - obj_tip.x) / r3_dir.x);
        let hit = v2(center.x, hit_y);
        g.push(
            Mobject::group([line(obj_tip, hit), line(hit, hit + RIGHT * far)]).stroke(TEAL, 2.5),
        );
    }
    if d_i.is_finite() {
        let real = d_i > 0.0;
        let img = arrow(v2(img_tip.x, center.y), img_tip).color(if real {
            ORANGE
        } else {
            ORANGE.with_alpha(0.6)
        });
        g.push(if real { img } else { img.dashed(0.1, 0.07) }.with_tag("image"));
        if !real {
            // virtual image: dashed back-extensions of the refracted rays
            g.push(
                Mobject::group([
                    dashed_line(lens_hit, img_tip),
                    dashed_line(obj_tip, img_tip),
                ])
                .stroke(GRAY_B, 2.0),
            );
        }
    }
    g
}

/// Image distance & magnification from the thin lens equation.
pub fn thin_lens(f: f64, d_o: f64) -> (f64, f64) {
    let d_i = 1.0 / (1.0 / f - 1.0 / d_o);
    (d_i, -d_i / d_o)
}

/// Concave/convex mirror arc of radius `r` facing left at `center` (vertex).
pub fn curved_mirror(vertex: Vec2, radius: f64, height: f64, concave: bool) -> Mobject {
    let half = (height / 2.0 / radius).asin();
    let (c, start) = if concave {
        (vertex + LEFT * radius, -half)
    } else {
        (vertex + RIGHT * radius, PI - half)
    };
    arc_centered(c, radius, start, 2.0 * half).stroke(GRAY_A, 5.0)
}

/// Snell's-law refraction diagram at a flat boundary (n1 above, n2 below).
pub fn refraction_diagram(n1: f64, n2: f64, incidence_deg: f64) -> Mobject {
    let th1 = incidence_deg * DEGREES;
    let s2 = n1 / n2 * th1.sin();
    let mut g = Mobject::group([
        rectangle(8.0, 3.0)
            .move_to(DOWN * 1.5)
            .fill(BLUE_E, 0.45)
            .no_stroke(),
        line(LEFT * 4.0, RIGHT * 4.0).stroke(BLUE_B, 3.0),
        dashed_line(UP * 2.5, DOWN * 2.5).stroke(GRAY_B, 2.0),
        tex_sized(&format!("n_1 = {n1}"), 0.4, WHITE).move_to(v2(-3.0, 1.0)),
        tex_sized(&format!("n_2 = {n2}"), 0.4, WHITE).move_to(v2(-3.0, -1.0)),
    ]);
    let src = Vec2::polar(2.5, PI / 2.0 + th1);
    g.push(arrow(src, Vec2::ZERO).color(YELLOW).with_tag("incident"));
    g.push(arc_centered(Vec2::ZERO, 0.8, PI / 2.0, th1).stroke(YELLOW, 2.5));
    if s2.abs() <= 1.0 {
        let th2 = s2.asin();
        g.push(
            arrow(Vec2::ZERO, Vec2::polar(2.5, -PI / 2.0 + th2))
                .color(ORANGE)
                .with_tag("refracted"),
        );
        g.push(arc_centered(Vec2::ZERO, 0.8, -PI / 2.0, th2).stroke(ORANGE, 2.5));
    } else {
        // total internal reflection
        g.push(
            arrow(Vec2::ZERO, Vec2::polar(2.5, PI / 2.0 - th1))
                .color(RED)
                .with_tag("reflected"),
        );
    }
    g.push(
        arrow(Vec2::ZERO, Vec2::polar(2.0, PI / 2.0 - th1))
            .color(YELLOW)
            .stroke_opacity(0.35)
            .fill_opacity(0.35),
    );
    g
}

/// White light dispersing through a triangular prism into a spectrum.
pub fn prism_dispersion(center: Vec2, size: f64) -> Mobject {
    let tri = triangle(size)
        .move_to(center)
        .fill(WHITE, 0.12)
        .stroke(GRAY_A, 3.0);
    let entry = center + v2(-size * 0.35, 0.0);
    let exit = center + v2(size * 0.32, -size * 0.12);
    let mut g = Mobject::group([tri, line(entry + v2(-3.0, -0.8), entry).stroke(WHITE, 4.0)]);
    let spectrum = [RED, ORANGE, YELLOW, GREEN, BLUE, PURPLE];
    for (i, c) in spectrum.iter().enumerate() {
        let k = i as f64 / (spectrum.len() - 1) as f64;
        g.push(
            line(entry, exit + v2(0.0, -0.05 * k))
                .stroke(*c, 2.0)
                .stroke_opacity(0.7),
        );
        let dir = Vec2::from_angle(-0.25 - 0.22 * k);
        g.push(line(exit + v2(0.0, -0.05 * k), exit + dir * 3.2).stroke(*c, 4.0));
    }
    g
}

// ---------------------------------------------------------------- circuits

fn lead_split(a: Vec2, b: Vec2, body: f64) -> (Vec2, Vec2, Vec2, Vec2) {
    let d = (b - a).normalize();
    let mid = (a + b) / 2.0;
    (mid - d * (body / 2.0), mid + d * (body / 2.0), d, d.perp())
}

pub fn resistor(a: Vec2, b: Vec2) -> Mobject {
    let (p, q, _, _) = lead_split(a, b, 1.2);
    Mobject::group([line(a, p), zigzag(p, q, 6, 0.18, 0.0), line(q, b)]).stroke(WHITE, 3.5)
}

pub fn capacitor(a: Vec2, b: Vec2) -> Mobject {
    let (p, q, _, n) = lead_split(a, b, 0.25);
    Mobject::group([
        line(a, p),
        line(p + n * 0.4, p - n * 0.4),
        line(q + n * 0.4, q - n * 0.4),
        line(q, b),
    ])
    .stroke(WHITE, 3.5)
}

pub fn inductor(a: Vec2, b: Vec2) -> Mobject {
    let (p, q, d, n) = lead_split(a, b, 1.2);
    let mut path = Path::new();
    path.move_to(p);
    let loops = 4;
    let w = 1.2 / loops as f64;
    for i in 0..loops {
        let s = p + d * (w * i as f64);
        path.cubic_to(s + n * 0.35, s + d * w + n * 0.35, s + d * w);
    }
    Mobject::group([line(a, p), Mobject::from_path(path), line(q, b)]).stroke(WHITE, 3.5)
}

/// Battery / DC source (long plate = +, at the `b` side).
pub fn battery(a: Vec2, b: Vec2) -> Mobject {
    let (p, q, _, n) = lead_split(a, b, 0.22);
    Mobject::group([
        line(a, p).stroke(WHITE, 3.5),
        line(p + n * 0.22, p - n * 0.22).stroke(WHITE, 6.0),
        line(q + n * 0.45, q - n * 0.45).stroke(WHITE, 3.5),
        line(q, b).stroke(WHITE, 3.5),
        tex_sized("+", 0.3, RED_B).move_to(q + n * 0.6 + (q - p).normalize() * 0.15),
    ])
}

pub fn ac_source(a: Vec2, b: Vec2) -> Mobject {
    let (p, q, d, _) = lead_split(a, b, 0.8);
    let c = (p + q) / 2.0;
    let wave = wave_line(c - d * 0.25, c + d * 0.25, 0.5, 0.12, 0.0);
    Mobject::group([line(a, p), circle_at(c, 0.4), wave, line(q, b)]).stroke(WHITE, 3.5)
}

/// Light bulb; `brightness` 0..1 adds a glow.
pub fn bulb(a: Vec2, b: Vec2, brightness: f64) -> Mobject {
    let (p, q, d, n) = lead_split(a, b, 0.8);
    let c = (p + q) / 2.0;
    let mut glass = circle_at(c, 0.4).stroke(WHITE, 3.5);
    if brightness > 0.0 {
        glass = glass
            .fill(YELLOW, (0.85 * brightness) as f32)
            .glow(0.35 * brightness, Some(YELLOW));
    }
    let filament = Mobject::group([
        line(c - d * 0.28 + n * 0.28, c + d * 0.28 - n * 0.28),
        line(c - d * 0.28 - n * 0.28, c + d * 0.28 + n * 0.28),
    ])
    .stroke(WHITE, 2.5);
    Mobject::group([
        line(a, p).stroke(WHITE, 3.5),
        glass,
        filament,
        line(q, b).stroke(WHITE, 3.5),
    ])
}

pub fn switch(a: Vec2, b: Vec2, closed: bool) -> Mobject {
    let (p, q, d, n) = lead_split(a, b, 0.9);
    let lever_end = if closed { q } else { p + (d * 0.85 + n * 0.45) };
    Mobject::group([
        line(a, p),
        line(p, lever_end),
        line(q, b),
        dot_r(p, 0.07, WHITE),
        dot_r(q, 0.07, WHITE),
    ])
    .stroke(WHITE, 3.5)
}

/// Meter (ammeter "A", voltmeter "V", galvanometer "G").
pub fn meter(a: Vec2, b: Vec2, letter: &str) -> Mobject {
    let (p, q, _, _) = lead_split(a, b, 0.8);
    let c = (p + q) / 2.0;
    Mobject::group([
        line(a, p).stroke(WHITE, 3.5),
        circle_at(c, 0.4).stroke(WHITE, 3.5),
        text_sized(letter, 0.4, WHITE).move_to(c),
        line(q, b).stroke(WHITE, 3.5),
    ])
}

pub fn ground(p: Vec2) -> Mobject {
    Mobject::group([
        line(p, p + DOWN * 0.3),
        line(p + v2(-0.35, -0.3), p + v2(0.35, -0.3)),
        line(p + v2(-0.22, -0.42), p + v2(0.22, -0.42)),
        line(p + v2(-0.1, -0.54), p + v2(0.1, -0.54)),
    ])
    .stroke(WHITE, 3.5)
}

pub fn wire(points: &[Vec2]) -> Mobject {
    polyline(points).stroke(WHITE, 3.5).line_cap(LineCap::Round)
}

pub fn junction(p: Vec2) -> Mobject {
    dot_r(p, 0.09, WHITE)
}

/// Charge carriers moving along a closed circuit loop (conventional current direction).
pub fn current_dots(loop_path: &Path, t: f64, speed: f64, spacing: f64, color: Color) -> Mobject {
    let len = loop_path.length().max(1e-9);
    let n = (len / spacing).floor().max(1.0) as usize;
    let offset = (speed * t / len).rem_euclid(1.0);
    Mobject::group((0..n).map(|i| {
        let a = (i as f64 / n as f64 + offset).rem_euclid(1.0);
        dot_r(loop_path.point_from_proportion(a), 0.07, color).glow(0.08, None)
    }))
}

/// A rectangular circuit loop path through the given corners (closed).
pub fn circuit_loop(corners: &[Vec2]) -> Path {
    Path::polyline(corners, true)
}

// ----------------------------------------------------------------- fields

/// Point charge symbol: red ⊕ / blue ⊖.
pub fn point_charge(p: Vec2, q: f64) -> Mobject {
    let (c, s) = if q >= 0.0 { (RED, "+") } else { (BLUE, "-") };
    let r = 0.22 + 0.04 * q.abs().min(4.0);
    Mobject::group([
        circle_at(p, r).no_stroke().sheen(c, UL),
        text_sized(s, r * 1.5, WHITE).move_to(p),
    ])
}

/// Electric field at `p` from point charges (k = 1).
pub fn e_field(charges: &[(Vec2, f64)], p: Vec2) -> Vec2 {
    charges.iter().fold(Vec2::ZERO, |acc, (c, q)| {
        let d = p - *c;
        let r2 = d.length_sq().max(1e-6);
        acc + d * (q / (r2 * r2.sqrt()))
    })
}

/// Electric potential at `p` (k = 1).
pub fn e_potential(charges: &[(Vec2, f64)], p: Vec2) -> f64 {
    charges
        .iter()
        .map(|(c, q)| q / (p - *c).length().max(1e-6))
        .sum()
}

/// Field lines starting around positive charges (or negatives, traced backwards).
pub fn field_lines(
    charges: &[(Vec2, f64)],
    per_unit_charge: usize,
    bounds: Rect,
    color: Color,
) -> Mobject {
    let has_pos = charges.iter().any(|c| c.1 > 0.0);
    let mut g = Mobject::empty();
    for (c, q) in charges {
        if (has_pos && *q <= 0.0) || (!has_pos && *q >= 0.0) {
            continue;
        }
        let n = ((q.abs() * per_unit_charge as f64).round() as usize).max(4);
        let sign = if has_pos { 1.0 } else { -1.0 };
        for i in 0..n {
            let a = TAU * (i as f64 + 0.5) / n as f64;
            let mut p = *c + Vec2::from_angle(a) * 0.25;
            let mut pts = vec![p];
            for _ in 0..1200 {
                let e = e_field(charges, p);
                if e.length() < 1e-9 {
                    break;
                }
                p += e.normalize() * (0.03 * sign);
                if !bounds.contains(p) {
                    pts.push(p);
                    break;
                }
                pts.push(p);
                if charges
                    .iter()
                    .any(|(cc, qq)| qq * sign < 0.0 && (p - *cc).length() < 0.2)
                {
                    break;
                }
            }
            if pts.len() > 3 {
                let curve = smooth_curve(&pts, false).stroke(color, 2.5);
                // arrow head in the middle of the line
                let mid = pts.len() / 2;
                let dir = (pts[mid + 1] - pts[mid]).normalize() * sign;
                g.push(Mobject::group([
                    curve,
                    arrow_tip(
                        pts[mid] + dir * 0.12,
                        dir,
                        0.18,
                        0.15,
                        TipShape::Triangle,
                        color,
                    ),
                ]));
            }
        }
    }
    g
}

/// Equipotential contours at the given potential levels.
pub fn equipotentials(
    charges: &[(Vec2, f64)],
    levels: &[f64],
    bounds: Rect,
    color: Color,
) -> Mobject {
    let mut g = Mobject::empty();
    for lv in levels {
        let segs = crate::graph::marching_squares(
            &|x, y| e_potential(charges, v2(x, y)) - lv,
            bounds.min.x,
            bounds.max.x,
            bounds.min.y,
            bounds.max.y,
            120,
        );
        let mut p = Path::new();
        for (a, b) in segs {
            p.move_to(a);
            p.line_to(b);
        }
        g.push(
            Mobject::from_path(p)
                .stroke(color, 2.0)
                .stroke_opacity(0.8)
                .dashed(0.08, 0.06),
        );
    }
    g
}

/// Magnetic field circles around a straight wire perpendicular to the screen.
/// `out_of_page`: current direction (⊙ out, ⊗ in) sets the arrow sense.
pub fn wire_magnetic_field(p: Vec2, out_of_page: bool, rings: usize, color: Color) -> Mobject {
    let symbol = if out_of_page {
        Mobject::group([circle_at(p, 0.22).stroke(WHITE, 3.0), dot_r(p, 0.06, WHITE)])
    } else {
        Mobject::group([
            circle_at(p, 0.22).stroke(WHITE, 3.0),
            cross(0.25).move_to(p).stroke(WHITE, 3.0),
        ])
    };
    let mut g = Mobject::group([symbol]);
    for i in 1..=rings {
        let r = 0.45 * i as f64;
        let ccw = out_of_page;
        g.push(
            circle_at(p, r)
                .stroke(color, 2.5)
                .stroke_opacity(1.0 - 0.12 * i as f32),
        );
        let a = PI / 4.0 * i as f64;
        let pt = p + Vec2::polar(r, a);
        let dir = Vec2::from_angle(a).perp() * if ccw { 1.0 } else { -1.0 };
        g.push(arrow_tip(
            pt + dir * 0.09,
            dir,
            0.18,
            0.15,
            TipShape::Triangle,
            color,
        ));
    }
    g
}

/// Bar magnet with N/S poles and field lines (dipole approximation).
pub fn bar_magnet(center: Vec2, length: f64) -> Mobject {
    let half = length / 2.0;
    let n_pole = rectangle(half, 0.5)
        .move_to(center + RIGHT * (half / 2.0))
        .fill(RED, 0.95)
        .no_stroke();
    let s_pole = rectangle(half, 0.5)
        .move_to(center + LEFT * (half / 2.0))
        .fill(BLUE, 0.95)
        .no_stroke();
    let charges = [
        (center + RIGHT * (half * 0.8), 1.0),
        (center + LEFT * (half * 0.8), -1.0),
    ];
    let lines = field_lines(
        &charges,
        10,
        Rect::from_center(center, length * 4.0, length * 2.5),
        GRAY_B,
    );
    Mobject::group([
        lines,
        n_pole,
        s_pole,
        text_sized("N", 0.35, WHITE).move_to(center + RIGHT * (half / 2.0)),
        text_sized("S", 0.35, WHITE).move_to(center + LEFT * (half / 2.0)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagrams_build() {
        let o = Vec2::ZERO;
        for m in [
            pendulum_mobject(o, 2.0, 0.3, 0.2, RED),
            spring_block(-4.0, 0.0, 0.0, 0.8, BLUE),
            hanging_spring(UP * 3.0, -1.0, 0.7, GREEN),
            incline(v2(-3.0, -2.0), 5.0, 0.5, GRAY),
            block_on_incline(v2(-3.0, -2.0), 0.5, 2.0, 0.6, BLUE),
            free_body_diagram(o, 1.0, &[(UP * 1.5, "N", GREEN), (DOWN * 1.5, "mg", RED)]),
            fixed_pulley(o, 0.5, 2.0, GRAY),
            rope_over_pulley(v2(-1.0, -2.0), o, 0.5, v2(1.0, -2.0), true, GOLD),
            atwood_machine(UP, 0.5, 2.0, 1.0, RED, BLUE),
            projectile_path(v2(-5.0, -2.0), 8.0, 0.8, 9.81, YELLOW),
            vector_components(o, v2(2.0, 1.5), YELLOW, ("v", "v_x", "v_y")),
            energy_bars(2.0, 1.0, 3.0, 2.0),
            travelling_wave(-5.0, 5.0, 1.0, 2.0, 1.0, 0.3, BLUE),
            standing_wave(-4.0, 8.0, 1.0, 3, 1.0, 0.1, TEAL),
            wavefronts(o, 0.5, 1.0, 1.0, 4.0, BLUE),
            lens_ray_diagram(o, 2.0, 4.0, 1.5),
            lens_ray_diagram(o, 2.0, 1.0, 1.0),
            lens_ray_diagram(o, -2.0, 4.0, 1.5),
            curved_mirror(o, 4.0, 3.0, true),
            refraction_diagram(1.0, 1.5, 40.0),
            refraction_diagram(1.5, 1.0, 60.0),
            prism_dispersion(o, 2.0),
            resistor(o, RIGHT * 2.0),
            capacitor(o, RIGHT * 2.0),
            inductor(o, RIGHT * 2.0),
            battery(o, UP * 2.0),
            ac_source(o, UP * 2.0),
            bulb(o, RIGHT * 2.0, 1.0),
            switch(o, RIGHT * 2.0, false),
            meter(o, RIGHT * 2.0, "A"),
            ground(o),
            point_charge(o, -2.0),
            wire_magnetic_field(o, true, 3, BLUE),
            bar_magnet(o, 2.0),
        ] {
            assert!(m.leaf_count() > 0 && m.width() > 0.0);
        }
        let (di, mag) = thin_lens(2.0, 4.0);
        assert!((di - 4.0).abs() < 1e-9 && (mag + 1.0).abs() < 1e-9);
        let charges = [(v2(-1.0, 0.0), 1.0), (v2(1.0, 0.0), -1.0)];
        let fl = field_lines(&charges, 8, Rect::from_center(o, 10.0, 8.0), WHITE);
        assert!(fl.children.len() >= 6);
        let eq = equipotentials(
            &charges,
            &[-0.5, 0.5],
            Rect::from_center(o, 6.0, 4.0),
            WHITE,
        );
        assert_eq!(eq.children.len(), 2);
        let lp = circuit_loop(&[v2(-2.0, -1.0), v2(2.0, -1.0), v2(2.0, 1.0), v2(-2.0, 1.0)]);
        assert!(current_dots(&lp, 0.5, 1.0, 0.5, YELLOW).children.len() >= 20);
    }

    #[test]
    fn rope_wraps_over_top() {
        let rope = rope_over_pulley(v2(-0.5, -3.0), Vec2::ZERO, 0.5, v2(0.5, -3.0), true, GOLD);
        let r = rope.bounds().unwrap();
        let (lo, hi) = (r.min, r.max);
        assert!(
            (hi.y - 0.5).abs() < 0.02,
            "rope should pass over the top: {hi:?}"
        );
        assert!(lo.y < -2.9);
        let under = rope_over_pulley(v2(-0.5, 3.0), Vec2::ZERO, 0.5, v2(0.5, 3.0), false, GOLD);
        let lo = under.bounds().unwrap().min;
        assert!((lo.y + 0.5).abs() < 0.02, "rope should pass under: {lo:?}");
    }
}
