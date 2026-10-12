//! The cheese wheel: the goat's drop, playable as a Mario shell — picked up,
//! thrown, slides at shell speed, ricochets off walls and crushes enemies.
//! Physics only lives here; pickup/throw/crush wiring is in `game.rs`.

use crate::math::Vec2;
use crate::tuning::TUNING as T;
use crate::world::level::Level;
use crate::world::physics::move_and_collide;

pub const WHEEL_SIZE: Vec2 = Vec2::new(12.0, 12.0);
pub const SLIDE_SPEED: f32 = 150.0;
/// Wall ricochets before the wheel decelerates and settles back into a
/// pickup (the settle burst is the caller's job — `step` returns true once).
pub const MAX_WALL_BOUNCES: u8 = 3;
/// Below this the wheel is gone (same rule as enemies).
const PIT_MARGIN: f32 = 32.0;
/// Most wheels in play at once: crush chains cannot flood the level.
pub const MAX_WHEELS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelState {
    /// Sitting on the ground, pickable. Never moves.
    Floor,
    /// Fired by the player: slides at SLIDE_SPEED, flips on walls.
    Slide { dir: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct CheeseWheel {
    pub pos: Vec2,
    pub vel: Vec2,
    pub state: WheelState,
    pub anim: f32,
    pub bounces: u8,
}

impl CheeseWheel {
    /// Dropped by a stomped goat, at the goat's place.
    pub fn dropped_at(feet: Vec2) -> Self {
        Self {
            pos: Vec2::new(feet.x - WHEEL_SIZE.x * 0.5, feet.y - WHEEL_SIZE.y),
            vel: Vec2::default(),
            state: WheelState::Floor,
            anim: 0.0,
            bounces: 0,
        }
    }

    /// Fired by the player from the front of their position (feet point).
    pub fn sliding_from(feet: Vec2, dir: f32) -> Self {
        Self {
            pos: Vec2::new(feet.x - WHEEL_SIZE.x * 0.5, feet.y - WHEEL_SIZE.y),
            vel: Vec2::new(dir * SLIDE_SPEED, 0.0),
            state: WheelState::Slide { dir },
            anim: 0.0,
            bounces: 0,
        }
    }

    pub fn aabb(&self) -> crate::math::Aabb {
        crate::math::Aabb::new(self.pos.x, self.pos.y, WHEEL_SIZE.x, WHEEL_SIZE.y)
    }

    /// Gone through a pit (the caller removes it).
    pub fn fallen_off(&self, height_px: f32) -> bool {
        self.pos.y > height_px + PIT_MARGIN
    }

    /// One fixed step. Returns true exactly once when the wheel settles to
    /// `Floor` (the caller plays the crust burst and keeps the wheel).
    pub fn step(&mut self, level: &Level, dt: f32) -> bool {
        self.anim += dt;
        match self.state {
            WheelState::Floor => false,
            WheelState::Slide { dir } => {
                let mut dir = dir;
                self.vel.x = dir * SLIDE_SPEED;
                self.vel.y = (self.vel.y + T.gravity_fall * dt).min(T.max_fall);
                let hits = move_and_collide(level, &mut self.pos, WHEEL_SIZE, self.vel * dt);
                if hits.down || hits.up {
                    self.vel.y = 0.0;
                }
                if hits.left || hits.right {
                    dir = -dir;
                    self.vel.x = dir * SLIDE_SPEED;
                    self.bounces += 1;
                    if self.bounces > MAX_WALL_BOUNCES {
                        // Out of bounces: it settles right here, back to a pickup.
                        self.state = WheelState::Floor;
                        self.vel = Vec2::default();
                        return true;
                    }
                    self.state = WheelState::Slide { dir };
                }
                false
            }
        }
    }
}
