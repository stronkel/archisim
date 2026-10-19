//! Checking a file against an IDS: which elements each specification
//! applies to, and which of them fail which requirement.

use archisim_ifc::step_read::StepFile;

use crate::model::{Found, IfcModel, Prop};
use crate::spec::{
    Facet, FacetCardinality, Ids, Restriction, SpecCardinality, Specification, ValueSpec,
};

/// The result of checking one file against one IDS.
#[derive(Debug, Clone)]
pub struct IdsReport {
    pub title: String,
    pub schema: String,
    pub specifications: Vec<SpecResult>,
}

impl IdsReport {
    pub fn passed(&self) -> bool {
        self.specifications.iter().all(|s| s.passed)
    }

    /// Readable summary in Spanish, as the command line prints it.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        let ok = self.specifications.iter().filter(|s| s.passed).count();
        s += &format!(
            "Requisitos «{}» sobre un IFC {}: {} de {} especificaciones cumplen.\n",
            if self.title.is_empty() {
                "sin título"
            } else {
                &self.title
            },
            self.schema,
            ok,
            self.specifications.len()
        );
        for r in &self.specifications {
            s += &format!(
                "\n{} {}  ({} elementos afectados, {} incumplen)\n",
                if r.passed { "✔" } else { "✘" },
                r.name,
                r.applicable,
                r.failures.len()
            );
            if let Some(n) = &r.note {
                s += &format!("   {n}\n");
            }
            for f in r.failures.iter().take(20) {
                s += &format!("   · {}: {}\n", f.element, f.reasons.join("; "));
            }
            if r.failures.len() > 20 {
                s += &format!("   · … y {} más\n", r.failures.len() - 20);
            }
        }
        s
    }
}

#[derive(Debug, Clone)]
pub struct SpecResult {
    pub name: String,
    pub description: String,
    pub passed: bool,
    /// Elements the specification applies to.
    pub applicable: usize,
    /// A reason that concerns the whole specification (no applicable
    /// element where one is required, not for this schema…).
    pub note: Option<String>,
    pub failures: Vec<ElementFailure>,
}

#[derive(Debug, Clone)]
pub struct ElementFailure {
    /// Step id in the checked file.
    pub step_id: u64,
    pub global_id: Option<String>,
    /// «IfcWall «Muro exterior» 2N1qQ…».
    pub element: String,
    pub reasons: Vec<String>,
}

/// Checks a parsed IFC file against an IDS.
pub fn check(ids: &Ids, file: &StepFile) -> IdsReport {
    let model = IfcModel::new(file);
    let mut out = IdsReport {
        title: ids.title.clone(),
        schema: model.schema_name.clone(),
        specifications: Vec::new(),
    };
    let mut ids_sorted: Vec<u64> = file.entities.keys().copied().collect();
    ids_sorted.sort_unstable();
    for spec in &ids.specifications {
        out.specifications
            .push(check_spec(spec, &model, &ids_sorted));
    }
    out
}

fn schema_matches(spec: &Specification, schema: &str) -> bool {
    spec.ifc_versions.is_empty()
        || spec.ifc_versions.iter().any(|v| {
            let v = v.as_str();
            v == schema
                || (v.starts_with("IFC4X3") && schema.starts_with("IFC4X3"))
                || (v == "IFC4" && schema == "IFC4")
                || (v == "IFC2X3" && schema.starts_with("IFC2X3"))
        })
}

