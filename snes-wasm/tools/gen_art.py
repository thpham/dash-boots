#!/usr/bin/env python3
"""Generate all original pixel art for the game (stdlib only, deterministic).

Outputs (in public/assets/):
  sprites.png  256x128  sprite atlas (layout mirrored in wasm/src/sprites.rs)
  tiles.png    128x48   16x16 tileset (gid = index + 1, shared by every level)
  font.png     128x48   8x8 bitmap font, ASCII 32..127 (+ colored icons on '#', '$', '^')
  bg_far.png   320x180  sky + mountains + clouds (opaque, horizontally tileable)
  bg_near.png  320x180  hills (transparent top, horizontally tileable)
  plus same-dimension theme variants (exact color remaps) for the campaign:
  tiles_{sunset,cave,night}.png, bg_{far,near}_{sunset,cave,night}.png

The future 3D pass keys the sky out of bg_far's sampled band (row 76 down):
theme skies must keep g > r + 31 there and mountain remaps g - r <= 24
(checked by asserts in main() — see FAR_THEMES).

Usage: python3 tools/gen_art.py [--preview DIR]
"""

import math
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pnglib import Canvas, hex_rgba, preview  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'public', 'assets')

C = hex_rgba
OUTLINE = C('#20142c')

# --------------------------------------------------------------------------
# Hero (16x24 frames, facing right)
# --------------------------------------------------------------------------

HERO_PAL = {
    'K': OUTLINE,
    'C': C('#28b0a0'),  # jacket / cap
    'c': C('#16786e'),
    'W': C('#f4f4f4'),
    'S': C('#f6c18c'),  # skin
    's': C('#d08a5a'),
    'H': C('#5a2e18'),  # hair
    'Y': C('#f8d040'),  # scarf
    'y': C('#c89820'),
    'P': C('#4a4a62'),  # pants
    'B': C('#8a5030'),  # boots
}
HERO_BOOTS_PAL = dict(HERO_PAL, B=C('#ff5a3a'), P=C('#4a4a62'))

HEAD = [
    "................",
    "................",
    ".....KKKKKK.....",
    "....KCCCCCCKK...",
    "...KCCWCCCCCCK..",
    "...KCCCCCCCCCCK.",
    "...KccccccKKKKKK",
    "...KHHSSSSSSSK..",
    "...KHSSSSSKSSK..",
    "...KHSSSSSKSSSK.",
    "...KHSsSSSSSSSK.",
    "....KsSSSSSKKK..",
    ".....KKSSSSK....",
]

TORSO_IDLE = [
    "....KYYYYYYK....",
    "...KCYyyyyYCK...",
    "..KSKCCCCCCKSK..",
    "..KSKCCCCCCKSK..",
    "...KKcCCCCcKK...",
]
TORSO_JUMP = [
    "....KYYYYYYK.KK.",
    "...KCYyyyyYCKSK.",
    "..KSKCCCCCCCKK..",
    "..KSKCCCCCCK....",
    "...KKcCCCCcK....",
]
TORSO_FALL = [
    "..KK.KYYYYYYK.KK",
    "..KSKCYyyyyYCKSK",
    "...KKCCCCCCCKK..",
    "....KCCCCCCK....",
    "....KcCCCCcK....",
]
TORSO_DASH = [
    "yYYYKYYYYYYK....",
    ".yyKCCYyyyYCK...",
    "...KCCCCCCCKSSK.",
    "...KCCCCCCCKKK..",
    "...KKcCCCCcK....",
]

LEGS_IDLE = [
    "....KPPPPPPK....",
    "....KPPKKPPK....",
    "....KPPKKPPK....",
    "...KBBBKKBBBK...",
    "...KBBBBKBBBBK..",
    "...KKKKKKKKKKK..",
]
LEGS_RUN1 = [
    "....KPPPPPPK....",
    "...KPPPKKPPPK...",
    "..KPPK....KPPK..",
    ".KBBK......KBBK.",
    "KBBBK......KBBBK",
    "KKKKK......KKKKK",
]
LEGS_RUN2 = [
    "....KPPPPPPK....",
    ".....KPPPPK.....",
    ".....KPPPPK.....",
    "....KBBBBBK.....",
    "....KBBBBBBK....",
    "....KKKKKKKK....",
]
LEGS_RUN3 = [
    "....KPPPPPPK....",
    "....KPPPKPPPK...",
    "...KPPK..KPPK...",
    "..KBBBK..KBBK...",
    "..KBBBBK.KBBBK..",
    "..KKKKKK.KKKKK..",
]
LEGS_JUMP = [
    "....KPPPPPPK....",
    "...KPPPKPPPPK...",
    "..KBBPK..KPPK...",
    "..KBBBK..KBBBK..",
    "..KKKK...KBBBBK.",
    "..........KKKKK.",
]

BLANK = "................"


def hero_frame(torso, legs):
    rows = HEAD + torso + legs
    assert len(rows) == 24
    return rows


def hero_crouch():
    head = HEAD[2:]  # 11 rows
    rows = [BLANK] * 10 + head + [
        "..KSKYYYYYYKSK..",
        "..KBBBKKKKBBBK..",
        "..KKKKK..KKKKK..",
    ]
    assert len(rows) == 24
    return rows


def hero_slide():
    head = HEAD[2:]
    rows = [BLANK] * 10 + head + [
        "..KCCCCCCCKKBBK.",
        "..KPPPPPPPKBBBBK",
        "...KKKKKKKKKKKKK",
    ]
    assert len(rows) == 24
    return rows


