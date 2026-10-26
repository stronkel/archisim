#!/usr/bin/env python3
"""Turns AD5's tree drawings (a DXF with one block per tree, drawn by the
studio) into the plan styles «AD5 …» of `archisim-flora`.

    pip install ezdxf
    python3 scripts/make_ad5_trees.py "Árboles AD5 Planta.dxf"

Writes `crates/archisim-flora/src/ad5_trees.txt`: each drawing scaled to a
crown of radius 1 about its centre, as lines (thin strokes and outlines),
colour washes with AD5's opacity (holes joined to their outline by a
zero-width slit, so each wash is one polygon) and the patches of texture
(their grass pattern is not copied: ArchiSim draws its own tufts in them).
The trunks are left out: ArchiSim draws the plant's own.

The DXF itself is AD5's and is not kept in the repository.

Format, one record per line, numbers in units of the crown radius:

    symbol <name>
    line open|closed|outline x y x y …
    wash <opacity> x y x y …
    patch x y x y …
"""
import math
import os
import sys

import ezdxf
from ezdxf import bbox
from ezdxf import path as zpath

OUT = os.path.join(
    os.path.dirname(__file__), "..", "crates", "archisim-flora", "src", "ad5_trees.txt"
)

# Block name in AD5's DXF → name of the drawing in ArchiSim.
BLOCKS = [
    ("arbolll", "círculos"),
    ("A$C5AA32782", "racimos"),
    ("02PTRX23", "estrella"),
    ("A$C2C581A5A", "copa"),
]


def flat(p, tol):
    return [(v.x, v.y) for v in p.flattening(tol)]


def chain(segs, eps):
    """Joins line segments that share their ends into polylines."""
    out = []
    segs = [list(s) for s in segs]
    while segs:
        cur = segs.pop(0)
        grown = True
        while grown:
            grown = False
            for i, s in enumerate(segs):
                if math.dist(cur[-1], s[0]) < eps:
                    cur += s[1:]
                elif math.dist(cur[-1], s[-1]) < eps:
                    cur += s[::-1][1:]
                elif math.dist(cur[0], s[-1]) < eps:
                    cur = s[:-1] + cur
                elif math.dist(cur[0], s[0]) < eps:
                    cur = s[::-1][:-1] + cur
                else:
                    continue
                segs.pop(i)
                grown = True
                break
        out.append(cur)
    return out


def area(poly):
    return sum(
        a[0] * b[1] - b[0] * a[1] for a, b in zip(poly, poly[1:] + poly[:1])
    ) / 2


def keyhole(loops):
    """One polygon from an outline and its holes: each hole, reversed, is
    joined to what is already there by a slit to its nearest vertex."""
    loops = sorted(loops, key=lambda l: -abs(area(l)))
    outer = loops[0]
    if area(outer) < 0:
        outer = outer[::-1]
    poly = list(outer)
    holes = [h if area(h) < 0 else h[::-1] for h in loops[1:]]
    while holes:
        best = None
        for hi, h in enumerate(holes):
            for i, p in enumerate(poly):
                for j, q in enumerate(h):
                    d = math.dist(p, q)
                    if best is None or d < best[0]:
                        best = (d, hi, i, j)
        _, hi, i, j = best
        h = holes.pop(hi)
        ring = h[j:] + h[: j + 1]
        poly = poly[: i + 1] + ring + poly[i:]
    return poly


def main(src):
    doc = ezdxf.readfile(src)
    out = ["# AD5's tree drawings in plan, made by scripts/make_ad5_trees.py."]
    for block, name in BLOCKS:
        b = doc.blocks[block]
        ext = bbox.extents(b)
        size = max(ext.size.x, ext.size.y)
        circles = [e for e in b if e.dxftype() == "CIRCLE"]
        if circles:
            # Drawn on a circle: the crown is that circle.
            big = max(circles, key=lambda e: e.dxf.radius)
            c, r = (big.dxf.center.x, big.dxf.center.y), big.dxf.radius
        else:
            # Rays from the trunk: the crown is centred on where they meet.
            lines = [e for e in b if e.dxftype() == "LINE"]
            if len(lines) == len(b):
                ends = [(e.dxf.start.x, e.dxf.start.y) for e in lines]
                c = min(ends, key=lambda p: math.dist(p, (ext.center.x, ext.center.y)))
                r = max(
                    max(math.dist(c, (e.dxf.start.x, e.dxf.start.y)),
                        math.dist(c, (e.dxf.end.x, e.dxf.end.y)))
                    for e in lines
                )
            else:
                c, r = (ext.center.x, ext.center.y), size / 2
        tol = size / 700
        eps = size / 1e5

        def norm(pts):
            return [((x - c[0]) / r, (y - c[1]) / r) for x, y in pts]

        def trunk(pts):
            # A small ring near the centre: the trunk.
            cx = sum(p[0] for p in pts) / len(pts)
            cy = sum(p[1] for p in pts) / len(pts)
            spread = max(math.dist((cx, cy), p) for p in pts)
            return spread < 0.06 and math.hypot(cx, cy) < 0.15

        lines, washes, patches, segs = [], [], [], []
        for e in b:
            t = e.dxftype()
            if t == "HATCH":
                loops = [norm(flat(p, tol)) for p in zpath.from_hatch(e)]
                loops = [l for l in loops if len(l) >= 3]
                if e.dxf.solid_fill:
                    loops = [l for l in loops if not trunk(l)]
                    washes.append((1.0 - e.transparency, keyhole(loops)))
                else:
                    patches.extend(loops)
            elif t == "LINE":
                segs.append([(e.dxf.start.x, e.dxf.start.y), (e.dxf.end.x, e.dxf.end.y)])
            else:
                pts = flat(zpath.make_path(e), tol)
                if len(pts) < 2 or max(math.dist(pts[0], q) for q in pts) < eps * 10:
                    continue
                closed = t == "CIRCLE" or math.dist(pts[0], pts[-1]) < eps
                if math.dist(pts[0], pts[-1]) < eps:
                    pts = pts[:-1]
                pts = norm(pts)
                if closed and trunk(pts):
                    continue
                lines.append((closed, pts))
        for s in chain(segs, eps):
            closed = len(s) > 3 and math.dist(s[0], s[-1]) < eps
            if closed:
                s = s[:-1]
            lines.append((closed, norm(s)))

        def nums(pts):
            return " ".join(f"{x:.4f} {y:.4f}" for x, y in pts)

        out.append(f"symbol {name}")
        for op, poly in washes:
            out.append(f"wash {op:.3f} {nums(poly)}")
        for poly in patches:
            out.append(f"patch {nums(poly)}")
        for closed, pts in lines:
            reach = max(math.hypot(*p) for p in pts)
            kind = "outline" if closed and reach > 0.7 and len(pts) > 24 else (
                "closed" if closed else "open")
            out.append(f"line {kind} {nums(pts)}")
        print(f"{name}: {len(lines)} líneas, {len(washes)} manchas, {len(patches)} zonas de textura")
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(out) + "\n")
    print("Escrito", os.path.normpath(OUT))


if __name__ == "__main__":
    main(sys.argv[1])
