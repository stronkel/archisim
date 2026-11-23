"""DXF geometry (geom.json) -> ArchiSim build spec (spec.txt) + debug data."""
import json, math, sys, re
from collections import defaultdict, Counter
from shapely.geometry import Polygon, Point, LineString, box
from shapely.ops import unary_union
G=json.load(open(sys.argv[1])); OUT=sys.argv[2]
EPS=1e-9
DOOR_INFO={}
# ---------------- levels ----------------
LEVELS=[("Planta baja",0.0),("Planta primera",4.55),("Cubierta",8.50),("Cubierta castillete",12.03)]
FLOORS={"PB":("Planta baja","Planta primera"),
        "P1":("Planta primera","Cubierta"),
        "PC":("Cubierta","Cubierta castillete")}
rooms=[]
def is_outdoor(r): return False
# ---------------- walls ----------------
def lines_of(segs):
    out=[]
    for s in segs:
        if s[0]!="L": continue
        x1,y1,x2,y2=s[1:5]; L=math.hypot(x2-x1,y2-y1)
        if L<0.02: continue
        a=math.atan2(y2-y1,x2-x1)%math.pi
        if a>=3*math.pi/4: a-=math.pi
        ux,uy=math.cos(a),math.sin(a)
        off=-uy*x1+ux*y1; t1=ux*x1+uy*y1; t2=ux*x2+uy*y2
        out.append([a,off,min(t1,t2),max(t1,t2)])
    out.sort()
    merged=[]
    for a,off,t1,t2 in out:
        for m in merged:
            if abs(m[0]-a)<0.003 and abs(m[1]-off)<0.004 and t1<=m[3]+0.005 and t2>=m[2]-0.005:
                m[2]=min(m[2],t1); m[3]=max(m[3],t2); break
        else: merged.append([a,off,t1,t2])
    return merged
def pair_walls(merged):
    pieces=[]
    n=len(merged)
    for i in range(n):
        a,o,s1,e1=merged[i]
        for j in range(n):
            if j==i: continue
            b,p,s2,e2=merged[j]
            if abs(a-b)>0.006: continue
            d=p-o
            if d<0.04 or d>0.75: continue
            lo=max(s1,s2); hi=min(e1,e2)
            if hi-lo<min(0.08,d*0.5): continue
            # nearest partner: no line strictly between overlapping most of [lo,hi]
            blocked=False
            for k in range(n):
                c,q,s3,e3=merged[k]
                if k in(i,j) or abs(c-a)>0.006: continue
                if o+0.01<q<p-0.01 and min(e3,hi)-max(s3,lo)>0.5*(hi-lo): blocked=True;break
            if blocked: continue
            pieces.append(dict(a=a,off=(o+p)/2,lo=lo,hi=hi,t=round(d,3)))
    return pieces
def xy(a,off,t):
    ux,uy=math.cos(a),math.sin(a); return (ux*t-uy*off, uy*t+ux*off)
def to_local(a,off,x,y):
    ux,uy=math.cos(a),math.sin(a); return ux*x+uy*y, -uy*x+ux*y - off
def snap_t(t):
    # usual thicknesses in cm
    c=round(t*100)
    return c/100
def build_floor(name):
    F=G[name]
    merged=lines_of(F.get("SECCION",[]))
    pieces=pair_walls(merged)
    carp=F.get("CARPINTERIA",[])
    arcs=[s for s in carp if s[0]=="A"]
    clines=lines_of(carp)
    # group pieces into wall lines
    groups=[]
    for p in sorted(pieces,key=lambda p:(p["a"],p["off"],p["lo"])):
        for g in groups:
            if abs(g["a"]-p["a"])<0.006 and abs(g["off"]-p["off"])<0.02 and abs(g["t"]-p["t"])<0.015:
                g["ps"].append(p); break
        else: groups.append(dict(a=p["a"],off=p["off"],t=p["t"],ps=[p]))
    walls=[]
    for g in groups:
        ps=sorted(g["ps"],key=lambda p:p["lo"])
        # union of intervals
        iv=[]
        for p in ps:
            if iv and p["lo"]<=iv[-1][1]+0.005: iv[-1][1]=max(iv[-1][1],p["hi"])
            else: iv.append([p["lo"],p["hi"]])
        a,off,t=g["a"],g["off"],g["t"]
        cur=None
        for lo,hi in iv:
            if cur is None: cur=dict(a=a,off=off,t=t,lo=lo,hi=hi,ops=[]); continue
            g0,g1=cur["hi"],lo; gap=g1-g0
            kind=classify_gap(a,off,t,g0,g1,arcs,clines,pieces) if gap<=6.0 else None
            if kind is None:
                walls.append(cur); cur=dict(a=a,off=off,t=t,lo=lo,hi=hi,ops=[]); continue
            if kind!="join": cur["ops"].append(dict(k=kind,at=(g0+g1)/2-cur["lo"],w=gap))
            cur["hi"]=hi
        walls.append(cur)
    # drop tiny walls (jambs, ticks)
    walls=[w for w in walls if w["hi"]-w["lo"]>=max(0.25,w["t"]*1.2) or w["ops"]]
    curtains=find_curtains(clines,walls)
    return walls,arcs,clines,curtains
def in_walls(x,y,walls,extra=0.06):
    for w in walls:
        u,v=to_local(w["a"],w["off"],x,y)
        if w["lo"]-extra<u<w["hi"]+extra and abs(v)<w["t"]/2+extra: return True
    return False
def find_curtains(clines,walls):
    # join dashed glazing first
    cl=[]
    for a,o,s1,e1 in sorted(clines):
        for m in cl:
            if abs(m[0]-a)<0.003 and abs(m[1]-o)<0.004 and s1<=m[3]+0.35 and e1>=m[2]-0.35:
                m[2]=min(m[2],s1); m[3]=max(m[3],e1); break
        else: cl.append([a,o,s1,e1])
    clines=cl
    cand=[]
    n=len(clines)
    for i in range(n):
        a,o,s1,e1=clines[i]
        for j in range(n):
            b,p,s2,e2=clines[j]
            if j==i or abs(a-b)>0.006: continue
            d=p-o
            if not(0.015<d<0.3): continue
            lo=max(s1,s2); hi=min(e1,e2)
            if hi-lo<1.0: continue
            mid=(o+p)/2
            # sample: mostly outside walls
            inside=sum(in_walls(*xy(a,mid,lo+(hi-lo)*k/10),walls) for k in range(11))
            if inside>3: continue
            cand.append([a,mid,lo,hi])
    # merge collinear (offset within 0.3) overlapping/close
    cand.sort()
    out=[]
    for a,m,lo,hi in cand:
        for c in out:
            if abs(c[0]-a)<0.01 and abs(c[1]-m)<0.3 and lo<=c[3]+0.6 and hi>=c[2]-0.6:
                c[2]=min(c[2],lo); c[3]=max(c[3],hi); break
        else: out.append([a,m,lo,hi])
    res=[dict(a=a,off=m,lo=lo,hi=hi) for a,m,lo,hi in out if hi-lo>=1.2]
    def near(x,y,skip):
        if in_walls(x,y,walls,0.5): return True
        for c in res:
            if c is skip: continue
            for tt in (c["lo"],c["hi"]):
                q=xy(c["a"],c["off"],tt)
                if math.hypot(q[0]-x,q[1]-y)<0.6: return True
        return False
    keep=[]
    for c in res:
        e1=xy(c["a"],c["off"],c["lo"]); e2=xy(c["a"],c["off"],c["hi"])
        if near(*e1,c) or near(*e2,c): keep.append(c)
    return keep
