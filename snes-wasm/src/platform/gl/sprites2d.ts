import { program } from './util';

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

export class Sprites2D {
  private prog: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  private vbo: WebGLBuffer;
  private vboBytes = 0;
  private uView: WebGLUniformLocation | null;

  constructor(private gl: WebGL2RenderingContext) {
    this.prog = program(gl, VS, FS);
    this.uView = gl.getUniformLocation(this.prog, 'u_view');
    gl.useProgram(this.prog);
    gl.uniform1i(gl.getUniformLocation(this.prog, 'u_tex'), 0);
    this.vao = gl.createVertexArray()!;
    gl.bindVertexArray(this.vao);
    this.vbo = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    const stride = FLOATS_PER_VERTEX * 4;
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, stride, 0);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 2, gl.FLOAT, false, stride, 8);
    gl.enableVertexAttribArray(2);
    gl.vertexAttribPointer(2, 4, gl.FLOAT, false, stride, 16);
    gl.bindVertexArray(null);
  }

  /** Draw a vertex list in a 320×180 coordinate space into the bound target/viewport. */
  draw(verts: Float32Array, batches: Uint32Array, textures: WebGLTexture[], viewW: number, viewH: number) {
    const gl = this.gl;
    gl.useProgram(this.prog);
    gl.uniform2f(this.uView, viewW, viewH);
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    if (verts.byteLength > this.vboBytes) {
      this.vboBytes = Math.max(verts.byteLength, this.vboBytes * 2);
      gl.bufferData(gl.ARRAY_BUFFER, this.vboBytes, gl.DYNAMIC_DRAW);
    }
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, verts);
    gl.disable(gl.DEPTH_TEST);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.activeTexture(gl.TEXTURE0);
    for (let i = 0; i + 2 < batches.length; i += 3) {
      const tex = textures[batches[i]];
      if (!tex) continue;
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.drawArrays(gl.TRIANGLES, batches[i + 1], batches[i + 2]);
    }
    gl.bindVertexArray(null);
  }
}
