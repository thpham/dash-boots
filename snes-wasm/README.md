# Dash Boots — SNES-style platformer (Rust → WASM)

A 16-bit style platformer that runs in the browser. All the game lives in a
Rust crate compiled to WebAssembly; the TypeScript side is a thin platform
layer (assets, input, WebGL2, WebAudio).

## Campaign

Four levels of rising (but gentle) difficulty, each with its own mood — all four
set in the Swiss Alps: a mountaineer hero, summit flags on the horizon, chalets
and grazing herds in the near hills.

| Level | Theme | Signature |
|---|---|---|
| 1 — Meadow | alpine meadow | tutorial ground: gentle pits, one crawl tunnel, coin hideout, no goats |
| 2 — Sunset | golden evening | plank bridges, terraces with a wheel-horn goat, a marmot corridor under a coin shower |
| 3 — Caverns | the fromagerie (cheese-brick gallery) | crawl tunnels, a goat pen, a guarded coin stash |
| 4 — Night | moonlit frost | two goats guarding the double crawl, a five-cow herd, everything combined |

Each level has its own embedded tracker song (`wasm/assets/*.xm`, rendered
in-wasm — attribution table above) and tileset/parallax palette (exact color
remaps of the meadow art — same tile meaning everywhere, different clothes).
Finishing a level flows straight into the next; the whole run is saved in
`localStorage` (`dash-boots-progress`), so refreshing the page resumes at the
right level. Progress storage is skipped when a `?level=` override is used.

## Endless mode (generated levels)

The title screen offers **CAMPAIGN** or **ENDLESS** (↑/↓ to choose, ←/→ for the
start difficulty D1–D5, Enter/Space to start) and **CONTINUE RUN n** when a run
is saved. An endless run never ends: every cleared level generates the next one,
the difficulty rises by one step every two levels (up to D5) and the theme cycles
meadow → sunset → cave → night with its music. The HUD shows `RUN n Dd`, the clear
screen your best score (levels cleared) and what comes next.

- **Share a run:** the pause screen shows the run seed and start difficulty;
  `?seed=N&start=D` opens the title with that run preselected — the same seed
  gives the same levels in any browser.
- **Quit a run:** pause, then `T`. A refresh mid-run resumes it (CONTINUE).
- Storage: `dash-boots-run` (current run) and `dash-boots-best`, next to the
  untouched campaign progress.

### How levels are generated

Everything happens in the wasm, with zero network fetch:

1. A level is written as `<theme> <difficulty>` followed by ~25 **motifs**
   (`gap3`, `walk2`, `marsh`, `tunnel5`, `rift_out`, …) — `wasm/src/levelgen/motif.rs`.
2. A **tiny transformer** (460k parameters, ~480 KB of int8 weights embedded in
   the wasm, `wasm/assets/levelgen.bin`) samples the motifs one by one, ~8 ms per
   level. A **grammar** masks every motif that would be illegal where it stands
   (height bounds, cave ceiling and jump clearance, intro zone, checkpoint spacing,
   paired rift excursions with their runways, one pair of boots, length) — and it
   is proven never to dead-end (`compile.rs`).
3. The motifs **compile** to tiles and a regular Tiled map.
4. A **bot** of the requested skill must finish it; otherwise the model tries
   again (8 tries), then the section teacher, then the campaign level of that
   theme — there is always a level (`run.rs`).

Five graded bots (novice → expert, `bot.rs`) are both the critic and the
difficulty meter: a level's difficulty is the weakest bot that finishes it.
D1..D5 are therefore *bot* skills, ordered like the campaign but not calibrated
on human players.

### Retraining the level model

```
just levelgen-corpus      # ~20k bot-labelled levels: campaign mutations + section teacher (tools/levelgen/, not committed)
just levelgen-report      # where the bots fail, per motif and skill (tune teacher.rs with it)
just levelgen-train       # supervised pre-training (nix develop .#training, CPU PyTorch, ~2 min)
just levelgen-rl          # GRPO against the bots + int8 export gated on acceptance (~12 min)
just levelgen-eval        # acceptance per (theme, difficulty), monotonic difficulty, latency
```

The training shell (`nix develop .#training`) is the game shell plus CPU
PyTorch; the default shell and CI never pull it. **Editing `motif.rs` changes
the vocabulary: rerun the whole sequence** — until the weights match, the game
falls back to the teacher.

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

Gameplay: collect coins, stomp the marmots, grab the **dash boots** (faster run, a
dash burst that knocks enemies out, and they absorb one hit), slide through low
tunnels, touch checkpoints, reach the flag. Jump and bump a box from below and
it bounces, launching the marmots on top: they land on their backs (harmless,
kick them to finish them) and evaporate a moment later.

From level 2 on, the patrols include **cheese-carrying goats**: stomp one and it
sits dazed, dropping its cheese wheel — pick it up, carry it (it rides in front
of the hero), and throw it: the wheel rolls and crushes every marmot in its
path. The decor is alive too: holstein cows graze the flats, and a player who
walks up to one gets a single moo.

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
| `just levelgen-*` | endless-mode level model: corpus, report, train, rl, eval (see *Endless mode*) |

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
  color remaps of the meadow originals: summit flags, chalets and goat herds on
  the near ridge, a starfield and moon for the night, edelweiss and trail-sign
  tiles, the mountaineer hero, marmots, goats, cheese wheels and holstein cows
  in the atlas. The atlas layout is mirrored in
  `wasm/src/sprites.rs`; the future-3D sky keying constrains the themed skies
  (asserted at generation time).
- `wasm/assets/*.xm` — the four campaign songs, embedded in the wasm binary
  and rendered at 22050 Hz by libxm (attribution table above).
- `tools/gen_level.py` → `public/levels/level{1,2,3,4}.json` plus the small
  `test_rift.json`, regular **Tiled** maps (embedded tileset) you can open and
  inspect in Tiled. The generator is the source of truth: a shipped-level test
  pins every level byte-for-byte against its output (`gen_level.py --out <dir>`
  regenerates elsewhere), so keep level edits in the Python builders and
  re-run `just level` — a hand edit to the JSON fails the byte-pin test.

Tiled conventions (see the spec, §5.2): orthogonal, 16×16, CSV layer data; tile layers
`terrain`, `collision` (gid > 0 = solid), `coins` (gid > 0 = coin); object layer
`objects` with `player_spawn`, `goal`, `enemy_spawn`, `goat_spawn`, `cow`,
`checkpoint`, `dash_boots`, `rift`
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
