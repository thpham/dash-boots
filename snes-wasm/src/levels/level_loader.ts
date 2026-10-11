export type LevelData = {
  width: number;
  height: number;
  tileSize: number;
  terrain: number[];
  collision: number[];
  coins: number[];
  spawns: {
    player: { x: number; y: number };
    enemies: { x: number; y: number }[];
    goal: { x: number; y: number };
  };
};

function requireLayer(json: any, name: string) {
  const layer = (json.layers ?? []).find((l: any) => l.name === name);
  if (!layer) throw new Error(`Missing Tiled layer: ${name}`);
  return layer;
}

function requireObject(json: any, type: string) {
  const objectsLayer = requireLayer(json, 'objects');
  const obj = (objectsLayer.objects ?? []).find(
    (o: any) => o.type === type || o.name === type,
  );
  if (!obj) throw new Error(`Missing Tiled object: ${type}`);
  return obj;
}

function requireLayerData(json: any, name: string): number[] {
  const layer = requireLayer(json, name);
  const data = layer.data;
  if (!Array.isArray(data)) throw new Error(`Tiled layer missing data: ${name}`);
  return data;
}

export async function loadLevel(url: string): Promise<LevelData> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to load level: ${res.status}`);
  const json = (await res.json()) as any;

  const width = json.width;
  const height = json.height;
  const tileSize = json.tilewidth;
  if (typeof width !== 'number' || typeof height !== 'number') {
    throw new Error('Tiled JSON missing width/height');
  }
  if (typeof tileSize !== 'number') throw new Error('Tiled JSON missing tilewidth');

  const expectedLen = width * height;

  const terrainGids = requireLayerData(json, 'terrain');
  const collisionGids = requireLayerData(json, 'collision');
  const coinsGids = requireLayerData(json, 'coins');

  if (terrainGids.length !== expectedLen) {
    throw new Error(`Tiled layer terrain length ${terrainGids.length} != ${expectedLen}`);
  }
  if (collisionGids.length !== expectedLen) {
    throw new Error(
      `Tiled layer collision length ${collisionGids.length} != ${expectedLen}`,
    );
  }
  if (coinsGids.length !== expectedLen) {
    throw new Error(`Tiled layer coins length ${coinsGids.length} != ${expectedLen}`);
  }

  const terrain = terrainGids.map((gid: number) => gid);
  const collision = collisionGids.map((gid: number) => (gid > 0 ? 1 : 0));
  const coins = coinsGids.map((gid: number) => (gid > 0 ? 1 : 0));

  const playerSpawn = requireObject(json, 'player_spawn');
  const goalSpawn = requireObject(json, 'goal');

  const objectsLayer = requireLayer(json, 'objects');
  const enemySpawns = (objectsLayer.objects ?? []).filter(
    (o: any) => o.type === 'enemy_spawn' || o.name === 'enemy_spawn',
  );
  if (enemySpawns.length === 0) throw new Error('Missing Tiled object: enemy_spawn');

  return {
    width,
    height,
    tileSize,
    terrain,
    collision,
    coins,
    spawns: {
      player: { x: playerSpawn.x, y: playerSpawn.y },
      enemies: enemySpawns.map((o: any) => ({ x: o.x, y: o.y })),
      goal: { x: goalSpawn.x, y: goalSpawn.y },
    },
  };
}
