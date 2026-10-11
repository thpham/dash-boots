//! Render list built entirely in Rust: the host uploads `verts` once per frame
//! and issues one draw call per batch (one batch per texture switch).
//!
//! Vertex layout (8 × f32): x, y (screen pixels, origin top-left), u, v, r, g, b, a.
//! Batches are `[texture_id, first_vertex, vertex_count]` triples.

use crate::game::{Game, Phase};
use crate::math::Vec2;
use crate::sprites::{self, Frame, PlayerFrame};
use crate::world::enemy::EnemyState;
use crate::world::particles::ParticleKind;
use crate::world::player;

pub const TEX_BG_FAR: u32 = 0;
pub const TEX_BG_NEAR: u32 = 1;
pub const TEX_TILES: u32 = 2;
pub const TEX_SPRITES: u32 = 3;
pub const TEX_FONT: u32 = 4;
pub const TEX_COUNT: usize = 5;

pub const FLOATS_PER_VERTEX: usize = 8;

pub type Color = [f32; 4];
pub const WHITE: Color = [1.0, 1.0, 1.0, 1.0];
pub const BLACK: Color = [0.0, 0.0, 0.0, 1.0];
pub const SHADOW: Color = [0.08, 0.04, 0.12, 1.0];
pub const YELLOW: Color = [1.0, 0.88, 0.3, 1.0];
pub const CYAN: Color = [0.5, 0.95, 0.9, 1.0];

/// Solid white texels in the font atlas (inside cell 127), used for rectangles.
const SOLID: Frame = Frame { x: 121.0, y: 41.0, w: 6.0, h: 6.0 };

pub struct RenderList {
    pub verts: Vec<f32>,
    pub batches: Vec<u32>,
    tex_size: [(f32, f32); TEX_COUNT],
    pub view_w: f32,
    pub view_h: f32,
}

impl RenderList {
    pub fn new(view_w: f32, view_h: f32) -> Self {
        Self {
            verts: Vec::with_capacity(4096 * FLOATS_PER_VERTEX),
            batches: Vec::with_capacity(32),
            // Defaults match the generated assets; the host overrides them with
            // the real image sizes once loaded.
            tex_size: [(320.0, 180.0), (320.0, 180.0), (128.0, 48.0), (256.0, 128.0), (128.0, 48.0)],
            view_w,
            view_h,
        }
    }

    pub fn set_texture_size(&mut self, tex: u32, w: f32, h: f32) {
        if let Some(s) = self.tex_size.get_mut(tex as usize) {
            *s = (w.max(1.0), h.max(1.0));
        }
    }

    pub fn clear(&mut self) {
        self.verts.clear();
        self.batches.clear();
    }

    fn batch_for(&mut self, tex: u32) {
        let n = self.batches.len();
        if n >= 3 && self.batches[n - 3] == tex {
            return;
        }
        let first = (self.verts.len() / FLOATS_PER_VERTEX) as u32;
        self.batches.extend_from_slice(&[tex, first, 0]);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn quad_uv(&mut self, tex: u32, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], c: Color) {
        self.batch_for(tex);
        let [u0, v0, u1, v1] = uv;
        let (x1, y1) = (x + w, y + h);
        for (px, py, u, v) in [
            (x, y, u0, v0),
            (x1, y, u1, v0),
            (x, y1, u0, v1),
            (x, y1, u0, v1),
            (x1, y, u1, v0),
            (x1, y1, u1, v1),
        ] {
            self.verts.extend_from_slice(&[px, py, u, v, c[0], c[1], c[2], c[3]]);
        }
        let n = self.batches.len();
        self.batches[n - 1] += 6;
    }

    /// Draw a texture sub-rectangle at its native size (scaled by `scale`).
    #[allow(clippy::too_many_arguments)]
    pub fn image(
        &mut self,
        tex: u32,
        x: f32,
        y: f32,
        f: Frame,
        flip_x: bool,
        flip_y: bool,
        scale: f32,
        c: Color,
    ) {
        let (tw, th) = self.tex_size[tex as usize];
        let (mut u0, mut u1) = (f.x / tw, (f.x + f.w) / tw);
        let (mut v0, mut v1) = (f.y / th, (f.y + f.h) / th);
        if flip_x {
            std::mem::swap(&mut u0, &mut u1);
        }
        if flip_y {
            std::mem::swap(&mut v0, &mut v1);
        }
        self.quad_uv(tex, x, y, f.w * scale, f.h * scale, [u0, v0, u1, v1], c);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        let (tw, th) = self.tex_size[TEX_FONT as usize];
        let uv = [SOLID.x / tw, SOLID.y / th, (SOLID.x + SOLID.w) / tw, (SOLID.y + SOLID.h) / th];
        self.quad_uv(TEX_FONT, x, y, w, h, uv, c);
    }

