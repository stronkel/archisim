# Instrucciones para Claude Code

Lee `docs/FUNDACIONAL.md` antes de cualquier trabajo: define visión, principios y alcance.

## Quién es quién

- Antonio es arquitecto y experto BIM (ArchiCAD desde la universidad, conoce bien la metodología Revit). **No programa en Rust.** Habla con él en español, en términos de arquitectura, no de implementación. Cuando necesites que pruebe algo, dile exactamente qué comando ejecutar y qué debe mirar.
- Tú escribes todo el código. Antonio decide qué está bien desde el punto de vista profesional.

## Convenciones

- Código, identificadores y comentarios en inglés (el proyecto será open source). Documentación para Antonio en español.
- Unidades internas: metros, `f64`. Ángulos en radianes.
- Capas de muro de exterior a interior; la cara exterior a la izquierda del eje (inicio → fin).
- Cada elemento tiene un `GlobalId` IFC estable: nunca lo regeneres al exportar.
- Sala limpia: nada de código, ficheros o contenido de Autodesk o Graphisoft.

## Antes de dar algo por terminado

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p archisim-cli -- demo out/   # genera out/demo.ifc y out/demo.svg
python3 scripts/validate_ifc.py out/demo.ifc   # requiere: pip install ifcopenshell
```

Si cambias la exportación IFC, la validación con IfcOpenShell debe pasar sin errores. Si cambias la planta, mira el SVG resultante antes de entregarlo.

## Estructura

- `crates/archisim-core` — documento, elementos, tipos, transacciones. Sin dependencias internas.
- `crates/archisim-geom` — geometría 2D y contornos de muros.
- `crates/archisim-ifc` — escritor IFC4 STEP.
- `crates/archisim-plan` — planta en SVG.
- `crates/archisim-cli` — comandos de línea.

Al añadir un crate, respeta la dirección de dependencias descrita en `docs/FUNDACIONAL.md` §3.

## Prioridades actuales

Siguiente en la lista del MVP (`docs/FUNDACIONAL.md` §5): forjados, después huecos de puertas y ventanas, después formato de proyecto propio.
