# ArchiSim — Vegetación generada (vivero y plantaciones)

> 21 de octubre de 2026 · v0.37 · rama `bloque/vivero`. Pedido por Antonio el mismo día: «crear el editor como EZ-Tree y generarlos nosotros», con especies mediterráneas primero y variedad para cualquier clima, dibujos en planta como los de las láminas de paisajismo, deslizadores, tamaño visual en 2D y 3D y plantaciones a lo largo de líneas o splines.

## 1. La idea

Los árboles no se descargan: **crecen**. ArchiSim tiene un generador propio (crate `archisim-flora`, escrito desde cero, sin código de nadie: se sigue la idea de EZ-Tree, no su código) que, a partir de una **especie** y una **semilla**, hace una planta:

- su tronco o troncos, inclinados y retorcidos lo que diga la especie;
- sus ramas, por órdenes (ramas principales, secundarias, ramillas), que llenan el volumen de la copa;
- sus matas de hojas sobre las ramas y sobre la piel de la copa, más claras arriba y más oscuras dentro;
- sus flores y frutos, si los tiene;
- en las palmeras, el estípite con sus anillos y las frondas (pinnadas o en abanico);
- en gramíneas, pitas y yucas, sus hojas en roseta.

La misma especie y la misma semilla dan **siempre la misma planta** (en cualquier ordenador, al reabrir el proyecto o al importar su IFC); otra semilla da otra planta de la misma especie. Por eso cada árbol del proyecto es distinto sin que el fichero pese: se guardan unos números, no mallas.

De la planta crecida salen **su 3D**, en tres niveles de detalle, y **su planta**, en el estilo que se elija. El 3D y la planta son la misma planta: la copa que se ve en planta es la de su 3D visto desde arriba.

## 2. Especies

Una especie es un **tipo** de la familia de sistema «Vegetación» (`families/objetos/vegetacion.asfam`). Sus parámetros:

| Grupo | Parámetros |
|---|---|
| Dimensiones | Altura, ancho de la copa, diámetro del tronco, altura del fuste |
| Porte | Árbol, conífera, palmera, arbusto, mata, gramínea o roseta; forma de la copa (redonda, ovalada, parasol, extendida, columnar, cónica, en vaso, llorona, irregular, cúpula, cojín); troncos o tallos; inclinación; tortuosidad |
| Ramas | Ramas principales, órdenes de ramas, ángulo, caída (hacia arriba o colgantes) |
| Hojas | Follaje (hoja ancha, menuda, acículas, escamas, retama, suculenta, pinnada, abanico, hojas largas), densidad, tamaño de las matas de hojas, frondas y su largo |
| Flor y fruto | Floración y fruto (cuánto) |
| Materiales | Hojas, tronco, flor, fruto (sus colores) |
| Dibujo en planta | Presentación, según el estilo de la vista o técnico; estilo técnico, estilo de presentación, color (líneas, transparente, rellena, solo color) y opacidad |

Cada ejemplar tiene además **su semilla** (0: la suya propia, sacada de su `GlobalId`, así que una copia es otra planta), **su altura y su ancho** si se quieren distintos de los de la especie, **su variación** (cuánto se aparta de la especie) y su dibujo propio si se quiere.

### La biblioteca

64 especies integradas en el programa (no pesan nada): se ven en la galería de **Vegetación** y se traen al proyecto al elegirlas.

- **Árboles mediterráneos**: pino piñonero, pino carrasco, olivo, acebuche, encina, alcornoque, algarrobo, eucalipto, castaño, almez, naranjo, limonero, almendro en flor, granado, higuera.
- **Coníferas**: ciprés, ciprés de Arizona, abeto, cedro del Atlas, araucaria, sabina.
- **Palmeras**: datilera, canaria, washingtonia, palmito.
- **Árboles ornamentales**: jacaranda, castaño de Indias, plátano de sombra, tipuana, árbol del amor, mimosa, morera, ficus, magnolio, sauce llorón.
- **De ribera y clima templado**: chopo, álamo blanco, fresno, olmo, tilo, abedul, arce rojo, ginkgo.
- **Clima cálido**: acacia de sabana, árbol tropical, drago.
- **Arbustos**: romero, lentisco, retama, retama blanca, adelfa, mirto, coscoja, pitosporo, boj, lavanda, taray.
- **Matas y plantas de marisma**: tomillo, salicornia, almajo, barrilla.
- **Gramíneas y suculentas**: esparto, pennisetum, pita, yuca.

