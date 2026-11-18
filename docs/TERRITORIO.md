# Taller del territorio: traer un terreno a la realidad

Pedido por Antonio el 16/11/2026: una forma mucho mejor de traer un terreno a la realidad, con todas las opciones de urbanismo, tomando como modelo la forma de trabajar de **Cityburg** en Cities: Skylines y su colección de mods sin recursos (Steam, colección 3364126935, 55 mods). Lo más importante para él: algo como **Node Controller Renewal**. Condición: **el programa tiene que seguir yendo fluido** después de todo esto.

Este documento ordena el trabajo en tandas, como `CALLES_TALLER.md` y `EDIFICIOS_TALLER.md`, de las que es continuación. Rama prevista: `bloque/territorio`.

## 1. Cómo trabaja Cityburg (y qué copiamos)

Cityburg no «dibuja calles»: **reconstruye un sitio real por capas**, y en cada capa tiene control total y la posibilidad de saltarse las reglas del juego.

| Paso | Mods que usa | Qué hace | En ArchiSim |
|---|---|---|---|
| 1. Referencia | CSL Map View, 81 Tiles 2, Optimised / Unlimited Outside Connections | Ver el mapa entero y trabajar en un área grande con sus conexiones de fuera | Traer el entorno (Catastro, OSM, IGN, ortofoto, LiDAR) ya existe. **Falta**: área grande por teselas y la ortofoto como calco a escala en planta y 3D a la vez |
| 2. Red | Network Anarchy, Precision Engineering, Network Multitool, Network Extensions 2, One-Way Train Tracks, Metro Overhaul | Trazar sin que el juego impida nada: alturas libres, solapes, ángulos y pendientes exactos, pendiente uniforme entre dos puntos, paralelas | Trazado por tramos y curva continua (C1). **Falta**: modo «sin reglas», pendiente uniforme entre dos puntos, paralela a otra calle, cotas de ángulo y radio al trazar |
| 3. **Nudos** | **Node Controller Renewal** | Cada nudo se decide a mano: tipo, retranqueo de las esquinas, pasos de peatones, plano o con la rasante, desplazado, sin marcas | Los cruces **se calculan y no se guardan**: no se pueden tocar. **Es la pieza que falta** (§2) |
| 4. Marcas | Intersection Marking Tool | Marcas de cruce libres: líneas entre carriles concretos, rellenos, cebras, isletas pintadas | Marcas de la 8.2-IC sueltas (U4). **Falta**: marcas atadas a los carriles del nudo que lo siguen |
| 5. Reglas | TM:PE, Better Train Boarding, Stops & Stations, Improved Public Transport | Carriles con su flecha, prioridades, prohibido aparcar, paradas | **Falta**: conexiones de carril que pintan sus flechas y ponen sus señales. Sin simulación de tráfico (§5) |
| 6. Suelo | Surface Painter, Extra Landscaping Tools, No Radioactive Desert | Pintar el suelo por zonas exactas, no a brochazos | Pincel (C8) y cubiertas del suelo. **Falta**: superficies por contorno con su borde (rigola, bordillo, encintado) |
| 7. Formas libres | Procedural Objects, Move It, Picker, Hide It, Prop/Tree Control | Piezas a medida (muretes, escalones, isletas), mover cualquier cosa al milímetro, cuentagotas, ocultar piezas del juego | Concepto, mover con precisión (0.84) y cuentagotas ya están. **Falta**: mover cualquier pieza urbana (mobiliario, marcas, árboles de alineación) con Move It y ocultar piezas sueltas del nudo |
| 8. Variar | Network Skins, BOB, Repaint, Customize It | Otro árbol, otra farola u otro pavimento **en un solo tramo**; sustituir en bloque | **Falta**: cambios por tramo (no por tipo de calle) y «Sustituir todos los X por Y» |
| 9. Llenar | Zoning Adjuster, Ploppable RICO, Plop the Growables, Historical Districts | Parcelas exactas y edificios donde él quiere, que no cambian solos | Edificios del mundo colocados a mano (tanda 16). **Falta**: manzanas → parcelas → edificios (C5 de calles) |
| 10. Fluidez | Loading Screen Mod, UI Resolution, Yet Another Toolbar, UUI | Cargar y moverse con todo eso puesto | §4 de este documento |

