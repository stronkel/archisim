# Formato de proyecto `.archisim` (versión 1)

El proyecto de ArchiSim se guarda en un único fichero de texto JSON con extensión `.archisim`. Es el equivalente al `.pln` de ArchiCAD o al `.rvt` de Revit, pero legible con cualquier editor de texto y comparable entre versiones (Git, diff).

## Principios

- **Todo en metros.** El campo `"units": "m"` lo deja escrito. La interfaz mostrará milímetros, pero el fichero guarda metros.
- **Identidad estable.** Cada elemento conserva su `id` interno y su `guid` (el `GlobalId` IFC de 22 caracteres). Guardar, abrir y volver a exportar da **el mismo IFC**: los ingenieros pueden federar y comparar versiones sin que los elementos "cambien de identidad".
- **Versionado.** `"version": 2` es la versión del formato, no del programa. ArchiSim actualiza los ficheros antiguos al abrirlos (la versión 1, de ArchiSim 0.4 a 0.9, se abre añadiendo listas de cotas vacías); un fichero guardado por un ArchiSim más nuevo se rechaza con un aviso claro en vez de abrirse mal.
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
| `walls` | Muros: `type`, `level`, `start` y `end` como `[x, y]`, línea de referencia (`center`, `exterior_face`, `interior_face`), altura y desfase de la base | `IfcWall` |
| `slab_types` | Tipos de forjado: función (`floor`, `base_slab`, `roof`) y capas **de arriba abajo** | `IfcSlabType` + capas |
| `slabs` | Forjados: contorno (lista de `[x, y]`), nivel y desfase de la cara superior | `IfcSlab` |
| `opening_types` | Tipos de puerta o ventana: `category` (`door` / `window`), funcionamiento, hueco de obra, perfil y materiales del marco y de la hoja o vidrio | `IfcDoorType` / `IfcWindowType` |
| `openings` | Puertas y ventanas: muro, posición del centro (`along`), antepecho (`sill`), lado de apertura, mano (UNE-EN 12519) y retranqueo (`null` = centrada) | `IfcOpeningElement` + `IfcDoor` / `IfcWindow` |
| `dimensions` | Cotas de planta (versión 2): planta, puntos medidos (`points`), dirección de medida (`direction`, vector unitario) y un punto por el que pasa la línea de cota (`position`) | No se exporta (anotación) |
| `elevation_marks` | Cotas de nivel (versión 2): planta y punto. El valor no se guarda: se lee del forjado que hay debajo | No se exporta (anotación) |

Las referencias entre elementos van por `id`: un muro con `"type": 7` usa el tipo de muro cuyo `id` es 7.

Funcionamientos válidos: puertas `single_swing`, `double_swing`, `sliding`; ventanas `fixed`, `single_casement`, `double_casement`, `sliding`.

## Editar a mano

Se puede, con cuidado: cambiar una altura, un nombre, mover una ventana o cambiar el tipo de un muro. Al abrir, ArchiSim avisa si algo no cuadra (por ejemplo, una ventana que pisa otra o un campo mal escrito, con su línea y columna). No dupliques elementos copiando y pegando sin cambiar su `id` y su `guid`: se rechazan por repetidos.
