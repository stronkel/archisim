//! `archisim` command line.
//!
//! ```text
//! archisim demo [carpeta]                    vivienda de prueba: .archisim, IFC y plantas
//! archisim exportar <proyecto.archisim> [carpeta]   IFC y plantas de un proyecto
//! archisim importar <fichero.ifc> [proyecto.archisim]   proyecto a partir de un IFC
//! archisim cartografia <ficheros…> [--proyecto p.archisim] [--radio m] [--centrar] [--salida s.archisim] [--lugar "dirección"] [--resultado n] [--en-el-origen] [--enlaces]   entorno del Catastro, OSM y MDT
//! archisim medir [n] [carpeta]             rendimiento con n × n viviendas
//! archisim png <dibujo.svg> [imagen.png] [ppp]   imagen de un SVG
//! archisim comprobar-ids | comparar | interferencias | bcf   colaboración (fase 4, `colaboracion`)
//! archisim --version
//! ```

mod armado;
mod cajas;
mod cajas_sobre;
mod calles;
mod cartografia;
mod catalogo;
mod cocina;
mod colaboracion;
mod concepto_malla;
mod cubiertas;
mod demo;
mod diseno;
mod edificio;
mod entorno;
mod entramado;
mod equipo;
mod escaleras;
mod fachadas;
mod figuras;
mod formas;
mod gama;
mod grande;
mod instalaciones;
mod locales;
mod modelos;
mod muestrario;
mod muros_talud;
mod ortofoto;
mod plantilla;
mod taller;
mod tierras;
mod tramas;
mod trazado;
mod vivero;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use archisim_core::Model;
use archisim_ifc::{export_ifc, import_ifc, ExportOptions};
use archisim_plan::{render_plan, PlanOptions};

const HELP: &str = "\
ArchiSim — herramienta BIM (versión de desarrollo)

