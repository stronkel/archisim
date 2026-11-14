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
| Render, cálculos y descargas en otro hilo | En el mismo hilo, con la barra de avance (más lento) | Hilos del navegador (Web Workers) |
| Traer el entorno (Catastro, OSM, IGN) | Soltando los ficheros descargados a mano; la descarga directa la bloquean algunos servidores desde una web | Un pequeño puente en el NAS que haga las descargas |
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