fn check_spec(spec: &Specification, m: &IfcModel, all: &[u64]) -> SpecResult {
    let mut r = SpecResult {
        name: if spec.name.is_empty() {
            "(sin nombre)".into()
        } else {
            spec.name.clone()
        },
        description: spec.description.clone(),
        passed: true,
        applicable: 0,
        note: None,
        failures: Vec::new(),
    };
    if let Some(why) = &spec.invalid {
        r.passed = false;
        r.note = Some(format!("Especificación no válida: {why}."));
        return r;
    }
    if !schema_matches(spec, &m.schema_name) {
        // Checked anyway, as other checkers do; the person is told.
        r.note = Some(format!(
            "Escrita para {}; el modelo es {}.",
            spec.ifc_versions.join(", "),
            m.schema_name
        ));
    }
    // Candidates: by class when the applicability names one, else all.
    let candidates: Vec<u64> = match spec.applicability.iter().find_map(|f| match f {
        Facet::Entity {
            name: ValueSpec::Simple(n),
            ..
        } => Some(n.clone()),
        _ => None,
    }) {
        Some(class) => {
            let mut v = m.file.of_kind(&class);
            if m.schema_name.starts_with("IFC2X3") {
                for (name, occ, _) in crate::schema::IFC2X3_TYPED_CLASSES {
                    if *name == class {
                        v.extend(m.file.of_kind(occ));
                    }
                }
            }
            v
        }
        None => all.to_vec(),
    };
    let applicable: Vec<u64> = candidates
        .into_iter()
        .filter(|&id| {
            !spec.applicability.is_empty()
                && spec.applicability.iter().all(|f| eval(f, id, m).holds)
        })
        .collect();
    r.applicable = applicable.len();
    match spec.cardinality {
        SpecCardinality::Required if applicable.is_empty() => {
            r.passed = false;
            r.note = Some(
                "Es obligatoria y ningún elemento del modelo la cumple en su aplicabilidad.".into(),
            );
        }
        SpecCardinality::Prohibited if !applicable.is_empty() => {
            r.passed = false;
            r.note = Some(format!(
                "Está prohibida y la cumplen {} elementos.",
                applicable.len()
            ));
            for &id in &applicable {
                r.failures.push(ElementFailure {
                    step_id: id,
                    global_id: m.global_id(id),
                    element: m.label(id),
                    reasons: vec!["no debería existir".into()],
                });
            }
            return r;
        }
        _ => {}
    }
    for &id in &applicable {
        let mut reasons = Vec::new();
        for req in &spec.requirements {
            let outcome = eval(&req.facet, id, m);
            let ok = match req.cardinality {
                FacetCardinality::Required => outcome.holds,
                FacetCardinality::Optional => outcome.holds || !outcome.present,
                FacetCardinality::Prohibited => !outcome.holds,
            };
            if !ok {
                reasons.push(match req.cardinality {
                    FacetCardinality::Prohibited => {
                        format!("tiene {}, que está prohibido", req.facet.describe())
                    }
                    _ => outcome
                        .why
                        .unwrap_or_else(|| format!("no cumple {}", req.facet.describe())),
                });
            }
        }
        if !reasons.is_empty() {
            r.passed = false;
            r.failures.push(ElementFailure {
                step_id: id,
                global_id: m.global_id(id),
                element: m.label(id),
                reasons,
            });
        }
    }
    r
}

/// How a facet came out for an element.
struct Outcome {
    holds: bool,
    /// Whether the information the facet asks about exists at all (an
    /// optional facet only checks what is there).
    present: bool,
    why: Option<String>,
}

impl Outcome {
    fn yes() -> Outcome {
        Outcome {
            holds: true,
            present: true,
            why: None,
        }
    }
    fn absent(why: String) -> Outcome {
        Outcome {
            holds: false,
            present: false,
            why: Some(why),
        }
    }
    fn wrong(why: String) -> Outcome {
        Outcome {
            holds: false,
            present: true,
            why: Some(why),
        }
    }
}

