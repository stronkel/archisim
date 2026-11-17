"""DXF geometry (geom.json) -> ArchiSim build spec (spec.txt) + debug data."""
import json, math, sys, re
from collections import defaultdict, Counter
from shapely.geometry import Polygon, Point, LineString, box
from shapely.ops import unary_union
G=json.load(open(sys.argv[1])); OUT=sys.argv[2]
EPS=1e-9
DOOR_INFO={}
# ---------------- levels ----------------
LEVELS=[("Planta baja",0.0),("Planta primera",4.45),("Cubierta",8.45),("Cubierta castillete",12.35)]
FLOORS={"250819 REF PB":("Planta baja","Planta primera"),
        "250819 REF P1":("Planta primera","Cubierta"),
        "250819 REF PCAST":("Cubierta","Cubierta castillete")}
# ---------------- rooms (sheet coordinates -> floor) ----------------
SHEET={"Planta baja":(0.0,-2,50),"Planta primera":(60.0,55,110),"Cubierta":(123.577491592152,118,170)}
def floor_of_y(y):
    for lv,(dy,a,b) in SHEET.items():
        if a<y<b: return lv,dy
    return None,None
rooms=[]
texts=G["room_texts"]
for poly in G["rooms"]:
    if len(poly)<3: continue
    xs=[p[0] for p in poly]; ys=[p[1] for p in poly]
    if not(-30<min(xs) and max(xs)<130): continue
    lv,dy=floor_of_y(sum(ys)/len(ys))
    if not lv: continue
    P=Polygon(poly)
    if not P.is_valid: P=P.buffer(0)
    if P.area<0.5: continue
    best=None
    for x,y,s in texts:
        if P.contains(Point(x,y)):
            m=re.search(r"([\d.,]+)\s*m²",s.replace("\n"," "))
            area=float(m.group(1).replace(".","").replace(",",".")) if m and "," in m.group(1) else (float(m.group(1)) if m else None)
            name=re.sub(r"\s+"," ",re.split(r"[\d.,]+\s*m²",s.replace("\n"," "))[0].replace("|"," ")).strip()
            cand=(abs((area or P.area)-P.area),name,area,(x,y-dy))
            if best is None or cand[0]<best[0]: best=cand
    if best is None: continue
    Q=P.buffer(0.002,join_style=2).buffer(-0.002,join_style=2).simplify(0.003)
    if Q.geom_type!="Polygon": Q=max(Q.geoms,key=lambda g:g.area)
    rooms.append(dict(level=lv,name=best[1],area=best[2],calc=P.area,label=best[3],
                      pts=[(x,y-dy) for x,y in Q.exterior.coords[:-1]],
                      holes=[[(x,y-dy) for x,y in h.coords[:-1]] for h in Q.interiors if Polygon(h).area>0.3]))
# dedupe: same level+name+area keep smallest area-diff
seen={}
for r in rooms:
    k=(r["level"],r["name"],r["label"])
    if k not in seen or abs(r["calc"]-(r["area"] or r["calc"]))<abs(seen[k]["calc"]-(seen[k]["area"] or seen[k]["calc"])): seen[k]=r
rooms=list(seen.values())
print("rooms",Counter(r["level"] for r in rooms),file=sys.stderr)
OUTSIDE_NAMES=("APARCAMIENTO","ENTRADA Y SALIDA","ACCESO PEATONAL","ZONA PEATONAL")
def is_outdoor(r): return any(k in r["name"].upper() for k in OUTSIDE_NAMES)
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

# ======================= finishing =======================
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
INDOOR=lambda r: not is_outdoor(r) and not any(k in r["name"].upper() for k in ("PATIO","TERRAZA"))
def union_of(rs):
    ps=[Polygon(r["pts"]).buffer(0) for r in rs]
    return unary_union(ps) if ps else None