Lo de simulación (Realistic Population, Lifecycle, Rebalanced Industries, Transfer Manager, Demand Controller, Advanced Vehicle Options) no aplica: ArchiSim no simula la ciudad.

## 2. Nudos (el «Node Controller» de ArchiSim)

### 2.1 Qué ve Antonio

Herramienta **Nudo** en Urbanismo (y botón en la barra de la calle seleccionada). Al pasar el ratón por un cruce, una L, una T o el extremo de una calle, se ilumina el **nudo**; al pinchar, se abren sus **tarjetas** y sus **tiradores** en planta y en 3D:

- **Tipo de nudo** (tarjetas):
  - **Cruce** — el de hoy, con esquinas y bordillos.
  - **Plataforma única** — calzada a la cota de la acera, sin bordillos, con bolardos (calles de coexistencia).
  - **Cruce elevado** — meseta con sus rampas (`raised_plateau` ya existe para marcas).
  - **Continuo** — la calle sigue sin cruce: une dos tramos sin esquinas (cambio de sección o de material sin «costura»).
  - **Doblez** — una curva cerrada de la misma calle, sin aceras de esquina.
  - **Fin** — fondo de saco: recto, en raqueta (círculo de giro) o en martillo.
  - **Sin nudo** — se cruzan sin tocarse (paso a distinto nivel forzado, el `OVERPASS` a mano).
- **Por cada brazo** (una fila por calle que llega, tiradores en planta):
  - **Retranqueo**: cuánto se separa el final del brazo del centro (arrastrando su borde).
  - **Paso de peatones**: sí/no, y a qué distancia.
  - **Bordillo rebajado y pavimento táctil**: sí/no.
  - **Ceda el paso, stop, semáforo**: la señal y su marca (§5).
  - **Rasante**: el brazo llega con la pendiente de su calle o se aplana los últimos X metros.
- **Por cada esquina** (tirador en el arco):
  - **Radio** propio (hoy uno por calle).
  - **Forma**: arco, chaflán (Barcelona), recta, en punta.
  - **Acera de esquina**: sí/no, y si se ensancha (orejas de acera, «bump-out»).
- **El nudo entero**:
  - **Desplazar** el centro sin mover las calles (como «Shift» de Node Controller).
  - **Plano / inclinado**: plano a una cota, o siguiendo la rasante de la calle principal.
  - **Calle principal**: cuál manda en la cota y en la prioridad.
  - **Ocultar** piezas: sin marcas, sin cebras, sin acera en una esquina (Hide It).
  - **Restablecer**: vuelve a lo calculado.

Todo con vista previa al arrastrar, un paso de Deshacer por cambio, y en la Maqueta igual.

### 2.2 Cómo se hace (para el programador)

Hoy `archisim_geom::crossroads::junctions(model, level)` rehace todos los cruces de una planta cada vez y `Junction` no tiene identidad. El cambio:

1. **Identidad estable del nudo** (`archisim-core::model::nodes`):
   - `NodeKey`: planta + calles que llegan (ids ordenados) + extremo o eje de cada una. No depende de la posición exacta: si una calle se mueve, el nudo sigue siendo el mismo mientras lleguen las mismas calles.
   - `NodeSettings` guardado en `Model::nodes` (formato nuevo): `kind`, `shift`, `flat`, `main`, `hidden` y `arms: Vec<ArmSettings>` (por `RoadId` y sentido: retranqueo, paso, rebaje, señal, aplanado) y `corners: Vec<CornerSettings>` (por pareja de brazos: radio, forma, oreja). Solo se guarda lo que Antonio cambia; lo demás sigue calculándose.
   - Al cambiar las calles, `Model::reconcile_nodes` busca para cada ajuste su nudo; si ya no existe (se borró una calle), sus ajustes se quitan (decisión 3 de §5).
2. **`junctions` lee los ajustes**: `assemble` usa el retranqueo, el radio y la forma de cada esquina; los tipos nuevos son variantes de `assemble` (`flat_plate`, `raised`, `seamless`, `bend`, `dead_end`).
3. **IFC**: el cruce ya sale como piezas; los ajustes van en un pset `ArchiSim_Node` en el `IfcBuildingElementProxy` del cruce para que vuelvan al importar.
4. **App**: `app/node_ui` (`Tool::Node`, `Pick::Node`, `Grip::NodeArm`, `Grip::NodeCorner`, `Grip::NodeShift`), tarjetas con `cards::option_cards` dibujadas en 3D como las de los tipos de calle.

