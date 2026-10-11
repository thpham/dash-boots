mod common;

use common::*;
use snes_wasm::audio::{AudioEvent, Sfx};
use snes_wasm::game::{Game, Phase};
use snes_wasm::input::InputState;
use snes_wasm::world::enemy::EnemyState;

#[test]
fn title_screen_waits_for_start() {
    let mut g = Game::new(level(&["..P..", "#####"]), 320.0, 180.0);
    run(&mut g, input(), 60);
    assert_eq!(g.phase, Phase::Title);
    assert_eq!(g.time, 0.0);
    run(&mut g, InputState { start: true, ..input() }, 1);
    assert_eq!(g.phase, Phase::Playing);
    assert!(g.drain_audio().contains(&AudioEvent::PlayMusic(0)));
}

#[test]
fn pause_freezes_the_game() {
    let mut g = playing(&["..P..", "#####"]);
    run(&mut g, input(), 10);
    run(&mut g, InputState { start: true, ..input() }, 1);
    assert!(g.paused);
    let t = g.time;
    run(&mut g, input(), 60);
    assert_eq!(g.time, t);
    run(&mut g, InputState { start: true, ..input() }, 1);
    assert!(!g.paused);
}

#[test]
fn coins_are_collected_once() {
    let mut g = playing(&["............", "..P..o.o....", "############"]);
    assert_eq!(g.coins_total, 2);
    run(&mut g, InputState { right: true, ..input() }, 90);
    assert_eq!(g.coins_collected, 2);
    assert!(g.drain_audio().contains(&AudioEvent::Sfx(Sfx::Coin)));
}

const ENEMY_LEVEL: &[&str] = &[
    "....................",
    "....................",
    "....................",
    "..P.......E.........",
    "####################",
];

#[test]
fn stomping_squashes_enemy_and_bounces() {
    let mut g = playing(ENEMY_LEVEL);
    run(&mut g, input(), 20);
    // Place the player right above the enemy, falling.
    let e = g.enemies[0];
    g.player.pos.x = e.pos.x + 2.0;
    g.player.pos.y = e.pos.y - 30.0;
    g.player.vel.y = 100.0;
    let hit = run_until(&mut g, input(), 30, |g| g.enemies[0].state != EnemyState::Walking);
    assert!(hit.is_some());
    assert!(matches!(g.enemies[0].state, EnemyState::Squashed(_)));
    assert!(g.player.vel.y < 0.0, "player bounces");
    assert_eq!(g.phase, Phase::Playing);
}

#[test]
fn touching_enemy_from_the_side_kills_then_respawns() {
    let mut g = playing(ENEMY_LEVEL);
    let died = run_until(&mut g, InputState { right: true, ..input() }, 300, |g| g.phase == Phase::Dying);
    assert!(died.is_some());
    assert_eq!(g.deaths, 1);
    let back = run_until(&mut g, input(), 200, |g| g.phase == Phase::Playing);
    assert!(back.is_some());
    assert!((g.player.feet().x - (2.0 * TS + 8.0)).abs() < 0.01, "respawned at start");
    assert_eq!(g.enemies[0].state, EnemyState::Dormant, "enemies reset");
}

#[test]
fn dash_boots_absorb_one_hit() {
    let mut g = playing(&[
        "....................",
        "....................",
        "....................",
        "..PB......E.........",
        "####################",
    ]);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 60, |g| g.player.dash_boots).expect("picks up boots");
    let hit = run_until(&mut g, right, 300, |g| !g.player.dash_boots);
    assert!(hit.is_some());
    assert_eq!(g.phase, Phase::Playing, "boots absorbed the hit");
    assert!(g.player.invuln > 0.0);
}

#[test]
fn dashing_knocks_enemies_out() {
    let mut g = playing(ENEMY_LEVEL);
    g.player.dash_boots = true;
    run(&mut g, input(), 20);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 300, |g| g.enemies[0].pos.x - g.player.pos.x < 40.0).unwrap();
    let ok = run_until(&mut g, InputState { right: true, dash: true, ..input() }, 30, |g| {
        g.enemies[0].state == EnemyState::Knocked
    });
    assert!(ok.is_some());
    assert_eq!(g.phase, Phase::Playing);
}

#[test]
fn falling_in_a_pit_kills_and_respawns_at_checkpoint() {
    let mut g = playing(&[
        "....................",
        "....................",
        "..P....C............",
        "##########....######",
    ]);
    let right = InputState { right: true, ..input() };
    let died = run_until(&mut g, right, 400, |g| g.phase == Phase::Dying);
    assert!(died.is_some());
    assert!(g.checkpoints[0].active);
    run_until(&mut g, input(), 200, |g| g.phase == Phase::Playing).unwrap();
    assert!((g.player.feet().x - (7.0 * TS + 8.0)).abs() < 0.01, "respawn at checkpoint");
}

#[test]
fn reaching_goal_wins_and_enter_restarts() {
    let mut g = playing(&["............", "..P....G....", "############"]);
    let won = run_until(&mut g, InputState { right: true, ..input() }, 300, |g| g.phase == Phase::Won);
    assert!(won.is_some());
    assert!(g.drain_audio().contains(&AudioEvent::Sfx(Sfx::Clear)));
    let t = g.time;
    run(&mut g, input(), 30);
    assert_eq!(g.time, t, "timer stops on clear");
    // Input ignored for a moment, then Enter restarts.
    run(&mut g, InputState { start: true, ..input() }, 1);
    assert_eq!(g.phase, Phase::Won);
    run(&mut g, input(), 90);
    run(&mut g, InputState { start: true, ..input() }, 1);
    assert_eq!(g.phase, Phase::Playing);
    assert_eq!(g.time, 0.0);
}

#[test]
fn jump_emits_sfx_once_per_press() {
    let mut g = playing(&["..P..", "#####"]);
    run(&mut g, input(), 20);
    g.drain_audio();
    run(&mut g, InputState { jump: true, ..input() }, 60);
    let jumps = g.drain_audio().iter().filter(|e| **e == AudioEvent::Sfx(Sfx::Jump)).count();
    assert_eq!(jumps, 1, "holding jump must not re-trigger");
}

#[test]
fn quick_tap_between_steps_is_not_lost() {
    let mut g = playing(&["..P..", "#####"]);
    run(&mut g, input(), 20);
    // A frame too short to run a step sees the press; the next frame sees the release.
    assert_eq!(g.advance(InputState { jump: true, ..input() }, 0.001), 0);
    g.advance(input(), 1.0 / 60.0);
    assert!(g.player.vel.y < 0.0, "tap should still jump");
}

#[test]
fn host_pause_only_pauses_during_play() {
    let mut g = Game::new(level(&["..P..", "#####"]), 320.0, 180.0);
    g.pause();
    assert!(!g.paused, "title screen is not paused");
    g.start();
    g.pause();
    assert!(g.paused);
    run(&mut g, InputState { start: true, ..input() }, 1);
    assert!(!g.paused);
}

#[test]
fn restarting_with_enter_held_does_not_pause_the_new_game() {
    let mut g = playing(&["............", "..P....G....", "############"]);
    run_until(&mut g, InputState { right: true, ..input() }, 300, |g| g.phase == Phase::Won).unwrap();
    run(&mut g, input(), 90);
    // Press Enter and keep holding it across the restart.
    run(&mut g, InputState { start: true, ..input() }, 5);
    assert_eq!(g.phase, Phase::Playing);
    assert!(!g.paused, "held Enter must not count as a second press");
}
