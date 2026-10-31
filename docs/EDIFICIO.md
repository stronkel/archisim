# El edificio de Florence Street: prueba de ArchiSim con un proyecto real

Antonio modeló en su máster de BIM manager un edificio de viviendas en Florence Street (Brunswick, Melbourne). El 31/10/2026 se rehízo entero con ArchiSim, a partir de sus plantas, su sección, sus alzados y las fotos de la obra. El objetivo era doble:

- tener un proyecto grande para medir la velocidad;
- ver qué le falta a la aplicación para modelar un edificio como lo haría un estudio.

`archisim edificio [carpeta]` lo construye y lo exporta todo: proyecto, IFC 4.3 e IFC4, plantas, planos, PDF, DXF, glTF, mediciones y comprobación CTE. El código está en `crates/archisim-cli/src/edificio.rs`.

## Qué tiene el modelo

- **Plantas:**
  - baja ±0,00;
  - plantas 1 a 4: +3,40, +6,60, +9,80 y +13,00;
  - cubierta +16,20;
  - pabellón +19,40.
- **Orientación:** el norte verdadero mira a la derecha de la planta, así que la fachada a la calle es el alzado sur.
- **Ejes:** 1 a 8 a lo largo y A a D a lo ancho.
- **Estructura:**
  - medianeras y núcleo de hormigón visto;
  - losas de 25 cm vistas, con tarima;
  - vigas de transición y pilares sobre el zócalo;
  - zapatas corridas bajo los muros y aisladas bajo los pilares.
- **Planta baja:**
  - zócalo de ladrillo visto con escaparates de acero, puerta pivotante y puerta seccional al callejón;
  - café con cocina y barra, y oficina de la cooperativa;
  - aseos, uno accesible;
  - ascensor y cuarto de basuras;
  - helechera bajo el patio, con la escalera;
  - aparcamiento de bicicletas con aparcabicis y bicicletas;
  - trasteros de malla.
- **Plantas 1 a 4:** seis viviendas por planta, 24 en total.
  - Muros de hormigón entre viviendas, que salen hasta el borde de los balcones.
  - Tabiques, puertas de vivienda EI2 30, baños, y dormitorios al patio o a los patinillos.
  - Fachada de acero negro y vidrio en las dos testeras.
  - Balcones con barandilla de barrotes, jardineras y mesa.
  - En la planta 1, terraza sobre el zócalo con antepecho de ladrillo.
- **Núcleo:** escalera en U de hormigón en cada planta, abierta al patio, y casetón y cuarto de máquinas en cubierta.
- **Cubierta:**
  - jardín con arriates hexagonales de gramíneas, lavanda y romero, pérgolas y mesas;
  - pabellón de invierno acristalado, con mesas largas y la lavandería.
- **Instalaciones:**
  - lámparas colgantes y plafones en los puntos de luz, cuadro de cada vivienda y enchufes;
  - conductos vistos de la campana a la fachada;
  - agua fría y caliente del montante a cada baño y cocina;
  - bajantes.
- **Locales:** 117, con su uso y acabados (tarima, gres, adoquín en espiga, hormigón). El pasillo común y la escalera forman un solo local.
- **Documentación:**
  - secciones A y B y los cuatro alzados;
  - etiquetas de puertas y ventanas, cotas y cotas de nivel;
  - siete planos A2 (A-01 a A-07, el último de presentación);
  - datos del proyecto, georreferencia (UTM 55 S) y extinción automática.
- **Entorno:** la calle con aparcamiento y acera, arbolado, la parcela y los edificios vecinos como volúmenes de Concepto.

Comprobación CTE: 249 cumplen, 0 no cumplen y 0 avisos. El IFC del edificio valida con IfcOpenShell (169 muros, 117 locales, 895 objetos).

## Vuelta de velocidad

Todas las medidas se tomaron en el mismo servidor, de dos núcleos, sobre el mismo proyecto (`archisim medir proyecto out/edificio.archisim`). El modelo tiene:

- 169 muros, 133 huecos, 887 objetos y 117 locales;
- 7,5 millones de triángulos en 3D, de los que 6,6 millones son muebles.

| Trabajo | Antes | Después |
|---|---|---|
| Montar el edificio con sus planos | 146 s | 13 s |
| Exportarlo todo (`archisim exportar`) | fallaba en el plano de presentación | 115 s |
| Sección A: calcularla | 4,6 s | 0,9 s |
| Sección B: calcularla | 4,1 s | 0,6 s |
| Un alzado: calcularlo | 11 s | 2,7–3,1 s |
| SVG de la sección A | 46 MB | 17 MB |
| SVG del alzado este | 74 MB | 28 MB |
| Axonometría de los planos | 768 MB de SVG | 2,5 s y 69 MB |
| IFC | 158 MB | 79 MB |
| Importar ese IFC | 2,8 s | 1,5 s |
| glTF para D5 o Twinmotion | 663 MB | 179 MB |
| PDF de los planos | 47 s | 32 s |
| Abrir y guardar el proyecto | 0,07 s y 0,03 s | igual |
| Plantas (dibujo e imagen en pantalla) | 0,1–0,2 s cada una | igual |
| Vista 3D: preparar la escena y subirla | — | 2,9 s y 0,3 s (224 MB en la tarjeta) |

