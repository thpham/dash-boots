# Dash Boots — SNES-style platformer (Rust → WASM)

A 16-bit style platformer that runs in the browser. All the game lives in a
Rust crate compiled to WebAssembly; the TypeScript side is a thin platform
layer (assets, input, WebGL2, WebAudio).

## Campaign

Four levels of rising (but gentle) difficulty, each with its own mood:

| Level | Theme | Signature |
|---|---|---|
| 1 — Meadow | green meadow | tutorial ground: gentle pits, one crawl tunnel, 4 rifts |
| 2 — Sunset | golden evening | plank bridges over pits, terraces, two excursions |
| 3 — Caverns | dark stone, moss & crystals | ceilings and crawl tunnels, longer no-light sections |
| 4 — Night | moonlit frost | everything combined, more enemies, final climb |

Each level has its own embedded tracker song (`wasm/assets/*.xm`, rendered
in-wasm — attribution table above) and tileset/parallax palette (exact color
remaps of the meadow art — same tile meaning everywhere, different clothes).
Finishing a level flows straight into the next; the whole run is saved in
`localStorage` (`dash-boots-progress`), so refreshing the page resumes at the
right level. Progress storage is skipped when a `?level=` override is used.

## Play

From the repo root (direnv loads the nix devShell automatically, or run `nix develop` first):

```bash
just dev        # builds the WASM, then http://localhost:5173
just serve      # production build, then http://localhost:4173
```

`just build` writes a static site to `snes-wasm/dist/` that can be hosted anywhere,
including under a sub-path (relative asset URLs). It must be served over HTTP
(`file://` cannot load the WASM).

A published copy also lives on GitHub Pages at
https://thpham.github.io/dash-boots/. The `.github/workflows/deploy-pages.yml`
workflow republishes it on pushed `v*` tags (`git tag v0.1.1 && git push
--tags`) and on manual runs via workflow_dispatch on the Actions tab.

### Controls

| Action | Keyboard | Gamepad (standard mapping) |
|---|---|---|
| Move | ←/→ or A/D | D-pad / left stick |
| Jump (hold = higher) | Space, K, Z | bottom / right face button |
| Run, + dash with boots | Shift, J, X | left / top face button, shoulders |
| Crouch / slide (crawl in tunnels) | ↓ or S | D-pad down |
| Start / pause | Enter, Esc, P | Start |
| Mute | M | — |
| Music volume down/up | G / H | — |
| Effects volume down/up | V / B | — |
| CRT filter on/off | C | — |

Gameplay: collect coins, stomp Grumbles, grab the **dash boots** (faster run, a
dash burst that knocks enemies out, and they absorb one hit), slide through low
tunnels, touch checkpoints, reach the flag. Jump and bump a box from below and
it bounces, launching the Grumbles on top: they land on their backs (harmless,
kick them to finish them) and evaporate a moment later.

## Tasks (`just --list`)

| Task | What it does |
|---|---|
| `just dev` / `serve` / `build` | dev server / preview of the prod build / prod build |
| `just wasm-test` | Rust tests: physics, rules, Tiled parser, a bot that completes levels 1-4 with (almost) no deaths |
| `just lint` | rustfmt check, clippy (`-D warnings`), TypeScript typecheck |
| `just e2e` | headless Chromium smoke test of the prod build (screenshots in `test-results/`) |
| `just check` | all of the above |
| `just assets` | regenerate the art from `tools/` (audio is synthesized in the wasm) |
| `just level` | regenerate all `level*.json` from `tools/gen_level.py` (overwrites Tiled edits) |

## Architecture

```
TypeScript (src/)                      Rust → WASM (wasm/src/)
─────────────────                      ───────────────────────
fetch level1.json (text) ────────────► tiled.rs     Tiled JSON → LevelData (clear errors)
keyboard/gamepad → u32 bits ─────────► input.rs     press edges, tap latching
rAF dt (ms) ─────────────────────────► game.rs      fixed 60 Hz steps, phases (title/play/dying/clear/pause)
                                        world/       player, enemies, physics, camera, items, particles
WebGL2: 1 draw call per batch ◄─────── render.rs    vertex buffer (parallax, tiles, sprites, HUD text)
WebAudio buses ◄────────────────────── audio.rs     u8 cue stream (SFX ids, music play/pause/stop)
WebGL2 passes: past RT, future RT, ◄── render/scene3d.rs, render/camera3d.rs, render/plan.rs
  bloom, composite, HUD
```

