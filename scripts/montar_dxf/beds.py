import json,sys,collections

import outlines
from outlines import lines
from shapely.geometry import Polygon, Point
from shapely.ops import unary_union
from shapely import prepared
# The flattened drawing; set by the caller (with outlines.G).
G=None
def faces(k,lays,tol=0.03):
    U=unary_union(lines(k,lays)).buffer(tol)
    gs=[U] if U.geom_type=="Polygon" else list(U.geoms)
    out=[]
    for g in gs:
        for h in g.interiors:
            out.append(Polygon(h).buffer(tol))
    return out
def plant_points(k):
    return [(s[1],s[2]) for s in G[k].get("JARDINERIA",[]) if s[0] in("L","C")]
def beds(k,lays,xmax=33,minpts=20,maxarea=150):
    pts=[p for p in plant_points(k) if p[0]<xmax]
    res=[]
    for f in faces(k,lays):
        if f.area>maxarea or f.area<0.3: continue
        pf=prepared.prep(f)
        n=sum(1 for q in pts if pf.contains(Point(q)))
        if n>=minpts: res.append((f,n))
    return res
if __name__=="__main__":
    G=outlines.G=json.load(open(sys.argv[1]))
    for k,l in [("250819 REF PCAST",["PROYECCION","SECCION"]),("250819 REF P1",["PROYECCION","SECCION","CARPINTERIA"]),("250819 REF PB",["PROYECCION","SECCION","CARPINTERIA"])]:
        b=beds(k,l)
        print(k,[(round(f.area,1),n,[round(v,1) for v in f.bounds]) for f,n in b])
