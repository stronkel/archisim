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

9. **Tanda 9, época, estado y más regiones** (hecha, en revisión):
   - **Época**: cada lugar tiene su versión **de hoy**: cubierta plana (los nórdicos, Japón, Centroeuropa y la montaña, inclinada de zinc), huecos grandes con correderas, terrazas de losa, sin molduras ni rejas. Conserva el color del sitio (blanco en el sur, hormigón y madera en el norte) y la persiana enrollable en España.
   - **Estado**:
     - **abandonado**: ventanas tapiadas con tablero o vacías, sin flores, ropa ni visillos, el revoco envejecido, la humedad subiendo por el zócalo y hierbas al pie;
     - **en obra**: estructura de hormigón con los pilares de la última planta esperando con sus barras, ladrillo en la mitad de abajo con los huecos de las ventanas, andamio con plataformas y red a la calle, valla de obra y, desde tres plantas, la grúa torre.
   - **Seis lugares más de España**: montaña cantábrica (casa montañesa con solana), Valencia y su huerta (con la barraca de cubierta de paja como vivienda aislada), La Mancha, Cuenca (casas colgadas con balcones de madera), Extremadura y Baleares (piedra de marés, arcos y persianas mallorquinas). Al traer el entorno, cada uno sale por su zona.
   - Época y estado se eligen en las recetas («Época», «Estado») y en Propiedades → «Edificio del mundo».

10. **Tandas 10 y 11, equipamientos y comercio de cada lugar** (hechas, en revisión): una sola familia, «Equipamiento del mundo», con el parámetro «Programa» y los diecinueve programas de abajo. Todos llevan la piel del lugar elegido: sus colores, ventanas, cubiertas y remates. Los que son un edificio de calle se hacen con la misma fachada que las viviendas del lugar. Las naves, las gradas, los puestos y los monumentos se dibujan aparte con sus colores. Cada uno puede ser de hoy, abandonado o en obra, como las viviendas.
   - **Equipamientos**:
     - ayuntamiento: balcón de honor con tres banderas, rótulo, ático con reloj y espadaña con campana, o torrecilla fuera del mundo hispano; soportales donde el lugar los tiene;
     - colegio: pabellón con ventanas anchas y patio con pista, canastas, valla, mástiles y árboles;
     - biblioteca: planta baja acristalada, marquesina, rótulo y aparcabicis;
     - parque de bomberos: nave de autobombas con portones rojos, oficinas con la fachada del lugar, torre de maniobras y explanada;
     - comisaría: rótulo azul, bandera, bolardos y coches;
     - polideportivo: zócalo, chapa, banda de luz y bóveda de chapa, con vestíbulo acristalado;
     - gimnasio: de calle, con gran rótulo;
     - estadio: campo con sus líneas y porterías, gradas por los cuatro lados, tribuna cubierta y torres de focos;
     - fábrica: nave de ladrillo con dientes de sierra, muelle de carga, oficinas del lugar y chimenea.
   - **Comercio**:
     - quiosco: octogonal, con sus revistas;
     - tienda: con toldos y género en la acera;
     - mercadillo: puestos con lonas a rayas, fruta, ropa y cacharros;
     - galería comercial: pasaje con tiendas a los dos lados y bóveda de vidrio sobre arcos de hierro;
     - centro comercial: caja con atrio de vidrio, rótulos y aparcamiento con coches, farolas y árboles.
   - **Hostelería**:
     - chiringuito: tarima sobre la arena, chamizo de tablas con barra, techo de paja y mesas con sombrilla;
     - restaurante: con terraza, jardineras y sombrillas;
     - taberna: con su rótulo colgado, el farol y toneles como mesas.
   - **Plaza**: fuente en el centro, árboles, bancos, farolas y templete de música.
   - **Edificio emblemático de cada lugar**:
     - Andalucía: torre como la Giralda, con su cuerpo de campanas y la linterna;
     - Italia, Levante, Valencia y Baleares: campanario con chapitel de teja;
     - Castilla, Extremadura, Cuenca, la montaña y el Algarve: torre con aguja de piedra;
     - Galicia, Lisboa y la América colonial: iglesia de dos torres con cúpulas; Centroeuropa, con agujas; el Este, con cebollas;
     - California: misión con espadaña de tres campanas y soportal;
     - La Mancha: tres molinos; Países Bajos: molino con su balconada;
     - París, Estados Unidos y San Francisco: cúpula sobre tambor con pórtico de columnas;
     - Londres y el trópico: torre del reloj;
     - Grecia: templo;
     - China, Japón y Corea: pagoda de cinco cubiertas;
     - Magreb: mezquita con su alminar;
     - países nórdicos: iglesia de madera;
     - Nueva York: rascacielos de los años 30.
   - Para verlos: `archisim equipamientos out/ andalucia japon` (un proyecto por lugar). Fotos con `ARCHISIM_EQUIPAMIENTOS_OUT=carpeta cargo test --release -p archisim-app -- facility_pictures` (cinco hojas por lugar; `ARCHISIM_MUNDO` para elegir lugares).
   - El quiosco cambia con el lugar (tanda 12): de prensa con las trampillas abiertas en España; con cono en Lisboa, Italia y Grecia; verde con cúpula y faldón festoneado en París, Centroeuropa y el Este; con cubierta oriental en China, Japón y Corea; caja de metal con su banda luminosa en América; de madera con pirámide en el norte; encalado con cupulita en el Magreb; de paja en el trópico.
   - En Japón y Corea, los edificios públicos se encalan y dejan la madera oscura para la carpintería y los recercados (antes salían negros).
   - **Al traer el entorno** (tanda 12), cada edificio toma su programa por las etiquetas de OpenStreetMap o el uso del Catastro: ayuntamiento, colegio (también guardería e instituto), biblioteca, bomberos, comisaría, mercado, restaurante y café, bar, templo; polideportivo, gimnasio y estadio; quiosco, fábrica (industrial de más de 300 m²); y el comercio por su tamaño: quiosco por debajo de 25 m², tienda hasta dos plantas y centro comercial por encima de 2.500 m². Se puede cambiar a mano: en las propiedades del edificio, tipología «Equipamiento o comercio» y «Programa».

