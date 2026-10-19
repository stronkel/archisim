# Familias paramétricas — diseño (fase 2, punto 2.1)

> 18 de octubre de 2026 · diseño aprobado por Antonio. **Bloque A hecho (v0.14):** el lenguaje, las familias de sistema y los tipos por familia con marca. **Bloque B hecho (v0.15):** símbolos de planta por escala desde las recetas, ventana practicable de 1 a 4 hojas con funcionamiento por hoja (también en IFC), y el diálogo de tipos generado desde la receta. Siguen C (IFC desde las familias), D (configurador completo) y E (navegador y biblioteca).
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
| — | Ventana fija (18/10, tras el bloque A) | **No es una familia aparte**: es una ventana practicable con hojas fijas. Cada hoja se configura por separado (fija, practicable, oscilobatiente, abatible, proyectante, basculante), como la carpintería real y como lo modela IFC |

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
| **Abatible**: 1 o 2 hojas (hoy iguales; desiguales, hoja activa y tarjas en la librería) | **Practicable**: 1 a 4 hojas, cada una fija, practicable, oscilobatiente, abatible, proyectante o basculante, con montantes |
| **Corredera**: sobre muro (empotrada en armazón, en la librería) | **Corredera**: 2 hojas (hasta 4 y hojas fijas, en la librería) |
| **Plegable** (en la librería): hojas que se pliegan de dos en dos sobre una guía. Es familia aparte porque su planta (el zigzag), su 3D y su funcionamiento IFC (`FOLDING_TO_LEFT`, `FOLDING_TO_RIGHT`, `DOUBLE_DOOR_FOLDING`) son otros | |

La balconera es la ventana practicable con antepecho 0. Plegables, pivotantes, basculantes y puertas de garaje vienen después en la librería (2.2), con las mismas piezas.

Los tipos anteriores a las familias se convierten así: puerta de 1 y de 2 hojas → *Abatible*; puerta corredera → *Corredera*; ventana fija → *Practicable* de 1 hoja fija; practicable de 1 y de 2 hojas → *Practicable* con sus hojas practicables; ventana corredera → *Corredera*.

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

Ejes de las familias alojadas en muro (metros): origen en el **centro del hueco de obra**, a la altura del **antepecho** (el suelo en las puertas) y en el **plano medio del marco**; **x** a lo largo del muro, hacia la derecha vista desde el lado al que abren las hojas; **y** a través del muro, hacia ese lado; **z** hacia arriba. Así la familia no necesita saber el espesor ni la orientación del muro, y la mano y el lado de apertura la reflejan solos.

Ejes de los **objetos** (familias `host none` de categoría `furniture`, `sanitary` o `kitchen`, v0.25): origen en el **centro del lado de atrás**, a ras de suelo; **x** hacia la derecha, **y** hacia delante (hacia donde mira el objeto), **z** hacia arriba. Así una cama o un inodoro se colocan con un clic contra la cara del muro. Sus salidas `ifc_class` (`IfcFurniture` o `IfcSanitaryTerminal`) e `ifc_type` (`BED`, `TOILETPAN`…) deciden la clase IFC. Las familias de sistema de objetos están en `crates/archisim-core/families/objetos/`.

## El lenguaje (como está en el bloque B)

Palabras clave en inglés, como IFC, porque el código será abierto; lo que ve el usuario (nombres, secciones, opciones, mensajes) va en español. La indentación (con espacios) marca qué va dentro de qué. `#` empieza un comentario.

