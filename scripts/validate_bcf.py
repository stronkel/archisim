"""Validates a BCF 3.0 file written by ArchiSim against buildingSMART's XML
schemas (https://github.com/buildingSMART/BCF-XML, folder Schemas).

    python3 scripts/validate_bcf.py out/ejemplo.bcf <BCF-XML>/Schemas
    (requires: pip install xmlschema)
"""
import sys
import zipfile

import xmlschema

bcf, schemas = sys.argv[1], sys.argv[2]
by_kind = {
    "bcf.version": "version.xsd",
    "extensions.xml": "extensions.xsd",
    "project.bcfp": "project.xsd",
    "documents.xml": "documents.xsd",
    ".bcf": "markup.xsd",
    ".bcfv": "visinfo.xsd",
}
cache = {}
errors = 0
with zipfile.ZipFile(bcf) as z:
    for name in z.namelist():
        xsd = next((x for k, x in by_kind.items() if name.endswith(k)), None)
        if xsd is None:
            continue
        schema = cache.setdefault(xsd, xmlschema.XMLSchema(f"{schemas}/{xsd}"))
        try:
            schema.validate(z.read(name).decode("utf-8"))
            print(f"  ✔ {name}")
        except xmlschema.XMLSchemaException as e:
            errors += 1
            print(f"  ✘ {name}: {str(e).splitlines()[0]}")
            print("    " + "\n    ".join(str(e).splitlines()[1:6]))
sys.exit(1 if errors else 0)
