//! Atlas layouts. Must match `tools/gen_art.py`.

/// Rectangle in texture pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

const fn f(x: f32, y: f32, w: f32, h: f32) -> Frame {
    Frame { x, y, w, h }
}

// sprites.png
pub const PLAYER_W: f32 = 16.0;
pub const PLAYER_H: f32 = 24.0;
pub const PLAYER_ROW_BOOTS: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerFrame {
    Idle = 0,
    Run1,
    Run2,
    Run3,
    Jump,
    Fall,
    Crouch,
    Slide,
    Dash,
}

pub fn player_frame(frame: PlayerFrame, boots: bool) -> Frame {
    f(frame as u32 as f32 * PLAYER_W, if boots { PLAYER_ROW_BOOTS } else { 0.0 }, PLAYER_W, PLAYER_H)
}

pub const ENEMY_WALK: [Frame; 2] = [f(0.0, 48.0, 16.0, 16.0), f(16.0, 48.0, 16.0, 16.0)];
pub const ENEMY_SQUASH: Frame = f(32.0, 48.0, 16.0, 16.0);
pub const COIN: [Frame; 4] = [
    f(48.0, 48.0, 16.0, 16.0),
    f(64.0, 48.0, 16.0, 16.0),
    f(80.0, 48.0, 16.0, 16.0),
    f(96.0, 48.0, 16.0, 16.0),
];
pub const BOOTS: Frame = f(112.0, 48.0, 16.0, 16.0);
pub const GOAL: Frame = f(0.0, 64.0, 16.0, 48.0);
pub const CHECKPOINT_OFF: Frame = f(16.0, 64.0, 16.0, 32.0);
pub const CHECKPOINT_ON: [Frame; 2] = [f(32.0, 64.0, 16.0, 32.0), f(48.0, 64.0, 16.0, 32.0)];
pub const DUST: [Frame; 3] = [f(64.0, 64.0, 8.0, 8.0), f(72.0, 64.0, 8.0, 8.0), f(80.0, 64.0, 8.0, 8.0)];
pub const SPARKLE: [Frame; 2] = [f(88.0, 64.0, 8.0, 8.0), f(96.0, 64.0, 8.0, 8.0)];
pub const STAR: [Frame; 3] = [f(104.0, 64.0, 8.0, 8.0), f(112.0, 64.0, 8.0, 8.0), f(120.0, 64.0, 8.0, 8.0)];
pub const RIFT: [Frame; 3] =
    [f(16.0, 96.0, 16.0, 32.0), f(32.0, 96.0, 16.0, 32.0), f(48.0, 96.0, 16.0, 32.0)];

// Swiss alps (see tools/gen_art.py build_sprites: the goat row + the cow row).
pub const GOAT_WALK: [Frame; 2] = [f(128.0, 64.0, 16.0, 16.0), f(144.0, 64.0, 16.0, 16.0)];
pub const GOAT_DAZED: Frame = f(160.0, 64.0, 16.0, 16.0);
pub const CHEESE: [Frame; 2] = [f(176.0, 64.0, 16.0, 16.0), f(192.0, 64.0, 16.0, 16.0)];
pub const CHEESE_ROLL: Frame = f(208.0, 64.0, 16.0, 16.0);
pub const COW_WALK: [Frame; 2] = [f(128.0, 80.0, 16.0, 16.0), f(144.0, 80.0, 16.0, 16.0)];
pub const COW_BROWSE: Frame = f(160.0, 80.0, 16.0, 16.0);

// tiles.png: 16x16 tiles, 8 columns, local tile id = gid - firstgid + 1.
pub const TILE_SIZE: f32 = 16.0;

// font.png: 8x8 cells, 16 columns, ASCII 32..127. Cell 127 is a solid white block.
pub const FONT_CELL: f32 = 8.0;
pub const FONT_COLUMNS: u32 = 16;
pub const FONT_ADVANCE: f32 = 6.0;
