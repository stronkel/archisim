//! The concept phase: volumes, faces and masses by floors; pushing them,
//! their parallel contours, guides, painting and turning them into a
//! building. Each change one undoable step.

use archisim_core::{
    DrawLayer, DrawLayerId, EdgeFinish, LineStyle, MassId, MassShape, MaterialId, Model, NewDraft,
    NewMass, Point2, Primitive, Shape, SlabFunction, SlabTypeId, ViewRef, WallFunction, WallTypeId,
};

use super::{EditResult, Editor};

/// The contour the volume tools draw with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassMode {
    Polygon,
    Rectangle,
    Circle,
    /// A band along a path ("Seguir").
    Path,
    /// Round forms: centre and radius (Formas).
    Sphere,
    Cylinder,
    Cone,
    /// A half-section drawn laid flat, turned round its axis.
    Revolution,
}

impl MassMode {
    /// Whether it makes a round form (the Formas tile).
    pub fn is_form(self) -> bool {
        matches!(
            self,
            MassMode::Sphere | MassMode::Cylinder | MassMode::Cone | MassMode::Revolution
        )
    }
}

/// What the contour makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassWhat {
    /// A flat face, to push (Línea, Rectángulo, Círculo).
    Face,
    /// A prism of a height.
    Volume,
    /// A mass by floors.
    Mass,
}

/// Settings of the volume tools.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassSettings {
    pub mode: MassMode,
    pub what: MassWhat,
    /// Height of a volume and of a band along a path.
    pub height: f64,
    pub floors: usize,
    pub floor_height: f64,
    /// Width of a band along a path.
    pub width: f64,
    pub base_offset: f64,
    pub material: MaterialId,
    /// Radius of a cone's top (0: a point).
    pub top_radius: f64,
    /// The profile Seguir sweeps.
    pub profile: SweepProfile,
}

/// The profile of a band along a path: the plain rectangle, a ready one
/// (`archisim_geom::sculpt::PRESETS`) or one taken from the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SweepProfile {
    #[default]
    Band,
    Preset(usize),
    Drawn,
}

impl SweepProfile {
    pub fn label(self) -> String {
        match self {
            SweepProfile::Band => "Rectangular".into(),
            SweepProfile::Preset(i) => {
                let n = archisim_geom::sculpt::PRESETS
                    .get(i)
                    .copied()
                    .unwrap_or("?");
                let mut c = n.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            }
            SweepProfile::Drawn => "Dibujado".into(),
        }
    }

    /// The profile's points, unit sized (the band stretches it to its
    /// width and height); `drawn` is the one taken from the drawing.
    pub fn points(self, drawn: Option<&[Point2]>) -> Option<Vec<Point2>> {
        match self {
            SweepProfile::Band => None,
            SweepProfile::Preset(i) => archisim_geom::sculpt::PRESETS
                .get(i)
                .and_then(|n| archisim_geom::sculpt::preset_profile(n, 1.0, 1.0)),
            SweepProfile::Drawn => drawn.map(<[Point2]>::to_vec),
        }
    }
}

