use serde::Serialize;

#[derive(Serialize)]
pub struct SpriteDraw {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

#[derive(Serialize)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize)]
pub struct TilemapRenderData {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
    pub terrain: Vec<u16>,
    pub coins: Vec<u8>,
}

#[derive(Serialize)]
pub struct FrameEvents {
    pub sprites: Vec<SpriteDraw>,
    pub coin_count: u32,
    pub won: bool,
    pub sfx: Vec<String>,
    pub camera: Camera,
    pub tilemap: TilemapRenderData,
}
