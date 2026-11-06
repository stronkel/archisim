# Cálculo de la estructura

ArchiSim calcula la estructura del modelo tal como está dibujado. No hay que dibujar otro modelo: no se añaden elementos ni se guarda nada aparte de tres cosas, los datos del emplazamiento, los materiales de la estructura y las cargas puestas a mano (formato `.archisim` versiones 81 y 82).

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
- **Hormigón armado**: Código Estructural y EN 1992-1-1.
- **Uniones de acero**: EN 1993-1-8.

## 2. Emplazamiento y acciones

Colaboración → **Cálculo de la estructura…**:

- **Altitud**: si no se da, la de la georreferencia del proyecto.
- **Nieve**: la zona de clima invernal.
- **Viento**: zona A, B o C y grado de aspereza del entorno (I a V).
- **Sismo**: ab/g, K, C e importancia de la construcción.
- **Terreno**: presión admisible, ángulo de rozamiento φ y peso γ (para los muros de sótano).
- **Materiales**: hormigón (HA-25 a HA-50), acero de armar (B400S, B500S), acero laminado (S235, S275, S355), madera de vigas y pilares (C18, C24, C30, GL24h, GL28h, GL32h) y resistencia de la fábrica fk (DB-SE-F, tabla 4.4). Por defecto, HA-25, B500S, S275, C24 y 4 N/mm². Los entramados ligeros siguen en C24 y S350GD. **Uniones de acero**: articuladas (por defecto) o rígidas (chapa de testa atornillada); con rígidas, las vigas de acero se empotran en los pilares en el pórtico.

De dónde salen las acciones:

- **Sobrecargas de uso** (tabla 3.1): del uso de cada local, y donde no hay local, del uso del edificio. Las cubiertas no transitables llevan la carga de mantenimiento.
- **Pesos propios**: de las capas de cada tipo de muro, forjado y cubierta. Si el tipo tiene entramado, se cuentan sus piezas.
- **Nieve**: tabla E.2 según zona y altitud, con el coeficiente de forma de la pendiente.
- **Viento**: presión qb de la zona; ce de la tabla D.2 a la altura de cada planta; cp de la tabla D.3.
- **Sismo**: NCSE-02, método simplificado (§5).

Las cargas puestas a mano (puntuales, lineales o por superficie) se suman a su caso: permanente, uso, nieve o viento.

## 3. Descenso de cargas

Los muros y pilares de la planta más baja atraviesan la solera hasta su zapata; solo una losa de cimentación los recoge (un forjado sobre el terreno que se llama «cimentación» o que tiene 30 cm o más de hormigón).


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
- **Desplomes** con el viento: el de cada planta contra h/250 y el total contra H/500 (DB-SE 4.3.3.2).
- **Segundo orden**: el índice de estabilidad θ = P·Δ/(V·h) de cada planta con el viento y el sismo (EN 1993-1-1, 5.2.1). Por debajo de 0,10 se desprecia. Entre 0,10 y 0,30 el cálculo **amplifica** el viento y el sismo de esa dirección por k = 1/(1 − θ) y vuelve a resolver el pórtico con ellos (EN 1993-1-1, 5.2.2(5)): todos los esfuerzos y comprobaciones ya llevan el segundo orden. Por encima de 0,30, el pórtico necesita arriostramiento.

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

**Análisis modal espectral** (NCSE-02, 3.6.2), cuando el pórtico toma el sismo:

- las masas de cada planta se reparten en sus nudos;
- los modos salen por iteración en subespacio sobre la rigidez del pórtico (seis), con su periodo y la masa que mueven en X y en Y;
- cada modo lleva la aceleración del espectro en su periodo;
- los cortantes de planta de los modos se combinan por la raíz de la suma de cuadrados, y las fuerzas de planta que dan esos cortantes sustituyen a las del método simplificado.

Comprobado contra el oscilador de un grado de libertad: una planta con cuatro pilares da T = 2π·√(m/k) y la fuerza del espectro en ese periodo.

Límite: no se aplican las reglas de detalle del capítulo 4 de la norma.

## 6. Comprobaciones

De cada elemento se da la pieza o la sección más desfavorable de cada regla, con su aprovechamiento (1,00 es el límite) y, si no cumple, una propuesta.

### Entramados ligeros (steel frame y madera)

- **Viguetas y cabios**: como viga continua (flexión, cortante, abolladura del alma en los apoyos, flecha con fluencia, vibración).
- **Montantes**: compresión con pandeo y viento.
- **Dinteles, jambas y durmientes**.
- **Muros arriostrados**: EN 1995-1-1, 9.2.4.2, método A, con sus anclajes de tracción.
- **Fuego**: DB-SI 6, orientativo.

