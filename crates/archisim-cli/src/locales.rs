//! `archisim locales`: the rooms of a project from a list (name; use; x;
//! y), each found from its point by the walls around it, as the Room tool
//! does with a click. For a house drawn without rooms (an IFC without
//! `IfcSpace`), so that occupancy, ventilation and the envelope can be
//! worked out.

use std::fmt::Write as _;
use std::path::Path;

use archisim_core::{LevelId, Model, NewRoom, Point2};
use archisim_geom::room::{room_outline, Contour};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// One line of the list: name; use; x; y (metres, decimal comma or point).
fn line(text: &str) -> Option<(String, String, Point2)> {
    let f: Vec<&str> = text.split(';').map(str::trim).collect();
    if f.len() < 4 {
        return None;
    }
    let n = |s: &str| s.replace(',', ".").parse::<f64>().ok();
    Some((
        f[0].to_string(),
        f[1].to_string(),
        Point2::new(n(f[2])?, n(f[3])?),
    ))
}

/// The level with most walls: the one the rooms go on by default.
fn main_level(m: &Model) -> Option<LevelId> {
    m.levels()
        .iter()
        .max_by_key(|l| m.walls_on_level(l.id).count())
        .map(|l| l.id)
}

/// Adds the rooms of `list` to `m`; returns what it says.
pub fn add_rooms(m: &mut Model, list: &str, level: Option<LevelId>) -> Res<String> {
    let level = level
        .or_else(|| main_level(m))
        .ok_or("el proyecto no tiene plantas")?;
    let mut t = String::new();
    let mut usages: Vec<String> = m.room_usages().to_vec();
    let mut n = m.rooms().filter(|r| r.level == level).count();
    for raw in list.lines() {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let Some((name, usage, at)) = line(raw) else {
            let _ = writeln!(t, "  no se entiende «{raw}» (nombre; uso; x; y)");
            continue;
        };
        // A point in a room already there: the same space (rooms open to
        // each other through a door opening are one, as in Revit).
        let same = m
            .rooms()
            .find(|r| r.level == level && archisim_core::point_in_polygon(at, &r.outline))
            .map(|r| r.name.clone());
        if let Some(other) = same {
            let _ = writeln!(
                t,
                "  {name}: es el mismo espacio que «{other}» (sin muros entre ellos)"
            );
            continue;
        }
        match room_outline(m, level, at, Contour::InteriorFace) {
            Ok(outline) => {
                n += 1;
                let id = m.add_room(NewRoom {
                    level,
                    number: n.to_string(),
                    name: name.clone(),
                    outline,
                    holes: Vec::new(),
                    label: at,
                    bounded: true,
                })?;
                let mut r = m.room(id).unwrap().clone();
                r.usage = usage.clone();
                m.set_room(r)?;
                if !usage.is_empty() && !usages.contains(&usage) {
                    usages.push(usage.clone());
                }
                let _ = writeln!(
                    t,
                    "  {n} {name} ({usage}): {:.2} m²",
                    m.room(id).unwrap().area()
                );
            }
            Err(e) => {
                let _ = writeln!(
                    t,
                    "  {name}: no se cierra en ({:.2}, {:.2}): {e}",
                    at.x, at.y
                );
            }
        }
    }
    if !m.room_usages().is_empty() {
        m.set_room_usages(usages)?;
    }
    Ok(t)
}

pub fn run(project: &Path, list: &Path, output: &Path) -> Res<String> {
    let mut m = archisim_file::open(project)?;
    let mut t = format!("Locales de {}:\n", list.display());
    t.push_str(&add_rooms(&mut m, &std::fs::read_to_string(list)?, None)?);
    archisim_file::save(&m, output)?;
    let _ = writeln!(t, "Proyecto: {}", output.display());
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rooms_from_a_list() {
        let mut m = crate::cajas::build().unwrap();
        let before = m.rooms().count();
        // A point inside the first box, and one outside every wall.
        let b = m.boxes().next().unwrap();
        let (c, level) = (b.centre(), b.level);
        let list = format!(
            "# nombre; uso; x; y\nEstar; Estancia; {:.3}; {:.3}\nOtra vez; Estancia; {:.3}; {:.3}\nFuera; Estancia; 500; 500\nmal\n",
            c.x, c.y, c.x + 0.1, c.y
        );
        let text = add_rooms(&mut m, &list, Some(level)).unwrap();
        assert_eq!(m.rooms().count(), before + 1, "{text}");
        assert!(
            text.contains("no se cierra") && text.contains("no se entiende"),
            "{text}"
        );
        assert!(text.contains("es el mismo espacio que «Estar»"), "{text}");
        let r = m.rooms().find(|r| r.name == "Estar").unwrap();
        assert_eq!(r.usage, "Estancia");
    }
}
