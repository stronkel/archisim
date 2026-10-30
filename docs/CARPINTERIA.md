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

## 6. Correderas, guillotina, y lo de fuera en el IFC (29/10/2026, noche)

- **Ventana y balconera corredera** (`props:slider`): marco con sus carriles, hojas en carriles alternos solapadas donde se encuentran, vidrio con junquillos y cuarterones, y el cierre de cada hoja en el montante junto a la jamba. **Perfil**: aluminio estándar, con rotura de puente térmico, PVC, madera, acero fino o minimalista de perfil oculto. **Cierre**: uñero, manilla embutida, maneta de cierre o tirador de barra. **Mosquitera** en un carril propio por fuera, **persiana** con capialzado y bajada al porcentaje que se diga, y las contraventanas y rejas de la practicable.
- **Puerta corredera sobre el muro** (`props:sliding_door`): la hoja colgada por la cara del muro del lado al que abre, de un **carril visto** (de granero, con ruedas y tacos), un **carril con galería** (el cajón de madera que lo tapa) o un **carril oculto**, sobre un listón; cerco en todo el grueso del muro y tapajuntas solo por la cara que no tapa la hoja. Las hojas y tapajuntas de la abatible y su **tirador**: uñero embutido, tirador de barra o asa de granero.
- **Puerta corredera empotrada**: la hoja en el plano del cerco, que entra en el muro; uñeros por las dos caras y tirador en el canto; tapajuntas por las dos caras.
- **Ventana de guillotina** (`props:sash_window`): las dos hojas en sus planos, separadas por el junquillo de las jambas; cierre de media luna, asas en la de abajo y, en madera, los cuernos de la de arriba; el perfil y los cuarterones de la practicable.
- **Abierta en 3D** (parámetro de cada puerta o ventana colocada): cuánto está corrida la puerta corredera o subida la guillotina, solo para el 3D (la planta sigue con su dibujo de siempre).
- **Lo de fuera como elemento propio**: la reja es un `IfcRailing` con su peso estimado (hierro, 7.850 kg/m³, por el volumen de las barras) en `ArchiSim_OpeningPart` y `Pset_RailingCommon`; las contraventanas, un `IfcShadingDevice` .SHUTTER.; los dos salen en los alzados, las secciones y el render (en su versión sencilla) y en las mediciones, capítulo **11 Cerrajería y protecciones** (reja en kg, contraventanas en m²). Al importar el IFC se reconocen y no se convierten en barandillas sueltas.

## 7. Pivotante, plegable y garaje (30/10/2026)

- **Pivotante** (`props:door` con `pivot`): la hoja de la colección girando sobre sus pivotes de suelo y de dintel, sin bisagras; manilla por defecto **tirador largo** (nueva también en la abatible).
- **Plegable** (`props:folding_door`): las hojas de la colección colgadas de una guía en el dintel, con bisagras entre ellas y la manilla en la última; «Abierta en 3D (%)» las pliega en zigzag hacia el lado al que abre.
- **Garaje** (`props:garage_door`): **seccional** (paneles, guías por las jambas, carriles bajo el dintel y eje de muelles), **basculante** (una hoja que bascula sobre sus brazos, con los contrapesos en cajas junto a las jambas) y **enrollable** (lamas, guías y tambor). «Acabado de la hoja»: acanalada, lisa, cuarterones, lamas de madera, con ventanas o nervada vertical; puerta peatonal con su manilla; «Abierta en 3D (%)».

## 8. Arcos y huecos de forma (30/10/2026)

El hueco de obra ya no tiene por qué ser rectangular. La receta lo dice con dos salidas, `opening_shape` (Recto, Medio punto, Rebajado, Apuntado, Carpanel u Óculo) y `opening_rise` (la flecha), y ArchiSim recorta el muro con esa forma en el 3D, los alzados, las secciones, el render, el IFC (`IfcOpeningElement` con su perfil en el plano del muro) y las mediciones (se descuenta la forma, no la caja). Encima del arco el muro se rellena capa a capa (`archisim_core::shape`, `solid::infill_prism`).

Las piezas que siguen la forma las hace la instrucción `outline` de las recetas (`docs/FAMILIAS.md`): un anillo entre dos contornos metidos (el cerco, el bastidor de una hoja, un recercado con distancias negativas, fuera del hueco) o el contorno lleno (un vidrio, una hoja), recortados por una caja. Así se componen:

- **Puerta en arco**: «Hojas en arco» (una o dos hojas que siguen el arco; ciegas, con cuarterones o vidrieras, con vidrio en el arco) o **«Puerta con fijos en arco»** (la puerta rectangular en el centro, montantes, fijos laterales con zócalo si se quiere y el fijo superior que completa el arco, como la de la foto de Antonio); cuarterones de los vidrios.
- **Ventana en arco**: hojas bajo un fijo en arco, hojas que siguen el arco o fija; óculo; cuarterones; vierteaguas.
- **Recercado por fuera** en las dos: plano, moldurado (tres escalones) o con clave e impostas, del material que se elija (piedra caliza por defecto).
- **Hueco** (sin carpintería): ahora también de medio punto, rebajado, apuntado, carpanel u óculo.

## 9. El diálogo de tipos al estilo ArchiCAD (E6, primera entrega, 30/10/2026)

En «Tipos y materiales», una puerta o una ventana tiene ahora, debajo de la vista previa (alzado, planta, sección y 3D, que cambian al momento con cada valor), seis pestañas como las de ArchiCAD:

- **Hojas**: las dimensiones del hueco, el número de hojas y el funcionamiento de cada una (en tarjetas con su alzado), cerco o marco, tarja y fijos, el modelo de la colección, y dos grupos que juntan lo que antes estaba repartido: **Estilo de hoja** (hoja, retícula, cuarterones, perfil, tapajuntas, bisagras) y **Manilla y herrajes** (manilla, altura, caras, mirilla, cierrapuertas, zócalo).
- **Hueco de muro**: forma del hueco, arco y recercado.
- **Accesorios**: persiana, premarco, vierteaguas, contraventanas y rejas.
- **Representación**: líneas de giro y de apertura.
- **Clasificación**: nombre, marca, descripción, clase IFC y resistencia al fuego.
- **Propiedades**: cuántos hay colocados, GlobalId y los parámetros del proyecto.

Nuevo en las recetas:

- **Retícula a medida** en las hojas de puerta (abatible, pivotante, plegable y correderas): vidrios a lo ancho y a lo alto (`grid_cols`, `grid_rows`).
- **Cuarterones «A medida»** en las ventanas (practicable, corredera y guillotina): vidrios a lo ancho y a lo alto de cada hoja (`bar_cols`, `bar_rows`).
- **Manilla: altura** (puertas, 1,02 m desde el suelo; ventanas, desde el pie de la hoja, 0 = centrada) y **Manilla: caras** de las puertas (las dos, solo del lado al que abre o solo del otro).

## 10. Siguiente

- E6, segunda entrega: hueco de muro (telar, mocheta, dintel, arcos de albañilería) y accesorios (persiana con cajón, vierteaguas, albardilla, parasol de lamas, premarco) en sus pestañas.
- E6, tercera: representación (tramas y líneas en planta y sección), clasificación y propiedades.

- Los recercados de los huecos sin carpintería, y la herradura y el arco lobulado (no son convexos).
- Los arcos en muros curvos (hoy se quedan rectos en ellos).
- El taller de carpintería (§4).
