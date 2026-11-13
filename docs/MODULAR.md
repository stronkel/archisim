# Vivienda modular por cajas (steel frame)

> 9 de noviembre de 2026 · Propuesta para Antonio, **sin programar todavía**. Sale de su pregunta sobre qué le falta a ArchiSim para proyectar viviendas modulares de un fabricante con el que trabaja AD5, y del IFC de uno de sus modelos (exportado de Revit; no se sube al repositorio).

## 1. El sistema

- **Cajas completas** hechas en taller, que van en camión y se colocan en obra.
- Módulos de **hasta 9 × 3,60 m**. Los habituales miden **6,20 × 3,60 m**.
- Estructura de acero con perfiles de catálogo de fabricante (en ArchiSim, como fichero del estudio: `docs/ENTRAMADOS.md` §3 ter) y soldadura de taller.
- Las cajas apoyan sobre pilares cortos de hormigón, encima de una losa, con zócalo perimetral de bloque (forjado sanitario).

## 2. Qué dice el modelo de ejemplo (IFC de Revit, IFC2X3)

**La vivienda.** Planta en L con 8 cajas:
- 6 en fila y 2 en el ala;
- paso de 3,60 m entre ejes de caja;
- unos 6,1 m de largo de suelo.

**Cómo está modelado:**
- **Cada capa del cerramiento es un muro distinto**: placa cerámica de 10 mm, membrana, placa con aislamiento de 51 mm, lana de 100 mm y placa interior de 2 × 12,5 mm. Son 195 «muros» de placa cerámica.
- **Montantes:** solo 70, colocados a mano como pilares de 40 × 100 mm. Llevan un nombre de pilar genérico y material «aluminio».
- **Lo que no está dibujado:**
  - canales;
  - chasis de suelo y de techo;
  - estructura de suelo y de cubierta de las cajas (solo los acabados y el remate de los petos).
- **El IFC no dice qué pertenece a cada caja:** no hay agrupaciones.
- **Tampoco hay locales** (`IfcSpace`).
- **Cimentación:**
  - losa de 45 cm;
  - 27 pilares de hormigón de 25 × 25 × 60 cm;
  - zócalo de bloque de 60 cm.
- **Exterior:** un porche con forjado de hormigón de 30 cm, y piscina y terraza.

**Lo que hace ArchiSim hoy con ese IFC** (`archisim importar`):
- Lo abre en menos de un segundo. Entran 508 muros, 36 forjados, 28 carpinterías, 97 pilares y los ejes.
- La comprobación CTE da 204 «no cumple». Casi todos son las capas sueltas tomadas por muros de fábrica de 10 mm que cargan. No es un fallo del proyecto: el modelo está pensado para dibujar, no para calcular.
- Sin locales no hay ocupación, ventilación ni HE 1.
- Los ejes llegan cuatro veces (uno por planta en el fichero). Hay que juntarlos al importar.

## 3. Lo que hay que añadir

**M1 · La caja como elemento.** ✅ Hecho el 9/11/2026, rama `bloque/caja`.
- **La caja.** Un rectángulo en planta con su largo, ancho y alto, la base del chasis bajo el suelo, su marca («MOD-03») y su tipo (texto hasta M8).
- **Sus lados.** A y C son los largos; B y D, los testeros. Cada lado está libre (fachada y pretil), unido con el muro de la junta, o unido y abierto.
- **Lo que lleva.** Lo que está dentro, o en un lado que no está abierto. No se guarda en una lista: se deduce de dónde está, así que mover un muro dentro o fuera no pide nada más.
- **Avisos.** Si se pasa de lo que fabrica el sistema (9 × 3,60 m), si pide transporte especial (más de 2,55 m de ancho) o si sobre el camión pasa de 4 m. También si una junta no tiene dueño o lleva dos muros.
- **Colocarla.** Un clic pone una caja; cerca de otra se pega a ella con 2 cm de junta, y la que ya estaba lleva el muro. Espacio la gira. Puede ponerle muros, suelo, cubierta y techo al colocarla.
- **Con lo que lleva.** Se mueve, se copia, se gira y se refleja con todo.
- **En el IFC.** Un `IfcElementAssembly` montado en fábrica con sus piezas. Vuelve igual al importarlo.

**M2 · Juntas entre cajas.** ✅ Hecho el 9/11/2026, rama `bloque/juntas`.
- **Muros de cajas distintas:** no se unen (en las esquinas solo se tocan); cada junta tiene su dueño.
- **Un lado contra dos cajas:** un lado puede tocar dos cajas. Si está abierto, cada una de las otras lleva su tramo de muro.
- **Avisos:**
  - lado abierto con parte al exterior (hueco en la fachada);
  - muro de junta con parte al exterior;
  - caja de planta alta que no apoya en ninguna de abajo, o desplazada de su retícula.
- **Petos y cubierta continua.** La envolvente pone la cubierta 20 cm por debajo de la coronación. Los muros de los lados libres suben hasta arriba, como peto; en los lados de junta, el muro acaba en la cubierta, que sigue por encima. Bajo otra caja no hay peto: forjado doble, el suelo de una sobre el techo de la otra, con su junta de 2 cm.
- **Despiece por caja:** chasis y entramado de lo que lleva.
- Queda la banda de lana de la junta y la de impermeabilización de la cubierta en 3D.

**M3 · Chasis, peso e izado.** ✅ Hecho el 9/11/2026, rama `bloque/juntas`.
- **El chasis** de §5 bis, guardado en la caja con sus medidas, como el módulo B:
  - marco de suelo de 120 × 100 con viguetas de 60 × 40 en dos vanos sobre una viga central, enrasadas abajo;
  - marco de techo de 60 × 40 en el plano del techo, con sus viguetas y su viga central;
  - pilares de esquina y pilares intermedios (en los lados largos abiertos, o donde se cuelga);
  - en los lados libres, el peto: montantes de 60 × 40 sobre el marco de techo y la coronación de 120 × 100 arriba.
  - En los lados de junta no hay peto, y el pilar entre dos de ellos acaba en el marco de techo.
  - Se ve en 3D y en planta (pilares cortados y puntos de izado).
  - En el IFC, un `IfcMember` por tubo con su perfil hueco, dentro del ensamblaje.
