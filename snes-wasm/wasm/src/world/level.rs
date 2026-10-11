use serde::{Deserialize, Serialize};

/// Compact runtime level produced by the TypeScript Tiled loader
/// (`src/levels/level_loader.ts`). Extra fields (terrain, tileset) are ignored here.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelData {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
    /// `width * height` cells, row-major; 0 = empty, >0 = solid.
    pub collision: Vec<u8>,
    /// `width * height` cells, row-major; 0 = none, >0 = coin.
    pub coins: Vec<u8>,
    /// `width * height` visual tile ids local to the tileset (1-based, 0 = empty).
    /// May be empty (no visuals, e.g. in tests).
    #[serde(default)]
    pub terrain: Vec<u16>,
    #[serde(default = "default_tileset_columns")]
    pub tileset_columns: u32,
    pub spawns: Spawns,
}

fn default_tileset_columns() -> u32 {
    8
}

/// All spawn points are "feet" points: x = horizontal center, y = bottom, in pixels.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spawns {
    pub player: SpawnPoint,
    #[serde(default)]
    pub enemies: Vec<SpawnPoint>,
    pub goal: SpawnPoint,
    #[serde(default)]
    pub checkpoints: Vec<SpawnPoint>,
    #[serde(default)]
    pub dash_boots: Vec<SpawnPoint>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpawnPoint {
    pub x: f32,
    pub y: f32,
}

impl LevelData {
    pub fn validate(&self) -> Result<(), String> {
        let n = (self.width as usize) * (self.height as usize);
        if self.width == 0 || self.height == 0 {
            return Err(format!("level size must be non-zero ({}x{})", self.width, self.height));
        }
        if !self.tile_size.is_finite() || self.tile_size <= 0.0 {
            return Err(format!("invalid tileSize {}", self.tile_size));
        }
        if self.collision.len() != n {
            return Err(format!("collision has {} cells, expected {}", self.collision.len(), n));
        }
        if self.coins.len() != n {
            return Err(format!("coins has {} cells, expected {}", self.coins.len(), n));
        }
        if !self.terrain.is_empty() && self.terrain.len() != n {
            return Err(format!("terrain has {} cells, expected {}", self.terrain.len(), n));
        }
        if self.tileset_columns == 0 {
            return Err("tileset has zero columns".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Level {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
    pub collision: Vec<u8>,
}

impl Level {
    pub fn from_data(data: &LevelData) -> Self {
        Self {
            width: data.width,
            height: data.height,
            tile_size: data.tile_size,
            collision: data.collision.clone(),
        }
    }

    pub fn width_px(&self) -> f32 {
        self.width as f32 * self.tile_size
    }

    pub fn height_px(&self) -> f32 {
        self.height as f32 * self.tile_size
    }

    /// Bounds policy: left/right outside the map are walls, rows above the map
    /// extend the top row upward (no walking over walls), below the map is open
    /// (pits) so the player can fall out and die.
    pub fn is_solid_tile(&self, tx: i32, ty: i32) -> bool {
        if tx < 0 || tx >= self.width as i32 {
            return true;
        }
        if ty >= self.height as i32 {
            return false;
        }
        let ty = ty.max(0);
        let idx = ty as usize * self.width as usize + tx as usize;
        self.collision.get(idx).copied().unwrap_or(0) != 0
    }

    pub fn is_solid_at_pixel(&self, x: f32, y: f32) -> bool {
        let tx = (x / self.tile_size).floor() as i32;
        let ty = (y / self.tile_size).floor() as i32;
        self.is_solid_tile(tx, ty)
    }
}
