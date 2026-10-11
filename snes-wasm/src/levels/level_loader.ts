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

export async function loadLevel(url: string): Promise<LevelData> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to load level: ${res.status}`);
  const json = (await res.json()) as LevelData;
  return json;
}
