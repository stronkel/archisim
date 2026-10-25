# ArchiSim — Hoja de ruta después del MVP

> 17 de octubre de 2026 · versión 0.8 · MVP (fase 1) terminado. Actualizada el 24/10/2026 (§9, hoja de ruta común entre conversaciones, v0.56). Actualizada el 23/10/2026 (§8, Maqueta y cubiertas, v0.48). Actualizada el 18/10/2026 (D2 decidida; §1 cerrada en v0.22) y el 19/10/2026 (fase 2 y revisión R1–R11, fase 3 en v0.25, fase 4: 4.1 a 4.4 en v0.26; todo junto en v0.27).
> Lista de todo lo que hay que preparar a continuación, ordenada por fases.
> Antonio decide el orden dentro de cada fase; lo marcado con ★ es lo que
> propongo hacer primero. Tamaño orientativo: **S** (un bloque corto),
> **M** (un bloque), **L** (varios bloques).

## 0. Decisiones que tiene que tomar Antonio

Sin código: son decisiones de producto que condicionan lo demás.

| # | Decisión | Por qué importa ahora |
|---|---|---|
| D1 | **Orden de los próximos bloques** (ver §1) | Edición con ratón, cotas o fase 2 directamente |
| D2 ✅ | **Lenguaje de familias** — decidido el 18/10/2026: **receta de texto** como motor y **configurador tipo ArchiCAD** para crear tipos; una familia por funcionamiento con piezas comunes; tipos con nombre y marca reutilizables en ficheros de familia (como Revit); copia en el proyecto; detalle por escala; empezando por puertas y ventanas (ver `docs/FAMILIAS.md`) | Es la pieza central de la fase 2: puertas, ventanas, mobiliario y sanitarios dependen de ella |
| D3 ✅ | **IFC de salida por defecto**: **IFC 4.3** (decidido por Antonio el 19/10/2026, por el entorno: georreferencia, vegetación, vehículos). Hecho en v0.29; IFC4 sigue como «Exportar IFC4 (compatibilidad)» | Afecta a la validación y a lo que prueban los ingenieros |
| D4 ◐ | **Licencia** — decidida el 18/10/2026: **GPL-3.0 o posterior**; repositorio público desde ese día. Falta el **nombre definitivo** (ArchiSim es provisional) | El nombre conviene fijarlo antes de anunciarlo a la comunidad |
| D5 | **Normas de dibujo de referencia**: tramas, grosores y rotulación (UNE-EN ISO 128, UNE 1-027 para plegado, criterio propio de AD5) | Fija cómo se ven plantas, secciones y planos |
| D6 | **Plantilla al importar**: añadir siempre la plantilla de ArchiSim a un proyecto importado o solo cuando faltan tipos (hoy: solo cuando faltan) | Comodidad frente a limpieza del proyecto |
| D8–D12 | **Disciplinas y mediciones** (propuesta del 18/10/2026 en `docs/DISCIPLINAS.md`): un solo modelo con disciplina por elemento, IFC 4.3 para urbanismo, banco de precios de referencia (BCCA), criterio de deducción de huecos y orden de los bloques | Estructura, urbanismo y mediciones con bancos de precios dependen de ellas |
| D17–D22 ✅ | **Interfaz por disciplinas** — decididas el 20/10/2026 (`docs/INTERFAZ.md` §11): bloques I1 → I2 → I3 → I5 → I4 → I6; las teclas cambian de disciplina; atenuación activada; trabajo en equipo primero en una carpeta de Google Drive y después en servidor propio; reservas por zona y por elemento; cursor de los compañeros opcional y apagado | Rehace la caja de herramientas, la barra de modificar y los módulos del masterplan |
| D23 ✅ | **Librería 3D** — decidida el 21/10/2026: objetos con modelo 3D detallado (malla con texturas y niveles de detalle) además de su símbolo de planta paramétrico; modelos de uso libre (CC0 o que permitan redistribuir, nunca `.rfa` ni `.gsm`); **una biblioteca básica con lo imprescindible integrada** y el resto como **descarga aparte**, con **varios modelos de cada objeto** para comparar marcas y estética | Fija de dónde salen los objetos y cuánto pesa el programa |
| D24 ✅ | **Estilos de presentación** — 21/10/2026: Antonio manda láminas de AD5 (planta de vivienda 1:50 y planta general 1:200) y de otros estudios como referencia (ver §6.3). Se hacen estilos de vista de presentación aplicables con un clic | Planos de concurso y de cliente sin pasar por Illustrator |
| D7 | **Ficheros reales de prueba**: IFC exportados por Antonio desde sus propios proyectos de ArchiCAD y Revit, para probar la importación (se usan en local, no se suben al repositorio) | La importación solo se ha probado con ficheros sintéticos y de IfcOpenShell/SketchUp |

## 1. Cierre de la fase 1 (deudas del MVP)

Cosas pequeñas que faltan para trabajar con comodidad con lo que ya existe.

| # | Tarea | Tamaño |
|---|---|---|
| 1.1 ✅ v0.9 | **Mover, copiar y estirar con el ratón** (muros, forjados, huecos), con los mismos forzados que al dibujar; arrastrar un extremo de muro arrastra la esquina unida | M |
| 1.2 ✅ v0.9 | **Forjado por varita mágica**: clic dentro de un recinto de muros (por la cara interior, exterior o eje) | S |
| 1.3 ✅ v0.9 | **Edición de vértices** de forjados: mover, añadir y quitar esquinas | S |
| 1.4 ✅ v0.10 | **Cotas básicas**: lineales y encadenadas, cota automática de huecos en un muro, cotas de nivel | M |
| 1.5 ✅ v0.10 | **Gestión de plantas**: añadir, renombrar, cambiar cota y borrar plantas desde la aplicación | S |
| 1.6 ✅ v0.12 | **Editor de tipos y materiales**: crear y modificar multicapas (capas, espesores, función), materiales (color, trama) y tipos de puerta y ventana | M |
| 1.7 ✅ v0.20 | **Importación IFC, segunda vuelta**: muros curvos (v0.19), muros de varios tramos y circulares cerrados, forjados con huecos (también dibujarlos), huecos sin carpintería (familia Hueco). Pasan a la fase 2, porque necesitan elementos que aún no existen: forjados inclinados (2.7), pilares y vigas (2.5) y locales `IfcSpace` (2.9) | M |
| 1.8 ✅ v0.21 | **Instalación**: aplicación de Mac universal en `.dmg` (v0.13) e **instalador de Windows** (v0.21) con icono, menú Inicio, ejemplos, asociación de `.archisim` y «Abrir con» de `.ifc` | S |
| 1.9 ✅ v0.22 | **Rendimiento** con modelos grandes: medido con `archisim medir` (hasta 6300 muros, 9000 huecos y un IFC de 56 MB) y optimizado en uniones, planta en pantalla, forzados y selección, 3D, exportación e importación | M |

