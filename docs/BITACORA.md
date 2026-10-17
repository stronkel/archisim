# Bitácora de ArchiSim

Registro de sesiones de trabajo, de la más reciente a la más antigua. Cada conversación nueva lo lee para saber dónde estamos, y añade su entrada al terminar.

Formato de cada entrada: fecha · bloque · qué se hizo · qué quedó pendiente · qué debe probar Antonio.

---

## 2026-10-17 · Fase 1, bloque 5 — Herramientas de forjado, puerta y ventana

**Hecho**
- **Forjado** (tecla F): contorno por clics, como el forjado poligonal de ArchiCAD. Cada clic añade una esquina con los mismos forzados que el muro (extremos y esquinas de muros, caras, puntos medios, alineación a 0/45/90°, Mayús para ángulo forzado) y longitud tecleada + Intro. Se cierra con clic en la primera esquina o con Intro; Esc quita la última esquina. Vista previa con el contorno, el lado que cerrará en discontinua, longitud, ángulo y superficie. Se rechazan contornos que se cruzan.
  - Ajustes en el cuadro de información: tipo (con su canto) y cota de la cara superior sobre la planta. Por defecto, la solera.
- **Puerta** (tecla P) y **Ventana** (tecla V): un clic sobre el muro. Se colocan centradas en el cursor (al centímetro, sin salirse del muro) y abren hacia el lado del muro donde está el cursor. Vista previa con el hueco, el marco y, en las puertas, la hoja abierta y su giro; etiqueta con la distancia libre a cada lado y hacia dónde abre. Si no cabe (se solapa, se sale, el marco no entra) se ve en rojo con el motivo y no se coloca.
  - Ajustes: tipo, mano (UNE-EN 12519), antepecho (puertas 0, ventanas 900 mm por defecto) y retranqueo del marco desde la cara exterior (vacío = centrado).
- Cada colocación es un paso de deshacer («Crear forjado», «Colocar puerta», «Colocar ventana»), y lo colocado se edita después en Propiedades como siempre.
- Pruebas: con clics simulados se dibuja la casa, su solera por las cuatro esquinas, una puerta y una ventana; un segundo hueco encima se rechaza. La vista previa coincide exactamente con lo que se dibuja al colocar. El IFC de esa sesión (muros, solera, puerta y ventana) se valida con IfcOpenShell en la integración continua. Probada la ventana real con capturas.
- Versión 0.6.0.

**Pendiente / limitaciones conocidas**
- Forjado por «varita mágica» (clic dentro de un recinto de muros) y edición de vértices de un forjado existente.
- Mover, copiar y estirar con el ratón; acotación.
- La vista previa de puertas correderas y ventanas es esquemática (hueco, marco y vidrio); al colocarlas se dibujan completas.

**Decisión de Antonio (17/10/2026):** al cambiar la línea de referencia de un muro se mantiene el comportamiento actual (la línea se queda en su sitio y el muro se desplaza, para no romper las esquinas).

**Antonio debe probar**
1. Descarga la última versión: GitHub → pestaña **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**. Descomprime y abre `ArchiSim.exe` (o arrastra `demo.archisim` encima).
2. **Forjado**: Archivo → Nuevo, dibuja un contorno de muros (tecla M). Tecla **F** y clic en las cuatro esquinas exteriores (aparece el cuadrado verde de «Extremo»); clic otra vez en la primera para cerrar. Prueba también un forjado en L tecleando longitudes, y Esc para quitar una esquina mal puesta.
3. **Puerta**: tecla **P**, pasa el cursor por un muro de un lado y del otro: ¿la hoja abre hacia donde está el cursor? Cambia la mano arriba y comprueba que sigue UNE-EN 12519. Clic para colocarla.
4. **Ventana**: tecla **V**, antepecho 900; prueba a ponerla encima de la puerta (sale en rojo y explica por qué). Prueba un retranqueo de `130` en la fachada.
5. Selecciona lo colocado y cámbialo en Propiedades; Ctrl+Z.
6. Exporta el IFC y ábrelo en ArchiCAD y Revit: solera como forjado/suelo, puertas y ventanas en su hueco.
7. ¿Qué prefieres después: varita mágica para forjados, mover/estirar con el ratón, o la vista 3D?

