//! Where the interface gets the plan geometry of walls from.
//!
//! Snapping and picking look at every wall of the level on each mouse
//! move: with thousands of walls, working out each one's joins every time
//! would be far too slow. The editor keeps the geometry of every wall,
//! worked out once per change of the model (through one index of the walls,
//! see [`archisim_geom::WallIndex`]); a bare [`Model`] (in tests) works it
//! out each time.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use archisim_core::{Model, WallId};
use archisim_geom::WallGeometry;

/// A model and the plan geometry of its walls.
pub trait Geo {
    fn model(&self) -> &Model;
    fn wall_geometry(&self, id: WallId) -> Option<Rc<WallGeometry>>;
}

#[cfg(test)]
impl Geo for Model {
    fn model(&self) -> &Model {
        self
    }

    fn wall_geometry(&self, id: WallId) -> Option<Rc<WallGeometry>> {
        archisim_geom::wall_geometry(self, id).map(Rc::new)
    }
}

/// The geometry of every wall, by wall.
type Geometries = HashMap<WallId, Rc<WallGeometry>>;

/// The geometry of every wall for one revision of a model.
#[derive(Default)]
pub struct GeoCache {
    cell: RefCell<Option<(u64, Geometries)>>,
}

impl GeoCache {
    /// The geometry of a wall of `model` at `revision`, working out every
    /// wall's the first time it is asked for at that revision.
    pub fn wall(&self, model: &Model, revision: u64, id: WallId) -> Option<Rc<WallGeometry>> {
        let mut cell = self.cell.borrow_mut();
        let fresh = cell.as_ref().is_some_and(|(r, _)| *r == revision);
        if !fresh {
            let idx = archisim_geom::WallIndex::new(model);
            let all = model
                .walls()
                .filter_map(|w| {
                    archisim_geom::wall_geometry_in(&idx, w.id).map(|g| (w.id, Rc::new(g)))
                })
                .collect();
            *cell = Some((revision, all));
        }
        cell.as_ref().and_then(|(_, m)| m.get(&id).cloned())
    }
}

impl std::fmt::Debug for GeoCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GeoCache")
    }
}

/// A model with one index of its walls, for a batch of geometry of a model
/// that is not the editor's (the model after a drag, while it is shown).
pub struct Indexed<'a>(pub archisim_geom::WallIndex<'a>);

impl<'a> Indexed<'a> {
    pub fn new(model: &'a Model) -> Self {
        Self(archisim_geom::WallIndex::new(model))
    }
}

impl Geo for Indexed<'_> {
    fn model(&self) -> &Model {
        self.0.model
    }

    fn wall_geometry(&self, id: WallId) -> Option<Rc<WallGeometry>> {
        archisim_geom::wall_geometry_in(&self.0, id).map(Rc::new)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::time::Instant;

    use archisim_core::Point2;

    use crate::drag::{Drag, Grip};
    use crate::editor::Editor;
    use crate::pick::Element;

    fn timed<T>(what: &str, f: impl FnOnce() -> T) -> T {
        let t = Instant::now();
        let out = f();
        println!(
            "  {what:<44} {:>9.1} ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
        out
    }

    /// The editor's geometry is the model's, also after a change and its
    /// undo.
    #[test]
    fn editor_geometry_follows_the_model() {
        let (m, _, _) = crate::pick::tests::room();
        let mut ed = Editor::with_model(m, None);
        let same = |ed: &Editor| {
            for w in ed.model().walls() {
                let cached = crate::geo::Geo::wall_geometry(ed, w.id).map(|g| (*g).clone());
                assert_eq!(cached, archisim_geom::wall_geometry(ed.model(), w.id));
            }
        };
        same(&ed);
        let w = ed.model().walls().next().unwrap();
        let (id, at) = (w.id, w.midpoint());
        ed.selection = BTreeSet::from([Element::Wall(id)]);
        let before = crate::geo::Geo::wall_geometry(&ed, id).unwrap();
        ed.drag(&Drag {
            grip: Grip::Body,
            from: at,
            to: at + Point2::new(0.3, 0.2),
            variant: false,
        })
        .unwrap();
        same(&ed);
        assert_ne!(*crate::geo::Geo::wall_geometry(&ed, id).unwrap(), *before);
        ed.undo().unwrap();
        same(&ed);
        assert_eq!(*crate::geo::Geo::wall_geometry(&ed, id).unwrap(), *before);
    }

    /// What the plan view does on every mouse move, on a large model
    /// (`archisim medir 20 carpeta` writes one):
    /// `ARCHISIM_GRANDE=carpeta/grande.archisim cargo test --release -p
    /// archisim-app -- --ignored large_model --nocapture`.
    #[test]
    #[ignore]
    fn large_model() {
        let Ok(path) = std::env::var("ARCHISIM_GRANDE") else {
            return;
        };
        let mut ed = Editor::open(std::path::Path::new(&path)).unwrap();
        let level = ed.level;
        let walls = ed.model().walls().count();
        println!("{walls} muros:");
        let wall = ed.model().walls().nth(walls / 2).map(|w| w.id).unwrap();
        let at = ed.model().wall(wall).unwrap().midpoint();
        timed("Geometría de todos los muros (tras un cambio)", || {
            crate::geo::Geo::wall_geometry(&ed, wall)
        });
        timed("Forzado bajo el cursor", || {
            crate::snap::snap(&ed, level, at + Point2::new(0.3, 0.3), 0.1, &[])
        });
        timed("Elemento bajo el cursor", || {
            crate::pick::pick(&ed, level, at, 0.1)
        });
        ed.selection = BTreeSet::from([Element::Wall(wall)]);
        timed("Cotas temporales de un muro", || {
            crate::temp_dims::temp_dims(&ed, &ed.selection)
        });
        timed("Asa bajo el cursor", || {
            crate::drag::grip_at(&ed, &ed.selection, at, 0.1)
        });
        let drag = Drag {
            grip: Grip::Body,
            from: at,
            to: at + Point2::new(0.5, 0.0),
            variant: false,
        };
        let ghost = timed("Vista previa de mover un muro", || {
            crate::drag::preview(&ed, &ed.selection, &drag)
        });
        assert!(!ghost.unwrap().is_empty());
        timed("Mover el muro (y rehacer la geometría)", || {
            ed.drag(&drag).unwrap();
            crate::geo::Geo::wall_geometry(&ed, wall)
        });
    }
}
