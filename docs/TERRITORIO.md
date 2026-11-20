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

### 2.7 N4 hecho: cambios por tramo y «Sustituir todos» (18/11/2026, versión 0.98)

- **Por tramos** (Network Skins, Repaint): en las propiedades de una calle, grupo «Por tramos»: «Nuevo tramo», desde y hasta a lo largo del eje, y el material de la calzada, el bordillo, la acera y la tierra de ese trozo (vacío: el de la calle). Se ve en 3D, sale en el IFC con sus piezas y se guarda (formato 116).
- **Sustituir todos** (BOB): ficha «Sustituir todos» en Urbanismo → Calle, o el botón «Sustituir todos…» de la calle. Cambia:
  - **objetos de un tipo** por otro tipo (un árbol por otro, una farola por otra, un banco…), conservando su sitio, giro y valores;
  - **un material de las calles** por otro.
- **Dónde**: en todo el proyecto, en esta planta, en una **zona** (dos clics en planta, un rectángulo) o en un **tramo de la calle elegida** (desde y hasta). Los materiales en una zona o un tramo se cambian creando tramos propios en las calles, sin tocar el resto; en el proyecto o la planta, la calle entera.
- Un paso de Deshacer por sustitución.
- Ejemplo: `archisim nudos out/` pone adoquín de granito en la Calle Real entre la del Olmo y la del Pino.

### 2.8 N5 hecho: superficies por contorno (18/11/2026, versión 0.99)

- **Fichas nuevas** en Urbanismo → Calle: **Plaza**, **Isleta**, **Aparcamiento** y **Jardín** (y **Mediana** desde las tarjetas de la herramienta). Se dibujan como una parcela: clic en cada esquina (o rectángulo) y se cierran en la primera o con Intro.
- **Se pegan** a los bordillos de las calles, a las líneas de las parcelas (fachadas) y a otras superficies (Parking Lot Snapping).
- **Cada clase trae lo suyo**, y todo se cambia en Propiedades:
  - plaza con encintado de 30 cm y juntas cada metro;
  - isleta y mediana elevadas 15 cm con su bordillo;
  - aparcamiento con rigola y plazas de 2,50 × 5,00 m en batería, en filas a lo largo del lado más largo con calle de 6 m entre cada dos (ancho, fondo y ángulo: 90° en batería, 45° o 60° en espiga), con el número de plazas;
  - jardín con bordillo de 10 cm.
- **Se apoyan** en el terreno o en la calle que tengan debajo (una isleta sobre un cruce va sobre su calzada) y suben su altura.
- **Planta**: pavimento, borde, juntas desde 1:200 y plazas pintadas. **3D**: pavimento, borde (el bordillo un poco por encima) y pintura de las plazas.
- **IFC**: `IfcBuildingElementProxy` con su pset `ArchiSim_UrbanSurface` (superficie y número de plazas incluidos); vuelven al importar.
- Se mueven, copian, giran, reflejan y cambian de planta como lo demás. Formato 117.
- Ejemplo: `archisim nudos out/`: aparcamiento y jardín en la manzana de la casa, plaza de adoquín en la de al lado y una mediana al final de la Avenida de los Carriles.

### 2.9 N6 hecho: ocultar y soltar piezas de la calle (18/11/2026, versión 0.100)

- **Ocultar piezas** (Hide It): ficha «Ocultar piezas» en Urbanismo → Calle; cada clic oculta la pieza de calle que hay debajo: una raya de las líneas de carril, una plaza pintada del aparcamiento, una barra de cebra, un alcorque, la señal de un nudo (con su poste) o un bolardo. Se guardan por dónde están a lo largo de la calle y a su lado, así que siguen ocultas si la calle se mueve. En la calle, «Mostrar las piezas ocultas» las devuelve (y dice cuántas hay).
- **Soltar paso de peatones** (Move It): un clic en una cebra de una calle la convierte en una marca vial suelta, elegida, que se mueve, gira o copia con las órdenes de siempre (arrastrar, flechas del teclado, Mover con distancia). El rebaje de los bordillos se queda con la calle; se quita con la cebra.
- El mobiliario, los árboles de alineación y las marcas sueltas ya eran elementos propios: se mueven al milímetro con lo de la 0.84 (flechas, distancia tecleada, guías).
- Se guarda en la calle (formato 118) y en el IFC (`ArchiSim_Road.Hidden`).
- Ejemplo: `archisim nudos out/` oculta un bolardo de la plataforma única y la señal sur del cruce del Olmo.

