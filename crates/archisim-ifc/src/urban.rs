//! Plots and roads, on the site (not in the building).
//!
//! Each plot is an `IfcSite` inside the project's site, with its boundary
//! as a `FootPrint`, its cadastral reference as `LandTitleNumber`,
//! `Qto_SiteBaseQuantities` and `ArchiSim_Plot` (setback and level) to read
//! it back.
//!
//! A road, in IFC 4.3, is an `IfcRoad` in the site holding an
//! `IfcPavement` for its carriageway, parking and sidewalks and an
//! `IfcKerb` for each kerb; in IFC4, which has none of them, those pieces
//! are proxies on the site. The carriageway carries `ArchiSim_Road` (axis
//! and section) to read the road back.

use std::collections::BTreeMap;

use archisim_core::{GlobalId, MaterialId, Point2, RoadBand};

use crate::step::{list, real, string, Ref};
use crate::{derived_guid, Exporter, Schema};

/// A piece of a road for IFC: its band, outline and material, how far it
/// reaches under and over the carriageway, and how high it is.
type Part = (RoadBand, Vec<Point2>, MaterialId, f64, f64, Height);

/// How high a piece of a road is.
#[derive(Clone)]
enum Height {
    /// On the road, following its profile.
    Road,
    /// On the road, lowered at a crossing (U4): its piece.
    Lowered(archisim_core::RoadPiece),
    /// A junction's piece, its top over the road's.
    Fixed(f64),
    /// A junction's piece following the roads that meet there (U4): the
    /// index of the junction.
    Junction(usize),
}

