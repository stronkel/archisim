//! Façades in full detail (E9): the bond of the exterior finish of a wall
//! type — face brick in stretcher, header, English, Flemish or open
//! (palomero) bond, blocks, precast panels, ventilated façade panels and
//! stone cladding — and the piece over the edges of the slabs it passes
//! («frente de forjado», the slab front).
//!
//! The pieces are not kept: they are laid by rule on the face of each
//! wall whenever they are needed ([`Coursing::course`] gives the pieces of
//! one course; `archisim_geom::facade` lays them on the face, round the
//! doors and windows and the corners).

use super::*;

/// What a façade is built of: it chooses the usual size of its pieces
/// and the names the dialogs use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FacadeSystem {
    /// Face brick (ladrillo cara vista).
    #[default]
    Brick,
    /// Concrete blocks left exposed.
    Block,
    /// Precast concrete panels, a module repeated.
    Panel,
    /// Ventilated façade: panels on a frame, with open joints.
    Ventilated,
    /// Stone or ceramic slabs fixed to the wall (aplacado).
    Cladding,
}

impl FacadeSystem {
    pub const ALL: [FacadeSystem; 5] = [
        FacadeSystem::Brick,
        FacadeSystem::Block,
        FacadeSystem::Panel,
        FacadeSystem::Ventilated,
        FacadeSystem::Cladding,
    ];

    /// Its name in the dialogs.
    pub fn name(self) -> &'static str {
        match self {
            FacadeSystem::Brick => "Ladrillo cara vista",
            FacadeSystem::Block => "Bloque de hormigón",
            FacadeSystem::Panel => "Panel prefabricado",
            FacadeSystem::Ventilated => "Fachada ventilada",
            FacadeSystem::Cladding => "Aplacado",
        }
    }

    /// Its name in files.
    pub fn key(self) -> &'static str {
        match self {
            FacadeSystem::Brick => "brick",
            FacadeSystem::Block => "block",
            FacadeSystem::Panel => "panel",
            FacadeSystem::Ventilated => "ventilated",
            FacadeSystem::Cladding => "cladding",
        }
    }

    pub fn from_key(s: &str) -> Option<FacadeSystem> {
        FacadeSystem::ALL.into_iter().find(|f| f.key() == s)
    }

    /// The usual piece and bond of the system.
    pub fn coursing(self) -> Coursing {
        let base = Coursing {
            system: self,
            length: 0.24,
            height: 0.0525,
            depth: 0.115,
            joint: 0.01,
            bond: Bond::Stretcher,
            joint_material: None,
            accent: None,
            pattern: Pattern::Plain,
            soldier_heads: false,
        };
        match self {
            FacadeSystem::Brick => Coursing {
                soldier_heads: true,
                ..base
            },
            FacadeSystem::Block => Coursing {
                length: 0.39,
                height: 0.19,
                depth: 0.19,
                ..base
            },
            FacadeSystem::Panel => Coursing {
                length: 2.4,
                height: 1.2,
                depth: 0.12,
                joint: 0.02,
                bond: Bond::Stack,
                ..base
            },
            FacadeSystem::Ventilated => Coursing {
                length: 1.2,
                height: 0.6,
                depth: 0.012,
                joint: 0.008,
                bond: Bond::Stack,
                ..base
            },
            FacadeSystem::Cladding => Coursing {
                length: 0.6,
                height: 0.4,
                depth: 0.03,
                joint: 0.005,
                ..base
            },
        }
    }
}

/// How the pieces of one course stand over those of the course below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Bond {
    /// A soga: stretchers, each course half a piece along.
    #[default]
    Stretcher,
    /// A tizón: headers, each course half a header along.
    Header,
    /// Inglés: a course of stretchers, a course of headers.
    English,
    /// Flamenco: stretcher and header in every course, each header over the
    /// middle of a stretcher.
    Flemish,
    /// Palomero: stretchers with a gap between them (a lattice).
    Lattice,
    /// Junta continua: every joint over the one below (panels, slabs).
    Stack,
}

impl Bond {
    pub const ALL: [Bond; 6] = [
        Bond::Stretcher,
        Bond::Header,
        Bond::English,
        Bond::Flemish,
        Bond::Lattice,
        Bond::Stack,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Bond::Stretcher => "A soga",
            Bond::Header => "A tizón",
            Bond::English => "Inglés",
            Bond::Flemish => "Flamenco",
            Bond::Lattice => "Palomero",
            Bond::Stack => "Junta continua",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Bond::Stretcher => "stretcher",
            Bond::Header => "header",
            Bond::English => "english",
            Bond::Flemish => "flemish",
            Bond::Lattice => "lattice",
            Bond::Stack => "stack",
        }
    }

    pub fn from_key(s: &str) -> Option<Bond> {
        Bond::ALL.into_iter().find(|b| b.key() == s)
    }
}