Uso:
  archisim demo [carpeta]
      Genera una vivienda de prueba, la guarda como proyecto demo.archisim
      y la exporta a demo.ifc (IFC4) y a una planta 1:50 por nivel:
      demo.svg (planta baja), demo-cubierta.svg…, y otra 1:100
      (demo-1a100.svg…), con los símbolos de puertas y ventanas de esa
      escala, y cada plano con su cajetín (demo-plano-a-01.svg).
      Carpeta por defecto: ./out

  archisim exportar <proyecto.archisim> [carpeta]
      Abre un proyecto y lo exporta a IFC4 y a plantas 1:50 y 1:100 con el
      mismo nombre (casa.archisim → casa.ifc, casa.svg, casa-1a100.svg…) y sus
      planos (casa-plano-a-01.svg…). Por defecto, en la
      misma carpeta del proyecto.

  archisim importar <fichero.ifc> [proyecto.archisim]
      Lee un IFC (IFC2X3 o IFC4, de ArchiCAD, Revit u otro programa) y lo
      guarda como proyecto de ArchiSim: plantas, muros con sus capas,
      forjados, puertas y ventanas, con sus GlobalId. Explica qué ha
      quedado fuera. Por defecto, casa.ifc → casa.archisim en la misma
      carpeta.

  archisim cartografia <ficheros…> [--proyecto p.archisim] [--radio 150] [--centrar] [--salida s.archisim]
  archisim cartografia --lugar «Calle Ancha 12, Punta Umbría» [--resultado 1] [--en-el-origen] [--enlaces] [ficheros…]
      Trae el entorno de ficheros de cartografía gratuita: edificios por
      plantas y parcelas del Catastro (GML INSPIRE), calles y árboles de
      OpenStreetMap (.osm) y el terreno del CNIG (MDT en .asc), dentro de
      un radio alrededor del origen del proyecto. Sin proyecto, empieza
      uno nuevo georreferenciado por los ficheros; con --centrar, el
      origen del proyecto se lleva al centro de lo que traen los ficheros.
      Con --lugar se busca el sitio por su dirección, un municipio, una
      referencia catastral o unas coordenadas (CartoCiudad, OpenStreetMap,
      Catastro; hace falta red): el proyecto sin emplazamiento lo toma con
      su origen ahí, y el que lo tiene trae alrededor del lugar (o de su
      origen, con --en-el-origen). --resultado elige otro de los
      encontrados; con --enlaces, las direcciones de la descarga.

  archisim calles [carpeta]
      Unas calles en una ladera que se encuentran de todas las maneras:
      un cruce, tres en T y una esquina en L, con las esquinas redondeadas
      y las calles que suben siguiendo el terreno (rasante): calles.archisim,
      calles.ifc (con los ejes como IfcAlignment), planta y axonometría.

  archisim entorno [carpeta]
      Una calle con los trece estilos de edificios del entorno (U6):
      entorno.archisim, entorno.ifc y su planta.
  archisim tipos-calle [carpeta]
      Todos los tipos de calle, uno al lado de otro y por grupos:
      tipos-calle.archisim, tipos-calle.ifc, planta y axonometría.

  archisim biblioteca [carpeta] [proyecto.archisim]
      Escribe los tipos de puerta, ventana y objeto de la plantilla (o de
      un proyecto) como ficheros de biblioteca: una familia por fichero
      .asfam con su catálogo de tipos, en subcarpetas por categoría
      (Puertas, Ventanas, Mobiliario…). Es lo que lee la ventana
      Biblioteca de la aplicación. Carpeta por defecto: ./out/biblioteca

  archisim biblioteca --ampliada [carpeta]
      Escribe la biblioteca ampliada (más de ochenta árboles, arbustos,
      mobiliario urbano, personas y vehículos) en la carpeta, por defecto
      ./biblioteca-ampliada. Se copia dentro de la carpeta de la
      biblioteca de la aplicación.

  archisim vivero [carpeta]
      Todas las especies de vegetación que ArchiSim genera, plantadas por
      grupos, con un paseo y un arriate: vivero.archisim, vivero.ifc y
      plantas 1:200 de presentación y técnica. Carpeta por defecto: ./out
  archisim modelos [carpeta]
      Todos los muebles, sanitarios, cocina, personas y coches de la
      biblioteca con su 3D hecho por ArchiSim, en filas: modelos.archisim,
      modelos.ifc, axonometría y planta en SVG. Por defecto: ./out
  archisim concepto-malla <fichero> [carpeta] [--edificios] [--unidades m|cm|mm|in]
      Trae un modelo de volúmenes (DAE de SketchUp, OBJ, STL…, también
      dentro de un .zip) como volúmenes de Concepto, como la aplicación:
      cada pieza cerrada es un volumen; las que son un contorno extruido,
      prismas normales; las demás, mallas con sus caras planas. Escribe
      concepto-malla.archisim, .ifc, la planta en SVG y el informe (qué
      es cada pieza, si está cerrada, sus caras). Con --edificios pasa los
      prismas a edificio con plantas de 3 m. Por defecto: ./out
  archisim diseno <modelo> [carpeta] [Fabricante]
      Importa el modelo 3D de un fabricante (FBX, OBJ, glTF, COLLADA, 3DS,
      STL o el .zip tal como se descarga) como pieza de diseño, igual que
      la aplicación: dice qué ha leído (unidades, caras antes y después de
      aligerarlo, materiales y su acabado) y escribe diseno.archisim,
      diseno.ifc y una planta 1:20 en SVG. Por defecto: ./out
  archisim edificios [carpeta]
      El taller de edificios: cada edificio nuevo del entorno, en revisión,
      frente al edificio del mismo tipo que hace hoy ArchiSim:
      edificios.archisim y edificios.ifc.
  archisim equipamientos [carpeta] [lugares…]
      Los equipamientos y el comercio de cada lugar (ayuntamiento, colegio,
      biblioteca, bomberos, comisaría, polideportivo, gimnasio, estadio,
      fábrica, quiosco, tienda, mercadillo, galería, centro comercial,
      chiringuito, restaurante, taberna, plaza y su edificio emblemático):
      equipamientos-<lugar>.archisim y .ifc por lugar.
  archisim mundo [carpeta] [lugares…]
      Los edificios del mundo: por cada lugar (andalucia, paris, japon…),
      tres casas de su calle, su bloque y su vivienda aislada, en una fila:
      mundo.archisim y mundo.ifc.
  archisim taller [carpeta]
      El taller de modelos: cada mueble nuevo, en revisión, delante del
      de la biblioteca al que sustituiría: taller.archisim, taller.ifc,
      axonometría y planta en SVG. Por defecto: ./out
  archisim tramas [carpeta]
      Las tramas de presentación (madera, metal, hormigón y piedra), cada
      una en un cuadro con su nombre, a 1:20 y a 1:50: tramas.archisim,
      SVG, PDF y DXF. Por defecto: ./out
  archisim figuras [carpeta]
      Árboles, palmeras, arbustos, personas y coches en fila delante de
      un muro, vistos desde el sur como los dibuja un arquitecto: alzado
      1:50 técnico y en color y axonometría en SVG. Por defecto: ./out
  archisim cocina [carpeta]
      Una cocina por módulos: columna, cajoneras, fregadero, lavavajillas,
      horno, placa y botellero con su encimera continua, muebles altos y
      campana, en cuatro filas (frentes liso, con marco, ranurado y con
      cristal): cocina.archisim, cocina.ifc, planta y axonometría en SVG.
  archisim escaleras [carpeta]
      Todas las escaleras: recta, en L y en U con meseta o compensadas, de
      caracol con mástil y helicoidal de losa, de zancas, de zanca central
      y de peldaños volados, con y sin tabica: escaleras.archisim,
      escaleras.ifc, plantas y axonometría en SVG. Por defecto: ./out

  archisim cubiertas [carpeta]
      Lo que llevan las cubiertas: buhardillas a dos aguas, a un agua y
      planas, ventanas de cubierta, tejas de cada clase, caballetes,
      canalones y bajantes: cubiertas.archisim, cubiertas.ifc, planta de
      cubiertas y axonometría en SVG. Por defecto: ./out
  archisim cajas [carpeta]
      Una vivienda de cajas modulares en L (ocho cajas de 6,20 × 3,60 m
      con su envolvente): cajas.archisim, cajas.ifc (cada caja, un
      IfcElementAssembly con sus piezas), cajas-planta.svg y cajas.txt
      (lo que lleva cada caja y sus avisos, su peso e izado, el plan de
      grúa y el despiece) y los planos de taller (cajas-taller.pdf).
      Por defecto: ./out
  archisim cajas-sobre <proyecto> <salida.archisim> [--modulo 6.10x3.60] [--tipos <carpeta> [--prefijo L160]]
      Las cajas de una vivienda modular dibujada en otro programa e
      importada: una caja sobre cada suelo del tamaño de una caja (llevado
      al módulo si cabe), con sus lados leídos de los muros de las juntas
      (del muro, abierto o libre). Con --tipos, una caja tipo por caja en
      esa carpeta (<prefijo>-01, -02…), en diseño.
  archisim locales <proyecto> <locales.csv> <salida.archisim>
      Los locales de un proyecto desde una lista (nombre; uso; x; y, en
      metros): cada uno se cierra con los muros alrededor de su punto,
      como la herramienta Local. Para una vivienda importada sin locales.
  archisim taller-cajas <proyecto> [carpeta]
      Los planos de taller de las cajas modulares de un proyecto, A3:
      montaje y plan de grúa (M-00) y una hoja por caja con su chasis en
      planta y alzados, despiece, materiales, peso e izado
      (<proyecto>-taller.pdf y una SVG por hoja). Por defecto: ./out
  archisim plantilla <proyecto> <plantilla.archisim> [--perfiles <catálogo.csv>] [--entramado]
      La plantilla de un proyecto: sus plantas, materiales, tipos,
      plantillas de vista, cajetines, planos, tablas y datos de cálculo,
      sin nada dibujado. Con un catálogo de perfiles (nombre; material;
      canto; ala; labio; espesor, en mm), los tipos con entramado toman
      sus montantes y canales por el grueso de la capa que llenan; con
      --entramado, también los muros con aislamiento entre placas.
  archisim catalogo [carpeta]
      El catálogo del sistema: tres cajas tipo aprobadas en «Cajas tipo/»
      (dormitorio doble con ventana a elegir, baño + cocina, estar
      abierta) y una vivienda con cuatro copias pegadas una a otra:
      catalogo.archisim, catalogo.ifc, catalogo-planta.svg y
      catalogo.txt. Por defecto: ./out
  archisim entramado [carpeta]
      Entramados ligeros: dos casas, una de steel frame y otra de madera,
      con muros, forjado y cubierta de entramado: entramado.archisim,
      entramado.ifc, el despiece (lista de corte y paneles) en SVG y PNG,
      entramado.xlsx para Excel y entramado.bc3. Por defecto: ./out
  archisim armado [carpeta]
      Hormigón armado: un pórtico de cuatro pilares sobre zapatas, vigas,
      una zapata corrida y un forjado con hueco, todos con su armado:
      armado.archisim, armado.ifc (con las barras) y la planta a 1:20 con
      las barras de los pilares. Por defecto: ./out
  archisim edificio [carpeta]
      Un edificio de viviendas entero (el de Florence Street del máster
      BIM): cinco plantas sobre un zócalo de ladrillo, 24 viviendas,
      núcleo de hormigón, helechera, bicicletas y cubierta con jardín y
      pabellón: edificio.archisim, edificio.ifc, plantas, planos y
      comprobación CTE. Por defecto: ./out
  archisim instalaciones [carpeta]
      Instalaciones, segunda vuelta: una casa de dos plantas con los
      mecanismos eléctricos (luces, interruptores, conmutadores, enchufes,
      base de 25 A, tomas de TV y datos, cuadro) y sus circuitos del REBT,
      tuberías con codos de su radio, un montante y una bajante entre las
      dos plantas: instalaciones.archisim, instalaciones.ifc y las plantas
      en SVG. Por defecto: ./out
  archisim fachadas [carpeta]
      Las fachadas pieza a pieza: un bloque de dos plantas por cada tipo de
      fachada de la plantilla (ladrillo visto a soga, inglés, flamenco y con
      rombos, bloque, paneles, ventilada y aplacado), con dinteles a
      sardinel, esquinas trabadas y frentes de forjado: fachadas.archisim y
      fachadas.ifc, y cuántas piezas lleva. Por defecto: ./out
  archisim muros-talud [carpeta]
      Muros de sección variable: un muro de contención de hormigón en talud
      1:10 con su zarpa (talón y puntera), en L con otro igual y con un muro
      de ladrillo en T, y un muro de mampostería en talud por las dos caras
      con una puerta y una ventana, en L con uno de ladrillo:
      muros-talud.archisim, muros-talud.ifc, las plantas y dos secciones a
      1:50 en SVG y la comprobación del muro de contención. Por defecto: ./out
  archisim tierras [carpeta]
      Movimiento de tierras en una ladera al 12 %: la plataforma de una
      vivienda y el vaciado de su garaje (desde sus forjados), una terraza
      dibujada a mano, la rampa de acceso al garaje, la plataforma de la
      piscina y la piscina: tierras.archisim, tierras.ifc (IFC 4.3, con
      IfcEarthworksCut e IfcEarthworksFill), tierras-ifc4.ifc, la planta a
      1:200 con el desmonte y el terraplén coloreados y el informe de
      tierras. Por defecto: ./out
  archisim gama [carpeta]
      La gama estilizada de materiales: un muestrario con cada uno (fachadas,
      cubiertas, suelos, pavimentos y pinturas) y una calle con acantilado,
      mar y piscina. Escribe gama.archisim. Por defecto: ./out
  archisim formas [carpeta]
      Las formas de Concepto juntas: aristas redondeadas y achaflanadas,
      esfera, cilindro, cono, revolución, uniones, restas, intersecciones,
      aristas donde se cruzan las caras y materiales de visualización:
      formas.archisim, planta y axonometría en SVG. Por defecto: ./out
  archisim equipo <carpeta> [recibido.archisim]
      Un proyecto de equipo (carpeta compartida): quién está, qué ha
      publicado y reservado cada uno; con un segundo argumento, guarda el
      proyecto tal como lo recibe cualquiera.
  archisim equipo prueba [carpeta]
      Comparte la casa de la demo entre dos personas (Antonio y Ana), que
      reservan, dibujan y publican: carpeta equipo/ para abrirla con la
      aplicación, y equipo-recibido.archisim e .ifc. Por defecto: ./out
  archisim muestrario [carpeta]
      Un muro con todas las puertas y ventanas de la plantilla, para verlas
      en planta, en 3D y en IFC: muestrario.archisim, muestrario.ifc y
      muestrario.svg. Carpeta por defecto: ./out

  archisim cte <proyecto.archisim> [informe.csv]
  archisim fontaneria <proyecto.archisim> [--diametros salida.archisim] [--esquemas carpeta]   cálculo del suministro de agua (DB-HS 4); con --diametros, guarda el proyecto con los diámetros calculados; con --esquemas, el esquema vertical y la distribución por planta en PDF y DXF
  archisim climatizacion <proyecto.archisim>   envolvente por el DB-HE 1 (transmitancias, terreno por la UNE-EN ISO 13370, puentes térmicos, K, control solar, permeabilidad, n50 y condensaciones) y cargas de calefacción y refrigeración de cada local, con sus equipos
  archisim renovables <proyecto.archisim>     energías renovables: agua caliente por el DB-HE 4 (demanda, agua fría de la provincia y parte renovable) y fotovoltaica mínima por el HE 5
  archisim ventilacion <proyecto.archisim> [--tamanos salida.archisim] [--esquemas carpeta]   ventilación de las viviendas (DB-HS 3), de los garajes y de los locales del RITE: caudales, aireadores, bocas, pasos, conductos y campanas; con --tamanos, agranda lo que se queda corto; con --esquemas, el esquema vertical y la red de cada planta en PDF y DXF
  archisim memoria-instalaciones <proyecto.archisim> [carpeta]   la memoria de instalaciones (REBT, DB-HS 4, DB-HS 5, DB-HS 3 y DB-HE 1) con las tablas del cálculo, en PDF A4 y en texto
  archisim ventilacion --ejemplo <carpeta>   un piso de ejemplo con su ventilación y la campana con su conducto (ventilacion-ejemplo.archisim)
  archisim ventilacion --ejemplo-terciario <carpeta>   una oficina (RITE: IDA, aire exterior, conductos por velocidad) sobre un garaje de diez plazas con ventilación natural (ventilacion-oficina.archisim)
  archisim saneamiento <proyecto.archisim> [--diametros salida.archisim] [--esquemas carpeta]   evacuación de aguas (DB-HS 5): residuales por unidades de desagüe y pluviales por cubierta; con --diametros, agranda lo que se queda corto; con --esquemas, el esquema vertical y la red de cada planta en PDF y DXF
  archisim electricidad <proyecto.archisim> [carpeta]   cálculo de la instalación eléctrica (REBT); con carpeta, el esquema unifilar en PDF y DXF
  archisim estructura <proyecto.archisim> [carpeta]   acciones, descenso de cargas y cálculo de la estructura; con carpeta, el anejo de cálculo y los planos de estructura en PDF
      Comprueba el proyecto con el Código Técnico: ocupación y evacuación
      (DB-SI 3), accesibilidad (DB-SUA 9), escaleras y rampas (DB-SUA 1) y
      superficies, según el uso del edificio guardado en el proyecto.
      Escribe el informe en pantalla y, si se da, en CSV.

  archisim medir [n] [carpeta]
      Repite la vivienda de prueba en una rejilla de n × n (20 por defecto:
      400 viviendas, 2800 muros) y mide cuánto tarda cada trabajo: planta,
      3D, varita mágica, guardar y abrir, exportar e importar el IFC. Deja
      grande.archisim y grande.ifc en la carpeta (./out por defecto).
  archisim medir vistas proyecto.archisim   lo que tarda cada alzado y planta en pantalla
  archisim medir proyecto proyecto.archisim  abrir, 3D, IFC, CTE, mediciones y vistas de un proyecto

  archisim vida-calle <proyecto.archisim> [salida.archisim]
      Viste las calles del proyecto: árboles en las aceras, farolas,
      bancos, papeleras, coches aparcados, alguna persona y los jardines
      delanteros de las viviendas con su seto, su cubo y un arbusto.

  archisim png <dibujo.svg> [imagen.png] [ppp]
      Convierte una planta o un plano en SVG en una imagen PNG, a 200
      puntos por pulgada si no se dice otra cosa.

  archisim render <proyecto.archisim> [imagen.png] [ancho] [--atardecer | --noche] [--sin-texturas]
      Hace una imagen realista del proyecto visto desde el suroeste, con el
      sol del proyecto, sus sombras, la luz del cielo y las texturas de los
      materiales (1600 píxeles de ancho si no se dice otra cosa). Al
      atardecer y de noche se encienden las lámparas, con sus sombras.

  archisim glb <proyecto.archisim> [modelo.glb]
      El proyecto entero en glTF (.glb) para D5, Twinmotion o Blender: un
      objeto por elemento, los materiales con su nombre, color y textura.

  archisim --version        Muestra la versión
  archisim --help           Muestra esta ayuda
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("demo") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(run_demo(&dir))
        }
        Some("png") => match args.get(1) {
            Some(svg) => {
                let svg = PathBuf::from(svg);
                let png = args
                    .get(2)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| svg.with_extension("png"));
                let dpi: f32 = args.get(3).and_then(|d| d.parse().ok()).unwrap_or(200.0);
                report(svg_to_png(&svg, &png, dpi))
            }
            None => {
                eprintln!("Falta el dibujo: archisim png <dibujo.svg> [imagen.png] [ppp]");
                ExitCode::FAILURE
            }
        },
        Some("render") => {
            // Flags anywhere: the light and no textures.
            let light = if args.iter().any(|a| a == "--noche") {
                archisim_render::Light::Night
            } else if args.iter().any(|a| a == "--atardecer") {
                archisim_render::Light::Evening
            } else {
                archisim_render::Light::Day
            };
            let textures = !args.iter().any(|a| a == "--sin-texturas");
            // `--camara=x,y,z,tx,ty,tz`: from where and to where it looks.
            let camera: Option<[f64; 6]> = args
                .iter()
                .find_map(|a| a.strip_prefix("--camara="))
                .and_then(|v| {
                    let n: Vec<f64> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    n.try_into().ok()
                });
            let plain: Vec<&String> = args
                .iter()
                .skip(1)
                .filter(|a| !a.starts_with("--"))
                .collect();
            match plain.first() {
                Some(project) => {
                    let project = PathBuf::from(project);
                    let png = plain
                        .get(1)
                        .map(PathBuf::from)
                        .unwrap_or_else(|| project.with_extension("png"));
                    let width: usize = plain.get(2).and_then(|w| w.parse().ok()).unwrap_or(1600);
                    report(run_render(&project, &png, width, light, textures, camera))
                }
                None => {
                    eprintln!("Falta el proyecto: archisim render <proyecto.archisim> [imagen.png] [ancho] [--atardecer | --noche] [--sin-texturas] [--camara=x,y,z,tx,ty,tz]");
                    ExitCode::FAILURE
                }
            }
        }
        Some("glb") => match args.get(1) {
            Some(project) => {
                let project = PathBuf::from(project);
                let out = args
                    .get(2)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| project.with_extension("glb"));
                report(run_glb(&project, &out))
            }
            None => {
                eprintln!("Falta el proyecto: archisim glb <proyecto.archisim> [modelo.glb]");
                ExitCode::FAILURE
            }
        },
        Some("secciones") => match args.get(1) {
            Some(project) => {
                let project = PathBuf::from(project);
                let dir = match args.get(2) {
                    Some(d) => PathBuf::from(d),
                    None => project.parent().map(Path::to_path_buf).unwrap_or_default(),
                };
                report(run_sections(&project, &dir))
            }
            None => {
                eprintln!("uso: archisim secciones <proyecto.archisim> [carpeta]");
                ExitCode::FAILURE
            }
        },
        Some("exportar") | Some("export") => match args.get(1) {
            Some(project) => {
                let project = PathBuf::from(project);
                let dir = match args.get(2) {
                    Some(d) => PathBuf::from(d),
                    None => project.parent().map(Path::to_path_buf).unwrap_or_default(),
                };
                report(run_export(&project, &dir))
            }
            None => {
                eprintln!("Falta el proyecto: archisim exportar <proyecto.archisim> [carpeta]");
                ExitCode::FAILURE
            }
        },
        Some("cartografia") | Some("cartografía") => {
            let mut files = Vec::new();
            let (mut project, mut radius, mut out, mut recentre) = (None, 150.0, None, false);
            let mut links = false;
            let (mut lugar, mut pick, mut keep_origin) = (None, 1usize, false);
            let mut it = args[1..].iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--proyecto" => project = it.next().map(PathBuf::from),
                    "--radio" => {
                        radius = it
                            .next()
                            .and_then(|v| v.replace(',', ".").parse().ok())
                            .unwrap_or(radius)
                    }
                    "--salida" => out = it.next().map(PathBuf::from),
                    "--centrar" => recentre = true,
                    "--enlaces" => links = true,
                    "--lugar" => lugar = it.next().cloned(),
                    "--resultado" => pick = it.next().and_then(|v| v.parse().ok()).unwrap_or(1),
                    "--en-el-origen" => keep_origin = true,
                    _ => files.push(PathBuf::from(a)),
                }
            }
            if let Some(text) = &lugar {
                // The place by its address, cadastral reference or
                // coordinates.
                let mut m = match cartografia::open(project.as_deref()) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{e}");
                        return ExitCode::FAILURE;
                    }
                };
                let mut opts = archisim_library::carto::CartoOptions {
                    radius,
                    recentre,
                    ..Default::default()
                };
                println!("Buscando «{text}»…");
                if let Err(e) = cartografia::place(
                    &mut m,
                    text,
                    pick,
                    keep_origin,
                    &mut opts,
                    &mut cartografia::http_get,
                ) {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
                if links {
                    match archisim_library::download::plan(&m, &opts) {
                        Ok(rs) => {
                            for r in rs {
                                println!("{}\n  {}", r.source.name(), r.url);
                            }
                        }
                        Err(e) => {
                            eprintln!("{e}");
                            return ExitCode::FAILURE;
                        }
                    }
                }
                if links && files.is_empty() && out.is_none() {
                    return ExitCode::SUCCESS;
                }
                let out = out
                    .or_else(|| project.clone())
                    .unwrap_or_else(|| PathBuf::from("out/entorno.archisim"));
                let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
                if refs.is_empty() {
                    // Only placed: saved with its georeference.
                    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    return report(
                        archisim_file::save(&m, &out)
                            .map(|_| println!("  Proyecto: {}", out.display()))
                            .map_err(|e| e.into()),
                    );
                }
                return report(cartografia::run_with(m, &refs, &opts, &out));
            }
            if links {
                // The addresses «Descargar el entorno» asks, to try them in
                // a browser.
                let Some(p) = &project else {
                    eprintln!(
                        "Uso: archisim cartografia --enlaces --proyecto p.archisim [--radio 150]"
                    );
                    return ExitCode::FAILURE;
                };
                let m = match archisim_file::open(p) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{e}");
                        return ExitCode::FAILURE;
                    }
                };
                let opts = archisim_library::carto::CartoOptions {
                    radius,
                    ..Default::default()
                };
                match archisim_library::download::plan(&m, &opts) {
                    Ok(rs) => {
                        for r in rs {
                            println!("{}\n  {}", r.source.name(), r.url);
                        }
                        return ExitCode::SUCCESS;
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            if files.is_empty() {
                eprintln!("Uso: archisim cartografia <ficheros .gml .osm .asc…> [--proyecto p.archisim] [--radio 150] [--centrar] [--salida entorno.archisim]  (o --enlaces --proyecto p.archisim: las direcciones de «Descargar el entorno»; con --lugar \"Calle Ancha 12, Punta Umbría\", una referencia catastral o coordenadas, alrededor de ese sitio)");
                return ExitCode::FAILURE;
            }
            let out = out
                .or_else(|| project.clone())
                .unwrap_or_else(|| PathBuf::from("out/entorno.archisim"));
            let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
            report(cartografia::run(
                project.as_deref(),
                &refs,
                radius,
                recentre,
                &out,
            ))
        }
        Some("importar") | Some("import") => match args.get(1) {
            Some(ifc) => {
                let ifc = PathBuf::from(ifc);
                let project = match args.get(2) {
                    Some(p) => PathBuf::from(p),
                    None => ifc.with_extension(archisim_file::EXTENSION),
                };
                report(run_import(&ifc, &project))
            }
            None => {
                eprintln!("Falta el fichero: archisim importar <fichero.ifc> [proyecto.archisim]");
                ExitCode::FAILURE
            }
        },
        Some("equipo") => match (args.get(1).map(String::as_str), args.get(2)) {
            (Some("prueba"), dir) => {
                let dir = PathBuf::from(dir.map(String::as_str).unwrap_or("out"));
                report(equipo::sample(&dir))
            }
            (Some(root), None) => report(equipo::status(Path::new(root))),
            (Some(root), Some(out)) => report(equipo::export(Path::new(root), Path::new(out))),
            (None, _) => {
                eprintln!("Uso: archisim equipo <carpeta> [recibido.archisim] · archisim equipo prueba [carpeta]");
                ExitCode::FAILURE
            }
        },
        Some("modelos") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(modelos::write(&dir))
        }
        Some("concepto-malla") => {
            let flags: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
            let unit = flags
                .iter()
                .position(|a| *a == "--unidades")
                .and_then(|i| flags.get(i + 1))
                .and_then(|u| match *u {
                    "m" => Some(1.0),
                    "cm" => Some(0.01),
                    "mm" => Some(0.001),
                    "in" | "pulgadas" => Some(0.0254),
                    _ => None,
                });
            let mut plain: Vec<&str> = Vec::new();
            let mut skip = false;
            for a in &flags {
                if skip {
                    skip = false;
                } else if *a == "--unidades" {
                    skip = true;
                } else if !a.starts_with("--") {
                    plain.push(a);
                }
            }
            match plain.first() {
                Some(model) => {
                    let dir = PathBuf::from(plain.get(1).copied().unwrap_or("out"));
                    report(concepto_malla::write(
                        Path::new(model),
                        &dir,
                        flags.contains(&"--edificios"),
                        unit,
                    ))
                }
                None => {
                    eprintln!("Uso: archisim concepto-malla <fichero> [carpeta] [--edificios] [--unidades m|cm|mm|in]");
                    ExitCode::FAILURE
                }
            }
        }
        Some("diseno") | Some("diseño") => {
            match args.get(1) {
                Some(model) => {
                    let dir = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("out"));
                    report(diseno::write(
                        Path::new(model),
                        &dir,
                        args.get(3).map(String::as_str),
                    ))
                }
                None => {
                    eprintln!("Uso: archisim diseno <modelo o .zip del fabricante> [carpeta] [Fabricante]");
                    ExitCode::FAILURE
                }
            }
        }
        Some("taller") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(taller::write(&dir))
        }
        Some("edificios") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(taller::write_buildings(&dir))
        }
        Some("equipamientos") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            let only: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();
            report(taller::write_facilities(&dir, &only))
        }
        Some("mundo") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            let only: Vec<&str> = args.iter().skip(2).map(String::as_str).collect();
            report(taller::write_world(&dir, &only))
        }
        Some("tramas") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(tramas::write(&dir))
        }
        Some("figuras") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(figuras::write(&dir))
        }
        Some("cocina") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(cocina::write(&dir))
        }
        Some("catalogo") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(catalogo::write(&dir))
        }
        Some("cajas") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(cajas::write(&dir))
        }
        Some("cajas-sobre") => {
            let flag = |name: &str| {
                args.iter()
                    .position(|a| a == name)
                    .and_then(|i| args.get(i + 1))
            };
            let module = flag("--modulo")
                .and_then(|m| {
                    let (a, b) = m
                        .replace(',', ".")
                        .split_once(['x', '×'])
                        .map(|(a, b)| (a.to_string(), b.to_string()))?;
                    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
                })
                .unwrap_or((6.10, 3.60));
            let types = flag("--tipos").map(PathBuf::from);
            let prefix = flag("--prefijo").cloned().unwrap_or_else(|| "CB".into());
            match (args.get(1), args.get(2)) {
                (Some(input), Some(output))
                    if !input.starts_with("--") && !output.starts_with("--") =>
                {
                    report((|| {
                        let text = cajas_sobre::run(
                            Path::new(input),
                            Path::new(output),
                            module,
                            types.as_deref().map(|d| (d, prefix.as_str())),
                        )?;
                        print!("{text}");
                        Ok(())
                    })())
                }
                _ => {
                    eprintln!("Uso: archisim cajas-sobre <proyecto> <salida.archisim> [--modulo 6.10x3.60] [--tipos <carpeta> [--prefijo L160]]");
                    ExitCode::FAILURE
                }
            }
        }
        Some("locales") => match (args.get(1), args.get(2), args.get(3)) {
            (Some(p), Some(l), Some(o)) => report((|| {
                print!(
                    "{}",
                    locales::run(Path::new(p), Path::new(l), Path::new(o))?
                );
                Ok(())
            })()),
            _ => {
                eprintln!("Uso: archisim locales <proyecto> <locales.csv> <salida.archisim>");
                ExitCode::FAILURE
            }
        },
        Some("plantilla") => {
            let flag = |name: &str| args.iter().position(|a| a == name);
            let catalogue = flag("--perfiles")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from);
            let cavities = flag("--entramado").is_some();
            match (args.get(1), args.get(2)) {
                (Some(input), Some(output))
                    if !input.starts_with("--") && !output.starts_with("--") =>
                {
                    report((|| {
                        let text = plantilla::run(
                            Path::new(input),
                            Path::new(output),
                            catalogue.as_deref(),
                            cavities,
                        )?;
                        print!("{text}");
                        Ok(())
                    })())
                }
                _ => {
                    eprintln!("Uso: archisim plantilla <proyecto> <plantilla.archisim> [--perfiles <catálogo.csv>] [--entramado]");
                    ExitCode::FAILURE
                }
            }
        }
        Some("taller-cajas") => match args.get(1) {
            Some(project) => {
                let path = PathBuf::from(project);
                let dir = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("out"));
                report((|| {
                    let m = archisim_file::open(&path)?;
                    std::fs::create_dir_all(&dir)?;
                    let stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "proyecto".into());
                    print!("{}", archisim_struct::boxes::boxes_report(&m));
                    let n = cajas::workshop(&m, &dir, &stem)?;
                    if n == 0 {
                        println!("El proyecto no tiene cajas modulares.");
                    } else {
                        println!(
                            "Planos de taller: {n} hojas A3 en {}",
                            dir.join(format!("{stem}-taller.pdf")).display()
                        );
                    }
                    Ok(())
                })())
            }
            None => {
                eprintln!("Uso: archisim taller-cajas <proyecto> [carpeta]");
                std::process::exit(2);
            }
        },
        Some("entramado") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(entramado::write(&dir))
        }
        Some("trazado") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(trazado::write(&dir))
        }
        Some("calles") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(calles::write(&dir))
        }
        Some("ortofoto") => {
            match args.get(1) {
                Some(p) => {
                    let p = PathBuf::from(p);
                    let out = args.get(2).map_or_else(|| p.clone(), PathBuf::from);
                    report(ortofoto::run(&p, &out))
                }
                None => {
                    eprintln!("Uso: archisim ortofoto <proyecto georreferenciado.archisim> [salida.archisim]");
                    ExitCode::FAILURE
                }
            }
        }
        Some("vida-calle") => match args.get(1) {
            Some(p) => {
                let p = PathBuf::from(p);
                let out = args.get(2).map_or_else(|| p.clone(), PathBuf::from);
                report(street_life(&p, &out))
            }
            None => {
                eprintln!("Uso: archisim vida-calle <proyecto.archisim> [salida.archisim]");
                ExitCode::FAILURE
            }
        },
        Some("entorno") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(entorno::write(&dir))
        }
        Some("tipos-calle") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(calles::write_types(&dir))
        }
        Some("escaleras") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(escaleras::write(&dir))
        }
        Some("armado") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(armado::write(&dir))
        }
        Some("edificio") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(edificio::write(&dir))
        }
        Some("instalaciones") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(instalaciones::write(&dir))
        }
        Some("tierras") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(tierras::write(&dir))
        }
        Some("muros-talud") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(muros_talud::write(&dir))
        }
        Some("fachadas") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(fachadas::write(&dir))
        }
        Some("cubiertas") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(cubiertas::write(&dir))
        }
        Some("gama") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(gama::write(&dir))
        }
        Some("formas") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(formas::write(&dir))
        }
        Some("vivero") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(vivero::write(&dir))
        }
        Some("muestrario") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out"));
            report(muestrario::write(&dir))
        }
        Some("biblioteca") if args.iter().any(|a| a == "--ampliada") => {
            let rest: Vec<&String> = args[1..].iter().filter(|a| *a != "--ampliada").collect();
            let dir = PathBuf::from(rest.first().map_or("biblioteca-ampliada", |s| s.as_str()));
            report(run_extended_library(&dir))
        }
        Some("biblioteca") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("out/biblioteca"));
            report(run_library(&dir, args.get(2).map(Path::new)))
        }
        Some("fontaneria") => match args.get(1) {
            Some(project) => {
                let after = |flag: &str| {
                    args.iter()
                        .position(|a| a == flag)
                        .and_then(|i| args.get(i + 1))
                        .map(Path::new)
                };
                report(run_water(
                    Path::new(project),
                    after("--diametros"),
                    after("--esquemas"),
                ))
            }
            None => {
                eprintln!("Falta el proyecto: archisim fontaneria <proyecto.archisim>");
                ExitCode::FAILURE
            }
        },
        Some("saneamiento") => match args.get(1) {
            Some(project) => {
                let after = |flag: &str| {
                    args.iter()
                        .position(|a| a == flag)
                        .and_then(|i| args.get(i + 1))
                        .map(Path::new)
                };
                report(run_drainage(
                    Path::new(project),
                    after("--diametros"),
                    after("--esquemas"),
                ))
            }
            None => {
                eprintln!("Falta el proyecto: archisim saneamiento <proyecto.archisim>");
                ExitCode::FAILURE
            }
        },
        Some("renovables") => match args.get(1) {
            Some(project) => report(run_renewables(
                Path::new(project),
                args.iter()
                    .position(|a| a == "--pvgis")
                    .and_then(|i| args.get(i + 1))
                    .map(Path::new),
            )),
            None => {
                eprintln!("Falta el proyecto: archisim renovables <proyecto.archisim>");
                ExitCode::FAILURE
            }
        },
        Some("climatizacion") => match args.get(1) {
            Some(project) => report(run_thermal(Path::new(project))),
            None => {
                eprintln!("Falta el proyecto: archisim climatizacion <proyecto.archisim>");
                ExitCode::FAILURE
            }
        },
        Some("ventilacion") => match args.get(1).map(String::as_str) {
            Some("--ejemplo") => report(ventilation_example(Path::new(
                args.get(2).map_or("out", String::as_str),
            ))),
            Some("--ejemplo-terciario") => report(office_example(Path::new(
                args.get(2).map_or("out", String::as_str),
            ))),
            Some(project) => {
                let after = |flag: &str| {
                    args.iter()
                        .position(|a| a == flag)
                        .and_then(|i| args.get(i + 1))
                        .map(Path::new)
                };
                report(run_ventilation(
                    Path::new(project),
                    after("--tamanos"),
                    after("--esquemas"),
                ))
            }
            None => {
                eprintln!("Falta el proyecto: archisim ventilacion <proyecto.archisim>");
                ExitCode::FAILURE
            }
        },
        Some("memoria-instalaciones") => match args.get(1) {
            Some(project) => report(run_services_report(
                Path::new(project),
                Path::new(args.get(2).map_or("out", String::as_str)),
            )),
            None => {
                eprintln!(
                    "Falta el proyecto: archisim memoria-instalaciones <proyecto.archisim> [carpeta]"
                );
                ExitCode::FAILURE
            }
        },
        Some("electricidad") => match args.get(1) {
            Some(project) => report(run_electrical(
                Path::new(project),
                args.get(2).map(Path::new),
            )),
            None => {
                eprintln!("Falta el proyecto: archisim electricidad <proyecto.archisim> [carpeta]");
                ExitCode::FAILURE
            }
        },
        Some("estructura") => match args.get(1) {
            Some(project) => report(run_structure(
                Path::new(project),
                args.get(2).map(Path::new),
            )),
            None => {
                eprintln!("Falta el proyecto: archisim estructura <proyecto.archisim>");
                ExitCode::FAILURE
            }
        },
        Some("cte") => match args.get(1) {
            Some(project) => report(run_cte(Path::new(project), args.get(2).map(Path::new))),
            None => {
                eprintln!("Falta el proyecto: archisim cte <proyecto.archisim> [informe.csv]");
                ExitCode::FAILURE
            }
        },
        Some("medir") if args.get(1).map(String::as_str) == Some("proyecto") => {
            let path = PathBuf::from(
                args.get(2)
                    .map(String::as_str)
                    .unwrap_or("out/edificio.archisim"),
            );
            report(grande::measure_project(&path))
        }
        Some("medir") if args.get(1).map(String::as_str) == Some("vistas") => {
            let path = PathBuf::from(
                args.get(2)
                    .map(String::as_str)
                    .unwrap_or("out/demo.archisim"),
            );
            report(grande::measure_views(&path))
        }
        Some("medir") => {
            let n = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(20);
            let dir = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("out"));
            report(
                std::fs::create_dir_all(&dir)
                    .map_err(Into::into)
                    .and_then(|()| grande::measure(n, &dir)),
            )
        }
        Some("--version") | Some("-V") => {
            println!("archisim {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") | None => {
            print!("{HELP}\n{}", colaboracion::HELP);
            ExitCode::SUCCESS
        }
        Some(other) => colaboracion::run(&args).unwrap_or_else(|| {
            eprintln!("Orden desconocida: {other}\n\n{HELP}");
            ExitCode::FAILURE
        }),
    }
}

