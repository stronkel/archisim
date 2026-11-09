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

**M1 · La caja como elemento.** «Módulo»:
- un rectángulo (o contorno) en planta con su altura, sus muros, su suelo y su techo;
- su marca («MOD-03»), su contorno de transporte y aviso si se pasa de lo que admite el camión;
- en el IFC, un `IfcElementAssembly` con sus piezas;
- colocado con forzado a la retícula de cajas, y copiable.

**M2 · Juntas entre cajas.** Donde se tocan dos cajas:
- holgura entre chasis y cada junta con su dueño: muro en una caja y lado abierto en la otra (§5.2); en dos plantas, forjado doble (suelo de una, techo de la de abajo);
- los muros de cajas distintas no se unen;
- el despiece del entramado sale por caja.

**M3 · Chasis, peso e izado.**
- Bastidor de suelo y de techo y pilares de esquina, con perfiles más pesados que los montantes y uniones soldadas.
- Peso y centro de gravedad de la caja terminada.
- Puntos de izado y caso de carga del izado.
- Orden de montaje.

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

**M8 · Catálogo del sistema.** Las cajas tipo como ficheros cerrados, con versión.
- Cada una lleva lo que se puede elegir (acabados, carpinterías, distribución dentro de unas reglas) y lo que no se toca.
- Una caja aprobada se congela con su número de revisión.
- La primera caja tipo sale del L-160.

**M9 · Taller.**
- Planos y despiece de cada caja: paneles en alzado, chasis, lista de corte y lista de materiales, por caja.
- Estado de cada caja: diseñada, aprobada, en fábrica (en qué puesto), en camión, montada.
- Orden de montaje en obra y plan de grúa.

**M10 · Obra previa desde las cajas.** Los apoyos se colocan solos en las esquinas y en los medios de cada caja, y salen la losa, los pilares y los pernos con sus cotas.

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

- Orden aprobado (9/11/2026): M4 (hecho) → M7 techos (hecho) → M1 caja → M8 catálogo con la primera caja tipo → M2 juntas y M3 chasis → M9 taller. M5, M6 y M10 cuando toquen.
- Exportar desde Revit el modelo de estructura vinculado como IFC aparte, para partir de él en el chasis de la caja tipo.
- Catálogo de perfiles del fabricante como fichero (`.csv`, §3 ter de ENTRAMADOS).
- Propuesta de §5 aprobada (9/11/2026).