### 2.3 Hecho en la primera vuelta (N1, 17/11/2026, versión 0.94)

- **Herramienta Nudo** en Urbanismo (tecla NU): un clic cerca de un cruce, una T o una L lo elige (se marca en azul, con los extremos de los brazos en naranja) y Propiedades enseña sus ajustes.
- **Tipo de nudo**: Cruce, Cruce elevado (meseta a la cota del bordillo con rampas de 2 m en cada brazo), Plataforma única (pavimento de acera, sin bordillos, bolardos cada 1,5 m) y Sin nudo (las calles se cruzan sin cruce).
- **Por brazo**: retranqueo (hasta dónde llega la placa del cruce por esa calle).
- **Por esquina**: radio propio, forma (arco, chaflán o en punta) y acera de esquina sí/no.
- **Restablecer**: vuelve a lo calculado.
- **Guardado**: solo lo cambiado, por la identidad del nudo (calles que llegan y por qué extremo; si se mueve una calle, el nudo sigue siendo el mismo). Si se borra una calle, sus ajustes se pierden. Formato 113; en el IFC, en el pset `ArchiSim_Road` de la primera calle del nudo (propiedad `Nodes`), y vuelven al importar.
- **Ejemplo**: `archisim nudos out/`.

### 2.4 Hecho en la segunda vuelta (N1, 17/11/2026, versión 0.94)

- **Tipos nuevos**, en tarjetas con su dibujo (solo salen los que caben en el nudo):
  - **Continuo**: dos calles que se continúan; bordillos y aceras rectos de una sección a la otra en 6 m por cada lado (una avenida que se estrecha sin costura).
  - **Doblez**: dos calles que forman una curva; la esquina de dentro con su radio y la de fuera alrededor del mismo centro, con las aceras siguiendo la curva.
  - **Fin**: el final libre de una calle (un clic en él lo elige); con Fin lleva un círculo de giro con su bordillo y su acera (radio de 9 m si no se cambia).
  - Las calles que acaban una donde empieza la otra, en línea, no forman nudo por sí solas (así siguen las calles trazadas por tramos); se eligen con la herramienta y entonces se les puede poner Continuo o Doblez.
- **El nudo entero**: **plano** a la cota de su centro; **calle principal**, que manda en la cota y en el pavimento, y un botón «Ceda el paso en las demás calles»; **sin marcas viales** cerca del nudo (quita cebras, líneas y las líneas de detención en 15 m pasado cada brazo).
- **Por brazo**: **señal** (ceda el paso con su línea discontinua M-4.2, stop con su línea continua M-4.1, semáforo con su línea y su báculo), con la línea solo sobre los carriles que entran (no sobre el aparcamiento) y el poste en la acera a la derecha del tráfico; en planta, triángulo, octógono o círculo. **Paso de peatones** sí/no: pone o quita el paso de la calle justo pasado el nudo, con su bordillo rebajado.
- **Por esquina**: **oreja** (la acera sale sobre el aparcamiento, dejando al menos un carril de 2,75 m), con su bordillo que cierra el aparcamiento.
- **Chaflán estándar**: en las esquinas de un nudo o de toda la planta; 9 m en Barcelona (por la georreferencia), 7 m en Cataluña y Valencia, 5 m en lo demás. Luego se cambia cada uno a mano.
- **Tiradores en planta**: el extremo de cada brazo (retranqueo) y el centro del arco de cada esquina (radio), con la medida mientras se arrastra.
- Sin cambio de formato (sigue en el 113): los ajustes nuevos van en el mismo texto, que lee también lo de la primera vuelta.

