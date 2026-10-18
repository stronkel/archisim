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
| `archisim-core` | Documento del proyecto, niveles, materiales, tipos de muro y de forjado multicapa, muros, forjados, tipos de puerta y ventana, huecos en muro, deshacer/rehacer; edición de materiales y tipos con comprobación de los elementos que los usan |
| `archisim-geom` | Vectores 2D, contorno de muros con uniones en esquina a inglete y uniones en T con prioridad por capa (`joins`), área de polígonos, posición de huecos y recorte de muros en tramos; recintos cerrados por muros (varita mágica); sólidos 3D de muros (por capas, con sus huecos), forjados y carpinterías, los mismos que exporta el IFC |
| `archisim-ifc` | Escritor IFC4 (STEP) con jerarquía espacial, tipos, capas de material y Psets de muros y forjados; huecos (`IfcOpeningElement`) rellenos con puertas y ventanas. Lector STEP e importador IFC2X3/IFC4 de plantas, muros con capas, forjados, puertas y ventanas, con informe de lo que queda fuera |
| `archisim-plan` | Planta en SVG: poché por capa, forjados según plano de corte, muro interrumpido en huecos, símbolos de puerta y ventana, ejes, rotulación; cotas y cotas de nivel con rotulación técnica de trazo propia (`lettering`) |
| `archisim-file` | Proyecto `.archisim`: JSON versionado que conserva ids y `GlobalId`, se comprueba al abrir y se guarda de forma segura (ver `docs/FORMATO_ARCHISIM.md`) |
| `archisim-library` | Plantilla de inicio: materiales y tipos de muro, forjado, puerta y ventana de construcción española habitual, con los que empieza todo proyecto nuevo (y la vivienda de prueba) |
| `archisim-cli` | Línea de comandos: `demo` genera una vivienda de prueba, la guarda y la exporta; `exportar` abre un `.archisim` y lo exporta; `importar` convierte un IFC en proyecto |
| `archisim-app` | La aplicación de escritorio (egui sobre wgpu): vista de planta y vista 3D, herramientas muro, forjado (por polígono o varita mágica), puerta, ventana y cota, gestión de plantas, tipos y materiales (multicapas, puertas, ventanas y materiales, con vista previa), selección y propiedades, mover, copiar y estirar con el ratón, deshacer, abrir/guardar `.archisim` (también soltando el fichero en la ventana o, en Mac, con doble clic en el Finder), importar IFC, exportar IFC, planta e imagen 3D |

Reglas de dependencias: `core` no depende de nadie del proyecto; `geom` solo de `core`; los exportadores dependen de `core` + `geom`; `file` (formato de proyecto) y `library` (plantilla) solo de `core`; la interfaz depende de todo y nada depende de la interfaz.

La vista de planta de la aplicación **es** la planta 1:50 que se exporta: se dibuja el mismo SVG de la zona visible y se rasteriza a la resolución de pantalla (grosores de línea reales a 1:50, como la vista de «grosor de línea real» de ArchiCAD). Lo que se ve es lo que se exporta. Encima, en vectorial, van la selección, la vista previa del muro y los puntos de forzado.

La vista 3D dibuja con la tarjeta gráfica (wgpu) los sólidos de `archisim-geom`, que son las mismas formas que recibe el IFC (el volumen de cada muro coincide con su `NetVolume`). Cada muro y cada forjado es una sola piel cerrada, con el color de cada capa en sus cantos y mochetas; las aristas de contorno salen de la propia imagen (cambios de orientación y de profundidad), el vidrio es transparente y la imagen se suaviza con supermuestreo ×2 y multimuestreo ×4. La selección es la misma que en planta.

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
| Familia (fase 2, `docs/FAMILIAS.md`) | Familia .rfa | Objeto GDL | `IfcTypeObject` + geometría; parámetros en `ArchiSim_Family` |
| Parámetro compartido | Parámetro compartido | Propiedad | `IfcPropertySet` |

Cada elemento tiene un `ElementId` interno y un `GlobalId` IFC de 22 caracteres que no cambia nunca, ni al copiar el proyecto ni al exportar varias veces. Así los ingenieros pueden federar modelos y comparar versiones.

### Muros

