//! Columns and beams (`IfcColumn`, `IfcBeam`).
//!
//! The section comes, in this order, from the material profile set of the
//! element or its type (how ArchiSim, Revit and ArchiCAD write it), from
//! the profile its body is extruded from, or from the box of its body
//! (then as a rectangle, with a warning). The position and size come from
//! the body itself, measured in the element's own axes: a column is the
//! vertical one, a beam runs along its longest horizontal axis.

use archisim_core::{
    MemberTypeId, MemberUsage, ModelError, NewBeam, NewColumn, Point2, Profile, WallTop,
};

use super::geometry::{dot, sub, P3};
use super::{explain, snap, snap2, Importer};
use crate::step_read::{Entity, Value};

/// Key of an imported member type: IFC type, usage, section and material.
pub(super) type MemberKey = (Option<u64>, bool, [i64; 4], u8, u64);

fn mm(v: f64) -> i64 {
    (v * 1e4).round() as i64
}

fn key_of(p: Profile) -> ([i64; 4], u8) {
    match p {
        Profile::Rectangle { width, depth } => ([mm(width), mm(depth), 0, 0], 0),
        Profile::Circle { diameter } => ([mm(diameter), 0, 0, 0], 1),
        Profile::IShape {
            width,
            depth,
            web,
            flange,
        } => ([mm(width), mm(depth), mm(web), mm(flange)], 2),
        Profile::UShape {
            width,
            depth,
            web,
            flange,
        } => ([mm(width), mm(depth), mm(web), mm(flange)], 3),
        Profile::TShape {
            width,
            depth,
            web,
            flange,
        } => ([mm(width), mm(depth), mm(web), mm(flange)], 4),
        Profile::LShape {
            width,
            depth,
            thickness,
        } => ([mm(width), mm(depth), mm(thickness), 0], 5),
        Profile::RectHollow { width, depth, wall } => ([mm(width), mm(depth), mm(wall), 0], 6),
        Profile::CircleHollow { diameter, wall } => ([mm(diameter), mm(wall), 0, 0], 7),
    }
}

impl<'a> Importer<'a> {
    /// A section read from an IFC profile definition.
    fn profile_def(&self, e: &Entity) -> Option<Profile> {
        let len = self.geo.length;
        let f = |v: &Value| v.as_f64().map(|x| snap(x * len));
        match e.kind.as_str() {
            "IFCRECTANGLEPROFILEDEF" | "IFCROUNDEDRECTANGLEPROFILEDEF" => {
                Some(Profile::Rectangle {
                    width: f(e.arg(3))?,
                    depth: f(e.arg(4))?,
                })
            }
            "IFCCIRCLEPROFILEDEF" => Some(Profile::Circle {
                diameter: 2.0 * f(e.arg(3))?,
            }),
            "IFCISHAPEPROFILEDEF" => Some(Profile::IShape {
                width: f(e.arg(3))?,
                depth: f(e.arg(4))?,
                web: f(e.arg(5))?,
                flange: f(e.arg(6))?,
            }),
            "IFCRECTANGLEHOLLOWPROFILEDEF" => Some(Profile::RectHollow {
                width: f(e.arg(3))?,
                depth: f(e.arg(4))?,
                wall: f(e.arg(5))?,
            }),
            "IFCCIRCLEHOLLOWPROFILEDEF" => Some(Profile::CircleHollow {
                diameter: 2.0 * f(e.arg(3))?,
                wall: f(e.arg(4))?,
            }),
            "IFCUSHAPEPROFILEDEF" => Some(Profile::UShape {
                depth: f(e.arg(3))?,
                width: f(e.arg(4))?,
                web: f(e.arg(5))?,
                flange: f(e.arg(6))?,
            }),
            "IFCTSHAPEPROFILEDEF" => Some(Profile::TShape {
                depth: f(e.arg(3))?,
                width: f(e.arg(4))?,
                web: f(e.arg(5))?,
                flange: f(e.arg(6))?,
            }),
            "IFCLSHAPEPROFILEDEF" => Some(Profile::LShape {
                depth: f(e.arg(3))?,
                width: f(e.arg(4)).or(f(e.arg(3)))?,
                thickness: f(e.arg(5))?,
            }),
            _ => None,
        }
    }

