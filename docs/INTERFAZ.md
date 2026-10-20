# ArchiSim — Interfaz por disciplinas (diseño)

> 20 de octubre de 2026 · Diseño acordado con Antonio en conversación, a partir de su boceto de hace unos años. **Sin código todavía**: este documento es lo que se programará en los bloques del §10, y Antonio lo revisa antes.
> Sustituye a la caja de herramientas por secciones de la v0.16 (Selección, Modelado, Documentación, Dibujo, Edición, Vistas), que con la fase 2 y 3 ha llegado a unas 50 herramientas en una sola lista.
> Los iconos acordados están dibujados en `docs/interfaz/iconos.svg`.

## 1. Objetivo

Limpia, funcional, elegante y moderna, pero casi como un juego. Pensada para arquitectos de todas las edades y de cualquier nivel de BIM: el estudiante y el arquitecto mayor tienen que encontrar las cosas sin buscarlas; el experto no puede perder ni un clic.

Tres ideas la sostienen:

1. **La disciplina es un modo de trabajo**, no un menú: cambia las herramientas *y* la forma de ver el modelo.
2. **Todo está siempre en el mismo sitio y con el mismo tamaño**: nada se mueve, nada cambia de forma al elegir una herramienta.
3. **Hay un atajo para todo**: teclas y buscador de órdenes, para que el paso por la disciplina no frene a nadie.

