//! IFC4 export.
//!
//! Produces a STEP file with:
//! - the spatial structure Project → Site → Building → Storeys;
//! - materials with surface colours;
//! - wall types (`IfcWallType`) with their material layer sets;
//! - walls (`IfcWall`) with an axis, a swept-solid body following the
//!   resolved corners, a material layer set usage, `Pset_WallCommon` and
//!   `Qto_WallBaseQuantities`.
//!
//! Layer set orientation: walls are placed with local X along the drawn line
//! and local Y to its left (the exterior side). The layer set usage is
//! `AXIS2 / POSITIVE`, starting at the interior face, so layer sets are
//! written from interior to exterior.
//!
//! Relationship GUIDs are derived from element GUIDs, so exporting the same
//! model twice gives the same file apart from the header timestamp.

pub mod step;

use std::collections::BTreeMap;

use archisim_core::{GlobalId, LevelId, MaterialId, Model, Point2, WallFunction, WallTypeId};
use archisim_geom::{signed_area, wall_geometry};
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

    fn polyline(&mut self, pts: &[Point2], closed: bool) -> Ref {
        let mut refs: Vec<Ref> = pts.iter().map(|&p| self.point2(p)).collect();
        if closed {
            refs.push(refs[0]);
        }
        self.w.add(format!("IFCPOLYLINE({})", list(&refs)))
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
            let shading = self.w.add(format!("IFCSURFACESTYLESHADING({colour},0.)"));
            let style = self.w.add(format!(
                "IFCSURFACESTYLE({},.BOTH.,({shading}))",
                string(&m.name)
            ));
            let item = self.w.add(format!("IFCSTYLEDITEM($,({style}),$)"));
            let rep = self.w.add(format!(
                "IFCSTYLEDREPRESENTATION({},'Style','Material',({item}))",
                self.body_ctx
            ));
            self.w.add(format!(
                "IFCMATERIALDEFINITIONREPRESENTATION($,$,({rep}),{r})"
            ));
            materials.insert(m.id, r);
        }

        // Wall types and their layer sets (interior → exterior, see module docs).
        let mut layer_sets: BTreeMap<WallTypeId, Ref> = BTreeMap::new();
        let mut type_refs: BTreeMap<WallTypeId, Ref> = BTreeMap::new();
        for t in model.wall_types() {
            let layers: Vec<Ref> = t
                .layers
                .iter()
                .rev()
                .map(|l| {
                    let name = model
                        .material(l.material)
                        .map(|m| m.name.clone())
                        .unwrap_or_default();
                    self.w.add(format!(
                        "IFCMATERIALLAYER({},{},{},{},$,{},$)",
                        materials[&l.material],
                        real(l.thickness),
                        boolean(l.function == archisim_core::LayerFunction::AirGap),
                        string(&name),
                        string(l.function.ifc_category())
                    ))
                })
                .collect();
            let set = self.w.add(format!(
                "IFCMATERIALLAYERSET({},{},$)",
                list(&layers),
                string(&t.name)
            ));
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

        // Walls.
        let mut contained: BTreeMap<LevelId, Vec<Ref>> = BTreeMap::new();
        let mut by_type: BTreeMap<WallTypeId, Vec<Ref>> = BTreeMap::new();
        for wall in model.walls() {
            let (Some(wtype), Some(geo), Some(&(_, storey_pl))) = (
                model.wall_type(wall.wall_type),
                wall_geometry(model, wall.id),
                storeys.get(&wall.level),
            ) else {
                continue;
            };
            let length = wall.length();
            let u = (wall.end - wall.start) * (1.0 / length);
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

            let axis_line =
                self.polyline(&[Point2::new(0.0, 0.0), Point2::new(length, 0.0)], false);
            let axis_rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Axis','Curve2D',({axis_line}))",
                self.axis_ctx
            ));
            let local: Vec<Point2> = geo.outline.iter().map(|&p| to_local(p)).collect();
            let outline = self.polyline(&local, true);
            let profile = self
                .w
                .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{outline})"));
            let solid_pl = self
                .w
                .add(format!("IFCAXIS2PLACEMENT3D({},$,$)", self.origin));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                self.z,
                real(wall.height)
            ));
            let body_rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
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

            // Properties.
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Reference',$,IFCIDENTIFIER({}),$)",
                    string(&wtype.name)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN({}),$)",
                    boolean(wtype.function == WallFunction::Exterior)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('LoadBearing',$,IFCBOOLEAN({}),$)",
                    boolean(wtype.is_load_bearing())
                )),
            ];
            let pset = self.w.add(format!(
                "IFCPROPERTYSET({},$,'Pset_WallCommon',$,{})",
                string(derived_guid(&wall.guid, "Pset_WallCommon").as_str()),
                list(&props)
            ));
            self.w.add(format!(
                "IFCRELDEFINESBYPROPERTIES({},$,$,$,({w}),{pset})",
                string(derived_guid(&wall.guid, "rel Pset_WallCommon").as_str())
            ));

            // Quantities.
            let footprint = signed_area(&geo.outline);
            let qs = [
                ("Length", "IFCQUANTITYLENGTH", length),
                ("Width", "IFCQUANTITYLENGTH", wtype.thickness()),
                ("Height", "IFCQUANTITYLENGTH", wall.height),
                ("GrossFootprintArea", "IFCQUANTITYAREA", footprint),
                ("GrossSideArea", "IFCQUANTITYAREA", length * wall.height),
                ("GrossVolume", "IFCQUANTITYVOLUME", footprint * wall.height),
            ]
            .map(|(name, entity, value)| {
                self.w
                    .add(format!("{entity}('{name}',$,$,{},$)", real(value)))
            });
            let qto = self.w.add(format!(
                "IFCELEMENTQUANTITY({},$,'Qto_WallBaseQuantities',$,$,{})",
                string(derived_guid(&wall.guid, "Qto_WallBaseQuantities").as_str()),
                list(&qs)
            ));
            self.w.add(format!(
                "IFCRELDEFINESBYPROPERTIES({},$,$,$,({w}),{qto})",
                string(derived_guid(&wall.guid, "rel Qto_WallBaseQuantities").as_str())
            ));

            contained.entry(wall.level).or_default().push(w);
            by_type.entry(wall.wall_type).or_default().push(w);
        }

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
    use archisim_core::{CutPattern, Layer, LayerFunction, NewWall, ReferenceLine};

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
        m.add_wall(NewWall {
            wall_type: t,
            level: lvl,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(5.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
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
