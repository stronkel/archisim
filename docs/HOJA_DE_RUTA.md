# ArchiSim — Hoja de ruta después del MVP

> 17 de octubre de 2026 · versión 0.8 · MVP (fase 1) terminado. Actualizada el 18/10/2026 (D2 decidida; §1 cerrada en v0.22).
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
| D3 | **IFC de salida por defecto**: IFC4 (lo que mejor abren hoy Revit y ArchiCAD) o IFC 4.3 (lo que dice el principio 1) | Afecta a la validación y a lo que prueban los ingenieros |
| D4 ◐ | **Licencia** — decidida el 18/10/2026: **GPL-3.0 o posterior**; repositorio público desde ese día. Falta el **nombre definitivo** (ArchiSim es provisional) | El nombre conviene fijarlo antes de anunciarlo a la comunidad |
| D5 | **Normas de dibujo de referencia**: tramas, grosores y rotulación (UNE-EN ISO 128, UNE 1-027 para plegado, criterio propio de AD5) | Fija cómo se ven plantas, secciones y planos |
| D6 | **Plantilla al importar**: añadir siempre la plantilla de ArchiSim a un proyecto importado o solo cuando faltan tipos (hoy: solo cuando faltan) | Comodidad frente a limpieza del proyecto |
| D8–D12 | **Disciplinas y mediciones** (propuesta del 18/10/2026 en `docs/DISCIPLINAS.md`): un solo modelo con disciplina por elemento, IFC 4.3 para urbanismo, banco de precios de referencia (BCCA), criterio de deducción de huecos y orden de los bloques | Estructura, urbanismo y mediciones con bancos de precios dependen de ellas |
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

**Modos de construcción y muros curvos** ✅ v0.19 (pedidos por Antonio el 18/10/2026, junto a 1.7): métodos de la herramienta Muro (recto, cadena, rectángulo por dos esquinas, arco por inicio, fin y punto de paso, círculo por centro y punto) y de la herramienta Forjado (polígono, rectángulo por dos esquinas, varita mágica); muros curvos de punta a punta (planta con capas, 3D, ingletes con muros rectos y curvos, IFC exacto con arcos, `.archisim` versión 5), curvar con el ratón y radio en Propiedades. Falta: puertas y ventanas en muros curvos, uniones en T por capas contra muros curvos (de momento el muro que acomete se detiene en la cara), forjados con bordes curvos o circulares, rectángulo girado, arco tangente.

**Ayudas al dibujo** ✅ v0.18 (pedidas por Antonio el 18/10/2026, antes del bloque C): cotas temporales azules editables, forzado a puntos medios de caras y aristas, muros ortogonales por defecto con línea de rastreo, Espacio y flechas para girar puertas y ventanas, cambiar de planta muros y forjados, muros de otras plantas cortados por el plano de corte y calco de la planta inferior. Falta: cotas temporales de forjados y cotas a ejes o a núcleo (como opción de Revit), seleccionar en una planta los muros de otra que se ven cortados, copiar a otra planta.

**Restricción superior de muros** ✅ v0.17 (pedida por Antonio el 18/10/2026, antes del bloque C de familias): muros hasta una planta con desfase, que siguen los cambios de cota. Falta lo mismo para pilares cuando existan (2.5) y el recorte con forjados y cubiertas (2.3, 2.4).

