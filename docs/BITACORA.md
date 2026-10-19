# Bitácora de ArchiSim

Registro de sesiones de trabajo, de la más reciente a la más antigua. Cada conversación nueva lo lee para saber dónde estamos, y añade su entrada al terminar.

Formato de cada entrada: fecha · bloque · qué se hizo · qué quedó pendiente · qué debe probar Antonio.

---

## 2026-10-19 (noche) · Fase 2 sin supervisión: pilares y vigas (2.5), locales (2.9), secciones y alzados (2.10), forjados inclinados (2.7), cubiertas inclinadas (2.4), escaleras (2.6), terreno y norte (2.12), fases de reforma (2.11) y las cuatro mejoras de Antonio (v0.25, rama `bloque/fase2-noche`)

Antonio pidió avanzar «todo lo posible» en la fase 2 a partir del 2.2 mientras dormía, más cuatro mejoras pequeñas. Todo va en la rama `bloque/fase2-noche` (pull request para revisar), con commit y push tras cada avance. Esta entrada se completa conforme avanza la noche.

**Hecho**
- **Pilares y vigas (2.5)**, de punta a punta:
  - Modelo: tipos de pilar y de viga (nombre, material y sección rectangular, circular o perfil I/H), pilares (centro, giro, desfase de base y restricción superior como los muros: siguen las cotas de las plantas) y vigas (eje por dos puntos y cara superior sobre su planta; la sección cuelga). Se rechaza lo que no cabe (un pilar unido a una planta que quedaría sin altura, un alma más gruesa que el perfil…).
  - Plantilla: pilares HA 25×25, 30×30, 40×40, Ø30, HEB 160 y madera GL24h 20×20; vigas HA 30×50, plana HA 60×30, IPE 300 y madera GL24h 16×32 (materiales nuevos: «Acero S275» y «Madera laminada GL24h»). Al importar un IFC sin tipos de pilar y viga se añaden.
  - Formato `.archisim` **versión 7** (`member_types`, `columns`, `beams`); los ficheros anteriores se abren igual.
  - Planta: el pilar cortado por el plano de corte se dibuja con el relleno y la trama de su material y línea gruesa, **encima de los muros** (el hormigón atraviesa la fábrica); también los de otras plantas que el plano de corte atraviesa. Las vigas de la planta y las de la planta de encima (las del techo) se dibujan discontinuas; cortadas, rellenas.
  - 3D: pilares y vigas con el color de su material.
  - IFC: `IfcColumnType`/`IfcBeamType` con `IfcMaterialProfileSet` (perfil paramétrico `IfcRectangleProfileDef`, `IfcCircleProfileDef` o `IfcIShapeProfileDef`, compartido por el tipo), `IfcColumn`/`IfcBeam` extruidos de ese perfil, `Pset_ColumnCommon`/`Pset_BeamCommon` (`LoadBearing` = sí) y `Qto_ColumnBaseQuantities`/`Qto_BeamBaseQuantities` (longitud, área de la sección, volumen). Validado con IfcOpenShell (volúmenes incluidos).
  - Importación: pilares y vigas de cualquier programa (sección del perfil del material o de la extrusión; si no, la caja del sólido, con aviso); la ida y vuelta de ArchiSim sale idéntica.
  - Aplicación: herramientas **Pilar (I)** y **Viga (G)** en la caja. Pilar: clic en el centro; Espacio gira 90° el siguiente; por defecto llega hasta la planta superior. Viga: clic inicio, clic final, y sigue encadenando; Esc o clic derecho termina; dibujada en una planta, por defecto pertenece a la de encima (lleva su forjado, como en Revit). Fuerzan al centro y esquinas de los pilares y a extremos y punto medio de las vigas. Seleccionados: mover, copiar (Ctrl), estirar los extremos de una viga, borrar, Espacio (gira el pilar 90° / invierte la viga), cambiar de planta, y en Propiedades sus restricciones, giro, longitud, volumen y el grupo **Tipo** (nombre, material, forma y medidas de la sección, duplicar y borrar tipo), que cambia todos los elementos del tipo. En el navegador, categorías Pilares y Vigas con su número.
  - Demo: porche al sur con dos pilares HA 30×30 y una viga HA 30×50 bajo un forjado de cubierta.
- **Locales (2.9)**, como las zonas de ArchiCAD y las habitaciones de Revit:
  - Herramienta **Local (Z)**: por defecto, clic dentro de un recinto de muros y se crea el local por sus caras interiores (superficie útil); también por polígono o rectángulo. Número automático (el siguiente libre de la planta: 1, 2… o 0.01 → 0.02), nombre y uso (lista de usos habituales: Estar, Cocina, Dormitorio, Baño…; al elegir uso, el nombre lo sigue si no se había cambiado) y altura (por defecto, de la planta a la de encima). No deja crear dos locales en el mismo recinto.
  - En planta, el **sello** del local: número y nombre, y debajo «Su 12,45 m²», centrado (en un local en L, dentro de él). Se selecciona haciendo clic en el sello o en su borde (dentro del recinto sigue seleccionándose el forjado); el sello se arrastra por su asa, y «Rótulo al centro» lo devuelve. Mover, copiar (con número nuevo) y borrar como el resto.
  - Propiedades: número (único en la planta), nombre, uso, planta, altura, **superficie útil**, **superficie construida** (hasta los ejes de los muros que lo rodean), perímetro y volumen. En el navegador, «Locales (planta)» con cada local y su superficie.
  - **Rotulación técnica con letras**: el trazo propio de las cotas (UNE-EN ISO 3098) tiene ahora el alfabeto completo con ñ, tildes, ² y signos, para los nombres de locales (y las futuras etiquetas), igual en pantalla, en el SVG y en el futuro PDF.
  - IFC: `IfcSpace` agregado a su planta (número en `Name`, nombre en `LongName`, uso en `ObjectType`), extruido por su altura, con `Pset_SpaceCommon` y `Qto_SpaceBaseQuantities` (superficie útil y construida, perímetros, volumen); validado con IfcOpenShell. Se importan los `IfcSpace` de cualquier programa; la ida y vuelta de ArchiSim sale idéntica (también el sitio del sello).
  - Demo: cuatro locales (Estar-comedor, Dormitorio, Cocina, Dormitorio 2), numerados 0.01 a 0.04; las cotas de nivel se han apartado de los sellos.
- **Fases de reforma (2.11)**, como las de Revit o el estado de reforma de ArchiCAD, con la convención española de color:
  - Cada muro, forjado, puerta, ventana, pilar y viga es **Nuevo** (por defecto), **Existente** o **A derribar** (Propiedades → «Fases (reforma)», también con varios seleccionados). Las puertas y ventanas siguen a su muro al cambiarle la fase, y luego se les puede dar otra (una puerta nueva en un muro existente).
  - **Dibujar como**: en las propiedades de la vista, «Dibujar como: Existente» para levantar el estado actual; lo que se dibuje desde entonces nace existente (deshacer no lo cambia).
  - **Qué muestra la planta** (propiedades de la vista y, si el proyecto tiene fases, un desplegable «Fase» en la barra inferior): **Todo**; **Estado actual** (lo existente y lo que se derriba, sin lo nuevo); **Demoliciones** (lo que se conserva normal y lo que se derriba en **amarillo**); **Estado reformado** (lo que se conserva normal y lo nuevo en **rojo**, sin lo derribado). El 3D y la planta exportada siguen la misma elección.
  - IFC: la propiedad estándar `Status` de `Pset_WallCommon`, `Pset_SlabCommon`, `Pset_DoorCommon`… (`EXISTING`, `DEMOLISH`; sin ella, nuevo), que es como la leen Revit y ArchiCAD; se importa igual. Formato `.archisim`: lista `phases` (versión 7).
- **Secciones y alzados (2.10)**, dibujados de los mismos sólidos que el 3D y el IFC (nada se dibuja aparte):
  - Herramienta **Sección (S)** en la caja (grupo Vistas): dos clics, inicio y final; la sección **mira hacia la izquierda del trazo** (una flecha azul lo enseña mientras se dibuja) y recibe la siguiente letra libre (A, B…). Profundidad de vista elegible (2, 4, 8, 15 o 30 m; luego, cualquiera en Propiedades).
  - **Alzados**: botón «Alzados» en la caja o «Crear alzados» en el navegador: los cuatro (Norte, Sur, Este y Oeste) a 3 m del edificio, mirándolo, con la anchura y la profundidad justas para verlo entero (solo crea los que falten).
  - En planta, las **marcas**: la sección con línea de trazo y punto, tramos gruesos en los extremos y en cada extremo un círculo con su letra y la flecha hacia donde mira; el alzado, línea fina discontinua y un círculo con su inicial y su flecha. Se ven en todas las plantas; se seleccionan con un clic, se mueven, copian (Ctrl), estiran por sus extremos y borran como el resto; **doble clic abre su vista**. Propiedades: nombre, profundidad, longitud, «Abrir vista» e «Invertir» (que mire al otro lado).
  - **La vista**, en una pestaña propia (navegador → «Secciones y alzados», o el «+» de las pestañas), a 1:100 por defecto, con zoom, encuadre (Inicio), escala y «Líneas finas»: lo **cortado** con el relleno y la trama de cada material y línea gruesa, y línea fina entre capas (como en planta); lo que se ve **más allá**, hasta la profundidad, en blanco con sus aristas, y **sin líneas ocultas** (lo de delante tapa lo de detrás); el vidrio en azul claro; las **plantas** con línea de trazo y punto, su cota y su nombre. Sigue la fase elegida («Demoliciones», «Estado reformado»). En Propiedades, la altura del cursor respecto al ±0,00.
  - La línea de comandos `archisim demo` / `exportar` escribe también cada sección y alzado en SVG a 1:100 (`demo-seccion-a.svg`, `demo-alzado-norte.svg`…).
  - Formato `.archisim`: lista `sections` (versión 7). No van al IFC (son vistas, no edificio).
  - Demo: Sección A (estar y cocina, mirando al norte), Sección B (por el porche, mirando al oeste) y los cuatro alzados.
- **Forjados inclinados (2.7)**, para rampas y suelos en pendiente, con la flecha de pendiente de Revit:
  - Propiedades del forjado → grupo **Pendiente**: «Inclinado», **Pendiente (%)** (negativa: baja), **Sube hacia** (este, norte, oeste, sur) y, a título informativo, el ángulo y la cota más baja y más alta de la cara superior. «Cara superior» es la altura en el arranque de la flecha.
  - En planta, la **flecha de pendiente** con su porcentaje («10 %»), que sube en el sentido de la flecha; con el forjado seleccionado, sus dos extremos se arrastran para darle cualquier dirección (el porcentaje se mantiene). Mover y copiar el forjado se lleva la flecha.
  - Las capas mantienen su espesor perpendicular a la cara; los cantos quedan verticales, así que la planta sigue siendo el contorno. 3D, secciones y alzados, cota de nivel sobre la rampa (da la altura del punto) y el plano de corte (si corta la rampa, se ve cortada).
  - IFC: `IfcSlab` extruido verticalmente desde un perfil en ejes inclinados (lo que leen bien Revit y ArchiCAD), con superficies y volúmenes reales; validado con IfcOpenShell (volumen incluido). **Importación**: los forjados inclinados de otros programas (cara superior plana inclinada, por extrusión o por caras) entran inclinados con su flecha; antes se descartaban.
  - Demo: **rampa de acceso** a la puerta oeste, de −0,30 al ±0,00 con un 10 %.
- **Cubiertas inclinadas (2.4)**, por faldones, como la cubierta de varios faldones de ArchiCAD o la cubierta por contorno de Revit:
  - Herramienta **Cubierta (T)**: por defecto, clic dentro del edificio y la cubierta sale por las **caras exteriores de sus muros**, con 50 cm de alero, en la **planta de encima** (cubre los muros de la planta en la que se dibuja, como las vigas); también por polígono o rectángulo. Forma **a cuatro aguas**, **a dos aguas** (faldones en los lados largos, piñones en los cortos) o **a un agua**; pendiente en % (30 % por defecto); arranque (altura de la cara superior en el contorno). Tipo nuevo en la plantilla: «Cubierta inclinada de teja sobre losa HA».
  - Cada lado del contorno es **faldón** (con su pendiente) o **piñón**; los faldones se cortan entre sí en cumbreras y limatesas, y el alero y los vuelos de los piñones bajan con la pendiente. Las capas mantienen su espesor perpendicular al faldón; los cantos son verticales.
  - Propiedades: forma, pendiente de todos los faldones, alero, **cada lado** («30» o «piñón»), planta, arranque, tipo, superficie inclinada y en planta, cota de cumbrera y de alero, fase. Seleccionada: mover, copiar, arrastrar las esquinas del contorno, borrar, cambiar de planta. Navegador: «Cubiertas».
  - **Planta de cubiertas**: en la planta de la cubierta, cada faldón en tono claro con sus aristas, el alero algo más grueso, los muros bajo el alero discontinuos y en cada faldón una flecha hacia donde vierte el agua con su pendiente.
  - 3D, secciones y alzados la muestran (corte con sus capas). IFC: `IfcRoof` (tipo `HIP_ROOF`, `GABLE_ROOF` o `SHED_ROOF`) con un `IfcSlab` `ROOF` por faldón, cada uno con las capas del tipo, `Pset_RoofCommon` y `Qto_RoofBaseQuantities` (superficie inclinada y en proyección); validado con IfcOpenShell. ArchiSim la lee de vuelta como cubierta (`ArchiSim_Roof`); las cubiertas de otros programas entran como sus faldones (forjados inclinados).
  - Demo: el **porche** pasa a ser una cubierta de teja **a un agua** al 10 % que vierte hacia el sur, apoyada en la viga (que ahora sigue su cara inferior).
  - Limitación: el contorno no puede tener esquinas entrantes (una casa en L lleva dos cubiertas, que se solapan en la limahoya); los muros aún no se recortan bajo la cubierta.
- **Escaleras (2.6)**, por su **línea de huella**, con la comprobación del **CTE DB-SUA 1**:
  - Herramienta **Escalera (E)**: clic en el pie, clic en cada giro y doble clic, Intro o clic derecho en la llegada. Cada tramo de la línea es un **tramo** de la escalera; en cada giro, una **meseta** del ancho de la escalera (en L, cuadrada; en U, el trozo corto entre tramos forma parte de la meseta; en recta, una meseta intermedia si la línea tiene un punto en medio). Mientras se dibuja se ven los peldaños y una etiqueta «20 tabicas de 17,8 cm · 18 huellas de 28,0 cm ✓ CTE» (o el primer incumplimiento).
  - Llega por defecto a la **planta de encima** (sigue sus cambios de cota) o a una altura propia. **Todas las tabicas iguales**; cada tramo reparte sus huellas para llenarlo, lo más cerca posible de la huella pedida (28 cm por defecto). Losa de hormigón de 15 cm medida desde la garganta de los peldaños.
  - **CTE**: huella ≥ 28 cm, tabica 13–18,5 cm, 54 ≤ 2T+H ≤ 70 cm, ancho ≥ 1,00 m, ≥ 3 peldaños por tramo y ≤ 3,20 m por tramo (uso general); en **uso restringido** (vivienda) huella ≥ 22 cm, tabica ≤ 20 cm, ancho ≥ 0,80 m. Propiedades lo muestra con ✓/✗ y el porqué.
  - Propiedades: ancho, huella, losa, uso, planta, llegada, tabicas, huellas por tramo, mesetas, fase. Seleccionada: mover, copiar, arrastrar los puntos de su línea de huella, borrar. Navegador: «Escaleras».
  - **Planta**: contorno de tramos y mesetas, una línea en cada tabica, lo que queda por encima del plano de corte discontinuo tras la **línea de corte en zigzag**, y la línea de huella con un punto en el pie y flecha en la llegada.
  - 3D, secciones y alzados (con sus peldaños y losa). IFC: `IfcStair` (recta, de cuarto o media vuelta) con un `IfcStairFlight` por tramo (número de tabicas y huellas, tabica y huella) y un `IfcSlab` `LANDING` por meseta, `Pset_StairCommon`, `Pset_StairFlightCommon`, cantidades; validado con IfcOpenShell (volumen de cada tramo incluido). ArchiSim la lee de vuelta (`ArchiSim_Stair`).
  - Demo: **escalera exterior de acceso a la cubierta** por la fachada este: dos tramos (10 y 8 huellas) con meseta, 20 tabicas de 17,75 cm. Con un solo tramo el CTE no se cumplía (salvaba 3,55 m > 3,20 m): por eso la meseta.
  - **Barandillas** de acero a 90 cm sobre la línea de peldaños (CTE), por defecto a los dos lados (Propiedades: a los dos, izquierda, derecha o ninguna): pasamanos que sigue la pendiente de cada tramo, montantes cada metro como mucho, y en las mesetas un pasamanos horizontal en los bordes donde no llega ningún tramo. El CTE avisa si falta pasamanos (al salvar 55 cm o más) o si con 1,20 m de ancho solo hay en un lado. IFC: `IfcRailing` (`HANDRAIL`) dentro de la escalera, con `Pset_RailingCommon`.
  - Las **rampas** ya se dibujan con forjados inclinados (2.7). Pendiente: barandillas sueltas (de terraza o balcón) y de vidrio o con barrotes, peldaños compensados y de caracol, zancas de acero o madera, y ver la escalera desde la planta de llegada.
- **Terreno y norte (2.12)**:
  - Herramienta **Terreno (R)**: se escribe la cota del punto en Propiedades y se hace clic en su sitio, punto a punto (como un levantamiento); Intro o doble clic crea el terreno (tres puntos como mínimo). Los puntos se unen en triángulos (Delaunay) y forman la superficie; el terreno llega 1 m por debajo de su punto más bajo (para que las secciones lo corten con su trama).
  - **Planta topográfica**: curvas de nivel cada equidistancia (por defecto 0,50 m), acotadas («-0,30»; si son muchas, una de cada cinco más gruesa y acotada), los puntos con su cota y el borde del terreno discontinuo. En Propiedades: equidistancia, profundidad, cotas más baja y más alta y la **cota de cada punto** (editable); con el terreno seleccionado, sus puntos se arrastran.
  - Secciones y alzados lo cortan (tierra con su trama de puntos), el 3D lo muestra. IFC: `IfcGeographicElement` `TERRAIN` con superficie triangulada, validado; ArchiSim lo lee de vuelta.
  - **Norte real**: Propiedades → Proyecto → «Norte (°)», en sentido horario desde el arriba de la planta. Se dibuja la **flecha de norte** en las plantas exportadas, la brújula del 3D lo sigue y va al IFC (`TrueNorth`; también se lee al importar de otro programa).
  - Demo: parcela con 25 puntos que cae suavemente hacia el sureste (de -0,01 a -0,49) y norte a 12°.
- **Punto 0 del proyecto forzable** (mejora 1 de Antonio): el origen (0, 0) se dibuja como un hito (círculo con dos cuartos rellenos) y es un punto de forzado propio, «Punto 0 del proyecto», que alcanza desde algo más lejos que los demás, para empezar muros, líneas o cotas desde él.
- **Cotas azules con extremos arrastrables** (mejora 2), como los de Revit: cada cota temporal tiene dos círculos en sus extremos. El de la **referencia** se arrastra a cualquier punto que fuerce (esquina, cara, extremo de muro, punto 0…) y la cota pasa a medir hasta él; el del **elemento** salta a otra línea del propio elemento: en puertas y ventanas, la otra jamba o el **centro** del hueco; en muros, la otra cara o el **eje**. Después se escribe el valor como siempre. Las asas del elemento seleccionado (extremos de muro, esquinas) tienen preferencia sobre estos círculos. Lo elegido se mantiene mientras el elemento sigue seleccionado.
- **Líneas finas** (mejora 3): botón «Líneas finas» en la barra inferior (también en Vista y en las propiedades de la vista): todas las líneas de la planta a un píxel, sin grosor; apagado, cada línea con su grosor real a la escala de la pestaña. Para cuando haya grosores por pluma (D5), este mismo botón los alternará.
- **Caja de sección en 3D** (mejora 4), como la de Revit: botón «Caja de sección» en la barra inferior del 3D (y en las propiedades de la vista 3D). Todo lo que queda fuera de la caja se recorta; lo cortado se ve **macizo**, con la tapa del corte del color de cada material (no hueco), y con sus aristas. Cada cara tiene una **flecha azul** que se arrastra para moverla; en Propiedades, las seis caras en mm y atajos: **Todo el modelo**, **Selección** (la caja alrededor de lo seleccionado), **Planta activa** (de su cota a la de la planta de encima) y **Planta en 3D** (corta la planta activa por su plano de corte: una planta en perspectiva). Lo que queda fuera de la caja no se puede seleccionar. Al apagarla y volver a encenderla, vuelve donde estaba.
- Versión 0.25.0.

