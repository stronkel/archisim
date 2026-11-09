# ArchiSim — Formas de Concepto y materiales de visualización

> 22 de octubre de 2026 · v0.39 · rama `bloque/modelado`. Es el bloque M de la hoja de ruta (§6.4): M1 primitivas y aristas, M2 operaciones entre volúmenes, M3 texturas y materiales de visualización. Pensado para volumetrías de concurso, piezas singulares y entorno: no es modelado BIM. Los volúmenes de Concepto no salen en el IFC, como las masas de Revit.

## 1. Formas (M1)

En la disciplina Concepto, la ficha **Formas** dibuja volúmenes redondos alrededor de un eje vertical. En la barra superior se elige cuál:

| Forma | Cómo se dibuja | En Propiedades |
|---|---|---|
| **Esfera** | Centro y radio | Radio (es tan alta como ancha) |
| **Cilindro** | Centro y radio | Radio y altura |
| **Cono** | Centro y radio | Radio de abajo, radio de arriba (0: cono; más: tronco de cono) y altura |
| **Revolución** | Clic en el eje y, luego, el medio perfil **abatido sobre la planta**: al lado del eje, el radio; hacia arriba, la altura. Clic otra vez en el último punto o Intro para girarlo | El número de puntos del perfil y el radio mayor |

El perfil de una revolución puede tener tantos tramos como se quiera (un jarrón, una columna con basa y capitel, una cúpula). Cada tramo es una cara aparte, que se puede pintar de otro material.

### Aristas achaflanadas o redondeadas

Un volumen por contorno (prisma, banda de Seguir), un cilindro, un cono o una revolución tiene en Propiedades el grupo **Aristas**:

- **Acabado**: chaflán o redondeo.
- **Tamaño**: el cateto del chaflán o el radio del redondeo (se reduce solo donde los lados son cortos).
- **Cuáles**: las **esquinas** (aristas verticales; no en las formas redondas), las de **arriba** y las de **abajo**.

El acabado no destruye el volumen: si se cambia el contorno o la altura, las aristas se rehacen con él.

## 2. Operaciones entre volúmenes (M2)

Con dos o más volúmenes seleccionados (con Mayús), Propiedades enseña **Operaciones con volúmenes**:

- **Sobre**: el volumen que queda; los demás se le unen, se le restan o se intersecan con él.
- **Unir**: un solo volumen con todos.
- **Restar**: quita los demás del volumen elegido (un patio cilíndrico en un bloque, un hueco en una masa).
- **Intersecar**: deja solo lo que tienen en común.
- **Intersecar caras**: no cambia los volúmenes. Dibuja **aristas sueltas** donde se cruzan sus caras, como en SketchUp, para ver los encuentros y repartir las caras al pintar.

El resultado es un **sólido**: ya no es un prisma ni una forma redonda, así que no tiene contorno ni radio que editar. Se mueve, se gira y se copia como cualquier volumen, y Deshacer vuelve atrás. **Cada cara conserva su material**: lo que viene de otro volumen sale pintado con el material de ese volumen (el corte de un cilindro de hormigón en un bloque de ladrillo queda de hormigón), y las caras pintadas siguen pintadas.

Por dentro, cada volumen se convierte en un sólido cerrado de triángulos que sabe de qué cara es cada uno. Las operaciones se hacen con árboles de partición del espacio (el método clásico, escrito para ArchiSim). Al final, los trozos de cada cara se vuelven a unir, para que no queden líneas ni astillas dentro de una cara. En planta, el sólido se dibuja por la huella que cubre, con sus huecos.

## 3. Materiales de visualización (M3)

Cada material tiene, además de su color y trama de corte, un **aspecto en 3D** (Tipos y materiales → el material → **Aspecto en 3D**):

- **Color de la superficie**: el color con que se ve en 3D, si no es el de corte (marcar la casilla y elegirlo).
- **Textura**:
  - fábricas: ladrillo (a soga, con llagas), piedra (piezas irregulares), hormigón visto (con las juntas del encofrado y los huecos de los latiguillos) y enfoscado;
  - suelos y revestimientos: baldosa, madera, tablas (tarima o revestimiento);
  - exteriores: césped, grava y agua;
  - chapa grecada.
- **Tamaño de la pieza**: el largo del ladrillo, el lado de la baldosa, el ancho de la tabla.

La vista 3D dibuja la textura en cada cara a partir de su posición: en las caras que miran arriba, sobre la planta; en las demás, a lo largo de la cara y hacia arriba. No hay imágenes ni nada que desplegar, así que sirve igual en un muro, un volumen o una esfera, y el proyecto no pesa más.

