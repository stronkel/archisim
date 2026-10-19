//! Curtain walls written by ArchiSim: an `IfcCurtainWall` with the
//! `ArchiSim_CurtainWall` property set (line, top, height) is read back as
//! one curtain wall, of the type made from its `IfcCurtainWallType` and
//! the `ArchiSim_CurtainWallType` set (grid, frames, panels, extras, with
//! materials by name). Every such type is read, used or not. Curtain walls
//! from other programs are not imported yet.

use archisim_core::{
    CurtainCaps, CurtainDesign, CurtainDivision, CurtainJoint, CurtainLouvres, CurtainPanel,
    CurtainProfile, CurtainRest, CurtainTypeId, MaterialId, NewCurtainWall, Point2, WallTop,
};

use super::{explain, snap, snap2, Importer};

/// A division from its text ("1 0.6 max 1.2", "count 3"), lengths scaled
/// by `len`.
pub(super) fn parse_division(text: &str, len: f64) -> Option<CurtainDivision> {
    let mut fixed = Vec::new();
    let mut words = text.split_whitespace();
    while let Some(w) = words.next() {
        match w {
            "max" => {
                let v: f64 = words.next()?.parse().ok()?;
                return Some(CurtainDivision {
                    fixed,
                    rest: CurtainRest::MaxSpacing(snap(v * len)),
                });
            }
            "count" => {
                let n: usize = words.next()?.parse().ok()?;
                return Some(CurtainDivision {
                    fixed,
                    rest: CurtainRest::Count(n),
                });
            }
            v => fixed.push(snap(v.parse::<f64>().ok()? * len)),
        }
    }
    None
}

/// Numbers, then a material name after `|` ("0.05 0.15|Aluminio"), and
/// whatever follows a second `|`.
fn numbers_and_name(text: &str, len: f64) -> Option<(Vec<f64>, String, String)> {
    let mut parts = text.splitn(3, '|');
    let numbers = parts
        .next()?
        .split_whitespace()
        .map(|v| v.parse::<f64>().ok().map(|v| snap(v * len)))
        .collect::<Option<Vec<f64>>>()?;
    let name = parts.next().unwrap_or("").to_string();
    let rest = parts.next().unwrap_or("").to_string();
    Some((numbers, name, rest))
}

