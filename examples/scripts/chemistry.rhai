// Balancing an equation, an atom model and a molecule — in two scenes.
let header = add(title("Combustion of methane"));
play(write(header));
let eq = register(tex(balance(["CH4", "O2"], ["CO2", "H2O"]), 1.2));
play(write(eq));
play(circumscribe(eq, YELLOW));
wait(0.5);

next_scene("slide:from-right", 0.6);
let atom = register(bohr_atom(8, 2.0, 0.0).shift(LEFT * 3.5 + UP * 0.5));
let cfg = register(tex(electron_config(8), 0.8).shift(LEFT * 3.5 + DOWN * 2.6));
let mol = register(molecule("ethanol", 1.6, true).shift(RIGHT * 3.0));
play(lagged_start([grow_from_center(atom), write(cfg), create(mol)], 0.3));
wait(1.0);
