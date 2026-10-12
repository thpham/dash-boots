use super::{RenderList, TEX_BG_FAR, TEX_BG_NEAR, TEX_SPRITES, TEX_TILES, WHITE};
use crate::game::{Game, Phase};
use crate::math::Vec2;
use crate::sprites::{self, Frame, PlayerFrame};
use crate::world::cheese::WheelState;
use crate::world::enemy::{EnemyState, Kind};
use crate::world::particles::ParticleKind;
use crate::world::player;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteKind {
    Hero,
    Entity,
    Particle,
}

/// A sprite in level pixel space (top-left corner), shared by the 2D and 3D renderers.
#[derive(Clone, Copy, Debug)]
pub struct WorldSprite {
    pub frame: Frame,
    pub x: f32,
    pub y: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub kind: SpriteKind,
}

pub fn draw(g: &Game, r: &mut RenderList) {
    r.clear();
    let cam = g.camera.pixel_pos();
    let max_cam_y = (g.level.height_px() - r.view_h).max(0.0);
    draw_parallax(r, cam, max_cam_y);
    draw_tiles(g, r, cam);
    for s in world_sprites(g) {
        let sx = (s.x - cam.x).round();
        let sy = (s.y - cam.y).round();
        if sx + s.frame.w < 0.0 || sy + s.frame.h < 0.0 || sx > r.view_w || sy > r.view_h {
            continue;
        }
        r.image(TEX_SPRITES, sx, sy, s.frame, s.flip_x, s.flip_y, 1.0, WHITE);
    }
}

/// Every visible sprite in draw order: checkpoints, goal, active rifts, coins, pickups, enemies, hero, particles.
pub fn world_sprites(g: &Game) -> Vec<WorldSprite> {
    let mut out = Vec::with_capacity(64);
    let fr = g.frame;
    let mut push = |frame: Frame, x: f32, y: f32, flip_x: bool, flip_y: bool, kind: SpriteKind| {
        out.push(WorldSprite { frame, x, y, flip_x, flip_y, kind })
    };
    for c in &g.checkpoints {
        let f =
            if c.active { sprites::CHECKPOINT_ON[((fr / 15) % 2) as usize] } else { sprites::CHECKPOINT_OFF };
        push(f, c.feet.x - 4.0, c.feet.y - 32.0, false, false, SpriteKind::Entity);
    }
    push(sprites::GOAL, g.goal.feet.x - 5.0, g.goal.feet.y - 48.0, false, false, SpriteKind::Entity);
    for (i, r) in g.rifts.iter().enumerate() {
        if r.active {
            // Irregular flicker: the frame advances whenever `fr` is a
            // multiple of 4 or of 7 (hold lengths of 1..4 steps, two
            // advances as close as 1 step apart), shifted per rift.
            let k = (fr / 4 + fr / 7 + i as u64) % 3;
            let f = sprites::RIFT[k as usize];
            push(f, r.feet.x - f.w * 0.5, r.feet.y - f.h, false, false, SpriteKind::Entity);
        }
    }
    for (i, c) in g.coins.iter().enumerate() {
        if !c.collected {
            let f = sprites::COIN[((fr / 8 + i as u64 / 3) % 4) as usize];
            push(f, c.tile_pos.x, c.tile_pos.y, false, false, SpriteKind::Entity);
        }
    }
    for p in &g.pickups {
        if !p.taken {
            let bob = ((fr as f32) * 0.08).sin() * 2.0;
            push(
                sprites::BOOTS,
                p.feet.x - 8.0,
                p.feet.y - 16.0 + bob.round(),
                false,
                false,
                SpriteKind::Entity,
            );
        }
    }
    for e in &g.enemies {
        let (x, y) = (e.pos.x - 1.0, e.pos.y - 2.0);
        // Goats sit dazed instead of flipping over (their wheel drops); marmots
        // keep the shell-style flip.
        let (walk, dazed) = if e.kind == Kind::Goat {
            (sprites::GOAT_WALK, sprites::GOAT_DAZED)
        } else {
            (sprites::ENEMY_WALK, sprites::ENEMY_SQUASH)
        };
        match e.state {
            EnemyState::Dormant | EnemyState::Walking => {
                let f = walk[((e.anim * 6.0) as usize) % 2];
                push(f, x, y, e.dir > 0.0, false, SpriteKind::Entity);
            }
            EnemyState::Squashed(_) => push(dazed, x, y, false, false, SpriteKind::Entity),
            EnemyState::Flipped(_) if e.kind == Kind::Goat => {
                push(dazed, x, y, false, false, SpriteKind::Entity)
            }
            EnemyState::Flipped(_) => {
                // On its back: upside-down, wiggling legs.
                let f = sprites::ENEMY_WALK[((e.anim * 12.0) as usize) % 2];
                push(f, x, y, false, true, SpriteKind::Entity);
            }
            EnemyState::Knocked => push(walk[0], x, y, false, true, SpriteKind::Entity),
            EnemyState::Dead => {}
        }
    }
    for w in &g.wheels {
        let f = match w.state {
            WheelState::Floor => sprites::CHEESE[((w.anim * 2.0) as usize) % 2],
            // Rolling: the streak pose, facing the way it slides (the
            // renderer cannot scale, so the squash-stretch is the roll streak).
            WheelState::Slide { dir } => {
                push(
                    sprites::CHEESE_ROLL,
                    w.pos.x - 2.0,
                    w.pos.y - 2.0,
                    dir < 0.0,
                    false,
                    SpriteKind::Entity,
                );
                continue;
            }
        };
        push(f, w.pos.x - 2.0, w.pos.y - 2.0, false, false, SpriteKind::Entity);
    }
    for c in &g.cows {
        let f =
            if c.browsing() { sprites::COW_BROWSE } else { sprites::COW_WALK[((c.anim * 6.0) as usize) % 2] };
        push(f, c.pos.x - 1.0, c.pos.y - 4.0, c.dir > 0.0, false, SpriteKind::Entity);
    }
    if let Some((f, x, y, flip_x, flip_y)) = hero_sprite(g) {
        push(f, x, y, flip_x, flip_y, SpriteKind::Hero);
        // Carrying: the wheel rides in front of the hero's chest hand.
        if g.player.cheese {
            let (ox, oy) = if g.player.facing > 0.0 { (14.0, -10.0) } else { (-16.0, -10.0) };
            push(
                sprites::CHEESE[0],
                g.player.pos.x + ox,
                g.player.pos.y + oy,
                false,
                false,
                SpriteKind::Entity,
            );
        }
    }
    for p in g.particles_list() {
        let t = p.t();
        let f = match p.kind {
            ParticleKind::Dust => sprites::DUST[((t * 3.0) as usize).min(2)],
            ParticleKind::Sparkle => sprites::SPARKLE[((p.age * 12.0) as usize) % 2],
            ParticleKind::Star => sprites::STAR[((t * 3.0) as usize).min(2)],
        };
        push(f, p.pos.x - 4.0, p.pos.y - 4.0, false, false, SpriteKind::Particle);
    }
    out
}