/// A closed outline drawn in plan as a profile: across from its middle,
/// up from its lowest point.
pub fn profile_from_outline(outline: &[Point2]) -> Option<Vec<Point2>> {
    if outline.len() < 3 || archisim_core::signed_area(outline).abs() < 1e-6 {
        return None;
    }
    let (lo, hi) = outline.iter().fold(
        (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        ),
        |(lo, hi), p| {
            (
                Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    );
    let mid = (lo.x + hi.x) / 2.0;
    Some(
        outline
            .iter()
            .map(|p| Point2::new(p.x - mid, p.y - lo.y))
            .collect(),
    )
}

impl MassSettings {
    pub fn new(model: &Model) -> Self {
        let material = model
            .materials()
            .find(|m| m.category == "concept")
            .or_else(|| model.materials().find(|m| m.category == "concrete"))
            .or_else(|| model.materials().next())
            .map_or(MaterialId(0), |m| m.id);
        MassSettings {
            mode: MassMode::Polygon,
            what: MassWhat::Volume,
            height: 3.0,
            floors: 3,
            floor_height: 3.0,
            width: 0.3,
            base_offset: 0.0,
            material,
            top_radius: 0.0,
            profile: SweepProfile::Band,
        }
    }

    /// The height and floor height of what the tool makes.
    fn heights(&self) -> (f64, Option<f64>) {
        match self.what {
            MassWhat::Face => (0.0, None),
            MassWhat::Volume => (self.height, None),
            MassWhat::Mass => (
                self.floors.max(1) as f64 * self.floor_height,
                Some(self.floor_height),
            ),
        }
    }
}

/// The types a mass is turned into a building with: the first of each
/// kind (an exterior wall; a slab on the ground, a floor and a roof).
pub fn default_building_types(
    model: &Model,
) -> Option<(WallTypeId, SlabTypeId, SlabTypeId, SlabTypeId)> {
    let wall = model
        .wall_types()
        .find(|t| t.function == WallFunction::Exterior)
        .or_else(|| model.wall_types().next())?
        .id;
    let slab = |f: SlabFunction| {
        model
            .slab_types()
            .find(|t| t.function == f)
            .or_else(|| model.slab_types().next())
            .map(|t| t.id)
    };
    Some((
        wall,
        slab(SlabFunction::BaseSlab)?,
        slab(SlabFunction::Floor)?,
        slab(SlabFunction::Roof)?,
    ))
}

/// A volume, face or mass by its contour, inside a transaction.
pub fn add_mass_in(
    m: &mut Model,
    s: &MassSettings,
    level: archisim_core::LevelId,
    outline: Vec<Point2>,
) -> Result<MassId, archisim_core::ModelError> {
    add_mass_shape_in(m, s, level, MassShape::Prism(outline))
}

fn add_mass_shape_in(
    m: &mut Model,
    s: &MassSettings,
    level: archisim_core::LevelId,
    shape: MassShape,
) -> Result<MassId, archisim_core::ModelError> {
    let (height, floor_height) = s.heights();
    m.add_mass(NewMass {
        name: m.next_mass_name(floor_height.is_some()),
        level,
        shape,
        base_offset: s.base_offset,
        height,
        floor_height,
        material: s.material,
    })
}

/// A circle as a contour of 32 sides.
pub fn circle(centre: Point2, radius: f64) -> Vec<Point2> {
    (0..32)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 32.0;
            centre + Point2::new(a.cos(), a.sin()) * radius
        })
        .collect()
}

impl Editor {
    /// A volume, face or mass by its contour on the active level.
    pub fn add_mass_outline(
        &mut self,
        s: &MassSettings,
        outline: Vec<Point2>,
    ) -> EditResult<MassId> {
        self.add_mass_shape(s, MassShape::Prism(outline))
    }

    /// A band along a path.
    pub fn add_mass_path(
        &mut self,
        s: &MassSettings,
        path: Vec<Point2>,
        drawn: Option<&[Point2]>,
    ) -> EditResult<MassId> {
        let mut s = *s;
        if s.what == MassWhat::Face {
            s.what = MassWhat::Volume;
        }
        self.add_mass_shape(
            &s,
            MassShape::Sweep {
                path,
                width: s.width,
                profile: s.profile.points(drawn),
            },
        )
    }

    /// Divides a face of a volume along a line (see
    /// [`archisim_geom::sculpt::divide_mass_face`]). Returns the new face.
    pub fn divide_mass_face(
        &mut self,
        id: MassId,
        face: u32,
        point: [f64; 3],
        normal: [f64; 3],
    ) -> EditResult<u32> {
        let mut why = None;
        let r = self.transact("Dividir cara", |m| {
            archisim_geom::sculpt::divide_mass_face(m, id, face, point, normal).map_err(|e| {
                why = Some(e.to_string());
                archisim_core::ModelError::InvalidShape("operation")
            })
        });
        r.map_err(|e| why.unwrap_or(e))
    }

