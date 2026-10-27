# Biblioteca básica de modelos 3D (L3)

Este documento explica para Antonio cómo hace ArchiSim el 3D de los muebles, sanitarios, la cocina, las personas y los coches de la biblioteca, y qué se puede cambiar.

## La idea

Con los árboles (`docs/VEGETACION.md`) ya hacemos lo mismo: ArchiSim no descarga modelos de nadie. **Genera el 3D a partir de las medidas del tipo.** Un sofá de 2,40 m con tres plazas sale con tres cojines de asiento y tres de respaldo, y si lo alargas a 3 m con cuatro plazas, se rehace con cuatro. Así:

- no hace falta pedir licencia a nadie (sala limpia, sin catálogos de terceros);
- el modelo siempre coincide con el símbolo de planta y con las medidas de Propiedades;
- pesa poco: cada mueble tiene entre unos cientos y unos pocos miles de triángulos.

El dibujo de planta sigue saliendo de la receta de la familia (`plan 1:50`, `plan 1:100`); lo que cambia es el 3D, que antes eran cajas y ahora es un modelo con detalle.

## Qué hay

| Familia | Modelos 3D |
|---|---|
| Sofá | Contemporáneo, Clásico (brazos enrollados y patas torneadas), Nórdico (patas de madera inclinadas) |
| Silla | Madera, Tubo metálico, Tapizada |
| Mesa | Cuatro patas, Patas metálicas, Caballetes; con sus sillas alrededor si se piden |
| Mesa de centro | Madera con balda, Bloque, Vidrio y metal |
| Cama | Tapizada, Madera, Tarima; con almohadas y edredón |
| Mesita de noche | Cajonera, Con patas; con lámpara |
| Armario, Estantería (con libros), Mueble TV (con televisor) | uno cada uno |
| Muebles de cocina | Zócalo, cuerpos, puertas con tiradores, encimera con fregadero, grifo y placa |
| Muebles altos, Frigorífico combi, Lavadora | uno cada uno |
| Inodoro, Bidé | De pie, Suspendido |
| Lavabo | De pedestal, Con mueble, Suspendido |
| Bañera | Rectangular, hueca de verdad |
| Ducha | Plato, Plato y mampara, Esquina con mampara |
| Persona | Hombre, Mujer, Caminando (la altura se cambia) |
| Coche | Berlina, Compacto, Todoterreno (el largo se cambia) |

La plantilla trae 19 tipos nuevos para enseñarlos (Sofá clásico de 3 plazas, Sofá nórdico de 2 plazas, Silla metálica, Silla tapizada, Mesa de caballetes 200×100, Mesa de patas metálicas 140×80, Mesa de centro, Mesa de centro de vidrio, Mesita de noche, Mueble TV 180, Cama de madera 160×200, Cama tarima 150×200, Inodoro suspendido, Bidé suspendido, Lavabo con mueble 80, Lavabo suspendido 60, Ducha con mampara 120×80, Mujer, Todoterreno) y diez materiales para ellos (Tela, Tela clara, Edredón, Cromado, Negro mate, Libros, Libros claros, Encimera, Piloto, Vidrio doble).

## Cómo se cambia

En Propiedades del tipo (o en Tipos y materiales), cada familia tiene un grupo **Modelo 3D**:

- **Modelo 3D**: el estilo (Contemporáneo, Clásico…).
- Un material por pieza: tapicería, cojines, patas, encimera, vidrio, cromados… Cambia el material y cambia el color o la textura en 3D y en el IFC.

Las medidas de siempre (ancho, fondo, alto, plazas, sillas…) mueven el modelo.

## En el IFC

Cada tipo lleva su modelo como mapa de representación (`IfcRepresentationMap` con `IfcTriangulatedFaceSet` y el color de cada pieza), igual que los modelos importados de L2: los ejemplares lo comparten, así que un comedor con veinte sillas iguales no repite veinte veces la malla.

## Para el programador

- Crate `archisim-props` (sin dependencias): `make(kind, params)` devuelve un `PropMesh` con triángulos por *papel* (`mat`, `frame`, `cushion`, `glass`, `metal`…); `mesh_of` lo guarda. Piezas en `build` (cajas redondeadas, cilindros, tornos, elipsoides, huecos).
- La familia lo pide con `outputs generator = "props:<kind>"` y el parámetro `model3d`; `archisim_core::Model::type_prop` traduce los valores de la familia y asigna a cada papel el material de la familia (`candidates`).
- 3D: `archisim_geom::solid::prop_solids`. IFC: `archisim_ifc::objects::mesh_map`.
- `archisim modelos out/` hace `modelos.archisim`, `modelos.ifc`, `modelos-3d.svg` (axonometría) y `modelos-planta.svg` con los 41 objetos en filas.

## Muebles y accesorios (bloque 5 de la revisión del 22/10/2026, v0.45)

Antonio pidió muebles estirables desde la vista 3D, más parámetros (número de cajones, posición, tipos de pomo) y accesorios: los de ArchiCAD que siguen a muros, forjados y cubiertas, y los de baño y cocina.

**Estirar en 3D.** Con un mueble seleccionado en la vista 3D salen cuadros azules a sus lados, en su frente y encima: se arrastran y cambian su ancho (crece por los dos lados), su fondo y su altura, como los tiradores de la planta. Si su tipo lo usan otros objetos, se copia, así que solo cambia ese.

**Frentes.** Las familias con puertas o cajones tienen un grupo **Frentes**:

| Familia | Lo que se elige |
|---|---|
| Mesita de noche | Cajones (1 a 6) y tirador |
| Armario | Filas de cajones abajo (0 a 4) y tirador |
| Encimera de cocina | Cajones bajo la placa (cajonera de 60 cm) y tirador |
| Mueble alto de cocina | Tirador |
| Mueble TV | Frentes de cajón o puertas, y tirador |
| **Cómoda** (nueva) | Cajones por columna, columnas, tirador; zócalo o patas |
| **Escritorio** (nuevo) | Cajonera a la izquierda, a la derecha o ninguna, cajones, tirador; patas de madera o estructura metálica |

**Tiradores**: barra, pomo, uñero, asa o sin tirador (de empuje). Las puertas llevan el tirador junto al canto que abre y los cajones arriba y centrado.

**Baño y cocina** (familias nuevas, colgadas a su altura): espejo (con marco, sin marco o redondo), toallero (barra o radiador toallero), portarrollos, barra de apoyo (fija o abatible, para baños accesibles), campana extractora (decorativa con chimenea o integrada) y balda (con escuadras o flotante).

**Accesorios de los tipos de muro y de forjado**, como los barridos de Revit: en Tipos y materiales → el tipo de muro → **Accesorios**, se añaden rodapiés, molduras, cornisas y albardillas. Cada uno dice en qué cara va (exterior, interior o coronación), a qué distancia del pie o de la coronación y de qué material. Siguen a todos los muros del tipo, cierran las esquinas y **se cortan en las puertas y ventanas que cruzan su altura** (el rodapié se corta en las puertas y la cornisa sigue por encima). En el tipo de forjado, **Canto**: cornisa o alero alrededor de todos sus forjados horizontales. En planta se ven como una línea fina (discontinua si están por encima del plano de corte); en 3D, secciones y alzados, con su material; en el IFC, cada tramo es un `IfcCovering` (`SKIRTINGBOARD`, `MOLDING`, `COPING`) que cubre su muro o forjado, y al importar un IFC de ArchiSim vuelven a sus tipos. Los vierteaguas siguen siendo parte de las ventanas (v0.32).

La demo trae rodapiés de madera en los muros de fachada y en los tabiques y una cornisa de piedra alrededor de la cubierta. `archisim modelos out/` enseña todos los muebles y accesorios nuevos, con el frente hacia quien mira.

Para el programador: `archisim_props::fronts` (`Handle`, `drawer`, `door`, `drawers`; `CHOICES` y `choice_number` traducen los valores elegidos por nombre), `accessories` (baño y cocina), `chest` y `desk`; `app/grips3d` (estirar en 3D, `drag::set_object_size`); `archisim_core::WallSweep`, `SlabEdge` (`model/sweeps`, `WallType::sweeps`, `SlabType::edge`; formato versión 29), `archisim_geom::sweeps` y `sculpt::sweep_closed`, `archisim_ifc::accessories`.

