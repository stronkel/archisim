"""Example drawings for elevations (C1, 24/10/2026), drawn here from
scratch for ArchiSim: a tree in lines (DXF) and a person as a cut-out
picture (PNG with transparency). They show how an office's own drawings
are used; they are not anyone's library.

    python3 scripts/make_elevation_examples.py
"""
import math
import ezdxf
from PIL import Image, ImageDraw

OUT = "docs/ejemplos"

# A tree in elevation, as drawn by hand: a trunk that forks and a crown
# of overlapping rounded lobes; 8 m high, in metres.
doc = ezdxf.new(dxfversion="R2010")
doc.header["$INSUNITS"] = 6
msp = doc.modelspace()
doc.layers.add("TRONCO", color=1)
doc.layers.add("COPA", color=3)
trunk = [(-0.18, 0), (-0.12, 1.6), (-0.6, 3.1), (-0.45, 3.2), (0.0, 2.1),
         (0.35, 3.3), (0.5, 3.2), (0.12, 1.6), (0.18, 0)]
msp.add_lwpolyline(trunk, close=True, dxfattribs={"layer": "TRONCO"})
lobes = [(-1.9, 4.4, 1.3), (-0.8, 5.6, 1.6), (0.7, 5.9, 1.7), (1.9, 4.6, 1.3),
         (0.0, 4.0, 1.5), (-1.1, 6.6, 1.2), (0.9, 6.7, 1.25)]
for (cx, cy, r) in lobes:
    msp.add_circle((cx, cy), r, dxfattribs={"layer": "COPA"})
doc.saveas(f"{OUT}/arbol-alzado.dxf")

# A person seen from the front, a soft silhouette in two tones, 350 px
# high (any size: it is drawn as tall as the figure).
W, H = 140, 350
img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
coat = (58, 92, 104, 255)
trousers = (196, 178, 150, 255)
skin = (222, 186, 152, 255)
hair = (70, 52, 40, 255)
d.rounded_rectangle((44, 175, 66, 340), radius=10, fill=trousers)
d.rounded_rectangle((74, 175, 96, 340), radius=10, fill=trousers)
d.rounded_rectangle((36, 70, 104, 200), radius=26, fill=coat)
d.rounded_rectangle((22, 78, 40, 180), radius=9, fill=coat)
d.rounded_rectangle((100, 78, 118, 180), radius=9, fill=coat)
d.ellipse((24, 172, 40, 192), fill=skin)
d.ellipse((100, 172, 116, 192), fill=skin)
d.ellipse((50, 14, 90, 62), fill=skin)
d.chord((48, 8, 92, 48), 180, 360, fill=hair)
d.rounded_rectangle((40, 334, 68, 348), radius=6, fill=hair)
d.rounded_rectangle((72, 334, 100, 348), radius=6, fill=hair)
img.save(f"{OUT}/persona-alzado.png")
# The same tree seen from above, for presentation plans: washes of green
# in lobes, a darker side and the trunk.
S = 400
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
for (cx, cy, r, a) in [(200, 200, 180, 150), (150, 170, 120, 120), (250, 160, 110, 120),
                       (230, 250, 120, 140), (160, 240, 110, 120)]:
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(layer).ellipse((cx - r, cy - r, cx + r, cy + r), fill=(110, 150, 96, a))
    img = Image.alpha_composite(img, layer)
d = ImageDraw.Draw(img)
d.ellipse((188, 188, 212, 212), fill=(96, 74, 56, 255))
img.save(f"{OUT}/arbol-planta.png")
print("docs/ejemplos/arbol-alzado.dxf, persona-alzado.png, arbol-planta.png")