impl Exporter<'_> {
    pub(crate) fn write_urban(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        site: Ref,
        site_pl: Ref,
    ) {
        let model = self.model;
        let elevation = |l| model.level(l).map_or(0.0, |l| l.elevation);
        let level_guid = |l| {
            model
                .level(l)
                .map(|l| l.guid.to_string())
                .unwrap_or_default()
        };

        // Plots.
        let mut plots = Vec::new();
        for p in model.plots() {
            let loc = self.point3(0.0, 0.0, elevation(p.level));
            let pl = self.placement(Some(site_pl), loc, None);
            let curve = self.polyline(&p.outline, true);
            let set = self.w.add(format!("IFCGEOMETRICCURVESET(({curve}))"));
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'FootPrint','GeometricCurveSet',({set}))",
                self.axis_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let land = if p.reference.is_empty() {
                "$".to_string()
            } else {
                string(&p.reference)
            };
            let el = self.w.add(format!(
                "IFCSITE({},$,{},$,'Parcela',{pl},{shape},$,.PARTIAL.,$,$,$,{land},$)",
                string(p.guid.as_str()),
                string(&p.name)
            ));
            self.quantities(
                el,
                &p.guid,
                "Qto_SiteBaseQuantities",
                &[
                    ("GrossArea", "IFCQUANTITYAREA", p.area()),
                    ("GrossPerimeter", "IFCQUANTITYLENGTH", p.perimeter()),
                ],
            );
            let props = [
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Setback',$,IFCLENGTHMEASURE({}),$)",
                    real(p.setback)
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Level',$,IFCIDENTIFIER({}),$)",
                    string(&level_guid(p.level))
                )),
            ];
            self.pset(el, &p.guid, "ArchiSim_Plot", &props);
            plots.push(el);
        }
        if !plots.is_empty() {
            self.rel_aggregates(derived_guid(&model.site_guid, "plots"), site, &plots);
        }

        // Roads.
        let mut roads = Vec::new();
        let mut on_site = Vec::new();
        let mut alignments = Vec::new();
        let junctions: Vec<archisim_geom::crossroads::Junction> = model
            .levels()
            .iter()
            .flat_map(|l| archisim_geom::crossroads::junctions(model, l.id))
            .collect();
        for r in model.roads() {
            if self.schema == Schema::Ifc4x3 {
                alignments.push(self.write_alignment(r, elevation(r.level), site_pl));
            }
            let top = elevation(r.level) + r.top_offset;
            let loc = self.point3(0.0, 0.0, top);
            let road_pl = self.placement(Some(site_pl), loc, None);
            let road = match self.schema {
                Schema::Ifc4x3 => Some(self.w.add(format!(
                    "IFCROAD({},$,{},$,$,{road_pl},$,$,.ELEMENT.,.NOTDEFINED.)",
                    string(r.guid.as_str()),
                    string(&r.name)
                ))),
                Schema::Ifc4 => None,
            };
            let s = &r.section;
            let kh = if s.single_level { 0.0 } else { s.kerb_height };
            let mut pieces = Vec::new();
            // Its pieces but where its junctions are, and the junctions it
            // owns (U1): (band, outline, material, depth below and height
            // above the carriageway, the junction's top over the road's).
            let gaps = archisim_geom::crossroads::road_gaps(&junctions, r.id);
            let mut parts: Vec<Part> = r
                .pieces_except(&gaps)
                .into_iter()
                .map(|p| {
                    let (z0, z1) = match (p.band, p.raised) {
                        (RoadBand::Kerb, true) => (-s.depth, kh),
                        (_, true) => (kh - 0.2, kh),
                        _ => (-s.depth, 0.0),
                    };
                    let h = if p.lowered == archisim_core::Lowered::No {
                        Height::Road
                    } else {
                        Height::Lowered(p.clone())
                    };
                    (p.band, p.outline, p.material, z0, z1, h)
                })
                .collect();
            for (ji, j) in junctions
                .iter()
                .enumerate()
                .filter(|(_, j)| j.owner == r.id)
            {
                let draped = j.ring.is_none()
                    && j.arms
                        .iter()
                        .any(|a| model.road(a.road).is_some_and(|x| x.sloped()));
                let up = if draped {
                    Height::Junction(ji)
                } else {
                    Height::Fixed(j.top - r.top_offset)
                };
                let jk = j.kerb_height;
                parts.push((
                    RoadBand::Carriageway,
                    j.plate.clone(),
                    j.pavement,
                    -j.depth,
                    0.0,
                    up.clone(),
                ));
                for k in &j.kerbs {
                    parts.push((
                        RoadBand::Kerb,
                        k.clone(),
                        j.kerb_material,
                        -j.depth,
                        jk,
                        up.clone(),
                    ));
                }
                for w in &j.sidewalks {
                    parts.push((
                        RoadBand::Sidewalk,
                        w.clone(),
                        j.sidewalk_material,
                        jk - 0.2,
                        jk,
                        up.clone(),
                    ));
                }
                // A roundabout's island and apron (U3).
                for p in &j.parts {
                    parts.push((
                        p.band,
                        p.outline.clone(),
                        p.material,
                        p.z0,
                        p.z1,
                        up.clone(),
                    ));
                }
            }
            let sloped = r.sloped();
            for (i, (band, outline, material, z0, z1, height)) in parts.into_iter().enumerate() {
                let follow = match height {
                    Height::Road => sloped,
                    Height::Junction(_) | Height::Lowered(_) => true,
                    Height::Fixed(_) => false,
                };
                let shape = if follow {
                    // Following the profile: a body of triangles, heights
                    // from the road's placement.
                    let base = elevation(r.level) + r.top_offset;
                    let tris = archisim_geom::solid::draped_prism(&outline, |q| {
                        let top = match &height {
                            Height::Junction(ji) => junctions[*ji].top_under(model, q),
                            _ => r.top_under(q),
                        };
                        let t = elevation(r.level) + top;
                        match &height {
                            Height::Lowered(p) => {
                                let up = p.top_at(r.station_of(q), kh);
                                let under = if p.band == RoadBand::Kerb {
                                    z0
                                } else {
                                    up - 0.2
                                };
                                (t + under, t + up)
                            }
                            _ => (t + z0, t + z1),
                        }
                    });
                    if tris.is_empty() {
                        continue;
                    }
                    self.triangulated_shape(&tris, base)
                } else {
                    let up = match height {
                        Height::Fixed(u) => u,
                        _ => 0.0,
                    };
                    let curve = self.polyline(&outline, true);
                    let profile = self
                        .w
                        .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{curve})"));
                    let o = self.point3(0.0, 0.0, up + z0);
                    let solid_pl = self.w.add(format!("IFCAXIS2PLACEMENT3D({o},$,$)"));
                    let solid = self.w.add(format!(
                        "IFCEXTRUDEDAREASOLID({profile},{solid_pl},{},{})",
                        self.z,
                        real(z1 - z0)
                    ));
                    let body = self.w.add(format!(
                        "IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({solid}))",
                        self.body_ctx
                    ));
                    self.w
                        .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({body}))"))
                };
                let pl = self.placement(Some(road_pl), self.origin, None);
                let guid: GlobalId = derived_guid(&r.guid, &format!("band {i}"));
                let name = string(&format!("{} · {}", r.name, band.name()));
                let el =
                    match (self.schema, band) {
                        (Schema::Ifc4x3, RoadBand::Kerb) => self.w.add(format!(
                            "IFCKERB({},$,{name},$,$,{pl},{shape},$,.NOTDEFINED.)",
                            string(guid.as_str())
                        )),
                        (Schema::Ifc4x3, _) => self.w.add(format!(
                            "IFCPAVEMENT({},$,{name},$,$,{pl},{shape},$,.{}.)",
                            string(guid.as_str()),
                            if band.raised() { "RIGID" } else { "FLEXIBLE" }
                        )),
                        (Schema::Ifc4, _) => self.w.add(format!(
                        "IFCBUILDINGELEMENTPROXY({},$,{name},$,{},{pl},{shape},$,.USERDEFINED.)",
                        string(guid.as_str()),
                        string(if band == RoadBand::Kerb { "KERB" } else { "PAVEMENT" })
                    )),
                    };
                if let Some(m) = materials.get(&material) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                        string(derived_guid(&guid, "material").as_str())
                    ));
                }
                if pieces.is_empty() {
                    // To read the road back: its axis and section.
                    let axis: Vec<String> = r
                        .axis
                        .iter()
                        .map(|p: &Point2| format!("{},{}", real(p.x), real(p.y)))
                        .collect();
                    let mut props = vec![
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Road',$,IFCIDENTIFIER({}),$)",
                            string(r.guid.as_str())
                        )),
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('RoadName',$,IFCLABEL({}),$)",
                            string(&r.name)
                        )),
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Axis',$,IFCTEXT({}),$)",
                            string(&axis.join(";"))
                        )),
                        self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Level',$,IFCIDENTIFIER({}),$)",
                            string(&level_guid(r.level))
                        )),
                    ];
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Section',$,IFCTEXT({}),$)",
                        string(&s.encode())
                    )));
                    if !r.changes.is_empty() {
                        props.push(self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Changes',$,IFCTEXT({}),$)",
                            string(&r.encode_changes())
                        )));
                    }
                    if !r.profile.is_empty() {
                        props.push(self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('Profile',$,IFCTEXT({}),$)",
                            string(&r.encode_profile())
                        )));
                    }
                    for (key, v) in [
                        ("KerbHeight", s.kerb_height),
                        ("Depth", s.depth),
                        ("TopOffset", r.top_offset),
                    ] {
                        props.push(self.w.add(format!(
                            "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLENGTHMEASURE({}),$)",
                            real(v)
                        )));
                    }
                    let mats = [
                        ("Pavement", Some(r.pavement)),
                        ("KerbMaterial", Some(r.kerb_material)),
                        ("SidewalkMaterial", Some(r.sidewalk_material)),
                        ("PlantingMaterial", s.planting),
                        ("BikeMaterial", s.bike),
                        ("MarkingMaterial", s.markings),
                    ];
                    for (key, m) in mats.into_iter().filter_map(|(k, m)| Some((k, m?))) {
                        if let Some(x) = model.material(m) {
                            props.push(self.w.add(format!(
                                "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLABEL({}),$)",
                                string(&x.name)
                            )));
                        }
                    }
                    self.pset(el, &guid, "ArchiSim_Road", &props);
                }
                pieces.push(el);
            }
            match road {
                Some(road) => {
                    self.w.add(format!(
                        "IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,{},{road})",
                        string(derived_guid(&r.guid, "contains").as_str()),
                        list(&pieces)
                    ));
                    roads.push(road);
                }
                None => on_site.extend(pieces),
            }
        }
        // Roundabouts and squares (U3): their circle and what they are, to
        // read them back (their pieces go with the junction's road).
        for c in model.roundabouts() {
            let loc = self.point3(0.0, 0.0, elevation(c.level) + c.top_offset);
            let pl = self.placement(Some(site_pl), loc, None);
            let curve = self.polyline(&c.outline(), true);
            let set = self.w.add(format!("IFCGEOMETRICCURVESET(({curve}))"));
            let rep = self.w.add(format!(
                "IFCSHAPEREPRESENTATION({},'FootPrint','GeometricCurveSet',({set}))",
                self.axis_ctx
            ));
            let shape = self
                .w
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
            let el = self.w.add(format!(
                "IFCBUILDINGELEMENTPROXY({},$,{},$,{},{pl},{shape},$,.USERDEFINED.)",
                string(c.guid.as_str()),
                string(&c.name),
                string(if c.plaza { "PLAZA" } else { "ROUNDABOUT" })
            ));
            let mut props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Centre',$,IFCTEXT({}),$)",
                    string(&format!("{},{}", real(c.centre.x), real(c.centre.y)))
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Shape',$,IFCTEXT({}),$)",
                    string(&c.encode())
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Level',$,IFCIDENTIFIER({}),$)",
                    string(&level_guid(c.level))
                )),
            ];
            for (key, m) in [
                ("Pavement", c.pavement),
                ("KerbMaterial", c.kerb_material),
                ("IslandMaterial", c.island_material),
            ] {
                if let Some(x) = model.material(m) {
                    props.push(self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('{key}',$,IFCLABEL({}),$)",
                        string(&x.name)
                    )));
                }
            }
            self.pset(el, &c.guid, "ArchiSim_Roundabout", &props);
            on_site.push(el);
        }
        // Road markings at will (U4): their paint as triangles over the
        // pavement, and what they are to read them back.
        for m in model.markings() {
            let base = elevation(m.level);
            let tris: Vec<[[f64; 3]; 3]> = archisim_geom::solid::marking_solids(model, m.id)
                .into_iter()
                .flat_map(|x| x.triangles)
                .collect();
            if tris.is_empty() {
                continue;
            }
            let loc = self.point3(0.0, 0.0, base);
            let pl = self.placement(Some(site_pl), loc, None);
            let shape = self.triangulated_shape(&tris, base);
            let el = self.w.add(format!(
                "IFCBUILDINGELEMENTPROXY({},$,{},$,'MARKING',{pl},{shape},$,.USERDEFINED.)",
                string(m.guid.as_str()),
                string(m.kind.name())
            ));
            if let Some(x) = materials.get(&m.material) {
                self.w.add(format!(
                    "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{x})",
                    string(derived_guid(&m.guid, "material").as_str())
                ));
            }
            let path: Vec<String> = m
                .path
                .iter()
                .map(|p| format!("{},{}", real(p.x), real(p.y)))
                .collect();
            let mut props = vec![
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Kind',$,IFCIDENTIFIER({}),$)",
                    string(m.kind.code())
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Path',$,IFCTEXT({}),$)",
                    string(&path.join(";"))
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Sizes',$,IFCTEXT({}),$)",
                    string(&m.encode())
                )),
                self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Level',$,IFCIDENTIFIER({}),$)",
                    string(&level_guid(m.level))
                )),
            ];
            if let Some(x) = model.material(m.material) {
                props.push(self.w.add(format!(
                    "IFCPROPERTYSINGLEVALUE('Paint',$,IFCLABEL({}),$)",
                    string(&x.name)
                )));
            }
            self.pset(el, &m.guid, "ArchiSim_Marking", &props);
            on_site.push(el);
        }
        if !roads.is_empty() {
            self.rel_aggregates(derived_guid(&model.site_guid, "roads"), site, &roads);
        }
        if !alignments.is_empty() {
            self.w.add(format!(
                "IFCRELREFERENCEDINSPATIALSTRUCTURE({},$,$,$,{},{site})",
                string(derived_guid(&model.site_guid, "alignments").as_str()),
                list(&alignments)
            ));
        }
        if !on_site.is_empty() {
            self.w.add(format!(
                "IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,{},{site})",
                string(derived_guid(&model.site_guid, "contains").as_str()),
                list(&on_site)
            ));
        }
    }

    /// The road's axis and profile as an IFC 4.3 alignment (U1): its
    /// horizontal layout (a straight segment per stretch of the axis) and
    /// its vertical one (constant grades and parabolic curves), nested.
    fn write_alignment(&mut self, r: &archisim_core::Road, level_z: f64, site_pl: Ref) -> Ref {
        let g = &r.guid;
        let mut h_segs = Vec::new();
        let mut s_at = 0.0;
        for (k, w) in r.axis.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let len = a.distance(b);
            if len < 1e-9 {
                continue;
            }
            let p = self
                .w
                .add(format!("IFCCARTESIANPOINT(({},{}))", real(a.x), real(a.y)));
            let dir = (b.y - a.y).atan2(b.x - a.x);
            let design = self.w.add(format!(
                "IFCALIGNMENTHORIZONTALSEGMENT($,$,{p},{},0.,0.,{},$,.LINE.)",
                real(dir),
                real(len)
            ));
            h_segs.push(self.w.add(format!(
                "IFCALIGNMENTSEGMENT({},$,$,$,$,$,$,{design})",
                string(derived_guid(g, &format!("h segment {k}")).as_str())
            )));
            s_at += len;
        }
        let horizontal = self.w.add(format!(
            "IFCALIGNMENTHORIZONTAL({},$,$,$,$,$,$)",
            string(derived_guid(g, "horizontal").as_str())
        ));
        self.w.add(format!(
            "IFCRELNESTS({},$,$,$,{horizontal},{})",
            string(derived_guid(g, "horizontal segments").as_str()),
            list(&h_segs)
        ));
        // The vertical layout: grades between the curves' ends, and the
        // curves; level all along with no profile.
        let len = s_at;
        let mut stops: Vec<(f64, bool)> = vec![(0.0, false)];
        if r.sloped() {
            let mut p = r.profile.clone();
            p.sort_by(|a, b| a.at.total_cmp(&b.at));
            for v in p.iter().skip(1).take(p.len().saturating_sub(2)) {
                let half = v.curve / 2.0;
                if half > 1e-6 {
                    stops.push((v.at - half, true));
                    stops.push((v.at + half, false));
                } else {
                    stops.push((v.at, false));
                }
            }
        }
        stops.push((len, false));
        stops.retain(|x| x.0 >= -1e-9 && x.0 <= len + 1e-9);
        stops.sort_by(|a, b| a.0.total_cmp(&b.0));
        stops.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-6);
        let slope_at = |s: f64| {
            let e = 1e-3;
            (r.top_at((s + e).min(len)) - r.top_at((s - e).max(0.0)))
                / ((s + e).min(len) - (s - e).max(0.0))
        };
        let mut v_segs = Vec::new();
        for (k, w) in stops.windows(2).enumerate() {
            let (s0, s1) = (w[0].0, w[1].0);
            if s1 - s0 < 1e-6 {
                continue;
            }
            let curve = w[0].1;
            let (g0, g1) = (slope_at(s0 + 1e-3), slope_at(s1 - 1e-3));
            let (kind, radius) = if curve && (g1 - g0).abs() > 1e-9 {
                ("PARABOLICARC", real((s1 - s0) / (g1 - g0)))
            } else {
                ("CONSTANTGRADIENT", "$".to_string())
            };
            let design = self.w.add(format!(
                "IFCALIGNMENTVERTICALSEGMENT($,$,{},{},{},{},{},{radius},.{kind}.)",
                real(s0),
                real(s1 - s0),
                real(level_z + r.top_at(s0)),
                real(g0),
                real(g1)
            ));
            v_segs.push(self.w.add(format!(
                "IFCALIGNMENTSEGMENT({},$,$,$,$,$,$,{design})",
                string(derived_guid(g, &format!("v segment {k}")).as_str())
            )));
        }
        let vertical = self.w.add(format!(
            "IFCALIGNMENTVERTICAL({},$,$,$,$,$,$)",
            string(derived_guid(g, "vertical").as_str())
        ));
        self.w.add(format!(
            "IFCRELNESTS({},$,$,$,{vertical},{})",
            string(derived_guid(g, "vertical segments").as_str()),
            list(&v_segs)
        ));
        let pl = self.placement(Some(site_pl), self.origin, None);
        let alignment = self.w.add(format!(
            "IFCALIGNMENT({},$,{},$,$,{pl},$,.NOTDEFINED.)",
            string(derived_guid(g, "alignment").as_str()),
            string(&format!("Eje · {}", r.name))
        ));
        self.w.add(format!(
            "IFCRELNESTS({},$,$,$,{alignment},({horizontal},{vertical}))",
            string(derived_guid(g, "alignment layouts").as_str())
        ));
        alignment
    }
}
