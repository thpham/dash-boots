mod common;

use common::*;
use snes_wasm::input::InputState;
use snes_wasm::sim::fixed_timestep::FixedTimestep;
use snes_wasm::tuning::TUNING;

const FLAT: &[&str] = &[
    "....................",
    "....................",
    "....................",
    "....................",
    "..P.................",
    "....................",
    "####################",
];

#[test]
fn player_lands_on_ground_and_stays() {
    let mut g = playing(FLAT);
    run(&mut g, input(), 120);
    assert!(g.player.on_ground);
    assert!((g.player.feet().y - 6.0 * TS).abs() < 0.01, "feet at {}", g.player.feet().y);
    run(&mut g, input(), 600);
    assert!((g.player.feet().y - 6.0 * TS).abs() < 0.01);
}

#[test]
fn friction_stops_the_player() {
    let mut g = playing(FLAT);
    let right = InputState { right: true, ..input() };
    run(&mut g, right, 60);
    assert!(g.player.vel.x > 90.0);
    run(&mut g, input(), 30);
    assert_eq!(g.player.vel.x, 0.0, "player should stop within 0.5s");
}

#[test]
fn run_is_faster_than_walk_and_boots_faster_still() {
    let mut g = playing(FLAT);
    run(&mut g, InputState { right: true, ..input() }, 60);
    let walk = g.player.vel.x;
    let mut g = playing(FLAT);
    run(&mut g, InputState { right: true, dash: true, ..input() }, 60);
    let runv = g.player.vel.x;
    let mut g = playing(FLAT);
    g.player.dash_boots = true;
    run(&mut g, InputState { right: true, dash: true, ..input() }, 60);
    let boots = g.player.vel.x;
    assert!(walk < runv && runv < boots, "walk {walk} run {runv} boots {boots}");
}

#[test]
fn wall_stops_horizontal_motion_flush() {
    let mut g = playing(&["..........", "..........", "..P.....#.", "..........", "##########"]);
    run(&mut g, InputState { right: true, dash: true, ..input() }, 120);
    let right_edge = g.player.pos.x + 10.0;
    assert!((right_edge - 8.0 * TS).abs() < 0.01, "right edge {right_edge}");
    assert_eq!(g.player.vel.x, 0.0);
}

#[test]
fn dash_does_not_tunnel_through_thin_wall() {
    let mut g = playing(&[
        "....................",
        "....................",
        "..P.........#.......",
        "............#.......",
        "####################",
    ]);
    g.player.dash_boots = true;
    run(&mut g, input(), 30);
    // Repeated dashes toward the wall.
    for _ in 0..10 {
        run(&mut g, InputState { right: true, dash: true, ..input() }, 20);
        run(&mut g, InputState { right: true, ..input() }, 20);
    }
    assert!(g.player.pos.x + 10.0 <= 12.0 * TS + 0.01, "tunneled to x={}", g.player.pos.x);
}

#[test]
fn falling_from_height_does_not_tunnel_through_floor() {
    let mut rows = vec!["....."; 60];
    rows[1] = "..P..";
    rows.push("#####");
    let mut g = playing(&rows);
    run(&mut g, input(), 600);
    assert!(g.player.on_ground);
    assert!((g.player.feet().y - 60.0 * TS).abs() < 0.01);
    assert!(g.player.vel.y <= TUNING.max_fall);
}

#[test]
fn ceiling_stops_jump() {
    let mut g =
        playing(&["..........", "..........", "..######..", "..........", "...P......", "##########"]);
    run(&mut g, input(), 30);
    let jump = InputState { jump: true, ..input() };
    let mut min_top = f32::MAX;
    for _ in 0..40 {
        run(&mut g, jump, 1);
        min_top = min_top.min(g.player.pos.y);
    }
    assert!((min_top - 3.0 * TS).abs() < 0.01, "head reached {min_top}");
}

fn jump_height(hold_frames: u32) -> f32 {
    let mut g = playing(&[
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "...P......",
        "##########",
    ]);
    run(&mut g, input(), 30);
    let ground = g.player.feet().y;
    let mut best = 0.0f32;
    for i in 0..90 {
        let held = i < hold_frames;
        run(&mut g, InputState { jump: held, ..input() }, 1);
        best = best.max(ground - g.player.feet().y);
    }
    best
}

