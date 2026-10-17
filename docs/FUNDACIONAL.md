# ArchiSim — Documento fundacional

> Nombre provisional. Renombrar el proyecto es barato mientras sea pequeño.
> Versión 0.1 · 17 de octubre de 2026 · Antonio Fernández (AD5)

## 1. Visión

Una herramienta BIM de autoría, multiplataforma (Windows, macOS, Linux), escrita en Rust, que combine:

- **De ArchiCAD:** la facilidad de uso. Herramientas directas (muro, forjado, ventana) con diálogo de configuración claro, multicapas y materiales de construcción con prioridad de intersección, y una librería de objetos lista para trabajar desde el primer día.
- **De Revit:** la forma de organizar la información, que es el estándar del mercado y lo que conocen ingenieros y técnicos. Categorías, familias → tipos → ejemplares, parámetros compartidos, fases, vistas y planos, tablas de planificación y modelo central.

Enfoque principal: **arquitectura**. Objetivo secundario: que ingenieros (estructuras, instalaciones) puedan trabajar sobre el mismo modelo o intercambiarlo sin fricción.

## 2. Principios no negociables

1. **IFC 4.3 es el modelo de datos.** No es un formato de exportación tardío: cada elemento nace con su clase IFC, su GUID estable y sus property sets. Cualquier dato que no tenga equivalente IFC se guarda en property sets propios con prefijo `ArchiSim_`.
2. **Sala limpia.** No se copia código, ficheros ni contenido de Autodesk ni de Graphisoft. Se replican comportamientos observables, flujos de trabajo y estándares públicos (IFC, BCF, IDS, bSDD). La librería de objetos es propia.
3. **No se promete leer o escribir .rvt/.rfa directamente.** La compatibilidad con Revit se consigue con IFC y, más adelante, con un complemento C# que corre dentro de Revit usando su API oficial.
4. **El arquitecto decide.** Toda salida gráfica (plantas, secciones, tramas, acotación) se valida con criterio profesional antes de darla por buena.
5. **Unidades:** internamente metros en `f64`; en la interfaz, milímetros por defecto.

## 3. Arquitectura

```
┌──────────────────────────────────────────────────────────────┐
│ app (interfaz)  — wgpu + egui, estilo ArchiCAD               │
├──────────────────────────────────────────────────────────────┤
│ tools — herramientas de dibujo: muro, forjado, hueco…         │
├──────────────┬───────────────────┬───────────────────────────┤
│ plan         │ ifc               │ (futuro) families, views   │
│ vistas 2D    │ import/export     │ sheets, schedules          │
├──────────────┴───────────────────┴───────────────────────────┤
│ geom — geometría: polígonos, uniones de muros, extrusiones    │
├──────────────────────────────────────────────────────────────┤
│ core — documento, elementos, tipos, materiales, transacciones │
└──────────────────────────────────────────────────────────────┘
```

Crates actuales (`crates/`):

| Crate | Responsabilidad |
|---|---|
| `archisim-core` | Documento del proyecto, niveles, materiales, tipos de muro y de forjado multicapa, muros, forjados, tipos de puerta y ventana, huecos en muro, deshacer/rehacer |
| `archisim-geom` | Vectores 2D, contorno de muros con uniones en esquina a inglete, área de polígonos, posición de huecos y recorte de muros en tramos |
| `archisim-ifc` | Escritor IFC4 (STEP) con jerarquía espacial, tipos, capas de material y Psets de muros y forjados; huecos (`IfcOpeningElement`) rellenos con puertas y ventanas |
| `archisim-plan` | Planta en SVG: poché por capa, forjados según plano de corte, muro interrumpido en huecos, símbolos de puerta y ventana, ejes, rotulación |
| `archisim-file` | Proyecto `.archisim`: JSON versionado que conserva ids y `GlobalId`, se comprueba al abrir y se guarda de forma segura (ver `docs/FORMATO_ARCHISIM.md`) |
| `archisim-cli` | Línea de comandos: `demo` genera una vivienda de prueba, la guarda y la exporta; `exportar` abre un `.archisim` y lo exporta |

Reglas de dependencias: `core` no depende de nadie del proyecto; `geom` solo de `core`; los exportadores dependen de `core` + `geom`; `file` (formato de proyecto) solo de `core`; la interfaz depende de todo y nada depende de la interfaz.