### Acero laminado (DB-SE-A, EN 1993-1-1)

- Perfiles en I y H, tubos rectangulares y circulares, macizos; UPN, T y angulares en elástico.
- Compresión con pandeo por flexión (curvas a, b y c según el perfil), flexión en los dos ejes, cortante e interacción lineal.
- **Vuelco lateral** de las vigas de perfil abierto que no tienen un forjado encima (C1 1,13).
- Los pilares se suponen intraslacionales: el desplazamiento del pórtico se mira en el segundo orden.

### Madera de vigas y pilares (DB-SE-M, EN 1995-1-1)

Secciones rectangulares de madera aserrada o laminada, de la clase elegida: flexión, cortante, compresión con pandeo (βc 0,2 o 0,1) y tracción, con el kmod de cada combinación.

### Uniones de acero (EN 1993-1-8)

- **Placas de anclaje** de los pilares de acero que apoyan en una zapata: placa 100 mm más ancha que el perfil por cada lado, compresión del hormigón con fjd = ⅔·1,5·fcd, espesor para esa presión, pernos 4.6 para la tracción que deja el momento y cortante por rozamiento y por los pernos.
- **Uniones articuladas** de las vigas en I: una chapa con una fila de tornillos 8.8 a cortante simple, los que pida el cortante del apoyo y si caben en el alma.
- **Uniones rígidas** (si en los materiales del cálculo se eligen «Uniones de acero: rígidas»; entonces el pórtico de acero se calcula empotrado): chapa de testa atornillada. El momento del extremo de la viga se toma como un par entre sus alas; una fila de dos tornillos 8.8 junto al ala traccionada, o dos filas con la chapa extendida; el cortante, con los dos tornillos de abajo. Se comprueba además el **panel del alma del pilar** a cortante (EN 1993-1-8, 6.2.6.1). Si no basta con dos filas, se pide una cartela; si no basta el alma, rigidizadores y chapa de refuerzo.

### Uniones de madera (EN 1995-1-1)

- **Vigas**: colgadas de una chapa de acero de 10 mm embebida en una ranura, con pasadores de acero Ø12, Ø16 o Ø20 que trabajan a doble cortadura (modos f, g y h de Johansen, 8.2.3; aplastamiento fh,0,k = 0,082·(1 − 0,01·d)·ρk y, a 90° de la fibra, dividido por k90). Los pasadores, en una o dos columnas, a 3d entre sí, 4d del borde de arriba y 3d del de abajo; si no caben en el canto, no cumple. Se comprueba la **hienda** de la viga (8.1.4) con el pasador más alto.
- **Pies de los pilares**: un herraje en U sobre la zapata con chapa embebida y pasadores, que toman el levantamiento (a lo largo de la fibra) y el cortante (a través), cuatro pernos 4.6 al hormigón y la compresión de la testa del pilar (fc,0).

### Hormigón armado (Código Estructural)

- **Vigas** (sección rectangular):
  - flexión con bloque rectangular y x/d ≤ 0,45;
  - cortante (Vrd,c, cercos y bielas);
  - cuantía mínima.
- **Pilares**: compresión con flexión esviada, por compatibilidad de deformaciones, con la excentricidad mínima (h/20 o 2 cm), la cuantía mínima (0,4 %) y la máxima (4 %).
- **Pilares circulares**: la misma compatibilidad de deformaciones, con el hormigón integrado en franjas del círculo y las barras repartidas por la corona (seis por lo menos). Como un círculo no tiene eje débil, los dos momentos se suman como un vector. Si no hay armado dibujado, se propone, por ejemplo «8Ø16 en corona».
- **Esbeltez de pilares**: con λ por encima de λlim = 20·A·B·C/√n (A 0,7, B 1,1, C 0,7), se suma la excentricidad de segundo orden de la curvatura nominal (EN 1992-1-1, 5.8.8). Por encima de λ 100, no cumple.
- Si el elemento tiene **armado dibujado** (Estructura → Armado), se compara con él. Si no, se da el que pide, por ejemplo «abajo 4Ø16, arriba 3Ø12, cercos Ø8 c/24».
- **Losas y forjados de hormigón**: se calculan con un **emparrillado**:
  - barras cada 50 cm o más, en la dirección del primer lado del contorno y en la perpendicular, cada una con la inercia y la torsión de su franja;
  - apoyos en los muros, vigas y pilares de debajo;
  - el armado sale por Wood–Armer (m ± |mxy|);
  - la flecha a largo plazo es la elástica × 3 (fisuración y fluencia), contra L/400;
  - el armado se compara con la malla dibujada (dirección del primer lado y la perpendicular) o se propone;
  - **punzonamiento** en cada pilar (art. 46): perímetro u1 a 2d, β 1,15 en pilares interiores, 1,4 de borde y 1,5 de esquina, contra vRd,c con la armadura superior; y la cara del pilar contra 0,5·ν·fcd;
  - si no basta el hormigón, y no se pasa de 1,5·vRd,c, se propone la **armadura de punzonamiento** (art. 46.4): perímetros de cercos o pernos cada 0,75·d, con cuántas ramas y de qué diámetro, hasta el perímetro exterior donde ya no hace falta. Sale en ámbar (cumple con esa armadura). Si se pasa de 1,5 veces, no cumple: más canto, ábaco o capitel.
