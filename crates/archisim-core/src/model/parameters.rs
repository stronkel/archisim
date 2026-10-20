//! Parameters of the project (Revit's project and shared parameters,
//! ArchiCAD's properties): data the architect adds to the elements of some
//! categories — the fire rating of a door, the finish of a wall, the cost
//! code of a slab — of the element itself or of its type. They show in the
//! properties, the schedules and the IFC (each in its property set), and a
//! file of shared parameters (`.asparam`) takes the same definitions, with
//! the same identity, to other projects, so that "Resistencia al fuego" is
//! the same column in every schedule of the studio.
//!
//! Values are kept by element (or type) id in a table of the model, as
//! phases are: any element can have them without knowing about it.

use super::*;

/// Identifies a [`UserParameter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserParameterId(pub u64);

/// What a parameter holds.
#[derive(Debug, Clone, PartialEq)]
pub enum UserKind {
    Text,
    /// A plain number.
    Number,
    Integer,
    /// Metres.
    Length,
    /// Square metres.
    Area,
    Bool,
    /// One of a list.
    Choice(Vec<String>),
}

impl UserKind {
    /// The name in shared parameter files.
    pub fn code(&self) -> &'static str {
        match self {
            UserKind::Text => "text",
            UserKind::Number => "number",
            UserKind::Integer => "integer",
            UserKind::Length => "length",
            UserKind::Area => "area",
            UserKind::Bool => "bool",
            UserKind::Choice(_) => "choice",
        }
    }

    /// The name shown to the user.
    pub fn label(&self) -> &'static str {
        match self {
            UserKind::Text => "Texto",
            UserKind::Number => "Número",
            UserKind::Integer => "Entero",
            UserKind::Length => "Longitud",
            UserKind::Area => "Superficie",
            UserKind::Bool => "Sí/no",
            UserKind::Choice(_) => "Lista de opciones",
        }
    }

    /// The value a new parameter of this kind starts with.
    pub fn empty(&self) -> Value {
        match self {
            UserKind::Text => Value::Text(String::new()),
            UserKind::Bool => Value::Bool(false),
            UserKind::Choice(o) => Value::Text(o.first().cloned().unwrap_or_default()),
            _ => Value::Number(0.0),
        }
    }

    /// Whether a value is one of this kind.
    pub fn accepts(&self, v: &Value) -> bool {
        match (self, v) {
            (UserKind::Text, Value::Text(_)) => true,
            (UserKind::Bool, Value::Bool(_)) => true,
            (UserKind::Choice(o), Value::Text(t)) => o.contains(t),
            (UserKind::Integer, Value::Number(n)) => n.is_finite() && n.fract() == 0.0,
            (UserKind::Number | UserKind::Length | UserKind::Area, Value::Number(n)) => {
                n.is_finite()
            }
            _ => false,
        }
    }
}

/// The kinds of element a parameter can be given to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParamCategory {
    Walls,
    Slabs,
    Doors,
    Windows,
    Columns,
    Beams,
    Roofs,
    Stairs,
    CurtainWalls,
    Railings,
    Objects,
    Rooms,
}

impl ParamCategory {
    pub const ALL: [ParamCategory; 12] = [
        ParamCategory::Walls,
        ParamCategory::Slabs,
        ParamCategory::Doors,
        ParamCategory::Windows,
        ParamCategory::Columns,
        ParamCategory::Beams,
        ParamCategory::Roofs,
        ParamCategory::Stairs,
        ParamCategory::CurtainWalls,
        ParamCategory::Railings,
        ParamCategory::Objects,
        ParamCategory::Rooms,
    ];

