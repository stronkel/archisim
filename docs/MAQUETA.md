# ArchiSim — Entorno Maqueta

> 23 de octubre de 2026 · Diseño acordado con Antonio en conversación el mismo día, a partir de su idea: un editor tan intuitivo como Los Sims y, sobre todo, tan modular como Paralives, para arquitectura, formas y urbanismo, con una imagen más cuidada (cartoon, pero con sombras) sin castigar a un MacBook de 2016. **Lo que más importa es la fluidez.**
> Rama `bloque/maqueta`.

## 1. La idea

La maqueta **no es otro programa ni otra disciplina**: es otra forma de presentar la misma ventana, sobre el mismo modelo. Las disciplinas dicen *qué* se modela (estructura, instalaciones, urbanismo); la maqueta dice *cómo* se maneja: a ojo, en 3D, eligiendo cosas de un catálogo.

- Un interruptor **Técnica · Maqueta** arriba a la derecha, en el mismo sitio en los dos entornos, y la tecla **Tab** (salvo escribiendo en un campo de texto).
- Todo lo que se hace en la maqueta está a la vez en la planta técnica, las mediciones, el CTE y el IFC, y al revés: son los mismos muros, forjados, cubiertas y objetos.
- **Lo seleccionado sigue seleccionado al cambiar de entorno**: eliges el sofá en la maqueta, pulsas Técnica y lo tienes en la planta con sus propiedades.

Así un boceto con el cliente es ya el proyecto, y quien no sabe BIM (el equipo que viene de AutoCAD, un estudiante) puede empezar por la maqueta y pasar a la técnica sin cambiar de programa ni de fichero.

## 2. Dos capas

**Regla**: lo que es una *herramienta* va a los dos entornos; lo que es una *forma de presentarla* (catálogo grande, estilos en vez de tipos, menos texto), solo a la maqueta.

### 2.1 Lo común (mejora ArchiSim entero)

| Qué | Dónde se nota |
|---|---|
| **Muros recortados** en 3D: arriba, recortados a 1 m, solo los de delante recortados, solo planta | Vista 3D de los dos entornos |
| **Habitación** como método de Muro: arrastrar un rectángulo hace sus muros, su forjado y su local | Barra contextual de Muro |
| **Cuentagotas**: clic en un elemento y el siguiente sale igual (tipo, colores, parámetros) | Barra contextual |
| **Buhardillas y ventanas de cubierta** que se pinchan en un faldón y se adaptan a él (§5) | Arquitectura → Cubierta, y Construir |
| **Tejas, caballetes, canalones y bajantes** de las cubiertas (§5) | Propiedades de la cubierta |
| **Imagen** (§4): sombras, penumbra en rincones, luz de ilustración, texturas ligeras, niveles de calidad | Vista 3D |

### 2.2 Lo propio de la maqueta

```
┌──────────────────────────────────────────────────────────────────────────┐
│ ↶  ▲ Planta baja ▼  · Muros recortados · Cuadrícula     Técnica│Maqueta │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│                     Vista 3D (casi toda la ventana)        ┌──────────┐  │
│                                                            │ Sofá     │  │
│                                                            │ ● ● ● ●  │  │
│                                                            │ ↻ ⧉ ⌖ 🗑 │  │
│                                                            └──────────┘  │
├──────────────────────────────────────────────────────────────────────────┤
│ Construir · Amueblar · Exterior │ Salón · Cocina · Baño · Dormitorio  🔍 │
│ [miniatura] [miniatura] [miniatura] [miniatura] [miniatura] [miniatura]  │
└──────────────────────────────────────────────────────────────────────────┘
```