Las medidas y colores son una propuesta para que Antonio los corrija: se cambian en el Vivero y se guardan en la biblioteca del estudio como cualquier tipo.

## 3. Dibujo en planta

Veintiún estilos: diecisiete sacados de la propia planta crecida y cuatro que son los dibujos de AD5:

| Estilo | Cómo es |
|---|---|
| Círculo | Círculo y tronco: el símbolo técnico |
| Contorno | El contorno real de la copa |
| Dentado | El contorno en dientes pequeños |
| Nube | El contorno en lóbulos redondeados |
| A mano | Dos contornos que no coinciden, como a mano alzada |
| Ramas | Las ramas vistas desde arriba (árbol sin hojas) |
| Hojas | Hojitas sueltas donde están las matas de hojas (acículas en estrella en los pinos, rayitas en la retama) |
| Ramas y hojas | Las dos cosas |
| Follaje | Las masas de hojas con sus huecos |
| Punteado | Puntos, más juntos en el lado de sombra |
| Radial | Líneas del tronco al borde (pinos, coníferas, retamas, gramíneas) |
| Frondas | Las hojas de palmeras, pitas y yucas, con sus foliolos |
| Acuarela | Manchas de color superpuestas, sin líneas |
| Corona | Grupos de hojas en roseta alrededor del borde de la copa, el centro abierto |
| Silueta | La copa maciza, del color de la línea, con su borde de hojas y las ramas en blanco a través de ella; en alzado, tronco macizo |
| Ramitos | Las ramas con foliolos a pares a lo largo de las ramillas, como plumas |
| Ramas y borde | Las ramas dentro de un contorno dentado |
| AD5 círculos | El dibujo de AD5: tres copas superpuestas en manchas transparentes, con anillos discontinuos |
| AD5 racimos | El dibujo de AD5: racimos de hojas sobre una mancha clara, con matas de hierba dentro de los racimos |
| AD5 estrella | El dibujo de AD5: rayos del tronco al borde (en color, sobre una mancha clara de la copa) |
| AD5 copa | El dibujo de AD5: el círculo de la copa con una masa de hojas irregular encima |

Los cuatro últimos (v0.61, pedidos por Antonio el 25/10/2026) son dibujos de línea con aire de biblioteca de CAD. Son diseño propio: salen del mismo árbol crecido, así que cada ejemplar es distinto. Valen igual en planta y en alzado. Los proyectos que ya tenían la familia Vegetación los ven después de **Biblioteca → Actualizar familias** (la familia pasa a la versión 2). `archisim vivero out/` dibuja todo el vivero en cada uno: `vivero-<estilo>-planta.svg` y sus alzados.

En las axonometrías de los planos, las copas se dibujan por su contorno (no por cada faceta) y, en las secciones, rellenas; las sombras usan la versión más ligera.

Cada uno puede ir en **líneas**, **transparente** (color con su opacidad, con la mitad en sombra un poco más oscura), **rellena** o **solo color**; las flores y los frutos salen como manchas de su color. Según el tamaño en el papel, el dibujo se simplifica solo (a 1:500 un árbol pequeño es un círculo).

### Los dibujos de AD5 (v0.70)

Los cuatro estilos «AD5 …» son los árboles de la biblioteca del estudio (26/10/2026), traídos de su DXF con `scripts/make_ad5_trees.py` a `crates/archisim-flora/src/ad5_trees.txt` (el DXF no se guarda en el repositorio). Cada uno se pone sobre la planta a la medida de su copa: se estira lo justo para seguir su contorno (un ciprés lo lleva estrecho; un pino piñonero, ancho) y se gira un ángulo distinto en cada ejemplar, así que una alineación no se ve repetida. Las manchas llevan el color de las hojas de la especie con la opacidad que tenían en AD5 (con la opacidad por defecto, 0,55; más o menos si se cambia); las líneas van con la plumilla de la vista; el tronco es el de la planta. En «líneas» quedan solo las líneas; en «rellena», las manchas opacas; en «solo color», sin líneas. Las matas de hierba de «racimos» son dibujo propio de ArchiSim (no se copia la trama del DXF) y solo salen cuando la copa mide en el papel 2 cm o más. En alzado, que no son dibujos de lado, se dibujan con el estilo de ArchiSim más parecido (nube, follaje o contorno).

Para retocar uno de los cuatro basta cambiar su bloque en el DXF y volver a pasar el script; añadir un quinto pide además su estilo en el programa.

## 4. 3D