```
family "Ventana practicable"
  category window              # door, window, furniture, sanitary…
  host wall                    # va en un muro y le abre el hueco de obra (o none)
  version 1
  description "Ventana practicable de una o dos hojas"

include "piezas"               # piezas comunes; se copian dentro al cargar la familia

type_params
  section "Dimensiones"
    width  length = 1.20 range 0.10..20.00 label "Hueco de obra: ancho"
    height length = 1.20 range 0.10..20.00 label "Hueco de obra: alto"
  section "Hojas"
    leaves integer = 2 range 1..2 label "Nº de hojas"
  section "Marco"
    frame_width length = 0.06 range 0.001..2.00 label "Marco: anchura vista"
    frame_depth length = 0.07 range 0.001..2.00 label "Marco: fondo"
  section "Persiana"
    shutter bool = no label "Persiana y capialzado"
    box_h   length = 0.25 visible_if shutter label "Altura del capialzado"
  section "Materiales"
    frame_material material = "Aluminio lacado" label "Marco"
    panel_material material = "Vidrio doble" label "Vidrio"

instance_params
  finish choice("lacado", "anodizado") = "lacado" label "Acabado"

let f = frame_width
require (leaves + 1) * f < width else "El marco y el montante no dejan hueco para el vidrio"

component shutter_box(w, h, d, box_h, mat)       # una pieza propia, con sus argumentos
  use block(-w / 2, w / 2, d, h, h + box_h, mat)   # «block» viene de las piezas comunes

body
  use window_frame(width, height, f, frame_depth, frame_material)
  let clear = (width - 2 * f - (leaves - 1) * f) / leaves
  repeat i in 0..leaves
    let a = -width / 2 + f + i * (clear + f)
    use block(a, a + clear, min(0.024, frame_depth), f, height - f, panel_material)
  if shutter
    use shutter_box(width, height, frame_depth, box_h, frame_material)

plan 1:50                      # símbolo en planta a 1:50 (y a 1:20, la más cercana)
  if sill_height > 0
    use sill_plan(width, frame_depth, wall_ext, wall_int)   # vierteaguas
  use frame_cut_plan(-width / 2, width / 2, frame_depth)     # marco cortado
  line (-width / 2 + f, 0) to (width / 2 - f, 0) pen thin     # vidrio
  if hand == "left"
    use window_leaf_plan(-width / 2 + f, 1, width - 2 * f, frame_depth, "dashed")

plan 1:100                     # a 1:100, más sencillo
  use frame_cut_plan(-width / 2, width / 2, frame_depth)

outputs                        # lo que lee el resto del programa
  opening_width = width
  operation = if leaves == 2 then "double_casement" else "single_casement"

ifc                            # datos IFC
  panel_count = leaves
  panel_1 = "SIDEHUNG" + "LEFT" + "HAND"
  panel_2 = "SIDEHUNGRIGHTHAND"
```

- **Parámetros**: `nombre clase = valor`, con `range`, `label` y `visible_if` opcionales. Clases: `length`, `angle` (radianes), `number`, `integer`, `bool` (`yes`/`no`), `material` (por defecto, el nombre de un material; ArchiSim busca el del proyecto), `text`, `choice("a", "b", …)` y `linestyle` (tipo de línea, por su nombre en inglés: `"dashed"`).
- **Fórmulas**: `+ - * /`, comparaciones (`== != < <= > >=`), `and`, `or`, `not`, `if … then … else …` y las funciones `min`, `max`, `abs`, `sqrt`, `floor`, `ceil`, `round`, `sin`, `cos`, `tan`.
- **Geometría**: `box (x, y, z) to (x, y, z) material m`; `use pieza(…)`; `repeat i in a..b` (b excluido); `if` / `else if` / `else`; `let`. Una pieza común solo ve sus propios argumentos.
- **Textos**: se unen con `+` (`"SIDEHUNG" + side + "HAND"`).
- **Tipos de línea**: detrás de la pluma, `dashed`, `dotted`, `dashdot`, `dashdotdot` o `longdash` (continua si no se pone nada), o `style <valor>` con un parámetro de clase `linestyle` («Línea del giro» en las puertas y ventanas de sistema). En pantalla se eligen por su nombre: continua, discontinua, de puntos, trazo y punto, trazo y dos puntos, trazos largos. Los patrones van en milímetros de papel (UNE-EN ISO 128-2), así que se ven igual a cualquier escala.
- **Símbolos de planta**: un bloque `plan 1:N` por escala; se dibuja el de la escala más cercana (1:20 usa el de 1:50 si no hay otro). Dentro, `line (x, y) to (x, y) pen P [dashed]`, `polygon (x, y) (x, y) (x, y)… pen P [filled]` (relleno blanco, tapa lo que hay debajo) y `arc center (x, y) from (x, y) to (x, y) pen P` (por el camino corto), además de `let`, `if`, `repeat` y piezas comunes. Plumas: `cut` (corte), `frame` (marcos y hojas), `view` (proyección), `thin` (vidrios, giros, cantos) y `hatch`; su grosor real lo pone la escala (D5), no la familia. Una caja no puede ir en planta ni una línea en el cuerpo 3D.
- **Reglas**: `require condición else "mensaje"`; si no se cumple, el cambio se rechaza con ese mensaje.
- **Lo que pone el programa**: `opens_to` (`"interior"` / `"exterior"`) y `hand` (`"left"` / `"right"`, UNE-EN 12519) en las familias alojadas en muro; en planta, además, `wall_ext` y `wall_int` (dónde están las caras exterior e interior del muro, en los ejes de la familia) y `sill_height` (antepecho).
- **Salidas que lee ArchiSim** de una familia de puerta o ventana: `opening_width`, `opening_height`, `frame_width`, `frame_depth`, `frame_material`, `panel_material` y `operation` (`single_swing`, `double_swing`, `sliding`, `folding`; `fixed`, `single_casement`, `double_casement`, `sliding`); en `ifc`, para ventanas, `panel_count` y `panel_1`… con el funcionamiento IFC de cada hoja (`FIXEDCASEMENT`, `SIDEHUNGLEFTHAND`, `TILTANDTURNRIGHTHAND`, `TOPHUNG`…). El IFC escribe como cuerpo de cada puerta o ventana las piezas (`box`) de su receta.
- **Límites de seguridad**: sin acceso a ficheros ni red; como mucho 1 000 vueltas por `repeat`, 20 000 piezas y 32 piezas anidadas.

