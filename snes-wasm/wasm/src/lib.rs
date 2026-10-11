mod events;
mod sim {
    pub mod fixed_timestep;
}
mod tuning;
pub mod world {
    pub mod entities;
    pub mod level;
    pub mod player;
}

pub use tuning::{Tuning, TUNING};

use events::{Camera, FrameEvents, SpriteDraw, TilemapRenderData};
use serde_wasm_bindgen;
use wasm_bindgen::prelude::*;
use world::player::{InputState, Player};

#[wasm_bindgen]
pub struct GameHandle {
    player: Player,
    entities: world::entities::Entities,
    spawn_player_pos: world::player::Vec2,
    level: world::level::Level,
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
            level: world::level::Level {
                width: 0,
                height: 0,
                tile_size: 16.0,
                collision: Vec::new(),
                terrain: Vec::new(),
                coins: Vec::new(),
            },
        }
    }
}

#[wasm_bindgen]
pub fn game_load_level(game: &mut GameHandle, level_js: JsValue) {
    let level: world::level::LevelData = serde_wasm_bindgen::from_value(level_js).unwrap();
    game.level = world::level::Level::from_data(level);
}

#[wasm_bindgen]
pub fn game_tick(game: &mut GameHandle, input_js: JsValue) -> JsValue {
    let input: InputState = serde_wasm_bindgen::from_value(input_js).unwrap_or_default();
    let dt = 1.0 / 60.0;

    let mut sfx: Vec<&'static str> = Vec::new();

    if !game.entities.won {
        sfx = game.player.step(&input, &game.level, dt);

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

    // Camera: follow player, clamp to level bounds.
    // NOTE: viewport size is currently fixed to the canvas size used by the web renderer.
    let viewport_w = 256.0;
    let viewport_h = 224.0;
    let level_w_px = game.level.width as f32 * game.level.tile_size;
    let level_h_px = game.level.height as f32 * game.level.tile_size;

    let mut cam_x = game.player.pos.x + 8.0 - viewport_w * 0.5;
    let mut cam_y = game.player.pos.y + 8.0 - viewport_h * 0.5;

    let max_x = (level_w_px - viewport_w).max(0.0);
    let max_y = (level_h_px - viewport_h).max(0.0);
    cam_x = cam_x.clamp(0.0, max_x);
    cam_y = cam_y.clamp(0.0, max_y);

    let ev = FrameEvents {
        sprites,
        coin_count: game.entities.coin_count,
        won: game.entities.won,
        sfx: sfx.into_iter().map(|s| s.to_string()).collect(),
        camera: Camera { x: cam_x, y: cam_y },
        tilemap: TilemapRenderData {
            width: game.level.width,
            height: game.level.height,
            tile_size: game.level.tile_size,
            terrain: game.level.terrain.clone(),
            coins: game.level.coins.clone(),
        },
    };

    serde_wasm_bindgen::to_value(&ev).unwrap()
}
