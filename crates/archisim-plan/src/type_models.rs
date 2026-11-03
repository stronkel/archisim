//! Small models to draw a type alone: a short piece of wall of a wall
//! type, a slab type's layers, a door or window in a piece of wall, an
//! object, a column or beam, a piece of curtain wall — with the materials
//! of the project they come from. The types dialog draws them in plan,
//! elevation, section and 3D; the carpentry sheets draw their doors and
//! windows from them.

use archisim_core::{
    Layer, LayerFunction, LevelId, MaterialId, Model, NewOpening, NewWall, OpeningType, Point2,
    ReferenceLine, WallFunction, WallSide,
};

/// A model with one level and one straight wall from (0, 0) to
/// (`length`, 0), the exterior on top, built from `layers` with the
/// materials of `source`. `None` if the layers are not valid.
pub fn layers_model(source: &Model, layers: &[Layer], length: f64) -> Option<(Model, LevelId)> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mapped = copy_layers(source, &mut m, layers)?;
    let t = m
        .add_wall_type("preview", WallFunction::Exterior, mapped)
        .ok()?;
    m.add_wall(NewWall {
        wall_type: t,
        level,
        start: Point2::new(0.0, 0.0),
        end: Point2::new(length, 0.0),
        reference: ReferenceLine::Center,
        height: 3.0,
        base_offset: 0.0,
        bulge: 0.0,
        top: archisim_core::WallTop::Unconnected,
    })
    .ok()?;
    Some((m, level))
}

/// The layers with the materials of `source` copied into `m` (each once).
fn copy_layers(source: &Model, m: &mut Model, layers: &[Layer]) -> Option<Vec<Layer>> {
    let mut mapped = Vec::new();
    let mut ids: Vec<(MaterialId, MaterialId)> = Vec::new();
    for l in layers {
        let id = match ids.iter().find(|(from, _)| *from == l.material) {
            Some((_, to)) => *to,
            None => {
                let mat = source.material(l.material)?;
                let to = m.add_material(
                    mat.name.clone(),
                    mat.category.clone(),
                    mat.cut_color,
                    mat.cut_pattern,
                );
                ids.push((l.material, to));
                to
            }
        };
        mapped.push(Layer {
            material: id,
            ..l.clone()
        });
    }
    Some(mapped)
}

/// A piece of wall of the layers, 1.2 m long and high, for the 3D preview.
pub fn wall_piece_model(source: &Model, layers: &[Layer]) -> Option<Model> {
    let (mut m, _) = layers_model(source, layers, 1.2)?;
    let wall = m.walls().next()?.id;
    m.set_wall_height(wall, 1.2, 0.0).ok()?;
    Some(m)
}

/// A square of slab of the layers, 1.2 m wide, for the 3D preview.
pub fn slab_piece_model(source: &Model, layers: &[Layer]) -> Option<Model> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mapped = copy_layers(source, &mut m, layers)?;
    let t = m
        .add_slab_type("preview", archisim_core::SlabFunction::Floor, mapped)
        .ok()?;
    let s = 1.2;
    m.add_slab(archisim_core::NewSlab {
        slab_type: t,
        level,
        outline: vec![
            Point2::new(0.0, 0.0),
            Point2::new(s, 0.0),
            Point2::new(s, s),
            Point2::new(0.0, s),
        ],
        top_offset: 0.0,
    })
    .ok()?;
    Some(m)
}

