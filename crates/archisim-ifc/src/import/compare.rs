//! Compares two models element by element, matching them by `GlobalId`:
//! what a round trip through IFC must keep.

use archisim_core::{Layer, MaterialId, Model, Point2};

/// Lengths closer than this are equal (0.001 mm).
const TOL: f64 = 1e-6;

fn layers_text(model: &Model, layers: &[Layer]) -> String {
    layers
        .iter()
        .map(|l| {
            let m = model.material(l.material);
            format!(
                "{} {:.6} {:?} [{}/{}/{:?}/{:?}]",
                m.map_or("?", |m| m.name.as_str()),
                l.thickness,
                l.function,
                m.map_or("", |m| m.category.as_str()),
                m.map_or(String::new(), |m| format!("{:?}", m.cut_color)),
                m.map(|m| m.cut_pattern),
                m.and_then(|m| m.priority),
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOL
}

/// Differences between `a` and `b`, in words. Empty when they describe
/// the same building (ids may differ; GUIDs, geometry and types may not).
pub fn differences(a: &Model, b: &Model) -> Vec<String> {
    let mut out = Vec::new();
    let mut diff = |what: String| out.push(what);
    if a.project_name != b.project_name {
        diff(format!(
            "nombre del proyecto: «{}» / «{}»",
            a.project_name, b.project_name
        ));
    }
    for (x, y, what) in [
        (&a.project_guid, &b.project_guid, "proyecto"),
        (&a.site_guid, &b.site_guid, "emplazamiento"),
        (&a.building_guid, &b.building_guid, "edificio"),
    ] {
        if x != y {
            diff(format!("GlobalId del {what}"));
        }
    }

    let la = a.levels();
    let lb = b.levels();
    if la.len() != lb.len() {
        diff(format!("niveles: {} / {}", la.len(), lb.len()));
    }
    for l in &la {
        match lb.iter().find(|m| m.guid == l.guid) {
            None => diff(format!("nivel «{}» ({}) falta", l.name, l.guid)),
            Some(m) => {
                if m.name != l.name || !close(m.elevation, l.elevation) {
                    diff(format!(
                        "nivel «{}» {} / «{}» {}",
                        l.name, l.elevation, m.name, m.elevation
                    ));
                }
            }
        }
    }
    let level_guid = |m: &Model, id| m.level(id).map(|l| l.guid.clone());

    let count = |m: &Model| (m.walls().count(), m.slabs().count(), m.openings().count());
    if count(a) != count(b) {
        diff(format!(
            "número de muros, forjados y huecos: {:?} / {:?}",
            count(a),
            count(b)
        ));
    }

    for w in a.walls() {
        let Some(v) = b.walls().find(|v| v.guid == w.guid) else {
            diff(format!("muro {} falta", w.guid));
            continue;
        };
        let (ta, tb) = (a.wall_type(w.wall_type), b.wall_type(v.wall_type));
        let same_points = w.start.distance(v.start) <= TOL && w.end.distance(v.end) <= TOL;
        let same_bend = (w.bulge - v.bulge).abs() <= 1e-6 * w.bulge.abs().max(1.0);
        if !same_points
            || !same_bend
            || w.reference != v.reference
            || !close(w.height, v.height)
            || !close(w.base_offset, v.base_offset)
            || level_guid(a, w.level) != level_guid(b, v.level)
        {
            diff(format!("muro {}: {:?} / {:?}", w.guid, w, v));
        }
        match (ta, tb) {
            (Some(ta), Some(tb)) => {
                if ta.guid != tb.guid
                    || ta.name != tb.name
                    || ta.function != tb.function
                    || layers_text(a, &ta.layers) != layers_text(b, &tb.layers)
                {
                    diff(format!(
                        "tipo del muro {}: «{}» {:?} [{}] / «{}» {:?} [{}]",
                        w.guid,
                        ta.name,
                        ta.function,
                        layers_text(a, &ta.layers),
                        tb.name,
                        tb.function,
                        layers_text(b, &tb.layers)
                    ));
                }
            }
            _ => diff(format!("tipo del muro {} falta", w.guid)),
        }
    }

    for s in a.slabs() {
        let Some(t) = b.slabs().find(|t| t.guid == s.guid) else {
            diff(format!("forjado {} falta", s.guid));
            continue;
        };
        // The same loop, from whichever corner it starts.
        let n = s.outline.len();
        let shift = t
            .outline
            .iter()
            .position(|q| s.outline.first().is_some_and(|p| p.distance(*q) <= TOL));
        let same_outline = n == t.outline.len()
            && shift.is_some_and(|k| {
                (0..n).all(|i| s.outline[i].distance(t.outline[(i + k) % n]) <= TOL)
            });
        // The same top face (level or inclined), whatever its arrow.
        let same_top = s.outline.iter().all(|p| close(s.top_at(*p), t.top_at(*p)))
            && s.slope.is_some() == t.slope.is_some();
        if !same_outline || !same_top || level_guid(a, s.level) != level_guid(b, t.level) {
            diff(format!("forjado {}: {:?} / {:?}", s.guid, s, t));
        }
        match (a.slab_type(s.slab_type), b.slab_type(t.slab_type)) {
            (Some(ta), Some(tb)) => {
                if ta.guid != tb.guid
                    || ta.name != tb.name
                    || ta.function != tb.function
                    || layers_text(a, &ta.layers) != layers_text(b, &tb.layers)
                {
                    diff(format!(
                        "tipo del forjado {}: «{}» [{}] / «{}» [{}]",
                        s.guid,
                        ta.name,
                        layers_text(a, &ta.layers),
                        tb.name,
                        layers_text(b, &tb.layers)
                    ));
                }
            }
            _ => diff(format!("tipo del forjado {} falta", s.guid)),
        }
    }

    for o in a.openings() {
        let Some(p) = b.openings().find(|p| p.guid == o.guid) else {
            diff(format!("puerta o ventana {} falta", o.guid));
            continue;
        };
        let wall_guid = |m: &Model, id| m.wall(id).map(|w| w.guid.clone());
        let setback_same = match (o.setback, p.setback) {
            (None, None) => true,
            (Some(x), Some(y)) => close(x, y),
            _ => false,
        };
        if !close(o.along, p.along)
            || !close(o.sill, p.sill)
            || o.opens_to != p.opens_to
            || o.hand != p.hand
            || !setback_same
            || wall_guid(a, o.wall) != wall_guid(b, p.wall)
        {
            diff(format!("puerta o ventana {}: {:?} / {:?}", o.guid, o, p));
        }
        match (
            a.opening_type(o.opening_type),
            b.opening_type(p.opening_type),
        ) {
            (Some(x), Some(y)) => {
                let mat = |m: &Model, id| m.material(id).map(|m| m.name.clone());
                if x.guid != y.guid
                    || x.name != y.name
                    || x.kind != y.kind
                    || !close(x.width, y.width)
                    || !close(x.height, y.height)
                    || !close(x.frame_width, y.frame_width)
                    || !close(x.frame_depth, y.frame_depth)
                    || mat(a, x.frame_material) != mat(b, y.frame_material)
                    || mat(a, x.panel_material) != mat(b, y.panel_material)
                    || x.mark != y.mark
                    || x.description != y.description
                    || x.panels != y.panels
                {
                    diff(format!("tipo de {}: {:?} / {:?}", o.guid, x, y));
                }
                // The same family (recipe) and values, materials by name.
                let fam = |m: &Model, t: &archisim_core::OpeningType| {
                    m.family(t.family).map(|f| f.source.clone())
                };
                if fam(a, x) != fam(b, y) {
                    diff(format!("familia del tipo «{}» distinta", x.name));
                }
                let named = |m: &Model, v: &archisim_core::Values| -> Vec<(String, String)> {
                    v.iter()
                        .map(|(k, v)| {
                            let text = match v {
                                archisim_core::Value::Material(id) => m
                                    .material(MaterialId(*id))
                                    .map_or(String::new(), |m| m.name.clone()),
                                other => format!("{other}"),
                            };
                            (k.clone(), text)
                        })
                        .collect()
                };
                if named(a, &x.values) != named(b, &y.values) {
                    diff(format!(
                        "valores del tipo «{}»: {:?} / {:?}",
                        x.name,
                        named(a, &x.values),
                        named(b, &y.values)
                    ));
                }
                if named(a, &o.values) != named(b, &p.values) {
                    diff(format!("valores de ejemplar de {}", o.guid));
                }
            }
            _ => diff(format!("tipo de {} falta", o.guid)),
        }
    }

    // Phases (by GlobalId).
    let phases = |m: &Model| -> Vec<(String, archisim_core::Phase)> {
        let mut v: Vec<(String, archisim_core::Phase)> = Vec::new();
        for w in m.walls() {
            v.push((w.guid.to_string(), m.phase(w.id.0)));
        }
        for s in m.slabs() {
            v.push((s.guid.to_string(), m.phase(s.id.0)));
        }
        for o in m.openings() {
            v.push((o.guid.to_string(), m.phase(o.id.0)));
        }
        for c in m.columns() {
            v.push((c.guid.to_string(), m.phase(c.id.0)));
        }
        for b in m.beams() {
            v.push((b.guid.to_string(), m.phase(b.id.0)));
        }
        for r in m.roofs() {
            v.push((r.guid.to_string(), m.phase(r.id.0)));
        }
        for s in m.stairs() {
            v.push((s.guid.to_string(), m.phase(s.id.0)));
        }
        v.sort();
        v
    };
    if phases(a) != phases(b) {
        diff("fases de los elementos distintas".to_string());
    }

    // Rooms.
    if a.rooms().count() != b.rooms().count() {
        diff(format!(
            "número de locales: {} / {}",
            a.rooms().count(),
            b.rooms().count()
        ));
    }
    let same_geo = match (a.georeference(), b.georeference()) {
        (None, None) => true,
        (Some(g), Some(h)) => {
            g.crs == h.crs
                && (g.eastings - h.eastings).abs() < 1e-3
                && (g.northings - h.northings).abs() < 1e-3
                && (g.height - h.height).abs() < 1e-3
        }
        _ => false,
    };
    if !same_geo {
        diff(format!(
            "georreferencia: {:?} / {:?}",
            a.georeference(),
            b.georeference()
        ));
    }
    if (a.true_north() - b.true_north()).abs() > 1e-6 {
        diff(format!("norte: {} / {}", a.true_north(), b.true_north()));
    }
    if a.terrains().count() != b.terrains().count() {
        diff(format!(
            "número de terrenos: {} / {}",
            a.terrains().count(),
            b.terrains().count()
        ));
    }
    for s in a.terrains() {
        let Some(t) = b.terrains().find(|t| t.guid == s.guid) else {
            diff(format!("terreno {} falta", s.guid));
            continue;
        };
        let same = s.points.len() == t.points.len()
            && s.points
                .iter()
                .zip(&t.points)
                .all(|(p, q)| p.at.distance(q.at) <= TOL && close(p.z, q.z))
            && close(s.depth, t.depth)
            && close(s.contour, t.contour)
            && s.lines == t.lines
            && level_guid(a, s.level) == level_guid(b, t.level);
        if !same {
            diff(format!("terreno {}: {:?} / {:?}", s.guid, s, t));
        }
    }
    if a.footings().count() != b.footings().count() {
        diff(format!(
            "número de cimentaciones: {} / {}",
            a.footings().count(),
            b.footings().count()
        ));
    }
    for s in a.footings() {
        let Some(t) = b.footings().find(|t| t.guid == s.guid) else {
            diff(format!("cimentación {} falta", s.guid));
            continue;
        };
        let (os, ot) = (s.outline(), t.outline());
        let host = |m: &Model, f: &archisim_core::Footing| match f.kind {
            archisim_core::FootingKind::Pad {
                column: Some(c), ..
            } => m.column(c).map(|c| c.guid.to_string()),
            archisim_core::FootingKind::Strip { wall: Some(w), .. } => {
                m.wall(w).map(|w| w.guid.to_string())
            }
            _ => None,
        };
        let same = os.len() == ot.len()
            && os.iter().zip(&ot).all(|(p, q)| p.distance(*q) <= TOL)
            && close(s.depth, t.depth)
            && close(s.top_offset, t.top_offset)
            && std::mem::discriminant(&s.kind) == std::mem::discriminant(&t.kind)
            && host(a, s) == host(b, t)
            && level_guid(a, s.level) == level_guid(b, t.level);
        if !same {
            diff(format!("cimentación {}: {:?} / {:?}", s.guid, s, t));
        }
    }
    let grids = |m: &Model| {
        let mut v: Vec<(String, [i64; 4])> = m
            .grids()
            .map(|g| {
                let r = |x: f64| (x * 1000.0).round() as i64;
                (
                    g.name.clone(),
                    [r(g.start.x), r(g.start.y), r(g.end.x), r(g.end.y)],
                )
            })
            .collect();
        v.sort();
        v
    };
    if grids(a) != grids(b) {
        diff(format!("ejes: {:?} / {:?}", grids(a), grids(b)));
    }
    if a.plots().count() != b.plots().count() {
        diff(format!(
            "número de parcelas: {} / {}",
            a.plots().count(),
            b.plots().count()
        ));
    }
    for s in a.plots() {
        let Some(t) = b.plots().find(|t| t.guid == s.guid) else {
            diff(format!("parcela {} falta", s.guid));
            continue;
        };
        let same = s.outline.len() == t.outline.len()
            && s.outline
                .iter()
                .zip(&t.outline)
                .all(|(p, q)| p.distance(*q) <= TOL)
            && close(s.setback, t.setback)
            && s.name == t.name
            && s.reference == t.reference
            && level_guid(a, s.level) == level_guid(b, t.level);
        if !same {
            diff(format!("parcela {}: {:?} / {:?}", s.guid, s, t));
        }
    }
    let runs = |m: &Model| {
        let mut v: Vec<RunKey> = m
            .runs()
            .map(|r| {
                let q = |x: f64| (x * 1000.0).round() as i64;
                (
                    r.guid.to_string(),
                    r.system.code().to_string(),
                    r.path.iter().map(|p| [q(p.x), q(p.y)]).collect(),
                    [q(r.elevation), q(r.width), q(r.height), q(r.slope * 100.0)],
                )
            })
            .collect();
        v.sort();
        v
    };
    if runs(a) != runs(b) {
        diff(format!(
            "tuberías y conductos: {:?} / {:?}",
            runs(a),
            runs(b)
        ));
    }
    let devices = |m: &Model| {
        let mut v: Vec<(String, String, [i64; 3])> = m
            .devices()
            .map(|d| {
                let q = |x: f64| (x * 1000.0).round() as i64;
                (
                    d.guid.to_string(),
                    d.kind.code().to_string(),
                    [q(d.position.x), q(d.position.y), q(d.elevation)],
                )
            })
            .collect();
        v.sort();
        v
    };
    if devices(a) != devices(b) {
        diff(format!("aparatos: {:?} / {:?}", devices(a), devices(b)));
    }
    if a.roads().count() != b.roads().count() {
        diff(format!(
            "número de viales: {} / {}",
            a.roads().count(),
            b.roads().count()
        ));
    }
    for s in a.roads() {
        let Some(t) = b.roads().find(|t| t.guid == s.guid) else {
            diff(format!("vial {} falta", s.guid));
            continue;
        };
        let (x, y) = (s.section, t.section);
        let same = s.axis.len() == t.axis.len()
            && s.axis
                .iter()
                .zip(&t.axis)
                .all(|(p, q)| p.distance(*q) <= TOL)
            && close(x.carriageway, y.carriageway)
            && close(x.parking, y.parking)
            && close(x.kerb, y.kerb)
            && close(x.sidewalk, y.sidewalk)
            && close(s.top_offset, t.top_offset)
            && s.name == t.name
            && level_guid(a, s.level) == level_guid(b, t.level);
        if !same {
            diff(format!("vial {}: {:?} / {:?}", s.guid, s, t));
        }
    }
    if a.objects().count() != b.objects().count() {
        diff(format!(
            "número de objetos: {} / {}",
            a.objects().count(),
            b.objects().count()
        ));
    }
    for s in a.objects() {
        let Some(t) = b.objects().find(|t| t.guid == s.guid) else {
            diff(format!("objeto {} falta", s.guid));
            continue;
        };
        let kind = |m: &Model, o: &archisim_core::Object| {
            m.object_type(o.object_type).map(|ty| {
                let fam = m.family(ty.family).map(|f| f.name().to_string());
                let values: Vec<String> = ty
                    .values
                    .iter()
                    .map(|(k, v)| match v {
                        archisim_core::Value::Material(id) => {
                            format!("{k}={:?}", material_name(m, archisim_core::MaterialId(*id)))
                        }
                        archisim_core::Value::Number(x) => format!("{k}={x:.6}"),
                        other => format!("{k}={other:?}"),
                    })
                    .collect();
                (ty.name.clone(), fam, values)
            })
        };
        let same = s.position.distance(t.position) <= TOL
            && (s.rotation - t.rotation).sin().abs() < 1e-6
            && (s.rotation - t.rotation).cos() > 0.0
            && close(s.base_offset, t.base_offset)
            && level_guid(a, s.level) == level_guid(b, t.level)
            && kind(a, s) == kind(b, t);
        if !same {
            diff(format!("objeto {}: {:?} / {:?}", s.guid, s, t));
        }
    }
    if a.railings().count() != b.railings().count() {
        diff(format!(
            "número de barandillas: {} / {}",
            a.railings().count(),
            b.railings().count()
        ));
    }
    for s in a.railings() {
        let Some(t) = b.railings().find(|t| t.guid == s.guid) else {
            diff(format!("barandilla {} falta", s.guid));
            continue;
        };
        let same = s.path.len() == t.path.len()
            && s.path
                .iter()
                .zip(&t.path)
                .all(|(p, q)| p.distance(*q) <= TOL)
            && close(s.base_offset, t.base_offset)
            && close(s.height, t.height)
            && close(s.post_spacing, t.post_spacing)
            && level_guid(a, s.level) == level_guid(b, t.level)
            && material_name(a, s.material) == material_name(b, t.material)
            && s.glass.and_then(|g| material_name(a, g))
                == t.glass.and_then(|g| material_name(b, g));
        if !same {
            diff(format!("barandilla {}: {:?} / {:?}", s.guid, s, t));
        }
    }
    if a.curtain_walls().count() != b.curtain_walls().count() {
        diff(format!(
            "número de muros cortina: {} / {}",
            a.curtain_walls().count(),
            b.curtain_walls().count()
        ));
    }
    for s in a.curtain_types() {
        let same = b
            .curtain_types()
            .find(|t| t.guid == s.guid)
            .is_some_and(|t| {
                t.name == s.name
                    && crate::curtain::design_texts(a, &s.design)
                        == crate::curtain::design_texts(b, &t.design)
            });
        if !same {
            diff(format!("tipo de muro cortina «{}» ({})", s.name, s.guid));
        }
    }
    if a.curtain_types().count() != b.curtain_types().count() {
        diff(format!(
            "número de tipos de muro cortina: {} / {}",
            a.curtain_types().count(),
            b.curtain_types().count()
        ));
    }
    for s in a.curtain_walls() {
        let Some(t) = b.curtain_walls().find(|t| t.guid == s.guid) else {
            diff(format!("muro cortina {} falta", s.guid));
            continue;
        };
        let same = s.start.distance(t.start) <= TOL
            && s.end.distance(t.end) <= TOL
            && close(s.base_offset, t.base_offset)
            && close(s.height, t.height)
            && level_guid(a, s.level) == level_guid(b, t.level)
            && match (
                a.curtain_type(s.curtain_type),
                b.curtain_type(t.curtain_type),
            ) {
                (Some(x), Some(y)) => {
                    x.name == y.name
                        && crate::curtain::design_texts(a, &x.design)
                            == crate::curtain::design_texts(b, &y.design)
                }
                _ => false,
            };
        if !same {
            diff(format!("muro cortina {}: {:?} / {:?}", s.guid, s, t));
        }
    }
    if a.stairs().count() != b.stairs().count() {
        diff(format!(
            "número de escaleras: {} / {}",
            a.stairs().count(),
            b.stairs().count()
        ));
    }
    for s in a.stairs() {
        let Some(t) = b.stairs().find(|t| t.guid == s.guid) else {
            diff(format!("escalera {} falta", s.guid));
            continue;
        };
        let same = s.path.len() == t.path.len()
            && s.path
                .iter()
                .zip(&t.path)
                .all(|(p, q)| p.distance(*q) <= TOL)
            && close(s.width, t.width)
            && close(s.going, t.going)
            && close(s.base_offset, t.base_offset)
            && close(s.height, t.height)
            && close(s.waist, t.waist)
            && s.usage == t.usage
            && s.railings == t.railings
            && s.design.form == t.design.form
            && s.design.structure == t.design.structure
            && s.design.closed_risers == t.design.closed_risers
            && (s.design.sweep - t.design.sweep).abs() < 1e-6
            && close(s.design.core, t.design.core)
            && close(s.design.tread, t.design.tread)
            && close(s.design.stringer[0], t.design.stringer[0])
            && close(s.design.stringer[1], t.design.stringer[1])
            && s.design
                .tread_material
                .and_then(|m| a.material(m))
                .map(|m| &m.name)
                == t.design
                    .tread_material
                    .and_then(|m| b.material(m))
                    .map(|m| &m.name)
            && level_guid(a, s.level) == level_guid(b, t.level)
            && match (s.top, t.top) {
                (
                    archisim_core::WallTop::Level {
                        level: x,
                        offset: o,
                    },
                    archisim_core::WallTop::Level {
                        level: y,
                        offset: p,
                    },
                ) => level_guid(a, x) == level_guid(b, y) && close(o, p),
                (archisim_core::WallTop::Unconnected, archisim_core::WallTop::Unconnected) => true,
                _ => false,
            };
        if !same {
            diff(format!("escalera {}: {:?} / {:?}", s.guid, s, t));
        }
    }
    // Walls trimmed under roofs (by GlobalId).
    let trims = |m: &Model| -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = m
            .walls()
            .filter_map(|w| {
                let r = m.wall_roof(w.id).and_then(|r| m.roof(r))?;
                Some((w.guid.to_string(), r.guid.to_string()))
            })
            .collect();
        v.sort();
        v
    };
    if trims(a) != trims(b) {
        diff("muros recortados bajo cubierta distintos".to_string());
    }
    if a.roofs().count() != b.roofs().count() {
        diff(format!(
            "número de cubiertas: {} / {}",
            a.roofs().count(),
            b.roofs().count()
        ));
    }
    for r in a.roofs() {
        let Some(t) = b.roofs().find(|t| t.guid == r.guid) else {
            diff(format!("cubierta {} falta", r.guid));
            continue;
        };
        let same = r.outline.len() == t.outline.len()
            && r.outline
                .iter()
                .zip(&t.outline)
                .all(|(p, q)| p.distance(*q) <= TOL)
            && r.slopes.iter().zip(&t.slopes).all(|(x, y)| match (x, y) {
                (Some(x), Some(y)) => close(*x, *y),
                (None, None) => true,
                _ => false,
            })
            && close(r.base_offset, t.base_offset)
            && close(r.overhang, t.overhang)
            && level_guid(a, r.level) == level_guid(b, t.level)
            && a.slab_type(r.roof_type).map(|x| &x.guid)
                == b.slab_type(t.roof_type).map(|x| &x.guid);
        if !same {
            diff(format!("cubierta {}: {:?} / {:?}", r.guid, r, t));
        }
        // Its dormers and covering (materials by name).
        let mat = |m: &Model, id| m.material(id).map(|x| x.name.clone());
        let same_dormers = r.dormers.len() == t.dormers.len()
            && r.dormers.iter().zip(&t.dormers).all(|(x, y)| {
                x.guid == y.guid
                    && x.kind == y.kind
                    && x.side == y.side
                    && close(x.along, y.along)
                    && close(x.up, y.up)
                    && close(x.width, y.width)
                    && close(x.height, y.height)
                    && close(x.pitch, y.pitch)
                    && close(x.window_width, y.window_width)
                    && close(x.window_height, y.window_height)
                    && mat(a, x.wall_material) == mat(b, y.wall_material)
                    && mat(a, x.frame_material) == mat(b, y.frame_material)
                    && mat(a, x.glass_material) == mat(b, y.glass_material)
            });
        let (cr, ct) = (&r.covering, &t.covering);
        let same_covering = cr.tiles == ct.tiles
            && cr.tile_material.and_then(|m| mat(a, m)) == ct.tile_material.and_then(|m| mat(b, m))
            && match (&cr.gutter, &ct.gutter) {
                (None, None) => true,
                (Some(x), Some(y)) => {
                    x.shape == y.shape
                        && close(x.size, y.size)
                        && close(x.downpipe, y.downpipe)
                        && close(x.spacing, y.spacing)
                        && mat(a, x.material) == mat(b, y.material)
                }
                _ => false,
            };
        if !same_dormers || !same_covering {
            diff(format!(
                "buhardillas o cubrición de la cubierta {} distintas",
                r.guid
            ));
        }
    }
    for s in a.rooms() {
        let Some(t) = b.rooms().find(|t| t.guid == s.guid) else {
            diff(format!("local {} falta", s.guid));
            continue;
        };
        let same_ring = |p: &[Point2], q: &[Point2]| {
            p.len() == q.len() && p.iter().zip(q).all(|(p, q)| p.distance(*q) <= TOL)
        };
        let same_height = match (s.height, t.height) {
            (None, None) => true,
            (Some(p), Some(q)) => close(p, q),
            _ => false,
        };
        if !same_ring(&s.outline, &t.outline)
            || s.holes.len() != t.holes.len()
            || !s.holes.iter().zip(&t.holes).all(|(p, q)| same_ring(p, q))
            || s.number != t.number
            || s.name != t.name
            || s.usage != t.usage
            || s.finishes != t.finishes
            || s.bounded != t.bounded
            || !same_height
            || s.label.distance(t.label) > TOL
            || level_guid(a, s.level) != level_guid(b, t.level)
        {
            diff(format!("local {}: {:?} / {:?}", s.guid, s, t));
        }
    }

    // Columns and beams, with their types (section, material by name).
    let member_type = |m: &Model, id| {
        m.member_type(id).map(|t| {
            (
                t.guid.clone(),
                t.name.clone(),
                t.usage,
                format!("{:?}", t.profile),
                m.material(t.material).map(|m| m.name.clone()),
            )
        })
    };
    if a.columns().count() != b.columns().count() || a.beams().count() != b.beams().count() {
        diff(format!(
            "número de pilares y vigas: {} {} / {} {}",
            a.columns().count(),
            a.beams().count(),
            b.columns().count(),
            b.beams().count()
        ));
    }
    for c in a.columns() {
        let Some(d) = b.columns().find(|d| d.guid == c.guid) else {
            diff(format!("pilar {} falta", c.guid));
            continue;
        };
        let turn = (c.rotation - d.rotation).rem_euclid(std::f64::consts::TAU);
        let same_turn = turn < 1e-6 || std::f64::consts::TAU - turn < 1e-6;
        if c.position.distance(d.position) > TOL
            || !same_turn
            || !close(c.base_offset, d.base_offset)
            || !close(c.height, d.height)
            || level_guid(a, c.level) != level_guid(b, d.level)
        {
            diff(format!("pilar {}: {:?} / {:?}", c.guid, c, d));
        }
        if member_type(a, c.member_type) != member_type(b, d.member_type) {
            diff(format!(
                "tipo del pilar {}: {:?} / {:?}",
                c.guid,
                member_type(a, c.member_type),
                member_type(b, d.member_type)
            ));
        }
    }
    for x in a.beams() {
        let Some(y) = b.beams().find(|y| y.guid == x.guid) else {
            diff(format!("viga {} falta", x.guid));
            continue;
        };
        if x.start.distance(y.start) > TOL
            || x.end.distance(y.end) > TOL
            || !close(x.top_offset, y.top_offset)
            || level_guid(a, x.level) != level_guid(b, y.level)
        {
            diff(format!("viga {}: {:?} / {:?}", x.guid, x, y));
        }
        if member_type(a, x.member_type) != member_type(b, y.member_type) {
            diff(format!(
                "tipo de la viga {}: {:?} / {:?}",
                x.guid,
                member_type(a, x.member_type),
                member_type(b, y.member_type)
            ));
        }
    }
    // Parameters of the project, by name, and their values by element.
    let defs = |m: &Model| -> Vec<String> {
        let mut v: Vec<String> = m
            .shared_parameters()
            .iter()
            .map(|d| archisim_core::shared_parameters_text(std::slice::from_ref(d)))
            .collect();
        v.sort();
        v
    };
    if defs(a) != defs(b) {
        diff(format!(
            "parámetros del proyecto: {:?} / {:?}",
            defs(a),
            defs(b)
        ));
    }
    let values = |m: &Model| -> Vec<String> {
        let mut v: Vec<String> = m
            .walls()
            .map(|x| x.id.0)
            .chain(m.openings().map(|x| x.id.0))
            .chain(m.slabs().map(|x| x.id.0))
            .chain(m.rooms().map(|x| x.id.0))
            .chain(m.objects().map(|x| x.id.0))
            .flat_map(|id| {
                let g = m
                    .element_guid(id)
                    .map(|g| g.to_string())
                    .unwrap_or_default();
                m.element_parameters(id)
                    .into_iter()
                    .map(move |(p, v)| format!("{g} {} = {v:?}", p.name))
            })
            .collect();
        v.sort();
        v
    };
    let (va, vb) = (values(a), values(b));
    if va != vb {
        let only: Vec<&String> = va.iter().filter(|x| !vb.contains(x)).take(5).collect();
        diff(format!("valores de parámetros distintos, p. ej. {only:?}"));
    }
    out
}

fn material_name(m: &Model, id: archisim_core::MaterialId) -> Option<&str> {
    m.material(id).map(|x| x.name.as_str())
}

/// A pipe or duct compared: GUID, system, path and sizes in millimetres.
type RunKey = (String, String, Vec<[i64; 2]>, [i64; 4]);
