//! Earthworks as solids (bloque Terreno): what the platforms of a terrain
//! dig ("desmonte") and fill ("terraplén"), and what pools dig, each a
//! closed triangulated solid with its volume, for IFC 4.3
//! (`IfcEarthworksCut`, `IfcEarthworksFill`), the measurements and the
//! earthworks report; and the banks of each platform in plan (the line
//! where the bank meets the ground, its top and its foot).
//!
//! Nothing here is kept in the model: it is worked out from the ground as
//! surveyed and as graded (`Terrain::ground`, `Terrain::points`) on a
//! grid of `EARTHWORKS_STEP` (coarser on big sites).

use super::*;

/// Less than this (m³) is no earthwork (the noise of the grid where a
/// bank meets the ground).
pub const MIN_EARTH_VOLUME: f64 = 0.05;

/// Less than this (m) is no change of the ground.
const THIN: f64 = 1e-3;

/// A closed solid of triangles (outward, counter-clockwise seen from
/// outside), in absolute coordinates, with what it measures.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EarthMesh {
    pub points: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    /// m³.
    pub volume: f64,
    /// Area in plan it covers, m².
    pub area: f64,
    /// Its greatest depth (cut) or height (fill), m.
    pub depth: f64,
    /// Its extent in plan.
    pub lo: Point2,
    pub hi: Point2,
}

impl EarthMesh {
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty() || self.volume < MIN_EARTH_VOLUME
    }

    /// The volume of its triangles (divergence theorem): to check that it
    /// is closed and agrees with `volume`.
    pub fn solid_volume(&self) -> f64 {
        let p = &self.points;
        let o = p.first().copied().unwrap_or([0.0; 3]);
        self.triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| [p[i][0] - o[0], p[i][1] - o[1], p[i][2] - o[2]]);
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum()
    }

    /// Edges used by other than two triangles (none if it is closed).
    pub fn open_edges(&self) -> usize {
        let mut count: std::collections::HashMap<(usize, usize), i32> = Default::default();
        for t in &self.triangles {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *count.entry((a, b)).or_default() += 1;
            }
        }
        count
            .iter()
            .filter(|((a, b), n)| **n != 1 || count.get(&(*b, *a)) != Some(&1))
            .count()
    }
}

/// Dug or filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarthSide {
    Cut,
    Fill,
}

/// What an earthwork is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarthUse {
    /// A platform of a terrain, of this kind.
    Pad(PadKind),
    /// The dig of a pool.
    Pool,
}

/// Where an earthwork comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarthSource {
    /// Platform `pad` of a terrain.
    Pad { terrain: TerrainId, pad: usize },
    /// A pool built in place.
    Pool(PoolId),
    /// A pool that is an object of the library.
    Object(ObjectId),
}

/// One earthwork of the project: a platform's cut or fill, or a pool's dig.
#[derive(Debug, Clone, PartialEq)]
pub struct Earthwork {
    pub source: EarthSource,
    /// The identifier it is derived from (the terrain's, the pool's).
    pub owner: GlobalId,
    /// What tells it apart from the others of its owner («pad-0-cut»).
    pub key: String,
    pub name: String,
    pub side: EarthSide,
    pub what: EarthUse,
    /// The terrain it digs or fills.
    pub terrain: TerrainId,
    pub mesh: EarthMesh,
}

/// What one platform digs and fills.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PadEarthworks {
    pub cut: EarthMesh,
    pub fill: EarthMesh,
}

/// A mark of a platform's bank in plan, from the platform's edge outward
/// to where the bank meets the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BankMark {
    /// On the platform's edge.
    pub edge: Point2,
    /// Where the bank meets the ground.
    pub ground: Point2,
    /// The bank goes up from the edge (a cut): its top ("cabeza") is
    /// `ground`; otherwise (a fill) its top is `edge` and its foot `ground`.
    pub cut: bool,
}

impl BankMark {
    /// Top and foot of the bank.
    pub fn top_foot(&self) -> (Point2, Point2) {
        if self.cut {
            (self.ground, self.edge)
        } else {
            (self.edge, self.ground)
        }
    }
}

/// How much the ground goes up (fill, positive) or down (cut) at the
/// nodes of a grid from `lo`, `step` apart, `nx` by `ny` (by rows).
#[derive(Debug, Clone, PartialEq)]
pub struct EarthMap {
    pub lo: Point2,
    pub step: f64,
    pub nx: usize,
    pub ny: usize,
    pub change: Vec<Option<f64>>,
}

