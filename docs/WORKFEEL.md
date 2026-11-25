# ArchiSim — «Work-feel»: que el programa desaparezca al dibujar

> 24 de noviembre de 2026 · sobre la v0.110 (`main`, 25ca21f). Pedido por Antonio: «el dibujo de arquitectura tiene que tener libertad total para hacer de forma sencillísima lo que se te ocurra, y sin estudiar el programa tienes que poder sacar planos preciosos para una presentación». Referencias: ArchiCAD, Revit y Synaps (synaps.app).

## 1. Diagnóstico

ArchiSim ya **tiene** casi todo lo que hace falta: Habitación (muros, suelo y local de una vez), varita mágica, cotas temporales, líneas de rastreo, modo boceto, estilos de presentación, «Plano de presentación», el botón Entregar, la Maqueta. Lo que falla es el **camino** entre esas piezas:

| Lo que pasa hoy | Por qué molesta |
|---|---|
| Planta, 3D, secciones y planos van en **pestañas**: se ve una cosa cada vez | No ves el edificio mientras lo dibujas; Synaps y el uso real de ArchiCAD con dos ventanas enseñan lo contrario |
| Dibujas cuatro muros cerrados con la herramienta Muro y **no pasa nada más**: el forjado y el local son otras dos herramientas | Habitación lo hace, pero solo si eliges ese método antes; el arquitecto dibuja muros y espera que el programa entienda que ha cerrado una estancia |
| El dibujo de trabajo es **técnico** hasta que eliges un estilo de presentación o haces un plano de presentación | Lo bonito exige saber que existe y dónde está |
| El plano de presentación sale con su estilo, pero **se ve después**, en su pestaña | No hay vista previa en vivo de la lámina mientras se elige estilo, papel o escala |
| Un plano escaneado o una foto **se calcan a mano** (el calco sirve de fondo) | En reformas es el primer trabajo de todos y el más tedioso |
| Las opciones A/B son **copias del proyecto**; los comentarios del cliente van por correo | Synaps los pone sobre el dibujo |

## 2. Qué se aprende de cada uno

- **ArchiCAD**: la paleta que sale al pinchar un elemento (ya la tenemos en la Maqueta: llevarla a Técnica), las cifras que se escriben mientras dibujas sin abrir nada, la planta que ya sale «de presentación» sin configurarla.
- **Revit**: cotas temporales y modo boceto (hechos), plantillas de vista (hechas): pulir, no rehacer.
- **Synaps**: un único lienzo con planta, alzados, secciones y 3D del mismo edificio a la vez; muy pocas órdenes; calco de imagen a dibujo editable; opciones una junto a otra y chinchetas sobre los elementos; historial por elemento e instantáneas con nombre; «acelerar sin quitar la autoría». Su generación de plantas por texto queda **fuera**: no es lo que se busca aquí.

## 3. Bloques W

Mismo modo de trabajo que los bloques C, E y U (§9 de la hoja de ruta): se marca «en curso» antes de tocar código.

| # | Bloque | Qué notará Antonio | Tamaño | Estado |
|---|---|---|---|---|
| W2a | **Puntería y muros sencillos** (§3 bis) | Forzado más generoso que no salta, rótulo de lo que se coge, cadena que se cierra | S | Hecho en `bloque/w2a-punteria` (v0.112) |
| W0 | **Prueba de los 10 minutos** | Una prueba fija en cada versión: vivienda de 90 m² (salón-cocina, dos dormitorios, baño) y una lámina A3 de presentación. Se cuentan clics, tiempo y atascos; la misma prueba en ArchiCAD como referencia. También como prueba automática (clics simulados, como `drawn_project`), que cuenta las órdenes | S | Prueba automática hecha en `bloque/w0-diez-minutos` (v0.113): 90 acciones; falta la de Antonio a mano |
| W1 | **Lienzo partido** | Botón en la barra inferior (y teclas) para ver planta + 3D, planta + sección o cuatro vistas; las vistas siguen a la misma selección (lo elegido se ilumina en todas), el 3D se centra en lo que estás dibujando y lo dibujado aparece al momento en las otras | M | |
| W2 | **Dibujar sin elegir herramienta** | Al cerrar un recinto con la herramienta Muro, una burbuja «¿Hacer estancia?» crea forjado y local en un solo Deshacer (o siempre, si se activa). Doble clic en un muro coloca la puerta o ventana de la última vez. Arrastrar desde el extremo de un muro seleccionado sigue el muro; desde su centro, tira un tabique en escuadra. La paleta de la Maqueta (girar, duplicar, cuentagotas, borrar) también en Técnica | M | |
| W3 | **Bonito por defecto** | Proyecto nuevo con un estilo de presentación ya puesto en la planta de trabajo (conmutable Técnico ↔ Presentación con una tecla). «Entregar → Lámina de presentación» abre una vista previa en vivo: estilos en miniatura, papel, escala y vistas; título, escala gráfica, norte y cuadro de superficies colocados solos; un clic y está el PDF | M | |
| W4 | **Calco de imagen o PDF** | Arrastrar una foto o un PDF de una planta: se escala con una cota conocida y se proponen muros (gruesos y finos), huecos y locales, que se aceptan o corrigen antes de entrar en el modelo | L | |
| W5 | **Opciones y cliente** | Opciones A/B dentro del mismo proyecto, una junto a otra en el lienzo partido; chinchetas de comentario sobre un elemento (BCF por dentro); instantáneas con nombre («Revisión cliente v3») y volver atrás un solo elemento | L | |

