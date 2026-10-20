//! Parameters of the project (Revit's project and shared parameters,
//! ArchiCAD's property manager): the window where they are defined, and
//! the group of the properties palette where each element's values are
//! given.

use archisim_core::{
    parse_shared_parameters, shared_parameters_text, NewUserParameter, ParamCategory, UserKind,
    UserParameterId, Value,
};
use eframe::egui::{self, Ui};

use super::props::{group, row};
use super::*;
use crate::pick::Element;

/// The state of the parameters window.
#[derive(Default)]
pub struct ParamsWindow {
    pub open: bool,
    /// The parameter being edited (`None`: a new one).
    selected: Option<UserParameterId>,
    draft: Option<Draft>,
}

/// A parameter as the form edits it.
#[derive(Clone)]
struct Draft {
    name: String,
    kind: usize,
    options: String,
    group: String,
    categories: Vec<ParamCategory>,
    by_type: bool,
    default: String,
    description: String,
}

const KINDS: [&str; 7] = [
    "Texto",
    "Número",
    "Entero",
    "Longitud",
    "Superficie",
    "Sí/no",
    "Lista de opciones",
];

fn kind_index(k: &UserKind) -> usize {
    match k {
        UserKind::Text => 0,
        UserKind::Number => 1,
        UserKind::Integer => 2,
        UserKind::Length => 3,
        UserKind::Area => 4,
        UserKind::Bool => 5,
        UserKind::Choice(_) => 6,
    }
}

fn value_text(v: &Value) -> String {
    match v {
        Value::Text(t) => t.clone(),
        Value::Bool(true) => "sí".into(),
        Value::Bool(false) => "no".into(),
        Value::Number(n) => format!("{n}").replace('.', ","),
        Value::Material(m) => m.to_string(),
    }
}

impl Draft {
    fn empty() -> Self {
        Draft {
            name: String::new(),
            kind: 0,
            options: String::new(),
            group: "AD5_Datos".into(),
            categories: vec![ParamCategory::Doors],
            by_type: true,
            default: String::new(),
            description: String::new(),
        }
    }

    fn of(p: &archisim_core::UserParameter) -> Self {
        Draft {
            name: p.name.clone(),
            kind: kind_index(&p.kind),
            options: match &p.kind {
                UserKind::Choice(o) => o.join(", "),
                _ => String::new(),
            },
            group: p.group.clone(),
            categories: p.categories.clone(),
            by_type: p.by_type,
            default: value_text(&p.default),
            description: p.description.clone(),
        }
    }

    /// The definition, or why it is not valid.
    fn build(&self) -> Result<NewUserParameter, String> {
        let kind = match self.kind {
            0 => UserKind::Text,
            1 => UserKind::Number,
            2 => UserKind::Integer,
            3 => UserKind::Length,
            4 => UserKind::Area,
            5 => UserKind::Bool,
            _ => UserKind::Choice(
                self.options
                    .split(',')
                    .map(|o| o.trim().to_string())
                    .filter(|o| !o.is_empty())
                    .collect(),
            ),
        };
        let d = self.default.trim();
        let default = match &kind {
            UserKind::Text => Value::Text(d.to_string()),
            UserKind::Bool => Value::Bool(matches!(
                d.to_lowercase().as_str(),
                "sí" | "si" | "yes" | "1"
            )),
            UserKind::Choice(o) => Value::Text(if d.is_empty() {
                o.first().cloned().unwrap_or_default()
            } else {
                d.to_string()
            }),
            UserKind::Length => {
                if d.is_empty() {
                    Value::Number(0.0)
                } else {
                    // Typed in millimetres, as every length.
                    Value::Number(
                        crate::units::parse_length(d)
                            .ok_or("el valor por defecto no es una medida")?,
                    )
                }
            }
            _ => Value::Number(if d.is_empty() {
                0.0
            } else {
                d.replace(',', ".")
                    .parse()
                    .map_err(|_| "el valor por defecto no es un número")?
            }),
        };
        Ok(NewUserParameter {
            name: self.name.clone(),
            kind,
            group: self.group.clone(),
            categories: self.categories.clone(),
            by_type: self.by_type,
            default,
            description: self.description.clone(),
        })
    }
}

