# Muros de sección variable: talud y zarpa

Muros de contención y de mampostería (piedra, tapial) cuyas caras no van a plomo: más gruesos abajo, con la cara exterior, la interior o las dos en talud, y con la zapata corrida («zarpa») como parte del propio muro. Bloque de la entrada 133 de la bitácora; formato `.archisim` versión 100.

## 1. Qué es

- **El tipo manda el espesor en coronación.** Las capas del tipo de muro son las de arriba. Bajando, cada cara en talud sale hacia fuera lo que diga su talud por metro de altura, y la capa que crece (la estructural, salvo que se elija otra) absorbe la diferencia. Las demás capas (enfoscados, aislamientos) mantienen su espesor y siguen su cara.
- **Talud** de la cara exterior y de la interior, cada uno por separado. Se escribe como pendiente «1:10» (uno en horizontal por diez en vertical), como porcentaje «10 %», como ángulo de la cara con la vertical «5,7°», o se da el **espesor en la base** y el programa calcula el talud (repartido entre las caras como estuviera).
- **Zarpa**: zapata corrida en la base, dentro de la altura del muro: su **canto** desde la base del muro hacia arriba, y su **vuelo** por cada lado, medido desde el pie de la cara de ese lado. Es del material de la capa que crece. Las caras en talud arrancan de lo alto de la zarpa.
- **Del tipo o del muro.** La sección puede ser del tipo (Tipos y materiales → el tipo de muro → «Sección variable: talud y zarpa»), y la siguen todos sus muros, o propia de un muro (Propiedades → «Sección variable»), que manda sobre la del tipo. «Como el tipo» quita la propia. Un muro puede tener una sección «Constante» propia para no llevar la del tipo.
- Solo **muros rectos**.

**Valores por defecto** (al elegir «Talud» o «Zarpa» en la lista): talud exterior 1:10; zarpa de 50 cm de canto con 60 cm de vuelo exterior y 40 cm de vuelo interior.

## 2. Cómo se dibuja

- **3D**: cada capa es un sólido de caras planas entre el pie de las caras y la coronación (también hasta lo alto de un perfil en alzado, si lo tiene), y la zarpa un prisma. Las esquinas de los extremos se deslizan por la línea de su encuentro: dos muros en talud que se encuentran en L a inglete se siguen encontrando a cualquier altura, y la zarpa también va a inglete. Contra un muro de sección constante, en L o en T, el muro en talud se mete en el otro lo que sale (se solapan, no queda hueco). El perfil en alzado lo recorta, y las puertas y ventanas atraviesan el espesor que haya a su altura (también la zarpa, si llegan a ella).
- **Planta**: el muro se corta con el espesor que tiene a la altura del plano de corte. Por debajo, en línea discontinua, el pie de las caras en talud y el contorno de la zarpa. Las puertas y ventanas se dibujan con su marco en el espesor de coronación.
- **Secciones y alzados**: la sección trapecial de cada capa con su trama, y la zarpa debajo.
- **IFC**: `IfcWall` con su cuerpo como sólidos de caras planas (`IfcFacetedBrep`, uno por capa y otro para la zarpa), el conjunto de capas del tipo (con el espesor de coronación) y el pset `ArchiSim_WallSection` (talud exterior e interior, capa que crece, vuelos y canto de la zarpa, y los espesores en coronación y en la base), con el que ArchiSim lo vuelve a leer igual. Los huecos de puertas y ventanas atraviesan también lo que sale por el talud y la zarpa. Las cantidades (`Qto_WallBaseQuantities`) dan el volumen real.
- **Mediciones**: los muros de sección variable van en su propia partida, «… de sección variable», en **m³** con su volumen real (menos puertas y ventanas), en lugar de los m² de los demás muros.

## 3. Cálculo

En Colaboración → Cálculo de la estructura, un muro exterior con tierras contra su cara exterior y sin forjado encima es un muro de contención en ménsula. Si es de sección variable con zarpa:

- **Vuelco** sobre la puntera (≥ 1,8), **deslizamiento** con rozamiento tg(⅔φ) (≥ 1,5) y **presión en el borde** (≤ 1,25 σadm), con el peso real del alzado en talud y de la zarpa (su centro de gravedad), la zarpa descentrada (puntera del lado interior, talón del lado de las tierras), las tierras sobre el talón y sobre la cara en talud, y la sobrecarga de 10 kN/m² encima.
- Las tierras cuentan desde lo alto de la zarpa; la flexión y el cortante del alzado, con el espesor al pie de las caras.
- El peso del muro sale del peso por m² de su tipo dividido entre su espesor (25 kN/m³ en hormigón armado).

## 4. Ejemplo

`archisim muros-talud out/` hace:

- en el sótano (−3,00), un muro de contención de hormigón de 30 cm en coronación, 3,50 m de alto, en talud 1:10 por la cara de las tierras y con zarpa de 50 cm de canto (80 cm de talón y 50 de puntera), que dobla en L con otro igual, y un muro de ladrillo que acomete en T sobre su zarpa;
- en planta baja, un muro de mampostería de 45 cm de piedra entre un enfoscado de cal fuera y otro dentro, en talud 1:20 por las dos caras, con una puerta y una ventana, en L con un muro de ladrillo;
- `muros-talud.archisim`, `muros-talud.ifc`, las dos plantas y dos secciones a 1:50 en SVG, y la comprobación del muro de contención.

## 5. Lo que no hace (todavía)

- Muros curvos de sección variable.
- Contrafuertes, tacón bajo la zarpa y zarpas escalonadas.
- Los encuentros con forjados y cubiertas no se restan del muro en talud (se solapan), ni los muros que acometen a él (también se solapan).
- Las puertas y ventanas se colocan y dibujan con el espesor de coronación (el hueco sí atraviesa todo).
- Las caras entre capas en los cortes de la vista 3D (se ve el otro lado del muro).
- La fachada pieza a pieza (E9) y el entramado ligero no se colocan sobre caras en talud.
- La biblioteca de tipos (`.astipos`) no guarda la sección del tipo; el IFC la guarda en cada muro y al importar la sección es propia de cada muro.
- El descenso de cargas usa el peso por m² del tipo (el de coronación) en los muros de sección variable.
