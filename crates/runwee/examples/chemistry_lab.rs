//! Chemistry: periodic table, orbiting Bohr atom, electron configuration,
//! balanced equation, molecules in 2D and rotating 3D, lab glassware.
use runwee::chemistry::*;
use runwee::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut s = Scene::new();
    let pt = s.register(periodic_table(0.62).shift(UP * 0.3));
    s.play(lagged_start(vec![fade_in(pt)], 0.0).run_time(1.5));
    let fe = s.register(element_tile(26, 1.6).to_corner(DR, 0.4));
    s.play(grow_from_center(fe));
    s.wait(0.8);
    s.play([fade_out(pt), fade_out(fe)]);

    // Bohr model of sodium + configuration
    let t0 = s.time();
    let atom = s.always_redraw(move |ctx| bohr_atom(11, 2.4, (ctx.t - t0) * 1.2).shift(LEFT * 3.2));
    let cfg = s.register(
        tex_sized(&electron_configuration_tex(11, false), 0.6, YELLOW)
            .shift(RIGHT * 2.8 + UP * 1.4),
    );
    let boxes = s.register(orbital_box_diagram(11, 0.45).shift(RIGHT * 2.8 + DOWN * 0.4));
    s.play(write(cfg));
    s.play(create(boxes).run_time(1.5));
    s.wait(1.0);
    s.play([fade_out(atom), fade_out(cfg), fade_out(boxes)]);

    // Balancing an equation
    let eq = balanced_equation(&["C3H8", "O2"], &["CO2", "H2O"]).unwrap();
    let raw = s.register(chem("C3H8 + O2 -> CO2 + H2O").scale(1.2).shift(UP * 1.5));
    s.play(write(raw));
    let target = chem(&eq).scale(1.2).shift(UP * 1.5);
    s.play(transform(raw, target));
    let mm = s.register(
        tex_sized(
            &format!(
                r"M(\mathrm{{C_3H_8}}) = {:.2}\ \mathrm{{g/mol}}",
                molar_mass("C3H8")
            ),
            0.5,
            TEAL,
        )
        .shift(DOWN * 0.2),
    );
    s.play(fade_in_shift(mm, UP * 0.3));

    // 2D benzene and 3D ethanol side by side
    let benz = s.register(
        molecule_2d("benzene")
            .unwrap()
            .render(0.9, true)
            .shift(LEFT * 3.5 + DOWN * 2.0),
    );
    s.play(create(benz).run_time(1.5));
    let rig = s.camera3d(70.0, -60.0);
    let mol = s.add_3d(rig, molecule_3d("ethanol").unwrap().mesh(0.9));
    s.add_updater(mol, |m, _| m.shift(v2(3.2, -2.0)));
    s.ambient_rotation(rig, 0.8, 4.0);
    s.wait(3.5);
    s.play([fade_out(raw), fade_out(mm), fade_out(benz), fade_out(mol)]);

    // glassware
    let t1 = s.time();
    let lab = s.always_redraw(move |ctx| {
        let tt = ctx.t - t1;
        Mobject::group([
            beaker(2.0, 2.6, 0.55, BLUE).shift(LEFT * 4.0),
            bubbles(Rect::from_center(v2(-4.0, -0.55), 1.7, 1.2), 12, tt, 3),
            erlenmeyer_flask(2.4, 0.35 + 0.1 * (tt * 0.8).sin().abs(), PINK).shift(LEFT * 0.8),
            burette(4.2, 0.8 - 0.05 * tt, GREEN_B).shift(RIGHT * 1.8 + UP * 0.3),
            bunsen_burner(1.8, tt).shift(RIGHT * 4.3 + DOWN * 1.2),
            ph_scale(6.0, 7.0 + 3.0 * (tt * 0.5).sin()).to_edge(DOWN, 0.5),
        ])
    });
    s.wait(4.0);
    s.play(fade_out(lab));
    run(&s, "chemistry_lab")
}
