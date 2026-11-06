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

> Sustituida el 6/11/2026 por la **cinta y la barra de opciones** al estilo Revit: ver §13. Lo que sigue se conserva como historia.

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

La **línea de órdenes** al estilo AutoCAD sigue existiendo, pero **oculta por defecto** (decidido por Antonio al probar I1): lo que se escribe aparece junto al cursor y el paso de la orden en marcha, en la barra contextual. Se deja fija desde Vista → Línea de órdenes fija, sin que el dibujo salte.

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
- El **buscador de órdenes** (Ctrl+Espacio, o el campo de la barra superior; Espacio solo ya sirve para la varita, girar puertas y, con las teclas de AutoCAD, como Intro): se escribe «viga», «zapata», «cota de nivel», «girar» y se elige. Enseña, además, en qué disciplina está cada cosa y su tecla.
- Una herramienta puede estar **en varias disciplinas** (forjado en Arquitectura y en Estructura, con un tipo por defecto distinto; ejes en Concepto, Arquitectura y Estructura; forma en Concepto y Urbanismo).

## 4. Herramientas por disciplina

**Concepto** (12): Línea · Rectángulo · Círculo · Empujar · Seguir · Equidistancia · Volumen · Masa · Cinta · Pintar · Ejes · Convertir.

- Las líneas de Concepto son **aristas en 3D** (como SketchUp): al cerrar un contorno se crea una cara que se puede empujar. No son las líneas de dibujo 2D de la barra inferior; en Propiedades se distingue «Arista del modelo» de «Línea de dibujo».
- **Masa**: volumen por plantas con superficie construida, edificabilidad y ocupación en directo mientras se empuja.
- **Convertir**: pasa el volumen a edificio (caras en muros, plantas en forjados, tapa en cubierta) con los tipos elegidos, y recuerda la relación: si el volumen cambia, ofrece actualizar.

**Arquitectura** (12): Muro · Forjado · Puerta · Ventana · Abertura · Cubierta · Escalera · Barandilla · Muro cortina · Objeto · Local · Ejes.

**Estructura** (8): Pilar · Viga · Forjado (estructural) · Muro de carga · Zapata · Losa · Abertura · Ejes.

**Instalaciones** (6, hechas en el bloque Instalaciones): Tubería (H) · Conducto (D) · Aparato · Iluminación · Enchufe · Interruptor. (La «rejilla» que pidió Antonio son los ejes, no la de ventilación; las rejillas y difusores serán un tipo de Aparato.)

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
| **I1** ✅ | Barra de disciplinas, fichas en formato fijo con los iconos nuevos, Propiedades debajo, navegador con pestañas Modelo/Proyecto, buscador de órdenes, teclas que cambian de disciplina | No |
| **I2** ✅ | Barra contextual: métodos de inserción de las herramientas que ya existen, y Modificar con sus dos formas (rápida y precisa); barra inferior segmentada con forzados y coordenadas; línea de órdenes oculta. Pedido por Antonio al probar I1: tablas y mediciones en pestañas, y arrastrar plantas, secciones, tablas y la vista 3D a un plano a su escala | Sí (vistas 3D en planos, versión 14) |
| **I3** ✅ | Disciplina por elemento y atenuación de la vista por disciplina, también al imprimir; ojo y candado por disciplina en el navegador | Sí (disciplina a mano, si se cambia) |
| **I4** ✅ | Disciplina Concepto: aristas y caras, empujar, seguir, volumen, masa con edificabilidad, cinta, pintar, convertir a edificio | Sí |
| **I5** ✅ | Módulos: colocar otro `.archisim` varias veces con giro y simetría, actualizar, mediciones por copias, IFC con identificadores por copia | Sí |
| **I6** ✅ | Tamaño de letra grande y misiones de iniciación (`docs/INICIACION.md`) | No |
| 4.6 ✅ | Trabajo en equipo (§9.2): carpeta compartida en Google Drive primero (D20; `docs/EQUIPO.md`) | No |

Las herramientas de Instalaciones (tubería, conducto, aparato, iluminación, enchufe e interruptor) son un bloque propio de modelado, fuera de este plan de interfaz; están hechas desde el bloque Instalaciones (rama `bloque/instalaciones`).

## 11. Decisiones de Antonio (20/10/2026)

| # | Decisión | Decidido |
|---|---|---|
| D17 | Orden de los bloques | **I1 → I2 → I3 → I5 → I4 → I6** |
| D18 | Una tecla de herramienta cambia la disciplina sola | **Sí** |
| D19 | Atenuación por disciplina activada por defecto | **Sí**, con interruptor en Preferencias |
| D20 | Dónde vive el proyecto compartido | **Los dos, empezando por la nube**: una carpeta compartida de **Google Drive**, que es como trabaja hoy el equipo de AD5; después, un servidor propio |
| D21 | Reservas | **Por zona y por elemento** |
| D22 | Cursor de los compañeros en directo | **Opcional, apagado por defecto** |

