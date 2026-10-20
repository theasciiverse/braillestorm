#!/usr/bin/env python3
"""Draw a headless Braillestorm frame as a PNG, Braille dots as round dots.

    cargo run --release -- --shot 3 --size 100x30 --cells | python3 tools/shot.py out.png
"""
import re
import sys
from PIL import Image, ImageDraw, ImageFont

FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
CW, CH = 10, 20
font = ImageFont.truetype(FONT, 16)
lines = sys.stdin.read().splitlines()
w, h = map(int, lines[0].split())
img = Image.new("RGB", (w * CW, h * CH))
d = ImageDraw.Draw(img)
rgb = lambda s: tuple(int(s[i:i + 2], 16) for i in (0, 2, 4))
# bit of each dot, by column then row, as in U+2800..U+28FF
BITS = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]]
R = 1.6
for line in lines[1:]:
    x, y, fg, bg, sym = line.split(" ", 4)
    x, y, fg, bg = int(x), int(y), rgb(fg), rgb(bg)
    sym = "".join(chr(int(c, 16)) for c in re.findall(r"\{([0-9a-f]+)\}", sym))
    box = (x * CW, y * CH, x * CW + CW - 1, y * CH + CH - 1)
    d.rectangle(box, fill=bg)
    if len(sym) == 1 and 0x2800 <= ord(sym) <= 0x28FF:
        bits = ord(sym) - 0x2800
        for dx in range(2):
            for dy in range(4):
                if bits & BITS[dx][dy]:
                    cx = box[0] + CW * (dx * 2 + 1) / 4
                    cy = box[1] + CH * (dy * 2 + 1) / 8
                    d.ellipse((cx - R, cy - R, cx + R, cy + R), fill=fg)
    elif sym.strip():
        d.text((x * CW, y * CH), sym, font=font, fill=fg)
img.save(sys.argv[1])
