pub struct Tuning {
    pub gravity: f32,
    pub jump_v: f32,
    pub accel_ground: f32,
    pub accel_air: f32,
    pub friction: f32,
    pub max_speed: f32,
    pub max_speed_dash: f32,
}

pub const TUNING: Tuning = Tuning {
    gravity: 1800.0,
    jump_v: -520.0,
    accel_ground: 4200.0,
    accel_air: 2600.0,
    friction: 5200.0,
    max_speed: 220.0,
    max_speed_dash: 320.0,
};
