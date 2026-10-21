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
├──────────────┬───────────────┬──────────────┬────────────────┤
│ plan         │ ifc           │ export       │ file, library  │
│ plantas,     │ import/export │ PDF, DXF,    │ proyecto,      │
│ planos,tablas│               │ BC3          │ plantilla      │
├──────────────┴───────────────┴──────────────┴────────────────┤
│ draft — dibujo 2D: órdenes de edición, forzados, rotulación  │
├──────────────────────────────────────────────────────────────┤
│ geom — geometría: polígonos, uniones de muros, extrusiones    │
├──────────────────────────────────────────────────────────────┤
│ core — documento, elementos, tipos, materiales, transacciones │
└──────────────────────────────────────────────────────────────┘
```

Crates actuales (`crates/`):

| Crate | Responsabilidad |
|---|---|
| `archisim-family` | El lenguaje de las familias (recetas `.asfam`): lector, fórmulas, condiciones, repeticiones, piezas comunes y evaluación (geometría 3D y salidas). No sabe nada del modelo; ver `docs/FAMILIAS.md` |
| `archisim-dxf` | Lector de DXF de texto y del levantamiento topográfico que contiene (puntos con cota, curvas de nivel, cotas escritas, por capas), y de listas de puntos CSV/TXT. Sin dependencias |
| `archisim-core` | Documento del proyecto, niveles, materiales, tipos de muro y de forjado multicapa, muros, forjados, familias (copia de la receta en el proyecto y las cinco familias de sistema), tipos de puerta y ventana como tipos de familia con marca, biblioteca de tipos entre proyectos (cargar por `GlobalId`, actualizar familias, purgar), huecos en muro, deshacer/rehacer; edición de materiales y tipos con comprobación de los elementos que los usan |
| `archisim-geom` | Vectores 2D, contorno de muros con uniones en esquina a inglete y uniones en T con prioridad por capa (`joins`), área de polígonos, posición de huecos y recorte de muros en tramos; recintos cerrados por muros (varita mágica); encuentros de muros y forjados por prioridad de capa (`junctions`, con las operaciones booleanas de polígonos de `i_overlay`); sólidos 3D de muros (por capas, con sus huecos), forjados y carpinterías (el cuerpo 3D de su familia), los mismos que exporta el IFC |
| `archisim-draft` | Geometría del dibujo 2D al estilo AutoCAD (fase 3): segmentos y arcos, intersecciones, equidistancia, recortar, alargar, empalme, chaflán, partir, juntar, descomponer, estirar, matrices, recintos para sombrear, tramas, forzados a objeto, pinzamientos, cotas angulares, de radio y de arco, y la rotulación técnica de trazo (`font`). Solo depende de `core` |
| `archisim-export` | Salidas de dibujo (fase 3): **PDF vectorial** de plantas y planos (a su tamaño de papel, sin fuentes), **DXF** (AutoCAD 2007) de una planta en mm o de un plano en mm de papel, con capas, colores, grosores y tipos de línea, y el dibujo 2D nativo (líneas, polilíneas con arcos, textos, sombreados con su patrón); **importar DXF** como dibujo 2D (bloques y cotas descompuestos); **mediciones BC3** (FIEBDC-3). Lee el mismo SVG de `archisim-plan` con usvg |
| `archisim-ifc` | Escritor IFC 4.3 (por defecto, D3) e IFC4 (STEP), con georreferencia (`IfcMapConversion`) con jerarquía espacial, tipos, capas de material y Psets de muros y forjados; huecos (`IfcOpeningElement`) rellenos con puertas y ventanas. Lector STEP e importador IFC2X3/IFC4 de plantas, muros con capas, forjados, puertas y ventanas, con informe de lo que queda fuera |
| `archisim-plan` | Planta en SVG a cualquier escala: poché por capa, forjados según plano de corte, muro interrumpido en huecos, símbolos de puerta y ventana (los de su familia para la escala), ejes, rotulación; cotas y cotas de nivel con rotulación técnica de trazo propia (`lettering`); dibujo 2D por capas (`drafting`), rótulos de locales y etiquetas (`rooms`), plumillas por escala y visibilidad por vista; **planos** con vistas a escala recortadas, títulos y cajetín (`sheet`); **tablas de planificación** y sus mediciones (`schedules`) |
| `archisim-file` | Proyecto `.archisim`: JSON versionado que conserva ids y `GlobalId`, se comprueba al abrir y se guarda de forma segura (ver `docs/FORMATO_ARCHISIM.md`) |
| `archisim-library` | Plantilla de inicio: materiales y tipos de muro, forjado, puerta y ventana de construcción española habitual, con los que empieza todo proyecto nuevo (y la vivienda de prueba) |
| `archisim-ids` | Requisitos de información IDS 1.0 de buildingSMART: lectura y comprobación de cualquier IFC2X3/4/4.3 (pasa los 334 casos oficiales) |
| `archisim-bcf` | Incidencias BCF 2.1 y 3.0: temas, comentarios y puntos de vista (cámara, selección, colores), lectura y escritura |
| `archisim-coord` | Coordinación: cualquier IFC como elementos con su malla, huecos y huella de datos; versiones por `GlobalId`; interferencias; incidencias desde interferencias e IDS (`docs/COLABORACION.md`) |
| `archisim-cte` | Comprobación del Código Técnico sobre el modelo (`docs/CTE.md`): ocupación, salidas, recorridos y anchuras de evacuación (DB-SI 3), accesibilidad (DB-SUA 9), escaleras y rampas (DB-SUA 1) y superficies útil y construida |
| `archisim-cli` | Línea de comandos: `demo` genera una vivienda de prueba, la guarda y la exporta (IFC, plantas SVG y DXF, planos SVG, DXF y PDF, tablas CSV, mediciones BC3); `exportar` abre un `.archisim` y lo exporta igual; `importar` convierte un IFC en proyecto; `png` rasteriza un SVG para verlo |
| `archisim-app` | La aplicación de escritorio (egui sobre wgpu), con la interfaz descrita en «Interfaz» (§4): vista de planta y vista 3D en pestañas, herramientas muro, forjado (por polígono o varita mágica), puerta, ventana y cota, gestión de plantas, tipos y materiales (multicapas, puertas, ventanas y materiales, con vista previa), selección y propiedades, mover, copiar y estirar con el ratón, deshacer, abrir/guardar `.archisim` (también soltando el fichero en la ventana o, en Mac, con doble clic en el Finder), importar IFC, exportar IFC, planta e imagen 3D |

Reglas de dependencias: `family` (el lenguaje), `dxf` (lectura de DXF) y `bcf` (incidencias) no dependen de nadie; `ids` solo de `ifc` (el lector STEP); `coord` de `core`, `geom`, `ifc`, `ids` y `bcf`; `core` solo de `family`; `geom` (y `i_overlay`, MIT, para unir y restar polígonos) y `draft` solo de `core`; `plan` de `core`, `geom` y `draft`; `cte` (Código Técnico) de `core` y `geom`; `ifc` de `core` y `geom`; `export` de `core`, `draft` y `plan`; `file` (formato de proyecto) y `library` (plantilla) solo de `core`; la interfaz depende de todo y nada depende de la interfaz.

La vista de planta de la aplicación **es** la planta 1:50 que se exporta: se dibuja el mismo SVG de la zona visible y se rasteriza a la resolución de pantalla (grosores de línea reales a 1:50, como la vista de «grosor de línea real» de ArchiCAD). Lo que se ve es lo que se exporta. Encima, en vectorial, van la selección, la vista previa del muro y los puntos de forzado.

Cada planta se ve a la escala de su pestaña (1:20, 1:50, 1:100 o 1:200): grosores, tramas, textos y símbolos de puertas y ventanas son los de esa escala, como en ArchiCAD.

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
| `OpeningType` (familia + valores + marca) | Tipo de familia de puerta / ventana | Objeto puerta / ventana con sus ajustes | `IfcDoorType` / `IfcWindowType` + `IfcMaterialConstituentSet` |
| `Opening` | Ejemplar de puerta / ventana (alojado en muro) | Puerta / ventana | `IfcOpeningElement` (vacía el muro) + `IfcDoor` / `IfcWindow` (lo rellena) |
| `Family` (receta `.asfam`, copia en el proyecto; `docs/FAMILIAS.md`) | Familia .rfa | Objeto GDL | `IfcDoorType`/`IfcWindowType` (`ElementType` = familia, `Tag` = marca) con `ArchiSim_Family` (receta) y `ArchiSim_FamilyValues` (parámetros) |
| Parámetro compartido | Parámetro compartido | Propiedad | `IfcPropertySet` |
| `DrawLayer` | Subcategoría / estilo de línea | Capa | — (capa del DXF) |
| `Draft` (línea, polilínea, arco, texto, sombreado, cota angular…) | Líneas de detalle, texto, región rellena | Elementos 2D | — (entidades del DXF) |
| `Dimension` con `anchors` | Cota (asociativa) | Cota asociativa | — (anotación) |
| `Room` | Habitación | Zona | `IfcSpace` (`Pset_SpaceCommon`, `Pset_SpaceCoveringRequirements`, `Qto_SpaceBaseQuantities`) |
| `Tag` | Etiqueta | Marcador / etiqueta | — (anotación) |
| `Sheet` + `Viewport` | Plano + vista colocada | Plano del Libro de planos + dibujo | — (PDF, DXF) |
| `SheetTable` | Tabla de planificación | Listado interactivo | — (CSV, BC3) |

Cada elemento tiene un `ElementId` interno y un `GlobalId` IFC de 22 caracteres que no cambia nunca, ni al copiar el proyecto ni al exportar varias veces. Así los ingenieros pueden federar modelos y comparar versiones.

### Interfaz

Como ArchiCAD en la disposición, con las propiedades como Revit (v0.16):

- **Barra superior**: menús (Archivo, Edición, Vista, Ayuda), nombre del proyecto y fichero («sin guardar» si hay cambios), deshacer y rehacer (con lo que deshacen en la ayuda), guardar, **Exportar IFC**, mostrar u ocultar las propiedades y el navegador, y tema claro u oscuro. El dibujo es siempre papel blanco: lo que se ve es lo que se imprime.
- **Disciplinas y herramientas** (izquierda, desde el bloque I1 de `docs/INTERFAZ.md`): una barra estrecha con la **Flecha** arriba y las ocho disciplinas (Concepto, Arquitectura, Estructura, Instalaciones, Urbanismo, Documentación, Análisis, Coordinación); a su lado, las herramientas de la disciplina elegida en fichas de **tamaño fijo**, cuatro por fila y siempre tres filas, con su dibujo de plano, su nombre y, al pasar el ratón, su tecla. Las que aún no existen se ven apagadas y dicen qué las trae. Las teclas valen desde cualquier disciplina y llevan a la de su herramienta (I, pilar → Estructura). El **buscador** (Ctrl+Espacio o el campo de la barra superior) encuentra cualquier herramienta u orden por su nombre o su tecla, también las de dibujo y edición 2D de AutoCAD (línea, polilínea, sombreado, desplazar, recortar…), que ya no tienen ficha.
- **Línea de órdenes** (fase 3, como AutoCAD; **oculta por defecto** desde I2: lo que se escribe sale junto al cursor y el paso de la orden en la barra contextual; se deja fija en Vista → Línea de órdenes fija): se escribe la orden o su alias en español o en inglés (`L`/`LINEA`, `RR`/`TRIM`, `EQ`/`O`…), con sus preguntas, opciones clicables, coordenadas tecleadas (`3000,2000`, `@1500,0`, `@2000<45`, distancia directa), Intro o Espacio para repetir y Esc para terminar. Vista → Teclas de AutoCAD elige si las letras sueltas lanzan órdenes de AutoCAD o herramientas de ArchiSim; F8, orto.
- **Pestañas de vistas** sobre el dibujo: cada planta y la vista 3D en su pestaña. Cada pestaña de planta recuerda su encuadre, su escala y su plano de corte. «+» abre una vista; × o clic con la rueda la cierra (la última no); Ctrl+Tab pasa a la siguiente; F2 vuelve a la última planta y F3 abre el 3D. Una planta añadida se abre en su propia pestaña; una planta borrada cierra la suya.
- **Navegador** (a la derecha), como el de ArchiCAD o el Navegador de proyectos de Revit: **Vistas** (plantas con su cota, 3D; secciones y alzados cuando existan), **Planos** (cada plano abre su pestaña; «+ Nuevo plano»), **Tablas** (carpinterías, muros, forjados, cuadro de superficies) y **Modelo**: cada categoría (muros, forjados, puertas, ventanas) con sus tipos y el número de elementos de cada uno en el proyecto (el tipo es la unidad de medición). Clic en un tipo: selecciona sus elementos en la planta; doble clic o menú contextual: «Editar tipo». Materiales y «Tipos y materiales…» abren el diálogo.
- **Propiedades** (a la izquierda, debajo de las herramientas, como la paleta de Revit: elegir herramienta y tipo es un movimiento corto del ratón): arriba el **selector de tipo** con su **dibujo a 1:20** (corte en planta del multicapa, capas del forjado, o la puerta o ventana en planta dentro de su muro), categoría y clase IFC, y el tipo, que se elige en una **galería** de fichas con su dibujo y un buscador; «✎ Editar tipo» y «Seleccionar los N» del mismo tipo en la planta; debajo los parámetros por grupos plegables: **Restricciones**, **Construcción**, **Dimensiones**, **Capas** y **Datos de identidad**. Con una herramienta activa muestra lo que se va a dibujar (sustituye al cuadro de información de ArchiCAD); sin nada seleccionado, las propiedades de la vista (escala, plano de corte, líneas de referencia, rejilla; nombre y cota de la planta) y del proyecto.
- **Barra contextual** (bajo las pestañas, fija, desde I2): con una herramienta, sus **métodos de inserción** (recta, cadena, rectángulo, polígono, arco, círculo, varita, **spline** —curva libre: muros curvos, bordes de forjados, locales, barandillas, viales— y **elegir línea** —seguir una línea, polilínea, arco o círculo del dibujo 2D o de un DXF—; los que no valen, apagados) y, mientras se dibuja, los campos **L** y **∠** para teclear el siguiente punto; con algo seleccionado, **Modificar · Muro (1)** con Transformar (mover, copiar, girar, simetría, escala, matriz) y Geometría (desfase, alinear, recortar, alargar, partir, unir); con una orden en marcha, su paso y sus opciones.
- **Barra inferior** por segmentos: escala y zoom; **Línea**, **Relleno**, **Texto** y **Cota** (clic: su orden; flecha: las demás formas y sus ajustes de capa, grosor, tipo de línea, trama, altura de texto o tipo de cota); **forzados** como dibujos (extremos, medios, aristas, rastreo y orto); a la derecha, el punto forzado, las coordenadas y la planta. Rejilla, líneas de referencia, líneas finas, calco y fases están en las propiedades de la vista y en Vista. En 3D: perspectiva o axonometría, ajustar y guardar imagen.
- **Disciplina por elemento y atenuación** (desde I3): cada elemento es de Arquitectura, Estructura, Instalaciones o Urbanismo (pilares y vigas, estructura; muros y forjados con capa estructural de hormigón, acero o madera, estructura; terreno, vegetación y vehículos, urbanismo; lo demás, arquitectura), y se cambia a mano en Propiedades → Disciplina. Al trabajar en Estructura, Instalaciones o Urbanismo lo demás se ve en gris, y en Concepto todo; en planta, sección, 3D y en los planos (Visibilidad → Atenuación), también al imprimir. Vista → Atenuar por disciplina lo apaga. En el navegador (Modelo → Disciplinas), un ojo oculta cada disciplina en todas las vistas y un candado impide seleccionarla.
- **Ejes y cimentación** (desde el bloque de ejes y cimentación): ejes de replanteo por dos clics, nombrados solos (1, 2… los que suben; A, B… los que cruzan), en todas las plantas y en las secciones que los cortan; zapatas aisladas bajo pilares y corridas bajo muros que los siguen, vigas de atado y «Cimentar planta»; la losa de cimentación es un forjado de su tipo. En el IFC, `IfcGrid` e `IfcFooting`.
- **Concepto** (desde I4): caras, volúmenes y masas por plantas al estilo SketchUp (línea, rectángulo, círculo, empujar, seguir, equidistancia, cinta, pintar), con huella, superficie construida, edificabilidad y ocupación sobre la superficie de la parcela, y **Convertir** a muros, forjados y cubierta, recordando la relación para rehacerlo si el volumen cambia. Los volúmenes no van al IFC.
- **Urbanismo**: parcelas por su linde con retranqueo y referencia catastral (de ellas sale la superficie para la edificabilidad de las masas) y viales por su eje con una sección tipo (calzada, aparcamiento, bordillo y acera). En el IFC, `IfcSite` por parcela e `IfcRoad` con `IfcPavement` e `IfcKerb` (IFC 4.3).
- **Módulos** (desde I5): otro proyecto (una vivienda tipo) se coloca varias veces con giro y simetría (Coordinación → Módulo, tecla K). El proyecto guarda una copia del módulo y avisa en el navegador cuando su fichero cambia, para actualizar todas las copias a la vez. Vistas, tablas, mediciones e IFC cuentan las copias como elementos, con identificadores propios y estables por copia.
- **Tablas y mediciones en pestañas** (desde I2): cada tabla del navegador y **Mediciones** (todas las de cantidades seguidas, con CSV y BC3) se abren como una vista más.
- **Arrastrar al plano** (desde I2): plantas, secciones, alzados, tablas y la vista 3D se arrastran desde el navegador hasta un plano abierto y quedan donde se sueltan, a la escala de su pestaña. La vista 3D se coloca como **axonometría** desde donde se mira, a la escala que quepa en el papel, dibujada como vectores (sale igual en PDF y DXF).
- **Sobre el dibujo**: las teclas del paso siguiente en una pastilla abajo («Clic siguiente punto · Longitud + Intro exacto · Mayús 45° · Intro/Esc terminar») y el resultado de cada acción como un aviso arriba a la izquierda que se desvanece (los errores duran más). Junto al cursor, longitud, ángulo y superficie.
- Tipografía IBM Plex (licencia SIL OFL, incluida en `crates/archisim-app/assets/fonts`) e iconos dibujados como líneas por el propio programa (sin ficheros de imagen), nítidos a cualquier resolución.

### Muros

- Definidos por un eje entre dos puntos, nivel base, altura y tipo. El eje puede ser **recto o un arco de circunferencia** (v0.19): el arco se guarda por su comba (`bulge`, como las polilíneas DXF: `tan(θ/4)` del ángulo que barre; 1 = media circunferencia), que no cambia al mover, girar o escalar el muro, así que estirar un extremo conserva la forma.
- **Métodos de la herramienta Muro** (v0.19, como el «método geométrico» de ArchiCAD), en Propiedades: **Recto** (un muro por cada dos clics), **Cadena** (por defecto), **Rectángulo** (dos esquinas opuestas, o la primera y `6000x4000` + Intro; cuatro muros en sentido horario, con la cara exterior fuera), **Arco** (inicio, fin y un punto por el que pasa, como «inicio-fin-radio» de Revit; o el radio tecleado, hacia el lado del cursor; se encadenan) y **Círculo** (centro y un punto de la línea de referencia, o el radio tecleado; dos medias circunferencias en sentido horario). Un rectángulo o un círculo es un solo paso de deshacer.
- **Muros curvos**: en planta, sus capas, tramas (también el zigzag del aislamiento) y caras siguen el arco; se unen a inglete con muros rectos o curvos que comparten extremo (si continúan tangentes, el corte es radial); un muro que acomete a un muro curvo, o un muro curvo que acaba dentro de otro, se detiene en su cara, sin entrar por capas. Fuerzan su centro, sus extremos, las esquinas de sus extremos, el punto medio del arco y de cada cara, y cualquier punto de sus caras. Seleccionados, un **círculo blanco** en su punto medio los curva arrastrándolo (en línea con los extremos, quedan rectos); en Propiedades, **Radio** (editable, los extremos no se mueven), ángulo y **Enderezar**. Se rechaza un arco más cerrado que el espesor del muro. Aún no admiten puertas ni ventanas, ni cotas temporales.
- En IFC el eje de un muro curvo es un `IfcTrimmedCurve` sobre `IfcCircle` y su cuerpo una extrusión de un `IfcIndexedPolyCurve` con arcos exactos (`IfcArcIndex`); longitud, superficie y volumen son los del arco exacto.
- La **línea de referencia** puede ir en el centro, en la cara exterior o en la interior (como en ArchiCAD y Revit).
- Las capas se ordenan de **exterior a interior**; la cara exterior queda a la izquierda del eje, mirando de inicio a fin.
- Las uniones en esquina se resuelven a inglete cuando dos muros comparten un extremo.
- **Uniones en T con prioridad de intersección por capa** (como ArchiCAD): un muro cuyo extremo cae dentro de otro (en su cara, en su eje o en su espesor), lejos de sus esquinas, acomete a él. Cada capa del muro que acomete entra en el otro atravesando sus capas de menor prioridad, desde la cara por la que llega, y se detiene en la primera de prioridad igual o mayor: la estructura atraviesa, los acabados se detienen. Prioridades por función de capa: estructura 4 > soporte (hoja interior) 3 > aislamiento 2 > cámara y lámina 1 > acabado 0. Así, la hoja de LHD de un tabique atraviesa el enlucido de la fachada hasta su hoja interior; un muro de carga atraviesa la hoja interior y el aislamiento hasta la hoja exterior. Nunca se atraviesa la última capa del otro muro, y solo se entra si el muro que acomete cabe en la altura del otro y está lejos de sus esquinas; si no, se detiene en su cara.
- El muro que recibe pierde esa parte de sus capas solo en la altura del que acomete: en planta, en 3D (una sola piel cerrada por muro) y en el IFC (una extrusión por tramo de altura; el volumen neto sigue coincidiendo). En planta la cara del muro que recibe no se corta donde acomete el otro: una línea fina marca dónde se detienen sus capas.
- **Restricción superior** (v0.17, como la «restricción superior» de Revit o la planta de enlace de ArchiCAD): un muro tiene **altura desconectada** (propia) o llega **hasta una planta** con un **desfase superior** (negativo: por debajo, por ejemplo el canto del forjado). Con planta superior, la altura la calcula el modelo (cota de la planta superior + desfase − cota de la planta base − desfase de base) y **cambia sola** al cambiar la cota de cualquiera de las dos plantas o el desfase de base. Se rechaza entero lo que no cabe: una cota que dejaría la coronación en la base o por debajo, o una ventana fuera del muro. Escribir una altura desconecta el muro; desconectarlo conserva su altura; borrar la planta superior lo desconecta con la altura que tenía. La herramienta Muro dibuja por defecto **hasta la planta superior** (la de encima de la que se dibuja, como «planta inicial + 1» de ArchiCAD); si no hay ninguna, con su altura propia. El IFC no cambia: lleva la altura resultante, como Revit.
- Pendiente: recorte de muros con forjados y cubiertas, prioridades editables por material, uniones en L por capas y cruces en X (fase 2).

### Forjados

- Definidos por un contorno poligonal cerrado (cualquier forma sin cruces: rectángulo, L, U…), un nivel, un tipo y un desfase.
- **Métodos de la herramienta Forjado** (v0.19), en Propiedades: **Polígono** (esquina a esquina), **Rectángulo** (dos esquinas opuestas, o la primera y `6000x4000` + Intro) y **Varita mágica**.
- La referencia es la **cara superior**, como en ArchiCAD y Revit: queda a la cota del nivel más el desfase, y las capas cuelgan hacia abajo.
- Las capas del tipo se ordenan de **arriba abajo** (pavimento → estructura → acabado inferior).
- Función del tipo: forjado de planta (`FLOOR`), solera (`BASESLAB`) o cubierta plana (`ROOF`). Solera y cubierta se marcan como exteriores (`IsExternal`).
- En planta se dibujan según el plano de corte (1,10 m por defecto): por debajo, con trama de cubrición y línea fina de proyección; cortados, con poché de su capa estructural y línea gruesa; por encima, discontinuos.
- **Varita mágica**: un clic dentro de un recinto de muros crea el forjado con su contorno, hasta la cara interior de los muros, hasta su eje, o por la cara exterior de todo el edificio. Los muros acometidos a la cara de otro (tabiques) cierran recintos; un muro suelto dentro de un recinto se ignora (no hay huecos en forjados todavía).
- **Huecos de forjado** (v0.20: escaleras, patinillos): contornos cerrados dentro del forjado, sin cruzar su borde ni otros huecos. Se dibujan con la herramienta Forjado marcando «Hueco en un forjado» (polígono o rectángulo), y se recortan del forjado de la planta que los contiene; en Propiedades se ven con su superficie y se quitan. La superficie del forjado descuenta los huecos; al mover o copiar el forjado van con él; un clic en el hueco no lo selecciona. En planta, el hueco queda vacío con su aspa (discontinua si el forjado está por encima del plano de corte); en 3D lo atraviesa con sus caras; en IFC son los huecos del perfil (`IfcArbitraryProfileDefWithVoids`), con superficie bruta y neta.
- Pendiente: forjados inclinados y recorte con muros (fase 2), editar los vértices de un hueco (hoy se quita y se vuelve a dibujar).

### Edición con el ratón

- Solo se arrastra lo seleccionado (como ArchiCAD); arrastrar en vacío dibuja una ventana de selección. Cada arrastre es un paso de deshacer, con los mismos forzados que al dibujar (sin forzar a lo que se mueve), Mayús para ángulos de 45° y la distancia tecleada + Intro.
- **Mover** (arrastrar el cuerpo de lo seleccionado): los muros no seleccionados unidos por sus extremos se estiran para que la esquina siga cerrada, y los que acometen a una cara de un muro movido se alargan o acortan por su eje para seguir contra esa cara (como Revit). Las puertas y ventanas van con su muro; seleccionadas solas, se deslizan a lo largo de él.
- **Copiar** (Ctrl al soltar): copia muros (con sus puertas y ventanas), forjados y huecos; quedan seleccionadas las copias. Los `GlobalId` son nuevos.
- **Estirar** (arrastrar un extremo de muro): mueve ese extremo y los de los muros unidos en esa esquina; con Ctrl, solo el muro seleccionado (sale de la esquina). Las puertas y ventanas se quedan donde estaban en planta.
- **Vértices de forjado**: arrastrar una esquina la mueve (soltada sobre la esquina vecina, la quita); arrastrar el rombo del centro de una arista la desplaza paralela a sí misma; con Ctrl, añade una esquina.
- Lo que no cabe se rechaza entero, explicando el motivo (por ejemplo, una puerta que quedaría fuera de un tabique que se acorta).
- **Cotas temporales** (como Revit): con un muro o una puerta o ventana seleccionados (también el último muro dibujado y la puerta o ventana recién colocada) aparecen cotas azules a las referencias más cercanas. Puerta o ventana: de cada jamba (hueco de obra) a la referencia más próxima de ese lado, medida en la cara hacia la que abre: la cara de un muro que la cruza o acomete, el final del muro o la jamba del hueco vecino. Muro: la longitud de su línea de referencia y la luz libre de cara a cara hasta el muro paralelo más próximo a cada lado. Un clic en el valor permite escribirlo (mm, o con unidad) e Intro recoloca el elemento para que la cota sea exacta: la puerta se desliza por el muro; el muro se desplaza (los unidos le siguen, como al moverlo) o se alarga desde su final. Es un paso de deshacer; Esc deja la cota como estaba.
- **Girar puertas y ventanas**: Espacio con una seleccionada recorre las cuatro posiciones (bisagras a la otra jamba, hoja al otro lado…); dos flechas azules junto a ella, en el lado contrario a la apertura, la pasan al otro lado del muro (bisagras en la misma jamba; la mano UNE cambia de nombre porque se lee desde el otro lado) o cambian la mano. Con la herramienta Puerta o Ventana, Espacio cambia la mano de la que se va a colocar. Con muros seleccionados, Espacio los invierte.

### Ayudas al dibujo

- **Forzados**: extremos y puntos medios de las líneas de referencia; esquinas y **puntos medios de cada cara de muro y arista de forjado**; cualquier punto de una cara o arista; rastreo.
- **Rastreo ortogonal por defecto**: dibujando muros, forjados o cotas, o arrastrando, la línea queda horizontal o vertical mientras el cursor no se aparte más de 8° de esa dirección, y una **línea azul** discontinua lo indica; cerca de 45° también se bloquea. Si el punto cae sobre una cara, se desliza por ella hasta quedar en escuadra (un tabique dibujado hasta una cara acaba recto). Para inclinarlo basta apartar el cursor; Mayús fuerza siempre múltiplos de 45°. El rastreo se apaga en la barra inferior.

### Cotas (anotaciones de planta)

- **Cota lineal o encadenada**: puntos medidos, dirección (horizontal, vertical o alineada) y posición de la línea. Se dibujan como es habitual en España (UNE-EN ISO 129-1): líneas finas, trazo oblicuo a 45°, línea de cota que sobrepasa los trazos extremos, valor encima de la línea legible desde abajo o desde la derecha, en metros con dos decimales (`3,45`).
- **Cota automática de huecos**: un clic en un muro acota esa cara con los extremos (esquinas a inglete incluidas) y las jambas de sus puertas y ventanas.
- **Cota de nivel**: triángulo semirrelleno con la cota del suelo terminado respecto al ±0,00 (`+0,35`, `±0,00`), leída del forjado más alto de la planta en ese punto; se actualiza sola.
- El texto es una **rotulación técnica de trazo** propia (estilo UNE-EN ISO 3098): las mismas líneas en pantalla, en el SVG y en el futuro PDF, sin depender de las fuentes del ordenador.
- Son anotaciones: no tienen `GlobalId` ni se exportan a IFC (tampoco lo hacen Revit ni ArchiCAD). Se guardan en el `.archisim` (formato versión 2).
- **Asociativas** (v0.25): cada punto que cae en una esquina de muro, una jamba o una esquina de forjado queda atado a él; al mover, estirar o cambiar el elemento, la cota se rehace. Si el elemento se borra, el punto queda libre.
- **Cotas de dibujo** (v0.25): angulares (`ACOANGULO`, entre dos líneas o caras de muro, o de un arco), de radio y de diámetro (`ACORADIO`, `ACODIAMETRO`) y de longitud de arco (`ACOARCO`); miden su propia geometría, así que cambian al estirarlas o escalarlas.
- Pendiente: cotas de altura en sección (con 2.10) y estilos de cota por escala.

### Dibujo 2D, capas y estilos de vista (fase 3, v0.25)

- **Elementos 2D** en plantas y planos: líneas, polilíneas (con arcos y grosor), rectángulos, polígonos, arcos, círculos, elipses, puntos, textos de una o varias líneas (alturas en mm de papel, 9 justificaciones), sombreados por recinto o contorno con tramas propias (líneas, cuadrícula, diagonal, ladrillo, hormigón, grava, baldosa, tierra, césped, madera, sólido) y fondo, directrices y nubes de revisión. Cada uno en una **capa** (color, grosor, tipo de línea; visible, bloqueada, imprimible) o con su estilo propio.
- **Edición al estilo AutoCAD**: desplazar, copiar, girar, escalar, simetría, alinear, desfase, recortar y alargar (Mayús invierte), empalme y chaflán (también de polilíneas), matriz rectangular y polar, partir, juntar, descomponer, estirar, longitud, borrar, igualar propiedades, dividir; consultas de distancia, área y lista. Las órdenes de transformación valen también para muros, forjados, cotas y locales.
- **Forzados a objeto**: punto final, medio, centro, cuadrante, nodo, intersección, perpendicular y tangente, sobre el dibujo 2D y sobre las caras de muros y aristas de forjados.
- **Normas gráficas** del proyecto: plumillas finas (1:20, 1:50) y gruesas (desde 1:100) para corte, carpintería, vista, capas, tramas, cotas y textos. Cada vista oculta las categorías y capas que quiera y tiene su **nivel de detalle** (bajo, medio, alto).

### Locales y etiquetas (v0.25)

- **Local** (`LOCAL`, ficha «Local»): un clic dentro de los muros crea el local con el contorno de sus caras interiores, número siguiente y nombre; **sigue a los muros** cuando cambian (se rehace desde su rótulo). Con la opción Polígono se dibuja el contorno. Tiene número, nombre, uso, superficie útil, superficie construida (a ejes, en el cuadro), perímetro, altura libre (hasta la planta superior o propia), volumen y acabados de suelo, paredes y techo. Su rótulo (nombre, número y superficie) se arrastra. Se exporta e importa como `IfcSpace` con sus cantidades y acabados.
- **Etiquetas** (`ETIQUETA`): de puertas y ventanas (marca y medidas en un recuadro), de muros (tipo) y de locales; escriben lo que es su elemento y lo acompañan cuando se mueve; se borran con él. Opción «Todas» para etiquetar todas las carpinterías de la planta.

### Planos y salida (v0.25)

- **Planos** de A4 a A0, horizontales o verticales, con marco y **cajetín propio** (estudio, proyecto, promotor, emplazamiento, fase, plano, escala, fecha, autor, revisión, número), cuyos datos se escriben una vez para todo el proyecto. Cada plano en su pestaña, a escala 1:1 del papel.
- **Vistas en el plano**: una planta a su escala (1:20 a 1:500), con su plano de corte, recorte, visibilidad propia y título con escala; se arrastran y doble clic abre su planta. En el plano también se dibuja (textos, líneas) y se colocan **tablas**.
- **Exportar**: planta (a cualquier escala, con lo que muestra la vista) o plano en **PDF vectorial**, **DXF** o SVG; todos los planos en un PDF. **Importar DXF** en la vista activa (también soltándolo en la ventana): capas, colores, grosores, tipos de línea, textos, sombreados y bloques.

### Tablas y mediciones (v0.25)

- **Tablas de planificación**: carpinterías (marca, tipo, clase, medidas, unidades, plantas), muros por tipo (espesor, nº, longitud, superficie descontando huecos, volumen), forjados por tipo (superficie, volumen) y **cuadro de superficies** (nº, local, uso, planta, superficie útil y construida, perímetro, altura, volumen, acabados), de una planta o de todo el edificio, con totales. Se ven en una ventana, se exportan a **CSV** (con punto y coma y coma decimal, para Excel en español) y se colocan en los planos; se rehacen solas.
- **Mediciones en BC3** (FIEBDC-3): capítulos de fábricas y muros (m², con los huecos deducidos línea a línea), forjados (m²), carpintería (ud) y acabados de locales (m²), una partida por tipo y una línea de medición por elemento, sin precios, para Presto, Arquímedes o TCQ.

### Plantas

- Diálogo **Plantas** (botón junto al selector de planta o *Vista → Plantas…*), como los ajustes de plantas de ArchiCAD: nombre, cota, altura hasta la planta de encima y contenido de cada una; añadir planta encima (con la misma altura de planta que la última) o debajo; borrar. Cambiar la cota de una planta mueve todo lo que contiene; borrarla borra su contenido, previa confirmación. La última planta no se puede borrar.
- **Cambiar de planta**: en Propiedades, «Restricción de base» de un muro y «Planta» de un forjado (o de varios a la vez) llevan el elemento a otra planta conservando sus desfases; las puertas y ventanas van con su muro. Un muro que llegaba a la planta de encima sigue llegando a la de encima de la nueva (si no hay, queda con su altura, desconectado); unido a otra planta, la conserva, y se rechaza si no queda por encima.
- **Otras plantas en planta**: los muros de otras plantas que atraviesa el plano de corte (un muro de la planta baja que sube más allá de la primera) se dibujan cortados, también en la planta exportada, como hace Revit con su rango de vista. El **calco** (activado por defecto en pantalla, *Calco* en la barra inferior o en las propiedades de la vista) dibuja en gris azulado claro los muros de la planta inferior y la posición de sus huecos; sus puntos fuerzan, para dibujar encima. El calco no se exporta.

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

- Un **tipo** de puerta o ventana es un tipo de una **familia** (`docs/FAMILIAS.md`) con sus parámetros de tipo fijados y una **marca de tipo** única (P-01, V-03) para etiquetas y mediciones. Las cuatro familias de sistema (puerta abatible y corredera; ventana practicable, de 1 a 4 hojas con funcionamiento por hoja, y corredera) fijan el funcionamiento (puerta abatible de una o dos hojas o corredera; ventana fija, practicable de una o dos hojas o corredera), el **hueco de obra** (ancho × alto), el perfil del marco (anchura vista y fondo) y dos materiales: marco y hoja/vidrio.
- **Huecos sin carpintería** (v0.20, como el «hueco» de ArchiCAD o la abertura de muro de Revit): la familia de sistema **Hueco** (marca H-01…) solo tiene hueco de obra; vacía el muro con sus mochetas y no lo rellena. Se coloca con la herramienta Ventana (en su galería de tipos; la plantilla trae «Hueco de paso 90×210»). En IFC es el `IfcOpeningElement` solo, con su propio `GlobalId`, y el tipo en `ArchiSim_Void`.
- Cada **ejemplar** vive en un muro: posición del centro medida sobre la línea de referencia desde el inicio del muro (como Revit), altura de antepecho desde el nivel, lado hacia el que abre (cara interior o exterior) y mano según UNE-EN 12519 (mirando desde el lado hacia el que abre la hoja, bisagras a izquierda o derecha). El retranqueo de la carpintería se mide desde la cara exterior del muro; si no se indica, se centra.
- Se rechaza lo que no cabe: fuera del largo o de la altura del muro, solapado con otro hueco del mismo muro, o con un marco más profundo que el muro. Borrar el muro borra sus huecos; acortarlo de forma que un hueco se salga se rechaza; invertirlo deja los huecos en su sitio.
- En planta, los huecos que corta el plano de corte interrumpen el muro y todas sus capas; las mochetas se cierran con línea de corte. En fachadas de doble hoja, la hoja exterior **vuelve** en la mocheta cerrando la cámara (el aislamiento se detiene a un espesor de ladrillo de la jamba) y el revestimiento exterior (monocapa) **envuelve** la mocheta hasta la carpintería; tabiques y muros sin cámara se cortan rectos. Puertas: cerco, hoja abierta a 90° y arco de giro (correderas: hoja sobre la cara y flecha). Ventanas: marco, línea de vidrio, montante central en las de dos hojas, giro de las hojas practicables (línea fina y arco), hojas desfasadas en las correderas, vierteaguas al exterior y borde del antepecho al interior. Las ventanas que quedan enteras por encima del plano de corte se dibujan discontinuas sin interrumpir el muro.
- En IFC el hueco atraviesa el muro (`IfcRelVoidsElement`) y la puerta o ventana lo rellena (`IfcRelFillsElement`). Su geometría 3D es sencilla (marco, hojas y vidrios cerrados) y lleva `Pset_DoorCommon` / `Pset_WindowCommon` y cantidades. El muro añade superficie lateral y volumen netos.
- Pendiente: premarcos, vuelta del revestimiento interior, persianas y capialzados, dinteles, huecos sin carpintería, puertas y ventanas paramétricas en la librería de familias (fase 2).

### Importación IFC

- Lee IFC2X3 e IFC4 (y sus variantes, también IFC 4.3) de cualquier programa, en metros, milímetros, pies o pulgadas.
- Trae plantas, muros (por su eje o, si no lo tienen, por su colocación y su sólido), sus capas y su línea de referencia (de `IfcMaterialLayerSetUsage`), forjados (por el contorno de su extrusión o por la cara superior de su sólido), y puertas y ventanas alojadas en muros (hueco de obra desde el `IfcOpeningElement`, funcionamiento y mano, marco desde las propiedades de cerco). Se conservan todos los `GlobalId`.
- La cara exterior de un muro en IFC no está definida: en los muros marcados como exteriores (`IsExternal`), se toma como exterior el lado opuesto al centro de los muros de su planta, y el muro se gira si hace falta para que quede a la izquierda, como en ArchiSim.
- Muros curvos de un solo arco (v0.19), con sus capas medidas en radial; muros de varios tramos (eje en polilínea o curva compuesta de rectas y arcos) como un muro por tramo, unidos (el primero conserva el `GlobalId`, los demás reciben uno derivado, igual en cada importación), y muros circulares cerrados como dos medias circunferencias (v0.20).
- Forjados con huecos: los del perfil y los `IfcOpeningElement` que los vacían (así exportan Revit y ArchiCAD escaleras y patinillos), por su contorno en planta (v0.20).
- Huecos de muro sin puerta ni ventana, como huecos de la familia Hueco de su tamaño (v0.20).
- Lo que no se importa (pilares, vigas, escaleras, cubiertas inclinadas, forjados inclinados, locales…) se cuenta en un **informe**; lo que se importa con aproximaciones (capas sin espesor, marcos sin medidas, huecos en forjados), también.
- Lo que exporta ArchiSim se importa idéntico (ida y vuelta comprobada en pruebas): para ello el IFC lleva, además, el cerco de puertas y ventanas (`IfcDoorLiningProperties`, `IfcWindowLiningProperties`), el funcionamiento de cada hoja de ventana (`IfcWindowPanelProperties`), la mano de cada hueco (`ArchiSim_Opening`) y la trama de corte de cada material (`ArchiSim_Material`).

### Colaboración (fase 4, v0.26)

- Menú **Colaboración** y ventana **Coordinación** (Vínculos, Interferencias, Incidencias BCF, Requisitos IDS). Detalle en `docs/COLABORACION.md`.
- **Vínculos**: IFC de otras disciplinas vistos en planta (cortados por el plano de corte) y en 3D, sin importarlos ni exportarlos; se recargan solos al cambiar el fichero y colorean lo nuevo y lo modificado por `GlobalId`. Se guardan junto al proyecto (`<proyecto>.coordinacion.txt`); el formato `.archisim` no cambia.
- **Interferencias** con tolerancia (10 mm), sin contar contactos ni lo que pasa por huecos; vista 3D aislada (el resto del modelo en fantasma); a BCF con historial (siguen, nuevas, resueltas).
- **BCF 2.1 y 3.0**: abrir, aplicar vistas, estado, asignación y comentarios, nueva incidencia desde la vista 3D con imagen.
- **IDS 1.0**: el proyecto tal como sale en su IFC contra los requisitos del cliente; clic en un fallo para seleccionar el elemento.

### Instalación

- **Mac** (v0.13): aplicación universal (chip Apple e Intel) en un `.dmg`, con icono y tipos de documento: los `.archisim` se abren con ArchiSim y los `.ifc` lo ofrecen en «Abrir con». Firmada ad hoc (sin cuenta de desarrollador de Apple): la primera vez hay que confirmarla en Ajustes del Sistema → Privacidad y seguridad.
- **Windows** (v0.21): instalador `ArchiSim-<versión>-Windows.exe` (Inno Setup, `scripts/windows_installer.iss`) para el usuario, sin pedir permisos de administrador (se puede elegir instalar para todos): el programa con su icono y su información de versión dentro del `.exe` (`crates/archisim-app/build.rs`), entrada en el menú Inicio, icono opcional en el escritorio, la vivienda de prueba y los IFC de ejemplo. Los `.archisim` se abren con ArchiSim con su icono; los `.ifc` lo ofrecen en «Abrir con» y, si se marca, se abren con él al hacer doble clic. El desinstalador lo quita todo y no toca los proyectos. Sin firma de código: Windows SmartScreen pide «Más información → Ejecutar de todas formas» la primera vez.
- La integración continua monta los dos en cada pull request; el de Windows se instala y desinstala allí mismo para comprobar los tipos de fichero.

### Rendimiento

- Medido con `archisim medir [n] [carpeta]` (v0.22): la vivienda de prueba repetida en una rejilla de n × n (20 × 20: 2800 muros, 4000 huecos, 800 forjados; 30 × 30: 6300 muros, 9000 huecos, 1800 forjados, IFC de 56 MB). Escribe `grande.archisim` y `grande.ifc` para abrirlos en la aplicación; con `ARCHISIM_MEDIR_IMAGENES=1`, también lo que se vería en pantalla.
- Las uniones buscan los muros vecinos en una rejilla de celdas (`archisim_geom::WallIndex`), no entre todos: la geometría de cada muro cuesta lo mismo en una casa que en una urbanización.
- La aplicación guarda la geometría de todos los muros mientras el modelo no cambia (`geo`): forzar, seleccionar, las cotas temporales y la vista previa de un arrastre (solo lo que está cerca de lo arrastrado) no rehacen uniones a cada movimiento del ratón.
- La planta en pantalla dibuja solo la zona visible y, cuando se aleja mucho, lo que no se distinguiría: sin tramas ni zigzag (separación de trama < 2,5 px), muros como una banda gris sin capas (espesor < 4 px) y huecos sin símbolo (ancho < 8 px). Las plantas exportadas (SVG) se dibujan siempre completas.
- Cifras de la 30 × 30 en el servidor de pruebas: planta en pantalla de una zona de 30 × 20 m, 0,07 s; toda la urbanización a la vista, 0,5 s; forzado o selección bajo el cursor, 10 ms; mover un muro, 0,1 s; 3D completo, 0,8 s; exportar el IFC, 0,9 s; importarlo, 3,8 s; abrir el proyecto, 0,2 s.

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
- **Fase 3:** planos y maquetación, acotación, etiquetas, tablas de planificación, impresión a PDF — hecha en su mayor parte en v0.25 (falta lo que depende de las secciones y alzados de la fase 2).
- **Fase 4:** complemento puente para Revit, BCF, colaboración (modelo central), disciplinas de ingeniería.

## 7. Forma de trabajo

- **Antonio:** experto BIM y dueño del producto. Prueba cada versión, abre los IFC en ArchiCAD y Revit, y da feedback.
- **Claude (Claude Code):** escribe todo el código, las pruebas y la documentación técnica. Ver `CLAUDE.md`.
- Cada cambio llega con pruebas automáticas. Cada exportación IFC se valida con IfcOpenShell antes de darla por buena.
- Feedback de Antonio → issue en GitHub con captura y, si se puede, el IFC o SVG afectado.

## 8. Licencia

**GPL-3.0 o posterior** (`GPL-3.0-or-later`), decidida por Antonio el 18/10/2026 al abrir el repositorio: ArchiSim es software libre y cualquier versión modificada que se distribuya también tiene que serlo, para que siga siendo una alternativa libre a los programas de pago. Texto completo en `LICENSE`. Las fuentes IBM Plex van con su propia licencia (SIL Open Font License 1.1, `crates/archisim-app/assets/fonts/OFL-IBM-Plex.txt`).