## Símbolos de planta desde DXF (bloque 6 de la revisión, v0.46)

Cualquier objeto puede dibujarse en planta con un símbolo hecho en CAD: los árboles, arbustos, personas o muebles que el estudio ya tiene dibujados. Selecciona un objeto → Propiedades → **Símbolo de planta** → **Importar DXF…**. Se trae todo el dibujo del fichero (los bloques, despiezados; los textos y cotas se quedan fuera), con los colores y grosores que tiene, centrado en el objeto. Lo dibujan todos los objetos de ese tipo.

- **Tamaño**: «Del objeto» lo estira al tamaño del objeto y cambia con él (así se traen la vegetación y las personas); «Del dibujo» lo deja a su medida real (así se traen los muebles). Se puede cambiar después.
- **Cambiar…** pone otro; **Quitar** vuelve al dibujo de la familia.
- Las unidades del DXF se leen de su cabecera (`$INSUNITS`); si no lo dice, milímetros.
- El símbolo es solo de la planta: el 3D y el IFC siguen siendo los de la familia.

La demo dibuja uno de sus árboles con un símbolo de CAD (copa, círculo interior y ocho ramas). Para el programador: `archisim_core::PlanSymbol` y `SymbolItem` (`ObjectType::symbol`, formato versión 30), `archisim_export::symbol::symbol_from_dxf`, `archisim_plan::draw_symbol` y `drafting::draw_shape`; en la app, `object_ui::symbol_group`.

## Biblioteca acogedora (v0.49, revisión de Antonio del 23/10/2026)

Lo que hace que una casa parezca vivida, como en Paralives: piezas de formas blandas (cojines que se hinchan, cantos redondeados, patas torneadas), con pocas piezas cada una para que una habitación llena siga siendo ligera, y una paleta cálida de materiales. Todo está en la Maqueta, en Amueblar → **Salón** y en la nueva pestaña **Decoración**; y en la galería de objetos de Técnica.

| Familia | Modelos 3D | Tipos de la biblioteca |
|---|---|---|
| Butaca | de orejas, redonda, de ratán, baja de patas | una de cada |
| Puf | redondo, de punto, cubo | uno de cada |
| Mesa auxiliar | pie central, tres patas, nido | una de cada |
| Aparador | nórdico de patas (puertas y cajones), puertas de lamas | 160 y 180 |
| Alfombra | lisa con ribete, redonda, de rayas, bereber con flecos | cuatro |
| Lámpara | de pie, de mesa, colgante (con su cable hasta el techo), de arco | cuatro |
| Planta de interior | costilla de Adán, ficus, cactus, helecho, sansevieria | cinco |
| Cortina | larga, visillo (transparente), hasta el alféizar; abierta o corrida | lino y visillo |
| Decoración | jarrón con ramas, frutero, libros y vela, cerámicas, cesta con manta | cinco |

- El **Sofá** lleva ahora **cojines sueltos** (de 0 a 4, apoyados en el respaldo) y **manta sobre el brazo**, cada uno con su material: «Sofá acogedor de 3 plazas» (lino arena, cojines mostaza y manta teja) y «Sofá nórdico con cojines».
- **Paleta acogedora** (`archisim_library::COZY_PALETTE`): linos, bouclé, lanas, terciopelos (salvia, mostaza, rosa), pana, robles, nogal, ratán, latón, mármol, terracota y cerámicas.
- **Color por pieza**: en la tarjeta de la Maqueta, un objeto seleccionado muestra una fila de colores para cada pieza (tapicería, cojines, patas…), con los de la paleta que le van (telas para lo tapizado, maderas y metales para las patas). Cambia solo ese objeto: si su tipo lo comparten otros, se le hace una copia («Sofá de 3 plazas · Terciopelo mostaza»). Un solo paso de Deshacer.
- La decoración se pone en el suelo; para subirla a una mesa o una balda, el desfase de la base. Que se apoye sola en las superficies queda pendiente.
- `archisim modelos out/` coloca en sus dos primeras filas todas las piezas nuevas; la demo tiene en el salón el sofá acogedor, una alfombra, una butaca de orejas, una lámpara de pie y una costilla de Adán.

Para el programador: `archisim-props::cozy` (`rug`, `lamp`, `potted_plant`, `armchair`, `pouf`, `side_table`, `sideboard`, `curtain`, `decor`) y `Builder::blob` (elipsoide con ejes cualesquiera, para hojas y ramas); las recetas en `families/objetos/` (`butaca`, `puf`, `mesa_auxiliar`, `aparador`, `alfombra`, `lampara`, `planta_interior`, `cortina`, `decoracion`); los papeles nuevos y sus parámetros en `model/props::candidates`; la tarjeta en `app/maquette_ui::object_pieces` y `Editor::paint_object_piece`. Sin cambios de formato: los proyectos guardan las familias que usan.

## Colecciones (v0.50, revisión de Antonio del 23/10/2026)

Un estilo para toda la casa. Una colección no tiene muebles propios: dice qué madera, qué telas, qué metal, qué cerámica, qué piedra y qué fibra lleva cada pieza, y qué modelo 3D toman algunas familias. Así **cada pieza de la biblioteca tiene su versión en cada colección**, con el nombre «Sofá de 3 plazas · Japandi».

| Colección | Madera | Tela | Segunda tela | Cojines y colchas | Metal | Cerámica | Piedra | Fibra |
|---|---|---|---|---|---|---|---|---|
| Nórdico cálido | roble claro | lino arena | bouclé crudo | terciopelo salvia | metal negro | cerámica blanca | mármol | ratán |
| Mediterráneo | madera encalada | lino blanco | lino arena | lino añil | hierro forjado | terracota | microcemento | enea |
| Años 50 | teca | lana oliva | lana cruda | terciopelo mostaza | latón | cerámica turquesa | terrazo | ratán |
| Japandi | roble ahumado | lino piedra | lino gris | lana carbón | metal negro | gres | piedra gris | bambú |

- Modelos por colección, cuando la pieza no elige el suyo: en nórdico, sofá nórdico, cama con cabecero tapizado y mesita de patas; en mediterráneo, cama de madera, butaca de ratán y taburete de enea; en años 50, sofá de patas, mesita de patas cónicas y butaca baja; en japandi, cama de lamas, mesita flotante y aparador de lamas.
- Lo que no es del mueble se queda como está: sábanas, pantallas de lámpara, vidrio, espejos, grifería cromada, porcelana de los sanitarios, plantas y lo que hay encima de las mesas.
- **En la Maqueta**: «Colección» encima del catálogo (Amueblar y Exterior). Al elegir una, las tarjetas (la pieza en 3D desde v0.51) la enseñan en sus colores y llevan la tira de la colección, y las piezas se colocan ya en ella. **Aplicar** pasa a la colección lo seleccionado (con un recuadro, una habitación entera); «Sin colección» + Aplicar lo devuelve a sus colores. Un paso de Deshacer.

**Piezas nuevas en el mismo estilo** (44 tipos, 12 familias nuevas y dos ampliadas):

| Dónde | Familia | Modelos 3D |
|---|---|---|
| Dormitorio | Cama | + cabecero tapizado (capitoné en canales, más ancho que la cama) y cabecero de lamas (tarima baja); **colcha a los pies** y **cojines** (0–4) |
| Dormitorio | Mesita de noche | + patas cónicas (cajón y balda abierta) y flotante (colgada, con uñero) |
| Cocina | Taburete | de madera, tapizado con respaldo, de enea |
| Cocina | Isla de cocina | con voladizo para taburetes, mesa de trabajo con cestas, con fregadero |
| Cocina | Cosas de la encimera | tablas y aceite, botes de cristal, cafetera y tazas, hierbas aromáticas |
| Baño | Toallas | dobladas en pila, colgadas de una barra, enrolladas en una bandeja |
| Baño | Cesta | cesto de ropa con tapa, con rollos, con asas y una toalla |
| Decoración | Cuadro o espejo | cuadro con paspartú, tríptico, espejo de arco, espejo sol, reloj |
| Decoración | Cojín de suelo | cuadrado capitoné, zafu, pila |
| Decoración | Manta | doblada con flecos, pila de mantas, en escalera |
| Jardín y terraza | Tumbona | de teca con ruedas (y colchoneta), de aluminio, cama de exterior |
| Jardín y terraza | Sombrilla | redonda, descentrada, de paja con flecos |
| Jardín y terraza | Macetón | con olivo, ánfora, con gramíneas, con lavanda |
| Jardín y terraza | Pérgola | de madera, de lamas de aluminio, con cañizo y trepadora (`IfcBuildingElementProxy`) |

