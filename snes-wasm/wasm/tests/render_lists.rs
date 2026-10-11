mod common;

use common::*;
use snes_wasm::render::retro::{world_sprites, SpriteKind};
use snes_wasm::render::{draw_game, RenderList};

#[test]
fn hud_is_drawn_in_its_own_list() {
    let g = playing(&["..P.o..E.......", "###############"]);
    let (mut retro, mut hud) = (RenderList::new(320.0, 180.0), RenderList::new(320.0, 180.0));
    draw_game(&g, &mut retro, &mut hud);
    assert!(!retro.verts.is_empty() && !hud.verts.is_empty());
    // HUD uses only the font texture (id 4); the world never does.
    assert!(hud.batches.chunks(3).all(|b| b[0] == 4));
    assert!(retro.batches.chunks(3).all(|b| b[0] != 4));
}

#[test]
fn world_sprites_list_the_hero_once() {
    let g = playing(&["..P.o..E.......", "###############"]);
    let s = world_sprites(&g);
    assert_eq!(s.iter().filter(|s| s.kind == SpriteKind::Hero).count(), 1);
    assert!(s.iter().any(|s| s.kind == SpriteKind::Entity), "coin and enemy");
}