| # | Tarea | Tamaño | Depende de |
|---|---|---|---|
| 2.1 ★ ◐ v0.24 | **Sistema de familias paramétricas** — bloques A, B y C hechos, D casi (falta símbolo 1:20 y puntos de control); sigue E — (diseño en `docs/FAMILIAS.md`), en cinco bloques: **A** núcleo (recetas, parámetros por secciones, fórmulas, piezas comunes, 3D; puertas y ventanas actuales convertidas en cinco familias; formato v3), **B** símbolos de planta por escala y alzado, **C** IFC (tipos con marca, Psets, `ArchiSim_Family`), **D** configurador con vista previa en directo y puntos de control, **E** navegador de familias y tipos con nº de ejemplares, biblioteca de tipos (`.asfam`) y herramienta Objeto | L | D2 ✅ |
| 2.2 ◐ v0.25 | **Librería inicial propia** (hechas la puerta plegable, la corredera empotrada y la persiana con capialzado): puertas y ventanas paramétricas (con premarco, persiana y capialzado, vierteaguas, tarjas, puerta de dos hojas desiguales y hoja activa, corredera empotrada en armazón, corredera de hasta 4 hojas, **puerta plegable**), mobiliario, sanitarios, cocina; plantilla con un catálogo de tipos habituales de AD5 | L | 2.1 |
| 2.3 ◐ v0.11 | **Uniones en T y prioridades de intersección por capa** (la estructura atraviesa, los acabados se detienen), como ArchiCAD — hechas, también las prioridades editables por material (v0.25, de 0 a 999); falta el recorte de muros con forjados | M | — |
| 2.4 ◐ v0.25 | **Cubiertas inclinadas** por faldones (pendiente, aleros, limatesas y limahoyas) y recorte de muros bajo cubierta — hechas las cubiertas sobre contorno convexo (a cuatro, dos o un agua y a medida por lado, alero, planta de cubiertas, 3D, secciones, IFC `IfcRoof` y su lectura de vuelta); y el recorte de muros bajo cubierta (piñones, IFC con semiespacios); falta: contornos con esquinas entrantes (en L: limahoyas), buhardillas | L | 2.3 |
| 2.5 ◐ v0.25 | **Pilares y vigas** (hormigón, acero, madera) con perfiles; `IfcColumn`, `IfcBeam`; también importarlos — hechos (sección rectangular, circular, I/H, U, T, L y tubos, restricción superior de pilares, planta, 3D, IFC e importación); falta el recorte con muros y forjados | M | — |
| 2.6 ◐ v0.25 | **Escaleras** por tramos y mesetas con comprobación de huella, tabica y anchura (CTE DB-SUA); **barandillas**; **rampas** — hechas las escaleras de losa de hormigón por línea de huella (rectas, en L, en U, con mesetas; tabicas iguales hasta la planta de llegada; comprobación CTE uso general y restringido; planta con línea de corte, 3D, secciones, IFC `IfcStair` y su lectura de vuelta); barandillas de escalera con pasamanos y montantes; las rampas se hacen ya con forjados inclinados (2.7). barandillas sueltas de barrotes para terrazas y cubiertas (cumplen los 10 cm del CTE). Falta: barandillas de vidrio o chapa, peldañeado compensado y escaleras de caracol, zancas metálicas y de madera | L | 2.1 |
| 2.7 ✅ v0.25 | **Huecos en forjados** (hechos en v0.20) y **forjados inclinados** (también importarlos) — hechos (flecha de pendiente con % y dirección, arrastrable; 3D, secciones, IFC e importación). Falta: forjados con varias pendientes (eso son las cubiertas, 2.4) | S | — |
| 2.8 ◐ v0.25 | **Muros curvos** (hechos, sin huecos todavía), **muros cortina** (hechos: rectos con retícula regular de montantes, travesaños y vidrios) y **muros de perfil complejo** (pendiente) | L | 2.1 |
| 2.9 ✅ v0.25 | **Locales** (`IfcSpace`): nombre, número, superficie útil y construida, uso; también importarlos — hechos (herramienta Local por recinto, polígono o rectángulo; sello en planta; IFC e importación). Falta: editar los vértices del contorno, rellenos por uso, tablas de superficies (3.7) | M | 1.2 |
| 2.10 ✅ v0.25 | **Secciones y alzados** con líneas ocultas, profundidad de vista y poché por capa — hechos (herramienta Sección, cuatro alzados automáticos, marcas en planta, vista en pestaña con corte por material y capas, vista sin líneas ocultas, plantas acotadas, SVG desde la línea de comandos). Falta: cotas en sección (3.2), línea de terreno, secciones quebradas | L | 2.3 |
| 2.11 ✅ v0.25 | **Fases**: existente, a derribar, nuevo (reformas), con su grafismo — hechas (fase por elemento, «dibujar como», vistas Estado actual / Demoliciones / Estado reformado en amarillo y rojo, 3D, IFC `Status`). Falta: tramas o colores configurables por estudio (D5) y fase en locales | M | — |
| 2.12 ◐ v0.25 | **Terreno** sencillo (curvas de nivel o puntos), emplazamiento y orientación real del norte — hechos el terreno por puntos de cota (superficie triangulada, curvas de nivel acotadas en planta, sección del terreno, IFC `IfcGeographicElement`) y el norte real (flecha en las plantas exportadas, brújula del 3D, IFC `TrueNorth`). Falta: importar curvas de nivel de un DXF o puntos de un CSV, explanaciones y vaciados, coordenadas geográficas del emplazamiento | M | — |

## 3. Fase 3 — Documentación

