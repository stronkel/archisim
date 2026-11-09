//! Models of volumes from SketchUp (or any program that exports COLLADA,
//! OBJ, STL…) brought in as volumes of the concept phase, to go on
//! working them: each closed piece of the model is a volume. A piece that
//! is a contour pushed up becomes an ordinary volume (its contour and its
//! height: it can be pushed, its contour edited, made a mass by floors and
//! turned into a building); any other, a solid of triangles whose flat
//! faces are faces of the volume (painted and pushed one by one). The
//! materials of the model become materials of the project with their
//! colour, on the faces that had them.

use std::collections::HashMap;

use archisim_core::{
    Appearance, CutPattern, FaceRef, LevelId, MassId, MassMesh, MassShape, MaterialId, Model,
    NewMass, Point2,
};
use archisim_mesh::{extrusion, flat_face, is_plain_material, solids, Mesh, Solid};

use crate::imported::{finish_of, free_name};

/// Points closer than this are one (metres).
pub const WELD_TOLERANCE: f64 = 0.001;

/// How a model is brought in as volumes.
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptOptions {
    /// The level the volumes stand on (the model's `z = 0` on its floor).
    pub level: LevelId,
    /// The name of the volumes («<name>», or «<name> 1», «<name> 2»…).
    pub name: String,
    /// Where the middle of the model's base goes; `None`: the model's
    /// origin on the project's.
    pub at: Option<Point2>,
    /// Points closer than this are one, metres.
    pub tolerance: f64,
    /// Whether the pieces that are a contour pushed up become ordinary
    /// volumes (otherwise every piece is a solid of triangles).
    pub prisms: bool,
}

impl ConceptOptions {
    pub fn new(level: LevelId, name: &str) -> Self {
        Self {
            level,
            name: name.to_string(),
            at: None,
            tolerance: WELD_TOLERANCE,
            prisms: true,
        }
    }
}

/// What a piece of the model became.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    /// A contour pushed up: an ordinary volume.
    Prism,
    /// A single horizontal face (a volume of no height, to push).
    Face,
    /// A solid of triangles.
    Mesh,
}

impl PieceKind {
    pub fn label(self) -> &'static str {
        match self {
            PieceKind::Prism => "prisma",
            PieceKind::Face => "cara",
            PieceKind::Mesh => "malla",
        }
    }
}

/// A volume made from a piece of the model.
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptPiece {
    pub mass: MassId,
    pub name: String,
    pub kind: PieceKind,
    /// Closed solid (an open one is brought in as it is: a surface).
    pub closed: bool,
    pub holes_closed: usize,
    /// Its faces (flat faces of one material).
    pub faces: usize,
    pub triangles: usize,
    /// m² seen from above (a prism's contour).
    pub footprint: f64,
    pub height: f64,
    /// m³, of a closed piece.
    pub volume: Option<f64>,
    /// Faces painted with a material other than the volume's.
    pub painted: usize,
}

/// What was brought in.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConceptImport {
    pub pieces: Vec<ConceptPiece>,
    /// Pieces left out: too small to be a volume (fewer than four
    /// triangles that are not a flat face).
    pub skipped: usize,
    /// Faces exported twice (both sides) that were dropped.
    pub doubled: usize,
    /// Materials of the project made for the model.
    pub materials: Vec<MaterialId>,
}

impl ConceptImport {
    pub fn count(&self, kind: PieceKind) -> usize {
        self.pieces.iter().filter(|p| p.kind == kind).count()
    }

    pub fn open(&self) -> usize {
        self.pieces.iter().filter(|p| !p.closed).count()
    }

