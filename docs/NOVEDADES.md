# Novedades de ArchiSim

Lo que cambia en cada versión, en pocas líneas. ArchiSim enseña la parte de su versión antes de actualizarse (Ayuda → «Buscar actualizaciones…»), y la página de cada versión en GitHub la lleva también. El detalle, día a día, está en `BITACORA.md`.

Cada versión empieza con una línea `## <versión>` (por ejemplo `## 0.81.0`): es la que busca la publicación.

## 0.115.0

- **La planta, bonita desde el principio**: un proyecto nuevo dibuja su planta con el estilo «Lápiz», con muros macizos en gris oscuro, líneas grises finas y rellenos muy claros, como una planta de presentación. **F7** o el botón «Presentada» de la barra inferior enseñan la planta técnica y vuelven.
- **Árboles y camas con más vida**: los árboles de la plantilla salen con una copa dibujada hoja a hoja, con su borde de lóbulos, sus matas y sus trazos de hojas; las camas, con almohadas mullidas, edredón con su embozo y sus pliegues y una manta a los pies. Los árboles de la vegetación de la biblioteca (los del jardín y los que trae el entorno) estrenan el estilo «dibujada», con el mismo trazo cuidado.
- **Árboles y plantas blancos** en la planta presentada: el estilo «Lápiz» pasa los colores a aguadas claras, así que la vegetación sale dibujada con sus líneas, sin manchas grises. El formato del fichero sube a la versión 122: los proyectos guardados con esta versión no se abren en las anteriores.
- **Pavimentos colocados a mano**: con un local elegido, arrastra en planta el círculo para mover su trama, el punto naranja para girarla (se engancha a los muros y cada 15°) y la flecha para cambiar su tamaño. Los pavimentos empiezan en una esquina del local y lo acompañan al moverlo o girarlo; las tablas de la tarima se ven ya en la planta presentada. El formato del fichero sube a la versión 123.
- **Muros rematados**: los tipos de muro de la plantilla doblan sus revestimientos alrededor de los extremos libres, en lugar de enseñar todas las capas al final del muro.
- **Muebles con mejor dibujo en planta**: sofás, butacas, camas, mesas con sus sillas, armarios, sanitarios, electrodomésticos, la cocina, estanterías, pianos, plantas de interior con sus hojas, el mobiliario de terraza, oficina, bares y tiendas, bancos y papeleras con esquinas redondeadas, cojines y almohadas, y rellenos de blanco que tapan el suelo. Las alfombras quedan debajo de los muebles. En los proyectos que ya tienes, cada familia se pone al día con su botón «Actualizar» de la ventana Biblioteca.
- **Sofás y butacas limpios en alzados y secciones**: los cojines y las partes acolchadas se dibujan por su contorno y sus pliegues marcados, sin las decenas de líneas de sus caras redondeadas.
- **Suelo según el local**: al nombrar un local, su pavimento sale solo en las plantas presentadas: tarima en dormitorios y estancias, baldosa en cocinas y baños, losas en terrazas, hormigón en garajes y trasteros. Si eliges otro a mano, se respeta.

## 0.114.0

- **Espacio invierte el muro** mientras lo dibujas: la cara exterior pasa al otro lado de la línea, para ese muro y los siguientes, hasta pulsar Espacio otra vez.
- **Las puertas se eligen por su hoja**: un clic en la hoja o en el arco de apertura elige la puerta, sin coger el muro ni el local.
- **Tiradores más grandes** en muros, puertas, ventanas y objetos, que se cogen desde más lejos.
- **Ver antes de colocar**: el ascensor, los aparatos y las escaleras se ven bajo el cursor antes del clic, en planta y en 3D; en planta, lo que vas a poner lleva un velo azul transparente.
- **Encajar el entorno con precisión**: al moverlo o girarlo, los clics se pegan a las esquinas y bordes de los edificios, calles y parcelas, y junto al cursor sale el ángulo.

## 0.113.0

- **La prueba de los 10 minutos** (bloque W0, `docs/WORKFEEL.md` §4) ya es una prueba automática: una vivienda de 90 m² con salón-cocina, dos dormitorios y baño, una puerta de entrada, tres de paso y cuatro ventanas, el forjado, los cuatro locales con nombre y una lámina A3 de presentación en PDF, dibujada con clics y teclas como lo haría una persona. Cuenta lo que cuesta cada paso (hoy, 90 acciones: 28 clics en el dibujo, 52 teclas y 10 clics en paneles) y avisa si una versión necesita más que la anterior.

