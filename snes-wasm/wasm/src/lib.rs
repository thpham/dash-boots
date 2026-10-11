mod events;
mod sim {
    pub mod fixed_timestep;
}
mod tuning;
mod world {
    pub mod player;
}

pub use tuning::{Tuning, TUNING};

use events::{FrameEvents, SpriteDraw};
use wasm_bindgen::prelude::*;
use world::player::{InputState, Player};

#[wasm_bindgen]
pub struct GameHandle {
    player: Player,
}

#[wasm_bindgen]
impl GameHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GameHandle {
        GameHandle {
            player: Player::new(),
        }
    }
}

#[wasm_bindgen]
pub fn game_tick(game: &mut GameHandle) -> JsValue {
    // Minimal: no JS input wired yet; keep player stepping deterministic.
    let input = InputState::default();
    game.player.step(&input, 1.0 / 60.0);

    let ev = FrameEvents {
        sprites: vec![SpriteDraw {
            x: game.player.pos.x,
            y: game.player.pos.y,
            w: 16.0,
            h: 16.0,
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        }],
    };

    serde_wasm_bindgen::to_value(&ev).unwrap()
}
