use crate::input::Buttons;
use crate::math::{approach, Aabb, Vec2};
use crate::tuning::TUNING as T;
use crate::world::level::Level;
use crate::world::physics::{move_and_collide, on_ground, overlaps_solid};

pub const WIDTH: f32 = 10.0;
pub const HEIGHT_STAND: f32 = 22.0;
pub const HEIGHT_CROUCH: f32 = 14.0;

#[derive(Clone, Copy, Debug)]
pub struct Player {
    /// Top-left of the hitbox.
    pub pos: Vec2,
    pub vel: Vec2,
    pub on_ground: bool,
    /// 1.0 = facing right, -1.0 = facing left.
    pub facing: f32,
    pub crouching: bool,
    pub sliding: bool,
    pub dash_boots: bool,
    pub dash_timer: f32,
    pub dash_cooldown: f32,
    pub coyote: f32,
    pub jump_buffer: f32,
    /// True while rising from a jump with the button still held (low gravity).
    pub jumping: bool,
    pub invuln: f32,
    /// Distance walked, drives the run cycle.
    pub run_anim: f32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct PlayerEvents {
    pub jumped: bool,
    pub landed: bool,
    pub dashed: bool,
    pub bumped: bool,
    pub slide_started: bool,
}

impl Player {
    /// Spawn standing with feet at `feet` (x = center, y = bottom).
    pub fn spawn_at(feet: Vec2) -> Self {
        Self {
            pos: Vec2::new(feet.x - WIDTH * 0.5, feet.y - HEIGHT_STAND),
            vel: Vec2::default(),
            on_ground: false,
            facing: 1.0,
            crouching: false,
            sliding: false,
            dash_boots: false,
            dash_timer: 0.0,
            dash_cooldown: 0.0,
            coyote: 0.0,
            jump_buffer: 0.0,
            jumping: false,
            invuln: 0.0,
            run_anim: 0.0,
        }
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(WIDTH, if self.crouching { HEIGHT_CROUCH } else { HEIGHT_STAND })
    }

    pub fn aabb(&self) -> Aabb {
        let s = self.size();
        Aabb::new(self.pos.x, self.pos.y, s.x, s.y)
    }

    /// Feet point (center x, bottom y).
    pub fn feet(&self) -> Vec2 {
        let s = self.size();
        Vec2::new(self.pos.x + s.x * 0.5, self.pos.y + s.y)
    }

    pub fn is_dashing(&self) -> bool {
        self.dash_timer > 0.0
    }

    fn can_stand(&self, level: &Level) -> bool {
        if !self.crouching {
            return true;
        }
        let y = self.pos.y + HEIGHT_CROUCH - HEIGHT_STAND;
        !overlaps_solid(level, self.pos.x, y, WIDTH, HEIGHT_STAND)
    }

    fn set_crouch(&mut self, crouch: bool) {
        if crouch == self.crouching {
            return;
        }
        // Keep the feet in place.
        let d = HEIGHT_STAND - HEIGHT_CROUCH;
        self.pos.y += if crouch { d } else { -d };
        self.crouching = crouch;
        if !crouch {
            self.sliding = false;
        }
    }

    /// Airborne and blocked by a wall whose top is just above the feet: pop
    /// up onto the ledge instead of sliding down into the pit.
    fn ledge_assist(&mut self, level: &Level, vx: f32) -> bool {
        const MAX_LIFT: i32 = 10;
        let size = self.size();
        let dx = vx.signum();
        if dx == 0.0 || !overlaps_solid(level, self.pos.x + dx, self.pos.y, size.x, size.y) {
            return false;
        }
        for k in 1..=MAX_LIFT {
            let y = self.pos.y - k as f32;
            if !overlaps_solid(level, self.pos.x + dx, y, size.x, size.y) {
                // Snap the feet onto the tile top.
                let feet = ((y + size.y) / level.tile_size).ceil() * level.tile_size;
                self.pos.y = feet - size.y;
                self.pos.x += dx;
                self.vel.y = self.vel.y.min(0.0);
                return true;
            }
        }
        false
    }

    /// Head clipped a ceiling corner by a few pixels while jumping: nudge
    /// sideways and keep the jump.
    fn corner_correct(&mut self, level: &Level) -> bool {
        const MAX_NUDGE: i32 = 4;
        let size = self.size();
        for k in 1..=MAX_NUDGE {
            for dir in [1.0f32, -1.0] {
                let x = self.pos.x + dir * k as f32;
                if !overlaps_solid(level, x, self.pos.y - 1.0, size.x, size.y) {
                    self.pos.x = x;
                    return true;
                }
            }
        }
        false
    }

