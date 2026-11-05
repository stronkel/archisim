//! Predimensioning of the light frames (steel frame and timber frame): the
//! studs, headers, joists and rafters of every framed wall, floor and roof
//! against the loads that reach them, by hand-calculation rules. It is a
//! first check while drawing, to know whether the sections are in the
//! right range; it does not replace the structural calculation of the
//! project (connections, bracing of the panels by their boards, local
//! loads, fire).
//!
//! What it takes (to be confirmed for each project):
//!
//! - actions (CTE DB-SE-AE): floors 1,5 kN/m² dead (frame, boards,
//!   finishes and light partitions) and 2,0 kN/m² live (dwellings, A1);
//!   roofs 1,0 kN/m² dead and 0,4 kN/m² of snow; walls 0,5 kN/m² of their
//!   face; wind 0,8 kN/m² on exterior walls;
//! - combinations (DB-SE): 1,35 G + 1,5 Q (+ 0,9 W), and 1,35 G + 1,05 Q
//!   + 1,5 W for the exterior walls; deflection under G + Q, L/300;
//! - steel: S350GD (fy 350 MPa, E 210 GPa), γM 1,0; the section reduced
//!   to its effective widths (EN 1993-1-5, 4.4) and buckling curve b
//!   (EN 1993-1-3, 6.2), about the strong axis only (the boards and the
//!   noggins hold the studs the other way);
//! - timber: C24 (fm 24, fc,0 21, E0,05 7 400, E0,mean 11 000 MPa),
//!   kmod 0,8, γM 1,3; buckling by EN 1995-1-1, 6.3.2;
//! - a wall carries the floors and roofs over it (half the span of the
//!   joists or rafters that bear on it, or 30 cm of what runs along it)
//!   and the walls standing on it.

use archisim_core::{FrameMaterial, FrameSection, Model, Point2, Vec2, WallFunction};
use archisim_geom::framing::{roof_members, slab_members, wall_members, Member, Role};

use crate::{Item, Report, Section, Status, Subject};

pub const G_FLOOR: f64 = 1.5;
pub const Q_FLOOR: f64 = 2.0;
pub const G_ROOF: f64 = 1.0;
pub const SNOW: f64 = 0.4;
pub const G_WALL: f64 = 0.5;
pub const WIND: f64 = 0.8;
/// Steel S350GD, Pa.
const FY: f64 = 350e6;
const E_STEEL: f64 = 210e9;
/// Timber C24, Pa.
const FM_K: f64 = 24e6;
const FC_K: f64 = 21e6;
const E_005: f64 = 7.4e9;
const E_MEAN: f64 = 11e9;
const KMOD: f64 = 0.8;
const GAMMA_T: f64 = 1.3;
/// Deflection limit: span / LIMIT.
const LIMIT: f64 = 300.0;

/// What a section resists, SI units: axial (N) as a column `length` long
/// (buckling about the strong axis), bending (N·m), and its second moment
/// (m⁴) for deflections.
#[derive(Debug, Clone, Copy)]
pub struct Resistance {
    pub n_rd: f64,
    pub m_rd: f64,
    pub ei: f64,
}

/// The effective width factor of a plate `b` wide and `t` thick in
/// compression (EN 1993-1-5, 4.4): internal (k 4) or outstand (k 0,43).
fn rho(b: f64, t: f64, internal: bool) -> f64 {
    let eps = (235e6 / FY).sqrt();
    let k: f64 = if internal { 4.0 } else { 0.43 };
    let lp = (b / t) / (28.4 * eps * k.sqrt());
    if internal {
        if lp <= 0.673 {
            1.0
        } else {
            ((lp - 0.22) / (lp * lp)).min(1.0)
        }
    } else if lp <= 0.748 {
        1.0
    } else {
        ((lp - 0.188) / (lp * lp)).min(1.0)
    }
}

