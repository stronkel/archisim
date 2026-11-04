#!/usr/bin/env python3
"""Writes the recipes of the world's buildings (archisim-library/taller/edificio_mundo_*.asfam)
from the skins and materials of archisim-props. Run from the root of the repository."""
import re
src=open('crates/archisim-props/src/fine/buildings/world/mod.rs').read()
mats=re.findall(r'\("([a-z_]+)", "([^"]+)", "[a-z]+", \[',src.split('pub const MATERIALS')[1].split('];')[0])
sk=open('crates/archisim-props/src/fine/buildings/world/skins.rs').read()
seen=[]
for k,n in re.findall(r'key: "([a-z_]+)",\n\s+name: "([^"]+)"',sk):
    if (k,n) not in seen: seen.append((k,n))
names=", ".join(f"{i} {n}" for i,(k,n) in enumerate(seen))
def recipe(family, desc, gen, dims, h, extra=""):
    out=[f'family "{family}"','  category street','  host none','  version 2',f'  description "{desc}"','  folder "Taller · Edificios del mundo"','','type_params','  section "Lugar"',
         f'    skin integer = 0 range 0..{len(seen)-1} label "Lugar ({names})"',
         '    variant integer = 1 range 0..99 label "Variante (cambia colores, persianas y detalles)"',
         '    lod integer = 0 range 0..1 label "Versión (0 completa, 1 ligera, de lejos)"',
         '    era integer = 0 range 0..1 label "Época (0 la tradicional del lugar, 1 de hoy)"',
         '    state integer = 0 range 0..2 label "Estado (0 en uso, 1 abandonado, 2 en obra)"','  section "Dimensiones"']
    out+=dims
    out.append('  section "Materiales"')
    for role,mat in mats:
        out.append(f'    {role} material = "{mat}" label "{mat}"')
    w = 'width' if 'width length = 8' in "\n".join(dims) or ('width length' in "\n".join(dims) and gen != 'fine_world_facility') else None
    box = f'  box (-(width) / 2, 0, 0) to ((width) / 2, depth, {h}) material w_white' if w else f'  box (-10, 0, 0) to (10, 18, {h}) material w_white'
    poly = '  polygon (-(width) / 2, 0) ((width) / 2, 0) ((width) / 2, depth) (-(width) / 2, depth) pen view' if w else '  polygon (-10, 0) (10, 0) (10, 18) (-10, 18) pen view'
    out+=['','body',box,'','plan 1:50',poly,'','outputs',f'  generator = "props:{gen}"','  ifc_class = "IfcBuildingElementProxy"','  ifc_type = "USERDEFINED"','']
    return "\n".join(out)
R=[]
R.append(("mundo_entre_medianeras",recipe("Casa entre medianeras del mundo (taller)","Casa de una calle entre medianeras vestida con la piel de un lugar: sus colores y piedras, sus ventanas y lo que las cierra, sus balcones, cómo es la planta baja, la cornisa y la cubierta (modelo del taller, en revisión)","fine_world_townhouse",
 ['    width length = 8.00 range 4.00..30.00 label "Fachada"','    depth length = 12.00 range 7.00..25.00 label "Fondo"','    floors integer = 3 range 1..10 label "Plantas (con la baja)"'],"3.6 + 3.0 * (floors - 1) + 1.5")))
R.append(("mundo_bloque",recipe("Bloque de viviendas del mundo (taller)","Bloque de viviendas exento vestido con la piel de un lugar: balcones, galerías, escaleras de incendios, jaulas o terrazas acristaladas según el sitio, bajos comerciales o soportales y su cubierta (modelo del taller, en revisión)","fine_world_block",
 ['    width length = 22.00 range 10.00..60.00 label "Fachada"','    depth length = 14.00 range 9.00..24.00 label "Fondo"','    floors integer = 6 range 1..16 label "Plantas (con la baja)"'],"3.6 + 3.0 * (floors - 1) + 2")))
R.append(("mundo_aislada",recipe("Vivienda aislada del mundo (taller)","Vivienda aislada en su parcela vestida con la piel de un lugar: muros, carpinterías, porche o galería, cubierta, valla o tapia, césped, camino y árbol (modelo del taller, en revisión)","fine_world_house",
 ['    width length = 10.00 range 6.00..20.00 label "Fachada"','    depth length = 8.00 range 6.00..16.00 label "Fondo"','    floors integer = 2 range 1..3 label "Plantas"'],"3.0 + 2.9 * (floors - 1) + 3")))
