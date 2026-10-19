//! Rooms ("locales", IfcSpace) and tags ("etiquetas") of doors, windows,
//! walls and rooms.
//!
//! A room has a number, a name, finishes and an outline in plan. A room
//! found inside walls (by clicking in it, as ArchiCAD's zones and Revit's
//! rooms) remembers the point it was found from, and its outline follows
//! the walls when they change. Its height is the clear height to the level
//! above unless it says otherwise.
//!
//! A tag is an annotation tied to an element: it writes what that element
//! is (a door's mark and size, a wall's type, a room's name and area) and
//! changes when the element does.

use crate::ids::GlobalId;
use crate::ids::*;
use crate::math::{signed_area, Point2};
use crate::model::{finite_point, polygon_contains, Model, ModelError};

/// Floor, wall and ceiling finishes of a room (the "cuadro de acabados").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Finishes {
    pub floor: String,
    pub walls: String,
    pub ceiling: String,
}

/// A room of a level.
#[derive(Debug, Clone, PartialEq)]
pub struct Room {
    pub id: RoomId,
    pub guid: GlobalId,
    pub level: LevelId,
    pub number: String,
    pub name: String,
    /// What it is used for ("Estancia", "Cocina", "Aseo"…).
    pub usage: String,
    /// Counter-clockwise, without repeating the first point.
    pub outline: Vec<Point2>,
    /// Columns, shafts and other islands inside it.
    pub holes: Vec<Vec<Point2>>,
    /// Where its name is written; for a room bounded by walls, also the
    /// point it is found from again.
    pub label: Point2,
    /// Bounded by walls: its outline follows them.
    pub bounded: bool,
    /// Clear height, metres; `None`: up to the level above.
    pub height: Option<f64>,
    pub finishes: Finishes,
}

impl Room {
    /// Floor area: the outline less its holes.
    pub fn area(&self) -> f64 {
        signed_area(&self.outline).abs()
            - self.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>()
    }

    /// Length of the boundary (outline and holes): the skirting.
    pub fn perimeter(&self) -> f64 {
        let ring = |p: &[Point2]| {
            (0..p.len())
                .map(|i| p[i].distance(p[(i + 1) % p.len()]))
                .sum::<f64>()
        };
        ring(&self.outline) + self.holes.iter().map(|h| ring(h)).sum::<f64>()
    }

    pub fn contains(&self, p: Point2) -> bool {
        polygon_contains(&self.outline, p) && !self.holes.iter().any(|h| polygon_contains(h, p))
    }
}

/// What is needed to create a room.
#[derive(Debug, Clone, PartialEq)]
pub struct NewRoom {
    pub level: LevelId,
    pub number: String,
    pub name: String,
    pub outline: Vec<Point2>,
    pub holes: Vec<Vec<Point2>>,
    pub label: Point2,
    pub bounded: bool,
}

/// The element a tag is tied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TagTarget {
    Opening(OpeningId),
    Wall(WallId),
    Room(RoomId),
}

/// A tag in plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub id: TagId,
    pub level: LevelId,
    pub target: TagTarget,
    /// Where the tag is drawn.
    pub at: Point2,
    /// A line from the tag to its element.
    pub leader: bool,
}

/// Centroid of a polygon's area.
pub fn centroid(poly: &[Point2]) -> Point2 {
    let n = poly.len();
    let (mut cx, mut cy, mut a2) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let cross = p.x * q.y - q.x * p.y;
        a2 += cross;
        cx += (p.x + q.x) * cross;
        cy += (p.y + q.y) * cross;
    }
    if a2.abs() < 1e-12 {
        let s = poly.iter().fold(Point2::new(0.0, 0.0), |acc, p| acc + *p);
        return s * (1.0 / n.max(1) as f64);
    }
    Point2::new(cx / (3.0 * a2), cy / (3.0 * a2))
}