Las recetas de las cuatro familias de sistema están en `crates/archisim-core/families/` (`puerta_abatible.asfam`, `puerta_corredera.asfam`, `ventana_practicable.asfam`, `ventana_corredera.asfam`), con sus piezas comunes de 3D y de planta en `piezas.asfam`. Sirven de ejemplo.

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
| **A · Núcleo** ✅ v0.14 | Lector de recetas: parámetros con secciones y visibilidad, fórmulas, condiciones, repeticiones, piezas comunes, geometría 3D. Tipos con nombre, marca y familia. Los tipos actuales pasan a las cinco familias (abatible, corredera; practicable, corredera, fija). Formato `.archisim` **versión 3** con migración | La demo y los proyectos v1/v2 se abren y dan **el mismo 3D y el mismo IFC** que hoy (volúmenes y `GlobalId` incluidos), validado con IfcOpenShell |
| **B · Planta** ✅ v0.15 | Símbolos de planta por escala (1:50 y 1:100; el 1:20 y el alzado, con D); el dibujo fijo de puertas y ventanas de `archisim-plan` se sustituye por el de las familias. Además: la fija entra en la practicable, de 1 a 4 hojas con funcionamiento por hoja (también en el IFC y la importación); el diálogo de tipos muestra los parámetros de la receta por secciones; tipo nuevo de cualquier familia | La planta 1:50 de la demo sale igual que antes (comparada en imagen, 0 píxeles distintos) y 176 casos de prueba dibujan lo mismo; plantas 1:100 para revisar |
| **C · IFC** ✅ v0.23 | Cuerpo de puertas y ventanas desde las piezas de la receta; marca en `Tag` del tipo y en `Reference` de `Pset_DoorCommon`/`Pset_WindowCommon`; familia en `ElementType`; descripción; `ArchiSim_Family` (nombre, versión y receta) y `ArchiSim_FamilyValues` (un valor por parámetro, materiales por nombre) en el tipo, y los de ejemplar en la puerta o ventana; la importación rehace la familia, los valores y la marca | Ida y vuelta idéntica también con una familia propia; volúmenes comprobados con IfcOpenShell; IFC abierto en ArchiCAD y Revit (Antonio) |
| **D · Configurador** ◐ v0.24 | Hecho en v0.24: diálogo de tipos al estilo ArchiCAD (carpetas por familia, función o categoría, con dibujo de cada tipo; buscador; «＋» de tipo nuevo por familia); vista previa en planta, sección, alzado y 3D girable, dibujada desde las piezas de la receta; todas las opciones en grupos; **tipos de línea** en los símbolos (continua, discontinua, de puntos, trazo y punto, trazo y dos puntos, trazos largos) y parámetro `linestyle` («Línea del giro» en abatible y practicable). Falta: símbolo 1:20 y líneas de apertura en alzado desde la receta; puntos de control en planta | Probado con clics simulados y en la ventana real; las vistas previas de cada lista se recorren todas |
| **E · Navegador y biblioteca** | Navegador de familias y tipos con nº de ejemplares; seleccionar ejemplares de un tipo; purgar; ficheros `.asfam` con catálogo de tipos; guardar tipo en biblioteca y cargar tipos en otro proyecto; herramienta **Objeto** para familias sueltas (mobiliario) | Un tipo creado en un proyecto se carga en otro y sale igual, con el mismo `GlobalId` |

Después viene la **librería** (2.2): primero completar las cinco familias con todas sus opciones (premarco, tarjas, persiana y capialzado, vierteaguas, manillas, subdivisiones), luego mobiliario, sanitarios y cocina.

## Pendiente de decidir más adelante

- Editor gráfico de recetas (encima del mismo formato).
- Parámetros compartidos entre familias y propiedades de usuario (`ArchiSim_` Psets libres).
- Familias de anotación (etiquetas, fase 3).
- Importar objetos de otros programas (`IfcFurniture` con malla) como familias fijas, con la importación de segunda vuelta (1.7).