/// The hero's frame and top-left position, or `None` on invulnerability blink frames.
fn hero_sprite(g: &Game) -> Option<(Frame, f32, f32, bool, bool)> {
    let p = &g.player;
    if p.invuln > 0.0 && (g.frame / 3).is_multiple_of(2) {
        return None;
    }
    let (frame, flip_y) = match g.phase {
        Phase::Dying => (PlayerFrame::Fall, true),
        Phase::Won => (if p.on_ground { PlayerFrame::Idle } else { PlayerFrame::Jump }, false),
        _ => {
            let f = if p.is_dashing() {
                PlayerFrame::Dash
            } else if p.crouching {
                if p.sliding {
                    PlayerFrame::Slide
                } else {
                    PlayerFrame::Crouch
                }
            } else if !p.on_ground {
                if p.vel.y < 0.0 {
                    PlayerFrame::Jump
                } else {
                    PlayerFrame::Fall
                }
            } else if p.vel.x.abs() > 8.0 {
                const CYCLE: [PlayerFrame; 4] =
                    [PlayerFrame::Run1, PlayerFrame::Run2, PlayerFrame::Run3, PlayerFrame::Run2];
                CYCLE[((p.run_anim / 7.0) as usize) % 4]
            } else {
                PlayerFrame::Idle
            };
            (f, false)
        }
    };
    let f = sprites::player_frame(frame, p.dash_boots);
    let feet = p.feet();
    let x = p.pos.x + player::WIDTH * 0.5 - sprites::PLAYER_W * 0.5;
    let y = feet.y - sprites::PLAYER_H;
    Some((f, x, y, p.facing < 0.0, flip_y))
}

fn draw_parallax(r: &mut RenderList, cam: Vec2, max_cam_y: f32) {
    let (vw, vh) = (r.view_w, r.view_h);
    // Textures repeat horizontally and clamp vertically (host sampler setup):
    // above the image we get the top row (sky / transparent), below the bottom row.
    for (tex, fx, fy) in [(TEX_BG_FAR, 0.15, 0.1), (TEX_BG_NEAR, 0.4, 0.3)] {
        let (tw, th) = r.tex_size[tex as usize];
        let off_x = (cam.x * fx).round();
        let off_y = ((max_cam_y - cam.y) * fy).round();
        let uv = [off_x / tw, -off_y / th, (off_x + vw) / tw, (vh - off_y) / th];
        r.quad_uv(tex, 0.0, 0.0, vw, vh, uv, WHITE);
    }
}

fn draw_tiles(g: &Game, r: &mut RenderList, cam: Vec2) {
    let d = &g.data;
    if d.terrain.is_empty() {
        return;
    }
    let ts = d.tile_size;
    let x0 = (cam.x / ts).floor().max(0.0) as u32;
    let y0 = (cam.y / ts).floor().max(0.0) as u32;
    let x1 = (((cam.x + r.view_w) / ts).ceil() as u32).min(d.width);
    let y1 = (((cam.y + r.view_h) / ts).ceil() as u32).min(d.height);
    let cols = d.tileset_columns;
    for ty in y0..y1 {
        for tx in x0..x1 {
            let t = d.terrain[(ty * d.width + tx) as usize] as u32;
            if t == 0 {
                continue;
            }
            let f = Frame {
                x: ((t - 1) % cols) as f32 * sprites::TILE_SIZE,
                y: ((t - 1) / cols) as f32 * sprites::TILE_SIZE,
                w: sprites::TILE_SIZE,
                h: sprites::TILE_SIZE,
            };
            let dy = g
                .bumps
                .iter()
                .find(|b| b.ty == ty as i32 && b.tx0 <= tx as i32 && tx as i32 <= b.tx1)
                .map(|b| b.offset())
                .unwrap_or(0.0);
            r.image(
                TEX_TILES,
                tx as f32 * ts - cam.x,
                ty as f32 * ts - cam.y - dy,
                f,
                false,
                false,
                1.0,
                WHITE,
            );
        }
    }
}
