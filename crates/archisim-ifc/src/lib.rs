//! IFC4 export.
//!
//! Produces a STEP file with:
//! - the spatial structure Project → Site → Building → Storeys;
//! - materials with surface colours;
//! - wall types (`IfcWallType`) with their material layer sets;
//! - walls (`IfcWall`) with an axis, a swept-solid body following the
//!   resolved corners, a material layer set usage, `Pset_WallCommon` and
//!   `Qto_WallBaseQuantities`;
//! - slab types (`IfcSlabType`) and slabs (`IfcSlab`) extruded from their
//!   outline, with a material layer set usage, `Pset_SlabCommon` and
//!   `Qto_SlabBaseQuantities`;
//! - doors and windows: an `IfcOpeningElement` voiding the host wall
//!   (`IfcRelVoidsElement`), filled (`IfcRelFillsElement`) by an `IfcDoor`
//!   or `IfcWindow` with its type (`IfcDoorType` / `IfcWindowType`), a
//!   material constituent set (frame + leaf or glazing), a simple 3D body
//!   (frame, leaves and panes in their closed position), `Pset_DoorCommon` /
//!   `Pset_WindowCommon` and base quantities. Walls carry net side area and
//!   net volume once their openings are cut.
//!
//! - rooms (`IfcSpace`, see `spaces`), aggregated into their storey;
//! - column and beam types and members (see `members`), extruded from the
//!   type's profile with an `IfcMaterialProfileSet`.
//!
//! Layer set orientation: walls are placed with local X along the drawn line
//! and local Y to its left (the exterior side). The layer set usage is
//! `AXIS2 / POSITIVE`, starting at the interior face, so wall layer sets are
//! written from interior to exterior. Slabs are placed at their soffit and
//! extruded upwards; their usage is `AXIS3 / POSITIVE` from the soffit, so
//! slab layer sets are written from bottom to top.
//!
//! Door and window placement: each one is placed relative to its opening,
//! which is placed relative to the wall. Its local Y axis points to the side
//! the leaves open towards, as IFC expects for `OperationType`; left and
//! right in IFC are seen looking along +Y, the opposite of the UNE-EN 12519
//! handing used in ArchiSim, so they swap on export.
//!
//! Curved walls: the axis is an `IfcTrimmedCurve` on an `IfcCircle`, and the
//! body one extrusion of an `IfcIndexedPolyCurve` with exact arcs
//! (`IfcArcIndex`); their quantities are those of the exact arcs.
//!
//! Relationship GUIDs are derived from element GUIDs, so exporting the same
//! model twice gives the same file apart from the header timestamp.

pub mod import;
mod members;
mod roofs;
mod spaces;
pub mod step;
pub mod step_read;

pub use import::{differences, import_ifc, ImportError, ImportReport, Imported};

use std::collections::BTreeMap;

use archisim_core::{
    perimeter, DoorOperation, GlobalId, Hand, Layer, LayerFunction, LevelId, MaterialId, Model,
    OpeningKind, OpeningType, OpeningTypeId, Point2, SlabTypeId, WallFunction, WallTypeId, EPSILON,
};
use archisim_geom::{path_area, removed_area, signed_area, OpeningGeometry, PathSeg};
use step::{boolean, list, real, string, Ref, StepWriter};

/// Header information for the exported file.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub file_name: String,
    pub author: String,
    pub organization: String,
    /// ISO 8601 timestamp; `None` uses the current UTC time.
    pub timestamp: Option<String>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            file_name: "model.ifc".into(),
            author: String::new(),
            organization: String::new(),
            timestamp: None,
        }
    }
}

const APP: &str = concat!("ArchiSim ", env!("CARGO_PKG_VERSION"));

/// Exports the whole model as IFC4 text.
pub fn export_ifc(model: &Model, options: &ExportOptions) -> String {
    let mut x = Exporter::new(model);
    x.write();
    let timestamp = options.timestamp.clone().unwrap_or_else(utc_now);
    format!(
        "ISO-10303-21;\nHEADER;\n\
         FILE_DESCRIPTION(('ViewDefinition [DesignTransferView]'),'2;1');\n\
         FILE_NAME({},{},({}),({}),{},{},'');\n\
         FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n{}ENDSEC;\nEND-ISO-10303-21;\n",
        string(&options.file_name),
        string(&timestamp),
        string(&options.author),
        string(&options.organization),
        string(APP),
        string(APP),
        x.w.data()
    )
}

/// A GUID derived deterministically from another one (FNV-1a, 128 bits).
fn derived_guid(base: &GlobalId, salt: &str) -> GlobalId {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for b in base.as_str().bytes().chain([0u8]).chain(salt.bytes()) {
        h ^= b as u128;
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    GlobalId::from_u128(h)
}

struct Exporter<'a> {
    model: &'a Model,
    w: StepWriter,
    origin: Ref,
    z: Ref,
    body_ctx: Ref,
    axis_ctx: Ref,
    /// Surface style of each material, reused on door and window solids.
    styles: BTreeMap<MaterialId, Ref>,
}

impl<'a> Exporter<'a> {
    fn new(model: &'a Model) -> Self {
        let mut w = StepWriter::default();
        let origin = w.add("IFCCARTESIANPOINT((0.,0.,0.))");
        let z = w.add("IFCDIRECTION((0.,0.,1.))");
        Self {
            model,
            w,
            origin,
            z,
            body_ctx: Ref(0),
            axis_ctx: Ref(0),
            styles: BTreeMap::new(),
        }
    }

    fn point3(&mut self, x: f64, y: f64, z: f64) -> Ref {
        self.w.add(format!(
            "IFCCARTESIANPOINT(({},{},{}))",
            real(x),
            real(y),
            real(z)
        ))
    }

    fn point2(&mut self, p: Point2) -> Ref {
        self.w
            .add(format!("IFCCARTESIANPOINT(({},{}))", real(p.x), real(p.y)))
    }

    fn placement(&mut self, relative_to: Option<Ref>, location: Ref, x_dir: Option<Ref>) -> Ref {
        let ap = match x_dir {
            Some(d) => self
                .w
                .add(format!("IFCAXIS2PLACEMENT3D({location},{},{d})", self.z)),
            None => self.w.add(format!("IFCAXIS2PLACEMENT3D({location},$,$)")),
        };
        let rel = relative_to.map_or("$".to_string(), |r| r.to_string());
        self.w.add(format!("IFCLOCALPLACEMENT({rel},{ap})"))
    }