- Definidos por un eje entre dos puntos, nivel base, altura y tipo.
- La **línea de referencia** puede ir en el centro, en la cara exterior o en la interior (como en ArchiCAD y Revit).
- Las capas se ordenan de **exterior a interior**; la cara exterior queda a la izquierda del eje, mirando de inicio a fin.
- Las uniones en esquina se resuelven a inglete cuando dos muros comparten un extremo.
- **Uniones en T con prioridad de intersección por capa** (como ArchiCAD): un muro cuyo extremo cae dentro de otro (en su cara, en su eje o en su espesor), lejos de sus esquinas, acomete a él. Cada capa del muro que acomete entra en el otro atravesando sus capas de menor prioridad, desde la cara por la que llega, y se detiene en la primera de prioridad igual o mayor: la estructura atraviesa, los acabados se detienen. Prioridades por función de capa: estructura 4 > soporte (hoja interior) 3 > aislamiento 2 > cámara y lámina 1 > acabado 0. Así, la hoja de LHD de un tabique atraviesa el enlucido de la fachada hasta su hoja interior; un muro de carga atraviesa la hoja interior y el aislamiento hasta la hoja exterior. Nunca se atraviesa la última capa del otro muro, y solo se entra si el muro que acomete cabe en la altura del otro y está lejos de sus esquinas; si no, se detiene en su cara.
- El muro que recibe pierde esa parte de sus capas solo en la altura del que acomete: en planta, en 3D (una sola piel cerrada por muro) y en el IFC (una extrusión por tramo de altura; el volumen neto sigue coincidiendo). En planta la cara del muro que recibe no se corta donde acomete el otro: una línea fina marca dónde se detienen sus capas.
- Pendiente: recorte de muros con forjados y cubiertas, prioridades editables por material, uniones en L por capas y cruces en X (fase 2).

### Forjados

- Definidos por un contorno poligonal cerrado (cualquier forma sin cruces: rectángulo, L, U…), un nivel, un tipo y un desfase.
- La referencia es la **cara superior**, como en ArchiCAD y Revit: queda a la cota del nivel más el desfase, y las capas cuelgan hacia abajo.
- Las capas del tipo se ordenan de **arriba abajo** (pavimento → estructura → acabado inferior).
- Función del tipo: forjado de planta (`FLOOR`), solera (`BASESLAB`) o cubierta plana (`ROOF`). Solera y cubierta se marcan como exteriores (`IsExternal`).
- En planta se dibujan según el plano de corte (1,10 m por defecto): por debajo, con trama de cubrición y línea fina de proyección; cortados, con poché de su capa estructural y línea gruesa; por encima, discontinuos.
- **Varita mágica**: un clic dentro de un recinto de muros crea el forjado con su contorno, hasta la cara interior de los muros, hasta su eje, o por la cara exterior de todo el edificio. Los muros acometidos a la cara de otro (tabiques) cierran recintos; un muro suelto dentro de un recinto se ignora (no hay huecos en forjados todavía).
- Pendiente: huecos en forjado (escaleras, patinillos), forjados inclinados, y recorte con muros (fase 2).

### Edición con el ratón

- Solo se arrastra lo seleccionado (como ArchiCAD); arrastrar en vacío dibuja una ventana de selección. Cada arrastre es un paso de deshacer, con los mismos forzados que al dibujar (sin forzar a lo que se mueve), Mayús para ángulos de 45° y la distancia tecleada + Intro.
- **Mover** (arrastrar el cuerpo de lo seleccionado): los muros no seleccionados unidos por sus extremos se estiran para que la esquina siga cerrada, y los que acometen a una cara de un muro movido se alargan o acortan por su eje para seguir contra esa cara (como Revit). Las puertas y ventanas van con su muro; seleccionadas solas, se deslizan a lo largo de él.
- **Copiar** (Ctrl al soltar): copia muros (con sus puertas y ventanas), forjados y huecos; quedan seleccionadas las copias. Los `GlobalId` son nuevos.
- **Estirar** (arrastrar un extremo de muro): mueve ese extremo y los de los muros unidos en esa esquina; con Ctrl, solo el muro seleccionado (sale de la esquina). Las puertas y ventanas se quedan donde estaban en planta.
- **Vértices de forjado**: arrastrar una esquina la mueve (soltada sobre la esquina vecina, la quita); arrastrar el rombo del centro de una arista la desplaza paralela a sí misma; con Ctrl, añade una esquina.
- Lo que no cabe se rechaza entero, explicando el motivo (por ejemplo, una puerta que quedaría fuera de un tabique que se acorta).

### Cotas (anotaciones de planta)

