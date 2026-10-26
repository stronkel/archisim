# Renderizado y presupuestos

Bloque 7 de la revisión de Antonio del 22/10/2026 (v0.47).

## 1. Renderizado

Una imagen realista del modelo, hecha por ArchiSim sin programas externos y sin tarjeta gráfica (vale en cualquier ordenador).

**En la aplicación.** En la vista 3D, Propiedades → **Renderizado**:

- **Calidad**: *Borrador* (800 píxeles de ancho, unos segundos, para ver la luz), *Normal* (1600) o *Alta* (2800, para presentar).
- **Renderizar** hace la imagen desde la cámara de la vista 3D, con la forma de la ventana. Se calcula aparte: se puede seguir trabajando mientras la barra avanza. Al terminar se abre la imagen con **Guardar PNG…** y **Volver a renderizar** (desde la cámara de ese momento).

**En la línea de órdenes.** `archisim render proyecto.archisim imagen.png 1600` hace la imagen desde el suroeste con todo el modelo encuadrado; `--atardecer` o `--noche` cambian la luz y `--sin-texturas` deja los colores lisos.

**Luz y texturas (C4, v0.65).** La imagen lleva la luz de la vista 3D (Día, Atardecer o Noche, en la Maqueta) y sus texturas (salvo con «Sin texturas»):

- **Atardecer**: el sol del proyecto más bajo de fuerza y anaranjado, el cielo cálido y las lámparas a media luz.
- **Noche**: sin sol, un cielo azul oscuro y las lámparas encendidas: pantallas, bombillas, velas, chimeneas y farolas brillan y alumbran lo que tienen cerca, **con sus sombras** (la pantalla de una lámpara no tapa su propia luz).
- **Texturas**: las mismas que la vista 3D (ladrillo, piedra, madera, tablas, hormigón, telas, fibras…), sacadas de imágenes que se repiten sin costura (`archisim_render::textures`).

**Qué lleva la imagen**

- Los mismos sólidos que la vista 3D y el IFC: muros con el color de cada capa, caras pintadas, el color y la textura de cada material, muebles, vegetación, coches, modelos importados.
- **El sol del proyecto** (Propiedades del proyecto → Sol: día y hora solar, con la latitud de la georreferencia y el norte): luz directa y **sombras arrojadas**, las mismas que las de las plantas y axonometrías de presentación.
- **La luz del cielo**: cada punto se aclara u oscurece según cuánto cielo ve (los rincones, los aleros y el suelo bajo los árboles quedan en penumbra).
- **El vidrio** refleja el cielo y deja pasar la luz (y las sombras del vidrio son más suaves).
- Sin terreno, un suelo claro bajo el edificio; con terreno, el del proyecto. Lo que queda fuera del modelo es una bruma clara, como una maqueta fotografiada.

**Lo que no hace todavía**: interiores con rebotes de luz entre paredes, reflejos en el suelo, desenfoque, gente y fondos fotográficos. La caja de sección de la vista 3D no recorta la imagen.

## 1 bis. El proyecto para D5, Twinmotion o Blender (C4, v0.65)

**Archivo → Exportar modelo 3D (glTF) para D5, Twinmotion, Blender…** (o `archisim glb proyecto.archisim modelo.glb`; `archisim exportar` y la demo lo escriben también) guarda el proyecto entero en un solo fichero `.glb` (glTF 2.0):

- Un objeto por elemento, con su nombre («Muro 12», «Sofá de 3 plazas 40»…), y dentro una pieza por material.
- Los materiales con su nombre y su color; los que tienen textura la llevan como imagen, puesta en cada cara igual que en la vista 3D, para cambiarla por la del programa de render si se quiere. El vidrio va transparente.
- En metros y con el eje vertical como lo pide glTF, así que entra a escala y de pie.
- Se comprueba con `python3 scripts/validate_glb.py modelo.glb` (requiere `pip install pygltflib trimesh`).

Las texturas de la vista 3D pueden verse también **«En imagen»** (Propiedades de la vista 3D → Texturas): las mismas, sacadas de esas imágenes en lugar de calcularse en cada píxel. Es lo que usan por defecto los Mac con Intel, que se quedaban colgados con las completas; con OpenGL (tarjetas antiguas) se ven las ligeras.

## 2. Presupuestos con bancos de precios