    /// Placement and shape of a slab (level or inclined): placed at the
    /// soffit and swept up by its thickness; inclined, a profile on the
    /// soffit in tilted axes swept plumb.
    fn slab_shape(
        &mut self,
        slab: &archisim_core::Slab,
        thickness: f64,
        storey_pl: Ref,
    ) -> (Ref, Ref) {
        let vertical = slab.vertical(thickness);
        let solid = match slab.slope.and_then(|s| s.direction().map(|u| (s, u))) {
            None => {
                let loc = self.point3(0.0, 0.0, slab.top_offset - thickness);
                let pl = self.placement(Some(storey_pl), loc, None);
                let profile = self.slab_profile(slab, &|p| p);
                let solid_pl = self
                    .w
                    .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
                let solid = self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                    self.z,
                    real(thickness)
                ));
                (pl, solid)
            }
            Some((slope, u)) => {
                // Inclined: the profile lies on the soffit, in axes
                // tilted with it (x up the slope, z square to it), and
                // is swept vertically, so the edges stay plumb.
                let loc = self.point3(0.0, 0.0, 0.0);
                let pl = self.placement(Some(storey_pl), loc, None);
                let k = slope.slope;
                let n = (1.0 + k * k).sqrt();
                let x3 = [u.x / n, u.y / n, k / n];
                let z3 = [-k * u.x / n, -k * u.y / n, 1.0 / n];
                let y3 = [-u.y, u.x, 0.0];
                let o = slope.from;
                let o3 = [o.x, o.y, slab.top_offset - vertical];
                let soffit = |p: Point2| slab.top_at(p) - vertical;
                let to_local = |p: Point2| {
                    let d = [p.x - o3[0], p.y - o3[1], soffit(p) - o3[2]];
                    Point2::new(
                        d[0] * x3[0] + d[1] * x3[1] + d[2] * x3[2],
                        d[0] * y3[0] + d[1] * y3[1] + d[2] * y3[2],
                    )
                };
                let profile = self.slab_profile(slab, &to_local);
                let at = self.point3(o3[0], o3[1], o3[2]);
                let axis = self.w.add(format!(
                    "IFCDIRECTION(({},{},{}))",
                    real(z3[0]),
                    real(z3[1]),
                    real(z3[2])
                ));
                let xdir = self.w.add(format!(
                    "IFCDIRECTION(({},{},{}))",
                    real(x3[0]),
                    real(x3[1]),
                    real(x3[2])
                ));
                let solid_pl = self
                    .w
                    .add(format!("IFCAXIS2PLACEMENT3D({at},{axis},{xdir})"));
                // The vertical in the tilted axes.
                let up = self.w.add(format!(
                    "IFCDIRECTION(({},0.,{}))",
                    real(k / n),
                    real(1.0 / n)
                ));
                let solid = self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{up},{})",
                    real(vertical)
                ));
                (pl, solid)
            }
        };
        let (pl, solid) = solid;
        let body_rep = self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
            self.body_ctx
        ));
        let shape = self
            .w
            .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body_rep}))"));
        (pl, shape)
    }

    /// The profile of a slab (its outline, with its holes as voids), each
    /// point taken to the profile's axes by `to_local`.
    fn slab_profile(
        &mut self,
        slab: &archisim_core::Slab,
        to_local: &dyn Fn(Point2) -> Point2,
    ) -> Ref {
        let local = |pts: &[Point2]| pts.iter().map(|p| to_local(*p)).collect::<Vec<_>>();
        let outline = self.polyline(&local(&slab.outline), true);
        if slab.holes.is_empty() {
            self.w
                .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{outline})"))
        } else {
            // Holes (stairs, shafts): the voids of the profile.
            let inner: Vec<Ref> = slab
                .holes
                .iter()
                .map(|h| self.polyline(&local(h), true))
                .collect();
            self.w.add(format!(
                "IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,{outline},{})",
                list(&inner)
            ))
        }
    }

    fn polyline(&mut self, pts: &[Point2], closed: bool) -> Ref {
        let mut refs: Vec<Ref> = pts.iter().map(|&p| self.point2(p)).collect();
        if closed {
            refs.push(refs[0]);
        }
        self.w.add(format!("IFCPOLYLINE({})", list(&refs)))
    }

    /// A closed `IfcIndexedPolyCurve` of straight pieces and exact arcs.
    fn indexed_curve(&mut self, path: &[PathSeg], to_local: &dyn Fn(Point2) -> Point2) -> Ref {
        let mut pts: Vec<Point2> = Vec::new();
        let index = |p: Point2, pts: &mut Vec<Point2>| -> usize {
            match pts.iter().position(|q| q.approx_eq(p)) {
                Some(i) => i + 1,
                None => {
                    pts.push(p);
                    pts.len()
                }
            }
        };
        let mut segs = Vec::new();
        for seg in path {
            match *seg {
                PathSeg::Line(a, b) => {
                    let (i, j) = (index(a, &mut pts), index(b, &mut pts));
                    segs.push(format!("IFCLINEINDEX(({i},{j}))"));
                }
                PathSeg::Arc(a, b, c) => {
                    let (i, j, k) = (index(a, &mut pts), index(b, &mut pts), index(c, &mut pts));
                    segs.push(format!("IFCARCINDEX(({i},{j},{k}))"));
                }
            }
        }
        let coords = pts
            .iter()
            .map(|p| {
                let l = to_local(*p);
                format!("({},{})", real(l.x), real(l.y))
            })
            .collect::<Vec<_>>()
            .join(",");
        let list = self.w.add(format!("IFCCARTESIANPOINTLIST2D(({coords}),$)"));
        self.w.add(format!(
            "IFCINDEXEDPOLYCURVE({list},({}),.F.)",
            segs.join(",")
        ))
    }

    /// An `IfcMaterialLayerSet` with the layers in the given order.
    fn layer_set<'l>(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        layers: impl Iterator<Item = &'l Layer>,
        name: &str,
    ) -> Ref {
        let model = self.model;
        let refs: Vec<Ref> = layers
            .map(|l| {
                let mat_name = model
                    .material(l.material)
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                self.w.add(format!(
                    "IFCMATERIALLAYER({},{},{},{},$,{},$)",
                    materials[&l.material],
                    real(l.thickness),
                    boolean(l.function == LayerFunction::AirGap),
                    string(&mat_name),
                    string(l.function.ifc_category())
                ))
            })
            .collect();
        self.w.add(format!(
            "IFCMATERIALLAYERSET({},{},$)",
            list(&refs),
            string(name)
        ))
    }

    /// A property set or quantity set attached to one element.
    fn attach_set(&mut self, element: Ref, element_guid: &GlobalId, set: Ref, name: &str) {
        self.w.add(format!(
            "IFCRELDEFINESBYPROPERTIES({},$,$,$,({element}),{set})",
            string(derived_guid(element_guid, &format!("rel {name}")).as_str())
        ));
    }

    /// The common properties shared by walls and slabs.
    /// IFC's `Status` property for an element that is not new (phases of
    /// a refurbishment).
    fn status_prop(&mut self, id: u64) -> Option<Ref> {
        let phase = self.model.phase(id);
        (phase != archisim_core::Phase::New).then(|| {
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Status',$,IFCLABEL({}),$)",
                string(phase.ifc_status())
            ))
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn common_pset(
        &mut self,
        element: Ref,
        id: u64,
        guid: &GlobalId,
        pset_name: &str,
        reference: &str,
        is_external: bool,
        load_bearing: bool,
    ) {
        let status = self.status_prop(id);
        let mut props = vec![
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER({}),$)",
                string(reference)
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN({}),$)",
                boolean(is_external)
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('LoadBearing',$,IFCBOOLEAN({}),$)",
                boolean(load_bearing)
            )),
        ];
        props.extend(status);
        let pset = self.w.add(format!(
            "IFCPROPERTYSET({},$,{},$,{})",
            string(derived_guid(guid, pset_name).as_str()),
            string(pset_name),
            list(&props)
        ));
        self.attach_set(element, guid, pset, pset_name);
    }

    /// A quantity set from `(name, entity, value)` triples.
    fn quantities(
        &mut self,
        element: Ref,
        guid: &GlobalId,
        qto_name: &str,
        values: &[(&str, &str, f64)],
    ) {
        let qs: Vec<Ref> = values
            .iter()
            .map(|(name, entity, value)| {
                self.w
                    .add(format!("{entity}('{name}',$,$,{},$)", real(*value)))
            })
            .collect();
        let qto = self.w.add(format!(
            "IFCELEMENTQUANTITY({},$,{},$,$,{})",
            string(derived_guid(guid, qto_name).as_str()),
            string(qto_name),
            list(&qs)
        ));
        self.attach_set(element, guid, qto, qto_name);
    }

    /// A property set from already written `IfcPropertySingleValue`s.
    fn pset(&mut self, element: Ref, guid: &GlobalId, name: &str, props: &[Ref]) {
        let pset = self.w.add(format!(
            "IFCPROPERTYSET({},$,{},$,{})",
            string(derived_guid(guid, name).as_str()),
            string(name),
            list(props)
        ));
        self.attach_set(element, guid, pset, name);
    }

    fn rect_profile(&mut self, x0: f64, x1: f64, y0: f64, y1: f64) -> Ref {
        let pts = [
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ];
        let curve = self.polyline(&pts, true);
        self.w
            .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{curve})"))
    }

    /// A box between two corners in the current placement (x along the
    /// wall, y across it, z up), as a profile in the XZ plane extruded
    /// across. `None` for a box with no volume.
    fn box_solid(&mut self, min: [f64; 3], max: [f64; 3], material: MaterialId) -> Option<Ref> {
        if (0..3).any(|k| max[k] - min[k] <= EPSILON) {
            return None;
        }
        let profile = self.rect_profile(min[0], max[0], min[2], max[2]);
        let at = self.point3(0.0, max[1], 0.0);
        let axis = self.w.add("IFCDIRECTION((0.,-1.,0.))");
        let xdir = self.w.add("IFCDIRECTION((1.,0.,0.))");
        let pl = self
            .w
            .add(format!("IFCAXIS2PLACEMENT3D({at},{axis},{xdir})"));
        let solid = self.w.add(format!(
            "IFCEXTRUDEDAREASOLID({profile},{pl},{},{})",
            self.z,
            real(max[1] - min[1])
        ));
        if let Some(&style) = self.styles.get(&material) {
            self.w.add(format!("IFCSTYLEDITEM({solid},({style}),$)"));
        }
        Some(solid)
    }

    /// `ArchiSim_Family` (family name, version and recipe) and
    /// `ArchiSim_FamilyValues` (one property per parameter, if any) for a
    /// type, as property set definitions of the type.
    fn family_psets(
        &mut self,
        type_guid: &GlobalId,
        family: &archisim_core::Family,
        values: &archisim_core::Values,
    ) -> (Ref, Option<Ref>) {
        let props = [
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Family',$,IFCLABEL({}),$)",
                string(family.name())
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Version',$,IFCINTEGER({}),$)",
                family.version()
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Source',$,IFCTEXT({}),$)",
                string(&family.source)
            )),
        ];
        let set = self.w.add(format!(
            "IFCPROPERTYSET({},$,'ArchiSim_Family',$,{})",
            string(derived_guid(type_guid, "ArchiSim_Family").as_str()),
            list(&props)
        ));
        let value_props = self.value_properties(family, values);
        let values_set = (!value_props.is_empty()).then(|| {
            self.w.add(format!(
                "IFCPROPERTYSET({},$,'ArchiSim_FamilyValues',$,{})",
                string(derived_guid(type_guid, "ArchiSim_FamilyValues").as_str()),
                list(&value_props)
            ))
        });
        (set, values_set)
    }

    /// One `IfcPropertySingleValue` per parameter value, typed by the
    /// parameter's kind: lengths in metres, materials by name.
    fn value_properties(
        &mut self,
        family: &archisim_core::Family,
        values: &archisim_core::Values,
    ) -> Vec<Ref> {
        use archisim_core::{ParamKind, Value};
        let mut out = Vec::new();
        for (name, value) in values {
            let kind = family.def().param(name).map(|p| p.kind.clone());
            let typed = match (kind, value) {
                (Some(ParamKind::Length), Value::Number(x)) => {
                    format!("IFCLENGTHMEASURE({})", real(*x))
                }
                (Some(ParamKind::Integer), Value::Number(x)) => {
                    format!("IFCINTEGER({})", x.round() as i64)
                }
                (_, Value::Number(x)) => format!("IFCREAL({})", real(*x)),
                (_, Value::Bool(b)) => format!("IFCBOOLEAN({})", boolean(*b)),
                (_, Value::Text(t)) => format!("IFCLABEL({})", string(t)),
                (_, Value::Material(m)) => {
                    let name = self
                        .model
                        .material(MaterialId(*m))
                        .map_or(String::new(), |m| m.name.clone());
                    format!("IFCLABEL({})", string(&name))
                }
            };
            out.push(self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE({},$,{typed},$)",
                string(name)
            )));
        }
        out
    }

    /// `IfcDoorType` or `IfcWindowType` with its material constituents.
    fn opening_type(&mut self, materials: &BTreeMap<MaterialId, Ref>, t: &OpeningType) -> Ref {
        // Frame sizes and, for windows, how each leaf opens, so they survive
        // a round trip and other programs can redraw them.
        let lining_guid = derived_guid(&t.guid, "lining");
        let (depth, width) = (real(t.frame_depth), real(t.frame_width));
        let mut props = vec![match t.kind {
            OpeningKind::Door(_) | OpeningKind::Void => self.w.add(format!(
                "IFCDOORLININGPROPERTIES({},$,'Marco',$,{depth},{width},$,$,$,$,$,$,$,$,$,$,$)",
                string(lining_guid.as_str())
            )),
            OpeningKind::Window(_) => self.w.add(format!(
                "IFCWINDOWLININGPROPERTIES({},$,'Marco',$,{depth},{width},$,$,$,$,$,$,$,$,$,$)",
                string(lining_guid.as_str())
            )),
        }];
        if let OpeningKind::Window(_) = t.kind {
            // How each leaf opens, from the family, left to right.
            let n = t.panels.len();
            for (i, operation) in t.panels.iter().enumerate() {
                let position = match (i, n) {
                    (_, 1) => "MIDDLE",
                    (0, _) => "LEFT",
                    (i, n) if i + 1 == n => "RIGHT",
                    _ => "MIDDLE",
                };
                let guid = derived_guid(&t.guid, &format!("panel {i}"));
                props.push(self.w.add(format!(
                    "IFCWINDOWPANELPROPERTIES({},$,'Hoja {}',$,.{operation}.,.{position}.,{depth},{width},$)",
                    string(guid.as_str()),
                    i + 1
                )));
            }
        }
        // ArchiSim's own data: the family (name, version and recipe) and the
        // value of each type parameter, so that any family comes back the
        // same, and others can read the parameters.
        let model = self.model;
        if let Some(family) = model.family(t.family) {
            let (a, b) = self.family_psets(&t.guid, family, &t.values);
            props.push(a);
            if let Some(b) = b {
                props.push(b);
            }
        }
        let props = list(&props);
        let description = if t.description.is_empty() {
            "$".to_string()
        } else {
            string(&t.description)
        };
        let tag = if t.mark.is_empty() {
            "$".to_string()
        } else {
            string(&t.mark)
        };
        let element_type = model
            .family(t.family)
            .map_or("$".to_string(), |f| string(f.name()));
        let (ty, constituents) = match t.kind {
            OpeningKind::Void => unreachable!("voids have no IFC type"),
            OpeningKind::Door(op) => (
                self.w.add(format!(
                    "IFCDOORTYPE({},$,{},{description},$,{props},$,{tag},{element_type},.DOOR.,.{}.,.F.,$)",
                    string(t.guid.as_str()),
                    string(&t.name),
                    door_operation(op, Hand::Right)
                )),
                [("Lining", t.frame_material), ("Panel", t.panel_material)],
            ),
            OpeningKind::Window(_) => (
                self.w.add(format!(
                    "IFCWINDOWTYPE({},$,{},{description},$,{props},$,{tag},{element_type},.WINDOW.,.{}.,.F.,$)",
                    string(t.guid.as_str()),
                    string(&t.name),
                    window_partitioning(t.panels.len())
                )),
                [("Framing", t.frame_material), ("Glazing", t.panel_material)],
            ),
        };
        let refs: Vec<Ref> = constituents
            .iter()
            .map(|(name, m)| {
                self.w.add(format!(
                    "IFCMATERIALCONSTITUENT({},$,{},$,$)",
                    string(name),
                    materials[m]
                ))
            })
            .collect();
        let set = self.w.add(format!(
            "IFCMATERIALCONSTITUENTSET({},$,{})",
            string(&t.name),
            list(&refs)
        ));
        self.w.add(format!(
            "IFCRELASSOCIATESMATERIAL({},$,$,$,({ty}),{set})",
            string(derived_guid(&t.guid, "material").as_str())
        ));
        ty
    }

    /// Writes the opening that voids `wall` and the door or window filling
    /// it. Returns the door or window; a void ("hueco", no door or window)
    /// is the `IfcOpeningElement` alone, with the element's own GUID and
    /// its type in `ArchiSim_Void`.
    #[allow(clippy::too_many_arguments)]
    fn opening(
        &mut self,
        wall: Ref,
        wall_pl: Ref,
        og: &OpeningGeometry,
        guid: &GlobalId,
        t: &OpeningType,
        wall_base: f64,
        wall_top: f64,
        is_external: bool,
        tag: u64,
    ) -> Option<Ref> {
        let (w, h) = (og.width(), og.height());
        let is_void = t.is_void();
        let mid = (og.t0 + og.t1) / 2.0;

        // The void: the rough opening through the whole wall thickness. It
        // overshoots the faces, and the base or top when it reaches them, so
        // the boolean cut is clean.
        let m = 0.05;
        let below = if (og.sill - wall_base).abs() < EPSILON {
            m
        } else {
            0.0
        };
        let above = if (og.head - wall_top).abs() < EPSILON {
            m
        } else {
            0.0
        };
        let void_guid = if is_void {
            guid.clone()
        } else {
            derived_guid(guid, "opening")
        };
        let loc = self.point3(mid, 0.0, og.sill - wall_base);
        let void_pl = self.placement(Some(wall_pl), loc, None);
        let outline = self.polyline(
            &[
                Point2::new(-w / 2.0, og.interior_offset - m),
                Point2::new(w / 2.0, og.interior_offset - m),
                Point2::new(w / 2.0, og.exterior_offset + m),
                Point2::new(-w / 2.0, og.exterior_offset + m),
            ],
            true,
        );
        let profile = self
            .w
            .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{outline})"));
        let at = self.point3(0.0, 0.0, -below);
        let solid_pl = self.w.add(format!("IFCAXIS2PLACEMENT3D({at},$,$)"));
        let solid = self.w.add(format!(
            "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
            self.z,
            real(h + below + above)
        ));
        let rep = self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
            self.body_ctx
        ));
        let shape = self
            .w
            .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
        let void_name = if is_void {
            format!("{} {tag}", t.name)
        } else {
            format!("Hueco {}", t.name)
        };
        let void = self.w.add(format!(
            "IFCOPENINGELEMENT({},$,{},$,{},{void_pl},{shape},{},.OPENING.)",
            string(void_guid.as_str()),
            string(&void_name),
            if is_void {
                string(&t.name)
            } else {
                "$".to_string()
            },
            if is_void {
                string(&tag.to_string())
            } else {
                "$".to_string()
            }
        ));
        self.w.add(format!(
            "IFCRELVOIDSELEMENT({},$,$,$,{wall},{void})",
            string(derived_guid(guid, "voids").as_str())
        ));
        self.quantities(
            void,
            &void_guid,
            "Qto_OpeningElementBaseQuantities",
            &[
                ("Width", "IFCQUANTITYLENGTH", w),
                ("Height", "IFCQUANTITYLENGTH", h),
                (
                    "Depth",
                    "IFCQUANTITYLENGTH",
                    og.exterior_offset - og.interior_offset,
                ),
            ],
        );
        if is_void {
            // Nothing fills it. ArchiSim's own data: its type, so it comes
            // back as the same type.
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('TypeName',$,IFCLABEL({}),$)",
                    string(&t.name)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('TypeGuid',$,IFCIDENTIFIER({}),$)",
                    string(t.guid.as_str())
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Mark',$,IFCIDENTIFIER({}),$)",
                    string(&t.mark)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN({}),$)",
                    boolean(is_external)
                )),
            ];
            self.pset(void, &void_guid, "ArchiSim_Void", &props);
            return None;
        }

        // The door or window, in the frame plane, its Y axis towards the
        // side it opens to.
        let y_mid = (og.frame_exterior + og.frame_interior) / 2.0;
        let facing = og.swing_dir().dot(og.n);
        let loc = self.point3(0.0, y_mid, 0.0);
        let xdir = self
            .w
            .add(format!("IFCDIRECTION(({},0.,0.))", real(facing.signum())));
        let pl = self.placement(Some(void_pl), loc, Some(xdir));

        // The body: the 3D pieces of the family, one box each. The family
        // draws with x to the right seen from the side the leaves open to,
        // which is the opposite of this placement's x.
        let is_door = matches!(t.kind, OpeningKind::Door(_));
        let parts = self
            .model
            .evaluate_opening(og.opening)
            .map(|e| e.parts)
            .unwrap_or_default();
        let mut solids = Vec::new();
        for part in &parts {
            let solid = self.box_solid(
                [-part.max[0], part.min[1], part.min[2]],
                [-part.min[0], part.max[1], part.max[2]],
                MaterialId(part.material),
            );
            if let Some(solid) = solid {
                solids.push(solid);
            }
        }
        let rep = self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',{})",
            self.body_ctx,
            list(&solids)
        ));
        let shape = self
            .w
            .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
        let name = format!("{} {tag}", t.name);
        let filling = if is_door {
            let OpeningKind::Door(op) = t.kind else {
                unreachable!("checked above")
            };
            self.w.add(format!(
                "IFCDOOR({},$,{},$,{},{pl},{shape},{},{},{},.DOOR.,.{}.,$)",
                string(guid.as_str()),
                string(&name),
                string(&t.name),
                string(&tag.to_string()),
                real(h),
                real(w),
                door_operation(op, og.hand)
            ))
        } else {
            self.w.add(format!(
                "IFCWINDOW({},$,{},$,{},{pl},{shape},{},{},{},.WINDOW.,.{}.,$)",
                string(guid.as_str()),
                string(&name),
                string(&t.name),
                string(&tag.to_string()),
                real(h),
                real(w),
                window_partitioning(t.panels.len())
            ))
        };
        self.w.add(format!(
            "IFCRELFILLSELEMENT({},$,$,$,{void},{filling})",
            string(derived_guid(guid, "fills").as_str())
        ));

        let (pset_name, qto_name) = if is_door {
            ("Pset_DoorCommon", "Qto_DoorBaseQuantities")
        } else {
            ("Pset_WindowCommon", "Qto_WindowBaseQuantities")
        };
        // `Reference` is the type mark ("P-01"), as IFC defines it; the name
        // when the type has none.
        let reference = if t.mark.is_empty() { &t.name } else { &t.mark };
        let status = self.status_prop(og.opening.0);
        let mut props = vec![
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER({}),$)",
                string(reference)
            )),
            self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN({}),$)",
                boolean(is_external)
            )),
        ];
        props.extend(status);
        self.pset(filling, guid, pset_name, &props);
        // Instance parameters of the family, if it has any.
        if let Some(o) = self.model.opening(og.opening) {
            if !o.values.is_empty() {
                if let Some(family) = self.model.family(t.family) {
                    let values = self.value_properties(family, &o.values);
                    self.pset(filling, guid, "ArchiSim_FamilyValues", &values);
                }
            }
        }
        // ArchiSim's own data: the hand (UNE-EN 12519), which IFC only
        // records for doors.
        let hand = self.w.add(format!(
            "IFCPROPERTYSINGLEVALUE('Hand',$,IFCLABEL({}),$)",
            string(match og.hand {
                Hand::Left => "Left",
                Hand::Right => "Right",
            })
        ));
        self.pset(filling, guid, "ArchiSim_Opening", &[hand]);
        self.quantities(
            filling,
            guid,
            qto_name,
            &[
                ("Width", "IFCQUANTITYLENGTH", w),
                ("Height", "IFCQUANTITYLENGTH", h),
                ("Area", "IFCQUANTITYAREA", w * h),
            ],
        );
        Some(filling)
    }

    fn rel_aggregates(&mut self, guid: GlobalId, parent: Ref, children: &[Ref]) {
        self.w.add(format!(
            "IFCRELAGGREGATES({},$,$,$,{parent},{})",
            string(guid.as_str()),
            list(children)
        ));
    }

    fn write(&mut self) {
        let model = self.model;

        // Contexts and units.
        let wcs = self
            .w
            .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
        let ctx = self.w.add(format!(
            "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,{wcs},$)"
        ));
        self.body_ctx = self.w.add(format!(
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,{ctx},$,.MODEL_VIEW.,$)"
        ));
        self.axis_ctx = self.w.add(format!(
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Axis','Model',*,*,*,*,{ctx},$,.GRAPH_VIEW.,$)"
        ));
        let units = [
            self.w.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)"),
            self.w.add("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)"),
            self.w.add("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)"),
            self.w.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)"),
        ];
        let units = self.w.add(format!("IFCUNITASSIGNMENT({})", list(&units)));

        // Spatial structure.
        let project = self.w.add(format!(
            "IFCPROJECT({},$,{},$,$,$,$,({ctx}),{units})",
            string(model.project_guid.as_str()),
            string(&model.project_name)
        ));
        let site_pl = self.placement(None, self.origin, None);
        let site = self.w.add(format!(
            "IFCSITE({},$,'Emplazamiento',$,$,{site_pl},$,$,.ELEMENT.,$,$,$,$,$)",
            string(model.site_guid.as_str())
        ));
        let bldg_pl = self.placement(Some(site_pl), self.origin, None);
        let building = self.w.add(format!(
            "IFCBUILDING({},$,{},$,$,{bldg_pl},$,$,.ELEMENT.,$,$,$)",
            string(model.building_guid.as_str()),
            string(&model.project_name)
        ));
        self.rel_aggregates(
            derived_guid(&model.project_guid, "aggregates"),
            project,
            &[site],
        );
        self.rel_aggregates(
            derived_guid(&model.site_guid, "aggregates"),
            site,
            &[building],
        );

        let mut storeys: BTreeMap<LevelId, (Ref, Ref)> = BTreeMap::new();
        for level in model.levels() {
            let loc = self.point3(0.0, 0.0, level.elevation);
            let pl = self.placement(Some(bldg_pl), loc, None);
            let s = self.w.add(format!(
                "IFCBUILDINGSTOREY({},$,{},$,$,{pl},$,$,.ELEMENT.,{})",
                string(level.guid.as_str()),
                string(&level.name),
                real(level.elevation)
            ));
            storeys.insert(level.id, (s, pl));
        }
        if !storeys.is_empty() {
            let refs: Vec<Ref> = storeys.values().map(|(s, _)| *s).collect();
            self.rel_aggregates(
                derived_guid(&model.building_guid, "aggregates"),
                building,
                &refs,
            );
        }

        // Materials.
        let mut materials: BTreeMap<MaterialId, Ref> = BTreeMap::new();
        for m in model.materials() {
            let r = self.w.add(format!(
                "IFCMATERIAL({},$,{})",
                string(&m.name),
                string(&m.category)
            ));
            let [cr, cg, cb] = m.cut_color.map(|c| c as f64 / 255.0);
            let colour = self.w.add(format!(
                "IFCCOLOURRGB($,{},{},{})",
                real(cr),
                real(cg),
                real(cb)
            ));
            // Glass is see-through in 3D viewers.
            let transparency = if m.category == "glass" { 0.6 } else { 0.0 };
            let shading = self.w.add(format!(
                "IFCSURFACESTYLESHADING({colour},{})",
                real(transparency)
            ));
            let style = self.w.add(format!(
                "IFCSURFACESTYLE({},.BOTH.,({shading}))",
                string(&m.name)
            ));
            self.styles.insert(m.id, style);
            let item = self.w.add(format!("IFCSTYLEDITEM($,({style}),$)"));
            let rep = self.w.add(format!(
                "IFCSTYLEDREPRESENTATION({},'Style','Material',({item}))",
                self.body_ctx
            ));
            self.w.add(format!(
                "IFCMATERIALDEFINITIONREPRESENTATION($,$,({rep}),{r})"
            ));
            // ArchiSim's own data: how the material is hatched when cut.
            let pattern = self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('CutPattern',$,IFCLABEL({}),$)",
                string(import::pattern_name(m.cut_pattern))
            ));
            self.w.add(format!(
                "IFCMATERIALPROPERTIES('ArchiSim_Material',$,({pattern}),{r})"
            ));
            materials.insert(m.id, r);
        }

        // Wall types and their layer sets (interior → exterior, see module docs).
        let mut layer_sets: BTreeMap<WallTypeId, Ref> = BTreeMap::new();
        let mut type_refs: BTreeMap<WallTypeId, Ref> = BTreeMap::new();
        for t in model.wall_types() {
            let set = self.layer_set(&materials, t.layers.iter().rev(), &t.name);
            let ty = self.w.add(format!(
                "IFCWALLTYPE({},$,{},$,$,$,$,$,$,.STANDARD.)",
                string(t.guid.as_str()),
                string(&t.name)
            ));
            self.w.add(format!(
                "IFCRELASSOCIATESMATERIAL({},$,$,$,({ty}),{set})",
                string(derived_guid(&t.guid, "material").as_str())
            ));
            layer_sets.insert(t.id, set);
            type_refs.insert(t.id, ty);
        }

        // Slab types and their layer sets (bottom → top, see module docs).
        let mut slab_sets: BTreeMap<SlabTypeId, Ref> = BTreeMap::new();
        let mut slab_type_refs: BTreeMap<SlabTypeId, Ref> = BTreeMap::new();
        for t in model.slab_types() {
            let set = self.layer_set(&materials, t.layers.iter().rev(), &t.name);
            let ty = self.w.add(format!(
                "IFCSLABTYPE({},$,{},$,$,$,$,$,$,.{}.)",
                string(t.guid.as_str()),
                string(&t.name),
                t.function.ifc_predefined_type()
            ));
            self.w.add(format!(
                "IFCRELASSOCIATESMATERIAL({},$,$,$,({ty}),{set})",
                string(derived_guid(&t.guid, "material").as_str())
            ));
            slab_sets.insert(t.id, set);
            slab_type_refs.insert(t.id, ty);
        }

        // Door and window types.
        let mut opening_type_refs: BTreeMap<OpeningTypeId, Ref> = BTreeMap::new();
        // Voids have no IFC type: the opening element is all there is.
        for t in model.opening_types().filter(|t| !t.is_void()) {
            let ty = self.opening_type(&materials, t);
            opening_type_refs.insert(t.id, ty);
        }
        let mut openings_by_type: BTreeMap<OpeningTypeId, Vec<Ref>> = BTreeMap::new();

        // Walls.
        let mut contained: BTreeMap<LevelId, Vec<Ref>> = BTreeMap::new();
        let mut by_type: BTreeMap<WallTypeId, Vec<Ref>> = BTreeMap::new();
        let idx = archisim_geom::WallIndex::new(model);
        for wall in model.walls() {
            let (Some(wtype), Some(geo), Some(&(_, storey_pl))) = (
                model.wall_type(wall.wall_type),
                archisim_geom::wall_geometry_in(&idx, wall.id),
                storeys.get(&wall.level),
            ) else {
                continue;
            };
            let length = wall.length();
            let chord = wall.start.distance(wall.end);
            let u = (wall.end - wall.start) * (1.0 / chord);
            let n = u.perp_left();
            let to_local = |p: Point2| {
                let d = p - wall.start;
                Point2::new(d.dot(u), d.dot(n))
            };

            let loc = self.point3(wall.start.x, wall.start.y, wall.base_offset);
            let xdir = self
                .w
                .add(format!("IFCDIRECTION(({},{},0.))", real(u.x), real(u.y)));
            let pl = self.placement(Some(storey_pl), loc, Some(xdir));

            let axis_line = match wall.arc() {
                Some(arc) => {
                    // A circle around the centre, trimmed at both ends,
                    // counter-clockwise or not as the wall turns.
                    let c = self.point2(to_local(arc.centre));
                    let ap = self.w.add(format!("IFCAXIS2PLACEMENT2D({c},$)"));
                    let circle = self.w.add(format!("IFCCIRCLE({ap},{})", real(arc.radius)));
                    let p0 = self.point2(Point2::new(0.0, 0.0));
                    let p1 = self.point2(Point2::new(chord, 0.0));
                    self.w.add(format!(
                        "IFCTRIMMEDCURVE({circle},({p0}),({p1}),{},.CARTESIAN.)",
                        boolean(arc.sweep > 0.0)
                    ))
                }
                None => self.polyline(&[Point2::new(0.0, 0.0), Point2::new(length, 0.0)], false),
            };
            let axis_rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Axis','Curve2D',({axis_line}))",
                self.axis_ctx
            ));
            // Walls butting into this one take part of its layers over
            // their own height only: one extrusion per height band.
            let base = wall.base_offset;
            let top = base + wall.height;
            let mut zs = vec![base, top];
            zs.extend(
                geo.notch_heights()
                    .into_iter()
                    .filter(|z| *z > base + 1e-6 && *z < top - 1e-6),
            );
            zs.sort_by(f64::total_cmp);
            let mut solids = Vec::new();
            let mut volume = 0.0;
            let path = geo.outline_path();
            for band in zs.windows(2) {
                let (z0, z1) = (band[0], band[1]);
                let outline = match &path {
                    Some(path) => {
                        volume += path_area(path).abs() * (z1 - z0);
                        self.indexed_curve(path, &to_local)
                    }
                    None => {
                        let poly = geo.outline_at(Some((z0 + z1) / 2.0));
                        volume += signed_area(&poly) * (z1 - z0);
                        let local: Vec<Point2> = poly.iter().map(|&p| to_local(p)).collect();
                        self.polyline(&local, true)
                    }
                };
                let profile = self
                    .w
                    .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{outline})"));
                let solid_pl = if (z0 - base).abs() < 1e-9 {
                    self.w
                        .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin))
                } else {
                    let at = self.point3(0.0, 0.0, z0 - base);
                    self.w.add(format!("IFCAXIS2PLACEMENT3D({at},$,$)"))
                };
                solids.push(self.w.add(format!(
                    "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                    self.z,
                    real(z1 - z0)
                )));
            }
            let items = solids
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let body_rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({items}))",
                self.body_ctx
            ));
            let shape = self.w.add(format!(
                "IFCPRODUCTDEFINITIONSHAPE($,$,({axis_rep},{body_rep}))"
            ));
            let name = format!("{} {}", wtype.name, wall.id.0);
            let w = self.w.add(format!(
                "IFCWALL({},$,{},$,{},{pl},{shape},{},.STANDARD.)",
                string(wall.guid.as_str()),
                string(&name),
                string(&wtype.name),
                string(&wall.id.0.to_string())
            ));

            let usage = self.w.add(format!(
                "IFCMATERIALLAYERSETUSAGE({},.AXIS2.,.POSITIVE.,{},$)",
                layer_sets[&wall.wall_type],
                real(geo.interior_offset)
            ));
            self.w.add(format!(
                "IFCRELASSOCIATESMATERIAL({},$,$,$,({w}),{usage})",
                string(derived_guid(&wall.guid, "material").as_str())
            ));

            // Properties and quantities.
            self.common_pset(
                w,
                wall.id.0,
                &wall.guid,
                "Pset_WallCommon",
                &wtype.name,
                wtype.function == WallFunction::Exterior,
                wtype.is_load_bearing(),
            );
            // Doors and windows: void the wall, then fill the void.
            let mut cut_side = 0.0;
            let mut cut_volume = 0.0;
            for og in idx.openings_in_wall(wall.id) {
                let (Some(o), Some(otype)) = (
                    model.opening(og.opening),
                    model
                        .opening(og.opening)
                        .and_then(|o| model.opening_type(o.opening_type)),
                ) else {
                    continue;
                };
                cut_side += og.width() * og.height();
                cut_volume += removed_area(&geo, &og) * og.height();
                let filling = self.opening(
                    w,
                    pl,
                    &og,
                    &o.guid,
                    otype,
                    wall.base_offset,
                    wall.base_offset + wall.height,
                    wtype.function == WallFunction::Exterior,
                    o.id.0,
                );
                if let Some(filling) = filling {
                    contained.entry(wall.level).or_default().push(filling);
                    openings_by_type.entry(otype.id).or_default().push(filling);
                }
            }

            let footprint = match &path {
                Some(path) => path_area(path).abs(),
                None => signed_area(&geo.outline_at(Some(base + 1e-6))),
            };
            let side = length * wall.height;
            self.quantities(
                w,
                &wall.guid,
                "Qto_WallBaseQuantities",
                &[
                    ("Length", "IFCQUANTITYLENGTH", length),
                    ("Width", "IFCQUANTITYLENGTH", wtype.thickness()),
                    ("Height", "IFCQUANTITYLENGTH", wall.height),
                    ("GrossFootprintArea", "IFCQUANTITYAREA", footprint),
                    ("GrossSideArea", "IFCQUANTITYAREA", side),
                    ("NetSideArea", "IFCQUANTITYAREA", side - cut_side),
                    ("GrossVolume", "IFCQUANTITYVOLUME", volume),
                    ("NetVolume", "IFCQUANTITYVOLUME", volume - cut_volume),
                ],
            );

            contained.entry(wall.level).or_default().push(w);
            by_type.entry(wall.wall_type).or_default().push(w);
        }

        // Slabs: placed at the soffit, extruded upwards by the type thickness.
        let mut slabs_by_type: BTreeMap<SlabTypeId, Vec<Ref>> = BTreeMap::new();
        for slab in model.slabs() {
            let (Some(stype), Some(&(_, storey_pl))) =
                (model.slab_type(slab.slab_type), storeys.get(&slab.level))
            else {
                continue;
            };
            let thickness = stype.thickness();
            let (pl, shape) = self.slab_shape(slab, thickness, storey_pl);
            let name = format!("{} {}", stype.name, slab.id.0);
            let sl = self.w.add(format!(
                "IFCSLAB({},$,{},$,{},{pl},{shape},{},.{}.)",
                string(slab.guid.as_str()),
                string(&name),
                string(&stype.name),
                string(&slab.id.0.to_string()),
                stype.function.ifc_predefined_type()
            ));

            let usage = self.w.add(format!(
                "IFCMATERIALLAYERSETUSAGE({},.AXIS3.,.POSITIVE.,0.,$)",
                slab_sets[&slab.slab_type]
            ));
            self.w.add(format!(
                "IFCRELASSOCIATESMATERIAL({},$,$,$,({sl}),{usage})",
                string(derived_guid(&slab.guid, "material").as_str())
            ));

            self.common_pset(
                sl,
                slab.id.0,
                &slab.guid,
                "Pset_SlabCommon",
                &stype.name,
                stype.function.is_external(),
                stype.is_load_bearing(),
            );
            // Areas on the face (larger than in plan if it slopes);
            // volumes are plan area by vertical height.
            let stretch = slab.vertical(1.0);
            let area = slab.area() * stretch;
            let gross = signed_area(&slab.outline).abs() * stretch;
            let perim = perimeter(&slab.outline);
            self.quantities(
                sl,
                &slab.guid,
                "Qto_SlabBaseQuantities",
                &[
                    ("Width", "IFCQUANTITYLENGTH", thickness),
                    ("Perimeter", "IFCQUANTITYLENGTH", perim),
                    ("GrossArea", "IFCQUANTITYAREA", gross),
                    ("NetArea", "IFCQUANTITYAREA", area),
                    ("GrossVolume", "IFCQUANTITYVOLUME", gross * thickness),
                    ("NetVolume", "IFCQUANTITYVOLUME", area * thickness),
                ],
            );

            contained.entry(slab.level).or_default().push(sl);
            slabs_by_type.entry(slab.slab_type).or_default().push(sl);
        }

        self.write_members(&materials, &storeys, &mut contained);
        self.write_roofs(&storeys, &slab_sets, &mut contained, &mut slabs_by_type);
        self.write_spaces(&storeys);

        for (level, elements) in &contained {
            let (storey, _) = storeys[level];
            let guid = model
                .level(*level)
                .map(|l| l.guid.clone())
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,{},{storey})",
                string(derived_guid(&guid, "contains").as_str()),
                list(elements)
            ));
        }
        for (t, walls) in &by_type {
            let guid = model
                .wall_type(*t)
                .map(|t| t.guid.clone())
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELDEFINESBYTYPE({},$,$,$,{},{})",
                string(derived_guid(&guid, "type").as_str()),
                list(walls),
                type_refs[t]
            ));
        }
        for (t, elements) in &openings_by_type {
            let guid = model
                .opening_type(*t)
                .map(|t| t.guid.clone())
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELDEFINESBYTYPE({},$,$,$,{},{})",
                string(derived_guid(&guid, "type").as_str()),
                list(elements),
                opening_type_refs[t]
            ));
        }
        for (t, slabs) in &slabs_by_type {
            let guid = model
                .slab_type(*t)
                .map(|t| t.guid.clone())
                .unwrap_or_default();
            self.w.add(format!(
                "IFCRELDEFINESBYTYPE({},$,$,$,{},{})",
                string(derived_guid(&guid, "type").as_str()),
                list(slabs),
                slab_type_refs[t]
            ));
        }
    }
}