- **Cota lineal o encadenada**: puntos medidos, dirección (horizontal, vertical o alineada) y posición de la línea. Se dibujan como es habitual en España (UNE-EN ISO 129-1): líneas finas, trazo oblicuo a 45°, línea de cota que sobrepasa los trazos extremos, valor encima de la línea legible desde abajo o desde la derecha, en metros con dos decimales (`3,45`).
- **Cota automática de huecos**: un clic en un muro acota esa cara con los extremos (esquinas a inglete incluidas) y las jambas de sus puertas y ventanas.
- **Cota de nivel**: triángulo semirrelleno con la cota del suelo terminado respecto al ±0,00 (`+0,35`, `±0,00`), leída del forjado más alto de la planta en ese punto; se actualiza sola.
- El texto es una **rotulación técnica de trazo** propia (estilo UNE-EN ISO 3098): las mismas líneas en pantalla, en el SVG y en el futuro PDF, sin depender de las fuentes del ordenador.
- Son anotaciones: no tienen `GlobalId` ni se exportan a IFC (tampoco lo hacen Revit ni ArchiCAD). Se guardan en el `.archisim` (formato versión 2).
- Pendiente (fase 3): cotas asociativas que siguen a los muros, cotas de altura en sección, de superficie, estilos por escala.

### Plantas

- Diálogo **Plantas** (botón junto al selector de planta o *Vista → Plantas…*), como los ajustes de plantas de ArchiCAD: nombre, cota, altura hasta la planta de encima y contenido de cada una; añadir planta encima (con la misma altura de planta que la última) o debajo; borrar. Cambiar la cota de una planta mueve todo lo que contiene; borrarla borra su contenido, previa confirmación. La última planta no se puede borrar.

### Tipos y materiales

- Diálogo **Tipos y materiales** (*Vista → Tipos y materiales…*, botón en el cuadro de información, o **✎ Editar tipo…** en las Propiedades de un muro, forjado, puerta o ventana, como «Editar tipo» de Revit). Pestañas: muros, forjados, puertas, ventanas y materiales.
- Como en Revit, los tipos nuevos se crean **duplicando** uno existente (nombre «… (copia)», `GlobalId` nuevo) y se modifican en el sitio. Los nombres son únicos dentro de cada lista (materiales, tipos de muro, de forjado, de puerta y ventana): el IFC y la importación identifican los materiales por su nombre.
- **Multicapas** de muro (de exterior a interior) y de forjado (de cara superior a inferior): material, espesor y función de cada capa (estructura, soporte, aislamiento, cámara de aire, lámina, acabado), subir y bajar, duplicar y quitar capas; nombre y función del tipo. Vista previa: el corte en planta de un trozo de muro de ese tipo a 1:20, dibujado por el mismo motor que la planta; en forjados, las mismas capas como banda en sección.
- **Puertas y ventanas**: funcionamiento (sin pasar de puerta a ventana), hueco de obra, anchura vista y fondo del marco, material del marco y de la hoja o vidrio. Vista previa en planta dentro de un muro del proyecto.
- **Materiales**: nombre, categoría IFC (`IfcMaterial.Category`; los vidrios se ven transparentes), color y trama de corte. Se ve en qué tipos se usa.
- Cada cambio es **un paso de deshacer** y se aplica al momento a **todos los elementos del tipo**, que conservan su `GlobalId`: los muros mantienen su línea de referencia y las puertas y ventanas su centro y antepecho. Se rechaza entero lo que no cabe (un muro más fino que el marco de una de sus puertas, una ventana que crece hasta salirse del muro o pisar otra), con el motivo.
- Solo se borra lo que no se usa (dice cuántos elementos o qué tipos lo usan) y siempre queda al menos un tipo de muro, de forjado, de puerta y de ventana.
- Sin cambios en el formato `.archisim` ni en el IFC: los tipos y materiales ya se guardaban y exportaban enteros.

### Puertas y ventanas