/// The visual centre of an outline, where a room's name goes by default:
/// its centroid, if it lies inside; otherwise the middle of the widest
/// horizontal run through the centroid (inside an L-shaped room).
pub fn label_center(outline: &[Point2]) -> Point2 {
    let c = centroid(outline);
    if polygon_contains(outline, c) {
        return c;
    }
    let n = outline.len();
    let mut xs: Vec<f64> = (0..n)
        .filter_map(|i| {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            ((a.y > c.y) != (b.y > c.y)).then(|| a.x + (c.y - a.y) / (b.y - a.y) * (b.x - a.x))
        })
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.chunks(2)
        .filter(|p| p.len() == 2)
        .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])))
        .map_or(c, |p| Point2::new((p[0] + p[1]) / 2.0, c.y))
}

/// Heights of rooms on a level without their own: to the level above, or
/// this when there is none.
pub const DEFAULT_ROOM_HEIGHT: f64 = 2.5;

impl Model {
    /// The uses offered for rooms in this project (empty: the program's
    /// default list).
    pub fn room_usages(&self) -> &[String] {
        &self.room_usages
    }

    /// Sets the project's list of room uses: trimmed, none empty, no
    /// repeats (the first stays).
    pub fn set_room_usages(&mut self, usages: Vec<String>) -> Result<(), ModelError> {
        let mut out: Vec<String> = Vec::new();
        for u in usages {
            let u = u.trim().to_string();
            if u.is_empty() {
                return Err(ModelError::EmptyName);
            }
            if !out.contains(&u) {
                out.push(u);
            }
        }
        self.room_usages = out;
        Ok(())
    }

    /// Room numbers are unique on their level (empty numbers are allowed).
    fn check_room_number(
        &self,
        level: LevelId,
        number: &str,
        me: Option<RoomId>,
    ) -> Result<(), ModelError> {
        let number = number.trim();
        if !number.is_empty()
            && self
                .rooms
                .values()
                .any(|o| Some(o.id) != me && o.level == level && o.number.trim() == number)
        {
            return Err(ModelError::DuplicateNumber(number.to_string()));
        }
        Ok(())
    }

    fn check_room(
        &self,
        level: LevelId,
        outline: &[Point2],
        label: Point2,
    ) -> Result<(), ModelError> {
        if self.level(level).is_none() {
            return Err(ModelError::UnknownLevel(level));
        }
        finite_point(label, "room label")?;
        if outline.len() < 3 {
            return Err(ModelError::InvalidOutline);
        }
        Ok(())
    }

    pub fn add_room(&mut self, spec: NewRoom) -> Result<RoomId, ModelError> {
        self.check_room(spec.level, &spec.outline, spec.label)?;
        self.check_room_number(spec.level, &spec.number, None)?;
        let outline = Model::normalized_outline(spec.outline)?;
        let holes = spec
            .holes
            .into_iter()
            .map(Model::normalized_outline)
            .collect::<Result<Vec<_>, _>>()?;
        let id = RoomId(self.take_id());
        self.rooms.insert(
            id,
            Room {
                id,
                guid: GlobalId::new(),
                level: spec.level,
                number: spec.number,
                name: spec.name,
                usage: String::new(),
                outline,
                holes,
                label: spec.label,
                bounded: spec.bounded,
                height: None,
                finishes: Finishes::default(),
            },
        );
        Ok(id)
    }

    /// Replaces a room's data (keeps its id, GUID and level).
    pub fn set_room(&mut self, room: Room) -> Result<(), ModelError> {
        let old = self
            .rooms
            .get(&room.id)
            .ok_or(ModelError::UnknownRoom(room.id))?;
        self.check_room(old.level, &room.outline, room.label)?;
        self.check_room_number(old.level, &room.number, Some(room.id))?;
        if let Some(h) = room.height {
            if !h.is_finite() || h <= 0.0 {
                return Err(ModelError::NonPositive("room height"));
            }
        }
        let outline = Model::normalized_outline(room.outline)?;
        let holes = room
            .holes
            .into_iter()
            .map(Model::normalized_outline)
            .collect::<Result<Vec<_>, _>>()?;
        let guid = old.guid.clone();
        let level = old.level;
        self.rooms.insert(
            room.id,
            Room {
                guid,
                level,
                outline,
                holes,
                ..room
            },
        );
        Ok(())
    }

