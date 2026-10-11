import { FULLSCREEN_VS, locations, program } from './util';

const FS = `#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_past;
uniform sampler2D u_future;
uniform sampler2D u_bloom;
uniform float u_mix, u_glitch, u_crt, u_warmth, u_vignette, u_bloomStrength, u_time;
out vec4 o;

float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }

vec3 scene(vec2 uv) {
  vec3 past = texture(u_past, uv).rgb;
  // Scanlines aligned on the 180 game rows: darken the edges of each row.
  float row = fract(uv.y * 180.0);
  past *= 1.0 - u_crt * 0.55 * smoothstep(0.55, 1.0, abs(row - 0.5) * 2.0);
  // Soft halo: a touch of the neighbouring rows.
  vec3 halo = texture(u_past, uv + vec2(0.0, 1.0 / 180.0)).rgb + texture(u_past, uv - vec2(0.0, 1.0 / 180.0)).rgb;
  past += halo * 0.08 * u_crt;
  past = mix(past, past * vec3(1.08, 0.97, 0.84) + vec3(0.035, 0.015, 0.0), u_warmth * 0.7);
  vec3 future = texture(u_future, uv).rgb + texture(u_bloom, uv).rgb * u_bloomStrength;
  return mix(past, future, u_mix);
}

void main() {
  vec2 uv = v_uv;
  vec3 c;
  if (u_glitch > 0.0) {
    float band = floor(uv.y * 24.0);
    float h = hash(vec2(band, floor(u_time * 60.0)));
    uv.x += (h - 0.5) * 0.08 * u_glitch * step(0.55, h);
    float off = 0.01 * u_glitch;
    // Cyan/white split only (no saturated red flashes).
    c = vec3(scene(uv + vec2(off, 0.0)).r * (1.0 - 0.5 * u_glitch), scene(uv).g, scene(uv - vec2(off, 0.0)).b);
    c += (hash(gl_FragCoord.xy + u_time) - 0.5) * 0.2 * u_glitch;
  } else {
    c = scene(uv);
  }
  vec2 d = v_uv - 0.5;
  c *= 1.0 - u_vignette * dot(d, d) * 1.6;
  o = vec4(c, 1.0);
}`;

const NAMES = ['u_past', 'u_future', 'u_bloom', 'u_mix', 'u_glitch', 'u_crt', 'u_warmth', 'u_vignette', 'u_bloomStrength', 'u_time'] as const;

export type CompositeInputs = {
  past: WebGLTexture;
  future: WebGLTexture;
  bloom: WebGLTexture;
  mix: number;
  glitch: number;
  crt: number;
  warmth: number;
  vignette: number;
  bloomStrength: number;
  time: number;
};

export class Composite {
  private prog: WebGLProgram;
  private u: Record<(typeof NAMES)[number], WebGLUniformLocation | null>;
  private vao: WebGLVertexArrayObject;

  constructor(private gl: WebGL2RenderingContext) {
    this.prog = program(gl, FULLSCREEN_VS, FS);
    this.u = locations(gl, this.prog, NAMES);
    this.vao = gl.createVertexArray()!;
  }

  draw(i: CompositeInputs) {
    const gl = this.gl;
    gl.useProgram(this.prog);
    gl.bindVertexArray(this.vao);
    gl.disable(gl.DEPTH_TEST);
    gl.disable(gl.BLEND);
    [i.past, i.future, i.bloom].forEach((tex, unit) => {
      gl.activeTexture(gl.TEXTURE0 + unit);
      gl.bindTexture(gl.TEXTURE_2D, tex);
    });
    gl.uniform1i(this.u.u_past, 0);
    gl.uniform1i(this.u.u_future, 1);
    gl.uniform1i(this.u.u_bloom, 2);
    gl.uniform1f(this.u.u_mix, i.mix);
    gl.uniform1f(this.u.u_glitch, i.glitch);
    gl.uniform1f(this.u.u_crt, i.crt);
    gl.uniform1f(this.u.u_warmth, i.warmth);
    gl.uniform1f(this.u.u_vignette, i.vignette);
    gl.uniform1f(this.u.u_bloomStrength, i.bloomStrength);
    gl.uniform1f(this.u.u_time, i.time);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindVertexArray(null);
  }
}