**Orden propuesto:** W2a (§3 bis) → W0 → W2 → W3 → W1 → W4 → W5. W0 primero, porque sin medir solo hay opiniones; W2 y W3 son exactamente «dibujar lo que tienes en la cabeza» y «planos preciosos sin estudiar».

## 3 bis. W2a — Puntería y muros sencillos (primero, pedido por Antonio el 24/11/2026)

Antonio: «falla en cosas pequeñas, fricción a la hora de hacer clic, coger puntos de referencia, que los muros se puedan dibujar de forma muy sencilla». Lo que se veía en el código (v0.110):

- Alcance del forzado de **10 px** (`app.rs`, `const REACH: f32 = 10.0`, pasado por `touch::reach` y `view.to_metres`): poco en pantallas grandes o con zoom alejado.
- **Sin histéresis**: `snap::snap_filtered` elige en cada fotograma el punto más cercano, y el marcador salta entre esquina, punto medio y cara.
- No se anuncia antes del clic si el muro continúa otro, hace una T o empieza suelto.

Qué hacer, y cómo quedó (v0.112):

1. Alcance de unos 18 px y un punto cogido que se mantiene hasta alejarse claramente (1,5 veces el alcance). → `SNAP_REACH` = 18 px para las herramientas de dibujo (el de seleccionar sigue en 10); `snap::hold`: el punto cogido se queda mientras el cursor está a menos de `HOLD` = 1,5 alcances, y otro punto solo lo sustituye si el cursor está a menos de la mitad de distancia de él. Se guarda por revisión del modelo, planta y herramienta (`ArchiSimApp::held_snap`).
2. Rótulo junto al cursor con lo que se va a coger: Extremo, Esquina, Punto medio, Sobre la cara, Intersección, Perpendicular. → `snap_caption`, encima del marcador (las medidas van debajo). Con la herramienta Muro dice además «Extremo · continúa el muro», «Sobre la cara · encuentro en T» o «Cerrar».
3. Muros encadenados por defecto; se cierran al volver al primer punto; doble clic o Esc terminan. → La cadena ya era el método por defecto; ahora el primer punto se coge desde 1,5 alcances con al menos dos muros dibujados (`wall_closing_point`), sin que lo tape el rastreo.
4. Longitud tecleada directamente mientras se mueve el ratón, sin pinchar en un campo. → Ya funcionaba (`WallTool::type_char`); queda cubierto por la prueba nueva.
5. Pruebas con clics simulados: `points_are_taken_from_afar_and_kept` (un punto a 15 px se coge, un temblor de 3 px no lo mueve, a 80 px se suelta) y `chained_walls_close_by_their_first_point` (longitud tecleada, cierre a 15 px del primer punto con el rótulo «Cerrar»).

## 4. La prueba de los 10 minutos (W0), para hacerla ya en la v0.110

Sin cambiar nada del programa, Antonio:

1. Proyecto nuevo. Dibuja una vivienda de una planta de unos 90 m² con muros de fachada y tabiques: salón-cocina, dos dormitorios y un baño.
2. Pon una puerta de entrada, tres de paso y cuatro ventanas.
3. Saca los forjados y los locales con sus nombres y superficies.
4. Saca una lámina A3 «para enseñar a un cliente», con planta amueblada y una vista 3D, en PDF.

Apunta: tiempo total, y **cada momento en que dudaste, buscaste algo o hiciste algo dos veces**, con la herramienta en la que estabas. Esa lista decide el contenido fino de W2 y W3.

### Resultado de la prueba automática (v0.113)

`ten_minute_house` (`app/tests.rs`), con `ARCHISIM_TEST_OUT` escribe `diez-minutos.txt` y `diez-minutos.pdf`:

| Paso | Clics en el dibujo | Teclas | Clics en paneles |
|---|---:|---:|---:|
| Fachadas | 5 | 1 | 0 |
| Tabiques | 6 | 4 | 2 |
| Puertas y ventanas | 8 | 2 | 0 |
| Forjado | 5 | 2 | 0 |
| Locales | 4 | 42 | 4 |
| Lámina A3 en PDF | 0 | 1 | 4 |
| **Total (90)** | **28** | **52** | **10** |

Lo que más pesa: nombrar los locales y volver a dibujar el contorno para el forjado. Los dos los quita W2 («¿Hacer estancia?» con nombre propuesto). La prueba falla si una versión necesita más acciones que la última apuntada (`LAST`); cuando un bloque las reduzca, se baja `LAST`.

## 5. Para el programador

- No hay vistas simultáneas: `ViewMode` es una sola vista por pestaña (`app/tabs.rs`). W1 necesita que la planta (`plan_image`, `plan_ui`) y el 3D (`view3d`) se pinten en dos rectángulos del mismo marco, con la selección compartida que ya tiene el editor.
- Habitación ya existe como método (Maqueta y barra contextual de Muro); W2 reutiliza su transacción y la detección de recintos (`archisim_geom` varita mágica) al cerrar una cadena de muros. Pendiente de MQ1: la Habitación en un solo Deshacer.
- `new_presentation_sheet` (`app/sheet_ui.rs`) ya elige estilo y monta la lámina con `archisim_plan::layout::add_presentation_sheet`: W3 le pone delante un diálogo con vista previa.
- El calco actual (`app/underlay_ui.rs`) solo mueve, encaja y escala el fondo; W4 añade la detección de líneas sobre una imagen y la propuesta de muros.
- Puntería (W2a): el alcance de las herramientas de dibujo es `SNAP_REACH` (`app.rs`); el de pinchar elementos, `REACH`. El punto cogido lo guarda `snap_at` (`app/plan_ui.rs`) con `snap::hold`; el rótulo lo pinta `draw_cursor` con `snap_caption`.
