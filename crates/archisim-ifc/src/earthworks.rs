//! Earthworks (bloque Terreno): what the platforms of a terrain dig and
//! fill and what the pools dig, each a closed triangulated solid in the
//! site. IFC 4.3: `IfcEarthworksCut` (`CUT` for platforms and ramps,
//! `EXCAVATION` for basements and pools) and `IfcEarthworksFill`
//! (`EMBANKMENT`), with `Qto_EarthworksCutBaseQuantities` /
//! `Qto_EarthworksFillBaseQuantities`. IFC4 has neither: they go down to
//! `IfcBuildingElementProxy` (`USERDEFINED`, «Desmonte» / «Terraplén»)
//! with the same quantities in `ArchiSim_EarthworksQuantities`.
//!
//! A cut is a subtraction (`IfcFeatureElementSubtraction`): IFC 4.3 wants
//! it to void an element and not to be in a spatial structure, so it
//! voids its terrain (`IfcRelVoidsElement` to the `IfcGeographicElement`:
//! the ground it took away, which lies over the graded ground, or in the
//! hole of a pool already left out of it, so nothing changes in its
//! body). A fill, and the IFC4 proxies, are contained in the site.
//!
//! All carry `ArchiSim_Earthworks`, so the importer leaves them out (they
//! are made again from the terrain and the pools).

use std::collections::BTreeMap;

use archisim_core::{EarthSide, EarthUse, Earthwork, MaterialId, PadKind};

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter, Schema};

/// Rounded to the millimetre.
fn mm(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The predefined type of an earthwork in IFC 4.3.
pub(crate) fn predefined(e: &Earthwork) -> &'static str {
    match (e.side, e.what) {
        (EarthSide::Cut, EarthUse::Pool) | (EarthSide::Cut, EarthUse::Pad(PadKind::Basement)) => {
            "EXCAVATION"
        }
        (EarthSide::Cut, _) => "CUT",
        (EarthSide::Fill, _) => "EMBANKMENT",
    }
}

impl Exporter<'_> {
    /// Writes every earthwork; returns those to be contained in the site
    /// (the cuts void their terrain instead).
    pub(crate) fn write_earthworks(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        site_pl: Ref,
    ) -> Vec<Ref> {
        let model = self.model;
        let mut out = Vec::new();
        let written = self.w.by_guid();
        for e in model.earthworks() {
            let Some(terrain) = model.terrain(e.terrain) else {
                continue;
            };
            let earth = Some(terrain.material);
            // The terrain a cut voids (in IFC 4.3).
            let voided = written.get(terrain.guid.as_str()).copied();
            if self.schema == Schema::Ifc4x3 && e.side == EarthSide::Cut && voided.is_none() {
                continue;
            }
            let mesh = &e.mesh;
            let pts: Vec<String> = mesh
                .points
                .iter()
                .map(|p| format!("({},{},{})", real(p[0]), real(p[1]), real(p[2])))
                .collect();
            let faces: Vec<String> = mesh
                .triangles
                .iter()
                .map(|t| format!("({},{},{})", t[0] + 1, t[1] + 1, t[2] + 1))
                .collect();
            let list3 = self
                .w
                .add(format!("IFCCARTESIANPOINTLIST3D(({}),$)", pts.join(",")));
            let body = self.w.add(format!(
                "IFCTRIANGULATEDFACESET({list3},$,.T.,({}),$)",
                faces.join(",")
            ));
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'Body','Tessellation',({body}))",
                self.body_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let pl = self.placement(Some(site_pl), self.origin, None);
            let guid = derived_guid(&e.owner, &e.key);
            let (word, what) = match e.side {
                EarthSide::Cut => ("Desmonte", "Cut"),
                EarthSide::Fill => ("Terraplén", "Fill"),
            };
            let kind = predefined(&e);
            let el = match (self.schema, e.side) {
                (Schema::Ifc4x3, EarthSide::Cut) => self.w.add(format!(
                    "IFCEARTHWORKSCUT({},$,{},$,$,{pl},{shape},$,.{kind}.)",
                    string(guid.as_str()),
                    string(&e.name),
                )),
                (Schema::Ifc4x3, EarthSide::Fill) => self.w.add(format!(
                    "IFCEARTHWORKSFILL({},$,{},$,$,{pl},{shape},$,.{kind}.)",
                    string(guid.as_str()),
                    string(&e.name),
                )),
                (Schema::Ifc4, _) => self.w.add(format!(
                    "IFCBUILDINGELEMENTPROXY({},$,{},$,{},{pl},{shape},$,.USERDEFINED.)",
                    string(guid.as_str()),
                    string(&e.name),
                    string(word),
                )),
            };
            // (A cut takes ground away: IFC allows it no material.)
            let fill = e.side == EarthSide::Fill || self.schema == Schema::Ifc4;
            if let Some(m) = earth.and_then(|m| materials.get(&m)).filter(|_| fill) {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                    string(derived_guid(&guid, "material").as_str())
                ));
            }
            let volume = (mesh.volume * 1000.0).round() / 1000.0;
            let (length, width) = {
                let (dx, dy) = (mesh.hi.x - mesh.lo.x, mesh.hi.y - mesh.lo.y);
                (dx.max(dy), dx.min(dy))
            };
            let volume_name = match e.side {
                EarthSide::Cut => "UndisturbedVolume",
                EarthSide::Fill => "CompactedVolume",
            };
            let qto = match (self.schema, e.side) {
                (Schema::Ifc4x3, EarthSide::Cut) => "Qto_EarthworksCutBaseQuantities",
                (Schema::Ifc4x3, EarthSide::Fill) => "Qto_EarthworksFillBaseQuantities",
                (Schema::Ifc4, _) => "ArchiSim_EarthworksQuantities",
            };
            self.quantities(
                el,
                &guid,
                qto,
                &[
                    ("Length", "IFCQUANTITYLENGTH", mm(length)),
                    ("Width", "IFCQUANTITYLENGTH", mm(width)),
                    ("Depth", "IFCQUANTITYLENGTH", mm(mesh.depth)),
                    (volume_name, "IFCQUANTITYVOLUME", volume),
                ],
            );
            let (source, index) = match e.source {
                archisim_core::EarthSource::Pad { pad, .. } => ("Pad", Some(pad)),
                archisim_core::EarthSource::Pool(_) => ("Pool", None),
                archisim_core::EarthSource::Object(_) => ("PoolObject", None),
            };
            let use_name = match e.what {
                EarthUse::Pool => "pool",
                EarthUse::Pad(k) => k.code(),
            };
            let mut props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Kind',$,IFCLABEL({}),$)",
                    string(what)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Source',$,IFCLABEL({}),$)",
                    string(source)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Use',$,IFCLABEL({}),$)",
                    string(use_name)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Volume',$,IFCVOLUMEMEASURE({}),$)",
                    real(volume)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('PlanArea',$,IFCAREAMEASURE({}),$)",
                    real((mesh.area * 100.0).round() / 100.0)
                )),
            ];
            if let Some(i) = index {
                props.push(
                    self.w
                        .add(format!("IFCPROPERTYSINGLEVALUE('Pad',$,IFCINTEGER({i}),$)")),
                );
            }
            self.pset(el, &guid, "ArchiSim_Earthworks", &props);
            match (self.schema, e.side, voided) {
                (Schema::Ifc4x3, EarthSide::Cut, Some(ground)) => {
                    self.w.add(format!(
                        "IFCRELVOIDSELEMENT({},$,$,$,{ground},{el})",
                        string(derived_guid(&guid, "voids").as_str())
                    ));
                }
                _ => out.push(el),
            }
        }
        out
    }
}