- Lo de la encimera, las toallas, las cestas y las mantas dobladas se posa solo donde se pincha; los taburetes, los cojines de suelo y lo del jardín van al suelo. Los cuadros, espejos y relojes llevan su altura desde el suelo.
- Exterior tiene una pestaña nueva, **Jardín y terraza**.
- `archisim modelos out/` empieza con una fila por colección (las mismas once piezas en los cuatro estilos) y sigue con las piezas nuevas.

Para el programador: `archisim_library::collections` (`COLLECTIONS`, `restyle`, `add_collection_entry`, `type_in_collection`, `apply_collection`, `collection_library` para las fotos del catálogo); `archisim-props::home` (`stool`, `island`, `counter_decor`, `towels`, `basket`, `wall_decor`, `floor_cushion`, `blanket`), `archisim-props::garden` (`lounger`, `parasol`, `big_pot`, `pergola`) y `Builder::obox` (caja girada: lamas y respaldos inclinados); en la app, `app/collection_ui` y `Editor::apply_collection`/`collection_object_type`. Sin cambios de formato.

## Mobiliario urbano, chimeneas y más mobiliario (v0.52, revisión de Antonio del 24/10/2026)

Antonio: «el mobiliario urbano sigue siendo muy básico y cuando pongo una farola no se ilumina como una lámpara alrededor». Ahora cada pieza de exterior tiene su 3D hecho por ArchiSim, con varios modelos (las familias son las de siempre, con los mismos parámetros: los proyectos que ya las usan se ven mejor sin tocar nada).

| Familia | Modelos 3D |
|---|---|
| Banco | de listones sobre patas de fundición, de hormigón con asiento de madera, de listones sobre marco de acero (con o sin respaldo) |
| Farola | báculo curvo, **fernandina** (columna acanalada y farol de cuatro caras), columna con globo, baliza |
| Papelera | de chapa perforada sobre poste, de listones de madera, doble para reciclar |
| Bolardo | de fundición con bola, tubo de acero con banda reflectante, cono de hormigón |
| Buzón | de pie, sobre poste, buzones de portal |
| Contenedor de residuos | de carga lateral (ruedas, tapa, pedal), iglú de vidrio, soterrado |
| Fuente | de fundición con pila y rejilla, bloque de piedra con caño, estanque redondo con surtidor |
| Jardinera | de hormigón con arbustos, de madera con flores, de acero corten con gramíneas |
| Alcorque | de fundición en anillos, de pletinas, bordillo de piedra con grava |
| Aparcabicis | U invertida, onda, soporte de rueda |
| Señal | circular, triangular, rectangular |
| Bicicleta | de paseo con cesta, de carretera, de carga |

- **Las farolas alumbran de noche** (Maqueta → Luz: Noche): su farol brilla y alumbra hasta 11 m alrededor. La demo tiene dos fernandinas junto al camino.
- **Chimeneas** (familia nueva, Amueblar → Salón): de obra con repisa, hogar y solera; estufa de leña con su tubo hasta el techo; moderna lineal en un paramento. El fuego brilla y alumbra de noche.
- **Más mobiliario**: perchero de pie y recibidor con banco, ganchos y cestas; sillas de oficina (giratoria y de malla); mesa de bistró con dos sillas, mesa de teca para seis y sofá de exterior; silla colgante de ratán y hamaca; radiadores (de elementos y de panel), aire acondicionado de pared, ventiladores de techo; piano de pared y de cola; cama cabaña con guirnalda de luces y litera; y la estantería «Con objetos» (libros, jarrones, plantas, libros apilados, cestas).
- `archisim modelos out/` tiene cuatro filas con todo esto.

Para el programador: `archisim-props::street` y `archisim-props::extras`; lo que brilla y alumbra de noche, `view3d::scene::glow_materials` (lámparas 4,5 m; farolas 11 m).

## Personas y coches en el estilo acogedor (C1, v0.58)

Personas y coches se rehicieron con las formas suaves de la biblioteca acogedora:

- **Personas**: figuras de maqueta redondeadas (hombre, mujer con vestido y persona caminando), con la ropa (`mat`), el pantalón y los zapatos (`mat2`), la piel (`skin`) y el pelo (`hair`) cada uno en su material. Si el tipo trae un solo material, la figura sale de un color, como antes. En la biblioteca, «Persona», «Mujer» y «Niño» traen colores de la paleta (pana azul petróleo, lana teja, terciopelo mostaza, vaquero; piel; pelo castaño o negro).
- **Coches**: carrocería y parachoques muy redondeados, faros y espejos redondos, ruedas gruesas con tapacubos abombado. «Turismo», «Todoterreno» y «Furgoneta» traen chapa salvia, crema y teja.

En los **alzados y secciones** las personas y los coches (categorías personas y vehículos) se dibujan por su **silueta**, no cara a cara:
- en técnico, en blanco con su contorno y las piezas (lunas, ruedas, cara) en línea fina;
- en presentación, cada material en su color.

La silueta sale de los triángulos de su 3D vistos desde la sección, pintados en una rejilla de 1 cm y contorneados (`archisim_geom::cover`). En 3D y en las axonometrías se ven con su volumen.

## Dibujos propios en alzado y en planta (v0.59, pedido por Antonio el 24/10/2026)

En Propiedades de un objeto, **Dibujo en alzado** permite cargarle al tipo sus propios dibujos:

- **DXF de frente** o **DXF de lado**, para las vistas técnicas;
- **PNG de frente** o **PNG de lado**, una imagen con fondo transparente, para las de presentación.

Se dibujan en alzados, secciones y axonometrías en lugar de lo que hace ArchiSim:
- de pie en su sitio, tan altos como el objeto (un árbol, como ha crecido), manteniendo su proporción;
- mirando a quien mira, y del revés si se ve por detrás o por el otro lado;
- en su orden de profundidad: las partes cerradas del DXF van en blanco y tapan lo de detrás.

Si el tipo solo tiene uno de los dos, ese sirve para todo.

**Imagen en planta de presentación**: un PNG visto desde arriba, que en las plantas de presentación sustituye al símbolo. Se dibuja del tamaño del objeto en planta, girado con él y con su parte de arriba hacia el frente del objeto.

Los ficheros se copian dentro del proyecto (en base64), así que se abre igual en otro ordenador. `docs/ejemplos/arbol-alzado.dxf`, `persona-alzado.png` y `arbol-planta.png` son ejemplos dibujados para ArchiSim (`scripts/make_elevation_examples.py`), y `archisim figuras out/` los usa.

## Lo que se cambia en la Maqueta es del ejemplar (v0.59)

Al cambiar en la Maqueta las medidas, los colores o las opciones (cajones, frente, tirador) de una pieza, solo cambia esa pieza:
- recibe un tipo propio, «<tipo> (ejemplar)», que no sale en el catálogo ni en las galerías;
- el tipo del que salió, el de la biblioteca, no cambia, y lo que se coloque después sale como siempre;
- «Aplicar a todos», en la barra del objeto, lleva lo cambiado al tipo y a todas sus piezas;
- al borrar la pieza se borra su tipo propio.

Cambiar el tipo desde Propiedades, en Técnica, sigue cambiando todas las piezas del tipo, como en Revit.

## Pendiente

- Estanterías con objetos (hecho en v0.52); muebles en L; más colecciones (rústica, industrial) si Antonio las quiere.