- Un **tipo** de puerta o ventana fija el funcionamiento (puerta abatible de una o dos hojas o corredera; ventana fija, practicable de una o dos hojas o corredera), el **hueco de obra** (ancho × alto), el perfil del marco (anchura vista y fondo) y dos materiales: marco y hoja/vidrio.
- Cada **ejemplar** vive en un muro: posición del centro medida sobre la línea de referencia desde el inicio del muro (como Revit), altura de antepecho desde el nivel, lado hacia el que abre (cara interior o exterior) y mano según UNE-EN 12519 (mirando desde el lado hacia el que abre la hoja, bisagras a izquierda o derecha). El retranqueo de la carpintería se mide desde la cara exterior del muro; si no se indica, se centra.
- Se rechaza lo que no cabe: fuera del largo o de la altura del muro, solapado con otro hueco del mismo muro, o con un marco más profundo que el muro. Borrar el muro borra sus huecos; acortarlo de forma que un hueco se salga se rechaza; invertirlo deja los huecos en su sitio.
- En planta, los huecos que corta el plano de corte interrumpen el muro y todas sus capas; las mochetas se cierran con línea de corte. En fachadas de doble hoja, la hoja exterior **vuelve** en la mocheta cerrando la cámara (el aislamiento se detiene a un espesor de ladrillo de la jamba) y el revestimiento exterior (monocapa) **envuelve** la mocheta hasta la carpintería; tabiques y muros sin cámara se cortan rectos. Puertas: cerco, hoja abierta a 90° y arco de giro (correderas: hoja sobre la cara y flecha). Ventanas: marco, línea de vidrio, montante central en las de dos hojas, giro de las hojas practicables (línea fina y arco), hojas desfasadas en las correderas, vierteaguas al exterior y borde del antepecho al interior. Las ventanas que quedan enteras por encima del plano de corte se dibujan discontinuas sin interrumpir el muro.
- En IFC el hueco atraviesa el muro (`IfcRelVoidsElement`) y la puerta o ventana lo rellena (`IfcRelFillsElement`). Su geometría 3D es sencilla (marco, hojas y vidrios cerrados) y lleva `Pset_DoorCommon` / `Pset_WindowCommon` y cantidades. El muro añade superficie lateral y volumen netos.
- Pendiente: premarcos, vuelta del revestimiento interior, persianas y capialzados, dinteles, huecos sin carpintería, puertas y ventanas paramétricas en la librería de familias (fase 2).

### Importación IFC

- Lee IFC2X3 e IFC4 (y sus variantes, también IFC 4.3) de cualquier programa, en metros, milímetros, pies o pulgadas.
- Trae plantas, muros (por su eje o, si no lo tienen, por su colocación y su sólido), sus capas y su línea de referencia (de `IfcMaterialLayerSetUsage`), forjados (por el contorno de su extrusión o por la cara superior de su sólido), y puertas y ventanas alojadas en muros (hueco de obra desde el `IfcOpeningElement`, funcionamiento y mano, marco desde las propiedades de cerco). Se conservan todos los `GlobalId`.
- La cara exterior de un muro en IFC no está definida: en los muros marcados como exteriores (`IsExternal`), se toma como exterior el lado opuesto al centro de los muros de su planta, y el muro se gira si hace falta para que quede a la izquierda, como en ArchiSim.
- Lo que no se importa (pilares, vigas, escaleras, cubiertas inclinadas, muros curvos, huecos sin carpintería…) se cuenta en un **informe**; lo que se importa con aproximaciones (capas sin espesor, marcos sin medidas, huecos en forjados), también.
- Lo que exporta ArchiSim se importa idéntico (ida y vuelta comprobada en pruebas): para ello el IFC lleva, además, el cerco de puertas y ventanas (`IfcDoorLiningProperties`, `IfcWindowLiningProperties`), el funcionamiento de cada hoja de ventana (`IfcWindowPanelProperties`), la mano de cada hueco (`ArchiSim_Opening`) y la trama de corte de cada material (`ArchiSim_Material`).

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
| 8 | Ventana de la aplicación: vista de planta, herramienta muro, selección | ✅ v0.5 |
| 9 | Vista 3D | ✅ v0.7 |
| 10 | Importación IFC básica | ✅ v0.8 |

## 6. Fases siguientes

La lista detallada, con tamaños, dependencias y decisiones pendientes, está en `docs/HOJA_DE_RUTA.md`.


- **Fase 2:** sistema de familias paramétricas (recetas de texto propias, diseño en `docs/FAMILIAS.md`), librería inicial de puertas, ventanas, mobiliario y sanitarios; cubiertas, escaleras, secciones y alzados con líneas ocultas; recorte de muros con forjados.
- **Fase 3:** planos y maquetación, acotación, etiquetas, tablas de planificación, impresión a PDF.
- **Fase 4:** complemento puente para Revit, BCF, colaboración (modelo central), disciplinas de ingeniería.

## 7. Forma de trabajo

- **Antonio:** experto BIM y dueño del producto. Prueba cada versión, abre los IFC en ArchiCAD y Revit, y da feedback.
- **Claude (Claude Code):** escribe todo el código, las pruebas y la documentación técnica. Ver `CLAUDE.md`.
- Cada cambio llega con pruebas automáticas. Cada exportación IFC se valida con IfcOpenShell antes de darla por buena.
- Feedback de Antonio → issue en GitHub con captura y, si se puede, el IFC o SVG afectado.

## 8. Licencia

Pendiente de decidir. El repositorio será privado hasta entonces. Candidatas para abrirlo: **MPL-2.0** (permite usarlo en productos cerrados pero obliga a publicar mejoras del propio código) o **GPL-3.0** (todo derivado debe ser libre). No se añade fichero de licencia hasta decidir.
