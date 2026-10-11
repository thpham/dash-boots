// Browser host. All game logic, level parsing and render-list building live in
// the Rust/WASM core; this file only wires the platform: assets, input, GL, audio.

import initWasm, { GameHandle, render_sfx } from '../wasm/pkg/snes_wasm';
import { createAudio, VOLUME_STEP } from './platform/audio';
import { Executor, type TextureSpec } from './platform/gl/executor';
import { createInput } from './platform/input';
import { FutureCap } from './platform/perf';
import { PLAN, type FrameData } from './platform/plan';

const VIEW_W = 320;
const VIEW_H = 180;
const BASE = import.meta.env.BASE_URL;
// Reserved code (wasm/src/audio.rs LEVEL_CLEAR): the clear screen is done and
// the host should move on (next level, or replay on the last one).
const LEVEL_CLEAR_CODE = 250;
const SAVE_KEY = 'dash-boots-progress';

// The campaign: one level per entry, in order. Texture ids 0/1/2 (parallax +
// tiles) are theme-dependent; sprites and font stay shared.
type Theme = 'meadow' | 'sunset' | 'cave' | 'night';
const THEME_FILES: Record<Theme, { bgFar: string; bgNear: string; tiles: string }> = {
  meadow: { bgFar: 'bg_far.png', bgNear: 'bg_near.png', tiles: 'tiles.png' },
  sunset: { bgFar: 'bg_far_sunset.png', bgNear: 'bg_near_sunset.png', tiles: 'tiles_sunset.png' },
  cave: { bgFar: 'bg_far_cave.png', bgNear: 'bg_near_cave.png', tiles: 'tiles_cave.png' },
  night: { bgFar: 'bg_far_night.png', bgNear: 'bg_near_night.png', tiles: 'tiles_night.png' },
};
const LEVELS: { name: string; theme: Theme }[] = [
  { name: 'level1', theme: 'meadow' },
  { name: 'level2', theme: 'sunset' },
  { name: 'level3', theme: 'cave' },
  { name: 'level4', theme: 'night' },
];

function loadProgress(): number {
  try {
    const saved = parseInt(localStorage.getItem(SAVE_KEY) ?? '', 10);
    if (Number.isInteger(saved) && saved >= 0 && saved < LEVELS.length) return saved;
  } catch {
    // Storage unavailable (private mode…): start from the first level.
  }
  return 0;
}

function saveProgress(levelIndex: number) {
  try {
    localStorage.setItem(SAVE_KEY, String(levelIndex));
  } catch {
    // Ignore: progression simply won't survive a refresh.
  }
}

function levelUrl(index: number, name: string): string {
  return `${BASE}levels/${(index >= 0 ? LEVELS[index].name : name)}.json`;
}

/** Boot level: `?level=<name>` wins (any shipped level, e.g. the test ones,
 * run standalone on the meadow theme), else the saved progression. */
function startLevel(): { name: string; index: number } {
  const name = new URLSearchParams(location.search).get('level');
  if (name !== null) {
    if (!/^[a-z0-9_-]+$/.test(name)) throw new Error(`Invalid level name: ${name}`);
    return { name, index: LEVELS.findIndex((l) => l.name === name) };
  }
  const index = loadProgress();
  return { name: LEVELS[index].name, index };
}

/** Theme-dependent textures (ids 0/1/2) then the shared ones (sprites, font). */
function texturesFor(theme: Theme): TextureSpec[] {
  const files = THEME_FILES[theme];
  return [
    { id: 0, url: `${BASE}assets/${files.bgFar}`, repeatX: true },
    { id: 1, url: `${BASE}assets/${files.bgNear}`, repeatX: true },
    { id: 2, url: `${BASE}assets/${files.tiles}` },
    { id: 3, url: `${BASE}assets/sprites.png` },
    { id: 4, url: `${BASE}assets/font.png` },
  ];
}

