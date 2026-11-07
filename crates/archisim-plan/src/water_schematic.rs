//! The diagrams of the water installations, A3 landscape pages (SVG in
//! millimetres) drawn from the calculations of `archisim-mep`:
//!
//! - **Plumbing** (`pages`, from `water`): FO-01, the vertical diagram
//!   (esquema vertical): the levels one over another, the source (meter,
//!   tank or heater) with its pressure, the risers with each stretch
//!   between floors and its diameter, and on each floor a box for every
//!   branch that leaves the risers (a dwelling, a wet room): what it
//!   feeds, the diameter where it starts and the least pressure at its
//!   taps; cold water on the left half, hot on the right. Then one page per
//!   level with taps (distribución): the tree of every branch of the level,
//!   stretch by stretch with its name and diameter, down to each tap with
//!   the pressure left at it.
//! - **Drainage** (`drainage_pages`, from `drain`): SA-01, the same
//!   vertical diagram with the outlet at the foot, the downpipes and a box
//!   for every branch with its appliances and units of discharge; waste
//!   water on the left, rain water on the right (each roof's inlets and
//!   the area they take). Then one page per level with its branches down
//!   to every appliance or inlet.
//!
//! - **Ventilation** (`ventilation_pages`, from `ventilation`): VE-01,
//!   the extract shafts with what enters them on each floor and their fans
//!   on the roof; the general extraction on the left, the kitchens' hoods
//!   by their own ducts on the right. Then one page per level.
//!
//! All are drawn from one form (`Net`: lines, the stretches; leaves, the
//! taps, appliances or inlets), so they look alike. The diameters written
//! are the calculated ones; where the pipe drawn is smaller, both are
//! written («Ø20→26») in red.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use archisim_core::{Appliance, DeviceId, LevelId, Model, Point2, ServiceSystem};
use archisim_mep::drain::{self as dr, Drainage};
use archisim_mep::ventilation::{self as vt, Ventilation};
use archisim_mep::water::{self as wa, Source, Water, TAP_PRESSURE};

use crate::lettering::{paint, text_path, text_width, Align};
use crate::structure_sheets::{
    centred, line, n, page, rect, text, BLOCK_H, BLOCK_W, GREY, H, INK, MARGIN, MARK, RULE, W,
};

const STROKE: f64 = 0.35;
/// The dashes of the hot water (and of the rain water).
const HOT_DASH: &str = "2.2 1.1";
/// Where the drawing ends above the title block.
const BOTTOM: f64 = H - MARGIN - BLOCK_H - 6.0;

/// A stretch of a network, as drawn.
struct Line {
    name: String,
    level: LevelId,
    parent: Option<usize>,
    /// The riser or downpipe it goes up.
    riser: Option<DeviceId>,
    /// «Ø26» or «Ø20→26».
    diameter: String,
    red: bool,
    /// How many leaves it serves.
    count: usize,
}

/// What a network ends in: a tap, an appliance's drain, a rain inlet.
struct Leaf {
    path: Vec<usize>,
    /// What it is, for the boxes («Inodoro», «Sumidero»).
    kind: String,
    /// Under the leaf on a level's page.
    label: String,
    red: bool,
    /// The number the boxes give: least pressure, units, area.
    value: f64,
    /// With the pipes as drawn, for the boxes' red.
    drawn_red: bool,
}

/// What a box of the vertical diagram says under its rows.
#[derive(Clone, Copy)]
enum Footer {
    /// «P mín. 122 kPa».
    Pressure,
    /// «12 UD».
    Units,
    /// «45,0 m²».
    Area,
    /// «24,0 l/s».
    Flow,
}

