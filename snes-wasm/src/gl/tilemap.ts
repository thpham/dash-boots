import type { SpriteDraw, TilemapRenderData } from './renderer';

export type TilemapDrawOpts = {
  viewportW: number;
  viewportH: number;
  cameraX: number;
  cameraY: number;
};

// Very small helper: convert a tilemap into a list of sprite-like quads.
// We keep it simple: brute-force viewport culling.
export function buildVisibleTileSprites(
  tilemap: TilemapRenderData,
  opts: TilemapDrawOpts,
): SpriteDraw[] {
  const { width, height, tileSize, terrain } = tilemap;

  const startCol = Math.max(0, Math.floor(opts.cameraX / tileSize));
  const startRow = Math.max(0, Math.floor(opts.cameraY / tileSize));

  const endCol = Math.min(
    width,
    Math.ceil((opts.cameraX + opts.viewportW) / tileSize) + 1,
  );
  const endRow = Math.min(
    height,
    Math.ceil((opts.cameraY + opts.viewportH) / tileSize) + 1,
  );

  const out: SpriteDraw[] = [];

  for (let row = startRow; row < endRow; row++) {
    for (let col = startCol; col < endCol; col++) {
      const idx = row * width + col;
      const gid = terrain[idx] ?? 0;
      if (gid === 0) continue;

      // Placeholder tileset: 1 tile at (0,0) in tiles.png.
      out.push({
        x: col * tileSize - opts.cameraX,
        y: row * tileSize - opts.cameraY,
        w: tileSize,
        h: tileSize,
        u0: 0,
        v0: 0,
        u1: 1,
        v1: 1,
      });
    }
  }

  return out;
}
