//! False ceilings (M7): `IfcCovering` `.CEILING.` (or whose type is an
//! `IfcCoveringType` `.CEILING.`), from ArchiSim or from any program. Its
//! outline is that of its body (the largest vertical extrusion, else its
//! lowest horizontal face), its face is the bottom of its body, and its
//! layers those of its material layer set, as with slabs.

use archisim_core::{Layer, LayerFunction, NewCeiling, SlabFunction};

use super::{area, clean_outline, explain, geometry, snap, Importer};

impl Importer<'_> {
    /// Whether an `IfcCovering` is a false ceiling.
    pub(super) fn is_ceiling(&self, id: u64) -> bool {
        let Some(e) = self.ent(id) else {
            return false;
        };
        if e.arg(8).as_enum() == Some("CEILING") {
            return true;
        }
        self.ix
            .type_of
            .get(&id)
            .and_then(|t| self.ent(*t))
            .is_some_and(|t| t.kind == "IFCCOVERINGTYPE" && t.arg(9).as_enum() == Some("CEILING"))
    }

    pub(super) fn import_ceilings(&mut self) {
        let mut ids = self.file.of_kind("IFCCOVERING");
        ids.sort_unstable();
        for id in ids {
            if !self.is_ceiling(id) || self.pool_parts.contains(&id) {
                continue;
            }
            match self.import_ceiling(id) {
                Ok(()) => self.ceiling_ids.insert(id),
                Err(why) => {
                    self.report.skip("techos", why);
                    self.ceiling_ids.insert(id)
                }
            };
        }
    }

    fn import_ceiling(&mut self, id: u64) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let (_, body) = self.shape(e, &xf);
        if body.is_empty() {
            return Err("sin geometría que ArchiSim sepa leer".into());
        }
        let (outline, holes, bottom, top) = if let Some(x) = body
            .extrusions
            .iter()
            .max_by(|a, b| area(&a.outline).total_cmp(&area(&b.outline)))
        {
            (x.outline.clone(), x.holes.clone(), x.bottom, x.top)
        } else {
            let (lo, hi) = body.bounds().ok_or("sin geometría")?;
            match geometry::top_outline(&body, hi[2]) {
                Some((outline, holes)) => (outline, holes, lo[2], hi[2]),
                None => return Err("techo que no es plano (aún no se importa)".into()),
            }
        };
        let outline = clean_outline(&outline);
        if outline.len() < 3 {
            return Err("contorno sin superficie".into());
        }
        let holes: Vec<_> = holes
            .iter()
            .map(|h| clean_outline(h))
            .filter(|h| h.len() >= 3)
            .collect();
        let body_thickness = snap(top - bottom);

        let set = self.layer_set(id);
        let mut type_name = String::new();
        let layers: Vec<Layer> = match set.filter(|s| !s.layers.is_empty()) {
            Some(set) => {
                type_name = set.name.clone();
                let positive = set.usage.is_none_or(|(p, _)| p);
                let mut v: Vec<Layer> = set
                    .layers
                    .iter()
                    .map(|&(material, thickness, function)| Layer {
                        material,
                        thickness,
                        function,
                    })
                    .collect();
                // Written from the bottom up: ArchiSim keeps them top down.
                if positive {
                    v.reverse();
                }
                v
            }
            None => {
                if body_thickness < 1e-3 {
                    return Err("techo sin espesor".into());
                }
                self.report.warn(
                    "Techos sin capas de material: se importan con una sola capa de su espesor.",
                );
                vec![Layer {
                    material: self.single_material(id),
                    thickness: body_thickness,
                    function: LayerFunction::Finish,
                }]
            }
        };
        let ifc_type = self
            .ix
            .type_of
            .get(&id)
            .copied()
            .filter(|t| self.ent(*t).is_some_and(|t| t.kind == "IFCCOVERINGTYPE"));
        if let Some(t) = ifc_type.and_then(|t| self.ent(t)) {
            if let Some(name) = t.arg(2).as_str().filter(|s| !s.trim().is_empty()) {
                type_name = name.to_string();
            }
        }
        if type_name.is_empty() {
            type_name = e
                .arg(4)
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(String::from)
                .unwrap_or_else(|| "Falso techo".into());
        }
        let ceiling_type = self
            .slab_type(ifc_type, &type_name, SlabFunction::Ceiling, layers)
            .map_err(|e| format!("tipo no válido ({})", explain(&e)))?;
        let bottom = snap(bottom);
        // The level of the room it is the ceiling of: the highest one at
        // or under its face (Revit hangs it from the level above, with a
        // negative offset), else the one the file puts it in.
        let level = self
            .levels
            .iter()
            .rev()
            .find(|(_, z)| *z <= bottom + 1e-6)
            .map(|(l, _)| *l)
            .unwrap_or_else(|| self.level_of(id, bottom));
        let elevation = self.elevation(level);
        let cid = self
            .model
            .add_ceiling(NewCeiling {
                ceiling_type,
                level,
                outline,
                height: snap(bottom - elevation),
            })
            .map_err(|e| explain(&e))?;
        if !holes.is_empty() && self.model.edit_ceiling(cid, |c| c.holes = holes).is_err() {
            self.report
                .warn("Huecos de techos que no caben en su contorno: se importan cerrados.");
        }
        self.report.ceilings += 1;
        self.sources.push((cid.0, id));
        if let Some(g) = e.arg(0).as_str() {
            self.guids.ceilings.push((cid, g.to_string()));
        }
        Ok(())
    }
}
