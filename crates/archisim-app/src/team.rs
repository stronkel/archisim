//! Team work in the application, without the interface: the session on the
//! shared folder and the guard that keeps each edit to what the person has
//! reserved. The window, the avatars and the drawing of reservations are in
//! `app/team_ui`.

use crate::geo::Indexed;
use crate::pick::{self, Element};
use archisim_core::{LevelId, Model, Point2};
use archisim_team::{diff, doc_of, Locator, Session, Zone};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The id of an element in the model (and in the project file).
pub fn raw_id(e: Element) -> u64 {
    match e {
        Element::Dimension(i) => i.0,
        Element::Mark(i) => i.0,
        Element::Draft(i) => i.0,
        Element::Opening(i) => i.0,
        Element::Wall(i) => i.0,
        Element::Slab(i) => i.0,
        Element::Column(i) => i.0,
        Element::Beam(i) => i.0,
        Element::Section(i) => i.0,
        Element::Roof(i) | Element::Dormer(i, _) => i.0,
        Element::Stair(i) => i.0,
        Element::Terrain(i) => i.0,
        Element::CurtainWall(i) => i.0,
        Element::Railing(i) => i.0,
        Element::Object(i) => i.0,
        Element::Room(i) => i.0,
        Element::Tag(i) => i.0,
        Element::Footing(i) => i.0,
        Element::Grid(i) => i.0,
        Element::RefPlane(i) => i.0,
        Element::Module(i) => i.0,
        Element::Mass(i) => i.0,
        Element::Plot(i) => i.0,
        Element::Pool(i) => i.0,
        Element::Elevator(i) => i.0,
        Element::Road(i) => i.0,
        Element::Roundabout(i) => i.0,
        Element::Marking(i) => i.0,
        Element::Decal(i) => i.0,
        Element::Planting(i) => i.0,
        Element::Run(i) => i.0,
        Element::Edge(i) => i.0,
        Element::Device(i) => i.0,
    }
}

fn centroid(poly: &[Point2]) -> Point2 {
    let n = poly.len().max(1) as f64;
    let (x, y) = poly.iter().fold((0.0, 0.0), |(x, y), p| (x + p.x, y + p.y));
    Point2::new(x / n, y / n)
}

/// Finds the elements of a model by id to tell whether they are in a zone:
/// on its level and, if it has a polygon, with the middle of their outline
/// inside it.
pub struct ModelLocator<'a> {
    model: &'a Model,
    levels: RefCell<BTreeMap<u64, BTreeMap<u64, Element>>>,
}

impl<'a> ModelLocator<'a> {
    pub fn new(model: &'a Model) -> Self {
        Self {
            model,
            levels: RefCell::new(BTreeMap::new()),
        }
    }
}

impl Locator for ModelLocator<'_> {
    fn in_zone(&self, id: u64, zone: &Zone) -> bool {
        let mut cache = self.levels.borrow_mut();
        let on = cache.entry(zone.level).or_insert_with(|| {
            pick::elements_on_level(self.model, LevelId(zone.level))
                .into_iter()
                .map(|e| (raw_id(e), e))
                .collect()
        });
        let Some(e) = on.get(&id) else {
            return false;
        };
        if zone.polygon.len() < 3 {
            return true;
        }
        let geo = Indexed::new(self.model);
        let Some(poly) = pick::outline(&geo, *e) else {
            return false;
        };
        let c = centroid(&poly);
        archisim_team::inside(&zone.polygon, [c.x, c.y])
    }
}

/// The project file and who has each element inside a zone, at one
/// revision of the editor.
type Before = (u64, Value, BTreeMap<u64, (bool, String)>);

/// Who has what, as the editor needs it to accept or refuse an edit.
#[derive(Default)]
pub struct Guard {
    /// Elements someone else reserved: id → their name.
    pub others: BTreeMap<u64, String>,
    /// Elements I reserved.
    pub mine: BTreeSet<u64>,
    /// Every zone: whether it is mine, whose, and the zone.
    pub zones: Vec<(bool, String, Zone)>,
    /// Elements edited that nobody had: they become mine.
    pub auto: BTreeSet<u64>,
    /// The project file and the zones' elements before the edit, by
    /// revision of the editor.
    before: Option<Before>,
}