fn report(result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_demo(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let doc = demo::build()?;
    std::fs::create_dir_all(dir)?;
    let project = dir.join(format!("demo.{}", archisim_file::EXTENSION));
    archisim_file::save(doc.model(), &project)?;
    std::fs::write(dir.join("banco-ejemplo.bc3"), demo::EXAMPLE_BANK)?;
    summary(doc.model());
    println!("  Proyecto: {}", project.display());
    export_all(doc.model(), dir, "demo")?;
    // The house as a module of an urbanisation (I5).
    let master = demo::masterplan(&project)?;
    let master_path = dir.join(format!("demo-masterplan.{}", archisim_file::EXTENSION));
    archisim_file::save(&master, &master_path)?;
    let ifc_path = dir.join("demo-masterplan.ifc");
    std::fs::write(&ifc_path, ifc_text(&master, "demo-masterplan", None))?;
    std::fs::write(
        dir.join("demo-masterplan-ifc4.ifc"),
        ifc_text_in(&master, "demo-masterplan", None, archisim_ifc::Schema::Ifc4),
    )?;
    println!(
        "  Urbanización con {} copias de la vivienda: {} y {}",
        master.modules().count(),
        master_path.display(),
        ifc_path.display()
    );
    Ok(())
}

fn run_extended_library(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut model = archisim_core::Model::new("Biblioteca ampliada");
    model.add_level("Planta baja", 0.0);
    let n = archisim_library::extended::add_extended_library(&mut model)?;
    let files = write_library_files(&model, dir)?;
    println!(
        "Biblioteca ampliada: {n} tipos en {files} ficheros, en {}",
        dir.display()
    );
    Ok(())
}

/// Writes the family types of a model as library files in folders by
/// category; returns how many files.
fn write_library_files(
    model: &archisim_core::Model,
    dir: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut n = 0;
    for (category, family, text) in model.catalog_files() {
        let folder = library_folder(&category);
        let sub = dir.join(folder);
        std::fs::create_dir_all(&sub)?;
        let path = sub.join(format!("{}.asfam", family.replace(['/', '\\', ':'], "-")));
        std::fs::write(&path, text)?;
        println!("  {}", path.display());
        n += 1;
    }
    Ok(n)
}

fn library_folder(category: &str) -> &'static str {
    match category {
        "door" => "Puertas",
        "window" => "Ventanas",
        "void" => "Huecos",
        "furniture" => "Mobiliario",
        "sanitary" => "Sanitarios",
        "kitchen" => "Cocina",
        "site" => "Entorno",
        "street" => "Mobiliario urbano",
        "nature" => "Elementos naturales",
        "people" => "Personas",
        "vehicle" => "Vehículos",
        _ => "Otros",
    }
}