- Llevar los accesorios en los ficheros `.astipos` de la biblioteca (al abrir un IFC de ArchiSim ya vuelven a sus tipos; los de otros programas se leen como revestimientos que no se importan).
- Accesorios en muros curvos e inclinados, y en los bordes de las cubiertas inclinadas (canalones, remates).
- ~~Elegir un bloque de un DXF con muchos~~: hecho en v0.63 (A1), con miniaturas y la carpeta «Símbolos de planta» de la biblioteca.

- Más objetos: electrodomésticos de encastre, campana, aparatos de iluminación, mobiliario de oficina, plantas de interior en maceta.
- Un cuarto nivel de detalle para las vistas lejanas de modelos muy grandes (hoy ya son ligeros).
- Biblioteca ampliada descargable con modelos CC0 (Poly Haven, Kenney) importados por L1.

## Sofá rinconera y encimera continua (C2, v0.62)

- **Sofá rinconera** (`props:corner_sofa`, familia «Sofá rinconera»): largo a lo largo del respaldo y largo del lado, hacia la derecha o la izquierda (`side`); plazas en cada tramo (`places`, `places2`) y la de la esquina; un brazo en cada extremo libre; los tres modelos del sofá (contemporáneo sobre zócalo, clásico, nórdico de patas). En la biblioteca, 260×170 y 300×200.
- **Encimera continua**: los módulos bajos que se tocan forman un tramo con una sola encimera (`Model::worktop_runs`, en `archisim-core::model::worktops`), hecha por `archisim_geom::solid::worktop_solids` y llevada por el primer módulo del tramo; el cuerpo del módulo ya no tiene tapa (`Builder::open_box`), porque siempre lleva una encimera encima. El hueco del fregadero lo da `archisim_props::sink_hole` al módulo y al tramo.

## Taller de modelos de alta calidad (M1, 25/10/2026)

Antonio pidió modelos mejores que los que hay, iguales o mejores que los de Paralives, poco a poco: **primero se enseñan, y cuando haya un paquete aprobado se meten en la biblioteca**. Hasta entonces viven en el **taller**: no están en la biblioteca, ni en el catálogo, ni en la plantilla.

**Qué los hace mejores.** Los muebles de la biblioteca están hechos de cajas redondeadas y cojines hinchados por un solo lado. Los del taller están hechos de **tela rellena** (`fine::soft::Soft`):
- las caras grandes se abomban y los lados también, menos que ellas;
- el asiento se hunde un poco por delante de su centro, donde uno se sienta;
- la tela tira de las esquinas y hace pliegues suaves;
- nada es del todo regular: cada cojín es un poco distinto;
- el **vivo** de las costuras es parte de la forma de la tela, a media vuelta del canto, así que se ve como una costura en relieve y no como una raya dibujada;
- los **cojines sueltos** van cosidos por el canto, gordos en el centro y con las esquinas en punta, apoyados y algo girados, como quedan después de sentarse;
- las **patas torneadas** se abren un poco hacia fuera.

**Cómo se dibuja la tela en 3D.** Lo que es tela rellena (`PropMesh::soft`) se sombrea suave por completo y solo se perfila donde acaba contra lo de detrás: no lleva rayas en sus propios pliegues y costuras, que antes salían como líneas de puntos. Lo demás se dibuja como siempre, y los muebles de la biblioteca no cambian.

**Peso.** El sofá de 3 plazas del taller tiene unos 18.500 triángulos, lo mismo que el sofá acogedor de la biblioteca (19.100). Una prueba lo vigila: ninguna pieza del taller puede pesar más de vez y media la que sustituye.

**Primera entrega**: el sofá («Sofá (taller)», `props:fine_sofa`, modelo «Contemporáneo de patas»), con las medidas de siempre (largo, fondo, altura del asiento y del respaldo, plazas) y de 0 a 4 cojines sueltos. Dos tipos: 3 plazas en lino arena con cojines mostaza y patas de roble; 2 plazas en terciopelo salvia con un cojín de lino blanco y patas de nogal.

**Cómo verlos**:
- `archisim taller out/taller`: `taller.archisim` con cada pieza nueva delante de la de la biblioteca que sustituiría, `taller.ifc`, axonometría y planta.
- `ARCHISIM_TALLER_OUT=out/taller cargo test -p archisim-app workshop_pictures`: fotos de estudio con el mismo dibujo 3D de la Maqueta (tres cuartos, frente, otro lado, detalle, y antes y después).

Para el programador: `archisim-props::fine` (`KINDS`, `make`, `sofa`) y `fine::soft` (`Soft` con `crown`, `side`, `sag`, `creases`, `wobble`, `welt`; `knife_pillow`; `tube`); `Builder::mark`/`transform_from` para inclinar una pieza hecha derecha. Las recetas en `archisim-library/taller/*.asfam` y el proyecto en `archisim_library::workshop` (`PIECES`, `project`). `Model::object_soft_materials` dice qué materiales de un objeto son tela rellena, y `view3d::scene` los dibuja como el follaje (`bias` 2) con las normales suavizadas.

### Segunda entrega del taller (26/10/2026): planta, fregadero y farola

Pedido de Antonio con capturas de Paralives al lado de las nuestras: «las hojas son planos estáticos antinaturales, el fregadero son unos tubos extruidos que ni se tocan, en la farola hay elementos al aire… quiero que sean visualmente preciosos». Tres piezas nuevas en el taller, cada una delante de la de la biblioteca que sustituiría:

- **Planta de interior (taller)** (`props:fine_plant`), dos modelos:
  - **Espatifilo** en maceta esmaltada de panza redonda: catorce hojas lanceoladas con su nervio central, venas laterales marcadas en la lámina, las dos mitades levantadas en V, la punta que cae y un poco de giro; cada hoja en su peciolo, que sale de la tierra, se curva hacia fuera y **sigue como nervio de la hoja**, sin cortes. Las de dentro, más tiernas y derechas; las de fuera, tumbadas. Dos flores blancas (espata y espiga).
  - **Pilea** en maceta de barro con su plato: tres tallos con hojas redondas sujetas por debajo, como las de verdad, en peciolos largos y finos que se arquean.
  - La maceta tiene grueso: borde redondeado, cara de dentro y tierra un poco abombada con terrones.
- **Módulo fregadero (taller)** (`props:fine_sink_unit`): el mismo módulo bajo de la biblioteca (frentes, tiradores, puertas) con **fregadero encastrado de acero**: ceja de 2 cm apoyada en la encimera y doblada hacia dentro, seno de esquinas redondeadas con el fondo que cae a la válvula, y válvula con su cestillo. **Grifo de caño alto de una pieza**: base, cuerpo torneado, cuello que sale del cuerpo y hace la curva sin juntas, ducha con su aireador y maneta que nace del cuerpo. El mueble ya no se rebaja debajo del seno (antes se veía el hueco).
- **Farola de fundición (taller)** (`props:fine_streetlight`): basa ochavada, un solo perfil torneado de la basa al capitel (toro, campana, collarines, jarrón, fuste con su anillo, capitel que se abre bajo el farol), **cruceta** para la escalera, cuatro **volutas** que sujetan la bandeja, y el farol de cuatro caras: montantes que se abren hacia arriba, cristales con su junquillo, bandas arriba y abajo, tejadillo curvo con alero y remate con bola. Todo se toca: no queda nada al aire. Por la noche sus cristales se encienden como los de la farola de la biblioteca.

**Peso.** Una planta de la biblioteca tenía unos 1.600 triángulos; las del taller, 14.000–17.000 (menos que un sofá). El fregadero, 5.500 (antes 3.000); la farola, 5.700 (antes 1.200). La prueba del taller vigila ahora un máximo por pieza (`Piece::most`) en lugar de la proporción con la que sustituye, porque una hoja de verdad pesa más que una bola.

**El árbol, todavía no.** Se probó a cambiar cada mata de la copa por un ramillete de hojas sueltas sobre un corazón más oscuro: de lejos la copa se deshilacha mejor, pero de cerca sigue viéndose facetada y triplica el peso (60.000 triángulos). La calidad de Paralives en los árboles viene de hojas dibujadas en una textura con transparencia sobre planos pequeños; es otro trabajo (el dibujo 3D, el render y el `.glb` tienen que saber recortar por la textura) y queda propuesto como bloque propio.

