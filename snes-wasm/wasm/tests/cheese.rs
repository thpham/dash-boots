//! Cheese-wheel physics: slide, wall ricochets, settle and pit behavior —
//! the projectile side of the goat/cheese mechanic.

mod common;

use common::*;
use snes_wasm::math::Vec2;
use snes_wasm::tuning::DT;
use snes_wasm::world::cheese::{CheeseWheel, WheelState, MAX_WALL_BOUNCES, SLIDE_SPEED, WHEEL_SIZE};
use snes_wasm::world::level::Level;

// A flat open lane, one tile of headroom (the wheel fits, the player doesn't).
fn lane() -> Level {
    // Ceiling, 1-tile lane, floor; walls at both ends (the corridor tests
    // bounce between them).
    Level::from_data(&level(&["##########", "#P........", "##########", "##########", "##########"]))
}

fn place(x: f32) -> Vec2 {
    Vec2::new(x, 32.0 - WHEEL_SIZE.y)
}

#[test]
fn wheel_thrown_hits_a_wall_and_ricochets() {
    // Lane with a wall at the right end.
    let lvl = lane();
    let mut w = CheeseWheel::sliding_from(place(24.0), 1.0);
    for _ in 0..240 {
        if w.vel.x < 0.0 {
            break; // direction flipped: the ricochet happened
        }
        w.step(&lvl, DT);
    }
    assert_eq!(w.vel.x, -SLIDE_SPEED, "the wheel must have turned around");
    assert_eq!(w.bounces, 1);
    // The lane's right wall (col 9, x 144..160): the wheel ends flush against it.
    assert_eq!(w.pos.x, 160.0 - WHEEL_SIZE.x, "rests flush against the wall");
}

#[test]
fn sliding_wheel_rolls_back_out_of_a_corridor() {
    // 1-tile-high corridor, walls on both sides: it must settle there,
    // never wedge in a corner or tunnel through anything.
    let lvl = lane();
    let mut w = CheeseWheel::sliding_from(place(16.0), 1.0);
    let mut bursts = 0;
    for _ in 0..2_000 {
        if w.step(&lvl, DT) {
            bursts += 1;
        }
    }
    assert_eq!(bursts, 1, "exactly one settle burst");
    assert_eq!(w.state, WheelState::Floor, "settled after the bounces");
    assert!(w.pos.x > 8.0 && w.pos.x < 160.0 - WHEEL_SIZE.x, "still inside the corridor");
    assert!(w.pos.y < 32.0, "never sank into the floor");
}

#[test]
fn wheel_falling_into_a_pit_is_gone() {
    let lvl = Level::from_data(&level(&["##P#....####", "#...........", "#...........", "#..........."]));
    let mut w = CheeseWheel::sliding_from(place(48.0), 1.0);
    let mut below = false;
    for _ in 0..600 {
        w.step(&lvl, DT);
        if w.pos.y > lvl.height_px() + 32.0 {
            below = true;
            break;
        }
    }
    assert!(below, "the wheel must fall through the gap and pass height_px + 32");
}

#[test]
fn wheel_settles_to_floor_after_three_bounces() {
    let lvl = lane();
    let mut w = CheeseWheel::sliding_from(place(16.0), 1.0);
    let mut bursts = 0;
    while w.bounces < MAX_WALL_BOUNCES {
        w.step(&lvl, DT);
    }
    for _i in 0..600 {
        if w.step(&lvl, DT) {
            bursts += 1;
            assert_eq!(w.state, WheelState::Floor);
            assert_eq!(w.bounces, MAX_WALL_BOUNCES + 1, "the settle touch is the {MAX_WALL_BOUNCES}+1st");
            break;
        }
    }
    assert_eq!(bursts, 1, "step() signals the settle exactly once");
}

#[test]
fn floor_wheel_is_static() {
    let lvl = lane();
    let mut w = CheeseWheel::dropped_at(Vec2::new(60.0, 32.0));
    let start = w.pos;
    for _ in 0..120 {
        w.step(&lvl, DT);
    }
    assert_eq!(w.pos, start, "a settled wheel waits, motionless, to be picked up");
}
