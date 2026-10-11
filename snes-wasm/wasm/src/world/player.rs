use crate::tuning::TUNING;

#[derive(Clone, Copy, Default, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub down: bool,
    pub jump: bool,
    pub dash: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub on_ground: bool,
    pub dash_boots: bool,
}

impl Player {
    pub fn new() -> Self {
        Self {
            pos: Vec2 { x: 40.0, y: 40.0 },
            vel: Vec2::default(),
            on_ground: false,
            dash_boots: true,
        }
    }

    pub fn step(&mut self, input: &InputState, dt: f32) {
        let dir = (input.right as i32 - input.left as i32) as f32;
        let accel = if self.on_ground {
            TUNING.accel_ground
        } else {
            TUNING.accel_air
        };
        self.vel.x += dir * accel * dt;

        let max = if self.dash_boots && input.dash {
            TUNING.max_speed_dash
        } else {
            TUNING.max_speed
        };
        self.vel.x = self.vel.x.clamp(-max, max);

        if self.on_ground && input.jump {
            self.vel.y = TUNING.jump_v;
            self.on_ground = false;
        }

        self.vel.y += TUNING.gravity * dt;
        self.pos.x += self.vel.x * dt;
        self.pos.y += self.vel.y * dt;
    }
}