    /// The profile and material of an element's material profile set (its
    /// own or its type's).
    fn profile_set(&self, element: u64) -> Option<(Option<Profile>, Option<u64>)> {
        let mut def = self.ent(self.material_of(element)?)?;
        if def.kind == "IFCMATERIALPROFILESETUSAGE" {
            def = self.deref(def.arg(0))?;
        }
        let profile = match def.kind.as_str() {
            "IFCMATERIALPROFILESET" => self.deref(def.arg(2).as_list().first()?)?,
            "IFCMATERIALPROFILE" => def,
            _ => return None,
        };
        let section = self.deref(profile.arg(3)).and_then(|p| self.profile_def(p));
        Some((section, profile.arg(2).as_ref()))
    }

    /// The section of the first extrusion of an element's body, if it is
    /// one ArchiSim knows (also inside mapped items).
    fn swept_profile(&self, e: &Entity) -> Option<Profile> {
        let shape = self.deref(e.arg(6))?;
        let mut stack: Vec<&Entity> = Vec::new();
        for rep in shape.arg(2).as_list() {
            let rep = self.deref(rep)?;
            if rep.arg(1).as_str() == Some("Body") {
                stack.extend(rep.arg(3).as_list().iter().filter_map(|i| self.deref(i)));
            }
        }
        let mut guard = 0;
        while let Some(item) = stack.pop() {
            guard += 1;
            if guard > 64 {
                break;
            }
            match item.kind.as_str() {
                "IFCEXTRUDEDAREASOLID" => {
                    if let Some(p) = self.deref(item.arg(0)).and_then(|p| self.profile_def(p)) {
                        return Some(p);
                    }
                }
                "IFCBOOLEANCLIPPINGRESULT" | "IFCBOOLEANRESULT" => {
                    stack.extend(self.deref(item.arg(1)));
                }
                "IFCMAPPEDITEM" => {
                    let map = self.deref(item.arg(0))?;
                    let rep = self.deref(map.arg(1))?;
                    stack.extend(rep.arg(3).as_list().iter().filter_map(|i| self.deref(i)));
                }
                _ => {}
            }
        }
        None
    }

    pub(super) fn import_members(&mut self) {
        for (kinds, usage) in [
            (
                &["IFCCOLUMN", "IFCCOLUMNSTANDARDCASE"][..],
                MemberUsage::Column,
            ),
            (&["IFCBEAM", "IFCBEAMSTANDARDCASE"][..], MemberUsage::Beam),
        ] {
            let mut ids: Vec<u64> = kinds.iter().flat_map(|k| self.file.of_kind(k)).collect();
            ids.sort_unstable();
            for id in ids {
                let what = match usage {
                    MemberUsage::Column => "pilares",
                    MemberUsage::Beam => "vigas",
                };
                if let Err(why) = self.import_member(id, usage) {
                    self.report.skip(what, why);
                }
            }
        }
    }