# Order must match wasm/src/sprites.rs (PlayerFrame).
HERO_FRAMES = [
    hero_frame(TORSO_IDLE, LEGS_IDLE),   # 0 idle
    hero_frame(TORSO_IDLE, LEGS_RUN1),   # 1 run1
    hero_frame(TORSO_IDLE, LEGS_RUN2),   # 2 run2
    hero_frame(TORSO_IDLE, LEGS_RUN3),   # 3 run3
    hero_frame(TORSO_JUMP, LEGS_JUMP),   # 4 jump
    hero_frame(TORSO_FALL, LEGS_RUN3),   # 5 fall
    hero_crouch(),                       # 6 crouch
    hero_slide(),                        # 7 slide
    hero_frame(TORSO_DASH, LEGS_RUN1),   # 8 dash
]

# --------------------------------------------------------------------------
# Enemy "Grumble" (16x16)
# --------------------------------------------------------------------------

ENEMY_PAL = {
    'K': OUTLINE,
    'M': C('#9a52c8'),
    'm': C('#5e2a86'),
    'W': C('#f4f4f4'),
    'F': C('#f0a840'),
}

ENEMY_BODY = [
    "................",
    "................",
    ".....KKKKKK.....",
    "...KKMMMMMMKK...",
    "..KMMMMMMMMMMK..",
    ".KMmMMMMMMMMmMK.",
    ".KMKKMMMMMMKKMK.",
    "KMMMKKWMMWKKMMMK",
    "KMMMWKWMMWKWMMMK",
    "KMMMWWWMMWWWMMMK",
    "KmMMMMMMMMMMMMmK",
    ".KmMMKKKKKKMMmK.",
    "..KmmmmmmmmmmK..",
]
ENEMY_WALK1 = ENEMY_BODY + [
    "...KKFFKKFFKK...",
    "..KFFFFKKFFFFK..",
    "..KKKKK..KKKKK..",
]
ENEMY_WALK2 = ENEMY_BODY + [
    "...KKFFKKFFKK...",
    "...KFFFFKFFFFK..",
    "...KKKKK.KKKKK..",
]
ENEMY_SQUASH = [BLANK] * 9 + [
    "....KKKKKKKK....",
    "..KKMMMMMMMMKK..",
    ".KMMKKMMMMKKMMK.",
    "KMMMKKMMMMKKMMMK",
    "KmmmmmmmmmmmmmmK",
    ".KKFFKKKKKKFFKK.",
    "..KKK......KKK..",
]

# --------------------------------------------------------------------------
# Items
# --------------------------------------------------------------------------

BOOTS_PAL = {
    'K': OUTLINE,
    'O': C('#ff6a3a'),
    'W': C('#f4f4f4'),
    'd': C('#6a3020'),
}
BOOTS_PICKUP = [
    "................",
    "................",
    "......KKKK......",
    ".....KOOOOK.....",
    ".....KOWWOK.....",
    ".....KOOOOK.....",
    ".WW..KOWWOK.....",
    "WWWWKKOOOOK.....",
    ".WWWKOOOOOKK....",
    "..WKOOOOOOOOKK..",
    "..KOOOOOOOOOOOK.",
    "..KOWWWWOOOOOOK.",
    "..KOOOOOOOOOOOK.",
    "..KKKKKKKKKKKKK.",
    "..KddddddddddK..",
    "...KKKKKKKKKK...",
]

GOLD = C('#f8c838')
GOLD_DARK = C('#c88a18')
GOLD_LIGHT = C('#fff4b0')
GOLD_OUTLINE = C('#6a3a10')


def draw_coin(cv, ox, oy, squeeze):
    """Coin of 10x12 pixels, horizontally squeezed for the spin animation."""
    cx, cy = ox + 7.5, oy + 7.5
    rx, ry = 5.0 * squeeze, 6.0
    for y in range(16):
        for x in range(16):
            dx = (x + 0.5 + ox - 0.5 - cx + 0.5) / max(rx, 0.6)
            dy = (y + oy + 0.5 - cy) / ry
            d = dx * dx + dy * dy
            if d > 1.0:
                continue
            inner = ((x + ox + 0.5 - cx) / max(rx - 1.2, 0.3)) ** 2 + ((y + oy + 0.5 - cy) / (ry - 1.2)) ** 2
            if inner > 1.0 or rx < 1.0:
                col = GOLD_OUTLINE
            elif (x + ox + 0.5 - cx) > rx * 0.35:
                col = GOLD_DARK
            elif (x + ox + 0.5 - cx) < -rx * 0.25 and squeeze > 0.5:
                col = GOLD_LIGHT
            else:
                col = GOLD
            cv.set(ox + x, oy + y, col)
    if squeeze > 0.8:
        # Engraved center line.
        for y in range(oy + 5, oy + 11):
            cv.set(ox + 8, y, GOLD_DARK)