**No hecho, y por qué**:
- **Desplazar el centro** sin mover las calles: con las esquinas, el retranqueo por brazo y las orejas ya se coloca el nudo donde haga falta; mover su centro sin mover los ejes dejaría los bordillos fuera de las calles. Si lo echas en falta, dime en qué caso.
- **Aplanar un brazo** los últimos metros: el nudo plano con un retranqueo largo da lo mismo en el nudo; aplanar la calle fuera del nudo es cambiar su rasante, que ya se hace en la calle.
- **Fondo de saco en martillo**: solo el recto y el círculo de giro.
- **Tiradores en 3D y en la Maqueta**: los nudos se cambian desde la planta.

### 2.5 N2 hecho: trazado sin reglas y con precisión (17/11/2026, versión 0.95)

- **Anarquía**: interruptor en la herramienta Calle y en las propiedades de cada calle. La calle no se une a las que pisa (ni cruce ni cebras) y no modela el terreno; queda a la cota que se le dé. Sirve para pasos elevados, rampas, solapes de maqueta o calles que van a otra cota. Se guarda en la calle (formato 114) y en el IFC (`ArchiSim_Road.Free`).
- **Pendiente uniforme**: en Propiedades de la calle, grupo «Trazado»: desde, hasta, cota inicial y final (o «Cotas actuales»), con la pendiente que resulta antes de aplicarla. Los puntos de rasante de en medio se quitan; lo demás no cambia.
- **Paralela**: a la izquierda o a la derecha a la distancia tecleada entre ejes, con la misma sección, materiales y rasante.
- **Precisión al trazar** (por tramos): mientras se dibuja, Propiedades enseña la longitud, el giro y el radio de la curva tangente hasta el cursor; con «Poner la curva» sale un tramo en curva del radio y el giro tecleados, tangente al anterior. La longitud y el ángulo ya se tecleaban en la barra de opciones (L y ∠).
- **Hacer glorieta** sobre un nudo de tres calles o más: herramienta Nudo, «Hacer glorieta aquí»; el radio crece si las calles son anchas.
- Ejemplo: `archisim nudos out/` lleva un carril libre que pisa las calles del Olmo y del Pino, la Calle Alta del Parque paralela a la Real subiendo al 2,5 % y la Calle Estrecha acabando en una curva de 25 m.

**Hecho después** (18/11/2026, versión 0.96):
- **Pendiente uniforme con el ratón**: botón «Elegir los dos puntos en planta» en el grupo «Trazado»; dos clics sobre la calle dan desde y hasta, con las cotas que tiene ahora; se cambian si hace falta y se aplica.
- **Radio y giro en la barra de opciones** (R y Giro), junto a L y ∠, mientras se traza por tramos: Intro pone la curva tangente al tramo anterior.

### 2.6 N3 hecho: carriles y marcas del nudo (18/11/2026, versión 0.97)

- **Dónde va cada carril**: el nudo sabe los carriles que entran y salen por cada calle y calcula sus conexiones: de frente desde todos, a la derecha desde el del bordillo y a la izquierda desde el del eje (todo desde un carril único). Con la herramienta Nudo se ven en planta como líneas azules con su flecha.
- **Giros por carril** (como las flechas de TM:PE): en Propiedades, grupo «Carriles», cada carril que entra tiene sus botones ← ↑ → ↶; «Giros calculados» los devuelve.
- **Conexiones a mano** (Intersection Marking Tool): «Conectar carriles a mano» y en planta el final de un carril que entra (punto verde) y luego el principio del carril por el que sale (punto blanco); repetirlo quita la conexión. La primera conexión a mano de una calle conserva las que tenía calculadas para que solo cambies lo que quieres. Las hechas a mano salen en naranja.
- **Flechas pintadas** (M-5.2) en cada carril que entra, 4 m detrás de la línea de detención, en las calles con más de un carril de entrada o cambiadas a mano; y **líneas continuas** de 20 m entre esos carriles antes del nudo. Siguen la rasante de su calle.
- **Línea de detención retrasada**: por calle, cuántos metros se retrasa (con su señal y sus flechas).
- **Isletas pintadas**: «Dibujar isleta pintada», clic en cada esquina e Intro; cebreado (M-7.1), relleno o solo el borde. Se guardan en los ejes del nudo: si se mueven sus calles, van con él.
- Todo se guarda en el texto del nudo (formato 115) y vuelve del IFC.
- Ejemplo: `archisim nudos out/` añade la Avenida de los Carriles (dos carriles por sentido) con la Calle de la Isleta.

