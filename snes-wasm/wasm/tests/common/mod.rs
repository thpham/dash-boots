#![allow(dead_code)]

use snes_wasm::game::Game;
use snes_wasm::input::InputState;
use snes_wasm::tuning::DT;
use snes_wasm::world::level::{LevelData, SpawnPoint, Spawns};

pub const TS: f32 = 16.0;

/// Build a level from ASCII art. `#` solid, `o` coin, `P` player, `E` enemy,
/// `G` goal, `C` checkpoint, `B` dash boots, `R` rift. Spawns use the cell's feet point.
/// Without `G`, the goal is placed at the far right bottom (out of the way).
pub fn level(rows: &[&str]) -> LevelData {
    let h = rows.len() as u32;
    let w = rows[0].len() as u32;
    let mut collision = vec![0u8; (w * h) as usize];
    let mut coins = vec![0u8; (w * h) as usize];
    let mut player = None;
    let mut goal = None;
    let mut enemies = vec![];
    let mut goats = vec![];
    let mut cows = vec![];
    let mut checkpoints = vec![];
    let mut dash_boots = vec![];
    let mut rifts = vec![];
    for (ty, row) in rows.iter().enumerate() {
        assert_eq!(row.len() as u32, w, "ragged row {ty}");
        for (tx, ch) in row.chars().enumerate() {
            let i = ty * w as usize + tx;
            let feet = SpawnPoint { x: tx as f32 * TS + 8.0, y: (ty + 1) as f32 * TS };
            match ch {
                '#' => collision[i] = 1,
                'o' => coins[i] = 1,
                'P' => player = Some(feet),
                'E' => enemies.push(feet),
                'g' => goats.push(feet),
                'w' => cows.push(feet),
                'G' => goal = Some(feet),
                'C' => checkpoints.push(feet),
                'B' => dash_boots.push(feet),
                'R' => rifts.push(feet),
                _ => {}
            }
        }
    }
    LevelData {
        width: w,
        height: h,
        tile_size: TS,
        collision,
        coins,
        terrain: vec![],
        tileset_columns: 8,
        spawns: Spawns {
            player: player.expect("level needs a P"),
            goal: goal.unwrap_or(SpawnPoint { x: w as f32 * TS - 8.0, y: h as f32 * TS + 1000.0 }),
            enemies,
            goats,
            cows,
            checkpoints,
            dash_boots,
            rifts,
        },
    }
}

/// A game already in the Playing phase.
pub fn playing(rows: &[&str]) -> Game {
    let mut g = Game::new(level(rows), 320.0, 180.0);
    g.start();
    g
}

pub fn input() -> InputState {
    InputState::default()
}

/// Run `steps` fixed steps holding `inp`, through the real frame path.
pub fn run(g: &mut Game, inp: InputState, steps: u32) {
    for _ in 0..steps {
        let n = g.advance(inp, DT);
        assert_eq!(n, 1, "advance(DT) must run exactly one step");
    }
}

/// Run until `pred` holds (max `limit` steps). Returns steps taken.
pub fn run_until(g: &mut Game, inp: InputState, limit: u32, pred: impl Fn(&Game) -> bool) -> Option<u32> {
    for i in 0..limit {
        if pred(g) {
            return Some(i);
        }
        g.advance(inp, DT);
    }
    None
}
