import { bindTarget, createTarget, FULLSCREEN_VS, locations, program, resizeTarget, type Target } from './util';

const BRIGHT_FS = `#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_src;
out vec4 o;
void main() {
  vec3 c = texture(u_src, v_uv).rgb;
  float l = max(max(c.r, c.g), c.b);
  // Only cool/hologram light may bloom: holograms are cyan (b > r), while the
  // hero's warm pixels (skin) have r > b and must keep their true colors.
  float cool = smoothstep(0.0, 0.15, c.b - c.r);
  o = vec4(c * smoothstep(0.45, 0.9, l) * cool, 1.0);
}`;

const BLUR_FS = `#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_src;
uniform vec2 u_dir; // one texel step along the blur axis
out vec4 o;
void main() {
  float w[5] = float[5](0.227, 0.195, 0.122, 0.054, 0.016);
  vec3 c = texture(u_src, v_uv).rgb * w[0];
  for (int i = 1; i < 5; i++) {
    c += texture(u_src, v_uv + u_dir * float(i) * 1.5).rgb * w[i];
    c += texture(u_src, v_uv - u_dir * float(i) * 1.5).rgb * w[i];
  }
  o = vec4(c, 1.0);
}`;

export class Bloom {
  private bright: WebGLProgram;
  private blur: WebGLProgram;
  private ub;
  private ul;
  private half: Target;
  private quarterA: Target;
  private quarterB: Target;
  private vao: WebGLVertexArrayObject;

  constructor(private gl: WebGL2RenderingContext) {
    this.bright = program(gl, FULLSCREEN_VS, BRIGHT_FS);
    this.blur = program(gl, FULLSCREEN_VS, BLUR_FS);
    this.ub = locations(gl, this.bright, ['u_src'] as const);
    this.ul = locations(gl, this.blur, ['u_src', 'u_dir'] as const);
    this.half = createTarget(gl, 160, 90, false, gl.LINEAR);
    this.quarterA = createTarget(gl, 80, 45, false, gl.LINEAR);
    this.quarterB = createTarget(gl, 80, 45, false, gl.LINEAR);
    this.vao = gl.createVertexArray()!;
  }

  /** Returns a quarter-resolution blurred glow of `src`. */
  apply(src: Target): WebGLTexture {
    const gl = this.gl;
    resizeTarget(gl, this.half, src.w / 2, src.h / 2);
    resizeTarget(gl, this.quarterA, src.w / 4, src.h / 4);
    resizeTarget(gl, this.quarterB, src.w / 4, src.h / 4);
    gl.disable(gl.DEPTH_TEST);
    gl.disable(gl.BLEND);
    gl.bindVertexArray(this.vao);
    gl.activeTexture(gl.TEXTURE0);

    bindTarget(gl, this.half, 0, 0);
    gl.useProgram(this.bright);
    gl.bindTexture(gl.TEXTURE_2D, src.tex);
    gl.uniform1i(this.ub.u_src, 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    gl.useProgram(this.blur);
    gl.uniform1i(this.ul.u_src, 0);
    bindTarget(gl, this.quarterA, 0, 0);
    gl.bindTexture(gl.TEXTURE_2D, this.half.tex);
    gl.uniform2f(this.ul.u_dir, 1 / this.half.w, 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    bindTarget(gl, this.quarterB, 0, 0);
    gl.bindTexture(gl.TEXTURE_2D, this.quarterA.tex);
    gl.uniform2f(this.ul.u_dir, 0, 1 / this.quarterA.h);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    gl.bindVertexArray(null);
    return this.quarterB.tex;
  }
}