## 2. Fase 2 — Modelado completo

**Modos de construcción y muros curvos** ✅ v0.19 (pedidos por Antonio el 18/10/2026, junto a 1.7): métodos de la herramienta Muro (recto, cadena, rectángulo por dos esquinas, arco por inicio, fin y punto de paso, círculo por centro y punto) y de la herramienta Forjado (polígono, rectángulo por dos esquinas, varita mágica); muros curvos de punta a punta (planta con capas, 3D, ingletes con muros rectos y curvos, IFC exacto con arcos, `.archisim` versión 5), curvar con el ratón y radio en Propiedades. Puertas y ventanas en muros curvos ✅ v0.48. Falta: uniones en T por capas contra muros curvos (de momento el muro que acomete se detiene en la cara), forjados con bordes curvos o circulares, rectángulo girado, arco tangente.

**Ayudas al dibujo** ✅ v0.18 (pedidas por Antonio el 18/10/2026, antes del bloque C): cotas temporales azules editables, forzado a puntos medios de caras y aristas, muros ortogonales por defecto con línea de rastreo, Espacio y flechas para girar puertas y ventanas, cambiar de planta muros y forjados, muros de otras plantas cortados por el plano de corte y calco de la planta inferior. Falta: cotas temporales de forjados y cotas a ejes o a núcleo (como opción de Revit), seleccionar en una planta los muros de otra que se ven cortados, copiar a otra planta.

**Restricción superior de muros** ✅ v0.17 (pedida por Antonio el 18/10/2026, antes del bloque C de familias): muros hasta una planta con desfase, que siguen los cambios de cota. Falta lo mismo para pilares cuando existan (2.5) y el recorte con forjados y cubiertas (2.3, 2.4).

