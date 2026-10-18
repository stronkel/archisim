# Familias paramétricas — diseño (fase 2, punto 2.1)

> 18 de octubre de 2026 · diseño aprobado por Antonio, todavía sin código.
> Equivale a las familias de Revit (`.rfa`) y a los objetos GDL de ArchiCAD,
> con formato y contenido propios (sala limpia: nada de GDL ni de `.rfa`).

## Decisiones de Antonio (18/10/2026)

| # | Decisión | Elegido |
|---|---|---|
| D2 | Cómo se describe una familia | **Receta de texto**: un fichero legible. Un editor gráfico podrá venir después, escribiendo el mismo fichero |
| — | Dónde viven las familias | **Copia dentro del proyecto** (como Revit): el `.archisim` se abre igual en cualquier equipo. La biblioteca en disco sirve para cargar familias y actualizarlas |
| — | Nivel de detalle del símbolo en planta | **Por escala** (como ArchiCAD): 1:100 sencillo, 1:50 medio, 1:20 detallado, automático |
| — | Por dónde empezar | **Puertas y ventanas**: las actuales se convierten en familias; mobiliario y sanitarios después |

## Qué es una familia

Una familia es un fichero `.asfam` con cinco partes:

1. **Parámetros.** Cada uno es de **tipo** (igual para todos los ejemplares de un tipo, como el hueco de obra) o de **ejemplar** (propio de cada elemento colocado, como la mano). Clases: longitud, ángulo, entero, sí/no, material, texto y lista de opciones. Cada uno con valor por defecto, rango permitido y nombre visible en español. Se pueden **calcular con fórmulas** a partir de otros (`hoja = (ancho − 2·marco) / hojas`).
2. **Geometría 3D**, a partir de piezas sencillas: cajas, prismas de un perfil, perfiles barridos a lo largo de un recorrido (marcos, premarcos), con **repeticiones** (nº de hojas, cuarterones) y **condiciones** (con o sin persiana). Cada pieza lleva su material.
3. **Símbolo en planta** por nivel de detalle (1:100, 1:50, 1:20), con líneas de **corte**, **proyección**, **ocultas** y **por encima del plano de corte**. Los grosores reales los fija la escala (D5), no la familia.
4. **Comportamiento.** Categoría (puerta, ventana, mobiliario, sanitario, equipamiento de cocina…), si va **alojada en un muro** y qué hueco de obra le abre, punto de inserción, y **puntos de control** (los *hotspots* de ArchiCAD) que estiran un parámetro con el ratón.
5. **IFC.** Clase (`IfcDoor`, `IfcWindow`, `IfcFurniture`, `IfcSanitaryTerminal`…), tipo predefinido, particiones y funcionamiento de hojas, y qué parámetros salen en qué property set.

Ejes locales de la familia: **x** a lo largo del muro (o hacia la derecha del objeto), **y** a través del muro desde la cara exterior hacia la interior (o hacia el frente del objeto), **z** hacia arriba desde la base (el antepecho en ventanas). Metros.

## Ejemplo orientativo de receta

La sintaxis exacta se fija en el bloque A. Las palabras clave van en inglés, como IFC, porque el código será abierto; lo que ve el usuario (nombres de parámetros, opciones) va en español.

```
family "Ventana practicable"
  category window              # IfcWindow
  host wall                    # va en un muro y le abre el hueco de obra
  version 1

type_params
  width      length   1.20  range 0.40..3.00  label "Hueco de obra: ancho"
  height     length   1.20  range 0.40..3.00  label "Hueco de obra: alto"
  leaves     integer  2     range 1..2        label "Nº de hojas"
  frame_w    length   0.06                    label "Marco: anchura vista"
  frame_d    length   0.07                    label "Marco: fondo"
  frame_mat  material "Aluminio lacado"       label "Material del marco"
  glass_mat  material "Vidrio"                label "Vidrio"

instance_params
  opens_to   choice [interior, exterior] = interior  label "Abre hacia"
  hand       choice [izquierda, derecha] = izquierda label "Mano (UNE-EN 12519)"

let leaf_w = (width - 2 * frame_w) / leaves

body
  sweep rect(frame_w, frame_d) along opening_outline  material frame_mat
  repeat i in 0..leaves
    box at (frame_w + i * leaf_w, frame_d / 2, frame_w)
        size (leaf_w, 0.024, height - 2 * frame_w)    material glass_mat

plan 1:50
  cut  rect(0, 0, width, frame_d)                     # marco cortado
  line (frame_w, frame_d / 2) to (width - frame_w, frame_d / 2)   # vidrio
  repeat i in 0..leaves
    swing leaf i to opens_to                          # giro de la hoja

plan 1:100
  line (0, frame_d / 2) to (width, frame_d / 2)

hotspot width  at (width, 0)                          # estirar el ancho con el ratón

ifc
  predefined_type WINDOW
  partitioning = if leaves == 1 then SINGLE_PANEL else DOUBLE_PANEL_VERTICAL
```

