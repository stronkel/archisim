//! FBX, binary (6.1 to 7.7, arrays compressed or not) and in text (6.1
//! and 7.x): the format most furniture manufacturers hand out. Meshes of
//! any number of sides (fanned into triangles) placed by their hierarchy
//! of models (translation, rotation in any order, scaling, pivots, pre-
//! and post-rotations and the geometric offset of 3ds Max), with their
//! materials (diffuse colour, opacity) and textures (inside the file or
//! next to it), and the file's axes and units (`GlobalSettings`).

use std::collections::HashMap;
use std::rc::Rc;

use crate::texture::{from_unit, tint, Texture};
use crate::{
    apply, mirrors, mul, Builder, Error, Mat4, Material, Mesh, Options, Resources, Rgb, Up,
    IDENTITY,
};

/// A value of a node.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Prop {
    Int(i64),
    Num(f64),
    Str(String),
    Raw(Vec<u8>),
    Ints(Vec<i64>),
    Nums(Vec<f64>),
}

impl Prop {
    fn num(&self) -> Option<f64> {
        match self {
            Prop::Int(i) => Some(*i as f64),
            Prop::Num(x) => Some(*x),
            _ => None,
        }
    }

    fn str(&self) -> Option<&str> {
        match self {
            Prop::Str(s) => Some(s),
            _ => None,
        }
    }

    fn nums(&self) -> Vec<f64> {
        match self {
            Prop::Nums(v) => v.clone(),
            Prop::Ints(v) => v.iter().map(|i| *i as f64).collect(),
            other => other.num().into_iter().collect(),
        }
    }

    fn ints(&self) -> Vec<i64> {
        match self {
            Prop::Ints(v) => v.clone(),
            Prop::Nums(v) => v.iter().map(|x| *x as i64).collect(),
            other => other.num().map(|x| x as i64).into_iter().collect(),
        }
    }

    /// An object's id: its number (7.x) or its full name (6.1).
    fn key(&self) -> Option<String> {
        match self {
            Prop::Int(i) => Some(i.to_string()),
            Prop::Num(x) => Some((*x as i64).to_string()),
            Prop::Str(s) => Some(s.clone()),
            _ => None,
        }
    }
}

/// A node of the file: its name, values and nodes inside.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Node {
    pub name: String,
    pub props: Vec<Prop>,
    pub children: Vec<Node>,
}

impl Node {
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// The first value of a child node (`MappingInformationType: "ByPolygon"`).
    fn value(&self, name: &str) -> Option<&Prop> {
        self.child(name).and_then(|c| c.props.first())
    }

    /// The array of a child node: values in a row (6.1 text) or one array.
    fn array(&self, name: &str) -> Option<Vec<f64>> {
        let c = self.child(name)?;
        Some(c.props.iter().flat_map(|p| p.nums()).collect())
    }

    fn int_array(&self, name: &str) -> Option<Vec<i64>> {
        let c = self.child(name)?;
        Some(c.props.iter().flat_map(|p| p.ints()).collect())
    }

    /// The properties of an object: `Properties70` (`P: name, type, label,
    /// flags, values…`) or `Properties60` (`Property: name, type, flags,
    /// values…`).
    fn properties(&self) -> HashMap<&str, &[Prop]> {
        let mut out = HashMap::new();
        if let Some(p) = self.child("Properties70") {
            for p in p.all("P") {
                if let (Some(name), Some(v)) =
                    (p.props.first().and_then(|n| n.str()), p.props.get(4..))
                {
                    out.insert(name, v);
                }
            }
        }
        if let Some(p) = self.child("Properties60") {
            for p in p.all("Property") {
                if let (Some(name), Some(v)) =
                    (p.props.first().and_then(|n| n.str()), p.props.get(3..))
                {
                    out.insert(name, v);
                }
            }
        }
        out
    }
}

fn invalid(s: impl Into<String>) -> Error {
    Error::Invalid(format!("FBX: {}", s.into()))
}

// ---------------------------------------------------------------- binary

