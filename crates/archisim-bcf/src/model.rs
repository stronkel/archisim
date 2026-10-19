//! What a BCF file holds: topics (incidencias) with their comments and
//! viewpoints (camera, selected and hidden elements by `GlobalId`, colours,
//! clipping planes and a snapshot).

/// The BCF version a file was read from or is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BcfVersion {
    V2_1,
    #[default]
    V3_0,
}

impl BcfVersion {
    pub fn id(self) -> &'static str {
        match self {
            BcfVersion::V2_1 => "2.1",
            BcfVersion::V3_0 => "3.0",
        }
    }
}

/// A whole BCF file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bcf {
    pub version: BcfVersion,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub extensions: Extensions,
    pub topics: Vec<Topic>,
}

/// The values a project allows (BCF 3.0 `extensions.xml`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Extensions {
    pub topic_types: Vec<String>,
    pub topic_statuses: Vec<String>,
    pub priorities: Vec<String>,
    pub topic_labels: Vec<String>,
    pub stages: Vec<String>,
    pub users: Vec<String>,
}

impl Extensions {
    /// What ArchiSim offers in a new file, in Spanish, as a Spanish
    /// studio would set up BIMcollab or similar.
    pub fn archisim() -> Extensions {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect();
        Extensions {
            topic_types: v(&[
                "Interferencia",
                "Consulta",
                "Error",
                "Propuesta",
                "Requisito",
            ]),
            topic_statuses: v(&["Abierta", "En curso", "Resuelta", "Cerrada"]),
            priorities: v(&["Alta", "Normal", "Baja"]),
            topic_labels: v(&[
                "Arquitectura",
                "Estructura",
                "Instalaciones",
                "Urbanización",
            ]),
            stages: v(&[
                "Anteproyecto",
                "Proyecto básico",
                "Proyecto de ejecución",
                "Obra",
            ]),
            users: Vec::new(),
        }
    }
}

/// The model files a topic refers to (markup header).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileRef {
    pub ifc_project: Option<String>,
    pub ifc_spatial_element: Option<String>,
    pub is_external: bool,
    pub filename: Option<String>,
    pub date: Option<String>,
    pub reference: Option<String>,
}

/// One issue.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Topic {
    pub guid: String,
    pub topic_type: Option<String>,
    pub status: Option<String>,
    pub title: String,
    pub priority: Option<String>,
    pub index: Option<i64>,
    pub labels: Vec<String>,
    /// ISO 8601 dates, as written (`2026-10-19T17:30:00Z`).
    pub creation_date: String,
    pub creation_author: String,
    pub modified_date: Option<String>,
    pub modified_author: Option<String>,
    pub due_date: Option<String>,
    pub assigned_to: Option<String>,
    pub stage: Option<String>,
    pub description: Option<String>,
    pub reference_links: Vec<String>,
    pub related_topics: Vec<String>,
    pub files: Vec<FileRef>,
    pub comments: Vec<Comment>,
    pub viewpoints: Vec<Viewpoint>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Comment {
    pub guid: String,
    pub date: String,
    pub author: String,
    pub text: String,
    /// The viewpoint it talks about.
    pub viewpoint: Option<String>,
    pub modified_date: Option<String>,
    pub modified_author: Option<String>,
}

pub type V3 = [f64; 3];

#[derive(Debug, Clone, PartialEq)]
pub enum Camera {
    Perspective {
        position: V3,
        direction: V3,
        up: V3,
        /// Vertical field of view, degrees.
        field_of_view: f64,
        aspect_ratio: f64,
    },
    Orthogonal {
        position: V3,
        direction: V3,
        up: V3,
        /// Height of the view in metres.
        view_to_world_scale: f64,
        aspect_ratio: f64,
    },
}

impl Camera {
    pub fn position(&self) -> V3 {
        match self {
            Camera::Perspective { position, .. } | Camera::Orthogonal { position, .. } => *position,
        }
    }

    pub fn direction(&self) -> V3 {
        match self {
            Camera::Perspective { direction, .. } | Camera::Orthogonal { direction, .. } => {
                *direction
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClippingPlane {
    pub location: V3,
    /// Points to the side that is cut away.
    pub direction: V3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub start: V3,
    pub end: V3,
}

/// A saved view of the model.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewpoint {
    pub guid: String,
    pub index: Option<i64>,
    pub camera: Option<Camera>,
    /// Selected elements, by `GlobalId`.
    pub selection: Vec<String>,
    /// Whether elements are shown unless listed in `exceptions`.
    pub default_visibility: bool,
    pub exceptions: Vec<String>,
    pub spaces_visible: bool,
    pub openings_visible: bool,
    /// Colour (`RRGGBB` or `AARRGGBB`) and the elements painted with it.
    pub coloring: Vec<(String, Vec<String>)>,
    pub clipping_planes: Vec<ClippingPlane>,
    pub lines: Vec<Line>,
    /// PNG image of the view.
    pub snapshot: Option<Vec<u8>>,
}

impl Viewpoint {
    pub fn new(guid: String) -> Viewpoint {
        Viewpoint {
            guid,
            index: None,
            camera: None,
            selection: Vec::new(),
            default_visibility: true,
            exceptions: Vec::new(),
            spaces_visible: false,
            openings_visible: false,
            coloring: Vec::new(),
            clipping_planes: Vec::new(),
            lines: Vec::new(),
            snapshot: None,
        }
    }
}

/// A new GUID in the textual form BCF uses.
pub fn new_guid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// The current time as BCF writes it (UTC, seconds).
pub fn now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    iso_utc(secs)
}

/// Seconds since 1970 → `YYYY-MM-DDThh:mm:ssZ`.
pub fn iso_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil from days (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_792_423_800), "2026-10-19T15:30:00Z");
        assert_eq!(new_guid().len(), 36);
    }
}
