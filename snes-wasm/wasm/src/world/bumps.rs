//! Boxes bumped from below (Mario Bros): the box bounces up a few pixels
//! and settles back.

use crate::tuning::TUNING as T;

/// One bumped tile span: row `ty`, inclusive columns `tx0..=tx1`.
#[derive(Clone, Copy, Debug)]
pub struct Bump {
    pub ty: i32,
    pub tx0: i32,
    pub tx1: i32,
    /// Seconds left in the bounce.
    pub t: f32,
}

impl Bump {
    pub fn new(ty: i32, tx0: i32, tx1: i32) -> Self {
        Self { ty, tx0, tx1, t: T.bump_time }
    }

    /// How high the box is raised right now: rises to its peak, settles back.
    pub fn offset(&self) -> f32 {
        let p = 1.0 - (self.t / T.bump_time).clamp(0.0, 1.0);
        T.bump_dip * (std::f32::consts::PI * p).sin()
    }

    /// Does this bump already cover that tile span (same row, overlapping columns)?
    pub fn covers(&self, ty: i32, tx0: i32, tx1: i32) -> bool {
        self.ty == ty && self.tx0 <= tx1 && self.tx1 >= tx0
    }

    pub fn step(&mut self, dt: f32) {
        self.t -= dt;
    }
}
