// Mirrors wasm/src/render/plan.rs and render/scene3d.rs. Keep in sync.
export const PLAN = {
  VIEW_PROJ: 0,
  EYE: 16,
  TIME: 19,
  WORLD_MIX: 20,
  FUTURE_W: 21,
  FUTURE_H: 22,
  GLITCH: 23,
  CRT: 24,
  WARMTH: 25,
  VIGNETTE: 26,
  BLOOM: 27,
  CAM_RIGHT: 28,
  FUTURE_ENABLED: 31,
  FLOOR_Y: 32,
  FOCUS_X: 33,
  LEN: 34,
} as const;

export const BLOCK_FLOATS = 10;
export const BILLBOARD_FLOATS = 12;
export const SHADOW_FLOATS = 5;

export type FrameData = {
  retroVerts: Float32Array;
  retroBatches: Uint32Array;
  hudVerts: Float32Array;
  hudBatches: Uint32Array;
  plan: Float32Array;
  blocks: Float32Array;
  billboards: Float32Array;
  shadows: Float32Array;
};
