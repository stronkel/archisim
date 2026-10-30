//! Objects ("objetos"): furniture, sanitary ware and kitchen units placed
//! on a level, made by families that need no wall (`host none`). Like door
//! and window types, an object type is a family plus the values of its
//! type parameters; each object places one at a point, turned.
//!
//! Family axes for objects: origin at the middle of the back side, `x` to
//! the right, `y` forwards, `z` up from the floor (plus the object's base
//! offset). IFC: `IfcFurniture` or `IfcSanitaryTerminal`, as the recipe's
//! `ifc_class` and `ifc_type` outputs say.

use super::*;

/// Families whose objects ArchiSim places on a level: furniture, sanitary
/// ware, kitchen units, the site (vegetation, street furniture, rocks and
/// other natural things), people
/// and vehicles; and railing types (`railing`), which are not placed as
/// objects but give their look to railings.
pub const OBJECT_CATEGORIES: [&str; 9] = [
    "furniture",
    "sanitary",
    "kitchen",
    "site",
    "street",
    "nature",
    "people",
    "vehicle",
    "railing",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectType {
    pub id: ObjectTypeId,
    pub guid: GlobalId,
    pub name: String,
    pub family: FamilyId,
    /// Value of every type parameter of the family.
    pub values: Values,
    /// A 3D model shown instead of the family's boxes (scaled to the
    /// type's height).
    pub mesh: Option<MeshId>,
    /// A plan symbol brought from a DXF, drawn instead of the family's
    /// (bloque 6 de la revisión del 22/10/2026).
    pub symbol: Option<PlanSymbol>,
    /// More plan symbols of the same species (remate de A1): each object
    /// of the type takes one of them or `symbol`, always the same one, so
    /// a row of trees is not all one drawing.
    pub more_symbols: Vec<PlanSymbol>,
    /// A type of one object only: what was changed on that object in the
    /// Maqueta (its size, its colours, its drawers), leaving the type it
    /// came from as it was (as ArchiCAD keeps each object's own values).
    /// Not offered in the catalogue nor the galleries; it goes with its
    /// object.
    pub own: Option<OwnType>,
    /// Its own drawings for elevations, sections and axonometries (from
    /// a DXF or a PNG), drawn instead of the one ArchiSim makes.
    pub elevation: Vec<ElevationFigure>,
    /// A picture (PNG) for presentation plans, drawn instead of its
    /// symbol, as large as its plan box.
    pub plan_image: Option<PlanImage>,
}

/// A picture of an object type seen from above, for presentation plans.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanImage {
    pub source: String,
    pub png: std::sync::Arc<Vec<u8>>,
    /// Its width over its height.
    pub aspect: f64,
}

/// A drawing of an object type seen from the front or from the side, for
/// elevations, sections and axonometries: drawn standing at its foot, as
/// tall as the object (keeping its proportion), facing the viewer.
#[derive(Debug, Clone, PartialEq)]
pub struct ElevationFigure {
    /// Where it came from (the file's name), to show.
    pub source: String,
    /// Seen from its side (along its `x`); otherwise from its front.
    pub side: bool,
    pub art: ElevationArt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElevationArt {
    /// A DXF's lines, arcs, circles and fills, `x` across from its middle
    /// and `y` up from its lowest point, 1 high.
    Lines(Vec<SymbolItem>),
    /// A picture with transparency (PNG): its bytes and its width over its
    /// height.
    Image {
        png: std::sync::Arc<Vec<u8>>,
        aspect: f64,
    },
}

impl ElevationArt {
    /// Its width over its height.
    pub fn aspect(&self) -> f64 {
        match self {
            ElevationArt::Image { aspect, .. } => *aspect,
            ElevationArt::Lines(items) => {
                let (mut lo, mut hi) = (f64::MAX, f64::MIN);
                for i in items {
                    for p in shape_extent(&i.shape) {
                        lo = lo.min(p.x);
                        hi = hi.max(p.x);
                    }
                }
                if lo > hi {
                    1.0
                } else {
                    hi - lo
                }
            }
        }
    }
}

/// The points that bound a drawn shape (enough for its box).
pub fn shape_extent(shape: &crate::Shape) -> Vec<Point2> {
    use crate::Shape;
    let ring = |c: Point2, r: f64| vec![c + Vec2::new(r, r), c - Vec2::new(r, r)];
    match shape {
        Shape::Line { a, b } => vec![*a, *b],
        Shape::Polyline(p) => p.points.clone(),
        Shape::Circle { centre, radius } | Shape::Arc { centre, radius, .. } => {
            ring(*centre, *radius)
        }
        Shape::Ellipse { centre, axis, .. } => ring(*centre, axis.length()),
        Shape::Point { at } => vec![*at],
        Shape::Hatch(h) => h.loops.iter().flat_map(|l| l.points.clone()).collect(),
        Shape::Text(_) | Shape::Dim(_) => Vec::new(),
    }
}

/// Which of its drawings an object shows to a view, and whether flipped.
#[derive(Debug, Clone, PartialEq)]
pub struct ShownFigure {
    pub figure: ElevationFigure,
    /// Drawn the other way round (seen from its back, or its other side).
    pub flip: bool,
    /// How tall it is drawn, metres.
    pub height: f64,
}

/// Whose own type an object type is, and the type it was made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnType {
    pub object: ObjectId,
    pub base: ObjectTypeId,
}

