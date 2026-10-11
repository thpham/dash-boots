//! Fixed-layout composition plan read by the host every frame (spec §4.2).

use super::camera3d::pivot_camera;
use crate::game::Game;
use crate::world::shift::ShiftParams;

pub const PLAN_LEN: usize = 34;

pub mod idx {
    pub const VIEW_PROJ: usize = 0; // 16 floats, column-major
    pub const EYE: usize = 16; // 3
    pub const TIME: usize = 19;
    pub const WORLD_MIX: usize = 20;
    pub const FUTURE_W: usize = 21;
    pub const FUTURE_H: usize = 22;
    pub const GLITCH: usize = 23;
    pub const CRT: usize = 24;
    pub const WARMTH: usize = 25;
    pub const VIGNETTE: usize = 26;
    pub const BLOOM: usize = 27;
    pub const CAM_RIGHT: usize = 28; // 3
    pub const FUTURE_ENABLED: usize = 31;
    pub const FLOOR_Y: usize = 32;
    pub const FOCUS_X: usize = 33;
}

#[derive(Clone, Copy, Debug)]
pub struct RenderSettings {
    /// Canvas size in device pixels (an integer multiple of 320×180).
    pub canvas_w: f32,
    pub canvas_h: f32,
    pub max_future_h: f32,
    pub future_supported: bool,
    pub reduced_motion: bool,
    pub crt_enabled: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            canvas_w: 320.0,
            canvas_h: 180.0,
            max_future_h: 1440.0,
            future_supported: true,
            reduced_motion: false,
            crt_enabled: true,
        }
    }
}

pub fn effective_params(g: &Game, s: &RenderSettings) -> ShiftParams {
    let mut p = g.shift.params(s.reduced_motion);
    if !s.future_supported {
        p.world_mix = 0.0;
        p.glitch = 0.0;
    }
    p
}

/// Future render target size: 320×180 at focus 0, up to the canvas (capped) at focus 1.
pub fn future_size(focus: f32, s: &RenderSettings) -> (f32, f32) {
    let max_h = s.max_future_h.min(s.canvas_h).max(180.0);
    let h = (180.0 + (max_h - 180.0) * focus.clamp(0.0, 1.0)).round();
    ((h * 16.0 / 9.0).round(), h)
}

pub fn build_plan(g: &Game, s: &RenderSettings, p: &ShiftParams, out: &mut [f32; PLAN_LEN]) {
    let (vw, vh) = (g.camera.view_w, g.camera.view_h);
    let cam = pivot_camera(g.camera.pixel_pos(), vw, vh, g.player.feet(), p.pivot);
    out[idx::VIEW_PROJ..idx::VIEW_PROJ + 16].copy_from_slice(&cam.view_proj.to_cols_array());
    out[idx::EYE..idx::EYE + 3].copy_from_slice(&cam.eye.to_array());
    out[idx::TIME] = (g.frame as f64 / 60.0 % 3600.0) as f32;
    out[idx::WORLD_MIX] = p.world_mix;
    let (fw, fh) = future_size(p.focus, s);
    out[idx::FUTURE_W] = fw;
    out[idx::FUTURE_H] = fh;
    out[idx::GLITCH] = p.glitch;
    out[idx::CRT] = if s.crt_enabled { p.crt } else { 0.0 };
    out[idx::WARMTH] = p.warmth;
    out[idx::VIGNETTE] = 0.25 + 0.1 * p.world_mix;
    out[idx::BLOOM] = 0.8 * p.world_mix;
    let right = cam.view.row(0).truncate().normalize_or_zero();
    out[idx::CAM_RIGHT..idx::CAM_RIGHT + 3].copy_from_slice(&right.to_array());
    out[idx::FUTURE_ENABLED] = if s.future_supported { 1.0 } else { 0.0 };
    out[idx::FLOOR_Y] = -g.level.height_px() - 0.1;
    out[idx::FOCUS_X] = g.player.feet().x;
}