    /// Moves a room (and its tags) to another level, same outline. Its
    /// number has to be free there.
    pub fn set_room_level(&mut self, id: RoomId, level: LevelId) -> Result<(), ModelError> {
        let number = self
            .rooms
            .get(&id)
            .ok_or(ModelError::UnknownRoom(id))?
            .number
            .clone();
        if self.level(level).is_none() {
            return Err(ModelError::UnknownLevel(level));
        }
        self.check_room_number(level, &number, Some(id))?;
        if let Some(r) = self.rooms.get_mut(&id) {
            r.level = level;
        }
        for t in self.tags.values_mut() {
            if t.target == TagTarget::Room(id) {
                t.level = level;
            }
        }
        Ok(())
    }

    /// Removes a room and its tags.
    pub fn remove_room(&mut self, id: RoomId) -> Result<Room, ModelError> {
        let r = self.rooms.remove(&id).ok_or(ModelError::UnknownRoom(id))?;
        self.tags.retain(|_, t| t.target != TagTarget::Room(id));
        Ok(r)
    }

    pub fn room(&self, id: RoomId) -> Option<&Room> {
        self.rooms.get(&id)
    }

    pub fn rooms(&self) -> impl Iterator<Item = &Room> {
        self.rooms.values()
    }

    pub fn rooms_on_level(&self, level: LevelId) -> impl Iterator<Item = &Room> {
        self.rooms.values().filter(move |r| r.level == level)
    }

    /// The clear height of a room: its own, or to the level above.
    pub fn room_height(&self, r: &Room) -> f64 {
        if let Some(h) = r.height {
            return h;
        }
        let Some(level) = self.level(r.level) else {
            return DEFAULT_ROOM_HEIGHT;
        };
        self.levels()
            .iter()
            .filter(|l| l.elevation > level.elevation + 1e-6)
            .map(|l| l.elevation - level.elevation)
            .fold(None, |m: Option<f64>, h| Some(m.map_or(h, |m| m.min(h))))
            .unwrap_or(DEFAULT_ROOM_HEIGHT)
    }

    /// The next free room number of a level: one more than the highest
    /// number, keeping its prefix ("1.03" → "1.04"); "1" on an empty
    /// level.
    pub fn next_room_number(&self, level: LevelId) -> String {
        let numbers: Vec<&str> = self
            .rooms_on_level(level)
            .map(|r| r.number.as_str())
            .collect();
        let mut best: Option<(String, u64, usize)> = None;
        for n in &numbers {
            let digits = n.chars().rev().take_while(|c| c.is_ascii_digit()).count();
            if digits == 0 {
                continue;
            }
            let (prefix, tail) = n.split_at(n.len() - digits);
            let v: u64 = tail.parse().unwrap_or(0);
            if best.as_ref().is_none_or(|b| v > b.1) {
                best = Some((prefix.to_string(), v, digits));
            }
        }
        match best {
            Some((prefix, v, width)) => format!("{prefix}{:0width$}", v + 1),
            None => "1".into(),
        }
    }

    // ---- Tags ----------------------------------------------------------

    /// The level an element a tag can point at is on.
    pub fn tag_target_level(&self, t: TagTarget) -> Option<LevelId> {
        match t {
            TagTarget::Opening(o) => self.wall(self.opening(o)?.wall).map(|w| w.level),
            TagTarget::Wall(w) => self.wall(w).map(|w| w.level),
            TagTarget::Room(r) => self.room(r).map(|r| r.level),
        }
    }

    pub fn add_tag(
        &mut self,
        target: TagTarget,
        at: Point2,
        leader: bool,
    ) -> Result<TagId, ModelError> {
        let level = self.tag_target_level(target).ok_or(match target {
            TagTarget::Opening(o) => ModelError::UnknownOpening(o),
            TagTarget::Wall(w) => ModelError::UnknownWall(w),
            TagTarget::Room(r) => ModelError::UnknownRoom(r),
        })?;
        finite_point(at, "tag position")?;
        let id = TagId(self.take_id());
        self.tags.insert(
            id,
            Tag {
                id,
                level,
                target,
                at,
                leader,
            },
        );
        Ok(id)
    }

    pub fn move_tag(&mut self, id: TagId, at: Point2) -> Result<(), ModelError> {
        finite_point(at, "tag position")?;
        self.tags.get_mut(&id).ok_or(ModelError::UnknownTag(id))?.at = at;
        Ok(())
    }

