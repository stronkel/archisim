# Pantalla de inicio y vistas previas (bloque 3 de la revisión del 22/10/2026)

Antonio pidió una pantalla de inicio con bienvenida y proyectos recientes, y ver «cualquier cosa» antes de soltarla al moverla o colocarla en la planta.

## Pantalla de inicio

Al abrir ArchiSim sin un fichero aparece la bienvenida sobre la ventana (oscurecida):

- **Proyecto nuevo**, **Abrir…**, **Importar IFC…** y **Proyecto en equipo…**.
- **Recientes**: los últimos doce proyectos abiertos o guardados (también IFC importados y proyectos de equipo), cada uno con un dibujo pequeño de su planta más llena y cuándo se cambió («hace 2 h», «ayer», «12/10/2026»). Clic para abrirlo; clic derecho para quitarlo de la lista. Si el fichero ya no está, sale atenuado con «No se encuentra».
- **Primeros pasos**: las misiones de iniciación. **Seguir sin abrir nada** o Esc la cierran.

Vuelve con Archivo → **Pantalla de inicio**; Archivo → **Abrir reciente** tiene la misma lista. La lista se guarda en las preferencias del ordenador y los dibujos en la carpeta de ajustes (`miniaturas/`), al guardar o abrir.

## Vistas previas reales en la planta

Antes de hacer clic, la planta dibuja lo que se va a colocar tal como quedará: un mueble con su símbolo, una puerta o ventana con su hoja y su barrido en el muro ya cortado, un muro con sus capas y sus uniones, un pilar, una viga o un forjado. Al arrastrar (mover, copiar, estirar), lo que se mueve se ve dibujado en su sitio nuevo y desaparece del antiguo. Encima sigue el contorno azul y la medida.

Se hace con una copia del modelo y el mismo dibujo de la planta, solo en la zona que cambia. Con modelos de más de 3000 muros, al colocar se ve solo el contorno, para no frenar el ratón.

## Para el programador

- `app/start_ui` (`StartScreen`, `remember`, `thumbnail`, `save_thumbnail`, `note_recent`, `recent_menu`); `Prefs::recent` («recientes» en `preferencias.json`).
- `app/ghost3d`: `PlanGhost`, `draw_plan_ghost` (desde `plan_ui` después de la planta), `build_plan_ghost` con el mismo `Would` que la vista 3D; `drag::ghost` da el modelo después del arrastre y su zona. `plan_ui::screen_plan_options` reúne las opciones de la planta en pantalla.

## Pendiente

- Vista previa real también para cimentaciones, plantaciones, instalaciones y módulos (hoy, su contorno), y al arrastrar en la vista 3D.
