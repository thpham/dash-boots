mod common;

use common::*;
use snes_wasm::input::InputState;
use snes_wasm::render::plan::*;
use snes_wasm::render::scene3d::{build_scene, Scene3d};

fn settings() -> RenderSettings {
    RenderSettings { canvas_w: 1920.0, canvas_h: 1080.0, ..RenderSettings::default() }
}

const LEVEL: &[&str] = &[
    "....................................",
    "..PR....................R...........",
    "####################################",
];

#[test]
fn future_size_goes_from_native_past_to_capped_canvas() {
    let s = settings();
    assert_eq!(future_size(0.0, &s), (320.0, 180.0));
    assert_eq!(future_size(1.0, &s), (1920.0, 1080.0));
    let big = RenderSettings { canvas_w: 3840.0, canvas_h: 2160.0, ..s };
    assert_eq!(future_size(1.0, &big), (2560.0, 1440.0), "capped at 1440 px high");
    let small = RenderSettings { canvas_w: 320.0, canvas_h: 180.0, ..s };
    assert_eq!(future_size(1.0, &small), (320.0, 180.0));
}

/// Plan index layout contract. The host's `src/platform/plan.ts` mirrors these
/// exact numbers (it reads the WASM block raw, with no serialization), so the
/// layout may only change together with that file, never by accident.
#[test]
fn plan_layout_contract() {
    assert_eq!(PLAN_LEN, 34);
    assert_eq!(idx::VIEW_PROJ, 0); // 16 floats, column-major
    assert_eq!(idx::EYE, 16); // 3
    assert_eq!(idx::TIME, 19);
    assert_eq!(idx::WORLD_MIX, 20);
    assert_eq!(idx::FUTURE_W, 21);
    assert_eq!(idx::FUTURE_H, 22);
    assert_eq!(idx::GLITCH, 23);
    assert_eq!(idx::CRT, 24);
    assert_eq!(idx::WARMTH, 25);
    assert_eq!(idx::VIGNETTE, 26);
    assert_eq!(idx::BLOOM, 27);
    assert_eq!(idx::CAM_RIGHT, 28); // 3
    assert_eq!(idx::FUTURE_ENABLED, 31);
    assert_eq!(idx::FLOOR_Y, 32);
    assert_eq!(idx::FOCUS_X, 33);
}

#[test]
fn plan_values_stay_finite_and_bounded_through_two_shifts() {
    for reduced in [false, true] {
        let mut g = playing(LEVEL);
        let s = RenderSettings { reduced_motion: reduced, ..settings() };
        let mut plan = [0.0f32; PLAN_LEN];
        for _ in 0..600 {
            g.advance(InputState { right: true, ..Default::default() }, 1.0 / 60.0);
            let p = effective_params(&g, &s);
            build_plan(&g, &s, &p, &mut plan);
            assert!(plan.iter().all(|v| v.is_finite()), "{plan:?}");
            for i in [idx::WORLD_MIX, idx::GLITCH, idx::CRT, idx::WARMTH, idx::VIGNETTE, idx::BLOOM] {
                assert!((0.0..=1.0).contains(&plan[i]), "index {i} = {}", plan[i]);
            }
            assert!(plan[idx::FUTURE_W] >= 320.0 && plan[idx::FUTURE_H] >= 180.0);
        }
        assert!(!g.rifts[1].active, "walked through both rifts");
    }
}

#[test]
fn unsupported_future_renders_the_past_only() {
    let mut g = playing(LEVEL);
    let s = RenderSettings { future_supported: false, ..settings() };
    for _ in 0..120 {
        g.advance(InputState { right: true, ..Default::default() }, 1.0 / 60.0);
        let p = effective_params(&g, &s);
        assert_eq!((p.world_mix, p.glitch), (0.0, 0.0));
    }
    assert!(g.shift.is_shifting() || g.shift.world() == snes_wasm::world::shift::World::Future);
}

#[test]
fn crt_toggle_only_affects_crt() {
    let g = playing(LEVEL);
    let off = RenderSettings { crt_enabled: false, ..settings() };
    let mut plan = [0.0f32; PLAN_LEN];
    build_plan(&g, &off, &effective_params(&g, &off), &mut plan);
    assert_eq!(plan[idx::CRT], 0.0);
    build_plan(&g, &settings(), &effective_params(&g, &settings()), &mut plan);
    assert!(plan[idx::CRT] > 0.0);
}

#[test]
fn level_without_rifts_never_needs_a_3d_scene() {
    let mut g = playing(&["..P...........", "##############"]);
    let s = settings();
    let mut scene = Scene3d::default();
    for _ in 0..300 {
        g.advance(InputState { right: true, ..Default::default() }, 1.0 / 60.0);
        let p = effective_params(&g, &s);
        assert_eq!(p.world_mix, 0.0);
        // lib.rs only builds the scene when world_mix > 0; mirror that rule here.
        if p.world_mix > 0.0 {
            build_scene(&g, &p, &mut scene);
        }
    }
    assert_eq!(scene.block_count(), 0);
}