## 4. Modelo de datos (equivalencias)

| Concepto ArchiSim | Revit | ArchiCAD | IFC |
|---|---|---|---|
| `Level` | Nivel | Planta | `IfcBuildingStorey` |
| `Material` | Material | Material de construcción | `IfcMaterial` |
| `WallType` (capas) | Tipo de muro (estructura compuesta) | Multicapa | `IfcWallType` + `IfcMaterialLayerSet` |
| `Wall` | Ejemplar de muro | Muro | `IfcWall` + `IfcMaterialLayerSetUsage` |
| `SlabType` (capas) | Tipo de suelo / cubierta (estructura compuesta) | Multicapa de forjado | `IfcSlabType` + `IfcMaterialLayerSet` |
| `Slab` | Suelo / Cubierta por contorno | Forjado | `IfcSlab` + `IfcMaterialLayerSetUsage` (`AXIS3`) |
| Línea de referencia | Línea de ubicación | Línea de referencia | Eje + `OffsetFromReferenceLine` |
| `OpeningType` | Tipo de familia de puerta / ventana | Objeto puerta / ventana | `IfcDoorType` / `IfcWindowType` + `IfcMaterialConstituentSet` |
| `Opening` | Ejemplar de puerta / ventana (alojado en muro) | Puerta / ventana | `IfcOpeningElement` (vacía el muro) + `IfcDoor` / `IfcWindow` (lo rellena) |
| Familia (futuro) | Familia .rfa | Objeto GDL | `IfcTypeObject` + geometría paramétrica |
| Parámetro compartido | Parámetro compartido | Propiedad | `IfcPropertySet` |

Cada elemento tiene un `ElementId` interno y un `GlobalId` IFC de 22 caracteres que no cambia nunca, ni al copiar el proyecto ni al exportar varias veces. Así los ingenieros pueden federar modelos y comparar versiones.

### Muros

- Definidos por un eje entre dos puntos, nivel base, altura y tipo.
- La **línea de referencia** puede ir en el centro, en la cara exterior o en la interior (como en ArchiCAD y Revit).
- Las capas se ordenan de **exterior a interior**; la cara exterior queda a la izquierda del eje, mirando de inicio a fin.
- Las uniones en esquina se resuelven a inglete cuando dos muros comparten un extremo. Las uniones en T, las prioridades de intersección por capa y los recortes con forjados vienen en la fase 2.

### Forjados

- Definidos por un contorno poligonal cerrado (cualquier forma sin cruces: rectángulo, L, U…), un nivel, un tipo y un desfase.
- La referencia es la **cara superior**, como en ArchiCAD y Revit: queda a la cota del nivel más el desfase, y las capas cuelgan hacia abajo.
- Las capas del tipo se ordenan de **arriba abajo** (pavimento → estructura → acabado inferior).
- Función del tipo: forjado de planta (`FLOOR`), solera (`BASESLAB`) o cubierta plana (`ROOF`). Solera y cubierta se marcan como exteriores (`IsExternal`).
- En planta se dibujan según el plano de corte (1,10 m por defecto): por debajo, con trama de cubrición y línea fina de proyección; cortados, con poché de su capa estructural y línea gruesa; por encima, discontinuos.
- Pendiente: huecos en forjado (escaleras, patinillos), forjados inclinados, y recorte con muros (fase 2).

### Puertas y ventanas

