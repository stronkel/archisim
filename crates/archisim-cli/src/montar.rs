//! Builds a project from a plain-text description of a building
//! ("montar"): levels, walls by thickness, doors and windows, curtain walls,
//! slabs and rooms. The description is what a converter from CAD drawings
//! (double-line walls, door swings, room outlines) writes; one line per
//! element, fields separated by `|`:
//!
//! ```text
//! LEVEL|name|elevation
//! WALL|level|top level or -|thickness|ext or int|x1|y1|x2|y2|height if top is -
//! OPEN|wall index|door, door2, window, fixed or void|width|height|sill|along|ext or int|L or R
//! CURTAIN|level|top level|x1|y1|x2|y2
//! SLAB|level|solera, forjado or cubierta|x,y x,y …
//! HOLE|x,y x,y …            (a hole in the slab just above)
//! ROOM|level|number|name|label x,y|x,y x,y …|area in the drawing|holes (x,y … ; x,y …)
//! ```
//!
//! Walls are drawn on their centre line and go from their level up to the
//! top level. Doors and windows are measured from the start of the wall.
//! Whatever does not fit is skipped and reported, so a drawing with odd
//! bits still gives a model.

use std::collections::BTreeMap;
use std::path::Path;

use archisim_core::{
    Hand, Layer, LayerFunction, LevelId, Model, NewCurtainWall, NewOpening, NewOpeningType,
    NewRoom, NewSlab, NewWall, OpeningKind, OpeningTypeId, Point2, ReferenceLine, WallFunction,
    WallSide, WallTop, WallTypeId,
};

type R<T> = Result<T, Box<dyn std::error::Error>>;

/// What was built and what was left out.
#[derive(Default, Debug)]
pub struct Report {
    pub walls: usize,
    pub openings: usize,
    pub curtains: usize,
    pub slabs: usize,
    pub rooms: usize,
    pub skipped: Vec<String>,
}

fn num(s: &str) -> R<f64> {
    s.trim()
        .parse::<f64>()
        .map_err(|e| format!("número «{s}»: {e}").into())
}

fn points(s: &str) -> R<Vec<Point2>> {
    s.split_whitespace()
        .map(|p| {
            let (x, y) = p.split_once(',').ok_or("punto sin coma")?;
            Ok(Point2::new(num(x)?, num(y)?))
        })
        .collect()
}

