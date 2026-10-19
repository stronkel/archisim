//! Number formatting and parsing for the interface (millimetres, Spanish
//! decimal comma) and Spanish messages for model and file errors.

use archisim_core::ModelError;
use archisim_file::FileError;

/// Metres with three decimals and a decimal comma: `3,250`.
pub fn fmt_m(v: f64) -> String {
    let v = if v.abs() < 0.0005 { 0.0 } else { v };
    format!("{v:.3}").replace('.', ",")
}

/// A length given in metres, shown in millimetres: `2700`, or `2700,5`
/// when it is not a whole number of millimetres.
pub fn fmt_mm(v: f64) -> String {
    let mm = (v * 1000.0 * 10.0).round() / 10.0;
    let mm = if mm == 0.0 { 0.0 } else { mm };
    if (mm - mm.round()).abs() < 1e-9 {
        format!("{}", mm.round() as i64)
    } else {
        format!("{mm:.1}").replace('.', ",")
    }
}

/// Square metres with two decimals: `108,00 m²`.
pub fn fmt_area(v: f64) -> String {
    format!("{v:.2} m²").replace('.', ",")
}

/// Angle in degrees, counter-clockwise from the X axis, in `[0, 360)`.
pub fn fmt_angle(radians: f64) -> String {
    let mut d = radians.to_degrees() % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    if (d - 360.0).abs() < 0.05 {
        d = 0.0;
    }
    format!("{d:.1}°").replace('.', ",")
}

/// Reads a length typed by the user and returns it in metres.
///
/// Millimetres by default. Accepts a decimal comma or point (`2700,5`,
/// `2700.5`), a point as thousands separator (`2.700` is 2700 mm), and the
/// suffixes `mm`, `cm` and `m` (`3m` is 3000 mm).
pub fn parse_length(text: &str) -> Option<f64> {
    let t: String = text.trim().chars().filter(|c| !c.is_whitespace()).collect();
    let t = t.to_lowercase();
    let (number, factor) = if let Some(n) = t.strip_suffix("mm") {
        (n, 0.001)
    } else if let Some(n) = t.strip_suffix("cm") {
        (n, 0.01)
    } else if let Some(n) = t.strip_suffix('m') {
        (n, 1.0)
    } else {
        (t.as_str(), 0.001)
    };
    if number.is_empty() {
        return None;
    }
    let normalized = if number.contains(',') {
        // Decimal comma; any points are thousands separators.
        number.replace('.', "").replace(',', ".")
    } else if is_thousands(number) {
        number.replace('.', "")
    } else {
        number.to_string()
    };
    let value: f64 = normalized.parse().ok()?;
    value.is_finite().then_some(value * factor)
}

/// Reads a size typed as two lengths, `6000x4000` (also with `*` or `;`),
/// and returns both in metres, if both are positive.
pub fn parse_size(text: &str) -> Option<(f64, f64)> {
    let t = text.to_lowercase();
    let (a, b) = t.split_once(['x', '*', ';'])?;
    let (w, d) = (parse_length(a)?, parse_length(b)?);
    (w > 0.0 && d > 0.0).then_some((w, d))
}

/// `1.200` or `12.345.678`: groups of three digits after each point.
fn is_thousands(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let mut parts = s.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    let rest: Vec<&str> = parts.collect();
    !rest.is_empty()
        && (1..=3).contains(&first.len())
        && first.chars().all(|c| c.is_ascii_digit())
        && rest
            .iter()
            .all(|p| p.len() == 3 && p.chars().all(|c| c.is_ascii_digit()))
}