- **Sin columnas**: sin barra de disciplinas, fichas, Propiedades ni navegador. El 3D ocupa casi toda la ventana.
- **Barra superior reducida**: deshacer, planta activa con subir y bajar, muros recortados, cuadrícula y el interruptor.
- **Catálogo abajo** con miniaturas, en tres grupos que sustituyen a fichas y tipos a la vez:

  | Grupo | Temas | Qué hay |
  |---|---|---|
  | **Construir** | Habitaciones · Muros · Puertas · Ventanas · Cubiertas · Escaleras | Habitación, muro, forjado; los tipos de puerta y ventana del proyecto; cubierta a dos y a cuatro aguas, buhardillas y ventana de cubierta; escalera y barandilla |
  | **Amueblar** | Salón · Comedor · Cocina · Baño · Dormitorio · Oficina | Los objetos del proyecto y de la biblioteca, por la estancia que dice su familia |
  | **Exterior** | Terreno · Vegetación · Caminos · Mobiliario urbano · Formas | Especies del vivero, plantación, vial, mobiliario urbano, volúmenes de Concepto |

  Una miniatura *es* un tipo (o una herramienta con su tipo): al pulsarla se activa la herramienta de siempre con ese tipo. Se colocan igual que en técnica: la vista previa sigue al ratón, Espacio gira, clic suelta.
- **Tarjeta flotante** junto a lo seleccionado, en lugar de Propiedades: qué es y su tipo, **girar** un cuarto de vuelta, **duplicar** (una copia al lado), **cuentagotas**, **borrar** y **Más…** (las Propiedades completas en una ventana). Las medidas se cambian con los tiradores 3D que ya existen. Los colores rápidos por pieza llegan con la rueda de color (MQ2).
- **Estilos en lugar de tipos**: lo que sale del catálogo usa los tipos del proyecto, pero se ve por su nombre y su dibujo. Quien quiera los tipos los tiene en Más….
- **Cuadrícula en el suelo** de la planta activa (50 cm), encendida la primera vez que se entra en la maqueta; los clics sobre el suelo se ajustan a ella solo cuando se ve. (Sin tecla propia: la G ya es la viga.)

## 3. Fluidez

- La maqueta no añade trabajo por fotograma: el catálogo dibuja sus miniaturas una vez y las guarda; la tarjeta es un panel de la interfaz.
- Los muros recortados se hacen al dibujar (el sombreador descarta lo que queda por encima de la altura de recorte), sin rehacer la escena.
- Objetivo: el MacBook Pro de 2016 a 60 imágenes por segundo en calidad *Ligera* con la vivienda de prueba. Si no, no se da por bueno.

## 4. Imagen: cartoon cuidado y ligero (bloque V1)

Técnicas de juego que corren en portátiles viejos:

1. **Sombras del sol** del proyecto: un mapa de sombras (una pasada más), con el borde suavizado.
2. **Penumbra en rincones calculada una vez**, al cambiar el modelo, y guardada en los vértices: cero coste por fotograma.
3. **Luz de ilustración**: sombras de tono frío y sol cálido, en lugar de oscurecer a gris.
4. **Texturas como pequeñas imágenes** que ArchiSim dibuja al arrancar (256 píxeles, sin descargar nada): la tarjeta solo las lee, que es lo más barato que hace. Es lo que devolverá las texturas a los Mac con Intel, cuyo controlador se colgaba con las texturas calculadas.
5. Cielo degradado, bruma lejana y suelo de maqueta.

Tres niveles (**Ligera**, **Normal**, **Bonita**) elegidos solos según el ordenador, y **la imagen mejora al parar**: mientras se gira, menos resolución y sombra dura; al soltar el ratón se afina sola.

**Hecho en v0.49 (primera vuelta de V1)**: sombras del sol del proyecto (fecha y hora de Propiedades del proyecto → Sol) con mapa de sombras que solo se redibuja cuando cambia el modelo, el sol, el recorte de muros o la zona que se mira, así que girar la cámara no cuesta más; la zona de sombras se ajusta a lo que se ve, para que en una parcela grande sigan siendo finas; luz de ilustración (sol cálido, cielo frío, sombras azuladas); niveles **Ligera** (sin sombras ni suavizado), **Normal** (sombras suavizadas) y **Bonita** (sombras más finas y penumbra de los rincones calculada en pantalla), en la barra de la Maqueta y en Propiedades de la vista 3D, guardados por ordenador (en los Mac con Intel empieza en Ligera). Lo que los muros recortados quitan no da sombra. Pendiente: texturas como imagen (punto 4), cielo y bruma (5) y afinar al parar.

