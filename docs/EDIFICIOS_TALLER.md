# Taller de edificios del entorno

Los edificios del entorno (U6) se rehacen con el mismo cuidado que los muebles para que todo el modelo se vea igual de trabajado. Antes de sustituir nada, cada edificio nuevo se mira en su propio taller, frente al edificio del mismo tipo que ArchiSim hace hoy.

- `archisim edificios out/` crea `out/edificios.archisim`: una calle con los nuevos al sur, mirando al norte, y los de hoy enfrente.
- Fotos: `ARCHISIM_EDIFICIOS_OUT=carpeta cargo test --release -p archisim-app -- building_pictures`. Por cada edificio salen `NN-nuevo.png` y `NN-hoy-y-nuevo.png` (el de hoy a la izquierda). Con `ARCHISIM_EDIFICIOS=1,4` se hacen solo algunos.
- Para el programador:
  - el código está en `archisim_props::fine::buildings` (las herramientas comunes en `mod.rs`, un fichero por edificio);
  - las recetas, en `archisim-library/taller/edificio_*.asfam`;
  - el proyecto del taller, en `archisim_library::building_workshop`.

## 1. Cómo se ordena el catálogo

Cada edificio es una combinación de cinco ejes:

1. **Cómo ocupa la parcela.** Entre medianeras, casa-patio, en hilera, pareada, aislada, manzana cerrada, bloque abierto, torre, nave.
2. **Uso.** Vivienda, comercio y hostelería, oficinas, equipamiento, industria, campo, turismo, infraestructura.
3. **Época.** Tradicional, siglo XIX y ensanche, regionalismo y modernismo, racionalismo, posguerra, desarrollismo, años 80–90, contemporáneo.
4. **Región.** Andalucía, Levante, Cataluña, Castilla, Cantábrico, Galicia, Canarias, Baleares, Extremadura; después el resto del mundo.
5. **Estado.** Nuevo, habitado, en obra, abandonado.

Los ejes 1 y 2 dan el volumen; los ejes 3, 4 y 5 dan la piel. Hay detalles que hacen «español» cualquier edificio, y son parámetros, no edificios aparte:
- persiana enrollable con su cajón, rejas, cierros, toldos;
- aires acondicionados y tendederos en fachada;
- azotea con depósito y antenas;
- macetas, alicantinas, zócalos.

## 2. Lo que ya hay (U6) frente a la tanda 1