impl EarthMap {
    /// The change in the middle of cell `(i, j)` (the mean of its
    /// corners), if the ground is there.
    pub fn cell(&self, i: usize, j: usize) -> Option<f64> {
        let k = j * self.nx + i;
        let v = [
            self.change[k]?,
            self.change[k + 1]?,
            self.change[k + self.nx]?,
            self.change[k + self.nx + 1]?,
        ];
        Some(v.iter().sum::<f64>() / 4.0)
    }
}

/// What the plan draws of a terrain's platforms (`Terrain::earth_plan`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EarthPlan {
    pub banks: Vec<Vec<BankMark>>,
    pub map: Option<EarthMap>,
    /// Empty unless asked for.
    pub volumes: Vec<PadEarthworks>,
}

fn map_of(g: &Grid) -> EarthMap {
    EarthMap {
        lo: g.lo,
        step: g.step,
        nx: g.nx,
        ny: g.ny,
        change: g
            .before
            .iter()
            .zip(&g.after)
            .map(|(b, a)| Some((*a)? - (*b)?))
            .collect(),
    }
}

/// The ground before and after, at the nodes of a grid.
struct Grid {
    lo: Point2,
    step: f64,
    nx: usize,
    ny: usize,
    before: Vec<Option<f64>>,
    after: Vec<Option<f64>>,
}

impl Grid {
    fn at(&self, i: usize, j: usize) -> Point2 {
        Point2::new(
            self.lo.x + i as f64 * self.step,
            self.lo.y + j as f64 * self.step,
        )
    }

    /// After minus before at `p`, between the nodes round it.
    fn change_at(&self, p: Point2) -> Option<f64> {
        let (b, a) = self.both_at(p)?;
        Some(a - b)
    }

    /// Before and after at `p`, between the nodes round it.
    fn both_at(&self, p: Point2) -> Option<(f64, f64)> {
        let b = self.interpolate(&self.before, p)?;
        let a = self.interpolate(&self.after, p)?;
        Some((b, a))
    }

    fn interpolate(&self, values: &[Option<f64>], p: Point2) -> Option<f64> {
        let fx = (p.x - self.lo.x) / self.step;
        let fy = (p.y - self.lo.y) / self.step;
        if fx < 0.0 || fy < 0.0 {
            return None;
        }
        let (i, j) = (fx.floor() as usize, fy.floor() as usize);
        if i + 1 >= self.nx || j + 1 >= self.ny {
            return None;
        }
        let (u, v) = (fx - i as f64, fy - j as f64);
        let d = |i: usize, j: usize| values[j * self.nx + i];
        let (a, b, c, e) = (d(i, j)?, d(i + 1, j)?, d(i, j + 1)?, d(i + 1, j + 1)?);
        Some((a * (1.0 - u) + b * u) * (1.0 - v) + (c * (1.0 - u) + e * u) * v)
    }
}

