//! SNES-inspired platformer core. Everything game-related lives here (level
//! parsing, simulation, camera, HUD/screens, render list); the host (browser)
//! only provides input bits, frame time, textures, a GL context and audio.

pub mod audio;
pub mod c_heap;
pub mod game;
pub mod input;
pub mod math;
pub mod music;

/// Campaign music rendered in-wasm by the tracker (stereo interleaved f32,
/// 22050 Hz, peak-normalized). One buffer per track, rendered on demand.
#[wasm_bindgen]
pub fn render_music(id: u8) -> Vec<f32> {
    music::render_music(id)
}

/// Incremental music render (the host pumps it between frames so the game
/// keeps painting; a whole-track render can take seconds of simulation).
#[wasm_bindgen]
pub struct MusicRenderTask {
    task: music::tracker::MusicRenderTask,
}

#[wasm_bindgen]
impl MusicRenderTask {
    #[wasm_bindgen(constructor)]
    pub fn new(id: u8) -> Result<MusicRenderTask, JsError> {
        Ok(Self { task: music::tracker::MusicRenderTask::new(id) })
    }

    /// Advance up to `chunks` internal steps; `true` = done, `take` is ready.
    /// Each step is sub-millisecond work by design.
    pub fn step(&mut self, chunks: u32) -> bool {
        self.task.step(chunks as usize)
    }

    /// The rendered buffer, normalized; consumes the task (final call).
    pub fn take(self) -> Vec<f32> {
        self.task.take()
    }
}

/// SFX synthesized in-wasm from the ported Python recipes (stereo duplicated,
/// 22050 Hz, peak-normalized).
#[wasm_bindgen]
pub fn render_sfx(id: u8) -> Vec<f32> {
    music::render_sfx(id)
}
pub mod render;
pub mod sprites;
pub mod tiled;
pub mod tuning;
pub mod sim {
    pub mod fixed_timestep;
}
pub mod world {
    pub mod bumps;
    pub mod camera;
    pub mod cheese;
    pub mod cow;
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
    retro: render::RenderList,
    hud: render::RenderList,
    audio: Vec<u8>,
    scene: render::scene3d::Scene3d,
    plan: [f32; render::plan::PLAN_LEN],
    settings: render::plan::RenderSettings,
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
            retro: render::RenderList::new(w, h),
            hud: render::RenderList::new(w, h),
            audio: Vec::new(),
            scene: Default::default(),
            plan: [0.0; render::plan::PLAN_LEN],
            settings: Default::default(),
        })
    }

    pub fn set_texture_size(&mut self, tex: u32, w: u32, h: u32) {
        self.retro.set_texture_size(tex, w as f32, h as f32);
        self.hud.set_texture_size(tex, w as f32, h as f32);
    }

    /// Advance by `dt_ms` of wall time with the given held buttons (bitmask, see
    /// `input.rs`), then rebuild the render list. Returns the number of fixed steps run.
    pub fn frame(&mut self, buttons: u32, dt_ms: f32) -> u32 {
        let steps = self.game.advance(input::InputState::from_bits(buttons), dt_ms / 1000.0);
        self.audio.extend(self.game.drain_audio().into_iter().map(|e| e.code()));
        render::draw_game(&self.game, &mut self.retro, &mut self.hud);
        let params = render::plan::effective_params(&self.game, &self.settings);
        if params.world_mix > 0.0 {
            render::scene3d::build_scene(&self.game, &params, &mut self.scene);
        } else {
            self.scene.clear();
        }
        render::plan::build_plan(&self.game, &self.settings, &params, &mut self.plan);
        steps
    }

    pub fn plan_ptr(&self) -> *const f32 {
        self.plan.as_ptr()
    }
    pub fn plan_len(&self) -> usize {
        self.plan.len()
    }
    pub fn blocks_ptr(&self) -> *const f32 {
        self.scene.blocks.as_ptr()
    }
    pub fn blocks_len(&self) -> usize {
        self.scene.blocks.len()
    }
    pub fn billboards_ptr(&self) -> *const f32 {
        self.scene.billboards.as_ptr()
    }
    pub fn billboards_len(&self) -> usize {
        self.scene.billboards.len()
    }
    pub fn shadows_ptr(&self) -> *const f32 {
        self.scene.shadows.as_ptr()
    }
    pub fn shadows_len(&self) -> usize {
        self.scene.shadows.len()
    }

    /// Canvas size in device pixels.
    pub fn set_viewport(&mut self, w: u32, h: u32) {
        self.settings.canvas_w = w.max(320) as f32;
        self.settings.canvas_h = h.max(180) as f32;
    }
    pub fn set_future_supported(&mut self, on: bool) {
        self.settings.future_supported = on;
    }
    pub fn set_max_future_height(&mut self, px: u32) {
        self.settings.max_future_h = px.max(180) as f32;
    }
    pub fn set_reduced_motion(&mut self, on: bool) {
        self.settings.reduced_motion = on;
    }
    pub fn set_crt_enabled(&mut self, on: bool) {
        self.settings.crt_enabled = on;
    }

    pub fn vertices_ptr(&self) -> *const f32 {
        self.retro.verts.as_ptr()
    }

    /// Number of f32 values (vertex count × 8).
    pub fn vertices_len(&self) -> usize {
        self.retro.verts.len()
    }

    pub fn batches_ptr(&self) -> *const u32 {
        self.retro.batches.as_ptr()
    }

    /// Number of u32 values (batch count × 3).
    pub fn batches_len(&self) -> usize {
        self.retro.batches.len()
    }

    pub fn hud_vertices_ptr(&self) -> *const f32 {
        self.hud.verts.as_ptr()
    }

    pub fn hud_vertices_len(&self) -> usize {
        self.hud.verts.len()
    }

    pub fn hud_batches_ptr(&self) -> *const u32 {
        self.hud.batches.as_ptr()
    }

    pub fn hud_batches_len(&self) -> usize {
        self.hud.batches.len()
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

    /// Music track index for the current level (see `audio::MUSIC_NAMES`).
    pub fn set_music(&mut self, id: u8) {
        self.game.set_music(id);
    }

    /// Whether this is the campaign's last level (adapts the clear-screen text).
    pub fn set_last_level(&mut self, last: bool) {
        self.game.set_last_level(last);
    }
}
