//! Render list built entirely in Rust: the host uploads `verts` once per frame
//! and issues one draw call per batch (one batch per texture switch).
//!
//! Vertex layout (8 × f32): x, y (screen pixels, origin top-left), u, v, r, g, b, a.
//! Batches are `[texture_id, first_vertex, vertex_count]` triples.

use crate::game::Game;
use crate::sprites::{self, Frame};

pub mod camera3d;
pub mod hud;
pub mod retro;

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
    pub(crate) tex_size: [(f32, f32); TEX_COUNT],
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

/// Draw the frame: world into `retro`, HUD and screens into `hud`.
pub fn draw_game(g: &Game, retro: &mut RenderList, hud: &mut RenderList) {
    retro::draw(g, retro);
    hud::draw(g, hud);
}