## 0.112.0

- **Puntería al dibujar** (bloque W2a, `docs/WORKFEEL.md`):
  - Los puntos (extremos, esquinas, puntos medios, intersecciones…) se cogen desde unos 18 píxeles, casi el doble que antes.
  - El punto cogido se queda aunque la mano tiemble: solo se suelta al alejarse una vez y media esa distancia, o al acercarse claramente a otro.
  - Junto al cursor aparece lo que vas a coger: Extremo, Esquina, Punto medio, Sobre la cara, Intersección, Perpendicular… Con el muro, además, «continúa el muro», «encuentro en T» o «Cerrar».
  - La cadena de muros se cierra al volver cerca del primer punto, sin tener que acertar en él.

## 0.110.0

- **Las manzanas siguen a las calles.** Al mover, curvar o trazar una calle, las manzanas que cambian de forma se vuelven a parcelar solas con la zona que tenían, y se vuelven a edificar si lo estaban. Las parcelas bloqueadas y las dibujadas a mano no se tocan. Los edificios por los que pasa una calle nueva desaparecen. Un aviso dice qué se ha rehecho, y todo se deshace en un paso.
- **Edificios de esquina con dos fachadas.** La parcela de esquina de una manzana se edifica entera, con fachada a las dos calles. Las casas con jardín delantero siguen como estaban.
- **Esquinas de parcelas y superficies.** Las esquinas de una parcela, una plaza, un aparcamiento o un jardín se mueven arrastrándolas.
- **Plazas con huecos y jardines hundidos.** «Editar contorno» en una plaza, un aparcamiento o un jardín permite dibujarle huecos con su encintado (un alcorque, una fuente, un parterre) y bordes curvos. Con altura negativa la superficie queda hundida y recorta el terreno.
- **Aparejos del pavimento:** cuadrícula, espiga, rombo o a matajunta.
- **Tramos de calle a la vista.** En planta, los tramos con materiales propios se tiñen con su color, y sus extremos se arrastran con tiradores.
- **Parcelas como un casco real.** En el casco antiguo y en las casas, los frentes de las parcelas varían.
- **Flechas repetidas** en los accesos largos a un cruce.
- **Ocultar piezas por ventana.** Con «Ocultar piezas», arrastrar una ventana oculta todas las que caen dentro. Las ocultas se ven en gris y un clic en una la devuelve.
- **Tráfico en la Maqueta:** tarjeta propia en «Edificios y calles».
- **Peatones que cruzan.** En el tráfico de ilustración, la gente cruza por los pasos de peatones.
- **El área recorta las calles.** Al traer el entorno, también se recortan al área las calles que ya estaban en el proyecto.
- **Nudos a medida.** El centro de un cruce se mueve sin mover las calles; un brazo puede aplanarse los últimos metros; el fin de una calle puede ser un martillo en lugar de un círculo de giro. Los tiradores del nudo se usan también en la vista 3D y en la Maqueta, con sus conexiones de carril a la vista.
- **Carriles.** Flecha de cambio de sentido; un carril puede quedarse sin salida; las esquinas de las isletas pintadas se arrastran; las vías del tranvía siguen de frente por el cruce.
- **Mobiliario por tramo.** «Amueblar el tramo» pone en un tramo de calle los árboles, farolas, bancos y papeleras que elijas.
- **Sustituir todos en cualquier zona:** un rectángulo, un contorno a mano o una parcela o manzana con un clic.
- **Aparcamientos accesibles.** Plazas de movilidad reducida de 3,70 m en azul con su símbolo, plazas numeradas y filas que siguen el lado que elijas.
- **Soltar flechas y señales.** «Soltar piezas» convierte una flecha de un cruce en una marca vial y una señal en un objeto «Señal de tráfico», para moverlas. Ocultar y soltar piezas funciona también desde la vista 3D.
- **Pincel de zonas** para dar la misma zona a muchas parcelas de una pasada.
- **Entorno a tu medida.** El recuadro se gira y el contorno se puede dibujar a mano; el terreno se trae con más detalle si se pide; las calles que salen del área dicen a dónde van.
- **Tráfico más vivo.** Autobuses en sus carriles, tranvías por sus vías, y los coches y personas cercanos con su modelo de la biblioteca.
- **Trazar con cotas.** Al trazar, el ángulo que gira la calle y el radio de la curva se dibujan en la planta; al elegir una pendiente uniforme se ve el tramo entre los dos clics.

