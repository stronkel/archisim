//! Triangles of any IFC element, in world coordinates (metres), from the
//! usual geometry of other programs: extrusions of any profile (also the
//! parametric steel sections), faceted and tessellated bodies, mapped
//! (shared) geometry, swept disks (pipes and cables), revolutions and CSG
//! primitives. Booleans keep their first operand (the rest only removes
//! material) and are marked as approximate; so are curved bodies
//! (NURBS), which fall back to their bounding box when the file has one.

use std::collections::HashMap;
use std::f64::consts::{PI, TAU};

use archisim_core::Point2;
use archisim_geom::solid::{triangulate, with_holes};
use archisim_ifc::import::geometry::{cross, dot, normalize, scale, sub, Geo, Xf, P3};
use archisim_ifc::step_read::{Entity, StepFile, Value};

pub type Tri = [P3; 3];

/// A profile in its own plane: outline and holes.
type Outline2 = (Vec<[f64; 2]>, Vec<Vec<[f64; 2]>>);

/// The body of one element.
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub triangles: Vec<Tri>,
    /// Part of the shape could not be read exactly (clippings, curved
    /// surfaces, unknown items).
    pub approximate: bool,
    /// Items that could not be read at all, by entity name.
    pub unsupported: Vec<String>,
}

impl Mesh {
    pub fn bounds(&self) -> Option<(P3, P3)> {
        bounds_of(self.triangles.iter().flatten().copied())
    }

    /// Enclosed volume: the sum of its solids' volumes (exact for closed
    /// meshes; overlapping solids count twice, as in IfcOpenShell).
    pub fn volume(&self) -> f64 {
        self.triangles
            .iter()
            .map(|[a, b, c]| dot(*a, cross(*b, *c)) / 6.0)
            .sum::<f64>()
            .abs()
    }
}

pub fn bounds_of(points: impl Iterator<Item = P3>) -> Option<(P3, P3)> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut any = false;
    for p in points {
        any = true;
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    any.then_some((lo, hi))
}

/// Reads the bodies of a file.
pub struct Mesher<'a> {
    pub geo: Geo<'a>,
}

impl<'a> Mesher<'a> {
    /// A mesher with the file's length and angle units.
    pub fn new(file: &'a StepFile) -> Mesher<'a> {
        let mut geo = Geo {
            file,
            length: 1.0,
            angle: 1.0,
        };
        if let Some(project) = file
            .of_kind("IFCPROJECT")
            .first()
            .and_then(|&p| file.get(p))
        {
            if let Some(assign) = file.deref(project.arg(8)) {
                for u in assign.arg(0).as_list() {
                    let Some(unit) = file.deref(u) else { continue };
                    let kind = unit.arg(1).as_enum().unwrap_or("");
                    if let Some(f) = unit_factor(file, u) {
                        match kind {
                            "LENGTHUNIT" => geo.length = f,
                            "PLANEANGLEUNIT" => geo.angle = f,
                            _ => {}
                        }
                    }
                }
            }
        }
        Mesher { geo }
    }

    fn ent(&self, v: &Value) -> Option<&'a Entity> {
        self.geo.file.deref(v)
    }

    /// The body of a product: its placement applied to its "Body"
    /// representation (or, failing that, any 3D one; a bounding box as a
    /// last resort).
    pub fn product(&self, e: &Entity) -> Mesh {
        let mut mesh = Mesh::default();
        let xf = self.geo.placement(e.arg(5));
        let Some(shape) = self.ent(e.arg(6)) else {
            return mesh;
        };
        let reps: Vec<&Entity> = shape
            .arg(2)
            .as_list()
            .iter()
            .filter_map(|r| self.ent(r))
            .collect();
        let id = |r: &Entity| r.arg(1).as_str().unwrap_or("").to_string();
        let ty = |r: &Entity| r.arg(2).as_str().unwrap_or("").to_string();
        let solid_types = [
            "SweptSolid",
            "AdvancedSweptSolid",
            "Brep",
            "AdvancedBrep",
            "SolidModel",
            "SurfaceModel",
            "Tessellation",
            "Clipping",
            "CSG",
            "MappedRepresentation",
        ];
        let chosen = reps
            .iter()
            .find(|r| id(r) == "Body")
            .or_else(|| reps.iter().find(|r| id(r) == "Facetation"))
            .or_else(|| {
                reps.iter()
                    .find(|r| solid_types.contains(&ty(r).as_str()) && id(r) != "Axis")
            })
            .or_else(|| {
                reps.iter()
                    .find(|r| ty(r) == "BoundingBox" || id(r) == "Box")
            });
        if let Some(rep) = chosen {
            for item in rep.arg(3).as_list() {
                self.item(item, &xf, &mut mesh, 0);
            }
        }
        // Curved bodies we cannot tessellate: their box, if there is one.
        if mesh.triangles.is_empty() {
            if let Some(b) = reps
                .iter()
                .find(|r| ty(r) == "BoundingBox" || id(r) == "Box")
            {
                for item in b.arg(3).as_list() {
                    self.item(item, &xf, &mut mesh, 0);
                }
                mesh.approximate = true;
            }
        }
        mesh
    }