R.append(("mundo_torre",recipe("Torre del mundo (taller)","Torre de un lugar: viviendas apiladas con sus balcones, muro cortina de vidrio con su vestíbulo y la pantalla de sus máquinas, o el ladrillo y la piedra de Nueva York que se retranquean hasta su corona y su aguja (modelo del taller, en revisión)","fine_world_tower",
 ['    width length = 24.00 range 12.00..60.00 label "Fachada"','    depth length = 20.00 range 12.00..60.00 label "Fondo"','    floors integer = 20 range 1..60 label "Plantas (con la baja)"'],"5 + 3.6 * (floors - 1) + 3")))
R.append(("mundo_patio",recipe("Casa patio del mundo (taller)","Casa alrededor de un patio con la piel de un lugar (casa patio, riad, siheyuan, casona colonial): una crujía por lado y el patio empedrado con su pozo, sus macetas y su naranjo (modelo del taller, en revisión)","fine_world_courtyard",
 ['    width length = 16.00 range 10.00..40.00 label "Fachada"','    depth length = 18.00 range 10.00..40.00 label "Fondo"','    floors integer = 1 range 1..3 label "Plantas"'],"3.0 + 2.9 * (floors - 1) + 3")))
R.append(("mundo_hilera",recipe("Casas en hilera del mundo (taller)","Fila de casas de calle con la piel de un lugar, cada una de su color, con su jardín delantero, su camino y la valla, el seto o la verja de su sitio (modelo del taller, en revisión)","fine_world_row",
 ['    width length = 24.00 range 8.00..80.00 label "Largo de la fila"','    depth length = 10.00 range 7.00..20.00 label "Fondo"','    floors integer = 3 range 1..5 label "Plantas (con la baja)"'],"3.4 + 3.0 * (floors - 1) + 3")))
R.append(("mundo_muestra",recipe("Muestra de planta del mundo (taller)","Las tipologías del mundo sobre plantas que no son rectángulos (una L en esquina, una esquina achaflanada, un solar en ángulo), completas o en su versión ligera para verlas de lejos (modelo del taller, en revisión)","fine_world_sample",
 ['    plan integer = 0 range 0..2 label "Planta (0 L en esquina, 1 esquina achaflanada, 2 solar en ángulo)"','    typology integer = 0 range 0..4 label "Tipología (0 entre medianeras, 1 bloque, 2 aislada, 3 torre, 4 casa patio)"','    floors integer = 0 range 0..60 label "Plantas (0: las habituales del lugar)"'],"20")))
R.append(("mundo_equipamiento",recipe("Equipamiento del mundo (taller)","Equipamientos y comercio de cada lugar con su piel: ayuntamiento, colegio, biblioteca, bomberos, comisaría, polideportivo, gimnasio, estadio, fábrica, quiosco, tienda, mercadillo, galería comercial, centro comercial, chiringuito, restaurante, taberna, la plaza y el edificio emblemático del lugar; parques, paseos, alamedas y jardines; cementerio, ermita, faro, lonja, estaciones, hospital, hotel, puente y casa cuartel; hórreo, palloza, casa de piedra, casa-patio y VPO (modelo del taller, en revisión)","fine_world_facility",
 ['    programme integer = 0 range 0..37 label "Programa (0 Ayuntamiento, 1 Colegio, 2 Biblioteca, 3 Parque de bomberos, 4 Comisaría, 5 Polideportivo, 6 Gimnasio, 7 Estadio, 8 Fábrica, 9 Quiosco, 10 Tienda, 11 Mercadillo, 12 Galería comercial, 13 Centro comercial, 14 Chiringuito, 15 Restaurante, 16 Taberna, 17 Plaza del lugar, 18 Edificio emblemático del lugar, 19 Parque, 20 Paseo marítimo, 21 Alameda, 22 Jardín, 23 Cementerio, 24 Ermita, 25 Faro, 26 Lonja y puerto, 27 Estación de tren, 28 Estación de autobuses, 29 Hospital, 30 Hotel, 31 Puente de piedra, 32 Casa cuartel, 33 Hórreo, 34 Palloza, 35 Casa de piedra de sierra, 36 Casa-patio de una planta, 37 Bloque de VPO)"','    width length = 0.00 range 0.00..200.00 label "Ancho (0: el habitual)"','    depth length = 0.00 range 0.00..200.00 label "Fondo (0: el habitual)"','    floors integer = 0 range 0..12 label "Plantas (0: las habituales)"'],"12")))
for n,t in R:
    open(f'crates/archisim-library/taller/edificio_{n}.asfam','w').write(t)
print(len(mats), "materials")
