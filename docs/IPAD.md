# ArchiSim en el iPad

Pedido de Antonio (14/11/2026): **modelar en serio desde el sofá**, con el Apple Pencil, con teclado y trackpad, y también solo con los dedos. Decidido: **primero la web, después la app nativa** (D29).

## 1. Por qué la web primero

- ArchiSim se dibuja con egui sobre wgpu, y los dos funcionan en el navegador. Safari 26 (iPadOS 26) trae WebGPU, así que el 3D usa la gráfica del iPad, sin pasar por WebGL.
- Se prueba con un enlace: sin Xcode, sin App Store, sin cuenta de desarrollador. Cada versión llega al recargar la página.
- **Se sirve desde el NAS de casa con usuario y contraseña** (D30), no en una web abierta: un contenedor con Caddy pide la contraseña y otro descarga solo cada versión publicada (`scripts/nas/`, con sus pasos en `scripts/nas/README.md`). Tailscale Funnel le pone el HTTPS que Safari exige para usar la gráfica.
- Todo lo que se haga para el tacto (gestos, Pencil, botones grandes) vale igual para la app nativa de después. El paso a nativa es sobre todo de empaquetado.

## 2. Cómo se maneja (modo táctil)

La regla es la de los programas de dibujo de tableta: **el Pencil dibuja, los dedos mueven la vista.** Así se pueden dibujar muros con el lápiz sin que un dedo apoyado mueva nada.

| Gesto | Planta, sección, plano | Vista 3D |
|---|---|---|
| Pencil: tocar | Clic: lo que haría el ratón con la herramienta activa (colocar un punto, seleccionar) | Igual |
| Pencil: arrastrar | Arrastrar: dibujar, mover, estirar con los pinzamientos, ventana de selección | Igual |
| Pencil: pasar por encima sin tocar (iPad Pro y Air M2 o posterior) | El cursor: forzados, rastreo y cotas temporales antes de tocar | Igual |
| Un dedo: arrastrar | Desplazar | Girar alrededor de lo que hay debajo |
| Dos dedos: pellizcar / arrastrar | Zoom en el punto / desplazar | Zoom / desplazar |
| Pulsación larga (dedo o Pencil) | Menú del clic derecho | Igual |
| Tocar con dos dedos | Deshacer | Igual |
| Tocar con tres dedos | Rehacer | Igual |
| Doble toque en el Pencil (2.ª generación y Pro) | Volver a la herramienta anterior (configurable: Esc, cotas) | Igual |

**Solo con los dedos.** Un interruptor «El dedo dibuja» en la barra inferior. Con él encendido, un dedo hace lo que el Pencil, desplazar y girar pasan a dos dedos, y al dibujar aparece una **lupa** junto al dedo con el punto forzado, para que el dedo no tape lo que se está colocando. Se enciende solo si el iPad no ha visto nunca un Pencil.

**Con teclado y trackpad.** Funciona como en el ordenador: atajos de Revit, línea de órdenes, cotas tecleadas, clic derecho con dos dedos en el trackpad.

**Sin teclado.** Al escribir una cota temporal (o cualquier número) sale un **teclado numérico propio** junto al punto, con Intro, Tab (siguiente cota), coma decimal, ±, y las teclas de los métodos de la herramienta activa (Espacio para girar, cadena, rectángulo…).

## 3. Interfaz con el dedo

- **Tamaño táctil**: botones de la cinta, fichas, filas de Propiedades y pinzamientos de al menos 44 puntos. Se activa solo en el iPad y en pantallas táctiles, y se puede elegir en Preferencias.
- **Cinta recogida** por defecto en el iPad. Propiedades y Navegador como paneles laterales que se abren y cierran con un toque, para dejar el dibujo a pantalla completa.
- **La Maqueta** como entorno de inicio en el iPad: catálogo de tarjetas y vista 3D, lo más cómodo con el dedo.
- **Pinzamientos más grandes** y tolerancia de selección mayor con el dedo que con el Pencil.
- **Iconos con nombre** en los menús que salen de una pulsación larga: no hay ratón para ver las ayudas al pasar por encima.
- El iPad girado en vertical: los paneles se apilan abajo.

## 4. Qué cambia en la versión web