    fn item(&self, v: &Value, xf: &Xf, mesh: &mut Mesh, depth: usize) {
        let start = mesh.triangles.len();
        self.item_raw(v, xf, mesh, depth);
        // Each solid wound outwards, whatever its placement (mirrored
        // mappings, extrusions downwards), so volumes add up.
        let new = &mut mesh.triangles[start..];
        if !closed(new) {
            // An open surface has no inside: clashes with it are guesses.
            mesh.approximate = true;
            return;
        }
        let signed: f64 = new
            .iter()
            .map(|[a, b, c]| dot(*a, cross(*b, *c)) / 6.0)
            .sum();
        if signed < 0.0 {
            for t in new.iter_mut() {
                t.swap(1, 2);
            }
        }
    }

    fn item_raw(&self, v: &Value, xf: &Xf, mesh: &mut Mesh, depth: usize) {
        let Some(e) = self.ent(v) else { return };
        if depth > 24 {
            return;
        }
        match e.kind.as_str() {
            "IFCEXTRUDEDAREASOLID" | "IFCEXTRUDEDAREASOLIDTAPERED" => {
                let Some((outer, holes)) = self.profile(e.arg(0)) else {
                    mesh.unsupported.push(
                        self.ent(e.arg(0))
                            .map_or_else(|| "perfil".into(), |p| p.kind.clone()),
                    );
                    return;
                };
                let pos = xf.then(&self.geo.axis_placement(e.arg(1)));
                let dir = self.geo.direction(e.arg(2)).unwrap_or([0.0, 0.0, 1.0]);
                let d = e.arg(3).as_f64().unwrap_or(0.0) * self.geo.length;
                extrude(&outer, &holes, &pos, scale(dir, d), mesh);
            }
            "IFCREVOLVEDAREASOLID" => {
                let Some((outer, _)) = self.profile(e.arg(0)) else {
                    mesh.unsupported.push("perfil de revolución".into());
                    return;
                };
                let pos = xf.then(&self.geo.axis_placement(e.arg(1)));
                let Some(axis) = self.ent(e.arg(2)) else {
                    return;
                };
                let origin = self.geo.point(axis.arg(0)).unwrap_or([0.0; 3]);
                let adir = self.geo.direction(axis.arg(1)).unwrap_or([0.0, 1.0, 0.0]);
                let angle = e.arg(3).as_f64().unwrap_or(TAU) * self.geo.angle;
                revolve(&outer, origin, adir, angle, &pos, mesh);
            }
            "IFCSWEPTDISKSOLID" | "IFCSWEPTDISKSOLIDPOLYGONAL" => {
                let mut curved = false;
                let Some(path) = self.geo.curve(e.arg(0), &mut curved) else {
                    mesh.unsupported.push("eje de tubo".into());
                    return;
                };
                let r = e.arg(1).as_f64().unwrap_or(0.0) * self.geo.length;
                let path: Vec<P3> = path.iter().map(|&p| xf.apply(p)).collect();
                tube(&path, r, mesh);
            }
            "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => {
                let op = e.arg(0).as_enum().unwrap_or("DIFFERENCE");
                self.item(e.arg(1), xf, mesh, depth + 1);
                match op {
                    "UNION" => self.item(e.arg(2), xf, mesh, depth + 1),
                    _ => mesh.approximate = true,
                }
            }
            "IFCCSGSOLID" => self.item(e.arg(0), xf, mesh, depth + 1),
            "IFCBLOCK" => {
                let pos = xf.then(&self.geo.axis_placement(e.arg(0)));
                let k = self.geo.length;
                let d = [1, 2, 3].map(|i| e.arg(i).as_f64().unwrap_or(0.0) * k);
                block(&pos, [0.0; 3], d, mesh);
            }
            "IFCRIGHTCIRCULARCYLINDER" => {
                let pos = xf.then(&self.geo.axis_placement(e.arg(0)));
                let h = e.arg(1).as_f64().unwrap_or(0.0) * self.geo.length;
                let r = e.arg(2).as_f64().unwrap_or(0.0) * self.geo.length;
                extrude(&circle(r, 16), &[], &pos, [0.0, 0.0, h], mesh);
            }
            "IFCFACETEDBREP"
            | "IFCFACETEDBREPWITHVOIDS"
            | "IFCADVANCEDBREP"
            | "IFCADVANCEDBREPWITHVOIDS" => {
                self.shell(e.arg(0), xf, mesh);
            }
            "IFCSHELLBASEDSURFACEMODEL" | "IFCFACEBASEDSURFACEMODEL" => {
                for s in e.arg(0).as_list() {
                    self.shell(s, xf, mesh);
                }
            }
            "IFCCLOSEDSHELL" | "IFCOPENSHELL" | "IFCCONNECTEDFACESET" => {
                self.shell(v, xf, mesh);
            }
            "IFCTRIANGULATEDFACESET" | "IFCPOLYGONALFACESET" => self.face_set(e, xf, mesh),
            "IFCMAPPEDITEM" => {
                let Some(source) = self.ent(e.arg(0)) else {
                    return;
                };
                let origin = self.geo.axis_placement(source.arg(0));
                let target = self.operator(e.arg(1));
                let inner = xf.then(&target).then(&origin);
                if let Some(rep) = self.ent(source.arg(1)) {
                    for it in rep.arg(3).as_list() {
                        self.item(it, &inner, mesh, depth + 1);
                    }
                }
            }
            "IFCBOUNDINGBOX" => {
                let Some(c) = self.geo.point(e.arg(0)) else {
                    return;
                };
                let k = self.geo.length;
                let d = [1, 2, 3].map(|i| e.arg(i).as_f64().unwrap_or(0.0) * k);
                block(xf, c, d, mesh);
            }
            "IFCSTYLEDITEM" => {}
            // Lines, text and points have no volume.
            "IFCPOLYLINE"
            | "IFCINDEXEDPOLYCURVE"
            | "IFCTRIMMEDCURVE"
            | "IFCCOMPOSITECURVE"
            | "IFCCARTESIANPOINT"
            | "IFCGEOMETRICSET"
            | "IFCGEOMETRICCURVESET"
            | "IFCTEXTLITERAL"
            | "IFCTEXTLITERALWITHEXTENT"
            | "IFCANNOTATIONFILLAREA" => {}
            other => {
                mesh.approximate = true;
                mesh.unsupported.push(other.to_string());
            }
        }
    }

    fn operator(&self, v: &Value) -> Xf {
        let Some(e) = self.ent(v) else {
            return Xf::IDENTITY;
        };
        let origin = self.geo.point(e.arg(2)).unwrap_or([0.0; 3]);
        let s = e.arg(3).as_f64().unwrap_or(1.0);
        let x = self.geo.direction(e.arg(0)).unwrap_or([1.0, 0.0, 0.0]);
        let (y, z) = if e.kind.starts_with("IFCCARTESIANTRANSFORMATIONOPERATOR3D") {
            let z = self.geo.direction(e.arg(4)).unwrap_or_else(|| {
                let y = self.geo.direction(e.arg(1)).unwrap_or([0.0, 1.0, 0.0]);
                normalize(cross(x, y)).unwrap_or([0.0, 0.0, 1.0])
            });
            let y = self.geo.direction(e.arg(1)).unwrap_or_else(|| cross(z, x));
            (y, z)
        } else {
            let y = self.geo.direction(e.arg(1)).unwrap_or([-x[1], x[0], 0.0]);
            (y, [0.0, 0.0, 1.0])
        };
        Xf {
            x: scale(x, s),
            y: scale(y, s),
            z: scale(z, s),
            origin,
        }
    }

    /// Faces with their holes, each triangulated in its own plane.
    fn shell(&self, v: &Value, xf: &Xf, mesh: &mut Mesh) {
        let Some(shell) = self.ent(v) else { return };
        for face in shell.arg(0).as_list() {
            let Some(face) = self.ent(face) else { continue };
            let mut outer: Option<Vec<P3>> = None;
            let mut holes: Vec<Vec<P3>> = Vec::new();
            for bound in face.arg(0).as_list() {
                let Some(bound) = self.ent(bound) else {
                    continue;
                };
                let Some(lp) = self.ent(bound.arg(0)) else {
                    continue;
                };
                if lp.kind != "IFCPOLYLOOP" {
                    // Edge loops of curved faces: not read.
                    mesh.approximate = true;
                    continue;
                }
                let mut pts: Vec<P3> = lp
                    .arg(0)
                    .as_list()
                    .iter()
                    .filter_map(|p| self.geo.point(p))
                    .map(|p| xf.apply(p))
                    .collect();
                if bound.arg(1).as_bool() == Some(false) {
                    pts.reverse();
                }
                if pts.len() < 3 {
                    continue;
                }
                if bound.kind == "IFCFACEOUTERBOUND" && outer.is_none() {
                    outer = Some(pts);
                } else {
                    holes.push(pts);
                }
            }
            // A face with only inner bounds: the largest is the outline.
            if outer.is_none() && !holes.is_empty() {
                let i = (0..holes.len())
                    .max_by(|&a, &b| area_len(&holes[a]).total_cmp(&area_len(&holes[b])))
                    .unwrap_or(0);
                outer = Some(holes.swap_remove(i));
            }
            if let Some(o) = outer {
                planar_face(&o, &holes, mesh);
            }
        }
    }

    fn face_set(&self, e: &Entity, xf: &Xf, mesh: &mut Mesh) {
        let Some(list) = self.ent(e.arg(0)) else {
            return;
        };
        let k = self.geo.length;
        let coords: Vec<P3> = list
            .arg(0)
            .as_list()
            .iter()
            .map(|c| {
                let n = c.numbers();
                let at = |i: usize| n.get(i).copied().unwrap_or(0.0) * k;
                xf.apply([at(0), at(1), at(2)])
            })
            .collect();
        let tri = e.kind == "IFCTRIANGULATEDFACESET";
        let remap = if tri {
            e.arg(4).numbers()
        } else {
            e.arg(3).numbers()
        };
        let point = |i: f64| -> Option<P3> {
            let i = i as usize;
            let i = if remap.is_empty() {
                i
            } else {
                *remap.get(i.checked_sub(1)?)? as usize
            };
            coords.get(i.checked_sub(1)?).copied()
        };
        if tri {
            for f in e.arg(3).as_list() {
                let n = f.numbers();
                if let (Some(a), Some(b), Some(c)) = (
                    n.first().and_then(|&i| point(i)),
                    n.get(1).and_then(|&i| point(i)),
                    n.get(2).and_then(|&i| point(i)),
                ) {
                    mesh.triangles.push([a, b, c]);
                }
            }
        } else {
            for face in e.arg(2).as_list() {
                let Some(face) = self.ent(face) else { continue };
                let outer: Vec<P3> = face
                    .arg(0)
                    .numbers()
                    .iter()
                    .filter_map(|&i| point(i))
                    .collect();
                let holes: Vec<Vec<P3>> = face
                    .arg(1)
                    .as_list()
                    .iter()
                    .map(|h| h.numbers().iter().filter_map(|&i| point(i)).collect())
                    .collect();
                if outer.len() >= 3 {
                    planar_face(&outer, &holes, mesh);
                }
            }
        }
    }

    /// A profile's outline and holes in its plane (z = 0), in metres:
    /// those `Geo` reads, plus the parametric sections of structural
    /// steel and hollow sections.
    fn profile(&self, v: &Value) -> Option<(Vec<P3>, Vec<Vec<P3>>)> {
        let e = self.ent(v)?;
        let k = self.geo.length;
        let n = |i: usize| e.arg(i).as_f64().map(|x| x * k);
        let place = |pts: Vec<[f64; 2]>| -> Vec<P3> {
            let xf = self.geo.axis_placement(e.arg(2));
            pts.into_iter()
                .map(|[x, y]| xf.apply([x, y, 0.0]))
                .collect()
        };
        let shape: Option<Outline2> = match e.kind.as_str() {
            "IFCISHAPEPROFILEDEF" | "IFCASYMMETRICISHAPEPROFILEDEF" => {
                let (b, h, tw, tf) = (n(3)?, n(4)?, n(5)?, n(6)?);
                let (x, y, w) = (b / 2.0, h / 2.0, tw / 2.0);
                Some((
                    vec![
                        [-x, -y],
                        [x, -y],
                        [x, -y + tf],
                        [w, -y + tf],
                        [w, y - tf],
                        [x, y - tf],
                        [x, y],
                        [-x, y],
                        [-x, y - tf],
                        [-w, y - tf],
                        [-w, -y + tf],
                        [-x, -y + tf],
                    ],
                    vec![],
                ))
            }
            "IFCLSHAPEPROFILEDEF" => {
                let (h, b, t) = (n(3)?, n(4).or(n(3))?, n(5)?);
                let (cx, cy) = (b / 2.0, h / 2.0);
                Some((
                    vec![
                        [-cx, -cy],
                        [cx, -cy],
                        [cx, -cy + t],
                        [-cx + t, -cy + t],
                        [-cx + t, cy],
                        [-cx, cy],
                    ],
                    vec![],
                ))
            }
            "IFCTSHAPEPROFILEDEF" => {
                let (h, b, tw, tf) = (n(3)?, n(4)?, n(5)?, n(6)?);
                let (x, y, w) = (b / 2.0, h / 2.0, tw / 2.0);
                Some((
                    vec![
                        [-w, -y],
                        [w, -y],
                        [w, y - tf],
                        [x, y - tf],
                        [x, y],
                        [-x, y],
                        [-x, y - tf],
                        [-w, y - tf],
                    ],
                    vec![],
                ))
            }
            "IFCUSHAPEPROFILEDEF" => {
                let (h, b, tw, tf) = (n(3)?, n(4)?, n(5)?, n(6)?);
                let (x, y) = (b / 2.0, h / 2.0);
                Some((
                    vec![
                        [-x, -y],
                        [x, -y],
                        [x, -y + tf],
                        [-x + tw, -y + tf],
                        [-x + tw, y - tf],
                        [x, y - tf],
                        [x, y],
                        [-x, y],
                    ],
                    vec![],
                ))
            }
            "IFCCSHAPEPROFILEDEF" => {
                let (h, b, t, g) = (n(3)?, n(4)?, n(5)?, n(6).unwrap_or(0.0));
                let (x, y) = (b / 2.0, h / 2.0);
                let g = g.max(t);
                Some((
                    vec![
                        [-x, -y],
                        [x, -y],
                        [x, -y + g],
                        [x - t, -y + g],
                        [x - t, -y + t],
                        [-x + t, -y + t],
                        [-x + t, y - t],
                        [x - t, y - t],
                        [x - t, y - g],
                        [x, y - g],
                        [x, y],
                        [-x, y],
                    ],
                    vec![],
                ))
            }
            "IFCZSHAPEPROFILEDEF" => {
                let (h, b, tw, tf) = (n(3)?, n(4)?, n(5)?, n(6)?);
                let (y, w) = (h / 2.0, tw / 2.0);
                Some((
                    vec![
                        [-w, -y],
                        [b - w, -y],
                        [b - w, -y + tf],
                        [w, -y + tf],
                        [w, y],
                        [-b + w, y],
                        [-b + w, y - tf],
                        [-w, y - tf],
                    ],
                    vec![],
                ))
            }
            "IFCRECTANGLEHOLLOWPROFILEDEF" => {
                let (bx, by, t) = (n(3)? / 2.0, n(4)? / 2.0, n(5)?);
                Some((
                    vec![[-bx, -by], [bx, -by], [bx, by], [-bx, by]],
                    vec![vec![
                        [-bx + t, -by + t],
                        [bx - t, -by + t],
                        [bx - t, by - t],
                        [-bx + t, by - t],
                    ]],
                ))
            }
            "IFCCIRCLEHOLLOWPROFILEDEF" => {
                let (r, t) = (n(3)?, n(4)?);
                let c = |r: f64| -> Vec<[f64; 2]> {
                    circle(r, 16).iter().map(|p| [p[0], p[1]]).collect()
                };
                Some((c(r), vec![c(r - t)]))
            }
            "IFCELLIPSEPROFILEDEF" => {
                let (a, b) = (n(3)?, n(4)?);
                Some((
                    (0..16)
                        .map(|i| {
                            let t = TAU * i as f64 / 16.0;
                            [a * t.cos(), b * t.sin()]
                        })
                        .collect(),
                    vec![],
                ))
            }
            "IFCTRAPEZIUMPROFILEDEF" => {
                let (bot, top, h, off) = (n(3)?, n(4)?, n(5)?, n(6).unwrap_or(0.0));
                let (x, y) = (bot / 2.0, h / 2.0);
                Some((
                    vec![[-x, -y], [x, -y], [-x + off + top, y], [-x + off, y]],
                    vec![],
                ))
            }
            _ => None,
        };
        if let Some((outer, holes)) = shape {
            return Some((place(outer), holes.into_iter().map(place).collect()));
        }
        if e.kind == "IFCCOMPOSITEPROFILEDEF" {
            // Take the largest part; the others are reported as approximate.
            let mut best: Option<(Vec<P3>, Vec<Vec<P3>>)> = None;
            for p in e.arg(2).as_list() {
                if let Some(x) = self.profile(p) {
                    if best
                        .as_ref()
                        .is_none_or(|b| area_len(&x.0) > area_len(&b.0))
                    {
                        best = Some(x);
                    }
                }
            }
            return best;
        }
        self.geo.profile(v)
    }
}

