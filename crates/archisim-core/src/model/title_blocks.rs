//! Title blocks (cajetines) kept in the project (version 74): a layout to
//! start from (Clásico, Hilo, Faro), the rows of data it shows and their
//! labels, the studio's name, monogram and logo, a stamp and an accent
//! colour. Each sheet takes one; without one it takes the project's
//! default, and without that, the layout of the graphic standard.

use super::*;
use crate::graphics::TitleBlockDesign;
use crate::ids::TitleBlockId;

/// What a row of a title block shows.
#[derive(Debug, Clone, PartialEq)]
pub enum TitleValue {
    Project,
    Client,
    Site,
    Author,
    Studio,
    /// The project's number in the studio.
    Number,
    Phase,
    Date,
    SheetName,
    SheetNumber,
    Scale,
    Revision,
    /// A text of its own ("Colegiado nº 1234").
    Text(String),
}

impl TitleValue {
    /// Every kind, a free text last.
    pub const KINDS: [TitleValue; 12] = [
        TitleValue::Project,
        TitleValue::Client,
        TitleValue::Site,
        TitleValue::Author,
        TitleValue::Studio,
        TitleValue::Number,
        TitleValue::Phase,
        TitleValue::Date,
        TitleValue::SheetName,
        TitleValue::SheetNumber,
        TitleValue::Scale,
        TitleValue::Revision,
    ];

    /// Its code in files ("text:…" for a text of its own).
    pub fn code(&self) -> String {
        match self {
            TitleValue::Project => "project".into(),
            TitleValue::Client => "client".into(),
            TitleValue::Site => "site".into(),
            TitleValue::Author => "author".into(),
            TitleValue::Studio => "studio".into(),
            TitleValue::Number => "number".into(),
            TitleValue::Phase => "phase".into(),
            TitleValue::Date => "date".into(),
            TitleValue::SheetName => "sheet".into(),
            TitleValue::SheetNumber => "sheet_number".into(),
            TitleValue::Scale => "scale".into(),
            TitleValue::Revision => "revision".into(),
            TitleValue::Text(t) => format!("text:{t}"),
        }
    }

    pub fn from_code(code: &str) -> Option<TitleValue> {
        if let Some(t) = code.strip_prefix("text:") {
            return Some(TitleValue::Text(t.to_string()));
        }
        TitleValue::KINDS.into_iter().find(|k| k.code() == code)
    }

    /// Its name, to choose it.
    pub fn label(&self) -> &'static str {
        match self {
            TitleValue::Project => "Proyecto",
            TitleValue::Client => "Promotor",
            TitleValue::Site => "Emplazamiento",
            TitleValue::Author => "Autor",
            TitleValue::Studio => "Estudio",
            TitleValue::Number => "Expediente",
            TitleValue::Phase => "Fase",
            TitleValue::Date => "Fecha",
            TitleValue::SheetName => "Nombre del plano",
            TitleValue::SheetNumber => "Número del plano",
            TitleValue::Scale => "Escala",
            TitleValue::Revision => "Revisión",
            TitleValue::Text(_) => "Texto propio",
        }
    }
}

/// A row of a title block: its label and what it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleRow {
    pub label: String,
    pub value: TitleValue,
}

impl TitleRow {
    pub fn new(label: &str, value: TitleValue) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

/// A title block of the project.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleBlock {
    pub id: TitleBlockId,
    pub name: String,
    /// The layout it is drawn in.
    pub design: TitleBlockDesign,
    /// The colour of its labels, stamp and the titles of the views (black
    /// in the classic one).
    pub accent: crate::Rgb,
    /// The studio's name as shown; empty takes the project's.
    pub studio: String,
    /// The letters of the monogram (Hilo); empty takes the studio's first.
    pub monogram: String,
    /// The studio's logo, a PNG, instead of its name.
    pub logo: Option<DecalImage>,
    /// The rows of data, from the top.
    pub rows: Vec<TitleRow>,
    /// A stamp («No válido para obra»), shown while the phase is not
    /// the construction project; empty: no stamp.
    pub stamp: String,
    /// Width of its column or strip, paper millimetres.
    pub width: f64,
}

