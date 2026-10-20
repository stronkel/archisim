//! The tools by discipline (`docs/INTERFAZ.md` §2.2–2.3): a narrow rail of
//! disciplines on the far left and, beside it, the tools of the one chosen
//! as tiles of a fixed size, four to a row, with Properties below. Tools
//! still to come are shown faint and say what brings them, so every
//! discipline already has its final shape.
//!
//! Besides the tiles there are the orders that are not tools of a
//! discipline (2D drafting and editing, AutoCAD's): they are found with the
//! command search (Ctrl+Espacio) and typed in the command line.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui, Vec2};

use super::coord_ui::CoordTab;
use super::glyphs::{self, Glyph};
use super::icons::{self, Icon};
use super::theme::Palette;
use super::widgets::strong_font;
use super::{ArchiSimApp, Tool};
use crate::dim_tool::DimMode;

/// A way of working: what the column of tools offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Discipline {
    Concept,
    #[default]
    Architecture,
    Structure,
    Services,
    Site,
    Documentation,
    Analysis,
    Coordination,
}

impl Discipline {
    /// In the order of the rail.
    pub const ALL: [Discipline; 8] = [
        Discipline::Concept,
        Discipline::Architecture,
        Discipline::Structure,
        Discipline::Services,
        Discipline::Site,
        Discipline::Documentation,
        Discipline::Analysis,
        Discipline::Coordination,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Discipline::Concept => "Concepto",
            Discipline::Architecture => "Arquitectura",
            Discipline::Structure => "Estructura",
            Discipline::Services => "Instalaciones",
            Discipline::Site => "Urbanismo",
            Discipline::Documentation => "Documentación",
            Discipline::Analysis => "Análisis",
            Discipline::Coordination => "Coordinación",
        }
    }

    fn purpose(self) -> &'static str {
        match self {
            Discipline::Concept => "Anteproyecto: volúmenes, masas con edificabilidad y ejes",
            Discipline::Architecture => "El edificio",
            Discipline::Structure => "Pilares, vigas, forjados, muros de carga y cimentación",
            Discipline::Services => "Fontanería, climatización, electricidad e iluminación",
            Discipline::Site => "Parcela, terreno, viales y vegetación",
            Discipline::Documentation => "Cotas, etiquetas, secciones, alzados, planos y tablas",
            Discipline::Analysis => "Comprobaciones del proyecto: CTE y mediciones",
            Discipline::Coordination => {
                "Vínculos, módulos, interferencias, incidencias, requisitos y equipo"
            }
        }
    }

    /// The drawing of the discipline on the rail.
    fn glyph(self) -> Glyph {
        match self {
            Discipline::Concept => Glyph::Volume,
            Discipline::Architecture => Glyph::Wall,
            Discipline::Structure => Glyph::Column,
            Discipline::Services => Glyph::Pipe,
            Discipline::Site => Glyph::Tree,
            Discipline::Documentation => Glyph::Dimension,
            Discipline::Analysis => Glyph::Cte,
            Discipline::Coordination => Glyph::Link,
        }
    }

    /// Its tiles, in their order.
    pub fn tiles(self) -> &'static [Tile] {
        match self {
            Discipline::Concept => CONCEPT,
            Discipline::Architecture => ARCHITECTURE,
            Discipline::Structure => STRUCTURE,
            Discipline::Services => SERVICES,
            Discipline::Site => SITE,
            Discipline::Documentation => DOCUMENTATION,
            Discipline::Analysis => ANALYSIS,
            Discipline::Coordination => COORDINATION,
        }
    }

    /// Whether one of its tiles does `pick`.
    pub fn offers(self, pick: Pick) -> bool {
        self.tiles().iter().any(|t| t.pick == pick)
    }

    /// The discipline a pick belongs to when it is taken from elsewhere
    /// (its key, the search): the first that offers it, the building first.
    pub fn home(pick: Pick) -> Option<Discipline> {
        const BY_PRIORITY: [Discipline; 8] = [
            Discipline::Architecture,
            Discipline::Structure,
            Discipline::Documentation,
            Discipline::Site,
            Discipline::Analysis,
            Discipline::Coordination,
            Discipline::Concept,
            Discipline::Services,
        ];
        BY_PRIORITY.into_iter().find(|d| d.offers(pick))
    }
}

/// Rows of tiles kept for every discipline, so that Properties below does
/// not move when the discipline changes (the one with most tools fills
/// them).
pub const TILE_ROWS: usize = 3;
pub const TILE_COLUMNS: usize = 4;
const TILE_HEIGHT: f32 = 54.0;
const TILE_GAP: f32 = 4.0;

