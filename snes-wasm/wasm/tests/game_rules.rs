mod common;

use common::*;
use snes_wasm::audio::{AudioEvent, Sfx};
use snes_wasm::game::{Game, Phase};
use snes_wasm::input::InputState;
use snes_wasm::math::Vec2;
use snes_wasm::world::enemy::{self, Enemy, EnemyState};
use snes_wasm::world::particles::ParticleKind;
use snes_wasm::world::shift::{ShiftState, World};

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

const RIFT_LEVEL: &[&str] =
    &["..............................", "..PR......C....R..............", "#########################..###"];

#[test]
fn touching_a_rift_shifts_to_the_future_once() {
    let mut g = playing(RIFT_LEVEL);
    g.drain_audio();
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 60, |g| g.shift.is_shifting()).expect("rift triggers");
    assert_eq!(g.shift.world(), World::Future);
    assert!(!g.rifts[0].active, "single use");
    let audio = g.drain_audio();
    assert!(audio.contains(&AudioEvent::Sfx(Sfx::Rift)));
    assert!(audio.contains(&AudioEvent::MusicFuture(true)));
}

#[test]
fn rift_touched_while_shifting_is_ignored_and_stays_active() {
    let mut g = playing(&["..............", "..PRR.........", "##############"]);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 60, |g| g.shift.is_shifting()).unwrap();
    run_until(&mut g, right, 60, |g| g.player.pos.x > 4.0 * TS + 8.0).unwrap();
    assert!(g.rifts[1].active);
    assert!(matches!(g.shift.state, ShiftState::ToFuture(_)));
}

#[test]
fn second_rift_brings_back_the_past() {
    let mut g = playing(&[
        "..................................",
        "..PR.....................R........",
        "##################################",
    ]);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 600, |g| !g.rifts[1].active).expect("second rift");
    assert_eq!(g.shift.world(), World::Past);
    assert!(g.drain_audio().contains(&AudioEvent::Sfx(Sfx::Return)));
}

#[test]
fn checkpoint_remembers_the_future_and_respawn_reactivates_later_rifts() {
    let mut g = playing(RIFT_LEVEL);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 900, |g| g.phase == Phase::Dying).expect("falls in the pit");
    assert_eq!(g.checkpoints[0].world, World::Future);
    run_until(&mut g, input(), 200, |g| g.phase == Phase::Playing).unwrap();
    assert_eq!(g.shift.state, ShiftState::Future, "respawn settled in the checkpoint's world");
    assert!(!g.rifts[0].active, "rift before the checkpoint stays used");
    assert!(g.rifts[1].active, "rift after the checkpoint is back");
}

#[test]
fn pausing_freezes_the_shift() {
    let mut g = playing(RIFT_LEVEL);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 60, |g| g.shift.is_shifting()).unwrap();
    run(&mut g, input(), 10);
    g.pause();
    let frozen = g.shift.state;
    run(&mut g, input(), 60);
    assert_eq!(g.shift.state, frozen);
    run(&mut g, InputState { start: true, ..input() }, 1);
    run(&mut g, input(), 5);
    assert_ne!(g.shift.state, frozen);
}

#[test]
fn dying_mid_shift_respawns_cleanly_in_the_past() {
    let mut g = playing(&["................", "..PR.....E......", "################"]);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 60, |g| g.shift.is_shifting()).unwrap();
    // Walk into the enemy while the camera is still pivoting.
    run_until(&mut g, right, 300, |g| g.phase == Phase::Dying).expect("hit by the enemy");
    run_until(&mut g, input(), 200, |g| g.phase == Phase::Playing).unwrap();
    assert_eq!(g.shift.state, ShiftState::Past);
    assert!(g.rifts[0].active, "rift after the spawn point is back");
}

#[test]
fn restart_after_visiting_the_future_is_back_in_the_past() {
    let mut g =
        playing(&["..........................", "..PR........G.............", "##########################"]);
    run_until(&mut g, InputState { right: true, ..input() }, 600, |g| g.phase == Phase::Won).unwrap();
    g.drain_audio();
    g.restart();
    assert!(g.drain_audio().contains(&AudioEvent::MusicFuture(false)), "host filter re-synced");
    assert_eq!(g.shift.state, ShiftState::Past);
    assert!(g.rifts.iter().all(|r| r.active));
    assert_eq!(g.shift.params(false).crt, snes_wasm::world::shift::CRT_BASE);
}

#[test]
fn animation_clock_stops_while_paused() {
    let mut g = playing(&["..P.o..", "#######"]);
    run(&mut g, input(), 5);
    g.pause();
    let frame = g.frame;
    run(&mut g, input(), 30);
    assert_eq!(g.frame, frame, "paused frames must not advance the animation clock");
    run(&mut g, InputState { start: true, ..input() }, 1);
    run(&mut g, input(), 3);
    assert!(g.frame > frame, "clock resumes after unpausing");
}