impl Terrain {
    /// The ground as surveyed and as graded on a grid over where the
    /// platforms change it (none without platforms).
    fn earth_grid(&self) -> Option<Grid> {
        let ground = self.ground.as_ref()?;
        if self.pads.is_empty() {
            return None;
        }
        let surveyed = Terrain {
            points: ground.clone(),
            pads: Vec::new(),
            ground: None,
            ..self.clone()
        };
        let (mut lo, mut hi) = (
            Point2::new(f64::MAX, f64::MAX),
            Point2::new(f64::MIN, f64::MIN),
        );
        for p in self.points.iter().skip(ground.len()) {
            lo = Point2::new(lo.x.min(p.at.x), lo.y.min(p.at.y));
            hi = Point2::new(hi.x.max(p.at.x), hi.y.max(p.at.y));
        }
        for p in self.pads.iter().flat_map(|p| &p.outline) {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        if lo.x > hi.x {
            return None;
        }
        let area = (hi.x - lo.x) * (hi.y - lo.y);
        let step = (area / 250_000.0).sqrt().max(EARTHWORKS_STEP);
        let nx = ((hi.x - lo.x) / step).ceil().max(1.0) as usize + 1;
        let ny = ((hi.y - lo.y) / step).ceil().max(1.0) as usize + 1;
        // `sample` gives the middles of cells: half a step back, the nodes.
        let half = Point2::new(lo.x - step / 2.0, lo.y - step / 2.0);
        Some(Grid {
            lo,
            step,
            nx,
            ny,
            before: surveyed.sample(half, step, nx, ny),
            after: self.sample(half, step, nx, ny),
        })
    }

    /// What each platform digs and fills, in the order of `pads`: the
    /// ground between the survey and the graded surface, each piece of it
    /// given to the nearest platform.
    pub fn pad_earthworks(&self) -> Vec<PadEarthworks> {
        match self.earth_grid() {
            Some(g) => self.pad_earthworks_on(&g),
            None => vec![PadEarthworks::default(); self.pads.len()],
        }
    }

    /// What the plan draws of its platforms, from one grid: their banks,
    /// where the ground is dug and filled and, with `volumes`, each one's
    /// cut and fill.
    pub fn earth_plan(&self, volumes: bool) -> EarthPlan {
        let Some(g) = self.earth_grid() else {
            return EarthPlan {
                banks: vec![Vec::new(); self.pads.len()],
                ..EarthPlan::default()
            };
        };
        EarthPlan {
            banks: self.pads.iter().map(|p| self.banks_on(&g, p)).collect(),
            map: Some(map_of(&g)),
            volumes: if volumes {
                self.pad_earthworks_on(&g)
            } else {
                Vec::new()
            },
        }
    }

    fn pad_earthworks_on(&self, g: &Grid) -> Vec<PadEarthworks> {
        let n = self.pads.len();
        let nodes: Vec<Point2> = (0..g.ny)
            .flat_map(|j| (0..g.nx).map(move |i| (i, j)))
            .map(|(i, j)| g.at(i, j))
            .collect();
        let both: Vec<Option<(f64, f64)>> = g
            .before
            .iter()
            .zip(&g.after)
            .map(|(b, a)| Some(((*b)?, (*a)?)))
            .collect();
        // Each cell in two triangles, given to the nearest platform.
        let mut cut_tris: Vec<Vec<[usize; 3]>> = vec![Vec::new(); n];
        let mut fill_tris: Vec<Vec<[usize; 3]>> = vec![Vec::new(); n];
        for j in 0..g.ny - 1 {
            for i in 0..g.nx - 1 {
                let k = j * g.nx + i;
                let (a, b, c, d) = (k, k + 1, k + g.nx + 1, k + g.nx);
                for tri in [[a, b, c], [a, c, d]] {
                    let Some(v) = tri.map(|x| both[x]).into_iter().collect::<Option<Vec<_>>>()
                    else {
                        continue;
                    };
                    let dug = v.iter().any(|(b, a)| b - a > THIN);
                    let filled = v.iter().any(|(b, a)| a - b > THIN);
                    if !dug && !filled {
                        continue;
                    }
                    let m = (nodes[tri[0]] + nodes[tri[1]] + nodes[tri[2]]) * (1.0 / 3.0);
                    // The platform that made the ground there what it is
                    // (inside it, or held by its bank); of those, the
                    // nearest.
                    let before = (v[0].0 + v[1].0 + v[2].0) / 3.0;
                    let after = (v[0].1 + v[1].1 + v[2].1) / 3.0;
                    let owner = (0..n)
                        .map(|x| {
                            let pad = &self.pads[x];
                            let (d, near) = nearest_out(m, &pad.outline);
                            let zr = pad.z_at(if d > 0.0 { near } else { m });
                            let held = before.clamp(zr - d * pad.slope, zr + d * pad.slope);
                            let miss = ((after - held).abs() * 20.0).round();
                            (x, miss, d)
                        })
                        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)))
                        .map_or(0, |a| a.0);
                    if dug {
                        cut_tris[owner].push(tri);
                    }
                    if filled {
                        fill_tris[owner].push(tri);
                    }
                }
            }
        }
        (0..n)
            .map(|p| {
                let bottom_cut: Vec<f64> = both.iter().map(|v| v.map_or(0.0, |v| v.1)).collect();
                let thick_cut: Vec<f64> = both
                    .iter()
                    .map(|v| v.map_or(0.0, |(b, a)| (b - a).max(0.0)))
                    .collect();
                let bottom_fill: Vec<f64> = both.iter().map(|v| v.map_or(0.0, |v| v.0)).collect();
                let thick_fill: Vec<f64> = both
                    .iter()
                    .map(|v| v.map_or(0.0, |(b, a)| (a - b).max(0.0)))
                    .collect();
                PadEarthworks {
                    cut: closed_mesh(&nodes, &bottom_cut, &thick_cut, &cut_tris[p]),
                    fill: closed_mesh(&nodes, &bottom_fill, &thick_fill, &fill_tris[p]),
                }
            })
            .collect()
    }

    /// The banks of platform `pad` in plan: marks about every metre along
    /// its edge (and fanned round its outer corners), in order, from its
    /// edge to where the bank meets the ground. None where the platform is
    /// at the height of the ground.
    pub fn pad_banks(&self, pad: usize) -> Vec<BankMark> {
        let (Some(g), Some(p)) = (self.earth_grid(), self.pads.get(pad)) else {
            return Vec::new();
        };
        self.banks_on(&g, p)
    }

    /// The banks of all its platforms, in the order of `pads`.
    pub fn all_banks(&self) -> Vec<Vec<BankMark>> {
        match self.earth_grid() {
            Some(g) => self.pads.iter().map(|p| self.banks_on(&g, p)).collect(),
            None => vec![Vec::new(); self.pads.len()],
        }
    }

    /// Where its platforms dig and fill, cell by cell (for the plan).
    pub fn earth_map(&self) -> Option<EarthMap> {
        Some(map_of(&self.earth_grid()?))
    }

    fn banks_on(&self, g: &Grid, pad: &TerrainPad) -> Vec<BankMark> {
        let edge = &pad.outline;
        let n = edge.len();
        let normal = |i: usize| {
            let d = edge[(i + 1) % n] - edge[i];
            let l = (d.x * d.x + d.y * d.y).sqrt().max(1e-12);
            Vec2::new(d.y / l, -d.x / l)
        };
        let reach = 4.0 * (g.nx.max(g.ny) as f64) * g.step;
        let mut out = Vec::new();
        // The bank goes on while the ground is changed and follows it
        // (not into another platform).
        let tolerance = 0.1 + g.step * pad.slope;
        let mut walk = |q: Point2, d: Vec2| {
            let start = g.change_at(q + d * (g.step * 0.25));
            let Some(t0) = start.filter(|t| t.abs() > 0.02) else {
                return;
            };
            let cut = t0 < 0.0;
            let zr = pad.z_at(q);
            let dd = g.step * 0.25;
            let mut s = dd;
            let mut last = q + d * s;
            while s < reach {
                let r = q + d * s;
                let bank = if cut {
                    zr + s * pad.slope
                } else {
                    zr - s * pad.slope
                };
                match g.both_at(r) {
                    Some((b, a))
                        if ((a - b < -0.01 && cut) || (a - b > 0.01 && !cut))
                            && (a - bank).abs() < tolerance =>
                    {
                        last = r
                    }
                    _ => break,
                }
                s += dd;
            }
            out.push(BankMark {
                edge: q,
                ground: last,
                cut,
            });
        };
        for i in 0..n {
            let (a, b) = (edge[i], edge[(i + 1) % n]);
            let (prev, here) = (normal((i + n - 1) % n), normal(i));
            // An outer corner: a fan of marks.
            let turn = prev.x * here.y - prev.y * here.x;
            if turn > 1e-9 {
                let a0 = prev.y.atan2(prev.x);
                let mut a1 = here.y.atan2(here.x);
                while a1 < a0 {
                    a1 += std::f64::consts::TAU;
                }
                let k = ((a1 - a0) / 0.4).ceil().max(1.0) as usize;
                for j in 1..k {
                    let t = a0 + (a1 - a0) * j as f64 / k as f64;
                    walk(a, Vec2::new(t.cos(), t.sin()));
                }
            }
            let steps = (a.distance(b) / CONTOUR_POINT_SPACING).ceil().max(1.0) as usize;
            for k in 0..steps {
                walk(a + (b - a) * ((k as f64 + 0.5) / steps as f64), here);
            }
        }
        out
    }
}