    pub fn code(self) -> &'static str {
        match self {
            ParamCategory::Walls => "walls",
            ParamCategory::Slabs => "slabs",
            ParamCategory::Doors => "doors",
            ParamCategory::Windows => "windows",
            ParamCategory::Columns => "columns",
            ParamCategory::Beams => "beams",
            ParamCategory::Roofs => "roofs",
            ParamCategory::Stairs => "stairs",
            ParamCategory::CurtainWalls => "curtain_walls",
            ParamCategory::Railings => "railings",
            ParamCategory::Objects => "objects",
            ParamCategory::Rooms => "rooms",
        }
    }

    pub fn from_code(code: &str) -> Option<ParamCategory> {
        ParamCategory::ALL.into_iter().find(|c| c.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            ParamCategory::Walls => "Muros",
            ParamCategory::Slabs => "Forjados",
            ParamCategory::Doors => "Puertas",
            ParamCategory::Windows => "Ventanas",
            ParamCategory::Columns => "Pilares",
            ParamCategory::Beams => "Vigas",
            ParamCategory::Roofs => "Cubiertas",
            ParamCategory::Stairs => "Escaleras",
            ParamCategory::CurtainWalls => "Muros cortina",
            ParamCategory::Railings => "Barandillas",
            ParamCategory::Objects => "Objetos",
            ParamCategory::Rooms => "Locales",
        }
    }

    /// Whether its elements have types (walls, doors…) or not (rooms…).
    pub fn has_types(self) -> bool {
        !matches!(
            self,
            ParamCategory::Stairs | ParamCategory::Railings | ParamCategory::Rooms
        )
    }
}

/// A parameter of the project.
#[derive(Debug, Clone, PartialEq)]
pub struct UserParameter {
    pub id: UserParameterId,
    /// Its identity across projects (a shared parameter keeps it).
    pub guid: GlobalId,
    /// The name shown and written to IFC, e.g. "Resistencia al fuego".
    pub name: String,
    pub kind: UserKind,
    /// The IFC property set it goes in, e.g. "AD5_Datos" (also the group in
    /// the properties).
    pub group: String,
    pub categories: Vec<ParamCategory>,
    /// Of the type (every element of the type has the same value) or of
    /// each element.
    pub by_type: bool,
    /// The value of the elements that have not been given one.
    pub default: Value,
    pub description: String,
}

/// A parameter as it is defined, before it is in a project.
#[derive(Debug, Clone, PartialEq)]
pub struct NewUserParameter {
    pub name: String,
    pub kind: UserKind,
    pub group: String,
    pub categories: Vec<ParamCategory>,
    pub by_type: bool,
    pub default: Value,
    pub description: String,
}

impl Model {
    /// The project's parameters, in the order they were added.
    pub fn user_parameters(&self) -> impl Iterator<Item = &UserParameter> {
        self.user_params.values()
    }

    pub fn user_parameter(&self, id: UserParameterId) -> Option<&UserParameter> {
        self.user_params.get(&id)
    }

    fn check_user_parameter(
        &self,
        p: &NewUserParameter,
        except: Option<UserParameterId>,
    ) -> Result<(), ModelError> {
        let bad = |why: &str| Err(ModelError::InvalidParameter(why.to_string()));
        if p.name.trim().is_empty() {
            return bad("el parámetro necesita un nombre");
        }
        if p.group.trim().is_empty() {
            return bad("el parámetro necesita un grupo (su conjunto de propiedades IFC)");
        }
        if p.categories.is_empty() {
            return bad("el parámetro tiene que ser de alguna categoría");
        }
        if let UserKind::Choice(o) = &p.kind {
            if o.is_empty() {
                return bad("una lista necesita al menos una opción");
            }
        }
        if !p.kind.accepts(&p.default) {
            return bad("el valor por defecto no es de la clase del parámetro");
        }
        if p.by_type && !p.categories.iter().all(|c| c.has_types()) {
            return bad("los locales, escaleras y barandillas no tienen tipo: el parámetro tiene que ser del elemento");
        }
        let taken = self
            .user_params
            .values()
            .any(|q| Some(q.id) != except && q.name.trim().eq_ignore_ascii_case(p.name.trim()));
        if taken {
            return Err(ModelError::InvalidParameter(format!(
                "ya hay un parámetro «{}»",
                p.name.trim()
            )));
        }
        Ok(())
    }

    /// Adds a parameter to the project.
    pub fn add_user_parameter(
        &mut self,
        p: NewUserParameter,
    ) -> Result<UserParameterId, ModelError> {
        self.check_user_parameter(&p, None)?;
        let id = UserParameterId(self.take_id());
        self.user_params.insert(
            id,
            UserParameter {
                id,
                guid: GlobalId::new(),
                name: p.name.trim().to_string(),
                kind: p.kind,
                group: p.group.trim().to_string(),
                categories: p.categories,
                by_type: p.by_type,
                default: p.default,
                description: p.description,
            },
        );
        Ok(id)
    }

