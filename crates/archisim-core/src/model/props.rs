//! Objects whose 3D model ArchiSim makes from their measurements (crate
//! `archisim-props`): those whose family says `generator = "props:sofa"`
//! (or `chair`, `wc`, `car`…) in its outputs. The model is made from the
//! type's values (and remade when they change), each piece in the
//! material of the family parameter named as its role; the plan symbol
//! is the recipe's, as for any object. A 3D model brought from another
//! program, if the type has one, comes first.

use super::*;
use archisim_family::Expr;
use archisim_props as props;
use std::sync::Arc;

/// The materials a role takes, in order of preference (the family's
/// parameter of that name, or of a related one), before the main one.
fn candidates(role: &str) -> &'static [&'static str] {
    match role {
        "metal" => &["metal", "frame"],
        "frame" => &["frame", "metal"],
        "cushion" => &["cushion"],
        "linen" => &["linen", "cushion"],
        "duvet" => &["duvet", "linen", "cushion"],
        "shade" => &["shade", "linen"],
        "books" => &["books"],
        "books2" => &["books2", "books"],
        "books3" => &["books3", "books2", "books"],
        "top" => &["top"],
        "plinth" => &["plinth"],
        "dark" => &["dark", "frame"],
        "ring" => &["ring", "metal", "frame"],
        "screen" => &["screen", "dark", "frame"],
        "seat" => &["seat"],
        "water" => &["water", "glass"],
        "glass" => &["glass"],
        "unit" => &["unit"],
        "light" => &["light"],
        "rear" => &["rear", "light"],
        "trim" => &["trim", "tyre"],
        "hub" => &["hub", "metal", "frame"],
        "body" => &["body_mat", "body"],
        "tyre" => &["tyre"],
        // The cosy pieces (v0.49).
        "pot" => &["pot_mat", "pot"],
        "bulb" => &["bulb", "light"],
        "flame" => &["flame", "bulb", "light"],
        "flame2" => &["flame2", "flame", "bulb", "light"],
        // The workshop's fireplaces call their firewood `wood` (`logs`
        // is whether there is any).
        "logs" => &["wood", "logs"],
        "fruit2" => &["fruit2", "fruit"],
        "stem" => &["stem", "frame"],
        "pillow" => &["pillow", "cushion"],
        "throw" => &["throw_mat", "cushion"],
        // The collections' pieces.
        "spread" => &["spread_mat", "duvet"],
        "accent" => &["accent", "pillow", "cushion"],
        "weave" => &["weave", "frame"],
        "mat2" => &["mat2"],
        "flowers" => &["flowers", "leaves"],
        // People (C1): face and hands, hair.
        "skin" => &["skin"],
        "hair" => &["hair"],
        // The workshop's sink and tap (M1).
        "sink" => &["sink", "metal"],
        "spadix" => &["spadix", "flowers"],
        "leaves2" => &["leaves2", "leaves"],
        // The bedrooms' children's pieces and valet stands (paquete 3):
        // `toy` and `clothes` are also whether there is one.
        "toy" => &["toy_mat", "pillow", "cushion"],
        "clothes" => &["clothes_mat"],
        // The terraces' climber (paquete 5a).
        "vine_stem" => &["vine_stem", "stem", "frame"],
        "vine" => &["vine", "leaves"],
        "vine2" => &["vine2", "vine", "leaves2", "leaves"],
        "bracts" => &["bracts", "flowers", "leaves"],
        _ => &[],
    }
}

impl Model {
    /// The value of a family's `generator` output, if it has one.
    pub fn family_generator(&self, family: FamilyId) -> Option<String> {
        let f = self.families.get(&family)?;
        f.def()
            .outputs
            .iter()
            .find(|o| o.name == "generator")
            .and_then(|o| match &o.expr {
                Expr::Str(s) => Some(s.clone()),
                _ => None,
            })
    }

    /// What ArchiSim makes for objects of a family (`sofa`, `wc`…), if it
    /// makes their model.
    pub fn family_prop(&self, family: FamilyId) -> Option<String> {
        self.family_generator(family)?
            .strip_prefix("props:")
            .map(str::to_string)
    }

    /// Whether objects of a type have a model made by ArchiSim (and not one
    /// brought from another program).
    pub fn type_has_prop(&self, t: ObjectTypeId) -> bool {
        self.object_types
            .get(&t)
            .is_some_and(|ty| ty.mesh.is_none() && self.family_prop(ty.family).is_some())
    }