    /// A line for the window: «3 volúmenes: 2 prismas, 1 malla».
    pub fn summary(&self) -> String {
        let n = self.pieces.len();
        let mut parts = Vec::new();
        for (k, one, many) in [
            (PieceKind::Prism, "prisma", "prismas"),
            (PieceKind::Mesh, "malla", "mallas"),
            (PieceKind::Face, "cara", "caras"),
        ] {
            let c = self.count(k);
            if c > 0 {
                parts.push(format!("{c} {}", if c == 1 { one } else { many }));
            }
        }
        let mut s = format!(
            "{n} {}: {}",
            if n == 1 { "volumen" } else { "volúmenes" },
            parts.join(", ")
        );
        let open = self.open();
        if open > 0 {
            s.push_str(&format!(
                "; {open} {} (traída{} como superficie)",
                if open == 1 { "abierta" } else { "abiertas" },
                if open == 1 { "" } else { "s" }
            ));
        }
        s
    }

    /// The report, piece by piece.
    pub fn report(&self, source: &str) -> String {
        let mut s = format!("Volúmenes de Concepto traídos de {source}\n\n");
        s.push_str(&format!("{}.\n", self.summary()));
        if self.doubled > 0 {
            s.push_str(&format!(
                "{} caras venían por las dos caras (exportación a dos caras): se queda una.\n",
                self.doubled
            ));
        }
        if self.skipped > 0 {
            s.push_str(&format!(
                "{} piezas sueltas demasiado pequeñas para ser un volumen (menos de cuatro triángulos): no se traen.\n",
                self.skipped
            ));
        }
        s.push_str(&format!(
            "{} materiales nuevos en el proyecto.\n\n",
            self.materials.len()
        ));
        s.push_str(
            "Volumen | Es | Cerrado | Caras | Triángulos | Planta m² | Alto m | Volumen m³\n",
        );
        for p in &self.pieces {
            let closed = match (p.closed, p.holes_closed) {
                (false, _) => "no (superficie)".to_string(),
                (true, 0) => "sí".to_string(),
                (true, h) => format!("sí ({h} agujero{} cerrado{})", plural(h), plural(h)),
            };
            s.push_str(&format!(
                "{} | {} | {} | {} | {} | {} | {} | {}\n",
                p.name,
                p.kind.label(),
                closed,
                p.faces,
                p.triangles,
                comma(p.footprint),
                comma(p.height),
                p.volume.map_or("—".into(), comma)
            ));
        }
        s.push_str(
            "\nLos prismas son volúmenes como los dibujados: se empujan, se edita su contorno, se hacen masas por plantas y se pasan a edificio. Las mallas se pintan y se empujan cara a cara.\n",
        );
        s
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn comma(v: f64) -> String {
    format!("{:.2}", v + 0.0)
        .replace('.', ",")
        .replace("-0,00", "0,00")
}

/// The project's material for a material of the model: the one of the
/// same name, or a new one with its colour (of the kind its name says);
/// a face left with the default colour, the concept material.
fn project_material(
    model: &mut Model,
    mesh: &Mesh,
    index: usize,
    made: &mut Vec<MaterialId>,
    cache: &mut HashMap<usize, MaterialId>,
) -> MaterialId {
    if let Some(m) = cache.get(&index) {
        return *m;
    }
    let id = match mesh.materials.get(index) {
        Some(mat) if !is_plain_material(&mat.name) => {
            let name = mat.name.replace('_', " ").trim().to_string();
            let same = model.materials().find(|x| x.name == name).map(|x| x.id);
            match same {
                Some(x) => x,
                None => {
                    let name = free_name(|n| model.materials().any(|x| x.name == n), &name);
                    let (category, texture) = finish_of(&mat.name);
                    let category = if mat.alpha < 0.6 { "glass" } else { category };
                    let id = model.add_material(name, category, mat.color, CutPattern::Solid);
                    if let Some(t) = texture.filter(|_| category != "glass") {
                        let _ = model.set_appearance(
                            id,
                            Appearance {
                                color: None,
                                texture: Some(t),
                                scale: None,
                                growth: None,
                            },
                        );
                    }
                    made.push(id);
                    id
                }
            }
        }
        _ => concept_material(model, made),
    };
    cache.insert(index, id);
    id
}

/// The material of the concept volumes, made if the project has none.
fn concept_material(model: &mut Model, made: &mut Vec<MaterialId>) -> MaterialId {
    if let Some(m) = model
        .materials()
        .find(|x| x.category == "concept")
        .map(|x| x.id)
    {
        return m;
    }
    let id = model.add_material(
        "Volumen de concepto",
        "concept",
        [222, 222, 214],
        CutPattern::Solid,
    );
    made.push(id);
    id
}

/// The area of each face of a solid.
fn face_areas(s: &Solid) -> Vec<f64> {
    let mut a = vec![0.0; s.face_materials.len()];
    for (t, f) in s.triangles.iter().zip(&s.faces) {
        let [p, q, r] = t.map(|i| s.positions[i as usize]);
        let u = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
        let v = [r[0] - p[0], r[1] - p[1], r[2] - p[2]];
        let c = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        a[*f as usize] += (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt() / 2.0;
    }
    a
}

fn ring_area(o: &[[f64; 2]]) -> f64 {
    (0..o.len())
        .map(|i| {
            let (a, b) = (o[i], o[(i + 1) % o.len()]);
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>()
        / 2.0
}

/// Brings `mesh` (metres, `z` up) into `model` as volumes of the concept
/// phase.
pub fn import_concept(
    model: &mut Model,
    mesh: &Mesh,
    options: &ConceptOptions,
) -> Result<ConceptImport, String> {
    if model.level(options.level).is_none() {
        return Err("la planta no existe".into());
    }
    let (lo, hi) = mesh.bounds();
    let size = [0, 1, 2].map(|k| (hi[k] - lo[k]) as f64);
    if size.iter().any(|s| *s > 20_000.0) {
        return Err(format!(
            "el modelo mide {:.0} m: compruebe sus unidades (¿milímetros, centímetros o pulgadas?)",
            size.iter().fold(0.0f64, |a, b| a.max(*b))
        ));
    }
    // Where it goes: its origin on the project's, or the middle of its
    // base where asked.
    let shift = match options.at {
        Some(p) => [
            p.x - (lo[0] + hi[0]) as f64 / 2.0,
            p.y - (lo[1] + hi[1]) as f64 / 2.0,
        ],
        None => [0.0, 0.0],
    };
    let found = solids(mesh, options.tolerance);
    let mut out = ConceptImport {
        doubled: found.doubled,
        ..Default::default()
    };
    let mut cache = HashMap::new();
    let usable: Vec<&Solid> = found
        .solids
        .iter()
        .filter(|s| {
            let ok = s.triangles.len() >= 4 || flat_face(s, options.tolerance).is_some();
            if !ok {
                out.skipped += 1;
            }
            ok
        })
        .collect();
    let many = usable.len() > 1;
    let stem = if options.name.trim().is_empty() {
        "Volumen importado".to_string()
    } else {
        options.name.trim().to_string()
    };
    for (i, s) in usable.into_iter().enumerate() {
        let base_name = if many {
            format!("{stem} {}", i + 1)
        } else {
            stem.clone()
        };
        let name = free_name(|n| model.masses().any(|m| m.name == n), &base_name);
        // The material of each face of the piece, and the most used (by
        // area) for the volume.
        let mats: Vec<MaterialId> = s
            .face_materials
            .iter()
            .map(|m| project_material(model, mesh, *m, &mut out.materials, &mut cache))
            .collect();
        let areas = face_areas(s);
        let mut by_mat: HashMap<MaterialId, f64> = HashMap::new();
        for (m, a) in mats.iter().zip(&areas) {
            *by_mat.entry(*m).or_default() += a;
        }
        let main = by_mat
            .iter()
            .max_by(|a, b| a.1.total_cmp(b.1).then(b.0 .0.cmp(&a.0 .0)))
            .map(|(m, _)| *m)
            .unwrap_or_else(|| concept_material(model, &mut out.materials));
        let to_plan = |p: [f64; 2]| Point2::new(p[0] + shift[0], p[1] + shift[1]);
        let prism = options
            .prisms
            .then(|| extrusion(s, options.tolerance))
            .flatten();
        let flat = if prism.is_none() && options.prisms {
            flat_face(s, options.tolerance)
        } else {
            None
        };
        let (kind, shape, base, height, footprint, paints) = if let Some(e) = &prism {
            let mut paints: Vec<(FaceRef, MaterialId)> = vec![
                (FaceRef::Top, mats[e.top_face as usize]),
                (FaceRef::Bottom, mats[e.bottom_face as usize]),
            ];
            for (k, f) in e.side_faces.iter().enumerate() {
                paints.push((FaceRef::Side(k), mats[*f as usize]));
            }
            (
                PieceKind::Prism,
                MassShape::Prism(e.outline.iter().map(|p| to_plan(*p)).collect()),
                e.base,
                e.height,
                ring_area(&e.outline).abs(),
                paints,
            )
        } else if let Some((o, z)) = &flat {
            let m = mats.first().copied().unwrap_or(main);
            (
                PieceKind::Face,
                MassShape::Prism(o.iter().map(|p| to_plan(*p)).collect()),
                *z,
                0.0,
                ring_area(o).abs(),
                vec![(FaceRef::Top, m), (FaceRef::Bottom, m)],
            )
        } else {
            let zmin = s
                .positions
                .iter()
                .map(|p| p[2])
                .fold(f64::INFINITY, f64::min);
            let zmax = s
                .positions
                .iter()
                .map(|p| p[2])
                .fold(f64::NEG_INFINITY, f64::max);
            let body = MassMesh {
                positions: s
                    .positions
                    .iter()
                    .map(|p| [p[0] + shift[0], p[1] + shift[1], p[2] - zmin])
                    .collect(),
                triangles: s.triangles.clone(),
                faces: s.faces.clone(),
            };
            // Seen from above: what its faces looking up cover, roughly
            // (the plan draws it exactly).
            let (up, down) = s.triangles.iter().fold((0.0, 0.0), |(u, d), t| {
                let [p, q, r] = t.map(|i| s.positions[i as usize]);
                let a = ((q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])) / 2.0;
                if a > 0.0 {
                    (u + a, d)
                } else {
                    (u, d - a)
                }
            });
            let up: f64 = f64::max(up, down);
            let paints = mats
                .iter()
                .enumerate()
                .map(|(f, m)| (FaceRef::Part(f as u32), *m))
                .collect();
            (
                PieceKind::Mesh,
                MassShape::Mesh(body),
                zmin,
                zmax - zmin,
                up,
                paints,
            )
        };
        let id = model
            .add_mass(NewMass {
                name: name.clone(),
                level: options.level,
                shape,
                base_offset: base,
                height,
                floor_height: None,
                material: main,
            })
            .map_err(|e| format!("«{name}»: {e}"))?;
        let mut painted = 0;
        for (face, m) in paints {
            if m != main && model.set_face_paint(id.0, face, Some(m)).is_ok() {
                painted += 1;
            }
        }
        let volume = s.closed.then(|| match kind {
            PieceKind::Prism => footprint * height,
            _ => archisim_mesh::volume(s),
        });
        out.pieces.push(ConceptPiece {
            mass: id,
            name,
            kind,
            closed: s.closed,
            holes_closed: s.holes_closed,
            faces: match kind {
                PieceKind::Prism => prism.as_ref().map_or(0, |e| e.outline.len() + 2),
                PieceKind::Face => 1,
                PieceKind::Mesh => s.face_materials.len(),
            },
            triangles: s.triangles.len(),
            footprint,
            height,
            volume,
            painted,
        });
    }
    if out.pieces.is_empty() {
        return Err("el modelo no tiene ninguna pieza que pueda ser un volumen".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Mesh {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../archisim-mesh/tests/fixtures")
            .join(name);
        let mut l = archisim_mesh::load(&path, None, archisim_mesh::Options::default()).unwrap();
        l.guess_for_site();
        l.mesh
    }

    #[test]
    fn sketchup_volumes_become_two_prisms_and_a_mesh() {
        let mut m = crate::new_project("Volúmenes");
        let level = m.levels()[0].id;
        let r = import_concept(
            &mut m,
            &fixture("volumenes.dae"),
            &ConceptOptions::new(level, "Volúmenes"),
        )
        .unwrap();
        assert_eq!(r.pieces.len(), 3);
        assert_eq!(r.count(PieceKind::Prism), 2);
        assert_eq!(r.count(PieceKind::Mesh), 1);
        assert_eq!(r.doubled, 12);
        let a = m.mass(r.pieces[0].mass).unwrap();
        assert!(matches!(a.shape, MassShape::Prism(_)));
        assert!((a.height - 9.0).abs() < 1e-3);
        assert!((r.pieces[0].footprint - 120.0).abs() < 1e-2);
        // Walls white, gravel on top: the volume is white, its top painted.
        assert_eq!(m.material(a.material).unwrap().name, "Fachada blanca");
        let top = m.face_paint(a.id.0, FaceRef::Top).unwrap();
        assert_eq!(m.material(top).unwrap().name, "Cubierta grava");
        assert_eq!(m.material(top).unwrap().cut_color, [140, 140, 133]);
        // The tower, without bottom, closed.
        assert_eq!(r.pieces[1].holes_closed, 1);
        // The house: a mesh with its roofs painted tile.
        let c = m.mass(r.pieces[2].mass).unwrap();
        let MassShape::Mesh(body) = &c.shape else {
            panic!("a mesh");
        };
        assert!(body.positions.iter().all(|p| p[2] >= -1e-9));
        assert!((c.height - 9.0).abs() < 1e-3);
        let roofs = body
            .face_ids()
            .into_iter()
            .filter(|f| {
                m.face_paint(c.id.0, FaceRef::Part(*f))
                    .is_some_and(|x| m.material(x).unwrap().name == "Teja roja")
            })
            .count();
        assert!(roofs >= 4, "{roofs}");
        // A prism turns into a building.
        let wt = m.wall_types().next().unwrap().id;
        let st = m.slab_types().next().unwrap().id;
        let id = r.pieces[1].mass;
        m.edit_mass(id, |x| x.floor_height = Some(3.0)).unwrap();
        let n = m.mass_to_building(id, wt, st, st, st).unwrap();
        // Five floors of four walls and a slab, and the roof.
        assert_eq!(n, 26);
        let text = r.report("volumenes.dae");
        assert!(
            text.contains("| prisma | sí (1 agujero cerrado) |"),
            "{text}"
        );
        assert!(text.contains("| malla | sí |"), "{text}");
    }

    #[test]
    fn a_curved_volume_a_surface_and_a_place() {
        let mut m = crate::new_project("Curvo");
        let level = m.levels()[0].id;
        let mut o = ConceptOptions::new(level, "Pieza");
        o.at = Some(Point2::new(100.0, 50.0));
        let r = import_concept(&mut m, &fixture("volumenes.obj"), &o).unwrap();
        assert_eq!(r.pieces.len(), 2);
        assert_eq!(r.count(PieceKind::Mesh), 2);
        assert_eq!(r.open(), 1);
        assert!(r.summary().contains("1 abierta"), "{}", r.summary());
        assert_eq!(r.pieces[0].name, "Pieza 1");
        // Its base centred where asked.
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &r.pieces {
            for q in m.mass(p.mass).unwrap().outline() {
                lo = lo.min(q.x);
                hi = hi.max(q.x);
            }
        }
        assert!(((lo + hi) / 2.0 - 100.0).abs() < 0.05, "{lo} {hi}");
        // A single material: no faces painted.
        assert_eq!(r.pieces[0].painted, 0);
        let dome = m.mass(r.pieces[0].mass).unwrap();
        assert_eq!(m.material(dome.material).unwrap().name, "Hormigon");
        let v = r.pieces[0].volume.unwrap();
        // A capsule of radius 3 and 8 m between its hemispheres.
        let exact = std::f64::consts::PI * 9.0 * 8.0 + 4.0 / 3.0 * std::f64::consts::PI * 27.0;
        assert!((v - exact).abs() / exact < 0.05, "{v} {exact}");
    }
}