/// The reduction for buckling (EN 1993-1-1, 6.3.1.2) with imperfection
/// `alpha`.
fn chi(lambda: f64, alpha: f64) -> f64 {
    let phi = 0.5 * (1.0 + alpha * (lambda - 0.2) + lambda * lambda);
    (1.0 / (phi + (phi * phi - lambda * lambda).max(0.0).sqrt())).min(1.0)
}

/// What a section resists as a member `length` long.
pub fn resistance(s: &FrameSection, length: f64) -> Resistance {
    if s.is_steel() {
        let (h, b, c, t) = (s.depth, s.width, s.lip, s.thickness.max(1e-4));
        // Gross second moment about the strong axis (centre lines).
        let lip_i = if c > 0.0 {
            2.0 * (t * c.powi(3) / 12.0 + c * t * (h / 2.0 - c / 2.0).powi(2))
        } else {
            0.0
        };
        let i = t * h.powi(3) / 12.0 + 2.0 * b * t * (h / 2.0).powi(2) + lip_i;
        // Effective widths in compression.
        let rw = rho(h, t, true);
        let rf = rho(b, t, c > 0.0);
        let rl = if c > 0.0 { rho(c, t, false) } else { 0.0 };
        let a_eff = t * (rw * h + 2.0 * rf * b + 2.0 * rl * c);
        // In bending only the compressed flange (and lip) is reduced.
        let flange_loss = (1.0 - rf) * b * t * (h / 2.0).powi(2)
            + (1.0 - rl) * c * t * (h / 2.0 - c / 2.0).powi(2);
        let i_eff = (i - flange_loss).max(i * 0.3);
        let w_eff = i_eff / (h / 2.0);
        let n_cr = std::f64::consts::PI.powi(2) * E_STEEL * i / (length * length).max(1e-6);
        let lambda = (a_eff * FY / n_cr).sqrt();
        Resistance {
            n_rd: chi(lambda, 0.34) * a_eff * FY,
            m_rd: w_eff * FY,
            ei: E_STEEL * i_eff,
        }
    } else {
        let (h, b) = (s.depth, s.width);
        let a = b * h;
        let i = b * h.powi(3) / 12.0;
        let w = b * h * h / 6.0;
        let r = (i / a).sqrt();
        let lambda = (length / r) / std::f64::consts::PI * (FC_K / E_005).sqrt();
        let k = 0.5 * (1.0 + 0.2 * (lambda - 0.3) + lambda * lambda);
        let kc = (1.0 / (k + (k * k - lambda * lambda).max(0.0).sqrt())).min(1.0);
        Resistance {
            n_rd: kc * a * KMOD * FC_K / GAMMA_T,
            m_rd: w * KMOD * FM_K / GAMMA_T,
            ei: E_MEAN * i,
        }
    }
}

