#!/usr/bin/env python3
"""Generate public/levels/level1.json, a regular Tiled map (open it in Tiled to edit).

The layout is described below as a list of building operations on a tile grid;
this script auto-tiles the ground, places decorations and writes the Tiled
JSON with the conventions from the design spec (§5.2):
  tile layers  terrain / collision / coins,  object layer  objects.

Usage: python3 tools/gen_level.py [--print]
"""

import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'public', 'levels', 'level1.json')

W, H = 220, 15
TS = 16
BASE = 12  # default ground top row

# Tileset ids (gid, firstgid = 1). Mirrors TILE_BUILDERS in gen_art.py.
GROUND_TL, GROUND_T, GROUND_TR, GROUND_L, GROUND_F, GROUND_R = 1, 2, 3, 4, 5, 6
BRICK, STONE, PLANK_L, PLANK_M, PLANK_R, GEM = 7, 8, 9, 10, 11, 12
BUSH_L, BUSH_R, FLOWER, TUFT, FENCE, SIGN, ROCK, CAVE = 13, 14, 15, 16, 17, 18, 19, 20
GROUND_PILLAR_TOP, GROUND_PILLAR = 21, 22
COIN_MARK, SOLID_MARK = 23, 24

# Cell kinds for the solid grid.
EMPTY, GROUND, BRICKK, STONEK, PLANKK, GEMK = 0, 1, 2, 3, 4, 5


class Level:
    def __init__(self):
        self.solid = [[EMPTY] * W for _ in range(H)]
        self.deco = {}  # (x, y) -> tile
        self.coins = set()
        self.objects = []

    # --- terrain -------------------------------------------------------
    def ground(self, x0, x1, top=BASE):
        """Ground columns x0..x1 (inclusive) with surface at row `top`."""
        for x in range(x0, x1 + 1):
            for y in range(top, H):
                self.solid[y][x] = GROUND

    def pit(self, x0, x1):
        for x in range(x0, x1 + 1):
            for y in range(H):
                if self.solid[y][x] == GROUND:
                    self.solid[y][x] = EMPTY

    def block(self, kind, x0, y0, x1=None, y1=None):
        x1 = x0 if x1 is None else x1
        y1 = y0 if y1 is None else y1
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.solid[y][x] = kind

    def plank(self, x0, x1, y):
        self.block(PLANKK, x0, y, x1, y)

    def stairs(self, x0, n, up=True, base=BASE):
        """Stone staircase of n steps standing on ground row `base`."""
        for i in range(n):
            h = i + 1 if up else n - i
            self.block(STONEK, x0 + i, base - h, x0 + i, base - 1)

    # --- content -------------------------------------------------------
    def coin_row(self, x0, x1, y):
        for x in range(x0, x1 + 1):
            self.coins.add((x, y))

    def coin_arc(self, x0, x1, y_top, height=2):
        n = x1 - x0
        for i, x in enumerate(range(x0, x1 + 1)):
            t = (i / n) * 2 - 1 if n else 0
            self.coins.add((x, y_top + round(height * t * t)))

    def obj(self, kind, x, feet_row, w=16, h=16):
        """Object whose bottom-center sits on top of row `feet_row` at column x."""
        self.objects.append({
            'kind': kind,
            'x': x * TS + TS / 2 - w / 2,
            'y': feet_row * TS - h,
            'w': w,
            'h': h,
        })

    def surface(self, x):
        for y in range(H):
            if self.solid[y][x] != EMPTY:
                return y
        return None

    def on_surface(self, kind, x, w=16, h=16):
        self.obj(kind, x, self.surface(x), w, h)

    def decorate(self, tile, x, y=None):
        if y is None:
            y = self.surface(x)
            assert y is not None, f'no surface at column {x}'
        y -= 1
        self.deco[(x, y)] = tile