/// What a tile does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Tool(Tool),
    /// The dimension tool placing level marks.
    LevelMark,
    /// The slab tool drawing holes through slabs.
    SlabHole,
    /// An action, not a tool: it opens or makes something.
    Action(Action),
    /// A 2D drafting or editing command (AutoCAD's), by its name.
    Command(&'static str),
    /// Not there yet: what brings it.
    Later(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The four elevations around the building.
    Elevations,
    NewSheet,
    Schedule,
    Takeoff,
    Cte,
    Link,
    Versions,
    Clashes,
    Issues,
    Requirements,
}

pub struct Tile {
    pub pick: Pick,
    pub glyph: Glyph,
    pub name: &'static str,
    pub key: &'static str,
    pub hint: &'static str,
}

const fn tile(
    pick: Pick,
    glyph: Glyph,
    name: &'static str,
    key: &'static str,
    hint: &'static str,
) -> Tile {
    Tile {
        pick,
        glyph,
        name,
        key,
        hint,
    }
}

const CONCEPT_LATER: &str = "el bloque I4 (disciplina Concepto)";
const GRID_LATER: &str = "los ejes y rejillas de replanteo";
const FOOTING_LATER: &str = "la cimentación (docs/DISCIPLINAS.md)";
const SERVICES_LATER: &str = "las herramientas de instalaciones (MEP)";
const SITE_LATER: &str = "el urbanismo (docs/DISCIPLINAS.md)";
const TEAM_LATER: &str = "el trabajo en equipo (4.6, carpeta compartida en Google Drive)";

const CONCEPT: &[Tile] = &[
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::Edge,
        "Línea",
        "",
        "Aristas del modelo, como SketchUp: al cerrar un contorno se crea una cara",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::FaceRect,
        "Rectángulo",
        "",
        "Una cara rectangular para empujar",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::FaceCircle,
        "Círculo",
        "",
        "Una cara circular para empujar",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::PushPull,
        "Empujar",
        "",
        "Empujar o tirar de una cara para hacer un volumen",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::FollowMe,
        "Seguir",
        "",
        "Llevar un perfil a lo largo de un camino",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::FaceOffset,
        "Equidist.",
        "",
        "Contorno paralelo dentro o fuera de una cara",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::Volume,
        "Volumen",
        "",
        "Prisma por su contorno y altura",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::Mass,
        "Masa",
        "",
        "Volumen por plantas con superficie construida, edificabilidad y ocupación en directo",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::Tape,
        "Cinta",
        "",
        "Medir y dejar guías",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::Paint,
        "Pintar",
        "",
        "Dar un material a una cara",
    ),
    tile(
        Pick::Later(GRID_LATER),
        Glyph::Grid,
        "Ejes",
        "",
        "Ejes de replanteo con su numeración",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::Convert,
        "Convertir",
        "",
        "Pasar el volumen a edificio: muros, forjados y cubierta con los tipos elegidos",
    ),
];

const ARCHITECTURE: &[Tile] = &[
    tile(
        Pick::Tool(Tool::Wall),
        Glyph::Wall,
        "Muro",
        "M",
        "Muros multicapa por su línea de referencia",
    ),
    tile(
        Pick::Tool(Tool::Slab),
        Glyph::Slab,
        "Forjado",
        "F",
        "Forjados por contorno o con la varita mágica",
    ),
    tile(
        Pick::Tool(Tool::Door),
        Glyph::Door,
        "Puerta",
        "P",
        "Puertas en muros",
    ),
    tile(
        Pick::Tool(Tool::Window),
        Glyph::Window,
        "Ventana",
        "V",
        "Ventanas en muros",
    ),
    tile(
        Pick::SlabHole,
        Glyph::Opening,
        "Abertura",
        "",
        "Huecos en forjados: patinillos, escaleras, lucernarios (en muros, la familia Hueco de Puerta)",
    ),
    tile(
        Pick::Tool(Tool::Roof),
        Glyph::Roof,
        "Cubierta",
        "T",
        "Cubiertas inclinadas por faldones: a cuatro, dos o un agua, con alero",
    ),
    tile(
        Pick::Tool(Tool::Stair),
        Glyph::Stair,
        "Escalera",
        "E",
        "Escaleras por su línea de huella: tramos, mesetas y comprobación del CTE",
    ),
    tile(
        Pick::Tool(Tool::Railing),
        Glyph::Railing,
        "Barandilla",
        "B",
        "Barandillas sueltas para terrazas, cubiertas y huecos: montantes, pasamanos y barrotes (CTE)",
    ),
    tile(
        Pick::Tool(Tool::CurtainWall),
        Glyph::CurtainWall,
        "Muro cortina",
        "U",
        "Muros cortina: montantes, travesaños y vidrios con modulación regular",
    ),
    tile(
        Pick::Tool(Tool::Object),
        Glyph::Object,
        "Objeto",
        "O",
        "Mobiliario, sanitarios y cocina: mesa, sillas, sofá, camas, inodoro, lavabo, ducha, bañera…",
    ),
    tile(
        Pick::Tool(Tool::Zone),
        Glyph::Room,
        "Local",
        "Z",
        "Locales (IfcSpace): clic dentro de un recinto (superficie útil), polígono o rectángulo; número, nombre y uso",
    ),
    tile(
        Pick::Later(GRID_LATER),
        Glyph::Grid,
        "Ejes",
        "",
        "Ejes de replanteo con su numeración",
    ),
];

