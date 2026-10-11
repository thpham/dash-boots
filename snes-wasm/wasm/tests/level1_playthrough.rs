// A naive bot plays every shipped level through the real Tiled parser: proves
// each level loads, is completable, and that nothing gets the player stuck.
// (Named level1_playthrough for history: it outgrew level1.)

use snes_wasm::game::{Game, Phase};
use snes_wasm::input::InputState;
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

struct Bot {
    jump_hold: u32,
}

impl Bot {
    fn decide(&mut self, g: &Game) -> InputState {
        let p = &g.player;
        let lvl = &g.level;
        let mut inp = InputState { right: true, dash: true, ..Default::default() };
        let front = p.pos.x + player::WIDTH;
        let feet = p.feet().y;

        // Low ceiling ahead (crawl space): stand-height blocked, crouch-height free.
        let stand_blocked =
            overlaps_solid(lvl, front + 2.0, feet - player::HEIGHT_STAND, 12.0, player::HEIGHT_STAND - 1.0);
        let crouch_free = !overlaps_solid(
            lvl,
            front + 2.0,
            feet - player::HEIGHT_CROUCH,
            12.0,
            player::HEIGHT_CROUCH - 1.0,
        );
        let tunnel = (stand_blocked && crouch_free)
            || (p.crouching && overlaps_solid(lvl, p.pos.x, feet - player::HEIGHT_STAND, player::WIDTH, 8.0));
        if tunnel && p.on_ground {
            inp.down = true;
            return inp;
        }

        // Gap planning: if a short gap starts soon, walk instead of run so the
        // jump lands on the next (possibly small) platform.
        let has_floor = |x: f32| overlaps_solid(lvl, x, feet - 40.0, 1.0, lvl.height_px() - feet + 41.0);
        if let Some(start) = (0..6).map(|i| front + 2.0 + i as f32 * 8.0).find(|x| !has_floor(*x)) {
            let width =
                (0..16).map(|i| start + i as f32 * 8.0).take_while(|x| !has_floor(*x)).count() as f32 * 8.0;
            let land = start + width;
            let landing =
                (0..8).map(|i| land + i as f32 * 8.0).take_while(|x| has_floor(*x)).count() as f32 * 8.0;
            if width <= 56.0 && landing <= 48.0 {
                inp.dash = false;
            }
        }

        let wall_ahead = overlaps_solid(lvl, front, feet - 20.0, 20.0, 18.0);
        let gap_ahead = (0..=3).any(|i| {
            let x = front + 2.0 + i as f32 * 2.0;
            !overlaps_solid(lvl, x, feet, 1.0, lvl.height_px() - feet + 1.0)
        });
        let enemy_ahead = g.enemies.iter().any(|e| {
            e.is_dangerous()
                && e.pos.x - front > -4.0
                && e.pos.x - front < 40.0
                && (e.pos.y + 14.0 - feet).abs() < 20.0
        });

        // Release the button once landed so the next jump is a fresh press.
        if p.on_ground && self.jump_hold > 0 && self.jump_hold < 20 {
            self.jump_hold = 0;
            return inp;
        }
        // Air control: when descending over a landing spot with a pit right
        // after it, brake (like a human aiming for a small platform).
        if !p.on_ground && p.vel.y > 0.0 {
            let probe = |x: f32| overlaps_solid(lvl, x, feet, 1.0, 64.0);
            if probe(p.pos.x + player::WIDTH * 0.5) && !probe(front + 10.0) {
                inp.right = false;
                inp.left = true;
            }
        }
        if self.jump_hold > 0 {
            self.jump_hold -= 1;
            inp.jump = true;
        } else if p.on_ground && (wall_ahead || gap_ahead || enemy_ahead) {
            // Small hop for low steps, full jump otherwise.
            let ts = lvl.tile_size;
            let wall_h = [2.0, 8.0, 14.0, 20.0]
                .iter()
                .map(|dx| {
                    (1..=5)
                        .take_while(|i| lvl.is_solid_at_pixel(front + dx, feet - *i as f32 * ts + 1.0))
                        .count()
                })
                .max()
                .unwrap_or(0);
            self.jump_hold = if wall_ahead && !gap_ahead && !enemy_ahead && wall_h <= 1 { 6 } else { 24 };
            inp.jump = true;
        }
        inp
    }
}

#[test]
fn bot_completes_every_shipped_level() {
    for name in ["level1", "level2", "level3", "level4"] {
        let mut g = load(name);
        let mut bot = Bot { jump_hold: 0 };
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