fn unit_factor(file: &StepFile, v: &Value) -> Option<f64> {
    let u = file.deref(v)?;
    match u.kind.as_str() {
        "IFCSIUNIT" => Some(match u.arg(2).as_enum() {
            Some("KILO") => 1e3,
            Some("HECTO") => 1e2,
            Some("DECA") => 1e1,
            Some("DECI") => 1e-1,
            Some("CENTI") => 1e-2,
            Some("MILLI") => 1e-3,
            Some("MICRO") => 1e-6,
            _ => 1.0,
        }),
        "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
            let m = file.deref(u.arg(3))?;
            Some(m.arg(0).as_f64()? * unit_factor(file, m.arg(1)).unwrap_or(1.0))
        }
        _ => None,
    }
}

/// Whether a set of triangles closes a volume: the area vectors of a
/// closed surface add up to nothing (also with T-junctions, which a test
/// on shared edges would reject).
fn closed(tris: &[Tri]) -> bool {
    let mut sum = [0.0; 3];
    let mut total = 0.0;
    for [a, b, c] in tris {
        let n = cross(sub(*b, *a), sub(*c, *a));
        for k in 0..3 {
            sum[k] += n[k];
        }
        total += dot(n, n).sqrt();
    }
    total > 0.0 && dot(sum, sum).sqrt() <= total * 1e-6
}