### 2.10 N7 hecho: manzanas, parcelas y llenado por zonas (18/11/2026, versión 0.101)

- **Manzanas**: las calles que se cierran (con sus cruces y glorietas) dejan huecos; cada hueco de más de 80 m² es una manzana, por su línea de fachada. Las calles en anarquía no cuentan.
- **Parcelar** (Zoning Adjuster): Urbanismo → Territorio → **Manzanas** abre el panel. Se elige la zona y, si se quiere, el frente y el fondo de parcela (si no, los habituales de la zona). Luego se pincha dentro de una manzana, o «Parcelar todas».
  - Las parcelas van en escuadra con su fachada.
  - Cada esquina es una parcela que da a las dos calles.
  - Queda un patio de manzana si el fondo cabe; una manzana pequeña es una sola parcela.
  - Se llaman «Manzana n · k».
- **Zonas**: casco antiguo (casas entre medianeras), bloques de viviendas, viviendas aisladas (con jardín delante), comercio (tiendas) y equipamiento (colegio). En planta se tiñe cada parcela con el color de su zona. La zona se cambia parcela a parcela en sus propiedades.
- **Llenar** (Ploppable RICO): un edificio del mundo del lugar en cada parcela vacía, con su fachada a la calle, tan ancho como el frente y tan hondo como deja la parcela. «Rehacer» quita los de las parcelas sin bloquear y las vuelve a llenar; «Vaciar» solo los quita.
- **Bloquear** (Historical Districts): en las propiedades de la parcela, «Bloqueada»: llenar, rehacer y vaciar no la tocan. La planta lo rotula.
- Cada parcela se puede llenar o vaciar sola desde sus propiedades. Todo es un paso de Deshacer.
- **Fichero** (formato 119: `zone` y `locked` de cada parcela) e **IFC** (`ArchiSim_Plot.Zone` y `ArchiSim_Plot.Locked`, que vuelven al importar).
- Ejemplo: `archisim manzanas out/`: cuatro manzanas, una de cada zona y un colegio en la esquina del comercio, con una casa bloqueada en el casco antiguo.

### 2.11 N8 hecho: área grande, conexiones con fuera y vista de mapa (18/11/2026, versión 0.102)

- **Área del entorno**: al traer el entorno se guarda lo que se trajo, el círculo de su radio o, con la opción nueva «recuadro», un cuadrado de lado el doble del radio (hasta 2 × 2 km con radio 1 000 m). Lo que se trae queda dentro del recuadro: edificios, árboles, terreno y calles.
- **Conexiones con fuera** (Outside Connections): las calles se cortan en el borde del área con toda su sección, en escuadra con su eje, y cada extremo cortado es una conexión con fuera. En planta, el borde del área va en trazo largo y cada conexión lleva una flecha hacia fuera con el nombre de la calle.
- **Vista de mapa** (CSL Map View): Urbanismo → Territorio → **Mapa** abre una ventana con toda el área, plana y en color:
  - Calles y avenidas (las de 20 m o más, en amarillo) con sus nombres.
  - Edificios del entorno, con los equipamientos en otro color, y el edificio del proyecto en rojo teja.
  - Árboles, jardines, piscinas y las zonas de las parcelas.
  - Las conexiones con fuera.
  - Un recuadro rojo dice qué parte enseña la planta. Un clic (o arrastrar) en el mapa lleva la planta ahí; «Ver toda el área» la encuadra entera.
- La planta se puede alejar ahora hasta ver 2 km en la ventana.
- **Fichero** (formato 120: `surroundings`) e **IFC** (`ArchiSim_Surroundings.Area` en el proyecto).
- **Línea de órdenes**:
  - `archisim area-grande out/` hace un área de 2 × 2 km con 36 conexiones y 544 edificios. Escribe su mapa y su planta y dice cuánto tarda cada cosa (el IFC unos 3 s; el mapa y la planta, centésimas).
  - `archisim mapa <proyecto>` saca el mapa de cualquier proyecto.