| U6 hoy (13 estilos) | Taller, tanda 1 |
|---|---|
| Casa andaluza entre medianeras | **Casa de pueblo andaluza**: 1 a 3 plantas, cierro o rejas voladas, balcón de forja, macetas, farol, alero de dos hiladas de teja |
| Casco histórico | **Casa señorial del casco**: portada de piedra con escudo, planta noble con guardapolvos, sillares en las esquinas, cornisa sobre canecillos, pretil con bolas, torre mirador |
| Ensanche | **Edificio de ensanche**: arcos de sillería con abanicos, miradores de madera, alicantinas, ático con balaustrada |
| Bloque de los 60–70 | **Bloque desarrollista**: ladrillo visto, terrazas voladas (unas cerradas con aluminio, otras con el toldo verde bajado), escalera entre plantas, locales con cierre, azotea con antenas |
| Hotel de costa | **Torre de apartamentos de costa**: soportales, terrazas corridas con aletas, barandilla de fábrica o de vidrio, toldos a rayas, placas solares |
| Adosada | **Adosados de promotora**: jardín delantero con murete y cancela, garaje, porche, muros cortafuegos sobre la cubierta |
| Casa patio | **Corral de vecinos** (tanda 2): patio alargado con galería de hierro en tres lados bajo cubiertas a un agua, zaguán con cancela, escalera, ropa tendida, naranjo, pila. Y **cortijo** (tanda 2): casa de dos plantas al fondo del patio de albero, naves con portones, torre de contrapeso, portada con espadaña, pozo, abrevadero, olivos y palmera |
| Casa andaluza (otra variante) | **Viviendas de colonización** (tanda 2): pieza alta y pieza baja con cubiertas a un agua opuestas, huecos hondos, marcos de color, celosía de ladrillo, chimenea alta, corral con portón y cobertizo |
| Chalet | **Chalet de los 80** (tanda 2): ladrillo visto en esquinas y recercados, porche de tres arcos con terraza y balaustrada, mallorquinas, cuatro aguas, garaje, murete con pilastras, palmera y piscina |
| Pareada | **Viviendas pareadas** (tanda 2): una cubierta a cuatro aguas para las dos, porches con columnas, miradores, balcones con toldo, jardines con setos y cancelas |
| Nave industrial | **Nave de polígono** (tanda 2): bloque y panel nervado, hastial con rótulo, puerta seccional con portillo, muelle de carga con abrigo, oficinas con ventanas corridas, policarbonato, lucernarios, valla de malla |
| Equipamiento | **Mercado de abastos**, **iglesia parroquial**, **plaza de toros**, **colegio público**, **centro de salud** (tanda 3) |
| Comercio | **Gasolinera** sin marca (tanda 3) |
| Casco histórico, chalet, casa patio (otras regiones) | **Casa castellana con soportales**, **caserío vasco**, **casa con galería gallega**, **masía catalana**, **casa canaria**, **casa payesa ibicenca** (tanda 4) |
| Oficinas | **Torre de oficinas** de muro cortina con plaza y marquesina, **oficinas de los 70** con antepechos prefabricados, parasoles y pilotis, **parque empresarial** con aparcamiento cubierto (tanda 5) |
| Comercio con viviendas | **Cafetería de esquina** con toldos y terraza, **comercio con viviendas** (farmacia, banco, panadería o moda; cuatro colores de fachada), **hotel urbano** con marquesina, banderas y terraza en la azotea, **supermercado** con aparcamiento (tanda 5) |

## 3. Orden de las tandas

1. **Tanda 1** (hecha, aprobada por Antonio el 03/11/2026 como salto de calidad): casa de pueblo, casa señorial, ensanche, bloque de los 60, torre de costa, adosados.
2. **Tanda 2** (hecha, en revisión): corral de vecinos, cortijo, viviendas de colonización, chalet de los 80, pareadas, nave de polígono. Pendientes de Andalucía para otra tanda: casa-patio de una planta, VPO de ladrillo, chiringuito.
3. **Tanda 3, usos singulares** (hecha, en revisión): mercado de abastos, iglesia parroquial, plaza de toros, colegio, centro de salud, gasolinera. Quedan para otra: ermita, ayuntamiento, casa cuartel, cementerio, estación de autobuses.
4. **Tanda 4, regiones de España** (hecha, en revisión): casa castellana con soportales, caserío vasco, casa con galería gallega, masía catalana, casa canaria, casa payesa ibicenca. Quedan: casa montañesa con solana, hórreo y palloza, barraca valenciana, casa manchega, casas colgadas, casa de piedra de sierra.
5. **Tanda 5, ciudad y comercio** (hecha, en revisión): torre de oficinas, oficinas de los 70, parque empresarial, cafetería de esquina, comercio con viviendas en cuatro versiones, hotel urbano, supermercado.
6. **Tanda 6, pieles por lugar** (hecha, en revisión): tres tipologías (casa entre medianeras, bloque exento y vivienda aislada) vestidas con la piel de 24 lugares (§4).
7. **Tanda 7, por sitio** (hecha, en revisión): al traer el entorno (Catastro, OpenStreetMap, LiDAR), cada edificio toma la tipología por su estilo, sus plantas y sus medianeras, y la piel por la zona de la georreferencia. Antes se hicieron las plantas poligonales y la versión ligera. Todo se puede cambiar a mano en Propiedades, en «Edificio del mundo»: lugar, tipología y variante.

