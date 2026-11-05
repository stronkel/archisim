# Taller de calles

Antonio pidió el 04/11/2026 que poner calles sea como en Cities: Skylines:
- se ponen por tramos, viendo ya el tipo de calle;
- se adaptan al terreno;
- hay vías en altura, peatonales y de todo tipo, también las de cada región, con tanto catálogo como los edificios;
- calles y edificios se adaptan unos a otros;
- el detalle está a la altura de los edificios del taller.

Este documento ordena ese trabajo en tandas, como `EDIFICIOS_TALLER.md`. Rama `bloque/calles`.

## 1. Lo que ya había (U1–U5)

- **Viales**: por su eje y una sección tipo de franjas: calzada, aparcamiento, bordillo, acera, carril bici, mediana, alcorques, tranvía, cuneta…
- **Rasante**: con acuerdos verticales.
- **Cruces calculados**: en cruz, en T y en L, con sus esquinas.
- **Glorietas y plazas**.
- **Tipos de calle**: unos treinta, en tarjetas dibujadas en sección.
- **Marcas viales**: las de la Norma 8.2-IC.
- **Redes urbanas** bajo la calle.

## 2. Orden de las tandas

1. **Tanda C1, trazado por tramos** (hecha, en revisión):
   - La herramienta Vial dibuja por **tramos rectos** o **tramos curvos**. Cada clic pone el tramo con su sección y el siguiente sigue de él.
   - Empezando en el extremo libre de una calle de la misma sección, la alarga. Empezando o acabando sobre otra calle, se pega a su eje y el cruce se hace solo.
   - Mientras se mueve el ratón se ve el tramo con sus franjas en planta, entero en 3D y con la pendiente del terreno que salva.
   - **Sigue el terreno**: la rasante sigue el terreno, suavizada y sin pasar de la pendiente máxima (8 % por defecto). Donde el terreno es más empinado, la calle va en desmonte o en terraplén.
   - El modo de antes, el eje entero por sus puntos (o por el borde de una acera), sigue como «Eje entero».
   - **Segunda vuelta (05/11/2026)**, tras la revisión de Antonio («más natural y fluido, como Cities: Skylines»):
     - modo nuevo **«Curva continua»**, el de por defecto: cada tramo sale tangente al anterior y se curva hacia el ratón, sin quiebros;
     - **marcas viales de serie**: las líneas de separación de carriles salen solas en cada calle nueva, y los **pasos de peatones** en los brazos de cada cruce con aceras; se cambian o se quitan como cualquier marca;
     - las **tarjetas de los tipos de calle** se ven en 3D (un trozo de calle de 12 m con sus franjas).
2. **Tanda C2, la calle y el terreno** (primera parte hecha, en revisión):
   - **El terreno se modela con la calle** (segunda vuelta, 05/11/2026): en lugar de taludes y muros pegados, el propio terreno baja o sube hasta la calle, con taludes suaves (1:1,5 en terraplén, 1:1 en desmonte) que se funden con el terreno hasta 30 m a cada lado; la malla se afina a 2 m junto a las calles, así que no hay escalones. Varias calles juntas se respetan entre sí.
   - **Viaducto**: por encima de 6 m, con tablero, pilas cada 24 m y pretiles; bajo él el terreno sigue su forma.
   - **Pasos a distinto nivel**: dos calles que se cruzan con más de 3,5 m de diferencia no hacen cruce (una pasa por encima).
   - Para verlo: `archisim trazado out/`.
   - Falta:
     - túneles con sus bocas (desmonte de más de 8 m);
     - puentes de un vano sobre el agua o sobre otra calle, con estribos;
     - rampas y enlaces;
     - pasarelas peatonales;
     - estribos donde empieza el viaducto (hoy se ve el corte del terreno);
     - muros de contención a elegir donde no cabe el talud.
3. **Tanda C3, catálogo de vías**:
   - **Más tipos**:
     - peatonales: de plaza, escalonadas, en rampa, porticadas, entarimadas;
     - de coexistencia y de casco histórico: empedrado, adoquín, canto rodado, lajas;
     - de ciudad: rondas, bulevares con paseo central, rambla, gran vía con laterales;
     - de fuera de la ciudad: autovía con mediana y bionda, carretera nacional, comarcal y local, camino agrícola, de tierra o de grava, vía verde, sendero;
     - calles de urbanización con fondo de saco.
   - **Regionales**: las de cada uno de los 30 lugares de los edificios, por ejemplo:
     - calzada portuguesa en Lisboa;
     - panot y chaflán en Barcelona;
     - empedrado y acera de granito en Galicia;
     - albero y naranjos en Andalucía;
     - aceras de losa de York en Londres;
     - acera con franja de césped y boca de incendios en Estados Unidos;
     - calle japonesa sin acera con su línea blanca y su canal;
     - calle de la medina;
     - adoquín y bolardos en París.
   - Se eligen, como los edificios, en tarjetas en 3D por grupos.