Consecuencia de D20 para el trabajo en equipo (§9.2): Google Drive sincroniza **ficheros**, no es un servidor que responda al momento. El proyecto compartido será, por tanto, una **carpeta** con el modelo, un fichero de cambios por persona (cada uno escribe solo el suyo, así Drive nunca tiene que mezclar dos versiones del mismo fichero) y los ficheros de reservas. ArchiSim lee los de los demás al recibir y escribe el suyo al publicar. El mismo esquema sirve tal cual en una carpeta de un NAS o un servidor propio, y más adelante se le puede poner un servidor delante para la presencia en directo (cursores, avisos al momento), que con Drive llegará con unos segundos de retraso.

## 12. Segunda versión (2 de noviembre de 2026)

Decidida con Antonio sobre el prototipo «ArchiSim · interfaz por colores y barras» (pizarra navegable y hoja de disciplinas). Con las funciones de las v0.33 a v0.78, la regla del formato fijo (§2.3) se había roto (Urbanismo con 16 fichas, Arquitectura y Concepto con 15), la barra superior y la inferior mezclaban cosas distintas, las salidas estaban repartidas (render en Propiedades del 3D, estilos en Vista, entorno y PDF en Archivo) y la ventana era más fría que lo que dibuja.

### 12.1 Color por disciplina

Cada disciplina tiene un color suave, en el carril, en la ficha activa, en la cabecera de Propiedades y en el navegador. El azul queda solo para la selección. El dibujo sigue siendo papel blanco: el color está en la ventana, no en la planta.

| Disciplina | Color | Fondo | Texto |
|---|---|---|---|
| Concepto | `#A0784A` arena | `#F5EEE4` | `#6B4F2C` |
| Arquitectura | `#C0582F` terracota | `#FBEDE6` | `#8E3B1C` |
| Estructura | `#4C6A92` pizarra | `#E9EFF6` | `#34507A` |
| Instalaciones | `#16897A` verde agua | `#E2F4F0` | `#0F6A5E` |
| Urbanismo | `#4F8A2B` verde hoja | `#EAF3E0` | `#3A6B1C` |
| Documentación | `#6E6B64` grafito | `#EFEDE8` | `#45433E` |
| Presentación | `#B8457A` frambuesa | `#FBEAF2` | `#8F2F5C` |
| Análisis | `#B87514` ámbar | `#FBF0DC` | `#83500A` |
| Coordinación | `#6E55C2` violeta | `#EFEBFB` | `#5440A0` |

### 12.2 Fichas agrupadas, doce como mucho

Tres filas de cuatro. Las variantes de una herramienta van en una sola ficha con un triángulo en la esquina, que abre las variantes encima del dibujo sin mover nada. Lo que se usa en cualquier disciplina sale de las fichas y va a la barra inferior.

| Disciplina | Fichas (▸ con variantes) |
|---|---|
| Concepto | Arista · Empujar ▸ (Seguir) · Equidistancia · Dividir ▸ (Deformar) · Volumen ▸ (Formas) · Masa · Pintar · Convertir |
| Arquitectura | Muro · Forjado · Puerta · Ventana · Abertura · Cubierta · Escalera ▸ (Ascensor) · Barandilla · Muro cortina · Objeto · Local |
| Estructura | Pilar · Viga · Forjado · Muro de carga · Cimentación ▸ (Zapata, Losa) · Armado · Abertura · Ejes |
| Instalaciones | Tubería · Conducto · Aparato ▸ (Equipos de agua) · Iluminación · Mecanismo ▸ (Enchufe, Interruptor) · Cuadro · Montante · Clima · Incendios · Placas solares |
| Urbanismo | Terreno · Parcela · Calle ▸ (Vial, Glorieta, Marca vial) · Redes ▸ (Pozo y arqueta) · Entorno ▸ (Edificio del entorno, Traer, Descargar) · Piscina · Vegetación ▸ (Plantación, Rocas, Vivero) · Pincel · Mobiliario urbano · Forma |
| Documentación | Sección · Alzado · Etiqueta · Plano · Tabla · Vista de dibujo |
| Presentación (nueva) | Cámara · Sol · Estilo · Render · Imagen · Pegatina · Recorrido (más adelante) |
| Análisis | CTE · Mediciones · Presupuesto · Superficies |
| Coordinación | Vincular · Módulo · Versiones · Interferencias · Incidencias · Requisitos · Equipo · Publicar |

Cambian de sitio: Cinta y Plano de referencia → **Guía**, en la barra inferior; Cota, Cota de nivel y Texto → **Anotar**, en la barra inferior; Pegatina → Presentación; el render, los estilos de presentación, la imagen 3D y el sol → Presentación.