fn circle(r: f64, n: usize) -> Vec<P3> {
    (0..n)
        .map(|i| {
            let a = TAU * i as f64 / n as f64;
            [r * a.cos(), r * a.sin(), 0.0]
        })
        .collect()
}

/// Rough size of a loop, to pick the largest.
fn area_len(pts: &[P3]) -> f64 {
    let n = pts.len();
    let mut a = [0.0; 3];
    for i in 0..n {
        let c = cross(pts[i], pts[(i + 1) % n]);
        for k in 0..3 {
            a[k] += c[k];
        }
    }
    dot(a, a).sqrt()
}

/// Normal of a loop (Newell), not normalised.
fn newell(pts: &[P3]) -> P3 {
    let mut n = [0.0; 3];
    let k = pts.len();
    for i in 0..k {
        let a = pts[i];
        let b = pts[(i + 1) % k];
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    n
}

/// Triangulates a planar face with holes, in its own plane.
fn planar_face(outer: &[P3], holes: &[Vec<P3>], mesh: &mut Mesh) {
    if outer.len() == 3 && holes.is_empty() {
        mesh.triangles.push([outer[0], outer[1], outer[2]]);
        return;
    }
    let Some(nrm) = normalize(newell(outer)) else {
        return;
    };
    // A frame in the plane.
    let u = normalize(cross(
        if nrm[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        },
        nrm,
    ))
    .unwrap_or([1.0, 0.0, 0.0]);
    let w = cross(nrm, u);
    let mut back: HashMap<(u64, u64), P3> = HashMap::new();
    let flat = |pts: &[P3], back: &mut HashMap<(u64, u64), P3>| -> Vec<Point2> {
        pts.iter()
            .map(|&p| {
                let q = Point2::new(dot(p, u), dot(p, w));
                back.insert((q.x.to_bits(), q.y.to_bits()), p);
                q
            })
            .collect()
    };
    let o2 = flat(outer, &mut back);
    let h2: Vec<Vec<Point2>> = holes.iter().map(|h| flat(h, &mut back)).collect();
    let poly = if h2.is_empty() {
        archisim_geom::ensure_ccw(o2)
    } else {
        with_holes(&o2, &h2)
    };
    let lift = |q: Point2| -> P3 {
        back.get(&(q.x.to_bits(), q.y.to_bits()))
            .copied()
            .unwrap_or_else(|| {
                // On the plane through the first point.
                let d = dot(outer[0], nrm);
                [0, 1, 2].map(|i| u[i] * q.x + w[i] * q.y + nrm[i] * d)
            })
    };
    for [a, b, c] in triangulate(&poly) {
        mesh.triangles
            .push([lift(poly[a]), lift(poly[b]), lift(poly[c])]);
    }
}

/// A prism: the profile (with holes) in the plane of `pos`, moved along
/// `shift` (in the coordinates of `pos`).
fn extrude(outer: &[P3], holes: &[Vec<P3>], pos: &Xf, shift: P3, mesh: &mut Mesh) {
    if outer.len() < 3 {
        return;
    }
    let to2 = |pts: &[P3]| -> Vec<Point2> { pts.iter().map(|p| Point2::new(p[0], p[1])).collect() };
    let o2 = archisim_geom::ensure_ccw(to2(outer));
    let h2: Vec<Vec<Point2>> = holes.iter().map(|h| to2(h)).collect();
    let poly = if h2.is_empty() {
        o2.clone()
    } else {
        with_holes(&o2, &h2)
    };
    let at = |p: Point2, top: bool| -> P3 {
        let local = if top {
            [p.x + shift[0], p.y + shift[1], shift[2]]
        } else {
            [p.x, p.y, 0.0]
        };
        pos.apply(local)
    };
    for [a, b, c] in triangulate(&poly) {
        mesh.triangles
            .push([at(poly[a], true), at(poly[b], true), at(poly[c], true)]);
        mesh.triangles
            .push([at(poly[a], false), at(poly[c], false), at(poly[b], false)]);
    }
    let mut rings = vec![o2];
    for h in h2 {
        let mut h = archisim_geom::ensure_ccw(h);
        h.reverse();
        rings.push(h);
    }
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (p, q) = (ring[i], ring[(i + 1) % n]);
            if p.approx_eq(q) {
                continue;
            }
            mesh.triangles
                .push([at(p, false), at(q, false), at(q, true)]);
            mesh.triangles
                .push([at(p, false), at(q, true), at(p, true)]);
        }
    }
}