/// The source symbol: a circle or a box with a letter.
#[derive(Clone, Copy)]
enum Mark {
    Circle(&'static str),
    Square(&'static str),
}

/// One network to draw.
struct Net {
    title: &'static str,
    dashed: bool,
    right: bool,
    source_level: Option<LevelId>,
    source: String,
    source_from: String,
    mark: Mark,
    lines: Vec<Line>,
    leaves: Vec<Leaf>,
    loose: usize,
    loose_text: &'static str,
    footer: Footer,
}

/// What the pages of one installation say.
struct Sheets {
    prefix: &'static str,
    vertical: &'static str,
    level_title: &'static str,
    discipline: &'static str,
    left: (&'static str, &'static str),
    right: (&'static str, &'static str),
    notes: Vec<String>,
}

/// The plumbing pages: the vertical diagram and one per level with taps.
/// Empty if there is no water network.
pub fn pages(model: &Model) -> Vec<String> {
    let w = wa::water(model);
    if w.networks.iter().all(|n| n.stretches.is_empty()) {
        return Vec::new();
    }
    let nets: Vec<Net> = w.networks.iter().map(|n| water_net(model, n)).collect();
    draw_pages(model, &nets, &water_sheets(&w))
}

/// The drainage pages: the vertical diagram and one per level with
/// branches. Empty if there is no drainage network.
pub fn drainage_pages(model: &Model) -> Vec<String> {
    let d = dr::drainage(model);
    if d.networks.iter().all(|n| n.stretches.is_empty()) {
        return Vec::new();
    }
    let nets: Vec<Net> = d.networks.iter().map(|n| drain_net(model, n)).collect();
    draw_pages(model, &nets, &drain_sheets(&d))
}

/// The ventilation pages (DB-HS 3): VE-01, the vertical diagram with the
/// extract shafts and, on each floor, what enters them (the valves of the
/// wet rooms, with their flow); the general extraction on the left and
/// the kitchens' hoods, by their own ducts, on the right. Then one page
/// per level with its ducts down to every valve and hood. Empty if there
/// is no extract duct.
pub fn ventilation_pages(model: &Model) -> Vec<String> {
    let v = vt::ventilation(model);
    if v.networks.iter().all(|n| n.stretches.is_empty()) {
        return Vec::new();
    }
    let mut nets = Vec::new();
    for hoods in [false, true] {
        let mine: Vec<&vt::Network> = v
            .networks
            .iter()
            .filter(|n| n.is_hood_duct() == hoods && !n.stretches.is_empty())
            .collect();
        if !mine.is_empty() {
            nets.push(vent_net(model, &v, &mine, hoods));
        }
    }
    draw_pages(model, &nets, &vent_sheets(&v))
}

/// The general extract networks (or the hoods' ducts) as one: their
/// stretches one after another.
fn vent_net(model: &Model, v: &Ventilation, nets: &[&vt::Network], hoods: bool) -> Net {
    let mut lines: Vec<Line> = Vec::new();
    let mut leaves: Vec<Leaf> = Vec::new();
    // A valve's room, for its kind and its flow.
    let room_of = |d| {
        v.dwellings
            .iter()
            .flat_map(|w| &w.rooms)
            .find(|r| r.vents.contains(&d))
    };
    for n in nets {
        let base = lines.len();
        for st in &n.stretches {
            let red = st.short();
            lines.push(Line {
                name: st.name.clone(),
                level: st.level,
                parent: st.parent.map(|p| p + base),
                riser: st.shaft,
                diameter: if red {
                    format!("{}→{}", st.size, st.sized_text)
                } else {
                    st.size.clone()
                },
                red,
                count: st.vents + st.hoods,
            });
        }
        let path = |at: Option<usize>| {
            let mut out = Vec::new();
            let mut cur = at;
            while let Some(i) = cur {
                out.push(i + base);
                cur = n.stretches[i].parent;
            }
            out.reverse();
            out
        };
        for (d, at) in n.vents.iter().zip(&n.vent_at) {
            let room = room_of(*d);
            let kind = match room {
                Some(r) if r.kitchen => "Cocina",
                Some(_) => "Baño o aseo",
                None => "Boca",
            };
            let flow = room.map_or(0.0, |r| r.extraction / r.vents.len().max(1) as f64);
            let red = room.is_some_and(|r| r.vent_short());
            leaves.push(Leaf {
                path: path(*at),
                kind: kind.to_string(),
                label: format!(
                    "{} · {} l/s",
                    room.map_or("Boca", |r| r.name.as_str()),
                    dec(flow, 1)
                ),
                red,
                value: flow,
                drawn_red: red,
            });
        }
        for (h, at) in n.hoods.iter().zip(&n.hood_at) {
            let hood = v.hoods.iter().find(|x| x.object == Some(*h));
            let red = hood.is_some_and(|x| v.hood_fails(x));
            leaves.push(Leaf {
                path: path(*at),
                kind: "Campana".to_string(),
                label: format!(
                    "Campana {} · {} l/s",
                    hood.map_or("", |x| x.dwelling.as_str()),
                    dec(vt::HOOD_FLOW, 0)
                ),
                red,
                value: vt::HOOD_FLOW,
                drawn_red: red,
            });
        }
    }
    let flow: f64 = nets.iter().map(|n| n.flow).sum();
    let fans: Vec<_> = nets
        .iter()
        .filter_map(|n| n.fan.and_then(|f| model.device(f)))
        .collect();
    let top = model
        .levels()
        .iter()
        .max_by(|a, b| a.elevation.total_cmp(&b.elevation))
        .map(|l| l.id);
    let missing = nets.iter().filter(|n| n.fan.is_none()).count();
    Net {
        title: if hoods {
            "CAMPANAS DE COCINA"
        } else {
            "EXTRACCIÓN GENERAL"
        },
        dashed: hoods,
        right: hoods,
        // The fan's level, or where the first duct comes out.
        source_level: fans
            .first()
            .map(|f| f.level)
            .or_else(|| {
                nets.iter()
                    .flat_map(|n| &n.stretches)
                    .find(|s| s.parent.is_none())
                    .map(|s| s.level)
            })
            .or(top),
        source: if hoods {
            format!(
                "Salida en cubierta · {} conducto{} · {} l/s",
                nets.len(),
                if nets.len() == 1 { "" } else { "s" },
                dec(flow, 0)
            )
        } else {
            format!(
                "{} extractor{} · {} m³/h",
                fans.len(),
                if fans.len() == 1 { "" } else { "es" },
                dec(flow * 3.6, 0)
            )
        },
        source_from: "la cubierta".to_string(),
        mark: Mark::Circle(if hoods { "C" } else { "E" }),
        lines,
        leaves,
        loose: if hoods { 0 } else { missing },
        loose_text: "redes sin extractor dibujado",
        footer: Footer::Flow,
    }
}

fn vent_sheets(v: &Ventilation) -> Sheets {
    let dwellings = v.dwellings.len();
    let flow: f64 = v.dwellings.iter().map(|d| d.flow).sum();
    Sheets {
        prefix: "VE",
        vertical: "Esquema vertical de ventilación",
        level_title: "Red de extracción",
        discipline: "Ventilación (CTE DB-HS 3) · sin escala",
        left: ("Extracción general", ""),
        right: ("Campanas de cocina", ""),
        notes: vec![
            format!(
                "{dwellings} vivienda{} · {} l/s de admisión y extracción en total (tabla 2.1) · extracción mecánica",
                if dwellings == 1 { "" } else { "s" },
                dec(flow, 1)
            ),
            format!(
                "Secciones de los conductos a {} cm² por l/s (tabla 4.2); bocas y aireadores a {} cm² por l/s (tabla 4.1). En rojo: lo dibujado menor (dibujado→calculado).",
                dec(vt::DUCT_PER_LS, 1),
                dec(vt::OPENING_PER_LS, 0)
            ),
            format!(
                "Cada campana, {} l/s por un conducto propio, independiente de la ventilación general (3.1.1).",
                dec(vt::HOOD_FLOW, 0)
            ),
        ],
    }
}

fn draw_pages(model: &Model, nets: &[Net], sheets: &Sheets) -> Vec<String> {
    let _font = crate::lettering::use_font(model.graphics.font);
    let trees: Vec<Tree> = nets.iter().map(Tree::of).collect();
    let mut out = vec![vertical_page(model, &trees, sheets)];
    let mut k = 2;
    for l in model.levels() {
        if trees.iter().any(|t| !t.roots_on(l.id).is_empty()) {
            let code = format!("{}-{k:02}", sheets.prefix);
            out.push(level_page(model, &trees, l.id, &code, sheets));
            k += 1;
        }
    }
    out
}

/// «Ø26», or «Ø20→26» if the pipe drawn is smaller; and whether it is.
fn diameter(drawn: f64, sized: f64) -> (String, bool) {
    let d = (drawn * 1000.0).round();
    let z = (sized * 1000.0).round();
    if d + 0.5 < z {
        (format!("Ø{d:.0}→{z:.0}"), true)
    } else {
        (format!("Ø{z:.0}"), false)
    }
}

fn short(a: Appliance) -> &'static str {
    match a {
        Appliance::Wc => "Inodoro",
        other => other.name(),
    }
}

fn dec(v: f64, d: usize) -> String {
    format!("{v:.d$}").replace('.', ",")
}

fn water_net(model: &Model, net: &wa::Network) -> Net {
    let hot = net.system == ServiceSystem::HotWater;
    let lines = net
        .stretches
        .iter()
        .map(|st| {
            let (diameter, red) = diameter(st.drawn.0, st.sized.0);
            Line {
                name: st.name.clone(),
                level: st.level,
                parent: st.parent,
                riser: st.riser,
                diameter,
                red,
                count: st.appliances,
            }
        })
        .collect();
    let leaves = net
        .taps
        .iter()
        .map(|t| Leaf {
            path: t.path.clone(),
            kind: short(t.kind).to_string(),
            label: format!("{} · {} kPa", short(t.kind), dec(t.sized_pressure, 0)),
            red: t.sized_pressure < TAP_PRESSURE || t.pressure < TAP_PRESSURE,
            value: t.sized_pressure,
            drawn_red: t.pressure < TAP_PRESSURE,
        })
        .collect();
    let source_level = match &net.source {
        Source::Device(d, _) => model.device(*d).map(|d| d.level),
        Source::End(l, _) => Some(*l),
    };
    let name = match &net.source {
        Source::Device(..) => wa::source_name(model, net),
        Source::End(..) => "Extremo de la red".to_string(),
    };
    Net {
        title: if hot {
            "AGUA CALIENTE SANITARIA"
        } else {
            "AGUA FRÍA"
        },
        dashed: hot,
        right: hot,
        source_level,
        source: format!("{name} · {} kPa", dec(net.source_pressure, 0)),
        source_from: wa::source_name(model, net),
        mark: if hot {
            Mark::Square("ACS")
        } else {
            Mark::Circle("C")
        },
        lines,
        leaves,
        loose: net.loose.len(),
        loose_text: "sin unir a esta red",
        footer: Footer::Pressure,
    }
}

fn drain_net(model: &Model, net: &dr::Network) -> Net {
    let rain = net.system == ServiceSystem::Rainwater;
    let lines: Vec<Line> = net
        .stretches
        .iter()
        .map(|st| {
            let (diameter, red) = diameter(st.drawn, st.sized);
            Line {
                name: st.name.clone(),
                level: st.level,
                parent: st.parent,
                riser: st.downpipe,
                diameter,
                red,
                count: if rain { 1 } else { st.appliances },
            }
        })
        .collect();
    let mut leaves: Vec<Leaf> = net
        .drains
        .iter()
        .map(|d| {
            let red = d.path.last().is_some_and(|i| net.stretches[*i].short());
            Leaf {
                path: d.path.clone(),
                kind: short(d.kind).to_string(),
                label: format!("{} · {} UD", short(d.kind), d.units),
                red,
                value: f64::from(d.units),
                drawn_red: red,
            }
        })
        .collect();
    leaves.extend(net.inlets.iter().map(|i| Leaf {
        path: i.path.clone(),
        kind: "Sumidero".to_string(),
        label: format!("Sumidero · {} m²", dec(i.area, 1)),
        red: false,
        value: i.area,
        drawn_red: false,
    }));
    let outlet = dr::outlet_name(model, net);
    Net {
        title: if rain {
            "AGUAS PLUVIALES"
        } else {
            "AGUAS RESIDUALES"
        },
        dashed: rain,
        right: rain,
        source_level: Some(net.outlet.0),
        source: if rain {
            format!("{outlet} · {} m²", dec(net.area(), 1))
        } else {
            format!("{outlet} · {} UD", net.units())
        },
        source_from: "la acometida".to_string(),
        mark: Mark::Square(if rain { "P" } else { "A" }),
        lines,
        leaves,
        loose: 0,
        loose_text: "",
        footer: if rain { Footer::Area } else { Footer::Units },
    }
}

fn water_sheets(w: &Water) -> Sheets {
    Sheets {
        prefix: "FO",
        vertical: "Esquema vertical de fontanería",
        level_title: "Distribución de fontanería",
        discipline: "Fontanería (CTE DB-HS 4) · sin escala",
        left: ("Agua fría", ""),
        right: ("Agua caliente sanitaria", ""),
        notes: vec![
            format!(
                "Presión en el contador {} kPa · {} · simultaneidad {} · v ≤ {} m/s · +{} % de pérdidas locales",
                dec(w.pressure, 0),
                w.material.name(),
                if w.dwelling {
                    "UNE 149201 (viviendas)"
                } else {
                    "Qt/√(n−1)"
                },
                dec(wa::DESIGN_SPEED, 1),
                dec(wa::FITTINGS * 100.0, 0)
            ),
            format!(
                "Diámetros exteriores calculados (mm). En rojo: tubería dibujada menor (dibujado→calculado) y grifos por debajo de {TAP_PRESSURE:.0} kPa con lo dibujado."
            ),
            "Presión mínima en grifos 100 kPa y en calentadores 150 kPa; máxima 500 kPa (HS 4, 2.1.3).".into(),
        ]
        .into_iter()
        .chain(w.group.as_ref().map(|g| {
            format!(
                "Grupo de presión{}: {} l/s, Pmín {} kPa, Pmáx {} kPa, {} bombas{}.",
                if g.drawn.is_some() { "" } else { " (a poner)" },
                dec(g.flow, 2),
                dec(g.min, 0),
                dec(g.max, 0),
                g.pumps,
                w.tank
                    .as_ref()
                    .map_or(String::new(), |t| format!(" · aljibe de {} l", dec(t.volume, 0)))
            )
        }))
        .chain(w.hot_return.as_ref().map(|r| {
            format!(
                "Retorno de ACS, {}: {} m al grifo más lejano; {} l/h por {} columna{}, tubería {} aislada (HS 4, 2.3 y tabla 4.4).",
                r.drawn_text(),
                dec(r.length, 1),
                dec(r.flow, 0),
                r.columns,
                if r.columns == 1 { "" } else { "s" },
                r.pipe.0
            )
        }))
        .collect(),
    }
}

fn drain_sheets(d: &Drainage) -> Sheets {
    Sheets {
        prefix: "SA",
        vertical: "Esquema vertical de saneamiento",
        level_title: "Red de saneamiento",
        discipline: "Saneamiento (CTE DB-HS 5) · sin escala",
        left: ("Aguas residuales", ""),
        right: ("Aguas pluviales", ""),
        notes: vec![
            format!(
                "PVC · unidades de desagüe de uso {} (tabla 4.1) · lluvia {} mm/h (factor {})",
                if d.public { "público" } else { "privado" },
                dec(d.intensity, 0),
                dec(d.factor(), 2)
            ),
            "Diámetros exteriores calculados (mm): derivaciones tabla 4.1, ramales 4.3, bajantes 4.4 y 4.8, colectores 4.5 y 4.9. En rojo: tubería dibujada menor (dibujado→calculado).".into(),
            "Ningún tramo disminuye de diámetro aguas abajo; los colectores mixtos, por la tabla 4.9 con las UD en superficie equivalente (4.3).".into(),
        ],
    }
}

/// A network as a tree of stretches.
struct Tree<'a> {
    net: &'a Net,
    children: Vec<Vec<usize>>,
    /// The leaves whose last stretch is each one.
    taps_on: Vec<Vec<usize>>,
    /// The stretches on the way to a riser (and the risers).
    trunk: Vec<bool>,
}