/// A plan symbol of an object type: lines, arcs, circles and fills of a
/// DXF drawing, centred on the middle of the type's plan box.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanSymbol {
    /// Where it came from (the file's name), to show.
    pub source: String,
    /// Stretched to the type's size (its items then fit a square of side
    /// 1 round the origin, drawn as large as the type's box) or at its
    /// own size (its items in metres round the origin).
    pub fit: bool,
    pub items: Vec<SymbolItem>,
}

/// One drawn piece of a symbol, with its own colour and line weight if
/// the drawing gave them (otherwise the category's ink and a fine line).
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolItem {
    pub shape: crate::Shape,
    pub color: Option<crate::Rgb>,
    /// Paper millimetres.
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub id: ObjectId,
    pub guid: GlobalId,
    pub level: LevelId,
    pub object_type: ObjectTypeId,
    /// The family's origin in plan.
    pub position: Point2,
    /// Turn of the family's `x` axis from the plan's, counter-clockwise.
    pub rotation: f64,
    pub base_offset: f64,
    /// Its own values of the family's instance parameters (how a tree's
    /// canopy is drawn…); those left out take the family's default.
    pub values: Values,
}

/// Data needed to place an object.
#[derive(Debug, Clone, PartialEq)]
pub struct NewObject {
    pub level: LevelId,
    pub object_type: ObjectTypeId,
    pub position: Point2,
    pub rotation: f64,
    pub base_offset: f64,
}

impl Object {
    /// A point in the family's axes, in plan.
    pub fn to_plan(&self, x: f64, y: f64) -> Point2 {
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        Point2::new(
            self.position.x + x * c - y * s,
            self.position.y + x * s + y * c,
        )
    }
}

/// How an object is drawn in plan ([`Model::object_drawing`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectDrawing {
    /// Its drawing of presentation (the canopy of a tree) in every view.
    Presentation,
    /// Its technical symbol for the scale in every view.
    Technical,
    /// As the view's style says.
    ByStyle,
}

impl Model {
    fn object_family(&self, family: FamilyId) -> Result<&Family, ModelError> {
        let fam = self
            .families
            .get(&family)
            .ok_or(ModelError::UnknownFamily(family))?;
        let def = fam.def();
        if !OBJECT_CATEGORIES.contains(&def.category.as_str())
            || def.host != archisim_family::Host::None
        {
            return Err(ModelError::NotAnObject(fam.name().to_string()));
        }
        Ok(fam)
    }

    fn check_object_type(&self, family: FamilyId, values: &Values) -> Result<(), ModelError> {
        let fam = self.object_family(family)?;
        for v in values.values() {
            if let Value::Material(m) = v {
                if !self.materials.contains_key(&MaterialId(*m)) {
                    return Err(ModelError::UnknownMaterial(MaterialId(*m)));
                }
            }
        }
        let instance = self.family_defaults(family, ParamScope::Instance)?;
        fam.def()
            .evaluate(values, &instance, &Values::new())
            .map(|_| ())
            .map_err(ModelError::Family)
    }

