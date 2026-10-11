//! Sound engine living in the wasm crate: sfx synth (ported from gen_audio.py)
//! and xm tracker rendering (libxm).

pub mod sfx;
pub mod synth;
pub mod tracker;
pub mod tracks;

pub fn render_music(id: u8) -> Vec<f32> {
    tracker::render_music(id)
}

pub fn render_sfx(id: u8) -> Vec<f32> {
    sfx::render_sfx(id)
}

pub use tracks::TRACKS;