- Un **tipo** de puerta o ventana fija el funcionamiento (puerta abatible de una o dos hojas o corredera; ventana fija, practicable de una o dos hojas o corredera), el **hueco de obra** (ancho × alto), el perfil del marco (anchura vista y fondo) y dos materiales: marco y hoja/vidrio.
- Cada **ejemplar** vive en un muro: posición del centro medida sobre la línea de referencia desde el inicio del muro (como Revit), altura de antepecho desde el nivel, lado hacia el que abre (cara interior o exterior) y mano según UNE-EN 12519 (mirando desde el lado hacia el que abre la hoja, bisagras a izquierda o derecha). El retranqueo de la carpintería se mide desde la cara exterior del muro; si no se indica, se centra.
- Se rechaza lo que no cabe: fuera del largo o de la altura del muro, solapado con otro hueco del mismo muro, o con un marco más profundo que el muro. Borrar el muro borra sus huecos; acortarlo de forma que un hueco se salga se rechaza; invertirlo deja los huecos en su sitio.
- En planta, los huecos que corta el plano de corte interrumpen el muro y todas sus capas; las mochetas se cierran con línea de corte. En fachadas de doble hoja, la hoja exterior **vuelve** en la mocheta cerrando la cámara (el aislamiento se detiene a un espesor de ladrillo de la jamba) y el revestimiento exterior (monocapa) **envuelve** la mocheta hasta la carpintería; tabiques y muros sin cámara se cortan rectos. Puertas: cerco, hoja abierta a 90° y arco de giro (correderas: hoja sobre la cara y flecha). Ventanas: marco, línea de vidrio, montante central en las de dos hojas, giro de las hojas practicables (línea fina y arco), hojas desfasadas en las correderas, vierteaguas al exterior y borde del antepecho al interior. Las ventanas que quedan enteras por encima del plano de corte se dibujan discontinuas sin interrumpir el muro.
- En IFC el hueco atraviesa el muro (`IfcRelVoidsElement`) y la puerta o ventana lo rellena (`IfcRelFillsElement`). Su geometría 3D es sencilla (marco, hojas y vidrios cerrados) y lleva `Pset_DoorCommon` / `Pset_WindowCommon` y cantidades. El muro añade superficie lateral y volumen netos.
- Pendiente: premarcos, vuelta del revestimiento interior, persianas y capialzados, dinteles, huecos sin carpintería, puertas y ventanas paramétricas en la librería de familias (fase 2).

## 5. MVP (fase 1)

Definición de "hecho": un arquitecto modela una vivienda sencilla de una planta, obtiene su planta en PDF/SVG legible y exporta un IFC que **Revit y ArchiCAD abren con muros reconocibles, tipos y capas correctos**.

| # | Entregable | Estado |
|---|---|---|
| 1 | Núcleo: niveles, materiales, tipos multicapa, muros, deshacer | ✅ v0.1 |
| 2 | Uniones de muros en L | ✅ v0.1 |
| 3 | Exportación IFC4 con capas, tipos y Psets | ✅ v0.1 |
| 4 | Planta SVG con poché por capa | ✅ v0.1 |
| 5 | Forjados (`IfcSlab`) por contorno | ✅ v0.2 |
| 6 | Puertas y ventanas como huecos en muro (`IfcOpeningElement`) | ✅ v0.3 |
| 7 | Formato de proyecto propio (`.archisim`, JSON versionado) | ✅ v0.4 |
| 8 | Ventana de la aplicación: vista de planta, herramienta muro, selección | ⬜ |
| 9 | Vista 3D | ⬜ |
| 10 | Importación IFC básica | ⬜ |

## 6. Fases siguientes

- **Fase 2:** sistema de familias paramétricas (lenguaje declarativo propio), librería inicial de puertas, ventanas, mobiliario y sanitarios; cubiertas, escaleras, secciones y alzados con líneas ocultas; uniones en T y prioridades de intersección.
- **Fase 3:** planos y maquetación, acotación, etiquetas, tablas de planificación, impresión a PDF.
- **Fase 4:** complemento puente para Revit, BCF, colaboración (modelo central), disciplinas de ingeniería.

## 7. Forma de trabajo

- **Antonio:** experto BIM y dueño del producto. Prueba cada versión, abre los IFC en ArchiCAD y Revit, y da feedback.
- **Claude (Claude Code):** escribe todo el código, las pruebas y la documentación técnica. Ver `CLAUDE.md`.
- Cada cambio llega con pruebas automáticas. Cada exportación IFC se valida con IfcOpenShell antes de darla por buena.
- Feedback de Antonio → issue en GitHub con captura y, si se puede, el IFC o SVG afectado.

## 8. Licencia

Pendiente de decidir. El repositorio será privado hasta entonces. Candidatas para abrirlo: **MPL-2.0** (permite usarlo en productos cerrados pero obliga a publicar mejoras del propio código) o **GPL-3.0** (todo derivado debe ser libre). No se añade fichero de licencia hasta decidir.