    /// An object's instance values: its own over the family's defaults.
    pub fn object_values(&self, id: ObjectId) -> Values {
        let Some(o) = self.objects.get(&id) else {
            return Values::new();
        };
        let Some(family) = self.object_types.get(&o.object_type).map(|t| t.family) else {
            return Values::new();
        };
        let Ok(mut v) = self.family_defaults(family, ParamScope::Instance) else {
            return Values::new();
        };
        for (k, x) in &o.values {
            if v.contains_key(k) {
                v.insert(k.clone(), x.clone());
            }
        }
        v
    }

    /// How an object is drawn in plan: its own «Dibujo en planta», or its
    /// type's when it says «del tipo». Objects whose family has no such
    /// choice follow the view's style.
    pub fn object_drawing(&self, id: ObjectId) -> ObjectDrawing {
        let own = self
            .object_values(id)
            .get("drawing")
            .and_then(|v| v.as_text().map(String::from));
        let of_type = || {
            let o = self.objects.get(&id)?;
            let t = self.object_types.get(&o.object_type)?;
            let mut v = self.family_defaults(t.family, ParamScope::Type).ok()?;
            v.extend(t.values.clone());
            v.get("plan_drawing")
                .and_then(|v| v.as_text().map(String::from))
        };
        let choice = match own.as_deref() {
            None | Some("del tipo") => of_type(),
            Some(_) => own,
        };
        match choice.as_deref() {
            Some("presentación") => ObjectDrawing::Presentation,
            Some("técnico") => ObjectDrawing::Technical,
            _ => ObjectDrawing::ByStyle,
        }
    }

