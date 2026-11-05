# Entramados ligeros: steel frame y madera

Antonio pidió el 05/11/2026 una herramienta de steel frame y de madera para las soluciones constructivas que está desarrollando el estudio: levantar fácilmente y que saque solo el despiece correcto. Decidido con él:
- los dos sistemas a la vez;
- muros, forjados y cubierta en la primera versión;
- el despiece con lista de corte, paneles prefabricados, y Excel y BC3;
- perfiles genéricos, sin catálogo de fabricante.

El mismo día pidió que ArchiSim haga también lo de Tricalc LSF y que el programa publicado no lleve marcas de otras empresas (el estudio trabaja con perfiles de SFSystem, sin estar asociado). Decidido: un predimensionado mientras se dibuja, la salida hacia los programas de cálculo y los catálogos de fabricante como ficheros propios del estudio (§3 ter).

## 1. Cómo se usa

- **El entramado es del tipo.** En Tipos y materiales, cada tipo de muro y de forjado tiene un grupo «Entramado ligero». Las cubiertas usan tipos de forjado.
  - Se activa con un interruptor y se elige:
    - **sistema**: steel frame (acero galvanizado) o madera C24;
    - **perfil de los montantes**, viguetas o cabios;
    - **perfil de los canales**, durmientes o zunchos;
    - **separación**: 300, 400, 488, 600 o 625 mm;
    - **travesaños** a media altura (sí o no);
    - **tablero** de arriostramiento: OSB de 12 o 15 mm, fenólico de 18, yeso o placa de cemento.
  - Al activarlo se proponen los perfiles que caben en la capa de estructura del tipo. Si el perfil tiene más canto que la capa, se avisa.
  - En las listas de perfiles salen los genéricos y, detrás, los de los catálogos del estudio (§3 ter).
- **Se dibuja como siempre.** Los muros, forjados y cubiertas de ese tipo se dibujan con las herramientas de siempre. El entramado sale solo y sigue a cada cambio: mover un muro, abrir una ventana, cambiar la altura.
- **En 3D**, con «Entramado a la vista» (barra de abajo, solo si el proyecto tiene entramados): los elementos con entramado se ven por su esqueleto en lugar de sus capas.
- **El despiece**, en Archivo → Exportar:
  - **Despiece de entramados (PDF)**, en hojas A3:
    - la lista de corte por perfil, pieza y longitud;
    - los totales, con las barras y tableros que hay que comprar y las fijaciones;
    - cada panel de muro dibujado en alzado a 1:50, y cada cajón de forjado y de cubierta visto desde arriba, con su marca, medidas, número de piezas y peso.
  - **Despiece de entramados (Excel)**, con cuatro hojas: lista de corte, totales y compra, paneles y cajones, y cada pieza con sus coordenadas.
  - **Mediciones y presupuesto (BC3)**: el capítulo 15, «Entramados ligeros».
    - Una partida por perfil: kg de acero o m³ de madera, con una línea por pieza.
    - Una partida por tablero, en m².
    - Las claves son `entramado:<perfil>` y `tablero:<nombre>`, y a ellas se les asignan los precios del banco.
  - **Ejes del entramado (DXF 3D)**: cada pieza como una línea 3D por su eje, en metros, con una capa por perfil y papel («C 100×40×10×1 - MONTANTE»), para llevarlo a un programa de cálculo de estructuras.
- **El predimensionado**, en Colaboración → Comprobación CTE, apartado «Entramado ligero (predimensionado)» (también `archisim cte <proyecto>`): §3 ter.

## 2. Qué se coloca

**Muros** (rectos y curvos):
- **Paneles prefabricados**: el muro se parte en paneles de 6 m como mucho (lo que cabe en un camión), lo más iguales posible. Cada corte se hace en un montante que no esté junto a un hueco. Cada panel lleva su marca: «M3-2» es el segundo panel del tercer muro con entramado.
- **Canales**: un canal inferior y uno superior por panel. En madera, además, una carrera de atado de todo el muro encima, que cubre las juntas.
- **Montantes**: a la separación elegida, medida desde el inicio del muro. Se ponen dos en cada extremo del muro y uno en cada borde de panel.
- **En cada puerta y ventana**:
  - un montante de refuerzo de toda la altura a cada lado;
  - un montante de apoyo del dintel;
  - un dintel de dos piezas de canto, una en cada cara, en cajón; su canto crece con la luz (una octava parte de la luz en madera, una décima en acero), hasta lo que cabe bajo la carrera;
  - un canal de antepecho bajo las ventanas;
  - montantes cortos a la separación encima del dintel y debajo del antepecho.
- **Travesaños** a media altura entre montantes, salvo donde hay un hueco.
- **Encuentros**:
  - en una T, el muro que llega se para en el entramado del otro;
  - en una L, el muro más antiguo pasa hasta la cara exterior del otro y el otro se para en él.
