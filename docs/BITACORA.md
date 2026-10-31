# Bitácora de ArchiSim

Registro de sesiones de trabajo, de la más reciente a la más antigua. Cada conversación nueva lo lee para saber dónde estamos, y añade su entrada al terminar.

Formato de cada entrada: fecha · bloque · qué se hizo · qué quedó pendiente · qué debe probar Antonio.

---

## 2026-10-31 (50) · v0.77: E2, E7, E9, E11, E13 y remates en `main`

Unidos #117 a #122: galerías visuales (E2), vistas de dibujo y calcos (E7), fachadas pieza a pieza (E9), hormigón armado (E11), instalaciones segunda vuelta (E13) y los remates de E15 y E6. Formato `.archisim` versión 51. Lo que hay que probar de cada uno está en sus entradas (44 a 49).

---

## 2026-10-31 (49) · Remates: anclajes en 3D y en alzado (E15) y hueco de muro en más carpinterías (E6)

Rama `bloque/remates-e15-e6` (sobre `bloque/e13-instalaciones`), con pull request. Sin cambios de formato.

**Hecho**
- **Mover en alzado y sección** (E15): en una pestaña de alzado o sección, arrastra un objeto, una puerta o una ventana.
  - Se mueve a lo largo de la sección y en altura.
  - Sus esquinas, los medios de sus lados y su centro se pegan a las esquinas de lo dibujado (con un círculo donde se pega) y a las líneas de las plantas.
  - Al soltar, un objeto cambia su sitio y su altura sobre la planta; una puerta o ventana, su posición en el muro y su antepecho.
- **Mover en 3D** (E15): el mueble que arrastras, si no lo ha pegado una pared, se pega por sus esquinas, medios y centro a los de los muebles de al lado (a menos de 5 cm).
- **Hueco de muro y accesorios** (E6) ya también en la **ventana de guillotina**, las **puertas correderas** (normal y empotrada) y la **puerta pivotante**, como en la practicable:
  - posición del marco y su retranqueo;
  - mocheta;
  - dintel visto;
  - recercado;
  - albardilla;
  - en la guillotina, además, alféizar por dentro y parasol de lamas.

  En el taller (`archisim taller out/`) hay un ejemplo de cada una en la fila de huecos de muro.

**Pendiente**
- En alzado y sección todavía no se mueven muros, pilares ni otros elementos, ni hay copiar ni simetría.

**Qué debe probar Antonio**
1. Abre la demo → Alzado Sur. Arrastra una ventana hacia arriba hasta que su dintel se pegue a la línea de la planta o a otra ventana; suelta. En planta y en 3D debe estar con su nuevo antepecho.
2. En un alzado, arrastra un objeto (una farola, un coche) por su esquina hasta la de otro.
3. En 3D, arrastra una mesa junto a otra: debe pegarse arista con arista.
4. Tipos y materiales → una ventana de guillotina → pestaña del hueco: ponle recercado moldurado, dintel de ladrillo y albardilla. Haz lo mismo con una puerta corredera y una pivotante.

---

## 2026-10-31 (48) · Instalaciones, segunda vuelta (E13)

Rama `bloque/e13-instalaciones` (sobre `bloque/e11-armado`), con pull request. Formato `.archisim` versión 51: hay más clases de aparato y circuitos dados a mano. Los ficheros anteriores se abren igual.

**Hecho**
- **Mecanismos eléctricos** con su símbolo de planta:
  - interruptor y **conmutador**;
  - enchufe y **base de 25 A** de la cocina;
  - **tomas de TV y de datos**;
  - **cuadro eléctrico**.

  Se colocan con las fichas Enchufe, Interruptor y Cuadro de Instalaciones; la clase concreta se elige en «Tipo».
- **Circuitos del REBT** (ITC-BT-25, tabla 1), solos:
  - C1 luces e interruptores;
  - C2 enchufes;
  - C5 los enchufes de baños, aseos y cocinas (por el local donde están);
  - C3 la base de 25 A.

  En Propiedades del aparato → «Circuito» se cambia (C4 para la lavadora, por ejemplo), con su magnetotérmico y su sección.
- **En planta**:
  - el número de circuito junto a cada mecanismo («C2»);
  - una línea discontinua que une los de cada circuito desde el cuadro.
- **Comprobación**, sección «Electricidad (REBT, ITC-BT-25)»: puntos de cada circuito por planta contra los máximos de la tabla (30 en C1, 20 en C2, 2 en C3, 3 en C4, 6 en C5).
- **Montantes y bajantes**:
  - tubería vertical entre plantas (fichas Montante y Bajante), con la altura que suben;
  - se dibujan en todas las plantas que cruzan («M Ø40», «B Ø110»);
  - los tramos que acaban en ellos llevan su curva hacia arriba o hacia abajo.
- **Codos con su radio**: 1,5 veces el diámetro en tuberías y el ancho en conductos. El tramo se corta donde empieza la curva.
- **IFC**:
  - cuadro como `IfcElectricDistributionBoard`;
  - tomas de TV y datos como `IfcOutlet`;
  - montantes y bajantes como `IfcPipeSegment`;
  - el circuito dado a mano se lee de vuelta.
- `archisim instalaciones out/`: una casa de dos plantas con todo esto.

**Pendiente**
- Esquema unifilar del cuadro.
- Curvas en los conductos de aire (siguen con codo de inglete).
- Los montantes no se unen todavía con tes a las tuberías que pasan por su lado.

**Qué debe probar Antonio**
1. `archisim instalaciones out/` y abre `out/instalaciones.archisim`:
   - en Planta baja mira los símbolos y los circuitos;
   - en 3D, los codos curvos y la bajante con la curva del desagüe del baño.
2. Pon un enchufe en un local llamado «Baño»: debe salir C5. Cámbialo a C4 en sus propiedades.
3. Pon 21 enchufes en una planta, en C2: la comprobación debe avisar.
4. Exporta el IFC y míralo en otro visor: cuadro, tomas, montante y bajante con su clase.

---

## 2026-10-31 (47) · Hormigón armado (E11)

Rama `bloque/e11-armado` (sobre `bloque/e9-fachadas`), con pull request. Formato `.archisim` versión 50: el proyecto guarda el armado. Los ficheros anteriores se abren igual, sin armado. Según D26, ArchiSim **dibuja y comprueba** el armado; **no calcula** la estructura.

**Hecho**
- **Armado** de pilares y vigas de hormigón (rectangulares o circulares), zapatas aisladas y corridas, y forjados horizontales: en Propiedades → **«Armado (hormigón armado)»** → «Lleva armado», escrito como en un plano:
  - **pilares**: recubrimiento, barras (número y Ø), cercos (Ø y separación) y, si se quiere, cercos más juntos en los dos extremos;
  - **vigas**: inferior y superior (número y Ø) y estribos;
  - **zapatas y forjados**: malla inferior en x y en y (Ø y separación) y, si se quiere, superior.

  Sale con valores habituales que ya cumplen (4Ø16 con cercos Ø8 c/15, Ø12 c/20 en zapatas, Ø10 c/20 en forjados…).
- **Las barras se colocan solas** dentro del hormigón, con su recubrimiento:
  - **pilares**: primero las esquinas, luego los lados más largos;
  - **vigas**: estribos repartidos;
  - **zapatas**: mallas con patillas;
  - **forjados**: mallas en la capa de estructura (no en el solado), cortadas en los bordes y en los huecos.
- **3D**: las barras en gris acero; se ven con la caja de sección.
- **Planta a 1:20 (hasta 1:25)**: las barras y el cerco de los pilares cortados.
- **Mediciones**, capítulo 14 «Acero en barras»: kg por diámetro. Cada línea es un grupo de barras de un elemento («Pilar 12 · cercos Ø8 c/15 (20 de 1,05 m)»).
- **Comprobación del armado** (Código Estructural, solo lo que no necesita cálculo): recubrimientos, diámetros mínimos, número y separación de barras, cuantías geométricas mínimas (y máxima en pilares) y diámetro y separación de cercos y estribos. Aparece:
  - en Propiedades («Comprobación del armado»: qué no cumple);
  - en la ventana de comprobación, sección «Armado (Código Estructural)».
- **IFC**:
  - cada grupo de barras es un `IfcReinforcingBar` (barras como sólidos de barrido), parte de su elemento;
  - lo escrito va en el elemento y se lee al importar.
- `archisim armado out/`: un pórtico con zapatas, vigas, zapata corrida y forjado con hueco, todo armado (708 kg, 102 comprobaciones que cumplen).

**Pendiente**
- **Decisión para Antonio**: confirmar los valores de las comprobaciones:
  - recubrimiento de 25 mm en interior y de 50 mm en cimentación con hormigón de limpieza;
  - cuantías mínimas de 4 ‰ en pilares, 2,8 ‰ en vigas, 1,8 ‰ en losas y 0,9 ‰ en zapatas;
  - separaciones máximas.
- La **sección** con las barras cortadas y el **despiece dibujado** (formas de doblado, marcas de barra).
- Esperas y solapes entre plantas, y refuerzos sobre apoyos.
- Pasar el **modelo analítico** a un programa de cálculo.
- Al copiar un elemento, su armado no se copia.

**Qué debe probar Antonio**
1. Ejecuta `archisim armado out/` y abre `out/armado.archisim`.
2. Vista 3D → Caja de sección. Baja la tapa hasta cortar los pilares y el forjado: deben verse las barras, los cercos y las mallas.
3. Selecciona un pilar → Propiedades → Armado: cambia a 6Ø12 y los cercos a c/30. «Comprobación del armado» debe decir qué no cumple, y la ventana de comprobación también.
4. Planta baja a 1:20: acércate a un pilar y mira sus barras y su cerco.
5. Mediciones, capítulo 14: kg por diámetro, con el despiece de cada elemento.
6. Exporta el IFC y míralo en otro visor: las barras son armaduras de su pilar, viga, zapata o forjado.

---

## 2026-10-31 (46) · Fachadas pieza a pieza (E9)

Rama `bloque/e9-fachadas` (sobre `bloque/e7-vistas-dibujo`), con pull request. Formato `.archisim` versión 49: los tipos de muro pueden llevar fachada. Los ficheros anteriores se abren igual, con acabados lisos.

**Hecho**
- **Fachada de un tipo de muro**: en Tipos y materiales → tipo de muro exterior → grupo **«Fachada: aparejo y despiece»**, todo en tarjetas dibujadas como quedaría la fachada:
  - **Sistema**: ladrillo cara vista, bloque de hormigón, panel prefabricado, fachada ventilada o aplacado. Cada uno trae su pieza y su junta habituales.
  - **Aparejo**: a soga, a tizón, inglés, flamenco, palomero o junta continua.
  - **Pieza**: largo, alto, tizón y junta, en milímetros, y el color de la junta (mortero; en la ventilada, junta abierta oscura).
  - **Dibujo**: tizones de otro color, hiladas de otro color o rombos, con su segundo material.
  - **Dinteles a sardinel** sobre puertas y ventanas.
  - **Frente de forjado**: sin pieza propia, plaquetas o ladrillo a sardinel, con su material y su vuelo.
- **Cómo se colocan las piezas**:
  - Hilada a hilada desde la cota 0, así que las hiladas casan entre muros y plantas.
  - Cada paño entre huecos empieza en la jamba: en las hiladas pares, con media pieza.
  - Las esquinas con otro muro con fachada se traban: en una hilada llega la soga de un muro y en la otra la del otro.
  - Se recortan donde la fachada se corta: cubiertas, perfiles de alzado, huecos en arco.
  - El frente de forjado sale solo donde la fachada pasa por delante del canto de un forjado (el que llega hasta la hoja exterior), sin muros ni niveles especiales.
- **En 3D y en la imagen realista**, cada pieza es un volumen 6 mm fuera de la cara, con la junta del color del mortero detrás. Se seleccionan con su muro y se recortan con él.
- **En planta a 1:20 (y hasta 1:25)**: las llagas de la hoja exterior en la hilada del plano de corte.
- **Mediciones**: capítulo 13 «Fachadas: piezas»: unidades por tipo y material, con enteras y cortadas de cada muro, y el frente de forjado aparte.
- **IFC**: piezas del muro (`IfcBuildingElementPart`) con su material, una por material y muro; la fachada del tipo se lee de vuelta al importar.
- **Plantilla**: ocho tipos nuevos: ladrillo visto a soga, inglés, flamenco con tizones oscuros y blanco con rombos; bloque de hormigón; paneles prefabricados; ventilada de placas; aplacado de piedra.
- `archisim fachadas out/` hace un bloque de dos plantas con cada uno.

**Pendiente**
- **Alzados y secciones**: siguen enseñando la capa lisa; queda dibujar el despiece en ellos a escala de detalle.
- Las jambas, el alféizar y el dintel por dentro del hueco no llevan piezas: se ve la capa con su textura.
- Los muros curvos siguen con el acabado liso.
- El palomero no deja huecos que atraviesen el muro: los huecos se ven del color de la junta.

**Qué debe probar Antonio**
1. Ejecuta `archisim fachadas out/` y abre `out/fachadas.archisim`. En 3D, acércate a cada bloque y mira:
   - el aparejo y las esquinas trabadas;
   - los dinteles a sardinel sobre las ventanas;
   - la banda del frente de forjado entre las dos plantas.
2. Abre Tipos y materiales → «Fachada ladrillo visto a soga 30» → grupo Fachada. Cambia el aparejo a flamenco, el dibujo a rombos y el frente de forjado a ladrillo a sardinel: el 3D lo sigue.
3. Pon uno de esos tipos a un muro de tu proyecto con forjados: el frente debe salir solo donde el forjado llega a la fachada.
4. Planta a 1:20: las llagas de la hoja exterior.
5. Mediciones, capítulo 13: piezas enteras y cortadas por muro.
6. Exporta el IFC y míralo en otro visor: las piezas son partes de cada muro.

---

## 2026-10-31 (45) · Vistas de dibujo y calcos (E7)

Rama `bloque/e7-vistas-dibujo` (sobre `bloque/e2-galerias`), con pull request. Formato `.archisim` versión 48: el proyecto guarda las vistas de dibujo y los calcos. Los ficheros anteriores se abren igual.

**Hecho**
- **Vistas de dibujo**: espacios solo de dibujo, como el espacio modelo de AutoCAD, para detalles constructivos y esquemas. Están en el Navegador → Proyecto → Planos → **Vistas de dibujo** → «+ Nueva vista de dibujo» (se crea a 1:20).
  - Cada una se abre en su pestaña, un lienzo blanco donde se dibuja en metros reales con las órdenes de AutoCAD (LINEA, CIRCULO, EQ, RR…), con sus forzados y el orto (F8).
  - Archivo → **Importar DXF en la vista** mete un DXF en ella.
  - En Propiedades (sin nada seleccionado) se cambian su nombre y su escala, o se borra.
  - **Se colocan en los planos**: arrastra la vista de dibujo del navegador al plano y queda a su escala, con su nombre como título.
- **Calcos**: debajo de una vista de dibujo o de una planta se puede poner cualquier planta, sección o vista de dibujo, como referencia para dibujar encima. Cada calco tiene:
  - **color** (todo de un color, o sus colores);
  - **opacidad**;
  - **mover X/Y**;
  - **girar**.

  En una vista de dibujo, en sus Propiedades → Calcos; en una planta, en las Propiedades de la vista → Calcos → «Añadir calco…».
- Si se borra lo que muestra un calco, el calco desaparece; una vista no puede ser su propio calco.

**Pendiente**
- Mover y girar el calco con el ratón (hoy con números en Propiedades).
- Las vistas de dibujo no van al IFC (son dibujo 2D, como en Revit).

**Qué debe probar Antonio**
1. Navegador → Proyecto → Planos → Vistas de dibujo → **+ Nueva vista de dibujo**. Se abre «Detalle 1»: dibuja con L, C, EQ… y con Archivo → Importar DXF en la vista mete un detalle del estudio.
2. Propiedades de la vista → Calcos → Añadir calco… → Planta · Planta baja. Ponle color, 30 % de opacidad, muévelo y gíralo.
3. Crea un plano y arrastra la vista de dibujo desde el navegador hasta él: debe quedar a 1:20, titulada con su nombre. Exporta el PDF de los planos.
4. En una planta → Propiedades de la vista → Calcos → Añadir calco… → tu vista de dibujo: el detalle se ve debajo de la planta.

---

## 2026-10-30 (44) · Galerías visuales en todo (E2)

Rama `bloque/e2-galerias` (sobre `main`, v0.76), con pull request. Sin cambios de formato ni de IFC.

**Hecho**
- Las opciones de **cómo se dibuja en planta** se eligen en tarjetas **dibujadas en planta**, cada una como quedaría: dibujo en planta (presentación, según el estilo, técnico), copa en presentación, estilo técnico y de presentación, color o relleno de la copa. Antes se dibujaban en 3D, donde todas se veían iguales.
- Las opciones **de cada ejemplar** (un árbol con su propio dibujo, su copa, su color) también van en tarjetas dibujadas en planta, en Propiedades → «Este objeto». Antes eran listas de texto.
- En Tipos y materiales, la **trama de corte** de un material se elige en tarjetas que la dibujan sobre su color de corte, y la **textura en 3D** en tarjetas con la muestra del material con cada textura.
- Todo lo demás ya iba en tarjetas desde las versiones 0.73 y 0.74: el modelo 3D y las opciones de los objetos, las de puertas y ventanas (en alzado o en 3D) y los materiales de cada pieza. Así que ya no quedan listas de texto en las opciones de objetos, árboles, puertas, ventanas ni materiales.

**Pendiente**
- Las listas de configuración (calidad del 3D, plantillas de vista, órdenes) siguen como desplegables: no son opciones de diseño.

**Qué debe probar Antonio**
1. Abre la demo, selecciona el olivo → Propiedades → «Este objeto» → **Copa**: cada tarjeta es la copa dibujada en planta. Elige otra y mira la planta.
2. En el tipo del árbol, **Dibujo en planta** y **Estilo**: también en planta.
3. Tipos y materiales → un material → **Trama de corte** y **Textura en 3D**: tarjetas con su dibujo.

---

## 2026-10-30 (43) · Versión 0.76

Las PR #113–#116 unidas a `main`: terreno con contorno a mano, cotas y flechas de pendiente (E5), explanaciones bajo el edificio (E8), perfil de alzado de los muros (E3) y armarios empotrados (E10). Formato `.archisim` versión 47. Qué probar: entradas 39 a 42.

---

## 2026-10-30 (42) · Armarios empotrados (E10)

Rama `bloque/e10-armarios` (sobre `bloque/e3-perfil`), con pull request. Sin cambios de formato ni de IFC.

**Hecho**
- Familia **Armario empotrado** (biblioteca → Armarios). Va de suelo a techo, con:
  - casco y zócalo;
  - puertas **batientes** (las parejas abren desde el centro) o **correderas** (en dos guías, alternas);
  - tapajuntas;
  - interior a elegir: **Baldas**, **Barra y balda** o **Cajones y barra**.
- Con «Abierto en 3D» se ve por dentro: las batientes giran y las correderas se deslizan.
- Dos tipos en el catálogo: «Armario empotrado 180» (batiente) y «Armario empotrado corredero 240».
- Las dos maneras de D25, con la misma familia, desde Propiedades → **Empotrado**:
  - **Encajar entre tabiques**: el armario toma el ancho de cara a cara de los muros que tiene a los lados y se centra entre ellos (el número de puertas sigue al ancho).
  - **Abrir hueco en el muro**: colocado dentro de un muro, abre en él un hueco sin carpintería de su ancho y su alto (un tipo «Hueco de armario 180×250» de la familia Hueco).
- En planta: contorno, barra en discontinua, hojas batientes o las dos guías de las correderas, y tapajuntas.
- IFC: `IfcFurniture` con su modelo 3D.

**Pendiente**
- El hueco del muro y el armario no quedan atados: si mueves uno, el otro se queda. D25 pedía una sola familia; aquí es la misma familia con dos acciones.
- Que la altura se ajuste sola al techo.

**Qué debe probar Antonio**
1. Dibuja dos tabiques paralelos a 2,10 m y un muro de fondo. Desde la biblioteca, coloca «Armario empotrado 180» contra el fondo, entre los tabiques → Propiedades → Empotrado → **Encajar entre tabiques**. Comprueba que llena el hueco y que pasa a tres puertas si lo ensanchas.
2. Cambia en su tipo «Puertas correderas», el interior («Cajones y barra») y «Abierto en 3D» a 70. Míralo en 3D.
3. Coloca otro dentro de un muro grueso → **Abrir hueco en el muro**. Mira el hueco en planta y en 3D.

---

## 2026-10-30 (41) · Muros, segunda vuelta: el perfil de alzado (E3)

Rama `bloque/e3-perfil` (sobre `bloque/e8-explanaciones`), con pull request. Formato `.archisim` versión 47 (los muros pueden guardar su perfil; los ficheros anteriores se abren igual).

**Hecho**
- **Perfil en alzado** de un muro recto, como «Editar perfil» de Revit: el contorno del muro visto desde su cara exterior y huecos de cualquier forma. Lo que queda fuera del contorno o dentro de un hueco desaparece del muro: en 3D, en la planta (donde el plano de corte no corta el muro, el muro se abre como sobre una puerta) y en el IFC.
- En Propiedades del muro, grupo **Perfil en alzado**:
  - **Hastial**: sube 1,50 m en el centro.
  - **Inclinado**: el remate baja 1,00 m hacia el final.
  - **Quitar perfil**.
  - **Editar perfil…**: abre una ventana con el muro en alzado, sus puertas y ventanas como referencia y el rectángulo original en discontinua. Ahí:
    - se arrastran los vértices (se ajustan al centímetro y a los bordes del muro);
    - un doble clic en un lado añade un vértice y un clic derecho lo quita;
    - el vértice elegido se puede escribir a lo largo y en altura;
    - se añaden huecos cuadrados o redondos, que se editan igual;
    - **Aplicar** lo deja en el muro (un paso de Deshacer).
- Un perfil más alto que el muro lo sube, si su altura es libre: un hastial sobre un muro de 3 m deja el muro a la altura de la cumbrera.
- El perfil sigue al muro:
  - si el muro se alarga o acorta, se estira con él;
  - al **Dividir en dos**, cada parte se lleva su trozo de perfil y de huecos.
- **IFC**: cada recorte es un `IfcOpeningElement` con su forma que vacía el muro, así que cualquier programa ve el muro recortado. ArchiSim lo lee de vuelta como perfil, no como huecos.

**Pendiente**
- Editar el perfil sobre la vista de alzado o de sección (hoy es una ventana propia).
- Perfil en muros curvos.
- Que las mediciones descuenten lo recortado: hoy el muro se mide por su rectángulo.

**Qué debe probar Antonio**
1. Abre `out/demo.archisim`. Selecciona la fachada este → Propiedades → Perfil en alzado → **Hastial**. Mira la Vista 3D. Como su altura va atada a la planta de cubierta, se recorta en lo alto; prueba también con un muro de altura libre.
2. **Editar perfil…** en un muro de altura libre: baja un vértice de arriba, añade un **Hueco redondo**, muévelo y **Aplicar**. Mira la planta (si el plano de corte pasa por el hueco, el muro se abre) y el 3D.
3. **Dividir en dos** ese muro: cada parte conserva su trozo.
4. Exporta el IFC y ábrelo en otro programa: el muro recortado con la forma del perfil.

---

## 2026-10-30 (40) · Terreno que se adapta al edificio (E8)

Rama `bloque/e8-explanaciones` (sobre `bloque/e5-terreno`), con pull request. Formato `.archisim` versión 46 (el terreno guarda sus explanaciones y el levantamiento original; los ficheros anteriores se abren igual, sin explanaciones).

**Hecho**
- **Explanaciones**: plataformas horizontales dentro del terreno, cada una con su contorno, su cota y su talud (por defecto 1:1). Dentro, el terreno queda a la cota de la plataforma; alrededor, el talud baja o sube hasta encontrar el terreno. Donde dos se tocan, gana la más baja (el vaciado del sótano manda sobre la plataforma de la planta baja).
- **Explanar bajo el edificio** (Propiedades del terreno → Explanaciones): hace una plataforma bajo cada forjado de la planta del terreno y de las de debajo (sótanos), a la cara inferior del forjado y con 50 cm de margen. Las cubiertas planas no cuentan.
- En Propiedades se cambian la cota y el talud de cada plataforma (talud en H:V: 1 = 45°, 2 = 1:2), se ven el **desmonte** y el **terraplén** en m³, y «Quitar explanaciones» devuelve el terreno del levantamiento.
- El levantamiento original se guarda aparte: al cambiar la cota de un punto, moverlo o quitarlo, el terreno se vuelve a explanar. Los puntos que añaden las plataformas no se ven como puntos del levantamiento ni se pueden arrastrar.
- **Planta**: el borde de cada plataforma con línea de trazo y punto, y «Explanación −0,35» por fuera de su lado más largo; las curvas de nivel ya siguen el terreno explanado.
- **Mediciones**: capítulo 12 «Movimiento de tierras» con desmonte (T01) y terraplén (T02) en m³, con clave para asignarles precio del banco.
- **IFC**: la superficie del terreno sale ya explanada; `ArchiSim_Terrain` lleva las plataformas, el levantamiento y los volúmenes (`CutVolume`, `FillVolume`), y la importación lo recupera todo.

**Pendiente**
- Dibujar una plataforma a mano (hoy salen de los forjados) y escribir `IfcEarthworksCut`/`IfcEarthworksFill` como elementos con su sólido.
- Las pasadas del pincel de terreno sobre un terreno explanado se pierden si después se vuelve a explanar.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-app -- out/demo.archisim` → selecciona el terreno (clic en el jardín, lejos de la casa) → Propiedades → Explanaciones → **Explanar bajo el edificio**. Mira el desmonte y el terraplén, y en planta las curvas de nivel rodeando la casa y la línea «Explanación».
2. Cambia el talud de la plataforma 1 a 2,00 y su cota a −1,00: el desmonte crece y las curvas se separan.
3. Vista 3D: el terreno a la cota de la plataforma bajo la casa, con su talud.
4. Mediciones: capítulo «12 Movimiento de tierras».
5. Exporta el IFC y ábrelo en otro programa: el terreno ya explanado y, en sus propiedades `ArchiSim_Terrain`, `CutVolume` y `FillVolume`.

---

## 2026-10-30 (39) · Terreno, segunda vuelta (E5)

Rama `bloque/e5-terreno` (sobre `main`, v0.75), con pull request. Formato `.archisim` versión 45 (el terreno guarda su borde dibujado y lo que enseña en planta; los ficheros anteriores se abren igual, con las cotas de los puntos ocultas).

**Hecho**
- Herramienta **Terreno**, con cuatro modos:
  - **Puntos** y **Línea de nivel**, como antes.
  - **Contorno**: el borde del terreno como un polígono. Al cerrarlo (clic en el primer vértice, Intro o doble clic) se crea el terreno con ese borde, aunque tenga entrantes (ya no es la envolvente). Si la planta ya tiene terreno, este toma el borde y se le añaden puntos por el borde a la cota del suelo.
  - **Varita**: un clic en una curva de nivel de un DXF dibujado en la planta la pasa al terreno, a la cota del texto-número más cercano (o a la escrita en la herramienta si no hay ninguno).
- **Cotas de los puntos ocultas por defecto.** Se encienden en Propiedades → En planta. Con un clic en el número se escribe la cota ahí mismo; Intro la aplica y Esc deja el punto como estaba.
- **Flechas de pendiente** en planta, hacia donde baja el terreno y con su pendiente en %, también desde Propiedades.
- **Editar contorno** (E4) edita el borde dibujado; «Quitar el contorno dibujado» vuelve al de siempre.
- En el IFC, el borde y las dos opciones van en `ArchiSim_Terrain`, y la importación los recupera.

**Pendiente**
- Cotas de nivel colocadas a mano sobre el terreno: hoy se ve la cota de los puntos y la de las curvas.

**Qué debe probar Antonio**
1. Proyecto nuevo → Terreno → modo **Contorno**: dibuja una L y ciérrala. Luego, en modo Puntos, da cotas dentro. El terreno no ocupa la esquina que falta (planta y 3D).
2. Selecciona el terreno de la demo → En planta → enciende **Cotas de los puntos** y **Flechas de pendiente**. Haz clic en un número, escribe 0,50 e Intro.
3. Importa un DXF topográfico como dibujo en la planta (Archivo → Importar DXF) → Terreno → **Varita** → clic en una curva con su cota escrita al lado.

## 2026-10-30 (38) · Versión 0.75: carpintería completa, barandillas, E6, E4 y E3 en `main`

Unidos a `main` los PR #103 a #112:
- Paquete 9 de carpintería:
  - ventanas con detalle, persiana, contraventanas y rejas;
  - puertas especiales y correderas;
  - guillotina, pivotante, plegable y garaje;
  - arcos y huecos de forma.
- Barandillas con tipo, también en las escaleras.
- Diálogo de puertas y ventanas al estilo ArchiCAD (E6, sus tres entregas).
- Editar contorno en modo boceto (E4).
- Dividir muros y franjas pintadas (E3, primera parte).

Formato `.archisim` versión 44. Qué probar de cada parte: las entradas 27 a 37.

## 2026-10-30 (37) · Muros, segunda vuelta, primera parte (E3)

Rama `bloque/e3-muros` (sobre `bloque/e4-contorno`), con pull request. Sin cambios de formato ni de IFC.

**Hecho**
- En Propiedades del muro, un grupo nuevo, **Caras y división**:
  - **Pintar franja**: una cara (interior o exterior), desde y hasta qué altura sobre la base, y el material. Sirve para zócalos, arrimaderos o bandas de fachada, a todo lo largo del muro y sin dibujar aristas en Concepto. Es una zona pintada de las de siempre, así que se ve en 3D, render y alzados.
  - **Dividir en dos** a la distancia que se diga desde su inicio (como «Dividir elemento» de Revit). La segunda parte lleva el tipo, la altura, las restricciones y la fase; las puertas y ventanas que quedan más allá del corte; la pintura de sus caras; y la parte de las zonas pintadas que le toca. Las dos partes se unen en el corte. No corta por una puerta o ventana, ni deja trozos de menos de 5 cm, ni corta muros curvos.
  - **Pintar a mano…** abre la herramienta Pintar sin pasar por Concepto. La ficha de Arquitectura ya tiene sus doce herramientas.
- La envoltura de la capa de acabado en los extremos ya estaba (v0.73).

**Pendiente** (la segunda parte de E3)
- Perfil de alzado editable del muro, con huecos de forma libre: cambia el formato `.archisim`, el 3D, la planta y el IFC, así que va en un bloque propio.
- Dividir muros curvos; dividir con un clic en planta (hoy se escribe la distancia).

**Qué debe probar Antonio**
1. Selecciona un tabique → Caras y división → franja interior de 0,00 a 1,00 con un azulejo → **Pintar franja**. Míralo en 3D.
2. En el mismo muro, **Dividir en dos** a 2,00 m. Cambia el tipo de una de las partes y comprueba que la otra sigue igual y que las ventanas siguen en su sitio.
3. Ctrl+Z deshace la división de una vez.

## 2026-10-30 (36) · Editar contorno en modo boceto (E4)

Rama `bloque/e4-contorno` (sobre `bloque/e6-clasificacion`), con pull request. Sin cambios de formato ni de IFC.

**Hecho**
- Botón **Editar contorno** en Propiedades de un forjado, una cubierta o un terreno. Entra en modo boceto como el de Revit: el contorno (y los huecos del forjado) en magenta sobre la planta, y el modelo no cambia hasta Finalizar.
- Con el ratón:
  - se arrastran vértices y aristas (la arista se mueve paralela);
  - doble clic añade un vértice;
  - Supr quita el vértice elegido.
- En la barra de arriba:
  - **Desfase** de todo el contorno, hacia fuera o hacia dentro;
  - **Arista en arco**, con la flecha en tanto por ciento de la arista;
  - **Recortar/alargar a esquina**: se elige una arista y luego la otra;
  - **Añadir hueco** y **Quitar hueco**, solo en forjados;
  - **Finalizar** (Intro) y **Cancelar** (Esc).
- Finalizar es un solo paso de Deshacer. Si el contorno no vale (se cruza o un hueco queda fuera), se queda en el boceto y explica por qué.
- Cubiertas: cada arista conserva su pendiente; un vértice nuevo parte la arista y las dos mitades toman la misma pendiente.
- Terreno: los puntos del borde se mueven, se añaden (con la cota interpolada) o se quitan.

**Pendiente**
- Los arcos se guardan como 16 tramos rectos: el forjado no tiene aún aristas curvas de verdad (haría falta cambiar el formato).
- Dibujar huecos de forma libre dentro del boceto (hoy se añade un cuadrado de 1 × 1 y se edita).

**Qué debe probar Antonio**
1. Selecciona el forjado de la cubierta de la demo → **Editar contorno**. Arrastra la arista sur 1 m hacia fuera y pulsa Intro. Luego deshaz con Ctrl+Z.
2. Otra vez: elige una arista y pulsa **Arista en arco** con 25 %; **Desfase** de 0,20 y Desfasar. Pulsa Esc: no debe cambiar nada.
3. Recorta una esquina: añade dos vértices con doble clic cerca de una esquina, quita la esquina con Supr, y luego **Recortar/alargar a esquina** sobre las dos aristas.
4. Haz lo mismo con una cubierta y comprueba en 3D que los faldones siguen bien.

## 2026-10-30 (35) · Representación, clasificación y propiedades (E6, tercera entrega)

Rama `bloque/e6-clasificacion` (sobre `bloque/e6-hueco`), con pull request. Sin cambios de formato; suben de versión todas las familias de puertas y ventanas. Con esta, E6 queda completo. Detalle en `docs/CARPINTERIA.md` §11.

**Hecho**
- **Representación**: «Marco cortado en planta» blanco o del color de corte del material.
- **Clasificación**: identidad, clase IFC y código de clasificación (GuBIMClass, Uniclass 2015, OmniClass), exportado como `IfcClassificationReference` del tipo.
- **Propiedades**: mediciones del tipo (hueco de obra, superficie del hueco, paso libre).
- La demo clasifica la puerta de entrada.

**Pendiente**
- Hueco de muro y accesorios en guillotina, correderas de puerta y pivotante; tramas propias del marco en sección.

**Qué debe probar Antonio**
1. P-01 → Representación → «Marco cortado en planta: Color del material»; mira la planta a 1:50.
2. P-01 → Clasificación: escribe un código de GuBIMClass, exporta el IFC y míralo en tu visor (clasificación del tipo de puerta).
3. P-01 → Propiedades: hueco, superficie y paso libre (0,86 m en la demo).

## 2026-10-30 (34) · Hueco de muro y accesorios (E6, segunda entrega)

Rama `bloque/e6-hueco` (sobre `bloque/e6-hojas`), con pull request. Sin cambios de formato; suben de versión la ventana practicable, la corredera y la puerta abatible. Detalle en `docs/CARPINTERIA.md` §10.

**Hecho**
- Pestaña **Hueco de muro**: posición del marco (centrado, por fuera o por dentro, con retranqueo), mocheta, dintel visto (hormigón, piedra, madera, acero, sardinel de ladrillo, arco rebajado de ladrillo) y recercado (plano, moldurado, con clave).
- Pestaña **Accesorios**: alféizar por dentro, albardilla sobre el hueco y parasol de lamas horizontales o verticales.
- 3D, alzados, planta (mocheta cortada, recercado y parasol), render e IFC.
- Instrucción `prism` en las recetas (contornos convexos en alzado).
- Taller: fila de siete huecos con todo esto.

**Pendiente**
- Las mismas piezas en guillotina, correderas de puerta y pivotante.
- Tercera entrega: tramas y líneas en planta y sección, clasificación y propiedades.

**Qué debe probar Antonio**
1. Tipos y materiales → Ventanas → una practicable → **Hueco de muro**: pon «Posición del marco: Por fuera» con 2 cm; en planta el marco se va a la fachada en todas las de ese tipo.
2. Mocheta de 5 cm y «Dintel visto: Arco rebajado de ladrillo»: míralo en 3D y en un alzado.
3. **Accesorios**: parasol de lamas verticales, cambia el ancho y la separación; alféizar y albardilla.
4. Puertas → P-01: recercado con clave y dintel de piedra.
5. Exporta el IFC: sigue validando y el hueco lleva sus piezas.

## 2026-10-30 (33) · Puertas y ventanas al estilo ArchiCAD, primera entrega (E6)

Rama `bloque/e6-hojas` (sobre `bloque/barandillas-escalera`), con pull request. Sin cambios de formato; suben de versión las familias de puertas y ventanas. Detalle en `docs/CARPINTERIA.md` §9.

**Hecho**
- En «Tipos y materiales», puertas y ventanas con **seis pestañas** bajo la vista previa: Hojas, Hueco de muro, Accesorios, Representación, Clasificación y Propiedades. La vista (alzado, planta, sección, 3D) queda siempre arriba y cambia con cada valor.
- Pestaña **Hojas**: dimensiones, hojas y su funcionamiento, y los grupos «Estilo de hoja» y «Manilla y herrajes».
- **Retícula a medida** en las hojas de puerta y **cuarterones a medida** en las ventanas (vidrios a lo ancho y a lo alto).
- **Altura de la manilla** en puertas y ventanas y **caras de la manilla** en puertas.

**Pendiente**
- Segunda entrega: telar, mocheta, dintel y arcos de albañilería; persiana con cajón, albardilla y parasol de lamas.
- Tercera: tramas y líneas en planta y sección, clasificación y propiedades completas.

**Qué debe probar Antonio**
1. Vista → Tipos y materiales → Puertas: elige P-01 y recorre las seis pestañas; la vista de arriba no se mueve.
2. En Hojas → Estilo de hoja, elige «Retícula a medida» y cambia los vidrios a lo ancho y a lo alto; míralo en 3D.
3. En Manilla y herrajes, sube la altura a 1,40 y pon «Solo del lado al que abre»: en 3D la manilla sube y sale en una sola cara.
4. Ventanas: «Cuarterones» → «A medida» con 3 × 4, y «Manilla: altura» a 0,40.
5. Abre un proyecto anterior: sus tipos siguen igual (la manilla a 1,02, centrada en las ventanas).

## 2026-10-30 (32) · Barandillas de escalera con tipo (E12, remate)

Rama `bloque/barandillas-escalera` (sobre `bloque/barandillas`), con pull request. Formato `.archisim` versión 44 (las escaleras guardan el tipo de sus barandillas; las de antes se abren igual). Detalle en `docs/BARANDILLAS.md`, «Barandillas de escalera».

**Hecho**
- La escalera tiene «Tipo de barandilla» en Propiedades, con los mismos tipos que las barandillas sueltas; «Sencilla» es la de siempre.
- Con tipo, la barandilla va por fuera de la escalera (7 cm), fijada al canto, inclinada con cada tramo y con el pasamanos a 90 cm sobre los bordes de las huellas; en las compensadas y de caracol se juntan los trozos cortos del lado de dentro.
- 3D, alzados, render, glTF e IFC (un cuerpo por material, y el nombre del tipo en `ArchiSim_Stair`, que la importación recupera).
- Al copiar un módulo, las barandillas (sueltas y de escalera) llevan el tipo de la copia.
- `archisim escaleras` pone tipo a cinco escaleras.

**Pendiente**
- Pasamanos curvo continuo en el lado de dentro de las compensadas y de caracol (hoy va por trozos rectos).

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-cli -- escaleras out/` y abre `out/escaleras.archisim`: mira en 3D la recta (vidrio con pinzas), la L compensada (contemporánea), la U (clásica, con balaustres torneados), la de caracol (industrial) y la recta de zanca central (nórdica).
2. Selecciona una escalera, cambia «Tipo de barandilla» y luego «Barandillas» a un solo lado: debe seguir el tipo.
3. Comprueba en un alzado que el pasamanos va paralelo a la línea de los peldaños.
4. Exporta el IFC y míralo en otro visor: la barandilla de la escalera sale con sus materiales.

## 2026-10-30 (31) · Barandillas con tipo (E12)

Rama `bloque/barandillas` (sobre `bloque/arcos`), con pull request. Formato `.archisim` versión 43 (las barandillas guardan su tipo; las de antes se abren igual). Detalle en `docs/BARANDILLAS.md`.

**Hecho**
- **Tipos de barandilla**: familia de sistema «Barandilla» con las ocho colecciones y «A tu gusto»: montantes (cuadrado, redondo, pletina, sin postes), pasamanos (rectangular, redondo, pletina, madera moldurado, sin pasamanos), relleno (barrotes, barrotes y pletina, balaustres torneados, vidrio, vidrio con pinzas, chapa perforada o lisa, lamas, cables, malla, forja con volutas), montante de arranque (más grueso, con bola, pilastra), anclaje (placa, al canto, empotrado) y hueco mayor entre barrotes (10 cm, CTE).
- La plantilla trae once tipos; las barandillas nuevas toman «Barandilla · Contemporáneo» (o «Sencilla», la de siempre).
- En Propiedades de la barandilla y de la herramienta: «Tipo de barandilla» y, debajo, los valores del tipo en tarjetas con su 3D.
- 3D, alzados, render, glTF e IFC (un `IfcRailing` con un cuerpo por material); la barandilla de la cubierta de la demo es ahora de vidrio con pinzas.
- Taller: fila de las once barandillas en L.

**Pendiente**
- Barandillas de escalera con tipo; balaustres y barrotes a medida; planta de las barandillas sin postes.

**Qué debe probar Antonio**
1. Herramienta **Barandilla**: dibuja una en L. En Propiedades, cambia el **Tipo de barandilla** entre las once y míralas en 3D.
2. En el tipo, cambia **Relleno** (prueba balaustres, forja con volutas, cables y malla), **Pasamanos**, **Montante de arranque** y **Anclaje** «Al canto».
3. Elige «Sencilla» y comprueba que vuelve la barandilla de antes.
4. Exporta el IFC de la demo: la barandilla de la cubierta sale como `IfcRailing` de vidrio con pinzas.
5. Abre un proyecto antiguo con barandillas: deben verse igual que antes.

## 2026-10-30 (30) · Pivotante, plegable, garaje, y arcos y huecos de forma (paquete 9, cuarta parte)

Rama `bloque/arcos` (sobre `bloque/correderas`), con pull request. Sin cambios de formato. Nuevas familias de sistema «Puerta en arco» y «Ventana en arco»; suben de versión «Puerta abatible», «Puerta pivotante», «Puerta plegable», «Puerta de garaje» y «Hueco». Detalle en `docs/CARPINTERIA.md` §7 y §8; el lenguaje de recetas, en `docs/FAMILIAS.md` (`outline`, `opening_shape`).

**Hecho**
- **Pivotante** con pivotes y tirador largo; **plegable** con guía, bisagras y «Abierta en 3D (%)»; **garaje** seccional (guías, carriles, muelles), basculante (contrapesos) y enrollable (lamas, tambor), con seis acabados, puerta peatonal y «Abierta en 3D (%)».
- **Huecos de forma**: medio punto, rebajado, apuntado, carpanel y óculo. El muro se recorta con la forma en 3D, alzados, secciones, render e IFC, y las mediciones descuentan la forma.
- **Puerta en arco**: hojas que siguen el arco, o puerta rectangular con fijos laterales y fijo en arco (la de la foto de Antonio), cuarterones y vidrieras.
- **Ventana en arco** (con fijo en arco, hojas en arco o fija; óculo) y **recercados** de piedra: plano, moldurado y con clave e impostas.
- **Hueco** sin carpintería con las mismas formas.
- Taller: fila de pivotantes, plegables y de garaje, y fila de arcos (con la puerta de la foto).

**Pendiente**
- Recercado en los huecos sin carpintería; herradura y lobulado; arcos en muros curvos.
- Las puertas y ventanas en arco no tienen aún modelo de colección (su 3D es el de la receta, sin herrajes finos).

**Qué debe probar Antonio**
1. Pon una **Puerta en arco** y elige «Puerta con fijos en arco», hoja «Vidriera» y «Cuarterones de los vidrios»: debe parecerse a la de tu foto. Míralo en 3D, en un alzado y en el IFC (el hueco del muro sale en arco).
2. Cambia el **Arco** a rebajado, apuntado y carpanel, y la **flecha**.
3. Una **Ventana en arco** con recercado «Con clave e impostas», y un **óculo**.
4. Un **Hueco** de medio punto entre dos estancias.
5. Una **puerta de garaje** seccional con ventanas y puerta peatonal, abierta al 50 %; una **plegable** abierta al 40 %; una **pivotante**.

## 2026-10-29 (29) · Correderas, guillotina, y la reja en el IFC (paquete 9, tercera parte)

Rama `bloque/correderas` (sobre `bloque/ventanas`), con pull request. Sin cambios de formato; el IFC añade la reja (`IfcRailing`) y las contraventanas (`IfcShadingDevice`). Las familias «Ventana corredera», «Ventana de guillotina», «Puerta corredera» y «Puerta corredera empotrada» suben de versión. Detalle en `docs/CARPINTERIA.md` §6.

**Hecho**
- **Reja y contraventanas como elementos propios**: en el IFC (la reja con su peso estimado), en los alzados, secciones y render, y en las mediciones (capítulo 11, «Cerrajería y protecciones»: reja en kg, contraventanas en m²). Al importar el IFC no se duplican.
- **Ventana y balconera corredera** con detalle en 3D: carriles, hojas solapadas, perfil (estándar, rotura de puente térmico, PVC, madera, acero fino, minimalista), cierre (uñero, manilla embutida, maneta, tirador de barra), cuarterones, mosquitera, persiana con capialzado, contraventanas y reja.
- **Puerta corredera sobre el muro** colgada de carril visto de granero, carril con galería o carril oculto, con las hojas y tapajuntas de la abatible y su tirador (uñero, barra, asa de granero); y **empotrada**, que entra en el muro.
- **Ventana de guillotina**: dos hojas en sus planos, cierre de media luna, asas y, en madera, cuernos.
- «Abierta en 3D (%)» en cada corredera y guillotina colocada.
- Taller: ocho correderas, cinco balconeras y variantes (cuatro hojas, minimalista, mosquitera, persiana, reja), tres guillotinas, ocho puertas correderas y dos empotradas.

**Pendiente**
- Pivotante, plegable y de garaje.
- La versión de la reja que va al IFC y a los alzados es la sencilla (sin volutas).
- Los proyectos de antes: Biblioteca → actualizar familias.

**Qué debe probar Antonio**
1. Una **ventana corredera**: Editar tipo → «A tu gusto»: Perfil (prueba el minimalista), Cierre, Cuarterones y Mosquitera; en «Persiana», actívala y bájala al 50 %.
2. Una **puerta corredera**: cambia la Guía entre carril visto, con galería y oculto, y el Tirador. Selecciona la puerta colocada y pon «Abierta en 3D» al 70 %.
3. Una **guillotina** de la colección Clásica, subida al 40 %.
4. Pon una **reja** a una ventana, exporta el IFC y ábrelo en otro programa: debe salir como barandilla (`IfcRailing`) con su peso en `ArchiSim_OpeningPart`. Mira también Mediciones: capítulo 11.
5. Mira un **alzado** de una fachada con rejas y contraventanas.

## 2026-10-29 (28) · Carpintería con detalle: las ventanas y las puertas especiales (paquete 9, segunda parte)

Rama `bloque/ventanas` (sobre `main` v0.74), con pull request. Sin cambios de formato ni de IFC (la familia «Ventana practicable» sube a la versión 9). Diseño en `docs/CARPINTERIA.md` §5.

**Hecho**
- **Ventana practicable con detalle en 3D**: la receta sigue siendo la ventana (planta, alzados, IFC y mediciones). En 3D, ArchiSim hace el marco con su galce, los montantes, las hojas solapando el marco, el vidrio con sus junquillos, los cuarterones, la manilla de cada hoja donde cierra según abra (practicable, oscilobatiente, abatible, proyectante, basculante, pivotante) y las bisagras del lado del giro. Las hojas fijas llevan el vidrio en el marco.
- **Ocho colecciones y «A tu gusto»**: Perfil (aluminio recto o redondeado, PVC, madera, madera con moldura, acero fino), Cuarterones (cuadrícula, cuadrícula fina, horizontales, verticales, solo arriba) y Manilla (moderna, de latón, cremona vista, falleba de forja).
- **Por fuera**: la persiana con sus guías y lamas bajadas al porcentaje que se diga; contraventanas (mallorquinas, de tablas o de cuarterones), abiertas contra la fachada o cerradas; y rejas (barrotes, barrotes y pletinas, cuadrícula, forja con volutas), planas sobre la fachada o de cajón con su vuelo. Las rejas y las contraventanas se dibujan también en planta.
- **Tarjetas en 3D** en Editar tipo: la reja y las contraventanas se ven desde la calle.
- **Puertas especiales** (pedidas por Antonio en esta sesión): hojas de chapa (lisa, embutida, con ventanilla cuadrada, ojo de buey o mirilla alargada), lisas con ventanilla y acorazada; barra antipánico, placa de empuje con tirador y escudo de seguridad; cierrapuertas y zócalo de acero. Taller: cortafuegos EI2 60 con ojo de buey, de hospital con ventanilla, de quirófano de dos hojas, metálica embutida, cortafuegos de dos hojas EI2 90 y acorazada (`docs/CARPINTERIA.md` §4 bis).
- **De lejos**: las puertas y ventanas ya no se simplifican juntando puntos (eso cerraba el vidrio y las ventanas lejanas se veían como bloques oscuros). Ahora su versión de lejos es una ventana sencilla del propio generador: marco, hojas y vidrio, sin herrajes, junquillos ni volutas.
- **Arreglo de las puertas de la v0.74**: en un proyecto nuevo, los herrajes salían con el primer material del proyecto (el mortero), porque «Acero inoxidable» no estaba en la plantilla. Ahora la plantilla añade de la paleta los materiales que piden las familias de puertas y ventanas.
- **Taller**: una fachada con las ocho ventanas y otra con persiana, mallorquinas, contraventanas cerradas y tres rejas. Fotos con `ARCHISIM_VENTANAS_OUT=carpeta cargo test --release -p archisim-app window_pictures`, que guarda también `ventanas.archisim`.

**Pendiente**
- La reja y las contraventanas aún no van en el IFC, en los alzados ni en las mediciones (solo en 3D y en planta).
- Corredera y guillotina, y las demás puertas, con el mismo sistema.
- Los proyectos de antes tienen su copia de la familia: para ver las opciones, Biblioteca → actualizar familias.

**Qué debe probar Antonio**
0. Una puerta abatible → Editar tipo → «A tu gusto»: Hoja «Chapa con ojo de buey», Manilla «Barra antipánico», Cierrapuertas y, en Datos, resistencia EI2 60. Prueba también la de dos hojas (las dos llevan barra) y la Acorazada con Escudo de seguridad y Mirilla.
1. Proyecto nuevo, una fachada con una ventana practicable. Mírala en 3D por dentro y por fuera.
2. Selecciónala → Editar tipo: cambia la colección («Modelo 3D») y, en «A tu gusto», el Perfil, los Cuarterones y la Manilla mirando las tarjetas.
3. En «Por fuera»: pon una Reja de forja con volutas, márcala «de cajón» y cambia el vuelo; luego prueba Mallorquinas abiertas y cerradas. Mira la planta: la reja de cajón sale delante de la fachada.
4. Activa la persiana y pon «Persiana: bajada» al 50 %.
5. Exporta el IFC: la ventana es la de siempre (la reja todavía no va).

## 2026-10-29 (27) · v0.74: la cadena del taller y la carpintería, unida a `main`

Antonio pidió unir a `main` todo lo pendiente (pull requests #76 a #102, encadenados) antes de seguir con las ventanas. `main` había avanzado por su lado con las versiones 0.71 a 0.73 (E1, E14–E16), así que la unión no fue directa:

- **Formato**: la versión 40 de `main` (extremos de muro que no se unen) y la 41 (envolver capas) se quedan como están; las piscinas construidas, que en la rama también eran la 40, pasan a ser la **42**. Un fichero guardado con la rama de piscinas (versión 40) se abre igual y exporta el mismo IFC (comprobado con la demo).
- **Vista 3D**: los cortes macizos de la v0.73 (cada capa en su color de corte, el relleno de los muebles cortados por la caja de sección, el corte en color liso) funcionan sobre la vista 3D de la cadena, que dibuja los muebles y las puertas como un modelo por tipo repetido. El relleno de los muebles cortados recorre también esos modelos.
- **Opciones de los objetos**: siguen en tarjetas (las de la cocina a tu gusto), que sustituyen al selector en imágenes de la v0.73.

Validado: formato, clippy, 847 pruebas, demo, IFC 4.3 e IFC4, sesión y tipos de la app, muestrario, ida y vuelta, DXF y fotos de la demo cortada.

**Qué debe probar Antonio**
1. Abre un proyecto tuyo de antes: debe abrir sin avisos.
2. Vista 3D → caja de sección sobre la cocina o el salón: los muros cortados por capas y los muebles cortados macizos; cambia el corte a Negro o Gris.
3. Maqueta: selecciona un sofá y cambia su modelo en las tarjetas.

## 2026-10-29 (26) · Carpintería con detalle: las puertas (paquete 9)

Rama `bloque/carpinteria` (sobre `bloque/personalizar`), con pull request. Sin cambios de formato ni de IFC (la familia «Puerta abatible» sube de versión). Diseño y taller futuro en `docs/CARPINTERIA.md`.

Antonio preguntó si las puertas y ventanas tenían el mismo mimo que los muebles del taller (tiradores, pomos, molduras) y pidió hacerlo, empezando por las puertas, con detalle según la distancia y dejando planteado un taller para dibujar hojas, molduras y rejas a medida.

**Hecho**
- **Puerta abatible con detalle en 3D**: la receta sigue siendo la puerta (planta, alzados, IFC y mediciones no cambian) y pide a ArchiSim que fabrique en 3D la hoja, el forro, los tapajuntas por las dos caras (en un muro de fachada, solo por dentro), las bisagras, la manilla con su roseta y su bocallave, el tope, el pasador de la hoja pasiva en las de dos hojas y la mirilla.
- **Ocho colecciones** («Modelo 3D»): cada una con su hoja, tapajuntas, manilla y bisagras. Y **A tu gusto**: Hoja (lisa, ranurada, ranurada vertical, dos, cuatro o seis cuarterones, plafonada, moldura aplicada, vidriera, media vidriera, vidriera con junquillos, lamas, tablas con travesaños y cruz por detrás), Tapajuntas (sin, plano, redondeado, moldurado, con junquillo), Manilla (palanca con roseta, con placa o recta, pomo, tirador de barra, sin manilla), Bisagras (ocultas, de pala, de pernio) y Mirilla, además del material de los herrajes.
- **Tarjetas en 3D**: en Editar tipo, las opciones de las puertas que fabrica ArchiSim se ven en tarjetas con la puerta en su muro, casi de frente.
- **Detalle según la distancia**: cada tipo de puerta es un solo modelo colocado en cada puerta, con versiones más sencillas al alejarse (como los muebles); la sombra, siempre con la versión fina.
- **Taller**: una tabiquería con las ocho puertas y una fachada con tres puertas de entrada (clásica de dos hojas y seis cuarterones, mediterránea de media vidriera y rústica de tablas). Fotos con `ARCHISIM_PUERTAS_OUT=carpeta cargo test --release -p archisim-app door_pictures`.
- Las hojas y las molduras son **datos** (`LeafDesign`, `Profile`), no programa: el taller de carpintería futuro podrá guardar hojas, molduras y rejas con nombre que aparezcan como una tarjeta más (`docs/CARPINTERIA.md` §4).

**Pendiente**
- Las demás puertas (corredera, pivotante, plegable, de garaje) siguen con el 3D de su receta; después, las ventanas (hojas, junquillos, cremonas, contraventanas, persianas y rejas).
- Los proyectos de antes tienen su copia de la familia: para ver las opciones, Biblioteca → actualizar familias.
- El taller de carpintería (hojas, molduras y rejas a medida) está planteado, no hecho.

**Qué debe probar Antonio**
1. Proyecto nuevo → Maqueta, dibuja una habitación y pon una puerta abatible. Acércate y aléjate: de cerca se ven la manilla, las bisagras y los tapajuntas.
2. Selecciona la puerta → Editar tipo: en «Modelo 3D» cambia la colección y en «A tu gusto» la Hoja, el Tapajuntas, la Manilla y las Bisagras mirando las tarjetas. Prueba también la Mirilla.
3. En Editar tipo, pon «Nº de hojas» a 2 y el ancho a 1,45 m, y mira la hoja pasiva con su pasador. Prueba también la tarja y un fijo lateral: siguen saliendo de la receta, junto a las hojas nuevas.
4. Exporta el IFC: la puerta es la de siempre (la receta), con sus hojas.
5. Lee `docs/CARPINTERIA.md` §4 (taller de hojas, molduras y rejas) y dime si es el camino.

## 2026-10-29 (25) · Cocina a tu gusto, opciones en tarjetas y Maqueta siempre en color

Rama `bloque/personalizar` (sobre `bloque/biblioteca-taller`), con pull request. Sin cambios de formato ni de IFC (las familias de cocina suben de versión).

Antonio, al ver el PR #100: le gusta que cada pieza tenga un modelo y se elija dentro, pero la cocina tiene que poder personalizarse además de las ocho colecciones; en Propiedades y en Editar tipo, las opciones tienen que verse en tarjetas, sean lo que sean; los materiales, con unos recomendados en tarjetas y los demás después; y la Maqueta, siempre en color como en Arquitectura y con líneas finas.

**Hecho**
- **Cocina a tu gusto**: el módulo bajo, la columna, el mueble alto y la campana tienen una sección «A tu gusto» con **Frente** (liso, de cantos redondeados, con marco de acero, cuarterón, cuarterón con moldura, moldura, tablas, listones, acanalado, rejilla, vidrio, vidrio con junquillos, malla), **Tirador** (sin tirador, gola, pomo de madera o de latón, argolla, concha, barra corta o larga, lengüeta de cuero, uñero), **Apoyo** (zócalo, rodapié con moldura, patas, bastidor de acero), y, según la pieza, **Fregadero**, **Grifo**, **Placa**, **En la pared** (armarios, vitrinas o cuatro clases de baldas) y **Forma de la campana**. Cada una empieza en «Como la colección». Una prueba coloca todas las opciones en las ocho colecciones y comprueba que nada queda en el aire (`a_kitchen_to_ones_taste_stands`); por ella, los tiradores de los cajones con cuarterón van sobre el panel hundido y los de vidrio, rejilla o malla, sobre el marco.
- **Opciones en tarjetas** (`app/cards`): en Propiedades (tipo de un objeto), en Editar tipo (puertas y ventanas) y en la barra de la Maqueta, cada opción de una lista se ve como tarjeta con su miniatura: los objetos en 3D, las puertas y ventanas en alzado (las hojas fija, practicable, oscilobatiente…). Se ve la elegida; un clic abre todas.
- **Materiales en tarjetas**: la muestra con su textura; un clic abre «Recomendados para esta pieza» (lo que le darían las ocho colecciones, los de la paleta de su clase y los del proyecto de su clase) y «Otros materiales del proyecto», con buscador. En Propiedades, Editar tipo, la barra de la Maqueta, las capas y accesorios de los tipos de muro y forjado, los muros cortina y Concepto.
- **Maqueta**: siempre en color (sin atenuación por disciplina) y siempre con líneas finas.
- La barra de la Maqueta ya no se sale de la ventana: los materiales que no caben van a «+N».

**Pendiente**
- Las miniaturas enseñan la pieza entera: en el grifo o la placa, el cambio es pequeño en la tarjeta (se ve bien al elegirlo).
- Las opciones de cada objeto concreto («Este objeto»: el dibujo de un árbol) y la ventana en esquina siguen en lista.
- Los proyectos de antes tienen su copia de las familias de cocina: para ver «A tu gusto», Biblioteca → actualizar familias.

**Qué debe probar Antonio**
1. Proyecto nuevo → Maqueta → Amueblar → Cocina. Coloca un «Bajo cajonera 60», selecciónalo y, en la barra de abajo, cambia **Frente**, **Tirador** y **Apoyo** mirando las tarjetas. Con varios módulos en fila, «a toda la fila» los cambia todos.
2. Cambia «Qué lleva» a Fregadero y mira **Fregadero** y **Grifo** (también en Técnica → Propiedades, al final del tipo).
3. En la barra, pulsa un material (por ejemplo, Frentes): arriba los recomendados con su textura, abajo los demás.
4. Vista → Tipos y materiales → Ventanas: en «Hoja 1», las tarjetas en alzado. En Muros, el material de un rodapié.
5. Con Técnica en la disciplina Estructura o Instalaciones, pasa a la Maqueta: todo debe verse en color, con líneas finas.

---

## 2026-10-29 (24) · El taller pasa a la biblioteca: paquetes 2 a 8

Rama `bloque/biblioteca-taller` (sobre `bloque/remates-modelos`), con pull request. Sin cambios de formato. Cambia el IFC de las piezas que se pasan (lámparas como `IfcLightFixture`, piezas nuevas).

Antonio aprobó los cuatro grupos: cocina y baño (2a, 2b), dormitorio y luces (3, 4), exterior, urbano y estudio (5, 6), y hostelería, oficinas, tiendas, deporte y piscinas (7, 8).

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquetes 2 a 8 en la biblioteca»)
- 32 familias de siempre suben de versión con el modelo de cada una de las ocho colecciones (cocina, baño, dormitorio, estudio, terraza, calle, plantas, decoración y cuadros); 53 familias nuevas (tocador, cuna, litera, lámparas, mesa, silla y sofá de terraza, bares, oficinas, tiendas, piscinas, gimnasio, pistas, gradas, parque infantil…).
- Como en el paquete 1, los modelos antiguos ya no se ofrecen; las piezas de la biblioteca van al más parecido de cada colección y tres cambian de nombre (Kentia, Tetera y cuencos, Cama infantil 90×190). La nueva mesa de jardín se llama **Mesa de terraza** porque «Mesa de exterior» ya era la del bistró.
- Una pieza nueva de la biblioteca por cada familia nueva; en la Maqueta, temas nuevos «Deporte», «Hostelería y comercio» y, en Exterior, «Piscinas y deporte».
- El taller sigue montando todo con las familias de la biblioteca; solo guarda la planta y la farola de la segunda entrega.

**Pendiente**
- La planta y la farola del taller, cuando Antonio las mire.
- Las fotos de todas las colecciones con las familias ya de sistema (las pruebas `*_pictures` siguen funcionando igual).

**Qué debe probar Antonio**
1. Abre un proyecto nuevo con la versión del pull request. En la Maqueta, Amueblar → Cocina, Baño, Dormitorio, Deporte y Hostelería y comercio; Exterior → Jardín y terraza, Mobiliario urbano y Piscinas y deporte. Coloca alguna pieza de cada uno y mírala en 3D.
2. Elige una colección (por ejemplo, Rústica) y coloca un inodoro, una cama y una farola: deben salir los modelos de esa colección.
3. Abre un proyecto tuyo de antes con inodoros o camas: debe abrirse igual. Si en la Biblioteca actualizas sus familias, los modelos antiguos pasan al nórdico cálido y las medidas se conservan.
4. Exporta el IFC y comprueba que las lámparas salen como luminarias (`IfcLightFixture`).

---

## 2026-10-29 (23) · Revisión automática de los modelos: nada en el aire

Misma rama `bloque/remates-modelos` y mismo pull request que la (22). Sin cambios de formato ni de IFC.

Antonio eligió, de las propuestas, la **revisión de modelos**: que una prueba busque lo que flota en todos los modelos, como las trece piezas de su lista, antes de que entren en la biblioteca.

**Hecho**
- `archisim_props::check`: parte cada modelo en sus piezas (triángulos que comparten esquinas), mira cuáles se tocan (a menos de 4 mm, se cruzan o una está dentro de otra) y busca las que ninguna cadena de piezas une al suelo (o a la pared, o al techo, en lo que va colgado: `anchors_for`). De cada pieza suelta da su papel, su caja, a cuánto está y de qué pieza. Las hojas y flores no se miran.
- Dos pruebas que ya no dejan pasar nada suelto: `nothing_floats` (archisim-props: todos los modelos de todas las familias, con sus medidas por defecto) y `nothing_in_the_air` (biblioteca: todas las piezas del taller, con sus medidas del taller). Con `--nocapture` dicen qué está suelto y dónde.
- La primera pasada encontró **164 tipos del taller y 37 modelos de la biblioteca** con algo en el aire. Arreglados todos. Lo más visible: la mesa de ping-pong (el tablero no apoyaba en las patas: 4 cm), los cuatro **tocadores** (el espejo, en la pared), las **cunas** (barandillas a 2 cm de los postes), la **litera clásica**, las **sillas de exterior** nórdica, industrial y años 50 (listones sin travesaños, patas a 5 mm del aro), las **mesas de exterior** industrial y rústica, las **tumbonas** de teca, las **duchas de piscina** (columnas a 3 cm de la bandeja, alcachofa y volante sueltos), la **farola fernandina** y las mediterránea y rústica (farol a 5 cm), la **canasta** y el **tobogán** (escalera hasta la plataforma), la **cinta de correr** (consola), el **columpio** (travesaño), los **bancos** de calle nórdico, años 50 e industrial, la **cesta de la papelera**, las **sillas de bar** japandi, industrial y clásica, la **mesa de bar** clásica, el **banco corrido** industrial (respaldo sin tablero), la **mampara** nórdica, los **puestos de trabajo** contemporáneo (bandeja de cables) y rústico, las **mesas expositoras** (balda y género), los **probadores** (cortina bajo la barra), los **grifos** (manetas), el **frigorífico clásico** y los **altos rústicos** (cornisa y barra de ganchos), los **apliques** años 50 e industrial, las **camas** (almohadas sobre el colchón), el **espejo industrial** del baño, la **lámpara de arco**, las **butacas** redonda y de ratán, las **cortinas** (barra en la pared), el **radiador** (secciones unidas), el **recibidor**, la **hamaca**, las **señales**, las **bicicletas**, la **fuente de fundición**, las **jardineras** y la **costilla de Adán**. Los tiradores con muesca de las cómodas y mesitas estaban 8 mm delante del frente.

**Qué debe probar Antonio**
1. Abre `taller.archisim` en la versión del pull request y mira, en 3D y de cerca, las piezas que te suenen de esta lista.
2. Si algo sigue mal, dime el tipo: esta prueba ve lo que está en el aire, pero no lo que **atraviesa** otra pieza (como la tela del galán), ni lo que sobresale o está a una altura rara; eso sigue necesitando tus ojos.

---

## 2026-10-29 (22) · Remates de modelos: lo que Antonio vio flotando o atravesado

Rama `bloque/remates-modelos` (sobre `bloque/paquete-8d`), con pull request. Sin cambios de formato ni de IFC; solo los modelos 3D del taller.

Antonio, mirando el taller, señaló trece modelos. Arreglados uno a uno y mirados desde cuatro lados y desde abajo:
- **Mesa de ping-pong**: los postes de la red ya no sobresalen; están junto al canto del tablero, sujetos con su pinza (una placa encima y la mordaza debajo).
- **Bicicleta estática**: la barra del manillar llega al manillar y a la consola.
- **Aparcabicis mediterráneo**: las volutas cuelgan de la barra de arriba, sin aire. **Rústico**: un poste en cada extremo del tronco (y más entre medias si es largo).
- **Silla de trabajo contemporánea** (y la de la biblioteca de siempre): el respaldo va delante de la barra; la barra sale de debajo del asiento, sube por detrás y sostiene el respaldo y el reposacabezas, que ya no flota.
- **Tumbona de piscina clásica**: el cuerpo de mimbre sigue bajo el respaldo, así que las patas de atrás apoyan en él. **Industrial y años 50**: las patas de atrás llegan al bastidor del asiento y un tirante sostiene el respaldo.
- **Piscina rústica**: quitadas las cinco piedras de adorno que parecían bultos.
- **Silla de exterior clásica**: bastidor del asiento sobre las patas; el arco del respaldo baja hasta él y las volutas se apoyan en una barra. **Mediterránea**: quitadas las volutas de debajo del asiento; en su lugar, un travesaño bajo entre las patas.
- **Literas** (y todas las camas con barandillas o piecero): el edredón queda dentro de los barrotes, sin pasar al otro lado.
- **Espejo de pie años 50**: las patas delanteras se unen al aro por abajo, no a media altura del espejo.
- **Galanes** contemporáneo, nórdico, años 50 y rústico: el palo llega a la percha, y la barra del pantalón está delante del palo con un brazo corto, así que la tela ya no lo atraviesa (el de nórdico y años 50 pierde la bandeja, que chocaba con el pantalón).

Para mirar cualquier pieza sola desde cinco puntos de vista: `ARCHISIM_PIEZAS_OUT=carpeta ARCHISIM_PIEZAS="Litera|Galán" cargo test --release -p archisim-app -- named_pictures` (sin `ARCHISIM_PIEZAS`, lista los tipos del taller).

**Qué debe probar Antonio**
1. Abre `taller.archisim` en la versión del pull request y mira las trece piezas de tu lista en 3D, girando alrededor.
2. Dime si ves más piezas con partes en el aire o que se atraviesan; con el nombre del tipo (como en tu lista) las encuentro enseguida.

---

## 2026-10-28 (21) · Paquete 8, tanda 8d: la piscina construida, que excava el terreno

Rama `bloque/paquete-8d` (sobre `bloque/paquete-8c`), con pull request. **Formato `.archisim` versión 40** (piscinas, `pools`; los ficheros anteriores se abren sin cambios). Cambia el IFC (piezas nuevas) y la aplicación (herramienta nueva).

Antonio: «con qué seguimos ahora?» → tanda 8d. Decisiones: contorno libre como un forjado; fondo con pendiente; IFC por piezas (solera `IfcSlab`, paredes `IfcWall`, revestimiento `IfcCovering`, coronación `IfcSlab`, en un `IfcElementAssembly` «Piscina», y el agua como `IfcSpace`); las piscinas-objeto de la 8a también recortan el terreno.

**Hecho**
- Elemento nuevo **Piscina** (`archisim-core::model::pools`): el borde del agua, profundidad menor y mayor con la dirección de la pendiente, resguardo, grueso del vaso, ancho de coronación, coronación sobre la planta y cuatro materiales. Superficie, perímetro y volumen de agua.
- 3D (`archisim-geom::pools`): solera y paredes de hormigón, revestimiento interior, coronación y agua; el fondo sigue la pendiente.
- **Recorte del terreno**: el terreno pierde el hueco del vaso (su contorno exterior), con sus paredes y su fondo, y también las áreas pintadas y la hierba que caían dentro. Lo hacen las piscinas construidas y las piscinas-objeto de la biblioteca.
- Planta: coronación, agua en azul claro, vaso oculto en discontinua, flecha de la pendiente con las dos profundidades, nombre y superficie. Categoría de visibilidad nueva «Piscinas».
- IFC: el conjunto «Piscina» con sus piezas (las paredes con `Pset_WallCommon` exteriores y portantes y su superficie lateral; la coronación como `IfcCovering` COPING, como las albardillas, para que cumpla los requisitos IDS de forjados) y `ArchiSim_Pool` para leerla de vuelta; el agua como `IfcSpace` «Lámina de agua» con superficie y volumen. La importación la recupera entera (ida y vuelta sin diferencias). `scripts/validate_ifc.py` comprueba las piscinas (apartado 14).
- Aplicación: herramienta **Piscina** en Urbanismo y en la Maqueta (Exterior → Terreno y caminos), con polígono y rectángulo; la pendiente sale a lo largo del primer lado dibujado. Propiedades con medidas, pendiente (en grados, «Girar 90°» e «Invertir»), materiales y mediciones (lámina de agua, volumen de agua, excavación aproximada). Esquinas arrastrables; mover, copiar, girar y simetría.
- La demo lleva la piscina del jardín como piscina construida (antes era un dibujo 2D).

**Pendiente**
- Escaleras romanas y escalerillas en la piscina construida; formas hechas (riñón, ovalada) como atajos del contorno.
- Las piscinas dentro de módulos no se copian al juntar el modelo (como las plantaciones).

**Qué debe probar Antonio**
1. Abre la demo (`archisim demo out/`) y mira la piscina del jardín en planta y en 3D: el terreno debe tener el hueco.
2. En Urbanismo, herramienta Piscina (teclas P S): dibuja una en forma de L sobre un terreno; cambia las profundidades, gira la pendiente y arrastra una esquina.
3. Exporta a IFC y ábrelo en tu visor: comprueba el conjunto «Piscina», sus piezas y el agua como espacio con su volumen.

---

## 2026-10-28 (20) · Paquete 8, tanda 8c: pistas, gradas, parque infantil y calistenia

Rama `bloque/paquete-8c` (sobre `bloque/paquete-8b`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio: «Con qué seguimos ahora?» → tanda 8c. Después: «No hay de fútbol, no? Tanto sala como hierba» → pista de fútbol sala y campo de fútbol 11 y 7.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 8, tanda 8c»)
- Diez familias nuevas, un modelo de cada una con sus colores por materiales: **pista de pádel** (20 × 10 m, césped, cristal escalonado y malla, red, puertas junto a la red y cuatro focos que se encienden de noche), **pista de tenis** (reglamentaria, con líneas de dobles e individuales, red con su caída al centro y silla de árbitro, sobre su zona de seguridad), **pista de baloncesto** (28 × 15 m con líneas, zonas pintadas, línea de triple y una canasta en cada fondo, con el mástil detrás de la línea de fondo), **pista de fútbol sala** (40 × 20 m, áreas de 6 m, puntos de 6 y 10 m, porterías de 3 × 2 m), **campo de fútbol** de hierba segada a franjas (fútbol 11, 105 × 68 m, o fútbol 7, 65 × 45 m, con áreas, puntos, arcos, banderines de córner y porterías), **grada** (de hormigón, de 1 a 10 filas, un asiento cada medio metro y barandilla), **columpio** (asiento plano y asiento cuna), **tobogán** (torre con escalera, plataforma a 1,2 m y tejadillo), **balancín**, **balancín de muelle**, **estación de calistenia** (dominadas a tres alturas, escalera horizontal, paralelas y barras de flexiones sobre caucho) y **elíptica de parque** (biosaludable).
- En el taller, después del gimnasio: las tres pistas en fila (la grada delante de la de tenis) la pista de fútbol sala y los dos campos de fútbol, y el parque infantil con la calistenia.

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión del pull request y ve a las pistas, después del gimnasio.
2. Comprueba las medidas y líneas de las pistas y los campos en planta y en 3D; cambia el campo de fútbol 11 a 7 y el largo y las filas de la grada. Las medidas de fútbol 7 (área de 13 × 30 m, área pequeña de 5 × 12 m, punto a 9 m, círculo de 6 m, porterías de 6 × 2 m) son las habituales; dime si tu federación usa otras.
3. Mira los focos del pádel de noche, y dime qué falta en el parque (arenero, tirolina, mesa de ajedrez, más aparatos biosaludables…).

---

## 2026-10-28 (19) · Paquete 8, tanda 8b: deporte en casa y gimnasio

Rama `bloque/paquete-8b` (sobre `bloque/paquete-8a`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio: «Sigue con el trabajo» → tanda 8b.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 8, tanda 8b»)
- Diez familias nuevas, un buen modelo de cada una y sus colores por materiales (sin colecciones, como se decidió): **canasta de baloncesto** (portátil, con base lastrada y altura del aro regulable de 2 a 3,05 m), **portería** (de 1,2 m a reglamentaria de 7,32 × 2,44, con red), **mesa de ping-pong** (plegable sobre ruedas, con red y líneas), **cama elástica** (redonda, con muelles, protector y red de seguridad; diámetro regulable), **cinta de correr**, **bicicleta estática**, **banco de pesas** con su soporte y la barra con discos, **mancuernero** de dos baldas, **espaldera** de madera por cuerpos de 90 cm y **esterilla de yoga** con bloques y pesa rusa.
- En el taller, después de las piscinas: el jardín (canasta, portería, dos mesas de ping-pong —azul y verde— y cama elástica) y el gimnasio con su pared.
- Arreglo de la tanda 8a: la forma de la piscina (rectangular, escalera romana, desbordante) no se aplicaba al elegirla; ahora sí. La escalera romana queda bajo el agua y solo se ve en sección.

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión del pull request y ve al deporte, después de las piscinas.
2. Cambia la altura del aro, el tamaño de la portería (prueba 7,32 × 2,44), el diámetro de la cama elástica y los cuerpos de la espaldera.
3. Cambia colores: tablero de la mesa, protector de la cama elástica, tapizado del banco. Dime qué falta en casa o en el gimnasio.

---

## 2026-10-28 (18) · Paquete 8, tanda 8a: piscinas como objeto y duchas de exterior

Rama `bloque/paquete-8a` (sobre `bloque/paquete-7c`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio: «Antes de eso, creo que sería interesante meter equipamiento deportivo y piscinas». Eligió los cuatro grupos de deporte (jardín y casa, gimnasio, pistas y equipamiento público, parque infantil y calistenia), las piscinas de las dos maneras (primero un objeto, después una herramienta constructiva que excava el terreno) y colecciones solo donde tenga sentido. Plan del paquete 8: 8a piscinas como objeto y ducha; 8b deporte en casa y gimnasio; 8c pistas, gradas, parque infantil y calistenia; 8d piscina constructiva.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 8, tanda 8a»)
- Familia **Piscina** con un modelo por colección: vaso de hormigón bajo el suelo revestido de gresite, pizarra o hormigón, lámina de agua, coronación, playa alrededor, escalerilla y focos. Largo, ancho, profundidad del agua y ancho de playa son parámetros; la forma puede ser rectangular, con escalera romana o desbordante (rebosadero con rejilla). La de los años 50 es un riñón y la clásica acaba en medio punto con escalones semicirculares.
- Familia **Ducha de piscina** con un modelo por colección (poste de roble, pilar encalado con azulejo, columna cromada curvada, caño de bambú, columna de acero, poste con regadera, tubo negro con llave de volante, columna de latón acanalada).
- En el taller, una piscina por colección con su ducha y dos tumbonas, después de las tiendas.

**Pendiente**
- La piscina como objeto no excava el terreno: el vaso queda por debajo del suelo y, si hay terreno, lo atraviesa. Eso lo resuelve la tanda 8d (herramienta constructiva).

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión del pull request y ve a las piscinas, después de las tiendas.
2. Cambia largo, ancho, profundidad y ancho de playa; prueba las tres formas en una misma colección.
3. Mira la vista de noche (los focos se encienden) y dime qué cambiarías, colección por colección.

---

## 2026-10-28 (17) · Paquete 7, tanda 7c: las tiendas de las ocho colecciones

Rama `bloque/paquete-7c` (sobre `bloque/paquete-7b`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca. Con esta tanda queda modelado el paquete 7 y, con él, el plan entero de la biblioteca nueva (paquetes 1 a 7), a falta de la revisión de Antonio.

Antonio: «Con qué seguimos ahora?» → tanda 7c, tiendas.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 7, tanda 7c»)
- Cinco familias nuevas, cada una con un modelo por colección: **mostrador de tienda** (con caja registradora y lector de tarjetas hacia el dependiente, baldas por detrás), **estantería de tienda** (de pared, con género: cajas, tarros y ropa doblada), **mesa expositora** (para el centro, con ropa doblada encima y cajas en la balda de abajo), **burro de ropa** (con perchas y prendas) y **probador** (espejo, perchas, taburete y cortina medio corrida).
- En el taller, una tienda por colección a continuación de las oficinas: dos estanterías y el probador en la pared, el mostrador, la mesa expositora y el burro delante.

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión del pull request y ve a las tiendas, después de las oficinas.
2. Alarga una estantería y sube su altura (las baldas se reparten de nuevo); alarga el burro (salen más perchas); ensancha el probador.
3. Dime qué cambiarías, colección por colección.

---

## 2026-10-28 (16) · Paquete 7, tanda 7b: las oficinas de las ocho colecciones

Rama `bloque/paquete-7b` (sobre `bloque/paquete-7a`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio: «qué toca ahora?» → tanda 7b, oficinas.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 7, tanda 7b»)
- Cinco familias nuevas, cada una con un modelo por colección: **puestos de trabajo** (mesa bench de puestos enfrentados con mampara, monitor y teclado en cada puesto), **mesa de reuniones** (con caja de conexiones en el tablero), **mostrador de recepción** (repisa alta para visitantes, un tramo a 80 cm para silla de ruedas y, detrás, la mesa de trabajo a 74 cm con cajonera y monitor), **armario de oficina** (puertas abajo y archivadores en baldas encima) y **mampara acústica** (panel de fieltro, de lamas, de malla o biombo de tres hojas).
- Las sillas son las de cada colección: la silla de trabajo de la tanda 6b en los puestos y la silla de comedor en la mesa de reuniones.
- En el taller, una oficina por colección a continuación de los bares: armario en la pared, recepción con su mampara, puestos para cuatro y mesa de reuniones para seis.

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión del pull request y ve a las oficinas, después de los bares.
2. Cambia el número de puestos por lado y el largo de la mesa bench; alarga el mostrador y comprueba que el tramo bajo queda a 80 cm.
3. Dime qué cambiarías, colección por colección.

---

## 2026-10-28 (15) · Paquete 7, tanda 7a: la hostelería de las ocho colecciones

Rama `bloque/paquete-7a` (sobre `bloque/paquete-6b`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio, con el paquete 6 terminado: «con qué seguimos ahora?» → paquete 7 (comercio, hostelería y oficinas) en tres tandas; empieza por la hostelería (7a).

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 7, tanda 7a»)
- Cinco familias nuevas, cada una con un modelo por colección: **barra de bar** (frente hacia los clientes con encimera volada y reposapiés; detrás, el lado de servicio con armarios, bandeja de acero y balda de vidrio), **contrabarra** (armarios bajos con encimera y, encima, baldas con botellas), **mesa de bar** (baja o alta), **silla de bar** y **banco corrido**.
- Lo que cambia de una colección a otra: el frente de la barra (listones de roble, fábrica encalada con azulejos, laminado de color con bandas cromadas, listones de roble ahumado, microcemento con tira de luz, piedra en hiladas, tablas en bastidor de acero con remaches, cuarterones entre pilastras), la contrabarra (baldas flotantes, hornacinas de medio punto, espejo con postes de latón, cuadrícula de nichos, panel iluminado con baldas de vidrio, baldas gruesas con escuadras de forja y barricas, tubo negro, espejo enmarcado con cornisa), la mesa (tres patas, azulejo con pie de forja, laminado con borde cromado, aspa de roble, disco de acero, barrica, pie de fundición en X, mármol con trípode de volutas), la silla y el banco.
- Las tiras de luz de la barra contemporánea y el panel de su contrabarra se encienden de noche.
- En el taller, un bar por colección a continuación de los estudios: la contrabarra en la pared, la barra delante con tres taburetes de la colección y un banco corrido con dos mesas y sus sillas.

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión del pull request y ve a los bares, después de los estudios.
2. Mira cada barra por los dos lados (el de los clientes y el de servicio), la contrabarra de frente y el rincón del banco; de noche, la luz del bar contemporáneo.
3. Alarga una barra y un banco corrido; sube una mesa a 1,05 m (mesa alta).
4. Dime qué cambiarías, colección por colección.

---

## 2026-10-28 (14) · Paquete 6, tanda 6b: el estudio de las ocho colecciones

Rama `bloque/paquete-6b` (sobre `bloque/paquete-6a`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 6, tanda 6b»)
- **Escritorio** por colección, con o sin cajonera a un lado: de roble sobre patas abiertas con cajón de uñero (nórdico), pintado con patas torneadas y argollas (mediterráneo), de teca sobre patas de compás con cajonera flotante (años 50), tablón sobre costados macizos con balda (japandi), tablero fino sobre patines de acero y cajonera con ruedas (contemporáneo), tablón sobre caballetes (rústica), madera sobre bastidor de acero con cajonera metálica (industrial) y escritorio de tapete de piel con canto de latón, patas acanaladas y cajones moldurados (clásica).
- **Silla de trabajo** por colección: giratoria de carcasa de roble (nórdico), cubeta de color giratoria (años 50), silla de oficina de malla (contemporáneo), taburete-silla de acero con aro reposapiés (industrial); el mediterráneo, japandi, rústico y clásico usan su silla de comedor.
- En el taller, un estudio por colección a continuación de los rincones de salón: el escritorio con su lámpara de mesa, la silla delante y la estantería de la colección.

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión del pull request y ve a los estudios, después de los rincones de salón.
2. Cambia la cajonera de un escritorio (a la izquierda, a la derecha, sin ella) y su ancho.
3. Dime qué cambiarías, colección por colección.

---

## 2026-10-28 (13) · Paquete 6, tanda 6a: plantas de interior, decoración y cuadros de las ocho colecciones

Rama `bloque/paquete-6a` (sobre `bloque/paquete-5b`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio, con el paquete 5 terminado: «qué toca ahora?» → paquete 6 (plantas, decoración y estudio); esta tanda son las plantas y la decoración, el estudio va en la 6b.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 6, tanda 6a»)
- **Planta de interior** por colección, cada una en su maceta y hoja a hoja: ficus lira (nórdico, maceta blanca sobre trípode de roble), limonero con sus limones (mediterráneo, terracota), costilla de Adán con las hojas partidas (años 50, maceta cónica sobre patas), sansevieria (japandi, piedra), ave del paraíso (contemporáneo, hormigón), helecho (rústica, cesta de mimbre), potos colgante (industrial, cubo de acero sobre soporte) y kentia (clásica, jardinera esmaltada).
- **Decoración** de mesa y balda por colección (libros, jarrones, fruteros, tetera, farol, velas encendidas de noche, candelabros) y **cuadro o espejo** de pared (lámina, platos, espejo sol, kakemono, lienzo, espejo de madera, reloj de estación, óleo con marco dorado).
- En el taller, un rincón de salón por colección a continuación de las calles: su aparador con la decoración encima, el cuadro sobre él y la planta al lado.
- El limonero comparte el árbol del olivo (`small_tree`), con hojas anchas y fruta.

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión del pull request y ve a los rincones de salón, después de las calles.
2. Mira las plantas de cerca, la decoración sobre cada aparador y el cuadro; de noche, las velas encendidas.
3. Cambia la altura de una planta y el ancho de un cuadro.
4. Dime qué cambiarías, colección por colección.

---

## 2026-10-28 (12) · Paquete 5, tanda 5b: mobiliario urbano de las ocho colecciones

Rama `bloque/paquete-5b` (sobre `bloque/paquete-5a`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio, con la terraza resuelta: «qué toca ahora?» → tanda 5b, lo urbano.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 5, tanda 5b»)
- Siete piezas de calle con un modelo por colección, añadidos después de los de siempre (los proyectos de antes no cambian): **banco**, **farola**, **papelera**, **bolardo**, **jardinera** (con las plantas de la terraza: olivo, gramíneas o lavanda, como el macetón de la colección), **aparcabicis** y **fuente de beber**.
- Las farolas se encienden de noche y se exportan como `IfcLightFixture`; la clásica es la farola de fundición del taller.
- Ocho calles en el taller, a continuación de las terrazas: banco, farola, papelera, dos bolardos, jardinera, aparcabicis de tres arcos y fuente.
- El banco de calle se llama «Banco de calle» en el taller, para no confundirse con el banco de dormitorio de cada colección.

**Pendiente**
- La revisión de Antonio de los paquetes 2 a 5.

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks) y abre `taller.archisim` (te lo mando).
2. En la Maqueta, las calles están a continuación de las terrazas: mira las siete piezas de cada colección, de día y de noche (las farolas encendidas).
3. Selecciona un banco de calle y quita el respaldo o alárgalo; selecciona una jardinera y alárgala: las plantas se reparten de nuevo.
4. Dime qué cambiarías, colección por colección.

---

## 2026-10-27 (11) · Tanda 5a, segunda vuelta: sombrillas y plantas de las terrazas

Rama `bloque/paquete-5a` (el mismo pull request). Sin cambios de formato; en el taller.

Antonio: «Por qué las sombrillas aparecen saliendo desde mitad de la butaca. Las macetas y las enredaderas no están al nivel de las plantas que hiciste al principio».

**Hecho**
- **Sombrillas**: en el taller cada sombrilla estaba en el mismo punto que su tumbona, así que el mástil salía por el centro. Ahora la de mástil se pone al lado del cabecero y le da sombra; las de brazo lateral (contemporánea e industrial) tienen el pie al lado y el toldo sobre el cabecero. El macetón y la silla suelta se apartan medio metro.
- **Plantas de los macetones**, rehechas hoja a hoja como la espatifilo y la pilea del taller (`fine::garden_plants`): un **olivo** de dos troncos retorcidos con cinco ramas, ramillas y hojas estrechas de dos verdes (haz y envés plateado); **gramíneas** con unas sesenta hojas largas que se arquean hacia fuera y espigas que cabecean; **lavanda** con su mata de hojas grises y tallos con espigas de flores. Mediterránea, contemporánea y clásica llevan olivo; nórdica, japandi e industrial, gramíneas; años 50 y rústica, lavanda.
- **Trepadora de las pérgolas** (rústica, industrial, clásica): una buganvilla con su tallo leñoso que sube enroscado al pilar y sigue por la viga, con brotes, hojas acorazonadas en sus tallitos, ramas que cuelgan donde va en horizontal y racimos de brácteas fucsia. Tiene sus propios materiales («Tallo de la trepadora», «Hojas de la trepadora», «Flores de la trepadora»), distintos de los de la planta del macetón.
- Materiales nuevos en la paleta: hoja y envés de olivo, corteza de olivo, gramínea, espiga, hoja de lavanda, corteza, hoja y flor de buganvilla.
- **Macetones**: la tierra queda 4 cm por debajo del borde, abombada y con algún terrón (en los cuadrados, plana), y el macetón tiene su borde; antes la maceta estaba maciza hasta arriba y la planta salía de un plano liso.
- **Tumbona contemporánea**: el larguero a la altura del asiento va ahora de la cabecera a los pies, así que las patas traseras llegan a él, y un tirante sube del larguero al respaldo; antes las patas acababan en el aire.
- **Vista 3D**: las hojas menudas (las de estas plantas) se dibujan sin líneas de contorno. Con contorno, cada hoja de olivo o de gramínea salía como un trazo oscuro y la planta se veía negra; ahora se ve su verde. Los muebles y las plantas de interior siguen como estaban.

**Qué debe probar Antonio**
1. Abre `taller.archisim` (te lo mando) en la versión de este pull request y ve a las terrazas.
2. Mira cada tumbona con su sombrilla: el mástil va al lado del cabecero, no a través de la tumbona.
3. Acércate a los macetones y a las pérgolas rústica, industrial y clásica: dime si el olivo, las gramíneas, la lavanda y la buganvilla están ya al nivel de las plantas de interior.

---

## 2026-10-27 (10) · Paquete 5, tanda 5a: jardín y terraza de las ocho colecciones

Rama `bloque/paquete-5a` (sobre `bloque/paquete-4`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio, con los paquetes 2 a 4 esperando su revisión: «Paquete 5: exterior y urbano». Esta tanda es el jardín y la terraza; lo urbano va en la 5b.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 5, tanda 5a»)
- Tres piezas nuevas con un modelo por colección: **mesa de exterior** (con sus sillas alrededor), **silla de exterior** y **sofá de exterior** (con cojines de asiento, de respaldo y sueltos).
- «Tumbona», «Sombrilla», «Macetón» y «Pérgola» ganan ocho modelos más, uno por colección, después de los suyos (los proyectos de antes no cambian). El macetón lleva las plantas de siempre (olivo, gramíneas, lavanda) en la maceta de cada colección.
- Ocho terrazas en el taller: la pérgola con la mesa y sus sillas debajo, el sofá, la tumbona con su sombrilla, el macetón y una silla suelta.

**Pendiente**
- Tanda 5b: lo urbano (bancos, papeleras, farolas, jardineras, bolardos).
- La revisión de Antonio de los paquetes 2 a 5.

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks) y abre `taller.archisim` (te lo mando).
2. En la Maqueta, las terrazas están entre las lámparas y los baños: mira mesa, sillas, sofá, tumbona, sombrilla, macetón y pérgola de cada colección.
3. Selecciona una mesa de exterior y cambia «Sillas alrededor» y el largo: las sillas se reparten de nuevo.
4. Dime qué cambiarías, colección por colección.

---

## 2026-10-27 (9) · Paquete 4: la iluminación de las ocho colecciones

Rama `bloque/paquete-4` (sobre `bloque/paquete-3`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca. El IFC gana `IfcLightFixture` para las familias que lo piden (validado).

Antonio, con cocina, baño y dormitorio esperando su revisión: «Paquete 4: iluminación».

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 4»)
- Cinco lámparas por colección: colgante, plafón, aplique, de pie y de mesa (la de mesa es la de su mesita de noche, a escala). Lo que es pantalla, difusor, papel, ratán, bombilla o llama se enciende de noche en la Maqueta.
- En el IFC son `IfcLightFixture` con su `IfcLightFixtureType` (punto de luz; el aplique, luz dirigida), y se leen de vuelta como lámparas.
- Ocho paredes de lámparas en el taller (cómoda con su lámpara, aplique, lámpara de pie, y colgante y plafón a 2,60 m); fotos de día y de noche.
- Material nuevo en la paleta: Papel de arroz.

**Pendiente**
- La revisión de Antonio de los paquetes 2, 3 y 4; después, a la biblioteca.
- Farolas y balizas de exterior van con el paquete 5.

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks) y abre `taller.archisim` (te lo mando).
2. En la Maqueta, las lámparas están entre los dormitorios y los baños: mira las cinco de cada colección.
3. En la barra de la Maqueta, cambia «Luz: Día» a «Noche» y mira cómo se encienden y cómo iluminan la pared.
4. Exporta el IFC y ábrelo en tu visor: las lámparas deben salir como luminarias.

---

## 2026-10-27 (8) · Paquete 3, segunda vuelta: el dormitorio completo

Rama `bloque/paquete-3` (la misma del pull request #86). Sin cambios de formato; en el taller, no en la biblioteca.

Antonio, tras ver los dormitorios: «Completar el dormitorio».

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 3, segunda vuelta»)
- Cama infantil en las ocho colecciones: la cama de la colección, más baja y estrecha, con una barrera en la mitad de la cabecera, su ropa de cama y un muñeco.
- Piezas nuevas en el taller, una por colección: **cuna** (barrotes, rejilla, lamas, panel o tubo alrededor del colchón; la de los años 50 y la clásica sobre patas), **litera** (dos camas sobre cuatro postes, barrera arriba y escalera), **galán de noche** (con un pantalón doblado en la barra; el mediterráneo y el clásico son silla galán) y **espejo de pie** (apoyado en la pared; el de los años 50 en caballete y el clásico basculante entre dos postes).
- Cada dormitorio del taller suma el galán y el espejo, y sigue la pared con el cuarto infantil: cama, cuna y litera.

**Pendiente**
- Lo mismo que en la entrada anterior: la revisión de Antonio de cocina, baño y dormitorio, y después los paquetes 2 y 3 a la biblioteca.

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request #86 (pestaña Checks) y abre `taller.archisim` (te lo mando de nuevo).
2. En cada dormitorio, a la derecha del tocador: el galán, el espejo de pie, la cama infantil, la cuna y la litera.
3. Dime qué cambiarías, colección por colección.

---

## 2026-10-27 (7) · Paquete 3: el dormitorio de las ocho colecciones

Rama `bloque/paquete-3` (sobre `bloque/paquete-2b`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio, con la cocina y el baño esperando su revisión: «Paquete 3: dormitorio».

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 3»)
- Cama, mesita de noche (con su lámpara), armario y cómoda hechos en las ocho colecciones, como modelos nuevos de las familias de siempre (los proyectos antiguos no cambian).
- Dos piezas nuevas en el taller: el banco a los pies de la cama y el tocador (mesa con su cajón, espejo en la pared y taburete), uno por colección.
- La ropa de cama de verdad: el colchón con su sábana, el edredón que cae por los lados y los pies con sus pliegues, la sábana vuelta sobre el edredón, almohadas, manta a los pies y cojines (en la clásica, además, almohadones de respaldo y un rulo).
- Ocho dormitorios montados en el taller, cada uno con su pared; fotos por colección.

**Pendiente**
- Que Antonio revise los dormitorios (y la cocina y el baño); con su visto bueno, los paquetes 2 y 3 a la biblioteca: el banco y el tocador como familias nuevas.
- Cuna, cama infantil y litera en las colecciones (la «Cama infantil» de la biblioteca sigue como está), galán de noche y espejo de pie.

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks) y abre `taller.archisim` (te lo mando).
2. En la Maqueta, a la derecha de los baños están los ocho dormitorios: mira la cama y su ropa, las mesitas con sus lámparas, el armario, la cómoda, el banco y el tocador de cada uno.
3. Selecciona una cama y cambia en Propiedades «Manta a los pies» y «Cojines», y el ancho: la ropa de cama se rehace.
4. Dime, dormitorio por dormitorio, qué cambiarías.

---

## 2026-10-27 (6) · Paquete 2, tanda 2b: el baño de las ocho colecciones

Rama `bloque/paquete-2b` (sobre `bloque/paquete-2a`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio, tras ver las cocinas: «sigue con el baño».

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Tanda 2b»)
- Lavabo, inodoro, bidé, ducha, bañera, espejo y toallero hechos en las ocho colecciones, con sus grifos (los de la cocina a escala de lavabo), mamparas o cortinas, rociadores, bañeras exentas, de patas, de obra o de madera, y toallas en los toalleros.
- Son modelos nuevos de las familias de siempre: los proyectos antiguos no cambian.
- Ocho baños montados en el taller, cada uno con su pared; fotos por colección.

**Pendiente**
- Que Antonio revise los baños (y las cocinas); con su visto bueno, el paquete 2 entero a la biblioteca.
- Grifería suelta (portarrollos, barra de apoyo) y mampara de esquina para baños accesibles en las colecciones.

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks) y abre `taller.archisim` (te lo mando).
2. En la Maqueta, a la derecha de las cocinas están los ocho baños: mira lavabo y grifo, inodoro y bidé, ducha, bañera, espejo y toallero de cada uno.
3. Dime, baño por baño, qué cambiarías.

---

## 2026-10-27 (5) · Paquete 2, tanda 2a: la cocina de las ocho colecciones

Rama `bloque/paquete-2a` (sobre `bloque/diseno`), con pull request. Sin cambios de formato; en el taller, no en la biblioteca.

Antonio eligió el paquete 2 (cocina y baño) antes que los clásicos de dominio público.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Tanda 2a»)
- La cocina por módulos de la biblioteca hecha en las ocho colecciones: módulos bajos (cajonera, puertas, fregadero, horno, lavavajillas, placa, botellero, esquina), columnas (despensa, horno y microondas, frigorífico), muebles altos o baldas, y campanas. Cada colección con sus frentes, tiradores, zócalo o patas, fregadero, grifo, placa, horno, campana y pared.
- Son modelos nuevos de las familias de siempre: los proyectos antiguos no cambian, y se siguen pegando con la encimera continua.
- Ocho cocinas montadas en el taller, cada una con su pared; fotos por colección.
- Arreglado del paquete 1: pomos, anillas y remates de barra de cortina, que estaban del revés.

**Pendiente**
- Que Antonio revise las ocho cocinas; con su visto bueno, a la biblioteca (familias con los modelos por colección y las colecciones que los eligen).
- Tanda 2b: el baño.
- Isla, frigorífico de pie y electrodomésticos sueltos en las colecciones (quedan los de la biblioteca).

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks) y abre el taller: `archisim taller salida` y abre `salida\taller.archisim` (o el fichero que te mando).
2. En la Maqueta, mira las ocho cocinas: frentes, tiradores, fregadero y grifo, placa, campana y baldas o armarios.
3. Selecciona un módulo y cambia «Colección» en la barra del objeto: el módulo cambia y sigue pegado a los de al lado con la encimera continua.
4. Dime, cocina por cocina, qué cambiarías.

---

## 2026-10-27 (4) · Diseño: los modelos de los fabricantes

Rama `bloque/diseno` (sobre `bloque/biblioteca-paquete-1`), con pull request. Sin cambios de formato; el IFC gana `Pset_ManufacturerTypeInformation` en las piezas de diseño (validado).

Antonio eligió seguir con la importación de modelos de diseño (lo acordado tras el paquete 1) antes del paquete 2.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Diseño: los modelos oficiales de los fabricantes»)
- ArchiSim lee **FBX** (binario y de texto), **3DS** y **STL**, además de OBJ, glTF y COLLADA, y abre directamente el **.zip** que se descarga del fabricante (elige el mejor modelo de dentro, encuentra sus texturas y dice qué trae que no lee: SketchUp, Revit, ArchiCAD…).
- Unidades supuestas por el tamaño cuando el fichero no las dice (OBJ, 3DS, STL).
- **Aligerar sin perder la forma**: los modelos de cientos de miles de caras bajan a 50.000 (o lo que se elija) colapsando aristas; también las versiones de lejos de todos los modelos importados, que antes se deformaban.
- Diálogo de importar nuevo: planta y alzado con el frente marcado y botones de girar, detalle, y «Pieza de diseño de un fabricante» con fabricante, modelo, referencia, diseñador, año y enlace, que van al tipo y al IFC; «Guardarla en mi biblioteca» la deja en `Modelos 3D/Diseño/<Fabricante>/`.
- Pestaña **Diseño** en Amueblar (Maqueta) y carpeta «Diseño» en la galería, con el fabricante y el diseñador en la nota.
- Materiales del modelo con la textura de ArchiSim que dice su nombre (cuero, tela, madera, mármol…), en español, inglés, italiano, alemán y francés.
- `archisim diseno <modelo o .zip> [carpeta] [Fabricante]`: lo mismo por línea de órdenes, diciendo qué ha leído.
- Comparativa de aligerado (sofá clásico de 33.948 caras a 4.000 y 1.200, antes y ahora) enviada a Antonio.

**Pendiente**
- SketchUp (.skp), Rhino (.3dm) y DWG 3D no se leen (formatos cerrados o sin especificación libre): hay que pedir al fabricante FBX, OBJ o 3DS.
- Aligerar un modelo muy pesado que se deja tal cual en la carpeta de la biblioteca se hace al abrir la galería (unos segundos la primera vez); lo que se importa con el diálogo ya se guarda aligerado.
- Paquete «Diseño» de clásicos de dominio público (Thonet nº 14, Calvet, Hill House…) y paquete 2 (cocina y baño).

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks).
2. Descarga de la web de un fabricante el modelo 3D de una pieza (por ejemplo Vitra, Fritz Hansen, Cassina, Knoll, Kartell…) en FBX, OBJ o 3DS; el .zip tal cual vale.
3. Arrastra el .zip a la ventana de ArchiSim (o Archivo → Importar modelo 3D). Mira en el diálogo: medidas, unidades, planta y alzado. Si el frente no queda abajo en la planta, pulsa «Girar».
4. Activa «Pieza de diseño de un fabricante», rellena fabricante, modelo y diseñador, deja «Guardarla en mi biblioteca» e importa. Colócala y mírala en la Maqueta.
5. Exporta el IFC y comprueba en tu visor (o en ArchiCAD) que la pieza lleva fabricante y modelo.
6. En un proyecto nuevo, Maqueta → Amueblar → Diseño: la pieza debe estar ahí.
7. Si algo se lee mal, pásame el nombre del fabricante y el formato, o ejecuta `archisim diseno ruta\al\fichero.zip salida` y pégame lo que dice.

---

## 2026-10-27 (3) · El paquete 1 entra en la biblioteca

Rama `bloque/biblioteca-paquete-1` (sobre `bloque/rendimiento-3d`), con pull request. Sin cambios de formato; el IFC solo cambia por los modelos nuevos (validado).

Antonio, tras ver el paquete 1 en 3D y pedir el rendimiento: «Perfecto, seguimos con lo siguiente», que era llevar el paquete a la biblioteca, añadir las colecciones nuevas y quitar los modelos antiguos.

**Hecho** (detalle en `docs/BIBLIOTECA_3D.md`, «Paquete 1 en la biblioteca»)
- Las 14 familias del salón y comedor (sofá, butaca, puf, silla, taburete, mesa, mesa de centro, mesa auxiliar, aparador, estantería, mueble de TV, alfombra, cortina y chimenea) dibujan con los modelos nuevos; su «Modelo 3D» son las ocho colecciones. Familia nueva: **Vitrina**. Piezas nuevas: «Vitrina 90» y «Chimenea colgante».
- La mesa lleva sus sillas alrededor, ahora las de su colección (la clásica, redonda, alrededor del pie); el sofá, cojines y una manta doblada; la estantería, sus baldas.
- Colecciones nuevas: **Contemporáneo, Rústica, Industrial y Clásica**. Una pieza en una colección toma el modelo de esa colección, salvo que tenga el suyo.
- Los modelos antiguos ya no se ofrecen; su código sigue para abrir los proyectos de antes. Al actualizar una familia, un modelo que ya no existe pasa al de por defecto (`reapply_family`; prueba `an_old_model_goes_back_to_the_default`).
- El sofá rinconera sigue con sus modelos de antes: al juntar dos sofás nuevos en L, toma el más parecido.
- El taller se queda con la planta, el fregadero y la farola. Las pruebas del 3D usan ahora el salón de la biblioteca en las ocho colecciones (`collections_scene`, `scene_benchmark`); `ARCHISIM_COLECCIONES_OUT=carpeta cargo test --release -p archisim-app -- collection_pictures` hace una foto por colección.

**Pendiente**
- Sofá rinconera con los modelos nuevos.
- Afinar los colores de cada colección pieza a pieza si Antonio lo pide (la biblioteca los pone por categoría: madera, tela, metal…, no a mano como en el taller).
- Siguiente bloque: importación mejorada de modelos oficiales de fabricantes (carpeta «Diseño») y paquete 2 (cocina y baño).

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks).
2. Proyecto nuevo: en la Maqueta, Amueblar, coloca sofás, mesas, aparadores, alfombras… y cámbialos de colección (las ocho).
3. Abre un proyecto antiguo con muebles: se ve como antes. Si en la Biblioteca pulsas «Actualizar familias», sus muebles pasan a los modelos nuevos.

---

## 2026-10-27 (2) · Rendimiento 3D: la Maqueta con proyectos grandes

Rama `bloque/rendimiento-3d` (sobre `bloque/paquete-1a`), con pull request. Sin cambios de formato ni de IFC (el IFC sale de `archisim-geom`, que solo gana `object_instance` y `model_solids_except`).

Antonio preguntó por el límite de 256 MB y cómo optimizar los modelos y el programa; decidió (27/10/2026) adelantar este bloque al paquete 2, con las llamas planas.

**Hecho** (todo en `archisim-app::view3d`)
- **Esquinas compartidas y compactas**: cada esquina se guarda una vez y los triángulos dicen cuáles unen (índices); la esquina ocupa 48 bytes en vez de 72 (normal en 16 bits, color sRGB en 8 bits, datos sueltos en bytes). `scene::Vertex::new`, `Mesh`, `MeshBuilder`.
- **Muebles por tipo una sola vez**: los objetos que ArchiSim fabrica (todos los de la biblioteca salvo plantas y modelos importados) se suben a la tarjeta una vez por tipo (`Group`) y se dibujan donde está cada uno (`Instance`: giro, posición, corte, selección y atenuación por disciplina en el sombreador). `archisim_geom::solid::object_instance` da el modelo en sus ejes y `model_solids_except` el resto.
- **Menos detalle a lo lejos**: cada modelo de más de 1.500 triángulos lleva dos versiones más bastas (agrupando esquinas cercanas del mismo color, `coarser`), y cada objeto se dibuja con la que corresponde a lo que ocupa en pantalla (`level_for`). Las sombras usan la intermedia.
- **Nada fuera de la vista**: los elementos dibujados una vez van en trozos de 32.768 triángulos con su caja, y lo que cae fuera de la cámara no se manda a dibujar (`frustum`, `in_sight`), ni los objetos.
- La selección por rayo, las cajas de los elementos y el ajuste a vértices en 3D recorren también los objetos colocados (`Scene::hit_in` devuelve el triángulo, `Hit`).
- **Llamas planas**: cada lengua es una silueta plana en dos planos cruzados (a ±35° del frente), con su corazón más claro (papel `flame2`, «Llama clara»); unas 40 caras por llama en vez de unas 200. En la chimenea, «Con llamas» y «Con leña» por separado (el material de la leña pasa a llamarse `wood`).

**Medidas** (`ARCHISIM_MEDIR_3D=1 cargo test --release -p archisim-app -- scene_benchmark --nocapture`; tarjeta por software, así que los tiempos valen para comparar):

| | antes | ahora |
|---|---|---|
| Taller (242 objetos): memoria en la tarjeta | 608 MB | 85 MB |
| Taller: fotograma | 726 ms | ~140 ms |
| Proyecto repetido (602 objetos, 12 tipos ×30 más el taller): memoria | 1.311 MB | 85 MB |
| Proyecto repetido: fotograma | 1.784 ms | ~230 ms |

- Ventana real abierta con Xvfb: el taller completo en la Maqueta, sin cerrarse, de lejos y acercándose.

**Pendiente**
- Las imágenes recortadas con transparencia (hojas, visillos con dibujo) siguen en el bloque técnico A; las llamas no las necesitaron.
- Construir la escena del taller sigue tardando unos 2 s (los modelos y sus normales); se puede guardar por tipo entre cambios si molesta.

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de este bloque (pestaña Checks).
2. Abre el `taller.archisim` completo y entra en la Maqueta: muévete, acércate y aléjate. Dime si va fluido en tu portátil y si notas algún mueble que cambie raro al alejarte.
3. En una chimenea, apaga «Con llamas» y deja «Con leña» (para D5 Render).

---

## 2026-10-27 · Paquete 1: la vista 3D ya no se cierra con proyectos grandes; alfombra años 50 y chimenea colgante

Rama `bloque/paquete-1a` (pull request #79). Sin cambios de formato ni de IFC.

Antonio (27/10/2026, madrugada): al abrir `taller.archisim` la aplicación se cerraba al entrar en la vista 3D, y tuvo que borrar la mitad de las piezas para verlo; todo bien salvo la alfombra años 50, «rara»; echa en falta una chimenea colgante «rollo moderna nórdica» (mandó la foto de una Gyrofocus).

**Hecho**
- **Vista 3D con modelos grandes**: la escena del taller son 8,7 millones de vértices (600 MB) y la tarjeta solo admite memorias de 256 MB por bloque, así que wgpu paraba el programa. Ahora cada clase de triángulos se sube en bloques de 128 MB como mucho (`view3d::render::upload`); prueba `workshop_scene_size`, que dibuja el taller entero.
- **Alfombra años 50** rehecha: círculos de dos colores repartidos en diagonal, sin tocarse, los mayores con un aro (sin las esquirlas, que se veían como astillas levantadas); las piezas del dibujo quedan casi a ras.
- **Chimenea colgante** (modelo «Colgante» de «Chimenea (taller)», pieza «Chimenea colgante · Nórdico cálido»): fuego abierto en un cuenco redondo de acero bajo una campana acampanada, colgados del tiro en medio de la sala, con tres tirantes, nada apoyado en el suelo. La Gyrofocus (Focus, Dominique Imbert) es un diseño protegido, así que no se copia su forma (cuerpo cerrado en gota con una boca); la nuestra es la tipología general de hogar central suspendido.
- **Llama**: color nuevo de la paleta para el fuego de todas las chimeneas, que con «Luz» casi no se veía sobre fondo claro.

**Pendiente**
- Lo mismo de la entrada anterior: aprobación del paquete 1 y llevarlo a la biblioteca.
- Con 600 MB de escena, la Maqueta irá pesada en un portátil: hará falta dibujar con menos detalle los objetos lejanos (niveles de detalle por distancia), sobre todo en el taller, que tiene dos copias de todo.

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request #79 (pestaña Checks) cuando termine.
2. Abre el `taller.archisim` completo (sin borrar nada) y entra en la vista 3D: ya no debe cerrarse. Dime si va muy lento.
3. Mira la alfombra años 50 y la chimenea colgante, al final de la fila de chimeneas.

---

## 2026-10-26 (7) · Paquete 1c: almacenaje, alfombras, cortinas y chimeneas; correcciones de Antonio

Rama `bloque/paquete-1a` (pull request #79, para que Antonio descargue la aplicación). Sin cambios de formato ni de IFC; es el taller.

Antonio miró en 3D las tandas 1a y 1b y pidió corregir siete piezas que flotaban o se atravesaban (hecho, ver la entrada anterior); después: «Perfecto, sigamos con la biblioteca».

**Hecho**
- Tanda 1c: 56 modelos más, una pieza por colección: aparadores, librerías (con libros), muebles de TV (con televisor), vitrinas (con cerámica), alfombras, cortinas y chimeneas (detalle en `docs/BIBLIOTECA_3D.md`, «Tanda 1c»). Con esto el paquete 1 (salón y comedor) está completo en el taller (124 piezas en total con la planta, el fregadero y la farola).
- Frentes, tiradores y patas reutilizables para los muebles de los paquetes siguientes (`fine::storage`: `Panel`, `Pull`).
- La foto del taller encuadra mejor las piezas altas.
- IFC del taller validado.

**Pendiente**
- Lo que diga Antonio del paquete 1 completo. Al aprobarlo: llevar las piezas a la biblioteca, añadir las colecciones contemporánea, rústica, industrial y clásica a `archisim_library::collections` y quitar los modelos antiguos.
- Después: importación mejorada de modelos oficiales de fabricantes (carpeta «Diseño») y paquete 2 (cocina y baño).

**Qué debe probar Antonio**
1. Mira las láminas que te mando en el chat.
2. Descarga ArchiSim-Windows del pull request #79 (pestaña Checks), abre `taller.archisim` y recorre en la Maqueta las filas nuevas; en las cortinas, cambia «Abierta» de 0 a 1; en las librerías, apaga «Con libros».
3. Pon la luz de noche en la Maqueta y mira el fuego de las chimeneas.

---

## 2026-10-26 (6) · Paquete 1b: sillas, taburetes y mesas de las ocho colecciones

Rama `bloque/paquete-1a` (la misma de la tanda 1a). Sin cambios de formato ni de IFC; es el taller, no entra nada en la biblioteca.

Antonio, al ver la tanda 1a: «me parece muy bueno hasta ahora». Decidió además (26/10/2026) que el mobiliario de diseño protegido (LC2, Togo, Eames Lounge, Barcelona, Egg, Wassily…) no se modela ni con otro nombre: se mejorará la importación de los modelos oficiales de los fabricantes (carpeta «Diseño» de la biblioteca, tras el paquete 1), habrá un paquete «Diseño» de clásicos de dominio público (Calvet, Thonet nº 14, Hill House, Café Museum, Postsparkasse, Windsor, bergère Luis XV) y piezas propias con el gesto de cada época.

**Hecho**
- Ocho **sillas**, ocho **taburetes** y 24 **mesas** (comedor, centro y auxiliar), una por colección (detalle en `docs/BIBLIOTECA_3D.md`, «Tanda 1b»).
- Prueba nueva: los materiales de cada pieza del taller existen en la paleta.
- Correcciones tras la primera revisión de Antonio en 3D: el cojín de las butacas nórdica y contemporánea ya no atraviesa el respaldo; en la silla contemporánea el respaldo va sobre las barras; en la rústica las tablillas llegan al asiento; en la industrial el aro une las patas y el respaldo se sujeta con dos pletinas; en la clásica los montantes suben a sujetar el medallón y un husillo lo apoya en el asiento; en la mesa de comedor industrial la barra une los dos marcos bajo el tablero.
- IFC del taller validado (68 piezas nuevas delante de las que sustituirían).

**Pendiente**
- Lo que diga Antonio de las sillas, los taburetes y las mesas.
- Tanda 1c: almacenaje (aparadores, librerías, muebles de TV, vitrinas), alfombras, cortinas y chimeneas por colección.
- Al aprobar el paquete 1: llevarlo a la biblioteca, añadir las colecciones nuevas a `archisim_library::collections` y quitar los modelos antiguos.

**Qué debe probar Antonio**
1. Mira las láminas de mesas que te mando en el chat.
2. `git checkout bloque/paquete-1a` y `cargo run --release -p archisim-cli -- taller out/taller`; abre `out/taller/taller.archisim` y en la Maqueta recorre las filas de sillas, taburetes y mesas.
3. Selecciona una mesa de comedor y cambia el «Modelo 3D», el largo y el ancho; comprueba que las patas y los travesaños siguen al tablero.

---

## 2026-10-26 (5) · Paquete 1a: sofás, butacas y pufs de las ocho colecciones

Rama `bloque/paquete-1a`, encima de `bloque/taller-2` (#76). Sin cambios de formato ni de IFC; es el taller, no entra nada en la biblioteca.

Antonio aprobó la calidad del taller y decidió (26/10/2026): biblioteca entera así, por paquetes (1 salón y comedor … 7 comercio, hostelería y oficinas), **por colecciones** (ocho: nórdico cálido, mediterráneo, años 50, japandi, contemporáneo, rústica, industrial y clásica), quitando los modelos antiguos al aprobar cada paquete, y los bloques técnicos (copas de árbol, luz de la Maqueta) al final. Inventario en el documento «Biblioteca de objetos de ArchiSim: inventario frente a Paralives».

**Hecho**
- Ocho **sofás**, ocho **butacas** y ocho **pufs**, uno por colección (detalle en `docs/BIBLIOTECA_3D.md`, «Tanda 1a»). Antonio vio los sofás y le parecieron bien.
- **Capitoné** en la tela rellena: botones en diamante con sus pliegues.
- Materiales nuevos de la paleta para las colecciones nuevas.
- IFC del taller validado (28 piezas nuevas delante de las que sustituirían).

**Pendiente**
- Lo que diga Antonio de las butacas y los pufs.
- Tanda 1b: sillas, taburetes y mesas por colección; 1c: almacenaje, alfombras, cortinas y chimeneas.
- Las colecciones contemporánea, rústica, industrial y clásica en `archisim_library::collections` (hoy solo existen en el taller), al meter el paquete en la biblioteca.

**Qué debe probar Antonio**
1. Mira las láminas de butacas y pufs que te mando en el chat.
2. `git checkout bloque/paquete-1a` y `cargo run --release -p archisim-cli -- taller out/taller`; abre `out/taller/taller.archisim` y en la Maqueta recorre las filas: delante cada pieza nueva, detrás la de la biblioteca.
3. Selecciona una butaca y cambia el «Modelo 3D» entre colecciones, el ancho y la altura del respaldo.

---

## 2026-10-26 (4) · M1, segunda entrega del taller: planta, fregadero y farola

Rama `bloque/taller-2`, encima de `main` (v0.70), con pull request para Antonio. Sin cambios de formato (la familia Vegetación no cambia) y sin versión nueva: es el taller, no entra nada en la biblioteca.

Pedido por Antonio el 26/10/2026 con ocho capturas, cuatro nuestras y sus contrapartidas de Paralives: «las hojas son planos estáticos antinaturales, los cojines no se recuestan sobre los respaldos, el fregadero son unos tubos extruidos que ni se tocan, en la farola hay elementos al aire… quiero que sean visualmente preciosos». Pidió también decompilar la biblioteca de Paralives para remodelarla: no se hace (sus modelos son de su estudio y su licencia no lo permite; principio de sala limpia), así que los modelos se hacen desde cero con el mismo cuidado.

**Hecho**
- **Planta de interior (taller)**, dos modelos: **espatifilo** en maceta esmaltada (hojas lanceoladas con nervio, venas, pliegue en V, punta caída y giro; peciolos que salen de la tierra y siguen como nervio de la hoja; dos flores blancas) y **pilea** en maceta de barro con plato (hojas redondas sujetas por debajo en peciolos largos). Maceta con grueso, borde y cara interior; tierra con terrones.
- **Módulo fregadero (taller)**: el módulo de la biblioteca con fregadero de acero encastrado (ceja sobre la encimera, seno de esquinas redondeadas, fondo hacia la válvula, cestillo) y **grifo de caño alto de una pieza** con la maneta naciendo del cuerpo. El mueble ya no se rebaja bajo el seno.
- **Farola de fundición (taller)**: basa ochavada, un solo torneado de la basa al capitel, cruceta, volutas bajo la bandeja y farol de cuatro caras con montantes, cristales con junquillo, bandas, tejadillo curvo y remate. Nada al aire. Se enciende de noche.
- Formas nuevas para todo el taller (`fine::shape`): tubos por curvas suaves, torneados de n lados, placas, cáscaras por rectángulos redondeados y hojas con grueso.
- La prueba del taller vigila un máximo de triángulos por pieza: planta ≤ 17.000 (espatifilo 16.600, pilea 14.650; las de la biblioteca, 1.600), fregadero ≤ 14.000 (5.500; antes 3.000), farola ≤ 12.000 (5.700; antes 1.200). Un sofá tiene 18.500.
- Fotos de estudio: encuadre por el tamaño real del modelo y `ARCHISIM_TALLER_SOLO` para hacer solo unas.
- **Cojines**: lo que pedías ya está en el sofá del taller de la entrega anterior (cojines de respaldo recostados, cojines sueltos apoyados y girados), pendiente de tu visto bueno.
- **Árbol**: probado y descartado por ahora (ramilletes de hojas sueltas: de lejos mejora, de cerca sigue facetado y pesa el triple). Lo que hace bonitos los árboles de Paralives son hojas pintadas en una textura con transparencia; queda propuesto como bloque propio.

**Pendiente**
- Lo que diga Antonio de las tres piezas: proporciones, colores, si las plantas y la farola están a la altura.
- Bloque propuesto: copas de árbol con hojas recortadas por textura (vista 3D, render y `.glb`).
- Bloque propuesto: la luz de la Maqueta (la calidez y la penumbra de los rincones que se ve en Paralives; hoy la penumbra solo sale en calidad «Bonita»).
- Cuando haya paquete aprobado: sustituir en la biblioteca la planta de interior, el bajo fregadero y la farola fernandina.

**Qué debe probar Antonio**
1. Mira las fotos del chat (antes y después de cada pieza, con la misma cámara).
2. `cargo run --release -p archisim-cli -- taller out/taller` y abre `out/taller/taller.archisim`. En la Maqueta (Tab), gira alrededor de cada pieza nueva (delante) y la de la biblioteca (detrás). Pon la luz en Noche y mira la farola.
3. Selecciona la planta y cambia en Propiedades el «Modelo 3D» entre Espatifilo y Pilea, la altura y el diámetro de la maceta.
4. Selecciona el fregadero y cambia el ancho (0,60 – 1,20), las puertas y el tirador.
5. Abre `out/taller/taller.ifc` en ArchiCAD o Revit: las piezas nuevas deben verse con su forma, como mobiliario.

---

## 2026-10-27 · v0.73, segunda vuelta: cortes macizos en todo, galería del modelo 3D, navegación 3D

Misma rama `bloque/e16-envolver` (PR #80), con lo que Antonio vio al probarla: los muros se cortan bien, pero faltaban forjados y objetos; la galería de tipos crecía sola; faltaba elegir el estilo 3D viendo la imagen; el zoom 3D se paraba y el desplazamiento con la rueda iba duro. Sin cambios de formato ni de IFC.

**Hecho**
- **Forjados y cubiertas cortados, macizos y por capas**: igual que los muros, cada capa con su color de corte y una línea entre capas (antes se veía el interior: la cara inferior del forjado).
- **Objetos cortados, macizos**: un mueble, un electrodoméstico, una persona o un coche cortado por la caja de sección enseña su sección rellena en el plano del corte, con el color de corte de sus piezas. Un módulo de cocina cortado a 60 cm se ve como un bloque, no como la caja vacía de sus tableros; un sofá cortado por el respaldo y los brazos sigue enseñando el asiento entre ellos. Las hojas de los árboles se quedan como están.
- **La galería de tipos ya no crece sola** hacia la derecha (pasaba al elegir tipo de muro, forjado… desde que tiene carpetas).
- **Estilo 3D en imagen**: en Propiedades de un objeto, «Tipo», las opciones que cambian su modelo 3D (Modelo 3D de muebles y sanitarios, tirador y frentes de la cocina; porte, forma de la copa y follaje de la vegetación) se eligen en una galería con el 3D de cada opción, como las tarjetas de la Maqueta.
- **Zoom en 3D**: la rueda acerca hacia lo que hay bajo el cursor (no hacia un punto fijo que podía quedar lejos) y nunca se queda parada: cerca de una superficie va despacio, y si se sigue, la atraviesa.
- **Desplazar con la rueda pulsada**: lo que se agarra va pegado al cursor, esté cerca o lejos.

**Pendiente**
- La galería con imagen para el dibujo en planta de los árboles (estilo técnico, de presentación, color) y para los demás desplegables: es el bloque E2.
- Los objetos solo se rellenan con la caja de sección (con «Muros recortados» de la Maqueta los objetos no se cortan, como antes).

**Qué debe probar Antonio**
1. Vista 3D → caja de sección por un lado del edificio: el forjado y la cubierta deben verse con sus capas, sin huecos. Baja la tapa de la caja a 0,60 m sobre la cocina y el salón: los muebles cortados deben verse macizos.
2. Selecciona un muro y abre el selector de tipo: la galería debe quedarse quieta.
3. Selecciona un sofá y, en Propiedades → Tipo, haz clic en «Modelo 3D»: debe salir una imagen por modelo. Lo mismo con un árbol en «Porte» y «Forma de la copa».
4. En 3D, acércate con la rueda a una ventana o a un mueble: debe acercarse a lo que señalas sin pararse. Desplaza con la rueda pulsada: lo que agarras debe seguir al ratón.

---

## 2026-10-26 (6) · v0.73: envolver capas y cortes macizos en 3D (E16)

Rama `bloque/e16-envolver`, montada sobre `bloque/e14-modificar` (#78), con pull request para Antonio. **Formato `.archisim` versión 41** (el envolvente de los tipos de muro; abre todas las anteriores). Pedido por Antonio el 26/10/2026: «Envolver en extremos» como Revit, también en las jambas de puertas y ventanas y en 3D; y que lo cortado en 3D se vea macizo.

**Hecho**
- **Envolver** en el tipo de muro (Tipos y materiales → muro → grupo «Envolver»), con dos opciones de cuatro valores (Ninguno, Exterior, Interior, Ambos):
  - **En extremos**: en un extremo libre (cuadrado, también los de «No permitir unión»), el acabado da la vuelta y tapa los cantos de las demás capas, en una franja tan ancha como él. Con Ambos, el exterior tapa hasta el acabado interior.
  - **En huecos**: el acabado entra en las jambas, el dintel y el alféizar de puertas y ventanas hasta la carpintería (el exterior, desde la fachada hasta el marco; el interior, desde la habitación hasta el marco).
  - En **planta**, **3D** (y por tanto secciones, alzados, render y `.glb`) e **IFC** (en un grupo de propiedades propio del tipo, `ArchiSim_WallType`, que vuelve al importar; la forma del muro en IFC no cambia, como en Revit). En 3D se decide por capas enteras: una capa entra en el remate si su mitad queda por fuera del marco.
  - Las fachadas que ya había siguen como se dibujaban: el revoco exterior envuelve los huecos y los extremos no.
- **Lo cortado en 3D se ve macizo** (caja de sección y muros recortados de la Maqueta). Antes se veía la cara interior del otro lado del muro, y parecía hueco. Ahora:
  - **Capas** (por defecto): cada capa con su color de corte, el mismo del poché de la planta, y una línea fina entre capas.
  - **Blanco, Negro o Gris**: todo lo cortado de un color liso.
  - Se elige en Propiedades de la vista 3D → Caja de sección → «Lo cortado», y en la Maqueta junto a los botones de muros recortados.

**Pendiente**
- Los tipos de muro guardados en la biblioteca (`.astipos`) no llevan todavía el envolvente (tampoco llevan los accesorios).
- En 3D, una capa partida por el marco entra entera o no entra (en planta el remate llega justo al marco).
- Los forjados cortados en 3D se ven macizos pero sin línea entre sus capas.

**Qué debe probar Antonio**
1. Descarga la versión del pull request. Abre la demo y pon la vista 3D con la caja de sección activa (Propiedades → Caja de sección → «Planta en 3D»): los muros cortados deben verse macizos, con el ladrillo y los enlucidos de su color. Cambia «Lo cortado» a Negro y a Blanco.
2. En la Maqueta, muros «Recortados»: lo mismo, con el selector junto a los botones.
3. Tipos y materiales → un muro de fachada → «Envolver»: pon «En extremos: Exterior». Dibuja un muro suelto de ese tipo: en planta y en 3D el revoco debe dar la vuelta en las puntas.
4. «En huecos: Interior» en un tipo con ventana: el enlucido debe entrar en las jambas hasta el marco. Prueba Ambos.
5. Exporta el IFC y vuelve a importarlo: el tipo debe conservar sus opciones.

---

## 2026-10-26 (5) · v0.72: Modificar como Revit, uniones, atajos de Revit y anclajes de los objetos (E14, E15)

Rama `bloque/e14-modificar`, pull request #78. **Formato `.archisim` versión 40** (los extremos de muro que no se unen; abre todas las anteriores); sin cambios de IFC; la familia Ventana practicable sube a la versión 8. Segunda vuelta el mismo día, tras probar Antonio la primera: Reflejar como Revit, deshacer, permitir unión, atajos de Revit y Tab. Antonio dio el visto bueno a E1 (unido a `main` como v0.71, #77) y respondió D25–D28.

**Hecho**
- **Cuatro órdenes nuevas en Modificar**, en un grupo propio al final de la barra (no hace falta seleccionar antes, como en Revit):
  - **Alinear caras** (`ALC`): la línea de referencia (cara o eje de un muro, un eje, un plano de referencia, una línea 2D, el canto de cualquier cosa) y después la cara del elemento que se lleva a ella. Se mueve en perpendicular, como al arrastrarlo: los muros unidos se alargan para seguirlo.
  - **Esquina** (`ESQ`, la TR de Revit): dos muros, cada uno por la parte que se queda; se alargan o se recortan hasta cerrar la esquina. Sigue activa para la siguiente esquina.
  - **Hasta límite** (`LIM1`) y **Varios al límite** (`LIM`): el límite (eje de un muro, eje, plano de referencia o línea) y después el muro o los muros, por la parte que se queda.
- **Reflejar como Revit** (sustituye a Simetría en la barra): con lo seleccionado, un clic en una línea (cara o eje de muro, eje, línea 2D) la toma como eje y sale la copia reflejada (MM); con la opción D, el eje se dibuja con dos puntos (DM); C cambia entre copiar y mover. La Simetría de AutoCAD (SI, con sus tres pasos) sigue en el buscador.
- **Deshacer**: Ctrl+Z no funcionaba mientras un botón de la barra (Esquina, por ejemplo) tenía el foco del teclado; ahora sí. Y U (DESHACER) escrito dentro de una orden deshace el último paso sin salir de ella.
- **Permitir / No permitir unión** en cada extremo de muro, como Revit: clic derecho cerca del extremo, o en Propiedades → Construcción → «Unión al inicio» / «Unión al final». Un extremo sin unión queda libre y cuadrado aunque toque otro muro (ninguno de los dos se ingletea ni hace T ahí), y al mover un muro el otro no lo sigue.
- **Atajos de teclado como Revit**, por defecto: dos letras y la herramienta empieza sin Intro (WA muro, DR puerta, WN ventana, CM componente, SB suelo, CL pilar, BM viga, GR rejilla, RP plano de referencia, DI cota, EL cota de nivel, TX texto, RM habitación, TG etiquetar, MV mover, CO copiar, RO rotar, RE escala, MM y DM reflejar, AR matriz, AL alinear, TR esquina, OF desfase, MA igualar propiedades, DE borrar, ZA/ZE/ZF zoom, TL líneas finas, HH/EH ocultar, HR mostrar, PT pintar, SF dividir cara; y algunos propios: RF cubierta, ST escalera, RA barandilla, CW muro cortina, FT cimentación, SE sección). **KS** abre la lista (también en el menú, «Atajos de teclado y ratón»). En Vista → Teclas se elige Revit, AutoCAD (orden + Intro) o ArchiCAD (una letra).
- **Tab ya no cambia a la Maqueta**: solo recorre los elementos que se pisan bajo el cursor, como en Revit. La Maqueta es **F4** (o el interruptor de arriba).
- **Simetría** ya vale para objetos, pilares, vigas, barandillas, ejes y planos de referencia. Un mueble va a su sitio simétrico, girado para mirar al lado simétrico (no se construye «al revés»). **Escala** se apaga cuando lo seleccionado no se puede escalar, en lugar de fallar.
- **Puntos de anclaje de los objetos en planta**: las esquinas, los puntos medios de los lados, las aristas y el centro de la caja de cada objeto, y su punto de inserción, son puntos de forzado. Valen para arrastrar, Mover, Copiar, Girar, Simetría y Matriz desde una esquina exacta. Los árboles solo dan su tronco.
- **D28**: el giro de las hojas de las ventanas no se dibuja en planta salvo que el tipo lo pida («Giro de las hojas en planta»), y entonces va con guiones.
- Lo de los cubos en el taller: la planta, el fregadero y la farola nuevos están en la rama `bloque/taller-2` de la otra conversación; una versión sin esa rama no tiene esos modelos y los dibuja como cajas. No se ha perdido nada.

**Pendiente**
- E15 en alzado, sección y 3D: hoy en esas vistas no se mueve con puntos de forzado (en 3D se arrastra sobre el suelo; en alzado y sección no se edita). Va con E7 (vistas de dibujo) o en un bloque propio, como diga Antonio.
- Recortar y extender muros curvos.
- «Envolver en extremos» (Revit): que la capa de acabado dé la vuelta en un extremo libre. Hoy un extremo sin unión queda cuadrado con todas las capas cortadas; hace falta que Antonio diga si va por tipo de muro (Ninguno / Exterior / Interior, como Revit) y si también en las jambas en 3D.

**Qué debe probar Antonio**
1. Descarga la versión del pull request y abre la demo. En proyectos tuyos, **Biblioteca → Actualizar familias**.
2. Dibuja dos muros que no se toquen, en ángulo. Barra de arriba → **Esquina**: pincha el primero y el segundo; la esquina se cierra. Haz lo mismo con dos muros que se cruzan, pinchando la parte que quieres conservar.
3. Un muro largo como límite y tres tabiques que no llegan o que se pasan: **Varios al límite** → pincha el muro largo y después cada tabique; Intro.
4. **Alinear caras**: pincha la cara de un muro y después la cara paralela de otro muro: se coloca a haces. Prueba también con un mueble contra la cara de un muro.
5. Coloca un mueble y empieza un Mover: al acercarte a sus esquinas deben salir los forzados. Haz una **Simetría** de un mueble: debe quedar en el lado simétrico mirando hacia el otro lado.
6. Ventanas: en planta, sin giro.
7. **Reflejar**: selecciona un muro o un mueble, MM y pincha la cara de otro muro: sale la copia reflejada en un solo clic. DM y dos clics: eje dibujado.
8. **Deshacer**: haz una Esquina con el botón de la barra y pulsa Ctrl+Z: se deshace. Dentro de la orden, U + Intro también.
9. **Permitir unión**: dos muros en L; clic derecho cerca de la esquina sobre uno → «No permitir unión en este extremo»: los dos quedan cuadrados, solapados. Mueve uno: el otro no lo sigue. Vuelve a permitirla desde el mismo menú o en Propiedades.
10. **Atajos**: escribe WA, DR, MV, MM, AL, TR… sin Intro. KS para la lista. Comprueba que Tab sobre dos elementos que se pisan ya no te lleva a la Maqueta (F4 sí). Editar tipo → Representación → «Giro de las hojas en planta»: aparece a guiones.

---

## 2026-10-26 (4) · v0.71: arreglos rápidos de la lista del 26/10/2026 (E1)

Rama `bloque/d1-arreglos`, con pull request para Antonio. Sin cambios de formato `.archisim` ni de IFC; cinco familias de puertas y ventanas suben de versión. Antonio entregó el 26/10/2026 una lista larga de mejoras; está contrastada con lo que ya hay y ordenada en bloques E1–E13 en `docs/HOJA_DE_RUTA.md` §10, con cuatro decisiones nuevas (D25–D28). Este bloque hace lo rápido.

**Hecho**
- **La Maqueta ya no se queda en blanco**: entrar en ella con Estructuras o Instalaciones activas, o coger dentro una herramienta de otra disciplina (pilar, tubería, terreno, volumen), ya no apaga el resto del edificio. La Maqueta enseña siempre todo en color; al volver a Técnica, la atenuación por disciplina sigue como estaba.
- **Líneas de apertura**: en las puertas (abatible, pivotante, plegable) salen **con guiones** por defecto en alzados y secciones, con su propio parámetro «Líneas de apertura (alzado y sección)»; el giro en planta sigue aparte («Línea del giro (planta)»). En las **ventanas** (practicable, corredera, guillotina) **no salen** en los alzados y secciones del modelo salvo que el tipo lo pida («Líneas de apertura en alzados y secciones del modelo»), y cuando salen van con guiones. En el plano y la memoria de carpinterías y en el diálogo del tipo salen siempre, porque ahí explican el funcionamiento.
- **Mover un muro unido mueve solo ese muro**, como en Revit: los muros que llegan a él (en esquina o en T) no se desplazan, solo se alargan o acortan sobre su propio eje para seguir tocándolo. Antes, en una esquina el vecino se torcía o se desplazaba con él. Arreglado también que un tabique dibujado hasta el eje del muro (dentro de su grueso) no lo siguiera.
- **Carpetas al elegir tipo** en Propiedades: muros en «Muros exteriores (fachada)» y «Muros interiores»; forjados por función; puertas y ventanas por familia, como en el diálogo de tipos.

**Pendiente**
- De E1, nada. Siguiente según el orden propuesto: E2 (galerías visuales en todas las opciones), si Antonio lo confirma.
- Decisiones D25–D28 (armarios empotrados, hormigón armado, fachadas al detalle, giro de las ventanas en planta).

**Qué debe probar Antonio**
1. Descarga ArchiSim-Windows del pull request de esta rama (GitHub → Actions) y abre la demo o un proyecto tuyo. En proyectos tuyos: **Biblioteca → Actualizar familias**, para que puertas y ventanas tomen las líneas nuevas.
2. Maqueta: en Técnica elige la disciplina Estructuras, pasa a la Maqueta (Tab): todo debe verse en color. Dentro, coge Terreno o un Volumen del catálogo: sigue en color.
3. Haz una sección por una puerta y una ventana: la puerta con el triángulo a guiones; la ventana sin triángulo. Selecciona la ventana → Editar tipo → Representación → activa «Líneas de apertura en alzados y secciones del modelo»: aparece a guiones. Archivo → Plano de carpinterías: las ventanas siguen con su triángulo.
4. Dibuja un rectángulo de muros y un tabique hasta el eje de uno de ellos. Selecciona el muro de arriba y muévelo en diagonal: él se mueve entero, los laterales solo se alargan (no se tuercen) y el tabique lo sigue.
5. Selecciona un muro → Propiedades → pincha en el nombre del tipo: la galería debe salir en dos carpetas, exteriores e interiores. Igual con una puerta (por familias).

---

## 2026-10-26 (3) · v0.70: árboles de AD5 en planta (T3)

Pedido por Antonio el 26/10/2026, con el DXF «Árboles AD5 Planta» (cuatro árboles dibujados en el estudio): tenerlos en planta como una visualización más bonita. Directamente en `main`: es pequeño y no cambia el formato `.archisim` ni el IFC. La familia Vegetación pasa a la versión 3.

**Hecho**
- **Cuatro estilos de planta nuevos** en la familia Vegetación, para cualquier especie: **AD5 círculos** (tres copas superpuestas en manchas transparentes con anillos discontinuos), **AD5 racimos** (racimos de hojas sobre una mancha clara, con matas de hierba dentro), **AD5 estrella** (rayos del tronco al borde) y **AD5 copa** (el círculo de la copa con una masa irregular de hojas). Se eligen como «Estilo de presentación» o «Estilo técnico» del tipo, o como «Estilo» de un ejemplar.
- Cada árbol lleva el dibujo **a la medida de su copa**, estirado lo justo para seguir su contorno, y **girado distinto en cada ejemplar**: una alineación no se ve repetida.
- Las manchas llevan el **color de hojas de la especie** con la **opacidad que tenían en AD5** (con la opacidad por defecto); en «líneas» quedan solo las líneas, en «rellena» las manchas opacas, en «solo color» sin líneas. El tronco es el de cada planta. Las flores de las especies que florecen salen encima.
- En alzado y sección, que no son dibujos de lado, se dibujan con el estilo de ArchiSim más parecido (nube, follaje o contorno).
- El DXF se convierte con `scripts/make_ad5_trees.py` a `crates/archisim-flora/src/ad5_trees.txt`; el DXF no se sube al repositorio. La trama de hierba del DXF no se copia: ArchiSim dibuja sus propias matas.
- `archisim vivero out/vivero` dibuja todo el vivero en cada estilo AD5: `vivero-ad5-circulos-planta.svg`, `…-racimos-…`, `…-estrella-…`, `…-copa-…`.

**Pendiente**
- Lo que diga Antonio. Ideas: líneas en el verde de AD5 en lugar de con la plumilla de la vista (hoy van en tinta, como el resto del dibujo); un dibujo de lado propio de AD5 para los alzados.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-cli -- vivero out/vivero` y abre en el navegador `out/vivero/vivero-ad5-circulos-planta.svg` y los otros tres: compáralos con tus bloques de AutoCAD.
2. En un proyecto tuyo con árboles: **Biblioteca → Actualizar familias** (para que la familia Vegetación pase a la versión 3). Selecciona un árbol → Propiedades → «Estilo de presentación» → «AD5 copa» (y los otros tres). Cambia también «Color en presentación» a líneas, rellena y solo color.
3. Haz una plantación en línea (Urbanismo → Plantación) con «AD5 círculos»: los árboles deben verse girados unos respecto a otros.
4. Mira un alzado con esos árboles: deben salir dibujados de lado, no con el dibujo de planta.

---

## 2026-10-26 (2) · Todo unido en `main` (v0.69)

Antonio dio el visto bueno el 26/10/2026: unidas en `main` todas las ramas pendientes, en orden: F1 (#66), C2 (#67), A1 (#68), C3 (#69), C4 (#70), C5 (#71), C6 (#72), C7 (#73), C8 (#75), el taller de modelos M1 (#74) y los árboles en línea T2 (#65). T2 se había numerado también v0.61; entra sin cambiar el número de versión, que queda en 0.69. Los conflictos eran solo de documentación (bitácora, hoja de ruta, CLAUDE.md) y de número de versión.

**Qué debe probar Antonio**
1. Descarga la aplicación de `main` (o `cargo run --release -p archisim-app`) y abre la demo: todo lo de C1–C8 debe estar, además del sofá del taller (`archisim taller out/taller`) y los cuatro estilos de árboles en línea (`archisim vivero out/vivero`).

---

## 2026-10-26 · v0.69: pincel único, cubiertas del suelo, rocas (C8)

Rama `bloque/c8-pincel`, encima de `bloque/c7-deudas` (#73), con pull request para Antonio. **Formato `.archisim` versión 39** (el aspecto de un material puede llevar hierba y flores; abre todas las anteriores). Pedido por Antonio el 26/10/2026 tras probar C5, con una captura de Paralives.

**Hecho**
- **Un solo Pincel, en Técnica y en Maqueta**: en Técnica está en la ficha Urbanismo (tecla `PI`); en la Maqueta, en Exterior → Terreno y caminos. Ya no hay un pincel por cada terreno.
- **Modos** (botones arriba del panel y en la barra contextual): Pintar, Borrar, Subir, Bajar, Suavizar, Allanar y Esparcir. Borrar quita lo pintado (vuelve a verse el terreno) y, si se quiere, las plantas y rocas que haya debajo.
- **Cabezales**: redondo, cuadrado, difuminado (más pequeño en el borde, para trazos suaves) y salpicado (manchas sueltas). Tamaño y fuerza con deslizadores; el contorno del pincel se ve en planta y en 3D.
- **Paleta de cubiertas** con muestras dibujadas: césped, césped natural, hierba alta, pradera seca, pradera con flores amarillas, flores silvestres, margaritas, amapolas; tierra, tierra oscura, camino de tierra, arena, grava, losas de piedra, hojarasca y corteza de pino. Cada una es un material del proyecto (se crea la primera vez que se usa).
- **Hierba y flores de verdad**: las cubiertas de césped y pradera levantan en 3D matas de hierba con su altura y flores de colores; en planta, trazos de hierba y puntos de flores. En el render y en el `.glb` también. Al pasar de unas 16 000 matas se aclaran para que la vista siga fluida.
- **Rocas y elementos naturales** (categoría nueva, en Urbanismo → Rocas y en la Maqueta → Exterior → Rocas y troncos): roca angulosa, canto rodado, laja, peña de 2 m, roca con musgo, grupos de rocas, de cantos y con musgo, cantos rodados sueltos, tronco caído y tocón. Varias variantes de forma por roca. IFC `IfcGeographicElement`.
- **Esparcir** coloca plantas o rocas sobre el terreno, giradas al azar y sin pisarse; con «Variar» alterna las variantes de la misma roca.

**Pendiente**
- Lo que diga Antonio del pincel. Ideas que no entraron: pintar sobre forjados y cubiertas (ahora solo terreno), más especies de flor, rocas con textura de estratos.

**Qué debe probar Antonio**
1. Abre la demo, ficha Urbanismo → Pincel. En Propiedades: elige «Flores silvestres», cabezal cuadrado, y pinta sobre el terreno en planta. Mira el resultado en la Maqueta.
2. Cambia a Borrar y pasa por encima: debe volver el terreno de antes.
3. Prueba los cabezales difuminado y salpicado con «Grava» y «Camino de tierra».
4. Modo Esparcir → elige «Roca angulosa 80» con «Variar» encendido, y pinta: deben salir rocas de varias formas. Bórralas con Borrar y «Plantas y rocas» activado.
5. En la Maqueta: Exterior → Terreno y caminos → Pincel, y el panel flotante; Exterior → Rocas y troncos.
6. Subir, Bajar, Suavizar y Allanar sobre el terreno, y Deshacer.
7. Exporta el IFC (rocas y troncos como elementos geográficos) y el `.glb` (Archivo → Exportar modelo 3D) y ábrelo en Blender o D5: la hierba va dentro.

---

## 2026-10-25 (10) · M1: taller de modelos de alta calidad, primera entrega (el sofá)

Rama `bloque/modelos`, encima de `bloque/c7-deudas` (#73), con pull request para Antonio. Sin cambios de formato ni de IFC, y sin versión nueva: es el taller, no entra nada en la biblioteca.

Lo que pidió Antonio: «necesito crear mejores modelos de los que tenemos, pero ir poco a poco: primero me enseñas lo que hagas y después, cuando tengamos un paquete, los incluimos en la biblioteca. Tienen que ser modelos de alta calidad, iguales o mejores que Paralives».

**Hecho**
- **Tela rellena** como pieza nueva para los modelos: caras que se abomban, asiento que se hunde donde uno se sienta, pliegues que tiran de las esquinas, pequeñas irregularidades y el vivo de las costuras en relieve. Cojines sueltos cosidos por el canto, con las esquinas en punta. Patas torneadas que se abren un poco.
- **Primer modelo: el sofá** (familia «Sofá (taller)»), con sus medidas, plazas y de 0 a 4 cojines sueltos. Dos tipos de prueba: 3 plazas en lino arena y 2 plazas en terciopelo salvia.
- **La tela se dibuja suave en el 3D**: sin las rayas de puntos que salían en los pliegues de los cojines, solo con su contorno. Solo afecta a los modelos del taller; los de la biblioteca se ven igual que antes.
- Pesa lo mismo que el sofá de la biblioteca (18.500 triángulos frente a 19.100). Una prueba vigila que ninguna pieza del taller pese más de vez y media la que sustituye.
- `archisim taller out/taller`: cada pieza nueva delante de la que sustituiría, con un tabique detrás (proyecto, IFC validado, axonometría y planta). Fotos de estudio con el dibujo 3D de la Maqueta: `ARCHISIM_TALLER_OUT=out/taller cargo test -p archisim-app workshop_pictures`.
- Descrito en `docs/BIBLIOTECA_3D.md`, «Taller de modelos de alta calidad».

**Pendiente**
- Lo que diga Antonio del sofá: proporciones, lo hundido del asiento, el vivo, los cojines sueltos.
- Las piezas siguientes del paquete, en el orden que diga Antonio. Propuesta: butaca, cama con ropa de cama, silla y mesa de comedor, y después cocina y baño.
- Cuando el paquete esté aprobado: meterlo en la biblioteca en lugar de los modelos de ahora, en la Maqueta y en las colecciones.

**Qué debe probar Antonio**
1. Mira las fotos que te mando en el chat: «antes y después» y las vistas del sofá nuevo.
2. `cargo run --release -p archisim-cli -- taller out/taller` y abre `out/taller/taller.archisim`. Ve a la Maqueta (Tab) y gira alrededor de los sofás: delante, los nuevos; detrás, los de la biblioteca.
3. Selecciona un sofá nuevo y cambia en Propiedades el largo, las plazas y los cojines sueltos: debe rehacerse con sus cojines.
4. Abre `out/taller/taller.ifc` en ArchiCAD o Revit: los sofás nuevos deben verse con su forma, como mobiliario.

---

## 2026-10-25 (9) · v0.68: deudas de escaleras y buhardillas (C7)

Rama `bloque/c7-deudas`, encima de `bloque/c6-documentacion` (#72), con pull request para Antonio. **Formato `.archisim` versión 38** (las escaleras guardan si sus mesetas van partidas y la meseta de llegada de las de caracol; abre todas las anteriores). Con esto el orden C1–C7 queda hecho.

**Hecho**
- **Mesetas partidas a 45°**: en una L, la meseta del giro se parte por la diagonal en dos peldaños triangulares; en una U, en tres. La escalera queda de un solo tramo y con las tabicas justas (la forma hecha quita los peldaños que sobran de los tramos). En la herramienta: Giros → «Meseta partida a 45°»; en Propiedades: Forma → «Mesetas partidas a 45°»; en la Maqueta: Construir → Escaleras → «En L con meseta partida». Planta, 3D, IFC (tramo `WINDER`, escalera `QUARTER_WINDING_STAIR`/`HALF_WINDING_STAIR`) y vuelta del IFC.
- **CTE**: las mesetas partidas solo se admiten en uso restringido (DB-SUA 1, 4.2.1). En uso general salen como incumplimiento; en vivienda no se les piden los 5 y 44 cm de los peldaños curvos.
- **Meseta de llegada de las de caracol**: un sector a la altura de la planta de arriba que sigue el giro, 90° por defecto en las nuevas; en Propiedades, «Meseta de llegada (°)» (0 la quita). La barandilla sigue por ella.
- **Buhardillas donde se juntan dos limahoyas**: en una cubierta en T o en cruz, por encima del punto donde llegan las dos limahoyas el faldón principal son dos caras en el mismo plano. Antes una buhardilla sobre la línea entre ellas «no cabía»; ahora cuenta como un solo faldón y corta las dos caras, con las tejas alrededor. Esa línea tampoco se dibuja ya como limahoya.
- `archisim escaleras out/`: doce escaleras (L y U con mesetas partidas, caracol con meseta de llegada). `archisim cubiertas out/`: una casa en T con su buhardilla sobre el encuentro.

**Pendiente**
- Lo que diga Antonio de C1–C7. Del orden pactado no queda nada.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- escaleras out/` y abre `out/escaleras.archisim`: las dos últimas son la L y la U con mesetas partidas; la de caracol con mástil lleva la meseta de llegada. Míralas en planta baja y primera y en la Maqueta.
2. Herramienta Escalera → Forma «En L» → Giros «Meseta partida a 45°», y dibújala. Selecciónala y cambia en Propiedades la Forma entre «Tramos y mesetas», «Mesetas partidas» y «Peldaños compensados»; mira la comprobación del CTE con uso general y con uso restringido.
3. Una de caracol: cambia «Meseta de llegada (°)» a 120 y a 0.
4. `cargo run -p archisim-cli -- cubiertas out/` y abre `out/cubiertas.archisim`: la casa en T de abajo tiene una buhardilla justo encima de donde se juntan las limahoyas. Prueba a colocar otra en el mismo sitio en tu propia cubierta en T.
5. Exporta el IFC de los dos y ábrelos en ArchiCAD o Revit.

---

## 2026-10-25 (8) · v0.67: memoria de carpinterías y cuadro de superficies (C6)

Rama `bloque/c6-documentacion`, encima de `bloque/c5-exterior` (#71), con pull request para Antonio. Sin cambios de formato ni de IFC.

**Hecho**
- **Memoria de carpinterías** (Archivo → Memoria de carpinterías (PDF)…): hojas A3 con el mismo cajetín que el plano de carpinterías, ocho tipos por hoja. Cada tipo con su marca, su nombre, un alzado pequeño a 1:100 y su descripción con palabras: «Ventana de 2 hojas practicables, de 1,80 m de ancho por 1,30 m de alto (medidas del hueco), con antepecho a 0,90 m. Carpintería de aluminio lacado, con marco de 6 cm de cara vista y 7 cm de fondo; acristalamiento de vidrio doble.», la descripción que tenga el tipo, la familia, las unidades por planta y la situación: en qué locales está, y las puertas entre qué dos locales.
- **Cuadro de superficies** (Archivo → Cuadro de superficies (PDF)…): la superficie útil de cada local por plantas (número, nombre, uso y m², con la útil de la planta), el resumen por plantas con la útil y la construida, y la útil por usos. Las cifras son las mismas de la comprobación CTE. Si una planta tiene muchos locales, la lista sigue en la columna siguiente o en otra hoja.
- **Plano de carpinterías**: cada tipo dice ahora también el **antepecho** de las ventanas («0,90 m», «0,90 y 1,10 m» si hay varios, «sin antepecho» en las balconeras) y la **situación** (unidades por planta). «2 hojas practicables» concuerda en plural.

**Pendiente**
- Lo que Antonio eche en falta del plano y de la memoria (otras columnas, otro orden, el texto de la memoria a su manera).
- C7: mesetas partidas a 45°, meseta de llegada de las de caracol, buhardillas sobre limahoyas.

**Qué debe probar Antonio**
1. Abre la demo (`cargo run --release -p archisim-app -- out/demo.archisim`).
2. Archivo → **Memoria de carpinterías (PDF)…** y guárdala. Ábrela: comprueba que la descripción de cada puerta y ventana es la que pondrías en una memoria y que la situación (los locales) es la correcta.
3. Archivo → **Cuadro de superficies (PDF)…**: compara los m² con los de la tabla de Superficies y con la comprobación CTE.
4. Archivo → **Plano de carpinterías (PDF)…**: mira las filas nuevas de Antepecho y Situación.

---

## 2026-10-25 (7) · v0.66: Exterior y Concepto en la Maqueta (C5)

Rama `bloque/c5-exterior`, encima de `bloque/c4-visualizacion` (#70), con pull request para Antonio. **Formato `.archisim` versión 37**: abre todas las anteriores. Guarda las zonas pintadas sobre el terreno y los varios dibujos de planta de un tipo. El IFC no cambia: el terreno sale igual y las zonas pintadas no van en él.

**Hecho**
- **Pinceles de Exterior** (Maqueta → Exterior → Terreno y caminos; o la herramienta Pincel, con sus maneras en la barra de arriba):
  - **Subir, Bajar, Alisar y Aplanar** el terreno arrastrando, más en el centro del pincel que en el borde. «Aplanar» lo deja a la altura del punto donde empiezas.
  - **Césped, Grava y Camino** pintan zonas sobre el terreno. Si pintas una encima de otra, la de debajo se recorta. En planta salen con su color; en 3D, como una capa fina con su textura.
  - **Esparcir plantas** de la especie que elijas en Propiedades, separadas según su tamaño y apoyadas en el terreno.
  - Si la planta no tiene terreno, el primer trazo crea uno llano de 40 × 40 m.
  - Cada trazo, sea largo o corto, se deshace de una vez.
  - Radio y fuerza (o densidad, en las plantas) en la barra.
- **Flechas y aros en los volúmenes de Concepto** (en 3D, con un volumen seleccionado):
  - flechas en los cuatro lados para empujar ese lado (con Alt, por los dos);
  - una flecha arriba para la altura (por plantas enteras si el volumen va por plantas);
  - un cuadrado en una esquina para escalarlo entero;
  - el aro naranja para girarlo, de 15 en 15° o libre con Mayús.
  - Mientras arrastras se ve como quedará.
- **Árboles** (remates de A1):
  - al elegir dibujos de un DXF con muchos, el interruptor **«Varios dibujos, que se alternan»** deja elegir varios; cada árbol toma uno, siempre el mismo, y una fila de olivos ya no es el mismo dibujo repetido;
  - el **dibujo en alzado** también se elige de esos DXF, viéndolo: Propiedades del objeto → Dibujo en alzado → «De mi biblioteca, de frente…» o «de lado…», o al cargar un DXF con varios bloques.

**Pendiente**
- Las zonas pintadas no van al IFC.
- Esparcir plantas no cuenta las plantas de una Plantación que ya estén allí.
- Los caminos se pintan a mano alzada. Para uno con su sección y bordillos sigue estando Camino o calle.

**Qué debe probar Antonio**
1. Proyecto nuevo, Tab a la Maqueta, Exterior → Terreno y caminos → **Subir**. Arrastra sobre el suelo varias veces: sale una loma. Prueba **Alisar** y **Aplanar** encima, y Ctrl+Z: cada trazo se deshace entero.
2. **Césped** y **Camino**: pinta una pradera y atraviésala con un camino. Mira la planta (Técnica): las dos zonas con su color.
3. **Esparcir plantas**: en Propiedades elige la especie (por ejemplo Olivo) y arrastra. Cambia la densidad en la barra.
4. Exterior → Formas → **Volumen**, dibuja uno y selecciónalo en 3D: tira de las flechas de los lados y de arriba, del cuadrado de la esquina y del aro.
5. Con un tipo de árbol, «Símbolo de planta» → «De mi biblioteca…», enciende **«Varios dibujos, que se alternan»**, elige tres y «Usar». Pon una fila de árboles: verás los tres dibujos repartidos.
6. Abre un proyecto guardado con una versión anterior: debe abrirse igual.

---

## 2026-10-25 (6) · v0.65: salida para visualización (C4)

Rama `bloque/c4-visualizacion`, encima de `bloque/c3-maqueta` (#69), con pull request para Antonio. Sin cambios de formato ni de IFC.

**Hecho**
- **El proyecto para D5, Twinmotion o Blender**: Archivo → **Exportar modelo 3D (glTF)…** guarda todo el proyecto en un `.glb`:
  - un objeto por elemento, con su nombre («Muro 12», «Sofá de 3 plazas 40»);
  - los materiales con su nombre y su color; los que tienen textura (ladrillo, madera, piedra, telas…) la llevan como imagen, puesta igual que en la vista 3D, para cambiarla en el programa de render si se quiere; el vidrio, transparente;
  - a escala y de pie. También `archisim glb proyecto.archisim`, y la demo y `archisim exportar` lo dejan junto al IFC.
- **Render con texturas, atardecer y noche**: Renderizar usa la luz de la vista 3D (Día, Atardecer o Noche) y sus texturas.
  - De noche se encienden las lámparas, las velas, la chimenea y las farolas; brillan y alumbran lo que tienen cerca, **con sus sombras**.
  - Al atardecer, sol bajo y anaranjado, cielo cálido y lámparas a media luz.
  - En la línea de órdenes, `--atardecer`, `--noche` y `--sin-texturas`.
- **Texturas «En imagen»** en la vista 3D (Propiedades de la vista 3D → Texturas):
  - son las mismas texturas, sacadas de imágenes que se hacen una vez en lugar de calcularse en cada punto;
  - es lo que usan ahora **por defecto los Mac con Intel**, en vez de las ligeras;
  - las mismas imágenes van en el render y en el `.glb`.
- **Alzados y planos sin espera al soltar** (remate de F1): al acabar de mover o acercar un alzado, una sección o un plano, el dibujo nuevo se hace aparte y, mientras tanto, se sigue viendo el anterior en su sitio.

**Pendiente**
- Con OpenGL (tarjetas antiguas o Linux sin Vulkan), «En imagen» no se ve bien: allí se usan las ligeras.
- El `.glb` de la demo pesa unos 44 MB, casi todo de los árboles. Si D5 o Twinmotion van lentos, se puede exportar la vegetación más ligera.
- Las sombras de las lámparas están en el render. En la vista 3D en tiempo real, las lámparas alumbran sin sombras.

**Qué debe probar Antonio**
1. Abre la demo, Archivo → **Exportar modelo 3D (glTF)…**, y ábrelo en D5 o en Twinmotion (o en Blender: Archivo → Importar → glTF). Mira que entre a escala y de pie, que los materiales tengan sus nombres y que el ladrillo y la madera lleven su textura.
2. En la Maqueta, pon **Noche** (arriba, «Luz») y en Propiedades de la vista 3D → **Renderizar**: las lámparas encendidas con sus sombras. Repite con Atardecer.
3. En tu MacBook: Propiedades de la vista 3D → Texturas debe decir **En imagen**. Mira que el ladrillo y la madera se vean como en el PC y que el 3D no vaya más lento.
4. Abre un alzado y desplázalo con el botón derecho, o acércalo con la rueda: al soltar, la imagen no se congela; el dibujo fino aparece un momento después.
5. `cargo run --release -p archisim-cli -- render out/demo.archisim out/noche.png 1600 --noche`.

---

## 2026-10-25 (5) · v0.64: detalles de la Maqueta y de la interfaz (C3)

Rama `bloque/c3-maqueta`, encima de `bloque/arboles-dxf` (#68), con pull request para Antonio. Sin cambios de formato ni de IFC (las encimeras con copete salen en el `IfcFurniture` «Encimera» de siempre).

Antonio pidió seguir la hoja de ruta en orden (C3 → C4 → C5 → C6 → C7) con los remates de A1, C2 y F1. Los de la cocina van aquí.

**Hecho**
- **Habitación en un solo Deshacer**: sus cuatro muros, su suelo y su local se deshacen y rehacen de una vez («Habitación»).
- **Mover en 3D junto a las paredes**: un mueble (o lo que esté seleccionado) arrastrado en 3D se pega a la cara del muro cuando se acerca a menos de 15 cm, y después a la del muro que la cruza, en la esquina. Mientras se mueve, unas cotas azules como las temporales de la planta miden lo que queda libre hasta los muros en sus cuatro lados. Los muros, puertas y ventanas no se pegan: siguen sus reglas.
- **Puertas y ventanas por el centro, desde un extremo o repartidas**, en la barra de la herramienta:
  - **Libre**: como siempre, donde está el cursor;
  - **Por el centro**: en el centro del tramo libre del muro, entre los muros que lo encuentran y las otras carpinterías, medido en la cara hacia la que abre;
  - **Desde un extremo**: la jamba a la distancia que se escribe (30 cm de partida) del extremo más cercano al cursor;
  - **Repartidas**: las que se digan (2 de partida), con machones iguales entre ellas y en los extremos, todas en un paso. Si no caben, lo dice: «no caben 3 de 1,80 en el tramo libre de 4,20».
- **Un mismo dibujo para los sí o no**: las casillas de todos los paneles y diálogos son ahora interruptores, como «Rejilla» o «Calco» en Propiedades.
- **Cocina** (remates de C2):
  - los módulos bajos tienen **Vuelo delante** y **Vuelo detrás** (sección Encimera): la encimera sale más que el mueble, para una barra con taburetes;
  - y **Copete**: una tira de 2 cm de grueso en el borde de atrás. En un tramo solo va donde no hay otro módulo detrás, así que en una isla espalda con espalda no sale. Se ve en 3D, va en el IFC con la encimera y se mide aparte en Mediciones («Copete de …», en metros).
  - **Dos sofás en L se convierten en una rinconera**: un sofá recto con la espalda contra el extremo de otro y el lado contra su frente, del mismo fondo y altura, se une con él en un «Sofá rinconera» del largo del primero y con el brazo del segundo, a la derecha o a la izquierda.
- `archisim cocina` pone copete a la L y una barra de 30 cm a un lado de la isla.

**Pendiente**
- Mientras se arrastra un mueble en 3D, sus flechas y el punto de giro se quedan donde estaba hasta soltarlo.
- La encimera de un tramo se sigue dibujando en planta módulo a módulo.

**Qué debe probar Antonio**
1. Maqueta → Construir → Habitación, dos clics. Ctrl+Z una vez: deben irse los cuatro muros, el suelo y el local. Ctrl+Y: vuelven.
2. En la Maqueta, coge un sofá y llévalo hacia una pared: se pega a ella y, cerca de la esquina, también a la otra. Mira las cotas azules mientras lo mueves.
3. Herramienta Ventana. En la barra de arriba, elige **Por el centro** y pasa por la fachada de un dormitorio: la ventana se centra entre los tabiques. Prueba **Desde un extremo** con 0,30 y **Repartidas** con 3 en una fachada larga. Ctrl+Z quita las tres a la vez.
4. Mira los paneles: Propiedades de la vista (Rejilla, Calco), Capas, Coordinación. Todo lo que era una casilla es ahora un interruptor.
5. `cargo run --release -p archisim-cli -- cocina out/cocina` y abre `out/cocina/cocina.archisim`: la L con copete y la isla con barra. Mira en 3D la barra y el copete, y en Mediciones el capítulo 10.
6. Coloca dos «Sofá de 3 plazas» en L (el segundo girado, con la espalda contra el extremo del primero): al soltarlo se hacen una rinconera.

---

## 2026-10-25 (2) · T2: árboles en línea (preparada como v0.61; entra en `main` con la v0.69)

Rama `bloque/arboles-linea`, encima de `bloque/tramas` (#64), con pull request. Sin cambios de formato; la familia Vegetación pasa a la versión 2.

Antonio pidió árboles en planta y alzado «en ese estilo», el de unas bibliotecas de CAD que no se pueden redistribuir, pero sin copiarlos. Miré sus dibujos solo para sacar los rasgos generales del género, y los estilos nuevos salen del generador de plantas de ArchiSim: cada árbol se dibuja desde sus propias ramas y matas de hojas, así que no hay ninguna forma calcada.

**Hecho**
- Cuatro estilos nuevos para el dibujo de la vegetación, en planta y en alzado:
  - **Corona**: rosetas de hojas alrededor del borde, con el centro abierto.
  - **Silueta**: la copa maciza en el color de la línea, con su borde de hojas y las ramas en blanco a través; en alzado, el tronco también macizo.
  - **Ramitos**: ramas con foliolos a pares, como plumas.
  - **Ramas y borde**: la estructura de ramas dentro de un contorno dentado.
- Se eligen en Propiedades, como los demás estilos (técnico, de presentación o el de cada ejemplar).
- `archisim vivero out/` dibuja el vivero entero en cada estilo, en planta (1:200) y en alzado (1:100).

**Pendiente**
- Los proyectos que ya tenían árboles los ven después de Biblioteca → Actualizar familias.
- Ajustes según lo que diga Antonio: grosor de las rosetas de la corona, densidad de los ramitos.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-cli -- vivero out/vivero` y abre `out/vivero/vivero-silueta-planta.svg`, `vivero-corona-planta.svg`, `vivero-ramitos-planta.svg` y `vivero-ramas-y-borde-planta.svg`, y los `…-alzado-1.svg` de cada uno.
2. En un proyecto, elige un árbol y, en Propiedades → Dibujo en planta, pon el estilo de presentación en «silueta» y el color en «líneas». Mira la planta y un alzado.
3. En un proyecto antiguo: Biblioteca → Actualizar familias, y después lo mismo.

---

## 2026-10-25 · v0.60: tramas de presentación (T1)

Rama `bloque/tramas`, encima de `bloque/figuras-2` (#63), con pull request para Antonio. **Formato `.archisim` versión 36**: abre todas las anteriores. Solo sube para que una versión anterior del programa avise de que el fichero es más nuevo.

Lo que pidió Antonio: «patrones de relleno para dibujo arquitectónico con un grosor de línea fina, solo en gris, en un formato estilizado para presentaciones limpias con un toque humano».

**Hecho**
- 28 tramas nuevas, dibujadas de cero para ArchiSim:
  - **Madera**: en tablas, vetas, terminada, pino, roble, arce, abedul, fresno, nogal y caoba.
  - **Metal**: cepillado, perforado, chapa lagrimada, chapa grecada y malla.
  - **Hormigón**: visto, en bruto, desnudo, adoquín en espiga y encofrado con tablas.
  - **Piedra**: grava gruesa, caliza, bloques, sillería, terrazo, camino de piedras, adoquín en abanico y mampostería.
- Van a tamaño real y pieza a pieza: cada tabla, piedra o agujero es distinto. Las juntas van al tresbolillo; las vetas siguen la ondulación de su tabla sin cruzarse; hay catedrales, nudos y radios del roble.
- El trazo es a mano: las líneas ondulan un poco y a veces no llegan a la esquina.
- Si se cambia el contorno, lo que queda dentro no se mueve.
- En un área muy grande se dibujan con líneas sencillas en lugar de con un relleno macizo.
- En el selector de tramas salen por familias: Madera, Metal, Hormigón y Piedra.
- En DXF, las tramas dibujadas pieza a pieza (también las de P1: piedra irregular, veta, grava y césped punteados) van como líneas, no como un patrón aproximado.
- `archisim tramas out/` hace una lámina con todas a 1:20 y a 1:50 (SVG, PDF, DXF y proyecto).

**Pendiente**
- Ajustes de densidad y de carácter de cada trama según lo que diga Antonio al verlas impresas.
- Los DXF de árboles y arbustos que mandó Antonio el 25/10/2026 son de CAD素材.com, una biblioteca de otros. No entran en el repositorio. Propuesta: una orden para cargar los bloques de un DXF en su biblioteca local.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-cli -- tramas out/tramas` y abre `out/tramas/tramas.pdf`: hoja 1 a 1:20, hoja 2 a 1:50. Mira si alguna trama está demasiado cargada o vacía.
2. Abre `out/tramas/tramas.archisim`, elige un cuadro y cambia su patrón en Propiedades → Sombreado → Patrón. Las tramas nuevas están abajo, por familias.
3. En un proyecto, elige un local y, en **Pavimento en planta**, pon Roble o Terrazo. Gira la trama y cambia su escala.
4. Exporta a DXF un plano con una de estas tramas y ábrelo en AutoCAD.

---

## 2026-10-24 (7) · v0.59: Tab entre elementos, cambios por ejemplar, galería en 3D y dibujos propios en alzado

Rama `bloque/figuras-2`, sobre C1 (lleva dentro #62), pull request para Antonio. **Formato `.archisim` versión 35**: abre todas las anteriores.

Lo que pidió Antonio al ver C1.

**Hecho**
- **Tab en planta**, como en Revit: con la flecha sobre varios elementos que se pisan (un eje sobre un muro, un muro sobre un forjado), Tab pasa al siguiente, Mayús+Tab al anterior, y el clic elige el resaltado. Abajo aparece cuál es («Tab: Muro (2 de 3)»). Fuera de ahí, Tab sigue cambiando entre Técnica y Maqueta.
- **Lo que se cambia en la Maqueta es del ejemplar**:
  - medidas, colores y opciones de una pieza cambian solo esa pieza, que recibe su tipo propio «(ejemplar)»;
  - el tipo de la biblioteca no cambia;
  - «Aplicar a todos», en la barra del objeto, lo lleva al tipo y a todas sus piezas;
  - al borrar la pieza se va su tipo propio.
- **Galería de objetos de Técnica en 3D**, como el catálogo de la Maqueta (la tarjeta del objeto elegido también).
- **Dibujos propios en alzado** (Propiedades del objeto → Dibujo en alzado):
  - DXF de frente o de lado para las vistas técnicas, y PNG con fondo transparente para las de presentación;
  - se dibujan de pie en alzados, secciones y axonometrías, tan altos como el objeto, mirando a quien mira y en su orden de profundidad.
- **Imagen PNG en planta de presentación**, en lugar del símbolo, del tamaño del objeto y girada con él.
- Ejemplos dibujados para ArchiSim en `docs/ejemplos` (árbol en DXF, persona en PNG, árbol en planta en PNG), que usa `archisim figuras`.

**Pendiente**
- Varios dibujos por especie, para que una fila de árboles no se repita (hoy, uno de frente y uno de lado por tipo).
- Las imágenes de planta de las plantas con tamaño propio usan el tamaño de su tipo.
- C2 sigue.

**Qué debe probar Antonio**
1. Abre un proyecto con un eje sobre un muro. Con la flecha, pon el cursor encima y pulsa Tab: se resalta el muro. Haz clic y queda elegido. Pulsa Mayús+Tab para volver al eje.
2. En la Maqueta, coloca dos sofás iguales del catálogo. Cambia el color o el ancho de uno: el otro no cambia, y uno nuevo del catálogo sale como el original. Pulsa «Aplicar a todos» en la barra: cambian los dos.
3. En Técnica, herramienta Objeto: la galería y la tarjeta del objeto se ven en 3D.
4. Selecciona un árbol. En Propiedades → Dibujo en alzado, carga `docs/ejemplos/arbol-alzado.dxf` (DXF de frente) y mira un alzado y la vista 3D de un plano. Prueba con un DXF vuestro.
5. Carga `docs/ejemplos/persona-alzado.png` en una persona (PNG de frente) y mira un alzado con estilo de presentación (por ejemplo, la sección del plano A-03).
6. En Propiedades → Símbolo de planta → Imagen en planta de presentación, carga `docs/ejemplos/arbol-planta.png` en un árbol y mira la planta de presentación.
7. `cargo run --release -p archisim-cli -- figuras out/figuras`: al final de la fila están el olivo dibujado (DXF y PNG en planta) y la persona en imagen.

---

## 2026-10-24 (6) · v0.58: C1, árboles, personas y coches en alzado, sección y axonometría

Rama `bloque/c1-figuras`, pull request para Antonio. Sin cambios de formato ni de IFC: la familia «Persona» pasa a la versión 3, con más materiales, y los proyectos la cogen de la biblioteca al actualizar familias.

Antonio confirmó el orden C1–C7 de la hoja de ruta común (§9). Este es C1.

**Hecho**
- **Plantas en alzados y secciones** como las dibuja un arquitecto, no con la red de triángulos de su 3D:
  - la misma planta vista de lado: copa, tronco desde el suelo, frondas;
  - en técnico, el estilo técnico de la especie en líneas y en blanco, de modo que tapa lo de detrás;
  - en las vistas y planos con estilo de presentación, su estilo y sus colores (nube, follaje, acuarela, ramas, flores, frutos);
  - las palmeras por sus frondas, de la más lejana a la más cercana;
  - cada planta en su sitio del orden de profundidad: el edificio la tapa o ella tapa al edificio.
- **Axonometrías**: cada árbol de pie mirando a quien mira, con su estilo de presentación, sin facetas.
- **Personas y coches**:
  - rehechos en el estilo de la biblioteca acogedora: figuras redondeadas con ropa, piel y pelo de colores, y coches de formas suaves con ruedas gruesas;
  - en alzados y secciones, por su **silueta**: blanca con su contorno en técnico, y cada pieza en su color en presentación.
- Los cuerpos lisos (un árbol de la biblioteca que no es generado, un mueble redondo) dibujan ahora su contorno en alzado, que antes salía en blanco.
- Los alzados pesan mucho menos: el sur de la demo pasa de 38 MB a 24 MB, y el de `figuras` es de 0,6 MB.
- `archisim figuras out/`: árboles, palmeras, arbustos, personas, coches y una bicicleta en fila delante de un muro, en alzado 1:50 técnico y en color y en axonometría. `archisim vivero out/` escribe también un alzado de cada fila del vivero.

**Pendiente**
- En las axonometrías, las personas y los coches siguen con su volumen 3D, que es como se ven bien en 3D.
- C2 (cocina y muebles, segunda vuelta) es lo siguiente.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-cli -- figuras out/figuras` y abre en el navegador `out/figuras/figuras-alzado-tecnico.svg`, `figuras-alzado-color.svg` y `figuras-3d.svg`. Mira si los árboles, las personas y los coches están dibujados como los quieres en un alzado.
2. Abre la demo, ve a la pestaña «Alzado Sur» (Navegador → Proyecto → Secciones y alzados) y mira los árboles y el coche. Cambia en Propiedades el estilo de una planta (técnico y de presentación) y míralo en el alzado.
3. Abre el plano A-03 Presentación: la sección y la vista 3D deben llevar los árboles en color.
4. `cargo run --release -p archisim-cli -- vivero out/vivero`: un alzado por cada grupo de especies (`vivero-alzado-*-color.svg` y `-tecnico.svg`).

---

## 2026-10-25 (4) · v0.63: árboles en planta elegidos de un DXF, viéndolos (A1)

Rama `bloque/arboles-dxf`, encima de `bloque/c2-cocina` (#67), con pull request para Antonio. Sin cambios de formato ni de IFC: el símbolo de planta del tipo ya se guardaba desde el bloque 6.

Antonio mandó un DXF con 76 árboles en planta, un bloque cada uno, y pidió usarlos como dibujo en planta viendo una previsualización al elegir. Los bloques se llaman «樹木平面（CAD素材.com）…»: son de la biblioteca de CAD素材.com, de terceros. Antonio puede usarlos en sus proyectos y en su biblioteca, pero el fichero no entra en el repositorio, que es GPL y de sala limpia.

**Hecho**
- **Elegir entre los dibujos de un DXF que es una biblioteca**:
  - en Propiedades → Símbolo de planta → «Importar DXF…», si el fichero tiene dos o más bloques colocados, se abre una ventana con todos ellos;
  - cada dibujo se ve como lo dibujará la planta en ese tipo, a su tamaño y en líneas finas;
  - van numerados como se leen en la lámina, por filas desde arriba a la izquierda; con el cursor encima, el nombre del bloque;
  - un clic se lo pone al tipo, en un paso de Deshacer.
  - Un DXF con un solo dibujo sigue entrando entero, como antes.
- **Tu biblioteca de símbolos**:
  - el DXF importado se copia a la carpeta «Símbolos de planta» de la biblioteca (Documentos/ArchiSim/Biblioteca, o la que tengas elegida);
  - el botón «De mi biblioteca…» abre la misma ventana con los DXF de esa carpeta, uno por pestaña, sin buscar el fichero.
- **Previsualización con su dibujo**:
  - la tarjeta del tipo en Propiedades y la galería de objetos enseñan su planta con el símbolo, en lugar del 3D, cuando el tipo tiene un dibujo propio;
  - las miniaturas de tipo llevan ya el símbolo y la imagen de planta (antes salía siempre el dibujo de la familia).
- Los árboles se estiran a la copa del tipo, como los demás símbolos de vegetación: el mismo dibujo vale para un olivo de 4 m y una encina de 9 m.
- Probado con el DXF de Antonio, desde fuera del repositorio: 76 dibujos, en el orden de la lámina, y la encina de la demo dibujada con el árbol 7.

**Pendiente**
- Varios dibujos por especie, para que una hilera no repita el mismo árbol.
- El mismo DXF para los dibujos de alzado (hoy el alzado toma el fichero entero).
- Un DXF de personas o coches funciona igual, pero los objetos que no son de vegetación usan el tamaño del dibujo y no el del objeto.

**Qué debe probar Antonio**
1. Abre la demo (`cargo run --release -p archisim-app -- out/demo.archisim`) y selecciona la encina, el árbol grande a la derecha de la casa.
2. En Propiedades, baja a «Símbolo de planta» y pulsa «Importar DXF…». Elige tu fichero de árboles: debe abrirse la ventana con los 76 dibujos.
3. Haz clic en uno: la encina debe dibujarse con él en planta, y la tarjeta de arriba debe enseñar su planta. Prueba Deshacer.
4. Selecciona otro árbol y pulsa «De mi biblioteca…»: deben salir los mismos 76 sin buscar el fichero.
5. Mira la planta de presentación (plano A-03) y a 1:100: el dibujo del DXF sustituye al de ArchiSim.

---

## 2026-10-25 (3) · v0.62: C2, cocina y muebles, segunda vuelta

Rama `bloque/c2-cocina`, encima de `bloque/fluidez` (#66), con pull request para Antonio. Sin cambios de formato: los tramos de encimera se calculan, no se guardan. El IFC cambia: lleva una encimera por tramo.

**Hecho**
- **Encimera continua de verdad**:
  - qué es un tramo: los módulos bajos que se tocan, a la misma altura y con la encimera del mismo material, en fila, en L, en U o en isla;
  - en 3D, secciones, alzados y axonometrías el tramo tiene una sola encimera, con el hueco de cada fregadero, sin juntas ni grietas, y los módulos van sin la suya;
  - separado del tramo, un módulo recupera su encimera.
- **Cocina en esquina**:
  - un módulo girado un cuarto de vuelta, acercado al extremo de una fila, se pega con su lateral contra el frente del último y su trasera a paño con el final de la fila;
  - también al revés, la fila hacia el tramo;
  - la encimera sale en L.
- **Isla por módulos**: los módulos espalda con espalda se pegan por la trasera, con los extremos alineados, y hacen una isla con una sola encimera.
- **Pegarse al arrastrar en planta**: lo que se pegaba al colocar o al arrastrar en 3D también se pega al arrastrarlo en planta (módulos, sofás, estanterías…). Los que se unen en uno, como sofás y estanterías, se unen al soltar.
- **Sofá rinconera** (familia nueva):
  - tramo a lo largo y tramo hacia delante, a la derecha o a la izquierda, con plazas en cada uno y la de la esquina, y los tres modelos del sofá;
  - en la biblioteca, 260×170 y 300×200;
  - en la barra del objeto: el lado, las plazas de cada tramo y sus materiales.
- **Color libre** en la barra del objeto:
  - las muestras de una pieza tienen al final «Color libre», con rueda, campos R, G y B y «Aplicar»;
  - crea un material como el de la pieza en ese color, conservando su textura, y se deshace en un paso;
  - el color de la **encimera** se aplica a todo su tramo, para que siga siendo una sola.
- **Mediciones**: capítulo «10 Mobiliario de cocina»:
  - encimeras por material, en metros a lo largo de los frentes, también la de un módulo suelto;
  - muebles de cocina por tipo, en unidades.
- **IFC**:
  - cada tramo es un `IfcFurniture` «Encimera», con su tipo por material, su material y el pset `ArchiSim_Worktop` (largo, superficie y módulos);
  - los módulos del tramo usan la geometría de su tipo sin encimera;
  - al importar, la encimera no se lee como objeto: vuelve a salir de los módulos (la cocina de ejemplo vuelve con sus 69 piezas).
- `archisim cocina out/` lleva además una cocina en L con fregadero y placa, una isla de seis módulos y un sofá rinconera.
- Revisado en la ventana real: la L y la isla en la Maqueta, el sofá rinconera con su barra y el color libre aplicado a unos frentes.

**Pendiente**
- El vuelo de la encimera de la isla (hoy, a paño con los módulos) y los remates de la encimera contra la pared (copete).
- Que un sofá recto acercado en esquina a otro se convierta en un rinconera.
- En planta, la encimera del tramo aún se ve como los rectángulos de cada módulo.
- C3 (detalles de la Maqueta y de la interfaz) es lo siguiente.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-cli -- cocina out/cocina` y abre `out/cocina/cocina.archisim`. En la Maqueta, al fondo: la cocina en L y la isla tienen una encimera de una pieza, con el hueco del fregadero.
2. Desde Amueblar → Cocina → Bajos, pon un módulo; gíralo con el anillo un cuarto de vuelta y acércalo al extremo de la fila de la L: debe pegarse en esquina. Pon dos espalda con espalda para empezar una isla.
3. En Técnica, en planta, selecciona un módulo de la isla y arrástralo un poco lejos y de vuelta: al acercarlo a otro, debe pegarse.
4. Selecciona un módulo del tramo y, en la barra del objeto, cambia «Encimera» a otro color: debe cambiar la encimera entera. Prueba «Frentes» → «Color libre»: elige un color con la rueda y pulsa «Aplicar».
5. Amueblar → Sofás y sillones → «Sofá rinconera 260×170». En su barra, cambia «Rinconera» a Izquierda y las plazas de cada tramo.
6. Pestaña Mediciones: capítulo «10 Mobiliario de cocina», con las encimeras en metros y los módulos por unidades.
7. Exporta el IFC de la cocina y ábrelo en ArchiCAD o Revit: cada tramo debe salir como un mueble «Encimera».

---

## 2026-10-25 (2) · v0.61: alzados que se abren y se mueven sin espera, y líneas finas en planta y 3D (F1)

Rama `bloque/fluidez`, pull request para Antonio. Sin cambios de formato ni de IFC.

Antonio dio por buenas la 0.58, la 0.59 y la 0.60: unidas a `main` (#62, #63, #64). Además pidió líneas finas también en planta y en 3D, y que el programa no tardase al abrir un alzado ni al desplazar con el botón derecho.

**Lo que pasaba**
- Al desplazar o acercar, la planta, los alzados y los planos se volvían a dibujar enteros a cada movimiento del ratón.
- El alzado sur de la demo tardaba 1,8 s en cada uno de esos dibujos. Era un dibujo de 24 MB:
  - 148.000 triángulos blancos, cada uno suelto;
  - los muebles de dentro de la casa, aunque los tapa la fachada.

**Hecho**
- **Desplazar y acercar sin espera** en planta, alzados, secciones y planos:
  - mientras se arrastra o se gira la rueda, se mueve la imagen ya dibujada;
  - al pararse (0,15 s sin moverse y el botón suelto), se dibuja una vez con todo su detalle;
  - lo que entra por los bordes sale en blanco hasta ese momento.
- **Alzados y secciones más ligeros**, con el mismo dibujo (comparado píxel a píxel con el de antes):
  - lo que queda entero detrás de algo opaco (el interior, la cara de atrás de las cosas) ya no se dibuja;
  - los triángulos blancos seguidos van en un solo trazo;
  - fuera, las astillas de menos de 1 mm², que son caras vistas de canto;
  - el dibujo de cada árbol visto de lado se guarda, en lugar de rehacerlo a cada redibujo.
- **Volver a un alzado ya abierto** es inmediato: se guardan los cuatro últimos mientras el modelo no cambie.
- Medido en la demo (`archisim medir vistas out/demo.archisim`):

  | | Antes | Ahora |
  |---|---|---|
  | Redibujo del alzado sur entero | 1,8 s | 0,3–0,4 s, solo al soltar |
  | Abrir el alzado sur | unos 2 s | unos 0,6 s |
  | Peso del alzado sur (SVG y PDF) | 24 MB | 5,6 MB |

- **Líneas finas**:
  - ficha «Líneas finas» en la barra de abajo de la planta y de la vista 3D, como ya estaba en alzados y secciones (el icono «Grosor real» de los forzados desaparece);
  - en 3D, las aristas pasan de dos píxeles a uno, también en la Maqueta;
  - la misma opción vale para todas las vistas, y está también en el menú Vista y en Propiedades de la planta.
- Orden nueva: `archisim medir vistas proyecto.archisim`, lo que tarda cada alzado, sección y planta en pantalla.

**Pendiente**
- El dibujo final, al soltar, sigue tardando unas décimas en los alzados grandes. Si molesta, el siguiente paso es dibujarlo en segundo plano.
- C2 (cocina y muebles, segunda vuelta) es lo siguiente, en `bloque/c2-cocina`.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-app -- out/demo.archisim`. En el Navegador → Proyecto, doble clic en «Alzado Sur»: debe abrirse en menos de un segundo.
2. En el alzado, mantén el botón derecho y arrastra: el dibujo debe seguir al ratón sin retraso. Al soltar, se completa lo que entra por los bordes. Lo mismo con la rueda para acercar.
3. Haz lo mismo en la planta y en un plano (A-03).
4. Abajo a la izquierda, en la planta, pulsa «Líneas finas»: todas las líneas a un píxel. Vuelve a pulsar: grosor real.
5. F3 (vista 3D): pulsa «Líneas finas» en la barra de abajo. Las aristas deben verse más finas, y la barandilla y la escalera más claras.
6. Si tienes un proyecto de AD5 grande, ábrelo y pasa `cargo run --release -p archisim-cli -- medir vistas tuproyecto.archisim`. Mándame lo que salga.

---

## 2026-10-24 (5) · v0.57: los remates de la 0.48, rehechos sobre la 0.56

Rama `bloque/remates-2`, pull request para Antonio (sustituye a #54). **Formato `.archisim` versión 34**: abre todas las anteriores, también los ficheros guardados con la 0.48, que tenía su propia versión 32.

Antonio pidió rehacer sobre la versión actual los remates del 23/10, que se habían quedado en una rama de la 0.48. Casi todo se ha traído tal cual. Las buhardillas no: la 0.53 ya tenía unas más completas (ventanas de cubierta, tejas y canalones), así que se quedan esas. Las buhardillas de un fichero de la 0.48 se colocan de nuevo, al abrirlo, en el mismo punto y con sus medidas.

**Hecho**
- **Texturas ligeras en 3D**: en las propiedades de la vista 3D, Texturas «Completas», «Ligeras» o «Sin texturas». Las ligeras se dibujan más sencillas, para las tarjetas gráficas de los Mac con Intel, y son las de serie en esos Mac. Ahora incluyen también las telas, fibras, terrazo y mármol de la Maqueta. Texturas, Imagen y la calidad del Renderizado son listas desplegables, para que el panel no se ensanche.
- **Puertas y ventanas en muros curvos**: el hueco es recto y en escuadra con el muro en su centro. Se ven en planta y 3D, salen en el IFC y se importan.
- **Instalaciones, segunda vuelta**:
  - pendiente de las tuberías;
  - codos y tes que salen solos del trazado (en el IFC, `IfcPipeFitting` e `IfcDuctFitting`);
  - radiadores, rejillas y difusores;
  - comprobación CTE de diámetros y pendientes (HS 4 y HS 5).
- **Presupuesto**:
  - la descomposición de cada partida, que se abre con ▸;
  - totales con gastos generales, beneficio industrial, PEC e IVA;
  - en el BC3, la descomposición (`~D`) y los porcentajes (`~K`).
- **Interferencias**: reglas por parejas de disciplinas y holguras. Lo que queda demasiado cerca sale en ámbar y se puede pasar a incidencias BCF.
- Revisado en la ventana real: la vista 3D con texturas ligeras y la tabla de Mediciones con su presupuesto.

**Pendiente**
- En Mediciones, el botón «Partida…» queda al borde derecho y hay que desplazar la tabla para verlo.
- Lo demás, en `docs/HOJA_DE_RUTA.md` §9 (C1–C7), a la espera del orden de Antonio.

**Qué debe probar Antonio**
1. Abre la demo (`out/demo.archisim`) y pulsa F3. En Propiedades, cambia Texturas a «Ligeras» y luego a «Sin texturas»: el 3D debe cambiar sin colgarse. En el MacBook con Intel deben salir «Ligeras» de serie.
2. Dibuja un muro curvo (Muro → arco) y pon en él una puerta y una ventana: deben quedar rectas, en escuadra con el muro. Mira la planta, el 3D y exporta el IFC.
3. Disciplina Instalaciones: dibuja una tubería de saneamiento con un quiebro y dale pendiente en Propiedades. Mira el codo en 3D y pon un radiador. Después, Colaboración → Comprobación CTE: deben salir diámetros y pendientes.
4. Pestaña Mediciones: abre `docs/ejemplos/banco-ejemplo.bc3` con «Abrir banco BC3…», asigna una partida y despliega su descomposición con ▸. Mira los totales con GG, BI e IVA y exporta el BC3.
5. Si guardaste algún proyecto con la 0.48 y buhardillas, ábrelo: las buhardillas deben seguir donde estaban.

---

## 2026-10-24 (4) · Hoja de ruta común entre conversaciones

En `main`, solo documentación (sin cambios de código, formato ni IFC).

Antonio trabaja con varias conversaciones a la vez y pidió una hoja de ruta común para no desandar lo andado. En esta conversación se habló de Paralives: no se usa su biblioteca (son modelos con derechos de su estudio y ArchiSim es GPL y de sala limpia), se replica lo que la hace bonita. Y de que los árboles se ven feos en alzado y sección: las secciones dibujan las aristas de la malla 3D de la copa, no un árbol.

**Hecho**
- `docs/HOJA_DE_RUTA.md` §9: la lista única de lo que falta (C1–C7), lo que ya está hecho y no se reabre, y la regla para trabajar a la vez: cada conversación marca en §9 el bloque que coge antes de tocar código.

**Pendiente**
- Que Antonio confirme o cambie el orden de §9. Propuesta: C1 (figuras y vegetación en alzado y sección, y coches y personas en el estilo de la biblioteca) primero.

**Qué debe probar Antonio**
- Nada que ejecutar. Leer `docs/HOJA_DE_RUTA.md` §9 y decir el orden.

---

## 2026-10-24 (3) · v0.56: la interfaz en islas, avisos con Deshacer y tres maneras de abrir las cosas (UI4)

Rama `bloque/islas`, sobre la 0.55 (lleva dentro #59), pull request para Antonio. Sin cambios de formato ni de IFC.

Antonio preguntó si se me había olvidado el rediseño de la interfaz del concepto. Se me había quedado a medias: las láminas «Técnica» y «Piezas comunes». Esto es lo que faltaba.

**Hecho**
- **Ventana en islas**: las fichas, Propiedades, el dibujo y el Navegador son tarjetas con esquinas redondeadas y sombra suave, separadas sobre un fondo cálido; el gris de antes pasa a una paleta crema.
- **Tres maneras de abrir las cosas**, cada una para lo suyo:
  - **panel lateral**, que entra deslizándose por la derecha y ocupa todo el alto entre las barras, para lo que se gestiona: Plantas, Capas de dibujo, Estilos de presentación, Parámetros del proyecto y Biblioteca;
  - **burbuja junto a su botón**, para lo corto: «+ Añadir planta» (en el Navegador, bajo las plantas) pide nombre, altura y si va encima o debajo, dibuja una pequeña sección con la planta nueva y explica qué pasa («Se pone a +3,55. Cubierta sube 3,00 m»);
  - **diálogo en el centro con el fondo oscurecido**, solo para decidir: el tipo ya está en el proyecto, purgar tipos sin usar, informe de importación.
- **Avisos abajo en el centro**, con un punto verde o rojo, y **Deshacer** en el mismo aviso cuando lo que se acaba de hacer cambió el modelo.
- **Tarjeta de tipo de puerta y ventana** con Planta · Alzado · 3D: por defecto en alzado, que es como se reconoce una carpintería.
- La caja de búsqueda de la barra superior ya no se monta sobre deshacer y rehacer cuando el nombre del fichero es largo.

**Pendiente**
- Del concepto quedan detalles menores: unificar todos los interruptores y botones con el mismo dibujo, y las opciones de colocación de huecos «Por el centro / Desde un extremo / Repartidas».
- La lista anterior sigue esperando el orden de Antonio: exportar `.glb`, render con texturas y de noche, pinceles de exterior, segunda vuelta de la cocina, detalles de la Maqueta, flechas de Concepto, deudas de escaleras y buhardillas, más láminas de documentación, decisiones D4, D5, D7, D8–D12, el catálogo de AD5 y las reglas del CTE.
- #54 (remates) espera su decisión: rehacerlo sobre la 0.56 o cerrarlo.

**Qué debe probar Antonio**
1. Abrir la demo: `cargo run --release -p archisim-app -- out/demo.archisim`. La ventana debe verse en tarjetas separadas sobre fondo crema.
2. Navegador → Proyecto → «+ Añadir planta»: la burbuja sale junto al botón. Cambiar la altura y «Encima / Debajo» y mirar la sección y la frase de abajo; «Añadir planta» y comprobar en «Plantas…» que las de encima han subido.
3. Justo después, abajo en el centro, el aviso con **Deshacer**: pulsarlo y ver que la planta desaparece y las demás vuelven a su cota.
4. «Plantas…», Capas, Biblioteca: entran por la derecha, con todo el alto. Purgar tipos (en Biblioteca): sale en el centro, con el fondo oscurecido.
5. Seleccionar una ventana: en Propiedades, la tarjeta del tipo con Planta · Alzado · 3D.

---

## 2026-10-24 (2) · v0.55: cocina por módulos, catálogo en árbol, diálogos y plano de carpinterías (UI2 y UI3)

Rama `bloque/cocina`, pull request para Antonio. Sin cambios de formato (familias nuevas, que los proyectos cargan de la biblioteca al usarlas) ni de IFC (los módulos son `IfcFurniture`).

Antonio dio por buena la 0.54 («funciona todo, sigue con lo que falta»): se unió a `main` (#58, que llevaba dentro #56 y #57). Esto es lo que faltaba del concepto.

**Hecho**
- **Catálogo en árbol**: grupo → tema → clase. Debajo de los temas, una fila de fichas con sus clases y «Todas»:
  - en Cocina: Bajos, Altos, Columnas, Campanas, Islas, Sobre la encimera, Electrodomésticos, Cocinas enteras;
  - en Ventanas y Puertas: Practicables, Abatibles, Correderas, Guillotina…;
  - en lo demás, las carpetas de la biblioteca.
- **Cocina por módulos**:
  - módulos bajos de 15 a 120 cm (cajonera, puertas, fregadero con grifo, horno, lavavajillas, placa sobre cajones, botellero, esquina), cada uno con su encimera hasta su borde. Al acercar uno a otro se pegan, y las encimeras se ven como una sola;
  - columnas (despensa, horno y microondas, frigorífico integrado);
  - frentes liso, con marco, ranurado o con cristal, también en los muebles altos (hay «Alto vitrina 80» con cristal);
  - 15 tipos nuevos en la biblioteca.
- **Barra del objeto**: además de girar, duplicar y los materiales, enseña lo que se puede elegir del mueble según su receta: «Qué lleva», cajones y puertas como botones, frente y tirador en una lista con «a toda la fila» para los módulos que se tocan en línea. La tarjeta dibuja el mueble en 3D.
- **Diálogos**: todos se abren junto al botón que los llama (Plantas…, Biblioteca, CTE…), no en la esquina sobre las herramientas; no se pliegan y tienen las mismas esquinas redondeadas y sombra. «Tipos y materiales», que es grande, sale en el centro.
- **Plano de carpinterías** (Archivo → Plano de carpinterías (PDF)…):
  - cada tipo de puerta y ventana que usa el proyecto, en alzado desde el exterior a 1:50, con ancho y alto acotados, su sección y sus datos: familia, medidas, apertura, carpintería y unidades;
  - seis por hoja A3, con cajetín. Los dibujos son los mismos del diálogo de tipos, que salen de las recetas.
- `archisim cocina out/`: la cocina de ejemplo en cuatro filas, una por frente (proyecto, IFC, planta y axonometría).
- Pruebas: módulos y estilos distintos en `archisim-props`; en la app, el tema Cocina con sus clases, la fila de módulos y «a toda la fila» frente a un solo módulo; las hojas de carpinterías y su PDF; la cocina de ejemplo con su 3D.
- Revisado en la ventana real: la cocina en la Maqueta con la barra de un módulo, el catálogo de Cocina con sus clases, el diálogo Plantas junto a su botón y las hojas de carpinterías de la demo.

**Pendiente**
- Encimeras de verdad continuas en el IFC y las mediciones (hoy cada módulo mide la suya); módulos de esquina en L con su encimera en ángulo; islas por módulos.
- Rueda de color libre (hoy, la paleta acogedora y los materiales del proyecto).
- Más hojas de documentación con el mismo cajetín (memoria de carpinterías, cuadro de superficies).

**Qué debe probar Antonio**
1. `archisim cocina out/` y abre `out/cocina.archisim`. Entra en la Maqueta, selecciona un módulo y, en su barra, cambia «Qué lleva» (de cajonera a fregadero), los cajones, el frente y el tirador. Prueba «a toda la fila».
2. En un proyecto tuyo: Amueblar → Cocina → Bajos. Pon una cajonera, un fregadero y un lavavajillas uno junto a otro contra una pared: deben pegarse y verse con una sola encimera. Estira el último hasta el muro con su flecha.
3. Construir → Ventanas: prueba las fichas Practicables, Correderas y las demás.
4. Navegador → Plantas…: el diálogo debe salir junto al botón, no arriba a la izquierda.
5. Archivo → Plano de carpinterías (PDF)… con la demo o con un proyecto de AD5. Ábrelo y dime qué datos faltan para que valga como plano del estudio.

---

## 2026-10-24 · v0.54: manejo común, menú del clic derecho y barra del objeto (UI1), unido a la 0.53

Rama `bloque/manejo`, pull request para Antonio. Sin cambios de formato ni de IFC.

Al probar la 0.49 de esta rama, Antonio echó de menos lo que ya tenía en la 0.53 de `bloque/escaleras`: el contorno de selección en lugar de pintarlo todo de azul, ver en vivo lo que se gira o se estira, la decoración, los muros recortados como en Los Sims y el día y la noche con colores cálidos. Esa rama, con `bloque/maqueta-2` dentro, aún no estaba unida a la versión principal y esta salió de la 0.48. Así que se ha unido aquí, como versión 0.54:
- la barra del objeto sustituye a la tarjeta flotante de la 0.49 de la Maqueta; sus muestras ofrecen primero los colores de la paleta acogedora que le van a cada pieza y después todos los materiales;
- el anillo de giro de la 0.49 se queda, pero naranja, delante del objeto como en planta, y gira sobre el centro del mueble;
- arrastrar muebles como en Paralives vale ya en los dos entornos; lo demás seleccionado se mueve con su vista previa;
- la cámara gira alrededor de lo seleccionado, ahora con el botón derecho.


Antonio aprobó el concepto de interfaz (artefacto «ArchiSim · concepto de interfaz», seis tableros) después de pedir sus cambios. Pidió quitar la barra negra de teclas, usar una barra del objeto como la de Paralives, cambiar el ratón (izquierdo elige y mueve, rueda desplaza, derecho mantenido gira, un clic derecho abre un menú), usar flechas y un aro distintos para estirar y girar, y que los objetos crezcan solo hacia donde se estiran. Este es el primer bloque (UI1). Los de después son el catálogo en árbol con la cocina por módulos (UI2) y las fichas y diálogos unificados con el plano de carpinterías (UI3).

**Hecho**
- **Ratón**, igual en planta y en 3D:
  - **izquierdo**: un clic selecciona; arrastrar en vacío abre una ventana de selección (de izquierda a derecha, lo que queda dentro; al revés, lo que toca); arrastrar lo seleccionado o un mueble lo **mueve** sobre su plano, con la distancia en una etiqueta;
  - **rueda pulsada**: desplaza la vista; girar la rueda acerca y aleja;
  - **derecho mantenido**: gira la vista 3D; en planta, desplaza.
- **Menú del clic derecho** (un solo clic, sin arrastrar), en planta y en 3D:
  - sobre un elemento: Propiedades, Cortar, Copiar, Pegar, Duplicar, Ocultar, Aislar, Seleccionar todos los de este tipo (también objetos), Llevar a la planta y Borrar;
  - en vacío: Pegar, Seleccionar todo y Mostrar todo;
  - Pegar pone la copia donde se abrió el menú; Cortar no borra nada hasta pegar, y entonces mueve;
  - Ocultar y Aislar valen para la sesión y no tocan el proyecto ni el IFC;
  - Ctrl+C, Ctrl+X y Ctrl+V hacen lo mismo, y Esc cierra el menú sin perder la selección.
- **Tiradores de los objetos**:
  - **flechas azules** en los lados derecho e izquierdo y en el frente (en 3D, también arriba); el objeto **crece solo hacia la flecha** y el lado contrario se queda donde estaba (se marca con un punto blanco); con **Alt**, crece por los dos lados;
  - **punto naranja** sobre un arco, delante del objeto, para girarlo a mano: de 15 en 15°, y de grado en grado si el cursor se aleja; mientras se gira, el anillo se ve entero;
  - en 3D, las flechas quedan un poco fuera del objeto, para que un mueble pequeño se pueda coger por su cuerpo y no por un tirador.
- **Sin el cartel negro de teclas** sobre el dibujo. Lo que hace el clic con la herramienta en uso aparece en gris al final de la barra de arriba; el resto, al dejar el cursor encima.
- **Barra del objeto** en la Maqueta, en lugar de la tarjeta flotante:
  - abajo a la izquierda, con su dibujo y su nombre en una etiqueta amarilla;
  - Girar, Invertir (puertas, ventanas y muros), Duplicar, Igualar y Borrar;
  - **sus materiales** como muestras redondas, que abren los colores del proyecto: cambian solo ese objeto (si su tipo lo comparten otros, se le hace uno propio);
  - Más….
- **Puertas y ventanas en alzado** en las tarjetas del catálogo de la Maqueta y en la barra del objeto; las tarjetas son algo más grandes, con el nombre entero en dos líneas.
- Pruebas nuevas: el menú del clic derecho en planta (Ocultar, Aislar, Copiar y Pegar, Cortar y Pegar), los materiales de la barra, la vista 3D con el botón derecho (gira sin mover nada, un clic abre el menú, el izquierdo arrastrando mueve) y el estirado por un solo lado y con Alt en planta y en 3D.
- Revisado en la ventana real (Xvfb): el menú en planta y en 3D, girar con el derecho, las flechas y el arco en planta, la barra del coche con sus muestras y el catálogo de ventanas en alzado.

**Pendiente**
- UI2: el catálogo en árbol (grupo → categoría con iconos → tipo) y la cocina por módulos (encimera que se une, cajones, tiradores, fregadero, placa, muebles altos, campanas y estilos).
- UI3: fichas y diálogos unificados (que nada aparezca en una esquina, como «Plantas») y el plano de carpinterías con alzados.
- Mover en 3D se hace sobre el plano del punto que se coge; todavía no se pega a las paredes ni muestra cotas temporales como en planta.

**Qué debe probar Antonio**
0. Comprueba que sigue todo lo de la 0.53: contorno de selección, decoración y colecciones, muros delante bajos, día y noche, escaleras.
1. Abre la demo (`out/demo.archisim`). En planta, haz clic derecho sobre un muro: debe salir el menú. Prueba Ocultar y después, con clic derecho en vacío, Mostrar todo. Prueba Copiar y luego clic derecho en otro sitio → Pegar.
2. Pulsa F3. Mantén el botón derecho y arrastra: la vista gira y no se mueve nada. Con la rueda pulsada, desplaza. Selecciona el coche con el izquierdo y arrástralo: se mueve por el suelo. En vacío, arrastra para hacer una ventana de selección (de izquierda a derecha y al revés).
3. Con el coche seleccionado, en planta o en 3D, estira una flecha azul: debe crecer solo hacia ese lado. Repítelo con Alt pulsado: crece por los dos. Coge el punto naranja y gíralo.
4. Entra en la Maqueta. Selecciona el coche o un mueble: abajo a la izquierda sale su barra. Cambia el color de la carrocería con su muestra redonda. Mira el catálogo Construir → Ventanas: deben verse en alzado.
5. Dime si el menú tiene lo que usas y si la barra del objeto te deja hacer sin teclado lo que hacías con teclas.
## 2026-10-24 (2) · v0.53: escaleras en L, en U, compensadas y de caracol, con zancas, zanca central, peldaños volados o mástil

Rama nueva `bloque/escaleras`, sacada de `bloque/maqueta-2`, con su pull request para que Antonio la revise. **Formato `.archisim` versión 33** (las escaleras guardan su forma y estructura en `design`; las de versiones anteriores se abren como tramos y mesetas sobre losa, igual que eran). Detalle en `docs/ESCALERAS.md`.

Antonio pidió mejorar mucho la forma de crear escaleras: en U, «diferentes tipos de castelillete», caracol. Lo he entendido como tipos de **estructura** (losa, zancas laterales, zanca central, peldaños volados, mástil); si quería decir otra cosa, que lo diga.

**Hecho**
- Formas hechas en dos clics, que miden lo que pide la altura: recta, en L, en U (con su ojo) y de caracol; y la libre de siempre. Espacio gira la L, la U o el caracol al otro lado mientras se dibuja; la vista previa dice tabicas, huellas y si cumple el CTE.
- Giros con meseta o con **peldaños compensados**, repartidos en cuatro peldaños antes y después, todos con la misma huella en la línea de huella (a 50 cm del lado interior).
- Estructura: losa (helicoidal en caracol), dos zancas laterales, zanca central, peldaños volados, mástil central. Con o sin tabica, y material del peldañeado (madera, piedra) con su grosor.
- Barandillas con pasamanos, listón bajo, postes y barrotes a 11 cm, siguiendo la escalera por las mesetas y los giros; la de caracol, solo por fuera.
- Planta genérica para todas (línea de corte en zigzag, lo de encima a trazos, mástil, línea de huella con su flecha), 3D y IFC (`IfcStair` con su forma, `WINDER`/`SPIRAL`, zancas `IfcMember STRINGER`), importación de vuelta igual.
- CTE: huella en la línea de huella, lado ancho ≤ 44 cm, lado estrecho ≥ 5 cm en vivienda, aviso de escaleras sin tabica en uso general.
- En la Maqueta, Construir → Escaleras: diez tarjetas con su miniatura en 3D (recta, L con meseta, L compensada, U con meseta, U compensada, caracol, de zancas, de zanca central, de peldaños volados, libre). Las escaleras ya no se cortan con los «muros recortados»: se ven enteras, como los muebles.
- Propiedades de una escalera: forma (meseta o compensada), giro total y sentido del caracol, radio del mástil, estructura, tabicas, zancas, peldañeado.
- `archisim escaleras out/`: las diez, cada una hasta la planta primera.

**Pendiente**
- Las mesetas partidas a 45° y la meseta de llegada de las de caracol.
- El `.glb` del proyecto para D5 o Twinmotion.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- escaleras out/` y abre `out/escaleras.archisim`: diez escaleras, todas de 17 tabicas de 17,5 cm. Míralas en planta (baja y primera) y en la Maqueta.
2. Herramienta Escalera → Forma «En U (ida y vuelta)»: clic en el arranque, mueve el ratón hacia donde sube (pulsa Espacio para que gire al otro lado) y clic. Prueba «Giros: peldaños compensados» y mira el reparto de los peldaños.
3. «De caracol»: clic en el centro y clic hacia donde arranca. Cambia la estructura a «Losa helicoidal» o «Zancas helicoidales».
4. Selecciona una escalera y cambia en Propiedades → Forma y estructura: compensada ↔ meseta, zancas, sin tabica, peldañeado de madera.
5. Exporta el IFC y ábrelo en ArchiCAD o Revit: las escaleras deben llegar con sus tramos, zancas y materiales, y con su tipo (en U, compensada, de caracol).

---

## 2026-10-24 (1) · v0.52: cámara alrededor de la selección, mobiliario urbano, chimeneas y más mobiliario

Misma rama `bloque/maqueta-2` y pull request #56. Sin cambios de formato `.archisim` ni de IFC (las familias de mobiliario urbano suben de versión con los mismos parámetros y un «Modelo 3D»). Detalle en `docs/BIBLIOTECA_3D.md` y `docs/MAQUETA.md` §6 sexies.

Antonio probó v0.51 (fluida incluso en su MacBook Pro de 2016) y pidió: mejores modelos (el mobiliario urbano era muy básico y las farolas no alumbraban), más accesorios y mobiliario, chimeneas, que la cámara gire alrededor de lo seleccionado y mejorar mucho las escaleras (en U, tipos de estructura, caracol). Las escaleras van en su propia rama, después de esto.

**Hecho**
- La cámara gira alrededor de lo seleccionado.
- Mobiliario urbano rehecho: bancos, farolas (báculo, fernandina, globo, baliza), papeleras, bolardos, buzones, contenedores, fuentes, jardineras, alcorques, aparcabicis, señales y bicicletas, con 3 o 4 modelos cada uno; 24 tipos nuevos.
- Las farolas y las chimeneas brillan y alumbran de noche (las farolas, hasta 11 m).
- Chimeneas (de obra, estufa de leña, moderna lineal), perchero y recibidor, sillas de oficina, mesas de exterior con sus sillas, silla colgante y hamaca, radiadores, aire acondicionado, ventiladores de techo, pianos, cama cabaña y litera, estantería con objetos; 22 tipos nuevos.
- La demo tiene dos farolas fernandinas junto al camino.

**Pendiente**
- Escaleras: en U, en L, peldaños compensados, caracol, tipos de estructura (zancas laterales, zanca central, losa, peldaños volados), con o sin tabica: rama `bloque/escaleras`, lo siguiente.
- El `.glb` del proyecto para D5 o Twinmotion.

**Qué debe probar Antonio**
1. Selecciona un mueble en la Maqueta y gira la cámara: debe girar a su alrededor, sin perderlo de la pantalla.
2. Demo en la Maqueta, **Luz: Noche**: las dos farolas del camino se encienden y alumbran el suelo.
3. Exterior → Mobiliario urbano y Jardín y terraza: bancos, farolas, papeleras, fuentes, jardineras, mesas de exterior y la hamaca, con sus modelos nuevos (cámbialos en «Modelo 3D»).
4. Amueblar → Salón: una chimenea de obra o una estufa de leña contra un muro; de noche, el fuego alumbra el salón.
5. `cargo run -p archisim-cli -- modelos out/`: las cuatro primeras filas son todo lo nuevo.

---

## 2026-10-23 (6) · v0.51: imagen de la Maqueta y miniaturas en 3D

Misma rama `bloque/maqueta-2` y pull request #56. Sin cambios de formato `.archisim` ni de IFC (los tipos de textura nuevos se guardan como los de siempre). Antonio eligió seguir por los puntos 2 (imagen) y 3 (miniaturas en 3D) y dejar para después el `.glb` para D5 y Twinmotion. Detalle en `docs/MAQUETA.md` §6 quinquies.

**Hecho**
- Cielo y suelo con bruma detrás del modelo en la Maqueta; la cuadrícula, dibujada en ese suelo.
- Menú **Luz**: día (sol del proyecto), atardecer (sol bajo, dorado) y noche (las lámparas encienden sus pantallas y bombillas y alumbran alrededor).
- Texturas de lino, bouclé, punto, terciopelo, mimbre, terrazo y mármol, que las telas, fibras y piedras de la paleta toman solas.
- **Captura grande…** en el menú **Imagen**: PNG de 3840 píxeles de ancho.
- **Miniaturas en 3D** en el catálogo, también en los colores de la colección elegida.
- Arreglo: líneas blancas alrededor de todo de noche (la imagen quedaba medio transparente en los contornos y el vidrio).

**Pendiente**
- El `.glb` de todo el proyecto para D5, Twinmotion o Blender (lo siguiente, cuando digas).
- La imagen realista (`archisim render`) sin las texturas nuevas ni la noche; sombras de las lámparas.

**Qué debe probar Antonio**
1. Abre la demo en la Maqueta. Gira la cámara hasta ver el horizonte: cielo azul arriba, el suelo perdiéndose en la bruma.
2. **Luz: Atardecer** y luego **Noche**: de noche, la lámpara de la mesita y la de pie del salón se encienden y alumbran el suelo y las paredes de su habitación.
3. Acércate al sofá con **Imagen: Bonita**: el lino, los cojines de terciopelo y la manta de lana deben tener su textura; una butaca de ratán, su trenzado.
4. Amueblar → Salón y Decoración: las tarjetas son ahora las piezas en 3D. Elige **Colección: Japandi**: salen con sus colores.
5. **Imagen → Captura grande…** y guarda el PNG: ábrelo, son 3840 píxeles de ancho, listos para pasarlos a una IA o a D5.

---

## 2026-10-23 (5) · v0.50: colecciones y piezas en el mismo estilo

Misma rama `bloque/maqueta-2` y pull request #56. Sin cambios de formato `.archisim` ni de IFC (la pérgola sale como `IfcBuildingElementProxy`; lo demás, `IfcFurniture`). Detalle en `docs/BIBLIOTECA_3D.md` («Colecciones») y `docs/MAQUETA.md` §6 quater.

**Hecho**
- Cuatro **colecciones** con su paleta: nórdico cálido, mediterráneo, años 50 y japandi. Cada pieza de la biblioteca tiene su versión en cada una («Sofá de 3 plazas · Japandi»): madera, telas, cojines, metal, cerámica, piedra y fibra de la colección, y el modelo 3D que le va (cama de lamas en japandi, taburete de enea en mediterráneo…). Sábanas, pantallas, vidrio, cromados y porcelana se quedan como están.
- En la Maqueta, **Colección** encima del catálogo (cada tarjeta lleva la tira de colores de la colección y la pieza se coloca ya en ella) y **Aplicar** para pasar lo seleccionado; «Sin colección» lo devuelve.
- 44 tipos nuevos: camas con cabecero tapizado o de lamas, con colcha a los pies y cojines; mesitas de patas cónicas y flotante; taburetes (madera, tapizado, enea); islas de cocina (con voladizo, mesa de trabajo con cestas, con fregadero); cosas de la encimera (tablas y aceite, botes, cafetera y tazas, hierbas); toallas (dobladas, colgadas, enrolladas); cestas; cuadros, tríptico, espejo de arco, espejo sol y reloj; cojines de suelo; mantas (doblada, pila, en escalera); tumbonas, cama de exterior, sombrillas, macetones (olivo, ánfora, gramíneas, lavanda) y pérgolas.
- Exterior tiene la pestaña **Jardín y terraza**. Lo de la encimera, las toallas y las mantas se posan solas donde se pinchan; los taburetes y lo del jardín, en el suelo.
- `archisim modelos out/` empieza con una fila por colección (las mismas once piezas en los cuatro estilos) y luego todas las piezas nuevas.

**Pendiente**
- Estanterías con objetos; muebles en L; más colecciones si las quieres (rústica, industrial…).
- Lo que dijiste en el punto 3 (texturas como imágenes, cielo, lámparas que alumbran de noche, captura grande) y el 4 (exportar el proyecto entero a `.glb` para D5 o Twinmotion).

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- modelos out/` y abre `out/modelos.archisim` en la Maqueta con imagen Bonita: las cuatro primeras filas son las mismas piezas en nórdico, mediterráneo, años 50 y japandi; detrás, todas las piezas nuevas. Dime qué colección o qué pieza cambiarías.
2. En la demo, Maqueta → Amueblar → **Colección: Japandi**. Mira Dormitorio y Cocina: cada tarjeta lleva la tira de colores de Japandi. Pon una cama y una mesita: deben salir en roble ahumado y lino piedra, la cama con cabecero de lamas.
3. Selecciona con un recuadro todo el salón de la demo, elige **Mediterráneo** y pulsa **Aplicar**: sofá, butaca, alfombra y lámpara cambian de golpe. Ctrl+Z lo deshace en un paso.
4. Exterior → **Jardín y terraza**: una pérgola con cañizo, dos tumbonas y una sombrilla junto a la casa; y un macetón con olivo.
5. Amueblar → Decoración: un cuadro y un espejo de arco contra una pared (se pegan de espaldas), unas mantas en escalera y un cojín de suelo.

---

## 2026-10-23 (4) · v0.49: colocar como en Paralives

Misma rama y pull request #56. Sin cambios de formato. Detalle en `docs/MAQUETA.md` §6 ter.

**Hecho**
- Lo pequeño (decoración, lámparas de mesa, plantas) se posa solo en la mesa, la balda o la encimera que hay debajo, en 3D y en planta.
- En la Maqueta, los muebles se arrastran por el suelo sin herramienta, viéndolos donde quedarán; cerca de un muro se pegan a él de espaldas, mirando a la habitación. También al colocarlos.
- Muebles que se unen: dos de cocina, altos, estanterías, aparadores o sofás iguales, uno junto a otro, se hacen uno (con sus puertas o plazas).

**Pendiente**
- Unirse al arrastrar en planta; muebles en L.

**Qué debe probar Antonio**
1. En la Maqueta de la demo, pincha el sofá y arrástralo por el salón: acércalo a otra pared y suéltalo; debe quedar de espaldas a ella.
2. Amueblar → Decoración → Jarrón con ramas, y pincha encima de la mesa del comedor: debe quedar sobre el tablero. Prueba también una lámpara de mesa sobre la mesita de noche.
3. Amueblar → Cocina → «Muebles de cocina 240» junto al que ya hay, en línea: deben quedar como un solo mueble de 4,80 m.
4. Pon dos sofás iguales uno al lado del otro: uno solo, más largo, con más plazas.

---

## 2026-10-23 (3) · v0.49: biblioteca acogedora (al modo de Paralives)

Misma rama `bloque/maqueta-2` y pull request #56. Sin cambios de formato `.archisim`. Detalle en `docs/BIBLIOTECA_3D.md` («Biblioteca acogedora»).

Antonio probó en el Mac la Maqueta con sombras y pidió seguir por la biblioteca, para tener la calidad y el aire acogedor de Paralives.

**Hecho**
- Nueve familias nuevas con su 3D hecho por ArchiSim y sus símbolos de planta: butaca (4 modelos), puf (3), mesa auxiliar (3), aparador (2), alfombra (4), lámpara (4, la colgante con su cable), planta de interior (5), cortina (3, abierta o corrida) y decoración (5: jarrón, frutero, libros y vela, cerámicas, cesta).
- Sofá con cojines sueltos y manta; 34 tipos nuevos en la biblioteca (`archisim modelos out/` los coloca todos).
- Paleta acogedora de 24 materiales y **color por pieza** en la tarjeta de la Maqueta (solo ese objeto; un paso de Deshacer).
- Pestaña **Decoración** en Amueblar (alfombras, lámparas, plantas, cortinas y decoración).
- La demo tiene un salón vestido (sofá acogedor, alfombra, butaca, lámpara y planta).
- Arreglo: con los muros recortados, las pantallas de las lámparas (y cualquier pieza alta de un mueble) desaparecían; ahora un mueble se corta por donde se apoya, no por su pieza más baja.

**Pendiente**
- Decoración que se apoya sola en mesas y baldas (hoy, con el desfase de la base).
- Más colecciones y piezas en el mismo estilo (dormitorio, cocina, baño).

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- demo out/` y abre `out/demo.archisim` en la Maqueta, imagen Bonita: el salón con su sofá con cojines y manta, la alfombra, la butaca, la lámpara y la planta.
2. Amueblar → **Decoración**: pon una alfombra, una lámpara de pie, una planta y unas cortinas delante de una ventana.
3. Selecciona el sofá: en la tarjeta, cambia la tapicería, los cojines y las patas con los colores de cada fila. Pon otro sofá igual: debe quedarse como estaba.
4. `cargo run -p archisim-cli -- modelos out/` y abre `out/modelos.archisim`: las dos primeras filas son todas las piezas nuevas. Dime cuáles te convencen y cuáles cambiarías: es lo que más mejora con tu ojo.

---

## 2026-10-23 (2) · v0.49: Maqueta, segunda vuelta (revisión de Antonio)

Rama `bloque/maqueta-2`, pull request para Antonio (v0.48 ya unida a `main`). Sin cambios de formato `.archisim` ni de IFC. Detalle en `docs/MAQUETA.md` §6 bis.

Antonio probó v0.48 y pidió: que el recorte de muros no corte los muebles; el modo de Los Sims en que se ocultan solo los muros de delante; ver el objeto estirarse mientras se arrastra; girar a cualquier ángulo; que la selección no tape el material con azul; y muebles estilizados que, como en Paralives, crezcan con fluidez al estirarlos.

**Hecho**
- **Los muebles no se cortan** con los muros recortados (quedan enteros; los de las plantas de arriba desaparecen). La caja de sección sigue cortándolo todo. Cada vértice del 3D sabe qué es su elemento (`view3d/cutaway.rs`) y el sombreador y la selección aplican la misma regla.
- **Muros delante bajos**, nuevo modo y el de entrada en la Maqueta: los muros entre la cámara y el centro de la vista bajan a 30 cm, con sus puertas y ventanas; los del fondo, enteros; sigue a la cámara.
- **Estirar en vivo** en 3D: el objeto se dibuja como quedará mientras se arrastra el pinzamiento (con sus puertas, plazas o cajones nuevos); al soltar cambia el modelo.
- **Giro libre**: anillo en el suelo alrededor del objeto (saltos de 15°, Mayús libre, también en vivo), ángulo escrito en la tarjeta junto a «Girar», y Mayús+Espacio (15°) al colocar.
- **Selección con contorno naranja** en 3D; los colores y texturas quedan tal cual.
- **Muebles que crecen por piezas**: sección `stretch` en las recetas (`docs/FAMILIAS.md`). Sofá (plazas), Mesa (sillas), Armario y Mueble alto (puertas), Estantería (baldas) y Cómoda (columnas); vale también al estirar en planta.
- **Más mullidos**: cojines, almohadas y edredón hinchados (`Builder::pillow`) y aristas redondeadas de los muebles sombreadas suaves en 3D.
- Esc, Espacio y Supr funcionan en la vista justo después de pinchar una tarjeta o un botón (el botón ya no se queda con el teclado).
- **W A S D y Q E** en la Maqueta: moverse por el suelo hacia donde mira la cámara, bajar y subir; con Mayús, más rápido.
- **Sombras del sol y luz de ilustración** (V1, primera vuelta; `docs/MAQUETA.md` §4): niveles de imagen Ligera, Normal y Bonita en la barra de la Maqueta y en Propiedades de la vista 3D. Las sombras siguen la fecha y la hora del sol del proyecto.
- El contorno de la selección se reescribió sin bucles: el compilador de sombreadores de Windows no lo aceptaba y el 3D no se habría dibujado allí.

**Pendiente**
- Los pinzamientos se quedan en su sitio mientras se gira el objeto (se recolocan al soltar).
- Un catálogo de muebles más estilizado al modo de Paralives (diseños nuevos, no solo los de hoy más mullidos) y la rueda de color por pieza (MQ2).
- V1, la imagen (sombras, luz de ilustración, niveles de calidad).

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-app -- out/demo.archisim` (antes `cargo run -p archisim-cli -- demo out/`) y pulsa **Maqueta**: los muros de delante bajos y los del fondo enteros. Gira la cámara alrededor de la casa: los que bajan cambian con ella. Prueba también «Muros recortados» y «Muros bajos»: armario, frigorífico y cama siguen enteros.
2. Pincha el armario del dormitorio: contorno naranja, su madera se ve igual. Arrastra el cuadradito azul de un lado: crece mientras arrastras y gana puertas al pasar de unos 1,30 m.
3. Arrastra el punto azul del anillo del suelo: gira a saltos de 15°; con Mayús pulsada, libre. En la tarjeta, escribe 30 junto a «Girar» y pulsa «Girar».
4. Amueblar → Salón → un sofá: estíralo a lo largo y mira cómo gana un cojín; la mesa del comedor, sus sillas.
5. Cambia el material del sofá con él seleccionado (Más… → material): se ve el cambio sin deseleccionar.
6. En el MacBook: dime si girar la cámara con los muros de delante bajos va fluido.
7. En la Maqueta, muévete con W A S D, baja y sube con Q y E.
8. Arriba, **Imagen: Ligera · Normal · Bonita**. En el PC, Normal y Bonita: sombras de árboles, muebles y tabiques; cambia la hora del sol en Propiedades del proyecto y míralas moverse. En el MacBook empieza en Ligera: prueba Normal y dime si va fluido o si se cuelga.

---

## 2026-10-23 · v0.48: entorno Maqueta y cubiertas (buhardillas, tejas y canalones)

Rama `bloque/maqueta`, pull request para Antonio. Formato `.archisim` versión 32 (abre todas las anteriores). Diseño en `docs/MAQUETA.md`, hoja de ruta §8.

Antonio quiere un editor tan intuitivo como Los Sims y tan modular como Paralives, con una imagen cartoon con sombras que no castigue a su MacBook de 2016. Decidido con él: no es otro programa ni otra disciplina, sino un entorno (Técnica · Maqueta) sobre el mismo modelo. Al probar, pidió además buhardillas (no se dejaban colocar: el elemento no existía) y tejas y canalones.

**Hecho**
- **Buhardillas y ventanas de cubierta** (K1): a dos aguas, a un agua y planas, y ventanas en el plano del faldón. Viven en su cubierta: se colocan por el centro del frente, miran aguas abajo, cortan el faldón solas y siguen a la cubierta (mover, girar, simetría, cambiar sus lados). Herramienta Cubierta → «Buhardilla» o «Ventana de cubierta» en la barra de arriba; Propiedades propias (clase, ancho, altura del frente, pendiente, ventana del frente, materiales). Se seleccionan, mueven, copian y borran solas.
- **Cubrición** (K2), en Propiedades de la cubierta: teja árabe, mixta, plana, pizarra o chapa (hiladas escalonadas y cobijas o juntas en 3D, caballete en cumbreras y limatesas, rayado en la planta de cubiertas) y canalón de media caña o cuadrado con bajantes hasta la planta más baja.
- **IFC**: la buhardilla, `IfcElementAssembly` con frente y laterales (`IfcWall`), cubierta (`IfcSlab` `ROOF`) y ventana; la ventana de cubierta, `IfcWindow` `SKYLIGHT`; las tejas, `IfcCovering` `ROOFING` que cubre la cubierta; canalones y bajantes, `IfcPipeSegment` `GUTTER` y `RIGIDSEGMENT`. Los faldones llevan los huecos y sus cantidades netas. Al importar un IFC de ArchiSim vuelven con su cubierta, no como muros o tuberías sueltos.
- **Mediciones y BC3**: m² de cubrición (con nº de piezas), ml de caballete, canalón y bajante, ud de buhardilla.
- **Entorno Maqueta** (MQ1, primera vuelta): botones Técnica · Maqueta arriba a la derecha y tecla Tab; el 3D a toda la ventana; catálogo abajo (Construir · Amueblar · Exterior, por temas y estancias, con miniaturas); tarjeta junto a lo seleccionado (girar, duplicar, cuentagotas, borrar, Más…); plantas arriba y abajo; cuadrícula de 50 cm con ajuste; **Habitación** (cuatro muros, suelo y local de una vez).
- **Muros recortados** en el 3D de los dos entornos: arriba, recortados a 1,20 m o bajos a 30 cm, con los cortes rellenos.
- `archisim cubiertas out/`: una casa con dos buhardillas a dos aguas, una a un agua, una plana y ventanas de cubierta, y cuatro garajes con cada clase de teja; también la demo lleva teja árabe, canalón y dos ventanas de cubierta en el porche, y teja mixta en la caseta.

**Pendiente**
- V1, la imagen (sombras, penumbra guardada, luz de ilustración, texturas como imagen para los Mac con Intel, niveles de calidad): siguiente bloque.
- De la Maqueta: «solo los muros de delante», miniaturas en 3D, arrastrar objetos por el suelo en 3D, Habitación en un solo paso de Deshacer (hoy tres), rueda de color por pieza (MQ2).
- Buhardillas: en cubiertas con limahoyas que pasen por encima de un encuentro; de momento deben caber en un solo faldón.

**Qué debe probar Antonio**
1. `archisim cubiertas out/` y abre `out/cubiertas.archisim`: mira la vista 3D, la planta de cubiertas (planta baja a 1:100) y las mediciones (Navegador → Tablas → Mediciones, capítulo Cubiertas).
2. En la demo, selecciona la cubierta del porche: en Propiedades → Cubrición, cambia la teja y quita y pon el canalón.
3. Herramienta Cubierta (T) → arriba «Buhardilla» → clic en un faldón (en planta o en 3D). Selecciónala, cambia su clase y su altura en Propiedades, muévela arrastrando, y mueve o gira la cubierta: la buhardilla debe ir con ella. Pruébala demasiado arriba: debe avisar de que no cabe.
4. Pulsa **Maqueta** (arriba a la derecha) o Tab. Prueba Construir → Habitaciones → Habitación (dos esquinas), luego Puertas y Ventanas, Cubiertas → A dos aguas (clic dentro del edificio) y Buhardilla; Amueblar → Salón → un sofá. Selecciona el sofá: en la tarjeta, Girar, Duplicar y Cuentagotas. Cambia entre muros arriba, recortados y bajos, y entre plantas con ▲▼.
5. En el MacBook: entra en la Maqueta con los muros recortados y gira la cámara; dime si va fluido.
6. Exporta el IFC de `cubiertas.archisim` y ábrelo en ArchiCAD o Revit: buhardillas, ventanas de cubierta, tejas y canalones deben verse en su sitio.

---

## 2026-10-22 (13) · v0.47.2: el 3D en el MacBook Pro de 2016

El muestreo del Mac de Antonio (MacBook Pro 2016, Intel con AMD Radeon, macOS 15 con controladores parcheados) muestra el programa esperando, sin hacer nada: no se queda colgado nuestro código, sino la tarjeta gráfica al pintar. Solo pasa con algo dibujado, es decir, cuando corre el sombreador de las caras, que desde hoy (bloque Formas) dibuja las texturas de los materiales. El compilador de sombreadores de AMD de ese Mac se cae también con Safari.

**Hecho**
- **Texturas de los materiales opcionales** (Propiedades de la vista 3D → «Texturas de los materiales»). Apagadas, el código de texturas ni siquiera llega a la tarjeta gráfica. En los Mac con Intel vienen apagadas por defecto; la elección se guarda en las preferencias.
- El vigilante solo apunta las esperas de verdad: un fotograma que no termina. Antes también apuntaba la ventana quieta esperando al ratón.

**Confirmado por Antonio** (22/10/2026, noche): con las texturas apagadas el 3D vuelve a ir fluido en su Mac; eran las texturas.

**Pendiente**
- Una versión de las texturas que su tarjeta aguante (sin `switch` ni bucles en el sombreador, o hechas como imagen y no calculadas), para poder encenderlas también en los Mac con Intel.

**Qué debe probar Antonio**
1. Baja la 0.47.2 del pull request #53 y entra en el 3D con la demo: debe ir fluido, con colores lisos.
2. En Propiedades de la vista 3D, enciende «Texturas de los materiales»: si se cuelga, era eso. Vuelve a abrir ArchiSim, quedarán apagadas.

---

## 2026-10-22 (12) · v0.47.1: lentitud, recuadro blanco y cierres del 3D en Mac

En la rama `bloque/render-presto` (#53), sobre lo de los bloques 2–7, tras la prueba de Antonio («más lento al abrir en Windows y en Mac; al dibujar un muro o un forjado sale un borde blanco enorme que tapa la cuadrícula; el Mac se cierra al entrar en la vista 3D»).

**Hecho**
- **Lentitud**: medido con `archisim medir` frente a `main`. Dos culpables:
  - Los accesorios de los tipos (bloque 5) rehacían el índice de muros de todo el proyecto por cada muro. Con 1100 muros, la planta en pantalla había pasado de 17 a 877 ms y el 3D de 170 a 954 ms. Ahora se calcula una vez: 39 y 155 ms.
  - Buscar una familia por su nombre leía y analizaba las 54 recetas cada vez. Empezar un proyecto nuevo había subido a 0,5 s; ahora se leen una sola vez y son 0,02 s.
- **Recuadro blanco**: la vista previa en planta (bloque 3) tapaba su zona con papel blanco. Ahora vuelve a dibujar la cuadrícula encima del papel, así que solo se ve lo que cambia.
- **Mac y 3D**:
  - En pantallas Retina el 3D se dibujaba al doble de una resolución que ya es doble: imágenes de unos 5000 × 3000 píxeles con cuatro muestras, cientos de MB de la tarjeta gráfica. Ahora, en Retina, a la resolución de la pantalla (y nunca más de 4096 píxeles de lado).
  - Un error de la tarjeta gráfica ya no cierra el programa: la vista 3D se detiene con un aviso.
  - **Vigilante**: si la ventana deja de responder más de 4 segundos (la pelota de playa), se apunta qué estaba haciendo (preparar la escena 3D, subirla a la tarjeta gráfica, dibujarla…).
  - **Registro de errores**: cualquier fallo se apunta en `errores.log`, en la carpeta de ajustes de ArchiSim. Al abrir otra vez, ArchiSim lo aparta como `errores-anterior.log` y avisa de dónde está.

**Pendiente**
- Si el Mac se sigue cerrando, leer su `errores-anterior.log`. No tengo un Mac para reproducirlo.

**Qué debe probar Antonio**
1. Abre ArchiSim en Windows y en Mac: ¿arranca y abre la demo con la rapidez de antes?
2. Dibuja un muro y un forjado: mientras los colocas, la cuadrícula debe seguir viéndose alrededor (sin recuadro blanco).
3. En el Mac, entra en la vista 3D (F3) varias veces, gira y haz zoom. Si se cierra, vuelve a abrir ArchiSim: avisará de un fichero `errores-anterior.log` en `~/Library/Application Support/ArchiSim/` (en Windows, `%APPDATA%\ArchiSim\`). Mándamelo.

---

## 2026-10-22 (11) · Revisión de Antonio, bloque 7: Render y Presto

Rama `bloque/render-presto` (sobre `bloque/simbolos`), pull request para Antonio. v0.47. Formato `.archisim` versión 31 (abre todas las anteriores). Sin cambios en el IFC. Descrito en `docs/RENDER_PRESUPUESTOS.md`. Con este bloque la revisión del 22/10/2026 queda completa.

**Hecho**
- **Renderizado**: imagen realista con el sol del proyecto, sombras, luz del cielo y vidrio, calculada en el procesador (sin tarjeta gráfica). En la vista 3D, Propiedades → Renderizado (Borrador, Normal, Alta) desde la cámara de la vista, mientras se sigue trabajando; Guardar PNG. Orden `archisim render`.
- **Presupuesto en Mediciones**: capítulos y partidas con cantidad, precio, importe, totales y ejecución material.
- **Bancos de precios BC3**: abrir cualquier banco FIEBDC-3 (precios calculados por descomposición cuando no los trae), buscar sin tildes, asignar una partida a cada tipo, acabado o instalación; precio a mano; aviso si la unidad no coincide.
- **BC3 para Presto** con los códigos, resúmenes, precios y textos del banco y las mediciones elemento a elemento.
- Banco de ejemplo propio (`docs/ejemplos/banco-ejemplo.bc3`, precios inventados) y la demo con su presupuesto.

**Pendiente**
- Render: texturas dibujadas, luces artificiales, rebotes de luz en interiores; que la caja de sección recorte la imagen.
- Presupuesto: descomposición propia, costes indirectos, GG y BI; partidas por elemento suelto.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-cli -- demo out/` y luego `cargo run --release -p archisim-cli -- render out/demo.archisim out/demo-render.png`: mira la imagen (sol de la mañana desde el sureste, sombras de los árboles en el suelo).
2. Abre `out/demo.archisim` en la aplicación, ve a la Vista 3D, gira la cámara a tu gusto y pulsa Propiedades → Renderizado → Renderizar (prueba Borrador y luego Normal). Cambia el día y la hora del sol en Propiedades del proyecto y vuelve a renderizar. Guarda el PNG.
3. Navegador → Proyecto → Tablas → **Mediciones**: el presupuesto de la vivienda con el banco de ejemplo. Pulsa **Abrir banco BC3…** y abre un banco real del estudio (el de tu comunidad o uno exportado de Presto); en una fila, **Partida…**, busca y **Asignar**. Escribe un precio a mano en otra.
4. **Exportar BC3…** y ábrelo en Presto: comprueba capítulos, códigos, precios, textos y las líneas de medición.

---

## 2026-10-22 (10) · Revisión de Antonio, bloque 6: Símbolos DXF

Rama `bloque/simbolos` (sobre `bloque/muebles`), pull request para Antonio. v0.46. Formato `.archisim` versión 30. Sin cambios en el IFC. Descrito en `docs/BIBLIOTECA_3D.md` («Símbolos de planta desde DXF»).

**Hecho**
- **Símbolo de planta desde DXF** para cualquier tipo de objeto (Propiedades → Símbolo de planta → Importar DXF…): sus líneas, arcos, círculos, polilíneas y rellenos con sus colores y grosores, estirado al objeto o a su medida; cambiar y quitar.
- La demo dibuja un árbol con un símbolo de CAD.

**Pendiente**
- Elegir un bloque concreto de un DXF con muchos.

**Qué debe probar Antonio**
1. En la demo, mira el árbol de abajo del todo: un símbolo de CAD verde (copa, círculo y ramas).
2. Selecciona otro árbol → Propiedades → Símbolo de planta → Importar DXF… y elige un símbolo de árbol del estudio (un DXF con un solo árbol). Cambia «Tamaño» entre «Del objeto» y «Del dibujo».
3. Haz lo mismo con un mueble (un sofá o una cama del estudio) con «Del dibujo».

---

## 2026-10-22 (9) · Revisión de Antonio, bloque 5: Muebles y accesorios

Rama `bloque/muebles` (sobre `bloque/modelado-2`), pull request para Antonio. v0.45. Formato `.archisim` versión 29 (abre todas las anteriores). IFC: accesorios como `IfcCovering`. Descrito en `docs/BIBLIOTECA_3D.md` («Muebles y accesorios»).

**Hecho**
- **Estirar muebles en la vista 3D** con cuadros azules en sus lados, frente y altura.
- **Frentes**: cajones y tiradores (barra, pomo, uñero, asa, sin tirador) en mesita, armario, encimera, mueble alto y mueble TV; **cómoda** y **escritorio** nuevos.
- **Baño y cocina**: espejo, toallero, portarrollos, barra de apoyo, campana extractora y balda.
- **Accesorios de los tipos**: rodapiés, molduras, cornisas y albardillas en los tipos de muro (siguen a sus muros y se cortan en puertas y ventanas) y canto (cornisa o alero) en los de forjado; en 3D, planta, secciones, alzados e IFC.

**Pendiente**
- Llevar los accesorios en `.astipos` (del IFC de ArchiSim ya vuelven); accesorios en muros curvos y en cubiertas inclinadas.

**Qué debe probar Antonio**
1. Abre la demo: en 3D mira los rodapiés por dentro y la cornisa de piedra de la cubierta; en el alzado sur, la cornisa.
2. Selecciona en 3D un mueble (el armario o la mesa) y arrastra los cuadros azules: ancho, fondo y altura.
3. Tipos y materiales → Muro de fachada → Accesorios → «Cornisa»: aparece en todos los muros de fachada. Prueba «Moldura» a 1 m y mira cómo se corta en las ventanas.
4. Tipos y materiales → un tipo de forjado → Canto → «Alero».
5. Coloca una Cómoda y un Escritorio; en Propiedades del tipo, cambia cajones, columnas, cajonera y tirador.
6. `archisim modelos out/` y mira `out/modelos-3d.svg`: todos los muebles y accesorios nuevos, de frente.

---

## 2026-10-22 (8) · Revisión de Antonio, bloque 4: Modelado

Rama `bloque/modelado-2` (sobre `bloque/inicio`), pull request para Antonio. v0.44. Formato `.archisim` versión 28 (abre todas las anteriores). Sin cambios en el IFC (los volúmenes de Concepto no se exportan). Descrito en `docs/FORMAS.md` §5.

**Hecho**
- **Dividir** caras de volúmenes con una línea; cada parte se pinta (también en planta) o se empuja aparte.
- **Empujar cualquier cara**: lados (el contorno sigue editable), base, partes divididas y caras de sólidos, hacia fuera o hacia dentro.
- **Deformar**: retorcer (girando el ratón) y doblar (con Mayús); torsión, curvatura y dirección en Propiedades.
- **Perfiles de Seguir**: tubo, L, U, doble T, cornisa, media caña o tomado del dibujo.
- **Redondear** aristas de los volúmenes seleccionados desde la barra bajo las pestañas.

**Pendiente**
- Dividir solo entre dos aristas (hoy, de borde a borde) y empujar caras curvas.

**Qué debe probar Antonio**
1. `archisim formas out/` y abre `out/formas.archisim`; mira la vista 3D: torre retorcida, torre doblada, bloque escalonado, cornisa y tubo.
2. Concepto → Arista con rectángulo, empújalo 3 m. Con Empujar, clic junto a un lado en planta y aléjate: el volumen crece por ese lado.
3. Concepto → Dividir: en 3D, dos clics cruzando la cara de arriba. Empuja una de las partes hacia arriba y pinta la otra de madera.
4. Concepto → Deformar: clic en el volumen y gira el ratón a su alrededor; con Mayús, dóblalo. Prueba también en Propiedades → Deformar.
5. Concepto → Seguir: en Propiedades elige Perfil «Cornisa» y dibuja un camino. Dibuja una polilínea cerrada con el dibujo 2D y prueba «Tomar del dibujo…».
6. Selecciona un volumen y pulsa «Redondear» en la barra bajo las pestañas.

---

## 2026-10-22 (7) · Revisión de Antonio, bloque 3: Inicio y vistas previas

Rama `bloque/inicio` (sobre `bloque/graficos`), pull request para Antonio. v0.43. Sin cambios en el formato `.archisim` ni en el IFC. Descrito en `docs/INICIO.md`.

**Hecho**
- **Pantalla de inicio** al abrir sin fichero: proyecto nuevo, abrir, importar IFC, proyecto en equipo, primeros pasos y los recientes con un dibujo de su planta y cuándo se cambiaron. Archivo → Abrir reciente y Archivo → Pantalla de inicio.
- **Vista previa real en la planta**: antes del clic se ve lo que se coloca con su dibujo de verdad (mueble, puerta o ventana en su muro cortado, muro con capas y uniones, pilar, viga, forjado); al arrastrar, lo que se mueve en su sitio nuevo y sin dibujar en el antiguo.

**Pendiente**
- Vista previa real para cimentaciones, plantaciones, instalaciones y módulos, y al arrastrar en 3D.

**Qué debe probar Antonio**
1. Abre ArchiSim sin fichero (desde el menú de inicio o el icono): sale la bienvenida. Abre la demo con «Abrir…», ciérrala y vuelve a abrir ArchiSim: la demo sale en Recientes con su planta. Clic derecho → «Quitar de la lista».
2. En la demo, Objeto con una mesa: mueve el ratón por el salón y verás la mesa con sus sillas antes de hacer clic. Prueba Puerta sobre un muro: se ve la hoja y el muro cortado.
3. Selecciona la cama y arrástrala: se ve en su sitio nuevo y desaparece del antiguo; Esc cancela.

---

## 2026-10-22 (6) · Revisión de Antonio, bloque 2: Gráficos y vistas

Rama `bloque/graficos`, pull request para Antonio. v0.42. Formato `.archisim` versión 27 (abre todas las anteriores). Sin cambios en el IFC salvo que los planos de referencia no se exportan. Descrito en `docs/GRAFICOS.md`.

**Hecho**
- **Visibilidad y gráficos** por vista, como en Revit: por categoría, visible, color de líneas, relleno de corte, relleno de superficie, transparencia, medios tonos, tramas, grosor y, en muros, qué líneas de capas se ven (todas, solo la cámara —aislamiento y cámara juntos— o solo el contorno). Se imprime igual en PDF y DXF.
- **Plantillas de vista**: AD5 (blanco con líneas negras, muros con solo la cámara, sin tramas, jardín y urbanismo en pastel), Albañilería y Superficies; se guardan plantillas nuevas desde la ventana VG.
- **Vistas guardadas** de cada planta (nombre, escala, plano de corte y gráficos) en el navegador, arrastrables a los planos. La vista de cada planta recuerda ya su aspecto en el proyecto.
- **Plantas de techo**: el forjado de encima, luminarias, instalaciones y vigas; sin mobiliario ni pavimentos.
- **Planos de referencia** (Estructura o Documentación → Plano ref., tecla Q): verdes, con nombre, en todas las plantas; los muros, líneas y cotas se fuerzan a ellos.
- **Tipografía**: IBM Plex Sans (libre) por defecto en textos, cotas, locales, etiquetas, cajetines y tablas, como contornos (igual en pantalla, PDF y DXF). La de trazo sigue en Normas gráficas.
- **Unidades**: metros por defecto (también centímetros y milímetros) en Vista → Unidades; se escribe en la unidad elegida o con sufijo (`270cm`).
- **Líneas finas / grosor real** en la barra inferior.

**Pendiente**
- Gráficos por categoría en secciones, alzados y 3D; sustituciones por elemento y por filtro; falsos techos como elementos.

**Qué debe probar Antonio**
1. Abre la demo. En el navegador, bajo Planta baja, abre «Planta baja — AD5»: muros blancos con solo la cámara, sin tramas, jardín en pastel. Compárala con «— albañilería», «— superficies» y «— techos».
2. En tu planta, sin nada seleccionado, Propiedades → «Visibilidad y gráficos…»: cambia el relleno de corte de los muros, la transparencia de la vegetación o apaga las tramas; luego «Plantilla» → AD5.
3. Clic derecho en una planta del navegador → «Planta de techos». Arrástrala a un plano.
4. Estructura → Plano ref. (Q): dos clics; dibuja un muro y comprueba que se fuerza a él.
5. Vista → Unidades: metros; escribe «2,70» en una cota temporal y en Propiedades. Prueba el botón de grosor real de la barra inferior.
6. Mira los textos y cotas con la letra nueva; si prefieres la de antes, Normas gráficas → Tipografía.

---

## 2026-10-22 (5) · Revisión de Antonio, bloque 1: Urbanismo

Directo a `main` (arreglos pequeños). v0.41.1. Sin cambios en el formato `.archisim` ni en el IFC. Antonio entregó su lista de feedback acumulado; queda repartida en siete bloques en `docs/HOJA_DE_RUTA.md` §7.

**Hecho**
- Al seleccionar un árbol (en planta o en 3D), cambiar su tipo abre la galería en su carpeta y solo con vegetación, personas y vehículos; un banco o una farola, solo con mobiliario urbano. Antes salía todo el mobiliario.
- **Vista previa de la calle** mientras se dibuja: calzada, aparcamiento, bordillos y aceras pintados con su ancho real.
- **Referencia del vial**: eje, borde izquierdo o borde derecho de las aceras, en la barra bajo las pestañas («Referencia», tres dibujos). Dibujando por el borde de la parcela, la calle queda hacia fuera.
- **Referencia de los muros** en la misma barra (cara exterior, eje, cara interior), además de en Propiedades.
- **Editar un vial después**: con el vial seleccionado, sus puntos del eje se arrastran; doble clic en el vial añade un punto; clic en un punto y Supr lo quita. La sección y los materiales ya se cambiaban en Propiedades.
- Con letra Grande, el buscador de la barra superior ya no tapa deshacer y rehacer.

**Qué debe probar Antonio**
1. Urbanismo → Vial. En la barra bajo las pestañas, «Referencia»: elige el primer dibujo (borde izquierdo) y dibuja de izquierda a derecha: la calle queda debajo de la línea, con sus bandas pintadas mientras mueves el ratón.
2. Selecciona la calle: arrastra un punto del eje; doble clic en medio del vial para añadir otro; clic en él y Supr.
3. Selecciona un árbol y pulsa su tipo en Propiedades: la galería abre en los árboles. Prueba también en la vista 3D.

---

## 2026-10-22 (4) · Letra grande y misiones de iniciación (I6)

Rama `bloque/iniciacion` (sobre `bloque/biblioteca-3d`), pull request para Antonio. v0.41. Sin cambios en el formato `.archisim` ni en el IFC. Descrito en `docs/INICIACION.md`. Con esto la interfaz por disciplinas (I1–I6) queda completa.

**Hecho**
- **Tamaño de letra** Normal, Grande (120 %) y Muy grande (140 %), en Vista → Tamaño de letra: crece toda la ventana a la vez (fichas, barra contextual, Propiedades, navegador, diálogos) sin que nada se corte.
- **Preferencias recordadas** en este ordenador: tamaño de letra, tema claro u oscuro y teclas de AutoCAD (antes se perdían al cerrar).
- **Misiones de iniciación**: la primera vez, una tarjeta ofrece empezarlas (y elegir el tamaño de letra); después, en Ayuda → Misiones de iniciación. Seis: tu primer muro, puertas y ventanas, tu primer forjado, míralo en 3D, tu primera planta y tu primer plano. Cada paso rodea en azul el control que hay que usar y se marca solo al hacerlo (también por otro camino: tecla, buscador). Saltar paso, dejar la misión, siguiente misión; las hechas quedan con ✓.
- Corregido de paso: con la ventana Plantas abierta y Propiedades mostrando la planta, los campos de nombre y cota de los dos sitios se confundían (mismo identificador interno).

**Pendiente**
- Más misiones (secciones y alzados, tipos y materiales, cotas, Concepto, trabajo en equipo) según lo que Antonio vea que cuesta encontrar.
- Recordar también la disposición de las paletas y la última carpeta.

**Qué debe probar Antonio**
1. Para ver la bienvenida como alguien nuevo, borra el fichero de preferencias (en Mac: `~/Library/Application Support/ArchiSim/preferencias.json`; en Windows: `%APPDATA%\ArchiSim\preferencias.json`) y abre ArchiSim. Elige «Grande» en la tarjeta y mira que todo crece; vuelve a «Normal» en Vista → Tamaño de letra.
2. Pulsa Empezar y haz la primera misión siguiendo los anillos azules; prueba en la segunda a pulsar la tecla P en vez de la ficha.
3. Cierra y vuelve a abrir: el tamaño y el tema oscuro (si lo pusiste) se mantienen y la bienvenida no vuelve a salir. En Ayuda → Misiones de iniciación, mira las ✓.
4. Dime qué otras misiones pondrías y si los textos se entienden a la primera.

---

## 2026-10-22 (3) · Biblioteca básica de modelos 3D (L3)

Rama `bloque/biblioteca-3d` (sobre `bloque/modelado`), pull request para Antonio. v0.40. Sin cambios en el formato `.archisim`. Descrito en `docs/BIBLIOTECA_3D.md`.

**Hecho**
- **El 3D de los muebles lo hace ArchiSim desde sus medidas**, como los árboles: sofás, sillas, mesas (con sus sillas), mesas de centro, camas con almohadas y edredón, mesitas con lámpara, armario, estantería con libros, mueble de TV, muebles de cocina con fregadero, grifo y placa, muebles altos, frigorífico, lavadora, inodoro y bidé (de pie y suspendidos), lavabo (pedestal, mueble, suspendido), bañera, duchas con mampara, personas (hombre, mujer, caminando) y coches (berlina, compacto, todoterreno). Nada descargado ni con licencia de nadie.
- Cada familia tiene en Propiedades un grupo **Modelo 3D**: el estilo y un material por pieza (tapicería, cojines, patas, encimera, cromados, vidrio…). Cambiar ancho, plazas o número de sillas rehace el modelo.
- Tres familias nuevas (Mesa de centro, Mesita de noche, Mueble TV), 19 tipos nuevos en la plantilla y diez materiales (Tela, Edredón, Cromado, Encimera…).
- IFC: cada tipo lleva su modelo una sola vez (mapa de representación con color por pieza) y los ejemplares lo comparten.
- Crate nuevo `archisim-props`. `archisim modelos out/` coloca los 41 objetos en filas y escribe proyecto, IFC, axonometría y planta.

**Pendiente**
- Más objetos (encastres, campana, luminarias, oficina, plantas de interior).
- Biblioteca ampliada descargable con modelos CC0 importados.
- Retocar proporciones de lo que Antonio vea raro.

**Qué debe probar Antonio**
1. `archisim modelos out/` y abre `out/modelos.archisim`; en 3D recorre las filas (salón, comedor, dormitorio, cocina, baño, personas y coches). Mira también `out/modelos-3d.svg`.
2. Selecciona el «Sofá de 3 plazas», en Propiedades cambia el ancho a 3000 y las plazas a 4: deben salir cuatro cojines. Cambia «Modelo 3D» a Clásico y luego a Nórdico.
3. En la «Mesa de comedor 160×90», sube las sillas a 8 y mira cómo se reparten.
4. Cambia el material «Tapicería» del sofá a otro de la lista y míralo en 3D.
5. Exporta el IFC y ábrelo en tu visor: cada mueble con sus colores.

---

## 2026-10-22 (2) · Formas de Concepto y materiales de visualización (M1–M3)

Rama `bloque/modelado` (sobre `bloque/equipo`), pull request para Antonio. Formato `.archisim` versión 26 (abre las anteriores tal cual). v0.39. Diseño en `docs/FORMAS.md`.

**Hecho**
- **Formas** (Concepto → Formas): esfera, cilindro, cono o tronco de cono, y **revolución** de un medio perfil dibujado abatido sobre la planta. Radios y altura en Propiedades.
- **Aristas achaflanadas o redondeadas** (Propiedades → Aristas) en esquinas, coronación y base de los volúmenes por contorno, y arriba y abajo en las formas redondas. No destructivas: siguen al volumen.
- **Unir, restar e intersecar** volúmenes (varios seleccionados → Operaciones con volúmenes), con cada cara conservando su material: el corte de un cilindro de hormigón en un bloque de ladrillo queda de hormigón. **Intersecar caras** dibuja aristas sueltas donde se cruzan.
- **Materiales de visualización**: cada material tiene un aspecto en 3D (color de superficie y textura: ladrillo, piedra, baldosa, madera, tablas, hormigón visto, enfoscado, césped, grava, agua, chapa) con el tamaño de su pieza. La vista 3D los dibuja en cada cara sin imágenes. Se pinta cara a cara con Pintar, también las caras de esferas, revoluciones y sólidos.
- La plantilla da su aspecto a los materiales que se ven y trae cuatro acabados nuevos: Ladrillo visto, Piedra, Tarima de madera y Césped.
- Los volúmenes se dibujan en planta por su huella real (con huecos). `archisim formas out/` junta todas las formas, operaciones y materiales en `formas.archisim`, con planta y axonometría.

**Pendiente**
- Texturas en las axonometrías de los planos, en los PDF y en el IFC.
- Editar un sólido ya operado (hoy, deshacer y repetir) y redondear sus aristas.
- Pintar zonas de una cara de volumen limitadas por aristas.

**Qué debe probar Antonio**
1. `archisim formas out/` y abrir `out/formas.archisim`. En 3D, mira el ladrillo del bloque, también en el redondeo de las esquinas; el corte del patio cilíndrico en gris hormigón; la esfera de piedra sobre la peana achaflanada; y el jarrón con la barriga de madera.
2. En un proyecto nuevo: Concepto → Formas → Cilindro, dos clics; en Propiedades → Aristas, «Redondeo», 300 mm, Arriba. Haz un bloque con Volumen que lo atraviese, selecciona los dos con Mayús y prueba Unir, Restar e Intersecar (y Deshacer entre medias).
3. Revolución: clic en un punto (el eje) y dibuja hacia la derecha y hacia arriba el medio perfil de una columna con basa y capitel; clic otra vez en el último punto.
4. Tipos y materiales → un material → Aspecto en 3D: cambia la textura y el tamaño de la pieza y míralo en 3D. Pinta una sola cara de un volumen con Pintar.

---

## 2026-10-22 · Trabajo en equipo (4.6)

Rama `bloque/equipo`, pull request para Antonio. v0.38. Sin cambios en el formato `.archisim` ni en el IFC. Diseño en `docs/EQUIPO.md`.

**Hecho**
- **Proyecto compartido en una carpeta** (Google Drive, Dropbox o un NAS; D20): Colaboración → **Compartir en equipo…** y → **Abrir proyecto de equipo…** (o doble clic en `equipo.asequipo`). Cada persona trabaja en su copia, también sin conexión, y solo escribe sus propios ficheros: Drive nunca tiene que mezclar dos versiones del mismo.
- **Publicar y Recibir** en la barra superior, con el número de elementos por publicar y de cambios por recibir. Los cambios se guardan elemento a elemento y se aplican en el mismo orden en todos los ordenadores (reloj lógico, no la hora de cada uno). Lo que ya no encaja (una puerta en un muro que otro borró) se deja fuera con un aviso, igual para todos.
- **Reservas por elemento y por zona** (D21): reservar la selección, dibujar una zona (dos esquinas) o la planta entera. Lo que se cambia sin que nadie lo tenga se reserva solo; lo que se dibuja nuevo es de quien lo dibuja. Lo que tiene otro no se deja cambiar («reservado por Ana: pídeselo»). Si dos reservan a la vez, se lo queda el primero.
- **Peticiones y mensajes**: «Pedir» en Propiedades cuando lo tiene otro; a él le llega el mensaje con «Liberar».
- **Presencia**: un círculo por persona con sus iniciales y su color en la barra superior (apagado si no está). En la planta, lo reservado se ve del color de su dueño y las zonas con su nombre. Cursor de los compañeros opcional y apagado (D22).
- **Ventana Equipo**: personas y su planta, publicar con nota, reservas, mensajes, historial y opciones. En Propiedades, de quién es cada elemento y quién lo cambió por última vez.
- **Copia de trabajo** en la carpeta (`trabajo/<nombre>.archisim`): Guardar la escribe, y el programa la guarda solo cada minuto y al cerrar; al volver a abrir, sigue lo no publicado. Si la misma persona lo tiene abierto en dos ordenadores, avisa.
- **Identificadores**: cada persona numera sus elementos nuevos en un bloque propio, así que dos personas dibujando a la vez nunca chocan. El `GlobalId` de cada elemento no cambia. Las copias de módulos se numeran por encima de todo lo usado.
- Crate nuevo `archisim-team`. `archisim equipo prueba out/` comparte la casa de la demo entre Antonio y Ana (reservas, una zona, dos publicaciones y una petición) y escribe `out/equipo/` y `equipo-recibido.archisim` e `.ifc`; `archisim equipo <carpeta>` dice quién está y qué ha hecho cada uno.

**Pendiente**
- Servidor propio para la presencia al momento (con Drive, unos segundos de retraso).
- Compactar `cambios/` (crece con cada publicación; solo guarda lo cambiado).
- Recibir solo sin pulsar; reservar tipos desde el diálogo de tipos (hoy se reservan al cambiarlos).
- Medir con modelos muy grandes: cada cambio compara el proyecto entero para saber qué se tocó.

**Qué debe probar Antonio**
1. En un solo ordenador: `archisim equipo prueba out/`, y en la aplicación Colaboración → Abrir proyecto de equipo… → `out/equipo/equipo.asequipo`, como «Antonio Fernández». Mira los círculos de Antonio y Ana arriba, los tres muros reservados de tu color, la zona «Jardín» de Ana en azul y su mensaje en la ventana Equipo (pulsa «Liberar»).
2. Intenta mover el muro del huerto de Ana: no debe dejarte, y debe decir que es suyo.
3. Con dos ordenadores (el PC y el MacBook) y una carpeta de Google Drive: comparte un proyecto desde uno y ábrelo desde el otro con otro nombre. Dibuja en los dos a la vez, publica en uno y mira cómo «Recibir» se ilumina en el otro a los pocos segundos.
4. Cierra sin publicar, vuelve a abrir: lo no publicado debe seguir ahí.

---

## 2026-10-21 (noche, 3) · Vegetación generada: vivero y plantaciones

Rama `bloque/vivero` (sobre `bloque/mallas`, #42), pull request para Antonio. Formato `.archisim` versión 25 (abre las anteriores tal cual). v0.37. Diseño en `docs/VEGETACION.md`.

**Hecho**
- **Los árboles crecen** (crate nuevo `archisim-flora`, sin dependencias, sin código de nadie): de una especie y una semilla salen el tronco o troncos (inclinados y retorcidos), las ramas por órdenes dentro de la forma de la copa, las matas de hojas, flores y frutos; en las palmeras, el estípite con anillos y las frondas pinnadas o en abanico; en gramíneas, pitas y yucas, sus hojas en roseta. La misma semilla da siempre la misma planta; cada planta del proyecto, la suya (sacada de su `GlobalId`: una copia es otra planta).
- **Familia de sistema «Vegetación»**: cada especie es un tipo con altura, ancho, tronco, fuste, porte (árbol, conífera, palmera, arbusto, mata, gramínea, roseta), forma de copa (11), troncos, inclinación, tortuosidad, ramas, ángulo, caída, follaje (9), densidad, matas, frondas, floración, fruto y sus colores. Cada planta, su semilla, su alto y ancho propios y su variación.
- **3D** en tres niveles de detalle (entera / ligera / muy ligera) según cuántas plantas tenga el proyecto; las hojas, redondeadas y sin aristas entre sus caras. **IFC**: cada planta con su malla (`IfcTriangulatedFaceSet` con colores) y sus valores, que vuelven idénticos al importar.
- **Planta** en 13 estilos sacados de la propia planta (círculo, contorno, dentado, nube, a mano, ramas, hojas, ramas y hojas, follaje, punteado, radial, frondas, acuarela), en líneas, transparente, rellena o solo color, con flores y frutos de su color; se simplifica sola a escalas pequeñas.
- **Biblioteca**: 64 especies integradas (pino piñonero, olivo, acebuche, encina, alcornoque, eucalipto, castaño, jacaranda, ciprés, naranjo, almez, palmeras, romero, lentisco, retamas, almajo, salicornia… y de clima templado y cálido), en la galería de Vegetación (Entorno › su grupo). Vegetación empieza ahora con un olivo.
- **Vivero** (Urbanismo, o «Vivero…» en una planta): especies del proyecto, todos sus parámetros con deslizadores, la planta en 3D y en planta técnica y de presentación al momento, «🎲 Otro ejemplar», variación, duplicar especie y traer de la biblioteca.
- **Plantación** (Urbanismo, tecla J): por clics, Spline o Elegir línea; a lo largo de la línea o dentro del contorno; número de plantas, especies mezcladas, variación de tamaño y de forma, irregularidad y azar. Se ven las plantas antes de plantar; luego, la plantación se cambia en Propiedades y sus plantas se rehacen; moverla, copiarla o borrarla lleva sus plantas. Su línea se ve verde discontinua en pantalla (no se imprime).
- **Tirador** en el borde de la copa de una planta seleccionada: su ancho; con Ctrl, alto y ancho en proporción. En sus Propiedades, «🎲 Otra planta».
- La demo tiene olivo, encina, almendro en flor, palmera datilera, romero, lavanda y una hilera de 8 cipreses (plantación). `archisim vivero out/` planta todas las especies (y un paseo y un arriate) en `vivero.archisim`, `vivero.ifc` y plantas 1:200 de presentación y técnica.

**Pendiente**
- Tiradores en 3D (alto y ancho arrastrando en la vista 3D).
- En secciones y alzados, las palmeras salen como trazos sueltos y las copas sin contorno.
- Hojas con textura (M3): hoy las matas son cuerpos facetados de color, redondeados al sombrear.
- Setos como masa continua en planta; estaciones (caducas en invierno, otoño).
- Una planta movida a mano dentro de una plantación vuelve a su sitio al cambiar la plantación.
- Las medidas y colores de las 64 especies son una propuesta: Antonio las corrige en el Vivero.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- demo out/` y `cargo run --release -p archisim-app -- out/demo.archisim`: en la planta, la hilera de cipreses, el olivo, la encina, el almendro en flor y la palmera; F3 para verlos en 3D.
2. Urbanismo → **Vivero**: elige el Olivo y mueve **Tortuosidad**, **Inclinación**, **Forma de la copa** y **Densidad**: el 3D y las dos plantas cambian al momento, y también los olivos del proyecto. Pulsa **🎲 Otro ejemplar** varias veces. Trae de la biblioteca el **Pino piñonero** y la **Palmera canaria**.
3. Urbanismo → **Vegetación**: abre la galería (Entorno › Árboles mediterráneos, Palmeras, Arbustos…) y coloca varias especies. Selecciona una: arrastra el **tirador** del borde de la copa (con Ctrl, también crece en altura) y pulsa **🎲 Otra planta**.
4. Urbanismo → **Plantación**: tres clics en curva y doble clic (o clic derecho): se plantan 8 olivos. En Propiedades, cambia el **Número de plantas**, la **Irregularidad**, la **Variación de tamaño** y pulsa **🎲 Otro**. Elige otra especie en la galería con **Mayús** para mezclarla.
5. Plantación con **Distribución: Dentro del contorno** y cuatro clics: un bosquete. Y otra con el método **Spline** de la barra de arriba.
6. Propiedades de una planta → «Este ejemplar» y «Dibujo»: prueba los estilos (nube, follaje, ramas, punteado, acuarela) y los colores; y en la vista, Presentación → Bosque.
7. Exporta el IFC y ábrelo en ArchiCAD o Revit: cada árbol con su forma y sus colores (`IfcGeographicElement`).
8. `cargo run -p archisim-cli -- vivero out/` y abre `out/vivero-presentacion.svg`: todas las especies, para corregir medidas y colores.

---

## 2026-10-21 (noche, 2) · Modelos 3D (L1–L2), vértices y árboles con su copa

Rama `bloque/mallas` (sobre `main` con #41), pull request para Antonio. Formato `.archisim` versión 24 (abre las anteriores tal cual). v0.36.

**Hecho**
- **Árboles con su copa por defecto**: los árboles de la biblioteca se dibujan con su copa (forma, relleno, opacidad, color) en cualquier vista, también en las técnicas, así que lo que se cambia se ve enseguida. En el tipo, «Dibujo en planta»: presentación (por defecto), según el estilo o técnico; en cada árbol, «del tipo» o uno propio.
- **Vértices como en ArchiCAD**: con una línea, polilínea, relleno o forjado seleccionado, **doble clic en una arista** añade un vértice allí, que se arrastra como cualquier otro. Un **clic en un vértice** lo elige (se ve con un círculo azul) y **Supr** quita solo ese vértice, no el elemento. En arcos, el vértice parte el arco en dos de la misma circunferencia.
- **Importar modelos 3D** (L1): Archivo → **Importar modelo 3D (OBJ, glTF, DAE)…**, o soltar el fichero en la ventana. Lee:
  - **COLLADA (.dae)**, lo que exporta SketchUp (también el gratuito): componentes, pulgadas y eje Z;
  - **OBJ** con su **MTL** y texturas;
  - **glTF** y **GLB** (Blender, catálogos).
  El diálogo dice medidas, caras y materiales, y pide nombre, qué es (mobiliario, cocina, sanitarios, vegetación, mobiliario urbano, personas, vehículos), unidades y eje vertical. Pasa a ser un **tipo de objeto**: su 3D es el del modelo con sus materiales (uno del proyecto por cada uno) y los colores de sus texturas; su planta, el **contorno visto desde arriba**; la altura del tipo lo escala entero. Los muebles se colocan por el centro de su espalda; árboles, personas y mobiliario urbano, por el centro de su base.
- **Niveles de detalle** (L2): cada modelo guarda tres versiones (el modelo, unas 4.000 caras y unas 600). Con muchos modelos en el proyecto (más de 400.000 caras), todas las vistas toman la más ligera que cabe, para que un plan con cientos de árboles siga yendo fluido. En Propiedades → «Modelo 3D»: fichero, caras de cada versión y la que se está viendo.
- **IFC**: el modelo va una sola vez en el tipo (`IfcRepresentationMap` con un `IfcTriangulatedFaceSet` por material, con su color y el de cada cara) y cada objeto lo coloca; con la clase de lo que es (`IfcFurniture`, `IfcGeographicElement` VEGETATION, `IfcVehicle`…). Valida en IFC 4.3 e IFC4.
- **Biblioteca**: los `.obj`, `.gltf`, `.glb` y `.dae` de la carpeta de la biblioteca salen en la galería de objetos (por la carpeta en la que están: «Vegetación», «Cocina»…) y se cargan al elegirlos. «Guardar en la biblioteca» de un tipo con modelo lo guarda como `.glb` en `Modelos 3D/<qué es>/`, que también abre cualquier otro programa.
- La demo lleva un árbol importado de un fichero de SketchUp de prueba (junto a la entrada) y sale en la planta, el 3D, la lámina A-03 y el IFC.

**Pendiente**
- Las texturas se ven como colores en los vértices (en 3D no hay todavía texturas de verdad: va con M3).
- Un modelo como volumen de Concepto (para encajes de SketchUp) no está aún.
- Al reimportar el IFC, un objeto con modelo 3D vuelve con la caja de su familia (el modelo no viaja de vuelta).
- Los modelos dentro de módulos (I5) no se copian.
- La carpeta de la biblioteca con muchos modelos pesados tarda la primera vez que se abre la galería (luego se recuerdan).
- El `.skp` de SketchUp no se puede leer (formato cerrado): hay que exportar a DAE.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- demo out/` y `cargo run --release -p archisim-app -- out/demo.archisim`.
2. En la planta, los árboles se ven con su copa aunque la vista sea técnica. Seleccione uno y cambie «Copa en presentación» del tipo: se ve al momento. Pruebe «Dibujo en planta: según el estilo» y cambie el estilo de la vista.
3. Vista 3D (F3): delante de la entrada está el árbol importado de SketchUp (copa redonda y tronco).
4. Exporte desde SketchUp un árbol, una silla y un coche como **COLLADA (.dae)** (Archivo → Exportar → Modelo 3D) y llévelos a ArchiSim con Archivo → Importar modelo 3D. Mire medidas, unidades y eje en el diálogo, impórtelos y colóquelos. Mire el 3D, la planta (técnica y en estilo Bosque) y Propiedades → «Modelo 3D». Si tiene modelos en OBJ o glTF de catálogos, pruébelos también.
5. Ponga algunos de esos ficheros en la carpeta de la biblioteca, dentro de una carpeta «Vegetación» o «Cocina», y abra la galería de Objetos o de Vegetación: deben salir con su planta.
6. Exporte el IFC y ábralo en otro visor: los modelos deben salir con sus colores.
7. Dibuje un relleno rectangular, selecciónelo, haga **doble clic en una arista**: arrastre el vértice nuevo. Haga clic en ese vértice y pulse **Supr**: vuelve el rectángulo. Pruebe lo mismo con una polilínea y con un forjado.

---

## 2026-10-21 (noche) · Vegetación, mobiliario urbano y rellenos

Rama `bloque/vegetacion` (sobre `main` con #39 y #40), pull request para Antonio. Formato `.archisim` versión 23 (abre las anteriores tal cual).

**Hecho**
- **Vegetación** (Urbanismo) abre directamente los árboles, sin pasar por los muebles; la galería solo enseña Entorno, Gente y Vehículos, y sus miniaturas dibujan cada copa como en presentación, así que se distingue cada especie a simple vista.
- **Mobiliario urbano** (Urbanismo, herramienta nueva): farola de báculo, farola de vial, baliza de jardín, buzón, papelera, bolardo, fuente de beber, aparcabicis, contenedor de residuos, señal de tráfico, jardinera, alcorque y banco; cada uno con planta técnica (1:50) y de presentación (la farola, con su círculo de luz). IFC: `IfcFurniture`.
- **Dibujo de cada árbol**: en el tipo, «Copa en presentación» con siete formas (lóbulos, círculo, acuarela, ramas, follaje, palmera, conífera), relleno (transparente, rellena, contorno) y opacidad. En el **propio árbol** (Propiedades → «Este objeto»): «Dibujo en planta» según el estilo, siempre técnico o siempre de presentación; y forma, relleno, opacidad y color propios. Así se controla el dibujo árbol a árbol sin depender del estilo de la vista.
- **Biblioteca ampliada**: 83 tipos (38 árboles —plátano, encina, alcornoque, olivo, jacaranda, ciprés, palmera datilera…—, 14 arbustos y setos, y farolas, personas y vehículos variados) con `GlobalId` fijos. Biblioteca → «Instalar la biblioteca ampliada» los copia a la carpeta de la biblioteca, en `Ampliada/`; desde ese momento salen en las galerías de Vegetación y Mobiliario urbano y se cargan al elegirlos. También `archisim biblioteca --ampliada [carpeta]`, y en el repositorio en `biblioteca-ampliada/`.
- **Rellenos directos**: el segmento **Relleno** de la barra inferior tiene las mismas variantes que Línea: rectángulo, polígono, spline, polígono regular, círculo y elipse (órdenes RELLRECT, RELLPOLIG, RELLSPLINE, RELLREGULAR, RELLCIRC, RELLELIPSE), y «Por recinto o contorno» (SB) como antes. Se dibuja y queda relleno, sin contorno previo.
- **Trama a la vista**: al elegir la trama (barra inferior, propiedades del sombreado y pavimento de los locales) se ve su dibujo junto al nombre, y al pulsar, una cuadrícula con todas, técnicas y de presentación.
- **Opacidad del relleno**: deslizador de 5 a 100 % en la barra inferior y en las propiedades del sombreado; sale en pantalla, SVG y PDF.

**Pendiente**
- Los árboles nuevos son recetas (planta y 3D sencillo de cajas); los modelos 3D de malla bonitos llegan con L1–L2 (importar glTF/OBJ y niveles de detalle).
- En el DXF los rellenos semitransparentes no salen (el DXF no tiene transparencia de sombreado de forma fiable): solo su contorno.
- El deslizador de opacidad en Propiedades es corto (la paleta es estrecha); el número se puede teclear.
- Las especies y tamaños de la biblioteca ampliada son una propuesta: Antonio puede corregir nombres, diámetros y colores.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-app -- out/demo.archisim` (antes, `cargo run -p archisim-cli -- demo out/`).
2. Urbanismo → **Vegetación**: debe salir directamente un árbol, y al pulsar su nombre, la galería de árboles con cada copa dibujada distinta. Coloque un par.
3. Seleccione un árbol → Propiedades → **Este objeto** → «Dibujo en planta: presentación»: ese árbol se dibuja relleno aunque la vista sea técnica. Pruebe forma (acuarela, palmera…), opacidad y color propio. Más abajo, en el tipo, «Copa en presentación» cambia todos los de ese tipo.
4. Urbanismo → **Mobiliario urbano**: farolas, bancos, papeleras, bolardos… Póngalos junto al vial y mírelos también en un estilo de presentación (Propiedades de la vista → Presentación → Bosque).
5. Vista → Biblioteca… → **Instalar la biblioteca ampliada**; vuelva a Vegetación: en la galería aparecen las especies nuevas (encina, plátano, jacaranda…). Elija una y colóquela.
6. Barra inferior → flecha de **Relleno** → Trama: elija «Tarima» viendo la cuadrícula, baje la opacidad a 40 % y pulse **Rectángulo**; dos clics en la planta y queda el relleno hecho. Pruebe Círculo y Polígono.
7. Seleccione el relleno: en Propiedades → Sombreado se ve la trama con su dibujo y la opacidad; cámbielas.
8. Exporte la planta a PDF y mire que el relleno semitransparente y los árboles salen igual que en pantalla.

---

## 2026-10-21 (tarde) · Bloque P: presentación (P1–P4)

Rama `bloque/presentacion` (sobre `bloque/revision-r`), pull request #40 para Antonio. Formato `.archisim` versión 22 (abre las anteriores tal cual).

**Hecho**
- **P1, estilos de presentación** (Vista → Estilos de presentación…, o Propiedades de la vista → Presentación). Cada estilo tiene:
  - papel de color;
  - paleta de tonos de oscuro a claro (cada color del dibujo pasa al tono de su claridad; sin paleta, los colores propios);
  - muros y pilares cortados macizados en un color;
  - grueso de las líneas;
  - sombras.
  - De partida: **AD5** (líneas finas negras, muros negros, colores propios), **Terracota**, **Bosque**, **Azul** y **Monocromo**; «Estilos de partida» los añade a proyectos antiguos. Biblioteca del estudio: «Guardar / Cargar de la biblioteca» (`Estilos.aspres`).
  - **Tramas nuevas** en sombreados y suelos: adoquín, losa, piedra irregular (cada piedra distinta), tarima, veta de madera, grava punteada, césped en franjas, césped punteado y lámina de agua.
  - **Pavimento en planta** de cada local (Propiedades del local). Los sombreados 2D se dibujan debajo de muros, objetos y árboles.
- **P2, vegetación y figuras**: en las vistas con estilo, los árboles se dibujan como copas de lóbulos irregulares, cada uno distinto, transparentes, rellenas o solo contorno (tipo → «Copa en presentación») y del color de su especie. También arbustos, setos, personas y coches rellenos. Especies nuevas en la biblioteca: árbol en flor, otoñal y pino. El PDF dibuja las transparencias.
- **P3, sombras**: el sol del proyecto (Propiedades del proyecto, sin nada seleccionado: «Sol: día» y «Sol: hora solar»), desde la latitud de la georreferencia y el norte. Los estilos con sombras las dibujan sobre el suelo de la planta (no dentro de los forjados) y sobre el terreno de las axonometrías de los planos. Las curvas de nivel ya estaban y toman la paleta.
- **P4, láminas**: **+ Plano de presentación** en el Navegador (Planos) hace una lámina A3 de la planta activa:
  - título del proyecto con emplazamiento, fase y fecha;
  - la planta, la vista 3D y una sección a las escalas que caben;
  - el cuadro de superficies útiles;
  - el recorrido solar con el norte (solsticios, equinoccio, horas y el sol de las sombras);
  - todo en el estilo de la vista. Cada plano puede tener su propio estilo (Propiedades del plano → Presentación: papel, cajetín, tablas y textos). **Exportar plano → PNG** (200 ppp).
- La demo tiene jardín (césped, piscina, grava, árbol en flor y pino), suelos de tarima y gres, y la lámina **A-03 Presentación** en estilo Bosque, también en `out/demo-plano-a-03.png`.

**Pendiente**
- En el DXF no hay transparencias: las sombras no salen y las copas solo con su contorno.
- La paleta cambia todos los colores por su claridad (el agua pasa al tono de su claridad, no queda azul); si hace falta, se puede añadir que ciertos colores se respeten.
- Las sombras de los árboles salen escalonadas (su 3D son cajas), y solo caen sobre el suelo, no sobre fachadas ni cubiertas. Mejorará con la librería 3D (L1–L3).
- Secciones con fondo de vegetación y diagramas de concurso más elaborados, si Antonio los quiere.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- demo out/`: abre `out/demo-planos.pdf` (tercera hoja) y `out/demo-plano-a-03.png`.
2. `cargo run --release -p archisim-app -- out/demo.archisim`. Con la planta baja y nada seleccionado, en Propiedades → **Presentación → Estilo** elige AD5, Terracota o Bosque: muros macizados, copas de colores y sombras.
3. En las mismas propiedades del proyecto, cambia **Sol: día** (21/12) y **Sol: hora solar** (16:00): las sombras se alargan y giran.
4. **Vista → Estilos de presentación…**: crea uno, cámbiale papel, tonos, macizado, sombras y grueso; **Usar en la vista activa**.
5. Selecciona un árbol y, en su tipo, «Copa en presentación»: transparente, rellena o contorno.
6. Selecciona un local y, en **Pavimento en planta**, prueba Tarima, Piedra irregular o Losa.
7. Navegador → Planos → **+ Plano de presentación**; después Archivo → Exportar plano en **PNG**.

---

## 2026-10-21 (tarde) · Bloque R, segunda vuelta (R17–R21)

Misma rama `bloque/revision-r` y mismo pull request (#39). El formato sigue en la versión 21 (aún no estaba en `main`): las caras pintadas guardan ahora el elemento y la cara, y hay zonas pintadas y aristas con altura por punto.

**Hecho**
- **R17**: la **Spline** de la Línea (dibujo 2D) curva como la de Arista: el mismo trazado, que pasa por los puntos con curvas suaves.
- **R18, Pintar**: la pintura se ve siempre con su color (antes la atenuación de la disciplina la apagaba, y por eso apenas cambiaba). En Propiedades de Pintar hay ahora **Pintar / Quitar pintura**: quitar deja a la vista el material del elemento. En planta se pinta el lado del muro donde se pincha; en 3D, la cara que se ve.
- **R19**: en un volumen se pinta **solo la cara** pinchada (la de arriba, la de abajo o un lado). Para el volumen entero, su material en Propiedades.
- **R20, aristas en fachada**: en 3D, Arista se apoya en las caras: con **Mayús** el segundo punto sube en vertical. Una arista dibujada de abajo arriba en una fachada la divide, y **Pintar** pinta solo la zona entre esas aristas (y los bordes del muro). Así se pinta una fachada con dos o más materiales.
- **R21, vista previa en 3D**: antes del clic se ve, transparente y con sus colores, lo que se va a hacer: muro (con sus capas y uniones), puerta y ventana en su muro, pilar, viga, forjado y objeto.

**Pendiente**
- Las zonas pintadas son por cara de muro (no siguen a una arista que cruce dos muros) y las aristas inclinadas no dividen zonas (solo las que van por la cara).
- La vista previa en 3D se dibuja por encima de todo (se ve también si queda detrás de otro elemento).
- Bloque P (presentación), siguiente en su propia rama.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- demo out/` y `cargo run --release -p archisim-app -- out/demo.archisim`.
2. **Línea** de la barra inferior → **Spline**, cuatro clics e Intro: la curva es como la de Arista.
3. **F3**, **Concepto → Pintar** con Ladrillo perforado: clic en la fachada sur. Cambia a ladrillo solo esa cara. En Propiedades, **Quitar pintura** y clic en la misma cara: vuelve el material del muro.
4. Dibuja un volumen (Arista, Rectángulo, y Empujar) y píntale un lado: solo ese lado cambia.
5. En 3D, **Arista**: clic abajo en la fachada, **Mayús** y clic arriba, Intro. **Pintar** con Piedra natural a un lado de la arista: solo esa zona.
6. En 3D, **Muro**: un clic y mueve el ratón: el muro se ve antes del segundo clic. **Ventana**: pasa el ratón por un muro y se ve dónde queda.

---

## 2026-10-21 (mediodía) · Bloque R: la revisión de Antonio (R12–R16)

Rama `bloque/revision-r` (pull request para Antonio). Formato `.archisim` versión 21 (abre las anteriores tal cual).

**Hecho**
- **R12, Arista**: la Línea de Concepto se llama ahora **Arista** y es una línea del modelo, no del dibujo:
  - abierta (Intro, doble clic o clic derecho), queda como arista suelta, dibujada en planta con un punto en cada extremo y en 3D como línea;
  - al cerrar el contorno en su primer punto, se hace una cara;
  - si cierra un contorno con otras aristas que ya están, también se hace la cara, y esas aristas desaparecen dentro de ella (como en SketchUp);
  - se selecciona, mueve, copia, gira y borra como lo demás, y en Propiedades tiene su altura y su longitud.
- **R13**: fuera las fichas Rectángulo y Círculo de Concepto. Son métodos de Arista en la barra superior, y sus huecos quedan libres.
- **R14, la ventana 3D**: las herramientas funcionan también ahí (al pasar de la planta al 3D, la herramienta sigue puesta):
  - el cursor cae sobre lo que hay debajo (la cara de un muro, un volumen, el terreno) o, si no hay nada, sobre el plano de la planta activa;
  - fuerza a las esquinas y puntos medios de las aristas del elemento bajo el cursor, y a los mismos puntos que en planta;
  - se dibujan muros, forjados, locales, volúmenes y aristas, y se colocan pilares, objetos, puertas, ventanas y aparatos, con las mismas teclas (Intro, Esc, longitudes escritas, Espacio para girar);
  - **Empujar**: clic en la cara y sube o baja el ratón (o escribe la altura); se ve el volumen creciendo con su altura;
  - **Pintar**: clic en la cara de un muro y se pinta ese lado (exterior o interior) con el material elegido; también en planta, por el lado del muro donde se pincha; en un volumen, el volumen entero;
  - arrastrar sigue girando la cámara.
- **R15**: las órdenes de dibujo de la barra inferior (Línea, Relleno, Texto, Cota) muestran sus variantes en la barra superior mientras se dibuja (Línea, Polilínea, **Spline**, Rectángulo, Polígono, Arco, Círculo, Elipse), y después el paso de la orden. Con dibujo 2D seleccionado, la barra Modificar de siempre (copiar, mover, desfase, recortar…).
- **R16, Spline**:
  - abierto por defecto en todas las herramientas, y se cierra con un clic en su primer punto;
  - en Concepto ya no sale un círculo ni hace falta pasar por Camino;
  - orden nueva **Spline** (SPL) en el dibujo 2D: una polilínea de arcos que pasa por los puntos (C para cerrarla).
- Al borrar una planta se borran también las cimentaciones, módulos, volúmenes, aristas, parcelas, viales e instalaciones que había en ella (antes se quedaban huérfanos).

**Pendiente**
- En 3D, los objetos se colocan siempre sobre la planta (no encima de una mesa o de otro forjado). Las aristas no se pueden seleccionar en 3D (sí en planta).
- Pintar solo pinta caras de muros y volúmenes enteros; no forjados, cubiertas ni objetos. La pintura no va al IFC: es para ver y presentar.
- Las previsualizaciones en 3D son la línea de lo que se dibuja; la del muro con su grosor solo se ve en planta.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- demo out/` y `cargo run --release -p archisim-app -- out/demo.archisim`.
2. **Concepto → Arista**: tres clics en planta e Intro (queda una arista abierta); después, desde su último punto, dos clics más hasta su primer punto e Intro: se hace la cara.
3. **F3** (3D) con **Arista** puesta, método **Rectángulo**: dos clics sobre el terreno. **Empujar**: clic en la cara, sube el ratón y clic (o escribe 9000 e Intro).
4. En 3D, **Arquitectura → Muro**: dos clics en el suelo. **Puerta**: clic en ese muro.
5. **Concepto → Pintar**, elige en Propiedades un material (Ladrillo) y, en 3D, clic en la cara de un muro: solo ese lado cambia de color.
6. **Línea** de la barra inferior: arriba salen Polilínea, Spline, Rectángulo…; elige **Spline**, cuatro clics e Intro.

---

## 2026-10-21 (mañana) · Unión de los siete bloques y revisión de Antonio

Unidos a `main`, por orden, los pull requests #32 a #38 (I3, ejes y cimentación, módulos, concepto, urbanismo, métodos e instalaciones), todos en verde. Antonio los ha probado: funcionan, con cinco correcciones (R12–R16 en `docs/HOJA_DE_RUTA.md` §6.1): línea de Concepto como arista, fuera las fichas Rectángulo y Círculo de Concepto, herramientas en la ventana 3D, barra contextual para el dibujo 2D de la barra inferior y Spline abierto por defecto (también en el dibujo 2D).

Decididas D23 (librería 3D: modelos de uso libre, básica integrada y ampliada como descarga, varios modelos por objeto) y D24 (estilos de presentación, con láminas de AD5 y de otros estudios como referencia). Bloques nuevos apuntados en §6: P1–P4 (presentación), L1–L3 (librería 3D e importar glTF/OBJ/Collada de SketchUp) y M1–M3 (modelado de formas). Las láminas de referencia no se suben al repositorio (son proyectos reales y trabajos de otros).

**Siguiente**: bloque R (R12–R16), en su rama y pull request.

---

## 2026-10-21 (madrugada) · Instalaciones

Rama `bloque/instalaciones` (sale de la de Spline y Elegir línea; pull request para Antonio). Formato `.archisim` versión 20 (abre las anteriores tal cual, sin instalaciones).

**Hecho** (las fichas de Instalaciones dejan de estar apagadas)
- **Tubería** (tecla H) y **Conducto** (tecla D): clic en cada punto del recorrido; doble clic, Intro o clic derecho terminan. En Propiedades, el **sistema** (agua fría, agua caliente, saneamiento, calefacción; impulsión, retorno, extracción de aire), el **diámetro** o el **ancho y alto** del conducto y la **altura del eje** sobre la planta. Valen Spline, Elegir línea y los campos L y ∠.
- **Aparato**, **Iluminación**, **Enchufe** e **Interruptor**: un clic los coloca, cada uno a su altura habitual (luminaria a 2,60 m, enchufe a 0,30, interruptor a 1,10, equipo a 1,20), que se cambia en Propiedades; Espacio los gira 90°.
- **En planta**: cada sistema con su color (fría azul, caliente roja, saneamiento marrón…); las tuberías gruesas y los conductos, con sus dos caras; los conductos, a trazos. El sistema y el tamaño se rotulan a lo largo del tramo más largo, sin pisar otros rótulos, otras tuberías ni muros. Símbolos: luminaria (círculo con aspa), enchufe, interruptor y equipo (caja con aspa).
- **En 3D**, su volumen con el color del sistema; en Mediciones (y BC3), un capítulo **Instalaciones**: metros por sistema y tamaño, y unidades por aparato.
- Se seleccionan, mueven, copian, giran, cambian de planta y borran como lo demás; son de la disciplina Instalaciones (con ella activa, el resto se ve en gris).
- **IFC**: cada tramo es un `IfcPipeSegment` o `IfcDuctSegment` con su perfil circular o rectangular; los aparatos, `IfcLightFixture`, `IfcOutlet`, `IfcSwitchingDevice` e `IfcUnitaryEquipment`. Todo va en el `IfcDistributionSystem` de su sistema (agua fría, agua caliente, saneamiento, calefacción, ventilación…, más «Alumbrado» y «Fuerza»), que sirve al edificio. En IFC4, las mismas clases. Se lee de vuelta al importar.
- La **demo** lleva agua fría, caliente y saneamiento desde la caldera hasta la cocina, la extracción de la campana, una luminaria por zona, enchufes y un interruptor.

**Pendiente**
- Sin uniones (codos, tes) ni accesorios: cada tramo es recto y se tocan en sus extremos. Sin pendientes en el saneamiento, sin cálculo de caudales ni secciones.
- Los aparatos son cajas con su clase IFC; aún no hay familias de aparatos (sanitarios conectados, radiadores, rejillas y difusores).
- En sección y alzado no se dibujan todavía.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-app -- out/demo.archisim` (antes `cargo run -p archisim-cli -- demo out/`). Disciplina **Instalaciones** (la barra de la izquierda): las tuberías de colores a lo largo del muro este y norte de la cocina, con su rótulo; las luminarias en cada zona.
2. **Tubería**: clic, clic, clic e Intro. En Propiedades cambia el sistema a **Agua caliente** y el diámetro a 20: cambia el color y el rótulo.
3. **Conducto** → sistema **Impulsión**, 400 × 250, altura 2700: dibújalo por el pasillo; se ve a trazos.
4. **Iluminación**: clic en el centro de un local; **Enchufe**: Espacio para girarlo y clic junto a un muro.
5. **F3** (3D) → Caja de sección activa → **Planta activa** y baja **Z máx.** a 2800: las tuberías y las luminarias dentro de la casa.
6. **Exportar IFC** y ábrelo en tu visor: las tuberías como segmentos de su sistema (en BIMcollab o Solibri, en el árbol de sistemas).
7. Pestaña **Mediciones**: el capítulo 09 Instalaciones con los metros de cada tubería.

---

## 2026-10-20 (noche, 7) · Spline, Elegir línea y campos de longitud y ángulo

Rama `bloque/metodos` (sale de la de Urbanismo; pull request para Antonio). Sin cambios de formato ni de IFC.

**Hecho** (los dos métodos que faltaban en la barra contextual, y sus campos)
- **Spline**: con Muro, Forjado (también sus huecos), Local, Barandilla, Vial, Parcela y los volúmenes de Concepto, los clics son los puntos de una **curva libre** que pasa por ellos (se ve mientras se dibuja). Doble clic, Intro o clic derecho terminan; en un contorno, también un clic en el primer punto. Los **muros** salen como **muros curvos** de verdad (un arco entre cada dos puntos, con su IFC exacto), no como tramitos rectos; los contornos, como polígonos finos.
- **Elegir línea**: clic en una **línea, polilínea, arco o círculo del dibujo 2D** (también lo que viene de un **DXF** importado) y la herramienta lo sigue: muros con sus arcos, forjados, locales, parcelas, cubiertas y volúmenes sobre una línea cerrada; barandillas, viales y bandas de Seguir a lo largo; vigas, muros cortina, zapatas corridas o vigas de atado y ejes, uno por cada tramo recto.
- **Campos L y ∠** en la barra contextual mientras se dibuja: enseñan la longitud y el ángulo desde el último punto hasta el cursor; escribe una longitud (mm, o 1,2m · 120cm) o un ángulo (grados) y **Intro** pone el punto ahí, con cualquier herramienta de dibujo.
- La pastilla de abajo explica cada método mientras está en uso; Esc quita el último punto de la curva o vuelve al método de antes.

**Pendiente**
- Elegir línea no toma aún aristas de otros elementos (el borde de un forjado o la cara de un muro); solo el dibujo 2D.
- Los terrenos, las escaleras y las secciones no tienen Spline.

**Qué debe probar Antonio**
1. Herramienta **Muro** → en la barra contextual, **Spline**. Haz tres o cuatro clics haciendo una ola y Intro: muros curvos. Míralos en 3D.
2. **Forjado** → Spline: cuatro clics y clic en el primero: un forjado de bordes redondeados.
3. Importa un DXF con una polilínea cerrada (o dibújala con Línea → Polilínea) y, con **Local** → **Elegir línea**, pincha en ella: el local con ese contorno.
4. Con **Muro**, haz el primer clic; en el campo **L** escribe 4500 e Intro; en **∠** escribe 90 e Intro: el siguiente tramo va recto hacia arriba.

---

## 2026-10-20 (noche, 6) · Urbanismo: parcela, vial y forma

Rama `bloque/urbanismo` (sale de la de Concepto; pull request para Antonio). Formato `.archisim` **versión 19**: parcelas y viales. Los proyectos anteriores se abren igual.

**Hecho**
- **Parcela** (Urbanismo → Parcela, tecla **Y**): la linde por sus esquinas (o rectángulo, en la barra contextual), con su **retranqueo** (3 m por defecto). En planta, la linde en trazo y dos puntos, el retranqueo discontinuo dentro y, en el centro, nombre, superficie y referencia catastral. En Propiedades: nombre, referencia catastral, retranqueo, superficie, perímetro, área edificable y la edificabilidad de las masas que tiene encima.
- **Las masas toman la superficie de las parcelas**: sin escribirla a mano, la edificabilidad y la ocupación salen de las parcelas dibujadas. Propiedades de la masa dice si queda **dentro del retranqueo** o se sale.
- **Vial** (tecla **W**): el eje por puntos (doble clic o Enter para terminar) con una **sección tipo** en Propiedades: calzada (6 m), aparcamiento (2,20 m a cada lado; 0 para quitarlo), bordillo y acera; altura del bordillo, firme y rasante; materiales de calzada, bordillo y acera. En planta: calzada y aparcamiento con sus plazas cada 2,50 m, bordillos, aceras y el eje; en 3D, la calzada con su firme y las aceras subidas sobre el bordillo. Propiedades mide la longitud y los m² de calzada, aparcamiento y acera.
- **Forma** (ya en la entrega anterior): volúmenes del entorno con la herramienta Volumen.
- **IFC**: cada parcela es un `IfcSite` dentro del emplazamiento con su linde, la referencia catastral (`LandTitleNumber`) y su superficie; cada vial, en IFC 4.3, un `IfcRoad` con `IfcPavement` (calzada, aparcamiento y aceras) e `IfcKerb`; en IFC4 (que no tiene esas clases) van como objetos genéricos en el emplazamiento. Las parcelas y los viales se leen de vuelta al importar (las parcelas, de cualquier programa).
- La urbanización de la demo trae dos parcelas con su referencia catastral y la «Calle de la Huerta» entre ellas.

**Pendiente**
- Los viales no tienen aún curvas ni cruces (cada tramo es recto y los encuentros se solapan) ni pendiente longitudinal.
- La parcela no sale todavía en las mediciones ni en un capítulo «Urbanización» del BC3; tampoco los viales.
- Zonas verdes y pavimentos exteriores por contorno, mobiliario urbano.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- demo out/` y abre `out/demo-masterplan.archisim`: dos parcelas (A y B) con su linde, retranqueo y referencia, y la calle entre ellas.
2. Pincha la masa «Bloque de viviendas»: en Propiedades, la edificabilidad sale ahora de las parcelas y dice si está dentro del retranqueo.
3. Pulsa **Y** y dibuja una parcela; cambia su retranqueo y su referencia catastral en Propiedades.
4. Pulsa **W**, pon el aparcamiento a 0 en Propiedades y dibuja un vial con dos tramos; míralo en 3D (F3).
5. Abre `out/demo-masterplan.ifc` en ArchiCAD o Revit: las parcelas como emplazamientos y la calle como vial con su pavimento y bordillos.

---

## 2026-10-20 (noche, 5) · Interfaz por disciplinas, bloque I4: Concepto

Rama `bloque/concepto` (sale de la de módulos; pull request para Antonio). Formato `.archisim` **versión 18**: volúmenes de concepto y superficie de la parcela. Los proyectos anteriores se abren igual. Sin cambios en el IFC (los volúmenes no se exportan, como las masas de Revit).

**Hecho** (todas las fichas de Concepto funcionan)
- **Línea, Rectángulo y Círculo** dibujan **caras** planas (un volumen de altura cero) para empujarlas. En la barra contextual se cambia el contorno: polígono, rectángulo, círculo o camino.
- **Empujar**: clic en una cara o un volumen y el ratón hacia arriba lo sube (o se escribe la altura y Enter). Junto al cursor, la altura y, en una masa, sus plantas, la superficie construida y la edificabilidad.
- **Volumen**: un prisma de la altura de Propiedades. **Masa**: un volumen por plantas (3 × 3 m por defecto); al empujarlo va de planta en planta.
- **Seguir**: un perfil (ancho × alto) a lo largo de un camino (doble clic o Enter para terminar): pérgolas, muros de parcela, marquesinas.
- **Equidistancia**: clic en un volumen y se dibuja un contorno paralelo dentro o fuera, encima de él, para empujarlo (áticos retranqueados).
- **Cinta**: mide entre dos puntos y deja una **guía** en la capa «Guías» (discontinua, no se imprime).
- **Pintar**: da a un volumen el material elegido en Propiedades. La plantilla trae «Volumen de concepto» (azul claro).
- **Convertir** (o «Convertir en edificio» en Propiedades del volumen): muros exteriores en sus caras (la cara exterior en el contorno), un forjado en cada planta (solera en la baja) y una cubierta plana arriba, creando las plantas que falten. Se acuerda de lo que hizo: si cambias el volumen, Propiedades avisa y **Actualizar el edificio** lo rehace. Convertido, el volumen solo se ve como contorno discontinuo en planta.
- **Superficies** de cada volumen en Propiedades: huella, construida, edificabilidad (m²/m²) y ocupación (%), con la **superficie de la parcela** que se escribe en las propiedades de las herramientas de Concepto.
- Los volúmenes se ven en planta (relleno claro de su material, con nombre, plantas y m²), en sección y en 3D; se mueven, copian, giran, escalan y reflejan; la ficha **Forma** de Urbanismo hace también volúmenes (edificios vecinos).
- La urbanización de la demo trae una masa de tres plantas (1.008 m², edificabilidad 0,56) en una parcela de 1.800 m².

**Pendiente**
- Empujar se hace desde la planta; en el 3D aún no se arrastran las caras.
- Las caras no se pegan entre sí como en SketchUp (cada volumen es un prisma propio) ni se empujan caras laterales.
- Convertir solo hace muros, forjados y cubierta plana (ni huecos ni cubiertas inclinadas).
- La parcela es una superficie escrita a mano; con el bloque de Urbanismo saldrá de la parcela dibujada.

**Qué debe probar Antonio**
1. Proyecto nuevo, disciplina **Concepto** (la barra de la izquierda). **Rectángulo**: dos clics. **Empujar**: clic en él y sube el ratón; clic para dejarlo.
2. **Masa**: dibuja un contorno; luego Empujar y escribe 12000 + Enter: cuatro plantas. Mira en Propiedades la superficie construida; escribe la superficie de la parcela en las propiedades de la herramienta y mira la edificabilidad.
3. **Equidistancia** sobre la masa con −2000 + Enter y empuja esa cara 3000: un ático retranqueado.
4. Selecciona la masa y pulsa **Convertir**: muros, forjados y cubierta. Cambia sus plantas a 5 y pulsa **Actualizar el edificio**.
5. Abre `out/demo-masterplan.archisim` (tras `cargo run -p archisim-cli -- demo out/`): la masa «Bloque de viviendas» con sus cifras.

---

## 2026-10-20 (noche, 4) · Interfaz por disciplinas, bloque I5: módulos

Rama `bloque/modulos` (sale de la de ejes y cimentación; pull request para Antonio). Formato `.archisim` **versión 17**: módulos. Los proyectos anteriores se abren igual.

**Hecho**
- **Módulo** (Coordinación → Módulo, tecla **K**): otro proyecto de ArchiSim (una vivienda tipo) se coloca en este tantas veces como haga falta.
  - En Propiedades, **Cargar módulo…** elige el fichero. Sus tipos y materiales pasan al proyecto (los que ya hay iguales, de la misma plantilla, no se repiten).
  - Cada clic coloca una copia con su origen (0,0) en el punto; **Espacio** gira la siguiente 90°; **Simetría** la refleja (pareadas, adosadas).
  - Si el módulo tiene más plantas que las que hay encima, se crean con su altura.
- **Una copia se maneja entera**: se selecciona con un clic dentro, se mueve, copia y gira como cualquier elemento, y en Propiedades se cambian planta, X, Y, giro y simetría. No se edita por dentro: se cambia el proyecto del módulo.
- **Navegador → Proyecto → Módulos**: cada módulo con su número de copias. Si su fichero ha cambiado lo dice («ha cambiado») con un botón **Actualizar**: todas las copias cambian a la vez. Clic derecho: actualizar o quitar el módulo con sus copias.
- **El proyecto guarda una copia de cada módulo**: se abre igual en otro ordenador aunque falte el fichero («sin fichero»).
- **Plantas, secciones, 3D, planos, tablas, mediciones, BC3, CTE e IFC** cuentan las copias como elementos: las mediciones multiplican solas por el número de copias. Los locales de cada copia llevan delante su número de copia («2-0.01»).
- **IFC**: cada elemento de cada copia sale con un `GlobalId` propio calculado del de la copia y el del elemento: no chocan entre copias y son los mismos en cada exportación (comprobado: la urbanización exportada dos veces da el mismo IFC).
- La demo escribe ahora además `out/demo-masterplan.archisim` y su IFC: la vivienda de la demo tres veces, una reflejada y otra girada 180°.

**Pendiente**
- Al seleccionar una copia no se resalta en el 3D (sí en planta).
- Del módulo no pasan sus cotas, textos, secciones, ejes ni terreno: en la urbanización mandan los suyos.
- Las mediciones no separan aún la urbanización en su propio capítulo.
- En el IFC las copias no van agrupadas (cada copia podría ser un `IfcElementAssembly`).

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- demo out/` y abre `out/demo-masterplan.archisim`: tres viviendas, la segunda reflejada (las entradas enfrentadas) y la tercera girada.
2. Pincha dentro de una vivienda: se selecciona entera. Cambia Giro o marca Simetría en Propiedades. Arrástrala.
3. Pulsa **K** y haz clic en un sitio libre: otra copia. Espacio antes de hacer clic la gira.
4. Abre `out/demo.archisim`, cambia algo (mueve un tabique) y guarda. Vuelve a la urbanización: en el navegador, Módulos → «ha cambiado» → **Actualizar**. Las tres copias cambian.
5. Exporta el IFC de la urbanización y ábrelo en ArchiCAD o Revit: 33 muros y 12 puertas, todos con identificadores distintos.

---

## 2026-10-20 (noche, 3) · Ejes y cimentación

Rama `bloque/ejes-cimentacion` (sale de la de I3; pull request para Antonio). Formato `.archisim` **versión 16**: ejes y cimentación. Los proyectos anteriores se abren igual. El IFC lleva ahora `IfcGrid` e `IfcFooting`.

**Hecho**
- **Ejes** (ficha Ejes en Concepto, Arquitectura y Estructura; tecla **X**): dos clics por eje. Se nombran solos: los que suben, **1, 2, 3…**; los que cruzan, **A, B, C…**. Se ven en todas las plantas, con su burbuja en cada extremo y trazo y punto, y en las secciones que los cortan, en vertical con la burbuja arriba. El nombre y la longitud se cambian en Propiedades. Se mueven, copian y giran como lo demás.
- **Cimentación** (ficha Zapata en Estructura; tecla **A**), con tres métodos en la barra contextual:
  - **Aislada**: clic en un pilar y la zapata queda **bajo él y lo sigue** si se mueve o gira; clic en otro sitio, zapata suelta en ese punto. Por defecto 1200 × 1200 × 600 mm.
  - **Corrida**: clic en un muro y la zapata queda **bajo su eje y lo sigue**; clic fuera, se dibuja por dos puntos. Por defecto 600 × 500 mm.
  - **Viga de atado**: por dos puntos, encadenadas. Por defecto 400 × 400 mm.
  - **Cimentar planta**: zapatas bajo todos los pilares y muros rectos de la planta que aún no la tengan.
  - Cara superior por defecto a −350 mm (bajo la solera), en hormigón. Todo se cambia en Propiedades, también después.
- **Losa** (ficha en Estructura): la herramienta Forjado con el tipo nuevo de la plantilla **«Losa de cimentación HA 50»** (50 cm + 10 cm de limpieza).
- En planta, la cimentación se dibuja **a trazos** (está bajo el suelo); en sección se ve cortada; en 3D, con su volumen. Es de Estructura: se resalta al trabajar en esa disciplina.
- **IFC**: cada zapata es un `IfcFooting` (`PAD_FOOTING`, `STRIP_FOOTING`, `FOOTING_BEAM`) con su material, `Pset_FootingCommon` y cantidades (longitud, ancho, canto, volumen); los ejes, un `IfcGrid` con sus `IfcGridAxis`. Al importar un IFC de otro programa se leen sus zapatas rectangulares y sus ejes. La demo lleva ejes 1–2 y A–C, corridas bajo la fachada, zapatas bajo los pilares del porche y una viga de atado entre ellas.

**Pendiente**
- Prioridad de la cimentación sobre muros y forjados en los encuentros (hoy se solapan con la solera si se ponen a la misma cota).
- Zapatas bajo muros curvos, encepados y pilotes; cotas atadas a los ejes.
- Las mediciones y el BC3 aún no tienen capítulo de cimentación.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-app -- out/demo.archisim`: en la planta baja verás los ejes 1, 2, A, B y C con sus burbujas y, a trazos, las zapatas.
2. Abre la **Sección A**: los ejes 1 y 2 suben con su burbuja y las zapatas corridas aparecen cortadas bajo los muros.
3. Pincha **Estructura → Zapata**. Con **Aislada**, clic en un pilar del porche que no tenga zapata (o pon uno nuevo con Pilar); mueve el pilar: la zapata va con él.
4. Elige **Corrida** y pincha un tabique: queda la zapata bajo su eje. Luego **Cimentar planta**.
5. Pulsa **X** y dibuja un eje vertical y otro horizontal: deben llamarse 3 y D. Cambia el nombre en Propiedades.
6. Exporta el IFC y ábrelo en ArchiCAD o Revit: deberían salir como zapatas (cimentación estructural) y como rejilla de ejes.

---

## 2026-10-20 (noche, 2) · Interfaz por disciplinas, bloque I3

Rama `bloque/interfaz-i3` (pull request para Antonio). Formato `.archisim` **versión 15**: disciplina dada a mano y atenuación de cada vista. Los proyectos anteriores se abren igual. Sin cambios en el IFC.

**Hecho**
- **Cada elemento tiene su disciplina.** Sin tocar nada:
  - pilares y vigas son Estructura;
  - muros y forjados son Estructura si tienen una capa estructural de **hormigón, acero o madera**;
  - un muro de ladrillo es Arquitectura, porque en un edificio con estructura de pórticos cierra y no carga;
  - terreno, vegetación y vehículos son Urbanismo;
  - lo demás es Arquitectura.
- **Disciplina a mano:** en Propiedades → **Disciplina**, «Automática (…)» o la que quieras (por ejemplo, un muro de carga de ladrillo).
- **Atenuación**: al trabajar en **Estructura**, **Instalaciones** o **Urbanismo**, lo que no es de esa disciplina se ve en **gris**; en **Concepto**, todo el edificio en gris.
  - Vale en planta, sección y 3D.
  - Es un ajuste de la vista: en Propiedades → Visibilidad → **Atenuación** se elige también para cada vista colocada en un plano, y sale en el PDF y el DXF.
  - **Vista → Atenuar por disciplina** la apaga.
- **Navegador → Modelo → Disciplinas**: cada disciplina con su número de elementos, un **ojo**, que la oculta en todas las vistas, y un **candado**, que hace que se vea pero no se pueda seleccionar.

**Pendiente**
- El ojo y el candado no se guardan en el proyecto: duran la sesión.
- Los planos no copian solos la atenuación de la pestaña al colocar una vista: se elige en la vista colocada.

**Qué debe probar Antonio**
1. Abre la demo y pincha **Estructura** en la barra de la izquierda: los tabiques y la fachada de ladrillo se ven en gris; forjados, pilares y vigas, a su color. Pulsa F3: el 3D igual.
2. Pincha **Concepto**: todo en gris. Vuelve a **Arquitectura**: todo normal.
3. Selecciona un muro de fachada: Propiedades → Disciplina → Estructura. Vuelve a Estructura y míralo resaltado.
4. Navegador → **Modelo** → Disciplinas: apaga el ojo de Urbanismo (desaparecen terreno y árboles) y pon el candado a Arquitectura (los muros ya no se seleccionan).
5. En el plano A-01, selecciona la planta y en Visibilidad → Atenuación elige «Resaltar estructura»; exporta el PDF.

---

## 2026-10-20 (noche) · Interfaz por disciplinas, bloque I2 (v0.35)

Bloque I2 de `docs/INTERFAZ.md` y lo que pidió Antonio al probar I1, en la rama `bloque/interfaz-i2` (pull request para Antonio). Formato `.archisim` **versión 14** (vistas 3D en planos); los proyectos anteriores se abren igual. Sin cambios en el IFC.

**Hecho**
- **Línea de órdenes oculta por defecto.** Lo que se escribe sale junto al cursor y el paso de la orden en marcha, en la barra contextual. Se deja fija en **Vista → Línea de órdenes fija**; «Teclas de AutoCAD» pasa también a Vista.
- **Barra contextual fija bajo las pestañas**:
  - con una herramienta, sus **métodos**: Recta, Cadena, Rectángulo, Polígono, Arco, Círculo, Varita (Edificio en cubiertas, Recinto en locales), Spline y Elegir línea. Los que no valen se ven apagados, no desaparecen; Spline y Elegir línea llegarán más adelante. Con la Cota: lineal, huecos de muro, cota de nivel, angular, radio y longitud de arco;
  - con algo seleccionado, **Modificar · Muro (1)**: Mover, Copiar, Girar, Simetría, Escala y Matriz | Desfase, Alinear, Recortar, Alargar, Partir y Unir. Lo que no vale para la selección sale apagado (escalar una puerta). Si no caben con su nombre, se quedan en dibujos con su nombre y tecla al pasar el ratón;
  - con una orden en marcha, lo que pregunta, sus opciones como botones y «Terminar».
  Los métodos salen de Propiedades: allí quedan los ajustes de cada uno (contorno, hueco de forjado, dirección de la cota).
- **Barra inferior por segmentos**:
  - escala y zoom;
  - **Línea**, **Relleno**, **Texto** y **Cota**. El clic lanza su orden; la flecha abre las demás formas (polilínea, rectángulo, arco, círculo, elipse; directriz, nube, etiqueta; cota de nivel, angular…) y sus ajustes (capa, grosor, tipo de línea, trama, escala, ángulo, altura del texto);
  - los **forzados como dibujos**: extremos, medios, aristas, rastreo y orto;
  - las coordenadas.
  Rejilla, líneas de referencia, líneas finas, calco y fases siguen en las propiedades de la vista y en Vista.
- **Tablas y mediciones en pestañas**, no en ventana flotante. Cada tabla del navegador se abre como una vista más. La pestaña **Mediciones** pone todas las de cantidades seguidas, con su planta y la exportación CSV y BC3. En Análisis, la ficha Mediciones la abre.
- **Arrastrar a un plano**: con un plano abierto, se arrastran desde el navegador plantas, secciones, alzados, tablas y la **Vista 3D**, y quedan donde se sueltan, a la escala de su pestaña.
  - La vista 3D se coloca como **axonometría** desde donde la estés mirando, a la escala que quepa en el papel (1:200 en la demo).
  - Se dibuja con vectores: caras con el color del material, sombreadas, y aristas donde la superficie gira. Sale igual en el PDF y en el DXF.
  - En Propiedades, «Tomar el punto de vista de la vista 3D» la actualiza.
  - La demo trae una en el plano A-02.
- Versión 0.35.0.

**Pendiente**
- I3: atenuación por disciplina, ojo y candado por disciplina en el navegador.
- En la barra contextual faltan los campos de longitud y ángulo tecleables (hoy se teclean en las cotas azules temporales) y los métodos Spline y Elegir línea.
- La axonometría usa el orden de pintor: en encuentros complicados (petos, aleros) puede verse alguna cara mal tapada. Si molesta, la paso a líneas ocultas exactas.
- Vistas 3D en perspectiva en los planos: hoy solo axonometría, porque la perspectiva no tiene escala.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-app -- out/demo.archisim`. Abajo ya no está la línea de órdenes. Con «Teclas de AutoCAD» (menú Vista) escribe `L` e Intro: la L sale junto al cursor y el paso de la orden, en la barra bajo las pestañas.
2. Pincha Muro: arriba, Recta, Cadena, Rectángulo, Arco y Círculo encendidos y el resto apagados. Pasa a Forjado: Polígono, Rectángulo y Varita.
3. Selecciona un muro: «Modificar · Muro (1)»; pulsa Girar y sigue los pasos de la barra.
4. Abajo, pincha la flecha de **Línea** y elige Círculo; cambia la capa. Prueba los forzados.
5. Navegador → Tablas → **Mediciones**: se abre una pestaña con todo y los botones CSV y BC3.
6. Abre el plano A-01 y arrastra desde el navegador «Planta baja», «Muros» y «Vista 3D» al papel. Mira el plano A-02, que ya trae la axonometría, y expórtalo a PDF.

---

## 2026-10-20 (tarde) · Interfaz por disciplinas, bloque I1

Primer bloque de `docs/INTERFAZ.md`, en la rama `bloque/interfaz-i1` (pull request para Antonio). Sin cambios de formato ni de IFC.

**Hecho**
- **Barra de disciplinas** a la izquierda del todo: la Flecha arriba y, debajo, Concepto · Arquitectura · Estructura · Instalaciones · Urbanismo · Documentación · Análisis · Coordinación. La elegida se marca en azul.
- **Fichas de tamaño fijo**: cuatro por fila, 54 px, siempre el sitio de tres filas, con los **iconos nuevos** de `docs/interfaz/iconos.svg` (ventana y pilar en alzado, barandilla con su pasamanos, cubierta a dos aguas…). Sin borde en reposo, fondo suave al pasar el ratón, borde azul en la que está en uso. La tecla y para qué sirve salen al pasar el ratón.
- **Propiedades debajo de las fichas**, en la misma columna; al cambiar de disciplina no se mueven. La columna se ensancha arrastrando su borde y el navegador la sigue para que el dibujo quede centrado.
- Cada disciplina con sus herramientas: las que ya existen funcionan (Abertura dibuja huecos en forjados; Vegetación es Objeto con la librería de árboles; Alzado, Plano, Tabla, CTE, Mediciones, Vincular, Versiones, Interferencias, Incidencias y Requisitos abren lo que ya había); las que faltan (todo Concepto, Ejes, Zapata, Losa, Instalaciones, Parcela, Vial, Módulo, Equipo, Publicar) se ven apagadas y dicen qué bloque las trae.
- **Las teclas cambian de disciplina** (D18): I lleva a Estructura, N a Documentación; F en Estructura se queda en Estructura, porque el forjado está en las dos.
- **Buscador de órdenes**: Ctrl+Espacio o el campo «Buscar herramienta…» de la barra superior. Se escribe «viga», «cota», «simetría» (con o sin tilde) o la tecla («EQ»), flechas para elegir e Intro. Dice en qué disciplina está cada cosa y su tecla. Encuentra también las órdenes de dibujo y edición 2D (línea, polilínea, sombreado, desplazar, recortar, empalme…), que ya no tienen ficha: hasta el bloque I2 se usan desde aquí o desde la línea de órdenes.
- **Navegador con dos pestañas**: Modelo (lo construido por tipos, con su número) y Proyecto (plantas, vistas, secciones, alzados, planos y tablas).
- Quitados la caja antigua por secciones y el modo «solo iconos». En el diseño, el buscador pasa a **Ctrl+Espacio** (Espacio solo ya hace la varita y gira puertas) y la rejilla de ventilación sale de Instalaciones (la «rejilla» que pidió Antonio son los ejes).

**Pendiente**
- I2: barra contextual bajo las pestañas (métodos de inserción y Modificar) y barra inferior segmentada con línea, relleno, texto y cota; ahí vuelven a tener botón el dibujo y la edición 2D.
- Inicio y Preferencias al pie de la barra de disciplinas; recordar la disciplina entre sesiones.

**Qué debe probar Antonio**
1. `cargo run --release -p archisim-app -- out/demo.archisim`. Pasa por las ocho disciplinas: las fichas no cambian de tamaño y Propiedades se queda en su sitio.
2. En Arquitectura, pincha Muro: la ficha se marca y debajo sale el muro que vas a dibujar. Pincha «Teclas: AutoCAD» (a la derecha de la línea de órdenes) para que diga «Teclas: ArchiCAD» y pulsa I: salta a Estructura con el pilar.
3. Ctrl+Espacio, escribe «zapata» (sale apagada, todavía no), luego «girar» e Intro.
4. En Coordinación, Interferencias abre la ventana de coordinación; en Análisis, CTE abre la comprobación.
5. Navegador: cambia entre Modelo y Proyecto.
6. Dime si los iconos de la barra de disciplinas te parecen bien (he usado volumen, muro, pilar, tubería, árbol, cota, escudo y vínculo).

---

## 2026-10-20 (noche) · Familias: parámetros del proyecto y compartidos (v0.34)

Tanda 3 de las cinco pedidas el 20/10 (rama `bloque/parametros`, encima de la de las tandas 1–2).

**Hecho**
- **Parámetros del proyecto** (como los de proyecto y compartidos de Revit o el gestor de propiedades de ArchiCAD): **Vista → Parámetros del proyecto…**. Cada uno con nombre, clase (texto, número, entero, longitud, superficie, sí/no, lista de opciones), grupo (el conjunto de propiedades del IFC), categorías, si es del tipo o de cada elemento, valor por defecto y descripción.
- Se rellenan en **Propiedades → Parámetros del proyecto** (cualquier elemento seleccionado solo) y, los del tipo, en **Tipos y materiales**; en Propiedades se ven con «(tipo)».
- Salen como **columnas en las tablas** de su categoría y en el **IFC** en cada elemento, en el conjunto de su grupo; reimportando el IFC vuelven las definiciones y los valores.
- **Compartidos**: cada parámetro tiene identidad fija (`GlobalId`). «Guardar como compartidos» escribe `Parámetros.asparam` en la carpeta de la biblioteca; «Cargar los compartidos…» los lleva a otro proyecto.
- Las recetas pueden poner un parámetro en un conjunto estándar del IFC (`ifc "Pset_DoorCommon.FireRating"`): la puerta abatible tiene ya **Resistencia al fuego** (EI2 30-C5…), que sale en `Pset_DoorCommon` y en la tabla de carpinterías.
- La demo trae dos de ejemplo: «Acabado de suelo» de los locales (`AD5_Acabados`) y «Código de medición» de los tipos de muro (`AD5_Mediciones`, FAB01…).
- De paso: el IFC de los muros con huecos tenía una representación «CSG» mal declarada (IfcOpenShell la aceptaba, otros podrían no): ahora es un `IfcCsgSolid` como manda la norma, y el importador la lee.
- Formato `.archisim` **versión 13** (los proyectos anteriores se abren sin parámetros). Versión 0.34.0.

**Pendiente**
- Tandas 4 (familias de anotación: etiquetas como recetas, que muestran valores de parámetros) y 5 (importar objetos de otros programas por IFC y editor gráfico de recetas).
- Los parámetros del tipo van en el IFC en cada elemento (con el valor de su tipo), no en el `IfcWallType`…: así los ve cualquier programa en el elemento. Si preferís en el tipo, como Revit, es un cambio pequeño.

**Qué debe probar Antonio**
1. Abre la demo y selecciona el **Dormitorio**: al final de Propiedades, «Parámetros del proyecto → Acabado de suelo: Tarima». Cámbialo a Microcemento.
2. **Vista → Parámetros del proyecto…** → «＋ Nuevo»: crea «Resistencia al fuego» para Muros, del tipo, lista «—, EI 60, EI 120». Abre Tipos y materiales → Muros → Fachada: ponle EI 120.
3. Navegador → Tablas → Muros: aparecen las columnas «Código de medición» y «Resistencia al fuego».
4. Exporta el IFC y ábrelo en otro visor (BIMcollab, Solibri, Revit): en un muro, el conjunto `AD5_Mediciones`; en un local, `AD5_Acabados`; en una puerta, `Pset_DoorCommon.FireRating`.
5. «Guardar como compartidos», abre un proyecto nuevo y «Cargar los compartidos…»: están los mismos.

---

## 2026-10-20 (tarde) · Familias: lo que faltaba del configurador, tipos multicapa en la biblioteca y familias nuevas (v0.33)

Tandas 1 y 2 de las cinco pedidas el 20/10 a las 11:31 (pendientes de familias y familias nuevas); siguen parámetros compartidos, familias de anotación, importación de objetos y editor gráfico de recetas.

**Hecho**
- **Símbolo 1:20 en las recetas**: la puerta abatible dibuja a 1:20 el cerco con el **galce** donde cierra la hoja; la ventana practicable, el perfil de **cada hoja**, los montantes y el **doble vidrio**. Se ven en la planta a 1:20 y en la vista previa de Tipos y materiales (que es 1:20). El resto de familias usan a 1:20 su símbolo de 1:50, como hasta ahora.
- **Alzado en las recetas** (bloque `elevation`, descrito en `docs/FAMILIAS.md`) con las **líneas de apertura**: el vértice del triángulo señala las bisagras; oscilobatiente con los dos triángulos; abatible y proyectante con el vértice abajo o arriba; basculante y pivotante con un rombo y su eje en trazo y punto; correderas con una flecha; plegable con el zigzag. Salen en la vista previa «Alzado» de los tipos y en **los alzados y secciones del proyecto**: continuas vistas desde el lado al que abren las hojas, **discontinuas** desde el otro (así se ven por fuera las ventanas que abren hacia dentro).
- **Hoja pivotante** (eje vertical) en la ventana practicable, junto a las seis que había: en planta, la hoja girada atravesando el marco; en IFC, `PIVOTVERTICAL` (también al importar).
- **Puntos de control de puertas y ventanas** (los *hotspots* de ArchiCAD): con una puerta o ventana seleccionada aparecen dos en sus jambas, en la cara del muro contraria a la que abre (en la otra están las cotas temporales). Arrastrando uno, el hueco se ensancha o se estrecha con la otra jamba quieta. Si el tipo lo usan otras puertas, se hace un tipo propio (con marca nueva) para no cambiarlas.
- **Seleccionar los ejemplares de todo el proyecto**: Navegador → clic derecho sobre un tipo → «Seleccionar los N de todo el proyecto» (todas las plantas); lo que cambies en Propiedades se aplica a todos.
- **Tipos de muro y de forjado en la biblioteca**: ficheros `Muros.astipos` y `Forjados.astipos` (texto legible: cada tipo con su `GlobalId`, función y capas, y los materiales con su color y trama). Navegador → clic derecho sobre un tipo de muro o forjado → «Guardar en la biblioteca»; ventana Biblioteca → «Cargar», con la misma regla del `GlobalId` que las puertas (igual: no hace nada; distinto: sustituir o cargar como otro tipo). `archisim biblioteca` los escribe también, en la carpeta `Multicapas`.
- **Familias nuevas** (tanda 2):
  - **Puerta de garaje**: seccional (con su nº de paneles), basculante o enrollable (con tambor), puerta peatonal en la hoja si se quiere. En planta, en discontinua, por dónde va al abrir (las guías bajo el techo, el vuelo de la basculante o el tambor). IFC: `ROLLINGUP` la enrollable; las otras, `USERDEFINED` con `OVERHEAD` (IFC no tiene un funcionamiento propio para ellas).
  - **Puerta pivotante**: una hoja con el eje separado de la jamba («Eje: distancia a la jamba»); en planta la hoja atraviesa el hueco con sus dos giros. IFC `USERDEFINED` con `PIVOT`.
  - **Ventana de guillotina**: dos hojas que deslizan en vertical en dos planos; se mueve la de abajo o las dos; altura del encuentro. IFC `DOUBLE_PANEL_HORIZONTAL` con `SLIDINGVERTICAL`.
  - **Fijos laterales** en la puerta abatible: izquierdo, derecho o los dos, con su ancho y zócalo ciego, separados de la hoja por un montante. Para el CTE cuenta el paso libre de la hoja, no el hueco entero (las familias pueden dar ahora la salida `clear_width`).
  - **Ventana en esquina**: en la ventana practicable, el parámetro del ejemplar «Ventana en esquina» (izquierda o derecha, vista desde donde abre) quita la jamba de ese lado, la lleva al encuentro con el muro contiguo (donde se cruzan los ejes de los dos muros) y abre el muro a través de la esquina, para que se encuentre con la ventana del otro muro sin machón. Sigue a la esquina al mover los muros o cambiar el ancho. Se cambia en **Propiedades → Parámetros del ejemplar**, un grupo nuevo que sale con cualquier familia que tenga parámetros de ejemplar.
  - Hojas **pivotante** y basculante en la ventana practicable (la basculante ya estaba como opción de hoja).
  - Siete tipos nuevos en la plantilla: puerta de entrada con fijo lateral 140×215, puerta pivotante 120×240, puertas de garaje seccional 250×220 y enrollable 300×250, ventana de guillotina 90×150, ventana pivotante 80×100 y ventana basculante 100×60. `archisim muestrario` los pone todos en un muro.
- Familias de sistema: puerta abatible v6, ventana practicable v6, ventana corredera v4, puertas correderas y plegable una versión más (los proyectos anteriores las actualizan desde la ventana Biblioteca).
- Versión 0.33.0. Sin cambios de formato `.archisim` ni de IFC (demo validada en IFC 4.3 e IFC4; ida y vuelta idéntica salvo la fecha; DXF y PDF validados).

**Pendiente**
- Tandas 3 a 5 (parámetros compartidos, anotación, importación de objetos y editor gráfico de recetas).
- La ventana en esquina está en la practicable; la corredera en esquina, si la usáis, sería igual.
- El símbolo 1:20 de correderas, plegable y huecos (usan el de 1:50).

**Qué debe probar Antonio**
1. Abre la demo. En la barra inferior pon la planta a **1:20** y acércate a una puerta y a una ventana: el cerco con su galce y las hojas con doble vidrio.
2. Abre **Alzado Sur** (Navegador → Secciones y alzados): las ventanas tienen sus líneas de apertura **discontinuas** (abren hacia dentro y las ves desde fuera), con el vértice hacia las bisagras. Dime si es la convención que usáis.
3. **Tipos y materiales → Ventanas → V-01 → Alzado**: líneas de apertura continuas. Cambia la hoja 1 a «oscilobatiente», «proyectante» y «pivotante» y mira el alzado y la planta.
4. Selecciona una puerta en planta: arrastra el punto de control de una de sus jambas 20 cm. La puerta se ensancha con la otra jamba quieta; si había más puertas de ese tipo, aparece un tipo nuevo («… (2)»).
5. Navegador → MODELO → Muros → clic derecho sobre «Fachada doble hoja 26,5» → **Guardar en la biblioteca**. Archivo → Nuevo, Vista → Biblioteca…: en «Muros · tipos multicapa» pulsa **Cargar**.
6. Navegador → Puertas → clic derecho sobre un tipo → «Seleccionar los N de todo el proyecto».
7. `cargo run -p archisim-cli -- muestrario out/` y abre `out/muestrario.archisim`: puertas de garaje (seccional y enrollable), pivotante, puerta con fijo lateral, guillotina, ventanas pivotante y basculante, en planta, en 3D (F3) y en alzado. Abre `out/muestrario.ifc` en ArchiCAD o Revit y mira cómo leen la puerta de garaje y la pivotante.
8. **Ventana en esquina**: dibuja dos muros en L, pon una ventana practicable cerca de la esquina en cada uno; selecciona la primera y en **Propiedades → Parámetros del ejemplar → Ventana en esquina** elige el lado de la esquina (si eliges el otro, se va a la otra punta del muro); haz lo mismo con la otra. Mira en planta y en 3D que se encuentran sin machón.

---

## 2026-10-20 (mediodía) · Diseño de la interfaz por disciplinas (sin código)

**Hecho**
- Diseño, en conversación con Antonio y a partir de su boceto de hace unos años, de la nueva interfaz: **`docs/INTERFAZ.md`**. Bocetos interactivos revisados con él en varias vueltas.
- **Barra de disciplinas** como modo de trabajo (cambia las herramientas y atenúa la vista de lo que no es de la disciplina): Concepto · Arquitectura · Estructura · Instalaciones · Urbanismo · Documentación · Análisis · Coordinación.
- **Fichas de herramientas en formato fijo** (cuatro columnas con icono y nombre, variante B elegida por Antonio) y **Propiedades** debajo, siempre con sitio.
- **Iconos propios** en lenguaje de dibujo de arquitectura, revisados uno a uno con Antonio (ventana, pilar y cubierta en alzado; barandilla con pasamanos; zapata en sección…): **`docs/interfaz/iconos.svg`**.
- **Barra contextual fija** bajo las pestañas: métodos de inserción con la herramienta activa (recta, cadena, rectángulo, arco, círculo, spline, elegir línea) y **Modificar** con la selección (mover, copiar, girar, simetría, escala, matriz; desfase, alinear, recortar, alargar, partir, unir), con la forma rápida (arrastrar) y la precisa de Revit (punto base y destino). Los ajustes de cada método, en Propiedades.
- Pestañas de vistas y **barra inferior segmentada** como en el boceto de Antonio (escala, línea, relleno, texto, cota, con forzados y coordenadas). Tema claro por defecto.
- Disciplina **Concepto** al estilo SketchUp (aristas y caras, empujar, seguir, volumen, masa con edificabilidad, convertir a edificio) y **Coordinación** (vínculos, **módulos** para viviendas tipo de un masterplan, versiones, interferencias, incidencias, requisitos, equipo, publicar), con las buenas prácticas de ficheros separados frente a un solo proyecto y la propuesta de trabajo en equipo con reservas.
- Plan por bloques I1–I6 y decisiones D17–D22 en la hoja de ruta. Sin cambios en el programa, el formato `.archisim` ni el IFC.

**Pendiente**
- Que Antonio decida D17–D22; después, bloque I1.

**Antonio debe revisar**
1. `docs/INTERFAZ.md` entero, sobre todo §2.6 (barra contextual), §4 (herramientas por disciplina) y §9 (módulos y equipo).
2. `docs/interfaz/iconos.svg` (se abre con el navegador): ¿algún icono que cambiar?
3. Las decisiones D17 a D22 del §11.

---

## 2026-10-20 (tarde) · Librería de AD5, primera vuelta (2.2, v0.32)

Quinto y último bloque de los pedidos el 19/10. La v0.31 (CTE) quedó fusionada en `main` con la integración continua en verde.

**Qué se hizo** (detalle en `docs/FAMILIAS.md`, «Librería de AD5»)
- **Puerta abatible**: dos hojas **desiguales** con hoja activa, **tarja** acristalada y **premarco**.
- **Ventana practicable**: **tarja** fija, **premarco** y **vierteaguas** en 3D.
- **Ventana corredera**: de **2 a 4 hojas**, con premarco y vierteaguas.
- **Nueve tipos habituales** en la plantilla, como propuesta para el catálogo de AD5 (puertas de paso de 72,5 y 82,5 —esta cumple la vivienda accesible—, entrada con tarja, dos hojas desiguales, oscilobatiente, practicable con persiana y vierteaguas, balconera con persiana, corredera de 3 hojas, practicable con tarja).
- `archisim biblioteca [carpeta]`: escribe los tipos de la plantilla como ficheros de biblioteca (Puertas, Ventanas, Mobiliario…), listos para la ventana Biblioteca. `archisim muestrario [carpeta]`: un muro con todas las puertas y ventanas (IFC validado).
- Un material de un parámetro oculto (el vidrio de la tarja en una puerta sin tarja) ya no cuenta como «en uso».
- Sin cambios de formato: los proyectos antiguos ofrecen «Actualizar» de estas familias en la ventana Biblioteca.

**Pendiente**
- El **catálogo real de AD5**: dime qué puertas y ventanas usáis de verdad (medidas, materiales, marcas) y lo cargo.
- Puerta de garaje, ventana pivotante y basculante, fijos laterales en puertas.

**Qué debe probar Antonio**
1. `cargo run -p archisim-cli -- muestrario out/` y abre `out/muestrario.archisim`: en la planta y en 3D (F3) mira las puertas con tarja y premarco, la de dos hojas desiguales, la corredera de 3 hojas y los vierteaguas. Abre también `out/muestrario.ifc` en ArchiCAD.
2. En **Tipos y materiales → Puertas**, elige «Puerta de 2 hojas desiguales» y cambia «Hoja activa: ancho» y la mano de una puerta colocada: la hoja grande cambia de lado.
3. `cargo run -p archisim-cli -- biblioteca` y, en la app, **Vista → Biblioteca… → Cambiar carpeta…** a `out/biblioteca`: salen todas las familias con sus tipos para cargarlas en otro proyecto.
4. Revisa la lista de nueve tipos: ¿son los que usaríais? ¿Qué cambiarías?

---

## 2026-10-20 (mediodía) · Comprobación del Código Técnico (4.7, v0.31)

Cuarto bloque de los pedidos el 19/10. La v0.30 (biblioteca de tipos) quedó fusionada en `main` con la integración continua en verde.

**Qué se hizo** (detalle en `docs/CTE.md`)
- **Datos del edificio** en el proyecto: uso (Residencial Vivienda, Administrativo, Docente…), accesibilidad exigida (ninguna, itinerarios accesibles o vivienda accesible para silla de ruedas) y extinción automática. Cada local puede fijar su **densidad de ocupación** (Propiedades → Ocupación (CTE)). Formato `.archisim` versión 12; los proyectos anteriores se abren como viviendas.
- **Ocupación** (SI 3, tabla 2.1) desde el uso de cada local.
- **Evacuación** (SI 3): salidas (puertas al exterior en la planta de acceso, escaleras en las demás), **recorrido más largo** de cada planta andando por dentro de los locales y por sus puertas, con su límite (25/35/50 m, +25 % con rociadores) y el máximo de ocupantes con una salida; **anchura de puertas y pasillos** por las personas que pasan (P/200); hojas de 0,60–1,23 m; puertas abatibles y que **abren hacia la salida** cuando toca. En viviendas, desde la puerta de cada vivienda por las zonas comunes.
- **Accesibilidad** (SUA 9 y Anejo A): puertas (0,80 y 0,78 m), pasillos (1,20 / 1,10 m) y **círculos de giro** de Ø 1,50 que **descuentan los muebles** colocados, en vestíbulos, aseos accesibles o, en vivienda accesible, estar, dormitorios, cocina, un baño y terrazas (Ø 1,20).
- **Escaleras y rampas** (SUA 1): la comprobación de cada escalera y la pendiente de los forjados inclinados (12 %, o 10/8/6 % accesibles, tramos de 9 m).
- **Superficies**: útil por local, uso y planta; construida por planta y del edificio.
- **Ventana Colaboración → Comprobación CTE…**: los datos del edificio arriba, los resultados por apartado (✖ no cumple, ! aviso, ✔ cumple), «Solo lo que falla», exportar CSV. Un clic en un resultado selecciona el elemento en su planta y, si es un recorrido, lo **dibuja en naranja** hasta la salida.
- **Orden** `archisim cte proyecto.archisim [informe.csv]`; la demo deja `out/demo-cte.txt` y ahora es «vivienda accesible», para que se vea algo: las P-02 de 80 cm dejan 70 cm libres y no cumplen, y su rampa del 10 % tampoco.
- Pruebas: oficina de 3 despachos con una salida (recorrido de más de 25 m que no cumple) y con dos (cumple), puertas y pasillo, local sin puerta, sala de más de 50 personas con la puerta en un sentido y en otro, escalera como salida de la planta alta, cama en un dormitorio que quita el círculo de giro, ventana en la app.

**Pendiente**
- Lo que lista `docs/CTE.md` al final (escaleras protegidas y sectores, anchura de escaleras por ocupación, recorridos alternativos, barandillas por altura de caída, habitabilidad autonómica).
- Llevar la ocupación al IFC (`Pset_SpaceOccupancyRequirements`).
- Siguiente bloque: librería de AD5 (2.2). D4 (nombre) sigue pendiente de Antonio.

**Qué debe probar Antonio**
1. Abre la demo y **Colaboración → Comprobación CTE…**. Con «vivienda accesible» deben salir en rojo las puertas P-02 (0,70 m libres) y la rampa del 10 %; en verde los giros de dormitorios, cocina y estar.
2. Cambia el **uso** a «Administrativo»: aparece el recorrido de evacuación más largo; púlsalo y mira en la planta el recorrido naranja hasta la salida. ¿Te parece que mide lo que mediría un técnico?
3. Pon una **cama** (herramienta Objeto) en medio de un dormitorio con «vivienda accesible»: su círculo de giro debe pasar a no cumplir.
4. Selecciona un local y en **Propiedades → Ocupación (CTE)** escribe otra densidad (p. ej. 2): cambia la ocupación de la planta en la ventana.
5. Desde la terminal: `cargo run -p archisim-cli -- cte out/demo.archisim out/cte.csv` y abre el CSV en Excel.
6. Dime qué reglas faltan o qué interpretación no compartes (por ejemplo, qué usos de local debería reconocer o cómo medir la anchura libre de las puertas).

---

## 2026-10-20 (mañana) · Bloque E de familias: biblioteca de tipos (v0.30)

Tercer bloque de los pedidos el 19/10. La v0.29 (IFC 4.3) quedó fusionada en `main` con la integración continua en verde.

**Qué se hizo**
- **Ficheros de familia con catálogo**: un `.asfam` de la biblioteca es la receta seguida de su catálogo de tipos (nombre, marca, `GlobalId`, descripción, valores) y de los materiales que usan, por nombre, con categoría y color. Formato descrito en `docs/FAMILIAS.md`, «La biblioteca».
- **Guardar en la biblioteca**: Navegador → clic derecho sobre un tipo de puerta, ventana u objeto. Va al fichero de su familia en la carpeta de la biblioteca (por defecto `Documentos/ArchiSim/Biblioteca`); guardar otra vez el mismo tipo sustituye su entrada, no la duplica.
- **Ventana Biblioteca** (Vista → Biblioteca…, o el botón al pie del Navegador): lista las familias de la carpeta y sus tipos con buscador; «Cargar» trae el tipo con su familia y sus materiales (los crea si faltan) y **con el mismo `GlobalId`**. Si el proyecto ya lo tiene igual, no hace nada; si lo tiene con otros valores, pregunta: sustituir el del proyecto o cargarlo como otro tipo.
- **Actualizar familias**: en la misma ventana, cada familia del proyecto con versión más nueva en ArchiSim o en la biblioteca ofrece «Actualizar a vN». Tipos y ejemplares conservan sus valores y toman por defecto los parámetros nuevos; si algo no cabe con la receta nueva, no cambia nada. Así los proyectos antiguos pueden recibir, por ejemplo, la línea del giro de las puertas.
- **Purgar tipos sin usar** (puertas, ventanas, objetos, muros y forjados; siempre queda uno de muro y uno de forjado), con confirmación y deshacer.
- **Navegador**: puertas, ventanas y objetos agrupados por familia, con marca y número de ejemplares.
- Sin cambios de formato `.archisim` ni de IFC: el tipo cargado exporta su `IfcDoorType`/`IfcWindowType` con el mismo `GlobalId` en los dos proyectos.
- Pruebas: un tipo creado en un proyecto se carga en otro igual y con el mismo `GlobalId` (también pasando por la carpeta), cargar dos veces no duplica, conflicto/sustituir/renombrar, material que falta creado con su color, actualización de familia y receta ajena rechazada, purga, y la ventana dibujada en la app. Validación IFC (4.3 e IFC4), ida y vuelta, DXF y PDF como siempre.

**Pendiente**
- «Seleccionar todos los ejemplares del proyecto» desde el Navegador: hoy selecciona los de la planta activa.
- Tipos de muro y forjado en la biblioteca (hoy solo puertas, ventanas y objetos, que son los de familia).
- Siguiente bloque: 4.7, comprobaciones CTE (superficies útiles, evacuación DB-SI, accesibilidad DB-SUA). D4 (nombre) sigue pendiente de Antonio.

**Qué debe probar Antonio**
1. Abre la demo (`cargo run --release -p archisim-app -- out/demo.archisim` o la v0.30 instalada). En el Navegador, abre **MODELO → Puertas**: salen agrupadas por familia (Puerta abatible, Puerta corredera…).
2. Clic derecho sobre «P-02 Puerta de paso 1 hoja 80×205» → **Guardar en la biblioteca**. El aviso dice el fichero (`Puerta abatible.asfam` en `Documentos/ArchiSim/Biblioteca`).
3. **Archivo → Nuevo** y **Vista → Biblioteca…**: despliega «Puerta abatible» y pulsa **Cargar** en la P-02. Colócala con la herramienta Puerta y exporta el IFC de los dos proyectos: el `IfcDoorType` de la P-02 debe tener el **mismo GlobalId** en ambos.
4. En el proyecto nuevo, cambia el ancho de la P-02 en Tipos y materiales y vuelve a cargarla: debe preguntarte si sustituir o cargarla como otro tipo.
5. **Purgar tipos sin usar…** en la ventana Biblioteca: dice cuántos tipos se van; acepta y comprueba con Ctrl+Z que se deshace.
6. Si tienes un proyecto guardado con una versión antigua, mira en «Familias del proyecto» si alguna ofrece «Actualizar a vN» y prueba a actualizarla.

---

## 2026-10-20 (madrugada) · IFC 4.3 por defecto, georreferencia y entorno (v0.29)

Segundo bloque. **D3 decidida por Antonio: IFC 4.3**, «para poder aprovechar el tema de entorno».

**Hecho**
- **El IFC sale en IFC 4.3** (`IFC4X3_ADD2`, la versión de la norma ISO 16739-1:2024). Todo lo que ya exportaba ArchiSim valida igual en 4.3 con IfcOpenShell, reglas EXPRESS incluidas.
- **IFC4 sigue disponible** para los programas que aún no leen 4.3: Archivo → «Exportar IFC4 (compatibilidad)…»; `archisim demo`/`exportar` escriben los dos (`demo.ifc` en 4.3 y `demo-ifc4.ifc`).
- **Georreferencia del proyecto** (propiedades del proyecto, sin nada seleccionado → «Emplazamiento (georreferencia)»): sistema de coordenadas (ETRS89 UTM 29N, 30N y 31N; REGCAN95 para Canarias; WGS 84; EPSG), X e Y UTM del punto 0,0 de la planta y cota del ±0,00 sobre el nivel del mar. Muestra la latitud y longitud y la **convergencia de la cuadrícula**. En el IFC: `IfcProjectedCRS` + `IfcMapConversion` (el eje X del modelo girado con el norte real más la convergencia, para que el norte de la cuadrícula UTM quede bien) y la latitud, longitud y altitud del `IfcSite`. Se lee de vuelta al importar.
- **Entorno con las clases de 4.3**: árboles, arbustos y setos como `IfcGeographicElement` de tipo **`VEGETATION`**; el coche y la bicicleta como **`IfcVehicle`** (`VEHICLEWHEELED`). Solo la persona sigue como elemento genérico (4.3 no tiene clase para personas). Los de antes se importan igual, y al importar se reconocen también vehículos, firmes, bordillos y rellenos de 4.3 en el informe.
- Importación probada con un IFC 4.3 escrito por IfcOpenShell (el mismo edificio que el de IFC4: mismas plantas, muros, forjados, puertas y ventanas).
- Demo georreferenciada (calle Mayor 1, Madrid, ETRS89 UTM 30N, 650 m).
- Formato `.archisim` **versión 11** (la georreferencia); los de la 10 se abren tal cual. Familias de sistema Árbol, Arbusto, Seto, Turismo y Bicicleta en su versión 2 (solo cambia la clase IFC; los proyectos que ya las tienen guardan su copia anterior). Versión 0.29.0.

**Pendiente / limitaciones conocidas**
- Altura ortométrica: se escribe la cota que pongas, sin datum vertical (en España sería el de Alicante); dime si quieres que se elija.
- El terreno no se coloca aún desde coordenadas UTM de un levantamiento: sigue en coordenadas de la planta. Con la georreferencia, el paso siguiente natural es importar el levantamiento en UTM y llevarlo al 0,0 del proyecto.
- Revit y ArchiCAD: comprobar qué versión de cada uno abre bien el 4.3; si alguno falla, el de compatibilidad.

**Antonio debe probar**
1. Instala la v0.29.
2. Propiedades del proyecto (clic en vacío) → «Emplazamiento»: marca «Georreferenciado», sistema ETRS89 UTM 29N y pon las coordenadas de un solar tuyo de Punta Umbría (del catastro o del levantamiento), y su cota. ¿Salen la latitud y la longitud que esperas?
3. Exporta el IFC (4.3) y ábrelo en tu ArchiCAD y en Revit: ¿lo abren? ¿Aparece en su sitio (en ArchiCAD, «Ubicación del proyecto»; en Revit, «Ubicación»)? Si alguno no lo abre, prueba «Exportar IFC4 (compatibilidad)» y dime cuál y qué versión.
4. Si tienes BIMcollab Zoom, Solibri o un visor web (por ejemplo, el de That Open o xeokit), ábrelo ahí también: los árboles deben salir como vegetación y el coche como vehículo.

## 2026-10-20 (madrugada) · Planos completos: secciones y alzados en planos, tablas y mediciones de todo, alturas en sección (v0.28)

Primer bloque de los cinco que Antonio pidió en orden (planos completos, IFC 4.3, bloque E, CTE, librería AD5). Decisiones suyas: **D3 → IFC 4.3**; **D4** (nombre) pendiente, con una propuesta de nombres (Alarife, Tapial, Odiel…) enviada en el chat.

**Hecho**
- **Secciones y alzados en los planos.** En el panel del plano, «Añadir vista» ofrece las plantas y también cada sección y alzado. Al colocarla se calcula una vez lo que ocupa (recorte), con las cotas de nivel de la derecha enteras; luego se arrastra, se recorta y se titula como una planta («SECCIÓN A», «ALZADO SUR»). Doble clic en la vista abre la sección. Borrar una sección quita sus vistas de los planos. PDF y DXF de los planos las llevan.
- **Exportar una sección o un alzado** a PDF, DXF o SVG (Archivo → «Exportar sección o alzado», con la pestaña de la sección delante), con las fases que muestra la vista.
- **Alturas en sección**, automáticas (se apagan con «Acotar alturas» en las propiedades de la sección): la cadena de alturas entre plantas a la izquierda y, en las secciones, la **altura libre de cada local cortado**, medida en el propio corte: de la cara superior del suelo a la cara inferior de lo que haya encima (o hasta la altura propia del local, si la tiene, como un falso techo).
- **Tablas nuevas**: Pilares y vigas (nº, longitud, volumen), Cubiertas (pendientes, alero, superficie en planta e inclinada), Escaleras (tabicas, huella, ancho y si cumple el DB-SUA) y Muros cortina. En el navegador y para colocar en los planos.
- **Mediciones BC3**: capítulo **00 Demoliciones** en las reformas (muros y forjados a derribar en m², carpinterías a desmontar, pilares y vigas en m³), y lo existente que se conserva ya no se mide; capítulos nuevos de **Estructura** (hormigón y madera en m³, acero en kg), **Cubiertas** (m² inclinados), **Escaleras y barandillas** y **Muros cortina**.
- Demo: plano **A-02 «Alzado sur y sección A»** a 1:100.
- Recuperadas dos pruebas del formato que se habían perdido al juntar las ramas (capas y dibujo 2D en ficheros antiguos; planos, vistas y cajetín).
- Formato `.archisim` **versión 10** (vistas de secciones, «acotar alturas»); los de la 9 se abren tal cual. Sin cambios en el IFC. Versión 0.28.0.

**Pendiente / limitaciones conocidas**
- Acotar a mano sobre una sección (cotas propias, textos): la línea de órdenes todavía no dibuja en las secciones.
- Una vista de sección en un plano no tiene fase de reforma propia (muestra todo).
- El recorte de una sección en un plano se calcula al colocarla: si el edificio crece, quita y vuelve a poner «Recorte propio».

**Antonio debe probar**
1. Instala la v0.28 (se publica sola).
2. Abre la demo y el plano **A-02**: alzado sur y sección A a 1:100. ¿Grosores, rótulos y cotas como los pondrías en un plano de proyecto?
3. En un plano tuyo: panel «Añadir vista» → elige «Sección A» o un alzado, escala 1:50, «Colocar en el plano». Arrástrala y recórtala. Exporta el plano a PDF y a DXF.
4. Abre la Sección A: a la izquierda la altura entre plantas; en cada local, su **altura libre**. ¿Es la que medirías tú? Pon a un local una altura propia de 2,50 (falso techo) y mira cómo cambia.
5. Navegador → Tablas → **Escaleras** y **Pilares y vigas**; Archivo → Exportar mediciones (BC3) y ábrelo en Presto o Arquímedes. En un proyecto con fases, marca un tabique «A derribar»: debe salir en el capítulo 00 y no en el 01.

## 2026-10-19 (noche) · Las cuatro ramas juntas en `main`, con un solo local (v0.27)

Antonio dio luz verde para unificar los locales y fusionar en `main` las PR #19 (fase 2), #20 (revisión R1–R11), #21 (fase 3, documentación) y #22 (fase 4, colaboración). Se hizo en la rama `bloque/fusion` y, con todo comprobado, se subió a `main`.

**Hecho**
- **Un solo local.** Las ramas de la fase 2 y de la fase 3 habían hecho cada una su local (`Space` y `Room`) y las dos guardaban «versión 7». Queda el de la fase 3 (`Room`: huecos para pilares y patinillos, sigue a sus muros, acabados, etiquetas, cuadro de superficies, BC3) con lo que solo tenía el de la fase 2:
  - la **herramienta Local (Z)** de la caja, con sus tres métodos (recinto, polígono, rectángulo) y nombre, uso y altura elegidos antes de hacer clic; la orden **LOCAL** de la línea de órdenes sigue igual;
  - la **lista de usos del proyecto** (R3) en las propiedades del proyecto y el desplegable «Uso» del local;
  - **superficie construida** (a ejes de los muros que lo rodean) junto a la útil, el GlobalId, cambiar de planta, y **«Rótulo al centro»**;
  - número **único en su planta** (como pedía la fase 2) y «ese recinto ya tiene un local» al repetir clic;
  - el rótulo se arrastra por su asa, y no puede salir del local (de él se vuelve a buscar el recinto cuando cambian los muros).
- **Formato `.archisim` versión 9.** Abre los ficheros de la versión 7 de las dos ramas y los de la 8: los locales de la fase 2 pasan a ser locales sin seguir a los muros, con su altura y su rótulo, y `space_usages` pasa a `room_usages`. Comprobado con ficheros de verdad, generados con el programa de cada rama (fase 2, v7; fase 3, v7; revisión, v8): los tres se abren con sus 4 locales y su IFC valida.
- **IfcSpace unificado:** huecos, acabados (`Pset_SpaceCoveringRequirements`), altura propia o hasta la planta superior, superficies útil y construida, y `ArchiSim_Space` (rótulo, si sigue a los muros, si la altura es propia) para que la ida y vuelta salga idéntica.
- **Encajes entre fases** que ninguna rama podía ver sola:
  - las secciones y alzados usan las plumillas de su escala (a 1:100, las finas) y rotulan el título con el trazo técnico, como las plantas;
  - pilares y vigas, mobiliario, muros cortina, barandillas, marcas de sección y norte van en el DXF en capas propias (`A-ESTRUCTURA`, `A-MOBILIARIO`, `A-MURO-CORTINA`, `A-BARANDILLA`, `A-SECCION`, `A-NORTE`), no en `A-OTROS`;
  - **Visibilidad** de cada vista (y de cada vista en un plano): además de muros, cotas…, ahora se pueden ocultar pilares y vigas, cubiertas, escaleras, muros cortina, barandillas, objetos y mobiliario, terreno y marcas de sección;
  - las órdenes de edición 2D (`D` desplazar, `CP` copiar, `GI` girar) mueven, copian y giran también pilares, vigas, cubiertas, escaleras, muros cortina, barandillas, objetos, terreno y secciones (escalar y simetría siguen siendo solo para muros, forjados y dibujo 2D); un forjado inclinado se lleva su flecha de pendiente;
  - con las teclas de ArchiCAD, las letras de las herramientas nuevas (I, G, Z, S, T, E, R, U, B, O) cambian de herramienta; mientras se dibuja una viga, escalera, barandilla, muro cortina o sección, lo tecleado son medidas, no órdenes;
  - en 3D, la caja de sección (fase 2) recorta también los modelos vinculados (fase 4) a la hora de elegir;
  - soltar un DXF en la ventana lo importa como dibujo 2D (fase 3); con la herramienta Terreno activa o un terreno seleccionado, como topografía (fase 2). Un DWG, siempre como topografía (avisa de que hay que pasarlo a DXF).
- **Demo:** los locales de la fase 3 con los nombres de la fase 2 (que casan con el mobiliario: estar-comedor y cocina al norte, dormitorios al sur) y sus acabados; el texto «PLANTA BAJA» apartado de las cotas del terreno.
- El IDS de ejemplo admite mesetas de escalera como forjados. La persona y el coche del jardín salen como elementos genéricos (`IfcBuildingElementProxy`), que ese IDS prohíbe: es un hallazgo real, y la prueba lo espera.
- Versión **0.27.0**. Comprobado: `cargo fmt`, `clippy -D warnings`, todas las pruebas; demo validada con IfcOpenShell (también las reglas EXPRESS); ida y vuelta idéntica salvo la fecha; importación; DXF con ezdxf; PDF con qpdf; la casa dibujada con la app y su IFC; ventana real abierta con Xvfb.

**Decisiones tomadas por mí (cámbialas si no te convencen)**
- Un clic **dentro** de un local selecciona el local (después de los muros, antes del forjado), como en la fase 3; en la fase 2 seleccionaba el forjado. Al forjado se llega por su borde.
- Un local nuevo sin altura propia llega **hasta la planta superior**; en la última planta, 2,50 m (la fase 2 usaba 3,00 m).
- El tipo predefinido del IfcSpace es `SPACE` (la fase 3 escribía `INTERNAL`).

**Pendiente / limitaciones conocidas**
- Las **secciones y alzados no se pueden colocar en los planos** todavía (los planos solo admiten plantas); tampoco se exportan a PDF/DXF desde la app (sí a SVG desde la línea de órdenes).
- Las **tablas y el BC3** no cuentan pilares, vigas, cubiertas, escaleras ni muros cortina.
- Las vistas colocadas en un plano no tienen fase de reforma propia (muestran todo).
- Lo pendiente de cada rama sigue igual (ver sus entradas): bloque E de familias, cotas de altura en sección, holguras y reglas por disciplina en interferencias, bSDD, 4.5–4.7.

**Antonio debe probar**
1. Instala la v0.27.0 (se publica sola al subir a `main`): en el Mac, `curl -fsSL https://github.com/stronkel/archisim/releases/latest/download/instalar-mac.sh | bash`; en Windows, `ArchiSim-Windows.exe` de la página de versiones.
2. **Tus ficheros de antes:** abre cualquier `.archisim` que guardaras con las versiones de prueba de las PR #19, #20, #21 o #22. Deben abrirse con sus locales. Guárdalo y vuelve a abrirlo.
3. **Locales:** herramienta **Local (Z)**, «Recinto», elige el uso «Cocina» antes de hacer clic y pincha dentro de una habitación: sale con nombre y uso; repite el clic: «ese recinto ya tiene un local». Prueba «Rectángulo». Con un local seleccionado mira Propiedades: superficie útil y construida, acabados, «Sigue a los muros», «Rótulo al centro». Escribe `LOCAL` en la línea de órdenes y haz clic en otra habitación: debe salir el mismo tipo de local.
4. **Usos:** sin nada seleccionado, «Usos de los locales» → añade el conjunto «Oficinas»: el desplegable «Uso» de la herramienta y de cada local los ofrece.
5. **Visibilidad:** en las propiedades de la vista, quita «Objetos y mobiliario» y «Terreno»: la planta debe quedar limpia. Haz lo mismo en la vista del plano A-01 y exporta el plano a PDF.
6. **DXF:** exporta la planta baja a DXF y ábrela en AutoCAD: ¿pilares, mobiliario, barandilla y muro cortina en sus capas?
7. **Órdenes sobre elementos nuevos:** selecciona los dos pilares del porche y escribe `GI` (girar) 90: deben girar con su sección.
8. Repasa la decisión del clic dentro del local (arriba): ¿prefieres que dentro se elija el forjado, como en la fase 2?

## 2026-10-19 (mañana) · Revisión de Antonio sobre v0.25 (R1 a R11, rama `bloque/revision-19oct`)

Antonio revisó la fase 2 nocturna y pidió once cambios (apuntados en `docs/HOJA_DE_RUTA.md` como R1 a R11). Están todos hechos, en la PR #20 (sobre la rama de la PR #19).

**Hecho**
- **R1 · Líneas de ayuda de alineación** (como las guías de ArchiCAD y Revit): al dibujar cualquier cosa por puntos (muros, forjados, cubiertas, locales, cotas, escaleras, barandillas…), los puntos ya dibujados y los últimos puntos de los elementos que el cursor ha tocado (extremos, esquinas, puntos medios, el punto 0; recuerda ocho) prestan su línea horizontal y vertical. Si el cursor pasa cerca, se engancha y aparece la guía azul discontinua desde ese punto (con un circulito en él). Si además va por la línea de rastreo del tramo, se engancha justo donde se cruzan: así el último muro de una vivienda termina en línea con la primera esquina. Libre, se engancha al cruce de una guía horizontal y otra vertical. Los forzados a objeto mandan sobre las guías; se apagan con «Rastreo» en la barra inferior o manteniendo Mayúsculas. El cursor muestra «Alineado con un punto».
- **R2 · Cotas re-anclables**: con una cota seleccionada, cada punto medido es un asa (como los extremos de muro): se arrastra a otro punto (esquina, cara, extremo, punto 0…, con los forzados) y la cota pasa a medir hasta él, sin borrarla; su línea se queda donde estaba. Mientras se acota, el elemento bajo el cursor enseña con puntos azules sus vértices, los sitios donde la cota puede anclarse.
- **R3 · Usos de local propios del proyecto**: en las propiedades del proyecto (sin nada seleccionado), grupo «Usos de los locales»: la lista de usos que ofrece el desplegable del local. Se quitan con ✕, se añaden escribiendo uno nuevo e Intro, y con «Añadir conjunto» se suman de golpe los habituales de Oficinas, Docente, Sanitario, Comercial, Hostelería o Industrial (y Vivienda, la de siempre). Se guarda con el proyecto (`.archisim`, campo `space_usages`).
- **R4 · Terreno editable en planta** (sin la lista de puntos en Propiedades):
  - Herramienta Terreno con dos modos: **Puntos** (un clic, un punto a la cota escrita) y **Línea de nivel** (una curva de varios clics, Intro o doble clic para terminarla; todos sus puntos a la cota escrita, uno cada metro como mucho). Si la planta ya tiene terreno, lo que dibujas se le añade; un clic sobre uno de sus puntos le pone la cota escrita. Si no hay terreno, se crea en cuanto hay superficie: Intro con tres puntos, o al terminar una segunda línea a otra cota.
  - Con la flecha y el terreno seleccionado se ven sus puntos con su cota y las líneas dibujadas (en marrón). Clic justo sobre un punto lo elige (Mayús: añadir o quitar); clic sobre una línea elige todos sus puntos. En Propiedades, «Puntos elegidos» con **una sola casilla de cota** para todos; Supr o «Quitar del terreno» los borra (las líneas se acortan con ellos). Los puntos se siguen arrastrando para moverlos.
  - Las líneas se guardan en el `.archisim` (campo opcional `lines` del terreno) y en el IFC (`ArchiSim_Terrain.Lines`), y vuelven al importar. La demo lleva una línea a −0,05 al norte.
  - Pendiente (con R9): crear las líneas con la varita sobre un dibujo CAD importado.
- **R5 · Seleccionar en secciones y alzados**: en la pestaña de una sección o un alzado, el elemento bajo el cursor se tiñe de azul (y el cursor pasa a mano); clic lo selecciona (lo cortado manda sobre lo que se ve detrás; Mayús añade o quita; clic en vacío o Esc deselecciona) y sus propiedades salen a la izquierda como en planta, para cambiarlas desde ahí. Lo seleccionado se ve con su corte perfilado en azul y sus caras vistas teñidas. Supr lo borra. Aún no se mueve ni se estira arrastrando en la sección (eso, si lo quieres, es otro paso).
- **R6 · Encuentro de muros con forjados** (como ArchiCAD, por prioridad de capa): donde un muro y un forjado ocuparían el mismo sitio, cada capa del muro se compara con cada capa del forjado que pisa; se queda el sitio la de más prioridad (la del material o, si no tiene, la de su función: estructura 900, soporte 700, aislante 500, lámina/cámara 300, acabado 100) y, a igualdad, el forjado. La capa del muro entra en el forjado desde donde está el muro (desde abajo si está debajo, desde arriba si sigue por encima), capa a capa mientras gane, y se para en la primera que pierde. Así:
  - Un tabique hasta la cota de la planta de arriba llega a la cara inferior del hormigón: su ladrillo atraviesa el enlucido del techo (que se corta contra él) y su yeso para debajo del enlucido. No asoma por encima del forjado.
  - Un muro de dos plantas pasa por el forjado intermedio: el hormigón lo corta, y por arriba su ladrillo baja atravesando solado y mortero hasta el hormigón (el solado se corta contra el muro).
  - Si el muro no cubre todo el grueso de una capa (acaba a media capa), esa capa es del forjado.
  - Se ve en 3D, secciones y alzados, y en el IFC: el muro y el forjado llevan restado lo que pierden (`IfcBooleanResult` con prismas exactos) y sus volúmenes netos lo descuentan; IfcOpenShell da los mismos volúmenes. Al importar un IFC propio, los encuentros se recalculan (sin avisos).
  - La demo ahora sube fachadas y tabiques hasta la cota de Cubierta (como se haría en ArchiCAD) y el forjado de cubierta los recorta: en la Sección A el tabique central llega al hormigón.
  - Límites: solo muros rectos y forjados horizontales (los muros curvos y los forjados inclinados se siguen solapando); un borde de forjado oblicuo al muro corta el muro en escuadra; la cubierta inclinada sigue con «Bajo cubierta» (recorta el muro bajo toda la cubierta, sin comparar capas). El cuadro de prioridades con barras (R7) permite ajustarlas de forma visual.
- **R7 · Cuadro de prioridades con barras** (como el de ArchiCAD): en las propiedades del proyecto (sin nada seleccionado), grupo «Encuentros» → **«Prioridades de intersección…»** (también desde el material, en «Tipos y materiales»: «Ver todas las prioridades…»). Una fila por material de los muros y forjados, ordenadas de más a menos prioridad, con su color y una **barra de 0 a 999**: arrastrar o hacer clic sobre ella le da prioridad propia (de 10 en 10, barra azul); sin prioridad propia la barra es gris y llega a la de la función de sus capas (si está en capas de funciones distintas, «según capa»). En la escala de arriba, las marcas de cada función (acabado 100, lámina 300, aislante 500, soporte 700, estructura 900). «↺» devuelve un material a su función; «Todas según la función de la capa» quita todas las propias de una vez (un solo deshacer). Al pasar el ratón por el nombre, dónde se usa. Todo cambio rehace al momento las uniones en T y los encuentros con forjados en planta, 3D, secciones e IFC.
- **R8 · Galería de objetos con carpetas y vista previa** (como la biblioteca de ArchiCAD):
  - En la herramienta Objeto (y en las propiedades de un objeto), una tarjeta con el **dibujo en planta** del tipo, su ruta («Mobiliario › Camas») y un botón que abre la **galería**: a la izquierda las categorías **Mobiliario, Sanitarios, Entorno, Gente y Vehículos** con sus carpetas (Camas, Sofás y sillones, Mesas, Sillas, Armarios, Estanterías, Cocina, Electrodomésticos; Inodoros y bidés, Lavabos, Duchas y bañeras; Vegetación, Mobiliario urbano; Personas; Turismos, Bicicletas), a la derecha los dibujos, y arriba un buscador. Recuerda la carpeta abierta.
  - La galería ofrece los tipos del proyecto **y los de la biblioteca de ArchiSim que el proyecto aún no tiene** («biblioteca» en la nota): al elegir uno se carga en el proyecto (un solo deshacer). Así los proyectos antiguos o importados también tienen árboles, gente y coches.
  - Familias nuevas (recetas propias): **Estantería**, **Mueble alto de cocina** (discontinuo en planta), **Árbol** (copa y tronco; tipos de 6 m, 12 m y palmera), **Arbusto**, **Seto**, **Banco**, **Persona** (adulto y niño), **Turismo** (y furgoneta) y **Bicicleta**; además, «Sillón» y «Librería 160» como tipos nuevos de familias que ya había. Las familias llevan una línea nueva `folder` con su carpeta.
  - Árboles, personas y vehículos colocados con la herramienta **se apoyan en el terreno** que haya debajo.
  - IFC: la vegetación sale como `IfcGeographicElement` y gente y vehículos como `IfcBuildingElementProxy`, con su tipo y su familia; se leen de vuelta. El validador los comprueba como objetos.
  - La demo tiene ahora jardín: dos árboles, dos arbustos junto a la entrada, una persona entrando, un coche en la parcela y muebles altos en la cocina.
  - Límite: el 3D de las familias solo sabe hacer cajas, así que árboles y personas son «de bloques»; para copas redondas y ruedas haría falta enseñar al lenguaje cilindros y esferas (lo propongo para el bloque de familias).
- **R9 · Importar la topografía desde DXF (y CSV/TXT)**: Archivo → **«Importar topografía (DXF, CSV)…»** (o soltar el DXF en la ventana). Se abre una ventana con las **capas del dibujo** (puntos con cota, curvas, textos de cota y rango de cotas de cada una) y se marcan solas las que parecen del levantamiento; lo dibujado a cota 0 (edificios, viales) queda fuera. Lee puntos (`POINT`), bloques de punto con la cota en la inserción o en un atributo (`COTA`, `Z`…), caras 3D (TIN), polilíneas 2D con elevación y 3D (sus vértices; si están todas a una cota, son además **curvas de nivel**), líneas, y textos con número para dar cota a puntos dibujados planos («643,25» a menos de 2 m). Opciones: unidades del dibujo (si el fichero no las dice), distancia mínima entre puntos para aligerar, **qué cota del levantamiento es el ±0,00** (propone el metro entero por debajo del punto más bajo), equidistancia (propone la de las curvas del dibujo) y centrar el levantamiento en el origen del proyecto (las coordenadas UTM se quedan en un número redondo que se muestra). Crea el terreno de la planta (o lo añade al que hay), con las curvas como líneas de nivel editables (R4). Las listas de puntos CSV/TXT (con número de punto y código, coma decimal, cabecera) se leen igual, con la opción de norte primero.
  - El **DWG** es un formato cerrado: el programa explica que hay que guardarlo como DXF (o pasarlo por el ODA File Converter, gratuito). Leer DWG directamente pediría una biblioteca externa (LibreDWG, GPL); queda como propuesta.
  - De paso, la triangulación del terreno es ahora mucho más rápida (20 000 puntos en 0,02 s; antes 3 s) y completa siempre el contorno convexo (la anterior podía perder triángulos finos del borde).
  - Crate nuevo `archisim-dxf` (sin dependencias), con ficheros de prueba generados por `scripts/make_dxf_fixtures.py` (formatos R12 y R2018).
- **R10 · Una sola cubierta en L (y en T, U, cruz…) con limahoyas**, como pediste:
  - La herramienta Cubierta (Edificio, Polígono o Rectángulo) acepta ya contornos con esquinas entrantes. Los faldones se calculan con el **esqueleto recto ponderado**: cada lado avanza hacia dentro según su pendiente (los piñones no se mueven) y barre su faldón; salen **limatesas** en las esquinas salientes, **limahoyas** en las entrantes y las cumbreras donde se encuentran, con pendientes distintas por lado si las pones. El alero y el vuelo sobre los piñones siguen al faldón que llega (en esquina saliente el más bajo, en entrante el más alto). Las cubiertas convexas siguen calculándose como antes (no cambian los proyectos hechos).
  - **Mover limatesas y limahoyas a mano**: con la cubierta seleccionada, en la planta salen puntos azules donde se juntan limatesas, limahoyas y cumbrera; al arrastrar uno, los faldones que llegan a él cambian de pendiente para pasar por el punto nuevo a la misma altura (por ejemplo, alargar una cumbrera haciendo más empinado el faldón del testero, o descentrarla). Un deshacer.
  - Planta de cubiertas, 3D, secciones, IFC (un `IfcSlab` por faldón) e importación funcionan igual. «Bajo cubierta» recorta cada muro solo con los faldones que tiene encima (validado con IfcOpenShell en una casa en L).
  - Límite: un muro que pasa por debajo de una limahoya (cruzando de un faldón a otro de alas distintas) se recorta con el más bajo de los dos, no siguiendo la V; los muros de fachada y los que acaban en la esquina entrante quedan bien.

- **R11 · Tipos de muro cortina y su editor** (como el de ArchiCAD, más sencillo y legible):
  - Un muro cortina es ahora una línea con su pie y su coronación, y **todo lo demás es de su tipo**, que se guarda en el proyecto y se elige en Propiedades con su alzado (como los tipos de muro). La biblioteca trae cinco: «Montante y travesaño 120 × 150», «Fachada con antepecho opaco», «Con tapajuntas 150», «Ventanal corrido» y «Con lamas horizontales». Los proyectos de la v0.25 se abren con un tipo por cada combinación de medidas que tenían (misma geometría).
  - Ventana **Vista → «Tipos de muro cortina…»** (o «Editar tipo» desde la herramienta o el muro seleccionado, que abre con su largo y alto como muestra). A la izquierda los tipos con su miniatura y cuántos muros los usan (Nuevo, Duplicar, Borrar si no se usa); en medio el tipo, de arriba abajo:
    1. **Esquema: columnas y filas** — a lo largo y en altura, paños fijos desde el inicio o desde abajo (un módulo de puerta, un antepecho de 1,00…) y el resto «iguales, sin pasar de» una medida o «en N partes iguales».
    2. **Marcos** — marco perimetral, montantes y travesaños, cada uno con ancho visto × fondo y material.
    3. **Uniones** — montantes corridos (los travesaños van entre ellos) o travesaños corridos (ventanal corrido).
    4. **Paneles** — los tipos de panel (vidrio, antepecho…: nombre, espesor, material) y, si hay más de uno, qué panel lleva cada fila de la muestra (las de más arriba repiten la última).
    5. **Accesorios** — tapajuntas exteriores sobre cada perfil y lamas horizontales de sombra (vuelo, espesor, separación, distancia al marco, material).
  - A la derecha, **siempre al día**: alzado exterior con las medidas de paños y filas, detalle en planta, sección vertical y 3D, sobre una muestra cuyo largo y alto se pueden cambiar. Cada cambio es un paso de deshacer y lo siguen todos los muros del tipo; si alguno se quedara sin sitio entre marcos, el cambio no se hace y se avisa.
  - Planta (corta lo que atraviesa el plano de corte: marcos y tapajuntas con su trama, panel opaco con la suya, vidrio en línea; lamas vistas por debajo), 3D, secciones e IFC: `IfcCurtainWallType` con sus propiedades `ArchiSim_CurtainWallType` (se leen de vuelta, materiales por nombre, se usen o no), `IfcMember` (marcos y tapajuntas), `IfcPlate` (paneles, con el nombre de su tipo de panel) e `IfcShadingDevice` (`LOUVER`) para las lamas. Validado con IfcOpenShell (volúmenes de cada pieza).
  - Formato `.archisim` versión 8 (`curtain_types`; los muros cortina llevan `type`).
  - Límites: los paneles van todos centrados en el fondo del marco; el panel se elige por filas (no por paño suelto) y las divisiones no se repiten en patrón (fijos + resto). Si echas en falta paneles por paño, puertas en el muro cortina o esquinas entre dos muros cortina, es el siguiente paso.
**Antonio debe probar**
1. **Guías**: herramienta Muro, dibuja tres lados de un rectángulo y, en el cuarto tramo, acércate a la vertical de la primera esquina: el cursor se engancha y sale la guía azul. Pasa antes el cursor por una esquina de otro muro y luego aléjate: su línea horizontal y vertical también guían. ¿Es como lo esperabas? ¿Las quieres también a 45°?
2. **Cotas**: en la demo selecciona la cota de 12,00 de la fachada sur y arrastra su extremo derecho a la esquina interior del muro: debe pasar a medir 11,735 sin moverse la línea. Con la herramienta Cota, pasa el cursor por un muro o una ventana: ¿se ven bien los puntos donde puede anclar?
3. **Usos**: sin nada seleccionado, baja en Propiedades a «Usos de los locales», añade el conjunto «Oficinas» y escribe un uso propio («Sala de lactancia», por ejemplo). Selecciona un local: el desplegable «Uso» ofrece los nuevos. ¿Qué conjuntos echas en falta?
4. **Terreno**: en la demo, haz clic en el terreno (fuera de la casa) para seleccionarlo y luego sobre la línea marrón del norte: se eligen sus 25 puntos; escribe −200 en «Cota» y mira el 3D. Haz clic justo en un punto de la malla y dale otra cota. En un proyecto nuevo, herramienta Terreno, modo «Línea de nivel»: cota 0 y dibuja una curva, Intro; cota 1000 y otra curva más arriba, Intro: el terreno aparece con sus curvas cada 500. ¿Te sirve así para pasar un levantamiento a mano?
5. **Sección**: abre «Sección A» en el navegador, pasa el cursor por la casa (se tiñe lo que hay debajo) y haz clic en el muro cortado de la derecha: salen sus propiedades; cambia el «Desfase superior» y mira cómo se redibuja la sección. Haz clic en la ventana del fondo: se selecciona la ventana. ¿Echas en falta poder arrastrar en la sección (cambiar alturas con el ratón)?
6. **Encuentros**: abre la demo, «Sección A»: el tabique central llega al hormigón de la cubierta y el enlucido del techo se corta contra él. Prueba en un proyecto tuyo: muro de dos plantas atravesando un forjado (dibújalo en planta baja con restricción superior a la planta de arriba de la de arriba) y mira la sección: ¿se resuelve como lo haría ArchiCAD con esas prioridades? Exporta el IFC y ábrelo en ArchiCAD/Revit: los muros y forjados llevan su recorte.
7. **Prioridades**: sin nada seleccionado, Propiedades → «Encuentros» → «Prioridades de intersección…». Arrastra la barra del «Hormigón armado» a 500 (por debajo del ladrillo) y mira la Sección A: ahora el tabique atraviesa el hormigón de la cubierta. Vuelve con «↺». ¿Echas en falta algo del cuadro de ArchiCAD (por ejemplo, ordenar por nombre o filtrar por tipo)?
8. **Galería de objetos**: herramienta Objeto (tecla O): en Propiedades, pulsa el nombre del objeto y recorre Mobiliario, Entorno, Gente y Vehículos con sus carpetas; elige «Palmera» y colócala en el jardín (se apoya en el terreno). Abre un IFC importado o un proyecto antiguo: la galería ofrece igualmente toda la biblioteca. ¿Qué carpetas y objetos echas en falta para tus proyectos?
9. **Topografía**: Archivo → «Importar topografía (DXF, CSV)…» con un levantamiento tuyo en DXF (o el CSV que preparas para ArchiCAD). Revisa qué capas marca, pon la cota del ±0,00 que quieras y pulsa Importar. Mira la planta, el 3D y una sección. ¿Lee bien tus levantamientos (bloques de punto, textos de cota, curvas)? Si alguno falla, mándame el DXF (o uno parecido sin datos del cliente) y lo ajusto.
10. **Cubierta en L**: dibuja con Muro una vivienda en L, crea la planta de encima y, en planta baja, herramienta Cubierta (T) → Edificio → clic dentro: una sola cubierta con su limahoya. Mira la planta de cubiertas y el 3D. Pon a «piñón» los testeros de las alas en Propiedades. Selecciona la cubierta y arrastra el punto azul donde la limahoya llega a la cumbrera: ¿se comporta como esperabas para las geometrías especiales? Pulsa «Recortar los muros de debajo».
11. **Muros cortina**: herramienta Cortina (U): en Propiedades elige «Fachada con antepecho opaco» en la galería de tipos y dibuja un muro de 6 m; selecciónalo y pulsa «Editar tipo». En la ventana: cambia los paños a «en 5 partes iguales», añade un paño fijo de 900 al inicio, pon los travesaños corridos, activa tapajuntas y lamas, y en Paneles cambia la fila 2 a «Antepecho»; mira cada vez el alzado, la sección y el 3D. Duplica el tipo, renómbralo y dáselo a otro muro. ¿Es lo bastante claro comparado con ArchiCAD? ¿Qué echas en falta (paneles por paño, puertas en el muro cortina, esquinas)?

---

## 2026-10-19 (noche) · Fase 2 sin supervisión: pilares y vigas (2.5), locales (2.9), secciones y alzados (2.10), forjados inclinados (2.7), cubiertas inclinadas (2.4), escaleras y barandillas (2.6), terreno y norte (2.12), muros cortina (2.8), perfiles de acero (2.5), prioridades por material (2.3), puerta plegable, corredera empotrada, persiana y mobiliario (2.2), fases de reforma (2.11) y las cuatro mejoras de Antonio (v0.25, rama `bloque/fase2-noche`)

Antonio pidió avanzar «todo lo posible» en la fase 2 a partir del 2.2 mientras dormía, más cuatro mejoras pequeñas. Todo va en la rama `bloque/fase2-noche` (pull request para revisar), con commit y push tras cada avance. Esta entrada se completa conforme avanza la noche.

**Hecho**
- **Pilares y vigas (2.5)**, de punta a punta:
  - Modelo: tipos de pilar y de viga (nombre, material y sección rectangular, circular o perfil I/H), pilares (centro, giro, desfase de base y restricción superior como los muros: siguen las cotas de las plantas) y vigas (eje por dos puntos y cara superior sobre su planta; la sección cuelga). Se rechaza lo que no cabe (un pilar unido a una planta que quedaría sin altura, un alma más gruesa que el perfil…).
  - Plantilla: pilares HA 25×25, 30×30, 40×40, Ø30, HEB 160 y madera GL24h 20×20; vigas HA 30×50, plana HA 60×30, IPE 300 y madera GL24h 16×32 (materiales nuevos: «Acero S275» y «Madera laminada GL24h»). Al importar un IFC sin tipos de pilar y viga se añaden.
  - Formato `.archisim` **versión 7** (`member_types`, `columns`, `beams`); los ficheros anteriores se abren igual.
  - Planta: el pilar cortado por el plano de corte se dibuja con el relleno y la trama de su material y línea gruesa, **encima de los muros** (el hormigón atraviesa la fábrica); también los de otras plantas que el plano de corte atraviesa. Las vigas de la planta y las de la planta de encima (las del techo) se dibujan discontinuas; cortadas, rellenas.
  - 3D: pilares y vigas con el color de su material.
  - IFC: `IfcColumnType`/`IfcBeamType` con `IfcMaterialProfileSet` (perfil paramétrico `IfcRectangleProfileDef`, `IfcCircleProfileDef` o `IfcIShapeProfileDef`, compartido por el tipo), `IfcColumn`/`IfcBeam` extruidos de ese perfil, `Pset_ColumnCommon`/`Pset_BeamCommon` (`LoadBearing` = sí) y `Qto_ColumnBaseQuantities`/`Qto_BeamBaseQuantities` (longitud, área de la sección, volumen). Validado con IfcOpenShell (volúmenes incluidos).
  - Importación: pilares y vigas de cualquier programa (sección del perfil del material o de la extrusión; si no, la caja del sólido, con aviso); la ida y vuelta de ArchiSim sale idéntica.
  - Aplicación: herramientas **Pilar (I)** y **Viga (G)** en la caja. Pilar: clic en el centro; Espacio gira 90° el siguiente; por defecto llega hasta la planta superior. Viga: clic inicio, clic final, y sigue encadenando; Esc o clic derecho termina; dibujada en una planta, por defecto pertenece a la de encima (lleva su forjado, como en Revit). Fuerzan al centro y esquinas de los pilares y a extremos y punto medio de las vigas. Seleccionados: mover, copiar (Ctrl), estirar los extremos de una viga, borrar, Espacio (gira el pilar 90° / invierte la viga), cambiar de planta, y en Propiedades sus restricciones, giro, longitud, volumen y el grupo **Tipo** (nombre, material, forma y medidas de la sección, duplicar y borrar tipo), que cambia todos los elementos del tipo. En el navegador, categorías Pilares y Vigas con su número.
  - Demo: porche al sur con dos pilares HA 30×30 y una viga HA 30×50 bajo un forjado de cubierta.
- **Locales (2.9)**, como las zonas de ArchiCAD y las habitaciones de Revit:
  - Herramienta **Local (Z)**: por defecto, clic dentro de un recinto de muros y se crea el local por sus caras interiores (superficie útil); también por polígono o rectángulo. Número automático (el siguiente libre de la planta: 1, 2… o 0.01 → 0.02), nombre y uso (lista de usos habituales: Estar, Cocina, Dormitorio, Baño…; al elegir uso, el nombre lo sigue si no se había cambiado) y altura (por defecto, de la planta a la de encima). No deja crear dos locales en el mismo recinto.
  - En planta, el **sello** del local: número y nombre, y debajo «Su 12,45 m²», centrado (en un local en L, dentro de él). Se selecciona haciendo clic en el sello o en su borde (dentro del recinto sigue seleccionándose el forjado); el sello se arrastra por su asa, y «Rótulo al centro» lo devuelve. Mover, copiar (con número nuevo) y borrar como el resto.
  - Propiedades: número (único en la planta), nombre, uso, planta, altura, **superficie útil**, **superficie construida** (hasta los ejes de los muros que lo rodean), perímetro y volumen. En el navegador, «Locales (planta)» con cada local y su superficie.
  - **Rotulación técnica con letras**: el trazo propio de las cotas (UNE-EN ISO 3098) tiene ahora el alfabeto completo con ñ, tildes, ² y signos, para los nombres de locales (y las futuras etiquetas), igual en pantalla, en el SVG y en el futuro PDF.
  - IFC: `IfcSpace` agregado a su planta (número en `Name`, nombre en `LongName`, uso en `ObjectType`), extruido por su altura, con `Pset_SpaceCommon` y `Qto_SpaceBaseQuantities` (superficie útil y construida, perímetros, volumen); validado con IfcOpenShell. Se importan los `IfcSpace` de cualquier programa; la ida y vuelta de ArchiSim sale idéntica (también el sitio del sello).
  - Demo: cuatro locales (Estar-comedor, Dormitorio, Cocina, Dormitorio 2), numerados 0.01 a 0.04; las cotas de nivel se han apartado de los sellos.
- **Fases de reforma (2.11)**, como las de Revit o el estado de reforma de ArchiCAD, con la convención española de color:
  - Cada muro, forjado, puerta, ventana, pilar y viga es **Nuevo** (por defecto), **Existente** o **A derribar** (Propiedades → «Fases (reforma)», también con varios seleccionados). Las puertas y ventanas siguen a su muro al cambiarle la fase, y luego se les puede dar otra (una puerta nueva en un muro existente).
  - **Dibujar como**: en las propiedades de la vista, «Dibujar como: Existente» para levantar el estado actual; lo que se dibuje desde entonces nace existente (deshacer no lo cambia).
  - **Qué muestra la planta** (propiedades de la vista y, si el proyecto tiene fases, un desplegable «Fase» en la barra inferior): **Todo**; **Estado actual** (lo existente y lo que se derriba, sin lo nuevo); **Demoliciones** (lo que se conserva normal y lo que se derriba en **amarillo**); **Estado reformado** (lo que se conserva normal y lo nuevo en **rojo**, sin lo derribado). El 3D y la planta exportada siguen la misma elección.
  - IFC: la propiedad estándar `Status` de `Pset_WallCommon`, `Pset_SlabCommon`, `Pset_DoorCommon`… (`EXISTING`, `DEMOLISH`; sin ella, nuevo), que es como la leen Revit y ArchiCAD; se importa igual. Formato `.archisim`: lista `phases` (versión 7).
- **Secciones y alzados (2.10)**, dibujados de los mismos sólidos que el 3D y el IFC (nada se dibuja aparte):
  - Herramienta **Sección (S)** en la caja (grupo Vistas): dos clics, inicio y final; la sección **mira hacia la izquierda del trazo** (una flecha azul lo enseña mientras se dibuja) y recibe la siguiente letra libre (A, B…). Profundidad de vista elegible (2, 4, 8, 15 o 30 m; luego, cualquiera en Propiedades).
  - **Alzados**: botón «Alzados» en la caja o «Crear alzados» en el navegador: los cuatro (Norte, Sur, Este y Oeste) a 3 m del edificio, mirándolo, con la anchura y la profundidad justas para verlo entero (solo crea los que falten).
  - En planta, las **marcas**: la sección con línea de trazo y punto, tramos gruesos en los extremos y en cada extremo un círculo con su letra y la flecha hacia donde mira; el alzado, línea fina discontinua y un círculo con su inicial y su flecha. Se ven en todas las plantas; se seleccionan con un clic, se mueven, copian (Ctrl), estiran por sus extremos y borran como el resto; **doble clic abre su vista**. Propiedades: nombre, profundidad, longitud, «Abrir vista» e «Invertir» (que mire al otro lado).
  - **La vista**, en una pestaña propia (navegador → «Secciones y alzados», o el «+» de las pestañas), a 1:100 por defecto, con zoom, encuadre (Inicio), escala y «Líneas finas»: lo **cortado** con el relleno y la trama de cada material y línea gruesa, y línea fina entre capas (como en planta); lo que se ve **más allá**, hasta la profundidad, en blanco con sus aristas, y **sin líneas ocultas** (lo de delante tapa lo de detrás); el vidrio en azul claro; las **plantas** con línea de trazo y punto, su cota y su nombre. Sigue la fase elegida («Demoliciones», «Estado reformado»). En Propiedades, la altura del cursor respecto al ±0,00.
  - La línea de comandos `archisim demo` / `exportar` escribe también cada sección y alzado en SVG a 1:100 (`demo-seccion-a.svg`, `demo-alzado-norte.svg`…).
  - Formato `.archisim`: lista `sections` (versión 7). No van al IFC (son vistas, no edificio).
  - Demo: Sección A (estar y cocina, mirando al norte), Sección B (por el porche, mirando al oeste) y los cuatro alzados.
- **Forjados inclinados (2.7)**, para rampas y suelos en pendiente, con la flecha de pendiente de Revit:
  - Propiedades del forjado → grupo **Pendiente**: «Inclinado», **Pendiente (%)** (negativa: baja), **Sube hacia** (este, norte, oeste, sur) y, a título informativo, el ángulo y la cota más baja y más alta de la cara superior. «Cara superior» es la altura en el arranque de la flecha.
  - En planta, la **flecha de pendiente** con su porcentaje («10 %»), que sube en el sentido de la flecha; con el forjado seleccionado, sus dos extremos se arrastran para darle cualquier dirección (el porcentaje se mantiene). Mover y copiar el forjado se lleva la flecha.
  - Las capas mantienen su espesor perpendicular a la cara; los cantos quedan verticales, así que la planta sigue siendo el contorno. 3D, secciones y alzados, cota de nivel sobre la rampa (da la altura del punto) y el plano de corte (si corta la rampa, se ve cortada).
  - IFC: `IfcSlab` extruido verticalmente desde un perfil en ejes inclinados (lo que leen bien Revit y ArchiCAD), con superficies y volúmenes reales; validado con IfcOpenShell (volumen incluido). **Importación**: los forjados inclinados de otros programas (cara superior plana inclinada, por extrusión o por caras) entran inclinados con su flecha; antes se descartaban.
  - Demo: **rampa de acceso** a la puerta oeste, de −0,30 al ±0,00 con un 10 %.
- **Cubiertas inclinadas (2.4)**, por faldones, como la cubierta de varios faldones de ArchiCAD o la cubierta por contorno de Revit:
  - Herramienta **Cubierta (T)**: por defecto, clic dentro del edificio y la cubierta sale por las **caras exteriores de sus muros**, con 50 cm de alero, en la **planta de encima** (cubre los muros de la planta en la que se dibuja, como las vigas); también por polígono o rectángulo. Forma **a cuatro aguas**, **a dos aguas** (faldones en los lados largos, piñones en los cortos) o **a un agua**; pendiente en % (30 % por defecto); arranque (altura de la cara superior en el contorno). Tipo nuevo en la plantilla: «Cubierta inclinada de teja sobre losa HA».
  - Cada lado del contorno es **faldón** (con su pendiente) o **piñón**; los faldones se cortan entre sí en cumbreras y limatesas, y el alero y los vuelos de los piñones bajan con la pendiente. Las capas mantienen su espesor perpendicular al faldón; los cantos son verticales.
  - Propiedades: forma, pendiente de todos los faldones, alero, **cada lado** («30» o «piñón»), planta, arranque, tipo, superficie inclinada y en planta, cota de cumbrera y de alero, fase. Seleccionada: mover, copiar, arrastrar las esquinas del contorno, borrar, cambiar de planta. Navegador: «Cubiertas».
  - **Planta de cubiertas**: en la planta de la cubierta, cada faldón en tono claro con sus aristas, el alero algo más grueso, los muros bajo el alero discontinuos y en cada faldón una flecha hacia donde vierte el agua con su pendiente.
  - 3D, secciones y alzados la muestran (corte con sus capas). IFC: `IfcRoof` (tipo `HIP_ROOF`, `GABLE_ROOF` o `SHED_ROOF`) con un `IfcSlab` `ROOF` por faldón, cada uno con las capas del tipo, `Pset_RoofCommon` y `Qto_RoofBaseQuantities` (superficie inclinada y en proyección); validado con IfcOpenShell. ArchiSim la lee de vuelta como cubierta (`ArchiSim_Roof`); las cubiertas de otros programas entran como sus faldones (forjados inclinados).
  - Demo: el **porche** pasa a ser una cubierta de teja **a un agua** al 10 % que vierte hacia el sur, apoyada en la viga (que ahora sigue su cara inferior).
  - **Muros bajo cubierta** (recorte de 2.3/2.4): un muro recto puede ir «Bajo cubierta» (Propiedades → Restricciones): sube hasta la cara inferior de la cubierta y se recorta con sus faldones, así que los **piñones salen solos**. En la cubierta, botón **«Recortar los muros de debajo»** (los muros rectos de su planta y de la de debajo que quedan bajo ella). 3D, planta, secciones y alzados lo muestran; en el IFC, el cuerpo del muro recortado por un semiespacio por faldón (`IfcBooleanClippingResult`, como Revit), con su volumen exacto; validado con IfcOpenShell y leído de vuelta.
  - Demo: además del porche, una **caseta** en el jardín (cuatro tabiques bajo una cubierta de teja a dos aguas al 35 %, recortados: se ven los piñones en el Alzado Oeste).
  - Limitación: el contorno no puede tener esquinas entrantes (una casa en L lleva dos cubiertas, que se solapan en la limahoya); los muros curvos no se recortan.
- **Escaleras (2.6)**, por su **línea de huella**, con la comprobación del **CTE DB-SUA 1**:
  - Herramienta **Escalera (E)**: clic en el pie, clic en cada giro y doble clic, Intro o clic derecho en la llegada. Cada tramo de la línea es un **tramo** de la escalera; en cada giro, una **meseta** del ancho de la escalera (en L, cuadrada; en U, el trozo corto entre tramos forma parte de la meseta; en recta, una meseta intermedia si la línea tiene un punto en medio). Mientras se dibuja se ven los peldaños y una etiqueta «20 tabicas de 17,8 cm · 18 huellas de 28,0 cm ✓ CTE» (o el primer incumplimiento).
  - Llega por defecto a la **planta de encima** (sigue sus cambios de cota) o a una altura propia. **Todas las tabicas iguales**; cada tramo reparte sus huellas para llenarlo, lo más cerca posible de la huella pedida (28 cm por defecto). Losa de hormigón de 15 cm medida desde la garganta de los peldaños.
  - **CTE**: huella ≥ 28 cm, tabica 13–18,5 cm, 54 ≤ 2T+H ≤ 70 cm, ancho ≥ 1,00 m, ≥ 3 peldaños por tramo y ≤ 3,20 m por tramo (uso general); en **uso restringido** (vivienda) huella ≥ 22 cm, tabica ≤ 20 cm, ancho ≥ 0,80 m. Propiedades lo muestra con ✓/✗ y el porqué.
  - Propiedades: ancho, huella, losa, uso, planta, llegada, tabicas, huellas por tramo, mesetas, fase. Seleccionada: mover, copiar, arrastrar los puntos de su línea de huella, borrar. Navegador: «Escaleras».
  - **Planta**: contorno de tramos y mesetas, una línea en cada tabica, lo que queda por encima del plano de corte discontinuo tras la **línea de corte en zigzag**, y la línea de huella con un punto en el pie y flecha en la llegada.
  - 3D, secciones y alzados (con sus peldaños y losa). IFC: `IfcStair` (recta, de cuarto o media vuelta) con un `IfcStairFlight` por tramo (número de tabicas y huellas, tabica y huella) y un `IfcSlab` `LANDING` por meseta, `Pset_StairCommon`, `Pset_StairFlightCommon`, cantidades; validado con IfcOpenShell (volumen de cada tramo incluido). ArchiSim la lee de vuelta (`ArchiSim_Stair`).
  - Demo: **escalera exterior de acceso a la cubierta** por la fachada este: dos tramos (10 y 8 huellas) con meseta, 20 tabicas de 17,75 cm. Con un solo tramo el CTE no se cumplía (salvaba 3,55 m > 3,20 m): por eso la meseta.
  - **Barandillas** de acero a 90 cm sobre la línea de peldaños (CTE), por defecto a los dos lados (Propiedades: a los dos, izquierda, derecha o ninguna): pasamanos que sigue la pendiente de cada tramo, montantes cada metro como mucho, y en las mesetas un pasamanos horizontal en los bordes donde no llega ningún tramo. El CTE avisa si falta pasamanos (al salvar 55 cm o más) o si con 1,20 m de ancho solo hay en un lado. IFC: `IfcRailing` (`HANDRAIL`) dentro de la escalera, con `Pset_RailingCommon`.
  - Las **rampas** ya se dibujan con forjados inclinados (2.7). Pendiente: barandillas sueltas (de terraza o balcón) y de vidrio o con barrotes, peldaños compensados y de caracol, zancas de acero o madera, y ver la escalera desde la planta de llegada.
- **Terreno y norte (2.12)**:
  - Herramienta **Terreno (R)**: se escribe la cota del punto en Propiedades y se hace clic en su sitio, punto a punto (como un levantamiento); Intro o doble clic crea el terreno (tres puntos como mínimo). Los puntos se unen en triángulos (Delaunay) y forman la superficie; el terreno llega 1 m por debajo de su punto más bajo (para que las secciones lo corten con su trama).
  - **Planta topográfica**: curvas de nivel cada equidistancia (por defecto 0,50 m), acotadas («-0,30»; si son muchas, una de cada cinco más gruesa y acotada), los puntos con su cota y el borde del terreno discontinuo. En Propiedades: equidistancia, profundidad, cotas más baja y más alta y la **cota de cada punto** (editable); con el terreno seleccionado, sus puntos se arrastran.
  - Secciones y alzados lo cortan (tierra con su trama de puntos), el 3D lo muestra. IFC: `IfcGeographicElement` `TERRAIN` con superficie triangulada, validado; ArchiSim lo lee de vuelta.
  - **Norte real**: Propiedades → Proyecto → «Norte (°)», en sentido horario desde el arriba de la planta. Se dibuja la **flecha de norte** en las plantas exportadas, la brújula del 3D lo sigue y va al IFC (`TrueNorth`; también se lee al importar de otro programa).
  - Demo: parcela con 25 puntos que cae suavemente hacia el sureste (de -0,01 a -0,49) y norte a 12°.
- **Muros cortina (2.8)**:
  - Herramienta **Cortina** (tecla U): dos clics, inicio y final; el exterior queda a la izquierda del trazo, como en los muros. Mientras se dibuja se ven los montantes que tendrá.
  - El muro cortina reparte su largo en **módulos iguales** no mayores que el «módulo máximo» y su altura en **filas** no mayores que la «fila máxima» (como la retícula de Revit con «distancia máxima»). Montantes en cada línea (los de los extremos, enrasados con los extremos), travesaños entre montantes en cada fila (abajo y arriba enrasados) y un **vidrio** en cada paño, centrado en el fondo del perfil.
  - Propiedades: altura, desfase de base, módulo y fila máximos, ancho y fondo del perfil, espesor del vidrio, y la retícula resultante («4 × 2 (1.200 × 1.350 mm)»). Puede llegar **hasta una planta** («Llega a» en Propiedades), como los muros: sigue sus cambios de cota (la de la demo sigue a la planta Cubierta, con desfase hasta la viga). Se estira por sus extremos, se mueve, copia, borra y cambia de planta como lo demás.
  - Planta: montantes cortados con el color de su material y los vidrios como rectángulo fino; si el plano de corte no lo corta, su contorno visto (discontinuo si está por encima). 3D, secciones y alzados desde las mismas piezas.
  - IFC: `IfcCurtainWall` con sus `IfcMember` (`MULLION`) y `IfcPlate` (`CURTAIN_PANEL`), cada pieza con su material y cantidades (volúmenes comprobados con IfcOpenShell), `Pset_CurtainWallCommon` y `Qto_CurtainWallQuantities`; `ArchiSim_CurtainWall` para leerlo de vuelta como uno. Formato `.archisim` versión 7 (campo `curtain_walls`).
  - Demo: una mampara de vidrio cierra el lado este del porche, de la fachada al pilar y hasta la viga.
- **Perfiles de acero U, T, L y tubos (2.5)**: las secciones de pilares y vigas admiten además **UPN** (alma a un lado, alas hacia el otro), **T** (ala arriba), **angular L** de alas iguales o distintas y **tubos** rectangulares y circulares (huecos por dentro en planta, 3D e IFC: `IfcRectangleHollowProfileDef`, `IfcCircleHollowProfileDef`). Se eligen en el tipo (Sección: «Perfil U (UPN)», «Perfil T», «Angular L») con sus espesores; planta, 3D y secciones desde el contorno. IFC con sus definiciones paramétricas (`IfcUShapeProfileDef`, `IfcTShapeProfileDef`, `IfcLShapeProfileDef`, volúmenes comprobados) y lectura de vuelta, también de otros programas. Tipos nuevos en la plantilla: «Viga UPN 200», «Viga T 100», «Angular L 80×8», «Pilar tubo 100×100×5» y «Pilar tubo Ø114,3×5».
- **Prioridad de intersección por material (2.3)**, como la de ArchiCAD: en *Tipos y materiales… → Materiales*, cada material tiene «Prioridad de intersección» de 0 a 999. Vacía, manda la función de la capa (estructura 900, soporte 700, aislante 500, lámina o cámara 300, acabado 100), como hasta ahora; con número, manda el material: en las uniones en T una capa atraviesa las del otro muro de menor prioridad. Se guarda en el `.archisim` y en el IFC (`ArchiSim_Material`), y vuelve al importar.
- **La escalera en la planta de llegada (2.6)**: una escalera cuyo final está atado a una planta se dibuja también en esa planta, vista desde arriba (todos sus peldaños en continua, con su línea de huella y barandillas). En la demo, la escalera exterior aparece en la planta Cubierta.
- **Barandillas sueltas (2.6)**: herramienta **Barandilla** (tecla B): clic en cada punto, Intro/doble clic/clic derecho para terminar. Montantes en cada esquina y repartidos sin pasar de la separación pedida (1,50 m por defecto), pasamanos arriba, barra inferior a 10 cm y **barrotes que nunca dejan más de 10 cm libres** (CTE DB-SUA 1). Altura por defecto 1,10 m. Con «Vidrio» marcado (en la herramienta o en Propiedades), paneles de vidrio laminado entre montantes en vez de barrotes. En planta, el pasamanos visto y los montantes (rellenos si el plano de corte los corta); 3D, secciones y alzados desde las mismas barras. Se estira por sus puntos, se mueve, copia y cambia de planta. IFC: `IfcRailing` `GUARDRAIL` con su material, `Pset_RailingCommon` y `ArchiSim_Railing` para leerla de vuelta. Demo: barandilla alrededor de la cubierta plana, abierta donde llega la escalera exterior.
- **Objetos: mobiliario, sanitarios y cocina (2.2)**: herramienta **Objeto** (tecla O). Trece familias de sistema sin muro (`host none`), recetas de texto como las de puertas y ventanas: mesa (con 0 a 16 sillas alrededor), silla, sofá (1 a 5 plazas), cama (con almohadas y embozo en planta), armario (puertas y barra de colgar), muebles de cocina (con fregadero y placa opcionales), frigorífico, lavadora, inodoro, bidé, lavabo, plato de ducha y bañera, cada una con su símbolo en planta y su 3D. La plantilla trae 15 tipos («Mesa de comedor 160×90», «Cama doble 150×200», «Cama individual 90×190», «Muebles de cocina 240», «Plato de ducha 120×80»…). Se colocan con un clic (el lado de atrás en el punto, la flecha marca hacia dónde mira; Espacio gira 90°); se mueven, copian, giran (Espacio con la flecha) y cambian de planta; seleccionados, tienen dos **puntos de control** (como los de ArchiCAD): el del lado derecho estira el ancho y el del frente el fondo (si el tipo lo comparten otros objetos, se hace un tipo propio para no cambiarlos). En Propiedades, el tipo y sus medidas (cambian todos los objetos del tipo; «Tipo propio para este objeto» lo separa para cambiarlo solo). IFC: `IfcFurniture` (`TABLE`, `CHAIR`, `SOFA`, `BED`, `SHELF`) o `IfcSanitaryTerminal` (`TOILETPAN`, `BIDET`, `WASHHANDBASIN`, `SHOWER`, `BATH`) con su tipo, que lleva la receta, y vuelven al importar. Demo: muebles de cocina y frigorífico, sofá, mesa, una cama en cada dormitorio y un armario.
- **Puerta plegable (2.2, primera pieza de la librería)**: nueva familia de sistema «Puerta plegable» (de librillo), de 2 a 8 hojas iguales que se recogen hacia el lado de la mano; en planta, las hojas a medio plegar en zigzag con una flecha hacia donde se recogen (1:50 con cerco cortado, 1:100 solo las hojas); en 3D, las hojas cerradas. Nuevo tipo en la plantilla: «Puerta plegable 4 hojas 160×205» (marca P-05). IFC: `IfcDoor` con `FOLDING_TO_LEFT` / `FOLDING_TO_RIGHT` (y al importar, también `DOUBLE_DOOR_FOLDING`). Pasa la prueba de todas las combinaciones de opciones de las familias.
- **Puerta corredera empotrada (2.2)**: nueva familia de sistema: la hoja se esconde dentro del muro, en un armazón junto al hueco, hacia el lado de la mano. En planta, la hoja a medio meter y el armazón en discontinua; en 3D, la hoja cerrada. Tipo en la plantilla: «Puerta corredera empotrada 80×205». IFC como corredera (`SLIDING_TO_LEFT/RIGHT`). El muro aún no se aligera donde va el armazón.
- **Persiana enrollable con capialzado (2.2)**: la familia «Ventana practicable» (versión 4) tiene una sección «Persiana»: con ella marcada, el capialzado ocupa lo alto del hueco de obra (alto editable, 25 cm por defecto) y la ventana queda debajo; en planta, la persiana en discontinua por fuera del marco. Nuevo tipo en la plantilla: «Ventana practicable 2 hojas con persiana 120×150». Los proyectos ya hechos conservan su copia de la familia (sin persiana) hasta que se actualice.
- **Punto 0 del proyecto forzable** (mejora 1 de Antonio): el origen (0, 0) se dibuja como un hito (círculo con dos cuartos rellenos) y es un punto de forzado propio, «Punto 0 del proyecto», que alcanza desde algo más lejos que los demás, para empezar muros, líneas o cotas desde él.
- **Cotas azules con extremos arrastrables** (mejora 2), como los de Revit: cada cota temporal tiene dos círculos en sus extremos. El de la **referencia** se arrastra a cualquier punto que fuerce (esquina, cara, extremo de muro, punto 0…) y la cota pasa a medir hasta él; el del **elemento** salta a otra línea del propio elemento: en puertas y ventanas, la otra jamba o el **centro** del hueco; en muros, la otra cara o el **eje**. Después se escribe el valor como siempre. Las asas del elemento seleccionado (extremos de muro, esquinas) tienen preferencia sobre estos círculos. Lo elegido se mantiene mientras el elemento sigue seleccionado.
- **Líneas finas** (mejora 3): botón «Líneas finas» en la barra inferior (también en Vista y en las propiedades de la vista): todas las líneas de la planta a un píxel, sin grosor; apagado, cada línea con su grosor real a la escala de la pestaña. Para cuando haya grosores por pluma (D5), este mismo botón los alternará.
- **Caja de sección en 3D** (mejora 4), como la de Revit: botón «Caja de sección» en la barra inferior del 3D (y en las propiedades de la vista 3D). Todo lo que queda fuera de la caja se recorta; lo cortado se ve **macizo**, con la tapa del corte del color de cada material (no hueco), y con sus aristas. Cada cara tiene una **flecha azul** que se arrastra para moverla; en Propiedades, las seis caras en mm y atajos: **Todo el modelo**, **Selección** (la caja alrededor de lo seleccionado), **Planta activa** (de su cota a la de la planta de encima) y **Planta en 3D** (corta la planta activa por su plano de corte: una planta en perspectiva). Lo que queda fuera de la caja no se puede seleccionar. Al apagarla y volver a encenderla, vuelve donde estaba.
- Versión 0.25.0.

**Pendiente / limitaciones conocidas**
- Librería (2.2): falta el resto (premarco, tarjas, guías de persiana en 3D, puerta de dos hojas desiguales, corredera de hasta 4 hojas) más familias de objetos (mesas redondas, sillones, escritorio…) y crear tipos de objeto nuevos desde el diálogo de tipos.
- Muros cortina: solo rectos y con retícula regular; aún sin puertas o paneles opacos por paño, sin perfiles de distinta sección, sin unión con los muros de fábrica (se dibujan encima) y sin leer muros cortina de otros programas (se cuentan en el informe).
- Secciones: sin cotas ni textos propios todavía (3.2), sin línea de terreno, sin sombras; las secciones quebradas (con saltos) llegarán más adelante. Un alzado de un modelo grande pesa (cada cara se dibuja aparte): si la vista va lenta, se optimizará uniendo caras.
- Pilares y vigas aún no recortan muros ni forjados (en planta el pilar tapa el muro; en 3D y en el IFC se solapan) ni se unen entre sí.
- Cubiertas: solo sobre contornos convexos (una casa en L lleva dos cubiertas que se solapan; la limahoya de verdad, con una sola cubierta en L, espera a que Antonio diga cómo la quiere). Los muros curvos no se recortan bajo cubierta.
- Escaleras: falta la de caracol, el peldañeado compensado y las zancas metálicas o de madera.
- Terreno: falta importar levantamientos (DXF de curvas, CSV de puntos), según cómo le lleguen a Antonio.
- Objetos de otros programas (IfcFurniture sin la receta de ArchiSim) aún no se importan: se cuentan en el informe.
- Pilares y vigas inclinados: más adelante. Al importar de otro programa un perfil U o L, su giro de 180° (hacia qué lado mira el alma) no se distingue todavía.

**Antonio debe probar**
0. Fases (piensa en Poetas Andaluces): abre `demo.archisim`, selecciona todos los muros (ventana de selección) y ponlos en **Existente**; pon la tabiquería de un dormitorio **A derribar**; dibuja un tabique nuevo. En la barra inferior, «Fase»: mira **Demoliciones** (amarillo) y **Estado reformado** (rojo), en planta y en 3D. ¿Los colores y lo que aparece en cada estado son los que usáis en AD5? Exporta el IFC y comprueba en Revit/ArchiCAD si leen el estado (Status).
0. Locales: herramienta **Local** (tecla Z), clic dentro de cada recinto de una planta: ¿el sello (número, nombre, superficie útil) se lee bien a 1:50 y a 1:100? Cambia el uso en Propiedades y compara la superficie construida con la que medirías tú. Exporta el IFC y mira en Revit/ArchiCAD si salen como habitaciones/zonas con su número y nombre.
1. Descarga la versión de la rama (pull request «Fase 2 nocturna…», *Artifacts*) y abre `demo.archisim`: abajo, el porche con dos pilares cortados y la viga discontinua. En el navegador, Pilares (2) y Vigas (1).
2. Herramienta **Pilar** (tecla I): coloca pilares; Espacio los gira; cambia en Propiedades el tipo (galería con su sección). En la planta superior el pilar llega hasta ella: cambia la cota de esa planta en *Plantas…* y mira que el pilar la sigue (Propiedades → Altura).
3. Herramienta **Viga** (tecla G) en planta baja: une los centros de dos pilares. La viga sale discontinua (es de la planta de encima). Abre la Vista 3D.
4. Selecciona un pilar: en Propiedades → **Tipo**, cambia el ancho a 350: cambian todos los pilares de ese tipo. Prueba «Duplicar tipo» y «Perfil I / H».
5. *Exportar IFC* y ábrelo en **ArchiCAD y Revit**: ¿se ven pilares y vigas con su perfil, como pilares y vigas (no como objetos genéricos), con su material y el tipo con su nombre?
6. Pasa el cursor cerca del punto 0 (el hito de la esquina suroeste de la demo) con la herramienta Muro: debe engancharse a él («Punto 0 del proyecto» abajo a la derecha).
7. Selecciona una puerta: arrastra el círculo del extremo lejano de una cota azul hasta el punto 0 (o hasta la esquina exterior de la fachada) y el otro círculo hasta el centro de la puerta; escribe 2000 en la cota: el centro de la puerta debe quedar a 2 m de ese punto. Con un muro: arrastra su extremo propio al eje y el otro a cualquier punto.
8. Barra inferior → **Líneas finas**: la planta pasa a líneas de un píxel; otra vez, grosores reales. Prueba a 1:20 y a 1:100.
9. Vista 3D → barra inferior → **Caja de sección**: arrastra la flecha azul de la cara superior hacia abajo hasta ver el interior; gira la cámara. Prueba en Propiedades «Planta en 3D» y «Selección» (con un muro o un pilar seleccionado). ¿Las tapas del corte se leen bien como sección?
10. **Secciones**: en el navegador, «Secciones y alzados» → **Sección B** (pasa por el porche): ¿se lee como una sección vuestra? Mira el forjado de cubierta cortado con sus capas, la viga del porche cortada, el tabique y la puerta del fondo en vista. Abre **Alzado Sur**: fachada con sus dos ventanas. Cambia la escala a 1:50 y prueba «Líneas finas».
11. Herramienta **Sección** (tecla S): dibuja una línea de izquierda a derecha por tu casa (la flecha azul indica hacia dónde mira); doble clic en su marca abre la vista. Selecciónala, arrastra un extremo y vuelve a la pestaña: la sección se actualiza. Prueba «Invertir» en Propiedades.
12. **Rampa**: en `demo.archisim`, la rampa de la puerta oeste con su flecha «10 %». Selecciónala: en Propiedades → Pendiente, cambia a 8 y luego «Sube hacia: Norte»; arrastra la punta de la flecha en diagonal. Mírala en 3D y en «Alzado Oeste». Pon una cota de nivel encima (tecla N) a media rampa. Exporta el IFC: ¿Revit y ArchiCAD la leen como forjado inclinado?
13. **Cubierta**: abre un proyecto nuevo, dibuja cuatro muros en rectángulo y crea la planta de encima (*Plantas…*). En Planta baja, herramienta **Cubierta** (tecla T), clic dentro: cubierta a cuatro aguas en la planta de encima. En Propiedades cambia **Forma: a dos aguas**, la pendiente a 40 y el alero a 800; pon un lado concreto en «piñón». Pulsa **«Recortar los muros de debajo»**: los muros suben hasta la cubierta y aparecen los piñones (mira el 3D). Mira la **planta de cubiertas** (pestaña de la planta de encima), el 3D y una sección. Exporta el IFC: ¿Revit y ArchiCAD la leen como cubierta con sus faldones? ¿Cómo quieres resolver las casas en L (limahoyas): cubierta única con contorno en L, o dos cubiertas que se unen?
14. **Escalera**: en un proyecto con dos plantas, herramienta **Escalera** (tecla E) en la de abajo: clic, clic (gira 90°), clic e Intro: escalera en L con meseta. Mira la etiqueta mientras dibujas y, al acabar, el grupo «CTE DB-SUA 1» de Propiedades; prueba a acortar el tramo hasta que salte el aviso, y cambia el **Uso** a restringido. Mira la planta (línea de corte, flecha), el 3D y una sección por la escalera. En la demo, la escalera exterior de la fachada este; en la planta **Cubierta** se ve llegar (todos sus peldaños, desde arriba). Exporta el IFC: ¿Revit y ArchiCAD la leen como escalera con sus tramos y meseta?
15. **Terreno y norte**: en la demo, planta baja: curvas de nivel acotadas, puntos con su cota y la flecha de norte en la planta exportada (`demo.svg`, arriba a la derecha). Selecciona el terreno (clic en una zona vacía de la parcela) y cambia la cota de un punto: las curvas se mueven. Mira la **Sección A**: el terreno cortado bajo la casa. Prueba la herramienta **Terreno** (tecla R) en un proyecto nuevo con 5 o 6 puntos. Dime cómo te llegan normalmente los levantamientos (DXF con curvas, CSV de puntos…) para importarlos.
16. **Muro cortina**: en la demo, planta baja, la mampara del porche junto al pilar del sureste; mira el **Alzado Este** (retícula de 3 × 2) y el 3D. Selecciónala y cambia el módulo máximo a 600: la retícula se rehace. En un proyecto nuevo, herramienta **Cortina** (tecla U), dos clics. Exporta el IFC: ¿Revit y ArchiCAD lo leen como muro cortina con montantes y paneles? Dime qué modulaciones y perfiles usáis normalmente (anchos, fondos) para afinar los valores por defecto.
17. **Puerta plegable**: en un tabique, herramienta **Puerta** y en el selector de tipo «Puerta plegable 4 hojas 160×205». Mira el símbolo a 1:50 y 1:100 y cambia la mano con Espacio (las hojas se recogen al otro lado). En *Tipos y materiales…* cambia el nº de hojas (2 a 8). ¿El símbolo es el que usáis en AD5? Exporta el IFC y mira si Revit/ArchiCAD la reconocen como plegable.
18. **Persiana**: herramienta **Ventana**, tipo «Ventana practicable 2 hojas con persiana 120×150»; mira la planta (persiana en discontinua al exterior), el alzado y el 3D (capialzado arriba). En *Tipos y materiales…*, en cualquier ventana practicable, marca «Persiana enrollable con capialzado» y cambia su alto. ¿Cómo lo dibujáis en AD5 (capialzado visto en planta, guías)? ¿El capialzado debe ir dentro del hueco de obra, como ahora, o encima?
19. **Corredera empotrada**: en un tabique, tipo «Puerta corredera empotrada 80×205»; mira la planta a 1:50 (hoja a medio meter, armazón en discontinua) y cambia la mano con Espacio: el armazón pasa al otro lado. ¿Comprobamos que haya muro libre para el armazón y avisamos si no?
20. **Barandilla**: en la demo, planta Cubierta: la barandilla del perímetro con su hueco hacia la escalera; mira el Alzado Sur y el 3D. Selecciónala y cambia la altura a 900 o la separación de montantes. Herramienta **Barandilla** (tecla B) en un proyecto nuevo. Marca «Vidrio» y mira el 3D. ¿Qué otros rellenos usáis (chapa perforada, barrotes horizontales…)?
21. **Perfiles U, T, L y tubos** (también «Pilar tubo 100×100×5», hueco por dentro en planta): herramienta **Viga** con el tipo «Viga UPN 200», «Viga T 100» o «Angular L 80×8»; mira la sección transversal en una Sección y el 3D. En un tipo de pilar, cambia la Sección a «Angular L» y ajusta las alas. Exporta el IFC: ¿Revit/ArchiCAD reconocen los perfiles por su forma (UPN, T, L)?
22. **Prioridad de intersección**: en la demo, *Tipos y materiales… → Materiales*, pon al material del enlucido de la fachada prioridad 950: el ladrillo de los tabiques deja de entrar en la fachada (se queda en su cara). Bórrala y vuelve a entrar. ¿Qué valores usáis en ArchiCAD para hormigón, fábrica, aislante y acabados? Los pondría como prioridades de la plantilla.
23. **Mobiliario**: en la demo, planta baja: cocina, sofá, mesa y camas. Selecciona la mesa y cambia su largo a 2000 en Propiedades (grupo «Tipo»), o arrastra su punto de control derecho. Herramienta **Objeto** (tecla O): elige «Inodoro» o «Plato de ducha 120×80», Espacio para girarlo y clic contra la cara de un muro. Mira los símbolos a 1:50 y 1:100 y el 3D. ¿Son los símbolos que usáis? ¿Qué piezas echas en falta para un proyecto de vivienda? Exporta el IFC: ¿salen en Revit/ArchiCAD como mobiliario y aparatos sanitarios?

## 2026-10-19 · Fase 3 — documentación completa, con dibujo 2D al estilo AutoCAD (v0.25)

**Hecho** (bloques 3A a 3I de `docs/DOCUMENTACION.md`, rama `bloque/documentacion`)
- **Dibujo 2D y línea de órdenes al estilo AutoCAD**: línea de órdenes bajo el dibujo con alias en español e inglés (`L`, `PL`, `REC`, `C`, `A`, `EL`, `TX`, `SB`, `D`, `CP`, `GI`, `ES`, `SI`, `EQ`, `RR`, `AL`, `EM`, `CHA`, `MA`, `PA`, `JUN`, `DES`, `EST`, `LO`, `B`, `IP`, `DIV`, `DI`, `AA`, `LI`…), coordenadas tecleadas (`3000,2000`, `@1500,0`, `@2000<45`, distancia directa), Intro/Espacio para repetir, Esc, opciones clicables, orto (F8). Líneas, polilíneas con arcos y grosor, rectángulos, polígonos, arcos, círculos, elipses, puntos, textos de varias líneas, sombreados por recinto con tramas propias, directrices y nubes. Edición completa: desplazar, copiar, girar, escalar, simetría, alinear, desfase, recortar/alargar, empalme y chaflán, matriz rectangular y polar, partir, juntar, descomponer, estirar, longitud, igualar propiedades, dividir; distancia, área y lista. Forzados a objeto (final, medio, centro, cuadrante, nodo, intersección, perpendicular, tangente) sobre el dibujo y sobre muros y forjados. Pinzamientos para estirar. Las transformaciones valen también para muros, forjados, cotas y locales.
- **Capas de dibujo** (diálogo `CAPA`: color, grosor, tipo de línea, visible, bloqueada, imprimible) y estilo propio por elemento («PorCapa» por defecto).
- **Normas gráficas** (plumillas finas y gruesas por escala) y **visibilidad por vista**: categorías, capas y nivel de detalle de cada planta.
- **Cotas asociativas**: siguen a esquinas de muro, jambas y esquinas de forjado al mover o estirar. **Cotas angulares, de radio, de diámetro y de longitud de arco** (`ACOANGULO`/`DAN`, `ACORADIO`/`DRA`, `ACODIAMETRO`/`DDI`, `ACOARCO`/`DAR`).
- **Locales** (`LOCAL`, ficha «Local»): clic dentro de los muros; siguen a los muros; número, nombre, uso, superficies útil y construida, perímetro, altura, volumen y acabados; **IfcSpace** con cantidades y acabados, también al importar. **Etiquetas** (`ETIQUETA`, opción «Todas»): carpinterías (marca y medida), muros y locales; acompañan a su elemento.
- **Planos**: A4 a A0 con cajetín propio y datos del proyecto, vistas a escala con recorte, título y visibilidad propia, que se arrastran; dibujo y tablas sobre el papel; pestaña propia y lista en el Navegador.
- **Salida**: planta o plano en **PDF vectorial**, **DXF** (con capas, colores, grosores y tipos de línea; dibujo 2D nativo) o SVG; todos los planos en un PDF; **importar DXF** en la vista (o soltándolo en la ventana).
- **Tablas de planificación** (Navegador → Tablas): carpinterías, muros, forjados y cuadro de superficies, por planta o del edificio; CSV y en planos. **Mediciones BC3** (Archivo → Exportar mediciones).
- Arreglado de paso: en `ESCALA` y `GIRA` un número tecleado es el factor o el ángulo (antes se tomaba como distancia).
- La demo trae capas, ejes, terraza sombreada, cuatro locales con acabados, etiquetas y el plano A-01 con su cuadro de superficies; `archisim demo` escribe además DXF, PDF, CSV y BC3.
- Formato `.archisim` **versión 7** (los de la 6 se abren tal cual). IFC validado con IfcOpenShell (también las reglas EXPRESS); ida y vuelta idéntica salvo la fecha; DXF auditados con ezdxf (`scripts/validate_dxf.py`); PDF comprobado con qpdf y mirado. Versión 0.25.0.

**Pendiente / limitaciones conocidas**
- Cotas de altura en sección (llegan con las secciones, 2.10); plantillas de vista con nombre; desglose por capas en las tablas.
- En el DXF de un plano, las vistas van como líneas recortadas en espacio modelo (en mm de papel), no como ventanas de espacio papel; los textos de cotas y etiquetas van como trazos. No se ha podido abrir en AutoCAD aquí: lo valida ezdxf.
- Al importar DXF, las tramas no sólidas pasan a «Diagonal» y los sólidos 3D se dejan fuera (lo dice el aviso).
- La tabla se ve en una ventana (no en pestaña).

**Antonio debe probar**
1. Descarga la versión de esta rama (pull request «Fase 3: documentación…», *Artifacts*) y abre `demo.archisim` (`archisim demo` o la que viene).
2. **Línea de órdenes**: con el cursor sobre la planta escribe `L` Intro, clic, `@3000,0` Intro, `@0,2000` Intro, `C` (cerrar). Prueba `REC`, `C` (círculo), `EQ` sobre una línea, `RR` para recortar dos líneas que se cruzan, `EM` con radio, `MA` matriz. ¿Te resulta natural viniendo de AutoCAD? ¿Qué alias o qué comportamiento echas en falta?
3. **Capas**: escribe `CAPA`, crea una capa roja discontinua, hazla actual y dibuja; apágala y bloquéala.
4. **Cotas**: acota la fachada con la herramienta Cota y luego arrastra un muro: la cota debe seguirle. Prueba `DAN` entre dos caras de muro y `DRA` sobre un círculo.
5. **Locales**: ficha «Local» y clic dentro de una habitación; mueve un tabique y mira cómo cambia la superficie. Rellena acabados en Propiedades. `ETIQUETA` → `T` etiqueta todas las carpinterías.
6. **Planos**: Navegador → Planos → «A-01». Mueve la vista y la tabla arrastrándolas; cambia la escala de la vista; rellena los datos del cajetín en Propiedades. Crea un plano nuevo y coloca la planta a 1:100.
7. **Salida**: Archivo → «Exportar todos los planos (PDF)» y ábrelo; «Exportar este plano» en DXF y ábrelo en **AutoCAD** (capas, grosores, textos); «Exportar planta → 1:50» en DXF. Importa un DXF tuyo (un topográfico o un plano antiguo) con «Importar DXF en la vista».
8. **Tablas y BC3**: Navegador → Tablas → Cuadro de superficies → «Exportar CSV…» y ábrelo en Excel; Archivo → «Exportar mediciones (BC3)» y ábrelo en Presto o Arquímedes: ¿capítulos, partidas y líneas de medición como esperas?
9. Exporta el IFC y ábrelo en ArchiCAD y Revit: ¿aparecen los locales como zonas / habitaciones con su nombre, número, superficie y acabados?

## 2026-10-19 · Fase 4 — colaboración e ingenierías: vínculos, interferencias, BCF e IDS (v0.26)

Rama `bloque/colaboracion`, desde `main` (v0.24). Pedida por Antonio el 19/10/2026: «ve desarrollando la fase 4». Diseño y uso en `docs/COLABORACION.md`.

**Hecho**
- **Requisitos IDS 1.0** (4.4, crate `archisim-ids`): comprueba cualquier IFC2X3, IFC4 o IFC4.3 —el del proyecto o el de otro programa— contra los requisitos del cliente: clase y tipo predefinido, atributos, propiedades y cantidades (con tipo de dato, unidades y herencia del tipo), materiales, clasificaciones y pertenencia (planta, conjunto, grupo, hueco). Pasa los **334 casos oficiales de buildingSMART** y da lo mismo que ifctester (IfcOpenShell) con la vivienda de prueba.
- **Incidencias BCF 2.1 y 3.0** (4.3, crate `archisim-bcf`): temas con tipo, estado, prioridad, etiquetas, fechas, asignación, fase, enlaces, comentarios y vistas (cámara, selección, visibilidad, colores, planos de corte, líneas, imagen). Lee y reescribe idénticos los 38 BCF de prueba de buildingSMART; lo que escribe valida contra sus esquemas.
- **Vínculos IFC con versiones** (4.1, crate `archisim-coord`): cualquier IFC leído como elementos con su malla (perfiles metálicos, tubos, mallas, geometría compartida…), sus huecos y una huella de sus datos. Los volúmenes coinciden con IfcOpenShell en los **21 modelos de ejemplo de buildingSMART** y en la vivienda. Comparación de versiones por GlobalId: nuevos, borrados, movidos, cambian de forma o de datos.
- **Interferencias** (4.2): con tolerancia (10 mm), sin contar contactos ni lo que pasa por un hueco; a BCF con historial (las que siguen reciben un comentario, las que desaparecen pasan a «Resuelta»).
- **En la aplicación**: menú **Colaboración** y ventana **Coordinación** con cuatro pestañas. Vínculos en planta (cortados por el plano de corte) y en 3D con color por disciplina, transparentes u ocultos, recarga automática con los cambios en verde y naranja; clic en un elemento vinculado en 3D dice qué es. Interferencias: lista, clic para verlas en 3D **aisladas** (el resto del modelo en fantasma), pasar a incidencias. Incidencias: abrir y guardar BCF (3.0 o 2.1), aplicar la vista de cada una, estado, asignación, comentarios y **nueva incidencia desde la vista 3D** con imagen. Requisitos: abrir un IDS, ver qué cumple, clic en un fallo para seleccionar el elemento, pasar a incidencias.
- Vínculos, fichero de incidencias y tolerancia se guardan **junto al proyecto** en `<proyecto>.coordinacion.txt`: **sin cambios en el formato `.archisim` ni en el IFC** (demo validada con IfcOpenShell; ida y vuelta idéntica salvo la fecha).
- Órdenes nuevas: `comprobar-ids` (con BCF de salida), `comparar`, `interferencias`, `bcf` y `demo-coordinacion` (la vivienda con una estructura y unas instalaciones con interferencias a propósito, y requisitos IDS de ejemplo).
- Ventana real revisada con capturas (Xvfb y Vulkan por software): cortes de pilares y bajante en planta, lista de interferencias, pilar metido en la puerta aislado en 3D, incidencias abiertas con su vista. Encontrado y corregido así: con dos vínculos, pasar a incidencias daba por resueltas las del primero.
- Versión 0.26.0.

**Pendiente / limitaciones conocidas**
- `main` no se ha movido durante la sesión. Al fusionar con `bloque/documentacion` habrá conflictos pequeños en `app.rs`, `main.rs` de la app y de la línea de órdenes; con `bloque/revision-19oct`, en `app.rs`, `view3d_ui.rs` y `scene.rs` (los he probado). Los resuelvo yo cuando lleguen a `main`.
- Faltan: holguras, reglas por disciplina e interferencias entre vínculos (4.2); bSDD (4.4); BCF API con servidores; planos de corte y líneas de las vistas BCF (se guardan, no se dibujan); caja de sección en 3D. 4.5 (puente con Revit), 4.6 (trabajo en equipo) y 4.7 (CTE, espera a los locales) siguen por hacer.
- Pilares y vigas de ArchiSim (fase 2, en otra rama) entrarán solos en las interferencias en cuanto se exporten a IFC.
- Decisiones nuevas D13 a D16 en `docs/COLABORACION.md` (dónde guardar los vínculos, versión de BCF, tolerancia, orden de lo que queda).

**Antonio debe probar**
1. Descarga la versión de esta rama (pull request «Fase 4: colaboración…», *Artifacts*) o, con Rust, `cargo run --release -p archisim-cli -- demo-coordinacion out/` (deja `out/demo.archisim`, `demo-estructura.ifc`, `demo-instalaciones.ifc` y `demo-requisitos.ids`).
2. Abre `demo.archisim` → *Colaboración → Vincular IFC…* y elige los dos IFC. En planta deben verse los pilares (uno, por error, dentro de la puerta del pasillo) y el círculo de la bajante; en 3D, la estructura azul y las instalaciones celestes.
3. *Colaboración → Comprobar interferencias*: deben salir 15. Pincha la de la puerta con el pilar P-3: el 3D va allí con lo demás en fantasma. *Ver todo* lo devuelve.
4. *Pasar a incidencias* → pestaña *Incidencias*: cambia el estado de una a «En curso», asígnala, comenta, y *Guardar como…* un `.bcf`. **Ábrelo en BIMcollab, Solibri o ArchiCAD** (y en Revit con el BCF Manager guardándolo como 2.1): ¿ves las incidencias, sus vistas y los elementos seleccionados?
5. Guarda el proyecto y vuelve a abrirlo: los vínculos y el BCF deben volver solos. Cambia el IFC de estructura (o expórtalo de nuevo desde Revit con algo movido): ArchiSim debe avisar y colorear lo nuevo y lo modificado.
6. *Comprobar requisitos (IDS)…* con `demo-requisitos.ids`: debe cumplir 6 de 7 (falta la transmitancia de las fachadas). Si tienes un IDS de algún cliente o de BIMcollab, pruébalo y dime qué falla.
7. Lo más útil: **un IFC de estructura y otro de instalaciones de un proyecto real de AD5** (en local, no se suben) vinculados a su arquitectura. ¿Las interferencias que salen son las que un coordinador marcaría? ¿Sobran o faltan?

## 2026-10-19 · Familias, bloque D — diálogo de tipos al estilo ArchiCAD y tipos de línea (v0.24)

**Hecho**
- **Paletas laterales más estrechas y simétricas**: caja de herramientas y propiedades ocupan juntas lo mismo que el navegador (340 puntos por lado, ajustable arrastrando el borde de propiedades; el navegador lo sigue), así que el área de dibujo queda centrada aunque estén las tres abiertas. Iconos y rótulos de la caja algo más pequeños.
- **Diálogo «Tipos y materiales» rehecho**, al estilo de los ajustes de objeto de ArchiCAD:
  - A la izquierda, **carpetas** con los tipos agrupados por lo que no puede cambiar entre ellos: puertas y ventanas por **familia** (abatible, corredera; practicable, corredera, hueco), muros por **función** (exteriores, interiores), forjados por función (de planta, soleras, cubiertas) y materiales por **categoría**. Cada tipo con su **dibujo** (alzado de la carpintería, sección de las capas, muestra del material), su marca, medidas y nº de colocados. Buscador por nombre o marca, y un **«＋» en cada carpeta de familia** para crear un tipo nuevo de esa familia (aunque la carpeta esté vacía).
  - A la derecha, primero la **vista previa** con selector: **Alzado, Planta, Sección y 3D** en puertas y ventanas; Planta, Sección y 3D en muros; Sección y 3D en forjados. El alzado y la sección salen de las mismas piezas de la receta que el 3D y el IFC; la sección corta por la primera hoja con el muro anfitrión cortado capa a capa (con su trama) sobre el dintel y bajo el antepecho. El 3D se gira arrastrando, se acerca con la rueda y se encuadra con doble clic.
  - Debajo, **todas las opciones en grupos plegables** como la paleta de propiedades: Identidad, una por cada sección de la receta (Dimensiones, Hojas, Marco, Materiales, Representación…) y Datos.
- **Tipos de línea en las recetas**: continua, discontinua, de puntos, trazo y punto, trazo y dos puntos, trazos largos (patrones en mm de papel, escalados con el plano). Cualquier línea, polígono o arco de un símbolo puede llevarlos, y hay un tipo de parámetro nuevo, `linestyle`. La **puerta abatible** y la **ventana practicable** (versión 3) tienen «Línea del giro» en la sección Representación; en el diálogo se elige con una muestra de cada línea. Por defecto, continua (la planta sale igual que antes).
- Los huecos sin carpintería aparecen con las ventanas también al crear tipos nuevos.
- Versión 0.24.0. **Sin cambios en el formato `.archisim` ni en el IFC** (demo validada con IfcOpenShell; ida y vuelta idéntica salvo la fecha).
- **Comprobado por Antonio** (19/10/2026, 02:23): «funciona todo perfecto».

**Pendiente / limitaciones conocidas**
- Los proyectos anteriores guardan su copia de la puerta abatible y la ventana practicable versión 2, sin «Línea del giro»: la tendrán al actualizar familias (bloque E) o en proyectos nuevos.
- Del bloque D falta: símbolo 1:20 y líneas de apertura en el alzado desde la receta, y puntos de control (tiradores) en planta.
- Los nombres largos de la paleta de propiedades se recortan («Plano de corte (…»); se ven enteros al pasar el ratón.

**Antonio debe probar**
1. Descarga la versión de esta rama (pull request «Familias: diálogo de tipos al estilo ArchiCAD…», *Artifacts*).
2. Abre `demo.archisim` con las tres paletas abiertas: ¿te queda espacio cómodo para dibujar y el dibujo centrado? Arrastra el borde derecho de Propiedades: el navegador debe cambiar igual.
3. Navegador → *Tipos y materiales…* → **Puertas**: carpetas «Puerta abatible» y «Puerta corredera» con el dibujo de cada tipo. Cambia entre **Alzado, Planta, Sección y 3D**; en 3D, gíralo arrastrando.
4. En **Ventanas**, elige V-01, baja hasta **Representación → Línea del giro** y pon «Discontinua»: la vista Planta y el plano deben mostrar el giro discontinuo. Prueba el resto de tipos de línea.
5. Pulsa el **«＋»** de la carpeta «Hueco» y de «Ventana corredera»: debe crearse un tipo nuevo dentro de esa carpeta, seleccionado.
6. Revisa **Muros** (Planta/Sección/3D), **Forjados** (Sección/3D) y **Materiales** (carpetas por categoría). Dime qué opciones echas de menos respecto a ArchiCAD y qué agruparías distinto.

## 2026-10-19 · Familias, bloque C — IFC desde las familias (v0.23)

**Hecho**
- **El cuerpo IFC de cada puerta y ventana son las piezas de su receta** (las mismas cajas que ve la vista 3D), cada una con el estilo de su material. Volúmenes comprobados con el motor de geometría de IfcOpenShell (puerta de paso 80×205: 0,0168 m³ de cerco + 0,056 m³ de hoja = 0,0728 m³, igual que en ArchiSim).
- **Marca de tipo en el IFC**: en el atributo `Tag` del `IfcDoorType`/`IfcWindowType` y en la propiedad `Reference` de `Pset_DoorCommon`/`Pset_WindowCommon`, que es donde IFC pide la marca («A-1»). El tipo lleva también la **descripción** y el nombre de la **familia** (`ElementType`).
- **Los parámetros viajan en el IFC**: el tipo lleva `ArchiSim_Family` (familia, versión y la receta entera) y `ArchiSim_FamilyValues` (cada parámetro con su valor: longitudes en metros, sí/no, opciones, materiales por su nombre). Los parámetros de ejemplar, si la familia los tiene, van en la propia puerta o ventana. Cualquier programa que lea IFC los ve como propiedades.
- **Importación**: un IFC de ArchiSim rehace cada tipo con su familia (aunque sea una familia propia que el proyecto no tenía), sus valores, su marca y su descripción. Los IFC de otros programas se importan como antes, por medidas.
- La comparación de ida y vuelta comprueba ahora también marcas, descripciones, recetas, valores de tipo y de ejemplar y el funcionamiento de cada hoja. Prueba nueva con una familia inventada («Puerta de prueba», con mirilla, acabado y un parámetro de ejemplar), marca puesta a mano y descripción con comillas: vuelve idéntica, y su IFC valida con IfcOpenShell.
- Corregido de paso: importar un hueco sin carpintería creaba un material «Carpintería metálica» que no estaba en el fichero.
- Versión 0.23.0. Sin cambios en el formato `.archisim`.

**Pendiente / limitaciones conocidas**
- La geometría va en cada puerta y ventana, no compartida por tipo (`RepresentationMaps`): se probará cuando Antonio confirme cómo lo leen ArchiCAD y Revit.
- Un IFC de ArchiSim con otra versión de una familia que el proyecto ya tiene (al importar en un proyecto existente) se importa por medidas, con aviso.

**Antonio debe probar**
1. Descarga la versión de esta rama (pull request «Familias: IFC desde las recetas…», *Artifacts*) o espera a que se publique.
2. Abre `demo.archisim`, *Exportar IFC* y ábrelo en **ArchiCAD** y en **Revit**. Mira una puerta y una ventana: ¿se ven igual que antes (cerco, hoja, vidrio, montantes)? En sus propiedades de tipo: ¿aparece la **marca** (P-01, V-03…) donde la esperas? (En Revit suele salir como «Marca de tipo» o en `Reference`; en ArchiCAD, en la información IFC del tipo.)
3. Busca en las propiedades IFC del tipo `ArchiSim_FamilyValues`: ancho, alto, hojas, marco, materiales.
4. Cambia en ArchiSim la marca de un tipo a `P-10`, exporta, e *Importar IFC* ese mismo fichero: el tipo debe volver con `P-10` y la misma familia.

## 2026-10-18 · Mac: Cmd+Q pregunta antes de salir, y versiones publicadas (v0.22.2)

**Hecho**
- Antonio comprueba la v0.22.1: ya cierra sin aviso de fallo, pero **Cmd+Q cerraba sin preguntar** aunque hubiera cambios sin guardar. El menú de aplicación de macOS («Salir», Cmd+Q, «Salir» del Dock, cerrar la sesión) terminaba el programa sin avisar a la ventana. Ahora esa petición se cancela y se pasa a la ventana, que hace lo mismo que con el botón rojo: pregunta si hay cambios y, si no, sale.
- **Versiones publicadas en GitHub** (opción gratuita elegida por Antonio frente a la cuenta de desarrollador de Apple): al subir una etiqueta `vX.Y.Z` se publica la versión con `ArchiSim-Mac.dmg`, `ArchiSim-Windows.exe` e `instalar-mac.sh`. En el Mac se instala y actualiza con una línea en Terminal:
  `curl -fsSL https://github.com/stronkel/archisim/releases/latest/download/instalar-mac.sh | bash`
  Lo que baja `curl` no queda en cuarentena, así que ya no hace falta `xattr`. El script cierra ArchiSim si está abierto (preguntando si hay cambios) y sustituye la aplicación en Aplicaciones.
- La integración continua del Mac instala ahora con ese script desde el `.dmg` recién hecho, comprueba que no queda en cuarentena y que «Salir» cierra la aplicación.
- README y LÉEME del `.dmg` con la nueva forma de instalar. Versión 0.22.2. Sin cambios de formato ni de IFC.

**Pendiente**
- Fusionados en `main` los pull requests #13 a #16 (18/10/2026, a petición de Antonio); cerrados sin fusionar #10 y #11, que ya estaban en `main` por el #12. **Publicación automática**: cada vez que llega a `main` una versión de `Cargo.toml` que no está publicada, se publica sola (también se puede con una etiqueta `vX.Y.Z`). La v0.22.2 se publica así.
- Cerrar la sesión de macOS con ArchiSim abierto se cancela una vez (ArchiSim sale, pero hay que volver a cerrar la sesión). Se puede afinar si molesta.

**Antonio debe probar** (en el MacBook)
1. Instala la versión de esta rama desde Actions (con el `xattr`, por última vez).
2. Dibuja un muro y pulsa **Cmd+Q**: debe preguntar si guardas. Prueba «Cancelar» (sigue abierta) y luego «No guardar» (se cierra sin aviso de fallo).
3. Sin cambios, Cmd+Q cierra directamente. Repite con clic derecho en el icono del Dock → Salir.
4. Cuando esté publicada la versión, pega la línea de `curl` en Terminal y ábrela desde Aplicaciones: no debe pedir nada.

## 2026-10-18 · Mac: cierre sin aviso de fallo (v0.22.1)

**Hecho**
- Antonio informa de que, en su MacBook Pro con Touch Bar (Intel, macOS 15.8), cada vez que cierra ArchiSim macOS muestra el aviso de que la aplicación se ha cerrado inesperadamente. El informe de fallo lo sitúa en la Touch Bar: al destruir la ventana, AppKit intenta quitar un observador de la Touch Bar de una ventana ya cerrada y lanza una excepción. No pasa en Macs sin Touch Bar (por eso no salía en la integración continua) ni se pierde nada: ocurre después de guardar o descartar.
- Arreglo: en macOS, cuando ya se puede cerrar (proyecto guardado o descartado), la aplicación termina en el acto sin desmontar la ventana, así que la Touch Bar no llega a hacer esa limpieza. Windows y Linux, sin cambios.
- Versión 0.22.1. Sin cambios de formato ni de IFC (demo validada con IfcOpenShell).
- **Comprobado por Antonio** en su MacBook (18/10/2026, 22:15): ya cierra sin aviso de fallo.

**Pendiente**
- La cuarentena de macOS (`xattr` en cada versión): solo desaparece de verdad firmando y notarizando con una cuenta de desarrollador de Apple (99 USD al año); la alternativa gratuita es publicar versiones (*Releases*) e instalarlas con una sola orden que no deja cuarentena. Decide Antonio.
- Comprobar en el Mac que Cmd+Q con cambios sin guardar pregunta antes de salir (el menú de aplicación de macOS podría cerrar sin preguntar).

**Antonio debe probar** (en el MacBook)
1. Instala la versión de esta rama (Actions → ArchiSim-Mac) como siempre, con el `xattr` de momento.
2. Abre ArchiSim, dibuja un muro, guarda y ciérrala con el botón rojo: no debe salir ningún aviso de fallo.
3. Ábrela otra vez, dibuja algo y ciérrala sin guardar: debe preguntar; elige «No guardar»: tampoco debe salir aviso.
4. Repite el paso 3 con **Cmd+Q** y dime si pregunta antes de salir.

## 2026-10-18 · Repositorio público con licencia GPL-3.0

**Hecho**
- Antonio hace público el repositorio (los minutos de la integración continua del plan gratuito se habían agotado; en un repositorio público son gratis) y elige la licencia **GPL-3.0 o posterior** (D4, a falta del nombre definitivo).
- Revisado antes todo el historial: sin claves, sin IFC de proyectos reales, sin contenido de Autodesk ni Graphisoft.
- Fichero `LICENSE` (texto oficial de la GPL-3.0), licencia y repositorio en la descripción de cada paquete, `docs/FUNDACIONAL.md` §8 y hoja de ruta (D4).
- README nuevo para la comunidad: qué hace hoy, cómo instalarlo y compilarlo, cómo colaborar (sala limpia incluida) y licencia.
- Se hace directamente en `main`; los pull requests #13, #14 y #15 lo reciben sin conflictos (comprobado).

**Pendiente**
- Nombre definitivo (resto de D4) antes de anunciarlo.
- Publicar versiones (*Releases* de GitHub) para que se pueda descargar sin cuenta.

**Antonio debe probar**
1. Abre la página del repositorio en GitHub: debe salir el README nuevo y, a la derecha, «GPL-3.0 license».

## 2026-10-18 · Bloque 22 — Rendimiento con modelos grandes (1.9); fase 1 cerrada

**Hecho** (petición de Antonio: «termina todo lo que falta de la fase 1»)
- **Medir**: comando nuevo `archisim medir [n] [carpeta]`. Repite la vivienda de prueba en una rejilla de n × n y mide cada trabajo: planta entera, planta en pantalla (una zona y todo a la vista), 3D, geometría de un muro, varita mágica, guardar y abrir, exportar e importar el IFC. Deja `grande.archisim` y `grande.ifc` en la carpeta para abrirlos en la aplicación (con `ARCHISIM_MEDIR_IMAGENES=1`, también lo que se vería en pantalla). Una prueba aparte mide lo que hace la aplicación con el ratón sobre ese modelo.
- **Uniones**: cada muro buscaba sus vecinos entre todos los muros del modelo (el coste crecía con el cuadrado del número de muros). Ahora se buscan en una rejilla de celdas de 4 m: la geometría de un muro cuesta lo mismo en una casa que en una urbanización. Lo usan la planta, el 3D, el IFC y la varita mágica (que ya no recorre todo el modelo: busca en un radio que crece hasta encontrar el recinto).
- **Aplicación**: la geometría de todos los muros se calcula una vez por cambio del modelo y se reutiliza para forzar, seleccionar, el asa bajo el cursor, las cotas temporales y la vista previa de los arrastres (que solo compara lo que está cerca de lo arrastrado). Las puertas y ventanas de cada muro se buscan una sola vez.
- **Planta en pantalla**: solo la zona visible; al alejarse mucho se deja fuera lo que no se distinguiría: tramas y zigzag del aislamiento, las capas (el muro se dibuja como una banda gris, con sus líneas de corte) y el símbolo de las puertas y ventanas pequeñas (queda el hueco en el muro). Al acercarse vuelve todo. **Las plantas exportadas en SVG no cambian** (comprobado: idénticas a las de la v0.21, y el IFC también, salvo los `GlobalId` nuevos de la demo).
- Escritura de números del SVG más rápida (mismo resultado, con prueba).
- Cifras (servidor de pruebas, versión optimizada), rejilla **30 × 30: 6300 muros, 9000 huecos, 1800 forjados**:

  | Trabajo | Tiempo |
  |---|---|
  | Planta en pantalla, zona de 30 × 20 m | 0,07 s |
  | Planta en pantalla, todo a la vista | 0,5 s |
  | Forzado / elemento bajo el cursor | 10 ms / 8 ms |
  | Vista previa de mover un muro | 15 ms |
  | Mover un muro (con la geometría de todo rehecha) | 0,1 s |
  | 3D de todo el modelo (2,8 millones de triángulos) | 0,8 s |
  | Planta entera 1:50 exportada (SVG de 80 MB) | 1,5 s |
  | Guardar / abrir el proyecto (4,3 MB) | 0,09 s / 0,2 s |
  | Exportar el IFC (56 MB) / importarlo | 0,9 s / 3,8 s |

  Antes, en la 20 × 20 (2800 muros), la geometría de un muro tardaba ~9 ms y cada movimiento del ratón la recalculaba para todos los muros de la planta (segundos); la planta con todo a la vista tardaba 1,5 s y ahora 0,4 s.
- El IFC de 56 MB pasa la validación de IfcOpenShell.
- Pruebas nuevas: la geometría guardada coincide con la calculada antes y después de mover y de deshacer; la planta muy alejada dibuja bandas sin capas ni símbolos, y de cerca es idéntica a la de papel.
- Versión 0.22.0. **Con esto la fase 1 (§1 de la hoja de ruta) queda completa.**

**Pendiente / limitaciones conocidas**
- La **integración continua no se ha ejecutado** en los pull requests #13 y #14 (ni en este): GitHub no arranca los trabajos porque la cuenta tiene un **pago fallido o el límite de gasto agotado** (aviso en cada trabajo). Hasta que se resuelva en *Settings → Billing and plans* de GitHub no hay instaladores para descargar ni comprobación en Windows y Mac. Todo lo demás se ha comprobado aquí (Linux).
- En el 3D, cada cambio rehace la escena entera (0,8 s con 6300 muros): suficiente por ahora; si molesta, se rehará solo lo que cambia.
- Al alejarse mucho, todos los muros se ven del mismo gris (no por material).

**Antonio debe probar**
1. Resuelve el aviso de facturación de GitHub y vuelve a lanzar los trabajos de los pull requests (*Actions → Re-run all jobs*); descarga ArchiSim-Windows o ArchiSim-Mac de **Artifacts**.
2. Genera el modelo grande: `archisim medir 20 carpeta` (o `cargo run --release -p archisim-cli -- medir 20 out/medir`) y abre `grande.archisim` con la aplicación.
3. Aléjate del todo (botón de encuadre): debe verse la urbanización entera con muros grises, sin esperas largas. Acércate a una vivienda: capas, tramas y puertas como siempre.
4. Con la flecha, pasa el ratón por encima de muros y puertas (resaltado al momento), selecciona un tabique y arrástralo: la vista previa debe seguir al cursor sin tirones. Dibuja un muro con forzados a otros muros.
5. Exporta el IFC (unos 25 MB con la 20 × 20) y ábrelo en ArchiCAD o Revit; importa un IFC grande de un proyecto tuyo y dime cuánto tarda.

## 2026-10-18 · Bloque 21 — Instalador de Windows (1.8)

**Hecho**
- **Icono e información de versión** dentro de `ArchiSim.exe` (icono `.ico` generado con el resto de iconos por `scripts/make_icon.py`).
- **Instalador** `ArchiSim-<versión>-Windows.exe` (Inno Setup, `scripts/windows_installer.iss`): se instala para el usuario sin pedir permisos de administrador (se puede elegir instalar para todos); entrada en el menú Inicio, icono en el escritorio opcional, carpeta «Ejemplos de ArchiSim» con la vivienda de prueba y los IFC de ejemplo.
- **Tipos de fichero**: los `.archisim` se abren con ArchiSim y llevan su icono; los `.ifc` lo ofrecen en «Abrir con» y, si se marca en el instalador, se abren con él al hacer doble clic.
- El desinstalador quita el programa, los accesos y los tipos de fichero, y no toca los proyectos.
- La integración continua monta el instalador, lo instala en silencio, comprueba los tipos de fichero y lo desinstala.
- Versión 0.21.0.

**Pendiente / limitaciones conocidas**
- Sin firma de código: la primera vez, Windows SmartScreen pide «Más información → Ejecutar de todas formas».
- Pendiente de la primera ejecución en la integración continua (ver bloque 22: la cuenta de GitHub no arranca los trabajos).

**Antonio debe probar**
1. En un Windows: descarga ArchiSim-Windows de **Artifacts** del pull request «Instalación: instalador de Windows…» y ejecuta el instalador.
2. Menú Inicio → ArchiSim abre el programa; «Ejemplos de ArchiSim» tiene la vivienda de prueba.
3. Doble clic en un `.archisim` lo abre en ArchiSim (con su icono); botón derecho en un `.ifc` → «Abrir con» → ArchiSim.
4. Desinstala desde Configuración → Aplicaciones: desaparecen el programa y los accesos, y los proyectos siguen ahí.

## 2026-10-18 · Bloque 20 — Importación IFC, segunda vuelta (1.7), huecos de forjado y huecos sin carpintería

**Hecho** (petición de Antonio: «termina todo lo que falta de la fase 1»)
- **Muros de varios tramos**: un `IfcWall` con eje en polilínea, o en curva compuesta de rectas y arcos, se importa como un muro por tramo, unidos en sus esquinas (un muro en L son dos muros a inglete). El primero conserva el `GlobalId`; los demás reciben uno derivado, el mismo en cada importación. Las puertas y ventanas van al tramo en el que están.
- **Muros circulares cerrados** (eje `IfcCircle`, o un arco que da la vuelta entera): dos medias circunferencias.
- **Muros curvos de otros programas**: también con el arco recortado por ángulos en grados (como Revit) y capas centradas en un eje que no pasa por el origen del muro (corregido de paso: el desfase de las capas se mide desde el eje, esté donde esté).
- **Huecos de forjado** (escaleras, patinillos), nuevos en el modelo:
  - Herramienta Forjado → «**Hueco en un forjado**» (con Polígono o Rectángulo): el contorno se recorta del forjado de la planta que lo contiene. Se rechaza lo que se sale del forjado o pisa otro hueco.
  - Propiedades del forjado: lista de huecos con su superficie y «Quitar». La superficie descuenta los huecos. Mover y copiar el forjado lleva sus huecos. Un clic dentro del hueco no selecciona el forjado.
  - Planta: el hueco vacío con su **aspa** (discontinua si el forjado queda por encima del plano de corte); trama recortada. 3D: el hueco atraviesa todas las capas con sus caras. IFC: huecos del perfil (`IfcArbitraryProfileDefWithVoids`), superficie y volumen brutos y netos.
  - Importación: los huecos del perfil y los `IfcOpeningElement` que vacían el forjado (como exportan Revit y ArchiCAD).
- **Huecos sin carpintería** («hueco», abertura de muro de Revit): familia de sistema **Hueco**, marca H-01…, solo con hueco de obra. Se coloca con la herramienta Ventana (la plantilla trae «Hueco de paso 90×210»). Vacía el muro con sus mochetas; nada dentro. IFC: el `IfcOpeningElement` solo, con su propio `GlobalId` y el tipo en `ArchiSim_Void`; se importa (de ArchiSim, como el mismo tipo; de otros programas, «Hueco 90×200» por tamaño). La vivienda de prueba tiene uno, entre la cocina y el dormitorio.
- **Formato `.archisim` versión 6**: `holes` en los forjados; los ficheros de la versión 5 se abren tal cual.
- **Fichero de prueba nuevo** de IfcOpenShell, `ifcopenshell_formas.ifc`, escrito entidad a entidad como lo hace Revit (IFC2X3, milímetros): muro en L, cuarto de círculo con ángulos en grados, muro redondo, forjado con escalera en el perfil y forjado con patinillo como `IfcOpeningElement`. Se importa sin nada fuera y vuelve igual.
- El validador acepta huecos sin relleno marcados `ArchiSim_Void` (con sus cantidades).
- Pruebas nuevas: núcleo (huecos de forjado: dentro, cruces, solapes, contorno que los deja fuera, guardar), 3D (forjado con dos huecos: volumen y piel cerrada), fichero (huecos y familia Hueco, versión 5), importación (formas de Revit, hueco sin carpintería de la casa de IfcOpenShell) y ventana con clics (hueco de forjado, rechazo, selección, mover, quitar; hueco de paso con la herramienta Ventana, ida y vuelta por IFC).
- Pasan a la fase 2, porque necesitan elementos que aún no existen: **forjados inclinados** (2.7), **pilares y vigas** (2.5) y **locales** `IfcSpace` (2.9). Siguen en el informe de importación.
- Versión 0.20.0.

**Pendiente / limitaciones conocidas**
- Los vértices de un hueco de forjado no se editan con el ratón: se quita y se vuelve a dibujar.
- Los huecos sin carpintería salen en la galería y en el navegador junto a las ventanas.

**Antonio debe probar**
1. Descarga la versión del pull request «Importación IFC, segunda vuelta…» → **Artifacts** → ArchiSim-Windows o ArchiSim-Mac. Abre la vivienda de prueba: el **hueco de paso** entre la cocina y el dormitorio.
2. **F** → Método Rectángulo → marca «Hueco en un forjado» y dibuja un rectángulo dentro de la solera: aspa en planta, agujero en 3D. Selecciona la solera: el hueco está en Propiedades con «Quitar».
3. **V** → en la galería, «Hueco de paso 90×210», antepecho 0, clic en un tabique.
4. Importa un IFC real de ArchiCAD o Revit con una **escalera o un patinillo** y algún **muro en L o curvo**: los huecos deben salir abiertos y los muros, partidos por tramos.
5. Exporta y abre el IFC en ArchiCAD/Revit: el hueco de forjado y el hueco de paso deben verse abiertos.

## 2026-10-18 · Bloque 19 — Modos de construcción y muros curvos

**Hecho** (petición de Antonio al revisar 1.7: «otras formas de modelar muros, no solo de punto a punto: curvos, un cuadrado cerrado o un círculo; y forjados rectangulares por dos esquinas»)
- **Método de la herramienta Muro** (Propiedades → «Método», con iconos, como el método geométrico de ArchiCAD):
  - **Recto**: un muro por cada dos clics. **Cadena**: como hasta ahora (por defecto).
  - **Rectángulo**: dos esquinas opuestas → cuatro muros en sentido horario (la cara exterior queda fuera), unidos a inglete. También primera esquina + `6000x4000` + Intro (hacia el cuadrante del cursor).
  - **Arco**: inicio, fin y un punto por el que pasa (como «inicio-fin-radio» de Revit). En el tercer paso se puede teclear el **radio** (hacia el lado del cursor). Los arcos se encadenan; Esc quita el final del arco, otra vez Esc termina.
  - **Círculo**: centro y un punto de la línea de referencia (o radio tecleado): dos medias circunferencias en sentido horario.
  - Un rectángulo o un círculo es **un solo paso de deshacer** («Crear muros»).
- **Método de la herramienta Forjado**: Polígono · **Rectángulo** (dos esquinas, o `6000x4000`) · Varita mágica.
- **Muros curvos de punta a punta**:
  - Planta: capas, tramas y zigzag del aislamiento siguen el arco; caras con línea de corte; línea de referencia curva.
  - Uniones: inglete con muros rectos o curvos que comparten extremo (tangentes: corte radial, por ejemplo las dos mitades de un círculo); un tabique que acomete a un muro curvo se detiene en su cara (y un muro curvo que acaba dentro de otro, también). Aún no entran por capas.
  - 3D: una piel cerrada por muro, sin caras entre capas. Varita mágica dentro de un recinto curvo (cara interior, eje y cara exterior).
  - Forzado: **centro** (nuevo, círculo verde), extremos, esquinas, punto medio del arco y de cada cara, cualquier punto de las caras.
  - Edición: mover, copiar y estirar conservan la curvatura; un **círculo blanco** en el punto medio del muro seleccionado lo **curva** arrastrándolo (en línea con los extremos queda recto). Propiedades: **Radio** editable (los extremos no se mueven), ángulo y **Enderezar**. Longitud editable (la curva se escala).
  - Se rechaza un arco más cerrado que el espesor del muro, y un tipo que al engordar lo dejaría así.
  - **IFC**: eje `IfcTrimmedCurve` sobre `IfcCircle`; cuerpo extruido de un `IfcIndexedPolyCurve` con **arcos exactos** (`IfcArcIndex`); longitud, superficie y volumen del arco exacto. El validador afina el mallado de IfcOpenShell para comparar volúmenes de curvas.
  - **Importación** (parte de 1.7): muros curvos de un arco desde ArchiSim, ArchiCAD o Revit (`IfcTrimmedCurve`/`IfcCircle` o `IfcIndexedPolyCurve` con arco), con capas medidas en radial. Ida y vuelta idéntica comprobada.
  - **Formato `.archisim` versión 5**: campo `bulge` en los muros curvos (los rectos no lo escriben); los ficheros de la versión 4 se abren tal cual.
- Pruebas nuevas: arcos en el núcleo (comba, radio mínimo, curvar, puertas rechazadas), geometría (habitación redonda: áreas, capas, arcos exactos, volúmenes, varita; tabique contra cara curva; inglete recto-curvo y tangente), planta curva, IFC de ida y vuelta, herramienta Muro por modos, forjado rectangular y la ventana manejada con clics (rectángulo, círculo, arco, rectángulo tecleado, forjado rectangular, curvar con el ratón, puerta rechazada, guardar/exportar/importar). **Ventana real** revisada con capturas (Xvfb): selector de método, vista previa del arco con radio y longitud, círculo con radio tecleado, Propiedades de un muro curvo y curvarlo arrastrando.
- La demo da **el mismo IFC y las mismas plantas SVG** que la v0.18 (muros rectos sin cambios). IFC validados con IfcOpenShell (demo, sesiones de la app y `modos.ifc` con curvos).
- Versión 0.19.0.

**Pendiente / limitaciones conocidas**
- Puertas y ventanas en muros curvos (y su símbolo curvo); cotas temporales de muros curvos.
- Uniones en T por capas contra muros curvos (ahora el que acomete se detiene en la cara); tabiques que siguen a un muro curvo al moverlo (sí, por su cara); un muro curvo no «resbala» como seguidor.
- Forjados con bordes curvos o circulares (la varita da un polígono fino que sigue la curva); rectángulo girado, arco tangente, polígono regular.
- Resto de 1.7: muros de varios tramos, forjados inclinados y con huecos, huecos sin carpintería, `IfcSpace`.

**Antonio debe probar**
1. Descarga la versión del pull request «Modos de construcción…» (rama `bloque/modos-construccion`) → **Actions** → última ejecución en verde → **Artifacts** → ArchiSim-Windows o ArchiSim-Mac. Proyecto nuevo.
2. **M** → Propiedades → Método **Rectángulo**: clic en una esquina y en la opuesta. Cuatro muros cerrados; Ctrl+Z los quita de una vez. Repite con la primera esquina y escribe `8000x6000` + Intro.
3. Método **Círculo**: clic en el centro, escribe `3000` + Intro (radio de la línea de referencia). Mira la planta a 1:50 y en 3D (F3).
4. Método **Arco**: clic en el inicio, clic en el final y mueve el ratón: verás radio y longitud; clic para fijarlo (o escribe un radio + Intro).
5. **F** → Método **Rectángulo**: forjado por dos esquinas. Prueba también la varita dentro del círculo.
6. Selecciona un muro curvo: arrastra el **círculo blanco** del centro para curvarlo más o menos; en Propiedades cambia el **Radio** y prueba **Enderezar**.
7. Dibuja un tabique recto desde dentro del círculo hasta su cara: debe acabar contra la cara interior.
8. Exporta el IFC y ábrelo en **ArchiCAD y Revit**: ¿los reconocen como muros curvos (con su multicapa) o como objetos? Es lo que más me interesa saber de esta versión.
9. Dime qué método echas en falta (rectángulo girado, arco tangente, polígono regular, forjado circular) y si quieres puertas en muros curvos antes del bloque C.

## 2026-10-18 · Bloque 17 — Ayudas al dibujo (cotas temporales, rastreo, giros, otras plantas)

**Hecho** (petición de Antonio: «funciones de calidad de vida antes de las herramientas del bloque 2»; las seis, en este orden)
1. **Cotas temporales azules** (como Revit). Al seleccionar un muro o una puerta o ventana —y también el último muro dibujado y la puerta o ventana recién colocada, que quedan seleccionados— aparecen cotas azules:
   - **Puerta o ventana**: de cada jamba (hueco de obra) a la referencia más cercana de ese lado, medida en la cara hacia la que abre: la cara de un muro que la cruza o acomete (tabique), el final del muro (esquina interior) o la jamba del hueco vecino.
   - **Muro**: su longitud (línea de referencia) y la **luz libre de cara a cara** hasta el muro paralelo más próximo a cada lado.
   - **Clic en el número** → se escribe la medida (mm, o `1,2m`, `120cm`; el valor sale seleccionado y se sustituye al teclear) → **Intro**: el elemento se recoloca para que la cota sea exacta. La puerta se desliza por el muro; el muro se desplaza en perpendicular (los muros unidos le siguen, como al moverlo) o se alarga desde su final (la esquina le acompaña). Un paso de deshacer («Mover» / «Estirar»). **Esc** deja la cota como estaba. Lo que no cabe se rechaza con el motivo.
   - Solo con la flecha o con la herramienta que acaba de dibujar el elemento (las demás herramientas conservan sus clics).
2. **Puntos medios**: además del centro de la línea de referencia, ahora fuerzan el punto medio de **cada cara de muro** (también el centro del canto en los extremos) y de **cada arista de forjado**, con su triángulo verde.
3. **Muros rectos por defecto**: dibujando (muros, forjados, cotas) o arrastrando, la línea queda **horizontal o vertical** mientras el cursor no se aparte más de **8°**, y una **línea azul discontinua** a lo ancho de la vista lo indica; cerca de 45° también se bloquea. Si el punto cae sobre una **cara**, se desliza por ella hasta quedar **en escuadra**: un tabique dibujado hasta una cara acaba recto aunque el clic no sea exacto. Para inclinar un muro basta apartar el cursor más de 8°; Mayús sigue forzando múltiplos de 45°.
4. **Girar puertas y ventanas** (como Revit): con una seleccionada, **Espacio** recorre sus cuatro posiciones (bisagras a la otra jamba, luego la hoja al otro lado…). Dos **flechas azules** junto a ella (en el lado al que no abre): ⇅ la pasa **al otro lado del muro** con las bisagras en la misma jamba (la mano UNE cambia de nombre, porque se mira desde el otro lado) y ⇄ **cambia la mano**. Con la herramienta Puerta o Ventana, Espacio cambia la mano de la que vas a colocar. De regalo, como en Revit: Espacio con muros seleccionados los **invierte**.
5. **Cambiar de planta**: en Propiedades, «**Restricción de base**» del muro y «**Planta**» del forjado son ahora una lista de plantas (también con varios elementos seleccionados). El elemento pasa a esa planta con sus desfases; las puertas y ventanas van con su muro (en sus Propiedades se ve en qué planta están). Un muro que llegaba **a la planta de encima** sigue llegando a la de encima de la nueva (en la última planta queda con su altura, desconectado); unido a otra planta, la conserva, y se rechaza si no queda por encima. Un paso de deshacer («Cambiar de planta»).
6. **Otras plantas en planta**:
   - Los **muros de otras plantas que atraviesa el plano de corte** se dibujan **cortados**, con sus capas, tramas y huecos (por ejemplo, un muro de la planta baja con un desfase superior de 2 m que sube por la primera), en pantalla y en la planta exportada, como el rango de vista de Revit. También fuerzan.
   - **Calco de la planta inferior** (activado por defecto en pantalla; *Calco* en la barra inferior, en las propiedades de la vista o en *Vista*): los muros de la planta de debajo, rellenos en gris azulado claro con su contorno, y la posición de sus puertas y ventanas. **Sus puntos fuerzan** (extremos, esquinas, puntos medios, caras), para dibujar encima. No se exporta en la planta SVG.
- Ayudas en la pastilla de teclas y en *Ayuda → Ratón y teclado*.
- Pruebas nuevas: forzado (puntos medios de caras y aristas, la cara que lleva el punto, rastreo por ángulo, deslizamiento hasta la escuadra sobre una cara), cotas temporales (puerta a las esquinas interiores, teclear y deslizar, rechazo, longitud y luz libre de un muro, mover y estirar), giros (las cuatro posiciones de Espacio, las flechas conservan jamba o lado), núcleo (cambiar de planta muros y forjados, restricción superior relativa, rechazo sin cambios), planta (muros de otra planta cortados, calco solo si se pide) y la ventana manejada con clics y teclas simulados (muros rectos por defecto, último muro seleccionado, clic en la cota de una puerta + `500` + Intro, Esc, Espacio cuatro veces, las dos flechas, Espacio en un muro, calco que fuerza, muro alto de la planta baja visto desde la primera, cambiar de planta tres muros y volverlos). **Ventana real** revisada con capturas: cotas de una ventana, clic en el valor, `1000` + Intro («Cota fijada en 1000 mm»), Espacio, cotas de un tabique, planta Cubierta con la fachada alta cortada y el calco, forzado a una esquina del calco y la línea azul de rastreo.
- **IFC sin cambios**: la demo da el mismo IFC que la v0.17 (salvo los `GlobalId`, nuevos en cada demo), y las mismas plantas SVG byte a byte; validado con IfcOpenShell (reglas EXPRESS incluidas), también las sesiones de la app. Formato `.archisim` sin cambios (versión 4).
- Versión 0.18.0.

**Pendiente / limitaciones conocidas**
- Cotas temporales de **forjados** y cotas a **ejes** o a **núcleo** (Revit permite elegirlo); de momento: jambas y caras.
- Los muros de otra planta que se ven cortados **no se seleccionan** desde esta planta: se editan en la suya.
- En el calco, los muros de la planta inferior no se unen con los de la activa (son dibujos superpuestos).
- **Copiar a otra planta** (como «Pegar alineado» de Revit): no está; se puede cambiar de planta, no duplicar.
- Las cotas temporales no siguen al cursor mientras se arrastra (aparecen al soltar).

**Antonio debe probar**
1. Descarga la versión del pull request «Ayudas al dibujo…» (rama `bloque/ayudas-dibujo`) → **Actions** → la última ejecución en verde → **Artifacts** → **ArchiSim-Windows** o **ArchiSim-Mac**. Abre `demo.archisim`.
2. **Cotas temporales**: clic en la puerta de un dormitorio. Verás dos cotas azules de cada jamba a la cara del muro más cercano. Clic en un número, escribe `100` e Intro: la puerta queda a 10 cm exactos de esa cara (la mocheta habitual). Ctrl+Z. Prueba también `Esc` mientras escribes.
3. Selecciona un **tabique**: cota de longitud y luz libre hasta los muros paralelos. Escribe una luz libre (por ejemplo `3000`): el tabique se desplaza y los que acometen a él le siguen.
4. **Muros rectos**: tecla M y dibuja moviendo el ratón «a pulso»: deben salir rectos, con la línea azul. Aparta el cursor claramente para hacer un muro inclinado. Dibuja un tabique hasta la cara de una fachada haciendo el último clic un poco torcido: debe acabar en escuadra.
5. **Puntos medios**: con M, acerca el cursor al centro de la cara interior de una fachada o de un lado de la solera: triángulo verde.
6. **Girar**: selecciona una puerta y pulsa **Espacio** varias veces (cuatro posiciones); usa las dos flechas azules. Comprueba que la mano que dice Propiedades es la que esperas según UNE-EN 12519. Con la herramienta P, Espacio antes de colocarla.
7. **Cambiar de planta**: selecciona un tabique → Propiedades → «Restricción de base» → Cubierta: desaparece de la planta baja (el aviso lo dice) y aparece en Cubierta. Ctrl+Z.
8. **Otras plantas**: en una fachada de la planta baja pon «Desfase superior» `+2000` (o súbela de altura) y abre la planta **Cubierta**: esa fachada aparece **cortada**, y el resto de la planta baja en **gris** (calco). Dibuja un muro en Cubierta enganchando a las esquinas grises. Apaga *Calco* en la barra inferior.
9. Exporta el IFC tras estos cambios y ábrelo en ArchiCAD/Revit: los muros cambiados de planta deben salir en su nueva planta (`IfcBuildingStorey`).
10. Dime si las cotas de las puertas deben medir a la cara (como ahora) o al eje de los muros, y si echas en falta cotas temporales en algún otro elemento.

## 2026-10-18 · Bloque 16 — Restricción superior de muros

**Hecho** (petición de Antonio: «restricciones superiores de muro, de Planta baja a Planta 1, que se ajusten si cambia la altura de planta»; decidido hacerlo antes del bloque C)
- **Restricción superior**: un muro tiene altura desconectada (propia) o llega **hasta una planta** con un **desfase superior** (negativo: por debajo, por ejemplo el canto del forjado). Su altura la calcula el modelo y **cambia sola** al cambiar la cota de la planta superior o de la base, o el desfase de base.
- Se rechaza entero, explicando el motivo, lo que no cabe: bajar una planta hasta dejar la coronación en la base, o hasta que una ventana se salga del muro. Escribir una altura desconecta el muro (como en ArchiCAD); desconectarlo conserva la altura; borrar la planta superior lo desconecta con la altura que tenía.
- **Herramienta Muro**: en Propiedades, «Restricción de base», «Desfase de base», «Restricción superior» (Desconectada · **Planta superior** · Hasta cada planta de encima) y «Desfase superior» o «Altura desconectada». Por defecto dibuja **hasta la planta superior** (la de encima de la que dibujas, como «planta inicial + 1» de ArchiCAD); sin planta encima, 3 m propios, y lo avisa.
- **Muro seleccionado** (y varios a la vez, si están en la misma planta): la misma restricción superior; con planta, la altura se ve como «3000 · de las plantas».
- **Vivienda de prueba**: las fachadas llegan hasta la planta Cubierta menos el canto de la cubierta (los mismos 3 m de antes: el IFC sale idéntico). Sube la cota de Cubierta y las fachadas crecen con ella; los tabiques (2,70 m propios) no.
- **Formato `.archisim` versión 4**: los muros unidos a una planta llevan `"top": {"level": …, "offset": …}`. Los proyectos de las versiones anteriores se abren igual (sus muros, con su altura propia). El IFC no cambia (lleva la altura resultante, como Revit).
- **Propiedades a la izquierda** (bloque 15, segunda vuelta): ver la entrada anterior. Columna de nombres un poco más ancha.
- **Propuesta de disciplinas y mediciones** en `docs/DISCIPLINAS.md`: estructura (pilar, viga, cimentación con zapatas aisladas y corridas, losa, vigas de atado), urbanismo (terreno, parcela, viales por sección tipo, paisajismo; IFC 4.3, fuera de las mediciones del edificio) y mediciones con bancos BC3 y presupuesto vivo, en cinco fases (M1–M5). Decisiones D8–D12 para Antonio.
- Pruebas nuevas: núcleo (muro unido a la planta de arriba: sigue sus cotas y las de su base, rechaza lo que no cabe sin cambiar nada, se guarda y se reconstruye desde las cotas, se libera al escribir una altura o al borrar la planta), fichero (versión 4 con `top`, cota cambiada a mano en el fichero, fichero de versión 3 sin `top`) y ventana (dibujar hasta la planta superior, cambiar su cota, propiedades, desconectar y deshacer). Ventana real revisada con capturas.
- Versión 0.17.0.

**Pendiente / limitaciones conocidas**
- La restricción superior no viaja en el IFC (ni Revit la exporta): al importar, los muros quedan con altura propia.
- Los muros no se recortan todavía con el forjado de encima (por eso el desfase superior negativo).
- Pilares con restricción superior, cuando existan (2.5).

**Antonio debe probar**
1. Descarga la versión del pull request «Muros: restricción superior…» (rama `bloque/restriccion-superior`) → **Artifacts** → **ArchiSim-Windows** o **ArchiSim-Mac**. Abre `demo.archisim` **nuevo** (el del paquete).
2. Selecciona una fachada: «Restricción superior: Hasta Cubierta», «Desfase superior −548» (el canto de la cubierta) y «Altura 3000 · de las plantas».
3. Navegador → **Plantas…** → cambia la cota de **Cubierta** de 3548 a 3848. Mira la fachada en 3D: ha crecido 30 cm; los tabiques no. Ctrl+Z lo deshace.
4. Añade una «Planta primera» encima de la baja, vuelve a la planta baja y dibuja un muro: en Propiedades verás «Restricción superior: Planta superior (Planta primera)». Cambia la cota de la planta primera y comprueba que el muro la sigue.
5. Escribe una altura a mano en un muro unido: pasa a «Desconectada». Vuelve a unirlo eligiendo una planta en la lista.
6. Intenta bajar la Cubierta por debajo de la planta baja: debe negarse explicando por qué.
7. Lee `docs/DISCIPLINAS.md` y contesta D8 a D12; si puedes, pásame un presupuesto BC3 de una vivienda de AD5 como referencia de capítulos y criterios.

## 2026-10-18 · Bloque 15 — Interfaz rediseñada (ArchiCAD + propiedades de Revit)

**Hecho** (petición de Antonio: interfaz moderna, minimalista y práctica, «como un juego», parecida a ArchiCAD con las propiedades de Revit)
- **Barra superior** fina: menús, proyecto y fichero («sin guardar» si hay cambios), deshacer y rehacer, guardar, **Exportar IFC**, mostrar u ocultar la columna derecha, **tema claro u oscuro** (el dibujo sigue siendo papel blanco).
- **Caja de herramientas** a la izquierda por secciones (Selección · Modelado · Documentación · Vistas), con iconos propios dibujados como líneas, nombre y tecla. **Cota de nivel** pasa a ser herramienta propia (tecla **N**). Las que aún no existen se ven apagadas y dicen con qué punto de la hoja de ruta llegan. Botón para dejarla solo con iconos.
- **Pestañas de vistas** sobre el dibujo: cada planta y el 3D en su pestaña; cada planta recuerda su encuadre, su **escala** y su plano de corte. «+» abre vistas, × o clic con la rueda cierra, Ctrl+Tab pasa de una a otra, F2/F3 siguen funcionando. Una planta nueva se abre en su pestaña; borrarla la cierra.
- **Navegador**: Vistas (plantas con su cota, 3D), Planos (fase 3) y Modelo, con cada tipo de muro, forjado, puerta y ventana y su **número de ejemplares**. Clic: selecciona los de esa planta; doble clic: Editar tipo.
- **Propiedades al estilo Revit**: selector de tipo arriba (categoría, clase IFC, tipo con espesor o medidas, «✎ Editar tipo», «Seleccionar los N»), y grupos plegables Restricciones · Construcción · Dimensiones · Capas · Datos de identidad. Con una herramienta activa muestra lo que vas a dibujar (sustituye al antiguo cuadro de información de arriba); sin selección, la vista (escala, plano de corte, líneas de referencia, rejilla; nombre y cota de la planta) y el proyecto. El separador entre navegador y propiedades se arrastra.
- **Barra inferior**: escala **1:20 · 1:50 · 1:100 · 1:200** (cambia de verdad grosores, tramas, textos y símbolos de puertas y ventanas en pantalla), zoom en % del papel real, acercar/alejar/ajustar, **forzados activables** (Extremos, Medios, Aristas, Rastreo), Rejilla y Líneas de referencia; a la derecha el punto forzado, X/Y y la planta. En 3D: Perspectiva/Axonometría, ajustar, guardar imagen.
- **Sobre el dibujo**: las teclas del paso siguiente en una pastilla abajo y el resultado de cada acción como aviso que se desvanece arriba a la izquierda (los errores duran más).
- **Segunda vuelta con Antonio (mismo día)**: «las propiedades en el extremo opuesto obligan a cruzar la pantalla al elegir herramienta». **Propiedades pasa a la izquierda, junto a la caja de herramientas**, y el Navegador queda solo a la derecha (como Revit por defecto); cada columna se oculta por separado. El selector de tipo muestra el **dibujo del tipo a 1:20** (corte en planta del muro, capas del forjado, la puerta o ventana en planta) y el tipo se elige en una **galería** de fichas con su dibujo, nombre, medidas y un **buscador**. Ventana inicial de 1440 × 900.
- Tipografía IBM Plex (licencia OFL, incluida). Código de la ventana repartido en módulos (`app/`: barras, caja, pestañas, navegador, propiedades, tema, iconos).
- Pruebas nuevas: pestañas (nivel, encuadre y escala de cada una, F2/F3, Ctrl+Tab, cerrar, planta añadida y borrada), teclas y fichas de la caja, forzados de la barra inferior, la paleta de propiedades en todos sus estados y en los dos temas, elementos del mismo tipo. Todas las pruebas anteriores de la ventana pasan sin cambios. Ventana real revisada con capturas (planta, muro y ventana seleccionados, herramienta muro dibujando, 3D, tema oscuro, navegador, dos plantas a distinta escala, caja compacta, diálogos de tipos y plantas).
- Versión 0.16.0. **Sin cambios** en el modelo, el formato `.archisim` ni el IFC (demo, ida y vuelta y sesiones de la app validadas con IfcOpenShell).

**Pendiente / limitaciones conocidas**
- La disposición, el tema y las pestañas abiertas no se recuerdan al cerrar (se empieza siempre con la planta más baja, tema claro).
- Secciones, alzados y planos aparecen en el navegador como «llegan con…»: no existen todavía.
- No hay todavía paleta de órdenes ni atajos configurables.

**Antonio debe probar**
1. Descarga la versión de esta rama: GitHub → **Actions** → la última ejecución en verde del pull request «Interfaz: …» (rama `bloque/interfaz`) → **Artifacts** → **ArchiSim-Windows** o **ArchiSim-Mac**. Abre `demo.archisim`.
2. **Caja de herramientas**: pulsa M, F, P, V, C y N y mira cómo se ilumina cada ficha; pasa el ratón por «Pilar» o «Escalera»: dicen cuándo llegan. Pulsa la flecha ‹ de arriba: queda solo con iconos.
3. **Propiedades** (ahora a la izquierda): con la herramienta Muro activa, mira el corte del muro arriba; pulsa el nombre del tipo y elige otro en la galería (prueba a escribir «tabique» en el buscador); cambia la altura y dibuja un muro. Lo mismo con Puerta y Ventana. Selecciona un muro: «Seleccionar los 4» selecciona las fachadas; «✎ Editar tipo» abre su multicapa. Pliega y despliega los grupos.
4. **Navegador**: Modelo → Muros → clic en «Tabique LHD 10» (se seleccionan los tabiques), doble clic (se abre su tipo). Clic en «Cubierta» en Vistas: se abre en otra pestaña.
5. **Escala**: en la barra inferior pon **1:100** en la planta baja y luego vuelve a la pestaña de Cubierta y pon 1:20: cada pestaña conserva la suya. ¿Se ven bien los grosores y los símbolos a 1:100 y 1:20?
6. **Forzados**: con la herramienta Muro, apaga «Extremos» y comprueba que ya no se engancha a las esquinas; apaga «Rastreo» y comprueba que ya no se bloquea en horizontal.
7. **Tema oscuro**: botón ◐ arriba a la derecha.
8. Dime qué cambiarías: tamaño de las fichas, qué herramientas futuras sobran o faltan en la caja, si prefieres el navegador a la izquierda (como el Navegador de proyectos de Revit) y si el cuadro de información de ArchiCAD arriba se echa de menos o está bien en Propiedades.

## 2026-10-18 · Fase 2, bloque 14 — Familias: planta por escala y hojas configurables (bloque B)

**Hecho**
- **Las puertas y ventanas se dibujan en planta con el símbolo de su receta**, uno por escala (`plan 1:50`, `plan 1:100`), como ArchiCAD elige el nivel de detalle por escala. La planta ya no tiene dibujo de puertas y ventanas programado: cerco, hoja abierta a 90° y su giro, corredera con flecha, marco, vidrio, montantes, giro de cada hoja, vierteaguas y canto del alféizar salen de las recetas, con piezas comunes de planta (`piezas.asfam`).
- **Misma planta que antes**: la vivienda guardada por la v0.13 da la planta 1:50 idéntica en imagen (0 píxeles distintos a 4 aumentos), el mismo IFC y el mismo 3D. Una prueba nueva dibuja 176 casos (todos los funcionamientos, dos tamaños, los dos lados, las dos manos, con y sin antepecho, con y sin retranqueo, en un muro inclinado) con el dibujo antiguo y con las recetas y compara las imágenes.
- **Planta 1:100**: puertas con la hoja como una línea y su giro, sin cerco; ventanas con marco y vidrio. Se exporta desde *Archivo → Exportar planta 1:100 (SVG)…* y la línea de comandos la genera junto a la 1:50 (`demo-1a100.svg`).
- **Ventana practicable de 1 a 4 hojas, cada una con su funcionamiento** (tu propuesta): fija, practicable, oscilobatiente, abatible, proyectante o basculante. La «Ventana fija» deja de ser familia: es una practicable de una hoja fija (los proyectos anteriores se convierten solos). En planta giran las practicables y oscilobatientes; las hojas de la mitad izquierda giran en su jamba izquierda y las de la derecha en la derecha; una hoja sola, o la central si son tres, según la mano. Montantes entre hojas en planta y en 3D.
- **IFC por hoja**: el tipo de ventana lleva la partición (1, 2 o 3 hojas; 4 como «definida por el usuario») y el funcionamiento IFC de cada hoja (`FIXEDCASEMENT`, `TILTANDTURNLEFTHAND`, `TOPHUNG`…) con su posición. Validado con IfcOpenShell. **La importación lo lee**: una ventana de 3 o 4 hojas de otro programa entra con sus hojas y su funcionamiento (antes se importaba como de dos).
- **Diálogo de tipos generado desde la receta**: en Puertas y Ventanas, debajo de nombre, marca, descripción y familia, aparecen los parámetros de la receta por secciones (Dimensiones, Hojas, Cerco/Marco, Materiales) con el control que toca a cada uno (medida en mm, número de hojas, lista de funcionamiento, material); solo se ven los que aplican (la hoja 3 aparece al elegir 3 hojas). Cada cambio es un paso de deshacer y se aplica a todas las del tipo; la vista previa en planta 1:20 se dibuja con la receta.
- **＋ Nuevo…** en las pestañas de puertas y ventanas: un tipo nuevo de cualquier familia (las del proyecto y las de sistema) con sus valores por defecto. El funcionamiento ya no se cambia en un tipo existente (una corredera no se convierte en abatible): se crea un tipo de la otra familia.
- En Propiedades, «Funcionamiento» dice cómo abre cada hoja («3 hojas: fija · oscilobatiente · fija»).
- Lenguaje: bloques `plan 1:N` con líneas, contornos y arcos, plumas (corte, marco, vista, fina, rayado), nombres que da el programa en planta (`wall_ext`, `wall_int`, `sill_height`) y unión de textos con `+`.
- Pruebas: lenguaje (15), núcleo (36, con las familias recorridas en todas sus opciones de hojas, medidas extremas, lados y manos, también en planta 1:50 y 1:100 y con su IFC por hoja), planta (comparación de imagen), IFC (ida y vuelta con ventanas de 3 y 4 hojas y una proyectante), editor (valores de la receta, hojas fija y oscilobatiente, tipo nuevo), y la ventana real con capturas (diálogo, planta y 3D de una ventana de tres hojas).
- Versión 0.15.0. Sin cambios en la estructura del fichero `.archisim` (versión 3); las familias de sistema pasan a la versión 2 de su receta. Los proyectos guardados con la v0.14 conservan su copia de las familias antiguas y se siguen abriendo igual.

**Pendiente / limitaciones conocidas**
- El símbolo 1:20 y el alzado no están todavía en las recetas: a 1:20 se usa el de 1:50 (bloque D). Los muros a 1:100 se dibujan igual que a 1:50 (con capas y tramas): eso es de las normas de dibujo (D5) y las plantillas de vista (3.1).
- El IFC sigue escribiendo el cuerpo de puertas y ventanas con el código anterior (ya con el número de hojas de la receta); que salga de la receta, con la marca y los parámetros en Psets, es el bloque C.
- Hojas de puerta desiguales, hoja activa, tarjas, corredera empotrada y la **puerta plegable** van en la librería (2.2).
- Las hojas abatibles, proyectantes y basculantes no tienen símbolo en planta (se verán en alzado).

**Validación de Antonio (18/10/2026):** probada la versión de los bloques A y B: «funciona todo perfecto». Integrados en `main` los PR #6 y #7.

**Antonio debe probar**
1. Descarga la versión de esta rama: GitHub → **Actions** → la última ejecución en verde del pull request «Familias: planta por escala…» (rama `bloque/familias-planta`) → **Artifacts** → **ArchiSim-Windows** o **ArchiSim-Mac**. Abre `demo.archisim` (o uno tuyo de antes): la planta debe verse exactamente igual.
2. *Tipos y materiales* → **Ventanas** → «Ventana practicable 2 hojas 180×130»: pon **Nº de hojas = 3**, Hoja 1 **Fija**, Hoja 2 **Oscilobatiente**, Hoja 3 **Fija**. Cierra y mira la fachada sur en planta: dos fijos, montantes y la hoja central con su giro. Mira el 3D (tres paños). Ctrl+Z deshace cada paso.
3. Selecciona esa ventana: en Propiedades, «Funcionamiento: 3 hojas: fija · oscilobatiente · fija». Cambia su **Mano**: la hoja central gira hacia el otro lado.
4. «Ventana fija 60×60»: verás que su familia es «Ventana practicable» con 1 hoja fija. Cámbiala a practicable y mira el giro.
5. **＋ Nuevo…** → «Puerta corredera»: aparece «Puerta corredera (nuevo)» con la marca siguiente; colócala con la herramienta Puerta.
6. *Archivo → Exportar planta 1:100 (SVG)…* y ábrela: ¿te vale como planta 1:100 de puertas y ventanas?
7. Exporta el IFC y ábrelo en ArchiCAD y Revit: la ventana de 3 hojas debe verse con sus tres paños. Si tu programa muestra las propiedades de hoja del tipo (`IfcWindowPanelProperties`), deben decir fija, oscilobatiente y fija.
8. Dime qué sigue: **C** (IFC desde las familias, con marca y parámetros) o **D** (configurador con vista previa en alzado y 3D).

## 2026-10-18 · Fase 2, bloque 13 — Familias: núcleo (bloque A)

**Hecho**
- **Lenguaje de familias** (recetas `.asfam`, crate nuevo `archisim-family`): parámetros de tipo y de ejemplar por secciones, con rango, nombre en español y condición de visibilidad; fórmulas, condiciones (`if / else if / else`), repeticiones, **piezas comunes** reutilizables y reglas `require … else "mensaje"`. Es seguro: no toca ficheros ni red y no puede quedarse colgado (topes de repeticiones y de piezas). La sintaxis real, con un ejemplo que las pruebas comprueban, está en `docs/FAMILIAS.md`.
- **Cinco familias de sistema** escritas como recetas, con sus piezas comunes (`piezas.asfam`: cerco, hoja, marco, acristalamiento con montantes): **Puerta abatible** (1 o 2 hojas), **Puerta corredera**, **Ventana fija**, **Ventana practicable** (1 o 2 hojas) y **Ventana corredera**. Se pueden leer en `crates/archisim-core/families/`.
- **Los tipos de puerta y ventana son ahora tipos de familia**: familia + valores de sus parámetros + **marca de tipo** (P-01, P-02… para puertas, V-01… para ventanas, asignada sola y editable, única en el proyecto) + descripción. Duplicar un tipo copia familia y valores con marca nueva y `GlobalId` nuevo.
- **El 3D de puertas y ventanas sale de las recetas.** La vista 3D de la vivienda de prueba es idéntica, píxel a píxel, a la de la versión anterior; una prueba compara además 112 puertas y ventanas (todos los funcionamientos, dos tamaños, los dos lados y las dos manos, en un muro inclinado) con el cálculo antiguo: coinciden a la billonésima de metro.
- **Formato `.archisim` versión 3**: el proyecto guarda una copia de cada familia que usa (la receta, una línea por renglón, legible) y los tipos como familia + valores + marca. Los proyectos v1 y v2 se abren solos: cada tipo pasa a su familia de sistema con el mismo nombre, `GlobalId`, medidas y materiales, y recibe su marca. Comprobado con la vivienda guardada por la v0.13 (incluida como fichero de prueba): **mismo IFC, salvo la fecha, y mismas plantas byte a byte** que exportaba la v0.13.
- **Aplicación**: en *Tipos y materiales* → Puertas / Ventanas aparecen **Marca de tipo** y **Descripción** (editables, con deshacer) y la **Familia** de la que sale el tipo; la lista muestra la marca delante de las medidas. En Propiedades de una puerta o ventana, su marca y su familia. Los errores de las familias se explican en español con el nombre del parámetro («Hueco de obra: ancho = 25 está fuera de su rango…») o con el mensaje de la propia receta («El marco y el montante no dejan hueco para el vidrio»).
- Pruebas nuevas: lenguaje (12: lectura, fórmulas, piezas, condiciones, errores de sintaxis con su línea, valores fuera de rango, límites), núcleo (familias de sistema recorridas en **más de mil combinaciones** de opciones, medidas extremas, lados y manos: geometría válida y dentro del hueco; conversión exacta de los 7 funcionamientos antiguos; marcas; duplicar y editar), fichero (migración v1 y v2, la vivienda de la v0.13, errores de recetas y valores), editor (marca y deshacer) y la comparación del 3D con el cálculo anterior. Ventana real probada con capturas (Propiedades, diálogo de tipos, 3D). IFC de la demo y de las sesiones de la app validados con IfcOpenShell (reglas EXPRESS incluidas).
- Versión 0.14.0.

**Pendiente / limitaciones conocidas**
- **Planta e IFC siguen dibujando puertas y ventanas como antes**, a partir de las medidas que da la familia (bloques B y C). Por eso, una familia nueva distinta de las cinco de sistema todavía no tendría símbolo en planta propio.
- El diálogo de tipos sigue siendo el de la v0.12 (funcionamiento, medidas, marco, materiales); el configurador generado desde la receta, con vista previa en planta, alzado y 3D, es el bloque D. Cambiar el funcionamiento de abatible a corredera aún se permite desde ese diálogo (cambia de familia); en el configurador no se permitirá, como en ArchiCAD.
- Aún no se pueden cargar familias propias ni guardar tipos en una biblioteca (bloque E). La marca no sale todavía en el IFC (bloque C).

**Antonio debe probar**
1. Descarga la versión de esta rama: GitHub → **Actions** → la última ejecución en verde del pull request «Familias: núcleo…» (rama `bloque/familias-nucleo`) → **Artifacts** → **ArchiSim-Windows** (o **ArchiSim-Mac** en el MacBook).
2. Abre un proyecto **guardado con la versión anterior** (por ejemplo tu `demo.archisim` de antes): debe abrirse igual, en planta y en 3D.
3. Selecciona la puerta de entrada: en Propiedades verás **Marca de tipo P-01** y **Familia Puerta abatible (v1)**. Pulsa ✎ Editar tipo…: en la lista, cada tipo lleva su marca delante. Cambia la marca a `P-10` y escribe una descripción; Ctrl+Z lo deshace. Prueba a poner a otra puerta la misma marca: debe negarse («la marca «P-10» ya la tiene otro tipo»).
4. **Duplicar** un tipo de ventana: la copia recibe la siguiente marca libre (V-05).
5. Ventanas → «Ventana practicable 2 hojas 180×130»: pon «Marco: anchura vista» a `620` mm. Debe negarse con el mensaje de la propia receta: «El marco y el montante no dejan hueco para el vidrio» (dos jambas y el montante central suman 1860 mm, más que el hueco de 1800).
6. Guarda, cierra y abre otra vez; exporta el IFC y ábrelo en ArchiCAD/Revit: puertas y ventanas iguales que con la versión anterior.
7. Abre el `.archisim` guardado con un editor de texto y busca `"families"`: verás las recetas de las familias, línea a línea. Échales un vistazo (por ejemplo «Ventana practicable»): ¿se entiende qué hace cada parte?
8. Dime qué prefieres después: **B** (símbolos de planta de las familias a 1:100, 1:50 y 1:20) o **C** (IFC desde las familias, con la marca y los parámetros).

## 2026-10-18 · Fase 2 — Diseño de las familias paramétricas (decisión D2)

**Hecho**
- Diseño del sistema de familias en `docs/FAMILIAS.md`: qué es una familia (parámetros de tipo y de ejemplar con fórmulas, geometría 3D, símbolo de planta por escala, comportamiento y puntos de control, salida IFC), un ejemplo de receta de una ventana practicable, las reglas del sistema y el plan en cuatro bloques con su criterio de hecho.
- **Decisiones de Antonio**: D2 = **receta de texto** (editor gráfico más adelante sobre el mismo formato); familias **copiadas dentro del proyecto**, como Revit; nivel de detalle **por escala**, como ArchiCAD; se empieza por **puertas y ventanas**.
- **Afinado con Antonio** (misma mañana): el arquitecto crea tipos con un **configurador como el de ArchiCAD** (todas las opciones por secciones, vista previa en planta, alzado y 3D en directo); la receta es el motor de debajo. **Una familia por funcionamiento** (puerta abatible y corredera; ventana practicable, corredera y fija) con hojas, tarjas, subdivisiones, persiana… como opciones y **piezas comunes** compartidas. **Tipos con nombre, marca (P-03) y `GlobalId`**, todo lo que define el producto es del tipo, **guardables en ficheros de familia** con su catálogo para usarlos en otros proyectos, y **navegador de familias y tipos con su número de ejemplares** (el tipo es la unidad de medición). El plan pasa a cinco bloques (A–E).
- Hoja de ruta, fundacional y prioridades de `CLAUDE.md` actualizados. Sin código: ningún cambio en el programa, el formato `.archisim` ni el IFC.

**Pendiente**
- Bloque A (núcleo de familias). Criterio: la demo y los proyectos antiguos dan exactamente el mismo 3D e IFC que hoy, pero con las puertas y ventanas como familias.
- Sigue abierto D5 (normas de dibujo): fija los grosores con los que se dibujarán los símbolos de las familias por escala.

**Antonio debe probar**
1. Lee `docs/FAMILIAS.md`, sobre todo el ejemplo de receta: ¿entiendes qué hace cada línea? ¿Te ves retocando una (cambiar un rango, añadir un parámetro)?
2. Revisa la sección «Tipos: reutilizables y medibles» y la tabla de los cinco bloques: ¿es así como quieres trabajar con tus puertas y ventanas de AD5?
3. Si tienes un objeto de ArchiCAD que uses mucho (ventana con persiana y capialzado, por ejemplo), descríbeme sus parámetros tal como los ves en su diálogo: será la primera de la librería (2.2).

## 2026-10-17 · Cierre de fase 1, bloque 12 — Aplicación de Mac

**Hecho**
- **ArchiSim para Mac** en un `.dmg`: la integración continua compila la aplicación para chip Apple (M1 y posteriores) e Intel en un solo `ArchiSim.app` (universal) y la mete en una imagen de disco con un acceso a Aplicaciones, la vivienda de prueba (`demo.archisim`), los IFC de ejemplo y un `LEEME.txt` con cómo abrirla la primera vez. Se descarga igual que la de Windows: *Actions → Artifacts → **ArchiSim-Mac***.
- **Doble clic** en un `.archisim` en el Finder abre ArchiSim con ese proyecto (los `.archisim` quedan asociados a ArchiSim, con su icono). Los `.ifc` aparecen en *Abrir con → ArchiSim* sin quitarle el doble clic a tu visor IFC habitual. También se puede soltar un fichero sobre el icono del Dock. Si hay cambios sin guardar, pregunta antes.
- En **todos los sistemas**: soltar un `.archisim` o un `.ifc` sobre la ventana lo abre o lo importa; cualquier otro fichero se rechaza con un aviso.
- **Icono** propio (provisional, hasta decidir nombre e imagen): la esquina de un muro de fachada en planta con sus capas y una puerta. También es el icono de la ventana en Windows y Linux.
- La aplicación no está firmada con una cuenta de desarrollador de Apple (99 €/año): va firmada «ad hoc», que basta para usarla tras confirmarla una vez en *Ajustes del Sistema → Privacidad y seguridad → Abrir igualmente*.
- Pruebas: abrir ficheros soltados en la ventana (proyecto, IFC, fichero ajeno, con cambios sin guardar) con la app manejada sin pantalla. En la integración continua, en un Mac real: se monta el `.app`, se comprueba su firma y su `Info.plist`, se crea el `.dmg` y se abre `demo.archisim` «desde el Finder» (orden `open`) comprobando que llega a ArchiSim.
- Versión 0.13.0 (la 0.11 y la 0.12 son las de los PR #2 y #3). Sin cambios en el formato `.archisim` ni en el IFC.

**Pendiente / limitaciones conocidas**
- Sin firma ni notarización de Apple: hace falta la cuenta de desarrollador cuando se distribuya a otros.
- Windows sigue sin instalador, sin icono en el `.exe` y sin asociación de ficheros (resto del punto 1.8).
- Un fichero cada vez: si sueltas varios, se abre el primero y se avisa de los demás.

**Validación de Antonio (18/10/2026):** probada en su MacBook Pro Intel con macOS Sequoia la versión con los bloques 10, 11 y 12 juntos (tras salir del bucle de Gatekeeper con `xattr`): funciona. Integrados en `main` los PR #2, #3 y #4.

**Antonio debe probar**
1. En el MacBook: GitHub → **Actions** → la última ejecución en verde de `main` → **Artifacts** → **ArchiSim-Mac**. Descomprime y abre el `.dmg`.
2. Arrastra **ArchiSim** a **Aplicaciones** y ábrela. Cuando macOS diga que no puede comprobar el desarrollador: *Ajustes del Sistema → Privacidad y seguridad → Abrir igualmente*. En macOS Sequoia puede volver a pedirlo en bucle: entonces, en Terminal, `xattr -dr com.apple.quarantine /Applications/ArchiSim.app` y doble clic (comprobado por Antonio en un MacBook Pro Intel con Sequoia).
3. Copia `demo.archisim` del `.dmg` a tu Mac y haz **doble clic**: debe abrirse en ArchiSim. Navega en planta y en 3D (F3) con el trackpad: ¿va fluido?
4. Clic derecho en un `.ifc` → **Abrir con → ArchiSim**: debe importarse y salir el informe.
5. En Windows y en Mac: arrastra un `.archisim` o un `.ifc` sobre la ventana de ArchiSim abierta.
6. ¿Te vale el icono provisional o prefieres otra idea?
## 2026-10-17 · Cierre de fase 1, bloque 11 — Editor de tipos y materiales

**Hecho**
- Diálogo **Tipos y materiales**: *Vista → Tipos y materiales…*, botón «Tipos y materiales…» arriba a la derecha, o **✎ Editar tipo…** en las Propiedades de un muro, forjado, puerta o ventana (abre el diálogo en su tipo, como «Editar tipo» de Revit). Pestañas **Muros · Forjados · Puertas · Ventanas · Materiales**: a la izquierda la lista (con espesor y cuántos elementos lo usan), a la derecha sus datos.
- **Multicapas** de muro y forjado: nombre, función (muro exterior/interior; forjado de planta, solera o cubierta plana) y tabla de capas — material, espesor (mm) y función de cada una (estructura, soporte, aislamiento, cámara de aire, lámina, acabado), botones para subir y bajar, duplicar y quitar capas. Espesor total al día.
- **Puertas y ventanas**: nombre, funcionamiento (abatible, dos hojas, corredera; fija, practicable de 1 y 2 hojas, corredera), hueco de obra, anchura vista y fondo del marco, material del marco y de la hoja o vidrio.
- **Materiales**: nombre, categoría IFC, color de corte (selector de color o tecleado `232, 196, 178`) y trama de corte (liso, rayado diagonal, rayado cruzado, zigzag de aislamiento, punteado, vacío). Se ve en qué tipos se usa.
- **Vista previa a 1:20** con el mismo motor de dibujo que la planta (no es un dibujo aparte): un trozo de muro del tipo cortado en planta, exterior arriba; las capas del forjado como banda en sección (cara superior arriba); la puerta o ventana colocada en un muro del proyecto, con su mocheta y su giro; el material en un muro de 20 cm.
- **Crear = Duplicar** (como Revit): copia con nombre «… (copia)» y `GlobalId` nuevo, y queda seleccionada para editarla. **Borrar** solo lo que no se usa, diciendo por qué no (p. ej. «lo usan 4 muros», o los tipos que usan un material); siempre queda al menos un tipo de cada clase.
- Cada cambio es **un paso de deshacer** («Modificar tipo de muro», «Modificar material»…; arrastrar en el selector de color es un solo paso) y se aplica al momento a **todos los elementos del tipo**, en planta y en 3D: los muros conservan su línea de referencia y su `GlobalId`; las puertas y ventanas, su centro y antepecho. Lo que no cabe se rechaza entero con el motivo, en la barra de estado y en el propio diálogo (un muro más fino que el marco de sus puertas, una puerta que al ensancharse se sale del muro o pisa una ventana).
- Nombres únicos por lista (vacío o repetido se rechaza): el IFC y la importación reconocen los materiales por su nombre.
- Si se borra (o se deshace) el tipo que tenía una herramienta, la herramienta pasa a otro tipo existente.
- Pruebas: núcleo (editar y borrar materiales, multicapas y tipos de puerta comprobando todos los elementos que los usan, con rechazo y sin cambios), editor (duplicar con nombre libre, nombres repetidos o vacíos, borrar en uso, deshacer, último tipo), vistas previas (todas las puertas y ventanas de la plantilla quedan cortadas por el plano de corte) y la app: casa dibujada con clics simulados, las cinco pestañas con su vista previa, fachada duplicada con 8 cm de aislamiento y un material nuevo, puerta ensanchada y marco demasiado profundo rechazado, deshacer y rehacer. Esa casa se guarda, se reabre igual, se exporta (`out/app/tipos.ifc`, validado con IfcOpenShell con reglas EXPRESS, también en la integración continua) y se reimporta idéntica. Ventana real probada con capturas.
- Versión **0.12.0** (la 0.11 es la del PR #2, uniones en T, pendiente de revisión). Sin cambios en el formato `.archisim` ni en el IFC exportado.

**Pendiente / limitaciones conocidas**
- Sin prioridades de intersección editables por material (el PR #2 las deduce de la función de la capa; editarlas va con el punto 2.3).
- Sin importar o exportar materiales y tipos entre proyectos (bibliotecas o plantillas propias) ni ordenar las listas a mano.
- Los tipos de puerta y ventana siguen siendo los sencillos del MVP: premarco, persiana, capialzado y vierteaguas paramétricos llegarán con las familias (2.1, 2.2).
- La vista previa del forjado es la banda de capas con las tramas de planta: la sección de verdad llega con 2.10.
- Las categorías de material son una lista cerrada en el diálogo; las de otros programas importadas se ven como «Otra (…)» y se conservan.

**Antonio debe probar**
1. Descarga la versión de esta rama: GitHub → **Actions** → la última ejecución en verde del pull request «Tipos: editor de tipos y materiales (v0.12)» (rama `bloque/tipos-y-materiales`) → **Artifacts** → **ArchiSim-Windows**. Abre `demo.archisim`.
2. Selecciona la fachada sur → Propiedades → **✎ Editar tipo…**. Cambia la lana mineral de `50` a `80` + Intro: el espesor pasa a 295 mm y las cuatro fachadas engordan hacia dentro (la cara exterior se queda en su sitio). Ctrl+Z lo deshace.
3. **⧉ Duplicar** la fachada, renómbrala, añade una capa con ⧉, cámbiale el material y súbela con ▲. Aplica el tipo nuevo a un muro desde Propiedades. Mira la vista previa: ¿se lee como tu multicapa a 1:20?
4. Pestaña **Puertas** → «Puerta de entrada 1 hoja 100×215»: pon fondo de marco `400` → debe negarse («el marco no cabe en el espesor del muro»). Ancho `1100` → la puerta de la fachada oeste se ensancha sin moverse de su centro.
5. Pestaña **Materiales** → «Ladrillo perforado»: cámbiale el color con el selector y la trama a «Rayado cruzado». La planta cambia al cerrar el selector. Intenta **Borrar**: dice qué tipos lo usan. Duplica un material y bórralo.
6. Pestaña **Forjados** → cambia el canto del hormigón de la cubierta y mira en 3D (F3) que la cubierta cambia.
7. Guarda, cierra, abre de nuevo: los cambios siguen ahí. Exporta el IFC y ábrelo en ArchiCAD y Revit: tipos con el nombre nuevo, capas con los espesores nuevos y el material con su nombre.
8. Dime si echas en falta algo que uses a diario en ArchiCAD (por ejemplo, prioridades de intersección por material, o guardar tus multicapas para otros proyectos).
## 2026-10-17 · Fase 2, bloque 10 — Uniones en T y prioridades de intersección por capa

**Hecho**
- **Uniones en T** automáticas: un muro cuyo extremo cae dentro de otro (en su cara, en su eje o en cualquier punto de su espesor), lejos de sus esquinas y en ángulo de más de 15°, acomete a él. Da igual si lo dibujaste hasta la cara o hasta el eje: el resultado es el mismo.
- **Prioridad de intersección por capa**, como en ArchiCAD: cada capa del muro que acomete entra en el otro atravesando sus capas de menor prioridad, desde la cara por la que llega, y se detiene en la primera de prioridad igual o mayor. Prioridades por función de capa: **estructura 4 > soporte/hoja interior 3 > aislamiento 2 > cámara y lámina 1 > acabado 0**.
  - Tabique LHD contra fachada: el ladrillo hueco del tabique atraviesa el enlucido de la fachada y llega a su hoja interior de LHD; los enlucidos se detienen en la cara.
  - Muro de carga contra fachada: su ladrillo perforado atraviesa enlucido, hoja interior y aislamiento hasta la hoja exterior.
  - Tabique contra tabique: los dos tienen la misma prioridad, se detiene en la cara.
  - Nunca atraviesa la última capa del otro muro; si el muro que acomete es más alto que el otro o llega muy cerca de una esquina, simplemente se detiene en la cara.
- **Planta**: el muro que recibe pierde esa parte de sus capas; su línea de corte no se dibuja donde acomete el otro, y una línea fina marca dónde se detienen las capas que no entran. La hoja de LHD del tabique se ve continua con la de la fachada.
- **3D y IFC**: el muro que recibe pierde esa parte solo en la altura del que acomete (un tabique de 2,70 bajo una fachada de 3,00 no le quita nada por encima de 2,70). En 3D cada muro sigue siendo una sola piel cerrada; en el IFC el muro se exporta como una extrusión por tramo de altura, y su volumen neto coincide con el sólido (validado con IfcOpenShell). La importación de lo exportado sigue dando el mismo proyecto.
- **Vista 3D en equipos con OpenGL**: en tarjetas o controladores que no permiten ver una textura en otro formato (algunos OpenGL, máquinas virtuales) la pestaña 3D cerraba la aplicación. Ahora copia la imagen para mostrarla y funciona; la prueba de dibujo 3D vuelve a ejecutarse en esos equipos.
- Pruebas: geometría de la T (alcance por capa, pérdida exacta del muro que recibe, trozos de capa, muro más alto o junto a una esquina que solo topa, dibujado hasta el eje), sólidos 3D cerrados con volúmenes que cuadran (T a 90° y a 45°, tabiques más bajos que la fachada), demo exportada y validada con IfcOpenShell (volúmenes incluidos), ida y vuelta IFC. Planta de la demo revisada con zoom en las tres T; ventana real y vista 3D probadas con capturas.
- Versión 0.11.0. Sin cambios en el formato `.archisim`.

**Pendiente / limitaciones conocidas**
- Recorte de muros con forjados y cubiertas (siguiente parte del punto 2.3).
- Prioridades por función de capa, no editables por material (llegará con el editor de materiales, 1.6).
- Las esquinas en L siguen a inglete (sin envolver capas) y no hay cruces en X.
- Las T con muros en ángulo junto a huecos de puertas o ventanas del muro que recibe pueden dar geometría 3D imperfecta (caso raro).
- En 3D aparecen algunos puntos sueltos finos en la junta entre la coronación de una fachada y el canto del forjado de cubierta, donde el muro tiene aristas (jambas y ahora también las T) que el forjado no tiene. Es cosmético; se resolverá con el recorte muro–forjado.

**Antonio debe probar**
1. Descarga la nueva versión (GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**) y abre `demo.archisim`.
2. Haz zoom fuerte en planta en el encuentro del tabique central con la fachada sur, y en el del tabique este–oeste con la fachada este: el LHD del tabique debe llegar a la hoja interior de la fachada y los enlucidos detenerse. ¿Es así como lo dibujarías a 1:50 y 1:20?
3. Cambia el tipo de un tabique a «Muro de carga ladrillo perforado 1 pie 27» (Propiedades): su ladrillo debe atravesar hasta la hoja exterior de la fachada.
4. Dibuja un tabique hasta el **eje** de una fachada y otro hasta su **cara interior**: deben quedar iguales.
5. Exporta el IFC y ábrelo en ArchiCAD y Revit: los muros deben seguir entrando como muros con sus capas; mira en 3D o en sección el encuentro y las cantidades (volumen) de la fachada.
6. Si tu equipo tenía problemas con la pestaña 3D, vuelve a probarla.
7. Dime si las prioridades por función de capa te sirven o necesitas fijarlas por material (como ArchiCAD).

---

## 2026-10-17 · Cierre de fase 1, bloque 9 — Cotas, cotas de nivel y gestión de plantas

**Hecho**
- **Herramienta Cota** (tecla **C**), con tres métodos en el cuadro de información:
  - **Lineal**: clic en cada punto (con los forzados de siempre), Intro / doble clic / botón derecho, y clic donde va la línea. Dos puntos dan una cota simple; más, una **cadena**. Dirección automática (horizontal o vertical según dónde pongas la línea) o fija: horizontal, vertical o alineada. Vista previa en azul con el valor.
  - **Huecos de muro**: clic en un muro del lado de la cara a acotar → cadena con los extremos de esa cara (esquinas a inglete incluidas) y las jambas de todas sus puertas y ventanas; segundo clic para la línea.
  - **Cota de nivel**: clic → triángulo semirrelleno con la cota del suelo terminado respecto al ±0,00 (`+0,35`, `±0,00`, `-0,35`), leída del forjado más alto de la planta en ese punto. Si cambias el forjado o la cota de la planta, se actualiza sola.
- **Dibujo de las cotas** (planta exportada y pantalla, idénticas): líneas finas, trazo oblicuo a 45°, línea que sobrepasa 2 mm los trazos extremos, líneas auxiliares separadas 1,5 mm del elemento, valor encima de la línea, legible desde abajo o desde la derecha, en **metros con dos decimales** (`3,45`), texto de 2,5 mm.
- **Rotulación técnica propia**: los números se dibujan con trazos (estilo UNE-EN ISO 3098, diseño propio), no con una fuente del sistema: se ven igual en pantalla, en el SVG y se verán igual en PDF.
- Las cotas y cotas de nivel **se seleccionan** (clic sobre la línea o el símbolo), se borran, se copian (Ctrl) y se mueven: arrastradas solas, se desplaza solo la línea de cota; junto con muros, van con ellos. Propiedades: tramos, total, dirección; en la cota de nivel, el valor y la planta.
- La vivienda de prueba trae cotas de huecos y totales en las fachadas sur y oeste y dos cotas de nivel.
- **Plantas** (botón «Plantas…» junto al selector, o *Vista → Plantas…*): lista de arriba abajo con nombre y cota editables, altura hasta la planta de encima y contenido; **añadir planta encima** (con la altura de la última planta) o **debajo**; **borrar** (si tiene elementos, pide confirmación y los borra con ella; la última planta no se puede borrar). Cambiar la cota mueve todo su contenido. Todo se deshace con Ctrl+Z. Las plantas nuevas salen en el IFC como `IfcBuildingStorey`.
- **Formato `.archisim` versión 2** (cotas y cotas de nivel). Los proyectos de versión 1 se abren igual (prueba incluida) y se guardan ya en versión 2. De paso se corrige la lectura de decimales largos del fichero (p. ej. 3,9000000000000004), que podía cambiar en la última cifra al reabrir.
- Al encuadrar (Inicio) se tienen en cuenta las cotas.
- Pruebas: núcleo (cotas, cotas de nivel, plantas, ida y vuelta), fichero (versión 1 → 2), dibujo de cotas, herramienta de cota (cadena, dirección automática, huecos de muro por cada cara, cotas de nivel), arrastres de cotas y la app con clics simulados (acotar, seleccionar, mover la línea, borrar; añadir, renombrar, mover y borrar plantas con deshacer). Ventana real probada con capturas; planta de la demo revisada en SVG.
- Versión 0.10.0. Sin cambios en el IFC exportado.

**Pendiente / limitaciones conocidas**
- Las cotas **no son asociativas**: si mueves un muro, su cota no le sigue (fase 3, punto 3.2). Al moverlas junto con los muros, sí van con ellos.
- Sin estilos de cota por escala ni elección de unidades (m con 2 decimales fijo; decisión D5 pendiente), sin cotas de altura (sección) ni de superficie.
- Las cotas no se exportan al IFC (como Revit y ArchiCAD).
- No se ven otras plantas en gris debajo de la activa.

**Antonio debe probar**
1. Descarga la nueva versión (GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**) y abre `demo.archisim`: cotas en las fachadas sur y oeste y dos cotas de nivel. Mira de cerca el trazo oblicuo, los números y su posición: ¿es como acotas en el estudio?
2. Exporta la planta (SVG) y ábrela en el navegador con zoom: las cotas deben verse igual que en pantalla.
3. Tecla **C** → Lineal: acota el ancho de una habitación de cara a cara (los forzados «Sobre arista» ayudan), Intro, y coloca la línea dentro. Prueba una cadena de 4 puntos y la dirección Alineada.
4. Método **Huecos de muro**: clic por fuera de la fachada norte y otro clic más afuera para la línea. Repite por dentro: debe acotar la cara interior.
5. Método **Cota de nivel** sobre la solera; cambia la cota de la solera en sus Propiedades (p. ej. 50) y comprueba que la cota de nivel pasa a `+0,05`.
6. Selecciona una cota y arrástrala: se mueve solo la línea. Supr la borra; Ctrl+Z.
7. **Plantas…**: añade una planta encima, renómbrala, cámbiale la cota, dibuja algo en ella y bórrala (debe avisar). Exporta el IFC y comprueba en ArchiCAD/Revit que aparecen las plantas con su cota.
8. Abre un proyecto guardado con la versión anterior: debe abrirse sin problemas.
9. Dime si prefieres las cotas en centímetros o con otro número de decimales (es la decisión D5).

---

## 2026-10-17 · Cierre de fase 1, bloque 8 — Mover, copiar y estirar; varita mágica; vértices de forjado

**Hecho**
- **Mover con el ratón** (herramienta Flecha): se arrastra lo seleccionado, como en ArchiCAD (arrastrar en vacío sigue siendo ventana de selección). Mientras se arrastra se ve en azul cómo quedará, una línea desde el punto agarrado y la distancia; Mayús fuerza ángulos de 45°, se puede teclear la distancia (mm) + Intro, y Esc cancela. Se agarra por el punto especial más cercano (extremo, esquina, cara…) y se suelta con los mismos forzados que al dibujar, sin forzar a lo que se está moviendo.
  - Los muros unidos por sus extremos a uno que se mueve se **estiran** para que la esquina siga cerrada, y los tabiques que **acometen a la cara** de un muro movido se alargan o acortan por su eje para seguir contra esa cara (como Revit).
  - Las puertas y ventanas van con su muro; seleccionadas solas, **se deslizan a lo largo del muro**.
- **Copiar**: Ctrl al soltar. Copia muros con sus puertas y ventanas, forjados y huecos; quedan seleccionadas las copias (como Revit), con `GlobalId` nuevos.
- **Estirar**: los muros seleccionados muestran sus extremos; arrastrar uno mueve esa esquina y la de los muros unidos en ella. Con Ctrl, solo ese muro (sale de la esquina). Las puertas y ventanas se quedan en su sitio en planta (su distancia al inicio se recalcula).
- **Vértices de forjado** (punto 1.3 de la hoja de ruta): arrastrar una esquina la mueve; soltarla sobre la esquina vecina la quita; los rombos blancos del centro de cada arista desplazan la arista paralela a sí misma; con Ctrl, añaden una esquina.
- Lo que no cabe se rechaza entero con el motivo en la barra de estado (p. ej. mover un tabique de forma que otro tabique acometido se acorta y su puerta se saldría).
- **Varita mágica** para forjados (herramienta Forjado → Método: «Varita mágica», o **Espacio + clic** en modo polígono, como ArchiCAD): un clic dentro de un recinto de muros crea el forjado. Contorno: **cara interior** del recinto, **eje de muros** o **cara exterior** de todo el edificio (para la solera o la cubierta de la casa entera, con un solo clic en cualquier habitación). Vista previa del contorno y su superficie al pasar el cursor. Reconoce tabiques acometidos (T), ignora muros sueltos dentro del recinto y avisa si el clic cae en un muro o los muros no cierran.
- Pruebas: núcleo (estirar con huecos), geometría (recintos con tabiques, en L con encuentro en T, abiertos, con muro suelto, a eje y a cara exterior; los cuatro recintos de la vivienda de prueba), arrastres (mover, seguir esquinas y caras, copiar, estirar con y sin Ctrl, huecos que se deslizan o se salen, esquinas y aristas de forjado, vista previa) y la app con clics y teclas simulados (mover 1000 mm tecleados, copiar con Ctrl, estirar una esquina, cancelar con Esc, desplazar una arista de forjado, varita mágica en los tres contornos y con Espacio). La casa editada se exporta y valida con IfcOpenShell (`out/app/edicion.ifc`). Probada la ventana real con capturas (mover un tabique con sus acometidos, mover una fachada, varita mágica).
- La prueba de dibujo 3D se salta (con aviso) en adaptadores gráficos que no admiten vistas de textura en otro formato (algunos OpenGL por software), en vez de fallar.
- Versión 0.9.0. Sin cambios en el formato `.archisim` ni en el IFC exportado.

**Pendiente / limitaciones conocidas**
- Los forjados no siguen a los muros al moverlos (no hay asociación muro–forjado; ArchiCAD tampoco la tiene).
- La varita mágica no hace huecos en el forjado para muros sueltos ni patinillos (llegará con los huecos en forjados, 2.7).
- Sin «multiplicar» (copias en serie), girar ni simetría; sin arrastrar puertas y ventanas a otro muro.
- Soltar fuera de la planta cancela el arrastre.

**Antonio debe probar**
1. Descarga la nueva versión (GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**) y abre `demo.archisim`.
2. **Mover**: clic en el tabique vertical del centro; arrástralo 30 cm a la derecha y suelta. Los dos tabiques que acometen a él deben seguirle (uno se alarga, otro se acorta). Prueba a arrastrarlo más (1,3 m): debe negarse porque la puerta del tabique de la derecha se saldría. Ctrl+Z.
3. **Distancia exacta**: selecciona la fachada oeste, empieza a arrastrarla, mantén **Mayús**, teclea `1000` e Intro sin soltar el ratón. La casa se ensancha 1 m: fachadas norte y sur estiradas, esquinas a inglete cerradas.
4. **Copiar**: selecciona la puerta de entrada (fachada oeste), arrástrala 1,5 m a lo largo del muro con **Ctrl** pulsado al soltar: aparece una segunda puerta. Selecciona un tabique y cópialo con Ctrl: la copia queda seleccionada.
5. **Estirar**: selecciona la fachada este y arrastra su cuadrado negro de la esquina noreste hacia fuera: la fachada norte la acompaña. Repite con **Ctrl**: solo se mueve la fachada este.
6. **Forjado**: selecciona la solera; arrastra un rombo blanco de una arista hacia fuera (la arista se desplaza paralela), arrastra una esquina, añade una esquina con Ctrl en un rombo y quítala soltándola sobre la vecina.
7. **Varita mágica**: tecla **F**, Método «Varita mágica», pasa el cursor por las habitaciones (se ve el contorno y la superficie) y haz clic. Prueba los tres contornos: cara interior, eje y cara exterior (esta última da toda la casa). También en modo Polígono con **Espacio + clic**.
8. Exporta el IFC tras editar y ábrelo en ArchiCAD/Revit: los muros movidos y estirados conservan su `GlobalId` (compara con el IFC anterior), las copias tienen uno nuevo.
9. Cuéntame si el comportamiento al mover (tabiques que siguen a la cara, esquinas que se estiran) es el que esperas o prefieres el de ArchiCAD (la esquina se rompe al mover un solo muro).

---

## 2026-10-17 · Fase 1, bloque 7 — Importación IFC (MVP terminado) y hoja de ruta

**Hecho**
- **Importar IFC** (*Archivo → Importar IFC…*, o arrastrando un `.ifc` sobre `ArchiSim.exe`, o `archisim importar casa.ifc` en la línea de comandos). Lee **IFC2X3 e IFC4** (también IFC 4.3) de cualquier programa, en metros, milímetros, pies o pulgadas, con un lector STEP propio escrito desde cero.
  - **Plantas** con su nombre y cota.
  - **Muros** por su eje (o, si no lo tienen, por su colocación y su sólido), altura y cota de base desde el sólido, **capas** y **línea de referencia** desde el `IfcMaterialLayerSetUsage` (lado, orden y desfase). Las capas de espesor cero (membranas de Revit) se omiten con aviso. Los muros sin capas entran con una capa de su espesor.
  - **Lado exterior**: en IFC no existe; en los muros marcados como exteriores se toma como exterior el lado contrario al centro de los muros de la planta, y se giran para que la cara exterior quede a la izquierda, como en ArchiSim (un contorno dibujado al revés vuelve con las capas bien puestas).
  - **Forjados** por el contorno de su extrusión o por la cara superior de su sólido (también mallas trianguladas), con su cara superior como referencia, capas de arriba abajo y función (forjado, solera, cubierta).
  - **Puertas y ventanas** alojadas en muros: hueco de obra desde el `IfcOpeningElement`, posición sobre la línea de referencia, antepecho, lado hacia el que abren, mano UNE-EN 12519 (traducida desde la convención IFC), funcionamiento (abatible, dos hojas, corredera; ventana fija, practicable de una o dos hojas, corredera), marco desde las propiedades de cerco y materiales de marco y hoja/vidrio.
  - Se conservan **todos los `GlobalId`** (proyecto, plantas, tipos, muros, forjados, huecos), para poder comparar versiones con los ingenieros.
  - **Informe de importación** en una ventana: qué ha entrado y qué no (pilares, vigas, escaleras, espacios, cubiertas inclinadas, muros curvos, huecos sin carpintería…), con el motivo, y las aproximaciones hechas.
  - Si el fichero no trae tipos de muro, forjado, puerta o ventana, se añade la plantilla de ArchiSim para que todas las herramientas tengan con qué dibujar. El proyecto importado queda «sin guardar» con el nombre del IFC.
- **Ida y vuelta exacta**: lo que exporta ArchiSim se importa idéntico (muros, capas, materiales con color y trama, forjados, huecos con mano, retranqueo y marco, `GlobalId`). Para ello el IFC exportado lleva además las propiedades de cerco de puertas y ventanas (`IfcDoorLiningProperties`, `IfcWindowLiningProperties`), el funcionamiento de cada hoja de ventana (`IfcWindowPanelProperties`), la mano de cada hueco (`ArchiSim_Opening`) y la trama de cada material (`ArchiSim_Material`). Sigue validando con IfcOpenShell.
- Pruebas: ida y vuelta con un modelo que usa todas las opciones y con la vivienda de prueba; un fichero tipo Revit en IFC2X3 y milímetros escrito a mano (muros dibujados al revés, capas desde el exterior a la derecha, membrana sin espesor, puerta con `IfcDoorStyle`, forjado con capas de arriba abajo, un pilar); dos ficheros generados con **IfcOpenShell** (otro programa, no ArchiSim) en IFC4/metros e IFC2X3/milímetros, que también se incluyen para que los pruebes; y la importación desde la aplicación con el informe. Probado además con los ficheros de ejemplo de buildingSMART (SketchUp, IFC2X3, IFC4 e IFC 4.3). Ventana real probada con capturas (informe, planta y 3D).
- Barra de estado: los mensajes largos ya no se montan sobre la ayuda.
- **Hoja de ruta** después del MVP en `docs/HOJA_DE_RUTA.md`: decisiones pendientes, deudas de la fase 1 y fases 2, 3 y 4 con tamaños y dependencias.
- Versión 0.8.0. **MVP terminado**: los 10 puntos de la fase 1 están hechos.

**Pendiente / limitaciones conocidas**
- No se importan: muros curvos o de varios tramos, forjados inclinados (se avisan), huecos en forjados (se cierran, con aviso), huecos sin carpintería, puertas y ventanas en muros cortina, y ninguna otra clase (pilares, vigas, escaleras, cubiertas, mobiliario, espacios…). Todo queda contado en el informe.
- Los muros recortados por una cubierta entran rectos con su altura máxima (aviso).
- Las capas se ordenan por el lado exterior deducido; en muros interiores de otros programas el orden de las caras puede salir al revés (no cambia nada del dibujo, solo qué cara se llama «exterior»).
- Probado con ficheros sintéticos, de IfcOpenShell y de SketchUp; falta probar con exportaciones reales de ArchiCAD y Revit (punto D7 de la hoja de ruta).

**Antonio debe probar**
1. Descarga la nueva versión: GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**. Ahora trae una carpeta `ejemplos-ifc` con dos IFC escritos por IfcOpenShell.
2. Abre `ArchiSim.exe` → *Archivo → Importar IFC…* → `ejemplos-ifc/ifcopenshell_ifc2x3_mm.ifc`. Lee el informe: 5 muros, 1 forjado, 2 puertas, 1 ventana; un pilar y un hueco vacío sin importar. Mira la planta (F2) y el 3D (F3): las fachadas deben tener el monocapa por fuera aunque el fichero las dibuja al revés.
3. Selecciona un muro de fachada y mira sus Propiedades: tipo «Fachada 4 capas», línea de referencia en la cara exterior. Selecciona la puerta de entrada: mano y lado de apertura.
4. **Ida y vuelta**: abre `demo.archisim`, *Exportar IFC4…*, después *Importar IFC…* ese mismo fichero: debe salir la misma casa.
5. **Lo importante**: exporta un IFC de un proyecto tuyo sencillo desde **ArchiCAD** y otro desde **Revit** (IFC2X3 o IFC4, como lo hagas normalmente) e impórtalos. Mándame el informe (captura) y lo que veas mal: muros girados, capas en el lado equivocado, puertas con la mano cambiada, forjados que faltan. Los ficheros no se suben al repositorio.
6. Lee `docs/HOJA_DE_RUTA.md` y decide: el orden de los próximos bloques (propongo mover/estirar + varita mágica) y, cuando puedas, las decisiones D2 a D5 (lenguaje de familias, IFC4 o IFC 4.3, licencia y nombre, normas de dibujo).

---

## 2026-10-17 · Fase 1, bloque 6 — Vista 3D

**Hecho**
- **Vista 3D** del edificio entero (todas las plantas), en una pestaña junto a la planta: **F2** planta, **F3** 3D (como ArchiCAD), o con las pestañas «Planta · …» / «3D» encima del dibujo.
- **Lo que se ve es lo que recibe el IFC**: muros con sus capas y los huecos de puertas y ventanas, forjados con sus capas colgando de la cara superior, y carpinterías con marco, hojas y vidrio. Comprobado: el volumen de cada muro en 3D coincide exactamente con el `NetVolume` que exporta el IFC.
  - Colores de los materiales de corte, como en la exportación IFC; el vidrio, transparente.
  - Las capas se ven en los cantos de forjado, en la coronación de los muros y en las mochetas; las láminas de 2–3 mm aparecen como una línea fina.
  - Aristas de contorno en tinta (siluetas, cambios de plano y saltos de profundidad), como el sombreado con contornos de ArchiCAD.
  - Cada muro y cada forjado es una sola piel cerrada: las capas y los tramos que se tocan dentro del elemento no tienen caras, y los muros unidos a inglete se cierran el uno al otro. Así no hay parpadeos ni puntos sueltos en las aristas.
- **Navegación** (como Revit y ArchiCAD): arrastrar con el botón izquierdo (o Mayús + rueda) para girar alrededor del modelo; arrastrar con la rueda o el botón derecho para desplazar; rueda para acercar donde está el cursor; doble clic en la rueda o **Inicio** para ajustar. **Perspectiva** o **axonometría** (proyección paralela). Flecha de norte en la esquina, que gira con la vista.
- **Selección en 3D**: clic sobre un elemento (Mayús+clic para añadir o quitar). Lo seleccionado se tiñe de azul y sus propiedades aparecen a la derecha, igual que en planta; Supr lo borra. Las teclas de herramienta (M, F, P, V) vuelven a la planta para dibujar.
- **Guardar imagen 3D** en PNG (botón «Guardar imagen…» o *Archivo → Guardar imagen 3D*), a doble resolución de la pantalla.
- Calidad de imagen: supermuestreo ×2 y multimuestreo ×4. Se dibuja solo cuando algo cambia (cámara, modelo, selección o tamaño).
- Pruebas: cámara (encuadre, zoom en el cursor, rayos), selección por rayo, geometría (pieles cerradas, volúmenes, esquinas a inglete cerradas), y un dibujo real con la tarjeta gráfica (o una por software) que se comprueba píxel a píxel; en la integración continua se guarda como `vista3d.png`. Con clics simulados: F3, seleccionar un muro en 3D, girar, borrar y volver a planta. Probada la ventana real con capturas.
- No cambia nada de lo exportado: IFC, plantas y `.archisim` siguen igual (validados con IfcOpenShell; ida y vuelta idéntica).
- Versión 0.7.0.

**Pendiente / limitaciones conocidas**
- Sin terreno, sombras proyectadas ni texturas; sin secciones 3D ni recorte por plantas (se ve todo el edificio).
- Las puertas se ven cerradas; las vueltas de mocheta y el premarco son de dibujo de planta y no aparecen en 3D (tampoco en el IFC).
- Sin dibujar en 3D: las herramientas trabajan en planta.
- En equipos sin tarjeta gráfica compatible (Vulkan, DirectX 12 o Metal) la pestaña 3D lo indica con un aviso.

**Antonio debe probar**
1. Descarga la nueva versión: GitHub → **Actions** → última ejecución en verde → **Artifacts** → **ArchiSim-Windows**. Arrastra `demo.archisim` sobre `ArchiSim.exe`.
2. Pulsa **F3**. Gira con el botón izquierdo, desplaza con el derecho o la rueda pulsada, acerca con la rueda. Inicio para volver a encuadrar. ¿Se mueve con fluidez en tu equipo?
3. Mira con zoom: esquinas a inglete, mochetas de puertas y ventanas (capas en la mocheta), cantos de la solera y de la cubierta con sus capas, vidrio transparente. ¿Los colores y las aristas se leen bien como maqueta?
4. Cambia a **Axonometría** y compara con una axonometría tuya.
5. Haz clic en una ventana en 3D, cambia su antepecho en Propiedades y comprueba que se mueve en 3D y en planta (F2). Prueba Supr y Ctrl+Z desde la vista 3D.
6. Dibuja algo en planta (M, F, P, V) y vuelve a F3: aparece al momento.
7. «Guardar imagen…» y abre el PNG.
8. Abre `demo.ifc` en ArchiCAD o Revit y compara en 3D con lo que ves en ArchiSim: deberían coincidir.
9. ¿Qué sigue: importar IFC (último punto del MVP), varita mágica para forjados, mover/estirar con el ratón o cotas?

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
