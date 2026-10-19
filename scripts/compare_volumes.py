"""Compares the volume ArchiSim's coordination mesher gives each element of
an IFC file with IfcOpenShell's geometry engine (the reference).

    python3 scripts/compare_volumes.py fichero.ifc   (requires ifcopenshell)
"""
import os
import subprocess
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.shape

path = os.path.abspath(sys.argv[1])
out = subprocess.run(
    ["cargo", "test", "-q", "-p", "archisim-coord", "print_volumes", "--", "--nocapture"],
    env={**os.environ, "MESH_VOLUMES": path},
    capture_output=True,
    text=True,
).stdout
ours = {}
for line in out.splitlines():
    if line.startswith("VOL "):
        _, gid, cls, vol, approx = line.split()
        ours[gid] = (cls, float(vol), approx == "1")

f = ifcopenshell.open(path)
settings = ifcopenshell.geom.settings()
settings.set("use-world-coords", True)
# ArchiSim keeps openings apart (they are tested as voids in clashes).
settings.set("disable-opening-subtractions", True)
worst = 0.0
bad = 0
for gid, (cls, vol, approx) in sorted(ours.items()):
    el = f.by_guid(gid)
    try:
        shape = ifcopenshell.geom.create_shape(settings, el)
        ref = ifcopenshell.util.shape.get_volume(shape.geometry)
    except Exception as e:  # noqa: BLE001
        print(f"  ? {cls} {gid}: IfcOpenShell no lo procesa ({e})")
        continue
    diff = abs(vol - ref) / max(ref, 1e-9)
    worst = max(worst, diff) if not approx else worst
    mark = "≈" if approx else ("✔" if diff < 0.01 else "✘")
    if mark != "✔":
        bad += mark == "✘"
        print(f"  {mark} {cls} {gid}: ArchiSim {vol:.6f} m³, IfcOpenShell {ref:.6f} m³")
print(f"{len(ours)} elementos; peor diferencia en los exactos: {worst:.4%}; distintos: {bad}")