## 5. Cubiertas: buhardillas, ventanas de cubierta, tejas y canalones

Pedido por Antonio el 23/10/2026 al probar inserciones: la buhardilla no se podía colocar porque **no existía como elemento** (una ventana normal solo admite muros como anfitrión).

### 5.1 Buhardillas

Un elemento propio que vive en una cubierta: clic en un faldón y se adapta a él.

| Tipo | Cómo es |
|---|---|
| **A dos aguas** | Frente con su piñón, cumbrera horizontal que muere en el faldón, limahoyas |
| **A un agua** | Cubierta con menos pendiente que el faldón, de la cabeza del frente al faldón |
| **Plana** | Cubierta horizontal |
| **Ventana de cubierta** | En el plano del faldón, con su cerco, su vidrio y su babero (`IfcWindow` `SKYLIGHT`) |

- **Parámetros**: ancho, altura del frente, pendiente propia, retranqueo desde el alero (se fija al pinchar), ventana del frente (ancho y alto) y materiales (frente y laterales, carpintería, cubierta propia).
- Se coloca por el punto medio del frente, sobre el faldón; el frente mira siempre aguas abajo. Se mueve arrastrándolo y sigue al faldón si la cubierta cambia (si el punto queda fuera de la cubierta, se avisa y no se dibuja).
- **La cubierta se corta sola** donde está la buhardilla; los laterales bajan hasta la cara inferior del faldón.
- **IFC**: la buhardilla es un `IfcElementAssembly` (`ObjectType` «Buhardilla») con su frente y sus laterales (`IfcWall`), su cubierta (`IfcSlab` `ROOF`) y su ventana (`IfcWindow`); el hueco en el faldón, un `IfcOpeningElement`. Se lee de vuelta por su `ArchiSim_Dormer`.

### 5.2 Tejas y canalones

Propiedades de la cubierta → **Cubrición**:

- **Tejas**: ninguna (la capa del tipo, como hoy), teja árabe, teja mixta, teja plana, pizarra o chapa. En 3D, hiladas con su escalón y, en la árabe y la mixta, las cobijas en relieve; **caballete** sobre cumbreras y limatesas. En planta, el rayado de las tejas en el sentido de la pendiente. Es la piel exterior de la cubierta; las capas del tipo siguen debajo.
- **Canalón** en los aleros (no en los piñones): media caña o cuadrado, con su desarrollo y material (zinc, cobre, aluminio lacado, PVC). **Bajantes** en los extremos de cada alero (y cada 10 m como máximo), hasta la planta más baja.
- **Mediciones**: m² de cubrición y número de tejas por m², ml de caballete, ml de canalón y ml de bajante, cada uno en su partida.
- **IFC**: las tejas como `IfcCovering` (`ROOFING`) de cada faldón; canalones (`IfcPipeSegment` `GUTTER`) y bajantes (`IfcPipeSegment` `RIGIDSEGMENT`) en un sistema de saneamiento de pluviales.

## 6. Hecho en v0.48 (rama `bloque/maqueta`)

- **K1 y K2 completos**: buhardillas a dos aguas, a un agua y planas y ventanas de cubierta (herramienta Cubierta → «Buhardilla» o «Ventana de cubierta» en la barra de arriba, o desde el catálogo), con sus Propiedades (clase, ancho, altura del frente, pendiente, ventana, materiales); tejas, caballetes, canalones y bajantes en Propiedades de la cubierta → Cubrición. Planta de cubiertas, 3D, render, IFC (ida y vuelta), mediciones y BC3. `archisim cubiertas out/` hace un ejemplo con todo.
- **MQ1, primera vuelta**: interruptor y Tab; ventana sin columnas; catálogo Construir · Amueblar · Exterior con sus temas y miniaturas (las de planta de los tipos; las 3D, pendientes); tarjeta flotante con girar, duplicar, cuentagotas, borrar y Más…; plantas arriba y abajo; **muros recortados** (arriba, recortados a 1,20 m, bajos a 30 cm, también en la vista 3D técnica, con sus cortes rellenos); cuadrícula de 50 cm con ajuste; **Habitación** (cuatro muros, su suelo y su local de una vez; desde el catálogo).

