# ArchiSim — Entorno Maqueta

> 23 de octubre de 2026 · Diseño acordado con Antonio en conversación el mismo día, a partir de su idea: un editor tan intuitivo como Los Sims y, sobre todo, tan modular como Paralives, para arquitectura, formas y urbanismo, con una imagen más cuidada (cartoon, pero con sombras) sin castigar a un MacBook de 2016. **Lo que más importa es la fluidez.**
> Rama `bloque/maqueta`.

## 1. La idea

La maqueta **no es otro programa ni otra disciplina**: es otra forma de presentar la misma ventana, sobre el mismo modelo. Las disciplinas dicen *qué* se modela (estructura, instalaciones, urbanismo); la maqueta dice *cómo* se maneja: a ojo, en 3D, eligiendo cosas de un catálogo.

- Un interruptor **Técnica · Maqueta** arriba a la derecha, en el mismo sitio en los dos entornos, y la tecla **Tab** (salvo escribiendo en un campo de texto).
- Todo lo que se hace en la maqueta está a la vez en la planta técnica, las mediciones, el CTE y el IFC, y al revés: son los mismos muros, forjados, cubiertas y objetos.
- **Lo seleccionado sigue seleccionado al cambiar de entorno**: eliges el sofá en la maqueta, pulsas Técnica y lo tienes en la planta con sus propiedades.

Así un boceto con el cliente es ya el proyecto, y quien no sabe BIM (el equipo que viene de AutoCAD, un estudiante) puede empezar por la maqueta y pasar a la técnica sin cambiar de programa ni de fichero.

## 2. Dos capas

**Regla**: lo que es una *herramienta* va a los dos entornos; lo que es una *forma de presentarla* (catálogo grande, estilos en vez de tipos, menos texto), solo a la maqueta.

### 2.1 Lo común (mejora ArchiSim entero)

| Qué | Dónde se nota |
|---|---|
| **Muros recortados** en 3D: arriba, recortados a 1 m, solo los de delante recortados, solo planta | Vista 3D de los dos entornos |
| **Habitación** como método de Muro: arrastrar un rectángulo hace sus muros, su forjado y su local | Barra contextual de Muro |
| **Cuentagotas**: clic en un elemento y el siguiente sale igual (tipo, colores, parámetros) | Barra contextual |
| **Buhardillas y ventanas de cubierta** que se pinchan en un faldón y se adaptan a él (§5) | Arquitectura → Cubierta, y Construir |
| **Tejas, caballetes, canalones y bajantes** de las cubiertas (§5) | Propiedades de la cubierta |
| **Imagen** (§4): sombras, penumbra en rincones, luz de ilustración, texturas ligeras, niveles de calidad | Vista 3D |

### 2.2 Lo propio de la maqueta

```
┌──────────────────────────────────────────────────────────────────────────┐
│ ↶  ▲ Planta baja ▼  · Muros recortados · Cuadrícula     Técnica│Maqueta │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│                     Vista 3D (casi toda la ventana)        ┌──────────┐  │
│                                                            │ Sofá     │  │
│                                                            │ ● ● ● ●  │  │
│                                                            │ ↻ ⧉ ⌖ 🗑 │  │
│                                                            └──────────┘  │
├──────────────────────────────────────────────────────────────────────────┤
│ Construir · Amueblar · Exterior │ Salón · Cocina · Baño · Dormitorio  🔍 │
│ [miniatura] [miniatura] [miniatura] [miniatura] [miniatura] [miniatura]  │
└──────────────────────────────────────────────────────────────────────────┘
```