/// The id of an element as the model keeps parameter values.
pub(super) fn raw_id(e: Element) -> Option<u64> {
    Some(match e {
        Element::Wall(i) => i.0,
        Element::Slab(i) => i.0,
        Element::Opening(i) => i.0,
        Element::Column(i) => i.0,
        Element::Beam(i) => i.0,
        Element::Roof(i) => i.0,
        Element::Stair(i) => i.0,
        Element::CurtainWall(i) => i.0,
        Element::Railing(i) => i.0,
        Element::Object(i) => i.0,
        Element::Room(i) => i.0,
        _ => return None,
    })
}

/// Where the shared parameters of the studio are: in the library folder.
fn shared_file() -> std::path::PathBuf {
    crate::editor::library::library_dir().join("Parámetros.asparam")
}

impl ArchiSimApp {
    pub(super) fn params_dialog(&mut self, ctx: &egui::Context) {
        if !self.params.open {
            return;
        }
        let mut open = true;
        egui::Window::new("Parámetros del proyecto")
            .open(&mut open)
            .default_width(720.0)
            .default_height(520.0)
            .show(ctx, |ui| self.params_contents(ui));
        self.params.open = open;
    }

    fn params_contents(&mut self, ui: &mut Ui) {
        ui.label(
            egui::RichText::new(
                "Datos propios de los elementos (resistencia al fuego, acabados, códigos de medición…): salen en Propiedades, en las tablas y en el IFC, cada uno en su conjunto de propiedades. Los compartidos llevan su identidad a otros proyectos.",
            )
            .weak(),
        );
        ui.separator();
        let params: Vec<archisim_core::UserParameter> =
            self.ed.model().user_parameters().cloned().collect();
        let mut pick: Option<Option<UserParameterId>> = None;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(240.0);
                if params.is_empty() {
                    ui.label(egui::RichText::new("El proyecto no tiene parámetros.").weak());
                }
                for p in &params {
                    let on = self.params.selected == Some(p.id);
                    let cats: Vec<&str> = p.categories.iter().map(|c| c.label()).collect();
                    let r = ui
                        .selectable_label(
                            on,
                            format!("{}\n{} · {}", p.name, p.group, cats.join(", ")),
                        )
                        .on_hover_text(&p.description);
                    if r.clicked() {
                        pick = Some(Some(p.id));
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("＋ Nuevo").clicked() {
                        pick = Some(None);
                    }
                });
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Compartidos (biblioteca)").strong());
                if ui
                    .button("Cargar los compartidos…")
                    .on_hover_text(format!("Añadir los de {}", shared_file().display()))
                    .clicked()
                {
                    self.load_shared_params();
                }
                if ui
                    .button("Guardar como compartidos")
                    .on_hover_text(format!(
                        "Escribir los del proyecto en {}",
                        shared_file().display()
                    ))
                    .clicked()
                {
                    self.save_shared_params();
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                if self.params.draft.is_none() {
                    ui.label(egui::RichText::new("Elige un parámetro o crea uno nuevo.").weak());
                    return;
                }
                self.param_form(ui);
            });
        });
        if let Some(sel) = pick {
            self.params.selected = sel;
            self.params.draft = Some(
                match sel.and_then(|id| self.ed.model().user_parameter(id)) {
                    Some(p) => Draft::of(p),
                    None => Draft::empty(),
                },
            );
        }
    }

    fn param_form(&mut self, ui: &mut Ui) {
        let Some(mut d) = self.params.draft.clone() else {
            return;
        };
        let mut apply = false;
        let mut remove = false;
        egui::Grid::new("param-form").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Nombre");
            ui.text_edit_singleline(&mut d.name);
            ui.end_row();
            ui.label("Clase");
            egui::ComboBox::from_id_salt("param-kind")
                .selected_text(KINDS[d.kind])
                .show_ui(ui, |ui| {
                    for (i, k) in KINDS.iter().enumerate() {
                        ui.selectable_value(&mut d.kind, i, *k);
                    }
                });
            ui.end_row();
            if d.kind == 6 {
                ui.label("Opciones");
                ui.text_edit_singleline(&mut d.options)
                    .on_hover_text("Separadas por comas: «—, EI2 30-C5, EI2 60-C5»");
                ui.end_row();
            }
            ui.label("Grupo (conjunto IFC)").on_hover_text(
                "El conjunto de propiedades del IFC donde va, y el grupo en Propiedades: «AD5_Datos», o uno de IFC como «Pset_DoorCommon»",
            );
            ui.text_edit_singleline(&mut d.group);
            ui.end_row();
            ui.label("Del tipo").on_hover_text(
                "Del tipo: todos los elementos del tipo tienen el mismo valor (se da en Tipos y materiales). Si no, cada elemento el suyo.",
            );
            ui.checkbox(&mut d.by_type, "");
            ui.end_row();
            ui.label("Valor por defecto");
            ui.text_edit_singleline(&mut d.default).on_hover_text(
                "Las longitudes en milímetros; sí/no se escribe «sí» o «no»",
            );
            ui.end_row();
            ui.label("Descripción");
            ui.text_edit_singleline(&mut d.description);
            ui.end_row();
        });
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Categorías").strong());
        ui.horizontal_wrapped(|ui| {
            for c in ParamCategory::ALL {
                let mut on = d.categories.contains(&c);
                if ui.checkbox(&mut on, c.label()).changed() {
                    if on {
                        d.categories.push(c);
                    } else {
                        d.categories.retain(|x| *x != c);
                    }
                }
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let label = if self.params.selected.is_some() {
                "Aplicar"
            } else {
                "Crear"
            };
            if ui.button(label).clicked() {
                apply = true;
            }
            if self.params.selected.is_some()
                && ui
                    .button("🗑 Borrar")
                    .on_hover_text("Con todos sus valores")
                    .clicked()
            {
                remove = true;
            }
        });
        self.params.draft = Some(d.clone());
        if apply {
            match d.build() {
                Err(e) => self.complain(format!("No se puede: {e}.")),
                Ok(def) => match self.params.selected {
                    Some(id) => {
                        let r = self
                            .ed
                            .transact("Modificar parámetro", |m| m.edit_user_parameter(id, def));
                        self.report(r, "Parámetro modificado.");
                    }
                    None => match self
                        .ed
                        .transact("Nuevo parámetro", |m| m.add_user_parameter(def))
                    {
                        Ok(id) => {
                            self.params.selected = Some(id);
                            self.say("Parámetro creado: sale en Propiedades de sus elementos, en las tablas y en el IFC.");
                        }
                        Err(e) => self.complain(format!("No se puede: {e}.")),
                    },
                },
            }
        }
        if remove {
            if let Some(id) = self.params.selected {
                let r = self.ed.transact("Borrar parámetro", |m| {
                    m.remove_user_parameter(id).map(|_| ())
                });
                self.report(r, "Parámetro borrado.");
                self.params.selected = None;
                self.params.draft = None;
            }
        }
    }

    fn load_shared_params(&mut self) {
        let path = shared_file();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                self.complain(format!(
                    "No hay parámetros compartidos en {} todavía: guárdalos desde un proyecto que los tenga.",
                    path.display()
                ));
                return;
            }
        };
        match parse_shared_parameters(&text) {
            Err(e) => self.complain(format!("{}: {e}", path.display())),
            Ok(defs) => match self.ed.transact("Cargar parámetros compartidos", |m| {
                Ok(m.load_shared_parameters(&defs))
            }) {
                Ok(n) => self.say(format!("{n} parámetros compartidos añadidos al proyecto.")),
                Err(e) => self.complain(format!("No se puede: {e}.")),
            },
        }
    }

    fn save_shared_params(&mut self) {
        let path = shared_file();
        // The file keeps the studio's: the project's are added or replace
        // those of the same identity.
        let mut all = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| parse_shared_parameters(&t).ok())
            .unwrap_or_default();
        for p in self.ed.model().shared_parameters() {
            match all.iter_mut().find(|x| x.guid == p.guid) {
                Some(x) => *x = p,
                None => all.push(p),
            }
        }
        let n = all.len();
        let result = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, shared_parameters_text(&all)));
        match result {
            Ok(()) => self.say(format!("{n} parámetros compartidos en {}.", path.display())),
            Err(e) => self.complain(format!("No se puede escribir {}: {e}.", path.display())),
        }
    }

    /// The group of the properties palette with the project's parameters
    /// of the selected element: its own, and those of its type (given in
    /// Tipos y materiales, shown here).
    pub(super) fn user_params_group(&mut self, ui: &mut Ui, e: Element) {
        let Some(id) = raw_id(e) else { return };
        let params: Vec<(archisim_core::UserParameter, Value)> = self
            .ed
            .model()
            .element_parameters(id)
            .into_iter()
            .map(|(p, v)| (p.clone(), v))
            .collect();
        if params.is_empty() {
            return;
        }
        let ty = self.ed.model().type_of_element(id);
        let mut change: Option<(u64, UserParameterId, Value)> = None;
        group(ui, "user-params", "Parámetros del proyecto", |ui| {
            for (p, v) in &params {
                let target = if p.by_type { ty } else { Some(id) };
                let label = if p.by_type {
                    format!("{} (tipo)", p.name)
                } else {
                    p.name.clone()
                };
                let hint = if p.by_type {
                    format!(
                        "{} · del tipo: cambia en todos los elementos del tipo",
                        p.description
                    )
                } else {
                    p.description.clone()
                };
                if let (Some(new), Some(t)) = (param_value_row(ui, id, p, v, &label, &hint), target)
                {
                    change = Some((t, p.id, new));
                }
            }
        });
        if let Some((t, p, v)) = change {
            let r = self
                .ed
                .transact("Cambiar parámetro", |m| m.set_parameter_value(t, p, v));
            self.report(r, "Parámetro cambiado.");
        }
    }

    /// The same group for a type, in Tipos y materiales.
    pub(super) fn type_params_group(&mut self, ui: &mut Ui, type_id: u64) {
        let params: Vec<(archisim_core::UserParameter, Value)> = self
            .ed
            .model()
            .type_parameters(type_id)
            .into_iter()
            .map(|(p, v)| (p.clone(), v))
            .collect();
        if params.is_empty() {
            return;
        }
        let mut change: Option<(UserParameterId, Value)> = None;
        group(ui, "type-user-params", "Parámetros del proyecto", |ui| {
            for (p, v) in &params {
                if let Some(new) = param_value_row(ui, type_id, p, v, &p.name, &p.description) {
                    change = Some((p.id, new));
                }
            }
        });
        if let Some((p, v)) = change {
            let r = self.ed.transact("Cambiar parámetro del tipo", |m| {
                m.set_parameter_value(type_id, p, v)
            });
            self.report(r, "Parámetro del tipo cambiado.");
        }
    }
}

