import { createInput } from './input/input';
import { createRenderer } from './gl/renderer';
import { loadLevel } from './levels/level_loader';

// Note: wasm pkg import path will be wired up in a later task.
// eslint-disable-next-line @typescript-eslint/consistent-type-imports
import { GameHandle, game_tick } from '../wasm/pkg/snes_wasm';

const canvas = document.querySelector<HTMLCanvasElement>('#game');
if (!canvas) throw new Error('Missing #game canvas');

const input = createInput();
const renderer = createRenderer(canvas);

loadLevel('/levels/level1.json')
  .then((level) => {
    console.log('level loaded', level);
  })
  .catch((err) => {
    console.error('failed to load level', err);
  });

const game = new GameHandle();

function raf() {
  const state = input.poll();
  // TODO: pass to WASM later
  void state;

  renderer.resize();
  const frame = game_tick(game);
  renderer.render(frame);

  requestAnimationFrame(raf);
}
requestAnimationFrame(raf);