## 0.109.0

- **Tiradores de las calles en 3D.** Una calle seleccionada se moldea también en la Vista 3D: nudos, curvatura y bordillos, viendo cómo queda con sus cruces.
- **Tiradores al pasar el ratón.** En planta salen los tiradores de la calle que hay bajo el cursor, sin seleccionarla antes.
- **Las calles se unen solas.** El extremo suelto de una calle se pega a la calle, al extremo, al cruce o a la glorieta cerca de la que se suelta. Si sigue en línea con otra calle, las dos pasan a ser una sola; si son de tipos distintos, con una transición entre las dos secciones.
- **Cruces que se juntan.** Un cruce que quedaría pegado a otro se junta con él: la calle se curva suave para pasar por su centro.
- **Ensanchar un tramo.** Las flechas de los bordillos ensanchan o estrechan el tramo entre dos cruces, con transiciones suaves.

## 0.108.0

- **Ver la calle real mientras se traza.** En planta se ve la calle tal como quedará, con sus aceras y el cruce ya formado con las que pisa. En 3D aparece con sus materiales, y las calles que encuentra se rehacen con sus cruces en el acto.
- **Moldear las calles arrastrando.** Al seleccionar una calle solo salen sus nudos (extremos y cruces) y un tirador redondo en medio de cada tramo.
  - Al tirar del redondo, el tramo se curva suave.
  - Al mover un nudo, se lleva las calles que llegan a él, que se doblan suaves hasta su nudo siguiente.
- **Cruces que no se pisan.** Al trazar, el clic se pega al centro del cruce que hay cerca. Si la calle va a hacer un cruce encima de otro, la etiqueta se pone roja.
- **Curvas continuas en lo traído de OpenStreetMap.** Los quiebros de las calles se redondean según la clase de vía.

## 0.107.0

- **Las calles traídas de OpenStreetMap, como una red.** Antes, cada trozo de OpenStreetMap llegaba como una calle suelta, y una glorieta o una avenida con dos calzadas se rompían en piezas que se pisaban. Ahora:
  - las glorietas llegan como glorietas, con su isleta ajardinada;
  - la calle que cruza por el medio de una glorieta la atraviesa entera;
  - las dos calzadas de una avenida se juntan en una sola avenida con su mediana, al ancho real;
  - los trozos de una misma calle se unen en una sola calle hasta el siguiente cruce.
- Sin árboles, farolas ni coches dentro de las glorietas.

## 0.106.0

- **La barra de calles.** Con la herramienta Calle, una barra pequeña abajo en el dibujo (en planta, en 3D y en la Maqueta) lleva todo lo de trazar: el tipo de calle con sus tarjetas, cómo se traza (continua, recta, curva o eje), seguir el terreno, anarquía y la altura con − y + o RePág y AvPág, para pasos elevados y subterráneos.
- **Cuadrícula, Paralela y Mejorar.** Una cuadrícula de calles con tres clics, con sus cruces y cebras, en un solo paso de Deshacer. Una paralela a la distancia que se diga, del lado donde se hace clic. Y «Mejorar»: otro tipo y otros materiales sobre una calle que ya está, sin mover su eje.
- **Medidas mientras se traza.** Junto al cursor van la longitud del tramo, su rumbo, el radio y la pendiente.

## 0.105.0

- **Encajar el entorno.** El terreno, la ortofoto, los edificios del entorno, las calles, las parcelas y los árboles traídos se mueven, giran y suben en bloque, sin tocar el edificio (Urbanismo → Territorio → Encajar el entorno; se abre solo al traer el entorno). Se encajan por dos puntos, sin saber coordenadas ni ángulo: esta esquina del Catastro va sobre esta esquina de la nave, y una segunda pareja da el giro. También se pueden mover, girar o subir tecleando, y llevar un punto del terreno a la cota ±0,00. La georreferencia y el norte se corrigen solos, así que el IFC sigue bien situado.
- **Lo que pisa el edificio.** Los edificios del entorno que caen sobre el proyecto se ocultan o se quitan con un botón, y todo el entorno se puede ocultar de una vez para trabajar en el edificio.
- **Mover o girar el terreno** lo lleva entero: su borde, sus explanaciones y las zonas pintadas van con sus puntos.