## 3. Lo demás, por tandas

| Tanda | Qué | Mods de referencia | Tamaño |
|---|---|---|---|
| **N0** | **Fluidez del territorio** (§4): teselas, rehacer solo lo tocado, cruces con índice. Va **primero**: sin ella, N1–N7 hacen el programa lento | Loading Screen Mod, 81 Tiles | G |
| **N1** | **Nudos** (§2): identidad, ajustes, tipos, tiradores, IFC | Node Controller Renewal | G |
| **N2** | **Trazado sin reglas y con precisión**: modo «Anarquía» (alturas libres, solapes sin cruce, sin recorte del terreno), pendiente uniforme entre dos puntos, paralela a otra calle a una distancia, ángulo y radio tecleados al trazar, «hacer glorieta» sobre un nudo | Network Anarchy, Precision Engineering, Network Multitool | M |
| **N3** | **Carriles y marcas del nudo**: conexiones de carril en el nudo (de qué carril a cuál), sus flechas pintadas, isletas pintadas y rellenos, líneas de detención; atadas al nudo, lo siguen al cambiarlo | Intersection Marking Tool, TM:PE | G |
| **N4** | **Cambios por tramo**: en un trozo de calle, otro árbol, farola, pavimento o bordillo; «Sustituir todos» en el proyecto o en una zona | Network Skins, BOB, Repaint | M |
| **N5** | **Superficies por contorno**: plazas, isletas, medianas, aparcamientos y jardines dibujados por su borde con su encintado y sus juntas; snap a bordillos y fachadas | Surface Painter, Parking Lot Snapping | M |
| **N6** | **Mover y ocultar cualquier pieza urbana**: mobiliario, árboles de alineación, señales y marcas se sueltan de la calle y se mueven al milímetro (Move It); ocultar piezas sueltas | Move It, Hide It, Picker | P |
| **N7** | **Manzanas, parcelas y llenado**: lo de C5 de calles; parcelas exactas por la línea de fachada, llenar con edificios del lugar por zonas, y bloquear lo que no debe cambiar | Zoning Adjuster, Ploppable RICO, Plop the Growables, Historical Districts | G |
| **N8** | **Área grande**: hasta 2 × 2 km traídos de una vez, con las conexiones de fuera (las calles que salen del recuadro se cortan con su sección, no en punta) y vista de mapa | 81 Tiles 2, CSL Map View, Outside Connections | M |
| **N9** | **Tráfico de ilustración** (el plus que pidió Antonio): coches y personas que circulan por los carriles y aceras según las conexiones de carril de N3 y los semáforos, en la vista 3D y en el render; sin cálculo de capacidad | TM:PE (lo que se ve) | M |

Lo que ya está en `CALLES_TALLER.md` (túneles, puentes, catálogo regional, detalle C4) sigue su camino; N1–N3 le dan la parte de cruces que allí faltaba.

## 4. Fluidez: el presupuesto antes que las funciones

Lo que hoy hace lento el urbanismo, medido en el código:

- **Los cruces se rehacen enteros** cada vez que se piden (`crossroads::junctions` se llama desde once sitios: planta, 3D, IFC, marcas, terreno…), y comparan cada calle con todas.
- **El terreno modelado por las calles** (`road_ground::shaped_ground`) se rehace entero al tocar una calle.
- **La escena 3D** se rehace entera en otro hilo (`scene_job`): ya no congela, pero tarda más cuanto más grande es el sitio.
- **No hay cálculo en paralelo** fuera del render.

El plan, en este orden, cada paso con su medida antes y después:

1. **Escenario de referencia** (`archisim territorio-grande out/`): 1 × 1 km de Punta Umbría traído del Catastro y OSM, con terreno, ortofoto, ~600 edificios vestidos, ~200 calles, ~300 nudos, vegetación y mobiliario. Objetivos, en el PC de Antonio (RTX 5080) y en el MacBook:
   - navegar en 3D a **60 fotogramas** (MacBook: 30);
   - tocar un nudo o arrastrar un tramo y verlo en **menos de 50 ms**;
   - abrir el proyecto en **menos de 5 s**;
   - en el iPad, menos de 800 MB.
   Se añade al medidor (`archisim medir`) y a la integración continua para que no empeore sin que se vea.