11. **Tanda 13, plazas y espacios públicos** (hecha, en revisión): la plaza ya es la de cada lugar (plaza mayor porticada, plaza de naranjos, piazza, plaza del mercado, plaza de armas, explanada de templo, green americano y plaza de la medina) y hay cuatro programas nuevos: parque de barrio con juegos, paseo marítimo, alameda y jardín con invernadero.
12. **Tanda 14, lo que remata un pueblo** (hecha, en revisión): cementerio, ermita, faro, lonja y puerto, estación de tren, estación de autobuses, hospital, hotel, puente de piedra y casa cuartel.
13. **Tanda 15, arquitectura popular que faltaba** (hecha, en revisión): hórreo (gallego, y asturiano en la montaña), palloza, casa de piedra de sierra, casa-patio de una planta y bloque de VPO de ladrillo.
   - Son programas del mismo «Equipamiento del mundo» (del 19 al 37) y llevan la piel del lugar. Se ven con `archisim equipamientos out/ <lugares>`; fotos con `ARCHISIM_HOJAS=6,7,8,9,10` para las hojas nuevas.
   - Al traer el entorno se reconocen también hospitales, hoteles, ermitas, estaciones, faros y cementerios.
14. **Tanda 16, colocar los edificios a mano, como en CityRT** (hecha, en revisión): para cuando no se quiere traer el entorno ni dibujar parcelas.
   - Herramienta «Edificio del mundo» (Entorno, atajo EM) y, en la Maqueta, Exterior → «Edificios y calles»: una galería en tarjetas con el edificio en 3D, agrupada (viviendas, arquitectura popular, equipamientos, deporte, comercio y hostelería, plazas y parques, transporte y puerto, monumentos), con el lugar elegible.
   - Cada tipología trae sus medidas habituales (entre medianeras: el ancho del lugar × 12 m; bloque 22 × 14; aislada 10 × 8; torre 24 × 24; casa patio 16 × 18; los equipamientos, las de su programa) y sus plantas; se cambian antes de colocar (Fachada, Fondo, Plantas) o después, como cualquier volumen.
   - Un clic lo pone: si hay una calle a menos de 25 m, con la fachada en el borde de la acera mirándola; si no, como esté girado (Espacio gira 90°). Con «Dos clics», el primero y el segundo marcan la fachada.
   - La fachada principal se guarda en el edificio (formato 78) y manda sobre la que se deduce de las calles.
   - Las calles se dibujan con la herramienta de siempre (también en la ficha «Calle» junto a la galería).
15. **Remates técnicos** (hechos con la tanda 16):
   - Las casas profundas entre medianeras ya no quedan con cubierta plana: un faldón a la calle y el resto plano detrás.
   - Las tejas se ven también de lejos: la versión ligera lleva un escalón cada dos hiladas (U6) o bandas (edificios del mundo).
   - La plaza de toros (de 95.000 a 62.000 triángulos) y el corral (de 124.000 a 81.000) más ligeros: pomos, flores y frutas de pocas caras, balaustres cuadrados.
   - La ortofoto se ve como imagen de verdad, píxel a píxel, en la vista 3D, en el glTF y en el render.
   - El modelo de superficies (MDS) del recuadro del proyecto se pide al WCS del IGN con el terreno, para las plantas de los edificios.

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
- **Los lugares:**
  - España: Andalucía; Castilla y Madrid; Galicia y el Cantábrico; Cataluña y Levante; y desde la tanda 9 montaña cantábrica, Valencia, La Mancha, Cuenca, Extremadura y Baleares (30 lugares en total).
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
  - los materiales, uno por papel, están en `world::MATERIALS`;
  - la época de hoy es `skins::contemporary` y los estados están en `world::state` (`abandon`, `site_on`);
  - los equipamientos están en `world::civic` (`Programme`, `facility`) y los edificios emblemáticos en `world::landmarks` (`kind_of` por lugar);
  - las plazas por lugar, los parques, paseos, alamedas y jardines en `world::places` (`square_kind`), lo que remata un pueblo en `world::town` y la arquitectura popular en `world::vernacular`; la receta la escribe `scripts/make_world_recipes.py`.