impl<'a> Tree<'a> {
    fn of(net: &'a Net) -> Self {
        let k = net.lines.len();
        let mut children = vec![Vec::new(); k];
        for (i, s) in net.lines.iter().enumerate() {
            if let Some(p) = s.parent {
                children[p].push(i);
            }
        }
        let mut taps_on = vec![Vec::new(); k];
        for (t, tap) in net.leaves.iter().enumerate() {
            if let Some(last) = tap.path.last() {
                taps_on[*last].push(t);
            }
        }
        // Children come after their parent: from the end, upwards.
        let mut trunk = vec![false; k];
        for i in (0..k).rev() {
            if net.lines[i].riser.is_some() || children[i].iter().any(|c| trunk[*c]) {
                trunk[i] = true;
            }
        }
        Tree {
            net,
            children,
            taps_on,
            trunk,
        }
    }

    /// The branches that leave the trunk (or the source) on a level, or
    /// come down from another level.
    fn roots_on(&self, level: LevelId) -> Vec<usize> {
        let st = &self.net.lines;
        (0..st.len())
            .filter(|i| !self.trunk[*i] && st[*i].level == level)
            .filter(|i| match st[*i].parent {
                None => true,
                Some(p) => self.trunk[p] || st[p].level != level,
            })
            .collect()
    }