2. **Teselas del territorio**: el sitio se reparte en cuadrados de 100 × 100 m. Cada tesela guarda su trozo de terreno modelado, sus calles, cruces, marcas, vegetación y mobiliario ya en triángulos (en la planta y en el 3D).
3. **Rehacer solo lo tocado**: cada cambio dice qué rectángulo toca (la calle antes y después, más el alcance del talud, 30 m); solo se rehacen las teselas que cruza. Arrastrar un nudo rehace una o dos teselas, no el sitio.
4. **Cruces con índice y cache**: un índice por celdas de las calles (como `WallIndex` para los muros); los cruces se guardan por `NodeKey` y solo se recalculan los de las calles que han cambiado. Los once sitios piden el mismo cache.
5. **En paralelo**: las teselas son independientes, así que se hacen a la vez en todos los núcleos (crate `rayon` en escritorio; en la web, los *workers* que ya usa el render).
6. **Nivel de detalle por tesela**: cerca, todo; a media distancia, sin marcas finas ni mobiliario pequeño y con los edificios en su versión ligera; lejos, una sola malla simplificada por tesela con la ortofoto o los colores horneados, y los árboles como imágenes planas (ya decidido en `CALLES_TALLER.md` §3).
7. **Repetir en lugar de copiar**: farolas, bancos, bolardos, árboles de alineación, señales y piezas de bordillo se dibujan como ejemplares de un mismo modelo (como ya se hace con los muebles), no como triángulos copiados.
8. **Planta por teselas**: la imagen de la planta se guarda por tesela y escala; al cambiar algo, solo se redibujan las teselas tocadas.
9. **Ficheros**: las teselas no se guardan en el `.archisim` (se rehacen al abrir, en paralelo); si abrir un sitio grande pasa de 5 s, se guarda un cache aparte junto al proyecto.

### 4.1 Medido (N0, primera vuelta, 16/11/2026)

`archisim territorio-grande out/` hace el sitio de referencia (inventado, porque desde aquí no se puede descargar el Catastro): 1 × 1 km de terreno ondulado, 90 calles en cuadrícula que siguen el terreno, calles cortas en T, 5 glorietas, 219 cruces, 560 edificios del mundo y 13 692 objetos de la vida de la calle. Escribe `territorio.archisim` y `territorio-medidas.txt`. Tiempos en la máquina de pruebas (2 núcleos, más lenta que el PC de Antonio):

| Qué | Antes | Ahora |
|---|---|---|
| Crear el sitio (trazar las calles siguiendo el terreno) | 2,0 s | 0,2 s |
| Cruces de la planta | 5 ms | 4 ms la primera vez, 0,5 ms las demás |
| Sólidos de las calles | 13,9 s | 0,8 s |
| Terreno modelado por las calles | 17,4 s | 3,8 s |
| 3D de todo el sitio (76 millones de triángulos) | 55,5 s | 31,7 s |
| Mover una calle: calles y terreno de nuevo | 14,6 s + 17 s | 1,0 s + 3,6 s |
| Planta en pantalla, 200 × 120 m a 1:500 | — | 0,46 s |
| Planta en pantalla, todo el sitio a 1:2000 | — | 5,6 s |
| Guardar / abrir | — | 0,10 s / 0,07 s |

Qué era lo lento: **cada vez que se preguntaba la cota del terreno en un punto se volvía a triangular el levantamiento entero**. Una calle que sigue el terreno pregunta cada pocos metros, y 90 calles lo hacían cientos de miles de veces. Ahora la superficie del terreno se triangula una vez y se guarda con un índice por celdas (`Terrain::surface`) mientras sus puntos no cambian. Los cruces se guardan igual (`crossroads::shared_junctions`) y solo comparan las calles cuyas cajas se tocan.

Lo que falta de N0, por orden de lo que más pesa:
- **El terreno se rehace entero** al mover una calle (3,6 s): hay que partirlo en teselas y rehacer solo las que toca la calle (pasos 2 y 3).
- **Los objetos de la vida de la calle** son 69 de los 76 millones de triángulos (árboles con todo su detalle): repetir el modelo y detalle por distancia (pasos 6 y 7). La vista 3D ya los repite; el render, el glTF y el IFC no.
- **La planta del sitio entero** (5,6 s): por teselas (paso 8).
- Se ha visto que el número de triángulos de los objetos **cambia un poco de una vez a otra** con el mismo proyecto (unas miles de 69 millones): algo en las plantas no es repetible y hay que encontrarlo.

