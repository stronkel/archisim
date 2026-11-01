#!/usr/bin/env python3
"""Invented cartography for the tests and `archisim cartografia` (H7):
a block plan of a made-up town in UTM 30N (ETRS89, near 37.39 N 5.99 W)
as a cadastre GML (building parts with floors and parcels), an
OpenStreetMap file (streets and trees) and an ESRI ASCII terrain grid.
Nothing here comes from the real cadastre or OpenStreetMap."""
import math, os, sys

out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    os.path.dirname(__file__), "..", "crates", "archisim-carto", "tests", "fixtures")
os.makedirs(out, exist_ok=True)
E0, N0 = 235_000.0, 4_142_000.0      # the middle of the town, UTM 30N
BLOCK, STREET = 40.0, 12.0

# --- Cadastre: 4 × 4 blocks, each split in four parcels with a building.
feats = []
srs = "http://www.opengis.net/def/crs/EPSG/0/25830"
def ring(pts):
    pts = pts + [pts[0]]
    return " ".join(f"{x:.2f} {y:.2f}" for x, y in pts)
k = 0
for i in range(-2, 2):
    for j in range(-2, 2):
        x0 = E0 + i * (BLOCK + STREET) + STREET / 2
        y0 = N0 + j * (BLOCK + STREET) + STREET / 2
        for a in range(2):
            for b in range(2):
                k += 1
                px, py = x0 + a * BLOCK / 2, y0 + b * BLOCK / 2
                ref = f"{9000000 + k:07d}TG3490S"
                parcel = [(px, py), (px + BLOCK / 2, py), (px + BLOCK / 2, py + BLOCK / 2), (px, py + BLOCK / 2)]
                # The building fills the street side of the parcel.
                bx0 = px + (0 if a == 0 else 4)
                by0 = py + (0 if b == 0 else 6)
                w, d = BLOCK / 2 - 4, BLOCK / 2 - 6
                building = [(bx0, by0), (bx0 + w, by0), (bx0 + w, by0 + d), (bx0, by0 + d)]
                floors = 1 + (k * 7) % 5
                feats.append(f"""<gml:featureMember>
<cp:CadastralParcel gml:id="ES.SDGC.CP.{ref}">
<cp:geometry><gml:MultiSurface srsName="{srs}"><gml:surfaceMember><gml:Surface><gml:patches><gml:PolygonPatch><gml:exterior><gml:LinearRing>
<gml:posList srsDimension="2">{ring(parcel)}</gml:posList></gml:LinearRing></gml:exterior></gml:PolygonPatch></gml:patches></gml:Surface></gml:surfaceMember></gml:MultiSurface></cp:geometry>
<cp:nationalCadastralReference>{ref}</cp:nationalCadastralReference>
</cp:CadastralParcel>
</gml:featureMember>
<gml:featureMember>
<bu-ext2d:BuildingPart gml:id="ES.SDGC.BU.{ref}_part1">
<bu-core2d:localId>{ref}_part1</bu-core2d:localId>
<bu-ext2d:geometry><gml:Surface srsName="{srs}"><gml:patches><gml:PolygonPatch><gml:exterior><gml:LinearRing>
<gml:posList srsDimension="2" count="5">{ring(building)}</gml:posList></gml:LinearRing></gml:exterior></gml:PolygonPatch></gml:patches></gml:Surface></bu-ext2d:geometry>
<bu-ext2d:numberOfFloorsAboveGround>{floors}</bu-ext2d:numberOfFloorsAboveGround>
<bu-ext2d:numberOfFloorsBelowGround>0</bu-ext2d:numberOfFloorsBelowGround>
</bu-ext2d:BuildingPart>
</gml:featureMember>""")
with open(os.path.join(out, "pueblo-catastro.gml"), "w") as f:
    f.write('<?xml version="1.0" encoding="UTF-8"?>\n<gml:FeatureCollection xmlns:gml="http://www.opengis.net/gml/3.2" '
            'xmlns:cp="urn:x-inspire:specification:gmlas:CadastralParcels:3.0" '
            'xmlns:bu-ext2d="http://inspire.jrc.ec.europa.eu/schemas/bu-ext2d/2.0" '
            'xmlns:bu-core2d="http://inspire.jrc.ec.europa.eu/schemas/bu-core2d/2.0">\n')
    f.write("\n".join(feats))
    f.write("\n</gml:FeatureCollection>\n")

