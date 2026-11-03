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
| Casa patio, pareada, chalet, oficinas, comercio con viviendas, nave, equipamiento | Siguientes tandas |

## 3. Orden de las tandas

1. **Tanda 1** (hecha, en revisión): las seis de arriba.
2. **Andalucía y costa de Huelva:** corral de vecinos, cortijo, pueblo de colonización, chalet de los 80 con arcos, pareada, casa-patio, VPO de ladrillo, nave de polígono, chiringuito.
3. **Usos singulares:** mercado de abastos, iglesia parroquial y ermita, plaza de toros, colegio, centro de salud, gasolinera.
4. **Resto de España por regiones:** Castilla con soportales, caserío, galería gallega, masía, casa canaria, casa ibicenca.
5. **Extranjero:** Haussmann, casa de canal de Ámsterdam, hilera inglesa, nórdico, suburbio americano, japonés, rascacielos.

Cuando Antonio apruebe una tanda, la integración será que los estilos de U6 (y U7, al traer el Catastro) usen estos modelos sobre la planta y las plantas que tenga cada masa.