Para el programador: las formas nuevas están en `archisim-props::fine::shape` (`curve` Catmull-Rom, `sweep` tubo por una curva con radio variable y extremos planos o redondos, `turned` torneado de n lados, `plate`, `bar`, `rounded_rect`/`loft_loop` para cáscaras como el seno, y `leaf` con `Leaf`/`Outline`: contorno, pliegue en V, caída, giro, abombado, ondulación del borde, venas hundidas, punto de sujeción y grueso, cerrada y con el borde compartido por las dos caras). Las piezas en `fine::plant`, `fine::sink` (el módulo llama a `kitchen::base_unit` con `fine_sink` para que no haga encimera, seno ni grifo) y `fine::lamp`. Papeles nuevos: `leaves2`, `spadix`, `sink` (cae en `metal`), con su sitio en `candidates`. Las hojas, tallos y flores son `soft` (sombreado suave, contorno solo contra el fondo). Las recetas en `archisim-library/taller/planta.asfam`, `fregadero.asfam` y `farola.asfam`; materiales nuevos de la paleta: Hoja verde tierna, Tallo verde, Pétalo blanco, Espádice, Acero inoxidable, Fundición y Cristal esmerilado. `workshop::Piece` admite valores de texto (`T`) y sí/no (`B`) y familias `flora:<especie>` para cuando vuelva el árbol. Las fotos de estudio encuadran por el tamaño real del modelo y aceptan `ARCHISIM_TALLER_SOLO=<parte del nombre>`.

## Plan de la biblioteca nueva (26/10/2026)

Antonio aprobó la calidad del taller y quiere **toda** la biblioteca así. El inventario, zona por zona y comparado con Paralives, está en el documento «Biblioteca de objetos de ArchiSim: inventario frente a Paralives» (claude.ai). Decisiones de Antonio del 26/10/2026:

1. **Orden por paquetes**, cada uno se enseña en el taller y entra en la biblioteca al aprobarse: 1 salón y comedor, 2 cocina y baño, 3 dormitorio, 4 iluminación, 5 exterior y urbano, 6 plantas, decoración y estudio, 7 comercio, hostelería y oficinas.
2. **Por colecciones**: cada pieza se modela por colección de estilo (no un modelo vestido con materiales distintos). Ocho, decididas el 26/10/2026: nórdico cálido, mediterráneo, años 50, japandi (las que ya había), contemporáneo, rústica, industrial y clásica (nuevas). No toda pieza existe en todas. El paquete 1 se entrega en tres tandas: 1a sofás, butacas y pufs (24 modelos, uno por pieza y colección); 1b sillas, taburetes y mesas; 1c almacenaje, alfombras, cortinas y chimeneas.
3. **Los modelos antiguos se quitan** de la biblioteca al aprobar cada paquete; quedan en las versiones anteriores.
4. **Comercio, hostelería y oficinas** entran como paquete 7: ArchiSim es para cualquier edificio, no solo viviendas.
5. **Bloques técnicos al final**: copas de árbol con hojas por textura (A) y luz de la Maqueta (B).
6. **Mobiliario de diseño** (26/10/2026): las piezas de autor aún protegidas (LC2/LC3, Togo, Marshmallow, Lounge Chair, Barcelona, Huevo, Wassily…) no se modelan, ni con otro nombre. Se usan (a) importando los modelos 3D oficiales de los fabricantes a una carpeta «Diseño» de la biblioteca de cada usuario, con importación afinada (bloque propio tras el paquete 1); (b) modelando clásicos de dominio público (Calvet de Gaudí, Thonet nº 14 y mecedora, Hill House, Café Museum de Loos, butaca de Otto Wagner, Windsor, bergère; el Chesterfield ya está en la colección clásica) como paquete «Diseño»; (c) con piezas originales que recogen el gesto de una época sin parecerse a ningún modelo concreto.

### Tanda 1a: sofás, butacas y pufs de las ocho colecciones (26/10/2026)

En el taller, 24 modelos, uno por pieza y colección (rama `bloque/paquete-1a`):

- **Sofás** («Sofá (taller)», `props:fine_sofa`, modelos con el nombre de cada colección): nórdico de patas torneadas; mediterráneo de madera encalada con lamas y colchoneta atada; años 50 de brazos finos, respaldo abotonado y patas de compás con puntera de latón; japandi bajo sobre zócalo con tableros; contemporáneo de brazos anchos sobre zócalo oscuro; rústico de madera maciza con postes, travesaños y lamas; industrial de tubo de acero negro y cuero; clásico capitoné con brazos enrollados y patas de bola.
- **Butacas** («Butaca (taller)», `props:fine_armchair`): nórdica redonda envolvente sobre patas abiertas; mediterránea de ratán con aros, husillos y cojines de lino; años 50 de carcasa sobre patas de compás; frailero rústico de madera y cuero tensado con remates torneados; de orejas clásica con respaldo abotonado. La japandi, la contemporánea y la industrial son su sofá a una plaza.
- **Pufs** («Puf (taller)», `props:fine_pouf`): tambor con vivos (nórdico, años 50 sobre patas, industrial sobre aro de acero), de esparto en anillos con tapa en espiral, cojín grueso japandi, cubo contemporáneo, banqueta rústica de patas torneadas y asiento de enea, y reposapiés capitoné clásico.
- **Capitoné** en la tela rellena (`Soft::tufts`, `tuft_depth`, `tuft_faces`): botones en diamante con los pliegues entre ellos, solo en las caras que se ven; necesita una malla fina (paso de 2,6 cm), por eso el sofá clásico pesa unos 34.000 triángulos.
- Materiales nuevos de la paleta: Tela gris, Pino viejo, Lino crudo, Cuero coñac y Terciopelo verde botella.

Pesos: sofás 14.000–21.000 triángulos (clásico 34.000), butacas 7.000–20.000, pufs 2.000–11.000.

### Tanda 1b: sillas, taburetes y mesas de las ocho colecciones (26/10/2026)

En el taller, 40 modelos más (rama `bloque/paquete-1a`):

- **Sillas** («Silla (taller)», `props:fine_chair`): nórdica de roble con respaldo curvado; andaluza de enea; años 50 de carcasa sobre patas de compás; japandi de cordel; contemporánea de tubo; rústica de tablillas; industrial de chapa de acero; Luis XVI con medallón ovalado tapizado.
- **Taburetes** («Taburete (taller)», `props:fine_stool`), de bar o bajos según la altura del asiento, uno por colección.
- **Mesas de comedor** («Mesa de comedor (taller)», `props:fine_dining_table`): nórdica de patas cónicas; mediterránea encalada con faldón; años 50 ovalada; japandi de tablones sobre dos pies con travesaño; contemporánea de tablero fino sobre patas de acero en V; rústica de tablones con balda baja; industrial de tablones sobre marcos de acero; clásica redonda sobre pie torneado de tres garras curvas con su remate.
- **Mesas de centro** («Mesa de centro (taller)», `props:fine_coffee_table`): nórdica redonda de dos tableros; mediterránea con balda de lamas; años 50 ovalada; japandi en caja abierta con dos huecos; contemporánea de mármol sobre aro de acero; rústica en baúl con flejes y cerradura; industrial de tablones con ruedas; clásica de mármol sobre patas torneadas con balda.
- **Mesas auxiliares** («Mesa auxiliar (taller)», `props:fine_side_table`): nórdica de bandeja con borde; mediterránea de cerámica en forma de diábolo; años 50 de dos pisos; japandi de tablero redondo sobre dos tablas encajadas en cruz; contemporánea en C de chapa doblada con tablero de madera; rústica de tronco con su corteza; industrial de cubo de acero con cajón; velador clásico de tres garras.
- Material nuevo de la paleta: Corteza de pino. Una prueba comprueba que todos los materiales que piden las piezas existen en la paleta (antes, si no existía, la pieza se quedaba con el de la receta sin avisar).