    fn import_member(&mut self, id: u64, usage: MemberUsage) -> Result<(), String> {
        let e = self.ent(id).ok_or("no existe")?;
        let xf = self.geo.placement(e.arg(5));
        let (_, body) = self.shape(e, &xf);
        if body.is_empty() {
            return Err("sin geometría que ArchiSim sepa leer".into());
        }
        // The body in the element's axes.
        let axes = [xf.x, xf.y, xf.z];
        let local = |p: P3| {
            let d = sub(p, xf.origin);
            [dot(d, axes[0]), dot(d, axes[1]), dot(d, axes[2])]
        };
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in body.all_points() {
            let q = local(p);
            for i in 0..3 {
                lo[i] = lo[i].min(q[i]);
                hi[i] = hi[i].max(q[i]);
            }
        }
        let range = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let centre = [
            (hi[0] + lo[0]) / 2.0,
            (hi[1] + lo[1]) / 2.0,
            (hi[2] + lo[2]) / 2.0,
        ];
        let world = |l: [f64; 3]| -> P3 {
            let mut p = xf.origin;
            for (i, axis) in axes.iter().enumerate() {
                for k in 0..3 {
                    p[k] += axis[k] * l[i];
                }
            }
            p
        };
        // Which local axis is vertical.
        let up = (0..3).find(|&i| axes[i][2].abs() > 1.0 - 1e-4);
        let Some(up) = up else {
            return Err(match usage {
                MemberUsage::Column => "pilar inclinado o girado (aún no se importa)".into(),
                MemberUsage::Beam => "viga inclinada o girada (aún no se importa)".into(),
            });
        };
        let (set_profile, set_material) = self.profile_set(id).unwrap_or((None, None));
        let section = set_profile.or_else(|| self.swept_profile(e));
        let material = match set_material {
            Some(m) => self.material(m),
            None => self.single_material(id),
        };
        let ifc_type = self.ix.type_of.get(&id).copied().filter(|t| {
            self.ent(*t)
                .is_some_and(|t| t.kind == "IFCCOLUMNTYPE" || t.kind == "IFCBEAMTYPE")
        });
        let guid = e.arg(0).as_str().unwrap_or("").to_string();

        match usage {
            MemberUsage::Column => {
                let (ax, ay) = match up {
                    0 => (1, 2),
                    1 => (2, 0),
                    _ => (0, 1),
                };
                let (w, d) = (range[ax], range[ay]);
                if w < 1e-3 || d < 1e-3 || range[up] < 1e-3 {
                    return Err("pilar sin volumen".into());
                }
                let mut rotation = axes[ax][1].atan2(axes[ax][0]);
                let profile = match section {
                    Some(Profile::Rectangle { .. }) | None => {
                        if section.is_none() {
                            self.report.warn(
                                "Pilares sin sección paramétrica: se importan con la caja de su sólido.",
                            );
                        }
                        Profile::Rectangle {
                            width: snap(w),
                            depth: snap(d),
                        }
                    }
                    Some(Profile::IShape { width, .. })
                        if (width - w).abs() > (width - d).abs() =>
                    {
                        // The section is turned a quarter in its solid.
                        rotation += std::f64::consts::FRAC_PI_2;
                        section.unwrap()
                    }
                    Some(p) => p,
                };
                let mut c = centre;
                c[up] = lo[up];
                let base = world(c);
                c[up] = hi[up];
                let top = world(c)[2];
                let (bottom, top) = (base[2].min(top), base[2].max(top));
                let level = self.level_of(id, bottom + 0.01);
                let elev = self.elevation(level);
                let t = self
                    .member_type(ifc_type, usage, profile, material, e)
                    .map_err(|e| format!("tipo no válido ({})", explain(&e)))?;
                let cid = self
                    .model
                    .add_column(NewColumn {
                        member_type: t,
                        level,
                        position: snap2(Point2::new(base[0], base[1])),
                        rotation: snap_angle(rotation),
                        base_offset: snap(bottom - elev),
                        top: WallTop::Unconnected,
                        height: snap(top - bottom),
                    })
                    .map_err(|e| explain(&e))?;
                self.report.columns += 1;
                self.sources.push((cid.0, id));
                self.guids.columns.push((cid, guid));
            }
            MemberUsage::Beam => {
                // Along the longest horizontal axis.
                let horizontal: Vec<usize> = (0..3).filter(|&i| i != up).collect();
                let (along, across) = if range[horizontal[0]] >= range[horizontal[1]] {
                    (horizontal[0], horizontal[1])
                } else {
                    (horizontal[1], horizontal[0])
                };
                if range[along] < 1e-3 || range[across] < 1e-3 || range[up] < 1e-3 {
                    return Err("viga sin volumen".into());
                }
                let mut c = centre;
                c[along] = lo[along];
                let start = world(c);
                c[along] = hi[along];
                let end = world(c);
                c[up] = hi[up];
                let top_world = world(c)[2].max({
                    c[up] = lo[up];
                    world(c)[2]
                });
                let profile = match section {
                    Some(
                        p @ (Profile::Circle { .. }
                        | Profile::IShape { .. }
                        | Profile::UShape { .. }
                        | Profile::TShape { .. }
                        | Profile::LShape { .. }
                        | Profile::RectHollow { .. }
                        | Profile::CircleHollow { .. }),
                    ) => p,
                    _ => {
                        if section.is_none() {
                            self.report.warn(
                                "Vigas sin sección paramétrica: se importan con la caja de su sólido.",
                            );
                        }
                        Profile::Rectangle {
                            width: snap(range[across]),
                            depth: snap(range[up]),
                        }
                    }
                };
                // The level whose elevation is closest above the top face
                // (a beam carries the floor of its level), else the one
                // holding it.
                let level = self.beam_level(id, top_world);
                let elev = self.elevation(level);
                let t = self
                    .member_type(ifc_type, usage, profile, material, e)
                    .map_err(|e| format!("tipo no válido ({})", explain(&e)))?;
                let bid = self
                    .model
                    .add_beam(NewBeam {
                        member_type: t,
                        level,
                        start: snap2(Point2::new(start[0], start[1])),
                        end: snap2(Point2::new(end[0], end[1])),
                        top_offset: snap(top_world - elev),
                    })
                    .map_err(|e| explain(&e))?;
                self.report.beams += 1;
                self.sources.push((bid.0, id));
                self.guids.beams.push((bid, guid));
            }
        }
        Ok(())
    }

