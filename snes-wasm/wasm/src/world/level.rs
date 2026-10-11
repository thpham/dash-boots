use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelData {
    pub width: u32,
    pub height: u32,
    #[serde(rename = "tileSize")]
    pub tile_size: f32,
    pub terrain: Vec<u16>,
    pub collision: Vec<u8>,
    pub coins: Vec<u8>,
    pub spawns: Spawns,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spawns {
    pub player: SpawnPoint,
    pub enemies: Vec<SpawnPoint>,
    pub goal: SpawnPoint,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub struct Level {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
    pub collision: Vec<u8>,
    pub terrain: Vec<u16>,
    pub coins: Vec<u8>,
}

impl Level {
    pub fn from_data(data: LevelData) -> Self {
        Self {
            width: data.width,
            height: data.height,
            tile_size: data.tile_size,
            collision: data.collision,
            terrain: data.terrain,
            coins: data.coins,
        }
    }

    pub fn is_solid_at_pixel(&self, x: f32, y: f32) -> bool {
        if x < 0.0 || y < 0.0 {
            return false;
        }
        let tx = (x / self.tile_size).floor() as i32;
        let ty = (y / self.tile_size).floor() as i32;
        if tx < 0 || ty < 0 || tx >= self.width as i32 || ty >= self.height as i32 {
            return false;
        }
        let idx = (ty as u32 * self.width + tx as u32) as usize;
        self.collision.get(idx).copied().unwrap_or(0) != 0
    }
}
