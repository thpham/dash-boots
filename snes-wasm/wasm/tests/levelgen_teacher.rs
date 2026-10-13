use snes_wasm::levelgen::bot::{self, play, EXPERT};
use snes_wasm::levelgen::compile::compile_strict;
use snes_wasm::levelgen::motif::{Motif, Theme};
use snes_wasm::levelgen::teacher::generate;

#[test]
fn teacher_output_is_always_legal() {
    for theme in Theme::ALL {
        for d in 1..=5 {
            for seed in 0..40 {
                let s = generate(theme, d, seed);
                assert!(compile_strict(theme, &s).is_some(), "{theme:?} d{d} seed {seed}: {s:?}");
                assert!(s.contains(&Motif::RiftOut), "an excursion");
                assert!(s.iter().filter(|m| **m == Motif::Checkpoint).count() >= 2, "sections");
            }
        }
    }
}

#[test]
fn teacher_is_deterministic() {
    assert_eq!(generate(Theme::Cave, 3, 42), generate(Theme::Cave, 3, 42));
    assert_ne!(generate(Theme::Cave, 3, 42), generate(Theme::Cave, 3, 43));
}

#[test]
fn the_expert_finishes_most_teacher_levels() {
    for theme in Theme::ALL {
        let mut won = 0;
        for d in 1..=5 {
            for seed in 0..20 {
                let lvl = compile_strict(theme, &generate(theme, d, seed)).unwrap();
                won += play(lvl.level_data(), EXPERT, seed, 2).won as u32;
            }
        }
        assert!(won >= 80, "{theme:?}: expert finished only {won}/100");
    }
}

#[test]
fn target_difficulty_moves_the_measured_one() {
    for theme in Theme::ALL {
        let mean = |d: u8| -> f32 {
            (0..20)
                .map(|seed| {
                    let lvl = compile_strict(theme, &generate(theme, d, seed)).unwrap();
                    bot::measure(&lvl.level_data(), seed).unwrap_or(6) as f32
                })
                .sum::<f32>()
                / 20.0
        };
        let (easy, hard) = (mean(1), mean(5));
        assert!(hard > easy + 0.5, "{theme:?}: measured d1 {easy} vs d5 {hard}");
    }
}
