//! Past ↔ future camera. At pivot 0 it reproduces the 2D side view exactly
//! (the seam); at pivot 1 it sits behind and above the hero (3/4 rear view).

use glam::{Mat4, Vec3};

use crate::math::Vec2;
use crate::world::shift::{FINAL_PITCH_DEG, FINAL_YAW_DEG};

pub const PAST_FOV_DEG: f32 = 30.0;
pub const FUTURE_FOV_DEG: f32 = 60.0;
pub const FUTURE_DISTANCE: f32 = 150.0;
pub const FUTURE_LOOK_AHEAD: f32 = 64.0;
pub const FUTURE_TARGET_UP: f32 = 24.0;
/// Feet placement in the 2D view; must match `world::camera::Camera::FEET_Y`.
pub const FEET_Y_FRACTION: f32 = crate::world::camera::Camera::FEET_Y;
pub const NEAR: f32 = 4.0;
pub const FAR: f32 = 2400.0;

/// Level pixels (x right, y down) → 3D world (x forward, y up, z lateral).
pub fn to_world(p: Vec2, z: f32) -> Vec3 {
    Vec3::new(p.x, -p.y, z)
}

#[derive(Clone, Copy, Debug)]
pub struct Camera3d {
    pub eye: Vec3,
    pub target: Vec3,
    pub view: Mat4,
    pub proj: Mat4,
    pub view_proj: Mat4,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// `cam2d` is the 2D camera's pixel position (top-left of the view).
pub fn pivot_camera(cam2d: Vec2, view_w: f32, view_h: f32, hero_feet: Vec2, pivot: f32) -> Camera3d {
    let u = pivot.clamp(0.0, 1.0);
    let fov = lerp(PAST_FOV_DEG, FUTURE_FOV_DEG, u).to_radians();
    // Distance at which the z = 0 plane fills exactly view_h pixels.
    let past_distance = (view_h * 0.5) / (PAST_FOV_DEG.to_radians() * 0.5).tan();
    let distance = lerp(past_distance, FUTURE_DISTANCE, u);

    let past_target = to_world(Vec2::new(cam2d.x + view_w * 0.5, cam2d.y + view_h * 0.5), 0.0);
    // The future target stands on the ground line of the (smoothed, death-frozen)
    // 2D view — the hero's feet would dive under the floor while dying and hop
    // on every jump, but the 2D camera does neither.
    let feet_y = cam2d.y + view_h * FEET_Y_FRACTION;
    let future_target = to_world(Vec2::new(hero_feet.x + FUTURE_LOOK_AHEAD, feet_y - FUTURE_TARGET_UP), 0.0);
    let target = past_target.lerp(future_target, u);

    let yaw = (FINAL_YAW_DEG * u).to_radians();
    let pitch = (FINAL_PITCH_DEG * u).to_radians();
    // Unit vector from target to eye: +Z (side view) rotating toward -X (behind), tilted up.
    let back = Vec3::new(-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos());
    let eye = target + back * distance;

    let view = Mat4::look_at_rh(eye, target, Vec3::Y);
    let proj = Mat4::perspective_rh_gl(fov, view_w / view_h, NEAR, FAR);
    Camera3d { eye, target, view, proj, view_proj: proj * view }
}

/// Project a world point to top-left pixel coordinates of a `view_w`×`view_h` image.
pub fn project(m: &Mat4, p: Vec3, view_w: f32, view_h: f32) -> Option<Vec2> {
    let c = *m * p.extend(1.0);
    if c.w <= 0.0 {
        return None;
    }
    let (x, y) = (c.x / c.w, c.y / c.w);
    Some(Vec2::new((x * 0.5 + 0.5) * view_w, (1.0 - (y * 0.5 + 0.5)) * view_h))
}