spec_rooms=[]
lines=[]
for lv,el in LEVELS: lines.append(f"LEVEL|{lv}|{el}")
nlev={lv:i for i,(lv,_) in enumerate(LEVELS)}
wall_count=0
for blk,(lv,top) in FLOORS.items():
    walls,curt=allwalls[lv]
    extend_ends(walls)
    rl=[r for r in rooms if r["level"]==lv and INDOOR(r)]
    inside=union_of(rl)
    for w in walls:
        p1=xy(w["a"],w["off"],w["lo"]); p2=xy(w["a"],w["off"],w["hi"])
        ext=False
        if inside is not None:
            for k in (0.25,0.5,0.75):
                for sgn in (1,-1):
                    m=xy(w["a"],w["off"]+sgn*(w["t"]/2+0.5),w["lo"]+(w["hi"]-w["lo"])*k)
                    if not inside.contains(Point(m)): ext=True
        else: ext=True
        t=snap_t(w["t"])
        fence = lv=="Planta baja" and min(p1[0],p2[0])>31.5
        lines.append(f"WALL|{lv}|{'-' if fence else top}|{t:.3f}|{'ext' if ext else 'int'}|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}"+("|2.0" if fence else ""))
        for o in w["ops"]:
            k=o["k"]; width=round(o["w"],3)
            if t<0.075: continue
            key=None
            for kk,hs in DOOR_INFO.items():
                if abs(kk[0]-round(w["a"],4))<1e-3 and abs(kk[1]-round(w["off"],3))<0.03:
                    for h in hs:
                        if abs(h[0]-(w["lo"]+o["at"]))<width/2+0.2: key=hs;break
                if key: break
            side="ext"; hand="R"
            if key:
                hu,sv,_=key[0]; centre=w["lo"]+o["at"]
                side="ext" if sv>0 else "int"
                at_start = hu<centre
                hand = ("R" if at_start else "L") if sv>0 else ("L" if at_start else "R")
            if k in("door","door2"):
                h=2.10 if width<1.6 else 2.40
                kind="door2" if (k=="door2" or width>1.3) else "door"
                lines.append(f"OPEN|{wall_count}|{kind}|{width:.2f}|{h:.2f}|0.00|{o['at']:.4f}|{side}|{hand}")
            elif k=="window":
                if width>=2.0: lines.append(f"OPEN|{wall_count}|fixed|{width:.2f}|2.10|0.30|{o['at']:.4f}|ext|R")
                else: lines.append(f"OPEN|{wall_count}|window|{width:.2f}|1.20|1.00|{o['at']:.4f}|ext|R")
            else:
                lines.append(f"OPEN|{wall_count}|void|{width:.2f}|{1.90 if fence else 2.10:.2f}|0.00|{o['at']:.4f}|ext|R")
        wall_count+=1
    for c in curt:
        p1=xy(c["a"],c["off"],c["lo"]); p2=xy(c["a"],c["off"],c["hi"])
        lines.append(f"CURTAIN|{lv}|{top}|{p1[0]:.4f}|{p1[1]:.4f}|{p2[0]:.4f}|{p2[1]:.4f}")
# slabs
def poly_lines(level,kind,geom,minarea=4.0):
    out=[]
    gs=[geom] if geom.geom_type=="Polygon" else list(geom.geoms)
    for g in gs:
        if g.area<minarea: continue
        g=g.simplify(0.05)
        if kind=="solera": g=Polygon(g.exterior)
        ext=" ".join(f"{x:.3f},{y:.3f}" for x,y in g.exterior.coords[:-1])
        out.append(f"SLAB|{level}|{kind}|{ext}")
        for h in g.interiors:
            if Polygon(h).area>2: out.append("HOLE|"+" ".join(f"{x:.3f},{y:.3f}" for x,y in h.coords[:-1]))
    return out
def close(g,b): return g.buffer(b,join_style=2).buffer(-b+0.15,join_style=2)
pb=[r for r in rooms if r["level"]=="Planta baja" and INDOOR(r)]
p1all=[r for r in rooms if r["level"]=="Planta primera" and not is_outdoor(r)]
p1in=[r for r in p1all if INDOOR(r)]
lines+=poly_lines("Planta baja","solera",close(union_of(pb),0.4))
lines+=poly_lines("Planta primera","forjado",close(union_of(p1all),0.4))
lines+=poly_lines("Cubierta","cubierta",close(union_of(p1in),0.4))
# castillete roof: footprint of its walls
cw,_=allwalls["Cubierta"]
polys=[]
for w in cw:
    p1=xy(w["a"],w["off"],w["lo"]); p2=xy(w["a"],w["off"],w["hi"])
    polys.append(LineString([p1,p2]).buffer(w["t"]/2+0.05,cap_style=2))
cast=unary_union(polys)
if not cast.is_empty:
    cast=Polygon(cast.convex_hull.exterior) if cast.convex_hull.geom_type=="Polygon" else cast
    lines+=poly_lines("Cubierta castillete","cubierta",cast)
# rooms
n=Counter()
for r in sorted(rooms,key=lambda r:(nlev[r["level"]],-r["label"][1],r["label"][0])):
    if is_outdoor(r) and r["level"]!="Planta baja": continue
    n[r["level"]]+=1; num=f"{'PB' if r['level']=='Planta baja' else 'P1' if r['level']=='Planta primera' else 'PC'}.{n[r['level']]:02d}"
    pts=" ".join(f"{x:.3f},{y:.3f}" for x,y in r["pts"])
    lines.append(f"ROOM|{r['level']}|{num}|{r['name']}|{r['label'][0]:.3f},{r['label'][1]:.3f}|{pts}|{r['area'] or 0:.2f}|"+";".join(" ".join(f"{x:.3f},{y:.3f}" for x,y in h) for h in r["holes"]))
open(OUT,"w").write("\n".join(lines)+"\n")
print("spec lines",Counter(l.split("|")[0] for l in lines),file=sys.stderr)