fn run_library(dir: &Path, project: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let model = match project {
        Some(p) => {
            archisim_file::open(p).map_err(|e| format!("no se puede abrir {}: {e}", p.display()))?
        }
        None => archisim_library::new_project("Plantilla"),
    };
    let n = write_library_files(&model, dir)?;
    // Wall and slab types, one file for each.
    for (file, walls) in [("Muros.astipos", true), ("Forjados.astipos", false)] {
        let mut cat = archisim_core::LayeredCatalog::default();
        let refs: Vec<archisim_core::LayeredRef> = if walls {
            model
                .wall_types()
                .map(|t| archisim_core::LayeredRef::Wall(t.id))
                .collect()
        } else {
            model
                .slab_types()
                .map(|t| archisim_core::LayeredRef::Slab(t.id))
                .collect()
        };
        for r in refs {
            let (entry, materials) = model.layered_entry(r)?;
            for m in materials {
                if !cat.materials.iter().any(|x| x.name == m.name) {
                    cat.materials.push(m);
                }
            }
            cat.types.push(entry);
        }
        let sub = dir.join("Multicapas");
        std::fs::create_dir_all(&sub)?;
        let path = sub.join(file);
        std::fs::write(&path, archisim_core::layered_catalog_text(&cat))?;
        println!("  {} ({} tipos)", path.display(), cat.types.len());
    }
    println!(
        "{n} familias con sus tipos y los tipos de muro y forjado en {}",
        dir.display()
    );
    Ok(())
}