## 2. Zonas de la ventana

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Barra superior: menús · proyecto · (equipo) · deshacer · guardar · tema  │
├────┬──────────────┬──────────────────────────────────────┬───────────────┤
│    │ Herramientas │ Pestañas de vistas                   │ Navegador     │
│ D  │ de la        ├──────────────────────────────────────┤  Modelo       │
│ i  │ disciplina   │ Barra contextual (método / modificar)│  Proyecto     │
│ s  │ (fichas)     ├──────────────────────────────────────┤               │
│ c  ├──────────────┤                                      │               │
│ i  │ Propiedades  │            Dibujo                    │               │
│ p  │ (selector de │   (papel blanco; 3D con cubo de      │               │
│ l  │  tipo con    │    vistas y modos de visualización)  │               │
│ i  │  dibujo,     │                                      │               │
│ n  │  grupos)     │       pastilla de teclas del paso    │               │
│ a  │              ├──────────────────────────────────────┤               │
│ s  │              │ Barra inferior: escala · línea ·     │               │
│    │              │ relleno · texto · cota · forzados · XY│               │
└────┴──────────────┴──────────────────────────────────────┴───────────────┘
```

### 2.1 Barra superior

Menús (Archivo, Edición, Vista, Ayuda), nombre del proyecto y fichero («sin guardar»), buscador de órdenes, deshacer y rehacer, guardar, mostrar u ocultar cada columna, tema claro u oscuro. Con el trabajo en equipo (§9) aparecen aquí los **avatares** de quién está conectado y el botón **Publicar**.

**Tema claro por defecto**, oscuro con un botón, como ahora. El dibujo es siempre papel blanco en los dos.

### 2.2 Barra de disciplinas (izquierda del todo)

Una columna estrecha de iconos. Arriba, la **Flecha** (seleccionar), siempre disponible. Debajo, las disciplinas en este orden:

| # | Disciplina | Para qué |
|---|---|---|
| 1 | **Concepto** | Anteproyecto: volúmenes al estilo SketchUp, masas con edificabilidad, ejes |
| 2 | **Arquitectura** | El edificio |
| 3 | **Estructura** | Pilares, vigas, forjados y muros portantes, cimentación |
| 4 | **Instalaciones** | Fontanería, climatización, electricidad e iluminación |
| 5 | **Urbanismo** | Parcela, terreno, viales, vegetación |
| 6 | **Documentación** | Cotas, etiquetas, secciones, alzados, planos, tablas |
| 7 | **Análisis** | Comprobaciones del propio proyecto: CTE, mediciones |
| 8 | **Coordinación** | Vínculos, módulos, interferencias, incidencias, requisitos, equipo |

Al pie de la columna, **Inicio** (proyectos recientes y plantillas) y **Preferencias**.

### 2.3 Herramientas de la disciplina

Fichas en **cuatro columnas** con icono y nombre (variante B, elegida por Antonio): 54 px de alto, sin borde en reposo, fondo suave al pasar el ratón y borde y fondo de acento en la herramienta activa. La tecla sale al pasar el ratón.

**Formato fijo**: el tamaño de las fichas no cambia nunca, ni al elegir una herramienta ni al cambiar de disciplina. La disciplina con más herramientas (Arquitectura, 12) ocupa tres filas; Propiedades se queda siempre con el resto de la columna.

### 2.4 Propiedades (debajo de las herramientas)

Como la paleta de Revit, en la misma columna que las herramientas, para que elegir herramienta y tipo sea un movimiento corto del ratón:

- **Selector de tipo** con el **dibujo del tipo a 1:20** (corte en planta del muro, capas del forjado, la puerta o ventana en planta), su nombre y medidas, y la **galería de tipos** con buscador.
- Los parámetros en **grupos plegables**: Restricciones (base, superior y desfases), Construcción, Dimensiones, Capas, Datos de identidad.
- **Los ajustes de cada método de dibujo** (referencia del muro, lados del polígono, radio) están aquí, no en la barra contextual, para que esta no cambie de largo (decidido por Antonio).
- Con una herramienta activa muestra lo que se va a dibujar; con una selección, lo seleccionado; sin nada, la vista y el proyecto.

### 2.5 Pestañas de vistas

Pestañas de verdad sobre el dibujo, como en el boceto de Antonio (no burbujas flotantes): cada planta con su escala, el 3D, cada sección, alzado y plano. «+» abre una vista; × o clic con la rueda la cierra.

### 2.6 Barra contextual (debajo de las pestañas)

La pieza nueva, el equivalente de la pestaña *Modificar* de Revit, pero **fija bajo las pestañas, pegada al dibujo**: no aparece arriba del todo ni flota por la pantalla (lo que no convence a Antonio del menú de ArchiCAD: se mueve, cambia de opciones y no es claro). No oculta nada: siempre está ahí, en el mismo sitio.

**Con una herramienta activa**: el nombre de la herramienta, sus **métodos de inserción** y los campos de longitud y ángulo (se rellenan al mover el ratón y se pueden teclear).

| Método | Para |
|---|---|
| Recta | Un tramo |
| Cadena | Tramos seguidos |
| Rectángulo | Cuatro tramos o un contorno rectangular |
| Arco | Por tres puntos o centro y extremos |
| Círculo | Por centro y radio |
| Spline | Curva libre (muros curvos, bordes de terreno, viales) |
| Elegir línea | Seguir una línea existente (del dibujo 2D, de un DXF o de otro elemento) |

Cada herramienta ofrece los que le sirven (una puerta, ninguno); los que no aplican se ven apagados, no desaparecen.

**Con algo seleccionado**: «Modificar · Muro (1)» y dos grupos, siempre en el mismo orden:

| Grupo | Órdenes (tecla) |
|---|---|
| Transformar | Mover (D) · Copiar (CP) · Girar (GI) · Simetría (SI) · Escala (ES) · Matriz (MA) |
| Geometría | Desfase (EQ) · Alinear (ALI) · Recortar (RR) · Alargar (AL) · Partir (PA) · Unir (JUN) |

Las teclas son las mismas que la línea de órdenes al estilo AutoCAD (`cad::ALIASES`). Lo que no vale para la selección se ve apagado (escalar una puerta); con una selección mixta solo se encienden las órdenes que valen para todos. **Sin selección**, la barra se ve en gris, sin desaparecer.

**Dos formas de mover, copiar, girar y simetría** (Antonio trabaja como en Revit y está abierto a mejoras):

- **Rápida**: con la selección hecha, arrastrar mueve; con Ctrl, copia (como hoy).
- **Precisa, como en Revit**: botón (o tecla) → punto base → destino, con distancia y ángulo tecleables en la barra. Girar: centro → ángulo de referencia → ángulo final. Simetría: eje.

### 2.7 Dibujo

Papel blanco. En 3D, el **cubo de vistas** arriba a la derecha y los **modos de visualización** (aristas, sombreado, líneas ocultas, realista) arriba a la izquierda, como en el boceto de Antonio. Abajo, la **pastilla con las teclas del paso** («Clic: inicio · Longitud + Intro · Esc: terminar») y los avisos que se desvanecen.

### 2.8 Barra inferior

Segmentada a lo ancho del dibujo, como en el boceto de Antonio:

**Escala** (1:20…1:200) · **Línea** · **Relleno** · **Texto** · **Cota** · forzados (extremos, medios, aristas, rastreo, en iconos) · **coordenadas X Y** y punto forzado.

Línea, relleno, texto y cota son el dibujo y la anotación 2D de la vista: están aquí, siempre a mano, en cualquier disciplina. Los forzados y las coordenadas están abajo (decidido por Antonio).

La **línea de órdenes** al estilo AutoCAD sigue existiendo: aparece justo encima de la barra inferior al empezar a escribir una orden, y se puede dejar fija desde Vista.

### 2.9 Navegador (derecha)

Dos pestañas:

- **Modelo**: la escena por disciplina (Arquitectura, Estructura, Instalaciones, Urbanismo, Dibujo 2D, Vínculos y módulos), cada una con su **ojo** (ver u ocultar) y su **candado** (bloquear), y dentro sus tipos con el número de elementos (como ahora).
- **Proyecto**: plantas, vistas 3D, secciones y alzados, planos, tablas, carpetas.

## 3. La disciplina como modo de trabajo

Al elegir una disciplina cambian dos cosas:

1. **Las herramientas** de la columna.
2. **La vista**: lo que no es de la disciplina se atenúa, sin ocultarse, para trabajar sobre ello como referencia.

| Disciplina | Cómo se ve lo demás |
|---|---|
| Concepto | Los elementos del edificio en volumen simple, gris |
| Arquitectura | Todo normal; la estructura con su trama |
| Estructura | La arquitectura en gris; pilares, vigas, forjados y muros portantes resaltados |
| Instalaciones | La arquitectura en gris claro |
| Urbanismo | El edificio en volumen simple |
| Documentación | Todo a su color normal (se anota sobre todo) |
| Análisis, Coordinación | Todo normal; los resultados en color (incumplimientos, interferencias) |

La atenuación es un estilo de vista (`graphics`), así que también se puede usar al imprimir (una planta de estructura sale con la arquitectura en gris). Se puede apagar en Preferencias.

**Atajos que saltan la disciplina**:

- Las **teclas** de herramienta valen desde cualquier disciplina (M siempre es muro, I pilar). Al usarla, la disciplina cambia sola a la de esa herramienta.
- El **buscador de órdenes** (Espacio, o en la barra superior): se escribe «viga», «zapata», «cota de nivel», «girar» y se elige. Enseña, además, en qué disciplina está cada cosa y su tecla.
- Una herramienta puede estar **en varias disciplinas** (forjado en Arquitectura y en Estructura, con un tipo por defecto distinto; ejes en Concepto, Arquitectura y Estructura; forma en Concepto y Urbanismo).

## 4. Herramientas por disciplina

**Concepto** (12): Línea · Rectángulo · Círculo · Empujar · Seguir · Equidistancia · Volumen · Masa · Cinta · Pintar · Ejes · Convertir.

- Las líneas de Concepto son **aristas en 3D** (como SketchUp): al cerrar un contorno se crea una cara que se puede empujar. No son las líneas de dibujo 2D de la barra inferior; en Propiedades se distingue «Arista del modelo» de «Línea de dibujo».
- **Masa**: volumen por plantas con superficie construida, edificabilidad y ocupación en directo mientras se empuja.
- **Convertir**: pasa el volumen a edificio (caras en muros, plantas en forjados, tapa en cubierta) con los tipos elegidos, y recuerda la relación: si el volumen cambia, ofrece actualizar.

**Arquitectura** (12): Muro · Forjado · Puerta · Ventana · Abertura · Cubierta · Escalera · Barandilla · Muro cortina · Objeto · Local · Ejes.

**Estructura** (8): Pilar · Viga · Forjado (estructural) · Muro de carga · Zapata · Losa · Abertura · Ejes.

**Instalaciones** (6, provisionales hasta que lleguen las herramientas MEP): Tubería · Conducto · Rejilla (de ventilación) · Aparato · Iluminación · Mecanismo (enchufes e interruptores).

**Urbanismo** (5): Terreno · Parcela · Vial · Vegetación · Forma.

**Documentación** (8): Cota · Cota de nivel · Etiqueta · Texto · Sección · Alzado · Plano · Tabla. (Las cotas angular, de radio y de arco, la directriz y la nube de revisión, como métodos o variantes de Cota y Texto.)

**Análisis** (2, irá a más): CTE · Mediciones.

**Coordinación** (8): Vincular · Módulo · Versiones · Interferencias · Incidencias · Requisitos · Equipo · Publicar.

Las órdenes de edición 2D de AutoCAD (equidistancia, recortar, alargar, empalme, matriz…) dejan de estar en la caja: están en la barra contextual (§2.6) y en la línea de órdenes.

## 5. Iconos

Propios de ArchiSim, **en el lenguaje del dibujo de arquitectura**: cómo se ve cada elemento en un plano (planta, alzado o sección, lo que mejor lo reconozca), no iconos genéricos. Trazo fino de 1,3 px en una cuadrícula de 20 × 20, sin rellenos salvo alguna marca pequeña, del color del texto (de acento cuando están activos). Dibujados por el programa como líneas (`app/icons`), nítidos a cualquier resolución.

Decisiones de Antonio sobre los iconos:

- **Ventana en alzado** (marco, dos hojas y el triángulo de apertura discontinuo), más representativa que su planta.
- **Pilar en alzado** (basa, fuste y capitel), para que no se confunda con el conducto (rectángulo con aspa).
- **Barandilla** con el pasamanos aparte del barandal superior, para que no se confunda con la escalera.
- **Muro cortina** en alzado (montantes y travesaños), distinto de la ventana.
- **Zapata** en sección con el pilar encima (en planta sería un cuadrado como el pilar).
- **Abertura** como rectángulo con aspa, como se dibujan los huecos; por eso la **cubierta** va en alzado (a dos aguas, con su alero), no en planta, donde se confundiría con ella.
- **Iluminación**: círculo con aspa (símbolo de luminaria en planta); **mecanismo**: el de toma de corriente.

## 6. Para todas las edades

- **Tamaño de letra** normal o grande en Preferencias; las fichas, la barra contextual y Propiedades crecen con él.
- Las **teclas de cada paso** en la pastilla del dibujo y en la ayuda de cada ficha y botón.
- **Misiones de iniciación** opcionales (tu primer muro, tu primera planta, tu primer plano), que enseñan dónde está cada cosa. Lo más parecido a un juego sin estorbar al profesional.

## 7. Lo que no cambia

- Las pestañas de vistas, las plantas a su escala, el papel blanco, el selector de tipo con dibujo y galería, los grupos de propiedades, el navegador con número de ejemplares, el tema claro y oscuro: todo lo de las v0.16 a v0.32 sigue, recolocado.
- El modelo, el formato `.archisim` y el IFC: la interfaz por disciplinas no los toca (salvo la disciplina de cada elemento, §8, y los módulos, §9).

## 8. Disciplina de cada elemento

Para atenuar por disciplina, cada elemento tiene que saberla. La mayoría la deduce el programa de su clase (un pilar es estructura, un conducto instalaciones, un vial urbanismo); los que pueden ser de dos (muros y forjados) la toman de su **función estructural** (portante o no, `LoadBearing` en IFC, ya existe en los tipos). Se puede cambiar a mano en Propiedades. Es la decisión D8 de `docs/DISCIPLINAS.md` (un solo modelo, disciplina por elemento), que este diseño da por buena.

## 9. Módulos y trabajo en equipo

### 9.1 Masterplan: viviendas tipo como módulos

Buena práctica que ArchiSim propone como forma natural de trabajar:

- **Una vivienda tipo por fichero** `.archisim`, con sus plantas, tipos y mediciones; se diseña y revisa sola, y la puede llevar otra persona.
- **El masterplan** tiene la parcela, el terreno, los viales y la urbanización, y coloca las viviendas como **módulos**: posición, giro y **simetría** (pareadas, adosadas), tantas veces como haga falta (Coordinación → Módulo). Es lo que en ArchiCAD son los módulos enlazados y en Revit los vínculos con varias copias.
- **Cambias el tipo una vez y se actualizan todas las copias**; el masterplan avisa («Vivienda tipo B ha cambiado: actualizar») en lugar de cambiar a escondidas.
- **Mediciones**: el presupuesto multiplica cada tipo por su número de copias y lleva la urbanización en su propio capítulo.
- **IFC**: cada copia sale con identificadores propios y estables, calculados a partir del de la copia y el del elemento: las 24 viviendas no chocan entre sí y no cambian de una exportación a otra.
- Si una copia necesita algo distinto, se crea **otro tipo** («B con garaje»); no se edita una copia suelta.

Cuándo separar y cuándo no:

| Separar en ficheros (módulos) | Un solo proyecto |
|---|---|
| El mismo tipo se repite en varias parcelas | Elementos muy trabados: un bloque con medianeras y forjados compartidos |
| Lo lleva otra persona u otra disciplina | Repeticiones dentro de un edificio (planta tipo): **grupos** en el mismo fichero |
| El conjunto pesaría demasiado | Variantes del mismo proyecto: **opciones de diseño** |

### 9.2 Varias personas a la vez (futuro, 4.6 de la hoja de ruta)

Como el trabajo en equipo de ArchiCAD, con presencia en directo:

1. **Proyecto compartido** en un servidor: uno del estudio (un NAS) o en la nube. Cada uno trabaja en su copia, también sin conexión.
2. **Reservas**: para modificar un elemento o una zona («Planta 1», «Fachada norte») se reserva; nadie más puede tocarla mientras tanto. Sin conflictos imposibles, el gran problema del fichero central de Revit.
3. **Publicar / Recibir**: se envían y reciben solo los cambios. Encaja con cómo está hecho ArchiSim por dentro: cada edición es ya un paso con nombre y cada elemento tiene identidad estable.
4. **Presencia**: los avatares de quién está conectado en la barra superior; en la vista, lo que tiene reservado cada uno, de su color; si se quiere, su cursor (como Figma). **Historial por elemento**: quién cambió este muro y cuándo.

## 10. Plan por bloques

| Bloque | Contenido | Cambia el formato |
|---|---|---|
| **I1** | Barra de disciplinas, fichas en formato fijo con los iconos nuevos, Propiedades debajo, navegador con pestañas Modelo/Proyecto, buscador de órdenes, teclas que cambian de disciplina | No |
| **I2** | Barra contextual: métodos de inserción de las herramientas que ya existen, y Modificar con sus dos formas (rápida y precisa); barra inferior segmentada con forzados y coordenadas; línea de órdenes encima | No |
| **I3** | Disciplina por elemento y atenuación de la vista por disciplina, también al imprimir; ojo y candado por disciplina en el navegador | Sí (disciplina a mano, si se cambia) |
| **I4** | Disciplina Concepto: aristas y caras, empujar, seguir, volumen, masa con edificabilidad, cinta, pintar, convertir a edificio | Sí |
| **I5** | Módulos: colocar otro `.archisim` varias veces con giro y simetría, actualizar, mediciones por copias, IFC con identificadores por copia | Sí |
| **I6** | Tamaño de letra grande y misiones de iniciación | No |
| 4.6 | Trabajo en equipo (§9.2) | Sí |

Las herramientas de Instalaciones (tubería, conducto, rejilla, aparato, iluminación, mecanismo) son un bloque propio de modelado, fuera de este plan de interfaz; mientras no existan, sus fichas se ven apagadas y dicen cuándo llegan.

## 11. Decisiones para Antonio

| # | Decisión | Propuesta |
|---|---|---|
| D17 | Orden de los bloques I1–I6 | I1 → I2 → I3, después I5 (masterplans de adosadas y pareadas), I4 e I6 |
| D18 | Si una tecla de herramienta cambia la disciplina sola | Sí (si no, la columna mostraría herramientas de otra disciplina que la que se usa) |
| D19 | Atenuación por disciplina activada por defecto | Sí, con interruptor en Preferencias |
| D20 | Servidor del trabajo en equipo: propio del estudio, en la nube o los dos | Los dos, empezando por el propio |
| D21 | Reservas por elemento, por zona (planta, ala, fachada) o las dos | Las dos: por zona para el reparto del trabajo, por elemento para cambios puntuales |
| D22 | Ver el cursor de los compañeros en directo | Opcional, apagado por defecto |