/// The loads that reach a wall from above, per metre of wall, kN/m:
/// dead, live and snow.
pub fn wall_loads(model: &Model, id: archisim_core::WallId) -> (f64, f64, f64) {
    let Some(w) = model.wall(id) else {
        return (0.0, 0.0, 0.0);
    };
    let level_z = model.level(w.level).map_or(0.0, |l| l.elevation);
    let top = level_z + w.base_offset + w.height;
    let mid = w.midpoint();
    let along = (w.end - w.start)
        .normalized()
        .unwrap_or(Vec2::new(1.0, 0.0));
    let (mut g, mut q, mut s) = (0.0, 0.0, 0.0);
    // Half the extent of a polygon along `dir` if the wall bears what runs
    // that way, or 30 cm of what runs along it.
    let share = |poly: &[Point2], dir: Vec2| -> f64 {
        if along.dot(dir).abs() > 0.5 {
            return 0.3;
        }
        let (lo, hi) = poly
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.dot(dir)), hi.max(p.dot(dir)))
            });
        ((hi - lo) / 2.0).max(0.3)
    };
    let covers = |poly: &[Point2]| near_polygon(poly, mid, 0.5);
    for sl in model.slabs() {
        let z = model.level(sl.level).map_or(0.0, |l| l.elevation) + sl.top_offset;
        if z < top - 0.5 || !covers(&sl.outline) {
            continue;
        }
        let n = sl.outline.len();
        let longest = (0..n)
            .map(|i| sl.outline[(i + 1) % n] - sl.outline[i])
            .max_by(|a, b| a.length().total_cmp(&b.length()));
        let Some(dir) = longest.and_then(|e| e.normalized()).map(|e| e.perp_left()) else {
            continue;
        };
        let t = share(&sl.outline, dir);
        g += G_FLOOR * t;
        q += Q_FLOOR * t;
    }
    for r in model.roofs() {
        let z = model.level(r.level).map_or(0.0, |l| l.elevation) + r.base_offset;
        if z < top - 0.5 || !covers(&r.eaves()) {
            continue;
        }
        let planes = r.planes();
        let Some(plane) = planes
            .iter()
            .find(|p| near_polygon(&p.outline, mid, 0.5))
            .or(planes.first())
        else {
            g += G_ROOF * 0.3;
            continue;
        };
        let t = plane
            .slope
            .direction()
            .map_or(0.3, |d| share(&plane.outline, d));
        g += G_ROOF * t;
        s += SNOW * t;
    }
    // The walls standing on it.
    for o in model.walls() {
        if o.id == id {
            continue;
        }
        let oz = model.level(o.level).map_or(0.0, |l| l.elevation) + o.base_offset;
        if oz < top - 0.3 {
            continue;
        }
        let d = distance_to_segment(mid, o.start, o.end);
        if d < 0.3 {
            g += G_WALL * o.height;
        }
    }
    (g, q, s)
}

fn distance_to_segment(p: Point2, a: Point2, b: Point2) -> f64 {
    let d = b - a;
    let l2 = d.dot(d);
    if l2 < 1e-12 {
        return p.distance(a);
    }
    let t = ((p - a).dot(d) / l2).clamp(0.0, 1.0);
    p.distance(a + d * t)
}

fn near_polygon(poly: &[Point2], p: Point2, tol: f64) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (b.x - a.x) * (p.y - a.y) / (b.y - a.y) {
            inside = !inside;
        }
    }
    inside || (0..n).any(|i| distance_to_segment(p, poly[i], poly[(i + 1) % n]) <= tol)
}

fn dec(v: f64, places: usize) -> String {
    format!("{v:.places$}").replace('.', ",")
}

/// The usual sections of a framing's material, lightest first, to suggest
/// one that would do.
fn lighter_first(material: FrameMaterial, like: &FrameSection) -> Vec<FrameSection> {
    let mut list = match material {
        FrameMaterial::Steel => archisim_core::steel_studs(),
        FrameMaterial::Timber => archisim_core::timber_sections(),
    };
    list.retain(|s| s.depth <= like.depth.max(0.3) + 0.12);
    list.sort_by(|a, b| a.kg_per_m().total_cmp(&b.kg_per_m()));
    list
}

/// One result: the worst use of the members of an element, and what they
/// are.
struct Worst {
    ratio: f64,
    deflection: Option<(f64, f64)>,
    what: String,
    suggestion: Option<String>,
}

fn status_of(ratio: f64) -> Status {
    if ratio > 1.0 {
        Status::Fails
    } else if ratio > 0.9 {
        Status::Warning
    } else {
        Status::Passes
    }
}

fn push(
    report: &mut Report,
    subject: Subject,
    level: Option<archisim_core::LevelId>,
    rule: &'static str,
    name: &str,
    w: Worst,
) {
    let mut ratio = w.ratio;
    let mut text = format!("{name}: {}", w.what);
    if let Some((d, lim)) = w.deflection {
        ratio = ratio.max(d / lim);
        text.push_str(&format!(
            "; flecha {} mm (límite L/300: {} mm)",
            dec(d * 1000.0, 1),
            dec(lim * 1000.0, 1)
        ));
    }
    if let Some(s) = w.suggestion.filter(|_| ratio > 1.0) {
        text.push_str(&format!(". Cumpliría con {s}"));
    }
    report.items.push(Item {
        section: Section::Framing,
        status: status_of(ratio),
        subject,
        level,
        rule,
        text,
        value: format!("aprovechamiento {}", dec(ratio, 2)),
        required: "≤ 1,00".into(),
        route: Vec::new(),
    });
}

