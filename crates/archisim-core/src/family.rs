//! Families in a project and the system families that ship with ArchiSim.
//!
//! A project keeps a full copy of the recipe of every family it uses (as
//! Revit keeps its loaded families), so it opens the same on any computer.
//! Shared pieces (`include "piezas"`) are expanded into that copy.
//!
//! The system families reproduce ArchiSim's door and window types: the
//! types of older projects, of the template and of imported IFC files are
//! converted to them (see [`legacy_family`]).

use std::fmt;
use std::sync::Arc;

pub use archisim_family::{
    Evaluation, FamilyDef, FamilyError, Host, Param, ParamKind, ParamScope, Part, Value, Values,
};

use crate::ids::FamilyId;
use crate::model::{DoorOperation, OpeningKind, WindowOperation};

/// A family loaded in a project: its recipe text and the parsed recipe.
#[derive(Clone)]
pub struct Family {
    pub id: FamilyId,
    /// The recipe, with its shared pieces expanded.
    pub source: String,
    def: Arc<FamilyDef>,
}

impl Family {
    /// Reads a recipe. Shared pieces must already be expanded.
    pub fn new(id: FamilyId, source: impl Into<String>) -> Result<Self, FamilyError> {
        // One form of the text, so that saving and opening keep it equal.
        let mut source = source.into().replace("\r\n", "\n");
        if !source.ends_with('\n') {
            source.push('\n');
        }
        let def = archisim_family::parse(&source)?;
        Ok(Self {
            id,
            source,
            def: Arc::new(def),
        })
    }

    pub fn name(&self) -> &str {
        &self.def.name
    }

    pub fn version(&self) -> u32 {
        self.def.version
    }

    pub fn def(&self) -> &FamilyDef {
        &self.def
    }

    /// Whether it makes doors or windows hosted in walls.
    pub fn is_opening(&self) -> bool {
        self.def.host == Host::Wall && matches!(self.def.category.as_str(), "door" | "window")
    }
}

impl fmt::Debug for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Family({}, «{}» v{})",
            self.id.0,
            self.name(),
            self.version()
        )
    }
}

impl PartialEq for Family {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.source == other.source
    }
}

/// Shared pieces of the system families, by the name used in `include`.
pub const SHARED_PIECES: [(&str, &str); 1] = [("piezas", include_str!("../families/piezas.asfam"))];

/// Recipes of the system families, as written (with their `include`s).
const SYSTEM_SOURCES: [&str; 5] = [
    include_str!("../families/puerta_abatible.asfam"),
    include_str!("../families/puerta_corredera.asfam"),
    include_str!("../families/ventana_fija.asfam"),
    include_str!("../families/ventana_practicable.asfam"),
    include_str!("../families/ventana_corredera.asfam"),
];

pub const SWING_DOOR: &str = "Puerta abatible";
pub const SLIDING_DOOR: &str = "Puerta corredera";
pub const FIXED_WINDOW: &str = "Ventana fija";
pub const CASEMENT_WINDOW: &str = "Ventana practicable";
pub const SLIDING_WINDOW: &str = "Ventana corredera";

/// Expands the shared pieces of the system library in a recipe. Line ends
/// come out as `\n` whatever the files had (a Windows checkout may give
/// CRLF).
pub fn expand_shared_pieces(source: &str) -> Result<String, FamilyError> {
    let expanded = archisim_family::expand_includes(source, |name| {
        SHARED_PIECES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, text)| text.to_string())
    })?;
    Ok(expanded.replace('\r', ""))
}

/// The system families: `(name, recipe with its pieces expanded)`.
pub fn system_families() -> Vec<(String, String)> {
    SYSTEM_SOURCES
        .iter()
        .map(|src| {
            let source = expand_shared_pieces(src).expect("system families expand");
            let name = archisim_family::parse(&source)
                .expect("system families parse")
                .name;
            (name, source)
        })
        .collect()
}

/// The recipe of one system family, by name.
pub fn system_family(name: &str) -> Option<String> {
    system_families()
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, s)| s)
}