fn run_water(
    project: &Path,
    sized: Option<&Path>,
    sheets_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let w = archisim_mep::water::water(&model);
    print!("{}", archisim_mep::water::report(&model, &w));
    if let Some(dir) = sheets_dir {
        std::fs::create_dir_all(dir)?;
        let stem = project
            .file_stem()
            .map_or("proyecto".into(), |s| s.to_string_lossy().to_string());
        let sheets = archisim_plan::water_schematic::pages(&model);
        let titles: Vec<String> = (0..sheets.len())
            .map(|i| format!("FO-{:02}", i + 1))
            .collect();
        let pdf_pages: Vec<archisim_export::pdf::Page> = sheets
            .iter()
            .zip(&titles)
            .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
            .collect();
        if !sheets.is_empty() {
            let pdf = dir.join(format!("{stem}-fontaneria.pdf"));
            std::fs::write(
                &pdf,
                archisim_export::pdf::pdf(&pdf_pages, "Esquemas de fontanería")?,
            )?;
            for (svg, title) in sheets.iter().zip(&titles) {
                let base = dir.join(format!("{stem}-fontaneria-{}", title.to_lowercase()));
                std::fs::write(base.with_extension("svg"), svg)?;
                std::fs::write(
                    base.with_extension("dxf"),
                    archisim_export::dxf::page_dxf(&model, svg)?,
                )?;
            }
            println!(
                "\nEsquemas de fontanería ({} hojas A3): {} (y en SVG y DXF)",
                sheets.len(),
                pdf.display()
            );
        }
    }
    if let Some(out) = sized {
        let runs = w.sized_runs();
        for (run, d) in &runs {
            model.edit_run(*run, |r| r.width = *d)?;
        }
        archisim_file::save(&model, out)?;
        println!(
            "\nDiámetros calculados en {} tuberías: {}",
            runs.len(),
            out.display()
        );
    }
    Ok(())
}

