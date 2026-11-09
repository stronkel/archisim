# Instalaciones: dibujo y cálculo

ArchiSim dibuja las instalaciones (tuberías, conductos y aparatos) y las calcula sobre lo dibujado. Este documento recoge la segunda vuelta: cómo se meten las tuberías, el cálculo eléctrico con su esquema unifilar, el cálculo de fontanería, el de saneamiento, el de ventilación con las campanas, la envolvente con las cargas térmicas, los esquemas y la memoria de instalaciones.

Código: `archisim-core::model::pipe_routing` (uniones, cambio de planta y trazado automático), `archisim-core::model::water` (datos del suministro), crate `archisim-mep` (cálculos, depende de `archisim-core` y `archisim-geom`: `electrical`, `water`, `drain`, `ventilation` y `thermal`), `archisim-plan::single_line` (esquema unifilar), `archisim-plan::water_schematic` (esquemas de fontanería, saneamiento y ventilación), `archisim-plan::services_report` (memoria de instalaciones), `archisim-cte::electrical`, `archisim-cte::services::calculation`, `archisim-cte::ventilation` y `archisim-cte::energy` (comprobación) y, en la app, `app/services_ui`, `app/electrical_ui`, `app/water_ui`, `app/drain_ui`, `app/ventilation_ui` y `app/climate_ui`.

## 1. Meter las tuberías

- **Enganche.** Con la herramienta Tubería, cada clic se engancha a lo que hay cerca:
  - un aparato que sirve a ese sistema: montante (agua y calefacción), bajante (saneamiento), termo, contador, llave, depósito o grupo de presión (agua), radiador (calefacción), rejilla o difusor (conductos);
  - un aparato sanitario (lavabo, inodoro, bidé, ducha, bañera, fregadero, lavadora, lavavajillas), por el centro de su parte de atrás;
  - el extremo de otra tubería del mismo sistema, o cualquier punto de su eje (una te).
  
  Un aro azul marca dónde se va a enganchar. Si el último clic cae en un aparato, la tubería termina sola.
- **Altura.** Una tubería que sale de otra toma su altura en ese punto.
- **Pendiente automática** (saneamiento): 2,5 % en las derivaciones de menos de 75 mm y 2 % en los colectores (HS 5). La tubería baja siempre hacia la tubería o la bajante a la que llega, aunque se dibuje al revés, y llega a la otra tubería a su altura. Se puede quitar y poner la pendiente a mano.
- **Cambio de planta.** Mientras se dibuja, **RePág** sube y **AvPág** baja. La tubería acaba donde está, se pone un montante (una bajante en saneamiento) entre las dos plantas y se sigue dibujando desde el mismo punto en la otra planta.
- **Trazado automático.** En las propiedades de la herramienta, «Conectar los aparatos de esta planta» dibuja la derivación de cada aparato sanitario que no tiene tubería de ese sistema hasta lo más cercano: la tubería de ese sistema (en te), un montante (agua) o una bajante (saneamiento).
  - Va en dos tramos rectos, por el lado que cruza menos muros.
  - Usa el diámetro de cada aparato: 110 el inodoro, 40 la ducha, la bañera y el fregadero, 32 el lavabo y el bidé; en agua, 20 la bañera y la lavadora y 16 los demás.
  - El agua caliente solo va a los aparatos que la llevan.
  - Quedan seleccionadas para retocarlas.

### En secciones y alzados

- **Tuberías y conductos vistos.** Se dibujan por su eje, en el color de su sistema, como en planta.
  - Si su grueso en el papel es mayor que una plumilla, se dibujan como banda: su contorno en el color del sistema y el relleno en ese color aclarado.
  - Los conductos van siempre como banda: con su alto cuando van tumbados y con su ancho cuando suben.
  - Los montantes, bajantes y conductos verticales van como bandas de planta a planta.
  - Lo que está delante los tapa: un muro, un forjado o un mueble.
- **Tuberías y conductos cortados.**
  - Una tubería, como un círculo relleno del color de su sistema.
  - Un conducto, como un rectángulo con sus diagonales.
  - A 1:50 o más, con su sistema y su medida («Agua fría Ø 20 mm»), sobre blanco, donde no pisa otra tubería ni otro rótulo.
- Antes se cortaban como un sólido cualquiera, en negro. Por eso las cotas de altura libre de las secciones se paraban en la primera tubería; ahora llegan al techo.
- En los **alzados** no sale lo enterrado: lo que va por debajo de la planta más baja.
- Si la vista de la sección esconde un grupo de instalaciones (agua, saneamiento, ventilación…), ese grupo no sale, igual que en planta.
- `archisim secciones <proyecto> [carpeta]` dibuja todas las secciones y alzados de un proyecto a 1:100 y a 1:50. `archisim exportar` también saca ahora las de 1:50.

## 2. Cálculo eléctrico (REBT)

Colaboración → **Cálculo eléctrico…**. Sale del cuadro (aparato «Cuadro»), de los mecanismos con su circuito (Propiedades → Circuito, de C1 a C10) y de las lavadoras y lavavajillas dibujados. Con varios cuadros, cada mecanismo va al más cercano.

- **Grado de electrificación** (ITC-BT-10): el elegido, o uno calculado.
  - Es elevada con más de 160 m² de locales, con más puntos de los que admiten C1, C2 o C5, o con mecanismos en C6 a C10.
  - La básica da 5 750 W e IGA de 25 A; la elevada, 9 200 W e IGA de 40 A.
- **Circuitos** (ITC-BT-25, tabla 1):
  - C1 a C5 están siempre; los de la elevada (C6 iluminación, C7 tomas, C8 calefacción, C9 aire acondicionado, C10 secadora), si tienen puntos.
  - Potencia P = n·Pa·Fs·Fu con la potencia por punto y los factores de la tabla; un circuito de un solo punto toma su potencia con el factor de utilización.
  - Intensidad I = P/230.
  - Comprobación Ib ≤ In ≤ Iz, con Iz de la norma UNE-HD 60364-5-52, tabla B.52.2, método B1: tubo empotrado, PVC, dos conductores cargados.
  - Longitud hasta el punto más lejano, como van los cables: en planta en dos direcciones, subiendo del cuadro al techo (2,5 m) y bajando al punto, y entre plantas.
  - Caída de tensión ΔU = 2·L·P/(γ·S·U²), con cobre a 70 °C (γ = 48), contra el 3 % (ITC-BT-19). Si la sección de la tabla no basta, se da la que basta y su tubo.
- **Derivación individual** (ITC-BT-15): la sección, de 6 mm² en adelante, que lleva la intensidad y cae como mucho un 1,5 % (un solo usuario) o un 1 % (contadores centralizados). Se dan la longitud (dato de la ventana) y el tubo.
- **Protecciones**: el IGA, un diferencial de 40 A y 30 mA por cada cinco circuitos (ITC-BT-25) y el protector de sobretensiones (ITC-BT-23).

Todo sale en la ventana, en la Comprobación CTE (sección eléctrica, con el cuadro seleccionable) y en `archisim electricidad <proyecto> [carpeta]`.

## 3. Esquema unifilar

Una hoja A3 por cuadro: red, CGP, contador, derivación individual con sus conductores, IGA, protector de sobretensiones, diferenciales y, debajo de cada uno, sus circuitos con el PIA y la sección. Debajo, el cuadro de circuitos: uso, puntos, potencia, intensidad, PIA, conductores, tubo, longitud y caída de tensión. En rojo lo que no cumple; con ↑ la sección que sube por la caída de tensión.

- PDF: Archivo → Exportar → Esquema unifilar, o desde la ventana del cálculo.
- DXF: desde la ventana (un archivo por cuadro).
- `archisim electricidad <proyecto> <carpeta>` deja `<proyecto>-unifilar.pdf` y un DXF por hoja.

## 4. Fontanería (DB-HS 4)