8. **Tanda 8, más tipologías con piel** (hecha, en revisión):
   - la **torre** de cada lugar, de tres maneras:
     - viviendas apiladas con los balcones del sitio (Andalucía, Levante, Latinoamérica, Corea, el Este…);
     - muro cortina de vidrio con su vestíbulo y la pantalla de sus máquinas (París, Londres, Centroeuropa, Estados Unidos, China, Japón, Dubái);
     - el ladrillo y la piedra de Nueva York, que se retranquean hasta su corona y su aguja, con el depósito de agua;
   - la **casa patio**: crujías de delante y de detrás enteras y las laterales entre ellas, con la piel de la vivienda del lugar (patio andaluz, riad, siheyuan, casona colonial) y el patio empedrado con su pozo, sus macetas y su naranjo;
   - las **casas en hilera**, cada una de su color, con jardín delantero, camino y la valla, el seto o la verja del sitio.
   - Al traer el entorno, la torre sale con diez plantas o más (u oficinas desde seis) y la casa patio con el estilo «casa patio».

Cuando Antonio apruebe una tanda, la integración será que los estilos de U6 (y U7, al traer el Catastro) usen estos modelos sobre la planta y las plantas que tenga cada masa.

## 4. Las pieles por lugar (tanda 6)

Una **piel** dice con qué y cómo construye un lugar; las **tipologías** son el volumen. Con eso, un edificio traído de un mapa puede tomar la piel de donde está (tanda 7) y cambiarse luego a mano con el parámetro «Lugar».

- **Tipologías:** «Casa entre medianeras del mundo», «Bloque de viviendas del mundo» y «Vivienda aislada del mundo» (carpeta «Taller · Edificios del mundo»). Cada una tiene «Lugar», «Variante», fachada, fondo y plantas.
- **Qué lleva cada piel:**
  - colores de fachada, zócalo, recercados, carpintería, contraventanas y puertas: cada edificio elige uno de la lista por su variante;
  - las ventanas: abatibles, de guillotina o correderas, con sus cuarterones, y lo que las cierra (persiana enrollable, contraventanas de lamas o de tablas);
  - el remate del hueco: recercado, guardapolvo, frontón, arco o clave;
  - los balcones: de forja, corridos, de madera con tejadillo, galería, mirador, escalera de incendios, verandas de encaje, celosías, jaulas, terrazas acristaladas o balcones corridos con antepecho;
  - la planta baja: puerta, comercio, escalinata, soportales, garaje, celosía de madera o pilotis con coches;
  - la cornisa;
  - la cubierta: plana con pretil o balaustrada, a dos aguas, hastial a la calle escalonado o de campana, a cuatro aguas, mansarda con buhardillas o la oriental. Va de teja curva, en hiladas (pizarra, teja plana, tejuela) o de chapa con sus juntas;
  - lo que cada sitio pone en la fachada y en la azotea: aires acondicionados, ropa tendida, jardineras, depósitos, chimeneas con sus cañones, la chimenea algarvía, el depósito de madera de Nueva York, los calentadores solares, rótulos verticales…
- **Los 24 lugares:**
  - España: Andalucía; Castilla y Madrid; Galicia y el Cantábrico; Cataluña y Levante.
  - Península: Lisboa y Portugal; Algarve.
  - Europa: París; Italia; Reino Unido; Centroeuropa; Países Bajos y Bélgica; Países nórdicos.
  - América: Nueva York y Chicago; Estados Unidos rural; California; San Francisco; Latinoamérica colonial.
  - Asia: China; Japón; Corea.
  - Otros: Grecia y el Mediterráneo oriental; Magreb y Oriente Medio; Europa del Este; Australia y el trópico.
- **Para verlos:**
  - `archisim mundo out/` crea `out/mundo.archisim`, con una fila por lugar: tres casas de su calle, su bloque y su vivienda aislada. Con `archisim mundo out/ paris japon` saca solo esos lugares.
  - Fotos: `ARCHISIM_MUNDO_OUT=carpeta cargo test --release -p archisim-app -- world_pictures`. Hace una hoja por lugar; con `ARCHISIM_MUNDO=paris,japon` solo esos.
- **Para el programador:**
  - las pieles son datos en `archisim_props::fine::buildings::world::skins` (`SKINS`);
  - la fachada está en `world::facade`, la cubierta en `world::roofs` y las tipologías en `world::types`;
  - los materiales, uno por papel, están en `world::MATERIALS`.