## Reglas del sistema

- **El muro sigue siendo del muro.** La familia declara el hueco de obra y dónde se asienta su marco (retranqueo); las mochetas, la vuelta de la hoja exterior y el envolvente del monocapa siguen siendo reglas del muro, como hoy. Así una ventana de la librería queda bien en cualquier multicapa.
- **Recetas seguras y deterministas.** Sin acceso a ficheros ni red, sin bucles infinitos (las repeticiones tienen tope). La misma receta con los mismos valores da siempre la misma geometría, en planta, 3D e IFC.
- **Se rechaza lo que no cabe**, como en el resto del programa: un valor fuera de rango, una fórmula que da una hoja negativa, un hueco que se sale del muro. El mensaje dice qué parámetro y por qué («ancho 0,30 fuera del rango 0,40–3,00»).
- **Tipos y ejemplares como Revit.** Un tipo es una familia con sus parámetros de tipo fijados («Ventana practicable 120×120»). Cambiar un tipo cambia todos sus ejemplares, que conservan su `GlobalId`. Los tipos nuevos se crean duplicando, como ahora.
- **Copia en el proyecto.** El `.archisim` guarda el texto entero de cada familia que usa, con su versión. *Cargar familia* la trae de la biblioteca; *Actualizar desde biblioteca* sustituye la copia por una versión más nueva, comprobando que todos los ejemplares siguen cabiendo (si no, se rechaza entera, con el motivo).
- **Nivel de detalle por escala**: la planta elige el símbolo de la escala más cercana que la familia defina (1:20 → 1:50 → 1:100). La vista de planta de la aplicación sigue siendo la planta 1:50 exportada.
- **IFC.** Cada ejemplar sale con su geometría propia (como hoy) y su tipo (`IfcDoorType`, `IfcWindowType`, `IfcFurnitureType`…). Los parámetros van en los Psets estándar que les correspondan (`Pset_WindowCommon`…) y, todos, en `ArchiSim_Family` (familia, versión y valores), para que la ida y vuelta sea exacta. La geometría compartida por tipo (`RepresentationMaps`) queda para más adelante, cuando se pruebe en Revit y ArchiCAD.

## Cómo se hace: cuatro bloques

| Bloque | Qué | Criterio de hecho |
|---|---|---|
| **A · Núcleo** | Lector de recetas, parámetros, fórmulas, condiciones y repeticiones, geometría 3D. Las puertas y ventanas actuales pasan a ser cinco familias de la plantilla (puerta abatible de 1 o 2 hojas, puerta corredera, ventana fija, ventana practicable de 1 o 2 hojas, ventana corredera). Formato `.archisim` **versión 3** con migración | La demo y los proyectos v1/v2 se abren y dan **el mismo 3D y el mismo IFC** que hoy (volúmenes y `GlobalId` incluidos), validado con IfcOpenShell |
| **B · Planta** | Símbolos 2D por nivel de detalle; el dibujo fijo de puertas y ventanas de `archisim-plan` se sustituye por el de las familias | La planta 1:50 de la demo sale igual que hoy (comparada en imagen); nuevas plantas 1:100 y 1:20 para revisar |
| **C · IFC** | Tipos y Psets desde los parámetros, `ArchiSim_Family`, importación de lo exportado | Ida y vuelta idéntica; IFC abierto en ArchiCAD y Revit (Antonio) |
| **D · Aplicación** | Diálogo de tipos generado desde los parámetros de cada familia; *Cargar familia* y *Actualizar desde biblioteca*; puntos de control en planta; herramienta **Objeto** para colocar familias sueltas (mobiliario) | Probado con clics simulados y en la ventana real |

Después viene la **librería** (2.2): premarco, persiana y capialzado, vierteaguas, puertas de paso con cuarterones, mobiliario, sanitarios y cocina, ya como recetas.

## Pendiente de decidir más adelante

- Editor gráfico de familias (encima del mismo formato).
- Parámetros compartidos entre familias y propiedades de usuario (`ArchiSim_` Psets libres).
- Familias anidadas (una manilla dentro de una puerta) y familias de anotación (etiquetas, fase 3).
- Importar objetos de otros programas (`IfcFurniture` con malla) como familias fijas, con la importación de segunda vuelta (1.7).