/// A box from `corner` with sizes `d`, in the coordinates of `xf`.
fn block(xf: &Xf, corner: P3, d: P3, mesh: &mut Mesh) {
    let base: Vec<P3> = vec![
        [corner[0], corner[1], 0.0],
        [corner[0] + d[0], corner[1], 0.0],
        [corner[0] + d[0], corner[1] + d[1], 0.0],
        [corner[0], corner[1] + d[1], 0.0],
    ];
    let pos = xf.then(&Xf {
        origin: [0.0, 0.0, corner[2]],
        ..Xf::IDENTITY
    });
    extrude(&base, &[], &pos, [0.0, 0.0, d[2]], mesh);
}

/// A closed tube of radius `r` along a polyline.
fn tube(path: &[P3], r: f64, mesh: &mut Mesh) {
    if path.len() < 2 || r <= 0.0 {
        return;
    }
    const N: usize = 12;
    let ring = |p: P3, dir: P3| -> Vec<P3> {
        let a = normalize(cross(
            if dir[2].abs() < 0.9 {
                [0.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0]
            },
            dir,
        ))
        .unwrap_or([1.0, 0.0, 0.0]);
        let b = cross(dir, a);
        (0..N)
            .map(|i| {
                let t = TAU * i as f64 / N as f64;
                [0, 1, 2].map(|k| p[k] + r * (a[k] * t.cos() + b[k] * t.sin()))
            })
            .collect()
    };
    for w in path.windows(2) {
        let Some(dir) = normalize(sub(w[1], w[0])) else {
            continue;
        };
        let r0 = ring(w[0], dir);
        let r1 = ring(w[1], dir);
        for i in 0..N {
            let j = (i + 1) % N;
            mesh.triangles.push([r0[i], r0[j], r1[j]]);
            mesh.triangles.push([r0[i], r1[j], r1[i]]);
        }
        // Each piece closed on its own: overlapping pieces at the bends
        // still enclose the pipe.
        for k in 1..N - 1 {
            mesh.triangles.push([r0[0], r0[k + 1], r0[k]]);
            mesh.triangles.push([r1[0], r1[k], r1[k + 1]]);
        }
    }
}

