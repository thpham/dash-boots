use snes_wasm::tiled::parse_tiled_json;

fn map(layers_extra: &str, objects: &str) -> String {
    format!(
        r#"{{
  "orientation": "orthogonal", "infinite": false, "width": 3, "height": 2,
  "tilewidth": 16, "tileheight": 16,
  "tilesets": [{{ "firstgid": 1, "columns": 8, "name": "tiles" }}],
  "layers": [
    {{ "name": "terrain", "type": "tilelayer", "width": 3, "height": 2, "data": [0,0,0, 2,2147483650,3] }},
    {{ "name": "collision", "type": "tilelayer", "width": 3, "height": 2, "data": [0,0,0, 1,1,1] }},
    {{ "name": "coins", "type": "tilelayer", "width": 3, "height": 2, "data": [0,9,0, 0,0,0] }}
    {layers_extra},
    {{ "name": "objects", "type": "objectgroup", "objects": [{objects}] }}
  ]
}}"#
    )
}

const OBJS: &str = r#"
  { "name": "", "type": "player_spawn", "x": 0, "y": 0, "width": 16, "height": 16 },
  { "class": "goal", "x": 32, "y": 0, "width": 16, "height": 16 },
  { "name": "enemy_spawn", "x": 16, "y": 16, "point": true, "width": 0, "height": 0 }
"#;

#[test]
fn parses_layers_objects_and_masks_flip_flags() {
    let d = parse_tiled_json(&map("", OBJS)).unwrap();
    assert_eq!((d.width, d.height, d.tile_size), (3, 2, 16.0));
    assert_eq!(d.collision, vec![0, 0, 0, 1, 1, 1]);
    assert_eq!(d.coins, vec![0, 1, 0, 0, 0, 0]);
    assert_eq!(d.terrain, vec![0, 0, 0, 2, 2, 3], "flip flags masked");
    assert_eq!((d.spawns.player.x, d.spawns.player.y), (8.0, 16.0), "feet = bottom center");
    assert_eq!((d.spawns.goal.x, d.spawns.goal.y), (40.0, 16.0), "class supported");
    assert_eq!(d.spawns.enemies.len(), 1);
    assert_eq!((d.spawns.enemies[0].x, d.spawns.enemies[0].y), (16.0, 16.0), "point objects");
}

#[test]
fn missing_layer_is_a_clear_error() {
    let text = map("", OBJS).replace("\"coins\"", "\"gold\"");
    let e = parse_tiled_json(&text).unwrap_err();
    assert!(e.contains("missing layer `coins`"), "{e}");
}

#[test]
fn missing_player_spawn_is_a_clear_error() {
    let objs = OBJS.replace("player_spawn", "something_else");
    let e = parse_tiled_json(&map("", &objs)).unwrap_err();
    assert!(e.contains("player_spawn"), "{e}");
}

#[test]
fn wrong_layer_size_is_rejected() {
    let text = map("", OBJS).replace("[0,0,0, 1,1,1]", "[0,0,1,1,1]");
    let e = parse_tiled_json(&text).unwrap_err();
    assert!(e.contains("collision") && e.contains("expected 6"), "{e}");
}

#[test]
fn base64_layers_are_rejected_with_hint() {
    let text = map("", OBJS).replace(
        r#""name": "coins", "type": "tilelayer","#,
        r#""name": "coins", "type": "tilelayer", "encoding": "base64","#,
    );
    let e = parse_tiled_json(&text).unwrap_err();
    assert!(e.contains("CSV"), "{e}");
}

#[test]
fn invalid_json_is_reported() {
    assert!(parse_tiled_json("{ nope").unwrap_err().contains("invalid JSON"));
}
