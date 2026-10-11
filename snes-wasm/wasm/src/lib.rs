mod events;

use events::{FrameEvents, SpriteDraw};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GameHandle {}

#[wasm_bindgen]
impl GameHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GameHandle {
        GameHandle {}
    }
}

#[wasm_bindgen]
pub fn game_tick(_game: &mut GameHandle) -> JsValue {
    let ev = FrameEvents {
        sprites: vec![SpriteDraw {
            x: 20.0,
            y: 20.0,
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