#[test]
fn variable_jump_height() {
    let full = jump_height(60);
    let tap = jump_height(1);
    assert!(full > 4.0 * TS && full < 4.7 * TS, "full jump {full}");
    assert!(tap > 1.8 * TS && tap < 2.6 * TS, "tap jump {tap}");
}

#[test]
fn coyote_time_allows_late_jump() {
    let mut g = playing(&[
        "............",
        "............",
        "............",
        "..P.........",
        "#####.......",
        "............",
    ]);
    run(&mut g, input(), 30);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 200, |g| !g.player.on_ground).expect("walks off ledge");
    run(&mut g, right, 3);
    assert!(!g.player.on_ground);
    run(&mut g, InputState { right: true, jump: true, ..input() }, 1);
    assert!(g.player.vel.y < -300.0, "coyote jump should trigger, vy={}", g.player.vel.y);
}

#[test]
fn jump_buffer_triggers_on_landing() {
    let mut g =
        playing(&["..P.......", "..........", "..........", "..........", "..........", "##########"]);
    // Fall until ~4 frames from the ground, then press jump once (and keep holding).
    run_until(&mut g, input(), 200, |g| g.player.feet().y > 5.0 * TS - 20.0).unwrap();
    assert!(!g.player.on_ground);
    let jump = InputState { jump: true, ..input() };
    let landed_and_jumped = run_until(&mut g, jump, 20, |g| g.player.vel.y < -300.0);
    assert!(landed_and_jumped.is_some(), "buffered jump should fire on landing");
}

#[test]
fn slide_goes_through_low_tunnel_and_stays_crouched_inside() {
    let mut g = playing(&[
        "........................",
        "........................",
        "...........######.......",
        "..P.....................",
        "########################",
    ]);
    run(&mut g, input(), 30);
    run(&mut g, InputState { right: true, dash: true, ..input() }, 45);
    run(&mut g, InputState { right: true, dash: true, down: true, ..input() }, 1);
    assert!(g.player.crouching && g.player.sliding);
    let slide = InputState { right: true, down: true, ..input() };
    run_until(&mut g, slide, 200, |g| g.player.pos.x > 12.0 * TS).expect("slides/crawls into the tunnel");
    // Release down inside the tunnel: cannot stand up, keeps crawling with right held.
    run(&mut g, InputState { right: true, ..input() }, 2);
    assert!(g.player.crouching, "no headroom to stand");
    let ok = run_until(&mut g, InputState { right: true, ..input() }, 400, |g| g.player.pos.x > 17.0 * TS);
    assert!(ok.is_some(), "should get through the tunnel, x={}", g.player.pos.x);
    run(&mut g, input(), 5);
    assert!(!g.player.crouching, "stands up after the tunnel");
}

#[test]
fn fixed_timestep_clamps_long_frames() {
    let mut ts = FixedTimestep::new(60.0);
    ts.push_frame_dt(5.0);
    let mut n = 0;
    while ts.pop_step() {
        n += 1;
    }
    assert!(n <= 6, "a 5s stall must not run {n} steps");
    ts.push_frame_dt(f32::NAN);
    assert!(!ts.pop_step());
}

#[test]
fn fixed_timestep_is_display_rate_independent() {
    for hz in [30.0f32, 60.0, 75.0, 144.0, 240.0] {
        let mut g = playing(FLAT);
        let mut steps = 0;
        for _ in 0..(hz as u32 * 2) {
            steps += g.advance(input(), 1.0 / hz);
        }
        assert!((119..=121).contains(&steps), "{hz} Hz ran {steps} steps in 2s");
    }
}

#[test]
fn camera_stays_inside_level() {
    let mut g = playing(&[
        "..............................",
        "..............................",
        "..............................",
        "..............................",
        "..............................",
        "..............................",
        "..............................",
        "..............................",
        "..............................",
        "..............................",
        "..............................",
        "..P...........................",
        "..............................",
        "##############################",
    ]);
    for dir in [false, true] {
        let inp = InputState { right: dir, left: !dir, dash: true, ..input() };
        for _ in 0..300 {
            run(&mut g, inp, 1);
            let c = g.camera;
            assert!(c.x >= 0.0 && c.x <= 30.0 * TS - 320.0 + 0.01, "cam x {}", c.x);
            assert!(c.y >= 0.0 && c.y <= 14.0 * TS - 180.0 + 0.01, "cam y {}", c.y);
        }
    }
}