    /// Pushes any face of a volume along its normal (see
    /// [`archisim_geom::sculpt::push_mass_face`]).
    pub fn push_mass_face(&mut self, id: MassId, face: u32, d: f64) -> EditResult {
        let mut why = None;
        let r = self.transact("Empujar", |m| {
            archisim_geom::sculpt::push_mass_face(m, id, face, d).map_err(|e| {
                why = Some(e.to_string());
                archisim_core::ModelError::InvalidShape("operation")
            })
        });
        r.map_err(|e| why.unwrap_or(e))
    }

    /// Twists and bends a volume (nothing: straight again).
    pub fn set_mass_deform(&mut self, id: MassId, d: archisim_core::Deform) -> EditResult {
        self.transact("Deformar", |m| {
            m.edit_mass(id, |x| x.deform = (!d.is_none()).then_some(d))
        })
    }

    /// The profile a band along a path sweeps (nothing: the rectangle).
    pub fn set_sweep_profile(&mut self, id: MassId, profile: Option<Vec<Point2>>) -> EditResult {
        self.transact("Perfil", |m| {
            m.edit_mass(id, |x| {
                if let MassShape::Sweep { profile: p, .. } = &mut x.shape {
                    *p = profile;
                }
            })
        })
    }

    /// Rounds the corners and top edges of volumes, or leaves them sharp
    /// if they were all rounded.
    pub fn round_mass_edges(&mut self, ids: &[MassId]) -> EditResult<bool> {
        let rounded = ids.iter().all(|id| {
            self.model().mass(*id).is_some_and(|m| {
                m.finish
                    .is_some_and(|f| f.kind == archisim_core::EdgeKind::Round)
            })
        });
        self.transact("Redondear aristas", |m| {
            for id in ids {
                let round = !matches!(
                    m.mass(*id).map(|x| &x.shape),
                    Some(MassShape::Mesh(_)) | None
                );
                if !round {
                    continue;
                }
                m.edit_mass(*id, |x| {
                    x.finish = (!rounded).then_some(archisim_core::EdgeFinish {
                        kind: archisim_core::EdgeKind::Round,
                        size: 0.15,
                        vertical: matches!(x.shape, MassShape::Prism(_)),
                        top: true,
                        bottom: false,
                    })
                })?;
            }
            Ok(())
        })?;
        Ok(!rounded)
    }

    fn add_mass_shape(&mut self, s: &MassSettings, shape: MassShape) -> EditResult<MassId> {
        let level = self.level;
        let label = match s.what {
            MassWhat::Face => "Crear cara",
            MassWhat::Volume => "Crear volumen",
            MassWhat::Mass => "Crear masa",
        };
        self.transact(label, |m| add_mass_shape_in(m, s, level, shape))
    }

    pub fn edit_mass(
        &mut self,
        label: &str,
        id: MassId,
        edit: impl FnOnce(&mut archisim_core::Mass),
    ) -> EditResult {
        self.transact(label, |m| m.edit_mass(id, edit))
    }

    /// Pushes a mass (or a face) to a height; a mass by floors to a whole
    /// number of them.
    pub fn push_mass(&mut self, id: MassId, height: f64) -> EditResult {
        let height = snap_height(self.model(), id, height);
        self.transact("Empujar", |m| m.edit_mass(id, |x| x.height = height))
    }

    /// A face `d` outside the contour of a mass (inside, if negative).
    pub fn offset_mass(&mut self, id: MassId, d: f64) -> EditResult<MassId> {
        let mass = self
            .model()
            .mass(id)
            .ok_or("el volumen ya no existe")?
            .clone();
        let outline = archisim_core::offset_contour(&mass.outline(), d)
            .ok_or("el contorno no cabe a esa distancia")?;
        self.transact("Equidistancia", |m| {
            m.add_mass(NewMass {
                name: m.next_mass_name(false),
                level: mass.level,
                shape: MassShape::Prism(outline),
                base_offset: mass.base_offset + mass.height,
                height: 0.0,
                floor_height: None,
                material: mass.material,
            })
        })
    }

