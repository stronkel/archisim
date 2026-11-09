# Vivienda modular por cajas (steel frame)

> 9 de noviembre de 2026 · Propuesta para Antonio, **sin programar todavía**. Sale de su pregunta sobre qué le falta a ArchiSim para proyectar viviendas modulares de un fabricante con el que trabaja AD5, y del IFC de uno de sus modelos (exportado de Revit; no se sube al repositorio).

## 1. El sistema

- **Cajas completas** hechas en taller, que van en camión y se colocan en obra.
- Módulos de **hasta 9 × 3,60 m**. Los habituales miden **6,20 × 3,60 m**.
- Estructura de acero con perfiles de catálogo de fabricante (en ArchiSim, como fichero del estudio: `docs/ENTRAMADOS.md` §3 ter) y soldadura de taller.
- Las cajas apoyan sobre pilares cortos de hormigón, encima de una losa, con zócalo perimetral de bloque (forjado sanitario).

## 2. Qué dice el modelo de ejemplo (IFC de Revit, IFC2X3)

**La vivienda.** Planta en L con 8 cajas:
- 6 en fila y 2 en el ala;
- paso de 3,60 m entre ejes de caja;
- unos 6,1 m de largo de suelo.

**Cómo está modelado:**
- **Cada capa del cerramiento es un muro distinto**: placa cerámica de 10 mm, membrana, placa con aislamiento de 51 mm, lana de 100 mm y placa interior de 2 × 12,5 mm. Son 195 «muros» de placa cerámica.
- **Montantes:** solo 70, colocados a mano como pilares de 40 × 100 mm. Llevan un nombre de pilar genérico y material «aluminio».
- **Lo que no está dibujado:**
  - canales;
  - chasis de suelo y de techo;
  - estructura de suelo y de cubierta de las cajas (solo los acabados y el remate de los petos).
- **El IFC no dice qué pertenece a cada caja:** no hay agrupaciones.
- **Tampoco hay locales** (`IfcSpace`).
- **Cimentación:**
  - losa de 45 cm;
  - 27 pilares de hormigón de 25 × 25 × 60 cm;
  - zócalo de bloque de 60 cm.
- **Exterior:** un porche con forjado de hormigón de 30 cm, y piscina y terraza.

**Lo que hace ArchiSim hoy con ese IFC** (`archisim importar`):
- Lo abre en menos de un segundo. Entran 508 muros, 36 forjados, 28 carpinterías, 97 pilares y los ejes.
- La comprobación CTE da 204 «no cumple». Casi todos son las capas sueltas tomadas por muros de fábrica de 10 mm que cargan. No es un fallo del proyecto: el modelo está pensado para dibujar, no para calcular.
- Sin locales no hay ocupación, ventilación ni HE 1.
- Los ejes llegan cuatro veces (uno por planta en el fichero). Hay que juntarlos al importar.

## 3. Lo que hay que añadir

**M1 · La caja como elemento.** «Módulo»:
- un rectángulo (o contorno) en planta con su altura, sus muros, su suelo y su techo;
- su marca («MOD-03»), su contorno de transporte y aviso si se pasa de lo que admite el camión;
- en el IFC, un `IfcElementAssembly` con sus piezas;
- colocado con forzado a la retícula de cajas, y copiable.

**M2 · Juntas entre cajas.** Donde se tocan dos cajas:
- muro doble y forjado doble (suelo de una, techo de la de abajo) con la holgura del tipo;
- los muros de cajas distintas no se unen;
- el despiece del entramado sale por caja.

**M3 · Chasis, peso e izado.**
- Bastidor de suelo y de techo y pilares de esquina, con perfiles más pesados que los montantes y uniones soldadas.
- Peso y centro de gravedad de la caja terminada.
- Puntos de izado y caso de carga del izado.
- Orden de montaje.

**M4 · Del IFC por capas a muros con tipo.** Al importar, «Juntar capas apiladas en un muro»: los muros finos paralelos y pegados se convierten en un muro con su tipo multicapa, y el entramado sale solo en su capa de estructura. Sirve para traer los modelos que ya están en Revit.

**M5 · Envolvente con los montantes.** HE 1 debe contar el puente térmico de los montantes de acero en la U del muro, del suelo y del techo (hoy cuenta las capas como continuas).

**M6 · Instalaciones en las juntas y máquina.**
- Tuberías cortadas en la junta, con su conexión.
- Pasos por los montantes.
- Fichero para la perfiladora, cuando se sepa cuál es.

## 4. Buenas prácticas que el programa debe ayudar a seguir

- **La retícula de cajas manda.** Muros de carga y esquinas alineados en vertical; nada que apoye a media luz de una caja.
- **Huecos fuera de juntas y esquinas, en múltiplos de la separación de montantes.**
- **Cuartos húmedos agrupados**, a ser posible en una caja con su patinillo.
- **Juntas resueltas en el tipo:** acústica (DB-HR), fuego (DB-SI), estanqueidad y tolerancias. La junta que se cierra en obra se detalla primero.
- **Ambiente marino:** protección de los perfiles, tornillos y conectores acorde.
- **Aislamiento continuo por fuera de los montantes.**
- **Un solo modelo** del que salen los planos, el despiece y la máquina, con el diseño congelado antes de fábrica.

## 5. Pendiente de Antonio

- ¿El orden M4 → M1 → M2 → M5 → M3 → M6 le vale? M4 primero, para trabajar ya con los modelos que tiene.
- Catálogo de perfiles del fabricante como fichero (`.csv`, §3 ter de ENTRAMADOS).
- Sección del chasis y de los pilares de esquina, y cómo se unen las cajas entre sí.
- Holgura entre cajas y si los muros de junta son dobles completos o uno de ellos sin placa.
- Qué es el forjado de hormigón de 30 cm del porche.
