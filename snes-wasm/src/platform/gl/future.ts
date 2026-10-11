import { PLAN, type FrameData } from '../plan';
import { Billboards } from './billboards';
import { Blocks } from './blocks';
import { Bloom } from './bloom';
import { Environment } from './environment';
import { bindTarget, createTarget, resizeTarget, solidTexture, type Target } from './util';

const FOG: [number, number, number] = [0.008, 0.024, 0.05];
const TEX_BG_FAR = 0;
const TEX_TILES = 2;

export class FutureRenderer {
  private target: Target;
  private blocks: Blocks;
  private env: Environment;
  private billboards: Billboards;
  private bloom: Bloom;
  private black: WebGLTexture;
  private linear = false;

  constructor(
    private gl: WebGL2RenderingContext,
    private tex: (id: number) => WebGLTexture | undefined,
    private tilesetSize: [number, number],
    private spritesSize: [number, number],
    private canvasH: () => number,
  ) {
    // NEAREST: at low focus the future looks pixelated; it sharpens as the resolution rises.
    this.target = createTarget(gl, 320, 180, true, gl.NEAREST);
    this.blocks = new Blocks(gl);
    this.env = new Environment(gl);
    this.billboards = new Billboards(gl);
    this.bloom = new Bloom(gl);
    this.black = solidTexture(gl, [0, 0, 0, 255]);
  }

  render(f: FrameData): { color: WebGLTexture; bloom: WebGLTexture } {
    const gl = this.gl;
    const p = f.plan;
    if (p[PLAN.WORLD_MIX] <= 0) return { color: this.black, bloom: this.black };
    resizeTarget(gl, this.target, p[PLAN.FUTURE_W], p[PLAN.FUTURE_H]);
    // Past half of the canvas: the big integer-ish pixelated scaling keeps
    // NEAREST. Once the target exceeds half the canvas height the upscale is
    // an arbitrary factor, so point sampling would shimmer — go LINEAR. This
    // also covers the bloom pass, which point-sampled the target before.
    const linear = p[PLAN.FUTURE_H] > this.canvasH() * 0.5;
    if (linear !== this.linear) {
      this.linear = linear;
      const filter = linear ? gl.LINEAR : gl.NEAREST;
      gl.bindTexture(gl.TEXTURE_2D, this.target.tex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
    }
    bindTarget(gl, this.target, 0, 0);
    gl.clearColor(FOG[0], FOG[1], FOG[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    const viewProj = p.subarray(PLAN.VIEW_PROJ, PLAN.VIEW_PROJ + 16);
    const eye = p.subarray(PLAN.EYE, PLAN.EYE + 3);
    const time = p[PLAN.TIME];
    this.env.draw(viewProj, eye, p[PLAN.FLOOR_Y], p[PLAN.FOCUS_X], time, this.tex(TEX_BG_FAR));
    const tiles = this.tex(TEX_TILES);
    if (tiles) this.blocks.draw(f.blocks, viewProj, eye, tiles, this.tilesetSize, time, FOG);
    const sprites = this.tex(3);
    if (sprites && tiles) {
      const camRight = p.subarray(PLAN.CAM_RIGHT, PLAN.CAM_RIGHT + 3);
      this.billboards.draw(f.billboards, f.shadows, viewProj, camRight, eye, sprites, tiles, this.spritesSize, this.tilesetSize, time);
    }
    const bloom = this.bloom.apply(this.target);
    return { color: this.target.tex, bloom };
  }
}
