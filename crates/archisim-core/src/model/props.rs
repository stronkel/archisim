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
        "fruit2" => &["fruit2", "fruit"],
        "stem" => &["stem", "frame"],
        "pillow" => &["pillow", "cushion"],
        "throw" => &["throw_mat", "cushion"],
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
        let mut parts: Vec<(MaterialId, Vec<[u32; 3]>)> = Vec::new();
        for (role, tris) in &mesh.parts {
            let m = candidates(role)
                .iter()
                .find_map(|c| material_of(c))
                .or_else(|| material_of(role))
                .unwrap_or(main);
            match parts.iter_mut().find(|(x, _)| *x == m) {
                Some((_, v)) => v.extend(tris.iter().copied()),
                None => parts.push((m, tris.clone())),
            }
        }
        Some((mesh, parts))
    }

    /// The model of an object, if ArchiSim makes it.
    #[allow(clippy::type_complexity)]
    pub fn object_prop(
        &self,
        id: ObjectId,
    ) -> Option<(Arc<props::PropMesh>, Vec<(MaterialId, Vec<[u32; 3]>)>)> {
        let o = self.objects.get(&id)?;
        self.type_prop(o.object_type)
    }
}
