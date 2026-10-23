# Comprobación del Código Técnico (4.7)

> v0.31 · 20 de octubre de 2026. Crate `archisim-cte`; ventana **Colaboración → Comprobación CTE…** y orden `archisim cte proyecto.archisim [informe.csv]`.

ArchiSim lee el modelo tal como está dibujado y dice qué cumple y qué no del Código Técnico. No sustituye al técnico: comprueba lo que se puede medir en el modelo y avisa de lo que el dibujo no dice.

## Datos del edificio

Se guardan en el proyecto (formato `.archisim` versión 12) y se cambian en la propia ventana:

- **Uso del edificio** (Anejo SI A): Residencial Vivienda, Residencial Público, Administrativo, Docente, Hospitalario, Comercial, Pública Concurrencia o Aparcamiento.
- **Accesibilidad** (DB-SUA 9): no se comprueba, itinerarios accesibles (zonas de uso público con sus aseos accesibles) o vivienda accesible para usuarios de silla de ruedas.
- **Extinción automática**: con rociadores en todo el edificio, los recorridos máximos crecen un 25 % (SI 3, tabla 3.1, nota 1).

## Lo que dice cada local

El **uso** de cada local (o su nombre, si no tiene uso) dice qué es. Se reconocen, sin importar mayúsculas ni tildes: despacho/oficina, aula, laboratorio, taller, gimnasio, biblioteca o sala de lectura, sala de espera, habitación u hospitalización, comedor/restaurante, bar/cafetería, cocina o zona de servicio, vestíbulo/recibidor/portal/entrada/recepción, pasillo/distribuidor/corredor/rellano, aseo/baño/vestuario, «aseo accesible» o «aseo adaptado», dormitorio, salón/estar/estancia, terraza/balcón, almacén/archivo/trastero, instalaciones/cuarto de máquinas/limpieza, garaje/aparcamiento, escalera y las zonas comunes de un edificio de viviendas («Zona común», «Portal», «Rellano», «Escalera», «Galería»).

## Ocupación (SI 3, 2)

Cada local: superficie útil ÷ densidad de la **tabla 2.1** para su uso dentro del uso del edificio (p. ej. un despacho en un edificio administrativo, 10 m²/persona; un aula, 1,5; un comedor de restaurante, 1,5; una vivienda, 20). Los pasillos y escaleras no suman ocupación (uso alternativo, 2.2) y las salas de instalaciones tienen ocupación nula. La ocupación de cada planta se redondea al alza en su conjunto.

Un local puede fijar su propia densidad en **Propiedades → Ocupación (CTE)** (vacío: la de su uso; 0: ocupación nula).

## Evacuación (SI 3)

- **Salidas.** En la planta de acceso (la más cercana al ±0,00) cada puerta que da al exterior es salida de edificio. En las demás plantas, la salida de planta es la **escalera** que lleva hacia la planta de acceso: su desembarco al bajar, o su arranque en un sótano.
- **Recorridos** (tabla 3.1): desde cualquier punto de cada local ocupado hasta la salida más cercana, andando por dentro de los locales (rodeando sus esquinas) y cruzando las puertas y pasos. Límite: 25 m con una sola salida (35 m en aparcamiento), 50 m con varias (35 m donde se duerme: residencial público y hospitalario), +25 % con rociadores. Con una sola salida, la planta no puede pasar de 100 personas (en viviendas, 500 en todo el edificio). Al pulsar el resultado en la ventana se dibuja en la planta el recorrido más largo.
- **Residencial Vivienda.** Los recorridos empiezan en la puerta de cada vivienda (Anejo SI A, origen de evacuación): se comprueban en las zonas comunes, desde las puertas que dan de una vivienda a ellas. Una vivienda unifamiliar no tiene recorridos que comprobar.
- **Puertas y pasos** (tabla 4.1): anchura libre A ≥ P/200 ≥ 0,80 m, donde P son las personas que pasan por ella siguiendo su recorrido más corto; hoja entre 0,60 y 1,23 m. La anchura libre es la del hueco menos los dos cercos.
- **Pasillos** (tabla 4.1): A ≥ P/200 ≥ 1,00 m, medida como el círculo más grande que cabe en el pasillo.
- **Puertas** (6): las previstas para más de 50 personas, abatibles de eje vertical; abren en el sentido de la evacuación las de un recinto de más de 50 ocupantes y las de salida para más de 100 personas (200 en viviendas).

## Accesibilidad (SUA 9 y Anejo A)

