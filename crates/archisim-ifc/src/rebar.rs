//! The reinforcement (E11): each group of bars of an element («4Ø16»,
//! «cercos Ø8 c/15») is an `IfcReinforcingBar` (`MAIN`, `LIGATURE`…)
//! whose body is a swept disk along each bar, part of its column, beam,
//! footing or slab (`IfcRelAggregates`), with `Pset_ReinforcingBarCommon`
//! (`NominalDiameter`, `BarLength`) and `ArchiSim_Rebar` (what it is and
//! how many), so a file read back does not take the bars for elements of
//! their own. What the designer wrote goes on the element itself, in
//! `ArchiSim_Reinforcement`, and is read back.

use std::collections::BTreeMap;

use archisim_core::{LevelId, MaterialId, Model, Reinforcement};
use archisim_geom::rebar::BarRole;

use crate::step::{real, string, Ref};
use crate::{derived_guid, Exporter};

impl Exporter<'_> {
    pub(crate) fn write_rebar(
        &mut self,
        materials: &BTreeMap<MaterialId, Ref>,
        storeys: &BTreeMap<LevelId, (Ref, Ref)>,
    ) {
        let model = self.model;
        let list: Vec<(u64, Reinforcement)> =
            model.reinforcements().map(|(id, r)| (id, *r)).collect();
        let steel = model.rebar_material();
        for (id, r) in list {
            let Some(&host) = self.hosts.get(&id) else {
                continue;
            };
            let Some((guid, level)) = guid_and_level(model, id) else {
                continue;
            };
            let Some(&(_, storey_pl)) = storeys.get(&level) else {
                continue;
            };
            let storey_z = model.level(level).map_or(0.0, |l| l.elevation);
            // What the designer wrote, to read it back.
            let text = encode(&r);
            let p = self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Reinforcement',$,IFCTEXT({}),$)",
                string(&text)
            ));
            let summary = self.w.add(format!(
                "IFCPROPERTYSINGLEVALUE('Summary',$,IFCLABEL({}),$)",
                string(&r.summary())
            ));
            self.pset(host, &guid, "ArchiSim_Reinforcement", &[p, summary]);
            let mut parts = Vec::new();
            for (k, g) in archisim_geom::rebar::element_bars(model, id)
                .into_iter()
                .enumerate()
            {
                if g.bars.is_empty() {
                    continue;
                }
                let radius = g.diameter / 2.0;
                let mut items = Vec::new();
                for bar in &g.bars {
                    let pts: Vec<String> = bar
                        .iter()
                        .map(|p| {
                            self.w
                                .add(format!(
                                    "IFCCARTESIANPOINT(({},{},{}))",
                                    real(p[0]),
                                    real(p[1]),
                                    real(p[2] - storey_z)
                                ))
                                .to_string()
                        })
                        .collect();
                    let line = self.w.add(format!("IFCPOLYLINE(({}))", pts.join(",")));
                    items.push(
                        self.w
                            .add(format!("IFCSWEPTDISKSOLID({line},{},$,$,$)", real(radius)))
                            .to_string(),
                    );
                }
                let body = items.join(",");
                if let Some(&style) = steel.and_then(|m| self.styles.get(&m)) {
                    for item in &items {
                        self.w.add(format!("IFCSTYLEDITEM({item},({style}),$)"));
                    }
                }
                let rep = self.w.add(format!(
                    "IFCSHAPEREPRESENTATION({},'Body','AdvancedSweptSolid',({body}))",
                    self.body_ctx
                ));
                let shape = self
                    .w
                    .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,({rep}))"));
                let pl = self.placement(Some(storey_pl), self.origin, None);
                let predefined = match g.role {
                    BarRole::Main | BarRole::Mesh => "MAIN",
                    BarRole::Tie => "LIGATURE",
                };
                let bar_guid = derived_guid(&guid, &format!("rebar {k}"));
                let n = g.bars.len();
                let el = self.w.add(format!(
                    "IFCREINFORCINGBAR({},$,{},$,{},{pl},{shape},$,$,$,$,$,.{predefined}.,.TEXTURED.)",
                    string(bar_guid.as_str()),
                    string(&format!("{} · {}", g.label, g.place)),
                    string(&g.place),
                ));
                if let Some(m) = steel.and_then(|m| materials.get(&m)) {
                    self.w.add(format!(
                        "IFCRELASSOCIATESMATERIAL({},$,$,$,({el}),{m})",
                        string(derived_guid(&bar_guid, "material").as_str())
                    ));
                }
                let common = vec![
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('NominalDiameter',$,IFCPOSITIVELENGTHMEASURE({}),$)",
                        real(g.diameter)
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('BarLength',$,IFCPOSITIVELENGTHMEASURE({}),$)",
                        real(g.length() / n as f64)
                    )),
                ];
                self.pset(el, &bar_guid, "Pset_ReinforcingBarCommon", &common);
                let own = vec![
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Bars',$,IFCCOUNTMEASURE({n}),$)"
                    )),
                    self.w.add(format!(
                        "IFCPROPERTYSINGLEVALUE('Weight',$,IFCMASSMEASURE({}),$)",
                        real(g.weight())
                    )),
                ];
                self.pset(el, &bar_guid, "ArchiSim_Rebar", &own);
                parts.push(el.to_string());
            }
            if !parts.is_empty() {
                self.w.add(format!(
                    "IFCRELAGGREGATES({},$,'Armado',$,{host},({}))",
                    string(derived_guid(&guid, "rebar").as_str()),
                    parts.join(",")
                ));
            }
        }
    }
}