Colaboración → **Cálculo de fontanería…**. Lo único que se escribe en la ventana es lo que no sale del dibujo:
- la **presión en el contador** que garantiza la compañía (por defecto 300 kPa; 100 kPa ≈ 1 bar ≈ 10 m de agua);
- la **tubería**: multicapa (por defecto), PE-X, PP-R o cobre, con sus medidas comerciales, su rugosidad y su velocidad máxima;
- las **pérdidas en el contador y el filtro** y **en el calentador**, en kPa: los datos del fabricante, que se restan a la presión a la entrada de cada red (por defecto 0, no se cuentan).

Lo demás sale del dibujo:
- **La red.** El agua fría se sigue desde el **contador** (o un aljibe o un grupo de presión) y la caliente desde el **termo o acumulador**. Se recorren las tuberías, las tes (una tubería que acaba sobre otra) y los **montantes**, de planta a planta, hasta los aparatos sanitarios. Un aparato está unido si una tubería de ese sistema acaba a menos de 45 cm de él. Si no hay contador o termo dibujado, se empieza en el extremo más bajo y se avisa.
- **Caudal de cada tramo.** Suma de los caudales instantáneos de los aparatos que alimenta (HS 4, tabla 2.1), con simultaneidad:
  - en viviendas, UNE 149201: Qc = 0,682·Qt^0,45 − 0,14;
  - en otros usos, Qc = Qt/√(n − 1).
  
  Nunca menos que el aparato mayor ni más que la suma.
- **Diámetro.** El menor comercial de la tubería elegida que deja el agua por debajo de 2 m/s y no baja de lo que pide el HS 4: el del aparato en su derivación (tabla 4.2) y 20 mm en lo que sirve a dos o más aparatos y en los montantes (tabla 4.3). Si después algún grifo se queda sin presión, el tramo de su camino que más pierde pasa a la medida siguiente, hasta que llegue.
- **Pérdidas de carga.** Darcy–Weisbach con el factor de fricción de Swamee–Jain (agua fría a 20 °C, caliente a 60 °C), más un 30 % por codos, tes y llaves.
- **Presión en cada grifo.** La del contador, menos las pérdidas del camino, menos la altura que sube hasta el grifo (la ducha a 2 m del suelo, el inodoro a 0,2, los demás a 1 m). Se compara con:
  - 100 kPa mínimo en cada grifo (HS 4, 2.1.3);
  - 150 kPa a la entrada del calentador;
  - 500 kPa como máximo en el contador (si hay más, reductor de presión).
  
  El agua caliente parte de la presión con que la fría llega al termo.

Cada tramo («F1», «F2»… en fría; «C1»… en caliente) se calcula dos veces: **como está dibujado** (esta es la comprobación) y **con el diámetro calculado**. La ventana da la tabla de tramos (longitud, aparatos, Qt, Qc, diámetro, velocidad y pérdida de cada versión) y la presión de cada grifo.

**«Poner los diámetros calculados»** da a cada tubería el mayor diámetro de sus tramos, en un paso de Deshacer. También achica las que estaban de más.

En la **Comprobación CTE** (Agua y saneamiento) salen:
- la presión de cada grifo y la del calentador;
- cada tramo con su diámetro dibujado contra el calculado;
- la velocidad (más de 3,5 m/s en plástico o 2 m/s en cobre no cumple);
- los aparatos sin unir.

«Arreglar lo que no cumple» pone el diámetro calculado en los tramos cortos.

**Grupo de presión, aljibe y retorno**
- **Grupo de presión** (HS 4, 4.5.2). Se calcula si hace falta (con los diámetros calculados, la presión de la red no llega a 100 kPa en algún grifo o a 150 en el calentador) o si hay uno dibujado (aparato «Grupo de presión»). Da:
  - el caudal: el simultáneo de todo el edificio;
  - la presión mínima a su salida, la que deja el grifo peor justo en 100 kPa;
  - la de parada: la mínima más 200 kPa;
  - dos bombas iguales, una de reserva.
  
  Si está dibujado, el cálculo cuenta con lo que da. Si hace falta y no está, la comprobación no cumple. El volumen del depósito de presión lo da el fabricante con esas presiones.
- **Aljibe** (HS 4, 4.5.1). Con grupo de presión, o con un «Depósito» dibujado, guarda 15 minutos del caudal simultáneo. Si no está dibujado, sale un aviso con su volumen.
- **Retorno de ACS** (HS 4, 2.3 y tabla 4.4). Cuando del acumulador al grifo de agua caliente más lejano hay 15 m o más de tubería, hace falta una red de retorno. Se da el caudal (250 l/h por columna, al menos), su tubería por la tabla 4.4 (en pulgadas y en multicapa) y que vaya aislada.
  - El retorno se dibuja como sistema propio, **Retorno de ACS** (en morado). Va con la herramienta Tubería, del final de cada distribuidor de agua caliente al montante, por él y hasta el acumulador.
  - Comprobación:
    - sin dibujar, sale como aviso;
    - dibujado, cumple si una de sus puntas llega al acumulador (a menos de 45 cm) y ninguna tubería es más fina que la de la tabla 4.4;
    - las que lo son salen una a una.
  - «Poner los diámetros calculados» les da la de la tabla.
  - Sale en la plantilla Instalaciones · Fontanería.

Los tres salen en la ventana, en la Comprobación CTE y en las notas de la hoja FO-01.

En la línea de órdenes: `archisim fontaneria <proyecto>`, o `archisim fontaneria <proyecto> --diametros salida.archisim` para guardarlo con los diámetros calculados (y `--esquemas <carpeta>` para las hojas, abajo).

