export type SpriteDraw = {
  x: number;
  y: number;
  w: number;
  h: number;
  u0: number;
  v0: number;
  u1: number;
  v1: number;
};

export type FrameEvents = {
  sprites: SpriteDraw[];
  coin_count: number;
  won: boolean;
  sfx: string[];
};

const BASE_WIDTH = 320;
const BASE_HEIGHT = 180;

export function createRenderer(canvas: HTMLCanvasElement): {
  resizeToDisplaySize(): void;
  render(frame: FrameEvents): void;
} {
  const gl = canvas.getContext('webgl2');
  if (!gl) throw new Error('Missing WebGL2 context');

  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  function resizeToDisplaySize() {
    // Pixel-perfect integer scaling.
    // Keep the internal render resolution fixed at BASE_* and scale the canvas
    // in CSS by an integer factor that fits within the window.
    const maxW = Math.max(1, window.innerWidth);
    const maxH = Math.max(1, window.innerHeight);

    const scale = Math.max(
      1,
      Math.floor(Math.min(maxW / BASE_WIDTH, maxH / BASE_HEIGHT)),
    );

    const cssW = BASE_WIDTH * scale;
    const cssH = BASE_HEIGHT * scale;

    // Set internal resolution.
    if (canvas.width !== BASE_WIDTH || canvas.height !== BASE_HEIGHT) {
      canvas.width = BASE_WIDTH;
      canvas.height = BASE_HEIGHT;
    }

    // Set CSS size (integer scaled).
    const cssWpx = `${cssW}px`;
    const cssHpx = `${cssH}px`;
    if (canvas.style.width !== cssWpx) canvas.style.width = cssWpx;
    if (canvas.style.height !== cssHpx) canvas.style.height = cssHpx;

    gl.viewport(0, 0, canvas.width, canvas.height);
  }

  function render(_frame: FrameEvents) {
    // TODO: draw sprites once we have a texture atlas + shader pipeline.
    gl.clearColor(0x10 / 0xff, 0x10 / 0xff, 0x18 / 0xff, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);
  }

  // Ensure viewport is initialized.
  resizeToDisplaySize();

  return { resizeToDisplaySize, render };
}
