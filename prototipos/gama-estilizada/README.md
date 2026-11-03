# Gama de materiales estilizada (prototipo)

Prueba, fuera de la aplicación, de una gama de materiales con aspecto **estilizado**: a medio camino entre el realista y el de dibujos animados, con colores vivos, al estilo de Paralives y CityRT. Es una página web (three.js) para decidir el aspecto **antes** de meterlo en ArchiSim.

Se ha tomado de esos programas el estilo, no sus texturas ni sus modelos. Todo lo dibuja nuestro código.

## Cómo está hecho (y por qué se puede pasar tal cual a ArchiSim)

Usa el mismo método que la vista 3D de ArchiSim (`view3d/shaders.wgsl`, función `textured`): cada textura se calcula desde la posición del punto sobre su cara. No hay imágenes ni nada que desplegar. Lo que añade la gama:

1. **Relieve por pieza.** Cada material devuelve, además del color, una altura pequeña en metros: el bisel del ladrillo, el labio de la tabla solapada, la teja curva. Esa altura inclina la normal (con las derivadas de pantalla, `dpdx`/`dpdy` en WGSL), así que la luz del sol marca cada pieza. Es lo que da el aspecto «de maqueta» de CityRT.
2. **Pocos tonos por pieza** (`tones`): cada ladrillo, tablón o teja toma uno de 3 o 4 tonos, como un pintor, en lugar de un ruido continuo de foto. Hay alguna pieza quemada o más clara.
3. **Ejes del faldón en las cubiertas:** las hiladas van paralelas al alero y los labios miran hacia abajo, en cualquier pendiente. Hoy ArchiSim proyecta las caras inclinadas en planta o en alzado.
4. **Ejes alrededor de un eje** para torres y conos (`uMap = 1`): las piezas siguen la vuelta.
5. **De lejos, el color medio:** cuando la pieza ocupa menos de unos píxeles se funde en su color medio y pierde el relieve. Así no hay muaré, y es lo que pasaba con las hiladas de teja de la entrada 84 de la bitácora.
6. **Luz de ilustración:**
   - el lado iluminado envuelve un poco más y su borde es más nítido, como una sombra pintada;
   - las sombras son frías (cielo azulado) y las luces cálidas;
   - un pie de muro algo más oscuro;
   - contornos del color de la superficie oscurecido, en lugar de negros.

## Los materiales

- **Fachadas:** ladrillo rojo y amarillo, sillería, mampostería, estuco amarillo y almagra, encalado, tablas solapadas azul y verde, hormigón suave.
- **Cubiertas:** tejuela de pizarra y verde, escamas, teja curva, cobre con pátina, zinc.
- **Suelos:** adoquín, losa de acera, asfalto, tarima, césped (con briznas, flores y matas en 3D), roca en estratos, agua de mar y de piscina (con movimiento y transparencia), gresite.
- **Pavimentos:** adoquín en diagonal y en abanico, empedrado, empedrado antiguo, opus romano, laja, laja en césped, mármol, guijarros y grava.
- **Detalles:** pinturas crema, verde carruaje, chocolate y granate, vidrio pintado, follaje, madera de poste.

Colores y tamaños de pieza en `src/palette.js`. Las funciones de cada material en `src/materials.glsl.js`, que también lleva las texturas actuales de ArchiSim para compararlas (botón «Actuales»).

## Construir y mirar

```bash
npm install
sh build.sh          # out/index.html, una sola página con todo dentro
# capturas sin pantalla (Playwright):
node shot.js '[{"name":"calle"},{"name":"muestrario","view":"board"}]'
```

## En ArchiSim

Llevado a la vista 3D en la rama `bloque/gama-estilizada` (entrada 86 de la bitácora). Lo que sigue es el resumen de cómo se hizo.

## Para llevarlo a ArchiSim

- **Materiales:** `TextureKind` nuevos o una variante «estilizada» de los actuales en `model/appearance.rs`. El tamaño de pieza (`dims`) va en la escala que ya hay o en un campo nuevo, con su número de versión del formato.
- **Sombreador:**
  - las funciones de `materials.glsl.js` pasan a `shaders.wgsl` casi línea a línea;
  - la altura de relieve se suma con `dpdx`/`dpdy` en `fs_model`;
  - `textures_light.wgsl` y `textures_image.wgsl` deben dibujar lo mismo;
  - `archisim_render::textures` las hornea para el render y el glTF.
- **Luz:** la curva de `shade()` y el tinte de las sombras.
