# Terreno y movimiento de tierras

El terreno se hace con puntos de cota, líneas de nivel o un levantamiento importado (DXF, CSV), con su contorno dibujado a mano (E5). Este documento trata de las **explanaciones**: las plataformas y rampas que se excavan o rellenan en él, su desmonte y su terraplén, y cómo salen en planta, en las mediciones, en el informe de tierras y en el IFC.

## 1. Plataformas y rampas

Una **plataforma** es una superficie horizontal a una cota, con un **talud** alrededor hasta encontrar el terreno (se da como H:V: 1 es 45°, 1,5 es 3:2). Una **rampa** es una plataforma inclinada: tiene su cota de arranque, su cota de llegada (o su pendiente) y su dirección.

Clases de plataforma (dicen cómo se escriben en el IFC):

| Clase | De dónde sale |
|---|---|
| Plataforma | Dibujada a mano (terraza, patio, aparcamiento, la de la piscina) |
| Plataforma del edificio | «Explanar bajo el edificio»: bajo cada forjado de la planta del terreno, a su cara inferior |
| Vaciado de sótano | «Explanar bajo el edificio»: bajo los forjados de las plantas de debajo |
| Rampa | Dibujada a mano con la herramienta Terreno, modo Rampa |

Donde dos plataformas se encuentran, **gana la más baja** (el vaciado del sótano dentro de la plataforma de la vivienda). «Explanar bajo el edificio» rehace solo las del edificio: las dibujadas a mano se quedan.

### Dibujarlas

Herramienta **Terreno** (tecla R), segunda fila de modos:

- **Plataforma**: con «Rectángulo», dos clics en esquinas opuestas; sin él, un polígono (un clic por vértice; se cierra con un clic en el primero, Intro o doble clic; Esc quita el último).
- **Rampa**: dos clics, de la arrancada a la llegada de su eje; su ancho en la herramienta (3,00 m por defecto).
- **Cota**: escrita, o (por defecto) **la del terreno** en el primer clic. En la rampa, la de arranque y la de llegada por separado.
- **Talud** (H:V), 1 por defecto.

Al terminarla se explana el terreno y la barra de avisos dice su desmonte y su terraplén.

### Cambiarlas

Con el terreno seleccionado:

- **Clic dentro de una plataforma** la elige (se marca en azul); **Supr** la quita.
- **Pinzamientos** en todas sus esquinas: arrastrándolos, el terreno se vuelve a explanar.
- En **Propiedades → Explanaciones**, cada plataforma con su desmonte y terraplén; al abrirla: nombre, cota (en las rampas, cota de arranque, cota de llegada, pendiente en % y dirección en grados), talud, desmonte y terraplén con su superficie, **Editar contorno** (en modo boceto, como los forjados) y Quitar. Abajo, el total, la compensación y el botón **Informe de tierras**.
- En **Propiedades → En planta**, «Desmonte y terraplén»: colorea en planta lo que se excava (rojo) y lo que se rellena (verde) y escribe junto a cada plataforma sus volúmenes.

## 2. Cómo se calcula

El terreno guarda el levantamiento (`ground`) y el terreno explanado (`points`). El desmonte y el terraplén salen de comparar las dos superficies en una rejilla de 50 cm (más gruesa en solares grandes, para no pasar de unos 250 000 nudos): cada triángulo de la rejilla donde el terreno cambia se da a la plataforma que lo ha hecho así (la que tiene dentro el punto, o cuyo talud lo sostiene; de esas, la más cercana). Cada desmonte y cada terraplén es un **sólido cerrado de triángulos** entre las dos superficies; su volumen es el de ese sólido.

Las piscinas (construidas o de la biblioteca) se vacían desde el terreno explanado hasta el fondo de su vaso, dentro del contorno exterior de sus muros.

Los volúmenes son **en perfil**: sin esponjamiento ni compactación.

## 3. En planta

- El contorno de cada plataforma en trazo y punto.
- El **talud**: la línea donde encuentra el terreno (cabeza del talud en desmonte, pie en terraplén) y las rayas de talud, largas y cortas alternas, desde la cabeza hacia el pie.
- Su nombre y su cota fuera de su lado más largo, pasado el talud; en las rampas, «de +a a +b · p %» y una flecha hacia donde sube.
- Con «Desmonte y terraplén» activado: las zonas coloreadas y «Desmonte … m³ · Terraplén … m³» bajo el nombre.

## 4. Mediciones e informe

Capítulo **12 Movimiento de tierras**, una línea por plataforma y piscina con su superficie en planta:

