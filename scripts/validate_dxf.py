#!/usr/bin/env python3
"""Comprueba DXF de ArchiSim con ezdxf: que se leen sin reparar nada, que la
auditoría no encuentra errores, y cuenta entidades por capa.

Uso: python3 scripts/validate_dxf.py out/demo.dxf [más.dxf…]
Requiere: pip install ezdxf
"""

import sys
from collections import Counter

try:
    import ezdxf
    from ezdxf import recover
except ImportError:
    sys.exit("Falta ezdxf: pip install ezdxf")

ok = True
for path in sys.argv[1:]:
    doc, auditor = recover.readfile(path)
    audit = doc.audit()
    problems = len(auditor.errors) + len(auditor.fixes) + len(audit.errors) + len(audit.fixes)
    counts = Counter(e.dxftype() for e in doc.modelspace())
    layers = Counter(e.dxf.layer for e in doc.modelspace())
    print(f"{path}: {doc.dxfversion}, {sum(counts.values())} entidades "
          f"({', '.join(f'{n} {k}' for k, n in counts.most_common())}), {len(layers)} capas")
    for e in list(auditor.errors)[:10] + list(audit.errors)[:10]:
        print("  ERROR:", e.message)
    for e in list(auditor.fixes)[:10] + list(audit.fixes)[:10]:
        print("  REPARADO:", e.message)
    if problems:
        ok = False
print("OK: los DXF son válidos." if ok else "HAY PROBLEMAS en algún DXF.")
sys.exit(0 if ok else 1)