// Mario Bros: a box with enemies on top can be bumped from below.
const BUMP_LEVEL: &[&str] = &["....E.......", "######......", "............", "..P.........", "############"];

#[test]
fn bumping_the_box_flips_the_enemy_and_the_box_bounces() {
    let mut g = playing(BUMP_LEVEL);
    run(&mut g, input(), 20);
    // Line the enemy up on the tile the player stands under (col 2).
    g.enemies[0].pos = Vec2::new(2.0 * TS + 1.0, TS - enemy::SIZE.y);
    g.enemies[0].dir = -1.0;
    g.player.pos.x = 35.0;
    let flipped = run_until(&mut g, InputState { jump: true, ..input() }, 60, |g| {
        matches!(g.enemies[0].state, EnemyState::Flipped(_))
    });
    assert!(flipped.is_some(), "enemy must be flipped by the head bump");
    let hop = g.enemies[0].vel.y;
    assert!(hop < -250.0, "it is launched high into the air (got {hop})");
    assert_eq!(g.phase, Phase::Playing);
    // The box itself bounces up and settles back.
    let dipped = run_until(&mut g, input(), 20, |g| g.bumps.iter().any(|b| b.offset() > 2.0));
    assert!(dipped.is_some(), "the box must dip under the bump");
    run(&mut g, input(), 30);
    assert!(g.bumps.is_empty(), "the box settles back");
}

#[test]
fn enemies_off_the_bumped_tile_are_untouched() {
    let mut g = playing(BUMP_LEVEL);
    run(&mut g, input(), 20);
    g.enemies[0].pos = Vec2::new(2.0 * TS + 1.0, TS - enemy::SIZE.y); // above the bump
    g.enemies[0].dir = -1.0;
    // A second patroller standing on the ground, far from the bump.
    g.enemies.push(Enemy::spawn_at(Vec2::new(10.0 * TS + 8.0, 4.0 * TS)));
    g.enemies[1].state = EnemyState::Walking;
    g.enemies[1].dir = 1.0;
    g.player.pos.x = 35.0;
    run(&mut g, InputState { jump: true, ..input() }, 40);
    assert!(matches!(g.enemies[0].state, EnemyState::Flipped(_)));
    assert!(matches!(g.enemies[1].state, EnemyState::Walking), "far enemy keeps walking");
}

#[test]
fn touching_a_flipped_enemy_kicks_it_out_safely() {
    let mut g = playing(BUMP_LEVEL);
    run(&mut g, input(), 20);
    g.enemies[0].pos = Vec2::new(2.0 * TS + 1.0, TS - enemy::SIZE.y);
    g.enemies[0].dir = -1.0;
    g.player.pos.x = 35.0;
    run_until(&mut g, InputState { jump: true, ..input() }, 60, |g| {
        g.enemies[0].on_ground && matches!(g.enemies[0].state, EnemyState::Flipped(_))
    })
    .expect("enemy flipped and back on its platform");
    // Touch it from the side on the platform: harmless, but it must be kicked out.
    let e = g.enemies[0];
    g.player.pos.x = e.pos.x - 6.0;
    g.player.pos.y = e.pos.y + e.aabb().size.y - snes_wasm::world::player::HEIGHT_STAND;
    let kicked = run_until(&mut g, input(), 30, |g| g.enemies[0].state == EnemyState::Knocked);
    assert!(kicked.is_some(), "touching a flipped enemy kicks it out");
    assert_eq!(g.phase, Phase::Playing, "a flipped enemy is harmless to the player");
}

#[test]
fn a_flipped_enemy_evaporates_after_a_moment() {
    let mut g = playing(BUMP_LEVEL);
    run(&mut g, input(), 20);
    g.enemies[0].pos = Vec2::new(2.0 * TS + 1.0, TS - enemy::SIZE.y);
    g.enemies[0].dir = -1.0;
    g.player.pos.x = 35.0;
    run_until(&mut g, InputState { jump: true, ..input() }, 60, |g| {
        matches!(g.enemies[0].state, EnemyState::Flipped(_))
    })
    .unwrap();
    // It lands on its back, waits a moment, then: poof!
    let on_back = run_until(&mut g, input(), 60, |g| g.enemies[0].on_ground);
    assert!(on_back.is_some(), "it lands back on the box");
    let gone = run_until(&mut g, input(), 120, |g| g.enemies[0].state == EnemyState::Dead);
    assert!(gone.is_some(), "the flipped enemy must evaporate, never recover");
    assert_eq!(g.enemies_alive(), 0);
    assert!(
        g.particles_list().iter().any(|p| p.kind == ParticleKind::Dust),
        "a puff of particles marks the evaporation"
    );
}
