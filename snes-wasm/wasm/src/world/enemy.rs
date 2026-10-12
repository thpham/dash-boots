use crate::math::{Aabb, Vec2};
use crate::tuning::TUNING as T;
use crate::world::level::Level;
use crate::world::physics::{move_and_collide, on_ground};

pub const SIZE: Vec2 = Vec2::new(14.0, 14.0);
pub const SPEED: f32 = 32.0;
const SQUASH_TIME: f32 = 0.5;
/// Goats trot for this long, then browse for the rest of their cycle.
const GOAT_TROT: f32 = 2.0;
const GOAT_CYCLE: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnemyState {
    /// Waiting off-screen; wakes up when the camera gets close.
    Dormant,
    Walking,
    /// Bumped from below (head bump on its tile): hops, lands on its back,
    /// kicks away on touch, then gets up again.
    Flipped(f32),
    /// Stomped: flat sprite for a moment, then gone.
    Squashed(f32),
    /// Hit by a dash/slide/kick: flips and falls off the screen.
    Knocked,
    Dead,
}

/// Alpine folk: every spawn is a marmot; goat spawns (Tiled `goat_spawn`)
/// carry the cheese wheel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Marmot,
    Goat,
}

/// "Grumble": walks, falls with gravity, turns around at walls and ledges.
#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub pos: Vec2,
    pub vel: Vec2,
    pub dir: f32,
    pub state: EnemyState,
    pub kind: Kind,
    pub on_ground: bool,
    pub anim: f32,
    /// Seconds since the goat woke, drives its trot/browse cycle.
    pub cycle: f32,
}

impl Enemy {
    pub fn spawn_at(feet: Vec2, kind: Kind) -> Self {
        Self {
            pos: Vec2::new(feet.x - SIZE.x * 0.5, feet.y - SIZE.y),
            vel: Vec2::default(),
            dir: -1.0,
            state: EnemyState::Dormant,
            kind,
            on_ground: false,
            anim: 0.0,
            cycle: 0.0,
        }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.pos.x, self.pos.y, SIZE.x, SIZE.y)
    }

    /// Can hurt or be stomped by the player.
    pub fn is_dangerous(&self) -> bool {
        self.state == EnemyState::Walking
    }

    pub fn squash(&mut self) {
        self.state = EnemyState::Squashed(SQUASH_TIME);
    }

    pub fn knock(&mut self, from_dir: f32) {
        self.state = EnemyState::Knocked;
        self.vel = Vec2::new(from_dir * 90.0, -260.0);
    }

    /// Forced hop and fall onto its back after the player bumped its tile.
    pub fn flip(&mut self) {
        self.state = EnemyState::Flipped(T.flip_time);
        self.vel = Vec2::new(0.0, T.flip_hop_v);
    }

    pub fn step(&mut self, level: &Level, dt: f32, wake_x: f32) {
        match self.state {
            EnemyState::Dormant => {
                if self.pos.x < wake_x {
                    self.state = EnemyState::Walking;
                }
            }
            EnemyState::Walking => {
                self.anim += dt;
                // Goats trot, then stop to browse at the bottom of the cycle.
                if self.kind == Kind::Goat {
                    self.cycle = (self.cycle + dt) % GOAT_CYCLE;
                }
                let browsing = self.kind == Kind::Goat && self.cycle > GOAT_TROT;
                // Turn around at ledges (only when grounded, so they still fall
                // if spawned mid-air).
                if self.on_ground {
                    let ahead_x = if self.dir > 0.0 { self.pos.x + SIZE.x + 1.0 } else { self.pos.x - 1.0 };
                    if !level.is_solid_at_pixel(ahead_x, self.pos.y + SIZE.y + 1.0) {
                        self.dir = -self.dir;
                    }
                }
                self.vel.x = if browsing { 0.0 } else { self.dir * SPEED };
                self.vel.y = (self.vel.y + T.gravity_fall * dt).min(T.max_fall);
                let hits = move_and_collide(level, &mut self.pos, SIZE, self.vel * dt);
                if hits.left || hits.right {
                    self.dir = -self.dir;
                }
                if hits.down || hits.up {
                    self.vel.y = 0.0;
                }
                self.on_ground = on_ground(level, self.pos, SIZE);
                if self.pos.y > level.height_px() + 32.0 {
                    self.state = EnemyState::Dead;
                }
            }
            EnemyState::Flipped(t) => {
                self.anim += dt;
                // Ballistic hop, no walking: gravity, bounce on the ground, done.
                self.vel.x = 0.0;
                self.vel.y = (self.vel.y + T.gravity_fall * 0.6 * dt).min(T.max_fall);
                let hits = move_and_collide(level, &mut self.pos, SIZE, self.vel * dt);
                if hits.down || hits.up {
                    self.vel.y = 0.0;
                }
                self.on_ground = on_ground(level, self.pos, SIZE);
                let t = t - dt;
                // Never recovers: after a moment on its back, it evaporates
                // (the game spawns the poof when it sees Dead here).
                self.state = if t <= 0.0 { EnemyState::Dead } else { EnemyState::Flipped(t) };
            }
            EnemyState::Squashed(t) => {
                let t = t - dt;
                self.state = if t <= 0.0 { EnemyState::Dead } else { EnemyState::Squashed(t) };
            }
            EnemyState::Knocked => {
                self.vel.y = (self.vel.y + T.gravity_fall * 0.6 * dt).min(T.max_fall);
                self.pos = self.pos + self.vel * dt;
                if self.pos.y > level.height_px() + 64.0 {
                    self.state = EnemyState::Dead;
                }
            }
            EnemyState::Dead => {}
        }
    }
}