/// A profile turned around an axis (in the profile's plane).
fn revolve(outer: &[P3], origin: P3, axis: P3, angle: f64, pos: &Xf, mesh: &mut Mesh) {
    let steps = ((angle.abs() / TAU * 24.0).ceil() as usize).max(2);
    let rot = |p: P3, t: f64| -> P3 {
        // Rodrigues around `axis` through `origin`.
        let v = sub(p, origin);
        let (s, c) = t.sin_cos();
        let kxv = cross(axis, v);
        let kdv = dot(axis, v);
        let r = [0, 1, 2].map(|i| v[i] * c + kxv[i] * s + axis[i] * kdv * (1.0 - c));
        pos.apply([r[0] + origin[0], r[1] + origin[1], r[2] + origin[2]])
    };
    let n = outer.len();
    for s in 0..steps {
        let t0 = angle * s as f64 / steps as f64;
        let t1 = angle * (s + 1) as f64 / steps as f64;
        for i in 0..n {
            let (p, q) = (outer[i], outer[(i + 1) % n]);
            let (a, b, c, d) = (rot(p, t0), rot(q, t0), rot(q, t1), rot(p, t1));
            mesh.triangles.push([a, b, c]);
            mesh.triangles.push([a, c, d]);
        }
    }
    if (angle.abs() - TAU).abs() > 1e-6 {
        // Ends of a partial revolution.
        let mut start = Mesh::default();
        let ends: Vec<P3> = outer.iter().map(|&p| rot(p, 0.0)).collect();
        planar_face(&ends, &[], &mut start);
        let ends: Vec<P3> = outer.iter().map(|&p| rot(p, angle)).collect();
        planar_face(&ends, &[], &mut start);
        mesh.triangles.extend(start.triangles);
    }
    let _ = PI;
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_ifc::step_read::parse;

    fn file(data: &str) -> StepFile {
        parse(&format!(
            "ISO-10303-21;HEADER;FILE_SCHEMA(('IFC4'));ENDSEC;DATA;{data}ENDSEC;END-ISO-10303-21;"
        ))
        .unwrap()
    }

    const UNITS_MM: &str = "#1=IFCPROJECT('p',$,$,$,$,$,$,$,#2);#2=IFCUNITASSIGNMENT((#3));\
        #3=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);\
        #10=IFCCARTESIANPOINT((0.,0.,0.));#11=IFCAXIS2PLACEMENT3D(#10,$,$);\
        #12=IFCLOCALPLACEMENT($,#11);#13=IFCDIRECTION((0.,0.,1.));";

    fn product(data: &str) -> Mesh {
        let f = file(&format!("{UNITS_MM}{data}"));
        let m = Mesher::new(&f);
        let id = f.of_kind("IFCBEAM")[0];
        m.product(f.get(id).unwrap())
    }

    #[test]
    fn ipe_beam_in_millimetres() {
        // IPE 300, 6 m long.
        let mesh = product(
            "#20=IFCISHAPEPROFILEDEF(.AREA.,'IPE300',$,150.,300.,7.1,10.7,15.,$,$);\
             #21=IFCEXTRUDEDAREASOLID(#20,#11,#13,6000.);\
             #22=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#21));\
             #23=IFCPRODUCTDEFINITIONSHAPE($,$,(#22));\
             #30=IFCBEAM('b',$,'IPE',$,$,#12,#23,$,$);",
        );
        // Area without root fillets: 2·150·10.7 + (300 − 21.4)·7.1 mm².
        let area = (2.0 * 150.0 * 10.7 + (300.0 - 21.4) * 7.1) * 1e-6;
        assert!(
            (mesh.volume() - area * 6.0).abs() < 1e-9,
            "{}",
            mesh.volume()
        );
        let (lo, hi) = mesh.bounds().unwrap();
        assert!((hi[2] - lo[2] - 6.0).abs() < 1e-9);
        assert!(!mesh.approximate);
    }

    #[test]
    fn pipe_and_hollow_section() {
        let pipe = product(
            "#20=IFCPOLYLINE((#10,#21));#21=IFCCARTESIANPOINT((4000.,0.,0.));\
             #22=IFCSWEPTDISKSOLID(#20,50.,$,$,$);\
             #23=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#22));\
             #24=IFCPRODUCTDEFINITIONSHAPE($,$,(#23));\
             #30=IFCBEAM('b',$,'tubo',$,$,#12,#24,$,$);",
        );
        // A 12-sided prism: a little less than the circle.
        let v = pipe.volume();
        let exact = PI * 0.05 * 0.05 * 4.0;
        assert!(v < exact && v > exact * 0.95, "{v}");
        let hollow = product(
            "#20=IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,$,$,200.,100.,10.,$,$);\
             #21=IFCEXTRUDEDAREASOLID(#20,#11,#13,1000.);\
             #22=IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#21));\
             #23=IFCPRODUCTDEFINITIONSHAPE($,$,(#22));\
             #30=IFCBEAM('b',$,'RHS',$,$,#12,#23,$,$);",
        );
        let expect = (0.2 * 0.1 - 0.18 * 0.08) * 1.0;
        assert!(
            (hollow.volume() - expect).abs() < 1e-9,
            "{}",
            hollow.volume()
        );
    }

    #[test]
    fn brep_face_with_hole_and_mapped_item() {
        // A unit cube as a faceted brep, used twice through a mapping.
        let mut d = String::from(UNITS_MM);
        let pts = [
            [0, 0, 0],
            [1000, 0, 0],
            [1000, 1000, 0],
            [0, 1000, 0],
            [0, 0, 1000],
            [1000, 0, 1000],
            [1000, 1000, 1000],
            [0, 1000, 1000],
        ];
        for (i, p) in pts.iter().enumerate() {
            d += &format!(
                "#{}=IFCCARTESIANPOINT(({}.,{}.,{}.));",
                100 + i,
                p[0],
                p[1],
                p[2]
            );
        }
        let faces = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ];
        let mut face_ids = Vec::new();
        for (i, f) in faces.iter().enumerate() {
            let id = 200 + i * 3;
            d += &format!(
                "#{}=IFCPOLYLOOP((#{},#{},#{},#{}));#{}=IFCFACEOUTERBOUND(#{},.T.);#{}=IFCFACE((#{}));",
                id,
                100 + f[0],
                100 + f[1],
                100 + f[2],
                100 + f[3],
                id + 1,
                id,
                id + 2,
                id + 1
            );
            face_ids.push(format!("#{}", id + 2));
        }
        d += &format!(
            "#300=IFCCLOSEDSHELL(({}));#301=IFCFACETEDBREP(#300);\
             #302=IFCSHAPEREPRESENTATION($,'Body','Brep',(#301));\
             #303=IFCREPRESENTATIONMAP(#11,#302);\
             #304=IFCCARTESIANPOINT((5000.,0.,0.));\
             #305=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#304,$,$);\
             #306=IFCMAPPEDITEM(#303,#305);\
             #307=IFCSHAPEREPRESENTATION($,'Body','MappedRepresentation',(#306));\
             #308=IFCPRODUCTDEFINITIONSHAPE($,$,(#307));\
             #30=IFCBEAM('b',$,'caja',$,$,#12,#308,$,$);",
            face_ids.join(",")
        );
        let mesh = product(&d[UNITS_MM.len()..]);
        assert_eq!(mesh.triangles.len(), 12);
        assert!((mesh.volume() - 1.0).abs() < 1e-9);
        let (lo, _) = mesh.bounds().unwrap();
        assert!((lo[0] - 5.0).abs() < 1e-9);
    }
}
