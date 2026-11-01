//! Number formatting and parsing for the interface (lengths in the unit
//! chosen in Vista → Unidades, metres by default; Spanish decimal comma)
//! and Spanish messages for model and file errors.

use std::borrow::Cow;
use std::cell::Cell;

use archisim_core::ModelError;
use archisim_file::FileError;

/// The unit lengths are shown and typed in (the model is always in
/// metres).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LengthUnit {
    #[default]
    Metre,
    Centimetre,
    Millimetre,
}

impl LengthUnit {
    pub const ALL: [LengthUnit; 3] = [
        LengthUnit::Metre,
        LengthUnit::Centimetre,
        LengthUnit::Millimetre,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            LengthUnit::Metre => "m",
            LengthUnit::Centimetre => "cm",
            LengthUnit::Millimetre => "mm",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LengthUnit::Metre => "Metros",
            LengthUnit::Centimetre => "Centímetros",
            LengthUnit::Millimetre => "Milímetros",
        }
    }

    pub fn from_symbol(s: &str) -> Option<LengthUnit> {
        LengthUnit::ALL.into_iter().find(|u| u.symbol() == s)
    }

    /// Metres in one of this unit.
    fn factor(self) -> f64 {
        match self {
            LengthUnit::Metre => 1.0,
            LengthUnit::Centimetre => 0.01,
            LengthUnit::Millimetre => 0.001,
        }
    }
}

thread_local! {
    // The tests type millimetres, as the program did before.
    static UNIT: Cell<LengthUnit> = const {
        Cell::new(if cfg!(test) { LengthUnit::Millimetre } else { LengthUnit::Metre })
    };
}

/// The unit lengths are shown and typed in.
pub fn unit() -> LengthUnit {
    UNIT.with(|u| u.get())
}

pub fn set_unit(u: LengthUnit) {
    UNIT.with(|c| c.set(u));
}

/// A label written for millimetres («Ancho (mm)») in the unit in use.
pub fn unit_label(label: &str) -> Cow<'_, str> {
    let u = unit();
    if u != LengthUnit::Millimetre && label.contains("(mm)") {
        Cow::Owned(label.replace("(mm)", &format!("({})", u.symbol())))
    } else {
        Cow::Borrowed(label)
    }
}

/// Metres with three decimals and a decimal comma: `3,250`.
pub fn fmt_m(v: f64) -> String {
    let v = if v.abs() < 0.0005 { 0.0 } else { v };
    format!("{v:.3}").replace('.', ",")
}

/// A length given in metres, shown in the unit in use: in metres with two
/// decimals (three when it has millimetres: `2,70`, `2,705`), in
/// centimetres (`270`, `270,5`) or in millimetres (`2700`, `2700,5`).
pub fn fmt_mm(v: f64) -> String {
    match unit() {
        LengthUnit::Metre => {
            let mm = (v * 1000.0).round();
            let mm = if mm == 0.0 { 0.0 } else { mm };
            if (mm / 10.0 - (mm / 10.0).round()).abs() < 1e-9 {
                format!("{:.2}", mm / 1000.0).replace('.', ",")
            } else {
                format!("{:.3}", mm / 1000.0).replace('.', ",")
            }
        }
        LengthUnit::Centimetre => {
            let cm = (v * 100.0 * 10.0).round() / 10.0;
            let cm = if cm == 0.0 { 0.0 } else { cm };
            if (cm - cm.round()).abs() < 1e-9 {
                format!("{}", cm.round() as i64)
            } else {
                format!("{cm:.1}").replace('.', ",")
            }
        }
        LengthUnit::Millimetre => fmt_millimetres(v),
    }
}

/// A length with its unit: `2,70 m`.
pub fn fmt_len(v: f64) -> String {
    format!("{} {}", fmt_mm(v), unit().symbol())
}