    /// The model of objects of a type, and its triangles by material.
    #[allow(clippy::type_complexity)]
    pub fn type_prop(
        &self,
        t: ObjectTypeId,
    ) -> Option<(Arc<props::PropMesh>, Vec<(MaterialId, Vec<[u32; 3]>)>)> {
        self.type_prop_without(t, &[])
    }

    /// [`Model::type_prop`] leaving out the pieces of some roles (a
    /// base module in a worktop run leaves out its own `top`).
    #[allow(clippy::type_complexity)]
    pub fn type_prop_without(
        &self,
        t: ObjectTypeId,
        skip: &[&str],
    ) -> Option<(Arc<props::PropMesh>, Vec<(MaterialId, Vec<[u32; 3]>)>)> {
        let (mesh, roles) = self.prop_roles(t)?;
        let mut parts: Vec<(MaterialId, Vec<[u32; 3]>)> = Vec::new();
        for ((role, tris), (_, m)) in mesh.parts.iter().zip(&roles) {
            if skip.contains(role) {
                continue;
            }
            match parts.iter_mut().find(|(x, _)| x == m) {
                Some((_, v)) => v.extend(tris.iter().copied()),
                None => parts.push((*m, tris.clone())),
            }
        }
        Some((mesh, parts))
    }

    /// The generator of a type made by ArchiSim and the measurements it is
    /// made with (its values, choices by number, the model by number).
    pub fn prop_params(&self, t: ObjectTypeId) -> Option<(String, props::Params)> {
        let ty = self.object_types.get(&t)?;
        if ty.mesh.is_some() {
            return None;
        }
        let kind = self.family_prop(ty.family)?;
        let mut values = self.family_defaults(ty.family, ParamScope::Type).ok()?;
        values.extend(ty.values.clone());
        let mut params = props::Params::default();
        for (k, v) in &values {
            match v {
                Value::Number(n) => params.set(k, *n),
                Value::Bool(b) => params.set(k, if *b { 1.0 } else { 0.0 }),
                // Handles, a desk's drawers: chosen by name.
                Value::Text(t) => {
                    if let Some(n) = props::choice_number(k, t) {
                        params.set(k, n);
                    }
                }
                _ => {}
            }
        }
        // The model chosen by name.
        if let Some(Value::Text(name)) = values.get("model3d") {
            if let Some(i) = props::styles(&kind).iter().position(|s| s == name) {
                params.set("style", (i + 1) as f64);
            }
        }
        Some((kind, params))
    }