def build():
    L = Level()

    # A. Tutorial meadow -------------------------------------------------
    L.ground(0, 24)
    L.on_surface('player_spawn', 3)
    L.decorate(SIGN, 6)
    L.decorate(BUSH_L, 1)
    L.decorate(BUSH_R, 2)
    L.decorate(FLOWER, 8)
    L.decorate(TUFT, 10)
    L.coin_row(9, 12, 9)
    L.block(BRICKK, 15, 10, 16, 11)             # first obstacle (2 high)
    L.coin_row(15, 16, 8)
    L.decorate(FENCE, 18)
    L.decorate(FENCE, 19)
    L.on_surface('enemy_spawn', 21)
    L.coin_arc(25, 27, 8)                      # 3-wide pit
    L.ground(28, 49)

    # B. Planks, plateau, dash boots ---------------------------------------
    L.plank(31, 34, 9)
    L.coin_row(31, 34, 7)
    L.decorate(ROCK, 38)
    L.ground(43, 49, top=10)                   # plateau, 2 tiles up
    L.on_surface('enemy_spawn', 45)            # patrol the plateau (turn at ledges)
    L.on_surface('enemy_spawn', 48)
    L.decorate(BUSH_L, 45)
    L.decorate(BUSH_R, 46)
    L.coin_arc(50, 53, 7)                      # 4-wide pit down from the plateau
    L.ground(54, 80)
    L.decorate(SIGN, 56)
    L.on_surface('dash_boots', 59)
    L.decorate(FLOWER, 61)
    # Brick block with a crawl space under it (slide for coins, or jump over).
    L.block(BRICKK, 64, 9, 68, 10)
    for x in range(64, 69):
        L.coins.add((x, 11))
        L.deco[(x, 11)] = CAVE
    L.on_surface('enemy_spawn', 72)
    L.on_surface('checkpoint', 76)
    L.decorate(TUFT, 78)

    # C. Pits and stone stairs --------------------------------------------
    L.pit(81, 88)                              # 8-wide gap with a stepping plank
    L.plank(83, 85, 11)
    L.coin_row(83, 85, 9)
    L.ground(89, 125)
    L.on_surface('rift', 90)                   # excursion 1: into the future
    L.on_surface('enemy_spawn', 108)
    L.stairs(97, 4, up=True)
    L.pit(101, 102)
    L.stairs(103, 4, up=False)
    L.coin_arc(100, 103, 5, height=1)
    L.decorate(FLOWER, 109)
    L.block(GEMK, 111, 8, 114, 8)
    L.coin_row(111, 114, 6)
    L.obj('enemy_spawn', 113, 8)
    L.decorate(BUSH_L, 120)
    L.decorate(BUSH_R, 121)
    L.on_surface('rift', 115)                  # back to the past before checkpoint 2
    L.on_surface('checkpoint', 117)

    # D. Big gap, enemy parade, mandatory slide tunnel ----------------------
    L.pit(126, 133)                            # 8 wide: boots jump, or use the pillar
    L.block(STONEK, 129, 12, 130, 14)           # stepping pillar
    L.coin_arc(126, 133, 6, height=2)
    L.ground(134, 166)
    L.on_surface('enemy_spawn', 141)
    L.on_surface('enemy_spawn', 145)
    L.on_surface('enemy_spawn', 148)
    L.decorate(SIGN, 149)                      # "slide!"
    # Wall reaching the top of the map with a 1-tile-high crawl space.
    L.block(BRICKK, 151, 0, 157, 10)
    for x in range(151, 158):
        L.deco[(x, 11)] = CAVE
    L.coin_row(152, 156, 11)
    L.decorate(TUFT, 159)

    # E. Climb, plateau, finale ----------------------------------------------
    L.on_surface('rift', 158)                  # excursion 2: out of the tunnel, into the future
    L.pit(167, 176)
    L.plank(168, 170, 10)
    L.plank(172, 174, 8)
    L.coin_row(172, 174, 6)
    L.ground(177, 186, top=9)                  # high plateau
    L.on_surface('checkpoint', 178)            # remembers the future
    L.on_surface('enemy_spawn', 182)
    L.decorate(FENCE, 180)
    L.decorate(FENCE, 181)
    L.ground(187, 219)
    L.on_surface('rift', 188)                  # back to the past for the last stretch
    L.coin_arc(189, 193, 7, height=1)
    L.pit(197, 199)
    L.coin_row(197, 199, 9)
    L.on_surface('enemy_spawn', 201)
    L.stairs(203, 5, up=True)
    L.on_surface('goal', 212, w=16, h=48)
    L.decorate(BUSH_L, 215)
    L.decorate(BUSH_R, 216)
    L.decorate(FLOWER, 218)
    return L


def is_ground(L, x, y):
    if x < 0 or x >= W:
        return True  # map edges continue the ground
    if y < 0:
        return False
    if y >= H:
        return True
    return L.solid[y][x] == GROUND


def autotile(L, x, y):
    k = L.solid[y][x]
    if k == BRICKK:
        return BRICK
    if k == STONEK:
        return STONE
    if k == GEMK:
        return GEM
    if k == PLANKK:
        left = x > 0 and L.solid[y][x - 1] == PLANKK
        right = x < W - 1 and L.solid[y][x + 1] == PLANKK
        return PLANK_M if left and right else (PLANK_R if left else PLANK_L)
    top = not is_ground(L, x, y - 1)
    lo = not is_ground(L, x - 1, y)
    ro = not is_ground(L, x + 1, y)
    if top:
        return GROUND_PILLAR_TOP if lo and ro else GROUND_TL if lo else GROUND_TR if ro else GROUND_T
    return GROUND_PILLAR if lo and ro else GROUND_L if lo else GROUND_R if ro else GROUND_F


