//! The view templates of the library (the file «Plantillas de
//! vista.asplantillas»): JSON like a project's, with what refers to a
//! project — the 2D layers hidden and the presentation style — by name or
//! whole, so they load in any project (`Model::load_saved_template`).

use archisim_core as core;
use serde::{Deserialize, Serialize};

use crate::schema::PresentationStyleOf;
use crate::views::ViewTemplateOf;
use crate::{FileError, CURRENT_VERSION};

const FORMAT: &str = "ArchiSim plantillas";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplatesFile {
    format: String,
    version: u32,
    templates: Vec<SavedTemplateOf>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedTemplateOf {
    template: ViewTemplateOf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_layers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    presentation: Option<PresentationStyleOf>,
}

/// The library file with `templates`.
pub fn templates_to_string(templates: &[core::SavedTemplate]) -> String {
    let file = TemplatesFile {
        format: FORMAT.into(),
        version: CURRENT_VERSION,
        templates: templates
            .iter()
            .map(|t| SavedTemplateOf {
                template: ViewTemplateOf::out(&t.template),
                hidden_layers: t.hidden_layers.clone(),
                presentation: t.presentation.as_ref().map(PresentationStyleOf::out),
            })
            .collect(),
    };
    let mut s = serde_json::to_string_pretty(&file).expect("templates serialize");
    s.push('\n');
    s
}

/// The templates of a library file.
pub fn templates_from_str(text: &str) -> Result<Vec<core::SavedTemplate>, FileError> {
    let probe: serde_json::Value = serde_json::from_str(text).map_err(FileError::Json)?;
    if probe.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(FileError::NotAProject);
    }
    let version = probe.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
    if version > u64::from(CURRENT_VERSION) {
        return Err(FileError::TooNew {
            version: version as u32,
        });
    }
    let file: TemplatesFile = serde_json::from_value(probe).map_err(FileError::Json)?;
    file.templates
        .into_iter()
        .map(|t| {
            Ok(core::SavedTemplate {
                template: t.template.into_core()?,
                hidden_layers: t.hidden_layers,
                presentation: t.presentation.map(|p| p.into_core()).transpose()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer_named(name: &str) -> core::DrawLayer {
        core::DrawLayer::new(name, [0, 0, 0], 0.25, core::LineStyle::Continuous)
    }

    #[test]
    fn templates_go_to_the_library_and_back() {
        let mut m = core::Model::new("Origen");
        let p = m
            .add_presentation_style(core::PresentationStyle::new("Tinta"))
            .unwrap();
        let layer = m.add_draw_layer(layer_named("Cotas de obra")).unwrap();
        let mut style = core::ViewStyle::default();
        style.hidden.insert(core::Category::Objects);
        style.hidden_layers.insert(layer);
        style.presentation = Some(p);
        let t = m
            .add_view_template(core::ViewTemplate {
                id: core::ViewTemplateId(0),
                name: "Albañilería 1:50".into(),
                style,
                scale: Some(50.0),
                cut_plane: None,
            })
            .unwrap();
        let text = templates_to_string(&[m.saved_template(t).unwrap()]);
        let back = templates_from_str(&text).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].hidden_layers, vec!["Cotas de obra".to_string()]);
        assert_eq!(back[0].template.scale, Some(50.0));
        assert_eq!(back[0].template.cut_plane, None);

        // Loaded in another project: the style comes along, the layer is
        // matched by name, a second load replaces the first.
        let mut other = core::Model::new("Destino");
        let l2 = other.add_draw_layer(layer_named("Cotas de obra")).unwrap();
        let id = other.load_saved_template(&back[0]).unwrap();
        let got = other.view_template(id).unwrap();
        assert!(got.style.hidden.contains(&core::Category::Objects));
        assert_eq!(
            got.style.hidden_layers.iter().copied().collect::<Vec<_>>(),
            vec![l2]
        );
        let style = got.style.presentation.unwrap();
        assert_eq!(other.presentation_style(style).unwrap().name, "Tinta");
        assert_eq!(other.load_saved_template(&back[0]).unwrap(), id);
        assert_eq!(other.view_templates().count(), 1);
        assert_eq!(other.presentation_styles().count(), 1);

        assert!(matches!(
            templates_from_str(&text.replace("ArchiSim plantillas", "ArchiSim")),
            Err(FileError::NotAProject)
        ));
    }
}