- **Peso y centro de gravedad** de la caja terminada: chasis, muros con su entramado, carpinterías, forjados y techos.
- **Izado** con balancín desde cuatro o seis puntos: lo que tira cada eslinga, y la flexión y la flecha del borde largo del chasis (peso × 1,35 × 1,20).
- **Orden de montaje:** por plantas; de una esquina y siguiendo por las cajas que tocan a las ya puestas.
- **Con las capas del ejemplo:**
  - cada caja pesa entre 10 y 12,5 t (el suelo colaborante y la fachada son lo que más pesa);
  - colgada de cuatro esquinas, el borde de 120 × 100 × 4 no llega;
  - colgada de seis puntos, llega justo.
- **Queda:**
  - contar el muro como viga;
  - las orejetas y las uniones atornilladas;
  - los perfiles del catálogo del fabricante.

**M4 · Del IFC por capas a muros con tipo.** ✅ Hecho el 9/11/2026, rama `bloque/m4-capas` (`docs/FUNDACIONAL.md`, «Importación IFC»).
- Al importar, los muros finos paralelos y pegados se convierten en un muro con su tipo multicapa.
- Con el modelo de ejemplo: 411 muros del fichero pasan a 92 muros con tipo. Quedan sueltos solo los revestimientos que no tienen nada detrás, como los frentes del porche.
- La fachada principal queda en un solo tipo: «197 mm · EXT-PLACA_CERÁMICA + AIS-Tabihaus + AIS-MEMBRANA + EXT_MADERA + AIS-LANA + INT-PLACA».
- El entramado se activa en ese tipo (capa de lana de 100 mm). Es lo que hace que la comprobación lo calcule como steel frame y no como fábrica.
- Mientras se probaba salió un fallo del 3D de los forjados (una división de triángulos sin límite que se quedaba sin memoria al exportar). Queda acotado.

**M5 · Envolvente con los montantes.** HE 1 debe contar el puente térmico de los montantes de acero en la U del muro, del suelo y del techo (hoy cuenta las capas como continuas).

**M6 · Instalaciones en las juntas y máquina.**
- Tuberías cortadas en la junta, con su conexión.
- Pasos por los montantes.
- Fichero para la perfiladora, cuando se sepa cuál es.

**M7 · Techos.** ✅ Hecho el 9/11/2026, rama `bloque/techos`. El falso techo como elemento, que antes no existía (solo las vistas de techo).
- Tiene su contorno, su altura, sus capas y sus registros. Se dibuja por contorno o dentro de un local.
- Sale en la planta de techos, en las secciones y en 3D, y se mide.
- En el IFC es un `IfcCovering` de techo, a la ida y a la vuelta.
- Al importar entran los techos de otros programas (los del L-160 se quedaban fuera).

**M8 · Catálogo del sistema.** ✅ Hecho el 9/11/2026, rama `bloque/catalogo`.
- **Qué es una caja tipo.** Un fichero de ArchiSim con una caja y lo que lleva. Tiene código («CB-620-DD»), nombre, estado (en diseño o aprobada), revisión, fecha y notas.
- **Cómo se hace.** Se selecciona una caja de cualquier proyecto (por ejemplo, el L-160 importado con sus cajas dibujadas encima) y se usa «Guardar como caja tipo…». Sale en el origen, a ±0,00, en la carpeta «Cajas tipo» de la biblioteca.
- **Aprobarla la cierra** en su revisión: el programa no deja cambiar nada. Para mejorarla se abre la revisión siguiente, se cambia y se aprueba.
- **Lo que elige cada vivienda.** Por ahora, puertas y ventanas: una de la caja tipo que se puede cambiar por otras del mismo fichero. Los acabados y la distribución quedan para más adelante.
- **En la vivienda.** Las cajas tipo se colocan desde la herramienta Caja. Cada copia se pega a la caja de al lado, toma la marca siguiente (MOD-nn) y lleva en su tipo el código y la revisión. Cuando una caja tipo cambia, las viviendas la actualizan como cualquier módulo.
- **La primera caja tipo, del L-160:** la hace Antonio con su fichero (no se sube al repositorio). El ejemplo `archisim catalogo` trae tres inventadas con las medidas y la fachada del L-160.

**M9 · Taller.** Hecho (10/11/2026, formato versión 106), salvo los paneles de entramado en alzado dentro de la hoja de cada caja (están en el despiece de entramados de siempre).
- **Planos de taller**, un juego A3 por proyecto (`archisim taller-cajas`, o Archivo → Exportar → Planos de taller de las cajas):
  - M-00, montaje: la planta con las cajas coloreadas por su estado, el orden de montaje, la grúa con los círculos de su tabla y la tabla de izados (carga, radio, lo que admite, uso);
  - M-01…, una hoja por caja en el orden de montaje: chasis de suelo y de techo con peto en planta, los cuatro alzados del chasis a 1:60, despiece (lista de corte del chasis y totales del entramado), materiales (m² de muros, suelos, cubierta y techos; carpinterías) y peso e izado.
- **Estado de cada caja**: en diseño, aprobada, en fábrica (con su puesto), en camión, montada, con la fecha. Sale en planta bajo la marca, con su color, y en el IFC (`ArchiSim_Box`: Stage, Station, StageDate).
- **Plan de grúa**: dónde está la grúa (o se propone en el centro de las cajas), su tabla de cargas (por ahora una orientativa de grúa móvil de 50 t; hay que poner la de la grúa que se contrate) y el peso del aparejo. Para cada caja, el radio, la carga y si la grúa la levanta allí.

