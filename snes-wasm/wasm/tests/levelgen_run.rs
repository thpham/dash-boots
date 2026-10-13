use snes_wasm::levelgen::model::Model;
use snes_wasm::levelgen::motif::Theme;
use snes_wasm::levelgen::run::{Run, Source, MODEL_TRIES};

#[test]
fn progression_rises_every_two_levels_and_rotates_themes() {
    let r = Run { seed: 1, start: 2 };
    let d: Vec<u8> = (0..8).map(|n| r.difficulty(n)).collect();
    assert_eq!(d, [2, 2, 3, 3, 4, 4, 5, 5]);
    assert_eq!(r.difficulty(1000), 5);
    let t: Vec<Theme> = (0..5).map(|n| r.theme(n)).collect();
    assert_eq!(t, [Theme::Meadow, Theme::Sunset, Theme::Cave, Theme::Night, Theme::Meadow]);
    assert_eq!(Run { seed: 1, start: 9 }.difficulty(0), 5, "start is clamped");
    assert_eq!(Run { seed: 1, start: 0 }.difficulty(0), 1);
}

#[test]
fn same_run_same_levels_different_seeds_differ() {
    let m = Model::embedded().unwrap();
    let a = Run { seed: 7, start: 1 };
    assert_eq!(a.level(3, Some(&m)).level.motifs, a.level(3, Some(&m)).level.motifs);
    assert_ne!(a.level_seed(3), Run { seed: 8, start: 1 }.level_seed(3));
    assert_ne!(a.level_seed(3), a.level_seed(4));
}

#[test]
fn a_failing_model_falls_back_to_the_teacher_then_the_campaign() {
    let r = Run { seed: 3, start: 1 };
    let mut calls = 0;
    let lvl = r.level_with(0, |_, _, _| {
        calls += 1;
        None
    });
    assert_eq!(calls, MODEL_TRIES);
    assert!(matches!(lvl.source, Source::Teacher | Source::Handmade), "{:?}", lvl.source);
    assert!(Run { seed: 3, start: 5 }.level(5, None).level.width > 0, "no model at all still yields a level");
}

#[test]
fn garbage_weights_degrade_to_teacher_levels() {
    assert!(Model::from_bytes(b"LGM2garbage").is_err());
    let lvl = Run { seed: 11, start: 2 }.level(0, None);
    assert!(!matches!(lvl.source, Source::Model(_)));
}
