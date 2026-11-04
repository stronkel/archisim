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
2. **Tanda C2, la calle y el terreno** (primera parte hecha, en revisión):
   - **Terraplén**: con sus taludes (3 de base por 2 de altura) donde la calle va por encima del terreno.
   - **Viaducto**: por encima de 6 m, con tablero, pilas cada 24 m y pretiles.
   - **Muro de contención**: a cada lado donde la calle va por debajo del terreno.
   - El terreno solo se recorta donde la calle va metida en él.
   - **Pasos a distinto nivel**: dos calles que se cruzan con más de 3,5 m de diferencia no hacen cruce (una pasa por encima).
   - Para verlo: `archisim trazado out/`.
   - Falta:
     - túneles con sus bocas (desmonte de más de 8 m);
     - puentes de un vano sobre el agua o sobre otra calle, con estribos;
     - rampas y enlaces;
     - pasarelas peatonales;
     - el talud de desmonte en lugar del muro, a elegir.
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
  - `archisim-geom::road_ground`: `road_ground_solids` (taludes, viaducto con pilas y pretiles, muros) y `road_holes` (el agujero en el terreno solo donde la calle va metida).
  - Los sólidos se añaden en `solid::road_solids_with`; los agujeros, en `solid::terrain_holes`.
  - Los cruces se saltan con `crossroads::OVERPASS`.
- **Proyecto de ejemplo**: `archisim trazado out/`.