# --- OpenStreetMap: the streets between the blocks and trees.
def utm_to_ll(e, n, zone=30):
    a = 6378137.0; f = 1 / 298.257222101; k0 = 0.9996
    e2 = f * (2 - f); ep2 = e2 / (1 - e2)
    x = e - 500000.0; m = n / k0
    mu = m / (a * (1 - e2 / 4 - 3 * e2**2 / 64 - 5 * e2**3 / 256))
    e1 = (1 - math.sqrt(1 - e2)) / (1 + math.sqrt(1 - e2))
    p1 = (mu + (3 * e1 / 2 - 27 * e1**3 / 32) * math.sin(2 * mu)
          + (21 * e1**2 / 16 - 55 * e1**4 / 32) * math.sin(4 * mu)
          + (151 * e1**3 / 96) * math.sin(6 * mu) + (1097 * e1**4 / 512) * math.sin(8 * mu))
    s, c = math.sin(p1), math.cos(p1); t = s / c
    n1 = a / math.sqrt(1 - e2 * s * s); t1 = t * t; c1 = ep2 * c * c
    r1 = a * (1 - e2) / (1 - e2 * s * s) ** 1.5; d = x / (n1 * k0)
    lat = p1 - (n1 * t / r1) * (d * d / 2 - (5 + 3 * t1 + 10 * c1 - 4 * c1 * c1 - 9 * ep2) * d**4 / 24
                                + (61 + 90 * t1 + 298 * c1 + 45 * t1 * t1 - 252 * ep2 - 3 * c1 * c1) * d**6 / 720)
    lon0 = math.radians(zone * 6 - 183)
    lon = lon0 + (d - (1 + 2 * t1 + c1) * d**3 / 6
                  + (5 - 2 * c1 + 28 * t1 - 3 * c1 * c1 + 8 * ep2 + 24 * t1 * t1) * d**5 / 120) / c
    return math.degrees(lat), math.degrees(lon)

nodes, ways = [], []
nid = [0]
def node(e, n, tags=()):
    nid[0] += 1
    lat, lon = utm_to_ll(e, n)
    t = "".join(f'<tag k="{k}" v="{v}"/>' for k, v in tags)
    nodes.append(f'<node id="{nid[0]}" lat="{lat:.7f}" lon="{lon:.7f}">{t}</node>' if t
                 else f'<node id="{nid[0]}" lat="{lat:.7f}" lon="{lon:.7f}"/>')
    return nid[0]
span = 2 * (BLOCK + STREET) + 30
names = ["Calle Real", "Calle Nueva", "Avenida del Río", "Calle de la Iglesia", "Calle del Agua"]
classes = ["residential", "residential", "secondary", "living_street", "residential"]
w = 100
for idx, i in enumerate(range(-2, 3)):
    off = i * (BLOCK + STREET)
    for vertical in (False, True):
        a = node(E0 + off, N0 - span) if vertical else node(E0 - span, N0 + off)
        b = node(E0 + off, N0 + span) if vertical else node(E0 + span, N0 + off)
        w += 1
        name = names[idx] if not vertical else f"Calle {idx + 1}"
        cls = classes[idx] if not vertical else "residential"
        oneway = '<tag k="oneway" v="yes"/>' if vertical and idx % 2 else ""
        ways.append(f'<way id="{w}"><nd ref="{a}"/><nd ref="{b}"/><tag k="highway" v="{cls}"/>'
                    f'<tag k="name" v="{name}"/>{oneway}</way>')
# A path that is not a street, a row of orange trees and single trees.
a, b = node(E0 - 20, N0 - 20), node(E0 + 20, N0 + 20)
w += 1; ways.append(f'<way id="{w}"><nd ref="{a}"/><nd ref="{b}"/><tag k="highway" v="footway"/></way>')
a, b = node(E0 - 60, N0 + 3.5), node(E0 + 60, N0 + 3.5)
w += 1; ways.append(f'<way id="{w}"><nd ref="{a}"/><nd ref="{b}"/><tag k="natural" v="tree_row"/><tag k="genus" v="Citrus"/></way>')
for (de, dn, genus) in [(26, 30, "Jacaranda"), (-78, -26, "Platanus"), (78, 26, "Phoenix")]:
    node(E0 + de, N0 + dn, [("natural", "tree"), ("genus", genus)])
with open(os.path.join(out, "pueblo.osm"), "w") as f:
    f.write('<?xml version="1.0" encoding="UTF-8"?>\n<osm version="0.6" generator="make_carto_fixtures.py">\n')
    f.write("\n".join(nodes + ways))
    f.write("\n</osm>\n")

# --- Terrain: 5 m cells, a gentle slope down to the south-east and a hill.
cell, n = 5.0, 64
x0, y0 = E0 - n * cell / 2, N0 - n * cell / 2
rows = []
for r in range(n):
    yc = y0 + (n - 1 - r + 0.5) * cell
    row = []
    for c in range(n):
        xc = x0 + (c + 0.5) * cell
        z = 12.0 - 0.02 * (xc - E0) + 0.03 * (yc - N0) + 2.5 * math.exp(-(((xc - E0 + 90) ** 2 + (yc - N0 - 90) ** 2) / 1800))
        row.append(f"{z:.2f}")
    rows.append(" ".join(row))
with open(os.path.join(out, "pueblo-mdt05.asc"), "w") as f:
    f.write(f"ncols {n}\nnrows {n}\nxllcorner {x0:.1f}\nyllcorner {y0:.1f}\ncellsize {cell}\nNODATA_value -9999\n")
    f.write("\n".join(rows) + "\n")
print("escritos en", out)