/// A closed solid over the triangles `tris` of `nodes`: its bottom at
/// `bottom`, its top `thick` above (a node with no thickness is one point,
/// top and bottom), and walls round the edges used by one triangle.
fn closed_mesh(nodes: &[Point2], bottom: &[f64], thick: &[f64], tris: &[[usize; 3]]) -> EarthMesh {
    let mut points: Vec<[f64; 3]> = Vec::new();
    let mut low: std::collections::HashMap<usize, usize> = Default::default();
    let mut high: std::collections::HashMap<usize, usize> = Default::default();
    let mut mesh = EarthMesh {
        lo: Point2::new(f64::MAX, f64::MAX),
        hi: Point2::new(f64::MIN, f64::MIN),
        ..EarthMesh::default()
    };
    let mut bot = |i: usize, points: &mut Vec<[f64; 3]>| {
        *low.entry(i).or_insert_with(|| {
            points.push([nodes[i].x, nodes[i].y, bottom[i]]);
            points.len() - 1
        })
    };
    let mut ids: Vec<([usize; 3], [usize; 3])> = Vec::with_capacity(tris.len());
    for t in tris {
        let b = t.map(|i| bot(i, &mut points));
        ids.push((b, [0; 3]));
    }
    for (k, t) in tris.iter().enumerate() {
        let top = t.map(|i| {
            if thick[i] <= THIN {
                low[&i]
            } else {
                *high.entry(i).or_insert_with(|| {
                    points.push([nodes[i].x, nodes[i].y, bottom[i] + thick[i]]);
                    points.len() - 1
                })
            }
        });
        ids[k].1 = top;
    }
    let mut edges: std::collections::HashMap<(usize, usize), (usize, usize, usize)> =
        Default::default();
    for (t, (b, top)) in tris.iter().zip(&ids) {
        let [pa, pb, pc] = t.map(|i| nodes[i]);
        // Counter-clockwise in plan (as the grid makes them).
        let area = ((pb - pa).cross(pc - pa) / 2.0).abs();
        let th: Vec<f64> = t.iter().map(|&i| thick[i]).collect();
        mesh.volume += area * (th[0] + th[1] + th[2]) / 3.0;
        mesh.area += area * th.iter().filter(|x| **x > THIN).count() as f64 / 3.0;
        mesh.depth = th.iter().fold(mesh.depth, |a, x| a.max(*x));
        for p in [pa, pb, pc] {
            mesh.lo = Point2::new(mesh.lo.x.min(p.x), mesh.lo.y.min(p.y));
            mesh.hi = Point2::new(mesh.hi.x.max(p.x), mesh.hi.y.max(p.y));
        }
        mesh.triangles.push(*top);
        mesh.triangles.push([b[0], b[2], b[1]]);
        for k in 0..3 {
            let (x, y) = (t[k], t[(k + 1) % 3]);
            let e = edges.entry((x.min(y), x.max(y))).or_insert((x, y, 0));
            e.2 += 1;
        }
    }
    // Walls on the edges of one triangle: outward is to the right of the
    // edge as its triangle goes round.
    let mut walls: Vec<(usize, usize)> = edges
        .values()
        .filter(|e| e.2 == 1)
        .map(|e| (e.0, e.1))
        .collect();
    walls.sort();
    for (a, b) in walls {
        let (ab, bb) = (low[&a], low[&b]);
        let at = high
            .get(&a)
            .copied()
            .filter(|_| thick[a] > THIN)
            .unwrap_or(ab);
        let bt = high
            .get(&b)
            .copied()
            .filter(|_| thick[b] > THIN)
            .unwrap_or(bb);
        if bt != bb {
            mesh.triangles.push([ab, bb, bt]);
        }
        if at != ab {
            mesh.triangles.push([ab, bt, at]);
        }
    }
    mesh.points = points;
    if mesh.triangles.is_empty() {
        mesh.lo = Point2::new(0.0, 0.0);
        mesh.hi = Point2::new(0.0, 0.0);
    }
    mesh
}