    /// The leaves after a stretch.
    fn taps_after(&self, s: usize) -> Vec<usize> {
        (0..self.net.leaves.len())
            .filter(|t| self.net.leaves[*t].path.contains(&s))
            .collect()
    }

    /// The boxes of a branch for the vertical diagram: the branch itself,
    /// or, if it is large and splits, each of the branches it splits into
    /// (with the leaves on the split, if any, as one more), split again in
    /// the same way.
    fn boxes(&self, root: usize) -> Vec<Group> {
        let mut cur = root;
        while self.children[cur].len() == 1 && self.taps_on[cur].is_empty() {
            cur = self.children[cur][0];
        }
        let parts = self.children[cur].len() + usize::from(!self.taps_on[cur].is_empty());
        if self.net.lines[root].count <= 6 || parts < 2 {
            return vec![Group {
                head: root,
                taps: self.taps_after(root),
            }];
        }
        // Each part split again while it is large.
        let mut out: Vec<Group> = self.children[cur]
            .iter()
            .flat_map(|c| self.boxes(*c))
            .collect();
        if !self.taps_on[cur].is_empty() {
            out.push(Group {
                head: cur,
                taps: self.taps_on[cur].clone(),
            });
        }
        out
    }

    /// The trunk stretch a branch hangs from, if any.
    fn trunk_of(&self, s: usize) -> Option<usize> {
        let mut p = self.net.lines[s].parent;
        while let Some(i) = p {
            if self.trunk[i] {
                return Some(i);
            }
            p = self.net.lines[i].parent;
        }
        None
    }
}

/// A box of the vertical diagram.
struct Group {
    head: usize,
    taps: Vec<usize>,
}

fn dash_line(s: &mut String, a: (f64, f64), b: (f64, f64), w: f64, colour: &str, dashed: bool) {
    if !dashed {
        line(s, a, b, w, colour);
        return;
    }
    let _ = writeln!(
        s,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{colour}" stroke-width="{}" stroke-dasharray="{HOT_DASH}"/>"#,
        n(a.0),
        n(a.1),
        n(b.0),
        n(b.1),
        n(w)
    );
}

/// A text written upwards, its baseline on `x`, starting at `y`.
fn text_up(s: &mut String, t: &str, x: f64, y: f64, h: f64, colour: &str) {
    if t.trim().is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        r#"<path d="{}" {}/>"#,
        text_path(t, Point2::new(x, -y), Point2::new(0.0, 1.0), h, Align::Left),
        paint(h, colour)
    );
}

fn frame(s: &mut String, model: &Model, code: &str, title: &str, discipline: &str) {
    rect(
        s,
        MARGIN,
        MARGIN,
        W - 2.0 * MARGIN,
        H - 2.0 * MARGIN,
        0.5,
        INK,
    );
    let (x, y) = (W - MARGIN - BLOCK_W, H - MARGIN - BLOCK_H);
    rect(s, x, y, BLOCK_W, BLOCK_H, 0.35, INK);
    line(
        s,
        (x + BLOCK_W - 30.0, y),
        (x + BLOCK_W - 30.0, y + BLOCK_H),
        0.25,
        INK,
    );
    text(s, &model.project_name, x + 4.0, y + 7.0, 3.5, INK);
    text(s, title, x + 4.0, y + 15.0, 3.0, INK);
    text(s, discipline, x + 4.0, y + 22.5, 2.5, GREY);
    centred(s, code, x + BLOCK_W - 15.0, y + 17.0, 6.0, INK);
}

/// The legend and the data of the calculation, under the drawing.
fn legend(s: &mut String, sheets: &Sheets) {
    let y = H - MARGIN - BLOCK_H + 4.0;
    let x = MARGIN + 6.0;
    line(s, (x, y), (x + 12.0, y), STROKE, INK);
    text(s, sheets.left.0, x + 14.0, y + 1.0, 2.4, INK);
    dash_line(s, (x + 46.0, y), (x + 58.0, y), STROKE, INK, true);
    text(s, sheets.right.0, x + 60.0, y + 1.0, 2.4, INK);
    for (i, note) in sheets.notes.iter().enumerate() {
        text(s, note, x, y + 7.0 + 4.0 * i as f64, 2.2, GREY);
    }
}

/// The levels with something of the networks, bottom first, and the
/// height of each on the page.
fn level_heights(model: &Model, trees: &[Tree]) -> Vec<(LevelId, f64)> {
    let mut used: Vec<LevelId> = Vec::new();
    for t in trees {
        for st in &t.net.lines {
            used.push(st.level);
        }
        used.extend(t.net.source_level);
    }
    let levels: Vec<LevelId> = model
        .levels()
        .iter()
        .map(|l| l.id)
        .filter(|l| used.contains(l))
        .collect();
    let (top, bottom) = (MARGIN + 46.0, BOTTOM - 22.0);
    let k = levels.len();
    if k <= 1 {
        return levels.into_iter().map(|l| (l, bottom)).collect();
    }
    let step = ((bottom - top) / (k - 1) as f64).min(60.0);
    // Few levels: in the middle of the page.
    let lift = ((bottom - top) - step * (k - 1) as f64) / 2.0;
    levels
        .into_iter()
        .enumerate()
        .map(|(i, l)| (l, bottom - lift - step * i as f64))
        .collect()
}