---

## 2026-10-17 · Fase 1, bloque 4 — Aplicación: ventana, vista de planta, herramienta muro y selección

**Hecho**
- Primera ventana de ArchiSim (`archisim-app`, egui sobre wgpu), con la disposición de ArchiCAD: caja de herramientas a la izquierda (Flecha, Muro), cuadro de información arriba con los ajustes de la herramienta activa y el selector de planta, paleta de **Propiedades** a la derecha (como Revit) y barra de estado con coordenadas, punto de forzado y mensajes. Todo en español y en milímetros (acepta también `2,7m` o `270cm`).
- **Vista de planta = la planta 1:50 que se exporta**: se dibuja el mismo SVG de la zona visible a resolución de pantalla, con grosores de línea reales a 1:50 (como el «grosor de línea real» de ArchiCAD). Lo que ves es lo que exportas: poché por capa, mochetas, puertas y ventanas, solera. Rueda para zoom en el cursor, arrastrar con la rueda o con el botón derecho para encuadre, doble clic en la rueda o tecla Inicio para ajustar. Retícula de 1 m y 5 m de fondo.
- **Herramienta muro** (tecla M): clic de inicio y clic de final, encadenando como el muro poligonal de ArchiCAD; los muros encadenados se unen a inglete. Clic en el primer punto cierra el contorno. Mientras dibujas, escribes la longitud en mm y pulsas Intro (va en la dirección del cursor). Vista previa con las capas del tipo, la línea de referencia y la marca «ext.» en el lado exterior; etiqueta con longitud y ángulo.
  - Forzados: extremos de muro, esquinas de muros y forjados, puntos medios y caras de muro o bordes de forjado (para acometer tabiques a la cara interior de la fachada). Alineación automática a horizontal, vertical y 45° cuando el cursor está cerca; con Mayús, ángulo forzado. Fuera de forzados, las coordenadas se redondean al centímetro.
  - Ajustes en el cuadro de información: tipo, línea de referencia (al elegir un tipo de fachada pasa a «cara exterior»; uno interior, a «eje»), altura y desfase de base.
- **Selección**: clic (prioridad puerta/ventana → muro → forjado), Mayús+clic para añadir o quitar, ventana de selección arrastrando (elementos enteros dentro), Ctrl+A, Esc. Resaltado al pasar por encima; seleccionados en azul con puntos de control y la línea de referencia en rojo discontinuo.
- **Propiedades editables** (cada cambio es un paso de deshacer, se confirma con Intro o al salir del campo; Esc lo anula):
  - Muro: tipo, línea de referencia, longitud (desde el inicio), altura, desfase de base, invertir; espesor, planta, inicio, fin, nº de huecos, capas y GlobalId.
  - Puerta/ventana: tipo (solo del mismo tipo de elemento), posición, antepecho, lado de apertura y mano (UNE-EN 12519); hueco de obra, funcionamiento, muro anfitrión y GlobalId.
  - Forjado: tipo y cota de la cara superior; canto, superficie, vértices y GlobalId.
  - Varios muros: tipo y altura para todos, invertir. Cualquier selección: borrar.
  - Sin selección: nombre del proyecto y resumen.
