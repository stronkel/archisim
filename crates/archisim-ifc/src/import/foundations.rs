//! Footings (`IfcFooting`, from any program: pads, strips and tie beams
//! from the box of their extruded body) and setting-out grids (`IfcGrid`,
//! a grid line for each straight axis).

use archisim_core::{FootingKind, NewFooting, NewGrid, Point2};

use super::{explain, snap, Importer};

impl<'a> Importer<'a> {
    pub(super) fn import_footings(&mut self) {
        let mut ids = self.file.of_kind("IFCFOOTING");
        ids.sort_unstable();
        for id in ids {
            if let Err(why) = self.import_footing(id) {
                self.report.skip("cimentación", why);
            }
        }
    }

    fn import_footing(&mut self, id: u64) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let (_, body) = self.shape(e, &xf);
        let ex = body
            .extrusions
            .first()
            .ok_or("sin un sólido extruido en vertical que ArchiSim sepa leer")?;
        let mut pts: Vec<Point2> = ex
            .outline
            .iter()
            .map(|p| Point2::new(snap(p[0]), snap(p[1])))
            .collect();
        if pts.len() > 4 && pts.first() == pts.last() {
            pts.pop();
        }
        if pts.len() != 4 {
            return Err("cimentación que no es rectangular (aún no se importa)".into());
        }
        let level = self.level_of(id, ex.bottom);
        let top_offset = snap(ex.top - self.elevation(level));
        let depth = snap(ex.top - ex.bottom);
        let (e0, e1) = (pts[1] - pts[0], pts[2] - pts[1]);
        let predefined = e.arg(8).as_enum().unwrap_or("").to_ascii_uppercase();
        let mid =
            |a: Point2, b: Point2| Point2::new(snap((a.x + b.x) / 2.0), snap((a.y + b.y) / 2.0));
        let along = |pts: &[Point2]| {
            if e0.length() >= e1.length() {
                (mid(pts[0], pts[3]), mid(pts[1], pts[2]), snap(e1.length()))
            } else {
                (mid(pts[0], pts[1]), mid(pts[3], pts[2]), snap(e0.length()))
            }
        };
        let (kind, width) = match predefined.as_str() {
            "STRIP_FOOTING" => {
                let (start, end, width) = along(&pts);
                let wall = self
                    .property_own(id, "ArchiSim_Footing", "Wall")
                    .and_then(|v| v.as_str())
                    .and_then(|g| {
                        self.guids
                            .walls
                            .iter()
                            .find(|(_, s)| s == g)
                            .map(|(w, _)| *w)
                    });
                (FootingKind::Strip { start, end, wall }, width)
            }
            "FOOTING_BEAM" => {
                let (start, end, width) = along(&pts);
                (FootingKind::TieBeam { start, end }, width)
            }
            _ => {
                let c = Point2::new(
                    snap((pts[0].x + pts[2].x) / 2.0),
                    snap((pts[0].y + pts[2].y) / 2.0),
                );
                let column = self
                    .property_own(id, "ArchiSim_Footing", "Column")
                    .and_then(|v| v.as_str())
                    .and_then(|g| {
                        self.guids
                            .columns
                            .iter()
                            .find(|(_, s)| s == g)
                            .map(|(c, _)| *c)
                    });
                (
                    FootingKind::Pad {
                        position: c,
                        length: snap(e1.length()),
                        rotation: e0.y.atan2(e0.x),
                        column,
                    },
                    snap(e0.length()),
                )
            }
        };
        let material = self.single_material(id);
        let fid = self
            .model
            .add_footing(NewFooting {
                level,
                kind,
                material,
                width,
                depth,
                top_offset,
            })
            .map_err(|e| explain(&e))?;
        // Off its column, and its piles, if ArchiSim wrote them.
        let real = |v: &crate::step_read::Value| {
            v.as_f64()
                .or_else(|| v.as_list().first().and_then(|x| x.as_f64()))
        };
        let ox = self
            .property_own(id, "ArchiSim_Footing", "OffsetX")
            .and_then(real);
        let oy = self
            .property_own(id, "ArchiSim_Footing", "OffsetY")
            .and_then(real);
        if ox.is_some() || oy.is_some() {
            let _ = self.model.set_footing_offset(
                fid,
                Some(archisim_core::Vec2::new(
                    ox.unwrap_or(0.0),
                    oy.unwrap_or(0.0),
                )),
            );
        }
        if let Some(t) = self
            .property_own(id, "ArchiSim_Footing", "Piles")
            .and_then(|v| {
                v.as_str()
                    .or_else(|| v.as_list().first().and_then(|x| x.as_str()))
            })
        {
            let n: Vec<f64> = t.split(';').filter_map(|x| x.trim().parse().ok()).collect();
            if n.len() == 4 {
                let _ = self.model.set_footing_piles(
                    fid,
                    Some(archisim_core::Piles {
                        count: n[0] as u32,
                        diameter: n[1],
                        length: n[2],
                        capacity: n[3],
                    }),
                );
            }
        }
        self.report.footings += 1;
        self.sources.push((fid.0, id));
        self.guids
            .footings
            .push((fid, e.arg(0).as_str().unwrap_or("").to_string()));
        Ok(())
    }

    pub(super) fn import_grids(&mut self) {
        let mut ids = self.file.of_kind("IFCGRID");
        ids.sort_unstable();
        // Some programs write the same grid once per storey: each axis is
        // brought once (same name, same ends either way round, to the mm).
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            let Some(e) = self.ent(id) else { continue };
            let xf = self.geo.placement(e.arg(5));
            for list in [e.arg(7), e.arg(8), e.arg(9)] {
                for axis in list.as_list() {
                    let Some(a) = self.deref(axis) else { continue };
                    let name = a.arg(0).as_str().unwrap_or("").to_string();
                    if name == "-" {
                        // The axis written only because IFC asks for two lists.
                        continue;
                    }
                    let mut curved = false;
                    let Some(pts) = self.geo.curve(a.arg(1), &mut curved) else {
                        self.report
                            .skip("ejes", "eje sin línea que ArchiSim sepa leer");
                        continue;
                    };
                    let (Some(&p), Some(&q)) = (pts.first(), pts.last()) else {
                        continue;
                    };
                    let (p, q) = (xf.apply(p), xf.apply(q));
                    let mm = |v: f64| (v * 1000.0).round() as i64;
                    let (a, b) = ((mm(p[0]), mm(p[1])), (mm(q[0]), mm(q[1])));
                    if !seen.insert((name.clone(), a.min(b), a.max(b))) {
                        self.report.warn(
                            "Ejes repetidos en el fichero (uno por planta): se importan una vez.",
                        );
                        continue;
                    }
                    let r = self.model.add_grid(NewGrid {
                        name,
                        start: Point2::new(snap(p[0]), snap(p[1])),
                        end: Point2::new(snap(q[0]), snap(q[1])),
                    });
                    match r {
                        Ok(_) => self.report.grids += 1,
                        Err(e) => self.report.skip("ejes", explain(&e)),
                    }
                }
            }
        }
    }
}
