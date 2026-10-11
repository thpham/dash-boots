mod common;

use common::*;
use snes_wasm::tiled::parse_tiled_json;
use snes_wasm::world::level::LevelData;

fn shipped(name: &str) -> LevelData {
    let path = format!("{}/../public/levels/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}.json: {e}"));
    parse_tiled_json(&text).unwrap_or_else(|e| panic!("[{name}] {e}"))
}

/// The first jump of a level happens before the dash boots (reach ≈ 8 tiles),
/// so it must sit inside the plain jump curve: walking reach ≈ 4 tiles,
/// running ≈ 6.5. Level 1 teaches 3-tile gaps; level 4 opened with a 5-tile
/// gap onto a 1-tile pillar — a frame-perfect precision landing no human
/// input curve can make fair. The rule: early gaps ≤ 3 tiles wide, landing
/// platforms ≥ 3 tiles, measured up to the boots pickup.
#[test]
fn level4_first_gaps_fit_the_jump_curve() {
    let lvl = shipped("level4");
    let boots_col = (lvl.spawns.dash_boots[0].x / lvl.tile_size) as usize;
    let w = lvl.width as usize;
    let solid: Vec<bool> =
        (0..w).map(|x| (0..lvl.height as usize).any(|y| lvl.collision[y * w + x] > 0)).collect();
    // Walk the columns left of the boots and collect gap/landing runs.
    let mut gaps = vec![];
    let mut x = 0;
    while x < boots_col {
        if !solid[x] {
            let gap_start = x;
            while x < boots_col && !solid[x] {
                x += 1;
            }
            let gap = x - gap_start;
            let land_start = x;
            while x < boots_col && solid[x] {
                x += 1;
            }
            gaps.push((gap, x - land_start));
        } else {
            x += 1;
        }
    }
    assert_eq!(gaps.len(), 2, "two gaps before the boots: {gaps:?}");
    for (i, &(gap, landing)) in gaps.iter().enumerate() {
        assert!(gap <= 3, "gap #{i} is {gap} tiles wide (jump curve tops at 4 walking)");
        assert!(landing >= 3, "gap #{i} lands on {landing} tile(s); a fair landing needs 3");
    }
}

#[test]
fn rift_with_flat_runway_is_valid() {
    let d = level(&["..........", ".PR.......", "##########"]);
    assert_eq!(d.spawns.rifts.len(), 1);
    d.validate().unwrap();
}

#[test]
fn rift_before_a_pit_is_rejected() {
    let d = level(&["..........", ".PR.......", "######.###"]);
    let e = d.validate().unwrap_err();
    assert!(e.contains("rift") && e.contains("flat"), "{e}");
}

#[test]
fn rift_before_a_step_is_rejected() {
    let d = level(&["..........", ".PR...#...", "##########"]);
    let e = d.validate().unwrap_err();
    assert!(e.contains("rift") && e.contains("flat"), "{e}");
}

#[test]
fn rift_with_enemy_in_runway_is_rejected() {
    let d = level(&["..........", ".PR..E....", "##########"]);
    let e = d.validate().unwrap_err();
    assert!(e.contains("rift") && e.contains("enemy"), "{e}");
}

#[test]
fn enemy_just_after_runway_is_fine() {
    let d = level(&["...........", ".PR......E.", "###########"]);
    d.validate().unwrap();
}
