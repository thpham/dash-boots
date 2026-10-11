"""Tiny stdlib-only RGBA canvas + PNG writer used by the asset generators."""

import struct
import zlib


def hex_rgba(h, a=255):
    h = h.lstrip('#')
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


TRANSPARENT = (0, 0, 0, 0)


class Canvas:
    def __init__(self, w, h, fill=TRANSPARENT):
        self.w = w
        self.h = h
        self.px = [[fill for _ in range(w)] for _ in range(h)]

    def set(self, x, y, c):
        if 0 <= x < self.w and 0 <= y < self.h and c is not None:
            if c[3] == 0:
                return
            self.px[y][x] = c

    def get(self, x, y):
        return self.px[y][x]

    def rect(self, x, y, w, h, c):
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                self.set(xx, yy, c)

    def grid(self, rows, palette, ox, oy, flip_x=False):
        """Blit a list of strings; each char is looked up in `palette` ('.' = transparent)."""
        width = len(rows[0])
        for y, row in enumerate(rows):
            assert len(row) == width, f'row {y} has width {len(row)} != {width}: {row!r}'
            for x, ch in enumerate(row):
                if ch == '.':
                    continue
                xx = width - 1 - x if flip_x else x
                self.set(ox + xx, oy + y, palette[ch])

    def save(self, path):
        raw = b''.join(
            b'\x00' + bytes(v for p in row for v in p) for row in self.px
        )

        def chunk(tag, data):
            body = tag + data
            return struct.pack('>I', len(data)) + body + struct.pack('>I', zlib.crc32(body) & 0xFFFFFFFF)

        png = b'\x89PNG\r\n\x1a\n'
        png += chunk(b'IHDR', struct.pack('>IIBBBBB', self.w, self.h, 8, 6, 0, 0, 0))
        png += chunk(b'IDAT', zlib.compress(raw, 9))
        png += chunk(b'IEND', b'')
        with open(path, 'wb') as f:
            f.write(png)


def preview(canvas, path, scale=4, bg=(40, 40, 60, 255)):
    """Write an upscaled copy on an opaque background (for eyeballing art)."""
    out = Canvas(canvas.w * scale, canvas.h * scale, bg)
    for y in range(canvas.h):
        for x in range(canvas.w):
            c = canvas.px[y][x]
            if c[3] == 0:
                continue
            for dy in range(scale):
                for dx in range(scale):
                    out.px[y * scale + dy][x * scale + dx] = c
    out.save(path)
