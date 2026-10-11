import { BILLBOARD_FLOATS, SHADOW_FLOATS } from '../plan';
import { locations, program } from './util';

const BB_VS = `#version 300 es
layout(location = 0) in vec2 a_corner; // x in [-0.5, 0.5], y in [0, 1]
layout(location = 1) in vec3 i_anchor;
layout(location = 2) in vec2 i_size;
layout(location = 3) in vec4 i_atlas;
layout(location = 4) in vec3 i_misc; // texture id, flags, alpha
uniform mat4 u_viewProj;
uniform vec3 u_camRight;
uniform vec3 u_eye;
uniform vec2 u_spritesSize, u_tilesSize;
out vec2 v_uv;
flat out int v_tex;
flat out int v_flags;
out float v_alpha;
out float v_dist;
void main() {
  int flags = int(i_misc.y + 0.5);
  vec3 right = normalize(vec3(u_camRight.x, 0.0, u_camRight.z));
  vec3 world = i_anchor + right * a_corner.x * i_size.x + vec3(0.0, 1.0, 0.0) * a_corner.y * i_size.y;
  vec2 f = vec2(a_corner.x + 0.5, 1.0 - a_corner.y);
  if ((flags & 2) != 0) f.x = 1.0 - f.x;
  if ((flags & 4) != 0) f.y = 1.0 - f.y;
  v_tex = int(i_misc.x + 0.5);
  vec2 size = v_tex == 2 ? u_tilesSize : u_spritesSize;
  v_uv = (i_atlas.xy + f * i_atlas.zw) / size;
  v_flags = flags;
  v_alpha = i_misc.z;
  v_dist = distance(world, u_eye);
  gl_Position = u_viewProj * vec4(world, 1.0);
}`;

const BB_FS = `#version 300 es
precision highp float;
in vec2 v_uv;
flat in int v_tex;
flat in int v_flags;
in float v_alpha;
in float v_dist;
uniform sampler2D u_sprites, u_tiles;
uniform float u_time;
out vec4 o;
void main() {
  vec4 c = v_tex == 2 ? texture(u_tiles, v_uv) : texture(u_sprites, v_uv);
  if (c.a < 0.5) discard;
  if ((v_flags & 1) != 0) {
    float lum = dot(c.rgb, vec3(0.3, 0.55, 0.15));
    float scan = 0.8 + 0.2 * sin(gl_FragCoord.y * 1.5 + u_time * 8.0);
    c.rgb = vec3(0.45, 0.88, 1.0) * (0.35 + lum * 1.3) * scan;
    c.a = 0.85;
  }
  float fog = clamp((v_dist - 250.0) / 900.0, 0.0, 1.0);
  o = vec4(c.rgb * (1.0 - 0.8 * fog), c.a * v_alpha);
}`;

const SH_VS = `#version 300 es
layout(location = 0) in vec2 a_corner;
layout(location = 1) in vec3 i_pos;
layout(location = 2) in vec2 i_misc; // radius, alpha
uniform mat4 u_viewProj;
out vec2 v_q;
out float v_alpha;
void main() {
  v_q = a_corner * 2.0;
  v_alpha = i_misc.y;
  vec3 world = i_pos + vec3(a_corner.x * 2.0 * i_misc.x, 0.0, (a_corner.y - 0.5) * 2.0 * i_misc.x);
  gl_Position = u_viewProj * vec4(world, 1.0);
}`;

const SH_FS = `#version 300 es
precision highp float;
in vec2 v_q;
in float v_alpha;
out vec4 o;
void main() {
  vec2 q = vec2(v_q.x, v_q.y - 1.0);
  float d = length(q);
  if (d > 1.0) discard;
  o = vec4(0.0, 0.02, 0.05, (1.0 - d * d) * 0.6 * v_alpha);
}`;

// Two triangles with corners x in [-0.5, 0.5], y in [0, 1].
const QUAD = new Float32Array([-0.5, 0, 0.5, 0, -0.5, 1, -0.5, 1, 0.5, 0, 0.5, 1]);

