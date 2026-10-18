//! Compares two models element by element, matching them by `GlobalId`:
//! what a round trip through IFC must keep.

use archisim_core::{Layer, Model};

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
        let same_outline = s.outline.len() == t.outline.len()
            && s.outline
                .iter()
                .zip(&t.outline)
                .all(|(p, q)| p.distance(*q) <= TOL);
        if !same_outline
            || !close(s.top_offset, t.top_offset)
            || level_guid(a, s.level) != level_guid(b, t.level)
        {
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
                {
                    diff(format!("tipo de {}: {:?} / {:?}", o.guid, x, y));
                }
            }
            _ => diff(format!("tipo de {} falta", o.guid)),
        }
    }
    out
}