Las **Mediciones** (Navegador → Proyecto → Tablas → Mediciones) empiezan ahora por el **presupuesto**: los capítulos de siempre (demoliciones, fábricas y muros, forjados, carpintería, acabados de locales, estructura, cubiertas, escaleras y barandillas, muros cortina, instalaciones), cada partida con su cantidad medida del modelo y, a la derecha, **precio**, **importe**, total de cada capítulo y **presupuesto de ejecución material**.

**El banco de precios.** **Abrir banco BC3…** lee cualquier banco en FIEBDC-3: los de las comunidades autónomas, el del estudio o uno exportado de Presto o Arquímedes (ANSI, MS-DOS 850 o UTF-8). De cada concepto toma el código, la unidad, el resumen, el precio y el texto largo; si una partida no trae precio propio, lo calcula con su descomposición (mano de obra, maquinaria y materiales con sus rendimientos). ArchiSim recuerda el último banco abierto.

**Asignar una partida.** **Partida…** en cada fila abre el buscador del banco: se escribe (sin preocuparse de tildes) y salen las partidas que tienen todas las palabras, primero las de la misma unidad que mide el modelo; al pasar por encima se ve el capítulo y el texto. **Asignar** la copia en el proyecto: código, unidad, resumen, precio y texto viajan con el `.archisim`, sin necesitar el banco después.

- El **precio** se puede escribir a mano en su casilla (con o sin partida del banco); vaciarla o **✕** quita el precio.
- Si la unidad de la partida no es la que mide el modelo (por ejemplo, una carpintería en m² frente a unidades), aparece un **⚠** con la diferencia.
- La partida se asigna a lo que se mide, no a un elemento: a un **tipo** de muro, forjado, carpintería, pilar o viga, cubierta o muro cortina; a un **acabado** de los locales (suelo, paredes o techo, por su nombre); a las escaleras, las barandillas, cada red de instalaciones por diámetro o cada clase de aparato. Al dibujar más muros de ese tipo, el importe sube solo.

**A Presto.** **Exportar BC3…** escribe el presupuesto entero: cada partida con el código del banco, su resumen, su precio, su texto largo (`~T`) y sus líneas de medición elemento a elemento (muro a muro con largo y alto y los huecos a deducir, local a local…). Dos tipos con la misma partida del banco se juntan en una partida con las mediciones de ambos. Las partidas sin precio salen con precio cero para completarlas en Presto.

**El banco de ejemplo.** `docs/ejemplos/banco-ejemplo.bc3` es un banco pequeño hecho para probar (precios orientativos inventados, no de ningún banco oficial; desde v0.57 sus partidas traen descomposición: oficial, peón, medios auxiliares, materiales y un 2 % de costes directos complementarios); `archisim demo out/` lo copia en `out/` y la vivienda de prueba ya trae sus partidas asignadas desde él.

### Descomposición, gastos generales y beneficio industrial (v0.57)

- **Descomposición.** Al asignar una partida del banco se copia también cómo está hecha: cada pieza con su código, unidad, resumen, tipo (mano de obra, maquinaria, material u otros), rendimiento y precio. El botón **▸** de la fila la despliega debajo; el rendimiento y el precio de cada pieza se pueden cambiar allí, y el precio de la partida se rehace con ellos. Las piezas de porcentaje (código que empieza por `%` o unidad `%`: costes directos complementarios, medios auxiliares) se calculan sobre la suma de las piezas de encima, como en Presto.
- Si se escribe el precio de la partida a mano, deja de tener descomposición. Si la descomposición no suma el precio (porque el banco da un precio redondeado), aparece un aviso en rojo debajo.
- **Del PEM al PEC.** Al final de la tabla: presupuesto de ejecución material, **gastos generales** (13 % por defecto) y **beneficio industrial** (6 %), presupuesto de ejecución por contrata, **IVA** (21 %) y total. Los tres porcentajes se cambian en sus casillas y se guardan con el proyecto.
- **En el BC3.** Cada partida con descomposición sale con su `~D` y un `~C` por pieza con su tipo FIEBDC (1 mano de obra, 2 maquinaria, 3 material); los porcentajes de gastos generales, beneficio e IVA van en el registro `~K`. Leído otra vez como banco (o en Presto), la partida sale con sus piezas.

**Pendiente**: añadir o quitar piezas a mano (hoy se editan las que trae el banco), coeficiente de costes indirectos propio del proyecto, y partidas por elemento suelto.
