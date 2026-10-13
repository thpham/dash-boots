use snes_wasm::levelgen::compile::{
    self, random_legal, width, Compiler, CHECKPOINT_MAX, CHECKPOINT_MIN, FINALE_RUN, INTRO_END, MAX_BODY,
    MIN_BODY,
};
use snes_wasm::levelgen::motif::{Motif, Theme, ALL};

#[test]
fn every_motif_footprint_matches_its_width() {
    for theme in Theme::ALL {
        for &m in ALL {
            let mut c = Compiler::new(theme);
            for _ in 0..4 {
                assert!(c.push(Motif::Flat(8)));
            }
            if c.allowed(m) {
                let x = c.x();
                assert!(c.push(m));
                assert_eq!(c.x() - x, width(m), "{theme:?} {m:?}");
            }
        }
    }
}

#[test]
fn random_legal_walks_never_dead_end() {
    for theme in Theme::ALL {
        for seed in 0..400 {
            let c = random_legal(theme, seed);
            assert!(c.can_end(), "{theme:?} seed {seed}: stuck at x={} after {:?}", c.x(), c.motifs);
        }
    }
}

#[test]
fn grammar_properties_hold_on_random_walks() {
    for theme in Theme::ALL {
        for seed in 0..400 {
            let lvl = random_legal(theme, seed).finish();
            lvl.level_data().validate().unwrap_or_else(|e| panic!("{theme:?} seed {seed}: {e}"));
            let rifts: Vec<Motif> = lvl
                .motifs
                .iter()
                .copied()
                .filter(|m| matches!(m, Motif::RiftOut | Motif::RiftBack))
                .collect();
            assert!(!rifts.is_empty() && rifts.len() <= 4, "{theme:?} seed {seed}: {rifts:?}");
            for pair in rifts.chunks(2) {
                assert_eq!(pair, [Motif::RiftOut, Motif::RiftBack]);
            }
            let mut last_cp = 0;
            for s in &lvl.spans {
                if s.x0 < INTRO_END {
                    assert!(
                        s.motif.intro_ok(),
                        "{theme:?} seed {seed}: {:?} in the intro at {}",
                        s.motif,
                        s.x0
                    );
                }
                if s.motif == Motif::Checkpoint {
                    assert!(s.x0 - last_cp >= CHECKPOINT_MIN);
                    last_cp = s.x0;
                } else if s.motif.is_hazard() {
                    assert!(s.x0 - last_cp <= CHECKPOINT_MAX, "{theme:?} seed {seed}: hazard at {}", s.x0);
                }
            }
            assert!(lvl.motifs.iter().filter(|m| **m == Motif::Boots).count() <= 1);
            let body = lvl.width as i32 - FINALE_RUN;
            assert!((MIN_BODY..=MAX_BODY).contains(&body), "body {body}");
        }
    }
}

#[test]
fn cave_roof_stays_whole_and_free_of_coins() {
    for seed in 0..300 {
        let lvl = random_legal(Theme::Cave, seed).finish();
        let w = lvl.width as usize;
        for y in 0..5 {
            for x in 0..w {
                assert_eq!(lvl.collision[y * w + x], 1, "seed {seed}: roof hole at ({x},{y})");
                assert_eq!(lvl.coins[y * w + x], 0);
            }
        }
    }
}

#[test]
fn compile_strict_rejects_illegal_sequences() {
    assert!(compile::compile_strict(Theme::Meadow, &[Motif::Walkers(1)]).is_none(), "enemy in the intro");
    assert!(
        compile::compile_strict(Theme::Meadow, &[Motif::Flat(8); 3]).is_none(),
        "too short, no excursion"
    );
}

#[test]
fn tiled_export_round_trips_through_the_parser() {
    let lvl = random_legal(Theme::Night, 7).finish();
    let json = lvl.to_tiled_json();
    let data = snes_wasm::tiled::parse_tiled_json(&json).expect("parses");
    assert_eq!(data.width, lvl.width);
    assert_eq!(data.spawns.rifts.len(), lvl.level_data().spawns.rifts.len());
    assert!(json.contains(Theme::Night.background()));
}
