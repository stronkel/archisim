# ArchiSim — Fase 3: documentación

> 19 de octubre de 2026 · Diseño de la fase 3 de `docs/HOJA_DE_RUTA.md` (3.1 a 3.8), pedida por Antonio:
> «desarróllala al completo… herramientas de dibujo completas y eficientes para que los que vienen de AutoCAD se sientan cómodos».

## 1. Principios

- **Lo que se ve es lo que se imprime.** La planta en pantalla, el SVG, el PDF y el DXF salen del mismo dibujo (`archisim-plan`). Un plano en pantalla es el papel.
- **Dibujo 2D de verdad, no un añadido.** Quien viene de AutoCAD encuentra sus órdenes, sus alias, la línea de órdenes, las coordenadas tecleadas, los forzados a objeto y las capas. Quien viene de ArchiCAD o Revit, sus herramientas de siempre.
- **Anotación = papel.** Textos, cotas, etiquetas y tramas de dibujo tienen su tamaño en **milímetros de papel** (como Revit y ArchiCAD): un texto de 2,5 mm mide 2,5 mm impreso a 1:50 y a 1:100.
- **Lo que no es edificio no va al IFC.** Líneas, textos, tramas, cotas, etiquetas y planos se guardan en el `.archisim`, no en el IFC (como Revit y ArchiCAD). Los **locales** sí: son `IfcSpace`.

## 2. Dibujo 2D (estilo AutoCAD)

### Elementos

| Elemento | Qué es | AutoCAD |
|---|---|---|
| Línea | Segmento entre dos puntos | LINEA |
| Polilínea | Tramos rectos y de arco, abierta o cerrada; anchura opcional | POL |
| Rectángulo | Polilínea cerrada por dos esquinas (con giro) | RECTANG |
| Polígono | Polilínea regular inscrita o circunscrita | POLIGONO |
| Arco | Tres puntos; centro, inicio y fin; inicio, fin y radio | ARCO |
| Círculo | Centro y radio o diámetro; dos o tres puntos | CIRCULO |
| Elipse | Centro y ejes; arco de elipse | ELIPSE |
| Punto | Marca de un punto | PUNTO |
| Texto | Una línea o párrafo, con altura en mm de papel, giro y justificación | TEXTO / TEXTOM |
| Sombreado | Trama o relleno dentro de contornos (clic dentro de un recinto o eligiendo contornos) | SOMBCONT |
| Nube de revisión | Polilínea de arcos | NUBEREV |
| Directriz | Flecha con texto | DIRECTRIZ |

Cada elemento pertenece a una **vista**: la planta de un nivel o una hoja de plano.

### Capas de dibujo

Como las capas de AutoCAD: nombre, color, grosor (mm), tipo de línea, visible, bloqueada (no se selecciona ni se modifica) e imprimible. Cada elemento de dibujo tiene su capa y puede **cambiar color, grosor o tipo de línea** («PorCapa» si no). Capa «0» siempre presente. La capa actual se elige en la barra de propiedades del dibujo.

### Órdenes de edición

Desplazar, copiar, girar (con referencia), escala (con referencia), simetría, equidistancia, recortar, alargar, empalme (con radio), chaflán, matriz rectangular y polar, partir, partir en un punto, juntar, descomponer, estirar (ventana de captura), alinear, borrar, igualar propiedades, y medidas: distancia, área y lista.

Desplazar, copiar, girar, escala y simetría valen también para **muros, forjados, cotas y textos** (lo que tenga sentido en un edificio).

### Línea de órdenes

En la barra inferior, como AutoCAD:

- **Se escribe y se pulsa Intro o Espacio**: `L` línea, `PL` polilínea, `C` círculo, `A` arco, `REC` rectángulo, `D` desplazar, `CP` copiar, `GI` girar, `SI` simetría, `EQ` equidistancia, `RR` recortar, `AL` alargar, `EM` empalme, `CHA` chaflán, `MA` matriz, `ES` escala, `EST` estirar, `PA` partir, `JUN` juntar, `DESCOMP` descomponer, `B` borrar, `SB` sombreado, `TX` texto, `DI` distancia, `AA` área, `CA` capas… Valen también los nombres completos en español y las órdenes y alias en inglés (`LINE`, `MOVE`, `TRIM`, `O`, `TR`, `EX`…).
- Las herramientas BIM también: `MURO`, `FORJADO`, `PUERTA`, `VENTANA`, `COTA`…
- **Intro o Espacio con la línea vacía repite la última orden.** Esc cancela.
- **Coordenadas** cuando se pide un punto: `3000,2000` absolutas, `@3000,2000` relativas, `@3000<45` polares, o solo `3000` en la dirección del cursor (entrada directa de distancia). En milímetros, como el resto de la interfaz; con `m` o `cm` detrás si se quiere.
- La orden en curso dice qué espera y sus opciones entre corchetes: `CIRCULO Precise centro o [3P/2P]:`; las opciones se eligen escribiendo su letra o con un clic.
- **Teclas inmediatas** (ArchiCAD) como alternativa: M, F, P, V, C, N activan las herramientas BIM al momento, como hasta ahora (interruptor en la barra inferior).

