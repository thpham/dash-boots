use crate::{tuning::TUNING, world::level::Level};

#[derive(Clone, Copy, Default, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub down: bool,
    pub jump: bool,
    pub dash: bool,

    #[serde(default)]
    pub prev_jump: bool,
    #[serde(default)]
    pub prev_dash: bool,
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

    pub fn step(&mut self, input: &InputState, level: &Level, dt: f32) -> Vec<&'static str> {
        let mut sfx = Vec::new();
        let dir = (input.right as i32 - input.left as i32) as f32;
        let accel = if self.on_ground {
            TUNING.accel_ground
        } else {
            TUNING.accel_air
        };
        self.vel.x += dir * accel * dt;

        let max = if self.dash_boots && input.dash {
            if !input.prev_dash {
                sfx.push("dash");
            }
            TUNING.max_speed_dash
        } else {
            TUNING.max_speed
        };
        self.vel.x = self.vel.x.clamp(-max, max);

        if self.on_ground && input.jump && !input.prev_jump {
            self.vel.y = TUNING.jump_v;
            self.on_ground = false;
            sfx.push("jump");
        }

        self.vel.y += TUNING.gravity * dt;

        // Move + resolve collisions (X then Y).
        let size = Vec2 { x: 16.0, y: 16.0 };

        // X axis.
        self.pos.x += self.vel.x * dt;
        if self.vel.x > 0.0 {
            let right = self.pos.x + size.x;
            let top = self.pos.y;
            let bottom = self.pos.y + size.y - 0.001;
            if level.is_solid_at_pixel(right, top) || level.is_solid_at_pixel(right, bottom) {
                let tile_x = (right / level.tile_size).floor();
                self.pos.x = tile_x * level.tile_size - size.x;
                self.vel.x = 0.0;
            }
        } else if self.vel.x < 0.0 {
            let left = self.pos.x;
            let top = self.pos.y;
            let bottom = self.pos.y + size.y - 0.001;
            if level.is_solid_at_pixel(left, top) || level.is_solid_at_pixel(left, bottom) {
                let tile_x = (left / level.tile_size).floor();
                self.pos.x = (tile_x + 1.0) * level.tile_size;
                self.vel.x = 0.0;
            }
        }

        // Y axis.
        self.pos.y += self.vel.y * dt;
        self.on_ground = false;
        if self.vel.y > 0.0 {
            let bottom = self.pos.y + size.y;
            let left = self.pos.x;
            let right = self.pos.x + size.x - 0.001;
            if level.is_solid_at_pixel(left, bottom) || level.is_solid_at_pixel(right, bottom) {
                let tile_y = (bottom / level.tile_size).floor();
                self.pos.y = tile_y * level.tile_size - size.y;
                self.vel.y = 0.0;
                self.on_ground = true;
            }
        } else if self.vel.y < 0.0 {
            let top = self.pos.y;
            let left = self.pos.x;
            let right = self.pos.x + size.x - 0.001;
            if level.is_solid_at_pixel(left, top) || level.is_solid_at_pixel(right, top) {
                let tile_y = (top / level.tile_size).floor();
                self.pos.y = (tile_y + 1.0) * level.tile_size;
                self.vel.y = 0.0;
            }
        }

        sfx
    }
}
