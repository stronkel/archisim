# Instalaciones: dibujo y cálculo

ArchiSim dibuja las instalaciones (tuberías, conductos y aparatos) y las calcula sobre lo dibujado. Este documento recoge la segunda vuelta: cómo se meten las tuberías, el cálculo eléctrico con su esquema unifilar y el cálculo de fontanería. Lo que sigue (saneamiento HS 5, ventilación HS 3, climatización y DB-HE) se añadirá aquí.

Código: `archisim-core::model::pipe_routing` (uniones, cambio de planta y trazado automático), `archisim-core::model::water` (datos del suministro), crate `archisim-mep` (cálculos, depende solo de `archisim-core`: `electrical` y `water`), `archisim-plan::single_line` (esquema unifilar), `archisim-plan::water_schematic` (esquemas de fontanería), `archisim-cte::electrical` y `archisim-cte::services::calculation` (comprobación) y, en la app, `app/services_ui`, `app/electrical_ui` y `app/water_ui`.

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
- la **tubería**: multicapa (por defecto), PE-X, PP-R o cobre, con sus medidas comerciales, su rugosidad y su velocidad máxima.

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

En la línea de órdenes: `archisim fontaneria <proyecto>`, o `archisim fontaneria <proyecto> --diametros salida.archisim` para guardarlo con los diámetros calculados (y `--esquemas <carpeta>` para las hojas, abajo).

El edificio de Florence Street (`archisim edificio`) lleva ahora la red entera: contador y acumulador junto al montante, montante hasta la cubierta, distribuidor en cada rellano, derivación de cada vivienda y todos los aparatos unidos. Son 114 aparatos y 24 viviendas. Con 300 kPa en el contador, los diámetros dibujados dejan 95 grifos por debajo de 100 kPa; con los calculados, el peor llega a 122 kPa.

### Esquemas de fontanería

Hojas A3 sacadas del cálculo, con los **diámetros calculados**. Donde la tubería dibujada es menor se escriben los dos en rojo («Ø20→26»), y también en rojo los grifos sin presión con lo dibujado. El agua fría va en línea continua y la caliente a trazos.

- **FO-01, esquema vertical.** Las plantas una sobre otra, con su cota, y abajo el origen de cada red (contador o acumulador, con su presión). Los montantes suben de planta a planta con el tramo y el diámetro de cada trozo. En cada planta, una caja por cada derivación que sale del montante: qué aparatos alimenta, el tramo y el diámetro con que arranca, y la presión mínima de sus grifos. Las derivaciones grandes se parten hasta llegar a cada vivienda o local húmedo. Agua fría a la izquierda y caliente a la derecha.
- **FO-02 en adelante, distribución.** Una hoja por planta con aparatos: el árbol de cada derivación, tramo a tramo con su nombre y diámetro, hasta cada grifo con la presión que le llega.

Cómo sacarlos:
- PDF: Archivo → Exportar → Esquemas de fontanería, o desde la ventana del cálculo.
- DXF: desde la ventana, un archivo por hoja.
- En la línea de órdenes, `archisim fontaneria <proyecto> --esquemas <carpeta>` deja el PDF, y un SVG y un DXF por hoja.

## 5. Lo que no hace (todavía)

- Trifásica, viviendas con más de 14 490 W, locales y oficinas (ITC-BT-28 en adelante).
- Caída de tensión por tramos con la carga repartida: se pone toda la carga en el punto más lejano, lo que queda del lado de la seguridad.
- Cortocircuito y poder de corte (se pone 6 kA).
- Fontanería:
  - los grupos de presión y los aljibes dimensionados;
  - el retorno del agua caliente, obligatorio con más de 15 m de tubería al grifo más alejado;
  - las pérdidas propias de contadores, filtros y calentadores, que habría que restar a mano;
  - los fluxores.
- Saneamiento (HS 5, bajantes y colectores por unidades de desagüe), ventilación (HS 3), climatización y DB-HE: los siguientes pasos.