    /// The plan symbol of an object type (none: the family's drawing).
    pub fn set_object_type_symbol(
        &mut self,
        id: ObjectTypeId,
        symbol: Option<PlanSymbol>,
    ) -> Result<(), ModelError> {
        if let Some(s) = &symbol {
            if s.items.is_empty() {
                return Err(ModelError::InvalidShape(
                    "a plan symbol needs something drawn",
                ));
            }
        }
        let t = self
            .object_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownObjectType(id))?;
        t.symbol = symbol;
        t.more_symbols.clear();
        Ok(())
    }

    /// Several plan symbols for an object type, that its objects take in
    /// turn (the first is its `symbol`); none: the family's drawing.
    pub fn set_object_type_symbols(
        &mut self,
        id: ObjectTypeId,
        symbols: Vec<PlanSymbol>,
    ) -> Result<(), ModelError> {
        let mut it = symbols.into_iter();
        self.set_object_type_symbol(id, it.next())?;
        let more: Vec<PlanSymbol> = it.filter(|s| !s.items.is_empty()).collect();
        if let Some(t) = self.object_types.get_mut(&id) {
            t.more_symbols = more;
        }
        Ok(())
    }

    /// The plan symbol an object is drawn with: its type's, or one of its
    /// type's several, chosen by the object (always the same for it).
    pub fn object_symbol(&self, id: ObjectId) -> Option<&PlanSymbol> {
        let o = self.objects.get(&id)?;
        let t = self.object_types.get(&o.object_type)?;
        let first = t.symbol.as_ref()?;
        if t.more_symbols.is_empty() {
            return Some(first);
        }
        // A mix of the id's bits, so neighbours differ.
        let mut h = id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
        let i = (h % (t.more_symbols.len() as u64 + 1)) as usize;
        if i == 0 {
            Some(first)
        } else {
            t.more_symbols.get(i - 1)
        }
    }

    /// Sets an object's own instance values (checked against its family).
    pub fn set_object_values(&mut self, id: ObjectId, values: Values) -> Result<(), ModelError> {
        let o = self.objects.get(&id).ok_or(ModelError::UnknownObject(id))?;
        let t = self
            .object_types
            .get(&o.object_type)
            .ok_or(ModelError::UnknownObjectType(o.object_type))?;
        let def = self.object_family(t.family)?.def();
        let mut all = self.family_defaults(t.family, ParamScope::Instance)?;
        all.extend(values.clone());
        def.evaluate(&t.values, &all, &Values::new())
            .map_err(ModelError::Family)?;
        if let Some(o) = self.objects.get_mut(&id) {
            o.values = values;
        }
        Ok(())
    }

    /// A new object type of a family (furniture, sanitary or kitchen).
    pub fn add_object_type(
        &mut self,
        name: impl Into<String>,
        family: FamilyId,
        values: Values,
    ) -> Result<ObjectTypeId, ModelError> {
        self.check_object_type(family, &values)?;
        let id = ObjectTypeId(self.take_id());
        self.object_types.insert(
            id,
            ObjectType {
                id,
                guid: GlobalId::new(),
                name: name.into(),
                family,
                values,
                mesh: None,
                symbol: None,
                more_symbols: Vec::new(),
                own: None,
                elevation: Vec::new(),
                plan_image: None,
            },
        );
        Ok(id)
    }

    /// Gives an object type a picture for presentation plans (none: its
    /// symbol).
    pub fn set_object_type_plan_image(
        &mut self,
        id: ObjectTypeId,
        image: Option<PlanImage>,
    ) -> Result<(), ModelError> {
        if let Some(i) = &image {
            if i.png.is_empty() || !(i.aspect.is_finite() && i.aspect > 0.0) {
                return Err(ModelError::InvalidShape("a plan picture needs a picture"));
            }
        }
        let t = self
            .object_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownObjectType(id))?;
        t.plan_image = image;
        Ok(())
    }

    /// Gives an object type its own drawings for elevations (none: the
    /// ones ArchiSim makes).
    pub fn set_object_type_elevation(
        &mut self,
        id: ObjectTypeId,
        figures: Vec<ElevationFigure>,
    ) -> Result<(), ModelError> {
        for f in &figures {
            let ok = match &f.art {
                ElevationArt::Lines(items) => !items.is_empty(),
                ElevationArt::Image { png, aspect } => {
                    !png.is_empty() && aspect.is_finite() && *aspect > 0.0
                }
            };
            if !ok {
                return Err(ModelError::InvalidShape(
                    "an elevation drawing needs something drawn",
                ));
            }
        }
        let t = self
            .object_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownObjectType(id))?;
        t.elevation = figures;
        Ok(())
    }

    /// How tall an object is: a plant, as it grew; anything else, its
    /// type's 3D.
    pub fn figure_height(&self, id: ObjectId) -> f64 {
        if let Some(p) = self.object_plant(id) {
            return p.height.max(0.05);
        }
        let Some(o) = self.objects.get(&id) else {
            return 1.0;
        };
        self.object_type_box_3d(o.object_type)
            .map_or(1.0, |(lo, hi)| (hi[2] - lo[2]).max(0.05))
    }

    /// The drawing of its own an object shows to a view looking along
    /// `toward` (level, in the project's axes): from the side or the
    /// front as it is seen, a DXF in a technical view and a picture in a
    /// presentation (either, if it has only one). `None`: ArchiSim draws
    /// it.
    pub fn shown_figure(
        &self,
        id: ObjectId,
        toward: [f64; 2],
        presenting: bool,
    ) -> Option<ShownFigure> {
        let o = self.objects.get(&id)?;
        let t = self.object_types.get(&o.object_type)?;
        if t.elevation.is_empty() {
            return None;
        }
        // The view in the object's axes: its front looks towards +y.
        let (sn, cs) = (-o.rotation).sin_cos();
        let local = [
            toward[0] * cs - toward[1] * sn,
            toward[0] * sn + toward[1] * cs,
        ];
        let side = local[0].abs() > local[1].abs();
        let fits = |f: &&ElevationFigure| f.side == side;
        let image = |f: &&ElevationFigure| matches!(f.art, ElevationArt::Image { .. });
        let wanted = |f: &&ElevationFigure| image(f) == presenting;
        let pick = t
            .elevation
            .iter()
            .filter(fits)
            .find(wanted)
            .or_else(|| t.elevation.iter().find(fits))
            .or_else(|| t.elevation.iter().find(wanted))
            .or_else(|| t.elevation.first())?;
        // A front seen from behind, a side seen from the other side: the
        // other way round. Drawings of a side show it looking along -x.
        let flip = if pick.side {
            local[0] > 0.0
        } else {
            local[1] > 0.0
        };
        Some(ShownFigure {
            figure: pick.clone(),
            flip,
            height: self.figure_height(id),
        })
    }

    /// The object's own type ([`OwnType`]): the one it has, or a copy of
    /// its type made its own now, «(ejemplar)». Changing it changes only
    /// this object.
    pub fn own_object_type(&mut self, id: ObjectId) -> Result<ObjectTypeId, ModelError> {
        let o = self.objects.get(&id).ok_or(ModelError::UnknownObject(id))?;
        let t = self
            .object_types
            .get(&o.object_type)
            .ok_or(ModelError::UnknownObjectType(o.object_type))?
            .clone();
        if t.own.is_some_and(|w| w.object == id) {
            return Ok(t.id);
        }
        // The type it comes from: a copy of an own type comes from that
        // type's.
        let base = t.own.map_or(t.id, |w| w.base);
        let base_name = self
            .object_types
            .get(&base)
            .map_or(t.name.clone(), |b| b.name.clone());
        let mut name = format!("{base_name} (ejemplar)");
        let mut n = 2;
        while self.object_types.values().any(|x| x.name == name) {
            name = format!("{base_name} (ejemplar {n})");
            n += 1;
        }
        let new = ObjectTypeId(self.take_id());
        self.object_types.insert(
            new,
            ObjectType {
                id: new,
                guid: GlobalId::new(),
                name,
                own: Some(OwnType { object: id, base }),
                ..t
            },
        );
        self.edit_object(id, |o| o.object_type = new)?;
        Ok(new)
    }

    /// Takes what was changed on an object to the type it came from: every
    /// object of that type takes it, this one goes back to it and its own
    /// type goes. Returns the type.
    pub fn own_type_to_base(&mut self, id: ObjectId) -> Result<ObjectTypeId, ModelError> {
        let o = self.objects.get(&id).ok_or(ModelError::UnknownObject(id))?;
        let t = self
            .object_types
            .get(&o.object_type)
            .ok_or(ModelError::UnknownObjectType(o.object_type))?
            .clone();
        let Some(own) = t.own.filter(|w| w.object == id) else {
            return Ok(t.id);
        };
        if !self.object_types.contains_key(&own.base) {
            // Its type is gone: this one becomes a type of its own.
            let name = t.name.replace(" (ejemplar)", "");
            if let Some(x) = self.object_types.get_mut(&t.id) {
                x.own = None;
                x.name = name;
            }
            return Ok(t.id);
        }
        self.set_object_type_values(own.base, t.values.clone())?;
        if let Some(b) = self.object_types.get_mut(&own.base) {
            b.mesh = t.mesh;
            b.symbol = t.symbol.clone();
            b.more_symbols = t.more_symbols.clone();
        }
        self.edit_object(id, |o| o.object_type = own.base)?;
        self.object_types.remove(&t.id);
        Ok(own.base)
    }

    /// Changes the values of an object type; every object of it follows.
    pub fn set_object_type_values(
        &mut self,
        id: ObjectTypeId,
        values: Values,
    ) -> Result<(), ModelError> {
        let family = self
            .object_types
            .get(&id)
            .ok_or(ModelError::UnknownObjectType(id))?
            .family;
        self.check_object_type(family, &values)?;
        if let Some(t) = self.object_types.get_mut(&id) {
            t.values = values;
        }
        Ok(())
    }

    /// Renames an object type (names are unique among object types).
    pub fn rename_object_type(
        &mut self,
        id: ObjectTypeId,
        name: impl Into<String>,
    ) -> Result<(), ModelError> {
        let name = name.into();
        if self
            .object_types
            .values()
            .any(|t| t.id != id && t.name == name)
        {
            return Err(ModelError::DuplicateName(name));
        }
        let t = self
            .object_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownObjectType(id))?;
        t.name = name;
        Ok(())
    }

    /// Gives an object type a `GlobalId` of its own choosing (a library
    /// made always the same, as the extended library is).
    pub fn set_object_type_guid(
        &mut self,
        id: ObjectTypeId,
        guid: GlobalId,
    ) -> Result<(), ModelError> {
        if self
            .object_types
            .values()
            .any(|t| t.id != id && t.guid == guid)
        {
            return Err(ModelError::DuplicateGuid(guid.to_string()));
        }
        let t = self
            .object_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownObjectType(id))?;
        t.guid = guid;
        Ok(())
    }

    pub fn object_type(&self, id: ObjectTypeId) -> Option<&ObjectType> {
        self.object_types.get(&id)
    }

    pub fn object_types(&self) -> impl Iterator<Item = &ObjectType> {
        self.object_types.values()
    }

    /// How many objects use a type (and railings and stairs, for a
    /// railing type).
    pub fn object_type_users(&self, id: ObjectTypeId) -> usize {
        self.objects
            .values()
            .filter(|o| o.object_type == id)
            .count()
            + self
                .railings
                .values()
                .filter(|r| r.railing_type == Some(id))
                .count()
            + self
                .stairs
                .values()
                .filter(|s| s.railing_type == Some(id))
                .count()
    }

    fn validate_object(&self, o: &NewObject) -> Result<(), ModelError> {
        if !self.levels.contains_key(&o.level) {
            return Err(ModelError::UnknownLevel(o.level));
        }
        if !self.object_types.contains_key(&o.object_type) {
            return Err(ModelError::UnknownObjectType(o.object_type));
        }
        finite_point(o.position, "object position")?;
        finite(o.rotation, "object rotation")?;
        finite(o.base_offset, "object base offset")
    }

    pub fn add_object(&mut self, spec: NewObject) -> Result<ObjectId, ModelError> {
        self.validate_object(&spec)?;
        let id = ObjectId(self.take_id());
        self.objects.insert(
            id,
            Object {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                object_type: spec.object_type,
                position: spec.position,
                rotation: spec.rotation,
                base_offset: spec.base_offset,
                values: Values::new(),
            },
        );
        self.stamp_phase(id.0);
        Ok(id)
    }

    /// Changes an object; nothing changes if the result is not valid.
    pub fn edit_object(
        &mut self,
        id: ObjectId,
        edit: impl FnOnce(&mut Object),
    ) -> Result<(), ModelError> {
        let mut o = self
            .objects
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownObject(id))?;
        edit(&mut o);
        self.validate_object(&spec_of(&o))?;
        self.objects.insert(id, o);
        Ok(())
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Result<Object, ModelError> {
        let o = self
            .objects
            .remove(&id)
            .ok_or(ModelError::UnknownObject(id))?;
        // Its own type goes with it; if a copy of it has it too, the copy
        // keeps it as its own.
        if self
            .object_types
            .get(&o.object_type)
            .is_some_and(|t| t.own.is_some_and(|w| w.object == id))
        {
            let heir = self
                .objects
                .values()
                .find(|x| x.object_type == o.object_type)
                .map(|x| x.id);
            match heir {
                Some(h) => {
                    if let Some(t) = self.object_types.get_mut(&o.object_type) {
                        if let Some(w) = t.own.as_mut() {
                            w.object = h;
                        }
                    }
                }
                None => {
                    self.object_types.remove(&o.object_type);
                }
            }
        }
        Ok(o)
    }

    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    pub fn objects(&self) -> impl Iterator<Item = &Object> {
        self.objects.values()
    }

    pub fn objects_on_level(&self, level: LevelId) -> impl Iterator<Item = &Object> {
        self.objects.values().filter(move |o| o.level == level)
    }

    /// The 3D body and outputs of a placed object, in the family's axes.
    pub fn evaluate_object(&self, id: ObjectId) -> Result<Evaluation, ModelError> {
        let o = self.objects.get(&id).ok_or(ModelError::UnknownObject(id))?;
        let t = self
            .object_types
            .get(&o.object_type)
            .ok_or(ModelError::UnknownObjectType(o.object_type))?;
        let values = self.object_values(id);
        self.object_family(t.family)?
            .def()
            .evaluate(&t.values, &values, &Values::new())
            .map_err(ModelError::Family)
    }

    /// The plan symbol of a placed object for a drawing scale, in the
    /// family's axes.
    pub fn object_plan_symbol(&self, id: ObjectId, scale: f64) -> Result<Vec<Mark>, ModelError> {
        if let Some(marks) = self.plant_symbol(id, false, scale) {
            return Ok(marks);
        }
        let o = self.objects.get(&id).ok_or(ModelError::UnknownObject(id))?;
        let t = self
            .object_types
            .get(&o.object_type)
            .ok_or(ModelError::UnknownObjectType(o.object_type))?;
        self.object_family(t.family)?
            .def()
            .evaluate_plan(&t.values, &self.object_values(id), &Values::new(), scale)
            .map_err(ModelError::Family)
    }

    /// The presentation symbol of a placed object (its family's `plan
    /// presentation`), each object a little different by its id; `None` if
    /// the family has none.
    pub fn object_presentation_symbol(
        &self,
        id: ObjectId,
        scale: f64,
    ) -> Option<Result<Vec<Mark>, ModelError>> {
        if let Some(marks) = self.plant_symbol(id, true, scale) {
            return Some(Ok(marks));
        }
        let o = self.objects.get(&id)?;
        let t = self.object_types.get(&o.object_type)?;
        let family = self.object_family(t.family).ok()?;
        // A number from 0 to 1 that stays with the object.
        let seed = {
            let mut h = id.0.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            h ^= h >> 31;
            h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h ^= h >> 29;
            (h >> 11) as f64 / (1u64 << 53) as f64
        };
        let builtins: Values = [("seed".to_string(), crate::family::Value::Number(seed))]
            .into_iter()
            .collect();
        family
            .def()
            .evaluate_presentation(&t.values, &self.object_values(id), &builtins)
            .map(|r| r.map_err(ModelError::Family))
    }

    /// The box of an object type in plan, in the family's axes: lowest and
    /// highest `x` and `y` of its parts.
    pub fn object_type_box(&self, id: ObjectTypeId) -> Option<([f64; 2], [f64; 2])> {
        let t = self.object_types.get(&id)?;
        let instance = self.family_defaults(t.family, ParamScope::Instance).ok()?;
        let ev = self
            .object_family(t.family)
            .ok()?
            .def()
            .evaluate(&t.values, &instance, &Values::new())
            .ok()?;
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in &ev.parts {
            for k in 0..2 {
                lo[k] = lo[k].min(p.min[k]);
                hi[k] = hi[k].max(p.max[k]);
            }
        }
        (!ev.parts.is_empty()).then_some((lo, hi))
    }

    /// The box of the objects of a type in 3D, in their own axes (`z` over
    /// their base).
    pub fn object_type_box_3d(&self, id: ObjectTypeId) -> Option<([f64; 3], [f64; 3])> {
        let t = self.object_types.get(&id)?;
        let instance = self.family_defaults(t.family, ParamScope::Instance).ok()?;
        let ev = self
            .object_family(t.family)
            .ok()?
            .def()
            .evaluate(&t.values, &instance, &Values::new())
            .ok()?;
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in &ev.parts {
            for k in 0..3 {
                lo[k] = lo[k].min(p.min[k]);
                hi[k] = hi[k].max(p.max[k]);
            }
        }
        (!ev.parts.is_empty()).then_some((lo, hi))
    }

    /// The box of a placed object in plan (its 3D parts seen from above),
    /// counter-clockwise; empty if it has none.
    pub fn object_outline(&self, id: ObjectId) -> Vec<Point2> {
        let (Some(o), Ok(ev)) = (self.objects.get(&id), self.evaluate_object(id)) else {
            return Vec::new();
        };
        if ev.parts.is_empty() {
            return Vec::new();
        }
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in &ev.parts {
            for k in 0..2 {
                lo[k] = lo[k].min(p.min[k]);
                hi[k] = hi[k].max(p.max[k]);
            }
        }
        vec![
            o.to_plan(lo[0], lo[1]),
            o.to_plan(hi[0], lo[1]),
            o.to_plan(hi[0], hi[1]),
            o.to_plan(lo[0], hi[1]),
        ]
    }

    pub(super) fn restore_objects(
        &mut self,
        types: Vec<ObjectType>,
        objects: Vec<Object>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for t in types {
            let name = format!("object type {} «{}»", t.id.0, t.name);
            claim(t.id.0, Some(&t.guid))
                .and_then(|_| self.check_object_type(t.family, &t.values))
                .and_then(|_| match t.mesh {
                    Some(m) if !self.meshes.contains_key(&m) => Err(ModelError::UnknownMesh(m)),
                    _ => Ok(()),
                })
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.object_types.insert(t.id, t);
        }
        for o in objects {
            let name = format!("object {} ({})", o.id.0, o.guid);
            claim(o.id.0, Some(&o.guid))
                .and_then(|_| self.validate_object(&spec_of(&o)))
                .map_err(|error| RestoreError {
                    element: name,
                    error,
                })?;
            self.objects.insert(o.id, o);
        }
        Ok(())
    }
}