- Deshacer/rehacer (Ctrl+Z / Ctrl+Y) con el nombre de cada paso en el menú; Supr borra (un muro se lleva sus huecos).
- **Archivos**: Nuevo (plantilla), Abrir, Guardar, Guardar como (`.archisim`), Exportar IFC4 y Exportar planta 1:50 (SVG). Aviso de cambios sin guardar al crear, abrir o cerrar. `archisim-app proyecto.archisim` abre directamente un proyecto.
- **Plantilla de inicio** (`archisim-library`): materiales y tipos de construcción española habitual — fachada doble hoja 26,5, tabique LHD 10, muro de carga de 1 pie 27; solera, forjado de planta HA 30, cubierta plana invertida; puertas de entrada, de paso de 1 y 2 hojas, corredera, y cuatro ventanas. La vivienda de prueba se construye ahora sobre la misma plantilla.
- Núcleo: cambiar tipo, altura, desfase y línea de referencia de un muro, tipo de puerta o ventana y tipo y cota de forjado, conservando el `GlobalId` y rechazando lo que deja huecos fuera del muro o marcos que no caben.
- Planta: el rayado y el zigzag del aislamiento son ahora geometría recortada de verdad a cada capa, sin máscaras SVG. La planta exportada es igual píxel a píxel salvo un detalle a mejor (desaparece una línea de rayado que caía justo sobre el inglete y se veía como una junta), el SVG pesa la mitad y en pantalla se dibuja 7 a 80 veces más rápido (9 ms por vista en vez de hasta 740 ms con mucho zoom).
- Pruebas: la aplicación se maneja sin pantalla con clics y teclas simulados (dibujar la casa con longitudes tecleadas, acometer un tabique a las caras, seleccionar, borrar, deshacer, ventana de selección, zoom, aviso de cambios). La casa dibujada así se guarda, se reabre y su IFC se valida con IfcOpenShell (también en la integración continua). Además se ha probado la ventana real en una pantalla virtual con capturas.
- Versión 0.5.0.

**Pendiente / limitaciones conocidas**
- Herramientas de forjado, puerta y ventana en la app (los forjados y huecos existentes se seleccionan y editan; aún no se dibujan nuevos).
- Mover, copiar y estirar con el ratón (por ahora longitud y posiciones se cambian en Propiedades). Puntos de control arrastrables como en ArchiCAD.
- Encuentros en T siguen cortados a escuadra (fase 2); un tabique acometido a la cara de la fachada no se une a ella.
- La vista no muestra todavía otras plantas en gris ni cotas; no hay crear/borrar plantas.
- El aspecto (claro u oscuro) sigue al del sistema.

