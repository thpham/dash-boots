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
LEVELS_DIR = os.path.join(ROOT, 'public', 'levels')

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
    def __init__(self, w=W):
        self.w = w
        self.solid = [[EMPTY] * w for _ in range(H)]
        # Highest walkable scan row (skipped rows = fixed roof, e.g. the cave
        # ceiling): surface/objects anchor to what is below the roof.
        self.roof = 0
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
        for y in range(self.roof, H):
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


def build_meadow():
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


def build_sunset():
    """Level 2 - rolling sunset ridges. A notch harder than the meadow: wider
    coin arcs, plank staircases over a big gap, more patrolling enemies, one
    visit to the future. Same building blocks, everything a bit broader.
    (Shape follows level1's sections so the naive bot stays comfortable.)"""
    L = Level(200)
    # A: gentle start ------------------------------------------------------
    L.ground(0, 29)
    L.decorate(BUSH_L, 1)
    L.decorate(BUSH_R, 2)
    L.on_surface('player_spawn', 3)
    L.decorate(SIGN, 6)
    L.decorate(TUFT, 8)
    L.coin_row(9, 12, 9)
    L.on_surface('enemy_spawn', 14)
    L.decorate(FENCE, 18)
    L.decorate(FENCE, 19)
    L.block(BRICKK, 22, 10, 25, 11)             # low wall with coins on top
    L.coin_arc(22, 25, 8)
    L.pit(26, 28)                               # first pit (3 wide)
    L.coin_arc(26, 28, 9)

    # B: plank terrace climb, plateau ---------------------------------------
    L.ground(30, 74)
    L.ground(40, 60, top=9)                     # upper terrace (reachable +3)
    L.plank(33, 36, 9)
    L.coin_row(33, 36, 7)
    L.on_surface('enemy_spawn', 45)
    L.on_surface('enemy_spawn', 52)
    L.decorate(BUSH_L, 47)
    L.decorate(BUSH_R, 48)
    L.coin_arc(44, 58, 6, height=2)
    L.pit(62, 63)                               # small post-plateau gap
    L.on_surface('dash_boots', 65)              # on the lower ground, terrace exit
    L.decorate(FLOWER, 68)
    L.decorate(ROCK, 71)

    # C: stone stairs up, two-wide pit, stairs down -------------------------
    L.ground(75, 118)
    L.stairs(80, 4, up=True)
    L.pit(84, 85)
    L.stairs(86, 4, up=False)
    L.coin_arc(83, 86, 5, height=1)
    L.on_surface('checkpoint', 90)
    L.on_surface('rift', 92)                    # excursion 1: into the future
    L.on_surface('enemy_spawn', 101)
    L.block(GEMK, 105, 8, 108, 8)               # gem platform, enemy on top
    L.coin_row(105, 108, 6)
    L.obj('enemy_spawn', 107, 8)
    L.decorate(ROCK, 112)
    L.on_surface('rift', 112)                   # ... and back before checkpoint 2
    L.on_surface('checkpoint', 118)

    # D: wide valley on planks, crawl tunnel -------------------------------
    L.pit(119, 128)                             # 10 wide: plank + stepping pillar
    L.plank(120, 122, 11)
    L.block(STONEK, 125, 12, 126, 14)
    L.coin_arc(119, 128, 7, height=2)
    L.ground(129, 160)
    L.on_surface('enemy_spawn', 136)
    L.on_surface('enemy_spawn', 140)
    L.decorate(SIGN, 144)                       # "slide!"
    L.block(BRICKK, 146, 0, 152, 10)            # wall to the sky, crawl under
    for x in range(146, 153):
        L.deco[(x, 11)] = CAVE
    L.coin_row(147, 151, 11)
    L.decorate(TUFT, 154)

    # E: ridge climb and finale --------------------------------------------
    L.pit(161, 168)                             # 8 wide: plank hops
    L.plank(162, 164, 10)
    L.plank(166, 167, 8)
    L.coin_row(166, 167, 6)
    L.ground(169, 178, top=9)                   # high plateau, checkpoint
    L.on_surface('checkpoint', 171)             # remembers the future (rift next)
    L.on_surface('rift', 172)
    L.decorate(FENCE, 173)
    L.ground(179, 199)
    L.coin_arc(183, 188, 8, height=1)
    L.on_surface('enemy_spawn', 185)
    L.on_surface('rift', 189)                   # back to the past for the last stretch
    L.on_surface('goal', 196, w=16, h=48)
    L.decorate(BUSH_L, 194)
    L.decorate(FLOWER, 198)
    return L