| Código | Partida | Lleva |
|---|---|---|
| T01 | Desmonte (excavación a cielo abierto) | Plataformas, plataformas del edificio y rampas |
| T02 | Terraplén (relleno compactado) | Todos los rellenos |
| T03 | Excavación en vaciado (sótanos y piscinas) | Vaciados de sótano y de piscina |

El **informe de tierras** (Propiedades del terreno, o `archisim tierras`) da cada plataforma con su cota y talud, cada desmonte, terraplén y vaciado con su clase, superficie en planta, profundidad máxima y volumen, y los totales con la **compensación** (lo que sobra para vertedero o falta de préstamo).

## 5. IFC

| ArchiSim | IFC 4.3 | Tipo predefinido |
|---|---|---|
| Desmonte de plataforma o rampa | `IfcEarthworksCut` | `CUT` |
| Vaciado de sótano o de piscina | `IfcEarthworksCut` | `EXCAVATION` |
| Terraplén | `IfcEarthworksFill` | `EMBANKMENT` |

- Cuerpo: `IfcTriangulatedFaceSet` cerrado, en coordenadas del emplazamiento.
- Cantidades: `Qto_EarthworksCutBaseQuantities` (`Length`, `Width`, `Depth`, `UndisturbedVolume`) y `Qto_EarthworksFillBaseQuantities` (`Length`, `Width`, `Depth`, `CompactedVolume`); largo y ancho son los de su rectángulo en planta, y la profundidad la mayor.
- Relaciones: el terraplén está **contenido en el `IfcSite`**. El desmonte, en IFC 4.3, es una sustracción (`IfcFeatureElementSubtraction`) y la norma no deja que esté en una estructura espacial: **vacía el terreno** (`IfcRelVoidsElement` al `IfcGeographicElement`). Como el terreno ya va explanado, el vacío no le quita nada a su sólido.
- Material: el del terreno, en los terraplenes (a una sustracción el IFC no le deja material).
- Pset `ArchiSim_Earthworks`: `Kind` (Cut/Fill), `Source` (Pad, Pool, PoolObject), `Use`, `Volume`, `PlanArea` y `Pad` (su número). ArchiSim los quita al importar: se rehacen desde el terreno y las piscinas.
- Las plataformas, con su nombre, clase y rampa, viajan en `ArchiSim_Terrain` (`Pads`, `PadNames`, `ShowEarthworks`).
- **IFC4** (compatibilidad): no existen esas clases; van como `IfcBuildingElementProxy` (`USERDEFINED`, «Desmonte» o «Terraplén») en el `IfcSite`, con las cantidades en `ArchiSim_EarthworksQuantities`.

`scripts/validate_ifc.py` comprueba (punto 15) la clase, la relación con el terreno o el emplazamiento, las cantidades y que el volumen del sólido coincida con ellas.

## 6. Ejemplo

`archisim tierras [carpeta]`: una ladera al 12 % hacia el norte; una vivienda de 14 × 10 m con su plataforma y el vaciado del garaje bajo una parte (8 × 10 m, a −3,00), una terraza dibujada a mano a +0,60 con talud 3:2, la rampa de acceso al garaje desde la calle, la plataforma de la piscina a +0,15 y una piscina de 8 × 4 m. Escribe `tierras.archisim`, `tierras.ifc` (IFC 4.3), `tierras-ifc4.ifc`, `tierras-planta.svg` (1:200, con el desmonte y el terraplén coloreados) y `tierras-informe.txt`.

## 7. Lo que no hace

- **Zanjas** de instalaciones como `IfcEarthworksCut` `TRENCH`: las zanjas siguen midiéndose en el capítulo de redes urbanas (`zanja:`), sin sólido.
- **Esponjamiento** y coeficiente de paso a compactado: los volúmenes son en perfil.
- Plataformas con **pendiente en dos direcciones** o alabeadas, bermas y taludes escalonados; taludes distintos en cada lado.
- **Muros de contención** que sustituyan al talud: el talud se dibuja aunque haya un muro; hay que darle un talud muy tendido o recortar la plataforma a mano.
- Capas del terreno (tierra vegetal, roca): el desmonte es un solo material.
- El vaciado del sótano no se recorta por los muros de sótano ni por los pozos de cimentación; las zapatas no se miden como excavación.
- Las curvas de nivel pueden cruzar en diagonal una plataforma que está justo a la cota de una curva (triángulos largos del terreno explanado).
- La relación de los desmontes con el `IfcSite`: la norma la prohíbe en IFC 4.3 (van por el terreno que vacían).