    pub fn paint_mass(&mut self, id: MassId, material: MaterialId) -> EditResult {
        self.transact("Pintar", |m| m.edit_mass(id, |x| x.material = material))
    }

    /// A guide through two points, on the «Guías» layer (made if missing,
    /// not printed), as SketchUp's tape measure leaves.
    pub fn add_guide(&mut self, a: Point2, b: Point2) -> EditResult {
        let level = self.level;
        let Some(u) = (b - a).normalized() else {
            return Err("marca dos puntos distintos".into());
        };
        self.transact("Guía", |m| {
            let layer = guide_layer(m)?;
            m.add_draft(NewDraft::new(
                ViewRef::Level(level),
                layer,
                Shape::Line {
                    a: a - u * 50.0,
                    b: b + u * 50.0,
                },
            ))
            .map(drop)
        })
    }

    /// Turns the masses into buildings with the default types (and the
    /// ones made before, again). Returns how many elements they made.
    pub fn masses_to_buildings(&mut self, ids: &[MassId]) -> EditResult<usize> {
        let (w, g, f, r) = default_building_types(self.model())
            .ok_or("el proyecto no tiene tipos de muro y forjado")?;
        self.transact("Convertir en edificio", |m| {
            let mut n = 0;
            for id in ids {
                let (w, g, f, r) = m
                    .mass(*id)
                    .and_then(|x| x.building.as_ref())
                    .map_or((w, g, f, r), |b| {
                        (b.wall_type, b.ground_type, b.floor_type, b.roof_type)
                    });
                n += m.mass_to_building(*id, w, g, f, r)?;
            }
            Ok(n)
        })
    }

    /// A sphere, cylinder or cone with its centre and radius (a sphere is
    /// as high as it is wide).
    pub fn add_mass_form(
        &mut self,
        s: &MassSettings,
        centre: Point2,
        radius: f64,
    ) -> EditResult<MassId> {
        let mut s = *s;
        s.what = MassWhat::Volume;
        let primitive = match s.mode {
            MassMode::Sphere => {
                s.height = 2.0 * radius;
                Primitive::Sphere {
                    center: centre,
                    radius,
                }
            }
            MassMode::Cone => Primitive::Cone {
                center: centre,
                radius,
                top_radius: s.top_radius.min(radius * 4.0),
            },
            _ => Primitive::Cylinder {
                center: centre,
                radius,
            },
        };
        let level = self.level;
        let label = format!("Crear {}", primitive.label());
        self.transact(&label, |m| {
            add_mass_shape_in(m, &s, level, MassShape::Primitive(primitive))
        })
    }

    /// A solid of revolution: its axis in plan and its half-section drawn
    /// laid flat beside it (to the side: the distance from the axis; up
    /// the plan: the height).
    pub fn add_mass_revolution(
        &mut self,
        s: &MassSettings,
        axis: Point2,
        points: &[Point2],
    ) -> EditResult<MassId> {
        let profile: Vec<Point2> = points
            .iter()
            .map(|p| Point2::new((p.x - axis.x).abs(), (p.y - axis.y).max(0.0)))
            .collect();
        let top = profile.iter().map(|p| p.y).fold(0.0, f64::max);
        if top <= 1e-6 {
            return Err(
                "el perfil necesita altura: dibújalo hacia arriba en la planta desde el eje".into(),
            );
        }
        let mut s = *s;
        s.what = MassWhat::Volume;
        s.height = top;
        let level = self.level;
        self.transact("Crear revolución", |m| {
            add_mass_shape_in(
                m,
                &s,
                level,
                MassShape::Primitive(Primitive::Revolution {
                    center: axis,
                    profile,
                }),
            )
        })
    }

    /// Chamfers or rounds the edges of a volume (`None`: sharp).
    pub fn set_mass_finish(&mut self, id: MassId, finish: Option<EdgeFinish>) -> EditResult {
        self.transact("Aristas", |m| m.edit_mass(id, |x| x.finish = finish))
    }