| Hoy (escritorio) | En la web, primera vuelta | Más adelante |
|---|---|---|
| Abrir y guardar en carpetas | «Abrir» con el selector de archivos del iPad (Archivos, iCloud Drive, Google Drive), «Guardar» descarga el `.archisim`. Copia de seguridad automática en el propio navegador cada pocos minutos | Guardar encima del mismo archivo (en la app nativa) |
| Biblioteca del estudio en una carpeta | La biblioteca de la plantilla, más los ficheros que se carguen a mano | Biblioteca en una carpeta de iCloud o del NAS |
| Render, cálculos y descargas en otro hilo | El render en los hilos del navegador (desde la 0.90); los cálculos, en el mismo hilo | Los cálculos también en los hilos |
| Traer el entorno (Catastro, OSM, IGN) | Por el puente del NAS (desde la 0.90), o soltando los ficheros descargados a mano | — |
| Trabajo en equipo sobre una carpeta compartida | No | En la app nativa |
| Actualizar desde el programa | No hace falta: al recargar está la última | — |
| Exportar IFC, PDF, DXF, BC3, glTF | Igual, como descargas | — |

**Memoria.** Safari limita la memoria de una página. Se medirá con la casa de la demo, con el L-160 y con Florence Street, y en el iPad la vista 3D usará antes las versiones de lejos.

## 5. Orden de trabajo

| Bloque | Qué | Antonio prueba |
|---|---|---|
| T1 | **ArchiSim en el navegador.** La aplicación compila para la web, la integración continua la publica en GitHub Pages en cada versión y abre la demo. Lo que no funciona en la web se apaga con su aviso | Abrir el enlace en Safari del iPad y en el ordenador: la demo en planta y en 3D |
| T2 | **Archivos.** Abrir y guardar `.archisim` e IFC desde Archivos e iCloud; exportaciones como descargas; copia automática en el navegador y recuperación al volver | Abrir un proyecto suyo desde iCloud, cambiarlo, guardarlo y abrirlo en el PC |
| T3 | **Gestos y Pencil.** La tabla de §2: el Pencil dibuja y el dedo mueve, pellizcar, pulsación larga, deshacer con dos dedos, cursor al pasar el Pencil por encima | Dibujar una vivienda pequeña con el Pencil: muros, forjado, puertas, cotas |
| T4 | **Tamaño táctil y teclado numérico.** §3, el teclado numérico de las cotas y el modo «El dedo dibuja» con la lupa | La misma vivienda solo con los dedos, y con el teclado |
| T5 | **Lo pesado.** Render y cálculos en hilos del navegador, medir la memoria con proyectos grandes, el entorno traído desde el NAS | Florence Street en el iPad; un render |
| T6 | **App nativa para iPad**, si la web se queda corta: el mismo programa compilado para iPadOS (Metal), con Archivos e iCloud, guardar encima, carpeta del estudio y trabajo en equipo. Se instala con TestFlight; necesita el MacBook con Xcode y la cuenta de desarrollador de Apple (99 €/año) | Instalarla desde TestFlight |

Los gestos y el tamaño táctil (T3 y T4) sirven también en las pantallas táctiles de Windows y por escritorio remoto.

## 6. Decisiones

- **D29** (14/11/2026, Antonio): iPad por la web primero y la app nativa después; Pencil, teclado y dedos.
- **D30** (14/11/2026, Antonio): la versión web no se publica abierta; se sirve desde el NAS con usuario y contraseña (el código sigue público en GitHub).

## 7. Cómo está hecho (T1)

- `crates/archisim-app` compila también para `wasm32-unknown-unknown`. Lo que solo existe en el escritorio va aparte por plataforma: diálogos de archivo (`file_dialog`: en la web contestan «cancelado» y avisan), hilos (`platform::spawn`: en la web se hace en el momento), descargas (`http_get`), el reloj (`web_time`), el bucle de ventana propio de `main.rs` y lo de macOS. El render usa un solo hilo en la web.
- `web.rs` arranca el programa en el lienzo de `index.html` (con `eframe::WebRunner`) y abre `demo.archisim`, que la compilación pone al lado.
- `scripts/build_web.sh` lo compila con trunk y hace `dist/archisim-web.zip`; la integración continua lo compila en cada cambio, lo abre en Chromium (`scripts/web_smoke.py`: planta y 3D) y lo adjunta a cada versión publicada.
- El programa pesa unos 51 MB (13 MB comprimido). Afinarlo (wasm-opt) va en T5.