/// Which pieces take the second material (the «dibujos» of a brick
/// façade).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pattern {
    /// All alike.
    #[default]
    Plain,
    /// The headers (English and Flemish bonds).
    Headers,
    /// One course in every so many.
    Courses(u8),
    /// Diamonds of so many pieces.
    Diamonds(u8),
}

impl Pattern {
    pub fn name(self) -> &'static str {
        match self {
            Pattern::Plain => "Sin dibujo",
            Pattern::Headers => "Tizones de otro color",
            Pattern::Courses(_) => "Hiladas de otro color",
            Pattern::Diamonds(_) => "Rombos",
        }
    }
}

/// The bond of a façade: its piece, joints and how they are laid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coursing {
    pub system: FacadeSystem,
    /// The face of a stretcher: its length and its height (metres).
    pub length: f64,
    pub height: f64,
    /// The face of a header (the width of the piece).
    pub depth: f64,
    /// Width of the joints, both ways.
    pub joint: f64,
    pub bond: Bond,
    /// The colour of the joints (the mortar; dark for open joints). `None`:
    /// a light grey mortar.
    pub joint_material: Option<MaterialId>,
    /// The second material, for the pieces [`Coursing::pattern`] picks.
    pub accent: Option<MaterialId>,
    pub pattern: Pattern,
    /// Lintels of bricks on end (a sardinel) over the doors and windows.
    pub soldier_heads: bool,
}

impl Default for Coursing {
    fn default() -> Self {
        FacadeSystem::Brick.coursing()
    }
}

/// Pieces of a course as (length, whether a header).
type Pieces = Vec<(f64, bool)>;

/// How a course starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// As the bond says (every other course with its closer).
    Free,
    /// At a bonded corner, with a whole piece running to it.
    Corner,
    /// At a bonded corner, with the end of the other wall's piece.
    Turn,
}

/// One piece of a course, along it from where the course starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub start: f64,
    pub end: f64,
    /// A header (its end shows), not a stretcher.
    pub header: bool,
    /// One of the second material.
    pub accent: bool,
}

impl Coursing {
    /// Height of a course with its joint.
    pub fn course_height(&self) -> f64 {
        self.height + self.joint
    }

    /// Whether the sizes make sense (positive, the joint smaller than the
    /// piece).
    pub fn is_valid(&self) -> bool {
        [self.length, self.height, self.depth]
            .iter()
            .all(|v| v.is_finite() && *v > 0.002)
            && self.joint.is_finite()
            && self.joint >= 0.0
            && self.joint < self.height.min(self.length)
    }

    /// The pieces of course `k` (counted from the datum: course 0 starts at
    /// height 0) along `len` metres from the start of the stretch of face,
    /// the last one cut where the stretch ends. `turn`: the course starts
    /// with the end of the piece of the other wall turning the corner (it
    /// takes the first `depth` metres), as in a bonded corner.
    pub fn course(&self, k: i64, len: f64, turn: bool) -> Vec<Unit> {
        self.course_from(k, len, if turn { Start::Turn } else { Start::Free })
    }

    /// Whether a bonded corner moves the courses of this bond (in the
    /// bonds of one kind of piece per course, the corner takes the place of
    /// the closer that moves every other course).
    pub fn bonds_at_corners(&self) -> bool {
        matches!(self.bond, Bond::Stretcher | Bond::Header | Bond::Lattice)
    }

