# Traer el entorno (H7)

Para no modelar a mano lo que rodea al proyecto, ArchiSim lee cartografía gratuita y la convierte en elementos suyos:

| De dónde | Qué fichero | Qué sale en ArchiSim |
|---|---|---|
| **Catastro** (Sede Electrónica, descarga INSPIRE por municipio) | GML de edificios con sus partes (`buildingpart`) y GML de parcelas (`cadastralparcel`) | Cada parte de edificio, un **volumen de Concepto por plantas** con el número de plantas sobre rasante del Catastro (3 m por planta, se cambia); cada parcela, una **Parcela** con su referencia catastral |
| **OpenStreetMap** (openstreetmap.org → Exportar) | `.osm` de la zona | Las calles, como **Viales** con el tipo de vía según su clase (ver abajo); los árboles sueltos y las filas de árboles, como **árboles de la biblioteca**; los edificios, solo si no hay Catastro |
| **CNIG** (Centro de Descargas → MDT05 o MDT02) | Hoja en formato ASC | Un **Terreno** con sus puntos dentro del radio (unos 4 000 como mucho) |

Todo es gratuito y de uso libre: el Catastro con su licencia de reutilización, OpenStreetMap con la ODbL (hay que citar «© colaboradores de OpenStreetMap» en lo que se publique) y el CNIG con la CC BY 4.0 («© Instituto Geográfico Nacional»).

## Cómo se usa

- En la aplicación: **Archivo → Traer el entorno (Catastro, OpenStreetMap, MDT)…**, se eligen uno o varios ficheros (o se sueltan en la ventana). La ventana dice qué es cada uno y deja elegir qué traer (edificios, parcelas, calles, árboles, terreno), el radio alrededor del origen y la altura de planta. Se deshace en un paso.
- En la línea de órdenes: `archisim cartografia <ficheros…> [--proyecto p.archisim] [--radio 150] [--centrar] [--salida s.archisim]`.

## Dónde se coloca

- Si el proyecto está georreferenciado (propiedades del proyecto, sin nada seleccionado: «Emplazamiento (georreferencia)»), cada cosa va a su sitio con esa georreferencia y el norte del proyecto, igual que en el IFC.
- Si no lo está, el proyecto toma la del Catastro (su sistema, EPSG:25830 por ejemplo) o, con solo OpenStreetMap, la del huso UTM de la zona en ETRS89, con el origen en el centro de lo que traen los ficheros y la cota ±0,00 en el terreno de ese punto si hay MDT.
- Con «Llevar el origen del proyecto al centro de lo que traen los ficheros» (`--centrar`), el proyecto se mueve allí aunque ya estuviera georreferenciado.
- Los edificios que pisan los forjados del proyecto no se traen (se puede desmarcar).
- Los edificios se apoyan en el terreno; las calles van a la cota media del terreno a lo largo de ellas.

## Tipos de vía según OpenStreetMap

| Clase en OSM | Tipo de vía |
|---|---|
| `primary`, `secondary`, `trunk` | Avenida con mediana arbolada |
| `tertiary`, `residential`, `unclassified` | Calle de doble sentido con aparcamiento (o de un sentido, si es `oneway`) |
| `living_street`, `pedestrian` | Calle de plataforma única |
| caminos, sendas, pistas, autovías | No se traen |

Si la calle no cabe entre las parcelas (o los edificios) de los dos lados, se estrecha: primero a una calle de doble sentido con aparcamiento, luego de un sentido, luego sin aparcamiento y, si aún no cabe, todas sus franjas en proporción. Después se edita como cualquier vial (H6).

## Lo que queda pendiente

- Las calles son horizontales: en una calle en pendiente quedan a la cota media (hace falta la rasante del vial).
- Los cruces entre calles no se resuelven: las calles se solapan en las esquinas.
- Las alturas de los edificios son por número de plantas, no por la altura real (el Catastro no la da; el LiDAR del PNOA sí, en una vuelta futura).