**Validación de Antonio (17/10/2026):** probada en su Windows con la descarga de GitHub (*Actions → Artifacts → ArchiSim-Windows*): la demo funciona como se esperaba. Integrado en `main` (PR #1).

**Antonio debe probar**
1. Abrir la app con la vivienda de prueba:
   ```
   cargo run -p archisim-cli -- demo out
   cargo run --release -p archisim-app -- out/demo.archisim
   ```
   (la primera compilación en `--release` tarda varios minutos). Juzga la planta en pantalla: ¿se lee como tu planta 1:50? Haz zoom fuerte en una esquina y en una mocheta.
2. Navegación: rueda (zoom en el cursor), arrastrar con la rueda o el botón derecho, Inicio para ajustar. En un portátil Mac: dos dedos = zoom; clic con dos dedos y arrastrar = encuadre. ¿Se mueve con fluidez?
3. Dibujar: Archivo → Nuevo, tecla **M**. Clic en un punto; mueve el cursor hacia arriba, escribe `8000` e Intro; hacia la derecha `10000` Intro; hacia abajo `8000` Intro; clic en el primer punto para cerrar. Esquinas a inglete y cara exterior hacia fuera (dibujando en sentido horario).
4. Tabique: en el cuadro de información elige «Tabique LHD 10» (pasa solo a «Eje»), clic cerca de la cara interior de la fachada sur (aparece una ✕ verde: «Sobre arista») y otro en la cara norte. Esc dos veces.
5. Seleccionar y editar: clic en un muro, cambia en Propiedades la altura a `2700` + Intro, el tipo, la línea de referencia; «Invertir». Selecciona una puerta de la demo y cambia su mano o el lado de apertura: ¿la planta responde como esperas? Supr, Ctrl+Z, Ctrl+Y.
6. Guardar como `prueba.archisim`, cerrar, volver a abrir; Exportar IFC4 y abrirlo en ArchiCAD y Revit: muros con su tipo, capas y esquinas, como siempre.
7. Dime qué echas en falta primero en la app (forjado, puertas y ventanas, mover/estirar, cotas…) y si la línea de referencia debe quedarse en su sitio al cambiarla (ahora se queda la línea y se desplaza el muro, para no romper las esquinas) o al revés.

---

## 2026-10-17 · Fase 1, bloque 3 — Formato de proyecto `.archisim`

**Hecho**
- Formato de proyecto propio `.archisim`: un único fichero JSON legible, con cabecera (`format`, `version: 1`, `units: "m"`, programa que lo guardó), datos del proyecto y todos los elementos: niveles, materiales, tipos de muro y de forjado con sus capas, muros, forjados, tipos de puerta y ventana y sus ejemplares. Descrito para ti en `docs/FORMATO_ARCHISIM.md`.
- Identidad estable: cada elemento conserva su `id` y su `GlobalId`. Comprobado que **guardar → abrir → exportar da el mismo IFC** byte a byte (salvo la fecha de la cabecera) y las mismas plantas SVG; además, guardar dos veces el mismo proyecto da exactamente el mismo fichero (bueno para comparar versiones).
- Al abrir se comprueba todo como si se volviera a dibujar: referencias que existen, `id` y `GlobalId` sin repetir, números válidos, contornos sin cruces, puertas y ventanas dentro de su muro y sin solaparse, funcionamiento coherente (una puerta no puede ser «fija»). Si falla, no se abre a medias: dice qué elemento y por qué; los campos mal escritos se señalan con línea y columna.
- Versionado: un fichero de un ArchiSim más nuevo se rechaza con aviso; hay sitio preparado para actualizar ficheros antiguos cuando cambie el formato.
- Guardado seguro: se escribe un temporal y se sustituye al final; un fallo al guardar no estropea el proyecto anterior.
- De paso, el núcleo rechaza ahora valores no numéricos (NaN, infinito) en cotas, alturas y posiciones, que antes se colaban.
- Línea de comandos: `demo` guarda también `demo.archisim`; nueva orden `exportar <proyecto.archisim> [carpeta]`. La integración continua comprueba la ida y vuelta y valida los dos IFC con IfcOpenShell (ambos válidos).
- Versión del programa: 0.4.0.

**Pendiente / limitaciones conocidas**
- No se guarda el historial de deshacer (como ArchiCAD y Revit).
- Las unidades del fichero son siempre metros; la interfaz mostrará milímetros.
- Todavía no hay propiedades de usuario (parámetros compartidos / `ArchiSim_` Psets) que guardar: cuando existan, se añaden al formato con su migración.
- Siguiente bloque del MVP: ventana de la aplicación (vista de planta, herramienta muro, selección), que abrirá y guardará `.archisim`.

**Antonio debe probar**
- `cargo run -p archisim-cli -- demo out` → ahora aparece `out/demo.archisim`. Ábrelo con un editor de texto (Bloc de notas, VS Code) y juzga si se entiende: niveles, tipos con sus capas, muros, puertas y ventanas.
- Edición a mano y reexportación:
  1. En `out/demo.archisim` cambia `"name": "Vivienda de prueba"` por otro nombre, y en algún muro `"height": 3.0` por `2.7`. Guarda.
  2. `cargo run -p archisim-cli -- exportar out/demo.archisim`
  3. Abre `out/demo.ifc` en ArchiCAD o Revit: nombre del proyecto y altura del muro cambiados; el resto, igual.
- Identidad estable: ejecuta la demo, abre `out/demo.ifc` en ArchiCAD/Revit y anota el GUID de un muro; ejecuta `exportar` y vuelve a abrirlo (o compáralo con el anterior en Solibri/BIMcollab Zoom): el GUID debe ser el mismo.
- Errores: escribe mal un campo (`"heigth"`) o pon una ventana encima de una puerta (`"along"` igual al de la puerta) y ejecuta `exportar`: debe decirte qué está mal y dónde, sin generar nada.

---

## 2026-10-17 · Fase 1, bloque 2b — Mochetas y giro de ventanas (feedback de Antonio)

**Hecho**
- Mochetas en fachadas de doble hoja: la hoja exterior (ladrillo perforado) vuelve en cada jamba y cierra la cámara hasta la hoja interior; el aislamiento se detiene un espesor de ladrillo (11,5 cm) antes de la jamba. Líneas finas solo donde cambia el material (vuelta contra aislamiento y contra la hoja interior); la vuelta y la hoja exterior se leen como una pieza continua.
- El revestimiento exterior (monocapa) envuelve la mocheta desde la cara de fachada hasta la carpintería. A 1:50 sus 1,5 cm quedan bajo la línea de corte, como corresponde a la escala.
- La regla es automática según el tipo de muro: vuelve la primera hoja (estructura o soporte) seguida de una cámara (aislamiento, cámara de aire o lámina) con algo detrás; envuelve el acabado exterior si el muro es de fachada. Tabiques y muros sin cámara se cortan rectos.
- Ventanas practicables: giro de cada hoja en planta (línea fina a 90° y arco) hacia el lado de apertura; en las de dos hojas, cada hoja llega al eje del montante.
- Correcciones de dibujo: el vierteaguas ya no se mete bajo el muro cortado (solo sobresale de las jambas por fuera de la fachada) y las líneas de corte se dibujan las últimas, para que marcos y vierteaguas no tapen media mocheta.

**Pendiente / limitaciones conocidas**
- Sin premarco ni vuelta del enlucido interior; sin dintel ni capialzado en planta ni en IFC.
- Las vueltas de mocheta son de dibujo: en IFC el muro sigue siendo un sólido con su conjunto de capas (`IfcMaterialLayerSetUsage`), como lo exportan Revit y ArchiCAD.
- Siguiente bloque del MVP: formato de proyecto propio (`.archisim`).

**Antonio debe probar**
- `cargo run -p archisim-cli -- demo out` y abrir `out/demo.svg` con mucho zoom en cualquier jamba de fachada:
  - ¿La vuelta del ladrillo y el corte del aislamiento son como los dibujarías? ¿Ancho de vuelta = espesor del ladrillo te vale, o prefieres otro (p. ej. medio pie fijo)?
  - ¿La carpintería a 13 cm apoya bien contra la vuelta?
  - Giro de ventanas practicables (salón sur, cocina norte, dormitorio sureste): ¿sentido y mano correctos?

---

## 2026-10-17 · Fase 1, bloque 2 — Puertas y ventanas

**Hecho**
- Tipos de puerta (abatible 1 hoja, abatible 2 hojas, corredera) y de ventana (fija, practicable 1 hoja, practicable 2 hojas, corredera 2 hojas) con hueco de obra, perfil de marco (anchura vista y fondo) y materiales de marco y de hoja o vidrio.
- Puertas y ventanas alojadas en muro: posición del centro sobre la línea de referencia, antepecho desde el nivel, lado de apertura, mano (UNE-EN 12519) y retranqueo de la carpintería desde la cara exterior (o centrada). Se rechazan si se salen del muro, se solapan con otro hueco o el marco no cabe en el espesor. Borrar un muro borra sus huecos; acortarlo dejando un hueco fuera se rechaza; invertirlo los deja en su sitio. Mover una ventana conserva su `GlobalId`.
- Planta: el muro y todas sus capas se interrumpen en los huecos cortados, con mochetas en línea de corte. Puertas con cerco, hoja abierta a 90° y arco; correderas con hoja sobre la cara y flecha. Ventanas con marco, vidrio, montante central (dos hojas), hojas desfasadas (corredera), vierteaguas al exterior y borde del antepecho al interior. La ventana alta por encima del plano de corte (1,10 m) sale discontinua sin cortar el muro.
- IFC4: `IfcOpeningElement` que vacía el muro, `IfcDoor` / `IfcWindow` que lo rellenan con su tipo (`IfcDoorType` / `IfcWindowType`), `IfcMaterialConstituentSet` (marco + hoja o vidrio), geometría 3D sencilla con el vidrio transparente, `Pset_DoorCommon` / `Pset_WindowCommon` (`Reference`, `IsExternal`) y cantidades de ancho, alto y superficie. Los muros añaden `NetSideArea` y `NetVolume`, exactos incluso con huecos junto a una esquina a inglete.
- Vivienda de prueba con 4 puertas y 5 ventanas: entrada de 100×215 en fachada oeste, tres puertas de paso de 80×205, balconera corredera de 240×220, ventanas practicables de 1 y 2 hojas y una ventana fija alta (antepecho 1,60) por encima del plano de corte.
- Validador IFC ampliado: cada hueco vacía un muro y tiene un único relleno; cada puerta y ventana tiene tipo, materiales, Pset, cantidades, geometría 3D y planta; el volumen 3D de cada muro (ya perforado) coincide con su volumen neto. Pasa sin errores con reglas EXPRESS activas.

**Pendiente / limitaciones conocidas**
- Mochetas: el muro se corta recto en todo su espesor; no se dibujan la vuelta de la hoja exterior, el premarco ni el cierre de la cámara.
- Sin dinteles, persianas, capialzados ni huecos sin carpintería (paso libre); sin huecos en forjado.
- Ventanas practicables: en planta no se dibuja el giro de las hojas; en IFC no se exporta todavía la operación de cada hoja (`IfcWindowPanelProperties`).
- Mano de puertas en IFC: `OperationType` sigue la convención IFC (vista desde el lado contrario al de apertura), por eso izquierda y derecha se invierten al exportar. Está razonado pero sin contrastar con Revit/ArchiCAD.
- La posición se comprueba sobre la línea de referencia: un hueco pegado a una esquina a inglete puede invadir el triángulo de la esquina por la cara interior.
- Siguiente bloque del MVP: formato de proyecto propio (`.archisim`).

**Antonio debe probar**
- `cargo run -p archisim-cli -- demo out` y abrir `out/demo.svg` en el navegador (con zoom). Juzgar como planta 1:50:
  - Mochetas, cerco y hoja de la entrada (fachada oeste, abre hacia dentro, bisagras al norte).
  - Puertas de paso: ¿abren hacia donde esperas y con la mano correcta?
  - Ventanas: marco retranqueado 13 cm (detrás del ladrillo perforado), vierteaguas, línea de antepecho interior y montante de la de dos hojas.
  - Balconera corredera (fachada norte, salón) y ventana fija alta discontinua (fachada este).
  - ¿Qué nivel de detalle quieres en mochetas y en el giro de hojas de ventana?
- Abrir `out/demo.ifc` en ArchiCAD y Revit y comprobar:
  - Las puertas y ventanas entran como puertas y ventanas (no objetos genéricos), en su hueco y con el muro perforado en 3D.
  - Tipos con sus nombres («Puerta de entrada 1 hoja 100×215», «Ventana practicable 2 hojas 180×130»…), materiales (madera, aluminio, vidrio) y vidrio transparente.
  - Ancho, alto y antepecho correctos; propiedades `IsExternal` (entrada y ventanas sí, puertas de paso no).
  - Mano de las puertas en las propiedades (`OperationType`): ¿coincide con lo que ves? Si sale al revés, dímelo y se corrige.
  - Cantidades del muro: superficie y volumen netos descontando huecos.

---

## 2026-10-17 · Fase 1, bloque 1 — Forjados

**Hecho**
- Tipos de forjado multicapa (capas de arriba abajo) con función: forjado de planta, solera o cubierta plana.
- Forjado por contorno poligonal (cualquier forma sin cruces), con nivel y desfase de la cara superior. Se rechazan contornos que se cruzan, con menos de tres puntos o sin superficie. Mover el contorno conserva el `GlobalId`. Un tipo en uso no se puede borrar.
- Exportación IFC4: `IfcSlabType` (`FLOOR` / `BASESLAB` / `ROOF`), `IfcSlab` extruido desde la cara inferior, `IfcMaterialLayerSetUsage` en vertical (`AXIS3`, capas de abajo arriba), `Pset_SlabCommon` (`IsExternal`, `LoadBearing`, `Reference`) y `Qto_SlabBaseQuantities` (canto, perímetro, superficie y volumen).
- Planta: los forjados se dibujan según el plano de corte (1,10 m): por debajo, trama de cubrición clara y línea fina; cortados, poché de la capa estructural; por encima, discontinuos.
- Vivienda de prueba: solera de 34,7 cm bajo cota 0 y cubierta plana invertida con grava de 54,8 cm apoyada sobre los muros (cara inferior a +3,00). El nivel «Cubierta» pasa a +3,548 (cara superior de la cubierta). Se genera una planta SVG por nivel (`demo.svg`, `demo-cubierta.svg`).
- Validador IFC ampliado: comprueba tipo, capas, Psets, cantidades, volumen 3D, dirección vertical de capas y orden de abajo arriba en cada forjado. Pasa sin errores.

**Pendiente / limitaciones conocidas**
- Huecos en forjado (escaleras, patinillos) → cuando se haga el hueco genérico (bloque 2).
- Recorte muro–forjado: la solera y la cubierta ocupan toda la huella, también bajo y sobre los muros; no hay intersecciones porque los muros acaban justo en la cara del forjado, pero no hay prioridades de intersección → fase 2.
- Forjados inclinados y cubiertas con pendiente → fase 2 (cubiertas).
- Siguiente bloque del MVP: huecos de puertas y ventanas.

**Antonio debe probar**
- Abrir `out/demo.ifc` en ArchiCAD y Revit y comprobar:
  - Solera y cubierta entran como **forjados** (ArchiCAD) / **suelos** y **cubierta o suelo** (Revit), no como objetos genéricos.
  - Tipos «Solera HA 15 con encachado» y «Cubierta plana invertida con grava».
  - En sección: capas en orden, con la baldosa / grava arriba y el encachado / enlucido abajo.
  - La cubierta apoya sobre los muros (cara inferior a +3,00) y la solera queda bajo cota 0 (cara inferior a −0,347).
  - Propiedades `IsExternal` y `LoadBearing`, y cantidades de superficie (108 m²) y volumen.
- Abrir `out/demo.svg` y `out/demo-cubierta.svg` en el navegador y juzgar si la representación de la solera bajo los muros y del forjado de cubierta es la que esperarías en una planta 1:50.

---

## 2026-10-17 · Bloque 0 — Fundación

**Hecho**
- Documento fundacional (`docs/FUNDACIONAL.md`) e instrucciones para Claude (`CLAUDE.md`).
- Workspace Rust con cinco crates: core, geom, ifc, plan, cli.
- Núcleo: niveles, materiales, tipos de muro multicapa, muros, deshacer/rehacer.
- Uniones de muros en esquina (L) a inglete, capa por capa.
- Exportación IFC4: tipos, capas (`IfcMaterialLayerSetUsage`), `Pset_WallCommon`, `Qto_WallBaseQuantities`. Validado con IfcOpenShell (esquema, reglas EXPRESS y volumen de cada muro).
- Planta SVG 1:50 con relleno por material, rayado, zigzag de aislamiento y gruesos de línea.
- Integración continua en GitHub (Windows, macOS, Linux) y validación IFC.

**Pendiente / limitaciones conocidas**
- Encuentros en T a tope (línea de fachada continua) → fase 2.
- Siguiente bloque del MVP: forjados.

**Antonio debe probar**
- Abrir `demo.ifc` en ArchiCAD y Revit: muros reconocidos, tipos, orden de capas (monocapa fuera, yeso dentro), esquinas limpias, propiedades y cantidades.