def classify_gap(a,off,t,g0,g1,arcs,clines,pieces):
    gap=g1-g0; band=t/2+0.05
    # a crossing wall in the gap -> junction
    for p in pieces:
        if abs(p["a"]-a)<0.05 or abs(abs(p["a"]-a)-math.pi)<0.05: continue
        for tt in (p["lo"],p["hi"],(p["lo"]+p["hi"])/2):
            x,y=xy(p["a"],p["off"],tt); u,v=to_local(a,off,x,y)
            if g0-0.05<u<g1+0.05 and abs(v)<band+p["t"]: return "join"
    if gap<0.05: return "join"
    # door swings: arcs whose centre is a jamb of the gap
    hinges=[]
    for s in arcs:
        _,cx,cy,r,px,py,qx,qy=s
        u,v=to_local(a,off,cx,cy)
        if abs(v)<band+0.1 and g0-0.15<u<g1+0.15 and 0.3<r<gap+0.2:
            # the open end is the arc end farthest from the wall line
            pu,pv=to_local(a,off,px,py); qu,qv=to_local(a,off,qx,qy)
            side = pv if abs(pv)>abs(qv) else qv
            hinges.append((u,1 if side>0 else -1,r))
    if hinges:
        DOOR_INFO[(round(a,4),round(off,3),round(g0,3))]=hinges
        return "door2" if len(hinges)>=2 and gap>1.15 else "door"
    # glazing: carpentry lines parallel within the band covering the gap
    cover=0.0
    for c,q,s,e in clines:
        if abs(c-a)>0.01: continue
        v=q-off
        if abs(v)<band: cover=max(cover,min(e,g1)-max(s,g0))
    if cover>0.5*gap: return "window"
    if gap<=0.35: return "join"
    if gap<=2.6: return "void"
    return None
spec=[]
for lv,el in LEVELS: spec.append(f"LEVEL|{lv}|{el}")
allwalls={}
for blk,(lv,top) in FLOORS.items():
    walls,arcs,clines,curt=build_floor(blk)
    allwalls[lv]=(walls,curt)
    print(lv,"walls",len(walls),"openings",Counter(o["k"] for w in walls for o in w["ops"]),file=sys.stderr)
json.dump({k:v[0] for k,v in allwalls.items()},open("walls_dbg.json","w"))
json.dump({k:v[1] for k,v in allwalls.items()},open("curt_dbg.json","w"))
for k,v in allwalls.items(): print(k,"curtain walls",len(v[1]),round(sum(c["hi"]-c["lo"] for c in v[1]),1),"m",file=sys.stderr)
json.dump(rooms,open("rooms_dbg.json","w"))

def line_inter(w,v):
    """parameter along w where v's centre line crosses it, and along v."""
    ax,ay=math.cos(w["a"]),math.sin(w["a"]); bx,by=math.cos(v["a"]),math.sin(v["a"])
    den=ax*by-ay*bx
    if abs(den)<0.2: return None
    p=xy(w["a"],w["off"],0); q=xy(v["a"],v["off"],0)
    dx,dy=q[0]-p[0],q[1]-p[1]
    s=(dx*by-dy*bx)/den; t=(dx*ay-dy*ax)/den
    return s,t
def extend_ends(walls):
    for w in walls:
        for end in ("lo","hi"):
            best=None
            for v in walls:
                if v is w: continue
                r=line_inter(w,v)
                if not r: continue
                s,t=r
                reach=v["t"]/2+0.35
                if end=="lo": ok = w["lo"]-reach<=s<=w["lo"]+0.02
                else: ok = w["hi"]-0.02<=s<=w["hi"]+reach
                if not ok: continue
                if not(v["lo"]-w["t"]/2-0.35<=t<=v["hi"]+w["t"]/2+0.35): continue
                d=abs(s-w[end])
                if best is None or d<best[0]: best=(d,s)
            if best:
                if end=="lo":
                    shift=w["lo"]-best[1]; w["lo"]=best[1]
                    for o in w["ops"]: o["at"]+=shift
                else: w["hi"]=best[1]

# ======================= the twin: from here on, for the per-floor drawings =======================
# Heights read in the cross section (SECCIÓN TRANSVERSAL, finished floors):
# PB ±0,00; P1 +4,55; terrace of the roof +8,50; top of the castillete
# +12,03. Clear heights to the false ceilings 3,35 (PB) and 2,80 (P1).
# Openings: the front and rear elevations, measured from each storey's
# finished floor.
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import outlines, beds
outlines.G = beds.G = G
from outlines import lines as dxf_lines
from beds import faces as dxf_faces
from shapely.geometry import MultiPolygon
from shapely.ops import polygonize
from shapely import prepared

ELEV = dict(LEVELS)
CEIL = {"Planta baja": 3.35, "Planta primera": 2.80}
ROOF_SLAB = 0.545     # the roof's build-up: 35 cm of slab and 19,5 of layers
PARAPET = 1.14        # roof parapet over the terrace (section: +9,64)
OUT_KEY = {"PB": "Planta baja", "P1": "Planta primera", "PC": "Cubierta"}
CLIP = box(15, 0, 74.5, 50)

def ring(pts): return " ".join(f"{x:.3f},{y:.3f}" for x, y in pts)
def polys_of(g):
    if g is None or g.is_empty: return []
    if g.geom_type == "Polygon": return [g]
    return [p for p in getattr(g, "geoms", []) if p.geom_type == "Polygon"]
def seg(w):
    return xy(w["a"], w["off"], w["lo"]), xy(w["a"], w["off"], w["hi"])

lines = []
for lv, el in LEVELS: lines.append(f"LEVEL|{lv}|{el}")

