mod common;

use common::*;
use snes_wasm::input::InputState;
use snes_wasm::render::retro::{world_sprites, SpriteKind};
use snes_wasm::render::{draw_game, RenderList};
use snes_wasm::sprites::RIFT;

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

fn rift_sprites(g: &snes_wasm::game::Game) -> Vec<snes_wasm::render::retro::WorldSprite> {
    world_sprites(g).into_iter().filter(|s| RIFT.contains(&s.frame)).collect()
}

#[test]
fn active_rifts_are_drawn_and_used_ones_are_not() {
    let mut g = playing(&["..P....R...........", "###################"]);
    let s = rift_sprites(&g);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].kind, SpriteKind::Entity);
    // Anchored bottom-centre on the rift's feet.
    let feet = g.rifts[0].feet;
    assert_eq!((s[0].x + s[0].frame.w * 0.5, s[0].y + s[0].frame.h), (feet.x, feet.y));
    run_until(&mut g, InputState { right: true, ..input() }, 300, |g| !g.rifts[0].active).expect("rift used");
    assert!(rift_sprites(&g).is_empty(), "a used rift is not drawn");
}
