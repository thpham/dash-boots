import { createInput } from './input/input';

const canvas = document.querySelector<HTMLCanvasElement>('#game');
if (!canvas) throw new Error('Missing #game canvas');

const ctx = canvas.getContext('2d');
if (!ctx) throw new Error('Missing 2D context');

const input = createInput();

let t = 0;
function raf() {
  const state = input.poll();
  // TODO: pass to WASM later
  void state;

  t += 1;
  ctx.fillStyle = '#101018';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = '#7cf';
  ctx.fillRect(10 + (t % 100), 20, 16, 16);
  requestAnimationFrame(raf);
}
requestAnimationFrame(raf);