**Pendiente / limitaciones conocidas**
- Secciones: sin cotas ni textos propios todavía (3.2), sin línea de terreno, sin sombras; las secciones quebradas (con saltos) llegarán más adelante. Un alzado de un modelo grande pesa (cada cara se dibuja aparte): si la vista va lenta, se optimizará uniendo caras.
- Pilares y vigas aún no recortan muros ni forjados (en planta el pilar tapa el muro; en 3D y en el IFC se solapan) ni se unen entre sí.
- Pilares y vigas inclinados, perfiles en L, U, T y tubos: más adelante.

**Antonio debe probar**
0. Fases (piensa en Poetas Andaluces): abre `demo.archisim`, selecciona todos los muros (ventana de selección) y ponlos en **Existente**; pon la tabiquería de un dormitorio **A derribar**; dibuja un tabique nuevo. En la barra inferior, «Fase»: mira **Demoliciones** (amarillo) y **Estado reformado** (rojo), en planta y en 3D. ¿Los colores y lo que aparece en cada estado son los que usáis en AD5? Exporta el IFC y comprueba en Revit/ArchiCAD si leen el estado (Status).
0. Locales: herramienta **Local** (tecla Z), clic dentro de cada recinto de una planta: ¿el sello (número, nombre, superficie útil) se lee bien a 1:50 y a 1:100? Cambia el uso en Propiedades y compara la superficie construida con la que medirías tú. Exporta el IFC y mira en Revit/ArchiCAD si salen como habitaciones/zonas con su número y nombre.
1. Descarga la versión de la rama (pull request «Fase 2 nocturna…», *Artifacts*) y abre `demo.archisim`: abajo, el porche con dos pilares cortados y la viga discontinua. En el navegador, Pilares (2) y Vigas (1).
2. Herramienta **Pilar** (tecla I): coloca pilares; Espacio los gira; cambia en Propiedades el tipo (galería con su sección). En la planta superior el pilar llega hasta ella: cambia la cota de esa planta en *Plantas…* y mira que el pilar la sigue (Propiedades → Altura).
3. Herramienta **Viga** (tecla G) en planta baja: une los centros de dos pilares. La viga sale discontinua (es de la planta de encima). Abre la Vista 3D.
4. Selecciona un pilar: en Propiedades → **Tipo**, cambia el ancho a 350: cambian todos los pilares de ese tipo. Prueba «Duplicar tipo» y «Perfil I / H».
5. *Exportar IFC* y ábrelo en **ArchiCAD y Revit**: ¿se ven pilares y vigas con su perfil, como pilares y vigas (no como objetos genéricos), con su material y el tipo con su nombre?
6. Pasa el cursor cerca del punto 0 (el hito de la esquina suroeste de la demo) con la herramienta Muro: debe engancharse a él («Punto 0 del proyecto» abajo a la derecha).
7. Selecciona una puerta: arrastra el círculo del extremo lejano de una cota azul hasta el punto 0 (o hasta la esquina exterior de la fachada) y el otro círculo hasta el centro de la puerta; escribe 2000 en la cota: el centro de la puerta debe quedar a 2 m de ese punto. Con un muro: arrastra su extremo propio al eje y el otro a cualquier punto.
8. Barra inferior → **Líneas finas**: la planta pasa a líneas de un píxel; otra vez, grosores reales. Prueba a 1:20 y a 1:100.
9. Vista 3D → barra inferior → **Caja de sección**: arrastra la flecha azul de la cara superior hacia abajo hasta ver el interior; gira la cámara. Prueba en Propiedades «Planta en 3D» y «Selección» (con un muro o un pilar seleccionado). ¿Las tapas del corte se leen bien como sección?
10. **Secciones**: en el navegador, «Secciones y alzados» → **Sección B** (pasa por el porche): ¿se lee como una sección vuestra? Mira el forjado de cubierta cortado con sus capas, la viga del porche cortada, el tabique y la puerta del fondo en vista. Abre **Alzado Sur**: fachada con sus dos ventanas. Cambia la escala a 1:50 y prueba «Líneas finas».
11. Herramienta **Sección** (tecla S): dibuja una línea de izquierda a derecha por tu casa (la flecha azul indica hacia dónde mira); doble clic en su marca abre la vista. Selecciónala, arrastra un extremo y vuelve a la pestaña: la sección se actualiza. Prueba «Invertir» en Propiedades.
12. **Rampa**: en `demo.archisim`, la rampa de la puerta oeste con su flecha «10 %». Selecciónala: en Propiedades → Pendiente, cambia a 8 y luego «Sube hacia: Norte»; arrastra la punta de la flecha en diagonal. Mírala en 3D y en «Alzado Oeste». Pon una cota de nivel encima (tecla N) a media rampa. Exporta el IFC: ¿Revit y ArchiCAD la leen como forjado inclinado?
13. **Cubierta**: abre un proyecto nuevo, dibuja cuatro muros en rectángulo y crea la planta de encima (*Plantas…*). En Planta baja, herramienta **Cubierta** (tecla T), clic dentro: cubierta a cuatro aguas en la planta de encima. En Propiedades cambia **Forma: a dos aguas**, la pendiente a 40 y el alero a 800; pon un lado concreto en «piñón». Mira la **planta de cubiertas** (pestaña de la planta de encima), el 3D y una sección. Exporta el IFC: ¿Revit y ArchiCAD la leen como cubierta con sus faldones? ¿Cómo quieres resolver las casas en L (limahoyas): cubierta única con contorno en L, o dos cubiertas que se unen?
14. **Escalera**: en un proyecto con dos plantas, herramienta **Escalera** (tecla E) en la de abajo: clic, clic (gira 90°), clic e Intro: escalera en L con meseta. Mira la etiqueta mientras dibujas y, al acabar, el grupo «CTE DB-SUA 1» de Propiedades; prueba a acortar el tramo hasta que salte el aviso, y cambia el **Uso** a restringido. Mira la planta (línea de corte, flecha), el 3D y una sección por la escalera. En la demo, la escalera exterior de la fachada este. Exporta el IFC: ¿Revit y ArchiCAD la leen como escalera con sus tramos y meseta?
15. **Terreno y norte**: en la demo, planta baja: curvas de nivel acotadas, puntos con su cota y la flecha de norte en la planta exportada (`demo.svg`, arriba a la derecha). Selecciona el terreno (clic en una zona vacía de la parcela) y cambia la cota de un punto: las curvas se mueven. Mira la **Sección A**: el terreno cortado bajo la casa. Prueba la herramienta **Terreno** (tecla R) en un proyecto nuevo con 5 o 6 puntos. Dime cómo te llegan normalmente los levantamientos (DXF con curvas, CSV de puntos…) para importarlos.

## 2026-10-19 · Familias, bloque D — diálogo de tipos al estilo ArchiCAD y tipos de línea (v0.24)

**Hecho**
- **Paletas laterales más estrechas y simétricas**: caja de herramientas y propiedades ocupan juntas lo mismo que el navegador (340 puntos por lado, ajustable arrastrando el borde de propiedades; el navegador lo sigue), así que el área de dibujo queda centrada aunque estén las tres abiertas. Iconos y rótulos de la caja algo más pequeños.
- **Diálogo «Tipos y materiales» rehecho**, al estilo de los ajustes de objeto de ArchiCAD:
  - A la izquierda, **carpetas** con los tipos agrupados por lo que no puede cambiar entre ellos: puertas y ventanas por **familia** (abatible, corredera; practicable, corredera, hueco), muros por **función** (exteriores, interiores), forjados por función (de planta, soleras, cubiertas) y materiales por **categoría**. Cada tipo con su **dibujo** (alzado de la carpintería, sección de las capas, muestra del material), su marca, medidas y nº de colocados. Buscador por nombre o marca, y un **«＋» en cada carpeta de familia** para crear un tipo nuevo de esa familia (aunque la carpeta esté vacía).
  - A la derecha, primero la **vista previa** con selector: **Alzado, Planta, Sección y 3D** en puertas y ventanas; Planta, Sección y 3D en muros; Sección y 3D en forjados. El alzado y la sección salen de las mismas piezas de la receta que el 3D y el IFC; la sección corta por la primera hoja con el muro anfitrión cortado capa a capa (con su trama) sobre el dintel y bajo el antepecho. El 3D se gira arrastrando, se acerca con la rueda y se encuadra con doble clic.
  - Debajo, **todas las opciones en grupos plegables** como la paleta de propiedades: Identidad, una por cada sección de la receta (Dimensiones, Hojas, Marco, Materiales, Representación…) y Datos.
- **Tipos de línea en las recetas**: continua, discontinua, de puntos, trazo y punto, trazo y dos puntos, trazos largos (patrones en mm de papel, escalados con el plano). Cualquier línea, polígono o arco de un símbolo puede llevarlos, y hay un tipo de parámetro nuevo, `linestyle`. La **puerta abatible** y la **ventana practicable** (versión 3) tienen «Línea del giro» en la sección Representación; en el diálogo se elige con una muestra de cada línea. Por defecto, continua (la planta sale igual que antes).
- Los huecos sin carpintería aparecen con las ventanas también al crear tipos nuevos.
- Versión 0.24.0. **Sin cambios en el formato `.archisim` ni en el IFC** (demo validada con IfcOpenShell; ida y vuelta idéntica salvo la fecha).
- **Comprobado por Antonio** (19/10/2026, 02:23): «funciona todo perfecto».

