# Formato de proyecto `.archisim` (versión 7)

El proyecto de ArchiSim se guarda en un único fichero de texto JSON con extensión `.archisim`. Es el equivalente al `.pln` de ArchiCAD o al `.rvt` de Revit, pero legible con cualquier editor de texto y comparable entre versiones (Git, diff).

## Principios

- **Todo en metros.** El campo `"units": "m"` lo deja escrito. La interfaz mostrará milímetros, pero el fichero guarda metros.
- **Identidad estable.** Cada elemento conserva su `id` interno y su `guid` (el `GlobalId` IFC de 22 caracteres). Guardar, abrir y volver a exportar da **el mismo IFC**: los ingenieros pueden federar y comparar versiones sin que los elementos "cambien de identidad".
- **Versionado.** `"version": 7` es la versión del formato, no del programa. ArchiSim actualiza los ficheros antiguos al abrirlos: la versión 1 (ArchiSim 0.4 a 0.9) se abre añadiendo listas de cotas vacías; la versión 2 (ArchiSim 0.10 a 0.13) convirtiendo cada tipo de puerta o ventana en un tipo de la familia de sistema que le corresponde, con el mismo nombre, `guid`, medidas y materiales, y una marca nueva (P-01… para puertas, V-01… para ventanas, por orden); la versión 3 (ArchiSim 0.14 a 0.16) se abre tal cual, con todos sus muros de altura propia; la versión 4 (ArchiSim 0.17 y 0.18), tal cual, con todos sus muros rectos; la versión 5 (ArchiSim 0.19), tal cual, sin huecos de forjado ni huecos sin carpintería (la familia Hueco); la versión 6 (ArchiSim 0.20 a 0.24), tal cual, con solo la capa de dibujo «0» y sin dibujo 2D, planos, locales ni tablas. Un fichero guardado por un ArchiSim más nuevo se rechaza con un aviso claro en vez de abrirse mal.
- **Familias dentro del proyecto.** El fichero lleva copia de la receta de cada familia que usa (como Revit guarda sus familias cargadas), así que el proyecto se abre igual en cualquier ordenador aunque la biblioteca cambie.
- **Se comprueba al abrir.** El modelo se reconstruye aplicando las mismas reglas que al dibujar: referencias existentes, `id` y `guid` únicos, números válidos, contornos sin cruces, puertas y ventanas dentro de su muro y sin solaparse. Si algo falla, el proyecto no se abre a medias: se dice qué elemento y por qué.
- **Guardado seguro.** Se escribe primero un fichero temporal junto al definitivo y luego se sustituye; un corte de luz al guardar no deja un proyecto truncado.
- **No se guarda el historial de deshacer**, como en ArchiCAD y Revit.

## Estructura