/// A length given in metres, always in millimetres: `2700`, or `2700,5`
/// when it is not a whole number of millimetres.
pub fn fmt_millimetres(v: f64) -> String {
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
/// In the unit in use unless it says another one (the suffixes `mm`, `cm`
/// and `m`: `270cm`, `3m`). Accepts a decimal comma or point (`2,70`,
/// `2.70`); in millimetres a point between groups of three digits is a
/// thousands separator (`2.700` is 2700 mm).
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
        (t.as_str(), unit().factor())
    };
    if number.is_empty() {
        return None;
    }
    let normalized = if number.contains(',') {
        // Decimal comma; any points are thousands separators.
        number.replace('.', "").replace(',', ".")
    } else if (factor - 0.001).abs() < 1e-12 && is_thousands(number) {
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
        ModelError::InvalidParameter(why) => why.clone(),
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
        ModelError::UnknownSection(_) => "la sección ya no existe".into(),
        ModelError::ZeroLengthSection => "la sección necesita dos puntos distintos".into(),
        ModelError::UnknownRoof(_) => "la cubierta ya no existe".into(),
        ModelError::UnknownStair(_) => "la escalera ya no existe".into(),
        ModelError::UnknownTerrain(_) => "el terreno ya no existe".into(),
        ModelError::UnknownCurtainWall(_) => "el muro cortina ya no existe".into(),
        ModelError::UnknownRailing(_) => "la barandilla ya no existe".into(),
        ModelError::EmptyName => "el nombre no puede quedar vacío".into(),
        ModelError::UnknownObjectType(_) => "el tipo de objeto ya no existe".into(),
        ModelError::UnknownObject(_) => "el objeto ya no existe".into(),
        ModelError::UnknownMesh(_) => "el modelo 3D ya no está en el proyecto".into(),
        ModelError::InvalidMesh(why) => format!("el modelo 3D no es válido: {why}"),
        ModelError::NotAnObject(name) => {
            format!("la familia «{name}» no hace objetos (es una puerta, una ventana o va en un muro)")
        }
        ModelError::InvalidPriority => "la prioridad de intersección va de 0 a 999".into(),
        ModelError::InvalidDress => "un edificio del entorno es una masa por plantas (de 1 a 60) de contorno; las ventanas, más estrechas que su ritmo (de 1,2 a 12 m)".into(),
        ModelError::InvalidRailing => "la barandilla necesita dos puntos al menos (a más de 5 cm), una altura de 30 cm a 3 m y montantes a 30 cm – 5 m".into(),
        ModelError::InvalidCurtainWall => "el muro cortina necesita altura y sitio entre los marcos de su tipo en cada paño y cada fila".into(),
        ModelError::UnknownCurtainType(_) => "el tipo de muro cortina ya no existe".into(),
        ModelError::InvalidCurtainType => "el tipo de muro cortina necesita paños fijos de al menos 10 cm (20 cm si se reparten iguales), marcos de 5 mm a 1 m, al menos un panel no más grueso que el marco y filas que usen sus paneles".into(),
        ModelError::InvalidWallProfile(_) => {
            "el perfil necesita un muro recto y un contorno (y huecos) con superficie".into()
        }
        ModelError::InvalidTerrain => {
            "el terreno necesita tres puntos que no estén en línea y algo de profundidad".into()
        }
        ModelError::InvalidStair => "la escalera necesita una línea de huella con sitio para un tramo, algo que subir, un ancho de al menos 40 cm y huella de al menos 10 cm".into(),
        ModelError::InvalidRoof => "cada lado necesita una pendiente (de 0 a 300 %) o ser piñón, al menos uno con pendiente, y el alero no puede pasar de 3 m".into(),
        ModelError::UnknownDormer(_) => "la buhardilla ya no existe".into(),
        ModelError::InvalidDormer => "la buhardilla necesita un ancho de 40 cm a 8 m, una altura de 30 cm a 4 m y una pendiente de hasta el 300 %; el canalón, de 5 a 50 cm".into(),
        ModelError::DormerDoesNotFit => "la buhardilla no cabe entera en ese faldón (o su cubierta tiene tanta pendiente como el faldón): pruébala más abajo, más estrecha o más baja".into(),
        ModelError::RoofNotConvex => "no se pueden resolver los faldones de ese contorno (o el alero se cruza consigo mismo: prueba con menos vuelo)".into(),
        ModelError::InvalidSlope => {
            "la pendiente necesita una dirección y no puede pasar del 300 %".into()
        }
        ModelError::UnknownElement(_) => "el elemento ya no existe".into(),
        ModelError::DuplicateNumber(n) => {
            format!("otro local de esta planta ya tiene el número «{n}»")
        }
        ModelError::UnknownDrawLayer(_) => "la capa no existe".into(),
        ModelError::UnknownDraft(_) => "el elemento de dibujo ya no existe".into(),
        ModelError::UnknownDraftView(_) => "la vista de dibujo ya no existe".into(),
        ModelError::UnknownSheet(_) => "el plano ya no existe".into(),
        ModelError::UnknownViewport(_) => "la vista ya no está en el plano".into(),
        ModelError::UnknownRoom(_) => "el local ya no existe".into(),
        ModelError::UnknownPresentationStyle(_) => "el estilo de presentación ya no existe".into(),
        ModelError::UnknownPlanView(_) => "la vista guardada ya no existe".into(),
        ModelError::UnknownViewTemplate(_) => "la plantilla de vista ya no existe".into(),
        ModelError::UnknownRefPlane(_) => "el plano de referencia ya no existe".into(),
        ModelError::UnknownTag(_) => "la etiqueta ya no existe".into(),
        ModelError::UnknownTable(_) => "la tabla ya no está en el plano".into(),
        ModelError::InvalidShape(what) => es_shape(what).into(),
        ModelError::DuplicateName(name) => format!("ya hay otro elemento llamado «{name}»"),
        ModelError::TypeConflict(name) => {
            format!("el proyecto ya tiene el tipo «{name}» con otros valores")
        }
    }
}

