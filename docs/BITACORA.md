# Bitácora de ArchiSim

Registro de sesiones de trabajo, de la más reciente a la más antigua. Cada conversación nueva lo lee para saber dónde estamos, y añade su entrada al terminar.

Formato de cada entrada: fecha · bloque · qué se hizo · qué quedó pendiente · qué debe probar Antonio.

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