### 4.2 Segunda vuelta (16/11/2026, noche)

Lo que se mide ahora es lo que ve Antonio: **cuánto tarda la vista 3D de la aplicación en rehacerse después de un cambio** (prueba `territory_scene`; `ARCHISIM_TERRITORIO=out/territorio.archisim cargo test --release -p archisim-app -- territory_scene --nocapture`). En la 0.92 eran **40 s** con este sitio, aunque solo se moviera una calle. Ahora:

| Después de mover una calle 1 m | 0.92 | 0.93 |
|---|---|---|
| Escena 3D entera de nuevo | 40 s | **2,2 s** |
| Árboles de alineación (4 662) | 31 s | 0,3 s |
| Edificios vestidos (560) | 6,5 s | 0,6 s |
| Terreno y calles | 3,5 s | 0,3 s |
| Calles y terreno sin la vista (el cálculo) | 14,6 s + 17 s | 0,06 s + 0,2 s |
| Planta en pantalla, 200 × 120 m | 0,46 s | 0,30 s |
| Planta del sitio entero | 5,6 s | 1,8 s |

Lo que se ha hecho:
- **Terreno por cuadrados de 100 m**: cada cuadrado se modela con las calles que le llegan y se guarda con ellas; al mover una calle se rehacen solo los cuadrados que cruza, en todos los núcleos a la vez. Cada punto mira solo las calles de su celda de 20 m, no todas. El borde y las paredes de los huecos se buscan solo en los cuadrados que los tocan.
- **Cada calle se guarda** con lo que la hace (su trazado, sus cruces, las calles que llegan a ellos y el terreno de debajo): al mover una, se rehacen ella y las que cruza.
- **Los árboles ya no se vuelven a hacer crecer**: la caché de plantas se vaciaba entera al pasar de 3 000, y con 4 662 árboles se vaciaba en cada vuelta. Ahora guarda por tamaño y olvida primero lo que lleva más tiempo sin usarse.
- **En la vista 3D**, cada árbol se convierte una vez en sus propios ejes y en cada vuelta solo se gira y se coloca. Los edificios vestidos, los árboles de cerca y cada cuadrado de terreno, calle y marca se guardan convertidos bajo una huella de sus triángulos y colores, y se reutilizan si no han cambiado.
- **En planta** se descartan antes de calcularlos los objetos lejos de la vista, y las piezas de calle, alcorques y marcas fuera de ella.

Lo que falta para bajar de 2 s a menos de 50 ms (siguiente vuelta, N0-3):
- **Saber qué elementos cambiaron** en cada paso de Deshacer, para no volver a generar lo que no ha cambiado (hoy se genera todo y se reconoce por su huella; lo caro ya no se repite, pero recorrerlo todo sigue costando unos 2 s en esta máquina de 2 núcleos).
- **Que la tarjeta gráfica conserve los trozos que no cambian** (hoy se vuelve a subir la escena entera).
- **Exportar a glTF, render o IFC con miles de árboles a todo detalle** ocupa unos 5 GB de memoria: hay que repetir el modelo de cada árbol también ahí.
- **Las piezas de las calles largas en planta** (≈90 ms por vista).

### 4.3 Tercera vuelta (17/11/2026)

La escena 3D ya no se hace entera tras cada cambio: **se conserva y se rehace solo lo tocado**.

- **Qué cambió**: el modelo se compara con el de antes elemento a elemento (`Model::changes_since`). Un tipo, un material o una planta cambiados rehacen todo; lo demás, solo sus elementos.
- **Lo que depende de ello**, por alcance: una calle rehace sus cruces, las calles que llegan, el terreno de debajo y los edificios a 30 m que la miran; un mueble, los de al lado; un árbol, nada más.
- **Celdas de 64 m**: las piezas de la escena van por celdas y capas (muebles, terreno, calles, lo demás); las piezas grandes, en celdas propias. Al rehacer, solo se tocan esas celdas; las demás siguen igual **y la tarjeta gráfica no las vuelve a recibir**.
- **Terreno por cuadrados de 50 m**, cada uno con su borde y sus paredes guardados; cada tramo de calle y cada cruce, guardados por lo que los hace.

