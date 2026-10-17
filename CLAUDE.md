# Instrucciones para Claude Code

Lee `docs/FUNDACIONAL.md` antes de cualquier trabajo: define visión, principios y alcance. Lee después `docs/BITACORA.md` para saber en qué punto está el proyecto y qué feedback hay pendiente.

## Al cerrar cada sesión de trabajo

1. Añade una entrada al principio de `docs/BITACORA.md`: qué se hizo, qué quedó pendiente y qué debe probar Antonio.
2. Actualiza la tabla del MVP en `docs/FUNDACIONAL.md` §5 si cambió algún estado.
3. Commit en español, en imperativo y con el bloque al principio (p. ej. `Forjados: exportar IfcSlab con capas`), y push a `main`. Si el cambio es grande o arriesgado, usa una rama `bloque/<nombre>` y abre un pull request para que Antonio lo revise.

## Quién es quién

- Antonio es arquitecto y experto BIM (ArchiCAD desde la universidad, conoce bien la metodología Revit). **No programa en Rust.** Habla con él en español, en términos de arquitectura, no de implementación. Cuando necesites que pruebe algo, dile exactamente qué comando ejecutar y qué debe mirar.
- Tú escribes todo el código. Antonio decide qué está bien desde el punto de vista profesional.

## Convenciones

- Código, identificadores y comentarios en inglés (el proyecto será open source). Documentación para Antonio en español.
- Unidades internas: metros, `f64`. Ángulos en radianes.
- Capas de muro de exterior a interior; la cara exterior a la izquierda del eje (inicio → fin).
- Capas de forjado de arriba abajo; la referencia del forjado es su cara superior.
- Cada elemento tiene un `GlobalId` IFC estable: nunca lo regeneres al exportar.
- Sala limpia: nada de código, ficheros o contenido de Autodesk o Graphisoft.

## Antes de dar algo por terminado

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p archisim-cli -- demo out/   # genera out/demo.ifc y una planta SVG por nivel
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

Siguiente en la lista del MVP (`docs/FUNDACIONAL.md` §5): huecos de puertas y ventanas, después formato de proyecto propio.
