# Carpintería con detalle (paquete 9)

Puertas, y después ventanas, con el mismo nivel de detalle que los muebles del taller: el diseño de la hoja, el tapajuntas, la manilla, las bisagras y la mirilla, en las ocho colecciones y «A tu gusto». Este documento dice cómo está hecho y cómo se abrirá más adelante un **taller de carpintería**, donde se dibuje una hoja, una moldura o una reja a medida.

## 1. Dos dibujos de la misma puerta

La receta (`.asfam`) sigue siendo la puerta: sus medidas, su hueco, su planta, su alzado, su IFC y sus cuentas. Lo nuevo es que la receta puede pedir a ArchiSim que fabrique la carpintería en 3D:

- `outputs generator = "props:door"` y unas salidas con lo que el fabricante necesita saber (`leaf_x0`, `leaf_x1`, `leaf_split`, `leaf_top`, `leaf_t`, `lining_w`, `lining_h`, `hinge_left`).
- La palabra reservada `detailed`: vale `false` para la planta, los alzados, el IFC y las mediciones (todo sale de la receta, como siempre) y `true` para la vista 3D, la Maqueta y las miniaturas. La receta envuelve sus hojas en `if not detailed`; en 3D las hace `archisim_props::fine::door`.

Por eso **el IFC no cambia**: la puerta exportada es la de la receta, con sus hojas y su marco. El 3D detallado es la imagen.

En 3D cada tipo de puerta es un modelo (`OpeningDetail`, de `archisim_geom::solid::opening_detail`) colocado en cada ejemplar: en su muro, girado y, si abre hacia el otro lado, reflejado. Como los muebles, tiene versiones más bastas por distancia, así que cien puertas de seis cuarterones no pesan más que una. La sombra se hace siempre con la versión fina (una puerta está tan pegada al muro que la basta la ensombrecería).

## 2. Las opciones

En la sección «Modelo 3D» de la familia: la colección y el material de los herrajes. En «A tu gusto», cada una empezando en «Como la colección»:

- **Hoja**: lisa, ranurada (horizontal o vertical), dos, cuatro o seis cuarterones, plafonada, moldura aplicada, vidriera, media vidriera, vidriera con junquillos, lamas y tablas (con travesaños y cruz por detrás).
- **Tapajuntas**: sin tapajuntas, plano, redondeado, moldurado y con junquillo.
- **Manilla**: palanca con roseta, con placa o recta, pomo, tirador de barra o sin manilla.
- **Bisagras**: ocultas, de pala o de pernio.
- **Mirilla**.

En Editar tipo se eligen en tarjetas con la puerta dibujada en 3D en su muro.

## 3. Lo que es dato y no programa

Todo lo que define el aspecto de una hoja y de una moldura es **dato**, no código. Por eso un taller podrá crear piezas nuevas sin programar:

- **`LeafDesign`**, el diseño de una hoja: lisa (con ranuras, horizontales o verticales, o molduras aplicadas), de tablas (cuántas) o de bastidor y paneles (ancho de largueros, de travesaños y del travesaño bajo, y filas de campos). Cada fila (`Row`) tiene su altura relativa y sus campos (`Field`): panel hundido, panel en relieve, vidrio, vidrio con junquillos en cuadrícula (`Bars(columnas, filas)`) o lamas.
- **`Profile`**, la sección de una moldura: una lista de puntos (ancho, grueso) que se barre a lo largo de un camino con los ingletes resueltos (`moulding`). Los tapajuntas son perfiles; los junquillos de los paneles y los vidrios, también.

Las catorce hojas y los cinco tapajuntas que vienen son entradas de esas listas (`DESIGNS`, `CASINGS`).

## 4. El taller de carpintería (para más adelante)

La idea, para cuando Antonio lo pida:

1. **Hojas a medida**: un editor en alzado donde se dibuja la retícula de la hoja (largueros, travesaños, filas y campos) y se elige qué lleva cada campo. Guarda un `LeafDesign` con nombre en la biblioteca (`Carpintería/Hojas/<nombre>`), y aparece como una tarjeta más en «Hoja».
2. **Molduras a medida**: un editor de perfil (polilínea en sección, con medidas) o un perfil leído de un DXF. Guarda un `Profile` que sirve para tapajuntas, junquillos, cornisas o zócalos.
3. **Rejas** (muy habituales en Andalucía): mismo principio que la hoja, una retícula en alzado con barrotes, pletinas, volutas y remates elegidos de una lista, más el saliente de la reja (plana o de cajón, con su «buche»). Colgará de las ventanas como hoy cuelgan las hojas de las puertas: un generador `props:reja` alimentado por un dato `Grille`.
4. Cada pieza a medida va a la biblioteca como fichero de texto (como las recetas), se comparte entre proyectos y entra en las tarjetas.

Nada de esto pide cambiar el formato del proyecto ni el IFC: lo nuevo serían datos con nombre que la familia elige como una opción más.

## 5. Siguiente

- Las demás familias de puerta (corredera, pivotante, plegable, de garaje) con el mismo generador.
- Las ventanas: hojas, junquillos, cremonas, contraventanas, persianas, y la reja.