    /// Joins volumes to `target`, cuts them from it, or keeps what they
    /// have in common (see [`archisim_geom::forms::combine_masses`]).
    pub fn combine_masses(
        &mut self,
        target: MassId,
        others: &[MassId],
        op: archisim_geom::csg::Op,
    ) -> EditResult {
        if others.is_empty() {
            return Err("elige otro volumen".into());
        }
        let label = match op {
            archisim_geom::csg::Op::Union => "Unir volúmenes",
            archisim_geom::csg::Op::Subtract => "Restar volúmenes",
            archisim_geom::csg::Op::Intersect => "Intersecar volúmenes",
        };
        let mut why = None;
        let r = self.transact(label, |m| {
            archisim_geom::forms::combine_masses(m, target, others, op).map_err(|e| {
                why = Some(e.to_string());
                archisim_core::ModelError::InvalidShape("operation")
            })
        });
        r.map_err(|e| why.unwrap_or(e))
    }

    /// Loose edges where the faces of the volumes cross each other
    /// (SketchUp's «intersecar caras»). Returns how many.
    pub fn cross_masses(&mut self, ids: &[MassId]) -> EditResult<usize> {
        let mut why = None;
        let r = self.transact("Intersecar caras", |m| {
            archisim_geom::forms::cross_masses(m, ids).map_err(|e| {
                why = Some(e.to_string());
                archisim_core::ModelError::InvalidShape("operation")
            })
        });
        r.map_err(|e| why.unwrap_or(e))
    }

    pub fn set_plot_area(&mut self, area: Option<f64>) -> EditResult {
        self.transact("Superficie de la parcela", |m| m.set_plot_area(area))
    }
}

/// A height for a mass: a mass by floors goes by whole floors (one at
/// least); others stay at or above zero.
pub fn snap_height(model: &Model, id: MassId, height: f64) -> f64 {
    let h = height.max(0.0);
    match model.mass(id).and_then(|m| m.floor_height) {
        Some(f) if f > 0.0 => (h / f).round().max(1.0) * f,
        _ => (h * 1000.0).round() / 1000.0,
    }
}