Medido en la máquina de pruebas (2 núcleos lentos), sitio de 1 km²:

| Tras… | 0.92 | 0.93, 2.ª vuelta | 0.93, 3.ª vuelta |
|---|---|---|---|
| Nada (otra vez igual) | 40 s | 1,9 s | **0,02 s** |
| Mover un árbol | 40 s | ≈2 s | **0,03 s** |
| Girar un banco | 40 s | ≈2 s | **0,03 s** |
| Subir un edificio una planta | 40 s | ≈2 s | **0,03 s** |
| Mover una calle | 40 s | 1,8 s | **0,12 s** |

Con los muebles, los árboles y los edificios ya se cumplen los 50 ms. Con las calles, no en esta máquina: la mitad del tiempo es rehacer y convertir los cuadrados de terreno que modela, que se reparten entre los núcleos, así que en el PC de Antonio (muchos más núcleos y más rápidos) debería quedar cerca de 40–60 ms. Falta medirlo allí.

Lo que queda:
- **Abrir el sitio** sigue tardando ~30 s la primera vez (los 4 662 árboles y los 560 edificios se hacen enteros).
- **Exportar a glTF, render o IFC** con miles de árboles a todo detalle sigue ocupando mucha memoria.
- Si en el PC de Antonio las calles pasan de 50 ms: convertir el terreno más fino, por trozos de cuadrado.

## 5. Decisiones de Antonio (16/11/2026)

1. **Orden**: N0 completo primero, después N1 y el resto en el orden de §3.
2. **Tráfico**: las reglas se dibujan y se miden (flechas, señales, prioridad, prohibido aparcar, semáforos). Ver circular coches sería un plus muy interesante: se deja como tanda aparte al final (**N9, tráfico de ilustración**: coches y personas moviéndose por los carriles y aceras según las conexiones de carril, solo para ver y para el render, sin cálculo de capacidad).
3. **Nudos huérfanos**: si una calle se borra y su nudo desaparece, sus ajustes se pierden. Al volver a dibujarla sale el nudo calculado y se ajusta otra vez a mano.
4. **Área grande**: 2 × 2 km basta.
5. **Chaflanes**: sale un tamaño estándar (por defecto el del lugar, p. ej. 20 m en el Ensanche de Barcelona) y luego se ajusta a mano por esquina.

## 6. Qué probar en cada tanda

Cada tanda trae su orden de ejemplo (`archisim nudos out/`, `archisim territorio-grande out/`…) y sus fotos de prueba como las de los edificios del mundo; en la bitácora se dice qué abrir y qué mirar.

## 7. Para pulir al final

Lo que se deja para después, tanda a tanda, para repasarlo junto con la cuarta vuelta de fluidez. Se va tachando lo que se hace.

**De N1 (nudos)**
- Desplazar el centro del nudo sin mover las calles (Shift de Node Controller).
- Aplanar un brazo los últimos metros.
- Fondo de saco en martillo.
- Tiradores de los nudos en 3D y en la Maqueta.

**De N3 (carriles)**
- Cambio de sentido (↶): se conecta, pero no tiene flecha pintada (la 8.2-IC no la trae entre las urbanas).
- Quitar la última conexión a mano de una calle la devuelve a sus giros calculados: no se puede dejar una calle «sin salida» por un carril.
- Las flechas son de 5 m y solo una por carril; en avenidas largas la norma pide repetirlas.
- Las conexiones se ven solo en planta y con la herramienta Nudo; no en 3D ni en la Maqueta.
- Isletas: no se editan sus esquinas una vez dibujadas (se quitan y se dibujan otra vez).
- Carriles bus y tranvía: el bus cuenta como carril; el tranvía, no.

**De N2 (trazado)**
- La pendiente uniforme elegida con el ratón no enseña marcas en planta entre los dos clics (solo el aviso).
- Cotas de ángulo y radio dibujadas sobre la planta mientras se traza (hoy, en la barra y en Propiedades).