    pub fn step(&mut self, b: &Buttons, level: &Level, dt: f32) -> PlayerEvents {
        let mut ev = PlayerEvents::default();
        let held = b.held;
        let dir = (held.right as i32 - held.left as i32) as f32;
        let was_on_ground = self.on_ground;

        // Timers.
        self.invuln = (self.invuln - dt).max(0.0);
        self.dash_cooldown = (self.dash_cooldown - dt).max(0.0);
        self.coyote = if self.on_ground { T.coyote_time } else { (self.coyote - dt).max(0.0) };
        self.jump_buffer = if b.jump_pressed { T.jump_buffer } else { (self.jump_buffer - dt).max(0.0) };

        // Crouch / slide.
        if self.on_ground && held.down && !self.crouching && !self.is_dashing() {
            self.set_crouch(true);
            self.sliding = self.vel.x.abs() >= T.slide_min_speed;
            ev.slide_started = self.sliding;
        }
        if self.crouching && !(held.down && self.on_ground) && self.can_stand(level) {
            self.set_crouch(false);
        }
        if self.sliding && self.vel.x.abs() < 20.0 {
            self.sliding = false;
        }

        // Dash burst (boots only; the same button runs when held).
        if self.dash_boots && b.dash_pressed && self.dash_cooldown <= 0.0 && !self.crouching {
            let d = if dir != 0.0 { dir } else { self.facing };
            self.facing = d;
            self.dash_timer = T.dash_time;
            self.dash_cooldown = T.dash_cooldown;
            self.vel.x = d * T.dash_speed;
            self.vel.y = 0.0;
            self.jumping = false;
            ev.dashed = true;
        }

        // Horizontal motion.
        if self.is_dashing() {
            self.dash_timer = (self.dash_timer - dt).max(0.0);
        } else {
            let max = match (held.dash, self.dash_boots) {
                (true, true) => T.max_speed_dash,
                (true, false) => T.max_speed_run,
                _ => T.max_speed,
            };
            let decel = if self.on_ground { T.friction } else { T.air_drag };
            if self.crouching {
                if self.sliding {
                    self.vel.x = approach(self.vel.x, 0.0, T.slide_friction * dt);
                } else if dir != 0.0 {
                    // Crawl (lets the player enter and cross low tunnels).
                    self.vel.x = approach(self.vel.x, dir * T.crawl_speed, T.accel_ground * dt);
                    self.facing = dir;
                } else {
                    self.vel.x = approach(self.vel.x, 0.0, T.friction * dt);
                }
            } else if dir != 0.0 {
                if self.vel.x * dir < max {
                    let accel = if !self.on_ground {
                        T.accel_air
                    } else if self.vel.x * dir < 0.0 {
                        T.turn_accel
                    } else {
                        T.accel_ground
                    };
                    self.vel.x += dir * accel * dt;
                    if self.vel.x * dir > max {
                        self.vel.x = dir * max;
                    }
                } else {
                    // Faster than allowed (after a dash or releasing run): ease down.
                    self.vel.x = approach(self.vel.x, dir * max, decel * dt);
                }
                self.facing = dir;
            } else {
                self.vel.x = approach(self.vel.x, 0.0, decel * dt);
            }
        }

        // Jump (buffered + coyote time). Jumping cancels a dash and keeps its speed.
        if self.jump_buffer > 0.0 && self.coyote > 0.0 && self.can_stand(level) {
            self.set_crouch(false);
            self.dash_timer = 0.0;
            self.vel.y = T.jump_v - self.vel.x.abs() * T.jump_speed_bonus;
            self.on_ground = false;
            self.coyote = 0.0;
            self.jump_buffer = 0.0;
            self.jumping = true;
            ev.jumped = true;
        }

        // Gravity (variable jump height: low gravity only while rising with jump held).
        if !self.is_dashing() {
            if self.jumping && !held.jump {
                self.jumping = false;
            }
            let g = if self.jumping && self.vel.y < 0.0 { T.gravity } else { T.gravity_fall };
            self.vel.y = (self.vel.y + g * dt).min(T.max_fall);
        }

        // Move + collide.
        let size = self.size();
        let vel_before = self.vel;
        let mut hits = move_and_collide(level, &mut self.pos, size, self.vel * dt);
        if (hits.left || hits.right) && !self.on_ground && self.ledge_assist(level, vel_before.x) {
            hits.left = false;
            hits.right = false;
        }
        if hits.up && vel_before.y < 0.0 && self.corner_correct(level) {
            hits.up = false;
        }
        if hits.left || hits.right {
            self.vel.x = 0.0;
            self.dash_timer = 0.0;
        }
        if hits.up {
            if self.vel.y < 0.0 {
                ev.bumped = true;
            }
            self.vel.y = 0.0;
            self.jumping = false;
        }
        if hits.down {
            self.vel.y = 0.0;
        }
        self.on_ground = self.vel.y >= 0.0 && on_ground(level, self.pos, size);
        if self.on_ground {
            self.jumping = false;
            if !was_on_ground {
                ev.landed = true;
            }
        }
        self.run_anim += self.vel.x.abs() * dt;
        ev
    }
}
