export async function loadLevel(url: string) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to load level: ${res.status}`);
  const json = await res.json();
  return json;
}
