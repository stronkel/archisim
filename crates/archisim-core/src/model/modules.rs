//! Modules: another ArchiSim project placed in this one, as many times as
//! needed, each copy moved, turned and, if asked, mirrored (a masterplan
//! that places its house types; ArchiCAD's hotlinked modules, Revit's
//! links with several copies).
//!
//! The project keeps a copy of each module file as it was when it was
//! loaded or last updated ([`ModuleSource`]), so it opens the same on any
//! computer; the program compares it with the file on disk to say when a
//! module has changed. When a module is loaded or updated, its types and
//! materials are brought into the project (by `GlobalId`; materials by
//! name), so that the copies are measured and exported with the project's
//! own types.
//!
//! The copies are not elements of the project: [`Model::flattened`] makes
//! a model with every copy's elements in it, with ids of their own and
//! `GlobalId`s worked out from the copy's and the element's, the same in
//! every export. Views, schedules and exports draw that model. A copy is
//! changed only as a whole: to have something different, another module.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::*;

/// A module file kept in the project.
#[derive(Debug, Clone)]
pub struct ModuleSource {
    pub id: ModuleSourceId,
    /// What it is called in the project («Vivienda tipo B»).
    pub name: String,
    /// Where its file is: relative to the project's folder when it can be.
    pub path: String,
    /// The module file as it was loaded (its text).
    pub text: Arc<str>,
    /// Fingerprint of the file on disk when it was loaded, to tell when it
    /// changes ([`text_stamp`]).
    pub stamp: String,
    /// The module, read.
    pub model: Arc<Model>,
}

/// The model is what the text says: comparing the texts is enough (and
/// does not walk the whole module).
impl PartialEq for ModuleSource {
    fn eq(&self, o: &Self) -> bool {
        self.id == o.id
            && self.name == o.name
            && self.path == o.path
            && self.stamp == o.stamp
            && (Arc::ptr_eq(&self.text, &o.text) || self.text == o.text)
            && (Arc::ptr_eq(&self.model, &o.model) || self.text == o.text)
    }
}

/// A copy of a module in the project.
#[derive(Debug, Clone, PartialEq)]
pub struct ModulePlacement {
    pub id: ModuleId,
    pub guid: GlobalId,
    pub source: ModuleSourceId,
    /// The level the module's lowest level goes on; the others go on the
    /// levels above, in order.
    pub level: LevelId,
    /// Where the module's origin goes.
    pub position: Point2,
    /// Turn of the module, radians, counter-clockwise.
    pub rotation: f64,
    /// Mirrored about the module's own `y` axis (before turning).
    pub mirror: bool,
}

/// Data needed to place a copy of a module.
#[derive(Debug, Clone, PartialEq)]
pub struct NewModulePlacement {
    pub source: ModuleSourceId,
    pub level: LevelId,
    pub position: Point2,
    pub rotation: f64,
    pub mirror: bool,
}

