//! The content of an IDS file: specifications, their applicability and
//! requirements as facets, and the values facets ask for (a simple value
//! or an XML Schema restriction).

use regex::Regex;

/// A whole IDS document.
#[derive(Debug, Clone, Default)]
pub struct Ids {
    pub title: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub specifications: Vec<Specification>,
}

/// How many applicable elements a specification wants, from the
/// `minOccurs`/`maxOccurs` of its applicability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecCardinality {
    /// At least one applicable element must exist.
    Required,
    /// Any number, including none.
    Optional,
    /// No element may be applicable.
    Prohibited,
}

#[derive(Debug, Clone)]
pub struct Specification {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub identifier: String,
    /// Schemas it applies to (`IFC2X3`, `IFC4`, `IFC4X3_ADD2`).
    pub ifc_versions: Vec<String>,
    pub cardinality: SpecCardinality,
    pub applicability: Vec<Facet>,
    pub requirements: Vec<Requirement>,
    /// Why the specification itself is not valid IDS, if it is not (a
    /// prohibited specification with requirements…). An invalid
    /// specification always fails.
    pub invalid: Option<String>,
}

/// How a requirement facet must hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetCardinality {
    Required,
    /// If the information is there, it must satisfy the facet.
    Optional,
    /// The facet must not hold.
    Prohibited,
}

#[derive(Debug, Clone)]
pub struct Requirement {
    pub facet: Facet,
    pub cardinality: FacetCardinality,
    /// Free text for the person filling the model.
    pub instructions: String,
}

#[derive(Debug, Clone)]
pub enum Facet {
    Entity {
        name: ValueSpec,
        predefined_type: Option<ValueSpec>,
    },
    Attribute {
        name: ValueSpec,
        value: Option<ValueSpec>,
    },
    Property {
        property_set: ValueSpec,
        base_name: ValueSpec,
        value: Option<ValueSpec>,
        data_type: Option<String>,
    },
    Classification {
        system: Option<ValueSpec>,
        value: Option<ValueSpec>,
    },
    Material {
        value: Option<ValueSpec>,
    },
    PartOf {
        relation: Option<String>,
        name: ValueSpec,
        predefined_type: Option<ValueSpec>,
    },
}

impl Facet {
    /// A short Spanish description, for reports: «propiedad Pset_WallCommon.FireRating = EI 60».
    pub fn describe(&self) -> String {
        match self {
            Facet::Entity {
                name,
                predefined_type,
            } => match predefined_type {
                Some(p) => format!("clase {name} de tipo predefinido {p}"),
                None => format!("clase {name}"),
            },
            Facet::Attribute { name, value } => match value {
                Some(v) => format!("atributo {name} = {v}"),
                None => format!("atributo {name}"),
            },
            Facet::Property {
                property_set,
                base_name,
                value,
                data_type,
            } => {
                let mut s = format!("propiedad {property_set}.{base_name}");
                if let Some(v) = value {
                    s += &format!(" = {v}");
                }
                if let Some(t) = data_type {
                    s += &format!(" ({t})");
                }
                s
            }
            Facet::Classification { system, value } => match (system, value) {
                (Some(s), Some(v)) => format!("clasificación {s} {v}"),
                (Some(s), None) => format!("clasificación {s}"),
                (None, Some(v)) => format!("clasificación {v}"),
                (None, None) => "alguna clasificación".into(),
            },
            Facet::Material { value } => match value {
                Some(v) => format!("material {v}"),
                None => "algún material".into(),
            },
            Facet::PartOf {
                relation,
                name,
                predefined_type,
            } => {
                let rel = match relation.as_deref() {
                    Some("IFCRELAGGREGATES") => "parte de",
                    Some("IFCRELCONTAINEDINSPATIALSTRUCTURE") => "contenido en",
                    Some("IFCRELASSIGNSTOGROUP") => "del grupo",
                    Some("IFCRELNESTS") => "anidado en",
                    Some(_) => "alojado en",
                    None => "dentro de",
                };
                match predefined_type {
                    Some(p) => format!("{rel} {name} ({p})"),
                    None => format!("{rel} {name}"),
                }
            }
        }
    }
}

/// What a facet parameter accepts.
#[derive(Debug, Clone)]
pub enum ValueSpec {
    Simple(String),
    Restriction(Restriction),
}

/// An `xs:restriction`: every kind of constraint given must hold; the
/// enumeration values and the patterns are alternatives among themselves.
#[derive(Debug, Clone, Default)]
pub struct Restriction {
    /// `xs:string`, `xs:double`…, without the prefix.
    pub base: String,
    pub enumeration: Vec<String>,
    pub patterns: Vec<Pattern>,
    pub min_inclusive: Option<f64>,
    pub max_inclusive: Option<f64>,
    pub min_exclusive: Option<f64>,
    pub max_exclusive: Option<f64>,
    pub length: Option<usize>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
}