/// `IfcDoorTypeOperationEnum` value. IFC reads left and right looking along
/// the door's +Y axis (towards the side it opens to), so the UNE-EN 12519
/// hand, seen from that side, swaps.
fn door_operation(op: DoorOperation, hand: Hand) -> &'static str {
    match (op, hand) {
        (DoorOperation::SingleSwing, Hand::Left) => "SINGLE_SWING_RIGHT",
        (DoorOperation::SingleSwing, Hand::Right) => "SINGLE_SWING_LEFT",
        (DoorOperation::DoubleSwing, _) => "DOUBLE_DOOR_SINGLE_SWING",
        (DoorOperation::Sliding, Hand::Left) => "SLIDING_TO_RIGHT",
        (DoorOperation::Sliding, Hand::Right) => "SLIDING_TO_LEFT",
    }
}

/// `IfcWindowTypePartitioningEnum` value for leaves side by side.
fn window_partitioning(leaves: usize) -> &'static str {
    match leaves {
        0 | 1 => "SINGLE_PANEL",
        2 => "DOUBLE_PANEL_VERTICAL",
        3 => "TRIPLE_PANEL_VERTICAL",
        _ => "USERDEFINED",
    }
}

/// Current UTC time as `YYYY-MM-DDThh:mm:ss`.
fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{
        CutPattern, NewOpening, NewOpeningType, NewSlab, NewWall, ReferenceLine, SlabFunction,
        WallSide,
    };

    fn model() -> Model {
        let mut m = Model::new("Casa de prueba");
        let lvl = m.add_level("Planta baja", 0.0);
        let mat = m.add_material(
            "Ladrillo cerámico",
            "brick",
            [180, 80, 60],
            CutPattern::Diagonal,
        );
        let t = m
            .add_wall_type(
                "Fábrica 1/2 pie",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.115,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let wall = m
            .add_wall(NewWall {
                wall_type: t,
                level: lvl,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                bulge: 0.0,
                top: archisim_core::WallTop::Unconnected,
            })
            .unwrap();
        let door = m
            .add_opening_type(NewOpeningType {
                name: "Puerta 90".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.9,
                height: 2.1,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: mat,
                panel_material: mat,
            })
            .unwrap();
        m.add_opening(NewOpening {
            opening_type: door,
            wall,
            along: 2.0,
            sill: 0.0,
            opens_to: WallSide::Interior,
            hand: Hand::Left,
            setback: None,
        })
        .unwrap();
        let st = m
            .add_slab_type(
                "Solera 15",
                SlabFunction::BaseSlab,
                vec![Layer {
                    material: mat,
                    thickness: 0.15,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.add_slab(NewSlab {
            slab_type: st,
            level: lvl,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(5.0, 0.0),
                Point2::new(5.0, 4.0),
                Point2::new(0.0, 4.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        m
    }

    #[test]
    fn exports_expected_entities() {
        let opts = ExportOptions {
            timestamp: Some("2026-10-17T00:00:00".into()),
            ..Default::default()
        };
        let text = export_ifc(&model(), &opts);
        assert!(text.starts_with("ISO-10303-21;"));
        assert!(text.trim_end().ends_with("END-ISO-10303-21;"));
        for needle in [
            "FILE_SCHEMA(('IFC4'))",
            "IFCPROJECT(",
            "IFCBUILDINGSTOREY(",
            "IFCWALLTYPE(",
            "IFCWALL(",
            "IFCMATERIALLAYERSETUSAGE(",
            "'Pset_WallCommon'",
            "'Qto_WallBaseQuantities'",
            "IFCRELDEFINESBYTYPE(",
            "IFCRELCONTAINEDINSPATIALSTRUCTURE(",
            "cer\\X2\\00E1\\X0\\mico",
            "IFCSLABTYPE(",
            ".BASESLAB.)",
            "IFCMATERIALLAYERSETUSAGE(#",
            ".AXIS3.,.POSITIVE.,0.,$)",
            "'Pset_SlabCommon'",
            "'Qto_SlabBaseQuantities'",
            "IFCQUANTITYVOLUME('GrossVolume',$,$,3.,$)",
            "IFCDOORTYPE(",
            "IFCOPENINGELEMENT(",
            "IFCRELVOIDSELEMENT(",
            "IFCRELFILLSELEMENT(",
            ".DOOR.,.SINGLE_SWING_RIGHT.,$)",
            "'Pset_DoorCommon'",
            "'Qto_DoorBaseQuantities'",
            "IFCMATERIALCONSTITUENTSET(",
            // 5 × 0.115 × 3 minus the 0.9 × 2.1 door.
            "IFCQUANTITYVOLUME('NetVolume',$,$,1.50765,$)",
        ] {
            assert!(text.contains(needle), "missing {needle}");
        }
    }

    #[test]
    fn export_is_deterministic() {
        let m = model();
        let opts = ExportOptions {
            timestamp: Some("2026-10-17T00:00:00".into()),
            ..Default::default()
        };
        assert_eq!(export_ifc(&m, &opts), export_ifc(&m, &opts));
    }

    #[test]
    fn timestamp_format() {
        let t = utc_now();
        assert_eq!(t.len(), 19);
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[10..11], "T");
    }
}
