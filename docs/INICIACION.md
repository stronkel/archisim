# Letra grande y misiones de iniciación (I6)

Último bloque de la interfaz por disciplinas (`docs/INTERFAZ.md` §6, «Para todas las edades»).

## Tamaño de letra

Tres tamaños: **Normal**, **Grande** (120 %) y **Muy grande** (140 %). Crece toda la ventana a la vez: textos, fichas de herramientas, barra bajo las pestañas, Propiedades, navegador y diálogos, así que nada se descoloca ni se corta. El dibujo de la planta sigue nítido (se rasteriza a la resolución de la pantalla).

Dónde se cambia:
- **Vista → Tamaño de letra**.
- En la tarjeta de bienvenida, la primera vez que se abre el programa.
- En la ventana de las misiones (Ayuda → Misiones de iniciación).

El programa lo recuerda en este ordenador, junto con el tema claro u oscuro y las teclas de AutoCAD (fichero `preferencias.json` en la carpeta de ajustes de ArchiSim, la misma que la de la biblioteca y el equipo).

## Misiones de iniciación

Tareas cortas, opcionales, que enseñan dónde está cada cosa sin estorbar al que ya sabe. La primera vez que se abre ArchiSim aparece una tarjeta: «¿Primera vez con ArchiSim?», con Empezar o Ahora no. Después están siempre en **Ayuda → Misiones de iniciación**.

Mientras se hace una, una tarjeta abajo a la derecha del dibujo dice el paso («Pulsa la ficha Muro (o la tecla M)»), y **el control que hay que usar se ve rodeado en azul**, latiendo. Si no está a la vista (una ficha de otra disciplina, una vista del navegador con la pestaña Modelo puesta), se rodea el control que lo trae (la disciplina, la pestaña Proyecto). Cada paso se marca solo al hacerlo: el programa mira lo que ha pasado, no pregunta. Si se hace algo por otro camino (la tecla en vez de la ficha, el buscador), también vale. «Saltar paso» y «Dejar la misión» siempre están.

| Misión | Pasos |
|---|---|
| Tu primer muro | Disciplina Arquitectura → ficha Muro → método Rectángulo → dos clics (o 8000x6000 e Intro) |
| Puertas y ventanas | Ficha Puerta → clic en un muro (Espacio la gira) → ficha Ventana → clic en un muro (cota azul para colocarla) |
| Tu primer forjado | Ficha Forjado → método Varita → clic dentro de los muros |
| Míralo en 3D | Pestaña Proyecto del navegador → Vista 3D → volver a la planta por su pestaña |
| Tu primera planta | «Plantas…» → «Añadir planta encima» → abrir la planta nueva en el navegador |
| Tu primer plano | «+ Nuevo plano» → arrastrar una planta al papel |

Al terminar cada una, la tarjeta dice qué probar después y ofrece la siguiente. Las hechas se marcan con ✓ en la ventana; «Empezar de cero» quita las marcas.

## Para el programador

- `crates/archisim-app/src/missions.rs`: las misiones (`MISSIONS`), lo que la ventana ve en cada momento (`Obs`) y el avance (`Progress`), sin interfaz y con pruebas.
- `crates/archisim-app/src/prefs.rs`: `Prefs` y `TextSize` (factor de zoom de egui); en las pruebas no se lee ni se escribe nada.
- `app/missions_ui.rs`: tarjetas, ventana y anillo. Los controles se apuntan con `missions_ui::mark(ctx, "tile:Muro", rect)` al dibujarse (fichas, disciplinas, métodos de la barra contextual, segmentos, botones del navegador, filas de plantas y de la Vista 3D, pestañas de planta, «Añadir planta encima»). Para una misión nueva: sus pasos en `MISSIONS`, con la marca del control a usar; si el control no tiene marca, añádesela donde se dibuja.
- La prueba `onboarding_missions_by_clicks` hace las seis pulsando los controles marcados.