Pendiente de MQ1: miniaturas en 3D, arrastrar objetos por el suelo en 3D sin herramienta, que la habitación sea un solo paso de Deshacer (hoy son tres: muros, suelo y local).

## 6 bis. Hecho en v0.49 (rama `bloque/maqueta-2`, revisión de Antonio del 23/10/2026)

- **Los muebles no se cortan**: los muros recortados bajan los muros, los forjados de arriba y la cubierta, pero los muebles, sanitarios, aparatos, plantas y coches de la planta quedan enteros (los de las plantas de arriba desaparecen enteros). La caja de sección, que es técnica, sigue cortándolo todo.
- **Muros delante bajos** (el «recorte» de Los Sims): los muros entre la cámara y el centro de la vista bajan a 30 cm y los del fondo quedan enteros; las puertas y ventanas van con su muro. Sigue a la cámara al girar. Es lo que se ve al entrar en la Maqueta; también en la vista 3D técnica.
- **Estirar en vivo**: mientras se arrastra un pinzamiento del objeto en 3D se ve el objeto como quedará (con sus cojines, puertas y cajones nuevos), dibujado en su sitio; el modelo cambia al soltar, en un solo paso de Deshacer.
- **Giro libre**: un anillo en el suelo alrededor del objeto seleccionado lo gira a cualquier ángulo, a saltos de 15° (con Mayús, libre); la tarjeta gira lo seleccionado el ángulo que se escriba al lado de «Girar» (90° por defecto); al colocar un objeto, Mayús+Espacio lo gira 15°.
- **Selección con contorno**: en 3D lo seleccionado ya no se tiñe de azul; se marca con un contorno naranja, y sus colores, materiales y texturas se ven tal cual mientras se cambian.
- **Muebles que crecen por piezas**: la sección `stretch` de las recetas (`docs/FAMILIAS.md`) hace que, al estirar, el sofá gane plazas, la mesa sillas, el armario y el mueble alto puertas, la estantería baldas y la cómoda columnas de cajones, cuando las de antes quedarían demasiado anchas o estrechas.
- **Más mullidos**: los cojines del sofá y la silla tapizada, la almohada y el edredón se hinchan en el centro (`Builder::pillow`), y en 3D las aristas redondeadas de los muebles se sombrean suaves, sin una línea en cada faceta.

## 6 ter. Colocar como en Paralives (v0.49)

- **Lo pequeño se apoya solo**: la decoración, las lámparas de mesa, las plantas y cualquier objeto pequeño (de menos de 70 cm de lado y 1 m de alto) se posan en la mesa, la balda, la encimera o el alféizar que hay debajo del cursor, en 3D y en planta (donde toma la cara más alta del mueble que hay debajo: el tablero de la mesa, no los respaldos de sus sillas). La vista previa ya sale encima.
- **Arrastrar muebles en la Maqueta**, sin herramienta: se pincha un mueble y se lleva por el suelo; se ve donde quedará mientras se arrastra. Cerca de un muro (a menos de 35 cm) se pega a él de espaldas y mirando a la habitación; si no, a la cuadrícula. Lo pequeño, arrastrado, se sube a lo que haya debajo.
- **Contra la pared al colocar**: en la Maqueta, un mueble que se coloca cerca de un muro ya se pone de espaldas a él.
- **Piezas que se unen**: dos muebles de cocina, muebles altos, estanterías, aparadores o sofás del mismo tipo de fondo y altura, colocados uno junto a otro en línea, se convierten en uno solo tan largo como los dos (y, por la sección `stretch` de su receta, con sus puertas o plazas). Lo dice la salida `joins = "width"` de la receta. Al colocar y al arrastrar en la Maqueta; un paso de Deshacer.