| # | Tarea | Tamaño | Depende de |
|---|---|---|---|
| 3.1 | **Plantillas de vista y estilos gráficos**: qué se ve, con qué grosores y tramas, por escala (1:100, 1:50, 1:20); capas de dibujo | M | D5 |
| 3.2 | **Acotación completa**: cotas asociativas que se actualizan al mover, cotas de altura en sección, cotas de superficie | M | 1.4, 2.10 |
| 3.3 | **Etiquetas**: de puertas y ventanas (tipo, medida), de locales (nombre, superficie), de muros (tipo) | S | 2.9 |
| 3.4 | **Textos, líneas, tramas y detalles 2D** sobre las vistas | M | — |
| 3.5 | **Planos y maquetación**: hojas con cajetín propio, varias vistas por hoja a distintas escalas, numeración | L | 3.1 |
| 3.6 | **Salida PDF vectorial** e impresión; exportación **DXF** | M | 3.5 |
| 3.7 | **Tablas de planificación**: carpinterías, superficies de locales, mediciones de muros y forjados por capa | M | 2.9 |
| 3.8 | **Mediciones para presupuesto** en formato **FIEBDC-3 (BC3)** | M | 3.7 |

## 4. Fase 4 — Colaboración e ingenierías

| # | Tarea | Tamaño | Depende de |
|---|---|---|---|
| 4.1 | **Vínculo de modelos IFC** de estructuras e instalaciones como referencia (sin importarlos), con control de versiones por `GlobalId` | M | — |
| 4.2 | **Detección de interferencias** entre el modelo y los vinculados | M | 4.1 |
| 4.3 | **BCF**: incidencias con vista guardada, intercambiables con Revit, ArchiCAD, Solibri o BIMcollab | M | — |
| 4.4 | **IDS**: comprobar el modelo contra los requisitos de información del cliente; **bSDD** para clasificaciones | M | — |
| 4.5 | **Complemento puente para Revit** (C#, API oficial de Revit): llevar tipos y elementos nativos en ambos sentidos | L | — |
| 4.6 | **Trabajo en equipo**: modelo central, reservas de elementos o zonas, historial | L | — |
| 4.7 | **Comprobaciones normativas** del CTE: evacuación (DB-SI), accesibilidad (DB-SUA), superficies útiles | L | 2.9 |

## 5. Transversal (cuando toque)

- **Formato `.archisim`**: migraciones probadas con proyectos de cada versión; guardado más rápido para proyectos grandes.
- **Interfaz** ✅ v0.16: rediseñada como ArchiCAD con propiedades de Revit (caja de herramientas por secciones, pestañas de vistas con su escala, navegador de vistas, planos y tipos, barra inferior con escala y forzados, tema claro y oscuro). Pendiente: recordar la disposición y el tema entre sesiones, paleta de órdenes (buscar cualquier orden por su nombre), atajos configurables.
- **Manual de usuario** en español y lista de atajos; **traducción al inglés** de la interfaz antes de abrir el código.
- **Informes de fallo** con el proyecto adjunto (sin telemetría).
- **Validación continua**: además de IfcOpenShell, abrir los IFC de cada versión en ArchiCAD y Revit (Antonio) y guardar capturas de referencia.

## Propuesta de orden

1. ~~Bloque 8: **mover / copiar / estirar** con el ratón y **varita mágica** (1.1 + 1.2)~~ — hecho en v0.9, con la edición de vértices (1.3).
2. ~~Bloque 9: **cotas básicas** y **gestión de plantas** (1.4 + 1.5)~~ — hecho en v0.10.
3. ~~Bloque 10: **uniones en T y prioridades** (2.3)~~ — hecho en v0.11 (falta el recorte con forjados).
4. ~~Bloque 11: **editor de tipos y materiales** (1.6)~~ — hecho en v0.12.
5. ~~Bloque 12: **aplicación de Mac** (parte de 1.8)~~ — hecho en v0.13.
6. Fase 2 con **familias** (2.1): D2 decidida el 18/10/2026. Bloques 13 a 17 = A a E de `docs/FAMILIAS.md`; después la librería (2.2). Bloque 13 (A, núcleo) hecho en v0.14; bloque 14 (B, planta y hojas configurables) en v0.15.
7. ~~Bloques 19 a 22: **cierre de la fase 1** (1.7, 1.8 y 1.9) con modos de construcción y muros curvos~~ — hechos en v0.19 a v0.22. La §1 queda completa.
