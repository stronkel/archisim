# Cálculo de la estructura

ArchiSim calcula la estructura del modelo tal como está dibujado. No hay que dibujar otro modelo: no se añaden elementos ni se guarda nada aparte de dos cosas, los datos del emplazamiento y las cargas puestas a mano (formato `.archisim` versión 81).

Es una ayuda al cálculo, pensada para que el estudio no necesite otro programa en las obras habituales (viviendas, entramados ligeros, pórticos de hormigón sencillos). El anejo lo firma el técnico, que revisa y asume sus hipótesis.

Código: crate `archisim-struct`, que depende solo de `archisim-core` y `archisim-geom`.

## 1. Normativa

- **Bases y acciones**: CTE DB-SE (combinaciones 4.2, coeficientes de las tablas 4.1 y 4.2) y DB-SE-AE (sobrecargas de uso, nieve y viento).
- **Acero**: DB-SE-A, y para la chapa conformada en frío EN 1993-1-3 y EN 1993-1-5.
- **Madera**: DB-SE-M y EN 1995-1-1.
- **Fábrica**: DB-SE-F.
- **Cimientos**: DB-SE-C.
- **Fuego**: DB-SI 6.
- **Sismo**: NCSE-02.
- **Hormigón armado**: Código Estructural.

## 2. Emplazamiento y acciones

Colaboración → **Cálculo de la estructura…**:

- **Altitud**: si no se da, la de la georreferencia del proyecto.
- **Nieve**: la zona de clima invernal.
- **Viento**: zona A, B o C y grado de aspereza del entorno (I a V).
- **Sismo**: ab/g, K, C e importancia de la construcción.
- **Terreno**: presión admisible.

De dónde salen las acciones:

- **Sobrecargas de uso** (tabla 3.1): del uso de cada local, y donde no hay local, del uso del edificio. Las cubiertas no transitables llevan la carga de mantenimiento.
- **Pesos propios**: de las capas de cada tipo de muro, forjado y cubierta. Si el tipo tiene entramado, se cuentan sus piezas.
- **Nieve**: tabla E.2 según zona y altitud, con el coeficiente de forma de la pendiente.
- **Viento**: presión qb de la zona; ce de la tabla D.2 a la altura de cada planta; cp de la tabla D.3.
- **Sismo**: NCSE-02, método simplificado (§5).

Las cargas puestas a mano (puntuales, lineales o por superficie) se suman a su caso: permanente, uso, nieve o viento.

## 3. Descenso de cargas

1. Cada forjado y cubierta se trocea en celdas de 25 cm.
2. Cada celda manda su carga a lo que la sostiene:
   - en los entramados, en la dirección de las viguetas o los cabios;
   - en el resto, al apoyo más cercano;
   - lo que cae justo encima de un apoyo va directo a él.
3. Muros, vigas y pilares bajan las cargas planta a planta hasta el terreno.
4. Al final se comprueba el equilibrio: lo puesto tiene que llegar al terreno.

## 4. Pórtico de pilares y vigas

- Se calcula en 3D, en régimen elástico lineal y en primer orden, por el método de la rigidez.
- Cada viga se parte en 4 piezas por vano y cada pilar en 2.
- Uniones:
  - en hormigón, rígidas;
  - en acero y madera, articuladas en las vigas;
  - los pies de pilar, empotrados.
- El viento y el sismo van al pórtico solo en las direcciones en las que no hay muros de carga.
- Resultados: envolventes de esfuerzos, flechas (integridad, confort y apariencia; DB-SE 4.3.3) y reacciones.

## 5. Sismo (NCSE-02)

Se aplica si la construcción no es de importancia moderada y ab ≥ 0,04·g.

- **Aceleración de cálculo**: ac = S·ρ·ab, con ρ = 1,3 si la importancia es especial y 1,0 si es normal. S según 2.2 a partir de C.
- **Periodo fundamental**:
  - pórticos de hormigón: T = 0,09·n;
  - muros: T = 0,06·n·√(H/(2L+H))·√(H/(L+H)).
- **Espectro**: α(T) según 2.3, con TA = K·C/10 y TB = K·C/2,5.
- **Ductilidad**: μ = 2 en pórticos y entramados, μ = 1 en fábrica; β = 1/μ.
- **Masas por planta**: G + ψ·Q, con ψ = 0,5 en viviendas, hoteles y oficinas, 0,6 en locales públicos y 1 en aparcamientos.
- **Fuerza de cada planta**: ac·α·β·ηk·peso, con el primer modo Φ = sen(π·h/2H).