**M10 · Obra previa desde las cajas.** Hecho (10/11/2026; sin cambios de formato). Desde las cajas que están sobre el terreno:
- **Apoyos**: uno bajo cada esquina y otro en el medio de cada lado largo, en la línea de los pilares del chasis (medio tubo hacia dentro). Los de cajas juntas (a menos de 35 cm) son uno solo, más grande, con su placa para las dos, tres o cuatro cajas. Se numeran A1, A2… por filas.
- **Qué carga cada apoyo**: el peso de la caja terminada y de las que tiene encima, más el uso (2 kN/m²); una esquina se lleva 3/32 y un medio 5/16 (cada lado largo, como viga continua de dos vanos). Se comprueba el pilar de hormigón y, con la losa y el zócalo, la presión en el terreno contra la tensión admisible de los datos de Estructura.
- **Hacerla** (Propiedades de una caja → «Obra previa»): en una planta nueva bajo las cajas, «Cimentación de las cajas», con su cara superior en la base del chasis menos el alto de los apoyos (60 cm por defecto). Lleva la losa (45 cm, 30 cm de vuelo por fuera de las cajas, una por grupo de cajas que se tocan, con su contorno en L o como sea), un pilar por apoyo (25 × 25, o 40 × 40 si es de varias cajas) y el zócalo de bloque de 20 cm a haces con las fachadas. Volver a hacerla quita la anterior. Son losas, pilares y muros normales: salen en planta (cortada a 30 cm por su plantilla), en 3D, en el IFC, en las mediciones y en el cálculo.
- **Replanteo**: la última hoja de los planos de taller, con los apoyos en planta medidos desde la esquina de las cajas por sus ejes, y su tabla (X, Y, cota de la placa, pilar, placa, pernos, carga mayorada y cajas).
- **Queda**: las rejillas de ventilación del zócalo (el forjado sanitario es ventilado), los pernos como piezas del IFC y el armado de la losa (ya se puede dibujar y comprobar con Armado).

## 3 ter. Hacia el LOD 400 (10/11/2026)

Lo que falta para montar una vivienda con detalle de fabricación y trabajar cómodo, en el orden propuesto a Antonio:

**M11 · Plantilla del sistema y perfiles del fabricante.** Hecho (10/11/2026; sin cambios de formato).
- **Plantilla desde un proyecto** (Archivo → «Guardar como plantilla…», o `archisim plantilla`): el proyecto sin nada dibujado, con sus plantas, materiales, tipos, plantillas de vista, cajetines, planos, tablas y datos de cálculo. **«Nuevo desde plantilla…»** (Archivo y pantalla de inicio) empieza un proyecto con ella, o con cualquier proyecto tomado como plantilla.
- **Perfiles del fabricante**: un catálogo como fichero de la biblioteca del estudio (carpeta «Perfiles de entramado»; nombre; material; canto; ala; labio; espesor, en mm). Al hacer la plantilla, los tipos con entramado toman sus montantes y canales por el grueso de la capa que llenan (el C más profundo que cabe, en su espesor más fino, y la U en la que entra), y con `--entramado` también los muros con aislamiento entre placas: esa capa pasa a ser la estructura del tipo.
- **Con el L-160 y el catálogo SFsystem** (SFC/SFU): las fachadas de 197 mm y el tabique de 150 toman SFC100 1,2 y SFU100 1,2. Los tabiques de placa de 100 mm (lana de 50) no tienen perfil en ese catálogo: son de otro sistema (montantes de 48) y quedan sin entramado.
- **Queda para Antonio**: la serie SFM del catálogo (no dice si es C o U y repite cantos), el perfil de los tabiques interiores y confirmar el espesor (1,2 o 1,5) que da el cálculo.

**M12 · Las cajas de una vivienda importada.** Hecho (10/11/2026; sin cambios de formato).
- **Cajas sobre los suelos**: en la herramienta Caja, «Poner N cajas sobre los suelos» (o `archisim cajas-sobre`). Una caja sobre cada suelo del tamaño de una caja (cerrado por muros: una terraza no), llevada al módulo de 6,10 × 3,60 si cabe sin pisar a la vecina. Sus lados se leen de los muros de las juntas: del muro (con el muro a su lado de la junta), abierto o libre.
- **Cajas tipo** de cada caja con `--tipos <carpeta> --prefijo L160`, en diseño, para revisarlas y aprobarlas.
- **Locales desde una lista** (`archisim locales`, nombre; uso; x; y): cada uno se cierra con los muros alrededor de su punto. Los puntos que caen en el mismo espacio (sin muros entre ellos) se dicen y no se repiten.
- **Locales con lo que dibujan otros programas**: los pilares cierran locales (como en Revit) y los huecos de hasta 15 cm entre muros se puentean al buscar el local.
- **Con el L-160**: 8 cajas de 6,10 × 3,60, con el salón, el comedor y la cocina abiertos entre cajas. 10 locales, con 160,26 m² útiles: los del nombre del modelo. Avisos que salen del modelo de Revit: en cuatro juntas hay tabique en las dos cajas (dos placas de pladur una junto a otra); debe llevarlo solo una.

**M13 · La junta completa y las uniones.** Hecho (10/11/2026; sin cambios de formato). Las piezas no se guardan: salen de las cajas (`archisim_geom::box_joints`), como el chasis. Valores estándar, a confirmar con el fabricante y el instalador:

- **Junta** (cada junta la lleva la primera caja de la pareja): lana mineral en el hueco entre chasis, de la base a la cubierta; banda impermeable de 30 cm sobre la junta en cubierta (no si hay otra caja encima); membrana transpirable con cinta de 20 cm en los extremos de la junta que dan a fachada; tapajuntas de aluminio de 8 cm en suelo y techo cuando los dos lados están abiertos.
- **Uniones**: placas de acero 200 × 100 × 10 mm atornilladas a los dos chasis (cuatro tornillos M16 8.8), en los extremos de la junta a 15 cm y en el centro si pasa de 4 m, en el marco del suelo y en el del techo; orejetas de izado en los puntos de izado; placas de apoyo sobre los pilares de la obra previa (250 × 250 mm con una caja, 400 × 250 con dos, 400 × 400 con más; 15 mm) con dos pernos M16 por caja, de 30 cm, con resina.
- **Cuentas**: cuatro tornillos autotaladrantes por montante del entramado; soldadura en todo el contorno de los dos extremos de cada tubo del chasis.
- **Dónde sale**: en 3D, en el IFC (`IfcCovering` de aislamiento, membrana o moldura, `IfcPlate`, `IfcDiscreteAccessory` y `IfcMechanicalFastener` .ANCHORBOLT., con `ArchiSim_BoxPart`, agregados a la caja), en las mediciones (capítulo 17, «Cajas modulares: chasis, juntas y uniones», una línea por caja) y en la tabla de materiales de la hoja de taller de cada caja. Los materiales («Lana mineral de junta», «Banda impermeable de junta», «Membrana de junta», «Tapajuntas de aluminio») los añade la biblioteca al poner cajas (`archisim_library::joint_materials`).

