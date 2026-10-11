//! SNES-inspired platformer core. Everything game-related lives here (level
//! parsing, simulation, camera, HUD/screens, render list); the host (browser)
//! only provides input bits, frame time, textures, a GL context and audio.

pub mod audio;
pub mod game;
pub mod input;
pub mod math;
pub mod render;
pub mod sprites;
pub mod tiled;
pub mod tuning;
pub mod sim {
    pub mod fixed_timestep;
}
pub mod world {
    pub mod camera;
    pub mod enemy;
    pub mod entities;
    pub mod level;
    pub mod particles;
    pub mod physics;
    pub mod player;
    pub mod shift;
}

pub use tuning::{Tuning, TUNING};

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

/// SFX file stems indexed by SFX id (see `audio.rs` for the event encoding).
#[wasm_bindgen]
pub fn sfx_names() -> Vec<String> {
    audio::SFX_NAMES.iter().map(|s| s.to_string()).collect()
}

/// Music file stems indexed by music id.
#[wasm_bindgen]
pub fn music_names() -> Vec<String> {
    audio::MUSIC_NAMES.iter().map(|s| s.to_string()).collect()
}

#[wasm_bindgen]
pub struct GameHandle {
    game: game::Game,
    render: render::RenderList,
    audio: Vec<u8>,
}

#[wasm_bindgen]
impl GameHandle {
    /// Build a game from a Tiled JSON map (see `tiled.rs` for the conventions).
    #[wasm_bindgen(constructor)]
    pub fn new(level_json: &str, view_w: u32, view_h: u32) -> Result<GameHandle, JsError> {
        let data = tiled::parse_tiled_json(level_json).map_err(|e| JsError::new(&e))?;
        let (w, h) = (view_w as f32, view_h as f32);
        Ok(GameHandle {
            game: game::Game::new(data, w, h),
            render: render::RenderList::new(w, h),
            audio: Vec::new(),
        })
    }

    pub fn set_texture_size(&mut self, tex: u32, w: u32, h: u32) {
        self.render.set_texture_size(tex, w as f32, h as f32);
    }

    /// Advance by `dt_ms` of wall time with the given held buttons (bitmask, see
    /// `input.rs`), then rebuild the render list. Returns the number of fixed steps run.
    pub fn frame(&mut self, buttons: u32, dt_ms: f32) -> u32 {
        let steps = self.game.advance(input::InputState::from_bits(buttons), dt_ms / 1000.0);
        self.audio.extend(self.game.drain_audio().into_iter().map(|e| e.code()));
        render::draw_game(&self.game, &mut self.render);
        steps
    }

    pub fn vertices_ptr(&self) -> *const f32 {
        self.render.verts.as_ptr()
    }

    /// Number of f32 values (vertex count × 8).
    pub fn vertices_len(&self) -> usize {
        self.render.verts.len()
    }

    pub fn batches_ptr(&self) -> *const u32 {
        self.render.batches.as_ptr()
    }

    /// Number of u32 values (batch count × 3).
    pub fn batches_len(&self) -> usize {
        self.render.batches.len()
    }

    /// Audio event codes since the last call.
    pub fn take_audio_events(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.audio)
    }

    /// Pause if playing (e.g. the tab lost focus).
    pub fn pause(&mut self) {
        self.game.pause();
    }

    pub fn restart(&mut self) {
        self.game.restart();
    }
}
