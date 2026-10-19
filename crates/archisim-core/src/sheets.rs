//! Sheets ("planos"): paper of a given size with the studio's title block,
//! views of the model placed on it at their scale, and 2D drafting of its
//! own; and the project data the title block shows.
//!
//! Sheet coordinates are paper metres from the lower-left corner of the
//! sheet (1 mm of paper is 0.001), y up, so drafting on a sheet works as in
//! a plan at 1:1.

use crate::graphics::ViewStyle;
use crate::ids::{LevelId, SheetId, TableId, ViewportId};
use crate::math::{Point2, EPSILON};
use crate::model::{Model, ModelError};

/// The project's data, shown in the title block of every sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectInfo {
    /// Client ("Promotor").
    pub client: String,
    /// Site ("Emplazamiento").
    pub site: String,
    /// Architect or author.
    pub author: String,
    /// Studio name.
    pub studio: String,
    /// Project number in the studio.
    pub number: String,
    /// Phase: "Proyecto básico", "Proyecto de ejecución"…
    pub phase: String,
    /// Date shown on the sheets.
    pub date: String,
}

/// Paper sizes (UNE-EN ISO 216), in millimetres, portrait.
pub const PAPER_SIZES: [(&str, f64, f64); 5] = [
    ("A4", 210.0, 297.0),
    ("A3", 297.0, 420.0),
    ("A2", 420.0, 594.0),
    ("A1", 594.0, 841.0),
    ("A0", 841.0, 1189.0),
];

/// A drawing sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub id: SheetId,
    /// Sheet number ("A-01"), unique.
    pub number: String,
    /// Sheet name ("Planta baja. Distribución").
    pub name: String,
    /// Paper width and height, millimetres.
    pub width: f64,
    pub height: f64,
    /// Draws the frame and the title block.
    pub title_block: bool,
    /// Revision shown in the title block ("0", "A"…).
    pub revision: String,
}

impl Sheet {
    /// The paper size's name ("A1 horizontal"), or its millimetres.
    pub fn size_name(&self) -> String {
        for (name, w, h) in PAPER_SIZES {
            if (self.width - w).abs() < 0.5 && (self.height - h).abs() < 0.5 {
                return format!("{name} vertical");
            }
            if (self.width - h).abs() < 0.5 && (self.height - w).abs() < 0.5 {
                return format!("{name} horizontal");
            }
        }
        format!("{:.0} × {:.0} mm", self.width, self.height)
    }
}

/// Data to create a sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSheet {
    pub number: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
}

/// A plan of a level placed on a sheet at a scale.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    pub id: ViewportId,
    pub sheet: SheetId,
    pub level: LevelId,
    /// Scale denominator (50 is 1:50).
    pub scale: f64,
    /// Height of the cut plane above the level, metres.
    pub cut_plane: f64,
    /// The part of the plan shown (model metres, lower-left and
    /// upper-right corners); `None` shows the whole level.
    pub crop: Option<(Point2, Point2)>,
    /// Where the centre of the view is on the sheet (paper metres).
    pub centre: Point2,
    /// What the view shows.
    pub style: ViewStyle,
    /// Title under the view; empty takes the level's name.
    pub title: String,
    pub show_title: bool,
}

/// Data to place a view on a sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct NewViewport {
    pub sheet: SheetId,
    pub level: LevelId,
    pub scale: f64,
    pub cut_plane: f64,
    pub crop: Option<(Point2, Point2)>,
    pub centre: Point2,
}

fn finite(v: f64, what: &'static str) -> Result<(), ModelError> {
    if v.is_finite() {
        Ok(())
    } else {
        Err(ModelError::NotFinite(what))
    }
}

impl Model {
    // ---- Project data ---------------------------------------------------

    pub fn info(&self) -> &ProjectInfo {
        &self.info
    }

    pub fn set_info(&mut self, info: ProjectInfo) {
        self.info = info;
    }

    // ---- Sheets ------------------------------------------------------------