def draw_goal(cv, ox, oy):
    """16x48 goal flagpole with a star on top and a big checkered flag."""
    pole = C('#d8d8e8')
    pole_dark = C('#8888a8')
    for y in range(6, 46):
        cv.set(ox + 3, oy + y, OUTLINE)
        cv.set(ox + 4, oy + y, pole)
        cv.set(ox + 5, oy + y, pole_dark)
        cv.set(ox + 6, oy + y, OUTLINE)
    # Base block.
    cv.rect(ox + 1, oy + 44, 8, 4, OUTLINE)
    cv.rect(ox + 2, oy + 45, 6, 2, C('#686880'))
    # Star top.
    star = [
        "...K...",
        "..KYK..",
        "KKYYYKK",
        ".KYYYK.",
        ".KYKYK.",
        "KK...KK",
    ]
    cv.grid(star, {'K': OUTLINE, 'Y': C('#ffe040')}, ox + 1, oy)
    # Flag (checkered), to the right of the pole.
    for y in range(7, 19):
        for x in range(7, 16):
            edge = x == 15 or y == 7 or y == 18
            if edge:
                cv.set(ox + x, oy + y, OUTLINE)
            else:
                checker = ((x - 7) // 2 + (y - 7) // 2) % 2 == 0
                cv.set(ox + x, oy + y, C('#f4f4f4') if checker else C('#e04848'))


def draw_checkpoint(cv, ox, oy, active, wave):
    """16x32 checkpoint post; flag down (inactive) or raised (active)."""
    pole = C('#b0b0c8')
    for y in range(4, 30):
        cv.set(ox + 3, oy + y, OUTLINE)
        cv.set(ox + 4, oy + y, pole)
        cv.set(ox + 5, oy + y, OUTLINE)
    cv.rect(ox + 1, oy + 28, 7, 4, OUTLINE)
    cv.rect(ox + 2, oy + 29, 5, 2, C('#686880'))
    cv.rect(ox + 2, oy + 2, 5, 3, OUTLINE)
    cv.rect(ox + 3, oy + 3, 3, 1, C('#ffe040') if active else C('#808098'))
    flag_col = C('#48d058') if active else C('#c04040')
    shade = C('#2a8a38') if active else C('#802828')
    top = 5 if active else 18
    for y in range(7):
        for x in range(9):
            if x > 8 - abs(y - 3) * 2:
                continue
            yy = top + y + (1 if wave and x > 4 else 0)
            col = OUTLINE if (y == 0 or y == 6 or x == 0) else (shade if y > 3 else flag_col)
            cv.set(ox + 6 + x, oy + yy, col)


def draw_particles(cv, ox, oy):
    # Dust puffs (3 frames, shrinking), 8x8 each.
    dust = C('#f0e8d8')
    dust_shade = C('#c0b090')
    for i, r in enumerate([3.4, 2.6, 1.6]):
        for y in range(8):
            for x in range(8):
                d = math.hypot(x + 0.5 - 4, y + 0.5 - 4)
                if d <= r:
                    cv.set(ox + i * 8 + x, oy + y, dust_shade if (x + y) > 7 + i else dust)
    # Sparkle (2 frames).
    sp = C('#fff8c0')
    for i, n in enumerate([3, 2]):
        cx = ox + 24 + i * 8 + 3
        cy = oy + 3
        for k in range(-n, n + 1):
            cv.set(cx + k, cy, sp)
            cv.set(cx, cy + k, sp)
        cv.set(cx, cy, C('#ffffff'))
    # Stomp stars (3 frames).
    st = C('#ffffff')
    for i, r in enumerate([1, 2, 3]):
        bx = ox + 40 + i * 8
        for k in range(-r, r + 1):
            cv.set(bx + 3 + k, oy + 3 + k, st)
            cv.set(bx + 3 + k, oy + 3 - k, st)


# Rift: a crack of light that does not belong here (cold, translucent, glitchy).
RIFT_CORE = C('#ffffff')
RIFT_ICE = C('#c8f6ff')
RIFT_CYAN = C('#48dcf4')
RIFT_DEEP = C('#2a86c8', 220)
RIFT_HALO = C('#48dcf4', 90)
# Horizontal offset of the crack per row (fixed, so the crack is stable between frames).
RIFT_PATH = [0, 0, 1, 1, 0, -1, -1, 0, 0, 1, 2, 1, 0, 0, -1, -1,
             -1, 0, 1, 1, 0, 0, -1, -2, -1, 0, 0, 1, 1, 0, 0, 0]
# Per frame: (row band displaced sideways, shift, shard pixels (x, y), core thinning).
RIFT_FRAMES = [
    ((9, 12), 2, [(3, 6), (12, 9), (2, 20), (13, 24), (11, 3)], False),
    ((19, 21), -2, [(4, 11), (12, 16), (3, 27), (12, 2), (2, 15)], False),
    ((25, 27), 1, [(12, 7), (3, 13), (13, 21), (2, 4), (11, 28)], True),
]


def draw_rift(cv, ox, oy, band, shift, shards, thin):
    """16x32 rift frame: white core, ice/cyan edges, a translucent halo and loose pixel shards."""
    for y in range(1, 32):
        # Lens-shaped: widest in the middle, a single pixel at the tips.
        hw = 0.5 + 1.5 * math.sin(math.pi * y / 32.0)
        cx = 7.5 + RIFT_PATH[y] + (shift if band[0] <= y <= band[1] else 0)
        for x in range(16):
            d = abs(x + 0.5 - cx)
            if d <= (0.5 if thin else 0.8) and hw > 0.9:
                col = RIFT_CORE
            elif d <= hw * 0.6:
                col = RIFT_ICE
            elif d <= hw * 0.9:
                col = RIFT_CYAN
            elif d <= hw + 0.5:
                col = RIFT_DEEP
            elif d <= hw + 1.5:
                col = RIFT_HALO
            else:
                continue
            cv.set(ox + x, oy + y, col)
    for i, (x, y) in enumerate(shards):
        cv.set(ox + x, oy + y, RIFT_CORE if i % 2 == 0 else RIFT_CYAN)
        if i % 3 == 0:
            cv.set(ox + x, oy + y + 1, RIFT_ICE)


def build_sprites():
    cv = Canvas(256, 128)
    for i, f in enumerate(HERO_FRAMES):
        cv.grid(f, HERO_PAL, i * 16, 0)
        cv.grid(f, HERO_BOOTS_PAL, i * 16, 24)
    cv.grid(ENEMY_WALK1, ENEMY_PAL, 0, 48)
    cv.grid(ENEMY_WALK2, ENEMY_PAL, 16, 48)
    cv.grid(ENEMY_SQUASH, ENEMY_PAL, 32, 48)
    for i, sq in enumerate([1.0, 0.6, 0.2, 0.6]):
        draw_coin(cv, 48 + i * 16, 48, sq)
    cv.grid(BOOTS_PICKUP, BOOTS_PAL, 112, 48)
    draw_goal(cv, 0, 64)
    draw_checkpoint(cv, 16, 64, False, False)
    draw_checkpoint(cv, 32, 64, True, False)
    draw_checkpoint(cv, 48, 64, True, True)
    draw_particles(cv, 64, 64)
    for i, fr in enumerate(RIFT_FRAMES):
        draw_rift(cv, 16 + i * 16, 96, *fr)
    return cv


# --------------------------------------------------------------------------
# Tileset (16x16, 8 columns)
# --------------------------------------------------------------------------

GRASS_HI = C('#9ce858')
GRASS = C('#4cba34')
GRASS_DK = C('#2a7a28')
DIRT = C('#c47a3a')
DIRT_DK = C('#8a5028')
DIRT_HI = C('#e0a060')
DIRT_EDGE = C('#5a3018')


def tile_ground(cv, ox, oy, top, left, right, seed):
    rnd = random.Random(seed)
    for y in range(16):
        for x in range(16):
            cv.set(ox + x, oy + y, DIRT)
    # Speckles (pebbles) in a deterministic but organic pattern.
    for _ in range(9):
        x = rnd.randrange(1, 15)
        y = rnd.randrange(1, 15)
        cv.set(ox + x, oy + y, DIRT_DK)
        cv.set(ox + x + 1, oy + y, DIRT_DK)
        cv.set(ox + x, oy + y - 1, DIRT_HI)
    if left:
        for y in range(16):
            cv.set(ox, oy + y, DIRT_EDGE)
            cv.set(ox + 1, oy + y, DIRT_DK)
    if right:
        for y in range(16):
            cv.set(ox + 15, oy + y, DIRT_EDGE)
            cv.set(ox + 14, oy + y, DIRT_DK)
    if top:
        drips = [3, 4, 2, 5, 3, 4, 6, 3, 4, 2, 5, 4, 3, 5, 4, 3]
        for x in range(16):
            cv.set(ox + x, oy, GRASS_DK)
            cv.set(ox + x, oy + 1, GRASS_HI)
            cv.set(ox + x, oy + 2, GRASS_HI if x % 5 == 0 else GRASS)
            for y in range(3, drips[x] + 1):
                cv.set(ox + x, oy + y, GRASS)
            cv.set(ox + x, oy + drips[x] + 1, GRASS_DK)
        if left:
            for y in range(0, 6):
                cv.set(ox, oy + y, GRASS_DK)
        if right:
            for y in range(0, 6):
                cv.set(ox + 15, oy + y, GRASS_DK)


def tile_brick(cv, ox, oy):
    base, dark, hi = C('#d0603a'), C('#7a2a20'), C('#f09060')
    cv.rect(ox, oy, 16, 16, base)
    for y in range(16):
        row = y // 4
        if y % 4 == 3:
            for x in range(16):
                cv.set(ox + x, oy + y, dark)
        else:
            off = 0 if row % 2 == 0 else 4
            for x in (off + 7, (off + 15) % 16):
                cv.set(ox + x, oy + y, dark)
            if y % 4 == 0:
                for x in range(16):
                    if cv.get(ox + x, oy + y) == base:
                        cv.set(ox + x, oy + y, hi)


def tile_stone(cv, ox, oy):
    base, dark, hi = C('#9098b0'), C('#4a5068'), C('#d0d8e8')
    cv.rect(ox, oy, 16, 16, base)
    for i in range(16):
        cv.set(ox + i, oy, hi)
        cv.set(ox, oy + i, hi)
        cv.set(ox + i, oy + 15, dark)
        cv.set(ox + 15, oy + i, dark)
    cv.rect(ox + 4, oy + 4, 8, 8, C('#a8b0c8'))
    for (x, y) in [(2, 2), (13, 2), (2, 13), (13, 13)]:
        cv.set(ox + x, oy + y, dark)


def tile_plank(cv, ox, oy, part):
    base, dark, hi = C('#c89048'), C('#6a4020'), C('#f0c070')
    for y in range(0, 7):
        for x in range(16):
            col = base
            if y == 0:
                col = hi
            elif y == 6:
                col = dark
            cv.set(ox + x, oy + y, col)
    # Rope/bolt details and end caps.
    if part == 'left':
        for y in range(7):
            cv.set(ox, oy + y, dark)
    if part == 'right':
        for y in range(7):
            cv.set(ox + 15, oy + y, dark)
    for x in (3, 12):
        cv.set(ox + x, oy + 3, dark)
    # Support strut below.
    if part != 'mid':
        sx = 3 if part == 'left' else 11
        for y in range(7, 12):
            cv.set(ox + sx, oy + y, dark)
            cv.set(ox + sx + 1, oy + y, base)


def tile_gem_block(cv, ox, oy):
    base, dark, hi = C('#f0b030'), C('#8a5010'), C('#fff0a0')
    cv.rect(ox, oy, 16, 16, base)
    for i in range(16):
        cv.set(ox + i, oy, OUTLINE)
        cv.set(ox + i, oy + 15, OUTLINE)
        cv.set(ox, oy + i, OUTLINE)
        cv.set(ox + 15, oy + i, OUTLINE)
        cv.set(ox + i, oy + 14, dark) if 0 < i < 15 else None
        cv.set(ox + 1, oy + i, hi) if 0 < i < 15 else None
    gem = [
        "..KK..",
        ".KcCK.",
        "KccCCK",
        "KcCCWK",
        ".KCCK.",
        "..KK..",
    ]
    cv.grid(gem, {'K': OUTLINE, 'C': C('#48d0f0'), 'c': C('#2080c0'), 'W': C('#ffffff')}, ox + 5, oy + 5)


def tile_bush(cv, ox, oy, side):
    hi, base, dark = C('#78d860'), C('#3aa040'), C('#1e6a30')
    cx = 15.5 if side == 'left' else -0.5
    for y in range(16):
        for x in range(16):
            dx = x + 0.5 - cx
            dy = y + 0.5 - 16
            d = math.hypot(dx / 1.0, dy / 1.15)
            bump = 2.0 * math.sin((x + (0 if side == 'left' else 16)) * 0.9)
            if d < 11.5 + bump:
                col = base
                if d > 10.2 + bump:
                    col = dark
                elif dy < -9 and (x + y) % 3 == 0:
                    col = hi
                cv.set(ox + x, oy + y, col)


def tile_flower(cv, ox, oy):
    stem = C('#2a8a38')
    for y in range(9, 16):
        cv.set(ox + 7, oy + y, stem)
    cv.set(ox + 6, oy + 12, stem)
    cv.set(ox + 5, oy + 11, stem)
    petals = [
        ".KKK.",
        "KPPPK",
        "KPYPK",
        "KPPPK",
        ".KKK.",
    ]
    cv.grid(petals, {'K': C('#a02050'), 'P': C('#f870a0'), 'Y': C('#ffe040')}, ox + 5, oy + 5)


def tile_tuft(cv, ox, oy):
    for x, h in [(3, 3), (4, 5), (5, 3), (9, 4), (10, 6), (11, 4), (12, 2)]:
        for y in range(16 - h, 16):
            cv.set(ox + x, oy + y, GRASS if y > 16 - h else GRASS_HI)


def tile_fence(cv, ox, oy):
    wood, dark = C('#e8d0a0'), C('#8a6a40')
    for px in (2, 11):
        for y in range(4, 16):
            cv.set(ox + px, oy + y, dark)
            cv.set(ox + px + 1, oy + y, wood)
            cv.set(ox + px + 2, oy + y, dark)
        cv.set(ox + px + 1, oy + 3, dark)
    for ry in (7, 12):
        for x in range(16):
            if cv.get(ox + x, oy + ry)[3] == 0:
                cv.set(ox + x, oy + ry, wood)
                cv.set(ox + x, oy + ry + 1, dark)


def tile_sign(cv, ox, oy):
    wood, dark = C('#d8a868'), C('#6a4020')
    for y in range(9, 16):
        cv.set(ox + 7, oy + y, dark)
        cv.set(ox + 8, oy + y, wood)
    board = [
        "KKKKKKKKKKK...",
        "KwwwwwwwwwwK..",
        "KwwwwwKwwwwwK.",
        "KwKKKKKKwwwwwK",
        "KwwwwwKwwwwwK.",
        "KwwwwwwwwwwK..",
        "KKKKKKKKKKK...",
    ]
    cv.grid(board, {'K': dark, 'w': wood}, ox + 1, oy + 2)


def tile_rock(cv, ox, oy):
    base, dark, hi = C('#a0a0b0'), C('#585868'), C('#d8d8e0')
    for y in range(16):
        for x in range(16):
            d = math.hypot((x + 0.5 - 8) / 6.0, (y + 0.5 - 16) / 6.0)
            if d < 1.0:
                col = base
                if d > 0.85:
                    col = dark
                elif x < 7 and y < 13:
                    col = hi
                cv.set(ox + x, oy + y, col)


def tile_cave(cv, ox, oy):
    base, dark = C('#3a2438'), C('#2a1828')
    cv.rect(ox, oy, 16, 16, base)
    for (x, y) in [(3, 4), (10, 2), (12, 11), (5, 12), (8, 7)]:
        cv.set(ox + x, oy + y, dark)
        cv.set(ox + x + 1, oy + y, dark)


def tile_collision_marker(cv, ox, oy):
    red = C('#ff3050', 140)
    for i in range(16):
        cv.set(ox + i, oy, red)
        cv.set(ox + i, oy + 15, red)
        cv.set(ox, oy + i, red)
        cv.set(ox + 15, oy + i, red)
        cv.set(ox + i, oy + i, red)
        cv.set(ox + 15 - i, oy + i, red)


# Order defines gids (index + 1). Mirrored in tools/gen_level.py (TILE dict).
TILE_BUILDERS = [
    lambda cv, x, y: tile_ground(cv, x, y, True, True, False, 1),    # 1 ground top-left
    lambda cv, x, y: tile_ground(cv, x, y, True, False, False, 2),   # 2 ground top
    lambda cv, x, y: tile_ground(cv, x, y, True, False, True, 3),    # 3 ground top-right
    lambda cv, x, y: tile_ground(cv, x, y, False, True, False, 4),   # 4 ground left
    lambda cv, x, y: tile_ground(cv, x, y, False, False, False, 5),  # 5 ground fill
    lambda cv, x, y: tile_ground(cv, x, y, False, False, True, 6),   # 6 ground right
    tile_brick,                                                      # 7 brick
    tile_stone,                                                      # 8 stone
    lambda cv, x, y: tile_plank(cv, x, y, 'left'),                   # 9 plank left
    lambda cv, x, y: tile_plank(cv, x, y, 'mid'),                    # 10 plank mid
    lambda cv, x, y: tile_plank(cv, x, y, 'right'),                  # 11 plank right
    tile_gem_block,                                                  # 12 gem block
    lambda cv, x, y: tile_bush(cv, x, y, 'left'),                    # 13 bush left
    lambda cv, x, y: tile_bush(cv, x, y, 'right'),                   # 14 bush right
    tile_flower,                                                     # 15 flower
    tile_tuft,                                                       # 16 grass tuft
    tile_fence,                                                      # 17 fence
    tile_sign,                                                       # 18 arrow sign
    tile_rock,                                                       # 19 rock
    tile_cave,                                                       # 20 cave wall (background)
    lambda cv, x, y: tile_ground(cv, x, y, True, True, True, 21),    # 21 ground top pillar
    lambda cv, x, y: tile_ground(cv, x, y, False, True, True, 22),   # 22 ground pillar
    lambda cv, x, y: draw_coin(cv, x, y, 1.0),                        # 23 coin marker (coins layer, editor only)
    tile_collision_marker,                                           # 24 collision marker (collision layer, editor only)
]


def build_tiles():
    cv = Canvas(128, 48)
    for i, fn in enumerate(TILE_BUILDERS):
        fn(cv, (i % 8) * 16, (i // 8) * 16)
    return cv


# --------------------------------------------------------------------------
# Font (8x8 cells, glyphs 5x7, ASCII 32..127, 16 columns)
# --------------------------------------------------------------------------

GLYPHS = {
    'A': [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    'B': ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
    'C': [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
    'D': ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
    'E': ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
    'F': ["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
    'G': [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####"],
    'H': ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    'I': [".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###."],
    'J': ["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."],
    'K': ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
    'L': ["#....", "#....", "#....", "#....", "#....", "#....", "#####"],
    'M': ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
    'N': ["#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#"],
    'O': [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    'P': ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
    'Q': [".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"],
    'R': ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
    'S': [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
    'T': ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
    'U': ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    'V': ["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."],
    'W': ["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "##.##", "#...#"],
    'X': ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"],
    'Y': ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
    'Z': ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
    '0': [".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."],
    '1': ["..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."],
    '2': [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"],
    '3': ["#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###."],
    '4': ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."],
    '5': ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
    '6': ["..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."],
    '7': ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."],
    '8': [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."],
    '9': [".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."],
    ':': [".....", "..#..", "..#..", ".....", "..#..", "..#..", "....."],
    '!': ["..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#.."],
    '.': [".....", ".....", ".....", ".....", ".....", ".##..", ".##.."],
    ',': [".....", ".....", ".....", ".....", ".##..", "..#..", ".#..."],
    '-': [".....", ".....", ".....", "#####", ".....", ".....", "....."],
    '/': ["....#", "...#.", "...#.", "..#..", ".#...", ".#...", "#...."],
    "'": ["..#..", "..#..", ".#...", ".....", ".....", ".....", "....."],
    '?': [".###.", "#...#", "....#", "...#.", "..#..", ".....", "..#.."],
    '(': ["...#.", "..#..", ".#...", ".#...", ".#...", "..#..", "...#."],
    ')': [".#...", "..#..", "...#.", "...#.", "...#.", "..#..", ".#..."],
    '+': [".....", "..#..", "..#..", "#####", "..#..", "..#..", "....."],
    '=': [".....", ".....", "#####", ".....", "#####", ".....", "....."],
    '<': ["...#.", "..#..", ".#...", "#....", ".#...", "..#..", "...#."],
    '>': [".#...", "..#..", "...#.", "....#", "...#.", "..#..", ".#..."],
    '*': [".....", "#.#.#", ".###.", "#####", ".###.", "#.#.#", "....."],
    'x': [".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#"],
}

ICON_PAL = {
    'K': OUTLINE, 'G': GOLD, 'g': GOLD_DARK, 'W': C('#fffbe0'),
    'O': C('#ff6a3a'), 'w': C('#f4f4f4'),
}
ICONS = {
    '$': [  # coin
        "..KKKK..",
        ".KGGGGK.",
        "KGWGGGgK",
        "KGWGGGgK",
        "KGWGGGgK",
        "KGGGGGgK",
        ".KggggK.",
        "..KKKK..",
    ],
    '^': [  # dash boots
        "..KKK...",
        "..KOOK..",
        "..KOOK..",
        ".KOOOKK.",
        ".KOOOOOK",
        ".KOwwOOK",
        ".KKKKKKK",
        "........",
    ],
    '#': [  # skull (deaths)
        ".KKKKKK.",
        "KwwwwwwK",
        "KwKwwKwK",
        "KwKwwKwK",
        "KwwwwwwK",
        ".KwKKwK.",
        "..KwwK..",
        "...KK...",
    ],
}


def build_font():
    cv = Canvas(128, 48)
    white = C('#ffffff')
    for code in range(32, 128):
        ch = chr(code)
        cell = code - 32
        ox, oy = (cell % 16) * 8, (cell // 16) * 8
        if code == 127:
            cv.rect(ox, oy, 8, 8, white)
            continue
        if ch in ICONS:
            cv.grid(ICONS[ch], ICON_PAL, ox, oy)
            continue
        g = GLYPHS.get(ch) or GLYPHS.get(ch.upper())
        if not g:
            continue
        for y, row in enumerate(g):
            for x, v in enumerate(row):
                if v == '#':
                    cv.set(ox + x + 1, oy + y, white)
    return cv


# --------------------------------------------------------------------------
# Parallax backgrounds (320x180, tileable horizontally)
# --------------------------------------------------------------------------

BAYER = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(4))


def periodic(x, terms, period=320):
    return sum(a * math.sin(2 * math.pi * k * x / period + ph) for (k, a, ph) in terms)


# --------------------------------------------------------------------------
# Themes: exact color remaps that build level-2-4 variants of the meadow art.
# Keys are meadow source colors; anything unlisted stays as-is.
# --------------------------------------------------------------------------

def recolor(cv, mapping):
    """Swap every pixel exactly equal to a mapped source color."""
    table = {C(k): C(v) for k, v in mapping.items()}
    for y in range(cv.h):
        row = cv.px[y]
        for x in range(cv.w):
            c = row[x]
            if c in table:
                row[x] = table[c]


MEADOW_SKY = [C('#3a6ad8'), C('#5aa0f0'), C('#8cd0f8'), C('#d0f0ff')]

# bg_far: the future 3D pass samples the image from row 76 down, so the LAST
# sky stop (which fills rows >= span) must keep the keying (g > r + 31) and
# every mountain remap must stay a mountain (g - r <= 24). Upper stops and
# clouds only render in the 2D parallax view — any color works there.
FAR_THEMES = {
    'sunset': {
        'sky': [C('#2a1656'), C('#6a2a7e'), C('#d85a48'), C('#2e8898')],
        'span': 72.0,
        'remap': {
            '#ffffff': '#fff2d8', '#d8e8f8': '#f0b0a0',   # clouds: rose gold
            '#a8b8e8': '#c87890', '#8898d0': '#a05878',   # far range: mauve
            '#f0f4ff': '#ffd8c0', '#d0dcf4': '#e8b098',   # sun-tinted caps
            '#7890d0': '#6a3070', '#6478b8': '#502458',   # near range: dusk
        },
    },
    'cave': {
        'sky': [C('#06060e'), C('#0c1020'), C('#141a30'), C('#0e3848')],
        'span': 72.0,
        'remap': {
            '#ffffff': '#28304a', '#d8e8f8': '#1c2238',   # mist
            '#a8b8e8': '#4a5068', '#8898d0': '#363c52',   # far rock
            '#f0f4ff': '#c0cee0', '#d0dcf4': '#9cb0cc',   # pale crystal caps
            '#7890d0': '#252a40', '#6478b8': '#1a1e30',   # near rock
        },
    },
    'night': {
        'sky': [C('#080c1e'), C('#0e1630'), C('#16264a'), C('#0e4058')],
        'span': 72.0,
        'remap': {
            '#ffffff': '#c8d4f0', '#d8e8f8': '#98a8d0',   # faint moon cloud
            '#a8b8e8': '#6a78b0', '#8898d0': '#546098',   # moonlit slate
            '#f0f4ff': '#c8d4ee', '#d0dcf4': '#a8b4d0',
            '#7890d0': '#3a4480', '#6478b8': '#2a3068',
        },
    },
}

NEAR_THEMES = {
    'sunset': {  # golden dry hills, olive trees
        '#a0e878': '#f0d878', '#58c060': '#c09c48', '#3a9a50': '#947030', '#2a7a48': '#664c1e',
        '#2e8a40': '#7c5c22', '#48a850': '#98742c', '#6a4020': '#503018',
    },
    'cave': {
        '#a0e878': '#4a5e8c', '#58c060': '#2e3c60', '#3a9a50': '#1f2c48', '#2a7a48': '#131c30',
        '#2e8a40': '#234a66', '#48a850': '#396a84', '#6a4020': '#101c2c',
    },
    'night': {
        '#a0e878': '#5878a8', '#58c060': '#2e4a74', '#3a9a50': '#1e3458', '#2a7a48': '#122238',
        '#2e8a40': '#18324e', '#48a850': '#2a446c', '#6a4020': '#0c1626',
    },
}

TILE_THEMES = {
    'sunset': {  # golden dry grass, warm ground
        '#9ce858': '#f4d862', '#4cba34': '#d4a832', '#2a7a28': '#a0761e',
        '#78d860': '#e8c060', '#3aa040': '#b8862e', '#1e6a30': '#7c5c1c',
        '#2a8a38': '#9a7424', '#f870a0': '#ff9440', '#a02050': '#c44c14',
        '#3a2438': '#5c2030', '#2a1828': '#3a1420',
    },
    'cave': {  # teal moss on dark stone, crystal bushes, glow toadstools
        '#9ce858': '#5ae0c0', '#4cba34': '#2a9a80', '#2a7a28': '#1a6a56',
        '#c47a3a': '#5c5674', '#8a5028': '#3c3652', '#e0a060': '#7e7694', '#5a3018': '#221e34',
        '#9098b0': '#707692', '#4a5068': '#424a66', '#d0d8e8': '#9aa2ba', '#a8b0c8': '#7e86a0',
        '#78d860': '#50d8d0', '#3aa040': '#30a0a0', '#1e6a30': '#1e6a6e',
        '#2a8a38': '#38688c', '#f870a0': '#7ce8ff', '#a02050': '#2c7ac8', '#ffe040': '#eaffff',
        '#3a2438': '#46295a', '#2a1828': '#301c44',
        '#a0a0b0': '#767696', '#585868': '#444460', '#d8d8e0': '#a0a0c4',
        '#f0b030': '#b878f0', '#8a5010': '#6a30a0', '#fff0a0': '#e8bcff',
    },
    'night': {  # moonlit frost grass, cold dirt, violet brick
        '#9ce858': '#8fd8c0', '#4cba34': '#3f9a80', '#2a7a28': '#266a58',
        '#c47a3a': '#68688e', '#8a5028': '#42426a', '#e0a060': '#8e8eb2', '#5a3018': '#262642',
        '#d0603a': '#8a6ab8', '#7a2a20': '#4c3a70', '#f09060': '#b2a0d8',
        '#78d860': '#74c8d4', '#3aa040': '#3f8f96', '#1e6a30': '#2c606a',
        '#2a8a38': '#33627a', '#f870a0': '#d8e8ff', '#a02050': '#6a7ac0', '#ffe040': '#ffffff',
        '#3a2438': '#22284a', '#2a1828': '#141832',
    },
}


def build_bg_far(sky=MEADOW_SKY, span=130.0):
    w, h = 320, 180
    cv = Canvas(w, h)
    stops = sky
    for y in range(h):
        t = min(1.0, y / span) * (len(stops) - 1)
        i = min(int(t), len(stops) - 2)
        f = t - i
        for x in range(w):
            # Ordered dithering between the two nearest stops: banded SNES look.
            col = stops[i + 1] if f * 16 > BAYER[y % 4][x % 4] + 0.5 else stops[i]
            cv.px[y][x] = col
    # Clouds.
    rnd = random.Random(7)
    for _ in range(7):
        cx = rnd.randrange(0, w)
        cy = rnd.randrange(12, 70)
        puffs = [(rnd.uniform(-14, 14), rnd.uniform(-3, 3), rnd.uniform(5, 9)) for _ in range(5)]
        for y in range(cy - 14, cy + 12):
            for xr in range(cx - 26, cx + 26):
                inside = False
                shade = False
                for (px, py, r) in puffs:
                    d = math.hypot(xr - (cx + px), (y - (cy + py)) * 1.4)
                    if d < r:
                        inside = True
                        if y > cy + py + r * 0.3:
                            shade = True
                if inside and y < cy + 5:
                    cv.set(xr % w, y, C('#d8e8f8') if shade else C('#ffffff'))
    # Two mountain ranges made of wrapped triangular peaks (snow caps on the far range).
    rnd = random.Random(11)
    for (count, hmin, hmax, base, col, shade, snow) in [
        (6, 50, 78, 150, C('#a8b8e8'), C('#8898d0'), True),
        (8, 26, 46, 150, C('#7890d0'), C('#6478b8'), False),
    ]:
        peaks = [(rnd.uniform(0, w), rnd.uniform(hmin, hmax), rnd.uniform(0.7, 1.1)) for _ in range(count)]
        for x in range(w):
            best_top, best_peak = h, None
            for (px, ph, slope) in peaks:
                dx = min(abs(x - px), w - abs(x - px))
                top = base - ph + dx * slope
                if top < best_top:
                    best_top, best_peak = top, (px, ph, slope)
            top = int(best_top)
            px = best_peak[0]
            right_side = ((x - px) % w) < w / 2
            for y in range(max(top, 0), h):
                c = shade if right_side else col
                if snow and y < top + 6 and best_peak[1] > 60 and y < base - best_peak[1] + 14:
                    c = C('#f0f4ff') if not right_side else C('#d0dcf4')
                cv.px[y][x] = c
    return cv


def build_bg_near():
    w, h = 320, 180
    cv = Canvas(w, h)
    rim, base, dark, darker = C('#a0e878'), C('#58c060'), C('#3a9a50'), C('#2a7a48')
    for x in range(w):
        top = int(132 + periodic(x, [(2, 10, 0.0), (5, 5, 1.3), (3, 3, 2.2)]))
        for y in range(top, h):
            d = y - top
            if d < 2:
                col = rim
            elif d < 18:
                col = base if BAYER[y % 4][x % 4] > (d - 2) else dark
            else:
                col = dark if BAYER[y % 4][x % 4] > (d - 18) // 2 else darker
            cv.px[y][x] = col
    # Little round trees on the hills.
    rnd = random.Random(3)
    for _ in range(9):
        tx = rnd.randrange(0, w)
        top = int(132 + periodic(tx, [(2, 10, 0.0), (5, 5, 1.3), (3, 3, 2.2)]))
        r = rnd.randrange(5, 9)
        for y in range(top - 2 * r - 4, top + 2):
            for xx in range(tx - r - 1, tx + r + 2):
                d = math.hypot(xx - tx, y - (top - r - 4))
                if d <= r:
                    col = C('#2e8a40') if (xx - tx) + (y - (top - r - 4)) > 2 else C('#48a850')
                    cv.set(xx % w, y, col)
        for y in range(top - 4, top + 2):
            cv.set(tx % w, y, C('#6a4020'))
    return cv


def assert_band_safe(stops, remap):
    """The future 3D pass keys bg_far's band (row 76 down): the flat bottom
    sky must key out (g > r + 31) and every mountain remap must stay alpha."""
    c = stops[-1]
    assert c[1] - c[0] >= 31, f'sky bottom {c} fails the future keying'
    for v in remap.values():
        c = C(v)
        assert c[1] - c[0] <= 24, f'mountain/cloud color {v} fails the keying'


def main():
    prev_dir = None
    if '--preview' in sys.argv:
        prev_dir = sys.argv[sys.argv.index('--preview') + 1]
        os.makedirs(prev_dir, exist_ok=True)
    os.makedirs(OUT, exist_ok=True)
    assert_band_safe(MEADOW_SKY, {})
    outputs = {
        'sprites.png': build_sprites(),
        'tiles.png': build_tiles(),
        'font.png': build_font(),
        'bg_far.png': build_bg_far(),
        'bg_near.png': build_bg_near(),
    }
    # Campaign themes: fresh builds, then exact remaps to the themed palette.
    for theme, cfg in FAR_THEMES.items():
        assert_band_safe(cfg['sky'], cfg['remap'])
        far = build_bg_far(cfg['sky'], cfg['span'])
        recolor(far, cfg['remap'])
        outputs[f'bg_far_{theme}.png'] = far
    for theme, mapping in NEAR_THEMES.items():
        near = build_bg_near()
        recolor(near, mapping)
        outputs[f'bg_near_{theme}.png'] = near
    for theme, mapping in TILE_THEMES.items():
        tiles = build_tiles()
        recolor(tiles, mapping)
        outputs[f'tiles_{theme}.png'] = tiles
    icon = Canvas(32, 32)
    for y, row in enumerate(BOOTS_PICKUP):
        for x, ch in enumerate(row):
            if ch != '.':
                icon.rect(x * 2, y * 2, 2, 2, BOOTS_PAL[ch])
    icon.save(os.path.join(ROOT, 'public', 'favicon.png'))
    for name, cv in outputs.items():
        cv.save(os.path.join(OUT, name))
        print('wrote', os.path.join(OUT, name))
        if prev_dir:
            scale = 2 if cv.w >= 320 else 4
            preview(cv, os.path.join(prev_dir, name), scale)


if __name__ == '__main__':
    main()