| Bloque | Contenido | Equivale a |
|---|---|---|
| `format`, `version`, `units`, `generator` | Cabecera: tipo de fichero, versión del formato, unidades, programa que lo guardó | — |
| `project` | Nombre y `guid` del proyecto, del emplazamiento y del edificio | `IfcProject`, `IfcSite`, `IfcBuilding` |
| `levels` | Niveles con su cota | `IfcBuildingStorey` |
| `materials` | Materiales: categoría IFC, color de corte (`#RRGGBB`) y trama (`solid`, `diagonal`, `cross`, `insulation`, `dots`, `empty`) | `IfcMaterial` |
| `wall_types` | Tipos de muro: función (`exterior` / `interior`) y capas **de exterior a interior** | `IfcWallType` + capas |
| `walls` | Muros: `type`, `level`, `start` y `end` como `[x, y]`, línea de referencia (`center`, `exterior_face`, `interior_face`), altura y desfase de la base; si es curvo (versión 5), `bulge`: la comba del arco que va de `start` a `end`, `tan(θ/4)` del ángulo que barre, positiva en sentido antihorario (1 = media circunferencia; sin `bulge`, muro recto); y, si llega hasta una planta (versión 4), `top` con esa planta (`level`) y el desfase superior (`offset`); sin `top`, la altura es propia. Con `top`, la altura escrita es la que resulta, y al abrir se vuelve a calcular desde las cotas de las plantas | `IfcWall` |
| `slab_types` | Tipos de forjado: función (`floor`, `base_slab`, `roof`) y capas **de arriba abajo** | `IfcSlabType` + capas |
| `slabs` | Forjados: contorno (lista de `[x, y]`), nivel y desfase de la cara superior; si tiene huecos (versión 6), `holes`: una lista de contornos | `IfcSlab` |
| `families` | Familias (versión 3): `name` y `version` (los de la receta, para quien lea el fichero) y la receta `source`, una línea de texto por elemento de la lista, con sus piezas comunes ya incluidas. Ver `docs/FAMILIAS.md` | Familia `.rfa` / objeto GDL |
| `opening_types` | Tipos de puerta o ventana (versión 3): nombre, marca de tipo (`mark`, única o vacía), descripción, familia (`family`, su `id`) y el valor de cada parámetro de tipo de la familia (`values`). Lo demás (funcionamiento, hueco de obra, marco) lo calcula la familia al abrir | `IfcDoorType` / `IfcWindowType` |
| `openings` | Puertas y ventanas: muro, posición del centro (`along`), antepecho (`sill`), lado de apertura, mano (UNE-EN 12519), retranqueo (`null` = centrada) y, si la familia los tiene, sus parámetros de ejemplar (`values`) | `IfcOpeningElement` + `IfcDoor` / `IfcWindow` |
| `dimensions` | Cotas de planta (versión 2): planta, puntos medidos (`points`), dirección de medida (`direction`, vector unitario) y un punto por el que pasa la línea de cota (`position`) | No se exporta (anotación) |
| `elevation_marks` | Cotas de nivel (versión 2): planta y punto. El valor no se guarda: se lee del forjado que hay debajo | No se exporta (anotación) |
| `dimensions[].anchors` | (versión 7) A qué está atado cada punto de la cota (`null`: a nada): `{"kind": "wall_end", "wall", "start", "side"}` (esquina de un extremo de muro en su cara `exterior`/`interior` o el extremo de su eje `axis`), `{"kind": "jamb", "opening", "end", "side"}` (jamba de puerta o ventana) o `{"kind": "slab_corner", "slab", "index"}`. Sin `anchors`, la cota es libre | Cota asociativa |
| `draw_layers` | Capas de dibujo (versión 7): nombre único, color, grosor (mm de papel), tipo de línea, visible, bloqueada, imprimible. La capa «0» (`id` 0) existe siempre | Capas de AutoCAD / ArchiCAD |
| `drafts` | Dibujo 2D (versión 7): vista (`{"level": id}` o `{"sheet": id}`), capa, forma (`shape` con `kind`: `line`, `polyline` con `bulges` como DXF, `circle`, `arc`, `ellipse`, `point`, `text`, `hatch`, `angle_dim`, `radius_dim`, `diameter_dim`, `arc_length_dim`) y, si no son «PorCapa», color, grosor y tipo de línea propios. Coordenadas en metros del modelo (en un plano, metros de papel); alturas de texto en mm de papel | Líneas de detalle, textos, regiones rellenas y cotas de AutoCAD |
| `graphics` | Normas gráficas (versión 7, opcional): plumillas finas (1:20, 1:50) y gruesas (desde 1:`coarse_from`), en mm de papel | Juego de plumillas |
| `info` | Datos del cajetín (versión 7, opcional): estudio, promotor, emplazamiento, autor, nº de expediente, fase, fecha | Información de proyecto |
| `sheets` | Planos (versión 7): número, nombre, papel en mm (`width`, `height`), si lleva marco y cajetín, revisión | Plano de Revit / Libro de planos de ArchiCAD |
| `viewports` | Vistas colocadas en planos (versión 7): plano, planta, escala, plano de corte, recorte opcional (`crop`), centro en el papel (m), título, si se ve el título y qué muestra (`style`: categorías y capas ocultas, nivel de detalle) | Vista en plano / Dibujo colocado |
| `rooms` | Locales (versión 7): `guid`, planta, número, nombre, uso, contorno y huecos, punto del rótulo (`label`), si sigue a los muros (`bounded`), altura libre propia (sin ella, hasta la planta superior) y acabados (`floor`, `walls`, `ceiling`) | `IfcSpace` |
| `tags` | Etiquetas (versión 7): planta, elemento (`{"kind": "opening"/"wall"/"room", "id"}`), posición y si lleva línea de referencia | Etiqueta |
| `tables` | Tablas colocadas en planos (versión 7): plano, clase (`openings`, `walls`, `slabs`, `rooms`), planta (sin ella, todo el edificio) y esquina superior izquierda en el papel (m) | Tabla de planificación / Listado interactivo |

Las referencias entre elementos van por `id`: un muro con `"type": 7` usa el tipo de muro cuyo `id` es 7.

Valores de parámetros (`values`): longitudes en metros (`"width": 0.8`), números, `true`/`false`, textos (opciones de una lista) y materiales por su `id` (`"frame_material": 12`). Al abrir se comprueban contra la familia: un valor fuera de rango (`"leaves": 5` en una puerta de 1 o 2 hojas) o una regla de la receta que no se cumple impiden abrir, diciendo qué tipo y por qué.

Ejemplo de tipo:

```json
{
  "id": 40,
  "guid": "0aBtlXbPAoAjdMxCGUZic",
  "name": "Puerta de entrada 1 hoja 100×215",
  "mark": "P-01",
  "description": "",
  "family": 90,
  "values": {
    "frame_depth": 0.1,
    "frame_material": 17,
    "frame_width": 0.07,
    "height": 2.15,
    "leaves": 1.0,
    "panel_material": 17,
    "width": 1.0
  }
}
```

## Editar a mano

Se puede, con cuidado: cambiar una altura, un nombre, mover una ventana, cambiar el tipo de un muro o un valor de un tipo de puerta. La receta de una familia también se puede retocar, pero entonces conviene subir su `version` (en la receta y en el campo `version`, que deben coincidir). Al abrir, ArchiSim avisa si algo no cuadra (por ejemplo, una ventana que pisa otra o un campo mal escrito, con su línea y columna). No dupliques elementos copiando y pegando sin cambiar su `id` y su `guid`: se rechazan por repetidos.