    fn check_sheet(
        &self,
        number: &str,
        width: f64,
        height: f64,
        this: Option<SheetId>,
    ) -> Result<(), ModelError> {
        let number = number.trim();
        if number.is_empty() {
            return Err(ModelError::InvalidShape("sheet number is empty"));
        }
        if self
            .sheets
            .values()
            .any(|s| s.number.eq_ignore_ascii_case(number) && Some(s.id) != this)
        {
            return Err(ModelError::DuplicateName(number.to_string()));
        }
        finite(width, "sheet width")?;
        finite(height, "sheet height")?;
        if width < 50.0 || height < 50.0 || width > 5000.0 || height > 5000.0 {
            return Err(ModelError::InvalidShape(
                "sheet size must be between 50 and 5000 mm",
            ));
        }
        Ok(())
    }

    pub fn add_sheet(&mut self, spec: NewSheet) -> Result<SheetId, ModelError> {
        self.check_sheet(&spec.number, spec.width, spec.height, None)?;
        let id = SheetId(self.take_id());
        self.sheets.insert(
            id,
            Sheet {
                id,
                number: spec.number.trim().to_string(),
                name: spec.name.trim().to_string(),
                width: spec.width,
                height: spec.height,
                title_block: true,
                revision: String::new(),
            },
        );
        Ok(id)
    }

    /// Replaces a sheet's number, name, size, title block and revision.
    pub fn set_sheet(&mut self, sheet: Sheet) -> Result<(), ModelError> {
        if !self.sheets.contains_key(&sheet.id) {
            return Err(ModelError::UnknownSheet(sheet.id));
        }
        self.check_sheet(&sheet.number, sheet.width, sheet.height, Some(sheet.id))?;
        let id = sheet.id;
        self.sheets.insert(
            id,
            Sheet {
                number: sheet.number.trim().to_string(),
                name: sheet.name.trim().to_string(),
                ..sheet
            },
        );
        Ok(())
    }

    /// Removes a sheet with its views and its drafting.
    pub fn remove_sheet(&mut self, id: SheetId) -> Result<Sheet, ModelError> {
        let s = self
            .sheets
            .remove(&id)
            .ok_or(ModelError::UnknownSheet(id))?;
        self.viewports.retain(|_, v| v.sheet != id);
        self.tables.retain(|_, t| t.sheet != id);
        self.drafts
            .retain(|_, d| d.view != crate::drafting::ViewRef::Sheet(id));
        Ok(s)
    }

    pub fn sheet(&self, id: SheetId) -> Option<&Sheet> {
        self.sheets.get(&id)
    }

    /// Every sheet, by number.
    pub fn sheets(&self) -> Vec<&Sheet> {
        let mut v: Vec<&Sheet> = self.sheets.values().collect();
        v.sort_by_key(|a| natural(&a.number));
        v
    }

    /// The next free sheet number after the last one ("A-01" → "A-02").
    pub fn next_sheet_number(&self) -> String {
        let n = self.sheets.len() + 1;
        (n..n + 1000)
            .map(|i| format!("A-{i:02}"))
            .find(|c| {
                !self
                    .sheets
                    .values()
                    .any(|s| s.number.eq_ignore_ascii_case(c))
            })
            .unwrap_or_else(|| "A-99".into())
    }

    // ---- Views on sheets ----------------------------------------------------

    fn check_viewport(&self, v: &NewViewport) -> Result<(), ModelError> {
        if !self.sheets.contains_key(&v.sheet) {
            return Err(ModelError::UnknownSheet(v.sheet));
        }
        if self.level(v.level).is_none() {
            return Err(ModelError::UnknownLevel(v.level));
        }
        finite(v.scale, "view scale")?;
        finite(v.cut_plane, "cut plane")?;
        finite(v.centre.x, "view position")?;
        finite(v.centre.y, "view position")?;
        if v.scale < 1.0 || v.scale > 10000.0 {
            return Err(ModelError::NonPositive("view scale"));
        }
        if v.cut_plane <= 0.0 {
            return Err(ModelError::NonPositive("cut plane"));
        }
        if let Some((a, b)) = v.crop {
            for p in [a, b] {
                finite(p.x, "view crop")?;
                finite(p.y, "view crop")?;
            }
            if b.x - a.x <= EPSILON || b.y - a.y <= EPSILON {
                return Err(ModelError::InvalidShape("view crop has no size"));
            }
        }
        Ok(())
    }

