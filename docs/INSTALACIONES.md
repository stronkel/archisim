# Instalaciones: dibujo y cálculo

ArchiSim dibuja las instalaciones (tuberías, conductos y aparatos) y las calcula sobre lo dibujado. Este documento recoge la segunda vuelta: cómo se meten las tuberías y el cálculo eléctrico con su esquema unifilar. Lo que sigue (fontanería HS 4, saneamiento HS 5, ventilación HS 3, climatización y DB-HE) se añadirá aquí.

Código: `archisim-core::model::pipe_routing` (uniones, cambio de planta y trazado automático), crate `archisim-mep` (cálculos, depende solo de `archisim-core`), `archisim-plan::single_line` (esquema unifilar), `archisim-cte::electrical` (comprobación) y, en la app, `app/services_ui` y `app/electrical_ui`.

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

## 4. Lo que no hace (todavía)

- Trifásica, viviendas con más de 14 490 W, locales y oficinas (ITC-BT-28 en adelante).
- Caída de tensión por tramos con la carga repartida: se pone toda la carga en el punto más lejano, lo que queda del lado de la seguridad.
- Cortocircuito y poder de corte (se pone 6 kA).
- Fontanería (HS 4), saneamiento (HS 5), ventilación (HS 3), climatización y DB-HE: los siguientes pasos.
