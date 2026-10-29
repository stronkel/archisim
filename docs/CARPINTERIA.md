# Carpintería con detalle (paquete 9)

Puertas, y después ventanas, con el mismo nivel de detalle que los muebles del taller: el diseño de la hoja, el tapajuntas, la manilla, las bisagras y la mirilla, en las ocho colecciones y «A tu gusto». Este documento dice cómo está hecho y cómo se abrirá más adelante un **taller de carpintería**, donde se dibuje una hoja, una moldura o una reja a medida.

## 1. Dos dibujos de la misma puerta

La receta (`.asfam`) sigue siendo la puerta: sus medidas, su hueco, su planta, su alzado, su IFC y sus cuentas. Lo nuevo es que la receta puede pedir a ArchiSim que fabrique la carpintería en 3D:

- `outputs generator = "props:door"` y unas salidas con lo que el fabricante necesita saber (`leaf_x0`, `leaf_x1`, `leaf_split`, `leaf_top`, `leaf_t`, `lining_w`, `lining_h`, `hinge_left`).
- La palabra reservada `detailed`: vale `false` para la planta, los alzados, el IFC y las mediciones (todo sale de la receta, como siempre) y `true` para la vista 3D, la Maqueta y las miniaturas. La receta envuelve sus hojas en `if not detailed`; en 3D las hace `archisim_props::fine::door`.

Por eso **el IFC no cambia**: la puerta exportada es la de la receta, con sus hojas y su marco. El 3D detallado es la imagen.

En 3D cada tipo de puerta es un modelo (`OpeningDetail`, de `archisim_geom::solid::opening_detail`) colocado en cada ejemplar: en su muro, girado y, si abre hacia el otro lado, reflejado. De lejos se dibuja su versión sencilla, que hace el mismo generador (marco, hojas y vidrio, sin herrajes ni molduras), así que cien puertas de seis cuarterones no pesan más que una. La sombra se hace siempre con la versión fina (una puerta está tan pegada al muro que la basta la ensombrecería).

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

## 4 bis. Puertas especiales: cortafuegos, metálicas, de seguridad y con ventanilla (29/10/2026)

Pedidas por Antonio para hospitales, industria y edificios públicos. Son opciones de la misma «Puerta abatible», así que se combinan con todo lo demás:

- **Hoja**: chapa lisa, chapa embutida (dos cuarterones estampados), chapa con ventanilla cuadrada, con ojo de buey o con mirilla alargada; lisa (madera) con ventanilla cuadrada o con ojo de buey; y **acorazada** (chapa con el canto ancho y los cuarterones estampados). Las ventanillas son un dato (`Vision`: cuadrada o redonda, con su tamaño y su altura), así que el taller futuro podrá añadir otras.
- **Manilla**: barra antipánico (en la cara desde la que se empuja, en las dos hojas si es doble; manilla por la otra cara de la activa), placa de empuje con tirador (hospitales) y escudo de seguridad con protector del bombín.
- **Cierrapuertas** (cuerpo en la hoja y brazo al cerco, en la cara hacia la que abre) y **zócalo de acero** en las dos caras.
- La **resistencia al fuego** (EI2 30, 60 o 90) ya estaba en la receta y va al IFC (`Pset_DoorCommon.FireRating`).

El ojo de buey lleva su aro de acero en cada cara; la ventanilla cuadrada, junquillos. En el IFC la hoja sigue siendo la de la receta (sin el hueco de la ventanilla).

## 5. Las ventanas (segunda parte, 29/10/2026)

La ventana practicable funciona igual que la puerta: su receta sigue dando planta, alzados, IFC y mediciones. En 3D, `archisim_props::fine::window` (`generator = "props:window"`) hace el marco, los montantes y las hojas. El premarco, la tarja, el capialzado y el vierteaguas siguen saliendo de la receta.

- **Modelo 3D**: las ocho colecciones y el material de los herrajes.
- **A tu gusto**:
  - **Perfil**: aluminio recto o redondeado, PVC, madera, madera con moldura o acero fino. Cambia lo que se ve de la hoja, su fondo, el redondeo de las aristas, cuánto sobresale la hoja por dentro, el junquillo y si se ven las bisagras.
  - **Cuarterones**: cuadrícula, cuadrícula fina, horizontales, verticales o solo arriba.
  - **Manilla**: moderna, de latón, cremona vista o falleba de forja.
- Cada hoja lleva su manilla donde cierra, según abra: en el larguero las practicables, oscilobatientes y pivotantes; arriba las abatibles y basculantes; abajo las proyectantes. Las bisagras van del lado del giro. Las hojas fijas llevan el vidrio directamente en el marco.
- **Por fuera**:
  - **Persiana**: con capialzado, sus guías y las lamas bajadas el porcentaje que se diga («Persiana: bajada (%)»).
  - **Contraventanas**: mallorquinas, de tablas o de cuarterones, abiertas contra la fachada o cerradas, con sus pernios. Las de tablas llevan los travesaños y la cruz hacia el muro.
  - **Reja**: barrotes, barrotes y pletinas, cuadrícula o forja con volutas, plana sobre la fachada (con garras al muro y copete) o **de cajón** con su vuelo. Las rejas y las contraventanas también se dibujan en planta.
- Las tarjetas de Editar tipo enseñan la ventana en 3D: la reja y las contraventanas desde la calle, lo demás desde dentro.

La reja es un dato (`Grille`), como los cuarterones (`Bars`) y las hojas de puerta, así que el taller de rejas del §4 podrá añadir las suyas.

**Lo que falta**: la reja y las contraventanas todavía no van en el IFC ni en los alzados (solo en 3D y en planta). Habría que exportarlas como elemento propio (reja como `IfcRailing` o `IfcElementAssembly`, contraventanas como parte de la ventana) para medirlas y presupuestarlas.

## 6. Siguiente

- La reja y las contraventanas en el IFC, en los alzados y en las mediciones.
- Las demás ventanas (corredera y guillotina) y las demás puertas (corredera, pivotante, plegable y de garaje) con el mismo sistema.
