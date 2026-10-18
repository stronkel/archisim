# ArchiSim — Hoja de ruta después del MVP

> 17 de octubre de 2026 · versión 0.8 · MVP (fase 1) terminado. Actualizada el 18/10/2026 (D2 decidida).
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
| D4 | **Licencia** (MPL-2.0 o GPL-3.0) y **nombre definitivo** | Necesario antes de abrir el repositorio |
| D5 | **Normas de dibujo de referencia**: tramas, grosores y rotulación (UNE-EN ISO 128, UNE 1-027 para plegado, criterio propio de AD5) | Fija cómo se ven plantas, secciones y planos |
| D6 | **Plantilla al importar**: añadir siempre la plantilla de ArchiSim a un proyecto importado o solo cuando faltan tipos (hoy: solo cuando faltan) | Comodidad frente a limpieza del proyecto |
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
| 1.7 | **Importación IFC, segunda vuelta**: muros curvos y de varios tramos, forjados inclinados y con huecos, huecos sin carpintería, pilares y vigas como elementos propios en cuanto existan (§2), locales (`IfcSpace`) | M |
| 1.8 ◐ v0.13 | **Instalación**: hecha la aplicación de Mac universal en `.dmg` con icono y asociación de ficheros; falta el instalador de Windows con icono y asociación de `.archisim` e `.ifc` | S |
| 1.9 | **Rendimiento** con modelos grandes (miles de elementos, IFC de 50–100 MB): medir y optimizar planta, 3D e importación | M |

## 2. Fase 2 — Modelado completo

| # | Tarea | Tamaño | Depende de |
|---|---|---|---|
| 2.1 ★ ◐ v0.15 | **Sistema de familias paramétricas** — bloques A y B hechos; siguen C a E — (diseño en `docs/FAMILIAS.md`), en cinco bloques: **A** núcleo (recetas, parámetros por secciones, fórmulas, piezas comunes, 3D; puertas y ventanas actuales convertidas en cinco familias; formato v3), **B** símbolos de planta por escala y alzado, **C** IFC (tipos con marca, Psets, `ArchiSim_Family`), **D** configurador con vista previa en directo y puntos de control, **E** navegador de familias y tipos con nº de ejemplares, biblioteca de tipos (`.asfam`) y herramienta Objeto | L | D2 ✅ |
| 2.2 | **Librería inicial propia**: puertas y ventanas paramétricas (con premarco, persiana y capialzado, vierteaguas, tarjas, puerta de dos hojas desiguales y hoja activa, corredera empotrada en armazón, corredera de hasta 4 hojas, **puerta plegable**), mobiliario, sanitarios, cocina; plantilla con un catálogo de tipos habituales de AD5 | L | 2.1 |
| 2.3 ◐ v0.11 | **Uniones en T y prioridades de intersección por capa** (la estructura atraviesa, los acabados se detienen), como ArchiCAD — hechas; falta el recorte de muros con forjados y las prioridades editables por material | M | — |
| 2.4 | **Cubiertas inclinadas** por faldones (pendiente, aleros, limatesas y limahoyas) y recorte de muros bajo cubierta | L | 2.3 |
| 2.5 | **Pilares y vigas** (hormigón, acero, madera) con perfiles; `IfcColumn`, `IfcBeam` | M | — |
| 2.6 | **Escaleras** por tramos y mesetas con comprobación de huella, tabica y anchura (CTE DB-SUA); **barandillas**; **rampas** | L | 2.1 |
| 2.7 | **Huecos en forjados** (escaleras, patinillos) y **forjados inclinados** | S | — |
| 2.8 | **Muros curvos, muros de perfil complejo y muros cortina** (montantes, travesaños, paneles) | L | 2.1 |
| 2.9 | **Locales** (`IfcSpace`): nombre, número, superficie útil y construida, uso | M | 1.2 |
| 2.10 | **Secciones y alzados** con líneas ocultas, profundidad de vista y poché por capa | L | 2.3 |
| 2.11 | **Fases**: existente, a derribar, nuevo (reformas), con su grafismo | M | — |
| 2.12 | **Terreno** sencillo (curvas de nivel o puntos), emplazamiento y orientación real del norte | M | — |

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