fn vertical_page(model: &Model, trees: &[Tree], sheets: &Sheets) -> String {
    let mut s = String::new();
    let code = format!("{}-01", sheets.prefix);
    frame(&mut s, model, &code, sheets.vertical, sheets.discipline);
    text(
        &mut s,
        &sheets.vertical.to_uppercase(),
        MARGIN + 6.0,
        MARGIN + 10.0,
        4.5,
        INK,
    );
    let heights = level_heights(model, trees);
    let y_of = |l: LevelId| {
        heights
            .iter()
            .find(|h| h.0 == l)
            .map_or(BOTTOM - 22.0, |h| h.1)
    };
    // The floor lines across the page, with their names.
    for (l, y) in &heights {
        let Some(level) = model.level(*l) else {
            continue;
        };
        let _ = writeln!(
            s,
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{RULE}" stroke-width="0.25" stroke-dasharray="4 2"/>"#,
            n(MARGIN + 4.0),
            n(*y),
            n(W - MARGIN - 4.0),
            n(*y)
        );
        text(&mut s, &level.name, MARGIN + 5.0, y - 1.5, 2.6, INK);
        text(
            &mut s,
            &format!(
                "{}{} m",
                if level.elevation >= 0.0 { "+" } else { "" },
                dec(level.elevation, 2)
            ),
            MARGIN + 5.0,
            y + 3.5,
            2.2,
            GREY,
        );
    }
    let halves = [
        (MARGIN + 50.0, W / 2.0 + 6.0),
        (W / 2.0 + 18.0, W - MARGIN - 6.0),
    ];
    // Several networks on one half share it.
    for side in [false, true] {
        let mine: Vec<&Tree> = trees.iter().filter(|t| t.net.right == side).collect();
        let (x0, x1) = halves[usize::from(side)];
        let k = mine.len().max(1) as f64;
        let w = (x1 - x0) / k;
        for (i, tree) in mine.iter().enumerate() {
            let a = x0 + w * i as f64;
            network_half(&mut s, model, tree, a, a + w - 4.0, &y_of, i == 0);
        }
    }
    legend(&mut s, sheets);
    page(&s)
}

