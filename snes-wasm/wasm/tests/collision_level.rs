use snes_wasm::world::{level::Level, player::{InputState, Player, Vec2}};

#[test]
fn player_lands_on_ground_tile() {
    // Build a 10x5 level with a solid row at y=4.
    let width = 10;
    let height = 5;
    let tile_size = 16.0;

    let mut collision = vec![0u8; (width * height) as usize];
    for x in 0..width {
        collision[(4 * width + x) as usize] = 1;
    }

    let level = Level {
        width,
        height,
        tile_size,
        collision,
        terrain: vec![0u16; (width * height) as usize],
        coins: vec![0u8; (width * height) as usize],
    };

    let mut player = Player::new();
    player.pos = Vec2 { x: 32.0, y: 0.0 };
    player.vel = Vec2 { x: 0.0, y: 0.0 };

    let input = InputState::default();
    let dt = 1.0 / 60.0;

    for _ in 0..240 {
        player.step(&input, &level, dt);
    }

    // Player is 16px tall; ground row starts at y=4*16.
    let expected_y = 4.0 * tile_size - 16.0;
    assert!((player.pos.y - expected_y).abs() < 0.01, "pos.y={} expected {}", player.pos.y, expected_y);
    assert!(player.on_ground);
}