async function fetchText(url: string): Promise<string> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to load ${url}: HTTP ${res.status}`);
  return res.text();
}

function showError(message: string) {
  const el = document.querySelector<HTMLElement>('#status');
  if (el) {
    el.textContent = message;
    el.classList.add('error');
    el.hidden = false;
  }
}

/** Transient feedback (volume keys, mute): fades out on its own. */
let toastTimer: number | undefined;

function showToast(text: string) {
  const el = document.querySelector<HTMLElement>('#toast');
  if (!el) return;
  el.textContent = text;
  el.classList.add('show');
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => el.classList.remove('show'), 1400);
}

async function main() {
  const canvas = document.querySelector<HTMLCanvasElement>('#game');
  if (!canvas) throw new Error('Missing #game canvas');

  const wasm = await initWasm();
  const executor = new Executor(canvas);
  const motion = window.matchMedia('(prefers-reduced-motion: reduce)');

  let crt = true;
  let viewport = { w: VIEW_W, h: VIEW_H };
  // Set once the boot level is built below; handlers created meanwhile guard
  // with `?.` so a blur before boot is a harmless no-op.
  let game: GameHandle;
  const bootLevel = startLevel();
  // index -1: a standalone ?level= launch (test levels); it replays itself.
  let currentLevel = bootLevel.index;
  let swapping = false;
  const launchedFromQuery = new URLSearchParams(location.search).has('level');
  const cap = new FutureCap((maxHeight) => game?.set_max_future_height(maxHeight));

  // Fetching a level and building its handle: JSON, textures, music. Every
  // setting lives in the handle, so a swap re-applies them (configure()).
  const buildGame = async (index: number, name: string) => {
    const theme = index >= 0 ? LEVELS[index].theme : 'meadow';
    const files = texturesFor(theme);
    const text = await fetchText(levelUrl(index, name));
    const handle = new GameHandle(text, VIEW_W, VIEW_H);
    if (index >= 0) handle.set_music(index); // in-wasm track id (rendered lazily by audio.ts)
    handle.set_last_level(index < 0 || index === LEVELS.length - 1);
    const sizes = await executor.loadTextures(files);
    files.forEach((f, i) => handle.set_texture_size(f.id, sizes[i].w, sizes[i].h));
    configure(handle);
    return { handle, sizes };
  };
  const configure = (game: GameHandle) => {
    game.set_viewport(viewport.w, viewport.h);
    game.set_future_supported(executor.futureReady);
    game.set_reduced_motion(motion.matches);
    game.set_max_future_height(cap.current());
    game.set_crt_enabled(crt);
  };

  const boot = await buildGame(currentLevel, bootLevel.name);
  game = boot.handle;

  // If the future pipeline is unavailable, the game stays in the past world.
  // Theme tilesets keep identical dimensions, so no re-call on level swaps.
  const [tileSize, spriteSize] = [boot.sizes[2], boot.sizes[3]];
  game.set_future_supported(executor.enableFuture([tileSize.w, tileSize.h], [spriteSize.w, spriteSize.h]));

  const audio = createAudio({ sfx: render_sfx });
  const input = createInput();

  // Clear screen done: the sim emitted LEVEL_CLEAR. Advance (or replay on the
  // last level and on standalone ?level= launches); save progress so a
  // refresh resumes at the right level.
  const advance = async () => {
    if (currentLevel < 0 || currentLevel + 1 >= LEVELS.length) {
      game.restart(); // replay (existing clear-screen behaviour)
      return;
    }
    const next = currentLevel + 1;
    if (swapping) return;
    swapping = true;
    try {
      const built = await buildGame(next, LEVELS[next].name);
      currentLevel = next;
      game = built.handle;
      if (!launchedFromQuery) saveProgress(next);
      game.restart(); // into play directly: the player just finished one
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    } finally {
      swapping = false;
    }
  };

  // Browsers only allow audio after a user gesture.
  const unlock = () => audio.unlock();
  window.addEventListener('keydown', unlock);
  window.addEventListener('pointerdown', unlock);
  window.addEventListener('keydown', (e) => {
    // G/H shifts the music volume, V/B the effects — plain letter keys (no
    // shift combos), and the same physical positions on AZERTY and QWERTY.
    // Key repeat is allowed here: holding the key ramps the volume.
    if (e.code === 'KeyG' || e.code === 'KeyH') {
      const vol = audio.adjustMusic(e.code === 'KeyH' ? VOLUME_STEP : -VOLUME_STEP);
      showToast(`Music ${Math.round(vol * 100)}%`);
    } else if (e.code === 'KeyV' || e.code === 'KeyB') {
      const vol = audio.adjustSfx(e.code === 'KeyB' ? VOLUME_STEP : -VOLUME_STEP);
      showToast(`Effects ${Math.round(vol * 100)}%`);
    }
    if (e.repeat) return;
    if (e.code === 'KeyM') {
      showToast(audio.toggleMute() ? 'Muted' : 'Sound on');
    }
    if (e.code === 'KeyC') {
      crt = !crt;
      game?.set_crt_enabled(crt);
    }
  });

  const pause = () => {
    input.reset();
    game?.pause();
  };
  window.addEventListener('blur', pause);
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) pause();
  });
  const resize = () => {
    const { w, h } = executor.resize();
    viewport = { w, h };
    game?.set_viewport(w, h);
  };
  window.addEventListener('resize', resize);
  resize();

  document.querySelector<HTMLElement>('#status')!.hidden = true;
  canvas.hidden = false;

  let last = performance.now();
  const frame = (now: number) => {
    const dt = now - last;
    last = now;
    game.frame(input.poll(), dt);
    const codes = game.take_audio_events();
    if (codes.includes(LEVEL_CLEAR_CODE)) void advance();
    audio.handle(codes.filter((c) => c !== LEVEL_CLEAR_CODE));
    // Views are re-created every frame: WASM memory may grow and move.
    const mem = wasm.memory.buffer;
    const frameData: FrameData = {
      retroVerts: new Float32Array(mem, game.vertices_ptr(), game.vertices_len()),
      retroBatches: new Uint32Array(mem, game.batches_ptr(), game.batches_len()),
      hudVerts: new Float32Array(mem, game.hud_vertices_ptr(), game.hud_vertices_len()),
      hudBatches: new Uint32Array(mem, game.hud_batches_ptr(), game.hud_batches_len()),
      plan: new Float32Array(mem, game.plan_ptr(), game.plan_len()),
      blocks: new Float32Array(mem, game.blocks_ptr(), game.blocks_len()),
      billboards: new Float32Array(mem, game.billboards_ptr(), game.billboards_len()),
      shadows: new Float32Array(mem, game.shadows_ptr(), game.shadows_len()),
    };
    executor.render(frameData);
    cap.sample(dt, frameData.plan[PLAN.WORLD_MIX] > 0);
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

main().catch((err: unknown) => {
  console.error(err);
  showError(err instanceof Error ? err.message : String(err));
});