/// One network on the vertical diagram, between `x0` and `x1`.
fn network_half(
    s: &mut String,
    model: &Model,
    tree: &Tree,
    x0: f64,
    x1: f64,
    y_of: &dyn Fn(LevelId) -> f64,
    first: bool,
) {
    let net = tree.net;
    let dashed = net.dashed;
    if first {
        text(s, net.title, x0, MARGIN + 18.0, 3.0, INK);
    }
    // A column for each riser.
    let mut risers: Vec<DeviceId> = net.lines.iter().filter_map(|s| s.riser).collect();
    risers.sort();
    risers.dedup();
    // Many risers: a column each across the width, and what enters each
    // one on every floor written by it instead of boxes.
    let columns = risers.len() > 4;
    let spacing = if columns {
        ((x1 - x0 - 12.0) / risers.len() as f64).clamp(9.0, 36.0)
    } else {
        9.0
    };
    let col_of = |d: Option<DeviceId>| {
        let i = d
            .and_then(|d| risers.iter().position(|r| *r == d))
            .unwrap_or(0);
        x0 + 6.0 + spacing * i as f64
    };
    // The source, under its floor.
    let y_src = net.source_level.map_or(BOTTOM - 22.0, y_of);
    let xs = col_of(risers.first().copied());
    let pipe = |y: f64| y - 4.0;
    let sy = y_src + 9.0;
    match net.mark {
        Mark::Square(t) => {
            rect(s, xs - 3.5, sy - 3.0, 7.0, 6.0, STROKE, INK);
            centred(s, t, xs, sy + 1.0, 2.0, INK);
        }
        Mark::Circle(t) => {
            let _ = writeln!(
                s,
                r#"<circle cx="{}" cy="{}" r="3" fill="none" stroke="{INK}" stroke-width="{STROKE}"/>"#,
                n(xs),
                n(sy)
            );
            centred(s, t, xs, sy + 1.0, 2.2, INK);
        }
    }
    let source = fit_text(std::slice::from_ref(&net.source), x1 - xs - 5.0, 2.3);
    text(s, &source, xs + 5.0, sy + 1.0, 2.3, INK);
    dash_line(s, (xs, sy - 3.0), (xs, pipe(y_src)), STROKE, INK, dashed);
    // The trunk: risers up the columns, the rest named by the source.
    let mut by_source: Vec<String> = Vec::new();
    for (i, st) in net.lines.iter().enumerate() {
        if !tree.trunk[i] {
            continue;
        }
        let (d, red) = (&st.diameter, st.red);
        let colour = if red { MARK } else { INK };
        match st.riser {
            Some(r) => {
                let x = col_of(Some(r));
                let below = st.parent.map_or(y_src, |p| y_of(net.lines[p].level));
                let (ya, yb) = (pipe(below), pipe(y_of(st.level)));
                dash_line(s, (x, ya), (x, yb), 0.5, INK, dashed);
                // Along the riser, on its left.
                let label = format!("{} {d}", st.name);
                let len = text_width(&label, 2.1);
                text_up(s, &label, x - 0.9, (ya + yb + len) / 2.0, 2.1, colour);
            }
            None => by_source.push(format!("{} {d}", st.name)),
        }
    }
    if !by_source.is_empty() {
        let fits = fit_text(&by_source, x1 - xs - 5.0, 2.1);
        text(s, &fits, xs + 5.0, sy + 5.0, 2.1, GREY);
    }
    // The boxes of each floor.
    let mut by_level: BTreeMap<i64, (LevelId, Vec<Group>)> = BTreeMap::new();
    for l in model.levels() {
        let roots = tree.roots_on(l.id);
        if roots.is_empty() {
            continue;
        }
        let groups: Vec<Group> = roots.iter().flat_map(|r| tree.boxes(*r)).collect();
        by_level.insert((l.elevation * 1000.0) as i64, (l.id, groups));
    }
    let riser_of = |g: &Group| tree.trunk_of(g.head).and_then(|t| net.lines[t].riser);
    if columns {
        // The collector at the foot of the risers.
        let last = col_of(risers.last().copied());
        dash_line(s, (xs, pipe(y_src)), (last, pipe(y_src)), 0.5, INK, dashed);
        // What enters each riser on each floor, by it.
        let h = (spacing / 8.0).clamp(1.5, 2.1);
        for (level, groups) in by_level.values() {
            let y = y_of(*level);
            let mut by_riser: BTreeMap<Option<DeviceId>, Vec<&Group>> = BTreeMap::new();
            for g in groups {
                by_riser.entry(riser_of(g)).or_default().push(g);
            }
            for (r, gs) in by_riser {
                let x = col_of(r.or(risers.first().copied()));
                dash_line(
                    s,
                    (x, pipe(y)),
                    (x + spacing * 0.35, pipe(y)),
                    STROKE,
                    INK,
                    dashed,
                );
                let head = &net.lines[gs[0].head];
                let red = gs.iter().any(|g| net.lines[g.head].red);
                let taps: Vec<usize> = gs.iter().flat_map(|g| g.taps.iter().copied()).collect();
                text(
                    s,
                    &head.diameter,
                    x + 1.0,
                    pipe(y) - 1.2 - h,
                    h,
                    if red { MARK } else { INK },
                );
                if let Some((foot, red)) = footer(net, &taps) {
                    let foot = foot.replace("P mín. ", "");
                    text(
                        s,
                        &foot,
                        x + 1.0,
                        pipe(y) - 1.0,
                        h,
                        if red { MARK } else { GREY },
                    );
                }
            }
        }
        return;
    }
    let start = col_of(risers.last().copied()) + 16.0;
    for (level, groups) in by_level.values() {
        let y = y_of(*level);
        // Too many for the width: one box per riser.
        let merged: Vec<Group>;
        let groups = if groups.len() as f64 * 21.0 > x1 - start {
            let mut by_riser: BTreeMap<Option<DeviceId>, Group> = BTreeMap::new();
            for g in groups {
                let e = by_riser.entry(riser_of(g)).or_insert(Group {
                    head: g.head,
                    taps: Vec::new(),
                });
                e.taps.extend(g.taps.iter().copied());
            }
            merged = by_riser.into_values().collect();
            &merged
        } else {
            groups
        };
        let k = groups.len().max(1) as f64;
        let gap = 3.0;
        let bw = ((x1 - start - gap * (k - 1.0)) / k).clamp(18.0, 44.0);
        let hpx = (bw / 44.0).clamp(0.7, 1.0);
        for (i, g) in groups.iter().enumerate() {
            let bx = start + i as f64 * (bw + gap);
            if bx + bw > x1 + 1.0 {
                // No room: say how many are left.
                text(
                    s,
                    &format!("+{} ramales más", groups.len() - i),
                    bx.min(x1 - 20.0),
                    pipe(y) - 2.0,
                    2.2,
                    GREY,
                );
                break;
            }
            let from = col_of(
                tree.trunk_of(g.head)
                    .and_then(|t| net.lines[t].riser)
                    .or(risers.first().copied()),
            );
            let xc = bx + bw / 2.0;
            // The pipe from the riser along the floor, and up to the box.
            dash_line(s, (from, pipe(y)), (xc, pipe(y)), STROKE, INK, dashed);
            // What it feeds.
            let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
            for t in &g.taps {
                *kinds.entry(net.leaves[*t].kind.as_str()).or_default() += 1;
            }
            let rows = kinds.len() as f64;
            let rh = 3.1 * hpx;
            let bh = 5.0 + rh * (rows + 1.0) + 1.0;
            let top = pipe(y) - 3.0 - bh;
            dash_line(s, (xc, pipe(y)), (xc, top + bh), STROKE, INK, dashed);
            rect(s, bx, top, bw, bh, 0.3, INK);
            let st = &net.lines[g.head];
            let (d, red) = (&st.diameter, st.red);
            text(
                s,
                &format!("{} {d}", st.name),
                bx + 1.2,
                top + 3.6,
                2.3 * hpx,
                if red { MARK } else { INK },
            );
            line(s, (bx, top + 5.0), (bx + bw, top + 5.0), 0.15, RULE);
            for (r, (a, c)) in kinds.iter().enumerate() {
                text(
                    s,
                    &format!("{c} {a}"),
                    bx + 1.2,
                    top + 5.0 + rh * (r as f64 + 0.85),
                    2.1 * hpx,
                    INK,
                );
            }
            if let Some((foot, red)) = footer(net, &g.taps) {
                text(
                    s,
                    &foot,
                    bx + 1.2,
                    top + 5.0 + rh * (rows + 0.85),
                    2.1 * hpx,
                    if red { MARK } else { GREY },
                );
            }
        }
    }
    if net.loose > 0 {
        text(
            s,
            &format!(
                "{} aparato{} {}",
                net.loose,
                if net.loose == 1 { "" } else { "s" },
                net.loose_text
            ),
            x0,
            MARGIN + 23.0,
            2.3,
            MARK,
        );
    }
}

