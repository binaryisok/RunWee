//! Question-solving videos: a narrated kinematics problem, a quadratic solved with
//! the formula, a stoichiometry problem and a multiple-choice quiz, joined with
//! transitions. Captions are generated from the narration automatically.
use runwee::prelude::*;

fn scene_for(p: &Problem) -> Scene {
    let mut s = Scene::new();
    s.solve(p);
    s.wait(1.0);
    s
}

fn main() -> anyhow::Result<()> {
    let kin = scene_for(&kinematics_problem(5.0, 2.0, 4.0).accent(TEAL));
    let quad = scene_for(&quadratic_problem(2.0, -7.0, 3.0));
    let stoich = scene_for(
        &stoichiometry_problem(&["C3H8", "O2"], &["CO2", "H2O"], "C3H8", 22.0, "CO2")
            .unwrap()
            .accent(GREEN),
    );
    let mut quiz = Scene::new();
    quiz.quiz(
        "A ball is thrown straight up. At the highest point its acceleration is",
        &["zero", "$g$ downward", "$g$ upward", "infinite"],
        1,
        3.0,
    );
    quiz.wait(1.0);
    let movie = Movie::new()
        .then(kin)
        .transition(Transition::slide(Dir::FromRight, 0.6))
        .then(quad)
        .transition(Transition::new(Presentation::Iris, 0.7))
        .then(stoich)
        .transition(Transition::fade(0.6))
        .then(quiz);
    run(&movie, "question_solving")
}
