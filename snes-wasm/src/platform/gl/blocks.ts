import { BLOCK_FLOATS } from '../plan';
import { locations, program } from './util';

const VS = `#version 300 es
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_face;
layout(location = 3) in vec3 i_center;
layout(location = 4) in vec3 i_half;
layout(location = 5) in vec4 i_misc; // tile, extrusion, glow, pad
uniform mat4 u_viewProj;
uniform vec3 u_eye;
uniform vec2 u_tilesetSize; // texture size in pixels
out vec2 v_uv;
out vec2 v_face;
out vec3 v_normal;
out vec3 v_view;
out float v_glow;
out float v_dist;
out float v_y;
void main() {
  vec3 world = i_center + a_pos * 2.0 * i_half;
  float t = i_misc.x - 1.0;
  float cols = u_tilesetSize.x / 16.0;
  vec2 cell = vec2(mod(t, cols), floor(t / cols));
  v_uv = (cell + a_face) * 16.0 / u_tilesetSize;
  v_face = a_face;
  v_normal = a_normal;
  v_view = u_eye - world;
  v_dist = length(v_view);
  v_glow = i_misc.z;
  v_y = world.y;
  gl_Position = u_viewProj * vec4(world, 1.0);
}`;

const FS = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec2 v_face;
in vec3 v_normal;
in vec3 v_view;
in float v_glow;
in float v_dist;
in float v_y;
uniform sampler2D u_tiles;
uniform float u_time;
uniform vec3 u_fog;
out vec4 o;
void main() {
  vec4 tex = texture(u_tiles, v_uv);
  float edgeDist = min(min(v_face.x, 1.0 - v_face.x), min(v_face.y, 1.0 - v_face.y));
  float edge = 1.0 - smoothstep(0.0, 0.07, edgeDist);
  float fres = pow(1.0 - abs(dot(normalize(v_normal), normalize(v_view))), 2.0);
  float scan = 0.5 + 0.5 * sin(v_y * 0.4 - u_time * 3.0);
  vec3 holo = vec3(0.45, 0.85, 1.0);
  vec3 glass = holo * (0.10 + 0.6 * fres + 1.2 * edge) + tex.rgb * (0.25 + 0.15 * scan);
  float glassA = clamp(0.30 + 0.5 * fres + 0.7 * edge, 0.0, 1.0);
  vec3 col = mix(tex.rgb, glass, v_glow);
  float a = mix(tex.a, glassA, v_glow);
  if (a < 0.02) discard;
  float fog = clamp((v_dist - 250.0) / 900.0, 0.0, 1.0);
  o = vec4(mix(col, u_fog, fog), a * (1.0 - 0.7 * fog));
}`;

/** Unit cube centered at the origin: 36 vertices of pos(3), normal(3), face uv(2). */
function cube(): Float32Array {
  const faces: [number[], number[], number[]][] = [
    [[0, 0, 1], [1, 0, 0], [0, -1, 0]],
    [[0, 0, -1], [-1, 0, 0], [0, -1, 0]],
    [[1, 0, 0], [0, 0, -1], [0, -1, 0]],
    [[-1, 0, 0], [0, 0, 1], [0, -1, 0]],
    [[0, 1, 0], [1, 0, 0], [0, 0, 1]],
    [[0, -1, 0], [1, 0, 0], [0, 0, -1]],
  ];
  const out: number[] = [];
  for (const [n, u, v] of faces) {
    for (const [a, b] of [[0, 0], [1, 0], [0, 1], [0, 1], [1, 0], [1, 1]]) {
      for (let i = 0; i < 3; i++) out.push(n[i] * 0.5 + u[i] * (a - 0.5) + v[i] * (b - 0.5));
      out.push(n[0], n[1], n[2], a, b);
    }
  }
  return new Float32Array(out);
}

const NAMES = ['u_viewProj', 'u_eye', 'u_tilesetSize', 'u_tiles', 'u_time', 'u_fog'] as const;

export class Blocks {
  private prog: WebGLProgram;
  private u: Record<(typeof NAMES)[number], WebGLUniformLocation | null>;
  private vao: WebGLVertexArrayObject;
  private instances: WebGLBuffer;
  private instanceBytes = 0;

  constructor(private gl: WebGL2RenderingContext) {
    this.prog = program(gl, VS, FS);
    this.u = locations(gl, this.prog, NAMES);
    this.vao = gl.createVertexArray()!;
    gl.bindVertexArray(this.vao);
    const geo = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, geo);
    gl.bufferData(gl.ARRAY_BUFFER, cube(), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 32, 0);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 32, 12);
    gl.enableVertexAttribArray(2);
    gl.vertexAttribPointer(2, 2, gl.FLOAT, false, 32, 24);
    this.instances = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instances);
    const stride = BLOCK_FLOATS * 4;
    for (const [loc, size, offset] of [[3, 3, 0], [4, 3, 12], [5, 4, 24]]) {
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, stride, offset);
      gl.vertexAttribDivisor(loc, 1);
    }
    gl.bindVertexArray(null);
  }

  draw(blocks: Float32Array, viewProj: Float32Array, eye: Float32Array, tiles: WebGLTexture, tilesetSize: [number, number], time: number, fog: [number, number, number]) {
    const count = blocks.length / BLOCK_FLOATS;
    if (count === 0) return;
    const gl = this.gl;
    gl.useProgram(this.prog);
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instances);
    if (blocks.byteLength > this.instanceBytes) {
      this.instanceBytes = Math.max(blocks.byteLength, this.instanceBytes * 2);
      gl.bufferData(gl.ARRAY_BUFFER, this.instanceBytes, gl.DYNAMIC_DRAW);
    }
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, blocks);
    gl.uniformMatrix4fv(this.u.u_viewProj, false, viewProj);
    gl.uniform3fv(this.u.u_eye, eye);
    gl.uniform2f(this.u.u_tilesetSize, tilesetSize[0], tilesetSize[1]);
    gl.uniform1f(this.u.u_time, time);
    gl.uniform3f(this.u.u_fog, fog[0], fog[1], fog[2]);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, tiles);
    gl.uniform1i(this.u.u_tiles, 0);
    gl.enable(gl.DEPTH_TEST);
    gl.depthMask(true);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 36, count);
    gl.bindVertexArray(null);
  }
}