impl Model {
    /// Every earthwork of the project: the cut and fill of each platform
    /// of each terrain and the dig of each pool (down from the ground,
    /// graded, to the bottom of its shell), in that order. Those of no
    /// volume are left out.
    pub fn earthworks(&self) -> Vec<Earthwork> {
        let mut out = Vec::new();
        for t in self.terrains() {
            for (i, e) in t.pad_earthworks().into_iter().enumerate() {
                let pad = &t.pads[i];
                for (side, mesh, key, word) in [
                    (EarthSide::Cut, e.cut, "cut", "Desmonte"),
                    (EarthSide::Fill, e.fill, "fill", "Terraplén"),
                ] {
                    if mesh.is_empty() {
                        continue;
                    }
                    // The dig of a basement is a "vaciado".
                    let word = if side == EarthSide::Cut && pad.kind == PadKind::Basement {
                        "Vaciado"
                    } else {
                        word
                    };
                    out.push(Earthwork {
                        source: EarthSource::Pad {
                            terrain: t.id,
                            pad: i,
                        },
                        owner: t.guid.clone(),
                        key: format!("pad-{i}-{key}"),
                        name: format!("{word} · {}", pad.label(i)),
                        side,
                        what: EarthUse::Pad(pad.kind),
                        terrain: t.id,
                        mesh,
                    });
                }
            }
        }
        for (source, owner, name, dig) in self.excavations_of() {
            let Some(terrain) = self.excavation_terrain(&dig) else {
                continue;
            };
            let mesh = self.excavation_mesh(&dig);
            if mesh.is_empty() {
                continue;
            }
            out.push(Earthwork {
                source,
                owner,
                key: "excavation".into(),
                name: format!("Vaciado · {name}"),
                side: EarthSide::Cut,
                what: EarthUse::Pool,
                terrain: terrain.id,
                mesh,
            });
        }
        out
    }