Pesos: sillas y taburetes 1.000–8.000 triángulos, mesas 450–6.000.

### Tanda 1c: almacenaje de las ocho colecciones (26/10/2026)

En el taller, 32 modelos más (`archisim_props::fine::storage`): cada pieza se arma con frentes (`Panel`: liso, cuarterón con moldura, rejilla, cristal, cristal con junquillos, listones, acanalado, tablas y malla de acero), tiradores (`Pull`: pomo torneado, pomo de latón, argolla, concha, barra, lengüeta de cuero, uñero) y patas (cónicas abiertas, de compás, peonza Luis XVI, zócalo retranqueado), y se viste: libros en filas con huecos, pilas tumbadas, alguno inclinado y jarrones (`books`), cerámica en las vitrinas (`display`), cestas de enea y el televisor.

- **Aparadores** («Aparador (taller)», `props:fine_sideboard`): nórdico de patas abiertas con uñeros; mediterráneo encalado de rejilla; años 50 acanalado en nogal con latón; japandi de puertas correderas de listones; contemporáneo lacado con encimera de terrazo, sin tiradores; rústico de tablas con bisagras de forja y argollas; industrial con bastidor de acero, malla y conchas; Luis XVI con cuarterones, pilastras acanaladas y mármol.
- **Librerías** («Librería (taller)», `props:fine_bookcase`, con «Con libros»): nórdica abierta; mediterránea con hornacina en arco sobre armario de rejilla; años 50 de baldas de nogal en montantes negros sobre mueble con patas de compás; japandi de huecos escalonados; contemporánea en retícula lacada con algunos huecos cerrados; rústica de tablón con puertas de tablas; industrial de acero con aspas laterales; clásica con cornisa, pilastras y cuarterones.
- **Muebles de TV** («Mueble de TV (taller)», `props:fine_tv_unit`, con «Con televisor»).
- **Vitrinas** («Vitrina (taller)», `props:fine_vitrine`): con vidrio, baldas de vidrio y cerámica dentro; la clásica con arco de junquillo en la cabeza de las puertas.
- Materiales nuevos: Vidrio claro, Lacado blanco y arena, Fresno oscuro, cinco colores de lomos de libro y Pantalla de televisor.

Pesos: 4.500–42.000 triángulos (las librerías llenas y las japandi de listones, las que más).

Y, para cerrar el paquete 1, 24 modelos más (`fine::textiles` y `fine::hearth`):

- **Alfombras** («Alfombra (taller)», `props:fine_rug`): nórdica de lana con bandas; mediterránea redonda de yute en espiral con dos vueltas teñidas; años 50 de círculos repartidos en diagonal; japandi de fibra con ribete; contemporánea en forma de canto rodado con dos alturas de pelo; rústica de trapo a rayas con flecos; industrial kilim de rombos; clásica oriental con cenefas, medallón y esquinas.
- **Cortinas** («Cortina (taller)», `props:fine_curtain`, con «Abierta»): tela con pliegues de verdad (`drape`: ondas, tablas en la cabecera o fruncido, más hondos abajo, recogidos en el alzapaños), barras con remates, anillas, rieles de techo y alzapaños con borla; la japandi en paneles deslizantes; la clásica con bandó de ondas.
- **Chimeneas** («Chimenea (taller)», `props:fine_fireplace`): estufa nórdica redonda con su cristal curvo y tiro; campana mediterránea encalada; frente de ladrillo años 50 con banco y rejilla de latón; bloque japandi de microcemento con fuego sobre cantos y leñero; fuego lineal contemporáneo tras vidrio; rústica de sillares con viga de madera y morillos; estufa industrial de chapa con leñero; chimenea Luis XVI de mármol con pilastras acanaladas, frisos con rosetas y guardafuegos; y una **colgante** (cuenco de fuego abierto bajo campana, colgados del tiro; no es la Gyrofocus, que es un diseño protegido). El fuego brilla en la Maqueta como las lámparas (`glow_materials`), en el color «Llama».
- Materiales nuevos: Cal blanca, Piedra caliza y Ladrillo visto en la paleta.

### Paquete 1 en la biblioteca (27/10/2026)

Antonio lo dio por bueno tras mirarlo en 3D y pidió seguir. El paquete sale del taller y entra en la biblioteca (rama `bloque/biblioteca-paquete-1`):

- **Familias de sistema** (`archisim-core/families/objetos`): Sofá, Butaca, Puf, Silla, Taburete, Mesa, Mesa de centro, Mesa auxiliar, Aparador, Estantería, Mueble TV, Alfombra, Cortina y Chimenea suben de versión y dibujan con los modelos nuevos (`props:fine_*`); su «Modelo 3D» son las ocho colecciones (la chimenea, además, «Colgante»). Conservan sus medidas, su planta, cómo se estiran y cómo se juntan; la mesa sigue llevando sus sillas alrededor (ahora las de su colección; en la clásica, redonda, alrededor del pie), el sofá sus cojines sueltos y su manta (doblada en el asiento), y la estantería sus baldas. **Vitrina** es familia nueva.
- **Colecciones**: a las cuatro de siempre se añaden Contemporáneo, Rústica, Industrial y Clásica (`collections::COLLECTIONS`). Una pieza en una colección toma el modelo con el nombre de la colección, si su familia lo tiene, salvo que la pieza tenga el suyo (la butaca de orejas es la clásica en cualquier colección, con los colores de cada una).
- **Piezas de la biblioteca**: conservan sus nombres; las que tenían un modelo propio pasan al más parecido (butaca de ratán → Mediterráneo, mesa de caballetes → Rústica, alfombra bereber → Industrial…). Nuevas: «Vitrina 90» y «Chimenea colgante».
- **Los modelos antiguos** ya no se ofrecen. Su código sigue para que los proyectos de antes se abran igual (cada proyecto guarda sus familias); al actualizar sus familias desde la biblioteca, un modelo que ya no existe pasa al de por defecto (`reapply_family`) y lo demás se conserva.
- El taller se queda con la planta, el fregadero y la farola, que son de otros paquetes.

### Paquete 3, segunda vuelta: el dormitorio completo (27/10/2026)

En el taller (`archisim_props::fine::nursery`), lo que faltaba del dormitorio, uno por colección:

- **Cama infantil** («Cama infantil» gana ocho modelos después de «Cabaña» y «Litera»): la cama de cada colección (`bedroom::bed_frame`) a 90×190, con el colchón a 44 cm y el cabecero a 85; una barrera en la mitad de la cabecera, separada de la cama sobre dos soportes, para que el edredón caiga por dentro; su ropa de cama y un muñeco (`toy`).
- **Cuna** (`props:crib`, 60×120, el lado largo en la pared): barandillas en los cuatro lados, la de delante más baja; colchón, edredón pequeño y muñeco. Las barandillas son las de cada colección (`fence`): barrotes redondos, rejilla, lamas verticales de teca, lamas tumbadas, panel lacado, husillos torneados (rústica y clásica) o tubo de acero; la de los años 50 sobre patas de compás y la clásica sobre patas Luis XVI.
- **Litera** (`props:bunk_bed`): dos camas sobre cuatro postes de la colección, cabecero y piecero de barandilla, barrera alrededor de la de arriba con el hueco de la escalera, y la escalera en el lado derecho, cerca de los pies. Las dos camas hechas, con el edredón remetido (`Making::sides`).
- **Galán de noche** (`props:valet`): de pie sobre tres patas abiertas (nórdico, años 50 con regatones de latón), silla galán de enea (mediterráneo) o capitoné sobre patas Luis XVI (clásica), bastidor de listones (japandi), varilla de acero sobre un disco (contemporáneo), poste torneado sobre cruceta con colgadores de forja (rústica) y tubo sobre bridas con balda (industrial). Lleva un pantalón doblado en la barra (`clothes`, la misma tela que cae del edredón).
- **Espejo de pie** (`props:floor_mirror`): apoyado en la pared con su inclinación (roble con correa de cuero, arco encalado, listón de roble, marco negro fino con las esquinas de arriba redondeadas, pino ancho, acero con remaches); el de los años 50, ovalado en un caballete de teca, y el clásico, basculante entre dos postes torneados con su copete.
- Partes de material nuevas: `toy` (parámetro `toy_mat`) y `clothes` (`clothes_mat`).
- Los dormitorios del taller suman el galán y el espejo, y el cuarto infantil sigue en la misma pared; fotos con `ARCHISIM_DORMITORIOS_OUT` (vistas «galan», «infantil», «cuna» y «litera»).