### 2.12 N9 hecho: tráfico de ilustración (18/11/2026, versión 0.103)

- **Coches por los carriles**:
  - Por la derecha, en cada carril de cada calle.
  - En los cruces siguen las conexiones de carril de N3 (los giros de cada carril y las conexiones a mano), por la misma curva que dibuja el nudo.
  - Paran en rojo y en ámbar. Los semáforos de un cruce van en dos fases, una para cada dirección: 16 s en verde y 3 s en ámbar.
  - En los stop se detienen un momento.
  - Guardan la distancia con el de delante en su carril.
  - Al salir por un fondo de saco o por una conexión con fuera, aparecen en otro sitio.
- **Personas por las aceras**: van y vuelven, cada una a su paso.
- **Vista 3D**: Urbanismo → Territorio → **Tráfico** lo pone en marcha en la vista 3D y vuelve a pulsarse para pararlo. Hay un coche cada 40 m de calle y una persona cada 25 m, en colores variados. Al cambiar las calles empieza de nuevo sobre las calles nuevas.
- **Render**: con el tráfico en marcha, el render lo saca como está a los 20 s. En la línea de órdenes, `archisim render … --trafico`.
- Es para ilustrar: no calcula capacidades, colas ni prioridades. Dos coches de calles distintas pueden coincidir dentro del cruce.
- No toca el proyecto: no se guarda ni sale en el IFC.
- **Ejemplo**: `archisim trafico out/` toma las calles de `archisim nudos`, con semáforos en su cruce principal, y escribe:
  - la planta con los coches y las personas (`trafico-planta.svg`);
  - una imagen del cruce (`trafico.png`).

### 2.13 N10 hecho: la barra de calles, al estilo de Cities: Skylines 2 (18/11/2026, versión 0.106)

Pedido de Antonio: trazar y editar calles de forma ágil y rápida, sin perder el minimalismo y con el control de todo a mano.

- **La barra**: con la herramienta Calle, una barra pequeña flota abajo en el centro del dibujo, en planta, en 3D y en la Maqueta. Lleva todo lo de trazar sin ir a Propiedades:
  - **El tipo de calle**: su nombre; al pulsarlo se abren las tarjetas de todos los tipos, dibujados en sección.
  - **Cómo se traza**: Continua (la curva que sigue a la anterior), Recta, Curva y Eje (el eje entero por sus puntos).
  - **Cuadrícula**: tres clics. Una esquina, el final del primer lado y hasta dónde llega el fondo. Pone una calle cada 80 m (se cambia en la barra) en los dos sentidos, ajustada para que los dos lados acaben en calle, con sus cruces y sus cebras. Todo en un paso de Deshacer.
  - **Paralela**: clic en una calle, del lado donde va la nueva, a la distancia de la barra (30 m de partida).
  - **Mejorar**: clic en una calle. Le pone el tipo y los materiales de la barra sin mover su eje, y se queda con sus pasos de peatones y orejas.
  - **Terreno** y **Anarquía** encendidos o apagados.
  - **La altura**: − y + o las teclas AvPág y RePág, de metro en metro. Por encima o por debajo de la planta, la calle deja de seguir el terreno: así se hacen los pasos elevados y los subterráneos.
- **Medidas junto al cursor**: mientras se traza un tramo, en una etiqueta azul van su longitud, su rumbo desde el norte, el radio si es curvo y la pendiente si sigue el terreno (o la altura si va por encima). Con la cuadrícula se ven las calles que va a poner y cuántas son.
- **Para el programador**: `app/street_bar` (`RoadAct`, `grid_streets`, `lay_grid`, `road_act_click`, `road_height_keys`, `street_bar`, `draw_street_readout`) y los campos `act`, `grid_spacing` y `parallel` de `RoadSettings`. Sin cambios de formato.

### 2.14 Calles plásticas (19/11/2026, versión 0.108)

Antonio, al probar la 0.107: «no tiene nada de plasticidad, no se parece en absoluto a Cities: Skylines 2». Le propuse cuatro faltas y marcó las cuatro. Esta es la primera vuelta de cada una.

