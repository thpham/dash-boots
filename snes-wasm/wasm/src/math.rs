use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}

/// Axis-aligned box; `pos` is the top-left corner (y grows downward).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub pos: Vec2,
    pub size: Vec2,
}

impl Aabb {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { pos: Vec2::new(x, y), size: Vec2::new(w, h) }
    }

    pub fn overlaps(&self, o: &Aabb) -> bool {
        self.pos.x < o.pos.x + o.size.x
            && self.pos.x + self.size.x > o.pos.x
            && self.pos.y < o.pos.y + o.size.y
            && self.pos.y + self.size.y > o.pos.y
    }

    pub fn top(&self) -> f32 {
        self.pos.y
    }

    pub fn bottom(&self) -> f32 {
        self.pos.y + self.size.y
    }

    pub fn center_x(&self) -> f32 {
        self.pos.x + self.size.x * 0.5
    }
}

/// Move `v` toward `target` by at most `step` (never overshoots).
pub fn approach(v: f32, target: f32, step: f32) -> f32 {
    if v < target {
        (v + step).min(target)
    } else {
        (v - step).max(target)
    }
}