## 8. Cómo está hecho (T2: archivos)

- **Abrir** (Archivo → Abrir, Ctrl+O, «Nuevo desde plantilla», Importar IFC): sale una tarjeta sobre el programa con «Elegir archivo…». Safari solo abre el selector de archivos al tocar un botón de verdad, y los toques del programa le llegan un instante tarde; por eso la tarjeta. El selector del iPad deja coger de Archivos, iCloud Drive, el iPad o cualquier app con carpeta. No se le dice qué tipos: en el iPad saldrían en gris los `.archisim`, que no conoce. Abre `.archisim` e IFC; también soltándolos sobre la página.
- **Guardar** (Ctrl+S, Archivo → Guardar): otra tarjeta con «Guardar en Archivos…» (la hoja de compartir del iPad, que deja escoger carpeta, también iCloud Drive) y «Descargar» (a Descargas). Igual para exportar IFC 4.3 e IFC4. Lo demás que se exporta (PDF, DXF, BC3, imágenes…) todavía no; lo dice al pedirlo.
- **Copia en el navegador**: medio minuto después de cada cambio, y al guardar, el proyecto se guarda también en el propio navegador (IndexedDB). Al volver a abrir la página se abre esa copia, con un aviso de cuándo es; si no la hay, la casa de ejemplo. Si se cierra la página con cambios sin guardar, el navegador pregunta antes.
- Las preferencias (tamaño de letra, tema, teclas, misiones hechas) se guardan en el navegador (`prefs`, `localStorage`).
- Código: `web.rs` (`pick`, `deliver`, `keep`, `kept`, `set_unsaved`) sobre `web_files.js`; en la ventana, `ArchiSimApp::web_files` y `open_picked`; `Editor::import_bytes`, `ifc_text`, `mark_unhanded`. La prueba de la integración continua abre la casa con Ctrl+O, importa un IFC, guarda con «Descargar» y comprueba que al recargar abre la copia.

## 9. Cómo está hecho (T3: el Pencil y los dedos)

- `crate::touch` (`Touches::translate`) lee los toques antes que egui (`eframe::App::raw_input_hook`) y pone en su lugar lo que la ventana ya entiende: el lápiz, como el botón izquierdo del ratón; un dedo que se arrastra sobre el dibujo, como la rueda pulsada (en planta, sección, plano o vista de dibujo, desplaza) o el botón derecho (en 3D, gira); dos dedos, la rueda pulsada en su centro (desplaza) y `Zoom` al separarse o juntarse; un toque, un clic; un dedo quieto medio segundo (`LONG_PRESS`), el botón derecho (el menú); dos dedos tocados y levantados enseguida, Deshacer; tres, Rehacer.
- Solo sobre el dibujo: la zona la apunta cada vista al dibujarse (`ArchiSimApp::touch_area`), y no cuenta lo que tapa un menú o un diálogo (capas que no son el fondo). En los paneles, la cinta y los diálogos, un dedo es un ratón.
- **El dedo dibuja** (`Prefs::finger_draws`, botón en la barra de abajo en cuanto se toca la pantalla): un dedo hace lo del lápiz y dos mueven la vista. Viene encendido hasta que el iPad ve un Pencil; desde ese momento se apaga solo (y se recuerda), y se puede cambiar a mano.
- En el navegador, `web_files.js` (`archisim_watch_pencil`) mira qué toques son del Pencil (`touchType` «stylus»), pasa el Pencil que se mueve sobre la pantalla sin tocarla como el ratón que se mueve (en los iPad que lo detectan) y quita la copia «puntero» de los toques, que llega antes que el toque y apretaría el botón izquierdo.
- Pruebas: `touch::tests` (lápiz, arrastre, toque, pulsación larga, pellizco, deshacer y rehacer, dedo que dibuja, paneles) y, en la integración continua, dos dedos sobre la planta que la desplazan y la acercan (`fingers` en `scripts/web_smoke.py`).
- También sirve en el escritorio con pantalla táctil (Windows), donde no se distingue lápiz de dedo.


## 10. Cómo está hecho (T4: tamaño táctil, teclado numérico y lupa)