Desde v0.62 (C2) también al arrastrar en planta, y los módulos bajos de cocina se juntan además en esquina y espalda con espalda (§6 septies).

## 6 quater. Colecciones (v0.50)

- **Colección** encima del catálogo (en Amueblar y en Exterior): nórdico cálido, mediterráneo, años 50 o japandi, cada una con su tira de colores. Con una elegida, las tarjetas enseñan todas las piezas de la biblioteca con la tira de colores de la colección (desde v0.51, cada tarjeta es la pieza en 3D), y se colocan ya en sus maderas, telas, metales y cerámicas.
- **Aplicar** pasa lo seleccionado a la colección (una habitación entera, con un recuadro); «Sin colección» lo devuelve a la biblioteca. Un paso de Deshacer.
- Piezas nuevas en el mismo estilo: camas con cabecero tapizado o de lamas, colcha y cojines; mesitas de patas cónicas y flotantes; taburetes, islas y lo de la encimera; toallas y cestas; cuadros, espejos y relojes; cojines de suelo y mantas; y la pestaña **Jardín y terraza** en Exterior (tumbonas, sombrillas, macetones y pérgolas). Detalle en `docs/BIBLIOTECA_3D.md` («Colecciones»).

## 6 quinquies. Imagen de la Maqueta (v0.51)

- **Cielo y suelo con bruma**: en la Maqueta, detrás del modelo, un cielo que va del azul del cénit al blanco del horizonte y un suelo que se pierde en la bruma; lo lejano del modelo (árboles al fondo) se aclara con ella. La cuadrícula se dibuja en ese suelo. En Técnica, el fondo de siempre.
- **Luz**: menú «Luz» con **Día** (el sol del proyecto), **Atardecer** (sol bajo y dorado del oeste, sombras largas, cielo cálido) y **Noche** (cielo oscuro; las pantallas, bombillas y velas brillan, y cada lámpara alumbra lo que tiene alrededor, hasta 4,5 m, las 16 más cercanas a lo que se mira; la luz de las lámparas no atraviesa los muros).
- **Texturas de los muebles**: lino (hilos y nudos), bouclé (bucles), punto (puntos en V), terciopelo (brilla donde la tela se vuelve), mimbre (trenzado), terrazo (chinas de colores) y mármol (vetas). Las toman solas las telas, fibras y piedras de la paleta, por su nombre; se pueden elegir también en «Tipos y materiales», como las de siempre. Son dibujos del sombreador, sin imágenes.
- **Captura grande**: en el menú «Imagen», «Captura grande…» guarda la vista tal cual en un PNG de 3840 píxeles de ancho, con su cielo y su luz.
- **Miniaturas en 3D**: las tarjetas del catálogo son la pieza en 3D, vista de frente y un poco desde arriba, con sus colores y texturas (y los de la colección elegida). Se hacen unas pocas en cada fotograma; mientras tanto, el dibujo de planta.
- Arreglo: las líneas de contorno y el vidrio dejaban la imagen medio transparente (con fondo claro no se notaba; de noche salían líneas blancas).

Pendiente: la imagen realista (`archisim render` y Visualización) todavía no usa las texturas nuevas ni la noche; la luz de las lámparas no hace sombras.

## 6 sexies. Cámara y biblioteca (v0.52)

- **La cámara gira alrededor de lo seleccionado**: con algo seleccionado, arrastrar gira la vista en torno a ello, que se queda donde estaba en la pantalla (como en Revit o SketchUp). Sin nada seleccionado, alrededor del centro de la vista, como siempre. En las dos vistas 3D, Técnica y Maqueta.
- Mobiliario urbano con modelos de verdad, farolas que alumbran de noche, chimeneas y más mobiliario: `docs/BIBLIOTECA_3D.md`.

## 6 quater. Manejo común y barra del objeto (v0.54, rama `bloque/manejo`)

Del concepto de interfaz que Antonio aprobó el 24/10/2026 (tableros 1, 2 y 6), la primera vuelta:

- **Ratón, igual en planta y en 3D**: el botón izquierdo selecciona (clic), abre una ventana de selección arrastrando en vacío (de izquierda a derecha, lo que queda dentro; de derecha a izquierda, lo que toca) y **mueve** arrastrando lo seleccionado o un mueble; la rueda pulsada desplaza la vista y girarla acerca y aleja; el **botón derecho mantenido gira** la vista 3D (en planta, desplaza). Así, girar la vista ya no mueve nada sin querer.
- **Menú del clic derecho** (un solo clic, sin arrastrar) sobre un elemento: Propiedades, Cortar, Copiar, Pegar, Duplicar, Ocultar, Aislar, Seleccionar todos los de este tipo, Llevar a la planta y Borrar; en vacío: Pegar, Seleccionar todo y Mostrar todo. Ocultar y Aislar son de la sesión (no se guardan ni cambian el IFC). Cortar no borra nada: Pegar lo mueve donde se abrió el menú. Ctrl+C, Ctrl+X y Ctrl+V hacen lo mismo.
- **Tiradores con forma propia**: flecha azul para estirar, en la dirección en que crece; **crece solo hacia la flecha** y el lado opuesto se queda donde estaba (punto blanco mientras se estira); con **Alt**, por los dos lados. **Punto naranja** delante del objeto, sobre su anillo, para girarlo a mano sobre su centro: de 15 en 15°, y de grado en grado con Mayús o lejos del objeto. Mientras se estira, gira o mueve, se ve como quedará (la vista previa de la 0.49 de `bloque/maqueta-2`, ahora también para mover lo que no es un mueble y en la vista 3D técnica). En 3D, las flechas están un poco fuera del objeto, para que un mueble pequeño se pueda coger por su cuerpo.
- **Sin el cartel de teclas sobre el dibujo**: lo que hace el clic con la herramienta en uso está en gris al final de la barra de arriba (el resto, al dejar el cursor encima).
- **Barra del objeto** en la Maqueta, en lugar de la tarjeta flotante: abajo a la izquierda, sobre el catálogo, su dibujo y su nombre; Girar, Invertir (puertas, ventanas y muros), Duplicar, Igualar (cuentagotas), Borrar; **sus materiales** como muestras redondas que abren los colores del proyecto (solo cambian ese objeto: si su tipo lo comparten otros, se le hace uno propio); Más…
- **Puertas y ventanas en alzado** en las tarjetas del catálogo y en la barra del objeto, con el nombre entero en dos líneas.

## 6 quinquies. Cocina por módulos, catálogo en árbol, diálogos y carpinterías (v0.55, rama `bloque/cocina`)

- **Catálogo en tres niveles**: grupo (Construir, Amueblar, Exterior), tema (Cocina, Ventanas…) y, debajo, sus clases como fichas: «Todas», Bajos, Altos, Columnas, Campanas, Islas… en la cocina; Practicables, Correderas, Guillotina… en las ventanas; las carpetas de la biblioteca en lo demás.
- **Cocina por módulos**, como en Paralives: el **módulo bajo** (familia «Módulo bajo de cocina») es cajonera, puertas, fregadero, horno, lavavajillas, placa, botellero o esquina, con su encimera, que llega al borde del módulo: puestos uno junto a otro (se pegan solos al acercarlos), las encimeras forman una sola. La **columna** es despensa, horno y microondas o frigorífico integrado. El mueble alto lleva también estilo de frente. Frentes: liso, con marco, ranurado o con cristal.
- **La barra del objeto** enseña lo que se puede cambiar del mueble: qué lleva, cuántos cajones o puertas (botones 0 1 2 3 4), frente y tirador (en una lista, con «a toda la fila» para los módulos que se tocan en línea) y sus materiales. Su tarjeta lo dibuja en 3D.
- **Diálogos**: se abren junto al botón que los llama (a su izquierda si está a la derecha de la ventana), no plegables, con la misma forma redondeada y sombra; «Tipos y materiales», grande, en el centro.
- **Plano de carpinterías** (Archivo → Plano de carpinterías): cada tipo de puerta y ventana del proyecto en alzado a 1:50 con sus medidas, su sección y sus datos (familia, medidas, apertura, carpintería, unidades), seis por hoja A3, en PDF.
- `archisim cocina out/`: una cocina con columna, cajoneras, fregadero, lavavajillas, horno, placa y botellero, altos y campana, en cuatro filas con cada frente.