fn eval(f: &Facet, id: u64, m: &IfcModel) -> Outcome {
    let Some(e) = m.get(id) else {
        return Outcome::absent("no existe".into());
    };
    match f {
        Facet::Entity {
            name,
            predefined_type,
        } => {
            if !m.classes(id).iter().any(|c| matches_text(name, c)) {
                return Outcome::wrong(format!(
                    "es {}, se pide la clase {name}",
                    crate::model::class_display(&e.kind)
                ));
            }
            if let Some(p) = predefined_type {
                let types = m.predefined_type(id);
                if types.is_empty() {
                    return Outcome::wrong(format!("no tiene tipo predefinido, se pide {p}"));
                }
                if !types.iter().any(|t| matches_text(p, t)) {
                    return Outcome::wrong(format!(
                        "su tipo predefinido es «{}», se pide {p}",
                        types[0]
                    ));
                }
            }
            Outcome::yes()
        }
        Facet::Attribute { name, value } => {
            let attrs = m.schema.attributes(&e.kind);
            let mut any_name = false;
            let mut present = false;
            for (i, a) in attrs.iter().enumerate() {
                if !matches_text(name, a) {
                    continue;
                }
                any_name = true;
                let raw = e.arg(i);
                let (found, _) = crate::model::found(raw, 1.0);
                let Some(found) = found else {
                    // An empty string or list is there, but says nothing.
                    let empty = matches!(
                        raw,
                        archisim_ifc::step_read::Value::Str(_)
                            | archisim_ifc::step_read::Value::List(_)
                    ) || matches!(raw, archisim_ifc::step_read::Value::Typed(_, v) if matches!(**v, archisim_ifc::step_read::Value::Str(_)));
                    if empty {
                        present = true;
                    }
                    continue;
                };
                if matches!(found, Found::Logical(None)) {
                    present = true;
                    continue;
                }
                present = true;
                match value {
                    None => return Outcome::yes(),
                    Some(v) => {
                        if matches_found(v, &found) {
                            return Outcome::yes();
                        }
                    }
                }
            }
            if !any_name {
                Outcome::absent(format!("no tiene el atributo {name}"))
            } else if !present {
                Outcome::absent(format!("el atributo {name} está vacío"))
            } else if value.is_none() {
                Outcome::wrong(format!("el atributo {name} está vacío"))
            } else {
                Outcome::wrong(format!(
                    "el atributo {name} no vale {}",
                    value.as_ref().map(|v| v.to_string()).unwrap_or_default()
                ))
            }
        }
        Facet::Property {
            property_set,
            base_name,
            value,
            data_type,
        } => eval_property(
            m,
            id,
            property_set,
            base_name,
            value.as_ref(),
            data_type.as_deref(),
        ),
        Facet::Material { value } => match m.material_values(id) {
            None => Outcome::absent("no tiene material".into()),
            Some(names) => match value {
                None => Outcome::yes(),
                Some(v) => {
                    if names.iter().any(|n| matches_text(v, n)) {
                        Outcome::yes()
                    } else if names.is_empty() {
                        Outcome::wrong("su material no tiene nombre".into())
                    } else {
                        Outcome::wrong(format!(
                            "sus materiales son {}, se pide {v}",
                            names.join(", ")
                        ))
                    }
                }
            },
        },
        Facet::Classification { system, value } => match m.classifications(id) {
            None => Outcome::absent("no está clasificado".into()),
            Some(list) => {
                let ok = list.iter().any(|(sys, refs)| {
                    let sys_ok = match system {
                        None => true,
                        Some(s) => sys.as_deref().is_some_and(|n| matches_text(s, n)),
                    };
                    let val_ok = match value {
                        None => true,
                        Some(v) => refs.iter().any(|r| matches_text(v, r)),
                    };
                    sys_ok && val_ok
                });
                if ok {
                    Outcome::yes()
                } else {
                    let have: Vec<String> = list
                        .iter()
                        .map(|(s, r)| {
                            format!(
                                "{} {}",
                                s.as_deref().unwrap_or("?"),
                                r.first().map(String::as_str).unwrap_or("")
                            )
                        })
                        .collect();
                    Outcome::wrong(format!(
                        "su clasificación es {}, se pide {}",
                        have.join(", "),
                        f.describe()
                    ))
                }
            }
        },
        Facet::PartOf {
            relation,
            name,
            predefined_type,
        } => {
            let parents = m.parents(id, relation.as_deref());
            if parents.is_empty() {
                return Outcome::absent(format!("no está {}", f.describe()));
            }
            let ok = parents.iter().any(|&p| {
                let Some(pe) = m.get(p) else { return false };
                matches_text(name, &pe.kind)
                    && predefined_type
                        .as_ref()
                        .is_none_or(|t| m.predefined_type(p).iter().any(|pt| matches_text(t, pt)))
            });
            if ok {
                Outcome::yes()
            } else {
                Outcome::wrong(format!("no está {}", f.describe()))
            }
        }
    }
}

