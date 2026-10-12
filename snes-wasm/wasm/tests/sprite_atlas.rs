//! The swiss atlas slots: the wasm constants must mirror `tools/gen_art.py`.
//! (Atlas layout is the contract between the generator and the renderer.)

use snes_wasm::sprites::*;

fn f(x: f32, y: f32, w: f32, h: f32) -> Frame {
    Frame { x, y, w, h }
}

#[test]
fn swiss_frames_sit_at_their_documented_slots() {
    // y=64 row: goat walk/dazed, cheese wheels, goat
    assert_eq!(GOAT_WALK, [f(128.0, 64.0, 16.0, 16.0), f(144.0, 64.0, 16.0, 16.0)]);
    assert_eq!(GOAT_DAZED, f(160.0, 64.0, 16.0, 16.0));
    assert_eq!(CHEESE, [f(176.0, 64.0, 16.0, 16.0), f(192.0, 64.0, 16.0, 16.0)]);
    assert_eq!(CHEESE_ROLL, f(208.0, 64.0, 16.0, 16.0));
    // y=80 row: cows
    assert_eq!(COW_WALK, [f(128.0, 80.0, 16.0, 16.0), f(144.0, 80.0, 16.0, 16.0)]);
    assert_eq!(COW_BROWSE, f(160.0, 80.0, 16.0, 16.0));
}

#[test]
fn the_held_cheese_overlay_uses_a_floor_frame() {
    // The carry overlay draws CHEESE[0]; it must exist without panicking the
    // index math used by the renderer.
    assert_eq!(CHEESE.len(), 2);
    assert_eq!(CHEESE[0], f(176.0, 64.0, 16.0, 16.0));
}
