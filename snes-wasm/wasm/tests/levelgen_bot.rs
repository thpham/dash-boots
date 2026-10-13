mod common;

use snes_wasm::levelgen::bot::{self, play, EXPERT};
use snes_wasm::levelgen::compile::random_legal;
use snes_wasm::levelgen::motif::Theme;
use snes_wasm::tiled::parse_tiled_json;

fn shipped(name: &str) -> snes_wasm::world::level::LevelData {
    let path = format!("{}/../public/levels/{name}.json", env!("CARGO_MANIFEST_DIR"));
    parse_tiled_json(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn expert_finishes_every_shipped_level() {
    for name in ["level1", "level2", "level3", "level4"] {
        let r = play(shipped(name), EXPERT, 0, 4);
        assert!(r.won, "[{name}] {r:?}");
    }
}

#[test]
fn stronger_players_finish_whatever_weaker_ones_finish() {
    // The player of difficulty d has every strategy up to d (an expert can
    // also walk carefully): `finishes` must be monotonic in d.
    let mut levels: Vec<_> = ["level1", "level2", "level3", "level4"].iter().map(|n| shipped(n)).collect();
    for seed in 0..12 {
        levels.push(random_legal(Theme::ALL[seed as usize % 4], seed).finish().level_data());
    }
    for (li, data) in levels.iter().enumerate() {
        let wins: Vec<bool> = (1..=5).map(|d| bot::finishes(data, d, 1)).collect();
        for d in 0..4 {
            assert!(
                !wins[d] || wins[d + 1],
                "level #{li}: d{} finishes but d{} does not ({wins:?})",
                d + 1,
                d + 2
            );
        }
        let measured = bot::measure(data, 1);
        assert_eq!(measured, wins.iter().position(|w| *w).map(|i| i as u8 + 1), "level #{li}");
    }
}

#[test]
fn deaths_are_located() {
    let data = common::level(&[
        "..............................",
        ".P............................",
        "#####..............###########",
    ]);
    let r = play(data, EXPERT, 0, 2);
    assert!(!r.won);
    assert_eq!(r.death_x.len(), 3, "stops after max_deaths + 1: {r:?}");
    assert!(r.death_x.iter().all(|x| *x > 40.0 && *x < 320.0));
}

#[test]
fn measure_is_the_weakest_finishing_profile() {
    let flat = common::level(&[
        "......................................",
        ".P..................................G.",
        "######################################",
    ]);
    assert_eq!(bot::measure(&flat, 0), Some(1), "a flat walk is d1");
    assert!(bot::finishes(&flat, 1, 0));
}

#[test]
fn a_long_way_back_after_a_death_is_not_being_stuck() {
    // Death ~150 tiles in, no checkpoint: walking back takes > 8 s each time.
    let row = |c: char, gap: bool| -> String {
        (0..170).map(|x| if gap && (150..166).contains(&x) { '.' } else { c }).collect()
    };
    let mut spawn = row('.', false);
    spawn.replace_range(1..2, "P");
    let data = common::level(&[&row('.', false), &spawn, &row('#', true)]);
    let r = play(data, EXPERT, 0, 2);
    assert_eq!(r.stuck_x, None, "{r:?}");
    assert_eq!(r.death_x.len(), 3, "three tries before giving up: {r:?}");
}