/// The fingerprint of a file's text (FNV-1a, 64 bits, in hex).
pub fn text_stamp(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// A `GlobalId` worked out from two others (FNV-1a, 128 bits): the same
/// pair gives the same one in every export.
pub fn combined_guid(a: &GlobalId, b: &GlobalId) -> GlobalId {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for x in a.as_str().bytes().chain(*b"/").chain(b.as_str().bytes()) {
        h ^= x as u128;
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    GlobalId::from_u128(h)
}

/// Where a copy puts the module: mirrored (if asked), turned and moved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModuleTransform {
    pub position: Point2,
    pub rotation: f64,
    pub mirror: bool,
}

impl ModuleTransform {
    pub fn of(p: &ModulePlacement) -> Self {
        ModuleTransform {
            position: p.position,
            rotation: p.rotation,
            mirror: p.mirror,
        }
    }

    /// A point of the module in the project.
    pub fn apply(&self, q: Point2) -> Point2 {
        let x = if self.mirror { -q.x } else { q.x };
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        Point2::new(
            self.position.x + x * c - q.y * s,
            self.position.y + x * s + q.y * c,
        )
    }

    /// A direction (radians) of the module in the project.
    pub fn angle(&self, a: f64) -> f64 {
        let a = if self.mirror {
            std::f64::consts::PI - a
        } else {
            a
        };
        a + self.rotation
    }
}

impl Model {
    // ---- Sources ------------------------------------------------------------

    pub fn module_sources(&self) -> impl Iterator<Item = &ModuleSource> {
        self.module_sources.values()
    }

    pub fn module_source(&self, id: ModuleSourceId) -> Option<&ModuleSource> {
        self.module_sources.get(&id)
    }

    /// Loads a module file into the project (its types and materials too).
    /// `text` is the file and `model` that file read.
    pub fn add_module_source(
        &mut self,
        name: impl Into<String>,
        path: impl Into<String>,
        text: Arc<str>,
        stamp: impl Into<String>,
        model: Model,
    ) -> Result<ModuleSourceId, ModelError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ModelError::InvalidShape("a module needs a name"));
        }
        if model.levels().is_empty() {
            return Err(ModelError::InvalidShape("a module needs a level"));
        }
        self.adopt_types(&model)?;
        let id = ModuleSourceId(self.take_id());
        self.module_sources.insert(
            id,
            ModuleSource {
                id,
                name,
                path: path.into(),
                text,
                stamp: stamp.into(),
                model: Arc::new(model),
            },
        );
        Ok(id)
    }

    /// Takes a newer version of a module file: every copy changes with it,
    /// and so do the project's copies of its types.
    pub fn update_module_source(
        &mut self,
        id: ModuleSourceId,
        text: Arc<str>,
        stamp: impl Into<String>,
        model: Model,
    ) -> Result<(), ModelError> {
        if !self.module_sources.contains_key(&id) {
            return Err(ModelError::UnknownElement(id.0));
        }
        if model.levels().is_empty() {
            return Err(ModelError::InvalidShape("a module needs a level"));
        }
        self.adopt_types(&model)?;
        let s = self.module_sources.get_mut(&id).expect("checked");
        s.text = text;
        s.stamp = stamp.into();
        s.model = Arc::new(model);
        Ok(())
    }

    pub fn rename_module_source(
        &mut self,
        id: ModuleSourceId,
        name: impl Into<String>,
        path: Option<String>,
    ) -> Result<(), ModelError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ModelError::InvalidShape("a module needs a name"));
        }
        let s = self
            .module_sources
            .get_mut(&id)
            .ok_or(ModelError::UnknownElement(id.0))?;
        s.name = name;
        if let Some(p) = path {
            s.path = p;
        }
        Ok(())
    }

    /// Removes a module file and every copy of it.
    pub fn remove_module_source(&mut self, id: ModuleSourceId) -> Result<ModuleSource, ModelError> {
        let s = self
            .module_sources
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))?;
        self.modules.retain(|_, p| p.source != id);
        Ok(s)
    }

    /// How many copies a module has.
    pub fn module_copies(&self, id: ModuleSourceId) -> usize {
        self.modules.values().filter(|p| p.source == id).count()
    }

    // ---- Copies -------------------------------------------------------------

    fn validate_module(&self, p: &NewModulePlacement) -> Result<(), ModelError> {
        if !self.module_sources.contains_key(&p.source) {
            return Err(ModelError::UnknownElement(p.source.0));
        }
        if !self.levels.contains_key(&p.level) {
            return Err(ModelError::UnknownLevel(p.level));
        }
        finite_point(p.position, "module position")?;
        finite(p.rotation, "module rotation")?;
        Ok(())
    }

    /// Adds the levels a copy on `level` needs that the project lacks: the
    /// module's upper levels, at the same heights above it. Returns how
    /// many it added.
    pub fn add_levels_for_module(
        &mut self,
        source: ModuleSourceId,
        level: LevelId,
    ) -> Result<usize, ModelError> {
        let s = self
            .module_sources
            .get(&source)
            .ok_or(ModelError::UnknownElement(source.0))?;
        let mut theirs: Vec<(String, f64)> = s
            .model
            .levels
            .values()
            .map(|l| (l.name.clone(), l.elevation))
            .collect();
        theirs.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut ours: Vec<(LevelId, f64)> =
            self.levels.values().map(|l| (l.id, l.elevation)).collect();
        ours.sort_by(|a, b| a.1.total_cmp(&b.1));
        let start = ours
            .iter()
            .position(|(id, _)| *id == level)
            .ok_or(ModelError::UnknownLevel(level))?;
        let base = ours[start].1;
        let mut added = 0;
        for (i, (name, elevation)) in theirs.iter().enumerate().skip(1) {
            if start + i < ours.len() {
                continue;
            }
            let mut n = name.clone();
            let mut k = 2;
            while self.levels.values().any(|l| l.name == n) {
                n = format!("{name} ({k})");
                k += 1;
            }
            self.add_level(n, base + (elevation - theirs[0].1));
            added += 1;
        }
        Ok(added)
    }

    pub fn add_module(&mut self, spec: NewModulePlacement) -> Result<ModuleId, ModelError> {
        self.validate_module(&spec)?;
        let id = ModuleId(self.take_id());
        self.modules.insert(
            id,
            ModulePlacement {
                id,
                guid: GlobalId::new(),
                source: spec.source,
                level: spec.level,
                position: spec.position,
                rotation: spec.rotation,
                mirror: spec.mirror,
            },
        );
        Ok(id)
    }

    pub fn edit_module(
        &mut self,
        id: ModuleId,
        edit: impl FnOnce(&mut ModulePlacement),
    ) -> Result<(), ModelError> {
        let mut p = self
            .modules
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        edit(&mut p);
        self.validate_module(&NewModulePlacement {
            source: p.source,
            level: p.level,
            position: p.position,
            rotation: p.rotation,
            mirror: p.mirror,
        })?;
        self.modules.insert(id, p);
        Ok(())
    }

    pub fn remove_module(&mut self, id: ModuleId) -> Result<ModulePlacement, ModelError> {
        self.modules
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))
    }

    pub fn module(&self, id: ModuleId) -> Option<&ModulePlacement> {
        self.modules.get(&id)
    }

    pub fn modules(&self) -> impl Iterator<Item = &ModulePlacement> {
        self.modules.values()
    }

    pub fn modules_on_level(&self, level: LevelId) -> impl Iterator<Item = &ModulePlacement> {
        self.modules.values().filter(move |p| p.level == level)
    }

    /// Whether the project places any module.
    pub fn has_modules(&self) -> bool {
        !self.modules.is_empty()
    }

    /// The outline of a copy in plan: the box around the module's walls,
    /// slabs, roofs and columns, placed.
    pub fn module_outline(&self, id: ModuleId) -> Vec<Point2> {
        let Some(p) = self.modules.get(&id) else {
            return Vec::new();
        };
        let Some(s) = self.module_sources.get(&p.source) else {
            return Vec::new();
        };
        let Some((lo, hi)) = footprint(&s.model) else {
            return Vec::new();
        };
        let t = ModuleTransform::of(p);
        [lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)]
            .into_iter()
            .map(|q| t.apply(q))
            .collect()
    }

    // ---- Types --------------------------------------------------------------

    /// A material of `src` in this project: the one of the same name, or a
    /// copy.
    fn adopt_material(&mut self, src: &Model, id: MaterialId) -> MaterialId {
        let Some(m) = src.material(id) else {
            return id;
        };
        if let Some(found) = self.materials().find(|x| x.name == m.name) {
            return found.id;
        }
        let new = self.add_material(
            m.name.clone(),
            m.category.clone(),
            m.cut_color,
            m.cut_pattern,
        );
        if let Some(x) = self.materials.get_mut(&new) {
            x.priority = m.priority;
        }
        new
    }

    /// Brings the types of a module into the project, with their
    /// `GlobalId`s: a type the project already has takes the module's
    /// values. A type of the same name and the same definition (both made
    /// from the same template) is the project's: it is not brought twice.
    /// Materials go by name.
    fn adopt_types(&mut self, src: &Model) -> Result<(), ModelError> {
        for t in src.wall_types() {
            if self.twin_wall_type(src, t.id).is_none() {
                let (entry, mats) = src.layered_entry(LayeredRef::Wall(t.id))?;
                self.load_layered_type(&entry, &mats, OnConflict::Replace)?;
            }
        }
        for t in src.slab_types() {
            if self.twin_slab_type(src, t.id).is_none() {
                let (entry, mats) = src.layered_entry(LayeredRef::Slab(t.id))?;
                self.load_layered_type(&entry, &mats, OnConflict::Replace)?;
            }
        }
        for (t, _, _) in src.family_types() {
            if self.twin_family_type(src, t).is_none() {
                let e = src.catalog_entry(t)?;
                self.load_catalog_type(&e.recipe, &e.entry, &e.materials, OnConflict::Replace)?;
            }
        }
        for t in src.member_types() {
            if self.twin_member_type(src, t.id).is_some() {
                continue;
            }
            let material = self.adopt_material(src, t.material);
            let found = self.member_types().find(|x| x.guid == t.guid).map(|x| x.id);
            match found {
                Some(id) => self.edit_member_type(id, t.name.clone(), material, t.profile)?,
                None => {
                    let name = free_name(self.member_types().map(|x| x.name.as_str()), &t.name);
                    let id = self.add_member_type(name, t.usage, material, t.profile)?;
                    if let Some(x) = self.member_types.get_mut(&id) {
                        x.guid = t.guid.clone();
                    }
                }
            }
        }
        for t in src.curtain_types() {
            if self.twin_curtain_type(src, t.id).is_some() {
                continue;
            }
            let mut design = t.design.clone();
            for m in design_materials(&mut design) {
                *m = self.adopt_material(src, *m);
            }
            let found = self
                .curtain_types()
                .find(|x| x.guid == t.guid)
                .map(|x| x.id);
            match found {
                Some(id) => {
                    let name = self
                        .curtain_type(id)
                        .map(|x| x.name.clone())
                        .unwrap_or_default();
                    self.edit_curtain_type(id, name, design)?;
                }
                None => {
                    let name = free_name(self.curtain_types().map(|x| x.name.as_str()), &t.name);
                    let id = self.add_curtain_type(name, design)?;
                    if let Some(x) = self.curtain_types.get_mut(&id) {
                        x.guid = t.guid.clone();
                    }
                }
            }
        }
        // The materials elements use directly.
        let direct: Vec<MaterialId> = src
            .stairs()
            .map(|s| s.material)
            .chain(
                src.railings()
                    .flat_map(|r| [Some(r.material), r.glass])
                    .flatten(),
            )
            .chain(src.footings().map(|f| f.material))
            .collect();
        for m in direct {
            self.adopt_material(src, m);
        }
        Ok(())
    }

    // ---- The project's type for a module's -------------------------------

    /// The project's type for each type of `src`.
    fn type_maps(&self, src: &Model) -> TypeMaps {
        let fam = |t: FamilyTypeRef| self.twin_family_type(src, t);
        TypeMaps {
            walls: src
                .wall_types()
                .filter_map(|t| Some((t.id, self.twin_wall_type(src, t.id)?)))
                .collect(),
            slabs: src
                .slab_types()
                .filter_map(|t| Some((t.id, self.twin_slab_type(src, t.id)?)))
                .collect(),
            openings: src
                .opening_types()
                .filter_map(|t| match fam(FamilyTypeRef::Opening(t.id))? {
                    FamilyTypeRef::Opening(x) => Some((t.id, x)),
                    FamilyTypeRef::Object(_) => None,
                })
                .collect(),
            objects: src
                .object_types()
                .filter_map(|t| match fam(FamilyTypeRef::Object(t.id))? {
                    FamilyTypeRef::Object(x) => Some((t.id, x)),
                    FamilyTypeRef::Opening(_) => None,
                })
                .collect(),
            members: src
                .member_types()
                .filter_map(|t| Some((t.id, self.twin_member_type(src, t.id)?)))
                .collect(),
            curtains: src
                .curtain_types()
                .filter_map(|t| Some((t.id, self.twin_curtain_type(src, t.id)?)))
                .collect(),
        }
    }

    /// The project's wall type for one of `src`: the one of its `GlobalId`
    /// or, failing that, one of the same name and layers.
    fn twin_wall_type(&self, src: &Model, id: WallTypeId) -> Option<WallTypeId> {
        let t = src.wall_type(id)?;
        if let Some(x) = self.wall_types().find(|x| x.guid == t.guid) {
            return Some(x.id);
        }
        let (mine, _) = src.layered_entry(LayeredRef::Wall(id)).ok()?;
        self.wall_types()
            .filter(|x| x.name == t.name)
            .find(|x| {
                self.layered_entry(LayeredRef::Wall(x.id))
                    .is_ok_and(|(e, _)| e.layers == mine.layers && e.kind == mine.kind)
            })
            .map(|x| x.id)
    }

    fn twin_slab_type(&self, src: &Model, id: SlabTypeId) -> Option<SlabTypeId> {
        let t = src.slab_type(id)?;
        if let Some(x) = self.slab_types().find(|x| x.guid == t.guid) {
            return Some(x.id);
        }
        let (mine, _) = src.layered_entry(LayeredRef::Slab(id)).ok()?;
        self.slab_types()
            .filter(|x| x.name == t.name)
            .find(|x| {
                self.layered_entry(LayeredRef::Slab(x.id))
                    .is_ok_and(|(e, _)| e.layers == mine.layers && e.kind == mine.kind)
            })
            .map(|x| x.id)
    }

    fn twin_family_type(&self, src: &Model, t: FamilyTypeRef) -> Option<FamilyTypeRef> {
        let (_, _, guid) = src.type_parts(t).ok()?;
        let guid = guid.clone();
        let ours = self.family_types();
        if let Some((x, _, _)) = ours
            .iter()
            .find(|(x, _, _)| self.type_parts(*x).is_ok_and(|(_, _, g)| *g == guid))
        {
            return Some(*x);
        }
        let mine = src.catalog_entry(t).ok()?;
        let same = |e: &CatalogEntry| {
            e.family_name == mine.family_name
                && e.recipe == mine.recipe
                && e.entry.name == mine.entry.name
                && e.entry.values == mine.entry.values
        };
        ours.iter()
            .map(|(x, _, _)| *x)
            .filter(|x| {
                matches!(
                    (x, t),
                    (FamilyTypeRef::Opening(_), FamilyTypeRef::Opening(_))
                        | (FamilyTypeRef::Object(_), FamilyTypeRef::Object(_))
                )
            })
            .find(|x| self.catalog_entry(*x).is_ok_and(|e| same(&e)))
    }

    fn twin_member_type(&self, src: &Model, id: MemberTypeId) -> Option<MemberTypeId> {
        let t = src.member_type(id)?;
        if let Some(x) = self.member_types().find(|x| x.guid == t.guid) {
            return Some(x.id);
        }
        let mat = src.material(t.material).map(|m| m.name.clone());
        self.member_types()
            .find(|x| {
                x.name == t.name
                    && x.usage == t.usage
                    && x.profile == t.profile
                    && self.material(x.material).map(|m| m.name.clone()) == mat
            })
            .map(|x| x.id)
    }

    fn twin_curtain_type(&self, src: &Model, id: CurtainTypeId) -> Option<CurtainTypeId> {
        let t = src.curtain_type(id)?;
        if let Some(x) = self.curtain_types().find(|x| x.guid == t.guid) {
            return Some(x.id);
        }
        // Materials by name.
        let names = |m: &Model, d: &CurtainDesign| -> Vec<Option<String>> {
            let mut d = d.clone();
            design_materials(&mut d)
                .into_iter()
                .map(|id| m.material(*id).map(|x| x.name.clone()))
                .collect()
        };
        let mine = names(src, &t.design);
        self.curtain_types()
            .find(|x| {
                x.name == t.name && names(self, &x.design) == mine && {
                    let (mut a, mut b) = (x.design.clone(), t.design.clone());
                    for m in design_materials(&mut a) {
                        *m = MaterialId(0);
                    }
                    for m in design_materials(&mut b) {
                        *m = MaterialId(0);
                    }
                    a == b
                }
            })
            .map(|x| x.id)
    }

    // ---- Flattening ---------------------------------------------------------

    /// This project with the elements of every copy of its modules in it
    /// (and without the modules): what views, schedules and exports show.
    /// Copies of a module whose types are missing leave those elements out.
    pub fn flattened(&self) -> Result<Model, RestoreError> {
        if self.modules.is_empty() {
            return Ok(self.clone());
        }
        let mut parts = self.to_parts();
        parts.module_sources.clear();
        parts.modules.clear();
        let mut flat_sources: BTreeMap<ModuleSourceId, (Model, TypeMaps)> = BTreeMap::new();
        for (id, s) in &self.module_sources {
            let flat = s.model.flattened()?;
            let maps = self.type_maps(&flat);
            flat_sources.insert(*id, (flat, maps));
        }
        // Copies are numbered by module, in the order they were placed.
        let mut count: BTreeMap<ModuleSourceId, usize> = BTreeMap::new();
        for (id, base, _) in self.copy_ranges() {
            let p = &self.modules[&id];
            let Some((src, maps)) = flat_sources.get(&p.source) else {
                continue;
            };
            let n = count.entry(p.source).or_default();
            *n += 1;
            self.instantiate(&mut parts, src, maps, p, base, *n);
        }
        Model::from_parts(parts)
    }

    /// The ids each copy's elements take in [`Model::flattened`]: the
    /// module's ids raised by `base`, below `base + span`.
    pub fn copy_ranges(&self) -> Vec<(ModuleId, u64, u64)> {
        let spans: BTreeMap<ModuleSourceId, u64> = self
            .module_sources
            .iter()
            .map(|(id, s)| (*id, flat_next_id(&s.model) + 1))
            .collect();
        let mut base = self.first_unused_id();
        let mut out = Vec::new();
        for p in self.modules.values() {
            let Some(&span) = spans.get(&p.source) else {
                continue;
            };
            out.push((p.id, base, span));
            base += span;
        }
        out
    }

    /// The copy an element of [`Model::flattened`] comes from.
    pub fn module_of_flat_id(&self, id: u64) -> Option<ModuleId> {
        self.copy_ranges()
            .into_iter()
            .find(|(_, base, span)| id >= *base && id < base + span)
            .map(|(m, _, _)| m)
    }

    /// Adds to `parts` the elements of `src` as the copy `p` places them,
    /// their ids raised by `base`.
    fn instantiate(
        &self,
        parts: &mut ModelParts,
        src: &Model,
        maps: &TypeMaps,
        p: &ModulePlacement,
        base: u64,
        copy: usize,
    ) {
        let t = ModuleTransform::of(p);
        let mirror = p.mirror;
        // Levels: the module's, lowest first, on this level and the ones
        // above it.
        let mut ours: Vec<&Level> = self.levels.values().collect();
        ours.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        let mut theirs: Vec<&Level> = src.levels.values().collect();
        theirs.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        let start = ours.iter().position(|l| l.id == p.level).unwrap_or(0);
        let levels: BTreeMap<LevelId, LevelId> = theirs
            .iter()
            .enumerate()
            .filter_map(|(i, l)| ours.get(start + i).map(|o| (l.id, o.id)))
            .collect();
        let level = |l: LevelId| levels.get(&l).copied();
        let top = |w: WallTop| match w {
            WallTop::Unconnected => Some(WallTop::Unconnected),
            WallTop::Level { level: l, offset } => Some(level(l).map_or(
                WallTop::Unconnected,
                |level| WallTop::Level { level, offset },
            )),
        };
        let maps = &maps;
        let wall_type = |id: WallTypeId| maps.walls.get(&id).copied();
        let slab_type = |id: SlabTypeId| maps.slabs.get(&id).copied();
        let opening_type = |id: OpeningTypeId| maps.openings.get(&id).copied();
        let member_type = |id: MemberTypeId| maps.members.get(&id).copied();
        let curtain_type = |id: CurtainTypeId| maps.curtains.get(&id).copied();
        let object_type = |id: ObjectTypeId| maps.objects.get(&id).copied();
        let mat = |id: MaterialId| {
            let name = &src.material(id)?.name;
            self.materials().find(|m| m.name == *name).map(|m| m.id)
        };
        let guid = |g: &GlobalId| combined_guid(&p.guid, g);
        let pts = |v: &[Point2]| -> Vec<Point2> {
            let mut out: Vec<Point2> = v.iter().map(|q| t.apply(*q)).collect();
            if mirror {
                out.reverse();
            }
            out
        };
        let path = |v: &[Point2]| -> Vec<Point2> { v.iter().map(|q| t.apply(*q)).collect() };
        let mut placed: Vec<u64> = Vec::new();

        let mut walls_in = Vec::new();
        for w in src.walls() {
            let (Some(level), Some(wall_type), Some(top)) =
                (level(w.level), wall_type(w.wall_type), top(w.top))
            else {
                continue;
            };
            let (start, end) = if mirror {
                (t.apply(w.end), t.apply(w.start))
            } else {
                (t.apply(w.start), t.apply(w.end))
            };
            walls_in.push((w.id, w.length()));
            placed.push(w.id.0);
            parts.walls.push(Wall {
                id: WallId(w.id.0 + base),
                guid: guid(&w.guid),
                wall_type,
                level,
                start,
                end,
                top,
                ..w.clone()
            });
        }
        for o in src.openings() {
            let Some(&(_, length)) = walls_in.iter().find(|(w, _)| *w == o.wall) else {
                continue;
            };
            let Some(opening_type) = opening_type(o.opening_type) else {
                continue;
            };
            let mut values = o.values.clone();
            for v in values.values_mut() {
                if let archisim_family::Value::Material(m) = v {
                    if let Some(x) = mat(MaterialId(*m)) {
                        *m = x.0;
                    }
                }
            }
            placed.push(o.id.0);
            parts.openings.push(Opening {
                id: OpeningId(o.id.0 + base),
                guid: guid(&o.guid),
                opening_type,
                wall: WallId(o.wall.0 + base),
                along: if mirror { length - o.along } else { o.along },
                hand: match (mirror, o.hand) {
                    (true, Hand::Left) => Hand::Right,
                    (true, Hand::Right) => Hand::Left,
                    (false, h) => h,
                },
                values,
                ..o.clone()
            });
        }
        for s in src.slabs() {
            let (Some(level), Some(slab_type)) = (level(s.level), slab_type(s.slab_type)) else {
                continue;
            };
            placed.push(s.id.0);
            parts.slabs.push(Slab {
                id: SlabId(s.id.0 + base),
                guid: guid(&s.guid),
                slab_type,
                level,
                outline: pts(&s.outline),
                holes: s.holes.iter().map(|h| pts(h)).collect(),
                slope: s.slope.map(|sl| SlabSlope {
                    from: t.apply(sl.from),
                    to: t.apply(sl.to),
                    slope: sl.slope,
                }),
                ..s.clone()
            });
        }
        for c in src.columns() {
            let (Some(level), Some(member_type), Some(top)) =
                (level(c.level), member_type(c.member_type), top(c.top))
            else {
                continue;
            };
            placed.push(c.id.0);
            parts.columns.push(Column {
                id: ColumnId(c.id.0 + base),
                guid: guid(&c.guid),
                member_type,
                level,
                position: t.apply(c.position),
                rotation: t.angle(c.rotation),
                top,
                ..c.clone()
            });
        }
        for b in src.beams() {
            let (Some(level), Some(member_type)) = (level(b.level), member_type(b.member_type))
            else {
                continue;
            };
            placed.push(b.id.0);
            parts.beams.push(Beam {
                id: BeamId(b.id.0 + base),
                guid: guid(&b.guid),
                member_type,
                level,
                start: t.apply(b.start),
                end: t.apply(b.end),
                ..b.clone()
            });
        }
        let mut roofs_in = Vec::new();
        for r in src.roofs() {
            let (Some(level), Some(roof_type)) = (level(r.level), slab_type(r.roof_type)) else {
                continue;
            };
            let n = r.outline.len();
            let slopes = if mirror {
                (0..n).map(|j| r.slopes[(2 * n - 2 - j) % n]).collect()
            } else {
                r.slopes.clone()
            };
            roofs_in.push(r.id);
            placed.push(r.id.0);
            // Its dormers, each its own in every copy, on the same plane
            // (renumbered and running the other way when mirrored).
            let dormers = r
                .dormers
                .iter()
                .filter_map(|d| {
                    Some(Dormer {
                        guid: guid(&d.guid),
                        side: if mirror {
                            (2 * n - 2 - d.side) % n
                        } else {
                            d.side
                        },
                        along: if mirror { -d.along } else { d.along },
                        wall_material: mat(d.wall_material)?,
                        frame_material: mat(d.frame_material)?,
                        glass_material: mat(d.glass_material)?,
                        ..d.clone()
                    })
                })
                .collect();
            let covering = RoofCovering {
                tiles: r.covering.tiles,
                tile_material: r.covering.tile_material.and_then(&mat),
                gutter: r.covering.gutter.as_ref().and_then(|g| {
                    Some(Gutter {
                        material: mat(g.material)?,
                        ..g.clone()
                    })
                }),
            };
            parts.roofs.push(Roof {
                id: RoofId(r.id.0 + base),
                guid: guid(&r.guid),
                roof_type,
                level,
                outline: pts(&r.outline),
                slopes,
                dormers,
                covering,
                ..r.clone()
            });
        }
        for w in src.walls() {
            if let Some(r) = src.wall_roof(w.id) {
                if walls_in.iter().any(|(x, _)| *x == w.id) && roofs_in.contains(&r) {
                    parts
                        .wall_roofs
                        .push((WallId(w.id.0 + base), RoofId(r.0 + base)));
                }
            }
        }
        for s in src.stairs() {
            let (Some(level), Some(top), Some(material)) =
                (level(s.level), top(s.top), mat(s.material))
            else {
                continue;
            };
            placed.push(s.id.0);
            parts.stairs.push(Stair {
                id: StairId(s.id.0 + base),
                guid: guid(&s.guid),
                level,
                path: path(&s.path),
                top,
                material,
                railings: match (mirror, s.railings) {
                    (true, Railings::Left) => Railings::Right,
                    (true, Railings::Right) => Railings::Left,
                    (_, r) => r,
                },
                ..s.clone()
            });
        }
        for c in src.curtain_walls() {
            let (Some(level), Some(curtain_type), Some(top)) =
                (level(c.level), curtain_type(c.curtain_type), top(c.top))
            else {
                continue;
            };
            let (start, end) = if mirror {
                (t.apply(c.end), t.apply(c.start))
            } else {
                (t.apply(c.start), t.apply(c.end))
            };
            placed.push(c.id.0);
            parts.curtain_walls.push(CurtainWall {
                id: CurtainWallId(c.id.0 + base),
                guid: guid(&c.guid),
                curtain_type,
                level,
                start,
                end,
                top,
                ..c.clone()
            });
        }
        for r in src.railings() {
            let (Some(level), Some(material)) = (level(r.level), mat(r.material)) else {
                continue;
            };
            placed.push(r.id.0);
            parts.railings.push(Railing {
                id: RailingId(r.id.0 + base),
                guid: guid(&r.guid),
                level,
                path: path(&r.path),
                material,
                glass: r.glass.and_then(mat),
                ..r.clone()
            });
        }
        for o in src.objects() {
            let (Some(level), Some(object_type)) = (level(o.level), object_type(o.object_type))
            else {
                continue;
            };
            placed.push(o.id.0);
            parts.objects.push(Object {
                id: ObjectId(o.id.0 + base),
                guid: guid(&o.guid),
                level,
                object_type,
                position: t.apply(o.position),
                rotation: t.angle(o.rotation),
                ..o.clone()
            });
        }
        for f in src.footings() {
            let (Some(level), Some(material)) = (level(f.level), mat(f.material)) else {
                continue;
            };
            let kind = match f.kind {
                FootingKind::Pad {
                    position,
                    length,
                    rotation,
                    column,
                } => FootingKind::Pad {
                    position: t.apply(position),
                    length,
                    rotation: t.angle(rotation),
                    column: column
                        .filter(|c| placed.contains(&c.0))
                        .map(|c| ColumnId(c.0 + base)),
                },
                FootingKind::Strip { start, end, wall } => FootingKind::Strip {
                    start: t.apply(start),
                    end: t.apply(end),
                    wall: wall
                        .filter(|w| placed.contains(&w.0))
                        .map(|w| WallId(w.0 + base)),
                },
                FootingKind::TieBeam { start, end } => FootingKind::TieBeam {
                    start: t.apply(start),
                    end: t.apply(end),
                },
            };
            placed.push(f.id.0);
            parts.footings.push(Footing {
                id: FootingId(f.id.0 + base),
                guid: guid(&f.guid),
                level,
                kind,
                material,
                ..f.clone()
            });
        }
        for r in src.rooms() {
            let Some(level) = level(r.level) else {
                continue;
            };
            placed.push(r.id.0);
            parts.rooms.push(crate::rooms::Room {
                id: RoomId(r.id.0 + base),
                guid: guid(&r.guid),
                level,
                outline: pts(&r.outline),
                holes: r.holes.iter().map(|h| pts(h)).collect(),
                label: t.apply(r.label),
                bounded: false,
                // Room numbers are unique on a level: each copy's go
                // after its number («2-0.01»).
                number: if r.number.is_empty() {
                    String::new()
                } else {
                    format!("{copy}-{}", r.number)
                },
                ..r.clone()
            });
        }
        // Phases and disciplines given by hand go with their elements.
        for (id, phase) in src.phase_table() {
            if placed.contains(&id) {
                parts.phases.push((id + base, phase));
            }
        }
        for (id, d) in src.discipline_table() {
            if placed.contains(&id) {
                parts.disciplines.push((id + base, d));
            }
        }
    }

    pub(super) fn restore_modules(
        &mut self,
        sources: Vec<ModuleSource>,
        modules: Vec<ModulePlacement>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for s in sources {
            claim(s.id.0, None).map_err(|error| RestoreError {
                element: format!("module {} «{}»", s.id.0, s.name),
                error,
            })?;
            self.module_sources.insert(s.id, s);
        }
        for p in modules {
            let spec = NewModulePlacement {
                source: p.source,
                level: p.level,
                position: p.position,
                rotation: p.rotation,
                mirror: p.mirror,
            };
            claim(p.id.0, Some(&p.guid))
                .and_then(|_| self.validate_module(&spec))
                .map_err(|error| RestoreError {
                    element: format!("module copy {}", p.id.0),
                    error,
                })?;
            self.modules.insert(p.id, p);
        }
        Ok(())
    }
}

