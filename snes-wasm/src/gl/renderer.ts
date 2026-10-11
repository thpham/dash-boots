import { buildVisibleTileSprites } from './tilemap';

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

export type Camera = {
  x: number;
  y: number;
};

export type TilemapRenderData = {
  width: number;
  height: number;
  tileSize: number;
  terrain: number[];
  coins: number[];
};

export type FrameEvents = {
  sprites: SpriteDraw[];
  coin_count: number;
  won: boolean;
  sfx: string[];
  camera: Camera;
  tilemap: TilemapRenderData;
};

const BASE_WIDTH = 320;
const BASE_HEIGHT = 180;

export function createRenderer(canvas: HTMLCanvasElement): {
  resizeToDisplaySize(): void;
  render(frame: FrameEvents): void;
} {
  const gl = canvas.getContext('webgl2');
  if (!gl) throw new Error('Missing WebGL2 context');

  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  // --- Shader helpers ---
  function compileShader(type: number, source: string): WebGLShader {
    const sh = gl.createShader(type);
    if (!sh) throw new Error('Failed to create shader');
    gl.shaderSource(sh, source);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      const log = gl.getShaderInfoLog(sh) ?? '(no log)';
      gl.deleteShader(sh);
      throw new Error(`Shader compile failed: ${log}`);
    }
    return sh;
  }

  function createProgram(vsSource: string, fsSource: string): WebGLProgram {
    const vs = compileShader(gl.VERTEX_SHADER, vsSource);
    const fs = compileShader(gl.FRAGMENT_SHADER, fsSource);
    const prog = gl.createProgram();
    if (!prog) throw new Error('Failed to create program');
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      const log = gl.getProgramInfoLog(prog) ?? '(no log)';
      gl.deleteProgram(prog);
      throw new Error(`Program link failed: ${log}`);
    }
    return prog;
  }

  // --- Sprite pipeline (textured quads) ---
  const vsSource = `#version 300 es
precision highp float;

layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;

uniform vec2 u_resolution;
uniform vec2 u_translate;
uniform vec2 u_size;
uniform vec4 u_uvRect; // u0,v0,u1,v1

out vec2 v_uv;

void main() {
  vec2 pos = u_translate + a_pos * u_size;
  vec2 zeroToOne = pos / u_resolution;
  vec2 clip = zeroToOne * 2.0 - 1.0;
  gl_Position = vec4(clip * vec2(1.0, -1.0), 0.0, 1.0);

  v_uv = mix(u_uvRect.xy, u_uvRect.zw, a_uv);
}
`;

  const fsSource = `#version 300 es
precision highp float;

in vec2 v_uv;
uniform sampler2D u_tex;
out vec4 outColor;

void main() {
  outColor = texture(u_tex, v_uv);
}
`;

  const program = createProgram(vsSource, fsSource);

  const uResolution = gl.getUniformLocation(program, 'u_resolution');
  const uTranslate = gl.getUniformLocation(program, 'u_translate');
  const uSize = gl.getUniformLocation(program, 'u_size');
  const uUvRect = gl.getUniformLocation(program, 'u_uvRect');
  const uTex = gl.getUniformLocation(program, 'u_tex');

  if (!uResolution || !uTranslate || !uSize || !uUvRect || !uTex) {
    throw new Error('Missing shader uniforms');
  }

  // Quad geometry: (0,0) to (1,1)
  const vao = gl.createVertexArray();
  if (!vao) throw new Error('Failed to create VAO');
  gl.bindVertexArray(vao);

  const vbo = gl.createBuffer();
  if (!vbo) throw new Error('Failed to create VBO');
  gl.bindBuffer(gl.ARRAY_BUFFER, vbo);

  // interleaved: pos(x,y), uv(u,v)
  const quad = new Float32Array([
    // x, y, u, v
    0, 0, 0, 0,
    1, 0, 1, 0,
    0, 1, 0, 1,
    1, 1, 1, 1,
  ]);
  gl.bufferData(gl.ARRAY_BUFFER, quad, gl.STATIC_DRAW);

  const stride = 4 * 4;
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, stride, 0);
  gl.enableVertexAttribArray(1);
  gl.vertexAttribPointer(1, 2, gl.FLOAT, false, stride, 2 * 4);

  const ibo = gl.createBuffer();
  if (!ibo) throw new Error('Failed to create IBO');
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ibo);
  gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint16Array([0, 1, 2, 2, 1, 3]), gl.STATIC_DRAW);

  gl.bindVertexArray(null);

  // Texture
  const texture = gl.createTexture();
  if (!texture) throw new Error('Failed to create texture');
  gl.bindTexture(gl.TEXTURE_2D, texture);
  // 1x1 placeholder pixel until image loads
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([255, 0, 255, 255]));
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

  let texReady = false;
  const img = new Image();
  img.src = '/assets/sprites.png';
  img.onload = () => {
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
    texReady = true;
  };

  const tilesTexture = gl.createTexture();
  if (!tilesTexture) throw new Error('Failed to create tiles texture');
  gl.bindTexture(gl.TEXTURE_2D, tilesTexture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([0, 0, 0, 0]));
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

  let tilesReady = false;
  const tilesImg = new Image();
  tilesImg.src = '/assets/tiles.png';
  tilesImg.onload = () => {
    gl.bindTexture(gl.TEXTURE_2D, tilesTexture);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, tilesImg);
    tilesReady = true;
  };

  function resizeToDisplaySize() {
    const maxW = Math.max(1, window.innerWidth);
    const maxH = Math.max(1, window.innerHeight);

    const scale = Math.max(1, Math.floor(Math.min(maxW / BASE_WIDTH, maxH / BASE_HEIGHT)));

    const cssW = BASE_WIDTH * scale;
    const cssH = BASE_HEIGHT * scale;

    if (canvas.width !== BASE_WIDTH || canvas.height !== BASE_HEIGHT) {
      canvas.width = BASE_WIDTH;
      canvas.height = BASE_HEIGHT;
    }

    const cssWpx = `${cssW}px`;
    const cssHpx = `${cssH}px`;
    if (canvas.style.width !== cssWpx) canvas.style.width = cssWpx;
    if (canvas.style.height !== cssHpx) canvas.style.height = cssHpx;

    gl.viewport(0, 0, canvas.width, canvas.height);
  }

  function render(frame: FrameEvents) {
    gl.clearColor(0x10 / 0xff, 0x10 / 0xff, 0x18 / 0xff, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.useProgram(program);
    gl.bindVertexArray(vao);

    gl.uniform2f(uResolution, BASE_WIDTH, BASE_HEIGHT);

    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.uniform1i(uTex, 0);

    // If textures aren't ready yet, still draw with placeholder pixels.
    void texReady;
    void tilesReady;

    // --- Tilemap (behind sprites) ---
    gl.bindTexture(gl.TEXTURE_2D, tilesTexture);
    const tiles = buildVisibleTileSprites(frame.tilemap, {
      viewportW: BASE_WIDTH,
      viewportH: BASE_HEIGHT,
      cameraX: frame.camera.x,
      cameraY: frame.camera.y,
    });
    for (const t of tiles) {
      gl.uniform2f(uTranslate, t.x, t.y);
      gl.uniform2f(uSize, t.w, t.h);
      gl.uniform4f(uUvRect, t.u0, t.v0, t.u1, t.v1);
      gl.drawElements(gl.TRIANGLES, 6, gl.UNSIGNED_SHORT, 0);
    }

    // --- Sprites ---
    gl.bindTexture(gl.TEXTURE_2D, texture);
    for (const s of frame.sprites) {
      gl.uniform2f(uTranslate, s.x - frame.camera.x, s.y - frame.camera.y);
      gl.uniform2f(uSize, s.w, s.h);
      gl.uniform4f(uUvRect, s.u0, s.v0, s.u1, s.v1);
      gl.drawElements(gl.TRIANGLES, 6, gl.UNSIGNED_SHORT, 0);
    }

    gl.bindVertexArray(null);
  }

  resizeToDisplaySize();

  return { resizeToDisplaySize, render };
}