- **Sin columnas**: sin barra de disciplinas, fichas, Propiedades ni navegador. El 3D ocupa casi toda la ventana.
- **Barra superior reducida**: deshacer, planta activa con subir y bajar, muros recortados, cuadrícula y el interruptor.
- **Catálogo abajo** con miniaturas, en tres grupos que sustituyen a fichas y tipos a la vez:

  | Grupo | Temas | Qué hay |
  |---|---|---|
  | **Construir** | Habitaciones · Muros · Puertas · Ventanas · Cubiertas · Escaleras | Habitación, muro, forjado; los tipos de puerta y ventana del proyecto; cubierta a dos y a cuatro aguas, buhardillas y ventana de cubierta; escalera y barandilla |
  | **Amueblar** | Salón · Comedor · Cocina · Baño · Dormitorio · Oficina | Los objetos del proyecto y de la biblioteca, por la estancia que dice su familia |
  | **Exterior** | Terreno · Vegetación · Caminos · Mobiliario urbano · Formas | Especies del vivero, plantación, vial, mobiliario urbano, volúmenes de Concepto |

  Una miniatura *es* un tipo (o una herramienta con su tipo): al pulsarla se activa la herramienta de siempre con ese tipo. Se colocan igual que en técnica: la vista previa sigue al ratón, Espacio gira, clic suelta.
- **Tarjeta flotante** junto a lo seleccionado, en lugar de Propiedades: qué es y su tipo, **girar** un cuarto de vuelta, **duplicar** (una copia al lado), **cuentagotas**, **borrar** y **Más…** (las Propiedades completas en una ventana). Las medidas se cambian con los tiradores 3D que ya existen. Los colores rápidos por pieza llegan con la rueda de color (MQ2).
- **Estilos en lugar de tipos**: lo que sale del catálogo usa los tipos del proyecto, pero se ve por su nombre y su dibujo. Quien quiera los tipos los tiene en Más….
- **Cuadrícula en el suelo** de la planta activa (50 cm), encendida la primera vez que se entra en la maqueta; los clics sobre el suelo se ajustan a ella solo cuando se ve. (Sin tecla propia: la G ya es la viga.)

## 3. Fluidez

- La maqueta no añade trabajo por fotograma: el catálogo dibuja sus miniaturas una vez y las guarda; la tarjeta es un panel de la interfaz.
- Los muros recortados se hacen al dibujar (el sombreador descarta lo que queda por encima de la altura de recorte), sin rehacer la escena.
- Objetivo: el MacBook Pro de 2016 a 60 imágenes por segundo en calidad *Ligera* con la vivienda de prueba. Si no, no se da por bueno.

## 4. Imagen: cartoon cuidado y ligero (bloque V1)

Técnicas de juego que corren en portátiles viejos:

1. **Sombras del sol** del proyecto: un mapa de sombras (una pasada más), con el borde suavizado.
2. **Penumbra en rincones calculada una vez**, al cambiar el modelo, y guardada en los vértices: cero coste por fotograma.
3. **Luz de ilustración**: sombras de tono frío y sol cálido, en lugar de oscurecer a gris.
4. **Texturas como pequeñas imágenes** que ArchiSim dibuja al arrancar (256 píxeles, sin descargar nada): la tarjeta solo las lee, que es lo más barato que hace. Es lo que devolverá las texturas a los Mac con Intel, cuyo controlador se colgaba con las texturas calculadas.
5. Cielo degradado, bruma lejana y suelo de maqueta.

Tres niveles (**Ligera**, **Normal**, **Bonita**) elegidos solos según el ordenador, y **la imagen mejora al parar**: mientras se gira, menos resolución y sombra dura; al soltar el ratón se afina sola.

## 5. Cubiertas: buhardillas, ventanas de cubierta, tejas y canalones

Pedido por Antonio el 23/10/2026 al probar inserciones: la buhardilla no se podía colocar porque **no existía como elemento** (una ventana normal solo admite muros como anfitrión).

### 5.1 Buhardillas

Un elemento propio que vive en una cubierta: clic en un faldón y se adapta a él.