fn guid_and_level(model: &Model, id: u64) -> Option<(archisim_core::GlobalId, LevelId)> {
    use archisim_core::{BeamId, ColumnId, FootingId, SlabId};
    if let Some(c) = model.column(ColumnId(id)) {
        return Some((c.guid.clone(), c.level));
    }
    if let Some(b) = model.beam(BeamId(id)) {
        return Some((b.guid.clone(), b.level));
    }
    if let Some(f) = model.footing(FootingId(id)) {
        return Some((f.guid.clone(), f.level));
    }
    model.slab(SlabId(id)).map(|s| (s.guid.clone(), s.level))
}

/// What the designer wrote, as text: `kind|cover|…` with diameters and
/// spacings in metres.
pub(crate) fn encode(r: &Reinforcement) -> String {
    let c = |c: &archisim_core::Counted| format!("{} {}", c.count, real(c.diameter));
    let s = |s: &archisim_core::Spaced| format!("{} {}", real(s.diameter), real(s.spacing));
    let mesh = |m: &[archisim_core::Spaced; 2]| format!("{};{}", s(&m[0]), s(&m[1]));
    match r {
        Reinforcement::Column {
            cover,
            bars,
            ties,
            dense,
        } => format!(
            "column|{}|{}|{}|{}",
            real(*cover),
            c(bars),
            s(ties),
            dense.map_or(String::new(), |(a, b)| format!("{} {}", real(a), real(b)))
        ),
        Reinforcement::Beam {
            cover,
            bottom,
            top,
            stirrups,
        } => format!(
            "beam|{}|{}|{}|{}",
            real(*cover),
            c(bottom),
            c(top),
            s(stirrups)
        ),
        Reinforcement::Footing { cover, bottom, top } => format!(
            "footing|{}|{}|{}",
            real(*cover),
            mesh(bottom),
            top.as_ref().map_or(String::new(), mesh)
        ),
        Reinforcement::Slab { cover, bottom, top } => format!(
            "slab|{}|{}|{}",
            real(*cover),
            mesh(bottom),
            top.as_ref().map_or(String::new(), mesh)
        ),
    }
}

/// [`encode`] read back.
pub(crate) fn decode(text: &str) -> Option<Reinforcement> {
    let parts: Vec<&str> = text.split('|').collect();
    let f = |t: &str| t.parse::<f64>().ok();
    let pair = |t: &str| -> Option<(f64, f64)> {
        let mut it = t.split_whitespace();
        Some((f(it.next()?)?, f(it.next()?)?))
    };
    let counted = |t: &str| pair(t).map(|(n, d)| archisim_core::Counted::new(n as u32, d));
    let spaced = |t: &str| pair(t).map(|(d, s)| archisim_core::Spaced::new(d, s));
    let mesh = |t: &str| -> Option<[archisim_core::Spaced; 2]> {
        let (a, b) = t.split_once(';')?;
        Some([spaced(a)?, spaced(b)?])
    };
    let cover = f(parts.get(1)?)?;
    Some(match *parts.first()? {
        "column" => Reinforcement::Column {
            cover,
            bars: counted(parts.get(2)?)?,
            ties: spaced(parts.get(3)?)?,
            dense: parts.get(4).and_then(|t| pair(t)),
        },
        "beam" => Reinforcement::Beam {
            cover,
            bottom: counted(parts.get(2)?)?,
            top: counted(parts.get(3)?)?,
            stirrups: spaced(parts.get(4)?)?,
        },
        "footing" => Reinforcement::Footing {
            cover,
            bottom: mesh(parts.get(2)?)?,
            top: parts.get(3).and_then(|t| mesh(t)),
        },
        "slab" => Reinforcement::Slab {
            cover,
            bottom: mesh(parts.get(2)?)?,
            top: parts.get(3).and_then(|t| mesh(t)),
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::RebarKind;

    #[test]
    fn written_and_read_back() {
        for k in [
            RebarKind::Column,
            RebarKind::Beam,
            RebarKind::Footing,
            RebarKind::Slab,
        ] {
            let r = Reinforcement::default_for(k);
            assert_eq!(decode(&encode(&r)), Some(r));
        }
    }
}
