# ArchiSim

Herramienta BIM de autoría, multiplataforma y escrita en Rust: la facilidad de ArchiCAD con la organización estándar de Revit, sobre IFC 4.3 como modelo de datos. Nombre provisional.

Estado: **v0.6, fase 1 (MVP).** Aplicación con vista de planta, herramientas muro, forjado, puerta y ventana, y selección con propiedades. Debajo, un núcleo que modela niveles, materiales, tipos de muro y de forjado multicapa, muros, forjados, puertas y ventanas, los guarda como proyecto `.archisim` y los exporta a IFC4 y a plantas 1:50 en SVG.

Lee [`docs/FUNDACIONAL.md`](docs/FUNDACIONAL.md) para la visión, la arquitectura y el plan.

## Probarlo en Windows sin instalar nada

Cada cambio en GitHub genera la aplicación lista para usar. En la pestaña **Actions** del repositorio, abre la última ejecución en verde, baja hasta **Artifacts** y descarga **ArchiSim-Windows**. Descomprime el zip y abre `ArchiSim.exe` (la primera vez Windows avisa porque el programa no está firmado: *Más información → Ejecutar de todas formas*). La vivienda de prueba `demo.archisim` viene en la misma carpeta: ábrela con *Archivo → Abrir*, o arrastra el fichero sobre `ArchiSim.exe`.

## Probarlo compilando

1. Instala Rust (una sola vez): <https://rustup.rs>. En Windows acepta instalar las herramientas de compilación de Visual Studio que te propone.
2. En una terminal, dentro de esta carpeta:

   ```bash
   cargo run -p archisim-cli -- demo out
   ```

   La primera vez tarda un par de minutos en compilar. Genera:
   - `out/demo.archisim` — el proyecto, en texto legible ([formato](docs/FORMATO_ARCHISIM.md)).
   - `out/demo.ifc` — vivienda de prueba de 12 × 9 m con fachada de doble hoja, tabiquería, solera, cubierta plana, cuatro puertas y cinco ventanas.
   - `out/demo.svg` — planta baja a escala 1:50 (ábrela con el navegador).
   - `out/demo-cubierta.svg` — planta de cubierta a escala 1:50.

3. La aplicación, abriendo la vivienda de prueba:

   ```bash
   cargo run --release -p archisim-app -- out/demo.archisim
   ```

   Sin el nombre del proyecto empieza uno nuevo con la plantilla. `--release` tarda más en compilar la primera vez, pero la vista va mucho más fluida. Ratón y teclado: menú *Ayuda → Ratón y teclado*.

4. Para exportar un proyecto `.archisim` desde la línea de comandos (por ejemplo, después de editarlo a mano):

   ```bash
   cargo run -p archisim-cli -- exportar out/demo.archisim
   ```

   Genera `demo.ifc` y las plantas junto al proyecto.

### Qué mirar al abrir el IFC en ArchiCAD o Revit

- ¿Los muros aparecen como muros (no como objetos genéricos)?
- ¿Se reconocen los tipos «Fachada doble hoja 26,5» y «Tabique LHD 10»?
- ¿Las capas salen en orden correcto, con el monocapa hacia fuera y el yeso hacia dentro?
- ¿Las esquinas cierran limpias?
- ¿Aparecen las propiedades `IsExternal`, `LoadBearing` y las cantidades (longitud, superficie, volumen)?
- ¿La solera y la cubierta entran como forjados (losas) con sus tipos, capas en orden (acabado arriba) y cantidades?
- ¿Las puertas y ventanas entran como puertas y ventanas (no como objetos genéricos), dentro de su hueco, con su tipo y sus materiales, y el muro aparece perforado?

Apunta lo que veas (con capturas) en un *issue* de GitHub.

## Para desarrollo

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/validate_ifc.py out/demo.ifc   # pip install ifcopenshell pytest
```

Las instrucciones para Claude Code están en [`CLAUDE.md`](CLAUDE.md).

## Licencia

Pendiente de decidir (ver `docs/FUNDACIONAL.md` §8). Mientras tanto, todos los derechos reservados.