    /// The terrain a pool is dug in: the one over the middle of its outline.
    pub fn excavation_terrain(&self, dig: &Excavation) -> Option<&Terrain> {
        let n = dig.outline.len().max(1) as f64;
        let c = dig
            .outline
            .iter()
            .fold(Point2::new(0.0, 0.0), |a, p| a + *p * (1.0 / n));
        self.terrains().find(|t| t.height_at(c).is_some())
    }

    /// The ground a pool digs: inside its outline, from the ground (as
    /// graded) down to its bottom; empty where no terrain is over it.
    pub fn excavation_mesh(&self, dig: &Excavation) -> EarthMesh {
        let poly = {
            let mut p = dig.outline.clone();
            if signed_area(&p) < 0.0 {
                p.reverse();
            }
            p
        };
        let Some(terrain) = self.excavation_terrain(dig) else {
            return EarthMesh::default();
        };
        // Its edge every half metre, and a grid inside.
        let mut nodes: Vec<Point2> = Vec::new();
        let n = poly.len();
        let step = EARTHWORKS_STEP;
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let k = (a.distance(b) / step).ceil().max(1.0) as usize;
            for j in 0..k {
                nodes.push(a + (b - a) * (j as f64 / k as f64));
            }
        }
        let (mut lo, mut hi) = (poly[0], poly[0]);
        for p in &poly {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        let mut y = lo.y + step / 2.0;
        while y < hi.y {
            let mut x = lo.x + step / 2.0;
            while x < hi.x {
                let q = Point2::new(x, y);
                if super::roofs::point_in_polygon(q, &poly)
                    && nearest_out_edge(q, &poly) > step * 0.3
                {
                    nodes.push(q);
                }
                x += step;
            }
            y += step;
        }
        // The surface over them, in one pass over its triangles.
        let tris = terrain.triangles();
        let mut top: Vec<Option<f64>> = vec![None; nodes.len()];
        for [a, b, c] in tris {
            let (pa, pb, pc) = (terrain.points[a], terrain.points[b], terrain.points[c]);
            let x0 = pa.at.x.min(pb.at.x).min(pc.at.x);
            let x1 = pa.at.x.max(pb.at.x).max(pc.at.x);
            let y0 = pa.at.y.min(pb.at.y).min(pc.at.y);
            let y1 = pa.at.y.max(pb.at.y).max(pc.at.y);
            if x1 < lo.x || x0 > hi.x || y1 < lo.y || y0 > hi.y {
                continue;
            }
            for (k, q) in nodes.iter().enumerate() {
                if top[k].is_some() || q.x < x0 || q.x > x1 || q.y < y0 || q.y > y1 {
                    continue;
                }
                let d = (pb.at - pa.at).cross(pc.at - pa.at);
                if d.abs() < 1e-12 {
                    continue;
                }
                let u = (pb.at - *q).cross(pc.at - *q) / d;
                let v = (pc.at - *q).cross(pa.at - *q) / d;
                let w = 1.0 - u - v;
                if u >= -1e-9 && v >= -1e-9 && w >= -1e-9 {
                    top[k] = Some(u * pa.z + v * pb.z + w * pc.z);
                }
            }
        }
        let tris: Vec<[usize; 3]> = delaunay(&nodes)
            .into_iter()
            .filter(|t| {
                let m = (nodes[t[0]] + nodes[t[1]] + nodes[t[2]]) * (1.0 / 3.0);
                super::roofs::point_in_polygon(m, &poly) && t.iter().all(|&i| top[i].is_some())
            })
            .collect();
        let bottom = vec![dig.bottom; nodes.len()];
        let thick: Vec<f64> = top
            .iter()
            .map(|t| t.map_or(0.0, |z| (z - dig.bottom).max(0.0)))
            .collect();
        closed_mesh(&nodes, &bottom, &thick, &tris)
    }
}

