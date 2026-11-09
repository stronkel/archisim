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
use super::tabs::{SchedTab, TabKind};
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
    Presentation,
    Analysis,
    Coordination,
}

/// The colours of a discipline in the window (`docs/INTERFAZ.md` §12.1):
/// the colour itself, a soft background and a darker one for text on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hue {
    pub main: Color32,
    pub soft: Color32,
    pub ink: Color32,
}

impl Discipline {
    /// In the order of the rail.
    pub const ALL: [Discipline; 9] = [
        Discipline::Concept,
        Discipline::Architecture,
        Discipline::Structure,
        Discipline::Services,
        Discipline::Site,
        Discipline::Documentation,
        Discipline::Presentation,
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
            Discipline::Presentation => "Presentación",
            Discipline::Analysis => "Análisis",
            Discipline::Coordination => "Coordinación",
        }
    }

    pub fn purpose(self) -> &'static str {
        match self {
            Discipline::Concept => "Volúmenes, masas y edificabilidad",
            Discipline::Architecture => "El edificio",
            Discipline::Structure => "Pilares, vigas, forjados y cimentación",
            Discipline::Services => "Agua, clima, electricidad e incendios",
            Discipline::Site => "Parcela, terreno, calles y paisaje",
            Discipline::Documentation => "Vistas, planos y tablas",
            Discipline::Presentation => "Imágenes y láminas para enseñar",
            Discipline::Analysis => "Comprobaciones, superficies y costes",
            Discipline::Coordination => "Vínculos, versiones y equipo",
        }
    }

    /// Its colours, for the light or the dark theme.
    pub fn hue(self, dark: bool) -> Hue {
        let (main, soft, ink) = match self {
            Discipline::Concept => ([160, 120, 74], [245, 238, 228], [107, 79, 44]),
            Discipline::Architecture => ([192, 88, 47], [251, 237, 230], [142, 59, 28]),
            Discipline::Structure => ([76, 106, 146], [233, 239, 246], [52, 80, 122]),
            Discipline::Services => ([22, 137, 122], [226, 244, 240], [15, 106, 94]),
            Discipline::Site => ([79, 138, 43], [234, 243, 224], [58, 107, 28]),
            Discipline::Documentation => ([110, 107, 100], [239, 237, 232], [69, 67, 62]),
            Discipline::Presentation => ([184, 69, 122], [251, 234, 242], [143, 47, 92]),
            Discipline::Analysis => ([184, 117, 20], [251, 240, 220], [131, 80, 10]),
            Discipline::Coordination => ([110, 85, 194], [239, 235, 251], [84, 64, 160]),
        };
        let rgb = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
        if dark {
            // Over the dark panels: the colour lighter for text, and a
            // quarter of it over the panel for the background.
            let mix = |a: [u8; 3], b: [u8; 3], t: f32| {
                let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
                Color32::from_rgb(m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2]))
            };
            Hue {
                main: mix(main, [255, 255, 255], 0.15),
                soft: mix([29, 32, 37], main, 0.28),
                ink: mix(main, [255, 255, 255], 0.45),
            }
        } else {
            Hue {
                main: rgb(main),
                soft: rgb(soft),
                ink: rgb(ink),
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
            Discipline::Documentation => Glyph::Sheet,
            Discipline::Presentation => Glyph::Present,
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
            Discipline::Presentation => PRESENTATION,
            Discipline::Analysis => ANALYSIS,
            Discipline::Coordination => COORDINATION,
        }
    }

    /// Whether one of its tiles, or one of their variants, does `pick`.
    pub fn offers(self, pick: Pick) -> bool {
        self.tiles().iter().any(|t| t.does(pick))
    }

    /// The discipline a pick belongs to when it is taken from elsewhere
    /// (its key, the search): the first that offers it, the building first.
    pub fn home(pick: Pick) -> Option<Discipline> {
        const BY_PRIORITY: [Discipline; 9] = [
            Discipline::Architecture,
            Discipline::Structure,
            Discipline::Documentation,
            Discipline::Site,
            Discipline::Presentation,
            Discipline::Analysis,
            Discipline::Coordination,
            Discipline::Concept,
            Discipline::Services,
        ];
        BY_PRIORITY.into_iter().find(|d| d.offers(pick))
    }
}