/// Why an edit was refused, in Spanish.
pub fn explain(e: &ModelError) -> String {
    match e {
        ModelError::UnknownLevel(_) => "la planta ya no existe".into(),
        ModelError::UnknownMaterial(_) => "el material no existe".into(),
        ModelError::UnknownWallType(_) => "el tipo de muro no existe".into(),
        ModelError::UnknownWall(_) => "el muro ya no existe".into(),
        ModelError::UnknownSlabType(_) => "el tipo de forjado no existe".into(),
        ModelError::UnknownSlab(_) => "el forjado ya no existe".into(),
        ModelError::UnknownOpeningType(_) => "el tipo de puerta o ventana no existe".into(),
        ModelError::UnknownOpening(_) => "la puerta o ventana ya no existe".into(),
        ModelError::OpeningOutsideWall => {
            "una puerta o ventana quedaría fuera del muro (largo o altura)".into()
        }
        ModelError::OverlappingOpenings => {
            "la puerta o ventana se solaparía con otra del mismo muro".into()
        }
        ModelError::FrameTooDeep => "el marco no cabe en el espesor del muro".into(),
        ModelError::WallTopBelowBase(_) => {
            "un muro unido a una planta superior quedaría con su coronación en su base o por debajo (revisa las cotas de las plantas o el desfase superior)".into()
        }
        ModelError::InvalidOutline => {
            "el contorno necesita al menos tres puntos y no puede cruzarse".into()
        }
        ModelError::ZeroLengthWall => "el muro necesita dos puntos distintos".into(),
        ModelError::InvalidHole => {
            "el hueco del forjado tiene que quedar dentro del forjado, sin cruzar su borde ni otros huecos".into()
        }
        ModelError::ArcTooTight => {
            "el muro curvo se cierra demasiado para su espesor (su cara interior se quedaría sin radio)".into()
        }
        ModelError::OpeningInCurvedWall => {
            "los muros curvos aún no admiten puertas ni ventanas".into()
        }
        ModelError::NonPositive(what) => format!("{} debe ser mayor que cero", es_what(what)),
        ModelError::NotFinite(what) => format!("{} no es un número válido", es_what(what)),
        ModelError::DuplicateId(id) => format!("el id {id} está repetido"),
        ModelError::InvalidId(id) => format!("el id {id} no es válido"),
        ModelError::DuplicateGuid(g) => format!("el GlobalId {g} está repetido"),
        ModelError::EmptyType => "un tipo necesita al menos una capa".into(),
        ModelError::InUse(what) => format!("está en uso ({what})"),
        ModelError::CategoryMismatch => {
            "una puerta solo puede cambiarse por otra puerta, y una ventana por otra ventana".into()
        }
        ModelError::UnknownDimension(_) => "la cota ya no existe".into(),
        ModelError::UnknownElevationMark(_) => "la cota de nivel ya no existe".into(),
        ModelError::DegenerateDimension => {
            "una cota necesita al menos dos puntos separados en su dirección".into()
        }
        ModelError::LastLevel => "el proyecto necesita al menos una planta".into(),
        ModelError::UnknownFamily(_) => "la familia no existe".into(),
        ModelError::Family(e) => explain_family(e),
        ModelError::DuplicateFamily(name) => format!("ya hay una familia llamada «{name}»"),
        ModelError::DuplicateMark(mark) => format!("la marca «{mark}» ya la tiene otro tipo"),
        ModelError::NotAnOpening(e) => {
            format!(
                "la familia «{}» no sirve para puertas ni ventanas",
                e.family
            )
        }
        ModelError::UnknownMemberType(_) => "el tipo de pilar o viga no existe".into(),
        ModelError::UnknownColumn(_) => "el pilar ya no existe".into(),
        ModelError::UnknownBeam(_) => "la viga ya no existe".into(),
        ModelError::ColumnTopBelowBase(_) => {
            "un pilar unido a una planta superior quedaría con su coronación en su base o por debajo (revisa las cotas de las plantas o el desfase superior)".into()
        }
        ModelError::ZeroLengthBeam => "la viga necesita dos puntos distintos".into(),
        ModelError::InvalidProfile => {
            "el alma y las alas no caben en la sección del perfil".into()
        }
        ModelError::UnknownSpace(_) => "el local ya no existe".into(),
        ModelError::UnknownSection(_) => "la sección ya no existe".into(),
        ModelError::ZeroLengthSection => "la sección necesita dos puntos distintos".into(),
        ModelError::UnknownElement(_) => "el elemento ya no existe".into(),
        ModelError::DuplicateNumber(n) => {
            format!("otro local de esta planta ya tiene el número «{n}»")
        }
    }
}

/// Why a family refused some values (or its recipe is wrong), in Spanish,
/// with the parameter names the recipe shows to the user.
pub fn explain_family(e: &archisim_core::FamilyError) -> String {
    use archisim_core::FamilyError as F;
    let num = |x: f64| {
        let t = format!("{x}");
        t.replace('.', ",")
    };
    match e {
        F::Syntax { line, message } => {
            format!("la receta de la familia tiene un error en la línea {line} ({message})")
        }
        F::MissingValue { param } => format!("falta el valor de «{param}»"),
        F::UnknownParam { param } => format!("la familia no tiene el parámetro «{param}»"),
        F::WrongKind {
            label, expected, ..
        } => {
            let kind = match *expected {
                "length" => "una longitud",
                "angle" => "un ángulo",
                "number" | "integer" => "un número",
                "bool" => "sí o no",
                "material" => "un material",
                "choice" => "una de sus opciones",
                _ => "un texto",
            };
            format!("«{label}» debe ser {kind}")
        }
        F::OutOfRange {
            label,
            value,
            min,
            max,
            ..
        } => format!(
            "«{label}» = {} está fuera de su rango ({} a {})",
            num(*value),
            num(*min),
            num(*max)
        ),
        F::NotInteger { label, .. } => format!("«{label}» debe ser un número entero"),
        F::UnknownOption { label, value, .. } => {
            format!("«{value}» no es una opción de «{label}»")
        }
        F::Requirement { message, .. } => message.clone(),
        F::NegativeSize { .. } => {
            "con esas medidas una pieza de la familia no cabe (quedaría con medida negativa)".into()
        }
        F::Eval { line, message } => {
            format!("la receta de la familia falla en la línea {line} ({message})")
        }
        F::TooComplex { .. } => "la familia generaría demasiada geometría".into(),
    }
}