fn eval_property(
    m: &IfcModel,
    id: u64,
    pset: &ValueSpec,
    base: &ValueSpec,
    value: Option<&ValueSpec>,
    data_type: Option<&str>,
) -> Outcome {
    let sets = m.property_sets(id);
    let matched: Vec<_> = sets.iter().filter(|(n, _)| matches_text(pset, n)).collect();
    if matched.is_empty() {
        return Outcome::absent(format!("no tiene el conjunto de propiedades {pset}"));
    }
    let mut any = false;
    for (set_name, view) in matched {
        let props: Vec<&Prop> = view
            .props
            .values()
            .filter(|p| matches_text(base, &p.name))
            .collect();
        if props.is_empty() {
            return Outcome::absent(format!("{set_name} no tiene la propiedad {base}"));
        }
        for p in props {
            if p.unsupported {
                return Outcome::wrong(format!(
                    "{set_name}.{} es una propiedad compleja o de referencia, que IDS no comprueba",
                    p.name
                ));
            }
            let all: Vec<&(Found, Option<String>)> = p
                .values
                .iter()
                .filter(|(f, _)| !matches!(f, Found::Logical(None)))
                .collect();
            if all.is_empty() {
                return Outcome::absent(format!("{set_name}.{} no tiene valor", p.name));
            }
            // Only the values of the requested type count.
            let values: Vec<&Found> = all
                .iter()
                .filter(|(_, t)| match (data_type, t) {
                    (Some(want), Some(t)) => t.eq_ignore_ascii_case(want),
                    _ => true,
                })
                .map(|(f, _)| f)
                .collect();
            if values.is_empty() {
                return Outcome::wrong(format!(
                    "{set_name}.{} es {}, se pide {}",
                    p.name,
                    all[0].1.as_deref().unwrap_or("?"),
                    data_type.unwrap_or("?")
                ));
            }
            if let Some(v) = value {
                if !values.iter().any(|f| matches_found(v, f)) {
                    return Outcome::wrong(format!(
                        "{set_name}.{} vale {}, se pide {v}",
                        p.name,
                        values
                            .iter()
                            .map(|f| show(f))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            any = true;
        }
    }
    if any {
        Outcome::yes()
    } else {
        Outcome::absent(format!("no tiene la propiedad {base}"))
    }
}

fn show(f: &Found) -> String {
    match f {
        Found::Text(s) | Found::Enum(s) => format!("«{s}»"),
        Found::Number(n, _) => format!("{n}"),
        Found::Logical(Some(b)) => if *b { "verdadero" } else { "falso" }.into(),
        Found::Logical(None) => "desconocido".into(),
        Found::Object => "un objeto".into(),
    }
}

/// Whether a name (class, attribute, property set…) satisfies a value.
fn matches_text(v: &ValueSpec, s: &str) -> bool {
    matches_found(v, &Found::Text(s.to_string()))
}

const EPS: f64 = 1e-6;

fn approx_eq(expected: f64, x: f64) -> bool {
    let lo = expected - expected.abs() * EPS - EPS;
    let hi = expected + expected.abs() * EPS + EPS;
    // Inclusive, with room for the rounding of the bounds themselves.
    let slack = (expected.abs() + 1.0) * 4.0 * f64::EPSILON;
    lo - slack <= x && x <= hi + slack
}

/// An IDS number: decimal digits with an optional sign, point and
/// exponent (`1`, `-1.5`, `1.`, `.5`, `1e3`). No `inf`, `NaN`, spaces or
/// thousands separators.
fn ids_number(s: &str) -> Option<(f64, bool)> {
    let t = s;
    let body = t.strip_prefix(['-', '+']).unwrap_or(t);
    let (mant, exp) = match body.find(['e', 'E']) {
        Some(i) => (&body[..i], Some(&body[i + 1..])),
        None => (body, None),
    };
    let digits_ok = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_ascii_digit());
    let (int, frac) = match mant.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (mant, None),
    };
    let mant_ok = match frac {
        None => digits_ok(int),
        Some(f) => {
            (int.is_empty() || digits_ok(int))
                && (f.is_empty() || digits_ok(f))
                && !(int.is_empty() && f.is_empty())
        }
    };
    if !mant_ok {
        return None;
    }
    if let Some(e) = exp {
        let e = e.strip_prefix(['-', '+']).unwrap_or(e);
        if !digits_ok(e) {
            return None;
        }
    }
    let is_int = frac.is_none() && exp.is_none();
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(|v| (v, is_int))
}

/// Whether a value found in the file satisfies an IDS value.
pub fn matches_found(v: &ValueSpec, f: &Found) -> bool {
    match v {
        ValueSpec::Simple(s) => simple_matches(s, f),
        ValueSpec::Restriction(r) => restriction_matches(r, f),
    }
}

fn simple_matches(s: &str, f: &Found) -> bool {
    match f {
        Found::Text(t) | Found::Enum(t) => t == s,
        Found::Number(n, is_int) => match ids_number(s) {
            // An integer in the file cannot equal a number with decimals.
            Some((x, spec_int)) => {
                if *is_int && !spec_int {
                    return false;
                }
                if *is_int {
                    x == *n
                } else {
                    approx_eq(x, *n)
                }
            }
            None => false,
        },
        Found::Logical(Some(b)) => match s {
            "true" => *b,
            "false" => !*b,
            _ => false,
        },
        Found::Logical(None) | Found::Object => false,
    }
}

fn restriction_matches(r: &Restriction, f: &Found) -> bool {
    if !r.enumeration.is_empty() && !r.enumeration.iter().any(|e| simple_matches(e, f)) {
        return false;
    }
    let text = match f {
        Found::Text(t) | Found::Enum(t) => Some(t.as_str()),
        _ => None,
    };
    if !r.patterns.is_empty() {
        match text {
            Some(t) if r.patterns.iter().any(|p| p.is_match(t)) => {}
            _ => return false,
        }
    }
    let bounds = [
        r.min_inclusive,
        r.max_inclusive,
        r.min_exclusive,
        r.max_exclusive,
    ];
    if bounds.iter().any(Option::is_some) {
        let Found::Number(n, _) = f else {
            return false;
        };
        let n = *n;
        if r.min_inclusive.is_some_and(|b| n < b)
            || r.max_inclusive.is_some_and(|b| n > b)
            || r.min_exclusive.is_some_and(|b| n <= b)
            || r.max_exclusive.is_some_and(|b| n >= b)
        {
            return false;
        }
    }
    if r.length.is_some() || r.min_length.is_some() || r.max_length.is_some() {
        let Some(t) = text else { return false };
        let len = t.chars().count();
        if r.length.is_some_and(|l| len != l)
            || r.min_length.is_some_and(|l| len < l)
            || r.max_length.is_some_and(|l| len > l)
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_follow_ids_format_and_tolerance() {
        assert_eq!(ids_number("1."), Some((1.0, false)));
        assert_eq!(ids_number("42"), Some((42.0, true)));
        assert_eq!(ids_number("1e3"), Some((1000.0, false)));
        assert!(ids_number("1,5").is_none());
        assert!(ids_number("inf").is_none());
        assert!(ids_number(" 1").is_none());
        assert!(simple_matches("1", &Found::Number(1.000002 - 1e-9, false)));
        assert!(!simple_matches("1", &Found::Number(1.0000021, false)));
        assert!(!simple_matches("1.5", &Found::Number(1.0, true)));
        assert!(simple_matches("true", &Found::Logical(Some(true))));
        assert!(!simple_matches("TRUE", &Found::Logical(Some(true))));
    }
}
