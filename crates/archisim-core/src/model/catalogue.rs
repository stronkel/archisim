//! The system's catalogue (M8, `docs/MODULAR.md`): a box type is a project
//! file with one box and what it holds, placed in houses as a module. Its
//! [`CatalogueEntry`] says what it is (code, name), where it stands (draft
//! or approved, and its revision) and what each house may choose in it
//! ([`CatalogueOption`]: a door or window that may be swapped for others
//! of the file). An approved box type is frozen: changing it asks for a
//! new revision first (the app refuses any other change).
//!
//! [`Model::box_type_model`] makes a box type from a box of a project: the
//! box, what it holds and the rooms in it, moved so the box starts at the
//! origin, along `x`, on a level at ±0,00.

use super::*;
use std::collections::BTreeSet;

/// Where a box type stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogueStatus {
    /// Being designed: it can be changed.
    #[default]
    Draft,
    /// Approved for the factory: frozen at its revision.
    Approved,
}

impl CatalogueStatus {
    pub fn code(self) -> &'static str {
        match self {
            CatalogueStatus::Draft => "draft",
            CatalogueStatus::Approved => "approved",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        [CatalogueStatus::Draft, CatalogueStatus::Approved]
            .into_iter()
            .find(|s| s.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            CatalogueStatus::Draft => "En diseño",
            CatalogueStatus::Approved => "Aprobada",
        }
    }
}

/// Something each house may choose in a box type: a door or window type
/// (`slot`, by name) that may be swapped for any of `choices` (names of
/// other types of the same file).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CatalogueOption {
    /// What it is called where it is chosen («Ventana del dormitorio»).
    pub name: String,
    /// The door or window type of the file that may be swapped.
    pub slot: String,
    /// What it may be swapped for.
    pub choices: Vec<String>,
}

/// What a project that is a box type of the catalogue says of itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CatalogueEntry {
    /// Its code in the catalogue («CB-620-DD»).
    pub code: String,
    /// Its name («Caja 6,20 · dormitorio doble»).
    pub name: String,
    /// The last approved revision (0: never approved).
    pub revision: u32,
    pub status: CatalogueStatus,
    /// When the revision was approved (as written: «09/11/2026»).
    pub approved_on: String,
    /// What changed, what to watch.
    pub notes: String,
    pub options: Vec<CatalogueOption>,
}