export class Billboards {
  private bb: WebGLProgram;
  private sh: WebGLProgram;
  private ub;
  private us;
  private bbVao: WebGLVertexArrayObject;
  private shVao: WebGLVertexArrayObject;
  private bbBuf: WebGLBuffer;
  private shBuf: WebGLBuffer;
  private bbBytes = 0;
  private shBytes = 0;

  constructor(private gl: WebGL2RenderingContext) {
    this.bb = program(gl, BB_VS, BB_FS);
    this.sh = program(gl, SH_VS, SH_FS);
    this.ub = locations(gl, this.bb, ['u_viewProj', 'u_camRight', 'u_eye', 'u_spritesSize', 'u_tilesSize', 'u_sprites', 'u_tiles', 'u_time'] as const);
    this.us = locations(gl, this.sh, ['u_viewProj'] as const);
    const quad = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.bufferData(gl.ARRAY_BUFFER, QUAD, gl.STATIC_DRAW);

    this.bbVao = gl.createVertexArray()!;
    gl.bindVertexArray(this.bbVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 8, 0);
    this.bbBuf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.bbBuf);
    const bs = BILLBOARD_FLOATS * 4;
    for (const [loc, size, offset] of [[1, 3, 0], [2, 2, 12], [3, 4, 20], [4, 3, 36]]) {
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, bs, offset);
      gl.vertexAttribDivisor(loc, 1);
    }

    this.shVao = gl.createVertexArray()!;
    gl.bindVertexArray(this.shVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 8, 0);
    this.shBuf = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.shBuf);
    const ss = SHADOW_FLOATS * 4;
    for (const [loc, size, offset] of [[1, 3, 0], [2, 2, 12]]) {
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, ss, offset);
      gl.vertexAttribDivisor(loc, 1);
    }
    gl.bindVertexArray(null);
  }

  private upload(buf: WebGLBuffer, data: Float32Array, bytes: number): number {
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    if (data.byteLength > bytes) {
      bytes = Math.max(data.byteLength, bytes * 2);
      gl.bufferData(gl.ARRAY_BUFFER, bytes, gl.DYNAMIC_DRAW);
    }
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, data);
    return bytes;
  }

  draw(
    billboards: Float32Array,
    shadows: Float32Array,
    viewProj: Float32Array,
    camRight: Float32Array,
    eye: Float32Array,
    sprites: WebGLTexture,
    tiles: WebGLTexture,
    spritesSize: [number, number],
    tilesSize: [number, number],
    time: number,
  ) {
    const gl = this.gl;
    gl.enable(gl.DEPTH_TEST);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    const nShadows = shadows.length / SHADOW_FLOATS;
    if (nShadows > 0) {
      this.shBytes = this.upload(this.shBuf, shadows, this.shBytes);
      gl.useProgram(this.sh);
      gl.uniformMatrix4fv(this.us.u_viewProj, false, viewProj);
      gl.depthMask(false);
      gl.bindVertexArray(this.shVao);
      gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, nShadows);
    }

    const n = billboards.length / BILLBOARD_FLOATS;
    if (n > 0) {
      this.bbBytes = this.upload(this.bbBuf, billboards, this.bbBytes);
      gl.useProgram(this.bb);
      gl.uniformMatrix4fv(this.ub.u_viewProj, false, viewProj);
      gl.uniform3fv(this.ub.u_camRight, camRight);
      gl.uniform3fv(this.ub.u_eye, eye);
      gl.uniform2f(this.ub.u_spritesSize, spritesSize[0], spritesSize[1]);
      gl.uniform2f(this.ub.u_tilesSize, tilesSize[0], tilesSize[1]);
      gl.uniform1f(this.ub.u_time, time);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, sprites);
      gl.uniform1i(this.ub.u_sprites, 0);
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, tiles);
      gl.uniform1i(this.ub.u_tiles, 1);
      gl.depthMask(true);
      gl.bindVertexArray(this.bbVao);
      gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, n);
      gl.activeTexture(gl.TEXTURE0);
    }
    gl.depthMask(true);
    gl.bindVertexArray(null);
  }
}
