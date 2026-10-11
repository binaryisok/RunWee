use runwee_lesson::{build, inspect, LessonOptions};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixture/topic")
}

#[test]
fn inspects_fixture_lesson() {
    let info = inspect(&fixture()).expect("inspect");
    assert_eq!(info.id, "fixture-01");
    assert_eq!(info.presets.len(), 1);
    assert_eq!(info.intro.len(), 1);
}

#[test]
fn builds_and_renders_fixture_lesson() {
    let opts = LessonOptions::new(fixture(), 10.0);
    let scene = build(&opts).expect("build");
    assert!(scene.duration() > 1.0);
}
