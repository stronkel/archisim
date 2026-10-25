//! Furniture, sanitary ware, kitchen units, people and cars made by
//! ArchiSim from their own measurements (as the plants of
//! `archisim-flora`): a detailed 3D model for each object type of the
//! library, without downloading anything and without anybody's licence.
//! Change a sofa's length or number of places and its model is made again.
//!
//! Each model is made of rounded boxes, cylinders, solids of revolution and
//! the like (see [`build`]), each piece in a *role* (`mat`, `frame`,
//! `cushion`, `glass`…) that the family says which material takes. Several
//! models of each object: `style` 1, 2, 3.
//!
//! No dependencies. Described for Antonio in `docs/BIBLIOTECA_3D.md`.

mod accessories;
pub mod build;
mod cozy;
mod extras;
pub mod fine;
pub mod fronts;
mod furniture;
mod garden;
mod home;
mod kitchen;
mod people;
mod sanitary;
mod street;
mod vehicles;

pub use kitchen::{sink_hole, SINK_UNIT, UPSTAND, WORKTOP};

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

/// Measurements and choices of an object type (its family's values).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Params(BTreeMap<String, f64>);

impl Params {
    pub fn set(&mut self, name: &str, v: f64) {
        self.0.insert(name.to_string(), v);
    }

    pub fn get(&self, name: &str, default: f64) -> f64 {
        self.0
            .get(name)
            .copied()
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    }

    pub fn flag(&self, name: &str, default: bool) -> bool {
        self.0.get(name).map_or(default, |v| *v > 0.5)
    }

    /// Which model of the object (1, 2, 3).
    pub fn style(&self) -> i32 {
        (self.get("style", 1.0).round() as i32).max(1)
    }

    /// A key for the cache: every value, to a tenth of a millimetre.
    fn key(&self) -> Vec<(String, i64)> {
        self.0
            .iter()
            .map(|(k, v)| (k.clone(), (v * 10_000.0).round() as i64))
            .collect()
    }
}

/// A model: its points (metres, in the family's axes) and its triangles
/// by role.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PropMesh {
    pub positions: Vec<[f32; 3]>,
    pub parts: Vec<(&'static str, Vec<[u32; 3]>)>,
    /// The roles that are stuffed fabric (the workshop's cushions and
    /// upholstery): drawn soft, outlined only where they end against
    /// something behind, never at their own folds and seams.
    pub soft: Vec<&'static str>,
}

impl PropMesh {
    pub fn triangle_count(&self) -> usize {
        self.parts.iter().map(|p| p.1.len()).sum()
    }
}

