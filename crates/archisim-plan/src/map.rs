//! The map view (N8, `docs/TERRITORIO.md` §2.11), as CSL Map View draws
//! a city in Cities: Skylines: the whole area of the surroundings at a
//! glance, flat and in colour — land, gardens, water, streets by their
//! width with their names, buildings, trees, the plots' zones, the
//! project's own building and the connections with the outside. It is a
//! drawing to find one's way round a large area, not a plan to print: no
//! pens, no hatches, no symbols.

use std::fmt::Write as _;

use archisim_core::{BlockZone, LevelId, Model, Point2, SurfaceKind};

use crate::lettering::{text_path, Align};
use crate::{num, path, pt};

/// The map's colours.
const LAND: &str = "#f2efe9";
const OUTSIDE: &str = "#dedad2";
const GARDEN: &str = "#cfe3b4";
const PAVED: &str = "#e8e0d0";
const WATER: &str = "#a9d3ea";
const STREET: &str = "#ffffff";
const AVENUE: &str = "#f8d98c";
const STREET_EDGE: &str = "#bdb4a4";
const BUILDING: &str = "#d8cfc2";
const BUILDING_EDGE: &str = "#b0a492";
const FACILITY: &str = "#e2c4d6";
const OWN: &str = "#c8694c";
const TREE: &str = "#8fbf6a";
const TEXT: &str = "#4a4238";
const LINK: &str = "#9a6a2e";

/// A street at least this wide is drawn as an avenue, metres.
const AVENUE_WIDTH: f64 = 20.0;

/// What the map covers: the area of the surroundings, or everything on the
/// level.
pub fn map_frame(model: &Model, level: LevelId) -> Option<(Point2, Point2)> {
    let mut lo = Point2::new(f64::MAX, f64::MAX);
    let mut hi = Point2::new(f64::MIN, f64::MIN);
    let mut add = |p: Point2| {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    };
    if let Some(area) = model.surroundings() {
        area.iter().copied().for_each(&mut add);
    } else {
        for r in model.roads_on_level(level) {
            r.outline().into_iter().for_each(&mut add);
        }
        for m in model.masses_on_level(level) {
            m.outline().into_iter().for_each(&mut add);
        }
        for p in model.plots_on_level(level) {
            p.outline.iter().copied().for_each(&mut add);
        }
        for s in model.slabs().filter(|s| s.level == level) {
            s.outline.iter().copied().for_each(&mut add);
        }
    }
    (hi.x > lo.x && hi.y > lo.y).then_some((lo, hi))
}

