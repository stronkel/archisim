# ArchiSim

Herramienta BIM de autoría, **libre** y multiplataforma (Windows, Mac y Linux), escrita en Rust. Combina la facilidad de modelado de ArchiCAD con la organización por familias y tipos de Revit, y usa IFC como lengua común. Nombre provisional.

*ArchiSim is a free (GPL-3.0) cross-platform BIM authoring tool written in Rust, with IFC as its data model. The code, identifiers and comments are in English; the user interface and the project documents are in Spanish for now. Contributions in English are welcome.*

> **Estado: fase 1 completa, en desarrollo activo.** Sirve para modelar y probar, todavía no para un proyecto real de principio a fin. El formato de los proyectos puede cambiar; los ficheros antiguos se siguen abriendo.

## Qué hace hoy

- **Muros multicapa** rectos y curvos, dibujados por cadena, rectángulo, arco o círculo; uniones en L y en T con prioridad por capa; restricción superior a una planta.
- **Forjados** por contorno, rectángulo o varita mágica, con huecos (escaleras, patinillos).
- **Puertas, ventanas y huecos** como familias paramétricas descritas en recetas de texto (`.asfam`), con símbolo de planta por escala (1:50 y 1:100).
- **Planta** con tramas por material, cotas, cotas de nivel y calco de la planta inferior; **vista 3D**.
- **Edición** con el ratón: mover, copiar, estirar, curvar; forzados, cotas temporales editables y líneas de rastreo.
- **Tipos y materiales** editables en la aplicación.
- **IFC4**: exportación con tipos, capas, propiedades y cantidades (validada con IfcOpenShell), e **importación** de IFC2X3 e IFC4 de otros programas.
- Proyectos propios en **`.archisim`**, texto legible ([formato](docs/FORMATO_ARCHISIM.md)).
- Modelos grandes: miles de muros con respuesta inmediata en planta (`archisim medir` lo comprueba).

La visión, los principios y la arquitectura están en [`docs/FUNDACIONAL.md`](docs/FUNDACIONAL.md); lo que viene después, en [`docs/HOJA_DE_RUTA.md`](docs/HOJA_DE_RUTA.md); el día a día, en [`docs/BITACORA.md`](docs/BITACORA.md).

## Instalarlo

Mientras no haya versiones publicadas, cada cambio genera los instaladores en GitHub. Necesitas una cuenta de GitHub (gratuita) para descargarlos:

1. Abre la pestaña **Actions** del repositorio y entra en la última ejecución en verde de la rama `main`.
2. Baja hasta **Artifacts** y descarga:
   - **ArchiSim-Windows-Instalador**: el instalador `ArchiSim-<versión>-Windows.exe`. Asocia los `.archisim` y ofrece ArchiSim en «Abrir con» para los `.ifc`. Como no está firmado, la primera vez Windows avisa: *Más información → Ejecutar de todas formas*. (**ArchiSim-Windows** es el mismo programa sin instalar: descomprime y abre `ArchiSim.exe`.)
   - **ArchiSim-Mac**: un `.dmg` con la aplicación (chip Apple e Intel). La primera vez hay que permitirla en *Ajustes del Sistema → Privacidad y seguridad*.
3. La vivienda de prueba `demo.archisim` viene con el programa (en Windows, en el menú Inicio → «Ejemplos de ArchiSim»).

## Compilarlo

1. Instala Rust: <https://rustup.rs> (en Windows, acepta las herramientas de compilación de Visual Studio que propone).
2. En una terminal, dentro de esta carpeta:

   ```bash
   cargo run -p archisim-cli -- demo out            # vivienda de prueba: out/demo.archisim, out/demo.ifc y plantas SVG
   cargo run --release -p archisim-app -- out/demo.archisim   # la aplicación
   ```

   Otros comandos: `exportar` (proyecto → IFC y plantas), `importar` (IFC → proyecto), `medir` (rendimiento con un modelo grande). `cargo run -p archisim-cli -- --help` los lista.

## Colaborar

- **Arquitectos y técnicos**: lo más útil es probarlo y contar qué falla o qué falta en un *issue*, con capturas. Si exportas un IFC a ArchiCAD, Revit u otro programa, di qué se ve bien y qué no.
- **Programadores**: lee antes [`docs/FUNDACIONAL.md`](docs/FUNDACIONAL.md) y [`CLAUDE.md`](CLAUDE.md), que recoge las convenciones (unidades, orden de capas, identificadores IFC estables) y las comprobaciones que tiene que pasar cada cambio:

  ```bash
  cargo fmt --all
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  python3 scripts/validate_ifc.py out/demo.ifc   # pip install ifcopenshell
  ```

- **Sala limpia**: no se acepta código, ficheros, bibliotecas ni contenido de Autodesk o Graphisoft. Se replican comportamientos observables y estándares públicos (IFC, UNE), nunca su implementación. Tampoco se suben IFC de proyectos reales.

## Licencia

ArchiSim es software libre: puedes usarlo, estudiarlo, modificarlo y redistribuirlo según la **GNU General Public License, versión 3 o posterior** ([`LICENSE`](LICENSE)). Quien distribuya una versión modificada tiene que publicarla con la misma licencia. Se ofrece sin ninguna garantía.

Las fuentes IBM Plex que usa la interfaz tienen su propia licencia, la SIL Open Font License 1.1 ([`OFL-IBM-Plex.txt`](crates/archisim-app/assets/fonts/OFL-IBM-Plex.txt)).

Copyright © 2026 Antonio Fernández y colaboradores de ArchiSim.
