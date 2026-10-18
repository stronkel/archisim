"""Draws ArchiSim's application icon (an original design): the plan of a
wall corner with its layers — render, brick, insulation, inner leaf — and a
door swing, on a dark blue tile. Writes crates/archisim-app/assets/icon.png
(1024 × 1024), from which the macOS .icns and the window icon are made.

    python3 scripts/make_icon.py        # requires Pillow
"""
from pathlib import Path
from PIL import Image, ImageDraw

S = 4  # supersampling
N = 1024 * S
img = Image.new("RGBA", (N, N), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

def px(v):
    return int(v * S)

# Tile: macOS icon grid, 824 px square with ~185 px corner radius.
m = 100
d.rounded_rectangle([px(m), px(m), px(1024 - m), px(1024 - m)], radius=px(185),
                    fill=(30, 52, 70, 255))

ink = (20, 20, 20, 255)
# Wall corner: an L, exterior to the top-left. Layers from outside in.
x0, y0 = 230, 230          # outer corner
length = 560               # leg length from the corner
len_h = 400                # the top wall is shorter: a door follows it
layers = [                 # (thickness, fill)
    (22, (236, 232, 222, 255)),   # render
    (78, (226, 170, 140, 255)),   # brick
    (40, (250, 238, 180, 255)),   # insulation
    (50, (240, 214, 200, 255)),   # inner leaf
    (14, (250, 250, 250, 255)),   # plaster
]
total = sum(t for t, _ in layers)
off = 0
for t, fill in layers:
    a, b = off, off + t
    # horizontal leg and vertical leg, mitred at the corner
    d.polygon([(px(x0 + a), px(y0 + a)), (px(x0 + len_h), px(y0 + a)),
               (px(x0 + len_h), px(y0 + b)), (px(x0 + b), px(y0 + b))], fill=fill)
    d.polygon([(px(x0 + a), px(y0 + a)), (px(x0 + b), px(y0 + b)),
               (px(x0 + b), px(y0 + length)), (px(x0 + a), px(y0 + length))], fill=fill)
    off = b

# Brick hatch (45°) clipped to the brick layer, and insulation zigzag.
hatch = Image.new("L", (N, N), 0)
hd = ImageDraw.Draw(hatch)
a, b = layers[0][0], layers[0][0] + layers[1][0]
hd.polygon([(px(x0 + a), px(y0 + a)), (px(x0 + len_h), px(y0 + a)),
            (px(x0 + len_h), px(y0 + b)), (px(x0 + b), px(y0 + b)),
            (px(x0 + b), px(y0 + length)), (px(x0 + a), px(y0 + length))], fill=255)
lines = Image.new("RGBA", (N, N), (0, 0, 0, 0))
ld = ImageDraw.Draw(lines)
for k in range(-1024, 2048, 26):
    ld.line([(px(k), px(0)), (px(k + 1024), px(1024))], fill=(150, 80, 55, 255), width=px(4))
img.paste(lines, (0, 0), Image.composite(lines, Image.new("RGBA", (N, N)), hatch).split()[3])

a = layers[0][0] + layers[1][0]
b = a + layers[2][0]
mid = (a + b) / 2
amp = (b - a) / 2 - 6
pts = []
for i, x in enumerate(range(x0 + int(b), x0 + len_h + 1, 20)):
    pts.append((px(x), px(y0 + mid + (amp if i % 2 else -amp))))
d.line(pts, fill=(170, 140, 60, 255), width=px(4))
pts = []
for i, y in enumerate(range(y0 + int(b), y0 + length + 1, 20)):
    pts.append((px(x0 + mid + (amp if i % 2 else -amp)), px(y)))
d.line(pts, fill=(170, 140, 60, 255), width=px(4))

# Cut lines: outer and inner faces, and the ends of the legs.
w = px(10)
d.line([(px(x0), px(y0 + length)), (px(x0), px(y0)), (px(x0 + len_h), px(y0))], fill=ink, width=w, joint="curve")
d.line([(px(x0 + total), px(y0 + length)), (px(x0 + total), px(y0 + total)),
        (px(x0 + len_h), px(y0 + total))], fill=ink, width=w, joint="curve")
d.line([(px(x0 + len_h), px(y0)), (px(x0 + len_h), px(y0 + total))], fill=ink, width=w)
d.line([(px(x0), px(y0 + length)), (px(x0 + total), px(y0 + length))], fill=ink, width=w)

# A door at the end of the top wall: the leaf open at 90° into the room
# and its swing back to the closed position.
r = 210
hx, hy = x0 + len_h, y0 + total           # hinge: inner face, end of the wall
light = (235, 240, 245, 255)
d.rectangle([px(hx - 16), px(hy), px(hx), px(hy + r)], fill=light)
d.arc([px(hx - r), px(hy - r), px(hx + r), px(hy + r)], start=0, end=90,
      fill=light, width=px(5))

out = img.resize((1024, 1024), Image.LANCZOS)
path = Path(__file__).resolve().parent.parent / "crates/archisim-app/assets/icon.png"
out.save(path, optimize=True)
print(path)