/// The project's type for each type of a module.
struct TypeMaps {
    walls: BTreeMap<WallTypeId, WallTypeId>,
    slabs: BTreeMap<SlabTypeId, SlabTypeId>,
    openings: BTreeMap<OpeningTypeId, OpeningTypeId>,
    objects: BTreeMap<ObjectTypeId, ObjectTypeId>,
    members: BTreeMap<MemberTypeId, MemberTypeId>,
    curtains: BTreeMap<CurtainTypeId, CurtainTypeId>,
}

/// The `next_id` of a model once flattened.
fn flat_next_id(m: &Model) -> u64 {
    if m.modules.is_empty() {
        m.first_unused_id()
    } else {
        m.copy_ranges()
            .last()
            .map_or(m.first_unused_id(), |(_, base, span)| base + span)
    }
}

impl Model {
    /// The box around the walls, slabs, roofs, columns and rooms: what a
    /// copy of this model as a module covers.
    pub fn plan_box(&self) -> Option<(Point2, Point2)> {
        footprint(self)
    }
}

/// The box around a model's walls, slabs, roofs and columns.
fn footprint(m: &Model) -> Option<(Point2, Point2)> {
    let pts: Vec<Point2> = m
        .walls()
        .flat_map(|w| [w.start, w.end])
        .chain(m.slabs().flat_map(|s| s.outline.iter().copied()))
        .chain(m.roofs().flat_map(|r| r.outline.iter().copied()))
        .chain(m.columns().map(|c| c.position))
        .chain(m.rooms().flat_map(|r| r.outline.iter().copied()))
        .collect();
    let first = *pts.first()?;
    Some(pts.iter().fold((first, first), |(lo, hi), p| {
        (
            Point2::new(lo.x.min(p.x), lo.y.min(p.y)),
            Point2::new(hi.x.max(p.x), hi.y.max(p.y)),
        )
    }))
}