### 12.3 Una regla para cada barra

- **Arriba, el proyecto**: menús, nombre y fichero, buscador, deshacer, guardar, equipo y un solo botón **Entregar** con todas las salidas (IFC 4.3, IFC4, glTF; planos, carpinterías, superficies, mediciones BC3; render e imagen). Las mismas órdenes siguen en Archivo.
- **Abajo, la vista y lo que se dibuja sobre ella**, siempre los mismos tramos en el mismo orden: **Vista** (escala o proyección, **Grosores de impresión** —apagado por defecto: líneas finas para trabajar; encendido, como se imprime; no en 3D, donde el contorno es del aspecto—, estilo, atenuar, caja de sección) · **Anotar** (Línea, Relleno, Texto, Cota con sus variantes) · **Ayudas** (Guía y forzados) · coordenadas. Lo que no vale en una vista se ve apagado, no desaparece.

### 12.4 Presentación

Lo que se enseña al cliente, al ayuntamiento o en un concurso, aparte de los planos técnicos (que siguen en Documentación). Al entrar, la vista toma su estilo de presentación. Las cámaras y láminas de presentación tienen su grupo en el navegador.

### 12.5 La vista 3D

- **Cubo de vistas** discreto: semitransparente, sin recuadro, arriba a la derecha. Sus caras llevan a planta y alzados (perspectivados, con las verticales rectas), sus esquinas a las cuatro vistas de esquina; al pasar el ratón se ve entero y aparecen girar 90° y la vista de inicio.
- **Aspecto** arriba a la izquierda: Sombreado (sin texturas), Texturas y Realista (sombras finas y penumbra de los rincones). Aristas y Líneas ocultas, cuando tengan su propio sombreado.
- **Una cámara por modo** (sin selector):
  - **Técnica, cámara libre**: el derecho mantenido gira alrededor de lo que hay bajo el cursor (marcado con un punto naranja); con el derecho apretado, W A S D vuelan, Q y E bajan y suben, Mayús acelera; al soltar, las letras vuelven a ser órdenes.
  - **Maqueta, cámara de maqueta**: mira siempre a un punto del suelo, con la inclinación entre 30° y 70°; W A S D la mueven por el suelo sin pulsar nada.
  - Lo que hace cada tecla con la cámara del modo se lee abajo a la izquierda del dibujo.
  - Recorrer la casa a la altura de los ojos es la ficha Recorrido de Presentación.
- **Navegación** (en la barra inferior del 3D y en Vista → Navegación 3D): velocidad de paseo, de giro y paso de la rueda, del 1 al 10; girar alrededor de lo que hay bajo el cursor; no alejarse más allá del modelo (tres veces su tamaño); invertir el ratón. Las verticales rectas no son una opción: los alzados del cubo se miran a nivel.

### 12.6 Propiedades en tarjetas

Dos niveles: arriba, tarjetas con lo que más se toca (el tipo con su dibujo grande y los tipos parecidos para cambiar de un clic; las medidas como cifras grandes que se escriben; la función en botones; los acabados con su muestra; la altura dibujada de planta a planta); debajo, «Todas las propiedades» con la lista técnica completa de siempre. Con una herramienta activa, la galería de tipos; sin nada, las tarjetas de estilo y escala de la vista.

### 12.7 Maqueta

Las mismas barras en el mismo sitio. La columna de fichas pasa a ser el catálogo y el carril, las estancias. Las diferencias con Técnica quedan en: la cámara, las letras sueltas (cámara en Maqueta, órdenes en Técnica), el color (siempre a todo color), la selección (muebles primero) y el catálogo.

### 12.8 Bloques

| Bloque | Contenido |
|---|---|
| V1 ✅ | Colores por disciplina, fichas agrupadas, Presentación, herramientas recolocadas |
| V2 ✅ | Barras: Entregar arriba; Vista · Anotar · Ayudas abajo |
| V3 ✅ | Vista 3D: cubo, aspecto, cámara libre con vuelo, navegación |
| V4 ✅ (primera vuelta) | Propiedades: banda de color y medidas en tarjetas de muros, huecos y forjados |
| V5 | Maqueta con las mismas barras y su cámara |

## 13. La cinta y la barra de opciones (6 de noviembre de 2026)

Pedida por Antonio con cinco capturas de la pestaña *Modificar* de Revit (en reposo, colocando líneas de detalle, muros, suelos y pilares): «es un factor clave en la forma de modelar y dibujar; la nuestra es inferior y, cuando hay demasiados elementos, los últimos son difíciles de ver». La barra contextual de una fila (§2.6) pasa a ser dos piezas, en el mismo sitio, bajo las pestañas:

