# Escaleras

Cómo dibuja, construye y comprueba ArchiSim las escaleras (v0.53, rama `bloque/escaleras`). Formato `.archisim` versión 33; las mesetas partidas y la meseta de llegada, versión 38 (v0.68).

## 1. Formas

| Forma | Cómo se dibuja | En los giros |
|---|---|---|
| **Recta** | Clic en el arranque y clic hacia donde sube | — |
| **En L** | Igual; gira a la izquierda o a la derecha (Espacio mientras se dibuja) | Meseta cuadrada, meseta partida a 45° o peldaños compensados |
| **En U (ida y vuelta)** | Igual, con el **ojo** (hueco entre tramos, 20 cm por defecto) | Meseta, mesetas partidas a 45° o peldaños compensados |
| **De caracol** | Clic en el centro y clic hacia donde arranca el primer peldaño | Da las vueltas que pida la altura; meseta de llegada |
| **Libre** | Clic en cada giro de la línea de huella, Intro para terminar | Meseta, partida o peldaños compensados |

Las formas hechas miden lo que pide la altura: con la tabica máxima (17,5 cm por defecto) salen las tabicas justas, todas iguales, y la huella que se pide en cada peldaño. En una L o una U con meseta, los dos tramos tienen las mismas tabicas o uno más el primero.

**Peldaños compensados.** El giro se reparte en los cuatro peldaños de antes y de después (si caben). Todas las huellas miden lo mismo en la **línea de huella**, que en los giros va a 50 cm del lado interior (en el centro, si la escalera de una vivienda tiene menos de 1 m), como mide el CTE. Por fuera, las huellas se reparten por igual; por dentro queda lo que resulta (si se cruzaran, se reparten por igual por dentro).

**Mesetas partidas a 45°** (v0.68). La meseta de cada giro en escuadra se parte por su diagonal, de la esquina interior a la exterior, en dos peldaños triangulares; la de una U, por sus dos diagonales, en tres (dos triángulos y el del ojo). La escalera es un solo tramo, con una tabica más por cada peldaño de la meseta: la forma hecha quita los peldaños que hagan falta de los tramos para seguir con las tabicas justas. Un giro que no es en escuadra conserva su meseta. Se elige en la herramienta (Giros → «Meseta partida a 45°»), en Propiedades de la escalera (Forma → «Mesetas partidas a 45°») o en la Maqueta (Construir → Escaleras → «En L con meseta partida»).

**De caracol.** Peldaños desde el mástil (o el ojo) de radio que se pida hasta el ancho; la huella se mide a 50 cm del mástil. Con la de por defecto (ancho 80 cm, mástil de 12 cm, huella 24 cm, 1,84 m de diámetro), los peldaños miden 5,5 cm junto al mástil y 42 cm por fuera: cumple el uso restringido. Una de caracol suele pasar de una vuelta: en planta, los peldaños de arriba van a trazos sobre los de abajo.

**Meseta de llegada** (v0.68). La de caracol termina en una meseta a la altura de la planta de arriba, un sector del anillo de la escalera que sigue girando en el mismo sentido: 90° por defecto en las nuevas, lo que se escriba en Propiedades («Meseta de llegada (°)», 0 para quitarla). La barandilla sigue por ella.

## 2. Estructura y peldaños

| Estructura | Qué es | En 3D |
|---|---|---|
| Losa de hormigón | La de siempre; en caracol, **losa helicoidal** | Los tramos rectos, su perfil exacto; los compensados y los de caracol, peldaño a peldaño con la losa siguiendo los bordes de las huellas |
| Dos zancas laterales | Una zanca a cada lado, de acero o madera | Zancas por fuera de los peldaños, 4 cm por encima de los bordes de las huellas, con su canto |
| Zanca central | Una zanca dentada bajo el centro | Bajo cada peldaño, y bajo las mesetas |
| Peldaños volados | Empotrados en un muro, sin nada debajo | Solo los peldaños (y sus tabicas, si las tienen) |
| Mástil central | Solo en las de caracol | El poste hasta la altura del pasamanos de arriba |

- **Tabicas**: con o sin. La de losa, siempre con.
- **Peldañeado**: el material de las huellas (madera, piedra…). En la de losa es un acabado del grosor que se pida, y la losa baja ese grosor.
- **Barandillas**: pasamanos a 90 cm sobre los bordes de las huellas, un listón bajo a 5 cm, postes cada metro y barrotes cada 11 cm (menos de 10 cm de hueco). La de caracol con mástil o con ojo de menos de 25 cm no lleva barandilla por dentro.

## 3. Comprobación del CTE (DB-SUA 1, 4.2)

Lo de siempre (huella, tabica, 2T + H, ancho, tramos de 3 peldaños o más y de 3,20 m como mucho, pasamanos) y, en los peldaños curvos o compensados:

- huella en la línea de huella: 28 cm como mínimo (22 cm en uso restringido);
- en su lado ancho, 44 cm como máximo;
- en uso restringido, 5 cm como mínimo en su lado estrecho;
- **aviso** si una escalera de uso general no tiene tabica (no vale en zonas de uso público ni para evacuar hacia arriba);
- las **mesetas partidas** solo se admiten en uso restringido (DB-SUA 1, 4.2.1): en una escalera de uso general es un incumplimiento; en uso restringido sus peldaños no cuentan como curvos (no se les piden los 5 y 44 cm).

Las compensadas de 1 m de ancho y uso general son justas: en una U, el lado de fuera pasa de 44 cm. En vivienda (huella de 25 cm) cumplen.

## 4. IFC

- La de losa de tramos rectos (sin mesetas partidas), como antes: `IfcStairFlight` con su perfil extruido y `IfcSlab` `LANDING`.
- Las demás, pieza a pieza (caras trianguladas): tramos `IfcStairFlight` (`WINDER`, `SPIRAL` o `STRAIGHT`, con tabicas, huellas y sus medidas), mesetas `IfcSlab` `LANDING`, zancas `IfcMember` `STRINGER`, mástil `IfcMember` `POST`, peldañeado de las de losa `IfcCovering` `FLOORING`; cada pieza con su material.
- La escalera, `IfcStair` con su forma: `STRAIGHT_RUN_STAIR`, `QUARTER_TURN_STAIR`, `HALF_TURN_STAIR`, `QUARTER_WINDING_STAIR`, `HALF_WINDING_STAIR`, `SPIRAL_STAIR`…
- `ArchiSim_Stair` lleva además `Form`, `Structure`, `ClosedRisers`, `Sweep`, `SplitLandings` y `ArrivalLanding` (si las tiene), `Core`, `Tread`, `StringerWidth`, `StringerDepth`, `Material` y `TreadMaterial` (por nombre), para leerla de vuelta igual.

## 5. Ejemplo

`cargo run -p archisim-cli -- escaleras out/` hace doce escaleras hasta la planta primera, una de cada (con una L y una U de mesetas partidas y la de caracol con meseta de llegada): `escaleras.archisim`, `escaleras.ifc`, las dos plantas y una axonometría en SVG.