Dónde actúa:

- **En el pórtico**: casos «Sismo X» y «Sismo Y», con sus combinaciones sísmicas en los dos sentidos.
- **En los muros arriostrados de los entramados**: se toma el mayor entre el sismo y el viento mayorado.

Límite: el método simplificado vale para edificios regulares (hasta 20 plantas en pórticos, 6 en muros). No se aplican las reglas de detalle del capítulo 4 de la norma.

## 6. Comprobaciones

De cada elemento se da la pieza o la sección más desfavorable de cada regla, con su aprovechamiento (1,00 es el límite) y, si no cumple, una propuesta.

### Entramados ligeros (steel frame y madera)

- **Viguetas y cabios**: como viga continua (flexión, cortante, abolladura del alma en los apoyos, flecha con fluencia, vibración).
- **Montantes**: compresión con pandeo y viento.
- **Dinteles, jambas y durmientes**.
- **Muros arriostrados**: EN 1995-1-1, 9.2.4.2, método A, con sus anclajes de tracción.
- **Fuego**: DB-SI 6, orientativo.

### Hormigón armado (HA-25, B500S)

- **Vigas** (sección rectangular):
  - flexión con bloque rectangular y x/d ≤ 0,45;
  - cortante (Vrd,c, cercos y bielas);
  - cuantía mínima.
- **Pilares**: compresión con flexión esviada, por compatibilidad de deformaciones, con la excentricidad mínima (h/20 o 2 cm), la cuantía mínima (0,4 %) y la máxima (4 %).
- Si el elemento tiene **armado dibujado** (Estructura → Armado), se compara con él. Si no, se da el que pide, por ejemplo «abajo 4Ø16, arriba 3Ø12, cercos Ø8 c/24».
- **Losas y forjados de hormigón**: se calculan con un **emparrillado**:
  - barras cada 50 cm o más, en la dirección del primer lado del contorno y en la perpendicular, cada una con la inercia y la torsión de su franja;
  - apoyos en los muros, vigas y pilares de debajo;
  - el armado sale por Wood–Armer (m ± |mxy|);
  - la flecha a largo plazo es la elástica × 3 (fisuración y fluencia), contra L/400;
  - el armado se compara con la malla dibujada (dirección del primer lado y la perpendicular) o se propone.

### Muros de fábrica

DB-SE-F 5.2. La carga del metro más cargado en la base se compara con Φ·t·fd:

- excentricidad de 0,05·t más la accidental hef/450;
- esbeltez máxima 27;
- fk 4 N/mm² y γM 2,5.

### Cimentación

- **Zapatas aisladas**: bajo cada pilar.
- **Zapatas corridas**: bajo cada muro de carga.
- Carga de servicio centrada, más un 10 % por el peso de la zapata y la tierra.
- Se compara con la presión admisible del emplazamiento. Si no hay zapata dibujada, se da su tamaño.
- **Zapata rígida**: vuelo ≤ 2 cantos.

## 7. Dónde se ve

- **Ventana Cálculo de la estructura** (Colaboración): el emplazamiento, todos los resultados en texto y el botón «Anejo de cálculo (PDF)…».
- **Comprobación CTE**:
  - los entramados, en «Entramado ligero»;
  - el hormigón, la fábrica y la cimentación, en «Estructura (Código Estructural, DB-SE-F, DB-SE-C)».
  - Cada línea selecciona su elemento.
- **Anejo de cálculo** (Archivo → Exportar, o `archisim estructura <proyecto> <carpeta>`), en A4 y también en texto. Capítulos:
  1. Normativa y método
  2. Materiales
  3. Acciones (emplazamiento, forjados y cubiertas, viento, cargas a mano y sismo)
  4. Combinaciones
  5. Descenso de cargas
  6. Pórtico
  7. Comprobación de los elementos
  8. Alcance y limitaciones

## 8. Lo que no hace (todavía)

- Segundo orden (P-Δ), plasticidad y pandeo global del pórtico.
- Punzonamiento de losas en los pilares.
- Uniones de acero y madera, y anclajes al cimiento.
- Armado de zapatas a flexión. Zapatas flexibles, losas de cimentación y muros de sótano con empujes.
- Hormigón distinto de HA-25 y acero distinto de B500S. Secciones de hormigón que no son rectangulares.
- Análisis modal espectral (el sismo es el método simplificado) y las reglas de detalle sísmicas.