/// An XML Schema pattern (anchored at both ends). `None` if it could not
/// be compiled: it then matches nothing.
#[derive(Debug, Clone)]
pub struct Pattern {
    pub source: String,
    pub regex: Option<Regex>,
}

impl Pattern {
    pub fn new(source: &str) -> Pattern {
        Pattern {
            source: source.to_string(),
            regex: Regex::new(&format!("^(?:{})$", xsd_to_regex(source))).ok(),
        }
    }

    pub fn is_match(&self, s: &str) -> bool {
        self.regex.as_ref().is_some_and(|r| r.is_match(s))
    }
}

/// XML Schema regular expressions are close to Rust's; the differences
/// that matter are the `\i`/`\c` name classes and character class
/// subtraction, which are approximated.
fn xsd_to_regex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('i') => out.push_str(r"[\p{L}_:]"),
                Some('I') => out.push_str(r"[^\p{L}_:]"),
                Some('c') => out.push_str(r"[\p{L}\p{N}._:\-]"),
                Some('C') => out.push_str(r"[^\p{L}\p{N}._:\-]"),
                Some(n) => {
                    out.push('\\');
                    out.push(n);
                }
                None => out.push_str(r"\\"),
            }
        } else {
            out.push(c);
        }
    }
    out
}

impl std::fmt::Display for ValueSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueSpec::Simple(s) => write!(f, "«{s}»"),
            ValueSpec::Restriction(r) => {
                let mut parts = Vec::new();
                if !r.enumeration.is_empty() {
                    parts.push(format!("uno de {}", r.enumeration.join(" | ")));
                }
                for p in &r.patterns {
                    parts.push(format!("patrón {}", p.source));
                }
                if let Some(v) = r.min_inclusive {
                    parts.push(format!("≥ {v}"));
                }
                if let Some(v) = r.min_exclusive {
                    parts.push(format!("> {v}"));
                }
                if let Some(v) = r.max_inclusive {
                    parts.push(format!("≤ {v}"));
                }
                if let Some(v) = r.max_exclusive {
                    parts.push(format!("< {v}"));
                }
                if let Some(v) = r.length {
                    parts.push(format!("{v} caracteres"));
                }
                if let Some(v) = r.min_length {
                    parts.push(format!("al menos {v} caracteres"));
                }
                if let Some(v) = r.max_length {
                    parts.push(format!("como mucho {v} caracteres"));
                }
                write!(f, "[{}]", parts.join(", "))
            }
        }
    }
}

/// Why an IDS file could not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct IdsError(pub String);

impl std::fmt::Display for IdsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for IdsError {}

type Node<'a, 'i> = roxmltree::Node<'a, 'i>;

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn text(n: Option<Node>) -> String {
    n.and_then(|n| n.text()).unwrap_or("").trim().to_string()
}

/// Reads an IDS 1.0 document (also the 0.9 drafts, which differ only in
/// details this reader does not depend on).
pub fn parse_ids(xml: &str) -> Result<Ids, IdsError> {
    let doc =
        roxmltree::Document::parse(xml).map_err(|e| IdsError(format!("XML no válido: {e}")))?;
    let root = doc.root_element();
    if root.tag_name().name() != "ids" {
        return Err(IdsError(format!(
            "no es un fichero IDS (empieza por <{}>)",
            root.tag_name().name()
        )));
    }
    let mut ids = Ids::default();
    if let Some(info) = child(root, "info") {
        ids.title = text(child(info, "title"));
        ids.description = text(child(info, "description"));
        ids.author = text(child(info, "author"));
        ids.version = text(child(info, "version"));
    }
    let specs = child(root, "specifications")
        .ok_or_else(|| IdsError("el fichero IDS no tiene <specifications>".into()))?;
    for s in children(specs, "specification") {
        ids.specifications.push(read_specification(s));
    }
    Ok(ids)
}