impl CatalogueEntry {
    pub fn new(code: impl Into<String>, name: impl Into<String>) -> Self {
        CatalogueEntry {
            code: code.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn approved(&self) -> bool {
        self.status == CatalogueStatus::Approved
    }

    /// «CB-620-DD · rev. 2» or «CB-620-DD · en diseño (rev. 3)».
    pub fn label(&self) -> String {
        match self.status {
            CatalogueStatus::Approved => format!("{} · rev. {}", self.code, self.revision),
            CatalogueStatus::Draft => {
                format!("{} · en diseño (rev. {})", self.code, self.revision + 1)
            }
        }
    }

    /// What the copies' boxes are called: «CB-620-DD rev. 2 · Caja 6,20 ·
    /// dormitorio doble».
    pub fn box_kind(&self) -> String {
        let rev = match self.status {
            CatalogueStatus::Approved => format!("rev. {}", self.revision),
            CatalogueStatus::Draft => "en diseño".to_string(),
        };
        if self.name.is_empty() {
            format!("{} {rev}", self.code)
        } else {
            format!("{} {rev} · {}", self.code, self.name)
        }
    }

    /// The option a door or window type (by name) belongs to.
    pub fn option_for(&self, type_name: &str) -> Option<&CatalogueOption> {
        self.options.iter().find(|o| o.slot == type_name)
    }
}

impl Model {
    pub fn catalogue(&self) -> Option<&CatalogueEntry> {
        self.catalogue.as_ref()
    }

    /// Whether this file is an approved box type: frozen.
    pub fn is_frozen(&self) -> bool {
        self.catalogue.as_ref().is_some_and(|c| c.approved())
    }

    pub fn set_catalogue(&mut self, entry: Option<CatalogueEntry>) -> Result<(), ModelError> {
        if let Some(e) = &entry {
            if e.code.trim().is_empty() {
                return Err(ModelError::InvalidShape("a box type needs a code"));
            }
            for o in &e.options {
                if o.name.trim().is_empty() || o.slot.trim().is_empty() {
                    return Err(ModelError::InvalidShape(
                        "an option of a box type needs a name and a door or window type",
                    ));
                }
            }
        }
        self.catalogue = entry;
        Ok(())
    }

    /// Approves the box type: the next revision, frozen.
    pub fn approve_catalogue(&mut self, on: impl Into<String>) -> Result<u32, ModelError> {
        let c = self
            .catalogue
            .as_mut()
            .ok_or(ModelError::InvalidShape("this project is not a box type"))?;
        if c.approved() {
            return Err(ModelError::InvalidShape(
                "this box type is already approved",
            ));
        }
        c.revision += 1;
        c.status = CatalogueStatus::Approved;
        c.approved_on = on.into();
        Ok(c.revision)
    }

    /// Opens a new revision of an approved box type: it can be changed
    /// again until it is approved.
    pub fn new_catalogue_revision(&mut self) -> Result<(), ModelError> {
        let c = self
            .catalogue
            .as_mut()
            .ok_or(ModelError::InvalidShape("this project is not a box type"))?;
        c.status = CatalogueStatus::Draft;
        Ok(())
    }

    /// The first box of a module file, as a copy at `t` would place it.
    pub fn module_box(&self, source: ModuleSourceId) -> Option<&ModularBox> {
        self.module_sources
            .get(&source)
            .and_then(|s| s.model.boxes.values().next())
    }

    /// Where a copy of a box type goes so that its box stands against a
    /// box of `level` (a joint away, flush with an end), if one is within
    /// `reach` of where it would go at `at`: the copy's position.
    pub fn module_box_snap(
        &self,
        source: ModuleSourceId,
        level: LevelId,
        at: Point2,
        rotation: f64,
        mirror: bool,
        reach: f64,
    ) -> Option<Point2> {
        let b = self.module_box(source)?;
        let t = ModuleTransform {
            position: at,
            rotation,
            mirror,
        };
        let (origin, angle, sides) = placed_box(b, &t);
        let spec = NewBox {
            mark: None,
            kind: String::new(),
            level,
            origin,
            angle,
            length: b.length,
            width: b.width,
            bottom: b.bottom,
            height: b.height,
            sides,
        };
        // Against this project's boxes and those of the copies already
        // placed.
        let copies: Vec<ModularBox> = self
            .modules_on_level(level)
            .filter_map(|p| {
                let pb = self.module_box(p.source)?;
                let (origin, angle, sides) = placed_box(pb, &ModuleTransform::of(p));
                Some(ModularBox {
                    origin,
                    angle,
                    sides,
                    level,
                    ..pb.clone()
                })
            })
            .collect();
        let snapped = snap_against(
            &spec,
            reach,
            self.boxes_on_level(level).chain(copies.iter()),
        )?;
        Some(at + (snapped - origin))
    }

    /// A box type made from a box of this project: the box, what it holds
    /// and the rooms whose label is inside it, with every type and
    /// material, moved so that the box starts at the origin along `x` on
    /// one level at ±0,00. Walls tied to another level keep their height.
    pub fn box_type_model(&self, id: BoxId, entry: CatalogueEntry) -> Result<Model, ModelError> {
        let b = self
            .boxes
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownElement(id.0))?;
        let held = self.box_members(id);
        let keep: BTreeSet<u64> = held.ids().into_iter().chain([id.0]).collect();
        // The file: this project's types and materials, its box's level
        // and what the box holds.
        let mut parts = self.to_parts();
        let level = self
            .levels
            .get(&b.level)
            .cloned()
            .ok_or(ModelError::UnknownLevel(b.level))?;
        parts.levels = vec![level];
        parts.walls.retain(|w| keep.contains(&w.id.0));
        for w in &mut parts.walls {
            w.top = WallTop::Unconnected;
        }
        for c in &mut parts.columns {
            c.top = WallTop::Unconnected;
        }
        parts.openings.retain(|o| keep.contains(&o.id.0));
        parts.slabs.retain(|s| keep.contains(&s.id.0));
        parts.ceilings.retain(|s| keep.contains(&s.id.0));
        parts.columns.retain(|s| keep.contains(&s.id.0));
        parts.beams.retain(|s| keep.contains(&s.id.0));
        parts.objects.retain(|s| keep.contains(&s.id.0));
        parts.boxes.retain(|s| s.id == id);
        let rooms: Vec<u64> = parts
            .rooms
            .iter()
            .filter(|r| r.level == b.level && b.contains(r.label, 0.0))
            .map(|r| r.id.0)
            .collect();
        parts.rooms.retain(|r| rooms.contains(&r.id.0));
        for r in &mut parts.rooms {
            r.bounded = false;
        }
        let kept: BTreeSet<u64> = keep.iter().copied().chain(rooms).collect();
        parts.dimensions.clear();
        parts.elevation_marks.clear();
        parts.sections.clear();
        parts.grids.clear();
        parts.footings.clear();
        parts.roofs.clear();
        parts.wall_roofs.clear();
        parts.stairs.clear();
        parts.terrains.clear();
        parts.curtain_walls.clear();
        parts.railings.clear();
        parts.module_sources.clear();
        parts.modules.clear();
        parts.masses.clear();
        parts.edges.clear();
        parts.paint_zones.clear();
        parts.face_zones.clear();
        parts.plan_views.clear();
        parts.level_styles.clear();
        parts.ref_planes.clear();
        parts.cameras.clear();
        parts.plots.clear();
        parts.roads.clear();
        parts.roundabouts.clear();
        parts.markings.clear();
        parts.decals.clear();
        parts.plantings.clear();
        parts.pools.clear();
        parts.elevators.clear();
        parts.runs.clear();
        parts.devices.clear();
        parts.drafts.clear();
        parts.sheets.clear();
        parts.draft_views.clear();
        parts.tags.clear();
        parts.viewports.clear();
        parts.tables.clear();
        parts.level_underlays.clear();
        parts.structural_loads.clear();
        parts.device_circuits.clear();
        parts.georeference = None;
        parts.orthophoto = None;
        // What is kept by element id only for the kept elements; types
        // keep theirs.
        let types: BTreeSet<u64> = self
            .wall_types
            .keys()
            .map(|k| k.0)
            .chain(self.slab_types.keys().map(|k| k.0))
            .chain(self.opening_types().map(|t| t.id.0))
            .chain(self.member_types().map(|t| t.id.0))
            .chain(self.curtain_types().map(|t| t.id.0))
            .chain(self.object_types().map(|t| t.id.0))
            .chain(self.materials().map(|t| t.id.0))
            .collect();
        let alive = |k: u64| kept.contains(&k) || types.contains(&k);
        parts.phases.retain(|(k, _)| alive(*k));
        parts.disciplines.retain(|(k, _)| alive(*k));
        parts.face_paints.retain(|(k, _, _)| alive(*k));
        parts.parameter_values.retain(|(k, _, _)| alive(*k));
        parts.reinforcements.retain(|(k, _)| alive(*k));
        parts.slab_joists.retain(|(k, _)| alive(k.0));
        parts.footing_offsets.clear();
        parts.footing_piles.clear();
        parts.catalogue = None;
        let cut = Model::from_parts(parts).map_err(|e| e.error)?;
        // Moved to the origin: placed in an empty project at the inverse
        // of the box's place, and flattened.
        let mut host = Model::new(cut.project_name.clone());
        let ground = host.add_level(
            cut.levels()
                .first()
                .map_or("Planta baja".into(), |l| l.name.clone()),
            0.0,
        );
        let text: std::sync::Arc<str> = "".into();
        let source = host.add_module_source("caja", "", text, "", cut)?;
        let (c, s) = (b.angle.cos(), b.angle.sin());
        let back = Point2::new(
            -(b.origin.x * c + b.origin.y * s),
            b.origin.x * s - b.origin.y * c,
        );
        host.add_module(NewModulePlacement {
            source,
            level: ground,
            position: back,
            rotation: -b.angle,
            mirror: false,
        })?;
        let mut out = host.flattened().map_err(|e| e.error)?;
        out.catalogue = Some(entry);
        let only: Vec<BoxId> = out.boxes.keys().copied().collect();
        for bid in only {
            out.edit_box(bid, |x| {
                x.angle = 0.0;
                x.origin = Point2::new(0.0, 0.0);
            })?;
        }
        Ok(out)
    }
}

/// Where a box of a module goes in a copy placed by `t`: its origin, its
/// angle and its sides (mirrored, the box starts at its second corner and
/// B and D change places).
pub fn placed_box(b: &ModularBox, t: &ModuleTransform) -> (Point2, f64, [SideState; 4]) {
    let c = b.corners().map(|p| t.apply(p));
    let (origin, along, sides) = if t.mirror {
        (
            c[1],
            c[0] - c[1],
            [b.sides[0], b.sides[3], b.sides[2], b.sides[1]],
        )
    } else {
        (c[0], c[1] - c[0], b.sides)
    };
    (origin, along.y.atan2(along.x), sides)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approving_freezes_and_a_revision_opens_it() {
        let mut m = Model::new("Caja");
        m.add_level("PB", 0.0);
        assert!(!m.is_frozen());
        assert!(m.approve_catalogue("hoy").is_err());
        m.set_catalogue(Some(CatalogueEntry::new("CB-620-DD", "Dormitorio doble")))
            .unwrap();
        assert_eq!(
            m.catalogue().unwrap().label(),
            "CB-620-DD · en diseño (rev. 1)"
        );
        assert_eq!(m.approve_catalogue("09/11/2026").unwrap(), 1);
        assert!(m.is_frozen());
        assert_eq!(m.catalogue().unwrap().label(), "CB-620-DD · rev. 1");
        m.new_catalogue_revision().unwrap();
        assert!(!m.is_frozen());
        assert_eq!(m.approve_catalogue("10/11/2026").unwrap(), 2);
        assert!(m
            .set_catalogue(Some(CatalogueEntry::new(" ", "x")))
            .is_err());
    }

    #[test]
    fn a_box_becomes_a_box_type_and_its_copies_stand_side_by_side() {
        let mut m = Model::new("Casa");
        let lv = m.add_level("PB", 0.0);
        let mat = m.add_material("Placa", "board", [200, 200, 200], CutPattern::Solid);
        let layer = |t| Layer {
            material: mat,
            thickness: t,
            function: LayerFunction::Structure,
        };
        let wt = m
            .add_wall_type("Fachada", WallFunction::Exterior, vec![layer(0.2)])
            .unwrap();
        let ft = m
            .add_slab_type("Suelo", SlabFunction::Floor, vec![layer(0.27)])
            .unwrap();
        // A box turned and away from the origin.
        let b = m
            .add_box(NewBox::usual(
                lv,
                Point2::new(20.0, 5.0),
                std::f64::consts::FRAC_PI_2,
            ))
            .unwrap();
        let fill = BoxFill {
            wall_type: Some(wt),
            floor_type: Some(ft),
            roof_type: None,
            ceiling_type: None,
            ceiling_height: 2.5,
        };
        m.fill_box(b, fill).unwrap();
        let mut entry = CatalogueEntry::new("CB-620", "Caja 6,20");
        entry.notes = "primera".into();
        let mut t = m.box_type_model(b, entry).unwrap();
        assert_eq!(t.boxes().count(), 1);
        let tb = t.boxes().next().unwrap().clone();
        assert!(tb.origin.approx_eq(Point2::new(0.0, 0.0)), "{tb:?}");
        assert!(tb.angle.abs() < 1e-12);
        assert_eq!(t.walls().count(), 4);
        assert_eq!(t.box_members(tb.id).walls.len(), 4);
        // Every wall inside the box at the origin.
        for w in t.walls() {
            assert!(tb.contains((w.start + w.end) * 0.5, 0.01));
        }
        t.approve_catalogue("09/11/2026").unwrap();
        // Two copies in a house: the second snaps against the first.
        let mut h = Model::new("Vivienda");
        let pb = h.add_level("PB", 0.0);
        let text: std::sync::Arc<str> = "x".into();
        let src = h.add_module_source("CB-620", "", text, "s", t).unwrap();
        h.add_module(NewModulePlacement {
            source: src,
            level: pb,
            position: Point2::new(0.0, 0.0),
            rotation: 0.0,
            mirror: false,
        })
        .unwrap();
        // Near the place across side C of the first (3,62 m up).
        let at = h
            .module_box_snap(src, pb, Point2::new(0.4, 3.9), 0.0, false, 1.5)
            .unwrap();
        assert!(
            at.approx_eq(Point2::new(0.0, BOX_WIDTH + BOX_JOINT)),
            "{at:?}"
        );
        h.add_module(NewModulePlacement {
            source: src,
            level: pb,
            position: at,
            rotation: 0.0,
            mirror: false,
        })
        .unwrap();
        let flat = h.flattened().unwrap();
        let marks: Vec<String> = flat.boxes().map(|b| b.mark.clone()).collect();
        assert_eq!(marks, vec!["MOD-01", "MOD-02"]);
        assert!(flat.boxes().all(|b| b.kind == "CB-620 rev. 1 · Caja 6,20"));
        assert_eq!(flat.walls().count(), 8);
        for b in flat.boxes() {
            assert_eq!(flat.box_members(b.id).walls.len(), 4);
        }
        // Their sides touch.
        let first = flat.boxes().next().unwrap().id;
        assert!(flat.box_neighbours(first)[2].is_some());
    }
}