- **Hastiales y muros con perfil**: los montantes llegan hasta la cubierta a la que se recorta el muro, o hasta el borde de su perfil en alzado; la carrera superior se parte donde cambia la pendiente (en la cumbrera).
- **Muros curvos**: los montantes van por el arco a la separación y los canales se cortan en cuerdas de montante a montante.
- El entramado va centrado en la capa de estructura del tipo (o en la más gruesa, si ninguna es de estructura).

**Forjados**:
- **Viguetas** perpendiculares al lado más largo del contorno, a la separación, recortadas por el contorno y los huecos.
- Un **zuncho** por todo el borde y alrededor de los huecos.
- Una fila de **riostras** a media luz.
- **Cajones prefabricados** de 2,4 m de ancho como mucho: en cada junta la vigueta se dobla (una para cada cajón) y los zunchos se cortan. Se marcan «F1-1», «F1-2»…
- Los forjados inclinados también.

**Cubiertas**:
- **Cabios** en la línea de máxima pendiente de cada faldón.
- Un **zuncho de faldón** por sus bordes: alero, testero, cumbrera y limatesas.
- **Cerchas** en las cubiertas a dos aguas: un tirante al pie de cada pareja de cabios.
- Cada faldón en **cajones** como los forjados («C1-1»…).

## 3. El despiece

- **Lista de corte**: por perfil, pieza y longitud al milímetro, con unidades, metros, kg (acero) o m³ (madera).
- **Barras que comprar**: las piezas de cada perfil se colocan, de mayor a menor, en barras comerciales con 3 mm de corte. Las barras miden 6 m en acero y 5,4 m en madera.
- **Tableros**: la superficie de una cara de muros, forjados y faldones, con los huecos descontados, más un 10 % de recortes.
- **Fijaciones** (estimación):
  - tornillos autotaladrantes en acero: 4 por pieza y 20 por m² de tablero;
  - clavos en madera: 4 por pieza y 30 por m² de tablero;
  - anclajes de muro: uno cada 1,2 m y uno en cada extremo.
- **Peso de cada panel y cajón**: el entramado más el tablero, que cuenta a 600 kg/m³.
- **Tableros de los muros**: la cara hasta su coronación real (los hastiales cuentan su triángulo).

## 3 bis. En los planos y en el IFC

- **Planta a 1:25 o más detalle**: los montantes cortados por el plano de corte, cada C o cada pieza de madera en blanco con su contorno.
- **Sección a 1:50 o más detalle**: los montantes, viguetas, cabios y tirantes cortados.
- **IFC**: cada pieza es un `IfcMember` (montantes `STUD`, canales y zunchos `PLATE`, travesaños `STRUT`, cabios `RAFTER`, tirantes `CHORD`, viguetas y dinteles `MEMBER`) con su cuerpo, su material y `ArchiSim_FramePart` (pieza, perfil, longitud y panel), parte de su muro o forjado. Al importar el IFC se quitan: el entramado sale otra vez del tipo.

## 3 ter. Predimensionado y catálogos

**Qué comprueba.** Por cada tipo con entramado, en cada muro, forjado y cubierta de ese tipo, la pieza más cargada:
- **montantes**: compresión con pandeo y, en los muros exteriores, flexión por viento;
- **dinteles**: flexión y flecha con la luz del hueco y lo que baja por el muro;
- **viguetas y cabios**: flexión y flecha (L/300) con su luz libre. Un muro portante debajo cuenta como apoyo intermedio.

Cada línea da el aprovechamiento (1,00 es el límite) y, si no cumple, el perfil más ligero de la lista que cumpliría, o qué hace falta («un apoyo intermedio»). Al pulsar la línea se selecciona el elemento.

**Con qué cargas** (hay que confirmarlas en cada proyecto):
- forjados: 1,5 kN/m² de carga permanente y 2,0 kN/m² de uso (vivienda);
- cubiertas: 1,0 kN/m² de carga permanente y 0,4 kN/m² de nieve;
- muros: 0,5 kN/m² de su cara, y 0,8 kN/m² de viento en los exteriores;
- combinaciones del DB-SE: 1,35 G + 1,5 Q, y con el viento como principal en los exteriores;
- acero S350GD, con los anchos eficaces de las chapas finas (Eurocódigo 3, partes 1-3 y 1-5); madera C24 (Eurocódigo 5).

**Qué no es.** Es un predimensionado para saber, mientras se dibuja, si los perfiles están en su orden. No sustituye al cálculo firmado: no mira las uniones, el arriostramiento de los paneles por sus tableros, las cargas puntuales ni el fuego. Para eso, el DXF de ejes y el IFC van al programa de cálculo.

