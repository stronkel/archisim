#!/usr/bin/env python3
"""Writes the DXF test files of `archisim-dxf` (synthetic surveys, not real
projects), with ezdxf as another program would write them.

    pip install ezdxf
    python3 scripts/make_dxf_fixtures.py

- levantamiento_r12.dxf (AutoCAD R12 format): survey points as POINT with
  their height, contour lines as 2D POLYLINE with elevation, a breakline
  as 3D POLYLINE, height labels as TEXT and the outline of a building at
  height 0 on its own layer (to leave out).
- levantamiento_2018.dxf (R2018): contour lines as LWPOLYLINE with
  elevation, survey points as INSERT of a block with a COTA attribute (the
  insertion at height 0) and a small TIN of 3DFACE.

Coordinates are UTM-like (hundreds of kilometres), heights absolute (m).
The ground is a hill: z = 645 - 0.004·r² about (431250, 4581300).
"""
import math
import os

import ezdxf

X0, Y0 = 431250.0, 4581300.0
OUT = os.path.join(os.path.dirname(__file__), "..", "crates", "archisim-dxf", "tests", "fixtures")


def z(x, y):
    return 645.0 - 0.004 * ((x - X0) ** 2 + (y - Y0) ** 2)


def contour(level, n=48):
    r = math.sqrt((645.0 - level) / 0.004)
    return [(X0 + r * math.cos(2 * math.pi * i / n), Y0 + r * math.sin(2 * math.pi * i / n)) for i in range(n)]


def r12():
    doc = ezdxf.new("R12")
    msp = doc.modelspace()
    for name in ("PUNTOS", "CURVAS", "RUPTURA", "COTAS", "EDIFICACION"):
        doc.layers.add(name)
    for i in range(-3, 4):
        for j in range(-3, 4):
            x, y = X0 + 10 * i, Y0 + 10 * j
            msp.add_point((x, y, round(z(x, y), 3)), dxfattribs={"layer": "PUNTOS"})
            msp.add_text(f"{z(x, y):.2f}", dxfattribs={"layer": "COTAS", "insert": (x + 0.5, y + 0.5), "height": 0.5})
    for level in (641.0, 642.0, 643.0, 644.0):
        pl = msp.add_polyline2d(contour(level), close=True, dxfattribs={"layer": "CURVAS"})
        pl.dxf.elevation = (0, 0, level)
    brk = [(X0 - 30 + 6 * k, Y0 - 25) for k in range(11)]
    msp.add_polyline3d([(x, y, round(z(x, y), 3)) for x, y in brk], dxfattribs={"layer": "RUPTURA"})
    house = [(X0 + 2, Y0 + 2), (X0 + 14, Y0 + 2), (X0 + 14, Y0 + 11), (X0 + 2, Y0 + 11)]
    for a, b in zip(house, house[1:] + house[:1]):
        msp.add_line((a[0], a[1], 0), (b[0], b[1], 0), dxfattribs={"layer": "EDIFICACION"})
    doc.saveas(os.path.join(OUT, "levantamiento_r12.dxf"))


def r2018():
    doc = ezdxf.new("R2018")
    msp = doc.modelspace()
    for name in ("CURVAS NIVEL", "PUNTOS TOPO", "TIN"):
        doc.layers.add(name)
    block = doc.blocks.new(name="PUNTO")
    block.add_circle((0, 0), 0.2)
    block.add_attdef("COTA", (0.3, 0.3), dxfattribs={"height": 0.4})
    for level in (641.5, 642.5, 643.5):
        msp.add_lwpolyline(contour(level, 36), close=True,
                           dxfattribs={"layer": "CURVAS NIVEL", "elevation": level})
    for k in range(12):
        a = 2 * math.pi * k / 12
        x, y = X0 + 20 * math.cos(a), Y0 + 20 * math.sin(a)
        ins = msp.add_blockref("PUNTO", (x, y, 0), dxfattribs={"layer": "PUNTOS TOPO"})
        ins.add_auto_attribs({"COTA": f"{z(x, y):.3f}"})
    tin = [(X0 - 5, Y0 - 5), (X0 + 5, Y0 - 5), (X0 + 5, Y0 + 5), (X0 - 5, Y0 + 5)]
    pts = [(x, y, round(z(x, y), 3)) for x, y in tin]
    msp.add_3dface([pts[0], pts[1], pts[2]], dxfattribs={"layer": "TIN"})
    msp.add_3dface([pts[0], pts[2], pts[3]], dxfattribs={"layer": "TIN"})
    doc.saveas(os.path.join(OUT, "levantamiento_2018.dxf"))


if __name__ == "__main__":
    r12()
    r2018()
    print("Escritos:", sorted(os.listdir(OUT)))
