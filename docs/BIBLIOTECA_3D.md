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

## Pendiente

- Decoración que se apoya sola en mesas, baldas y encimeras.
- Más colecciones con su paleta (mediterránea, años 50, japandi) y más piezas: camas con cabecero tapizado y colchas, estanterías con objetos, cocina y baño en el mismo estilo.

- Llevar los accesorios en los ficheros `.astipos` de la biblioteca (al abrir un IFC de ArchiSim ya vuelven a sus tipos; los de otros programas se leen como revestimientos que no se importan).
- Accesorios en muros curvos e inclinados, y en los bordes de las cubiertas inclinadas (canalones, remates).
- Elegir un bloque de un DXF con muchos (una biblioteca de árboles en un solo fichero); hoy se trae el dibujo entero.

- Más objetos: electrodomésticos de encastre, campana, aparatos de iluminación, mobiliario de oficina, plantas de interior en maceta.
- Un cuarto nivel de detalle para las vistas lejanas de modelos muy grandes (hoy ya son ligeros).
- Biblioteca ampliada descargable con modelos CC0 (Poly Haven, Kenney) importados por L1.
