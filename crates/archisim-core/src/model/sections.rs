//! Section and elevation lines (Revit's section and elevation views,
//! ArchiCAD's section and elevation markers). They are views, not building
//! elements: they are saved with the project but not written to IFC.
//!
//! A section cuts the building along its line and looks to the **left** of
//! it (walking from `start` to `end`); it sees up to `depth` metres. An
//! elevation is the same, with its line outside the building: it cuts
//! nothing.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Section,
    Elevation,
}

/// A section or elevation line in plan.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionLine {
    pub id: SectionId,
    /// "A", "B"… for sections; "Norte", "Sur"… for elevations.
    pub name: String,
    pub kind: SectionKind,
    pub start: Point2,
    pub end: Point2,
    /// How far it sees beyond its line, in metres.
    pub depth: f64,
    /// Draws its heights: the chain of floor-to-floor heights and, in a
    /// section, the clear height of each room it cuts.
    pub heights: bool,
}

impl SectionLine {
    /// Unit vector along the line.
    pub fn along(&self) -> Option<Vec2> {
        (self.end - self.start).normalized()
    }

    /// The direction it looks in: to the left of the line.
    pub fn looks(&self) -> Option<Vec2> {
        self.along().map(|u| u.perp_left())
    }

    /// The title of its view: «Sección A», «Alzado Norte».
    pub fn title(&self) -> String {
        match self.kind {
            SectionKind::Section => format!("Sección {}", self.name),
            SectionKind::Elevation => format!("Alzado {}", self.name),
        }
    }
}

/// Data needed to create a section line.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSection {
    pub name: String,
    pub kind: SectionKind,
    pub start: Point2,
    pub end: Point2,
    pub depth: f64,
}

impl Model {
    fn validate_section(&self, s: &NewSection) -> Result<(), ModelError> {
        finite_point(s.start, "section start")?;
        finite_point(s.end, "section end")?;
        finite(s.depth, "section view depth")?;
        if s.start.approx_eq(s.end) {
            return Err(ModelError::ZeroLengthSection);
        }
        if s.depth <= EPSILON {
            return Err(ModelError::NonPositive("section view depth"));
        }
        Ok(())
    }

    pub fn add_section(&mut self, spec: NewSection) -> Result<SectionId, ModelError> {
        self.validate_section(&spec)?;
        let id = SectionId(self.take_id());
        self.sections.insert(
            id,
            SectionLine {
                id,
                name: spec.name,
                kind: spec.kind,
                start: spec.start,
                end: spec.end,
                depth: spec.depth,
                heights: true,
            },
        );
        Ok(id)
    }

    pub fn edit_section(
        &mut self,
        id: SectionId,
        edit: impl FnOnce(&mut SectionLine),
    ) -> Result<(), ModelError> {
        let mut s = self
            .sections
            .get(&id)
            .cloned()
            .ok_or(ModelError::UnknownSection(id))?;
        edit(&mut s);
        self.validate_section(&NewSection {
            name: s.name.clone(),
            kind: s.kind,
            start: s.start,
            end: s.end,
            depth: s.depth,
        })?;
        self.sections.insert(id, s);
        Ok(())
    }

    /// Removes a section line and the views of it placed on sheets.
    pub fn remove_section(&mut self, id: SectionId) -> Result<SectionLine, ModelError> {
        let s = self
            .sections
            .remove(&id)
            .ok_or(ModelError::UnknownSection(id))?;
        self.viewports.retain(|_, v| v.section != Some(id));
        self.drop_underlays_of(UnderlaySource::Section(id));
        Ok(s)
    }

    pub fn section(&self, id: SectionId) -> Option<&SectionLine> {
        self.sections.get(&id)
    }

    pub fn sections(&self) -> impl Iterator<Item = &SectionLine> {
        self.sections.values()
    }

    /// The next free letter for a section: A, B… Z, then AA, AB…
    pub fn next_section_name(&self) -> String {
        let taken: Vec<&str> = self
            .sections
            .values()
            .filter(|s| s.kind == SectionKind::Section)
            .map(|s| s.name.as_str())
            .collect();
        let name = |mut n: usize| {
            let mut s = String::new();
            loop {
                s.insert(0, (b'A' + (n % 26) as u8) as char);
                if n < 26 {
                    break;
                }
                n = n / 26 - 1;
            }
            s
        };
        (0..)
            .map(name)
            .find(|n| !taken.contains(&n.as_str()))
            .unwrap_or_default()
    }

    pub(super) fn restore_sections(
        &mut self,
        sections: Vec<SectionLine>,
        claim: &mut structure::Claim<'_>,
    ) -> Result<(), RestoreError> {
        for s in sections {
            let spec = NewSection {
                name: s.name.clone(),
                kind: s.kind,
                start: s.start,
                end: s.end,
                depth: s.depth,
            };
            claim(s.id.0, None)
                .and_then(|_| self.validate_section(&spec))
                .map_err(|error| RestoreError {
                    element: format!("section {} «{}»", s.id.0, s.name),
                    error,
                })?;
            self.sections.insert(s.id, s);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_directions() {
        let mut m = Model::new("t");
        let spec = |name: &str| NewSection {
            name: name.into(),
            kind: SectionKind::Section,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(10.0, 0.0),
            depth: 20.0,
        };
        assert_eq!(m.next_section_name(), "A");
        let a = m.add_section(spec("A")).unwrap();
        m.add_section(spec("C")).unwrap();
        assert_eq!(m.next_section_name(), "B");
        let s = m.section(a).unwrap();
        // Drawn west → east, it looks north.
        assert_eq!(s.looks(), Some(Point2::new(0.0, 1.0)));
        assert_eq!(s.title(), "Sección A");
        let mut bad = spec("D");
        bad.end = bad.start;
        assert_eq!(m.add_section(bad), Err(ModelError::ZeroLengthSection));
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back, m);
    }
}
