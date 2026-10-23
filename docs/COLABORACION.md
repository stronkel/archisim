# ArchiSim — Colaboración e ingenierías (fase 4)

> 19 de octubre de 2026 · v0.26 · Bloques 4.1 a 4.4 de `docs/HOJA_DE_RUTA.md`.
> Qué hace ArchiSim con los modelos de las otras disciplinas, cómo y con qué
> estándares, y qué queda por decidir.

## 1. Principio: estándares abiertos, nada propietario

Todo lo que entra y sale de la coordinación es un estándar público de
buildingSMART, para que funcione igual con Revit, ArchiCAD, Tekla, Cype,
Solibri, BIMcollab, Navisworks o Trimble Connect:

| Para qué | Estándar | En ArchiSim |
|---|---|---|
| Modelos de otras disciplinas | **IFC** 2X3, 4 y 4.3 | Vínculos (se ven, no se importan) |
| Requisitos del cliente | **IDS 1.0** | Comprobar requisitos |
| Incidencias con su vista | **BCF 2.1 y 3.0** | Incidencias |

Cada elemento se reconoce por su **GlobalId**: por eso ArchiSim nunca lo cambia
(ni al exportar ni al copiar el proyecto), y por eso pedimos lo mismo a los
ingenieros.

## 2. Modelos vinculados (4.1)

*Colaboración → Vincular IFC…*

- Cualquier IFC (2X3, 4, 4.3) de cualquier programa. Se lee su geometría tal
  cual: extrusiones de cualquier perfil (también los metálicos IPE, HEB, UPN,
  L, T, tubos y huecos), sólidos facetados, mallas, geometría compartida
  (`IfcMappedItem`), tubos (`IfcSweptDiskSolid`), revoluciones y primitivas.
  Lo que no se puede leer exactamente (recortes booleanos, superficies curvas
  NURBS, mallas abiertas) se marca como **aproximado** y se dice cuántos hay.
- Se ve **en planta**, cortado por el plano de corte de la vista (líneas del
  color del vínculo), y **en 3D** con un color por disciplina (estructura azul,
  climatización verde, fontanería celeste, electricidad amarillo…). Se puede
  ocultar o hacer transparente. Un clic en un elemento vinculado en 3D dice
  qué es (clase, nombre, planta y GlobalId).
- **No se importa ni se exporta**: el IFC del proyecto sigue siendo solo
  arquitectura, como hacen Revit y ArchiCAD con sus vínculos.
- **Versiones**: si el ingeniero manda una versión nueva con el mismo nombre,
  ArchiSim la vuelve a leer sola (lo comprueba cada pocos segundos) y compara
  elemento a elemento por GlobalId: **nuevos** (verde), **modificados**
  (naranja: movidos, con otra forma o con otros datos) y **borrados** (en la
  lista). Tolerancias: 1 mm y 1 cm³.
- Dónde se guarda: en un fichero junto al proyecto,
  `<proyecto>.coordinacion.txt` (rutas relativas, visibilidad, transparencia,
  color, fichero de incidencias y tolerancia). Así el formato `.archisim` no
  cambia; cuando las ramas abiertas lleguen a `main` se puede pasar dentro
  (decisión D13).

Validación: el volumen de cada elemento leído coincide con el del motor de
geometría de IfcOpenShell en los 21 modelos de ejemplo de buildingSMART
(arquitectura, estructura, climatización, urbanización, puente, carretera,
ferrocarril, fontanería, electricidad; IFC2X3, IFC4 e IFC4.3) y en la vivienda
de prueba (`scripts/compare_volumes.py`).

## 3. Interferencias (4.2)

*Colaboración → Comprobar interferencias*

- El proyecto (tal como sale en su IFC) contra cada vínculo visible.
- Hay interferencia cuando un elemento se mete en otro **más que la
  tolerancia** (10 mm por defecto). Lo que solo se toca (un muro sobre el
  forjado, una viga contra la cara de un pilar) no cuenta, y tampoco lo que
  pasa **por un hueco** (una bajante por el hueco de una ventana, un conducto
  por un pasamuros dibujado).