/// Rows of tiles kept for every discipline, so that Properties below does
/// not move when the discipline changes: twelve tiles at most (§12.2),
/// the variants of a tool in one tile.
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
    /// The slab tool with the raft foundation type.
    Raft,
    /// The slab tool drawing false ceilings (M7).
    Ceiling,
    /// Clicks that arm columns, beams, footings and slabs.
    Rebar,
    /// An action, not a tool: it opens or makes something.
    Action(Action),
    /// A 2D drafting or editing command (AutoCAD's), by its name.
    Command(&'static str),
    /// A tool of the Concepto discipline.
    Concept(super::concept_ui::ConceptPick),
    /// The device tool with a kind of device.
    Device(archisim_core::DeviceKind),
    /// The pipe tool for the networks under the street (U5).
    Network,
    /// The mass tool by floors with a style: a building of the
    /// surroundings (U6).
    EnvBuilding,
    /// The object tool with only part of the library (vegetation, street
    /// furniture).
    Objects(super::object_gallery::ObjectScope),
    /// Not there yet: what brings it.
    Later(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The four elevations around the building.
    Elevations,
    /// The selected volumes into a building.
    Convert,
    NewSheet,
    Schedule,
    Takeoff,
    /// The budget, at the top of the takeoff.
    Budget,
    /// The table of room areas.
    Areas,
    Cte,
    Link,
    Versions,
    Clashes,
    Issues,
    Requirements,
    /// The vivero: species of plants with their sliders and 3D.
    Vivero,
    /// A new drafting view.
    DraftingView,
    /// Bring the surroundings from downloaded files.
    BringSurroundings,
    /// Ask for the surroundings to the free services.
    DownloadSurroundings,
    /// The sun of the project, in its panel.
    Sun,
    /// Saves the point of view of the 3D as a camera.
    SaveCamera,
    /// Walks through the house at eye height.
    Walk,
    /// The presentation styles.
    Styles,
    /// A realistic picture of the 3D view.
    Render,
    /// The 3D view as a picture.
    Picture,
}

pub struct Tile {
    pub pick: Pick,
    pub glyph: Glyph,
    pub name: &'static str,
    pub key: &'static str,
    pub hint: &'static str,
    /// The variants of a tile that gathers several (its own pick is the
    /// first's); empty for a single tool.
    pub variants: &'static [Tile],
}

impl Tile {
    /// Whether the tile or one of its variants does `pick`.
    pub fn does(&self, pick: Pick) -> bool {
        self.pick == pick || self.variants.iter().any(|v| v.pick == pick)
    }

    /// The tile and its variants (the tile alone if it has none).
    pub fn all(&self) -> impl Iterator<Item = &Tile> {
        std::iter::once(self)
            .filter(|t| t.variants.is_empty())
            .chain(self.variants.iter())
    }
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
        variants: &[],
    }
}

/// A tile that gathers `variants`: it takes the first's tool and key.
const fn group(
    glyph: Glyph,
    name: &'static str,
    hint: &'static str,
    variants: &'static [Tile],
) -> Tile {
    Tile {
        pick: variants[0].pick,
        glyph,
        name,
        key: variants[0].key,
        hint,
        variants,
    }
}

const TEAM_LATER: &str = "el trabajo en equipo (4.6, carpeta compartida en la nube)";

