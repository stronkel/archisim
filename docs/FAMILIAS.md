# Familias paramétricas — diseño (fase 2, punto 2.1)

> 18 de octubre de 2026 · diseño aprobado por Antonio, todavía sin código.
> Equivale a las familias de Revit (`.rfa`) y a los objetos GDL de ArchiCAD,
> con formato y contenido propios (sala limpia: nada de GDL ni de `.rfa`).

## Decisiones de Antonio (18/10/2026)

| # | Decisión | Elegido |
|---|---|---|
| D2 | Cómo se describe una familia | **Receta de texto** como motor. Un editor gráfico de recetas podrá venir después, escribiendo el mismo fichero |
| — | Cómo crea tipos el arquitecto | **Configurador como el de ArchiCAD**: diálogo con todas las opciones por secciones y vista previa en tiempo real. Sin dibujar: se duplica un tipo y se configura |
| — | Cuántas familias | **Una familia por funcionamiento** (abatible, corredera…), con las variantes como opciones (hojas, tarjas, subdivisiones…) y **piezas comunes** compartidas por todas |
| — | Reutilizar y medir | **Tipos con nombre, guardables en un fichero de familia aparte** (como Revit) para usarlos en otros proyectos; **navegador de familias y tipos con su número de ejemplares**, y el tipo como unidad de medición |
| — | Dónde viven las familias | **Copia dentro del proyecto** (como Revit): el `.archisim` se abre igual en cualquier equipo. La biblioteca en disco sirve para cargar y actualizar |
| — | Nivel de detalle del símbolo en planta | **Por escala** (como ArchiCAD): 1:100 sencillo, 1:50 medio, 1:20 detallado, automático |
| — | Por dónde empezar | **Puertas y ventanas**: las actuales se convierten en familias; mobiliario y sanitarios después |

## Dos niveles: configurador y receta

- **Configurador (uso diario).** Diálogo con secciones —dimensiones, alféizar y antepecho, marco y premarco, hojas y funcionamiento, subdivisiones, tarjas, persiana y capialzado, vierteaguas, manilla, materiales, estilos de línea— y, al lado, la vista previa en **planta, alzado y 3D** en tiempo real, dibujada por el mismo motor que el proyecto. Solo aparecen las opciones que aplican (las de la segunda hoja, si hay dos). Crear un tipo = duplicar uno y configurarlo. No se puede convertir una familia en otra (una corredera no pasa a abatible), como en ArchiCAD.
- **Receta (libertad total).** El texto que hay debajo. El configurador se genera solo a partir de sus parámetros, agrupados en secciones. Lo que el configurador no cubra se resuelve con una receta nueva.

## Familias, opciones y piezas comunes

Regla:

- **Familia distinta** cuando cambia cómo funciona el elemento: su símbolo en planta, su giro y su funcionamiento en IFC.
- **Opción dentro de la familia** cuando solo añade piezas o cambia medidas: nº de hojas, tarja superior o lateral, subdivisiones, premarco, persiana, manilla, materiales.
- **Piezas comunes** (marco, premarco, hoja, tarja, persiana y capialzado, vierteaguas, manilla…), escritas y probadas una vez y usadas por todas las familias: una tarja o un capialzado se comportan igual en una puerta abatible que en una corredera.

Familias iniciales:

| Puertas | Ventanas |
|---|---|
| **Abatible**: 1 o 2 hojas (iguales o desiguales), con o sin tarjas | **Practicable / oscilobatiente**: 1 a 3 hojas, con fijos combinados |
| **Corredera**: sobre muro, o dentro del tabique en armazón | **Corredera**: 2 a 4 hojas |
| | **Fija** |

La balconera es la ventana practicable con antepecho 0. Plegables, pivotantes, basculantes y puertas de garaje vienen después en la librería (2.2), con las mismas piezas.

Los tipos actuales de la plantilla se convierten así: puerta de 1 y de 2 hojas → *Abatible*; puerta corredera → *Corredera*; ventana fija → *Fija*; practicable de 1 y de 2 hojas → *Practicable*; ventana corredera → *Corredera*.

## Tipos: reutilizables y medibles