fn spec_of(o: &Object) -> NewObject {
    NewObject {
        level: o.level,
        object_type: o.object_type,
        position: o.position,
        rotation: o.rotation,
        base_offset: o.base_offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every way of drawing a tree's canopy draws, as the tree's own
    /// choice or its type's.
    #[test]
    fn every_canopy_draws() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        m.add_material(
            "Vegetación",
            "vegetation",
            [126, 164, 98],
            CutPattern::Solid,
        );
        m.add_material("Corteza", "wood", [112, 86, 62], CutPattern::Solid);
        let f = m.system_family("Árbol").unwrap();
        let values = m.family_defaults(f, ParamScope::Type).unwrap();
        let t = m.add_object_type("Árbol", f, values).unwrap();
        let o = m
            .add_object(NewObject {
                level: pb,
                object_type: t,
                position: Point2::new(0.0, 0.0),
                rotation: 0.0,
                base_offset: 0.0,
            })
            .unwrap();
        for shape in [
            "lóbulos",
            "círculo",
            "acuarela",
            "ramas",
            "follaje",
            "palmera",
            "conífera",
        ] {
            for look in ["transparente", "rellena", "contorno"] {
                let v: Values = [
                    ("shape_own".to_string(), Value::Text(shape.into())),
                    ("look_own".to_string(), Value::Text(look.into())),
                    ("opacity_own".to_string(), Value::Number(0.8)),
                ]
                .into_iter()
                .collect();
                m.set_object_values(o, v).unwrap();
                let marks = m.object_presentation_symbol(o, 50.0).unwrap().unwrap();
                assert!(marks.len() >= 2, "{shape} {look}");
                let tinted = marks
                    .iter()
                    .any(|k| matches!(k, Mark::Polygon { tint: Some(_), .. }));
                assert!(tinted, "the trunk at least: {shape} {look}");
            }
        }
        assert!(m
            .set_object_values(
                o,
                [("shape_own".to_string(), Value::Text("cuadrada".into()))]
                    .into_iter()
                    .collect()
            )
            .is_err());
    }

    #[test]
    fn every_object_family_places_and_draws() {
        let mut m = Model::new("t");
        let pb = m.add_level("PB", 0.0);
        m.add_material("Madera de pino", "wood", [196, 160, 112], CutPattern::Solid);
        m.add_material("Porcelana", "ceramic", [240, 240, 236], CutPattern::Solid);
        for (name, _) in family::object_families() {
            let f = m.system_family(&name).unwrap();
            let values = m.family_defaults(f, ParamScope::Type).unwrap();
            let t = m
                .add_object_type(format!("{name} tipo"), f, values)
                .unwrap();
            let o = m
                .add_object(NewObject {
                    level: pb,
                    object_type: t,
                    position: Point2::new(1.0, 2.0),
                    rotation: 0.5,
                    base_offset: 0.0,
                })
                .unwrap();
            let ev = m.evaluate_object(o).unwrap();
            assert!(!ev.parts.is_empty(), "{name}");
            let category = m.family(f).unwrap().def().category.clone();
            // Trees, shrubs and people stand on their point.
            let centred = name == "Mesa" || category == "people" || category == "nature" || {
                (category == "site" && name != "Seto")
                    || matches!(
                        name.as_str(),
                        "Farola"
                            | "Papelera"
                            | "Bolardo"
                            | "Alcorque"
                            | "Aparcabicis"
                            | "Señal de tráfico"
                            | "Ducha de piscina"
                            | "Mampara acústica"
                    )
            };
            for p in &ev.parts {
                // On the floor; against the wall (nothing behind the
                // origin) but for tables, which have chairs all round.
                assert!(p.min[2] >= -1e-9, "{name} {p:?}");
                assert!(centred || p.min[1] >= -1e-9, "{name} {p:?}");
            }
            for scale in [50.0, 100.0] {
                assert!(
                    !m.object_plan_symbol(o, scale).unwrap().is_empty(),
                    "{name}"
                );
            }
            assert_eq!(m.object_outline(o).len(), 4);
            assert!([
                "IfcFurniture",
                "IfcSanitaryTerminal",
                "IfcGeographicElement",
                "IfcBuildingElementProxy",
                "IfcLightFixture",
                "IfcVehicle",
                "IfcRailing"
            ]
            .contains(&ev.outputs["ifc_class"].as_text().unwrap()));
            assert!(!m.family(f).unwrap().def().folder.is_empty(), "{name}");
        }
        // A door family cannot make objects.
        let door = m.system_family(family::SWING_DOOR).unwrap();
        let v = m.family_defaults(door, ParamScope::Type).unwrap();
        assert!(matches!(
            m.add_object_type("x", door, v),
            Err(ModelError::NotAnObject(_))
        ));
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}