/// Texts joined by « · », cut with «…» to fit `width` at height `h`.
fn fit_text(parts: &[String], width: f64, h: f64) -> String {
    let all = parts.join(" · ");
    if text_width(&all, h) <= width {
        return all;
    }
    let mut out = String::new();
    for c in all.chars() {
        let mut next = out.clone();
        next.push(c);
        next.push('…');
        if text_width(&next, h) > width {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}

/// The last line of a box: the least pressure, the units or the area of
/// its leaves; and whether it is in red.
fn footer(net: &Net, taps: &[usize]) -> Option<(String, bool)> {
    let leaves: Vec<&Leaf> = taps.iter().map(|t| &net.leaves[*t]).collect();
    if leaves.is_empty() {
        return None;
    }
    let red = leaves.iter().any(|l| l.red || l.drawn_red);
    Some(match net.footer {
        Footer::Pressure => {
            let least = leaves.iter().map(|l| l.value).fold(f64::INFINITY, f64::min);
            (format!("P mín. {} kPa", dec(least, 0)), red)
        }
        Footer::Units => {
            let all: f64 = leaves.iter().map(|l| l.value).sum();
            (format!("{} UD", dec(all, 0)), red)
        }
        Footer::Area => {
            let all: f64 = leaves.iter().map(|l| l.value).sum();
            (format!("{} m²", dec(all, 1)), red)
        }
        Footer::Flow => {
            let all: f64 = leaves.iter().map(|l| l.value).sum();
            (format!("{} l/s", dec(all, 1)), red)
        }
    })
}

/// A tree laid out: each stretch with its x, its depth and where its
/// children's bar is; each tap with its x.
struct Layout {
    x: BTreeMap<usize, f64>,
    depth: BTreeMap<usize, usize>,
    taps: Vec<(usize, f64, usize)>,
    deepest: usize,
}

/// Lays out the branches from `roots`, a leaf every `step` from `x`.
fn lay_out(tree: &Tree, roots: &[usize], x: &mut f64, step: f64) -> Layout {
    fn walk(tree: &Tree, s: usize, d: usize, x: &mut f64, step: f64, out: &mut Layout) -> f64 {
        let mut xs = Vec::new();
        for t in &tree.taps_on[s] {
            out.taps.push((*t, *x, d + 1));
            xs.push(*x);
            *x += step;
        }
        for c in &tree.children[s] {
            if tree.trunk[*c] {
                continue;
            }
            xs.push(walk(tree, *c, d + 1, x, step, out));
        }
        let me = if xs.is_empty() {
            let v = *x;
            *x += step;
            v
        } else {
            (xs[0] + xs[xs.len() - 1]) / 2.0
        };
        out.x.insert(s, me);
        out.depth.insert(s, d);
        out.deepest = out.deepest.max(d + 1);
        me
    }
    let mut out = Layout {
        x: BTreeMap::new(),
        depth: BTreeMap::new(),
        taps: Vec::new(),
        deepest: 0,
    };
    for r in roots {
        walk(tree, *r, 0, x, step, &mut out);
        *x += step * 0.8;
    }
    out
}

/// How many leaves the branches from `roots` have.
fn leaves(tree: &Tree, roots: &[usize]) -> usize {
    fn count(tree: &Tree, s: usize) -> usize {
        let kids: usize = tree.children[s]
            .iter()
            .filter(|c| !tree.trunk[**c])
            .map(|c| count(tree, *c))
            .sum();
        let own = tree.taps_on[s].len();
        (kids + own).max(1)
    }
    roots.iter().map(|r| count(tree, *r)).sum()
}

fn level_page(
    model: &Model,
    trees: &[Tree],
    level: LevelId,
    code: &str,
    sheets: &Sheets,
) -> String {
    let mut s = String::new();
    let name = model.level(level).map_or(String::new(), |l| l.name.clone());
    frame(
        &mut s,
        model,
        code,
        &format!("{} · {name}", sheets.level_title),
        sheets.discipline,
    );
    text(
        &mut s,
        &format!(
            "{} · {}",
            sheets.level_title.to_uppercase(),
            name.to_uppercase()
        ),
        MARGIN + 6.0,
        MARGIN + 10.0,
        4.5,
        INK,
    );
    let parts: Vec<(&Tree, Vec<usize>)> = trees
        .iter()
        .map(|t| (t, t.roots_on(level)))
        .filter(|(_, r)| !r.is_empty())
        .collect();
    let total: f64 = parts
        .iter()
        .map(|(t, r)| leaves(t, r) as f64 + 0.8 * r.len() as f64 + 2.0)
        .sum();
    let (left, right) = (MARGIN + 10.0, W - MARGIN - 8.0);
    let step = ((right - left) / total.max(1.0)).clamp(2.5, 20.0);
    let hp = (step / 8.0).clamp(0.65, 1.0);
    let top = MARGIN + 30.0;
    // Few taps: in the middle of the page.
    let mut x = left + ((right - left) - step * total).max(0.0) / 2.0 + step;
    for (tree, roots) in &parts {
        let net = tree.net;
        let dashed = net.dashed;
        let x_start = x;
        let lay = lay_out(tree, roots, &mut x, step);
        text(
            &mut s,
            net.title,
            x_start - step * 0.5,
            MARGIN + 18.0,
            3.0,
            INK,
        );
        let dy = ((BOTTOM - 34.0 - top) / (lay.deepest.max(1) as f64)).min(24.0);
        let leaf_y = top + dy * lay.deepest as f64 + 10.0;
        let y_at = |d: usize| top + dy * d as f64;
        for (st_i, sx) in &lay.x {
            let st = &net.lines[*st_i];
            let d = lay.depth[st_i];
            let (ya, yb) = (y_at(d), y_at(d + 1));
            dash_line(&mut s, (*sx, ya), (*sx, yb), STROKE, INK, dashed);
            // The bar to its children.
            let mut kids: Vec<f64> = tree.children[*st_i]
                .iter()
                .filter_map(|c| lay.x.get(c).copied())
                .collect();
            kids.extend(
                lay.taps
                    .iter()
                    .filter(|t| tree.taps_on[*st_i].contains(&t.0))
                    .map(|t| t.1),
            );
            if kids.len() > 1 {
                let (a, b) = kids
                    .iter()
                    .fold((f64::MAX, f64::MIN), |m, v| (m.0.min(*v), m.1.max(*v)));
                dash_line(&mut s, (a, yb), (b, yb), STROKE, INK, dashed);
            } else if let Some(k) = kids.first() {
                dash_line(&mut s, (*sx, yb), (*k, yb), STROKE, INK, dashed);
            }
            let (dt, red) = (&st.diameter, st.red);
            let label = format!("{} {dt}", st.name);
            let h = 1.8 * hp;
            let len = text_width(&label, h);
            text_up(
                &mut s,
                &label,
                sx + 0.9 + h,
                yb - ((yb - ya - len) / 2.0).max(0.6),
                h,
                if red { MARK } else { INK },
            );
            if d == 0 {
                // Where it comes from.
                let from = match st.parent {
                    Some(p) => format!("de {}", net.lines[p].name),
                    None => format!("de {}", net.source_from),
                };
                centred(&mut s, &from, *sx, ya - 1.5, 2.0 * hp, GREY);
            }
        }
        for (t, tx, d) in &lay.taps {
            let tap = &net.leaves[*t];
            dash_line(&mut s, (*tx, y_at(*d)), (*tx, leaf_y), STROKE, INK, dashed);
            // The tap: a small triangle.
            let _ = writeln!(
                s,
                r#"<path d="M {} {} L {} {} L {} {} Z" fill="{INK}"/>"#,
                n(tx - 1.0),
                n(leaf_y),
                n(tx + 1.0),
                n(leaf_y),
                n(*tx),
                n(leaf_y + 1.8)
            );
            let label = &tap.label;
            let h = 1.9 * hp;
            let len = text_width(label, h);
            let red = tap.red;
            text_up(
                &mut s,
                label,
                tx + h * 0.4,
                leaf_y + 3.0 + len,
                h,
                if red { MARK } else { INK },
            );
        }
        x += step * 2.0;
    }
    legend(&mut s, sheets);
    page(&s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{DeviceKind, NewDevice, NewObject, NewRun};

    /// A house of two floors: meter and heater downstairs, a cold and a
    /// hot riser, a bathroom on each floor and a kitchen sink downstairs.
    fn house() -> Model {
        let mut m = archisim_library::new_project("Casa");
        let l0 = m.levels()[0].id;
        let l1 = m.add_level("Planta 1", 3.0);
        let wc = archisim_library::add_object_entry(&mut m, "Inodoro").unwrap();
        let basin = archisim_library::add_object_entry(&mut m, "Lavabo 60").unwrap();
        let shower = archisim_library::add_object_entry(&mut m, "Plato de ducha 90×90").unwrap();
        for l in [l0, l1] {
            for (t, x) in [(wc, 2.0), (basin, 3.5), (shower, 5.0)] {
                m.add_object(NewObject {
                    level: l,
                    object_type: t,
                    position: Point2::new(x, 0.8),
                    rotation: 0.0,
                    base_offset: 0.0,
                })
                .unwrap();
            }
        }
        let device = |m: &mut Model, kind, name: &str, x: f64, y: f64, height: f64| {
            m.add_device(NewDevice {
                level: l0,
                kind,
                name: name.into(),
                position: Point2::new(x, y),
                rotation: 0.0,
                elevation: if kind == DeviceKind::Riser { 0.0 } else { 0.3 },
                width: 0.3,
                depth: 0.2,
                height,
            })
            .unwrap();
        };
        device(&mut m, DeviceKind::WaterMeter, "Contador", 0.0, 0.0, 0.4);
        device(&mut m, DeviceKind::WaterHeater, "Termo", 0.0, -1.0, 0.8);
        device(&mut m, DeviceKind::Riser, "Montante AF", 1.0, 0.0, 3.0);
        device(&mut m, DeviceKind::Riser, "Montante ACS", 1.0, -0.2, 3.0);
        let run = |m: &mut Model, level, system, path: Vec<Point2>| {
            m.add_run(NewRun {
                level,
                system,
                path,
                elevation: 0.3,
                width: 0.016,
                height: 0.016,
                slope: 0.0,
            })
            .unwrap();
        };
        let cold = ServiceSystem::ColdWater;
        let hot = ServiceSystem::HotWater;
        run(
            &mut m,
            l0,
            cold,
            vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)],
        );
        run(
            &mut m,
            l0,
            cold,
            vec![Point2::new(0.0, 0.0), Point2::new(0.0, -1.0)],
        );
        run(
            &mut m,
            l0,
            hot,
            vec![
                Point2::new(0.0, -1.0),
                Point2::new(1.0, -1.0),
                Point2::new(1.0, -0.2),
            ],
        );
        for l in [l0, l1] {
            let ps: Vec<(Point2, bool)> = m
                .appliances_on(l)
                .iter()
                .map(|a| (a.point, a.kind.hot()))
                .collect();
            run(
                &mut m,
                l,
                cold,
                vec![Point2::new(1.0, 0.0), Point2::new(6.0, 0.0)],
            );
            run(
                &mut m,
                l,
                hot,
                vec![Point2::new(1.0, -0.2), Point2::new(6.0, -0.2)],
            );
            for (p, takes_hot) in ps {
                run(&mut m, l, cold, vec![Point2::new(p.x, 0.0), p]);
                if takes_hot {
                    let q = Point2::new(p.x + 0.05, p.y);
                    run(&mut m, l, hot, vec![Point2::new(q.x, -0.2), q]);
                }
            }
        }
        m
    }

    #[test]
    fn a_house_draws_its_plumbing() {
        let m = house();
        let w = wa::water(&m);
        assert!(!w.networks.is_empty());
        for net in &w.networks {
            for (i, st) in net.stretches.iter().enumerate() {
                if let Some(p) = st.parent {
                    assert!(p < i, "a stretch hangs from an earlier one");
                }
            }
        }
        let pages = pages(&m);
        assert!(pages.len() >= 2, "the vertical diagram and a level");
        assert!(pages.iter().all(|p| p.starts_with("<svg")));
        assert!(
            pages[0].contains("stroke-dasharray=\"2.2 1.1\""),
            "hot water"
        );
        if let Ok(dir) = std::env::var("ARCHISIM_FONTANERIA_OUT") {
            std::fs::create_dir_all(&dir).unwrap();
            for (i, p) in pages.iter().enumerate() {
                std::fs::write(format!("{dir}/fontaneria-{:02}.svg", i + 1), p).unwrap();
            }
        }
    }

    #[test]
    fn a_house_draws_its_drainage() {
        let mut m = house();
        let l0 = m.levels()[0].id;
        let l1 = m.levels()[1].id;
        m.add_device(NewDevice {
            level: l0,
            kind: DeviceKind::Downpipe,
            name: "Bajante".into(),
            position: Point2::new(1.0, 1.5),
            rotation: 0.0,
            elevation: 0.0,
            width: 0.11,
            depth: 0.11,
            height: 3.0,
        })
        .unwrap();
        let d = ServiceSystem::Drainage;
        for l in [l0, l1] {
            let ps: Vec<Point2> = m.appliances_on(l).iter().map(|a| a.point).collect();
            m.add_run(NewRun {
                level: l,
                system: d,
                path: vec![Point2::new(6.0, 1.5), Point2::new(1.0, 1.5)],
                elevation: 0.4,
                width: 0.11,
                height: 0.11,
                slope: 0.02,
            })
            .unwrap();
            for p in ps {
                m.add_run(NewRun {
                    level: l,
                    system: d,
                    path: vec![p, Point2::new(p.x, 1.5)],
                    elevation: 0.45,
                    width: 0.04,
                    height: 0.04,
                    slope: 0.02,
                })
                .unwrap();
            }
        }
        m.add_run(NewRun {
            level: l0,
            system: d,
            path: vec![Point2::new(1.0, 1.5), Point2::new(1.0, -3.0)],
            elevation: -0.5,
            width: 0.11,
            height: 0.11,
            slope: 0.02,
        })
        .unwrap();
        let pages = drainage_pages(&m);
        assert!(pages.len() >= 2, "the vertical diagram and a level");
        if let Ok(dir) = std::env::var("ARCHISIM_FONTANERIA_OUT") {
            std::fs::create_dir_all(&dir).unwrap();
            for (i, p) in pages.iter().enumerate() {
                std::fs::write(format!("{dir}/saneamiento-{:02}.svg", i + 1), p).unwrap();
            }
        }
    }

    #[test]
    fn no_water_no_pages() {
        let m = Model::new("Vacío");
        assert!(pages(&m).is_empty());
    }
}
