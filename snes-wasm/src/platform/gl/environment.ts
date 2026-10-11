import { locations, program } from './util';

const FLOOR_VS = `#version 300 es
uniform mat4 u_viewProj;
uniform float u_floorY, u_focusX;
out vec3 v_world;
void main() {
  vec2 c[6] = vec2[6](vec2(-1,-1), vec2(1,-1), vec2(-1,1), vec2(-1,1), vec2(1,-1), vec2(1,1));
  vec2 q = c[gl_VertexID];
  v_world = vec3(u_focusX + 700.0 + q.x * 1400.0, u_floorY, q.y * 900.0);
  gl_Position = u_viewProj * vec4(v_world, 1.0);
}`;

const FLOOR_FS = `#version 300 es
precision highp float;
in vec3 v_world;
uniform vec3 u_eye;
out vec4 o;
void main() {
  vec2 g = fract(v_world.xz / 32.0) - 0.5;
  float dotMask = 1.0 - smoothstep(0.04, 0.09, length(g));
  float fade = 1.0 - clamp((distance(v_world, u_eye) - 200.0) / 1100.0, 0.0, 1.0);
  o = vec4(vec3(0.47, 0.86, 1.0), dotMask * 0.55 * fade);
}`;

const GHOST_VS = `#version 300 es
uniform mat4 u_viewProj;
uniform float u_floorY, u_focusX;
out vec2 v_uv;
void main() {
  vec2 c[6] = vec2[6](vec2(0,0), vec2(1,0), vec2(0,1), vec2(0,1), vec2(1,0), vec2(1,1));
  vec2 q = c[gl_VertexID];
  // A 3000×700 wall facing the camera, 1500 px ahead of the hero.
  vec3 world = vec3(u_focusX + 1500.0, u_floorY + q.y * 700.0, (q.x - 0.5) * 3000.0);
  v_uv = vec2(q.x * 3.0, mix(1.0, 0.45, q.y)); // mountains band of bg_far, repeated
  gl_Position = u_viewProj * vec4(world, 1.0);
}`;

const GHOST_FS = `#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_tex;
out vec4 o;
void main() {
  vec3 c = texture(u_tex, v_uv).rgb;
  float lum = dot(c, vec3(0.3, 0.5, 0.2));
  float top = smoothstep(0.45, 0.6, v_uv.y);
  o = vec4(vec3(0.5, 0.85, 1.0) * lum, 0.22 * top);
}`;

const MOTES_VS = `#version 300 es
uniform mat4 u_viewProj;
uniform vec3 u_eye;
uniform float u_time;
out float v_a;
float h(float n) { return fract(sin(n) * 43758.5453); }
void main() {
  float i = float(gl_VertexID);
  vec3 box = vec3(900.0, 300.0, 600.0);
  vec3 p = vec3(h(i * 1.3), h(i * 7.1), h(i * 3.7)) * box;
  p.y += u_time * (6.0 + 10.0 * h(i));
  p = mod(p, box) - box * vec3(0.1, 0.5, 0.5);
  vec3 world = u_eye + p;
  v_a = 0.25 + 0.5 * h(i * 9.9);
  gl_Position = u_viewProj * vec4(world, 1.0);
  gl_PointSize = 2.0;
}`;

const MOTES_FS = `#version 300 es
precision highp float;
in float v_a;
out vec4 o;
void main() { o = vec4(0.7, 0.95, 1.0, v_a); }`;

const MOTES = 400;

export class Environment {
  private floor: WebGLProgram;
  private ghost: WebGLProgram;
  private motes: WebGLProgram;
  private uf;
  private ug;
  private um;
  private vao: WebGLVertexArrayObject;

  constructor(private gl: WebGL2RenderingContext) {
    this.floor = program(gl, FLOOR_VS, FLOOR_FS);
    this.ghost = program(gl, GHOST_VS, GHOST_FS);
    this.motes = program(gl, MOTES_VS, MOTES_FS);
    this.uf = locations(gl, this.floor, ['u_viewProj', 'u_floorY', 'u_focusX', 'u_eye'] as const);
    this.ug = locations(gl, this.ghost, ['u_viewProj', 'u_floorY', 'u_focusX', 'u_tex'] as const);
    this.um = locations(gl, this.motes, ['u_viewProj', 'u_eye', 'u_time'] as const);
    this.vao = gl.createVertexArray()!;
  }

  draw(viewProj: Float32Array, eye: Float32Array, floorY: number, focusX: number, time: number, bgFar: WebGLTexture | undefined) {
    const gl = this.gl;
    gl.bindVertexArray(this.vao);
    gl.enable(gl.DEPTH_TEST);
    gl.depthMask(false);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    gl.useProgram(this.floor);
    gl.uniformMatrix4fv(this.uf.u_viewProj, false, viewProj);
    gl.uniform1f(this.uf.u_floorY, floorY);
    gl.uniform1f(this.uf.u_focusX, focusX);
    gl.uniform3fv(this.uf.u_eye, eye);
    gl.drawArrays(gl.TRIANGLES, 0, 6);

    if (bgFar) {
      gl.useProgram(this.ghost);
      gl.uniformMatrix4fv(this.ug.u_viewProj, false, viewProj);
      gl.uniform1f(this.ug.u_floorY, floorY);
      gl.uniform1f(this.ug.u_focusX, focusX);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, bgFar);
      gl.uniform1i(this.ug.u_tex, 0);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }

    gl.useProgram(this.motes);
    gl.uniformMatrix4fv(this.um.u_viewProj, false, viewProj);
    gl.uniform3fv(this.um.u_eye, eye);
    gl.uniform1f(this.um.u_time, time);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE);
    gl.drawArrays(gl.POINTS, 0, MOTES);

    gl.depthMask(true);
    gl.bindVertexArray(null);
  }
}