def build_cave():
    """Level 3 - crystal cave. Capped corridors for the cave feel, mandatory
    crawl tunnels, gem platforms (bump the enemy standing on them!), a rift
    excursion in the dark, plank bridges over flooded stretches."""
    L = Level(210)
    L.block(GROUND, 0, 0, 209, 4)               # cave roof across the whole map
    L.roof = 5
    L.decorate(CAVE, 5, 1)
    L.decorate(CAVE, 40, 1)
    L.decorate(CAVE, 80, 1)
    L.decorate(CAVE, 120, 1)
    L.decorate(CAVE, 160, 1)
    L.decorate(CAVE, 200, 1)
    # A: entry corridor ------------------------------------------------------
    L.ground(0, 55)
    L.decorate(CAVE, 1, 5)
    L.decorate(CAVE, 2, 5)
    L.on_surface('player_spawn', 3)
    L.decorate(SIGN, 6)
    L.decorate(ROCK, 9)
    L.coin_row(10, 13, 9)
    L.on_surface('enemy_spawn', 17)
    L.block(STONEK, 24, 10, 26, 11)             # low stone wall, coins above
    L.coin_arc(24, 26, 8)
    L.pit(28, 30)                               # flooded stretch (3 wide)
    L.coin_arc(28, 30, 9)
    L.decorate(CAVE, 34, 5)
    L.decorate(ROCK, 38)
    L.on_surface('enemy_spawn', 42)

    # B: gem terraces --------------------------------------------------------
    L.pit(50, 53)                               # 4 wide, jump or plank hop
    L.plank(51, 52, 10)
    L.ground(54, 62, top=10)                    # first terrace
    L.on_surface('enemy_spawn', 58)
    L.coin_row(57, 60, 8)
    L.ground(63, 66, top=12)
    L.ground(67, 78, top=8)                     # second terrace, higher
    L.block(STONEK, 67, 9, 78, 11)              # support column look
    L.on_surface('checkpoint', 68)
    L.coin_row(70, 75, 6)
    L.decorate(CAVE, 74, 4)
    L.ground(79, 82, top=12)
    L.ground(83, 117)
    L.on_surface('dash_boots', 86)
    L.decorate(ROCK, 89)

    # C: crawl tunnel, gem shelf with an enemy on it --------------------------
    L.block(BRICKK, 97, 0, 103, 10)             # wall to the roof, crawl under
    for x in range(97, 104):
        L.deco[(x, 11)] = CAVE
    L.coin_row(98, 102, 11)
    L.decorate(TUFT, 106)
    L.on_surface('enemy_spawn', 109)
    L.block(GEMK, 112, 9, 116, 9)
    L.coin_row(112, 116, 7)
    L.obj('enemy_spawn', 114, 9)

    # D: flooded stretch with stepping pillars --------------------------------
    L.pit(118, 129)                             # 12 wide: plank, pillars, plank
    L.plank(119, 121, 10)
    L.block(STONEK, 123, 12, 124, 14)
    L.plank(126, 128, 9)
    L.coin_arc(118, 129, 6, height=2)
    L.ground(130, 168)
    L.on_surface('checkpoint', 133)
    L.on_surface('rift', 136)                   # excursion: the future underfoot
    L.on_surface('enemy_spawn', 146)
    L.on_surface('enemy_spawn', 152)
    L.block(BRICKK, 144, 10, 147, 11)           # little hop-wall with coins
    L.coin_arc(144, 147, 8)
    L.decorate(CAVE, 149, 5)
    L.stairs(156, 3, up=True)
    L.pit(160, 162)
    L.stairs(163, 3, up=False)
    L.coin_arc(159, 163, 7, height=1)

    # E: the climb to daylight -----------------------------------------------
    L.pit(169, 176)                             # 8 wide, planks
    L.plank(170, 172, 10)
    L.plank(174, 175, 8)
    L.coin_row(174, 175, 6)
    L.ground(177, 186, top=9)
    L.on_surface('rift', 178)                   # back to the past up here
    L.on_surface('enemy_spawn', 185)
    L.decorate(FENCE, 181)
    L.ground(187, 209)
    L.on_surface('checkpoint', 190)
    L.pit(193, 195)
    L.coin_row(193, 195, 9)
    L.on_surface('enemy_spawn', 198)
    L.on_surface('goal', 205, w=16, h=48)
    L.decorate(ROCK, 201)
    L.decorate(BUSH_R, 207)
    return L


