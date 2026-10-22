# Biblioteca básica de modelos 3D (L3)

Este documento explica para Antonio cómo hace ArchiSim el 3D de los muebles, sanitarios, la cocina, las personas y los coches de la biblioteca, y qué se puede cambiar.

## La idea

Con los árboles (`docs/VEGETACION.md`) ya hacemos lo mismo: ArchiSim no descarga modelos de nadie. **Genera el 3D a partir de las medidas del tipo.** Un sofá de 2,40 m con tres plazas sale con tres cojines de asiento y tres de respaldo, y si lo alargas a 3 m con cuatro plazas, se rehace con cuatro. Así:

- no hace falta pedir licencia a nadie (sala limpia, sin catálogos de terceros);
- el modelo siempre coincide con el símbolo de planta y con las medidas de Propiedades;
- pesa poco: cada mueble tiene entre unos cientos y unos pocos miles de triángulos.

El dibujo de planta sigue saliendo de la receta de la familia (`plan 1:50`, `plan 1:100`); lo que cambia es el 3D, que antes eran cajas y ahora es un modelo con detalle.

## Qué hay

| Familia | Modelos 3D |
|---|---|
| Sofá | Contemporáneo, Clásico (brazos enrollados y patas torneadas), Nórdico (patas de madera inclinadas) |
| Silla | Madera, Tubo metálico, Tapizada |
| Mesa | Cuatro patas, Patas metálicas, Caballetes; con sus sillas alrededor si se piden |
| Mesa de centro | Madera con balda, Bloque, Vidrio y metal |
| Cama | Tapizada, Madera, Tarima; con almohadas y edredón |
| Mesita de noche | Cajonera, Con patas; con lámpara |
| Armario, Estantería (con libros), Mueble TV (con televisor) | uno cada uno |
| Muebles de cocina | Zócalo, cuerpos, puertas con tiradores, encimera con fregadero, grifo y placa |
| Muebles altos, Frigorífico combi, Lavadora | uno cada uno |
| Inodoro, Bidé | De pie, Suspendido |
| Lavabo | De pedestal, Con mueble, Suspendido |
| Bañera | Rectangular, hueca de verdad |
| Ducha | Plato, Plato y mampara, Esquina con mampara |
| Persona | Hombre, Mujer, Caminando (la altura se cambia) |
| Coche | Berlina, Compacto, Todoterreno (el largo se cambia) |

La plantilla trae 19 tipos nuevos para enseñarlos (Sofá clásico de 3 plazas, Sofá nórdico de 2 plazas, Silla metálica, Silla tapizada, Mesa de caballetes 200×100, Mesa de patas metálicas 140×80, Mesa de centro, Mesa de centro de vidrio, Mesita de noche, Mueble TV 180, Cama de madera 160×200, Cama tarima 150×200, Inodoro suspendido, Bidé suspendido, Lavabo con mueble 80, Lavabo suspendido 60, Ducha con mampara 120×80, Mujer, Todoterreno) y diez materiales para ellos (Tela, Tela clara, Edredón, Cromado, Negro mate, Libros, Libros claros, Encimera, Piloto, Vidrio doble).

## Cómo se cambia

En Propiedades del tipo (o en Tipos y materiales), cada familia tiene un grupo **Modelo 3D**:

- **Modelo 3D**: el estilo (Contemporáneo, Clásico…).
- Un material por pieza: tapicería, cojines, patas, encimera, vidrio, cromados… Cambia el material y cambia el color o la textura en 3D y en el IFC.

Las medidas de siempre (ancho, fondo, alto, plazas, sillas…) mueven el modelo.

## En el IFC

Cada tipo lleva su modelo como mapa de representación (`IfcRepresentationMap` con `IfcTriangulatedFaceSet` y el color de cada pieza), igual que los modelos importados de L2: los ejemplares lo comparten, así que un comedor con veinte sillas iguales no repite veinte veces la malla.

## Para el programador

- Crate `archisim-props` (sin dependencias): `make(kind, params)` devuelve un `PropMesh` con triángulos por *papel* (`mat`, `frame`, `cushion`, `glass`, `metal`…); `mesh_of` lo guarda. Piezas en `build` (cajas redondeadas, cilindros, tornos, elipsoides, huecos).
- La familia lo pide con `outputs generator = "props:<kind>"` y el parámetro `model3d`; `archisim_core::Model::type_prop` traduce los valores de la familia y asigna a cada papel el material de la familia (`candidates`).
- 3D: `archisim_geom::solid::prop_solids`. IFC: `archisim_ifc::objects::mesh_map`.
- `archisim modelos out/` hace `modelos.archisim`, `modelos.ifc`, `modelos-3d.svg` (axonometría) y `modelos-planta.svg` con los 41 objetos en filas.

## Pendiente

- Más objetos: electrodomésticos de encastre, campana, aparatos de iluminación, mobiliario de oficina, plantas de interior en maceta.
- Un cuarto nivel de detalle para las vistas lejanas de modelos muy grandes (hoy ya son ligeros).
- Biblioteca ampliada descargable con modelos CC0 (Poly Haven, Kenney) importados por L1.