**M14 · Tableros y placas por piezas.** Hecho (10/11/2026; sin cambios de formato). `archisim_geom::boards`: cada capa de placa o tablero de un muro, suelo, cubierta o falso techo se coloca en su formato comercial desde el inicio del muro (o la esquina del forjado), con las juntas de una capa a media placa de las de la siguiente y de una fila a la siguiente, y cada placa recortada alrededor de puertas y ventanas. Como cada muro y forjado de una caja acaba en la caja, las juntas de la fachada caen sobre la de las cajas. Formatos estándar por el nombre del material, a confirmar con el proveedor: placa de yeso 1200 × 2600, OSB y tablero 1250 × 2500, placa de cemento 1200 × 2500, placa de revestimiento 1200 × 2400, panel aislante 600 × 1200; la lana, las láminas, los morteros y las tarimas no son placas. Las placas a comprar son las enteras más los recortes empaquetados en placas (cortes de guillotina, girando la pieza si cabe mejor).

- **Lista de compra por caja** (`shopping_list`): placas por formato, perfiles en barras de 6 m, lana en paquetes de 7,2 m², láminas en rollos de 1,5 × 50 m con un 10 % de solape, junta y uniones (M13), chasis en kg y metros de soldadura, y tornillería.
- **Dónde sale**: `archisim compra <proyecto> [carpeta]` (`tableros.csv`, pieza a pieza con su marca «M12·1·03» —muro, capa, pieza—, y `compra.csv`, por caja y total); `archisim cajas` escribe `cajas-tableros.csv` y `cajas-compra.csv`; en la app, Archivo → Exportar → «Lista de compra de las cajas (CSV)».

**M5 · Montantes en el HE 1.** Hecho (10/11/2026; sin cambios de formato). La U de un muro, suelo o cubierta con entramado ligero cuenta los montantes: la capa que llena el entramado son dos caminos, la lana entre montantes y los montantes, y la resistencia es la media de los límites superior e inferior de la UNE-EN ISO 6946 (`archisim_mep::thermal::framed_u_value`). El montante de acero es su alma (su espesor, λ 50 W/m·K) a lo ancho de su ala; el de madera, todo su ancho (λ 0,13). Con todo el aislamiento entre montantes de acero cada 600 mm, la U casi se duplica (queda trabajando la mitad de la lana, como dicen las tablas de ASHRAE para el acero); con un aislamiento continuo por fuera, como el panel de 51 mm de la fachada de las cajas, sube mucho menos. Es un método simplificado: para el proyecto de ejecución conviene el valor de un cálculo por elementos finitos o del fabricante.

**M6 · Instalaciones en steel frame.** Hecho (10/11/2026; sin cambios de formato). `archisim_geom::box_services`:
- **En la junta**: cada tubería, conducto o canalización que cruza una junta entre cajas, y cada montante, bajante o patinillo que pasa de una caja a la de encima, se corta en fábrica y se une en obra: manguito (tuberías), brida (conductos) o caja de conexión (cables), con un registro para llegar a ella. Salen como avisos informativos en la comprobación y en el pedido de la caja.
- **Pasos por los montantes**: donde una tubería corre dentro de un muro con entramado y atraviesa el alma de un montante, el paso (con su pasamuros de goma, +10 mm) no puede pasar del 40 % del canto del montante ni estar a menos de 25 cm del canal. Valores habituales, a confirmar con el fabricante del perfil.
- **Pendiente**: el patinillo como elemento propio (hoy son los montantes y bajantes de siempre).

**M15 · Reglas del sistema como avisos y más elecciones por cliente.** Hecho (10/11/2026; sin cambios de formato).
- **Reglas** (Comprobación CTE → «Sistema modular (reglas del sistema)», `archisim_cte::modular`): los avisos de cada caja (medidas, transporte, juntas); puertas y ventanas con la jamba a menos de 30 cm de un pilar de esquina del chasis; huecos de más de 2,40 m sin viga; ninguna ventana en un lado que da a otra caja; locales húmedos que pasan por una junta; y las instalaciones de M6. Valores en la cabecera del módulo, a acordar con el fabricante.
- **Elecciones por cliente**: una opción de una caja tipo ya no es solo una puerta o ventana: puede ser un tipo de muro o de suelo (los acabados), un techo o un tipo de objeto (el equipamiento), y se cambia solo por otro de su clase (`SlotKind`). Cada vivienda elige en las propiedades de su copia, como antes.
- **Pendiente**: la distribución como elección (hoy se hace con cajas tipo distintas).

**M16 · Fábrica.** Hecho (10/11/2026; sin cambios de formato). `archisim fabrica <proyecto> [carpeta]` (también dentro de `archisim cajas`) y Archivo → Exportar → «Fábrica de las cajas»:
- **Paneles acotados**: una hoja A3 por caja (o más) con los paneles de sus muros en alzado desde fuera a 1:25, su largo y alto y la cadena de cotas de los montantes.
- **Etiquetas**: hojas A4 de 3 × 8 (70 × 37 mm), una por pieza, con su marca (caja·panel·pieza), caja, panel, elemento, perfil, largo y un código QR que lleva todo eso.
- **Pedido por caja**: un CSV por caja con su lista de compra (M14), más las uniones de instalaciones y los pasamuros de M6.
- **Perfiladora**: un CSV abierto, una línea por pieza: marca, caja, panel, elemento, perfil, largo en mm y operaciones en mm desde el inicio (tornillos donde se cruzan montante y canal, cortes de labio en los extremos de los canales, agujeros de instalaciones). El programa de la máquina que se compre lo traducirá a su formato; cuando se sepa cuál, se escribe directamente.

**M17 · El porche como módulo ligero de acero.** Hecho (10/11/2026; sin cambios de formato). Propiedades de una caja → «Porche» (`Model::build_porch` con `PorchOptions`): en el lado elegido, pilares de tubo cuadrado 100 × 100 × 4 en las esquinas de fuera y cada 3,60 m como mucho, vigas de tubo 160 × 80 × 5 por el frente y las dos vueltas hasta el marco de techo de la caja, y cubierta de chapa de 50 mm a la altura de ese marco. Son pilares, vigas y una cubierta de tipos propios («Pilar de porche…», «Viga de porche…», «Cubierta de porche…»): salen en planta, 3D, IFC, mediciones y cálculo como los demás. Volver a hacerlo en el mismo sitio quita el anterior. Dice su peso de acero (se fabrica como un bastidor y se iza con la caja). `archisim cajas` pone uno de 2,40 m en el primer lado largo libre de una caja sobre el terreno. Pendiente: sus placas de anclaje y que la obra previa le ponga apoyos.