Tres niveles de detalle: la planta entera (unos miles de caras; un ciprés, unas 20 000), una más ligera (las matas de hojas juntas, sin ramillas: unos 2 000–3 000) y una muy ligera (unas pocas masas y el tronco: unos cientos). Las vistas toman la más detallada con la que todas las plantas del proyecto caben en el presupuesto de caras (como los modelos importados): con pocos árboles se ven enteros; con cientos, ligeros. Las sombras de presentación salen de este 3D.

En el IFC cada planta lleva su propia malla (`IfcTriangulatedFaceSet` por material, con el color de cada cara), con la clase de su familia (`IfcGeographicElement` VEGETATION en IFC 4.3) y sus valores (`ArchiSim_FamilyValues`) para leerla de vuelta idéntica.

## 4 bis. En alzados, secciones y axonometrías (C1, v0.58)

En los alzados y las secciones las plantas no se dibujan con su malla 3D, sino como las dibuja un arquitecto: la misma planta vista de lado (su copa, su tronco, sus frondas), con los estilos de la planta.

- **Técnico** (vistas sin estilo de presentación): el estilo técnico de la especie, en líneas, con la copa en blanco, de modo que tapa lo que queda detrás. El «círculo» técnico se dibuja como el contorno real de la copa: un ciprés es alto y un pino piñonero plano.
- **Presentación** (vistas y planos con estilo de presentación): el estilo y el aspecto de presentación de la especie (contorno, dentado, nube, a mano, follaje, punteado, ramas, hojas, acuarela) con sus colores, sus flores y sus frutos.
- El tronco sube desde el suelo hasta la copa. Las palmeras, las gramíneas y las rosetas se dibujan siempre por sus frondas, de la más lejana a la más cercana, para que unas tapen a otras.
- La planta se pone en el orden de profundidad del dibujo: tapa lo que tiene detrás y la tapa lo que tiene delante. Las que quedan delante del plano de una sección no se dibujan.
- En las **axonometrías**, cada árbol se levanta de pie mirando a quien mira, con su estilo de presentación, en el orden de profundidad del resto del modelo.

`archisim figuras out/` pone en fila árboles, palmeras, arbustos, personas y coches delante de un muro y escribe su alzado 1:50, técnico y en color, y su axonometría. `archisim vivero out/` escribe además un alzado de cada fila del vivero, técnico y en color.

## 5. Plantaciones

Urbanismo → **Plantación** (tecla J): una línea (con clics, con **Spline** o con **Elegir línea** de un dibujo o DXF) o un contorno cerrado, y:

- **distribución**: a lo largo de la línea (alineaciones, setos, paseos) o repartidas dentro del contorno (bosquetes, arriates);
- **número de plantas**;
- **especies** que se mezclan al azar (una o varias);
- **variación de tamaño** (hasta ±40 %), **variación de forma** e **irregularidad** de las posiciones (de en fila y a distancias iguales a al azar);
- **azar** (🎲): otro reparto con las mismas reglas.

Mientras se dibuja la línea se ven las plantas donde van a ir. La plantación es un elemento: se selecciona por su línea (verde discontinua en pantalla, no se imprime) y en Propiedades se cambia todo; sus plantas se rehacen. Moverla o copiarla mueve o copia sus plantas; borrarla las borra. Cada planta es un objeto normal (se ve, se mide y se exporta como los demás); si se cambia la plantación, las plantas cambiadas a mano vuelven a sus reglas.

## 6. Vivero

Urbanismo → **Vivero** (o «Vivero…» en las propiedades de una planta): la ventana para crear y ajustar especies, como EZ-Tree.

- A la izquierda, las especies del proyecto con su dibujo, «Duplicar especie» y «Traer de la biblioteca».
- En el centro, todos los parámetros con **deslizadores**, por grupos.
- A la derecha, la planta en **3D** (se gira y se acerca con el ratón) y en **planta técnica y de presentación**, que se rehacen al mover los deslizadores. «🎲 Otro ejemplar» enseña otra planta de la misma especie; «Variación», cuánto se aparta.

Lo que se cambia es la especie: cambian todas sus plantas del proyecto.

## 7. Pendiente

- Tiradores en 3D para el alto y el ancho (en planta ya está: el tirador del borde de la copa cambia el ancho y, con Ctrl, también el alto).
- Hojas con textura (con M3): hoy las matas de hojas son cuerpos facetados de color.
- Setos como masa continua en planta (hoy, plantas juntas).
- Estaciones (hoja caduca en invierno, colores de otoño) por vista.
- Que una planta movida a mano salga de su plantación en lugar de volver a su sitio al cambiarla.