**Pendiente / limitaciones conocidas**
- Los proyectos anteriores guardan su copia de la puerta abatible y la ventana practicable versión 2, sin «Línea del giro»: la tendrán al actualizar familias (bloque E) o en proyectos nuevos.
- Del bloque D falta: símbolo 1:20 y líneas de apertura en el alzado desde la receta, y puntos de control (tiradores) en planta.
- Los nombres largos de la paleta de propiedades se recortan («Plano de corte (…»); se ven enteros al pasar el ratón.

**Antonio debe probar**
1. Descarga la versión de esta rama (pull request «Familias: diálogo de tipos al estilo ArchiCAD…», *Artifacts*).
2. Abre `demo.archisim` con las tres paletas abiertas: ¿te queda espacio cómodo para dibujar y el dibujo centrado? Arrastra el borde derecho de Propiedades: el navegador debe cambiar igual.
3. Navegador → *Tipos y materiales…* → **Puertas**: carpetas «Puerta abatible» y «Puerta corredera» con el dibujo de cada tipo. Cambia entre **Alzado, Planta, Sección y 3D**; en 3D, gíralo arrastrando.
4. En **Ventanas**, elige V-01, baja hasta **Representación → Línea del giro** y pon «Discontinua»: la vista Planta y el plano deben mostrar el giro discontinuo. Prueba el resto de tipos de línea.
5. Pulsa el **«＋»** de la carpeta «Hueco» y de «Ventana corredera»: debe crearse un tipo nuevo dentro de esa carpeta, seleccionado.
6. Revisa **Muros** (Planta/Sección/3D), **Forjados** (Sección/3D) y **Materiales** (carpetas por categoría). Dime qué opciones echas de menos respecto a ArchiCAD y qué agruparías distinto.

## 2026-10-19 · Familias, bloque C — IFC desde las familias (v0.23)

**Hecho**
- **El cuerpo IFC de cada puerta y ventana son las piezas de su receta** (las mismas cajas que ve la vista 3D), cada una con el estilo de su material. Volúmenes comprobados con el motor de geometría de IfcOpenShell (puerta de paso 80×205: 0,0168 m³ de cerco + 0,056 m³ de hoja = 0,0728 m³, igual que en ArchiSim).
- **Marca de tipo en el IFC**: en el atributo `Tag` del `IfcDoorType`/`IfcWindowType` y en la propiedad `Reference` de `Pset_DoorCommon`/`Pset_WindowCommon`, que es donde IFC pide la marca («A-1»). El tipo lleva también la **descripción** y el nombre de la **familia** (`ElementType`).
- **Los parámetros viajan en el IFC**: el tipo lleva `ArchiSim_Family` (familia, versión y la receta entera) y `ArchiSim_FamilyValues` (cada parámetro con su valor: longitudes en metros, sí/no, opciones, materiales por su nombre). Los parámetros de ejemplar, si la familia los tiene, van en la propia puerta o ventana. Cualquier programa que lea IFC los ve como propiedades.
- **Importación**: un IFC de ArchiSim rehace cada tipo con su familia (aunque sea una familia propia que el proyecto no tenía), sus valores, su marca y su descripción. Los IFC de otros programas se importan como antes, por medidas.
- La comparación de ida y vuelta comprueba ahora también marcas, descripciones, recetas, valores de tipo y de ejemplar y el funcionamiento de cada hoja. Prueba nueva con una familia inventada («Puerta de prueba», con mirilla, acabado y un parámetro de ejemplar), marca puesta a mano y descripción con comillas: vuelve idéntica, y su IFC valida con IfcOpenShell.
- Corregido de paso: importar un hueco sin carpintería creaba un material «Carpintería metálica» que no estaba en el fichero.
- Versión 0.23.0. Sin cambios en el formato `.archisim`.

**Pendiente / limitaciones conocidas**
- La geometría va en cada puerta y ventana, no compartida por tipo (`RepresentationMaps`): se probará cuando Antonio confirme cómo lo leen ArchiCAD y Revit.
- Un IFC de ArchiSim con otra versión de una familia que el proyecto ya tiene (al importar en un proyecto existente) se importa por medidas, con aviso.

**Antonio debe probar**
1. Descarga la versión de esta rama (pull request «Familias: IFC desde las recetas…», *Artifacts*) o espera a que se publique.
2. Abre `demo.archisim`, *Exportar IFC* y ábrelo en **ArchiCAD** y en **Revit**. Mira una puerta y una ventana: ¿se ven igual que antes (cerco, hoja, vidrio, montantes)? En sus propiedades de tipo: ¿aparece la **marca** (P-01, V-03…) donde la esperas? (En Revit suele salir como «Marca de tipo» o en `Reference`; en ArchiCAD, en la información IFC del tipo.)
3. Busca en las propiedades IFC del tipo `ArchiSim_FamilyValues`: ancho, alto, hojas, marco, materiales.
4. Cambia en ArchiSim la marca de un tipo a `P-10`, exporta, e *Importar IFC* ese mismo fichero: el tipo debe volver con `P-10` y la misma familia.

## 2026-10-18 · Mac: Cmd+Q pregunta antes de salir, y versiones publicadas (v0.22.2)

**Hecho**
- Antonio comprueba la v0.22.1: ya cierra sin aviso de fallo, pero **Cmd+Q cerraba sin preguntar** aunque hubiera cambios sin guardar. El menú de aplicación de macOS («Salir», Cmd+Q, «Salir» del Dock, cerrar la sesión) terminaba el programa sin avisar a la ventana. Ahora esa petición se cancela y se pasa a la ventana, que hace lo mismo que con el botón rojo: pregunta si hay cambios y, si no, sale.
- **Versiones publicadas en GitHub** (opción gratuita elegida por Antonio frente a la cuenta de desarrollador de Apple): al subir una etiqueta `vX.Y.Z` se publica la versión con `ArchiSim-Mac.dmg`, `ArchiSim-Windows.exe` e `instalar-mac.sh`. En el Mac se instala y actualiza con una línea en Terminal:
  `curl -fsSL https://github.com/stronkel/archisim/releases/latest/download/instalar-mac.sh | bash`
  Lo que baja `curl` no queda en cuarentena, así que ya no hace falta `xattr`. El script cierra ArchiSim si está abierto (preguntando si hay cambios) y sustituye la aplicación en Aplicaciones.
- La integración continua del Mac instala ahora con ese script desde el `.dmg` recién hecho, comprueba que no queda en cuarentena y que «Salir» cierra la aplicación.
- README y LÉEME del `.dmg` con la nueva forma de instalar. Versión 0.22.2. Sin cambios de formato ni de IFC.

**Pendiente**
- Fusionados en `main` los pull requests #13 a #16 (18/10/2026, a petición de Antonio); cerrados sin fusionar #10 y #11, que ya estaban en `main` por el #12. **Publicación automática**: cada vez que llega a `main` una versión de `Cargo.toml` que no está publicada, se publica sola (también se puede con una etiqueta `vX.Y.Z`). La v0.22.2 se publica así.
- Cerrar la sesión de macOS con ArchiSim abierto se cancela una vez (ArchiSim sale, pero hay que volver a cerrar la sesión). Se puede afinar si molesta.

**Antonio debe probar** (en el MacBook)
1. Instala la versión de esta rama desde Actions (con el `xattr`, por última vez).
2. Dibuja un muro y pulsa **Cmd+Q**: debe preguntar si guardas. Prueba «Cancelar» (sigue abierta) y luego «No guardar» (se cierra sin aviso de fallo).
3. Sin cambios, Cmd+Q cierra directamente. Repite con clic derecho en el icono del Dock → Salir.
4. Cuando esté publicada la versión, pega la línea de `curl` en Terminal y ábrela desde Aplicaciones: no debe pedir nada.

## 2026-10-18 · Mac: cierre sin aviso de fallo (v0.22.1)

**Hecho**
- Antonio informa de que, en su MacBook Pro con Touch Bar (Intel, macOS 15.8), cada vez que cierra ArchiSim macOS muestra el aviso de que la aplicación se ha cerrado inesperadamente. El informe de fallo lo sitúa en la Touch Bar: al destruir la ventana, AppKit intenta quitar un observador de la Touch Bar de una ventana ya cerrada y lanza una excepción. No pasa en Macs sin Touch Bar (por eso no salía en la integración continua) ni se pierde nada: ocurre después de guardar o descartar.
- Arreglo: en macOS, cuando ya se puede cerrar (proyecto guardado o descartado), la aplicación termina en el acto sin desmontar la ventana, así que la Touch Bar no llega a hacer esa limpieza. Windows y Linux, sin cambios.
- Versión 0.22.1. Sin cambios de formato ni de IFC (demo validada con IfcOpenShell).
- **Comprobado por Antonio** en su MacBook (18/10/2026, 22:15): ya cierra sin aviso de fallo.

**Pendiente**
- La cuarentena de macOS (`xattr` en cada versión): solo desaparece de verdad firmando y notarizando con una cuenta de desarrollador de Apple (99 USD al año); la alternativa gratuita es publicar versiones (*Releases*) e instalarlas con una sola orden que no deja cuarentena. Decide Antonio.
- Comprobar en el Mac que Cmd+Q con cambios sin guardar pregunta antes de salir (el menú de aplicación de macOS podría cerrar sin preguntar).

**Antonio debe probar** (en el MacBook)
1. Instala la versión de esta rama (Actions → ArchiSim-Mac) como siempre, con el `xattr` de momento.
2. Abre ArchiSim, dibuja un muro, guarda y ciérrala con el botón rojo: no debe salir ningún aviso de fallo.
3. Ábrela otra vez, dibuja algo y ciérrala sin guardar: debe preguntar; elige «No guardar»: tampoco debe salir aviso.
4. Repite el paso 3 con **Cmd+Q** y dime si pregunta antes de salir.

## 2026-10-18 · Repositorio público con licencia GPL-3.0

**Hecho**
- Antonio hace público el repositorio (los minutos de la integración continua del plan gratuito se habían agotado; en un repositorio público son gratis) y elige la licencia **GPL-3.0 o posterior** (D4, a falta del nombre definitivo).
- Revisado antes todo el historial: sin claves, sin IFC de proyectos reales, sin contenido de Autodesk ni Graphisoft.
- Fichero `LICENSE` (texto oficial de la GPL-3.0), licencia y repositorio en la descripción de cada paquete, `docs/FUNDACIONAL.md` §8 y hoja de ruta (D4).
- README nuevo para la comunidad: qué hace hoy, cómo instalarlo y compilarlo, cómo colaborar (sala limpia incluida) y licencia.
- Se hace directamente en `main`; los pull requests #13, #14 y #15 lo reciben sin conflictos (comprobado).

**Pendiente**
- Nombre definitivo (resto de D4) antes de anunciarlo.
- Publicar versiones (*Releases* de GitHub) para que se pueda descargar sin cuenta.

**Antonio debe probar**
1. Abre la página del repositorio en GitHub: debe salir el README nuevo y, a la derecha, «GPL-3.0 license».

## 2026-10-18 · Bloque 22 — Rendimiento con modelos grandes (1.9); fase 1 cerrada

**Hecho** (petición de Antonio: «termina todo lo que falta de la fase 1»)
- **Medir**: comando nuevo `archisim medir [n] [carpeta]`. Repite la vivienda de prueba en una rejilla de n × n y mide cada trabajo: planta entera, planta en pantalla (una zona y todo a la vista), 3D, geometría de un muro, varita mágica, guardar y abrir, exportar e importar el IFC. Deja `grande.archisim` y `grande.ifc` en la carpeta para abrirlos en la aplicación (con `ARCHISIM_MEDIR_IMAGENES=1`, también lo que se vería en pantalla). Una prueba aparte mide lo que hace la aplicación con el ratón sobre ese modelo.
- **Uniones**: cada muro buscaba sus vecinos entre todos los muros del modelo (el coste crecía con el cuadrado del número de muros). Ahora se buscan en una rejilla de celdas de 4 m: la geometría de un muro cuesta lo mismo en una casa que en una urbanización. Lo usan la planta, el 3D, el IFC y la varita mágica (que ya no recorre todo el modelo: busca en un radio que crece hasta encontrar el recinto).
- **Aplicación**: la geometría de todos los muros se calcula una vez por cambio del modelo y se reutiliza para forzar, seleccionar, el asa bajo el cursor, las cotas temporales y la vista previa de los arrastres (que solo compara lo que está cerca de lo arrastrado). Las puertas y ventanas de cada muro se buscan una sola vez.
- **Planta en pantalla**: solo la zona visible; al alejarse mucho se deja fuera lo que no se distinguiría: tramas y zigzag del aislamiento, las capas (el muro se dibuja como una banda gris, con sus líneas de corte) y el símbolo de las puertas y ventanas pequeñas (queda el hueco en el muro). Al acercarse vuelve todo. **Las plantas exportadas en SVG no cambian** (comprobado: idénticas a las de la v0.21, y el IFC también, salvo los `GlobalId` nuevos de la demo).
- Escritura de números del SVG más rápida (mismo resultado, con prueba).
- Cifras (servidor de pruebas, versión optimizada), rejilla **30 × 30: 6300 muros, 9000 huecos, 1800 forjados**:

  | Trabajo | Tiempo |
  |---|---|
  | Planta en pantalla, zona de 30 × 20 m | 0,07 s |
  | Planta en pantalla, todo a la vista | 0,5 s |
  | Forzado / elemento bajo el cursor | 10 ms / 8 ms |
  | Vista previa de mover un muro | 15 ms |
  | Mover un muro (con la geometría de todo rehecha) | 0,1 s |
  | 3D de todo el modelo (2,8 millones de triángulos) | 0,8 s |
  | Planta entera 1:50 exportada (SVG de 80 MB) | 1,5 s |
  | Guardar / abrir el proyecto (4,3 MB) | 0,09 s / 0,2 s |
  | Exportar el IFC (56 MB) / importarlo | 0,9 s / 3,8 s |

  Antes, en la 20 × 20 (2800 muros), la geometría de un muro tardaba ~9 ms y cada movimiento del ratón la recalculaba para todos los muros de la planta (segundos); la planta con todo a la vista tardaba 1,5 s y ahora 0,4 s.
- El IFC de 56 MB pasa la validación de IfcOpenShell.
- Pruebas nuevas: la geometría guardada coincide con la calculada antes y después de mover y de deshacer; la planta muy alejada dibuja bandas sin capas ni símbolos, y de cerca es idéntica a la de papel.
- Versión 0.22.0. **Con esto la fase 1 (§1 de la hoja de ruta) queda completa.**

**Pendiente / limitaciones conocidas**
- La **integración continua no se ha ejecutado** en los pull requests #13 y #14 (ni en este): GitHub no arranca los trabajos porque la cuenta tiene un **pago fallido o el límite de gasto agotado** (aviso en cada trabajo). Hasta que se resuelva en *Settings → Billing and plans* de GitHub no hay instaladores para descargar ni comprobación en Windows y Mac. Todo lo demás se ha comprobado aquí (Linux).
- En el 3D, cada cambio rehace la escena entera (0,8 s con 6300 muros): suficiente por ahora; si molesta, se rehará solo lo que cambia.
- Al alejarse mucho, todos los muros se ven del mismo gris (no por material).

**Antonio debe probar**
1. Resuelve el aviso de facturación de GitHub y vuelve a lanzar los trabajos de los pull requests (*Actions → Re-run all jobs*); descarga ArchiSim-Windows o ArchiSim-Mac de **Artifacts**.
2. Genera el modelo grande: `archisim medir 20 carpeta` (o `cargo run --release -p archisim-cli -- medir 20 out/medir`) y abre `grande.archisim` con la aplicación.
3. Aléjate del todo (botón de encuadre): debe verse la urbanización entera con muros grises, sin esperas largas. Acércate a una vivienda: capas, tramas y puertas como siempre.
4. Con la flecha, pasa el ratón por encima de muros y puertas (resaltado al momento), selecciona un tabique y arrástralo: la vista previa debe seguir al cursor sin tirones. Dibuja un muro con forzados a otros muros.
5. Exporta el IFC (unos 25 MB con la 20 × 20) y ábrelo en ArchiCAD o Revit; importa un IFC grande de un proyecto tuyo y dime cuánto tarda.

## 2026-10-18 · Bloque 21 — Instalador de Windows (1.8)

**Hecho**
- **Icono e información de versión** dentro de `ArchiSim.exe` (icono `.ico` generado con el resto de iconos por `scripts/make_icon.py`).
- **Instalador** `ArchiSim-<versión>-Windows.exe` (Inno Setup, `scripts/windows_installer.iss`): se instala para el usuario sin pedir permisos de administrador (se puede elegir instalar para todos); entrada en el menú Inicio, icono en el escritorio opcional, carpeta «Ejemplos de ArchiSim» con la vivienda de prueba y los IFC de ejemplo.
- **Tipos de fichero**: los `.archisim` se abren con ArchiSim y llevan su icono; los `.ifc` lo ofrecen en «Abrir con» y, si se marca en el instalador, se abren con él al hacer doble clic.
- El desinstalador quita el programa, los accesos y los tipos de fichero, y no toca los proyectos.
- La integración continua monta el instalador, lo instala en silencio, comprueba los tipos de fichero y lo desinstala.
- Versión 0.21.0.

**Pendiente / limitaciones conocidas**
- Sin firma de código: la primera vez, Windows SmartScreen pide «Más información → Ejecutar de todas formas».
- Pendiente de la primera ejecución en la integración continua (ver bloque 22: la cuenta de GitHub no arranca los trabajos).

**Antonio debe probar**
1. En un Windows: descarga ArchiSim-Windows de **Artifacts** del pull request «Instalación: instalador de Windows…» y ejecuta el instalador.
2. Menú Inicio → ArchiSim abre el programa; «Ejemplos de ArchiSim» tiene la vivienda de prueba.
3. Doble clic en un `.archisim` lo abre en ArchiSim (con su icono); botón derecho en un `.ifc` → «Abrir con» → ArchiSim.
4. Desinstala desde Configuración → Aplicaciones: desaparecen el programa y los accesos, y los proyectos siguen ahí.

## 2026-10-18 · Bloque 20 — Importación IFC, segunda vuelta (1.7), huecos de forjado y huecos sin carpintería

**Hecho** (petición de Antonio: «termina todo lo que falta de la fase 1»)
- **Muros de varios tramos**: un `IfcWall` con eje en polilínea, o en curva compuesta de rectas y arcos, se importa como un muro por tramo, unidos en sus esquinas (un muro en L son dos muros a inglete). El primero conserva el `GlobalId`; los demás reciben uno derivado, el mismo en cada importación. Las puertas y ventanas van al tramo en el que están.
- **Muros circulares cerrados** (eje `IfcCircle`, o un arco que da la vuelta entera): dos medias circunferencias.
- **Muros curvos de otros programas**: también con el arco recortado por ángulos en grados (como Revit) y capas centradas en un eje que no pasa por el origen del muro (corregido de paso: el desfase de las capas se mide desde el eje, esté donde esté).
- **Huecos de forjado** (escaleras, patinillos), nuevos en el modelo:
  - Herramienta Forjado → «**Hueco en un forjado**» (con Polígono o Rectángulo): el contorno se recorta del forjado de la planta que lo contiene. Se rechaza lo que se sale del forjado o pisa otro hueco.
  - Propiedades del forjado: lista de huecos con su superficie y «Quitar». La superficie descuenta los huecos. Mover y copiar el forjado lleva sus huecos. Un clic dentro del hueco no selecciona el forjado.
  - Planta: el hueco vacío con su **aspa** (discontinua si el forjado queda por encima del plano de corte); trama recortada. 3D: el hueco atraviesa todas las capas con sus caras. IFC: huecos del perfil (`IfcArbitraryProfileDefWithVoids`), superficie y volumen brutos y netos.
  - Importación: los huecos del perfil y los `IfcOpeningElement` que vacían el forjado (como exportan Revit y ArchiCAD).
- **Huecos sin carpintería** («hueco», abertura de muro de Revit): familia de sistema **Hueco**, marca H-01…, solo con hueco de obra. Se coloca con la herramienta Ventana (la plantilla trae «Hueco de paso 90×210»). Vacía el muro con sus mochetas; nada dentro. IFC: el `IfcOpeningElement` solo, con su propio `GlobalId` y el tipo en `ArchiSim_Void`; se importa (de ArchiSim, como el mismo tipo; de otros programas, «Hueco 90×200» por tamaño). La vivienda de prueba tiene uno, entre la cocina y el dormitorio.
- **Formato `.archisim` versión 6**: `holes` en los forjados; los ficheros de la versión 5 se abren tal cual.
- **Fichero de prueba nuevo** de IfcOpenShell, `ifcopenshell_formas.ifc`, escrito entidad a entidad como lo hace Revit (IFC2X3, milímetros): muro en L, cuarto de círculo con ángulos en grados, muro redondo, forjado con escalera en el perfil y forjado con patinillo como `IfcOpeningElement`. Se importa sin nada fuera y vuelve igual.
- El validador acepta huecos sin relleno marcados `ArchiSim_Void` (con sus cantidades).
- Pruebas nuevas: núcleo (huecos de forjado: dentro, cruces, solapes, contorno que los deja fuera, guardar), 3D (forjado con dos huecos: volumen y piel cerrada), fichero (huecos y familia Hueco, versión 5), importación (formas de Revit, hueco sin carpintería de la casa de IfcOpenShell) y ventana con clics (hueco de forjado, rechazo, selección, mover, quitar; hueco de paso con la herramienta Ventana, ida y vuelta por IFC).
- Pasan a la fase 2, porque necesitan elementos que aún no existen: **forjados inclinados** (2.7), **pilares y vigas** (2.5) y **locales** `IfcSpace` (2.9). Siguen en el informe de importación.
- Versión 0.20.0.

**Pendiente / limitaciones conocidas**
- Los vértices de un hueco de forjado no se editan con el ratón: se quita y se vuelve a dibujar.
- Los huecos sin carpintería salen en la galería y en el navegador junto a las ventanas.

**Antonio debe probar**
1. Descarga la versión del pull request «Importación IFC, segunda vuelta…» → **Artifacts** → ArchiSim-Windows o ArchiSim-Mac. Abre la vivienda de prueba: el **hueco de paso** entre la cocina y el dormitorio.
2. **F** → Método Rectángulo → marca «Hueco en un forjado» y dibuja un rectángulo dentro de la solera: aspa en planta, agujero en 3D. Selecciona la solera: el hueco está en Propiedades con «Quitar».
3. **V** → en la galería, «Hueco de paso 90×210», antepecho 0, clic en un tabique.
4. Importa un IFC real de ArchiCAD o Revit con una **escalera o un patinillo** y algún **muro en L o curvo**: los huecos deben salir abiertos y los muros, partidos por tramos.
5. Exporta y abre el IFC en ArchiCAD/Revit: el hueco de forjado y el hueco de paso deben verse abiertos.

## 2026-10-18 · Bloque 19 — Modos de construcción y muros curvos

**Hecho** (petición de Antonio al revisar 1.7: «otras formas de modelar muros, no solo de punto a punto: curvos, un cuadrado cerrado o un círculo; y forjados rectangulares por dos esquinas»)
- **Método de la herramienta Muro** (Propiedades → «Método», con iconos, como el método geométrico de ArchiCAD):
  - **Recto**: un muro por cada dos clics. **Cadena**: como hasta ahora (por defecto).
  - **Rectángulo**: dos esquinas opuestas → cuatro muros en sentido horario (la cara exterior queda fuera), unidos a inglete. También primera esquina + `6000x4000` + Intro (hacia el cuadrante del cursor).
  - **Arco**: inicio, fin y un punto por el que pasa (como «inicio-fin-radio» de Revit). En el tercer paso se puede teclear el **radio** (hacia el lado del cursor). Los arcos se encadenan; Esc quita el final del arco, otra vez Esc termina.
  - **Círculo**: centro y un punto de la línea de referencia (o radio tecleado): dos medias circunferencias en sentido horario.
  - Un rectángulo o un círculo es **un solo paso de deshacer** («Crear muros»).
- **Método de la herramienta Forjado**: Polígono · **Rectángulo** (dos esquinas, o `6000x4000`) · Varita mágica.
- **Muros curvos de punta a punta**:
  - Planta: capas, tramas y zigzag del aislamiento siguen el arco; caras con línea de corte; línea de referencia curva.
  - Uniones: inglete con muros rectos o curvos que comparten extremo (tangentes: corte radial, por ejemplo las dos mitades de un círculo); un tabique que acomete a un muro curvo se detiene en su cara (y un muro curvo que acaba dentro de otro, también). Aún no entran por capas.
  - 3D: una piel cerrada por muro, sin caras entre capas. Varita mágica dentro de un recinto curvo (cara interior, eje y cara exterior).
  - Forzado: **centro** (nuevo, círculo verde), extremos, esquinas, punto medio del arco y de cada cara, cualquier punto de las caras.
  - Edición: mover, copiar y estirar conservan la curvatura; un **círculo blanco** en el punto medio del muro seleccionado lo **curva** arrastrándolo (en línea con los extremos queda recto). Propiedades: **Radio** editable (los extremos no se mueven), ángulo y **Enderezar**. Longitud editable (la curva se escala).
  - Se rechaza un arco más cerrado que el espesor del muro, y un tipo que al engordar lo dejaría así.
  - **IFC**: eje `IfcTrimmedCurve` sobre `IfcCircle`; cuerpo extruido de un `IfcIndexedPolyCurve` con **arcos exactos** (`IfcArcIndex`); longitud, superficie y volumen del arco exacto. El validador afina el mallado de IfcOpenShell para comparar volúmenes de curvas.
  - **Importación** (parte de 1.7): muros curvos de un arco desde ArchiSim, ArchiCAD o Revit (`IfcTrimmedCurve`/`IfcCircle` o `IfcIndexedPolyCurve` con arco), con capas medidas en radial. Ida y vuelta idéntica comprobada.
  - **Formato `.archisim` versión 5**: campo `bulge` en los muros curvos (los rectos no lo escriben); los ficheros de la versión 4 se abren tal cual.
- Pruebas nuevas: arcos en el núcleo (comba, radio mínimo, curvar, puertas rechazadas), geometría (habitación redonda: áreas, capas, arcos exactos, volúmenes, varita; tabique contra cara curva; inglete recto-curvo y tangente), planta curva, IFC de ida y vuelta, herramienta Muro por modos, forjado rectangular y la ventana manejada con clics (rectángulo, círculo, arco, rectángulo tecleado, forjado rectangular, curvar con el ratón, puerta rechazada, guardar/exportar/importar). **Ventana real** revisada con capturas (Xvfb): selector de método, vista previa del arco con radio y longitud, círculo con radio tecleado, Propiedades de un muro curvo y curvarlo arrastrando.
- La demo da **el mismo IFC y las mismas plantas SVG** que la v0.18 (muros rectos sin cambios). IFC validados con IfcOpenShell (demo, sesiones de la app y `modos.ifc` con curvos).
- Versión 0.19.0.

**Pendiente / limitaciones conocidas**
- Puertas y ventanas en muros curvos (y su símbolo curvo); cotas temporales de muros curvos.
- Uniones en T por capas contra muros curvos (ahora el que acomete se detiene en la cara); tabiques que siguen a un muro curvo al moverlo (sí, por su cara); un muro curvo no «resbala» como seguidor.
- Forjados con bordes curvos o circulares (la varita da un polígono fino que sigue la curva); rectángulo girado, arco tangente, polígono regular.
- Resto de 1.7: muros de varios tramos, forjados inclinados y con huecos, huecos sin carpintería, `IfcSpace`.

**Antonio debe probar**
1. Descarga la versión del pull request «Modos de construcción…» (rama `bloque/modos-construccion`) → **Actions** → última ejecución en verde → **Artifacts** → ArchiSim-Windows o ArchiSim-Mac. Proyecto nuevo.
2. **M** → Propiedades → Método **Rectángulo**: clic en una esquina y en la opuesta. Cuatro muros cerrados; Ctrl+Z los quita de una vez. Repite con la primera esquina y escribe `8000x6000` + Intro.
3. Método **Círculo**: clic en el centro, escribe `3000` + Intro (radio de la línea de referencia). Mira la planta a 1:50 y en 3D (F3).
4. Método **Arco**: clic en el inicio, clic en el final y mueve el ratón: verás radio y longitud; clic para fijarlo (o escribe un radio + Intro).
5. **F** → Método **Rectángulo**: forjado por dos esquinas. Prueba también la varita dentro del círculo.
6. Selecciona un muro curvo: arrastra el **círculo blanco** del centro para curvarlo más o menos; en Propiedades cambia el **Radio** y prueba **Enderezar**.
7. Dibuja un tabique recto desde dentro del círculo hasta su cara: debe acabar contra la cara interior.
8. Exporta el IFC y ábrelo en **ArchiCAD y Revit**: ¿los reconocen como muros curvos (con su multicapa) o como objetos? Es lo que más me interesa saber de esta versión.
9. Dime qué método echas en falta (rectángulo girado, arco tangente, polígono regular, forjado circular) y si quieres puertas en muros curvos antes del bloque C.

## 2026-10-18 · Bloque 17 — Ayudas al dibujo (cotas temporales, rastreo, giros, otras plantas)

**Hecho** (petición de Antonio: «funciones de calidad de vida antes de las herramientas del bloque 2»; las seis, en este orden)
1. **Cotas temporales azules** (como Revit). Al seleccionar un muro o una puerta o ventana —y también el último muro dibujado y la puerta o ventana recién colocada, que quedan seleccionados— aparecen cotas azules:
   - **Puerta o ventana**: de cada jamba (hueco de obra) a la referencia más cercana de ese lado, medida en la cara hacia la que abre: la cara de un muro que la cruza o acomete (tabique), el final del muro (esquina interior) o la jamba del hueco vecino.
   - **Muro**: su longitud (línea de referencia) y la **luz libre de cara a cara** hasta el muro paralelo más próximo a cada lado.
   - **Clic en el número** → se escribe la medida (mm, o `1,2m`, `120cm`; el valor sale seleccionado y se sustituye al teclear) → **Intro**: el elemento se recoloca para que la cota sea exacta. La puerta se desliza por el muro; el muro se desplaza en perpendicular (los muros unidos le siguen, como al moverlo) o se alarga desde su final (la esquina le acompaña). Un paso de deshacer («Mover» / «Estirar»). **Esc** deja la cota como estaba. Lo que no cabe se rechaza con el motivo.
   - Solo con la flecha o con la herramienta que acaba de dibujar el elemento (las demás herramientas conservan sus clics).
2. **Puntos medios**: además del centro de la línea de referencia, ahora fuerzan el punto medio de **cada cara de muro** (también el centro del canto en los extremos) y de **cada arista de forjado**, con su triángulo verde.
3. **Muros rectos por defecto**: dibujando (muros, forjados, cotas) o arrastrando, la línea queda **horizontal o vertical** mientras el cursor no se aparte más de **8°**, y una **línea azul discontinua** a lo ancho de la vista lo indica; cerca de 45° también se bloquea. Si el punto cae sobre una **cara**, se desliza por ella hasta quedar **en escuadra**: un tabique dibujado hasta una cara acaba recto aunque el clic no sea exacto. Para inclinar un muro basta apartar el cursor más de 8°; Mayús sigue forzando múltiplos de 45°.
4. **Girar puertas y ventanas** (como Revit): con una seleccionada, **Espacio** recorre sus cuatro posiciones (bisagras a la otra jamba, luego la hoja al otro lado…). Dos **flechas azules** junto a ella (en el lado al que no abre): ⇅ la pasa **al otro lado del muro** con las bisagras en la misma jamba (la mano UNE cambia de nombre, porque se mira desde el otro lado) y ⇄ **cambia la mano**. Con la herramienta Puerta o Ventana, Espacio cambia la mano de la que vas a colocar. De regalo, como en Revit: Espacio con muros seleccionados los **invierte**.
5. **Cambiar de planta**: en Propiedades, «**Restricción de base**» del muro y «**Planta**» del forjado son ahora una lista de plantas (también con varios elementos seleccionados). El elemento pasa a esa planta con sus desfases; las puertas y ventanas van con su muro (en sus Propiedades se ve en qué planta están). Un muro que llegaba **a la planta de encima** sigue llegando a la de encima de la nueva (en la última planta queda con su altura, desconectado); unido a otra planta, la conserva, y se rechaza si no queda por encima. Un paso de deshacer («Cambiar de planta»).
6. **Otras plantas en planta**:
   - Los **muros de otras plantas que atraviesa el plano de corte** se dibujan **cortados**, con sus capas, tramas y huecos (por ejemplo, un muro de la planta baja con un desfase superior de 2 m que sube por la primera), en pantalla y en la planta exportada, como el rango de vista de Revit. También fuerzan.
   - **Calco de la planta inferior** (activado por defecto en pantalla; *Calco* en la barra inferior, en las propiedades de la vista o en *Vista*): los muros de la planta de debajo, rellenos en gris azulado claro con su contorno, y la posición de sus puertas y ventanas. **Sus puntos fuerzan** (extremos, esquinas, puntos medios, caras), para dibujar encima. No se exporta en la planta SVG.
- Ayudas en la pastilla de teclas y en *Ayuda → Ratón y teclado*.
- Pruebas nuevas: forzado (puntos medios de caras y aristas, la cara que lleva el punto, rastreo por ángulo, deslizamiento hasta la escuadra sobre una cara), cotas temporales (puerta a las esquinas interiores, teclear y deslizar, rechazo, longitud y luz libre de un muro, mover y estirar), giros (las cuatro posiciones de Espacio, las flechas conservan jamba o lado), núcleo (cambiar de planta muros y forjados, restricción superior relativa, rechazo sin cambios), planta (muros de otra planta cortados, calco solo si se pide) y la ventana manejada con clics y teclas simulados (muros rectos por defecto, último muro seleccionado, clic en la cota de una puerta + `500` + Intro, Esc, Espacio cuatro veces, las dos flechas, Espacio en un muro, calco que fuerza, muro alto de la planta baja visto desde la primera, cambiar de planta tres muros y volverlos). **Ventana real** revisada con capturas: cotas de una ventana, clic en el valor, `1000` + Intro («Cota fijada en 1000 mm»), Espacio, cotas de un tabique, planta Cubierta con la fachada alta cortada y el calco, forzado a una esquina del calco y la línea azul de rastreo.
- **IFC sin cambios**: la demo da el mismo IFC que la v0.17 (salvo los `GlobalId`, nuevos en cada demo), y las mismas plantas SVG byte a byte; validado con IfcOpenShell (reglas EXPRESS incluidas), también las sesiones de la app. Formato `.archisim` sin cambios (versión 4).
- Versión 0.18.0.

**Pendiente / limitaciones conocidas**
- Cotas temporales de **forjados** y cotas a **ejes** o a **núcleo** (Revit permite elegirlo); de momento: jambas y caras.
- Los muros de otra planta que se ven cortados **no se seleccionan** desde esta planta: se editan en la suya.
- En el calco, los muros de la planta inferior no se unen con los de la activa (son dibujos superpuestos).
- **Copiar a otra planta** (como «Pegar alineado» de Revit): no está; se puede cambiar de planta, no duplicar.
- Las cotas temporales no siguen al cursor mientras se arrastra (aparecen al soltar).

**Antonio debe probar**
1. Descarga la versión del pull request «Ayudas al dibujo…» (rama `bloque/ayudas-dibujo`) → **Actions** → la última ejecución en verde → **Artifacts** → **ArchiSim-Windows** o **ArchiSim-Mac**. Abre `demo.archisim`.
2. **Cotas temporales**: clic en la puerta de un dormitorio. Verás dos cotas azules de cada jamba a la cara del muro más cercano. Clic en un número, escribe `100` e Intro: la puerta queda a 10 cm exactos de esa cara (la mocheta habitual). Ctrl+Z. Prueba también `Esc` mientras escribes.
3. Selecciona un **tabique**: cota de longitud y luz libre hasta los muros paralelos. Escribe una luz libre (por ejemplo `3000`): el tabique se desplaza y los que acometen a él le siguen.
4. **Muros rectos**: tecla M y dibuja moviendo el ratón «a pulso»: deben salir rectos, con la línea azul. Aparta el cursor claramente para hacer un muro inclinado. Dibuja un tabique hasta la cara de una fachada haciendo el último clic un poco torcido: debe acabar en escuadra.
5. **Puntos medios**: con M, acerca el cursor al centro de la cara interior de una fachada o de un lado de la solera: triángulo verde.
6. **Girar**: selecciona una puerta y pulsa **Espacio** varias veces (cuatro posiciones); usa las dos flechas azules. Comprueba que la mano que dice Propiedades es la que esperas según UNE-EN 12519. Con la herramienta P, Espacio antes de colocarla.
7. **Cambiar de planta**: selecciona un tabique → Propiedades → «Restricción de base» → Cubierta: desaparece de la planta baja (el aviso lo dice) y aparece en Cubierta. Ctrl+Z.
8. **Otras plantas**: en una fachada de la planta baja pon «Desfase superior» `+2000` (o súbela de altura) y abre la planta **Cubierta**: esa fachada aparece **cortada**, y el resto de la planta baja en **gris** (calco). Dibuja un muro en Cubierta enganchando a las esquinas grises. Apaga *Calco* en la barra inferior.
9. Exporta el IFC tras estos cambios y ábrelo en ArchiCAD/Revit: los muros cambiados de planta deben salir en su nueva planta (`IfcBuildingStorey`).
10. Dime si las cotas de las puertas deben medir a la cara (como ahora) o al eje de los muros, y si echas en falta cotas temporales en algún otro elemento.

## 2026-10-18 · Bloque 16 — Restricción superior de muros

**Hecho** (petición de Antonio: «restricciones superiores de muro, de Planta baja a Planta 1, que se ajusten si cambia la altura de planta»; decidido hacerlo antes del bloque C)
- **Restricción superior**: un muro tiene altura desconectada (propia) o llega **hasta una planta** con un **desfase superior** (negativo: por debajo, por ejemplo el canto del forjado). Su altura la calcula el modelo y **cambia sola** al cambiar la cota de la planta superior o de la base, o el desfase de base.
- Se rechaza entero, explicando el motivo, lo que no cabe: bajar una planta hasta dejar la coronación en la base, o hasta que una ventana se salga del muro. Escribir una altura desconecta el muro (como en ArchiCAD); desconectarlo conserva la altura; borrar la planta superior lo desconecta con la altura que tenía.
- **Herramienta Muro**: en Propiedades, «Restricción de base», «Desfase de base», «Restricción superior» (Desconectada · **Planta superior** · Hasta cada planta de encima) y «Desfase superior» o «Altura desconectada». Por defecto dibuja **hasta la planta superior** (la de encima de la que dibujas, como «planta inicial + 1» de ArchiCAD); sin planta encima, 3 m propios, y lo avisa.
- **Muro seleccionado** (y varios a la vez, si están en la misma planta): la misma restricción superior; con planta, la altura se ve como «3000 · de las plantas».
- **Vivienda de prueba**: las fachadas llegan hasta la planta Cubierta menos el canto de la cubierta (los mismos 3 m de antes: el IFC sale idéntico). Sube la cota de Cubierta y las fachadas crecen con ella; los tabiques (2,70 m propios) no.
- **Formato `.archisim` versión 4**: los muros unidos a una planta llevan `"top": {"level": …, "offset": …}`. Los proyectos de las versiones anteriores se abren igual (sus muros, con su altura propia). El IFC no cambia (lleva la altura resultante, como Revit).
- **Propiedades a la izquierda** (bloque 15, segunda vuelta): ver la entrada anterior. Columna de nombres un poco más ancha.
- **Propuesta de disciplinas y mediciones** en `docs/DISCIPLINAS.md`: estructura (pilar, viga, cimentación con zapatas aisladas y corridas, losa, vigas de atado), urbanismo (terreno, parcela, viales por sección tipo, paisajismo; IFC 4.3, fuera de las mediciones del edificio) y mediciones con bancos BC3 y presupuesto vivo, en cinco fases (M1–M5). Decisiones D8–D12 para Antonio.
- Pruebas nuevas: núcleo (muro unido a la planta de arriba: sigue sus cotas y las de su base, rechaza lo que no cabe sin cambiar nada, se guarda y se reconstruye desde las cotas, se libera al escribir una altura o al borrar la planta), fichero (versión 4 con `top`, cota cambiada a mano en el fichero, fichero de versión 3 sin `top`) y ventana (dibujar hasta la planta superior, cambiar su cota, propiedades, desconectar y deshacer). Ventana real revisada con capturas.
- Versión 0.17.0.

**Pendiente / limitaciones conocidas**
- La restricción superior no viaja en el IFC (ni Revit la exporta): al importar, los muros quedan con altura propia.
- Los muros no se recortan todavía con el forjado de encima (por eso el desfase superior negativo).
- Pilares con restricción superior, cuando existan (2.5).

**Antonio debe probar**
1. Descarga la versión del pull request «Muros: restricción superior…» (rama `bloque/restriccion-superior`) → **Artifacts** → **ArchiSim-Windows** o **ArchiSim-Mac**. Abre `demo.archisim` **nuevo** (el del paquete).
2. Selecciona una fachada: «Restricción superior: Hasta Cubierta», «Desfase superior −548» (el canto de la cubierta) y «Altura 3000 · de las plantas».
3. Navegador → **Plantas…** → cambia la cota de **Cubierta** de 3548 a 3848. Mira la fachada en 3D: ha crecido 30 cm; los tabiques no. Ctrl+Z lo deshace.
4. Añade una «Planta primera» encima de la baja, vuelve a la planta baja y dibuja un muro: en Propiedades verás «Restricción superior: Planta superior (Planta primera)». Cambia la cota de la planta primera y comprueba que el muro la sigue.
5. Escribe una altura a mano en un muro unido: pasa a «Desconectada». Vuelve a unirlo eligiendo una planta en la lista.
6. Intenta bajar la Cubierta por debajo de la planta baja: debe negarse explicando por qué.
7. Lee `docs/DISCIPLINAS.md` y contesta D8 a D12; si puedes, pásame un presupuesto BC3 de una vivienda de AD5 como referencia de capítulos y criterios.

## 2026-10-18 · Bloque 15 — Interfaz rediseñada (ArchiCAD + propiedades de Revit)

**Hecho** (petición de Antonio: interfaz moderna, minimalista y práctica, «como un juego», parecida a ArchiCAD con las propiedades de Revit)
- **Barra superior** fina: menús, proyecto y fichero («sin guardar» si hay cambios), deshacer y rehacer, guardar, **Exportar IFC**, mostrar u ocultar la columna derecha, **tema claro u oscuro** (el dibujo sigue siendo papel blanco).
- **Caja de herramientas** a la izquierda por secciones (Selección · Modelado · Documentación · Vistas), con iconos propios dibujados como líneas, nombre y tecla. **Cota de nivel** pasa a ser herramienta propia (tecla **N**). Las que aún no existen se ven apagadas y dicen con qué punto de la hoja de ruta llegan. Botón para dejarla solo con iconos.
- **Pestañas de vistas** sobre el dibujo: cada planta y el 3D en su pestaña; cada planta recuerda su encuadre, su **escala** y su plano de corte. «+» abre vistas, × o clic con la rueda cierra, Ctrl+Tab pasa de una a otra, F2/F3 siguen funcionando. Una planta nueva se abre en su pestaña; borrarla la cierra.
- **Navegador**: Vistas (plantas con su cota, 3D), Planos (fase 3) y Modelo, con cada tipo de muro, forjado, puerta y ventana y su **número de ejemplares**. Clic: selecciona los de esa planta; doble clic: Editar tipo.
- **Propiedades al estilo Revit**: selector de tipo arriba (categoría, clase IFC, tipo con espesor o medidas, «✎ Editar tipo», «Seleccionar los N»), y grupos plegables Restricciones · Construcción · Dimensiones · Capas · Datos de identidad. Con una herramienta activa muestra lo que vas a dibujar (sustituye al antiguo cuadro de información de arriba); sin selección, la vista (escala, plano de corte, líneas de referencia, rejilla; nombre y cota de la planta) y el proyecto. El separador entre navegador y propiedades se arrastra.
- **Barra inferior**: escala **1:20 · 1:50 · 1:100 · 1:200** (cambia de verdad grosores, tramas, textos y símbolos de puertas y ventanas en pantalla), zoom en % del papel real, acercar/alejar/ajustar, **forzados activables** (Extremos, Medios, Aristas, Rastreo), Rejilla y Líneas de referencia; a la derecha el punto forzado, X/Y y la planta. En 3D: Perspectiva/Axonometría, ajustar, guardar imagen.
- **Sobre el dibujo**: las teclas del paso siguiente en una pastilla abajo y el resultado de cada acción como aviso que se desvanece arriba a la izquierda (los errores duran más).
- **Segunda vuelta con Antonio (mismo día)**: «las propiedades en el extremo opuesto obligan a cruzar la pantalla al elegir herramienta». **Propiedades pasa a la izquierda, junto a la caja de herramientas**, y el Navegador queda solo a la derecha (como Revit por defecto); cada columna se oculta por separado. El selector de tipo muestra el **dibujo del tipo a 1:20** (corte en planta del muro, capas del forjado, la puerta o ventana en planta) y el tipo se elige en una **galería** de fichas con su dibujo, nombre, medidas y un **buscador**. Ventana inicial de 1440 × 900.
- Tipografía IBM Plex (licencia OFL, incluida). Código de la ventana repartido en módulos (`app/`: barras, caja, pestañas, navegador, propiedades, tema, iconos).
- Pruebas nuevas: pestañas (nivel, encuadre y escala de cada una, F2/F3, Ctrl+Tab, cerrar, planta añadida y borrada), teclas y fichas de la caja, forzados de la barra inferior, la paleta de propiedades en todos sus estados y en los dos temas, elementos del mismo tipo. Todas las pruebas anteriores de la ventana pasan sin cambios. Ventana real revisada con capturas (planta, muro y ventana seleccionados, herramienta muro dibujando, 3D, tema oscuro, navegador, dos plantas a distinta escala, caja compacta, diálogos de tipos y plantas).
- Versión 0.16.0. **Sin cambios** en el modelo, el formato `.archisim` ni el IFC (demo, ida y vuelta y sesiones de la app validadas con IfcOpenShell).

**Pendiente / limitaciones conocidas**
- La disposición, el tema y las pestañas abiertas no se recuerdan al cerrar (se empieza siempre con la planta más baja, tema claro).
- Secciones, alzados y planos aparecen en el navegador como «llegan con…»: no existen todavía.
- No hay todavía paleta de órdenes ni atajos configurables.

**Antonio debe probar**
1. Descarga la versión de esta rama: GitHub → **Actions** → la última ejecución en verde del pull request «Interfaz: …» (rama `bloque/interfaz`) → **Artifacts** → **ArchiSim-Windows** o **ArchiSim-Mac**. Abre `demo.archisim`.
2. **Caja de herramientas**: pulsa M, F, P, V, C y N y mira cómo se ilumina cada ficha; pasa el ratón por «Pilar» o «Escalera»: dicen cuándo llegan. Pulsa la flecha ‹ de arriba: queda solo con iconos.
3. **Propiedades** (ahora a la izquierda): con la herramienta Muro activa, mira el corte del muro arriba; pulsa el nombre del tipo y elige otro en la galería (prueba a escribir «tabique» en el buscador); cambia la altura y dibuja un muro. Lo mismo con Puerta y Ventana. Selecciona un muro: «Seleccionar los 4» selecciona las fachadas; «✎ Editar tipo» abre su multicapa. Pliega y despliega los grupos.
4. **Navegador**: Modelo → Muros → clic en «Tabique LHD 10» (se seleccionan los tabiques), doble clic (se abre su tipo). Clic en «Cubierta» en Vistas: se abre en otra pestaña.
5. **Escala**: en la barra inferior pon **1:100** en la planta baja y luego vuelve a la pestaña de Cubierta y pon 1:20: cada pestaña conserva la suya. ¿Se ven bien los grosores y los símbolos a 1:100 y 1:20?
6. **Forzados**: con la herramienta Muro, apaga «Extremos» y comprueba que ya no se engancha a las esquinas; apaga «Rastreo» y comprueba que ya no se bloquea en horizontal.
7. **Tema oscuro**: botón ◐ arriba a la derecha.
8. Dime qué cambiarías: tamaño de las fichas, qué herramientas futuras sobran o faltan en la caja, si prefieres el navegador a la izquierda (como el Navegador de proyectos de Revit) y si el cuadro de información de ArchiCAD arriba se echa de menos o está bien en Propiedades.

## 2026-10-18 · Fase 2, bloque 14 — Familias: planta por escala y hojas configurables (bloque B)

**Hecho**
- **Las puertas y ventanas se dibujan en planta con el símbolo de su receta**, uno por escala (`plan 1:50`, `plan 1:100`), como ArchiCAD elige el nivel de detalle por escala. La planta ya no tiene dibujo de puertas y ventanas programado: cerco, hoja abierta a 90° y su giro, corredera con flecha, marco, vidrio, montantes, giro de cada hoja, vierteaguas y canto del alféizar salen de las recetas, con piezas comunes de planta (`piezas.asfam`).
- **Misma planta que antes**: la vivienda guardada por la v0.13 da la planta 1:50 idéntica en imagen (0 píxeles distintos a 4 aumentos), el mismo IFC y el mismo 3D. Una prueba nueva dibuja 176 casos (todos los funcionamientos, dos tamaños, los dos lados, las dos manos, con y sin antepecho, con y sin retranqueo, en un muro inclinado) con el dibujo antiguo y con las recetas y compara las imágenes.
- **Planta 1:100**: puertas con la hoja como una línea y su giro, sin cerco; ventanas con marco y vidrio. Se exporta desde *Archivo → Exportar planta 1:100 (SVG)…* y la línea de comandos la genera junto a la 1:50 (`demo-1a100.svg`).
- **Ventana practicable de 1 a 4 hojas, cada una con su funcionamiento** (tu propuesta): fija, practicable, oscilobatiente, abatible, proyectante o basculante. La «Ventana fija» deja de ser familia: es una practicable de una hoja fija (los proyectos anteriores se convierten solos). En planta giran las practicables y oscilobatientes; las hojas de la mitad izquierda giran en su jamba izquierda y las de la derecha en la derecha; una hoja sola, o la central si son tres, según la mano. Montantes entre hojas en planta y en 3D.
- **IFC por hoja**: el tipo de ventana lleva la partición (1, 2 o 3 hojas; 4 como «definida por el usuario») y el funcionamiento IFC de cada hoja (`FIXEDCASEMENT`, `TILTANDTURNLEFTHAND`, `TOPHUNG`…) con su posición. Validado con IfcOpenShell. **La importación lo lee**: una ventana de 3 o 4 hojas de otro programa entra con sus hojas y su funcionamiento (antes se importaba como de dos).
- **Diálogo de tipos generado desde la receta**: en Puertas y Ventanas, debajo de nombre, marca, descripción y familia, aparecen los parámetros de la receta por secciones (Dimensiones, Hojas, Cerco/Marco, Materiales) con el control que toca a cada uno (medida en mm, número de hojas, lista de funcionamiento, material); solo se ven los que aplican (la hoja 3 aparece al elegir 3 hojas). Cada cambio es un paso de deshacer y se aplica a todas las del tipo; la vista previa en planta 1:20 se dibuja con la receta.
- **＋ Nuevo…** en las pestañas de puertas y ventanas: un tipo nuevo de cualquier familia (las del proyecto y las de sistema) con sus valores por defecto. El funcionamiento ya no se cambia en un tipo existente (una corredera no se convierte en abatible): se crea un tipo de la otra familia.
- En Propiedades, «Funcionamiento» dice cómo abre cada hoja («3 hojas: fija · oscilobatiente · fija»).
- Lenguaje: bloques `plan 1:N` con líneas, contornos y arcos, plumas (corte, marco, vista, fina, rayado), nombres que da el programa en planta (`wall_ext`, `wall_int`, `sill_height`) y unión de textos con `+`.
- Pruebas: lenguaje (15), núcleo (36, con las familias recorridas en todas sus opciones de hojas, medidas extremas, lados y manos, también en planta 1:50 y 1:100 y con su IFC por hoja), planta (comparación de imagen), IFC (ida y vuelta con ventanas de 3 y 4 hojas y una proyectante), editor (valores de la receta, hojas fija y oscilobatiente, tipo nuevo), y la ventana real con capturas (diálogo, planta y 3D de una ventana de tres hojas).
- Versión 0.15.0. Sin cambios en la estructura del fichero `.archisim` (versión 3); las familias de sistema pasan a la versión 2 de su receta. Los proyectos guardados con la v0.14 conservan su copia de las familias antiguas y se siguen abriendo igual.

**Pendiente / limitaciones conocidas**
- El símbolo 1:20 y el alzado no están todavía en las recetas: a 1:20 se usa el de 1:50 (bloque D). Los muros a 1:100 se dibujan igual que a 1:50 (con capas y tramas): eso es de las normas de dibujo (D5) y las plantillas de vista (3.1).
- El IFC sigue escribiendo el cuerpo de puertas y ventanas con el código anterior (ya con el número de hojas de la receta); que salga de la receta, con la marca y los parámetros en Psets, es el bloque C.
- Hojas de puerta desiguales, hoja activa, tarjas, corredera empotrada y la **puerta plegable** van en la librería (2.2).
- Las hojas abatibles, proyectantes y basculantes no tienen símbolo en planta (se verán en alzado).

**Validación de Antonio (18/10/2026):** probada la versión de los bloques A y B: «funciona todo perfecto». Integrados en `main` los PR #6 y #7.

**Antonio debe probar**
1. Descarga la versión de esta rama: GitHub → **Actions** → la última ejecución en verde del pull request «Familias: planta por escala…» (rama `bloque/familias-planta`) → **Artifacts** → **ArchiSim-Windows** o **ArchiSim-Mac**. Abre `demo.archisim` (o uno tuyo de antes): la planta debe verse exactamente igual.
2. *Tipos y materiales* → **Ventanas** → «Ventana practicable 2 hojas 180×130»: pon **Nº de hojas = 3**, Hoja 1 **Fija**, Hoja 2 **Oscilobatiente**, Hoja 3 **Fija**. Cierra y mira la fachada sur en planta: dos fijos, montantes y la hoja central con su giro. Mira el 3D (tres paños). Ctrl+Z deshace cada paso.
3. Selecciona esa ventana: en Propiedades, «Funcionamiento: 3 hojas: fija · oscilobatiente · fija». Cambia su **Mano**: la hoja central gira hacia el otro lado.
4. «Ventana fija 60×60»: verás que su familia es «Ventana practicable» con 1 hoja fija. Cámbiala a practicable y mira el giro.
5. **＋ Nuevo…** → «Puerta corredera»: aparece «Puerta corredera (nuevo)» con la marca siguiente; colócala con la herramienta Puerta.
6. *Archivo → Exportar planta 1:100 (SVG)…* y ábrela: ¿te vale como planta 1:100 de puertas y ventanas?
7. Exporta el IFC y ábrelo en ArchiCAD y Revit: la ventana de 3 hojas debe verse con sus tres paños. Si tu programa muestra las propiedades de hoja del tipo (`IfcWindowPanelProperties`), deben decir fija, oscilobatiente y fija.
8. Dime qué sigue: **C** (IFC desde las familias, con marca y parámetros) o **D** (configurador con vista previa en alzado y 3D).

## 2026-10-18 · Fase 2, bloque 13 — Familias: núcleo (bloque A)

**Hecho**
- **Lenguaje de familias** (recetas `.asfam`, crate nuevo `archisim-family`): parámetros de tipo y de ejemplar por secciones, con rango, nombre en español y condición de visibilidad; fórmulas, condiciones (`if / else if / else`), repeticiones, **piezas comunes** reutilizables y reglas `require … else "mensaje"`. Es seguro: no toca ficheros ni red y no puede quedarse colgado (topes de repeticiones y de piezas). La sintaxis real, con un ejemplo que las pruebas comprueban, está en `docs/FAMILIAS.md`.
- **Cinco familias de sistema** escritas como recetas, con sus piezas comunes (`piezas.asfam`: cerco, hoja, marco, acristalamiento con montantes): **Puerta abatible** (1 o 2 hojas), **Puerta corredera**, **Ventana fija**, **Ventana practicable** (1 o 2 hojas) y **Ventana corredera**. Se pueden leer en `crates/archisim-core/families/`.
- **Los tipos de puerta y ventana son ahora tipos de familia**: familia + valores de sus parámetros + **marca de tipo** (P-01, P-02… para puertas, V-01… para ventanas, asignada sola y editable, única en el proyecto) + descripción. Duplicar un tipo copia familia y valores con marca nueva y `GlobalId` nuevo.
- **El 3D de puertas y ventanas sale de las recetas.** La vista 3D de la vivienda de prueba es idéntica, píxel a píxel, a la de la versión anterior; una prueba compara además 112 puertas y ventanas (todos los funcionamientos, dos tamaños, los dos lados y las dos manos, en un muro inclinado) con el cálculo antiguo: coinciden a la billonésima de metro.
- **Formato `.archisim` versión 3**: el proyecto guarda una copia de cada familia que usa (la receta, una línea por renglón, legible) y los tipos como familia + valores + marca. Los proyectos v1 y v2 se abren solos: cada tipo pasa a su familia de sistema con el mismo nombre, `GlobalId`, medidas y materiales, y recibe su marca. Comprobado con la vivienda guardada por la v0.13 (incluida como fichero de prueba): **mismo IFC, salvo la fecha, y mismas plantas byte a byte** que exportaba la v0.13.
- **Aplicación**: en *Tipos y materiales* → Puertas / Ventanas aparecen **Marca de tipo** y **Descripción** (editables, con deshacer) y la **Familia** de la que sale el tipo; la lista muestra la marca delante de las medidas. En Propiedades de una puerta o ventana, su marca y su familia. Los errores de las familias se explican en español con el nombre del parámetro («Hueco de obra: ancho = 25 está fuera de su rango…») o con el mensaje de la propia receta («El marco y el montante no dejan hueco para el vidrio»).
- Pruebas nuevas: lenguaje (12: lectura, fórmulas, piezas, condiciones, errores de sintaxis con su línea, valores fuera de rango, límites), núcleo (familias de sistema recorridas en **más de mil combinaciones** de opciones, medidas extremas, lados y manos: geometría válida y dentro del hueco; conversión exacta de los 7 funcionamientos antiguos; marcas; duplicar y editar), fichero (migración v1 y v2, la vivienda de la v0.13, errores de recetas y valores), editor (marca y deshacer) y la comparación del 3D con el cálculo anterior. Ventana real probada con capturas (Propiedades, diálogo de tipos, 3D). IFC de la demo y de las sesiones de la app validados con IfcOpenShell (reglas EXPRESS incluidas).
- Versión 0.14.0.

**Pendiente / limitaciones conocidas**
- **Planta e IFC siguen dibujando puertas y ventanas como antes**, a partir de las medidas que da la familia (bloques B y C). Por eso, una familia nueva distinta de las cinco de sistema todavía no tendría símbolo en planta propio.
- El diálogo de tipos sigue siendo el de la v0.12 (funcionamiento, medidas, marco, materiales); el configurador generado desde la receta, con vista previa en planta, alzado y 3D, es el bloque D. Cambiar el funcionamiento de abatible a corredera aún se permite desde ese diálogo (cambia de familia); en el configurador no se permitirá, como en ArchiCAD.
- Aún no se pueden cargar familias propias ni guardar tipos en una biblioteca (bloque E). La marca no sale todavía en el IFC (bloque C).

**Antonio debe probar**
1. Descarga la versión de esta rama: GitHub → **Actions** → la última ejecución en verde del pull request «Familias: núcleo…» (rama `bloque/familias-nucleo`) → **Artifacts** → **ArchiSim-Windows** (o **ArchiSim-Mac** en el MacBook).
2. Abre un proyecto **guardado con la versión anterior** (por ejemplo tu `demo.archisim` de antes): debe abrirse igual, en planta y en 3D.
3. Selecciona la puerta de entrada: en Propiedades verás **Marca de tipo P-01** y **Familia Puerta abatible (v1)**. Pulsa ✎ Editar tipo…: en la lista, cada tipo lleva su marca delante. Cambia la marca a `P-10` y escribe una descripción; Ctrl+Z lo deshace. Prueba a poner a otra puerta la misma marca: debe negarse («la marca «P-10» ya la tiene otro tipo»).
4. **Duplicar** un tipo de ventana: la copia recibe la siguiente marca libre (V-05).
5. Ventanas → «Ventana practicable 2 hojas 180×130»: pon «Marco: anchura vista» a `620` mm. Debe negarse con el mensaje de la propia receta: «El marco y el montante no dejan hueco para el vidrio» (dos jambas y el montante central suman 1860 mm, más que el hueco de 1800).
6. Guarda, cierra y abre otra vez; exporta el IFC y ábrelo en ArchiCAD/Revit: puertas y ventanas iguales que con la versión anterior.
7. Abre el `.archisim` guardado con un editor de texto y busca `"families"`: verás las recetas de las familias, línea a línea. Échales un vistazo (por ejemplo «Ventana practicable»): ¿se entiende qué hace cada parte?
8. Dime qué prefieres después: **B** (símbolos de planta de las familias a 1:100, 1:50 y 1:20) o **C** (IFC desde las familias, con la marca y los parámetros).

## 2026-10-18 · Fase 2 — Diseño de las familias paramétricas (decisión D2)

**Hecho**
- Diseño del sistema de familias en `docs/FAMILIAS.md`: qué es una familia (parámetros de tipo y de ejemplar con fórmulas, geometría 3D, símbolo de planta por escala, comportamiento y puntos de control, salida IFC), un ejemplo de receta de una ventana practicable, las reglas del sistema y el plan en cuatro bloques con su criterio de hecho.
- **Decisiones de Antonio**: D2 = **receta de texto** (editor gráfico más adelante sobre el mismo formato); familias **copiadas dentro del proyecto**, como Revit; nivel de detalle **por escala**, como ArchiCAD; se empieza por **puertas y ventanas**.
- **Afinado con Antonio** (misma mañana): el arquitecto crea tipos con un **configurador como el de ArchiCAD** (todas las opciones por secciones, vista previa en planta, alzado y 3D en directo); la receta es el motor de debajo. **Una familia por funcionamiento** (puerta abatible y corredera; ventana practicable, corredera y fija) con hojas, tarjas, subdivisiones, persiana… como opciones y **piezas comunes** compartidas. **Tipos con nombre, marca (P-03) y `GlobalId`**, todo lo que define el producto es del tipo, **guardables en ficheros de familia** con su catálogo para usarlos en otros proyectos, y **navegador de familias y tipos con su número de ejemplares** (el tipo es la unidad de medición). El plan pasa a cinco bloques (A–E).
- Hoja de ruta, fundacional y prioridades de `CLAUDE.md` actualizados. Sin código: ningún cambio en el programa, el formato `.archisim` ni el IFC.

**Pendiente**
- Bloque A (núcleo de familias). Criterio: la demo y los proyectos antiguos dan exactamente el mismo 3D e IFC que hoy, pero con las puertas y ventanas como familias.
- Sigue abierto D5 (normas de dibujo): fija los grosores con los que se dibujarán los símbolos de las familias por escala.

**Antonio debe probar**
1. Lee `docs/FAMILIAS.md`, sobre todo el ejemplo de receta: ¿entiendes qué hace cada línea? ¿Te ves retocando una (cambiar un rango, añadir un parámetro)?
2. Revisa la sección «Tipos: reutilizables y medibles» y la tabla de los cinco bloques: ¿es así como quieres trabajar con tus puertas y ventanas de AD5?
3. Si tienes un objeto de ArchiCAD que uses mucho (ventana con persiana y capialzado, por ejemplo), descríbeme sus parámetros tal como los ves en su diálogo: será la primera de la librería (2.2).

## 2026-10-17 · Cierre de fase 1, bloque 12 — Aplicación de Mac

**Hecho**
- **ArchiSim para Mac** en un `.dmg`: la integración continua compila la aplicación para chip Apple (M1 y posteriores) e Intel en un solo `ArchiSim.app` (universal) y la mete en una imagen de disco con un acceso a Aplicaciones, la vivienda de prueba (`demo.archisim`), los IFC de ejemplo y un `LEEME.txt` con cómo abrirla la primera vez. Se descarga igual que la de Windows: *Actions → Artifacts → **ArchiSim-Mac***.
- **Doble clic** en un `.archisim` en el Finder abre ArchiSim con ese proyecto (los `.archisim` quedan asociados a ArchiSim, con su icono). Los `.ifc` aparecen en *Abrir con → ArchiSim* sin quitarle el doble clic a tu visor IFC habitual. También se puede soltar un fichero sobre el icono del Dock. Si hay cambios sin guardar, pregunta antes.
- En **todos los sistemas**: soltar un `.archisim` o un `.ifc` sobre la ventana lo abre o lo importa; cualquier otro fichero se rechaza con un aviso.
- **Icono** propio (provisional, hasta decidir nombre e imagen): la esquina de un muro de fachada en planta con sus capas y una puerta. También es el icono de la ventana en Windows y Linux.
- La aplicación no está firmada con una cuenta de desarrollador de Apple (99 €/año): va firmada «ad hoc», que basta para usarla tras confirmarla una vez en *Ajustes del Sistema → Privacidad y seguridad → Abrir igualmente*.
- Pruebas: abrir ficheros soltados en la ventana (proyecto, IFC, fichero ajeno, con cambios sin guardar) con la app manejada sin pantalla. En la integración continua, en un Mac real: se monta el `.app`, se comprueba su firma y su `Info.plist`, se crea el `.dmg` y se abre `demo.archisim` «desde el Finder» (orden `open`) comprobando que llega a ArchiSim.
- Versión 0.13.0 (la 0.11 y la 0.12 son las de los PR #2 y #3). Sin cambios en el formato `.archisim` ni en el IFC.

**Pendiente / limitaciones conocidas**
- Sin firma ni notarización de Apple: hace falta la cuenta de desarrollador cuando se distribuya a otros.
- Windows sigue sin instalador, sin icono en el `.exe` y sin asociación de ficheros (resto del punto 1.8).
- Un fichero cada vez: si sueltas varios, se abre el primero y se avisa de los demás.

**Validación de Antonio (18/10/2026):** probada en su MacBook Pro Intel con macOS Sequoia la versión con los bloques 10, 11 y 12 juntos (tras salir del bucle de Gatekeeper con `xattr`): funciona. Integrados en `main` los PR #2, #3 y #4.

**Antonio debe probar**
1. En el MacBook: GitHub → **Actions** → la última ejecución en verde de `main` → **Artifacts** → **ArchiSim-Mac**. Descomprime y abre el `.dmg`.
2. Arrastra **ArchiSim** a **Aplicaciones** y ábrela. Cuando macOS diga que no puede comprobar el desarrollador: *Ajustes del Sistema → Privacidad y seguridad → Abrir igualmente*. En macOS Sequoia puede volver a pedirlo en bucle: entonces, en Terminal, `xattr -dr com.apple.quarantine /Applications/ArchiSim.app` y doble clic (comprobado por Antonio en un MacBook Pro Intel con Sequoia).
3. Copia `demo.archisim` del `.dmg` a tu Mac y haz **doble clic**: debe abrirse en ArchiSim. Navega en planta y en 3D (F3) con el trackpad: ¿va fluido?
4. Clic derecho en un `.ifc` → **Abrir con → ArchiSim**: debe importarse y salir el informe.
5. En Windows y en Mac: arrastra un `.archisim` o un `.ifc` sobre la ventana de ArchiSim abierta.
6. ¿Te vale el icono provisional o prefieres otra idea?
## 2026-10-17 · Cierre de fase 1, bloque 11 — Editor de tipos y materiales

**Hecho**
- Diálogo **Tipos y materiales**: *Vista → Tipos y materiales…*, botón «Tipos y materiales…» arriba a la derecha, o **✎ Editar tipo…** en las Propiedades de un muro, forjado, puerta o ventana (abre el diálogo en su tipo, como «Editar tipo» de Revit). Pestañas **Muros · Forjados · Puertas · Ventanas · Materiales**: a la izquierda la lista (con espesor y cuántos elementos lo usan), a la derecha sus datos.
- **Multicapas** de muro y forjado: nombre, función (muro exterior/interior; forjado de planta, solera o cubierta plana) y tabla de capas — material, espesor (mm) y función de cada una (estructura, soporte, aislamiento, cámara de aire, lámina, acabado), botones para subir y bajar, duplicar y quitar capas. Espesor total al día.
- **Puertas y ventanas**: nombre, funcionamiento (abatible, dos hojas, corredera; fija, practicable de 1 y 2 hojas, corredera), hueco de obra, anchura vista y fondo del marco, material del marco y de la hoja o vidrio.
- **Materiales**: nombre, categoría IFC, color de corte (selector de color o tecleado `232, 196, 178`) y trama de corte (liso, rayado diagonal, rayado cruzado, zigzag de aislamiento, punteado, vacío). Se ve en qué tipos se usa.
- **Vista previa a 1:20** con el mismo motor de dibujo que la planta (no es un dibujo aparte): un trozo de muro del tipo cortado en planta, exterior arriba; las capas del forjado como banda en sección (cara superior arriba); la puerta o ventana colocada en un muro del proyecto, con su mocheta y su giro; el material en un muro de 20 cm.
- **Crear = Duplicar** (como Revit): copia con nombre «… (copia)» y `GlobalId` nuevo, y queda seleccionada para editarla. **Borrar** solo lo que no se usa, diciendo por qué no (p. ej. «lo usan 4 muros», o los tipos que usan un material); siempre queda al menos un tipo de cada clase.
- Cada cambio es **un paso de deshacer** («Modificar tipo de muro», «Modificar material»…; arrastrar en el selector de color es un solo paso) y se aplica al momento a **todos los elementos del tipo**, en planta y en 3D: los muros conservan su línea de referencia y su `GlobalId`; las puertas y ventanas, su centro y antepecho. Lo que no cabe se rechaza entero con el motivo, en la barra de estado y en el propio diálogo (un muro más fino que el marco de sus puertas, una puerta que al ensancharse se sale del muro o pisa una ventana).
- Nombres únicos por lista (vacío o repetido se rechaza): el IFC y la importación reconocen los materiales por su nombre.
- Si se borra (o se deshace) el tipo que tenía una herramienta, la herramienta pasa a otro tipo existente.
- Pruebas: núcleo (editar y borrar materiales, multicapas y tipos de puerta comprobando todos los elementos que los usan, con rechazo y sin cambios), editor (duplicar con nombre libre, nombres repetidos o vacíos, borrar en uso, deshacer, último tipo), vistas previas (todas las puertas y ventanas de la plantilla quedan cortadas por el plano de corte) y la app: casa dibujada con clics simulados, las cinco pestañas con su vista previa, fachada duplicada con 8 cm de aislamiento y un material nuevo, puerta ensanchada y marco demasiado profundo rechazado, deshacer y rehacer. Esa casa se guarda, se reabre igual, se exporta (`out/app/tipos.ifc`, validado con IfcOpenShell con reglas EXPRESS, también en la integración continua) y se reimporta idéntica. Ventana real probada con capturas.
- Versión **0.12.0** (la 0.11 es la del PR #2, uniones en T, pendiente de revisión). Sin cambios en el formato `.archisim` ni en el IFC exportado.

**Pendiente / limitaciones conocidas**
- Sin prioridades de intersección editables por material (el PR #2 las deduce de la función de la capa; editarlas va con el punto 2.3).
- Sin importar o exportar materiales y tipos entre proyectos (bibliotecas o plantillas propias) ni ordenar las listas a mano.
- Los tipos de puerta y ventana siguen siendo los sencillos del MVP: premarco, persiana, capialzado y vierteaguas paramétricos llegarán con las familias (2.1, 2.2).
- La vista previa del forjado es la banda de capas con las tramas de planta: la sección de verdad llega con 2.10.
- Las categorías de material son una lista cerrada en el diálogo; las de otros programas importadas se ven como «Otra (…)» y se conservan.

**Antonio debe probar**
1. Descarga la versión de esta rama: GitHub → **Actions** → la última ejecución en verde del pull request «Tipos: editor de tipos y materiales (v0.12)» (rama `bloque/tipos-y-materiales`) → **Artifacts** → **ArchiSim-Windows**. Abre `demo.archisim`.
2. Selecciona la fachada sur → Propiedades → **✎ Editar tipo…**. Cambia la lana mineral de `50` a `80` + Intro: el espesor pasa a 295 mm y las cuatro fachadas engordan hacia dentro (la cara exterior se queda en su sitio). Ctrl+Z lo deshace.
3. **⧉ Duplicar** la fachada, renómbrala, añade una capa con ⧉, cámbiale el material y súbela con ▲. Aplica el tipo nuevo a un muro desde Propiedades. Mira la vista previa: ¿se lee como tu multicapa a 1:20?
4. Pestaña **Puertas** → «Puerta de entrada 1 hoja 100×215»: pon fondo de marco `400` → debe negarse («el marco no cabe en el espesor del muro»). Ancho `1100` → la puerta de la fachada oeste se ensancha sin moverse de su centro.
5. Pestaña **Materiales** → «Ladrillo perforado»: cámbiale el color con el selector y la trama a «Rayado cruzado». La planta cambia al cerrar el selector. Intenta **Borrar**: dice qué tipos lo usan. Duplica un material y bórralo.
6. Pestaña **Forjados** → cambia el canto del hormigón de la cubierta y mira en 3D (F3) que la cubierta cambia.
7. Guarda, cierra, abre de nuevo: los cambios siguen ahí. Exporta el IFC y ábrelo en ArchiCAD y Revit: tipos con el nombre nuevo, capas con los espesores nuevos y el material con su nombre.
8. Dime si echas en falta algo que uses a diario en ArchiCAD (por ejemplo, prioridades de intersección por material, o guardar tus multicapas para otros proyectos).
## 2026-10-17 · Fase 2, bloque 10 — Uniones en T y prioridades de intersección por capa

**Hecho**
- **Uniones en T** automáticas: un muro cuyo extremo cae dentro de otro (en su cara, en su eje o en cualquier punto de su espesor), lejos de sus esquinas y en ángulo de más de 15°, acomete a él. Da igual si lo dibujaste hasta la cara o hasta el eje: el resultado es el mismo.
- **Prioridad de intersección por capa**, como en ArchiCAD: cada capa del muro que acomete entra en el otro atravesando sus capas de menor prioridad, desde la cara por la que llega, y se detiene en la primera de prioridad igual o mayor. Prioridades por función de capa: **estructura 4 > soporte/hoja interior 3 > aislamiento 2 > cámara y lámina 1 > acabado 0**.
  - Tabique LHD contra fachada: el ladrillo hueco del tabique atraviesa el enlucido de la fachada y llega a su hoja interior de LHD; los enlucidos se detienen en la cara.
  - Muro de carga contra fachada: su ladrillo perforado atraviesa enlucido, hoja interior y aislamiento hasta la hoja exterior.
  - Tabique contra tabique: los dos tienen la misma prioridad, se detiene en la cara.
  - Nunca atraviesa la última capa del otro muro; si el muro que acomete es más alto que el otro o llega muy cerca de una esquina, simplemente se detiene en la cara.
- **Planta**: el muro que recibe pierde esa parte de sus capas; su línea de corte no se dibuja donde acomete el otro, y una línea fina marca dónde se detienen las capas que no entran. La hoja de LHD del tabique se ve continua con la de la fachada.
- **3D y IFC**: el muro que recibe pierde esa parte solo en la altura del que acomete (un tabique de 2,70 bajo una fachada de 3,00 no le quita nada por encima de 2,70). En 3D cada muro sigue siendo una sola piel cerrada; en el IFC el muro se exporta como una extrusión por tramo de altura, y su volumen neto coincide con el sólido (validado con IfcOpenShell). La importación de lo exportado sigue dando el mismo proyecto.
- **Vista 3D en equipos con OpenGL**: en tarjetas o controladores que no permiten ver una textura en otro formato (algunos OpenGL, máquinas virtuales) la pestaña 3D cerraba la aplicación. Ahora copia la imagen para mostrarla y funciona; la prueba de dibujo 3D vuelve a ejecutarse en esos equipos.
- Pruebas: geometría de la T (alcance por capa, pérdida exacta del muro que recibe, trozos de capa, muro más alto o junto a una esquina que solo topa, dibujado hasta el eje), sólidos 3D cerrados con volúmenes que cuadran (T a 90° y a 45°, tabiques más bajos que la fachada), demo exportada y validada con IfcOpenShell (volúmenes incluidos), ida y vuelta IFC. Planta de la demo revisada con zoom en las tres T; ventana real y vista 3D probadas con capturas.
- Versión 0.11.0. Sin cambios en el formato `.archisim`.

**Pendiente / limitaciones conocidas**
- Recorte de muros con forjados y cubiertas (siguiente parte del punto 2.3).
- Prioridades por función de capa, no editables por material (llegará con el editor de materiales, 1.6).
- Las esquinas en L siguen a inglete (sin envolver capas) y no hay cruces en X.
- Las T con muros en ángulo junto a huecos de puertas o ventanas del muro que recibe pueden dar geometría 3D imperfecta (caso raro).
- En 3D aparecen algunos puntos sueltos finos en la junta entre la coronación de una fachada y el canto del forjado de cubierta, donde el muro tiene aristas (jambas y ahora también las T) que el forjado no tiene. Es cosmético; se resolverá con el recorte muro–forjado.

**Antonio debe probar**
1. Descarga la nueva versión (GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**) y abre `demo.archisim`.
2. Haz zoom fuerte en planta en el encuentro del tabique central con la fachada sur, y en el del tabique este–oeste con la fachada este: el LHD del tabique debe llegar a la hoja interior de la fachada y los enlucidos detenerse. ¿Es así como lo dibujarías a 1:50 y 1:20?
3. Cambia el tipo de un tabique a «Muro de carga ladrillo perforado 1 pie 27» (Propiedades): su ladrillo debe atravesar hasta la hoja exterior de la fachada.
4. Dibuja un tabique hasta el **eje** de una fachada y otro hasta su **cara interior**: deben quedar iguales.
5. Exporta el IFC y ábrelo en ArchiCAD y Revit: los muros deben seguir entrando como muros con sus capas; mira en 3D o en sección el encuentro y las cantidades (volumen) de la fachada.
6. Si tu equipo tenía problemas con la pestaña 3D, vuelve a probarla.
7. Dime si las prioridades por función de capa te sirven o necesitas fijarlas por material (como ArchiCAD).

---

## 2026-10-17 · Cierre de fase 1, bloque 9 — Cotas, cotas de nivel y gestión de plantas

**Hecho**
- **Herramienta Cota** (tecla **C**), con tres métodos en el cuadro de información:
  - **Lineal**: clic en cada punto (con los forzados de siempre), Intro / doble clic / botón derecho, y clic donde va la línea. Dos puntos dan una cota simple; más, una **cadena**. Dirección automática (horizontal o vertical según dónde pongas la línea) o fija: horizontal, vertical o alineada. Vista previa en azul con el valor.
  - **Huecos de muro**: clic en un muro del lado de la cara a acotar → cadena con los extremos de esa cara (esquinas a inglete incluidas) y las jambas de todas sus puertas y ventanas; segundo clic para la línea.
  - **Cota de nivel**: clic → triángulo semirrelleno con la cota del suelo terminado respecto al ±0,00 (`+0,35`, `±0,00`, `-0,35`), leída del forjado más alto de la planta en ese punto. Si cambias el forjado o la cota de la planta, se actualiza sola.
- **Dibujo de las cotas** (planta exportada y pantalla, idénticas): líneas finas, trazo oblicuo a 45°, línea que sobrepasa 2 mm los trazos extremos, líneas auxiliares separadas 1,5 mm del elemento, valor encima de la línea, legible desde abajo o desde la derecha, en **metros con dos decimales** (`3,45`), texto de 2,5 mm.
- **Rotulación técnica propia**: los números se dibujan con trazos (estilo UNE-EN ISO 3098, diseño propio), no con una fuente del sistema: se ven igual en pantalla, en el SVG y se verán igual en PDF.
- Las cotas y cotas de nivel **se seleccionan** (clic sobre la línea o el símbolo), se borran, se copian (Ctrl) y se mueven: arrastradas solas, se desplaza solo la línea de cota; junto con muros, van con ellos. Propiedades: tramos, total, dirección; en la cota de nivel, el valor y la planta.
- La vivienda de prueba trae cotas de huecos y totales en las fachadas sur y oeste y dos cotas de nivel.
- **Plantas** (botón «Plantas…» junto al selector, o *Vista → Plantas…*): lista de arriba abajo con nombre y cota editables, altura hasta la planta de encima y contenido; **añadir planta encima** (con la altura de la última planta) o **debajo**; **borrar** (si tiene elementos, pide confirmación y los borra con ella; la última planta no se puede borrar). Cambiar la cota mueve todo su contenido. Todo se deshace con Ctrl+Z. Las plantas nuevas salen en el IFC como `IfcBuildingStorey`.
- **Formato `.archisim` versión 2** (cotas y cotas de nivel). Los proyectos de versión 1 se abren igual (prueba incluida) y se guardan ya en versión 2. De paso se corrige la lectura de decimales largos del fichero (p. ej. 3,9000000000000004), que podía cambiar en la última cifra al reabrir.
- Al encuadrar (Inicio) se tienen en cuenta las cotas.
- Pruebas: núcleo (cotas, cotas de nivel, plantas, ida y vuelta), fichero (versión 1 → 2), dibujo de cotas, herramienta de cota (cadena, dirección automática, huecos de muro por cada cara, cotas de nivel), arrastres de cotas y la app con clics simulados (acotar, seleccionar, mover la línea, borrar; añadir, renombrar, mover y borrar plantas con deshacer). Ventana real probada con capturas; planta de la demo revisada en SVG.
- Versión 0.10.0. Sin cambios en el IFC exportado.

**Pendiente / limitaciones conocidas**
- Las cotas **no son asociativas**: si mueves un muro, su cota no le sigue (fase 3, punto 3.2). Al moverlas junto con los muros, sí van con ellos.
- Sin estilos de cota por escala ni elección de unidades (m con 2 decimales fijo; decisión D5 pendiente), sin cotas de altura (sección) ni de superficie.
- Las cotas no se exportan al IFC (como Revit y ArchiCAD).
- No se ven otras plantas en gris debajo de la activa.

**Antonio debe probar**
1. Descarga la nueva versión (GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**) y abre `demo.archisim`: cotas en las fachadas sur y oeste y dos cotas de nivel. Mira de cerca el trazo oblicuo, los números y su posición: ¿es como acotas en el estudio?
2. Exporta la planta (SVG) y ábrela en el navegador con zoom: las cotas deben verse igual que en pantalla.
3. Tecla **C** → Lineal: acota el ancho de una habitación de cara a cara (los forzados «Sobre arista» ayudan), Intro, y coloca la línea dentro. Prueba una cadena de 4 puntos y la dirección Alineada.
4. Método **Huecos de muro**: clic por fuera de la fachada norte y otro clic más afuera para la línea. Repite por dentro: debe acotar la cara interior.
5. Método **Cota de nivel** sobre la solera; cambia la cota de la solera en sus Propiedades (p. ej. 50) y comprueba que la cota de nivel pasa a `+0,05`.
6. Selecciona una cota y arrástrala: se mueve solo la línea. Supr la borra; Ctrl+Z.
7. **Plantas…**: añade una planta encima, renómbrala, cámbiale la cota, dibuja algo en ella y bórrala (debe avisar). Exporta el IFC y comprueba en ArchiCAD/Revit que aparecen las plantas con su cota.
8. Abre un proyecto guardado con la versión anterior: debe abrirse sin problemas.
9. Dime si prefieres las cotas en centímetros o con otro número de decimales (es la decisión D5).

---

## 2026-10-17 · Cierre de fase 1, bloque 8 — Mover, copiar y estirar; varita mágica; vértices de forjado

**Hecho**
- **Mover con el ratón** (herramienta Flecha): se arrastra lo seleccionado, como en ArchiCAD (arrastrar en vacío sigue siendo ventana de selección). Mientras se arrastra se ve en azul cómo quedará, una línea desde el punto agarrado y la distancia; Mayús fuerza ángulos de 45°, se puede teclear la distancia (mm) + Intro, y Esc cancela. Se agarra por el punto especial más cercano (extremo, esquina, cara…) y se suelta con los mismos forzados que al dibujar, sin forzar a lo que se está moviendo.
  - Los muros unidos por sus extremos a uno que se mueve se **estiran** para que la esquina siga cerrada, y los tabiques que **acometen a la cara** de un muro movido se alargan o acortan por su eje para seguir contra esa cara (como Revit).
  - Las puertas y ventanas van con su muro; seleccionadas solas, **se deslizan a lo largo del muro**.
- **Copiar**: Ctrl al soltar. Copia muros con sus puertas y ventanas, forjados y huecos; quedan seleccionadas las copias (como Revit), con `GlobalId` nuevos.
- **Estirar**: los muros seleccionados muestran sus extremos; arrastrar uno mueve esa esquina y la de los muros unidos en ella. Con Ctrl, solo ese muro (sale de la esquina). Las puertas y ventanas se quedan en su sitio en planta (su distancia al inicio se recalcula).
- **Vértices de forjado** (punto 1.3 de la hoja de ruta): arrastrar una esquina la mueve; soltarla sobre la esquina vecina la quita; los rombos blancos del centro de cada arista desplazan la arista paralela a sí misma; con Ctrl, añaden una esquina.
- Lo que no cabe se rechaza entero con el motivo en la barra de estado (p. ej. mover un tabique de forma que otro tabique acometido se acorta y su puerta se saldría).
- **Varita mágica** para forjados (herramienta Forjado → Método: «Varita mágica», o **Espacio + clic** en modo polígono, como ArchiCAD): un clic dentro de un recinto de muros crea el forjado. Contorno: **cara interior** del recinto, **eje de muros** o **cara exterior** de todo el edificio (para la solera o la cubierta de la casa entera, con un solo clic en cualquier habitación). Vista previa del contorno y su superficie al pasar el cursor. Reconoce tabiques acometidos (T), ignora muros sueltos dentro del recinto y avisa si el clic cae en un muro o los muros no cierran.
- Pruebas: núcleo (estirar con huecos), geometría (recintos con tabiques, en L con encuentro en T, abiertos, con muro suelto, a eje y a cara exterior; los cuatro recintos de la vivienda de prueba), arrastres (mover, seguir esquinas y caras, copiar, estirar con y sin Ctrl, huecos que se deslizan o se salen, esquinas y aristas de forjado, vista previa) y la app con clics y teclas simulados (mover 1000 mm tecleados, copiar con Ctrl, estirar una esquina, cancelar con Esc, desplazar una arista de forjado, varita mágica en los tres contornos y con Espacio). La casa editada se exporta y valida con IfcOpenShell (`out/app/edicion.ifc`). Probada la ventana real con capturas (mover un tabique con sus acometidos, mover una fachada, varita mágica).
- La prueba de dibujo 3D se salta (con aviso) en adaptadores gráficos que no admiten vistas de textura en otro formato (algunos OpenGL por software), en vez de fallar.
- Versión 0.9.0. Sin cambios en el formato `.archisim` ni en el IFC exportado.

**Pendiente / limitaciones conocidas**
- Los forjados no siguen a los muros al moverlos (no hay asociación muro–forjado; ArchiCAD tampoco la tiene).
- La varita mágica no hace huecos en el forjado para muros sueltos ni patinillos (llegará con los huecos en forjados, 2.7).
- Sin «multiplicar» (copias en serie), girar ni simetría; sin arrastrar puertas y ventanas a otro muro.
- Soltar fuera de la planta cancela el arrastre.

**Antonio debe probar**
1. Descarga la nueva versión (GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**) y abre `demo.archisim`.
2. **Mover**: clic en el tabique vertical del centro; arrástralo 30 cm a la derecha y suelta. Los dos tabiques que acometen a él deben seguirle (uno se alarga, otro se acorta). Prueba a arrastrarlo más (1,3 m): debe negarse porque la puerta del tabique de la derecha se saldría. Ctrl+Z.
3. **Distancia exacta**: selecciona la fachada oeste, empieza a arrastrarla, mantén **Mayús**, teclea `1000` e Intro sin soltar el ratón. La casa se ensancha 1 m: fachadas norte y sur estiradas, esquinas a inglete cerradas.
4. **Copiar**: selecciona la puerta de entrada (fachada oeste), arrástrala 1,5 m a lo largo del muro con **Ctrl** pulsado al soltar: aparece una segunda puerta. Selecciona un tabique y cópialo con Ctrl: la copia queda seleccionada.
5. **Estirar**: selecciona la fachada este y arrastra su cuadrado negro de la esquina noreste hacia fuera: la fachada norte la acompaña. Repite con **Ctrl**: solo se mueve la fachada este.
6. **Forjado**: selecciona la solera; arrastra un rombo blanco de una arista hacia fuera (la arista se desplaza paralela), arrastra una esquina, añade una esquina con Ctrl en un rombo y quítala soltándola sobre la vecina.
7. **Varita mágica**: tecla **F**, Método «Varita mágica», pasa el cursor por las habitaciones (se ve el contorno y la superficie) y haz clic. Prueba los tres contornos: cara interior, eje y cara exterior (esta última da toda la casa). También en modo Polígono con **Espacio + clic**.
8. Exporta el IFC tras editar y ábrelo en ArchiCAD/Revit: los muros movidos y estirados conservan su `GlobalId` (compara con el IFC anterior), las copias tienen uno nuevo.
9. Cuéntame si el comportamiento al mover (tabiques que siguen a la cara, esquinas que se estiran) es el que esperas o prefieres el de ArchiCAD (la esquina se rompe al mover un solo muro).

---

## 2026-10-17 · Fase 1, bloque 7 — Importación IFC (MVP terminado) y hoja de ruta

**Hecho**
- **Importar IFC** (*Archivo → Importar IFC…*, o arrastrando un `.ifc` sobre `ArchiSim.exe`, o `archisim importar casa.ifc` en la línea de comandos). Lee **IFC2X3 e IFC4** (también IFC 4.3) de cualquier programa, en metros, milímetros, pies o pulgadas, con un lector STEP propio escrito desde cero.
  - **Plantas** con su nombre y cota.
  - **Muros** por su eje (o, si no lo tienen, por su colocación y su sólido), altura y cota de base desde el sólido, **capas** y **línea de referencia** desde el `IfcMaterialLayerSetUsage` (lado, orden y desfase). Las capas de espesor cero (membranas de Revit) se omiten con aviso. Los muros sin capas entran con una capa de su espesor.
  - **Lado exterior**: en IFC no existe; en los muros marcados como exteriores se toma como exterior el lado contrario al centro de los muros de la planta, y se giran para que la cara exterior quede a la izquierda, como en ArchiSim (un contorno dibujado al revés vuelve con las capas bien puestas).
  - **Forjados** por el contorno de su extrusión o por la cara superior de su sólido (también mallas trianguladas), con su cara superior como referencia, capas de arriba abajo y función (forjado, solera, cubierta).
  - **Puertas y ventanas** alojadas en muros: hueco de obra desde el `IfcOpeningElement`, posición sobre la línea de referencia, antepecho, lado hacia el que abren, mano UNE-EN 12519 (traducida desde la convención IFC), funcionamiento (abatible, dos hojas, corredera; ventana fija, practicable de una o dos hojas, corredera), marco desde las propiedades de cerco y materiales de marco y hoja/vidrio.
  - Se conservan **todos los `GlobalId`** (proyecto, plantas, tipos, muros, forjados, huecos), para poder comparar versiones con los ingenieros.
  - **Informe de importación** en una ventana: qué ha entrado y qué no (pilares, vigas, escaleras, espacios, cubiertas inclinadas, muros curvos, huecos sin carpintería…), con el motivo, y las aproximaciones hechas.
  - Si el fichero no trae tipos de muro, forjado, puerta o ventana, se añade la plantilla de ArchiSim para que todas las herramientas tengan con qué dibujar. El proyecto importado queda «sin guardar» con el nombre del IFC.
- **Ida y vuelta exacta**: lo que exporta ArchiSim se importa idéntico (muros, capas, materiales con color y trama, forjados, huecos con mano, retranqueo y marco, `GlobalId`). Para ello el IFC exportado lleva además las propiedades de cerco de puertas y ventanas (`IfcDoorLiningProperties`, `IfcWindowLiningProperties`), el funcionamiento de cada hoja de ventana (`IfcWindowPanelProperties`), la mano de cada hueco (`ArchiSim_Opening`) y la trama de cada material (`ArchiSim_Material`). Sigue validando con IfcOpenShell.
- Pruebas: ida y vuelta con un modelo que usa todas las opciones y con la vivienda de prueba; un fichero tipo Revit en IFC2X3 y milímetros escrito a mano (muros dibujados al revés, capas desde el exterior a la derecha, membrana sin espesor, puerta con `IfcDoorStyle`, forjado con capas de arriba abajo, un pilar); dos ficheros generados con **IfcOpenShell** (otro programa, no ArchiSim) en IFC4/metros e IFC2X3/milímetros, que también se incluyen para que los pruebes; y la importación desde la aplicación con el informe. Probado además con los ficheros de ejemplo de buildingSMART (SketchUp, IFC2X3, IFC4 e IFC 4.3). Ventana real probada con capturas (informe, planta y 3D).
- Barra de estado: los mensajes largos ya no se montan sobre la ayuda.
- **Hoja de ruta** después del MVP en `docs/HOJA_DE_RUTA.md`: decisiones pendientes, deudas de la fase 1 y fases 2, 3 y 4 con tamaños y dependencias.
- Versión 0.8.0. **MVP terminado**: los 10 puntos de la fase 1 están hechos.

**Pendiente / limitaciones conocidas**
- No se importan: muros curvos o de varios tramos, forjados inclinados (se avisan), huecos en forjados (se cierran, con aviso), huecos sin carpintería, puertas y ventanas en muros cortina, y ninguna otra clase (pilares, vigas, escaleras, cubiertas, mobiliario, espacios…). Todo queda contado en el informe.
- Los muros recortados por una cubierta entran rectos con su altura máxima (aviso).
- Las capas se ordenan por el lado exterior deducido; en muros interiores de otros programas el orden de las caras puede salir al revés (no cambia nada del dibujo, solo qué cara se llama «exterior»).
- Probado con ficheros sintéticos, de IfcOpenShell y de SketchUp; falta probar con exportaciones reales de ArchiCAD y Revit (punto D7 de la hoja de ruta).

**Antonio debe probar**
1. Descarga la nueva versión: GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**. Ahora trae una carpeta `ejemplos-ifc` con dos IFC escritos por IfcOpenShell.
2. Abre `ArchiSim.exe` → *Archivo → Importar IFC…* → `ejemplos-ifc/ifcopenshell_ifc2x3_mm.ifc`. Lee el informe: 5 muros, 1 forjado, 2 puertas, 1 ventana; un pilar y un hueco vacío sin importar. Mira la planta (F2) y el 3D (F3): las fachadas deben tener el monocapa por fuera aunque el fichero las dibuja al revés.
3. Selecciona un muro de fachada y mira sus Propiedades: tipo «Fachada 4 capas», línea de referencia en la cara exterior. Selecciona la puerta de entrada: mano y lado de apertura.
4. **Ida y vuelta**: abre `demo.archisim`, *Exportar IFC4…*, después *Importar IFC…* ese mismo fichero: debe salir la misma casa.
5. **Lo importante**: exporta un IFC de un proyecto tuyo sencillo desde **ArchiCAD** y otro desde **Revit** (IFC2X3 o IFC4, como lo hagas normalmente) e impórtalos. Mándame el informe (captura) y lo que veas mal: muros girados, capas en el lado equivocado, puertas con la mano cambiada, forjados que faltan. Los ficheros no se suben al repositorio.
6. Lee `docs/HOJA_DE_RUTA.md` y decide: el orden de los próximos bloques (propongo mover/estirar + varita mágica) y, cuando puedas, las decisiones D2 a D5 (lenguaje de familias, IFC4 o IFC 4.3, licencia y nombre, normas de dibujo).

---

## 2026-10-17 · Fase 1, bloque 6 — Vista 3D

**Hecho**
- **Vista 3D** del edificio entero (todas las plantas), en una pestaña junto a la planta: **F2** planta, **F3** 3D (como ArchiCAD), o con las pestañas «Planta · …» / «3D» encima del dibujo.
- **Lo que se ve es lo que recibe el IFC**: muros con sus capas y los huecos de puertas y ventanas, forjados con sus capas colgando de la cara superior, y carpinterías con marco, hojas y vidrio. Comprobado: el volumen de cada muro en 3D coincide exactamente con el `NetVolume` que exporta el IFC.
  - Colores de los materiales de corte, como en la exportación IFC; el vidrio, transparente.
  - Las capas se ven en los cantos de forjado, en la coronación de los muros y en las mochetas; las láminas de 2–3 mm aparecen como una línea fina.
  - Aristas de contorno en tinta (siluetas, cambios de plano y saltos de profundidad), como el sombreado con contornos de ArchiCAD.
  - Cada muro y cada forjado es una sola piel cerrada: las capas y los tramos que se tocan dentro del elemento no tienen caras, y los muros unidos a inglete se cierran el uno al otro. Así no hay parpadeos ni puntos sueltos en las aristas.
- **Navegación** (como Revit y ArchiCAD): arrastrar con el botón izquierdo (o Mayús + rueda) para girar alrededor del modelo; arrastrar con la rueda o el botón derecho para desplazar; rueda para acercar donde está el cursor; doble clic en la rueda o **Inicio** para ajustar. **Perspectiva** o **axonometría** (proyección paralela). Flecha de norte en la esquina, que gira con la vista.
- **Selección en 3D**: clic sobre un elemento (Mayús+clic para añadir o quitar). Lo seleccionado se tiñe de azul y sus propiedades aparecen a la derecha, igual que en planta; Supr lo borra. Las teclas de herramienta (M, F, P, V) vuelven a la planta para dibujar.
- **Guardar imagen 3D** en PNG (botón «Guardar imagen…» o *Archivo → Guardar imagen 3D*), a doble resolución de la pantalla.
- Calidad de imagen: supermuestreo ×2 y multimuestreo ×4. Se dibuja solo cuando algo cambia (cámara, modelo, selección o tamaño).
- Pruebas: cámara (encuadre, zoom en el cursor, rayos), selección por rayo, geometría (pieles cerradas, volúmenes, esquinas a inglete cerradas), y un dibujo real con la tarjeta gráfica (o una por software) que se comprueba píxel a píxel; en la integración continua se guarda como `vista3d.png`. Con clics simulados: F3, seleccionar un muro en 3D, girar, borrar y volver a planta. Probada la ventana real con capturas.
- No cambia nada de lo exportado: IFC, plantas y `.archisim` siguen igual (validados con IfcOpenShell; ida y vuelta idéntica).
- Versión 0.7.0.

**Pendiente / limitaciones conocidas**
- Sin terreno, sombras proyectadas ni texturas; sin secciones 3D ni recorte por plantas (se ve todo el edificio).
- Las puertas se ven cerradas; las vueltas de mocheta y el premarco son de dibujo de planta y no aparecen en 3D (tampoco en el IFC).
- Sin dibujar en 3D: las herramientas trabajan en planta.
- En equipos sin tarjeta gráfica compatible (Vulkan, DirectX 12 o Metal) la pestaña 3D lo indica con un aviso.

**Antonio debe probar**
1. Descarga la nueva versión: GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**. Arrastra `demo.archisim` sobre `ArchiSim.exe`.
2. Pulsa **F3**. Gira con el botón izquierdo, desplaza con el derecho o la rueda pulsada, acerca con la rueda. Inicio para volver a encuadrar. ¿Se mueve con fluidez en tu equipo?
3. Mira con zoom: esquinas a inglete, mochetas de puertas y ventanas (capas en la mocheta), cantos de la solera y de la cubierta con sus capas, vidrio transparente. ¿Los colores y las aristas se leen bien como maqueta?
4. Cambia a **Axonometría** y compara con una axonometría tuya.
5. Haz clic en una ventana en 3D, cambia su antepecho en Propiedades y comprueba que se mueve en 3D y en planta (F2). Prueba Supr y Ctrl+Z desde la vista 3D.
6. Dibuja algo en planta (M, F, P, V) y vuelve a F3: aparece al momento.
7. «Guardar imagen…» y abre el PNG.
8. Abre `demo.ifc` en ArchiCAD o Revit y compara en 3D con lo que ves en ArchiSim: deberían coincidir.
9. ¿Qué sigue: importar IFC (último punto del MVP), varita mágica para forjados, mover/estirar con el ratón o cotas?

---

## 2026-10-17 · Fase 1, bloque 5 — Herramientas de forjado, puerta y ventana

**Hecho**
- **Forjado** (tecla F): contorno por clics, como el forjado poligonal de ArchiCAD. Cada clic añade una esquina con los mismos forzados que el muro (extremos y esquinas de muros, caras, puntos medios, alineación a 0/45/90°, Mayús para ángulo forzado) y longitud tecleada + Intro. Se cierra con clic en la primera esquina o con Intro; Esc quita la última esquina. Vista previa con el contorno, el lado que cerrará en discontinua, longitud, ángulo y superficie. Se rechazan contornos que se cruzan.
  - Ajustes en el cuadro de información: tipo (con su canto) y cota de la cara superior sobre la planta. Por defecto, la solera.
- **Puerta** (tecla P) y **Ventana** (tecla V): un clic sobre el muro. Se colocan centradas en el cursor (al centímetro, sin salirse del muro) y abren hacia el lado del muro donde está el cursor. Vista previa con el hueco, el marco y, en las puertas, la hoja abierta y su giro; etiqueta con la distancia libre a cada lado y hacia dónde abre. Si no cabe (se solapa, se sale, el marco no entra) se ve en rojo con el motivo y no se coloca.
  - Ajustes: tipo, mano (UNE-EN 12519), antepecho (puertas 0, ventanas 900 mm por defecto) y retranqueo del marco desde la cara exterior (vacío = centrado).
- Cada colocación es un paso de deshacer («Crear forjado», «Colocar puerta», «Colocar ventana»), y lo colocado se edita después en Propiedades como siempre.
- Pruebas: con clics simulados se dibuja la casa, su solera por las cuatro esquinas, una puerta y una ventana; un segundo hueco encima se rechaza. La vista previa coincide exactamente con lo que se dibuja al colocar. El IFC de esa sesión (muros, solera, puerta y ventana) se valida con IfcOpenShell en la integración continua. Probada la ventana real con capturas.
- Versión 0.6.0.

**Pendiente / limitaciones conocidas**
- Forjado por «varita mágica» (clic dentro de un recinto de muros) y edición de vértices de un forjado existente.
- Mover, copiar y estirar con el ratón; acotación.
- La vista previa de puertas correderas y ventanas es esquemática (hueco, marco y vidrio); al colocarlas se dibujan completas.

**Decisión de Antonio (17/10/2026):** al cambiar la línea de referencia de un muro se mantiene el comportamiento actual (la línea se queda en su sitio y el muro se desplaza, para no romper las esquinas).

**Antonio debe probar**
1. Descarga la última versión: GitHub → pestaña **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**. Descomprime y abre `ArchiSim.exe` (o arrastra `demo.archisim` encima).
2. **Forjado**: Archivo → Nuevo, dibuja un contorno de muros (tecla M). Tecla **F** y clic en las cuatro esquinas exteriores (aparece el cuadrado verde de «Extremo»); clic otra vez en la primera para cerrar. Prueba también un forjado en L tecleando longitudes, y Esc para quitar una esquina mal puesta.
3. **Puerta**: tecla **P**, pasa el cursor por un muro de un lado y del otro: ¿la hoja abre hacia donde está el cursor? Cambia la mano arriba y comprueba que sigue UNE-EN 12519. Clic para colocarla.
4. **Ventana**: tecla **V**, antepecho 900; prueba a ponerla encima de la puerta (sale en rojo y explica por qué). Prueba un retranqueo de `130` en la fachada.
5. Selecciona lo colocado y cámbialo en Propiedades; Ctrl+Z.
6. Exporta el IFC y ábrelo en ArchiCAD y Revit: solera como forjado/suelo, puertas y ventanas en su hueco.
7. ¿Qué prefieres después: varita mágica para forjados, mover/estirar con el ratón, o la vista 3D?

---

## 2026-10-17 · Fase 1, bloque 4 — Aplicación: ventana, vista de planta, herramienta muro y selección

**Hecho**
- Primera ventana de ArchiSim (`archisim-app`, egui sobre wgpu), con la disposición de ArchiCAD: caja de herramientas a la izquierda (Flecha, Muro), cuadro de información arriba con los ajustes de la herramienta activa y el selector de planta, paleta de **Propiedades** a la derecha (como Revit) y barra de estado con coordenadas, punto de forzado y mensajes. Todo en español y en milímetros (acepta también `2,7m` o `270cm`).
- **Vista de planta = la planta 1:50 que se exporta**: se dibuja el mismo SVG de la zona visible a resolución de pantalla, con grosores de línea reales a 1:50 (como el «grosor de línea real» de ArchiCAD). Lo que ves es lo que exportas: poché por capa, mochetas, puertas y ventanas, solera. Rueda para zoom en el cursor, arrastrar con la rueda o con el botón derecho para encuadre, doble clic en la rueda o tecla Inicio para ajustar. Retícula de 1 m y 5 m de fondo.
- **Herramienta muro** (tecla M): clic de inicio y clic de final, encadenando como el muro poligonal de ArchiCAD; los muros encadenados se unen a inglete. Clic en el primer punto cierra el contorno. Mientras dibujas, escribes la longitud en mm y pulsas Intro (va en la dirección del cursor). Vista previa con las capas del tipo, la línea de referencia y la marca «ext.» en el lado exterior; etiqueta con longitud y ángulo.
  - Forzados: extremos de muro, esquinas de muros y forjados, puntos medios y caras de muro o bordes de forjado (para acometer tabiques a la cara interior de la fachada). Alineación automática a horizontal, vertical y 45° cuando el cursor está cerca; con Mayús, ángulo forzado. Fuera de forzados, las coordenadas se redondean al centímetro.
  - Ajustes en el cuadro de información: tipo, línea de referencia (al elegir un tipo de fachada pasa a «cara exterior»; uno interior, a «eje»), altura y desfase de base.
- **Selección**: clic (prioridad puerta/ventana → muro → forjado), Mayús+clic para añadir o quitar, ventana de selección arrastrando (elementos enteros dentro), Ctrl+A, Esc. Resaltado al pasar por encima; seleccionados en azul con puntos de control y la línea de referencia en rojo discontinuo.
- **Propiedades editables** (cada cambio es un paso de deshacer, se confirma con Intro o al salir del campo; Esc lo anula):
  - Muro: tipo, línea de referencia, longitud (desde el inicio), altura, desfase de base, invertir; espesor, planta, inicio, fin, nº de huecos, capas y GlobalId.
  - Puerta/ventana: tipo (solo del mismo tipo de elemento), posición, antepecho, lado de apertura y mano (UNE-EN 12519); hueco de obra, funcionamiento, muro anfitrión y GlobalId.
  - Forjado: tipo y cota de la cara superior; canto, superficie, vértices y GlobalId.
  - Varios muros: tipo y altura para todos, invertir. Cualquier selección: borrar.
  - Sin selección: nombre del proyecto y resumen.
- Deshacer/rehacer (Ctrl+Z / Ctrl+Y) con el nombre de cada paso en el menú; Supr borra (un muro se lleva sus huecos).
- **Archivos**: Nuevo (plantilla), Abrir, Guardar, Guardar como (`.archisim`), Exportar IFC4 y Exportar planta 1:50 (SVG). Aviso de cambios sin guardar al crear, abrir o cerrar. `archisim-app proyecto.archisim` abre directamente un proyecto.
- **Plantilla de inicio** (`archisim-library`): materiales y tipos de construcción española habitual — fachada doble hoja 26,5, tabique LHD 10, muro de carga de 1 pie 27; solera, forjado de planta HA 30, cubierta plana invertida; puertas de entrada, de paso de 1 y 2 hojas, corredera, y cuatro ventanas. La vivienda de prueba se construye ahora sobre la misma plantilla.
- Núcleo: cambiar tipo, altura, desfase y línea de referencia de un muro, tipo de puerta o ventana y tipo y cota de forjado, conservando el `GlobalId` y rechazando lo que deja huecos fuera del muro o marcos que no caben.
- Planta: el rayado y el zigzag del aislamiento son ahora geometría recortada de verdad a cada capa, sin máscaras SVG. La planta exportada es igual píxel a píxel salvo un detalle a mejor (desaparece una línea de rayado que caía justo sobre el inglete y se veía como una junta), el SVG pesa la mitad y en pantalla se dibuja 7 a 80 veces más rápido (9 ms por vista en vez de hasta 740 ms con mucho zoom).
- Pruebas: la aplicación se maneja sin pantalla con clics y teclas simulados (dibujar la casa con longitudes tecleadas, acometer un tabique a las caras, seleccionar, borrar, deshacer, ventana de selección, zoom, aviso de cambios). La casa dibujada así se guarda, se reabre y su IFC se valida con IfcOpenShell (también en la integración continua). Además se ha probado la ventana real en una pantalla virtual con capturas.
- Versión 0.5.0.

**Pendiente / limitaciones conocidas**
- Herramientas de forjado, puerta y ventana en la app (los forjados y huecos existentes se seleccionan y editan; aún no se dibujan nuevos).
- Mover, copiar y estirar con el ratón (por ahora longitud y posiciones se cambian en Propiedades). Puntos de control arrastrables como en ArchiCAD.
- Encuentros en T siguen cortados a escuadra (fase 2); un tabique acometido a la cara de la fachada no se une a ella.
- La vista no muestra todavía otras plantas en gris ni cotas; no hay crear/borrar plantas.
- El aspecto (claro u oscuro) sigue al del sistema.

**Validación de Antonio (17/10/2026):** probada en su Windows con la descarga de GitHub (*Actions → Artifacts → ArchiSim-Windows*): la demo funciona como se esperaba. Integrado en `main` (PR #1).

**Antonio debe probar**
1. Abrir la app con la vivienda de prueba:
   ```
   cargo run -p archisim-cli -- demo out
   cargo run --release -p archisim-app -- out/demo.archisim
   ```
   (la primera compilación en `--release` tarda varios minutos). Juzga la planta en pantalla: ¿se lee como tu planta 1:50? Haz zoom fuerte en una esquina y en una mocheta.
2. Navegación: rueda (zoom en el cursor), arrastrar con la rueda o el botón derecho, Inicio para ajustar. En un portátil Mac: dos dedos = zoom; clic con dos dedos y arrastrar = encuadre. ¿Se mueve con fluidez?
3. Dibujar: Archivo → Nuevo, tecla **M**. Clic en un punto; mueve el cursor hacia arriba, escribe `8000` e Intro; hacia la derecha `10000` Intro; hacia abajo `8000` Intro; clic en el primer punto para cerrar. Esquinas a inglete y cara exterior hacia fuera (dibujando en sentido horario).
4. Tabique: en el cuadro de información elige «Tabique LHD 10» (pasa solo a «Eje»), clic cerca de la cara interior de la fachada sur (aparece una ✕ verde: «Sobre arista») y otro en la cara norte. Esc dos veces.
5. Seleccionar y editar: clic en un muro, cambia en Propiedades la altura a `2700` + Intro, el tipo, la línea de referencia; «Invertir». Selecciona una puerta de la demo y cambia su mano o el lado de apertura: ¿la planta responde como esperas? Supr, Ctrl+Z, Ctrl+Y.
6. Guardar como `prueba.archisim`, cerrar, volver a abrir; Exportar IFC4 y abrirlo en ArchiCAD y Revit: muros con su tipo, capas y esquinas, como siempre.
7. Dime qué echas en falta primero en la app (forjado, puertas y ventanas, mover/estirar, cotas…) y si la línea de referencia debe quedarse en su sitio al cambiarla (ahora se queda la línea y se desplaza el muro, para no romper las esquinas) o al revés.

---

## 2026-10-17 · Fase 1, bloque 3 — Formato de proyecto `.archisim`

**Hecho**
- Formato de proyecto propio `.archisim`: un único fichero JSON legible, con cabecera (`format`, `version: 1`, `units: "m"`, programa que lo guardó), datos del proyecto y todos los elementos: niveles, materiales, tipos de muro y de forjado con sus capas, muros, forjados, tipos de puerta y ventana y sus ejemplares. Descrito para ti en `docs/FORMATO_ARCHISIM.md`.
- Identidad estable: cada elemento conserva su `id` y su `GlobalId`. Comprobado que **guardar → abrir → exportar da el mismo IFC** byte a byte (salvo la fecha de la cabecera) y las mismas plantas SVG; además, guardar dos veces el mismo proyecto da exactamente el mismo fichero (bueno para comparar versiones).
- Al abrir se comprueba todo como si se volviera a dibujar: referencias que existen, `id` y `GlobalId` sin repetir, números válidos, contornos sin cruces, puertas y ventanas dentro de su muro y sin solaparse, funcionamiento coherente (una puerta no puede ser «fija»). Si falla, no se abre a medias: dice qué elemento y por qué; los campos mal escritos se señalan con línea y columna.
- Versionado: un fichero de un ArchiSim más nuevo se rechaza con aviso; hay sitio preparado para actualizar ficheros antiguos cuando cambie el formato.
- Guardado seguro: se escribe un temporal y se sustituye al final; un fallo al guardar no estropea el proyecto anterior.
- De paso, el núcleo rechaza ahora valores no numéricos (NaN, infinito) en cotas, alturas y posiciones, que antes se colaban.
- Línea de comandos: `demo` guarda también `demo.archisim`; nueva orden `exportar <proyecto.archisim> [carpeta]`. La integración continua comprueba la ida y vuelta y valida los dos IFC con IfcOpenShell (ambos válidos).
- Versión del programa: 0.4.0.

**Pendiente / limitaciones conocidas**
- No se guarda el historial de deshacer (como ArchiCAD y Revit).
- Las unidades del fichero son siempre metros; la interfaz mostrará milímetros.
- Todavía no hay propiedades de usuario (parámetros compartidos / `ArchiSim_` Psets) que guardar: cuando existan, se añaden al formato con su migración.
- Siguiente bloque del MVP: ventana de la aplicación (vista de planta, herramienta muro, selección), que abrirá y guardará `.archisim`.

**Antonio debe probar**
- `cargo run -p archisim-cli -- demo out` → ahora aparece `out/demo.archisim`. Ábrelo con un editor de texto (Bloc de notas, VS Code) y juzga si se entiende: niveles, tipos con sus capas, muros, puertas y ventanas.
- Edición a mano y reexportación:
  1. En `out/demo.archisim` cambia `"name": "Vivienda de prueba"` por otro nombre, y en algún muro `"height": 3.0` por `2.7`. Guarda.
  2. `cargo run -p archisim-cli -- exportar out/demo.archisim`
  3. Abre `out/demo.ifc` en ArchiCAD o Revit: nombre del proyecto y altura del muro cambiados; el resto, igual.
- Identidad estable: ejecuta la demo, abre `out/demo.ifc` en ArchiCAD/Revit y anota el GUID de un muro; ejecuta `exportar` y vuelve a abrirlo (o compáralo con el anterior en Solibri/BIMcollab Zoom): el GUID debe ser el mismo.
- Errores: escribe mal un campo (`"heigth"`) o pon una ventana encima de una puerta (`"along"` igual al de la puerta) y ejecuta `exportar`: debe decirte qué está mal y dónde, sin generar nada.

---

## 2026-10-17 · Fase 1, bloque 2b — Mochetas y giro de ventanas (feedback de Antonio)

**Hecho**
- Mochetas en fachadas de doble hoja: la hoja exterior (ladrillo perforado) vuelve en cada jamba y cierra la cámara hasta la hoja interior; el aislamiento se detiene un espesor de ladrillo (11,5 cm) antes de la jamba. Líneas finas solo donde cambia el material (vuelta contra aislamiento y contra la hoja interior); la vuelta y la hoja exterior se leen como una pieza continua.
- El revestimiento exterior (monocapa) envuelve la mocheta desde la cara de fachada hasta la carpintería. A 1:50 sus 1,5 cm quedan bajo la línea de corte, como corresponde a la escala.
- La regla es automática según el tipo de muro: vuelve la primera hoja (estructura o soporte) seguida de una cámara (aislamiento, cámara de aire o lámina) con algo detrás; envuelve el acabado exterior si el muro es de fachada. Tabiques y muros sin cámara se cortan rectos.
- Ventanas practicables: giro de cada hoja en planta (línea fina a 90° y arco) hacia el lado de apertura; en las de dos hojas, cada hoja llega al eje del montante.
- Correcciones de dibujo: el vierteaguas ya no se mete bajo el muro cortado (solo sobresale de las jambas por fuera de la fachada) y las líneas de corte se dibujan las últimas, para que marcos y vierteaguas no tapen media mocheta.

**Pendiente / limitaciones conocidas**
- Sin premarco ni vuelta del enlucido interior; sin dintel ni capialzado en planta ni en IFC.
- Las vueltas de mocheta son de dibujo: en IFC el muro sigue siendo un sólido con su conjunto de capas (`IfcMaterialLayerSetUsage`), como lo exportan Revit y ArchiCAD.
- Siguiente bloque del MVP: formato de proyecto propio (`.archisim`).

**Antonio debe probar**
- `cargo run -p archisim-cli -- demo out` y abrir `out/demo.svg` con mucho zoom en cualquier jamba de fachada:
  - ¿La vuelta del ladrillo y el corte del aislamiento son como los dibujarías? ¿Ancho de vuelta = espesor del ladrillo te vale, o prefieres otro (p. ej. medio pie fijo)?
  - ¿La carpintería a 13 cm apoya bien contra la vuelta?
  - Giro de ventanas practicables (salón sur, cocina norte, dormitorio sureste): ¿sentido y mano correctos?

---

## 2026-10-17 · Fase 1, bloque 2 — Puertas y ventanas

**Hecho**
- Tipos de puerta (abatible 1 hoja, abatible 2 hojas, corredera) y de ventana (fija, practicable 1 hoja, practicable 2 hojas, corredera 2 hojas) con hueco de obra, perfil de marco (anchura vista y fondo) y materiales de marco y de hoja o vidrio.
- Puertas y ventanas alojadas en muro: posición del centro sobre la línea de referencia, antepecho desde el nivel, lado de apertura, mano (UNE-EN 12519) y retranqueo de la carpintería desde la cara exterior (o centrada). Se rechazan si se salen del muro, se solapan con otro hueco o el marco no cabe en el espesor. Borrar un muro borra sus huecos; acortarlo dejando un hueco fuera se rechaza; invertirlo los deja en su sitio. Mover una ventana conserva su `GlobalId`.
- Planta: el muro y todas sus capas se interrumpen en los huecos cortados, con mochetas en línea de corte. Puertas con cerco, hoja abierta a 90° y arco; correderas con hoja sobre la cara y flecha. Ventanas con marco, vidrio, montante central (dos hojas), hojas desfasadas (corredera), vierteaguas al exterior y borde del antepecho al interior. La ventana alta por encima del plano de corte (1,10 m) sale discontinua sin cortar el muro.
- IFC4: `IfcOpeningElement` que vacía el muro, `IfcDoor` / `IfcWindow` que lo rellenan con su tipo (`IfcDoorType` / `IfcWindowType`), `IfcMaterialConstituentSet` (marco + hoja o vidrio), geometría 3D sencilla con el vidrio transparente, `Pset_DoorCommon` / `Pset_WindowCommon` (`Reference`, `IsExternal`) y cantidades de ancho, alto y superficie. Los muros añaden `NetSideArea` y `NetVolume`, exactos incluso con huecos junto a una esquina a inglete.
- Vivienda de prueba con 4 puertas y 5 ventanas: entrada de 100×215 en fachada oeste, tres puertas de paso de 80×205, balconera corredera de 240×220, ventanas practicables de 1 y 2 hojas y una ventana fija alta (antepecho 1,60) por encima del plano de corte.
- Validador IFC ampliado: cada hueco vacía un muro y tiene un único relleno; cada puerta y ventana tiene tipo, materiales, Pset, cantidades, geometría 3D y planta; el volumen 3D de cada muro (ya perforado) coincide con su volumen neto. Pasa sin errores con reglas EXPRESS activas.

**Pendiente / limitaciones conocidas**
- Mochetas: el muro se corta recto en todo su espesor; no se dibujan la vuelta de la hoja exterior, el premarco ni el cierre de la cámara.
- Sin dinteles, persianas, capialzados ni huecos sin carpintería (paso libre); sin huecos en forjado.
- Ventanas practicables: en planta no se dibuja el giro de las hojas; en IFC no se exporta todavía la operación de cada hoja (`IfcWindowPanelProperties`).
- Mano de puertas en IFC: `OperationType` sigue la convención IFC (vista desde el lado contrario al de apertura), por eso izquierda y derecha se invierten al exportar. Está razonado pero sin contrastar con Revit/ArchiCAD.
- La posición se comprueba sobre la línea de referencia: un hueco pegado a una esquina a inglete puede invadir el triángulo de la esquina por la cara interior.
- Siguiente bloque del MVP: formato de proyecto propio (`.archisim`).

**Antonio debe probar**
- `cargo run -p archisim-cli -- demo out` y abrir `out/demo.svg` en el navegador (con zoom). Juzgar como planta 1:50:
  - Mochetas, cerco y hoja de la entrada (fachada oeste, abre hacia dentro, bisagras al norte).
  - Puertas de paso: ¿abren hacia donde esperas y con la mano correcta?
  - Ventanas: marco retranqueado 13 cm (detrás del ladrillo perforado), vierteaguas, línea de antepecho interior y montante de la de dos hojas.
  - Balconera corredera (fachada norte, salón) y ventana fija alta discontinua (fachada este).
  - ¿Qué nivel de detalle quieres en mochetas y en el giro de hojas de ventana?
- Abrir `out/demo.ifc` en ArchiCAD y Revit y comprobar:
  - Las puertas y ventanas entran como puertas y ventanas (no objetos genéricos), en su hueco y con el muro perforado en 3D.
  - Tipos con sus nombres («Puerta de entrada 1 hoja 100×215», «Ventana practicable 2 hojas 180×130»…), materiales (madera, aluminio, vidrio) y vidrio transparente.
  - Ancho, alto y antepecho correctos; propiedades `IsExternal` (entrada y ventanas sí, puertas de paso no).
  - Mano de las puertas en las propiedades (`OperationType`): ¿coincide con lo que ves? Si sale al revés, dímelo y se corrige.
  - Cantidades del muro: superficie y volumen netos descontando huecos.

---

## 2026-10-17 · Fase 1, bloque 1 — Forjados

**Hecho**
- Tipos de forjado multicapa (capas de arriba abajo) con función: forjado de planta, solera o cubierta plana.
- Forjado por contorno poligonal (cualquier forma sin cruces), con nivel y desfase de la cara superior. Se rechazan contornos que se cruzan, con menos de tres puntos o sin superficie. Mover el contorno conserva el `GlobalId`. Un tipo en uso no se puede borrar.
- Exportación IFC4: `IfcSlabType` (`FLOOR` / `BASESLAB` / `ROOF`), `IfcSlab` extruido desde la cara inferior, `IfcMaterialLayerSetUsage` en vertical (`AXIS3`, capas de abajo arriba), `Pset_SlabCommon` (`IsExternal`, `LoadBearing`, `Reference`) y `Qto_SlabBaseQuantities` (canto, perímetro, superficie y volumen).
- Planta: los forjados se dibujan según el plano de corte (1,10 m): por debajo, trama de cubrición clara y línea fina; cortados, poché de la capa estructural; por encima, discontinuos.
- Vivienda de prueba: solera de 34,7 cm bajo cota 0 y cubierta plana invertida con grava de 54,8 cm apoyada sobre los muros (cara inferior a +3,00). El nivel «Cubierta» pasa a +3,548 (cara superior de la cubierta). Se genera una planta SVG por nivel (`demo.svg`, `demo-cubierta.svg`).
- Validador IFC ampliado: comprueba tipo, capas, Psets, cantidades, volumen 3D, dirección vertical de capas y orden de abajo arriba en cada forjado. Pasa sin errores.

**Pendiente / limitaciones conocidas**
- Huecos en forjado (escaleras, patinillos) → cuando se haga el hueco genérico (bloque 2).
- Recorte muro–forjado: la solera y la cubierta ocupan toda la huella, también bajo y sobre los muros; no hay intersecciones porque los muros acaban justo en la cara del forjado, pero no hay prioridades de intersección → fase 2.
- Forjados inclinados y cubiertas con pendiente → fase 2 (cubiertas).
- Siguiente bloque del MVP: huecos de puertas y ventanas.

**Antonio debe probar**
- Abrir `out/demo.ifc` en ArchiCAD y Revit y comprobar:
  - Solera y cubierta entran como **forjados** (ArchiCAD) / **suelos** y **cubierta o suelo** (Revit), no como objetos genéricos.
  - Tipos «Solera HA 15 con encachado» y «Cubierta plana invertida con grava».
  - En sección: capas en orden, con la baldosa / grava arriba y el encachado / enlucido abajo.
  - La cubierta apoya sobre los muros (cara inferior a +3,00) y la solera queda bajo cota 0 (cara inferior a −0,347).
  - Propiedades `IsExternal` y `LoadBearing`, y cantidades de superficie (108 m²) y volumen.
- Abrir `out/demo.svg` y `out/demo-cubierta.svg` en el navegador y juzgar si la representación de la solera bajo los muros y del forjado de cubierta es la que esperarías en una planta 1:50.

---

## 2026-10-17 · Bloque 0 — Fundación

**Hecho**
- Documento fundacional (`docs/FUNDACIONAL.md`) e instrucciones para Claude (`CLAUDE.md`).
- Workspace Rust con cinco crates: core, geom, ifc, plan, cli.
- Núcleo: niveles, materiales, tipos de muro multicapa, muros, deshacer/rehacer.
- Uniones de muros en esquina (L) a inglete, capa por capa.
- Exportación IFC4: tipos, capas (`IfcMaterialLayerSetUsage`), `Pset_WallCommon`, `Qto_WallBaseQuantities`. Validado con IfcOpenShell (esquema, reglas EXPRESS y volumen de cada muro).
- Planta SVG 1:50 con relleno por material, rayado, zigzag de aislamiento y gruesos de línea.
- Integración continua en GitHub (Windows, macOS, Linux) y validación IFC.

**Pendiente / limitaciones conocidas**
- Encuentros en T a tope (línea de fachada continua) → fase 2.
- Siguiente bloque del MVP: forjados.

**Antonio debe probar**
- Abrir `demo.ifc` en ArchiCAD y Revit: muros reconocidos, tipos, orden de capas (monocapa fuera, yeso dentro), esquinas limpias, propiedades y cantidades.
