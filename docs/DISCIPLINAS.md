# ArchiSim — Disciplinas y mediciones (propuesta)

> 18 de octubre de 2026 · Propuesta para que Antonio decida alcance y orden. Sin código todavía.
> Nace de su petición: estructura (pilares, vigas, forjados, cimentación), urbanismo (entorno, paisajismo, viales) y mediciones con bancos de precios «como Presto, pero dentro del programa».

## 1. Principio: un solo modelo

No habrá programas ni ficheros separados por disciplina. Cada elemento del proyecto sabe:

- a qué **disciplina** pertenece: arquitectura, estructura o urbanismo;
- si es **estructural** o no (la «función estructural» de ArchiCAD, el «Estructural» de Revit). Un muro de carga es arquitectura y estructura a la vez.

Con eso:

- **La caja de herramientas** tiene una sección por disciplina (Modelado arquitectónico, Estructura, Urbanismo), igual que ahora tiene Modelado y Documentación.
- **Las vistas** filtran o atenúan lo que no es suyo: una planta de estructura muestra pilares, vigas, forjados y muros de carga con su trama, y lo demás en gris o nada (como las plantillas de vista de Revit o las combinaciones de capas de ArchiCAD; enlaza con 3.1 de la hoja de ruta).
- **El IFC** ya separa las disciplinas por clase (`IfcColumn`, `IfcFooting`, `IfcRoad`…) y por la propiedad `LoadBearing`; un ingeniero recibe el mismo fichero y filtra lo suyo.
- **Las mediciones** salen todas del mismo modelo, cada una en su capítulo.

La alternativa (un fichero por disciplina vinculados entre sí, como se suele hacer en Revit) sigue siendo posible para equipos grandes con la vinculación de modelos IFC (4.1 de la hoja de ruta), pero no es la forma principal de trabajar.

## 2. Estructura

Herramientas nuevas en la sección **Estructura**:

| Herramienta | Qué hace | IFC |
|---|---|---|
| **Pilar** | Sección rectangular, circular o perfil metálico; de planta a planta con restricción superior (como los muros de v0.17); material y recubrimiento | `IfcColumn` |
| **Viga** | Por dos puntos o entre pilares; sección; canto por debajo de la cara superior del forjado | `IfcBeam` |
| **Forjado estructural** | El forjado actual con su capa estructural marcada: unidireccional, reticular, losa maciza, chapa colaborante (como tipo) | `IfcSlab` con `LoadBearing` |
| **Muro de carga y de contención** | Los muros actuales con la función estructural marcada; contención con trasdós y drenaje | `IfcWall` con `LoadBearing` |
| **Cimentación** | **Zapata aislada** bajo un pilar (se coloca con un clic en el pilar y lo sigue), **zapata corrida** bajo un muro (sigue el muro), **losa**, **viga de atado** y de centrado entre zapatas; más adelante encepados y pilotes | `IfcFooting` (`PAD_FOOTING`, `STRIP_FOOTING`, `FOOTING_BEAM`), `IfcPile` |

Reglas de la casa:

- Los pilares y la cimentación tienen **prioridad sobre los muros y forjados** en las uniones (el hormigón atraviesa el ladrillo), con el mismo sistema de prioridades por capa que ya tienen los muros.
- La cimentación vive en la planta más baja (o en una planta «Cimentación» con cota negativa), con su cota de cara inferior.
- **Lo que no haremos**: el cálculo estructural. ArchiSim modela y documenta; el cálculo se hace en un programa de cálculo, intercambiando por IFC (y, más adelante, con el modelo analítico: ejes de barras y nudos, 4.x).

## 3. Urbanismo

Lo que rodea al edificio y lo que se urbaniza, sin mezclarse con el edificio:

| Herramienta | Qué hace | IFC 4.3 |
|---|---|---|
| **Terreno** | Superficie desde puntos o curvas de nivel (topográfico importado en DXF o CSV); explanaciones con su desmonte y terraplén | `IfcGeographicElement` (`TERRAIN`), `IfcEarthworksCut` / `IfcEarthworksFill` |
| **Parcela** | Lindes, superficie, retranqueos; la orientación real del norte (2.12) | `IfcSite` |
| **Vial** | Por un eje y una **sección tipo**: calzada, aparcamiento, bordillo, acera, alcorque; con su firme por capas (como un multicapa) | `IfcAlignment`, `IfcRoad`, `IfcCourse`, `IfcPavement`, `IfcKerb` |
| **Paisajismo** | Árboles y arbustos (objetos de la librería), zonas verdes, pavimentos exteriores | `IfcGeographicElement` |
| **Mobiliario urbano** | Bancos, farolas, papeleras (librería) | `IfcFurnishingElement` / `IfcLamp` |

