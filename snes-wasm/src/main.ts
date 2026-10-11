import { createInput } from './input/input';
import { createRenderer } from './gl/renderer';
import { loadLevel } from './levels/level_loader';
import { createAudio } from './audio/audio';

// Note: wasm pkg import path will be wired up in a later task.
// eslint-disable-next-line @typescript-eslint/consistent-type-imports
import initWasm, { GameHandle, game_tick } from '../wasm/pkg/snes_wasm';

const canvas = document.querySelector<HTMLCanvasElement>('#game');
if (!canvas) throw new Error('Missing #game canvas');

const input = createInput();
const renderer = createRenderer(canvas);

function onResize() {
  renderer.resizeToDisplaySize();
}
window.addEventListener('resize', onResize, { passive: true });
// Initialize size once on boot.
onResize();
const audio = createAudio();

let audioUnlockStarted = false;
async function unlockAudioOnce() {
  if (audioUnlockStarted) return;
  audioUnlockStarted = true;
  try {
    await audio.unlock();
  } catch (err) {
    console.warn('audio unlock failed', err);
  }
}

function installAudioUnlockListeners() {
  const opts: AddEventListenerOptions = { once: true, passive: true };
  window.addEventListener('pointerdown', unlockAudioOnce, opts);
  window.addEventListener('keydown', unlockAudioOnce, opts);
}
installAudioUnlockListeners();

loadLevel('/levels/level1.json')
  .then((level) => {
    console.log('level loaded', level);
  })
  .catch((err) => {
    console.error('failed to load level', err);
  });

async function main() {
  await initWasm();

  const game = new GameHandle();

  function raf() {
    const state = input.poll();
    // TODO: pass to WASM later
    void state;

    renderer.resizeToDisplaySize();
    const frame = game_tick(game);
    for (const sfx of frame.sfx) audio.playSfx(sfx);
    renderer.render(frame);

    requestAnimationFrame(raf);
  }
  requestAnimationFrame(raf);
}

main().catch((err) => {
  console.error('fatal init error', err);
});