const MAGIC: &[u8] = b"Kaydara FBX Binary  \0";

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
    wide: bool,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|e| *e <= self.b.len());
        let Some(end) = end else {
            return Err(invalid("se corta antes de tiempo"));
        };
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn offset(&mut self) -> Result<u64, Error> {
        if self.wide {
            Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
        } else {
            Ok(self.u32()? as u64)
        }
    }

    /// A node, or `None` at the empty record that closes a list.
    fn node(&mut self) -> Result<Option<Node>, Error> {
        let end = self.offset()? as usize;
        let count = self.offset()?;
        let _len = self.offset()?;
        let name_len = self.u8()? as usize;
        if end == 0 {
            return Ok(None);
        }
        if end > self.b.len() || end < self.at {
            return Err(invalid("un nodo apunta fuera del fichero"));
        }
        let name = String::from_utf8_lossy(self.take(name_len)?).into_owned();
        let mut props = Vec::new();
        for _ in 0..count {
            props.push(self.prop()?);
        }
        let mut children = Vec::new();
        while self.at < end {
            match self.node()? {
                Some(n) => children.push(n),
                None => break,
            }
        }
        self.at = end;
        Ok(Some(Node {
            name,
            props,
            children,
        }))
    }

    fn prop(&mut self) -> Result<Prop, Error> {
        let t = self.u8()?;
        Ok(match t {
            b'Y' => Prop::Int(i16::from_le_bytes(self.take(2)?.try_into().unwrap()) as i64),
            b'C' | b'B' => Prop::Int(self.u8()? as i64),
            b'I' => Prop::Int(i32::from_le_bytes(self.take(4)?.try_into().unwrap()) as i64),
            b'L' => Prop::Int(i64::from_le_bytes(self.take(8)?.try_into().unwrap())),
            b'F' => Prop::Num(f32::from_le_bytes(self.take(4)?.try_into().unwrap()) as f64),
            b'D' => Prop::Num(f64::from_le_bytes(self.take(8)?.try_into().unwrap())),
            b'S' => {
                let n = self.u32()? as usize;
                Prop::Str(String::from_utf8_lossy(self.take(n)?).into_owned())
            }
            b'R' => {
                let n = self.u32()? as usize;
                Prop::Raw(self.take(n)?.to_vec())
            }
            b'f' | b'd' | b'l' | b'i' | b'b' => {
                let count = self.u32()? as usize;
                let encoding = self.u32()?;
                let len = self.u32()? as usize;
                let raw = self.take(len)?;
                let data = if encoding == 1 {
                    miniz_oxide::inflate::decompress_to_vec_zlib(raw)
                        .map_err(|_| invalid("una lista comprimida no se puede leer"))?
                } else {
                    raw.to_vec()
                };
                let size = match t {
                    b'f' | b'i' => 4,
                    b'd' | b'l' => 8,
                    _ => 1,
                };
                if data.len() < count * size {
                    return Err(invalid("una lista es más corta de lo que dice"));
                }
                let items = data.chunks_exact(size).take(count);
                match t {
                    b'f' => Prop::Nums(
                        items
                            .map(|c| f32::from_le_bytes(c.try_into().unwrap()) as f64)
                            .collect(),
                    ),
                    b'd' => Prop::Nums(
                        items
                            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
                            .collect(),
                    ),
                    b'l' => Prop::Ints(
                        items
                            .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
                            .collect(),
                    ),
                    b'i' => Prop::Ints(
                        items
                            .map(|c| i32::from_le_bytes(c.try_into().unwrap()) as i64)
                            .collect(),
                    ),
                    _ => Prop::Ints(items.map(|c| c[0] as i64).collect()),
                }
            }
            other => return Err(invalid(format!("tipo de valor «{}»", other as char))),
        })
    }
}