## 0.104.0

- **Cuarta vuelta de fluidez del territorio.** Cambiar un nudo, el pavimento de un tramo o una pieza oculta de una calle ya no rehace en 3D todo lo que hay a lo largo de la calle, solo lo de alrededor: en un sitio de 1 km², elevar un cruce pasa de medio segundo a una décima.
- **Tráfico más creíble.** Los coches ya no se pisan dentro de los cruces: esperan a que pase el que cruza, y con ceda el paso o stop dejan pasar a quien viene por la otra calle. Los semáforos se ven en rojo, ámbar y verde en 3D y en el render.
- **Volver a parcelar una manzana.** Un clic en una manzana que ya tiene parcelas la parte de nuevo con la zona y las medidas elegidas, sin tocar las parcelas bloqueadas.
- **Mapa con leyenda y para guardar.** La ventana Mapa tiene «Guardar el mapa…» (PNG o SVG) y una leyenda con las zonas que hay.
- **Soltar un paso de peatones** deja los bordillos rebajados de la calle donde estaban.

## 0.103.0

- **Tráfico de ilustración (N9).** Urbanismo → Territorio → Tráfico pone coches y personas en la vista 3D. Los coches van por sus carriles, giran por donde mandan las conexiones de cada cruce y paran en los semáforos y en los stop. Las personas pasean por las aceras. El render los saca también. Es solo para ilustrar: no se guarda en el proyecto ni calcula capacidades.

## 0.102.0

- **Área grande y vista de mapa (N8).** El entorno se puede traer en un recuadro de hasta 2 × 2 km. Las calles se cortan en su borde con toda su sección y quedan como conexiones con fuera, con una flecha y su nombre. Urbanismo → Territorio → Mapa enseña toda el área plana y en color, con sus calles, edificios, árboles y zonas; un clic en el mapa lleva la planta a ese sitio.

## 0.101.0

- **Manzanas y parcelas (N7).** Urbanismo → Territorio → Manzanas: las calles que se cierran forman manzanas; un clic las parte en parcelas por la línea de fachada (con sus parcelas de esquina y su patio) y se llenan con edificios del lugar según su zona: casco antiguo, bloques, viviendas aisladas, comercio o equipamiento. «Rehacer» las vuelve a llenar; una parcela bloqueada no cambia.

## 0.100.0

- **Ocultar y soltar piezas de las calles (N6).** «Ocultar piezas» quita de un clic una raya, una plaza pintada, un alcorque, una señal o un bolardo que pone la calle, y siguen ocultos aunque la calle se mueva. «Soltar paso de peatones» convierte la cebra de una calle en una marca vial suelta para moverla o girarla.

## 0.99.0

- **Plazas, isletas, medianas, aparcamientos y jardines (N5).** Se dibujan por su contorno, pegándose a los bordillos y a las líneas de fachada, con su borde (bordillo, rigola o encintado), su altura sobre lo de debajo, las juntas del pavimento y, en los aparcamientos, las plazas pintadas en batería o en espiga, con su número.

## 0.98.0

- **Calles por tramos y «Sustituir todos» (N4).**
  - **Por tramos**: un trozo de una calle con otro pavimento, bordillo, acera o tierra (grupo «Por tramos» de la calle).
  - **Sustituir todos**: cambia de una vez un tipo de objeto (árbol, farola, banco…) o un material de las calles por otro, en todo el proyecto, en la planta, en una zona dibujada o en un tramo de calle.

## 0.97.0

- **Carriles y marcas de los cruces (N3).** Con la herramienta Nudo:
  - Se ve **a dónde va cada carril** que entra al cruce.
  - **Giros por carril** (← ↑ → ↶) en Propiedades, y **conexiones a mano** de un carril a otro pinchando en planta.
  - **Flechas pintadas** en los carriles y líneas continuas entre ellos antes del cruce, que salen solas de los giros.
  - **Línea de detención** retrasada por calle.
  - **Isletas pintadas** (cebreado, relleno o borde) que van con el cruce si se mueven sus calles.