impl Guard {
    /// The picture of a session's reservations.
    pub fn of(session: &Session) -> Self {
        let me = session.me.slug.clone();
        let mut g = Guard::default();
        for (who, ids) in session.reserved_by() {
            if who == me {
                g.mine.extend(ids);
            } else {
                let name = session.member(&who).map_or(who.clone(), |m| m.name.clone());
                g.others
                    .extend(ids.into_iter().map(|id| (id, name.clone())));
            }
        }
        g.zones = session
            .zones()
            .into_iter()
            .map(|(m, z)| (m.slug == me, m.name.clone(), z.clone()))
            .collect();
        g
    }

    /// Who has an element by a zone, in `model`.
    fn zone_owner(&self, loc: &ModelLocator, id: u64) -> Option<(bool, String)> {
        let mut best: Option<(u64, bool, &str)> = None;
        for (mine, who, z) in &self.zones {
            if loc.in_zone(id, z) && best.is_none_or(|b| z.clock < b.0) {
                best = Some((z.clock, *mine, who));
            }
        }
        best.map(|(_, m, w)| (m, w.to_string()))
    }

    /// Remembers the model as it is before an edit.
    pub fn prepare(&mut self, revision: u64, model: &Model) {
        if self.before.as_ref().is_some_and(|b| b.0 == revision) {
            return;
        }
        let mut zoned = BTreeMap::new();
        if !self.zones.is_empty() {
            let loc = ModelLocator::new(model);
            for (_, _, z) in &self.zones {
                for e in pick::elements_on_level(model, LevelId(z.level)) {
                    let id = raw_id(e);
                    if let Some(o) = self.zone_owner(&loc, id) {
                        zoned.insert(id, o);
                    }
                }
            }
        }
        self.before = Some((revision, doc_of(model), zoned));
    }

    /// Accepts the edit that turned the prepared model into `after`, or
    /// says why not. Elements nobody had become mine.
    pub fn check(&mut self, after: &Model) -> Result<(), String> {
        let Some((_, before, zoned)) = &self.before else {
            return Ok(());
        };
        let ops = diff::diff(before, &doc_of(after));
        let touched = diff::touched(&ops);
        if touched.is_empty() {
            return Ok(());
        }
        let loc = ModelLocator::new(after);
        let mut refused: BTreeMap<String, usize> = BTreeMap::new();
        let mut free = Vec::new();
        for id in touched {
            if self.mine.contains(&id) {
                continue;
            }
            if let Some(who) = self.others.get(&id) {
                *refused.entry(who.clone()).or_default() += 1;
                continue;
            }
            // In a zone before the edit (changed or removed), or after it
            // (added or moved into one).
            let zone = zoned
                .get(&id)
                .cloned()
                .or_else(|| self.zone_owner(&loc, id));
            match zone {
                Some((true, _)) => {}
                Some((false, who)) => *refused.entry(who).or_default() += 1,
                None => free.push(id),
            }
        }
        if !refused.is_empty() {
            let who: Vec<String> = refused
                .into_iter()
                .map(|(w, n)| format!("{n} elemento{} de {w}", if n == 1 { "" } else { "s" }))
                .collect();
            return Err(format!(
                "reservado por otra persona ({}): pídeselo desde Equipo",
                who.join(", ")
            ));
        }
        for id in free {
            self.mine.insert(id);
            self.auto.insert(id);
        }
        Ok(())
    }
}

/// The person's name and computer, kept between sessions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Identity {
    pub name: String,
    pub machine: String,
}

fn settings_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    }?;
    Some(base.join("ArchiSim"))
}

impl Identity {
    /// Who I am on this computer (`ARCHISIM_EQUIPO_NOMBRE` overrides it,
    /// for tests and for trying two people on one computer).
    pub fn load() -> Self {
        if cfg!(test) {
            return Identity {
                name: std::env::var("ARCHISIM_EQUIPO_NOMBRE").unwrap_or_default(),
                machine: "pruebas".into(),
            };
        }
        let file = settings_dir().map(|d| d.join("equipo.txt"));
        let text = file
            .as_deref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .unwrap_or_default();
        let mut lines = text.lines();
        let mut id = Identity {
            name: lines.next().unwrap_or("").trim().to_string(),
            machine: lines.next().unwrap_or("").trim().to_string(),
        };
        if let Ok(n) = std::env::var("ARCHISIM_EQUIPO_NOMBRE") {
            id.name = n;
        }
        if id.machine.is_empty() {
            id.machine = format!(
                "{}-{:x}",
                std::env::var("COMPUTERNAME")
                    .or_else(|_| std::env::var("HOSTNAME"))
                    .unwrap_or_else(|_| "ordenador".into()),
                archisim_team::folder::now()
            );
            id.save();
        }
        id
    }

