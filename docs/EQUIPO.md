# ArchiSim — Trabajo en equipo

> 22 de octubre de 2026 · v0.38 · rama `bloque/equipo`. Es el 4.6 de la hoja de ruta y el §9.2 de `docs/INTERFAZ.md`, con las decisiones D20 (una carpeta de Google Drive primero), D21 (reservas por zona y por elemento) y D22 (cursor de los compañeros opcional y apagado).

## 1. La idea

Varias personas trabajan a la vez en el mismo proyecto, cada una en su ordenador y en su copia, también sin conexión, como en el trabajo en equipo de ArchiCAD. No hace falta un servidor: el proyecto vive en una **carpeta compartida** (una carpeta de Google Drive sincronizada en el ordenador, de Dropbox o de un NAS del estudio).

- **Publicar** envía tus cambios al equipo, elemento a elemento; antes recibe los de los demás.
- **Recibir** trae lo que han publicado los demás. Lo que tú no has publicado se queda encima.
- **Reservas**: un elemento lo cambia una sola persona a la vez. Se reserva a mano, por **zonas** (un rectángulo o una planta entera) o solo, al cambiar algo que nadie tenía. Lo que dibujas nuevo es tuyo. Lo que otro tiene reservado no se puede cambiar: se le **pide**.
- **Presencia**: en la barra superior, un círculo por persona con sus iniciales y su color (apagado si no está conectada). En la planta, lo reservado por cada uno se ve de su color y las zonas con su nombre.

Google Drive sincroniza **ficheros**, no responde al momento: lo de los demás llega con unos segundos de retraso. Por eso nadie escribe nunca el fichero de otro. Cada persona solo escribe los suyos, y Drive nunca tiene que mezclar dos versiones de un mismo fichero.

## 2. Cómo se usa

1. **Compartir**: abre el proyecto y elige Colaboración → **Compartir en equipo…**. Escribe tu nombre y elige una **carpeta vacía** dentro de Google Drive (en el ordenador: «Google Drive/Mi unidad/…» o una unidad compartida). Comparte esa carpeta con el equipo desde Drive, como cualquier carpeta.
2. **Unirse**: cada compañero elige Colaboración → **Abrir proyecto de equipo…** y elige el fichero `equipo.asequipo` de la carpeta (también vale hacer doble clic en él o arrastrarlo a la ventana). La primera vez escribe su nombre; después el programa le reconoce.
3. **Trabajar**: dibuja y cambia como siempre. Si tocas algo que tiene otro, el programa no lo deja y dice de quién es. **Guardar** (Ctrl+S) guarda tu copia de trabajo en la carpeta, todavía sin publicar. El programa la guarda también solo, cada minuto.
4. **Publicar** (botón de la barra superior, con el número de elementos cambiados): escribe una nota corta en la ventana Equipo («Cocina: tabiques y puertas») y pulsa Publicar. **Recibir** se ilumina con el número de cambios de los demás que te faltan.
5. **Pedir y liberar**: selecciona lo que tiene otro. En Propiedades sale «Reservado por Ana · **Pedir**», y a Ana le llega el mensaje con un botón **Liberar**. Para liberar lo tuyo, primero tienes que publicar.

La ventana **Equipo** (Colaboración → Equipo…, o clic en un círculo de la barra) reúne todo:

- quién está y en qué planta;
- Publicar y Recibir;
- las reservas: de la selección, una zona dibujada, la planta entera, liberar;
- los mensajes;
- el historial de cambios;
- las opciones del cursor.

En Propiedades, cada elemento dice de quién es y quién lo cambió por última vez.

## 3. Qué hay en la carpeta

```
Casa en la playa/
  equipo.asequipo              qué proyecto es (el que se abre)
  base.archisim                el proyecto cuando se compartió
  miembros/antonio.json        cada persona: nombre y color
  cambios/antonio.json         lo que ha publicado cada uno, en orden
  reservas/antonio.json        lo que tiene reservado cada uno
  presencia/antonio.json       cuándo se le vio por última vez y dónde
  mensajes/antonio.json        lo que ha pedido o escrito cada uno
  trabajo/antonio.archisim     su copia de trabajo (con lo no publicado)
```

`trabajo/<nombre>.archisim` es un proyecto normal: se puede abrir aparte si hace falta. Como está en Drive, también sirve para seguir en otro ordenador (por ejemplo, del PC del estudio al portátil). No hay que tener el proyecto abierto en los dos a la vez: si pasa, el programa lo avisa.

## 4. Reglas que garantizan que todos vean lo mismo

- Los cambios se aplican en el **mismo orden en todos los ordenadores**: por un reloj lógico (uno más que todo lo que había visto su autor), no por la hora de cada ordenador.
- **Reservas a la vez**: si dos personas reservan lo mismo antes de que Drive sincronice, se lo queda quien lo reservó primero según ese reloj. El otro recibe un aviso, y sus cambios en ese elemento no se publican.
- **Lo que ya no encaja**: por ejemplo, una puerta en un muro que otro acaba de borrar. Se deja fuera con un aviso, igual en todos los ordenadores.
- **Identificadores**: cada persona numera sus elementos nuevos en un bloque propio de números. Dos personas dibujando a la vez nunca dan el mismo identificador. El `GlobalId` IFC de cada elemento no cambia al publicar ni al recibir.
- Todos deben usar **la misma versión de ArchiSim**. Si alguien publica con una más nueva, los demás reciben el aviso de actualizar.

## 5. Probarlo sin dos ordenadores

- `archisim equipo prueba out/` hace una carpeta `out/equipo` con la casa de la demo compartida entre Antonio y Ana:
  - Antonio reserva tres muros y dibuja una tapia;
  - Ana reserva el jardín como zona y dibuja un muro;
  - los dos publican y Ana le pide un muro a Antonio.
  
  Ábrela con Colaboración → Abrir proyecto de equipo… como «Antonio Fernández» o «Ana García». La orden escribe también `equipo-recibido.archisim` y `.ifc`.
- En un mismo ordenador se puede abrir ArchiSim dos veces y abrir la carpeta con dos nombres distintos: el menú pregunta siempre quién la abre.
- `archisim equipo <carpeta>` dice quién está y qué ha publicado y reservado cada uno. `archisim equipo <carpeta> recibido.archisim` guarda el proyecto tal como lo recibe cualquiera.

## 6. Pendiente

- Un servidor propio delante de la carpeta, para la presencia al momento (hoy, unos segundos con Drive).
- Compactar la carpeta: hoy cada publicación queda para siempre en `cambios/` (ocupa poco: solo los elementos cambiados).
- Recibir solo, sin pulsar (hoy se avisa con el número en «Recibir»).
- Reservar tipos y materiales desde el diálogo de tipos: hoy se reservan solos al cambiarlos.
- Con modelos muy grandes, cada cambio compara el proyecto entero para saber qué se ha tocado. Va bien con proyectos de vivienda; con miles de muros habrá que medirlo.
