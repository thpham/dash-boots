mod common;

use common::*;
use snes_wasm::audio::AudioEvent;
use snes_wasm::game::{Game, MenuItem, Phase, RunHud};
use snes_wasm::input::InputState;

fn title() -> Game {
    Game::new(level(&["..P..", "#####"]), 320.0, 180.0)
}

fn tap(g: &mut Game, i: InputState) {
    run(g, i, 1);
    run(g, input(), 2);
}

#[test]
fn campaign_is_the_default_and_starts_as_before() {
    let mut g = title();
    assert_eq!(g.menu.selected, MenuItem::Campaign);
    tap(&mut g, InputState { start: true, ..input() });
    assert_eq!(g.phase, Phase::Playing);
}

#[test]
fn endless_with_a_difficulty_asks_the_host_once_per_press() {
    let mut g = title();
    tap(&mut g, InputState { down: true, ..input() });
    assert_eq!(g.menu.selected, MenuItem::Endless);
    tap(&mut g, InputState { right: true, ..input() });
    tap(&mut g, InputState { right: true, ..input() });
    assert_eq!(g.menu.difficulty, 3);
    for _ in 0..6 {
        tap(&mut g, InputState { left: true, ..input() });
    }
    assert_eq!(g.menu.difficulty, 1, "clamped");
    g.drain_audio();
    // Enter held across many steps: one press edge, one event.
    run(&mut g, InputState { start: true, ..input() }, 30);
    let ev = g.drain_audio();
    assert_eq!(ev.iter().filter(|e| **e == AudioEvent::StartEndless).count(), 1);
    assert_eq!(g.phase, Phase::Title, "the host swaps the level");
}

#[test]
fn continue_appears_only_with_a_saved_run() {
    let mut g = title();
    assert_eq!(g.menu.items(), [MenuItem::Campaign, MenuItem::Endless]);
    g.menu.saved_run = Some(6);
    assert_eq!(g.menu.items(), [MenuItem::Campaign, MenuItem::Endless, MenuItem::Continue]);
    tap(&mut g, InputState { up: true, ..input() });
    assert_eq!(g.menu.selected, MenuItem::Continue, "wraps around");
    g.drain_audio();
    tap(&mut g, InputState { jump: true, ..input() });
    assert!(g.drain_audio().contains(&AudioEvent::ContinueRun));
}

#[test]
fn restart_keeps_the_run_hud_and_menu() {
    let mut g = title();
    g.menu.difficulty = 4;
    g.run_hud =
        Some(RunHud { n: 3, difficulty: 4, seed: 9, start: 2, best: 5, next: "NEXT: D5 CAVE".into() });
    g.restart();
    assert_eq!(g.run_hud.as_ref().map(|h| h.n), Some(3));
    assert_eq!(g.menu.difficulty, 4);
}

#[test]
fn a_button_held_across_a_level_swap_is_not_a_new_press() {
    // The host builds a fresh Game for every endless level while the player
    // may still hold Enter/Space from the title or clear screen.
    let mut g = title();
    g.hold_buttons(InputState { start: true, jump: true, ..input() });
    g.restart();
    run(&mut g, InputState { start: true, jump: true, ..input() }, 10);
    assert!(!g.paused, "a held Enter must not pause the new level");
    assert!(!g.player.jumping, "a held Space must not jump at spawn");
    // Releasing then pressing again is a real press.
    run(&mut g, input(), 2);
    run(&mut g, InputState { start: true, ..input() }, 1);
    assert!(g.paused);
}