**M18 · El despiece del estudio.** Hecho (10/11/2026; formato versión 108). Sale de la pregunta de Antonio sobre el despiece que hizo en Revit (en el IFC del L-160: Tabihaus de 1,20 × 3,00 en vertical, con la pieza cortada de 0,58 arriba en una columna y abajo en la siguiente; placa cerámica en columnas de 1,20 con juntas a ±0,00 y 2,50 —la cabeza de las puertas—, 0,81 encima y un zócalo bajo el suelo; lamas de madera en las columnas de puertas y ventanas hasta 2,50).
- **Por material** (Tipos y materiales → el material → «Despiece en placas»): formato de la placa, en vertical o no, y juntas a media placa, en cremallera (la pieza cortada arriba en una columna y abajo en la siguiente) o por líneas de junta (alturas sobre la planta), y en este último caso un acento: las piezas de la banda baja, hasta la altura que se diga, en las columnas que cruza una puerta o ventana, en otro material. Donde ese material sea una capa (fachada, forjado, techo), se coloca así.
- **Lo que cambia con él**: las placas pieza a pieza y la lista de compra (M14), los pedidos (M16) y, en las fachadas por líneas de junta, el 3D, los alzados y el render: cada placa como pieza con su junta abierta de 6 mm y las de acento 12 mm por fuera.
- **El despiece habitual** (`archisim despiece <proyecto> <salida>`, y ya en `archisim cajas`): los paneles aislantes en 1200 × 3000 vertical en cremallera, y la placa cerámica exterior en columnas de 1,20 con juntas a ±0,00 y 2,50 y madera en lamas alrededor de huecos hasta 2,50.
- **Remates (11/11/2026)**: la madera en su formato, hasta 1,20 × 2,70 (el material del acento lleva su propio despiece). Las columnas empiezan en cada jamba y en un marco de acento de 30 cm a cada lado del hueco («Marco» en el despiece): el ventanal de 3,00 queda con 0,30 + tres piezas de 1,00 + 0,30, la salida de dormitorio de 1,20 con 0,30 + 1,20 + 0,30 y la ventana de baño de 0,60 en una columna justa de 1,20. Hacia las esquinas van placas enteras desde el hueco y la pieza pequeña en la esquina; si quedara de menos de 30 cm, se reparte con la placa de al lado. En un paño ciego, la pieza pequeña se reparte entre las dos esquinas. La banda alta de 0,81 se corta de placas de 2,70 (tres por placa).

**M19 · La vivienda equipada (`archisim equipar`).** Hecho (11/11/2026). Desde un proyecto con locales (por ejemplo el L-160 importado con sus cajas y sus locales):
- **Amueblar** con una lista (nombre; x; y; giro, el punto en el centro de la trasera) de la biblioteca o del proyecto, y una colección para todo (`--coleccion «Nórdico cálido»`).
- **Electricidad** (ITC-BT-25, electrificación elevada): cuadro junto a la entrada, puntos de luz por superficie, interruptor o conmutador en cada puerta, tomas por superficie y uso, TV y datos, las de la cocina (C3, C5 en la encimera) y una toma por baño; C7 para los dormitorios y el despacho si pasa de 160 m² o de 20 tomas; la bomba de calor en su circuito.
- **Fontanería** (HS 4): contador en fachada, bomba de calor de ACS con depósito de 200 l y lavadora en la despensa (o el cuarto de instalaciones), agua fría y caliente por el falso techo en árbol hasta cada aparato y retorno si el grifo más lejano pasa de 15 m.
- **Saneamiento** (HS 5): una bajante corta en cada inodoro (y en cada grupo de aparatos lejos de ellos) con su válvula de aireación, colectores colgados en el forjado sanitario al 1,5 % hasta la arqueta de acometida y derivaciones al 3 %. **Pluviales**: los sumideros de la tabla 4.6 en cada cubierta, porche y terraza al aire, bajantes por fuera y colector enterrado a su propia arqueta.
- **Ventilación** (HS 3): aireador en la ventana de cada local seco, boca de extracción en cada húmedo y en los vestidores y trasteros de más de 4 m², conductos por el falso techo a un patinillo y el extractor en cubierta lejos de ventanas, y la campana con su conducto propio.
- **Reglas del sistema sobre lo importado**: el entramado de acero en la cámara de lana de los muros (las placas finas dejan de ser estructura), los montantes dibujados como pilares fuera, el suelo del sistema donde solo había acabado, un muro por junta (se quita el de la caja sin puertas), capas de suelo al revés, clases de material por su nombre; la cubierta de cada caja; porches ligeros (`--porche MOD-07:sur:3,0:7,2`) en lugar de los forjados de hormigón; el espesor de los montantes por cálculo (1,0, 1,2, 1,5 o 2,0 mm); la vivienda unifamiliar para el fuego; la estanqueidad de proyecto (`--n50 3`).
- **Con la L-160** (11/11/2026): 62 muebles y sanitarios del plano 01, 17 puntos de luz y 37 tomas, la red de agua con presiones de 250–320 kPa, el saneamiento con las 12 derivaciones dentro de su distancia, la ventilación con el extractor y la campana en cubierta, 26 sumideros. Montantes de 1,2 mm en la fachada de 197 (con 1,0 mm no llega al viento). De 140 «no cumple» de la comprobación a 17: los que quedan están en la bitácora (153).
- **Segunda vuelta** (11/11/2026, bitácora 154): el chasis entra en el cálculo general (los forjados de caja ya no son losas: se comprueban los largueros como dos vanos sobre la obra previa con la mitad de la caja cada uno, flecha L/300, y el izado; el tubo lo elige el cálculo con seis puntos de izado); las vigas del porche que llegan al marco de la caja van articuladas; la terraza baja apoya en el terreno; los porches cuentan como alero en el control solar. Se lanza también desde la app: Colaboración → «Equipar la vivienda…». En la L-160 quedan 5 «no cumple», todos de proyecto (la puerta del aseo, la V-02 en un lado de junta) o de dato a confirmar (Uw 1,4).
- **Pendiente**: la climatización (las unidades salen del cálculo de cargas pero no se colocan), la holgura de la puerta del aseo y los trozos de fachada sueltos del IFC.