- Cómo se mide: se recorren las aristas de cada elemento cada 2 cm y se
  mira si caen dentro del otro y a qué profundidad; se da el punto más
  profundo. Un mismo elemento en los dos modelos (el mismo GlobalId) no es
  una interferencia.
- La lista dice el elemento del proyecto, el vinculado, cuánto se meten y la
  planta. Un clic la muestra en **3D aislada**: los dos elementos en color y
  el resto del modelo en fantasma, para verla aunque esté dentro de un muro o
  bajo la cubierta (*Ver todo* vuelve al modelo entero).
- *Pasar a incidencias*: una incidencia BCF por pareja, con su vista. Al
  repetir la comprobación sobre el mismo BCF, las que siguen reciben un
  comentario, las que han desaparecido pasan a **Resuelta** y las nuevas se
  añaden. Las incidencias de otros vínculos no se tocan.

**Reglas por disciplinas y holguras** (v0.48). En la pestaña de
interferencias, *Reglas por disciplinas y holguras* es una tabla de parejas
de disciplinas (la de cada elemento sale de su clase IFC: arquitectura,
estructura, climatización y ventilación, fontanería y saneamiento,
electricidad, urbanización): si la pareja se comprueba y qué **holgura**
pide, en mm. Dos elementos de esa pareja que no se meten uno en otro pero
quedan más cerca que la holgura salen en la lista en ámbar, «a 30 mm
(holgura 50 mm)», después de las interferencias de verdad, y como
incidencia BCF «quedan a…, menos de la holgura de…». Las reglas con las que
empieza ArchiSim (*Las de ArchiSim* vuelve a ellas):

| Pareja | Se comprueba | Holgura |
|---|---|---|
| Climatización – Estructura | sí | 50 mm |
| Fontanería – Estructura | sí | 30 mm |
| Electricidad – Estructura | sí | — |
| Climatización – Fontanería | sí | 25 mm |
| Fontanería – Electricidad | sí | 30 mm |
| Climatización – Electricidad | sí | — |
| Urbanización – Arquitectura | no | — |
| Urbanización – Estructura | no | — |

Las parejas que no están en la tabla se comprueban sin holgura. Las reglas
cambiadas se guardan junto al proyecto (en el mismo fichero que los
vínculos). Son una propuesta para que Antonio las ajuste a lo que pide el
estudio.

Pendiente: interferencias entre dos vínculos y dentro del propio proyecto,
y una caja de sección en 3D.

## 4. Incidencias BCF (4.3)

*Colaboración → Incidencias (BCF)…*

- Abre BCF 2.1 y 3.0 de cualquier programa y guarda en 3.0 (o 2.1 para los
  que aún no leen la 3.0).
- Cada incidencia: tipo, estado, prioridad, etiquetas, fechas, asignada a,
  fase, descripción, comentarios y vistas. Un clic aplica su vista: cámara,
  elementos seleccionados (del proyecto o de los vínculos) y aislamiento.
- Se puede cambiar el estado, asignar y comentar (con el nombre del usuario
  del ordenador como autor).
- **Nueva incidencia desde la vista 3D**: guarda la cámara, lo seleccionado
  (también un elemento vinculado pinchado) y una imagen.
- Lista de valores en español por defecto: tipos (Interferencia, Consulta,
  Error, Propuesta, Requisito), estados (Abierta, En curso, Resuelta,
  Cerrada), prioridades (Alta, Normal, Baja), etiquetas por disciplina y fases
  (Anteproyecto, Proyecto básico, Proyecto de ejecución, Obra). Un BCF de otro
  programa conserva las suyas.