/// The studs of a wall: the worst one, with what it carries.
fn studs(model: &Model, id: archisim_core::WallId, members: &[Member]) -> Option<Worst> {
    let w = model.wall(id)?;
    let f = model.type_framing(w.wall_type.0)?;
    let exterior = model
        .wall_type(w.wall_type)
        .is_some_and(|t| t.function == WallFunction::Exterior);
    let (g, q, s) = wall_loads(model, id);
    let wind = if exterior { WIND } else { 0.0 };
    let height = members
        .iter()
        .filter(|m| m.role == Role::Stud)
        .map(|m| m.length())
        .fold(0.0, f64::max);
    if height <= 0.0 {
        return None;
    }
    let use_of = |sec: &FrameSection| {
        let r = resistance(sec, height);
        let sp = f.spacing;
        // kN → N.
        let n1 = (1.35 * (g + G_WALL * height * 0.5) + 1.5 * (q + s)) * sp * 1e3;
        let n2 = (1.35 * (g + G_WALL * height * 0.5) + 1.05 * (q + s)) * sp * 1e3;
        let m = |k: f64| k * wind * sp * height * height / 8.0 * 1e3;
        let a = n1 / r.n_rd + m(0.9) / r.m_rd;
        let b = n2 / r.n_rd + m(1.5) / r.m_rd;
        (a.max(b), n1.max(n2) / 1e3)
    };
    let (ratio, n) = use_of(&f.stud);
    let suggestion = (ratio > 1.0)
        .then(|| {
            lighter_first(f.material, &f.stud)
                .into_iter()
                .find(|s| use_of(s).0 <= 1.0)
                .map(|s| s.name)
        })
        .flatten();
    Some(Worst {
        ratio,
        deflection: None,
        what: format!(
            "montantes {} a {} mm, {} m de altura, {} kN cada uno{}",
            f.stud.name,
            (f.spacing * 1000.0).round(),
            dec(height, 2),
            dec(n, 1),
            if exterior { " y viento" } else { "" }
        ),
        suggestion,
    })
}

/// The headers of a wall: the worst one.
fn headers(model: &Model, id: archisim_core::WallId, members: &[Member]) -> Option<Worst> {
    let w = model.wall(id)?;
    let f = model.type_framing(w.wall_type.0)?;
    let (g, q, s) = wall_loads(model, id);
    let header = members
        .iter()
        .filter(|m| m.role == Role::Header)
        .max_by(|a, b| a.length().total_cmp(&b.length()))?;
    let l = header.length();
    let sec = &header.section;
    // Two members make the box.
    let use_of = |sec: &FrameSection| {
        let r = resistance(sec, l);
        let line = 1.35 * (g + G_WALL) + 1.5 * (q + s);
        let m_ed = line * l * l / 8.0 * 1e3;
        let d = 5.0 * (g + q + s + G_WALL) * 1e3 * l.powi(4) / (384.0 * 2.0 * r.ei);
        (m_ed / (2.0 * r.m_rd), d)
    };
    let (ratio, d) = use_of(sec);
    let fails = ratio.max(d / (l / LIMIT)) > 1.0;
    let suggestion = fails
        .then(|| {
            lighter_first(f.material, sec)
                .into_iter()
                .filter(|x| x.width <= sec.width + 1e-6 || f.material == FrameMaterial::Steel)
                .find(|x| {
                    let (r, d) = use_of(x);
                    r <= 1.0 && d <= l / LIMIT
                })
                .map(|x| format!("un dintel de 2 × {}", x.name))
        })
        .flatten();
    Some(Worst {
        ratio,
        deflection: Some((d, l / LIMIT)),
        what: format!("dintel de 2 × {} sobre {} m de luz", sec.name, dec(l, 2)),
        suggestion,
    })
}