/// Builds the project described by `text`.
pub fn build(name: &str, text: &str) -> R<(Model, Report)> {
    let mut m = Model::new(name);
    let lib = archisim_library::add_library(&mut m)?;
    let mut report = Report::default();
    let mut levels: BTreeMap<String, LevelId> = BTreeMap::new();

    // Materials for the walls: the brick of the template's partition for
    // thin walls, the one of its load-bearing wall for the rest.
    let core = |m: &Model, t: WallTypeId| {
        m.wall_type(t).and_then(|t| {
            t.layers
                .iter()
                .max_by(|a, b| a.thickness.total_cmp(&b.thickness))
                .map(|l| l.material)
        })
    };
    let thin_brick = core(&m, lib.partition).ok_or("la plantilla no tiene tabique")?;
    let thick_brick = core(&m, lib.bearing_wall).ok_or("la plantilla no tiene muro")?;
    let plaster = m
        .wall_type(lib.partition)
        .and_then(|t| {
            t.layers
                .iter()
                .find(|l| l.function == LayerFunction::Finish)
                .map(|l| l.material)
        })
        .unwrap_or(thin_brick);
    let door_type = m
        .opening_type(lib.interior_door)
        .cloned()
        .ok_or("la plantilla no tiene puerta")?;
    let window_type = m
        .opening_type(lib.window_two_leaves)
        .cloned()
        .ok_or("la plantilla no tiene ventana")?;

    let mut wall_types: BTreeMap<(i64, bool), WallTypeId> = BTreeMap::new();
    let mut opening_types: BTreeMap<(String, i64, i64), OpeningTypeId> = BTreeMap::new();
    let mut walls: Vec<Option<archisim_core::WallId>> = Vec::new();
    let mut last_slab: Option<archisim_core::SlabId> = None;

    for (n, line) in text.lines().enumerate() {
        let f: Vec<&str> = line.split('|').collect();
        let level = |levels: &BTreeMap<String, LevelId>, s: &str| -> R<LevelId> {
            levels
                .get(s)
                .copied()
                .ok_or_else(|| format!("línea {}: no hay planta «{s}»", n + 1).into())
        };
        match f[0] {
            "LEVEL" => {
                let id = m.add_level(f[1], num(f[2])?);
                levels.insert(f[1].to_string(), id);
            }
            "WALL" => {
                let base = level(&levels, f[1])?;
                // «-» as the top: a free-standing wall of the height given
                // last (a fence, a parapet).
                let top = if f[2] == "-" {
                    WallTop::Unconnected
                } else {
                    WallTop::Level {
                        level: level(&levels, f[2])?,
                        offset: 0.0,
                    }
                };
                let height = f.get(9).map(|h| num(h)).transpose()?.unwrap_or(3.0);
                let t = num(f[3])?;
                let exterior = f[4] == "ext";
                let cm = (t * 100.0).round() as i64;
                let wall_type = match wall_types.get(&(cm, exterior)) {
                    Some(id) => *id,
                    None => {
                        let (function, label) = if exterior {
                            (WallFunction::Exterior, "Muro exterior")
                        } else {
                            (WallFunction::Interior, "Tabique")
                        };
                        // Brick with 1.5 cm of plaster on each face when
                        // the wall is thick enough to have them.
                        let brick = if t < 0.2 { thin_brick } else { thick_brick };
                        let layers = if t >= 0.09 {
                            vec![
                                Layer {
                                    material: plaster,
                                    thickness: 0.015,
                                    function: LayerFunction::Finish,
                                },
                                Layer {
                                    material: brick,
                                    thickness: t - 0.03,
                                    function: LayerFunction::Structure,
                                },
                                Layer {
                                    material: plaster,
                                    thickness: 0.015,
                                    function: LayerFunction::Finish,
                                },
                            ]
                        } else {
                            vec![Layer {
                                material: brick,
                                thickness: t,
                                function: LayerFunction::Structure,
                            }]
                        };
                        let id = m.add_wall_type(format!("{label} {cm} cm"), function, layers)?;
                        wall_types.insert((cm, exterior), id);
                        id
                    }
                };
                let made = m.add_wall(NewWall {
                    wall_type,
                    level: base,
                    start: Point2::new(num(f[5])?, num(f[6])?),
                    end: Point2::new(num(f[7])?, num(f[8])?),
                    bulge: 0.0,
                    reference: ReferenceLine::Center,
                    height,
                    base_offset: 0.0,
                    top,
                });
                match made {
                    Ok(id) => {
                        report.walls += 1;
                        walls.push(Some(id));
                    }
                    Err(e) => {
                        report.skipped.push(format!("línea {}: muro: {e:?}", n + 1));
                        walls.push(None);
                    }
                }
            }
            "OPEN" => {
                let Some(Some(wall)) = walls.get(num(f[1])? as usize).copied() else {
                    continue;
                };
                let kind = f[2];
                let (w, h, sill, along) = (num(f[3])?, num(f[4])?, num(f[5])?, num(f[6])?);
                let key = (
                    kind.to_string(),
                    (w * 100.0).round() as i64,
                    (h * 100.0).round() as i64,
                );
                let opening_type = match opening_types.get(&key) {
                    Some(id) => *id,
                    None => {
                        use archisim_core::{DoorOperation as D, WindowOperation as W};
                        let (name, op, src) = match kind {
                            "door" => (
                                "Puerta 1 hoja",
                                OpeningKind::Door(D::SingleSwing),
                                &door_type,
                            ),
                            "door2" => (
                                "Puerta 2 hojas",
                                OpeningKind::Door(D::DoubleSwing),
                                &door_type,
                            ),
                            "window" => (
                                "Ventana 2 hojas",
                                OpeningKind::Window(W::DoubleCasement),
                                &window_type,
                            ),
                            "fixed" => (
                                "Acristalamiento fijo",
                                OpeningKind::Window(W::Fixed),
                                &window_type,
                            ),
                            _ => ("Hueco de paso", OpeningKind::Void, &door_type),
                        };
                        let id = m.add_opening_type(NewOpeningType {
                            name: format!("{name} {}×{}", key.1, key.2),
                            kind: op,
                            width: w,
                            height: h,
                            frame_width: src.frame_width,
                            frame_depth: src.frame_depth,
                            frame_material: src.frame_material,
                            panel_material: src.panel_material,
                        })?;
                        opening_types.insert(key, id);
                        id
                    }
                };
                let made = m.add_opening(NewOpening {
                    opening_type,
                    wall,
                    along,
                    sill,
                    opens_to: if f[7] == "ext" {
                        WallSide::Exterior
                    } else {
                        WallSide::Interior
                    },
                    hand: if f[8] == "L" { Hand::Left } else { Hand::Right },
                    setback: None,
                });
                match made {
                    Ok(_) => report.openings += 1,
                    Err(e) => report
                        .skipped
                        .push(format!("línea {}: {kind} de {w:.2} m: {e:?}", n + 1)),
                }
            }
            "CURTAIN" => {
                let (base, top) = (level(&levels, f[1])?, level(&levels, f[2])?);
                let made = m.add_curtain_wall(NewCurtainWall {
                    curtain_type: lib.curtain,
                    level: base,
                    start: Point2::new(num(f[3])?, num(f[4])?),
                    end: Point2::new(num(f[5])?, num(f[6])?),
                    base_offset: 0.0,
                    top: WallTop::Level {
                        level: top,
                        offset: 0.0,
                    },
                    height: 3.0,
                });
                match made {
                    Ok(_) => report.curtains += 1,
                    Err(e) => report
                        .skipped
                        .push(format!("línea {}: muro cortina: {e:?}", n + 1)),
                }
            }
            "SLAB" => {
                let slab_type = match f[2] {
                    "solera" => lib.ground_slab,
                    "cubierta" => lib.flat_roof,
                    _ => lib.floor_slab,
                };
                let made = m.add_slab(NewSlab {
                    slab_type,
                    level: level(&levels, f[1])?,
                    outline: points(f[3])?,
                    top_offset: 0.0,
                });
                match made {
                    Ok(id) => {
                        report.slabs += 1;
                        last_slab = Some(id);
                    }
                    Err(e) => {
                        last_slab = None;
                        report
                            .skipped
                            .push(format!("línea {}: forjado: {e:?}", n + 1));
                    }
                }
            }
            "HOLE" => {
                if let Some(slab) = last_slab {
                    if let Err(e) = m.add_slab_hole(slab, points(f[1])?) {
                        report
                            .skipped
                            .push(format!("línea {}: hueco: {e:?}", n + 1));
                    }
                }
            }
            "ROOM" => {
                let label = points(f[4])?.first().copied().unwrap_or_default();
                let made = m.add_room(NewRoom {
                    level: level(&levels, f[1])?,
                    number: f[2].to_string(),
                    name: f[3].to_string(),
                    outline: points(f[5])?,
                    holes: f
                        .get(7)
                        .map(|h| {
                            h.split(';')
                                .filter(|s| !s.trim().is_empty())
                                .map(points)
                                .collect::<R<Vec<_>>>()
                        })
                        .transpose()?
                        .unwrap_or_default(),
                    label,
                    bounded: false,
                });
                match made {
                    Ok(_) => report.rooms += 1,
                    Err(e) => {
                        report
                            .skipped
                            .push(format!("línea {}: local «{}»: {e:?}", n + 1, f[3]))
                    }
                }
            }
            "" => {}
            other => report
                .skipped
                .push(format!("línea {}: no sé qué es «{other}»", n + 1)),
        }
    }
    Ok((m, report))
}