    pub fn add_viewport(&mut self, spec: NewViewport) -> Result<ViewportId, ModelError> {
        self.check_viewport(&spec)?;
        let id = ViewportId(self.take_id());
        self.viewports.insert(
            id,
            Viewport {
                id,
                sheet: spec.sheet,
                level: spec.level,
                scale: spec.scale,
                cut_plane: spec.cut_plane,
                crop: spec.crop,
                centre: spec.centre,
                style: ViewStyle::default(),
                title: String::new(),
                show_title: true,
            },
        );
        Ok(id)
    }

    /// Replaces a view on a sheet (its level, scale, crop, position,
    /// style and title).
    pub fn set_viewport(&mut self, v: Viewport) -> Result<(), ModelError> {
        if !self.viewports.contains_key(&v.id) {
            return Err(ModelError::UnknownViewport(v.id));
        }
        self.check_viewport(&NewViewport {
            sheet: v.sheet,
            level: v.level,
            scale: v.scale,
            cut_plane: v.cut_plane,
            crop: v.crop,
            centre: v.centre,
        })?;
        self.viewports.insert(v.id, v);
        Ok(())
    }

    pub fn remove_viewport(&mut self, id: ViewportId) -> Result<Viewport, ModelError> {
        self.viewports
            .remove(&id)
            .ok_or(ModelError::UnknownViewport(id))
    }

    pub fn viewport(&self, id: ViewportId) -> Option<&Viewport> {
        self.viewports.get(&id)
    }

    pub fn viewports(&self) -> impl Iterator<Item = &Viewport> {
        self.viewports.values()
    }

    pub fn viewports_on(&self, sheet: SheetId) -> impl Iterator<Item = &Viewport> {
        self.viewports.values().filter(move |v| v.sheet == sheet)
    }

    pub(crate) fn restore_sheet(&mut self, s: Sheet) -> Result<(), ModelError> {
        self.check_sheet(&s.number, s.width, s.height, Some(s.id))?;
        self.sheets.insert(s.id, s);
        Ok(())
    }

    pub(crate) fn restore_viewport(&mut self, v: Viewport) -> Result<(), ModelError> {
        self.check_viewport(&NewViewport {
            sheet: v.sheet,
            level: v.level,
            scale: v.scale,
            cut_plane: v.cut_plane,
            crop: v.crop,
            centre: v.centre,
        })?;
        self.viewports.insert(v.id, v);
        Ok(())
    }
}

/// A sort key that orders "A-2" before "A-10".
fn natural(s: &str) -> Vec<(bool, String, u64)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut digits = false;
    for c in s.chars() {
        if c.is_ascii_digit() != digits && !cur.is_empty() {
            out.push(part(&cur, digits));
            cur.clear();
        }
        digits = c.is_ascii_digit();
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(part(&cur, digits));
    }
    out
}

fn part(s: &str, digits: bool) -> (bool, String, u64) {
    if digits {
        (true, String::new(), s.parse().unwrap_or(0))
    } else {
        (false, s.to_lowercase(), 0)
    }
}

/// What a schedule lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScheduleKind {
    /// Doors and windows by type.
    Openings,
    /// Walls by type: length, area, volume.
    Walls,
    /// Slabs by type: area, volume.
    Slabs,
    /// Rooms: areas, heights and finishes ("cuadro de superficies").
    Rooms,
}

impl ScheduleKind {
    pub const ALL: [ScheduleKind; 4] = [
        ScheduleKind::Openings,
        ScheduleKind::Walls,
        ScheduleKind::Slabs,
        ScheduleKind::Rooms,
    ];

    pub fn code(self) -> &'static str {
        match self {
            ScheduleKind::Openings => "openings",
            ScheduleKind::Walls => "walls",
            ScheduleKind::Slabs => "slabs",
            ScheduleKind::Rooms => "rooms",
        }
    }

    pub fn from_code(c: &str) -> Option<ScheduleKind> {
        ScheduleKind::ALL.into_iter().find(|k| k.code() == c)
    }

    pub fn label(self) -> &'static str {
        match self {
            ScheduleKind::Openings => "Carpinterías",
            ScheduleKind::Walls => "Muros",
            ScheduleKind::Slabs => "Forjados",
            ScheduleKind::Rooms => "Cuadro de superficies",
        }
    }
}

/// A schedule placed on a sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetTable {
    pub id: TableId,
    pub sheet: SheetId,
    pub kind: ScheduleKind,
    /// Only this level; `None`: the whole building.
    pub level: Option<LevelId>,
    /// Top-left corner on the paper, metres.
    pub at: Point2,
}