### 13.1 La cinta

Grupos con su nombre debajo. Botones **grandes** (las tres filas) para lo principal y **pequeños apilados de tres en tres** para el resto: caben muchas órdenes en poco ancho.

Siempre los mismos grupos, en el mismo orden:

| Grupo | Botones |
|---|---|
| Seleccionar | Los de su tipo · Todo (la flecha está arriba del todo en el carril de la izquierda) |
| Portapapeles | **Pegar** · Cortar · Copiar · Igualar propiedades |
| Geometría | Pintar · Dividir cara · Redondear (volúmenes) |
| Modificar | Mover, Copiar, Girar · Reflejar, Escala, Matriz · Alinear caras, Desfase, Alinear · Esquina, Hasta límite, Varios al límite · Recortar, Alargar, Partir · Unir, Borrar |
| Vista | Ocultar · Aislar · Mostrar todo |
| Medir | Cinta · Cota |
| Crear | **Crear similar** (CS) |

Detrás, en el color de la disciplina, los grupos de **lo que hay en la mano**:

| En la mano | Grupos que se añaden |
|---|---|
| Muro, forjado, cubierta, local, viga, barandilla… | **Dibujar**: galería de formas (recta, rectángulo, polígono, arco, círculo, varita, spline, elegir línea); lo que no vale, apagado |
| Puerta, ventana | **Colocación**: libre, por el centro, desde un extremo, repartidas |
| Cubierta | Dibujar y **Buhardillas** |
| Cota, cimentación, formas de Concepto, pincel | Sus maneras, con nombre |
| Modo boceto | **Modo** (✓ Aceptar, ✗ Cancelar) y **Dibujar**; el grupo Modificar pasa a tener las órdenes del boceto (esquina, recortar, alargar, partir, desfase, mover, copiar, borrar) |
| Algo elegido | Su grupo: Editar contorno o **Editar perfil** (muros) y sus caras para dividir; agrupar y desagrupar el dibujo 2D |
| Una orden de dibujo 2D | Sus variantes (línea, polilínea…) |

**Colores** (pedido de Antonio, 6/11/2026: «todo en negro no se distingue»): cada dibujo lleva el color de su familia, como los iconos de Revit, y el nombre va en el negro del texto.

| Familia | Color |
|---|---|
| Seleccionar | azul pizarra |
| Portapapeles | ámbar |
| Geometría (pintar, dividir, redondear) | terracota |
| Mover, copiar, girar, reflejar, escala, matriz | azul |
| Alinear caras, desfase, alinear | violeta |
| Esquina, límites, recortar, alargar, partir, unir | verde agua |
| Borrar | rojo |
| Vista | gris azulado |
| Medir | verde agua |
| Crear similar | verde |
| Lo que hay en la mano | el color de la disciplina |
| **Aceptar** / **Cancelar** del boceto | **verde** / **rojo**, sobre un fondo de su color |

En el tema oscuro, los mismos colores más claros. Lo que no vale ahora se ve gris, así que el color también dice qué se puede usar.

Lo que no vale ahora se ve apagado y su nota dice por qué. La orden que corre se ve encendida; pulsarla otra vez la termina.

**Cuando no cabe**, como en Revit, paso a paso hasta que quepa:
1. Los grupos fijos pierden los nombres de sus botones pequeños (queda el dibujo; el nombre, en su nota).
2. Se pliegan en un botón con su nombre (▾), que los abre debajo: Crear, Medir, Vista, Portapapeles y Geometría.
3. Los grupos de lo que hay en la mano pierden los nombres.
4. Se pliegan Seleccionar y Modificar.
5. Por último, los de lo que hay en la mano.

Nada se corta por la derecha.

**Recoger la cinta** (la flecha al final de la barra de opciones; se recuerda en `preferencias.json`): solo los nombres de los grupos, en una fila, que se abren al pulsarlos; si ni así caben, los menos usados se quedan con su dibujo. Para pantallas pequeñas.

### 13.2 La barra de opciones

Debajo de la cinta, una fila: **«Modificar | Colocar Muro»** y los ajustes de lo que hay en la mano, como en Revit.

- Muro: **Altura** (desconectada, planta superior o hasta una planta) con su valor o su desfase, **línea de ubicación** (cara exterior, eje, cara interior), **Cadena**, longitud y ángulo.
- Pilar: Altura.
- Vial: línea de ubicación.
- Puerta y ventana: la distancia o cuántas, según la colocación.
- Pincel: tamaño y fuerza.
- Una orden en marcha: lo que pide, sus opciones y Terminar.
- Modo boceto: qué se edita y qué tienen que hacer las líneas (antes era un cartel flotante sobre la planta).
- Siempre, al final y en gris: qué hace ahora un clic.