- **Qué es del tipo y qué del ejemplar.** Todo lo que define el producto es del **tipo**: medidas, hojas, tarjas, subdivisiones, materiales, manilla. Del **ejemplar** solo lo que depende de dónde se coloca: posición en el muro, antepecho, lado de apertura y mano. Así «12 × P-03» describe una puerta de verdad, que es lo que se mide y se pide al carpintero. (En ArchiCAD casi todo es del ejemplar, y por eso acabas recreando la misma puerta.)
- **Cada tipo tiene**: nombre («Puerta paso 1 hoja 82,5×203 lacada blanca»), **marca de tipo** (P-03, V-01) para etiquetas y tablas, descripción, `GlobalId` estable y la familia de la que sale con su versión.
- **Ficheros de familia**: un fichero `.asfam` contiene la receta de una familia **y su catálogo de tipos**, como un `.rfa` con sus tipos. Desde el proyecto, *Guardar tipo en biblioteca* añade o actualiza el tipo en el fichero de su familia de la biblioteca de AD5; desde otro proyecto, *Cargar tipos* lo trae con todo (receta incluida). La biblioteca es una carpeta compartible por todo el estudio.
- **Mismo tipo en varios proyectos**: un tipo cargado conserva su `GlobalId`. Si al cargarlo ya existe uno con el mismo nombre y los mismos valores, no se duplica; si tiene el mismo nombre y valores distintos, se pregunta si sustituir o renombrar.
- **Navegador del proyecto** (como el de Revit): Puertas → Abatible → tipos, cada uno con su **número de ejemplares**; doble clic abre el configurador; desde ahí, seleccionar todos los ejemplares de un tipo. Los tipos sin usar se ven y se pueden purgar.
- **Mediciones**: el tipo es la unidad. Las tablas de carpintería (3.7) agrupan por tipo (y, si se quiere, por mano), con la marca, la cantidad y sus datos; el IFC lleva el tipo enlazado (`IfcRelDefinesByType`) para que lo cuenten igual Revit, ArchiCAD o Presto.

## Qué es una familia

Una familia es un fichero `.asfam` con estas partes:

1. **Parámetros.** De **tipo** o de **ejemplar** (ver arriba). Clases: longitud, ángulo, entero, sí/no, material, estilo de línea, texto y lista de opciones. Cada uno con valor por defecto, rango, nombre visible en español, **sección** del configurador y, si hace falta, una **condición de visibilidad** («solo si hojas = 2»). Se pueden **calcular con fórmulas** (`hoja = (ancho − 2·marco) / hojas`).
2. **Geometría 3D**, con piezas sencillas (cajas, prismas de un perfil, perfiles barridos a lo largo de un recorrido) y **piezas comunes**, con **repeticiones** (nº de hojas) y **condiciones** (con o sin persiana). Cada pieza lleva su material.
3. **Símbolo en planta** por nivel de detalle (1:100, 1:50, 1:20) y **alzado** para la vista previa, con líneas de **corte**, **proyección**, **ocultas** y **por encima del plano de corte**. Los grosores reales los fija la escala (D5), no la familia.
4. **Comportamiento.** Categoría (puerta, ventana, mobiliario, sanitario, equipamiento de cocina…), si va **alojada en un muro** y qué hueco de obra le abre, punto de inserción, y **puntos de control** (los *hotspots* de ArchiCAD) que estiran un parámetro con el ratón.
5. **IFC.** Clase (`IfcDoor`, `IfcWindow`, `IfcFurniture`, `IfcSanitaryTerminal`…), tipo predefinido, particiones y funcionamiento de hojas, y qué parámetros salen en qué property set.
6. **Catálogo de tipos** (opcional): los tipos guardados de esa familia.

Ejes locales: **x** a lo largo del muro (o hacia la derecha del objeto), **y** a través del muro desde la cara exterior hacia la interior (o hacia el frente del objeto), **z** hacia arriba desde la base (el antepecho en ventanas). Metros.

## Ejemplo orientativo de receta

La sintaxis exacta se fija en el bloque A. Las palabras clave van en inglés, como IFC, porque el código será abierto; lo que ve el usuario (nombres, secciones, opciones) va en español.

```
family "Ventana practicable"
  category window              # IfcWindow
  host wall                    # va en un muro y le abre el hueco de obra
  version 1

type_params
  section "Dimensiones"
    width      length   1.20  range 0.40..3.00  label "Hueco de obra: ancho"
    height     length   1.20  range 0.40..3.00  label "Hueco de obra: alto"
  section "Hojas"
    leaves     integer  2     range 1..3        label "Nº de hojas"
    tilt_turn  bool     yes                     label "Oscilobatiente"
  section "Marco"
    frame_w    length   0.06                    label "Anchura vista"
    frame_d    length   0.07                    label "Fondo"
  section "Persiana"
    shutter    bool     no                      label "Persiana y capialzado"
    box_h      length   0.25  visible_if shutter label "Altura del capialzado"
  section "Materiales"
    frame_mat  material "Aluminio lacado"       label "Marco"
    glass_mat  material "Vidrio"                label "Vidrio"

instance_params
  opens_to   choice [interior, exterior] = interior  label "Abre hacia"
  hand       choice [izquierda, derecha] = izquierda label "Mano (UNE-EN 12519)"

let leaf_w = (width - 2 * frame_w) / leaves

body
  use frame(width, height, frame_w, frame_d, frame_mat)        # pieza común
  repeat i in 0..leaves
    use casement_leaf(at: frame_w + i * leaf_w, width: leaf_w, glass: glass_mat)
  if shutter
    use roller_shutter(width, box_h)                            # pieza común

plan 1:50
  use frame_plan(width, frame_w, frame_d)
  repeat i in 0..leaves
    swing leaf i to opens_to

plan 1:100
  line (0, frame_d / 2) to (width, frame_d / 2)

hotspot width  at (width, 0)          # estirar el ancho con el ratón

ifc
  predefined_type WINDOW
  partitioning = if leaves == 1 then SINGLE_PANEL else DOUBLE_PANEL_VERTICAL

types                                  # catálogo de tipos guardados
  "V-01 Practicable 2 hojas 120×120"  width 1.20 height 1.20 leaves 2
  "V-02 Balconera 2 hojas 140×220"    width 1.40 height 2.20 leaves 2 shutter yes
```