**M20 · El despiece a mano, cara a cara.** Hecho (11/11/2026; formato versión 109). Pedido de Antonio: dibujar él la composición de cada fachada (la placa exterior y el Tabihaus) en lugar de la regla del material, con la junta ya fijada, y controlar el acento y las piezas de las juntas entre cajas, que se colocan en obra.
- **Dónde**: propiedades del muro → «Despiece a mano»: una fila por cada capa de placas, con «Dibujar el despiece…» y, si ya está hecho, «Volver a la regla».
- **Cómo**: se abre el alzado de la cara visto desde fuera, como «Dividir cara», con las juntas que da la regla del material como líneas de boceto para empezar. Cada línea es una junta: corta las piezas que cruza de lado a lado (llevándola hasta otra junta o el borde, con 5 cm de margen); una que se queda corta no corta nada. Se dibuja con las órdenes de siempre (Línea, Desfase, Recortar, Mover, Copiar…), también en diagonal. Las piezas se ven mientras se dibuja con su medida (120×270).
- **Piezas**: en la cinta, «Acento» (clic en una pieza: pasa al material de acento, que se elige en la barra de opciones; empieza con el acento del material o una madera), «En obra» (se deja fuera de la fábrica) y «Normal». Otro clic lo deshace.
- **La junta** es del material: «Junta (mm)» en su despiece (6 mm la cerámica y la madera, 0 el Tabihaus). En 3D cada pieza va con la junta abierta alrededor.
- **Lo que sigue**: las placas pieza a pieza, la compra (las piezas en obra salen aparte, «En obra», en la caja que las lleva), los pedidos de fábrica, `tableros.csv` (columna «Dónde»: fábrica o en obra), el 3D, los alzados y el render. Se copia con las cajas tipo y los módulos (en simetría, reflejado).
- **Remates (12/11/2026, formato versión 110)**: la junta se elige en cada cara («Junta (mm)» en la barra de opciones; si no se toca, la del material); la barra y el alzado dicen qué capa se está despiezando («Capa 1 · Placa cerámica (10 mm) · junta 6 mm · vista desde fuera»); «Acento» pasa a ser **Pintar**: se elige el material en «Pintar con» (las tarjetas de materiales, con su muestra) y se pinta con un clic o **arrastrando** sobre las piezas; un clic en una pieza ya pintada de ese material la devuelve al de la capa. «En obra» y «Normal» también se pueden arrastrar.
- **Tercera vuelta (12/11/2026, formato versión 111)**:
  - **Copiar a iguales** y **… en simetría** (grupo «Despiece a mano» del muro): la misma cara en los muros del mismo tipo, largo y alto; en simetría, el inicio de uno es el final del otro.
  - **En obra en las juntas** (cinta, grupo «Piezas»): marca de una vez para colocar en obra las piezas de los extremos del muro que dan a una junta entre cajas.
  - **Repartir** (cinta): clic en una pieza y se parte en N iguales, en columnas o en filas (cuántas y de qué modo, en la barra de opciones). Deja las líneas de boceto, que se pueden retocar.
  - **Avisos** mientras se dibuja, y en la Comprobación CTE («Sistema modular: despiece a mano»): en rojo la pieza que no sale de una placa del material; en ámbar la de menos de 30 cm, la junta a menos de 5 cm de una jamba, un antepecho o un dintel, y la junta que cae sobre una junta de la capa de al lado (Tabihaus y yeso, por ejemplo). La barra de opciones dice cuántos hay.
  - **Planos de despiece** (Archivo → Exportar → «Despiece a mano de las fachadas…», `archisim despiece-planos <proyecto> [carpeta]`): una hoja A3 por cara compuesta a mano, vista desde fuera, con cada pieza y su marca (las de obra en naranja, las pintadas en su color), los huecos, las juntas acotadas abajo y a un lado y la tabla de piezas (marca, material, medida, fábrica u obra) con los totales por material.
  - **Forjados y falsos techos en planta**: el mismo grupo «Despiece a mano» en sus propiedades. «Dibujar…» abre el boceto sobre la planta (empieza sin líneas): cada línea que cruza el forjado de lado a lado es una junta, y las piezas se pintan, se dejan en obra o se reparten igual que en los muros. Lo siguen las placas pieza a pieza, la compra, `tableros.csv` y los pedidos; en 3D, los forjados siguen como capa continua.
  - **Plantillas**: «Guardar como plantilla…» ya no se lleva los despieces a mano (antes una plantilla de un proyecto con caras despiezadas no se podía abrir).
- **Cuarta vuelta (13/11/2026, sin cambios de formato)**, lo que pidió Antonio al probarlo:
  - **Quitar** (antes «Normal»): clic o arrastrar; la pieza pierde la pintura y la marca de obra.
  - **Los huecos se ven como huecos** en el boceto (gris claro con su aspa, «hueco»): ninguna pieza los tapa y no se pintan.
  - **La pieza bajo el cursor se resalta** con su ancho × alto mientras se pinta, se marca o se reparte.
  - **Forzados y rastreo en el boceto en alzado**: final, punto medio, intersección y perpendicular de las juntas, del borde de la cara y de las jambas, antepechos y dinteles, y de las juntas de las capas de al lado (en azul discontinuo); con su marca verde, al dibujar, al mover y al copiar, también arrastrando la línea con la flecha.
  - **Cotas azules**: con una junta elegida, su distancia a la línea siguiente a cada lado; clic en la cifra, se escribe otra (0,2 o 20cm) y la junta se mueve.
  - **Mover con precisión**: al arrastrar sale la distancia y el ángulo junto al cursor y las líneas en su sitio nuevo; un número tecleado mientras se arrastra (con coma decimal) es la distancia, y con Intro se suelta. En la línea de órdenes, `0,2` es una distancia con coma decimal (antes se leía como el punto 0;2) y las coordenadas van con punto y coma (`2;1`).
  - **Juntas cada…** (cinta, «Piezas»): el paso en la barra de opciones (1,20 por defecto), verticales u horizontales; clic en una jamba, un borde o una junta (se engancha) y salen las juntas a ese paso hasta los dos bordes.
  - **Matriz** en las órdenes del boceto: copias de las líneas elegidas a paso fijo.
  - **Una junta que se queda corta por menos de 5 cm** cuenta como que llega (antes 2 cm).
  - **Forjados y techos** empiezan el boceto con las juntas de la regla de su material.
  - **Despiece a mano en la cinta**: al elegir un muro, un forjado o un falso techo, el grupo «Despiece a mano» con un botón por capa de placas, «Capa 1 · Placa cerámica»; en Propiedades, cada capa también por su material.