def build_night():
    """Level 4 - moonlit night. Broadest gaps, the most enemies and one double
    crawl section; boots arrive early and checkpoints are generous, so it is
    hard but fair."""
    L = Level(200)
    # A: meadow in moonlight -------------------------------------------------
    L.ground(0, 39)
    L.decorate(BUSH_L, 1)
    L.on_surface('player_spawn', 3)
    L.decorate(SIGN, 6)
    L.coin_arc(8, 12, 9)
    L.on_surface('enemy_spawn', 13)
    L.block(BRICKK, 18, 10, 21, 11)
    L.coin_arc(18, 21, 8)
    L.on_surface('enemy_spawn', 24)
    L.pit(30, 34)                               # 5 wide: clears with a running jump
    L.coin_arc(30, 34, 9)

    # B: terrace over a wide valley ------------------------------------------
    L.ground(35, 80)
    L.pit(36, 37)                               # notch in the terrace
    L.pit(40, 41)
    L.plank(40, 41, 10)
    L.on_surface('dash_boots', 45)
    L.ground(48, 62, top=9)
    L.pit(49, 50)                               # notch in the upper terrace
    L.on_surface('enemy_spawn', 54)
    L.on_surface('enemy_spawn', 60)
    L.coin_arc(52, 60, 6, height=1)
    L.decorate(FENCE, 64)
    L.decorate(FENCE, 65)
    L.decorate(BUSH_R, 70)

    # C: double crawl ---------------------------------------------------------
    L.block(BRICKK, 74, 0, 80, 10)              # first wall-to-sky
    for x in range(74, 81):
        L.deco[(x, 11)] = CAVE
    L.coin_row(75, 79, 11)
    L.ground(81, 128)
    L.block(BRICKK, 84, 0, 90, 10)              # second wall-to-sky, coins in both
    for x in range(84, 91):
        L.deco[(x, 11)] = CAVE
    L.coin_row(85, 89, 11)
    L.on_surface('checkpoint', 94)
    L.on_surface('rift', 100)                   # excursion: night to neon
    L.block(GEMK, 108, 8, 111, 8)
    L.coin_row(108, 111, 6)
    L.obj('enemy_spawn', 110, 8)
    L.stairs(114, 4, up=True)
    L.pit(118, 119)
    L.stairs(120, 4, up=False)
    L.coin_arc(117, 120, 5, height=1)
    L.on_surface('rift', 124)                   # back to the past
    L.on_surface('checkpoint', 126)

    # D: the stepping-stone marsh --------------------------------------------
    L.ground(129, 168)
    L.pit(131, 141)                             # 11 wide: pillar hops
    L.block(STONEK, 133, 12, 134, 14)
    L.block(STONEK, 137, 11, 138, 14)
    L.plank(140, 141, 10)
    L.coin_arc(131, 141, 6, height=2)
    L.on_surface('enemy_spawn', 147)
    L.decorate(TUFT, 150)
    L.on_surface('enemy_spawn', 154)
    L.block(BRICKK, 158, 0, 164, 10)            # last crawl, guarded by an enemy
    for x in range(158, 165):
        L.deco[(x, 11)] = CAVE
    L.coin_row(159, 163, 11)
    L.on_surface('enemy_spawn', 167)

    # E: long ridge, finale ----------------------------------------------------
    L.pit(169, 177)                             # 9 wide: plank, pillar, plank
    L.plank(170, 171, 10)
    L.block(STONEK, 174, 12, 175, 14)
    L.plank(176, 177, 8)
    L.coin_arc(169, 177, 6, height=1)
    L.ground(178, 199)
    L.on_surface('checkpoint', 181)
    L.on_surface('enemy_spawn', 190)
    L.coin_arc(186, 190, 8, height=1)
    L.on_surface('goal', 195, w=16, h=48)
    L.decorate(BUSH_L, 193)
    L.decorate(BUSH_R, 196)
    return L