/// Reads the description, builds the project and saves it.
pub fn run(spec: &Path, out: &Path, name: &str) -> R<()> {
    let text = std::fs::read_to_string(spec)?;
    let (m, r) = build(name, &text)?;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    archisim_file::save(&m, out)?;
    println!(
        "  {} muros, {} puertas y ventanas, {} muros cortina, {} forjados, {} locales",
        r.walls, r.openings, r.curtains, r.slabs, r.rooms
    );
    if !r.skipped.is_empty() {
        println!("  Sin colocar ({}):", r.skipped.len());
        for s in &r.skipped {
            println!("    {s}");
        }
    }
    println!("  Proyecto: {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_small_description_builds() {
        let text = "LEVEL|Planta baja|0\nLEVEL|Cubierta|3\n\
WALL|Planta baja|Cubierta|0.25|ext|0|0|6|0\n\
OPEN|0|door|0.9|2.1|0|3|int|L\n\
SLAB|Planta baja|solera|0,0 6,0 6,4 0,4\n\
ROOM|Planta baja|PB.01|Sala|3,2|0,0 6,0 6,4 0,4|24\n";
        let (m, r) = super::build("prueba", text).unwrap();
        assert!(r.skipped.is_empty(), "{:?}", r.skipped);
        assert_eq!((r.walls, r.openings, r.slabs, r.rooms), (1, 1, 1, 1));
        assert_eq!(m.walls().count(), 1);
    }
}
