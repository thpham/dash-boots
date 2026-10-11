// WebGL2 host renderer: uploads the vertex buffer built by the WASM core and
// issues one draw call per batch (layout documented in wasm/src/render/mod.rs).

const FLOATS_PER_VERTEX = 8;

const VS = `#version 300 es
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;
layout(location = 2) in vec4 a_color;
uniform vec2 u_view;
out vec2 v_uv;
out vec4 v_color;
void main() {
  vec2 clip = a_pos / u_view * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  v_uv = a_uv;
  v_color = a_color;
}`;

const FS = `#version 300 es
precision mediump float;
in vec2 v_uv;
in vec4 v_color;
uniform sampler2D u_tex;
out vec4 out_color;
void main() {
  vec4 c = texture(u_tex, v_uv) * v_color;
  if (c.a <= 0.0) discard;
  out_color = c;
}`;

export type TextureSpec = { id: number; url: string; repeatX?: boolean };

export class Renderer {
  private gl: WebGL2RenderingContext;
  private textures: WebGLTexture[] = [];
  private vbo: WebGLBuffer;
  private vboBytes = 0;

  constructor(
    private canvas: HTMLCanvasElement,
    private viewW: number,
    private viewH: number,
  ) {
    const gl = canvas.getContext('webgl2', { antialias: false, alpha: false });
    if (!gl) throw new Error('WebGL2 is not available in this browser.');
    this.gl = gl;
    // The canvas always holds the native 320x180 image; CSS scales it up.
    canvas.width = viewW;
    canvas.height = viewH;

    const prog = this.program(VS, FS);
    gl.useProgram(prog);
    gl.uniform2f(gl.getUniformLocation(prog, 'u_view'), viewW, viewH);
    gl.uniform1i(gl.getUniformLocation(prog, 'u_tex'), 0);

    gl.bindVertexArray(gl.createVertexArray());
    this.vbo = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    const stride = FLOATS_PER_VERTEX * 4;
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, stride, 0);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 2, gl.FLOAT, false, stride, 8);
    gl.enableVertexAttribArray(2);
    gl.vertexAttribPointer(2, 4, gl.FLOAT, false, stride, 16);

    gl.disable(gl.DEPTH_TEST);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.viewport(0, 0, viewW, viewH);
    gl.activeTexture(gl.TEXTURE0);
  }

  private program(vs: string, fs: string): WebGLProgram {
    const gl = this.gl;
    const compile = (type: number, src: string) => {
      const sh = gl.createShader(type)!;
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) throw new Error(`shader: ${gl.getShaderInfoLog(sh)}`);
      return sh;
    };
    const prog = gl.createProgram()!;
    gl.attachShader(prog, compile(gl.VERTEX_SHADER, vs));
    gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, fs));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(`program: ${gl.getProgramInfoLog(prog)}`);
    return prog;
  }

  /** Load an image into texture slot `id`; resolves with its pixel size. */
  async loadTexture(spec: TextureSpec): Promise<{ w: number; h: number }> {
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
    // Nearest filtering: crisp pixels.
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, spec.repeatX ? gl.REPEAT : gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.textures[spec.id] = tex;
    return { w: img.naturalWidth, h: img.naturalHeight };
  }

  /** Integer scaling in *device* pixels so every game pixel has the same size. */
  resize() {
    const dpr = window.devicePixelRatio || 1;
    const scale = Math.max(
      1,
      Math.floor(Math.min((window.innerWidth * dpr) / this.viewW, (window.innerHeight * dpr) / this.viewH)),
    );
    const w = `${(this.viewW * scale) / dpr}px`;
    const h = `${(this.viewH * scale) / dpr}px`;
    if (this.canvas.style.width !== w) this.canvas.style.width = w;
    if (this.canvas.style.height !== h) this.canvas.style.height = h;
  }

  draw(verts: Float32Array, batches: Uint32Array, clear: boolean) {
    const gl = this.gl;
    if (clear) {
      gl.clearColor(0.23, 0.42, 0.85, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    if (verts.byteLength > this.vboBytes) {
      this.vboBytes = Math.max(verts.byteLength, this.vboBytes * 2);
      gl.bufferData(gl.ARRAY_BUFFER, this.vboBytes, gl.DYNAMIC_DRAW);
    }
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, verts);
    for (let i = 0; i + 2 < batches.length; i += 3) {
      const tex = this.textures[batches[i]];
      if (!tex) continue;
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.drawArrays(gl.TRIANGLES, batches[i + 1], batches[i + 2]);
    }
  }
}