## Reglas del sistema

- **El muro sigue siendo del muro.** La familia declara el hueco de obra y dónde se asienta su marco (retranqueo); las mochetas, la vuelta de la hoja exterior y el envolvente del monocapa siguen siendo reglas del muro, como hoy. Así una ventana queda bien en cualquier multicapa.
- **Recetas seguras y deterministas.** Sin acceso a ficheros ni red, repeticiones con tope. La misma receta con los mismos valores da siempre la misma geometría, en planta, 3D e IFC.
- **Familias probadas en todo su rango.** Cada familia de la librería se prueba automáticamente en todas las combinaciones de opciones y en los extremos de sus rangos: geometría cerrada, nada fuera del hueco, IFC válido. Así se evitan los errores típicos de las familias caseras.
- **Se rechaza lo que no cabe**, como en el resto del programa, con el parámetro y el motivo («ancho 0,30 fuera del rango 0,40–3,00»).
- **Cambiar un tipo cambia todos sus ejemplares**, que conservan su `GlobalId`. *Actualizar desde biblioteca* sustituye la receta por una versión más nueva comprobando que todo sigue cabiendo; si no, se rechaza entera, con el motivo.
- **Nivel de detalle por escala**: la planta usa el símbolo de la escala más cercana que defina la familia (1:20 → 1:50 → 1:100).
- **IFC.** Cada ejemplar sale con su geometría propia (como hoy) y enlazado a su tipo (`IfcDoorType`, `IfcWindowType`, `IfcFurnitureType`…) con nombre y marca. Los parámetros van en los Psets estándar (`Pset_WindowCommon`…) y, todos, en `ArchiSim_Family` (familia, versión y valores), para la ida y vuelta exacta. La geometría compartida por tipo (`RepresentationMaps`) queda para más adelante, cuando se pruebe en Revit y ArchiCAD.

## Cómo se hace: cinco bloques

| Bloque | Qué | Criterio de hecho |
|---|---|---|
| **A · Núcleo** | Lector de recetas: parámetros con secciones y visibilidad, fórmulas, condiciones, repeticiones, piezas comunes, geometría 3D. Tipos con nombre, marca y familia. Los tipos actuales pasan a las cinco familias (abatible, corredera; practicable, corredera, fija). Formato `.archisim` **versión 3** con migración | La demo y los proyectos v1/v2 se abren y dan **el mismo 3D y el mismo IFC** que hoy (volúmenes y `GlobalId` incluidos), validado con IfcOpenShell |
| **B · Planta** | Símbolos 2D por nivel de detalle y alzado; el dibujo fijo de puertas y ventanas de `archisim-plan` se sustituye por el de las familias | La planta 1:50 de la demo sale igual que hoy (comparada en imagen); nuevas plantas 1:100 y 1:20 para revisar |
| **C · IFC** | Tipos con nombre y marca, Psets desde los parámetros, `ArchiSim_Family`, importación de lo exportado | Ida y vuelta idéntica; IFC abierto en ArchiCAD y Revit (Antonio) |
| **D · Configurador** | El diálogo de tipos generado desde la receta, por secciones, con vista previa en planta, alzado y 3D en tiempo real; puntos de control en planta | Probado con clics simulados y en la ventana real; las opciones de una familia se recorren todas |
| **E · Navegador y biblioteca** | Navegador de familias y tipos con nº de ejemplares; seleccionar ejemplares de un tipo; purgar; ficheros `.asfam` con catálogo de tipos; guardar tipo en biblioteca y cargar tipos en otro proyecto; herramienta **Objeto** para familias sueltas (mobiliario) | Un tipo creado en un proyecto se carga en otro y sale igual, con el mismo `GlobalId` |

Después viene la **librería** (2.2): primero completar las cinco familias con todas sus opciones (premarco, tarjas, persiana y capialzado, vierteaguas, manillas, subdivisiones), luego mobiliario, sanitarios y cocina.

## Pendiente de decidir más adelante

- Editor gráfico de recetas (encima del mismo formato).
- Parámetros compartidos entre familias y propiedades de usuario (`ArchiSim_` Psets libres).
- Familias de anotación (etiquetas, fase 3).
- Importar objetos de otros programas (`IfcFurniture` con malla) como familias fijas, con la importación de segunda vuelta (1.7).