- **Itinerarios accesibles**: puertas con 0,80 m libres en una hoja y 0,78 m con la hoja abierta (descontando 4 cm de hoja); pasillos de 1,20 m (1,10 m en zonas comunes de viviendas); giro de Ø 1,50 m libre de obstáculos en vestíbulos; aseos accesibles con giro de Ø 1,50 m y puertas hacia fuera o correderas. Aviso si hay aseos y ninguno es accesible.
- **Vivienda accesible**: pasillos de 1,10 m; puertas como arriba; giro de Ø 1,50 m en vestíbulo, estancia principal (la mayor de salón, estar, estancia o comedor), todos los dormitorios, cocina y al menos un baño (con puertas hacia fuera o correderas); Ø 1,20 m en terrazas. Aviso si hay escaleras (no se admiten escalones).
- Los **círculos de giro** descuentan el mobiliario (los objetos colocados) y las escaleras del local.

## Escaleras y rampas (SUA 1, 4)

- **Escaleras**: la comprobación de cada escalera (huella, tabica, 2C + H, anchura, tramos, altura por tramo y pasamanos, en uso general o restringido). Aviso si una escalera de uso restringido está en un edificio que no es de viviendas.
- **Rampas**: los forjados inclinados entre el 4 % y el 20 %. Uso general, hasta el 12 %; en un itinerario accesible, 10 % si miden menos de 3 m, 8 % si menos de 6 m y 6 % en el resto, con tramos de 9 m como máximo.

## Agua y saneamiento (HS 4 y HS 5) · v0.48

Sobre las tuberías dibujadas con la herramienta Tubería, en cada planta:

- **Aparatos sanitarios**: los objetos que ArchiSim reconoce como inodoro, bidé, lavabo, ducha, bañera, fregadero (el mueble de cocina con fregadero), lavadora o lavavajillas (por el modelo 3D que genera ArchiSim, o porque el nombre del tipo lo dice: «Lavabo…», «Inodoro…»).
- **Desagüe** (HS 5, tabla 4.1, uso privado): la tubería de saneamiento más cercana a cada aparato (a menos de 1,5 m) tiene que tener al menos el diámetro de su derivación: lavabo y bidé 32 mm; ducha, bañera, fregadero, lavavajillas y lavadora 40 mm; inodoro con cisterna 100 mm. Se da también su número de unidades de desagüe (UD).
- **Agua fría y caliente** (HS 4, tabla 4.2, cobre o plástico): la tubería de agua fría (y la de caliente, salvo inodoro y lavadora) más cercana tiene que tener al menos 12 mm; bañera y lavadora, 20 mm.
- Si en el proyecto hay saneamiento (o agua) dibujado y un aparato no tiene ninguna tubería cerca, sale un aviso. Si no hay ninguna tubería dibujada, no se comprueba.
- **Pendientes del saneamiento**: las derivaciones (menos de 75 mm) entre el 2 % y el 5 %; los colectores colgados, al menos el 1 %; los enterrados (los que bajan por debajo de su planta), al menos el 2 %.
- El diámetro que se compara es el dibujado (la herramienta pide el exterior); los mínimos del CTE son nominales, así que se admite 1,5 mm menos.

**A comprobar por Antonio**: los valores de las tablas 4.1 de HS 5 y 4.2 de HS 4 están escritos de memoria del texto del CTE y no los he podido contrastar con el documento oficial desde aquí. Las pendientes sí están contrastadas.

## Superficies

Superficie útil de cada local, por uso y por planta; superficie construida de cada planta (lo que encierran las caras exteriores de los muros, con los muros, sin patios de más de 1 m²) y del edificio, con la ocupación.

## Lo que no se comprueba todavía

- Escaleras protegidas y sectores de incendio (SI 1), y la hipótesis de bloqueo de salidas.
- La longitud hasta el punto donde hay dos recorridos alternativos.
- Anchura de las escaleras de evacuación por ocupación (tabla 4.1, P/160).
- Barandillas: altura según la diferencia de cota (0,90 / 1,10 m).
- Superficies mínimas de las normas de habitabilidad de cada comunidad o del planeamiento: no son del CTE.
- Ascensores accesibles, plazas de aparcamiento accesibles y la reserva de plazas.
- Diámetros de bajantes y colectores por las unidades de desagüe que recogen (HS 5, tablas 4.3 a 4.5), caudales y diámetros de los tramos de la red de agua (HS 4, tablas 2.1 y 4.3), y la distancia de los aparatos a la bajante.

## Fuentes

Texto del CTE DB-SI (sección SI 3) y DB-SUA (Anejo A, SUA 1 y SUA 9) consultado en normatia.com el 20/10/2026. Pendientes de saneamiento de HS 5 contrastadas en certicalia.com («Pendiente mínima de saneamiento según el CTE») el 23/10/2026.