## 6 septies. Cocina y muebles, segunda vuelta (C2, v0.62, rama `bloque/c2-cocina`)

- **Una encimera de verdad** sobre los módulos bajos que se tocan, a la misma altura y del mismo material: en fila, en L, en U o en isla. En 3D, en secciones, en alzados y en el IFC es una sola pieza, con el hueco de cada fregadero, y los módulos van sin la suya. Se calcula de dónde están los módulos (`Model::worktop_runs`), no se guarda: separado del tramo, un módulo recupera su encimera.
- **Cocina en esquina**: un módulo girado un cuarto de vuelta, acercado al extremo de una fila, se pega con su lateral contra el frente del último módulo y su trasera a paño con el extremo de la fila, como en una esquina de dos paredes (también al revés, la fila hacia el tramo). La encimera sale en L.
- **Isla por módulos**: dos módulos espalda con espalda se pegan por la trasera, con sus extremos alineados; varias parejas hacen una isla bajo una sola encimera.
- **Pegarse al arrastrar en planta**: lo que se pega al colocar y al arrastrar en 3D (módulos de cocina, sofás, estanterías…) se pega también al arrastrarlo en planta; los que se unen en uno (sofás, estanterías) se unen al soltar.
- **Sofá rinconera** (familia «Sofá rinconera»): un tramo a lo largo del respaldo y otro hacia delante, a la derecha o a la izquierda; plazas en cada tramo y la de la esquina; los tres modelos del sofá. En la biblioteca, «Sofá rinconera 260×170» y «300×200».
- **Color libre**: en la barra del objeto, las muestras de una pieza tienen «Color libre», una rueda de color y «Aplicar»: un material como el que tenía (con su textura: la madera sigue con veta, la tela con su trama) en ese color, llamado «<material> #RRGGBB». Un paso de Deshacer.
- **Mediciones**: capítulo «10 Mobiliario de cocina», con las encimeras por material en metros a lo largo de los frentes (`encimera:<material>`) y los muebles de cocina por tipo en unidades (`mueble:<tipo>`), con sus precios del banco como las demás partidas.
- **IFC**: la encimera de cada tramo es un `IfcFurniture` «Encimera» (tipo `IfcFurnitureType` por material), con su material y el pset `ArchiSim_Worktop` (largo, superficie y los módulos); el tipo de un módulo bajo lleva dos geometrías, con y sin encimera, y los del tramo usan la segunda. Al importar, ArchiSim no lee la encimera como objeto: vuelve a salir de sus módulos.
- `archisim cocina out/` lleva además una cocina en L con fregadero y placa, una isla de seis módulos y un sofá rinconera.

## 6 octies. Detalles de la Maqueta y de la interfaz (C3, v0.64, rama `bloque/c3-maqueta`)

