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
| Oficinas, comercio con viviendas | Siguientes tandas |

## 3. Orden de las tandas

1. **Tanda 1** (hecha, aprobada por Antonio el 03/11/2026 como salto de calidad): casa de pueblo, casa señorial, ensanche, bloque de los 60, torre de costa, adosados.
2. **Tanda 2** (hecha, en revisión): corral de vecinos, cortijo, viviendas de colonización, chalet de los 80, pareadas, nave de polígono. Pendientes de Andalucía para otra tanda: casa-patio de una planta, VPO de ladrillo, chiringuito.
3. **Tanda 3, usos singulares** (hecha, en revisión): mercado de abastos, iglesia parroquial, plaza de toros, colegio, centro de salud, gasolinera. Quedan para otra: ermita, ayuntamiento, casa cuartel, cementerio, estación de autobuses.
4. **Resto de España por regiones:** Castilla con soportales, caserío, galería gallega, masía, casa canaria, casa ibicenca.
5. **Extranjero:** Haussmann, casa de canal de Ámsterdam, hilera inglesa, nórdico, suburbio americano, japonés, rascacielos.

Cuando Antonio apruebe una tanda, la integración será que los estilos de U6 (y U7, al traer el Catastro) usen estos modelos sobre la planta y las plantas que tenga cada masa.