/// The system family that makes a door or window of the given operation,
/// and the number of leaves to set, if that family has the parameter.
pub fn legacy_family(kind: OpeningKind) -> (&'static str, Option<f64>) {
    match kind {
        OpeningKind::Door(DoorOperation::SingleSwing) => (SWING_DOOR, Some(1.0)),
        OpeningKind::Door(DoorOperation::DoubleSwing) => (SWING_DOOR, Some(2.0)),
        OpeningKind::Door(DoorOperation::Sliding) => (SLIDING_DOOR, None),
        OpeningKind::Window(WindowOperation::Fixed) => (FIXED_WINDOW, None),
        OpeningKind::Window(WindowOperation::SingleCasement) => (CASEMENT_WINDOW, Some(1.0)),
        OpeningKind::Window(WindowOperation::DoubleCasement) => (CASEMENT_WINDOW, Some(2.0)),
        OpeningKind::Window(WindowOperation::Sliding) => (SLIDING_WINDOW, None),
    }
}

/// The operation named by a family's `operation` output.
pub fn operation_from_name(category: &str, name: &str) -> Option<OpeningKind> {
    Some(match (category, name) {
        ("door", "single_swing") => OpeningKind::Door(DoorOperation::SingleSwing),
        ("door", "double_swing") => OpeningKind::Door(DoorOperation::DoubleSwing),
        ("door", "sliding") => OpeningKind::Door(DoorOperation::Sliding),
        ("window", "fixed") => OpeningKind::Window(WindowOperation::Fixed),
        ("window", "single_casement") => OpeningKind::Window(WindowOperation::SingleCasement),
        ("window", "double_casement") => OpeningKind::Window(WindowOperation::DoubleCasement),
        ("window", "sliding") => OpeningKind::Window(WindowOperation::Sliding),
        _ => return None,
    })
}

/// Why a family cannot make door or window types.
#[derive(Debug, Clone, PartialEq)]
pub struct NotAnOpening {
    pub family: String,
    pub reason: String,
}

impl fmt::Display for NotAnOpening {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "family «{}» cannot make doors or windows: {}",
            self.family, self.reason
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::{CutPattern, Layer, LayerFunction, MaterialId, Point2, WallFunction};

