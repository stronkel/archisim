# Entramados ligeros: steel frame y madera

Antonio pidió el 05/11/2026 una herramienta de steel frame y de madera para las soluciones constructivas que está desarrollando el estudio: levantar fácilmente y que saque solo el despiece correcto. Decidido con él:
- los dos sistemas a la vez;
- muros, forjados y cubierta en la primera versión;
- el despiece con lista de corte, paneles prefabricados, y Excel y BC3;
- perfiles genéricos, sin catálogo de fabricante.

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
- **Se dibuja como siempre.** Los muros, forjados y cubiertas de ese tipo se dibujan con las herramientas de siempre. El entramado sale solo y sigue a cada cambio: mover un muro, abrir una ventana, cambiar la altura.
- **En 3D**, con «Entramado a la vista» (barra de abajo, solo si el proyecto tiene entramados): los elementos con entramado se ven por su esqueleto en lugar de sus capas.
- **El despiece**, en Archivo → Exportar:
  - **Despiece de entramados (PDF)**, en hojas A3:
    - la lista de corte por perfil, pieza y longitud;
    - los totales, con las barras y tableros que hay que comprar y las fijaciones;
    - cada panel de muro dibujado en alzado a 1:50, con su marca, medidas, número de piezas y peso.
  - **Despiece de entramados (Excel)**, con cuatro hojas: lista de corte, totales y compra, paneles, y cada pieza con sus coordenadas.
  - **Mediciones y presupuesto (BC3)**: el capítulo 15, «Entramados ligeros».
    - Una partida por perfil: kg de acero o m³ de madera, con una línea por pieza.
    - Una partida por tablero, en m².
    - Las claves son `entramado:<perfil>` y `tablero:<nombre>`, y a ellas se les asignan los precios del banco.

## 2. Qué se coloca

**Muros** (rectos):
- **Paneles prefabricados**: el muro se parte en paneles de 6 m como mucho (lo que cabe en un camión), lo más iguales posible. Cada corte se hace en un montante que no esté junto a un hueco. Cada panel lleva su marca: «M3-2» es el segundo panel del tercer muro con entramado.
- **Canales**: un canal inferior y uno superior por panel. En madera, además, una carrera de atado de todo el muro encima, que cubre las juntas.
- **Montantes**: a la separación elegida, medida desde el inicio del muro. Se ponen dos en cada extremo del muro y uno en cada borde de panel.
- **En cada puerta y ventana**:
  - un montante de refuerzo de toda la altura a cada lado;
  - un montante de apoyo del dintel;
  - un dintel de dos piezas de canto, una en cada cara, en cajón;
  - un canal de antepecho bajo las ventanas;
  - montantes cortos a la separación encima del dintel y debajo del antepecho.
- **Travesaños** a media altura entre montantes, salvo donde hay un hueco.
- **Encuentros**:
  - en una T, el muro que llega se para en el entramado del otro;
  - en una L, el muro más antiguo pasa hasta la cara exterior del otro y el otro se para en él.
- El entramado va centrado en la capa de estructura del tipo (o en la más gruesa, si ninguna es de estructura).

**Forjados**:
- **Viguetas** perpendiculares al lado más largo del contorno, a la separación, recortadas por el contorno y los huecos.
- Un **zuncho** por todo el borde y alrededor de los huecos.
- Una fila de **riostras** a media luz.
- Los forjados inclinados también.

**Cubiertas**:
- **Cabios** en la línea de máxima pendiente de cada faldón.
- Un **zuncho de faldón** por sus bordes: alero, testero, cumbrera y limatesas.

## 3. El despiece

- **Lista de corte**: por perfil, pieza y longitud al milímetro, con unidades, metros, kg (acero) o m³ (madera).
- **Barras que comprar**: las piezas de cada perfil se colocan, de mayor a menor, en barras comerciales con 3 mm de corte. Las barras miden 6 m en acero y 5,4 m en madera.
- **Tableros**: la superficie de una cara de muros, forjados y faldones, con los huecos descontados, más un 10 % de recortes.
- **Fijaciones** (estimación):
  - tornillos autotaladrantes en acero: 4 por pieza y 20 por m² de tablero;
  - clavos en madera: 4 por pieza y 30 por m² de tablero;
  - anclajes de muro: uno cada 1,2 m y uno en cada extremo.
- **Peso de cada panel**: el entramado más el tablero, que cuenta a 600 kg/m³.

## 4. Pendiente

- **Muros curvos y muros con perfil en alzado**: los muros curvos no llevan entramado, y los muros recortados por la cubierta (hastiales) o con perfil propio se entraman como rectángulos.
- **IFC**: el entramado no se escribe aún como `IfcMember` (piezas para fabricación).
- **Planta y sección**: los montantes no se dibujan aún en planta a 1:20 ni en sección.
- **Forjados en cajones prefabricados**: hoy solo los muros van en paneles.
- **Cerchas y cumbrera estructural** en cubierta; dinteles calculados por luz.
- **Catálogos de fabricante**: perfiles de catálogo, si el estudio los da.

## 5. Para el programador

- **Modelo**: `archisim-core::model::framing`.
  - `FrameMaterial`, `FrameSection` (`steel`, `timber`, `kg_per_m`), `steel_studs`, `steel_tracks`, `timber_sections`, `Sheathing`, `Framing` (`for_depth`).
  - En `Model`: `framings`, `type_framing`, `set_type_framing`, `frame_layer`, `frame_member_material`.
  - Los materiales de las piezas, con `archisim_library::frame_material`.
  - Formato versión 80: `framings` (`FramingOf`).
- **Piezas**: `archisim-geom::framing`.
  - Las piezas: `Member` con su `Role`, y `wall_members`, `slab_members`, `roof_members`, `all_members`.
  - Paneles y lista de corte: `wall_panels`, `panel_elevation`, `cut_list` (`CutLine`, `SectionTotal`, `BoardTotal`).
  - Sólidos: `member_triangles` (C y U de chapa, o madera maciza) y `frame_solids`.
  - En 3D, con `with_frames_shown`: `model_solids` quita los elementos con entramado (`is_framed`) y pone sus piezas.
- **Papel**: `archisim-plan::framing::breakdown_pages` (hojas A3) y `breakdown_svg` (una hoja larga).
- **Excel**: `archisim-export::xlsx` (`workbook`, `Sheet`, `Cell`; un zip sin comprimir) y `framing_workbook`.
- **Mediciones**: capítulo 15 en `archisim-plan::schedules::framing`.
- **App**:
  - `app/framing_ui::framing_group` en `types_ui`, con `Action::TypeFraming`;
  - `View3dState::frames` con el chip de la barra de abajo;
  - `Document::Framing` y `export_framing_xlsx`.
- **Ejemplo**: `archisim entramado out/`. Para fotos: `ARCHISIM_VISTAS_ENTRAMADO=1` en `project_pictures`.