- **Habitación en un paso**: sus cuatro muros, su suelo y su local se deshacen y rehacen juntos («Habitación» en Deshacer). `Document::join_last` junta los últimos pasos; la app lo usa con `Editor::undo_mark` y `Editor::join_since`.
- **Mover en 3D junto a las paredes**: lo que se arrastra en 3D (con la flecha o como mueble) se pega a la cara de un muro cuando se acerca a menos de 15 cm, y después a otra cara que la cruce (una esquina). Mientras se mueve, unas cotas azules como las temporales de la planta miden lo que queda libre hasta los muros en las cuatro direcciones de la pieza (`wall_faces`: `faces`, `snap_delta`, `clearances`; en la app, `take_for_move`, `near_walls`, `object_near_walls` y `draw_clearances_3d`). Los muros y lo que va en ellos se mueven con sus propias reglas y no se pegan.
- **Puertas y ventanas por el centro, desde un extremo o repartidas**: en la barra de la herramienta, cuatro maneras (`OpeningPlace`). El tramo libre es el del muro bajo el cursor en la cara hacia la que abre: entre los muros que lo encuentran y las otras puertas y ventanas (`opening_tool::clear_span`, que usa `temp_dims::face_stops`). «Desde un extremo» pide la distancia de la jamba al extremo más cercano al cursor; «Repartidas», cuántas, con machones iguales entre ellas y en los extremos, en un solo paso.
- **Un dibujo para todos los sí o no**: las casillas de egui son interruptores (`widgets::switch`, también como `ui.switch` con `SwitchExt`); las opciones excluyentes de los diálogos, botones seleccionables.
- **Cocina**: los módulos bajos tienen «Vuelo delante», «Vuelo detrás» (una barra para sentarse) y «Copete» (sección Encimera). En un tramo, el copete va solo en los módulos sin otro módulo detrás (`WorktopRun::upstands`), se dibuja en 3D y en el IFC con la encimera y se mide aparte (`copete:<material>`).
- **Dos sofás en L hacen una rinconera**: un sofá recto colocado con su espalda contra el extremo de otro y su lado contra el frente de este, del mismo fondo y altura, se convierte con él en un «Sofá rinconera» del largo del primero y con el brazo del segundo (`placing::corner_sofa`, llamada desde `join_alongside`).

## 6 sexies. Ventana en islas y maneras de abrir (v0.56, rama `bloque/islas`)

Lo que faltaba de las láminas «Técnica» y «Piezas comunes» del concepto:

- **Islas**: cada panel del entorno Técnica es una tarjeta (radio 14, sombra) separada por 5 px sobre el fondo `Palette::ground`; paleta clara cálida.
- **Panel lateral** (`widgets::side_sheet`): para lo que se gestiona (Plantas, Capas, Estilos de presentación, Parámetros, Biblioteca). Entra deslizándose en 0,15 s, pegado a la derecha y con todo el alto entre las barras.
- **Burbuja junto al botón**: para lo corto. «+ Añadir planta» en el Navegador abre `levels_ui::new_level_bubble` con nombre, altura, encima o debajo, una pequeña sección y la consecuencia («Cubierta sube 3,00 m»); lo hace `Editor::insert_level` en un solo paso que se deshace.
- **Diálogo de decisión** (`widgets::decision`): en el centro con el fondo oscurecido, para lo que pide elegir (tipo repetido, purgar, informe de importación).
- **Avisos** (`chrome::toast_area`): abajo en el centro, con «Deshacer» solo si el último cambio tocó el modelo.
- **Tarjeta de tipo** de puertas y ventanas con Planta · Alzado · 3D (`properties::CardView`, alzado por defecto).

## 7. Bloques

| Bloque | Contenido |
|---|---|
| **K1** | Buhardillas y ventanas de cubierta (§5.1) |
| **K2** | Tejas, caballetes, canalones y bajantes (§5.2) |
| **MQ1** | Entorno Maqueta: interruptor, ventana sin columnas, catálogo, tarjeta flotante, cuadrícula, muros recortados, habitación arrastrando, cuentagotas |
| **V1** | Imagen (§4) |
| **MQ2** | Muebles modulares que se unen solos (encimeras, estanterías, bancos corridos), rueda de color por pieza |
| **MQ3** | Pinceles de Exterior: subir, bajar, alisar y aplanar terreno; pintar césped o grava; esparcir plantas; caminos con una curva arrastrable |
| **MQ4** | Flechas y aros sobre los volúmenes de Concepto para empujar, girar y escalar |
| **UI1** | Manejo común: ratón, menú del clic derecho, tiradores en flecha y anillo, estirar por un lado, barra del objeto (§6 bis) |
| **UI2** | Catálogo en árbol y módulos de cocina con encimera continua y estilos |
| **UI3** | Fichas, diálogos y avisos unificados; plano de carpinterías con alzados |