fn parse_binary(b: &[u8]) -> Result<Vec<Node>, Error> {
    if b.len() < 27 {
        return Err(invalid("demasiado corto"));
    }
    let version = u32::from_le_bytes(b[23..27].try_into().unwrap());
    let mut r = Reader {
        b,
        at: 27,
        wide: version >= 7500,
    };
    let mut out = Vec::new();
    while r.at + if r.wide { 25 } else { 13 } <= b.len() {
        match r.node()? {
            Some(n) => out.push(n),
            None => break,
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ text

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    /// `Name:` opens a node.
    Key(String),
    Word(String),
    Str(String),
    Comma,
    Open,
    Close,
    Line,
}

fn tokens(c: &[u8]) -> Vec<Tok> {
    let text = |a: usize, b: usize| String::from_utf8_lossy(&c[a..b]).into_owned();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            b';' => {
                while i < c.len() && c[i] != b'\n' {
                    i += 1;
                }
            }
            b'\n' => {
                out.push(Tok::Line);
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b'{' => {
                out.push(Tok::Open);
                i += 1;
            }
            b'}' => {
                out.push(Tok::Close);
                i += 1;
            }
            b'"' => {
                let start = i + 1;
                i = start;
                while i < c.len() && c[i] != b'"' {
                    i += 1;
                }
                out.push(Tok::Str(text(start, i.min(c.len()))));
                i += 1;
            }
            ch if ch.is_ascii_whitespace() => i += 1,
            _ => {
                let start = i;
                while i < c.len()
                    && !matches!(c[i], b',' | b'{' | b'}' | b'"' | b';' | b':')
                    && !c[i].is_ascii_whitespace()
                {
                    i += 1;
                }
                if i < c.len() && c[i] == b':' {
                    out.push(Tok::Key(text(start, i)));
                    i += 1;
                } else if i == start {
                    i += 1;
                } else {
                    out.push(Tok::Word(text(start, i)));
                }
            }
        }
    }
    out
}

fn word(s: &str) -> Prop {
    if let Ok(i) = s.parse::<i64>() {
        Prop::Int(i)
    } else if let Ok(x) = s.parse::<f64>() {
        Prop::Num(x)
    } else {
        Prop::Str(s.to_string())
    }
}

/// The nodes from `at` until a `}` (or the end).
fn text_nodes(t: &[Tok], at: &mut usize) -> Vec<Node> {
    let mut out = Vec::new();
    while *at < t.len() {
        match &t[*at] {
            Tok::Close => {
                *at += 1;
                break;
            }
            Tok::Key(name) => {
                *at += 1;
                let mut node = Node {
                    name: name.clone(),
                    ..Default::default()
                };
                // Values: on its line, or on the next ones after a comma.
                let mut after_comma = false;
                while *at < t.len() {
                    match &t[*at] {
                        Tok::Word(w) if w.starts_with('*') => {
                            // `*N { a: 1,2,3 }`: an array of 7.x.
                            *at += 1;
                            if t.get(*at) == Some(&Tok::Open) {
                                *at += 1;
                                let inner = text_nodes(t, at);
                                let vals: Vec<f64> = inner
                                    .iter()
                                    .flat_map(|n| n.props.iter().flat_map(|p| p.nums()))
                                    .collect();
                                let ints = inner
                                    .iter()
                                    .flat_map(|n| &n.props)
                                    .all(|p| matches!(p, Prop::Int(_)));
                                node.props.push(if ints {
                                    Prop::Ints(vals.iter().map(|v| *v as i64).collect())
                                } else {
                                    Prop::Nums(vals)
                                });
                            }
                            after_comma = false;
                        }
                        Tok::Word(w) => {
                            node.props.push(word(w));
                            *at += 1;
                            after_comma = false;
                        }
                        Tok::Str(s) => {
                            node.props.push(Prop::Str(s.clone()));
                            *at += 1;
                            after_comma = false;
                        }
                        Tok::Comma => {
                            *at += 1;
                            after_comma = true;
                        }
                        Tok::Line if after_comma => *at += 1,
                        _ => break,
                    }
                }
                if t.get(*at) == Some(&Tok::Open) {
                    *at += 1;
                    node.children = text_nodes(t, at);
                }
                out.push(node);
            }
            _ => *at += 1,
        }
    }
    out
}

fn parse_text(bytes: &[u8]) -> Vec<Node> {
    let t = tokens(bytes);
    let mut at = 0;
    text_nodes(&t, &mut at)
}

/// The nodes of an FBX, binary or text.
pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<Node>, Error> {
    if bytes.starts_with(MAGIC) {
        parse_binary(bytes)
    } else {
        let nodes = parse_text(bytes);
        if nodes.iter().any(|n| n.name == "Objects") {
            Ok(nodes)
        } else {
            Err(invalid("no es un FBX"))
        }
    }
}

// ---------------------------------------------------------------- scene

/// The name of an object: `Name\0\x01Class` (binary) or `Class::Name`
/// (text).
fn object_name(s: &str) -> String {
    if let Some((n, _)) = s.split_once('\0') {
        n.to_string()
    } else if let Some((_, n)) = s.split_once("::") {
        n.to_string()
    } else {
        s.to_string()
    }
}

fn vec3(p: &HashMap<&str, &[Prop]>, name: &str, default: [f64; 3]) -> [f64; 3] {
    match p.get(name) {
        Some(v) if v.len() >= 3 => [
            v[0].num().unwrap_or(default[0]),
            v[1].num().unwrap_or(default[1]),
            v[2].num().unwrap_or(default[2]),
        ],
        _ => default,
    }
}

fn number(p: &HashMap<&str, &[Prop]>, name: &str) -> Option<f64> {
    p.get(name).and_then(|v| v.first()).and_then(|v| v.num())
}

fn translation(t: [f64; 3]) -> Mat4 {
    let mut m = IDENTITY;
    m[3] = [t[0], t[1], t[2], 1.0];
    m
}

fn scaling(s: [f64; 3]) -> Mat4 {
    let mut m = IDENTITY;
    m[0][0] = s[0];
    m[1][1] = s[1];
    m[2][2] = s[2];
    m
}

fn axis_rotation(axis: usize, degrees: f64) -> Mat4 {
    let (s, c) = degrees.to_radians().sin_cos();
    let (a, b) = match axis {
        0 => (1, 2),
        1 => (2, 0),
        _ => (0, 1),
    };
    let mut m = IDENTITY;
    m[a][a] = c;
    m[a][b] = s;
    m[b][a] = -s;
    m[b][b] = c;
    m
}

/// Euler angles in degrees applied in `order` (FBX's `RotationOrder`:
/// 0 XYZ, 1 XZY, 2 YZX, 3 YXZ, 4 ZXY, 5 ZYX; the first is applied first).
fn rotation(r: [f64; 3], order: i64) -> Mat4 {
    let seq: [usize; 3] = match order {
        1 => [0, 2, 1],
        2 => [1, 2, 0],
        3 => [1, 0, 2],
        4 => [2, 0, 1],
        5 => [2, 1, 0],
        _ => [0, 1, 2],
    };
    let mut m = IDENTITY;
    for a in seq {
        m = mul(&axis_rotation(a, r[a]), &m);
    }
    m
}

fn inverse_rotation(m: &Mat4) -> Mat4 {
    let mut out = IDENTITY;
    for c in 0..3 {
        for r in 0..3 {
            out[c][r] = m[r][c];
        }
    }
    out
}

fn neg(v: [f64; 3]) -> [f64; 3] {
    v.map(|x| -x)
}

/// A model's transform in its parent's frame, and its geometry's offset
/// (3ds Max's object offset, which its children do not inherit).
fn local(model: &Node) -> (Mat4, Mat4) {
    let p = model.properties();
    let zero = [0.0; 3];
    let order = number(&p, "RotationOrder").unwrap_or(0.0) as i64;
    let t = vec3(&p, "Lcl Translation", zero);
    let r = vec3(&p, "Lcl Rotation", zero);
    let s = vec3(&p, "Lcl Scaling", [1.0; 3]);
    let pre = rotation(vec3(&p, "PreRotation", zero), 0);
    let post = rotation(vec3(&p, "PostRotation", zero), 0);
    let roff = vec3(&p, "RotationOffset", zero);
    let rp = vec3(&p, "RotationPivot", zero);
    let soff = vec3(&p, "ScalingOffset", zero);
    let sp = vec3(&p, "ScalingPivot", zero);
    let chain = [
        translation(t),
        translation(roff),
        translation(rp),
        pre,
        rotation(r, order),
        inverse_rotation(&post),
        translation(neg(rp)),
        translation(soff),
        translation(sp),
        scaling(s),
        translation(neg(sp)),
    ];
    let m = chain.iter().fold(IDENTITY, |a, b| mul(&a, b));
    let geo = [
        translation(vec3(&p, "GeometricTranslation", zero)),
        rotation(vec3(&p, "GeometricRotation", zero), 0),
        scaling(vec3(&p, "GeometricScaling", [1.0; 3])),
    ]
    .iter()
    .fold(IDENTITY, |a, b| mul(&a, b));
    (m, geo)
}

/// How a layer's values are taken for each corner of each polygon.
struct Layer {
    values: Vec<f64>,
    width: usize,
    index: Option<Vec<i64>>,
    by_corner: bool,
}

impl Layer {
    fn of(node: &Node, values: &str, index: &str, width: usize) -> Option<Layer> {
        let mapping = node
            .value("MappingInformationType")
            .and_then(|p| p.str())
            .unwrap_or("");
        let reference = node
            .value("ReferenceInformationType")
            .and_then(|p| p.str())
            .unwrap_or("");
        let by_corner = match mapping {
            "ByPolygonVertex" => true,
            "ByVertice" | "ByVertex" | "ByControlPoint" => false,
            _ => return None,
        };
        Some(Layer {
            values: node.array(values)?,
            width,
            index: (reference != "Direct")
                .then(|| node.int_array(index))
                .flatten(),
            by_corner,
        })
    }

    /// The value at polygon corner `corner`, of control point `point`.
    fn get(&self, corner: usize, point: usize) -> Option<&[f64]> {
        let i = if self.by_corner { corner } else { point };
        let j = match &self.index {
            Some(ix) => usize::try_from(*ix.get(i)?).ok()?,
            None => i,
        };
        self.values.get(j * self.width..(j + 1) * self.width)
    }
}

/// A texture as read: its file and its image.
type Loaded = (String, Option<Rc<Texture>>);

/// A material as read, before being met by a mesh.
struct Mat {
    name: String,
    color: [f64; 3],
    alpha: f32,
    texture: Option<Loaded>,
}

pub fn read(bytes: &[u8], files: &dyn Resources, options: Options) -> Result<Mesh, Error> {
    let nodes = parse(bytes)?;
    let top = |name: &str| nodes.iter().find(|n| n.name == name);
    let objects = top("Objects").ok_or_else(|| invalid("sin objetos"))?;

    // Axes and units.
    let settings = top("GlobalSettings")
        .map(|g| g.properties())
        .unwrap_or_default();
    let int = |name: &str, d: i64| number(&settings, name).map_or(d, |v| v as i64);
    // Our x is the file's right (`CoordAxis`), our z its up; our y
    // follows from them (the files are right-handed: what `FrontAxis`
    // says is a parity that writers do not agree on).
    let (mut up, mut up_sign) = (int("UpAxis", 1), int("UpAxisSign", 1));
    let (mut right, mut right_sign) = (int("CoordAxis", 0), int("CoordAxisSign", 1));
    match options.up {
        Up::Y => (up, up_sign, right, right_sign) = (1, 1, 0, 1),
        Up::Z => (up, up_sign, right, right_sign) = (2, 1, 0, 1),
        Up::Auto => {}
    }
    let unit = options
        .unit
        .unwrap_or_else(|| number(&settings, "UnitScaleFactor").unwrap_or(1.0) * 0.01);
    let along = |a: i64, sign: i64| {
        let mut v = [0.0; 3];
        v[a.clamp(0, 2) as usize] = sign.signum() as f64;
        v
    };
    let u = along(up, up_sign);
    let mut r = along(right, right_sign);
    if r == u || r == u.map(|x| -x) {
        r = along((up + 1) % 3, 1);
    }
    let f = [
        u[1] * r[2] - u[2] * r[1],
        u[2] * r[0] - u[0] * r[2],
        u[0] * r[1] - u[1] * r[0],
    ];
    let dot = |a: [f64; 3], p: [f64; 3]| a[0] * p[0] + a[1] * p[1] + a[2] * p[2];
    let to_ours = |p: [f64; 3]| -> [f32; 3] {
        [
            (dot(r, p) * unit) as f32,
            (dot(f, p) * unit) as f32,
            (dot(u, p) * unit) as f32,
        ]
    };

    // Objects by id.
    let mut by_key: HashMap<String, &Node> = HashMap::new();
    for o in &objects.children {
        if let Some(k) = o.props.first().and_then(|p| p.key()) {
            by_key.insert(k, o);
        }
    }
    // Connections: children of each object, in order, with the property
    // they connect to.
    let mut children: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut parent: HashMap<String, String> = HashMap::new();
    if let Some(c) = top("Connections") {
        for c in c
            .children
            .iter()
            .filter(|c| c.name == "C" || c.name == "Connect")
        {
            let (Some(child), Some(par)) = (
                c.props.get(1).and_then(|p| p.key()),
                c.props.get(2).and_then(|p| p.key()),
            ) else {
                continue;
            };
            let prop = c
                .props
                .get(3)
                .and_then(|p| p.str())
                .unwrap_or("")
                .to_string();
            if by_key.get(&child).is_some_and(|n| n.name == "Model")
                && by_key.get(&par).is_some_and(|n| n.name == "Model")
            {
                parent.insert(child.clone(), par.clone());
            }
            children.entry(par).or_default().push((child, prop));
        }
    }
    let kids = |k: &str, class: &str| -> Vec<(&Node, String)> {
        children
            .get(k)
            .map(|v| {
                v.iter()
                    .filter_map(|(c, prop)| {
                        by_key
                            .get(c)
                            .filter(|n| n.name == class)
                            .map(|n| (*n, prop.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut world_of: HashMap<String, Mat4> = HashMap::new();
    fn world(
        k: &str,
        by_key: &HashMap<String, &Node>,
        parent: &HashMap<String, String>,
        memo: &mut HashMap<String, Mat4>,
        depth: usize,
    ) -> Mat4 {
        if let Some(m) = memo.get(k) {
            return *m;
        }
        let own = by_key.get(k).map_or(IDENTITY, |n| local(n).0);
        let m = match parent.get(k) {
            Some(p) if depth < 64 => mul(&world(p, by_key, parent, memo, depth + 1), &own),
            _ => own,
        };
        memo.insert(k.to_string(), m);
        m
    }

    let mut textures: HashMap<String, Option<Rc<Texture>>> = HashMap::new();
    let mut load = |tex: &Node| -> Loaded {
        let k = tex.props.first().and_then(|p| p.key()).unwrap_or_default();
        let file = tex
            .value("RelativeFilename")
            .or_else(|| tex.value("FileName"))
            .and_then(|p| p.str())
            .unwrap_or("")
            .to_string();
        let image = textures.entry(k.clone()).or_insert_with(|| {
            // Inside the file (a Video with its content), or next to it.
            let inside =
                kids(&k, "Video")
                    .into_iter()
                    .find_map(|(v, _)| match v.value("Content") {
                        Some(Prop::Raw(b)) if !b.is_empty() => Texture::decode(b),
                        Some(Prop::Str(s)) if !s.is_empty() => {
                            crate::base64::decode(s).and_then(|b| Texture::decode(&b))
                        }
                        _ => None,
                    });
            inside
                .or_else(|| {
                    [tex.value("RelativeFilename"), tex.value("FileName")]
                        .into_iter()
                        .flatten()
                        .filter_map(|p| p.str())
                        .find_map(|f| files.get(f).and_then(|b| Texture::decode(&b)))
                })
                .map(Rc::new)
        });
        (file, image.clone())
    };
    let material = |m: &Node, load: &mut dyn FnMut(&Node) -> Loaded| -> Mat {
        let k = m.props.first().and_then(|p| p.key()).unwrap_or_default();
        let p = m.properties();
        let color = if p.contains_key("DiffuseColor") {
            vec3(&p, "DiffuseColor", [0.8; 3])
        } else {
            vec3(&p, "Diffuse", [0.8; 3])
        };
        let alpha = match (number(&p, "Opacity"), number(&p, "TransparencyFactor")) {
            (Some(o), _) => o,
            (None, Some(t)) => 1.0 - t,
            _ => 1.0,
        }
        .clamp(0.0, 1.0) as f32;
        let texture = kids(&k, "Texture")
            .into_iter()
            .filter(|(_, prop)| {
                let p = prop.to_lowercase();
                p.is_empty() || p.contains("diffuse") || p.contains("base") || p == "color"
            })
            .map(|(t, _)| load(t))
            .next();
        let name = m
            .props
            .get(1)
            .and_then(|p| p.str())
            .map(object_name)
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Material".into());
        Mat {
            name,
            color,
            alpha,
            texture,
        }
    };

    let mut b = Builder::default();
    let models: Vec<(&String, &&Node)> = by_key.iter().filter(|(_, n)| n.name == "Model").collect();
    let mut models = models;
    models.sort_by(|a, b| a.0.cmp(b.0));
    for (k, model) in models {
        // Its geometry: connected to it (7.x) or inside it (6.1).
        let mut geometries: Vec<&Node> = kids(k, "Geometry").into_iter().map(|(g, _)| g).collect();
        if model.child("Vertices").is_some() {
            geometries.push(model);
        }
        if geometries.is_empty() {
            continue;
        }
        let m = mul(
            &world(k, &by_key, &parent, &mut world_of, 0),
            &local(model).1,
        );
        let flip = mirrors(&m);
        let mats: Vec<Mat> = kids(k, "Material")
            .into_iter()
            .map(|(n, _)| material(n, &mut load))
            .collect();
        for g in geometries {
            mesh_of(&mut b, g, &m, flip, &mats, &to_ours)?;
        }
    }
    Ok(b.mesh)
}

fn mesh_of(
    b: &mut Builder,
    g: &Node,
    m: &Mat4,
    flip: bool,
    mats: &[Mat],
    to_ours: &dyn Fn([f64; 3]) -> [f32; 3],
) -> Result<(), Error> {
    let v = g.array("Vertices").unwrap_or_default();
    let idx = g.int_array("PolygonVertexIndex").unwrap_or_default();
    let points: Vec<[f32; 3]> = v
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| to_ours(apply(m, *p)))
        .collect();
    let uv = g
        .child("LayerElementUV")
        .and_then(|n| Layer::of(n, "UV", "UVIndex", 2));
    let colors = g
        .child("LayerElementColor")
        .and_then(|n| Layer::of(n, "Colors", "ColorIndex", 4));
    let (by_polygon, mat_ix) = match g.child("LayerElementMaterial") {
        Some(n) => (
            n.value("MappingInformationType").and_then(|p| p.str()) == Some("ByPolygon"),
            n.int_array("Materials").unwrap_or_default(),
        ),
        None => (false, Vec::new()),
    };
    let mut ids: Vec<usize> = Vec::new();
    // Vertices already made, by (point, colour, material).
    let mut made: HashMap<(usize, Rgb, usize), u32> = HashMap::new();
    let mut materials: HashMap<usize, usize> = HashMap::new();
    let mut polygon = 0usize;
    let mut poly: Vec<(usize, usize)> = Vec::new();
    for (corner, &raw) in idx.iter().enumerate() {
        let (point, last) = if raw < 0 {
            ((-raw - 1) as usize, true)
        } else {
            (raw as usize, false)
        };
        poly.push((point, corner));
        if !last {
            continue;
        }
        let mi = if by_polygon {
            mat_ix.get(polygon).copied().unwrap_or(0)
        } else {
            mat_ix.first().copied().unwrap_or(0)
        };
        let mi = usize::try_from(mi)
            .unwrap_or(0)
            .min(mats.len().saturating_sub(1));
        let mat = mats.get(mi);
        let material = *materials.entry(mi).or_insert_with(|| {
            let (name, color, alpha, texture) = match mat {
                Some(m) => (m.name.as_str(), m.color, m.alpha, m.texture.as_ref()),
                None => ("Predeterminado", [0.8; 3], 1.0, None),
            };
            let base = match texture.and_then(|t| t.1.as_ref()) {
                Some(t) => t.average(),
                None => from_unit(color),
            };
            b.material(name, || Material {
                name: String::new(),
                color: base,
                alpha,
                texture: texture.map(|t| t.0.clone()).filter(|f| !f.is_empty()),
            })
        });
        let base = b.mesh.materials[material].color;
        let tex = mat
            .and_then(|m| m.texture.as_ref())
            .and_then(|t| t.1.as_ref());
        ids.clear();
        for &(p, c) in &poly {
            let Some(pos) = points.get(p) else {
                return Err(invalid("una cara usa un vértice que no existe"));
            };
            let color: Rgb = match (tex, uv.as_ref().and_then(|l| l.get(c, p))) {
                (Some(t), Some(uv)) => t.sample(uv[0] as f32, uv[1] as f32, true),
                _ => match colors.as_ref().and_then(|l| l.get(c, p)) {
                    Some(col) => tint(from_unit([col[0], col[1], col[2]]), base),
                    None => base,
                },
            };
            let id = *made
                .entry((p, color, material))
                .or_insert_with(|| b.vertex(*pos, color));
            ids.push(id as usize);
        }
        for k in 1..ids.len().saturating_sub(1) {
            let t = [ids[0], ids[k], ids[k + 1]].map(|i| i as u32);
            b.triangle(material, if flip { [t[0], t[2], t[1]] } else { t });
        }
        poly.clear();
        polygon += 1;
    }
    Ok(())
}
