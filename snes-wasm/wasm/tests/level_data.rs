mod common;

use common::*;
use snes_wasm::tiled::parse_tiled_json;
use snes_wasm::world::level::LevelData;

/// The Swiss campaign's two new object kinds: the goat is an enemy variant
/// (the cheese-wheel dropper), cows are non-interactive decor.
#[test]
fn goat_and_cow_spawns_parse() {
    let d = level(&["..P.", ".g.w", "####", "####", "####", "####", "####"]);
    assert_eq!(d.spawns.goats.len(), 1);
    assert_eq!(d.spawns.cows.len(), 1);
    // Maps without the new kinds parse fine: empty lists, not errors.
    let plain = level(&["..P..GG"]);
    assert!(plain.spawns.goats.is_empty());
    assert!(plain.spawns.cows.is_empty());
}

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

/// The swiss campaign's per-level enrichment contract: decor cows on every
/// level, cheese goats exactly where the wheel mechanic is taught and used,
/// and level 1 as the (no-goat) tutorial plus its coin hideout — a 2-col
/// alcove holding the level's densest coin pocket.
#[test]
fn swiss_enrichment_content_is_pinned() {
    let d1 = shipped("level1");
    assert!(d1.spawns.goats.is_empty(), "the meadow is the no-goat tutorial");
    assert!(d1.spawns.cows.len() >= 3, "cows graze level1 flats: {} cows", d1.spawns.cows.len());
    let w1 = d1.width as usize;
    let dens = (0..w1 - 1)
        .map(|x| {
            let spans = ((0..d1.height as usize).filter(|&y| d1.coins[y * w1 + x] > 0).count())
                + ((0..d1.height as usize).filter(|&y| d1.coins[y * w1 + x + 1] > 0).count());
            (spans, x)
        })
        .max()
        .unwrap();
    assert!(
        dens.0 >= 5,
        "level1 hideout holds >= 5 coins in one 2-col span (best {}) at x={}",
        dens.0,
        dens.1
    );

    let d2 = shipped("level2");
    assert_eq!(d2.spawns.goats.len(), 1, "one goat on the sunset plateau");
    assert!(!d2.spawns.cows.is_empty(), "level2 cows");
    let d3 = shipped("level3");
    assert_eq!(d3.spawns.goats.len(), 2, "the fromagerie pen holds two goats");
    let d4 = shipped("level4");
    assert_eq!(d4.spawns.goats.len(), 2, "two goats guard the night crawls");
    assert!(d4.spawns.cows.len() >= 3, "level4 herd");
}

/// `tools/gen_level.py` is the source of truth for the shipped maps: running
/// it must reproduce every shipped level byte for byte (the hand format is
/// its "one row per line" compact writer), two runs stay identical, and the
/// runs are redirected to a temp dir so the shipped files are never touched.
#[test]
fn gen_level_output_matches_shipped_bytes() {
    let root = format!("{}/..", env!("CARGO_MANIFEST_DIR"));
    let tools = format!("{root}/tools/gen_level.py");
    let run = |name: &str| {
        let dir = std::env::temp_dir().join(format!("dash-gen-level-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let out = std::process::Command::new("python3")
            .arg(&tools)
            .arg("--out")
            .arg(&dir)
            .current_dir(&root)
            .output()
            .expect("run gen_level.py");
        assert!(out.status.success(), "gen_level.py failed: {}", String::from_utf8_lossy(&out.stderr));
        dir
    };
    let a = run("a");
    let b = run("b");
    for level in ["level1", "level2", "level3", "level4"] {
        let gen = std::fs::read(a.join(format!("{level}.json"))).unwrap();
        assert_eq!(gen, std::fs::read(b.join(format!("{level}.json"))).unwrap(), "{level}: two runs differ");
        let shipped = std::fs::read(format!("{root}/public/levels/{level}.json")).unwrap();
        assert_eq!(gen, shipped, "{level}: generator output drifted from the shipped map");
    }
}