- **Pendiente**: el despiece de forjados y techos en 3D y en los planos de despiece.

## 3 bis. Cómo se organiza: componentes, cajas tipo y vivienda (9/11/2026)

Decidido con Antonio. **No se separa la estructura en un modelo vinculado** (como se pensó en Revit). La unidad que se fabrica, se transporta y se monta es la caja, y lleva todo junto: chasis, muros, suelo, techo, cubierta, instalaciones y acabados. Las disciplinas se separan con el filtro de disciplina, no en ficheros distintos. Hay tres niveles:

1. **Componentes del sistema**, en la biblioteca del estudio y no en cada proyecto: tipos de muro, suelo, techo y cubierta con su entramado y los perfiles del catálogo, el chasis, las uniones, las carpinterías y los acabados.
2. **Cajas tipo**: un fichero cerrado y con versión por caja («Caja 6,20 · dormitorio doble», «Caja 6,20 · baño + cocina», «Caja 6,20 · estar con lados abiertos»). Se diseñan y se comprueban una vez. Son los módulos de proyecto de ArchiSim (`model/modules`): un fichero colocado varias veces, girado o en simetría, que avisa cuando cambia.
3. **La vivienda**: la disposición de las cajas sobre la parcela, más lo propio del sitio (cimentación, acometidas, porche, terreno). El L-160 son 8 cajas y el porche.

Mejorar una caja tipo mejora todas las viviendas que la usan.

## 4. Buenas prácticas que el programa debe ayudar a seguir

- **La retícula de cajas manda.** Muros de carga y esquinas alineados en vertical; nada que apoye a media luz de una caja.
- **Huecos fuera de juntas y esquinas, en múltiplos de la separación de montantes.**
- **Cuartos húmedos agrupados**, a ser posible en una caja con su patinillo.
- **Juntas resueltas en el tipo:** acústica (DB-HR), fuego (DB-SI), estanqueidad y tolerancias. La junta que se cierra en obra se detalla primero.
- **Ambiente marino:** protección de los perfiles, tornillos y conectores acorde.
- **Aislamiento continuo por fuera de los montantes.**
- **Un solo modelo** del que salen los planos, el despiece y la máquina, con el diseño congelado antes de fábrica.

## 5. Propuesta de chasis, junta y porche (9/11/2026)

Antonio no tiene aún el chasis ni la junta desarrollados y pide una propuesta. Es un **punto de partida para predimensionar**, no un cálculo. Los perfiles salen del cálculo, que ArchiSim hará cuando exista M3. Parte de lo que fija el modelo L-160:
- caja de 3,60 × 6,20 m;
- apoyo inferior a −0,27;
- suelo terminado a ±0,00, falso techo a 2,93 y coronación a 3,31;
- apoyos en las cuatro esquinas y a media longitud.

### 5.1 La caja

Caja de **3,58 m de alto**: 27 cm de suelo, 2,93 m libres y 38 cm de cubierta.

**Chasis de suelo**, soldado en taller y galvanizado en caliente después de soldar (ambiente marino):
- **Vigas de borde**: dos largas de 6,20 m y dos testeras de 3,60 m, de perfil laminado en U o tubo rectangular de unos 200 mm de canto.
  - Se dimensionan para el **izado**: la caja colgada de sus cuatro esquinas es el caso que manda, no el apoyo en obra.
  - En obra apoyan a 3,05 m (esquinas y medio).
- **Viguetas de suelo**: perfil conformado de unos 150–200 mm a 400–600 mm, de borde a borde (3,60 m de luz), con tablero de 18–22 mm encima.
- **Bajo las viguetas**: aislamiento entre ellas y una placa de cierre por debajo, resistente a la humedad y a los roedores, porque la cámara del forjado sanitario es ventilada.
- **Comprobación de cantos**: 200 de chasis + 22 de tablero + 10 de mortero + 20 de madera ≈ 252 mm, dentro de los 270 del modelo.

**Pilares de esquina**: tubo cuadrado de unos 100–120 mm soldado al chasis de suelo y al de cubierta.
- Con un **pilar intermedio** a media longitud en los lados que van abiertos (ver 5.2), sobre el apoyo de medio.
- En una planta no cargan mucho: dan rigidez para el transporte y el izado y recogen la cubierta donde no hay muro.

**Chasis de cubierta**: igual que el de suelo, pero más ligero.
- Vigas de borde de unos 150–200 mm.
- Viguetas de 3,60 m de luz.
- Pendiente con aislamiento en cuña o rastreles.
- Peto en la coronación.

**Muros**: steel frame del catálogo del fabricante (montantes de 100 mm cada 600 mm) atornillado a los chasis. Composición de fuera a dentro:
1. placa cerámica de 10 mm;
2. membrana;
3. placa con aislamiento de 51 mm (aislamiento continuo por fuera de los montantes, que corta el puente térmico);
4. lana de 100 mm entre montantes;
5. placa de 2 × 12,5 mm.

**Izado**: cuatro orejetas o cantoneras en las esquinas del chasis de suelo, con balancín de grúa para que las eslingas vayan verticales y no aprieten los muros.

**Peso orientativo**: entre 5 y 8 t por caja terminada. ArchiSim lo calculará con su centro de gravedad (M3).

**Transporte**:
- **Altura**: 3,58 m de caja más la plataforma rebajada se queda en el entorno de los 4 m, que es el límite general de altura. Hay que confirmarlo con el transportista.
- **Ancho**: los 3,60 m superan los 2,55 m generales, así que es transporte especial con su autorización.

### 5.2 Lados abiertos y junta entre cajas

**Cada junta tiene dueño.** Donde dos cajas se tocan:
- si hay muro, lo lleva **una sola** de las dos;
- la otra deja el lado **abierto**, con su viga de borde y el pilar intermedio.

