//! 3D models brought from other programs (OBJ, glTF, COLLADA): kept in
//! the project as coloured triangle meshes, each with lighter versions for
//! far views (levels of detail). An object type with a model shows it in
//! 3D (and in the IFC) instead of its family's boxes; its family gives the
//! plan symbol, the size and the data, as for any other object.

use super::*;
use crate::Rgb;

/// The triangles of one material of a model.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshGroup {
    pub material: MaterialId,
    /// Indices into [`MeshLevel::positions`].
    pub triangles: Vec<[u32; 3]>,
}

/// One version of a model: the model itself, or a lighter one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeshLevel {
    /// Metres from the insertion point (the middle of its base), `z` up,
    /// at the size it was brought in.
    pub positions: Vec<[f32; 3]>,
    /// The colour of each vertex (its material's, or its texture's there).
    pub colors: Vec<Rgb>,
    pub groups: Vec<MeshGroup>,
}

impl MeshLevel {
    pub fn triangle_count(&self) -> usize {
        self.groups.iter().map(|g| g.triangles.len()).sum()
    }
}

/// A 3D model kept in the project.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshModel {
    pub id: MeshId,
    pub name: String,
    /// The file it was brought from (its name, for the record).
    pub source: String,
    /// The model first, then lighter versions, each lighter than the last.
    pub levels: Vec<MeshLevel>,
}

impl MeshModel {
    /// Its size as brought in: width (x), depth (y) and height (z).
    pub fn size(&self) -> [f64; 3] {
        let Some(l) = self.levels.first() else {
            return [0.0; 3];
        };
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in &l.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        if lo[0] > hi[0] {
            return [0.0; 3];
        }
        [0, 1, 2].map(|k| (hi[k] - lo[k]) as f64)
    }

    pub fn materials(&self) -> Vec<MaterialId> {
        let mut v: Vec<MaterialId> = self
            .levels
            .iter()
            .flat_map(|l| l.groups.iter().map(|g| g.material))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// The version with at most `max` triangles, or the lightest.
    pub fn level_under(&self, max: usize) -> Option<&MeshLevel> {
        self.levels
            .iter()
            .find(|l| l.triangle_count() <= max)
            .or_else(|| self.levels.last())
    }
}

/// Triangles of models that the 3D view and the drawings show at most
/// before taking the lighter versions of every model.
pub const MESH_BUDGET: usize = 400_000;

impl Model {
    fn check_mesh(&self, m: &MeshModel) -> Result<(), ModelError> {
        if m.levels.is_empty() || m.levels[0].triangle_count() == 0 {
            return Err(ModelError::InvalidMesh("el modelo no tiene caras".into()));
        }
        for (k, l) in m.levels.iter().enumerate() {
            if l.colors.len() != l.positions.len() {
                return Err(ModelError::InvalidMesh(format!(
                    "versión {k}: un color por vértice"
                )));
            }
            let n = l.positions.len() as u32;
            for g in &l.groups {
                if !self.materials.contains_key(&g.material) {
                    return Err(ModelError::UnknownMaterial(g.material));
                }
                if g.triangles.iter().flatten().any(|&i| i >= n) {
                    return Err(ModelError::InvalidMesh(format!(
                        "versión {k}: un triángulo fuera de los vértices"
                    )));
                }
            }
            if l.positions.iter().flatten().any(|c| !c.is_finite()) {
                return Err(ModelError::InvalidMesh(format!(
                    "versión {k}: coordenadas no válidas"
                )));
            }
        }
        Ok(())
    }

    /// Keeps a 3D model in the project (its materials must be there).
    pub fn add_mesh(&mut self, mut m: MeshModel) -> Result<MeshId, ModelError> {
        m.id = MeshId(self.take_id());
        self.check_mesh(&m)?;
        let id = m.id;
        self.meshes.insert(id, m);
        Ok(id)
    }

    pub fn mesh(&self, id: MeshId) -> Option<&MeshModel> {
        self.meshes.get(&id)
    }

    pub fn meshes(&self) -> impl Iterator<Item = &MeshModel> {
        self.meshes.values()
    }

    /// The object types that show a model.
    pub fn mesh_users(&self, id: MeshId) -> Vec<String> {
        self.object_types
            .values()
            .filter(|t| t.mesh == Some(id))
            .map(|t| t.name.clone())
            .collect()
    }

    /// Takes a model out of the project, if no type shows it.
    pub fn remove_mesh(&mut self, id: MeshId) -> Result<MeshModel, ModelError> {
        if !self.meshes.contains_key(&id) {
            return Err(ModelError::UnknownMesh(id));
        }
        let users = self.mesh_users(id);
        if !users.is_empty() {
            return Err(ModelError::InUse(users.join(", ")));
        }
        Ok(self.meshes.remove(&id).expect("checked above"))
    }

    /// The model an object type shows in 3D, or none (its family's boxes).
    pub fn set_object_type_mesh(
        &mut self,
        id: ObjectTypeId,
        mesh: Option<MeshId>,
    ) -> Result<(), ModelError> {
        if let Some(m) = mesh {
            if !self.meshes.contains_key(&m) {
                return Err(ModelError::UnknownMesh(m));
            }
        }
        let t = self
            .object_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownObjectType(id))?;
        t.mesh = mesh;
        Ok(())
    }

    /// The model of an object, the factor it is scaled by (its type's
    /// height over the model's) and which version the views take: every
    /// object the same, the heaviest that keeps all the models in the
    /// project under [`MESH_BUDGET`] triangles.
    pub fn object_mesh(&self, id: ObjectId) -> Option<(&MeshModel, f64, &MeshLevel)> {
        let o = self.objects.get(&id)?;
        let t = self.object_types.get(&o.object_type)?;
        let mesh = self.meshes.get(&t.mesh?)?;
        let natural = mesh.size()[2];
        let k = match self.object_height(t.id) {
            Some(h) if natural > 1e-6 => h / natural,
            _ => 1.0,
        };
        let detail = self.mesh_detail();
        let level = mesh
            .levels
            .get(detail.min(mesh.levels.len() - 1))
            .unwrap_or(&mesh.levels[0]);
        Some((mesh, k, level))
    }

    /// The height of an object type: its family's `height` value, if it
    /// has one.
    fn object_height(&self, id: ObjectTypeId) -> Option<f64> {
        let t = self.object_types.get(&id)?;
        let mut v = self.family_defaults(t.family, ParamScope::Type).ok()?;
        v.extend(t.values.clone());
        match v.get("height")? {
            Value::Number(h) => Some(*h),
            _ => None,
        }
    }

    /// Which version of the models the views take (0: the models
    /// themselves).
    pub fn mesh_detail(&self) -> usize {
        let mut per_level = [0usize; 3];
        for o in self.objects.values() {
            let Some(m) = self
                .object_types
                .get(&o.object_type)
                .and_then(|t| t.mesh)
                .and_then(|m| self.meshes.get(&m))
            else {
                continue;
            };
            for (k, total) in per_level.iter_mut().enumerate() {
                let l = &m.levels[k.min(m.levels.len() - 1)];
                *total += l.triangle_count();
            }
        }
        per_level
            .iter()
            .position(|t| *t <= MESH_BUDGET)
            .unwrap_or(per_level.len() - 1)
    }

    pub(super) fn restore_meshes(
        &mut self,
        meshes: Vec<MeshModel>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for m in meshes {
            let name = format!("3D model {} «{}»", m.id.0, m.name);
            claim(m.id.0, None)
                .and_then(|_| self.check_mesh(&m))
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.meshes.insert(m.id, m);
        }
        Ok(())
    }
}
