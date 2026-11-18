import sys, json, math, ezdxf
from ezdxf import path as zpath
ERR=[0]
def collect(doc, blkname):
    blk=doc.blocks.get(blkname); out={}
    def add(lay, item): out.setdefault(lay,[]).append(item)
    def one(e, parent_layer, depth):
        lay=e.dxf.layer.split("$0$")[-1].replace("01-BASICO-","")
        if lay=="0" and parent_layer: lay=parent_layer
        t=e.dxftype()
        if t=="INSERT":
            walk(list(e.virtual_entities()), lay, depth+1)
        elif t=="LINE":
            a,b=e.dxf.start,e.dxf.end; add(lay,("L",a.x,a.y,b.x,b.y))
        elif t=="ARC":
            c=e.ocs().to_wcs(e.dxf.center); p,q=e.start_point,e.end_point
            add(lay,("A",c.x,c.y,e.dxf.radius,p.x,p.y,q.x,q.y))
            add(lay,("AP",[(v.x,v.y) for v in e.flattening(0.01)]))
        elif t=="CIRCLE":
            c=e.ocs().to_wcs(e.dxf.center); add(lay,("C",c.x,c.y,e.dxf.radius))
        elif t in ("LWPOLYLINE","POLYLINE"):
            if t=="LWPOLYLINE" and e.closed:
                pts=[e.ocs().to_wcs((x,y,0)) for x,y,*_ in e.get_points()]
                add(lay,("P",[(q.x,q.y) for q in pts]))
            for s in e.virtual_entities(): one(s, lay, depth)
        elif t in ("TEXT","MTEXT"):
            add(lay,("T",e.dxf.insert.x,e.dxf.insert.y,e.dxf.text if t=="TEXT" else e.plain_text()))
        elif t in ("SPLINE","ELLIPSE"):
            pts=list(zpath.make_path(e).flattening(0.02))
            if t=="ELLIPSE" and len(pts)>2:
                # keep as arc if circular
                ra=e.dxf.major_axis.magnitude; rb=ra*e.dxf.ratio
                if abs(ra-rb)<1e-3:
                    c=e.dxf.center; add(lay,("A",c.x,c.y,ra,pts[0].x,pts[0].y,pts[-1].x,pts[-1].y))
            for p,q in zip(pts,pts[1:]): add(lay,("L",p.x,p.y,q.x,q.y))
    def walk(ents, parent_layer=None, depth=0):
        for e in ents:
            try: one(e,parent_layer,depth)
            except Exception as ex: ERR[0]+=1
    walk(blk); return out
doc=ezdxf.readfile(sys.argv[1]); res={}
for n in ["250819 REF PB","250819 REF P1","250819 REF PCAST","250819 REF CUBIERTA"]:
    res[n]=collect(doc,n); print(n,{k:len(v) for k,v in res[n].items() if k in("SECCION","CARPINTERIA","PILARES")})
print("errors",ERR[0])
msp=doc.modelspace()
res["rooms"]=[[(x,y) for x,y,*_ in p.get_points()] for p in msp.query('LWPOLYLINE[layer=="01-BASICO-SUPERFICIES"]')]
res["room_texts"]=[(t.dxf.insert.x,t.dxf.insert.y,t.plain_text()) for t in msp.query('MTEXT[layer=="01-BASICO-TEXTOS"]')]
json.dump(res,open(sys.argv[2],"w"))
