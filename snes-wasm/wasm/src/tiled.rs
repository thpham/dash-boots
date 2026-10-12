//! Tiled JSON map → `LevelData`.
//!
//! Format convention (see the design spec, §5.2):
//! - orthogonal, finite map, square tiles, CSV (array) layer data
//! - tile layers `terrain` (visuals), `collision` (gid > 0 = solid), `coins` (gid > 0 = coin)
//! - object layer `objects` with objects whose class/type (or name) is one of
//!   `player_spawn` (required), `goal` (required), `enemy_spawn`, `checkpoint`, `dash_boots`, `rift`
//!
//! Errors are explicit so a drifting export fails loudly instead of producing a broken level.

use serde_json::Value;

use crate::world::level::{LevelData, SpawnPoint, Spawns};

/// Tiled stores flip/rotation flags in the top bits of each gid.
const GID_MASK: u64 = 0x1FFF_FFFF;

fn err<T>(msg: impl Into<String>) -> Result<T, String> {
    Err(format!("Tiled map: {}", msg.into()))
}

fn get_u32(v: &Value, key: &str) -> Result<u32, String> {
    match v.get(key).and_then(Value::as_u64) {
        Some(n) => Ok(n as u32),
        None => err(format!("missing or invalid `{key}`")),
    }
}

fn get_f32(v: &Value, key: &str) -> f32 {
    v.get(key).and_then(Value::as_f64).unwrap_or(0.0) as f32
}

fn find_layer<'a>(layers: &'a [Value], name: &str, kind: &str) -> Result<&'a Value, String> {
    let layer = layers.iter().find(|l| l.get("name").and_then(Value::as_str) == Some(name));
    let Some(layer) = layer else {
        return err(format!("missing layer `{name}`"));
    };
    let ty = layer.get("type").and_then(Value::as_str).unwrap_or("");
    if ty != kind {
        return err(format!("layer `{name}` must be a {kind}, found `{ty}`"));
    }
    Ok(layer)
}

fn layer_gids(layers: &[Value], name: &str, expected: usize) -> Result<Vec<u32>, String> {
    let layer = find_layer(layers, name, "tilelayer")?;
    if let Some(enc) = layer.get("encoding").and_then(Value::as_str) {
        if enc != "csv" {
            return err(format!("layer `{name}` uses `{enc}` encoding; export with CSV layer format"));
        }
    }
    let Some(data) = layer.get("data").and_then(Value::as_array) else {
        return err(format!("layer `{name}` has no `data` array"));
    };
    if data.len() != expected {
        return err(format!("layer `{name}` has {} cells, expected {expected}", data.len()));
    }
    data.iter()
        .map(|g| match g.as_u64() {
            Some(g) => Ok((g & GID_MASK) as u32),
            None => err(format!("layer `{name}` contains a non-integer gid")),
        })
        .collect()
}

fn object_kind(o: &Value) -> &str {
    for key in ["class", "type", "name"] {
        if let Some(s) = o.get(key).and_then(Value::as_str) {
            if !s.is_empty() {
                return s;
            }
        }
    }
    ""
}

/// Feet point of an object: bottom-center for rectangles, the point itself for
/// point objects. Tile objects (with a gid) are anchored bottom-left in Tiled.
fn object_feet(o: &Value) -> SpawnPoint {
    let (x, y, w, h) = (get_f32(o, "x"), get_f32(o, "y"), get_f32(o, "width"), get_f32(o, "height"));
    if o.get("gid").is_some() {
        SpawnPoint { x: x + w * 0.5, y }
    } else {
        SpawnPoint { x: x + w * 0.5, y: y + h }
    }
}

pub fn parse_tiled_json(text: &str) -> Result<LevelData, String> {
    let json: Value = serde_json::from_str(text).map_err(|e| format!("Tiled map: invalid JSON: {e}"))?;

    if let Some(o) = json.get("orientation").and_then(Value::as_str) {
        if o != "orthogonal" {
            return err(format!("orientation must be orthogonal, found `{o}`"));
        }
    }
    if json.get("infinite").and_then(Value::as_bool) == Some(true) {
        return err("infinite maps are not supported");
    }
    let width = get_u32(&json, "width")?;
    let height = get_u32(&json, "height")?;
    let tile_w = get_u32(&json, "tilewidth")?;
    let tile_h = get_u32(&json, "tileheight")?;
    if tile_w != tile_h || tile_w == 0 {
        return err(format!("tiles must be square, found {tile_w}x{tile_h}"));
    }

    let Some(tileset) = json.get("tilesets").and_then(Value::as_array).and_then(|t| t.first()) else {
        return err("missing tileset");
    };
    let firstgid = get_u32(tileset, "firstgid")?;
    let columns = tileset.get("columns").and_then(Value::as_u64).unwrap_or(8) as u32;

    let Some(layers) = json.get("layers").and_then(Value::as_array) else {
        return err("missing `layers`");
    };
    let n = width as usize * height as usize;
    let terrain = layer_gids(layers, "terrain", n)?
        .into_iter()
        .map(|g| if g >= firstgid { (g - firstgid + 1) as u16 } else { 0 })
        .collect();
    let collision = layer_gids(layers, "collision", n)?.into_iter().map(|g| (g > 0) as u8).collect();
    let coins = layer_gids(layers, "coins", n)?.into_iter().map(|g| (g > 0) as u8).collect();

    let objects_layer = find_layer(layers, "objects", "objectgroup")?;
    let objects = objects_layer.get("objects").and_then(Value::as_array).cloned().unwrap_or_default();
    let of_kind = |kind: &str| -> Vec<SpawnPoint> {
        objects.iter().filter(|o| object_kind(o) == kind).map(object_feet).collect()
    };
    let one = |kind: &str| -> Result<SpawnPoint, String> {
        match of_kind(kind).as_slice() {
            [p] => Ok(*p),
            [] => err(format!("missing object `{kind}`")),
            _ => err(format!("expected exactly one `{kind}` object")),
        }
    };

    let data = LevelData {
        width,
        height,
        tile_size: tile_w as f32,
        collision,
        coins,
        terrain,
        tileset_columns: columns,
        spawns: Spawns {
            player: one("player_spawn")?,
            goal: one("goal")?,
            enemies: of_kind("enemy_spawn"),
            goats: of_kind("goat_spawn"),
            cows: of_kind("cow"),
            checkpoints: of_kind("checkpoint"),
            dash_boots: of_kind("dash_boots"),
            rifts: of_kind("rift"),
        },
    };
    data.validate().map_err(|e| format!("Tiled map: {e}"))?;
    Ok(data)
}
