// A naive bot plays every shipped level through the real Tiled parser: proves
// each level loads, is completable, and that nothing gets the player stuck.
// (Named level1_playthrough for history: it outgrew level1.)

use snes_wasm::game::{Game, Phase};
use snes_wasm::input::InputState;
use snes_wasm::levelgen::bot::{Bot, EXPERT};
use snes_wasm::tiled::parse_tiled_json;
use snes_wasm::tuning::DT;
use snes_wasm::world::physics::overlaps_solid;
use snes_wasm::world::player;

fn load(name: &str) -> Game {
    let path = format!("{}/../public/levels/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}.json: {e}"));
    let data = parse_tiled_json(&text).unwrap_or_else(|e| panic!("[{name}] {e}"));
    let mut g = Game::new(data, 320.0, 180.0);
    g.start();
    g
}

#[test]
fn bot_completes_every_shipped_level() {
    for name in ["level1", "level2", "level3", "level4"] {
        let mut g = load(name);
        let mut bot = Bot::new(EXPERT, 0);
        let mut best_x = 0.0f32;
        let mut last_progress = 0u32;
        let max_steps = 60 * 240;
        let mut won_at = None;

        for step in 0..max_steps {
            let inp = if g.phase == Phase::Playing { bot.decide(&g) } else { InputState::default() };
            g.advance(inp, DT);
            if g.phase == Phase::Won {
                won_at = Some(step);
                break;
            }
            if g.player.pos.x > best_x + 8.0 {
                best_x = g.player.pos.x;
                last_progress = step;
            }
            assert!(
                step - last_progress < 60 * 8,
                "[{name}] bot stuck around x={} (tile {}), y={} deaths={}",
                g.player.pos.x,
                (g.player.pos.x / 16.0) as i32,
                g.player.pos.y,
                g.deaths
            );
        }
        let steps = won_at.unwrap_or_else(|| {
            panic!("[{name}] did not reach the goal; best x={best_x} deaths={}", g.deaths)
        });
        println!(
            "[{name}] won in {:.1}s, coins {}/{}, deaths {}",
            steps as f32 / 60.0,
            g.coins_collected,
            g.coins_total,
            g.deaths
        );
        // Every level must be gentle enough for a naive bot: a couple of
        // deaths at most (checkpoints put them back at most one section back).
        assert!(g.deaths <= 4, "[{name}] too punishing for a naive bot: {} deaths", g.deaths);
        use snes_wasm::world::shift::World;
        assert_eq!(g.shift.world(), World::Past, "[{name}] the flag is reached in the past");
        assert!(g.rifts.iter().all(|r| !r.active), "[{name}] every excursion crossed");
    }
}

#[test]
fn level1_has_expected_content() {
    let g = load("level1");
    assert!(g.coins_total >= 40);
    assert!(g.enemies.len() >= 10);
    assert_eq!(g.pickups.len(), 1, "one dash boots pickup");
    assert_eq!(g.checkpoints.len(), 3);
    assert_eq!(g.rifts.len(), 4, "two excursions: 2 rifts out, 2 back");
    // Player spawns standing on ground.
    let p = g.player;
    assert!(overlaps_solid(&g.level, p.pos.x, p.feet().y, player::WIDTH, 1.0));
}