- **Ver la calle real al trazar.**
  - En planta, mientras se traza, se dibuja la calle tal como quedará, con sus aceras y el cruce que hace con las que pisa ya formado; antes la tapaba una banda azul.
  - En 3D, la calle se ve con sus materiales y las calles que encuentra se rehacen con sus cruces en el acto (`road_overlay_3d`, `Scene::of_solids_unmarked`, `joined`).
- **Moldear arrastrando.** Una calle seleccionada ya no enseña un tirador por cada punto de su eje (una curva tenía docenas), sino solo sus nudos (extremos y cruces) y un tirador redondo en medio de cada tramo entre nudos.
  - Al tirar del redondo, el tramo se curva suave sin mover sus nudos.
  - Al mover un nudo, se lleva todas las calles que llegan a él, cada una doblándose suave hasta su nudo siguiente.
  - En el código: `drag::Grip::RoadNode`/`RoadBend`, `road_nodes`, `node_roads`, `bend_road` y, en el núcleo, `shape_axis`, con paso suave en los dos lados para que la calle conserve su dirección en los nudos.
- **Encuentros sin pisarse.** Al trazar, el clic se pega al centro de un cruce que ya está, a menos de 10 m o del ancho de la calle (`NODE_PULL`). Si la calle va a hacer un cruce encima de otro o de una glorieta, la etiqueta del cursor se pone roja: «cruce encima de otro» (`ghost3d::crowded`).
- **Curvas continuas.** Las calles traídas de OpenStreetMap redondean sus quiebros con un radio según la clase de vía (40 m las principales, 12 m las de barrio), pero no donde llega otra calle (`osm_streets::round_bends`, `bend_radius`).

Lo que falta para la segunda vuelta:
- tiradores en 3D;
- enseñar los tiradores al pasar el ratón, sin seleccionar antes;
- que un nudo arrastrado se pegue a otra calle y se una a ella;
- que, en vez de solo avisar, un cruce pegado a otro se junte con él;
- cambios de ancho con transición al juntar dos tipos de calle.

### 2.15 Calles plásticas, segunda vuelta (19/11/2026, versión 0.109)

Lo que quedó de la primera vuelta, hecho:

- **Tiradores en la Vista 3D.** Con una calle seleccionada salen en 3D los mismos tiradores que en planta: cuadrados en los nudos, un redondo en medio de cada tramo y flechas en los bordillos. Se arrastran sobre la altura de la calzada, y la calle se ve como quedará, con sus cruces (`app/road_grips3d`).
- **Tiradores al pasar el ratón.** En planta, con la flecha, al pasar por encima de una calle sin seleccionarla salen sus tiradores, y se puede tirar de ellos directamente; al cogerlo, la calle queda seleccionada. El cuerpo de la calle no se mueve sin seleccionarla antes (`hover_road`, `grip_here`, `street_under`).
- **Un extremo suelto se une a otra calle.** Al soltar el extremo libre de una calle cerca de otra calle, de su extremo o de un cruce, el extremo se pega a ella y hace el cruce (T, L o uno más en el cruce).
  - Si se suelta en el extremo de otra calle que sigue en línea (menos de 35° de giro), las dos pasan a ser una sola calle continua.
  - Se unen solo si tienen los mismos materiales y ninguna lleva rasante, cambios de sección, tramos ni piezas ocultas propias.
  - Soltado junto a una glorieta o una plaza, el extremo va a su bordillo, mirando a su centro, y entra en ella.
  - Dos calles de tipos distintos también se unen, si se construyen igual (bordillo, canto y nivel): la segunda conserva su sección y entre las dos hay una transición de hasta 12 m alrededor de la junta (un cambio de sección, `JOIN_TRANSITION`).
  - En el código: `road_shaping::pull_end`, `is_loose_end`, `join_ends`, `NODE_REACH`, `JOIN_TURN`.
- **Cruces que se juntan solos.** Si al trazar o al moldear una calle hace un cruce pegado a otro (se pisarían), la calle se dobla suave para pasar por el centro del otro cruce y queda un único cruce.
  - Deja fuera los otros cruces que hacía con las mismas calles, para no cruzarlas dos veces.
  - El aviso en rojo queda solo para lo que no se puede juntar, como un cruce dentro de una glorieta.
  - En el código: `road_shaping::fuse_crowded`, `too_near`, `crowded`.