En la última fila, el tiempo por fotograma medido aquí, unos 2 s, no sirve: este servidor no tiene tarjeta gráfica y dibuja por software.

**Qué se cambió:**

1. **Las secciones no fabrican los muebles que no ven.** Los objetos que quedan detrás de la línea o más allá de la profundidad de la sección ni siquiera se construyen. Además, las caras se ordenan por su índice y no moviéndolas.
2. **Los muebles van simplificados en los dibujos** (`with_coarse_props`). En secciones, alzados y axonometrías, sus puntos se juntan en celdas de 3 cm: los pliegues de un cojín o los listones de una silla, que a 1:50 no llegan a una décima de milímetro. Es el «nivel de detalle bajo» de Revit para las vistas que no piden más. En 3D, render, glTF e IFC siguen completos.
3. **La axonometría deja fuera lo que no se ve.** Las caras que tapan por completo otras caras más cercanas (los muebles detrás de muros y forjados) ya no se escriben, una a una, en el plano.
4. **Los planos de presentación calculan cada dibujo una sola vez.** Antes se rehacía la axonometría y la sección por cada escala que probaban para encajarlas.
5. **Los alzados automáticos miden el edificio**, no el modelo entero con sus muebles, árboles y vecinos. Además, se llaman por el norte verdadero: antes el de arriba de la pantalla siempre era «Norte».
6. **En el IFC, las mallas comparten sus vértices** (barandillas, escaleras, terreno, piscinas). Antes se escribía cada esquina una vez por triángulo.
7. **En el glTF, cada mueble se escribe una vez** y se coloca en cada objeto, como hace ya la vista 3D.
8. **El PDF de los planos reutiliza los planos ya dibujados** en lugar de volver a dibujarlos.

## Qué le falta a ArchiSim (lo que salió al modelarlo)

Por orden de lo que más ha estorbado:

1. **Puertas en los muros cortina.** La fachada real es de acero y vidrio con balconeras. En ArchiSim un muro cortina no admite puertas, así que en el modelo no se sale al balcón. Revit lo resuelve con un panel de puerta en el muro cortina.
2. **Los muros cortina no cierran locales.** La varita no los ve: los estares que dan a la fachada de vidrio tuvieron que dibujarse con su contorno a mano.
3. **Protección contra incendios.** No hay sistema de rociadores ni bocas de incendio (las tuberías rojas de las fotos), ni su comprobación en el DB-SI 4.
4. **Ascensor.** No existe como elemento (cabina, puertas, foso y sobrerrecorrido, `IfcTransportElement`). Se modeló el hueco y una puerta corredera.
5. **Placas solares.** No hay familia de placas fotovoltaicas para la cubierta del pabellón.
6. **Trepadoras.** Solo crecen sobre pérgolas. No sirven para las fachadas verdes de cables o malla del edificio.
7. **Muebles demasiado pesados.** Una mesa de comedor tiene 26 000 triángulos y una jardinera 17 000. Tras esta vuelta ya no afecta a los dibujos, pero el 3D de un edificio entero lleva 7,5 millones. Conviene un presupuesto de triángulos por pieza en el taller.
8. **Instalaciones a mano.** Cada ramal de agua se trazó punto a punto. Falta «conectar el aparato a la red», que lleve la tubería del aparato a la montante más cercana.
9. **Deshacer copia el modelo entero en cada paso.** Con modelos grandes eso es memoria: conviene guardar solo lo que cambia.
10. **Mensajes de error sin contexto.** Cuando una puerta no cabe, dice «se solapa» o «fuera del muro», pero no qué muro ni dónde.
11. **Planos pesados.** Los alzados de un edificio con fachada de vidrio enseñan todos los muebles de dentro. El plano de alzados en DXF pesa 147 MB y el PDF tarda medio minuto. Falta poder ocultar los muebles en alzados lejanos, o dibujarlos como silueta.
12. **Recorrido solar en el hemisferio sur.** El edificio está en Melbourne y el diagrama de sol del plano de presentación sale con las horas montadas. Hay que comprobarlo.
13. **Cubierta ajardinada.** No hay tipo de forjado con sustrato y césped. El jardín se hizo con plantaciones y jardineras.
14. **Desnivel de la planta baja.** La zona de bicicletas está unos centímetros más baja, con una rampa hasta el callejón. No se modeló.

## Qué debe probar Antonio

1. Generar el edificio:
   ```
   cargo run --release -p archisim-cli -- edificio out/
   ```
   Abrirlo con:
   ```
   cargo run --release -p archisim-app -- out/edificio.archisim
   ```
   Recorrer las plantas, la sección A y la vista 3D. Comprobar si la distribución se parece a la del máster y qué cambiarías.
2. Abrir `out/edificio.ifc` en el visor IFC que uses y mirar las clases, los locales y las mediciones.
3. Mirar los planos `out/edificio-planos.pdf`, sobre todo A-05 (secciones), A-06 (alzados) y A-07 (presentación).
4. Medir en tu ordenador:
   ```
   cargo run --release -p archisim-cli -- medir proyecto out/edificio.archisim
   ```
   Pásame lo que salga, para compararlo con las cifras de arriba.
5. Decirme por cuál de los huecos de la lista empezamos.