const STRUCTURE: &[Tile] = &[
    tile(
        Pick::Tool(Tool::Column),
        Glyph::Column,
        "Pilar",
        "I",
        "Pilares de hormigón, acero o madera, hasta la planta superior",
    ),
    tile(
        Pick::Tool(Tool::Beam),
        Glyph::Beam,
        "Viga",
        "G",
        "Vigas por dos puntos, colgadas de la planta superior",
    ),
    tile(
        Pick::Tool(Tool::Slab),
        Glyph::StructuralSlab,
        "Forjado",
        "F",
        "Forjados por contorno o con la varita mágica",
    ),
    tile(
        Pick::Tool(Tool::Wall),
        Glyph::BearingWall,
        "Muro de carga",
        "M",
        "Muros portantes (la herramienta Muro con un tipo portante)",
    ),
    tile(
        Pick::Later(FOOTING_LATER),
        Glyph::Footing,
        "Zapata",
        "",
        "Zapatas aisladas bajo pilares y corridas bajo muros",
    ),
    tile(
        Pick::Later(FOOTING_LATER),
        Glyph::Raft,
        "Losa",
        "",
        "Losa de cimentación",
    ),
    tile(
        Pick::SlabHole,
        Glyph::Opening,
        "Abertura",
        "",
        "Huecos en forjados: patinillos, escaleras, lucernarios",
    ),
    tile(
        Pick::Later(GRID_LATER),
        Glyph::Grid,
        "Ejes",
        "",
        "Ejes de replanteo con su numeración",
    ),
];

const SERVICES: &[Tile] = &[
    tile(
        Pick::Later(SERVICES_LATER),
        Glyph::Pipe,
        "Tubería",
        "",
        "Fontanería, saneamiento y calefacción",
    ),
    tile(
        Pick::Later(SERVICES_LATER),
        Glyph::Duct,
        "Conducto",
        "",
        "Climatización y ventilación",
    ),
    tile(
        Pick::Later(SERVICES_LATER),
        Glyph::Fixture,
        "Aparato",
        "",
        "Calderas, bombas, máquinas de climatización",
    ),
    tile(
        Pick::Later(SERVICES_LATER),
        Glyph::Light,
        "Iluminación",
        "",
        "Luminarias",
    ),
    tile(
        Pick::Later(SERVICES_LATER),
        Glyph::Outlet,
        "Mecanismo",
        "",
        "Enchufes e interruptores",
    ),
];

const SITE: &[Tile] = &[
    tile(
        Pick::Tool(Tool::Terrain),
        Glyph::Terrain,
        "Terreno",
        "R",
        "Terreno por puntos de cota: curvas de nivel, sección del terreno y norte",
    ),
    tile(
        Pick::Later(SITE_LATER),
        Glyph::Plot,
        "Parcela",
        "",
        "Lindes, superficie y retranqueos",
    ),
    tile(
        Pick::Later(SITE_LATER),
        Glyph::Road,
        "Vial",
        "",
        "Viales por su eje y una sección tipo: calzada, aparcamiento, bordillo y acera",
    ),
    tile(
        Pick::Tool(Tool::Object),
        Glyph::Tree,
        "Vegetación",
        "O",
        "Árboles y arbustos (y vehículos): la herramienta Objeto, con la librería de vegetación",
    ),
    tile(
        Pick::Later(CONCEPT_LATER),
        Glyph::Volume,
        "Forma",
        "",
        "Volúmenes del entorno: edificios vecinos, taludes",
    ),
];

