//! 3D models made elsewhere (SketchUp, Blender, a manufacturer's
//! catalogue) as coloured triangle meshes: FBX (binary and text), OBJ with
//! its MTL, glTF 2.0 (`.gltf` and `.glb`), COLLADA (`.dae`, what SketchUp
//! exports), 3D Studio (`.3ds`) and STL, also from inside a `.zip` as
//! manufacturers hand them out ([`load`], [`Zip`]). A file that does not
//! say its units (OBJ, 3DS, STL) is read in the units that give it a
//! plausible size ([`plausible_unit`]).
//!
//! Every reader gives the same [`Mesh`]: metres, `z` up, one colour per
//! vertex (the material's, or its texture sampled at the vertex) and the
//! triangles grouped by material. On top: lighter versions of a mesh for
//! far views ([`simplify`], [`levels_of_detail`]) and its outline seen from
//! above ([`outline`]), for the plan symbol.
//!
//! It knows nothing of the model: the application turns a mesh into an
//! object type.

mod base64;
mod dae;
mod decimate;
mod fbx;
mod gltf;
mod obj;
mod outline;
mod simplify;
mod stl;
mod tds;
mod texture;
mod write;
mod zip;

#[cfg(test)]
mod tests;

pub use decimate::decimate;
pub use outline::outline;
pub use simplify::{levels_of_detail, simplify};
pub use write::write_glb;
pub use zip::{InZip, Zip};

use std::path::Path;

/// An RGB colour, 0–255.
pub type Rgb = [u8; 3];

/// A material of the model: what the faces of a [`Group`] are made of.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    /// Its colour; with a texture, the texture's average tinted by it.
    pub color: Rgb,
    /// 1 opaque, less: glass, water, leaves cut out…
    pub alpha: f32,
    /// The file of its texture, if it had one (for the report).
    pub texture: Option<String>,
}

/// The triangles of one material.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Group {
    /// Index into [`Mesh::materials`].
    pub material: usize,
    /// Indices into [`Mesh::positions`].
    pub triangles: Vec<[u32; 3]>,
}

/// A coloured triangle mesh in metres, `z` up.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    /// The colour of each vertex (same length as `positions`).
    pub colors: Vec<Rgb>,
    pub materials: Vec<Material>,
    pub groups: Vec<Group>,
}

/// What went wrong reading a model.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Not one of the formats read (by its extension).
    Format(String),
    /// A file could not be read.
    Io(String),
    /// The file is not what it says.
    Invalid(String),
    /// It has no triangles.
    Empty,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Format(e) => write!(
                f,
                "formato «{e}» no admitido (se leen FBX, OBJ, glTF, GLB, DAE de COLLADA, 3DS y STL, también dentro de un .zip)"
            ),
            Error::Io(e) => write!(f, "no se puede leer {e}"),
            Error::Invalid(e) => write!(f, "fichero no válido: {e}"),
            Error::Empty => write!(f, "el modelo no tiene caras"),
        }
    }
}

impl std::error::Error for Error {}

/// Which way is up in the file, when it does not say (OBJ never does).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Up {
    /// As the format says: glTF is `y` up, COLLADA says it, OBJ is taken
    /// as `y` up (Blender, most catalogues).
    Auto,
    Y,
    Z,
}

/// How a model is read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    pub up: Up,
    /// Metres per unit of the file; `None`: as the file says (COLLADA's
    /// `unit`, glTF is metres) or metres.
    pub unit: Option<f64>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            up: Up::Auto,
            unit: None,
        }
    }
}

/// Where the files a model refers to (its MTL, `.bin`, textures) are
/// read from: by their name as written in the model.
pub trait Resources {
    fn get(&self, name: &str) -> Option<Vec<u8>>;
}

/// Files next to the model, in its folder.
pub struct Folder<'a>(pub &'a Path);

impl Resources for Folder<'_> {
    fn get(&self, name: &str) -> Option<Vec<u8>> {
        let name = percent_decode(name);
        let name = name.trim_start_matches("file://");
        let p = Path::new(name);
        let path = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.0.join(p)
        };
        std::fs::read(&path).ok().or_else(|| {
            // Another folder of the author's computer: try by its name
            // next to the model.
            let file = Path::new(&name.replace('\\', "/"))
                .file_name()
                .map(|f| f.to_os_string())?;
            std::fs::read(self.0.join(file)).ok()
        })
    }
}

/// No other files (a model with everything inside, or tests).
pub struct NoResources;

impl Resources for NoResources {
    fn get(&self, _: &str) -> Option<Vec<u8>> {
        None
    }
}

/// `%20` and the like, as written in URIs.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The formats read, by extension.
pub const EXTENSIONS: [&str; 7] = ["fbx", "obj", "gltf", "glb", "dae", "3ds", "stl"];