    /// The model of a type and the material of each of its roles (in the
    /// order of its parts).
    #[allow(clippy::type_complexity)]
    fn prop_roles(
        &self,
        t: ObjectTypeId,
    ) -> Option<(Arc<props::PropMesh>, Vec<(&'static str, MaterialId)>)> {
        let ty = self.object_types.get(&t)?;
        if ty.mesh.is_some() {
            return None;
        }
        let (kind, params) = self.prop_params(t)?;
        let values = {
            let mut v = self.family_defaults(ty.family, ParamScope::Type).ok()?;
            v.extend(ty.values.clone());
            v
        };
        let mesh = props::mesh_of(&kind, &params)?;
        let material_of = |name: &str| match values.get(name) {
            Some(Value::Material(m)) if self.materials.contains_key(&MaterialId(*m)) => {
                Some(MaterialId(*m))
            }
            _ => None,
        };
        let main = material_of("mat")
            .or_else(|| {
                values.iter().find_map(|(_, v)| match v {
                    Value::Material(m) if self.materials.contains_key(&MaterialId(*m)) => {
                        Some(MaterialId(*m))
                    }
                    _ => None,
                })
            })
            .or_else(|| self.materials.keys().next().copied())?;
        let roles = mesh
            .parts
            .iter()
            .map(|(role, _)| {
                let m = candidates(role)
                    .iter()
                    .find_map(|c| material_of(c))
                    .or_else(|| material_of(role))
                    .unwrap_or(main);
                (*role, m)
            })
            .collect();
        Some((mesh, roles))
    }

    /// The materials of an object drawn as stuffed fabric (see
    /// `archisim_props::PropMesh::soft`): soft shading, outlined only
    /// where they end. A material also on a hard piece of the same model
    /// is drawn as before.
    pub fn object_soft_materials(&self, id: ObjectId) -> Vec<MaterialId> {
        let Some(o) = self.objects.get(&id) else {
            return Vec::new();
        };
        let Some((mesh, roles)) = self.prop_roles(o.object_type) else {
            return Vec::new();
        };
        let (soft, hard): (Vec<_>, Vec<_>) = roles.iter().partition(|(r, _)| mesh.soft.contains(r));
        let mut out: Vec<MaterialId> = soft
            .into_iter()
            .map(|(_, m)| m)
            .filter(|m| !hard.iter().any(|(_, h)| h == m))
            .collect();
        out.dedup();
        out
    }

    /// The materials of an object's model that are many small leaves
    /// (`PropMesh::fine`): the 3D view draws them without contour lines.
    pub fn object_fine_materials(&self, id: ObjectId) -> Vec<MaterialId> {
        let Some(o) = self.objects.get(&id) else {
            return Vec::new();
        };
        let Some((mesh, roles)) = self.prop_roles(o.object_type) else {
            return Vec::new();
        };
        let (fine, other): (Vec<_>, Vec<_>) =
            roles.iter().partition(|(r, _)| mesh.fine.contains(r));
        let mut out: Vec<MaterialId> = fine
            .into_iter()
            .map(|(_, m)| m)
            .filter(|m| !other.iter().any(|(_, h)| h == m))
            .collect();
        out.dedup();
        out
    }

    #[allow(clippy::type_complexity)]
    pub fn object_prop(
        &self,
        id: ObjectId,
    ) -> Option<(Arc<props::PropMesh>, Vec<(MaterialId, Vec<[u32; 3]>)>)> {
        let o = self.objects.get(&id)?;
        // A base module under a worktop run: the run has the worktop.
        if self.worktop_run_of(id).is_some() {
            return self.type_prop_without(o.object_type, &["top"]);
        }
        self.type_prop(o.object_type)
    }
}

impl Model {
    /// A door or window whose family makes its detailed 3D
    /// (`generator = "props:door"`, paquete 9): the generator and the
    /// measurements it is made with — the outputs of the family evaluated
    /// for the detailed 3D (where its leaves go), its type's and the
    /// opening's own values (choices by number, the model by number), and
    /// what the caller adds (`extra`: where the wall's faces are, the
    /// mirror…). `None` for a family without one.
    pub fn opening_prop_params(
        &self,
        id: OpeningId,
        extra: &[(&str, f64)],
    ) -> Option<(String, props::Params)> {
        let o = self.openings.get(&id)?;
        let t = self.opening_types.get(&o.opening_type)?;
        let kind = self.family_prop(t.family)?;
        let ev = self.evaluate_opening_detailed(id).ok()?;
        let mut values = self.family_defaults(t.family, ParamScope::Type).ok()?;
        values.extend(t.values.clone());
        values.extend(o.values.clone());
        let mut params = props::Params::default();
        for (k, v) in values.iter().chain(ev.outputs.iter()) {
            match v {
                Value::Number(n) => params.set(k, *n),
                Value::Bool(b) => params.set(k, if *b { 1.0 } else { 0.0 }),
                Value::Text(s) => {
                    if let Some(n) = props::choice_number(k, s) {
                        params.set(k, n);
                    }
                }
                _ => {}
            }
        }
        if let Some(Value::Text(name)) = values.get("model3d") {
            if let Some(i) = props::styles(&kind).iter().position(|s| s == name) {
                params.set("style", (i + 1) as f64);
            }
        }
        for (k, v) in extra {
            params.set(k, *v);
        }
        Some((kind, params))
    }

    /// The detailed model of a door or window (see
    /// [`Model::opening_prop_params`]) and its triangles by material: each
    /// role is the family's material parameter of its name (the frame's
    /// for one the family does not have).
    #[allow(clippy::type_complexity)]
    pub fn opening_prop(
        &self,
        id: OpeningId,
        extra: &[(&str, f64)],
    ) -> Option<(Arc<props::PropMesh>, Vec<(MaterialId, Vec<[u32; 3]>)>)> {
        let (kind, params) = self.opening_prop_params(id, extra)?;
        let o = self.openings.get(&id)?;
        let t = self.opening_types.get(&o.opening_type)?;
        let mut values = self.family_defaults(t.family, ParamScope::Type).ok()?;
        values.extend(t.values.clone());
        let mesh = props::mesh_of(&kind, &params)?;
        let material_of = |name: &str| match values.get(name) {
            Some(Value::Material(m)) if self.materials.contains_key(&MaterialId(*m)) => {
                Some(MaterialId(*m))
            }
            _ => None,
        };
        let main =
            material_of("frame_material").or_else(|| self.materials.keys().next().copied())?;
        let mut parts: Vec<(MaterialId, Vec<[u32; 3]>)> = Vec::new();
        for (role, tris) in &mesh.parts {
            let m = material_of(role).unwrap_or(main);
            match parts.iter_mut().find(|(x, _)| *x == m) {
                Some((_, v)) => v.extend(tris.iter().copied()),
                None => parts.push((m, tris.clone())),
            }
        }
        Some((mesh, parts))
    }
}