### Forzados a objeto

Además de los actuales: **extremo, punto medio, centro, cuadrante, intersección, perpendicular, tangente y cercano** sobre todo el dibujo 2D, con su marca de AutoCAD (cuadrado, triángulo, círculo, rombo, aspa…). F3 los activa y desactiva todos; **F8 orto** (Mayús sigue fijando 45°).

### Selección

Ventana de izquierda a derecha (lo que queda dentro entero) y **captura** de derecha a izquierda (lo que toca), como AutoCAD. Mayús quita de la selección. Seleccionado, se ven sus **pinzamientos**: arrastrar un extremo estira, el centro mueve.

## 3. Estilos gráficos de vista (3.1)

**Tabla de plumillas** (grosores) por escala, según UNE-EN ISO 128 (0,13 · 0,18 · 0,25 · 0,35 · 0,50 · 0,70 mm): corte, proyección, tramas, anotación. Por defecto la actual; la decisión **D5** de Antonio la ajusta. Cada vista puede ocultar categorías (muros, forjados, carpinterías, cotas, texto, tramas) y capas de dibujo, y tiene su **nivel de detalle** (1:20 con todas las capas, 1:100 simplificado, como hoy).

## 4. Acotación (3.2)

- Cotas **asociativas**: los puntos acotados sobre muros, huecos o dibujo siguen al elemento al moverlo o estirarlo.
- Cotas **angulares, de radio, de diámetro y de longitud de arco**.
- Cotas de **superficie** (de locales) y de altura en sección cuando existan las secciones (2.10).

## 5. Locales y etiquetas (2.9 y 3.3)

- **Locales** (`IfcSpace`): por varita mágica dentro de un recinto o por contorno; nombre, número, uso, superficie útil (y construida), altura. Se exportan e importan en IFC.
- **Etiquetas** asociativas: de puertas y ventanas (marca y medida), de muros (tipo), de locales (nombre, número, superficie). Siguen a su elemento; colocación automática de todas las de una planta.

## 6. Planos (3.5) y salida (3.6)

- **Hojas** A4 a A0 (y personalizadas), verticales u horizontales, con **cajetín propio** de AD5: proyecto, emplazamiento, promotor, plano, número, escala, fecha, autor, revisión.
- **Vistas en la hoja** a su escala (1:20 a 1:500), con su recorte, título y escala debajo. Dibujo propio de la hoja (textos, líneas).
- **PDF vectorial** de una hoja o de todas, y **DXF** (de una planta o de una hoja) con capas. **Importar DXF** como dibujo 2D (topográficos, planos antiguos de AutoCAD).

## 7. Tablas y mediciones (3.7 y 3.8)

- **Tablas de planificación**: carpinterías (marca, tipo, medidas, nº), muros por tipo y capa (longitud, superficie, volumen), forjados (superficie, volumen), locales (superficies). Se ven en una pestaña, se exportan a CSV y se colocan en una hoja.
- **Mediciones en BC3** (FIEBDC-3): capítulos por categoría, partidas por tipo con sus líneas de medición; se abre en Presto o Arquímedes. Es el primer paso de M1–M3 de `docs/DISCIPLINAS.md`.

## 8. Orden de los bloques

| Bloque | Contenido | Estado |
|---|---|---|
| 3A | Elementos de dibujo y capas en el modelo, el fichero (versión 7) y la planta | ✅ v0.25 |
| 3B | Geometría de las órdenes de edición (equidistancia, recortar, alargar, empalme…) con pruebas | ✅ v0.25 |
| 3C | Línea de órdenes, herramientas de dibujo, forzados a objeto y edición en la aplicación | ✅ v0.25 |
| 3D | Estilos gráficos de vista y plumillas | ✅ v0.25 |
| 3E | Acotación completa | ✅ v0.25 |
| 3F | Locales y etiquetas | ✅ v0.25 |
| 3G | Planos y maquetación | ✅ v0.25 |
| 3H | PDF, DXF (exportar e importar) | ✅ v0.25 |
| 3I | Tablas de planificación y mediciones BC3 | ✅ v0.25 |

## 9. Lo que quedó para después

- Cotas de altura en sección (con las secciones, 2.10) y estilos de cota por escala.
- Plantillas de vista con nombre (hoy cada vista guarda su propio estilo).
- Desglose de muros y forjados por capa en las tablas; tablas en pestaña propia (hoy en ventana).
- En DXF: las vistas de un plano van como líneas recortadas (no como ventanas gráficas de espacio papel) y los textos de cotas y etiquetas como trazos.
- Al importar DXF: las tramas que no son sólidas se convierten en «Diagonal»; los sólidos 3D y las imágenes se dejan fuera (el informe dice cuántos).