    pub fn save(&self) {
        if cfg!(test) {
            return;
        }
        let Some(dir) = settings_dir() else { return };
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(
                dir.join("equipo.txt"),
                format!("{}\n{}\n", self.name, self.machine),
            );
        }
    }
}

/// The open shared project.
pub struct Team {
    pub session: Session,
    /// Whether my cursor is shown to the others (off unless chosen).
    pub share_cursor: bool,
    /// Whether to show the others' cursors.
    pub show_cursors: bool,
    /// When the folder was last looked at (seconds of the app's clock).
    pub polled: f64,
    /// When I last said I am here.
    pub touched: f64,
    /// Editor revision of the last publish or receive (to know if there
    /// is anything to publish without comparing every frame).
    pub synced_revision: u64,
    /// My changes not published, worked out for `counted_revision`.
    pub local: usize,
    pub counted_revision: u64,
}

impl Team {
    pub fn new(session: Session, revision: u64) -> Self {
        Self {
            session,
            share_cursor: false,
            show_cursors: true,
            polled: f64::NEG_INFINITY,
            touched: f64::NEG_INFINITY,
            synced_revision: revision,
            local: 0,
            counted_revision: revision,
        }
    }

    /// Elements of `model` changed since the last receive (worked out
    /// again only when the model changed).
    pub fn local_changes(&mut self, model: &Model, revision: u64) -> usize {
        if revision != self.counted_revision {
            self.counted_revision = revision;
            self.local = if revision == self.synced_revision {
                0
            } else {
                let ops = self.session.local_ops(model);
                diff::touched(&ops).len().max(usize::from(!ops.is_empty()))
            };
        }
        self.local
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    use archisim_core::{NewWall, ReferenceLine, WallTop};

    fn wall(
        ed: &mut Editor,
        a: (f64, f64),
        b: (f64, f64),
    ) -> Result<archisim_core::WallId, String> {
        let t = ed.model().wall_types().next().unwrap().id;
        let level = ed.level;
        ed.transact("Muro", |m| {
            m.add_wall(NewWall {
                wall_type: t,
                level,
                start: Point2::new(a.0, a.1),
                end: Point2::new(b.0, b.1),
                bulge: 0.0,
                reference: ReferenceLine::Center,
                height: 3.0,
                base_offset: 0.0,
                top: WallTop::Unconnected,
            })
        })
    }

    #[test]
    fn edits_keep_to_what_is_reserved() {
        let mut ed = Editor::new_project();
        let w = wall(&mut ed, (0.0, 0.0), (5.0, 0.0)).unwrap();
        let mut g = Guard::default();
        g.others.insert(w.0, "Ana".into());
        ed.team = Some(g);
        // Ana's wall can't be moved…
        let e = ed
            .transact("Mover", |m| {
                m.move_wall(w, Point2::new(0.0, 1.0), Point2::new(5.0, 1.0))
            })
            .unwrap_err();
        assert!(e.contains("Ana"), "{e}");
        assert!((ed.model().wall(w).unwrap().start.y).abs() < 1e-9);
        // …but a new wall can be drawn, and is mine.
        let w2 = wall(&mut ed, (0.0, 0.0), (0.0, 5.0)).unwrap();
        let g = ed.team.as_ref().unwrap();
        assert!(g.mine.contains(&w2.0) && g.auto.contains(&w2.0));
        // A zone of Ana's keeps out new walls inside it.
        let level = ed.level.0;
        ed.team.as_mut().unwrap().zones.push((
            false,
            "Ana".into(),
            Zone {
                name: "Garaje".into(),
                level,
                polygon: vec![[10.0, -5.0], [20.0, -5.0], [20.0, 5.0], [10.0, 5.0]],
                clock: 1,
                time: 0,
            },
        ));
        assert!(wall(&mut ed, (12.0, 0.0), (18.0, 0.0)).is_err());
        assert!(wall(&mut ed, (0.0, 5.0), (5.0, 5.0)).is_ok());
    }
}