fn run_thermal(project: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let t = archisim_mep::thermal::thermal(&model);
    print!("{}", archisim_mep::thermal::report(&model, &t));
    Ok(())
}

/// The renewables of a project; with `--pvgis fichero`, with the climate
/// of a monthly file of PVGIS (only for this report: the project is not
/// changed).
fn run_renewables(project: &Path, pvgis: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    if let Some(file) = pvgis {
        let text = std::fs::read_to_string(file)
            .map_err(|e| format!("no se puede leer {}: {e}", file.display()))?;
        let (sun, air) = archisim_mep::solar::read_pvgis(&text).ok_or_else(|| {
            format!(
                "{}: no lleva la radiación mensual de PVGIS (columnas «month» y «H(h)_m»)",
                file.display()
            )
        })?;
        let mut site = model.renewables_site().clone();
        site.irradiation = Some(sun);
        if air.is_some() {
            site.ambient = air;
        }
        model
            .set_renewables_site(site)
            .map_err(|e| format!("{e}"))?;
    }
    let r = archisim_mep::renewables::renewables(&model);
    print!("{}", archisim_mep::renewables::report(&r));
    Ok(())
}

/// The installations' report: a PDF of A4 pages and the same in text.
fn run_services_report(project: &Path, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let model = model.flattened().map_err(|e| format!("{e}"))?;
    std::fs::create_dir_all(dir)?;
    let stem = project
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("proyecto");
    let blocks = archisim_plan::services_report::report(&model);
    let pages = archisim_plan::calc_report::pages(&model, &blocks);
    let titles: Vec<String> = (0..pages.len())
        .map(|i| format!("Memoria de instalaciones {}", i + 1))
        .collect();
    let pdf_pages: Vec<archisim_export::pdf::Page> = pages
        .iter()
        .zip(&titles)
        .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
        .collect();
    let pdf = dir.join(format!("{stem}-memoria-instalaciones.pdf"));
    std::fs::write(
        &pdf,
        archisim_export::pdf::pdf(&pdf_pages, "Memoria de instalaciones")?,
    )?;
    let txt = dir.join(format!("{stem}-memoria-instalaciones.txt"));
    std::fs::write(&txt, archisim_struct::report::to_text(&blocks))?;
    println!(
        "Memoria de instalaciones ({} hojas A4): {} (y {})",
        pages.len(),
        pdf.display(),
        txt.display()
    );
    Ok(())
}

fn run_ventilation(
    project: &Path,
    sized: Option<&Path>,
    sheets_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let v = archisim_mep::ventilation::ventilation(&model);
    print!("{}", archisim_mep::ventilation::report(&model, &v));
    if let Some(dir) = sheets_dir {
        let stem = project
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("proyecto");
        let sheets = archisim_plan::water_schematic::ventilation_pages(&model);
        if let Some(pdf) = write_sheets(
            &model,
            dir,
            stem,
            "ventilacion",
            "VE",
            "Esquemas de ventilación",
            &sheets,
        )? {
            println!(
                "\nEsquemas de ventilación ({} hojas A3): {} (y en SVG y DXF)",
                sheets.len(),
                pdf.display()
            );
        }
    }
    if let Some(out) = sized {
        let runs = v.sized_runs(&model);
        let devices = v.sized_devices(&model);
        for (run, w) in &runs {
            model.edit_run(*run, |r| r.width = r.width.max(*w))?;
        }
        for (d, w, dp) in &devices {
            model.edit_device(*d, |x| {
                x.width = *w;
                x.depth = *dp;
            })?;
        }
        archisim_file::save(&model, out)?;
        println!(
            "\nTamaños calculados en {} conductos y {} aparatos: {}",
            runs.len(),
            devices.len(),
            out.display()
        );
    }
    Ok(())
}

/// A set of A3 sheets as one PDF and an SVG and a DXF each:
/// `<stem>-<name>.pdf`, `<stem>-<name>-<code>.svg`. `None` if there are
/// none.
fn write_sheets(
    model: &archisim_core::Model,
    dir: &Path,
    stem: &str,
    name: &str,
    prefix: &str,
    title: &str,
    sheets: &[String],
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    if sheets.is_empty() {
        return Ok(None);
    }
    std::fs::create_dir_all(dir)?;
    let titles: Vec<String> = (0..sheets.len())
        .map(|i| format!("{prefix}-{:02}", i + 1))
        .collect();
    let pdf_pages: Vec<archisim_export::pdf::Page> = sheets
        .iter()
        .zip(&titles)
        .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
        .collect();
    let pdf = dir.join(format!("{stem}-{name}.pdf"));
    std::fs::write(&pdf, archisim_export::pdf::pdf(&pdf_pages, title)?)?;
    for (svg, title) in sheets.iter().zip(&titles) {
        let base = dir.join(format!("{stem}-{name}-{}", title.to_lowercase()));
        std::fs::write(base.with_extension("svg"), svg)?;
        std::fs::write(
            base.with_extension("dxf"),
            archisim_export::dxf::page_dxf(model, svg)?,
        )?;
    }
    Ok(Some(pdf))
}

fn ventilation_example(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let mut m = archisim_core::Model::new("Piso ventilado");
    let l0 = m.add_level("Planta baja", 0.0);
    let l1 = m.add_level("Cubierta", 3.0);
    let lib = archisim_library::add_library(&mut m)?;
    archisim_mep::ventilation::example_flat(
        &mut m,
        l0,
        l1,
        lib.partition,
        lib.facade,
        lib.interior_door,
    );
    let hood = archisim_library::add_object_entry(&mut m, "Campana decorativa 60")?;
    archisim_mep::ventilation::example_hood(&mut m, l0, l1, hood);
    let path = dir.join("ventilacion-ejemplo.archisim");
    archisim_file::save(&m, &path)?;
    let v = archisim_mep::ventilation::ventilation(&m);
    print!("{}", archisim_mep::ventilation::report(&m, &v));
    println!("\n{}", path.display());
    Ok(())
}

/// `archisim ventilacion --ejemplo-terciario`: an office over a garage
/// (RITE and the garage's natural ventilation).
fn office_example(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let mut m = archisim_core::Model::new("Oficina con garaje");
    let s = m.add_level("Sótano", -3.0);
    let l0 = m.add_level("Planta baja", 0.0);
    m.add_level("Cubierta", 3.0);
    let lib = archisim_library::add_library(&mut m)?;
    archisim_mep::rite::example_office_garage(
        &mut m,
        s,
        l0,
        lib.partition,
        lib.facade,
        lib.interior_door,
        lib.void,
    );
    let path = dir.join("ventilacion-oficina.archisim");
    archisim_file::save(&m, &path)?;
    let v = archisim_mep::ventilation::ventilation(&m);
    print!("{}", archisim_mep::ventilation::report(&m, &v));
    println!("\n{}", path.display());
    Ok(())
}

fn run_drainage(
    project: &Path,
    sized: Option<&Path>,
    sheets_dir: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let d = archisim_mep::drain::drainage(&model);
    print!("{}", archisim_mep::drain::report(&model, &d));
    if let Some(dir) = sheets_dir {
        std::fs::create_dir_all(dir)?;
        let stem = project
            .file_stem()
            .map_or("proyecto".into(), |s| s.to_string_lossy().to_string());
        let sheets = archisim_plan::water_schematic::drainage_pages(&model);
        let titles: Vec<String> = (0..sheets.len())
            .map(|i| format!("SA-{:02}", i + 1))
            .collect();
        let pdf_pages: Vec<archisim_export::pdf::Page> = sheets
            .iter()
            .zip(&titles)
            .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
            .collect();
        if !sheets.is_empty() {
            let pdf = dir.join(format!("{stem}-saneamiento.pdf"));
            std::fs::write(
                &pdf,
                archisim_export::pdf::pdf(&pdf_pages, "Esquemas de saneamiento")?,
            )?;
            for (svg, title) in sheets.iter().zip(&titles) {
                let base = dir.join(format!("{stem}-saneamiento-{}", title.to_lowercase()));
                std::fs::write(base.with_extension("svg"), svg)?;
                std::fs::write(
                    base.with_extension("dxf"),
                    archisim_export::dxf::page_dxf(&model, svg)?,
                )?;
            }
            println!(
                "\nEsquemas de saneamiento ({} hojas A3): {} (y en SVG y DXF)",
                sheets.len(),
                pdf.display()
            );
        }
    }
    if let Some(out) = sized {
        let (runs, pipes) = d.sized_runs();
        let mut n = 0;
        for (run, w) in &runs {
            if model.runs().any(|r| r.id == *run && r.width + 1e-4 < *w) {
                model.edit_run(*run, |r| r.width = *w)?;
                n += 1;
            }
        }
        for (pipe, w) in &pipes {
            if model
                .devices()
                .any(|x| x.id == *pipe && x.width + 1e-4 < *w)
            {
                model.edit_device(*pipe, |x| {
                    x.width = *w;
                    x.depth = *w;
                })?;
                n += 1;
            }
        }
        archisim_file::save(&model, out)?;
        println!(
            "\nDiámetros calculados en {n} tuberías y bajantes: {}",
            out.display()
        );
    }
    Ok(())
}