const DOCUMENTATION: &[Tile] = &[
    tile(
        Pick::Tool(Tool::Dimension),
        Glyph::Dimension,
        "Cota",
        "C",
        "Cotas lineales, encadenadas y de huecos de muro (angular DAN, radio DRA, arco DAR)",
    ),
    tile(
        Pick::LevelMark,
        Glyph::LevelMark,
        "Cota de nivel",
        "N",
        "Cota de nivel del suelo terminado",
    ),
    tile(
        Pick::Command("ETIQUETA"),
        Glyph::Tag,
        "Etiqueta",
        "ETI",
        "Etiquetas de puertas, ventanas, muros y locales (T: todas)",
    ),
    tile(
        Pick::Command("TEXTO"),
        Glyph::Text,
        "Texto",
        "TX",
        "Textos de una o varias líneas (directriz DIR, nube de revisión NR)",
    ),
    tile(
        Pick::Tool(Tool::Section),
        Glyph::Section,
        "Sección",
        "S",
        "Secciones por dos puntos: miran a la izquierda del trazo",
    ),
    tile(
        Pick::Action(Action::Elevations),
        Glyph::Elevation,
        "Alzado",
        "",
        "Crea los cuatro alzados (norte, sur, este y oeste) alrededor del edificio",
    ),
    tile(
        Pick::Action(Action::NewSheet),
        Glyph::Sheet,
        "Plano",
        "",
        "Un plano nuevo con su cajetín: arrastra sobre él vistas y tablas desde el navegador",
    ),
    tile(
        Pick::Action(Action::Schedule),
        Glyph::Schedule,
        "Tabla",
        "",
        "Tablas de planificación: carpintería, muros, forjados, locales, estructura…",
    ),
];

const ANALYSIS: &[Tile] = &[
    tile(
        Pick::Action(Action::Cte),
        Glyph::Cte,
        "CTE",
        "",
        "Comprobación del Código Técnico: evacuación, accesibilidad, escaleras y rampas, superficies",
    ),
    tile(
        Pick::Action(Action::Takeoff),
        Glyph::Takeoff,
        "Mediciones",
        "",
        "Mediciones por tipo, con demoliciones; se exportan a BC3 desde Archivo",
    ),
];

const COORDINATION: &[Tile] = &[
    tile(
        Pick::Action(Action::Link),
        Glyph::Link,
        "Vincular",
        "",
        "Vincular modelos IFC de estructuras e instalaciones como referencia",
    ),
    tile(
        Pick::Later("el bloque I5 (módulos)"),
        Glyph::Module,
        "Módulo",
        "",
        "Colocar otro proyecto (una vivienda tipo) varias veces, con giro y simetría",
    ),
    tile(
        Pick::Action(Action::Versions),
        Glyph::Versions,
        "Versiones",
        "",
        "Vínculos y sus versiones: lo nuevo, lo cambiado y lo borrado",
    ),
    tile(
        Pick::Action(Action::Clashes),
        Glyph::Clash,
        "Interferencias",
        "",
        "Comprobar interferencias con los modelos vinculados",
    ),
    tile(
        Pick::Action(Action::Issues),
        Glyph::Issue,
        "Incidencias",
        "",
        "Incidencias BCF: leer, crear y responder",
    ),
    tile(
        Pick::Action(Action::Requirements),
        Glyph::Requirements,
        "Requisitos",
        "",
        "Comprobar requisitos de información (IDS) sobre el IFC del proyecto",
    ),
    tile(
        Pick::Later(TEAM_LATER),
        Glyph::Team,
        "Equipo",
        "",
        "Varias personas en el mismo proyecto: quién está y qué tiene reservado",
    ),
    tile(
        Pick::Later(TEAM_LATER),
        Glyph::Publish,
        "Publicar",
        "",
        "Publicar los cambios para el resto del equipo",
    ),
];

/// An order that is not a tool of a discipline: found with the search.
pub struct Order {
    pub pick: Pick,
    pub icon: Icon,
    pub name: &'static str,
    pub key: &'static str,
    pub hint: &'static str,
    pub group: &'static str,
}

const fn order(
    group: &'static str,
    name: &'static str,
    command: &'static str,
    icon: Icon,
    key: &'static str,
    hint: &'static str,
) -> Order {
    Order {
        pick: Pick::Command(command),
        icon,
        name,
        key,
        hint,
        group,
    }
}

const DRAW: &str = "Dibujo 2D";
const EDIT: &str = "Modificar";
const NOTE: &str = "Anotación";

