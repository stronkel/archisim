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

pub mod build;
mod furniture;
mod kitchen;
mod people;
mod sanitary;
mod vehicles;

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
}

impl PropMesh {
    pub fn triangle_count(&self) -> usize {
        self.parts.iter().map(|p| p.1.len()).sum()
    }
}

/// What can be made, with the name of each model (`style`).
pub const KINDS: &[(&str, &[&str])] = &[
    ("sofa", &["Contemporáneo", "Clásico", "Nórdico"]),
    ("chair", &["Madera", "Tubo metálico", "Tapizada"]),
    ("table", &["Cuatro patas", "Patas metálicas", "Caballetes"]),
    (
        "coffee_table",
        &["Madera con balda", "Bloque", "Vidrio y metal"],
    ),
    ("bed", &["Tapizada", "Madera", "Tarima"]),
    ("nightstand", &["Cajonera", "Con patas"]),
    ("wardrobe", &["Puertas batientes"]),
    ("bookcase", &["Con libros"]),
    ("tv_unit", &["Con televisor"]),
    ("counter", &["Con fregadero y placa"]),
    ("wall_cabinet", &["Puertas"]),
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
    ("person", &["Hombre", "Mujer", "Caminando"]),
    ("car", &["Berlina", "Compacto", "Todoterreno"]),
];

/// The names of the models of a kind.
pub fn styles(kind: &str) -> &'static [&'static str] {
    KINDS.iter().find(|k| k.0 == kind).map_or(&[], |k| k.1)
}

/// Makes a model. `None` for a kind ArchiSim does not make.
pub fn make(kind: &str, p: &Params) -> Option<PropMesh> {
    let mut b = build::Builder::default();
    match kind {
        "sofa" => furniture::sofa(&mut b, p),
        "chair" => furniture::chair(&mut b, p),
        "table" => furniture::table(&mut b, p),
        "coffee_table" => furniture::coffee_table(&mut b, p),
        "bed" => furniture::bed(&mut b, p),
        "nightstand" => furniture::nightstand(&mut b, p),
        "wardrobe" => furniture::wardrobe(&mut b, p),
        "bookcase" => furniture::bookcase(&mut b, p),
        "tv_unit" => furniture::tv_unit(&mut b, p),
        "counter" => kitchen::counter(&mut b, p),
        "wall_cabinet" => kitchen::wall_cabinet(&mut b, p),
        "fridge" => kitchen::fridge(&mut b, p),
        "washer" => kitchen::washer(&mut b, p),
        "wc" => sanitary::wc(&mut b, p),
        "bidet" => sanitary::bidet(&mut b, p),
        "washbasin" => sanitary::washbasin(&mut b, p),
        "bath" => sanitary::bath(&mut b, p),
        "shower" => sanitary::shower(&mut b, p),
        "person" => people::person(&mut b, p),
        "car" => vehicles::car(&mut b, p),
        _ => return None,
    }
    Some(PropMesh {
        positions: b.positions,
        parts: b.parts.into_iter().collect(),
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
