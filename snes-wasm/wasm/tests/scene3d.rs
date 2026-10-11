mod common;

use common::*;
use snes_wasm::render::scene3d::*;
use snes_wasm::world::shift::{Shift, World};

fn future_params() -> snes_wasm::world::shift::ShiftParams {
    Shift::new(World::Future).params(false)
}

fn long_level() -> Vec<String> {
    let mut top = ".".repeat(120);
    top.replace_range(2..3, "P");
    top.replace_range(6..7, "E");
    vec![".".repeat(120), top, "#".repeat(120)]
}

#[test]
fn only_blocks_near_the_camera_are_sent() {
    let rows = long_level();
    let rows: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    let g = playing(&rows);
    let mut s = Scene3d::default();
    build_scene(&g, &future_params(), &mut s);
    // Hero at column 2: columns 0..=42 of the single ground row.
    assert_eq!(s.block_count(), 43);
    assert_eq!(s.blocks.len(), s.block_count() * BLOCK_FLOATS);
}

#[test]
fn every_column_of_the_2d_view_is_in_the_scene_at_the_swap() {
    // At the 2D->3D swap the 2D camera shows up to ~10 columns behind the hero
    // (deadzone up to 0.48 * 320 px), past VISIBLE_BEHIND_TILES. The ground
    // strip must not be missing there.
    let w = 60;
    let mut rows = vec![".".repeat(w); 3];
    rows[0].replace_range(30..31, "P");
    rows[2] = "#".repeat(w);
    let rows: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    let mut g = playing(&rows);
    let feet = g.player.feet();
    g.camera.x = feet.x - 153.0; // hero sits just inside the right deadzone edge
    let ts = TS;
    let mut s = Scene3d::default();
    build_scene(&g, &future_params(), &mut s);
    let first = (g.camera.x / ts).floor() as i32;
    let last = ((g.camera.x + g.camera.view_w) / ts).floor() as i32;
    let mut present: Vec<i32> = Vec::new();
    for b in s.blocks.chunks(BLOCK_FLOATS) {
        present.push(((b[0] / ts) - 0.5).floor() as i32); // block centre x -> column
    }
    for c in first..=last {
        assert!(present.contains(&c), "ground missing in column {c} ({first}..={last} visible)");
    }
}

#[test]
fn ground_blocks_form_a_road_and_extrude_with_the_wave() {
    let rows = long_level();
    let rows: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    let g = playing(&rows);
    let mut s = Scene3d::default();
    let mut p = future_params();
    p.extrude = 0.0;
    build_scene(&g, &p, &mut s);
    assert!(s.blocks.chunks(BLOCK_FLOATS).all(|b| b[5] == 0.0 && b[7] == 0.0), "flat before the wave");
    build_scene(&g, &future_params(), &mut s);
    assert!(s.blocks.chunks(BLOCK_FLOATS).all(|b| (b[5] - ROAD_HALF_WIDTH).abs() < 1e-3), "full road");
}

#[test]
fn extrusion_wave_spreads_from_the_hero() {
    assert_eq!(block_extrusion(0.0, 0.0), 0.0);
    assert!(block_extrusion(0.3, 2.0) > block_extrusion(0.3, 10.0));
    assert_eq!(block_extrusion(1.0, 39.0), 1.0);
    for d in 0..40 {
        let mut prev = 0.0;
        for i in 0..=20 {
            let e = block_extrusion(i as f32 / 20.0, d as f32);
            assert!(e >= prev && (0.0..=1.0).contains(&e));
            prev = e;
        }
    }
}

#[test]
fn hero_keeps_its_colors_while_entities_are_holograms() {
    let g = playing(&["..............", "..P...E.......", "##############"]);
    let mut s = Scene3d::default();
    build_scene(&g, &future_params(), &mut s);
    let flags: Vec<u32> = s.billboards.chunks(BILLBOARD_FLOATS).map(|b| b[10] as u32).collect();
    assert_eq!(flags.iter().filter(|f| **f & FLAG_HOLO == 0).count(), 1, "only the hero is not holographic");
}

#[test]
fn shadows_under_grounded_entities_but_not_over_pits() {
    let g = playing(&["..............", "..P...E.......", "##############"]);
    let mut s = Scene3d::default();
    build_scene(&g, &future_params(), &mut s);
    assert_eq!(s.shadow_count(), 2, "hero and enemy");
    let mut g = playing(&["..............", "..P...........", "##.###########"]);
    g.player.pos.x = 2.0 * TS + 3.0; // above the pit at column 2
    g.player.pos.y = 0.0;
    build_scene(&g, &future_params(), &mut s);
    assert_eq!(s.shadow_count(), 0);
}
