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
};

export function createRenderer(canvas: HTMLCanvasElement): {
  resize(): void;
  render(frame: FrameEvents): void;
} {
  const gl = canvas.getContext('webgl2');
  if (!gl) throw new Error('Missing WebGL2 context');

  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  function resize() {
    const dpr = window.devicePixelRatio ?? 1;
    const displayWidth = Math.max(1, Math.floor(canvas.clientWidth * dpr));
    const displayHeight = Math.max(1, Math.floor(canvas.clientHeight * dpr));

    if (canvas.width !== displayWidth || canvas.height !== displayHeight) {
      canvas.width = displayWidth;
      canvas.height = displayHeight;
    }

    gl.viewport(0, 0, canvas.width, canvas.height);
  }

  function render(_frame: FrameEvents) {
    // TODO: draw sprites once we have a texture atlas + shader pipeline.
    gl.clearColor(0x10 / 0xff, 0x10 / 0xff, 0x18 / 0xff, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);
  }

  // Ensure viewport is initialized.
  resize();

  return { resize, render };
}
