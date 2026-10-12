//! Decor cows: slow alpine livestock. Purely decorative — the player is a
//! non-event to them (and vice versa), except the one greeting: when the
//! player comes close, the cow answers with a moo (the caller's job: `step`
//! returns true when the sound should fire).

use crate::math::Vec2;
use crate::tuning::TUNING as T;
use crate::world::level::Level;
use crate::world::physics::{move_and_collide, on_ground};

pub const COW_SPEED: f32 = 12.0;
pub const COW_SIZE: Vec2 = Vec2::new(14.0, 12.0);
const WALK_TIME: f32 = 2.0;
const BROWSE_TIME: f32 = 3.0;
const CYCLE: f32 = WALK_TIME + BROWSE_TIME;
/// "Passing by": the horizontal closeness that earns a greeting — 5 tiles.
pub const MOO_RADIUS: f32 = 80.0;
/// And the band of height the greeting is for (a cow far below stays silent).
const MOO_HEIGHT: f32 = 48.0;
/// Cows are not wind-up toys: one greeting per `MOO_GAP` per cow, only on
/// entering the radius (staying nearby never earns a second one).
pub const MOO_GAP: f32 = 8.0;

pub struct Cow {
    pub pos: Vec2,
    pub dir: f32,
    pub anim: f32,
    /// Seconds into the walk/browse cycle; the spawn spot sets the phase so
    /// a herd on an alpage never browses in unison.
    cycle: f32,
    moo_timer: f32,
    was_near: bool,
    on_ground: bool,
}

impl Cow {
    pub fn spawn_at(feet: Vec2) -> Self {
        Self {
            pos: Vec2::new(feet.x - COW_SIZE.x * 0.5, feet.y - COW_SIZE.y),
            dir: -1.0,
            anim: 0.0,
            cycle: feet.x % CYCLE,
            moo_timer: 0.0,
            was_near: false,
            on_ground: false,
        }
    }

    /// Mid-cycle pause: the head is down in the grass (the renderer's pose).
    pub fn browsing(&self) -> bool {
        self.cycle >= WALK_TIME
    }

    pub fn aabb(&self) -> crate::math::Aabb {
        crate::math::Aabb::new(self.pos.x, self.pos.y, COW_SIZE.x, COW_SIZE.y)
    }

    /// One fixed step. Turns at walls and ledges like the enemy walk code;
    /// returns true when the player has just come close enough for a moo —
    /// at most once per `MOO_GAP`, and only entering the closeness band.
    pub fn step(&mut self, level: &Level, player_feet: Vec2, dt: f32) -> bool {
        self.anim += dt;
        self.moo_timer -= dt;
        self.cycle = (self.cycle + dt) % CYCLE;
        let browsing = self.cycle >= WALK_TIME;

        if self.on_ground {
            let ahead_x = if self.dir > 0.0 { self.pos.x + COW_SIZE.x + 1.0 } else { self.pos.x - 1.0 };
            if !level.is_solid_at_pixel(ahead_x, self.pos.y + COW_SIZE.y + 1.0) {
                self.dir = -self.dir;
            }
        }
        let vx = if browsing { 0.0 } else { self.dir * COW_SPEED };
        let vel = Vec2::new(vx, (0.0 + T.gravity_fall * dt).min(T.max_fall));

        let hits = move_and_collide(level, &mut self.pos, COW_SIZE, vel * dt);
        if hits.left || hits.right {
            self.dir = -self.dir;
        }
        self.on_ground = on_ground(level, self.pos, COW_SIZE);

        // The greeting: player feet vs the cow's center, per axis, so a cow
        // on a lower terrace does not answer someone walking the ridge above.
        let center = self.pos + COW_SIZE * 0.5;
        let near =
            (player_feet.x - center.x).abs() < MOO_RADIUS && (player_feet.y - center.y).abs() < MOO_HEIGHT;
        let says_hello = near && !self.was_near && self.moo_timer <= 0.0;
        self.moo_timer = if says_hello { MOO_GAP } else { self.moo_timer };
        self.was_near = near;
        says_hello
    }
}