    /// [`Coursing::course`] with how it starts.
    pub fn course_from(&self, k: i64, len: f64, start: Start) -> Vec<Unit> {
        let turn = start == Start::Turn;
        if !self.is_valid() || len <= 0.0 {
            return Vec::new();
        }
        let (l, d, j) = (self.length, self.depth, self.joint);
        let odd = k.rem_euclid(2) == 1;
        // The pieces of the course, repeated; the first ones may differ
        // (closers that shift the course).
        let (lead, repeat, gap): (Pieces, Pieces, f64) = match self.bond {
            Bond::Stretcher => (
                if odd {
                    vec![(l / 2.0 - j / 2.0, false)]
                } else {
                    vec![]
                },
                vec![(l, false)],
                j,
            ),
            Bond::Header => (
                if odd {
                    vec![(d / 2.0 - j / 2.0, true)]
                } else {
                    vec![]
                },
                vec![(d, true)],
                j,
            ),
            Bond::English => {
                if odd {
                    (vec![(d / 2.0 - j / 2.0, true)], vec![(d, true)], j)
                } else {
                    (vec![], vec![(l, false)], j)
                }
            }
            Bond::Flemish => {
                if odd {
                    let closer = (l - d) / 2.0 - j;
                    let lead = if closer > 0.005 {
                        vec![(closer, false)]
                    } else {
                        vec![]
                    };
                    (lead, vec![(d, true), (l, false)], j)
                } else {
                    (vec![], vec![(l, false), (d, true)], j)
                }
            }
            Bond::Lattice => {
                // A gap of half a piece: the courses alternate so each
                // piece stands on two.
                let gap = l / 2.0;
                (
                    if odd {
                        vec![(l / 4.0 - j / 2.0, false)]
                    } else {
                        vec![]
                    },
                    vec![(l, false)],
                    gap,
                )
            }
            Bond::Stack => (vec![], vec![(l, false)], j),
        };
        let mut out = Vec::new();
        let mut at = 0.0;
        let mut push = |at: &mut f64, size: f64, header: bool, idx: usize| -> bool {
            if *at >= len - 1e-6 {
                return false;
            }
            let end = (*at + size).min(len);
            out.push(Unit {
                start: *at,
                end,
                header,
                accent: self.accent_at(k, idx, header),
            });
            true
        };
        let mut idx = 0usize;
        if turn {
            // The end of the other wall's piece shows here.
            if !push(&mut at, d.min(len), true, idx) {
                return out;
            }
            idx += 1;
            at += d + j;
        } else if start == Start::Free {
            for (size, header) in lead {
                if !push(&mut at, size, header, idx) {
                    return out;
                }
                idx += 1;
                at += size + j;
            }
        }
        let mut guard = 0;
        'lay: loop {
            for (i, (size, header)) in repeat.iter().enumerate() {
                if !push(&mut at, *size, *header, idx) {
                    break 'lay;
                }
                idx += 1;
                let last = i + 1 == repeat.len();
                at += size + if last { gap } else { j };
            }
            guard += 1;
            if guard > 100_000 {
                break;
            }
        }
        out
    }

    /// Whether the piece whose middle is `x` metres along the face (from
    /// where the face starts) in course `k` takes the second material,
    /// for the patterns drawn by place (the diamonds); `None` for the
    /// rest, which go by the piece.
    pub fn accent_by_place(&self, x: f64, k: i64) -> Option<bool> {
        let Pattern::Diamonds(n) = self.pattern else {
            return None;
        };
        if self.accent.is_none() {
            return Some(false);
        }
        // On a grid of half pieces across and courses up, whose diamonds
        // have corners `n` pieces apart.
        let m = 2 * n.max(1) as i64;
        let i = (x / ((self.length + self.joint) / 2.0)).round() as i64;
        let (a, b) = (i.rem_euclid(2 * m) - m, k.rem_euclid(2 * m) - m);
        Some((a.abs() + b.abs() - m).abs() <= 1)
    }

    fn accent_at(&self, k: i64, idx: usize, header: bool) -> bool {
        if self.accent.is_none() {
            return false;
        }
        match self.pattern {
            Pattern::Plain => false,
            Pattern::Headers => header,
            Pattern::Courses(n) => n > 0 && k.rem_euclid(n as i64) == 0,
            Pattern::Diamonds(n) => {
                let n = n.max(2) as i64;
                let (a, b) = ((idx as i64).rem_euclid(2 * n) - n, k.rem_euclid(2 * n) - n);
                a.abs() + b.abs() == n
            }
        }
    }
}

/// The piece over the edge of each slab the façade passes (the «frente de
/// forjado» or «emparchado»): a band as high as the slab edge, of its own
/// piece and bond.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlabFront {
    /// Its piece and bond (plaquetas, bricks on end…).
    pub coursing: Coursing,
    /// Its material (`None`: the façade's).
    pub material: Option<MaterialId>,
    /// How far it stands out of the face of the façade (metres).
    pub projection: f64,
    /// How much it goes past the edge of the slab, above and below.
    pub over: f64,
}

impl SlabFront {
    /// Plaquetas: thin slips of brick in stretcher bond.
    pub fn slips() -> SlabFront {
        SlabFront {
            coursing: Coursing {
                depth: 0.02,
                soldier_heads: false,
                ..FacadeSystem::Brick.coursing()
            },
            material: None,
            projection: 0.0,
            over: 0.0,
        }
    }

