//! Static pickups and markers built from the level data.

use crate::math::{Aabb, Vec2};
use crate::world::shift::World;

#[derive(Clone, Copy, Debug)]
pub struct Coin {
    /// Top-left of the coin's tile.
    pub tile_pos: Vec2,
    pub collected: bool,
}

impl Coin {
    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.tile_pos.x + 3.0, self.tile_pos.y + 2.0, 10.0, 12.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pickup {
    pub feet: Vec2,
    pub taken: bool,
}

impl Pickup {
    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.feet.x - 7.0, self.feet.y - 14.0, 14.0, 14.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Checkpoint {
    pub feet: Vec2,
    pub active: bool,
    /// World the checkpoint was activated in.
    pub world: World,
}

impl Checkpoint {
    /// Tall trigger: jumping over the post still activates it.
    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.feet.x - 8.0, self.feet.y - 160.0, 16.0, 160.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Goal {
    pub feet: Vec2,
}

impl Goal {
    pub fn aabb(&self) -> Aabb {
        // Tall trigger so the goal can't be jumped over.
        Aabb::new(self.feet.x - 6.0, self.feet.y - 160.0, 12.0, 160.0)
    }
}

/// Past ↔ future trigger. Tall like checkpoints so it cannot be jumped over.
#[derive(Clone, Copy, Debug)]
pub struct Rift {
    pub feet: Vec2,
    pub active: bool,
}

impl Rift {
    pub fn aabb(&self) -> Aabb {
        Aabb::new(self.feet.x - 6.0, self.feet.y - 160.0, 12.0, 160.0)
    }
}