    pub fn text_width(s: &str, scale: f32) -> f32 {
        s.chars().count() as f32 * sprites::FONT_ADVANCE * scale
    }

    pub fn text(&mut self, x: f32, y: f32, s: &str, c: Color, scale: f32) {
        let mut cx = x;
        for ch in s.chars() {
            let code = ch as u32;
            if (33..128).contains(&code) {
                let cell = code - 32;
                let f = Frame {
                    x: (cell % sprites::FONT_COLUMNS) as f32 * sprites::FONT_CELL,
                    y: (cell / sprites::FONT_COLUMNS) as f32 * sprites::FONT_CELL,
                    w: sprites::FONT_CELL,
                    h: sprites::FONT_CELL,
                };
                self.image(TEX_FONT, cx, y, f, false, false, scale, c);
            }
            cx += sprites::FONT_ADVANCE * scale;
        }
    }

    /// Text with a 1px drop shadow (scaled), the classic HUD look.
    pub fn text_shadow(&mut self, x: f32, y: f32, s: &str, c: Color, scale: f32) {
        self.text(x + scale, y + scale, s, SHADOW, scale);
        self.text(x, y, s, c, scale);
    }

    pub fn text_centered(&mut self, y: f32, s: &str, c: Color, scale: f32) {
        let x = ((self.view_w - Self::text_width(s, scale)) * 0.5).round();
        self.text_shadow(x, y, s, c, scale);
    }
}

fn format_time(t: f32) -> String {
    let s = t.max(0.0) as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Draw the whole frame: parallax, tilemap, entities, HUD and screens.
pub fn draw_game(g: &Game, r: &mut RenderList) {
    r.clear();
    let cam = g.camera.pixel_pos();
    let (vw, vh) = (r.view_w, r.view_h);
    let max_cam_y = (g.level.height_px() - vh).max(0.0);

    draw_parallax(r, cam, max_cam_y);
    draw_tiles(g, r, cam);
    draw_entities(g, r, cam);
    draw_player(g, r, cam);
    draw_particles(g, r, cam);

    match g.phase {
        Phase::Title => draw_title(g, r),
        Phase::Won => {
            draw_hud(g, r);
            draw_clear(g, r);
        }
        _ => draw_hud(g, r),
    }
    if g.paused {
        r.rect(0.0, 0.0, vw, vh, [0.0, 0.0, 0.05, 0.55]);
        r.text_centered(vh * 0.5 - 12.0, "PAUSED", WHITE, 2.0);
        r.text_centered(vh * 0.5 + 10.0, "PRESS ENTER TO RESUME", CYAN, 1.0);
    }
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
            r.image(TEX_TILES, tx as f32 * ts - cam.x, ty as f32 * ts - cam.y, f, false, false, 1.0, WHITE);
        }
    }
}

/// World-space sprite with culling and pixel snapping.
fn world_sprite(r: &mut RenderList, cam: Vec2, f: Frame, x: f32, y: f32, flip_x: bool, flip_y: bool) {
    let sx = (x - cam.x).round();
    let sy = (y - cam.y).round();
    if sx + f.w < 0.0 || sy + f.h < 0.0 || sx > r.view_w || sy > r.view_h {
        return;
    }
    r.image(TEX_SPRITES, sx, sy, f, flip_x, flip_y, 1.0, WHITE);
}

fn draw_entities(g: &Game, r: &mut RenderList, cam: Vec2) {
    let fr = g.frame;
    for c in &g.checkpoints {
        let f =
            if c.active { sprites::CHECKPOINT_ON[((fr / 15) % 2) as usize] } else { sprites::CHECKPOINT_OFF };
        world_sprite(r, cam, f, c.feet.x - 4.0, c.feet.y - 32.0, false, false);
    }
    world_sprite(r, cam, sprites::GOAL, g.goal.feet.x - 5.0, g.goal.feet.y - 48.0, false, false);
    for (i, c) in g.coins.iter().enumerate() {
        if !c.collected {
            let f = sprites::COIN[((fr / 8 + i as u64 / 3) % 4) as usize];
            world_sprite(r, cam, f, c.tile_pos.x, c.tile_pos.y, false, false);
        }
    }
    for p in &g.pickups {
        if !p.taken {
            let bob = ((fr as f32) * 0.08).sin() * 2.0;
            world_sprite(r, cam, sprites::BOOTS, p.feet.x - 8.0, p.feet.y - 16.0 + bob.round(), false, false);
        }
    }
    for e in &g.enemies {
        let (x, y) = (e.pos.x - 1.0, e.pos.y - 2.0);
        match e.state {
            EnemyState::Dormant | EnemyState::Walking => {
                let f = sprites::ENEMY_WALK[((e.anim * 6.0) as usize) % 2];
                world_sprite(r, cam, f, x, y, e.dir > 0.0, false);
            }
            EnemyState::Squashed(_) => world_sprite(r, cam, sprites::ENEMY_SQUASH, x, y, false, false),
            EnemyState::Knocked => world_sprite(r, cam, sprites::ENEMY_WALK[0], x, y, false, true),
            EnemyState::Dead => {}
        }
    }
}