# ---- types: the real build-ups ----
WHITE = "Enfoscado de cal blanco/plaster/244,242,236"
BRICK = "Ladrillo perforado/brick/178,104,78"
WOOL = "Lana mineral/insulation/236,220,150"
BOARD = "Placa de yeso laminado/plaster/238,238,234"
STUD = "Perfilería de acero y lana mineral/insulation/220,214,190"
WOOD = "Tablero de madera de pino tratada/wood/170,120,78"
TILE = "Alicatado cerámico/ceramic/232,232,228"
def walltype(key, name, side, layers):
    lines.append(f"WALLTYPE|{key}|{name}|{side}|" + ";".join(layers))
def facade(t):
    # render, brick, insulation and a lined inner face: as thick as drawn
    core = round(t - 0.015 - 0.06 - 0.0625, 3)
    return [f"{WHITE}/0.015/acabado", f"{BRICK}/{core}/estructura", f"{WOOL}/0.06/aislamiento",
            f"{STUD}/0.05/camara", f"{BOARD}/0.0125/acabado"]
def timber(t):
    core = round(t - 0.025 - 0.04 - 0.06 - 0.0275, 3)
    return [f"{WOOD}/0.025/acabado", "Rastrelado/air/210,210,210/0.04/camara", f"{WOOL}/0.06/aislamiento",
            f"{BRICK}/{core}/estructura", f"{WHITE}/0.0275/acabado"]
def partition(t):
    gap = round(t - 0.05, 3)
    return [f"{BOARD}/0.025/acabado", f"{STUD}/{gap}/camara", f"{BOARD}/0.025/acabado"]
SLAB = {
    "solera": ("Solera con pavimento de gres 60 cm", "floor",
               ["Gres porcelánico/ceramic/222,216,206/0.015/acabado", "Mortero de agarre/mortar/200,196,188/0.035/soporte",
                "Recrecido de mortero/mortar/196,192,186/0.10/soporte", "Lámina de polietileno/membrane/60,60,64/0.002/lamina",
                "Solera de hormigón armado/concrete/190,190,186/0.448/estructura"]),
    "planta": ("Forjado con suelo técnico 60 cm", "floor",
               ["Suelo técnico con loseta/ceramic/214,210,200/0.04/acabado", "Cámara del suelo técnico/air/220,220,220/0.21/camara",
                "Losa de hormigón armado/concrete/200,200,196/0.35/estructura"]),
    "volado": ("Losa volada de hormigón blanco 35 cm", "floor",
               ["Hormigón visto blanco/concrete/236,234,228/0.35/estructura"]),
    "grava": ("Cubierta plana invertida con grava 55 cm", "roof",
              ["Grava/gravel/190,186,176/0.05/acabado", "Geotextil/membrane/230,230,230/0.005/lamina",
               "Poliestireno extruido/insulation/180,214,236/0.08/aislamiento", "Lámina impermeable/membrane/60,60,64/0.01/lamina",
               "Hormigón de pendientes/concrete/200,198,192/0.05/soporte", "Losa de hormigón armado/concrete/200,200,196/0.35/estructura"]),
    "techo": ("Falso techo de placa de yeso", "ceiling", [f"{BOARD}/0.015/acabado"]),
}
for k, (name, fn, ls) in SLAB.items(): lines.append(f"SLABTYPE|{k}|{name}|{fn}|" + ";".join(ls))
# Curtain walls: the PB shop fronts in panes of up to 2,40 m (3,60 on the
# curve), the first floor's in panes of up to 1,50 m; aluminium 6×15 cm.
lines.append("CURTAINTYPE|escaparate|Escaparate de aluminio lacado (módulos 2,40)|2.40|0|0.06|0.15|0.024")
lines.append("CURTAINTYPE|curva|Escaparate de la curva (módulos 3,60)|3.60|0|0.06|0.15|0.024")
lines.append("CURTAINTYPE|acceso|Acceso acristalado con montante|2.40|2.45|0.06|0.15|0.024")
lines.append("CURTAINTYPE|p1|Acristalamiento de la primera (módulos 1,50)|1.50|0|0.05|0.12|0.024")
lines.append("CURTAINTYPE|cast|Acristalamiento del castillete|1.50|0|0.05|0.12|0.024")
made_types = set()
def wall_kind(lv, t, ext):
    cm = int(round(t * 100))
    if lv == "Cubierta":
        key = f"cast{cm}"; name = f"Castillete revestido de madera {cm} cm"; ls = timber(t) if t >= 0.2 else partition(t)
    elif ext and t >= 0.2:
        key = f"fach{cm}"; name = f"Fachada enfoscada con trasdosado {cm} cm"; ls = facade(t)
    elif ext:
        key = f"cerr{cm}"; name = f"Cerramiento enfoscado {cm} cm"
        ls = [f"{WHITE}/0.015/acabado", f"{BRICK}/{round(t-0.03,3)}/estructura", f"{WHITE}/0.015/acabado"]
    elif t >= 0.2:
        key = f"mur{cm}"; name = f"Muro interior de ladrillo trasdosado {cm} cm"
        ls = [f"{BOARD}/0.025/acabado", f"{BRICK}/{round(t-0.05,3)}/estructura", f"{BOARD}/0.025/acabado"]
    else:
        key = f"tab{cm}"; name = f"Tabique de placa de yeso {cm} cm"; ls = partition(t)
    if key not in made_types:
        walltype(key, name, "ext" if ext else "int", ls); made_types.add(key)
    return key

# ---- footprints and patios ----
def floor_union(walls, curt, extra=()):
    parts = [LineString(seg(w)).buffer(w["t"] / 2 + 0.01, cap_style=2) for w in walls]
    parts += [LineString([xy(c["a"], c["off"], c["lo"]), xy(c["a"], c["off"], c["hi"])]).buffer(0.08, cap_style=2) for c in curt]
    parts += list(extra)
    return unary_union(parts)
def footprint(U, close_by=0.9):
    C = U.buffer(close_by, join_style=2).buffer(-close_by, join_style=2)
    return unary_union([Polygon(p.exterior) for p in polys_of(C) if p.area > 30])
def x_voids(blk, minlen=2.0):
    """Rectangles crossed out (two diagonals): voids and openings."""
    from shapely.ops import linemerge
    raw = []
    for s in G[blk].get("PROYECCION", []) + G[blk].get("0", []):
        if s[0] != "L": continue
        if abs(s[3] - s[1]) > 0.05 and abs(s[4] - s[2]) > 0.05 and CLIP.contains(Point(s[1], s[2])):
            raw.append(LineString([(s[1], s[2]), (s[3], s[4])]))
    ls = sorted({(tuple(round(v, 3) for v in g.coords[0]), tuple(round(v, 3) for v in g.coords[-1])) for g in raw if g.length > minlen})
    out = []
    for i in range(len(ls)):
        for j in range(i + 1, len(ls)):
            a, b = LineString(ls[i]), LineString(ls[j])
            if not a.intersects(b): continue
            pts = [ls[i][0], ls[i][1], ls[j][0], ls[j][1]]
            hull = Polygon(pts).convex_hull
            if hull.geom_type != "Polygon" or len(hull.exterior.coords) != 5: continue
            if abs(a.length - b.length) > 0.05 * a.length: continue
            m1 = a.interpolate(0.5, normalized=True); m2 = b.interpolate(0.5, normalized=True)
            if m1.distance(m2) > 0.05: continue
            if CLIP.contains(hull): out.append(hull)
    keep = []
    for h in sorted(out, key=lambda p: -p.area):
        if not any(h.intersection(k).area > 0.5 * h.area for k in keep): keep.append(h)
    return keep
