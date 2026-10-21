//! COLLADA (`.dae`, 1.4 and 1.5), what SketchUp exports: the geometries of
//! the visual scene with the transforms of their nodes (also components,
//! `instance_node`), triangles, polygon lists and polygons, the materials
//! bound to each with their diffuse colour or texture and transparency,
//! and the file's unit and up axis.

use std::collections::HashMap;

use roxmltree::{Document, Node};

use crate::texture::{from_unit, tint, Texture};
use crate::{
    apply, mirrors, mul, to_ours, Builder, Error, Mat4, Material, Mesh, Options, Resources, Rgb,
    Up, IDENTITY,
};

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(move |c| c.tag_name().name() == name)
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.tag_name().name() == name)
}

fn numbers(n: Node) -> Vec<f64> {
    n.text()
        .unwrap_or("")
        .split_ascii_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn ints(n: Node) -> Vec<usize> {
    n.text()
        .unwrap_or("")
        .split_ascii_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn url_id(u: &str) -> &str {
    u.trim_start_matches('#')
}

struct Effect {
    color: Rgb,
    alpha: f32,
    image: Option<String>,
}

struct Dae<'a, 'i> {
    ids: HashMap<&'a str, Node<'a, 'i>>,
    files: &'a dyn Resources,
    textures: HashMap<String, Option<Texture>>,
}

impl<'a, 'i> Dae<'a, 'i> {
    fn get(&self, url: &str) -> Option<Node<'a, 'i>> {
        self.ids.get(url_id(url)).copied()
    }

    /// The file of an image element.
    fn image_file(&self, image: Node) -> Option<String> {
        let init = child(image, "init_from")?;
        let text = match child(init, "ref") {
            Some(r) => r.text(),
            None => init.text(),
        }?;
        Some(text.trim().to_string())
    }

    fn effect(&self, effect: Node<'a, 'i>) -> Effect {
        let mut out = Effect {
            color: [200, 200, 200],
            alpha: 1.0,
            image: None,
        };
        let Some(profile) = child(effect, "profile_COMMON") else {
            return out;
        };
        // sid → (surface's image | sampler's surface sid).
        let mut params: HashMap<&str, Node> = HashMap::new();
        for p in children(profile, "newparam") {
            if let Some(sid) = p.attribute("sid") {
                params.insert(sid, p);
            }
        }
        let Some(tech) = child(profile, "technique") else {
            return out;
        };
        let Some(shading) = tech.children().find(|c| {
            matches!(
                c.tag_name().name(),
                "lambert" | "phong" | "blinn" | "constant"
            )
        }) else {
            return out;
        };
        if let Some(d) = child(shading, "diffuse").or_else(|| child(shading, "emission")) {
            if let Some(c) = child(d, "color") {
                let n = numbers(c);
                if n.len() >= 3 {
                    out.color = from_unit([n[0], n[1], n[2]]);
                }
            }
            if let Some(t) = child(d, "texture").and_then(|t| t.attribute("texture")) {
                out.image = self.sampler_image(t, &params);
            }
        }
        // Transparency: SketchUp writes `transparent` as a colour and
        // `transparency` as 1 − opacity is not what the standard says, so
        // only the plain number with the default A_ONE is taken.
        let transparency = child(shading, "transparency")
            .and_then(|t| child(t, "float"))
            .map(numbers)
            .and_then(|n| n.first().copied());
        let transparent = child(shading, "transparent");
        if let (Some(t), Some(tr)) = (transparency, transparent) {
            let opaque = tr.attribute("opaque").unwrap_or("A_ONE");
            let a = match child(tr, "color").map(numbers) {
                Some(c) if c.len() >= 4 && opaque == "A_ONE" => c[3] * t,
                Some(c) if c.len() >= 3 && opaque == "RGB_ZERO" => {
                    1.0 - (c[0] + c[1] + c[2]) / 3.0 * t
                }
                _ => t,
            };
            out.alpha = a.clamp(0.0, 1.0) as f32;
        }
        out
    }

    /// The image file behind a texture reference: a sampler's sid (1.4),
    /// or an image's id.
    fn sampler_image(&self, name: &str, params: &HashMap<&str, Node>) -> Option<String> {
        if let Some(p) = params.get(name) {
            if let Some(s) = child(*p, "sampler2D") {
                // 1.4: source → surface's sid; 1.5: instance_image.
                if let Some(src) = child(s, "source").and_then(|x| x.text()) {
                    return self.sampler_image(src.trim(), params);
                }
                if let Some(img) = child(s, "instance_image").and_then(|x| x.attribute("url")) {
                    return self.get(img).and_then(|i| self.image_file(i));
                }
            }
            if let Some(s) = child(*p, "surface") {
                let id = child(s, "init_from")?.text()?.trim().to_string();
                return self.get(&id).and_then(|i| self.image_file(i));
            }
        }
        self.get(name).and_then(|i| self.image_file(i))
    }

    fn texture(&mut self, file: &str) -> Option<&Texture> {
        if !self.textures.contains_key(file) {
            let t = self.files.get(file).and_then(|b| Texture::decode(&b));
            self.textures.insert(file.to_string(), t);
        }
        self.textures.get(file).and_then(|t| t.as_ref())
    }
}

/// A node's own transform: its `matrix`, `translate`, `rotate` and
/// `scale` children, in order.
fn local(node: Node) -> Mat4 {
    let mut m = IDENTITY;
    for c in node.children() {
        let n = numbers(c);
        let t: Mat4 = match c.tag_name().name() {
            "matrix" if n.len() == 16 => {
                // Row by row in the file.
                let mut t = IDENTITY;
                for r in 0..4 {
                    for k in 0..4 {
                        t[k][r] = n[r * 4 + k];
                    }
                }
                t
            }
            "translate" if n.len() >= 3 => {
                let mut t = IDENTITY;
                t[3] = [n[0], n[1], n[2], 1.0];
                t
            }
            "scale" if n.len() >= 3 => {
                let mut t = IDENTITY;
                t[0][0] = n[0];
                t[1][1] = n[1];
                t[2][2] = n[2];
                t
            }
            "rotate" if n.len() >= 4 => {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if len < 1e-12 {
                    continue;
                }
                let (x, y, z) = (n[0] / len, n[1] / len, n[2] / len);
                let (s, co) = n[3].to_radians().sin_cos();
                let k = 1.0 - co;
                [
                    [co + x * x * k, y * x * k + z * s, z * x * k - y * s, 0.0],
                    [x * y * k - z * s, co + y * y * k, z * y * k + x * s, 0.0],
                    [x * z * k + y * s, y * z * k - x * s, co + z * z * k, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ]
            }
            _ => continue,
        };
        m = mul(&m, &t);
    }
    m
}

/// A source's floats and its stride.
fn source(n: Node) -> (Vec<f64>, usize) {
    let data = child(n, "float_array").map(numbers).unwrap_or_default();
    let stride = child(n, "technique_common")
        .and_then(|t| child(t, "accessor"))
        .and_then(|a| a.attribute("stride"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    (data, stride.max(1))
}

pub fn read(bytes: &[u8], files: &dyn Resources, options: Options) -> Result<Mesh, Error> {
    let text = String::from_utf8_lossy(bytes);
    let doc = Document::parse(&text).map_err(|e| Error::Invalid(format!("XML: {e}")))?;
    let root = doc.root_element();
    let asset = child(root, "asset");
    let file_unit = asset
        .and_then(|a| child(a, "unit"))
        .and_then(|u| u.attribute("meter"))
        .and_then(|m| m.parse::<f64>().ok())
        .unwrap_or(1.0);
    let file_up = asset
        .and_then(|a| child(a, "up_axis"))
        .and_then(|u| u.text())
        .map(|t| t.trim().to_string());
    let up = match options.up {
        Up::Auto => match file_up.as_deref() {
            Some("Z_UP") => Up::Z,
            // X_UP is rare; it is read as Y_UP.
            _ => Up::Y,
        },
        u => u,
    };
    let unit = options.unit.unwrap_or(file_unit);
    let mut ids = HashMap::new();
    for n in root.descendants() {
        if let Some(id) = n.attribute("id") {
            ids.insert(id, n);
        }
    }
    let mut dae = Dae {
        ids,
        files,
        textures: HashMap::new(),
    };
    let scene = child(root, "scene")
        .and_then(|s| child(s, "instance_visual_scene"))
        .and_then(|i| i.attribute("url"))
        .and_then(|u| dae.get(u))
        .or_else(|| child(root, "library_visual_scenes").and_then(|l| child(l, "visual_scene")))
        .ok_or_else(|| Error::Invalid("sin escena".into()))?;
    let mut b = Builder::default();
    let mut stack: Vec<(Node, Mat4, usize)> =
        children(scene, "node").map(|n| (n, IDENTITY, 0)).collect();
    let mut effects: HashMap<String, (usize, Option<String>, Rgb)> = HashMap::new();
    while let Some((node, parent, depth)) = stack.pop() {
        if depth > 64 {
            continue;
        }
        let m = mul(&parent, &local(node));
        for c in node.children() {
            match c.tag_name().name() {
                "node" => stack.push((c, m, depth + 1)),
                "instance_node" => {
                    if let Some(n) = c.attribute("url").and_then(|u| dae.get(u)) {
                        stack.push((n, m, depth + 1));
                    }
                }
                "instance_geometry" => {
                    let Some(geo) = c.attribute("url").and_then(|u| dae.get(u)) else {
                        continue;
                    };
                    // symbol → material id.
                    let mut bound: HashMap<&str, &str> = HashMap::new();
                    for im in c
                        .descendants()
                        .filter(|d| d.tag_name().name() == "instance_material")
                    {
                        if let (Some(s), Some(t)) = (im.attribute("symbol"), im.attribute("target"))
                        {
                            bound.insert(s, url_id(t));
                        }
                    }
                    add_geometry(&mut dae, &mut b, &mut effects, geo, &bound, &m, up, unit)?;
                }
                _ => {}
            }
        }
    }
    Ok(b.mesh)
}

#[allow(clippy::too_many_arguments)]
fn add_geometry<'a, 'i>(
    dae: &mut Dae<'a, 'i>,
    b: &mut Builder,
    effects: &mut HashMap<String, (usize, Option<String>, Rgb)>,
    geo: Node<'a, 'i>,
    bound: &HashMap<&str, &str>,
    m: &Mat4,
    up: Up,
    unit: f64,
) -> Result<(), Error> {
    let Some(mesh) = child(geo, "mesh") else {
        return Ok(());
    };
    let flip = mirrors(m);
    let mut sources: HashMap<&str, (Vec<f64>, usize)> = HashMap::new();
    for s in children(mesh, "source") {
        if let Some(id) = s.attribute("id") {
            sources.insert(id, source(s));
        }
    }
    // The vertices element names the positions.
    let mut vertices: HashMap<&str, &str> = HashMap::new();
    for v in children(mesh, "vertices") {
        if let Some(id) = v.attribute("id") {
            for i in children(v, "input") {
                if i.attribute("semantic") == Some("POSITION") {
                    if let Some(s) = i.attribute("source") {
                        vertices.insert(id, url_id(s));
                    }
                }
            }
        }
    }
    for prim in mesh.children().filter(|c| {
        matches!(
            c.tag_name().name(),
            "triangles" | "polylist" | "polygons" | "trifans" | "tristrips"
        )
    }) {
        // The material: symbol bound in the instance, or the symbol itself
        // as a material id.
        let symbol = prim.attribute("material").unwrap_or("");
        let mat_id = bound.get(symbol).copied().unwrap_or(symbol).to_string();
        let (material, image, color) = match effects.get(&mat_id) {
            Some(e) => e.clone(),
            None => {
                let mat = dae.get(&mat_id);
                let effect = mat
                    .and_then(|m| child(m, "instance_effect"))
                    .and_then(|i| i.attribute("url"))
                    .and_then(|u| dae.get(u));
                let e = effect.map(|e| dae.effect(e)).unwrap_or(Effect {
                    color: [200, 200, 200],
                    alpha: 1.0,
                    image: None,
                });
                let name = mat
                    .and_then(|m| m.attribute("name"))
                    .unwrap_or(if mat_id.is_empty() {
                        "Predeterminado"
                    } else {
                        &mat_id
                    })
                    .to_string();
                let base = match e
                    .image
                    .clone()
                    .and_then(|f| dae.texture(&f).map(|t| t.average()))
                {
                    // SketchUp writes a texture with no colour: the
                    // texture's own.
                    Some(avg) => avg,
                    None => e.color,
                };
                let index = b.material(&name, || Material {
                    name: String::new(),
                    color: base,
                    alpha: e.alpha,
                    texture: e.image.clone(),
                });
                let entry = (index, e.image.clone(), e.color);
                effects.insert(mat_id.clone(), entry.clone());
                entry
            }
        };
        let base = b.mesh.materials[material].color;
        let inputs: Vec<Node> = children(prim, "input").collect();
        let stride = inputs
            .iter()
            .filter_map(|i| i.attribute("offset").and_then(|o| o.parse::<usize>().ok()))
            .max()
            .map_or(1, |m| m + 1);
        let mut pos_input = None;
        let mut uv_input = None;
        for i in &inputs {
            let offset: usize = i
                .attribute("offset")
                .and_then(|o| o.parse().ok())
                .unwrap_or(0);
            let src = url_id(i.attribute("source").unwrap_or(""));
            match i.attribute("semantic") {
                Some("VERTEX") => {
                    pos_input = vertices
                        .get(src)
                        .and_then(|s| sources.get(s))
                        .map(|s| (offset, s))
                }
                Some("POSITION") => pos_input = sources.get(src).map(|s| (offset, s)),
                Some("TEXCOORD") if uv_input.is_none() => {
                    uv_input = sources.get(src).map(|s| (offset, s))
                }
                _ => {}
            }
        }
        let Some((pos_off, (pos, pstride))) = pos_input else {
            continue;
        };
        // Polygons as lists of corners (each corner: its index tuple).
        let mut polys: Vec<Vec<usize>> = Vec::new();
        match prim.tag_name().name() {
            "triangles" => {
                let p = child(prim, "p").map(ints).unwrap_or_default();
                for t in p.chunks_exact(3 * stride) {
                    polys.push(t.to_vec());
                }
            }
            "polylist" => {
                let p = child(prim, "p").map(ints).unwrap_or_default();
                let counts = child(prim, "vcount").map(ints).unwrap_or_default();
                let mut at = 0;
                for c in counts {
                    let end = at + c * stride;
                    if end > p.len() {
                        break;
                    }
                    polys.push(p[at..end].to_vec());
                    at = end;
                }
            }
            "polygons" | "trifans" => {
                // Holes (`ph`) are not read.
                for p in children(prim, "p") {
                    polys.push(ints(p));
                }
            }
            _ => {
                for p in children(prim, "p") {
                    let p = ints(p);
                    let n = p.len() / stride;
                    for k in 0..n.saturating_sub(2) {
                        let corner = |j: usize| p[j * stride..(j + 1) * stride].to_vec();
                        let (a, c) = if k % 2 == 0 { (k, k + 1) } else { (k + 1, k) };
                        let mut t = corner(a);
                        t.extend(corner(c));
                        t.extend(corner(k + 2));
                        polys.push(t);
                    }
                }
            }
        }
        let tex = image.as_ref().and_then(|f| dae.texture(f));
        let mut made: HashMap<(usize, Option<usize>), u32> = HashMap::new();
        for poly in polys {
            let mut corners = Vec::new();
            for c in poly.chunks_exact(stride) {
                let pi = c[pos_off];
                let ti = uv_input.map(|(o, _)| c[o]);
                let id = match made.get(&(pi, ti)) {
                    Some(id) => *id,
                    None => {
                        let at = pi * pstride;
                        if at + 3 > pos.len() {
                            return Err(Error::Invalid(
                                "índice de vértice fuera de la lista".into(),
                            ));
                        }
                        let p = apply(m, [pos[at], pos[at + 1], pos[at + 2]]);
                        let color = match (tex, ti, uv_input) {
                            (Some(t), Some(ti), Some((_, (uv, us)))) if ti * us + 1 < uv.len() => {
                                let s = t.sample(uv[ti * us] as f32, uv[ti * us + 1] as f32, true);
                                if image.is_some() && color != [200, 200, 200] {
                                    tint(s, color)
                                } else {
                                    s
                                }
                            }
                            _ => base,
                        };
                        let id = b.vertex(to_ours(up, unit, p), color);
                        made.insert((pi, ti), id);
                        id
                    }
                };
                corners.push(id);
            }
            for k in 1..corners.len().saturating_sub(1) {
                let t = [corners[0], corners[k], corners[k + 1]];
                b.triangle(material, if flip { [t[0], t[2], t[1]] } else { t });
            }
        }
    }
    Ok(())
}