/// `name`, or «name (2)», «name (3)»… if taken.
fn free_name<'a>(taken: impl Iterator<Item = &'a str>, name: &str) -> String {
    let taken: Vec<&str> = taken.collect();
    let mut out = name.to_string();
    let mut n = 2;
    while taken.contains(&out.as_str()) {
        out = format!("{name} ({n})");
        n += 1;
    }
    out
}

/// Every material of a curtain wall design.
fn design_materials(d: &mut CurtainDesign) -> Vec<&mut MaterialId> {
    let mut v = vec![
        &mut d.border.material,
        &mut d.mullion.material,
        &mut d.transom.material,
    ];
    v.extend(d.panels.iter_mut().map(|p| &mut p.material));
    if let Some(c) = &mut d.caps {
        v.push(&mut c.material);
    }
    if let Some(l) = &mut d.louvres {
        v.push(&mut l.material);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A house type: one level, a wall of 6 m from the origin with a door
    /// 1 m from its start, and a slab.
    fn house() -> Model {
        let mut m = Model::new("Tipo A");
        let l = m.add_level("Planta baja", 0.0);
        let mat = m.add_material("Ladrillo", "brick", [200, 100, 80], CutPattern::Diagonal);
        let wt = m
            .add_wall_type(
                "Muro 24",
                WallFunction::Exterior,
                vec![Layer {
                    material: mat,
                    thickness: 0.24,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let st = m
            .add_slab_type(
                "Solera",
                SlabFunction::BaseSlab,
                vec![Layer {
                    material: mat,
                    thickness: 0.2,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        m.add_wall(NewWall {
            wall_type: wt,
            level: l,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(6.0, 0.0),
            reference: ReferenceLine::Center,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        m.add_slab(NewSlab {
            slab_type: st,
            level: l,
            outline: vec![
                Point2::new(0.0, 0.0),
                Point2::new(6.0, 0.0),
                Point2::new(6.0, 4.0),
                Point2::new(0.0, 4.0),
            ],
            top_offset: 0.0,
        })
        .unwrap();
        m
    }

    #[test]
    fn copies_are_placed_turned_and_mirrored() {
        let src = house();
        let mut m = Model::new("Masterplan");
        let l = m.add_level("Rasante", 0.0);
        let s = m
            .add_module_source("Tipo A", "tipo-a.archisim", Arc::from("{}"), "0", src)
            .unwrap();
        // Its types came along, with their GlobalIds.
        assert_eq!(m.wall_types().count(), 1);
        let a = m
            .add_module(NewModulePlacement {
                source: s,
                level: l,
                position: Point2::new(10.0, 0.0),
                rotation: 0.0,
                mirror: false,
            })
            .unwrap();
        m.add_module(NewModulePlacement {
            source: s,
            level: l,
            position: Point2::new(10.0, 0.0),
            rotation: 0.0,
            mirror: true,
        })
        .unwrap();
        assert_eq!(m.module_copies(s), 2);
        let flat = m.flattened().unwrap();
        assert_eq!(flat.walls().count(), 2);
        assert_eq!(flat.slabs().count(), 2);
        assert!(!flat.has_modules());
        // Each element knows the copy it comes from.
        for w in flat.walls() {
            assert!(m.module_of_flat_id(w.id.0).is_some());
        }
        assert_eq!(m.module_of_flat_id(1), None);
        let mut ends: Vec<(f64, f64)> = flat.walls().map(|w| (w.start.x, w.end.x)).collect();
        ends.sort_by(|a, b| a.0.total_cmp(&b.0));
        // The mirrored one runs from 10 back to 4, its ends swapped so the
        // exterior stays outside: from 4 to 10.
        assert_eq!(ends[0], (4.0, 10.0));
        assert_eq!(ends[1], (10.0, 16.0));
        // The same GlobalIds every time, and different for each copy.
        let again = m.flattened().unwrap();
        let g1: Vec<_> = flat.walls().map(|w| w.guid.clone()).collect();
        let g2: Vec<_> = again.walls().map(|w| w.guid.clone()).collect();
        assert_eq!(g1, g2);
        assert_ne!(g1[0], g1[1]);
        // Turned a quarter: the wall runs up.
        m.edit_module(a, |p| p.rotation = std::f64::consts::FRAC_PI_2)
            .unwrap();
        let outline = m.module_outline(a);
        assert_eq!(outline.len(), 4);
        let flat = m.flattened().unwrap();
        assert!(flat
            .walls()
            .any(|w| (w.start.x - 10.0).abs() < 1e-9 && (w.end.y - 6.0).abs() < 1e-9));
        // A module file kept in the project goes with it.
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.modules().count(), 2);
        assert_eq!(back, m);
    }

    #[test]
    fn mirrored_roof_keeps_its_slopes_on_their_sides() {
        // Side 0 (south) slopes, the others are gables.
        let slopes = [Some(0.5), None, None, None];
        let n = slopes.len();
        let mirrored: Vec<_> = (0..n).map(|j| slopes[(2 * n - 2 - j) % n]).collect();
        // Corners 0..3 reversed: the new side 2 runs from old corner 1 to
        // old corner 0, the old side 0.
        assert_eq!(mirrored, [None, None, Some(0.5), None]);
    }
}
