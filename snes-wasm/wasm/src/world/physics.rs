use crate::math::Vec2;
use crate::world::level::Level;

const EPS: f32 = 0.001;
/// Max distance moved per sub-step. Smaller than half a tile, so even the
/// fastest entity (dash) cannot skip over a 1-tile-thick wall or floor.
const MAX_SUBSTEP: f32 = 4.0;

#[derive(Clone, Copy, Default, Debug)]
pub struct Hits {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

/// True if the box overlaps any solid tile (every tile it spans is tested, so
/// boxes taller/wider than a tile are handled).
pub fn overlaps_solid(level: &Level, x: f32, y: f32, w: f32, h: f32) -> bool {
    let ts = level.tile_size;
    let x0 = (x / ts).floor() as i32;
    let x1 = ((x + w - EPS) / ts).floor() as i32;
    let y0 = (y / ts).floor() as i32;
    let y1 = ((y + h - EPS) / ts).floor() as i32;
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            if level.is_solid_tile(tx, ty) {
                return true;
            }
        }
    }
    false
}

/// Move a box by `delta`, resolving X then Y in sub-steps, snapping flush to
/// the tile it hits. Returns which sides were blocked.
pub fn move_and_collide(level: &Level, pos: &mut Vec2, size: Vec2, delta: Vec2) -> Hits {
    let mut hits = Hits::default();

    // Already embedded (spawned in a wall, stood up under a ceiling...): don't
    // snap to a random edge, just move so the entity can get out.
    if overlaps_solid(level, pos.x, pos.y, size.x, size.y) {
        *pos = *pos + delta;
        return hits;
    }

    let ts = level.tile_size;
    let steps = (delta.x.abs().max(delta.y.abs()) / MAX_SUBSTEP).ceil().max(1.0) as i32;
    let step = delta * (1.0 / steps as f32);
    let (mut move_x, mut move_y) = (step.x != 0.0, step.y != 0.0);

    for _ in 0..steps {
        if move_x {
            pos.x += step.x;
            if overlaps_solid(level, pos.x, pos.y, size.x, size.y) {
                if step.x > 0.0 {
                    pos.x = ((pos.x + size.x - EPS) / ts).floor() * ts - size.x;
                    hits.right = true;
                } else {
                    pos.x = (pos.x / ts).floor() * ts + ts;
                    hits.left = true;
                }
                move_x = false;
            }
        }
        if move_y {
            pos.y += step.y;
            if overlaps_solid(level, pos.x, pos.y, size.x, size.y) {
                if step.y > 0.0 {
                    pos.y = ((pos.y + size.y - EPS) / ts).floor() * ts - size.y;
                    hits.down = true;
                } else {
                    pos.y = (pos.y / ts).floor() * ts + ts;
                    hits.up = true;
                }
                move_y = false;
            }
        }
        if !move_x && !move_y {
            break;
        }
    }
    hits
}

/// True if there is solid ground directly under the box (1px probe).
pub fn on_ground(level: &Level, pos: Vec2, size: Vec2) -> bool {
    overlaps_solid(level, pos.x, pos.y + size.y, size.x, 1.0)
}