impl<'a> Importer<'a> {
    pub(super) fn import_curtain_walls(&mut self) {
        if !self.own_file {
            return;
        }
        let mut types = self.file.of_kind("IFCCURTAINWALLTYPE");
        types.sort_unstable();
        for t in types {
            if let Err(why) = self.import_curtain_type(t) {
                self.report.skip("tipos de muro cortina", why);
            }
        }
        let mut ids = self.file.of_kind("IFCCURTAINWALL");
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_curtain_wall(id) {
                self.report.skip("muros cortina", why);
            }
        }
    }

    /// A material of the model by name (those of the curtain walls' parts
    /// are read first), or a stand-in.
    fn curtain_material(&mut self, name: &str, fallback: &str) -> MaterialId {
        let found = self
            .model
            .materials()
            .find(|m| m.name == name)
            .map(|m| m.id);
        match found {
            Some(m) => m,
            None => self.fallback_material(fallback),
        }
    }

    fn import_curtain_type(&mut self, t: u64) -> Result<(), String> {
        let te = self.ent(t).ok_or("no existe")?;
        let props: Vec<(String, String)> = te
            .arg(5)
            .as_list()
            .iter()
            .filter_map(|p| self.deref(p))
            .filter(|p| {
                p.kind == "IFCPROPERTYSET" && p.arg(2).as_str() == Some("ArchiSim_CurtainWallType")
            })
            .flat_map(|set| set.arg(4).as_list().iter().filter_map(|p| self.deref(p)))
            .filter(|p| p.kind == "IFCPROPERTYSINGLEVALUE")
            .filter_map(|p| {
                Some((
                    p.arg(0).as_str()?.to_string(),
                    p.arg(2).as_str()?.to_string(),
                ))
            })
            .collect();
        if props.is_empty() {
            return Ok(());
        }
        let name = te
            .arg(2)
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Muro cortina")
            .to_string();
        // The materials of the parts of every curtain wall, so the type's
        // can be found by name.
        let parts: Vec<u64> = self
            .ix
            .parent
            .iter()
            .filter(|(_, &p)| self.ent(p).is_some_and(|e| e.kind == "IFCCURTAINWALL"))
            .map(|(&c, _)| c)
            .collect();
        for p in parts {
            if self.ix.material.contains_key(&p) {
                self.single_material(p);
            }
        }
        let len = self.geo.length;
        let get = |k: &str| props.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        let need = |k: &str| get(k).ok_or_else(|| format!("falta {k}"));
        let division = |k: &str| {
            need(k).and_then(|v| parse_division(v, len).ok_or_else(|| format!("{k} ilegible")))
        };
        let (columns, rows) = (division("Columns")?, division("Rows")?);
        let mut profile = |k: &str| -> Result<CurtainProfile, String> {
            let (n, m, _) = numbers_and_name(need(k)?, len).ok_or(format!("{k} ilegible"))?;
            let [width, depth] = n[..] else {
                return Err(format!("{k} ilegible"));
            };
            Ok(CurtainProfile {
                width,
                depth,
                material: self.curtain_material(&m, "frame"),
            })
        };
        let (border, mullion, transom) =
            (profile("Border")?, profile("Mullion")?, profile("Transom")?);
        let mut panels = Vec::new();
        for i in 1.. {
            let Some(text) = get(&format!("Panel{i}")) else {
                break;
            };
            let (n, m, panel_name) =
                numbers_and_name(text, len).ok_or(format!("Panel{i} ilegible"))?;
            let [thickness] = n[..] else {
                return Err(format!("Panel{i} ilegible"));
            };
            panels.push(CurtainPanel {
                name: panel_name,
                thickness,
                material: self.curtain_material(&m, "glass"),
            });
        }
        let row_panels = get("RowPanels")
            .map(|t| {
                t.split_whitespace()
                    .map(|v| {
                        v.parse::<usize>()
                            .map_err(|_| "RowPanels ilegible".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let caps = match get("Caps") {
            Some(t) => {
                let (n, m, _) = numbers_and_name(t, len).ok_or("Caps ilegible")?;
                let [width, depth] = n[..] else {
                    return Err("Caps ilegible".into());
                };
                Some(CurtainCaps {
                    width,
                    depth,
                    material: self.curtain_material(&m, "frame"),
                })
            }
            None => None,
        };
        let louvres = match get("Louvres") {
            Some(t) => {
                let (n, m, _) = numbers_and_name(t, len).ok_or("Louvres ilegible")?;
                let [depth, thickness, spacing, offset] = n[..] else {
                    return Err("Louvres ilegible".into());
                };
                Some(CurtainLouvres {
                    depth,
                    thickness,
                    spacing,
                    offset,
                    material: self.curtain_material(&m, "frame"),
                })
            }
            None => None,
        };
        let design = CurtainDesign {
            columns,
            rows,
            border,
            mullion,
            transom,
            joint: if get("Joint") == Some("transoms") {
                CurtainJoint::Transoms
            } else {
                CurtainJoint::Mullions
            },
            panels,
            row_panels,
            caps,
            louvres,
        };
        let id = self
            .model
            .add_curtain_type(name, design)
            .map_err(|e| explain(&e))?;
        self.curtain_types.insert(t, id);
        if let Some(g) = self.type_guid(Some(t)) {
            self.guids.curtain_types.push((id, g));
        }
        Ok(())
    }

    fn import_curtain_wall(&mut self, id: u64) -> Result<(), String> {
        const PSET: &str = "ArchiSim_CurtainWall";
        let text = |name: &str| {
            self.property_own(id, PSET, name)
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        let Some(line) = text("Line") else {
            return Ok(());
        };
        let len = self.geo.length;
        let number = |name: &str| {
            self.property_own(id, PSET, name)
                .and_then(|v| v.as_f64())
                .map(|v| snap(v * len))
                .ok_or_else(|| format!("falta {name}"))
        };
        let [base_offset, height] = [number("BaseOffset")?, number("Height")?];
        let curtain_type: CurtainTypeId = self
            .ix
            .type_of
            .get(&id)
            .and_then(|t| self.curtain_types.get(t))
            .copied()
            .ok_or("sin tipo de muro cortina")?;
        let ends: Option<Vec<Point2>> = line
            .split(';')
            .map(|p| {
                let mut it = p.split_whitespace().map(|v| v.parse::<f64>().ok());
                match (it.next().flatten(), it.next().flatten()) {
                    (Some(x), Some(y)) => Some(snap2(Point2::new(x * len, y * len))),
                    _ => None,
                }
            })
            .collect();
        let ends = ends.ok_or("línea ilegible")?;
        let [start, end] = ends[..] else {
            return Err("línea ilegible".into());
        };
        let top = match text("Top") {
            Some(t) if !t.trim().is_empty() => {
                let mut it = t.split_whitespace();
                let guid = it.next().unwrap_or_default().to_string();
                let offset = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) * len;
                let storey = self
                    .file
                    .of_kind("IFCBUILDINGSTOREY")
                    .into_iter()
                    .find(|s| self.ent(*s).and_then(|e| e.arg(0).as_str()) == Some(guid.as_str()));
                match storey.and_then(|s| self.storeys.get(&s)).copied() {
                    Some(level) => WallTop::Level {
                        level,
                        offset: snap(offset),
                    },
                    None => WallTop::Unconnected,
                }
            }
            _ => WallTop::Unconnected,
        };
        let level = self.level_of(id, 0.0);
        let cid = self
            .model
            .add_curtain_wall(NewCurtainWall {
                curtain_type,
                level,
                start,
                end,
                base_offset,
                top,
                height,
            })
            .map_err(|e| explain(&e))?;
        self.curtain_ids.insert(id);
        self.report.curtain_walls += 1;
        self.sources.push((cid.0, id));
        let e = self.ent(id).ok_or("no existe")?;
        self.guids
            .curtain_walls
            .push((cid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }
}