### Paquete 3: el dormitorio de las ocho colecciones (27/10/2026)

En el taller (rama `bloque/paquete-3`), las piezas del dormitorio hechas como las hace cada colección (`archisim_props::fine::bedroom`, con la ropa de cama en `fine::bedding`). Como en la cocina y el baño, «Cama», «Mesita de noche», «Armario» y «Cómoda» ganan ocho modelos más, uno por colección, después de los suyos; los proyectos de antes no cambian. Dos piezas nuevas, con un modelo por colección: **banco a los pies de la cama** (`props:bed_bench`) y **tocador** (`props:dressing_table`: la mesa con su cajón, el espejo en la pared con «Espejo encima» y «Altura del espejo», y el taburete con «Taburete»).

| Colección | Cama | Mesita y lámpara | Armario | Cómoda | Banco | Tocador |
|---|---|---|---|---|---|---|
| Nórdico cálido | cabecero tapizado con vivo, base tapizada sobre patas de roble abiertas | roble, cajón sobre hueco; pie de cerámica y pantalla de tambor | lacado con lengüetas de cuero, zócalo de roble | patas abiertas, uñeros | tapizado sobre patas de roble | patas abiertas, espejo redondo de roble, taburete redondo tapizado |
| Mediterráneo | encalada, cabecero de rejilla en arco sobre dos postes | puerta de rejilla; jarra de terracota y pantalla imperio | puertas de rejilla, pomos, cornisa | encalada con pomos | de enea con travesaños | espejo en arco, taburete de enea |
| Años 50 | teca, cabecero ancho con ranuras y alas, patas de compás | patas de compás, cajón con barra de latón; pie turquesa y pantalla cónica | sobre patas de compás, barras de latón | patas de compás, barras | de listones sobre patas de compás | dos cajones, espejo redondo de latón |
| Japandi | tarima baja más ancha que el colchón, cabecero bajo de listones | caja baja sobre zócalo oscuro con libros; farol de papel | correderas de listones | zócalo oscuro, uñeros | losa de roble con cojín | colgado de la pared, espejo cápsula con balda |
| Contemporáneo | cabecero alto de canales tapizados, base tapizada hasta una sombra | dos cajones sin tirador sobre trineo de acero; lámpara seta | sin tiradores, ranura donde se juntan las puertas | gola | bloque tapizado | consola sobre acero, espejo sin marco, puf cubo |
| Rústica | postes de pino con remates torneados, cabecero y piecero de tablas (el edredón remetido; edredón acolchado) | patas torneadas, anilla de forja; candelero de hierro | tablas con bisagras de forja, anillas, cornisa | patas de bollo, conchas | baúl con flejes y asas | patas torneadas, espejo de pino, taburete de tres patas |
| Industrial | tubo de acero negro con barrotes y bastidor de tubo cuadrado | bastidor de tubo, cajón con concha; bombilla en jaula | madera ahumada en bastidor de acero con aspas | bastidor de acero, conchas | tubo cuadrado con cojín de cuero | bastidor de acero, espejo colgado de una barra |
| Clásica | cabecero capitoné en marco moldurado con copete y roseta, postes con remate, patas Luis XVI; almohadones y rulo | patas Luis XVI, cajones moldurados con pomos de latón, mármol; columna de latón | cuarterones, pomos de latón, cornisa, pilastras | cajones moldurados, mármol | capitoné sobre patas Luis XVI | mármol, espejo con copete, banqueta capitoné |

- **La ropa de cama** (`Cover`): una tela que se apoya en un rectángulo y cae por sus bordes, con la vuelta redondeada, el vuelo hacia el bajo, pliegues que siguen alrededor de las esquinas y las esquinas cayendo como las de un mantel. Así se hacen el edredón (abombado; acolchado en la rústica), la sábana vuelta sobre su cabecera y la manta a los pies. Si hay piecero, el edredón se remete.
- Partes de material: `frame` (madera o acero de la estructura y el cuerpo), `front` (puertas y cajones), `top`, `stone`, `plinth`, `metal`, `glass`, `weave`, `dark`, `cushion` (tapizado), `linen` (sábanas y almohadas), `duvet`, `spread` (manta: parámetro `spread_mat`), `pillow` (cojines), `shade` (pantalla, brilla de noche), `ceramic` (pie de la lámpara), `bulb`, `books`, `books2` y `mirror_glass`.
- Ocho dormitorios montados en el taller (`workshop::BEDROOMS`, `place_bedroom`): armario, cómoda, mesita, cama, mesita y tocador contra la pared, y el banco a los pies de la cama; fotos con `ARCHISIM_DORMITORIOS_OUT=carpeta cargo test --release -p archisim-app -- bedroom_pictures`.
- Pesos: camas 13.000–32.000 triángulos (la ropa de cama, unos 12.000), lo demás 2.000–25.000.
- De paso: un frente nuevo, `Panel::Moulded` (tablero con moldura), más ligero que el de cuarterones, para los cajones clásicos.

### Tanda 2b: el baño de las ocho colecciones (27/10/2026)

En el taller (rama `bloque/paquete-2b`), los sanitarios y accesorios de la biblioteca hechos como los hace cada colección (`archisim_props::fine::bath`). Como en la cocina, no son familias nuevas: «Lavabo», «Inodoro», «Bidé», «Ducha», «Bañera», «Espejo» y «Toallero» ganan ocho modelos más, uno por colección, después de los suyos; los proyectos de antes no cambian. Los grifos son los de la cocina a otra escala (`kitchen::tap_scaled`).

| Colección | Lavabo | Inodoro y bidé | Ducha | Bañera | Espejo | Toallero |
|---|---|---|---|---|---|---|
| Nórdico cálido | mueble suspendido de roble con lavabo integrado, grifo de caño alto | suspendidos | mampara fija de vidrio, rociador redondo | exenta oval con grifo de pie | redondo de roble con correa | escalera de roble |
| Mediterráneo | de obra (microcemento) con hornacina en arco y toallas, cuenco cerámico, grifo de cisne | de pie con cisterna | cortina en barra, ducha vista de latón | de obra | de arco | aro de forja |
| Años 50 | de pedestal | de pie con cisterna | mamparas correderas de vidrio acanalado en marco cromado, rociador de campana | de obra con frente de azulejo | redondo cromado con aplique | barra cromada |
| Japandi | dos baldas de roble, cuenco de gres, grifo de pared | suspendidos | mampara de perfil negro, sumidero lineal | ofuro de madera con taburete | cápsula | escalera de roble |
| Contemporáneo | suspendido sin tirador, grifo de pared cuadrado | suspendidos de líneas rectas | vidrio sin perfil, rociador de techo cuadrado | exenta oval con grifo de pie cuadrado | con luz | barra |
| Rústica | tablón en escuadras de hierro, cuenco de piedra, cesta | con cisterna alta y cadena | cortina | de patas | marco de pino | aro de forja |
| Industrial | bastidor de acero con balda y pila de hormigón, grifo de tubo | adosado | mampara de cuarterones negros (tipo ventana de acero) | de patas, grifo de muelle | marco de acero colgado de una barra | escalera de tubo negro |
| Clásica | tocador de mármol sobre patas torneadas de latón, lavabo bajo encimera, barra con toalla | con cisterna alta | mampara con perfil de latón | de patas | dorado con copete | radiador toallero de latón |

Partes de material: `mat` (porcelana), `unit` (mueble u obra), `top` (encimera o piedra), `metal`, `seat`, `glass`, `frame`, `wood`, `towel` (toallas y cortina), `dark` y `light`. Ocho baños montados en el taller (`workshop::BATHROOMS`, `place_bathroom`), cada uno con su pared; fotos con `ARCHISIM_BANOS_OUT=carpeta cargo test --release -p archisim-app -- bathroom_pictures`.

