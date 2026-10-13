use snes_wasm::levelgen::compile::compile_strict;
use snes_wasm::levelgen::handmade::CAMPAIGN;
use snes_wasm::levelgen::motif::{Motif, Theme};
use snes_wasm::levelgen::reward::{repetition, score};

#[test]
fn repetition_counts_immediate_repeats() {
    let s = [Motif::Flat(3), Motif::Flat(3), Motif::Flat(3), Motif::Gap(2)];
    assert!((repetition(&s) - 2.0 / 3.0).abs() < 1e-6);
    assert_eq!(repetition(&[Motif::Gap(2)]), 0.0);
}

#[test]
fn unfinishable_for_the_target_player_is_minus_one() {
    // A 5-wide gap: walking profiles (d1, d2) cannot clear it.
    let mut seq = vec![Motif::Flat(8); 3];
    seq.extend([
        Motif::Gap(5),
        Motif::RiftOut,
        Motif::Flat(8),
        Motif::Flat(8),
        Motif::Flat(5),
        Motif::RiftBack,
    ]);
    seq.extend(std::iter::repeat_n(Motif::Flat(8), 9));
    let lvl = compile_strict(Theme::Meadow, &seq).expect("legal");
    let r = score(&lvl, 1, 0);
    assert_eq!(r.total, -1.0, "{r:?}");
    assert_eq!(r.measured, None);
}

#[test]
fn campaign_levels_score_well_at_their_measured_difficulty() {
    for t in &CAMPAIGN {
        let lvl = t.compile();
        let r = score(&lvl, 5, 0);
        assert!(r.measured.is_some(), "[{}] the expert finishes", t.name);
        assert!(r.style > 0.99);
        let exact = score(&lvl, r.measured.unwrap(), 0);
        assert!(exact.total > 1.05, "[{}] {exact:?}", t.name);
    }
}