No hay muros dobles: la sala del L-160, que cruza varias cajas, sale de lados abiertos. Dentro de una vivienda no hace falta el aislamiento acústico entre cajas que pediría una medianera.

**Holgura de 20 mm** entre chasis, la tolerancia de fabricación y montaje, rellena con banda de lana mineral comprimida. Así no queda una cámara hueca que haga de conducto oculto para el fuego ni para el aire.

**Unión entre cajas**: placas atornilladas entre los chasis de suelo y de cubierta en cada esquina y en el medio. Atornilladas para poder desmontar; la soldadura queda en el taller.

**Apoyo**: el apoyo compartido por dos esquinas (que es la mayoría en el L-160) no cabe en 25 × 25.
- Propuesta: pilar de **40 × 40** o una **placa de acero** de 400 × 250 encima.
- Sobre ella, calzos de nivelación y banda elastomérica.
- Pernos de anclaje que resistan también el levantamiento por viento.

**Cierres en obra**, lo único que se hace en la parcela:
- **Cubierta**: banda de impermeabilización de unos 30 cm sobre la junta, soldada a la de las dos cajas, que llegan con ese borde sin terminar.
- **Fachada**: la junta de la placa cerámica coincide con la de las cajas. Detrás va una banda de membrana solapada y una cinta de estanqueidad al aire (la ventilación con recuperación de calor necesita una envolvente estanca).
- **Suelo y techo interiores**: tapajuntas o la última tabla del suelo y una banda de placa del techo puestas en obra.
- **Instalaciones**: conexiones en un registro junto a la junta.

### 5.3 El porche

Un **módulo ligero** de estructura de acero, no forjados de hormigón. Los dos de 30 cm del modelo pesan unos 7,5 kN/m² cada uno, demasiado para colgarlos de una fachada de steel frame.

**Partido en dos piezas de 7,20 m**, dos pasos de caja, en lugar de una de 14,40:
- caben mejor en el camión;
- la junta cae sobre la pata de x = −7,06, que ya está en el eje de una caja.

**Apoyos**:
- **Pieza 1**: sus dos patas (x = −14,21 y −7,06) por delante, y por detrás la fachada sur de la fila de cajas.
- **Pieza 2**: la pata compartida, la fachada de detrás y, en su extremo, la fachada del ala.

**Unión a la fachada**: se cuelga de las **vigas de borde del chasis de cubierta y de los pilares** de las cajas, nunca de los montantes.
- Lo hace con un durmiente atornillado en los puntos de los pilares (esquinas y medio).
- Entre medias, una pieza de **rotura de puente térmico**.
- El viento levanta un porche: las patas van ancladas a la losa con pernos que trabajen a tracción, y la unión a la fachada también.

**Agua**: pendiente hacia fuera de la fachada, con su canalón. La junta con la fachada va con un vierteaguas que permita el movimiento entre las dos piezas.

**La piscina y la terraza** son de presentación y no forman parte de la vivienda.

## 5 bis. El módulo base de estructura (9/11/2026)

Antonio exportó de Revit el módulo B de 6,10 × 3,60 m con su estructura, como caja suelta (sin uniones con otras). Lo que lleva:

**Pilares y marcos.**
- Pilares de esquina de tubo 120 × 100 × 4 de toda la altura, de −0,27 a 3,31 (3,58 m).
- Marcos perimetrales de suelo y de techo del mismo tubo.

**Suelo, de abajo arriba.**
- Viguetas de tubo 60 × 40 × 3 cada 60 cm, en dos vanos de 1,62 m, con una viga central de 60 × 40.
- Bordes de 60 × 60.
- Forjado colaborante de chapa H90 + 30.
- Tabihaus 8 + 43 + 8.
- En total, los 27 cm del suelo.

**Techo y cubierta.**
- Viguetas de 60 × 40 a 2,93 (el plano del techo).
- Tabihaus a 2,99 y recrecido de pendiente.
- Pretil de 20 cm sobre montantitos de 40 × 60 en los cuatro lados, con lana y membrana.

Lo que cambia respecto a §5:
- **El chasis es de tubo soldado**, no de perfiles de chapa. El de §5.1 queda sustituido por este.
- **Pretil según el lado.** En otras variantes, la coronación cambia según el lado que se une a otra caja, para hacer una cubierta plana continua. En la caja tipo (M1, M8), cada lado tendrá un estado:
  - **libre**: con pretil y fachada;
  - **unido**: sin pretil, cubierta continua, lado abierto o con muro según el dueño de la junta.
- **Izado, a comprobar.** Colgada solo de las cuatro esquinas, el tubo de borde de 120 × 100 × 4 en 6,10 m va justo o no llega (unos 18 kNm frente a unos 15 kNm resistentes, cálculo rápido). Opciones:
  - seis puntos de izado;
  - contar con el muro de fachada como viga;
  - más canto en el borde.
- **Al importarlo en ArchiSim.**
  - Los tubos entran como piezas macizas con la forma de su caja, y las dos riostras (`IfcMember`) se quedan fuera.
  - Hacen falta perfiles tubulares y riostras de verdad (M1, M3).

## 6. Pendiente de Antonio

- Orden aprobado (9/11/2026): M4 (hecho) → M7 techos (hecho) → M1 caja (hecha) → M8 catálogo (hecho; falta la primera caja tipo real del L-160) → M2 juntas y M3 chasis (hechos) → M9 taller (hecho) → M10 obra previa (hecha, 10/11/2026) → M11 plantilla y perfiles (hecha) → M12 cajas de una vivienda importada (hecha) → M13 juntas y uniones (hecha) → M14 tableros por piezas y lista de compra (hecha) → M5 montantes en el HE 1 (hecho). → M6 instalaciones, M15 reglas y elecciones, M16 fábrica (hechos). → M17 porche (hecho) → M18 despiece y M19 vivienda equipada (hechos) → M20 despiece a mano (hecho). §3 ter completo; queda lo que diga Antonio al probarlo.
- Exportar desde Revit el modelo de estructura vinculado como IFC aparte, para partir de él en el chasis de la caja tipo.
- Catálogo de perfiles del fabricante como fichero (`.csv`, §3 ter de ENTRAMADOS).
- Propuesta de §5 aprobada (9/11/2026).