patios = [Polygon(s[1]).buffer(0) for s in G["P1"].get("PAVIMENTO TERRAZA", []) if s[0] == "H" and len(s[1]) > 3]
patios = [p for p in polys_of(unary_union([p for p in patios if p.area > 2]))]
print("patios", [round(p.area, 1) for p in patios], file=sys.stderr)

# ---- louvres, columns ----
def louvres(blk):
    out = []
    for s in G[blk].get("SECCION", []):
        if s[0] != "P" or len(s[1]) not in (4, 5): continue
        pts = s[1][:4]; a = math.dist(pts[0], pts[1]); b = math.dist(pts[1], pts[2])
        L, W = max(a, b), min(a, b)
        if not (0.4 < L < 2.5 and 0.02 < W < 0.2): continue
        i = 0 if a >= b else 1
        p, q = pts[i], pts[i + 1]
        cx = sum(x for x, _ in pts) / 4; cy = sum(y for _, y in pts) / 4
        out.append(dict(c=(cx, cy), rot=math.atan2(q[1] - p[1], q[0] - p[0]), w=L, d=W, poly=Polygon(pts)))
    return out
def columns(blk):
    return [(s[1], s[2], s[3] * 2) for s in G[blk].get("SECCION", []) if s[0] == "C" and 0.08 < s[3] < 0.2]

# ---- the elevations: sill and head of the outside openings ----
def window_heights(lv, w, width, north):
    if lv == "Planta baja": return 0.05, 3.40            # shop fronts, 0,05 → 3,45
    if lv == "Cubierta": return 1.20, 1.75               # castillete ribbon
    if north:                                            # rear: ribbons and slits
        return (1.15, 2.20) if width >= 2.5 else (0.85, 2.50)
    return 0.65, 2.70                                    # front: 0,65 → 3,35