### Tanda 2a: la cocina de las ocho colecciones (27/10/2026)

En el taller (rama `bloque/paquete-2a`), la cocina por módulos de la biblioteca hecha como la hace cada colección (`archisim_props::fine::kitchen`). No son familias nuevas: «Módulo bajo de cocina», «Columna de cocina», «Mueble alto de cocina» y «Campana extractora» ganan ocho modelos más, uno por colección, después de los suyos («Módulo», «Columna», «Puertas»; «Decorativa» e «Integrada» en la campana), así que los proyectos de antes no cambian y todo lo de la cocina por módulos sigue igual: se pegan, la encimera es continua (`worktop_runs`) y el plano es el de siempre.

Cada colección tiene sus frentes (`storage::Panel` del paquete 1), tiradores (`storage::Pull`), lo que pisa (zócalo, rodapié con moldura, patas cónicas con regatón de latón o bastidor de tubo), fregadero (de acero encastrado, bajo encimera o cerámico de delantal), grifo (de caño alto, cuello de cisne, de puente con crucetas, dos llaves de los 50, cuadrado, de tubo fino, de muelle), placa (inducción o gas con parrillas de fundición), horno, campana (de chimenea, de obra, de obra con viga, de obra con cornisa, curva, integrada, plana o de tubo) y lo de la pared (armarios, vitrinas con cuarterones y cornisa, baldas en ménsulas de yeso, flotantes, en escuadras de hierro con barra de ganchos o en tubos), con tarros y jarras en las baldas:

| Colección | Frentes | Tiradores | Pisa | Fregadero y grifo | Placa | Campana | Pared |
|---|---|---|---|---|---|---|---|
| Nórdico cálido | lisos lacados | lengüeta de cuero | zócalo de roble | acero encastrado, caño alto | inducción | chimenea | armarios |
| Mediterráneo | de lamas añil | anilla de latón | rodapié | cerámico, cuello de cisne | gas | de obra encalada | baldas en ménsulas |
| Años 50 | cantos redondeados celeste | barra cromada larga | patas cónicas | acero, dos llaves | gas, crema | curva | armarios |
| Japandi | roble | uñero en el canto | zócalo oscuro | granito negro bajo encimera, tubo fino | inducción | integrada en roble | baldas flotantes |
| Contemporáneo | lisos sin tirador | gola | zócalo negro | acero bajo encimera, cuadrado | inducción | plana | armarios |
| Rústica | cuarterones salvia | concha de hierro | rodapié | cerámico, puente | gas | de obra con viga | escuadras de hierro |
| Industrial | madera ahumada con marco de acero | barra negra | bastidor de tubo | acero bajo encimera, muelle | gas | de tubo con remaches | baldas en tubos |
| Clásica | plafón con moldura marfil | pomo de latón | rodapié | cerámico, puente | gas | de obra con cornisa | vitrinas con cornisa |

Partes de material nuevas: `plinth`, `sink`, `black` (placa, horno y microondas), `ring`, `wood`, `ceramic` y `hood`. Materiales nuevos en la paleta: Vitrocerámica, Granito negro, Hormigón pulido, Lacado azul celeste, Lacado azul añil y Lacado marfil. Ocho cocinas montadas en el taller (`workshop::KITCHENS`, `place_kitchen`): columna con frigorífico, cajonera, fregadero, lavavajillas, placa y horno, muebles altos o baldas y campana; fotos con `ARCHISIM_COCINAS_OUT=carpeta cargo test --release -p archisim-app -- kitchen_pictures`. El fregadero del taller anterior («Fregadero de acero 80») queda dentro de la cocina nórdica.

De paso, arreglado del paquete 1: los pomos de madera y de latón, las anillas y los remates de las barras de cortina estaban hechos en espejo (se veían por dentro, más oscuros).

### Diseño: los modelos oficiales de los fabricantes (27/10/2026)

Lo acordado el 26/10/2026 (punto 6 del plan): las piezas de autor protegidas no se modelan; se usan los modelos 3D que dan sus fabricantes, que cada usuario descarga y guarda en su biblioteca. Rama `bloque/diseno`. Sin cambios de formato.

- **Formatos** (`archisim-mesh`): además de OBJ, glTF y COLLADA, ahora **FBX** (binario de 6.1 a 7.7 y de texto; jerarquía con traslación, giros en cualquier orden, escala, pivotes, pre y posgiros y el desfase de geometría de 3ds Max; materiales con su color, opacidad y textura dentro del fichero o al lado; ejes y unidades de `GlobalSettings`, con el eje y deducido de arriba y derecha porque los programas no se ponen de acuerdo en el «frente»), **3DS** (objetos ya colocados, materiales con textura y su mosaico; sin unidades) y **STL**. Y el **.zip** tal como se descarga, sin descomprimir (`Zip`, `InZip`, `load`): escoge el mejor modelo de dentro (FBX, luego glTF, OBJ, COLLADA, 3DS, STL), lee sus texturas aunque estén en otra carpeta del zip o con la ruta del ordenador del autor, abre un zip dentro de otro y dice qué formatos trae que no se leen (SketchUp, Revit, ArchiCAD, Rhino…). Ficheros de prueba de verdad: `scripts/make_design_fixtures.py` los hace con Blender (`bpy`) y Assimp, y las pruebas comparan con las medidas que da Blender.
- **Unidades**: los formatos que no las dicen (OBJ, 3DS, STL) se leen en las que dan un tamaño razonable (`plausible_unit`: metros hasta 12, centímetros hasta 1200, milímetros más allá); el diálogo lo dice y se puede cambiar.
- **Aligerar sin perder la forma** (`archisim_mesh::decimate`): colapso de aristas por error cuadrático (Garland y Heckbert); mantiene los bordes abiertos y las líneas entre materiales, no da la vuelta a ningún triángulo y no se come piezas sueltas (hojas: entonces se agrupa por rejilla, como antes). Un millón de caras baja a 50.000 en unos 4–5 s, aparte, sin parar la ventana. Las versiones de lejos de **todos** los modelos importados se hacen ya así (antes, por rejilla, que deformaba los muebles: comparativa en la entrada de la bitácora).
- **Importación afinada** (`archisim_library::imported`): `ImportOptions` con giros de cuarto de vuelta (el frente), detalle (`DETAILS`: 150.000, 50.000 —por defecto— o 15.000 caras, o todo) y quién la hizo (`Maker`); `prepare` hace lo lento. Los materiales toman su categoría y su textura en 3D por su nombre en cinco idiomas (`finish_of`: cuero, tela, bouclé, terciopelo, lana, ratán, madera y sus especies, lacado, metal, mármol, piedra, vidrio…).
- **Quién la hizo**: fabricante, modelo, referencia, diseño de, año y enlace son parámetros de texto del tipo (sección «Fabricante»), que van al IFC en `Pset_ManufacturerTypeInformation` (Manufacturer, ModelLabel, ModelReference) y `ArchiSim_Design` (Designer, Year, URL); la familia queda en la carpeta «Diseño».
- **En la aplicación** (`app/mesh_ui`): el diálogo de importar enseña la planta y el alzado frontal con el frente marcado y los botones de girar, el modelo elegido del zip, las unidades supuestas, el detalle y, con «Pieza de diseño de un fabricante», sus datos y «Guardarla en mi biblioteca» (`Modelos 3D/Diseño/<Fabricante>/<nombre>.glb` y, al lado, `<nombre>.json` con sus datos; `design_paths`). La galería lee esa carpeta (`folder_model_info`: el `.json`, o el fabricante por la carpeta si se deja ahí el zip tal cual) y la Maqueta tiene la pestaña **Diseño** en Amueblar. Un modelo, o el zip del fabricante, se puede soltar en la ventana o abrir con la aplicación.
- **Línea de órdenes**: `archisim diseno <modelo o .zip> [carpeta] [Fabricante]` hace lo mismo que la aplicación y dice qué ha leído (para probar una descarga real sin abrir la aplicación).
- Los modelos de los fabricantes no se suben al repositorio ni van dentro de ArchiSim.

