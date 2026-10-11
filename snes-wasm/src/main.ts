// Browser host. All game logic, level parsing and render-list building live in
// the Rust/WASM core; this file only wires the platform: assets, input, GL, audio.

import initWasm, { GameHandle, music_names, sfx_names } from '../wasm/pkg/snes_wasm';
import { createAudio } from './platform/audio';
import { Renderer, type TextureSpec } from './platform/gl';
import { createInput } from './platform/input';

const VIEW_W = 320;
const VIEW_H = 180;
const BASE = import.meta.env.BASE_URL;

// Texture ids must match wasm/src/render.rs (TEX_*).
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

async function main() {
  const canvas = document.querySelector<HTMLCanvasElement>('#game');
  if (!canvas) throw new Error('Missing #game canvas');

  const [wasm, levelJson] = await Promise.all([initWasm(), fetchText(`${BASE}levels/level1.json`)]);
  const renderer = new Renderer(canvas, VIEW_W, VIEW_H);
  const game = new GameHandle(levelJson, VIEW_W, VIEW_H);

  const sizes = await Promise.all(TEXTURES.map((t) => renderer.loadTexture(t)));
  TEXTURES.forEach((t, i) => game.set_texture_size(t.id, sizes[i].w, sizes[i].h));

  const audio = createAudio(BASE, sfx_names(), music_names());
  const input = createInput();

  // Browsers only allow audio after a user gesture.
  const unlock = () => audio.unlock();
  window.addEventListener('keydown', unlock);
  window.addEventListener('pointerdown', unlock);
  window.addEventListener('keydown', (e) => {
    if (e.code === 'KeyM' && !e.repeat) audio.toggleMute();
  });

  const pause = () => {
    input.reset();
    game.pause();
  };
  window.addEventListener('blur', pause);
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) pause();
  });
  window.addEventListener('resize', () => renderer.resize());
  renderer.resize();

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
    renderer.draw(
      new Float32Array(mem, game.vertices_ptr(), game.vertices_len()),
      new Uint32Array(mem, game.batches_ptr(), game.batches_len()),
    );
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

main().catch((err: unknown) => {
  console.error(err);
  showError(err instanceof Error ? err.message : String(err));
});
