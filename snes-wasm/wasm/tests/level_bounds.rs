use snes_wasm::world::level::{Level, LevelData, SpawnPoint, Spawns};

#[test]
fn out_of_bounds_is_solid() {
    let w = 2;
    let h = 2;
    let tile_size = 16.0;

    let data = LevelData {
        width: w,
        height: h,
        tile_size,
        terrain: vec![0; (w * h) as usize],
        collision: vec![0; (w * h) as usize],
        coins: vec![0; (w * h) as usize],
        spawns: Spawns {
            player: SpawnPoint { x: 0.0, y: 0.0 },
            enemies: vec![],
            goal: SpawnPoint { x: 0.0, y: 0.0 },
        },
    };

    let level = Level::from_data(data);

    assert!(level.is_solid_at_pixel(-1.0, 0.0));
    assert!(level.is_solid_at_pixel(0.0, -1.0));
    assert!(level.is_solid_at_pixel((w as f32) * tile_size + 1.0, 0.0));
    assert!(level.is_solid_at_pixel(0.0, (h as f32) * tile_size + 1.0));
}
