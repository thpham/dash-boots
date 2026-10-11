import { PLAN, type FrameData } from '../plan';
import { Composite } from './composite';
import { Sprites2D } from './sprites2d';
import { bindTarget, createTarget, solidTexture, type Target } from './util';

export type TextureSpec = { id: number; url: string; repeatX?: boolean };

const VIEW_W = 320;
const VIEW_H = 180;

export class Executor {
  readonly gl: WebGL2RenderingContext;
  private textures: WebGLTexture[] = [];
  private sprites: Sprites2D;
  private composite: Composite;
  private past: Target;
  private black: WebGLTexture;
  private canvasW = VIEW_W;
  private canvasH = VIEW_H;
  /** True once the 3D passes are created successfully (Task 9). */
  futureReady = false;

  constructor(private canvas: HTMLCanvasElement) {
    const gl = canvas.getContext('webgl2', { antialias: false, alpha: false });
    if (!gl) throw new Error('WebGL2 is not available in this browser.');
    this.gl = gl;
    this.sprites = new Sprites2D(gl);
    this.composite = new Composite(gl);
    this.past = createTarget(gl, VIEW_W, VIEW_H, false, gl.NEAREST);
    this.black = solidTexture(gl, [0, 0, 0, 255]);
  }

  async loadTextures(specs: TextureSpec[]): Promise<{ w: number; h: number }[]> {
    return Promise.all(specs.map((s) => this.loadTexture(s)));
  }

  private async loadTexture(spec: TextureSpec): Promise<{ w: number; h: number }> {
    const img = new Image();
    img.src = spec.url;
    try {
      await img.decode();
    } catch {
      throw new Error(`Failed to load texture ${spec.url}`);
    }
    const gl = this.gl;
    const tex = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, spec.repeatX ? gl.REPEAT : gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.textures[spec.id] = tex;
    return { w: img.naturalWidth, h: img.naturalHeight };
  }

  texture(id: number): WebGLTexture | undefined {
    return this.textures[id];
  }

  /** Canvas = largest integer multiple of 320×180 that fits, in device pixels. */
  resize(): { w: number; h: number; scale: number } {
    const dpr = window.devicePixelRatio || 1;
    const scale = Math.max(
      1,
      Math.floor(Math.min((window.innerWidth * dpr) / VIEW_W, (window.innerHeight * dpr) / VIEW_H)),
    );
    const w = VIEW_W * scale;
    const h = VIEW_H * scale;
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.canvas.style.width = `${w / dpr}px`;
    this.canvas.style.height = `${h / dpr}px`;
    this.canvasW = w;
    this.canvasH = h;
    return { w, h, scale };
  }

  render(f: FrameData) {
    const gl = this.gl;
    // A. Past image at native resolution.
    bindTarget(gl, this.past, 0, 0);
    gl.clearColor(0.23, 0.42, 0.85, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    this.sprites.draw(f.retroVerts, f.retroBatches, this.textures, VIEW_W, VIEW_H);

    // B/C. Future scene + bloom (Task 9 / 10).
    const future = this.renderFuture(f);

    // D. Composite to the canvas.
    bindTarget(gl, null, this.canvasW, this.canvasH);
    const p = f.plan;
    this.composite.draw({
      past: this.past.tex,
      future: future.color,
      bloom: future.bloom,
      mix: p[PLAN.WORLD_MIX],
      glitch: p[PLAN.GLITCH],
      crt: p[PLAN.CRT],
      warmth: p[PLAN.WARMTH],
      vignette: p[PLAN.VIGNETTE],
      bloomStrength: p[PLAN.BLOOM],
      time: p[PLAN.TIME],
    });

    // E. Pixel HUD on top, integer-scaled through the 320×180 coordinate space.
    this.sprites.draw(f.hudVerts, f.hudBatches, this.textures, VIEW_W, VIEW_H);
  }

  /** Overridden in Task 9; until then the future is black. */
  protected renderFuture(_f: FrameData): { color: WebGLTexture; bloom: WebGLTexture } {
    return { color: this.black, bloom: this.black };
  }
}
