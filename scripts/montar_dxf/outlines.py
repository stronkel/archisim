import json,sys
from shapely.geometry import LineString, Polygon
from shapely.ops import polygonize, unary_union
# The flattened drawing (leer_dxf.py's output); set by the caller.
G=None
def lines(k,lays):
    out=[]
    for l in lays:
        for s in G[k].get(l,[]):
            if s[0]=="L" and (s[1],s[2])!=(s[3],s[4]): out.append(LineString([(s[1],s[2]),(s[3],s[4])]))
            elif s[0]=="AP" and len(s[1])>1: out.append(LineString(s[1]))
    return out
def polys(k,lays):
    ps=list(polygonize(unary_union(lines(k,lays)))); ps.sort(key=lambda p:-p.area); return ps
if __name__=="__main__":
    G=json.load(open(sys.argv[1]))
    for k in ["250819 REF P1","250819 REF PCAST","250819 REF PB"]:
        ps=polys(k,["PROYECCION"])
        print(k,len(ps),[(round(p.area,1),[round(v,1) for v in p.bounds]) for p in ps[:6]])
