mod events;
mod sim {
    pub mod fixed_timestep;
}
mod tuning;
mod world {
    pub mod entities;
    pub mod player;
}

pub use tuning::{Tuning, TUNING};

use events::{FrameEvents, SpriteDraw};
use wasm_bindgen::prelude::*;
use world::player::{InputState, Player};

#[wasm_bindgen]
pub struct GameHandle {
    player: Player,
    entities: world::entities::Entities,
    spawn_player_pos: world::player::Vec2,
}

#[wasm_bindgen]
impl GameHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GameHandle {
        let spawn_player_pos = world::player::Vec2 { x: 40.0, y: 40.0 };
        let mut player = Player::new();
        player.pos = spawn_player_pos;

        GameHandle {
            player,
            entities: world::entities::Entities::new_hardcoded(),
            spawn_player_pos,
        }
    }
}

#[wasm_bindgen]
pub fn game_tick(game: &mut GameHandle) -> JsValue {
    // Minimal: no JS input wired yet; keep player stepping deterministic.
    let input = InputState::default();
    let dt = 1.0 / 60.0;

    if !game.entities.won {
        game.player.step(&input, dt);

        // Enemy patrol update.
        for e in &mut game.entities.enemies {
            e.pos.x += e.vel_x * dt;
            if e.pos.x < e.patrol_min_x {
                e.pos.x = e.patrol_min_x;
                e.vel_x = e.vel_x.abs();
            } else if e.pos.x > e.patrol_max_x {
                e.pos.x = e.patrol_max_x;
                e.vel_x = -e.vel_x.abs();
            }
        }

        // Overlaps.
        let player_aabb = world::entities::Aabb {
            pos: game.player.pos,
            size: world::player::Vec2 { x: 16.0, y: 16.0 },
        };

        // Coins.
        game.entities.coins.retain(|c| {
            if player_aabb.overlaps(&c.aabb()) {
                game.entities.coin_count += 1;
                false
            } else {
                true
            }
        });

        // Enemies.
        let mut stomped_any = false;
        game.entities.enemies.retain(|e| {
            if player_aabb.overlaps(&e.aabb()) {
                // Stomp if falling and player's bottom is above enemy top.
                let stomp_epsilon = 2.0;
                let player_bottom = player_aabb.bottom();
                let enemy_top = e.aabb().top();
                if game.player.vel.y > 0.0 && player_bottom <= enemy_top + stomp_epsilon {
                    stomped_any = true;
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });
        if stomped_any {
            game.player.vel.y = -180.0;
        } else {
            // If we overlapped an enemy but didn't stomp, treat as hit.
            let hit = game
                .entities
                .enemies
                .iter()
                .any(|e| player_aabb.overlaps(&e.aabb()));
            if hit {
                game.player.pos = game.spawn_player_pos;
                game.player.vel = world::player::Vec2::default();
                game.entities.reset();
            }
        }

        // Goal.
        if player_aabb.overlaps(&game.entities.goal.aabb()) {
            game.entities.won = true;
        }
    }

    // Emit sprites (renderer may ignore UVs for now, but keep events consistent).
    let mut sprites = Vec::new();

    // Player sprite.
    sprites.push(SpriteDraw {
        x: game.player.pos.x,
        y: game.player.pos.y,
        w: 16.0,
        h: 16.0,
        u0: 0.0,
        v0: 0.0,
        u1: 1.0,
        v1: 1.0,
    });

    // Coins.
    for c in &game.entities.coins {
        sprites.push(SpriteDraw {
            x: c.pos.x,
            y: c.pos.y,
            w: 12.0,
            h: 12.0,
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        });
    }

    // Enemies.
    for e in &game.entities.enemies {
        sprites.push(SpriteDraw {
            x: e.pos.x,
            y: e.pos.y,
            w: 16.0,
            h: 16.0,
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        });
    }

    // Goal.
    sprites.push(SpriteDraw {
        x: game.entities.goal.pos.x,
        y: game.entities.goal.pos.y,
        w: 16.0,
        h: 24.0,
        u0: 0.0,
        v0: 0.0,
        u1: 1.0,
        v1: 1.0,
    });

    let ev = FrameEvents {
        sprites,
        coin_count: game.entities.coin_count,
        won: game.entities.won,
    };

    serde_wasm_bindgen::to_value(&ev).unwrap()
}