/// Whether a file name is of a model this crate reads (or a `.zip`, which
/// may hold one).
pub fn is_model_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| EXTENSIONS.contains(&e.as_str()) || e == "zip")
}

/// Whether a format says its units (the others are guessed).
fn says_units(ext: &str) -> bool {
    matches!(ext, "fbx" | "gltf" | "glb" | "dae")
}

/// Metres per unit of a model of `size` (in the file's units) that does
/// not say its units: metres if it measures up to 12, centimetres up to
/// 1200 (12 m), millimetres beyond. Inches are never guessed.
pub fn plausible_unit(size: [f32; 3]) -> f64 {
    let d = size.iter().fold(0.0f32, |a, b| a.max(*b)) as f64;
    if d <= 12.0 {
        1.0
    } else if d <= 1200.0 {
        0.01
    } else {
        0.001
    }
}

/// A model as read from a file, with what was decided reading it.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub mesh: Mesh,
    /// The units chosen because the file does not say them (metres per
    /// unit), when they were not given.
    pub guessed_unit: Option<f64>,
    /// The model read inside the archive, for a `.zip`.
    pub entry: Option<String>,
    /// Every model in the archive that could be read, best first.
    pub models: Vec<String>,
    /// Other formats of models the archive brings that are not read.
    pub others: Vec<&'static str>,
}

/// Reads the model at `path`, with the files it refers to from its
/// folder; for a `.zip`, the model `entry` inside (or the best one).
pub fn load(path: &Path, entry: Option<&str>, options: Options) -> Result<Loaded, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "zip" {
        let zip = Zip::open(bytes)?;
        let models = zip.models();
        let others = zip.other_formats();
        let Some(name) = entry.map(String::from).or_else(|| models.first().cloned()) else {
            return Err(Error::Invalid(if others.is_empty() {
                "el .zip no tiene ningún modelo 3D".into()
            } else {
                format!(
                    "el .zip solo trae {}, que ArchiSim no lee: descargue el modelo en FBX, OBJ o 3DS",
                    others.join(", ")
                )
            }));
        };
        let data = zip
            .read(&name)
            .ok_or_else(|| Error::Io(format!("{name} dentro del .zip")))?;
        let folder = name.rsplit_once('/').map_or("", |(a, _)| a).to_string();
        let inner_ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let (mesh, guessed_unit) =
            read_guessing(&data, &inner_ext, &InZip { zip: &zip, folder }, options)?;
        return Ok(Loaded {
            mesh,
            guessed_unit,
            entry: Some(name),
            models,
            others,
        });
    }
    let folder = path.parent().unwrap_or(Path::new("."));
    let (mesh, guessed_unit) = read_guessing(&bytes, &ext, &Folder(folder), options)?;
    Ok(Loaded {
        mesh,
        guessed_unit,
        entry: None,
        models: Vec::new(),
        others: Vec::new(),
    })
}

/// Reads the model at `path`, with the files it refers to from its folder
/// (the best model inside, for a `.zip`).
pub fn read_file(path: &Path, options: Options) -> Result<Mesh, Error> {
    load(path, None, options).map(|l| l.mesh)
}

/// Reads a model of format `ext` (`fbx`, `obj`, `gltf`, `glb`, `dae`,
/// `3ds`, `stl`).
pub fn read(
    bytes: &[u8],
    ext: &str,
    files: &dyn Resources,
    options: Options,
) -> Result<Mesh, Error> {
    read_guessing(bytes, ext, files, options).map(|r| r.0)
}