/// Why the magic wand found no room, in Spanish.
pub fn explain_room(e: &archisim_geom::RoomError) -> String {
    match e {
        archisim_geom::RoomError::InsideWall => {
            "has hecho clic dentro de un muro: haz clic dentro de un recinto".into()
        }
        archisim_geom::RoomError::NotEnclosed => {
            "los muros no cierran un recinto alrededor de ese punto".into()
        }
    }
}

/// Spanish name of the quantity named in a [`ModelError`].
fn es_what(what: &str) -> String {
    match what {
        "wall height" => "la altura del muro",
        "wall start" => "el inicio del muro",
        "wall end" => "el final del muro",
        "wall base offset" => "el desfase de base",
        "layer thickness" => "el espesor de capa",
        "slab top offset" => "el desfase del forjado",
        "slab outline point" => "un punto del contorno",
        "opening position" => "la posición",
        "sill height" => "el antepecho",
        "frame setback" => "el retranqueo",
        "opening width" => "el ancho",
        "opening height" => "el alto",
        "frame width" => "el ancho del marco",
        "frame depth" => "el fondo del marco",
        "clear opening inside the frame" => "el paso libre dentro del marco",
        "level elevation" => "la cota de la planta",
        "column height" => "la altura del pilar",
        "column position" => "la posición del pilar",
        "column rotation" => "el giro del pilar",
        "column base offset" => "el desfase de base del pilar",
        "column top offset" => "el desfase superior del pilar",
        "beam start" => "el inicio de la viga",
        "beam end" => "el final de la viga",
        "beam top offset" => "el desfase de la viga",
        "section width" => "el ancho de la sección",
        "section depth" => "el canto de la sección",
        "section diameter" => "el diámetro de la sección",
        "web thickness" => "el espesor del alma",
        "flange thickness" => "el espesor de las alas",
        "space height" => "la altura del local",
        "section start" => "el inicio de la sección",
        "section end" => "el final de la sección",
        "section view depth" => "la profundidad de la sección",
        "space label" => "la posición del rótulo del local",
        other => other,
    }
    .to_string()
}

/// Why a project could not be opened or saved, in Spanish.
pub fn explain_file(e: &FileError) -> String {
    match e {
        FileError::Io(e) => format!("error de lectura o escritura: {e}"),
        FileError::Json(e) => format!("el fichero está mal escrito: {e}"),
        FileError::NotAProject => "no es un proyecto de ArchiSim".into(),
        FileError::TooNew { version } => format!(
            "el proyecto se guardó con un ArchiSim más nuevo (versión de fichero {version}); \
             actualiza ArchiSim para abrirlo"
        ),
        FileError::UnknownVersion(v) => format!("versión de fichero desconocida ({v})"),
        FileError::Units(u) => format!("unidades «{u}» no admitidas (solo metros)"),
        FileError::Value { element, message } => format!("{element}: {message}"),
        FileError::Model(r) => format!("{}: {}", r.element, explain(&r.error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(fmt_m(3.25), "3,250");
        assert_eq!(fmt_m(-0.0001), "0,000");
        assert_eq!(fmt_mm(2.7), "2700");
        assert_eq!(fmt_mm(0.2655), "265,5");
        assert_eq!(fmt_mm(-0.05), "-50");
        assert_eq!(fmt_area(108.0), "108,00 m²");
        assert_eq!(fmt_angle(std::f64::consts::FRAC_PI_2), "90,0°");
        assert_eq!(fmt_angle(-std::f64::consts::FRAC_PI_2), "270,0°");
    }

    #[test]
    fn parses_lengths() {
        let eq = |t: &str, m: f64| {
            let v = parse_length(t).unwrap_or_else(|| panic!("{t}"));
            assert!((v - m).abs() < 1e-12, "{t} → {v}");
        };
        eq("2700", 2.7);
        eq(" 2700,5 ", 2.7005);
        eq("2700.5", 2.7005);
        eq("2.700", 2.7);
        eq("1.234,5", 1.2345);
        eq("-50", -0.05);
        eq("3m", 3.0);
        eq("3,5 m", 3.5);
        eq("27cm", 0.27);
        eq("450mm", 0.45);
        eq("0.5", 0.0005);
        assert_eq!(parse_length(""), None);
        assert_eq!(parse_length("abc"), None);
        assert_eq!(parse_length("m"), None);
    }

    #[test]
    fn errors_in_spanish() {
        assert!(explain(&ModelError::NonPositive("wall height")).starts_with("la altura"));
        assert!(explain(&ModelError::OverlappingOpenings).contains("solaparía"));
    }
}
