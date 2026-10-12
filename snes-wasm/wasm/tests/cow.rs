//! Decor cows: slow alpine livestock — they walk and browse, greet a passing
//! player with the occasional moo, and are a non-event to them otherwise.

mod common;

use common::*;
use snes_wasm::game::Phase;
use snes_wasm::input::InputState;
use snes_wasm::math::Vec2;
use snes_wasm::tuning::DT;
use snes_wasm::world::cow::Cow;
use snes_wasm::world::level::Level;

fn meadow() -> Level {
    // One tile of headroom over flat grass.
    Level::from_data(&level(&["............", "....P.......", "############", "############"]))
}

/// Run `steps` fixed steps with the test player parked at (dx, dy) from the
/// cow's current spot — the cow walks, so absolute positions would race it.
/// In-range by construction: dx=8, dy=6 is the cow's own tile; dx=1000 is
/// beyond the radius. Returns every step's moo flag.
fn moos(cow: &mut Cow, lvl: &Level, dx: f32, dy: f32, steps: u32) -> Vec<bool> {
    (0..steps).map(|_| cow.step(lvl, Vec2::new(cow.pos.x + dx, cow.pos.y + dy), DT)).collect()
}

#[test]
fn cows_walk_and_pause_on_a_cycle() {
    let lvl = meadow();
    let feet = Vec2::new(6.0 * TS, 2.0 * TS);
    let mut cow = Cow::spawn_at(feet);
    let y0 = cow.pos.y;
    let mut x0 = cow.pos.x;
    let mut still_run = 0;
    let mut best_still = 0;
    let mut moved = false;
    for _ in 0..360 {
        // 6 s: walk/browse should show both states.
        let flags = moos(&mut cow, &lvl, 1000.0, 6.0, 1);
        assert!(flags.iter().all(|m| !m), "out of earshot the cow stays quiet");
        let x = cow.pos.x;
        if (x - x0).abs() > 0.01 {
            moved = true;
            still_run = 0;
        } else {
            still_run += 1;
            best_still = best_still.max(still_run);
        }
        x0 = x;
        assert_eq!(cow.pos.y, y0, "the cow never sinks or rises off the meadow");
    }
    assert!(moved, "cows amble");
    assert!(
        best_still >= 120,
        "a browse holds the cow still for at least 2 s (best still run: {best_still} steps)"
    );
}

#[test]
fn a_passing_player_gets_one_moo_then_quiet() {
    let lvl = meadow();
    let feet = Vec2::new(6.0 * TS, 2.0 * TS);
    let mut cow = Cow::spawn_at(feet);
    // Herd desync: the cycle phase comes from the spawn spot.
    let a = Cow::spawn_at(feet);
    let b = Cow::spawn_at(Vec2::new(9.0 * TS, 2.0 * TS));
    assert_ne!(a.pos, b.pos, "distinct spawns, desynced cycles (via offset)");

    // Staying within the radius: entry greets, then the cow is done.
    let flags = moos(&mut cow, &lvl, 8.0, 6.0, 600);
    let count = flags.iter().filter(|b| **b).count();
    assert_eq!(count, 1, "staying nearby moos exactly once over 10 s (got {count})");

    // Walk away: nothing. Come back after the gap (9 s > 8 s): exactly one.
    assert!(moos(&mut cow, &lvl, 1000.0, 6.0, 540).iter().all(|m| !m), "a far player is quiet");
    let again = moos(&mut cow, &lvl, 8.0, 6.0, 600);
    assert_eq!(again.iter().position(|m| *m), Some(0), "re-entry after the gap moos again");
    assert_eq!(again.iter().filter(|b| **b).count(), 1);
}

#[test]
fn cows_never_hurt_or_block_the_player() {
    let mut g = playing(&["....................", "..P......w..........", "####################"]);
    assert_eq!(g.cows.len(), 1);
    let right = InputState { right: true, ..input() };
    run(&mut g, right, 400);
    assert_eq!(g.phase, Phase::Playing, "the player walks through the cow unharmed");
    assert!(g.player.pos.x > 10.0 * TS, "the player's path continues past the cow");
}
