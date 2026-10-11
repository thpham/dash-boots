//! Golden-adjacent tests for the in-wasm tracker rendering: every embedded
//! module must produce a bounded, normalized stereo loop, deterministically,
//! with attribution metadata filled in.

use snes_wasm::audio::MUSIC_NAMES;
use snes_wasm::music::tracker::{render_module, MusicRenderTask};
use snes_wasm::music::{render_music, TRACKS};

#[test]
fn every_track_renders_a_plausible_loop() {
    for (id, name) in MUSIC_NAMES.iter().enumerate().take(TRACKS.len()) {
        let out = render_music(id as u8);
        assert!(out.len() >= 22050 * 2 * 10, "[{}] loop too short: {} frames", name, out.len() / 2);
        // longest loop measured by the probe: 249 s (level 2) — the
        // anti-pathology cap must stay well above.
        assert!(out.len() <= 22050 * 2 * 260, "[{}] loop abnormally long: {} frames", name, out.len() / 2);
        assert!(out.iter().all(|s| s.is_finite()), "[{}] non-finite samples", name);
        let peak = out.iter().fold(0.0f32, |a, s| a.max(s.abs()));
        assert!(peak > 0.2, "[{}] silent render?", name);
        assert!(peak <= 0.951, "[{}] normalization overrun", name);
    }
}

#[test]
fn rendering_is_deterministic() {
    let a = render_music(0);
    let b = render_music(0);
    assert_eq!(a, b, "two renders of the same module must be identical");
    assert_ne!(render_music(0), render_music(1), "different tracks render differently");
}

#[test]
fn out_of_bounds_ids_panic_loudly() {
    assert!(std::panic::catch_unwind(|| render_music(TRACKS.len() as u8)).is_err());
}

#[test]
fn track_metadata_is_filled_for_attribution() {
    for t in &TRACKS {
        assert!(!t.title.is_empty(), "missing title");
        assert!(!t.author.is_empty(), "missing author");
        // licenses are deliberately "unlicensed (demoscene archive)" — the
        // test checks attribution completeness, not the license itself.
        assert!(t.license.contains("unlicensed"), "demoscene licenses expected in metadata");
        assert!(!t.source.is_empty(), "missing source URL");
    }
}

/// Re-grade of review Critical 1: incremental rendering (budget-pumped on
/// the host side) must produce EXACTLY the same buffer as a direct render.
#[test]
fn chunked_task_matches_direct_render() {
    for id in 0..TRACKS.len() as u8 {
        let mut task = MusicRenderTask::new(id);
        assert!(!task.step(1), "the first step cannot finish render {id}");
        loop {
            // variable stride (≠ 1) to exercise both branches
            if task.step(7) {
                break;
            }
        }
        assert_eq!(task.take(), render_music(id), "[{id}] incremental render != direct render");
    }
}

/// Re-grade of review Important 3: auditioning tools must render the SAME
/// region the game delivers (the looped region on a fresh context), not the
/// intro + first loop_count cycle of an ad-hoc context.
#[test]
fn render_module_matches_render_by_id() {
    for id in 0..TRACKS.len() as u8 {
        assert_eq!(
            render_module(TRACKS[id as usize].data.to_vec()),
            render_music(id),
            "[{id}] render_module(data) != render_music(id)"
        );
    }
}