- **Ensanchar un tramo por su bordillo.** Las flechas de los bordillos, una por lado en cada tramo entre nudos, ensanchan o estrechan ese tramo por su franja de fuera (la acera, por lo general), con transiciones de un cuarto del tramo, 10 m como mucho. El otro bordillo y el resto de la calle no se mueven.
  - Se guarda como cambios de sección (U2), así que pasa al IFC y a las mediciones como cualquier cambio de sección.
  - En el código: `Road::widened`, `road_shaping::widen_stretch`, `edge_point`, `Grip::RoadWidth`.
- Lo que se movió: `road_nodes`, `node_roads` y `bend_road` pasaron de la app a `archisim_geom::road_shaping`.

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

### 4.4 Cuarta vuelta (18/11/2026): lo de N1–N9

Medido en el mismo sitio de 1 km² (`archisim territorio-grande out/`, que ahora mide también nudos, superficies, manzanas, mapa y tráfico; y la prueba `territory_scene`, que mide la vista 3D tras cada cambio):

| Qué | Antes | Ahora |
|---|---|---|
| Vista 3D tras elevar un cruce (N1) | 512 ms (36 piezas rehechas) | **95 ms** (3) |
| Vista 3D tras cambiar el pavimento de un tramo (N4) | 430 ms (18) | **170 ms** (6) |
| Vista 3D tras añadir un aparcamiento (N5) | 37 ms | 30 ms |
| Cambiar 60 nudos / cruces de nuevo (N1) | — | 0,6 ms / 6 ms |
| Buscar las 130 manzanas / partirlas en 1 633 parcelas (N7) | — | 5 ms / 0,2 ms |
| Mapa de todo el sitio, 1 600 px (N8) | — | 16 ms |
| Un paso del tráfico con 400 coches y 600 personas / sus cuerpos (N9) | — | 0,15 ms / 1,6 ms |

Lo que era lento: **un nudo, un tramo o una pieza oculta contaban como si cambiase la calle entera**. La vista 3D rehacía todo lo que está a 30 m de la calle, y en una avenida de 1 km eso son decenas de calles y edificios. Ahora, si la calle no ha cambiado de forma, solo se mira alrededor de lo tocado: el cruce del nudo, el tramo o la pieza (`local_road_change` en `view3d::scene_build`). La prueba `a_kept_scene_is_the_whole_one` lo comprueba con un nudo elevado, un tramo pavimentado y una marca oculta.

Lo demás de N1–N9 ya cumple el presupuesto en esta máquina de 2 núcleos. Queda lo de antes: la planta del sitio entero a 1:2000 (1,5 s, casi todo los 13 692 objetos) y abrir el sitio la primera vez (~27 s).

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

**De N4 (tramos y sustituir)**
- En planta los tramos no se distinguen: la planta dibuja las franjas, no su material (salvo con presentación). Pintarlos con su color o una trama.
- Árbol, farola o banco «por tramo» de la calle: hoy se cambian los objetos que haya (Sustituir todos en un tramo); la calle no genera su mobiliario por sí sola, lo pone «La vida de la calle».
- La zona es un rectángulo; un contorno libre o una parcela/manzana elegida.
- Los tramos se cambian tecleando; tiradores en planta para arrastrar sus extremos.

**De N5 (superficies)**
- Sin huecos (un alcorque o una fuente dentro de la plaza) ni contornos curvos.
- Las plazas de aparcamiento no dejan el sitio de las de movilidad reducida ni las numeran; no siguen un lado inclinado (van paralelas al más largo).
- La superficie no recorta el terreno: se apoya sobre él (un jardín hundido o una plaza en bajada, no).
~~Mover sus esquinas con tiradores; hoy se rehace con Editar contorno o se dibuja de nuevo.~~ **Hecho (0.110)**: las esquinas de las superficies y de las parcelas se arrastran (`Grip::OutlineCorner`).
- Las juntas son una cuadrícula; aparejos (espiga, rombo) y despiece del encintado, no.