fn draw_player(g: &Game, r: &mut RenderList, cam: Vec2) {
    let p = &g.player;
    if p.invuln > 0.0 && (g.frame / 3).is_multiple_of(2) {
        return;
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
    world_sprite(r, cam, f, x, y, p.facing < 0.0, flip_y);
}

fn draw_particles(g: &Game, r: &mut RenderList, cam: Vec2) {
    for p in g.particles_list() {
        let t = p.t();
        let f = match p.kind {
            ParticleKind::Dust => sprites::DUST[((t * 3.0) as usize).min(2)],
            ParticleKind::Sparkle => sprites::SPARKLE[((p.age * 12.0) as usize) % 2],
            ParticleKind::Star => sprites::STAR[((t * 3.0) as usize).min(2)],
        };
        world_sprite(r, cam, f, p.pos.x - 4.0, p.pos.y - 4.0, false, false);
    }
}

fn draw_hud(g: &Game, r: &mut RenderList) {
    let coins = format!("$ {:02}/{:02}", g.coins_collected, g.coins_total);
    r.text_shadow(6.0, 6.0, &coins, WHITE, 1.0);
    if g.player.dash_boots {
        r.text_shadow(6.0 + RenderList::text_width(&coins, 1.0) + 6.0, 6.0, "^", WHITE, 1.0);
    }
    let deaths = format!("# {}", g.deaths);
    let dx = ((r.view_w - RenderList::text_width(&deaths, 1.0)) * 0.5).round();
    r.text_shadow(dx, 6.0, &deaths, WHITE, 1.0);
    let time = format!("TIME {}", format_time(g.time));
    let tx = r.view_w - RenderList::text_width(&time, 1.0) - 6.0;
    r.text_shadow(tx, 6.0, &time, WHITE, 1.0);
}

fn draw_title(g: &Game, r: &mut RenderList) {
    let (vw, vh) = (r.view_w, r.view_h);
    r.rect(0.0, 0.0, vw, vh, [0.02, 0.0, 0.08, 0.35]);
    r.text_centered(26.0, "DASH BOOTS", YELLOW, 3.0);
    r.text_centered(54.0, "A 16-BIT PLATFORM ADVENTURE", CYAN, 1.0);
    if (g.frame / 30).is_multiple_of(2) {
        r.text_centered(84.0, "PRESS ENTER OR SPACE", WHITE, 1.0);
    }
    let panel_y = 106.0;
    r.rect(24.0, panel_y - 4.0, vw - 48.0, 66.0, [0.0, 0.0, 0.0, 0.45]);
    let lines = [
        "ARROWS/WASD   MOVE",
        "SPACE/K/Z     JUMP (HOLD = HIGHER)",
        "SHIFT/J/X     RUN  (+ DASH W/ BOOTS)",
        "DOWN/S        CROUCH / SLIDE",
        "ENTER         PAUSE     M  MUTE",
    ];
    for (i, l) in lines.iter().enumerate() {
        r.text_shadow(34.0, panel_y + 2.0 + i as f32 * 12.0, l, WHITE, 1.0);
    }
}

fn draw_clear(g: &Game, r: &mut RenderList) {
    let (vw, vh) = (r.view_w, r.view_h);
    let t = (g.phase_time / 0.4).min(1.0);
    let h = (96.0 * t).round();
    r.rect(40.0, (vh - h) * 0.5, vw - 80.0, h, [0.0, 0.0, 0.08, 0.7]);
    if t < 1.0 {
        return;
    }
    let top = (vh - 96.0) * 0.5;
    r.text_centered(top + 8.0, "COURSE CLEAR!", YELLOW, 2.0);
    r.text_centered(
        top + 34.0,
        &format!("COINS   {:02}/{:02}", g.coins_collected, g.coins_total),
        WHITE,
        1.0,
    );
    r.text_centered(top + 46.0, &format!("TIME    {}", format_time(g.time)), WHITE, 1.0);
    r.text_centered(top + 58.0, &format!("DEATHS  {:5}", g.deaths), WHITE, 1.0);
    if g.phase_time > 1.2 && (g.frame / 30).is_multiple_of(2) {
        r.text_centered(top + 78.0, "PRESS ENTER TO PLAY AGAIN", CYAN, 1.0);
    }
}