/// What can be made, with the name of each model (`style`).
pub const KINDS: &[(&str, &[&str])] = &[
    ("sofa", &["Contemporáneo", "Clásico", "Nórdico"]),
    ("corner_sofa", &["Contemporáneo", "Clásico", "Nórdico"]),
    ("chair", &["Madera", "Tubo metálico", "Tapizada"]),
    ("table", &["Cuatro patas", "Patas metálicas", "Caballetes"]),
    (
        "coffee_table",
        &["Madera con balda", "Bloque", "Vidrio y metal"],
    ),
    (
        "bed",
        &[
            "Tapizada",
            "Madera",
            "Tarima",
            "Cabecero tapizado",
            "Cabecero de lamas",
        ],
    ),
    (
        "nightstand",
        &["Cajonera", "Con patas", "Patas cónicas", "Flotante"],
    ),
    ("wardrobe", &["Puertas batientes"]),
    ("chest", &["Zócalo", "Con patas"]),
    ("desk", &["Con cajonera", "Patas metálicas"]),
    ("bookcase", &["Con libros", "Con objetos"]),
    ("tv_unit", &["Con televisor"]),
    ("counter", &["Con fregadero y placa"]),
    ("wall_cabinet", &["Puertas"]),
    ("base_unit", &["Módulo"]),
    ("tall_unit", &["Columna"]),
    ("fridge", &["Combi"]),
    ("washer", &["Carga frontal"]),
    ("wc", &["De pie con cisterna", "Suspendido"]),
    ("bidet", &["De pie", "Suspendido"]),
    ("washbasin", &["De pedestal", "Con mueble", "Suspendido"]),
    ("bath", &["Rectangular"]),
    (
        "shower",
        &["Plato", "Plato y mampara", "Esquina con mampara"],
    ),
    ("mirror", &["Con marco", "Sin marco", "Redondo"]),
    ("towel_rail", &["Barra", "Radiador toallero"]),
    ("toilet_roll", &["De pared"]),
    ("grab_bar", &["Fija", "Abatible"]),
    ("hood", &["Decorativa", "Integrada"]),
    ("shelf", &["Con escuadras", "Flotante"]),
    ("person", &["Hombre", "Mujer", "Caminando"]),
    (
        "rug",
        &[
            "Lisa con ribete",
            "Redonda",
            "De rayas",
            "Bereber con flecos",
        ],
    ),
    ("lamp", &["De pie", "De mesa", "Colgante", "De arco"]),
    (
        "potted_plant",
        &[
            "Costilla de Adán",
            "Ficus",
            "Cactus",
            "Helecho",
            "Sansevieria",
        ],
    ),
    (
        "armchair",
        &["De orejas", "Redonda", "De ratán", "Baja de patas"],
    ),
    ("pouf", &["Redondo", "De punto", "Cubo"]),
    ("side_table", &["Pie central", "Tres patas", "Nido"]),
    ("sideboard", &["Nórdico de patas", "Puertas de lamas"]),
    ("curtain", &["Larga", "Visillo", "Hasta el alféizar"]),
    (
        "decor",
        &[
            "Jarrón con ramas",
            "Frutero",
            "Libros y vela",
            "Cerámicas",
            "Cesta con manta",
        ],
    ),
    ("stool", &["De madera", "Tapizado con respaldo", "De enea"]),
    (
        "island",
        &["Con voladizo", "Mesa de trabajo", "Con fregadero"],
    ),
    (
        "counter_decor",
        &[
            "Tablas y aceite",
            "Botes de cristal",
            "Cafetera y tazas",
            "Hierbas aromáticas",
        ],
    ),
    ("towels", &["Dobladas", "Colgadas", "Enrolladas"]),
    ("basket", &["Cesto de ropa", "Con rollos", "Con asas"]),
    (
        "wall_decor",
        &[
            "Cuadro",
            "Tríptico",
            "Espejo de arco",
            "Espejo sol",
            "Reloj",
        ],
    ),
    ("floor_cushion", &["Cuadrado", "Zafu", "Pila"]),
    ("blanket", &["Doblada", "Pila de mantas", "En escalera"]),
    ("lounger", &["De teca", "De aluminio", "Cama de exterior"]),
    ("parasol", &["Redonda", "Descentrada", "De paja"]),
    (
        "big_pot",
        &["Con olivo", "Ánfora", "Con gramíneas", "Con lavanda"],
    ),
    ("pergola", &["De madera", "De lamas", "Con cañizo"]),
    ("car", &["Berlina", "Compacto", "Todoterreno"]),
    // Street furniture (v0.52).
    (
        "bench",
        &["De listones", "Hormigón y madera", "Marco de acero"],
    ),
    (
        "streetlight",
        &["Báculo curvo", "Fernandina", "Columna con globo", "Baliza"],
    ),
    (
        "litter_bin",
        &[
            "Chapa perforada",
            "Listones de madera",
            "Doble para reciclar",
        ],
    ),
    ("bollard", &["Fundición", "Tubo de acero", "Hormigón"]),
    (
        "letter_box",
        &["De pie", "Sobre poste", "Buzones de portal"],
    ),
    (
        "container",
        &["Carga lateral", "Iglú de vidrio", "Soterrado"],
    ),
    (
        "fountain",
        &[
            "Fuente de fundición",
            "Bloque de piedra",
            "Estanque con surtidor",
        ],
    ),
    (
        "planter",
        &[
            "Hormigón con arbustos",
            "Madera con flores",
            "Acero corten con gramíneas",
        ],
    ),
    ("tree_grate", &["Fundición", "Pletinas", "Bordillo y grava"]),
    ("bike_rack", &["U invertida", "Onda", "Soporte de rueda"]),
    ("sign", &["Circular", "Triangular", "Rectangular"]),
    ("bicycle", &["Paseo", "Carretera", "De carga"]),
    // More of what a house has (v0.52).
    (
        "fireplace",
        &["De obra", "Estufa de leña", "Moderna lineal"],
    ),
    ("coat_stand", &["De pie", "Recibidor con banco"]),
    ("office_chair", &["Giratoria", "De malla"]),
    (
        "outdoor_set",
        &["Bistró", "Teca para seis", "Sofá de exterior"],
    ),
    ("hanging_chair", &["Huevo de ratán", "Hamaca"]),
    ("radiator", &["Aluminio", "Panel de acero"]),
    ("ac_unit", &["Split de pared"]),
    ("ceiling_fan", &["Aspas de madera", "Aspas de ratán"]),
    ("piano", &["De pared", "De cola"]),
    ("kids_bed", &["Cabaña", "Litera"]),
];