| Tipo | Cómo es |
|---|---|
| **A dos aguas** | Frente con su piñón, cumbrera horizontal que muere en el faldón, limahoyas |
| **A un agua** | Cubierta con menos pendiente que el faldón, de la cabeza del frente al faldón |
| **Plana** | Cubierta horizontal |
| **Ventana de cubierta** | En el plano del faldón, con su cerco, su vidrio y su babero (`IfcWindow` `SKYLIGHT`) |

- **Parámetros**: ancho, altura del frente, pendiente propia, retranqueo desde el alero (se fija al pinchar), ventana del frente (ancho y alto) y materiales (frente y laterales, carpintería, cubierta propia).
- Se coloca por el punto medio del frente, sobre el faldón; el frente mira siempre aguas abajo. Se mueve arrastrándolo y sigue al faldón si la cubierta cambia (si el punto queda fuera de la cubierta, se avisa y no se dibuja).
- **La cubierta se corta sola** donde está la buhardilla; los laterales bajan hasta la cara inferior del faldón.
- **IFC**: la buhardilla es un `IfcElementAssembly` (`ObjectType` «Buhardilla») con su frente y sus laterales (`IfcWall`), su cubierta (`IfcSlab` `ROOF`) y su ventana (`IfcWindow`); el hueco en el faldón, un `IfcOpeningElement`. Se lee de vuelta por su `ArchiSim_Dormer`.

### 5.2 Tejas y canalones

Propiedades de la cubierta → **Cubrición**:

- **Tejas**: ninguna (la capa del tipo, como hoy), teja árabe, teja mixta, teja plana, pizarra o chapa. En 3D, hiladas con su escalón y, en la árabe y la mixta, las cobijas en relieve; **caballete** sobre cumbreras y limatesas. En planta, el rayado de las tejas en el sentido de la pendiente. Es la piel exterior de la cubierta; las capas del tipo siguen debajo.
- **Canalón** en los aleros (no en los piñones): media caña o cuadrado, con su desarrollo y material (zinc, cobre, aluminio lacado, PVC). **Bajantes** en los extremos de cada alero (y cada 10 m como máximo), hasta la planta más baja.
- **Mediciones**: m² de cubrición y número de tejas por m², ml de caballete, ml de canalón y ml de bajante, cada uno en su partida.
- **IFC**: las tejas como `IfcCovering` (`ROOFING`) de cada faldón; canalones (`IfcPipeSegment` `GUTTER`) y bajantes (`IfcPipeSegment` `RIGIDSEGMENT`) en un sistema de saneamiento de pluviales.

## 6. Hecho en v0.48 (rama `bloque/maqueta`)

- **K1 y K2 completos**: buhardillas a dos aguas, a un agua y planas y ventanas de cubierta (herramienta Cubierta → «Buhardilla» o «Ventana de cubierta» en la barra de arriba, o desde el catálogo), con sus Propiedades (clase, ancho, altura del frente, pendiente, ventana, materiales); tejas, caballetes, canalones y bajantes en Propiedades de la cubierta → Cubrición. Planta de cubiertas, 3D, render, IFC (ida y vuelta), mediciones y BC3. `archisim cubiertas out/` hace un ejemplo con todo.
- **MQ1, primera vuelta**: interruptor y Tab; ventana sin columnas; catálogo Construir · Amueblar · Exterior con sus temas y miniaturas (las de planta de los tipos; las 3D, pendientes); tarjeta flotante con girar, duplicar, cuentagotas, borrar y Más…; plantas arriba y abajo; **muros recortados** (arriba, recortados a 1,20 m, bajos a 30 cm, también en la vista 3D técnica, con sus cortes rellenos); cuadrícula de 50 cm con ajuste; **Habitación** (cuatro muros, su suelo y su local de una vez; desde el catálogo).

Pendiente de MQ1: «solo los muros de delante recortados», miniaturas en 3D, arrastrar objetos por el suelo en 3D sin herramienta, que la habitación sea un solo paso de Deshacer (hoy son tres: muros, suelo y local).