wall_count = 0
ext_walls = {}
parapets = []   # the roof's parapets: over the first floor's outside walls
all_curtains = {}
for blk, (lv, top) in FLOORS.items():
    walls, curt = allwalls[lv]
    walls = [w for w in walls if w["t"] >= 0.075]
    # The plot's walls round the parking: fences, not the building's.
    fences = [w for w in walls if not CLIP.contains(Point(xy(w["a"], w["off"], (w["lo"] + w["hi"]) / 2)))]
    walls = [w for w in walls if w not in fences]
    curt = [c for c in curt if CLIP.contains(Point(xy(c["a"], c["off"], (c["lo"] + c["hi"]) / 2)))]
    if blk == "PB":
        for w in fences:
            p1, p2 = seg(w); t = round(w["t"], 2)
            kind = wall_kind("Planta baja", t, True)
            lines.append(f"WALL|{lv}|-|{t:.3f}|{kind}|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}|2.00")
            wall_count += 1
    lam = louvres(blk) if blk == "P1" else []
    if lam:
        zone = unary_union([l["poly"].buffer(0.05) for l in lam])
        walls = [w for w in walls if not zone.contains(Point(xy(w["a"], w["off"], (w["lo"] + w["hi"]) / 2)))]
        curt = [c for c in curt if not zone.buffer(0.3).contains(Point(xy(c["a"], c["off"], (c["lo"] + c["hi"]) / 2)))]
    U = floor_union(walls, curt)
    # The outline of the storey: the closing of everything drawn in it (the
    # ground floor's own projection on the first's sheet as well), wide
    # enough to bridge its doors and gaps, and the curtain walls.
    drawn = [l for l in dxf_lines(blk, ["SECCION"]) if CLIP.contains(l)]
    if blk == "PB": drawn += [l for l in dxf_lines("P1", ["PRY-PB"]) if CLIP.contains(l)]
    if lam: drawn = [l for l in drawn if not zone.buffer(0.1).intersects(l)]
    R = 3.5 if blk != "PC" else 0.6
    # holes filled while dilated, so that any gap under 2R closes
    D = unary_union(drawn + [U]).buffer(R, join_style=2)
    C = unary_union([Polygon(p.exterior) for p in polys_of(D)]).buffer(-R, join_style=2)
    fp = unary_union([Polygon(p.exterior) for p in polys_of(C) if p.area > 6])
    if os.environ.get("GEMELO_DEBUG"):
        import pickle; pickle.dump((C, fp, unary_union(drawn + [U])), open(f"/tmp/fp_{blk}.pkl", "wb"))
    if blk == "P1":
        for p in patios: fp = fp.difference(p.buffer(0.05))
        # no slivers left between a patio and the edge
        fp = unary_union([g for g in polys_of(fp.buffer(-0.4, join_style=2).buffer(0.4, join_style=2)) if g.area > 6])
    centre = fp.centroid
    # Walls drawn wider than any real one (the north party line and the
    # wall inside it): 30 cm, against the inside face.
    for w in walls:
        if w["t"] > 0.35 and w["hi"] - w["lo"] > 2:
            u, v = to_local(w["a"], w["off"], centre.x, centre.y)
            inner = w["off"] + (w["t"] / 2 if v > 0 else -w["t"] / 2)
            w["off"] = inner - (0.15 if v > 0 else -0.15); w["t"] = 0.30
    # Pieces of one wall split by their drawing, now alike: one wall.
    walls.sort(key=lambda w: (round(w["a"], 3), round(w["off"], 2), w["lo"]))
    joined = []
    for w in walls:
        j = joined[-1] if joined else None
        if j and abs(j["a"] - w["a"]) < 0.004 and abs(j["off"] - w["off"]) < 0.02 and abs(j["t"] - w["t"]) < 0.005 \
                and -0.05 < w["lo"] - j["hi"] < 0.12 and not w["ops"]:
            j["hi"] = max(j["hi"], w["hi"]); continue
        joined.append(w)
    walls = joined
    extend_ends(walls)
    fpp = prepared.prep(fp)
    for w in walls:
        p1, p2 = seg(w); L = w["hi"] - w["lo"]
        outl = outr = 0
        for k in (0.2, 0.5, 0.8):
            for sgn in (1, -1):
                m = xy(w["a"], w["off"] + sgn * (w["t"] / 2 + 0.6), w["lo"] + L * k)
                if not fpp.contains(Point(m)):
                    if sgn > 0: outl += 1
                    else: outr += 1
        ext = (outl + outr) >= 2 or lv == "Cubierta" and (outl + outr) > 0
        flip = ext and outr > outl
        t = round(w["t"], 2)
        kind = wall_kind(lv, t, ext)
        if flip: p1, p2 = p2, p1
        nx, ny = -(p2[1] - p1[1]), p2[0] - p1[0]          # left of the line: outside
        north = ext and ny > 0.7 * math.hypot(nx, ny) or ext and nx > 0.7 * math.hypot(nx, ny)
        # the first floor's walls stop under the roof slab, which bears on them
        under = f"3.0|{-ROOF_SLAB:.3f}" if lv == "Planta primera" else ""
        lines.append(f"WALL|{lv}|{top}|{t:.3f}|{kind}|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}|{under}")
        if ext and lv == "Planta primera":
            parapets.append((p1, p2, t, kind))
        if ext: ext_walls.setdefault(lv, []).append((p1, p2, t))
        for o in w["ops"]:
            k = o["k"]; width = round(o["w"], 3)
            key = None
            for kk, hs in DOOR_INFO.items():
                if abs(kk[0] - round(w["a"], 4)) < 1e-3 and abs(kk[1] - round(w["off"], 3)) < 0.03:
                    for h in hs:
                        if abs(h[0] - (w["lo"] + o["at"])) < width / 2 + 0.2: key = hs; break
                if key: break
            side = "ext"; hand = "R"
            if key:
                hu, sv, _ = key[0]; centre_u = w["lo"] + o["at"]
                side = "ext" if sv > 0 else "int"
                at_start = hu < centre_u
                hand = ("R" if at_start else "L") if sv > 0 else ("L" if at_start else "R")
            at = o["at"]
            if flip: at = L - at; side = {"ext": "int", "int": "ext"}[side]
            if k in ("door", "door2"):
                kd = "door2" if (k == "door2" or width > 1.3) else "door"
                if ext and lv == "Planta baja":
                    # outside doors: leaves of 2,40 and a fixed light over them to 3,45
                    lines.append(f"OPEN|{wall_count}|{kd}|{width:.2f}|2.40|0.00|{at:.4f}|{side}|{hand}")
                    lines.append(f"OPEN|{wall_count}|fixed|{width:.2f}|0.95|2.50|{at:.4f}|ext|R")
                else:
                    h = 2.50 if width >= 1.6 else 2.08
                    lines.append(f"OPEN|{wall_count}|{kd}|{width:.2f}|{h:.2f}|0.00|{at:.4f}|{side}|{hand}")
            elif k == "window":
                if ext:
                    sill, h = window_heights(lv, w, width, north)
                    lines.append(f"OPEN|{wall_count}|{'fixed' if width >= 2.0 or lv == 'Planta baja' else 'window'}|{width:.2f}|{h:.2f}|{sill:.2f}|{at:.4f}|ext|R")
                else:
                    lines.append(f"OPEN|{wall_count}|fixed|{width:.2f}|1.20|0.90|{at:.4f}|ext|R")
            else:
                lines.append(f"OPEN|{wall_count}|void|{width:.2f}|2.08|0.00|{at:.4f}|ext|R")
        wall_count += 1
    # Curtain walls: their doors where a door swing sits on them.
    arcs = [s for s in G[blk].get("CARPINTERIA", []) if s[0] == "A" and 0.4 < s[3] < 1.3]
    for c in curt:
        p1 = xy(c["a"], c["off"], c["lo"]); p2 = xy(c["a"], c["off"], c["hi"])
        L = c["hi"] - c["lo"]
        on_curve = abs(math.sin(2 * c["a"])) > 0.2
        doors = []
        for s in arcs:
            u, v = to_local(c["a"], c["off"], s[1], s[2])
            if c["lo"] + 0.1 < u < c["hi"] - 0.1 and abs(v) < 0.25:
                doors.append((u - c["lo"], s[3]))
        if lv == "Planta baja":
            if doors:
                # bays: up to each door, the door leaves, and on
                doors.sort(); bays = []; at = 0.0; door_spec = []
                for u, r in doors:
                    left = u - at
                    if left < 0.3: continue
                    if left > 0.35: bays.append(round(left, 3))
                    else: bays[-1:] = [round((bays[-1] if bays else 0) + left, 3)] if bays else []
                    dw = round(min(r * (2 if r < 1.0 else 1), L - u), 3) if False else round(r, 3)
                    door_spec.append(f"{len(bays)}:{dw:.2f}:2.40:R:ext")
                    bays.append(dw); at = u + dw
                bays = [b for b in bays if b >= 0.1]
                lines.append(f"CURTAIN|{lv}|-|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}|acceso|0.00|3.45|{';'.join(f'{b:.3f}' for b in bays)}|{','.join(door_spec)}")
            else:
                lines.append(f"CURTAIN|{lv}|-|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}|{'curva' if on_curve else 'escaparate'}|0.00|3.45")
        elif lv == "Planta primera":
            lines.append(f"CURTAIN|{lv}|-|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}|p1|0.00|2.80")
        else:
            lines.append(f"CURTAIN|{lv}|{top}|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}|cast|0.00|3.0")
    # What is left of the storey's edge with no wall nor glazing on it is
    # glass too (the shop fronts drawn as a single line).
    if lv != "Cubierta":
        covered = unary_union([LineString(seg(w)).buffer(w["t"] / 2 + 0.35, cap_style=2) for w in walls] +
                              [LineString([xy(c["a"], c["off"], c["lo"]), xy(c["a"], c["off"], c["hi"])]).buffer(0.45, cap_style=2) for c in curt])
        rings = [(p.exterior, False) for p in polys_of(fp)] + ([] if lv != "Planta primera" else [(p.exterior, True) for p in patios])
        for ring_, patio in rings:
            ccw = ring_.is_ccw
            pts = list(ring_.simplify(0.05).coords)
            for a_, b_ in zip(pts, pts[1:]):
                edge = LineString([a_, b_]).difference(covered)
                for piece in getattr(edge, "geoms", [edge]):
                    if piece.is_empty or piece.length < 0.9: continue
                    (x1, y1), (x2, y2) = piece.coords[0], piece.coords[-1]
                    L_ = math.hypot(x2 - x1, y2 - y1)
                    nx, ny = ((y2 - y1) / L_, -(x2 - x1) / L_) if ccw else (-(y2 - y1) / L_, (x2 - x1) / L_)
                    if patio: nx, ny = -nx, -ny          # a patio's edge faces into it
                    mx, my = (x1 + x2) / 2, (y1 + y2) / 2
                    if any(p.distance(Point(mx + nx * 0.6, my + ny * 0.6)) < 0.3 for p in patios): patio = True
                    rear = nx > 0.7 or ny > 0.7          # east (parking) and north: built
                    if rear and not patio:
                        # a 30 cm wall just inside the edge, its outside on its left,
                        # clear of the walls already there
                        ox, oy = -nx * 0.15, -ny * 0.15
                        body = unary_union([LineString(seg(w)).buffer(w["t"] / 2 + 0.16, cap_style=2) for w in walls])
                        cl = LineString([(x1 + ox, y1 + oy), (x2 + ox, y2 + oy)]).difference(body)
                        cl = max(getattr(cl, "geoms", [cl]), key=lambda g: g.length)
                        if cl.is_empty or cl.length < 0.9: continue
                        p1, p2 = cl.coords[0], cl.coords[-1]
                        if (p2[0] - p1[0]) * ny - (p2[1] - p1[1]) * nx > 0: p1, p2 = p2, p1
                        kind = wall_kind(lv, 0.30, True)
                        under = f"{-ROOF_SLAB:.3f}" if lv == "Planta primera" else ""
                        lines.append(f"WALL|{lv}|{top}|0.300|{kind}|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}|3.0|{under}|libre")
                        if lv == "Planta primera": parapets.append((p1, p2, 0.30, kind))
                        wall_count += 1
                    elif lv == "Planta baja":
                        lines.append(f"CURTAIN|{lv}|-|{x1:.4f}|{y1:.4f}|{x2:.4f}|{y2:.4f}|escaparate|0.00|3.45")
                    else:
                        lines.append(f"CURTAIN|{lv}|-|{x1:.4f}|{y1:.4f}|{x2:.4f}|{y2:.4f}|p1|0.00|2.80")
    all_curtains[lv] = curt
    for x, y, d in columns(blk):
        lines.append(f"COLUMN|{lv}|{top}|{d:.3f}|{x:.4f}|{y:.4f}")
    for l in lam:
        # from the floor to under the roof slab: 4,55 − (8,50 − 0,545) … 3,40 clear
        h = ELEV[top] - ELEV[lv] - 0.545
        lines.append(f"LOUVRE|{lv}|{l['c'][0]:.4f}|{l['c'][1]:.4f}|{l['rot']:.5f}|{l['w']:.3f}|{h:.2f}")
    allwalls[lv] = (walls, curt, fp, U)