- The render list is read straight from WASM memory (`vertices_ptr/len`,
  `batches_ptr/len`): no per-frame serialization.
- Physics: AABB vs tiles, X then Y, sub-stepped (≤ 4 px) so nothing tunnels;
  coyote time, jump buffer, variable jump height, ledge assist and ceiling corner correction.
- Determinism: the simulation only advances in fixed 1/60 s steps; long frames are clamped.

## Past ↔ future ("Mémoire vive")

Rifts (`rift` objects in Tiled) shift the world between the 16-bit past and a
holographic 3D "reconstructed memory". The simulation stays 2D; only the
presentation pivots (camera 3/4 rear, extruded glass blocks, bloom), then the
past comes back pixelated, warm and CRT-filtered.

- Level rule: the 6 columns right of a rift must be flat ground with no enemy spawn
  (checked at load time).
- `?level=test_rift` loads a tiny level with a rift right after the start.
- Respects `prefers-reduced-motion` (simple crossfade) and lowers the future
  resolution automatically on slow GPUs. If 3D setup fails, the game stays in the past.
- Design: `docs/superpowers/specs/2026-10-11-time-shift-engine-design.md`.

## Assets & level

All assets are original and generated by stdlib-only Python scripts:

- `tools/gen_art.py` → `public/assets/*.png` (sprite atlas, tileset, font, parallax
  layers), plus the three theme sets (`*_sunset/cave/night.png`) built by exact
  color remaps of the meadow originals. The atlas layout is mirrored in
  `wasm/src/sprites.rs`; the future-3D sky keying constrains the themed skies
  (asserted at generation time).
- `wasm/assets/*.xm` — the four campaign songs, embedded in the wasm binary
  and rendered at 22050 Hz by libxm (attribution table above).
- `tools/gen_level.py` → `public/levels/level{1,2,3,4}.json` plus the small
  `test_rift.json`, regular **Tiled** maps (embedded tileset) that can be opened
  and edited in Tiled. Once you edit a level in Tiled, treat the JSON as the
  source of truth and stop running `just level`.

Tiled conventions (see the spec, §5.2): orthogonal, 16×16, CSV layer data; tile layers
`terrain`, `collision` (gid > 0 = solid), `coins` (gid > 0 = coin); object layer
`objects` with `player_spawn`, `goal`, `enemy_spawn`, `checkpoint`, `dash_boots`
(matched on class/type, falling back to name; objects are anchored bottom-center).
After editing the level, `just wasm-test` checks it still loads and that the bot can finish it.

## Audio

Everything audio lives in the WASM — no audio files are fetched. SFX are
synthesized in Rust (port of the former `tools/gen_audio.py` recipes,
golden-checked against it); the four campaign songs are tracker modules
rendered at 22050 Hz by [libxm](https://github.com/nukep/libxm) (WTFPL),
loop-point detected by order-jump simulation and peak-normalized to 0.95.
See `wasm/src/music/` (`render_music`, `render_sfx`).

### Music attribution

These four modules come from the
[xm-files-for-player](https://github.com/thunder-red-star/xm-files-for-player)
demoscene collection, which declares **no license on any file**. They were
embedded at the maintainer's explicit request, with attribution preserved:

| Level | Module | Author | Source |
|-------|--------|--------|--------|
| 1 | Super Mario Bros 3 (arrangement) | jellydog | [jellydog-super_mario_bros3.xm](https://github.com/thunder-red-star/xm-files-for-player/blob/main/jellydog-super_mario_bros3.xm) |
| 2 | Summer Memories #3 | Arachno & Dualtrax | [arachno&dualtrax-summer_memories3.xm](https://github.com/thunder-red-star/xm-files-for-player/blob/main/arachno%26dualtrax-summer_memories3.xm) |
| 3 | Shock Therapy | Chromag | [chromag-shock_therapy23.xm](https://github.com/thunder-red-star/xm-files-for-player/blob/main/chromag-shock_therapy23.xm) |
| 4 | Bar Rocks | Cerror | [cerror-bar_rocks.xm](https://github.com/thunder-red-star/xm-files-for-player/blob/main/cerror-bar_rocks.xm) |

Notes: the collection's stated purpose is distribution/demo of an XM player
and the authors are credited above; if an original author asks us to, we
replace their module. Level 1 has the extra caveat that the Super Mario Bros 3
name and melody belong to Nintendo — same maintainer's decision.