fn run_electrical(project: &Path, out: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let model = model.flattened().map_err(|e| format!("{e}"))?;
    print!("{}", archisim_mep::electrical::report(&model));
    if let Some(dir) = out {
        std::fs::create_dir_all(dir)?;
        let stem = project
            .file_stem()
            .map_or("proyecto".into(), |s| s.to_string_lossy().to_string());
        let sheets = archisim_plan::single_line::pages(&model);
        if sheets.is_empty() {
            return Ok(());
        }
        let titles: Vec<String> = (0..sheets.len())
            .map(|i| format!("IE-{:02}", i + 1))
            .collect();
        let pdf_pages: Vec<archisim_export::pdf::Page> = sheets
            .iter()
            .zip(&titles)
            .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
            .collect();
        let pdf = dir.join(format!("{stem}-unifilar.pdf"));
        std::fs::write(
            &pdf,
            archisim_export::pdf::pdf(&pdf_pages, "Esquema unifilar")?,
        )?;
        for (svg, title) in sheets.iter().zip(&titles) {
            let dxf = dir.join(format!("{stem}-unifilar-{}.dxf", title.to_lowercase()));
            std::fs::write(&dxf, archisim_export::dxf::page_dxf(&model, svg)?)?;
        }
        println!(
            "\nEsquema unifilar ({} hojas A3): {} (y en DXF)",
            sheets.len(),
            pdf.display()
        );
    }
    Ok(())
}

fn run_structure(project: &Path, out: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let model = model.flattened().map_err(|e| format!("{e}"))?;
    print!("{}", archisim_struct::structure_report(&model));
    if let Some(dir) = out {
        std::fs::create_dir_all(dir)?;
        let stem = project
            .file_stem()
            .map_or("proyecto".into(), |s| s.to_string_lossy().to_string());
        let blocks = archisim_struct::report::report(&model);
        let pages = archisim_plan::calc_report::pages(&model, &blocks);
        let titled: Vec<(String, String)> = pages
            .iter()
            .enumerate()
            .map(|(i, p)| (p.clone(), format!("Anejo de cálculo {}", i + 1)))
            .collect();
        let pdf_pages: Vec<archisim_export::pdf::Page> = titled
            .iter()
            .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
            .collect();
        let pdf = archisim_export::pdf::pdf(&pdf_pages, "Anejo de cálculo de la estructura")?;
        let path = dir.join(format!("{stem}-anejo-calculo.pdf"));
        std::fs::write(&path, pdf)?;
        let txt = dir.join(format!("{stem}-anejo-calculo.txt"));
        std::fs::write(&txt, archisim_struct::report::to_text(&blocks))?;
        // The structure drawings, A3.
        let sheets = archisim_plan::structure_sheets::pages(&model);
        let titles: Vec<String> = (0..sheets.len())
            .map(|i| format!("E-{:02}", i + 1))
            .collect();
        let pdf_pages: Vec<archisim_export::pdf::Page> = sheets
            .iter()
            .zip(&titles)
            .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
            .collect();
        let planos = dir.join(format!("{stem}-planos-estructura.pdf"));
        std::fs::write(
            &planos,
            archisim_export::pdf::pdf(&pdf_pages, "Planos de estructura")?,
        )?;
        println!(
            "Planos de estructura ({} hojas A3): {}",
            sheets.len(),
            planos.display()
        );
        println!(
            "\nAnejo de cálculo ({} hojas A4): {}\nEn texto: {}",
            pages.len(),
            path.display(),
            txt.display()
        );
    }
    Ok(())
}

fn run_cte(project: &Path, csv: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let r = archisim_cte::check(&model);
    print!("{}", r.to_text(&model));
    if let Some(csv) = csv {
        std::fs::write(csv, r.to_csv(&model))?;
        println!("\nInforme CSV: {}", csv.display());
    }
    Ok(())
}

fn run_glb(project: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let model = model.flattened().map_err(|e| format!("{e}"))?;
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = archisim_render::glb::project_glb(&model);
    std::fs::write(out, &bytes)?;
    println!(
        "Modelo 3D en glTF ({:.1} MB): {}",
        bytes.len() as f64 / 1e6,
        out.display()
    );
    Ok(())
}

fn run_render(
    project: &Path,
    png: &Path,
    width: usize,
    light: archisim_render::Light,
    textures: bool,
    camera: Option<[f64; 6]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let model = model.flattened().map_err(|e| format!("{e}"))?;
    let width = width.clamp(16, 8000);
    let options = archisim_render::Options {
        width,
        height: width * 5 / 8,
        light,
        textures,
        ..Default::default()
    };
    let mut view = archisim_render::View::fit(&model);
    if let Some([x, y, z, tx, ty, tz]) = camera {
        view.eye = [x, y, z];
        view.target = [tx, ty, tz];
    }
    let start = std::time::Instant::now();
    let img = archisim_render::render(&model, &view, &options, &|_| {});
    let bytes =
        archisim_export::png::encode_rgb(img.width as u32, img.height as u32, &img.rgb, 96.0);
    if let Some(dir) = png.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(png, bytes)?;
    println!(
        "Imagen de {} × {} en {:.1} s: {}",
        img.width,
        img.height,
        start.elapsed().as_secs_f64(),
        png.display()
    );
    Ok(())
}

fn run_export(project: &Path, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let stem = project
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("proyecto")
        .to_string();
    if !dir.as_os_str().is_empty() {
        std::fs::create_dir_all(dir)?;
    }
    summary(&model);
    export_all(&model, dir, &stem)
}

fn run_sections(project: &Path, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let model = archisim_file::open(project)
        .map_err(|e| format!("no se puede abrir {}: {e}", project.display()))?;
    let stem = project
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("proyecto")
        .to_string();
    if !dir.as_os_str().is_empty() {
        std::fs::create_dir_all(dir)?;
    }
    write_sections(&model, dir, &stem)
}