pub const ORDERS: &[Order] = &[
    order(
        NOTE,
        "Cota angular",
        "ACOANGULO",
        Icon::DimAngle,
        "DAN",
        "Cota del ángulo entre dos líneas o de un arco",
    ),
    order(
        NOTE,
        "Cota de radio",
        "ACORADIO",
        Icon::DimRadius,
        "DRA",
        "Cota de radio de un arco o círculo (diámetro: DDI)",
    ),
    order(
        NOTE,
        "Cota de longitud de arco",
        "ACOARCO",
        Icon::DimArc,
        "DAR",
        "Cota de la longitud de un arco",
    ),
    order(
        NOTE,
        "Directriz",
        "DIRECTRIZ",
        Icon::Leader,
        "DIR",
        "Flecha con texto",
    ),
    order(
        NOTE,
        "Nube de revisión",
        "NUBEREV",
        Icon::Cloud,
        "NR",
        "Nube de revisión",
    ),
    order(
        DRAW,
        "Línea de dibujo",
        "LINEA",
        Icon::Line,
        "L",
        "Líneas 2D de punto a punto",
    ),
    order(
        DRAW,
        "Polilínea",
        "POL",
        Icon::Pline,
        "PL",
        "Tramos rectos y de arco, con anchura",
    ),
    order(
        DRAW,
        "Rectángulo de dibujo",
        "RECTANG",
        Icon::GeoRect,
        "REC",
        "Por dos esquinas, girado o con esquinas redondeadas",
    ),
    order(
        DRAW,
        "Polígono",
        "POLIGONO",
        Icon::GeoPolygon,
        "PG",
        "Polígono regular inscrito o circunscrito",
    ),
    order(
        DRAW,
        "Arco",
        "ARCO",
        Icon::GeoArc,
        "A",
        "Por tres puntos o por centro, inicio y fin",
    ),
    order(
        DRAW,
        "Círculo de dibujo",
        "CIRCULO",
        Icon::GeoCircle,
        "CI",
        "Centro y radio o diámetro; 2P; 3P",
    ),
    order(
        DRAW,
        "Elipse",
        "ELIPSE",
        Icon::Ellipse,
        "EL",
        "Por un eje y la otra distancia, o por su centro",
    ),
    order(
        DRAW,
        "Sombreado",
        "SOMBCONT",
        Icon::Hatch,
        "SB",
        "Tramas y rellenos: clic dentro de un recinto",
    ),
    order(
        EDIT,
        "Mover",
        "DESPLAZA",
        Icon::Move,
        "D",
        "Mover lo seleccionado de un punto base a otro",
    ),
    order(
        EDIT,
        "Copiar",
        "COPIA",
        Icon::Copy,
        "CP",
        "Copias de lo seleccionado, una tras otra",
    ),
    order(
        EDIT,
        "Girar",
        "GIRA",
        Icon::Rotate,
        "GI",
        "Girar alrededor de un punto (Copia, Referencia)",
    ),
    order(
        EDIT,
        "Simetría",
        "SIMETRIA",
        Icon::Mirror,
        "SI",
        "Reflejar respecto a una línea",
    ),
    order(
        EDIT,
        "Escala",
        "ESCALA",
        Icon::Scale,
        "ES",
        "Cambiar de tamaño desde un punto base",
    ),
    order(
        EDIT,
        "Matriz",
        "MATRIZ",
        Icon::Array,
        "MA",
        "Copias en filas y columnas o alrededor de un centro",
    ),
    order(
        EDIT,
        "Desfase",
        "EQUIDIST",
        Icon::Offset,
        "EQ",
        "Paralela a una distancia",
    ),
    order(
        EDIT,
        "Recortar",
        "RECORTA",
        Icon::Trim,
        "RR",
        "Recortar contra cualquier borde (Mayús: alargar)",
    ),
    order(
        EDIT,
        "Alargar",
        "ALARGA",
        Icon::Extend,
        "AL",
        "Alargar hasta el borde más próximo (Mayús: recortar)",
    ),
    order(
        EDIT,
        "Empalme",
        "EMPALME",
        Icon::Fillet,
        "EM",
        "Unir dos líneas en esquina o con un arco",
    ),
    order(
        EDIT,
        "Medir",
        "DIST",
        Icon::Measure,
        "DI",
        "Distancia entre dos puntos (AA: área)",
    ),
];

/// One result of the command search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Found {
    pub pick: Pick,
    pub name: &'static str,
    pub key: &'static str,
    pub hint: &'static str,
    /// The discipline or group it is in.
    pub place: &'static str,
    pub mark: Mark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mark {
    Glyph(Glyph),
    Icon(Icon),
}

