//! Measurements as a BC3 budget (FIEBDC-3, the exchange format of Spanish
//! budgeting programs: Presto, Arquímedes, TCQ…): one chapter per kind of
//! work, one item per type, one measurement line per element. Prices are
//! left at zero for the budget program to fill.

use std::fmt::Write;

use archisim_core::Model;
use archisim_plan::schedules::measurements;

/// `2026-10-19…` as BC3 writes dates: `19102026`.
pub fn bc3_date(iso: &str) -> String {
    let (y, rest) = iso.split_at(4.min(iso.len()));
    let m = rest.get(1..3).unwrap_or("01");
    let d = rest.get(4..6).unwrap_or("01");
    format!("{d}{m}{y}")
}

/// The BC3 file, in Windows-1252 as the format asks ("ANSI"). `date` as
/// `DDMMAAAA`.
pub fn bc3(model: &Model, date: &str, version: &str) -> Vec<u8> {
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if matches!(c, '|' | '\\' | '~') {
                    '/'
                } else {
                    c
                }
            })
            .filter(|c| !c.is_control())
            .collect()
    };
    let n = |v: f64| {
        let s = format!("{:.3}", (v * 1000.0).round() / 1000.0);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s == "-0" || s.is_empty() {
            "0".to_string()
        } else {
            s.to_string()
        }
    };
    let mut s = String::new();
    let _ = write!(
        s,
        "~V|ArchiSim|FIEBDC-3/2016\\{date}|ArchiSim {version}|{}|ANSI||2|||\r\n",
        clean(&model.project_name)
    );
    s.push_str("~K|\\2\\2\\3\\2\\2\\2\\2\\EUR\\|\r\n");
    let root = "ARCHISIM##";
    let _ = write!(
        s,
        "~C|{root}||{}|0|{date}|0|\r\n",
        clean(&model.project_name)
    );
    let chapters: Vec<_> = measurements(model)
        .into_iter()
        .filter(|(_, _, items)| !items.is_empty())
        .collect();
    let mut d = String::new();
    for (code, _, _) in &chapters {
        let _ = write!(d, "{code}#\\1\\1\\");
    }
    let _ = write!(s, "~D|{root}|{d}|\r\n");
    for (ci, (code, title, items)) in chapters.iter().enumerate() {
        let ch = format!("{code}#");
        let _ = write!(s, "~C|{ch}||{}|0|{date}|0|\r\n", clean(title));
        let mut d = String::new();
        for it in items {
            let _ = write!(d, "{}\\1\\{}\\", it.code, n(it.total()));
        }
        let _ = write!(s, "~D|{ch}|{d}|\r\n");
        for (ii, it) in items.iter().enumerate() {
            let _ = write!(
                s,
                "~C|{}|{}|{}|0|{date}|0|\r\n",
                it.code,
                it.unit,
                clean(&it.summary)
            );
            let mut lines = String::new();
            for l in &it.lines {
                let dim = |v: f64| if v == 0.0 { String::new() } else { n(v) };
                // Lines without dimensions carry their quantity as units.
                let units = if l.length == 0.0 && l.width == 0.0 && l.height == 0.0 {
                    l.quantity
                } else {
                    l.units
                };
                let _ = write!(
                    lines,
                    "\\{}\\{}\\{}\\{}\\{}\\",
                    clean(&l.comment),
                    n(units),
                    dim(l.length),
                    dim(l.width),
                    dim(l.height)
                );
            }
            let _ = write!(
                s,
                "~M|{ch}\\{}|{}\\{}\\|{}|{lines}|\r\n",
                it.code,
                ci + 1,
                ii + 1,
                n(it.total())
            );
        }
    }
    s.chars()
        .map(|c| match c {
            '€' => 0x80,
            c if (c as u32) < 256 => c as u32 as u8,
            '–' | '—' => b'-',
            '×' => b'x',
            _ => b'?',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archisim_core::{NewRoom, NewWall, Point2, ReferenceLine, WallTop};

    #[test]
    fn chapters_items_and_measurements() {
        let mut m = Model::new("Casa");
        let pb = m.add_level("Planta baja", 0.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        m.add_wall(NewWall {
            wall_type: lib.facade,
            level: pb,
            start: Point2::new(0.0, 0.0),
            end: Point2::new(5.0, 0.0),
            reference: ReferenceLine::ExteriorFace,
            height: 3.0,
            base_offset: 0.0,
            bulge: 0.0,
            top: WallTop::Unconnected,
        })
        .unwrap();
        let r = m
            .add_room(NewRoom {
                level: pb,
                number: "1".into(),
                name: "Salón".into(),
                outline: vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(4.0, 0.0),
                    Point2::new(4.0, 5.0),
                    Point2::new(0.0, 5.0),
                ],
                holes: Vec::new(),
                label: Point2::new(2.0, 2.0),
                bounded: false,
            })
            .unwrap();
        let mut room = m.room(r).unwrap().clone();
        room.finishes.floor = "Gres porcelánico".into();
        m.set_room(room).unwrap();
        assert_eq!(bc3_date("2026-10-19T15:00:00"), "19102026");
        let bytes = bc3(&m, "19102026", "0.25");
        let text: String = bytes.iter().map(|b| *b as char).collect();
        assert!(text.starts_with("~V|ArchiSim|FIEBDC-3/2016\\19102026|"));
        // The wall: 5 × 3 m = 15 m².
        assert!(text.contains("~D|01#|M01\\1\\15\\|"), "{text}");
        assert!(text.contains("\\Muro "), "{text}");
        // The floor finish, 20 m², with its accent in Windows-1252.
        assert!(text.contains("Suelo: Gres porcel\u{e1}nico"), "{text}");
        assert!(bytes.contains(&0xE1));
        assert!(text.contains("|20|"), "{text}");
        assert!(text.ends_with("\r\n"));
    }

    /// A refurbishment: the demolished wall goes to «Demoliciones», the
    /// existing one is not measured; a concrete column goes to
    /// «Estructura» in m³ and a steel beam in kg.
    #[test]
    fn demolitions_and_structure() {
        use archisim_core::{MemberUsage, NewBeam, NewColumn, Phase};
        let mut m = Model::new("Reforma");
        let pb = m.add_level("Planta baja", 0.0);
        m.add_level("Planta 1", 3.0);
        let lib = archisim_library::add_library(&mut m).unwrap();
        let wall = |m: &mut Model, y: f64| {
            m.add_wall(NewWall {
                wall_type: lib.facade,
                level: pb,
                start: Point2::new(0.0, y),
                end: Point2::new(4.0, y),
                reference: ReferenceLine::ExteriorFace,
                height: 2.5,
                base_offset: 0.0,
                bulge: 0.0,
                top: WallTop::Unconnected,
            })
            .unwrap()
        };
        let kept = wall(&mut m, 0.0);
        let gone = wall(&mut m, 5.0);
        wall(&mut m, 10.0);
        m.set_phase(kept.0, Phase::Existing).unwrap();
        m.set_phase(gone.0, Phase::Demolished).unwrap();
        let concrete = m
            .member_types()
            .find(|t| t.usage == MemberUsage::Column && t.name.contains("30"))
            .unwrap()
            .clone();
        let steel = m
            .member_types()
            .find(|t| t.usage == MemberUsage::Beam && t.name.contains("IPE"))
            .unwrap()
            .clone();
        m.add_column(NewColumn {
            member_type: concrete.id,
            level: pb,
            position: Point2::new(1.0, 1.0),
            rotation: 0.0,
            base_offset: 0.0,
            top: WallTop::Unconnected,
            height: 3.0,
        })
        .unwrap();
        m.add_beam(NewBeam {
            member_type: steel.id,
            level: pb,
            start: Point2::new(0.0, 2.0),
            end: Point2::new(5.0, 2.0),
            top_offset: 3.0,
        })
        .unwrap();
        let chapters = archisim_plan::schedules::measurements(&m);
        let chapter = |c: &str| chapters.iter().find(|x| x.0 == c).unwrap();
        // Demolitions: the one wall, 4 × 2,5 = 10 m².
        let demo = chapter("00");
        assert_eq!(demo.2.len(), 1);
        assert!((demo.2[0].total() - 10.0).abs() < 1e-9);
        // Walls: only the new one.
        let walls = chapter("01");
        assert!((walls.2.iter().map(|i| i.total()).sum::<f64>() - 10.0).abs() < 1e-9);
        // Structure: m³ of the column, kg of the beam.
        let st = chapter("05");
        let col = st.2.iter().find(|i| i.summary == concrete.name).unwrap();
        assert_eq!(col.unit, "m3");
        assert!((col.total() - concrete.profile.area() * 3.0).abs() < 1e-9);
        let beam = st.2.iter().find(|i| i.summary == steel.name).unwrap();
        assert_eq!(beam.unit, "kg");
        assert!((beam.total() - steel.profile.area() * 5.0 * 7850.0).abs() < 1e-6);
        let text: String = bc3(&m, "19102026", "0.28")
            .iter()
            .map(|b| *b as char)
            .collect();
        assert!(text.contains("~C|00#||Demoliciones|"), "{text}");
        assert!(text.contains("~C|05#||Estructura|"), "{text}");
    }
}