    /// A beam's level: the storey that holds it in the file, if any;
    /// otherwise the level just above or at its top face.
    fn beam_level(&mut self, id: u64, top: f64) -> archisim_core::LevelId {
        if self.ix.container.contains_key(&id) || self.ix.parent.contains_key(&id) {
            return self.level_of(id, top);
        }
        let above = self
            .levels
            .iter()
            .find(|(_, e)| *e >= top - 0.01)
            .map(|(l, _)| *l);
        match above {
            Some(l) => l,
            None => self.level_of(id, top),
        }
    }

    fn member_type(
        &mut self,
        ifc_type: Option<u64>,
        usage: MemberUsage,
        profile: Profile,
        material: archisim_core::MaterialId,
        e: &Entity,
    ) -> Result<MemberTypeId, ModelError> {
        let (dims, shape) = key_of(profile);
        let key: MemberKey = (
            ifc_type,
            usage == MemberUsage::Column,
            dims,
            shape,
            material.0,
        );
        if let Some(&t) = self.member_types.get(&key) {
            return Ok(t);
        }
        let named = ifc_type
            .and_then(|t| self.ent(t))
            .and_then(|t| t.arg(2).as_str())
            .or_else(|| e.arg(4).as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);
        let base = named.unwrap_or_else(|| {
            let what = match usage {
                MemberUsage::Column => "Pilar",
                MemberUsage::Beam => "Viga",
            };
            let cm = |v: f64| format!("{:.0}", v * 100.0);
            match profile {
                Profile::Rectangle { width, depth } => {
                    format!("{what} {}×{}", cm(width), cm(depth))
                }
                Profile::Circle { diameter } => format!("{what} Ø{}", cm(diameter)),
                Profile::IShape { width, depth, .. } => {
                    format!("{what} I {}×{}", cm(width), cm(depth))
                }
                Profile::UShape { width, depth, .. } => {
                    format!("{what} U {}×{}", cm(width), cm(depth))
                }
                Profile::TShape { width, depth, .. } => {
                    format!("{what} T {}×{}", cm(width), cm(depth))
                }
                Profile::LShape { width, depth, .. } => {
                    format!("{what} L {}×{}", cm(width), cm(depth))
                }
                Profile::RectHollow { width, depth, .. } => {
                    format!("{what} tubo {}×{}", cm(width), cm(depth))
                }
                Profile::CircleHollow { diameter, .. } => {
                    format!("{what} tubo Ø{}", cm(diameter))
                }
            }
        });
        let name = self.unique_name(&base);
        let id = self.model.add_member_type(name, usage, material, profile)?;
        if let Some(g) = self.type_guid(ifc_type) {
            self.guids.member_types.push((id, g));
        }
        self.member_types.insert(key, id);
        self.report.member_types += 1;
        Ok(id)
    }
}

/// Angles within a hundredth of a degree of a multiple of 90° become exact.
fn snap_angle(a: f64) -> f64 {
    let q = std::f64::consts::FRAC_PI_2;
    let k = (a / q).round();
    if (a - k * q).abs() < 1e-4 {
        let r = k * q;
        // Keep the angle in (-π, π].
        if r <= -std::f64::consts::PI + 1e-9 {
            return r + std::f64::consts::TAU;
        }
        return if r.abs() < 1e-12 { 0.0 } else { r };
    }
    a
}
