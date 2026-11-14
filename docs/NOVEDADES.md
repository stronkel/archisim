# Novedades de ArchiSim

Lo que cambia en cada versión, en pocas líneas. ArchiSim enseña la parte de su versión antes de actualizarse (Ayuda → «Buscar actualizaciones…»), y la página de cada versión en GitHub la lleva también. El detalle, día a día, está en `BITACORA.md`.

Cada versión empieza con una línea `## <versión>` (por ejemplo `## 0.81.0`): es la que busca la publicación.

## 0.85.0

- **Todo responde al instante, también en proyectos grandes.** La ventana rehacía en cada momento las tablas y los recuentos del navegador; en un edificio grande eso la frenaba en cualquier vista. Ahora se cuentan solo cuando cambia el modelo.
- **Zoom y desplazamiento sin esperas.** La planta, los alzados y las secciones se dibujan con un margen alrededor y en segundo plano: mientras llega el dibujo fino se ve el anterior, sin que la ventana se pare.
- **Alzados, secciones y 3D se preparan aparte.** Al abrirlos o cambiar el modelo, la ventana sigue respondiendo («Dibujando la sección…», «Preparando el 3D…»). Elegir en 3D es inmediato.
- **Se ve qué vas a coger.** En alzados, secciones y 3D, al pasar el ratón sale el contorno azul del elemento y su nombre; al elegirlo, solo su contorno, sin relleno. Donde hay varios uno encima de otro, **Tab** o un segundo clic coge el de detrás («2 de 4 · Tab»).
- **Guardar sin pararse.** Ctrl+S guarda en segundo plano.
- **Medidor** (Ctrl+Alt+F): cuánto tarda cada fotograma y qué parte de la ventana se lleva el tiempo, para avisarnos si algo va lento.

## 0.84.0

- **Mover el calco con el ratón.** En las propiedades de la planta o de la vista de dibujo, cada calco tiene «Mover…», «Encajar…», «Encajar y escalar…» y «Escalar por medida…».
  - Los puntos se enganchan a las líneas del propio calco y al modelo, con líneas de rastreo.
  - «Encajar» pide dos puntos del calco y dónde van: lo mueve, lo gira y, si se quiere, lo escala.
  - «Escalar por medida» pide dos puntos y cuánto mide eso de verdad.
  - Cada calco tiene **escala**, **candado** para no moverlo sin querer y **visible** para apagarlo sin perderlo.
- **Flechas del teclado.** Mueven lo seleccionado en planta, alzado, sección y vistas de dibujo, como en Revit: cuanto más cerca estás, más fino es el paso. Con Mayúsculas, diez veces más.
- **Guías al mover y estirar.** Al arrastrar un mueble, un pilar o una caja en planta, sus esquinas, medios y centro se alinean con los de lo que hay alrededor, con la línea guía azul. Al estirar, se ve la medida nueva.
- **Reflejar sobre cualquier arista.** El eje puede ser el canto de un forjado, el lado de una caja, un falso techo, un eje o un plano de referencia, aunque no haya nada seleccionable ahí. Antes de hacer clic se ve el eje y cómo quedaría lo reflejado.
- **Copiar en fila** (Modificar, o `FILA`): N copias a una distancia, o repartidas hasta un punto. Sirve para cajas, pilares o muebles.

## 0.83.0

- **Dibujar el despiece con precisión.** En el boceto en alzado hay forzados (final, medio, intersección, perpendicular, jambas y juntas de la capa de al lado) y líneas de rastreo. Al elegir una junta salen sus cotas azules: se hace clic en la cifra y se escribe otra.
- **Mover sabiendo cuánto.** Al arrastrar se ve la distancia junto al cursor; si escribes un número y pulsas Intro, se mueve justo eso. **0,2 ya es 0,2**: la coma es decimal; las coordenadas se escriben con punto y coma (2;1).
- **Juntas cada…** Clic en una jamba o un borde y salen juntas a un paso fijo (1,20 por defecto) hasta los dos bordes. Y **Matriz** para copiar líneas a paso fijo.
- **Los huecos se ven como huecos** en el despiece, y la pieza bajo el cursor se resalta con su medida.
- **«Quitar»** (antes «Normal») borra la pintura y la marca de obra de una pieza.
- **Despiece a mano en la cinta** al elegir un muro, un forjado o un techo, con cada capa por su material («Capa 1 · Placa cerámica»). Los forjados y techos empiezan con las juntas de su regla.

## 0.82.0

- **Despiece a mano, tercera vuelta.** Un despiece se copia a los muros iguales (también en simetría). Las piezas de las juntas entre cajas se marcan «en obra» de una vez. «Repartir» parte una pieza en tantas iguales como digas.
- **Avisos del despiece.** Mientras dibujas, y en la Comprobación CTE: en rojo la pieza que no sale de una placa; en ámbar la de menos de 30 cm, la junta pegada a una jamba y la que cae sobre una junta de la capa de al lado.
- **Planos de despiece.** Archivo → Exportar → «Despiece a mano de las fachadas…»: una hoja A3 por cara, vista desde fuera, con las marcas de las piezas, las juntas acotadas y la tabla de piezas para el taller y la obra.
- **Forjados y falsos techos despiezados a mano**, en planta: las mismas líneas, pinturas y piezas en obra que en los muros. Lo siguen la compra y los pedidos.


## 0.81.0

- **ArchiSim se actualiza solo.** Al abrirlo mira si hay una versión nueva y lo avisa en una esquina. En Ayuda → «Buscar actualizaciones…» se ve qué cambia, se descarga y se instala; se vuelve a abrir con el mismo proyecto. Se puede desactivar en esa ventana.
- **Despiece a mano, cara a cara.** En las propiedades del muro, «Despiece a mano»: cada capa de placas (cerámica, Tabihaus, yeso) se dibuja en alzado, visto desde fuera, empezando por las juntas de su regla. Cada línea es una junta. La junta se elige en cada cara. Las piezas se pintan con cualquier material arrastrando el ratón, y las de las juntas entre cajas se marcan «en obra»: salen aparte en la compra y en los pedidos.
- **El despiece del estudio.** Por material: placas a media placa, en cremallera o por líneas de junta, con un acento alrededor de puertas y ventanas (marcos de 30 cm, columnas de 1,20 desde las jambas, nunca piezas de menos de 30 cm).
- **Equipar la vivienda** (Colaboración → «Equipar la vivienda…»): electricidad, fontanería y ACS, saneamiento y pluviales, y ventilación de cada local por su uso, unidas y calculadas; con cajas, además, las reglas del sistema sobre lo que viene de Revit.
- **El chasis en el cálculo de la estructura.** Los suelos y techos de las cajas se comprueban como chasis (largueros sobre la obra previa y el izado), con el tubo elegido por cálculo; el porche, articulado al marco de la caja.
- **Cajas modulares**: obra previa, plantilla del fabricante con sus perfiles, cajas sobre una vivienda importada, juntas y uniones, placas por piezas con lista de compra, montantes en el HE 1, instalaciones en el entramado, reglas del sistema, fábrica (paneles, etiquetas, perfiladora) y porche ligero.