fn read_guessing(
    bytes: &[u8],
    ext: &str,
    files: &dyn Resources,
    options: Options,
) -> Result<(Mesh, Option<f64>), Error> {
    let mut mesh = match ext {
        "fbx" => fbx::read(bytes, files, options)?,
        "obj" => obj::read(bytes, files, options)?,
        "gltf" | "glb" => gltf::read(bytes, files, options)?,
        "dae" => dae::read(bytes, files, options)?,
        "3ds" => tds::read(bytes, files, options)?,
        "stl" => stl::read(bytes, options)?,
        other => return Err(Error::Format(other.to_string())),
    };
    mesh.clean();
    if mesh.triangle_count() == 0 {
        return Err(Error::Empty);
    }
    let mut guessed = None;
    if options.unit.is_none() && !says_units(ext) {
        let (lo, hi) = mesh.bounds();
        let k = plausible_unit([0, 1, 2].map(|i| hi[i] - lo[i]));
        if k != 1.0 {
            mesh.scale(k as f32);
        }
        guessed = Some(k);
    }
    Ok((mesh, guessed))
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        self.groups.iter().map(|g| g.triangles.len()).sum()
    }

    /// The smallest box around it: lowest and highest corners.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for g in &self.groups {
            for t in &g.triangles {
                for &i in t {
                    let p = self.positions[i as usize];
                    for k in 0..3 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
            }
        }
        if lo[0] > hi[0] {
            return ([0.0; 3], [0.0; 3]);
        }
        (lo, hi)
    }

    /// Moved so that its base is at `z = 0` and the middle of its base at
    /// the origin, which is where it is inserted.
    pub fn centre_on_base(&mut self) {
        let (lo, hi) = self.bounds();
        let d = [-(lo[0] + hi[0]) / 2.0, -(lo[1] + hi[1]) / 2.0, -lo[2]];
        for p in &mut self.positions {
            for k in 0..3 {
                p[k] += d[k];
            }
        }
    }

    /// Scaled by `k` (all three directions).
    pub fn scale(&mut self, k: f32) {
        for p in &mut self.positions {
            for c in p.iter_mut() {
                *c *= k;
            }
        }
    }

    /// Without degenerate triangles, empty groups and groups of the same
    /// material split apart.
    fn clean(&mut self) {
        let pos = &self.positions;
        let n = pos.len() as u32;
        for g in &mut self.groups {
            g.triangles.retain(|t| {
                t.iter().all(|&i| i < n) && t[0] != t[1] && t[1] != t[2] && t[0] != t[2] && {
                    let [a, b, c] = t.map(|i| pos[i as usize]);
                    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                    let x = [
                        u[1] * v[2] - u[2] * v[1],
                        u[2] * v[0] - u[0] * v[2],
                        u[0] * v[1] - u[1] * v[0],
                    ];
                    (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]) > 1e-20
                }
            });
        }
        let mut merged: Vec<Group> = Vec::new();
        for g in self.groups.drain(..) {
            if g.triangles.is_empty() {
                continue;
            }
            match merged.iter_mut().find(|m| m.material == g.material) {
                Some(m) => m.triangles.extend(g.triangles),
                None => merged.push(g),
            }
        }
        self.groups = merged;
        if self.colors.len() != self.positions.len() {
            self.colors.resize(self.positions.len(), [200, 200, 200]);
        }
    }
}

/// Builds a mesh vertex by vertex (readers use it).
#[derive(Default)]
pub(crate) struct Builder {
    pub mesh: Mesh,
}

impl Builder {
    /// The material named `name` (added with `make` the first time).
    pub fn material(&mut self, name: &str, make: impl FnOnce() -> Material) -> usize {
        if let Some(i) = self.mesh.materials.iter().position(|m| m.name == name) {
            return i;
        }
        let mut m = make();
        m.name = name.to_string();
        self.mesh.materials.push(m);
        self.mesh.materials.len() - 1
    }

    pub fn vertex(&mut self, p: [f32; 3], color: Rgb) -> u32 {
        self.mesh.positions.push(p);
        self.mesh.colors.push(color);
        (self.mesh.positions.len() - 1) as u32
    }

    pub fn triangle(&mut self, material: usize, t: [u32; 3]) {
        match self.mesh.groups.last_mut() {
            Some(g) if g.material == material => g.triangles.push(t),
            _ => self.mesh.groups.push(Group {
                material,
                triangles: vec![t],
            }),
        }
    }
}

/// A 4×4 transform, column-major as glTF and the shaders (`m[col][row]`).
pub(crate) type Mat4 = [[f64; 4]; 4];

pub(crate) const IDENTITY: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

pub(crate) fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (c, col) in m.iter_mut().enumerate() {
        for (r, v) in col.iter_mut().enumerate() {
            *v = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    m
}

pub(crate) fn apply(m: &Mat4, p: [f64; 3]) -> [f64; 3] {
    let mut out = [0.0; 3];
    for (r, o) in out.iter_mut().enumerate() {
        *o = m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r];
    }
    out
}

/// Whether a transform mirrors (its faces then turn inside out).
pub(crate) fn mirrors(m: &Mat4) -> bool {
    let d = m[0][0] * (m[1][1] * m[2][2] - m[2][1] * m[1][2])
        - m[1][0] * (m[0][1] * m[2][2] - m[2][1] * m[0][2])
        + m[2][0] * (m[0][1] * m[1][2] - m[1][1] * m[0][2]);
    d < 0.0
}

/// From a file's axes to ours (`z` up) in metres.
pub(crate) fn to_ours(up: Up, unit: f64, p: [f64; 3]) -> [f32; 3] {
    let [x, y, z] = p.map(|c| c * unit);
    match up {
        // y up: x stays, the file's z comes towards the viewer (our -y).
        Up::Y => [x as f32, -z as f32, y as f32],
        Up::Z | Up::Auto => [x as f32, y as f32, z as f32],
    }
}