    /// Changes a parameter's definition. The values it already has that no
    /// longer fit (another kind, an option that is gone, a category it no
    /// longer has) are dropped.
    pub fn edit_user_parameter(
        &mut self,
        id: UserParameterId,
        p: NewUserParameter,
    ) -> Result<(), ModelError> {
        if !self.user_params.contains_key(&id) {
            return Err(ModelError::UnknownElement(id.0));
        }
        self.check_user_parameter(&p, Some(id))?;
        let def = self.user_params.get_mut(&id).expect("checked");
        let by_type_changed = def.by_type != p.by_type;
        def.name = p.name.trim().to_string();
        def.kind = p.kind;
        def.group = p.group.trim().to_string();
        def.categories = p.categories;
        def.by_type = p.by_type;
        def.default = p.default;
        def.description = p.description;
        let def = def.clone();
        let targets: Vec<u64> = self.param_values.keys().copied().collect();
        for t in targets {
            let fits = !by_type_changed
                && self
                    .param_target_category(t, def.by_type)
                    .is_some_and(|c| def.categories.contains(&c));
            if let Some(vals) = self.param_values.get_mut(&t) {
                if let Some(v) = vals.get(&id) {
                    if !fits || !def.kind.accepts(v) {
                        vals.remove(&id);
                    }
                }
            }
        }
        self.param_values.retain(|_, v| !v.is_empty());
        Ok(())
    }

    /// Removes a parameter and its values.
    pub fn remove_user_parameter(
        &mut self,
        id: UserParameterId,
    ) -> Result<UserParameter, ModelError> {
        let p = self
            .user_params
            .remove(&id)
            .ok_or(ModelError::UnknownElement(id.0))?;
        for v in self.param_values.values_mut() {
            v.remove(&id);
        }
        self.param_values.retain(|_, v| !v.is_empty());
        Ok(p)
    }

    /// The category of an element (`by_type` false) or of a type (true),
    /// by its id.
    pub fn param_target_category(&self, id: u64, by_type: bool) -> Option<ParamCategory> {
        if by_type {
            if self.wall_types.contains_key(&WallTypeId(id)) {
                return Some(ParamCategory::Walls);
            }
            if self.slab_types.contains_key(&SlabTypeId(id)) {
                // A slab type is also a roof type: the parameter decides.
                return Some(ParamCategory::Slabs);
            }
            if let Some(t) = self.opening_types.get(&OpeningTypeId(id)) {
                return Some(if t.is_door() {
                    ParamCategory::Doors
                } else {
                    ParamCategory::Windows
                });
            }
            if let Some(t) = self.member_type(MemberTypeId(id)) {
                return Some(match t.usage {
                    MemberUsage::Column => ParamCategory::Columns,
                    MemberUsage::Beam => ParamCategory::Beams,
                });
            }
            if self.object_type(ObjectTypeId(id)).is_some() {
                return Some(ParamCategory::Objects);
            }
            if self.curtain_types.contains_key(&CurtainTypeId(id)) {
                return Some(ParamCategory::CurtainWalls);
            }
            return None;
        }
        if self.walls.contains_key(&WallId(id)) {
            return Some(ParamCategory::Walls);
        }
        if self.slabs.contains_key(&SlabId(id)) {
            return Some(ParamCategory::Slabs);
        }
        if let Some(o) = self.openings.get(&OpeningId(id)) {
            return self.opening_types.get(&o.opening_type).map(|t| {
                if t.is_door() {
                    ParamCategory::Doors
                } else {
                    ParamCategory::Windows
                }
            });
        }
        if self.column(ColumnId(id)).is_some() {
            return Some(ParamCategory::Columns);
        }
        if self.beam(BeamId(id)).is_some() {
            return Some(ParamCategory::Beams);
        }
        if self.roof(RoofId(id)).is_some() {
            return Some(ParamCategory::Roofs);
        }
        if self.stair(StairId(id)).is_some() {
            return Some(ParamCategory::Stairs);
        }
        if self.curtain_wall(CurtainWallId(id)).is_some() {
            return Some(ParamCategory::CurtainWalls);
        }
        if self.railing(RailingId(id)).is_some() {
            return Some(ParamCategory::Railings);
        }
        if self.object(ObjectId(id)).is_some() {
            return Some(ParamCategory::Objects);
        }
        if self.room(RoomId(id)).is_some() {
            return Some(ParamCategory::Rooms);
        }
        None
    }