def is_ground(L, x, y):
    if x < 0 or x >= L.w:
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
        right = x < L.w - 1 and L.solid[y][x + 1] == PLANKK
        return PLANK_M if left and right else (PLANK_R if left else PLANK_L)
    top = not is_ground(L, x, y - 1)
    lo = not is_ground(L, x - 1, y)
    ro = not is_ground(L, x + 1, y)
    if top:
        return GROUND_PILLAR_TOP if lo and ro else GROUND_TL if lo else GROUND_TR if ro else GROUND_T
    return GROUND_PILLAR if lo and ro else GROUND_L if lo else GROUND_R if ro else GROUND_F


def to_tiled(L, bg='#5aa0f0'):
    terrain, collision, coins = [], [], []
    for y in range(H):
        for x in range(L.w):
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
            'width': L.w, 'height': H, 'opacity': opacity, 'visible': visible, 'data': data,
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
        'width': L.w, 'height': H, 'tilewidth': TS, 'tileheight': TS,
        'compressionlevel': -1, 'nextlayerid': 5, 'nextobjectid': len(objects) + 1,
        'backgroundcolor': bg,
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
    w = m['width']
    text = json.dumps(m, indent=1)
    for layer in m['layers']:
        if 'data' not in layer:
            continue
        rows = [', '.join(str(v) for v in layer['data'][y * w:(y + 1) * w]) for y in range(len(layer['data']) // w)]
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
        for x in range(L.w):
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
    L = Level(60)
    L.ground(0, L.w - 1)
    L.on_surface('player_spawn', 2)
    L.on_surface('rift', 5)
    L.on_surface('rift', 30)
    L.on_surface('goal', 56, w=16, h=48)
    return to_tiled(L)


def main():
    shipped = [
        ('level1', build_meadow, '#5aa0f0'),
        ('level2', build_sunset, '#e8734a'),
        ('level3', build_cave, '#101433'),
        ('level4', build_night, '#0a1030'),
    ]
    os.makedirs(LEVELS_DIR, exist_ok=True)
    for name, build, bg in shipped:
        path = os.path.join(LEVELS_DIR, f'{name}.json')
        L = build()
        write_compact_json(path, to_tiled(L, bg))
        print(f'wrote {path}: {L.w}x{H}, {len(L.coins)} coins, {len(L.objects)} objects')
        if '--print' in sys.argv:
            print_map(L)
            print()
    test_out = os.path.join(LEVELS_DIR, 'test_rift.json')
    with open(test_out, 'w') as f:
        json.dump(build_test_rift(), f)
    print(f'wrote {test_out}')


if __name__ == '__main__':
    main()