## 6 bis. Manejo común y barra del objeto (v0.49, rama `bloque/manejo`)

Del concepto de interfaz que Antonio aprobó el 24/10/2026 (tableros 1, 2 y 6), la primera vuelta:

- **Ratón, igual en planta y en 3D**: el botón izquierdo selecciona (clic), abre una ventana de selección arrastrando en vacío (de izquierda a derecha, lo que queda dentro; de derecha a izquierda, lo que toca) y **mueve** arrastrando lo seleccionado o un mueble; la rueda pulsada desplaza la vista y girarla acerca y aleja; el **botón derecho mantenido gira** la vista 3D (en planta, desplaza). Así, girar la vista ya no mueve nada sin querer.
- **Menú del clic derecho** (un solo clic, sin arrastrar) sobre un elemento: Propiedades, Cortar, Copiar, Pegar, Duplicar, Ocultar, Aislar, Seleccionar todos los de este tipo, Llevar a la planta y Borrar; en vacío: Pegar, Seleccionar todo y Mostrar todo. Ocultar y Aislar son de la sesión (no se guardan ni cambian el IFC). Cortar no borra nada: Pegar lo mueve donde se abrió el menú. Ctrl+C, Ctrl+X y Ctrl+V hacen lo mismo.
- **Tiradores con forma propia**: flecha azul para estirar, en la dirección en que crece; **crece solo hacia la flecha** y el lado opuesto se queda donde estaba (punto blanco mientras se estira); con **Alt**, por los dos lados. **Punto naranja** delante del objeto, sobre su anillo, para girarlo a mano: de 15 en 15°, y de grado en grado lejos del objeto. En 3D, las flechas están un poco fuera del objeto, para que un mueble pequeño se pueda coger por su cuerpo.
- **Sin el cartel de teclas sobre el dibujo**: lo que hace el clic con la herramienta en uso está en gris al final de la barra de arriba (el resto, al dejar el cursor encima).
- **Barra del objeto** en la Maqueta, en lugar de la tarjeta flotante: abajo a la izquierda, sobre el catálogo, su dibujo y su nombre; Girar, Invertir (puertas, ventanas y muros), Duplicar, Igualar (cuentagotas), Borrar; **sus materiales** como muestras redondas que abren los colores del proyecto (solo cambian ese objeto: si su tipo lo comparten otros, se le hace uno propio); Más…
- **Puertas y ventanas en alzado** en las tarjetas del catálogo y en la barra del objeto, con el nombre entero en dos líneas.

Pendiente del concepto: catálogo en árbol (grupo → categoría con iconos → tipo), módulos de cocina con encimera continua y sus estilos, fichas y diálogos unificados (sin ventanas que aparecen en una esquina), y el plano de carpinterías.

## 7. Bloques

| Bloque | Contenido |
|---|---|
| **K1** | Buhardillas y ventanas de cubierta (§5.1) |
| **K2** | Tejas, caballetes, canalones y bajantes (§5.2) |
| **MQ1** | Entorno Maqueta: interruptor, ventana sin columnas, catálogo, tarjeta flotante, cuadrícula, muros recortados, habitación arrastrando, cuentagotas |
| **V1** | Imagen (§4) |
| **MQ2** | Muebles modulares que se unen solos (encimeras, estanterías, bancos corridos), rueda de color por pieza |
| **MQ3** | Pinceles de Exterior: subir, bajar, alisar y aplanar terreno; pintar césped o grava; esparcir plantas; caminos con una curva arrastrable |
| **MQ4** | Flechas y aros sobre los volúmenes de Concepto para empujar, girar y escalar |
| **UI1** | Manejo común: ratón, menú del clic derecho, tiradores en flecha y anillo, estirar por un lado, barra del objeto (§6 bis) |
| **UI2** | Catálogo en árbol y módulos de cocina con encimera continua y estilos |
| **UI3** | Fichas, diálogos y avisos unificados; plano de carpinterías con alzados |
