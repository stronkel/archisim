//! Footings and setting-out grids.
//!
//! A footing is an `IfcFooting` (`PAD_FOOTING`, `STRIP_FOOTING`,
//! `FOOTING_BEAM`) in the storey of its level: its outline in plan
//! extruded down from its top face, with `Pset_FootingCommon`, base
//! quantities and `ArchiSim_Footing` (the column or wall it is under) to
//! read it back.
//!
//! The grid lines are one `IfcGrid` in the lowest storey, an `IfcGridAxis`
//! for each line: those running like the first in `UAxes`, the others in
//! `VAxes` (IFC asks for both).

use std::collections::BTreeMap;

use archisim_core::{FootingKind, LevelId, MaterialId, Point2};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_footings(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        for f in model.footings() {
            let Some(&(_, storey_pl)) = storeys.get(&f.level) else {
                continue;
            };
            let outline = f.outline();
            if outline.len() < 3 {
                continue;
            }
            let curve = self.polyline(&outline, true);
            let profile = self
                .w
                .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{curve})"));
            let loc = self.point3(0.0, 0.0, f.top_offset - f.depth);
            let solid_pl = self.w.add(format!("IFCAXIS2PLACEMENT3D({loc},$,$)"));
            let solid = self.w.add(format!(
                "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                self.z,
                real(f.depth)
            ));
            let body = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"));
            let pl = self.placement(Some(storey_pl), self.origin, None);
            let predefined = match f.kind {
                FootingKind::Pad { .. } => "PAD_FOOTING",
                FootingKind::Strip { .. } => "STRIP_FOOTING",
                FootingKind::TieBeam { .. } => "FOOTING_BEAM",
            };
            let el = self.w.add(format!(
                "IFCFOOTING({},$,{},$,{},{pl},{shape},{},.{predefined}.)",
                string(f.guid.as_str()),
                string(&format!("{} {}", f.kind_name(), f.id.0)),
                string(f.kind_name()),
                string(&f.id.0.to_string()),
            ));
            self.hosts.insert(f.id.0, el);
            if let Some(m) = materials.get(&f.material) {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                    string(derived_guid(&f.guid, "material").as_str())
                ));
            }
            self.common_pset(
                el,
                f.id.0,
                &f.guid,
                "Pset_FootingCommon",
                f.kind_name(),
                false,
                true,
            );
            let area = f.volume() / f.depth;
            let (length, width) = match f.kind {
                FootingKind::Pad { length, .. } => (length, f.width),
                FootingKind::Strip { start, end, .. } | FootingKind::TieBeam { start, end } => {
                    (start.distance(end), f.width)
                }
            };
            self.quantities(
                el,
                &f.guid,
                "Qto_FootingBaseQuantities",
                &[
                    ("Length", "IFCQUANTITYLENGTH", length),
                    ("Width", "IFCQUANTITYLENGTH", width),
                    ("Height", "IFCQUANTITYLENGTH", f.depth),
                    ("CrossSectionArea", "IFCQUANTITYAREA", width * f.depth),
                    ("GrossVolume", "IFCQUANTITYVOLUME", area * f.depth),
                    ("NetVolume", "IFCQUANTITYVOLUME", area * f.depth),
                ],
            );
            // To read it back: what it is under.
            let mut props = vec![self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('TopOffset',$,IFCLENGTHMEASURE({}),$)",
                real(f.top_offset)
            ))];
            let host = match f.kind {
                FootingKind::Pad {
                    column: Some(c), ..
                } => model.column(c).map(|c| ("Column", c.guid.clone())),
                FootingKind::Strip { wall: Some(w), .. } => {
                    model.wall(w).map(|w| ("Wall", w.guid.clone()))
                }
                _ => None,
            };
            if let Some((what, guid)) = host {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE({},$,IFCIDENTIFIER({}),$)",
                    string(what),
                    string(guid.as_str())
                )));
            }
            self.pset(el, &f.guid, "ArchiSim_Footing", &props);
            contained.entry(f.level).or_default().push(el);
        }
    }

    pub(crate) fn write_grids(
        &mut self,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
        contained: &mut BTreeMap<LevelId, Vec<Ref>>,
    ) {
        let model = self.model;
        let lines: Vec<_> = model.grids().collect();
        if lines.is_empty() {
            return;
        }
        let Some(lowest) = model
            .levels()
            .iter()
            .min_by(|a, b| a.elevation.total_cmp(&b.elevation))
            .map(|l| l.id)
        else {
            return;
        };
        let Some(&(_, storey_pl)) = storeys.get(&lowest) else {
            return;
        };
        // Like the first line, or across it.
        let dir = |a: Point2, b: Point2| (b - a).normalized().unwrap_or(Point2::new(1.0, 0.0));
        let first = dir(lines[0].start, lines[0].end);
        let mut u: Vec<Ref> = Vec::new();
        let mut v: Vec<Ref> = Vec::new();
        let mut taken: Vec<String> = Vec::new();
        let mut curves: Vec<Ref> = Vec::new();
        let mut square = true;
        for g in &lines {
            let curve = self.polyline(&[g.start, g.end], false);
            curves.push(curve);
            // Tags must be unique in a grid.
            let mut tag = g.name.clone();
            let mut n = 2;
            while taken.contains(&tag) {
                tag = format!("{}.{n}", g.name);
                n += 1;
            }
            taken.push(tag.clone());
            let axis = self
                .w
                .add(format!("IFCGRIDAXIS({},{curve},.T.)", string(&tag)));
            let d = dir(g.start, g.end);
            let along = (d.x * first.x + d.y * first.y).abs();
            square &= !(1e-6..=1.0 - 1e-6).contains(&along);
            if along > 0.7 {
                u.push(axis);
            } else {
                v.push(axis);
            }
        }
        if v.is_empty() {
            if let Some(last) = (u.len() > 1).then(|| u.pop()).flatten() {
                v.push(last);
            } else {
                // A single line: IFC asks for two lists; one each way.
                let curve = self.polyline(&[lines[0].start, lines[0].end], false);
                v.push(self.w.add(format!("IFCGRIDAXIS('-',{curve},.T.)")));
            }
        }
        let set = self
            .w
            .add(format!("IFCGEOMETRICCURVESET({})", list(&curves)));
        let rep = self.w.add(format!(
            "IFCSHAPEREPRESENTATION({},'FootPrint','GeometricCurveSet',({set}))",
            self.axis_ctx
        ));
        let shape = self
            .w
            .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
        let pl = self.placement(Some(storey_pl), self.origin, None);
        let guid = derived_guid(&model.project_guid, "grid");
        let grid = self.w.add(format!(
            "IFCGRID({},$,'Ejes',$,$,{pl},{shape},{},{},$,.{}.)",
            string(guid.as_str()),
            list(&u),
            list(&v),
            if square { "RECTANGULAR" } else { "IRREGULAR" }
        ));
        contained.entry(lowest).or_default().push(grid);
    }
}
