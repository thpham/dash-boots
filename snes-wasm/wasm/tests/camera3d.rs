mod common;

use common::*;
use snes_wasm::game::{Game, Phase};
use snes_wasm::input::InputState;
use snes_wasm::math::Vec2;
use snes_wasm::render::camera3d::*;
use snes_wasm::world::shift::World;

const VW: f32 = 320.0;
const VH: f32 = 180.0;

#[test]
fn seam_hero_projects_onto_its_2d_pixel_at_pivot_zero() {
    for (cam, feet) in [
        (Vec2::new(0.0, 0.0), Vec2::new(48.0, 192.0)),
        (Vec2::new(812.0, 60.0), Vec2::new(950.0, 200.0)),
        (Vec2::new(1500.0, 37.0), Vec2::new(1530.0, 120.0)),
    ] {
        let c = pivot_camera(cam, VW, VH, feet, 0.0);
        for p2 in [feet, feet - Vec2::new(0.0, 11.0), Vec2::new(cam.x + 5.0, cam.y + 170.0)] {
            let got = project(&c.view_proj, to_world(p2, 0.0), VW, VH).unwrap();
            let want = p2 - cam;
            assert!((got.x - want.x).abs() <= 1.0 && (got.y - want.y).abs() <= 1.0, "{got:?} vs {want:?}");
        }
    }
}

#[test]
fn pivot_one_is_behind_and_above_the_hero_looking_ahead() {
    let feet = Vec2::new(1000.0, 192.0);
    let c = pivot_camera(Vec2::new(860.0, 60.0), VW, VH, feet, 1.0);
    let hero = to_world(feet, 0.0);
    assert!(c.eye.x < hero.x - 50.0, "behind");
    assert!(c.eye.y > hero.y, "above");
    assert!(c.target.x > hero.x, "looks ahead");
    // A point ahead on the road is on screen, roughly centered horizontally.
    let ahead = project(&c.view_proj, to_world(feet + Vec2::new(160.0, 0.0), 0.0), VW, VH).unwrap();
    assert!(ahead.x > 0.0 && ahead.x < VW && ahead.y > 0.0 && ahead.y < VH, "{ahead:?}");
}

#[test]
fn points_behind_the_camera_do_not_project() {
    let feet = Vec2::new(1000.0, 192.0);
    let c = pivot_camera(Vec2::new(860.0, 60.0), VW, VH, feet, 1.0);
    assert!(project(&c.view_proj, to_world(feet - Vec2::new(600.0, 0.0), 0.0), VW, VH).is_none());
}

#[test]
fn camera_moves_continuously_during_the_pivot() {
    let (cam, feet) = (Vec2::new(860.0, 60.0), Vec2::new(1000.0, 192.0));
    let mut prev = pivot_camera(cam, VW, VH, feet, 0.0).eye;
    for i in 1..=100 {
        let eye = pivot_camera(cam, VW, VH, feet, i as f32 / 100.0).eye;
        assert!(eye.distance(prev) < 15.0, "jump at {i}: {prev} -> {eye}");
        prev = eye;
    }
}

/// 15 rows like the real levels, ground on row 11, a rift then an enemy far ahead.
fn future_level() -> Vec<String> {
    let w = 60;
    let mut rows = vec![".".repeat(w); 15];
    let mut hero_row = ".".repeat(w);
    hero_row.replace_range(2..4, "PR");
    hero_row.replace_range(40..41, "E");
    rows[10] = hero_row;
    for r in &mut rows[11..] {
        *r = "#".repeat(w);
    }
    rows
}

fn future_eye(g: &Game) -> glam::Vec3 {
    pivot_camera(g.camera.pixel_pos(), VW, VH, g.player.feet(), 1.0).eye
}

fn settled_in_the_future(g: &mut Game) {
    let right = InputState { right: true, ..input() };
    run_until(g, right, 600, |g| g.shift.world() == World::Future && !g.shift.is_shifting())
        .expect("reaches the future");
}

#[test]
fn future_eye_stays_above_the_floor_while_dying() {
    let rows = future_level();
    let rows: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    let mut g = playing(&rows);
    settled_in_the_future(&mut g);
    let right = InputState { right: true, ..input() };
    run_until(&mut g, right, 600, |g| g.phase == Phase::Dying).expect("hit by the enemy");
    let floor_y = -g.level.height_px();
    let mut steps = 0;
    while g.phase == Phase::Dying {
        let eye = future_eye(&g);
        assert!(eye.y > floor_y, "eye {eye} under the floor {floor_y} after {steps} steps");
        run(&mut g, input(), 1);
        steps += 1;
    }
    assert!(steps > 30, "watched the whole death animation");
}

#[test]
fn future_eye_does_not_bob_on_a_jump() {
    let rows = future_level();
    let rows: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    let mut g = playing(&rows);
    settled_in_the_future(&mut g);
    let rest = future_eye(&g).y;
    let jump = InputState { jump: true, ..input() };
    let mut max_dy: f32 = 0.0;
    let mut min_feet = g.player.feet().y;
    for i in 0..60 {
        run(&mut g, if i < 15 { jump } else { input() }, 1);
        max_dy = max_dy.max((future_eye(&g).y - rest).abs());
        min_feet = min_feet.min(g.player.feet().y);
    }
    assert!(g.player.feet().y - min_feet > 30.0, "the hero really jumped");
    assert!(max_dy < 3.0, "eye moved {max_dy} px during a jump");
}