/// A model with a door or window of type `t` in the middle of a wall that
/// can hold its frame: the project's first wall type of the right kind
/// (façade for windows, partition for doors) if it is deep enough, or a
/// plain wall otherwise.
pub fn opening_model(source: &Model, t: &OpeningType) -> Option<(Model, LevelId)> {
    let wanted = if t.is_door() {
        WallFunction::Interior
    } else {
        WallFunction::Exterior
    };
    let host = source
        .wall_types()
        .filter(|w| w.thickness() >= t.frame_depth + 0.01)
        .min_by_key(|w| w.function != wanted);
    let layers = match host {
        Some(w) => w.layers.clone(),
        None => vec![Layer {
            material: t.frame_material,
            thickness: t.frame_depth + 0.1,
            function: LayerFunction::Structure,
        }],
    };
    let length = t.width + 0.6;
    let (mut m, level) = layers_model(source, &layers, length)?;
    // The same family and values, with the materials copied over.
    let mut copied: std::collections::HashMap<u64, MaterialId> = Default::default();
    let mut values = t.values.clone();
    for v in values.values_mut() {
        if let archisim_core::Value::Material(id) = v {
            let new = match copied.get(id) {
                Some(n) => *n,
                None => {
                    let mat = source.material(MaterialId(*id))?.clone();
                    let n = m.add_material(mat.name, mat.category, mat.cut_color, mat.cut_pattern);
                    // Its colour and texture in 3D too.
                    let _ = m.set_appearance(n, source.appearance(MaterialId(*id)));
                    copied.insert(*id, n);
                    n
                }
            };
            *v = archisim_core::Value::Material(new.0);
        }
    }
    let family = m.add_family(&source.family(t.family)?.source).ok()?;
    let ot = m
        .add_family_type(archisim_core::NewFamilyType {
            family,
            name: t.name.clone(),
            mark: Some(t.mark.clone()),
            description: String::new(),
            values,
        })
        .ok()?;
    let wall = m.walls().next()?.id;
    // Cut by the plan's cut plane (1.10 m): doors from the floor, windows
    // at a usual sill unless they are too low to reach it.
    let sill = if t.is_door() {
        0.0
    } else if t.height > 0.25 {
        0.9
    } else {
        1.1 - t.height / 2.0
    };
    let height = (sill + t.height + 0.2).max(3.0);
    m.set_wall_height(wall, height, 0.0).ok()?;
    m.add_opening(NewOpening {
        opening_type: ot,
        wall,
        along: length / 2.0,
        sill,
        opens_to: WallSide::Interior,
        hand: archisim_core::Hand::Left,
        setback: None,
    })
    .ok()?;
    Some((m, level))
}

/// [`opening_model`] for a picture in 3D: the wall only a little higher
/// than the door or window, so that it fills the picture.
pub fn opening_model_3d(source: &Model, t: &OpeningType) -> Option<(Model, LevelId)> {
    let (mut m, level) = opening_model(source, t)?;
    let wall = m.walls().next()?.id;
    let sill = m.openings().next()?.sill;
    m.set_wall_height(wall, sill + t.height + 0.12, 0.0).ok()?;
    Some((m, level))
}

/// A model with one object of type `t` at the origin, unturned: its plan
/// symbol as the plan draws it.
pub fn object_model(source: &Model, t: &archisim_core::ObjectType) -> Option<(Model, LevelId)> {
    object_model_with(source, t, None)
}