**Por cara**: con Pintar (Concepto) se da un material a una sola cara de un volumen o de un muro, y la cara se ve con su textura. En los volúmenes redondos, la superficie curva es una cara; en las revoluciones, cada tramo del perfil; en los sólidos, cada cara que salió de la operación.

La plantilla de un proyecto nuevo trae ya con su aspecto:

- los materiales que se ven en caras: monocapa y yeso (enfoscado), baldosa, hormigón, grava, agua y madera;
- cuatro acabados nuevos para pintar volúmenes: Ladrillo visto, Piedra, Tarima de madera y Césped.

## 4. Probarlo

`archisim formas out/` hace `out/formas.archisim` con todo junto, más su planta y una axonometría en SVG:

- un bloque de ladrillo con las esquinas y la coronación redondeadas, al que se le ha restado un cilindro;
- una esfera de piedra unida a una peana achaflanada;
- un cono y un tronco de cono;
- un jarrón de revolución con la barriga de madera;
- lo común de una caja y un cilindro;
- una torre y un puente con las aristas donde se cruzan;
- todo sobre un césped.

## 5. Modelado (bloque 4 de la revisión del 22/10/2026, v0.44)

Antonio echaba en falta, al probar Concepto: dividir caras con aristas y pintar solo esa parte; empujar caras que no sean la de arriba; doblar y retorcer; un perfil propio para Seguir; y que redondear aristas se encontrara a la primera.

- **Dividir** (Concepto → Dividir): dos clics sobre una cara de un volumen (mejor en 3D; en planta, sobre la cara de arriba) la parten por esa línea, de borde a borde. Cada parte se pinta con Pintar (también en planta) o se empuja por separado.
- **Empujar cualquier cara**: la de arriba de un volumen por contorno sigue cambiando su altura como siempre; un **lado** (en planta, clic junto a su arista; en 3D, sobre la cara) mueve esa arista hacia fuera o hacia dentro y el volumen sigue siendo un contorno editable; la **base** baja el volumen; y cualquier otra cara —una parte dividida, la cara de un sólido— sale a lo largo de su normal como en SketchUp: hacia fuera hace un saliente y hacia dentro un rebaje. Se escribe la distancia + Intro.
- **Deformar** (Concepto → Deformar): clic en un volumen y gira el ratón alrededor para **retorcerlo**; con Mayús, llévalo hacia un lado para **doblarlo**. En Propiedades → Deformar: torsión, curvatura y hacia dónde (y «Enderezar»). No destruye el volumen: si se cambia su contorno o su altura, se retuerce igual.
- **Perfiles de Seguir**: en las propiedades de la herramienta Seguir o de una banda ya hecha, **Perfil**: rectangular, tubo, L, U, doble T, cornisa o media caña, estirados al ancho y a la altura; o **Tomar del dibujo…**: clic en una polilínea cerrada del dibujo 2D o en una cara de Concepto, dibujada como el perfil (de lado el ancho, hacia arriba la altura).
- **Redondear**: con volúmenes seleccionados, botón **Redondear** en la barra bajo las pestañas: esquinas y aristas de arriba redondeadas 15 cm (otra vez: vivas). El tamaño y cuáles, en Propiedades → Aristas.

Al dividir o empujar una cara que no es de un contorno, el volumen pasa a ser un sólido (como al unir o restar): conserva sus caras pintadas.

`archisim formas out/` trae ahora una torre retorcida un cuarto de vuelta, otra doblada, un bloque con media cubierta subida y pintada de madera, una cornisa y un tubo.

Para el programador: `archisim_geom::sculpt` (`split`, `divide_face`, `push_face`, `deform`, `sweep`, `move_edge`, `preset_profile`; en el modelo, `divide_mass_face`, `push_mass_face`, `mass_to_mesh`); `Mass::deform` (`Deform`) y `MassShape::Sweep::profile`; formato versión 28; en la app, `app/sculpt_ui` y las herramientas `Tool::DivideFace` y `Tool::Deform`.

## 6. Volúmenes de SketchUp (bloque «Concepto», entrada 135 de la bitácora)

Los volúmenes que el estudio recibe hechos en SketchUp (exportados como DAE de COLLADA, OBJ o STL, también dentro de un `.zip`) entran como **volúmenes de Concepto editables**, no como muebles.