/// The «Guías» layer: thin, dashed and not printed.
fn guide_layer(m: &mut Model) -> Result<DrawLayerId, archisim_core::ModelError> {
    if let Some(l) = m.draw_layers().into_iter().find(|l| l.name == "Guías") {
        return Ok(l.id);
    }
    let mut layer = DrawLayer::new("Guías", [0, 160, 200], 0.13, LineStyle::Dashed);
    layer.printable = false;
    m.add_draw_layer(layer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_are_pushed_offset_painted_and_converted() {
        let mut ed = Editor::new_project();
        let mut s = MassSettings::new(ed.model());
        s.what = MassWhat::Face;
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        let face = ed.add_mass_outline(&s, square.clone()).unwrap();
        assert!(ed.model().mass(face).unwrap().is_face());
        ed.push_mass(face, 6.5).unwrap();
        assert!((ed.model().mass(face).unwrap().height - 6.5).abs() < 1e-9);
        // A mass by floors goes by whole floors.
        s.what = MassWhat::Mass;
        let mass = ed.add_mass_outline(&s, square).unwrap();
        ed.push_mass(mass, 10.4).unwrap();
        assert!((ed.model().mass(mass).unwrap().height - 9.0).abs() < 1e-9);
        let top = ed.offset_mass(mass, -2.0).unwrap();
        let t = ed.model().mass(top).unwrap();
        assert!((archisim_core::signed_area(&t.outline()) - 36.0).abs() < 1e-9);
        assert!((t.base_offset - 9.0).abs() < 1e-9);
        let red = ed.model().materials().last().unwrap().id;
        ed.paint_mass(mass, red).unwrap();
        assert_eq!(ed.model().mass(mass).unwrap().material, red);
        let n = ed.masses_to_buildings(&[mass]).unwrap();
        assert_eq!(n, 3 * 5 + 1);
        assert_eq!(ed.model().walls().count(), 12);
        ed.add_guide(Point2::new(0.0, 0.0), Point2::new(5.0, 0.0))
            .unwrap();
        assert!(ed
            .model()
            .draw_layers()
            .into_iter()
            .any(|l| l.name == "Guías" && !l.printable));
    }

    #[test]
    fn round_forms_joined_and_cut() {
        use archisim_geom::csg::Op;
        use archisim_geom::forms::{mass_body, mesh_volume};
        let mut ed = Editor::new_project();
        let mut s = MassSettings::new(ed.model());
        s.what = MassWhat::Volume;
        s.height = 4.0;
        let square = vec![
            Point2::new(0.0, 0.0),
            Point2::new(6.0, 0.0),
            Point2::new(6.0, 6.0),
            Point2::new(0.0, 6.0),
        ];
        let block = ed.add_mass_outline(&s, square).unwrap();
        s.mode = MassMode::Cylinder;
        let cyl = ed.add_mass_form(&s, Point2::new(6.0, 3.0), 2.0).unwrap();
        s.mode = MassMode::Sphere;
        let ball = ed.add_mass_form(&s, Point2::new(0.0, 0.0), 1.5).unwrap();
        assert!((ed.model().mass(ball).unwrap().height - 3.0).abs() < 1e-9);
        let rev = ed
            .add_mass_revolution(
                &s,
                Point2::new(20.0, 0.0),
                &[
                    Point2::new(21.0, 0.0),
                    Point2::new(22.0, 1.0),
                    Point2::new(20.5, 3.0),
                ],
            )
            .unwrap();
        assert!((ed.model().mass(rev).unwrap().height - 3.0).abs() < 1e-9);
        // Rounded top edges.
        ed.set_mass_finish(
            block,
            Some(EdgeFinish {
                kind: archisim_core::EdgeKind::Round,
                size: 0.4,
                vertical: true,
                top: true,
                bottom: false,
            }),
        )
        .unwrap();
        let v0 = mesh_volume(&mass_body(ed.model().mass(block).unwrap()));
        assert!(v0 < 144.0 && v0 > 130.0, "{v0}");
        // The cylinder, painted, cut out of the block: its curved face stays
        // with its paint.
        let red = ed.model().materials().last().unwrap().id;
        ed.transact("Pintar", |m| {
            m.set_face_paint(cyl.0, archisim_core::FaceRef::Side(0), Some(red))
        })
        .unwrap();
        ed.combine_masses(block, &[cyl], Op::Subtract).unwrap();
        assert!(ed.model().mass(cyl).is_none());
        let m = ed.model().mass(block).unwrap();
        assert!(matches!(m.shape, MassShape::Mesh(_)));
        let v1 = mesh_volume(&mass_body(m));
        assert!(v1 < v0 - 10.0, "{v0} {v1}");
        let solids = archisim_geom::solid::mass_solids(ed.model(), block);
        assert!(solids.iter().any(|s| s.material == red));
        // The ball joined to it.
        ed.combine_masses(block, &[ball], Op::Union).unwrap();
        let v2 = mesh_volume(&mass_body(ed.model().mass(block).unwrap()));
        assert!(v2 > v1, "{v1} {v2}");
        // Edges where two volumes cross.
        s.mode = MassMode::Polygon;
        let other = ed
            .add_mass_outline(
                &s,
                vec![
                    Point2::new(3.0, 3.0),
                    Point2::new(9.0, 3.0),
                    Point2::new(9.0, 9.0),
                    Point2::new(3.0, 9.0),
                ],
            )
            .unwrap();
        let n = ed.cross_masses(&[block, other]).unwrap();
        assert!(n >= 1);
        assert!(ed.model().edges().next().is_some());
        // Saved and opened again, all the same.
        let text = archisim_file::to_string(ed.model());
        let back = archisim_file::from_str(&text).unwrap();
        assert_eq!(archisim_file::to_string(&back), text);
    }
}