/// Lower case and without accents, to compare what is typed.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            c => c,
        })
        .collect()
}

/// Everything the search can find, each name once (a tool in several
/// disciplines is listed in the first, the building first).
fn searchable() -> Vec<Found> {
    let mut all: Vec<Found> = Vec::new();
    let tiles = std::iter::once(Discipline::Architecture)
        .chain(Discipline::ALL)
        .flat_map(|d| d.tiles().iter().map(move |t| (d, t)));
    for (d, t) in tiles {
        if all.iter().any(|f| f.name == t.name) {
            continue;
        }
        all.push(Found {
            pick: t.pick,
            name: t.name,
            key: t.key,
            hint: t.hint,
            place: d.name(),
            mark: Mark::Glyph(t.glyph),
        });
    }
    for o in ORDERS {
        all.push(Found {
            pick: o.pick,
            name: o.name,
            key: o.key,
            hint: o.hint,
            place: o.group,
            mark: Mark::Icon(o.icon),
        });
    }
    all
}

/// What the search offers for `text`: names that start with it first, then
/// names that contain it, then the exact key, then the hints.
pub fn search(text: &str) -> Vec<Found> {
    let q = fold(text.trim());
    if q.is_empty() {
        return Vec::new();
    }
    let mut ranked: Vec<(u8, Found)> = searchable()
        .into_iter()
        .filter_map(|f| {
            let name = fold(f.name);
            let rank = if name.starts_with(&q) {
                0
            } else if name.contains(&q) {
                1
            } else if fold(f.key) == q {
                2
            } else if fold(f.hint).contains(&q) || fold(f.place).starts_with(&q) {
                3
            } else {
                return None;
            };
            Some((rank, f))
        })
        .collect();
    ranked.sort_by_key(|(r, f)| (*r, matches!(f.pick, Pick::Later(_))));
    ranked.into_iter().map(|(_, f)| f).collect()
}

impl ArchiSimApp {
    /// Whether a tile is the tool in use (`None`: not there yet).
    fn picked(&self, pick: Pick) -> Option<bool> {
        let level_marks = self.dim_settings.mode == DimMode::LevelMark;
        let holes = self.slab_settings.hole;
        match pick {
            Pick::Tool(Tool::Dimension) => Some(self.tool == Tool::Dimension && !level_marks),
            Pick::Tool(Tool::Slab) => Some(self.tool == Tool::Slab && !holes),
            Pick::Tool(t) => Some(self.tool == t),
            Pick::LevelMark => Some(self.tool == Tool::Dimension && level_marks),
            Pick::SlabHole => Some(self.tool == Tool::Slab && holes),
            Pick::Action(a) => Some(match a {
                Action::Cte => self.cte.open,
                Action::Schedule | Action::Takeoff => self.schedule_open.is_some(),
                Action::Versions => {
                    self.coord_window.open && self.coord_window.tab == CoordTab::Links
                }
                Action::Clashes => {
                    self.coord_window.open && self.coord_window.tab == CoordTab::Clashes
                }
                Action::Issues => {
                    self.coord_window.open && self.coord_window.tab == CoordTab::Issues
                }
                Action::Requirements => {
                    self.coord_window.open && self.coord_window.tab == CoordTab::Requirements
                }
                Action::Elevations | Action::NewSheet | Action::Link => false,
            }),
            Pick::Command(name) => {
                Some(self.tool == Tool::Cad && self.cad.as_ref().is_some_and(|c| c.name() == name))
            }
            Pick::Later(_) => None,
        }
    }

    /// Takes the tool of a tile, of its key or of the search. A tool taken
    /// from outside the discipline shown brings its own discipline.
    pub(super) fn pick(&mut self, pick: Pick) {
        if !self.discipline.offers(pick) {
            if let Some(d) = Discipline::home(pick) {
                self.discipline = d;
            }
        }
        match pick {
            Pick::Tool(Tool::Dimension) => {
                if self.dim_settings.mode == DimMode::LevelMark {
                    self.dim_settings.mode = DimMode::Linear;
                    self.dim_tool.finish();
                }
                self.set_tool(Tool::Dimension);
            }
            Pick::Tool(Tool::Slab) => {
                self.slab_settings.hole = false;
                self.set_tool(Tool::Slab);
            }
            Pick::Tool(t) => self.set_tool(t),
            Pick::LevelMark => {
                if self.dim_settings.mode != DimMode::LevelMark {
                    self.dim_settings.mode = DimMode::LevelMark;
                    self.dim_tool.finish();
                }
                self.set_tool(Tool::Dimension);
            }
            Pick::SlabHole => {
                self.slab_settings.hole = true;
                self.set_tool(Tool::Slab);
            }
            Pick::Action(a) => self.act(a),
            Pick::Command(name) => self.start_command(name),
            Pick::Later(what) => self.say(format!("Esta herramienta llegará con {what}.")),
        }
    }