- **Cómo se trae**: Archivo → **Traer volúmenes (SketchUp, OBJ, STL)…**, o soltar el fichero en la ventana (o Archivo → Importar modelo 3D) y elegir arriba **Traer como: Volumen de Concepto**. El diálogo dice las medidas, enseña la planta y el alzado, y pide el nombre, **Colocar** («En sus coordenadas»: el origen del modelo en el del proyecto; «Donde haga clic»: el centro de su base donde se haga clic en la planta, con su contorno siguiendo al ratón; Esc los deja donde vinieron), las unidades, el eje vertical y **Reconocer los prismas**.
- **Cada pieza cerrada es un volumen**, en la planta activa, a su altura (la cota 0 del modelo en el suelo de la planta). Se juntan los puntos que coinciden a menos de 1 mm, se quitan las caras que SketchUp exporta por las dos caras, se giran todas hacia fuera y se cierran los agujeros planos (la base que suele faltar).
- **Prismas**: la pieza que es un contorno extruido (caras de los lados verticales, arriba y abajo horizontales, un solo contorno sin patios) pasa a ser un volumen normal con su contorno y su altura: se empuja, se edita su contorno, se hace masa por plantas y se pasa a edificio.
- **Mallas**: las demás (cubiertas a dos aguas, formas curvas) son sólidos en los que cada cara plana de un material es una cara de Concepto: se pintan y se empujan una a una.
- **Abiertas**: una pieza que no cierra (le falta más de una cara, o su borde no es plano) se trae igual, como superficie, y lo dice el aviso; su volumen no cuenta. Una sola cara horizontal se trae como cara de Concepto (altura 0), para empujarla.
- **Materiales**: cada material del modelo pasa a ser un material del proyecto con su color (si ya hay uno con ese nombre, se usa ese; los «_» se cambian por espacios; la textura la da su nombre, como en los muebles). El volumen toma el material que más superficie cubre y las demás caras van pintadas con el suyo. Las caras sin material (el blanco por defecto de SketchUp) toman el material «Volumen de concepto».
- **Unidades**: el DAE de SketchUp dice las suyas (pulgadas, metros…) y su eje vertical (`Z_UP`). OBJ y STL no las dicen: se suponen **metros si el modelo mide hasta 2 000 unidades y milímetros si mide más** (en centímetros, elegirlo a mano en el diálogo).
- **IFC**: como todos los volúmenes de Concepto, no se exportan (son una ayuda de proyecto); lo que se exporta es el edificio que se hace con ellos.

`archisim concepto-malla <fichero> [carpeta] [--edificios] [--unidades m|cm|mm|in]` hace lo mismo sin la aplicación: escribe `concepto-malla.archisim`, `concepto-malla.ifc`, la planta en SVG y `concepto-malla-informe.txt` (cada pieza: prisma o malla, cerrada o abierta, agujeros cerrados, caras, triángulos, superficie en planta, altura y volumen). Con `--edificios` pasa los prismas a edificio con plantas de 3 m.

Lo que no hace:
- Patios: un prisma con un patio queda como malla (los volúmenes por contorno no tienen huecos).
- Dos grupos de SketchUp que comparten una cara entera pueden quedar como uno solo o abiertos.
- Las superficies curvas suaves de SketchUp son muchas caras planas (una por triángulo), no una sola cara curva.
- Las texturas de las imágenes del modelo: se queda su color medio.
- Los componentes repetidos no se reconocen como tales: cada copia es un volumen.
- Leer el `.skp` directamente (hay que exportar desde SketchUp).

Para el programador: `archisim_mesh::solids` (`solids` → `Solids` con `Solid`: soldar, quitar las dobles, piezas por aristas, orientar, `fill_flat` con `ear_clip`, caras coplanarias por material; `extrusion` → `Extrusion`, `flat_face`, `volume`, `is_plain_material`), `plausible_site_unit` y `Loaded::guess_for_site`; `archisim_library::concept_mesh` (`import_concept` con `ConceptOptions` → `ConceptImport` con `ConceptPiece` y `PieceKind`, `summary`, `report`); en la app `Editor::import_concept`/`move_masses`, `MeshImport::as_volume`/`at_click`/`prisms`/`volumes`, `pick_concept_mesh`, `import_concept_now`, `ConceptDrop` (`concept_drop_click`, `draw_concept_drop`); ficheros de prueba de `scripts/make_concept_fixtures.py`. Sin cambios de formato.

## 7. Pendiente

- Las texturas en las axonometrías de los planos y en los PDF (hoy, el color liso del material) y en el IFC.
- Editar un sólido después de la operación (hoy se deshace y se repite).
- Redondear aristas de un sólido ya operado.
- Dividir una cara solo entre dos aristas suyas (hoy, de borde a borde).
- Empujar regiones curvas (hoy, caras planas).