/// Values of the families chosen by name, as numbers for the models:
/// the position in the list, from 1 (`handle`: the handle of the fronts;
/// `pedestal`: where a desk's drawers are).
pub const CHOICES: &[(&str, &[&str])] = &[
    ("handle", fronts::HANDLES),
    ("front_style", fronts::STYLES),
    ("unit", kitchen::UNITS),
    ("tall", kitchen::TALLS),
    (
        "pedestal",
        &["Sin cajonera", "A la izquierda", "A la derecha"],
    ),
    ("side", &["Derecha", "Izquierda"]),
];

/// The number of a value chosen by name (see [`CHOICES`]).
pub fn choice_number(key: &str, name: &str) -> Option<f64> {
    CHOICES
        .iter()
        .find(|(k, _)| *k == key)
        .and_then(|(_, names)| names.iter().position(|n| *n == name))
        .map(|i| (i + 1) as f64)
}

/// The names of the models of a kind.
pub fn styles(kind: &str) -> &'static [&'static str] {
    KINDS
        .iter()
        .chain(fine::KINDS)
        .find(|k| k.0 == kind)
        .map_or(&[], |k| k.1)
}

/// Makes a model. `None` for a kind ArchiSim does not make.
pub fn make(kind: &str, p: &Params) -> Option<PropMesh> {
    let mut b = build::Builder::default();
    match kind {
        "sofa" => furniture::sofa(&mut b, p),
        "corner_sofa" => furniture::corner_sofa(&mut b, p),
        "chair" => furniture::chair(&mut b, p),
        "table" => furniture::table(&mut b, p),
        "coffee_table" => furniture::coffee_table(&mut b, p),
        "bed" => furniture::bed(&mut b, p),
        "nightstand" => furniture::nightstand(&mut b, p),
        "wardrobe" => furniture::wardrobe(&mut b, p),
        "chest" => furniture::chest(&mut b, p),
        "desk" => furniture::desk(&mut b, p),
        "bookcase" => furniture::bookcase(&mut b, p),
        "tv_unit" => furniture::tv_unit(&mut b, p),
        "counter" => kitchen::counter(&mut b, p),
        "wall_cabinet" => kitchen::wall_cabinet(&mut b, p),
        "base_unit" => kitchen::base_unit(&mut b, p),
        "tall_unit" => kitchen::tall_unit(&mut b, p),
        "fridge" => kitchen::fridge(&mut b, p),
        "washer" => kitchen::washer(&mut b, p),
        "wc" => sanitary::wc(&mut b, p),
        "bidet" => sanitary::bidet(&mut b, p),
        "washbasin" => sanitary::washbasin(&mut b, p),
        "bath" => sanitary::bath(&mut b, p),
        "shower" => sanitary::shower(&mut b, p),
        "mirror" => accessories::mirror(&mut b, p),
        "towel_rail" => accessories::towel_rail(&mut b, p),
        "toilet_roll" => accessories::toilet_roll(&mut b, p),
        "grab_bar" => accessories::grab_bar(&mut b, p),
        "hood" => accessories::hood(&mut b, p),
        "shelf" => accessories::shelf(&mut b, p),
        "person" => people::person(&mut b, p),
        "car" => vehicles::car(&mut b, p),
        "rug" => cozy::rug(&mut b, p),
        "lamp" => cozy::lamp(&mut b, p),
        "potted_plant" => cozy::potted_plant(&mut b, p),
        "armchair" => cozy::armchair(&mut b, p),
        "pouf" => cozy::pouf(&mut b, p),
        "side_table" => cozy::side_table(&mut b, p),
        "sideboard" => cozy::sideboard(&mut b, p),
        "curtain" => cozy::curtain(&mut b, p),
        "decor" => cozy::decor(&mut b, p),
        "stool" => home::stool(&mut b, p),
        "island" => home::island(&mut b, p),
        "counter_decor" => home::counter_decor(&mut b, p),
        "towels" => home::towels(&mut b, p),
        "basket" => home::basket(&mut b, p),
        "wall_decor" => home::wall_decor(&mut b, p),
        "floor_cushion" => home::floor_cushion(&mut b, p),
        "blanket" => home::blanket(&mut b, p),
        "lounger" => garden::lounger(&mut b, p),
        "parasol" => garden::parasol(&mut b, p),
        "big_pot" => garden::big_pot(&mut b, p),
        "pergola" => garden::pergola(&mut b, p),
        "bench" => street::bench(&mut b, p),
        "streetlight" => street::streetlight(&mut b, p),
        "litter_bin" => street::litter_bin(&mut b, p),
        "bollard" => street::bollard(&mut b, p),
        "letter_box" => street::letter_box(&mut b, p),
        "container" => street::container(&mut b, p),
        "fountain" => street::fountain(&mut b, p),
        "planter" => street::planter(&mut b, p),
        "tree_grate" => street::tree_grate(&mut b, p),
        "bike_rack" => street::bike_rack(&mut b, p),
        "sign" => street::sign(&mut b, p),
        "bicycle" => street::bicycle(&mut b, p),
        "fireplace" => extras::fireplace(&mut b, p),
        "coat_stand" => extras::coat_stand(&mut b, p),
        "office_chair" => extras::office_chair(&mut b, p),
        "outdoor_set" => extras::outdoor_set(&mut b, p),
        "hanging_chair" => extras::hanging_chair(&mut b, p),
        "radiator" => extras::radiator(&mut b, p),
        "ac_unit" => extras::ac_unit(&mut b, p),
        "ceiling_fan" => extras::ceiling_fan(&mut b, p),
        "piano" => extras::piano(&mut b, p),
        "kids_bed" => extras::kids_bed(&mut b, p),
        _ => {
            if !fine::make(kind, &mut b, p) {
                return None;
            }
        }
    }
    Some(PropMesh {
        positions: b.positions,
        parts: b.parts.into_iter().collect(),
        soft: b.soft.into_iter().collect(),
    })
}

type Cache = Mutex<HashMap<(String, Vec<(String, i64)>), Arc<PropMesh>>>;

/// [`make`], kept: the same kind and values give the same model, made once.
pub fn mesh_of(kind: &str, p: &Params) -> Option<Arc<PropMesh>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (kind.to_string(), p.key());
    if let Some(m) = cache.lock().ok()?.get(&key) {
        return Some(m.clone());
    }
    let m = Arc::new(make(kind, p)?);
    if let Ok(mut c) = cache.lock() {
        if c.len() > 2000 {
            c.clear();
        }
        c.insert(key, m.clone());
    }
    Some(m)
}

#[cfg(test)]
mod tests;
