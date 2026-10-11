//! Gameplay constants. Units: pixels and seconds, y grows downward.
//!
//! Reference numbers (see tests/physics.rs):
//! - full jump from standstill ≈ 4.3 tiles high, tap jump ≈ 2.3 tiles
//! - running jump clears ≈ 6 tiles, boots-running jump ≈ 8 tiles

pub struct Tuning {
    /// Gravity while rising with the jump button held (variable jump height).
    pub gravity: f32,
    /// Gravity when falling or after releasing jump.
    pub gravity_fall: f32,
    /// Terminal velocity; keeps per-step motion well under one tile.
    pub max_fall: f32,
    /// Initial jump velocity (negative = up).
    pub jump_v: f32,
    /// Extra jump velocity per px/s of horizontal speed.
    pub jump_speed_bonus: f32,
    pub accel_ground: f32,
    pub accel_air: f32,
    /// Acceleration when reversing direction on the ground (skid).
    pub turn_accel: f32,
    pub friction: f32,
    pub air_drag: f32,
    /// Walk speed.
    pub max_speed: f32,
    /// Run speed (B held).
    pub max_speed_run: f32,
    /// Run speed with dash boots.
    pub max_speed_dash: f32,
    /// Horizontal speed of the dash burst (boots only).
    pub dash_speed: f32,
    pub dash_time: f32,
    pub dash_cooldown: f32,
    /// Minimum horizontal speed to start a slide when crouching.
    pub slide_min_speed: f32,
    pub slide_friction: f32,
    /// Speed when crawling under a low ceiling.
    pub crawl_speed: f32,
    pub coyote_time: f32,
    pub jump_buffer: f32,
    pub stomp_bounce: f32,
    pub stomp_bounce_held: f32,
    pub invuln_time: f32,
}

pub const TUNING: Tuning = Tuning {
    gravity: 1400.0,
    gravity_fall: 2600.0,
    max_fall: 400.0,
    jump_v: -440.0,
    jump_speed_bonus: 0.18,
    accel_ground: 1000.0,
    accel_air: 750.0,
    turn_accel: 2200.0,
    friction: 1200.0,
    air_drag: 150.0,
    max_speed: 100.0,
    max_speed_run: 165.0,
    max_speed_dash: 215.0,
    dash_speed: 320.0,
    dash_time: 0.18,
    dash_cooldown: 0.5,
    slide_min_speed: 70.0,
    slide_friction: 260.0,
    crawl_speed: 40.0,
    coyote_time: 0.09,
    jump_buffer: 0.12,
    stomp_bounce: -280.0,
    stomp_bounce_held: -440.0,
    invuln_time: 1.6,
};

/// Simulation step (fixed 60 Hz).
pub const DT: f32 = 1.0 / 60.0;