    pub fn set_tag_leader(&mut self, id: TagId, leader: bool) -> Result<(), ModelError> {
        self.tags
            .get_mut(&id)
            .ok_or(ModelError::UnknownTag(id))?
            .leader = leader;
        Ok(())
    }

    pub fn remove_tag(&mut self, id: TagId) -> Result<Tag, ModelError> {
        self.tags.remove(&id).ok_or(ModelError::UnknownTag(id))
    }

    pub fn tag(&self, id: TagId) -> Option<&Tag> {
        self.tags.get(&id)
    }

    pub fn tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.values()
    }

    pub fn tags_on_level(&self, level: LevelId) -> impl Iterator<Item = &Tag> {
        self.tags.values().filter(move |t| t.level == level)
    }

    /// The tags of an element.
    pub fn tags_of(&self, target: TagTarget) -> impl Iterator<Item = &Tag> {
        self.tags.values().filter(move |t| t.target == target)
    }

    /// Takes away the tags whose element is gone; how many.
    pub fn remove_orphan_tags(&mut self) -> usize {
        let before = self.tags.len();
        let gone: Vec<TagId> = self
            .tags
            .values()
            .filter(|t| self.tag_target_level(t.target).is_none())
            .map(|t| t.id)
            .collect();
        for id in gone {
            self.tags.remove(&id);
        }
        before - self.tags.len()
    }

    pub(crate) fn restore_room(&mut self, r: Room) -> Result<(), ModelError> {
        self.check_room(r.level, &r.outline, r.label)?;
        self.check_room_number(r.level, &r.number, Some(r.id))?;
        Model::normalized_outline(r.outline.clone())?;
        self.rooms.insert(r.id, r);
        Ok(())
    }

    pub(crate) fn restore_tag(&mut self, t: Tag) -> Result<(), ModelError> {
        finite_point(t.at, "tag position")?;
        if self.level(t.level).is_none() {
            return Err(ModelError::UnknownLevel(t.level));
        }
        self.tags.insert(t.id, t);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64) -> Vec<Point2> {
        vec![
            Point2::new(x, y),
            Point2::new(x + s, y),
            Point2::new(x + s, y + s),
            Point2::new(x, y + s),
        ]
    }

    #[test]
    fn rooms_numbers_heights_and_tags() {
        let mut m = Model::new("P");
        let pb = m.add_level("PB", 0.0);
        m.add_level("P1", 3.0);
        let a = m
            .add_room(NewRoom {
                level: pb,
                number: "0.01".into(),
                name: "Salón".into(),
                outline: square(0.0, 0.0, 4.0),
                holes: vec![square(1.0, 1.0, 0.5)],
                label: Point2::new(2.0, 3.0),
                bounded: false,
            })
            .unwrap();
        let r = m.room(a).unwrap();
        assert!((r.area() - 15.75).abs() < 1e-9);
        assert!((r.perimeter() - 18.0).abs() < 1e-9);
        assert!(!r.contains(Point2::new(1.2, 1.2)));
        assert_eq!(m.room_height(r), 3.0);
        assert_eq!(m.next_room_number(pb), "0.02");
        let t = m
            .add_tag(TagTarget::Room(a), Point2::new(2.0, 2.0), false)
            .unwrap();
        assert_eq!(m.tag(t).unwrap().level, pb);
        // Removing the room takes its tag.
        m.remove_room(a).unwrap();
        assert!(m.tag(t).is_none());
        assert_eq!(m.next_room_number(pb), "1");
        // Round trip through the parts.
        let b = m
            .add_room(NewRoom {
                level: pb,
                number: "1".into(),
                name: "Cocina".into(),
                outline: square(5.0, 0.0, 3.0),
                holes: Vec::new(),
                label: Point2::new(6.0, 1.0),
                bounded: true,
            })
            .unwrap();
        m.add_tag(TagTarget::Room(b), Point2::new(6.0, 2.0), true)
            .unwrap();
        let back = Model::from_parts(m.to_parts()).unwrap();
        assert_eq!(back.room(b), m.room(b));
        assert_eq!(back.tags().count(), 1);
    }
}