# The parapets of the roof terrace (section: +9,64, 1,14 over it), walls of
# their own standing on the roof over the first floor's outside walls.
for p1, p2, t, kind in parapets:
    lines.append(f"WALL|Cubierta|-|{t:.3f}|{kind}|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}|{PARAPET:.2f}")
    wall_count += 1

# ---- stairs (flights read from the tread lines of PROYECCIÓN) ----
STAIRS = [
    # The main stair, a U round a central wall: PB → P1 (13 + 13 risers) …
    ("Planta baja", "Planta primera", 0.0, 1.50, 0.30, [(50.54, 23.89), (50.54, 28.24), (52.39, 28.24), (52.39, 23.89)]),
    # … and on to the roof terrace in the castillete (12 + 12)
    ("Planta primera", "Cubierta", 0.0, 1.50, 0.30, [(50.53, 24.19), (50.53, 28.24), (52.40, 28.24), (52.40, 24.19)]),
    # The west stair: three flights round its well (8 + 7 + 8)
    ("Planta baja", "Planta primera", 0.0, 1.20, 0.30, [(38.51, 12.56), (41.21, 12.56), (41.21, 13.86), (38.21, 13.86), (38.21, 15.16), (40.91, 15.16)]),
]
# Straight stairs with a landing halfway: two flights and the landing slab.
for (x0, y0), (x1, y1), (x2, y2), (x3, y3), w, r1 in [
        ((21.28, 40.66), (25.18, 40.66), (26.68, 40.66), (30.28, 40.66), 1.20, 14),
        ((63.46, 32.52), (59.86, 32.52), (58.36, 32.52), (54.76, 32.52), 1.25, 13)]:
    n = r1 + round(math.dist((x2, y2), (x3, y3)) / 0.30) + 1
    h1 = round(ELEV["Planta primera"] * r1 / n, 3)
    STAIRS.append(("Planta baja", "-", 0.0, w, 0.30, [(x0, y0), (x1, y1)], h1))
    STAIRS.append(("Planta baja", "Planta primera", h1, w, 0.30, [(x2, y2), (x3, y3)]))
    ux, uy = (x2 - x1) / math.dist((x1, y1), (x2, y2)), (y2 - y1) / math.dist((x1, y1), (x2, y2))
    px, py = -uy * w / 2, ux * w / 2
    land = [(x1 + px, y1 + py), (x2 + px, y2 + py), (x2 - px, y2 - py), (x1 - px, y1 - py)]
    lines.append(f"SLAB|Planta baja|volado|{ring(land)}|{h1:.3f}")
stair_zones = []
for st in STAIRS:
    lv, top, base, w, g, path = st[:6]
    height = st[6] if len(st) > 6 else ELEV[top] - ELEV[lv]
    lines.append(f"STAIR|{lv}|{top}|{height:.3f}|{base:.3f}|{w:.2f}|{g:.2f}|{ring(path)}")
    stair_zones.append((lv, LineString(path).buffer(w / 2 + 0.05, cap_style=2, join_style=2)))

# ---- lifts (ASCENSOR: two cars side by side, doors to the east) ----
lifts = [b for b in G["PB"].get("BLOQUES", []) if b[1] == "M- Ascensor"]
for b in lifts:
    cx, cy, bw, bd = b[6], b[7], b[8], b[9]
    lines.append(f"LIFT|Planta baja|Planta primera|{cx:.3f}|{cy:.3f}|{-math.pi/2:.5f}|{bd:.2f}|{bw:.2f}|1.00|1.25|0.80|450")

# ---- slabs ----
def outer(blk, lays):
    ls = [l for l in dxf_lines(blk, lays) if CLIP.contains(l)]
    Uo = unary_union(ls).buffer(0.03)
    big = max(polys_of(Uo), key=lambda g: Polygon(g.exterior).area)
    return Polygon(big.exterior).buffer(-0.03)
USED = {}
def bodies(lv):
    ws = allwalls[lv][0]
    return [LineString(seg(w)).buffer(w["t"] / 2, cap_style=2, join_style=2) for w in ws]
def whole(geom, walls_below):
    """A slab that takes in whole every wall it partly covers, so that no
    wall meets its edge halfway through its thickness."""
    part = [b for b in walls_below if b.intersects(geom) and not geom.contains(b)]
    return unary_union([geom] + [b.buffer(0.005, join_style=2) for b in part])
