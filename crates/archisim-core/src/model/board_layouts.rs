//! How a material laid in boards is cut and laid (the board layout of the
//! studio, `docs/MODULAR.md`, M18): its commercial size, whether it stands
//! upright, and how its joints run — at half a board, in a «zipper» (the
//! cut piece of each column at the top and of the next at the bottom, so
//! no horizontal joint meets another), or along fixed lines over the level
//! (a cladding with a joint at the doors' head) — and, for a cladding, the
//! accent the pieces round the doors and windows take (a wood-look
//! format). Kept per material: wherever the material is a layer, it is
//! laid so. Without a layout, the usual format by its name.

use super::*;

/// How the joints of a board layout run.
#[derive(Debug, Clone, PartialEq)]
pub enum BoardJoints {
    /// Each row half a board off the last.
    HalfBond,
    /// Upright boards in columns; the cut piece at the top of one column
    /// and at the bottom of the next.
    Zipper,
    /// Columns of boards between fixed horizontal joints, at these heights
    /// over the level (m), and the face's bottom and top.
    Lines(Vec<f64>),
}

impl BoardJoints {
    pub fn code(&self) -> &'static str {
        match self {
            BoardJoints::HalfBond => "half",
            BoardJoints::Zipper => "zipper",
            BoardJoints::Lines(_) => "lines",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BoardJoints::HalfBond => "A media placa",
            BoardJoints::Zipper => "En cremallera",
            BoardJoints::Lines(_) => "Por líneas de junta",
        }
    }
}

/// The pieces round the doors and windows in another material.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardAccent {
    pub material: MaterialId,
    /// The pieces of the band from the lowest joint line up to this
    /// height over the level (m) whose column a door or window crosses.
    pub up_to: f64,
    /// The width of the frame each side of a door or window (m): a column
    /// of that width beside each jamb takes the accent too, so the opening
    /// is framed. Zero: only the columns the opening crosses.
    pub side: f64,
}

/// A board layout.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardLayout {
    /// Width and length of the board, m.
    pub width: f64,
    pub length: f64,
    /// Upright on walls (length up); across the slab's long side on floors.
    pub upright: bool,
    pub joints: BoardJoints,
    pub accent: Option<BoardAccent>,
}

impl BoardLayout {
    pub fn is_valid(&self) -> bool {
        let ok = |v: f64| v.is_finite() && (0.05..=6.0).contains(&v);
        ok(self.width)
            && ok(self.length)
            && match &self.joints {
                BoardJoints::Lines(v) => v.iter().all(|h| h.is_finite()),
                _ => true,
            }
            && self
                .accent
                .as_ref()
                .is_none_or(|a| a.up_to.is_finite() && (0.0..=1.2).contains(&a.side))
    }
}

impl Model {
    /// The board layout of a material, if the studio set one.
    pub fn board_layout(&self, material: MaterialId) -> Option<&BoardLayout> {
        self.board_layouts.get(&material)
    }

    pub fn board_layouts(&self) -> impl Iterator<Item = (MaterialId, &BoardLayout)> {
        self.board_layouts.iter().map(|(m, l)| (*m, l))
    }

    /// Sets (or, with `None`, takes away) a material's board layout.
    pub fn set_board_layout(
        &mut self,
        material: MaterialId,
        layout: Option<BoardLayout>,
    ) -> Result<(), ModelError> {
        if !self.materials.contains_key(&material) {
            return Err(ModelError::UnknownMaterial(material));
        }
        match layout {
            None => {
                self.board_layouts.remove(&material);
            }
            Some(l) => {
                if !l.is_valid() {
                    return Err(ModelError::InvalidShape("a board layout out of range"));
                }
                if let Some(a) = &l.accent {
                    if !self.materials.contains_key(&a.material) {
                        return Err(ModelError::UnknownMaterial(a.material));
                    }
                }
                self.board_layouts.insert(material, l);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layout_belongs_to_a_known_material() {
        let mut m = Model::new("T");
        let a = m.add_material("Panel", "insulation", [200, 200, 200], CutPattern::Solid);
        let l = BoardLayout {
            width: 1.2,
            length: 3.0,
            upright: true,
            joints: BoardJoints::Zipper,
            accent: None,
        };
        m.set_board_layout(a, Some(l.clone())).unwrap();
        assert_eq!(m.board_layout(a), Some(&l));
        assert!(m
            .set_board_layout(MaterialId(999), Some(l.clone()))
            .is_err());
        let bad = BoardLayout { width: 0.0, ..l };
        assert!(m.set_board_layout(a, Some(bad)).is_err());
        m.set_board_layout(a, None).unwrap();
        assert!(m.board_layout(a).is_none());
    }
}