## 0.96.0

- **Trazado de calles, remates (N2).**
  - **Pendiente uniforme con el ratón**: en la calle, «Elegir los dos puntos en planta» y dos clics sobre ella.
  - **Radio y giro en la barra de opciones** al trazar por tramos (R y Giro, junto a L y ∠): Intro pone la curva.

## 0.95.0

- **Trazar calles sin reglas y con precisión (N2).**
  - **Anarquía**: una calle que no se une a las que pisa ni modela el terreno, a la cota que le des (pasos elevados, rampas, maquetas). En la herramienta Calle y en cada calle.
  - **Pendiente uniforme** entre dos puntos de una calle, con la pendiente que resulta a la vista antes de aplicarla.
  - **Paralela** a una calle a la distancia que teclees, con su sección y su rasante.
  - **Al trazar**, la longitud, el giro y el radio hasta el cursor, y un tramo en curva de radio y giro tecleados, tangente al anterior.
  - **Hacer glorieta** sobre un cruce desde la herramienta Nudo.

## 0.94.0

- **Nudos de las calles, a mano (N1).** Herramienta **Nudo** en Urbanismo (NU): pincha un cruce, una T, una L o el final de una calle y cámbialo en Propiedades o con sus tiradores naranjas.
  - **Tipo**, en tarjetas con dibujo: cruce normal, **cruce elevado** (meseta con rampas), **plataforma única** (sin bordillos, con bolardos), **continuo** (una calle sigue en otra y cambia de sección sin costura), **doblez** (una curva con las aceras siguiendo el giro), **fin** (fondo de saco con círculo de giro) o **sin nudo**.
  - **El nudo entero**: plano a una cota, la **calle principal** que manda (y ceda el paso en las demás de un clic) y sin marcas viales cerca.
  - **Cada calle que llega**: retranqueo, **ceda el paso, stop o semáforo** con su línea en la calzada y su señal, y **paso de peatones** con su bordillo rebajado.
  - **Cada esquina**: radio, forma (arco, **chaflán** o en punta), acera y **oreja** sobre el aparcamiento.
  - **Chaflán estándar** del lugar del proyecto en un nudo o en toda la planta.
  - **Tiradores en planta** para el retranqueo y el radio, con la medida mientras arrastras.
  - Solo se guarda lo cambiado; si mueves una calle el nudo conserva sus ajustes, y si la borras se pierden. Van en el fichero y en el IFC.
  - `archisim nudos` hace un ejemplo con todo.

## 0.93.0

- **Taller del territorio: fluidez.**
  - **La vista 3D, mucho antes después de un cambio**: en un sitio de 1 km² con 90 calles, 560 edificios y 4 662 árboles, mover una calle rehacía el 3D en unos 40 s; ahora en unos 0,1 s. Los árboles no vuelven a crecer, los edificios y el terreno que no cambian se aprovechan tal cual, y el terreno se rehace solo en los cuadrados de 50 m por donde pasa la calle.
  - **Terreno mucho más rápido**: la cota del terreno bajo un punto ya no vuelve a triangular todo el levantamiento cada vez. Trazar las calles pasa de 2 s a 0,2 s y el terreno que modelan, de 17 s a 0,2 s al mover una calle.
  - **Cruces y calles guardados**: la planta, el 3D y el IFC comparten los cruces y cada calle mientras no cambian.
  - **Planta más ligera**: lo que queda fuera de la vista no se calcula.
  - **La vista 3D se rehace solo donde cambia**: mover un árbol, girar un banco o cambiar un edificio se ve en unas centésimas de segundo, y mover una calle en una décima, aunque el sitio tenga un kilómetro cuadrado. La tarjeta gráfica solo recibe los trozos nuevos.
  - **Sitio de prueba**: `archisim territorio-grande` hace un kilómetro cuadrado de ciudad inventada y mide lo que cuesta cada cosa.