- **Losas de cimentación**: el mismo emparrillado sobre muelles (balasto ks = 120·σadm). Cargas: los muros y pilares que se apoyan en ella. Resultados: presión máxima en servicio contra la admisible, armado y punzonamiento.

### Muros de fábrica

DB-SE-F 5.2. La carga del metro más cargado en la base se compara con Φ·t·fd:

- excentricidad de 0,05·t más la accidental hef/450;
- esbeltez máxima 27;
- fk de los materiales del proyecto y γM 2,5.

### Cimentación

- **Zapatas aisladas**: bajo cada pilar.
- **Zapatas corridas**: bajo cada muro de carga.
- Carga de servicio centrada, más un 10 % por el peso de la zapata y la tierra.
- Se compara con la presión admisible del emplazamiento. Si no hay zapata dibujada, se da su tamaño.
- **Zapata rígida**: vuelo ≤ 2 cantos. **Zapata flexible** (vuelo mayor): se calcula a flexión en la cara del pilar o del muro y a cortante a un canto de la cara (sin cercos).
- **Armado de las zapatas** (art. 58): flexión en una sección 0,15 del pilar o del muro por dentro de su cara, con la reacción del terreno en la combinación última (1,35 G + 1,5 lo demás). Se compara con la malla dibujada o se propone.

### Muros de sótano

- Los muros exteriores con tierras contra ellos: la altura de tierras sale del terreno donde lo hay y de ±0,00 donde no.
- Empuje al reposo K0 = 1 − sen φ con el peso del terreno y 10 kN/m² de sobrecarga.
- Si un forjado apoya arriba, el muro está apeado ahí; si no, trabaja en ménsula.
- Flexión en la base y en el vano, armadura de cada cara y cortante sin cercos. Si el muro no es de hormigón, aviso.
- **Muros de contención exentos** (sin forjado arriba): empuje activo Ka = (1 − sen φ)/(1 + sen φ) sobre las tierras y la zapata. Con la zapata corrida dibujada bajo el muro (centrada), se comprueban el **vuelco** sobre la punta (≥ 1,8), el **deslizamiento** con rozamiento tg(⅔φ) (≥ 1,5) y la **presión en el borde** (≤ 1,25 de la admisible). Sin zapata dibujada, se propone el ancho que pide.

### Reglas de construcción sísmicas (NCSE-02, capítulo 4)

Cuando hay que tener en cuenta el sismo, con ductilidad baja (μ = 2) y contra el armado dibujado:

- **Vigas** (ac ≥ 0,16g): dos Ø16 por lo menos en cada cara en toda la longitud, cuantía de las dos caras del 4 ‰ y cercos a 10 cm como mucho junto a los apoyos.
- **Pilares** (ac ≥ 0,12g): lado menor de 25 cm (30 con ac ≥ 0,16g); tres barras por cara (seis si es circular) a 20 cm como mucho (15); cuantía entre 1 % y 6 % con ac ≥ 0,16g; en las **zonas críticas** de los dos extremos, de dos cantos de largo, cercos Ø6 a c/3 y 10 cm con barras de 12–14 o 15 cm con barras de 16 o más (con ac ≥ 0,16g: Ø8, c/4, 7 y 10 cm).
- **Atado de la cimentación** (4.3.2): con ac ≥ 0,16g, cada zapata aislada atada con vigas de atado en dos direcciones; entre 0,08g y 0,16g, aviso: vale una solera continua de 15 cm al menos.

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

- **Plasticidad**: el cálculo es elástico, a propósito (es lo prudente para un programa de ayuda al proyecto).
- **Segundo orden**: se amplifica por 1/(1 − θ), no con un análisis no lineal; con θ > 0,30 se pide más rigidez.
- Muros de contención con la zapata descentrada (más punta o más talón), con tacón o con contrafuertes.
- Uniones de acero soldadas, con cartela calculada o atornilladas de más de dos filas; uniones de madera de clavija en la madera sin chapa, de tirafondos o de herrajes de catálogo.
- Secciones de hormigón en T, en L o huecas.
- Paneles de fábrica en zona sísmica (NCSE-02, 4.7) y ductilidades alta y muy alta (μ 3 y 4).
