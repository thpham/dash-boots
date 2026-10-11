//! 3D scene of the "reconstructed memory": instance buffers read by the host.

use super::camera3d::to_world;
use super::retro::{world_sprites, SpriteKind};
use super::{TEX_SPRITES, TEX_TILES};
use crate::game::Game;
use crate::math::Vec2;
use crate::sprites::TILE_SIZE;
use crate::world::shift::ShiftParams;

pub const BLOCK_FLOATS: usize = 10;
pub const BILLBOARD_FLOATS: usize = 12;
pub const SHADOW_FLOATS: usize = 5;
pub const FLAG_HOLO: u32 = 1;
pub const FLAG_FLIP_X: u32 = 2;
pub const FLAG_FLIP_Y: u32 = 4;
pub const VISIBLE_AHEAD_TILES: i32 = 40;
pub const VISIBLE_BEHIND_TILES: i32 = 8;
/// Half width (z) of extruded ground: a 5-tile wide road.
pub const ROAD_HALF_WIDTH: f32 = 40.0;
/// Decorations stand on the far side of the road.
pub const DECOR_Z: f32 = -28.0;
/// Wave radius in tiles when progress = 1, and the width of its soft edge.
const WAVE_RADIUS_TILES: f32 = 46.0;
const WAVE_EDGE_TILES: f32 = 4.0;

#[derive(Default, Debug)]
pub struct Scene3d {
    pub blocks: Vec<f32>,
    pub billboards: Vec<f32>,
    pub shadows: Vec<f32>,
}

impl Scene3d {
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.billboards.clear();
        self.shadows.clear();
    }
    pub fn block_count(&self) -> usize {
        self.blocks.len() / BLOCK_FLOATS
    }
    pub fn billboard_count(&self) -> usize {
        self.billboards.len() / BILLBOARD_FLOATS
    }
    pub fn shadow_count(&self) -> usize {
        self.shadows.len() / SHADOW_FLOATS
    }
}

pub fn block_extrusion(progress: f32, dist_tiles: f32) -> f32 {
    ((progress * WAVE_RADIUS_TILES - dist_tiles) / WAVE_EDGE_TILES).clamp(0.0, 1.0)
}

fn is_ground_tile(t: u16) -> bool {
    matches!(t, 1..=6 | 21 | 22)
}

fn is_decor_tile(t: u16) -> bool {
    matches!(t, 13..=20)
}

pub fn build_scene(g: &Game, params: &ShiftParams, s: &mut Scene3d) {
    s.clear();
    let d = &g.data;
    let ts = d.tile_size;
    let hero = g.player.feet();
    let (htx, hty) = ((hero.x / ts).floor() as i32, (hero.y / ts).floor() as i32);
    // The 2D camera's view must be fully covered: at the swap it shows up to
    // ~0.48 * view_w ≈ 10 columns behind the hero, past VISIBLE_BEHIND_TILES.
    let cam = g.camera.pixel_pos();
    let view_left = (cam.x / ts).floor() as i32;
    let view_right = ((cam.x + g.camera.view_w) / ts).floor() as i32;
    let x0 = (htx - VISIBLE_BEHIND_TILES).min(view_left).max(0);
    let x1 = (htx + VISIBLE_AHEAD_TILES).max(view_right).min(d.width as i32 - 1);

    for ty in 0..d.height as i32 {
        for tx in x0..=x1 {
            let i = (ty as u32 * d.width + tx as u32) as usize;
            let terrain = d.terrain.get(i).copied().unwrap_or(0);
            let center = to_world(Vec2::new(tx as f32 * ts + ts * 0.5, ty as f32 * ts + ts * 0.5), 0.0);
            if d.collision[i] != 0 {
                let dist = (((tx - htx).pow(2) + (ty - hty).pow(2)) as f32).sqrt();
                let e = block_extrusion(params.extrude, dist);
                let tile = if terrain == 0 { 5 } else { terrain };
                let half_z = if is_ground_tile(tile) { ROAD_HALF_WIDTH } else { ts * 0.5 };
                s.blocks.extend_from_slice(&[
                    center.x,
                    center.y,
                    center.z,
                    ts * 0.5,
                    ts * 0.5,
                    half_z * e,
                    tile as f32,
                    e,
                    e,
                    0.0,
                ]);
            } else if is_decor_tile(terrain) {
                let cols = d.tileset_columns as f32;
                let t = terrain as f32 - 1.0;
                let atlas =
                    [(t % cols).floor() * TILE_SIZE, (t / cols).floor() * TILE_SIZE, TILE_SIZE, TILE_SIZE];
                let bottom = to_world(Vec2::new(tx as f32 * ts + ts * 0.5, (ty + 1) as f32 * ts), DECOR_Z);
                push_billboard(s, bottom, ts, ts, atlas, TEX_TILES, FLAG_HOLO, params.extrude);
            }
        }
    }

    for sp in world_sprites(g) {
        let (w, h) = (sp.frame.w, sp.frame.h);
        let bottom = Vec2::new(sp.x + w * 0.5, sp.y + h);
        let mut flags = 0;
        if sp.kind != SpriteKind::Hero {
            flags |= FLAG_HOLO;
        }
        if sp.flip_x {
            flags |= FLAG_FLIP_X;
        }
        if sp.flip_y {
            flags |= FLAG_FLIP_Y;
        }
        let atlas = [sp.frame.x, sp.frame.y, w, h];
        push_billboard(s, to_world(bottom, 0.0), w, h, atlas, TEX_SPRITES, flags, 1.0);
        if sp.kind != SpriteKind::Particle {
            push_shadow(g, s, bottom, w.min(16.0) * 0.45);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_billboard(
    s: &mut Scene3d,
    anchor: glam::Vec3,
    w: f32,
    h: f32,
    atlas: [f32; 4],
    tex: u32,
    flags: u32,
    alpha: f32,
) {
    s.billboards.extend_from_slice(&[
        anchor.x,
        anchor.y,
        anchor.z,
        w,
        h,
        atlas[0],
        atlas[1],
        atlas[2],
        atlas[3],
        tex as f32,
        flags as f32,
        alpha,
    ]);
}

/// Disc on the first solid tile below `feet`; fainter the higher the entity.
fn push_shadow(g: &Game, s: &mut Scene3d, feet: Vec2, radius: f32) {
    let ts = g.level.tile_size;
    let tx = (feet.x / ts).floor() as i32;
    let mut ty = ((feet.y - 1.0) / ts).floor() as i32 + 1;
    while ty < g.level.height as i32 {
        if ty >= 0 && g.level.is_solid_tile(tx, ty) {
            let ground_y = ty as f32 * ts;
            let alpha = (1.0 - (ground_y - feet.y) / 96.0).clamp(0.2, 1.0);
            let p = to_world(Vec2::new(feet.x, ground_y), 0.0);
            s.shadows.extend_from_slice(&[p.x, p.y + 0.5, p.z, radius, alpha]);
            return;
        }
        ty += 1;
    }
}