/// Why a drafting element is not well formed, in Spanish.
fn es_shape(what: &str) -> &str {
    match what {
        "line has no length" => "la línea no tiene longitud",
        "polyline needs more points" => "la polilínea necesita más puntos",
        "hatch boundaries must be closed" => "el contorno del sombreado tiene que estar cerrado",
        "polyline needs one bulge per point" => "a la polilínea le faltan curvaturas",
        "polyline arc too large" => "un arco de la polilínea es demasiado grande",
        "polyline has no length" => "la polilínea no tiene longitud",
        "arc sweep must be between 0 and 360°" => "el arco tiene que abarcar entre 0 y 360°",
        "ellipse ratio must be in (0, 1]" => {
            "la proporción de la elipse tiene que estar entre 0 y 1"
        }
        "ellipse sweep must be in (0, 360°]" => {
            "el arco de elipse tiene que abarcar entre 0 y 360°"
        }
        "text is empty" => "el texto está vacío",
        "hatch needs a boundary" => "el sombreado necesita un contorno",
        "layer name is empty" => "la capa necesita un nombre",
        "layer 0 cannot be renamed" => "la capa 0 no se puede renombrar",
        "no room there" => {
            "ahí no hay un recinto cerrado por muros (dibuje su contorno con la opción Polígono)"
        }
        "nothing to tag there" => {
            "eso no se etiqueta: elija una puerta, una ventana, un muro o un local"
        }
        other => other,
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
        "presentation style" => "el grueso de las líneas del estilo",
        "floor fill scale" => "la escala del pavimento",
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
        "room height" => "la altura del local",
        "section start" => "el inicio de la sección",
        "terrain point" => "un punto del terreno",
        "terrain point height" => "la cota de un punto",
        "terrain depth" => "la profundidad del terreno",
        "terrain contour interval" => "la equidistancia de las curvas",
        "true north" => "el norte",
        "stair path point" => "un punto de la línea de huella",
        "stair width" => "el ancho de la escalera",
        "stair going" => "la huella",
        "stair base offset" => "el arranque de la escalera",
        "stair height" => "la altura de la escalera",
        "stair waist" => "el espesor de la losa",
        "roof base offset" => "la altura de la cubierta",
        "roof overhang" => "el alero",
        "roof outline point" => "un punto del contorno de la cubierta",
        "roof slope" => "la pendiente de la cubierta",
        "slab slope start" => "el inicio de la flecha de pendiente",
        "slab slope end" => "el final de la flecha de pendiente",
        "slab slope" => "la pendiente",
        "section end" => "el final de la sección",
        "section view depth" => "la profundidad de la sección",
        "room label" => "la posición del rótulo del local",
        "circle radius" => "el radio del círculo",
        "arc radius" => "el radio del arco",
        "ellipse axis" => "el eje de la elipse",
        "text height" => "la altura del texto",
        "hatch scale" => "la escala del sombreado",
        "line weight" | "layer line weight" => "el grosor de línea",
        "polyline width" => "la anchura de la polilínea",
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

    #[test]
    fn lengths_in_metres_and_centimetres() {
        set_unit(LengthUnit::Metre);
        assert_eq!(fmt_mm(2.7), "2,70");
        assert_eq!(fmt_mm(2.705), "2,705");
        assert_eq!(fmt_len(0.0), "0,00 m");
        assert_eq!(parse_length("2,7"), Some(2.7));
        assert_eq!(parse_length("2.70"), Some(2.7));
        assert_eq!(parse_length("270cm"), Some(2.7));
        assert_eq!(parse_length("2700mm"), Some(2.7));
        assert_eq!(parse_size("6x4"), Some((6.0, 4.0)));
        assert_eq!(unit_label("Ancho (mm)"), "Ancho (m)");
        set_unit(LengthUnit::Centimetre);
        assert_eq!(fmt_mm(2.705), "270,5");
        assert_eq!(parse_length("25"), Some(0.25));
        set_unit(LengthUnit::Millimetre);
        assert_eq!(unit_label("Ancho (mm)"), "Ancho (mm)");
    }
}