/// [`object_model`] with the object's own values `own` (how a tree is
/// drawn in plan…): with them, its drawing is not made the presentation
/// one, so the choices of plan drawing show as they are (E2).
pub fn object_model_with(
    source: &Model,
    t: &archisim_core::ObjectType,
    own: Option<archisim_core::Values>,
) -> Option<(Model, LevelId)> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mut copied: std::collections::HashMap<u64, MaterialId> = Default::default();
    let mut values = t.values.clone();
    for v in values.values_mut() {
        if let archisim_core::Value::Material(id) = v {
            let new = match copied.get(id) {
                Some(n) => *n,
                None => {
                    let mat = source.material(MaterialId(*id))?.clone();
                    let n = m.add_material(mat.name, mat.category, mat.cut_color, mat.cut_pattern);
                    copied.insert(*id, n);
                    n
                }
            };
            *v = archisim_core::Value::Material(new.0);
        }
    }
    let family = m.add_family(&source.family(t.family)?.source).ok()?;
    let ot = m.add_object_type(t.name.clone(), family, values).ok()?;
    // Its own plan drawing (a DXF symbol, a picture), as the plan draws it.
    if t.symbol.is_some() {
        m.set_object_type_symbol(ot, t.symbol.clone()).ok()?;
    }
    if t.plan_image.is_some() {
        m.set_object_type_plan_image(ot, t.plan_image.clone())
            .ok()?;
    }
    let id = m
        .add_object(archisim_core::NewObject {
            level,
            object_type: ot,
            position: Point2::new(0.0, 0.0),
            rotation: 0.0,
            base_offset: 0.0,
        })
        .ok()?;
    // Trees and the like show their drawing of presentation (the form of
    // the canopy is what tells one from another), when they have one.
    let presented = m
        .object_presentation_symbol(id, 20.0)
        .is_some_and(|r| r.is_ok());
    let has_choice = m.family(family).is_some_and(|f| {
        f.def()
            .params_of(archisim_core::family::ParamScope::Instance)
            .any(|p| p.name == "drawing")
    });
    if let Some(own) = own {
        if !own.is_empty() {
            let _ = m.set_object_values(id, own);
        }
    } else if presented && has_choice {
        let mut v = archisim_core::Values::new();
        v.insert(
            "drawing".into(),
            archisim_core::Value::Text("presentación".into()),
        );
        let _ = m.set_object_values(id, v);
    }
    Some((m, level))
}

/// A model with one column of the section and material of a column or
/// beam type, cut in plan: the section as the plan draws it.
pub fn member_model(source: &Model, t: &archisim_core::MemberType) -> Option<(Model, LevelId)> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mat = source.material(t.material)?;
    let mid = m.add_material(
        mat.name.clone(),
        mat.category.clone(),
        mat.cut_color,
        mat.cut_pattern,
    );
    let ct = m
        .add_member_type(
            "preview",
            archisim_core::MemberUsage::Column,
            mid,
            t.profile,
        )
        .ok()?;
    m.add_column(archisim_core::NewColumn {
        member_type: ct,
        level,
        position: Point2::new(0.0, 0.0),
        rotation: 0.0,
        base_offset: 0.0,
        top: archisim_core::WallTop::Unconnected,
        height: 3.0,
    })
    .ok()?;
    Some((m, level))
}

/// A model with one curtain wall of design `design` (materials copied
/// from `source`), from (0, 0) to (`length`, 0), `height` high: what the
/// curtain wall types window draws and shows in 3D.
pub fn curtain_model(
    source: &Model,
    design: &archisim_core::CurtainDesign,
    length: f64,
    height: f64,
) -> Option<Model> {
    let mut m = Model::new("preview");
    let level = m.add_level("preview", 0.0);
    let mut copied: std::collections::HashMap<MaterialId, MaterialId> = Default::default();
    let mut copy = |m: &mut Model, id: MaterialId| -> Option<MaterialId> {
        if let Some(n) = copied.get(&id) {
            return Some(*n);
        }
        let mat = source.material(id)?;
        let n = m.add_material(
            mat.name.clone(),
            mat.category.clone(),
            mat.cut_color,
            mat.cut_pattern,
        );
        copied.insert(id, n);
        Some(n)
    };
    let mut d = design.clone();
    for p in [&mut d.border, &mut d.mullion, &mut d.transom] {
        p.material = copy(&mut m, p.material)?;
    }
    for p in &mut d.panels {
        p.material = copy(&mut m, p.material)?;
    }
    if let Some(c) = &mut d.caps {
        c.material = copy(&mut m, c.material)?;
    }
    if let Some(l) = &mut d.louvres {
        l.material = copy(&mut m, l.material)?;
    }
    let t = m.add_curtain_type("preview", d).ok()?;
    m.add_curtain_wall(archisim_core::NewCurtainWall {
        curtain_type: t,
        level,
        start: Point2::new(0.0, 0.0),
        end: Point2::new(length, 0.0),
        base_offset: 0.0,
        top: archisim_core::WallTop::Unconnected,
        height,
    })
    .ok()?;
    Some(m)
}