/// The longest clear span of a member over the walls under it (whose top
/// is within 60 cm of `under`): between its ends and the walls it crosses.
fn clear_span(model: &Model, m: &Member, under: Option<f64>) -> f64 {
    let a = Point2::new(m.start[0], m.start[1]);
    let b = Point2::new(m.end[0], m.end[1]);
    let l = a.distance(b).max(0.1);
    let Some(z) = under else {
        return l;
    };
    let mut cuts = vec![0.0, l];
    let d = (b - a) * (1.0 / l);
    for w in model.walls() {
        let top = model.level(w.level).map_or(0.0, |x| x.elevation) + w.base_offset + w.height;
        if (top - z).abs() > 0.6 {
            continue;
        }
        let (p, q) = (w.start, w.end);
        let e = q - p;
        let den = d.cross(e);
        if den.abs() < 1e-9 {
            continue;
        }
        let t = (p - a).cross(e) / den;
        let u = (p - a).cross(d) / den;
        if t > 0.05 && t < l - 0.05 && (-0.01..=1.01).contains(&u) {
            cuts.push(t);
        }
    }
    cuts.sort_by(|x, y| x.total_cmp(y));
    cuts.windows(2).map(|w| w[1] - w[0]).fold(0.0, f64::max)
}

/// The joists or rafters of a floor or roof: the longest span. `under`:
/// the underside of a floor, to find the walls it rests on.
#[allow(clippy::too_many_arguments)]
fn spans(
    model: &Model,
    members: &[Member],
    role: Role,
    spacing: f64,
    g: f64,
    q: f64,
    material: FrameMaterial,
    under: Option<f64>,
) -> Option<Worst> {
    let longest = members
        .iter()
        .filter(|m| m.role == role)
        .max_by(|a, b| clear_span(model, a, under).total_cmp(&clear_span(model, b, under)))?;
    // The horizontal span between supports (rafters carry by plan area).
    let l = clear_span(model, longest, under);
    let use_of = |sec: &FrameSection| {
        let r = resistance(sec, l);
        let m_ed = (1.35 * g + 1.5 * q) * spacing * l * l / 8.0 * 1e3;
        let d = 5.0 * (g + q) * spacing * 1e3 * l.powi(4) / (384.0 * r.ei);
        (m_ed / r.m_rd, d)
    };
    let (ratio, d) = use_of(&longest.section);
    let suggestion = (ratio.max(d / (l / LIMIT)) > 1.0)
        .then(|| {
            lighter_first(material, &longest.section)
                .into_iter()
                .find(|s| {
                    let (r, d) = use_of(s);
                    r <= 1.0 && d <= l / LIMIT
                })
                .map(|s| s.name)
        })
        .flatten();
    let what = match role {
        Role::Rafter => "cabios",
        _ => "viguetas",
    };
    let suggestion = suggestion.or_else(|| {
        (ratio.max(d / (l / LIMIT)) > 1.0).then(|| {
            "un apoyo intermedio: ninguno de los perfiles habituales salva esta luz".to_string()
        })
    });
    Some(Worst {
        ratio,
        deflection: Some((d, l / LIMIT)),
        what: format!(
            "{what} {} a {} mm, {} m de luz",
            longest.section.name,
            (spacing * 1000.0).round(),
            dec(l, 2)
        ),
        suggestion,
    })
}