/// A row with the field of one parameter value; the value chosen, if it
/// changed.
fn param_value_row(
    ui: &mut Ui,
    owner: u64,
    p: &archisim_core::UserParameter,
    v: &Value,
    label: &str,
    hint: &str,
) -> Option<Value> {
    let field = egui::Id::unique(("user-param", owner, p.id.0));
    let label = match p.kind {
        UserKind::Length => format!("{label} (mm)"),
        UserKind::Area => format!("{label} (m²)"),
        _ => label.to_string(),
    };
    row(ui, &label, hint, |ui, w| match (&p.kind, v) {
        (UserKind::Choice(options), Value::Text(chosen)) => {
            let mut c = chosen.clone();
            egui::ComboBox::from_id_salt(field)
                .width(w - 8.0)
                .selected_text(c.clone())
                .show_ui(ui, |ui| {
                    for o in options {
                        ui.selectable_value(&mut c, o.clone(), o);
                    }
                });
            (c != *chosen).then_some(Value::Text(c))
        }
        (UserKind::Bool, Value::Bool(b)) => {
            let mut on = *b;
            ui.checkbox(&mut on, "")
                .changed()
                .then_some(Value::Bool(on))
        }
        (UserKind::Length, Value::Number(n)) => {
            length_field(ui, field, *n, w - 8.0).map(Value::Number)
        }
        (UserKind::Text, Value::Text(t)) => {
            text_edit_committed(ui, field, t, w - 8.0).map(Value::Text)
        }
        (_, Value::Number(n)) => {
            let shown = format!("{n}").replace('.', ",");
            text_edit_committed(ui, field, &shown, w - 8.0)
                .and_then(|t| t.trim().replace(',', ".").parse::<f64>().ok())
                .filter(|x| p.kind != UserKind::Integer || x.fract() == 0.0)
                .map(Value::Number)
        }
        _ => {
            ui.label("—");
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_build_definitions() {
        let mut d = Draft::empty();
        d.name = "Resistencia al fuego".into();
        d.kind = 6;
        d.options = "—, EI2 30-C5 ,EI2 60-C5".into();
        let def = d.build().unwrap();
        assert_eq!(
            def.kind,
            UserKind::Choice(vec!["—".into(), "EI2 30-C5".into(), "EI2 60-C5".into()])
        );
        assert_eq!(def.default, Value::Text("—".into()));
        d.kind = 3;
        d.default = "2100".into();
        assert_eq!(d.build().unwrap().default, Value::Number(2.1));
        d.kind = 1;
        d.default = "dos".into();
        assert!(d.build().is_err());
    }
}