fn read_specification(s: Node) -> Specification {
    let mut invalid = None;
    let mut note = |why: String| {
        if invalid.is_none() {
            invalid = Some(why);
        }
    };
    let app = child(s, "applicability");
    // IDS 1.0 puts the cardinality on <applicability>; the 0.9 drafts on
    // <specification>.
    let occurs = |name: &str| -> Option<String> {
        app.and_then(|a| a.attribute(name))
            .or_else(|| s.attribute(name))
            .map(str::to_string)
    };
    // As in XML Schema, an absent minOccurs is 1.
    let min = occurs("minOccurs").unwrap_or_else(|| "1".into());
    let max = occurs("maxOccurs").unwrap_or_else(|| "unbounded".into());
    let cardinality = if max == "0" {
        SpecCardinality::Prohibited
    } else if min.parse::<u64>().unwrap_or(0) >= 1 {
        SpecCardinality::Required
    } else {
        SpecCardinality::Optional
    };
    let mut applicability = Vec::new();
    if let Some(app) = app {
        for f in app.children().filter(|c| c.is_element()) {
            match read_facet(f) {
                Ok(facet) => applicability.push(facet),
                Err(why) => note(why),
            }
        }
    }
    let mut requirements = Vec::new();
    if let Some(req) = child(s, "requirements") {
        for f in req.children().filter(|c| c.is_element()) {
            let cardinality = match f.attribute("cardinality") {
                Some("optional") => FacetCardinality::Optional,
                Some("prohibited") => FacetCardinality::Prohibited,
                _ => FacetCardinality::Required,
            };
            match read_facet(f) {
                Ok(facet) => requirements.push(Requirement {
                    facet,
                    cardinality,
                    instructions: f.attribute("instructions").unwrap_or("").to_string(),
                }),
                Err(why) => note(why),
            }
        }
    }
    if cardinality == SpecCardinality::Prohibited && !requirements.is_empty() {
        note("una especificación prohibida no puede tener requisitos".into());
    }
    Specification {
        name: s.attribute("name").unwrap_or("").to_string(),
        description: s.attribute("description").unwrap_or("").to_string(),
        instructions: s.attribute("instructions").unwrap_or("").to_string(),
        identifier: s.attribute("identifier").unwrap_or("").to_string(),
        ifc_versions: s
            .attribute("ifcVersion")
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_uppercase)
            .collect(),
        cardinality,
        applicability,
        requirements,
        invalid,
    }
}

fn value(n: Option<Node>) -> Result<Option<ValueSpec>, String> {
    let Some(n) = n else {
        return Ok(None);
    };
    if let Some(sv) = child(n, "simpleValue") {
        return Ok(Some(ValueSpec::Simple(sv.text().unwrap_or("").to_string())));
    }
    if let Some(r) = child(n, "restriction") {
        let base = r
            .attribute("base")
            .unwrap_or("xs:string")
            .rsplit(':')
            .next()
            .unwrap_or("string")
            .to_string();
        let mut res = Restriction {
            base,
            ..Restriction::default()
        };
        for c in r.children().filter(|c| c.is_element()) {
            let v = c.attribute("value").unwrap_or("");
            let num = || -> Result<f64, String> {
                v.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("límite no numérico «{v}»"))
            };
            let len = || -> Result<usize, String> {
                v.trim()
                    .parse::<usize>()
                    .map_err(|_| format!("longitud no válida «{v}»"))
            };
            match c.tag_name().name() {
                "enumeration" => res.enumeration.push(v.to_string()),
                "pattern" => res.patterns.push(Pattern::new(v)),
                "minInclusive" => res.min_inclusive = Some(num()?),
                "maxInclusive" => res.max_inclusive = Some(num()?),
                "minExclusive" => res.min_exclusive = Some(num()?),
                "maxExclusive" => res.max_exclusive = Some(num()?),
                "length" => res.length = Some(len()?),
                "minLength" => res.min_length = Some(len()?),
                "maxLength" => res.max_length = Some(len()?),
                _ => {}
            }
        }
        return Ok(Some(ValueSpec::Restriction(res)));
    }
    Err(format!("<{}> sin valor", n.tag_name().name()))
}

fn required(n: Node, name: &str) -> Result<ValueSpec, String> {
    value(child(n, name))?
        .ok_or_else(|| format!("a la faceta <{}> le falta <{name}>", n.tag_name().name()))
}

fn read_facet(f: Node) -> Result<Facet, String> {
    Ok(match f.tag_name().name() {
        "entity" => Facet::Entity {
            name: required(f, "name")?,
            predefined_type: value(child(f, "predefinedType"))?,
        },
        "attribute" => Facet::Attribute {
            name: required(f, "name")?,
            value: value(child(f, "value"))?,
        },
        "property" => Facet::Property {
            property_set: required(f, "propertySet")?,
            // 0.9 drafts called it <name>.
            base_name: match child(f, "baseName") {
                Some(_) => required(f, "baseName")?,
                None => required(f, "name")?,
            },
            value: value(child(f, "value"))?,
            data_type: f
                .attribute("dataType")
                .or_else(|| f.attribute("datatype"))
                .map(str::to_string),
        },
        "classification" => Facet::Classification {
            system: value(child(f, "system"))?,
            value: value(child(f, "value"))?,
        },
        "material" => Facet::Material {
            value: value(child(f, "value"))?,
        },
        "partOf" => {
            let entity = child(f, "entity")
                .ok_or_else(|| "a la faceta <partOf> le falta <entity>".to_string())?;
            Facet::PartOf {
                relation: f.attribute("relation").map(str::to_uppercase),
                name: required(entity, "name")?,
                predefined_type: value(child(entity, "predefinedType"))?,
            }
        }
        other => return Err(format!("faceta desconocida <{other}>")),
    })
}
