import { createInput } from './input/input';
import { createRenderer } from './gl/renderer';
import { loadLevel } from './levels/level_loader';
import { createAudio } from './audio/audio';

// Note: wasm pkg import path will be wired up in a later task.
// eslint-disable-next-line @typescript-eslint/consistent-type-imports
import initWasm, { GameHandle, game_tick, game_load_level } from '../wasm/pkg/snes_wasm';

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


async function main() {
  await initWasm();

  const game = new GameHandle();

  const level = await loadLevel('/levels/level1.json');
  game_load_level(game, level as any);

  let prevJump = false;
  let prevDash = false;

  function raf() {
    const state = input.poll();
    const inputForWasm = {
      ...state,
      prev_jump: prevJump,
      prev_dash: prevDash,
    };
    prevJump = state.jump;
    prevDash = state.dash;

    renderer.resizeToDisplaySize();
    const frame = game_tick(game, inputForWasm as any);
    for (const sfx of frame.sfx) audio.playSfx(sfx);
    renderer.render(frame);

    requestAnimationFrame(raf);
  }
  requestAnimationFrame(raf);
}

main().catch((err) => {
  console.error('fatal init error', err);
});