El edificio de Florence Street (`archisim edificio`) lleva ahora la red entera: contador y acumulador junto al montante, montante hasta la cubierta, distribuidor en cada rellano, derivación de cada vivienda y todos los aparatos unidos. Son 114 aparatos y 24 viviendas. Con 300 kPa en el contador, los diámetros dibujados dejan 95 grifos por debajo de 100 kPa; con los calculados, el peor llega a 122 kPa. Del acumulador al grifo más lejano hay 41,8 m: necesita retorno (250 l/h, 3/4"). Lo lleva dibujado: del final de cada distribuidor de las plantas 1 a 4 al montante, y en la planta baja del montante al acumulador. Son 57,7 m y cumplen.

### Esquemas de fontanería

Hojas A3 sacadas del cálculo, con los **diámetros calculados**. Donde la tubería dibujada es menor se escriben los dos en rojo («Ø20→26»), y también en rojo los grifos sin presión con lo dibujado. El agua fría va en línea continua y la caliente a trazos.

- **FO-01, esquema vertical.** Las plantas una sobre otra, con su cota, y abajo el origen de cada red (contador o acumulador, con su presión). Los montantes suben de planta a planta con el tramo y el diámetro de cada trozo. En cada planta, una caja por cada derivación que sale del montante: qué aparatos alimenta, el tramo y el diámetro con que arranca, y la presión mínima de sus grifos. Las derivaciones grandes se parten hasta llegar a cada vivienda o local húmedo. Agua fría a la izquierda y caliente a la derecha.
- **FO-02 en adelante, distribución.** Una hoja por planta con aparatos: el árbol de cada derivación, tramo a tramo con su nombre y diámetro, hasta cada grifo con la presión que le llega.

Cómo sacarlos:
- PDF: Archivo → Exportar → Esquemas de fontanería, o desde la ventana del cálculo.
- DXF: desde la ventana, un archivo por hoja.
- En la línea de órdenes, `archisim fontaneria <proyecto> --esquemas <carpeta>` deja el PDF, y un SVG y un DXF por hoja.

## 5. Saneamiento (DB-HS 5)

Colaboración → **Cálculo de saneamiento…**. Se dibujan dos sistemas de tubería: **Saneamiento** (aguas residuales) y **Pluviales**, cada uno con sus **bajantes** (el aparato «Bajante»; con la herramienta Tubería, RePág y AvPág la ponen). Lo único que se escribe en la ventana es la **intensidad de lluvia** del lugar: la del anejo B del HS 5 (tabla B.1, por la zona A o B y la isoyeta). Por defecto es 100 mm/h, la de las tablas.

**Aguas residuales**
- **La red.** Cada aparato sanitario desagua en el extremo de saneamiento más cercano, a menos de 45 cm. Desde ahí la red se sigue por las tuberías, las tes y las bajantes hasta su punto más bajo, que es la acometida (la arqueta a la red).
- **Unidades de desagüe**, tabla 4.1. Se toma el uso privado en viviendas y el público en los demás usos (lavabo 1 o 2 UD, inodoro 4 o 5, etc.).
- **Cada tramo**, por lo que es:
  - **derivación** de un aparato: su diámetro de la tabla 4.1;
  - **ramal** que junta varios aparatos: tabla 4.3, con su pendiente (1, 2 o 4 %);
  - **bajante**: tabla 4.4, con todas las UD que recoge, las que le entran por la planta que más le da y si sirve a más de tres plantas. Lleva el mismo diámetro en toda su altura;
  - **colector**: lo que viene de una bajante, lo enterrado bajo la planta más baja o lo que llega a la acometida, por la tabla 4.5 con su pendiente.
- **Ningún tramo disminuye aguas abajo**, y nada que recoja un inodoro baja de Ø110.

**Aguas pluviales**
- **Superficies al aire.** Se toman las cubiertas, terrazas y balcones al aire: los forjados de las plantas de encima de la baja y las cubiertas, mirando qué queda arriba del todo en cada punto de una rejilla de 50 cm.
- **Sumideros.** Cada punto lleva su lluvia al sumidero más cercano que no esté por encima de él. Un sumidero es el extremo alto libre de una tubería de pluviales, o la cabeza de una bajante.
- **Diámetros.** Las bajantes van por la tabla 4.8 y los colectores por la tabla 4.9, con la superficie multiplicada por i/100.
- **Número de sumideros.** Cada cubierta o terraza de 25 m² o más se comprueba por la tabla 4.6: 2 sumideros por debajo de 100 m², 3 hasta 200, 4 hasta 500 y uno por cada 150 m² por encima. Se avisa también de la superficie que no tiene sumidero a su altura.
- **Canalones.** Las cubiertas inclinadas con canalón (cubrición de la cubierta) se comprueban por la tabla 4.7, al 1 %, con lo que toma cada bajante; sus bajantes, por la tabla 4.8.
- **Red mixta.** Si una red de pluviales acaba sobre un desagüe de residuales, los tramos de después llevan las dos aguas. Se dimensionan por la tabla 4.9 con las UD convertidas en superficie (4.3): 90 m² hasta 250 UD y 0,36 m² por UD por encima.

Cada tramo («S1», «S2»… en residuales; «P1»… en pluviales) se calcula con su diámetro dibujado y el exigido, en PVC (diámetro exterior = nominal de las tablas). **«Poner los diámetros calculados»** agranda las tuberías y bajantes que se quedan cortas, en un paso de Deshacer; las que están de más se quedan como están. En la **Comprobación CTE** salen todos los tramos, los sumideros y los canalones, y «Arreglar lo que no cumple» agranda lo corto.

### Ventilación de la red, sifones y derivaciones (HS 5, 3.3.1 y 3.3.3)

Todo sale del dibujo. Las bajantes que están una encima de otra en el mismo sitio son una **columna**; su cabeza es la parte de arriba de la más alta (su altura, `height`).

- **Ventilación primaria** (3.3.3.1). La columna debe salir por la cubierta que tiene encima:
  - 1,30 m sobre una cubierta no transitable y 2,00 m sobre una transitable. «Según el dibujo», es transitable un forjado de cubierta con locales o barandillas en su planta; las cubiertas inclinadas, no. En la ventana de saneamiento se puede decir «Transitables» o «No transitables» para todas;
  - si hay un aireador a menos de 6 m, más alta que él; si hay una ventana a menos de 6 m, 50 cm por encima de su dintel;
  - una **válvula de aireación** en su cabeza (el aparato nuevo «Válvula de aireación», a menos de 30 cm y a la altura de la cabeza) la sustituye (3.3.3.4).
  - La bajante que va por un patinillo (un hueco de los forjados) sale igual por la cubierta: se mira la cubierta de encima aunque el punto esté en el hueco.
- **Ventilación secundaria** (3.3.3.2): cuando a la columna desaguan 7 plantas o más (11 si está dibujada un diámetro por encima del exigido, «sobredimensionada»). Hace falta una **columna de ventilación** junto a ella: otra bajante dibujada a menos de 1 m, sin aparatos, que cubra todas las plantas que sirve, de al menos la mitad de su diámetro. También vale una válvula de aireación en el ramal de cada planta.
- **Ventilación terciaria** (3.3.3.3): en el ramal de una planta de más de 5 m (del aparato más lejano a la bajante) o en todos si la columna sirve a más de 14 plantas. Vale una válvula de aireación sobre el ramal o una columna de ventilación junto a su final, de al menos la mitad de su diámetro.
- **Cierres hidráulicos** (3.3.1.1). Cada aparato lleva su sifón, salvo los que desaguan por un **bote sifónico** (el aparato nuevo «Bote sifónico», dibujado sobre la tubería). La altura del cierre se escribe en la ventana (por defecto 50 mm) y debe estar entre 50 y 100 mm. Cada bote sifónico:
  - solo con aparatos de su mismo local y sin inodoros;
  - las derivaciones hasta él, de 2,50 m como mucho y entre el 2 y el 4 %;
  - a 2,00 m como mucho de la bajante.
- **Cada aparato con sifón propio, hasta la bajante** (3.3.1.4), por las tuberías dibujadas:
  - lavabos, bidés, fregaderos, lavavajillas y lavadoras: 4 m como mucho, su derivación entre el 2,5 y el 5 %;
  - bañeras y duchas: 4 m, su derivación hasta el 10 %;
  - inodoros: 1 m como mucho (desagüe directo o manguetón).
  - Si no hay bajante en su camino (la planta baja), hasta el primer colector.
  - La pendiente es la de su derivación propia; la del ramal que comparte con otros va por la tabla 4.3.

Sale en el informe de la ventana (al final), en la memoria de instalaciones («Ventilación y pequeña evacuación») y en la Comprobación CTE. **«Arreglar lo que no cumple»** prolonga cada bajante que no llega hasta la altura de su ventilación primaria, de 5 en 5 cm.

Los aparatos nuevos están en la ficha «Montante» de Instalaciones, en su «Tipo».

**Florence Street.** Con lo dibujado:
- Primaria: no llega ninguna. Las bajantes acaban 1 m sobre la terraza de cubierta, que es transitable (exige 2,00 m: faltan 1,00 m en 4 bajantes), y las 5 que están bajo el pabellón acaban dentro de él (faltan 3,50 m para salir 1,30 m sobre su forjado). Tres bajantes (1, 7 y 10) quedan dibujadas 39 cm por detrás de la fachada trasera, fuera de cualquier forjado: no se comprueban. «Arreglar lo que no cumple» prolonga las nueve.
- Secundaria: no hace falta (4 o 5 plantas por bajante).
- Terciaria: dos ramales largos sin ventilar, el de la planta baja de la bajante 7 (7,40 m) y el de la lavadora de la cubierta en la bajante 11 (5,25 m).
- Sifones: todos individuales, 50 mm: cumple.
- Derivaciones: 111 de 130 aparatos cumplen. No cumplen 17 inodoros, a entre 1,10 y 1,89 m de la bajante en las plantas tipo (4,10 m en la baja), y dos lavadoras de la cubierta, a 4,55 y 5,25 m.

**A comprobar por Antonio**: todos los valores de esta parte están escritos de memoria del texto del HS 5 y juntos al principio de `crates/archisim-mep/src/drain_vent.rs`. En particular, las 11 plantas con la bajante sobredimensionada (el encargo decía 14; recuerdo 11), los 4 m de las bañeras y duchas (el texto que recuerdo solo da su pendiente), el 2 % mínimo que les pongo, la mitad del diámetro para las columnas de ventilación y la válvula en cada ramal como sustituta de la secundaria.

### Esquemas de saneamiento

Igual que los de fontanería:
- **SA-01, esquema vertical.** Residuales a la izquierda y pluviales, a trazos, a la derecha. Abajo cada acometida con lo que recoge; las bajantes subiendo con el diámetro de cada trozo, y en cada planta lo que entra en cada bajante (diámetro y UD, o los sumideros y su superficie). Con más de cuatro bajantes, cada una lleva su rótulo planta a planta en lugar de cajas.
- **SA-02 en adelante.** Una hoja por planta con el árbol de cada ramal hasta cada aparato (con sus UD) o sumidero (con su superficie).

PDF en Archivo → Exportar → Esquemas de saneamiento o desde la ventana; DXF desde la ventana. En la línea de órdenes: `archisim saneamiento <proyecto> [--diametros salida.archisim] [--esquemas carpeta]`.

El edificio de Florence Street lleva su red:
- **Residuales:** una bajante por cada núcleo húmedo de las viviendas, cada aparato unido a la suya por el trazado automático, y colectores enterrados bajo la planta baja hasta la acometida de la fachada. Son 130 aparatos y 344 UD.
- **Pluviales:** cuatro sumideros en la terraza de la cubierta y cuatro en la del pabellón, con dos bajantes y su propio colector, y los dos balcones de la planta 1 con su sumidero y su bajante.

Con lo dibujado se quedan cortos 28 tramos, casi todos ramales de las cocinas (fregadero, lavavajillas y lavadora por una misma tubería de Ø40). Con los diámetros calculados no queda ninguno.

**A comprobar por Antonio**: los valores de las tablas 4.1 (uso público), 4.3, 4.4, 4.5, 4.6, 4.7, 4.8 y 4.9 del HS 5 están escritos de memoria del texto del CTE y no los he podido contrastar con el documento oficial desde aquí; están juntos al principio de `crates/archisim-mep/src/drain.rs` para corregirlos de una vez.

## 6. Ventilación de las viviendas (DB-HS 3)

Colaboración → **Cálculo de ventilación…**. Casi todo sale del dibujo. Arriba de la ventana se eligen dos datos (se guardan en el proyecto):
- el **sistema**: mecánica (por defecto) o híbrida (3.1.1: las viviendas llevan una de las dos);
- en híbrida, la **zona térmica** de la tabla 4.4 (W, X, Y o Z). Se puede elegir directamente o «Por provincia…», hasta 800 m o por encima. Por defecto es Z, la de Huelva.

Solo se calcula en edificios de viviendas (uso del edificio en los datos del CTE); los demás usos se ventilan por el RITE (IT 1.1.4.2), que no se calcula todavía.

**Qué se dibuja.** Cuatro aparatos nuevos, en el «Tipo» del aparato de la ficha Instalaciones:
- **Aireador**: en el muro o sobre la ventana de cada local seco. Su abertura libre es ancho × alto: el de serie, 400 × 20 mm, da 80 cm².
- **Boca de extracción**: en el techo de cada local húmedo. Su abertura es la del círculo de su diámetro (Ø125: 123 cm²).
- **Conducto de extracción**: el conducto vertical entre plantas (el shunt). Redondo si ancho y fondo son iguales, rectangular si no. Con la herramienta Conducto del sistema Extracción, RePág y AvPág lo ponen como el montante de una tubería.
- **Extractor**: en la cubierta, a la cabeza del conducto, unido a él por un trozo de conducto. En ventilación híbrida, el mismo aparato hace de **aspirador híbrido**.

Los conductos de cada planta, del sistema **Extracción**, van de cada boca al conducto vertical. Una boca está unida si un conducto acaba a menos de 45 cm.

**Cómo se calcula**
- **Las viviendas.** Son los locales de cada planta unidos por puertas, sin los comunes: pasillos y escaleras del edificio, portal, garaje, trasteros, cuartos de instalaciones y terrazas. Lo que es cada local se lee de su uso y su nombre:
  - **secos**: dormitorios, estar, salón, comedor, estudio y despacho;
  - **húmedos**: cocinas, baños, aseos y lavaderos;
  - **paso**: pasillos, distribuidores, vestíbulos, recibidores y vestidores.
  
  Un «estar-cocina» es las dos cosas: el aire entra por su parte de estar y sale por la cocina. Un hueco sin puerta une dos locales como una puerta.
- **Caudales**, tabla 2.1, según los dormitorios de la vivienda:

  | Dormitorios | Dormitorio principal | Otros dormitorios | Cada estar o comedor | Húmedos, en total | Cada húmedo |
  |---|---|---|---|---|---|
  | 0 o 1 | 8 l/s | — | 6 l/s | 12 l/s | 6 l/s |
  | 2 | 8 l/s | 4 l/s | 8 l/s | 24 l/s | 7 l/s |
  | 3 o más | 8 l/s | 4 l/s | 10 l/s | 33 l/s | 8 l/s |

  - El dormitorio principal es el mayor. El estudio o despacho cuenta como un dormitorio, también para el tipo de vivienda (el CTE no los nombra, pero computan como dormitorio).
  - Lo que entra tiene que salir: el caudal de la vivienda es el mayor de lo que piden los secos y los húmedos, y el otro lado se sube a él en proporción.
- **Aberturas**, tabla 4.1:
  - admisión en cada seco: 4·q cm²;
  - extracción en cada húmedo: 4·q cm²;
  - paso en cada puerta por la que va el aire: 8·q cm², y 70 cm² como mínimo. El aire de cada seco va, puerta a puerta, al húmedo más cercano que aún lo necesita. Una puerta da su holgura bajo la hoja, que se toma de 1 cm (paso libre × 1 cm); un hueco sin puerta, todo su tamaño. Si no basta, se dice la holgura que haría falta («holgura de 14 mm o rejilla de paso»).
- **Conductos.** Cada tramo lleva el caudal de las bocas que tiene detrás. Los tramos («V1», «V2»…) van del extractor hacia las bocas, y se da el tamaño que basta: el mismo alto con más ancho en los rectangulares, el diámetro siguiente en los redondos.
  - **Ventilación mecánica** (4.2.2): 2,5·q cm² en los conductos junto a los locales (fórmula 4.1) y 1,5·q cm² en los de la cubierta, por encima de la última planta con viviendas (fórmula 4.2).
  - **Ventilación híbrida** (4.2.1): la sección de la tabla 4.2 según el caudal del tramo y la clase de tiro del conducto. La clase sale de la tabla 4.3 con dos datos: las plantas que vierten al conducto (desde la más baja con una boca en él hasta la última, ambas incluidas) y la zona térmica.

    | Plantas | W | X | Y | Z |
    |---|---|---|---|---|
    | 1 | T-3 | T-3 | T-4 | T-4 |
    | 2 | T-2 | T-3 | T-3 | T-4 |
    | 3 | T-2 | T-2 | T-3 | T-3 |
    | 4 | T-2 | T-2 | T-2 | T-3 |
    | 5 | T-1 | T-2 | T-2 | T-3 |
    | 6 | T-1 | T-1 | T-2 | T-3 |
    | 7 | T-1 | T-1 | T-1 | T-2 |
    | 8 o más | T-1 | T-1 | T-1 | T-1 |

    | Caudal del tramo | T-1 | T-2 | T-3 | T-4 |
    |---|---|---|---|---|
    | hasta 100 l/s | 225 cm² | 400 | 625 | 625 |
    | 100 a 300 | 400 | 625 | 625 | 900 |
    | 300 a 500 | 625 | 900 | 900 | 2 × 900 |
    | 500 a 750 | 625 | 900 | 900 + 625 | 3 × 900 |
    | 750 a 1000 | 900 | 900 + 625 | 2 × 900 | 3 × 900 + 625 |

    Cuando la tabla pide más de un conducto, se dice («2 × 900 cm² (añadir conductos)») y no se agranda solo: hay que dibujar los conductos que faltan. Los conductos de los garajes y de las campanas son siempre mecánicos.
- **Extractor** (o aspirador híbrido): el caudal de su red, en l/s y m³/h. Si no está dibujado, no cumple. La presión que debe dar la pone el fabricante con el trazado.
- **Salida de cada conducto** (3.2.1). La salida es el extractor (su parte de arriba) o, en un conducto sin extractor como el de las campanas, su extremo libre más alto.
  - **Siempre:** a 3 m al menos de cualquier entrada de aire: ventanas, puertas exteriores, aireadores y terrazas, balcones o galerías (los locales con ese nombre, de su suelo a 2 m de altura). La distancia se mide en el espacio, al punto más cercano de cada hueco.
  - **En ventilación híbrida, además:**
    - 1 m al menos sobre la cubierta, o 2 m si es transitable (hay locales o una barandilla en esa planta);
    - más alta que los muros que haya a 2–10 m;
    - 1,3 veces la altura de los que estén a 2 m o menos.

    Se dice qué muro es el que manda.
- **Campana de la cocina** (3.1.1): 50 l/s por un conducto propio, que no puede ser el de la ventilación general. La campana es el objeto «Campana extractora» del catálogo, dentro de la cocina; su conducto se dibuja con la herramienta Conducto del sistema Extracción, desde la campana (a menos de 60 cm) hasta un conducto vertical propio que sube a la cubierta y sale por un trozo de conducto. Los conductos de las campanas de varias viviendas pueden ir a un mismo conducto vertical colectivo, sin bocas de otros locales. Cada tramo necesita 2,5·q cm²: 125 cm² para una campana (un Ø150 basta). No cumple la campana sin conducto o la que va a un conducto con bocas; se avisa de la cocina sin campana dibujada. Sus tramos se llaman «VC1», «VC2»…
- **Trasteros y almacenes de residuos** del edificio, tabla 2.2: 0,7 y 10 l/s por m². Si tienen bocas dibujadas, se comprueban contra 4·q cm² y su caudal entra en los conductos.
- **Aparcamientos y garajes** (los locales llamados garaje, aparcamiento o parking; los de bicicletas o motos no). Tabla 2.2: 120 l/s por plaza, contando los coches dibujados dentro o, si no hay ninguno, una plaza cada 25 m². Si tiene bocas dibujadas, se le aplica lo de la ventilación mecánica de 3.1.4:
  - una boca de extracción y una abertura de admisión por cada 100 m²;
  - las bocas de extracción a menos de 10 m unas de otras;
  - desde 15 plazas, dos redes de extracción en la planta, cada una con su extractor;
  - la red solo para el aparcamiento: no cumple si lleva bocas de otros locales o campanas.

  Se avisa:
  - de la detección de monóxido de carbono a partir de 5 plazas o 100 m² (que arranque los extractores a 50 ppm, o a 100 ppm sin personal);
  - si no tiene nada dibujado: puede ir con ventilación natural, por aberturas mixtas en dos fachadas opuestas, que no se comprueba todavía.

Todo sale en la ventana, en la Comprobación CTE (sección «Ventilación (DB-HS 3)») y en `archisim ventilacion <proyecto>`. **«Poner los tamaños calculados»** alarga los aireadores cortos (con su misma rendija) y agranda las bocas, conductos y conductos verticales cortos, en un paso de Deshacer. Lo mismo hace «Arreglar lo que no cumple» y `archisim ventilacion <proyecto> --tamanos salida.archisim`. Los pasos de las puertas no se tocan: holgura o rejilla, a decidir.

`archisim ventilacion --ejemplo <carpeta>` hace un piso de dos dormitorios con todo dibujado, también la campana con su conducto.

### Esquemas de ventilación

Hojas A3 como las de fontanería y saneamiento:
- **VE-01, esquema vertical.** A la izquierda, la extracción general: los conductos verticales subiendo planta a planta con su tramo y su sección, en cada planta lo que entra en cada uno (bocas de cocinas y baños con su caudal) y arriba los extractores con su caudal en m³/h. A la derecha, a trazos, los conductos de las campanas hasta su salida en cubierta.
- **VE-02 en adelante.** Una hoja por planta con el árbol de conductos hasta cada boca (local y l/s) y cada campana.

En rojo, lo dibujado que se queda corto («100 × 100→150 × 100»). PDF desde la ventana del cálculo o en Archivo → Exportar → Esquemas de ventilación; DXF desde la ventana; en la línea de órdenes, `archisim ventilacion <proyecto> --esquemas <carpeta>`.

El edificio de Florence Street lleva su ventilación:
- un aireador sobre cada ventana de los locales secos, o en la fachada de vidrio de los estares;
- una boca en cada local húmedo;
- conductos por el techo hasta un conducto vertical Ø200 junto al baño de cada vivienda, que sube de la planta 1 a la cubierta;
- un extractor en la cubierta por cada conducto vertical, llevado por un conducto sobre el forjado (no sobre los patios) al sitio más cercano que quede a 3 m de los aireadores y ventanas de la planta 4 y de la puerta del castillete; las salidas de las campanas, igual. Las 12 salidas cumplen;
- la campana de cada cocina, con su conducto por el techo hasta un conducto vertical colectivo de campanas por cada columna de viviendas (Ø300 para las cuatro cocinas que recoge), que sale a la cubierta. Las 24 campanas cumplen.

El aparcamiento de bicicletas de la planta baja no se toma por garaje (no son plazas de coche de la tabla 2.2). Son 24 viviendas y 6 redes de extracción (de 202 a 346 m³/h). Para el cálculo le faltaba un local: el vestíbulo de la vivienda D, que ahora está (y sale en el cuadro de superficies). Con lo dibujado solo no cumplen 12 pasos de puerta: los que llevan más de 9 l/s piden holguras de 11 a 14 mm o rejilla de paso.

Las tablas 2.1, 2.2, 4.1, 4.2, 4.3 y 4.4 y los apartados 3.1.4, 3.2.1 y 4.2.2 están contrastados con el texto del HS 3 (edición de 2022). Están juntos al principio de `crates/archisim-mep/src/ventilation.rs`. Antonio dio por buenas, el 7/11/2026, la holgura de 1 cm bajo las puertas y que el estudio o despacho cuente como dormitorio.

**A comprobar por Antonio:**
- las plantas de la tabla 4.3: se cuentan hasta la última planta que vierte al conducto, no hasta la última del edificio; así sale más sección;
- la plaza de 25 m² cuando no hay coches dibujados;
- que una cubierta es transitable si hay en ella algún local o una barandilla.

## 7. Envolvente (DB-HE 1) y cargas térmicas

Colaboración → **Envolvente y cargas térmicas…**. En la ventana se escribe lo que no sale del dibujo:
- la **zona climática** del municipio (DB-HE, anejo B; por defecto A4, la de Huelva);
- las **temperaturas** de cálculo: exterior en invierno y en verano (el botón «Temperaturas de la zona» pone unas habituales) e interior calefactado (21 °C) y refrigerado (25 °C);
- la **transmitancia de las ventanas** (marco y vidrio juntos; 2,7 W/m²K por defecto) y **de las puertas exteriores** (2,2), y el **factor solar** del vidrio con su protección (0,6);
- la **conductividad** de cada material de la envolvente. Sale la habitual por su nombre y su clase (lana mineral 0,040; XPS 0,034; ladrillo perforado 0,50; hueco 0,43; hormigón armado 2,5; enlucido de yeso 0,57…). Se puede cambiar por la del producto, y «Habitual» la devuelve.

**La envolvente de cada local** sale de su contorno:
- **Lados al exterior.** Un lado da al exterior si a 60 cm por fuera no hay otro local de la planta (se mira cada 25 cm a lo largo). Lleva su muro, con las ventanas y puertas que caen en ese lado, o el muro cortina si lo hay.
- **Lo que se deja fuera:**
  - los lados contra otro local (calefactado o no);
  - los muros interiores sin ventanas, que dan a una escalera o un patinillo;
  - las medianeras.
  
  Un muro interior con ventana se toma como de patio de luces.
- **Arriba**, el forjado de la planta de encima o una cubierta, si no hay otro local encima.
- **Abajo**, en la planta baja, la solera contra el terreno; en las demás plantas, el forjado que vuela sobre el exterior.

**Transmitancias.** U = 1/(Rsi + Σ e/λ + Rse) con las capas del tipo:
- resistencias superficiales de 0,13 y 0,04 en muros, 0,10 hacia arriba en cubiertas y 0,17 hacia abajo en suelos;
- una cámara de aire cuenta 0,17 m²K/W.

**Contra el terreno (UNE-EN ISO 13370).**
- **Solera.** La U no es la de sus capas solas: es la U equivalente de la norma, planta por planta.
  - A es la superficie de los locales acondicionados sobre el terreno.
  - P es su perímetro expuesto: los lados a la fachada y los lados a locales sin acondicionar de la misma planta (portal, garaje). Las medianeras no cuentan.
  - B' = A / (0,5·P); el espesor equivalente dt = w + λ·(Rsi + Rf + Rse), con w el grueso medio de la fachada (un muro cortina cuenta 15 cm) y λ la conductividad del terreno (2,0 W/mK por defecto, arena o grava; 1,5 arcilla, 3,5 roca).
  - U = 2λ/(πB' + dt)·ln(πB'/dt + 1) si dt < B'; si no, λ/(0,457·B' + dt).
  - **Aislamiento perimetral**, si lo hay (en la ventana: vertical u horizontal, profundidad o ancho, resistencia y espesor): ψ = −λ/π·[ln(D/dt + 1) − ln(D/(dt + d') + 1)], con 2D si es vertical, y U' = U + 2ψ/B'.
- **Sótanos.** Una planta por debajo del ±0,00 está a esa profundidad z bajo el terreno:
  - su suelo, con dt + 0,5·z;
  - sus muros de fachada, hasta la altura enterrada, son «muro contra el terreno»: U = 2λ/(πz)·(1 + 0,5·dt/(dt + z))·ln(z/dw + 1). Lo que asoma por encima es muro de fachada.

**Puentes térmicos lineales.** Las longitudes se miden en el dibujo, local a local:
- **frente de forjado**: a lo largo de cada fachada, en el suelo de cada planta que no es la del terreno;
- **pilar en fachada**: cada pilar dentro de la banda de un muro de fachada, una vez, con la altura del local;
- **contorno de huecos**: jambas, dintel y alféizar (o umbral) de cada ventana y puerta exterior, 2·(ancho + alto);
- **esquinas salientes y entrantes**: donde se encuentran dos lados de fachada del local, con su altura;
- **encuentro con la cubierta** a lo largo de la fachada si el local tiene cubierta encima, y **con la solera** si está sobre el terreno.

Cada clase lleva un ψ según dónde esté el aislamiento del muro, que se deduce de sus capas: una hoja (4 cm o más de algo que no aísla) a los dos lados del aislante es aislamiento **en cámara**; solo hacia dentro, aislamiento **por fuera**; solo hacia fuera, **por dentro**; sin capa aislante, **sin aislar**. Valores por defecto (W/mK, con las medidas interiores de los locales):

| Encuentro | Por fuera | Por dentro | En cámara | Sin aislar |
|---|---|---|---|---|
| Frente de forjado | 0,10 | 0,85 | 0,80 | 0,90 |
| Pilar en fachada | 0,05 | 0,90 | 0,80 | 0,90 |
| Contorno de huecos | 0,25 | 0,15 | 0,20 | 0,30 |
| Esquina saliente | 0,10 | 0,05 | 0,08 | 0,15 |
| Esquina entrante | −0,05 | −0,05 | −0,05 | −0,10 |
| Encuentro con la cubierta | 0,40 | 0,60 | 0,55 | 0,70 |
| Encuentro con la solera | 0,15 | 0,55 | 0,50 | 0,70 |

En la ventana, el grupo «Puentes térmicos» enseña las clases que hay en el dibujo con su longitud. El ψ de cada una se cambia por el del detalle del proyecto, y «Habitual» lo devuelve.

**DB-HE 1.**
- Cada tipo de la envolvente contra su límite de la tabla 3.1.1.a-HE1, por la letra de invierno:
  - muros y suelos al exterior: 0,80 · 0,70 · 0,56 · 0,49 · 0,41 · 0,37;
  - cubiertas: 0,55 · 0,50 · 0,44 · 0,40 · 0,35 · 0,33;
  - contra el terreno (suelos y muros, con su U equivalente): 0,90 · 0,80 · 0,75 · 0,70 · 0,65 · 0,59;
  - huecos: 3,2 · 2,7 · 2,3 · 2,1 · 1,8 · 1,8;
  - puertas: 5,7.
- El **coeficiente global K** = (Σ U·A + Σ ψ·L) / Σ A de los locales calefactados, contra la tabla 3.1.1.b-HE1 de vivienda nueva según la compacidad V/A (entre V/A = 1 y 4 en línea recta).
- **Control solar (3.1.2).** q sol;jul = Σ F sh,obst · g gl;sh;wi · (1 − F F) · A · H sol;jul / A útil:
  - H sol;jul, la radiación de julio por orientación y por la cifra de verano de la zona (en la zona 4: N 58, E y O 145, S 95, cenital 245 kWh/m²·mes);
  - g gl;sh;wi, el factor solar del vidrio con la protección móvil echada (0,15 por defecto, persiana exterior), y F F la fracción de marco (25 %);
  - F sh,obst, la sombra de lo que vuela sobre cada ventana: se busca hacia fuera de la fachada, cada 10 cm hasta 3 m, un forjado o balcón de la planta de encima o el alero de una cubierta, y se suma el retranqueo del marco. Con el ángulo del sol de julio en esa fachada (al sur, la altura a mediodía por la latitud; al este y oeste, 50°; al norte, 72°) sale la parte del vidrio en sombra, y deja fuera la parte directa de la radiación (S 60 %, E y O 70 %, N 30 %);
  - límite: 2,00 kWh/m²·mes en residencial privado y 4,00 en los demás usos.
- **Permeabilidad al aire (3.1.3).**
  - Cada tipo de ventana con su clase (UNE-EN 12207; la general del proyecto o la de cada tipo): a 100 Pa, clase 1 50, clase 2 27, clase 3 9, clase 4 3 m³/h·m². Límite de la tabla 3.1.3.a: 27 en las zonas α, A y B, y 9 en C, D y E.
  - **n50** por el método simplificado del anejo H: n50 = 0,629·(Co·Ao + Σ Ch·Ah) / V, con Co = 16 m³/h·m² (edificio nuevo; 29 existente) sobre la parte opaca al aire (muros, cubiertas y suelos al exterior; no lo que toca el terreno) y Ch la de la clase de cada hueco. Si se ha medido con un ensayo, se escribe en la ventana. Límite de la tabla 3.1.3.b para viviendas de más de 120 m² útiles: 6 h⁻¹ con V/A ≤ 2 y 3 con V/A ≥ 4, en línea recta entre medias.
- **Condensaciones superficiales.** fRsi = 1 − 0,25·U de cada tipo opaco (con la U de sus capas) y el fRsi de cada clase de puente térmico, contra fRsi,min por la clase de higrometría y la zona: 0,50 · 0,50 · 0,52 · 0,56 · 0,61 · 0,64 con higrometría 3 o menos (viviendas), 0,85 con 4 y 0,90 con 5. Los fRsi de los puentes por defecto:

| Encuentro | Por fuera | Por dentro | En cámara | Sin aislar |
|---|---|---|---|---|
| Frente de forjado | 0,90 | 0,65 | 0,70 | 0,60 |
| Pilar en fachada | 0,90 | 0,60 | 0,65 | 0,55 |
| Contorno de huecos | 0,75 | 0,75 | 0,75 | 0,65 |
| Esquina saliente | 0,85 | 0,80 | 0,80 | 0,70 |
| Esquina entrante | 0,95 | 0,95 | 0,95 | 0,90 |
| Encuentro con la cubierta | 0,80 | 0,70 | 0,70 | 0,60 |
| Encuentro con la solera | 0,85 | 0,70 | 0,70 | 0,65 |

**Cargas de cada local**
- **Calefacción:**
  - transmisión U·A·ΔT, con la mitad de la diferencia contra el terreno;
  - los puentes térmicos, ψ·L·ΔT;
  - ventilación 1,2·q·ΔT, con el caudal de admisión del DB-HS 3, o media renovación a la hora si el local no es de una vivienda.
- **Refrigeración:**
  - la misma transmisión con la diferencia de verano, con los puentes salvo el de la solera;
  - el sol por los vidrios: 100 W/m² al norte, 450 al este, 350 al sur, 500 al oeste y 750 en lucernarios, por el factor solar;
  - las personas: una por cada 10 m² en los locales vivideros, con 75 W sensibles y 55 latentes;
  - luz y aparatos: 5 + 3 W/m², o 5 + 20 en las cocinas;
  - la ventilación.
- **Equipos:**
  - una unidad interior en cada local vividero (de 2,0 a 7,1 kW) y una bomba de calor por vivienda (o por local, si no es de una vivienda), de 3,5 a 20 kW;
  - los radiadores dibujados se comparan con la carga de calefacción de su local, a 2000 W por m² de frente.

En la ventana se escriben también: el factor solar con la protección móvil, la fracción de marco, la clase de permeabilidad de las ventanas (y la de cada tipo), la n50 medida si la hay, el caudal de la parte opaca, la clase de higrometría, la conductividad del terreno y el aislamiento perimetral de la solera.

Todo sale en la ventana, en la Comprobación CTE (sección «Ahorro de energía (DB-HE 1) y cargas térmicas»), en la memoria de instalaciones y en `archisim climatizacion <proyecto>`.

El edificio de Florence Street, en A4:
- **Cumplen** el ladrillo de fachada, el muro cortina, las ventanas y la cubierta transitable; también la solera, que con la UNE-EN ISO 13370 baja de 2,87 a 0,58 W/m²K (A = 142 m², P = 45 m con los lados al portal, B' = 6,3 m).
- **No cumplen** el hormigón visto y los tabiques de los patios (U de 2,6 a 3,7) y la losa vista; además condensan en su cara interior (fRsi 0,07, 0,35 y 0,43, contra 0,50).
- **Puentes térmicos:** 90 W/K, sobre todo los frentes de forjado de los muros sin aislar (53 m) y el contorno de los huecos (112 m).
- **K** da 1,97 contra 0,80 (antes 2,27: la solera pesa menos y entran los puentes).
- **Control solar:** q sol;jul = 1,88 kWh/m²·mes, justo por debajo de 2,00. Lo marca sobre todo el muro cortina (unos 290 m² de vidrio al norte y al sur), al que no se le calcula sombra; de las 22 ventanas, 20 tienen un forjado o un balcón encima.
- **Permeabilidad:** todas las ventanas de clase 3 cumplen, y n50 sale 1,29 h⁻¹ (límite 3).

Los tipos de ese edificio están hechos para dibujarlo, no para cumplir el HE: para que cumpla hay que darles su aislamiento. En total salen 34 kW de calefacción y 101 kW de refrigeración, con una bomba de calor por vivienda y otra por cada local de la planta baja.

**A comprobar por Antonio**: están escritos de memoria y hay que contrastarlos con el CTE y sus documentos de apoyo:
- los límites de las tablas 3.1.1.a y 3.1.1.b del HE 1 (versión de 2019), las conductividades habituales, la radiación por orientación y las cargas interiores (al principio de `crates/archisim-mep/src/thermal.rs` y `crates/archisim-core/src/model/thermal.rs`);
- los ψ y fRsi por defecto de los puentes (DA DB-HE/3), la radiación de julio por zona (DA DB-HE/1), el límite de q sol;jul (2,00 en residencial privado, 4,00 en los demás), las permeabilidades de las clases y los límites de la tabla 3.1.3.a, el anejo H (0,629, Co 16 y 29) y la tabla 3.1.3.b (6 y 3 h⁻¹), y los fRsi,min (DA DB-HE/2). Están juntos al principio de `crates/archisim-mep/src/envelope.rs`.

## 8. Energías renovables (DB-HE 4 y HE 5)

Colaboración → **Energías renovables…** (o `archisim renovables <proyecto>`). Arriba, lo que el dibujo no dice:
- si el edificio es nuevo (o una ampliación o reforma entera): si no, no se aplican;
- la provincia y cuántos metros está el municipio por encima de su capital, para el agua fría de cada mes;
- la temperatura de acumulación;
- con qué es renovable el agua caliente: bomba de calor (con su SCOP), solar térmica u otra (con su fracción renovable), o ninguna;
- la potencia de las placas por m² (0,2 kWp/m²: 400 W en 2 m²);
- a mano, si hace falta, la demanda de agua caliente (otros usos que no son viviendas) y la superficie construida.

**HE 4, agua caliente.**
- Demanda del Anejo F: las viviendas del dibujo (las mismas que reconoce la ventilación), las personas por sus dormitorios (tabla a: 1 dormitorio 1,5 personas, 2 → 3, 3 → 4, 4 → 5, 5 y 6 → 6, más → 7), 28 l por persona a 60 °C y el factor de centralización por el número de viviendas (tabla b: hasta 3, 1; de 4 a 10, 0,95… más de 100, 0,70).
- Energía de cada mes con el agua fría de la capital de la provincia (Anejo G), bajada 0,0066 °C por metro de octubre a marzo y 0,0033 de abril a septiembre.
- Se aplica con más de 100 l/día. Se exige el 60 % renovable por debajo de 5.000 l/día y el 70 % por encima.
- La bomba de calor cuenta 1 − 1/SCOP de lo que calienta, y nada si su SCOP es de 2,5 o menos.
- **Solar térmica: fracción por f-Chart.** Con captadores solares dibujados (aparato «Captadores solares»), la fracción solar se calcula mes a mes por el método f-Chart. Sin captadores dibujados vale la que se da.
  - **Los captadores.**
    - Su área es la de los captadores a lo largo de su pendiente.
    - Su inclinación sale de su caja: lo que suben sobre su fondo.
    - Su orientación es hacia donde mira su frente bajo, con el norte del proyecto.
    - De la ficha del captador se dan η0 (rendimiento óptico; 0,75 por defecto) y a1 (pérdidas; 3,5 W/m²K).
  - **El sol sobre ellos.** Se parte de la radiación media diaria sobre el horizontal de cada mes.
    - Se pasa al plano de los captadores con el cielo isótropo de Liu y Jordan, con la parte difusa de Erbs. La radiación directa se calcula hora a hora, así que vale cualquier inclinación y orientación.
    - La latitud es la de la georreferencia o, si no hay, la de la capital de la provincia.
  - **El clima** se puede dar de tres maneras en la ventana:
    - leído de un fichero mensual de PVGIS (herramienta «Monthly radiation», con la temperatura);
    - escrito a mano mes a mes;
    - estimado: la radiación por la latitud con una claridad media de la península, y el aire por el agua fría del Anejo G más 2 a 6 °C. Si se usa la estimación, la comprobación avisa y la memoria lo dice.
  - **El método**, cada mes:
    - D1 = A·F'R(τα)·H·N/Q y D2 = A·F'R·UL·(100 − Ta)·Δt·K1·K2/Q;
    - con (τα)/(τα)n = 0,96, F'R/FR = 0,95, K1 por los litros de acumulación por m² (frente a 75) y K2 con el agua de uso a 45 °C y la fría del mes;
    - f = 1,029·D1 − 0,065·D2 − 0,245·D1² + 0,0018·D2² + 0,0215·D1³, entre 0 y 1.
  - La carga de cada mes es el agua más las pérdidas. La fracción anual es la energía solar entre la carga del año.
  - Los meses en que los captadores darían más de lo que se gasta salen como aviso de sobrecalentamiento.
  - En la línea de órdenes, `archisim renovables <proyecto> --pvgis fichero.csv` usa el clima de ese fichero sin cambiar el proyecto.
- **Pérdidas** (HE 4, 4.1: la demanda «incrementada» con ellas), sacadas de la red de agua caliente dibujada. La parte renovable se pide sobre el agua más las pérdidas:
  - **Recirculación.** Si la red necesita retorno (15 m o más del acumulador al grifo más lejano), pierden calor todo el año la ida hasta el grifo más lejano y el retorno dibujado. Si el retorno no está dibujado, se toma tan largo como la ida. La pérdida es de 0,25 W/mK (tubería aislada según el RITE), a la temperatura de acumulación menos 2,5 °C, en un local a 20 °C.
  - **Distribución.** En el resto de la tubería caliente, la que no tiene retorno, el agua que queda dentro se enfría hasta 20 °C tres veces al día.
  - **Acumulación.** La pérdida estática de un acumulador de clase C del etiquetado energético, 16,66 + 8,33·V^0,4 W a 45 K, a su temperatura.
  - El acumulador tiene el volumen que se da en la ventana o, si no se da, la demanda de un día.

**HE 5, electricidad.** Se aplica con más de 1.000 m² construidos. Potencia mínima P = mín(Fpr,el · S; 0,1 · (0,5 · Sc − Soc)):
- S: la superficie construida (la del cuadro de superficies);
- Fpr,el: 0,005 en viviendas y 0,010 en los demás usos;
- Sc: la cubierta no transitable, que es la huella de las cubiertas inclinadas y los forjados de cada planta (menos la más baja) sin ningún local encima;
- Soc: la que ocupan los captadores solares térmicos dibujados.

Se compara con las placas fotovoltaicas dibujadas: su superficie por la potencia por m².

El consumo de energía primaria (**HE 0**) no se calcula: se justifica con la herramienta oficial (HULC o los procedimientos reconocidos). Sale como dato en la Comprobación CTE y en la memoria.

Antonio dio por buenos, el 7/11/2026, la parte renovable de la bomba de calor, la cubierta no transitable y los 0,2 kWp/m².

**A comprobar por Antonio:** la estimación del clima (claridad media de la península y aire sobre el agua fría), que solo debe servir mientras no se dé el del lugar; y las cifras de las pérdidas.
- 0,25 W/mK de la tubería aislada.
- Tres enfriamientos al día del agua sin retorno.
- Acumulador de clase C.
- Florence Street pierde 8.174 kWh en la recirculación (41,8 m de ida y 57,7 m de retorno dibujado), 3.377 kWh en la distribución y 1.158 kWh en el acumulador. Son 12.700 kWh sobre los 18.800 que calientan su agua: el 68 % de más. Sigue siendo renovable al 67 %.

Las tablas están al principio de `crates/archisim-core/src/model/renewables.rs` y `crates/archisim-mep/src/renewables.rs`.

## 9. Memoria de instalaciones

Archivo → Exportar → **Memoria de instalaciones** (o `archisim memoria-instalaciones <proyecto> [carpeta]`, que deja el PDF y el mismo texto en `.txt`). Hojas A4, con portada y un capítulo por cada instalación dibujada:
1. **Instalación eléctrica (REBT)**: por cada cuadro, el grado de electrificación, la potencia, el IGA, la derivación individual y los diferenciales, y el cuadro de circuitos (puntos, potencia, intensidad, PIA, sección, longitud y caída de tensión).
2. **Suministro de agua (DB-HS 4)**: los datos de partida, la tabla de tramos de agua fría y caliente (caudal, diámetro dibujado y calculado, velocidad y pérdida), el grifo más desfavorable, el grupo de presión, el aljibe y el retorno.
3. **Evacuación de aguas (DB-HS 5)**: los tramos de cada red con sus UD o su superficie, la pendiente y el diámetro dibujado y exigido, los sumideros y los canalones.
4. **Calidad del aire interior (DB-HS 3)**: los caudales de cada vivienda, las aberturas de cada local, los pasos de las puertas, los conductos y las campanas.
5. **Envolvente térmica y cargas (DB-HE 1)**: la zona y las temperaturas, cada tipo de la envolvente con su U y su límite, el K, las cargas de cada local y los equipos.
6. **Energías renovables (DB-HE 4 y HE 5)**, cuando alguna se aplica: la demanda de agua caliente con sus viviendas, el agua fría, las pérdidas y la energía de cada mes, la parte renovable, y la potencia fotovoltaica mínima contra la dibujada.

Lo que no cumple sale marcado como en el anejo de cálculo de la estructura. La portada deja el hueco para el autor: los datos de partida los revisa y asume el técnico que firma.

## 10. Lo que no hace (todavía)

- Trifásica, viviendas con más de 14 490 W, locales y oficinas (ITC-BT-28 en adelante).
- Caída de tensión por tramos con la carga repartida: se pone toda la carga en el punto más lejano, lo que queda del lado de la seguridad.
- Cortocircuito y poder de corte (se pone 6 kA).
- Fontanería:
  - el volumen del depósito de presión del grupo;
  - los fluxores.
- Saneamiento:
  - los diámetros de las columnas de ventilación por su longitud (tablas 4.10 y 4.11): se pide la mitad del diámetro de la bajante o del ramal;
  - que la columna de ventilación esté unida a la bajante en cada planta (se mira solo que esté junto a ella y cubra sus plantas), y los obstáculos de la cubierta que no son tomas de aire ni ventanas;
  - los rebosaderos, los desagües enfrentados y el ángulo de las uniones a la bajante (3.3.1.4);
  - los aparatos de uso discontinuo, que piden 70 mm de cierre (se pone un solo valor para todos);
  - las bombas de elevación;
  - los canalones de sección cuadrada (se toman como semicirculares).
- Ventilación:
  - la ventilación natural de garajes, trasteros y cuartos de basuras (aberturas mixtas en fachadas opuestas, a 25 m de cualquier punto);
  - en los garajes, los dos tercios de bocas a menos de 0,5 m del techo y los garajes pequeños con aberturas en un mismo muro;
  - los obstáculos que no son muros (castilletes de cubierta hechos con otros elementos, cubiertas inclinadas cercanas);
  - los edificios que no son de viviendas (RITE).
- Envolvente y cargas:
  - las condensaciones intersticiales (método de Glaser con enero): harían falta los factores de resistencia al vapor de cada material;
  - los puentes térmicos de los muros cortina (montantes contra los forjados) y las esquinas entre un muro y un muro cortina;
  - el ψ que depende a la vez del aislamiento del muro y del de la cubierta o la solera: se toma el del muro;
  - las sombras de las aletas laterales, de los edificios vecinos y del entorno sobre las ventanas (solo lo que vuela encima y el retranqueo, y el retranqueo solo por arriba), y las de los muros cortina (sin sombra);
  - las orientaciones intermedias (NE, SE…): cada ventana va a la más cercana de las cuatro;
  - la permeabilidad de las puertas contra la tabla 3.1.3.a (cuentan en la n50);
  - el consumo de energía y las emisiones (HE 0);
  - los conductos y tuberías de la climatización, y las cargas hora a hora.
