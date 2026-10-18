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
cargo run -p archisim-cli -- importar out/demo.ifc out/importado/demo.archisim   # IFC → proyecto (informe de lo importado)
ARCHISIM_TEST_OUT=$PWD/out/app cargo test -p archisim-app -- drawn_project gpu_tests   # casa dibujada con la app (clics simulados), tipos editados y out/app/vista3d.png
python3 scripts/validate_ifc.py out/app/sesion.ifc
python3 scripts/validate_ifc.py out/app/tipos.ifc   # la misma casa con tipos y materiales modificados
```

La integración continua usa el Rust estable más reciente, que puede traer avisos nuevos de clippy; si clippy falla solo allí, los avisos salen como anotaciones del job (`gh api repos/stronkel/archisim/check-runs/<id>/annotations`).

Si cambias la aplicación, además de las pruebas (que la manejan con clics y teclas simulados, sin pantalla), ábrela de verdad (`cargo run --release -p archisim-app -- out/demo.archisim`; en un servidor sin pantalla, con Xvfb y capturas) y mira el resultado.

Si cambias la exportación IFC, la validación con IfcOpenShell debe pasar sin errores. Si cambias el modelo, el formato `.archisim` debe guardarlo entero: añade el campo al esquema y, si cambia la estructura del fichero, sube `CURRENT_VERSION` y escribe la migración en `archisim-file` (los proyectos antiguos tienen que seguir abriéndose). Si cambias la planta, mira el SVG resultante antes de entregarlo.

## Estructura

- `crates/archisim-family` — lenguaje de familias (recetas `.asfam`): lector, fórmulas, piezas comunes, evaluación. Sin dependencias; no conoce el modelo. Descrito en `docs/FAMILIAS.md`.
- `crates/archisim-core` — documento, elementos, tipos, familias, transacciones. Los muros pueden tener restricción superior (`WallTop`): su `height` la calcula el modelo desde las plantas y la mantiene al cambiar cotas (`refresh_wall_heights`); quien lea `height` no tiene que saber de restricciones. Solo depende de `archisim-family`. Las familias de sistema (puertas y ventanas) están en `crates/archisim-core/families/*.asfam`; los tipos de puerta y ventana son familia + valores + marca, y sus campos `kind`, `width`… los calcula el modelo desde la familia (no los escribas a mano). Una prueba de `archisim-geom` compara el 3D de las familias de sistema con el cálculo anterior: si cambias una receta de sistema, debe seguir pasando o explicarse.
- `crates/archisim-geom` — geometría 2D, contornos de muros (uniones en L a inglete y en T con prioridad por capa en `joins`) y sólidos 3D (`solid`): las mismas formas que el IFC, una piel cerrada por elemento, sin caras ocultas de contacto.
- `crates/archisim-ifc` — escritor IFC4 STEP; lector STEP (`step_read`) e importador IFC2X3/IFC4 (`import`). Ficheros de prueba de otro programa en `tests/fixtures/`, generados con `scripts/make_ifc_fixtures.py` (IfcOpenShell); no subas IFC de proyectos reales.
- `crates/archisim-plan` — planta en SVG; cotas (`annotate`) y rotulación técnica de trazo (`lettering`), sin depender de fuentes. Las puertas y ventanas se dibujan con el símbolo de su familia para la escala (`plan 1:50`, `plan 1:100` en la receta); no programes símbolos aquí. Una prueba compara, rasterizando, el dibujo de las familias de sistema con el anterior (`legacy_symbols`, solo en pruebas).
- `crates/archisim-file` — proyecto `.archisim` (JSON versionado). Formato descrito en `docs/FORMATO_ARCHISIM.md`.
- `crates/archisim-library` — plantilla de inicio: materiales y tipos con los que empieza un proyecto nuevo (y la demo).
- `crates/archisim-cli` — comandos de línea.
- `crates/archisim-app` — aplicación de escritorio (egui/eframe sobre wgpu). La vista de planta rasteriza el mismo SVG de `archisim-plan` para la zona visible, a la escala de la pestaña: no dupliques reglas de dibujo en la app. La ventana (descrita en `docs/FUNDACIONAL.md` §4, «Interfaz») se reparte en `app/`: `chrome` (barras superior e inferior, pastilla de teclas y avisos sobre el dibujo), `toolbox` (caja de herramientas por secciones), `tabs` (pestañas de vistas: la activa vive en `view`, `plan_scale`, `cut_plane` y `ed.level`; las demás guardan su copia), `navigator` (a la derecha), `properties` (a la izquierda, junto a la caja) con las piezas de `props` (selector de tipo con galería, grupos, filas) y los dibujos de tipo de `thumbs` (los de `type_preview`, guardados mientras se ven), `plan_ui` (vista de planta), `theme` (paletas clara y oscura, fuentes IBM Plex de `assets/fonts`), `icons` (iconos dibujados como líneas) y `widgets`. Los colores salen siempre de `theme::Palette`; el dibujo de la planta es siempre papel blanco. La lógica (vista, forzados, herramienta muro, selección, edición, arrastres de mover/copiar/estirar en `drag`, cámara 3D, selección por rayo) está separada de la interfaz y tiene pruebas propias. Las cotas temporales (`temp_dims`: qué se mide y qué arrastre hace cada valor tecleado) y el giro de puertas y ventanas (`flip`) no saben nada de la pantalla; su dibujo y sus clics están en `app/temp_ui`. Los muros de otras plantas que muestra una planta (cortados por su plano de corte, o el calco de la planta inferior) los decide `archisim_plan::other_level_walls`: la app los usa también para forzar. El diálogo «Tipos y materiales» (`app/types_ui`) edita a través de `Editor` (`editor/types`, nombres únicos por lista) y sus vistas previas (`type_preview`) son plantas de un modelo mínimo dibujadas por `archisim-plan` a 1:20: no dibujes multicapas a mano en la app. El bucle de eventos se crea en `main.rs` (no en eframe) para añadir en macOS, al delegado de aplicación de winit, el método que recibe los ficheros abiertos desde el Finder (`open_files`; winit no admite otro delegado); los ficheros soltados en la ventana llegan por el mismo camino. La aplicación de Mac (`.app` universal en un `.dmg`, con icono y tipos de documento) la monta `scripts/macos_bundle.sh` en la integración continua; el icono sale de `scripts/make_icon.py`. La vista 3D (`view3d`) dibuja con wgpu en una textura propia; su prueba de dibujo usa cualquier adaptador gráfico disponible (vale uno por software) y se salta si no hay ninguno.

Al añadir un crate, respeta la dirección de dependencias descrita en `docs/FUNDACIONAL.md` §3.

## Prioridades actuales

MVP (fase 1) terminado en v0.8; v0.9 añade mover/copiar/estirar con el ratón, varita mágica y edición de vértices de forjado; v0.10, cotas, cotas de nivel y gestión de plantas (formato `.archisim` versión 2); v0.11, uniones en T con prioridad de intersección por capa; v0.12, editor de tipos y materiales (sin cambios de formato). La lista de lo que sigue, con las decisiones pendientes de Antonio, está en `docs/HOJA_DE_RUTA.md`. v0.13, aplicación de Mac. D2 decidida (18/10/2026): familias como recetas de texto, diseño en `docs/FAMILIAS.md`. v0.14: bloque A de familias (lenguaje, familias de sistema, tipos con marca, 3D desde las recetas, formato `.archisim` versión 3). v0.15: bloque B (símbolos de planta por escala desde las recetas, ventana practicable de 1 a 4 hojas con funcionamiento por hoja, diálogo de tipos generado desde la receta, planta 1:100). v0.16: interfaz rediseñada (caja de herramientas por secciones, pestañas de vistas con su escala, navegador, propiedades estilo Revit, barra inferior con escala y forzados, tema claro y oscuro; sin cambios de formato ni de IFC). v0.17: restricción superior de muros (hasta una planta con desfase, siguiendo los cambios de cota; formato `.archisim` versión 4), decidida por Antonio antes del bloque C. v0.18: ayudas al dibujo pedidas por Antonio (cotas temporales azules editables, forzado a puntos medios de caras y aristas, muros ortogonales por defecto con línea de rastreo, Espacio y flechas para girar puertas y ventanas, cambiar de planta muros y forjados, muros de otras plantas cortados por el plano de corte y calco de la planta inferior; sin cambios de formato ni de IFC). Siguiente: bloque C (IFC desde las familias). Propuesta abierta para Antonio: disciplinas (estructura, urbanismo) y mediciones con bancos de precios en `docs/DISCIPLINAS.md`.
