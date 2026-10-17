# Bitácora de ArchiSim

Registro de sesiones de trabajo, de la más reciente a la más antigua. Cada conversación nueva lo lee para saber dónde estamos, y añade su entrada al terminar.

Formato de cada entrada: fecha · bloque · qué se hizo · qué quedó pendiente · qué debe probar Antonio.

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
