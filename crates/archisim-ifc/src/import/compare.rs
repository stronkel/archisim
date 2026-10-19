//! Compares two models element by element, matching them by `GlobalId`:
//! what a round trip through IFC must keep.

use archisim_core::{Layer, MaterialId, Model};

/// Lengths closer than this are equal (0.001 mm).
const TOL: f64 = 1e-6;

fn layers_text(model: &Model, layers: &[Layer]) -> String {
    layers
        .iter()
        .map(|l| {
            let m = model.material(l.material);
            format!(
                "{} {:.6} {:?} [{}/{}/{:?}]",
                m.map_or("?", |m| m.name.as_str()),
                l.thickness,
                l.function,
                m.map_or("", |m| m.category.as_str()),
                m.map_or(String::new(), |m| format!("{:?}", m.cut_color)),
                m.map(|m| m.cut_pattern),
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
    if a.spaces().count() != b.spaces().count() {
        diff(format!(
            "número de locales: {} / {}",
            a.spaces().count(),
            b.spaces().count()
        ));
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
    }
    for s in a.spaces() {
        let Some(t) = b.spaces().find(|t| t.guid == s.guid) else {
            diff(format!("local {} falta", s.guid));
            continue;
        };
        let same_outline = s.outline.len() == t.outline.len()
            && s.outline
                .iter()
                .zip(&t.outline)
                .all(|(p, q)| p.distance(*q) <= TOL);
        let same_label = match (s.label, t.label) {
            (None, None) => true,
            (Some(p), Some(q)) => p.distance(q) <= TOL,
            _ => false,
        };
        if !same_outline
            || s.number != t.number
            || s.name != t.name
            || s.usage != t.usage
            || !close(s.height, t.height)
            || !same_label
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
    out
}
