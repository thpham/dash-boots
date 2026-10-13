use snes_wasm::levelgen::bot;
use snes_wasm::levelgen::compile::compile_strict;
use snes_wasm::levelgen::model::{sample, Model};
use snes_wasm::levelgen::motif::{vocab, Theme};

/// 40 held-out seeds per cell: with 20, one unlucky sample decides a 90 % cell.
const SEEDS: u64 = 40;

#[test]
fn embedded_weights_load() {
    let m = Model::embedded().expect("embedded weights match the vocabulary");
    assert_eq!(m.vocab, vocab::size());
}

#[test]
fn samples_are_legal_accepted_and_ordered_by_difficulty() {
    let m = Model::embedded().unwrap();
    let mut mean = [0.0f32; 5];
    for theme in Theme::ALL {
        for d in 1..=5u8 {
            let mut ok = 0;
            for i in 0..SEEDS {
                let seed = 50_000 + i;
                let tr = sample(&m, theme, d, seed, 1.0).expect("never dead-ends");
                assert!(compile_strict(theme, &tr.level.motifs).is_some(), "{theme:?} d{d}: illegal sample");
                let data = tr.level.level_data();
                ok += bot::finishes_reliably(&data, d, seed) as u64;
                mean[d as usize - 1] += bot::measure(&data, seed).unwrap_or(6) as f32;
            }
            assert!(ok * 10 >= SEEDS * 9, "{theme:?} d{d}: only {ok}/{SEEDS} accepted (≥ 90 % required)");
        }
    }
    assert!(mean.windows(2).all(|w| w[1] > w[0]), "measured difficulty must increase with the tag: {mean:?}");
}