- Todo esto pertenece a la **parcela** (`IfcSite`), no al edificio (`IfcBuilding`). Por eso **no cuenta** en las superficies construidas ni en la medición del edificio: va en su propio capítulo («Urbanización») o en un presupuesto aparte.
- Las clases de viales y obra civil existen solo en **IFC 4.3**. Esto adelanta la decisión **D3** de la hoja de ruta: el urbanismo exporta en 4.3 aunque el edificio siga saliendo en IFC4, o pasamos todo a 4.3.

## 4. Mediciones y presupuesto

Es lo que más diferencia a ArchiSim de Revit y ArchiCAD: la medición y el presupuesto **dentro** del modelo, vivos.

### Cómo funcionaría

1. **Partidas en los tipos.** Cada tipo (de muro, de forjado, de puerta…) o cada capa de un multicapa se **vincula** a una o varias partidas, con un **criterio de medición**:
   - muros por cara o por eje, en m², descontando huecos según un criterio que se elige (por ejemplo, deducir solo los mayores de 1 m², o medir a cinta corrida, según la costumbre del estudio o el pliego);
   - capas en m² o m³ (la hoja de ladrillo en m², el aislamiento en m², el hormigón en m³);
   - forjados en m² (con o sin descontar huecos), soleras en m², cantos en ml;
   - puertas y ventanas por unidad (ud), con su marca de tipo (P-01, V-03);
   - pilares y vigas en m³ de hormigón, kg de acero, m² de encofrado;
   - urbanización en su capítulo.
2. **Bancos de precios.** Se importan en **BC3 (FIEBDC-3)**, el formato público español que usan Presto, Arquímedes y los bancos oficiales. Para AD5 el natural es la **Base de Costes de la Construcción de Andalucía (BCCA)**; habrá que comprobar sus condiciones de uso antes de incluir ninguna en el programa (el usuario siempre podrá importar la suya).
3. **El presupuesto se genera solo**: capítulos, partidas con sus líneas de medición (cada una señala el elemento del que sale; un clic lo selecciona en la planta), precio y total. **Se actualiza al cambiar el modelo**: alargas un muro y su partida cambia.
4. **Partidas a mano** para lo que no se modela (seguridad y salud, gestión de residuos, ayudas), como en Presto.
5. **Salida**: informe PDF del presupuesto y de las mediciones, y **exportación BC3** para quien siga con Presto o Arquímedes, o para el contratista.

### Por fases (no es «todo Presto» de golpe)

| Fase | Contenido |
|---|---|
| **M1** | Tablas de medición por tipo y capa (enlaza con 3.7 de la hoja de ruta): superficies de muro por cara con huecos, volúmenes por capa, unidades de carpintería por marca |
| **M2** | Importar bancos BC3 y vincular partidas a tipos y capas con su criterio |
| **M3** | Presupuesto vivo, partidas a mano, informe PDF y exportación BC3 |
| **M4** | Certificaciones, control de costes y comparativos de ofertas (lo que Presto hace en obra) |
| **M5** | Planificación (diagrama de Gantt) ligada a partidas y elementos |

Sala limpia, como siempre: se replica cómo se trabaja con Presto y el formato público BC3; nada de su código ni de sus ficheros.

## 5. Decisiones para Antonio

| # | Decisión | Propuesta |
|---|---|---|
| D8 | Disciplinas como **propiedad de cada elemento** en un solo modelo (§1), o ficheros separados vinculados | Un solo modelo |
| D9 | IFC para urbanismo: **4.3** solo para lo de la parcela, o todo el proyecto en 4.3 (resuelve D3) | Primero el edificio en IFC4 y la parcela en 4.3; pasar todo a 4.3 cuando Revit y ArchiCAD lo lean bien |
| D10 | **Banco de precios** de referencia | BCCA (Junta de Andalucía), si sus condiciones lo permiten; cualquier BC3 importable |
| D11 | **Criterio de deducción de huecos** por defecto en muros | Elegible por proyecto; por defecto, el que uséis en AD5 (dímelo) |
| D12 | **Orden** después del bloque C de familias | M1 (tablas de medición, ya hay tipos y capas para medir) → Estructura (pilar, viga, cimentación) → M2–M3 → Urbanismo |

## 6. Qué necesito de Antonio

1. Las decisiones D8 a D12.
2. Un presupuesto real de AD5 en BC3 (de Presto) de una vivienda parecida a la de prueba, para usar sus capítulos y criterios de medición como referencia (en local, no se sube al repositorio).
3. Cómo cimentáis normalmente vuestras viviendas (zapatas aisladas con vigas de atado, corridas, losa) para que la herramienta Cimentación empiece por lo que más usáis.