/// The map of a level as an SVG of `width_px` pixels across (its text and
/// lines are sized for it). None if there is nothing to map.
pub fn render_map(model: &Model, level: LevelId, width_px: f64) -> Option<String> {
    let (lo, hi) = map_frame(model, level)?;
    let margin = ((hi.x - lo.x).max(hi.y - lo.y) * 0.03).max(5.0);
    let (lo, hi) = (
        Point2::new(lo.x - margin, lo.y - margin),
        Point2::new(hi.x + margin, hi.y + margin),
    );
    let (w, h) = (hi.x - lo.x, hi.y - lo.y);
    // Metres per pixel.
    let px = w / width_px.max(100.0);
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}px" height="{}px" viewBox="{} {} {} {}">"#,
        num(width_px.round()),
        num((width_px * h / w).round()),
        num(lo.x),
        num(-hi.y),
        num(w),
        num(h)
    );
    // The land: inside the area, the rest dimmed.
    let _ = writeln!(
        s,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
        num(lo.x),
        num(-hi.y),
        num(w),
        num(h),
        if model.surroundings().is_some() {
            OUTSIDE
        } else {
            LAND
        }
    );
    if let Some(area) = model.surroundings() {
        let _ = writeln!(s, r#"<path d="{}" fill="{LAND}"/>"#, path(area));
    }
    // Plots: their zone's colour, faint.
    for p in model.plots_on_level(level) {
        if let Some(z) = p.zone {
            let [r, g, b] = z.rgb();
            let _ = writeln!(
                s,
                r#"<path d="{}" fill="rgb({r},{g},{b})" fill-opacity="0.18"/>"#,
                path(&p.outline)
            );
        }
    }
    // Surfaces: gardens green, the rest paved.
    for x in model.surfaces_on_level(level) {
        let fill = match x.kind {
            SurfaceKind::Garden => GARDEN,
            _ => PAVED,
        };
        let _ = writeln!(s, r#"<path d="{}" fill="{fill}"/>"#, path(&x.outline));
    }
    for p in model.pools_on_level(level) {
        let _ = writeln!(s, r#"<path d="{}" fill="{WATER}"/>"#, path(&p.outline));
    }
    // Streets: their edges first, under every street, then their fills,
    // so that crossings join without lines.
    let junctions = archisim_geom::crossroads::junctions(model, level);
    let mut streets: Vec<(Vec<Point2>, bool)> = Vec::new();
    for r in model.roads_on_level(level) {
        let o = r.outline();
        if o.len() >= 3 {
            streets.push((o, r.section.width() >= AVENUE_WIDTH));
        }
    }
    for c in model.roundabouts_on_level(level) {
        streets.push((c.outline(), false));
    }
    for j in &junctions {
        // An avenue's crossings in the avenue's colour.
        let wide = j.arms.iter().any(|a| {
            model
                .road(a.road)
                .is_some_and(|r| r.section.width() >= AVENUE_WIDTH)
        });
        for p in std::iter::once(&j.plate)
            .chain(&j.sidewalks)
            .chain(&j.kerbs)
        {
            if p.len() >= 3 {
                streets.push((p.clone(), wide));
            }
        }
    }
    let edge = 2.0 * px;
    s.push_str("<g>\n");
    for (o, _) in &streets {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{STREET_EDGE}" stroke="{STREET_EDGE}" stroke-width="{}" stroke-linejoin="round"/>"#,
            path(o),
            num(edge)
        );
    }
    for (o, wide) in &streets {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{}"/>"#,
            path(o),
            if *wide { AVENUE } else { STREET }
        );
    }
    s.push_str("</g>\n");
    // Buildings: the surroundings', the facilities', and the project's own
    // (its slabs and walls).
    for m in model.masses_on_level(level) {
        let o = m.outline();
        if o.len() < 3 {
            continue;
        }
        let facility = m
            .dress
            .as_ref()
            .and_then(|d| d.world.as_ref())
            .is_some_and(|w| w.typology == archisim_core::WorldTypology::Facility);
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="{}" stroke="{BUILDING_EDGE}" stroke-width="{}"/>"#,
            path(&o),
            if facility { FACILITY } else { BUILDING },
            num(px)
        );
    }
    for x in model.slabs().filter(|x| x.level == level) {
        let _ = writeln!(s, r#"<path d="{}" fill="{OWN}"/>"#, path(&x.outline));
    }
    for wall in model.walls().filter(|x| x.level == level) {
        let _ = writeln!(
            s,
            r#"<path d="M{} L{}" stroke="{OWN}" stroke-width="{}" stroke-linecap="round"/>"#,
            pt(wall.start),
            pt(wall.end),
            num((3.0 * px).max(0.3))
        );
    }
    // Trees: a green dot each.
    let r_tree = (2.5f64).max(2.0 * px);
    for o in model.objects_on_level(level) {
        if crate::object_category(model, o) != archisim_core::Category::Vegetation {
            continue;
        }
        let _ = writeln!(
            s,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{TREE}"/>"#,
            num(o.position.x),
            num(-o.position.y),
            num(r_tree)
        );
    }
    // The connections with the outside: an arrow out of the area.
    for e in model.outside_ends(level) {
        let side = e.out.perp_left();
        let a = 9.0 * px;
        let tip = e.point + e.out * (a * 1.6);
        let tri = [e.point + side * a, tip, e.point - side * a];
        let _ = writeln!(
            s,
            r#"<path d="M{} L{} L{} Z" fill="{LINK}"/>"#,
            pt(tri[0]),
            pt(tri[1]),
            pt(tri[2])
        );
    }
    if let Some(area) = model.surroundings() {
        let _ = writeln!(
            s,
            r#"<path d="{}" fill="none" stroke="{LINK}" stroke-width="{}" stroke-dasharray="{} {}"/>"#,
            path(area),
            num(1.5 * px),
            num(12.0 * px),
            num(6.0 * px)
        );
    }
    // Street names: once per name, on the longest piece, along it.
    let text_h = 11.0 * px;
    let mut named: Vec<(&str, f64, Point2, archisim_core::Vec2)> = Vec::new();
    for r in model.roads_on_level(level) {
        if r.name.trim().is_empty() || r.axis.len() < 2 {
            continue;
        }
        let len = r.length();
        let wide = r.name.chars().count() as f64 * text_h * 0.62;
        if len < wide * 1.3 {
            continue;
        }
        // Near the middle, but between crossings: the name clear of the
        // streets that cross it.
        let clear = |s: f64| {
            let (p, _) = r.axis_at(s);
            junctions
                .iter()
                .all(|j| j.at.distance(p) > wide / 2.0 + 12.0 * px)
        };
        let Some(at) = (0..40)
            .map(|k| {
                let d = (k / 2) as f64 * len / 40.0;
                len / 2.0 + if k % 2 == 0 { d } else { -d }
            })
            .find(|s| *s > wide / 2.0 && *s < len - wide / 2.0 && clear(*s))
        else {
            continue;
        };
        let (mid, dir) = r.axis_at(at);
        match named.iter_mut().find(|n| n.0 == r.name.as_str()) {
            Some(n) if n.1 >= len => {}
            Some(n) => *n = (r.name.as_str(), len, mid, dir),
            None => named.push((r.name.as_str(), len, mid, dir)),
        }
    }
    for (name, _, mid, dir) in named {
        let dir = if dir.x < -1e-9 { dir * -1.0 } else { dir };
        let at = mid - dir.perp_left() * (text_h / 2.0);
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(name, at, dir, text_h, Align::Centre),
            crate::lettering::paint(text_h, TEXT)
        );
    }
    // A scale bar, bottom left: a round length of about a fifth across.
    let want = w / 5.0;
    let step = [1.0, 2.0, 5.0]
        .iter()
        .flat_map(|m| (0..5).map(move |e| m * 10f64.powi(e)))
        .filter(|x| *x <= want)
        .fold(10.0, f64::max);
    let base = Point2::new(lo.x + margin * 0.6, lo.y + margin * 0.6);
    let bar_h = 4.0 * px;
    let _ = writeln!(
        s,
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{TEXT}"/>"#,
        num(base.x),
        num(-(base.y + bar_h)),
        num(step),
        num(bar_h)
    );
    let label = if step >= 1000.0 {
        format!("{} km", step / 1000.0)
    } else {
        format!("{step} m")
    };
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(
            &label,
            base + archisim_core::Vec2::new(0.0, bar_h + 3.0 * px),
            archisim_core::Vec2::new(1.0, 0.0),
            text_h,
            Align::Left
        ),
        crate::lettering::paint(text_h, TEXT)
    );
    legend(&mut s, model, level, hi, margin, px, text_h);
    s.push_str("</svg>\n");
    Some(s)
}