4. **Tanda C4, detalle como los edificios**:
   - bordillos con su forma (remontable, achaflanado, de granito) y rigolas;
   - imbornales, rejillas y tapas de registro;
   - juntas del pavimento por piezas; alcorques con su rejilla;
   - pasos de peatones con el bordillo rebajado y el pavimento táctil;
   - bolardos, vallas, señales verticales y semáforos en los cruces;
   - farolas y papeleras de la colección del lugar;
   - mobiliario y desgaste: parches del asfalto, manchas y hojas.
   - La versión de lejos, ligera, como en los edificios.
5. **Tanda C5, calles y edificios juntos**:
   - **Calle hacia los edificios**: al trazar una calle, los edificios del mundo que quedan debajo se avisan o se quitan; los de al lado se alinean a la acera con su fachada a la calle.
   - **Edificio hacia la calle**: al poner un edificio junto a una calle, su fachada va a la acera (ya en la tanda 16 de edificios).
   - **Manzanas como en Cities: Skylines** (a decidir con Antonio): las calles que cierran una manzana crean sus parcelas y se pueden llenar de edificios del lugar con un clic, por zonas (residencial, comercial, equipamientos).
   - Al mover una calle, los edificios que la miran la siguen.

## 3. Vegetación (decidido el 04/11/2026)

Antonio no está contento con la vegetación: no se parece a Paralives y pesa mucho. Se cambia de enfoque:
- **Copas**: dejan de hacerse hoja a hoja y pasan a ser pocas formas redondeadas y suaves, con degradado de color de abajo arriba y luz que las atraviesa. Unos cientos de triángulos por árbol, un modelo por especie y variante repetido.
- **De lejos**: una imagen plana.
- **Hierba**: textura con matas sueltas en lugar de briznas.

Primero una prueba con 6 especies junto a las de hoy; se sustituye solo cuando Antonio la apruebe. Va después de las calles.

## 4. Para el programador

- **Trazado**:
  - `archisim-core::model::road_drawing`: `RoadStretch`, `Model::add_road_stretch`, `road_end_near`, `road_axis_near`, `road_end_direction`, `curve_points`, `graded_heights`, `Model::road_profile_following`, `PROFILE_STEP`.
  - En la app, `urban_ui::RoadDraw`, `RoadSettings::follow_ground`/`max_grade`, `ArchiSimApp::road_chain`, `road_snap`, `road_stretch_points`, `build_road_polyline`.
  - El 3D de lo que se va a poner: `ghost3d::Would::Road`.
- **Terreno**:
  - `archisim-geom::road_ground`: `Shaper` (las calles cerca de un punto, `RoadLine` con sus `Sample`), `shape` (bajo el pavimento de cualquier calle, salvo viaducto; si no, entre el talud de terraplén y el de desmonte de la más cercana, `REACH` 30 m), `shaped_ground` (la malla afinada por aristas a `FINE_EDGE` 2 m, sin grietas) y `road_ground_solids` (solo el viaducto: tablero, pilas, pretiles).
  - `solid::terrain_solids` modela el terreno y rehace sus lados con las aristas sueltas de arriba; las calles ya no agujerean el terreno.
  - Los cruces se saltan con `crossroads::OVERPASS`.
- **Marcas de serie**: `Model::add_road_stretch` pone el material de las marcas si la sección no lo tiene; `Model::crossings_at` pone las cebras (`ZEBRA_WIDTH`, `ZEBRA_SETBACK`) en los brazos con acera de un cruce.
- **Curva continua**: `urban_ui::RoadDraw::Smooth` (control en `a + dir · d/2`).
- **Tarjetas en 3D**: `street_types_ui::street_model`, `ArchiSimApp::street_thumbs`; fotos de todas con `ARCHISIM_CALLES_OUT` en `every_street_type_in_3d`.
- **Proyecto de ejemplo**: `archisim trazado out/`.