    fn project() -> (Model, MaterialId, MaterialId, crate::WallId) {
        let mut m = Model::new("Familias");
        let lvl = m.add_level("Planta baja", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [200, 90, 60], CutPattern::Diagonal);
        let wood = m.add_material("Madera de pino", "wood", [196, 160, 112], CutPattern::Solid);
        let glass = m.add_material("Vidrio doble", "glass", [190, 222, 236], CutPattern::Solid);
        let wt = m
            .add_wall_type(
                "Muro 30",
                WallFunction::Exterior,
                vec![Layer {
                    material: brick,
                    thickness: 0.3,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let wall = m
            .add_wall(NewWall {
                wall_type: wt,
                level: lvl,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(10.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
            })
            .unwrap();
        (m, wood, glass, wall)
    }

    const KINDS: [OpeningKind; 7] = [
        OpeningKind::Door(DoorOperation::SingleSwing),
        OpeningKind::Door(DoorOperation::DoubleSwing),
        OpeningKind::Door(DoorOperation::Sliding),
        OpeningKind::Window(WindowOperation::Fixed),
        OpeningKind::Window(WindowOperation::SingleCasement),
        OpeningKind::Window(WindowOperation::DoubleCasement),
        OpeningKind::Window(WindowOperation::Sliding),
    ];

    fn legacy(kind: OpeningKind, a: MaterialId, b: MaterialId) -> NewOpeningType {
        NewOpeningType {
            name: format!("{kind:?}"),
            kind,
            width: 1.37,
            height: 2.03,
            frame_width: 0.065,
            frame_depth: 0.083,
            frame_material: a,
            panel_material: b,
        }
    }

    #[test]
    fn system_families_parse_and_are_complete() {
        let fams = system_families();
        let names: Vec<_> = fams.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                SWING_DOOR,
                SLIDING_DOOR,
                FIXED_WINDOW,
                CASEMENT_WINDOW,
                SLIDING_WINDOW
            ]
        );
        for (_, src) in &fams {
            let f = Family::new(crate::FamilyId(1), src.clone()).unwrap();
            assert!(f.is_opening(), "{}", f.name());
            assert!(!src.contains("\ninclude"));
            // Same text on every system, whatever line ends the checkout has.
            assert!(!src.contains('\r'));
            for p in f.def().params.iter() {
                assert!(!p.label.is_empty() && p.section.is_some(), "{}", p.name);
            }
        }
    }

    #[test]
    fn every_old_type_becomes_the_same_type_of_a_family() {
        let (mut m, wood, glass) = {
            let (m, w, g, _) = project();
            (m, w, g)
        };
        for kind in KINDS {
            let spec = legacy(kind, wood, glass);
            let id = m.add_opening_type(spec.clone()).unwrap();
            let t = m.opening_type(id).unwrap();
            let fam = m.family(t.family).unwrap();
            assert_eq!(fam.name(), legacy_family(kind).0);
            // What the rest of the program reads is exactly what was asked.
            assert_eq!(t.kind, kind);
            assert_eq!(t.name, spec.name);
            assert_eq!(
                (t.width, t.height, t.frame_width, t.frame_depth),
                (1.37, 2.03, 0.065, 0.083)
            );
            assert_eq!((t.frame_material, t.panel_material), (wood, glass));
        }
        // One copy of each system family, loaded on first use.
        assert_eq!(m.families().count(), 5);
        let marks: Vec<_> = m.opening_types().map(|t| t.mark.clone()).collect();
        assert_eq!(
            marks,
            ["P-01", "P-02", "P-03", "V-01", "V-02", "V-03", "V-04"]
        );
    }

    #[test]
    fn types_can_be_made_from_a_family_directly() {
        let (mut m, wood, _, wall) = project();
        let fam = m.system_family(CASEMENT_WINDOW).unwrap();
        let mut values = m.family_defaults(fam, ParamScope::Type).unwrap();
        // Defaults find the project's materials by name.
        assert_eq!(values["panel_material"], Value::Material(4));
        values.insert("leaves".into(), Value::Number(1.0));
        values.insert("width".into(), Value::Number(0.9));
        let t = m
            .add_family_type(NewFamilyType {
                family: fam,
                name: "V-practicable 90".into(),
                mark: Some("V-90".into()),
                description: "Dormitorios".into(),
                values: values.clone(),
            })
            .unwrap();
        let ty = m.opening_type(t).unwrap();
        assert_eq!(
            ty.kind,
            OpeningKind::Window(WindowOperation::SingleCasement)
        );
        assert_eq!((ty.width, ty.mark.as_str()), (0.9, "V-90"));

        // A repeated mark, a value out of range or a broken rule is refused.
        let refused = |m: &mut Model, mark: Option<&str>, key: &str, v: Value| {
            let mut vals = values.clone();
            vals.insert(key.into(), v);
            m.add_family_type(NewFamilyType {
                family: fam,
                name: "x".into(),
                mark: mark.map(str::to_string),
                description: String::new(),
                values: vals,
            })
            .unwrap_err()
        };
        assert_eq!(
            refused(&mut m, Some("V-90"), "width", Value::Number(1.0)),
            ModelError::DuplicateMark("V-90".into())
        );
        assert!(matches!(
            refused(&mut m, None, "width", Value::Number(25.0)),
            ModelError::Family(FamilyError::OutOfRange { .. })
        ));
        assert!(matches!(
            refused(&mut m, None, "frame_width", Value::Number(0.6)),
            ModelError::Family(FamilyError::Requirement { .. })
        ));
        assert_eq!(
            refused(&mut m, None, "frame_material", Value::Material(99)),
            ModelError::UnknownMaterial(MaterialId(99))
        );
        let _ = (wood, wall);
    }

    #[test]
    fn duplicating_editing_and_renaming_keep_identity() {
        let (mut m, wood, glass, wall) = project();
        let door = m
            .add_opening_type(legacy(
                OpeningKind::Door(DoorOperation::SingleSwing),
                wood,
                wood,
            ))
            .unwrap();
        let o = m
            .add_opening(NewOpening {
                opening_type: door,
                wall,
                along: 2.0,
                sill: 0.0,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })
            .unwrap();
        let guid = m.opening_type(door).unwrap().guid.clone();

        let copy = m.duplicate_opening_type(door, "Copia").unwrap();
        let c = m.opening_type(copy).unwrap();
        assert_eq!((c.name.as_str(), c.mark.as_str()), ("Copia", "P-02"));
        assert_ne!(c.guid, guid);
        assert_eq!(c.values, m.opening_type(door).unwrap().values);

        // Two leaves: same family, the door keeps its GUID and centre.
        let fam = m.opening_type(door).unwrap().family;
        let mut values = m.opening_type(door).unwrap().values.clone();
        values.insert("leaves".into(), Value::Number(2.0));
        m.edit_family_type(door, fam, values).unwrap();
        let t = m.opening_type(door).unwrap();
        assert_eq!(t.kind, OpeningKind::Door(DoorOperation::DoubleSwing));
        assert_eq!((t.guid.clone(), t.mark.as_str()), (guid.clone(), "P-01"));
        assert_eq!(m.opening(o).unwrap().along, 2.0);

        // The old-style edit can switch to another system family (sliding).
        let mut spec = legacy(OpeningKind::Door(DoorOperation::Sliding), wood, wood);
        spec.name = "Corredera".into();
        m.edit_opening_type(door, spec).unwrap();
        let t = m.opening_type(door).unwrap();
        assert_eq!(m.family(t.family).unwrap().name(), SLIDING_DOOR);
        assert_eq!((t.name.as_str(), t.mark.as_str()), ("Corredera", "P-01"));
        // …but a door never becomes a window.
        assert_eq!(
            m.edit_opening_type(
                door,
                legacy(OpeningKind::Window(WindowOperation::Fixed), wood, glass)
            ),
            Err(ModelError::CategoryMismatch)
        );

        m.set_opening_type_info(door, "Entrada", "P-10", "Acceso principal")
            .unwrap();
        assert_eq!(m.opening_type(door).unwrap().mark, "P-10");
        assert_eq!(
            m.set_opening_type_info(copy, "Copia", "P-10", ""),
            Err(ModelError::DuplicateMark("P-10".into()))
        );
        // Empty marks may repeat.
        m.set_opening_type_info(door, "Entrada", "", "").unwrap();
        m.set_opening_type_info(copy, "Copia", "", "").unwrap();
        // The next free mark fills the gaps.
        assert_eq!(
            m.next_mark(OpeningKind::Door(DoorOperation::SingleSwing)),
            "P-01"
        );
    }

    #[test]
    fn a_failed_type_does_not_leave_a_family_behind() {
        let (mut m, wood, _, _) = project();
        let mut spec = legacy(OpeningKind::Window(WindowOperation::Sliding), wood, wood);
        // Room for two frames but not for the mullion: refused by the family.
        spec.width = 0.15;
        spec.frame_width = 0.06;
        assert!(matches!(
            m.add_opening_type(spec),
            Err(ModelError::Family(FamilyError::Requirement { .. }))
        ));
        assert_eq!(m.families().count(), 0);
    }

    #[test]
    fn materials_in_family_values_are_in_use() {
        let (mut m, wood, glass, _) = project();
        m.add_opening_type(legacy(
            OpeningKind::Window(WindowOperation::Fixed),
            wood,
            glass,
        ))
        .unwrap();
        assert_eq!(m.material_users(glass), ["Window(Fixed)"]);
        assert!(m.remove_material(glass).is_err());
    }

    #[test]
    fn placed_openings_evaluate_with_their_side_and_hand() {
        let (mut m, wood, glass, wall) = project();
        let t = m
            .add_opening_type(legacy(
                OpeningKind::Window(WindowOperation::DoubleCasement),
                wood,
                glass,
            ))
            .unwrap();
        let o = m
            .add_opening(NewOpening {
                opening_type: t,
                wall,
                along: 3.0,
                sill: 0.9,
                opens_to: WallSide::Exterior,
                hand: Hand::Right,
                setback: None,
            })
            .unwrap();
        assert!(m.opening(o).unwrap().values.is_empty());
        let ev = m.evaluate_opening(o).unwrap();
        // Two jambs, two rails, one mullion and two panes.
        assert_eq!(ev.parts.len(), 7);
        let glass_parts = ev.parts.iter().filter(|p| p.material == glass.0).count();
        assert_eq!(glass_parts, 2);
        // Everything inside the rough opening.
        for p in &ev.parts {
            assert!(p.min[0] >= -1.37 / 2.0 - 1e-12 && p.max[0] <= 1.37 / 2.0 + 1e-12);
            assert!(p.min[2] >= 0.0 && p.max[2] <= 2.03 + 1e-12);
        }
        assert_eq!(
            opening_builtins(WallSide::Exterior, Hand::Right)["hand"],
            Value::Text("right".into())
        );
    }

    /// The example in `docs/FAMILIAS.md` is a working recipe.
    #[test]
    fn the_documented_example_works() {
        // Windows checkouts may turn the line ends into CRLF.
        let doc = include_str!("../../../docs/FAMILIAS.md").replace("\r\n", "\n");
        let start = doc.find("## El lenguaje").unwrap();
        let block = &doc[start..];
        let code_start = block.find("```\n").unwrap() + 4;
        let code_len = block[code_start..].find("```").unwrap();
        let recipe = expand_shared_pieces(&block[code_start..code_start + code_len]).unwrap();
        let fam = Family::new(crate::FamilyId(1), recipe).unwrap();
        let def = fam.def();
        let mut t = def.defaults(ParamScope::Type);
        t.insert("frame_width".into(), Value::Number(0.06));
        t.insert("frame_depth".into(), Value::Number(0.07));
        t.insert("frame_material".into(), Value::Material(1));
        t.insert("panel_material".into(), Value::Material(2));
        t.insert("shutter".into(), Value::Bool(true));
        let i = def.defaults(ParamScope::Instance);
        let b = opening_builtins(WallSide::Interior, Hand::Left);
        let ev = def.evaluate(&t, &i, &b).unwrap();
        // Frame (4), two panes and the shutter box.
        assert_eq!(ev.parts.len(), 7);
        assert_eq!(
            ev.outputs["operation"],
            Value::Text("double_casement".into())
        );
        assert!(def.is_visible("box_h", &t));
    }

    /// Every system family, over its whole range of options and at the ends
    /// of its ranges: the geometry is valid and stays in the rough opening.
    #[test]
    fn system_families_work_over_all_their_options() {
        let mut checked = 0;
        for (_, src) in system_families() {
            let fam = Family::new(crate::FamilyId(1), src).unwrap();
            let def = fam.def();
            let leaves: Vec<f64> = match def.param("leaves").and_then(|p| p.range) {
                Some((lo, hi)) => (lo as i64..=hi as i64).map(|n| n as f64).collect(),
                None => vec![0.0],
            };
            for &n in &leaves {
                for &w in &[0.3, 0.8, 1.6, 6.0, 20.0] {
                    for &h in &[0.3, 2.1, 20.0] {
                        for &f in &[0.001, 0.05, 0.09] {
                            for &d in &[0.001, 0.03, 0.07, 2.0] {
                                let mut v = def.defaults(ParamScope::Type);
                                v.insert("width".into(), Value::Number(w));
                                v.insert("height".into(), Value::Number(h));
                                v.insert("frame_width".into(), Value::Number(f));
                                v.insert("frame_depth".into(), Value::Number(d));
                                v.insert("frame_material".into(), Value::Material(1));
                                v.insert("panel_material".into(), Value::Material(2));
                                if n > 0.0 {
                                    v.insert("leaves".into(), Value::Number(n));
                                }
                                for side in [WallSide::Interior, WallSide::Exterior] {
                                    for hand in [Hand::Left, Hand::Right] {
                                        let b = opening_builtins(side, hand);
                                        let ev = match def.evaluate(&v, &Values::new(), &b) {
                                            Ok(ev) => ev,
                                            // Only the recipe's own rules may refuse.
                                            Err(FamilyError::Requirement { .. }) => continue,
                                            Err(e) => panic!("{} {v:?}: {e}", fam.name()),
                                        };
                                        checked += 1;
                                        assert!(!ev.parts.is_empty());
                                        for p in &ev.parts {
                                            for k in 0..3 {
                                                assert!(p.min[k] <= p.max[k]);
                                            }
                                            assert!(p.min[0] >= -w / 2.0 - 1e-9);
                                            assert!(p.max[0] <= w / 2.0 + 1e-9);
                                            assert!(p.min[1] >= -d / 2.0 - 1e-9);
                                            assert!(p.max[1] <= d / 2.0 + 1e-9);
                                            assert!(p.min[2] >= -1e-9 && p.max[2] <= h + 1e-9);
                                        }
                                        let kind = operation_from_name(
                                            &def.category,
                                            ev.outputs["operation"].as_text().unwrap(),
                                        );
                                        assert!(kind.is_some());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 1000, "{checked}");
    }
}
