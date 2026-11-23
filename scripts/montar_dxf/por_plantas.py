"""Plantas en ficheros DXF separados -> el dibujo aplanado (geom.json) que
leen describir.py y gemelo.py, en las coordenadas de la planta baja.

    python3 -I por_plantas.py salida.json PB=baja.dxf P1=primera.dxf@dx,dy PC=cubierta.dxf@dx,dy

Cada planta se aplana entera (bloques dentro de bloques, en coordenadas
reales) y se desplaza lo que diga su @dx,dy para casar con la baja. Las
capas pierden el prefijo «01-BASICO-»; los arcos llevan también su
trazado («AP»), como los de leer_dxf.py.
"""
import sys, json, math, logging
import ezdxf
from ezdxf import path as zpath
logging.disable(logging.WARNING)

def short(layer):
    l = layer.split("$0$")[-1]
    for p in ("01-BASICO-", "01-BASICO "):
        if l.startswith(p): return l[len(p):]
    return l

def flatten(fname, dx, dy):
    doc = ezdxf.readfile(fname); out = {}
    stack = []   # the points of the blocks being expanded
    def add(lay, item):
        out.setdefault(lay, []).append(item)
        if stack:
            k = item[0]
            pts = [(item[1], item[2]), (item[3], item[4])] if k == "L" else item[1] if k in ("P", "AP") else [(item[1] - item[3], item[2] - item[3]), (item[1] + item[3], item[2] + item[3])] if k == "C" else []
            for s in stack: s.extend(pts)
    def P(x, y): return (x + dx, y + dy)
    def one(e, parent):
        lay = short(e.dxf.layer)
        if lay == "0" and parent: lay = parent
        t = e.dxftype()
        if t == "INSERT":
            try: ents = list(e.virtual_entities())
            except Exception: return
            stack.append([])
            for s in ents: one(s, lay)
            pts = stack.pop()
            # Its box in its own axes: centre, length and width.
            r = math.radians(e.dxf.rotation); ix, iy = P(e.dxf.insert.x, e.dxf.insert.y)
            c, s_ = math.cos(r), math.sin(r)
            loc = [((x - ix) * c + (y - iy) * s_, -(x - ix) * s_ + (y - iy) * c) for x, y in pts]
            if loc:
                u0 = min(p[0] for p in loc); u1 = max(p[0] for p in loc); v0 = min(p[1] for p in loc); v1 = max(p[1] for p in loc)
                cu, cv = (u0 + u1) / 2, (v0 + v1) / 2
                box = (ix + cu * c - cv * s_, iy + cu * s_ + cv * c, u1 - u0, v1 - v0)
            else: box = (ix, iy, 0.0, 0.0)
            for st in stack: st.extend(pts)
            out.setdefault("BLOQUES", []).append(("B", e.dxf.name.split("$0$")[-1], lay, ix, iy, r, *box))
        elif t == "LINE":
            a, b = e.dxf.start, e.dxf.end; add(lay, ("L", *P(a.x, a.y), *P(b.x, b.y)))
        elif t == "ARC":
            c = e.ocs().to_wcs(e.dxf.center); p, q = e.start_point, e.end_point
            add(lay, ("A", *P(c.x, c.y), e.dxf.radius, *P(p.x, p.y), *P(q.x, q.y)))
            add(lay, ("AP", [P(v.x, v.y) for v in e.flattening(0.01)]))
        elif t == "CIRCLE":
            c = e.ocs().to_wcs(e.dxf.center); add(lay, ("C", *P(c.x, c.y), e.dxf.radius))
        elif t in ("LWPOLYLINE", "POLYLINE"):
            try:
                if t == "LWPOLYLINE":
                    pts = [e.ocs().to_wcs((x, y, 0)) for x, y, *_ in e.get_points()]
                    add(lay, ("P", [P(q.x, q.y) for q in pts], bool(e.closed)))
            except Exception: pass
            for s in e.virtual_entities(): one(s, lay)
        elif t in ("TEXT", "MTEXT"):
            add(lay, ("T", *P(e.dxf.insert.x, e.dxf.insert.y), e.dxf.text if t == "TEXT" else e.plain_text()))
        elif t == "HATCH":
            for p in e.paths:
                try:
                    pts = [P(v.x, v.y) for v in zpath.from_hatch_boundary_path(p, e.ocs(), e.dxf.elevation.z).flattening(0.02)]
                    add(lay, ("H", pts, e.dxf.pattern_name))
                except Exception: pass
        elif t in ("SPLINE", "ELLIPSE"):
            pts = list(zpath.make_path(e).flattening(0.02))
            for p, q in zip(pts, pts[1:]): add(lay, ("L", *P(p.x, p.y), *P(q.x, q.y)))
    for e in doc.modelspace(): one(e, None)
    return out

if __name__ == "__main__":
    res = {"rooms": [], "room_texts": []}
    for arg in sys.argv[2:]:
        key, rest = arg.split("=", 1)
        fname, _, shift = rest.partition("@")
        dx, dy = (float(v) for v in shift.split(",")) if shift else (0.0, 0.0)
        res[key] = flatten(fname, dx, dy)
        print(key, {k: len(v) for k, v in res[key].items() if k in ("SECCION", "CARPINTERIA", "PROYECCION")}, file=sys.stderr)
    json.dump(res, open(sys.argv[1], "w"))
