// Browser host. All game logic, level parsing and render-list building live in
// the Rust/WASM core; this file only wires the platform: assets, input, GL, audio.

import initWasm, { GameHandle, music_names, sfx_names } from '../wasm/pkg/snes_wasm';
import { createAudio } from './platform/audio';
import { Executor, type TextureSpec } from './platform/gl/executor';
import { createInput } from './platform/input';
import { FutureCap } from './platform/perf';
import { PLAN, type FrameData } from './platform/plan';

const VIEW_W = 320;
const VIEW_H = 180;
const BASE = import.meta.env.BASE_URL;

// Texture ids must match wasm/src/render/mod.rs (TEX_*).
const TEXTURES: TextureSpec[] = [
  { id: 0, url: `${BASE}assets/bg_far.png`, repeatX: true },
  { id: 1, url: `${BASE}assets/bg_near.png`, repeatX: true },
  { id: 2, url: `${BASE}assets/tiles.png` },
  { id: 3, url: `${BASE}assets/sprites.png` },
  { id: 4, url: `${BASE}assets/font.png` },
];

function showError(message: string) {
  const el = document.querySelector<HTMLElement>('#status');
  if (el) {
    el.textContent = message;
    el.classList.add('error');
    el.hidden = false;
  }
}

async function fetchText(url: string): Promise<string> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to load ${url}: HTTP ${res.status}`);
  return res.text();
}

function levelUrl(): string {
  const name = new URLSearchParams(location.search).get('level') ?? 'level1';
  if (!/^[a-z0-9_-]+$/.test(name)) throw new Error(`Invalid level name: ${name}`);
  return `${BASE}levels/${name}.json`;
}

async function main() {
  const canvas = document.querySelector<HTMLCanvasElement>('#game');
  if (!canvas) throw new Error('Missing #game canvas');

  const [wasm, levelJson] = await Promise.all([initWasm(), fetchText(levelUrl())]);
  const executor = new Executor(canvas);
  const game = new GameHandle(levelJson, VIEW_W, VIEW_H);

  const sizes = await executor.loadTextures(TEXTURES);
  TEXTURES.forEach((t, i) => game.set_texture_size(t.id, sizes[i].w, sizes[i].h));
  const size = (id: number) => sizes[TEXTURES.findIndex((t) => t.id === id)];
  game.set_future_supported(executor.enableFuture([size(2).w, size(2).h], [size(3).w, size(3).h]));
  // If the future renderer throws at render time, fall back to the past world.
  executor.onFutureLost = () => game.set_future_supported(false);

  const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
  game.set_reduced_motion(motion.matches);
  motion.addEventListener('change', () => game.set_reduced_motion(motion.matches));
  const cap = new FutureCap((h) => game.set_max_future_height(h));

  const audio = createAudio(BASE, sfx_names(), music_names());
  const input = createInput();

  // Browsers only allow audio after a user gesture.
  const unlock = () => audio.unlock();
  window.addEventListener('keydown', unlock);
  window.addEventListener('pointerdown', unlock);
  let crt = true;
  window.addEventListener('keydown', (e) => {
    if (e.repeat) return;
    if (e.code === 'KeyM') audio.toggleMute();
    if (e.code === 'KeyC') {
      crt = !crt;
      game.set_crt_enabled(crt);
    }
  });

  const pause = () => {
    input.reset();
    game.pause();
  };
  window.addEventListener('blur', pause);
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) pause();
  });
  const resize = () => {
    const { w, h } = executor.resize();
    game.set_viewport(w, h);
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
    audio.handle(game.take_audio_events());
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