/// The legend, top right: what the colours of the map are (the zones only
/// those the plots of the level have).
fn legend(
    s: &mut String,
    model: &Model,
    level: LevelId,
    hi: Point2,
    margin: f64,
    px: f64,
    text_h: f64,
) {
    let mut rows: Vec<(String, String)> = vec![
        ("Calle".into(), STREET.into()),
        ("Avenida".into(), AVENUE.into()),
        ("Edificio".into(), BUILDING.into()),
        ("Edificio público".into(), FACILITY.into()),
        ("Proyecto".into(), OWN.into()),
    ];
    if model
        .surfaces_on_level(level)
        .any(|x| x.kind == SurfaceKind::Garden)
    {
        rows.push(("Jardín".into(), GARDEN.into()));
    }
    for z in BlockZone::ALL {
        if model.plots_on_level(level).any(|p| p.zone == Some(z)) {
            let [r, g, b] = z.rgb();
            // As faint as on the map, over the land.
            let mix = |c: u8, l: u8| (0.18 * c as f64 + 0.82 * l as f64).round() as u8;
            rows.push((
                format!("Zona: {}", z.label().to_lowercase()),
                format!("rgb({},{},{})", mix(r, 0xf2), mix(g, 0xef), mix(b, 0xe9)),
            ));
        }
    }
    let line = text_h * 1.8;
    let swatch = text_h * 1.2;
    let widest = rows
        .iter()
        .map(|(t, _)| t.chars().count())
        .max()
        .unwrap_or(4) as f64;
    let width = swatch + text_h * (1.0 + widest * 0.75) + 2.0 * text_h;
    let height = line * rows.len() as f64 + text_h;
    let corner = Point2::new(hi.x - margin * 0.6 - width, hi.y - margin * 0.6);
    let _ = writeln!(
        s,
        r#"<g class="map-legend"><rect x="{}" y="{}" width="{}" height="{}" fill="white" fill-opacity="0.85" stroke="{STREET_EDGE}" stroke-width="{}"/>"#,
        num(corner.x),
        num(-corner.y),
        num(width),
        num(height),
        num(px)
    );
    for (k, (text, fill)) in rows.iter().enumerate() {
        let y = corner.y - text_h - k as f64 * line;
        let _ = writeln!(
            s,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{fill}" stroke="{STREET_EDGE}" stroke-width="{}"/>"#,
            num(corner.x + text_h),
            num(-y),
            num(swatch),
            num(swatch),
            num(px)
        );
        let _ = writeln!(
            s,
            r#"<path d="{}" {}/>"#,
            text_path(
                text,
                Point2::new(corner.x + 2.0 * text_h + swatch, y - swatch * 0.85),
                archisim_core::Vec2::new(1.0, 0.0),
                text_h,
                Align::Left
            ),
            crate::lettering::paint(text_h, TEXT)
        );
    }
    s.push_str("</g>\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewRoad, RoadSection};

    #[test]
    fn a_map_of_streets_and_their_connections() {
        let mut m = Model::new("Mapa");
        let level = m.add_level("Planta baja", 0.0);
        assert!(render_map(&m, level, 800.0).is_none(), "nothing to map");
        let mat = m.add_material(
            "Asfalto",
            "asphalt",
            [60, 60, 60],
            archisim_core::CutPattern::Solid,
        );
        for (name, a, b, section) in [
            (
                "Avenida",
                Point2::new(-200.0, 0.0),
                Point2::new(200.0, 0.0),
                RoadSection::classic(14.0, 2.2, 0.15, 5.0),
            ),
            (
                "Calle Mayor",
                Point2::new(0.0, -200.0),
                Point2::new(0.0, 100.0),
                RoadSection::default(),
            ),
        ] {
            m.add_road(NewRoad {
                name: name.into(),
                level,
                axis: vec![a, b],
                section,
                top_offset: 0.0,
                profile: Vec::new(),
                changes: Vec::new(),
                pavement: mat,
                kerb_material: mat,
                sidewalk_material: mat,
            })
            .unwrap();
        }
        m.set_surroundings(Some(archisim_core::square_area(
            Point2::new(0.0, 0.0),
            200.0,
        )))
        .unwrap();
        let svg = render_map(&m, level, 800.0).unwrap();
        // The avenue in its colour, three connections (both ends of the
        // avenue, the south end of the street).
        assert!(svg.contains(AVENUE));
        assert_eq!(svg.matches(&format!("fill=\"{LINK}\"")).count(), 3);
        let (lo, hi) = map_frame(&m, level).unwrap();
        assert!((hi.x - lo.x - 400.0).abs() < 1e-6);
    }
}
