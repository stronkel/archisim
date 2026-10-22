# Gráficos y vistas (bloque 2 de la revisión del 22/10/2026)

Antonio pidió «control total del dibujo, que no te controle a ti»: los colores de cada categoría como en Revit (forjados blancos, urbanismo y jardín en pastel, muros blancos con la cámara de aire solo por dentro, transparencias), una tipografía más elegante, metros, vistas guardadas por oficio (superficies, albañilería), plantas de techo, planos de referencia y líneas finas o grosor real. Este documento explica cómo quedó.

## Visibilidad y gráficos (VG)

Cada vista de planta tiene su ventana **Visibilidad y gráficos**: en Propiedades de la vista (con nada seleccionado) → grupo Vista → «Visibilidad y gráficos…», o en el menú Vista. Una fila por categoría (muros, forjados, puertas y ventanas, pilares y vigas, cubiertas, escaleras, muros cortina, barandillas, objetos, **vegetación**, terreno, volúmenes, parcelas, viales, instalaciones, cotas, cotas de nivel, marcas de sección, ejes, **planos de referencia**, dibujo 2D, locales y etiquetas) con:

| Columna | Qué hace |
|---|---|
| Visible | Se ve o no en esta vista |
| Líneas | Un color para todas sus líneas y textos (o «Auto»: el de cada elemento) |
| Relleno de corte | El relleno de lo cortado: muros, pilares, muros cortina, cimentación |
| Relleno de superficie | El de lo visto: forjados, suelos, terreno, viales, parcelas, objetos, copas de los árboles |
| Transparencia | De 0 a 100 % |
| Medios tonos | Todo a medio camino hacia el blanco |
| Tramas | Sin marcar, desaparecen las tramas de los materiales y el zigzag del aislamiento |
| Grosor | % del grosor de la norma gráfica |
| Líneas de capas (muros) | Todas las capas · **Solo la cámara de aire** (el contorno de lo que hay entre las hojas: aislamiento y cámara juntos) · Solo el contorno |

Se guarda con la vista y se imprime igual en PDF y DXF. El botón ↺ de cada fila la deja como siempre; «Restablecer todo», todas.

## Plantillas de vista

Un aspecto con nombre para dárselo a varias vistas (como las plantillas de Revit). La plantilla de inicio trae tres:

- **AD5**: todo lo cortado y lo visto en blanco con líneas negras, muros con solo la cámara, sin tramas; vegetación, terreno, parcelas y viales en tonos pastel.
- **Albañilería**: muros, puertas, ventanas y cotas; sin mobiliario, jardín, instalaciones, locales ni etiquetas.
- **Superficies**: los locales con su pavimento y superficie, con el aspecto AD5; sin mobiliario, cotas ni etiquetas.

Se aplican desde Propiedades de la vista («Plantilla») o desde la ventana VG, donde también se guarda el aspecto actual como plantilla nueva (o se sobrescribe una con el mismo nombre) y se borran. Aplicar una plantilla copia su aspecto: cambiar luego la plantilla no cambia las vistas que ya la usaron.

## Vistas guardadas y plantas de techo

Cada planta tiene su vista propia, que ahora **recuerda su aspecto** en el proyecto. Además se pueden guardar más vistas de una planta, cada una con su nombre, escala, plano de corte y gráficos:

- Propiedades de la vista → «Guardar vista» (o clic derecho en la planta, en el navegador).
- **Planta de techos**: la planta vista hacia arriba. Muestra el forjado de encima como superficie, las luminarias y las instalaciones, y las vigas; las puertas y ventanas sin barrido; sin mobiliario, vegetación, escaleras, terreno ni pavimentos.

En el navegador, las vistas guardadas cuelgan de su planta; clic para abrirlas en su pestaña (con su nombre), clic derecho para renombrarlas o borrarlas, y se arrastran a un plano como las plantas: llegan con su escala, plano de corte, aspecto y su nombre como título.

La demo trae cuatro: «Planta baja — AD5», «— albañilería», «— superficies» y «— techos».

## Planos de referencia

Estructura o Documentación → **Plano ref.** (tecla Q): dos clics. Una línea verde de trazo y punto, con su nombre, que se ve en todas las plantas. Los muros, las líneas y las cotas se fuerzan a ella: a sus extremos, a cualquier punto de su recorrido y a donde cruza otro plano o una cara de muro. Se seleccionan, se mueven, se copian y se estiran por sus extremos; en Propiedades, nombre y longitud. No se construyen ni van al IFC. Se ocultan con su categoría en VG.

## Tipografía

Los textos, cotas, rótulos de locales, etiquetas, cajetines y tablas usan ahora por defecto **IBM Plex Sans** (licencia libre SIL OFL) dibujada como contornos rellenos: se ve igual en pantalla, SVG, PDF y DXF sin depender de las fuentes del ordenador. La rotulación técnica de trazo de antes sigue disponible: Visibilidad → «Normas gráficas…» → Tipografía. Las alturas son de mayúscula, como antes, así que nada cambia de tamaño.

## Unidades

Vista → **Unidades**: metros (por defecto), centímetros o milímetros. Las medidas se ven y se escriben en esa unidad («2,70»); siempre se puede escribir otra con su sufijo (`270cm`, `2700mm`). Las etiquetas de Propiedades dicen la unidad: «Ancho (m)». Se recuerda en el ordenador.

## Líneas finas o grosor real

Botón con tres líneas de grosor creciente en la barra inferior, junto a los forzados: encendido, cada línea con su grosor a la escala de la vista; apagado, todas finas. También en Vista y en Propiedades de la vista.

## Para el programador

- Núcleo: `archisim_core::GraphicOverride`, `LayerLines`, `ViewStyle::overrides` (`graphics`, `set_graphics`, `ad5`), `Category::Vegetation` y `Category::RefPlanes`, `TextFont` en `GraphicStandard::font`; `model/views`: `PlanView` (`PlanKind`), `level_style`, `ViewTemplate`, `RefPlane`; `Viewport::ceiling`. Formato versión 27.
- Planta: cada categoría se dibuja dentro de `<g data-cat="…">` (`archisim_plan::overrides`); `overrides::apply` reescribe colores, grosores y opacidad del grupo sobre el SVG terminado y quita las tramas (`class="pat"`); las líneas entre capas de muro se deciden al dibujar (`overrides::layer_lines`). `PlanOptions::ceiling` para la planta de techos. Planos de referencia en `draw_ref_planes`.
- Letra: `archisim_draft::outline` (contornos de IBM Plex Sans con `ttf-parser`, fichero en `crates/archisim-draft/fonts/`), `archisim_draft::font::{current, use_font, set_font}`; en la planta, `lettering::text_path`, `text_d` y `paint` según la letra.
- App: `app/views_ui` (`sync_views`, `open_saved_view`, `duplicate_view`, `vg_window`), `ViewTab::saved`, `units::LengthUnit` (`fmt_mm` y `parse_length` en la unidad elegida, `fmt_len`, `unit_label`), `Tool::RefPlane` y `Element::RefPlane`.

## Pendiente

- Gráficos por categoría también en secciones, alzados y 3D (hoy, en plantas y en las plantas de los planos).
- Sustituciones por elemento (un muro concreto en rojo) y por filtro (todos los muros de un tipo).
- Más tipografías.
- Falsos techos como elementos, con su altura y su trama, para la planta de techos.