impl Model {
    pub fn add_table(
        &mut self,
        sheet: SheetId,
        kind: ScheduleKind,
        level: Option<LevelId>,
        at: Point2,
    ) -> Result<TableId, ModelError> {
        self.check_table(sheet, level, at)?;
        let id = TableId(self.take_id());
        self.tables.insert(
            id,
            SheetTable {
                id,
                sheet,
                kind,
                level,
                at,
            },
        );
        Ok(id)
    }

    fn check_table(
        &self,
        sheet: SheetId,
        level: Option<LevelId>,
        at: Point2,
    ) -> Result<(), ModelError> {
        if self.sheet(sheet).is_none() {
            return Err(ModelError::UnknownSheet(sheet));
        }
        if let Some(l) = level {
            if self.level(l).is_none() {
                return Err(ModelError::UnknownLevel(l));
            }
        }
        crate::model::finite_point(at, "table position")
    }

    pub fn set_table(&mut self, t: SheetTable) -> Result<(), ModelError> {
        if !self.tables.contains_key(&t.id) {
            return Err(ModelError::UnknownTable(t.id));
        }
        self.check_table(t.sheet, t.level, t.at)?;
        self.tables.insert(t.id, t);
        Ok(())
    }

    pub fn remove_table(&mut self, id: TableId) -> Result<SheetTable, ModelError> {
        self.tables.remove(&id).ok_or(ModelError::UnknownTable(id))
    }

    pub fn table(&self, id: TableId) -> Option<&SheetTable> {
        self.tables.get(&id)
    }

    pub fn tables(&self) -> impl Iterator<Item = &SheetTable> {
        self.tables.values()
    }

    pub fn tables_on(&self, sheet: SheetId) -> impl Iterator<Item = &SheetTable> {
        self.tables.values().filter(move |t| t.sheet == sheet)
    }

    pub(crate) fn restore_table(&mut self, t: SheetTable) -> Result<(), ModelError> {
        self.check_table(t.sheet, t.level, t.at)?;
        self.tables.insert(t.id, t);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drafting::{NewDraft, Shape, ViewRef, LAYER_ZERO};

    #[test]
    fn sheets_with_views_and_drafting() {
        let mut m = Model::new("P");
        let l = m.add_level("PB", 0.0);
        let s = m
            .add_sheet(NewSheet {
                number: "A-01".into(),
                name: "Planta baja".into(),
                width: 594.0,
                height: 420.0,
            })
            .unwrap();
        assert_eq!(m.sheet(s).unwrap().size_name(), "A2 horizontal");
        assert!(m
            .add_sheet(NewSheet {
                number: "a-01".into(),
                name: "".into(),
                width: 594.0,
                height: 420.0,
            })
            .is_err());
        assert_eq!(m.next_sheet_number(), "A-02");
        let v = m
            .add_viewport(NewViewport {
                sheet: s,
                level: l,
                scale: 50.0,
                cut_plane: 1.1,
                crop: None,
                centre: Point2::new(0.2, 0.2),
            })
            .unwrap();
        m.add_draft(NewDraft::new(
            ViewRef::Sheet(s),
            LAYER_ZERO,
            Shape::Point {
                at: Point2::new(0.1, 0.1),
            },
        ))
        .unwrap();
        // Removing the level removes its views; removing the sheet, its
        // drafting.
        let l2 = m.add_level("P1", 3.0);
        let mut vv = m.viewport(v).unwrap().clone();
        vv.level = l2;
        m.set_viewport(vv).unwrap();
        m.remove_level(l2).unwrap();
        assert_eq!(m.viewports().count(), 0);
        m.remove_sheet(s).unwrap();
        assert_eq!(m.drafts().count(), 0);
    }

    #[test]
    fn sheets_sort_naturally() {
        let mut m = Model::new("P");
        for n in ["A-10", "A-2", "A-1"] {
            m.add_sheet(NewSheet {
                number: n.into(),
                name: String::new(),
                width: 420.0,
                height: 297.0,
            })
            .unwrap();
        }
        let order: Vec<&str> = m.sheets().iter().map(|s| s.number.as_str()).collect();
        assert_eq!(order, ["A-1", "A-2", "A-10"]);
    }
}