pub fn check(model: &Model, report: &mut Report) {
    if model.framings().next().is_none() {
        return;
    }
    for w in model.walls() {
        let members = wall_members(model, w.id);
        if members.is_empty() {
            continue;
        }
        let name = model
            .wall_type(w.wall_type)
            .map_or(String::new(), |t| t.name.clone());
        if let Some(r) = studs(model, w.id, &members) {
            push(
                report,
                Subject::Wall(w.id),
                Some(w.level),
                "Montantes (EC3-1-3 / EC5, predimensionado)",
                &name,
                r,
            );
        }
        if let Some(r) = headers(model, w.id, &members) {
            push(
                report,
                Subject::Wall(w.id),
                Some(w.level),
                "Dinteles (flexión y flecha, predimensionado)",
                &name,
                r,
            );
        }
    }
    for s in model.slabs() {
        let Some(f) = model.type_framing(s.slab_type.0) else {
            continue;
        };
        let members = slab_members(model, s.id);
        let name = model
            .slab_type(s.slab_type)
            .map_or(String::new(), |t| t.name.clone());
        let underside = model.level(s.level).map_or(0.0, |l| l.elevation) + s.top_offset
            - model.slab_type(s.slab_type).map_or(0.0, |t| t.thickness());
        if let Some(r) = spans(
            model,
            &members,
            Role::Joist,
            f.spacing,
            G_FLOOR,
            Q_FLOOR,
            f.material,
            Some(underside),
        ) {
            push(
                report,
                Subject::Floor(s.id),
                Some(s.level),
                "Viguetas (flexión y flecha, predimensionado)",
                &name,
                r,
            );
        }
    }
    for r in model.roofs() {
        let Some(f) = model.type_framing(r.roof_type.0) else {
            continue;
        };
        let members = roof_members(model, r.id);
        let name = model
            .slab_type(r.roof_type)
            .map_or(String::new(), |t| t.name.clone());
        if let Some(w) = spans(
            model,
            &members,
            Role::Rafter,
            f.spacing,
            G_ROOF,
            SNOW,
            f.material,
            None,
        ) {
            push(
                report,
                Subject::Roof(r.id),
                Some(r.level),
                "Cabios (flexión y flecha, predimensionado)",
                &name,
                w,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeper_and_thicker_sections_resist_more() {
        let a = resistance(&FrameSection::steel(90.0, 40.0, 12.0, 1.0), 2.7);
        let b = resistance(&FrameSection::steel(150.0, 40.0, 15.0, 1.5), 2.7);
        assert!(b.n_rd > a.n_rd && b.m_rd > a.m_rd && b.ei > a.ei);
        // A C 90 of 1 mm, 2,7 m: some 25–60 kN in compression.
        assert!(a.n_rd > 15e3 && a.n_rd < 70e3, "{}", a.n_rd);
        let t = resistance(&FrameSection::timber(45.0, 145.0), 2.7);
        // A 45 × 145 C24 stud: some 40–90 kN.
        assert!(t.n_rd > 30e3 && t.n_rd < 100e3, "{}", t.n_rd);
        // Slender plates lose more of their width.
        assert!(rho(0.2, 0.001, true) < rho(0.09, 0.001, true));
        assert!((rho(0.02, 0.002, true) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_long_floor_without_a_support_fails_and_says_why() {
        use archisim_core::*;
        let mut m = Model::new("Forjado");
        let level = m.add_level("Planta", 3.0);
        let wool = m.add_material("Lana", "insulation", [230, 220, 120], CutPattern::Solid);
        let st = m
            .add_slab_type(
                "Forjado de madera",
                SlabFunction::Floor,
                vec![Layer {
                    material: wool,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.set_type_framing(st.0, Some(Framing::for_depth(FrameMaterial::Timber, 0.2)))
            .unwrap();
        m.add_slab(NewSlab {
            slab_type: st,
            level,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(9.0, 0.0),
                Point2::new(9.0, 7.0),
                Point2::new(0.0, 7.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        let mut report = Report::default();
        check(&m, &mut report);
        let item = report
            .items
            .iter()
            .find(|i| i.section == Section::Framing)
            .unwrap();
        assert_eq!(item.status, Status::Fails, "{}", item.text);
        assert!(item.text.contains("apoyo intermedio"), "{}", item.text);
    }
}