    fn act(&mut self, action: Action) {
        let coord_tab = |app: &mut Self, tab| {
            app.coord_window.open = true;
            app.coord_window.tab = tab;
        };
        match action {
            Action::Elevations => self.create_elevations(),
            Action::NewSheet => self.new_sheet(),
            Action::Schedule => {
                let kind = self
                    .schedule_open
                    .map_or(archisim_core::ScheduleKind::Openings, |o| o.0);
                self.schedule_open = Some((kind, None));
            }
            Action::Takeoff => {
                self.schedule_open = Some((archisim_core::ScheduleKind::Walls, None));
                self.say("Mediciones por tipo; el BC3 de todo el proyecto, en Archivo → Exportar mediciones.");
            }
            Action::Cte => self.cte.open = true,
            Action::Link => self.link_ifc_dialog(),
            Action::Versions => coord_tab(self, CoordTab::Links),
            Action::Clashes => {
                coord_tab(self, CoordTab::Clashes);
                self.run_clash_check();
            }
            Action::Issues => coord_tab(self, CoordTab::Issues),
            Action::Requirements => {
                coord_tab(self, CoordTab::Requirements);
                self.requirements_dialog();
            }
        }
    }

    /// The rail of disciplines, on the far left: the arrow on top, always
    /// there, then the disciplines.
    pub(super) fn discipline_rail(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let arrow = self.tool == Tool::Arrow;
            let r = rail_button(ui, arrow, |painter, rect, color| {
                icons::paint(painter, rect, Icon::Arrow, color)
            });
            if r.on_hover_text("Flecha (Esc) — seleccionar, mover, copiar y estirar")
                .clicked()
            {
                self.set_tool(Tool::Arrow);
            }
            let (line, _) = ui.allocate_exact_size(Vec2::new(28.0, 9.0), Sense::hover());
            ui.painter()
                .hline(line.x_range(), line.center().y, Stroke::new(1.0, p.line));
            for d in Discipline::ALL {
                let on = self.discipline == d;
                let r = rail_button(ui, on, |painter, rect, color| {
                    glyphs::paint(painter, rect, d.glyph(), color)
                });
                if r.on_hover_text(format!("{} — {}", d.name(), d.purpose()))
                    .clicked()
                {
                    self.discipline = d;
                }
            }
        });
    }

    /// The column beside the rail: the tiles of the discipline, always the
    /// same size, and Properties below.
    pub(super) fn tool_column(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(self.discipline.name())
                    .font(strong_font(12.5))
                    .color(p.text),
            );
        });
        ui.add_space(6.0);
        let full = ui.available_width();
        let w = (full - TILE_GAP * (TILE_COLUMNS as f32 - 1.0)) / TILE_COLUMNS as f32;
        let area = Vec2::new(
            full,
            TILE_ROWS as f32 * TILE_HEIGHT + (TILE_ROWS as f32 - 1.0) * TILE_GAP,
        );
        let (block, _) = ui.allocate_exact_size(area, Sense::hover());
        for (i, t) in self.discipline.tiles().iter().enumerate() {
            let (row, col) = (i / TILE_COLUMNS, i % TILE_COLUMNS);
            let min = block.min
                + Vec2::new(
                    col as f32 * (w + TILE_GAP),
                    row as f32 * (TILE_HEIGHT + TILE_GAP),
                );
            let rect = Rect::from_min_size(min, Vec2::new(w, TILE_HEIGHT));
            let active = self.picked(t.pick);
            let resp = glyph_tile(ui, rect, t, active);
            let hint = match t.pick {
                Pick::Later(what) => format!("{} — {} (llegará con {what})", t.name, t.hint),
                _ if t.key.is_empty() => format!("{} — {}", t.name, t.hint),
                _ => format!("{} ({}) — {}", t.name, t.key, t.hint),
            };
            if resp.on_hover_text(hint).clicked() {
                self.pick(t.pick);
            }
        }
        ui.add_space(8.0);
        if self.show_properties {
            let (line, _) = ui.allocate_exact_size(Vec2::new(full, 1.0), Sense::hover());
            ui.painter()
                .hline(line.x_range(), line.center().y, Stroke::new(1.0, p.line));
            egui::ScrollArea::vertical()
                .id_salt("properties")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.properties(ui);
                    ui.add_space(12.0);
                });
        }
    }
}