def slab_lines(level, kind, geom, minarea=4.0, holes=(), top=None, walls_below=()):
    out = []
    keep = unary_union(list(walls_below)).buffer(0.005, join_style=2) if walls_below else None
    for h in holes:
        if keep is not None: h = h.difference(keep)
        geom = geom.difference(h)
    if walls_below: geom = whole(geom, walls_below)
    for g in polys_of(geom):
        if g.area < minarea: continue
        g = g.simplify(0.02)
        # never two slabs over the same ground of a level
        if level in USED: g = g.difference(USED[level].buffer(0.003, join_style=2))
        for g in polys_of(g):
            g = g.buffer(0.002, join_style=2).buffer(-0.002, join_style=2).simplify(0.02)
            if g.geom_type != "Polygon" or not g.is_valid or g.area < minarea: continue
            g = Polygon(g.exterior.coords, [h for h in g.interiors if Polygon(h).area > 0.6])
            USED[level] = unary_union([USED.get(level, Polygon()), g])
            out.append(f"SLAB|{level}|{kind}|{ring(g.exterior.coords[:-1])}" + (f"|{top:.3f}" if top is not None else ""))
            for h in g.interiors: out.append("HOLE|" + ring(h.coords[:-1]))
    return out
fpPB = allwalls["Planta baja"][2]
fpP1 = allwalls["Planta primera"][2]
edgeP1 = outer("P1", ["PROYECCION", "SECCION"])
voids_p1 = x_voids("P1")
voids_pc = x_voids("PC")
print("voids P1", [round(v.area, 1) for v in voids_p1], "roof", [round(v.area, 1) for v in voids_pc], file=sys.stderr)
lines += slab_lines("Planta baja", "solera", fpPB)
stair_holes_p1 = [z for lv, z in stair_zones if lv == "Planta baja"]
p1_slab = fpP1.union(edgeP1).union(unary_union(patios))
inside_p1 = p1_slab
lines += slab_lines("Planta primera", "planta", fpP1.union(unary_union(patios)), holes=voids_p1 + stair_holes_p1)
volado = edgeP1.difference(fpP1.union(unary_union(patios)).buffer(0.01))
lines += slab_lines("Planta primera", "volado", volado, minarea=1.0)
edgePC = outer("PC", ["PROYECCION", "SECCION"])
cast_walls = allwalls["Cubierta"][0]
cast_fill = allwalls["Cubierta"][2]
roof = edgePC.union(fpP1)
roof_holes = [p.buffer(0.02) for p in patios] + voids_pc + [z for lv, z in stair_zones if lv == "Planta primera"]
lines += slab_lines("Cubierta", "grava", roof, holes=roof_holes, walls_below=bodies("Planta primera"))
lines += slab_lines("Cubierta castillete", "volado", cast_fill.buffer(0.15, join_style=2))
print("slabs: P1", round(fpP1.area, 1), "volado", round(volado.area, 1), "roof", round(roof.area, 1), "castillete", round(cast_fill.area, 1), file=sys.stderr)

# ---- planters on the roof and the first floor ----
PLANTS = {"Cubierta": ("Palmito;Pennisetum;Romero;Lavanda", 0.60), "Planta primera": ("Pennisetum;Romero;Lavanda;Palmito", 0.50)}
nbeds = 0
for blk, lv, lays in (("PC", "Cubierta", ["PROYECCION", "SECCION"]), ("P1", "Planta primera", ["PROYECCION", "SECCION", "CARPINTERIA"])):
    species, height = PLANTS[lv]
    pts = [(s[1], s[2]) for s in G[blk].get("JARDINERIA", []) if s[0] in ("L", "C") and CLIP.contains(Point(s[1], s[2]))]
    found = []
    for f in dxf_faces(blk, lays):
        if f.area > 150 or f.area < 0.8 or not CLIP.contains(f): continue
        pf = prepared.prep(f); n = sum(1 for q in pts if pf.contains(Point(q)))
        if n >= 20 and n / f.area > 8: found.append(f)
    keep = []
    for f in sorted(found, key=lambda f: -f.area):
        if not any(f.intersection(k).area > 0.3 * f.area for k in keep): keep.append(f)
    for g in keep:
        g = g.simplify(0.04); t = 0.12
        centre = g.buffer(-t / 2, join_style=2).simplify(0.03)
        earth = g.buffer(-t - 0.01, join_style=2).simplify(0.03)
        if centre.geom_type != "Polygon" or earth.geom_type != "Polygon" or earth.area < 0.3: continue
        lines.append(f"PLANTER|{lv}|{height:.2f}|{t:.2f}|{ring(centre.exterior.coords[:-1])}|{ring(earth.exterior.coords[:-1])}|{species}|{max(3, int(earth.area * 2))}")
        nbeds += 1

# ---- solar panels: the PLACA_FOTOVOLTAICA blocks, joined in rows ----
pv = [b for b in G["PC"].get("BLOQUES", []) if b[1] == "PLACA_FOTOVOLTAICA"]
rows = {}
for b in pv:
    cx, cy, bw, bd, r = b[6], b[7], b[8], b[9], b[5]
    rows.setdefault((round(cy, 1), round(r, 2), round(bw, 2), round(bd, 2)), []).append(cx)
nrows = 0
for (cy, r, bw, bd), xs in rows.items():
    xs.sort(); run = [xs[0]]
    def flush(run):
        global nrows
        cx = (run[0] + run[-1]) / 2; width = run[-1] - run[0] + bw
        lines.append(f"PV|Cubierta|{cx:.3f}|{cy:.3f}|{r:.4f}|{width:.3f}|{bd:.3f}|0.35"); nrows += 1
    for x in xs[1:]:
        if x - run[-1] < bw + 0.1: run.append(x)
        else: flush(run); run = [x]
    flush(run)
print("solar panels", len(pv), "in", nrows, "rows", file=sys.stderr)

# ---- objects from the drawing's blocks ----
OBJ = {"Inodoro": "Inodoro", "BATER MINUSVALIDO": "Inodoro", "LAVABO": "Lavabo 60", "PUPITRE": "Escritorio metálico 120",
       "BUTACA": "Sillón", "SOFA1P": "Sillón", "SOFA3P-PERS": "Sofá de 3 plazas", "H3-flat-1": "Mesa de centro",
       "Coche": "Turismo", "COLUMNA-LUMINARIA-DOBLE": "Farola 4,5 m", "COLUMNA-LUMINARIA-SIMPLE": "Farola 4,5 m",
       "BANCO_HORMIGON": "Banco con respaldo", "PAPELERA": "Papelera", "HORQUILLA_BICI": "Aparcabicis 5 plazas",
       "ISLA ALUMNOS": "Mesa de patas metálicas 140×80", "MESA": "Mesa de patas metálicas 140×80"}