**De N6 (ocultar y soltar)**
~~Al soltar una cebra, los bordillos rebajados se quitan con ella; habría que dejarlos donde estaban o moverlos con la cebra.~~ **Hecho (cuarta vuelta)**: la calle oculta las barras y se queda con los rebajes donde estaban.
- Soltar también las flechas y líneas de un nudo (hoy solo se ocultan) y las señales (pasarían a ser un objeto «señal»).
- Ocultar por ventana (varias piezas de una vez) y ver las ocultas en gris para devolverlas una a una.
- Ocultar en 3D y en la Maqueta (hoy, desde la planta).

**De N7 (manzanas)**
~~Al mover una calle, las parcelas y los edificios que la miran no la siguen; hay que vaciar, borrar las parcelas y volver a parcelar (C5 de calles).~~ **Hecho (0.110)**: al cambiar las calles, las manzanas que cambian de forma se vuelven a parcelar solas, con la zona que tenían, sin tocar las parcelas bloqueadas ni las dibujadas a mano, y se vuelven a edificar si lo estaban (`blocks::follow_streets`, desde `Editor::transact`; un aviso dice cuántas).
~~Al trazar una calle sobre edificios del mundo, no avisa ni los quita.~~ **Hecho (0.110)**: los quita, en el mismo paso de Deshacer, y lo avisa.
~~Las manzanas no se guardan: se buscan cada vez. Parcelar una manzana ya parcelada se niega (habría que poder «volver a parcelar» quitando lo no bloqueado).~~ **Hecho en parte (cuarta vuelta)**: un clic en una manzana ya parcelada la vuelve a parcelar sin tocar las bloqueadas (`blocks::replot_block`). Siguen buscándose cada vez (5 ms en 1 km²).
- Las parcelas son de un frente fijo; mezclar anchos (como un casco real) o leer la parcelación del Catastro y respetarla.
~~Los edificios de esquina son un rectángulo a una calle: falta la casa de esquina con dos fachadas.~~ **Hecho (0.110)**: la parcela de esquina se edifica entera, con fachada a las dos calles (`blocks::street_sides`, `Model::add_world_building_on`); las casas con jardín delantero siguen como estaban.
- Pintar zonas con un pincel sobre varias parcelas a la vez.
~~La planta tiñe las parcelas y las rotula solo si cabe el texto; una leyenda de zonas.~~ **Hecho en el mapa (cuarta vuelta)**; en la planta no.

**De N8 (área grande)**
- El recuadro no se puede girar ni dibujar a mano (un contorno libre o el término municipal).
- Las calles que ya estaban en el proyecto no se recortan al cambiar el área (solo se cortan al traerlas).
- El terreno se trae con unos 4 000 puntos: en 2 × 2 km quedan a unos 30 m; con más detalle, por teselas.
~~El mapa no se exporta desde la aplicación (sí desde `archisim mapa`) ni tiene leyenda ni capas que encender y apagar.~~ **Hecho en parte (cuarta vuelta)**: «Guardar el mapa…» en PNG o SVG, con leyenda. Faltan las capas.
- Las conexiones con fuera no dicen a dónde van (el pueblo o la carretera de destino).

**De N9 (tráfico)**
~~Dos coches que entran al cruce desde calles distintas pueden pisarse dentro: no se ceden el paso (ni ceda el paso ni prioridad a la derecha), solo se respetan semáforos y stop.~~ **Hecho (cuarta vuelta)**: no entra nadie en un camino que cruza otro ocupado, y con ceda el paso o stop se espera a quien llega por una calle sin señal. La prioridad a la derecha, no.
~~Los semáforos no se ven encenderse en 3D (el poste no cambia de color) y su ciclo es fijo.~~ **Hecho en parte (cuarta vuelta)**: rojo, ámbar y verde encendidos en cada cabeza (`Traffic::lamps`). El ciclo sigue fijo.
- Coches y personas son cajas de colores; usar los modelos de coche y persona de la biblioteca (con sus versiones de lejos) cuando el rendimiento lo permita.
- Las personas no cruzan por los pasos de peatones ni entran en plazas o parques.
- Sin autobuses en su carril ni tranvía.
- En la Maqueta no hay botón propio; se pone desde Urbanismo.

**De N2 (trazado)**
- La pendiente uniforme elegida con el ratón no enseña marcas en planta entre los dos clics (solo el aviso).
- Cotas de ángulo y radio dibujadas sobre la planta mientras se traza (hoy, en la barra y en Propiedades).