    /// A course of bricks on end (a sardinel) over the edge.
    pub fn soldiers() -> SlabFront {
        let b = FacadeSystem::Brick.coursing();
        SlabFront {
            coursing: Coursing {
                length: b.height,
                height: b.length,
                bond: Bond::Stack,
                soldier_heads: false,
                ..b
            },
            material: None,
            projection: 0.0,
            over: 0.0,
        }
    }
}

/// The façade of a wall type: the bond of its exterior finish (its first
/// layer) and the piece over the slab edges.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Facade {
    pub coursing: Coursing,
    pub slab_front: Option<SlabFront>,
}

impl Model {
    /// Sets (or takes away, `None`) the façade of a wall type.
    pub fn set_wall_type_facade(
        &mut self,
        id: WallTypeId,
        facade: Option<Facade>,
    ) -> Result<(), ModelError> {
        if let Some(f) = &facade {
            let mut fronts = vec![f.coursing];
            if let Some(s) = &f.slab_front {
                fronts.push(s.coursing);
                finite(s.projection, "slab front projection")?;
                finite(s.over, "slab front over")?;
                if let Some(m) = s.material {
                    if self.material(m).is_none() {
                        return Err(ModelError::UnknownMaterial(m));
                    }
                }
            }
            for c in fronts {
                if !c.is_valid() {
                    return Err(ModelError::NonPositive("facade piece"));
                }
                for m in [c.joint_material, c.accent].into_iter().flatten() {
                    if self.material(m).is_none() {
                        return Err(ModelError::UnknownMaterial(m));
                    }
                }
            }
        }
        let t = self
            .wall_types
            .get_mut(&id)
            .ok_or(ModelError::UnknownWallType(id))?;
        t.facade = facade;
        Ok(())
    }

    /// The façade of a wall's type, if it has one and the wall is straight
    /// (curved walls show their finish as before).
    pub fn wall_facade(&self, wall: WallId) -> Option<&Facade> {
        let w = self.wall(wall)?;
        if w.is_curved() {
            return None;
        }
        self.wall_type(w.wall_type)?.facade.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joints(c: &Coursing, k: i64, len: f64) -> Vec<f64> {
        c.course(k, len, false).iter().map(|u| u.start).collect()
    }

    #[test]
    fn stretcher_bond_moves_half_a_brick_each_course() {
        let c = FacadeSystem::Brick.coursing();
        let even = joints(&c, 0, 1.0);
        let odd = joints(&c, 1, 1.0);
        assert_eq!(even[0], 0.0);
        assert!((even[1] - 0.25).abs() < 1e-9);
        // The odd course starts with half a brick: its joints fall over
        // the middle of the bricks below.
        assert!((odd[1] - 0.125).abs() < 1e-9, "{odd:?}");
        // The last one is cut where the stretch ends.
        let last = c.course(0, 0.9, false).last().copied().unwrap();
        assert!((last.end - 0.9).abs() < 1e-9);
    }

    #[test]
    fn flemish_puts_each_header_over_the_middle_of_a_stretcher() {
        let c = Coursing {
            bond: Bond::Flemish,
            ..FacadeSystem::Brick.coursing()
        };
        let below = c.course(0, 2.0, false);
        let above = c.course(1, 2.0, false);
        assert!(!below[0].header && below[1].header);
        let stretcher_mid = (below[0].start + below[0].end) / 2.0;
        let header = above.iter().find(|u| u.header).unwrap();
        assert!(((header.start + header.end) / 2.0 - stretcher_mid).abs() < 1e-9);
    }

    #[test]
    fn english_alternates_courses_and_headers_take_the_accent() {
        let mut c = Coursing {
            bond: Bond::English,
            ..FacadeSystem::Brick.coursing()
        };
        assert!(c.course(0, 1.0, false).iter().all(|u| !u.header));
        assert!(c.course(1, 1.0, false).iter().all(|u| u.header));
        c.accent = Some(MaterialId(1));
        c.pattern = Pattern::Headers;
        assert!(c.course(1, 1.0, false).iter().all(|u| u.accent));
        assert!(c.course(0, 1.0, false).iter().all(|u| !u.accent));
    }

    #[test]
    fn a_bonded_corner_starts_with_the_end_of_the_other_brick() {
        let c = FacadeSystem::Brick.coursing();
        let units = c.course(0, 1.0, true);
        assert!(units[0].header && (units[0].end - c.depth).abs() < 1e-9);
        assert!((units[1].start - c.depth - c.joint).abs() < 1e-9);
    }
}