def to_tiled(L):
    terrain, collision, coins = [], [], []
    for y in range(H):
        for x in range(W):
            if L.solid[y][x] != EMPTY:
                terrain.append(autotile(L, x, y))
                collision.append(SOLID_MARK)
            else:
                terrain.append(L.deco.get((x, y), 0))
                collision.append(0)
            coins.append(COIN_MARK if (x, y) in L.coins and L.solid[y][x] == EMPTY else 0)

    def tile_layer(lid, name, data, visible=True, opacity=1.0):
        return {
            'id': lid, 'name': name, 'type': 'tilelayer', 'x': 0, 'y': 0,
            'width': W, 'height': H, 'opacity': opacity, 'visible': visible, 'data': data,
        }

    objects = []
    for i, o in enumerate(L.objects):
        objects.append({
            'id': i + 1, 'name': o['kind'], 'type': o['kind'],
            'x': o['x'], 'y': o['y'], 'width': o['w'], 'height': o['h'],
            'rotation': 0, 'visible': True,
        })

    return {
        'type': 'map', 'version': '1.10', 'tiledversion': '1.10.2',
        'orientation': 'orthogonal', 'renderorder': 'right-down', 'infinite': False,
        'width': W, 'height': H, 'tilewidth': TS, 'tileheight': TS,
        'compressionlevel': -1, 'nextlayerid': 5, 'nextobjectid': len(objects) + 1,
        'backgroundcolor': '#5aa0f0',
        'tilesets': [{
            'firstgid': 1, 'name': 'tiles', 'image': '../assets/tiles.png',
            'imagewidth': 128, 'imageheight': 48, 'tilewidth': TS, 'tileheight': TS,
            'tilecount': 24, 'columns': 8, 'margin': 0, 'spacing': 0,
        }],
        'layers': [
            tile_layer(1, 'terrain', terrain),
            tile_layer(2, 'collision', collision, visible=False, opacity=0.5),
            tile_layer(3, 'coins', coins),
            {'id': 4, 'name': 'objects', 'type': 'objectgroup', 'draworder': 'topdown',
             'x': 0, 'y': 0, 'opacity': 1, 'visible': True, 'objects': objects},
        ],
    }


def write_compact_json(path, m):
    """Tiled-like formatting: one map row per line in tile layers."""
    text = json.dumps(m, indent=1)
    for layer in m['layers']:
        if 'data' not in layer:
            continue
        rows = [', '.join(str(v) for v in layer['data'][y * W:(y + 1) * W]) for y in range(H)]
        flat = json.dumps(layer['data'], indent=1)
        text = text.replace(flat.replace('\n', '\n   '), '[\n    ' + ',\n    '.join(rows) + '\n   ]', 1)
    with open(path, 'w') as f:
        f.write(text + '\n')


def print_map(L):
    marks = {o['kind']: c for c, o in zip('PEGCBR', [{'kind': k} for k in
             ['player_spawn', 'enemy_spawn', 'goal', 'checkpoint', 'dash_boots', 'rift']])}
    objs = {}
    for o in L.objects:
        cx = int((o['x'] + o['w'] / 2) // TS)
        cy = int((o['y'] + o['h'] - 1) // TS)
        objs[(cx, cy)] = marks[o['kind']]
    chars = {EMPTY: '.', GROUND: '#', BRICKK: 'b', STONEK: 's', PLANKK: '=', GEMK: 'g'}
    for y in range(H):
        line = ''
        for x in range(W):
            if (x, y) in objs:
                line += objs[(x, y)]
            elif (x, y) in L.coins:
                line += 'o'
            else:
                line += chars[L.solid[y][x]]
        print(line)


def build_test_rift():
    """Tiny flat level used by the e2e test: a rift right after the spawn, and
    a second one far enough ahead that the e2e walks into the future and back."""
    global W
    saved_w = W
    W = 60
    L = Level()
    L.ground(0, W - 1)
    L.on_surface('player_spawn', 2)
    L.on_surface('rift', 5)
    L.on_surface('rift', 30)
    L.on_surface('goal', 56, w=16, h=48)
    m = to_tiled(L)
    W = saved_w
    return m


def main():
    global W
    L = build()
    m = to_tiled(L)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    write_compact_json(OUT, m)
    print(f'wrote {OUT}: {W}x{H}, {len(L.coins)} coins, {len(L.objects)} objects')
    test_out = os.path.join(ROOT, 'public', 'levels', 'test_rift.json')
    m2 = build_test_rift()
    with open(test_out, 'w') as f:
        json.dump(m2, f)
    print(f'wrote {test_out}')
    if '--print' in sys.argv:
        print_map(L)


if __name__ == '__main__':
    main()
