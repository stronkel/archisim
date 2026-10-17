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
- Puertas y ventanas: posición del centro sobre la línea de referencia desde el inicio del muro; antepecho desde el nivel; mano según UNE-EN 12519 (vista desde el lado hacia el que abre).
- Cada elemento tiene un `GlobalId` IFC estable: nunca lo regeneres al exportar.
- Sala limpia: nada de código, ficheros o contenido de Autodesk o Graphisoft.

## Antes de dar algo por terminado

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p archisim-cli -- demo out/   # genera out/demo.archisim, out/demo.ifc y una planta SVG por nivel
python3 scripts/validate_ifc.py out/demo.ifc   # requiere: pip install ifcopenshell
cargo run -p archisim-cli -- exportar out/demo.archisim out/reabierto   # ida y vuelta: IFC idéntico salvo la fecha
ARCHISIM_TEST_OUT=$PWD/out/app cargo test -p archisim-app -- drawn_project gpu_tests   # casa dibujada con la app (clics simulados) y out/app/vista3d.png
python3 scripts/validate_ifc.py out/app/sesion.ifc
```

Si cambias la aplicación, además de las pruebas (que la manejan con clics y teclas simulados, sin pantalla), ábrela de verdad (`cargo run --release -p archisim-app -- out/demo.archisim`; en un servidor sin pantalla, con Xvfb y capturas) y mira el resultado.

Si cambias la exportación IFC, la validación con IfcOpenShell debe pasar sin errores. Si cambias el modelo, el formato `.archisim` debe guardarlo entero: añade el campo al esquema y, si cambia la estructura del fichero, sube `CURRENT_VERSION` y escribe la migración en `archisim-file` (los proyectos antiguos tienen que seguir abriéndose). Si cambias la planta, mira el SVG resultante antes de entregarlo.

## Estructura

- `crates/archisim-core` — documento, elementos, tipos, transacciones. Sin dependencias internas.
- `crates/archisim-geom` — geometría 2D, contornos de muros y sólidos 3D (`solid`): las mismas formas que el IFC, una piel cerrada por elemento, sin caras ocultas de contacto.
- `crates/archisim-ifc` — escritor IFC4 STEP.
- `crates/archisim-plan` — planta en SVG.
- `crates/archisim-file` — proyecto `.archisim` (JSON versionado). Formato descrito en `docs/FORMATO_ARCHISIM.md`.
- `crates/archisim-library` — plantilla de inicio: materiales y tipos con los que empieza un proyecto nuevo (y la demo).
- `crates/archisim-cli` — comandos de línea.
- `crates/archisim-app` — aplicación de escritorio (egui/eframe sobre wgpu). La vista de planta rasteriza el mismo SVG 1:50 de `archisim-plan` para la zona visible: no dupliques reglas de dibujo en la app. La lógica (vista, forzados, herramienta muro, selección, edición, cámara 3D, selección por rayo) está separada de la interfaz y tiene pruebas propias. La vista 3D (`view3d`) dibuja con wgpu en una textura propia; su prueba de dibujo usa cualquier adaptador gráfico disponible (vale uno por software) y se salta si no hay ninguno.

Al añadir un crate, respeta la dirección de dependencias descrita en `docs/FUNDACIONAL.md` §3.

## Prioridades actuales

Siguiente en la lista del MVP (`docs/FUNDACIONAL.md` §5): importación IFC básica. Pendiente en la app: forjado por varita mágica, mover/estirar con el ratón y cotas (Antonio decide el orden; ver la bitácora).