/// A square button of the rail.
fn rail_button(
    ui: &mut Ui,
    on: bool,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
    let fill = if on {
        p.accent_soft
    } else if resp.hovered() {
        p.raised
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(9), fill);
    if on {
        // A mark on the outer edge, as the active tab of a dock.
        let bar = Rect::from_min_max(
            rect.left_top() + Vec2::new(-5.0, 9.0),
            rect.left_bottom() + Vec2::new(-3.0, -9.0),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::same(1), p.accent);
    }
    let color = if on { p.accent } else { p.text };
    draw(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(22.0)),
        color,
    );
    resp
}

/// A tile: the drawing and the name below it. No border at rest, a soft
/// fill under the mouse, accent border and fill when it is the tool in use.
fn glyph_tile(ui: &mut Ui, rect: Rect, t: &Tile, active: Option<bool>) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let sense = if active.is_some() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let resp = ui.interact(rect, ui.scope_id().with(("tile", t.name, t.glyph)), sense);
    let painter = ui.painter_at(rect.expand(1.0));
    let on = active == Some(true);
    let (fill, stroke, fg) = match active {
        Some(true) => (p.accent_soft, Stroke::new(1.5, p.accent), p.accent),
        Some(false) if resp.hovered() => (p.raised, Stroke::NONE, p.text),
        Some(false) => (Color32::TRANSPARENT, Stroke::NONE, p.text),
        None => (Color32::TRANSPARENT, Stroke::NONE, p.faint),
    };
    painter.rect(
        rect,
        CornerRadius::same(9),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    glyphs::paint(
        &painter,
        Rect::from_center_size(rect.center_top() + Vec2::new(0.0, 19.0), Vec2::splat(22.0)),
        t.glyph,
        fg,
    );
    // The name, smaller if it does not fit.
    let room = rect.width() - 4.0;
    let mut size = 10.5;
    let font = |s: f32| {
        if on {
            strong_font(s)
        } else {
            FontId::proportional(s)
        }
    };
    while size > 8.5 {
        let g = painter.layout_no_wrap(t.name.to_string(), font(size), fg);
        if g.size().x <= room {
            break;
        }
        size -= 0.5;
    }
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 10.0),
        Align2::CENTER_CENTER,
        t.name,
        font(size),
        fg,
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disciplines_fit_their_tiles() {
        for d in Discipline::ALL {
            assert!(
                d.tiles().len() <= TILE_ROWS * TILE_COLUMNS,
                "{} has too many tiles",
                d.name()
            );
            for t in d.tiles() {
                assert!(!t.name.is_empty() && !t.hint.is_empty());
                if let Pick::Tool(tool) = t.pick {
                    assert_eq!(Tool::from_key(t.key), Some(tool), "{}", t.name);
                }
            }
        }
    }

    #[test]
    fn tools_have_a_home() {
        assert_eq!(
            Discipline::home(Pick::Tool(Tool::Wall)),
            Some(Discipline::Architecture)
        );
        assert_eq!(
            Discipline::home(Pick::Tool(Tool::Column)),
            Some(Discipline::Structure)
        );
        assert_eq!(
            Discipline::home(Pick::LevelMark),
            Some(Discipline::Documentation)
        );
        assert_eq!(
            Discipline::home(Pick::Tool(Tool::Terrain)),
            Some(Discipline::Site)
        );
        assert_eq!(Discipline::home(Pick::Command("LINEA")), None);
    }

    #[test]
    fn search_finds_by_name_key_and_accents() {
        let names = |q: &str| search(q).iter().map(|f| f.name).collect::<Vec<_>>();
        assert_eq!(names("viga")[0], "Viga");
        assert_eq!(names("cota de nivel")[0], "Cota de nivel");
        assert_eq!(names("zapata")[0], "Zapata");
        assert_eq!(names("girar")[0], "Girar");
        assert_eq!(names("simetria")[0], "Simetría");
        assert_eq!(names("tuberia")[0], "Tubería");
        assert!(names("EQ").contains(&"Desfase"));
        // Each tool once, in its home discipline.
        let walls = search("muro");
        assert_eq!(walls.iter().filter(|f| f.name == "Muro").count(), 1);
        assert_eq!(
            walls.iter().find(|f| f.name == "Muro").unwrap().place,
            "Arquitectura"
        );
        assert!(search("  ").is_empty());
    }
}