const CONCEPT: &[Tile] = &[
    tile(
        Pick::Concept(super::concept_ui::ConceptPick::Line),
        Glyph::Edge,
        "Arista",
        "",
        "Líneas del modelo (no del dibujo): quedan abiertas, y al cerrar un contorno se hace una cara. Rectángulo y círculo, en la barra superior",
    ),
    group(
        Glyph::PushPull,
        "Empujar",
        "Empujar o tirar de una cara, o llevar un perfil a lo largo de un camino",
        &[
            tile(
                Pick::Concept(super::concept_ui::ConceptPick::Push),
                Glyph::PushPull,
                "Empujar",
                "",
                "Empujar o tirar de una cara para hacer un volumen",
            ),
            tile(
                Pick::Concept(super::concept_ui::ConceptPick::Follow),
                Glyph::FollowMe,
                "Seguir",
                "",
                "Llevar un perfil a lo largo de un camino",
            ),
        ],
    ),
    tile(
        Pick::Concept(super::concept_ui::ConceptPick::Offset),
        Glyph::FaceOffset,
        "Equidistancia",
        "",
        "Contorno paralelo dentro o fuera de una cara",
    ),
    group(
        Glyph::DivideFace,
        "Dividir",
        "Dividir una cara o deformar un volumen",
        &[
            tile(
                Pick::Concept(super::concept_ui::ConceptPick::Divide),
                Glyph::DivideFace,
                "Dividir cara",
                "",
                "Dividir una cara de un volumen con una línea, para pintar o empujar cada parte",
            ),
            tile(
                Pick::Concept(super::concept_ui::ConceptPick::Deform),
                Glyph::Deform,
                "Deformar",
                "",
                "Retorcer un volumen alrededor de su centro o doblarlo hacia un lado (con Mayús)",
            ),
        ],
    ),
    group(
        Glyph::Volume,
        "Volumen",
        "Prismas por su contorno y altura, y formas: esfera, cilindro, cono y revolución",
        &[
            tile(
                Pick::Concept(super::concept_ui::ConceptPick::Volume),
                Glyph::Volume,
                "Volumen",
                "",
                "Prisma por su contorno y altura",
            ),
            tile(
                Pick::Concept(super::concept_ui::ConceptPick::Form),
                Glyph::Sphere,
                "Formas",
                "",
                "Esfera, cilindro, cono y revolución de un perfil (en la barra superior). Para unir, restar o intersecar volúmenes, selecciona varios",
            ),
        ],
    ),
    tile(
        Pick::Concept(super::concept_ui::ConceptPick::Mass),
        Glyph::Mass,
        "Masa",
        "",
        "Volumen por plantas con superficie construida, edificabilidad y ocupación en directo",
    ),
    tile(
        Pick::Concept(super::concept_ui::ConceptPick::Paint),
        Glyph::Paint,
        "Pintar",
        "",
        "Dar un material a una cara",
    ),
    tile(
        Pick::Action(Action::Convert),
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
        Pick::Ceiling,
        Glyph::Ceiling,
        "Techo",
        "",
        "Falsos techos por contorno o dentro de un local (varita); se ven en la planta de techos",
    ),
    tile(
        Pick::Tool(Tool::Roof),
        Glyph::Roof,
        "Cubierta",
        "T",
        "Cubiertas inclinadas por faldones: a cuatro, dos o un agua, con alero",
    ),
    group(
        Glyph::Stair,
        "Escalera",
        "Escaleras y ascensores",
        &[
            tile(
                Pick::Tool(Tool::Stair),
                Glyph::Stair,
                "Escalera",
                "E",
                "Escaleras por su línea de huella: tramos, mesetas y comprobación del CTE",
            ),
            tile(
                Pick::Tool(Tool::Elevator),
                Glyph::Stair,
                "Ascensor",
                "AS",
                "Ascensor completo: hueco con foso y huida, puertas de piso en cada planta, cabina; abre los huecos de los forjados",
            ),
        ],
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
    group(
        Glyph::Footing,
        "Cimentación",
        "Zapatas aisladas y corridas, y losas de cimentación",
        &[
            tile(
                Pick::Tool(Tool::Footing),
                Glyph::Footing,
                "Zapata",
                "A",
                "Zapatas aisladas bajo pilares y corridas bajo muros",
            ),
            tile(Pick::Raft, Glyph::Raft, "Losa de cimentación", "", "Losa de cimentación"),
        ],
    ),
    tile(
        Pick::Rebar,
        Glyph::Rebar,
        "Armado",
        "",
        "Clic en un pilar, una viga, una zapata o un forjado para armarlo; sus barras, en Propiedades",
    ),
    tile(
        Pick::SlabHole,
        Glyph::Opening,
        "Abertura",
        "",
        "Huecos en forjados: patinillos, escaleras, lucernarios",
    ),
    tile(
        Pick::Tool(Tool::Grid),
        Glyph::Grid,
        "Ejes",
        "X",
        "Ejes de replanteo con su numeración",
    ),
];

const SERVICES: &[Tile] = &[
    tile(
        Pick::Tool(Tool::Pipe),
        Glyph::Pipe,
        "Tubería",
        "H",
        "Fontanería, saneamiento, calefacción y protección contra incendios",
    ),
    tile(
        Pick::Tool(Tool::Duct),
        Glyph::Duct,
        "Conducto",
        "D",
        "Climatización y ventilación",
    ),
    group(
        Glyph::Fixture,
        "Aparato",
        "Aparatos y equipos de agua",
        &[
            tile(
                Pick::Device(archisim_core::DeviceKind::Equipment),
                Glyph::Fixture,
                "Aparato",
                "",
                "Calderas, bombas, máquinas de climatización",
            ),
            tile(
                Pick::Device(archisim_core::DeviceKind::Valve),
                Glyph::Pipe,
                "Equipos de agua",
                "",
                "Llaves de paso; en su tipo, contadores, depósitos, grupos de presión y termos",
            ),
        ],
    ),
    tile(
        Pick::Device(archisim_core::DeviceKind::Light),
        Glyph::Light,
        "Iluminación",
        "",
        "Luminarias",
    ),
    group(
        Glyph::Outlet,
        "Mecanismo",
        "Enchufes e interruptores",
        &[
            tile(
                Pick::Device(archisim_core::DeviceKind::Socket),
                Glyph::Outlet,
                "Enchufe",
                "",
                "Tomas de corriente; en su tipo, base de 25 A, tomas de TV y de datos",
            ),
            tile(
                Pick::Device(archisim_core::DeviceKind::Switch),
                Glyph::Outlet,
                "Interruptor",
                "",
                "Interruptores; en su tipo, conmutadores",
            ),
        ],
    ),
    tile(
        Pick::Device(archisim_core::DeviceKind::Panel),
        Glyph::Fixture,
        "Cuadro",
        "",
        "Cuadro general de mando y protección",
    ),
    tile(
        Pick::Device(archisim_core::DeviceKind::Riser),
        Glyph::Pipe,
        "Montante",
        "",
        "Tubería vertical entre plantas; en su tipo, bajantes de saneamiento o pluviales",
    ),
    tile(
        Pick::Device(archisim_core::DeviceKind::Radiator),
        Glyph::Duct,
        "Clima",
        "",
        "Radiadores; en su tipo, rejillas de pared y difusores de techo",
    ),
    tile(
        Pick::Device(archisim_core::DeviceKind::Extinguisher),
        Glyph::Fixture,
        "Incendios",
        "",
        "Extintores; en su tipo, rociadores, BIE, detectores, pulsadores, sirenas, alumbrado de emergencia y señales de salida",
    ),
    tile(
        Pick::Device(archisim_core::DeviceKind::SolarPanel),
        Glyph::Light,
        "Placas solares",
        "",
        "Placas fotovoltaicas en fila; en su tipo, captadores térmicos",
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
        Pick::Tool(Tool::Plot),
        Glyph::Plot,
        "Parcela",
        "Y",
        "Lindes, superficie y retranqueos",
    ),
    group(
        Glyph::Road,
        "Calle",
        "Viales, glorietas y marcas viales",
        &[
            tile(
                Pick::Tool(Tool::Road),
                Glyph::Road,
                "Vial",
                "W",
                "Viales por su eje y una sección tipo: calzada, aparcamiento, bordillo y acera",
            ),
            tile(
                Pick::Tool(Tool::Roundabout),
                Glyph::Roundabout,
                "Glorieta",
                "GL",
                "Glorieta o plaza donde se encuentran las calles: clic en el cruce; las calles se recortan en su círculo",
            ),
            tile(
                Pick::Tool(Tool::Marking),
                Glyph::Marking,
                "Marca vial",
                "MV",
                "Pasos de peatones, líneas, flechas, STOP, cebreados y plazas pintadas (Norma 8.2-IC)",
            ),
        ],
    ),
    group(
        Glyph::Network,
        "Redes",
        "Redes bajo la calle, pozos y arquetas",
        &[
            tile(
                Pick::Network,
                Glyph::Network,
                "Redes",
                "",
                "Redes bajo la calle: saneamiento, pluviales, abastecimiento, baja y media tensión, alumbrado, telecomunicaciones y gas; pozos, acometidas y perfil longitudinal",
            ),
            tile(
                Pick::Device(archisim_core::DeviceKind::Manhole),
                Glyph::Manhole,
                "Pozo y arqueta",
                "",
                "Pozos de registro, arquetas, imbornales, hidrantes, farolas, centros de transformación y cajas generales de protección",
            ),
        ],
    ),
    group(
        Glyph::EnvBuilding,
        "Entorno",
        "Edificios del entorno con estilo, y el entorno traído de la cartografía gratuita",
        &[
            tile(
                Pick::Tool(Tool::World),
                Glyph::EnvBuilding,
                "Edificio del mundo",
                "EM",
                "Galería de edificios de cada lugar en 3D: viviendas, equipamientos, comercio, plazas, parques, estaciones… Un clic los pone con sus medidas habituales, con la fachada a la calle más cercana",
            ),
            tile(
                Pick::Tool(Tool::Road),
                Glyph::Road,
                "Calle",
                "W",
                "Calles con sus aceras y cruces: clics por su eje, doble clic para acabar",
            ),
            tile(
                Pick::EnvBuilding,
                Glyph::EnvBuilding,
                "Edificio del entorno",
                "",
                "Edificios del entorno con estilo: casa andaluza, ensanche, años 60, oficinas, nave… Clic en una parcela o dibuja su contorno",
            ),
            tile(
                Pick::Action(Action::BringSurroundings),
                Glyph::Plot,
                "Traer el entorno",
                "",
                "Edificios y parcelas del Catastro, calles y árboles de OpenStreetMap y el terreno del CNIG, de ficheros descargados",
            ),
            tile(
                Pick::Action(Action::DownloadSurroundings),
                Glyph::Deliver,
                "Descargar el entorno",
                "",
                "Lo mismo, pedido directamente a sus servicios gratuitos para el emplazamiento del proyecto y un radio",
            ),
        ],
    ),
    tile(
        Pick::Tool(Tool::Pool),
        Glyph::Pool,
        "Piscina",
        "PS",
        "Piscina construida por el borde del agua: vaso con pendiente, coronación y lámina de agua; recorta el terreno",
    ),
    group(
        Glyph::Tree,
        "Vegetación",
        "Árboles y plantas sueltos o en plantaciones, rocas y el vivero de especies",
        &[
            tile(
                Pick::Objects(super::object_gallery::ObjectScope::Vegetation),
                Glyph::Tree,
                "Vegetación",
                "",
                "Árboles, palmeras, arbustos y matas que crecen cada uno distinto (y personas y vehículos)",
            ),
            tile(
                Pick::Tool(Tool::Planting),
                Glyph::Planting,
                "Plantación",
                "J",
                "Plantas a lo largo de una línea o spline, o dentro de un contorno: cuántas, más o menos regulares y variadas",
            ),
            tile(
                Pick::Objects(super::object_gallery::ObjectScope::Nature),
                Glyph::Rock,
                "Rocas",
                "",
                "Rocas, cantos rodados, lajas, grupos de rocas, troncos caídos y tocones",
            ),
            tile(
                Pick::Action(Action::Vivero),
                Glyph::Nursery,
                "Vivero",
                "",
                "Las especies de vegetación con todos sus parámetros, en 3D y en planta: crear y ajustar especies",
            ),
        ],
    ),
    tile(
        Pick::Tool(Tool::Brush),
        Glyph::Brush,
        "Pincel",
        "PI",
        "Pinta el terreno con césped, praderas con flores, tierra, arena, grava o losas; bórralo; súbelo, bájalo, alísalo o aplánalo; esparce plantas y rocas",
    ),
    tile(
        Pick::Objects(super::object_gallery::ObjectScope::Street),
        Glyph::StreetLamp,
        "Mobiliario",
        "",
        "Farolas, bancos, papeleras, buzones, bolardos, fuentes, aparcabicis, contenedores, señales, jardineras y alcorques",
    ),
    tile(
        Pick::Concept(super::concept_ui::ConceptPick::Volume),
        Glyph::Volume,
        "Forma",
        "",
        "Volúmenes del entorno: edificios vecinos, taludes",
    ),
];

const DOCUMENTATION: &[Tile] = &[
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
        Pick::Command("ETIQUETA"),
        Glyph::Tag,
        "Etiqueta",
        "ETI",
        "Etiquetas de puertas, ventanas, muros y locales (T: todas)",
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
    tile(
        Pick::Action(Action::DraftingView),
        Glyph::Line,
        "Vista de dibujo",
        "",
        "Una vista de dibujo 2D a su escala, para detalles y esquemas, con calcos de plantas y secciones",
    ),
];

const PRESENTATION: &[Tile] = &[
    tile(
        Pick::Action(Action::SaveCamera),
        Glyph::Camera,
        "Cámara",
        "",
        "Guardar el punto de vista del 3D para volver a él, renderizarlo o ponerlo en una lámina: sale en el navegador, en Cámaras",
    ),
    tile(
        Pick::Action(Action::Sun),
        Glyph::Sun,
        "Sol",
        "",
        "La fecha, la hora y el emplazamiento que dan el sol y las sombras",
    ),
    tile(
        Pick::Action(Action::Styles),
        Glyph::LookStyle,
        "Estilo",
        "",
        "Estilos de presentación: papel, paleta, muros macizados y grueso de líneas",
    ),
    tile(
        Pick::Action(Action::Render),
        Glyph::Render,
        "Render",
        "",
        "Imagen realista de la vista 3D: luz del día, del atardecer o de noche, con sombras y reflejos",
    ),
    tile(
        Pick::Action(Action::Picture),
        Glyph::Picture,
        "Imagen",
        "",
        "La vista 3D tal como se ve, en una imagen PNG",
    ),
    tile(
        Pick::Tool(Tool::Decal),
        Glyph::Decal,
        "Pegatina",
        "PE",
        "Una imagen PNG (grafiti, cartel, rótulo, mancha…) sobre cualquier cara, para la vista 3D y el render",
    ),
    tile(
        Pick::Action(Action::Walk),
        Glyph::Walk,
        "Recorrido",
        "",
        "Pasear por la casa a la altura de los ojos: W A S D andan, el botón derecho mira, Esc sale",
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
        "Mediciones por tipo, con demoliciones; se entregan en BC3",
    ),
    tile(
        Pick::Action(Action::Budget),
        Glyph::Budget,
        "Presupuesto",
        "",
        "Precios de un banco BC3 para cada partida, con gastos generales, beneficio e IVA",
    ),
    tile(
        Pick::Action(Action::Areas),
        Glyph::Areas,
        "Superficies",
        "",
        "Cuadro de superficies: útil de cada local, construida por plantas y por usos",
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
        Pick::Tool(Tool::Module),
        Glyph::Module,
        "Módulo",
        "K",
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

/// What moved from the tiles to the bottom bar (§12.2): annotation and the
/// aids to drawing, for every discipline. The search finds them there.
pub const BAR_TOOLS: &[Tile] = &[
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
        Pick::Command("TEXTO"),
        Glyph::Text,
        "Texto",
        "TX",
        "Textos de una o varias líneas (directriz DIR, nube de revisión NR)",
    ),
    tile(
        Pick::Tool(Tool::RefPlane),
        Glyph::RefPlane,
        "Plano de referencia",
        "Q",
        "Planos de referencia: líneas para dibujar contra ellas, en todas las plantas",
    ),
    tile(
        Pick::Concept(super::concept_ui::ConceptPick::Tape),
        Glyph::Tape,
        "Cinta",
        "",
        "Medir y dejar guías",
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
        "Reflejar",
        "REFLEJAR",
        Icon::Mirror,
        "MM",
        "Reflejar eligiendo una línea como eje (DM: dibujar el eje); deja la copia",
    ),
    order(
        EDIT,
        "Simetría",
        "SIMETRIA",
        Icon::Mirror,
        "SI",
        "Simetría: dos puntos del eje y si se borra el original",
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
        "Alinear caras",
        "ALINEACARA",
        Icon::Move,
        "ALC",
        "Alinear: la línea de referencia y después la cara que se lleva a ella",
    ),
    order(
        EDIT,
        "Esquina",
        "ESQUINA",
        Icon::Trim,
        "ESQ",
        "Recortar/extender dos muros para formar esquina (TR)",
    ),
    order(
        EDIT,
        "Hasta límite",
        "LIMITE1",
        Icon::Extend,
        "LIM1",
        "Recortar/extender un muro hasta un límite",
    ),
    order(
        EDIT,
        "Varios al límite",
        "LIMITE",
        Icon::Extend,
        "LIM",
        "Recortar/extender varios muros hasta el mismo límite",
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

/// The name of an AutoCAD order for people («Sombreado» for SOMBCONT).
pub fn order_name(command: &str) -> Option<&'static str> {
    ORDERS
        .iter()
        .find(|o| matches!(o.pick, Pick::Command(c) if c == command))
        .map(|o| o.name)
        .or_else(|| {
            Discipline::ALL
                .iter()
                .flat_map(|d| d.tiles())
                .flat_map(|t| t.all())
                .find(|t| matches!(t.pick, Pick::Command(c) if c == command))
                .map(|t| t.name)
        })
}

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
        .flat_map(|d| d.tiles().iter().flat_map(|t| t.all()).map(move |t| (d, t)));
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
    for t in BAR_TOOLS {
        if all.iter().any(|f| f.name == t.name) {
            continue;
        }
        all.push(Found {
            pick: t.pick,
            name: t.name,
            key: t.key,
            hint: t.hint,
            place: "Barra inferior",
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

/// The slab type of raft foundations: the first whose name starts with
/// «Losa de cimentación».
pub(super) fn raft_type(model: &archisim_core::Model) -> Option<archisim_core::SlabTypeId> {
    model
        .slab_types()
        .find(|t| t.name.to_lowercase().starts_with("losa de cimentación"))
        .map(|t| t.id)
}

impl ArchiSimApp {
    /// Whether a tile is the tool in use (`None`: not there yet).
    fn picked(&self, pick: Pick) -> Option<bool> {
        let level_marks = self.dim_settings.mode == DimMode::LevelMark;
        let holes = self.slab_settings.hole;
        let ceiling = self.slab_settings.ceiling;
        let raft = raft_type(self.ed.model()) == Some(self.slab_settings.slab_type);
        match pick {
            Pick::Tool(Tool::Dimension) => Some(self.tool == Tool::Dimension && !level_marks),
            Pick::Tool(Tool::Slab) => Some(self.tool == Tool::Slab && !holes && !raft && !ceiling),
            Pick::Raft => Some(self.tool == Tool::Slab && !holes && raft && !ceiling),
            Pick::Ceiling => Some(self.tool == Tool::Slab && ceiling),
            Pick::Rebar => Some(self.tool == Tool::Arrow && self.rebar_pick),
            Pick::Concept(c) => Some(self.concept_picked(c)),
            Pick::Device(k) => Some(self.tool == Tool::Device && self.service_settings.device == k),
            Pick::Network => {
                Some(self.tool == Tool::Pipe && self.service_settings.pipe_system.is_urban())
            }
            Pick::EnvBuilding => Some(self.env_building_picked()),
            Pick::Objects(s) => Some(self.tool == Tool::Object && self.object_scope == Some(s)),
            Pick::Tool(Tool::Object) => {
                Some(self.tool == Tool::Object && self.object_scope.is_none())
            }
            Pick::Tool(t) => Some(self.tool == t),
            Pick::LevelMark => Some(self.tool == Tool::Dimension && level_marks),
            Pick::SlabHole => Some(self.tool == Tool::Slab && holes && !ceiling),
            Pick::Action(a) => Some(match a {
                Action::Cte => self.cte.open,
                Action::Vivero => self.vivero.open,
                Action::Schedule => matches!(
                    self.tabs.get(self.active_tab).map(|t| t.kind),
                    Some(TabKind::Schedule(SchedTab::Kind(_)))
                ),
                Action::Takeoff => {
                    self.tabs.get(self.active_tab).map(|t| t.kind)
                        == Some(TabKind::Schedule(SchedTab::Takeoff))
                }
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
                Action::Areas => matches!(
                    self.tabs.get(self.active_tab).map(|t| t.kind),
                    Some(TabKind::Schedule(SchedTab::Kind(
                        archisim_core::ScheduleKind::Rooms
                    )))
                ),
                Action::Render => self.render.open,
                Action::Styles => self.presentation.open,
                Action::Walk => self.v3d.walk_eye.is_some(),
                Action::Elevations
                | Action::NewSheet
                | Action::Link
                | Action::Convert
                | Action::DraftingView
                | Action::BringSurroundings
                | Action::DownloadSurroundings
                | Action::Sun
                | Action::SaveCamera
                | Action::Budget
                | Action::Picture => false,
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
                self.set_discipline(d);
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
                self.slab_settings.ceiling = false;
                self.set_tool(Tool::Slab);
            }
            Pick::Ceiling => {
                let m = self.ed.model();
                let wrong = m
                    .slab_type(self.slab_settings.ceiling_type)
                    .is_none_or(|t| t.function != archisim_core::SlabFunction::Ceiling);
                let first = m.ceiling_types().next().map(|t| t.id);
                if wrong {
                    match first {
                        Some(t) => self.slab_settings.ceiling_type = t,
                        None => {
                            self.complain(
                                "El proyecto no tiene tipos de falso techo: crea uno en Tipos y materiales (Forjados → Falso techo).",
                            );
                            return;
                        }
                    }
                }
                self.slab_settings.hole = false;
                self.slab_settings.ceiling = true;
                self.set_tool(Tool::Slab);
                if !self.active_ceiling() {
                    self.say("Techo: dibuja su contorno o haz clic dentro de un local con la varita. Se ve en la planta de techos.");
                }
            }
            Pick::Tool(Tool::Object) => {
                self.object_scope = None;
                self.set_tool(Tool::Object);
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
                self.slab_settings.ceiling = false;
                self.slab_settings.hole = true;
                self.set_tool(Tool::Slab);
            }
            Pick::Raft => match raft_type(self.ed.model()) {
                Some(t) => {
                    self.slab_settings.hole = false;
                    self.slab_settings.ceiling = false;
                    self.slab_settings.slab_type = t;
                    self.set_tool(Tool::Slab);
                    self.say("Losa de cimentación: dibuja su contorno (o varita dentro de los muros).");
                }
                None => self.complain(
                    "El proyecto no tiene un tipo de losa de cimentación: crea un tipo de forjado «Losa de cimentación…» en Tipos y materiales.",
                ),
            },
            Pick::Rebar => self.start_rebar_pick(),
            Pick::Action(a) => self.act(a),
            Pick::Concept(c) => self.concept_pick(c),
            Pick::Device(k) => self.device_pick(k),
            Pick::Network => self.network_pick(),
            Pick::EnvBuilding => self.env_building_pick(),
            Pick::Objects(scope) => self.objects_pick(scope),
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
            Action::Convert => {
                let ids = self.selected_masses();
                self.convert_masses(&ids);
            }
            Action::NewSheet => self.new_sheet(),
            Action::Schedule => {
                self.open_view(TabKind::Schedule(SchedTab::Kind(
                    archisim_core::ScheduleKind::Openings,
                )));
            }
            Action::Takeoff => self.open_view(TabKind::Schedule(SchedTab::Takeoff)),
            Action::Budget => {
                self.open_view(TabKind::Schedule(SchedTab::Takeoff));
                self.say("El presupuesto está al principio de Mediciones: elige un banco BC3 y da precio a cada partida.");
            }
            Action::Areas => self.open_view(TabKind::Schedule(SchedTab::Kind(
                archisim_core::ScheduleKind::Rooms,
            ))),
            Action::DraftingView => self.new_drafting_view(),
            Action::BringSurroundings => self.pick_carto(),
            Action::DownloadSurroundings => self.open_carto_download(),
            Action::Sun => {
                self.presentation.sun_open = true;
            }
            Action::SaveCamera => self.save_camera(),
            Action::Walk => self.toggle_walk(),
            Action::Styles => self.presentation.open = true,
            Action::Render => self.render.open = true,
            Action::Picture => {
                if self.mode == super::view3d_ui::ViewMode::ThreeD {
                    self.export_3d_picture();
                } else {
                    self.say("Abre la vista 3D y vuelve a pulsar Imagen: guarda lo que se ve.");
                }
            }
            Action::Cte => self.cte.open = true,
            Action::Vivero => self.open_vivero(None, Default::default()),
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

    /// Whether a tile, or one of its variants, is the tool in use.
    fn tile_picked(&self, t: &Tile) -> Option<bool> {
        if t.variants.is_empty() {
            self.picked(t.pick)
        } else {
            Some(t.variants.iter().any(|v| self.picked(v.pick) == Some(true)))
        }
    }

    /// The rail of disciplines, on the far left: the arrow on top, always
    /// there, then the disciplines, each with its colour.
    pub(super) fn discipline_rail(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let dark = ui.visuals().dark_mode;
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let arrow = self.tool == Tool::Arrow;
            let blue = Hue {
                main: p.accent,
                soft: p.accent_soft,
                ink: p.accent,
            };
            let r = rail_button(ui, arrow, blue, p.text, |painter, rect, color| {
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
                let hue = d.hue(dark);
                let r = rail_button(ui, on, hue, hue.ink, |painter, rect, color| {
                    glyphs::paint(painter, rect, d.glyph(), color)
                });
                super::missions_ui::mark(ui.ctx(), format!("rail:{}", d.name()), r.rect);
                if r.on_hover_text(format!("{} — {}", d.name(), d.purpose()))
                    .clicked()
                {
                    self.tile_group = None;
                    self.set_discipline(d);
                }
            }
        });
    }

    /// The column beside the rail: the tiles of the discipline, always the
    /// same size, and Properties below. A tile with variants opens them
    /// over what is below, without moving anything.
    pub(super) fn tool_column(&mut self, ui: &mut Ui) {
        let p = Palette::current(ui.ctx());
        let hue = self.discipline.hue(ui.visuals().dark_mode);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
            ui.painter()
                .rect_filled(dot, CornerRadius::same(3), hue.main);
            ui.label(
                egui::RichText::new(self.discipline.name())
                    .font(strong_font(12.5))
                    .color(hue.ink),
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new(self.discipline.purpose())
                        .small()
                        .color(p.muted),
                )
                .truncate(),
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
        let mut open_below: Option<(Rect, &'static Tile)> = None;
        for (i, t) in self.discipline.tiles().iter().enumerate() {
            let (row, col) = (i / TILE_COLUMNS, i % TILE_COLUMNS);
            let min = block.min
                + Vec2::new(
                    col as f32 * (w + TILE_GAP),
                    row as f32 * (TILE_HEIGHT + TILE_GAP),
                );
            let rect = Rect::from_min_size(min, Vec2::new(w, TILE_HEIGHT));
            let active = self.tile_picked(t);
            let open = self.tile_group == Some(t.name) && !t.variants.is_empty();
            let resp = glyph_tile(ui, rect, t, active, hue, open);
            super::missions_ui::mark(ui.ctx(), format!("tile:{}", t.name), rect);
            if open {
                open_below = Some((rect, t));
            }
            let hint = match t.pick {
                Pick::Later(what) => format!("{} — {} (llegará con {what})", t.name, t.hint),
                _ if !t.variants.is_empty() => format!(
                    "{} — {}. Variantes: {}",
                    t.name,
                    t.hint,
                    t.variants
                        .iter()
                        .map(|v| v.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                _ if t.key.is_empty() => format!("{} — {}", t.name, t.hint),
                _ => format!("{} ({}) — {}", t.name, t.key, t.hint),
            };
            if resp.on_hover_text(hint).clicked() {
                if t.variants.is_empty() {
                    self.tile_group = None;
                    self.pick(t.pick);
                } else {
                    // Keeps the variant in use, or takes the first; the
                    // variants open below (a second click closes them).
                    if active != Some(true) {
                        self.pick(t.pick);
                    }
                    self.tile_group = if open { None } else { Some(t.name) };
                }
            }
        }
        if let Some((rect, t)) = open_below {
            self.tile_variants(ui, rect, block, t, hue);
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

    /// The variants of a tile, in a strip over what is below it.
    fn tile_variants(&mut self, ui: &mut Ui, tile: Rect, block: Rect, t: &'static Tile, hue: Hue) {
        let p = Palette::current(ui.ctx());
        let area = egui::Area::new(egui::Id::unique(("tile-variants", t.name)))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(block.left(), tile.bottom() + 4.0));
        let mut chosen = None;
        let shown = area.show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(p.panel)
                .stroke(Stroke::new(1.0, hue.main))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(6))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 14,
                    spread: 0,
                    color: Color32::from_black_alpha(36),
                })
                .show(ui, |ui| {
                    ui.set_width(block.width() - 12.0);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for v in t.variants {
                        let on = self.picked(v.pick) == Some(true);
                        let (r, resp) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 30.0),
                            Sense::click(),
                        );
                        let fill = if on {
                            hue.soft
                        } else if resp.hovered() {
                            p.raised
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(r, CornerRadius::same(7), fill);
                        glyphs::paint(
                            ui.painter(),
                            Rect::from_center_size(
                                egui::pos2(r.left() + 16.0, r.center().y),
                                Vec2::splat(18.0),
                            ),
                            v.glyph,
                            hue.ink,
                        );
                        ui.painter().text(
                            egui::pos2(r.left() + 32.0, r.center().y),
                            Align2::LEFT_CENTER,
                            v.name,
                            if on {
                                strong_font(12.0)
                            } else {
                                FontId::proportional(12.0)
                            },
                            if on { hue.ink } else { p.text },
                        );
                        if !v.key.is_empty() {
                            ui.painter().text(
                                egui::pos2(r.right() - 8.0, r.center().y),
                                Align2::RIGHT_CENTER,
                                v.key,
                                FontId::monospace(10.5),
                                p.muted,
                            );
                        }
                        if resp.on_hover_text(v.hint).clicked() {
                            chosen = Some(v.pick);
                        }
                    }
                });
        });
        if let Some(pick) = chosen {
            self.tile_group = None;
            self.pick(pick);
        } else if shown.response.clicked_elsewhere()
            && !ui.rect_contains_pointer(tile)
            && ui.input(|i| i.pointer.any_click())
        {
            self.tile_group = None;
        }
    }
}

/// A square button of the rail: in the discipline's colour.
pub(super) fn rail_button(
    ui: &mut Ui,
    on: bool,
    hue: Hue,
    rest: Color32,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
    let fill = if on {
        hue.soft
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
            .rect_filled(bar, CornerRadius::same(1), hue.main);
    }
    let color = if on { hue.ink } else { rest };
    draw(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(22.0)),
        color,
    );
    resp
}

/// A tile: the drawing and the name below it, in the colour of the
/// discipline. No border at rest, a soft fill under the mouse, the colour's
/// border and fill when it is the tool in use; a corner triangle when it
/// has variants.
fn glyph_tile(
    ui: &mut Ui,
    rect: Rect,
    t: &Tile,
    active: Option<bool>,
    hue: Hue,
    open: bool,
) -> egui::Response {
    let p = Palette::current(ui.ctx());
    let sense = if active.is_some() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let resp = ui.interact(rect, ui.scope_id().with(("tile", t.name, t.glyph)), sense);
    let painter = ui.painter_at(rect.expand(1.0));
    let on = active == Some(true);
    let (fill, stroke, glyph_fg, text_fg) = match active {
        Some(true) => (hue.soft, Stroke::new(1.5, hue.main), hue.ink, hue.ink),
        Some(false) if resp.hovered() || open => (p.raised, Stroke::NONE, hue.ink, p.text),
        Some(false) => (Color32::TRANSPARENT, Stroke::NONE, hue.ink, p.text),
        None => (Color32::TRANSPARENT, Stroke::NONE, p.faint, p.faint),
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
        glyph_fg,
    );
    if !t.variants.is_empty() {
        // In the top right corner, clear of the name.
        let c = rect.right_top() + Vec2::new(-5.0, 5.0);
        painter.add(egui::Shape::convex_polygon(
            vec![c, c + Vec2::new(0.0, 7.0), c + Vec2::new(-7.0, 0.0)],
            if on { hue.ink } else { hue.main },
            Stroke::NONE,
        ));
    }
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
        let g = painter.layout_no_wrap(t.name.to_string(), font(size), text_fg);
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
        text_fg,
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
            for t in d.tiles().iter().flat_map(|t| t.all()) {
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
        // Annotation is in the bottom bar, for every discipline.
        assert_eq!(Discipline::home(Pick::LevelMark), None);
        assert_eq!(
            Discipline::home(Pick::Tool(Tool::Elevator)),
            Some(Discipline::Architecture)
        );
        assert_eq!(
            Discipline::home(Pick::Tool(Tool::Decal)),
            Some(Discipline::Presentation)
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
        assert_eq!(names("glorieta")[0], "Glorieta");
        assert_eq!(names("cota de nivel")[0], "Cota de nivel");
        assert_eq!(names("zapata")[0], "Zapata");
        assert_eq!(names("girar")[0], "Girar");
        assert_eq!(names("simetria")[0], "Simetría");
        assert_eq!(names("reflejar")[0], "Reflejar");
        assert!(names("MM").contains(&"Reflejar"));
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
