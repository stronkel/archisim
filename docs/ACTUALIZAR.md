# Actualizar ArchiSim desde el propio programa

ArchiSim no hace falta descargarlo de GitHub cada vez: se actualiza solo.

## Para quien lo usa

- **Al abrirlo**, ArchiSim pregunta a GitHub cuál es la última versión publicada (sin internet no pasa nada). Si hay una nueva, sale un aviso en la esquina de abajo a la derecha: «Ver novedades y actualizar» o «Más tarde».
- **Ayuda → «Buscar actualizaciones…»** abre la misma ventana cuando se quiera: la versión que tienes, la nueva y qué cambia (de `NOVEDADES.md`).
- **Descargar**: el instalador de tu sistema, con una barra de progreso.
- **Instalar y volver a abrir**: si hay cambios sin guardar, primero pide guardar. ArchiSim se cierra, se instala la versión nueva y se vuelve a abrir con el mismo proyecto.
  - **Windows**: el instalador de siempre, sin preguntas (`/SILENT`), en la misma carpeta y con las mismas opciones.
  - **Mac**: la misma instalación que la línea de Terminal de la página de la versión (`scripts/instalar_mac.sh`), en la carpeta donde está ArchiSim (Aplicaciones, normalmente), sin cuarentena.
  - En Linux se actualiza compilando.
- «No volver a avisarme de esta versión» y «Buscar actualizaciones al abrir ArchiSim» están en la misma ventana.

## Cómo se publica una versión

- La publica la integración continua (`.github/workflows/release.yml`) al unir a `main` un cambio con un número de versión que aún no está publicado (`version` de `Cargo.toml`). Hace el `.dmg` de Mac, el instalador de Windows y la página de la versión, con lo de su apartado en `docs/NOVEDADES.md`.
- **Al cerrar un bloque** que se une a `main`: subir la versión en `Cargo.toml` (la tercera cifra para remates, la segunda para bloques; `cargo update -w`) y escribir su apartado `## <versión>` al principio de `docs/NOVEDADES.md`, en términos de arquitectura.

## En el código

- `crates/archisim-app/src/updates.rs`: la última versión (`LATEST_URL`, `read_release`), comparar versiones (`is_newer`), qué cambia (`changes`), el instalador de este sistema (`installer_name`, `download_path`) y ponerlo en su sitio (`start_install`).
- `crates/archisim-app/src/app/update_ui.rs`: la búsqueda al abrir (en un hilo), el aviso (`update_note`), la ventana (`update_contents`) y la descarga (`fetch_file`).
- Preferencias `buscar_actualizaciones` y `version_saltada` (`Prefs::check_updates`, `skipped_version`).
- Windows: en `scripts/windows_installer.iss`, `[Run]` con `Check: UpdatingWith`/`UpdatingWithout` vuelve a abrir ArchiSim tras la instalación silenciosa (`/actualizar=1`, `/abrir=<proyecto>`).
