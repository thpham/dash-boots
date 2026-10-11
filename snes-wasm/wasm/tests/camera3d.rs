use snes_wasm::math::Vec2;
use snes_wasm::render::camera3d::*;

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