**El IFC** lleva en cada pieza, además, su perfil: canto, ala, pestaña, espesor y calidad (S350GD o C24).

**Catálogos de fabricante, sin marcas en el programa.** ArchiSim solo trae perfiles genéricos (C y U por medidas, madera C24). Los de un fabricante los pone el estudio en su biblioteca, en la carpeta **«Perfiles de entramado»**, como un fichero de texto (`.csv` o `.txt`) que se puede hacer con Excel. Una línea por perfil, separada por punto y coma:

```
# nombre;material;canto;ala;pestaña;espesor (mm)
C-90-1,0;acero;90;40;12;1,0
U-92-1,0;acero;92;35;0;1,0
Pino 45×145;madera;145;45
```

- El material es `acero` o `madera`; la pestaña es 0 en los U; la madera no lleva pestaña ni espesor.
- Los perfiles con pestaña salen primero en Montantes y los U primero en Canales; todos están en las dos listas.
- El fichero no viaja con el programa: es del estudio, como los símbolos de planta. Un proyecto guarda las medidas de los perfiles que usa, así que se abre igual en otro ordenador aunque no tenga el catálogo.

## 4. Pendiente

- **Ficheros para las máquinas** (perfiladoras de acero, centros de corte de madera): cuando se sepa qué máquina usa el taller, porque cada una lee su formato.
- **Cálculo completo**: uniones, arriostramiento por tableros, sismo y fuego quedan para el programa de cálculo.
- **El entramado del tipo en el IFC**: se escriben las piezas, pero aún no la definición del entramado en el tipo, así que un IFC importado no recupera el entramado.
- **Dinteles calculados**: el canto sale de una regla de proporción (luz/8 en madera, luz/10 en acero); no es un cálculo estructural.

## 5. Para el programador

- **Modelo**: `archisim-core::model::framing`.
  - `FrameMaterial`, `FrameSection` (`steel`, `timber`, `kg_per_m`), `steel_studs`, `steel_tracks`, `timber_sections`, `Sheathing`, `Framing` (`for_depth`).
  - En `Model`: `framings`, `type_framing`, `set_type_framing`, `frame_layer`, `frame_member_material`.
  - Los materiales de las piezas, con `archisim_library::frame_material`.
  - Formato versión 80: `framings` (`FramingOf`).
- **Piezas**: `archisim-geom::framing`.
  - Las piezas: `Member` con su `Role` y `along` (dónde va a lo largo del muro), y `wall_members` (sobre `WallLine`: recto o curvo, `top_at` con el perfil y los planos de `Model::wall_trim_planes`; `header_section`), `slab_members`, `roof_members` (con los tirantes `Role::Tie`), `all_members`.
  - Paneles, cajones y lista de corte: `wall_panels`, `panel_elevation`, `deck_panels` (`Cassette`, `CASSETTE_MAX`, `plan`, `size`), `cut_list` (`CutLine`, `SectionTotal`, `BoardTotal`).
  - Cortes: `member_cut` (las chapas de una pieza en un plano), que usan la planta (`draw_layers`, clase `frame-cut`) y la sección (`section::frame_detail`).
  - IFC: `archisim-ifc::framing` (`write_frame_members`; el importador quita `ArchiSim_FramePart`).
  - Sólidos: `member_triangles` (C y U de chapa, o madera maciza) y `frame_solids`.
  - En 3D, con `with_frames_shown`: `model_solids` quita los elementos con entramado (`is_framed`) y pone sus piezas.
- **Papel**: `archisim-plan::framing::breakdown_pages` (hojas A3) y `breakdown_svg` (una hoja larga).
- **Excel**: `archisim-export::xlsx` (`workbook`, `Sheet`, `Cell`; un zip sin comprimir) y `framing_workbook`.
- **Mediciones**: capítulo 15 en `archisim-plan::schedules::framing`.
- **Predimensionado**: `archisim-cte::framing` (`check`, `resistance`, `wall_loads`, `clear_span`; sección `Section::Framing`, sujetos `Subject::Wall`/`Floor`/`Roof`).
- **Catálogos**: `archisim_core::parse_section_catalogue`; en la app `framing_ui::library_sections` (carpeta `FOLDER`, leída cada 5 s) y `sections_for`.
- **DXF de ejes**: `archisim-export::framing_dxf` (R12, LINE en 3D); en la app `export_framing_dxf`.
- **App**:
  - `app/framing_ui::framing_group` en `types_ui`, con `Action::TypeFraming`;
  - `View3dState::frames` con el chip de la barra de abajo;
  - `Document::Framing` y `export_framing_xlsx`.
- **Ejemplo**: `archisim entramado out/` (con un tabique portante por casa; escribe también `entramado-ejes.dxf`). Para fotos: `ARCHISIM_VISTAS_ENTRAMADO=1` en `project_pictures`.
