use crate::world::player::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub pos: Vec2,
    pub size: Vec2,
}

impl Aabb {
    pub fn overlaps(&self, other: &Aabb) -> bool {
        let a_min_x = self.pos.x;
        let a_min_y = self.pos.y;
        let a_max_x = self.pos.x + self.size.x;
        let a_max_y = self.pos.y + self.size.y;

        let b_min_x = other.pos.x;
        let b_min_y = other.pos.y;
        let b_max_x = other.pos.x + other.size.x;
        let b_max_y = other.pos.y + other.size.y;

        a_min_x < b_max_x && a_max_x > b_min_x && a_min_y < b_max_y && a_max_y > b_min_y
    }

    pub fn bottom(&self) -> f32 {
        self.pos.y + self.size.y
    }

    pub fn top(&self) -> f32 {
        self.pos.y
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Coin {
    pub pos: Vec2,
}

impl Coin {
    pub const SIZE: Vec2 = Vec2 { x: 12.0, y: 12.0 };

    pub fn aabb(&self) -> Aabb {
        Aabb {
            pos: self.pos,
            size: Self::SIZE,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub pos: Vec2,
    pub vel_x: f32,
    pub patrol_min_x: f32,
    pub patrol_max_x: f32,
}

impl Enemy {
    pub const SIZE: Vec2 = Vec2 { x: 16.0, y: 16.0 };

    pub fn aabb(&self) -> Aabb {
        Aabb {
            pos: self.pos,
            size: Self::SIZE,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Goal {
    pub pos: Vec2,
}

impl Goal {
    pub const SIZE: Vec2 = Vec2 { x: 16.0, y: 24.0 };

    pub fn aabb(&self) -> Aabb {
        Aabb {
            pos: self.pos,
            size: Self::SIZE,
        }
    }
}

#[derive(Debug)]
pub struct Entities {
    pub coins: Vec<Coin>,
    pub enemies: Vec<Enemy>,
    pub goal: Goal,
    pub coin_count: u32,
    pub won: bool,
}

impl Entities {
    pub fn new_hardcoded() -> Self {
        Self {
            coins: vec![
                Coin {
                    pos: Vec2 { x: 80.0, y: 60.0 },
                },
                Coin {
                    pos: Vec2 { x: 120.0, y: 60.0 },
                },
                Coin {
                    pos: Vec2 { x: 160.0, y: 60.0 },
                },
            ],
            enemies: vec![Enemy {
                pos: Vec2 { x: 140.0, y: 120.0 },
                vel_x: 25.0,
                patrol_min_x: 120.0,
                patrol_max_x: 200.0,
            }],
            goal: Goal {
                pos: Vec2 { x: 240.0, y: 96.0 },
            },
            coin_count: 0,
            won: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new_hardcoded();
    }
}