    /// The `GlobalId` of an element or type of the model, by its id.
    pub fn element_guid(&self, id: u64) -> Option<GlobalId> {
        let g = |x: Option<&GlobalId>| x.cloned();
        g(self.walls.get(&WallId(id)).map(|x| &x.guid))
            .or_else(|| g(self.slabs.get(&SlabId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.openings.get(&OpeningId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.column(ColumnId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.beam(BeamId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.roof(RoofId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.stair(StairId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.curtain_wall(CurtainWallId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.railing(RailingId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.object(ObjectId(id)).map(|x| &x.guid)))
            .or_else(|| g(self.room(RoomId(id)).map(|x| &x.guid)))
    }

    /// The type of an element, by their ids, if it has one.
    pub fn type_of_element(&self, id: u64) -> Option<u64> {
        if let Some(w) = self.walls.get(&WallId(id)) {
            return Some(w.wall_type.0);
        }
        if let Some(s) = self.slabs.get(&SlabId(id)) {
            return Some(s.slab_type.0);
        }
        if let Some(o) = self.openings.get(&OpeningId(id)) {
            return Some(o.opening_type.0);
        }
        if let Some(c) = self.column(ColumnId(id)) {
            return Some(c.member_type.0);
        }
        if let Some(b) = self.beam(BeamId(id)) {
            return Some(b.member_type.0);
        }
        if let Some(r) = self.roof(RoofId(id)) {
            return Some(r.roof_type.0);
        }
        if let Some(c) = self.curtain_wall(CurtainWallId(id)) {
            return Some(c.curtain_type.0);
        }
        if let Some(o) = self.object(ObjectId(id)) {
            return Some(o.object_type.0);
        }
        None
    }

    /// The parameters an element has (those of its category), each with
    /// its value: the element's own, its type's for parameters of the
    /// type, or the default.
    pub fn element_parameters(&self, id: u64) -> Vec<(&UserParameter, Value)> {
        let Some(cat) = self.param_target_category(id, false) else {
            return Vec::new();
        };
        let ty = self.type_of_element(id);
        self.user_params
            .values()
            .filter(|p| p.categories.contains(&cat))
            .map(|p| {
                let target = if p.by_type { ty } else { Some(id) };
                let v = target
                    .and_then(|t| self.param_values.get(&t))
                    .and_then(|v| v.get(&p.id))
                    .cloned()
                    .unwrap_or_else(|| p.default.clone());
                (p, v)
            })
            .collect()
    }

    /// The value of one parameter for an element (its type's for a
    /// parameter of the type), or `None` if the element does not have it.
    pub fn element_parameter(&self, id: u64, param: UserParameterId) -> Option<Value> {
        self.element_parameters(id)
            .into_iter()
            .find(|(p, _)| p.id == param)
            .map(|(_, v)| v)
    }

    /// The parameters of the type of a category, with their values.
    pub fn type_parameters(&self, type_id: u64) -> Vec<(&UserParameter, Value)> {
        let Some(cat) = self.param_target_category(type_id, true) else {
            return Vec::new();
        };
        // A slab type is also a roof type.
        let also_roof = cat == ParamCategory::Slabs;
        self.user_params
            .values()
            .filter(|p| {
                p.by_type
                    && (p.categories.contains(&cat)
                        || (also_roof && p.categories.contains(&ParamCategory::Roofs)))
            })
            .map(|p| {
                let v = self
                    .param_values
                    .get(&type_id)
                    .and_then(|v| v.get(&p.id))
                    .cloned()
                    .unwrap_or_else(|| p.default.clone());
                (p, v)
            })
            .collect()
    }

    /// Gives an element (or a type, for a parameter of the type) a value
    /// of a parameter.
    pub fn set_parameter_value(
        &mut self,
        target: u64,
        param: UserParameterId,
        value: Value,
    ) -> Result<(), ModelError> {
        let p = self
            .user_params
            .get(&param)
            .ok_or(ModelError::UnknownElement(param.0))?;
        let cat = self
            .param_target_category(target, p.by_type)
            .ok_or(ModelError::UnknownElement(target))?;
        let roof_type = p.by_type
            && cat == ParamCategory::Slabs
            && p.categories.contains(&ParamCategory::Roofs);
        if !p.categories.contains(&cat) && !roof_type {
            return Err(ModelError::InvalidParameter(format!(
                "«{}» no es un parámetro de {}",
                p.name,
                cat.label().to_lowercase()
            )));
        }
        if !p.kind.accepts(&value) {
            return Err(ModelError::InvalidParameter(format!(
                "«{}» es de la clase {}",
                p.name,
                p.kind.label().to_lowercase()
            )));
        }
        self.param_values
            .entry(target)
            .or_default()
            .insert(param, value);
        Ok(())
    }

    /// Every value given, by element or type id (what the project file
    /// keeps): only of things that still exist.
    pub fn parameter_values(&self) -> Vec<(u64, UserParameterId, Value)> {
        let mut out = Vec::new();
        for (t, vals) in &self.param_values {
            for (p, v) in vals {
                let Some(def) = self.user_params.get(p) else {
                    continue;
                };
                if self.param_target_category(*t, def.by_type).is_some() {
                    out.push((*t, *p, v.clone()));
                }
            }
        }
        out
    }

    pub(super) fn restore_parameters(
        &mut self,
        defs: Vec<UserParameter>,
        values: Vec<(u64, UserParameterId, Value)>,
    ) -> Result<(), RestoreError> {
        for d in defs {
            self.next_id = self.next_id.max(d.id.0 + 1);
            self.user_params.insert(d.id, d);
        }
        for (t, p, v) in values {
            self.set_parameter_value(t, p, v)
                .map_err(|error| RestoreError {
                    element: format!("value of parameter {} of element {t}", p.0),
                    error,
                })?;
        }
        Ok(())
    }

    /// Adds the parameters of a shared parameter file that the project does
    /// not have (by `GlobalId`; a parameter of the same name and another
    /// identity is skipped). Returns how many it added.
    pub fn load_shared_parameters(&mut self, defs: &[SharedParameter]) -> usize {
        let mut n = 0;
        for d in defs {
            let guid = GlobalId::parse(&d.guid);
            if guid
                .as_ref()
                .is_some_and(|g| self.user_params.values().any(|p| p.guid == *g))
            {
                continue;
            }
            let Ok(id) = self.add_user_parameter(d.def.clone()) else {
                continue;
            };
            if let (Some(g), Some(p)) = (guid, self.user_params.get_mut(&id)) {
                p.guid = g;
            }
            n += 1;
        }
        n
    }

    /// The project's parameters as a shared parameter file keeps them.
    pub fn shared_parameters(&self) -> Vec<SharedParameter> {
        self.user_params
            .values()
            .map(|p| SharedParameter {
                guid: p.guid.as_str().to_string(),
                def: NewUserParameter {
                    name: p.name.clone(),
                    kind: p.kind.clone(),
                    group: p.group.clone(),
                    categories: p.categories.clone(),
                    by_type: p.by_type,
                    default: p.default.clone(),
                    description: p.description.clone(),
                },
            })
            .collect()
    }
}

/// A parameter in a shared parameter file: its identity and definition.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedParameter {
    pub guid: String,
    pub def: NewUserParameter,
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "'"))
}

fn value_text(v: &Value) -> String {
    match v {
        Value::Text(t) => quoted(t),
        Value::Bool(true) => "yes".into(),
        Value::Bool(false) => "no".into(),
        Value::Number(n) => format!("{n}"),
        Value::Material(m) => format!("{m}"),
    }
}

/// The text of a shared parameter file (`.asparam`), one line per
/// parameter:
///
/// ```text
/// parameter "Resistencia al fuego" guid "0x…" kind choice("EI2 30-C5", "EI2 60-C5") group "AD5_Datos" categories doors by_type yes default "EI2 30-C5"
/// ```
pub fn shared_parameters_text(defs: &[SharedParameter]) -> String {
    let mut s = String::from("# Parámetros compartidos de ArchiSim\n");
    for d in defs {
        let kind = match &d.def.kind {
            UserKind::Choice(o) => format!(
                "choice({})",
                o.iter().map(|x| quoted(x)).collect::<Vec<_>>().join(", ")
            ),
            k => k.code().to_string(),
        };
        let cats: Vec<&str> = d.def.categories.iter().map(|c| c.code()).collect();
        s.push_str(&format!(
            "parameter {} guid {} kind {kind} group {} categories {} by_type {} default {}",
            quoted(&d.def.name),
            quoted(&d.guid),
            quoted(&d.def.group),
            cats.join(","),
            if d.def.by_type { "yes" } else { "no" },
            value_text(&d.def.default)
        ));
        if !d.def.description.is_empty() {
            s.push_str(&format!(" description {}", quoted(&d.def.description)));
        }
        s.push('\n');
    }
    s
}

/// Words of a line: quoted texts whole, `choice(…)` whole.
fn words(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut depth = 0;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            '(' if !quoted => {
                depth += 1;
                cur.push(c);
            }
            ')' if !quoted => {
                depth -= 1;
                cur.push(c);
            }
            c if c.is_whitespace() && !quoted && depth == 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if quoted || depth != 0 {
        return Err("comillas o paréntesis sin cerrar".into());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

fn unquote(w: &str) -> String {
    w.trim_matches('"').to_string()
}

/// Reads a shared parameter file. Errors say the line.
pub fn parse_shared_parameters(text: &str) -> Result<Vec<SharedParameter>, String> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let no = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let w = words(line).map_err(|e| format!("línea {no}: {e}"))?;
        let bad = |what: String| format!("línea {no}: {what}");
        if w[0] != "parameter" {
            return Err(bad(format!("línea desconocida «{}»", w[0])));
        }
        let name = unquote(w.get(1).ok_or_else(|| bad("falta el nombre".into()))?);
        let field = |key: &str| {
            w.iter()
                .position(|x| x == key)
                .and_then(|k| w.get(k + 1))
                .cloned()
        };
        let kind_text = field("kind").unwrap_or_else(|| "text".into());
        let kind = if let Some(inner) = kind_text
            .strip_prefix("choice(")
            .and_then(|x| x.strip_suffix(')'))
        {
            UserKind::Choice(
                inner
                    .split(',')
                    .map(|o| unquote(o.trim()))
                    .filter(|o| !o.is_empty())
                    .collect(),
            )
        } else {
            match kind_text.as_str() {
                "text" => UserKind::Text,
                "number" => UserKind::Number,
                "integer" => UserKind::Integer,
                "length" => UserKind::Length,
                "area" => UserKind::Area,
                "bool" => UserKind::Bool,
                other => return Err(bad(format!("clase desconocida «{other}»"))),
            }
        };
        let categories = field("categories")
            .unwrap_or_default()
            .split(',')
            .filter(|c| !c.is_empty())
            .map(|c| {
                ParamCategory::from_code(c.trim())
                    .ok_or_else(|| bad(format!("categoría desconocida «{c}»")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let default = match field("default") {
            None => kind.empty(),
            Some(d) => match &kind {
                UserKind::Text | UserKind::Choice(_) => Value::Text(unquote(&d)),
                UserKind::Bool => Value::Bool(d == "yes"),
                _ => Value::Number(
                    d.replace(',', ".")
                        .parse()
                        .map_err(|_| bad(format!("«{d}» no es un número")))?,
                ),
            },
        };
        out.push(SharedParameter {
            guid: field("guid").map(|g| unquote(&g)).unwrap_or_default(),
            def: NewUserParameter {
                name,
                kind,
                group: field("group")
                    .map(|g| unquote(&g))
                    .unwrap_or_else(|| "ArchiSim_Parámetros".into()),
                categories,
                by_type: field("by_type").is_some_and(|b| b == "yes"),
                default,
                description: field("description")
                    .map(|d| unquote(&d))
                    .unwrap_or_default(),
            },
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fire() -> NewUserParameter {
        NewUserParameter {
            name: "Resistencia al fuego".into(),
            kind: UserKind::Choice(vec!["—".into(), "EI2 30-C5".into(), "EI2 60-C5".into()]),
            group: "AD5_Datos".into(),
            categories: vec![ParamCategory::Doors],
            by_type: true,
            default: Value::Text("—".into()),
            description: "Según DB SI".into(),
        }
    }

    fn project() -> (Model, OpeningId, OpeningTypeId, WallId) {
        let mut m = Model::new("p");
        let level = m.add_level("PB", 0.0);
        let brick = m.add_material("Ladrillo", "brick", [200, 120, 90], CutPattern::Diagonal);
        let wt = m
            .add_wall_type(
                "M",
                WallFunction::Interior,
                vec![Layer {
                    material: brick,
                    thickness: 0.12,
                    function: LayerFunction::Structure,
                }],
            )
            .unwrap();
        let wall = m
            .add_wall(NewWall {
                wall_type: wt,
                level,
                start: Point2::new(0.0, 0.0),
                end: Point2::new(5.0, 0.0),
                reference: ReferenceLine::Center,
                height: 2.7,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap();
        let dt = m
            .add_opening_type(NewOpeningType {
                name: "P".into(),
                kind: OpeningKind::Door(DoorOperation::SingleSwing),
                width: 0.8,
                height: 2.05,
                frame_width: 0.05,
                frame_depth: 0.07,
                frame_material: brick,
                panel_material: brick,
            })
            .unwrap();
        let door = m
            .add_opening(NewOpening {
                opening_type: dt,
                wall,
                along: 2.0,
                sill: 0.0,
                opens_to: WallSide::Interior,
                hand: Hand::Left,
                setback: None,
            })
            .unwrap();
        (m, door, dt, wall)
    }

    #[test]
    fn type_parameters_reach_every_element_of_the_type() {
        let (mut m, door, dt, wall) = project();
        let fire = m.add_user_parameter(fire()).unwrap();
        // Not twice, not without a category, not by type for rooms.
        assert!(m.add_user_parameter(super::tests::fire()).is_err());
        let mut rooms = super::tests::fire();
        rooms.name = "Otro".into();
        rooms.categories = vec![ParamCategory::Rooms];
        assert!(m.add_user_parameter(rooms).is_err());
        // The default until given.
        assert_eq!(
            m.element_parameter(door.0, fire),
            Some(Value::Text("—".into()))
        );
        assert_eq!(m.element_parameter(wall.0, fire), None);
        m.set_parameter_value(dt.0, fire, Value::Text("EI2 30-C5".into()))
            .unwrap();
        assert_eq!(
            m.element_parameter(door.0, fire),
            Some(Value::Text("EI2 30-C5".into()))
        );
        // Wrong option, wrong kind of element.
        assert!(m
            .set_parameter_value(dt.0, fire, Value::Text("EI 90".into()))
            .is_err());
        assert!(m
            .set_parameter_value(door.0, fire, Value::Text("—".into()))
            .is_err());
        // An element parameter.
        let note = m
            .add_user_parameter(NewUserParameter {
                name: "Observaciones".into(),
                kind: UserKind::Text,
                group: "AD5_Datos".into(),
                categories: vec![ParamCategory::Doors, ParamCategory::Walls],
                by_type: false,
                default: Value::Text(String::new()),
                description: String::new(),
            })
            .unwrap();
        m.set_parameter_value(wall.0, note, Value::Text("Revisar".into()))
            .unwrap();
        assert_eq!(m.element_parameters(wall.0).len(), 1);
        assert_eq!(m.parameter_values().len(), 2);
        // Removing the option drops the values that used it.
        let mut changed = super::tests::fire();
        changed.kind = UserKind::Choice(vec!["—".into(), "EI2 60-C5".into()]);
        m.edit_user_parameter(fire, changed).unwrap();
        assert_eq!(
            m.element_parameter(door.0, fire),
            Some(Value::Text("—".into()))
        );
        m.remove_user_parameter(note).unwrap();
        assert!(m.parameter_values().is_empty());
    }

    #[test]
    fn shared_parameter_files_keep_the_identity() {
        let (mut a, _, _, _) = project();
        a.add_user_parameter(fire()).unwrap();
        let text = shared_parameters_text(&a.shared_parameters());
        let read = parse_shared_parameters(&text).unwrap();
        assert_eq!(read, a.shared_parameters());
        let (mut b, _, _, _) = project();
        assert_eq!(b.load_shared_parameters(&read), 1);
        assert_eq!(b.load_shared_parameters(&read), 0);
        let (pa, pb) = (
            a.user_parameters().next().unwrap(),
            b.user_parameters().next().unwrap(),
        );
        assert_eq!(pa.guid, pb.guid);
        assert_eq!(pa.kind, pb.kind);
        assert!(parse_shared_parameters("parameter \"x\" kind colour\n")
            .unwrap_err()
            .contains("línea 1"));
    }
}