/// Distance from `p` to the edge of `poly` (inside or out).
fn nearest_out_edge(p: Point2, poly: &[Point2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let ab = b - a;
            let len2 = ab.x * ab.x + ab.y * ab.y;
            let t = if len2 < 1e-18 {
                0.0
            } else {
                (((p - a).x * ab.x + (p - a).y * ab.y) / len2).clamp(0.0, 1.0)
            };
            p.distance(a + ab * t)
        })
        .fold(f64::MAX, f64::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(x: f64, y: f64, z: f64) -> SurveyPoint {
        SurveyPoint {
            at: Point2::new(x, y),
            z,
        }
    }

    /// A hillside 40 × 40 m rising 1 m every 10 m to the north.
    fn hillside() -> (Model, TerrainId) {
        let mut m = Model::new("t");
        let l = m.add_level("PB", 0.0);
        let earth = m.add_material("Tierra", "earth", [150, 120, 90], CutPattern::Dots);
        let mut points = Vec::new();
        for i in 0..=8 {
            for j in 0..=8 {
                let (x, y) = (i as f64 * 5.0, j as f64 * 5.0);
                points.push(sp(x, y, y / 10.0));
            }
        }
        let id = m
            .add_terrain(NewTerrain {
                level: l,
                points,
                material: earth,
                depth: 2.0,
                contour: 0.5,
                lines: Vec::new(),
            })
            .unwrap();
        (m, id)
    }

    fn square(x0: f64, y0: f64, s: f64) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x0 + s, y0),
            Point2::new(x0 + s, y0 + s),
            Point2::new(x0, y0 + s),
        ]
    }

    #[test]
    fn a_platform_on_a_slope_digs_and_fills_closed_solids() {
        let (mut m, id) = hillside();
        // At 2,00 in the middle of the slope (ground 1,50 to 2,50 under it):
        // dug uphill, filled downhill.
        m.edit_terrain(id, |t| {
            t.set_pads(vec![TerrainPad::flat(square(15.0, 15.0, 10.0), 2.0, 1.0)])
        })
        .unwrap();
        let t = m.terrain(id).unwrap();
        let e = t.pad_earthworks();
        assert_eq!(e.len(), 1);
        let (cut, fill) = (&e[0].cut, &e[0].fill);
        // Inside: half a wedge of 10 × 5 × 0,5 / 2 each side, and the banks.
        assert!(cut.volume > 12.0 && cut.volume < 20.0, "{}", cut.volume);
        assert!(fill.volume > 12.0 && fill.volume < 20.0, "{}", fill.volume);
        for mesh in [cut, fill] {
            assert_eq!(mesh.open_edges(), 0);
            let v = mesh.solid_volume();
            assert!(
                (v - mesh.volume).abs() < 1e-4 * mesh.volume.max(1.0),
                "{v} {}",
                mesh.volume
            );
            assert!(mesh.area > 50.0);
            assert!(mesh.depth > 0.4 && mesh.depth < 0.7, "{}", mesh.depth);
        }
        // The terrain's earthworks are the same as its platforms'.
        let (c, f) = t.earthworks();
        assert!((c - cut.volume).abs() < 0.05 * c, "{c} {}", cut.volume);
        assert!((f - fill.volume).abs() < 0.05 * f, "{f} {}", fill.volume);
        // Banks: uphill a cut (its top out on the ground), downhill a fill.
        let banks = t.pad_banks(0);
        assert!(banks.iter().any(|b| b.cut && b.edge.y > 24.9));
        assert!(banks.iter().any(|b| !b.cut && b.edge.y < 15.1));
        let north = banks
            .iter()
            .find(|b| b.cut && (b.edge.y - 25.0).abs() < 1e-6 && b.edge.x > 19.0 && b.edge.x < 21.0)
            .unwrap();
        // Ground at 2,50 there, rising 1:10; bank 1:1 from 2,00: they meet
        // about 0,55 m out.
        let out = north.ground.y - north.edge.y;
        assert!(out > 0.3 && out < 0.9, "{out}");
        let ids = m.earthworks();
        assert_eq!(ids.len(), 2);
        assert!(ids[0].name.starts_with("Desmonte"));
        assert_eq!(ids[0].key, "pad-0-cut");
    }

    #[test]
    fn a_ramp_follows_its_grade() {
        let (mut m, id) = hillside();
        // A ramp 4 m wide up the slope at 1:10, from 0,50 to 2,50 over 20 m,
        // set 0,30 m below the ground: a cut all along.
        let ramp = TerrainPad::ramp(
            Point2::new(20.0, 5.0),
            Point2::new(20.0, 25.0),
            4.0,
            0.2,
            2.2,
            1.0,
        )
        .unwrap();
        assert_eq!(ramp.kind, PadKind::Ramp);
        assert!((ramp.z_at(Point2::new(20.0, 15.0)) - 1.2).abs() < 1e-9);
        let (a, b, len) = ramp.ramp_ends().unwrap();
        assert!((a - 0.2).abs() < 1e-9 && (b - 2.2).abs() < 1e-9 && (len - 20.0).abs() < 1e-9);
        m.edit_terrain(id, |t| t.set_pads(vec![ramp])).unwrap();
        let t = m.terrain(id).unwrap();
        // The graded ground on the ramp is its plane.
        let z = t.height_at(Point2::new(20.0, 15.0)).unwrap();
        assert!((z - 1.2).abs() < 0.02, "{z}");
        let e = &t.pad_earthworks()[0];
        // About 4 × 20 × 0,30 inside, and the banks.
        assert!(
            e.cut.volume > 24.0 && e.cut.volume < 34.0,
            "{}",
            e.cut.volume
        );
        assert!(e.fill.volume < 0.5, "{}", e.fill.volume);
        assert_eq!(e.cut.open_edges(), 0);
    }

    #[test]
    fn two_platforms_share_out_the_earthworks() {
        let (mut m, id) = hillside();
        m.edit_terrain(id, |t| {
            t.set_pads(vec![
                TerrainPad::flat(square(5.0, 5.0, 8.0), 0.5, 1.0),
                TerrainPad::flat(square(25.0, 25.0, 8.0), 3.5, 1.0),
            ])
        })
        .unwrap();
        let t = m.terrain(id).unwrap();
        let e = t.pad_earthworks();
        let (c, f) = t.earthworks();
        let sum_c: f64 = e.iter().map(|x| x.cut.volume).sum();
        let sum_f: f64 = e.iter().map(|x| x.fill.volume).sum();
        assert!((c - sum_c).abs() < 0.05 * c.max(1.0), "{c} {sum_c}");
        assert!((f - sum_f).abs() < 0.05 * f.max(1.0), "{f} {sum_f}");
        // Each one's is round itself.
        assert!(e[0].cut.hi.x < 20.0 && e[1].fill.lo.x > 18.0);
        assert!(e[0].fill.is_empty() && e[1].cut.is_empty());
    }

    #[test]
    fn a_pool_digs_down_to_its_shell() {
        let (mut m, _) = hillside();
        let level = m.levels()[0].id;
        let mat = m.add_material("Gresite", "ceramic", [80, 160, 200], CutPattern::Solid);
        // 8 × 4 m on flat ground at y 10…14 (ground 1,0…1,4), the water
        // at the level: its top 0 ⇒ dug from the ground to −2,25 −0,12.
        let o = vec![
            Point2::new(10.0, 10.0),
            Point2::new(18.0, 10.0),
            Point2::new(18.0, 14.0),
            Point2::new(10.0, 14.0),
        ];
        let id = m.add_pool(NewPool::new(level, o, mat)).unwrap();
        let works = m.earthworks();
        assert_eq!(works.len(), 1);
        let w = &works[0];
        assert_eq!(w.source, EarthSource::Pool(id));
        assert_eq!(w.side, EarthSide::Cut);
        assert_eq!(w.what, EarthUse::Pool);
        assert_eq!(w.mesh.open_edges(), 0);
        // Outside of the shell 8,5 × 4,5; mean ground 1,2; bottom −2,37.
        let expect = 8.5 * 4.5 * (1.2 + 2.37);
        assert!(
            (w.mesh.volume - expect).abs() < 0.03 * expect,
            "{} {expect}",
            w.mesh.volume
        );
        let v = w.mesh.solid_volume();
        assert!((v - w.mesh.volume).abs() < 1e-4 * v, "{v}");
    }
}