fn run_import(ifc: &Path, project: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bytes =
        std::fs::read(ifc).map_err(|e| format!("no se puede leer {}: {e}", ifc.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let imported = import_ifc(&text).map_err(|e| format!("{}: {e}", ifc.display()))?;
    print!("{}", imported.report);
    if let Some(dir) = project.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&imported.model, project)?;
    summary(&imported.model);
    println!("  Proyecto: {}", project.display());
    Ok(())
}

fn summary(model: &Model) {
    let count = |f: fn(&archisim_core::OpeningType) -> bool| {
        model
            .openings()
            .filter(|o| model.opening_type(o.opening_type).is_some_and(f))
            .count()
    };
    let (doors, windows, voids) = (
        count(|t| t.is_door()),
        count(|t| t.is_window()),
        count(|t| t.is_void()),
    );
    let voids = if voids > 0 {
        format!(", {voids} huecos")
    } else {
        String::new()
    };
    println!(
        "Proyecto «{}»: {} muros, {} forjados, {} puertas y {} ventanas{voids} en {} planta(s).",
        model.project_name,
        model.walls().count(),
        model.slabs().count(),
        doors,
        windows,
        model.levels().len()
    );
}

/// IFC 4.3 file of a model, named after the project file.
fn ifc_text(model: &Model, stem: &str, timestamp: Option<String>) -> String {
    ifc_text_in(model, stem, timestamp, archisim_ifc::Schema::Ifc4x3)
}

/// IFC file of a model in a schema.
fn ifc_text_in(
    model: &Model,
    stem: &str,
    timestamp: Option<String>,
    schema: archisim_ifc::Schema,
) -> String {
    export_ifc(
        model,
        &ExportOptions {
            file_name: format!("{stem}.ifc"),
            author: "ArchiSim".into(),
            organization: "ArchiSim".into(),
            timestamp,
            schema,
        },
    )
}

/// Plan file names: the first level keeps the plain name; the others get a
/// suffix ("casa.svg", "casa-cubierta.svg").
fn plan_files(model: &Model, stem: &str) -> Vec<(archisim_core::LevelId, String, String)> {
    model
        .levels()
        .iter()
        .enumerate()
        .map(|(i, level)| {
            let file = if i == 0 {
                format!("{stem}.svg")
            } else {
                format!("{stem}-{}.svg", slug(&level.name))
            };
            (level.id, level.name.clone(), file)
        })
        .collect()
}

fn export_all(model: &Model, dir: &Path, stem: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Views and exports show the copies of the modules as elements.
    let flat = model.flattened()?;
    let model = &flat;
    let ifc_path = dir.join(format!("{stem}.ifc"));
    std::fs::write(&ifc_path, ifc_text(model, stem, None))?;
    println!("  IFC 4.3: {}", ifc_path.display());
    // The same in IFC4, for programs that do not read 4.3 yet.
    let ifc4_path = dir.join(format!("{stem}-ifc4.ifc"));
    std::fs::write(
        &ifc4_path,
        ifc_text_in(model, stem, None, archisim_ifc::Schema::Ifc4),
    )?;
    println!("  IFC4:    {}", ifc4_path.display());
    let cte_path = dir.join(format!("{stem}-cte.txt"));
    std::fs::write(&cte_path, archisim_cte::check(model).to_text(model))?;
    println!("  Comprobación CTE: {}", cte_path.display());
    for (level, name, file) in plan_files(model, stem) {
        let svg_path = dir.join(&file);
        std::fs::write(
            &svg_path,
            render_plan(model, level, &PlanOptions::default()),
        )?;
        println!("  Planta «{name}»: {}", svg_path.display());
        let file_100 = format!("{}-1a100.svg", file.trim_end_matches(".svg"));
        let svg_100 = dir.join(&file_100);
        let options = PlanOptions {
            scale: 100.0,
            ..PlanOptions::default()
        };
        std::fs::write(&svg_100, render_plan(model, level, &options))?;
        println!("  Planta «{name}» 1:100: {}", svg_100.display());
        let dxf_path = dir.join(file.replace(".svg", ".dxf"));
        std::fs::write(
            &dxf_path,
            archisim_export::dxf::plan_dxf(model, level, &PlanOptions::default())?,
        )?;
        println!(
            "  Planta «{name}» en DXF (mm, 1:50): {}",
            dxf_path.display()
        );
        // Its reflected ceiling plan, if it has false ceilings (M7).
        if model.ceilings_on_level(level).next().is_some() {
            let file_ceiling = format!("{}-techos.svg", file.trim_end_matches(".svg"));
            let options = PlanOptions {
                ceiling: true,
                ..PlanOptions::default()
            };
            std::fs::write(dir.join(&file_ceiling), render_plan(model, level, &options))?;
            println!(
                "  Planta de techos «{name}»: {}",
                dir.join(&file_ceiling).display()
            );
        }
    }
    // Saved plans, each with its own scale, cut plane and look.
    for v in model.plan_views() {
        let path = dir.join(format!("{stem}-vista-{}.svg", slug(&v.name)));
        let options = PlanOptions {
            scale: v.scale,
            cut_plane: v.cut_plane,
            style: v.style.clone(),
            ceiling: v.kind == archisim_core::PlanKind::Ceiling,
            ..PlanOptions::default()
        };
        std::fs::write(&path, render_plan(model, v.level, &options))?;
        println!("  Vista «{}»: {}", v.name, path.display());
    }
    // Each sheet drawn once: its SVG also makes the PDF (below).
    let mut sheet_svgs: Vec<(String, String)> = Vec::new();
    for sheet in model.sheets() {
        let path = dir.join(format!("{stem}-plano-{}.svg", slug(&sheet.number)));
        let svg = archisim_plan::sheet::render_sheet(
            model,
            sheet.id,
            &archisim_plan::sheet::SheetOptions::default(),
        );
        std::fs::write(&path, &svg)?;
        sheet_svgs.push((svg, format!("{} {}", sheet.number, sheet.name)));
        println!(
            "  Plano {} «{}» ({}): {}",
            sheet.number,
            sheet.name,
            sheet.size_name(),
            path.display()
        );
        if sheet.presentation.is_some() {
            // Presentation boards also as an image, for screens and the web.
            let png = dir.join(format!("{stem}-plano-{}.png", slug(&sheet.number)));
            let svg = &sheet_svgs[sheet_svgs.len() - 1].0;
            std::fs::write(&png, archisim_export::png::png(svg, 200.0)?)?;
            println!(
                "  Plano {} en PNG (200 ppp): {}",
                sheet.number,
                png.display()
            );
        }
        let dxf = dir.join(format!("{stem}-plano-{}.dxf", slug(&sheet.number)));
        std::fs::write(&dxf, archisim_export::dxf::sheet_dxf(model, sheet.id)?)?;
        println!(
            "  Plano {} en DXF (mm de papel): {}",
            sheet.number,
            dxf.display()
        );
    }
    // Schedules as spreadsheets, and the measurements as a budget.
    for kind in archisim_core::ScheduleKind::ALL {
        let t = archisim_plan::schedules::table(model, kind, None);
        if t.rows.is_empty() {
            continue;
        }
        let path = dir.join(format!("{stem}-tabla-{}.csv", kind.code()));
        std::fs::write(&path, t.csv())?;
        println!("  Tabla «{}»: {}", kind.label(), path.display());
    }
    // The model for D5, Twinmotion or Blender.
    let glb = dir.join(format!("{stem}.glb"));
    let flat = model.flattened().unwrap_or_else(|_| model.clone());
    std::fs::write(&glb, archisim_render::glb::project_glb(&flat))?;
    println!(
        "  Modelo 3D en glTF (D5, Twinmotion, Blender): {}",
        glb.display()
    );
    let bc3 = dir.join(format!("{stem}.bc3"));
    let date = archisim_export::bc3::bc3_date(&archisim_ifc::utc_now());
    std::fs::write(
        &bc3,
        archisim_export::bc3::bc3(model, &date, env!("CARGO_PKG_VERSION")),
    )?;
    let (priced, pem) = archisim_plan::schedules::measurements(model)
        .iter()
        .flat_map(|c| &c.2)
        .filter_map(|i| model.price_of(&i.key).map(|c| c.price * i.total()))
        .fold((0, 0.0), |(n, t), a| (n + 1, t + a));
    if priced > 0 {
        println!(
            "  Presupuesto en BC3 (FIEBDC-3): {} ({priced} partidas con precio, ejecución material {pem:.2} €)",
            bc3.display()
        );
    } else {
        println!("  Mediciones en BC3 (FIEBDC-3): {}", bc3.display());
    }
    if !model.sheets().is_empty() {
        let pdf = dir.join(format!("{stem}-planos.pdf"));
        let pages: Vec<archisim_export::pdf::Page> = sheet_svgs
            .iter()
            .map(|(svg, title)| archisim_export::pdf::Page { svg, title })
            .collect();
        std::fs::write(
            &pdf,
            archisim_export::pdf::pdf(&pages, &model.project_name)?,
        )?;
        println!("  Planos en PDF: {}", pdf.display());
    }
    write_sections(model, dir, stem)
}

/// Every section and elevation of a project, at 1:100 and at 1:50 (with
/// the names of the pipes cut, the bars and the joints).
fn write_sections(model: &Model, dir: &Path, stem: &str) -> Result<(), Box<dyn std::error::Error>> {
    for sec in model.sections() {
        let drawing = archisim_plan::section::section_drawing(model, sec);
        for (scale, suffix) in [(100.0, ""), (50.0, "-1a50")] {
            let file = format!("{stem}-{}{suffix}.svg", slug(&sec.title()));
            let path = dir.join(&file);
            let options = PlanOptions {
                scale,
                ..PlanOptions::default()
            };
            std::fs::write(
                &path,
                archisim_plan::section::render_section(model, &drawing, &options),
            )?;
            println!("  {} 1:{scale}: {}", sec.title(), path.display());
        }
    }
    Ok(())
}

/// A file-name friendly version of a level name: "Planta 1ª" → "planta-1a".
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' | 'ª' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'º' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !(out.is_empty() || out.ends_with('-')) {
            out.push(c);
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Rasterizes an SVG drawing to a PNG at `dpi` dots per inch.
fn svg_to_png(svg: &Path, png: &Path, dpi: f32) -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(svg).map_err(|e| format!("{}: {e}", svg.display()))?;
    let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default())
        .map_err(|e| format!("{}: {e}", svg.display()))?;
    // usvg sizes are in CSS pixels (96 per inch).
    let k = dpi / 96.0;
    let size = tree.size();
    let w = (size.width() * k).ceil().max(1.0) as u32;
    let h = (size.height() * k).ceil().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("imagen demasiado grande")?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(k, k),
        &mut pixmap.as_mut(),
    );
    pixmap
        .save_png(png)
        .map_err(|e| format!("{}: {e}", png.display()))?;
    println!("{} ({w} × {h} px)", png.display());
    Ok(())
}

/// `archisim vida-calle`: trees, street lights, benches, bins, cars,
/// people and front gardens along the streets of a project.
fn street_life(project: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut m = archisim_file::open(project)?;
    let levels: Vec<_> = m.levels().iter().map(|l| l.id).collect();
    let mut total = archisim_library::street_life::LifeReport::default();
    for l in levels {
        let r = archisim_library::street_life::add_street_life(
            &mut m,
            l,
            &archisim_library::street_life::LifeOptions::default(),
        )?;
        total.trees += r.trees;
        total.furniture += r.furniture;
        total.cars += r.cars;
        total.people += r.people;
        total.garden_pieces += r.garden_pieces;
    }
    archisim_file::save(&m, out)?;
    println!("En las calles: {}", total.line());
    println!("Proyecto: {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saving the demo house and opening it again must give exactly the same
    /// IFC (every GlobalId kept) and the same plans.
    #[test]
    fn demo_survives_save_and_open() {
        let original = demo::build().unwrap().model().clone();
        let text = archisim_file::to_string(&original);
        let reopened = archisim_file::from_str(&text).unwrap();
        assert_eq!(reopened, original);
        let stamp = || Some("2026-10-17T00:00:00".to_string());
        assert_eq!(
            ifc_text(&reopened, "demo", stamp()),
            ifc_text(&original, "demo", stamp())
        );
        for (level, _, _) in plan_files(&original, "demo") {
            assert_eq!(
                render_plan(&reopened, level, &PlanOptions::default()),
                render_plan(&original, level, &PlanOptions::default())
            );
        }
        assert_eq!(archisim_file::to_string(&reopened), text);
    }

    /// The demo exported to IFC and imported again is the same building,
    /// with the same GlobalIds, and exports to an equivalent IFC.
    #[test]
    fn demo_survives_ifc_round_trip() {
        let original = demo::build().unwrap().model().clone();
        let ifc = ifc_text(&original, "demo", None);
        let imported = import_ifc(&ifc).unwrap();
        let diffs = archisim_ifc::differences(&original, &imported.model);
        assert!(diffs.is_empty(), "{}", diffs.join("\n"));
        assert!(imported.report.skipped.is_empty(), "{}", imported.report);
        assert!(imported.report.warnings.is_empty(), "{}", imported.report);
        // The accessories of the types come back to their types.
        for t in original.wall_types().filter(|t| !t.sweeps.is_empty()) {
            let back = imported
                .model
                .wall_types()
                .find(|x| x.name == t.name)
                .unwrap();
            assert_eq!(back.sweeps.len(), t.sweeps.len(), "{}", t.name);
        }
        for t in original.slab_types().filter(|t| t.edge.is_some()) {
            let back = imported
                .model
                .slab_types()
                .find(|x| x.name == t.name)
                .unwrap();
            assert_eq!(
                back.edge.as_ref().map(|e| e.profile.len()),
                t.edge.as_ref().map(|e| e.profile.len())
            );
        }
        // Saved as a project and opened again, still the same.
        let text = archisim_file::to_string(&imported.model);
        let reopened = archisim_file::from_str(&text).unwrap();
        assert!(archisim_ifc::differences(&original, &reopened).is_empty());
    }

    #[test]
    fn slugs() {
        assert_eq!(super::slug("Cubierta"), "cubierta");
        assert_eq!(super::slug("Planta 1ª (vivienda)"), "planta-1a-vivienda");
    }
}