impl TitleBlock {
    /// A title block of a layout as ArchiSim draws it.
    pub fn standard(design: TitleBlockDesign) -> TitleBlock {
        use TitleValue as V;
        let rows = match design {
            TitleBlockDesign::Classic => vec![
                TitleRow::new("ESCALA", V::Scale),
                TitleRow::new("FECHA", V::Date),
                TitleRow::new("AUTOR", V::Author),
                TitleRow::new("REV.", V::Revision),
                TitleRow::new("Nº PLANO", V::SheetNumber),
            ],
            TitleBlockDesign::Thread => vec![
                TitleRow::new("PROYECTO", V::Project),
                TitleRow::new("PROMOTOR", V::Client),
                TitleRow::new("EMPLAZAMIENTO", V::Site),
                TitleRow::new("FASE", V::Phase),
                TitleRow::new("FECHA", V::Date),
                TitleRow::new("ESCALA", V::Scale),
                TitleRow::new("PLANO", V::SheetName),
            ],
            TitleBlockDesign::Beacon => vec![
                TitleRow::new("FECHA", V::Date),
                TitleRow::new("EXPEDIENTE", V::Number),
                TitleRow::new("FASE", V::Phase),
            ],
        };
        TitleBlock {
            id: TitleBlockId(0),
            name: design.label().into(),
            design,
            accent: match design {
                TitleBlockDesign::Beacon => [214, 72, 44],
                _ => [26, 26, 26],
            },
            studio: String::new(),
            monogram: String::new(),
            logo: None,
            rows,
            stamp: match design {
                TitleBlockDesign::Beacon => "NO VÁLIDO PARA OBRA".into(),
                _ => String::new(),
            },
            width: design.default_width(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.width.is_finite()
            && (30.0..=250.0).contains(&self.width)
            && self.rows.len() <= 16
    }
}

impl Model {
    pub fn title_blocks(&self) -> impl Iterator<Item = &TitleBlock> {
        self.title_blocks.values()
    }

    pub fn title_block(&self, id: TitleBlockId) -> Option<&TitleBlock> {
        self.title_blocks.get(&id)
    }

    /// The title block a sheet is drawn with: its own, the project's
    /// default, or the standard one of the graphic standard's layout.
    pub fn sheet_title_block(&self, sheet: &crate::Sheet) -> std::borrow::Cow<'_, TitleBlock> {
        let found = sheet
            .title_type
            .or(self.default_title_block)
            .and_then(|id| self.title_blocks.get(&id));
        match found {
            Some(t) => std::borrow::Cow::Borrowed(t),
            None => {
                let mut t = TitleBlock::standard(self.graphics.title_block);
                t.accent = self.graphics.accent;
                std::borrow::Cow::Owned(t)
            }
        }
    }

    /// The default title block of the project's sheets.
    pub fn default_title_block(&self) -> Option<TitleBlockId> {
        self.default_title_block
    }

    pub fn set_default_title_block(&mut self, id: Option<TitleBlockId>) -> Result<(), ModelError> {
        if let Some(id) = id {
            if !self.title_blocks.contains_key(&id) {
                return Err(ModelError::UnknownTitleBlock(id));
            }
        }
        self.default_title_block = id;
        Ok(())
    }

    fn check_title_block(&self, t: &TitleBlock) -> Result<(), ModelError> {
        if !t.is_valid() {
            return Err(ModelError::InvalidShape("title block"));
        }
        if self
            .title_blocks
            .values()
            .any(|x| x.id != t.id && x.name == t.name)
        {
            return Err(ModelError::DuplicateName(t.name.clone()));
        }
        Ok(())
    }

    pub fn add_title_block(&mut self, mut t: TitleBlock) -> Result<TitleBlockId, ModelError> {
        t.name = t.name.trim().to_string();
        t.id = TitleBlockId(0);
        self.check_title_block(&t)?;
        t.id = TitleBlockId(self.take_id());
        let id = t.id;
        self.title_blocks.insert(id, t);
        Ok(id)
    }

    pub fn set_title_block(&mut self, mut t: TitleBlock) -> Result<(), ModelError> {
        if !self.title_blocks.contains_key(&t.id) {
            return Err(ModelError::UnknownTitleBlock(t.id));
        }
        t.name = t.name.trim().to_string();
        self.check_title_block(&t)?;
        self.title_blocks.insert(t.id, t);
        Ok(())
    }

    /// Removes a title block; the sheets that had it take the default.
    pub fn remove_title_block(&mut self, id: TitleBlockId) -> Result<TitleBlock, ModelError> {
        let t = self
            .title_blocks
            .remove(&id)
            .ok_or(ModelError::UnknownTitleBlock(id))?;
        for s in self.sheets.values_mut() {
            if s.title_type == Some(id) {
                s.title_type = None;
            }
        }
        if self.default_title_block == Some(id) {
            self.default_title_block = None;
        }
        Ok(t)
    }

    pub(super) fn restore_title_blocks(
        &mut self,
        blocks: Vec<TitleBlock>,
        default: Option<TitleBlockId>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for t in blocks {
            claim(t.id.0, None)
                .and_then(|_| self.check_title_block(&t))
                .map_err(|error| RestoreError {
                    element: format!("title block {}", t.id.0),
                    error,
                })?;
            self.title_blocks.insert(t.id, t);
        }
        self.set_default_title_block(default)
            .map_err(|error| RestoreError {
                element: "default title block".into(),
                error,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheets_take_their_title_block_or_the_default() {
        let mut m = Model::new("t");
        let id = m
            .add_sheet(crate::NewSheet {
                number: "A-01".into(),
                name: "Planta".into(),
                width: 420.0,
                height: 297.0,
            })
            .unwrap();
        let sheet = m.sheet(id).unwrap().clone();
        assert_eq!(
            m.sheet_title_block(&sheet).design,
            TitleBlockDesign::Classic
        );
        let mut faro = TitleBlock::standard(TitleBlockDesign::Beacon);
        faro.name = "Estudio · Faro".into();
        let faro = m.add_title_block(faro).unwrap();
        m.set_default_title_block(Some(faro)).unwrap();
        assert_eq!(m.sheet_title_block(&sheet).design, TitleBlockDesign::Beacon);
        let hilo = m
            .add_title_block(TitleBlock::standard(TitleBlockDesign::Thread))
            .unwrap();
        let mut s = sheet.clone();
        s.title_type = Some(hilo);
        m.set_sheet(s).unwrap();
        let sheet = m.sheet(id).unwrap().clone();
        assert_eq!(m.sheet_title_block(&sheet).design, TitleBlockDesign::Thread);
        // Names are unique; removing one sends its sheets to the default.
        assert!(m
            .add_title_block(TitleBlock::standard(TitleBlockDesign::Thread))
            .is_err());
        m.remove_title_block(hilo).unwrap();
        assert_eq!(m.sheet(id).unwrap().title_type, None);
        assert_eq!(
            TitleValue::from_code("text:Hola"),
            Some(TitleValue::Text("Hola".into()))
        );
        assert_eq!(TitleValue::from_code("scale"), Some(TitleValue::Scale));
    }
}
