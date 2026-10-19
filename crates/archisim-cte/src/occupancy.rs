//! Occupancy of each room (CTE DB-SI 3, 2.1): its floor area divided by
//! the area per person of its use, as table 2.1 gives it for the building's
//! use, unless the room says otherwise.

use archisim_core::{BuildingUse, Model, Room};

/// Area per person of a room and where it comes from.
#[derive(Debug, Clone, PartialEq)]
pub struct Density {
    /// m² per person; `None`: no occupants ("ocupación nula").
    pub per_person: Option<f64>,
    /// The line of table 2.1, or "del local".
    pub source: String,
}

/// Lower case, without accents, for matching uses.
pub fn plain(s: &str) -> String {
    s.to_lowercase()
        .chars()
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

fn has(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

/// What a room is, from its use (or its name when it has no use).
pub fn room_text(r: &Room) -> String {
    plain(if r.usage.trim().is_empty() {
        &r.name
    } else {
        &r.usage
    })
}

/// Circulation: corridors, halls, landings.
pub fn is_corridor(text: &str) -> bool {
    has(text, &["pasillo", "distribuidor", "corredor", "rellano"])
}

pub fn is_hall(text: &str) -> bool {
    has(
        text,
        &[
            "vestibulo",
            "recibidor",
            "portal",
            "entrada",
            "recepcion",
            "hall",
        ],
    )
}

/// Zones of a residential building shared by its dwellings.
pub fn is_common(text: &str) -> bool {
    has(
        text,
        &[
            "zona comun",
            "zonas comunes",
            "portal",
            "rellano",
            "escalera",
            "pasillo comun",
            "galeria",
        ],
    )
}

/// The density of table 2.1 for a room, in a building of the given use.
pub fn density(model: &Model, use_: BuildingUse, r: &Room) -> Density {
    if let Some(d) = r.occupancy {
        return Density {
            per_person: (d > 0.0).then_some(d),
            source: "indicada en el local".into(),
        };
    }
    let t = room_text(r);
    let d = |v: f64, s: &str| Density {
        per_person: Some(v),
        source: s.into(),
    };
    let none = |s: &str| Density {
        per_person: None,
        source: s.into(),
    };
    // Any use.
    if has(
        &t,
        &[
            "instalaciones",
            "maquinas",
            "limpieza",
            "contadores",
            "cuarto tecnico",
        ],
    ) {
        return none("Zonas de ocupación ocasional y mantenimiento: ocupación nula");
    }
    if has(&t, &["garaje", "aparcamiento"]) {
        return d(40.0, "Aparcamiento: 40 m²/persona");
    }
    if has(&t, &["almacen", "archivo", "trastero", "despensa"]) && use_ != BuildingUse::Dwelling {
        return d(40.0, "Archivos, almacenes: 40 m²/persona");
    }
    // Corridors and stairs are used one after the other with the rooms
    // they serve (2.2, alternancia).
    if is_corridor(&t) || has(&t, &["escalera"]) {
        return none("Circulaciones (uso alternativo con las zonas a las que sirven, 2.2)");
    }
    if use_ != BuildingUse::Dwelling && has(&t, &["aseo", "bano", "vestuario"]) {
        return d(3.0, "Aseos de planta: 3 m²/persona");
    }
    let ground = level_is_ground(model, r);
    match use_ {
        BuildingUse::Dwelling => d(
            20.0,
            "Residencial Vivienda, plantas de vivienda: 20 m²/persona",
        ),
        BuildingUse::PublicResidential => {
            if has(&t, &["salon", "uso multiple"]) {
                d(
                    1.0,
                    "Residencial Público, salones de uso múltiple: 1 m²/persona",
                )
            } else if is_hall(&t) {
                d(
                    2.0,
                    "Residencial Público, vestíbulos y zonas públicas: 2 m²/persona",
                )
            } else {
                d(
                    20.0,
                    "Residencial Público, zonas de alojamiento: 20 m²/persona",
                )
            }
        }
        BuildingUse::Administrative => {
            if is_hall(&t) {
                d(
                    2.0,
                    "Administrativo, vestíbulos y zonas de uso público: 2 m²/persona",
                )
            } else {
                d(
                    10.0,
                    "Administrativo, plantas o zonas de oficinas: 10 m²/persona",
                )
            }
        }
        BuildingUse::Teaching => {
            if has(&t, &["infantil", "lectura", "biblioteca"]) {
                d(
                    2.0,
                    "Docente, aulas de escuela infantil y salas de lectura: 2 m²/persona",
                )
            } else if has(&t, &["aula"]) {
                d(1.5, "Docente, aulas: 1,5 m²/persona")
            } else if has(&t, &["laboratorio", "taller", "gimnasio", "dibujo"]) {
                d(
                    5.0,
                    "Docente, laboratorios, talleres, gimnasios: 5 m²/persona",
                )
            } else {
                d(10.0, "Docente, conjunto de la planta: 10 m²/persona")
            }
        }
        BuildingUse::Hospital => {
            if has(&t, &["espera"]) {
                d(2.0, "Hospitalario, salas de espera: 2 m²/persona")
            } else if has(&t, &["hospitalizacion", "habitacion"]) {
                d(
                    15.0,
                    "Hospitalario, zonas de hospitalización: 15 m²/persona",
                )
            } else {
                d(
                    10.0,
                    "Hospitalario, servicios ambulatorios y de diagnóstico: 10 m²/persona",
                )
            }
        }
        BuildingUse::Commercial => {
            if has(&t, &["mueble", "vehiculo", "exposicion"]) {
                d(5.0, "Comercial, ventas sin gran afluencia: 5 m²/persona")
            } else if ground {
                d(
                    2.0,
                    "Comercial, áreas de venta en sótano, baja y entreplanta: 2 m²/persona",
                )
            } else {
                d(
                    3.0,
                    "Comercial, áreas de venta en otras plantas: 3 m²/persona",
                )
            }
        }
        BuildingUse::Assembly => {
            if has(&t, &["bar", "cafeteria"]) && !has(&t, &["sentado"]) {
                d(
                    1.0,
                    "Pública concurrencia, público de pie en bares: 1 m²/persona",
                )
            } else if has(&t, &["comedor", "restaurante"]) {
                d(
                    1.5,
                    "Pública concurrencia, público sentado en restaurantes: 1,5 m²/persona",
                )
            } else if has(&t, &["cocina", "barra", "servicio"]) {
                d(
                    10.0,
                    "Pública concurrencia, zonas de servicio: 10 m²/persona",
                )
            } else if has(&t, &["gimnasio"]) {
                d(
                    5.0,
                    "Pública concurrencia, gimnasios con aparatos: 5 m²/persona",
                )
            } else if has(&t, &["espera", "lectura", "museo", "exposicion", "sala"]) || is_hall(&t)
            {
                d(
                    2.0,
                    "Pública concurrencia, salas de espera, vestíbulos, exposiciones: 2 m²/persona",
                )
            } else {
                d(1.5, "Pública concurrencia, público sentado: 1,5 m²/persona")
            }
        }
        BuildingUse::Parking => d(40.0, "Aparcamiento: 40 m²/persona"),
    }
}

/// A basement, ground floor or mezzanine: the level nearest the ±0,00 or
/// below it.
fn level_is_ground(model: &Model, r: &Room) -> bool {
    model.level(r.level).is_some_and(|l| l.elevation <= 1.5)
}

/// People in a room: area ÷ area per person, rounded up.
pub fn occupants(area: f64, d: &Density) -> u32 {
    match d.per_person {
        Some(per) if per > 0.0 && area > 0.0 => (area / per - 1e-9).ceil().max(1.0) as u32,
        _ => 0,
    }
}