**Revisión de Antonio del 19/10/2026** (sobre v0.25, PR #19). Por orden de trabajo:

| # | Tarea | Tamaño | Estado |
|---|---|---|---|
| R1 ✅ | **Líneas de ayuda de intersección**: al dibujar muros, forjados, cubiertas, locales, cotas…, guías perpendiculares y de alineación desde los puntos ya dibujados (para cerrar una vivienda con el último vértice en línea con el primero), como la de ortogonalidad | M | — |
| R2 ✅ | **Cotas normales re-anclables**: arrastrar un extremo de una cota a otro punto, sin borrar y redibujar; anclajes más visibles | S | — |
| R3 ✅ | **Usos de local propios**: lista de usos editable por proyecto (no solo vivienda) | S | — |
| R4 ✅ | **Terreno editable en planta**: clic en un punto para escribir su cota (adiós a la lista); **líneas de nivel** dibujadas con su cota, que generan puntos; cambiar la cota de una línea entera; más adelante, crearlas con la varita sobre un dibujo CAD | M | — |
| R5 ✅ | **Seleccionar en sección y alzado** (y ver/editar sus propiedades) | M | — |
| R6 ✅ | **Encuentro de muros con forjados y cubiertas**: el muro se recorta con el forjado o la cubierta que lo atraviesa (prioridades) | L | 2.3 |
| R7 ✅ | **Cuadro de prioridades de intersección** en el proyecto, visual con barras como ArchiCAD, con criterios predefinidos | M | 2.3 |
| R8 ✅ | **Galería de objetos** con vistas previas y carpetas por categoría como ArchiCAD: Mobiliario (cocina, estanterías, camas, sofás…), Entorno (vegetación…), Gente, Vehículos; familias nuevas para cada una | L | 2.2 |
| R9 ✅ | **Importar topografía DXF** (puntos y curvas 3D) desde el programa; DWG más adelante (formato cerrado: hará falta un convertidor) | M | 2.12 |
| R10 ✅ | **Cubierta única en L con limahoyas** (contornos con esquinas entrantes) y **mover a mano limatesas y limahoyas** en geometrías especiales | L | 2.4 |
| R11 ✅ | **Editor de muros cortina** como el de ArchiCAD pero más claro: esquema de columnas y filas de paneles, marcos, tipos de panel, uniones y accesorios, con vista previa siempre y **tipos de muro cortina guardados** | L | 2.8 |

| # | Tarea | Tamaño | Depende de |
|---|---|---|---|
| 2.1 ★ ✅ v0.33 | **Sistema de familias paramétricas** — los cinco bloques hechos (E en v0.30: biblioteca de tipos, actualizar familias, purgar; v0.33: símbolo 1:20, alzado con líneas de apertura, puntos de control de puertas y ventanas, tipos de muro y forjado en la biblioteca, seleccionar los ejemplares de todo el proyecto); v0.34: parámetros del proyecto y compartidos; siguen editor gráfico de recetas, familias de anotación e importación de objetos (20/10) — (diseño en `docs/FAMILIAS.md`), en cinco bloques: **A** núcleo (recetas, parámetros por secciones, fórmulas, piezas comunes, 3D; puertas y ventanas actuales convertidas en cinco familias; formato v3), **B** símbolos de planta por escala y alzado, **C** IFC (tipos con marca, Psets, `ArchiSim_Family`), **D** configurador con vista previa en directo y puntos de control, **E** navegador de familias y tipos con nº de ejemplares, biblioteca de tipos (`.asfam`) y herramienta Objeto | L | D2 ✅ |
| 2.2 ◐ v0.32 | **Librería inicial propia** (hechas la puerta plegable, la corredera empotrada, la persiana con capialzado y trece familias de mobiliario, sanitarios y cocina con su herramienta Objeto; v0.32: premarco, vierteaguas, tarjas, puerta de dos hojas desiguales con hoja activa, corredera de hasta 4 hojas y nueve tipos habituales propuestos para AD5, con `archisim biblioteca` y `archisim muestrario`; v0.33: puerta de garaje, pivotante, guillotina, fijos laterales, ventana en esquina y hojas pivotante y basculante; falta que Antonio dé el catálogo real de AD5): puertas y ventanas paramétricas (con premarco, persiana y capialzado, vierteaguas, tarjas, puerta de dos hojas desiguales y hoja activa, corredera empotrada en armazón, corredera de hasta 4 hojas, **puerta plegable**), mobiliario, sanitarios, cocina; plantilla con un catálogo de tipos habituales de AD5 | L | 2.1 |
| 2.3 ◐ v0.11 | **Uniones en T y prioridades de intersección por capa** (la estructura atraviesa, los acabados se detienen), como ArchiCAD — hechas, también las prioridades editables por material (v0.25, de 0 a 999); falta el recorte de muros con forjados | M | — |
| 2.4 ✅ v0.48 | **Cubiertas inclinadas** por faldones (pendiente, aleros, limatesas y limahoyas) y recorte de muros bajo cubierta — hechas las cubiertas sobre contorno convexo (a cuatro, dos o un agua y a medida por lado, alero, planta de cubiertas, 3D, secciones, IFC `IfcRoof` y su lectura de vuelta); el recorte de muros bajo cubierta (piñones, IFC con semiespacios); contornos con esquinas entrantes (en L: limahoyas, R10) y **buhardillas** a un agua y a dos aguas (v0.48: planta de cubiertas, 3D, IFC como `IfcRoof` dentro de la cubierta e importación) | L | 2.3 |
| 2.5 ◐ v0.25 | **Pilares y vigas** (hormigón, acero, madera) con perfiles; `IfcColumn`, `IfcBeam`; también importarlos — hechos (sección rectangular, circular, I/H, U, T, L y tubos, restricción superior de pilares, planta, 3D, IFC e importación); falta el recorte con muros y forjados | M | — |
| 2.6 ◐ v0.25 | **Escaleras** por tramos y mesetas con comprobación de huella, tabica y anchura (CTE DB-SUA); **barandillas**; **rampas** — hechas las escaleras de losa de hormigón por línea de huella (rectas, en L, en U, con mesetas; tabicas iguales hasta la planta de llegada; comprobación CTE uso general y restringido; planta con línea de corte, 3D, secciones, IFC `IfcStair` y su lectura de vuelta); barandillas de escalera con pasamanos y montantes; las rampas se hacen ya con forjados inclinados (2.7). barandillas sueltas de barrotes o de vidrio para terrazas y cubiertas (cumplen los 10 cm del CTE). Falta: barandillas de chapa, peldañeado compensado y escaleras de caracol, zancas metálicas y de madera | L | 2.1 |
| 2.7 ✅ v0.25 | **Huecos en forjados** (hechos en v0.20) y **forjados inclinados** (también importarlos) — hechos (flecha de pendiente con % y dirección, arrastrable; 3D, secciones, IFC e importación). Falta: forjados con varias pendientes (eso son las cubiertas, 2.4) | S | — |
| 2.8 ◐ v0.57 | **Muros curvos** (hechos, con puertas y ventanas desde v0.57: rectas, en escuadra con el muro en su centro), **muros cortina** (hechos: rectos con retícula regular de montantes, travesaños y vidrios) y **muros de perfil complejo** (pendiente) | L | 2.1 |
| 2.9 ✅ v0.27 | **Locales** (`IfcSpace`): nombre, número, superficie útil y construida, uso, altura, acabados; siguen a los muros; también importarlos — hechos (herramienta Local por recinto, polígono o rectángulo, y orden LOCAL; sello en planta; etiquetas; cuadro de superficies; IFC e importación; un solo local desde v0.27, al juntar las fases 2 y 3). Falta: editar los vértices del contorno, rellenos por uso | M | 1.2 |
| 2.10 ✅ v0.25 | **Secciones y alzados** con líneas ocultas, profundidad de vista y poché por capa — hechos (herramienta Sección, cuatro alzados automáticos, marcas en planta, vista en pestaña con corte por material y capas, vista sin líneas ocultas, plantas acotadas, SVG desde la línea de comandos). Falta: cotas en sección (3.2), línea de terreno, secciones quebradas | L | 2.3 |
| 2.11 ✅ v0.25 | **Fases**: existente, a derribar, nuevo (reformas), con su grafismo — hechas (fase por elemento, «dibujar como», vistas Estado actual / Demoliciones / Estado reformado en amarillo y rojo, 3D, IFC `Status`). Falta: tramas o colores configurables por estudio (D5) y fase en locales | M | — |
| 2.12 ◐ v0.25 | **Terreno** sencillo (curvas de nivel o puntos), emplazamiento y orientación real del norte — hechos el terreno por puntos de cota (superficie triangulada, curvas de nivel acotadas en planta, sección del terreno, IFC `IfcGeographicElement`) y el norte real (flecha en las plantas exportadas, brújula del 3D, IFC `TrueNorth`). Falta: importar curvas de nivel de un DXF o puntos de un CSV, explanaciones y vaciados, coordenadas geográficas del emplazamiento | M | — |

## 3. Fase 3 — Documentación

Hecha en v0.25 (bloques 3A a 3I de `docs/DOCUMENTACION.md`), a falta de que Antonio la revise y de lo que espera a las secciones (2.10).

| # | Tarea | Tamaño | Depende de |
|---|---|---|---|
| 3.1 ◐ v0.25 | **Plantillas de vista y estilos gráficos**: normas gráficas (plumillas finas y gruesas por escala), categorías y capas ocultas y nivel de detalle por vista; capas de dibujo — hecho; falta guardar estilos con nombre (plantillas de vista) y la decisión D5 sobre tramas y grosores | M | D5 |
| 3.2 ◐ v0.28 | **Acotación completa**: cotas asociativas (hechas), angulares, de radio, de diámetro y de longitud de arco (hechas), superficie en el rótulo de los locales (hecha), alturas en sección automáticas (v0.28: entre plantas y altura libre de cada local cortado); falta acotar a mano sobre la sección | M | 1.4, 2.10 |
| 3.3 ✅ v0.25 | **Etiquetas**: de puertas y ventanas (marca, medida), de locales, de muros (tipo); siguen a su elemento; «etiquetar todas» | S | 2.9 |
| 3.4 ✅ v0.25 | **Textos, líneas, tramas y detalles 2D** sobre las vistas, con línea de órdenes, órdenes de edición y forzados al estilo AutoCAD | M | — |
| 3.5 ✅ v0.28 | **Planos y maquetación**: hojas A4–A0 con cajetín propio, varias vistas por hoja a distintas escalas con recorte y título, numeración, dibujo y tablas en el plano — hecho, también con secciones y alzados (v0.28) | L | 3.1 |
| 3.6 ✅ v0.25 | **Salida PDF vectorial** (plantas y planos, todos en un PDF); **DXF** de plantas y planos con capas; **importar DXF** | M | 3.5 |
| 3.7 ✅ v0.25 | **Tablas de planificación**: carpinterías, muros y forjados por tipo, cuadro de superficies; CSV y en planos (v0.28: también pilares y vigas, cubiertas, escaleras con su comprobación CTE y muros cortina; falta el desglose por capa de los multicapas) | M | 2.9 |
| 3.8 ✅ v0.28 | **Mediciones para presupuesto** en formato **FIEBDC-3 (BC3)**, sin precios (los bancos de precios, en `docs/DISCIPLINAS.md`); desde v0.28, capítulo de demoliciones en reformas y estructura, cubiertas, escaleras, barandillas y muros cortina | M | 3.7 |

## 4. Fase 4 — Colaboración e ingenierías

| # | Tarea | Tamaño | Depende de |
|---|---|---|---|
| 4.1 ✅ v0.26 | **Vínculo de modelos IFC** de estructuras e instalaciones como referencia (sin importarlos), con control de versiones por `GlobalId` — en planta y 3D, recarga automática con los cambios coloreados (diseño en `docs/COLABORACION.md`) | M | — |
| 4.2 ◐ v0.57 | **Detección de interferencias** entre el modelo y los vinculados — hecha, con tolerancia, huecos y vista aislada; **holguras y reglas por parejas de disciplinas** (v0.57); faltan interferencias entre vínculos | M | 4.1 |
| 4.3 ✅ v0.26 | **BCF**: incidencias con vista guardada, intercambiables con Revit, ArchiCAD, Solibri o BIMcollab — 2.1 y 3.0; falta BCF API (servidores) | M | — |
| 4.4 ◐ v0.26 | **IDS**: comprobar el modelo contra los requisitos de información del cliente (hecho, 334 casos de buildingSMART); **bSDD** para clasificaciones (falta) | M | — |
| 4.5 | **Complemento puente para Revit** (C#, API oficial de Revit): llevar tipos y elementos nativos en ambos sentidos | L | — |
| 4.6 ✅ | **Trabajo en equipo**: carpeta compartida (Google Drive, NAS), publicar y recibir, reservas de elementos o zonas, peticiones, historial por elemento. *Hecho en v0.38* (`docs/EQUIPO.md`); falta un servidor propio para la presencia al momento | L | — |
| 4.7 ◐ v0.31 | **Comprobaciones normativas** del CTE: evacuación (DB-SI), accesibilidad (DB-SUA), superficies útiles — hechas ocupación, salidas, recorridos, anchuras y sentido de apertura de puertas y pasillos (SI 3), itinerarios accesibles y vivienda accesible con círculos de giro que descuentan el mobiliario (SUA 9), escaleras y rampas (SUA 1) y superficies útil y construida (`docs/CTE.md`). Falta: escaleras protegidas y sectores (SI 1), anchura de escaleras por ocupación, recorridos alternativos, barandillas por altura de caída, habitabilidad autonómica | L | 2.9 |

Hecho en v0.26 (rama `bloque/colaboracion`): 4.1, 4.3 y la mayor parte de 4.2 y 4.4, con las órdenes `comprobar-ids`, `comparar`, `interferencias`, `bcf` y `demo-coordinacion`. Decisiones nuevas D13 a D16 en `docs/COLABORACION.md`. 4.7 espera a los locales (2.9) en `main`.

## 5. Transversal (cuando toque)

- **Formato `.archisim`**: migraciones probadas con proyectos de cada versión; guardado más rápido para proyectos grandes.
- **Interfaz** ✅ v0.16: rediseñada como ArchiCAD con propiedades de Revit (caja de herramientas por secciones, pestañas de vistas con su escala, navegador de vistas, planos y tipos, barra inferior con escala y forzados, tema claro y oscuro). Pendiente: recordar la disposición y el tema entre sesiones, atajos configurables. **Siguiente rediseño, acordado el 20/10/2026: interfaz por disciplinas** (`docs/INTERFAZ.md`, bloques I1–I6: barra de disciplinas Concepto · Arquitectura · Estructura · Instalaciones · Urbanismo · Documentación · Análisis · Coordinación, fichas de formato fijo con iconos propios, barra contextual de métodos y Modificar, atenuación por disciplina, disciplina Concepto al estilo SketchUp, módulos para masterplans, buscador de órdenes). **I1 hecho** (20/10/2026): barra de disciplinas, fichas de formato fijo con los iconos nuevos, Propiedades debajo, navegador Modelo/Proyecto, buscador de órdenes (Ctrl+Espacio) y teclas que cambian de disciplina. **I2 hecho** (20/10/2026): barra contextual con métodos y Modificar, barra inferior por segmentos, línea de órdenes oculta, tablas y mediciones en pestañas, y arrastrar plantas, secciones, tablas y la vista 3D a un plano a su escala. **I3 hecho** (20/10/2026): disciplina por elemento, atenuación por disciplina en planta, sección, 3D y planos, ojo y candado en el navegador. **I6 hecho** (22/10/2026, v0.41): tamaño de letra Normal, Grande o Muy grande y seis misiones de iniciación con el control de cada paso rodeado en azul (`docs/INICIACION.md`); el tema, las teclas y el tamaño se recuerdan entre sesiones.
- **Instalaciones** (primera vuelta, 21/10/2026, rama `bloque/instalaciones`): tuberías y conductos por su recorrido con sistema, tamaño y altura; luminarias, enchufes, interruptores y equipos; planta con colores y rótulos, 3D, mediciones e IFC en sistemas de distribución. Segunda vuelta (v0.48): codos y tes que salen del trazado (3D, planta, mediciones e IFC `IfcPipeFitting`/`IfcDuctFitting`), pendiente del saneamiento, radiadores, rejillas y difusores, y comprobación de diámetros y pendientes (CTE HS 4 tabla 4.2 y HS 5 tabla 4.1 y apartado 3.3). Siguiente: dibujo en sección y alzado, bajantes verticales, diámetros de colectores y bajantes por unidades de desagüe (HS 5, tablas 4.3 a 4.5) y caudales (HS 4).
- **Manual de usuario** en español y lista de atajos; **traducción al inglés** de la interfaz antes de abrir el código.
- **Informes de fallo** con el proyecto adjunto (sin telemetría).
- **Validación continua**: además de IfcOpenShell, abrir los IFC de cada versión en ArchiCAD y Revit (Antonio) y guardar capturas de referencia.

## 6. Revisión de Antonio del 21/10/2026 y bloques nuevos

Los siete bloques del 20/10 (I3, ejes y cimentación, módulos, concepto, urbanismo, métodos e instalaciones) se unieron a `main` el 21/10/2026. Antonio los probó: funcionan, con estos cambios.

### 6.1 Correcciones (bloque R) ✅ hechas el 21/10/2026 en `bloque/revision-r`

| # | Qué | Cómo |
|---|---|---|
| R12 | En Concepto, al elegir **Línea** salta a polígono, y se confunde con la línea de dibujo 2D de la barra inferior | La línea de Concepto es una **arista del modelo**: puede quedar abierta (una arista suelta, como en SketchUp) y solo forma cara al cerrarse. Nombre y pastilla que lo digan («Arista»), distinto de «Línea de dibujo» |
| R13 | **Rectángulo** y **Círculo** de Concepto repiten lo que ya hace Línea con los métodos de la barra contextual, y ocupan fichas | Se quitan como fichas: son métodos de Arista. Los huecos quedan para las herramientas de aristas, intersecciones y modelado de §6.4 |
| R14 | **No se puede trabajar en la ventana 3D**: dibujar muros y forjados, colocar objetos, dibujar y empujar volúmenes de Concepto, pintar caras | Herramientas en 3D: puntos sobre el plano de la planta activa o sobre la cara que hay bajo el cursor, con forzados 3D (vértices, aristas, puntos medios, caras); **Empujar** y **Pintar** directamente sobre las caras en 3D; colocar objetos y aparatos sobre suelos y caras |
| R15 | Las herramientas de dibujo de la barra inferior (Línea, Relleno, Texto, Cota) **no tienen la barra contextual**: ni Modificar (copiar, mover, desfase…) ni los métodos (recta, polilínea, polígono, spline…) | Las órdenes de dibujo 2D usan la misma barra contextual que las de modelo: sus métodos y la barra Modificar |
| R16 | **Spline** resulta raro: en Concepto sale un círculo cerrado que se deforma y hay que pasar por «Camino» para una curva abierta; en el dibujo 2D no hay Spline | Spline abierto por defecto en todas las herramientas; se cierra al pinchar en el primer punto (o con un botón «Cerrar»); sin pasar por Camino. Spline también para el dibujo 2D (polilínea curva) |
| R17 | El Spline de Línea (dibujo 2D) no curva igual que el de Arista | El mismo trazado (Catmull-Rom) en las dos: curvas naturales que pasan por los puntos |
| R18 | Al pintar un muro apenas cambia, y no se puede volver al material original | La pintura se ve siempre con su color (también con la disciplina atenuada); **Quitar pintura** en Propiedades de Pintar |
| R19 | Pintar una cara de un volumen pinta el volumen entero | Solo la cara pinchada (tapa, base o un lado); el volumen entero, cambiando su material en Propiedades |
| R20 | En 3D las aristas solo se apoyan en la planta: no se puede levantar una arista en una fachada para pintarla por zonas | Aristas en 3D sobre las caras (con Mayús, en vertical); Pintar pinta la zona de la fachada que cierran esas aristas |
| R21 | Al dibujar en 3D no se ve el muro, la puerta o la ventana antes de colocarlos | Vista previa en 3D de muros, puertas, ventanas, pilares, vigas, forjados y objetos, tal como quedarán |

### 6.2 Librería 3D (D23)

- **L1**: importar mallas **glTF, OBJ y Collada (.dae)** (lo que exporta SketchUp, también el gratuito) con materiales y texturas: como objeto de librería o como volumen de Concepto. El `.skp` no se lee (formato cerrado). *Hecho en v0.36 como objeto de librería* (las texturas, como colores en los vértices; como volumen de Concepto, pendiente).
- **L2**: los objetos de librería llevan, además de su símbolo de planta de la receta, **un modelo 3D por niveles de detalle** (el lejano, ligero). IFC: `IfcTriangulatedFaceSet` con su color o textura. *Hecho en v0.36* (tres versiones; todas las vistas toman la más ligera que cabe en 400.000 caras; IFC con colores por cara en el tipo).
- **L3**: **biblioteca básica integrada** (sanitarios, cocina, mobiliario de salón, comedor y dormitorio, vegetación, personas, coches) y **biblioteca ampliada como descarga** (varios modelos por objeto: marcas y estéticas). Fuentes: CC0 (Poly Haven, Kenney…) y fabricantes que permitan redistribuir. *Primera vuelta (v0.35, sin modelos 3D nuevos)*: biblioteca ampliada de recetas (38 árboles, 14 arbustos y setos, 31 de mobiliario urbano, personas y vehículos) que se instala en la carpeta de la biblioteca y aparece en las galerías; los modelos 3D de malla quedan para L1–L2. *Segunda vuelta (v0.40) ✅*: el 3D de muebles, sanitarios, cocina, personas y coches lo genera ArchiSim desde las medidas de cada tipo, con varios estilos por objeto y material por pieza (`docs/BIBLIOTECA_3D.md`).
- **L4 Vegetación generada** (pedida por Antonio el 21/10/2026, `docs/VEGETACION.md`): en lugar de descargar árboles, ArchiSim los hace crecer por reglas, como EZ-Tree: especies (porte, copa, tronco, ramas, follaje, flor y fruto) y una semilla por planta, 3D en tres niveles de detalle y planta en trece estilos de paisajismo. *Hecho en v0.37*: crate `archisim-flora`, 64 especies integradas (mediterráneas primero), **Vivero** con deslizadores y 3D al momento, **Plantación** a lo largo de líneas o splines o dentro de contornos, tirador de ancho en planta. Falta: tiradores en 3D, hojas con textura (M3), setos como masa continua, estaciones.

### 6.3 Presentación (D24)

Referencias de Antonio:

- **AD5**: plantas a 1:50 y 1:200 de líneas finas negras, césped en franjas verdes, vegetación en círculos de colores por especie, lámina de agua en cian rayada, tarima en lamas, mobiliario y coches en línea, cuadro de superficies y cajetín propio.
- **Otros estudios**:
  - muros macizados en un color (verde oscuro, terracota, azul);
  - pavimentos con textura (adoquín irregular, baldosa, veta de madera, tarima);
  - suelo exterior punteado o en trama de lamas;
  - árboles en copa transparente con punto de tronco, y follaje dibujado a mano;
  - **sombras arrojadas** suaves, curvas de nivel y recorrido solar;
  - una paleta de cuatro o cinco tonos por lámina;
  - axonometrías con colores suaves, personas y árboles;
  - secciones con fondo de vegetación;
  - láminas de concurso con título, diagramas, planta, sección y render.

Bloques:

- **P1 Estilos de presentación** ✅ (21/10/2026, `bloque/presentacion`): estilos de vista guardables entre proyectos, con:
  - paleta;
  - macizado de muros en color;
  - grosores;
  - fondo;
  - tramas con textura (adoquín, losa, piedra irregular, madera, tarima, grava punteada, césped en franjas o punteado, agua);
  - un estilo «AD5» y tres o cuatro de presentación de partida.
- **P2 Vegetación y figuras** ✅ (21/10/2026): copas de árbol y arbustos generados (cada uno distinto, como pinceles), por especie y color, transparentes o rellenas; personas, coches y mobiliario de presentación como símbolo de escala en las recetas.
- **P3 Sombras y diagramas** ✅ (21/10/2026; las curvas de nivel ya estaban y toman la paleta): sombras arrojadas en planta y en axonometría desde el sol del proyecto, recorrido solar, curvas de nivel, cuadros de superficies con estilo.
- **P4 Láminas** ✅ (21/10/2026): plantillas de lámina de presentación (título, paleta, diagramas, vistas, imágenes) y exportación PNG de alta resolución.

### 6.4 Modelado de formas (Concepto, para visualización)

- **M1** ✅: primitivas (esfera, cilindro, cono, revolución de un perfil), redondeo y chaflán de aristas. *Hecho en v0.39* (`docs/FORMAS.md`).
- **M2** ✅: operaciones entre volúmenes (unir, restar, intersecar), intersección de caras y aristas sueltas sobre caras. *Hecho en v0.39*: cada cara del sólido conserva su material; «Intersecar caras» deja aristas donde se cruzan.
- **M3** ✅: texturas por cara y materiales de visualización. *Hecho en v0.39*: aspecto en 3D de cada material (color de superficie y textura: ladrillo, piedra, baldosa, madera, tablas, hormigón visto, enfoscado, césped, grava, agua, chapa), dibujado por la vista 3D sin imágenes; se pinta cara a cara con Pintar. Falta: texturas en las axonometrías de los planos y en el IFC.

### Orden propuesto

R12–R21 → P1–P4 → L1–L3 → M1–M3. L1 (importar mallas) puede adelantarse a P si hace falta para traer modelos de SketchUp.

## 7. Revisión de Antonio del 22/10/2026 (feedback acumulado)

Antonio probó v0.37–v0.41 («funciona todo perfecto») y entregó la lista de lo que echa en falta. Decidido el 22/10/2026: se hace en este orden; **unidades en metros** por defecto; «Accesorios» son los dos tipos (los de ArchiCAD que siguen a muros, forjados y cubiertas, y los de baño y cocina).

| Bloque | Qué | Estado |
|---|---|---|
| 1. Urbanismo | Cambiar el tipo de un árbol abría el mobiliario; vista previa al dibujar una calle; editar una vía después (puntos del eje); referencia de vía por eje o borde; referencia de muros en la barra contextual | ✅ v0.41.1 |
| 2. Gráficos y vistas | Colores y estilos de cada categoría como en Revit (forjados blancos, urbanismo y jardín en pastel, muros blancos con la cámara de aire solo por dentro, transparencias): control total del dibujo; tipografía elegante para textos y cotas; metros; vistas guardadas (superficies, albañilería…); plantas de techo; planos de referencia; líneas finas o grosor real | ✅ v0.42 (`docs/GRAFICOS.md`) |
| 3. Inicio y vistas previas | Pantalla de inicio con bienvenida y proyectos recientes; vista previa de cualquier cosa al moverla o colocarla | ✅ v0.43 (`docs/INICIO.md`) |
| 4. Modelado | Dividir caras con aristas y pintar solo esa región; empujar caras laterales o regiones en cualquier dirección; doblar y retorcer; perfil personalizado para Seguir; redondear aristas (ya existe en Propiedades → Aristas: hacerlo más visible) | ✅ v0.44 (`docs/FORMAS.md` §5) |
| 5. Muebles y accesorios | Muebles estirables desde la vista 3D; más parámetros (cajones, posición, tipos de pomo); accesorios de muro, forjado y cubierta (zócalos, cornisas, molduras, aleros, vierteaguas) y de baño y cocina | ✅ v0.45 (`docs/BIBLIOTECA_3D.md`) |
| 6. Símbolos DXF | Importar símbolos de planta desde DXF para objetos (vegetación y demás) | ✅ v0.46 (`docs/BIBLIOTECA_3D.md`) |
| 7. Render y Presto | Renderizado; mediciones y presupuestos con bancos de precios (Presto, BC3) | ✅ v0.47 (`docs/RENDER_PRESUPUESTOS.md`) |

El trabajo colaborativo de la lista ya está (Colaboración → Compartir en equipo, v0.38). Con el bloque 7, la revisión del 22/10/2026 queda completa: siguiente, lo que diga Antonio al probar v0.41.1–v0.47.

## 8. Entorno Maqueta y cubiertas (23/10/2026)

Pedido por Antonio el 23/10/2026: un editor tan intuitivo como Los Sims y tan modular como Paralives, para arquitectura, formas y urbanismo, con una imagen cartoon con sombras que no castigue al MacBook de 2016 («lo importante siempre será la fluidez»). Decidido con él: **no es otro programa ni otra disciplina**, sino un entorno (Técnica · Maqueta) sobre el mismo modelo; lo que es herramienta va a los dos y lo que es forma de presentarla, solo a la maqueta. Diseño en `docs/MAQUETA.md`.

Al probar inserciones pidió además buhardillas que se adapten a la cubierta (no existían) y tejas y canalones en el modelo.

| Bloque | Qué | Estado |
|---|---|---|
| K1 | Buhardillas (a dos aguas, a un agua, plana) y ventanas de cubierta pinchadas en un faldón | ✅ v0.48 |
| K2 | Tejas, caballetes, canalones y bajantes | ✅ v0.48 |
| MQ1 | Entorno Maqueta: interruptor, ventana sin columnas, catálogo Construir · Amueblar · Exterior, tarjeta flotante, cuadrícula, muros recortados, habitación arrastrando, cuentagotas | ◐ v0.48 (falta: muros de delante recortados, miniaturas en 3D, arrastrar en 3D) |
| V1 | Imagen: sombras, penumbra guardada, luz de ilustración, texturas como imagen, niveles de calidad | — |
| MQ2–MQ4 | Muebles modulares, pinceles de Exterior, flechas y aros en Concepto | — |
| UI1 | Concepto de interfaz del 24/10/2026, primera vuelta: ratón (izquierdo elige y mueve, rueda desplaza, derecho mantenido gira), menú del clic derecho, flechas y anillo, estirar por un solo lado, barra del objeto con materiales, ventanas en alzado en el catálogo, sin cartel de teclas | ✅ v0.54 (`bloque/manejo`, unida a la 0.53) |
| UI2 | Catálogo en árbol (grupo → tema → clase); cocina por módulos: bajos que juntan su encimera (cajonera, puertas, fregadero, horno, lavavajillas, placa, botellero, esquina), columnas, altos y campanas; frentes liso, con marco, ranurado y con cristal; cajones, tirador y frente en la barra, para un módulo o toda la fila | ✅ v0.55 (`bloque/cocina`) |
| UI3 | Diálogos junto al botón que los abre (nada aparece en una esquina sobre las herramientas) y con la misma forma; plano de carpinterías en A3 con alzados, secciones y datos (Archivo → Plano de carpinterías) | ✅ v0.55 (`bloque/cocina`) |
| UI4 | Interfaz en islas (paneles como tarjetas sobre fondo cálido); paneles laterales para gestionar, burbuja junto al botón para lo corto (Añadir planta con su sección), diálogo central con fondo oscurecido para decidir; avisos abajo con Deshacer; tarjeta de tipo con Planta · Alzado · 3D | ✅ v0.56 (`bloque/islas`) |

## 9. Hoja de ruta común (24/10/2026, v0.56)

Antonio trabaja con **varias conversaciones a la vez**. Esta sección es la lista única de lo que falta: cada conversación la lee al empezar, **marca aquí el bloque que coge** («en curso, rama `bloque/…`») con un commit en `main` antes de tocar código, y no toca los ficheros de un bloque que lleva otra. Así nadie rehace lo que ya hizo otra (como pasó con la 0.49, que salió de la 0.48 sin la Maqueta segunda vuelta).

Ya hecho y que **no se vuelve a abrir** salvo que Antonio lo pida: figuras en alzado y sección, Tab entre elementos, cambios por ejemplar y dibujos propios en alzado (v0.58–v0.59), tramas de presentación (v0.60), remates de la 0.48 rehechos sobre la 0.56 (v0.57), biblioteca acogedora y colecciones (v0.49–v0.50), muebles que crecen por piezas y se unen, color por pieza, luz de día, atardecer y noche (V1, primera vuelta), mobiliario urbano rehecho (v0.52), escaleras (v0.53), manejo común, cocina por módulos, catálogo en árbol, diálogos e islas (UI1–UI4, v0.54–v0.56).

Orden **confirmado por Antonio el 24/10/2026**: de C1 a C7, uno tras otro.

| # | Bloque | Qué | Tamaño | Estado |
|---|---|---|---|---|
| C1 | **Figuras y vegetación en alzado y sección** | Árboles, palmeras y arbustos dibujados en secciones, alzados y axonometrías como los dibuja un arquitecto (silueta de la copa con estilos como los de planta: contorno, lóbulos, acuarela, ramas; tronco; sin la red de facetas de la malla); coches y personas con su silueta. Coches y personas rehechos en el estilo de la biblioteca acogedora (los generadores son del 22/10/2026) | M | ✅ v0.58 (`bloque/c1-figuras`) |
| C1 bis | **Pedido por Antonio el 24/10/2026** | Tab entre elementos que se pisan en planta; lo cambiado en la Maqueta, del ejemplar («Aplicar a todos» para el tipo); galería de objetos con su 3D; dibujos propios en alzado (DXF en técnico, PNG en presentación) e imagen PNG en planta de presentación | M | ✅ v0.59 (`bloque/figuras-2`) |
| T1 | **Tramas de presentación** (pedidas por Antonio el 25/10/2026) | Madera (tablas, vetas, terminada, pino, roble, arce, abedul, fresno, nogal, caoba), metal (cepillado, perforado, chapa lagrimada, grecada, malla), hormigón (visto, en bruto, desnudo, adoquín, encofrado con tablas) y piedra (grava, caliza, bloques, sillería, terrazo, camino, adoquín en abanico, mampostería): gris, línea fina, trazo a mano, a tamaño real | M | ✅ v0.60 (`bloque/tramas`) |
| F1 | **Fluidez y líneas finas** (pedidas por Antonio el 25/10/2026) | Abrir un alzado sin espera; desplazar con el botón derecho sin retraso; opción «Líneas finas» también en planta y en 3D, no solo en secciones y alzados | S | **en curso, rama `bloque/fluidez`** |
| A1 | **Árboles en planta desde un DXF** (pedido por Antonio el 25/10/2026) | Elegir uno de los bloques de un DXF con muchos (una biblioteca de árboles) viendo su dibujo, como símbolo de planta de la planta o árbol; guardado en la biblioteca local de Antonio (no en el repositorio: los DXF de CAD素材.com son de terceros) | S | **en curso, rama `bloque/arboles-dxf`** |
| C2 | **Cocina y muebles, segunda vuelta** | Encimera continua de verdad en el IFC y en las mediciones; módulos de esquina en L con la encimera en ángulo; islas por módulos; sofá rinconera; unirse también al arrastrar en planta; rueda de color libre | M |**en curso, rama `bloque/c2-cocina`** |
| C3 | **Detalles de la Maqueta y de la interfaz** | Habitación en un solo paso de Deshacer; mover en 3D pegándose a las paredes y con cotas temporales; colocación de huecos «Por el centro / Desde un extremo / Repartidas»; todos los interruptores y botones con el mismo dibujo | M | — |
| C4 | **Salida para visualización** | El proyecto entero a `.glb` (D5, Twinmotion, Blender); `archisim render` con las texturas nuevas y la noche; sombras de las lámparas; texturas como imagen para los Mac con Intel | M | — |
| C5 | **Exterior y Concepto en la Maqueta** (MQ3–MQ4) | Pinceles de Exterior (césped, caminos, plantaciones pintadas); flechas y aros en Concepto | M | — |
| C6 | **Documentación** | Más láminas con el cajetín del plano de carpinterías: memoria de carpinterías, cuadro de superficies; lo que Antonio eche en falta del plano de carpinterías | M | — |
| C7 | **Deudas** | Escaleras: mesetas partidas a 45°, meseta de llegada de las de caracol. Buhardillas sobre limahoyas que pasan por un encuentro | S | — |
| T1 | **Tramas de presentación** (pedidas por Antonio el 25/10/2026) | Madera (pino, roble, arce, abedul, fresno, nogal, caoba, terminada, tablas, vetas), metal (cepillado, perforado, chapa lagrimada, grecado, malla), hormigón (terminado, bruto, desnudo, adoquín, encofrado de tablas) y piedra (grava, caliza, bloques, sillería, terrazo, camino, adoquín, mampostería): gris, línea fina, trazo a mano | M | hecho (v0.60) |

Pendiente de Antonio, fuera de la tabla:
- **#54 (remates v0.48)**: rehecho sobre la 0.56 como v0.57 (rama `bloque/remates-2`, 24/10/2026): texturas ligeras, puertas y ventanas en muros curvos, instalaciones (pendientes, codos y tes, radiadores, HS 4 y HS 5), presupuesto con descomposición y holguras de interferencias; las buhardillas se quedan las de la 0.53.
- Decisiones **D4** (nombre), **D5** (normas de dibujo), **D7** (IFC reales de prueba), **D8–D12** (disciplinas y mediciones); **catálogo real de AD5** (2.2) y **reglas del CTE** que quiera añadir.

## Propuesta de orden

1. ~~Bloque 8: **mover / copiar / estirar** con el ratón y **varita mágica** (1.1 + 1.2)~~ — hecho en v0.9, con la edición de vértices (1.3).
2. ~~Bloque 9: **cotas básicas** y **gestión de plantas** (1.4 + 1.5)~~ — hecho en v0.10.
3. ~~Bloque 10: **uniones en T y prioridades** (2.3)~~ — hecho en v0.11 (falta el recorte con forjados).
4. ~~Bloque 11: **editor de tipos y materiales** (1.6)~~ — hecho en v0.12.
5. ~~Bloque 12: **aplicación de Mac** (parte de 1.8)~~ — hecho en v0.13.
6. Fase 2 con **familias** (2.1): D2 decidida el 18/10/2026. Bloques 13 a 17 = A a E de `docs/FAMILIAS.md`; después la librería (2.2). Bloque 13 (A, núcleo) hecho en v0.14; bloque 14 (B, planta y hojas configurables) en v0.15.
7. ~~Bloques 19 a 22: **cierre de la fase 1** (1.7, 1.8 y 1.9) con modos de construcción y muros curvos~~ — hechos en v0.19 a v0.22. La §1 queda completa.
8. ~~**Fase 3, documentación** (3.1 a 3.8 y 2.9), pedida por Antonio el 19/10/2026 con herramientas de dibujo completas para quien viene de AutoCAD~~ — hecha en v0.25 (bloques 3A a 3I).