nobj = 0
seen = set()
for blk, lv in (("PB", "Planta baja"), ("P1", "Planta primera")):
    for b in G[blk].get("BLOQUES", []):
        name = next((v for k, v in OBJ.items() if b[1] == k or (k in ("ISLA ALUMNOS", "MESA") and b[1].startswith(k))), None)
        if not name: continue
        cx, cy = b[6], b[7]
        outside = not (15 < cx < 74.5 and 0 < cy < 50)
        if outside and blk == "P1": continue          # the site is drawn on every floor
        key = (name, round(cx, 1), round(cy, 1))
        if key in seen: continue
        seen.add(key)
        lines.append(f"OBJECT|{lv}|{name}|{cx:.3f}|{cy:.3f}|{b[5]:.4f}"); nobj += 1

# ---- rooms: the spaces the walls close, named by what is in them ----
def name_room(lv, P, blk):
    pp = prepared.prep(P)
    inside = lambda x, y: pp.contains(Point(x, y))
    blocks = [b[1] for b in G[blk].get("BLOQUES", []) if inside(b[6], b[7])]
    texts = " ".join(s[3] for s in G[blk].get("TEXTO OCULTO", []) if s[0] == "T" and inside(s[1], s[2])).upper()
    if any(b == "M- Ascensor" for b in blocks): return "Ascensor"
    if any(z.intersection(P).area > 0.5 * P.area for l2, z in stair_zones if l2 == lv): return "Escalera"
    if any(b in ("TORITO", "TRANSPALETA", "CAMION") for b in blocks): return "Muelle de carga"
    if lv == "Planta baja" and P.centroid.y > 41 and P.area > 6: return "Almacén"
    if "BATER MINUSVALIDO" in blocks: return "Aseo accesible"
    if "Inodoro" in blocks or "LAVABO" in blocks: return "Aseos"
    if "ISLA ALUMNOS" in texts or "DEMO DOCENTE" in texts: return "Aula taller de pastelería"
    if "HORNO" in texts or "AMASADORA" in texts or "ARMARIO" in texts: return "Obrador"
    if "PUPITRE" in blocks: return "Aula de formación"
    if "CAJERO_AUTOMATICO" in blocks: return "Tienda"
    if any(b.startswith("SOFA") or b == "BUTACA" or b == "H3-flat-1" for b in blocks):
        if lv == "Planta baja": return "Cafetería"
        return "Vestíbulo" if P.area > 80 else "Sala de espera"
    mob = [s for s in G[blk].get("MOB OFICINA", []) if s[0] == "L" and inside(s[1], s[2])]
    if len(mob) > 40: return "Oficina"
    r = P.minimum_rotated_rectangle; xs = list(r.exterior.coords)
    a, b2 = math.dist(xs[0], xs[1]), math.dist(xs[1], xs[2])
    if min(a, b2) < 2.2 and max(a, b2) > 4: return "Pasillo"
    if P.area < 6: return "Almacén"
    if lv == "Planta primera": return "Despacho" if P.area < 30 else "Oficina"
    return "Sala"
nroom = 0
for blk, lv in (("PB", "Planta baja"), ("P1", "Planta primera"), ("PC", "Cubierta")):
    walls, curt, fp, U = allwalls[lv]
    # Door gaps closed: the walls whole, as drawn from end to end.
    closed = unary_union([U] + [LineString(seg(w)).buffer(w["t"] / 2, cap_style=2) for w in walls])
    # Doors closed too: from each swing's hinge to both its ends, and the
    # carpentry's own lines (glass partitions, frames).
    shut = [LineString([(c[1], c[2]), (c[3], c[4])]) for c in G[blk].get("CARPINTERIA", [])
            if c[0] == "L" and (c[1], c[2]) != (c[3], c[4]) and CLIP.contains(Point(c[1], c[2]))]
    for a in G[blk].get("CARPINTERIA", []):
        if a[0] == "A" and 0.3 < a[3] < 1.5:
            # the leaf shut: to the end of the swing that meets a wall
            ends = [(a[4], a[5]), (a[6], a[7])]
            far = min(ends, key=lambda e: U.distance(Point(e)))
            shut.append(LineString([(a[1], a[2]), far]))
    closed = unary_union([closed, unary_union(shut).buffer(0.04)])
    spaces = []
    free = fp.difference(closed.buffer(0.005))
    for g in polys_of(fp.difference(closed.buffer(0.3, join_style=2))):
        g = g.buffer(0.3, join_style=2).intersection(free)
        g = max(polys_of(g), key=lambda p: p.area, default=None)
        if g is None or g.area < 1.0: continue
        spaces.append(g)
    count = Counter()
    order = sorted(spaces, key=lambda g: (-g.centroid.y, g.centroid.x))
    names = [name_room(lv, g, blk) for g in order]
    # the biggest unnamed space of the ground floor is the shop
    if lv == "Planta baja":
        big = max(range(len(order)), key=lambda i: order[i].area if names[i] == "Sala" else 0)
        if names[big] == "Sala": names[big] = "Tienda"
    for g, nm in zip(order, names):
        g = g.buffer(0.002, join_style=2).buffer(-0.002, join_style=2).simplify(0.01)
        if g.geom_type != "Polygon": continue
        count[lv] += 1
        num = f"{'PB' if lv == 'Planta baja' else 'P1' if lv == 'Planta primera' else 'PC'}.{count[lv]:02d}"
        lab = g.representative_point()
        holes = ";".join(ring(h.coords[:-1]) for h in g.interiors if Polygon(h).area > 0.3)
        lines.append(f"ROOM|{lv}|{num}|{nm}|{lab.x:.3f},{lab.y:.3f}|{ring(g.exterior.coords[:-1])}|{g.area:.2f}|{holes}")
        nroom += 1
        if lv in CEIL and nm not in ("Escalera", "Ascensor") and g.area > 1.5:
            lines.append(f"CEILING|{lv}|techo|{CEIL[lv]:.2f}|{ring(g.exterior.coords[:-1])}")
    if lv == "Planta primera":
        for i, p in enumerate(patios):
            lab = p.representative_point(); count[lv] += 1
            lines.append(f"ROOM|{lv}|P1.{count[lv]:02d}|Patio|{lab.x:.3f},{lab.y:.3f}|{ring(p.simplify(0.01).exterior.coords[:-1])}|{p.area:.2f}|")
            nroom += 1
print("rooms", nroom, "planters", nbeds, "objects", nobj, file=sys.stderr)
open(OUT, "w").write("\n".join(lines) + "\n")
print("spec lines", Counter(l.split("|")[0] for l in lines), file=sys.stderr)