- **Tamaño táctil** (`Prefs::touch_size`, «Tamaño táctil» en la barra de abajo): se enciende solo en cuanto se toca la pantalla. Botones y filas de al menos 44 × 38 puntos, más separación y barras de desplazamiento anchas que no se esconden (`theme::style` con `touch`); los tiradores, los puntos de forzado y lo que se elige se cogen desde el doble de lejos (`crate::touch::reach`, que usan todas las herramientas donde antes había una distancia fija en píxeles).
- **Teclado numérico** (`app/keypad_ui`, `Prefs::keypad`, «Teclado numérico» en la barra de abajo): abajo a la derecha del dibujo, cuando hay una herramienta en la mano o algo seleccionado. Cifras, coma, menos, Retroceso, Tab (siguiente cota), Intro, Esc y Espacio (girar). Las teclas entran como si fueran del teclado (`keypad_queue`, que `raw_input_hook` pone en el fotograma siguiente), así que valen para las cotas temporales, la línea de órdenes y los métodos de todas las herramientas. Se recoge en un botón «123».
- **El punto no se pierde al tocar fuera.** Con pantalla táctil, el cursor de la planta y de las vistas de dibujo se queda donde dibujó el dedo cuando se toca el teclado o un botón: la cota tecleada sigue la dirección que tenía.
- **Lupa** (con «El dedo dibuja»): mientras un dedo dibuja en planta, un cuadrado encima del dedo enseña lo que hay debajo al doble, con una cruz en la yema y un círculo en el punto que se cogerá (el forzado). Se dibuja con la imagen de la planta que ya está en pantalla (`loupe_source`), sin rehacer nada.
- Prueba: `the_keypad_types_a_length` (con la herramienta Muro, cuatro metros tecleados con el teclado de la pantalla, un toque en el «0» que escribe y no pone punto, Retroceso e Intro), y en la integración continua la planta con el teclado a la vista después de tocar la pantalla (`webgl-7-teclado`).

## 11. Cómo está hecho (T5: lo pesado)

- **Render en los hilos del navegador.** La página no tiene hilos propios, pero puede abrir *workers*: cada uno carga el mismo programa (`web_files.js`: `archisim_run_aside`, con el programa ya compilado, `wasm_bindgen::module()`) y hace un trabajo (`archisim_worker_job`). El render se reparte entre hasta cuatro (uno por núcleo menos uno): cada uno recibe el proyecto y hace las filas que le tocan (`archisim_render::render_rows`, filas `y % n == k`; juntas son la misma imagen que en el escritorio, prueba `rows_in_turn_make_the_whole_picture`). La ventana sigue moviéndose y la barra avanza. Cada worker se cierra al acabar, y con él su memoria. Si el navegador no puede, se hace en la página como antes (con un aviso).
- **El puente del NAS** (`scripts/nas/Caddyfile`, `puente`): `proxy/<servidor>/…` lo pide el NAS al servidor, solo a los de `platform::BRIDGED_HOSTS` (Catastro, Overpass y OpenStreetMap, Nominatim, CartoCiudad, IGN y PNOA), sin la contraseña de ArchiSim, las cookies ni la página de origen. El programa pregunta una vez si está (`proxy/` responde «archisim-proxy») y pasa por él (`platform::bridged`, `web::http_get`, una petición que espera su respuesta). La descarga del entorno va dirección a dirección, una por fotograma (`carto_download_ui::Runner`), para que la ventana diga cómo va; sin el puente, la ventana lo avisa. Pruebas: `the_nas_bridges_every_service` (cada servidor del programa está en la lista y en el `Caddyfile`).
- **Memoria.** `web::memory_bytes` (la memoria de WebAssembly del programa, que solo crece) sale en el medidor (Ctrl+Alt+F) y en `window.archisimMemoria`.
- **Prueba de la integración continua** (`scripts/web_heavy.py`): la página servida por Caddy con el `Caddyfile` del NAS y contraseña. Sin contraseña no entra; el puente responde, CartoCiudad contesta por él y un servidor fuera de la lista no; el programa busca un lugar por el puente (`?prueba=lugar`); un render en borrador en los hilos (`?prueba=render`), con su tiempo; y Florence Street abierto con Ctrl+O, con la memoria al empezar, al abrir y en 3D.
- Pendiente: los cálculos (estructura, instalaciones, «Arreglar») y la escena 3D siguen en la página; el programa pesa unos 51 MB sin optimizar (`wasm-opt`).