Validación: lee y reescribe idénticos los 38 ficheros de prueba de
buildingSMART; lo que escribe valida contra sus esquemas XSD de BCF 3.0
(`scripts/validate_bcf.py`).

Pendiente: líneas y planos de corte en la vista (se leen y se guardan, no se
dibujan), BCF API (servidores BIMcollab o Trimble Connect, en línea), marcas
sobre la imagen.

## 5. Requisitos IDS (4.4)

*Colaboración → Comprobar requisitos (IDS)…*

- Un IDS dice qué información pide el cliente: qué clases, qué propiedades
  con qué valores (y tipos y unidades), materiales, clasificaciones y dónde
  está cada elemento. ArchiSim comprueba el proyecto tal como sale en su IFC
  y dice, por especificación, cuántos elementos afecta y cuáles fallan y por
  qué («Pset_WallCommon no tiene la propiedad ThermalTransmittance»). Un clic
  en un fallo selecciona el elemento. *Pasar a incidencias*: una por
  especificación incumplida.
- También desde la línea de órdenes, con cualquier IFC (de ArchiSim o de otro
  programa): `archisim comprobar-ids modelo.ifc requisitos.ids [incidencias.bcf]`.

Validación: pasa los **334 casos de prueba oficiales** de buildingSMART
(`scripts/ids_testcases.sh`) y da los mismos resultados que IfcOpenShell
(ifctester) con la vivienda de prueba.

Pendiente: **bSDD** (clasificaciones del diccionario de buildingSMART en los
tipos), un editor de IDS dentro del programa.

## 6. Línea de órdenes

```
archisim comprobar-ids <modelo> <requisitos.ids> [incidencias.bcf]
archisim comparar <version-anterior> <version-nueva>
archisim interferencias <modelo> <vinculado.ifc>... [--tolerancia mm] [--bcf incidencias.bcf]
archisim bcf <incidencias.bcf>
archisim demo-coordinacion [carpeta]
```

Un modelo puede ser un proyecto `.archisim` o un IFC de cualquier programa.
`demo-coordinacion` deja la vivienda de prueba con una estructura
(pilares 30×30, vigas) y unas instalaciones (conducto 40×30, bajante Ø110)
con interferencias a propósito, y requisitos IDS de ejemplo.

## 7. Piezas (para quien programe)

| Crate | Qué hace |
|---|---|
| `archisim-ids` | Lector IDS y comprobación sobre cualquier fichero STEP (`step_read`); tabla de atributos de IFC2X3/4/4.3 generada con `scripts/make_ifc_schema.py` |
| `archisim-bcf` | Modelo, lectura y escritura de BCF 2.1 y 3.0; ZIP propio sobre `flate2` |
| `archisim-coord` | Mallas de cualquier IFC (`mesh`), elementos con huecos y huella de datos (`elements`), versiones (`versions`), interferencias (`clash`), BCF desde interferencias e IDS (`issues`) |
| `archisim-app` | `coord` (estado: vínculos, interferencias, incidencias, requisitos; fichero junto al proyecto) y `app/coord_ui` (menú, ventana, planta y 3D) |

## 8. Decisiones para Antonio

| # | Decisión | Propuesta |
|---|---|---|
| D13 | Dónde guardar los vínculos e incidencias: junto al proyecto (hoy) o dentro del `.archisim` | Dentro, en la próxima versión del formato, cuando las ramas abiertas estén en `main` |
| D14 | BCF por defecto al guardar: 3.0 o 2.1 | 3.0 (BIMcollab, Solibri, ArchiCAD 27+); 2.1 a mano para Revit con el BCF Manager antiguo |
| D15 | Tolerancia por defecto de interferencias | 10 mm (ajustable) |
| D16 | Orden de lo que queda: CTE (4.7, cuando los locales estén en `main`), puente con Revit (4.5), trabajo en equipo (4.6), holguras y reglas por disciplina | 4.7 → holguras → 4.6 → 4.5 |
