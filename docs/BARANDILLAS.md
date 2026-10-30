# Barandillas con tipo (E12)

Hasta ahora una barandilla era siempre la misma: montantes, pasamanos, una baranda abajo y barrotes a menos de 10 cm, o paneles de vidrio. Desde la v0.75 cada barandilla puede tener un **tipo**, como un muro o una puerta: el tipo dice de qué está hecha y la barandilla, por dónde va, su altura y cada cuánto lleva montantes.

## El tipo

Es un tipo de la familia de sistema «Barandilla» (`families/objetos/barandilla.asfam`), así que se edita como los de los muebles: tarjetas con su dibujo en 3D y muestras de los materiales. Lleva:

- **Modelo 3D**: las ocho colecciones (nórdico cálido, mediterráneo, años 50, japandi, contemporáneo, rústica, industrial, clásica).
- **A tu gusto**, cada opción empezando en «Como la colección»:
  - **Montantes**: cuadrado, redondo, pletina o sin postes (el vidrio va en un perfil en U a lo largo de toda la barandilla).
  - **Pasamanos**: rectangular, redondo, pletina, madera moldurado o sin pasamanos.
  - **Relleno**: barrotes, barrotes y pletina, balaustres torneados, vidrio, vidrio con pinzas, chapa perforada, chapa lisa, lamas horizontales, cables, malla o forja con volutas.
  - **Montante de arranque** (en los extremos libres): igual que los demás, más grueso, con bola o pilastra.
  - **Anclaje**: placa al suelo, al canto (el montante baja por el frente del forjado con su placa) o empotrado.
  - **Hueco mayor entre barrotes**: 10 cm por el CTE (DB-SUA 1), y lo cumplen también los balaustres y las lamas.
- **Materiales**: montantes y estructura, pasamanos, relleno, anclajes.

La plantilla trae once tipos: uno por colección y «Barandilla de vidrio con pinzas», «de chapa perforada» y «de malla».

## Cómo se construye

El generador (`archisim_props::fine::railing`) hace un tramo recto cada vez; ArchiSim pone uno en cada segmento de la línea de la barandilla. En una esquina, el tramo que empieza en ella pone el montante y el pasamanos de los dos se solapan. Los montantes caen donde caían antes (`Railing::pieces`), así que la planta, las cotas y las mediciones no cambian.

En el IFC sigue siendo un `IfcRailing` (`GUARDRAIL`) con su cuerpo triangulado, ahora un conjunto de caras por material, y el nombre del tipo en `ArchiSim_Railing`. El fichero `.archisim` guarda el tipo de cada barandilla (versión 43); las barandillas de antes no tienen tipo y se ven igual que siempre.

## Lo que falta

- Las barandillas de las escaleras con tipo (hoy siguen siendo las de la escalera).
- Un taller de balaustres y barrotes a medida (perfil del balaustre dibujado, patrón de barrotes), como el de las hojas de puerta (`docs/CARPINTERIA.md` §4).
- En planta, los montantes de las barandillas sin postes se dibujan igual que los demás.