- **Montar un proyecto desde los planos de AutoCAD.** La nave SUMO (antiguo CEOSA) se ha montado desde el DXF del proyecto básico, con sus referencias unidas: 4 plantas (baja ±0,00, primera +4,45, cubierta +8,45 y cubierta del castillete +12,35), 302 muros con su espesor real, 99 puertas, ventanas y huecos, 62 tramos acristalados como muros cortina, forjados y los 104 locales con su nombre.
  - Los muros salen de las dos líneas de corte de cada muro, y las puertas del arco de su hoja (con su mano y hacia dónde abren).
  - Nueva orden `archisim montar <descripción.txt> <proyecto.archisim>`: levanta plantas, muros, carpinterías, muros cortina, forjados y locales desde una lista de texto. Los guiones de `scripts/montar_dxf/` escriben esa lista desde un DXF dibujado en capas.
- **IFC: forjados atravesados por muchos muros.** Cuando los muros formaban un anillo cerrado, el hueco que dejaban en el forjado se escribía como un contorno que se tocaba a sí mismo, y otros programas le quitaban de más (hasta 1 m³). Ahora se escribe un hueco por muro y el volumen cuadra.
  - Al importar, ArchiSim lee también los forjados con cadenas largas de huecos.
- **Panel de lamas orientable** (Objetos → Fachada): panel de lamas verticales de madera que gira sobre su eje, delante de una fachada, de ancho, alto, ancho y grueso de lama y separación a elegir. En IFC sale como protección solar (`IfcShadingDevice`) con su tipo, y vuelve igual al importarlo.
- **La nave SUMO, segunda vuelta**, con el render del proyecto como guía:
  - Las 43 lamas de la planta primera son paneles de lamas con el giro que tienen en el plano, no muros.
  - 57 pilares redondos de acero blanco de Ø20 en el perímetro.
  - El forjado de la primera y la cubierta llegan hasta la línea del vuelo, en hormigón blanco. La cubierta es plana, invertida y con grava, abierta sobre los patios.
  - Ocho jardineras (en cubierta, en la primera y en la entrada) con peto blanco, tierra y plantación de palmitos, pennisetum, romero y lavanda.
  - Fachadas enfoscadas en blanco y castillete revestido de madera, con su contorno real.
- **IFC**: las lamas de los muros cortina salían con un tipo (`LOUVER`) que no existe en IFC4 ni en IFC 4.3. Ahora salen como «Lama» definida por el usuario.

## 0.92.1

- **La planta y las secciones ya no se quedan esperando.** Si un dibujo no se puede hacer, la vista lo dice en rojo con el motivo (y queda anotado en `errores.log`) en lugar de quedarse en blanco o en «Dibujando la sección…» para siempre.
- **Vista → «Dibujar aparte».** Apagado, las plantas, secciones y alzados se dibujan al momento, como antes de la 0.85.

## 0.92.0

- **El muro cede al pilar.** Donde un pilar está metido en un muro, desaparecen las capas del muro que el pilar atraviesa; si el muro es más grueso y su trasdosado o revestimiento pasa por delante del pilar, ese sigue por encima. En planta, en 3D, en secciones y en el IFC.
- **Juntas a medida en el despiece a mano.** Elige una junta (un clic, aunque tengas Pintar activo): salen sus extremos y su largo en azul. Arrastra un extremo y la junta se acorta o alarga sin torcerse, por ejemplo para que una vertical pare en una horizontal; o escribe el largo en la cota.
- **Más fluidez.** El alzado del despiece se dibuja con margen y en segundo plano, como la planta; elegir en alzados y secciones es más rápido; el medidor (Ctrl+Alt+F) dice también lo que tarda pintar la ventana.

## 0.91.0

- **ArchiSim en el iPad, remates.**
  - **Menos memoria en 3D**: Florence Street en 3D pasa de 1,16 GB a 635 MB en el navegador. En el iPad, las piezas redondas llevan menos lados y las plantas van en su versión media.
  - **Cálculos sin parar la pantalla**: la estructura y la instalación eléctrica se calculan aparte, también en el escritorio; mientras, puedes seguir trabajando.
  - **Programa más ligero**: de 51 MB a 26 MB (9 MB por la red); carga antes la primera vez.
  - **Exportar en la web**: planos en PDF, DXF o imagen, documentos, anejos, esquemas, BC3, Excel, modelo 3D, render e imagen del 3D, como ficheros que guardas en Archivos o descargas.
  - **Interfaz del iPad**: al tocar la pantalla, la cinta se recoge y el Navegador se guarda en pantallas estrechas; el teclado numérico sale junto al punto; la lupa también en secciones; «Táctil ▾» en la barra de abajo, que se desliza si no cabe.
  - **3D sin WebGPU**: la vista 3D funciona también con WebGL (Safari anterior al 26).

## 0.90.0

- **ArchiSim en el iPad: lo pesado.**
  - **Render sin parar la pantalla**: en el navegador, la imagen realista se reparte entre varios hilos (hasta cuatro) y la ventana sigue respondiendo mientras avanza la barra.
  - **Traer el entorno desde el iPad**: Catastro, OpenStreetMap, IGN, la ortofoto y el buscador de lugares funcionan en la versión web a través del NAS, que hace de puente. Solo a esos servicios, y sin pasarles tu contraseña. Hay que copiar el `Caddyfile` nuevo al NAS.
  - **Memoria**: el medidor (Ctrl+Alt+F) dice cuánta memoria usa el programa en el navegador.

## 0.89.0

- **ArchiSim en el iPad: a la medida del dedo.**
  - **Tamaño táctil**: en cuanto tocas la pantalla, botones y filas más grandes, y los puntos de los muros, los tiradores y lo que seleccionas se cogen desde más lejos. Se apaga en la barra de abajo.
  - **Teclado numérico** sobre el dibujo mientras tienes una herramienta en la mano o algo seleccionado: cifras, coma, menos, Retroceso, Tab para pasar a la siguiente cota, Intro, Esc y Espacio para girar. Escribe las longitudes de los muros y las cotas temporales sin teclado. Se recoge en «123».
  - **Lupa** con «El dedo dibuja»: mientras dibujas con el dedo, encima sale lo que tapa la yema, al doble, con el punto que se va a coger.
- También en un ordenador con pantalla táctil.

## 0.88.0

- **ArchiSim en el iPad: el Pencil y los dedos.** Sobre el dibujo (planta, sección, plano o 3D):
  - El **Apple Pencil dibuja**: muros, forjados, cotas, seleccionar y arrastrar, como el ratón.
  - **Un dedo mueve la vista**: en planta la desplaza, en 3D la gira.
  - **Dos dedos**: pellizca para acercar o alejar, arrástralos para desplazar.
  - **Toque** con el dedo: un clic. **Mantener** el dedo quieto: el menú del botón derecho.
  - **Tocar con dos dedos**: Deshacer. **Con tres**: Rehacer.
  - Sin Pencil, **«El dedo dibuja»** (en la barra de abajo): un dedo dibuja y dos mueven la vista. Viene encendido hasta que el iPad ve el Pencil.
  - En los paneles, la cinta y los diálogos el dedo funciona como el ratón.
- También en un ordenador con pantalla táctil.

## 0.87.0

- **ArchiSim en el iPad: abrir y guardar.** En la versión web ya se abren proyectos e IFC y se guardan.
  - **Abrir** (Ctrl+O o Archivo → Abrir): «Elegir archivo…» deja coger el proyecto de Archivos, iCloud Drive o cualquier carpeta del iPad. También se pueden soltar sobre la página.
  - **Guardar** (Ctrl+S): «Guardar en Archivos…» deja escoger la carpeta, también iCloud Drive; «Descargar» lo deja en Descargas. Exportar IFC 4.3 e IFC4, igual.
  - **No se pierde nada**: el navegador guarda una copia medio minuto después de cada cambio y la abre la próxima vez. Si cierras la página con cambios sin guardar, pregunta antes.
  - Las preferencias (tamaño de letra, tema, misiones) se recuerdan.
  - Lo demás que se exporta (PDF, DXF, BC3, imágenes…) aún no está en la web; lo dice al pedirlo.
- En el escritorio no cambia nada.

## 0.86.0

- **ArchiSim en el iPad (primera vuelta).** ArchiSim funciona también en el navegador, con la gráfica del iPad (Safari 26). Se sirve desde tu NAS con usuario y contraseña y se pone al día solo con cada versión (`scripts/nas/README.md`).
  - Abre la casa de ejemplo en planta y en 3D.
  - Todavía no abre ni guarda archivos (llega en la próxima vuelta), ni descarga el entorno.
  - En el iPad: Compartir → «Añadir a pantalla de inicio» lo deja como una app.
- En el escritorio no cambia nada.

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
